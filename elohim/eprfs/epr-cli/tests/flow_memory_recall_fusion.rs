//! Governed-discovery station 4, task 4.5: the focused first screen fuses the lexical order with
//! the semantic provider's order by reciprocal rank — for ORDER only.
//!
//! The recipe is declared in the contract (`discovery.first_screen_fusion`), its method CID is the
//! object's `atom_cid`, and the honesty floor prints it. Each candidate keeps its producer ranks.
//! The semantic route is asked only when `--need` was typed, over the focused area (else the
//! session scope); an absent, unavailable or undeclared route is ONE omission line and the screen
//! stays the lexical screen — never an exit 2.
//!
//! Every test folds a temporary git tree with the `Fixture` embedder (a hashed bag of words) and
//! drives the shipped binary.
mod common;

use std::path::Path;
use std::process::Command;

use elohim_epr_cli::flow::memory::recall::index::{self, EmbedderChoice, FoldOptions, FoldRun};
use elohim_epr_cli::flow::memory::recall::{retrieve, Contract, CONTRACT_REL};
use elohim_epr_rea::{atom_cid, IndexMeasure};
use serde_json::{json, Value};
use tempfile::TempDir;

const MEASURE_REL: &str = ".epr-meta/elohim/algorithms/recall-semantic-index.json";
const NO_FOLD: &str = "semantic: no fold — run epr flow memory index fold";

/// The v18 contract's method CID (task 4.4); task 4.5 declared the fusion recipe, so the
/// contract's bytes — and its address — moved off this one.
const V18_METHOD_CID: &str = "bafkreifo5g3r6ymjtpbopdappjnc32426f3qhplza2cninotofelfdrt3e";

/// The v19 contract's method CID (task 4.5); its fix round declared that the fused first screen's
/// own semantic call does not consume the packet's explicit search, so the address moved again.
const V19_METHOD_CID: &str = "bafkreidf267h5ynctpq2lsh35yo3llfz53oi32pckie7bmqvqoqz2kbdi4";

/// The fixture's fold surface (the live measure's is the repository's authority layer).
const SURFACE: [&str; 4] = [
    "genesis/**/*.md",
    "genesis/**/*.py",
    "genesis/**/*.sh",
    ".claude/**/*.md",
];

/// Its only lexical term is `orbit` (every other word is under four characters), so the lexical
/// route finds the files that say `orbit` and the semantic route also finds the one that says
/// "who can fix the bug now".
const NEED: &str = "who can fix the orbit bug now";

fn live(rel: &str) -> Value {
    let raw = std::fs::read(common::repo_root().join(rel)).expect("live file reads");
    serde_json::from_slice(&raw).expect("live file parses")
}

fn put_json(root: &Path, rel: &str, value: &Value) {
    common::write(root, rel, &serde_json::to_string_pretty(value).unwrap());
}

/// A committed temporary repository: the live contract (its semantic provider reading the
/// fixture store), a fixture-surface copy of the live measure, and a small tree in which
/// `genesis/orbit.json` is lexical-only (JSON is not folded) and `genesis/triage.md` is
/// semantic-only (it never says `orbit`).
fn tree() -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let mut contract = live(CONTRACT_REL);
    contract["ceremony"]["providers"]["semantic"]["embedder"] = json!("fixture");
    put_json(root, CONTRACT_REL, &contract);
    let mut measure = live(MEASURE_REL);
    measure["surfaces"]["paths"] = json!(SURFACE);
    put_json(root, MEASURE_REL, &measure);
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
        "genesis/orbit.json",
        "{\"orbit\": {\"orbit\": \"orbit\", \"period\": \"orbit period\"}}\n",
    );
    common::write(
        root,
        "genesis/beta.md",
        "# Beta\nStewardship of the commons.\n",
    );
    common::write(root, ".claude/notes.md", "# Notes\nA household mesh.\n");
    commit(root);
    dir
}

fn commit(root: &Path) {
    common::git(root, &["init", "-q"]);
    common::git(root, &["add", "-A"]);
    common::git(root, &["commit", "-qm", "fixture"]);
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

/// One `epr flow memory recall` call: (exit code, stdout + stderr).
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

/// A focused open, as JSON; asserts it exits 0.
fn open_json(root: &Path, session: &str, extra: &[&str]) -> Value {
    open_json_under(root, CONTRACT_REL, session, extra)
}

fn open_json_under(root: &Path, contract_rel: &str, session: &str, extra: &[&str]) -> Value {
    let mut args = vec!["open", "--json"];
    args.extend_from_slice(extra);
    let (code, stdout, stderr) = cli_with(root, contract_rel, session, &args, &[]);
    assert_eq!(code, Some(0), "{stdout}{stderr}");
    serde_json::from_str(&stdout).expect("json")
}

fn open_text(root: &Path, session: &str, extra: &[&str]) -> String {
    let mut args = vec!["open"];
    args.extend_from_slice(extra);
    let (code, stdout, stderr) = cli_with(root, CONTRACT_REL, session, &args, &[]);
    assert_eq!(code, Some(0), "{stdout}{stderr}");
    stdout
}

fn focused(root: &Path, session: &str) -> Value {
    open_json(root, session, &["--need", NEED, "--scope", "genesis"])
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

fn semantic_lines(screen: &Value) -> Vec<String> {
    lines(&screen["omissions"])
        .into_iter()
        .filter(|line| line.starts_with("semantic:"))
        .collect()
}

/// The fusion recipe the fixture's contract declares, and its CID.
fn recipe_cid(root: &Path) -> String {
    let value = contract(root).value;
    atom_cid(&value["discovery"]["first_screen_fusion"])
        .unwrap()
        .to_string()
}

fn measure_cid(root: &Path) -> String {
    let measure: IndexMeasure = serde_json::from_value(
        serde_json::from_slice(&std::fs::read(root.join(MEASURE_REL)).unwrap()).unwrap(),
    )
    .unwrap();
    measure.cid().unwrap().to_string()
}

/// Σ 1/(k + rank) over the two orders; ties by path.
fn expected_rrf(local: &[String], semantic: &[String], k: f64) -> Vec<String> {
    let mut all: Vec<String> = local.iter().chain(semantic).cloned().collect();
    all.sort();
    all.dedup();
    let score = |path: &String| {
        [local, semantic]
            .iter()
            .filter_map(|order| order.iter().position(|p| p == path))
            .map(|at| 1.0 / (k + (at + 1) as f64))
            .sum::<f64>()
    };
    all.sort_by(|a, b| score(b).total_cmp(&score(a)).then_with(|| a.cmp(b)));
    all
}

/// Contract v19 declares the fusion recipe as plain JSON, and the bank is re-pinned.
#[test]
fn contract_v19_declares_the_fusion_recipe_and_repins_the_bank() {
    let root = common::repo_root();
    let contract = Contract::load(&root.join(CONTRACT_REL)).expect("live contract loads");
    let value = common::live_contract();
    assert!(value["version"].as_u64() >= Some(19));
    assert_eq!(
        value["discovery"]["first_screen_fusion"],
        json!({"recipe": "rrf-v1", "k": 60, "producers": ["local", "semantic"],
               "order_only": true})
    );
    let method = contract.method_cid();
    assert_ne!(method, V18_METHOD_CID, "the contract's bytes moved");
    assert_ne!(method, V19_METHOD_CID, "and moved again at v20");
    let bank = live(value["question_bank"].as_str().expect("bank"));
    assert_eq!(bank["recipe"].as_str(), Some(method.as_str()));
    contract
        .question_bank()
        .expect("every question is in scope of the v19 recipe");
}

/// Contract v20 (fix round 1, controller ruling): the method prose says the fused first screen's
/// own semantic call is part of the screen and does not consume the packet's explicit search.
#[test]
fn contract_v20_says_the_first_screens_semantic_call_is_part_of_the_screen() {
    let root = common::repo_root();
    let contract = Contract::load(&root.join(CONTRACT_REL)).expect("live contract loads");
    let value = common::live_contract();
    // v20 introduced this line; later versions keep it (each byte change bumps — 4.6 made v21).
    assert!(value["version"].as_u64() >= Some(20));
    let method_lines = value["method"].to_string();
    assert!(
        method_lines.contains("does not consume the packet's explicit search"),
        "{method_lines}"
    );
    let method = contract.method_cid();
    assert_ne!(method, V19_METHOD_CID, "the contract's bytes moved");
    let bank = live(value["question_bank"].as_str().expect("bank"));
    assert_eq!(bank["recipe"].as_str(), Some(method.as_str()));
    contract
        .question_bank()
        .expect("every question is in scope of the v20 recipe");
}

/// Step 4: the fused order is RRF over the two producer orders; a lexical-only target and a
/// semantic-only target both appear on one screen, each with its ranks and its linked read.
#[test]
fn the_fused_order_is_rrf_over_the_lexical_and_semantic_orders() {
    let dir = tree();
    let root = dir.path();

    // The lexical screen, before any fold: semantic is absent.
    let lexical = focused(root, "lexical");
    let local_order = paths(&lexical["first_screen"]["candidates"]);
    assert!(!local_order.is_empty(), "{lexical}");

    fold(root);
    let semantic = retrieve(root, &contract(root), "semantic", NEED, "genesis", &[], &[])
        .expect("an honest answer");
    let semantic_order = paths(&semantic["candidates"]);
    assert!(!semantic_order.is_empty(), "{semantic}");

    let view = focused(root, "fused");
    let screen = &view["first_screen"];
    let fused = paths(&screen["candidates"]);
    assert_eq!(
        fused,
        expected_rrf(&local_order, &semantic_order, 60.0),
        "{screen}"
    );

    for candidate in screen["candidates"].as_array().unwrap() {
        let path = candidate["path"].as_str().unwrap().to_string();
        let rank_in = |order: &[String]| {
            order
                .iter()
                .position(|p| *p == path)
                .map_or(Value::Null, |at| json!(at + 1))
        };
        assert_eq!(
            candidate["ranks"],
            json!({"local": rank_in(&local_order), "semantic": rank_in(&semantic_order)}),
            "{candidate}"
        );
    }

    // Lexical-only and semantic-only, on one screen.
    let by = |path: &str| {
        screen["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["path"] == path)
            .unwrap_or_else(|| panic!("{path} on the fused screen: {screen}"))
            .clone()
    };
    let lexical_only = by("genesis/orbit.json");
    assert!(lexical_only["ranks"]["semantic"].is_null());
    assert!(lexical_only["ranks"]["local"].is_u64());
    let semantic_only = by("genesis/triage.md");
    assert!(semantic_only["ranks"]["local"].is_null());
    assert_eq!(semantic_only["ranks"]["semantic"], 1);
    assert_eq!(semantic_only["best_section"]["lines"], "3:4");
    assert_eq!(semantic_only["method"], json!(measure_cid(root)));

    // The semantic-only candidate's linked read lands on its passage.
    let labels: Vec<&str> = view["actions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|a| a["label"].as_str())
        .collect();
    assert!(
        labels.contains(&"Read genesis/triage.md — Who fixes it (3:4)"),
        "{labels:?}"
    );

    // The recipe and its method, named on the screen.
    assert_eq!(screen["fusion"]["recipe"], "rrf-v1");
    assert_eq!(screen["fusion"]["cid"], json!(recipe_cid(root)));
    assert_eq!(screen["fusion"]["k"], 60);
    assert!(semantic_lines(screen).is_empty(), "{screen}");

    // Budget honesty: the semantic call is in the view's usage as part of the screen — its own
    // key, never the packet's `search_queries` — and never charged as source bytes.
    assert_eq!(
        view["usage"]["first_screen_semantic_calls"], 1,
        "{}",
        view["usage"]
    );
    assert_eq!(
        view["usage"]["search_queries"], lexical["usage"]["search_queries"],
        "the screen's own call does not consume the packet's search"
    );
    assert!(lexical["usage"]
        .get("first_screen_semantic_calls")
        .is_none());
    assert!(
        lexical["usage"].get("semantic_query_ms").is_none(),
        "an absent route charges nothing"
    );
    assert!(view["usage"]["semantic_chunks_scanned"].as_u64().unwrap() > 0);
    assert!(view["usage"]["semantic_query_ms"].is_u64());
    assert_eq!(
        view["usage"]["source_bytes"].as_u64().unwrap_or(0),
        lexical["usage"]["source_bytes"].as_u64().unwrap_or(0),
        "the vector scan is not source bytes"
    );
}

/// Ruling 5: at `standard` a candidate line prints its producer ranks and the method handle of
/// the producers that returned it; the floor line names the fusion recipe.
#[test]
fn standard_prints_ranks_and_the_floor_names_the_recipe() {
    let dir = tree();
    let root = dir.path();
    fold(root);
    let text = open_text(root, "standard", &["--need", NEED, "--scope", "genesis"]);
    let recipe = recipe_cid(root);
    let handle = format!("{}…{}", &recipe[..8], &recipe[recipe.len() - 4..]);
    let floor = text
        .lines()
        .find(|line| line.contains("omissions:"))
        .expect("floor line");
    assert!(
        floor.contains(&format!("fusion rrf-v1 {handle}")),
        "{floor}"
    );
    assert!(
        text.contains("local — · semantic #1"),
        "the semantic-only candidate prints its ranks:\n{text}"
    );
    assert!(
        text.lines()
            .any(|line| line.contains("local #") && line.contains("semantic —")),
        "the lexical-only candidate prints its ranks:\n{text}"
    );
    let measure = measure_cid(root);
    let measure_handle = format!("{}…{}", &measure[..8], &measure[measure.len() - 4..]);
    assert!(text.contains(&measure_handle), "{text}");
}

/// Rulings 4 and 5: at `minimal` the lens cut applies AFTER fusion — the one ordinary candidate
/// shown is the fused #1 — it carries one collapsed `fused` tag, and a `[floor]` candidate past
/// the cut still survives.
#[test]
fn the_lens_cut_and_the_content_floor_apply_after_fusion() {
    let dir = tree();
    let root = dir.path();
    common::write(
        root,
        "genesis/erratum.md",
        "---\ntitle: Erratum\ncontent_class: correction\n---\n# Erratum\nWho can fix the bug \
         now? Not the one we named.\n",
    );
    common::git(root, &["add", "-A"]);
    common::git(root, &["commit", "-qm", "erratum"]);
    fold(root);
    let view = open_json(
        root,
        "minimal-json",
        &["--need", NEED, "--scope", "genesis", "--lens", "minimal"],
    );
    let fused = paths(&view["first_screen"]["candidates"]);
    let at = fused
        .iter()
        .position(|p| p == "genesis/erratum.md")
        .expect("the correction is a candidate");
    assert!(
        at > 0,
        "precondition: the correction is past the cut: {fused:?}"
    );
    // Only the semantic route found it (it never says `orbit`), so its content class was read by
    // the fusion step itself.
    let erratum = &view["first_screen"]["candidates"][at];
    assert!(erratum["ranks"]["local"].is_null(), "{erratum}");
    assert_eq!(erratum["content_class"], "correction");
    assert_eq!(erratum["title"], "Erratum");

    let text = open_text(
        root,
        "minimal-text",
        &["--need", NEED, "--scope", "genesis", "--lens", "minimal"],
    );
    let block: Vec<&str> = text
        .split("Candidate sources:\n")
        .nth(1)
        .expect("a candidate block")
        .lines()
        .take_while(|line| line.starts_with("  "))
        .collect();
    let ordinary: Vec<&&str> = block.iter().filter(|l| !l.contains("[floor]")).collect();
    assert_eq!(ordinary.len(), 1, "{text}");
    assert!(ordinary[0].contains(&format!(" {} ", fused[0])), "{text}");
    assert!(ordinary[0].ends_with("[fused]"), "{text}");
    assert!(
        block
            .iter()
            .any(|l| l.contains("genesis/erratum.md") && l.contains("[floor]")),
        "{text}"
    );
    assert!(
        !text.contains("local #"),
        "ranks collapse at minimal:\n{text}"
    );
}

/// Ruling 2: an absent route is ONE omission line naming why; the screen is the lexical screen,
/// with no `fused` tag and no ranks — and the open exits 0.
#[test]
fn an_absent_semantic_route_is_one_omission_and_the_lexical_screen() {
    let dir = tree();
    let root = dir.path();
    let view = focused(root, "absent");
    let screen = &view["first_screen"];
    assert_eq!(
        semantic_lines(screen),
        vec![NO_FOLD.to_string()],
        "{screen}"
    );
    assert!(screen.get("fusion").is_none_or(Value::is_null), "{screen}");
    assert!(screen["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .all(|c| c.get("ranks").is_none()));
    assert!(
        !paths(&screen["candidates"]).contains(&"genesis/triage.md".to_string()),
        "no semantic-only candidate without the route"
    );
    let text = open_text(root, "absent-text", &["--need", NEED, "--scope", "genesis"]);
    assert!(
        !text.contains("fused") && !text.contains("local #"),
        "{text}"
    );
    assert!(!text.contains("fusion rrf-v1"), "{text}");
    assert!(text.contains(&format!("· {NO_FOLD}")), "{text}");

    // A recipe that declares no semantic provider at all: one line, the lexical screen.
    let mut undeclared = contract(root).value;
    undeclared["ceremony"]["providers"]
        .as_object_mut()
        .unwrap()
        .remove("semantic");
    put_json(root, "lexical-contract.json", &undeclared);
    fold(root);
    let view = open_json_under(
        root,
        "lexical-contract.json",
        "undeclared",
        &["--need", NEED, "--scope", "genesis"],
    );
    let screen = &view["first_screen"];
    let absent = semantic_lines(screen);
    assert_eq!(absent.len(), 1, "{screen}");
    assert!(absent[0].contains("not declared"), "{absent:?}");
    assert!(screen["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .all(|c| c.get("ranks").is_none()));
}

/// Step 4: a stale fold fuses AND says how stale.
#[test]
fn a_stale_fold_fuses_and_prints_how_far_behind() {
    let dir = tree();
    let root = dir.path();
    fold(root);
    for n in 0..3 {
        common::write(
            root,
            &format!("genesis/later/note-{n}.md"),
            &format!("# Later {n}\nWritten after the fold.\n"),
        );
    }
    common::git(root, &["add", "-A"]);
    let view = focused(root, "stale");
    let screen = &view["first_screen"];
    assert_eq!(screen["fusion"]["recipe"], "rrf-v1", "{screen}");
    assert!(
        lines(&screen["omissions"])
            .iter()
            .any(|m| m.starts_with("fold 3 files behind")),
        "{screen}"
    );
}

/// Ruling 4: the whole-tree root authority screen fuses too when semantic answers — the
/// authority set is the local order.
#[test]
fn the_root_authority_screen_fuses() {
    let dir = tree();
    let root = dir.path();
    common::write(
        root,
        "CLAUDE.md",
        "# Guide\n## Habits\nThe orbit relay habit is top red; move it toward green with proof.\n",
    );
    common::write(
        root,
        "genesis/manifests/habits.yaml",
        "habits:\n- id: orbit-relay\n  status: red\n  active: true\n  checks: ['a2o \
         @concern:orbit (genesis/run.sh)']\n  invariant: the orbit relay stays up\n",
    );
    common::write(
        root,
        "genesis/.epr-meta/orbit-relay.habit.md",
        "---\nid: orbit-relay\nstatus: red\nchecks:\n  - a2o @concern:orbit (genesis/run.sh)\n---\n\
         DELTA 2026-09-20: the orbit relay dropped.\n",
    );
    common::git(root, &["add", "-A"]);
    common::git(root, &["commit", "-qm", "habit"]);
    fold(root);
    let view = open_json(
        root,
        "authority",
        &["--need", "which orbit relay habit is red"],
    );
    let screen = &view["first_screen"];
    assert_eq!(screen["area"], ".", "{screen}");
    assert_eq!(screen["fusion"]["recipe"], "rrf-v1", "{screen}");
    assert!(
        screen["ranking"]
            .as_str()
            .is_some_and(|r| r.contains("authority set")),
        "{screen}"
    );
    let claude = screen["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["path"] == "CLAUDE.md")
        .expect("the authority set stays on the screen");
    assert!(claude["ranks"]["local"].is_u64(), "{claude}");
    assert!(
        screen["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["ranks"]["semantic"].is_u64()),
        "{screen}"
    );
}
/// A tree whose semantic route reads a PINNED store the declared procedure would embed against,
/// with an interpreter script that leaves a mark each time it runs and then exits 1 (so every
/// embedding that spawns fails). `label` relabels the store's embedder (`None` keeps the
/// procedure's own label). Returns (tree, marker, interpreter).
fn pinned_marker_tree(label: Option<&str>) -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    use elohim_epr_cli::flow::memory::recall::embedder::{MODEL_MANIFEST_REL, PROCEDURE_REL};
    use std::os::unix::fs::PermissionsExt;
    let dir = tree();
    let root = dir.path();
    let mut contract_value = contract(root).value;
    contract_value["ceremony"]["providers"]["semantic"]["embedder"] = json!("pinned");
    put_json(root, CONTRACT_REL, &contract_value);
    common::write(
        root,
        "genesis/manifests/habits.yaml",
        "habits:\n- id: alpha\n  status: red\n  active: true\n  checks: ['a2o @concern:alpha']\n  \
         invariant: alpha holds\n",
    );
    fold(root);
    // The fixture store, copied to the pinned store and labelled as the declared procedure's.
    let cid = measure_cid(root);
    let pinned = index::store_dir(root, &cid, EmbedderChoice::Pinned);
    std::fs::create_dir_all(&pinned).unwrap();
    std::fs::copy(
        index::store_dir(root, &cid, EmbedderChoice::Fixture).join("fold.sqlite"),
        pinned.join("fold.sqlite"),
    )
    .unwrap();
    let mut manifest = live(MODEL_MANIFEST_REL);
    let procedure_label = format!(
        "procedure {} on model {}",
        manifest["procedure"].as_str().unwrap(),
        manifest["model_bytes"].as_str().unwrap()
    );
    let conn = rusqlite::Connection::open(pinned.join("fold.sqlite")).unwrap();
    conn.execute(
        "UPDATE meta SET value = ?1 WHERE key = 'embedder'",
        [label.unwrap_or(&procedure_label)],
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
    (dir, marker, interpreter)
}

/// A focused open under the marking interpreter, as JSON; asserts exit 0.
fn focused_with_interpreter(root: &Path, session: &str, interpreter: &Path) -> Value {
    use elohim_epr_cli::flow::memory::recall::embedder::INTERPRETER_ENV;
    let (code, stdout, stderr) = cli_with(
        root,
        CONTRACT_REL,
        session,
        &["open", "--need", NEED, "--scope", "genesis", "--json"],
        &[(INTERPRETER_ENV, interpreter)],
    );
    assert_eq!(
        code,
        Some(0),
        "an unavailable route never exits the screen: {stdout}{stderr}"
    );
    serde_json::from_str(&stdout).unwrap()
}

/// Ruling 2: `open --purpose bootstrap` asks the semantic route nothing — no embedding process
/// spawns. A focused open with a typed `--need` leaves the interpreter's mark (the control), a
/// bootstrap open does not.
#[test]
fn bootstrap_spawns_no_embedding_process() {
    use elohim_epr_cli::flow::memory::recall::embedder::INTERPRETER_ENV;
    let (dir, marker, interpreter) = pinned_marker_tree(None);
    let root = dir.path();
    let (code, stdout, stderr) = cli_with(
        root,
        CONTRACT_REL,
        "boot",
        &["open", "--purpose", "bootstrap", "--json"],
        &[(INTERPRETER_ENV, interpreter.as_path())],
    );
    assert_eq!(code, Some(0), "{stdout}{stderr}");
    assert!(!marker.exists(), "bootstrap spawned an embedding process");
    let boot: Value = serde_json::from_str(&stdout).unwrap();
    assert!(boot["first_screen"].is_null(), "{boot}");

    // The control: a typed --need does ask the route, so the harness would have seen a spawn.
    let view = focused_with_interpreter(root, "focused", &interpreter);
    assert!(marker.exists(), "the control asked the pinned embedder");
    let absent = semantic_lines(&view["first_screen"]);
    assert_eq!(absent.len(), 1, "{view}");
    assert!(
        absent[0].starts_with("semantic: unavailable:"),
        "{absent:?}"
    );
}

/// Metering (fix round 1, item 7): an embedding process that ran and then failed is charged as
/// the screen's call — `first_screen_semantic_calls: 1` plus its seconds — though it ranked
/// nothing; never as the packet's `search_queries`.
#[test]
fn a_spawned_then_failed_embedder_is_charged_to_the_screen() {
    let (dir, marker, interpreter) = pinned_marker_tree(None);
    let root = dir.path();
    let view = focused_with_interpreter(root, "spawned", &interpreter);
    assert!(marker.exists(), "precondition: the embedding process ran");
    assert!(view["first_screen"]
        .get("fusion")
        .is_none_or(Value::is_null));
    let usage = &view["usage"];
    assert_eq!(usage["first_screen_semantic_calls"], 1, "{usage}");
    assert_eq!(usage["embedding_processes"], 1, "{usage}");
    assert!(
        usage["provider_seconds"].as_f64().unwrap_or(0.0) > 0.0,
        "{usage}"
    );
    assert_eq!(
        usage["search_queries"], 1,
        "the lexical traversal's own, only: {usage}"
    );
}

/// Metering (fix round 1, item 7): a route refused before any spawn — here a store folded under
/// another embedder label — ran nothing and is charged nothing.
#[test]
fn a_route_refused_before_any_spawn_is_charged_nothing() {
    let (dir, marker, interpreter) = pinned_marker_tree(Some("fixture"));
    let root = dir.path();
    let view = focused_with_interpreter(root, "refused", &interpreter);
    assert!(!marker.exists(), "precondition: nothing spawned");
    assert_eq!(
        semantic_lines(&view["first_screen"]),
        vec!["semantic: the fold was built under another method — refold".to_string()]
    );
    let usage = &view["usage"];
    for key in [
        "first_screen_semantic_calls",
        "embedding_processes",
        "provider_seconds",
        "semantic_query_ms",
        "semantic_chunks_scanned",
    ] {
        assert!(usage.get(key).is_none(), "{key} charged: {usage}");
    }
}

/// Fix round 1, finding 1: the generated habit register is never offered on a first screen,
/// whichever producer found it — a semantic-only hit on it never reaches the fused screen.
#[test]
fn a_semantic_hit_on_the_habit_register_never_reaches_the_fused_screen() {
    let dir = tree();
    let root = dir.path();
    let mut measure = live(MEASURE_REL);
    let mut surface: Vec<&str> = SURFACE.to_vec();
    surface.push("genesis/**/*.yaml");
    measure["surfaces"]["paths"] = json!(surface);
    put_json(root, MEASURE_REL, &measure);
    common::write(
        root,
        "genesis/manifests/habits.yaml",
        "habits:\n- id: triage\n  status: red\n  invariant: who can fix the bug now\n",
    );
    common::git(root, &["add", "-A"]);
    common::git(root, &["commit", "-qm", "register"]);
    fold(root);
    let semantic = retrieve(root, &contract(root), "semantic", NEED, "genesis", &[], &[])
        .expect("an honest answer");
    assert!(
        paths(&semantic["candidates"]).contains(&"genesis/manifests/habits.yaml".to_string()),
        "precondition: the semantic route ranks the register: {semantic}"
    );
    let view = focused(root, "register");
    let screen = &view["first_screen"];
    assert_eq!(screen["fusion"]["recipe"], "rrf-v1", "{screen}");
    assert!(
        !paths(&screen["candidates"]).contains(&"genesis/manifests/habits.yaml".to_string()),
        "{screen}"
    );
}

/// Fix round 1, finding 5: one absent producer does not cancel fusion for the others — the
/// screen fuses over those that answered, with one omission line per absent producer.
#[test]
fn an_absent_third_producer_does_not_cancel_fusion() {
    let dir = tree();
    let root = dir.path();
    fold(root);
    let mut value = contract(root).value;
    value["discovery"]["first_screen_fusion"]["producers"] =
        json!(["local", "semantic", "mempalace"]);
    put_json(root, "three-producers.json", &value);
    let view = open_json_under(
        root,
        "three-producers.json",
        "three",
        &["--need", NEED, "--scope", "genesis"],
    );
    let screen = &view["first_screen"];
    assert_eq!(screen["fusion"]["recipe"], "rrf-v1", "{screen}");
    let mempalace: Vec<String> = lines(&screen["omissions"])
        .into_iter()
        .filter(|line| line.starts_with("mempalace:"))
        .collect();
    assert_eq!(mempalace.len(), 1, "{screen}");
    let producers: Vec<&str> = screen["fusion"]["producers"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["id"].as_str())
        .collect();
    assert_eq!(producers, vec!["local", "semantic"]);
    assert!(
        paths(&screen["candidates"]).contains(&"genesis/triage.md".to_string()),
        "{screen}"
    );
}

/// Fix round 1, finding 6: no fusion token over nothing — a semantic route that answers with no
/// candidate in the area leaves the lexical screen, with no `fusion`, ranks or tag.
#[test]
fn a_semantic_answer_with_nothing_in_the_area_leaves_the_lexical_screen() {
    let dir = tree();
    let root = dir.path();
    common::write(
        root,
        "genesis/data/orbit.json",
        "{\"orbit\": \"orbit period\"}\n",
    );
    common::git(root, &["add", "-A"]);
    common::git(root, &["commit", "-qm", "data"]);
    fold(root);
    let view = open_json(
        root,
        "nothing",
        &["--need", NEED, "--scope", "genesis/data"],
    );
    let screen = &view["first_screen"];
    assert_eq!(
        paths(&screen["candidates"]),
        vec!["genesis/data/orbit.json".to_string()],
        "{screen}"
    );
    assert!(screen.get("fusion").is_none_or(Value::is_null), "{screen}");
    assert!(screen["candidates"][0].get("ranks").is_none(), "{screen}");
    let text = open_text(
        root,
        "nothing-text",
        &["--need", NEED, "--scope", "genesis/data"],
    );
    assert!(
        !text.contains("fusion rrf-v1") && !text.contains("local #"),
        "{text}"
    );
}
