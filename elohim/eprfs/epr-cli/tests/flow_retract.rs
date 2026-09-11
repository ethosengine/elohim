//! `epr flow retract` — withdrawing one sidecar dependency slot.
//!
//! Station six of `genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md`.
//! The residue these tests describe is real: stations 3b and 5 deleted the Python recall suite and
//! moved the recall contract, leaving sealed `DepEdge` records with both endpoints unreadable that
//! `hold`, `reseal` and `cites verify` cannot reach. The assertions below are the contract that
//! withdrawal is a RECORDED act rather than a deletion — the record survives, the concern does not,
//! and the count of withdrawals is visible in the same view that stopped showing them.

use std::path::Path;

use elohim_epr_cli::flow::concerns::concerns;
use elohim_epr_cli::flow::note::NoteActor;
use elohim_epr_cli::flow::retract::retract;
use elohim_epr_rea::{AgentRef, DepEdge, FlowRecord, FlowStore, Governor, SidecarFlowStore};
use eprfs_core::BlobCid;
use tempfile::TempDir;

fn git(root: &Path, args: &[&str]) {
    let out = elohim_epr_cli::process::build_command("git", args, root, &[])
        .env("GIT_AUTHOR_NAME", "Fixture Author")
        .env("GIT_AUTHOR_EMAIL", "author@example.test")
        .env("GIT_COMMITTER_NAME", "Fixture Author")
        .env("GIT_COMMITTER_EMAIL", "author@example.test")
        .env("GIT_AUTHOR_DATE", "2026-09-10T00:00:00Z")
        .env("GIT_COMMITTER_DATE", "2026-09-10T00:00:00Z")
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?} failed");
}

/// A committed tempdir repo with one readable doc, so a note can be dated against a HEAD.
fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(root.join("docs/up.md"), "upstream body\n").unwrap();
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "fixture"]);
    dir
}

/// Append a cite-seal edge sealed against `sealed_body`; returns the record's CID.
fn seal(root: &Path, from: &str, to: &str, sealed_body: &[u8], stamp: i64) -> cid::Cid {
    let edge = DepEdge::new(
        from.into(),
        to.into(),
        Some("asserted relationship".into()),
        Governor::CiteSeal,
        Some(*BlobCid::compute_raw(sealed_body).as_cid()),
        AgentRef("agent:test".into()),
        stamp,
        None,
    )
    .unwrap();
    SidecarFlowStore::open(root)
        .unwrap()
        .append(FlowRecord::Edge(edge))
        .unwrap()
}

fn actor() -> NoteActor {
    NoteActor::default()
}

#[test]
fn a_retracted_dangling_slot_leaves_the_concern_page_and_stays_in_the_sidecar() {
    let dir = fixture();
    let root = dir.path();
    // Two dangling slots — both endpoints gone, exactly the station-five residue shape.
    let gone = seal(
        root,
        "deleted/consumer.py",
        "deleted/contract.json",
        b"old",
        10,
    );
    seal(root, "deleted/other.py", "deleted/runtime.py", b"old", 10);

    let before = concerns(root, ".", 0, 10).unwrap();
    assert_eq!(before.counts.dangling, 2, "both slots start as dead ends");
    assert_eq!(before.counts.retracted, 0);
    let records_before = SidecarFlowStore::open(root)
        .unwrap()
        .records()
        .unwrap()
        .len();

    let outcome = retract(
        root,
        &gone.to_string(),
        "endpoints deleted by memory-kit replacement stations 3b and 5",
        &actor(),
    )
    .expect("a dangling cite-seal slot is retractable");
    assert_eq!(outcome.from, "deleted/consumer.py");
    assert!(!outcome.from_readable && !outcome.to_readable);
    assert!(outcome.note.appended, "the withdrawal is a new record");

    let after = concerns(root, ".", 0, 10).unwrap();
    assert_eq!(
        after.counts.dangling, 1,
        "the withdrawn slot is no longer offered as a concern"
    );
    assert_eq!(after.counts.retracted, 1, "and the withdrawal is COUNTED");
    assert_eq!(
        after.counts.total_edges, 1,
        "a withdrawn slot leaves the index entirely, not merely the selection"
    );
    assert!(
        after
            .omissions
            .iter()
            .any(|o| o.contains("withdrawn by retraction") && o.contains("deleted/consumer.py")),
        "the view must NAME what it stopped showing; got {:?}",
        after.omissions
    );

    // Append-only: the withdrawn edge record is still there, plus the retraction note.
    let records_after = SidecarFlowStore::open(root).unwrap().records().unwrap();
    assert_eq!(records_after.len(), records_before + 1);
    assert!(
        records_after.iter().any(|(c, _)| c == &gone),
        "retraction withdraws a declaration; it never deletes the record"
    );
}

#[test]
fn an_identical_rerun_is_a_no_op_and_a_different_reason_is_refused() {
    let dir = fixture();
    let root = dir.path();
    let gone = seal(root, "deleted/a.py", "deleted/b.json", b"old", 10);
    let reason = "endpoints deleted by memory-kit replacement stations 3b and 5";

    let first = retract(root, &gone.to_string(), reason, &actor()).unwrap();
    assert!(first.note.appended);
    let second = retract(root, &gone.to_string(), reason, &actor()).unwrap();
    assert!(
        !second.note.appended,
        "content-addressed idempotence: the same withdrawal twice is one record"
    );

    let conflicting = retract(root, &gone.to_string(), "some other ground", &actor())
        .expect_err("two withdrawal reasons for one slot must be refused");
    let text = conflicting.to_string();
    assert!(
        text.contains("already retracted") && text.contains(reason),
        "the refusal must quote the standing reason; got: {text}"
    );
}

#[test]
fn retract_refuses_every_record_that_is_not_a_dependency_edge() {
    let dir = fixture();
    let root = dir.path();
    let gone = seal(root, "deleted/a.py", "deleted/b.json", b"old", 10);
    // The retraction note itself is an EVENT — retracting a retraction is exactly the recursion a
    // "withdraw anything" verb would invite.
    let note = retract(root, &gone.to_string(), "withdrawn", &actor())
        .unwrap()
        .note
        .record_cid;

    let err = retract(root, &note, "withdraw the withdrawal", &actor())
        .expect_err("only a DepEdge is retractable");
    let text = err.to_string();
    assert!(
        text.contains("run event") && text.contains("DEPENDENCY slot"),
        "the refusal must name what the caller actually pointed at; got: {text}"
    );

    assert!(
        retract(root, "not-a-cid", "x", &actor())
            .expect_err("a path is not a record address")
            .to_string()
            .contains("is not a CID"),
        "a slug or path names a SLOT, not the record standing in it"
    );
    let unknown = "bafyreiey6xlfklrmv5ayewcgjafxldgyvlmq3pwdfremu6kzzvqt2efxlq";
    assert!(retract(root, unknown, "x", &actor()).is_err());
}

#[test]
fn retract_refuses_a_superseded_record_and_names_the_standing_one() {
    let dir = fixture();
    let root = dir.path();
    let older = seal(root, "deleted/a.py", "deleted/b.json", b"old", 10);
    let newer = seal(root, "deleted/a.py", "deleted/b.json", b"newer", 20);
    assert_ne!(older, newer);

    let err = retract(root, &older.to_string(), "withdrawn", &actor())
        .expect_err("withdrawing a displaced record would change nothing observable");
    let text = err.to_string();
    assert!(
        text.contains("superseded") && text.contains(&newer.to_string()),
        "the refusal must name the standing record; got: {text}"
    );

    // The standing one IS retractable, and withdrawing it withdraws the whole slot — the older
    // record does not float back up to take its place.
    retract(root, &newer.to_string(), "withdrawn", &actor()).unwrap();
    let after = concerns(root, ".", 0, 10).unwrap();
    assert_eq!(after.counts.total_edges, 0);
    assert_eq!(after.counts.retracted, 1);
}

#[test]
fn retraction_is_not_reachable_from_the_note_verb() {
    let dir = fixture();
    let root = dir.path();
    let err = elohim_epr_cli::flow::note::note(
        root,
        "docs/up.md",
        "retraction",
        "trying the back door",
        None,
        None,
        &actor(),
    )
    .expect_err("`--kind retraction` must not mint a withdrawal without the eligibility check");
    assert!(
        err.to_string().contains("epr flow retract"),
        "the refusal must point at where the act lives; got: {err}"
    );
}

#[test]
fn a_healthy_edge_may_be_withdrawn_too_and_the_view_says_so() {
    // Withdrawal is about the DECLARATION, not about the health of what it points at: an
    // assertion someone no longer stands behind is withdrawable even when it currently verifies.
    // What must never happen is a silent shrink, so the count and the omission are the assertion.
    let dir = fixture();
    let root = dir.path();
    // Sealed against the CANONICAL body, which is what `recompute_upstream` recomputes — the raw
    // file bytes would seal a digest no reader ever derives.
    let text = std::fs::read_to_string(root.join("docs/up.md")).unwrap();
    let body = elohim_epr_cli::flow::canonical_body(&text);
    let healthy = seal(root, "docs/consumer.md", "docs/up.md", body.as_bytes(), 10);
    let before = concerns(root, ".", 0, 10).unwrap();
    assert_eq!(before.counts.ok, 1, "the seal verifies before withdrawal");

    retract(
        root,
        &healthy.to_string(),
        "the relationship no longer stands",
        &actor(),
    )
    .unwrap();
    let after = concerns(root, ".", 0, 10).unwrap();
    assert_eq!(after.counts.ok, 0);
    assert_eq!(after.counts.total_edges, 0);
    assert_eq!(after.counts.retracted, 1);
    assert!(after
        .render_text()
        .contains("1 retracted (withdrawn, not indexed)"));
}
