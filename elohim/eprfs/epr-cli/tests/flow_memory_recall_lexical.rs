//! Governed-discovery station 4, task 4.8: the `lexical` provider ranks by FTS5 `bm25()` over the
//! fold it shares with the semantic route.
//!
//! `search --provider lexical` matches the question's terms — each stemmed and quoted as an FTS5
//! prefix string, so an inflection matches and no question text is ever FTS5 syntax — against every
//! live chunk in the semantic measure's store, keeps each file's best chunk and returns the top
//! `limits.search_results` files. Every candidate prints `producer: lexical`, the lexical
//! `IndexMeasure` CID as its method and the fold's lag. Absence is honest, never an error: no fold,
//! or a fold built under another method, each answer with one `unresolved` line and no candidates;
//! a stale fold answers and says so. It never runs an embedder.
//!
//! Every test folds a temporary tree with the `Fixture` embedder and reads the real SQLite store.
//! The contract is a COPY of the live one, re-declaring `ceremony.providers.lexical` to read the
//! fixture store (the live declaration, contract v22, reads the pinned one).
mod common;

use std::path::Path;
use std::process::Command;

use elohim_epr_cli::flow::memory::recall::index::{self, EmbedderChoice, FoldOptions, FoldRun};
use elohim_epr_cli::flow::memory::recall::{retrieve, Contract, CONTRACT_REL};
use elohim_epr_rea::IndexMeasure;
use serde_json::{json, Value};
use tempfile::TempDir;

const FOLD_REL: &str = ".epr-meta/elohim/algorithms/recall-semantic-index.json";
const LEXICAL_REL: &str = ".epr-meta/elohim/algorithms/recall-lexical-index.json";
const NO_FOLD: &str = "lexical: no fold — run epr flow memory index fold";
const OTHER_METHOD: &str = "lexical: the fold was built under another method — refold";

/// The fixture's own surface — declared identically by both measures, as the live pair is.
const SURFACE: [&str; 5] = [
    "genesis/**/*.md",
    "genesis/**/*.py",
    "genesis/**/*.sh",
    "genesis/**/*.yaml",
    ".claude/**/*.md",
];

fn live(rel: &str) -> Value {
    let raw = std::fs::read(common::repo_root().join(rel)).expect("live file reads");
    serde_json::from_slice(&raw).expect("live file parses")
}

fn put_json(root: &Path, rel: &str, value: &Value) {
    common::write(root, rel, &serde_json::to_string_pretty(value).unwrap());
}

/// The lexical declaration a contract copy carries, reading the store `embedder` folded.
fn declaration(embedder: &str) -> Value {
    json!({"kind": "lexical", "measure": LEXICAL_REL, "fold": FOLD_REL, "embedder": embedder,
           "optional": true})
}

/// A temporary repository: a copy of the live contract declaring the lexical provider (the
/// semantic one reading the fixture store), fixture-surface copies of both measures, and a tree.
fn tree() -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let mut contract = live(CONTRACT_REL);
    contract["ceremony"]["providers"]["semantic"]["embedder"] = json!("fixture");
    contract["ceremony"]["providers"]["lexical"] = declaration("fixture");
    put_json(root, CONTRACT_REL, &contract);
    for rel in [FOLD_REL, LEXICAL_REL] {
        let mut measure = live(rel);
        measure["surfaces"]["paths"] = json!(SURFACE);
        put_json(root, rel, &measure);
    }
    // One chunk carries all three terms once; each other file repeats one term.
    common::write(
        root,
        "genesis/all.md",
        "# Intro\nNothing in particular.\n## Keeping\nThe stewardship ledger keeps the commons.\n",
    );
    common::write(
        root,
        "genesis/steward.md",
        "# Care\nStewardship, stewardship, stewardship of the land.\n",
    );
    common::write(
        root,
        "genesis/ledger.md",
        "# Accounts\nA ledger, a ledger, a ledger of accounts.\n",
    );
    common::write(
        root,
        "genesis/commons.md",
        "# Shared\nThe commons, the commons, the commons again.\n",
    );
    // Only inflections: `folded`, `stamped` — never `folds`, `stamping`.
    common::write(
        root,
        "genesis/index.md",
        "# Index\nThe index was folded twice and stamped.\n",
    );
    common::write(root, "genesis/sky.md", "# Sky\nquasar nebula pulsar\n");
    common::write(
        root,
        "genesis/tool.py",
        "import os\n\ndef orbit_launch():\n    \"\"\"launch the orbit daemon\"\"\"\n    return 1\n",
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
    retrieve(root, &contract(root), "lexical", query, scope, &[], &[])
        .expect("an honest answer, never an error")
}

fn ask(root: &Path, query: &str) -> Value {
    ask_in(root, query, ".")
}

fn cid_of(root: &Path, rel: &str) -> String {
    let measure: IndexMeasure = serde_json::from_value(
        serde_json::from_slice(&std::fs::read(root.join(rel)).unwrap()).unwrap(),
    )
    .unwrap();
    measure.cid().unwrap().to_string()
}

fn paths(candidates: &Value) -> Vec<String> {
    candidates
        .as_array()
        .expect("candidates")
        .iter()
        .map(|c| c["path"].as_str().unwrap_or_default().to_string())
        .collect()
}

fn lines(value: &Value) -> Vec<String> {
    value
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|m| m.as_str().unwrap_or_default().to_string())
        .collect()
}

/// The live measure pair: the lexical measure ranks by bm25, pins no model, and declares the
/// fold measure's own chunk rule and surfaces (it shares that fold).
#[test]
fn the_lexical_measure_shares_the_semantic_fold_and_pins_no_model() {
    let lexical = live(LEXICAL_REL);
    let fold = live(FOLD_REL);
    let measure: IndexMeasure = serde_json::from_value(lexical.clone()).expect("an IndexMeasure");
    measure.validate().expect("the declaration validates");
    assert_eq!(
        lexical["measure"],
        json!({"id": "recall-lexical-index", "version": 1})
    );
    assert_eq!(lexical["ranking"], json!({"method": "bm25"}));
    assert!(lexical.get("embedding").is_none(), "no model pin");
    assert!(
        lexical.get("_fold").is_none(),
        "the contract's providers.<key>.fold is the fold reference's one home"
    );
    for key in [
        "chunkRule",
        "_chunk_rule",
        "surfaces",
        "retention",
        "foldLag",
    ] {
        assert_eq!(lexical[key], fold[key], "{key} is the fold measure's");
    }
    // A pin nothing uses is refused.
    let mut pinned = lexical.clone();
    pinned["embedding"] = fold["embedding"].clone();
    let pinned: IndexMeasure = serde_json::from_value(pinned).unwrap();
    assert!(pinned.validate().is_err());
}

/// Step 3: a multi-term question ranks the chunk carrying all its terms first — above files that
/// repeat one term — and the candidate lands on that chunk's passage.
#[test]
fn a_multi_term_question_ranks_the_chunk_carrying_every_term_first() {
    let dir = tree();
    let root = dir.path();
    fold(root);
    let answer = ask(root, "stewardship ledger commons");
    assert_eq!(answer["ranking_known"], true, "{answer}");
    assert!(lines(&answer["unresolved"]).is_empty(), "{answer}");
    let found = paths(&answer["candidates"]);
    assert_eq!(
        found.first().map(String::as_str),
        Some("genesis/all.md"),
        "{answer}"
    );
    assert_eq!(found.len(), 3, "limits.search_results: {answer}");
    let first = &answer["candidates"][0];
    assert_eq!(first["best_section"]["title"], "Keeping", "{first}");
    assert_eq!(first["best_section"]["lines"], "3:4", "{first}");
    let scores: Vec<f64> = answer["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["score"].as_f64().unwrap())
        .collect();
    assert!(
        scores.windows(2).all(|w| w[0] >= w[1]),
        "higher first: {scores:?}"
    );
    assert!(
        scores.iter().all(|s| *s > 0.0),
        "score is -bm25(): {scores:?}"
    );
}

/// Step 3: an inflection matches through the executor's stem plus an FTS5 prefix — `folds` and
/// `stamping` find a file that only says `folded` and `stamped`.
#[test]
fn an_inflection_matches_through_stem_and_prefix() {
    let dir = tree();
    let root = dir.path();
    fold(root);
    let answer = ask(root, "folds stamping");
    // The behaviour first: neither `folds` nor `stamping` occurs in the tree.
    assert_eq!(
        paths(&answer["candidates"]),
        vec!["genesis/index.md"],
        "{answer}"
    );
    assert_eq!(answer["match"], "\"fold\"* OR \"stamp\"*", "{answer}");
}

/// A declared short term (`top`, `red`) and the phrase of them match EXACTLY: a file that only
/// says `reduce topology` is not a `top red` match, however often it says it, and the file that
/// carries `top red habit` ranks first.
#[test]
fn a_short_declared_term_matches_exactly_never_as_a_prefix() {
    let dir = tree();
    let root = dir.path();
    common::write(
        root,
        "genesis/topology.md",
        "# Shape\nWe reduce topology; reduce the topology again, and reduce it once more.\n",
    );
    common::write(
        root,
        "genesis/register.md",
        "# Register\nThe top red habit.\n",
    );
    fold(root);
    let answer = ask(root, "Which habit is top red right now?");
    assert_eq!(answer["ranking_known"], true, "{answer}");
    let found = paths(&answer["candidates"]);
    assert_eq!(
        found.first().map(String::as_str),
        Some("genesis/register.md"),
        "{answer}"
    );
    assert!(
        !found.contains(&"genesis/topology.md".to_string()),
        "`top`/`red` are not prefixes of `topology`/`reduce`: {answer}"
    );
    let expression = answer["match"].as_str().unwrap();
    assert!(
        expression.contains("\"top red\"") && !expression.contains("\"top\"*"),
        "{expression}"
    );
}

/// FTS5 syntax typed into the question is inert: every term is a quoted string, so `NEAR(`, `OR`,
/// `*`, `^`, a column filter and a stray quote answer as words, never as an error or an operator.
#[test]
fn fts_syntax_in_the_question_is_inert() {
    let dir = tree();
    let root = dir.path();
    fold(root);
    let answer = ask(
        root,
        "NEAR(quasar pulsar, 2) OR \"nebula\" * AND NOT text:zephyr ^orbit \"unclosed",
    );
    assert_eq!(answer["ranking_known"], true, "{answer}");
    assert!(lines(&answer["unresolved"]).is_empty(), "{answer}");
    let expression = answer["match"].as_str().unwrap();
    for part in expression.split(" OR ") {
        assert!(
            part.starts_with('"') && part.ends_with("\"*"),
            "every term is one quoted prefix string: {expression}"
        );
    }
    let found = paths(&answer["candidates"]);
    assert_eq!(
        found.first().map(String::as_str),
        Some("genesis/sky.md"),
        "{answer}"
    );
    assert!(
        found.contains(&"genesis/tool.py".to_string()),
        "`orbit` is a word: {answer}"
    );
}

/// The characters a question term KEEPS (`-`, `.`, `/`; `question_terms` drops the rest) are FTS5
/// syntax errors in a bareword. Quoted, each term is a phrase of its tokens: the question answers
/// and ranks the file carrying them, never an unreadable-fold line.
#[test]
fn the_punctuation_a_term_keeps_is_inert() {
    let dir = tree();
    let root = dir.path();
    common::write(
        root,
        "genesis/hooks.md",
        "# Binary\nThe hook-binary resolver lives in _observation.py beside recall/lexical.\n",
    );
    fold(root);
    let answer = ask(root, "hook-binary _observation.py recall/lexical v0.7.0");
    assert_eq!(answer["ranking_known"], true, "{answer}");
    assert!(lines(&answer["unresolved"]).is_empty(), "{answer}");
    assert_eq!(
        paths(&answer["candidates"]).first().map(String::as_str),
        Some("genesis/hooks.md"),
        "{answer}"
    );
}

/// Every candidate prints its producer, the lexical measure's CID as its method, and the fold's
/// lag; the answer names the fold it read (the semantic measure's CID).
#[test]
fn every_candidate_prints_the_lexical_method_cid() {
    let dir = tree();
    let root = dir.path();
    fold(root);
    let method = cid_of(root, LEXICAL_REL);
    assert_ne!(
        method,
        cid_of(root, FOLD_REL),
        "its own method, not the fold's"
    );
    let answer = ask(root, "stewardship ledger commons orbit");
    assert_eq!(answer["method"], method.as_str());
    assert_eq!(answer["fold"], cid_of(root, FOLD_REL).as_str());
    assert_eq!(answer["fold_lag"], 0);
    let candidates = answer["candidates"].as_array().unwrap();
    assert!(!candidates.is_empty(), "{answer}");
    for candidate in candidates {
        assert_eq!(candidate["producer"], "lexical", "{candidate}");
        assert_eq!(candidate["method"], method.as_str(), "{candidate}");
        assert_eq!(candidate["fold_lag"], 0, "{candidate}");
        assert!(
            candidate["best_section"]["lines"].is_string(),
            "{candidate}"
        );
    }
    let usage = &answer["usage"];
    assert!(
        usage["lexical_chunks_matched"].as_u64().unwrap() > 0,
        "{usage}"
    );
    assert!(usage["lexical_query_ms"].is_u64(), "{usage}");
    assert!(
        usage["lexical_lag_ms"].is_u64(),
        "the lag walk is timed apart: {usage}"
    );
    assert!(usage.get("source_bytes").is_none(), "{usage}");
    assert!(usage.get("embedding_processes").is_none(), "{usage}");
}

/// No store yet: one unresolved line naming the fold command, no candidates, an unknown ranking —
/// and the method is still named, because the measure loaded.
#[test]
fn no_fold_is_one_unresolved_line() {
    let dir = tree();
    let root = dir.path();
    let answer = ask(root, "stewardship ledger");
    assert_eq!(answer["ranking_known"], false);
    assert_eq!(lines(&answer["unresolved"]), vec![NO_FOLD.to_string()]);
    assert!(paths(&answer["candidates"]).is_empty());
    assert_eq!(answer["method"], cid_of(root, LEXICAL_REL).as_str());
}

/// A lexical measure whose surfaces are not the fold's is another method, and so is a store whose
/// meta names another chunk rule: each is one unresolved line, never a ranking.
#[test]
fn a_fold_under_another_method_is_refused() {
    let dir = tree();
    let root = dir.path();
    fold(root);

    let mut measure = live(LEXICAL_REL);
    measure["surfaces"]["paths"] = json!(["genesis/**/*.md"]);
    put_json(root, LEXICAL_REL, &measure);
    let answer = ask(root, "stewardship ledger");
    assert_eq!(answer["ranking_known"], false, "{answer}");
    assert_eq!(lines(&answer["unresolved"]), vec![OTHER_METHOD.to_string()]);

    let mut measure = live(LEXICAL_REL);
    measure["surfaces"]["paths"] = json!(SURFACE);
    put_json(root, LEXICAL_REL, &measure);
    assert_eq!(ask(root, "stewardship ledger")["ranking_known"], true);

    let store = index::store_dir(root, &cid_of(root, FOLD_REL), EmbedderChoice::Fixture)
        .join("fold.sqlite");
    let conn = rusqlite::Connection::open(&store).unwrap();
    conn.execute(
        "UPDATE meta SET value = 'another' WHERE key = 'chunk_rule'",
        [],
    )
    .unwrap();
    drop(conn);
    let answer = ask(root, "stewardship ledger");
    assert_eq!(answer["ranking_known"], false, "{answer}");
    assert_eq!(lines(&answer["unresolved"]), vec![OTHER_METHOD.to_string()]);
}

/// The chunk rule is part of the method: a lexical measure cutting by another rule — its
/// `_chunk_rule` changed and `chunkRule` re-hashed to match, so the declaration itself is sound —
/// does not share a fold cut under the semantic measure's rule.
#[test]
fn a_lexical_measure_under_another_chunk_rule_is_refused() {
    let dir = tree();
    let root = dir.path();
    fold(root);
    let mut measure: Value =
        serde_json::from_slice(&std::fs::read(root.join(LEXICAL_REL)).unwrap()).unwrap();
    measure["_chunk_rule"]["max_chunks_per_file"] = json!(12);
    measure["chunkRule"] = json!(elohim_epr_rea::atom_cid(&measure["_chunk_rule"])
        .unwrap()
        .to_string());
    put_json(root, LEXICAL_REL, &measure);
    let typed: IndexMeasure = serde_json::from_value(measure).unwrap();
    typed.validate().expect("precondition: a sound declaration");
    let answer = ask(root, "stewardship ledger");
    assert_eq!(answer["ranking_known"], false, "{answer}");
    assert_eq!(lines(&answer["unresolved"]), vec![OTHER_METHOD.to_string()]);
    assert_eq!(
        answer["method"],
        cid_of(root, LEXICAL_REL).as_str(),
        "the method it declared is still named"
    );
}

/// A stale fold answers, and says how stale.
#[test]
fn a_stale_fold_answers_and_names_its_lag() {
    let dir = tree();
    let root = dir.path();
    fold(root);
    common::write(
        root,
        "genesis/late.md",
        "# Late\nA stewardship ledger, late.\n",
    );
    let answer = ask(root, "stewardship ledger commons");
    assert_eq!(answer["ranking_known"], true, "{answer}");
    assert_eq!(answer["fold_lag"], 1, "{answer}");
    assert!(
        lines(&answer["omissions"])
            .iter()
            .any(|line| line.starts_with("fold 1 files behind")),
        "{answer}"
    );
    assert!(
        !paths(&answer["candidates"]).contains(&"genesis/late.md".to_string()),
        "an unfolded file is not in the fold: {answer}"
    );
    assert_eq!(answer["candidates"][0]["fold_lag"], 1);
}

/// BM25 ranks LIVE chunks only: a file the next fold demoted (removed from the tree) is never
/// ranked — not ranked and then passed over as "no longer present", simply not in the ranking.
#[test]
fn a_demoted_chunk_never_ranks() {
    let dir = tree();
    let root = dir.path();
    fold(root);
    std::fs::remove_file(root.join("genesis/steward.md")).unwrap();
    fold(root);
    let answer = ask(root, "stewardship");
    assert_eq!(answer["ranking_known"], true, "{answer}");
    assert_eq!(answer["fold_lag"], 0, "{answer}");
    assert_eq!(
        paths(&answer["candidates"]),
        vec!["genesis/all.md"],
        "{answer}"
    );
    assert!(
        !lines(&answer["omissions"])
            .iter()
            .any(|line| line.contains("genesis/steward.md")),
        "{answer}"
    );
}

/// The search scope is a query predicate: only chunks under it rank.
#[test]
fn the_search_scope_bounds_the_ranking() {
    let dir = tree();
    let root = dir.path();
    fold(root);
    let answer = ask_in(root, "household mesh orbit", ".claude");
    assert_eq!(
        paths(&answer["candidates"]),
        vec![".claude/notes.md"],
        "{answer}"
    );
}

/// `epr flow memory recall <args> --root <root> --contract <contract_rel> --session <session>`
/// with `env` set; (exit code, stdout, stderr).
fn cli_with(
    root: &Path,
    contract_rel: &str,
    session: &str,
    args: &[&str],
    env: &[(&str, &Path)],
) -> (Option<i32>, String, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_epr"));
    command
        .args(["flow", "memory", "recall"])
        .args(args)
        .arg("--root")
        .arg(root)
        .args(["--contract", contract_rel, "--session", session]);
    for (key, value) in env {
        command.env(key, value);
    }
    let out = command.output().expect("epr runs");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// It never spawns an embedding process. Over a PINNED store labelled as the declared
/// procedure's, a model directory that resolves and an interpreter that leaves a mark each time
/// it runs, `search --provider lexical` answers and leaves no mark — and the control, the semantic
/// route over the same store, does leave one, so the harness would have seen a spawn.
#[test]
fn the_lexical_route_never_spawns_an_embedding_process() {
    use elohim_epr_cli::flow::memory::recall::embedder::{
        INTERPRETER_ENV, MODEL_MANIFEST_REL, PROCEDURE_REL,
    };
    use std::os::unix::fs::PermissionsExt;
    let dir = tree();
    let root = dir.path();
    commit(root);
    fold(root);
    // The fixture store, copied to the pinned store and labelled as the declared procedure's.
    let cid = cid_of(root, FOLD_REL);
    let pinned = index::store_dir(root, &cid, EmbedderChoice::Pinned);
    std::fs::create_dir_all(&pinned).unwrap();
    std::fs::copy(
        index::store_dir(root, &cid, EmbedderChoice::Fixture).join("fold.sqlite"),
        pinned.join("fold.sqlite"),
    )
    .unwrap();
    let mut manifest = live(MODEL_MANIFEST_REL);
    let label = format!(
        "procedure {} on model {}",
        manifest["procedure"].as_str().unwrap(),
        manifest["model_bytes"].as_str().unwrap()
    );
    let conn = rusqlite::Connection::open(pinned.join("fold.sqlite")).unwrap();
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'embedder'",
        [&label],
    )
    .unwrap();
    drop(conn);
    std::fs::create_dir_all(root.join("model")).unwrap();
    manifest["resolve"] = json!([root.join("model").to_string_lossy()]);
    put_json(root, MODEL_MANIFEST_REL, &manifest);
    let procedure = root.join(PROCEDURE_REL);
    std::fs::create_dir_all(procedure.parent().unwrap()).unwrap();
    std::fs::copy(common::repo_root().join(PROCEDURE_REL), &procedure).unwrap();
    let marker = root.join("spawned.log");
    let interpreter = root.join("interpreter.sh");
    std::fs::write(
        &interpreter,
        format!(
            "#!/bin/sh\necho spawned >> '{}'\nexit 1\n",
            marker.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&interpreter, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut value = contract(root).value;
    value["ceremony"]["providers"]["lexical"] = declaration("pinned");
    value["ceremony"]["providers"]["semantic"]["embedder"] = json!("pinned");
    put_json(root, CONTRACT_REL, &value);
    let env = [(INTERPRETER_ENV, interpreter.as_path())];

    let (code, stdout, stderr) = cli_with(
        root,
        CONTRACT_REL,
        "spawn",
        &["open", "--intent", "Find the stewardship ledger"],
        &env,
    );
    assert_eq!(code, Some(0), "{stdout}{stderr}");
    let search = |provider: &str| {
        cli_with(
            root,
            CONTRACT_REL,
            "spawn",
            &[
                "search",
                "--provider",
                provider,
                "--query",
                "stewardship ledger commons",
                "--search-scope",
                ".",
                "--json",
            ],
            &env,
        )
    };
    let (code, stdout, stderr) = search("lexical");
    assert_eq!(code, Some(0), "{stdout}{stderr}");
    assert!(
        !marker.exists(),
        "the lexical route spawned an embedding process"
    );
    let view: Value = serde_json::from_str(&stdout).expect("json");
    let retrieval = &view["retrieval"];
    assert_eq!(retrieval["ranking_known"], true, "{view}");
    assert_eq!(retrieval["embedder"], "pinned", "{view}");
    assert_eq!(
        paths(&retrieval["candidates"]).first().map(String::as_str),
        Some("genesis/all.md"),
        "{view}"
    );

    // The control: the semantic route over the same store does embed the question.
    let _ = search("semantic");
    assert!(marker.exists(), "the control asked the pinned embedder");
}

/// `--tag` is frontmatter membership, which only the local route reads: refused before anything
/// runs, as on the semantic route.
#[test]
fn a_tag_filter_is_refused_on_the_lexical_route() {
    let dir = tree();
    let root = dir.path();
    let refused = retrieve(
        root,
        &contract(root),
        "lexical",
        "stewardship",
        ".",
        &[],
        &["memory".to_string()],
    );
    assert!(refused.is_err());
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Three-producer fusion on the focused first screen
// ───────────────────────────────────────────────────────────────────────────────────────────────

fn commit(root: &Path) {
    common::git(root, &["init", "-q"]);
    common::git(root, &["add", "-A"]);
    common::git(root, &["commit", "-qm", "fixture"]);
}

fn open_json_under(root: &Path, contract_rel: &str, session: &str, extra: &[&str]) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(["flow", "memory", "recall", "open", "--json"])
        .args(extra)
        .arg("--root")
        .arg(root)
        .args(["--contract", contract_rel, "--session", session])
        .output()
        .expect("epr runs");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_str(&stdout).expect("json")
}

/// A recipe naming `lexical` as a third producer fuses its order for ORDER only: its method is
/// named beside the recipe, each candidate carries its lexical rank, the call is charged as part of
/// the screen (never `search_queries`), and the generated habit register a lexical hit finds is
/// never offered.
#[test]
fn a_three_producer_recipe_fuses_the_lexical_order() {
    let dir = tree();
    let root = dir.path();
    common::write(
        root,
        "genesis/manifests/habits.yaml",
        "habits:\n- id: orbit\n  status: red\n  invariant: the orbit launch daemon\n",
    );
    commit(root);
    fold(root);
    let need = "orbit launch daemon";
    let lexical = ask_in(root, need, "genesis");
    // Ruling I4 (final review): the provider itself withholds the register, before any fusion.
    assert!(
        !paths(&lexical["candidates"]).contains(&"genesis/manifests/habits.yaml".to_string())
            && lines(&lexical["omissions"])
                .contains(&"1 non-authority hit(s) withheld".to_string()),
        "precondition: the lexical route found the register and withheld it: {lexical}"
    );

    let mut value = contract(root).value;
    value["discovery"]["first_screen_fusion"]["producers"] =
        json!(["local", "semantic", "lexical"]);
    put_json(root, "three-producers.json", &value);
    let view = open_json_under(
        root,
        "three-producers.json",
        "three",
        &["--need", need, "--scope", "genesis"],
    );
    let screen = &view["first_screen"];
    assert_eq!(screen["fusion"]["recipe"], "rrf-v1", "{screen}");
    let producers: Vec<Value> = screen["fusion"]["producers"].as_array().unwrap().clone();
    assert_eq!(
        producers.last().unwrap(),
        &json!({"id": "lexical", "method": cid_of(root, LEXICAL_REL)}),
        "{screen}"
    );
    let found = paths(&screen["candidates"]);
    assert!(
        !found.contains(&"genesis/manifests/habits.yaml".to_string()),
        "{screen}"
    );
    let tool = screen["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["path"] == "genesis/tool.py")
        .unwrap_or_else(|| panic!("the orbit tool is on the screen: {screen}"));
    assert_eq!(tool["ranks"]["lexical"], 1, "{tool}");
    let usage = &view["usage"];
    assert_eq!(usage["first_screen_lexical_calls"], 1, "{usage}");
    assert!(
        usage["lexical_chunks_matched"].as_u64().unwrap() > 0,
        "{usage}"
    );
    assert!(
        !lines(&screen["omissions"])
            .iter()
            .any(|line| line.starts_with("lexical:")),
        "{screen}"
    );
}

/// An absent lexical route (no fold) is one omission line on the fused screen and charges nothing.
#[test]
fn an_absent_lexical_route_is_one_omission_on_the_screen() {
    let dir = tree();
    let root = dir.path();
    commit(root);
    let mut value = contract(root).value;
    value["discovery"]["first_screen_fusion"]["producers"] =
        json!(["local", "semantic", "lexical"]);
    put_json(root, "three-producers.json", &value);
    let view = open_json_under(
        root,
        "three-producers.json",
        "absent",
        &["--need", "orbit launch daemon", "--scope", "genesis"],
    );
    let screen = &view["first_screen"];
    let lexical: Vec<String> = lines(&screen["omissions"])
        .into_iter()
        .filter(|line| line.starts_with("lexical:"))
        .collect();
    assert_eq!(lexical, vec![NO_FOLD.to_string()], "{screen}");
    assert!(view["usage"].get("first_screen_lexical_calls").is_none());
}

/// The v21 contract's method CID (task 4.6); the integration declared `providers.lexical`, so the
/// contract's bytes — and its address — moved off this one.
const V21_METHOD_CID: &str = "bafkreic2grd6prwzdgcth45c7bf2henoiinyhn3jwgba6kte3326vr3jfu";

/// Station 4 integration (Task 4.8 ruling): contract v22 DECLARES the lexical provider, keeps it
/// out of the first screen's fusion recipe (the bank evidence decided), says so in its method
/// prose, and the bank is re-pinned.
#[test]
fn contract_v22_declares_lexical_outside_the_fusion_recipe() {
    let root = common::repo_root();
    let contract = Contract::load(&root.join(CONTRACT_REL)).expect("live contract loads");
    let value = common::live_contract();
    assert!(
        value["version"].as_u64() >= Some(22),
        "{}",
        value["version"]
    );
    assert_eq!(
        value["ceremony"]["providers"]["lexical"],
        json!({"kind": "lexical", "measure": LEXICAL_REL, "fold": FOLD_REL, "embedder": "pinned",
               "ranking": "FTS5 bm25 over the shared fold's live chunks; short declared terms \
                           exact, longer terms stem-prefixed",
               "optional": true})
    );
    assert_eq!(
        value["discovery"]["first_screen_fusion"]["producers"],
        json!(["local", "semantic"]),
        "lexical is declared, not fused"
    );
    // v23 (final review): the prose is state-free — it says what the recipe holds, and that ranks
    // fuse for order only with no score combined.
    let method_lines = value["method"].to_string();
    assert!(
        method_lines.contains("search --provider lexical")
            && method_lines.contains("the first screen's fusion recipe does not include it")
            && method_lines.contains("no scores are combined")
            && !method_lines.contains("may combine semantic and lexical scores"),
        "{method_lines}"
    );
    let method = contract.method_cid();
    assert_ne!(method, V21_METHOD_CID, "the contract's bytes moved");
    let bank = live(value["question_bank"].as_str().expect("bank"));
    assert_eq!(bank["recipe"].as_str(), Some(method.as_str()));
    contract
        .question_bank()
        .expect("every question is in scope of the v22 recipe");
}

/// `search --provider lexical` answers under the LIVE declaration — the tree's contract is the
/// live one with only the store it reads flipped to the fixture embedder's; nothing is added.
#[test]
fn search_provider_lexical_answers_under_the_live_declaration() {
    let dir = tree();
    let root = dir.path();
    let mut value = live(CONTRACT_REL);
    value["ceremony"]["providers"]["semantic"]["embedder"] = json!("fixture");
    value["ceremony"]["providers"]
        .get_mut("lexical")
        .expect("the live contract declares the lexical provider")["embedder"] = json!("fixture");
    put_json(root, CONTRACT_REL, &value);
    commit(root);
    fold(root);
    let (code, stdout, stderr) = cli_with(
        root,
        CONTRACT_REL,
        "live-lexical",
        &["open", "--intent", "Find the stewardship ledger"],
        &[],
    );
    assert_eq!(code, Some(0), "{stdout}{stderr}");
    let (code, stdout, stderr) = cli_with(
        root,
        CONTRACT_REL,
        "live-lexical",
        &[
            "search",
            "--provider",
            "lexical",
            "--query",
            "stewardship ledger commons",
            "--search-scope",
            ".",
            "--json",
        ],
        &[],
    );
    assert_eq!(code, Some(0), "{stdout}{stderr}");
    let view: Value = serde_json::from_str(&stdout).expect("json");
    let retrieval = &view["retrieval"];
    assert_eq!(retrieval["ranking_known"], true, "{view}");
    assert_eq!(
        paths(&retrieval["candidates"]).first().map(String::as_str),
        Some("genesis/all.md"),
        "{view}"
    );
    let first = &retrieval["candidates"][0];
    assert_eq!(first["producer"], "lexical", "{first}");
    assert_eq!(first["method"], json!(cid_of(root, LEXICAL_REL)), "{first}");
}

/// Station 4 final review, ruling I4: a lexical hit on the question bank never appears in
/// `search --provider lexical` — the same first-screen offer rule, applied inside the provider —
/// and is counted in one omission line. A contract copy naming another bank offers this file (the
/// precondition that it would rank at all).
#[test]
fn search_provider_lexical_never_offers_the_question_bank() {
    let dir = tree();
    let root = dir.path();
    let bank = contract(root).value["question_bank"]
        .as_str()
        .expect("the live contract names its bank")
        .to_string();
    let mut surface: Vec<Value> = SURFACE.iter().map(|p| json!(p)).collect();
    surface.push(json!(bank));
    for rel in [FOLD_REL, LEXICAL_REL] {
        let mut measure = live(rel);
        measure["surfaces"]["paths"] = json!(surface);
        put_json(root, rel, &measure);
    }
    common::write(
        root,
        &bank,
        "{\"question\": \"which quasar nebula pulsar is it\"}\n",
    );
    commit(root);
    fold(root);

    let mut elsewhere = contract(root).value;
    elsewhere["question_bank"] = json!("genesis/exam.json");
    let elsewhere = Contract::from_value(elsewhere).expect("a valid contract copy");
    let offered = retrieve(
        root,
        &elsewhere,
        "lexical",
        "quasar nebula pulsar",
        ".",
        &[],
        &[],
    )
    .expect("an honest answer");
    assert!(
        paths(&offered["candidates"]).contains(&bank),
        "precondition: the lexical route ranks the bank when it is not the bank: {offered}"
    );

    let (code, stdout, stderr) = cli_with(
        root,
        CONTRACT_REL,
        "bank",
        &["open", "--intent", "Find the sky notes"],
        &[],
    );
    assert_eq!(code, Some(0), "{stdout}{stderr}");
    let (code, stdout, stderr) = cli_with(
        root,
        CONTRACT_REL,
        "bank",
        &[
            "search",
            "--provider",
            "lexical",
            "--query",
            "quasar nebula pulsar",
            "--search-scope",
            ".",
            "--json",
        ],
        &[],
    );
    assert_eq!(code, Some(0), "{stdout}{stderr}");
    let view: Value = serde_json::from_str(&stdout).expect("json");
    let retrieval = &view["retrieval"];
    assert_eq!(retrieval["ranking_known"], true, "{view}");
    let found = paths(&retrieval["candidates"]);
    assert!(found.contains(&"genesis/sky.md".to_string()), "{view}");
    assert!(!found.contains(&bank), "{view}");
    assert!(
        lines(&retrieval["omissions"]).contains(&"1 non-authority hit(s) withheld".to_string()),
        "{view}"
    );
}
