//! INDEX — the semantic fold and its status (governed-discovery station 4, task 4.3):
//! `epr flow memory index fold|status`.
//!
//! The fold executes `recall-semantic-index@1` (the `IndexMeasure` the contract's `semantic`
//! provider names) against the tree. Which files it covers is `surface.rs`'s concern: the
//! measure's own glob surface over the declared source (git-tracked files on a checkout), with
//! the contract's excluded directories and the private-chain floor on top. Each candidate is
//! fingerprinted by the raw CID of its bytes — the cites convention's body CID — and a file whose
//! size and mtime match the store's row is not re-hashed at all. Only new and changed files are
//! re-chunked (by the declared rule, `chunk.rs`) and re-embedded, at most
//! `limits.fold_files_per_run` a run; removed and superseded chunks are demoted with a timestamp,
//! never deleted (`Retention` has no delete, and the store's own triggers refuse one). Every run
//! ends in a [`FoldAttestation`]: `complete` when nothing is left behind or unread, `degraded`
//! when the run cap stopped it or a candidate could not be read, `failed` when the method, the
//! source or the procedure refused — appended to `attestations.jsonl`, then the latest renamed
//! into `attestation.json`.
//!
//! The store is `.eprfs/status/index/<measure-cid>/<embedder>/fold.sqlite` — one per embedder, so
//! fixture vectors and live ones never share a store or a log — derived and gitignored: a
//! missing, corrupt or foreign store is rebuilt from scratch, never repaired by hand. One fold at a
//! time holds `fold.lock`; a second exits reporting `busy`, writing nothing.
//!
//! The store's tables, triggers and single-transaction write are `elohim_epr_index::store`'s
//! (schema v3, rows keyed by an opaque `unit_id` — here the repository-relative path, with the
//! plan's stat recorded as `"size:mtime_ns"`); a store of any earlier schema takes the rebuild
//! path below (post-station-4 sprint, ruling R-S1).
use super::chunk::ChunkRule;
use super::embedder::{fold_budget, Embedder, Fixture, PinnedProcedure};
use super::surface::{self, Listing, Surface};
use super::*;
use cid::Cid;
use elohim_epr_index::attest::{next_retry, record, LATEST_FILE};
use elohim_epr_index::rank::encode as vector_blob;
use elohim_epr_index::store::{
    ChunkRow, LiveUnit, Root, Store as IndexStore, UnitChunks, SCHEMA_VERSION,
};
use elohim_epr_index::IndexError;
use elohim_epr_rea::{
    atom_cid, AgentRef, FoldAttestation, FoldState, IndexMeasure, RankingMethod, ShardManifest,
};
use rusqlite::params;
use serde::Serialize;

/// Every semantic fold's store and attestations live under here, one directory per measure CID
/// and, beneath it, one per embedder.
pub const INDEX_DIR_REL: &str = ".eprfs/status/index";

/// Who attested a fold when no session claim names anyone — the honest literal, never a guess.
pub const UNCLAIMED: &str = "(unclaimed)";

/// What a second, concurrent fold prints.
pub const BUSY: &str = "busy: another fold holds the store";

const STORE_FILE: &str = "fold.sqlite";
const LOCK_FILE: &str = "fold.lock";
const ACTOR_LOG_REL: &str = ".eprfs/status/actors.jsonl";
const CHUNK_RULE_MISMATCH: &str = "chunk rule on disk does not hash to the measure";

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The crate boundary: `elohim_epr_index` errors are this executor's errors, one to one
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Every index error as the executor's own error for the same fault, rendering byte-identically
/// (the pure parts moved into `elohim_epr_index`, their messages did not).
impl From<IndexError> for FlowError {
    fn from(error: IndexError) -> Self {
        match error {
            IndexError::Refused(message) => FlowError::InvalidArguments(message),
            IndexError::Unavailable(reason) => FlowError::Unavailable(reason),
            IndexError::Io(error) => FlowError::Io(error),
            IndexError::Json(error) => FlowError::Json(error),
            IndexError::Fabric(error) => FlowError::Fabric(error),
            IndexError::Store(error) => db(error),
        }
    }
}

/// The way back, for an [`Embedder`] this executor implements (the pinned procedure): the
/// variants that procedure raises map one to one. `Read`, `Yaml` and `UnknownResource` are never
/// raised on that path; were one to be, it would carry its full message as an I/O error.
impl From<FlowError> for IndexError {
    fn from(error: FlowError) -> Self {
        match error {
            FlowError::InvalidArguments(message) => IndexError::Refused(message),
            FlowError::Unavailable(reason) => IndexError::Unavailable(reason),
            FlowError::Io(error) => IndexError::Io(error),
            FlowError::Json(error) => IndexError::Json(error),
            FlowError::Fabric(error) => IndexError::Fabric(error),
            other => IndexError::Io(std::io::Error::other(other.to_string())),
        }
    }
}

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

/// Which [`Embedder`] a fold runs — and so which store it writes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EmbedderChoice {
    /// The declared procedure the model manifest pins (the default).
    #[default]
    Pinned,
    /// The deterministic test embedder — never a claim of live fitness.
    Fixture,
}

impl EmbedderChoice {
    pub fn name(self) -> &'static str {
        match self {
            EmbedderChoice::Pinned => "pinned",
            EmbedderChoice::Fixture => "fixture",
        }
    }

    fn parse(value: &str) -> FlowResult<Self> {
        match value {
            "pinned" => Ok(EmbedderChoice::Pinned),
            "fixture" => Ok(EmbedderChoice::Fixture),
            other => Err(index_refused(format!(
                "--embedder is pinned|fixture, not {other}"
            ))),
        }
    }
}

/// Where a measure's store, lock and attestations live for one embedder.
pub fn store_dir(root: &Path, measure_cid: &str, embedder: EmbedderChoice) -> PathBuf {
    root.join(INDEX_DIR_REL)
        .join(measure_cid)
        .join(embedder.name())
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The declaration
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// The contract, the measure it names (with its compiled surface), the measure file's raw value
/// (for `_chunk_rule`) and its CID.
struct Declared {
    contract: Contract,
    measure: IndexMeasure,
    surface: Surface,
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
                index_refused(
                    "the recall contract names no semantic measure \
                     (ceremony.providers.semantic.measure)",
                )
            })?
            .to_string();
        Self::from_contract(root, contract, &rel)
    }

    /// The measure at `rel` (repository-relative), under a contract the caller already loaded.
    fn from_contract(root: &Path, contract: Contract, rel: &str) -> FlowResult<Self> {
        let path = root.join(rel);
        let bytes = std::fs::read(&path).map_err(|source| FlowError::Read { path, source })?;
        let raw: Value = serde_json::from_slice(&bytes)?;
        let measure: IndexMeasure = serde_json::from_value(raw.clone())?;
        measure
            .validate()
            .map_err(|error| index_refused(format!("{rel}: {error}")))?;
        let surface = Surface::declared(measure.surfaces.paths(), &contract)
            .map_err(|error| index_refused(format!("{rel}: {error}")))?;
        let cid = measure.cid()?;
        Ok(Self {
            contract,
            measure,
            surface,
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

    fn lag_bound(&self) -> String {
        format!(
            "fold_lag ceiling {} {}",
            self.measure.fold_lag.limit, self.measure.fold_lag.unit
        )
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The plan: the tree against the store
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// The raw CID of a file's bytes, stored as its string.
fn fingerprint(bytes: &[u8]) -> String {
    BlobCid::compute_raw(bytes).to_string()
}

/// The stat the plan compares and the store records: `"size:mtime_ns"` of the handle a file's
/// bytes were read through.
fn stat_text(size: u64, mtime_ns: i64) -> String {
    format!("{size}:{mtime_ns}")
}

#[derive(Default)]
struct Plan {
    /// New or changed files, sorted.
    behind: Vec<String>,
    /// Live in the store, gone from the source.
    removed: Vec<String>,
    /// Unchanged bytes under a moved stat: the fold records the new stat, nothing else.
    restat: Vec<(String, String)>,
    /// Candidates that exist but could not be read, and listing errors — their rows stay current.
    unreadable: usize,
}

impl Plan {
    fn lag(&self) -> usize {
        self.behind.len() + self.removed.len()
    }
}

/// `Err(why)` when the source cannot be listed at all.
fn plan(root: &Path, declared: &Declared, store: &Store) -> FlowResult<Result<Plan, String>> {
    let listing: Listing = match surface::list(root, &declared.contract, &declared.surface) {
        Ok(listing) => listing,
        Err(why) => return Ok(Err(why)),
    };
    let stored = store.live_rows()?;
    let mut plan = Plan {
        unreadable: listing.errors + listing.held.len(),
        ..Plan::default()
    };
    let mut present = BTreeSet::new();
    for candidate in &listing.files {
        present.insert(candidate.rel.clone());
        let Some(row) = stored.get(&candidate.rel) else {
            plan.behind.push(candidate.rel.clone());
            continue;
        };
        if row.stat.as_deref() == Some(stat_text(candidate.size, candidate.mtime_ns).as_str()) {
            continue;
        }
        match std::fs::read(root.join(&candidate.rel)) {
            Err(_) => plan.unreadable += 1,
            Ok(bytes) if fingerprint(&bytes) == row.fingerprint => plan.restat.push((
                candidate.rel.clone(),
                stat_text(candidate.size, candidate.mtime_ns),
            )),
            Ok(_) => plan.behind.push(candidate.rel.clone()),
        }
    }
    plan.removed = stored
        .keys()
        .filter(|path| !present.contains(*path) && !listing.holds(path))
        .cloned()
        .collect();
    Ok(Ok(plan))
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The store
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// The executor's store: the crate's fold store, keyed by repository-relative path, with this
/// executor's identity rows, rebuild policy and plan reads over it.
struct Store {
    inner: IndexStore,
}

/// The store's running count of chunks embedded from a truncated head (past the model's token
/// window), in `meta` beside the identity rows but no part of the identity. Present only while
/// every chunk the store ever embedded was counted: a fold by a procedure that does not count
/// removes it, and nothing puts it back short of a new store (station 4 final review, ruling I5).
const TRUNCATED_META_KEY: &str = "truncated";

/// A store's meta rows without the truncation tally — what identity comparisons read.
fn identity(mut meta: BTreeMap<String, String>) -> BTreeMap<String, String> {
    meta.remove(TRUNCATED_META_KEY);
    meta
}

/// The identity a store was built under; any difference is a different fold.
fn meta_rows(declared: &Declared, embedder: &str) -> BTreeMap<String, String> {
    [
        ("schema", SCHEMA_VERSION.to_string()),
        ("measure", declared.cid.to_string()),
        ("model", declared.model()),
        ("chunk_rule", declared.measure.chunk_rule.to_string()),
        ("embedder", embedder.to_string()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
}

impl Store {
    fn meta(&self) -> FlowResult<BTreeMap<String, String>> {
        Ok(self.inner.meta()?)
    }

    /// A readable store of this measure: `Ok(None)` when there is no file, `Err(why)` when there
    /// is one that cannot serve (the next fold rebuilds it). `status` opens it read-only.
    fn open_existing(
        path: &Path,
        declared: &Declared,
        read_only: bool,
    ) -> Result<Option<Self>, String> {
        let Some(store) = Self::open_readable(path, read_only, true)? else {
            return Ok(None);
        };
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

    /// A store file that opens — and, when `check` (the fold and `status`), passes its integrity
    /// check — whatever it was built under. The query path skips the check: integrity is the
    /// fold's job (it rebuilds a store that fails it), and a corrupt store a query touches still
    /// surfaces as the SQLite error of the read that meets it.
    fn open_readable(path: &Path, read_only: bool, check: bool) -> Result<Option<Self>, String> {
        Ok(IndexStore::open_readable(path, read_only, check)?.map(|inner| Self { inner }))
    }

    /// The store this fold writes to, and what happened to get it: `created`, `reused`, or
    /// `rebuilt (<why>)` — a store that cannot serve is replaced, never patched.
    fn open_or_rebuild(
        path: &Path,
        declared: &Declared,
        embedder: &str,
    ) -> FlowResult<(Self, String)> {
        let wanted = meta_rows(declared, embedder);
        let why = match Self::open_existing(path, declared, false) {
            Ok(None) => None,
            Ok(Some(store)) => match store.meta().map(identity) {
                Ok(meta) if meta == wanted => return Ok((store, "reused".into())),
                Ok(_) => Some("store was folded under another procedure".to_string()),
                Err(error) => Some(error.to_string()),
            },
            Err(why) => Some(why),
        };
        let outcome = match &why {
            Some(why) => {
                IndexStore::remove(path)?;
                format!("rebuilt ({why})")
            }
            None => "created".to_string(),
        };
        // A new store has embedded nothing yet: its tally starts known, at zero.
        let mut rows = wanted;
        rows.insert(TRUNCATED_META_KEY.to_string(), "0".to_string());
        let inner = IndexStore::create(path, &rows)?;
        Ok((Self { inner }, outcome))
    }

    /// Every file the fold currently holds, with its fingerprint and recorded stat.
    fn live_rows(&self) -> FlowResult<BTreeMap<String, LiveUnit>> {
        Ok(self.inner.live_units()?)
    }

    /// What the fold holds: live chunk count, their text bytes, and the CID of the sorted
    /// `(path, section, fingerprint)` manifest.
    fn shard(&self) -> FlowResult<ShardManifest> {
        Ok(self.inner.shard()?)
    }

    /// One `(root, head)` pair per declared include pattern (the surface's roots; a negation owns
    /// no files): the raw CID of the pattern's bytes, and the CID of the sorted
    /// `(path, fingerprint)` list of the held files it matches — so a later fold can name which
    /// root moved.
    fn heads(&self, declared: &Declared) -> FlowResult<Vec<(Cid, Cid)>> {
        type Matcher<'m> = Box<dyn Fn(&str) -> bool + 'm>;
        let matchers: Vec<(Cid, Matcher<'_>)> = declared
            .surface
            .includes()
            .map(|(text, pattern)| {
                let matches: Matcher<'_> =
                    Box::new(move |path: &str| Surface::include_matches(pattern, path));
                (*BlobCid::compute_raw(text.as_bytes()).as_cid(), matches)
            })
            .collect();
        let roots: Vec<Root<'_>> = matchers
            .iter()
            .map(|(root, matches)| (*root, matches.as_ref()))
            .collect();
        Ok(self.inner.heads(&roots)?)
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
    /// Files gone from the source whose chunks this run demoted.
    pub demoted: Vec<String>,
    /// Folded files skipped as binary or non-UTF-8.
    pub skipped: Vec<String>,
    /// Candidates that exist but could not be read, and listing errors.
    pub unreadable: usize,
    /// Chunks past `max_chunks_per_file`, dropped and counted.
    pub dropped_chunks: usize,
    /// Chunks this run embedded from a truncated head (longer than the model's token window);
    /// `None` when the embedding procedure does not count.
    pub truncated: Option<usize>,
    /// Files still behind after this run; `None` when the fold failed before measuring.
    pub lag: Option<usize>,
    pub attestation: FoldAttestation,
    pub attestation_cid: String,
}

/// A fold either ran (and attested), or found another fold holding the store.
#[derive(Debug)]
pub enum FoldRun {
    Done(Box<FoldReport>),
    Busy,
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

/// The exclusive fold lock, or `None` when another fold holds it.
fn lock(dir: &Path) -> FlowResult<Option<File>> {
    std::fs::create_dir_all(dir)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join(LOCK_FILE))?;
    match rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => Ok(Some(file)),
        Err(errno) if errno == rustix::io::Errno::WOULDBLOCK => Ok(None),
        Err(errno) => Err(FlowError::Io(errno.into())),
    }
}

fn attest(dir: &Path, report: &mut FoldReport, state: FoldState) -> FlowResult<()> {
    report.attestation.state = state;
    report.attestation.at = now();
    report.attestation_cid = record(dir, &report.attestation)?;
    Ok(())
}

/// Record the shard and heads a store holds, then attest `Failed{why}`.
fn attest_failed(
    dir: &Path,
    declared: &Declared,
    store: Option<&Store>,
    mut report: FoldReport,
    why: String,
) -> FlowResult<FoldRun> {
    if let Some(store) = store {
        report.attestation.shard = store.shard()?;
        report.attestation.heads_at = store.heads(declared)?;
    }
    attest(dir, &mut report, FoldState::Failed { why })?;
    Ok(FoldRun::Done(Box::new(report)))
}

/// `epr flow memory index fold`: fold what moved, at most the run cap, and attest the result.
/// A fold that ran always attests — `failed` included; a fold that found the lock held returns
/// [`FoldRun::Busy`] having written nothing; only an unreadable declaration (no measure CID to
/// attest under) or a bad option is an `Err`.
pub fn fold(root: &Path, opts: &FoldOptions) -> FlowResult<FoldRun> {
    let declared = Declared::load(root)?;
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
    let dir = store_dir(root, &declared.cid.to_string(), opts.embedder);
    let Some(_held) = lock(&dir)? else {
        return Ok(FoldRun::Busy);
    };
    let report = FoldReport {
        measure: declared.cid.to_string(),
        store: String::new(),
        embedder: String::new(),
        folded: Vec::new(),
        embedded_chunks: 0,
        truncated: Some(0),
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
            attested_by: attested_by(root, opts.session.as_deref()),
            at: 0,
        },
        attestation_cid: String::new(),
    };

    // Before any store is touched: the method, then the procedure that will run it.
    let existing = || {
        Store::open_existing(&dir.join(STORE_FILE), &declared, true)
            .ok()
            .flatten()
    };
    let rule = match declared.chunk_rule() {
        Ok(rule) => rule,
        Err(why) => return attest_failed(&dir, &declared, existing().as_ref(), report, why),
    };
    let (embedder, label) = match embedder_for(root, opts.embedder, &declared) {
        Ok(pair) => pair,
        Err(error) => {
            let why = error.to_string();
            return attest_failed(&dir, &declared, existing().as_ref(), report, why);
        }
    };
    let run = Run {
        root,
        declared: &declared,
        dir: &dir,
        rule,
        embedder: embedder.as_ref(),
        opts,
        max_files,
    };
    run.fold(report, label)
}

/// One chunk ready to write; its file is the `FoldedFile` it follows, in order.
struct Pending {
    section: String,
    text: String,
}

/// A file this run folds: its fingerprint and stat (taken with the bytes it read), and why it has
/// no chunks if it was skipped.
struct FoldedFile {
    path: String,
    fingerprint: String,
    size: u64,
    mtime_ns: i64,
    chunks: usize,
    skipped: Option<&'static str>,
}

/// Read a file and the stat of the very handle it was read through (the stat first, so a write
/// racing the read moves the mtime past what is recorded and the next plan re-hashes).
fn read_with_stat(path: &Path) -> std::io::Result<(Vec<u8>, u64, i64)> {
    let mut file = File::open(path)?;
    let (size, mtime_ns) = surface::stat_of(&file.metadata()?);
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok((bytes, size, mtime_ns))
}

/// One fold run, once the lock is held and the method and the embedder are both checked.
struct Run<'a> {
    root: &'a Path,
    declared: &'a Declared,
    dir: &'a Path,
    rule: ChunkRule,
    embedder: &'a dyn Embedder,
    opts: &'a FoldOptions,
    max_files: usize,
}

impl Run<'_> {
    fn fold(self, mut report: FoldReport, label: String) -> FlowResult<FoldRun> {
        let Run {
            root,
            declared,
            dir,
            rule,
            embedder,
            opts,
            max_files,
        } = self;
        let (mut store, outcome) = Store::open_or_rebuild(&dir.join(STORE_FILE), declared, &label)?;
        report.store = outcome;
        report.embedder = label;
        let plan = match plan(root, declared, &store)? {
            Ok(plan) => plan,
            Err(why) => return attest_failed(dir, declared, Some(&store), report, why),
        };
        report.unreadable = plan.unreadable;
        let in_scope = |path: &String| match &opts.scope {
            Some(scope) => {
                let scope = scope.trim_end_matches('/');
                path == scope || path.starts_with(&format!("{scope}/"))
            }
            None => true,
        };
        let selected: Vec<&String> = plan
            .behind
            .iter()
            .filter(|p| in_scope(p))
            .take(max_files)
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
        for path in selected {
            // Unreadable since the plan listed it: counted, its rows kept, not guessed.
            let Ok((bytes, size, mtime_ns)) = read_with_stat(&root.join(path)) else {
                report.unreadable += 1;
                continue;
            };
            let fingerprint = fingerprint(&bytes);
            let mut file = FoldedFile {
                path: path.clone(),
                fingerprint: fingerprint.clone(),
                size,
                mtime_ns,
                chunks: 0,
                skipped: None,
            };
            let Ok(text) = String::from_utf8(bytes) else {
                report.skipped.push(path.clone());
                file.skipped = Some("non-utf8");
                files.push(file);
                continue;
            };
            let chunked = rule.chunk(path, &text);
            report.dropped_chunks += chunked.dropped;
            file.chunks = chunked.chunks.len();
            files.push(file);
            for chunk in chunked.chunks {
                pending.push(Pending {
                    section: chunk.section,
                    text: chunk.text,
                });
            }
        }

        // Embed in declared batches; a refusal anywhere is the fold's failure, and nothing is
        // written.
        let dims = declared.dims();
        let embedded = fold_budget(&declared.contract).and_then(|budget| {
            let mut vectors = Vec::with_capacity(pending.len());
            // Known only while every batch counted.
            let mut truncated = Some(0usize);
            let texts: Vec<String> = pending.iter().map(|p| p.text.clone()).collect();
            for batch in texts.chunks(budget.texts) {
                let reply = embedder.embed(batch, budget)?;
                if reply.dims != dims || reply.vectors.iter().any(|v| v.len() != dims) {
                    return Err(FlowError::Unavailable(format!(
                        "the embedder replied {} dims; the measure pins {dims}",
                        reply.dims
                    )));
                }
                truncated = truncated.zip(reply.truncated).map(|(sum, n)| sum + n);
                vectors.extend(reply.vectors);
            }
            Ok((vectors, truncated))
        });
        let vectors = match embedded {
            Ok((vectors, truncated)) => {
                report.truncated = truncated;
                vectors
            }
            Err(error) => {
                report.lag = Some(plan.lag());
                let why = error.to_string();
                return attest_failed(dir, declared, Some(&store), report, why);
            }
        };

        let at = now();
        let embedded_chunks = pending.len();
        // Each folded file's chunks, in the order they were chunked and embedded.
        let mut chunks = pending.into_iter().zip(vectors);
        let insert: Vec<UnitChunks> = files
            .iter()
            .map(|file| UnitChunks {
                unit_id: file.path.clone(),
                fingerprint: file.fingerprint.clone(),
                stat: Some(stat_text(file.size, file.mtime_ns)),
                skipped: file.skipped.map(str::to_string),
                chunks: chunks
                    .by_ref()
                    .take(file.chunks)
                    .map(|(chunk, vector)| ChunkRow {
                        section: chunk.section,
                        text: chunk.text,
                        vector: Some(vector_blob(&vector)),
                    })
                    .collect(),
            })
            .collect();
        let truncated = report.truncated;
        store.inner.fold_txn(at, &removed, &insert, |tx| {
            // The store's tally: this run's count added while every chunk so far was counted; an
            // uncounted run makes it unknown for good.
            match truncated {
                Some(n) => tx.execute(
                    "UPDATE meta SET value = CAST(value AS INTEGER) + ?2 WHERE key = ?1",
                    params![TRUNCATED_META_KEY, n as i64],
                )?,
                None => tx.execute(
                    "DELETE FROM meta WHERE key = ?1",
                    params![TRUNCATED_META_KEY],
                )?,
            };
            for (path, stat) in &plan.restat {
                tx.execute(
                    "UPDATE units SET stat = ?2 WHERE unit_id = ?1",
                    params![path, stat],
                )?;
            }
            Ok(())
        })?;

        report.folded = files.iter().map(|file| file.path.clone()).collect();
        report.embedded_chunks = embedded_chunks;
        report.demoted = removed;
        // A file the run selected but could not read stays behind.
        let lag = plan.lag() - report.folded.len() - report.demoted.len();
        report.lag = Some(lag);
        report.attestation.shard = store.shard()?;
        report.attestation.heads_at = store.heads(declared)?;
        let state = if lag == 0 && report.unreadable == 0 {
            FoldState::Complete
        } else {
            FoldState::Degraded {
                retried: next_retry(dir),
            }
        };
        attest(dir, &mut report, state)?;
        Ok(FoldRun::Done(Box::new(report)))
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// The query side — the fold the semantic (task 4.4) and lexical (task 4.8) providers rank over
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// Why a declared fold cannot answer a question: each an honest absence the semantic provider
/// prints as one line, never an error.
pub(super) enum Absent {
    /// No store has been folded for this measure and embedder.
    NoFold,
    /// A store exists, but its meta names another schema, measure, model, chunk rule or embedder.
    OtherMethod,
    /// A store exists but cannot be read (the next fold rebuilds it).
    Unreadable(String),
}

/// The measure a semantic provider declares, with the embedder whose store it reads.
pub(super) struct SemanticFold {
    declared: Declared,
    choice: EmbedderChoice,
}

/// A store opened read-only whose meta matches the declaration — every key but the embedder's
/// label, which only the embedder, once built, can name (see [`FoldReader::built_by`]).
pub(super) struct FoldReader {
    store: Store,
    built_by: String,
}

impl SemanticFold {
    /// The measure at `measure_rel` under `contract`; an error only when the declaration itself
    /// does not load or validate.
    pub(super) fn declare(
        root: &Path,
        contract: Contract,
        measure_rel: &str,
        choice: EmbedderChoice,
    ) -> FlowResult<Self> {
        Ok(Self {
            declared: Declared::from_contract(root, contract, measure_rel)?,
            choice,
        })
    }

    /// The method every candidate prints: the `IndexMeasure` CID.
    pub(super) fn measure(&self) -> String {
        self.declared.cid.to_string()
    }

    /// The model CID the measure pins.
    pub(super) fn model(&self) -> String {
        self.declared.model()
    }

    pub(super) fn dims(&self) -> usize {
        self.declared.dims()
    }

    pub(super) fn lag_bound(&self) -> String {
        self.declared.lag_bound()
    }

    /// The store this measure's embedder folded, opened read-only (a concurrent fold's write is
    /// waited on for the busy timeout, never raced).
    pub(super) fn open(&self, root: &Path) -> Result<FoldReader, Absent> {
        let path = store_dir(root, &self.measure(), self.choice).join(STORE_FILE);
        let store = match Store::open_readable(&path, true, false) {
            Ok(Some(store)) => store,
            Ok(None) => return Err(Absent::NoFold),
            Err(why) => return Err(Absent::Unreadable(why)),
        };
        let mut meta = identity(
            store
                .meta()
                .map_err(|error| Absent::Unreadable(error.to_string()))?,
        );
        let built_by = meta.remove("embedder").unwrap_or_default();
        let mut wanted = meta_rows(&self.declared, "");
        wanted.remove("embedder");
        if meta != wanted
            || (self.choice == EmbedderChoice::Fixture
                && built_by != EmbedderChoice::Fixture.name())
        {
            return Err(Absent::OtherMethod);
        }
        Ok(FoldReader { store, built_by })
    }

    /// The embedder that answers a question under this measure, and the label a store it folded
    /// carries (the same pair a fold runs with).
    pub(super) fn embedder(&self, root: &Path) -> FlowResult<(Box<dyn Embedder>, String)> {
        embedder_for(root, self.choice, &self.declared)
    }

    /// The fold's lag at this moment — files new, changed or removed since their fold — or why the
    /// source could not be listed.
    pub(super) fn lag(&self, root: &Path, reader: &FoldReader) -> Result<usize, String> {
        match plan(root, &self.declared, &reader.store) {
            Ok(Ok(plan)) => Ok(plan.lag()),
            Ok(Err(why)) => Err(why),
            Err(error) => Err(error.to_string()),
        }
    }
}

/// A lexical measure (station 4, task 4.8): an `IndexMeasure` ranking by BM25 over the FTS5 table
/// of a semantic fold it SHARES. It folds nothing of its own, pins no model (`validate()` refuses a
/// pin no ranking uses) and never runs an embedder; its method is its own CID, and the fold it
/// reads is valid for it only when the two declarations cut and cover the same text.
pub(super) struct LexicalMeasure {
    declared: Declared,
}

impl LexicalMeasure {
    /// The measure at `measure_rel` under `contract`; an error when the declaration does not load,
    /// does not validate, or ranks by anything but `bm25`.
    pub(super) fn declare(root: &Path, contract: Contract, measure_rel: &str) -> FlowResult<Self> {
        let declared = Declared::from_contract(root, contract, measure_rel)?;
        if declared.measure.ranking != RankingMethod::Bm25 {
            return Err(index_refused(format!(
                "{measure_rel}: a lexical measure ranks by bm25"
            )));
        }
        Ok(Self { declared })
    }

    /// The method every lexical candidate prints: this measure's CID.
    pub(super) fn cid(&self) -> String {
        self.declared.cid.to_string()
    }

    /// Whether `fold` was cut and covered under this measure's own method: the chunk rule on disk
    /// hashes to the declared one, and the chunk rule and surfaces equal the fold measure's.
    pub(super) fn shares(&self, fold: &SemanticFold) -> bool {
        self.declared.chunk_rule().is_ok()
            && self.declared.measure.chunk_rule == fold.declared.measure.chunk_rule
            && self.declared.measure.surfaces == fold.declared.measure.surfaces
    }
}

impl FoldReader {
    /// Visit every LIVE chunk the FTS5 `expression` matches, with its `bm25()` (lower is a better
    /// match); returns how many matched. The FTS table holds live chunks only (a demotion deletes
    /// its row), and the join to `chunks` keeps a demoted row out even if one ever lingered.
    pub(super) fn lexical(
        &self,
        expression: &str,
        visit: impl FnMut(i64, &str, f64),
    ) -> FlowResult<u64> {
        Ok(self.store.inner.lexical(expression, visit)?)
    }

    /// The embedder label the store was folded under (`fixture`, or `procedure <cid> on model
    /// <cid>`): a query embedded by anything else would be compared against foreign vectors.
    pub(super) fn built_by(&self) -> &str {
        &self.built_by
    }

    /// Visit every live chunk's `(id, path, vector bytes)`; returns how many were visited.
    pub(super) fn scan(&self, mut visit: impl FnMut(i64, &str, &[u8])) -> FlowResult<u64> {
        // Every chunk this executor folds carries a vector; a NULL one reads as no vector at all
        // (zero wide), which the semantic route counts as malformed rather than ranks.
        Ok(self
            .store
            .inner
            .scan(|id, path, vector| visit(id, path, vector.unwrap_or_default()))?)
    }

    /// One chunk's `(section, text)`.
    pub(super) fn chunk(&self, id: i64) -> FlowResult<(String, String)> {
        Ok(self.store.inner.chunk(id)?)
    }
}

// ───────────────────────────────────────────────────────────────────────────────────────────────
// Status
// ───────────────────────────────────────────────────────────────────────────────────────────────

/// `epr flow memory index status`: the stable shape later routes read — `{measure, embedder,
/// source, model, chunk_rule, chunks, demoted, bytes, lag, unreadable, unusable, last}`. `model`
/// is `null` for the fixture (its vectors are no model's). `lag`/`unreadable` are `null` when there
/// is no readable fold; `unusable` then says WHY when a store file exists but cannot serve (another
/// schema or measure, a failed integrity check — the next fold builds a fresh one), and is `null`
/// when there is simply no store: "no fold" and "a fold nobody can read" are different answers.
/// `last` is the latest attestation's `{state, at, attested_by}` or `null`. Status writes nothing:
/// the store is opened read-only, and an unchanged file is not re-hashed.
pub fn status(root: &Path, embedder: EmbedderChoice) -> FlowResult<Value> {
    let declared = Declared::load(root)?;
    let dir = store_dir(root, &declared.cid.to_string(), embedder);
    let last = std::fs::read(dir.join(LATEST_FILE))
        .ok()
        .and_then(|raw| serde_json::from_slice::<FoldAttestation>(&raw).ok())
        .map(|a| json!({"state": a.state, "at": a.at, "attested_by": a.attested_by.0}));
    let model = match embedder {
        EmbedderChoice::Pinned => json!(declared.model()),
        EmbedderChoice::Fixture => Value::Null,
    };
    let mut view = json!({
        "measure": declared.cid.to_string(),
        "embedder": embedder.name(),
        "source": surface::source_of(root),
        "model": model,
        "chunk_rule": declared.measure.chunk_rule.to_string(),
        "chunks": 0,
        "demoted": 0,
        "bytes": 0,
        "lag": Value::Null,
        "unreadable": Value::Null,
        "unusable": Value::Null,
        "last": last.unwrap_or(Value::Null),
        "truncated": Value::Null,
    });
    let store = match Store::open_existing(&dir.join(STORE_FILE), &declared, true) {
        Ok(store) => store,
        Err(why) => {
            view["unusable"] = json!(why);
            None
        }
    };
    if let Some(store) = store {
        let tally = store.inner.tally()?;
        view["chunks"] = json!(tally.chunks);
        view["demoted"] = json!(tally.demoted);
        view["bytes"] = json!(tally.bytes);
        let plan = plan(root, &declared, &store)?
            .map_err(|why| FlowError::Unavailable(format!("fold source: {why}")))?;
        view["lag"] = json!(plan.lag());
        view["unreadable"] = json!(plan.unreadable);
        view["truncated"] = json!(store
            .meta()?
            .get(TRUNCATED_META_KEY)
            .and_then(|n| n.parse::<u64>().ok()));
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
        Some(n) => format!(
            "{n} files behind ({declared_lag}), {} unreadable",
            view["unreadable"]
        ),
        None => match view["unusable"].as_str() {
            Some(why) => format!("skipped — fold unusable ({why})"),
            None => "skipped — no fold".to_string(),
        },
    };
    let truncated = match view["truncated"].as_u64() {
        Some(n) => format!("truncated   {n} chunks past the model's token window\n"),
        None if view["lag"].is_u64() => {
            "truncated   unknown — a procedure that does not count folded part of this store\n"
                .to_string()
        }
        None => String::new(),
    };
    format!(
        "index status\nmeasure     {}\nembedder    {} ({} source)\nmodel       {}\n\
         chunk rule  {}\nfold        {} chunks, {} demoted, {} bytes\n{truncated}\
         lag         {lag}\nlast        {last}\n",
        view["measure"].as_str().unwrap_or_default(),
        view["embedder"].as_str().unwrap_or_default(),
        view["source"].as_str().unwrap_or_default(),
        view["model"]
            .as_str()
            .unwrap_or("none — fixture vectors are no model's"),
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
    let counted = [
        (report.demoted.len(), "demoted", "files"),
        (report.skipped.len(), "skipped", "non-UTF-8 files"),
        (report.unreadable, "unreadable", "files (rows kept)"),
        (
            report.dropped_chunks,
            "dropped",
            "chunks past the per-file cap",
        ),
    ];
    for (count, label, what) in counted {
        if count > 0 {
            out += &format!("{label:<11} {count} {what}\n");
        }
    }
    out += &match report.truncated {
        Some(n) => format!("truncated   {n} chunks past the model's token window\n"),
        None => "truncated   unknown — the embedding procedure does not count\n".to_string(),
    };
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
    "usage: epr flow memory index <fold|status> [--root DIR] [--embedder pinned|fixture] \
     [--json]\n\n  \
     fold    fold what moved under recall-semantic-index@1 and attest the result\n          \
     [--scope DIR] [--max-files N (default limits.fold_files_per_run)] [--session ID]\n  \
     status  the fold's lag, chunks and last attestation (per embedder; default pinned)\n"
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
            "--embedder" => opts.embedder = EmbedderChoice::parse(value)?,
            "--max-files" => {
                opts.max_files = Some(
                    value
                        .parse()
                        .map_err(|_| index_refused("--max-files takes a positive integer"))?,
                )
            }
            _ => return Err(index_refused(format!("unknown option {key}"))),
        }
        i += 2;
    }
    match operation.as_str() {
        "fold" => match fold(&root, &opts)? {
            FoldRun::Busy if json_output => println!("{}", json!({ "busy": BUSY })),
            FoldRun::Busy => println!("{BUSY}"),
            FoldRun::Done(report) if json_output => {
                println!("{}", serde_json::to_string(&report)?)
            }
            FoldRun::Done(report) => {
                let bound = Declared::load(&root)?.lag_bound();
                print!("{}", render_fold(&report, &bound)?);
            }
        },
        "status" => {
            if opts.scope.is_some() || opts.max_files.is_some() || opts.session.is_some() {
                return Err(index_refused(
                    "status takes only --root, --embedder and --json",
                ));
            }
            let view = status(&root, opts.embedder)?;
            if json_output {
                println!("{}", serde_json::to_string(&view)?);
            } else {
                let bound = Declared::load(&root)?.lag_bound();
                print!("{}", render_status(&view, &bound));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Schema v3 (post-station-4 sprint): a store folded under v2 — rows keyed by `path` in a
    /// `files` table — is not migrated. The next fold takes the rebuild path and names why; the
    /// fold after that reuses the v3 store it built.
    #[test]
    fn a_v2_store_is_rebuilt_under_schema_v3() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut value = crate::flow::memory::recall::tests_support::minimal_contract();
        value["ceremony"]["providers"]["semantic"]["embedder"] = json!("fixture");
        let measure = value["ceremony"]["providers"]["semantic"]["measure"]
            .as_str()
            .unwrap()
            .to_string();
        for (rel, bytes) in [
            (
                CONTRACT_REL.to_string(),
                serde_json::to_vec(&value).unwrap(),
            ),
            (measure.clone(), std::fs::read(repo.join(&measure)).unwrap()),
            (
                "note.md".to_string(),
                b"# Stewardship\nThe commons is tended.\n".to_vec(),
            ),
        ] {
            std::fs::create_dir_all(root.join(&rel).parent().unwrap()).unwrap();
            std::fs::write(root.join(&rel), bytes).unwrap();
        }
        let cid = Declared::load(root).unwrap().cid.to_string();
        let store = store_dir(root, &cid, EmbedderChoice::Fixture);
        std::fs::create_dir_all(&store).unwrap();
        let v2 = rusqlite::Connection::open(store.join(STORE_FILE)).unwrap();
        v2.execute_batch(&format!(
            "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE files (path TEXT PRIMARY KEY, fingerprint TEXT NOT NULL);
             INSERT INTO meta VALUES ('schema', '2'), ('measure', '{cid}');"
        ))
        .unwrap();
        drop(v2);

        let opts = FoldOptions {
            embedder: EmbedderChoice::Fixture,
            ..FoldOptions::default()
        };
        let FoldRun::Done(first) = fold(root, &opts).unwrap() else {
            panic!("the fold ran");
        };
        assert_eq!(
            first.store,
            "rebuilt (store was built under another schema)"
        );
        assert!(
            first.folded.contains(&"note.md".to_string()),
            "{:?}",
            first.folded
        );
        let rebuilt = rusqlite::Connection::open(store.join(STORE_FILE)).unwrap();
        let (schema, units): (String, i64) = rebuilt
            .query_row(
                "SELECT (SELECT value FROM meta WHERE key = 'schema'), \
                 (SELECT count(*) FROM units WHERE unit_id = 'note.md')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!((schema.as_str(), units), ("3", 1));
        let stat: String = rebuilt
            .query_row(
                "SELECT stat FROM units WHERE unit_id = 'note.md'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let (size, mtime_ns) = stat.split_once(':').expect("size:mtime_ns");
        assert_eq!(size, "37");
        assert!(mtime_ns.parse::<i64>().is_ok(), "{stat}");

        let FoldRun::Done(second) = fold(root, &opts).unwrap() else {
            panic!("the fold ran");
        };
        assert_eq!(second.store, "reused");
        assert!(
            second.folded.is_empty(),
            "an unchanged stat is not re-hashed"
        );
    }
}
