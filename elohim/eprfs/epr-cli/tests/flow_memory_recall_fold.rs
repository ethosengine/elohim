//! Governed-discovery station 4, task 4.3: the semantic fold — incremental, fingerprinted,
//! attested, bounded per run.
//!
//! `epr flow memory index fold` admits the files the measure's glob surface names (git-tracked
//! files on a checkout, else a walk), under the contract's `discovery.exclude_directories` and the
//! private-chain floor, fingerprints each (the raw CID of its bytes; an unchanged size and mtime
//! is not re-hashed), re-chunks and re-embeds only what moved — at most
//! `limits.fold_files_per_run` files a run — demotes (never deletes) what disappeared, and writes
//! a `FoldAttestation`: `complete`, `degraded` (the run cap stopped it, or a file could not be
//! read) or `failed` (the method, the source or the procedure refused). Every test runs the
//! `Fixture` embedder on a temporary tree carrying the live contract and a copy of the live
//! measure whose surface is the fixture's own, and reads the real SQLite store the fold wrote.
mod common;

use std::path::Path;
use std::process::Command;

use elohim_epr_cli::flow::memory::recall::index::{
    self, EmbedderChoice, FoldOptions, FoldReport, FoldRun, BUSY, UNCLAIMED,
};
use elohim_epr_cli::flow::memory::recall::CONTRACT_REL;
use elohim_epr_rea::{FoldAttestation, FoldState, IndexMeasure};
use serde_json::{json, Value};
use tempfile::TempDir;

const MEASURE_REL: &str = ".epr-meta/elohim/algorithms/recall-semantic-index.json";

/// The fixture's own surface — the live measure's is the repository's authority layer, which a
/// temporary tree does not have.
const FIXTURE_SURFACE: [&str; 4] = [
    "genesis/**/*.md",
    "genesis/**/*.py",
    ".claude/**/*.md",
    "!genesis/skip/**",
];

fn live(rel: &str) -> Value {
    let raw = std::fs::read(common::repo_root().join(rel)).expect("live file reads");
    serde_json::from_slice(&raw).expect("live file parses")
}

fn put_json(root: &Path, rel: &str, value: &Value) {
    common::write(root, rel, &serde_json::to_string_pretty(value).unwrap());
}

/// The live measure with `paths` as its surface; its CID derives from the file written.
fn put_measure(root: &Path, paths: &[&str]) -> Value {
    let mut measure = live(MEASURE_REL);
    measure["surfaces"]["paths"] = json!(paths);
    put_json(root, MEASURE_REL, &measure);
    measure
}

/// A temporary repository holding the live contract, a fixture-surface measure and a small tree:
/// two markdown notes, a Python module, a note under `.claude/`, a `.txt` no include admits, and
/// a note under a negated directory.
fn tree() -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let contract = root.join(CONTRACT_REL);
    std::fs::create_dir_all(contract.parent().unwrap()).unwrap();
    std::fs::copy(common::repo_root().join(CONTRACT_REL), contract).unwrap();
    put_measure(root, &FIXTURE_SURFACE);
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
    common::write(
        root,
        "genesis/ignored.txt",
        "no include admits a .txt file\n",
    );
    common::write(root, "genesis/skip/negated.md", "# Negated\nnever folded\n");
    dir
}

fn options() -> FoldOptions {
    FoldOptions {
        embedder: EmbedderChoice::Fixture,
        ..FoldOptions::default()
    }
}

fn done(run: FoldRun) -> FoldReport {
    match run {
        FoldRun::Done(report) => *report,
        FoldRun::Busy => panic!("no other fold holds the store"),
    }
}

fn fold_with(root: &Path, opts: FoldOptions) -> FoldReport {
    done(index::fold(root, &opts).expect("the fold runs"))
}

fn fold(root: &Path) -> FoldReport {
    fold_with(root, options())
}

fn fold_capped(root: &Path, max_files: usize) -> FoldReport {
    fold_with(
        root,
        FoldOptions {
            max_files: Some(max_files),
            ..options()
        },
    )
}

fn status(root: &Path) -> Value {
    index::status(root, EmbedderChoice::Fixture).expect("status reads")
}

fn measure_cid(root: &Path) -> String {
    let measure: IndexMeasure = serde_json::from_value(
        serde_json::from_slice(&std::fs::read(root.join(MEASURE_REL)).unwrap()).unwrap(),
    )
    .unwrap();
    measure.cid().unwrap().to_string()
}

fn fixture_dir(root: &Path) -> std::path::PathBuf {
    index::store_dir(root, &measure_cid(root), EmbedderChoice::Fixture)
}

fn store(root: &Path) -> rusqlite::Connection {
    rusqlite::Connection::open(fixture_dir(root).join("fold.sqlite")).expect("the store opens")
}

fn count(conn: &rusqlite::Connection, sql: &str, path: &str) -> i64 {
    conn.query_row(sql, [path], |row| row.get(0)).unwrap()
}

fn scalar(conn: &rusqlite::Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0)).unwrap()
}

fn attestations(root: &Path) -> Vec<FoldAttestation> {
    std::fs::read_to_string(fixture_dir(root).join("attestations.jsonl"))
        .expect("the attestation log reads")
        .lines()
        .map(|line| serde_json::from_str(line).expect("one attestation per line"))
        .collect()
}

fn git_commit_all(root: &Path) {
    common::git(root, &["init", "-q"]);
    common::git(root, &["add", "-A"]);
    common::git(root, &["commit", "-q", "-m", "fixture"]);
}

/// The first fold over a fresh tree attests `complete`: every admitted file is folded (the
/// negated directory and the `.txt` are not), the shard's atom count is the live chunk count,
/// the heads name every include pattern, and the attestation is both the latest snapshot and the
/// first line of the append-only log.
#[test]
fn the_first_fold_attests_complete() {
    let dir = tree();
    let root = dir.path();
    let report = fold(root);
    assert_eq!(report.attestation.state, FoldState::Complete);
    assert_eq!(report.lag, Some(0));
    assert_eq!(
        report.folded,
        [
            ".claude/notes.md",
            "genesis/alpha.md",
            "genesis/beta.md",
            "genesis/tool.py"
        ]
    );
    assert_eq!(report.attestation.attested_by.0, UNCLAIMED);
    assert_eq!(report.attestation.measure.to_string(), measure_cid(root));
    assert_eq!(report.attestation.heads_at.len(), 3, "one head per include");
    assert!(report.attestation.shard.arc.is_none());

    let conn = store(root);
    let live_chunks = scalar(
        &conn,
        "SELECT count(*) FROM chunks WHERE demoted_at IS NULL",
    );
    assert_eq!(report.attestation.shard.atoms, live_chunks as u64);
    let vector: Vec<u8> = conn
        .query_row(
            "SELECT vector FROM chunks WHERE path = 'genesis/alpha.md' LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(vector.len(), 384 * 4, "384 little-endian f32s per chunk");

    let latest: FoldAttestation =
        serde_json::from_slice(&std::fs::read(fixture_dir(root).join("attestation.json")).unwrap())
            .unwrap();
    assert_eq!(latest, report.attestation);
    assert_eq!(attestations(root), vec![report.attestation.clone()]);

    let seen = status(root);
    assert_eq!(seen["lag"], 0);
    assert_eq!(seen["unreadable"], 0);
    assert_eq!(seen["chunks"], live_chunks);
    assert_eq!(seen["source"], "walk");
    assert_eq!(seen["last"]["state"]["state"], "complete");
    assert_eq!(seen["last"]["attested_by"], UNCLAIMED);
}

/// Editing one file puts the fold one file behind; the next fold re-embeds exactly that file,
/// demotes its superseded chunks, and only the include holding it names a moved head.
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
    assert_eq!(second.folded, ["genesis/beta.md"]);
    assert_eq!(second.attestation.state, FoldState::Complete);
    assert_eq!(status(root)["lag"], 0);

    let conn = store(root);
    assert!(
        count(
            &conn,
            "SELECT count(*) FROM chunks WHERE path = ?1 AND demoted_at IS NOT NULL",
            "genesis/beta.md",
        ) > 0,
        "the superseded chunks are demoted, not deleted"
    );

    let moved: Vec<bool> = first
        .attestation
        .heads_at
        .iter()
        .zip(&second.attestation.heads_at)
        .map(|(a, b)| {
            assert_eq!(a.0, b.0, "the same include, in the same order");
            a.1 != b.1
        })
        .collect();
    assert_eq!(moved, [true, false, false], "only genesis/**/*.md moved");
    assert_eq!(attestations(root).len(), 2, "attestations are appended");
}

/// FTS5 holds live chunks only: after an edit, a term only the new text carries matches exactly
/// the live row, a term only the superseded text carried matches nothing, and the demoted rows
/// themselves are still in `chunks`. A chunk's text cannot be rewritten in place.
#[test]
fn the_lexical_index_holds_live_chunks_only() {
    let dir = tree();
    let root = dir.path();
    fold(root);
    common::write(root, "genesis/beta.md", "# Beta\nAn orchard, planted.\n");
    fold(root);
    let conn = store(root);
    let live_hits: Vec<(i64, Option<i64>)> = conn
        .prepare(
            "SELECT chunks.id, chunks.demoted_at FROM chunks_fts \
             JOIN chunks ON chunks.id = chunks_fts.rowid WHERE chunks_fts MATCH 'orchard'",
        )
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(live_hits.len(), 1);
    assert_eq!(live_hits[0].1, None, "the hit is the live row");
    assert_eq!(
        scalar(
            &conn,
            "SELECT count(*) FROM chunks_fts WHERE chunks_fts MATCH 'stewardship'"
        ),
        0,
        "the demoted text left the lexical index"
    );
    assert!(
        scalar(
            &conn,
            "SELECT count(*) FROM chunks WHERE text LIKE '%Stewardship%' AND demoted_at IS NOT NULL"
        ) > 0,
        "the demoted row itself remains"
    );
    assert!(conn
        .execute(
            "UPDATE chunks SET text = 'rewritten' WHERE path = 'genesis/beta.md'",
            []
        )
        .is_err());
    assert!(conn
        .execute(
            "UPDATE chunks SET demoted_at = NULL WHERE demoted_at IS NOT NULL",
            []
        )
        .is_err());
}

/// A deleted file's chunks are demoted — they stay in the store with `demoted_at` and leave the
/// live set a ranking reads — and the store refuses a delete outright. When the file returns it
/// is folded again, as new rows.
#[test]
fn a_deleted_file_is_demoted_and_a_returning_one_is_refolded() {
    let dir = tree();
    let root = dir.path();
    let first = fold(root);
    let original = std::fs::read(root.join("genesis/alpha.md")).unwrap();
    std::fs::remove_file(root.join("genesis/alpha.md")).unwrap();
    assert_eq!(status(root)["lag"], 1);

    let second = fold(root);
    assert_eq!(second.demoted, ["genesis/alpha.md"]);
    assert_eq!(second.attestation.state, FoldState::Complete);
    assert!(second.attestation.shard.atoms < first.attestation.shard.atoms);
    let conn = store(root);
    let live = "SELECT count(*) FROM chunks WHERE path = ?1 AND demoted_at IS NULL";
    assert_eq!(count(&conn, live, "genesis/alpha.md"), 0);
    assert!(
        count(
            &conn,
            "SELECT count(*) FROM chunks WHERE path = ?1 AND demoted_at IS NOT NULL",
            "genesis/alpha.md",
        ) > 0
    );
    assert!(conn.execute("DELETE FROM chunks", []).is_err());
    assert!(conn.execute("DELETE FROM files", []).is_err());
    assert_eq!(
        status(root)["demoted"],
        scalar(
            &conn,
            "SELECT count(*) FROM chunks WHERE demoted_at IS NOT NULL"
        )
    );

    std::fs::write(root.join("genesis/alpha.md"), original).unwrap();
    assert_eq!(status(root)["lag"], 1, "a returning file is behind");
    let third = fold(root);
    assert_eq!(third.folded, ["genesis/alpha.md"]);
    assert_eq!(third.attestation.state, FoldState::Complete);
    assert!(count(&conn, live, "genesis/alpha.md") > 0);
    assert_eq!(
        count(
            &conn,
            "SELECT count(*) FROM files WHERE path = ?1 AND demoted_at IS NULL",
            "genesis/alpha.md"
        ),
        1
    );
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

/// A `--scope` run folds only under its directory, and cannot attest `complete` while the surface
/// is still behind elsewhere.
#[test]
fn a_scoped_run_cannot_attest_complete_while_the_surface_is_behind() {
    let dir = tree();
    let root = dir.path();
    fold(root);
    common::write(root, "genesis/alpha.md", "# Alpha\nchanged\n");
    common::write(root, ".claude/notes.md", "# Notes\nchanged\n");
    let scoped = fold_with(
        root,
        FoldOptions {
            scope: Some("genesis".into()),
            ..options()
        },
    );
    assert_eq!(scoped.folded, ["genesis/alpha.md"]);
    assert_eq!(scoped.lag, Some(1));
    assert!(matches!(
        scoped.attestation.state,
        FoldState::Degraded { .. }
    ));
}

/// The per-run file cap is declared in the contract (`limits.fold_files_per_run`), never coded:
/// a contract declaring 2 folds two files a run.
#[test]
fn the_default_run_cap_is_the_contract_declaration() {
    let dir = tree();
    let root = dir.path();
    let mut contract = live(CONTRACT_REL);
    contract["limits"]["fold_files_per_run"] = json!(2);
    put_json(root, CONTRACT_REL, &contract);
    let report = fold(root);
    assert_eq!(report.folded.len(), 2);
    assert_eq!(report.attestation.state, FoldState::Degraded { retried: 0 });
}

/// An empty file and a non-UTF-8 file are seen (recorded with no chunk) and let the fold reach
/// `complete`; a second fold finds nothing behind.
#[test]
fn empty_and_non_utf8_files_reach_complete() {
    let dir = tree();
    let root = dir.path();
    common::write(root, "genesis/empty.md", "");
    std::fs::write(
        root.join("genesis/latin1.md"),
        [0x23, 0x20, 0xe9, 0xff, 0x0a],
    )
    .unwrap();
    let report = fold(root);
    assert_eq!(report.attestation.state, FoldState::Complete);
    assert_eq!(report.skipped, ["genesis/latin1.md"]);
    assert!(report.folded.contains(&"genesis/empty.md".to_string()));
    assert_eq!(status(root)["lag"], 0);
    let again = fold(root);
    assert!(again.folded.is_empty());
    assert_eq!(again.attestation.state, FoldState::Complete);
}

/// `status` never re-hashes an unchanged surface: a file whose size and mtime still match its row
/// is trusted (the same bargain git's index makes), and one whose stat moved is re-hashed.
#[test]
fn an_unchanged_stat_is_not_rehashed() {
    let dir = tree();
    let root = dir.path();
    fold(root);
    let path = root.join("genesis/alpha.md");
    let before = std::fs::metadata(&path).unwrap().modified().unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replace("stale", "STALE")).unwrap();
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(before)
        .unwrap();
    assert_eq!(
        status(root)["lag"],
        0,
        "same size, same mtime: not re-hashed"
    );

    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(before + std::time::Duration::from_secs(5))
        .unwrap();
    assert_eq!(
        status(root)["lag"],
        1,
        "a moved stat is re-hashed and found changed"
    );

    // Touching a file without changing it is a restat, never a re-embed.
    fold(root);
    let untouched = root.join("genesis/beta.md");
    let bumped = std::fs::metadata(&untouched).unwrap().modified().unwrap()
        + std::time::Duration::from_secs(5);
    std::fs::File::options()
        .write(true)
        .open(&untouched)
        .unwrap()
        .set_modified(bumped)
        .unwrap();
    assert_eq!(status(root)["lag"], 0);
    assert!(fold(root).folded.is_empty());
}

/// The private chain never enters a fold: with the contract's own exclusions a recall store and
/// a worktree under a surface stay out; and with a contract that omits `.eprfs` and `worktrees`
/// from `exclude_directories` entirely, the floor still refuses them.
#[test]
fn the_private_recall_store_and_worktrees_are_never_folded() {
    let dir = tree();
    let root = dir.path();
    let mut contract = live(CONTRACT_REL);
    let excluded = contract["discovery"]["exclude_directories"]
        .as_array_mut()
        .unwrap();
    excluded.retain(|d| d != ".eprfs" && d != "worktrees");
    put_json(root, CONTRACT_REL, &contract);
    put_measure(root, &["**/*.md"]);
    common::write(
        root,
        ".eprfs/status/recall/s1/receipt.md",
        "# Receipt\nprivate reasoning\n",
    );
    common::write(
        root,
        ".claude/worktrees/agent-a/CLAUDE.md",
        "# Another checkout\n",
    );
    common::write(root, "app/worktrees/x/notes.md", "# Another checkout\n");

    let report = fold(root);
    assert_eq!(report.attestation.state, FoldState::Complete);
    let conn = store(root);
    let leaked = scalar(
        &conn,
        "SELECT count(*) FROM files WHERE path LIKE '.eprfs/status/recall/%' \
         OR path LIKE '%worktrees/%'",
    );
    assert_eq!(leaked, 0, "{:?}", report.folded);
    assert!(report.folded.contains(&".claude/notes.md".to_string()));
}

/// On a git checkout the candidates are the tracked files: an untracked note is not declared
/// source and is never folded.
#[test]
fn a_git_checkout_folds_tracked_files_only() {
    let dir = tree();
    let root = dir.path();
    git_commit_all(root);
    common::write(root, "genesis/untracked.md", "# Untracked\nderived state\n");
    let report = fold(root);
    assert_eq!(report.attestation.state, FoldState::Complete);
    assert!(!report.folded.contains(&"genesis/untracked.md".to_string()));
    assert!(report.folded.contains(&"genesis/alpha.md".to_string()));
    assert_eq!(status(root)["source"], "git-tracked");
}

/// A candidate that exists but cannot be read is held, not demoted: it is counted `unreadable`,
/// its rows stay live, and the fold attests `degraded` until it reads again.
#[test]
fn an_unreadable_file_is_held_not_demoted() {
    let dir = tree();
    let root = dir.path();
    git_commit_all(root);
    fold(root);
    let path = root.join("genesis/beta.md");
    let original = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap(); // tracked, present, not a readable file

    let held = fold(root);
    assert_eq!(held.unreadable, 1);
    assert!(held.demoted.is_empty());
    assert_eq!(held.attestation.state, FoldState::Degraded { retried: 0 });
    let conn = store(root);
    assert!(
        count(
            &conn,
            "SELECT count(*) FROM chunks WHERE path = ?1 AND demoted_at IS NULL",
            "genesis/beta.md"
        ) > 0,
        "its rows stay current"
    );
    assert_eq!(status(root)["unreadable"], 1);

    std::fs::remove_dir(&path).unwrap();
    std::fs::write(&path, original).unwrap();
    let back = fold(root);
    assert_eq!(back.attestation.state, FoldState::Complete);
    assert!(
        back.folded.is_empty(),
        "the same bytes are a restat, not a refold"
    );
}

/// The method is the declaration: a chunk rule on disk that no longer hashes to the measure's
/// `chunkRule` is attested `failed`, and nothing is folded under it.
#[test]
fn a_chunk_rule_that_does_not_hash_to_the_measure_fails_the_fold() {
    let dir = tree();
    let root = dir.path();
    let mut measure = put_measure(root, &FIXTURE_SURFACE);
    measure["_chunk_rule"]["max_chunks_per_file"] = json!(41);
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
/// `unavailable: <reason>` and exits 0 — the fold ran and said what it found — and the pinned
/// `status` shows the failed state.
#[test]
fn an_unavailable_embedder_is_attested_failed_and_exits_zero() {
    use elohim_epr_cli::flow::memory::recall::embedder::{MODEL_MANIFEST_REL, PROCEDURE_REL};
    let dir = tree();
    let root = dir.path();
    let procedure = root.join(PROCEDURE_REL);
    std::fs::create_dir_all(procedure.parent().unwrap()).unwrap();
    std::fs::copy(common::repo_root().join(PROCEDURE_REL), &procedure).unwrap();
    let mut manifest = live(MODEL_MANIFEST_REL);
    manifest["resolve"] = json!([root.join("no-model-here").to_string_lossy()]);
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
    let seen = index::status(root, EmbedderChoice::Pinned).unwrap();
    assert_eq!(seen["embedder"], "pinned");
    assert_eq!(seen["last"]["state"]["state"], "failed");
    let why = seen["last"]["state"]["why"].as_str().unwrap();
    assert!(
        why.starts_with("unavailable: no model directory resolves"),
        "{why}"
    );
}

/// Fixture and live folds never share a store or a log: a fixture fold leaves the pinned store
/// byte-untouched, and a fixture status never prints a model CID as if it were live.
#[test]
fn fixture_and_pinned_stores_never_share() {
    let dir = tree();
    let root = dir.path();
    let pinned = index::store_dir(root, &measure_cid(root), EmbedderChoice::Pinned);
    let files = [
        ("fold.sqlite", b"pinned store bytes".as_slice()),
        ("attestation.json", b"{\"pinned\": true}\n".as_slice()),
        ("attestations.jsonl", b"{\"pinned\": 1}\n".as_slice()),
    ];
    std::fs::create_dir_all(&pinned).unwrap();
    for (name, bytes) in files {
        std::fs::write(pinned.join(name), bytes).unwrap();
    }
    fold(root);
    for (name, bytes) in files {
        assert_eq!(std::fs::read(pinned.join(name)).unwrap(), bytes, "{name}");
    }
    assert!(fixture_dir(root).join("fold.sqlite").is_file());
    assert_ne!(fixture_dir(root), pinned);

    let fixture = status(root);
    assert_eq!(fixture["embedder"], "fixture");
    assert!(fixture["model"].is_null());
    let live_status = index::status(root, EmbedderChoice::Pinned).unwrap();
    assert_eq!(live_status["embedder"], "pinned");
    assert!(live_status["model"].is_string());
}

/// One fold at a time: while another holds `fold.lock`, a fold reports busy, exits 0 and writes
/// no attestation.
#[test]
fn a_second_concurrent_fold_is_busy_and_writes_nothing() {
    let dir = tree();
    let root = dir.path();
    let store_dir = fixture_dir(root);
    std::fs::create_dir_all(&store_dir).unwrap();
    let lock = std::fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(store_dir.join("fold.lock"))
        .unwrap();
    rustix::fs::flock(&lock, rustix::fs::FlockOperation::NonBlockingLockExclusive).unwrap();

    assert!(matches!(
        index::fold(root, &options()).unwrap(),
        FoldRun::Busy
    ));
    let out = Command::new(env!("CARGO_BIN_EXE_epr"))
        .args([
            "flow",
            "memory",
            "index",
            "fold",
            "--embedder",
            "fixture",
            "--root",
        ])
        .arg(root)
        .output()
        .expect("epr runs");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), BUSY);
    assert!(!store_dir.join("attestations.jsonl").exists());
    assert!(!store_dir.join("attestation.json").exists());

    drop(lock);
    assert_eq!(fold(root).attestation.state, FoldState::Complete);
}

/// A corrupt store is rebuilt from scratch, never repaired by hand.
#[test]
fn a_corrupt_store_is_rebuilt() {
    let dir = tree();
    let root = dir.path();
    fold(root);
    std::fs::write(
        fixture_dir(root).join("fold.sqlite"),
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
    let mut got: Vec<&str> = seen
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    got.sort_unstable();
    let mut expected = vec![
        "measure",
        "embedder",
        "source",
        "model",
        "chunk_rule",
        "chunks",
        "demoted",
        "bytes",
        "lag",
        "unreadable",
        "last",
    ];
    expected.sort_unstable();
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
    git_commit_all(root);
    elohim_epr_cli::actor::claim(root, "agent:implementer@opus-5.5", "fold-session")
        .expect("the claim registers");
    let claimed = fold_with(
        root,
        FoldOptions {
            session: Some("fold-session".into()),
            ..options()
        },
    );
    assert_eq!(
        claimed.attestation.attested_by.0,
        "agent:implementer@opus-5.5"
    );

    common::write(root, "genesis/beta.md", "# Beta\nagain\n");
    let other = fold_with(
        root,
        FoldOptions {
            session: Some("nobody-claimed-this".into()),
            ..options()
        },
    );
    assert_eq!(other.attestation.attested_by.0, UNCLAIMED);
}

/// Contract v20's method CID — the recipe before task 4.6 declared the fold's listing budget.
const V20_METHOD_CID: &str = "bafkreie3ckteeb3p66smgm4t7ykv3s56dtjwa6tnumaixtm4bf6u4fitky";

/// Task 4.6 (the listing budget 4.3 deferred): the fold's `git ls-files` read is charged to its
/// OWN declared limit, `limits.fold_listing_bytes`, not the discovery traversal's `scan_bytes` —
/// a listing over the fold's budget attests `failed` naming the listing, and a discovery budget
/// too small for the listing no longer touches the fold.
#[test]
fn the_listing_charges_fold_listing_bytes_not_the_discovery_scan_budget() {
    let dir = tree();
    let root = dir.path();
    git_commit_all(root);

    let mut contract = live(CONTRACT_REL);
    contract["limits"]["fold_listing_bytes"] = json!(8);
    contract["limits"]["scan_bytes"] = json!(1_073_741_824);
    put_json(root, CONTRACT_REL, &contract);
    let starved = fold(root);
    match &starved.attestation.state {
        FoldState::Failed { why } => assert!(why.contains("git ls-files"), "{why}"),
        other => panic!("a listing over fold_listing_bytes fails the fold: {other:?}"),
    }

    contract["limits"]["fold_listing_bytes"] = json!(4_194_304);
    contract["limits"]["scan_bytes"] = json!(8);
    put_json(root, CONTRACT_REL, &contract);
    let fed = fold(root);
    assert_eq!(
        fed.attestation.state,
        FoldState::Complete,
        "the discovery scan budget no longer bounds the fold's listing"
    );
}

/// Contract v21 declares the fold's listing budget (`limits.fold_listing_bytes: 4194304`) and the
/// bank is re-pinned to the new recipe.
#[test]
fn contract_v21_declares_the_fold_listing_budget_and_repins_the_bank() {
    let root = common::repo_root();
    let contract = elohim_epr_cli::flow::memory::recall::Contract::load(&root.join(CONTRACT_REL))
        .expect("live contract loads");
    let value = common::live_contract();
    assert!(value["version"].as_u64() >= Some(21));
    assert_eq!(value["limits"]["fold_listing_bytes"], 4_194_304);
    let method = contract.method_cid();
    assert_ne!(method, V20_METHOD_CID, "the contract's bytes moved");
    let bank = live(value["question_bank"].as_str().expect("bank"));
    assert_eq!(bank["recipe"].as_str(), Some(method.as_str()));
    contract
        .question_bank()
        .expect("every question is in scope of the v21 recipe");
}
