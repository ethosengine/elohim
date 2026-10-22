//! Integration tests for `epr flow sense` — the measure plane's sensing organ — over a synthetic
//! committed tempdir repo.
//!
//! The probes here are `cat` of a file the test rewrites between runs, so each test controls
//! exactly what the "detector" reports and asserts only what the organ does with it: which folds
//! land, which candidates open and close, what a ruling excepts, and what the native report then
//! reads. Fixture shape follows `flow_report.rs`: every note is dated by the git HEAD it was
//! authored against, so the tree is committed before anything is sensed.

use std::path::{Path, PathBuf};

use elohim_epr_cli::flow::note::NoteActor;
use elohim_epr_cli::flow::report::{breaches, report, OutcomeStatus, ReportOptions};
use elohim_epr_cli::flow::sense::{rule, sense, Classification, Selection, SenseStatus};
use tempfile::TempDir;

const MEASURES: &str = r#"measures-version: 1
measures:
  - id: ledger-planes
    version: 1
    family: dev-ledgers
    unit: ledgers
    probe:
      run: "cat count.json"
      emits: count
      due: session
    default-authority: observation
    status: active
  - id: limit-models
    version: 1
    family: substrate-bypass
    unit: files
    probe:
      run: "cat candidates.json"
      emits: candidates
      due: session
    default-authority: observation
    status: active
  - id: limit-models-reset
    version: 1
    family: substrate-bypass
    unit: resets
    default-authority: observation
    status: active
  - id: sense-classes
    version: 1
    family: algedonic-sensing
    unit: candidates
    default-authority: observation
    status: active
  - id: concern-joins-unresolved
    version: 1
    family: concern-ontology
    unit: files
    probe:
      native: concern-joins
      emits: candidates
    default-authority: observation
    status: active
  - id: concern-evidence-unadopted
    version: 1
    family: concern-ontology
    unit: concerns
    probe:
      native: concern-evidence-unadopted
      emits: count
    default-authority: observation
    status: active
  - id: crashing
    version: 1
    family: substrate-bypass
    unit: files
    probe:
      run: "echo boom >&2; exit 3"
      emits: candidates
    default-authority: observation
    status: active
  - id: slow
    version: 1
    family: substrate-bypass
    unit: files
    probe:
      run: "sleep 5; echo '{}'"
      emits: count
      timeout-seconds: 0.3
    default-authority: observation
    status: active
  - id: garbled
    version: 1
    family: substrate-bypass
    unit: files
    probe:
      run: "echo not-json"
      emits: count
    default-authority: observation
    status: active

lenses:
  - id: ledger-planes-ceiling
    version: 1
    binding: binding-local
    class: measure
    consumes: [ledger-planes@1]
    subject: .
    hard: 1
    concern: ledgers-fold-once
    status: active
  - id: limit-models-open
    version: 1
    binding: binding-local
    class: measure
    derive: distinct-subjects-since-reset
    reset: limit-models-reset@1
    consumes: [limit-models@1]
    compare: at-or-above
    hard: 1
    concern: sensing-rides-the-measure-plane
    dispatch-agent: pattern-hunter
    dispatch-prompt: Inspect one open subject's contract.
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
    write(
        root,
        ".epr-meta/repository.yaml",
        "version: 1\nagent: repo:ethosengine/elohim\n",
    );
    write(root, ".claude/epr-meta/measures.yaml", MEASURES);
    write(root, ".claude/epr-meta/policies.yaml", "policies: []\n");
    write(
        root,
        ".epr-meta/ledgers-fold-once.habit.md",
        &habit("ledgers-fold-once"),
    );
    write(
        root,
        "genesis/a2o/features/devflow/story.feature",
        "@concern:story-x @concern:story-y\nFeature: a story\n",
    );
    for file in ["src/a.rs", "src/b.rs", "src/c.rs"] {
        write(root, file, &format!("// {file}\n"));
    }
    write(root, "count.json", "{\"value\": 7}\n");
    write(root, "candidates.json", "{\"candidates\": []}\n");
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "fixture"]);
    dir
}

/// A minimal valid habit atom whose one check adopts story concern `story-x`.
fn habit(id: &str) -> String {
    format!(
        "---\nepr-habit-version: 1\nid: {id}\ninvariant: >\n  holds\nstatus: red\n\
         active: false\nchecks:\n  - \"a2o @concern:story-x (features/devflow/story.feature)\"\n\
         retire-when: >\n  when it holds\n---\n"
    )
}

fn measures_path(root: &Path) -> PathBuf {
    root.join(".claude/epr-meta/measures.yaml")
}

fn select(measure: &str) -> Selection {
    Selection {
        measures: vec![measure.to_string()],
        ..Selection::default()
    }
}

fn run(root: &Path, measure: &str) -> elohim_epr_cli::flow::sense::SenseOutcome {
    let mut out = sense(
        root,
        &measures_path(root),
        &select(measure),
        false,
        &NoteActor::default(),
    )
    .expect("sense runs");
    assert_eq!(out.len(), 1, "one probe selected");
    out.remove(0)
}

fn report_candidates(root: &Path, subjects: &[&str]) {
    let rows: Vec<String> = subjects
        .iter()
        .map(|s| format!("{{\"subject\": \"{s}\", \"symbol\": \"L1\"}}"))
        .collect();
    write(
        root,
        "candidates.json",
        &format!("{{\"candidates\": [{}]}}\n", rows.join(", ")),
    );
}

fn outcome_of(root: &Path, bound: &str) -> (OutcomeStatus, Option<f64>) {
    let payload = report(root, &ReportOptions::new(root)).expect("report runs");
    let outcome = payload
        .outcomes()
        .iter()
        .find(|o| o.bound == bound)
        .unwrap_or_else(|| panic!("bound {bound} is declared"))
        .clone();
    (outcome.outcome, outcome.observed)
}

#[test]
fn a_count_probe_folds_its_value_and_only_folds_again_on_change() {
    let dir = fixture();
    let root = dir.path();
    assert_eq!(
        outcome_of(root, "ledger-planes-ceiling@1").0,
        OutcomeStatus::Skipped,
        "unsensed: numb, not zero"
    );

    let first = run(root, "ledger-planes@1");
    assert_eq!(first.status, SenseStatus::Sensed);
    assert_eq!(first.value, Some(7.0));
    assert_eq!(first.written.len(), 1);
    assert_eq!(
        outcome_of(root, "ledger-planes-ceiling@1"),
        (OutcomeStatus::Failed, Some(7.0))
    );

    let again = run(root, "ledger-planes@1");
    assert!(
        again.written.is_empty(),
        "an unchanged reading writes nothing"
    );

    write(root, "count.json", "{\"value\": 1}\n");
    let changed = run(root, "ledger-planes@1");
    assert_eq!(changed.written.len(), 1);
    assert_eq!(
        outcome_of(root, "ledger-planes-ceiling@1"),
        (OutcomeStatus::Passed, Some(1.0))
    );
}

#[test]
fn candidates_open_with_one_and_heal_with_zero_so_the_lens_reads_the_open_set() {
    let dir = fixture();
    let root = dir.path();

    report_candidates(root, &["src/a.rs", "src/b.rs"]);
    let first = run(root, "limit-models@1");
    assert_eq!(first.opened, vec!["src/a.rs", "src/b.rs"]);
    assert!(first.closed.is_empty());
    assert_eq!(first.value, Some(2.0));
    assert_eq!(
        outcome_of(root, "limit-models-open@1"),
        (OutcomeStatus::Failed, Some(2.0))
    );

    // b migrated, c appeared: one heal, one open, and nothing re-written for a.
    report_candidates(root, &["src/a.rs", "src/c.rs"]);
    let second = run(root, "limit-models@1");
    assert_eq!(second.opened, vec!["src/c.rs"]);
    assert_eq!(second.closed, vec!["src/b.rs"]);
    assert_eq!(
        outcome_of(root, "limit-models-open@1"),
        (OutcomeStatus::Failed, Some(2.0))
    );

    report_candidates(root, &[]);
    let drained = run(root, "limit-models@1");
    assert_eq!(drained.closed, vec!["src/a.rs", "src/c.rs"]);
    assert_eq!(
        outcome_of(root, "limit-models-open@1"),
        (OutcomeStatus::Passed, Some(0.0)),
        "every candidate healed: a witnessed zero, not a skip"
    );
}

#[test]
fn a_clean_first_reading_opens_the_window_so_the_lens_reads_zero_not_numb() {
    let dir = fixture();
    let root = dir.path();
    assert_eq!(
        outcome_of(root, "limit-models-open@1").0,
        OutcomeStatus::Skipped
    );

    let first = run(root, "limit-models@1");
    assert!(first.opened.is_empty());
    assert!(
        first.reset.is_some(),
        "an empty set still marks the reading"
    );
    assert_eq!(
        outcome_of(root, "limit-models-open@1"),
        (OutcomeStatus::Passed, Some(0.0)),
        "observed clean, not skipped for want of a fold"
    );

    let again = run(root, "limit-models@1");
    assert!(again.reset.is_none() && again.written.is_empty());
}

#[test]
fn a_deleted_candidate_drains_through_the_declared_reset_and_the_live_set_reopens() {
    let dir = fixture();
    let root = dir.path();
    report_candidates(root, &["src/a.rs", "src/b.rs"]);
    run(root, "limit-models@1");

    std::fs::remove_file(root.join("src/b.rs")).unwrap();
    report_candidates(root, &["src/a.rs"]);
    let out = run(root, "limit-models@1");
    assert!(out.reset.is_some(), "the gone subject cannot take a zero");
    assert!(out.stuck.is_empty());
    assert_eq!(
        out.opened,
        vec!["src/a.rs"],
        "the live set re-opens after the reset"
    );
    assert_eq!(
        outcome_of(root, "limit-models-open@1"),
        (OutcomeStatus::Failed, Some(1.0))
    );
}

#[test]
fn an_excepting_ruling_closes_and_names_its_subject_while_duplication_stays_open() {
    let dir = fixture();
    let root = dir.path();
    report_candidates(root, &["src/a.rs", "src/b.rs", "src/c.rs"]);
    run(root, "limit-models@1");

    let actor = NoteActor::default();
    let mp = measures_path(root);
    rule(
        root,
        &mp,
        "limit-models@1",
        "src/a.rs",
        Classification::Intentional,
        "a declaration reader, it judges nothing",
        &actor,
    )
    .unwrap();
    rule(
        root,
        &mp,
        "limit-models@1",
        "src/b.rs",
        Classification::Duplication,
        "re-judges the lens the native report already judges",
        &actor,
    )
    .unwrap();
    let missing_why = rule(
        root,
        &mp,
        "limit-models@1",
        "src/c.rs",
        Classification::FalsePositive,
        "  ",
        &actor,
    );
    assert!(
        missing_why.is_err(),
        "a ruling must state the contract difference"
    );

    let out = run(root, "limit-models@1");
    assert_eq!(
        out.closed,
        vec!["src/a.rs"],
        "the excepted subject is healed"
    );
    assert_eq!(out.excepted.len(), 1);
    assert_eq!(out.excepted[0].subject, "src/a.rs");
    assert_eq!(out.excepted[0].class, Classification::Intentional);
    assert!(
        out.open.contains_key("src/b.rs"),
        "duplication keeps it open"
    );
    assert_eq!(out.classes.get("intentional"), Some(&1));
    assert_eq!(out.classes.get("duplication"), Some(&1));
    assert_eq!(out.classes.get("unruled"), Some(&1));
    assert_eq!(
        outcome_of(root, "limit-models-open@1"),
        (OutcomeStatus::Failed, Some(2.0))
    );
}

#[test]
fn a_crashed_slow_or_garbled_probe_writes_nothing_and_is_refused() {
    let dir = fixture();
    let root = dir.path();
    for measure in ["crashing@1", "slow@1", "garbled@1"] {
        let out = run(root, measure);
        assert_eq!(out.status, SenseStatus::Refused, "{measure}");
        assert!(out.written.is_empty(), "{measure} wrote nothing");
        assert!(out.refused.is_some());
    }
    assert!(
        !root.join(".eprfs/status/flows.jsonl").exists(),
        "no record of any kind reached the sidecar"
    );
}

#[test]
fn breaches_name_the_concern_the_dispatch_and_the_outstanding_subjects() {
    let dir = fixture();
    let root = dir.path();
    report_candidates(root, &["src/a.rs", "src/c.rs"]);
    run(root, "limit-models@1");
    run(root, "ledger-planes@1");

    let found = breaches(root, &ReportOptions::new(root)).unwrap();
    let open = found
        .iter()
        .find(|b| b.bound == "limit-models-open@1")
        .expect("the candidate lens breaches");
    assert_eq!(
        open.concern.as_deref(),
        Some("sensing-rides-the-measure-plane")
    );
    assert_eq!(open.dispatch_agent.as_deref(), Some("pattern-hunter"));
    assert_eq!(open.subjects, vec!["src/a.rs", "src/c.rs"]);

    let ledgers = found
        .iter()
        .find(|b| b.bound == "ledger-planes-ceiling@1")
        .expect("the count lens breaches");
    assert_eq!(ledgers.subjects, vec!["."]);
    assert_eq!(ledgers.dispatch_agent, None);
}

#[test]
fn the_sense_headline_counts_observed_breached_and_numb_bounds() {
    let dir = fixture();
    let root = dir.path();
    let before = report(root, &ReportOptions::new(root)).unwrap();
    assert!(
        before.sense_line().contains("0 of 2 bounds observed"),
        "{}",
        before.sense_line()
    );
    assert!(before.sense_line().contains("numb 2"));

    run(root, "ledger-planes@1");
    let after = report(root, &ReportOptions::new(root)).unwrap();
    let line = after.sense_line();
    assert!(line.contains("1 of 2 bounds observed"), "{line}");
    assert!(line.contains("1 breached"), "{line}");
    assert!(line.contains("numb 1 (limit-models)"), "{line}");
}

#[test]
fn a_concern_reference_is_judged_by_its_relations_home_never_by_its_spelling() {
    let dir = fixture();
    let root = dir.path();

    // The lens pricing `ledgers-fold-once` resolves (a habit declares it); the one pricing
    // `sensing-rides-the-measure-plane` does not — no habit atom declares that id yet.
    let out = run(root, "concern-joins-unresolved@1");
    assert_eq!(out.status, SenseStatus::Sensed);
    assert_eq!(out.opened, vec![".claude/epr-meta/measures.yaml"]);
    let symbols = &out.open[".claude/epr-meta/measures.yaml"];
    assert_eq!(
        symbols,
        &vec!["prices:sensing-rides-the-measure-plane".to_string()]
    );

    // story-x is adopted by the habit's check; story-y waits for a habit: one unadopted story.
    let unadopted = run(root, "concern-evidence-unadopted@1");
    assert_eq!(unadopted.value, Some(1.0));
    assert!(
        unadopted.detail[0].starts_with("story-y"),
        "{:?}",
        unadopted.detail
    );

    // Declaring the habit is the repair — not respelling the reference.
    write(
        root,
        "elohim/eprfs/.epr-meta/sensing-rides-the-measure-plane.habit.md",
        &habit("sensing-rides-the-measure-plane"),
    );
    let healed = run(root, "concern-joins-unresolved@1");
    assert_eq!(healed.closed, vec![".claude/epr-meta/measures.yaml"]);
    assert!(healed.open.is_empty());
}
