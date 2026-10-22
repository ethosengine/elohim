//! Explicit legacy attribution is an append-only prerequisite, never acceptance or fulfillment.
use std::path::Path;
use std::process::{Command, Output};

use cid::Cid;
use elohim_epr_cli::{actor, flow};
use elohim_epr_rea::{FlowRecord, FlowStore, Intent, ReaVerb, ResourceSpec, SidecarFlowStore};
use serde_json::Value;
use tempfile::TempDir;

struct Fixture {
    dir: TempDir,
    commitment: String,
    implementer: String,
    acceptor: String,
}

fn cli(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(args)
        .args(["--root", root.to_str().unwrap(), "--json"])
        .env_remove("ELOHIM_SESSION_ID")
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .env_remove("CLAUDE_SESSION_ID")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .current_dir(root)
        .output()
        .unwrap()
}

fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn fixture() -> Fixture {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join(".epr-meta")).unwrap();
    std::fs::write(
        root.join(".epr-meta/repository.yaml"),
        "version: 1\nagent: repo:ethosengine/elohim\n",
    )
    .unwrap();
    std::fs::write(root.join("scope.md"), "Original bounded assertion").unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["add", "scope.md"],
        vec!["commit", "-qm", "fixture"],
    ] {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env("GIT_AUTHOR_NAME", "Fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
            .env("GIT_COMMITTER_NAME", "Fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.test")
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
    }
    let implementer = actor::claim(root, "agent:implementer@gpt-6", "implementation")
        .unwrap()
        .record_cid;
    let acceptor = actor::claim(root, "agent:acceptor@gpt-6", "acceptance")
        .unwrap()
        .record_cid;
    let scope = flow::body_cid("Original bounded assertion");
    let intent = SidecarFlowStore::open(root)
        .unwrap()
        .append(FlowRecord::Intent(Intent {
            action: ReaVerb::Produce,
            resource_spec: ResourceSpec {
                classified_as: vec!["gap:open".into(), "fixture-binding".into()],
                quantity: None,
            },
            in_scope_of: scope,
            raised_by: flow::repo_agent(),
        }))
        .unwrap();
    // The real legacy trigger: direct --as wins over the simultaneously supplied --session.
    let claimed = success(cli(
        root,
        &[
            "flow",
            "claim",
            "--on",
            &intent.to_string(),
            "--as",
            "agent:implementer@gpt-6",
            "--session",
            "implementation",
        ],
    ));
    Fixture {
        dir,
        commitment: claimed["commitment_cid"].as_str().unwrap().into(),
        implementer,
        acceptor,
    }
}

fn records(root: &Path) -> Vec<(Cid, FlowRecord)> {
    SidecarFlowStore::open(root).unwrap().records().unwrap()
}

fn appoint(f: &Fixture) -> Output {
    cli(
        f.dir.path(),
        &[
            "flow",
            "note",
            "--on",
            &f.commitment,
            "--kind",
            "ruling",
            "--appoint",
            &f.acceptor,
            "--reason",
            "Independent acceptance must exercise the oracle",
        ],
    )
}

fn bind(f: &Fixture) -> Output {
    cli(
        f.dir.path(),
        &[
            "flow",
            "note",
            "--on",
            &f.commitment,
            "--kind",
            "ruling",
            "--bind-actor",
            &f.implementer,
            "--session",
            "implementation",
            "--reason",
            "Explicitly bind the original implementer without rewriting the commitment",
        ],
    )
}

#[test]
fn named_session_claim_needs_explicit_binding_before_independent_appointment() {
    let f = fixture();
    let root = f.dir.path();
    let before = records(root);
    let original = before
        .iter()
        .find(|(cid, _)| cid.to_string() == f.commitment)
        .unwrap()
        .clone();
    let FlowRecord::Commitment(promise) = &original.1 else {
        panic!("commitment")
    };
    assert_eq!(promise.provider.0, "agent:implementer@gpt-6");
    assert!(!promise
        .resource_spec
        .classified_as
        .iter()
        .any(|s| s.starts_with("actor-claim:")));
    let refused = appoint(&f);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("no exact registered actor claim"));
    assert_eq!(records(root), before);

    let before_context = success(cli(root, &["flow", "context", "scope.md"]));
    assert_eq!(
        before_context["reconciliation"]["assertions"][0]["commitments"][0]["attribution"]
            ["eligible"],
        false
    );
    let witness = success(bind(&f));
    let after_context = success(cli(root, &["flow", "context", "scope.md"]));
    let native = &after_context["reconciliation"]["assertions"][0]["commitments"][0]["attribution"];
    assert_eq!(native["eligible"], true);
    assert_eq!(native["actor_claim"], f.implementer);
    assert_eq!(native["witness"], witness["record_cid"]);
    assert_eq!(native["issues"], serde_json::json!([]));
    let appointment = success(appoint(&f));
    let after = records(root);
    assert_eq!(
        after.iter().find(|(cid, _)| *cid == original.0).unwrap(),
        &original
    );
    assert_eq!(
        after
            .iter()
            .filter(|(_, r)| matches!(r, FlowRecord::Commitment(_)))
            .count(),
        1
    );
    for (id, record) in after
        .iter()
        .filter(|(id, _)| !before.iter().any(|(old, _)| old == id))
    {
        let FlowRecord::Event(event) = record else {
            panic!("only notes may be appended")
        };
        assert_eq!(event.action, ReaVerb::Cite);
        assert!(event.fulfills.is_empty() && event.satisfies.is_empty());
        if id.to_string() == appointment["record_cid"].as_str().unwrap() {
            assert!(event.classified_as.contains(&format!(
                "implementer-binding:{}",
                witness["record_cid"].as_str().unwrap()
            )));
        }
    }
    assert_eq!(success(bind(&f))["appended"], false);
    assert_eq!(records(root), after);
}

fn eligibility(f: &Fixture) -> Value {
    serde_json::to_value(flow::acceptance::attribution_eligibility(
        f.dir.path(),
        &records(f.dir.path()),
        &f.commitment.parse().unwrap(),
    ))
    .unwrap()
}

fn witness_event(f: &Fixture) -> elohim_epr_rea::FlowEvent {
    let result = success(bind(f));
    records(f.dir.path())
        .into_iter()
        .find_map(|(id, record)| match record {
            FlowRecord::Event(event)
                if id.to_string() == result["record_cid"].as_str().unwrap() =>
            {
                Some(event)
            }
            _ => None,
        })
        .unwrap()
}

#[test]
fn binding_requires_exact_current_session_provider_and_is_attribution_only() {
    let f = fixture();
    let root = f.dir.path();
    let unknown = flow::body_cid("not a registered actor claim").to_string();
    let cases = vec![
        vec!["--bind-actor", f.implementer.as_str()],
        vec![
            "--bind-actor",
            unknown.as_str(),
            "--session",
            "implementation",
        ],
        vec![
            "--bind-actor",
            f.acceptor.as_str(),
            "--session",
            "acceptance",
        ],
        vec![
            "--bind-actor",
            f.implementer.as_str(),
            "--session",
            "implementation",
            "--as",
            "agent:other@gpt-6",
        ],
        vec![
            "--bind-actor",
            f.implementer.as_str(),
            "--session",
            "implementation",
            "--appoint",
            f.acceptor.as_str(),
        ],
        vec![
            "--bind-actor",
            f.implementer.as_str(),
            "--session",
            "implementation",
            "--purpose",
            "acceptance",
        ],
    ];
    for extra in cases {
        let before = std::fs::read(root.join(".eprfs/status/flows.jsonl")).unwrap();
        let mut args = vec![
            "flow",
            "note",
            "--on",
            f.commitment.as_str(),
            "--kind",
            "ruling",
            "--reason",
            "fixture",
        ];
        args.extend(extra);
        let output = cli(root, &args);
        assert!(!output.status.success(), "unexpected admission: {args:?}");
        assert_eq!(
            std::fs::read(root.join(".eprfs/status/flows.jsonl")).unwrap(),
            before
        );
    }
    let alias = cli(
        root,
        &[
            "flow",
            "note",
            "--on",
            "fixture-binding",
            "--kind",
            "ruling",
            "--bind-actor",
            &f.implementer,
            "--session",
            "implementation",
            "--reason",
            "no alias",
        ],
    );
    assert!(!alias.status.success());
    let mut f = f;
    actor::claim(f.dir.path(), "agent:other@gpt-6", "implementation").unwrap();
    let stale = bind(&f);
    assert!(!stale.status.success());
    assert!(String::from_utf8_lossy(&stale.stderr).contains("current"));
    // Even a return to the same provider names a new exact claim, not a guessed old one.
    let returned = actor::claim(f.dir.path(), "agent:implementer@gpt-6", "implementation").unwrap();
    assert_ne!(returned.record_cid, f.implementer);
    assert!(!bind(&f).status.success());
    f.implementer = returned.record_cid;
    success(bind(&f));
    assert_eq!(eligibility(&f)["eligible"], true);
}

#[test]
fn historical_binding_survives_session_rollover_without_reassigning_the_original_actor() {
    let f = fixture();
    let bound = success(bind(&f));
    actor::claim(f.dir.path(), "agent:later-persona@gpt-6", "implementation").unwrap();
    let observed = eligibility(&f);
    assert_eq!(observed["eligible"], true);
    assert_eq!(observed["actor_claim"], f.implementer);
    assert_eq!(observed["witness"], bound["record_cid"]);
    success(appoint(&f));
}

#[test]
fn distinct_binding_rulings_are_refused_instead_of_choosing_latest() {
    let f = fixture();
    let mut original = witness_event(&f);
    let before = records(f.dir.path());
    let second = cli(
        f.dir.path(),
        &[
            "flow",
            "note",
            "--on",
            &f.commitment,
            "--kind",
            "ruling",
            "--bind-actor",
            &f.implementer,
            "--session",
            "implementation",
            "--reason",
            "a distinct ruling",
        ],
    );
    assert!(!second.status.success());
    assert_eq!(records(f.dir.path()), before);
    // Simulate a malformed/foreign sidecar producer with a valid CID, not raw-byte corruption.
    original.classified_as.retain(|s| !s.starts_with("reason:"));
    original
        .classified_as
        .push("reason:another producer's conflicting ruling".into());
    SidecarFlowStore::open(f.dir.path())
        .unwrap()
        .append(FlowRecord::Event(original))
        .unwrap();
    let observed = eligibility(&f);
    assert_eq!(observed["eligible"], false);
    assert!(observed["issues"].to_string().contains("conflicting"));
    assert!(!appoint(&f).status.success());
}

#[test]
fn malformed_or_forged_witnesses_are_not_filtered_out_of_the_shared_reader() {
    use elohim_epr_rea::{AgentRef, Magnitude};
    for fault in [
        "author",
        "provider",
        "scope",
        "unregistered",
        "duplicate-actor",
        "duplicate-binding",
        "duplicate-scope",
        "appointment",
        "purpose",
        "fulfills",
        "quantity",
        "kind",
        "missing-author",
        "duplicate-reason",
        "duplicate-steward",
        "duplicate-binding-pin",
        "receiver",
        "repository-scope",
        "process",
        "satisfies",
        "zero-quantity",
    ] {
        let f = fixture();
        let mut forged = witness_event(&f);
        match fault {
            "author" => {
                forged
                    .classified_as
                    .retain(|s| !s.starts_with("actor-claim:"));
                forged
                    .classified_as
                    .push(format!("actor-claim:{}", f.acceptor));
            }
            "provider" => forged.provider = AgentRef("agent:other@gpt-6".into()),
            "scope" => {
                forged
                    .classified_as
                    .retain(|s| !s.starts_with("scope-cid:"));
                forged
                    .classified_as
                    .push(format!("scope-cid:{}", flow::body_cid("other scope")));
            }
            "unregistered" => {
                forged
                    .classified_as
                    .retain(|s| !s.starts_with("actor-claim:") && !s.starts_with("bind-actor:"));
                let unknown = flow::body_cid("unknown actor");
                forged.classified_as.extend([
                    format!("actor-claim:{unknown}"),
                    format!("bind-actor:{unknown}"),
                ]);
            }
            "duplicate-actor" => forged
                .classified_as
                .push(format!("actor-claim:{}", f.implementer)),
            "duplicate-binding" => forged
                .classified_as
                .push(format!("bind-actor:{}", f.implementer)),
            "duplicate-scope" => forged.classified_as.push(format!(
                "scope-cid:{}",
                flow::body_cid("Original bounded assertion")
            )),
            "appointment" => forged.classified_as.push(format!("appoint:{}", f.acceptor)),
            "purpose" => forged.classified_as.push("purpose:acceptance".into()),
            "fulfills" => forged.fulfills.push(f.commitment.parse().unwrap()),
            "quantity" => {
                forged.quantity = Magnitude::Count {
                    value: 1.,
                    unit: "artifact".into(),
                }
            }
            "kind" => forged.classified_as[0] = "run:observation".into(),
            "duplicate-reason" => forged.classified_as.push("reason:duplicate".into()),
            "duplicate-steward" => forged.classified_as.push("steward:duplicate".into()),
            "duplicate-binding-pin" => forged.classified_as.extend([
                "implementer-binding:duplicate".into(),
                "implementer-binding:duplicate".into(),
            ]),
            "receiver" => forged.receiver = AgentRef("repo:other".into()),
            "repository-scope" => forged.in_scope_of = flow::body_cid("other repository"),
            "process" => forged.process = Some(flow::body_cid("not a process")),
            "satisfies" => forged.satisfies.push(f.commitment.parse().unwrap()),
            "zero-quantity" => {
                forged.quantity = Magnitude::Count {
                    value: 0.,
                    unit: "run-note".into(),
                }
            }
            "missing-author" => forged
                .classified_as
                .retain(|s| !s.starts_with("actor-claim:")),
            _ => unreachable!(),
        }
        SidecarFlowStore::open(f.dir.path())
            .unwrap()
            .append(FlowRecord::Event(forged))
            .unwrap();
        assert_eq!(eligibility(&f)["eligible"], false, "{fault}");
        assert!(!appoint(&f).status.success(), "{fault}");
    }
}

#[test]
fn missing_or_corrupt_registered_actor_bytes_refuse_an_existing_binding() {
    for corrupt in ["missing", "malformed", "cid-mismatch"] {
        let f = fixture();
        success(bind(&f));
        let path = f.dir.path().join(".eprfs/status/actors.jsonl");
        match corrupt {
            "missing" => std::fs::remove_file(&path).unwrap(),
            "malformed" => std::fs::write(&path, "{corrupt actor log\n").unwrap(),
            "cid-mismatch" => {
                let bytes = std::fs::read_to_string(&path).unwrap();
                assert!(bytes.contains("agent:implementer@gpt-6"));
                std::fs::write(
                    &path,
                    bytes.replace("agent:implementer@gpt-6", "agent:forged@gpt-6"),
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        assert_eq!(eligibility(&f)["eligible"], false);
        assert!(!appoint(&f).status.success());
    }
}

fn prepare_delivery(f: &Fixture) -> (String, String) {
    use elohim_epr_rea::{AgentRef, FlowEvent, Magnitude};
    let root = f.dir.path();
    let scope = flow::body_cid("Original bounded assertion");
    std::fs::write(
        root.join("fulfillment.txt"),
        "The original delivered result",
    )
    .unwrap();
    let resource = flow::body_cid("The original delivered result");
    let fulfillment = SidecarFlowStore::open(root)
        .unwrap()
        .append(FlowRecord::Event(FlowEvent {
            action: ReaVerb::Produce,
            provider: AgentRef("agent:implementer@gpt-6".into()),
            receiver: flow::repo_agent(),
            resource,
            quantity: Magnitude::Count {
                value: 1.,
                unit: "artifact".into(),
            },
            process: None,
            in_scope_of: scope,
            fulfills: vec![f.commitment.parse().unwrap()],
            satisfies: vec![],
            classified_as: vec![],
            occurred_at: "2026-10-22T00:00:00Z".into(),
        }))
        .unwrap()
        .to_string();
    let review = success(cli(
        root,
        &[
            "flow",
            "note",
            "--on",
            &f.commitment,
            "--kind",
            "verdict",
            "--verdict",
            "approved",
            "--reason",
            "Technical review exercised the declared oracle",
        ],
    ));
    let report = serde_json::json!({
        "intent": "Original bounded assertion", "revision": "exact-fixture-revision", "environment": "disposable repository",
        "actions": ["Inspect the original delivered result"], "observations": ["The declared result is present"],
        "evidence": [{"path": "fulfillment.txt", "cid": resource.to_string()}], "limitations": [],
    });
    std::fs::write(
        root.join("acceptance.json"),
        serde_json::to_vec(&report).unwrap(),
    )
    .unwrap();
    (fulfillment, review["record_cid"].as_str().unwrap().into())
}

fn accept(
    f: &Fixture,
    appointment: &str,
    fulfillment: &str,
    review: &str,
    session: &str,
) -> Output {
    cli(
        f.dir.path(),
        &[
            "flow",
            "note",
            "--on",
            &f.commitment,
            "--kind",
            "verdict",
            "--verdict",
            "approved",
            "--purpose",
            "acceptance",
            "--appointment",
            appointment,
            "--fulfillment",
            fulfillment,
            "--review",
            review,
            "--report",
            "acceptance.json",
            "--session",
            session,
            "--reason",
            "Independent exercise of the exact delivered result",
        ],
    )
}

#[test]
fn binding_preserves_original_fulfillment_and_requires_independent_acceptance() {
    let f = fixture();
    let root = f.dir.path();
    let (fulfillment, review) = prepare_delivery(&f);
    let original = records(root);
    let report_bytes = std::fs::read(root.join("fulfillment.txt")).unwrap();
    assert!(!appoint(&f).status.success());
    success(bind(&f));
    let appointment = success(appoint(&f));
    let appointed = appointment["record_cid"].as_str().unwrap();
    assert!(
        !accept(&f, appointed, &fulfillment, &review, "implementation")
            .status
            .success()
    );
    let accepted = success(accept(&f, appointed, &fulfillment, &review, "acceptance"));
    let observed = records(root);
    for old in &original {
        assert_eq!(observed.iter().find(|(id, _)| *id == old.0).unwrap(), old);
    }
    assert_eq!(
        std::fs::read(root.join("fulfillment.txt")).unwrap(),
        report_bytes
    );
    assert_eq!(
        observed
            .iter()
            .filter(|(_, r)| matches!(r, FlowRecord::Event(e) if !e.fulfills.is_empty()))
            .count(),
        1
    );
    let (_, FlowRecord::Event(event)) = observed
        .iter()
        .find(|(id, _)| id.to_string() == accepted["record_cid"].as_str().unwrap())
        .unwrap()
    else {
        panic!("acceptance")
    };
    flow::acceptance::validate_record(root, &observed, event).unwrap();
    let view = flow::reconciliation::reconcile(
        root,
        &flow::body_cid("Original bounded assertion"),
        Some("scope.md"),
        &observed,
    );
    assert_eq!(view.assertions[0].acceptance, "accepted");
    for (_, record) in observed
        .iter()
        .filter(|(id, _)| !original.iter().any(|(old, _)| old == id))
    {
        assert_eq!(
            flow::stocks::classify_record(record),
            flow::stocks::CommitmentFlow::Neither(flow::stocks::NotCounted::DischargesNothing)
        );
    }
}

#[test]
fn direct_session_claims_keep_their_original_pin_and_cannot_be_overridden() {
    let mut f = fixture();
    let root = f.dir.path();
    let original = records(root)
        .into_iter()
        .find_map(|(_, r)| match r {
            FlowRecord::Intent(i) => Some(i),
            _ => None,
        })
        .unwrap();
    let mut intent = original;
    intent.resource_spec.classified_as[1] = "fresh-session-claim".into();
    let intent_id = SidecarFlowStore::open(root)
        .unwrap()
        .append(FlowRecord::Intent(intent))
        .unwrap();
    let claimed = success(cli(
        root,
        &[
            "flow",
            "claim",
            "--on",
            &intent_id.to_string(),
            "--session",
            "implementation",
        ],
    ));
    f.commitment = claimed["commitment_cid"].as_str().unwrap().into();
    assert_eq!(eligibility(&f)["eligible"], true);
    assert_eq!(eligibility(&f)["actor_claim"], f.implementer);
    assert!(eligibility(&f)["witness"].is_null());
    assert!(!bind(&f).status.success());
    let appointment = success(appoint(&f));
    let observed = records(f.dir.path());
    let (_, FlowRecord::Event(event)) = observed
        .iter()
        .find(|(id, _)| id.to_string() == appointment["record_cid"].as_str().unwrap())
        .unwrap()
    else {
        panic!("appointment")
    };
    assert!(!event
        .classified_as
        .iter()
        .any(|s| s.starts_with("implementer-binding:")));
}

#[test]
fn later_binding_cannot_rehabilitate_unpinned_earlier_appointment_or_acceptance() {
    use elohim_epr_rea::{FlowEvent, Magnitude};
    let f = fixture();
    let root = f.dir.path();
    let (fulfillment, review) = prepare_delivery(&f);
    let earlier = FlowEvent {
        action: ReaVerb::Cite,
        provider: flow::repo_agent(),
        receiver: flow::repo_agent(),
        resource: f.commitment.parse().unwrap(),
        quantity: Magnitude::Count {
            value: 1.,
            unit: "run-note".into(),
        },
        process: None,
        in_scope_of: flow::repository_scope(root).unwrap(),
        fulfills: vec![],
        satisfies: vec![],
        classified_as: vec![
            "run:ruling".into(),
            f.commitment.clone(),
            "reason:legacy invalid appointment".into(),
            format!("appoint:{}", f.acceptor),
        ],
        occurred_at: "2026-10-22T00:00:00Z".into(),
    };
    let old_id = SidecarFlowStore::open(root)
        .unwrap()
        .append(FlowRecord::Event(earlier.clone()))
        .unwrap();
    assert!(flow::acceptance::validate_record(root, &records(root), &earlier).is_err());
    success(bind(&f));
    let error = flow::acceptance::validate_record(root, &records(root), &earlier)
        .unwrap_err()
        .to_string();
    assert!(error.contains("implementer-binding"), "{error}");
    assert!(
        !accept(&f, &old_id.to_string(), &fulfillment, &review, "acceptance")
            .status
            .success()
    );
    let fresh = success(appoint(&f));
    let decision = success(accept(
        &f,
        fresh["record_cid"].as_str().unwrap(),
        &fulfillment,
        &review,
        "acceptance",
    ));
    let current = records(root);
    let (_, FlowRecord::Event(accepted)) = current
        .iter()
        .find(|(id, _)| id.to_string() == decision["record_cid"].as_str().unwrap())
        .unwrap()
    else {
        panic!("acceptance")
    };
    let mut earlier_acceptance = accepted.clone();
    earlier_acceptance
        .classified_as
        .retain(|s| !s.starts_with("appointment:"));
    earlier_acceptance
        .classified_as
        .push(format!("appointment:{old_id}"));
    assert!(flow::acceptance::validate_record(root, &current, &earlier_acceptance).is_err());
    assert_eq!(
        current.iter().find(|(id, _)| *id == old_id).unwrap().1,
        FlowRecord::Event(earlier)
    );
}
