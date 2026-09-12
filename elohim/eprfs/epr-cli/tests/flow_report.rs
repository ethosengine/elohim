//! Integration tests over a synthetic tempdir repo for the bound-fold loop: `epr flow note
//! --measure` (the structured WRITE) and `epr flow report` (the READ that evaluates declared
//! bounds against it).
//!
//! Station one of `genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md`.
//! The fixture registry below is deliberately a *test fixture* rather than the live
//! `.claude/epr-meta/measures.yaml`: the live registry is authored by another owner and changes
//! under this crate's feet, and a test that asserted watermark arithmetic against it would be
//! measuring somebody else's editing cadence. The measure ids and watermarks mirror the ones the
//! integration seat declares (`memory-index-bytes@1` soft 20000 hard 24000; `gospel-bytes@1` hard
//! 12000; `memkit-report-tier-mb@1` soft 8; `cleanup-pressure@1` 120; `decompose-threshold@1` 40;
//! `entry-stale-days@1` 90) so the two stay honest about the same shape.
//!
//! Fixture shape follows `flow_note.rs`: a committed tempdir repo, because every note is dated by
//! the git HEAD it was authored against and an un-committed tree has nothing to date against.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use elohim_epr_cli::flow::note::{observe, NoteActor};
use elohim_epr_cli::flow::report::{
    declared_default, declared_recipes, report, BoundOutcome, OutcomeStatus, Recipe, ReportOptions,
};
use elohim_epr_cli::flow::FlowError;
use tempfile::TempDir;

const SUBJECT: &str = ".claude/memory/MEMORY.md";
const GOSPEL: &str = "CLAUDE.md";

/// The fixture registry: six measures and six bounds mirroring the integration seat's contract.
const MEASURES: &str = r#"measures-version: 1
measures:
  - id: memory-index-bytes
    version: 1
    family: memory-index
    unit: bytes
    default-authority: observation
    status: active
  - id: gospel-bytes
    version: 1
    family: gospel
    unit: bytes
    default-authority: observation
    status: active
  - id: memkit-report-tier-mb
    version: 1
    family: report-tier
    unit: megabytes
    default-authority: observation
    status: active
  - id: cleanup-pressure
    version: 1
    family: cleanup-pressure
    unit: pressure-points
    default-authority: observation
    status: active
  - id: decompose-threshold
    version: 1
    family: placement
    unit: gap-items
    default-authority: observation
    status: active
  - id: entry-stale-days
    version: 1
    family: memory
    unit: days
    default-authority: observation
    status: active
  - id: mempalace-mine-grace-seconds
    version: 1
    family: mempalace
    unit: seconds
    default-authority: observation
    status: active
  - id: recall-unmetered-bytes
    version: 1
    family: recall-journey
    unit: bytes
    default-authority: observation
    status: active

lenses:
  - id: memory-index-bytes-ceiling
    version: 1
    binding: binding-local
    class: measure
    consumes: [memory-index-bytes@1]
    context: memory-index-projection
    soft: 20000
    hard: 24000
    status: active
  - id: gospel-bytes-ceiling
    version: 1
    binding: binding-local
    class: inject
    consumes: [gospel-bytes@1]
    context: session-headline
    hard: 12000
    status: active
  - id: memkit-report-tier-mb-ceiling
    version: 1
    binding: binding-local
    class: measure
    consumes: [memkit-report-tier-mb@1]
    context: report-tier
    soft: 8
    status: active
  - id: cleanup-pressure-ceiling
    version: 1
    binding: binding-local
    class: measure
    consumes: [cleanup-pressure@1]
    context: cleanup-pressure
    hard: 120
    status: active
  - id: decompose-threshold-ceiling
    version: 1
    binding: binding-local
    class: measure
    consumes: [decompose-threshold@1]
    context: placement
    hard: 40
    status: active
  - id: entry-stale-days-ceiling
    version: 1
    binding: binding-local
    class: measure
    consumes: [entry-stale-days@1]
    context: memory-review
    hard: 90
    status: active
  - id: mempalace-grace-ceiling
    version: 1
    binding: binding-local
    class: measure
    consumes: [mempalace-mine-grace-seconds@1]
    context: mempalace
    hard: 604800
    status: active
  - id: recall-unmetered-bytes-ceiling
    version: 1
    headline: recall
    compare: at-or-above
    binding: binding-local
    class: inject
    consumes: [recall-unmetered-bytes@1]
    context: session-headline
    soft: 1
    hard: 20000
    status: active
"#;

/// A ceiling registry that DOES consume a measure, so the policies half of the report is exercised
/// rather than merely tolerated.
const POLICIES: &str = r#"epr-meta-policies-version: 1
policies:
  - id: source-file-loc-ceiling
    version: 1
    class: measure
    measure:
      kind: level
      loc-soft: 3000
      loc-hard: 7000
    status: active
  - id: scope-drift-ceiling
    version: 1
    class: measure
    consumes: scope-drift@1
    subject: genesis/manifests/cluster-state.yaml
    measure:
      kind: level
      loc-hard: 5
    status: active
"#;

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

fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, ".claude/epr-meta/measures.yaml", MEASURES);
    write(root, ".claude/epr-meta/policies.yaml", POLICIES);
    write(root, SUBJECT, "# memory index\n");
    write(root, GOSPEL, "# gospel\n");
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "fixture"]);
    dir
}

fn measures_path(root: &Path) -> PathBuf {
    root.join(".claude/epr-meta/measures.yaml")
}

fn options(root: &Path) -> ReportOptions {
    ReportOptions::new(root)
}

/// The primary recipe's method pin.
fn method(root: &Path) -> elohim_epr_cli::flow::report::RecipeRef {
    report(root, &options(root))
        .unwrap()
        .primary()
        .unwrap()
        .recipe
        .clone()
}

/// The primary recipe's outcome list — what every pre-recipe assertion in this file reads.
fn outcomes(payload: &elohim_epr_cli::flow::report::ReportPayload) -> &[BoundOutcome] {
    payload.outcomes()
}

/// Mint one structured observation with no env.
fn fold(root: &Path, measure: &str, subject: &str, value: f64) -> String {
    fold_env(root, measure, subject, value, &BTreeMap::new())
}

fn fold_env(
    root: &Path,
    measure: &str,
    subject: &str,
    value: f64,
    env: &BTreeMap<String, String>,
) -> String {
    observe(
        root,
        "observation",
        measure,
        subject,
        value,
        None,
        env,
        None,
        &NoteActor::default(),
        &measures_path(root),
    )
    .expect("observation runs")
    .record_cid
}

fn outcome_for<'a>(
    payload: &'a elohim_epr_cli::flow::report::ReportPayload,
    bound: &str,
) -> &'a elohim_epr_cli::flow::report::BoundOutcome {
    outcomes(payload)
        .iter()
        .find(|o| o.bound == bound)
        .unwrap_or_else(|| panic!("no outcome for bound `{bound}`"))
}

fn sidecar_lines(root: &Path) -> usize {
    std::fs::read_to_string(root.join(".eprfs/status/flows.jsonl"))
        .map(|t| t.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0)
}

// ── (1) registry refusal ────────────────────────────────────────────────────────────────────

#[test]
fn an_undeclared_measure_is_refused_naming_the_registry_and_writes_nothing() {
    let dir = fixture();
    let root = dir.path();

    let err = observe(
        root,
        "observation",
        "not-a-declared-measure@1",
        SUBJECT,
        1.0,
        None,
        &BTreeMap::new(),
        None,
        &NoteActor::default(),
        &measures_path(root),
    )
    .expect_err("an undeclared measure must be refused, never folded");

    assert!(matches!(err, FlowError::InvalidArguments(_)));
    let message = err.to_string();
    assert!(
        message.contains(".claude/epr-meta/measures.yaml"),
        "the refusal must name the registry path; got: {message}"
    );
    assert!(
        message.contains("unknown measure"),
        "the refusal must say what was wrong; got: {message}"
    );
    assert_eq!(
        sidecar_lines(root),
        0,
        "a refused observation appends nothing"
    );
}

#[test]
fn a_bare_measure_id_is_refused_because_a_pin_is_a_declared_dependency() {
    let dir = fixture();
    let root = dir.path();
    let err = observe(
        root,
        "observation",
        "memory-index-bytes",
        SUBJECT,
        1.0,
        None,
        &BTreeMap::new(),
        None,
        &NoteActor::default(),
        &measures_path(root),
    )
    .expect_err("a bare id is recency-shaped and must be refused");
    assert!(err.to_string().contains("version pin"), "{err}");
    assert_eq!(sidecar_lines(root), 0);
}

#[test]
fn a_magnitude_on_a_non_observation_kind_is_refused() {
    let dir = fixture();
    let root = dir.path();
    let err = observe(
        root,
        "correction",
        "memory-index-bytes@1",
        SUBJECT,
        1.0,
        None,
        &BTreeMap::new(),
        None,
        &NoteActor::default(),
        &measures_path(root),
    )
    .expect_err("a correction carrying a magnitude is a record whose tag and body disagree");
    assert!(err.to_string().contains("observation"), "{err}");
    assert_eq!(sidecar_lines(root), 0);
}

// ── (2) identical observations dedupe to one CID ────────────────────────────────────────────

#[test]
fn two_identical_observations_mint_one_cid_and_one_row() {
    let dir = fixture();
    let root = dir.path();

    let first = observe(
        root,
        "observation",
        "memory-index-bytes@1",
        SUBJECT,
        19_000.0,
        None,
        &BTreeMap::new(),
        None,
        &NoteActor::default(),
        &measures_path(root),
    )
    .unwrap();
    let second = observe(
        root,
        "observation",
        "memory-index-bytes@1",
        SUBJECT,
        19_000.0,
        None,
        &BTreeMap::new(),
        None,
        &NoteActor::default(),
        &measures_path(root),
    )
    .unwrap();

    assert!(first.appended, "the first observation appends");
    assert!(!second.appended, "the second is a no-op, not a second row");
    assert_eq!(
        first.record_cid, second.record_cid,
        "one measurement, one address"
    );
    assert_eq!(sidecar_lines(root), 1);
}

#[test]
fn an_integral_value_spelled_two_ways_is_one_measurement() {
    let dir = fixture();
    let root = dir.path();
    let a = fold(root, "memory-index-bytes@1", SUBJECT, 8.0);
    let b = fold(root, "memory-index-bytes@1", SUBJECT, 8.000);
    assert_eq!(a, b, "`8` and `8.0` are the same number and the same fold");
    assert_eq!(sidecar_lines(root), 1);
}

#[test]
fn a_different_env_is_a_different_measurement() {
    let dir = fixture();
    let root = dir.path();
    let plain = fold(root, "cleanup-pressure@1", SUBJECT, 45.0);
    let scoped = fold_env(
        root,
        "cleanup-pressure@1",
        SUBJECT,
        45.0,
        &BTreeMap::from([("host".to_string(), "alpha".to_string())]),
    );
    assert_ne!(
        plain, scoped,
        "env is part of the memoization key, so two envs are two folds"
    );
    assert_eq!(sidecar_lines(root), 2);
}

#[test]
fn env_slot_order_follows_the_key_not_the_argument_order() {
    let dir = fixture();
    let root = dir.path();
    let forward = fold_env(
        root,
        "cleanup-pressure@1",
        SUBJECT,
        45.0,
        &BTreeMap::from([
            ("a".to_string(), "1".to_string()),
            ("b".to_string(), "2".to_string()),
        ]),
    );
    let reverse = fold_env(
        root,
        "cleanup-pressure@1",
        SUBJECT,
        45.0,
        &BTreeMap::from([
            ("b".to_string(), "2".to_string()),
            ("a".to_string(), "1".to_string()),
        ]),
    );
    assert_eq!(forward, reverse, "a set has no argument order");
    assert_eq!(sidecar_lines(root), 1);
}

// ── (3) a bound with no fold is skipped ─────────────────────────────────────────────────────

#[test]
fn a_bound_with_no_fold_is_skipped_naming_the_missing_measure_never_zero() {
    let dir = fixture();
    let root = dir.path();

    let payload = report(root, &options(root)).expect("report runs");
    assert!(
        !outcomes(&payload).is_empty(),
        "the fixture declares bounds to evaluate"
    );
    for outcome in outcomes(&payload) {
        assert_eq!(
            outcome.outcome,
            OutcomeStatus::Skipped,
            "nothing has been measured yet, so nothing may read as passed"
        );
        assert!(
            outcome.observed.is_none(),
            "a skipped bound observes NOTHING — a zero here is the false reassurance this replaces"
        );
        assert!(outcome.fold_cid.is_none());
        assert!(
            outcome.summary.contains(&outcome.measure),
            "a skipped outcome names the measure whose fold is missing; got: {}",
            outcome.summary
        );
    }
    assert_eq!(payload.totals.passed, 0);
    assert_eq!(payload.totals.failed, 0);
    assert_eq!(payload.totals.skipped, outcomes(&payload).len());
}

#[test]
fn a_fold_for_one_bound_leaves_its_neighbours_skipped() {
    let dir = fixture();
    let root = dir.path();
    fold(root, "memory-index-bytes@1", SUBJECT, 100.0);

    let payload = report(root, &options(root)).unwrap();
    assert_eq!(
        outcome_for(&payload, "memory-index-bytes-ceiling@1").outcome,
        OutcomeStatus::Passed
    );
    assert_eq!(
        outcome_for(&payload, "gospel-bytes-ceiling@1").outcome,
        OutcomeStatus::Skipped,
        "one measured bound must not vouch for an unmeasured one"
    );
}

#[test]
fn a_fold_under_a_different_env_is_not_evidence_for_a_bound_that_declares_one() {
    let dir = fixture();
    let root = dir.path();
    // The policies-half bound declares `subject:` but no env; the lens half declares neither. The
    // subject filter is the one this asserts: a fold about CLAUDE.md is not evidence about MEMORY.md
    // for a bound that names MEMORY.md.
    write(
        root,
        ".claude/epr-meta/measures.yaml",
        &MEASURES.replace(
            "    context: memory-index-projection\n    soft: 20000",
            "    context: memory-index-projection\n    subject: .claude/memory/MEMORY.md\n    soft: 20000",
        ),
    );
    fold(root, "memory-index-bytes@1", GOSPEL, 100.0);

    let payload = report(root, &options(root)).unwrap();
    assert_eq!(
        outcome_for(&payload, "memory-index-bytes-ceiling@1").outcome,
        OutcomeStatus::Skipped,
        "a fold about another subject is not evidence about this one"
    );
}

// ── (4) soft and hard watermark evaluation ──────────────────────────────────────────────────

#[test]
fn below_the_soft_watermark_is_a_clean_pass() {
    let dir = fixture();
    let root = dir.path();
    let cid = fold(root, "memory-index-bytes@1", SUBJECT, 19_999.0);

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "memory-index-bytes-ceiling@1");
    assert_eq!(outcome.outcome, OutcomeStatus::Passed);
    assert!(
        !outcome.summary.starts_with("warn:"),
        "below soft carries no warning; got: {}",
        outcome.summary
    );
    assert_eq!(outcome.observed, Some(19_999.0));
    assert_eq!(
        outcome.unit.as_deref(),
        Some("bytes"),
        "the unit is adopted from the measure row"
    );
    assert_eq!(outcome.fold_cid.as_deref(), Some(cid.as_str()));
    assert_eq!(outcome.watermarks.soft, Some(20_000.0));
    assert_eq!(outcome.watermarks.hard, Some(24_000.0));
}

#[test]
fn between_soft_and_hard_is_a_pass_carrying_a_warn_summary() {
    let dir = fixture();
    let root = dir.path();
    fold(root, "memory-index-bytes@1", SUBJECT, 22_500.0);

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "memory-index-bytes-ceiling@1");
    assert_eq!(
        outcome.outcome,
        OutcomeStatus::Passed,
        "a soft crossing nudges; it never fails"
    );
    assert!(
        outcome.summary.starts_with("warn:"),
        "the nudge must be legible in the summary; got: {}",
        outcome.summary
    );
    assert!(outcome.summary.contains("20000"), "{}", outcome.summary);
    assert_eq!(outcome.observed, Some(22_500.0));
    assert_eq!(payload.totals.failed, 0);
}

#[test]
fn above_the_hard_watermark_fails() {
    let dir = fixture();
    let root = dir.path();
    fold(root, "memory-index-bytes@1", SUBJECT, 24_001.0);

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "memory-index-bytes-ceiling@1");
    assert_eq!(outcome.outcome, OutcomeStatus::Failed);
    assert!(outcome.summary.contains("hard"), "{}", outcome.summary);
    assert_eq!(outcome.observed, Some(24_001.0));
    assert_eq!(payload.totals.failed, 1);
}

#[test]
fn a_value_sitting_exactly_on_a_watermark_has_not_crossed_it() {
    let dir = fixture();
    let root = dir.path();
    fold(root, "memory-index-bytes@1", SUBJECT, 24_000.0);
    let payload = report(root, &options(root)).unwrap();
    assert_eq!(
        outcome_for(&payload, "memory-index-bytes-ceiling@1").outcome,
        OutcomeStatus::Passed,
        "a watermark is the last acceptable value, not the first unacceptable one"
    );

    let dir2 = fixture();
    let root2 = dir2.path();
    fold(root2, "memory-index-bytes@1", SUBJECT, 20_000.0);
    let payload2 = report(root2, &options(root2)).unwrap();
    let outcome = outcome_for(&payload2, "memory-index-bytes-ceiling@1");
    assert_eq!(outcome.outcome, OutcomeStatus::Passed);
    assert!(!outcome.summary.starts_with("warn:"), "{}", outcome.summary);
}

#[test]
fn a_single_threshold_row_evaluates_with_no_soft_watermark() {
    let dir = fixture();
    let root = dir.path();
    fold(root, "cleanup-pressure@1", SUBJECT, 121.0);

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "cleanup-pressure-ceiling@1");
    assert_eq!(outcome.outcome, OutcomeStatus::Failed);
    assert_eq!(outcome.watermarks.soft, None);
    assert_eq!(outcome.watermarks.hard, Some(120.0));
}

#[test]
fn a_soft_only_row_never_fails_however_far_past_it_the_fold_sits() {
    let dir = fixture();
    let root = dir.path();
    fold(root, "memkit-report-tier-mb@1", SUBJECT, 40.0);

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "memkit-report-tier-mb-ceiling@1");
    assert_eq!(
        outcome.outcome,
        OutcomeStatus::Passed,
        "a row with no hard watermark declares no breach condition"
    );
    assert!(outcome.summary.starts_with("warn:"), "{}", outcome.summary);
}

#[test]
fn the_latest_fold_wins_when_a_subject_is_measured_twice() {
    let dir = fixture();
    let root = dir.path();
    fold(root, "memory-index-bytes@1", SUBJECT, 30_000.0);
    let later = fold(root, "memory-index-bytes@1", SUBJECT, 100.0);

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "memory-index-bytes-ceiling@1");
    assert_eq!(
        outcome.observed,
        Some(100.0),
        "the report reads the LATEST fold, not the worst one"
    );
    assert_eq!(outcome.fold_cid.as_deref(), Some(later.as_str()));
    assert_eq!(outcome.outcome, OutcomeStatus::Passed);
}

#[test]
fn a_ceiling_row_in_policies_that_consumes_a_measure_is_a_bound_and_one_that_does_not_is_skipped_entirely(
) {
    let dir = fixture();
    let root = dir.path();
    let payload = report(root, &options(root)).unwrap();
    let ids: Vec<&str> = outcomes(&payload)
        .iter()
        .map(|o| o.bound.as_str())
        .collect();
    assert!(
        !ids.contains(&"source-file-loc-ceiling@1"),
        "a ceiling row naming no measure is not a bound and must not appear as permanent noise"
    );
    assert!(
        ids.contains(&"scope-drift-ceiling@1"),
        "a ceiling row that consumes a measure IS a bound; got {ids:?}"
    );
    let scope = outcome_for(&payload, "scope-drift-ceiling@1");
    assert_eq!(scope.source, "ceiling");
    assert_eq!(scope.subject, "genesis/manifests/cluster-state.yaml");
    assert_eq!(scope.watermarks.hard, Some(5.0), "loc-hard is read as hard");
}

#[test]
fn a_bound_filter_selects_one_row_by_bare_id_or_by_pin() {
    let dir = fixture();
    let root = dir.path();

    let mut opts = options(root);
    opts.bound = Some("memory-index-bytes-ceiling".into());
    assert_eq!(report(root, &opts).unwrap().outcomes().len(), 1);

    opts.bound = Some("memory-index-bytes-ceiling@1".into());
    assert_eq!(report(root, &opts).unwrap().outcomes().len(), 1);

    opts.bound = Some("no-such-bound".into());
    let err =
        report(root, &opts).expect_err("an unknown bound filter is a refusal, not an empty report");
    assert!(err.to_string().contains("no declared bound"), "{err}");
}

// ── (5) headline order ──────────────────────────────────────────────────────────────────────

#[test]
fn the_headline_prints_five_lines_in_the_gospel_declared_order() {
    let dir = fixture();
    let root = dir.path();
    let payload = report(root, &options(root)).unwrap();

    let lines: Vec<String> = ["recall", "mempalace", "cleanup", "scope", "budget"]
        .iter()
        .map(|slot| payload.headline_line(slot))
        .collect();

    assert!(lines[0].starts_with("recall:"), "{:?}", lines);
    assert!(lines[1].starts_with("mempalace:"), "{:?}", lines);
    assert!(lines[2].starts_with("cleanup:"), "{:?}", lines);
    assert!(lines[3].starts_with("scope:"), "{:?}", lines);
    assert!(
        lines[4].starts_with("memory-budget:"),
        "the memory-budget line comes last; got {:?}",
        lines
    );
}

#[test]
fn a_headline_slot_with_no_fold_says_skipped_naming_its_measure() {
    let dir = fixture();
    let root = dir.path();
    let payload = report(root, &options(root)).unwrap();
    let line = payload.headline_line("cleanup");
    assert_eq!(line, "cleanup: skipped (no fold for cleanup-pressure@1)");
}

/// Slot 0's absence has ONE meaning: nobody has walked the governed recall entry since the last
/// reset. "no fold for recall-unmetered-bytes@1" names the registry row; it does not tell a session
/// reader what to do, which is the whole job of the first line it reads.
#[test]
fn the_recall_slot_names_a_missing_journey_rather_than_a_missing_row() {
    let dir = fixture();
    let root = dir.path();
    let payload = report(root, &options(root)).unwrap();
    assert_eq!(
        payload.headline_line("recall"),
        "recall: skipped — no journey fold"
    );
}

#[test]
fn a_headline_slot_with_no_declared_bound_says_so_rather_than_vanishing() {
    let dir = fixture();
    let root = dir.path();
    // The fixture measures registry declares no `scope-*` measure; the policies half does, so
    // removing the policies file leaves the scope slot genuinely unbound.
    std::fs::remove_file(root.join(".claude/epr-meta/policies.yaml")).unwrap();
    let payload = report(root, &options(root)).unwrap();
    assert_eq!(
        payload.headline_line("scope"),
        "scope: skipped (no bound declared)",
        "an absent line reads as 'nothing to report'; an explicit one reads as 'nobody declared it'"
    );
}

#[test]
fn the_headline_carries_a_warning_marker_on_soft_and_hard_crossings_alike() {
    let dir = fixture();
    let root = dir.path();
    fold(root, "memory-index-bytes@1", SUBJECT, 22_500.0);
    fold(root, "cleanup-pressure@1", SUBJECT, 121.0);
    let payload = report(root, &options(root)).unwrap();

    let budget = payload.headline_line("budget");
    assert!(budget.starts_with("memory-budget: ⚠"), "{budget}");
    let cleanup = payload.headline_line("cleanup");
    assert!(cleanup.starts_with("cleanup: ⚠"), "{cleanup}");
    assert!(cleanup.contains("failed"), "{cleanup}");
}

#[test]
fn a_clean_pass_renders_without_a_warning_marker() {
    let dir = fixture();
    let root = dir.path();
    fold(root, "cleanup-pressure@1", SUBJECT, 45.0);
    let line = report(root, &options(root))
        .unwrap()
        .headline_line("cleanup");
    assert!(line.starts_with("cleanup: 45"), "{line}");
    assert!(!line.contains('⚠'), "{line}");
}

// ── (6) method CIDs ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_method_pins_the_registry_bytes_and_moves_when_they_do() {
    let dir = fixture();
    let root = dir.path();

    let before = method(root);
    assert!(
        before
            .measures_cid
            .as_deref()
            .is_some_and(|cid| cid.starts_with("bafkrei")),
        "registry bytes are RAW, not dag-cbor; got {:?}",
        before.measures_cid
    );
    assert!(before
        .policies_cid
        .as_deref()
        .is_some_and(|cid| cid.starts_with("bafkrei")));

    // Same bytes, second read: the same method.
    let again = method(root);
    assert_eq!(before.measures_cid, again.measures_cid);
    assert_eq!(before.policies_cid, again.policies_cid);

    // Edit the watermark: same path, different method.
    write(
        root,
        ".claude/epr-meta/measures.yaml",
        &MEASURES.replace("hard: 24000", "hard: 26000"),
    );
    let after = method(root);
    assert_ne!(
        before.measures_cid, after.measures_cid,
        "a report that names its inputs by path alone cannot tell these two runs apart"
    );
    assert_eq!(
        before.policies_cid, after.policies_cid,
        "the untouched registry keeps its pin"
    );
}

#[test]
fn an_absent_registry_is_a_null_pin_not_a_failure() {
    let dir = fixture();
    let root = dir.path();
    std::fs::remove_file(root.join(".claude/epr-meta/policies.yaml")).unwrap();

    let payload = report(root, &options(root)).expect("a repo with only lenses still reports");
    assert!(payload.primary().unwrap().recipe.measures_cid.is_some());
    assert!(payload.primary().unwrap().recipe.policies_cid.is_none());
}

#[test]
fn a_repository_with_no_registry_at_all_is_refused_naming_both_paths() {
    let dir = fixture();
    let root = dir.path();
    std::fs::remove_file(root.join(".claude/epr-meta/measures.yaml")).unwrap();
    std::fs::remove_file(root.join(".claude/epr-meta/policies.yaml")).unwrap();

    let err = report(root, &options(root)).expect_err("reporting over no rows is not a report");
    let message = err.to_string();
    assert!(message.contains("measures.yaml"), "{message}");
    assert!(message.contains("policies.yaml"), "{message}");
}

#[test]
fn a_repository_with_no_sidecar_reports_every_bound_as_skipped_rather_than_crashing() {
    let dir = fixture();
    let root = dir.path();
    assert!(
        !root.join(".eprfs/status/flows.jsonl").exists(),
        "the fixture has never been written to"
    );
    let payload = report(root, &options(root)).expect("a fresh clone still reports");
    assert!(payload.totals.skipped > 0);
    assert_eq!(payload.totals.passed + payload.totals.failed, 0);
}

#[test]
fn a_failed_bound_still_returns_a_report_rather_than_an_error() {
    let dir = fixture();
    let root = dir.path();
    fold(root, "memory-index-bytes@1", SUBJECT, 99_999.0);
    let payload = report(root, &options(root)).expect("the report never refuses over a red bound");
    assert_eq!(payload.totals.failed, 1);
    assert_eq!(payload.command, "flow report");
}

#[test]
fn a_bound_outcome_projects_onto_the_gate_finding_vocabulary_without_reporting_silence_as_health() {
    use elohim_epr_cli::report::FindingStatus;

    let dir = fixture();
    let root = dir.path();
    let payload = report(root, &options(root)).unwrap();
    let skipped = outcome_for(&payload, "memory-index-bytes-ceiling@1").to_finding();
    assert_eq!(
        skipped.status,
        FindingStatus::Warn,
        "an unmeasured bound is a gap in evidence, never an informational pass"
    );
    assert_eq!(skipped.code, "bound:memory-index-bytes-ceiling@1");

    fold(root, "memory-index-bytes@1", SUBJECT, 1.0);
    let payload = report(root, &options(root)).unwrap();
    assert_eq!(
        outcome_for(&payload, "memory-index-bytes-ceiling@1")
            .to_finding()
            .status,
        FindingStatus::Pass
    );
}

#[test]
fn the_json_payload_carries_the_method_and_the_three_valued_outcome_vocabulary() {
    let dir = fixture();
    let root = dir.path();
    fold(root, "memory-index-bytes@1", SUBJECT, 100.0);
    let payload = report(root, &options(root)).unwrap();
    let json = serde_json::to_value(&payload).unwrap();

    let primary = &json["recipes"][0];
    assert_eq!(primary["primary"], true);
    assert!(primary["recipe"]["measuresCid"].is_string());
    assert!(primary["recipe"]["policiesCid"].is_string());
    let outcome = primary["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["bound"] == "memory-index-bytes-ceiling@1")
        .unwrap();
    assert_eq!(outcome["outcome"], "passed");
    assert_eq!(outcome["observed"], 100.0);
    assert_eq!(outcome["watermarks"]["soft"], 20000.0);
    assert_eq!(outcome["watermarks"]["hard"], 24000.0);
    assert!(outcome["foldCid"].is_string());
    assert!(outcome["summary"].is_string());
}

// ── (7) plural recipes — the policy set is one lens among several ───────────────────────────

/// A second recipe directory whose only difference is a tighter memory-index bound.
fn strict_recipe(root: &Path) -> Recipe {
    write(
        root,
        "governance/strict/measures.yaml",
        &MEASURES.replace(
            "    soft: 20000\n    hard: 24000",
            "    soft: 50\n    hard: 100",
        ),
    );
    write(root, "governance/strict/policies.yaml", POLICIES);
    Recipe::at_dir(root, Path::new("governance/strict"))
}

#[test]
fn the_default_recipe_is_declared_not_implicit() {
    let dir = fixture();
    let root = dir.path();

    // No `.epr-meta/manifest.md`: the declared fallback is the .claude/epr-meta pair.
    let fallback = declared_default(root);
    assert_eq!(fallback.name, "claude-epr-meta");
    assert!(fallback
        .measures
        .ends_with(".claude/epr-meta/measures.yaml"));
    assert!(fallback
        .policies
        .ends_with(".claude/epr-meta/policies.yaml"));

    // A manifest that declares one wins.
    write(
        root,
        ".epr-meta/manifest.md",
        "---\nepr-meta-version: 1\npolicy-recipe: governance/strict\nroot: true\n---\n\n# body\n",
    );
    let declared = declared_default(root);
    assert_eq!(declared.name, "strict");
    assert!(declared
        .measures
        .ends_with("governance/strict/measures.yaml"));
    assert!(declared
        .policies
        .ends_with("governance/strict/policies.yaml"));
}

#[test]
fn a_manifest_without_the_key_falls_back_rather_than_refusing() {
    let dir = fixture();
    let root = dir.path();
    write(
        root,
        ".epr-meta/manifest.md",
        "---\nepr-meta-version: 1\nroot: true\n---\n\n# body\n",
    );
    assert_eq!(declared_default(root).name, "claude-epr-meta");
}

#[test]
fn two_recipes_read_the_same_folds_and_are_grouped_side_by_side_never_merged() {
    let dir = fixture();
    let root = dir.path();
    let strict = strict_recipe(root);
    fold(root, "memory-index-bytes@1", SUBJECT, 200.0);

    let mut opts = options(root);
    opts.recipes = vec![opts.recipes[0].clone(), strict];
    let payload = report(root, &opts).unwrap();

    assert_eq!(payload.recipes.len(), 2, "two lenses, two groups");
    assert!(
        payload.recipes[0].primary,
        "the first declared recipe is primary"
    );
    assert!(!payload.recipes[1].primary);

    let lenient = payload.recipes[0]
        .outcomes
        .iter()
        .find(|o| o.bound == "memory-index-bytes-ceiling@1")
        .unwrap();
    let strict_outcome = payload.recipes[1]
        .outcomes
        .iter()
        .find(|o| o.bound == "memory-index-bytes-ceiling@1")
        .unwrap();

    assert_eq!(lenient.outcome, OutcomeStatus::Passed);
    assert_eq!(
        strict_outcome.outcome,
        OutcomeStatus::Failed,
        "the same fold read through a tighter lens is a different reading, not a correction"
    );
    assert_eq!(
        lenient.observed, strict_outcome.observed,
        "one set of records, read twice"
    );
    assert_ne!(
        lenient.recipe.measures_cid, strict_outcome.recipe.measures_cid,
        "different lenses have different method pins"
    );
}

#[test]
fn every_outcome_carries_its_recipe_so_a_flattening_consumer_keeps_the_provenance() {
    let dir = fixture();
    let root = dir.path();
    let strict = strict_recipe(root);
    let mut opts = options(root);
    opts.recipes = vec![opts.recipes[0].clone(), strict];
    let payload = report(root, &opts).unwrap();

    for group in &payload.recipes {
        for outcome in &group.outcomes {
            assert_eq!(
                outcome.recipe, group.recipe,
                "an outcome lifted out of its group still names its lens"
            );
        }
    }
    // Flattened, the two lenses remain distinguishable.
    let flat: Vec<&str> = payload
        .recipes
        .iter()
        .flat_map(|g| g.outcomes.iter())
        .map(|o| o.recipe.name.as_str())
        .collect();
    assert!(flat.contains(&"claude-epr-meta"));
    assert!(flat.contains(&"strict"));
}

#[test]
fn the_totals_roll_up_across_recipes_without_adjudicating_between_them() {
    let dir = fixture();
    let root = dir.path();
    let strict = strict_recipe(root);
    let mut opts = options(root);
    opts.recipes = vec![opts.recipes[0].clone(), strict];
    let payload = report(root, &opts).unwrap();

    let summed = payload.recipes.iter().fold((0, 0, 0), |acc, r| {
        (
            acc.0 + r.totals.passed,
            acc.1 + r.totals.failed,
            acc.2 + r.totals.skipped,
        )
    });
    assert_eq!(
        (
            payload.totals.passed,
            payload.totals.failed,
            payload.totals.skipped
        ),
        summed
    );
}

#[test]
fn the_headline_prints_the_primary_lines_then_the_recipe_handle_then_one_alt_line_each() {
    let dir = fixture();
    let root = dir.path();
    let strict = strict_recipe(root);
    fold(root, "memory-index-bytes@1", SUBJECT, 200.0);

    let mut opts = options(root);
    opts.recipes = vec![opts.recipes[0].clone(), strict];
    let payload = report(root, &opts).unwrap();

    // The five slot lines still come from the PRIMARY recipe alone.
    let budget = payload.headline_line("budget");
    assert!(
        budget.starts_with("memory-budget: 200"),
        "the primary's lenient reading, not the alt's failure; got {budget}"
    );

    let primary = payload.primary().unwrap();
    let handle = primary.recipe.handle();
    assert!(
        handle.starts_with("claude-epr-meta@bafkrei"),
        "the handle names the recipe and pins its measures bytes; got {handle}"
    );

    let alts = payload.alt_lines();
    assert_eq!(alts.len(), 1, "one alt line per extra recipe, never more");
    assert!(alts[0].starts_with("alt: strict@bafkrei"), "{}", alts[0]);
    assert!(alts[0].contains("failed"), "{}", alts[0]);
    // Counts, never lines: the alt summary must not carry per-bound text.
    assert!(
        !alts[0].contains("memory-index-bytes-ceiling"),
        "an alt recipe gets counts, so the headline's shape cannot grow with the lens count"
    );
}

#[test]
fn a_single_recipe_headline_carries_no_alt_lines() {
    let dir = fixture();
    let root = dir.path();
    let payload = report(root, &options(root)).unwrap();
    assert!(payload.alt_lines().is_empty());
    assert!(payload.primary().unwrap().primary);
}

#[test]
fn a_recipe_directory_holding_no_registry_is_refused_naming_the_recipe() {
    let dir = fixture();
    let root = dir.path();
    let mut opts = options(root);
    opts.recipes = vec![Recipe::at_dir(root, Path::new("governance/absent"))];
    let err = report(root, &opts).expect_err("a recipe with no rows is not a lens");
    let message = err.to_string();
    assert!(message.contains("absent"), "{message}");
    assert!(message.contains("measures.yaml"), "{message}");
}

#[test]
fn an_empty_recipe_list_is_refused_rather_than_reported_as_all_clear() {
    let dir = fixture();
    let root = dir.path();
    let mut opts = options(root);
    opts.recipes = Vec::new();
    let err = report(root, &opts).expect_err("no lens is not a clean report");
    assert!(err.to_string().contains("recipe"), "{err}");
}

#[test]
fn the_row_binding_rung_reaches_the_outcome_uninterpreted() {
    let dir = fixture();
    let root = dir.path();
    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "memory-index-bytes-ceiling@1");
    assert_eq!(
        outcome.binding.as_deref(),
        Some("binding-local"),
        "the ladder rung is carried through, not read: the floor measures, the row decides cost"
    );
    let inject = outcome_for(&payload, "gospel-bytes-ceiling@1");
    assert_eq!(inject.binding.as_deref(), Some("binding-local"));
}

#[test]
fn the_json_payload_groups_outcomes_by_recipe() {
    let dir = fixture();
    let root = dir.path();
    let strict = strict_recipe(root);
    let mut opts = options(root);
    opts.recipes = vec![opts.recipes[0].clone(), strict];
    fold(root, "memory-index-bytes@1", SUBJECT, 200.0);
    let payload = report(root, &opts).unwrap();
    let json = serde_json::to_value(&payload).unwrap();

    let recipes = json["recipes"].as_array().unwrap();
    assert_eq!(recipes.len(), 2);
    assert_eq!(recipes[0]["primary"], true);
    assert_eq!(recipes[1]["primary"], false);
    assert_eq!(recipes[0]["recipe"]["name"], "claude-epr-meta");
    assert_eq!(recipes[1]["recipe"]["name"], "strict");
    assert!(recipes[0]["recipe"]["measuresCid"].is_string());
    assert!(recipes[0]["totals"]["passed"].is_number());

    let outcome = recipes[0]["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["bound"] == "memory-index-bytes-ceiling@1")
        .unwrap();
    assert_eq!(outcome["recipe"]["name"], "claude-epr-meta");
    assert!(outcome["recipe"]["measuresCid"].is_string());
    assert_eq!(outcome["binding"], "binding-local");
    assert!(json["totals"]["passed"].is_number());
}

// ── (8) seam fixes — help-probe discoverability and the repository-root subject ─────────────

/// The exact probe `.claude/hooks/_observation.py` runs to decide whether the running binary can
/// append a structured observation. A miss makes every drift-signal hook fall back SILENTLY, so
/// this is a capability contract rather than a documentation preference.
#[test]
fn both_usage_strings_name_the_structured_observation_flag_the_hook_probes_for() {
    let note = elohim_epr_cli::flow::note_usage();
    let flow = elohim_epr_cli::flow::usage_text();

    for (label, text) in [("note --help", &note), ("bare flow usage", &flow)] {
        assert!(
            text.contains("--measure"),
            "{label} must name --measure or _observation.py concludes the verb is absent"
        );
        assert!(text.contains("--subject"), "{label} must name --subject");
        assert!(text.contains("--value"), "{label} must name --value");
    }
    // The signature shape the coordinator's ruling names, so a reader learns the arm's whole shape
    // from one line rather than by trial.
    assert!(note.contains("--on <target> --kind <kind>"), "{note}");
    assert!(
        note.contains(
            "[--measure <id@version> --subject <path> --value <n> [--unit <u>] [--env k=v]...]"
        ),
        "{note}"
    );
    assert!(
        flow.contains("--unit <u>") && flow.contains("--env k=v"),
        "{flow}"
    );
    // The repository-root subject is discoverable from help alone.
    assert!(note.contains("`.` names the repository"), "{note}");
}

#[test]
fn help_answers_on_stdout_with_success_rather_than_as_an_argument_error() {
    // `run` is the same entry point the CLI dispatches through; a zero exit is what tells the
    // probe (and a person) that the question was answered rather than refused.
    let args: Vec<String> = vec!["note".into(), "--help".into()];
    assert_eq!(
        elohim_epr_cli::flow::run(&args).unwrap(),
        std::process::ExitCode::SUCCESS
    );
    let top: Vec<String> = vec!["--help".into()];
    assert_eq!(
        elohim_epr_cli::flow::run(&top).unwrap(),
        std::process::ExitCode::SUCCESS
    );
}

#[test]
fn the_repository_root_is_a_valid_subject_for_a_repository_wide_measurement() {
    let dir = fixture();
    let root = dir.path();

    let outcome = observe(
        root,
        "observation",
        "cleanup-pressure@1",
        ".",
        250.0,
        None,
        &BTreeMap::new(),
        None,
        &NoteActor::default(),
        &measures_path(root),
    )
    .expect("`.` names the repository, not a missing file");

    assert!(outcome.appended);
    assert_eq!(
        outcome.on, ".",
        "the subject label is the canonical root spelling"
    );
    assert_eq!(outcome.value.as_deref(), Some("250"));
}

#[test]
fn every_spelling_of_the_repository_root_is_one_subject_and_one_fold() {
    let dir = fixture();
    let root = dir.path();

    let dot = fold(root, "cleanup-pressure@1", ".", 250.0);
    let slash = fold(root, "cleanup-pressure@1", "./", 250.0);
    let absolute = fold(
        root,
        "cleanup-pressure@1",
        &root.canonicalize().unwrap().to_string_lossy(),
        250.0,
    );

    assert_eq!(dot, slash, "`.` and `./` are the same claim");
    assert_eq!(
        dot, absolute,
        "an absolute path equal to the root is the same claim spelled longer"
    );
    assert_eq!(sidecar_lines(root), 1, "one measurement, one row");
}

#[test]
fn a_repository_wide_fold_satisfies_a_bound_that_declares_no_subject() {
    let dir = fixture();
    let root = dir.path();
    fold(root, "cleanup-pressure@1", ".", 250.0);

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "cleanup-pressure-ceiling@1");
    assert_eq!(
        outcome.outcome,
        OutcomeStatus::Failed,
        "250 is past hard 120"
    );
    assert_eq!(outcome.subject, ".");
    assert_eq!(outcome.observed, Some(250.0));
}

#[test]
fn a_repository_wide_fold_satisfies_a_bound_that_declares_subject_dot() {
    let dir = fixture();
    let root = dir.path();
    write(
        root,
        ".claude/epr-meta/measures.yaml",
        &MEASURES.replace(
            "    context: cleanup-pressure\n    hard: 120",
            "    context: cleanup-pressure\n    subject: .\n    hard: 120",
        ),
    );
    fold(root, "cleanup-pressure@1", ".", 45.0);

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "cleanup-pressure-ceiling@1");
    assert_eq!(outcome.outcome, OutcomeStatus::Passed);
    assert_eq!(outcome.observed, Some(45.0));
    assert_eq!(outcome.subject, ".");
}

#[test]
fn a_row_spelling_the_root_differently_still_matches_the_fold() {
    let dir = fixture();
    let root = dir.path();
    write(
        root,
        ".claude/epr-meta/measures.yaml",
        &MEASURES.replace(
            "    context: cleanup-pressure\n    hard: 120",
            "    context: cleanup-pressure\n    subject: ./\n    hard: 120",
        ),
    );
    fold(root, "cleanup-pressure@1", ".", 45.0);

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "cleanup-pressure-ceiling@1");
    assert_eq!(
        outcome.outcome,
        OutcomeStatus::Passed,
        "`./` in a row and `.` in a fold are not two subjects"
    );
    assert_eq!(
        outcome.subject, ".",
        "the outcome reports the normalized spelling"
    );
}

#[test]
fn a_repository_wide_fold_is_not_evidence_for_a_bound_about_a_file() {
    let dir = fixture();
    let root = dir.path();
    write(
        root,
        ".claude/epr-meta/measures.yaml",
        &MEASURES.replace(
            "    context: memory-index-projection\n    soft: 20000",
            "    context: memory-index-projection\n    subject: .claude/memory/MEMORY.md\n    soft: 20000",
        ),
    );
    fold(root, "memory-index-bytes@1", ".", 999.0);

    let payload = report(root, &options(root)).unwrap();
    assert_eq!(
        outcome_for(&payload, "memory-index-bytes-ceiling@1").outcome,
        OutcomeStatus::Skipped,
        "a measurement of the whole tree does not answer a question about one file"
    );
}

#[test]
fn the_root_subject_stays_refused_on_the_prose_arm_so_no_existing_note_moves_address() {
    use elohim_epr_cli::flow::note::note;

    let dir = fixture();
    let root = dir.path();
    let err = note(
        root,
        ".",
        "correction",
        "a prose note still needs a real target",
        None,
        None,
        &NoteActor::default(),
    )
    .expect_err("the root subject is the structured arm's affordance alone");
    assert!(matches!(err, FlowError::UnknownResource(_)), "{err}");
    assert_eq!(sidecar_lines(root), 0);
}

// ── (9) comparator, kit value-parity, and the declared recipe map ───────────────────────────

/// A captured `placement-audit.py --headline` run against this repository on 2026-09-10, verbatim.
///
/// Checked in as a string rather than shelled out to on purpose: the kit is being retired, and a
/// parity test that spawned it would start failing for reasons that have nothing to do with the
/// native reader the moment the script goes. What must be preserved is the ANSWER the kit gave on
/// a known tree, so the native report can be held to it.
const KIT_HEADLINE: &str = "\
MEMORY BUDGET  (`placement-audit.py --ledger` = per-file queue · `--focus` = testable scope)
  debt: 27 no-status · 199 unlinked-memory · 12 claimed-unverified   |   pressure-dirs: empty ✅ · stray: ⚠ 2 (relocate — see scoreboard)
  review: 184/383 specs+plans decomposed · 199 UN-CAPTURED (un-reviewed backlog — `--coverage` for the queue)
  decompose: 8 plans past-due to decompose   (⚠ decompose-self → zero residue)
  memkit: over cap (8.3MB > 8MB cap); retention HELD, custody unverified   [40 cycles]
  mempalace: ⚠ 1 surface file(s) changed since last mine (2026-09-09) — re-mine due (index is behind the front-link)
  cleanup: pressure 250/120  (drift items since last cleanup)   (⚠ cleanup due → /memory-stasis-loop)  [rate/week: unknown — no stamped reset]
  scope: ⚠ 3 to hold (local-conductor,owned-substrate)  →  scope-reconcile.py --apply
  epr-meta: 19/54 regions claimed   (⚠ 35 unclaimed → `placement-audit.py --epr-meta` to remediate)
";

/// The four bridged rows, mirroring `.claude/epr-meta/measures.yaml` as the integration seat
/// declares them — including the `compare:` each carries.
const PARITY_MEASURES: &str = r#"measures-version: 1
measures:
  - id: memkit-report-tier-mb
    version: 1
    unit: megabytes
    default-authority: observation
    status: active
  - id: mempalace-surfaces-changed
    version: 1
    unit: files
    default-authority: observation
    status: active
  - id: cleanup-pressure
    version: 1
    unit: pressure-points
    default-authority: observation
    status: active
  - id: cleanup-pressure-reset
    version: 1
    unit: reset-marks
    default-authority: observation
    status: active
  - id: placement-drift-due
    version: 1
    unit: documents
    default-authority: observation
    status: active
  - id: scope-pending-moves
    version: 1
    unit: count
    default-authority: observation
    status: active

lenses:
  - id: memkit-report-tier-mb-ceiling
    version: 1
    binding: binding-local
    class: inject
    consumes: [memkit-report-tier-mb@1]
    context: session-headline
    soft: 8
    status: active
  - id: mempalace-surfaces-changed-ceiling
    version: 1
    binding: binding-local
    class: inject
    consumes: [mempalace-surfaces-changed@1]
    context: session-headline
    hard: 1
    compare: at-or-above
    status: active
  - id: cleanup-pressure-ceiling
    version: 1
    binding: binding-local
    class: inject
    derive: distinct-subjects-since-reset
    reset: cleanup-pressure-reset@1
    consumes: [cleanup-pressure@1, placement-drift-due@1]
    context: session-headline
    hard: 120
    compare: at-or-above
    status: active
  - id: scope-pending-moves-ceiling
    version: 1
    binding: binding-local
    class: inject
    consumes: [scope-pending-moves@1]
    context: session-headline
    hard: 1
    compare: at-or-above
    status: active
"#;

/// One bridged headline line: how the KIT spells it, and what the native side must agree on.
struct ParityCase {
    slot: &'static str,
    measure: &'static str,
    /// The kit line's prefix.
    kit_line: &'static str,
    /// The number follows this marker on the kit line.
    after: &'static str,
    /// How the kit line signals trouble. The idioms genuinely differ per line (`⚠` on three,
    /// `over cap` on memkit) — that divergence is part of what the native surface replaces with a
    /// single vocabulary, so the parity table records it rather than pretending it away.
    kit_warn_marker: &'static str,
    /// The kit's spelling of the unit, which must appear on its line.
    kit_unit: &'static str,
    /// The unit the measure row declares, which the native outcome must report.
    native_unit: &'static str,
    expected: f64,
    /// When the bound is DERIVED, the measure to accumulate the kit's number on as that many
    /// distinct subjects. The kit's `pressure 250/120` IS a cardinality, so the faithful native
    /// equivalent is 250 drifted things, not one fold carrying the number 250.
    derived_via: Option<&'static str>,
}

const PARITY_CASES: &[ParityCase] = &[
    ParityCase {
        slot: "memkit",
        measure: "memkit-report-tier-mb@1",
        kit_line: "memkit:",
        after: "over cap (",
        kit_warn_marker: "over cap",
        kit_unit: "MB",
        native_unit: "megabytes",
        expected: 8.3,
        derived_via: None,
    },
    ParityCase {
        slot: "mempalace",
        measure: "mempalace-surfaces-changed@1",
        kit_line: "mempalace:",
        after: "mempalace:",
        kit_warn_marker: "⚠",
        kit_unit: "surface file",
        native_unit: "files",
        expected: 1.0,
        derived_via: None,
    },
    ParityCase {
        slot: "cleanup",
        measure: "cleanup-pressure@1",
        kit_line: "cleanup:",
        after: "pressure ",
        kit_warn_marker: "⚠",
        kit_unit: "pressure",
        native_unit: "pressure-points",
        expected: 250.0,
        derived_via: Some("placement-drift-due@1"),
    },
    ParityCase {
        slot: "scope",
        measure: "scope-pending-moves@1",
        kit_line: "scope:",
        after: "scope:",
        kit_warn_marker: "⚠",
        kit_unit: "to hold",
        native_unit: "count",
        expected: 3.0,
        derived_via: None,
    },
];

fn kit_line<'a>(headline: &'a str, prefix: &str) -> &'a str {
    headline
        .lines()
        .find(|l| l.trim_start().starts_with(prefix))
        .unwrap_or_else(|| panic!("the kit fixture has no `{prefix}` line"))
}

/// The first numeric token following `marker`.
fn first_number_after(line: &str, marker: &str) -> f64 {
    let tail = &line[line
        .find(marker)
        .unwrap_or_else(|| panic!("`{marker}` not in `{line}`"))
        + marker.len()..];
    let start = tail
        .find(|c: char| c.is_ascii_digit())
        .unwrap_or_else(|| panic!("no number after `{marker}` in `{line}`"));
    let rest = &tail[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(rest.len());
    rest[..end]
        .trim_end_matches('.')
        .parse()
        .unwrap_or_else(|e| panic!("unparseable number in `{line}`: {e}"))
}

/// Whether the NATIVE outcome is in a warn-or-worse state.
fn native_warns(outcome: &BoundOutcome) -> bool {
    outcome.outcome == OutcomeStatus::Failed || outcome.summary.starts_with("warn:")
}

#[test]
fn the_native_report_agrees_with_the_kit_headline_value_for_value() {
    let dir = fixture();
    let root = dir.path();
    write(root, ".claude/epr-meta/measures.yaml", PARITY_MEASURES);

    // Fold each value the kit reported, parsed from the kit's own text rather than retyped.
    for case in PARITY_CASES {
        let line = kit_line(KIT_HEADLINE, case.kit_line);
        let value = first_number_after(line, case.after);
        assert_eq!(
            value, case.expected,
            "the parity table and the kit fixture disagree on `{}`",
            case.slot
        );
        match case.derived_via {
            // A derived bound accumulates THINGS; folding the kit's total as one number would
            // make the derived count 1 and the slot pass while the kit says it is due.
            Some(source) => {
                for n in 0..(value as usize) {
                    let subject = format!("docs/parity-{}-{n}.md", case.slot);
                    write(root, &subject, "# drifted\n");
                    fold(root, source, &subject, 1.0);
                }
            }
            None => {
                fold(root, case.measure, ".", value);
            }
        }
    }

    let payload = report(root, &options(root)).unwrap();

    for case in PARITY_CASES {
        let line = kit_line(KIT_HEADLINE, case.kit_line);
        let kit_value = first_number_after(line, case.after);
        let kit_warn = line.contains(case.kit_warn_marker);
        let outcome = payload
            .outcomes()
            .iter()
            .find(|o| o.measure == case.measure)
            .unwrap_or_else(|| panic!("no native outcome for `{}`", case.measure));

        // NUMBER
        assert_eq!(
            outcome.observed,
            Some(kit_value),
            "{}: native observed {:?}, kit line says {kit_value} — `{line}`",
            case.slot,
            outcome.observed
        );
        // UNIT — the kit's spelling on its line, the declared spelling on the outcome.
        assert!(
            line.contains(case.kit_unit),
            "{}: the kit line should spell its unit `{}` — `{line}`",
            case.slot,
            case.kit_unit
        );
        assert_eq!(
            outcome.unit.as_deref(),
            Some(case.native_unit),
            "{}: the outcome must report the measure's declared unit",
            case.slot
        );
        // WARN/OK
        assert_eq!(
            native_warns(outcome),
            kit_warn,
            "{}: native says warn={}, kit line says warn={kit_warn} — `{line}` vs `{}`",
            case.slot,
            native_warns(outcome),
            outcome.summary
        );
        // And the rendered headline line carries the marker a session reads.
        let rendered = payload.headline_line(case.slot);
        assert_eq!(
            rendered.contains('⚠'),
            kit_warn,
            "{}: rendered `{rendered}` disagrees with the kit line `{line}`",
            case.slot
        );
    }
}

#[test]
fn the_mempalace_trigger_fires_at_exactly_one_as_the_kit_does() {
    // The pinned inversion. The kit's `mempalace: ⚠ 1 surface file(s) changed` is a warning AT 1.
    // Read with the default `above` comparator, `hard: 1` reported that same 1 as PASSING — the
    // native surface saying "fine" where the kit says "due". Both readings are asserted here so a
    // future edit cannot quietly restore the inversion.
    let dir = fixture();
    let root = dir.path();
    write(root, ".claude/epr-meta/measures.yaml", PARITY_MEASURES);
    fold(root, "mempalace-surfaces-changed@1", ".", 1.0);

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "mempalace-surfaces-changed-ceiling@1");
    assert_eq!(
        outcome.outcome,
        OutcomeStatus::Failed,
        "1 has REACHED the hard watermark 1, which is what `compare: at-or-above` declares"
    );
    assert_eq!(outcome.compare, "at-or-above");
    assert!(
        payload.headline_line("mempalace").contains('⚠'),
        "{}",
        payload.headline_line("mempalace")
    );

    // The same value and watermark under the default comparator is the inversion itself.
    let dir2 = fixture();
    let root2 = dir2.path();
    write(
        root2,
        ".claude/epr-meta/measures.yaml",
        &PARITY_MEASURES.replace(
            "    hard: 1\n    compare: at-or-above\n    status: active\n  - id: cleanup",
            "    hard: 1\n    status: active\n  - id: cleanup",
        ),
    );
    fold(root2, "mempalace-surfaces-changed@1", ".", 1.0);
    let payload2 = report(root2, &options(root2)).unwrap();
    let lenient = outcome_for(&payload2, "mempalace-surfaces-changed-ceiling@1");
    assert_eq!(
        lenient.outcome,
        OutcomeStatus::Passed,
        "without the comparator the row means something else — that is why it must be declared"
    );
    assert_eq!(lenient.compare, "above");
}

#[test]
fn the_comparator_defaults_to_above_and_is_carried_into_the_payload() {
    let dir = fixture();
    let root = dir.path();
    fold(root, "memory-index-bytes@1", SUBJECT, 24_000.0);
    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "memory-index-bytes-ceiling@1");
    assert_eq!(
        outcome.compare, "above",
        "no `compare:` key means today's meaning"
    );
    assert_eq!(outcome.outcome, OutcomeStatus::Passed);

    let json = serde_json::to_value(&payload).unwrap();
    let row = json["recipes"][0]["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["bound"] == "memory-index-bytes-ceiling@1")
        .unwrap();
    assert_eq!(row["compare"], "above");
}

#[test]
fn at_or_above_applies_to_the_soft_watermark_too() {
    let dir = fixture();
    let root = dir.path();
    write(
        root,
        ".claude/epr-meta/measures.yaml",
        &MEASURES.replace(
            "    context: memory-index-projection\n    soft: 20000",
            "    context: memory-index-projection\n    compare: at-or-above\n    soft: 20000",
        ),
    );
    fold(root, "memory-index-bytes@1", SUBJECT, 20_000.0);

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "memory-index-bytes-ceiling@1");
    assert_eq!(outcome.outcome, OutcomeStatus::Passed);
    assert!(
        outcome.summary.starts_with("warn:"),
        "a row that fires AT its hard number would be lying if its soft number needed exceeding; \
         got: {}",
        outcome.summary
    );
    assert!(
        outcome.summary.contains("has reached"),
        "{}",
        outcome.summary
    );
}

#[test]
fn an_unknown_comparator_keeps_the_default_rather_than_taking_the_headline_down() {
    let dir = fixture();
    let root = dir.path();
    write(
        root,
        ".claude/epr-meta/measures.yaml",
        &MEASURES.replace(
            "    context: memory-index-projection\n    soft: 20000",
            "    context: memory-index-projection\n    compare: sideways\n    soft: 20000",
        ),
    );
    fold(root, "memory-index-bytes@1", SUBJECT, 24_000.0);
    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "memory-index-bytes-ceiling@1");
    assert_eq!(outcome.compare, "above");
    assert_eq!(outcome.outcome, OutcomeStatus::Passed);
}

// ── the declared recipe MAP ─────────────────────────────────────────────────────────────────

fn manifest_with(root: &Path, body: &str) {
    write(
        root,
        ".epr-meta/manifest.md",
        &format!("---\nepr-meta-version: 1\nroot: true\n{body}---\n\n# body\n"),
    );
}

#[test]
fn the_scalar_names_a_key_in_the_map_so_the_label_is_the_declared_name() {
    let dir = fixture();
    let root = dir.path();
    strict_recipe(root);
    manifest_with(
        root,
        "policy-recipe: default\npolicy-recipes:\n  \
         default:\n    dir: .claude/epr-meta\n  \
         strict:\n    dir: governance/strict\n",
    );

    let recipes = declared_recipes(root);
    assert_eq!(
        recipes.len(),
        2,
        "every map entry is evaluated, not just the default"
    );
    assert_eq!(
        recipes[0].name, "default",
        "the headline label is the DECLARED name, not the directory basename"
    );
    assert_eq!(recipes[1].name, "strict");
    assert!(recipes[0]
        .measures
        .ends_with(".claude/epr-meta/measures.yaml"));
    assert!(recipes[1]
        .policies
        .ends_with("governance/strict/policies.yaml"));
    assert_eq!(declared_default(root).name, "default");
}

#[test]
fn every_other_map_entry_is_an_alt_without_any_command_line_flag() {
    let dir = fixture();
    let root = dir.path();
    strict_recipe(root);
    manifest_with(
        root,
        "policy-recipe: default\npolicy-recipes:\n  \
         default:\n    dir: .claude/epr-meta\n  \
         strict:\n    dir: governance/strict\n",
    );
    fold(root, "memory-index-bytes@1", SUBJECT, 200.0);

    // No --recipe, no --measures: plurality is the resting state.
    let payload = report(root, &options(root)).unwrap();
    assert_eq!(payload.recipes.len(), 2);
    assert!(payload.recipes[0].primary);
    assert_eq!(payload.recipes[0].recipe.name, "default");
    assert_eq!(payload.recipes[1].recipe.name, "strict");

    let alts = payload.alt_lines();
    assert_eq!(alts.len(), 1);
    assert!(alts[0].starts_with("alt: strict@"), "{}", alts[0]);
    // The primary's lenient reading is what the five slot lines show.
    assert!(
        payload.headline_line("budget").contains("200"),
        "{}",
        payload.headline_line("budget")
    );
}

#[test]
fn an_explicit_measures_or_policies_path_overrides_the_entrys_dir() {
    let dir = fixture();
    let root = dir.path();
    strict_recipe(root);
    manifest_with(
        root,
        "policy-recipe: mixed\npolicy-recipes:\n  \
         mixed:\n    dir: .claude/epr-meta\n    measures: governance/strict/measures.yaml\n",
    );
    let recipes = declared_recipes(root);
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].name, "mixed");
    assert!(
        recipes[0]
            .measures
            .ends_with("governance/strict/measures.yaml"),
        "an explicit half overrides `dir:`"
    );
    assert!(
        recipes[0]
            .policies
            .ends_with(".claude/epr-meta/policies.yaml"),
        "the unspecified half still comes from `dir:`"
    );
}

#[test]
fn a_scalar_still_spelled_as_a_path_adopts_the_matching_named_entry_instead_of_duplicating_it() {
    let dir = fixture();
    let root = dir.path();
    manifest_with(
        root,
        "policy-recipe: .claude/epr-meta\npolicy-recipes:\n  \
         default:\n    dir: .claude/epr-meta\n",
    );
    let recipes = declared_recipes(root);
    assert_eq!(
        recipes.len(),
        1,
        "one recipe written two ways is one recipe, not a lens printed against itself"
    );
    assert_eq!(
        recipes[0].name, "default",
        "the declared name wins over the directory basename"
    );
}

#[test]
fn a_scalar_naming_no_key_and_matching_no_entry_still_reads_as_a_directory() {
    let dir = fixture();
    let root = dir.path();
    strict_recipe(root);
    manifest_with(
        root,
        "policy-recipe: governance/strict\npolicy-recipes:\n  \
         default:\n    dir: .claude/epr-meta\n",
    );
    let recipes = declared_recipes(root);
    assert_eq!(
        recipes.len(),
        2,
        "a manifest typo narrows the reading, never empties it"
    );
    assert_eq!(
        recipes[0].name, "strict",
        "the scalar-as-directory fallback"
    );
    assert_eq!(recipes[1].name, "default");
}

#[test]
fn a_manifest_with_no_map_keeps_the_scalar_as_directory_reading() {
    let dir = fixture();
    let root = dir.path();
    strict_recipe(root);
    manifest_with(root, "policy-recipe: governance/strict\n");
    let recipes = declared_recipes(root);
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].name, "strict");
}

#[test]
fn an_unparseable_manifest_frontmatter_falls_back_rather_than_refusing() {
    let dir = fixture();
    let root = dir.path();
    write(
        root,
        ".epr-meta/manifest.md",
        "---\npolicy-recipes:\n  default:\n   - broken: [unclosed\n---\n# body\n",
    );
    let recipes = declared_recipes(root);
    assert_eq!(recipes.len(), 1);
    assert_eq!(
        recipes[0].name, "claude-epr-meta",
        "a governance document mid-edit must not take the report down"
    );
}

// ── (10) floor-direction comparators ────────────────────────────────────────────────────────

/// A description-floor registry mirroring `agent-description-floor@1` (hard 80) and
/// `skill-description-floor@1` (hard 60) — rows whose number is a MINIMUM.
const FLOOR_MEASURES: &str = r#"measures-version: 1
measures:
  - id: package-description-chars
    version: 1
    unit: characters
    default-authority: observation
    status: active

lenses:
  - id: agent-description-floor
    version: 1
    binding: binding-local
    class: measure
    consumes: [package-description-chars@1]
    context: agent-audit
    hard: 80
    compare: below
    status: active
  - id: skill-description-floor
    version: 1
    binding: binding-local
    class: measure
    consumes: [package-description-chars@1]
    context: skill-audit
    subject: .claude/skills/example/SKILL.md
    hard: 60
    compare: at-or-below
    status: active
"#;

fn floor_fixture() -> TempDir {
    let dir = fixture();
    write(dir.path(), ".claude/epr-meta/measures.yaml", FLOOR_MEASURES);
    // A note's subject must be a readable path — the skill-floor row names one, so it exists here.
    write(
        dir.path(),
        ".claude/skills/example/SKILL.md",
        "---\nname: example\n---\n\n# example\n",
    );
    dir
}

#[test]
fn below_makes_a_floor_row_find_the_short_description_and_clear_the_long_one() {
    // The station-three assertion from the other side: a 40-char description is a finding, 80 is
    // not. Read as a ceiling this row says the exact opposite, which is the inversion `below` ends.
    let dir = floor_fixture();
    let root = dir.path();
    fold(root, "package-description-chars@1", SUBJECT, 40.0);

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "agent-description-floor@1");
    assert_eq!(
        outcome.outcome,
        OutcomeStatus::Failed,
        "fewer than 80 characters IS the finding"
    );
    assert_eq!(outcome.compare, "below", "the direction rides the payload");
    assert!(
        outcome.summary.contains("is under"),
        "a floor summary must not say `past`, which points the wrong way: {}",
        outcome.summary
    );

    // At the watermark exactly: `below` means 80 clears it.
    let dir2 = floor_fixture();
    let root2 = dir2.path();
    fold(root2, "package-description-chars@1", SUBJECT, 80.0);
    let payload2 = report(root2, &options(root2)).unwrap();
    let outcome2 = outcome_for(&payload2, "agent-description-floor@1");
    assert_eq!(outcome2.outcome, OutcomeStatus::Passed);
    assert!(
        outcome2.summary.contains("clears"),
        "a passing floor CLEARS its watermark rather than sitting `within` it: {}",
        outcome2.summary
    );
}

#[test]
fn at_or_below_makes_the_watermark_itself_the_finding() {
    let dir = floor_fixture();
    let root = dir.path();
    fold(
        root,
        "package-description-chars@1",
        ".claude/skills/example/SKILL.md",
        60.0,
    );

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "skill-description-floor@1");
    assert_eq!(
        outcome.outcome,
        OutcomeStatus::Failed,
        "`at-or-below` puts the watermark itself inside the finding"
    );
    assert_eq!(outcome.compare, "at-or-below");
    assert!(
        outcome.summary.contains("has fallen to"),
        "{}",
        outcome.summary
    );

    // One character above it clears.
    let dir2 = floor_fixture();
    let root2 = dir2.path();
    fold(
        root2,
        "package-description-chars@1",
        ".claude/skills/example/SKILL.md",
        61.0,
    );
    let payload2 = report(root2, &options(root2)).unwrap();
    assert_eq!(
        outcome_for(&payload2, "skill-description-floor@1").outcome,
        OutcomeStatus::Passed
    );
}

#[test]
fn a_floor_read_as_a_ceiling_is_the_inversion_the_direction_exists_to_end() {
    // The same row and the same value with the comparator removed: a 40-character description
    // PASSES an 80-character floor, and a thorough 200-character one FAILS it. Pinned so a future
    // edit cannot quietly restore the ceiling reading.
    let dir = fixture();
    let root = dir.path();
    write(
        root,
        ".claude/epr-meta/measures.yaml",
        &FLOOR_MEASURES.replace("    hard: 80\n    compare: below", "    hard: 80"),
    );
    fold(root, "package-description-chars@1", SUBJECT, 40.0);
    assert_eq!(
        outcome_for(
            &report(root, &options(root)).unwrap(),
            "agent-description-floor@1"
        )
        .outcome,
        OutcomeStatus::Passed,
        "without `compare: below` the floor says a 40-char description is fine"
    );

    let dir2 = fixture();
    let root2 = dir2.path();
    write(
        root2,
        ".claude/epr-meta/measures.yaml",
        &FLOOR_MEASURES.replace("    hard: 80\n    compare: below", "    hard: 80"),
    );
    fold(root2, "package-description-chars@1", SUBJECT, 200.0);
    assert_eq!(
        outcome_for(
            &report(root2, &options(root2)).unwrap(),
            "agent-description-floor@1"
        )
        .outcome,
        OutcomeStatus::Failed,
        "and that a thorough 200-char description is the problem — exactly backwards"
    );
}

#[test]
fn a_floor_soft_watermark_warns_before_the_hard_one_finds() {
    // Direction-agnostic ordering: hard is checked first, so a floor declares hard BELOW soft
    // (fall under 80 to warn, under 60 to fail) and the arms still fire in the right order.
    let dir = fixture();
    let root = dir.path();
    write(
        root,
        ".claude/epr-meta/measures.yaml",
        &FLOOR_MEASURES.replace(
            "    context: agent-audit\n    hard: 80\n    compare: below",
            "    context: agent-audit\n    soft: 80\n    hard: 60\n    compare: below",
        ),
    );
    fold(root, "package-description-chars@1", SUBJECT, 70.0);

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "agent-description-floor@1");
    assert_eq!(
        outcome.outcome,
        OutcomeStatus::Passed,
        "70 is under the soft floor 80 but clears the hard floor 60"
    );
    assert!(outcome.summary.starts_with("warn:"), "{}", outcome.summary);
    assert!(outcome.summary.contains("is under"), "{}", outcome.summary);
}

#[test]
fn every_declared_comparator_spelling_round_trips_into_the_payload() {
    for (declared, expected) in [
        ("above", "above"),
        ("at-or-above", "at-or-above"),
        ("below", "below"),
        ("at-or-below", "at-or-below"),
        // Aliases: the same four directions, spelled the way a row author might reach for.
        ("gt", "above"),
        ("gte", "at-or-above"),
        ("lt", "below"),
        ("lte", "at-or-below"),
    ] {
        let dir = fixture();
        let root = dir.path();
        write(
            root,
            ".claude/epr-meta/measures.yaml",
            &MEASURES.replace(
                "    context: memory-index-projection\n    soft: 20000",
                &format!(
                    "    context: memory-index-projection\n    compare: {declared}\n    soft: 20000"
                ),
            ),
        );
        fold(root, "memory-index-bytes@1", SUBJECT, 1.0);
        let payload = report(root, &options(root)).unwrap();
        assert_eq!(
            outcome_for(&payload, "memory-index-bytes-ceiling@1").compare,
            expected,
            "`compare: {declared}` must reach the payload as `{expected}`"
        );
    }
}

// ── station two: the scope slot is a DERIVATION, not a watermark ────────────────────────────────

/// The substrate manifest a scope derivation needs, with one capability down.
const SCOPE_CLUSTER_STATE: &str = "updated: 2026-09-10\nresources:\n  household-nodes:\n    available: true\n  shem:\n    available: false\n";

#[test]
fn the_scope_slot_prints_the_derivation_when_a_substrate_manifest_exists() {
    // Station two makes `epr flow report scope` the owner of this line. Scope is the one headline
    // slot whose fact is not a magnitude: "1 to hold (shem)" names a direction and a capability,
    // and a watermark over a count can say neither — while the root CLAUDE.md declares those exact
    // spellings as session triggers.
    let dir = fixture();
    let root = dir.path();
    write(
        root,
        "genesis/manifests/cluster-state.yaml",
        SCOPE_CLUSTER_STATE,
    );
    write(
        root,
        "genesis/docs/superpowers/plans/blocked.md",
        "---\ntitle: b\nrequires_env: [shem]\n---\n\n- [ ] one\n",
    );
    let payload = report(root, &options(root)).unwrap();
    let line = payload.headline_line("scope");
    assert_eq!(
        line, "scope: ⚠ 1 to hold (shem)  →  epr flow hold --scope --apply",
        "the scope slot must carry the derivation's counts, capability names and next action"
    );
    assert_eq!(payload.scope_pending_moves, Some(1));
}

#[test]
fn an_aligned_plate_renders_the_trigger_word_the_gospel_declares() {
    let dir = fixture();
    let root = dir.path();
    write(
        root,
        "genesis/manifests/cluster-state.yaml",
        SCOPE_CLUSTER_STATE,
    );
    let payload = report(root, &options(root)).unwrap();
    assert_eq!(
        payload.headline_line("scope"),
        "scope: aligned ✅  (plate matches substrate)"
    );
    assert_eq!(payload.scope_pending_moves, Some(0));
}

#[test]
fn a_tree_with_no_substrate_manifest_was_not_asked_rather_than_found_aligned() {
    // The honest absence. `fixture()` writes no cluster-state, so there is nothing to reconcile
    // against — and reporting "aligned ✅" there would be a false all-clear about a question nobody
    // put. The slot falls back to the BOUND plane, which in this fixture declares `scope-drift@1`
    // with no fold behind it — so the line names the missing measure rather than inventing a state.
    let dir = fixture();
    let payload = report(dir.path(), &options(dir.path())).unwrap();
    assert_eq!(payload.scope_line, None);
    assert_eq!(payload.scope_pending_moves, None);
    assert_eq!(
        payload.headline_line("scope"),
        "scope: skipped (no fold for scope-drift@1)"
    );
}

// ── (11) derived accumulations ──────────────────────────────────────────────────────────────

/// A cleanup-pressure lens derived over two drift measures, mirroring the live row's shape.
const DERIVE_MEASURES: &str = r#"measures-version: 1
measures:
  - id: cleanup-pressure
    version: 1
    unit: pressure-points
    default-authority: observation
    status: active
  - id: cleanup-pressure-reset
    version: 1
    unit: reset-marks
    default-authority: observation
    status: active
  - id: placement-drift-due
    version: 1
    unit: documents
    default-authority: observation
    status: active
  - id: map-currency-drift
    version: 1
    unit: seeds
    default-authority: observation
    status: active
  - id: map-currency-drift-reset
    version: 1
    unit: reset-marks
    default-authority: observation
    status: active

lenses:
  - id: cleanup-pressure-ceiling
    version: 1
    headline: cleanup
    compare: at-or-above
    binding: binding-local
    class: inject
    derive: distinct-subjects-since-reset
    reset: cleanup-pressure-reset@1
    consumes:
      - cleanup-pressure@1
      - placement-drift-due@1
      - map-currency-drift@1
    context: session-headline
    hard: 120
    status: active
  - id: map-currency-drift-ceiling
    version: 1
    binding: binding-local
    class: inject
    derive: distinct-subjects-since-reset
    reset: map-currency-drift-reset@1
    consumes: [map-currency-drift@1]
    context: session-headline
    hard: 1
    compare: at-or-above
    status: active
  - id: drift-event-count
    version: 1
    binding: observation
    class: measure
    derive: count-since-reset
    reset: cleanup-pressure-reset@1
    consumes: [placement-drift-due@1]
    context: probe
    hard: 1000
    status: active
"#;

fn derive_fixture() -> TempDir {
    let dir = fixture();
    write(
        dir.path(),
        ".claude/epr-meta/measures.yaml",
        DERIVE_MEASURES,
    );
    dir
}

/// Fold one drift observation about `subject`, creating the file so the note can address it.
fn drift(root: &Path, measure: &str, subject: &str) {
    write(root, subject, &format!("# {subject}\n"));
    fold(root, measure, subject, 1.0);
}

fn reset(root: &Path) {
    fold(root, "cleanup-pressure-reset@1", ".", 1.0);
}

#[test]
fn the_derived_count_equals_the_kits_activity_arithmetic_across_two_measures() {
    // `cleanup-pressure.py::activity()` sums len() over each accumulator file in turn, so the
    // cardinality is DISTINCT-PER-MEASURE THEN SUMMED. Three documents in placement-drift plus two
    // seeds in map-currency-drift is 5 — the number the kit's `--status` line would print.
    let dir = derive_fixture();
    let root = dir.path();
    for doc in ["docs/a.md", "docs/b.md", "docs/c.md"] {
        drift(root, "placement-drift-due@1", doc);
    }
    for seed in ["arch/one.md", "arch/two.md"] {
        drift(root, "map-currency-drift@1", seed);
    }

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "cleanup-pressure-ceiling@1");
    assert_eq!(
        outcome.observed,
        Some(5.0),
        "3 documents + 2 seeds = 5 distinct drifted items, summed per accumulator"
    );
    assert_eq!(
        outcome.derive.as_deref(),
        Some("distinct-subjects-since-reset")
    );
    assert_eq!(outcome.contributing_folds, Some(5));
    assert_eq!(
        outcome.outcome,
        OutcomeStatus::Passed,
        "5 is well under 120"
    );
}

#[test]
fn a_subject_drifting_twice_in_one_measure_counts_once() {
    let dir = derive_fixture();
    let root = dir.path();
    drift(root, "placement-drift-due@1", "docs/a.md");
    // The same document seen again by the hook: a second fold, still one drifted item.
    fold(root, "placement-drift-due@1", "docs/a.md", 2.0);
    drift(root, "placement-drift-due@1", "docs/b.md");

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "cleanup-pressure-ceiling@1");
    assert_eq!(
        outcome.observed,
        Some(2.0),
        "two distinct documents, three folds"
    );
    assert_eq!(
        outcome.contributing_folds,
        Some(3),
        "the fold count and the distinct count are different numbers and both are reported"
    );
}

#[test]
fn the_same_subject_in_two_measures_counts_twice_as_the_kit_counts_it() {
    // Deliberately NOT global-distinct: the kit sums per accumulator, so one path drifting in two
    // of them is two items. Matching a tidier rule would report a different number than the gate
    // this replaces.
    let dir = derive_fixture();
    let root = dir.path();
    drift(root, "placement-drift-due@1", "docs/shared.md");
    fold(root, "map-currency-drift@1", "docs/shared.md", 1.0);

    let payload = report(root, &options(root)).unwrap();
    assert_eq!(
        outcome_for(&payload, "cleanup-pressure-ceiling@1").observed,
        Some(2.0)
    );
}

#[test]
fn a_reset_observation_zeroes_the_accumulation() {
    let dir = derive_fixture();
    let root = dir.path();
    for doc in ["docs/a.md", "docs/b.md", "docs/c.md"] {
        drift(root, "placement-drift-due@1", doc);
    }
    assert_eq!(
        outcome_for(
            &report(root, &options(root)).unwrap(),
            "cleanup-pressure-ceiling@1"
        )
        .observed,
        Some(3.0)
    );

    reset(root);

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "cleanup-pressure-ceiling@1");
    assert_eq!(
        outcome.observed,
        Some(0.0),
        "a reset drains the accumulator; the folds before it are history, not pressure"
    );
    assert_eq!(outcome.contributing_folds, Some(0));
    assert!(
        outcome.reset_fold_cid.is_some(),
        "the reset that witnessed the zero is named"
    );
    assert_eq!(
        outcome.reset_measure.as_deref(),
        Some("cleanup-pressure-reset@1"),
        "the payload names the reset a person would append"
    );
}

#[test]
fn drift_after_a_reset_accumulates_again_from_zero() {
    let dir = derive_fixture();
    let root = dir.path();
    drift(root, "placement-drift-due@1", "docs/old.md");
    reset(root);
    drift(root, "placement-drift-due@1", "docs/new-one.md");
    drift(root, "placement-drift-due@1", "docs/new-two.md");

    let outcome_payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&outcome_payload, "cleanup-pressure-ceiling@1");
    assert_eq!(
        outcome.observed,
        Some(2.0),
        "only the folds appended after the latest reset count"
    );
}

#[test]
fn the_latest_reset_wins_when_several_have_been_appended() {
    let dir = derive_fixture();
    let root = dir.path();
    drift(root, "placement-drift-due@1", "docs/a.md");
    reset(root);
    drift(root, "placement-drift-due@1", "docs/b.md");
    // A second drain, appended later — an identical reset note would dedupe to one CID, so this
    // one differs by its reason, the way two real drains a week apart would.
    observe(
        root,
        "observation",
        "cleanup-pressure-reset@1",
        ".",
        1.0,
        None,
        &BTreeMap::new(),
        Some("second cleanup cycle"),
        &NoteActor::default(),
        &measures_path(root),
    )
    .unwrap();
    drift(root, "placement-drift-due@1", "docs/c.md");

    let payload = report(root, &options(root)).unwrap();
    assert_eq!(
        outcome_for(&payload, "cleanup-pressure-ceiling@1").observed,
        Some(1.0),
        "counting resumes from the NEWEST reset, not the first"
    );
}

#[test]
fn a_derived_bound_with_no_reset_and_no_folds_is_skipped_not_zero() {
    let dir = derive_fixture();
    let root = dir.path();
    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "cleanup-pressure-ceiling@1");
    assert_eq!(
        outcome.outcome,
        OutcomeStatus::Skipped,
        "nobody has measured or drained anything — a zero here would be the false reassurance again"
    );
    assert!(outcome.observed.is_none());
    assert!(outcome.summary.contains("no reset"), "{}", outcome.summary);
    assert!(
        payload
            .headline_line("cleanup")
            .starts_with("cleanup: skipped"),
        "{}",
        payload.headline_line("cleanup")
    );
}

#[test]
fn a_reset_with_nothing_since_it_is_a_witnessed_zero_rather_than_a_skip() {
    let dir = derive_fixture();
    let root = dir.path();
    reset(root);
    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "cleanup-pressure-ceiling@1");
    assert_eq!(
        outcome.outcome,
        OutcomeStatus::Passed,
        "someone drained it and that act is on the record — this zero is evidence"
    );
    assert_eq!(outcome.observed, Some(0.0));
}

#[test]
fn count_since_reset_counts_folds_where_distinct_subjects_counts_subjects() {
    let dir = derive_fixture();
    let root = dir.path();
    drift(root, "placement-drift-due@1", "docs/a.md");
    fold(root, "placement-drift-due@1", "docs/a.md", 2.0);
    drift(root, "placement-drift-due@1", "docs/b.md");

    let payload = report(root, &options(root)).unwrap();
    assert_eq!(
        outcome_for(&payload, "drift-event-count@1").observed,
        Some(3.0),
        "`count-since-reset` counts EVENTS"
    );
    assert_eq!(
        outcome_for(&payload, "drift-event-count@1")
            .derive
            .as_deref(),
        Some("count-since-reset")
    );
    assert_eq!(
        outcome_for(&payload, "cleanup-pressure-ceiling@1").observed,
        Some(2.0),
        "`distinct-subjects-since-reset` counts THINGS"
    );
}

#[test]
fn a_derived_bound_crosses_its_watermark_on_the_derived_value() {
    let dir = derive_fixture();
    let root = dir.path();
    write(
        root,
        ".claude/epr-meta/measures.yaml",
        &DERIVE_MEASURES.replace("    hard: 120\n", "    hard: 2\n"),
    );
    for doc in ["docs/a.md", "docs/b.md"] {
        drift(root, "placement-drift-due@1", doc);
    }

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "cleanup-pressure-ceiling@1");
    assert_eq!(
        outcome.outcome,
        OutcomeStatus::Failed,
        "2 has reached the hard 2"
    );
    assert!(
        outcome.summary.contains("since the beginning"),
        "an underived reading and a count-since-reset must not read the same: {}",
        outcome.summary
    );
    assert!(payload.headline_line("cleanup").contains('⚠'));
}

#[test]
fn the_reset_measure_defaults_to_the_primary_measures_reset_spelling() {
    let dir = derive_fixture();
    let root = dir.path();
    write(
        root,
        ".claude/epr-meta/measures.yaml",
        &DERIVE_MEASURES.replace("    reset: cleanup-pressure-reset@1\n", ""),
    );
    reset(root);
    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "cleanup-pressure-ceiling@1");
    assert_eq!(
        outcome.reset_measure.as_deref(),
        Some("cleanup-pressure-reset@1"),
        "an undeclared reset is `<primary-measure-id>-reset@1` — the command a person types"
    );
    assert_eq!(
        outcome.observed,
        Some(0.0),
        "and the default reset is honoured"
    );
}

#[test]
fn a_plain_bound_still_admits_only_its_primary_measure() {
    // The multi-`consumes:` reading is the DERIVED arm's affordance. A plain row that happens to
    // list several keeps the meaning it had before `derive:` existed.
    let dir = derive_fixture();
    let root = dir.path();
    write(
        root,
        ".claude/epr-meta/measures.yaml",
        &DERIVE_MEASURES.replace("    derive: distinct-subjects-since-reset\n", ""),
    );
    drift(root, "placement-drift-due@1", "docs/a.md");

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "cleanup-pressure-ceiling@1");
    assert_eq!(
        outcome.outcome,
        OutcomeStatus::Skipped,
        "without `derive:` a placement-drift fold is not evidence for a cleanup-pressure bound"
    );
    assert!(outcome.derive.is_none());
}

#[test]
fn the_json_payload_carries_the_derivation_and_its_reset() {
    let dir = derive_fixture();
    let root = dir.path();
    drift(root, "placement-drift-due@1", "docs/a.md");
    reset(root);
    drift(root, "placement-drift-due@1", "docs/b.md");

    let payload = report(root, &options(root)).unwrap();
    let json = serde_json::to_value(&payload).unwrap();
    let row = json["recipes"][0]["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["bound"] == "cleanup-pressure-ceiling@1")
        .unwrap();
    assert_eq!(row["derive"], "distinct-subjects-since-reset");
    assert_eq!(row["resetMeasure"], "cleanup-pressure-reset@1");
    assert!(row["resetFoldCid"].is_string());
    assert_eq!(row["contributingFolds"], 1);
    assert_eq!(row["observed"], 1.0);
}

#[test]
fn an_ordinary_bound_omits_every_derivation_key_from_its_payload() {
    let dir = fixture();
    let root = dir.path();
    fold(root, "memory-index-bytes@1", SUBJECT, 100.0);
    let payload = report(root, &options(root)).unwrap();
    let json = serde_json::to_value(&payload).unwrap();
    let row = json["recipes"][0]["outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["bound"] == "memory-index-bytes-ceiling@1")
        .unwrap();
    for key in [
        "derive",
        "resetMeasure",
        "resetFoldCid",
        "contributingFolds",
    ] {
        assert!(
            row.get(key).is_none(),
            "a pre-derive payload must stay byte-identical; `{key}` leaked"
        );
    }
}

#[test]
fn a_derived_bound_is_denominated_in_its_own_measures_unit_not_a_sources() {
    // The accumulation spans measures with different units (`documents`, `seeds`). Reporting
    // whichever source sorted first would label a pressure score `documents`.
    let dir = derive_fixture();
    let root = dir.path();
    drift(root, "placement-drift-due@1", "docs/a.md");
    drift(root, "map-currency-drift@1", "arch/one.md");

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "cleanup-pressure-ceiling@1");
    assert_eq!(
        outcome.unit.as_deref(),
        Some("pressure-points"),
        "the lens is denominated in cleanup-pressure@1's unit, not placement-drift-due@1's"
    );
    assert!(
        payload.headline_line("cleanup").contains("pressure-points"),
        "{}",
        payload.headline_line("cleanup")
    );
}

#[test]
fn a_plain_bound_still_reports_the_unit_its_fold_carried() {
    let dir = fixture();
    let root = dir.path();
    fold(root, "memory-index-bytes@1", SUBJECT, 100.0);
    assert_eq!(
        outcome_for(
            &report(root, &options(root)).unwrap(),
            "memory-index-bytes-ceiling@1"
        )
        .unit
        .as_deref(),
        Some("bytes")
    );
}

// ── (12) self-heal: a derived count must be able to FALL ─────────────────────────────────────

/// The producers' self-heal signal: value 0 on a subject that is no longer drifted.
fn healed(root: &Path, measure: &str, subject: &str) {
    fold(root, measure, subject, 0.0);
}

#[test]
fn a_reopened_document_is_retired_from_the_count() {
    // `placement-drift-signal.py:167-170` pops a re-opened doc out of the accumulator and folds
    // value 0 for it. Reading only the subject and never the value made the bound MONOTONE — it
    // could rise and never fall, so the `cleanup:` trigger became a false alarm that never cleared.
    let dir = derive_fixture();
    let root = dir.path();
    for doc in ["docs/a.md", "docs/b.md", "docs/c.md"] {
        drift(root, "placement-drift-due@1", doc);
    }
    assert_eq!(
        outcome_for(
            &report(root, &options(root)).unwrap(),
            "cleanup-pressure-ceiling@1"
        )
        .observed,
        Some(3.0)
    );

    healed(root, "placement-drift-due@1", "docs/b.md");

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "cleanup-pressure-ceiling@1");
    assert_eq!(
        outcome.observed,
        Some(2.0),
        "the re-opened doc is no longer outstanding, so the count falls by one"
    );
    assert_eq!(
        outcome.contributing_folds,
        Some(4),
        "the heal is still evidence — the fold count rises while the outstanding count falls"
    );
}

#[test]
fn a_subject_that_heals_then_drifts_again_counts_once() {
    let dir = derive_fixture();
    let root = dir.path();
    // The producer carries the doc's status in env (`placement-drift-signal.py:184`), so a
    // re-drift under a different status is a distinguishable act with its own address.
    let landed = BTreeMap::from([("status".to_string(), "landed".to_string())]);
    let superseded = BTreeMap::from([("status".to_string(), "superseded".to_string())]);
    write(root, "docs/a.md", "# a\n");
    fold_env(root, "placement-drift-due@1", "docs/a.md", 1.0, &landed);
    healed(root, "placement-drift-due@1", "docs/a.md");
    fold_env(root, "placement-drift-due@1", "docs/a.md", 1.0, &superseded);

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "cleanup-pressure-ceiling@1");
    assert_eq!(
        outcome.observed,
        Some(1.0),
        "the LATEST fold per subject decides; a subject re-enters once, not twice"
    );
    assert_eq!(outcome.contributing_folds, Some(3));
}

#[test]
fn a_byte_identical_re_drift_is_the_first_fold_and_cannot_resurrect_a_healed_subject() {
    // A real property of the plane, pinned rather than hidden: identity IS content, so a re-drift
    // that differs from the original in nothing at all — same measure, subject, value, unit, env,
    // and the same HEAD-derived date — dedupes to the ORIGINAL fold's address and appends nothing.
    // The heal therefore stays the latest fold and the subject stays retired.
    //
    // In practice the producers distinguish these: `placement-drift-signal.py:184` carries the
    // doc's status in env, and a re-drift in a later commit is dated by a later HEAD. A caller that
    // needs an indistinguishable re-drift to count must give it something to differ by.
    let dir = derive_fixture();
    let root = dir.path();
    drift(root, "placement-drift-due@1", "docs/a.md");
    healed(root, "placement-drift-due@1", "docs/a.md");
    let before = sidecar_lines(root);
    fold(root, "placement-drift-due@1", "docs/a.md", 1.0);
    assert_eq!(
        sidecar_lines(root),
        before,
        "the re-drift is byte-identical to the first fold, so it is a no-op"
    );

    let payload = report(root, &options(root)).unwrap();
    assert_eq!(
        outcome_for(&payload, "cleanup-pressure-ceiling@1").observed,
        Some(0.0),
        "nothing was appended, so the heal is still the latest word on this subject"
    );
}

#[test]
fn every_subject_healing_is_a_witnessed_zero_without_any_reset() {
    let dir = derive_fixture();
    let root = dir.path();
    drift(root, "placement-drift-due@1", "docs/a.md");
    drift(root, "placement-drift-due@1", "docs/b.md");
    healed(root, "placement-drift-due@1", "docs/a.md");
    healed(root, "placement-drift-due@1", "docs/b.md");

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "cleanup-pressure-ceiling@1");
    assert_eq!(
        outcome.outcome,
        OutcomeStatus::Passed,
        "the heals ARE the evidence — this zero is witnessed, not unmeasured"
    );
    assert_eq!(outcome.observed, Some(0.0));
    assert!(
        outcome.reset_fold_cid.is_none(),
        "no reset was needed to reach it"
    );
}

#[test]
fn a_map_refresh_zeroes_the_whole_accumulator_through_the_reset_measure() {
    // `map-drift-signal.py:143-147` empties `store["changed"]` ENTIRELY on a MAP.md refresh, so a
    // per-subject zero on MAP.md would leave every other accumulated seed counted. `_observation.py`
    // routes that path to `map-currency-drift-reset@1` on subject `.`; this is the reader half.
    let dir = derive_fixture();
    let root = dir.path();
    for seed in ["arch/one.md", "arch/two.md", "arch/three.md"] {
        drift(root, "map-currency-drift@1", seed);
    }
    let before = report(root, &options(root)).unwrap();
    let before_outcome = outcome_for(&before, "map-currency-drift-ceiling@1");
    assert_eq!(before_outcome.observed, Some(3.0));
    assert_eq!(
        before_outcome.outcome,
        OutcomeStatus::Failed,
        "3 seeds have reached the hard watermark 1"
    );

    fold(root, "map-currency-drift-reset@1", ".", 1.0);

    let after = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&after, "map-currency-drift-ceiling@1");
    assert_eq!(
        outcome.observed,
        Some(0.0),
        "one reset clears every accumulated seed, the way emptying the collection does"
    );
    assert_eq!(outcome.outcome, OutcomeStatus::Passed);
    assert!(outcome.reset_fold_cid.is_some());
}

#[test]
fn a_per_subject_zero_does_not_clear_the_other_subjects() {
    // The other half of the same distinction: a per-subject heal retires ONE subject. If this ever
    // cleared the collection it would be the bulk-clear bug wearing the opposite sign.
    let dir = derive_fixture();
    let root = dir.path();
    for seed in ["arch/one.md", "arch/two.md", "arch/three.md"] {
        drift(root, "map-currency-drift@1", seed);
    }
    healed(root, "map-currency-drift@1", "arch/one.md");

    let payload = report(root, &options(root)).unwrap();
    assert_eq!(
        outcome_for(&payload, "map-currency-drift-ceiling@1").observed,
        Some(2.0),
        "one seed retired, two still outstanding"
    );
}

#[test]
fn count_since_reset_still_counts_a_heal_as_an_event() {
    // The two derives part company here by design: `count-since-reset` measures how much HAPPENED,
    // `distinct-subjects-since-reset` measures how much is still OUTSTANDING.
    let dir = derive_fixture();
    let root = dir.path();
    drift(root, "placement-drift-due@1", "docs/a.md");
    healed(root, "placement-drift-due@1", "docs/a.md");

    let payload = report(root, &options(root)).unwrap();
    assert_eq!(
        outcome_for(&payload, "drift-event-count@1").observed,
        Some(2.0),
        "two events happened"
    );
    assert_eq!(
        outcome_for(&payload, "cleanup-pressure-ceiling@1").observed,
        Some(0.0),
        "nothing is outstanding"
    );
}

#[test]
fn the_derived_cleanup_slot_can_fail_when_it_drifts() {
    // Guards the parity fixture's shape: a PLAIN cleanup row folded with the kit's total would make
    // the derived count 1 and the slot pass while the kit says it is due.
    let dir = fixture();
    let root = dir.path();
    write(root, ".claude/epr-meta/measures.yaml", PARITY_MEASURES);
    for n in 0..121 {
        let subject = format!("docs/drift-{n}.md");
        write(root, &subject, "# drifted\n");
        fold(root, "placement-drift-due@1", &subject, 1.0);
    }
    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "cleanup-pressure-ceiling@1");
    assert_eq!(outcome.observed, Some(121.0));
    assert_eq!(outcome.outcome, OutcomeStatus::Failed);
    assert!(payload.headline_line("cleanup").contains('⚠'));
}

// ── (13) rate over a rolling window — the recall habit reads a quarter of journeys, not one ──
//
// `derive: rate-over-window` is a different shape than the reset-accumulations above: there is no
// reset, and the population is every admissible fold whose `occurred_at` falls in the last
// `window_days`, drawn from every measure the row `consumes:` (mirroring
// `recall-journey-window-ceiling@1`'s two middot, `recall-mistaken-assertions@1` and
// `recall-unmetered-bytes@1`). The observed value is the fraction of that windowed population
// whose fold value is greater than zero — "how many of the last quarter's journeys were NOT
// clean" — and `skipped` (never a zero) is the answer when fewer than three folds fall in the
// window, since a rate over one or two journeys is noise wearing a percentage.

/// Mirrors the shape of the live `recall-journey-window-ceiling@1` row, at a `window_days` small
/// enough for a test to control by hand. No `compare:` is declared (default `above`), so a
/// windowed rate exactly AT the hard watermark reads as `within` rather than `failed` — the
/// boundary the first fixture below exercises.
const RATE_MEASURES: &str = r#"measures-version: 1
measures:
  - id: recall-mistaken-assertions
    version: 1
    unit: count
    default-authority: observation
    status: active
  - id: recall-unmetered-bytes
    version: 1
    unit: bytes
    default-authority: observation
    status: active

lenses:
  - id: recall-journey-window-ceiling
    version: 1
    binding: binding-local
    class: inject
    derive: rate-over-window
    consumes:
      - recall-mistaken-assertions@1
      - recall-unmetered-bytes@1
    window-days: 10
    context: recall-journey
    hard: 0.2
    status: active
"#;

fn rate_fixture() -> TempDir {
    let dir = fixture();
    write(dir.path(), ".claude/epr-meta/measures.yaml", RATE_MEASURES);
    dir
}

fn git_at(root: &Path, when: &str, args: &[&str]) {
    let out = elohim_epr_cli::process::build_command("git", args, root, &[])
        .env("GIT_AUTHOR_NAME", "Fixture Author")
        .env("GIT_AUTHOR_EMAIL", "author@example.test")
        .env("GIT_COMMITTER_NAME", "Fixture Author")
        .env("GIT_COMMITTER_EMAIL", "author@example.test")
        .env("GIT_AUTHOR_DATE", when)
        .env("GIT_COMMITTER_DATE", when)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?} failed");
}

/// A fold is dated by the git HEAD it is authored against (see the module doc), so giving a fold
/// a date in the past means advancing HEAD to a commit dated in the past FIRST. One throwaway
/// marker file per call keeps each commit non-empty and distinct.
static MARKER_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn fold_at(root: &Path, when: &str, measure: &str, subject: &str, value: f64) -> String {
    let n = MARKER_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    write(root, &format!(".markers/m{n}"), when);
    git_at(root, when, &["add", "-A"]);
    git_at(root, when, &["commit", "-qm", &format!("marker {n}")]);
    fold(root, measure, subject, value)
}

/// An RFC 3339 timestamp `days` ago from the moment the test runs — the same clock
/// `evaluate_rate_over_window` reads `now` from, so the fixture and the derive agree on "recent".
fn days_ago(days: i64) -> String {
    (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339()
}

#[test]
fn rate_over_window_reads_the_fraction_of_recent_folds_with_any_positive_value() {
    let dir = rate_fixture();
    let root = dir.path();
    // 5 folds inside the 10-day window, 1 of them positive: 1/5 = 0.20, exactly the hard
    // watermark — and `above` (the default compare) reads "at" as still within, not crossed.
    fold_at(root, &days_ago(1), "recall-mistaken-assertions@1", ".", 0.0);
    fold_at(root, &days_ago(3), "recall-unmetered-bytes@1", ".", 0.0);
    fold_at(root, &days_ago(5), "recall-mistaken-assertions@1", ".", 0.0);
    fold_at(root, &days_ago(7), "recall-unmetered-bytes@1", ".", 0.0);
    fold_at(root, &days_ago(9), "recall-mistaken-assertions@1", ".", 1.0);

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "recall-journey-window-ceiling@1");
    assert_eq!(
        outcome.observed,
        Some(0.2),
        "1 of the 5 windowed folds is positive"
    );
    assert_eq!(outcome.contributing_folds, Some(5));
    assert_eq!(outcome.derive.as_deref(), Some("rate-over-window"));
    assert_eq!(
        outcome.outcome,
        OutcomeStatus::Passed,
        "0.20 has not gone PAST the hard watermark 0.2 under the default `above` comparator"
    );
    assert!(
        outcome.summary.contains("within hard 0.2"),
        "{}",
        outcome.summary
    );
}

#[test]
fn fewer_than_three_folds_in_the_window_is_skipped_not_a_rate() {
    let dir = rate_fixture();
    let root = dir.path();
    fold_at(root, &days_ago(1), "recall-mistaken-assertions@1", ".", 0.0);
    fold_at(root, &days_ago(2), "recall-unmetered-bytes@1", ".", 1.0);

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "recall-journey-window-ceiling@1");
    assert_eq!(
        outcome.outcome,
        OutcomeStatus::Skipped,
        "two journeys is not a rate — a percentage over that few is noise wearing a number"
    );
    assert!(outcome.observed.is_none());
    assert!(
        outcome.summary.contains("fewer than 3 journeys in window"),
        "{}",
        outcome.summary
    );
}

#[test]
fn a_fold_older_than_the_window_does_not_count_toward_the_rate() {
    let dir = rate_fixture();
    let root = dir.path();
    // Two folds well outside the 10-day window (they would make the rate 2/5 = 0.4 and FAIL if
    // wrongly admitted) plus three inside it, none positive.
    fold_at(
        root,
        &days_ago(30),
        "recall-mistaken-assertions@1",
        ".",
        1.0,
    );
    fold_at(root, &days_ago(45), "recall-unmetered-bytes@1", ".", 1.0);
    fold_at(root, &days_ago(1), "recall-mistaken-assertions@1", ".", 0.0);
    fold_at(root, &days_ago(2), "recall-unmetered-bytes@1", ".", 0.0);
    fold_at(root, &days_ago(3), "recall-mistaken-assertions@1", ".", 0.0);

    let payload = report(root, &options(root)).unwrap();
    let outcome = outcome_for(&payload, "recall-journey-window-ceiling@1");
    assert_eq!(
        outcome.contributing_folds,
        Some(3),
        "the two 30+ day old folds are outside the 10-day window"
    );
    assert_eq!(outcome.observed, Some(0.0));
    assert_eq!(outcome.outcome, OutcomeStatus::Passed);
}
