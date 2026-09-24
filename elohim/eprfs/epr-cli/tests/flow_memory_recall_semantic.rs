//! Governed-discovery station 4, task 4.4: the `semantic` provider ranks natively.
//!
//! `search --provider semantic` embeds the QUESTION TEXT once (never the lexical term list), takes
//! the cosine of it against every live chunk vector in the fold the recipe declares
//! (`ceremony.providers.semantic.embedder` names which store: `pinned` live, `fixture` here), keeps
//! each file's best chunk and returns the top `limits.search_results` files. Every candidate prints
//! its producer, the `IndexMeasure` CID it was ranked under and the fold's lag at answer time, and
//! is located at a passage so its linked `read` lands there. Absence is honest, never an error: no
//! fold, an unavailable embedder, or a fold built under another method each answer with one
//! `unresolved` line and no candidates.
//!
//! Every test folds a temporary tree with the `Fixture` embedder (a hashed bag of words: texts
//! sharing words land near each other) and reads the real SQLite store through the provider.
mod common;

use std::path::Path;
use std::process::Command;

use elohim_epr_cli::flow::memory::recall::index::{self, EmbedderChoice, FoldOptions, FoldRun};
use elohim_epr_cli::flow::memory::recall::{retrieve, Contract, CONTRACT_REL};
use elohim_epr_rea::IndexMeasure;
use serde_json::{json, Value};
use tempfile::TempDir;

const MEASURE_REL: &str = ".epr-meta/elohim/algorithms/recall-semantic-index.json";
const NO_FOLD: &str = "semantic: no fold — run epr flow memory index fold";
const OTHER_METHOD: &str = "semantic: the fold was built under another method — refold";

/// The fixture's own surface (the live measure's is the repository's authority layer).
const SURFACE: [&str; 4] = [
    "genesis/**/*.md",
    "genesis/**/*.py",
    "genesis/**/*.sh",
    ".claude/**/*.md",
];

/// A question whose every word is under four characters: the lexical route's term list for it is
/// empty (`question_terms` keeps short tokens only when the contract declares them), so only a
/// route that embeds the question text itself can rank anything.
const SHORT_WORDED: &str = "who can fix a bug now";

fn live(rel: &str) -> Value {
    let raw = std::fs::read(common::repo_root().join(rel)).expect("live file reads");
    serde_json::from_slice(&raw).expect("live file parses")
}

fn put_json(root: &Path, rel: &str, value: &Value) {
    common::write(root, rel, &serde_json::to_string_pretty(value).unwrap());
}

/// A temporary repository: the live contract (its semantic provider declaring `embedder`), a
/// fixture-surface copy of the live measure, and a small tree.
fn tree(embedder: &str) -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let mut contract = live(CONTRACT_REL);
    contract["ceremony"]["providers"]["semantic"]["embedder"] = json!(embedder);
    put_json(root, CONTRACT_REL, &contract);
    let mut measure = live(MEASURE_REL);
    measure["surfaces"]["paths"] = json!(SURFACE);
    put_json(root, MEASURE_REL, &measure);
    common::write(
        root,
        "genesis/alpha.md",
        "# Alpha\nThe stale index is rebuilt by a fold.\n## Detail\nFold lag is a bound.\n",
    );
    common::write(
        root,
        "genesis/beta.md",
        "# Beta\nStewardship of the commons.\n",
    );
    common::write(
        root,
        "genesis/triage.md",
        "# Intro\nA note about nothing in particular.\n## Who fixes it\nWho can fix the bug now? \
         Ask the one on call.\n",
    );
    common::write(
        root,
        "genesis/tool.py",
        "import os\n\ndef orbit_launch():\n    \"\"\"launch the orbit daemon\"\"\"\n    return 1\n",
    );
    common::write(
        root,
        "genesis/run.sh",
        "export ORBIT_HOME=/opt/orbit\nstart telemetry relay beacon quickly\n",
    );
    common::write(
        root,
        "genesis/dup.md",
        "# Guide\n## Example\nalpha beta gamma\n## Example\nquasar nebula pulsar\n",
    );
    common::write(
        root,
        "genesis/aio.py",
        "async def zephyr_wind():\n    return 'zephyr gust'\n",
    );
    common::write(root, ".claude/notes.md", "# Notes\nA household mesh.\n");
    dir
}

fn fold(root: &Path) {
    let opts = FoldOptions {
        embedder: EmbedderChoice::Fixture,
        ..FoldOptions::default()
    };
    match index::fold(root, &opts).expect("the fold runs") {
        FoldRun::Done(_) => {}
        FoldRun::Busy => panic!("no other fold holds the store"),
    }
}

fn contract(root: &Path) -> Contract {
    Contract::load(&root.join(CONTRACT_REL)).expect("the contract loads")
}

fn ask_in(root: &Path, query: &str, scope: &str) -> Value {
    retrieve(root, &contract(root), "semantic", query, scope, &[], &[])
        .expect("an honest answer, never an error")
}

fn ask(root: &Path, query: &str) -> Value {
    ask_in(root, query, ".")
}

fn measure_cid(root: &Path) -> String {
    let measure: IndexMeasure = serde_json::from_value(
        serde_json::from_slice(&std::fs::read(root.join(MEASURE_REL)).unwrap()).unwrap(),
    )
    .unwrap();
    measure.cid().unwrap().to_string()
}

fn paths(answer: &Value) -> Vec<String> {
    answer["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .map(|c| c["path"].as_str().unwrap_or_default().to_string())
        .collect()
}

fn unresolved(answer: &Value) -> Vec<String> {
    answer["unresolved"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|m| m.as_str().unwrap_or_default().to_string())
        .collect()
}

fn omissions(answer: &Value) -> Vec<String> {
    answer["omissions"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|m| m.as_str().unwrap_or_default().to_string())
        .collect()
}

/// The fixture fold's store copied to the PINNED store directory (its meta names the declared
/// measure and model), relabelled `embedder` when given; and the live model manifest, its model
/// resolving nowhere. The procedure `embed.py` is NOT copied: running it would fail loudly.
fn pinned_store_from_fixture(root: &Path, embedder: Option<&str>) {
    use elohim_epr_cli::flow::memory::recall::embedder::MODEL_MANIFEST_REL;
    let cid = measure_cid(root);
    let pinned = index::store_dir(root, &cid, EmbedderChoice::Pinned);
    std::fs::create_dir_all(&pinned).unwrap();
    std::fs::copy(
        index::store_dir(root, &cid, EmbedderChoice::Fixture).join("fold.sqlite"),
        pinned.join("fold.sqlite"),
    )
    .unwrap();
    if let Some(label) = embedder {
        let conn = rusqlite::Connection::open(pinned.join("fold.sqlite")).unwrap();
        conn.execute("UPDATE meta SET value = ?1 WHERE key = 'embedder'", [label])
            .unwrap();
    }
    let mut manifest = live(MODEL_MANIFEST_REL);
    manifest["resolve"] = json!([root.join("no-model-here").to_string_lossy()]);
    put_json(root, MODEL_MANIFEST_REL, &manifest);
}

/// The label a store folded by the declared procedure carries (`index.rs`'s `embedder_for`).
fn pinned_label() -> String {
    use elohim_epr_cli::flow::memory::recall::embedder::MODEL_MANIFEST_REL;
    let manifest = live(MODEL_MANIFEST_REL);
    format!(
        "procedure {} on model {}",
        manifest["procedure"].as_str().unwrap(),
        manifest["model_bytes"].as_str().unwrap()
    )
}

/// Step 3: a question the lexical route cannot rank (no term of four characters) still finds its
/// passage, because the semantic route embeds the question's own text.
#[test]
fn a_question_worded_unlike_the_targets_terms_ranks_the_target_first() {
    let dir = tree("fixture");
    let root = dir.path();
    fold(root);

    let lexical = retrieve(
        root,
        &contract(root),
        "local",
        SHORT_WORDED,
        "genesis",
        &[],
        &[],
    )
    .expect("the local route answers");
    assert!(
        !paths(&lexical).contains(&"genesis/triage.md".to_string()),
        "the lexical route has no term to find it by: {lexical}"
    );

    let answer = ask(root, SHORT_WORDED);
    assert!(unresolved(&answer).is_empty(), "{answer}");
    let found = paths(&answer);
    assert_eq!(found.first().map(String::as_str), Some("genesis/triage.md"));
    assert!(found.len() <= 3, "limits.search_results bounds the window");
    let top = &answer["candidates"][0];
    // The linked read lands on the passage: the outline heading the winning chunk came from.
    assert_eq!(top["best_section"]["title"], "Who fixes it");
    assert_eq!(top["best_section"]["lines"], "3:4");
    assert_eq!(top["best_section"]["window_complete"], true);
    assert!(top["best_section"]["hits"].is_object());
    let score = top["score"].as_f64().expect("a cosine");
    assert!(score > 0.0 && score <= 1.0, "{score}");
    assert_eq!((score * 10_000.0).round() / 10_000.0, score, "4 decimals");
    let scores: Vec<f64> = answer["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["score"].as_f64().unwrap())
        .collect();
    assert!(scores.windows(2).all(|w| w[0] >= w[1]), "{scores:?}");
    assert_eq!(answer["usage"]["search_queries"], 1);
    assert!(answer["usage"]["semantic_chunks_scanned"].as_u64().unwrap() > 0);
    assert!(answer["usage"]["semantic_query_ms"].is_number());
    assert!(answer["usage"]["provider_seconds"].is_number());
    assert!(
        answer["usage"].get("source_bytes").is_none(),
        "the vector scan reads a derived store; it is reported, never charged as source"
    );
}

/// Step 3: every candidate carries its producer and the method it was ranked under; the ranking
/// is known.
#[test]
fn every_candidate_prints_the_measure_cid_and_the_ranking_is_known() {
    let dir = tree("fixture");
    let root = dir.path();
    fold(root);
    let cid = measure_cid(root);
    let answer = ask(root, "stale index fold lag");
    assert_eq!(answer["ranking_known"], true);
    assert_eq!(answer["method"], json!(cid));
    assert_eq!(answer["producer"], "semantic");
    assert_eq!(answer["embedder"], "fixture");
    assert_eq!(answer["fold_lag"], 0);
    let candidates = answer["candidates"].as_array().expect("candidates");
    assert!(!candidates.is_empty());
    assert_eq!(candidates[0]["path"], "genesis/alpha.md");
    for candidate in candidates {
        assert_eq!(candidate["producer"], "semantic");
        assert_eq!(candidate["method"], json!(cid));
        assert_eq!(candidate["fold_lag"], 0);
        assert!(
            candidate.get("model").is_some_and(Value::is_null),
            "fixture vectors are no model's: {candidate}"
        );
    }
    assert!(answer["fitness"]
        .as_str()
        .unwrap_or_default()
        .starts_with("fixture embedder only"));
}

/// A python `def` locates by its outline heading; an unsectioned file's window falls back to the
/// winning chunk's own line range.
#[test]
fn best_section_is_the_outline_heading_or_the_chunks_line_range() {
    let dir = tree("fixture");
    let root = dir.path();
    fold(root);
    let py = ask(root, "orbit launch daemon");
    let top = &py["candidates"][0];
    assert_eq!(top["path"], "genesis/tool.py", "{py}");
    assert_eq!(top["best_section"]["title"], "def orbit_launch():");
    assert_eq!(top["best_section"]["lines"], "3:5");

    // A repeated title lands on the occurrence whose range holds the winning chunk.
    let dup = ask(root, "quasar nebula pulsar");
    let top = &dup["candidates"][0];
    assert_eq!(top["path"], "genesis/dup.md", "{dup}");
    assert_eq!(top["best_section"]["title"], "Example");
    assert_eq!(top["best_section"]["lines"], "4:5");

    // An `async def` is a chunk section but no outline heading: the chunk's own located range.
    let aio = ask(root, "zephyr wind gust");
    let top = &aio["candidates"][0];
    assert_eq!(top["path"], "genesis/aio.py", "{aio}");
    assert_eq!(top["best_section"]["title"], "async def zephyr_wind():");
    assert_eq!(top["best_section"]["lines"], "1:2");

    let sh = ask(root, "telemetry relay beacon");
    let top = &sh["candidates"][0];
    assert_eq!(top["path"], "genesis/run.sh", "{sh}");
    assert_eq!(top["best_section"]["title"], "lines 1-2");
    assert_eq!(top["best_section"]["lines"], "1:2");
}

/// A scoped search stays scoped.
#[test]
fn the_search_scope_bounds_the_candidates() {
    let dir = tree("fixture");
    let root = dir.path();
    fold(root);
    assert!(paths(&ask(root, "household mesh")).contains(&".claude/notes.md".to_string()));
    let scoped = ask_in(root, "household mesh", "genesis");
    assert!(paths(&scoped).iter().all(|p| p.starts_with("genesis/")));
    assert_eq!(scoped["scope"], "genesis");
}

/// Step 3: a stale fold answers, and says how stale — on the result and on every candidate.
#[test]
fn a_fold_thirty_files_behind_still_answers_and_says_so() {
    let dir = tree("fixture");
    let root = dir.path();
    fold(root);
    for n in 0..30 {
        common::write(
            root,
            &format!("genesis/later/note-{n:02}.md"),
            &format!("# Later {n}\nWritten after the fold.\n"),
        );
    }
    let answer = ask(root, SHORT_WORDED);
    assert!(unresolved(&answer).is_empty(), "{answer}");
    assert_eq!(answer["fold_lag"], 30);
    assert_eq!(
        paths(&answer).first().map(String::as_str),
        Some("genesis/triage.md")
    );
    for candidate in answer["candidates"].as_array().unwrap() {
        assert_eq!(candidate["fold_lag"], 30);
    }
    let omissions: Vec<String> = answer["omissions"]
        .as_array()
        .expect("omissions")
        .iter()
        .map(|m| m.as_str().unwrap_or_default().to_string())
        .collect();
    assert!(
        omissions
            .iter()
            .any(|m| m.starts_with("fold 30 files behind")),
        "{omissions:?}"
    );
}

/// Ruling 4: no store is an honest absence.
#[test]
fn no_fold_is_an_honest_absence() {
    let dir = tree("fixture");
    let root = dir.path();
    let answer = ask(root, SHORT_WORDED);
    assert_eq!(unresolved(&answer), vec![NO_FOLD.to_string()]);
    assert!(paths(&answer).is_empty());
    assert_eq!(
        answer["ranking_known"], false,
        "an absent route is not a known ranking"
    );
    assert_eq!(
        answer["method"],
        json!(measure_cid(root)),
        "a method on every result"
    );
    assert!(answer["fold_lag"].is_null());
}

/// Ruling 4: an embedder that cannot run answers `unavailable: <reason>` with no candidates.
#[test]
fn an_unavailable_embedder_is_an_honest_absence() {
    use elohim_epr_cli::flow::memory::recall::embedder::PROCEDURE_REL;
    let dir = tree("pinned");
    let root = dir.path();
    fold(root);
    // A pinned store labelled by the declared procedure, whose model resolves nowhere.
    pinned_store_from_fixture(root, Some(&pinned_label()));
    let procedure = root.join(PROCEDURE_REL);
    std::fs::create_dir_all(procedure.parent().unwrap()).unwrap();
    std::fs::copy(common::repo_root().join(PROCEDURE_REL), &procedure).unwrap();

    let answer = ask(root, SHORT_WORDED);
    let lines = unresolved(&answer);
    assert_eq!(lines.len(), 1, "{answer}");
    assert!(
        lines[0].starts_with("semantic: unavailable: no model directory resolves"),
        "{lines:?}"
    );
    assert!(paths(&answer).is_empty());
    assert_eq!(answer["embedder"], "pinned");
    assert_eq!(answer["ranking_known"], false);
}

/// Minor 5: a store folded by another procedure is refused BEFORE the embedder runs. No
/// procedure exists in this tree and the model resolves nowhere, so an embedding attempt would
/// answer `unavailable`; the answer is the method refusal instead.
#[test]
fn a_store_folded_by_another_procedure_never_spawns_the_embedder() {
    use elohim_epr_cli::flow::memory::recall::embedder::PROCEDURE_REL;
    let dir = tree("pinned");
    let root = dir.path();
    fold(root);
    pinned_store_from_fixture(root, None); // still labelled `fixture`
    assert!(!root.join(PROCEDURE_REL).exists());
    let answer = ask(root, SHORT_WORDED);
    assert_eq!(unresolved(&answer), vec![OTHER_METHOD.to_string()]);
    assert_eq!(answer["ranking_known"], false);
    assert_eq!(
        answer["usage"]["provider_seconds"], 0,
        "nothing was embedded"
    );
}

/// Minor 4: the measure's CID is printed wherever the measure loaded, even when the declared
/// embedder is not one this executor knows.
#[test]
fn an_unknown_declared_embedder_still_names_the_method() {
    let dir = tree("oracle");
    let root = dir.path();
    let answer = ask(root, SHORT_WORDED);
    assert_eq!(answer["method"], json!(measure_cid(root)));
    let lines = unresolved(&answer);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("embedder `oracle`"), "{lines:?}");
}

/// Ruling 4: a store whose meta names another model is never silently used.
#[test]
fn a_fold_built_under_another_method_is_refused_for_refold() {
    let dir = tree("fixture");
    let root = dir.path();
    fold(root);
    let store = index::store_dir(root, &measure_cid(root), EmbedderChoice::Fixture);
    let conn = rusqlite::Connection::open(store.join("fold.sqlite")).unwrap();
    conn.execute(
        "UPDATE meta SET value = 'bafkreiothermodel' WHERE key = 'model'",
        [],
    )
    .unwrap();
    drop(conn);
    let answer = ask(root, SHORT_WORDED);
    assert_eq!(unresolved(&answer), vec![OTHER_METHOD.to_string()]);
    assert!(paths(&answer).is_empty());
    assert_eq!(answer["ranking_known"], false);
}

/// Minor 7: a folded file gone from the tree is named, never offered, and never mislabelled as
/// outside the source roots.
#[test]
fn a_folded_file_no_longer_present_is_named_not_offered() {
    let dir = tree("fixture");
    let root = dir.path();
    fold(root);
    std::fs::remove_file(root.join("genesis/triage.md")).unwrap();
    let answer = ask(root, SHORT_WORDED);
    assert!(!paths(&answer).contains(&"genesis/triage.md".to_string()));
    let named = omissions(&answer);
    assert!(
        named.contains(&"genesis/triage.md: folded file no longer present".to_string()),
        "{named:?}"
    );
    assert!(
        !named.iter().any(|m| m.contains("outside the recipe")),
        "{named:?}"
    );
    assert_eq!(answer["fold_lag"], 1);
}

/// A ranked file outside the recipe's declared source roots is passed over and counted.
#[test]
fn a_file_outside_the_source_roots_is_passed_over_and_counted() {
    let dir = tree("fixture");
    let root = dir.path();
    fold(root);
    let mut narrowed = live(CONTRACT_REL);
    narrowed["ceremony"]["providers"]["semantic"]["embedder"] = json!("fixture");
    narrowed["source_roots"] = json!(["genesis/", ".epr-meta/"]);
    put_json(root, CONTRACT_REL, &narrowed);
    let answer = ask(root, "household mesh");
    assert!(
        !paths(&answer).contains(&".claude/notes.md".to_string()),
        "{answer}"
    );
    let named = omissions(&answer);
    assert!(
        named
            .iter()
            .any(|m| m
                .starts_with("1 ranked file(s) lie outside the recipe's declared source roots")),
        "{named:?}"
    );
}

/// Minor 8: no silent empties. A scope holding no folded chunk says so; no candidate is ever
/// offered on a cosine at or below zero.
#[test]
fn an_empty_scope_says_so_and_no_candidate_scores_at_or_below_zero() {
    let dir = tree("fixture");
    let root = dir.path();
    fold(root);
    let empty = ask_in(root, SHORT_WORDED, "nowhere");
    assert!(paths(&empty).is_empty());
    assert!(
        omissions(&empty).contains(&"semantic: no folded chunks under nowhere".to_string()),
        "{empty}"
    );
    let unrelated = ask(root, "xyzzyquux");
    for candidate in unrelated["candidates"].as_array().unwrap() {
        assert!(candidate["score"].as_f64().unwrap() > 0.0, "{candidate}");
    }
}

/// Ruling 5: `--tag` is exact frontmatter membership, which only the local provider reads.
#[test]
fn a_tag_filter_is_refused_on_the_semantic_provider() {
    let dir = tree("fixture");
    let root = dir.path();
    fold(root);
    let refused = retrieve(
        root,
        &contract(root),
        "semantic",
        SHORT_WORDED,
        ".",
        &[],
        &["alpha".to_string()],
    )
    .expect_err("a tag the provider would ignore is refused");
    assert!(
        refused
            .to_string()
            .contains("only the local provider reads frontmatter"),
        "{refused}"
    );
}

/// One `epr flow memory recall` call in the `semantic-cli` session: (exit code, stdout).
fn cli(root: &Path, args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(["flow", "memory", "recall"])
        .args(args)
        .arg("--root")
        .arg(root)
        .args([
            "--contract",
            CONTRACT_REL,
            "--session",
            "semantic-cli",
            "--json",
        ])
        .output()
        .expect("epr runs");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    (
        out.status.code(),
        format!("{stdout}{}", String::from_utf8_lossy(&out.stderr)),
    )
}

/// Ruling 5: the CLI names the provider; the answer is an ordinary view.
#[test]
fn search_provider_semantic_answers_through_the_cli() {
    let dir = tree("fixture");
    let root = dir.path();
    fold(root);
    common::git(root, &["init", "-q"]);
    common::git(root, &["add", "-A"]);
    common::git(root, &["commit", "-qm", "fixture"]);
    let (code, opened) = cli(root, &["open", "--intent", "Find who fixes bugs"]);
    assert_eq!(code, Some(0), "{opened}");
    let (code, stdout) = cli(
        root,
        &[
            "search",
            "--provider",
            "semantic",
            "--query",
            SHORT_WORDED,
            "--search-scope",
            ".",
        ],
    );
    assert_eq!(code, Some(0), "{stdout}");
    let view: Value = serde_json::from_str(&stdout).expect("json");
    let retrieval = &view["retrieval"];
    // Each located candidate is a linked choice, and the command it carries runs.
    let reads: Vec<&Value> = view["actions"]
        .as_array()
        .expect("actions")
        .iter()
        .filter(|a| {
            a["argv"]
                .as_array()
                .is_some_and(|argv| argv.iter().any(|w| w == "read"))
        })
        .collect();
    assert_eq!(
        reads.len(),
        retrieval["candidates"].as_array().unwrap().len(),
        "{view}"
    );
    assert_eq!(
        reads[0]["label"],
        "Read genesis/triage.md — Who fixes it (3:4)"
    );
    let argv: Vec<String> = reads[0]["argv"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w.as_str().unwrap().to_string())
        .collect();
    assert_eq!(argv[0], "epr");
    let lines_at = argv.iter().position(|w| w == "--lines").expect("--lines");
    assert_eq!(argv[lines_at + 1], "3:4");
    let path_at = argv.iter().position(|w| w == "--path").expect("--path");
    assert_eq!(argv[path_at + 1], "genesis/triage.md");
    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(&argv[1..])
        .current_dir(root)
        .output()
        .expect("the linked read runs");
    let read = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{read}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(read.contains("Who can fix the bug now"), "{read}");
    assert_eq!(retrieval["provider"], "semantic");
    assert_eq!(retrieval["candidates"][0]["path"], "genesis/triage.md");
    assert_eq!(
        retrieval["candidates"][0]["method"],
        json!(measure_cid(root))
    );
    assert!(view["usage"]["semantic_chunks_scanned"].as_u64().unwrap() > 0);
}

/// Station 4 final review, ruling I4: the question bank is the exam sheet, never an answer — on
/// any screen a provider feeds, not only the fused first screen. A semantic hit on the file the
/// contract's `question_bank` names never appears in `search --provider semantic`; it is counted in
/// one omission line. The rule reads the contract: a copy whose `question_bank` names another file
/// offers this one (the precondition that it would rank at all).
#[test]
fn search_provider_semantic_never_offers_the_question_bank() {
    let dir = tree("fixture");
    let root = dir.path();
    let bank = contract(root).value["question_bank"]
        .as_str()
        .expect("the live contract names its bank")
        .to_string();
    let mut measure = live(MEASURE_REL);
    let mut surface: Vec<Value> = SURFACE.iter().map(|p| json!(p)).collect();
    surface.push(json!(bank));
    measure["surfaces"]["paths"] = json!(surface);
    put_json(root, MEASURE_REL, &measure);
    common::write(root, &bank, "{\"question\": \"who can fix a bug now\"}\n");
    fold(root);
    common::git(root, &["init", "-q"]);
    common::git(root, &["add", "-A"]);
    common::git(root, &["commit", "-qm", "fixture"]);

    let mut elsewhere = contract(root).value;
    elsewhere["question_bank"] = json!("genesis/exam.json");
    let elsewhere = Contract::from_value(elsewhere).expect("a valid contract copy");
    let offered = retrieve(root, &elsewhere, "semantic", SHORT_WORDED, ".", &[], &[])
        .expect("an honest answer");
    assert!(
        paths(&offered).contains(&bank),
        "precondition: the semantic route ranks the bank when it is not the bank: {offered}"
    );

    let (code, opened) = cli(root, &["open", "--intent", "Find who fixes bugs"]);
    assert_eq!(code, Some(0), "{opened}");
    let (code, stdout) = cli(
        root,
        &[
            "search",
            "--provider",
            "semantic",
            "--query",
            SHORT_WORDED,
            "--search-scope",
            ".",
        ],
    );
    assert_eq!(code, Some(0), "{stdout}");
    let view: Value = serde_json::from_str(&stdout).expect("json");
    let retrieval = &view["retrieval"];
    assert_eq!(retrieval["ranking_known"], true, "{view}");
    assert!(!paths(retrieval).contains(&bank), "{view}");
    assert!(
        !stdout.contains(&format!("--path {bank}")) && !stdout.contains(&format!("\"{bank}\"")),
        "the bank is named nowhere in the answer: {stdout}"
    );
    assert!(
        omissions(retrieval).contains(&"1 non-authority hit(s) withheld".to_string()),
        "{view}"
    );
}

/// Station 4 final review, minor m1: the answer names the exact embedding procedure its ranking
/// rests on — `procedure` (the CID the model manifest pins) and `built_by` (the label the store
/// was folded under) — so a candidate list, and a `recall-bank-reach@1` fold that cites it, can
/// say which procedure produced the vectors. The fixture names no procedure.
#[test]
fn the_answer_names_its_embedding_procedure() {
    use elohim_epr_cli::flow::memory::recall::embedder::{MODEL_MANIFEST_REL, PROCEDURE_REL};
    let dir = tree("fixture");
    let root = dir.path();
    fold(root);
    let answer = ask(root, SHORT_WORDED);
    assert_eq!(answer["ranking_known"], true, "{answer}");
    assert_eq!(answer["built_by"], "fixture", "{answer}");
    assert_eq!(answer["procedure"], Value::Null, "{answer}");

    let dir = tree("pinned");
    let root = dir.path();
    fold(root);
    pinned_store_from_fixture(root, Some(&pinned_label()));
    let procedure = root.join(PROCEDURE_REL);
    std::fs::create_dir_all(procedure.parent().unwrap()).unwrap();
    std::fs::copy(common::repo_root().join(PROCEDURE_REL), &procedure).unwrap();
    let answer = ask(root, SHORT_WORDED);
    let pinned = live(MODEL_MANIFEST_REL)["procedure"].clone();
    assert!(pinned.is_string(), "the live manifest pins a procedure");
    assert_eq!(answer["procedure"], pinned, "{answer}");
    assert_eq!(answer["built_by"], json!(pinned_label()), "{answer}");
}
