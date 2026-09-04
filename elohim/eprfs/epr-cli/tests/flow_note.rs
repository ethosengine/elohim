//! Integration tests over a synthetic tempdir repo for `epr flow note` — the run-scale write
//! leg (Spec A §3,
//! `genesis/docs/superpowers/specs/2026-08-13-run-plane-projection-observation-events-design.md`).
//!
//! Fixture shape is `flow_fulfill.rs`'s: a recipe with a `scenario` stage under a
//! `genesis/a2o/features/...` glob, `project`ed once so a real scenario `Commitment` exists to
//! annotate. That matters for the empty-`fulfills` invariant below — it is only a meaningful
//! assertion against a commitment that `status` actually counts.
//!
//! The `classified_as` CID-stability pin (plan Task 1 Step 4 item g) lives where the field does,
//! in `elohim/epr-rea/src/model.rs::tests::an_unclassified_event_keeps_its_pre_classified_cid`,
//! whose golden was computed against the pre-`classified_as` struct. The note-record golden lives
//! in `note::tests::note_event_cid_is_stable`.

use std::path::Path;

use elohim_epr_cli::flow::note::{note, NoteActor};
use elohim_epr_cli::flow::{project, walk, FlowError};
use elohim_epr_rea::{FlowRecord, FlowStore, ReaVerb, SidecarFlowStore};
use tempfile::TempDir;

const FEATURE: &str = "genesis/a2o/features/dataplane/resiliency-saga/05-node-loss.feature";

fn git(root: &Path, args: &[&str]) {
    let status = elohim_epr_cli::process::build_command("git", args, root, &[])
        .env("GIT_AUTHOR_NAME", "Fixture Author")
        .env("GIT_AUTHOR_EMAIL", "author@example.test")
        .env("GIT_COMMITTER_NAME", "Fixture Author")
        .env("GIT_COMMITTER_EMAIL", "author@example.test")
        .output()
        .expect("git runs");
    assert!(status.status.success(), "git {args:?} failed");
}

fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

/// A committed tempdir repo carrying one feature file. `projected` mints the scenario
/// commitment; the raw fixture is used where the test must observe an untouched sidecar.
fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    write(
        root,
        ".claude/epr-meta/recipes.yaml",
        r#"version: 1
recipes:
  - id: resiliency-saga
    version: 1
    description: fixture saga
    stages:
      - name: scenario
        artifactKind: "a2o:feature"
        paths:
          - "genesis/a2o/features/dataplane/resiliency-saga/**/*.feature"
    edges: []
"#,
    );
    write(
        root,
        FEATURE,
        "Feature: Chapter five — node loss\n  @concern:resiliency\n  Scenario: data survives\n    Given a node dies\n    Then the data survives\n",
    );

    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn projected() -> TempDir {
    let dir = fixture();
    let recipes = dir.path().join(".claude/epr-meta/recipes.yaml");
    project::project(dir.path(), &recipes).expect("project runs");
    dir
}

fn sidecar_lines(root: &Path) -> usize {
    std::fs::read_to_string(root.join(".eprfs/status/flows.jsonl"))
        .map(|t| t.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0)
}

fn note_events(root: &Path) -> Vec<elohim_epr_rea::FlowEvent> {
    SidecarFlowStore::open(root)
        .unwrap()
        .records()
        .unwrap()
        .into_iter()
        .filter_map(|(_, r)| match r {
            FlowRecord::Event(e)
                if e.classified_as
                    .first()
                    .is_some_and(|t| t.starts_with("run:")) =>
            {
                Some(e)
            }
            _ => None,
        })
        .collect()
}

// (a) ────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn a_note_appends_exactly_one_record_that_round_trips_with_its_cid_reverified() {
    let dir = projected();
    let root = dir.path();
    let before = sidecar_lines(root);

    let outcome = note(
        root,
        FEATURE,
        "correction",
        "the doorway probe is per-doorway, not matthew-only",
        None,
        None,
        &NoteActor::default(),
    )
    .expect("note runs");

    assert!(outcome.appended);
    assert_eq!(sidecar_lines(root), before + 1, "exactly one line appended");

    // `records()` re-verifies every stored CID on read, so a successful read IS the round-trip
    // assertion — a drifted encoding would surface as FabricError::Integrity, not as a diff.
    let notes = note_events(root);
    assert_eq!(notes.len(), 1);
    let event = &notes[0];
    assert_eq!(event.action, ReaVerb::Cite, "a note cites, never produces");
    assert_eq!(event.classified_as[0], "run:correction");
    assert_eq!(event.classified_as[1], FEATURE, "subject in slot 1");
    assert_eq!(
        event.classified_as[2],
        "reason:the doorway probe is per-doorway, not matthew-only"
    );
    assert_eq!(
        event.classified_as.len(),
        3,
        "no switched-to slot was given"
    );
    assert!(
        matches!(&event.quantity, elohim_epr_rea::Magnitude::Count { unit, value }
            if unit == "run-note" && *value == 1.0),
        "a distinct unit keeps notes out of every existing unit-keyed fold"
    );
    assert_eq!(event.occurred_at, outcome.occurred_at);
    assert!(
        !event.occurred_at.is_empty(),
        "occurred_at is the git HEAD date, never empty"
    );
    assert_eq!(event.provider.0, "author@example.test", "git HEAD author");
}

#[test]
fn a_failed_approach_carries_its_consequence_in_a_fourth_slot() {
    let dir = projected();
    let root = dir.path();
    note(
        root,
        FEATURE,
        "failed-approach",
        "Tried Tsit5 for the perturbation ODE, system is too stiff",
        Some("Kvaerno5"),
        None,
        &NoteActor::default(),
    )
    .expect("note runs");

    let event = note_events(root).pop().expect("one note");
    assert_eq!(event.classified_as[0], "run:failed-approach");
    assert_eq!(event.classified_as[3], "switched-to:Kvaerno5");
}

// (b) ────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn an_identical_note_appended_twice_is_one_record() {
    let dir = projected();
    let root = dir.path();
    let first = note(
        root,
        FEATURE,
        "observation",
        "same body",
        None,
        None,
        &NoteActor::default(),
    )
    .expect("first");
    let before = sidecar_lines(root);
    let second = note(
        root,
        FEATURE,
        "observation",
        "same body",
        None,
        None,
        &NoteActor::default(),
    )
    .expect("second");

    assert!(first.appended);
    assert!(!second.appended, "a byte-identical note is a true no-op");
    assert_eq!(first.record_cid, second.record_cid);
    assert_eq!(sidecar_lines(root), before, "no second line");
    assert_eq!(note_events(root).len(), 1);

    // …but a different body is a different observation, so it IS a second record.
    let other = note(
        root,
        FEATURE,
        "observation",
        "different body",
        None,
        None,
        &NoteActor::default(),
    )
    .expect("third");
    assert!(other.appended);
    assert_ne!(other.record_cid, first.record_cid);
    assert_eq!(note_events(root).len(), 2);
}

// (c) ────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn an_unresolvable_target_appends_nothing_and_is_unknown_resource() {
    let dir = projected();
    let root = dir.path();
    let before = sidecar_lines(root);

    let err = note(
        root,
        "genesis/does-not-exist.md",
        "correction",
        "why",
        None,
        None,
        &NoteActor::default(),
    )
    .expect_err("an unresolvable --on must refuse");
    assert!(matches!(err, FlowError::UnknownResource(_)), "got: {err:?}");
    assert_eq!(sidecar_lines(root), before, "nothing appended");

    // A well-formed CID that no record mints or names is equally unresolvable — never an orphan.
    let stranger = elohim_epr_rea::atom_cid(&"never-recorded".to_string()).unwrap();
    let err = note(
        root,
        &stranger.to_string(),
        "correction",
        "why",
        None,
        None,
        &NoteActor::default(),
    )
    .expect_err("an unknown atom CID must refuse");
    assert!(matches!(err, FlowError::UnknownResource(_)), "got: {err:?}");
    assert_eq!(sidecar_lines(root), before, "still nothing appended");
}

#[test]
fn a_known_atom_cid_resolves_without_a_path() {
    let dir = projected();
    let root = dir.path();
    // The scenario commitment's own record CID — the `--on <commitment-cid>` shape.
    let commitment_cid = SidecarFlowStore::open(root)
        .unwrap()
        .commitments()
        .unwrap()
        .first()
        .map(|(cid, _)| *cid)
        .expect("project minted a scenario commitment");

    let outcome = note(
        root,
        &commitment_cid.to_string(),
        "correction",
        "scoped to the commitment itself",
        None,
        None,
        &NoteActor::default(),
    )
    .expect("a known commitment CID resolves");
    assert_eq!(outcome.on, commitment_cid.to_string());
    assert_eq!(outcome.resource, commitment_cid.to_string());
}

// (d) ────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn an_empty_reason_is_refused_before_the_store_is_opened() {
    let dir = fixture(); // deliberately NOT projected — no sidecar exists yet
    let root = dir.path();
    assert!(
        !root.join(".eprfs").exists(),
        "fixture starts with no sidecar"
    );

    let err = note(
        root,
        FEATURE,
        "correction",
        "   \n\t ",
        None,
        None,
        &NoteActor::default(),
    )
    .expect_err("an empty --reason must refuse");
    assert!(
        matches!(err, FlowError::InvalidArguments(_)),
        "got: {err:?}"
    );
    assert!(
        !root.join(".eprfs").exists(),
        "the store must not even be opened — SidecarFlowStore::open creates the tree"
    );

    // An unknown --kind is refused at the same gate, for the same reason.
    let err = note(
        root,
        FEATURE,
        "regression",
        "real text",
        None,
        None,
        &NoteActor::default(),
    )
    .expect_err("a fourth kind must refuse");
    assert!(
        matches!(err, FlowError::InvalidArguments(_)),
        "got: {err:?}"
    );
    assert!(!root.join(".eprfs").exists());
}

// (e) ────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn the_outcome_carries_the_appended_records_cid() {
    let dir = projected();
    let root = dir.path();
    let outcome = note(
        root,
        FEATURE,
        "observation",
        "json consumers read this",
        None,
        None,
        &NoteActor::default(),
    )
    .expect("note runs");

    let appended = SidecarFlowStore::open(root)
        .unwrap()
        .records()
        .unwrap()
        .pop()
        .expect("at least one record");
    assert_eq!(
        outcome.record_cid,
        appended.0.to_string(),
        "--json emits the appended record's CID"
    );

    // The outcome must serialize (the `--json` path) without losing the optional half.
    let json = serde_json::to_string(&outcome).expect("serializes");
    assert!(json.contains(&outcome.record_cid));
    assert!(json.contains("run:observation"));
}

// (f) ────────────────────────────────────────────────────────────────────────────────────────

/// The load-bearing rule of the leg: a note ANNOTATES. If `fulfills` were populated, the note
/// would mark its own commitment fulfilled and silently retire live work, because both `walk`
/// and `fulfill` derive the discharged set as "any event whose `fulfills` names the commitment".
#[test]
fn a_note_never_discharges_its_commitment() {
    let dir = projected();
    let root = dir.path();

    let before = walk::status(root).expect("status runs");
    assert!(
        before.unfulfilled_total > 0,
        "fixture must have an open commitment for this assertion to mean anything"
    );

    note(
        root,
        FEATURE,
        "correction",
        "this must not retire the chapter",
        Some("a different approach"),
        None,
        &NoteActor::default(),
    )
    .expect("note runs");

    let after = walk::status(root).expect("status runs");
    assert_eq!(
        after.unfulfilled_total, before.unfulfilled_total,
        "a note must never move a commitment out of the unfulfilled set"
    );
    assert_eq!(after.events, before.events + 1, "the note IS an event");

    let event = note_events(root).pop().expect("one note");
    assert!(event.fulfills.is_empty(), "fulfills stays empty");
    assert!(event.satisfies.is_empty(), "satisfies stays empty");
}

// attribution ────────────────────────────────────────────────────────────────────────────────

const CLAIMED: &str = "agent:rust-architect@opus-5";

#[test]
fn a_named_identity_provides_the_note_and_keeps_the_steward_in_the_last_slot() {
    let dir = projected();
    let root = dir.path();

    let outcome = note(
        root,
        FEATURE,
        "correction",
        "the salvage join reads the wrong pillar",
        None,
        None,
        &NoteActor {
            as_ref: Some(CLAIMED.to_string()),
            session: None,
        },
    )
    .expect("note runs");

    let event = note_events(root).pop().expect("one note");
    assert_eq!(
        event.provider.0, CLAIMED,
        "a named identity provides the note outright"
    );
    assert_eq!(
        event.classified_as,
        vec![
            "run:correction".to_string(),
            FEATURE.to_string(),
            "reason:the salvage join reads the wrong pillar".to_string(),
            "steward:author@example.test".to_string(),
        ],
        "tag, subject, reason, then steward LAST — readers index the leading slots positionally"
    );
    assert_eq!(outcome.actor.as_deref(), Some(CLAIMED));
    assert_eq!(outcome.steward.as_deref(), Some("author@example.test"));
}

#[test]
fn the_steward_slot_stays_last_even_when_a_consequence_precedes_it() {
    let dir = projected();
    let root = dir.path();

    note(
        root,
        FEATURE,
        "failed-approach",
        "Tried the household-blind XOR salvage",
        Some("diversity-first multi-pass"),
        None,
        &NoteActor {
            as_ref: Some(CLAIMED.to_string()),
            session: None,
        },
    )
    .expect("note runs");

    let event = note_events(root).pop().expect("one note");
    assert_eq!(
        event.classified_as,
        vec![
            "run:failed-approach".to_string(),
            FEATURE.to_string(),
            "reason:Tried the household-blind XOR salvage".to_string(),
            "switched-to:diversity-first multi-pass".to_string(),
            "steward:author@example.test".to_string(),
        ]
    );
}

#[test]
fn a_malformed_identity_refuses_and_never_falls_back_to_the_git_author() {
    let dir = projected();
    let root = dir.path();
    let before = sidecar_lines(root);

    let err = note(
        root,
        FEATURE,
        "correction",
        "real text",
        None,
        None,
        &NoteActor {
            as_ref: Some("scribe@opus-5".to_string()),
            session: None,
        },
    )
    .expect_err("a malformed --as must refuse rather than substitute the author");

    assert!(
        err.to_string().contains("agent:<role>@<model>"),
        "the refusal must name the legal shape; got: {err}"
    );
    assert_eq!(sidecar_lines(root), before, "nothing appended");
    assert!(
        note_events(root).is_empty(),
        "a note attributed to a substituted actor is worse than no note"
    );
}

#[test]
fn a_registered_session_claim_provides_the_note_without_naming_it() {
    let dir = projected();
    let root = dir.path();
    let session = "session-abc123";

    elohim_epr_cli::actor::claim(root, CLAIMED, session).expect("claim registers");

    let outcome = note(
        root,
        FEATURE,
        "observation",
        "the sidecar answered for this run",
        None,
        None,
        &NoteActor {
            as_ref: None,
            session: Some(session.to_string()),
        },
    )
    .expect("note runs");

    let event = note_events(root).pop().expect("one note");
    assert_eq!(event.provider.0, CLAIMED, "the session's current claim");
    assert_eq!(
        event.classified_as.last().map(String::as_str),
        Some("steward:author@example.test"),
        "the signing human stays attached to an attribution that is no longer theirs"
    );
    assert_eq!(outcome.actor.as_deref(), Some(CLAIMED));
}

#[test]
fn a_session_that_claimed_nothing_falls_through_to_the_author_arm_without_failing() {
    let dir = projected();
    let root = dir.path();

    // No actor sidecar at all — the identity plane has never run in this tree.
    let unattributed = note(
        root,
        FEATURE,
        "observation",
        "same body either way",
        None,
        None,
        &NoteActor::default(),
    )
    .expect("the author arm runs");

    let unclaimed_session = note(
        root,
        FEATURE,
        "observation",
        "same body either way",
        None,
        None,
        &NoteActor {
            as_ref: None,
            session: Some("session-nobody-claimed".to_string()),
        },
    )
    .expect("an unregistered session is an answer, never a failure");

    assert_eq!(
        unclaimed_session.record_cid, unattributed.record_cid,
        "an unresolvable session must mint the SAME record the author arm does"
    );
    assert!(
        !unclaimed_session.appended,
        "identical record, so the append is a true no-op"
    );
    assert_eq!(unclaimed_session.actor, None);
    assert_eq!(unclaimed_session.steward, None);

    // …and the same holds once a sidecar EXISTS but holds nothing for this session: a claim
    // registered by a different run must never leak into it.
    elohim_epr_cli::actor::claim(root, CLAIMED, "some-other-session").expect("claim registers");
    let still_unclaimed = note(
        root,
        FEATURE,
        "observation",
        "same body either way",
        None,
        None,
        &NoteActor {
            as_ref: None,
            session: Some("session-nobody-claimed".to_string()),
        },
    )
    .expect("a populated sidecar with no matching claim is still an answer");
    assert_eq!(still_unclaimed.record_cid, unattributed.record_cid);
}

#[test]
fn an_unattributed_note_carries_no_steward_slot_and_omits_both_payload_keys() {
    let dir = projected();
    let root = dir.path();

    let outcome = note(
        root,
        FEATURE,
        "observation",
        "the arm this leg has always had",
        None,
        None,
        &NoteActor::default(),
    )
    .expect("note runs");

    let event = note_events(root).pop().expect("one note");
    assert_eq!(event.provider.0, "author@example.test");
    assert_eq!(event.classified_as.len(), 3, "no steward slot is appended");
    assert!(
        !event
            .classified_as
            .iter()
            .any(|slot| slot.starts_with("steward:")),
        "the author arm has no steward to add — it would only repeat the provider"
    );

    // The `--json` payload must be byte-identical to the one this leg emitted before the
    // attribution fields existed: omitted, never null.
    let json = serde_json::to_string(&outcome).expect("serializes");
    assert!(!json.contains("\"actor\""), "got: {json}");
    assert!(!json.contains("\"steward\""), "got: {json}");
}

// path confinement ───────────────────────────────────────────────────────────────────────────

#[test]
fn a_target_outside_the_repo_root_is_refused_rather_than_written_against() {
    let dir = projected();
    let root = dir.path();
    let before = sidecar_lines(root);
    let err = note(
        root,
        "../outside.md",
        "observation",
        "escape attempt",
        None,
        None,
        &NoteActor::default(),
    )
    .expect_err("a path escaping the root must refuse");
    assert!(
        matches!(err, FlowError::InvalidArguments(_)),
        "got: {err:?}"
    );
    assert_eq!(sidecar_lines(root), before);
}
