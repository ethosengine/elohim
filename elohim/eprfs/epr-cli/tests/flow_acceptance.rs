//! Appointed acceptance admission over independent actor claims and immutable evidence pins.
use elohim_epr_cli::{
    actor,
    flow::{
        self,
        acceptance::{validate_record, AcceptanceOptions},
        note::{note, note_with_options, NoteActor},
    },
};
use elohim_epr_rea::{
    AgentRef, Commitment, CommitmentState, FlowEvent, FlowRecord, FlowStore, Magnitude, ReaVerb,
    ResourceSpec, SidecarFlowStore,
};
use std::path::Path;
use tempfile::TempDir;

fn fixture() -> (TempDir, String, AcceptanceOptions) {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(root.join("scope.md"), "The intended feature").unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["add", "scope.md"],
        vec!["commit", "-qm", "fixture"],
    ] {
        let result = std::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_AUTHOR_NAME", "Fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
            .env("GIT_COMMITTER_NAME", "Fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.test")
            .output()
            .unwrap();
        assert!(result.status.success(), "{:?}", result);
    }
    let implementer = actor::claim(root, "agent:implementer@gpt-6", "implementation").unwrap();
    let acceptor = actor::claim(root, "agent:acceptor@gpt-6", "acceptance").unwrap();
    let mut store = SidecarFlowStore::open(root).unwrap();
    let promise = store
        .append(FlowRecord::Commitment(Commitment {
            action: ReaVerb::Produce,
            provider: AgentRef("agent:implementer@gpt-6".into()),
            receiver: flow::repo_agent(),
            resource_spec: ResourceSpec {
                classified_as: vec![format!("actor-claim:{}", implementer.record_cid)],
                quantity: None,
            },
            in_scope_of: flow::body_cid("The intended feature"),
            valid_from: None,
            valid_until: None,
            state: CommitmentState::Active,
            satisfies: vec![],
            bound: None,
        }))
        .unwrap();
    let fulfillment = store
        .append(FlowRecord::Event(FlowEvent {
            action: ReaVerb::Produce,
            provider: flow::repo_agent(),
            receiver: flow::repo_agent(),
            resource: flow::body_cid("Exercised the actual CLI"),
            quantity: Magnitude::Count {
                value: 1.,
                unit: "artifact".into(),
            },
            process: None,
            in_scope_of: flow::body_cid("The intended feature"),
            fulfills: vec![promise],
            satisfies: vec![],
            classified_as: vec![],
            occurred_at: "2026-09-09T00:00:00Z".into(),
        }))
        .unwrap();
    let on = promise.to_string();
    let review = note(
        root,
        &on,
        "verdict",
        "Technical review passed",
        None,
        Some("approved"),
        &NoteActor::default(),
    )
    .unwrap();
    let appointment = note_with_options(
        root,
        &on,
        "ruling",
        "Appoint independent acceptance",
        None,
        None,
        &NoteActor::default(),
        &AcceptanceOptions {
            appoint: Some(acceptor.record_cid),
            ..Default::default()
        },
    )
    .unwrap();
    std::fs::write(root.join("evidence.txt"), "Exercised the actual CLI").unwrap();
    let report = serde_json::json!({"intent":"Understand delivered behavior", "revision":"fixture", "environment":"isolated repository", "actions":["Run context"], "observations":["Uncertainty remains visible"], "evidence":[{"path":"evidence.txt","cid":flow::body_cid("Exercised the actual CLI").to_string()}], "limitations":[]});
    std::fs::write(root.join("acceptance.json"), report.to_string()).unwrap();
    (
        dir,
        on,
        AcceptanceOptions {
            purpose: Some("acceptance".into()),
            appointment: Some(appointment.record_cid),
            fulfillment: Some(fulfillment.to_string()),
            review: Some(review.record_cid),
            report: Some("acceptance.json".into()),
            ..Default::default()
        },
    )
}
fn accept(
    root: &Path,
    on: &str,
    options: &AcceptanceOptions,
) -> flow::FlowResult<flow::note::NoteOutcome> {
    note_with_options(
        root,
        on,
        "verdict",
        "Used the feature against its intent",
        None,
        Some("approved"),
        &NoteActor {
            session: Some("acceptance".into()),
            as_ref: None,
        },
        options,
    )
}

#[test]
fn admission_pins_evidence_and_refuses_self_acceptance() {
    let (dir, on, options) = fixture();
    let root = dir.path();
    let result = accept(root, &on, &options).unwrap();
    assert!(result.appended);
    assert!(!accept(root, &on, &options).unwrap().appended);
    let records = SidecarFlowStore::open(root).unwrap().records().unwrap();
    let (_, FlowRecord::Event(event)) = records
        .iter()
        .find(|(cid, _)| cid.to_string() == result.record_cid)
        .unwrap()
    else {
        panic!()
    };
    assert!(event.fulfills.is_empty());
    validate_record(root, &records, event).unwrap();
    std::fs::write(root.join("evidence.txt"), "Changed evidence").unwrap();
    assert!(validate_record(root, &records, event).is_err());
    assert!(accept(root, &on, &options).is_err());
    let bad = note_with_options(
        root,
        &on,
        "verdict",
        "self",
        None,
        Some("approved"),
        &NoteActor {
            session: Some("implementation".into()),
            as_ref: None,
        },
        &options,
    );
    assert!(bad.unwrap_err().to_string().contains("independent"));
}

#[test]
fn malformed_references_and_foreign_evidence_append_nothing() {
    let (dir, on, options) = fixture();
    let root = dir.path();
    let before = std::fs::read(root.join(".eprfs/status/flows.jsonl")).unwrap();
    let mut wrong = options.clone();
    wrong.review = options.fulfillment.clone();
    assert!(accept(root, &on, &wrong).is_err());
    wrong = options.clone();
    wrong.supersedes = options.review.clone();
    assert!(accept(root, &on, &wrong).is_err());
    wrong = options;
    wrong.report = Some("../outside.json".into());
    assert!(accept(root, &on, &wrong).is_err());
    assert_eq!(
        before,
        std::fs::read(root.join(".eprfs/status/flows.jsonl")).unwrap()
    );
}

#[test]
fn concurrent_acceptance_deduplicates_under_transaction() {
    let (dir, on, options) = fixture();
    let root = dir.path().to_path_buf();
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let root = root.clone();
            let on = on.clone();
            let options = options.clone();
            std::thread::spawn(move || accept(&root, &on, &options).unwrap().appended)
        })
        .collect();
    assert_eq!(
        threads
            .into_iter()
            .map(|t| t.join().unwrap())
            .filter(|appended| *appended)
            .count(),
        1
    );
}

#[test]
fn appointment_cannot_be_self_issued_and_acceptance_requires_exact_session() {
    let (dir, on, options) = fixture();
    let root = dir.path();
    let acceptor = actor::claim(root, "agent:acceptor@gpt-6", "acceptance").unwrap();
    let self_appointed = note_with_options(
        root,
        &on,
        "ruling",
        "I appoint myself",
        None,
        None,
        &NoteActor {
            session: Some("acceptance".into()),
            as_ref: None,
        },
        &AcceptanceOptions {
            appoint: Some(acceptor.record_cid),
            ..Default::default()
        },
    );
    assert!(self_appointed
        .unwrap_err()
        .to_string()
        .contains("appoint itself"));
    let no_pin = note_with_options(
        root,
        &on,
        "verdict",
        "named actor alone",
        None,
        Some("approved"),
        &NoteActor {
            as_ref: Some("agent:acceptor@gpt-6".into()),
            session: None,
        },
        &options,
    );
    assert!(no_pin
        .unwrap_err()
        .to_string()
        .contains("registered --session"));
    let mismatch = note_with_options(
        root,
        &on,
        "verdict",
        "wrong named actor",
        None,
        Some("approved"),
        &NoteActor {
            as_ref: Some("agent:other@gpt-6".into()),
            session: Some("acceptance".into()),
        },
        &options,
    );
    assert!(mismatch.unwrap_err().to_string().contains("differs"));
}

#[test]
fn explicit_supersession_preserves_old_decision_and_changes_requested_is_admissible() {
    let (dir, on, mut options) = fixture();
    let root = dir.path();
    let first = accept(root, &on, &options).unwrap();
    options.supersedes = Some(first.record_cid.clone());
    let changed = note_with_options(
        root,
        &on,
        "verdict",
        "Exercise revealed a limitation",
        None,
        Some("changes-requested"),
        &NoteActor {
            session: Some("acceptance".into()),
            as_ref: None,
        },
        &options,
    )
    .unwrap();
    let records = SidecarFlowStore::open(root).unwrap().records().unwrap();
    for id in [first.record_cid, changed.record_cid] {
        let (_, FlowRecord::Event(e)) = records
            .iter()
            .find(|(cid, _)| cid.to_string() == id)
            .unwrap()
        else {
            panic!()
        };
        validate_record(root, &records, e).unwrap();
        assert!(e.fulfills.is_empty());
    }
}

#[test]
fn report_must_pin_fulfillment_resource_and_missing_files_refuse() {
    let (dir, on, options) = fixture();
    let root = dir.path();
    let report_path = root.join("acceptance.json");
    let original = std::fs::read_to_string(&report_path).unwrap();
    let mut report: serde_json::Value = serde_json::from_str(&original).unwrap();
    std::fs::write(root.join("unrelated.txt"), "unrelated").unwrap();
    report["evidence"] =
        serde_json::json!([{"path":"unrelated.txt","cid":flow::body_cid("unrelated").to_string()}]);
    std::fs::write(&report_path, report.to_string()).unwrap();
    assert!(accept(root, &on, &options)
        .unwrap_err()
        .to_string()
        .contains("fulfillment resource"));
    std::fs::write(&report_path, original).unwrap();
    std::fs::remove_file(root.join("evidence.txt")).unwrap();
    assert!(accept(root, &on, &options).is_err());
}

fn current_acceptance(root: &Path) -> String {
    let view = flow::context::context(root, "scope.md").unwrap();
    let json = serde_json::to_value(&view).unwrap();
    let state = json["reconciliation"]["assertions"][0]["acceptance"]
        .as_str()
        .unwrap();
    assert!(view.reconciliation.render_text(3).contains(state));
    state.to_owned()
}

#[test]
fn accepted_reader_revalidates_changed_report_and_explicit_replacement_recovers() {
    let (dir, on, mut options) = fixture();
    let root = dir.path();
    assert_eq!(current_acceptance(root), "acceptance-unestablished");
    let first = accept(root, &on, &options).unwrap();
    assert_eq!(current_acceptance(root), "accepted");
    let path = root.join("acceptance.json");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    report["observations"] = serde_json::json!(["A second independent exercise"]);
    std::fs::write(path, report.to_string()).unwrap();
    assert_eq!(current_acceptance(root), "revalidation-required");
    options.supersedes = Some(first.record_cid);
    accept(root, &on, &options).unwrap();
    assert_eq!(current_acceptance(root), "accepted");
    std::fs::remove_file(root.join("evidence.txt")).unwrap();
    assert_eq!(current_acceptance(root), "revalidation-required");
}

#[test]
fn conflicting_valid_acceptances_are_not_resolved_by_time_or_append_order() {
    let (dir, on, options) = fixture();
    let root = dir.path();
    accept(root, &on, &options).unwrap();
    note_with_options(
        root,
        &on,
        "verdict",
        "Independent exercise requests changes",
        None,
        Some("changes-requested"),
        &NoteActor {
            session: Some("acceptance".into()),
            as_ref: None,
        },
        &options,
    )
    .unwrap();
    assert_eq!(current_acceptance(root), "contested");
}

#[test]
fn accepted_evidence_does_not_establish_an_unlocatable_current_source() {
    let (dir, on, options) = fixture();
    accept(dir.path(), &on, &options).unwrap();
    assert_eq!(current_acceptance(dir.path()), "accepted");
    let scope = flow::body_cid("The intended feature").to_string();
    let view = flow::context::context(dir.path(), &scope).unwrap();
    assert_eq!(
        view.reconciliation.assertions[0].acceptance,
        "revalidation-required"
    );
}

#[test]
fn acceptance_of_an_old_delivery_cannot_hide_new_production_before_the_decision() {
    let (dir, on, options) = fixture();
    let root = dir.path();
    let mut store = SidecarFlowStore::open(root).unwrap();
    let records = store.records().unwrap();
    let (_, FlowRecord::Event(produced)) = records
        .iter()
        .find(|(cid, _)| Some(cid.to_string()) == options.fulfillment)
        .unwrap()
    else {
        panic!()
    };
    let mut newer = produced.clone();
    newer.occurred_at = "2026-09-10T00:00:00Z".into();
    store.append(FlowRecord::Event(newer)).unwrap();
    accept(root, &on, &options).unwrap();
    assert_eq!(current_acceptance(root), "revalidation-required");
}

#[test]
fn conflicting_or_discharging_technical_review_cannot_authorize_acceptance() {
    let (dir, on, mut options) = fixture();
    let root = dir.path();
    let mut store = SidecarFlowStore::open(root).unwrap();
    let records = store.records().unwrap();
    let (_, FlowRecord::Event(review)) = records
        .iter()
        .find(|(cid, _)| Some(cid.to_string()) == options.review)
        .unwrap()
    else {
        panic!()
    };
    let mut conflicting = review.clone();
    conflicting
        .classified_as
        .push("verdict:changes-requested".into());
    options.review = Some(
        store
            .append(FlowRecord::Event(conflicting))
            .unwrap()
            .to_string(),
    );
    assert!(accept(root, &on, &options)
        .unwrap_err()
        .to_string()
        .contains("duplicate verdict"));
    let mut discharging = review.clone();
    discharging.fulfills.push(on.parse().unwrap());
    options.review = Some(
        store
            .append(FlowRecord::Event(discharging))
            .unwrap()
            .to_string(),
    );
    assert!(accept(root, &on, &options)
        .unwrap_err()
        .to_string()
        .contains("non-discharging"));
}
