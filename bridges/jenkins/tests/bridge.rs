//! The consuming-app bridge end to end, over temp repositories with their OWN affiliations
//! (`human:alice` standing, `human:bob` fixture) and Jenkins' real archive for edge #1483
//! (SUCCESS) and #1484 (FAILURE), with the orchestrator runs #1903 and #1906 that dispatched them.
//! No verdict is ever written for a real Steward.

use std::path::Path;

use elohim_epr_cli::{
    actor,
    flow::{memory, note, project, walk},
};
use elohim_epr_rea::{FlowRecord, FlowStore, ReaVerb, SidecarFlowStore};
use eprfs_agent::memory::{Affiliation, AffiliationStanding, FileRef, MemberKind, MembershipRole};
use jenkins_bridge::{
    card, drift::Finding, observe, observe::DEFAULT_OFFER, observe::DEFAULT_RECIPES, BridgeError,
    BuildInputs, Context, ExternalClaim, Observation, SignatureStatus, Standing,
};

const COLLECTIVE: &str = include_str!("../../../.epr-meta/collective.json");
const RECIPES: &str = include_str!("../../../.claude/epr-meta/recipes.yaml");
const OFFER: &str = include_str!("../../.epr-meta/offers/jenkins-edge-pipeline.offer.md");
/// The offer's text frozen at the version the address goldens were pinned against. Every event is
/// `in_scope_of` its governing offer, so a legitimate edit to the live offer (which re-addresses
/// it and needs fresh approval) moves event CIDs; address-stability tests pin this copy instead.
const OFFER_GOLDEN: &str = include_str!("fixtures/offer-golden.offer.md");
const W1483: &str = include_str!("fixtures/edge-1483.wfapi.json");
const W1484: &str = include_str!("fixtures/edge-1484.wfapi.json");
const G1903: &str = include_str!("fixtures/orchestrator-1903.actual-build-graph.json");
const G1906: &str = include_str!("fixtures/orchestrator-1906.actual-build-graph.json");
/// Orchestrator #1910 archived a still-queued sibling (elohim-storybook) as `buildNumber: null`.
const W1485: &str = include_str!("fixtures/edge-1485.wfapi.json");
const G1910: &str = include_str!("fixtures/orchestrator-1910.actual-build-graph.json");
const AUTHOR: &str = "agent:implementer@claude-opus-5-5";

fn write(root: &Path, path: &str, text: &str) {
    let file = root.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, text).unwrap();
}

fn git(root: &Path, args: &[&str]) {
    let out = elohim_epr_cli::process::build_command("git", args, root, &[])
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.test")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn affiliate(root: &Path, member: &str, standing: AffiliationStanding) {
    affiliate_line(root, member, standing, None);
}

/// A member withdrawing themselves: leaving is never gated, the line names its own member.
fn withdraw_self(root: &Path, member: &str, standing: AffiliationStanding) {
    affiliate_line(root, member, standing, Some("2026-09-26T00:00:00Z"));
}

fn affiliate_line(
    root: &Path,
    member: &str,
    standing: AffiliationStanding,
    withdrawn: Option<&str>,
) {
    use std::io::Write;
    let bytes = std::fs::read(root.join(".epr-meta/collective.json")).unwrap();
    let record = Affiliation {
        version: 1,
        collective: FileRef {
            path: ".epr-meta/collective.json".into(),
            cid: eprfs_core::BlobCid::compute_raw(&bytes).to_string(),
        },
        member: member.into(),
        member_kind: MemberKind::Person,
        role: MembershipRole::Steward,
        // human:alice is the genesis Steward; every later line is sponsored by her.
        sponsor: match withdrawn {
            Some(_) => Some(member.to_string()),
            None => (member != "human:alice").then(|| "human:alice".to_string()),
        },
        acts_for: None,
        standing,
        since: "2026-09-25T00:00:00Z".into(),
        withdrawn: withdrawn.map(str::to_string),
    };
    let path = root.join(memory::AFFILIATIONS_PATH);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    writeln!(file, "{}", memory::affiliation_line(&record).unwrap()).unwrap();
}

/// A repository with the real collective declaration, recipes and offer, but its own Stewards.
fn fixture_with(offer: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, ".epr-meta/collective.json", COLLECTIVE);
    write(root, DEFAULT_RECIPES, RECIPES);
    write(root, DEFAULT_OFFER, offer);
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "fixture"]);
    affiliate(root, "human:alice", AffiliationStanding::Standing);
    affiliate(root, "human:bob", AffiliationStanding::Fixture);
    actor::claim(root, "human:alice", "alice").unwrap();
    actor::claim(root, "human:bob", "bob").unwrap();
    project::project(root, &root.join(DEFAULT_RECIPES)).unwrap();
    dir
}

fn fixture() -> tempfile::TempDir {
    fixture_with(OFFER)
}

fn ctx(root: &Path) -> Context {
    Context::load(root, DEFAULT_OFFER, DEFAULT_RECIPES).unwrap()
}

fn verdict(root: &Path, cid: &str, session: &str, outcome: &str) {
    note::note(
        root,
        cid,
        "verdict",
        "fixture verdict on the offer",
        None,
        Some(outcome),
        &note::NoteActor {
            as_ref: None,
            session: Some(session.into()),
        },
    )
    .unwrap();
}

fn inputs(stages: &str, graph: Option<&str>) -> BuildInputs {
    BuildInputs {
        stages: stages.into(),
        graph: graph.map(str::to_string),
        jenkins_base: jenkins_bridge::translate::DEFAULT_JENKINS.into(),
    }
}

fn sidecar_sha(root: &Path) -> String {
    let bytes = std::fs::read(root.join(".eprfs/status/flows.jsonl")).unwrap_or_default();
    eprfs_core::BlobCid::compute_raw(&bytes).to_string()
}

/// Minted and approved by the fixture co-steward: active at Bootstrap stakes.
fn active() -> (tempfile::TempDir, String) {
    active_with(OFFER)
}

fn active_with(offer: &str) -> (tempfile::TempDir, String) {
    let dir = fixture_with(offer);
    let c = ctx(dir.path());
    let (cid, _) = jenkins_bridge::mint_offer(&c).unwrap();
    verdict(dir.path(), &cid.to_string(), "bob", "approved");
    (dir, cid.to_string())
}

#[test]
fn a_stewardless_collectives_offer_is_never_active() {
    let (dir, _) = active();
    let root = dir.path();
    let c = ctx(root);
    let Standing::Minted(s) = c.standing().unwrap() else {
        panic!("minted")
    };
    assert!(s.is_active());
    // Both Stewards leave: leaving is never gated, and the collective reads stewardless.
    withdraw_self(root, "human:bob", AffiliationStanding::Fixture);
    withdraw_self(root, "human:alice", AffiliationStanding::Standing);
    let Standing::Minted(s) = c.standing().unwrap() else {
        panic!("minted")
    };
    assert!(!s.is_active());
    assert!(s.stewardless);
    assert!(
        s.stakes_line().contains("stewardless"),
        "{}",
        s.stakes_line()
    );
    assert!(s.missing.as_deref().unwrap().contains("stewardless"));
    assert!(
        s.ignored.iter().any(|i| i.contains("stewardless")),
        "{:?}",
        s.ignored
    );
    let err = observe(&c, &inputs(W1483, Some(G1903))).unwrap_err();
    assert_eq!(err.exit_code(), 3, "{err}");
}

#[test]
fn a_service_can_never_claim_or_author() {
    let dir = fixture();
    let root = dir.path();
    assert!(actor::claim(root, "service:jenkins", "ci").is_err());
    let authored = note::note(
        root,
        DEFAULT_OFFER,
        "observation",
        "jenkins speaking for itself",
        None,
        None,
        &note::NoteActor {
            as_ref: Some("service:jenkins".into()),
            session: None,
        },
    );
    assert!(authored.is_err(), "a service cannot author a note");
}

#[test]
fn observe_refuses_with_exit_3_while_the_offer_is_unminted_or_proposed_and_writes_nothing() {
    let dir = fixture();
    let c = ctx(dir.path());
    let before = sidecar_sha(dir.path());
    let err = observe(&c, &inputs(W1483, Some(G1903))).unwrap_err();
    assert_eq!(err.exit_code(), 3, "{err}");
    assert!(err.to_string().contains("not minted"), "{err}");
    assert_eq!(sidecar_sha(dir.path()), before);

    let (cid, minted) = jenkins_bridge::mint_offer(&c).unwrap();
    assert!(minted);
    assert!(
        !jenkins_bridge::mint_offer(&c).unwrap().1,
        "minting is idempotent"
    );
    let before = sidecar_sha(dir.path());
    let err = observe(&c, &inputs(W1483, Some(G1903))).unwrap_err();
    assert_eq!(err.exit_code(), 3);
    let message = err.to_string();
    assert!(message.contains("proposed"), "{message}");
    assert!(
        message.contains("--kind verdict --verdict approved"),
        "{message}"
    );
    assert!(message.contains(&cid.to_string()), "{message}");
    assert_eq!(sidecar_sha(dir.path()), before, "a refusal writes nothing");
}

#[test]
fn the_author_cannot_approve_its_own_offer_and_a_fixture_approval_reads_bootstrap() {
    let offer = OFFER.replace(&format!("author: {AUTHOR}"), "author: human:alice");
    assert_ne!(offer, OFFER);
    let dir = fixture_with(&offer);
    let c = ctx(dir.path());
    let (cid, _) = jenkins_bridge::mint_offer(&c).unwrap();
    verdict(dir.path(), &cid.to_string(), "alice", "approved");
    let Standing::Minted(s) = c.standing().unwrap() else {
        panic!("minted")
    };
    assert!(!s.is_active(), "self-approval never activates");
    assert_eq!(s.ignored.len(), 1, "{:?}", s.ignored);

    verdict(dir.path(), &cid.to_string(), "bob", "approved");
    let Standing::Minted(s) = c.standing().unwrap() else {
        panic!("minted")
    };
    assert!(s.is_active());
    let approval = s.approval.as_ref().unwrap();
    assert_eq!(approval.approver, "human:bob");
    assert_eq!(approval.validated_at, "bootstrap (fixture co-steward)");
    assert!(s.stakes_line().contains("bootstrap (fixture co-steward)"));
}

#[test]
fn a_stewards_contrary_verdict_withdraws_and_observe_refuses_again() {
    let (dir, cid) = active();
    let c = ctx(dir.path());
    assert!(observe(&c, &inputs(W1483, Some(G1903))).is_ok());
    verdict(dir.path(), &cid, "alice", "changes-requested");
    let before = sidecar_sha(dir.path());
    let err = observe(&c, &inputs(W1484, Some(G1906))).unwrap_err();
    assert_eq!(err.exit_code(), 3);
    assert!(err.to_string().contains("withdrawn"), "{err}");
    assert_eq!(sidecar_sha(dir.path()), before);
}

#[test]
fn a_network_crossing_offer_refuses_a_fixture_approval() {
    let offer = OFFER.replace("crossesNetwork: false", "crossesNetwork: true");
    let dir = fixture_with(&offer);
    let c = ctx(dir.path());
    let (cid, _) = jenkins_bridge::mint_offer(&c).unwrap();
    verdict(dir.path(), &cid.to_string(), "bob", "approved");
    let Standing::Minted(s) = c.standing().unwrap() else {
        panic!("minted")
    };
    assert!(
        !s.is_active(),
        "a fixture co-steward can never carry a network crossing"
    );
    assert!(
        s.ignored.iter().any(|i| i.contains("non-fixture")),
        "{:?}",
        s.ignored
    );
}

#[test]
fn observe_appends_idempotently_and_walk_renders_each_stage_with_its_duration() {
    let (dir, offer_cid) = active();
    let root = dir.path();
    let c = ctx(root);
    let first = observe(&c, &inputs(W1483, Some(G1903))).unwrap();
    // 19 stages declared-and-reported; 2 NOT_EXECUTED produce no event; +1 Process.
    assert_eq!(first.appended, 17 + 1, "{first:?}");
    assert!(first.stakes.contains("bootstrap (fixture co-steward)"));
    let again = observe(&c, &inputs(W1483, Some(G1903))).unwrap();
    assert_eq!((again.appended, again.present), (0, 18));
    assert_eq!(again.process, first.process, "same inputs, same CIDs");

    let failed = observe(&c, &inputs(W1484, Some(G1906))).unwrap();
    let walked = walk::walk_cid(root, &failed.process.parse().unwrap()).unwrap();
    assert_eq!(walked.head, "edge-pipeline@1");
    assert_eq!(walked.in_scope_of, offer_cid);
    let storage = walked
        .events
        .iter()
        .find(|e| e.classified_as.get(1).map(String::as_str) == Some("Build Storage"))
        .expect("the failed stage is walked");
    assert_eq!(storage.action, "dismiss");
    assert_eq!(storage.classified_as[0], "result:failure");
    assert_eq!(storage.quantity, "291830 ci-stage-ms");
    assert!(storage
        .classified_as
        .contains(&"sha:c84af3f27b555bde3bcc870fe4eb6e1f6e240175".to_string()));
    assert!(walked
        .events
        .iter()
        .any(|e| e.classified_as[0] == "result:unstable"));

    // Every record this bridge wrote is an observation, provenance service:jenkins, discharging
    // nothing, scoped to the offer.
    for (_, record) in SidecarFlowStore::open(root).unwrap().records().unwrap() {
        match record {
            FlowRecord::Event(e) if e.provider.0 == "service:jenkins" => {
                assert!(e.fulfills.is_empty() && e.satisfies.is_empty() && e.process.is_none());
                assert_eq!(e.in_scope_of.to_string(), offer_cid);
                assert!(e.classified_as.contains(&"reach:private".to_string()));
                assert!(e
                    .classified_as
                    .contains(&"signature:unattested".to_string()));
                assert!(matches!(e.action, ReaVerb::Produce | ReaVerb::Dismiss));
            }
            _ => {}
        }
    }
}

#[test]
fn malformed_or_foreign_input_refuses_with_the_sidecar_byte_identical() {
    let (dir, _) = active();
    let c = ctx(dir.path());
    let before = sidecar_sha(dir.path());
    let running = W1483.replacen("\"status\": \"SUCCESS\"", "\"status\": \"IN_PROGRESS\"", 1);
    for (stages, graph, why) in [
        ("{ not json", None, "malformed wfapi"),
        (
            W1483,
            Some(G1906),
            "a graph that dispatched a different build",
        ),
        (running.as_str(), None, "an unfinished build"),
        (
            W1483,
            Some("{\"schemaVersion\":\"2\"}"),
            "an unknown graph schema",
        ),
    ] {
        let err = observe(&c, &inputs(stages, graph)).expect_err(why);
        assert_eq!(err.exit_code(), 65, "{why}: {err}");
        assert!(matches!(err, BridgeError::Malformed(_)));
        assert_eq!(sidecar_sha(dir.path()), before, "{why} must write nothing");
    }
}

#[test]
fn translate_output_can_only_express_observations() {
    let (dir, _) = active();
    let c = ctx(dir.path());
    for (stages, graph) in [(W1483, Some(G1903)), (W1484, Some(G1906)), (W1484, None)] {
        let t = c.translate(&inputs(stages, graph)).unwrap();
        for observation in t.observations {
            // Exhaustive, no wildcard: a third variant would fail to compile here.
            let record = match observation {
                Observation::Process(p) => FlowRecord::Process(p),
                Observation::Event(e) => FlowRecord::Event(e),
            };
            assert!(
                matches!(record, FlowRecord::Process(_) | FlowRecord::Event(_)),
                "never a Commitment, Spec, Intent or Edge"
            );
        }
    }
}

/// The Process carries the build's slots and its own external-claim envelope, readable back with
/// the shared reader — while every stage EVENT keeps the exact address it had before the Process
/// grew a classification (goldens from dev 914e8aa87: the first stage event of #1483 and the last
/// of #1484; all 17+19+19 event records were diffed byte-identical when the envelope moved).
#[test]
fn the_process_carries_the_build_and_its_envelope_and_events_keep_their_address() {
    let (dir, _) = active_with(OFFER_GOLDEN);
    let c = ctx(dir.path());
    let offer = c.offer_cid().unwrap();
    for (stages, graph, build, result, event_golden) in [
        (
            W1483,
            G1903,
            1483,
            "result:success",
            "bafyreifp33g3ipkquuzvhrz36seu4falt6bltk3vkkgigzcqfypbanlyni",
        ),
        (
            W1484,
            G1906,
            1484,
            "result:failure",
            "bafyreibhom3jg3njjjgylnc43si633n2ga4s5ny2vnlp33ayqtrcfptm5y",
        ),
    ] {
        let t = c.translate(&inputs(stages, Some(graph))).unwrap();
        let Some(Observation::Process(process)) = t.observations.last() else {
            panic!("the Process is appended last");
        };
        let slots = &process.classified_as;
        assert_eq!(slots[0], result);
        assert_eq!(slots[1], format!("elohim-edge#{build}"));
        let claim = ExternalClaim::read(slots)
            .unwrap()
            .expect("the Process is an external claim");
        assert_eq!(claim.source(), "service:jenkins");
        assert_eq!(claim.in_scope_of(), offer);
        assert_eq!(claim.signature(), &SignatureStatus::Unattested);
        assert_eq!(claim.provenance(), t.url);
        for slot in [
            format!("build:{build}"),
            format!("url:{}", t.url),
            format!("sha:{}", t.sha.as_deref().unwrap()),
        ] {
            assert!(slots.contains(&slot), "{slot} on the Process");
        }
        let events: Vec<String> = t
            .observations
            .iter()
            .filter(|o| matches!(o, Observation::Event(_)))
            .map(|o| o.cid().unwrap().to_string())
            .collect();
        assert!(
            events.contains(&event_golden.to_string()),
            "stage events of #{build} moved address"
        );
        assert_eq!(
            process.outputs.len(),
            events.len(),
            "the Process groups every event"
        );
    }
}

#[test]
fn a_graph_with_a_still_queued_sibling_pipeline_translates() {
    // Real archive: #1910 dispatched elohim-edge #1485 while elohim-storybook was still queued.
    assert!(
        G1910.contains("\"buildNumber\": null"),
        "fixture no longer has a queued sibling"
    );
    let (dir, _) = active();
    let c = ctx(dir.path());
    let t = c.translate(&inputs(W1485, Some(G1910))).unwrap();
    assert_eq!(t.basis, "graph");
    assert!(t.sha.is_some());
    assert!(c.drift_of(&t).is_clean());
}

#[test]
fn wfapi_alone_translates_and_says_so() {
    let (dir, _) = active();
    let c = ctx(dir.path());
    let t = c.translate(&inputs(W1484, None)).unwrap();
    assert_eq!(t.basis, "wfapi-only");
    assert_eq!(
        t.url,
        "https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1484/"
    );
    assert!(t.sha.is_none());
}

#[test]
fn drift_over_the_real_builds_is_clean_and_names_skips_and_unexercised_disclosures() {
    let (dir, _) = active();
    let c = ctx(dir.path());
    let success = c.drift_of(&c.translate(&inputs(W1483, Some(G1903))).unwrap());
    assert!(success.is_clean(), "{:?}", success.findings);
    assert!(success.findings.contains(&Finding::NotExecuted {
        stage: "Deploy Edge Node - Prod".into()
    }));
    let failure = c.drift_of(&c.translate(&inputs(W1484, Some(G1906))).unwrap());
    assert!(failure.is_clean(), "{:?}", failure.findings);

    // Masking: the same build against an offer that hides the deploy credentials.
    let hidden = OFFER.replace(
        "    - id: deploy:kube-credentials\n",
        "    - id: deploy:hidden\n",
    );
    assert_ne!(hidden, OFFER);
    let dir = fixture_with(&hidden);
    let c = ctx(dir.path());
    let masked = c.drift_of(&c.translate(&inputs(W1483, Some(G1903))).unwrap());
    assert!(masked.findings.contains(&Finding::UndisclosedCapability {
        capability: "deploy:kube-credentials".into(),
        stage: "Deploy Edge Node - Alpha".into()
    }));
}

#[test]
fn the_card_is_legible_before_approval_and_prints_the_stakes_after() {
    let dir = fixture();
    let c = ctx(dir.path());
    let stewards =
        memory::execute(dir.path(), "collective", None, None).unwrap()["stewards"].clone();
    let unminted = card::render(&c, stewards.clone(), None).unwrap();
    assert_eq!(unminted["offer"]["state"], "unminted");

    let (cid, _) = jenkins_bridge::mint_offer(&c).unwrap();
    verdict(dir.path(), &cid.to_string(), "bob", "approved");
    let t = c.translate(&inputs(W1484, Some(G1906))).unwrap();
    let d = c.drift_of(&t);
    let value = card::render(
        &c,
        stewards,
        Some(card::LastObserved {
            translation: &t,
            drift: &d,
        }),
    )
    .unwrap();
    assert_eq!(value["offer"]["state"], "active");
    assert_eq!(
        value["offer"]["validatedAt"],
        "bootstrap (fixture co-steward)"
    );
    assert_eq!(
        value["buildDescription"],
        format!(
            "governed by collective:ethosengine/elohim · offer {cid} · bootstrap (fixture co-steward)"
        )
    );
    assert!(value["verify"][0]
        .as_str()
        .unwrap()
        .starts_with("epr flow walk "));
    assert!(value["feedback"]["route"]
        .as_str()
        .unwrap()
        .contains(&cid.to_string()));
    assert_eq!(value["lastObservedGap"]["build"], 1484);
    assert!(value["disclosure"]["inflows"][0]["presence"]
        .as_str()
        .unwrap()
        .starts_with("presence:"));
}

/// Stage the end-to-end receipt fixture at `$JENKINS_BRIDGE_RECEIPT_DIR` (a fresh, empty directory)
/// and keep it: the real collective, recipes and offer, the recipe projected, the offer minted and
/// approved by the fixture co-steward `human:bob`. The CLI is then run over it by hand:
/// `JENKINS_BRIDGE_RECEIPT_DIR=<dir> cargo test --test bridge stage_receipt -- --ignored`.
#[test]
#[ignore = "stages a kept fixture for the manual receipt; needs JENKINS_BRIDGE_RECEIPT_DIR"]
fn stage_receipt() {
    let target = std::path::PathBuf::from(
        std::env::var("JENKINS_BRIDGE_RECEIPT_DIR").expect("JENKINS_BRIDGE_RECEIPT_DIR"),
    );
    let (dir, cid) = active();
    let kept = dir.keep();
    std::fs::rename(&kept, &target).expect("target must not exist and share a filesystem");
    println!("receipt fixture at {} — offer {cid}", target.display());
}
