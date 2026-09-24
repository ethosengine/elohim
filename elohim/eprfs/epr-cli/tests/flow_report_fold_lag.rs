//! Governed-discovery station 4, task 4.6: fold-lag freshness replaces the manual mine gate.
//!
//! `derive: fold-lag` reads the semantic fold's own read-only status for the pinned embedder —
//! the lag the fold's size+mtime stat cache already computes — so the headline's `index:` line is
//! the fold's own reading, with no second walk and no producer script. With no pinned store the
//! bound is `skipped — no fold`, never a zero. The mempalace bound stays DECLARED (the visitor's
//! maintenance, answerable by name) but leaves the headline; `index` takes its slot position.
//!
//! Every fold here runs the `Fixture` embedder on a temporary tree carrying the live contract and
//! a copy of the live measure whose surface is the fixture's own. The derive reads the PINNED
//! store, which a fixture fold never writes, so a test that needs one copies the fixture store
//! into the pinned slot: the status read checks the store's schema and measure, not which
//! embedder filled its vectors, and lag is a property of the manifest, not of the vectors.
mod common;

use std::path::Path;
use std::process::Command;

use elohim_epr_cli::flow::memory::recall::index::{self, EmbedderChoice, FoldOptions, FoldRun};
use elohim_epr_cli::flow::memory::recall::CONTRACT_REL;
use elohim_epr_cli::flow::report::{report, BoundOutcome, OutcomeStatus, ReportOptions};
use elohim_epr_rea::IndexMeasure;
use serde_json::{json, Value};
use tempfile::TempDir;

const MEASURE_REL: &str = ".epr-meta/elohim/algorithms/recall-semantic-index.json";

/// The fixture registry: the index bound as the live registry declares it, and the mempalace
/// bound kept declared beside it (its marker absent, so it reads `skipped`).
const MEASURES: &str = r#"
measures:
  - id: index-fold-lag
    version: 1
    family: index
    unit: files
    procedure: "epr flow report — derive: fold-lag"
    status: active
  - id: mempalace-surfaces-changed
    version: 1
    family: mempalace
    unit: files
    procedure: "epr flow report — the native surface walk"
    status: active

lenses:
  - id: index-fold-lag-ceiling
    version: 1
    headline: index
    compare: above
    class: inject
    consumes: [index-fold-lag@1]
    derive: fold-lag
    hard: 25
    status: active
  - id: mempalace-surfaces-changed-ceiling
    version: 1
    headline: mempalace
    compare: at-or-above
    class: measure
    consumes: [mempalace-surfaces-changed@1]
    hard: 1
    derive: files-newer-than
    marker: .mempalace/.last-mine
    surfaces:
      - genesis
    suffix: .md
    status: active
"#;

fn live(rel: &str) -> Value {
    let raw = std::fs::read(common::repo_root().join(rel)).expect("live file reads");
    serde_json::from_slice(&raw).expect("live file parses")
}

/// A committed temporary repository: the live contract, the live measure narrowed to the
/// fixture's own surface, the fixture registry and two notes.
fn tree() -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let contract = serde_json::to_string_pretty(&common::live_contract()).unwrap();
    common::write(root, CONTRACT_REL, &contract);
    let mut measure = live(MEASURE_REL);
    measure["surfaces"]["paths"] = json!(["genesis/**/*.md"]);
    common::write(
        root,
        MEASURE_REL,
        &serde_json::to_string_pretty(&measure).unwrap(),
    );
    common::write(root, ".claude/epr-meta/measures.yaml", MEASURES);
    common::write(root, "genesis/alpha.md", "# Alpha\nThe fold has a lag.\n");
    common::write(root, "genesis/beta.md", "# Beta\nStewardship.\n");
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn git(root: &Path, args: &[&str]) {
    let out = elohim_epr_cli::process::build_command("git", args, root, &[])
        .env("GIT_AUTHOR_NAME", "Fixture Author")
        .env("GIT_COMMITTER_NAME", "Fixture Author")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.test")
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}");
}

fn measure_cid(root: &Path) -> String {
    let measure: IndexMeasure = serde_json::from_value(live_in(root, MEASURE_REL)).unwrap();
    measure.cid().unwrap().to_string()
}

fn live_in(root: &Path, rel: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root.join(rel)).unwrap()).unwrap()
}

/// Fold with the fixture embedder, then place that store in the pinned slot the derive reads.
fn fold_into_pinned(root: &Path) {
    let run = index::fold(
        root,
        &FoldOptions {
            embedder: EmbedderChoice::Fixture,
            ..FoldOptions::default()
        },
    )
    .expect("the fold runs");
    assert!(
        matches!(run, FoldRun::Done(_)),
        "no other fold holds the store"
    );
    let cid = measure_cid(root);
    let from = index::store_dir(root, &cid, EmbedderChoice::Fixture);
    let to = index::store_dir(root, &cid, EmbedderChoice::Pinned);
    std::fs::create_dir_all(&to).unwrap();
    std::fs::copy(from.join("fold.sqlite"), to.join("fold.sqlite")).unwrap();
}

/// `n` new tracked notes the fold has not seen.
fn add_notes(root: &Path, from: usize, n: usize) {
    for i in from..from + n {
        common::write(
            root,
            &format!("genesis/new-{i}.md"),
            &format!("# New {i}\n"),
        );
    }
    git(root, &["add", "-A"]);
}

fn index_outcome(root: &Path) -> BoundOutcome {
    report(root, &ReportOptions::new(root))
        .unwrap()
        .outcomes()
        .iter()
        .find(|o| o.bound.starts_with("index-fold-lag-ceiling@"))
        .cloned()
        .expect("the index bound is evaluated")
}

fn slot(root: &Path, name: &str) -> String {
    report(root, &ReportOptions::new(root))
        .unwrap()
        .headline_line(name)
}

fn headline(root: &Path) -> Vec<String> {
    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(["flow", "report", "--headline", "--root"])
        .arg(root)
        .output()
        .expect("epr runs");
    assert_eq!(out.status.code(), Some(0));
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| line.trim().to_string())
        .collect()
}

/// Step 5: `epr flow report --headline` prints `index:` in the position `mempalace:` held (the
/// second line), and with no pinned store it reads `skipped — no fold`, never a zero. The
/// mempalace line leaves the headline.
#[test]
fn the_headline_prints_index_in_mempalaces_slot_and_skipped_with_no_fold() {
    let dir = tree();
    let root = dir.path();
    let lines = headline(root);
    assert!(lines[0].starts_with("recall:"), "{lines:?}");
    assert_eq!(lines[1], "index: skipped — no fold", "{lines:?}");
    assert!(lines[2].starts_with("cleanup:"), "{lines:?}");
    assert!(lines[3].starts_with("scope:"), "{lines:?}");
    assert!(lines[4].starts_with("memory-budget:"), "{lines:?}");
    assert!(
        !lines.iter().any(|line| line.starts_with("mempalace:")),
        "the visitor's mine gate is no longer a headline slot: {lines:?}"
    );

    let outcome = index_outcome(root);
    assert_eq!(outcome.outcome, OutcomeStatus::Skipped);
    assert_eq!(outcome.observed, None, "no fold is never a zero");
    assert_eq!(outcome.derive.as_deref(), Some("fold-lag"));
}

/// The derive reads the store's own lag: files new or changed since their fold, from the fold's
/// read-only status.
#[test]
fn the_fold_lag_derive_reads_the_stores_lag() {
    let dir = tree();
    let root = dir.path();
    fold_into_pinned(root);
    let caught_up = index_outcome(root);
    assert_eq!(caught_up.outcome, OutcomeStatus::Passed, "{caught_up:?}");
    assert_eq!(
        caught_up.observed,
        Some(0.0),
        "a witnessed zero, from a real fold"
    );
    assert_eq!(
        slot(root, "index"),
        "index: 0 files behind the fold within hard 25 ✅"
    );

    add_notes(root, 0, 3);
    let behind = index_outcome(root);
    assert_eq!(behind.observed, Some(3.0));
    assert_eq!(behind.unit.as_deref(), Some("files"));
    assert_eq!(
        slot(root, "index"),
        "index: 3 files behind the fold within hard 25 ✅"
    );
}

/// The hard bound is a ceiling whose watermark is the last acceptable value — the same reading
/// the measure's own `foldLag` limit and the surfacing hook's DEGRADED banner take: 25 behind is
/// within, 26 is past.
#[test]
fn the_index_line_fails_only_past_the_hard_bound() {
    let dir = tree();
    let root = dir.path();
    fold_into_pinned(root);
    add_notes(root, 0, 25);
    assert_eq!(
        slot(root, "index"),
        "index: 25 files behind the fold within hard 25 ✅"
    );
    add_notes(root, 25, 1);
    let past = index_outcome(root);
    assert_eq!(past.outcome, OutcomeStatus::Failed);
    assert_eq!(
        slot(root, "index"),
        "index: ⚠ failed — 26 files behind the fold is past the hard watermark 25"
    );
}

/// The mempalace bound stays declared and evaluated — a reader asking for it by name gets its
/// own word back — it only left the headline's order.
#[test]
fn the_mempalace_bound_stays_answerable_by_name() {
    let dir = tree();
    let root = dir.path();
    let line = slot(root, "mempalace");
    assert!(line.starts_with("mempalace: skipped"), "{line}");
    let payload = report(root, &ReportOptions::new(root)).unwrap();
    assert!(payload
        .outcomes()
        .iter()
        .any(|o| o.bound.starts_with("mempalace-surfaces-changed-ceiling@")));
}

/// The live registry declares the four measures the plan names, and the headline bound agrees
/// with the measure's own declared `foldLag` limit (two statements of one ceiling that must not
/// drift apart). `index-fold-files-per-run@1` carries no number of its own: it cites the contract.
#[test]
fn the_live_registry_declares_the_index_measures_coherently() {
    let root = common::repo_root();
    let text = std::fs::read_to_string(root.join(".claude/epr-meta/measures.yaml")).unwrap();
    let doc: serde_yaml::Value = serde_yaml::from_str(&text).unwrap();
    let row = |section: &str, id: &str| -> serde_yaml::Value {
        doc[section]
            .as_sequence()
            .unwrap()
            .iter()
            .find(|r| r["id"].as_str() == Some(id))
            .cloned()
            .unwrap_or_else(|| panic!("{section} declares {id}"))
    };

    let lag = row("measures", "index-fold-lag");
    assert_eq!(lag["family"].as_str(), Some("index"));
    assert_eq!(lag["unit"].as_str(), Some("files"));

    let ceiling = row("lenses", "index-fold-lag-ceiling");
    assert_eq!(ceiling["headline"].as_str(), Some("index"));
    assert_eq!(ceiling["derive"].as_str(), Some("fold-lag"));
    let declared: IndexMeasure = serde_json::from_value(live(MEASURE_REL)).unwrap();
    assert_eq!(
        ceiling["hard"].as_f64(),
        Some(declared.fold_lag.limit),
        "the headline's hard bound is the measure's own foldLag limit"
    );

    let per_run = row("measures", "index-fold-files-per-run");
    let provenance = per_run["provenance"].as_str().unwrap();
    assert!(
        provenance.contains("limits.fold_files_per_run"),
        "{provenance}"
    );
    assert!(
        per_run.get("default").is_none() && per_run.get("value").is_none(),
        "one home: the contract carries the number"
    );

    let reach = row("measures", "recall-bank-reach");
    assert_eq!(reach["family"].as_str(), Some("recall"));
    assert!(reach["procedure"].as_str().unwrap().contains("model="));

    // The mempalace bound stays declared, off the headline order.
    row("lenses", "mempalace-surfaces-changed-ceiling");
}
