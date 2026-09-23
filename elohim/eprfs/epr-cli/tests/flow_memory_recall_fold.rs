//! Governed-discovery station 4, task 4.3: the semantic fold — incremental, fingerprinted,
//! attested, bounded per run.
//!
//! `epr flow memory index fold` walks `recall-semantic-index@1`'s surfaces under the contract's
//! `discovery.exclude_directories` and `first_screen_globs`, fingerprints each file (the raw CID
//! of its bytes), re-chunks and re-embeds only what moved — at most `limits.fold_files_per_run`
//! files a run — demotes (never deletes) what disappeared, and writes a `FoldAttestation` that
//! says what the fold holds and whether it is `complete`, `degraded` (the run cap stopped it) or
//! `failed` (the method or the procedure refused). Every test here runs the `Fixture` embedder on
//! a temporary tree carrying copies of the live contract and measure, and reads the real SQLite
//! store the fold wrote.
mod common;

use std::path::Path;
use std::process::Command;

use elohim_epr_cli::flow::memory::recall::index::{
    self, EmbedderChoice, FoldOptions, FoldReport, UNCLAIMED,
};
use elohim_epr_cli::flow::memory::recall::CONTRACT_REL;
use elohim_epr_rea::{FoldAttestation, FoldState, IndexMeasure};
use serde_json::Value;
use tempfile::TempDir;

const MEASURE_REL: &str = ".epr-meta/elohim/algorithms/recall-semantic-index.json";

fn live(rel: &str) -> Value {
    let raw = std::fs::read(common::repo_root().join(rel)).expect("live file reads");
    serde_json::from_slice(&raw).expect("live file parses")
}

fn put_json(root: &Path, rel: &str, value: &Value) {
    common::write(root, rel, &serde_json::to_string_pretty(value).unwrap());
}

/// A temporary repository holding the live contract and measure (byte copies, so the measure's
/// CID is the pinned one) and a small surface: two markdown notes, a Python module, a note under
/// `.claude/`, and a `.txt` file no `first_screen_glob` admits.
fn tree() -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    for rel in [CONTRACT_REL, MEASURE_REL] {
        let target = root.join(rel);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(common::repo_root().join(rel), target).unwrap();
    }
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
        "genesis/tool.py",
        "import os\n\ndef fold():\n    return 1\n\nclass Store:\n    pass\n",
    );
    common::write(root, ".claude/notes.md", "# Notes\nA household mesh.\n");
    common::write(root, "genesis/ignored.txt", "no glob admits a .txt file\n");
    dir
}

fn options() -> FoldOptions {
    FoldOptions {
        embedder: EmbedderChoice::Fixture,
        ..FoldOptions::default()
    }
}

fn fold(root: &Path) -> FoldReport {
    index::fold(root, &options()).expect("the fold runs")
}

fn fold_capped(root: &Path, max_files: usize) -> FoldReport {
    let opts = FoldOptions {
        max_files: Some(max_files),
        ..options()
    };
    index::fold(root, &opts).expect("the fold runs")
}

fn status(root: &Path) -> Value {
    index::status(root).expect("status reads")
}

fn measure_cid(root: &Path) -> String {
    let measure: IndexMeasure = serde_json::from_value(
        serde_json::from_slice(&std::fs::read(root.join(MEASURE_REL)).unwrap()).unwrap(),
    )
    .unwrap();
    measure.cid().unwrap().to_string()
}

fn store(root: &Path) -> rusqlite::Connection {
    let path = index::store_dir(root, &measure_cid(root)).join("fold.sqlite");
    rusqlite::Connection::open(path).expect("the fold store opens")
}

fn count(conn: &rusqlite::Connection, sql: &str, path: &str) -> i64 {
    conn.query_row(sql, [path], |row| row.get(0)).unwrap()
}

fn attestations(root: &Path) -> Vec<FoldAttestation> {
    let log = index::store_dir(root, &measure_cid(root)).join("attestations.jsonl");
    std::fs::read_to_string(log)
        .expect("the attestation log reads")
        .lines()
        .map(|line| serde_json::from_str(line).expect("one attestation per line"))
        .collect()
}

/// The first fold over a fresh tree attests `complete`: every admitted file is folded, the
/// shard's atom count is the live chunk count, the heads name every declared surface root, and
/// the attestation is both the latest snapshot and the first line of the append-only log.
#[test]
fn the_first_fold_attests_complete() {
    let dir = tree();
    let root = dir.path();
    let report = fold(root);
    assert_eq!(report.attestation.state, FoldState::Complete);
    assert_eq!(report.lag, Some(0));
    assert!(report.folded.contains(&"genesis/alpha.md".to_string()));
    assert!(report.folded.contains(&"genesis/tool.py".to_string()));
    assert!(report.folded.contains(&".claude/notes.md".to_string()));
    assert!(
        !report.folded.iter().any(|p| p.ends_with(".txt")),
        "only first_screen_globs are admitted"
    );
    assert_eq!(report.attestation.attested_by.0, UNCLAIMED);
    assert_eq!(report.attestation.measure.to_string(), measure_cid(root));
    let measure: IndexMeasure = serde_json::from_value(live(MEASURE_REL)).unwrap();
    assert_eq!(
        report.attestation.heads_at.len(),
        measure.surfaces.paths().len()
    );
    assert!(report.attestation.shard.arc.is_none());

    let conn = store(root);
    let live_chunks: i64 = conn
        .query_row(
            "SELECT count(*) FROM chunks WHERE demoted_at IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(report.attestation.shard.atoms, live_chunks as u64);
    let vector: Vec<u8> = conn
        .query_row(
            "SELECT vector FROM chunks WHERE path = 'genesis/alpha.md' LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(vector.len(), 384 * 4, "384 little-endian f32s per chunk");
    // The FTS5 table the lexical provider will share is built over the same rows.
    let hits: i64 = conn
        .query_row(
            "SELECT count(*) FROM chunks_fts WHERE chunks_fts MATCH 'stewardship'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(hits, 1);

    let latest: FoldAttestation = serde_json::from_slice(
        &std::fs::read(index::store_dir(root, &measure_cid(root)).join("attestation.json"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(latest, report.attestation);
    assert_eq!(attestations(root), vec![report.attestation.clone()]);

    let seen = status(root);
    assert_eq!(seen["lag"], 0);
    assert_eq!(seen["chunks"], live_chunks);
    assert_eq!(seen["last"]["state"]["state"], "complete");
    assert_eq!(seen["last"]["attested_by"], UNCLAIMED);
}

/// Editing one file puts the fold one file behind; the next fold re-embeds exactly that file,
/// demotes its superseded chunks, and only the surface root holding it names a moved head.
#[test]
fn editing_one_file_refolds_exactly_that_file() {
    let dir = tree();
    let root = dir.path();
    let first = fold(root);
    common::write(
        root,
        "genesis/beta.md",
        "# Beta\nStewardship of the commons, revised.\n",
    );
    assert_eq!(status(root)["lag"], 1);

    let second = fold(root);
    assert_eq!(second.folded, vec!["genesis/beta.md".to_string()]);
    assert_eq!(second.attestation.state, FoldState::Complete);
    assert_eq!(status(root)["lag"], 0);

    let conn = store(root);
    let demoted = count(
        &conn,
        "SELECT count(*) FROM chunks WHERE path = ?1 AND demoted_at IS NOT NULL",
        "genesis/beta.md",
    );
    assert!(
        demoted > 0,
        "the superseded chunks are demoted, not deleted"
    );
    assert_eq!(
        count(
            &conn,
            "SELECT count(*) FROM chunks WHERE path = ?1 AND demoted_at IS NULL AND text LIKE '%revised%'",
            "genesis/beta.md",
        ),
        1
    );

    let moved: Vec<bool> = first
        .attestation
        .heads_at
        .iter()
        .zip(&second.attestation.heads_at)
        .map(|(a, b)| {
            assert_eq!(a.0, b.0, "the same surface root, in the same order");
            a.1 != b.1
        })
        .collect();
    let measure: IndexMeasure = serde_json::from_value(live(MEASURE_REL)).unwrap();
    let genesis = measure
        .surfaces
        .paths()
        .iter()
        .position(|p| p == "genesis/")
        .expect("genesis/ is a surface");
    assert!(moved[genesis], "the root holding the edit moved");
    assert_eq!(
        moved.iter().filter(|m| **m).count(),
        1,
        "no other root moved"
    );
    assert_eq!(attestations(root).len(), 2, "attestations are appended");
}

/// A deleted file's chunks are demoted — they stay in the store with `demoted_at` and leave the
/// live set a ranking reads — and the store refuses a delete outright.
#[test]
fn a_deleted_file_is_demoted_never_deleted() {
    let dir = tree();
    let root = dir.path();
    let first = fold(root);
    std::fs::remove_file(root.join("genesis/alpha.md")).unwrap();
    assert_eq!(status(root)["lag"], 1);

    let second = fold(root);
    assert_eq!(second.demoted, vec!["genesis/alpha.md".to_string()]);
    assert_eq!(second.attestation.state, FoldState::Complete);
    assert!(second.attestation.shard.atoms < first.attestation.shard.atoms);

    let conn = store(root);
    assert_eq!(
        count(
            &conn,
            "SELECT count(*) FROM chunks WHERE path = ?1 AND demoted_at IS NULL",
            "genesis/alpha.md",
        ),
        0,
        "no live chunk of a removed file is left to rank"
    );
    assert!(
        count(
            &conn,
            "SELECT count(*) FROM chunks WHERE path = ?1 AND demoted_at IS NOT NULL",
            "genesis/alpha.md",
        ) > 0,
        "its rows are kept, demoted"
    );
    assert!(
        conn.execute("DELETE FROM chunks", []).is_err(),
        "demotion, never deletion: the store refuses a delete"
    );
    let demoted_rows: i64 = conn
        .query_row(
            "SELECT count(*) FROM chunks WHERE demoted_at IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(status(root)["demoted"], demoted_rows);
}

/// `--max-files 1` on three changes folds one, attests `degraded` with `retried: 0`, and leaves
/// the fold two files behind; each further capped run is a retry, and the run that drains the
/// backlog attests `complete`.
#[test]
fn the_per_run_cap_degrades_the_fold_and_counts_retries() {
    let dir = tree();
    let root = dir.path();
    fold(root);
    common::write(root, "genesis/alpha.md", "# Alpha\nchanged\n");
    common::write(root, "genesis/beta.md", "# Beta\nchanged\n");
    common::write(root, ".claude/notes.md", "# Notes\nchanged\n");
    assert_eq!(status(root)["lag"], 3);

    let capped = fold_capped(root, 1);
    assert_eq!(capped.folded.len(), 1);
    assert_eq!(capped.attestation.state, FoldState::Degraded { retried: 0 });
    assert_eq!(capped.lag, Some(2));
    assert_eq!(status(root)["lag"], 2);
    assert_eq!(status(root)["last"]["state"]["state"], "degraded");

    let again = fold_capped(root, 1);
    assert_eq!(again.attestation.state, FoldState::Degraded { retried: 1 });
    assert_eq!(again.lag, Some(1));

    let drained = fold(root);
    assert_eq!(drained.attestation.state, FoldState::Complete);
    assert_eq!(drained.lag, Some(0));
}

/// The per-run file cap is declared in the contract (`limits.fold_files_per_run`), never coded:
/// a contract declaring 2 folds two files a run.
#[test]
fn the_default_run_cap_is_the_contract_declaration() {
    let dir = tree();
    let root = dir.path();
    let mut contract = live(CONTRACT_REL);
    contract["limits"]["fold_files_per_run"] = serde_json::json!(2);
    put_json(root, CONTRACT_REL, &contract);
    let report = fold(root);
    assert_eq!(report.folded.len(), 2);
    assert!(matches!(
        report.attestation.state,
        FoldState::Degraded { retried: 0 }
    ));
}

/// The private chain never enters a fold: a recall session's private store and a sibling
/// worktree are skipped even when they sit under a declared surface, and a surface that NAMES the
/// private store is still refused by the exclusion at every depth.
#[test]
fn the_private_recall_store_and_worktrees_are_never_folded() {
    let dir = tree();
    let root = dir.path();
    common::write(
        root,
        "genesis/.eprfs/status/recall/s1/receipt.md",
        "# Receipt\nprivate reasoning\n",
    );
    common::write(
        root,
        ".eprfs/status/recall/s1/continuation.json",
        "{\"private\": true}\n",
    );
    common::write(
        root,
        ".claude/worktrees/agent-a/CLAUDE.md",
        "# Another checkout\n",
    );
    let mut measure = live(MEASURE_REL);
    let paths = measure["surfaces"]["paths"].as_array_mut().unwrap();
    paths.push(serde_json::json!(".eprfs/status/recall/"));
    put_json(root, MEASURE_REL, &measure);

    let report = fold(root);
    assert_eq!(report.attestation.state, FoldState::Complete);
    let conn = store(root);
    let leaked: i64 = conn
        .query_row(
            "SELECT count(*) FROM files WHERE path LIKE '%.eprfs/%' OR path LIKE '%worktrees/%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(leaked, 0, "{:?}", report.folded);
    assert!(report.folded.contains(&".claude/notes.md".to_string()));
}

/// The method is the declaration: a chunk rule on disk that no longer hashes to the measure's
/// `chunkRule` is attested `failed`, and nothing is folded under it.
#[test]
fn a_chunk_rule_that_does_not_hash_to_the_measure_fails_the_fold() {
    let dir = tree();
    let root = dir.path();
    let mut measure = live(MEASURE_REL);
    measure["_chunk_rule"]["max_chunks_per_file"] = serde_json::json!(13);
    put_json(root, MEASURE_REL, &measure);
    let report = fold(root);
    assert_eq!(
        report.attestation.state,
        FoldState::Failed {
            why: "chunk rule on disk does not hash to the measure".into()
        }
    );
    assert!(report.folded.is_empty());
    assert_eq!(report.attestation.shard.atoms, 0);
}

/// A fold whose embedding procedure cannot run here attests `failed` with the procedure's own
/// `unavailable: <reason>` and exits 0 — the fold ran and said what it found — and `status` shows
/// the failed state.
#[test]
fn an_unavailable_embedder_is_attested_failed_and_exits_zero() {
    use elohim_epr_cli::flow::memory::recall::embedder::{MODEL_MANIFEST_REL, PROCEDURE_REL};
    let dir = tree();
    let root = dir.path();
    let procedure = root.join(PROCEDURE_REL);
    std::fs::create_dir_all(procedure.parent().unwrap()).unwrap();
    std::fs::copy(common::repo_root().join(PROCEDURE_REL), &procedure).unwrap();
    let mut manifest = live(MODEL_MANIFEST_REL);
    manifest["resolve"] = serde_json::json!([root.join("no-model-here").to_string_lossy()]);
    put_json(root, MODEL_MANIFEST_REL, &manifest);

    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(["flow", "memory", "index", "fold", "--root"])
        .arg(root)
        .env_remove("EPR_EMBED_MODEL_DIR")
        .output()
        .expect("epr runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("failed"), "{stdout}");
    let seen = status(root);
    assert_eq!(seen["last"]["state"]["state"], "failed");
    let why = seen["last"]["state"]["why"].as_str().unwrap();
    assert!(
        why.starts_with("unavailable: no model directory resolves"),
        "{why}"
    );
}

/// A corrupt store is rebuilt from scratch, never repaired by hand.
#[test]
fn a_corrupt_store_is_rebuilt() {
    let dir = tree();
    let root = dir.path();
    fold(root);
    let path = index::store_dir(root, &measure_cid(root)).join("fold.sqlite");
    std::fs::write(
        &path,
        b"this is not a sqlite database, whatever it once was",
    )
    .unwrap();
    let seen = status(root);
    assert!(
        seen["lag"].is_null(),
        "an unreadable store is no fold: {seen}"
    );
    let report = fold(root);
    assert!(report.store.starts_with("rebuilt"), "{}", report.store);
    assert_eq!(report.attestation.state, FoldState::Complete);
    assert_eq!(status(root)["lag"], 0);
}

/// With no store there is no lag to report — `status` says so rather than inventing one, and its
/// JSON keeps the stable shape later tasks read.
#[test]
fn status_without_a_fold_says_skipped() {
    let dir = tree();
    let root = dir.path();
    let seen = status(root);
    let keys: Vec<&str> = seen
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let mut expected = vec![
        "measure",
        "model",
        "chunk_rule",
        "chunks",
        "demoted",
        "bytes",
        "lag",
        "last",
    ];
    expected.sort_unstable();
    let mut got = keys.clone();
    got.sort_unstable();
    assert_eq!(got, expected);
    assert!(seen["lag"].is_null());
    assert!(seen["last"].is_null());
    assert_eq!(seen["measure"], measure_cid(root));

    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args(["flow", "memory", "index", "status", "--root"])
        .arg(root)
        .output()
        .expect("epr runs");
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).contains("skipped — no fold"));
}

/// `--session` attributes the attestation to the sidecar's current claim for that session, in its
/// participant form; a session with no claim is the honest `(unclaimed)`.
#[test]
fn the_attestation_names_the_sessions_claimed_participant() {
    let dir = tree();
    let root = dir.path();
    common::git(root, &["init", "-q"]);
    common::git(root, &["add", "-A"]);
    common::git(root, &["commit", "-q", "-m", "fixture"]);
    elohim_epr_cli::actor::claim(root, "agent:implementer@opus-5.5", "fold-session")
        .expect("the claim registers");
    let claimed = index::fold(
        root,
        &FoldOptions {
            session: Some("fold-session".into()),
            ..options()
        },
    )
    .unwrap();
    assert_eq!(
        claimed.attestation.attested_by.0,
        "agent:implementer@opus-5.5"
    );

    common::write(root, "genesis/beta.md", "# Beta\nagain\n");
    let other = index::fold(
        root,
        &FoldOptions {
            session: Some("nobody-claimed-this".into()),
            ..options()
        },
    )
    .unwrap();
    assert_eq!(other.attestation.attested_by.0, UNCLAIMED);
}
