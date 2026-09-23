//! INDEX — the semantic fold and its status (governed-discovery station 4, task 4.3):
//! `epr flow memory index fold|status`.
//!
//! The fold executes `recall-semantic-index@1` (the `IndexMeasure` the contract's `semantic`
//! provider names) against the tree: it walks the measure's surfaces under the contract's
//! `discovery.exclude_directories` (at every depth — the private recall store under `.eprfs` and a
//! sibling checkout under `worktrees` never enter) and `first_screen_globs`, fingerprints each
//! file (the raw CID of its bytes — the cites convention's body CID), and diffs against its
//! derived store. Only new and changed files are re-chunked (by the declared rule, `chunk.rs`) and
//! re-embedded, at most `limits.fold_files_per_run` a run; removed and superseded chunks are
//! demoted with a timestamp, never deleted (`Retention` has no delete, and the store's own trigger
//! refuses one). Every run ends in a [`FoldAttestation`]: `complete` when nothing is left behind,
//! `degraded` when the run cap stopped it, `failed` when the method or the procedure refused —
//! the latest at `attestation.json` and every one appended to `attestations.jsonl`.
//!
//! The store is `.eprfs/status/index/<measure-cid>/fold.sqlite` — derived and gitignored: a
//! missing, corrupt or foreign store is rebuilt from scratch, never repaired by hand.
use super::chunk::ChunkRule;
use super::discovery::{atom_search_excluded, first_screen_globs};
use super::embedder::{EmbedBudget, Embedder, Fixture, PinnedProcedure};
use super::*;
use cid::Cid;
use elohim_epr_rea::{atom_cid, AgentRef, FoldAttestation, FoldState, IndexMeasure, ShardManifest};
use rusqlite::{params, Connection, OpenFlags};
use serde::Serialize;

/// Every semantic fold's store and attestations live under here, one directory per measure CID.
pub const INDEX_DIR_REL: &str = ".eprfs/status/index";

/// Who attested a fold when no session claim names anyone — the honest literal, never a guess.
pub const UNCLAIMED: &str = "(unclaimed)";

const STORE_FILE: &str = "fold.sqlite";
const LATEST_FILE: &str = "attestation.json";
const LOG_FILE: &str = "attestations.jsonl";
const ACTOR_LOG_REL: &str = ".eprfs/status/actors.jsonl";
const CHUNK_RULE_MISMATCH: &str = "chunk rule on disk does not hash to the measure";

/// Bumped when the tables below change shape; a store of another version is rebuilt.
const SCHEMA_VERSION: &str = "1";

/// The whole store, in one place. `files` is the fold's manifest of what it has seen (a file that
/// yields no chunk — empty, or skipped as non-UTF-8 — is still seen, so it is not behind forever);
/// `chunks` holds each chunk's text and its vector (little-endian `f32` × the pinned dims); the
/// external-content FTS5 table indexes the same rows for the lexical provider, which — like every
/// ranking — joins back on `demoted_at IS NULL`.
const SCHEMA: &str = "
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE files (
  path TEXT PRIMARY KEY,
  fingerprint TEXT NOT NULL,
  chunks INTEGER NOT NULL,
  skipped TEXT,
  folded_at INTEGER NOT NULL,
  demoted_at INTEGER
);
CREATE TABLE chunks (
  id INTEGER PRIMARY KEY,
  path TEXT NOT NULL,
  section TEXT NOT NULL,
  ordinal INTEGER NOT NULL,
  fingerprint TEXT NOT NULL,
  text TEXT NOT NULL,
  vector BLOB NOT NULL,
  folded_at INTEGER NOT NULL,
  demoted_at INTEGER
);
CREATE INDEX chunks_by_path ON chunks(path, demoted_at);
CREATE VIRTUAL TABLE chunks_fts USING fts5(
  text, path UNINDEXED, section UNINDEXED, content='chunks', content_rowid='id'
);
CREATE TRIGGER chunks_fts_insert AFTER INSERT ON chunks BEGIN
  INSERT INTO chunks_fts(rowid, text, path, section) VALUES (new.id, new.text, new.path, new.section);
END;
CREATE TRIGGER chunks_never_deleted BEFORE DELETE ON chunks BEGIN
  SELECT RAISE(ABORT, 'demotion, never deletion');
END;
CREATE TRIGGER files_never_deleted BEFORE DELETE ON files BEGIN
  SELECT RAISE(ABORT, 'demotion, never deletion');
END;
";

fn index_refused(message: impl Into<String>) -> FlowError {
    FlowError::InvalidArguments(format!("semantic index: {}", message.into()))
}

fn db(error: rusqlite::Error) -> FlowError {
    FlowError::Io(std::io::Error::other(format!("fold store: {error}")))
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// Where a measure's store and attestations live.
pub fn store_dir(root: &Path, measure_cid: &str) -> PathBuf {
    root.join(INDEX_DIR_REL).join(measure_cid)
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The declaration
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// The contract, the measure it names, the measure file's raw value (for `_chunk_rule`) and CID.
struct Declared {
    contract: Contract,
    measure: IndexMeasure,
    raw: Value,
    cid: Cid,
}

impl Declared {
    fn load(root: &Path) -> FlowResult<Self> {
        let contract = Contract::load(&root.join(CONTRACT_REL))?;
        let rel = contract
            .value
            .pointer("/ceremony/providers/semantic/measure")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                index_refused("the recall contract names no semantic measure (ceremony.providers.semantic.measure)")
            })?
            .to_string();
        let path = root.join(&rel);
        let bytes = std::fs::read(&path).map_err(|source| FlowError::Read { path, source })?;
        let raw: Value = serde_json::from_slice(&bytes)?;
        let measure: IndexMeasure = serde_json::from_value(raw.clone())?;
        measure
            .validate()
            .map_err(|error| index_refused(format!("{rel}: {error}")))?;
        let cid = measure.cid()?;
        Ok(Self {
            contract,
            measure,
            raw,
            cid,
        })
    }

    fn model(&self) -> String {
        self.measure
            .embedding
            .as_ref()
            .map(|pin| pin.model_bytes.to_string())
            .unwrap_or_default()
    }

    fn dims(&self) -> usize {
        self.measure
            .embedding
            .as_ref()
            .map_or(0, |pin| pin.dims as usize)
    }

    /// The method is the declaration: the rule object on disk must hash to `chunkRule`.
    fn chunk_rule(&self) -> Result<ChunkRule, String> {
        let rule = self.raw.get("_chunk_rule").unwrap_or(&Value::Null);
        match atom_cid(rule) {
            Ok(cid) if cid == self.measure.chunk_rule => {}
            _ => return Err(CHUNK_RULE_MISMATCH.to_string()),
        }
        ChunkRule::from_declared(rule).map_err(|error| error.to_string())
    }

    fn run_cap(&self) -> FlowResult<usize> {
        Ok(self.contract.positive_limit("fold_files_per_run")? as usize)
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The surface walk
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Every admitted file under the measure's surfaces, repository-relative and sorted. An excluded
/// directory name anywhere in a path — a surface root's own included — keeps it out; symlinks
/// are not followed.
fn walk(root: &Path, declared: &Declared) -> FlowResult<Vec<String>> {
    let excluded = atom_search_excluded(&declared.contract);
    let mut globs = Vec::new();
    for pattern in first_screen_globs(&declared.contract) {
        globs.push(glob::Pattern::new(&pattern).map_err(|_| {
            index_refused(format!(
                "first_screen_globs holds an invalid glob {pattern}"
            ))
        })?);
    }
    let excluded_path = |rel: &str| rel.split('/').any(|part| excluded.contains(part));
    let admitted = |name: &str| globs.iter().any(|g| g.matches(name));
    let mut found = BTreeSet::new();
    for surface in declared.measure.surfaces.paths() {
        let surface = surface.trim_end_matches('/');
        if surface.is_empty()
            || surface.starts_with('/')
            || surface.split('/').any(|part| part == "..")
            || excluded_path(surface)
        {
            continue;
        }
        let base = root.join(surface);
        let Ok(meta) = std::fs::symlink_metadata(&base) else {
            continue;
        };
        if meta.is_file() {
            let name = surface.rsplit('/').next().unwrap_or(surface);
            if admitted(name) {
                found.insert(surface.to_string());
            }
            continue;
        }
        if !meta.is_dir() {
            continue;
        }
        let mut stack = vec![(base, surface.to_string())];
        while let Some((dir, rel)) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                    continue;
                };
                let Ok(kind) = entry.file_type() else {
                    continue;
                };
                let child = format!("{rel}/{name}");
                if kind.is_dir() {
                    if !excluded.contains(&name) {
                        stack.push((entry.path(), child));
                    }
                } else if kind.is_file() && admitted(&name) {
                    found.insert(child);
                }
            }
        }
    }
    Ok(found.into_iter().collect())
}

/// The raw CID of a file's bytes, stored as its string.
fn fingerprint(bytes: &[u8]) -> String {
    BlobCid::compute_raw(bytes).to_string()
}

/// What the tree holds against what the store holds.
#[derive(Default)]
struct Plan {
    /// New or changed files, sorted.
    behind: Vec<String>,
    /// Live in the store, gone from the tree.
    removed: Vec<String>,
    /// Admitted files that could not be read to fingerprint (counted, not guessed).
    unreadable: usize,
}

impl Plan {
    fn lag(&self) -> usize {
        self.behind.len() + self.removed.len()
    }
}

fn plan(root: &Path, declared: &Declared, store: &Store) -> FlowResult<Plan> {
    let stored = store.live_files()?;
    let mut plan = Plan::default();
    let mut present = BTreeSet::new();
    for rel in walk(root, declared)? {
        let Ok(bytes) = std::fs::read(root.join(&rel)) else {
            plan.unreadable += 1;
            continue;
        };
        if stored.get(&rel) != Some(&fingerprint(&bytes)) {
            plan.behind.push(rel.clone());
        }
        present.insert(rel);
    }
    plan.removed = stored
        .keys()
        .filter(|path| !present.contains(*path))
        .cloned()
        .collect();
    Ok(plan)
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The store
// ───────────────────────────────────────────────────────────────────────────────────────────────

struct Store {
    conn: Connection,
}

/// The identity a store was built under; any difference is a different fold.
fn meta_rows(declared: &Declared, embedder: &str) -> Vec<(&'static str, String)> {
    vec![
        ("schema", SCHEMA_VERSION.to_string()),
        ("measure", declared.cid.to_string()),
        ("model", declared.model()),
        ("chunk_rule", declared.measure.chunk_rule.to_string()),
        ("embedder", embedder.to_string()),
    ]
}

impl Store {
    fn meta(&self) -> FlowResult<BTreeMap<String, String>> {
        let mut statement = self
            .conn
            .prepare("SELECT key, value FROM meta")
            .map_err(db)?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(db)?;
        rows.collect::<Result<_, _>>().map_err(db)
    }

    /// A readable store of this measure, without writing anything: `Ok(None)` when there is no
    /// file, `Err(why)` when there is one that cannot serve (the next fold rebuilds it).
    fn open_existing(path: &Path, declared: &Declared) -> Result<Option<Self>, String> {
        if !path.is_file() {
            return Ok(None);
        }
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)
            .map_err(|error| format!("store unreadable: {error}"))?;
        let store = Self { conn };
        let check: String = store
            .conn
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(|error| format!("store unreadable: {error}"))?;
        if check != "ok" {
            return Err(format!("store fails its integrity check: {check}"));
        }
        let meta = store
            .meta()
            .map_err(|error| format!("store unreadable: {error}"))?;
        if meta.get("schema").map(String::as_str) != Some(SCHEMA_VERSION) {
            return Err("store was built under another schema".to_string());
        }
        if meta.get("measure") != Some(&declared.cid.to_string()) {
            return Err("store was built under another measure".to_string());
        }
        Ok(Some(store))
    }

    /// The store this fold writes to, and what happened to get it: `created`, `reused`, or
    /// `rebuilt (<why>)` — a store that cannot serve is replaced, never patched.
    fn open_or_rebuild(
        path: &Path,
        declared: &Declared,
        embedder: &str,
    ) -> FlowResult<(Self, String)> {
        let wanted: BTreeMap<String, String> = meta_rows(declared, embedder)
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        let why = match Self::open_existing(path, declared) {
            Ok(None) => None,
            Ok(Some(store)) => match store.meta() {
                Ok(meta) if meta == wanted => return Ok((store, "reused".into())),
                Ok(_) => Some("store was folded under another embedder".to_string()),
                Err(error) => Some(error.to_string()),
            },
            Err(why) => Some(why),
        };
        let outcome = match &why {
            Some(why) => {
                for suffix in ["", "-journal", "-wal", "-shm"] {
                    let stale = PathBuf::from(format!("{}{suffix}", path.display()));
                    if stale.exists() {
                        std::fs::remove_file(&stale)?;
                    }
                }
                format!("rebuilt ({why})")
            }
            None => "created".to_string(),
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut conn = Connection::open(path).map_err(db)?;
        let tx = conn.transaction().map_err(db)?;
        tx.execute_batch(SCHEMA).map_err(db)?;
        for (key, value) in &wanted {
            tx.execute(
                "INSERT INTO meta (key, value) VALUES (?1, ?2)",
                params![key, value],
            )
            .map_err(db)?;
        }
        tx.commit().map_err(db)?;
        Ok((Self { conn }, outcome))
    }

    /// `path -> fingerprint` for every file the fold currently holds.
    fn live_files(&self) -> FlowResult<BTreeMap<String, String>> {
        let mut statement = self
            .conn
            .prepare("SELECT path, fingerprint FROM files WHERE demoted_at IS NULL")
            .map_err(db)?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(db)?;
        rows.collect::<Result<_, _>>().map_err(db)
    }

    fn count(&self, sql: &str) -> FlowResult<u64> {
        self.conn
            .query_row(sql, [], |row| row.get::<_, i64>(0))
            .map(|n| n.max(0) as u64)
            .map_err(db)
    }

    /// What the fold holds: live chunk count, their text bytes, and the CID of the sorted
    /// `(path, section, fingerprint)` manifest.
    fn shard(&self) -> FlowResult<ShardManifest> {
        let mut statement = self
            .conn
            .prepare("SELECT path, section, fingerprint, length(CAST(text AS BLOB)) FROM chunks WHERE demoted_at IS NULL")
            .map_err(db)?;
        let mut rows: Vec<(String, String, String)> = Vec::new();
        let mut bytes = 0u64;
        let mut query = statement.query([]).map_err(db)?;
        while let Some(row) = query.next().map_err(db)? {
            rows.push((
                row.get(0).map_err(db)?,
                row.get(1).map_err(db)?,
                row.get(2).map_err(db)?,
            ));
            bytes += row.get::<_, i64>(3).map_err(db)?.max(0) as u64;
        }
        rows.sort();
        Ok(ShardManifest {
            arc: None,
            atoms: rows.len() as u64,
            bytes,
            manifest: atom_cid(&rows)?,
        })
    }

    /// One `(root, head)` pair per declared surface root: the raw CID of the root's path bytes,
    /// and the CID of that root's sorted `(path, fingerprint)` list — so a later fold can name
    /// which root moved.
    fn heads(&self, declared: &Declared) -> FlowResult<Vec<(Cid, Cid)>> {
        let files = self.live_files()?;
        let mut heads = Vec::new();
        for surface in declared.measure.surfaces.paths() {
            let bare = surface.trim_end_matches('/');
            let under: Vec<(&String, &String)> = files
                .iter()
                .filter(|(path, _)| *path == bare || path.starts_with(&format!("{bare}/")))
                .collect();
            heads.push((
                *BlobCid::compute_raw(surface.as_bytes()).as_cid(),
                atom_cid(&under)?,
            ));
        }
        Ok(heads)
    }
}

/// An empty shard — what a fold that never wrote a store holds.
fn empty_shard() -> FlowResult<ShardManifest> {
    Ok(ShardManifest {
        arc: None,
        atoms: 0,
        bytes: 0,
        manifest: atom_cid(&Vec::<(String, String, String)>::new())?,
    })
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The fold
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Which [`Embedder`] a fold runs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EmbedderChoice {
    /// The declared procedure the model manifest pins (the default).
    #[default]
    Pinned,
    /// The deterministic test embedder — never a claim of live fitness.
    Fixture,
}

/// What one `index fold` may be told.
#[derive(Debug, Clone, Default)]
pub struct FoldOptions {
    /// Only files under this repository-relative directory are folded this run.
    pub scope: Option<String>,
    /// Overrides the contract's `limits.fold_files_per_run` for one run.
    pub max_files: Option<usize>,
    /// The session whose actor claim attests the fold.
    pub session: Option<String>,
    pub embedder: EmbedderChoice,
}

/// What a fold did, and the attestation it wrote.
#[derive(Debug, Clone, Serialize)]
pub struct FoldReport {
    pub measure: String,
    /// `created`, `reused`, or `rebuilt (<why>)`; empty when the fold failed before the store.
    pub store: String,
    pub embedder: String,
    /// Files re-chunked and re-embedded this run (or skipped as non-UTF-8), sorted.
    pub folded: Vec<String>,
    pub embedded_chunks: usize,
    /// Files gone from the tree whose chunks this run demoted.
    pub demoted: Vec<String>,
    /// Folded files skipped as binary or non-UTF-8.
    pub skipped: Vec<String>,
    /// Admitted files that could not be read to fingerprint.
    pub unreadable: usize,
    /// Chunks past `max_chunks_per_file`, dropped and counted.
    pub dropped_chunks: usize,
    /// Files still behind after this run; `None` when the fold failed before measuring.
    pub lag: Option<usize>,
    pub attestation: FoldAttestation,
    pub attestation_cid: String,
}

/// Who attests: the session's current claim in its participant form, or [`UNCLAIMED`].
fn attested_by(root: &Path, session: Option<&str>) -> AgentRef {
    let claimed = session
        .filter(|s| !s.trim().is_empty() && root.join(ACTOR_LOG_REL).is_file())
        .and_then(|s| crate::actor::current(root, s).ok())
        .and_then(|outcome| outcome.claim)
        .map(|claim| claim.claimed)
        .filter(|claimed| elohim_epr_rea::actor::parse_participant_ref(claimed).is_ok());
    AgentRef(claimed.unwrap_or_else(|| UNCLAIMED.to_string()))
}

fn embedder_for(
    root: &Path,
    choice: EmbedderChoice,
    declared: &Declared,
) -> FlowResult<(Box<dyn Embedder>, String)> {
    match choice {
        EmbedderChoice::Fixture => Ok((Box::new(Fixture), "fixture".to_string())),
        EmbedderChoice::Pinned => {
            let procedure = PinnedProcedure::declared(root)?;
            if procedure.manifest.model_bytes != declared.model() {
                return Err(FlowError::Unavailable(format!(
                    "the model manifest pins {}; the measure pins {}",
                    procedure.manifest.model_bytes,
                    declared.model()
                )));
            }
            let label = format!(
                "procedure {} on model {}",
                procedure.manifest.procedure.clone().unwrap_or_default(),
                procedure.manifest.model_bytes
            );
            Ok((Box::new(procedure), label))
        }
    }
}

/// The previous attestation's state decides `retried`: a capped run following a capped run is
/// the next retry of the same backlog.
fn next_retry(dir: &Path) -> u32 {
    std::fs::read(dir.join(LATEST_FILE))
        .ok()
        .and_then(|raw| serde_json::from_slice::<FoldAttestation>(&raw).ok())
        .map_or(0, |last| match last.state {
            FoldState::Degraded { retried } => retried + 1,
            _ => 0,
        })
}

/// Write the latest snapshot and append the act.
fn record(dir: &Path, attestation: &FoldAttestation) -> FlowResult<String> {
    std::fs::create_dir_all(dir)?;
    let cid = attestation.cid()?.to_string();
    let staged = dir.join(format!("{LATEST_FILE}.tmp"));
    std::fs::write(&staged, serde_json::to_string_pretty(attestation)? + "\n")?;
    std::fs::rename(&staged, dir.join(LATEST_FILE))?;
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(LOG_FILE))?;
    writeln!(log, "{}", serde_json::to_string(attestation)?)?;
    Ok(cid)
}

/// One chunk ready to write.
struct Pending {
    path: String,
    section: String,
    ordinal: usize,
    fingerprint: String,
    text: String,
}

/// A file this run folds: its fingerprint, and why it has no chunks if it was skipped.
struct FoldedFile {
    path: String,
    fingerprint: String,
    chunks: usize,
    skipped: Option<&'static str>,
}

fn vector_blob(vector: &[f32]) -> Vec<u8> {
    vector.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// `epr flow memory index fold`: fold what moved, at most the run cap, and attest the result.
/// A fold that ran always attests — `failed` included — and returns `Ok`; only an unreadable
/// declaration (no measure CID to attest under) is an `Err`.
pub fn fold(root: &Path, opts: &FoldOptions) -> FlowResult<FoldReport> {
    let declared = Declared::load(root)?;
    let dir = store_dir(root, &declared.cid.to_string());
    let by = attested_by(root, opts.session.as_deref());
    if let Some(scope) = &opts.scope {
        if scope.starts_with('/') || scope.split('/').any(|part| part == "..") {
            return Err(index_refused("--scope must be repository-relative"));
        }
    }
    let max_files = match opts.max_files {
        Some(0) => return Err(index_refused("--max-files must be positive")),
        Some(n) => n,
        None => declared.run_cap()?,
    };
    let mut report = FoldReport {
        measure: declared.cid.to_string(),
        store: String::new(),
        embedder: String::new(),
        folded: Vec::new(),
        embedded_chunks: 0,
        demoted: Vec::new(),
        skipped: Vec::new(),
        unreadable: 0,
        dropped_chunks: 0,
        lag: None,
        attestation: FoldAttestation {
            measure: declared.cid,
            shard: empty_shard()?,
            heads_at: Vec::new(),
            state: FoldState::Complete,
            attested_by: by,
            at: 0,
        },
        attestation_cid: String::new(),
    };

    // Before any store is touched: the method, then the procedure that will run it.
    let rule = match declared.chunk_rule() {
        Ok(rule) => rule,
        Err(why) => return failed_before_store(&declared, &dir, report, why),
    };
    let (embedder, label) = match embedder_for(root, opts.embedder, &declared) {
        Ok(pair) => pair,
        Err(error) => return failed_before_store(&declared, &dir, report, error.to_string()),
    };
    report.embedder = label.clone();
    let run = Run {
        root,
        declared: &declared,
        dir: &dir,
        rule,
        embedder: embedder.as_ref(),
        label: &label,
        opts,
        max_files,
    };
    run.fold(report)
}

/// A fold refused before it opened its store still attests, over whatever store already exists.
fn failed_before_store(
    declared: &Declared,
    dir: &Path,
    mut report: FoldReport,
    why: String,
) -> FlowResult<FoldReport> {
    if let Ok(Some(store)) = Store::open_existing(&dir.join(STORE_FILE), declared) {
        report.attestation.shard = store.shard()?;
        report.attestation.heads_at = store.heads(declared)?;
    }
    attest(dir, &mut report, FoldState::Failed { why })?;
    Ok(report)
}

fn attest(dir: &Path, report: &mut FoldReport, state: FoldState) -> FlowResult<()> {
    report.attestation.state = state;
    report.attestation.at = now();
    report.attestation_cid = record(dir, &report.attestation)?;
    Ok(())
}

/// One fold run, once the method and the embedder have both been checked.
struct Run<'a> {
    root: &'a Path,
    declared: &'a Declared,
    dir: &'a Path,
    rule: ChunkRule,
    embedder: &'a dyn Embedder,
    label: &'a str,
    opts: &'a FoldOptions,
    max_files: usize,
}

impl Run<'_> {
    fn fold(self, mut report: FoldReport) -> FlowResult<FoldReport> {
        let Run {
            root,
            declared,
            dir,
            rule,
            embedder,
            label,
            opts,
            max_files,
        } = self;
        let (mut store, outcome) = Store::open_or_rebuild(&dir.join(STORE_FILE), declared, label)?;
        report.store = outcome;
        let plan = plan(root, declared, &store)?;
        report.unreadable = plan.unreadable;
        let in_scope = |path: &String| match &opts.scope {
            Some(scope) => {
                let scope = scope.trim_end_matches('/');
                path == scope || path.starts_with(&format!("{scope}/"))
            }
            None => true,
        };
        let selected: Vec<String> = plan
            .behind
            .iter()
            .filter(|p| in_scope(p))
            .take(max_files)
            .cloned()
            .collect();
        let removed: Vec<String> = plan
            .removed
            .iter()
            .filter(|p| in_scope(p))
            .cloned()
            .collect();

        // Chunk what this run folds, by the declared rule.
        let mut files = Vec::new();
        let mut pending = Vec::new();
        for path in &selected {
            // Gone or unreadable since the plan fingerprinted it: counted, left behind, not guessed.
            let Ok(bytes) = std::fs::read(root.join(path)) else {
                report.unreadable += 1;
                continue;
            };
            let fingerprint = fingerprint(&bytes);
            let Ok(text) = String::from_utf8(bytes) else {
                report.skipped.push(path.clone());
                files.push(FoldedFile {
                    path: path.clone(),
                    fingerprint,
                    chunks: 0,
                    skipped: Some("non-utf8"),
                });
                continue;
            };
            let chunked = rule.chunk(path, &text);
            report.dropped_chunks += chunked.dropped;
            files.push(FoldedFile {
                path: path.clone(),
                fingerprint: fingerprint.clone(),
                chunks: chunked.chunks.len(),
                skipped: None,
            });
            for (ordinal, chunk) in chunked.chunks.into_iter().enumerate() {
                pending.push(Pending {
                    path: path.clone(),
                    section: chunk.section,
                    ordinal,
                    fingerprint: fingerprint.clone(),
                    text: chunk.text,
                });
            }
        }

        // Embed in declared batches; a refusal anywhere is the fold's failure, and nothing is written.
        let embedded = EmbedBudget::fold(&declared.contract).and_then(|budget| {
            let mut vectors = Vec::with_capacity(pending.len());
            let texts: Vec<String> = pending.iter().map(|p| p.text.clone()).collect();
            for batch in texts.chunks(budget.texts) {
                let reply = embedder.embed(batch, budget)?;
                if reply.dims != declared.dims()
                    || reply.vectors.iter().any(|v| v.len() != declared.dims())
                {
                    return Err(FlowError::Unavailable(format!(
                        "the embedder replied {} dims; the measure pins {}",
                        reply.dims,
                        declared.dims()
                    )));
                }
                vectors.extend(reply.vectors);
            }
            Ok(vectors)
        });
        let vectors = match embedded {
            Ok(vectors) => vectors,
            Err(error) => {
                report.attestation.shard = store.shard()?;
                report.attestation.heads_at = store.heads(declared)?;
                report.lag = Some(plan.lag());
                attest(
                    dir,
                    &mut report,
                    FoldState::Failed {
                        why: error.to_string(),
                    },
                )?;
                return Ok(report);
            }
        };

        let at = now();
        let tx = store.conn.transaction().map_err(db)?;
        for file in &files {
            tx.execute(
                "UPDATE chunks SET demoted_at = ?2 WHERE path = ?1 AND demoted_at IS NULL",
                params![file.path, at],
            )
            .map_err(db)?;
            tx.execute(
                "INSERT INTO files (path, fingerprint, chunks, skipped, folded_at, demoted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL)
             ON CONFLICT(path) DO UPDATE SET fingerprint = ?2, chunks = ?3, skipped = ?4,
               folded_at = ?5, demoted_at = NULL",
                params![
                    file.path,
                    file.fingerprint,
                    file.chunks as i64,
                    file.skipped,
                    at
                ],
            )
            .map_err(db)?;
        }
        for (chunk, vector) in pending.iter().zip(&vectors) {
            tx.execute(
                "INSERT INTO chunks (path, section, ordinal, fingerprint, text, vector, folded_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    chunk.path,
                    chunk.section,
                    chunk.ordinal as i64,
                    chunk.fingerprint,
                    chunk.text,
                    vector_blob(vector),
                    at
                ],
            )
            .map_err(db)?;
        }
        for path in &removed {
            tx.execute(
                "UPDATE chunks SET demoted_at = ?2 WHERE path = ?1 AND demoted_at IS NULL",
                params![path, at],
            )
            .map_err(db)?;
            tx.execute(
                "UPDATE files SET demoted_at = ?2 WHERE path = ?1 AND demoted_at IS NULL",
                params![path, at],
            )
            .map_err(db)?;
        }
        tx.commit().map_err(db)?;

        report.folded = files.iter().map(|file| file.path.clone()).collect();
        report.embedded_chunks = pending.len();
        report.demoted = removed;
        let lag = plan.lag() - report.folded.len() - report.demoted.len();
        report.lag = Some(lag);
        report.attestation.shard = store.shard()?;
        report.attestation.heads_at = store.heads(declared)?;
        let state = if lag == 0 {
            FoldState::Complete
        } else {
            FoldState::Degraded {
                retried: next_retry(dir),
            }
        };
        attest(dir, &mut report, state)?;
        Ok(report)
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Status
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// `epr flow memory index status`: the stable shape later routes read —
/// `{measure, model, chunk_rule, chunks, demoted, bytes, lag, last}`. `lag` is `null` when there
/// is no readable fold; `last` is the latest attestation's `{state, at, attested_by}` or `null`.
pub fn status(root: &Path) -> FlowResult<Value> {
    let declared = Declared::load(root)?;
    let dir = store_dir(root, &declared.cid.to_string());
    let last = std::fs::read(dir.join(LATEST_FILE))
        .ok()
        .and_then(|raw| serde_json::from_slice::<FoldAttestation>(&raw).ok())
        .map(|a| json!({"state": a.state, "at": a.at, "attested_by": a.attested_by.0}));
    let mut view = json!({
        "measure": declared.cid.to_string(),
        "model": declared.model(),
        "chunk_rule": declared.measure.chunk_rule.to_string(),
        "chunks": 0,
        "demoted": 0,
        "bytes": 0,
        "lag": Value::Null,
        "last": last.unwrap_or(Value::Null),
    });
    if let Ok(Some(store)) = Store::open_existing(&dir.join(STORE_FILE), &declared) {
        view["chunks"] =
            json!(store.count("SELECT count(*) FROM chunks WHERE demoted_at IS NULL")?);
        view["demoted"] =
            json!(store.count("SELECT count(*) FROM chunks WHERE demoted_at IS NOT NULL")?);
        view["bytes"] = json!(store.count(
            "SELECT coalesce(sum(length(CAST(text AS BLOB))), 0) FROM chunks WHERE demoted_at IS NULL"
        )?);
        view["lag"] = json!(plan(root, &declared, &store)?.lag());
    }
    Ok(view)
}

fn render_status(view: &Value, declared_lag: &str) -> String {
    let last = match view["last"].as_object() {
        Some(last) => format!(
            "{} at {} by {}",
            state_line(&last["state"]),
            last["at"],
            last["attested_by"].as_str().unwrap_or_default()
        ),
        None => "none".to_string(),
    };
    let lag = match view["lag"].as_u64() {
        Some(n) => format!("{n} files behind ({declared_lag})"),
        None => "skipped — no fold".to_string(),
    };
    format!(
        "index status\nmeasure     {}\nmodel       {}\nchunk rule  {}\nfold        {} chunks, {} demoted, {} bytes\nlag         {lag}\nlast        {last}\n",
        view["measure"].as_str().unwrap_or_default(),
        view["model"].as_str().unwrap_or_default(),
        view["chunk_rule"].as_str().unwrap_or_default(),
        view["chunks"],
        view["demoted"],
        view["bytes"],
    )
}

/// `complete`, `degraded (retried N)`, `failed: <why>`.
fn state_line(state: &Value) -> String {
    match state["state"].as_str() {
        Some("degraded") => format!("degraded (retried {})", state["retried"]),
        Some("failed") => format!("failed: {}", state["why"].as_str().unwrap_or_default()),
        Some(other) => other.to_string(),
        None => "unknown".to_string(),
    }
}

fn render_fold(report: &FoldReport, declared_lag: &str) -> FlowResult<String> {
    let mut out = format!("index fold\nmeasure     {}\n", report.measure);
    if !report.store.is_empty() {
        out += &format!("store       {}\n", report.store);
    }
    if !report.embedder.is_empty() {
        out += &format!("embedder    {}\n", report.embedder);
    }
    out += &format!(
        "folded      {} files, {} chunks embedded\n",
        report.folded.len(),
        report.embedded_chunks
    );
    if !report.demoted.is_empty() {
        out += &format!("demoted     {} files\n", report.demoted.len());
    }
    if !report.skipped.is_empty() {
        out += &format!("skipped     {} non-UTF-8 files\n", report.skipped.len());
    }
    if report.unreadable > 0 {
        out += &format!("unreadable  {} files\n", report.unreadable);
    }
    if report.dropped_chunks > 0 {
        out += &format!(
            "dropped     {} chunks past the per-file cap\n",
            report.dropped_chunks
        );
    }
    if let Some(lag) = report.lag {
        out += &format!("lag         {lag} files behind ({declared_lag})\n");
    }
    let state = serde_json::to_value(&report.attestation.state)?;
    out += &format!(
        "attested    {} by {} at {}\nattestation {}\n",
        state_line(&state),
        report.attestation.attested_by.0,
        report.attestation.at,
        report.attestation_cid
    );
    Ok(out)
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Command surface
// ───────────────────────────────────────────────────────────────────────────────────────────────

pub fn usage() -> String {
    "usage: epr flow memory index <fold|status> [--root DIR] [--json]\n\n  \
     fold    fold what moved under recall-semantic-index@1 and attest the result\n          \
     [--scope DIR] [--max-files N (default limits.fold_files_per_run)]\n          \
     [--embedder pinned|fixture] [--session ID]\n  \
     status  the fold's lag, chunks and last attestation\n"
        .to_string()
}

/// `epr flow memory index …`.
pub fn run(args: &[String]) -> FlowResult<ExitCode> {
    let Some(operation) = args.first() else {
        println!("{}", usage());
        return Ok(ExitCode::SUCCESS);
    };
    if operation == "--help" || operation == "-h" {
        println!("{}", usage());
        return Ok(ExitCode::SUCCESS);
    }
    let mut root = PathBuf::from(".");
    let mut opts = FoldOptions::default();
    let mut json_output = false;
    let mut i = 1;
    while i < args.len() {
        let key = args[i].as_str();
        if key == "--json" {
            json_output = true;
            i += 1;
            continue;
        }
        let value = args
            .get(i + 1)
            .ok_or_else(|| index_refused(format!("{key} needs a value")))?;
        match key {
            "--root" => root = value.into(),
            "--scope" => opts.scope = Some(value.clone()),
            "--session" => opts.session = Some(value.clone()),
            "--max-files" => {
                opts.max_files = Some(
                    value
                        .parse()
                        .map_err(|_| index_refused("--max-files takes a positive integer"))?,
                )
            }
            "--embedder" => {
                opts.embedder = match value.as_str() {
                    "pinned" => EmbedderChoice::Pinned,
                    "fixture" => EmbedderChoice::Fixture,
                    other => {
                        return Err(index_refused(format!(
                            "--embedder is pinned|fixture, not {other}"
                        )))
                    }
                }
            }
            _ => return Err(index_refused(format!("unknown option {key}"))),
        }
        i += 2;
    }
    let bound = |root: &Path| -> FlowResult<String> {
        let declared = Declared::load(root)?;
        Ok(format!(
            "fold_lag ceiling {} {}",
            declared.measure.fold_lag.limit, declared.measure.fold_lag.unit
        ))
    };
    match operation.as_str() {
        "fold" => {
            let report = fold(&root, &opts)?;
            if json_output {
                println!("{}", serde_json::to_string(&report)?);
            } else {
                print!("{}", render_fold(&report, &bound(&root)?)?);
            }
        }
        "status" => {
            if opts.scope.is_some() || opts.max_files.is_some() || opts.session.is_some() {
                return Err(index_refused("status takes only --root and --json"));
            }
            let view = status(&root)?;
            if json_output {
                println!("{}", serde_json::to_string(&view)?);
            } else {
                print!("{}", render_status(&view, &bound(&root)?));
            }
        }
        other => {
            return Err(index_refused(format!(
                "unknown operation `{other}` — the set is fold|status"
            )))
        }
    }
    Ok(ExitCode::SUCCESS)
}
