//! Station five: `epr flow concerns --corrections` — unresolved corrections by IDENTITY.
//!
//! The reading this replaces asked a DATE question: it picked the newest dated ceremony filename and
//! treated everything older as absorbed. That reading cannot say WHICH correction was discharged, so
//! it both hid live corrections behind an unrelated later ceremony and retired ones nobody had
//! looked at (`genesis/docs/analysis/2026-09-09-memory-ceremony-efficacy.md`, error one).
//!
//! The native reading asks an IDENTITY question instead: a correction is unresolved unless some
//! later note about the same subject carries `closes:<its exact CID>`. These tests hold that line
//! from both sides — a closure that names the correction closes it, and every near-miss (an earlier
//! note, a different subject, an unknown CID, a `failed-approach`) does not.

use std::path::Path;
use std::process::Command;

use elohim_epr_cli::flow::concerns::corrections;
use elohim_epr_cli::flow::note::{self, NoteActor};
use elohim_epr_rea::{FlowStore, SidecarFlowStore};
use tempfile::TempDir;

/// A note is dated by the tree it was authored against — git HEAD's author date — and that date is
/// part of its content address. So the fixture's commit date is PINNED: without it, two fixture
/// repositories built a second apart mint different CIDs for the same authored note, and the
/// additivity assertion below would pass or fail on the clock rather than on the slot vector.
const FIXTURE_DATE: &str = "2026-09-10T00:00:00+00:00";

fn git(root: &Path, args: &[&str]) {
    let out = elohim_epr_cli::process::build_command("git", args, root, &[])
        .env("GIT_AUTHOR_NAME", "Fixture Author")
        .env("GIT_COMMITTER_NAME", "Fixture Author")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.test")
        .env("GIT_AUTHOR_DATE", FIXTURE_DATE)
        .env("GIT_COMMITTER_DATE", FIXTURE_DATE)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}");
}

fn repo() -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    std::fs::create_dir_all(root.join("docs")).expect("mkdir");
    std::fs::write(root.join("docs/a.md"), "# A\nbody\n").expect("write");
    std::fs::write(root.join("docs/b.md"), "# B\nbody\n").expect("write");
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "fixture"]);
    dir
}

fn write_note(
    root: &Path,
    on: &str,
    kind: &str,
    reason: &str,
    closes: Option<&str>,
) -> note::NoteOutcome {
    note::note_with_options_closing(
        root,
        on,
        kind,
        reason,
        None,
        closes,
        None,
        &NoteActor::default(),
        &Default::default(),
    )
    .unwrap_or_else(|error| panic!("note {kind} on {on}: {error}"))
}

// ── the reading ─────────────────────────────────────────────────────────────────────────────────

#[test]
fn a_correction_with_a_closure_is_not_listed_and_one_without_is() {
    let dir = repo();
    let root = dir.path();
    let open = write_note(
        root,
        "docs/a.md",
        "correction",
        "STALE: the first claim drifted",
        None,
    );
    let closed = write_note(
        root,
        "docs/b.md",
        "correction",
        "STALE: the second claim drifted",
        None,
    );

    let before = corrections(root, None).expect("read");
    assert_eq!(before.counts.corrections, 2);
    assert_eq!(before.counts.unresolved, 2);
    assert_eq!(before.counts.stale_unresolved, 2);
    assert_eq!(before.counts.resolved, 0);
    assert!(before.surfaces.contains_key("docs/a.md"));
    assert!(before.surfaces.contains_key("docs/b.md"));
    assert!(before.issues.is_empty());

    write_note(
        root,
        "docs/b.md",
        "observation",
        "Repaired: the claim now matches the source",
        Some(&closed.record_cid),
    );

    let after = corrections(root, None).expect("read");
    assert_eq!(after.counts.corrections, 2);
    assert_eq!(after.counts.unresolved, 1);
    assert_eq!(after.counts.resolved, 1);
    assert!(after.surfaces.contains_key("docs/a.md"));
    assert!(!after.surfaces.contains_key("docs/b.md"));
    assert!(after.issues.is_empty());
    let closure = &after.resolved[&closed.record_cid][0];
    assert_eq!(closure.kind, "run:observation");
    assert!(!closure.closed_by.is_empty());
    assert!(closure.reason.contains("Repaired"));
    // The one that stays listed is the one nobody closed.
    assert_eq!(after.surfaces["docs/a.md"][0].cid, open.record_cid);
}

/// A repair CHANGES the bytes the correction was written against, so closure cannot require the
/// resource CID to match — only the subject and the named correction CID.
#[test]
fn a_closure_survives_the_repair_that_changed_the_source() {
    let dir = repo();
    let root = dir.path();
    let correction = write_note(root, "docs/a.md", "correction", "STALE: wrong", None);
    let original_resource = correction.resource.clone();
    std::fs::write(root.join("docs/a.md"), "# A\nrepaired body\n").expect("write");
    let closure = write_note(
        root,
        "docs/a.md",
        "ruling",
        "The repair landed; accepting the correction as discharged",
        Some(&correction.record_cid),
    );
    assert_ne!(
        closure.resource, original_resource,
        "the fixture must actually change the bytes"
    );
    let view = corrections(root, None).expect("read");
    assert_eq!(view.counts.unresolved, 0);
    assert_eq!(view.counts.resolved, 1);
    assert!(view.issues.is_empty());
}

#[test]
fn every_near_miss_leaves_the_correction_open() {
    let dir = repo();
    let root = dir.path();
    let correction = write_note(root, "docs/a.md", "correction", "STALE: wrong", None);

    // A closure naming a DIFFERENT subject is an issue, not a closure.
    write_note(
        root,
        "docs/b.md",
        "observation",
        "Repaired something else",
        Some(&correction.record_cid),
    );
    let wrong_subject = corrections(root, None).expect("read");
    assert_eq!(wrong_subject.counts.unresolved, 1);
    assert_eq!(wrong_subject.counts.resolved, 0);
    assert!(wrong_subject
        .issues
        .iter()
        .any(|i| i.contains("names a different subject")));

    // A closure naming an UNKNOWN correction CID is an issue, not a closure.
    let unknown = elohim_epr_cli::flow::body_cid("nothing here").to_string();
    write_note(
        root,
        "docs/a.md",
        "observation",
        "Closing a ghost",
        Some(&unknown),
    );
    let ghost = corrections(root, None).expect("read");
    assert!(ghost
        .issues
        .iter()
        .any(|i| i.contains("unknown correction")));
    assert_eq!(ghost.counts.unresolved, 1);

    // A later prose note about the same subject, carrying no `closes:`, closes nothing.
    write_note(root, "docs/a.md", "observation", "Looked at it again", None);
    assert_eq!(corrections(root, None).expect("read").counts.unresolved, 1);

    // And a `failed-approach` may not carry a closure at all.
    let refused = note::note_with_options_closing(
        root,
        "docs/a.md",
        "failed-approach",
        "Tried and gave up",
        None,
        Some(&correction.record_cid),
        None,
        &NoteActor::default(),
        &Default::default(),
    )
    .expect_err("failed-approach cannot close");
    assert!(format!("{refused}").contains("closes nothing"));
}

/// A closure that is not LATER than its correction closes nothing.
#[test]
fn a_closure_must_be_later_than_the_correction_it_names() {
    let dir = repo();
    let root = dir.path();
    // Author the closure FIRST, naming a CID the correction will only mint afterwards. The atom CID
    // is content-derived, so it can be computed before the correction is appended.
    let correction_reason = "STALE: predicted";
    let predicted = note::note_with_options_closing(
        root,
        "docs/a.md",
        "correction",
        correction_reason,
        None,
        None,
        None,
        &NoteActor::default(),
        &Default::default(),
    )
    .expect("correction");
    // Re-appending the identical note is a no-op, so re-order by writing the closure after a
    // deliberate reset of the ledger with the closure FIRST.
    let records = SidecarFlowStore::open(root)
        .expect("store")
        .records()
        .expect("records");
    assert_eq!(records.len(), 1);
    let closure = write_note(
        root,
        "docs/a.md",
        "observation",
        "Repaired",
        Some(&predicted.record_cid),
    );
    // Sanity: in the correct order, it closes.
    assert_eq!(corrections(root, None).expect("read").counts.resolved, 1);

    // Now rewrite the ledger with the closure BEFORE the correction and confirm it no longer counts.
    let lines: Vec<String> = std::fs::read_to_string(root.join(".eprfs/status/flows.jsonl"))
        .expect("ledger")
        .lines()
        .map(str::to_string)
        .collect();
    assert_eq!(lines.len(), 2);
    std::fs::write(
        root.join(".eprfs/status/flows.jsonl"),
        format!("{}\n{}\n", lines[1], lines[0]),
    )
    .expect("write");
    let reversed = corrections(root, None).expect("read");
    assert_eq!(reversed.counts.resolved, 0);
    assert_eq!(reversed.counts.unresolved, 1);
    assert!(reversed
        .issues
        .iter()
        .any(|i| i.contains("not later than the correction")));
    assert!(!closure.record_cid.is_empty());
}

/// A closure act written as `--kind correction` is not itself a fresh unresolved correction.
#[test]
fn a_closure_written_as_a_correction_is_not_a_new_open_concern() {
    let dir = repo();
    let root = dir.path();
    let first = write_note(root, "docs/a.md", "correction", "STALE: wrong", None);
    write_note(
        root,
        "docs/a.md",
        "correction",
        "Migrated closure: the repair is recorded and pinned",
        Some(&first.record_cid),
    );
    let view = corrections(root, None).expect("read");
    assert_eq!(view.counts.corrections, 1);
    assert_eq!(view.counts.unresolved, 0);
    assert_eq!(view.counts.resolved, 1);
}

/// `--since` is a DISPLAY filter. It suppresses rows from the list and reports how many; it never
/// resolves one, which is precisely the inference the date heuristic made.
#[test]
fn since_filters_the_display_and_resolves_nothing() {
    let dir = repo();
    let root = dir.path();
    write_note(root, "docs/a.md", "correction", "STALE: wrong", None);
    let dated = corrections(root, None).expect("read");
    let date = dated.surfaces["docs/a.md"][0].date.clone();
    assert_eq!(date.len(), 10);

    // The boundary is inclusive: a correction dated exactly on `--since` is still shown.
    let inclusive = corrections(root, Some(&date)).expect("read");
    assert_eq!(inclusive.counts.unresolved, 1);
    assert_eq!(inclusive.counts.omitted_by_since, 0);

    let later = corrections(root, Some("2099-01-01")).expect("read");
    assert_eq!(later.counts.unresolved, 0);
    assert_eq!(later.counts.omitted_by_since, 1);
    // Suppressed is not resolved.
    assert_eq!(later.counts.resolved, 0);

    for bad in ["not-a-date", "2026-1-1", "20260101", "2026-01-01T00:00:00Z"] {
        assert!(corrections(root, Some(bad)).is_err(), "{bad}");
    }
}

/// A repository with no flow plane answers honestly and does not create one.
#[test]
fn a_missing_ledger_is_reported_and_never_created() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let view = corrections(root, None).expect("read");
    assert_eq!(view.counts.corrections, 0);
    assert!(view
        .issues
        .iter()
        .any(|i| i == "correction ledger is missing"));
    assert!(
        !root.join(".eprfs").exists(),
        "asking a question left a trace"
    );
}

// ── the shipped surface ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_shipped_command_prints_the_closure_template_and_exits_on_issues() {
    let dir = repo();
    let root = dir.path();
    let correction = write_note(root, "docs/a.md", "correction", "STALE: wrong", None);

    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args([
            "flow",
            "concerns",
            "--corrections",
            "--root",
            &root.to_string_lossy(),
        ])
        .output()
        .expect("epr runs");
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("Unresolved corrections: 1 (1 STALE)"));
    // The template is the exact command that would close it — not a description of one.
    assert!(
        text.contains(&format!(
            "epr flow note --on docs/a.md --kind correction --closes {} --reason",
            correction.record_cid
        )),
        "{text}"
    );

    // `--json` carries the same template as argv a caller can execute.
    let json = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args([
            "flow",
            "concerns",
            "--corrections",
            "--json",
            "--root",
            &root.to_string_lossy(),
        ])
        .output()
        .expect("epr runs");
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).expect("json");
    let template = &value["surfaces"]["docs/a.md"][0]["closure_template"];
    assert_eq!(template[0], "epr");
    assert_eq!(template[7], "--closes");
    assert_eq!(template[8], correction.record_cid);

    // An unreadable row is a REVALIDATE signal, and a revalidation signal that exits zero is one
    // nobody acts on.
    write_note(
        root,
        "docs/a.md",
        "observation",
        "Closing a ghost",
        Some(&elohim_epr_cli::flow::body_cid("nothing").to_string()),
    );
    let noisy = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args([
            "flow",
            "concerns",
            "--corrections",
            "--root",
            &root.to_string_lossy(),
        ])
        .output()
        .expect("epr runs");
    assert_eq!(noisy.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&noisy.stdout).contains("REVALIDATE"));

    // The subcommand refuses a bare invocation rather than guessing which view was wanted.
    let bare = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args([
            "flow",
            "concerns",
            "--since",
            "2026-01-01",
            "--root",
            &root.to_string_lossy(),
        ])
        .output()
        .expect("epr runs");
    assert_ne!(bare.status.code(), Some(0));
}

/// The `--closes` slot is ADDITIVE: a note that closes nothing keeps the address it always had.
#[test]
fn the_closes_slot_is_additive_and_leaves_existing_addresses_alone() {
    let plain = {
        let dir = repo();
        write_note(dir.path(), "docs/a.md", "observation", "same body", None).record_cid
    };
    let through_closing_path = {
        let dir = repo();
        note::note_with_options(
            dir.path(),
            "docs/a.md",
            "observation",
            "same body",
            None,
            None,
            &NoteActor::default(),
            &Default::default(),
        )
        .expect("note")
        .record_cid
    };
    assert_eq!(plain, through_closing_path);

    // Adding a closure changes the record, as it must: it is a different claim.
    let dir = repo();
    let root = dir.path();
    let correction = write_note(root, "docs/a.md", "correction", "STALE: wrong", None);
    let with_closure = write_note(
        root,
        "docs/a.md",
        "observation",
        "same body",
        Some(&correction.record_cid),
    );
    assert_ne!(with_closure.record_cid, plain);
    assert_eq!(
        with_closure.closes.as_deref(),
        Some(correction.record_cid.as_str())
    );

    // A malformed closure target is refused before anything is appended.
    let refused = note::note_with_options_closing(
        root,
        "docs/a.md",
        "observation",
        "body",
        None,
        Some("not-a-cid"),
        None,
        &NoteActor::default(),
        &Default::default(),
    )
    .expect_err("prose closure");
    assert!(format!("{refused}").contains("is not a CID"));
}
