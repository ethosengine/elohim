//! S2 (2026-09-22 recall-Codex-trail sprint): `open --purpose resume --habit <id>` — a read-only
//! view of one concern's own last evidence, its plans and its local git standing.
//!
//! Every assertion here runs the SHIPPED BINARY, matching this seam's existing convention
//! (`flow_memory_recall.rs`'s own module doc). A dedicated fixture builder lives in THIS file
//! rather than `tests/common/mod.rs` (shared by every `flow_memory_recall*` binary and touched by
//! a concurrently-landing station) so this station's tests are self-contained.
use std::path::Path;

use serde_json::Value;

mod common;
use common::{contract_value, ok, run, save_contract, write};

const HABIT_ID: &str = "resume-fixture-habit";

/// One committed change, dated explicitly (git dates by author/committer env, never wall clock) —
/// `common::git` pins one fixed date for every call; this station needs two DIFFERENT dates (one
/// before, one after the atom's own delta date) so it keeps its own tiny helper.
fn git_dated(root: &Path, args: &[&str], date: &str) {
    let out = elohim_epr_cli::process::build_command("git", args, root, &[])
        .env("GIT_AUTHOR_NAME", "Fixture Author")
        .env("GIT_COMMITTER_NAME", "Fixture Author")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.test")
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The habit register + atom this station's fixtures share. `with_postdelta_change` decides
/// whether a SECOND commit, dated after the atom's own `DELTA 2026-09-05` line, touches one of the
/// habit's concern paths (`docs/plan.md`, named by the atom's own `refs:`) — the one variable that
/// distinguishes "no commit since the last evidence" from "implemented-but-unverified".
fn repo_with_habit(with_postdelta_change: bool) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    write(
        root,
        "genesis/manifests/habits.yaml",
        &format!(
            "habits:\n  - id: {HABIT_ID}\n    status: red\n    active: false\n    checks:\n      - \"just gate memory-ceremony\"\n"
        ),
    );
    write(
        root,
        &format!(".epr-meta/{HABIT_ID}.habit.md"),
        &format!(
            "---\n\
             epr-habit-version: 1\n\
             id: {HABIT_ID}\n\
             invariant: >\n  Test invariant for the resume-view fixture.\n\
             status: red\n\
             active: false\n\
             checks:\n  - \"just gate memory-ceremony\"\n\
             refs:\n  - \"docs/plan.md — the fixture's own plan\"\n\
             retire-when: >\n  never; this is a fixture.\n\
             ---\n\
             DELTA 2026-09-05 (fixture evidence): the fixture's baseline standing.\n"
        ),
    );
    write(root, "docs/plan.md", "# Plan\nOriginal plan text.\n");
    write(
        root,
        "docs/source.md",
        "# Source\nirrelevant to this station.\n",
    );
    save_contract(root, contract_value(false));

    git_dated(root, &["init", "-q"], "2026-09-01T00:00:00+00:00");
    git_dated(root, &["add", "-A"], "2026-09-01T00:00:00+00:00");
    git_dated(
        root,
        &["commit", "-qm", "fixture: habit register, atom and plan"],
        "2026-09-01T00:00:00+00:00",
    );

    if with_postdelta_change {
        write(
            root,
            "docs/plan.md",
            "# Plan\nOriginal plan text.\nA later change landed on the concern path.\n",
        );
        git_dated(root, &["add", "-A"], "2026-09-10T00:00:00+00:00");
        git_dated(
            root,
            &["commit", "-qm", "plan: a later change"],
            "2026-09-10T00:00:00+00:00",
        );
    }

    dir
}

#[test]
fn resume_refuses_a_habit_the_register_does_not_declare() {
    let dir = repo_with_habit(false);
    let run_result = run(
        dir.path(),
        &["open", "--purpose", "resume", "--habit", "no-such-habit"],
    );
    assert_eq!(
        run_result.code, 2,
        "expected a refusal, got: {}{}",
        run_result.stdout, run_result.stderr
    );
    let payload = run_result.json();
    let unresolved = payload["unresolved"][0].as_str().unwrap_or_default();
    assert!(
        unresolved.contains("no habit named"),
        "expected an unknown-habit refusal, got: {unresolved}"
    );
}

#[test]
fn resume_reports_evidence_current_with_no_postdelta_commits() {
    let dir = repo_with_habit(false);
    let view: Value = ok(
        dir.path(),
        &["open", "--purpose", "resume", "--habit", HABIT_ID],
    );
    let resume = &view["resume"];
    assert_eq!(resume["habit"]["id"], HABIT_ID);
    assert_eq!(resume["habit"]["status"], "red");
    assert_eq!(resume["habit"]["last_delta"]["date"], "2026-09-05");
    let verdict = resume["verdict"].as_str().unwrap_or_default();
    assert!(
        verdict.starts_with("no commit since the last evidence"),
        "expected an evidence-current verdict, got: {verdict}"
    );
    assert_eq!(resume["since_delta"]["count"], 0);
    // The plan is named by the atom's own `refs:`, exists on disk, and is reported as such.
    let plans = resume["plans"].as_array().expect("plans array");
    assert!(
        plans
            .iter()
            .any(|p| p["path"] == "docs/plan.md" && p["exists"] == true),
        "expected docs/plan.md among the plans, got: {plans:?}"
    );
    // The three standing omissions render regardless of what else happened.
    let omissions: Vec<&str> = resume["omissions"]
        .as_array()
        .expect("omissions array")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(omissions.iter().any(|o| o.contains("CI/Jenkins")));
    assert!(omissions
        .iter()
        .any(|o| o.contains("not evidence of coverage")));
    assert!(omissions
        .iter()
        .any(|o| o.contains("Worktrees of other sessions")));
}

#[test]
fn resume_reports_implemented_but_unverified_with_a_postdelta_commit() {
    let dir = repo_with_habit(true);
    let view: Value = ok(
        dir.path(),
        &["open", "--purpose", "resume", "--habit", HABIT_ID],
    );
    let resume = &view["resume"];
    let verdict = resume["verdict"].as_str().unwrap_or_default();
    assert!(
        verdict.starts_with("implemented-but-unverified"),
        "expected an implemented-but-unverified verdict, got: {verdict}"
    );
    assert!(verdict.contains("just gate memory-ceremony"));
    let commits = resume["since_delta"]["commits"]
        .as_array()
        .expect("commits array");
    assert!(
        !commits.is_empty(),
        "expected at least one since-delta commit"
    );
    assert!(
        commits
            .iter()
            .any(|c| c["subject"].as_str() == Some("plan: a later change")),
        "expected the post-delta commit to be named, got: {commits:?}"
    );
    assert_eq!(commits[0]["matched_by"], "path");
}

/// A fresh reader (2026-09-22) was shown a 2026-09-11 entry as the "last delta" of an atom whose
/// newest evidence, 2026-09-12, sat further down: the ledger is not reliably newest-first. The
/// newest entry is chosen by its DATE, whatever its position, and consecutive entries with no
/// blank line between them are still separate entries.
#[test]
fn resume_picks_the_latest_dated_entry_not_the_first() {
    let dir = repo_with_habit(false);
    write(
        dir.path(),
        &format!(".epr-meta/{HABIT_ID}.habit.md"),
        &format!(
            "---\n\
             epr-habit-version: 1\n\
             id: {HABIT_ID}\n\
             invariant: >\n  Test invariant.\n\
             status: red\n\
             active: false\n\
             checks:\n  - \"just gate memory-ceremony\"\n\
             refs:\n  - \"docs/plan.md — the fixture's own plan\"\n\
             retire-when: >\n  never.\n\
             ---\n\
             DELTA 2026-09-03 (older entry written at the top).\n\
             GREEN 2026-09-02 (an even older entry, no blank line before it).\n\
             \n\
             RED 2026-09-08 (the newest entry, appended at the bottom).\n"
        ),
    );
    let payload = ok(
        dir.path(),
        &["open", "--purpose", "resume", "--habit", HABIT_ID],
    );
    let delta = &payload["resume"]["habit"]["last_delta"];
    assert_eq!(delta["date"], "2026-09-08", "{delta}");
    assert!(
        delta["text"]
            .as_str()
            .unwrap_or_default()
            .starts_with("RED 2026-09-08"),
        "{delta}"
    );
}

/// Two entries share the latest date in an oldest-first (bottom-appended) ledger: the one written
/// last is the newest, because the ledger's own direction breaks the tie.
#[test]
fn resume_breaks_a_same_date_tie_by_the_ledgers_direction() {
    let dir = repo_with_habit(false);
    write(
        dir.path(),
        &format!(".epr-meta/{HABIT_ID}.habit.md"),
        &format!(
            "---\nid: {HABIT_ID}\nstatus: red\nactive: false\nchecks:\n  - \"just gate memory-ceremony\"\n---\n\
             DELTA 2026-09-01 (the first entry, oldest).\n\n\
             DELTA 2026-09-08 (an earlier entry that day).\n\n\
             RED 2026-09-08 (the last entry written that day).\n"
        ),
    );
    let payload = ok(
        dir.path(),
        &["open", "--purpose", "resume", "--habit", HABIT_ID],
    );
    let text = payload["resume"]["habit"]["last_delta"]["text"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(text.starts_with("RED 2026-09-08"), "{text}");
}
