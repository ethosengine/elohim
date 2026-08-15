//! Integration tests over a synthetic tempdir repo for `epr flow stocks` — the read-only
//! equilibrium fold (Spec B,
//! `genesis/docs/superpowers/specs/2026-08-13-dev-system-equilibrium-stocks-design.md`;
//! harness-borrows plan Task 2 Step 4).
//!
//! Fixture shape is `flow_note.rs`'s: a recipe with a `scenario` stage, `project`ed once so real
//! scenario `Commitment`s exist. Two things make it a *stock* fixture rather than a copy:
//!
//! 1. **Commit dates are set per commit** (`GIT_AUTHOR_DATE`), because a commitment's mint
//!    instant is resolved from the history of the artifact it promises. A fixture that let git
//!    stamp `now()` would put every mint outside any window a test could name, and every
//!    assertion here would pass for the wrong reason.
//! 2. **Discharges are appended in the exact shape `epr flow fulfill` mints them** —
//!    `ReaVerb::Produce`, unit `green-run`, `fulfills: [commitment]` (`fulfill.rs:336-352`).
//!    That is the whole point of test (f): the verb says *produce* and the meaning is *drain*,
//!    and a fold that read the verb would invert the sign of every finding this leg exists to
//!    report.
//!
//! The window is fixed at `2026-08-06..2026-08-13 --per week` — one period, so every rate in
//! these tests is also a raw count and a reader can check the arithmetic by eye.

use std::path::Path;

use elohim_epr::measure::Period;
use elohim_epr_cli::flow::stocks::{
    self, DerivedView, EquilibriumVerdict, RefusalReason, StockName, COMMITMENT_UNIT,
};
use elohim_epr_cli::flow::{project, walk};
use elohim_epr_rea::{
    AgentRef, Commitment, FlowEvent, FlowRecord, FlowStore, Magnitude, ReaVerb, SidecarFlowStore,
    Stock, StockError, Window,
};

const WINDOW: &str = "2026-08-06..2026-08-13";

/// Minted before the window opens — its promise is in the LEVEL and not in the inflow.
const OLD_FEATURE: &str = "genesis/a2o/features/dataplane/resiliency-saga/01-old.feature";
/// Minted inside the window — the inflow.
const NEW_FEATURE: &str = "genesis/a2o/features/dataplane/resiliency-saga/02-new.feature";

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

fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

fn feature(name: &str) -> String {
    format!("Feature: {name}\n  @concern:resiliency\n  Scenario: {name} holds\n    Given a node dies\n    Then the data survives\n")
}

/// A projected tempdir repo with two scenario commitments: one minted 2026-07-01 (before the
/// window) and one minted 2026-08-08 (inside it).
fn projected() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
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
    write(root, OLD_FEATURE, &feature("old"));
    git_at(root, "2026-07-01T00:00:00+00:00", &["init", "-q"]);
    git_at(root, "2026-07-01T00:00:00+00:00", &["add", "-A"]);
    git_at(
        root,
        "2026-07-01T00:00:00+00:00",
        &["commit", "-q", "-m", "before the window"],
    );

    write(root, NEW_FEATURE, &feature("new"));
    git_at(root, "2026-08-08T00:00:00+00:00", &["add", "-A"]);
    git_at(
        root,
        "2026-08-08T00:00:00+00:00",
        &["commit", "-q", "-m", "inside the window"],
    );

    let recipes = root.join(".claude/epr-meta/recipes.yaml");
    project::project(root, &recipes).expect("project runs");
    dir
}

fn window(per: Period) -> Window {
    stocks::parse_window(WINDOW, per).expect("the fixture window parses")
}

fn commitment_cids(root: &Path) -> Vec<cid::Cid> {
    SidecarFlowStore::open(root)
        .unwrap()
        .commitments()
        .unwrap()
        .into_iter()
        .map(|(cid, _)| cid)
        .collect()
}

/// Append a discharge in the EXACT shape `epr flow fulfill` mints one: a `Produce` whose meaning
/// is a drain, carried entirely by `fulfills`.
fn discharge(root: &Path, commitment: cid::Cid, occurred_at: &str) {
    let mut store = SidecarFlowStore::open(root).unwrap();
    let scope = elohim_epr_cli::flow::repo_scope_atom().unwrap();
    store
        .append(FlowRecord::Event(FlowEvent {
            action: ReaVerb::Produce,
            provider: AgentRef("ci:jenkins".into()),
            receiver: elohim_epr_cli::flow::repo_agent(),
            resource: scope,
            quantity: Magnitude::Count {
                value: 1.0,
                unit: "green-run".into(),
            },
            process: None,
            in_scope_of: scope,
            fulfills: vec![commitment],
            satisfies: Vec::new(),
            classified_as: Vec::new(),
            occurred_at: occurred_at.to_string(),
        }))
        .expect("append");
}

fn view(root: &Path) -> DerivedView {
    let store = SidecarFlowStore::open(root).unwrap();
    let records = store.records().unwrap();
    let labels: elohim_epr_cli::flow::Labels =
        std::fs::read_to_string(root.join(".eprfs").join("status").join("labels.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
    stocks::derive_commitment_view(root, &labels, &records).expect("view derives")
}

fn fold(root: &Path, w: &Window) -> elohim_epr_cli::flow::stocks::StockReport {
    stocks::report(StockName::Commitments, &view(root), w, COMMITMENT_UNIT)
}

// (a) ────────────────────────────────────────────────────────────────────────────────────────

/// A draining window: one promise minted inside it, two discharged inside it. `--check` greens.
#[test]
fn a_draining_window_reads_as_equilibrium_and_checks_clean() {
    let dir = projected();
    let root = dir.path();
    let cids = commitment_cids(root);
    assert_eq!(cids.len(), 2, "fixture mints one commitment per feature");
    for cid in &cids {
        discharge(root, *cid, "2026-08-09T12:00:00Z");
    }

    let w = window(Period::Week);
    let outcome = stocks::stocks(root, &w, &[StockName::Commitments]).expect("fold runs");
    let report = &outcome.stocks[0];
    let m = report
        .measures
        .as_ref()
        .expect("a verdict carries measures");

    assert_eq!(
        m.inflow, 1.0,
        "only the 2026-08-08 mint is inside the window"
    );
    assert_eq!(m.outflow, 2.0, "both discharges are inside the window");
    assert_eq!(m.level, 0.0, "both promises left the stock");
    assert_eq!(report.verdict, EquilibriumVerdict::Draining);
    assert!(outcome.equilibrium, "--check exits 0 on this window");
    assert_eq!(report.observed_in_window, 3);
    assert_eq!(report.excluded_by_unit, 0);
}

// (b) ────────────────────────────────────────────────────────────────────────────────────────

/// The mirror: a promise minted inside the window and nothing drained. `--check` must go
/// non-zero, and the stock must be NAMED in the output so the red is actionable.
#[test]
fn a_filling_window_is_non_equilibrium_and_names_the_stock() {
    let dir = projected();
    let root = dir.path();

    let w = window(Period::Week);
    let outcome = stocks::stocks(root, &w, &[StockName::Commitments]).expect("fold runs");
    let report = &outcome.stocks[0];
    let m = report
        .measures
        .as_ref()
        .expect("a verdict carries measures");

    assert_eq!(m.inflow, 1.0);
    assert_eq!(m.outflow, 0.0);
    assert_eq!(
        m.net,
        Some(1.0),
        "net change is the rate the level is moving"
    );
    assert_eq!(report.verdict, EquilibriumVerdict::Filling);
    assert!(
        !outcome.equilibrium,
        "--check must exit non-zero on a filling stock"
    );
    assert_eq!(report.stock, "commitments", "the red names its stock");
    assert_eq!(report.unit, COMMITMENT_UNIT);

    // The level is cumulative through window.end and is deliberately NOT windowed: both promises
    // are still open, including the one minted a month before the window.
    assert_eq!(m.level, 2.0);
}

/// The level this leg reports and the `unfulfilled_total` the rest of the system quotes are the
/// same number by construction — the classifier keys the outflow on `fulfills`, which is exactly
/// the field `store::unfulfilled_in_scope` reads. A divergence here would mean the equilibrium
/// verdict is about a different stock than the one everybody else is looking at.
#[test]
fn the_level_agrees_with_the_unfulfilled_total_every_other_reader_quotes() {
    let dir = projected();
    let root = dir.path();
    let w = stocks::parse_window("2026-07-01..2026-09-01", Period::Week).unwrap();

    let before = walk::status(root).expect("status runs");
    let level_before = fold(root, &w).measures.expect("measures").level;
    assert_eq!(level_before, before.unfulfilled_total as f64);

    discharge(root, commitment_cids(root)[0], "2026-08-09T12:00:00Z");

    let after = walk::status(root).expect("status runs");
    let level_after = fold(root, &w).measures.expect("measures").level;
    assert_eq!(level_after, after.unfulfilled_total as f64);
    assert_eq!(level_after, level_before - 1.0);
}

// (c) ────────────────────────────────────────────────────────────────────────────────────────

/// Mismatched `Period`s refuse with a typed reason, exit non-zero, and leave NO partial verdict.
///
/// This arm cannot be reached through the CLI window path, and that is worth pinning rather than
/// working around: both rates inherit `window.per` from the one declared `Window`
/// (`stock.rs:440-450`), so `Stock::new`'s `MismatchedPeriods` guard is structurally unreachable
/// from here. The test therefore does both halves — it asserts the window path cannot mint the
/// mismatch, and it drives the refusal through the verdict function with a genuinely mismatched
/// `Stock::new` result, which is the only honest way to prove the fail-closed arm runs.
#[test]
fn mismatched_periods_refuse_with_no_partial_verdict() {
    let dir = projected();
    let root = dir.path();

    // Half one: the window path gives both rates one period, whatever `--per` is.
    for per in [Period::Day, Period::Week, Period::Hour] {
        let report = fold(root, &window(per));
        assert!(
            report.measures.is_some(),
            "a single declared period never refuses"
        );
    }

    // Half two: the refusal arm itself.
    let level = elohim_epr::measure::Quantity {
        value: 2.0,
        kind: elohim_epr::measure::MeasureKind::Level,
        confidence: elohim_epr::measure::Confidence::witnessed(
            elohim_epr::measure::Interval::exact(2.0),
            "fixture",
        ),
    };
    let rate = |v: f64, per| elohim_epr::measure::Quantity {
        value: v,
        kind: elohim_epr::measure::MeasureKind::Rate { per },
        confidence: elohim_epr::measure::Confidence::witnessed(
            elohim_epr::measure::Interval::exact(v),
            "fixture",
        ),
    };
    let mismatched: Result<Stock, StockError> =
        Stock::new(level, rate(1.0, Period::Week), rate(9.0, Period::Day));
    let verdict = stocks::equilibrium_verdict(&mismatched, 42);
    match &verdict {
        EquilibriumVerdict::Refused {
            reason: RefusalReason::FoldRefused { kind, .. },
        } => assert_eq!(*kind, "mismatched-periods"),
        other => panic!("expected a typed refusal, got {other:?}"),
    }
    assert!(
        !verdict.is_equilibrium(),
        "a refusal must never green --check"
    );
}

// (d) ────────────────────────────────────────────────────────────────────────────────────────

/// A unit typo excludes every event silently (`count_in` is exact-string) and the stock must
/// then read as REFUSED, with the excluded count reported — never as a drained stock.
///
/// `"commitment"` vs `"commitments"` is this stock's spelling of the `"doc"`/`"docs"` mistake,
/// and it is a one-character difference on a fold key.
#[test]
fn a_unit_typo_reports_its_exclusions_and_never_reads_as_drained() {
    let dir = projected();
    let root = dir.path();
    let w = window(Period::Week);
    let v = view(root);

    let right = stocks::report(StockName::Commitments, &v, &w, COMMITMENT_UNIT);
    assert_eq!(right.observed_in_window, 1);
    assert_eq!(right.excluded_by_unit, 0);
    assert_eq!(right.verdict, EquilibriumVerdict::Filling);

    let typo = stocks::report(StockName::Commitments, &v, &w, "commitments");
    assert_eq!(
        typo.observed_in_window, 0,
        "the fold admitted nothing under the wrong unit"
    );
    assert_eq!(
        typo.excluded_by_unit, 1,
        "and the exclusion is REPORTED, not swallowed"
    );
    assert_eq!(
        typo.verdict,
        EquilibriumVerdict::Refused {
            reason: RefusalReason::NoObservations
        },
        "0 outflow vs 0 inflow from a unit typo must refuse, not pass"
    );
    // The fold SUCCEEDED — it simply admitted nothing — so its measures survive beside the
    // refusal as the evidence for it. `0.000 / 0.000` printed under a REFUSED verdict is what
    // makes a unit typo diagnosable; blanking it would leave a bare red with no handle. The
    // no-half-numbers rule applies to a fold that refused, which is asserted in
    // `mismatched_periods_refuse_with_no_partial_verdict`.
    let m = typo
        .measures
        .expect("a succeeded-but-empty fold keeps its evidence");
    assert_eq!((m.inflow, m.outflow), (0.0, 0.0));
    assert_eq!(m.level, 0.0, "no event carried the folded unit at all");
}

// (e) ────────────────────────────────────────────────────────────────────────────────────────

/// An empty window is a typed non-zero, never a silent green. A check that reads green because
/// it measured nothing is the over-claim this whole leg exists to refuse.
#[test]
fn an_empty_window_refuses_with_a_typed_reason_rather_than_greening() {
    let dir = projected();
    let root = dir.path();
    let w = stocks::parse_window("2026-01-01..2026-01-08", Period::Week).unwrap();

    let outcome = stocks::stocks(root, &w, &[StockName::Commitments]).expect("fold runs");
    let report = &outcome.stocks[0];
    assert_eq!(
        report.verdict,
        EquilibriumVerdict::Refused {
            reason: RefusalReason::NoObservations
        }
    );
    assert!(
        !outcome.equilibrium,
        "--check must exit non-zero on a window that observed nothing"
    );
    assert_eq!(report.observed_in_window, 0);
    // The level is cumulative through `window.end`, not windowed — so a January window over a
    // repo whose first promise is minted in July correctly reports an EMPTY stock, not the
    // present-day one. Which is the whole reason the refusal has to exist: `0 >= 0` on a stock
    // that had not been created yet is arithmetically true and means nothing, and without
    // `NoObservations` it would exit 0 and read as "the development system is in equilibrium."
    let m = report.measures.as_ref().expect("a level is still a level");
    assert_eq!(m.level, 0.0, "nothing had been minted by this window's end");
    assert_eq!((m.inflow, m.outflow), (0.0, 0.0));
}

// (f) ────────────────────────────────────────────────────────────────────────────────────────

/// **The central correctness risk, end to end.** A fulfillment is minted as `ReaVerb::Produce`
/// (`fulfill.rs:336-352`). Folded raw it would land in INFLOW and a draining stock would read as
/// the fastest-filling one — the sign inversion `stock.rs:219-228` warns about, arriving through
/// the data rather than through a caller's choice of index.
#[test]
fn a_produce_shaped_fulfillment_counts_as_discharge_never_as_inflow() {
    let dir = projected();
    let root = dir.path();
    let w = window(Period::Week);

    let before = fold(root, &w).measures.expect("measures");
    assert_eq!(before.inflow, 1.0);
    assert_eq!(before.outflow, 0.0);

    // Discharge the old promise (minted OUTSIDE the window) with an in-window Produce, so the
    // only thing that can move is the outflow. If the verb were read instead of `fulfills`, the
    // inflow would rise to 2.0 and the stock would read as filling twice as fast.
    let old = commitment_cids(root)
        .into_iter()
        .find(|cid| {
            let store = SidecarFlowStore::open(root).unwrap();
            store
                .commitments()
                .unwrap()
                .into_iter()
                .any(|(c, commitment)| c == *cid && names(&commitment, OLD_FEATURE))
        })
        .expect("the pre-window commitment");
    discharge(root, old, "2026-08-10T00:00:00Z");

    let after = fold(root, &w).measures.expect("measures");
    assert_eq!(
        after.inflow, 1.0,
        "a Produce fulfillment must NEVER raise the inflow"
    );
    assert_eq!(after.outflow, 1.0, "it is a discharge — outflow");
    assert_eq!(after.level, before.level - 1.0);
    assert_eq!(
        fold(root, &w).verdict,
        EquilibriumVerdict::Draining,
        "one in, one out over the window is dynamic equilibrium"
    );
}

fn names(commitment: &Commitment, rel: &str) -> bool {
    commitment
        .resource_spec
        .classified_as
        .get(1)
        .is_some_and(|s| s == rel)
}

/// A commitment can only leave the stock once. `fulfill`'s re-fulfillment path (a green run
/// after a regression) appends a SECOND `fulfills` edge for the same promise; counted twice it
/// would drive the level below the `unfulfilled_total` and inflate the outflow rate.
#[test]
fn a_re_fulfillment_does_not_drain_the_same_promise_twice() {
    let dir = projected();
    let root = dir.path();
    let old = commitment_cids(root)[0];
    discharge(root, old, "2026-08-09T00:00:00Z");
    discharge(root, old, "2026-08-10T00:00:00Z");

    let w = window(Period::Week);
    let v = view(root);
    assert_eq!(
        v.redundant_discharges, 1,
        "the second edge is counted, not folded"
    );
    let report = stocks::report(StockName::Commitments, &v, &w, COMMITMENT_UNIT);
    let m = report.measures.expect("measures");
    assert_eq!(m.outflow, 1.0, "one promise, one drain");
    assert_eq!(m.level, 1.0, "the other promise is still open");
}

/// The drain is dated by WHEN IT HAPPENED, not by where it landed in the append-only log.
///
/// This is the `a83b35079` regression channel — *"the exact `8a1c531dc` bug, reintroduced via
/// replay"* — at a second site in the same crate. A delayed or backfilled green report appends an
/// OLDER discharge after a newer one; a dedup keyed on position would then date the drain by the
/// wrong event and file it outside the window entirely, reporting a stock that drained as one
/// that did not.
#[test]
fn the_drain_is_dated_by_occurred_at_never_by_append_order() {
    let dir = projected();
    let root = dir.path();
    let promise = commitment_cids(root)[0];

    // Appended FIRST, but it happened LATER and lands outside the window.
    discharge(root, promise, "2026-08-20T00:00:00Z");
    // Appended SECOND, but it is the real drain and it is inside the window.
    discharge(root, promise, "2026-08-09T00:00:00Z");

    let w = window(Period::Week);
    let report = fold(root, &w);
    let m = report.measures.expect("measures");
    assert_eq!(
        m.outflow, 1.0,
        "the in-window 08-09 discharge is the one that counts; keyed on append order this \
         reads 0.0 and the stock looks undrained"
    );
    assert_eq!(report.verdict, EquilibriumVerdict::Draining);
}

/// Task 1's run-notes are neither inflow nor outflow of the commitments stock (Spec A/B Q2). A
/// correction ABOUT a promise does not move the promise, and the numbers must be byte-identical
/// before and after one is written.
#[test]
fn a_run_note_moves_no_number_in_this_stock() {
    let dir = projected();
    let root = dir.path();
    let w = window(Period::Week);
    let before = fold(root, &w);

    elohim_epr_cli::flow::note::note(
        root,
        NEW_FEATURE,
        "correction",
        "the window is declared, not inferred",
        None,
        &elohim_epr_cli::flow::note::NoteActor::default(),
    )
    .expect("note runs");

    let after = fold(root, &w);
    let (b, a) = (
        before.measures.expect("measures"),
        after.measures.expect("measures"),
    );
    assert_eq!(
        (b.level, b.inflow, b.outflow),
        (a.level, a.inflow, a.outflow)
    );
    assert_eq!(before.verdict, after.verdict);
    assert_eq!(
        before.observed_in_window, after.observed_in_window,
        "a run-note is not an observation of this stock"
    );
}

/// This leg APPENDS NOTHING. The sidecar is byte-identical after a fold, which is what makes it
/// safe to run against a shared worktree while other work is in flight.
#[test]
fn the_fold_is_read_only() {
    let dir = projected();
    let root = dir.path();
    let log = root.join(".eprfs/status/flows.jsonl");
    let before = std::fs::read(&log).unwrap();

    stocks::stocks(root, &window(Period::Week), &[StockName::Commitments]).expect("fold runs");

    assert_eq!(
        std::fs::read(&log).unwrap(),
        before,
        "epr flow stocks must never write"
    );
}

/// A commitment whose artifact has no git history cannot be dated. It stays in the LEVEL (it
/// exists) and drops out of the inflow RATE (we cannot say when it arrived) — which understates
/// inflow, the direction that would make a filling stock look calm. So it is counted and
/// reported rather than defaulted to a window bound.
#[test]
fn an_undatable_mint_is_reported_not_guessed() {
    let dir = projected();
    let root = dir.path();
    // A feature file added to the tree but never committed: `project` mints its commitment,
    // `producing_commit` has nothing to date it with.
    let orphan = "genesis/a2o/features/dataplane/resiliency-saga/03-uncommitted.feature";
    write(root, orphan, &feature("orphan"));
    project::project(root, &root.join(".claude/epr-meta/recipes.yaml")).expect("project runs");

    let v = view(root);
    assert_eq!(v.undated_mints, 1);
    let report = stocks::report(
        StockName::Commitments,
        &v,
        &window(Period::Week),
        COMMITMENT_UNIT,
    );
    assert_eq!(report.undated_mints, 1);
    let m = report.measures.expect("measures");
    assert_eq!(m.inflow, 1.0, "the orphan is absent from the rate");
    assert_eq!(m.level, 3.0, "…and present in the level");
}
