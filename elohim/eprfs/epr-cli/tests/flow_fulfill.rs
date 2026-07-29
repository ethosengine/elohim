//! Integration tests over a synthetic tempdir repo for `epr flow fulfill` — the
//! a2o-verdict → REA-fulfillment emitter (spec §5 joint 5). Mirrors `flow_integration.rs`'s
//! fixture shape: a recipe with a `scenario` stage under a `genesis/a2o/features/...` glob
//! (matching production layout) so the default `--surface-prefix genesis/a2o` join resolves
//! naturally against a cucumber-shaped `surface` (repo-relative under `genesis/a2o`).

use std::path::Path;

use elohim_epr_cli::flow::fulfill::{fulfill, FulfillOptions};
use elohim_epr_cli::flow::project;
use elohim_epr_rea::{FlowStore, ReaVerb, SidecarFlowStore};
use tempfile::TempDir;

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

/// A tempdir repo with a `resiliency-saga`-shaped recipe (one feature file) and the
/// `project`ed scenario commitment already minted.
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
      - name: validation
        artifactKind: "a2o:verdict"
        paths:
          - "genesis/a2o/reports/sprint-report-dataplane.json"
    edges:
      - { from: scenario, to: validation, meaningful: true, validators: [a2o-run] }
"#,
    );

    write(
        root,
        "genesis/a2o/features/dataplane/resiliency-saga/05-node-loss.feature",
        "Feature: Chapter five — node loss\n  @concern:resiliency\n  Scenario: data survives\n    Given a node dies\n    Then the data survives\n",
    );

    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "fixture"]);
    dir
}

/// Write a minimal sprint-report fixture: one concern, one scenario surface, the given
/// pass/fail/pending counts.
fn write_report(
    root: &Path,
    rel: &str,
    generated_at: &str,
    run_id: &str,
    failed: u32,
    pending: u32,
    passed: u32,
) {
    let status = if failed > 0 {
        "failed"
    } else if pending > 0 {
        "pending"
    } else {
        "passed"
    };
    let json = format!(
        r#"{{
  "generatedAt": "{generated_at}",
  "runId": "{run_id}",
  "profile": "alpha",
  "summary": {{
    "scenarios": {{ "total": 1, "passed": {passed}, "failed": {failed}, "skipped": 0, "pending": {pending} }},
    "findings": {{ "total": 0, "bySource": {{}}, "byPillar": {{}} }},
    "byConcern": {{
      "resiliency": {{
        "passed": {passed},
        "failed": {failed},
        "pending": {pending},
        "scenarios": [
          {{ "name": "data survives", "status": "{status}", "surface": "features/dataplane/resiliency-saga/05-node-loss.feature" }}
        ]
      }}
    }}
  }},
  "findings": []
}}
"#
    );
    write(root, rel, &json);
}

#[test]
fn green_report_fulfills_the_scenario_commitment_once() {
    let dir = fixture();
    let root = dir.path();
    let recipes = root.join(".claude/epr-meta/recipes.yaml");
    project::project(root, &recipes).expect("project runs");

    let report_path = root.join("report.json");
    write_report(
        root,
        "report.json",
        "2026-07-25T00:00:00Z",
        "run-1",
        0,
        0,
        1,
    );

    let summary = fulfill(root, &report_path, &FulfillOptions::default()).expect("fulfill runs");
    assert_eq!(summary.fulfilled_new, 1);
    assert_eq!(summary.already_fulfilled, 0);
    assert!(summary.unmatched_surfaces.is_empty());

    let store = SidecarFlowStore::open(root).unwrap();
    let produce_events: Vec<_> = store
        .events()
        .unwrap()
        .into_iter()
        .filter(|(_, e)| e.action == ReaVerb::Produce && !e.fulfills.is_empty())
        .collect();
    assert_eq!(
        produce_events.len(),
        1,
        "exactly one fulfilling Produce event"
    );
}

#[test]
fn rerunning_the_same_report_appends_nothing_new() {
    let dir = fixture();
    let root = dir.path();
    let recipes = root.join(".claude/epr-meta/recipes.yaml");
    project::project(root, &recipes).expect("project runs");

    let report_path = root.join("report.json");
    write_report(
        root,
        "report.json",
        "2026-07-25T00:00:00Z",
        "run-1",
        0,
        0,
        1,
    );

    fulfill(root, &report_path, &FulfillOptions::default()).expect("first run");
    let second = fulfill(root, &report_path, &FulfillOptions::default()).expect("second run");

    assert_eq!(second.fulfilled_new, 0, "identical report is a no-op");
    assert_eq!(second.already_fulfilled, 1);
}

#[test]
fn a_second_distinct_green_report_is_already_fulfilled() {
    let dir = fixture();
    let root = dir.path();
    let recipes = root.join(".claude/epr-meta/recipes.yaml");
    project::project(root, &recipes).expect("project runs");

    let first_report = root.join("report-1.json");
    write_report(
        root,
        "report-1.json",
        "2026-07-25T00:00:00Z",
        "run-1",
        0,
        0,
        1,
    );
    fulfill(root, &first_report, &FulfillOptions::default()).expect("first run");

    let second_report = root.join("report-2.json");
    write_report(
        root,
        "report-2.json",
        "2026-07-25T01:00:00Z",
        "run-2",
        0,
        0,
        1,
    );
    let summary = fulfill(root, &second_report, &FulfillOptions::default()).expect("second run");

    assert_eq!(
        summary.fulfilled_new, 0,
        "commitment is already discharged by the first report"
    );
    assert_eq!(summary.already_fulfilled, 1);
}

#[test]
fn a_red_report_after_a_green_one_dismisses_the_regression() {
    let dir = fixture();
    let root = dir.path();
    let recipes = root.join(".claude/epr-meta/recipes.yaml");
    project::project(root, &recipes).expect("project runs");

    let green_report = root.join("report-green.json");
    write_report(
        root,
        "report-green.json",
        "2026-07-25T00:00:00Z",
        "run-1",
        0,
        0,
        1,
    );
    fulfill(root, &green_report, &FulfillOptions::default()).expect("green run");

    let red_report = root.join("report-red.json");
    write_report(
        root,
        "report-red.json",
        "2026-07-25T02:00:00Z",
        "run-2",
        1,
        0,
        0,
    );
    let summary = fulfill(root, &red_report, &FulfillOptions::default()).expect("red run");

    assert_eq!(summary.regressions_dismissed, 1);
    assert_eq!(summary.fulfilled_new, 0);
    assert_eq!(summary.skipped_red, 0);

    let store = SidecarFlowStore::open(root).unwrap();
    let dismiss_events: Vec<_> = store
        .events()
        .unwrap()
        .into_iter()
        .filter(|(_, e)| e.action == ReaVerb::Dismiss)
        .collect();
    assert_eq!(dismiss_events.len(), 1, "exactly one Dismiss event");
    assert!(dismiss_events[0].1.fulfills.is_empty());
}

#[test]
fn a_red_report_with_no_prior_green_is_skipped_not_dismissed() {
    let dir = fixture();
    let root = dir.path();
    let recipes = root.join(".claude/epr-meta/recipes.yaml");
    project::project(root, &recipes).expect("project runs");

    let red_report = root.join("report-red.json");
    write_report(
        root,
        "report-red.json",
        "2026-07-25T00:00:00Z",
        "run-1",
        1,
        0,
        0,
    );
    let summary = fulfill(root, &red_report, &FulfillOptions::default()).expect("red run");

    assert_eq!(summary.skipped_red, 1);
    assert_eq!(summary.regressions_dismissed, 0);

    let store = SidecarFlowStore::open(root).unwrap();
    assert!(store
        .events()
        .unwrap()
        .into_iter()
        .all(|(_, e)| e.action != ReaVerb::Dismiss));
}

#[test]
fn regression_re_commitment_re_produces_after_a_dismissed_regression() {
    let dir = fixture();
    let root = dir.path();
    let recipes = root.join(".claude/epr-meta/recipes.yaml");
    project::project(root, &recipes).expect("project runs");

    // Produce: initial green fulfillment discharges the commitment.
    let green_report = root.join("report-green.json");
    write_report(
        root,
        "report-green.json",
        "2026-07-25T00:00:00Z",
        "run-1",
        0,
        0,
        1,
    );
    fulfill(root, &green_report, &FulfillOptions::default()).expect("green run");

    // Dismiss: a later red run regresses the discharged commitment.
    let red_report = root.join("report-red.json");
    write_report(
        root,
        "report-red.json",
        "2026-07-25T02:00:00Z",
        "run-2",
        1,
        0,
        0,
    );
    fulfill(root, &red_report, &FulfillOptions::default()).expect("red run");

    // Recovery: a fresh all-green report must re-produce, NOT count already_fulfilled —
    // this is the regression re-commitment the bug leaves sticky forever.
    let recovered_report = root.join("report-recovered.json");
    write_report(
        root,
        "report-recovered.json",
        "2026-07-25T04:00:00Z",
        "run-3",
        0,
        0,
        1,
    );
    let summary =
        fulfill(root, &recovered_report, &FulfillOptions::default()).expect("recovery run");

    assert_eq!(
        summary.refulfilled, 1,
        "regression re-commitment counted distinctly from fulfilled_new/already_fulfilled"
    );
    assert_eq!(
        summary.fulfilled_new, 0,
        "not a brand-new commitment fulfillment"
    );
    assert_eq!(summary.already_fulfilled, 0);

    let store = SidecarFlowStore::open(root).unwrap();
    let produce_events: Vec<_> = store
        .events()
        .unwrap()
        .into_iter()
        .filter(|(_, e)| e.action == ReaVerb::Produce && !e.fulfills.is_empty())
        .collect();
    assert_eq!(
        produce_events.len(),
        2,
        "the original fulfilling Produce plus the recovery Produce"
    );

    // Re-running the SAME recovered report is a true no-op — idempotent, no duplicate.
    let second = fulfill(root, &recovered_report, &FulfillOptions::default()).expect("rerun");
    assert_eq!(second.refulfilled, 0);
    assert_eq!(
        second.already_fulfilled, 1,
        "latest event is now Produce again — steady state, no re-commitment needed"
    );

    let produce_events_2: Vec<_> = SidecarFlowStore::open(root)
        .unwrap()
        .events()
        .unwrap()
        .into_iter()
        .filter(|(_, e)| e.action == ReaVerb::Produce && !e.fulfills.is_empty())
        .collect();
    assert_eq!(
        produce_events_2.len(),
        2,
        "no duplicate Produce appended on rerun"
    );
}

#[test]
fn out_of_order_backfill_does_not_re_produce_over_a_newer_dismiss() {
    let dir = fixture();
    let root = dir.path();
    let recipes = root.join(".claude/epr-meta/recipes.yaml");
    project::project(root, &recipes).expect("project runs");

    // Produce at T1: initial green fulfillment discharges the commitment.
    let green_report = root.join("report-green.json");
    write_report(
        root,
        "report-green.json",
        "2026-07-25T00:00:00Z", // T1
        "run-1",
        0,
        0,
        1,
    );
    fulfill(root, &green_report, &FulfillOptions::default()).expect("green run (T1)");

    // Dismiss at T3: a later red run regresses the discharged commitment.
    let red_report = root.join("report-red.json");
    write_report(
        root,
        "report-red.json",
        "2026-07-25T03:00:00Z", // T3
        "run-2",
        1,
        0,
        0,
    );
    fulfill(root, &red_report, &FulfillOptions::default()).expect("red run (T3)");

    // Backfill: an all-green report whose generatedAt (T2) is BETWEEN T1 and T3 arrives
    // AFTER the Dismiss in append order — the exact divergence scenario the bug reintroduced.
    // Append-order "latest" would say Produce (T2 was appended last); time-order — the same
    // rule saga-status.py uses — says the Dismiss (T3) is still latest, so this must NOT
    // re-produce.
    let backfilled_report = root.join("report-backfilled.json");
    write_report(
        root,
        "report-backfilled.json",
        "2026-07-25T02:00:00Z", // T2 (T1 < T2 < T3)
        "run-backfill",
        0,
        0,
        1,
    );
    let summary =
        fulfill(root, &backfilled_report, &FulfillOptions::default()).expect("backfill run (T2)");

    assert_eq!(
        summary.skipped_stale_recovery, 1,
        "a backfilled green older than the standing Dismiss must not re-produce"
    );
    assert_eq!(summary.refulfilled, 0);
    assert_eq!(summary.already_fulfilled, 0);

    let store = SidecarFlowStore::open(root).unwrap();
    let produce_events: Vec<_> = store
        .events()
        .unwrap()
        .into_iter()
        .filter(|(_, e)| e.action == ReaVerb::Produce && !e.fulfills.is_empty())
        .collect();
    assert_eq!(
        produce_events.len(),
        1,
        "only the original T1 Produce — no recovery Produce appended for the stale backfill"
    );

    // Latest-by-time is still the Dismiss: a fresh all-green report appended right now
    // (without advancing past T3) would STILL see the regression as latest and skip, proving
    // the Dismiss, not the backfilled Produce, is what the state machine currently reads as
    // "latest" — exactly what saga-status.py's time-ordered index_flow_state also concludes.
    let restate_report = root.join("report-restate.json");
    write_report(
        root,
        "report-restate.json",
        "2026-07-25T02:30:00Z", // still < T3
        "run-restate",
        0,
        0,
        1,
    );
    let restate_summary =
        fulfill(root, &restate_report, &FulfillOptions::default()).expect("restate run");
    assert_eq!(
        restate_summary.skipped_stale_recovery, 1,
        "still older than the Dismiss (T3) — still stale"
    );
}

#[test]
fn recovery_after_backfill_re_produces_once_truly_newer_than_the_dismiss() {
    let dir = fixture();
    let root = dir.path();
    let recipes = root.join(".claude/epr-meta/recipes.yaml");
    project::project(root, &recipes).expect("project runs");

    // Produce at T1.
    let green_report = root.join("report-green.json");
    write_report(
        root,
        "report-green.json",
        "2026-07-25T00:00:00Z", // T1
        "run-1",
        0,
        0,
        1,
    );
    fulfill(root, &green_report, &FulfillOptions::default()).expect("green run (T1)");

    // Dismiss at T3.
    let red_report = root.join("report-red.json");
    write_report(
        root,
        "report-red.json",
        "2026-07-25T03:00:00Z", // T3
        "run-2",
        1,
        0,
        0,
    );
    fulfill(root, &red_report, &FulfillOptions::default()).expect("red run (T3)");

    // Stale backfill at T2 (T1 < T2 < T3) — must be skipped (proven by the sibling test
    // above; repeated here so this test is self-contained about the starting state).
    let backfilled_report = root.join("report-backfilled.json");
    write_report(
        root,
        "report-backfilled.json",
        "2026-07-25T02:00:00Z", // T2
        "run-backfill",
        0,
        0,
        1,
    );
    let stale = fulfill(root, &backfilled_report, &FulfillOptions::default()).expect("T2 run");
    assert_eq!(stale.skipped_stale_recovery, 1);

    // Recovery at T4 (> T3): a TRULY newer all-green report must fire the recovery Produce.
    let recovered_report = root.join("report-recovered.json");
    write_report(
        root,
        "report-recovered.json",
        "2026-07-25T04:00:00Z", // T4
        "run-3",
        0,
        0,
        1,
    );
    let summary =
        fulfill(root, &recovered_report, &FulfillOptions::default()).expect("recovery run (T4)");

    assert_eq!(
        summary.refulfilled, 1,
        "T4 is strictly newer than the T3 Dismiss — recovery must fire"
    );
    assert_eq!(summary.skipped_stale_recovery, 0);
    assert_eq!(summary.fulfilled_new, 0);
    assert_eq!(summary.already_fulfilled, 0);

    let store = SidecarFlowStore::open(root).unwrap();
    let produce_events: Vec<_> = store
        .events()
        .unwrap()
        .into_iter()
        .filter(|(_, e)| e.action == ReaVerb::Produce && !e.fulfills.is_empty())
        .collect();
    assert_eq!(
        produce_events.len(),
        2,
        "exactly two fulfilling Produce events: the original T1 Produce plus the T4 recovery \
         Produce — the stale T2 backfill never appended a third"
    );
}

#[test]
fn ambiguous_surface_errors_instead_of_guessing() {
    let dir = fixture();
    let root = dir.path();
    let recipes = root.join(".claude/epr-meta/recipes.yaml");

    // A second feature file whose path ALSO ends with a shared suffix, so an empty
    // `--surface-prefix` join (`/foo.feature`) matches both.
    write(
        root,
        "genesis/a2o/features/dataplane/resiliency-saga/other/foo.feature",
        "Feature: other\n  Scenario: x\n    Given a\n    Then b\n",
    );
    write(
        root,
        "genesis/a2o/features/dataplane/resiliency-saga/another/foo.feature",
        "Feature: another\n  Scenario: x\n    Given a\n    Then b\n",
    );
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "add ambiguous features"]);

    project::project(root, &recipes).expect("project runs");

    let report = root.join("report.json");
    let json = r#"{
  "generatedAt": "2026-07-25T00:00:00Z",
  "runId": "run-1",
  "profile": "alpha",
  "summary": {
    "scenarios": { "total": 1, "passed": 1, "failed": 0, "skipped": 0, "pending": 0 },
    "findings": { "total": 0, "bySource": {}, "byPillar": {} },
    "byConcern": {
      "resiliency": {
        "passed": 1,
        "failed": 0,
        "pending": 0,
        "scenarios": [
          { "name": "x", "status": "passed", "surface": "foo.feature" }
        ]
      }
    }
  },
  "findings": []
}
"#;
    write(root, "report.json", json);

    let opts = FulfillOptions {
        dry_run: false,
        surface_prefix: String::new(),
    };
    let err = fulfill(root, &report, &opts).expect_err("ambiguous surface must error");
    let message = err.to_string();
    assert!(message.contains("ambiguous"), "got: {message}");
}
