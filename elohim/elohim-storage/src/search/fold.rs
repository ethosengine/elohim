//! The content lexical fold (post-station-4 sprint, ruling R-S3).
//!
//! [`SearchIndex`] folds the peer's content projection into an FTS5 store under the
//! `content-lexical-index` measure, one unit per content row (`unit_id = content.id`):
//!
//! - **Where.** `<storage_dir>/index/<measure-cid>/fold.sqlite` ([`store_path`]), the attestation
//!   files beside it. Category C: derived, rebuilt on corruption (the store's `quick_check`
//!   decides), never a diesel migration, never in a sync manifest, never gossiped. A store built
//!   under another schema or measure is replaced, never patched.
//! - **What a unit holds.** A `head` section (`title`, a blank line, `description`), the body
//!   (`content_body`) chunked by the measure's declared chunk rule (a markdown body splits at
//!   headings, every other body into line windows), and a `tags` section (the row's tags joined).
//! - **Incremental.** The fingerprint is the content's own CID — `blob_cid` when the row names
//!   one, else the raw CID of `content_body` — and the stored stat is the raw CID of the head and
//!   tags text ([`head_stat`]), so a unit is re-chunked only when its body or its head/tags moved;
//!   a row whose `updated_at` moved and nothing else is seen and skipped. A row that is gone from
//!   `content` has its unit demoted (never deleted).
//! - **Watermark.** Rows are read in `(updated_at, id)` order past the watermark kept in the
//!   store's `meta` table, at most [`Declared::fold_lag_limit`] rows a run, all writes in ONE
//!   transaction. `updated_at` is compared through SQLite's `datetime()` so the two spellings the
//!   content writers use (`2026-09-24 12:00:00` and `2026-09-24T12:00:00Z`) order as one clock;
//!   the watermark never passes the last [`SETTLE_SECONDS`], so a row stamped in the same second
//!   as a run (or committed just after it) is still read by the next one.
//! - **Attestation.** A run that changed the fold, or that is the first this process ran, ends in
//!   a [`FoldAttestation`] appended to `attestations.jsonl` and renamed into `attestation.json`
//!   (`elohim_epr_index::attest::record`): `complete` when nothing is behind, `degraded` while a
//!   backlog remains, `failed` with the reason when the run could not finish. A run that found
//!   nothing to do writes nothing — a 30 s timer must not grow the log by itself.
//! - **When.** [`SearchIndex::run_loop`]: once at boot, on [`SearchIndex::notify`] (the content
//!   service calls it on create, bulk create, update and delete), and every [`SWEEP_PERIOD`], so
//!   rows arriving through `upsert_with_anchor`, imports and CRDT sync fold without a hook in each.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use diesel::prelude::*;
use diesel::sql_types::{BigInt, Nullable, Text};
use diesel::SqliteConnection;
use elohim_epr_index::attest::{next_retry, record, LATEST_FILE};
use elohim_epr_index::chunk::ChunkRule;
use elohim_epr_index::store::{ChunkRow, LiveUnit, Store, UnitChunks, SCHEMA_VERSION};
use elohim_epr_rea::{atom_cid, AgentRef, FoldAttestation, FoldState};

use super::measure::Declared;
use crate::blob_store::BlobStore;
use crate::db::DbPool;

/// The directory under `storage_dir` every index lives in, one subdirectory per measure CID.
pub const INDEX_DIR: &str = "index";

/// The fold store's file name inside its measure directory.
pub const STORE_FILE: &str = "fold.sqlite";

/// How often the loop sweeps for rows no notify announced.
pub const SWEEP_PERIOD: Duration = Duration::from_secs(30);

/// The watermark never passes this many seconds before now: rows stamped inside the window are
/// re-read (and skipped when unchanged) until it closes.
pub const SETTLE_SECONDS: i64 = 5;

/// Who attests a fold when no claim names anyone — the honest literal, never a guess.
pub const UNCLAIMED: &str = "(unclaimed)";

/// The section a unit's title and description fold under.
pub const HEAD_SECTION: &str = "head";

/// The section a unit's tags fold under.
pub const TAGS_SECTION: &str = "tags";

/// `meta` keys: the identity the store was built under, and the watermark.
pub const META_SCHEMA: &str = "schema";
pub const META_MEASURE: &str = "measure";
pub const META_UNIT: &str = "unit";
pub const META_WATERMARK_AT: &str = "watermark_updated_at";
pub const META_WATERMARK_ID: &str = "watermark_id";
/// How far the demotion sweep has walked the store's own unit ids (ruling R-S10, review W1).
pub const META_DEMOTE_CURSOR: &str = "demote_cursor";
/// Rows this fold has read and could NOT date — `datetime()` could not read their `updated_at`
/// (ruling R-S11, review W3). It lives in the store's own `meta`, so it survives a restart, and
/// it is cleared only when the store is rebuilt: a hole in the watermark's order is not repaired
/// by a later batch that happens to read only datable rows.
pub const META_UNPARSED_AT: &str = "unparsed_at";

/// The one surface root a content fold's heads are taken per: the `Content` kind it declares.
const SURFACE_ROOT: &str = "Content";

/// `<storage_dir>/index/<measure-cid>` — the fold's directory.
pub fn store_dir(storage_dir: &Path, measure_cid: &str) -> PathBuf {
    storage_dir.join(INDEX_DIR).join(measure_cid)
}

/// `<storage_dir>/index/<measure-cid>/fold.sqlite` — the fold store.
pub fn store_path(storage_dir: &Path, measure_cid: &str) -> PathBuf {
    store_dir(storage_dir, measure_cid).join(STORE_FILE)
}

/// The raw CID (`CIDv1`, raw codec, sha2-256) of `bytes`, as its string — the peer's one raw
/// address ([`BlobStore::compute_cid`]), the same a blob's `blob_cid` is minted by.
pub fn raw_cid(bytes: &[u8]) -> String {
    BlobStore::compute_cid(bytes).to_string()
}

/// The fingerprint of a content row: its own CID — `blob_cid` when the row names one, else the
/// raw CID of `content_body` (of the empty body when it has none).
pub fn fingerprint(blob_cid: Option<&str>, content_body: Option<&str>) -> String {
    match blob_cid.map(str::trim).filter(|cid| !cid.is_empty()) {
        Some(cid) => cid.to_string(),
        None => raw_cid(content_body.unwrap_or_default().as_bytes()),
    }
}

/// The stat a unit records: the raw CID of what its `head` and `tags` sections are made of (and
/// the format that decides how its body is cut), so a retitle, a retag or a format change re-folds
/// a unit whose body CID did not move.
pub fn head_stat(title: &str, description: Option<&str>, format: &str, tags: &[String]) -> String {
    let text = format!(
        "{title}\u{0}{}\u{0}{format}\u{0}{}",
        description.unwrap_or_default(),
        tags.join("\u{0}")
    );
    format!("head:{}", raw_cid(text.as_bytes()))
}

/// One content row as the fold reads it.
#[derive(Debug, Clone, QueryableByName)]
pub struct ContentRow {
    #[diesel(sql_type = Text)]
    pub id: String,
    #[diesel(sql_type = Text)]
    pub h_app_id: String,
    #[diesel(sql_type = Text)]
    pub title: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub description: Option<String>,
    #[diesel(sql_type = Text)]
    pub content_format: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub content_body: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub blob_cid: Option<String>,
    /// `updated_at` on one clock: `coalesce(datetime(updated_at), updated_at)`.
    #[diesel(sql_type = Text)]
    pub at_key: String,
    /// 1 when SQLite's `datetime()` could not read this row's `updated_at`, so its `at_key` is
    /// the raw string and orders lexically among the parsed ones (review m2).
    #[diesel(sql_type = BigInt)]
    pub at_unparsed: i64,
}

#[derive(QueryableByName)]
struct TagRow {
    #[diesel(sql_type = Text)]
    tag: String,
}

#[derive(QueryableByName)]
struct IdRow {
    #[diesel(sql_type = Text)]
    id: String,
}

/// One `datetime()` value read from the database's own clock.
#[derive(QueryableByName)]
struct AtRow {
    #[diesel(sql_type = Text)]
    at_key: String,
}

#[derive(QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    n: i64,
}

/// `updated_at` on one clock, the key every read below orders and compares by.
const AT_KEY: &str = "coalesce(datetime(updated_at), updated_at)";

fn rows_after_sql() -> String {
    format!(
        "SELECT id, h_app_id, title, description, content_format, content_body, blob_cid, \
         {AT_KEY} AS at_key, (datetime(updated_at) IS NULL) AS at_unparsed FROM content \
         WHERE {AT_KEY} > ?1 OR ({AT_KEY} = ?1 AND id > ?2) \
         ORDER BY at_key, id LIMIT ?3"
    )
}

/// The settle cutoff, read from the SAME connection the `updated_at` values are compared on:
/// `datetime('now', '-N seconds')`. Taking it from the process clock instead compares two
/// clocks that can disagree — a forward step on either side parks the watermark at a time the
/// rows never reach, and the run goes on attesting against a cutoff its own store cannot
/// explain (ruling R-S10, review W2).
pub fn settle_cutoff(conn: &mut SqliteConnection) -> Result<String, String> {
    Ok(diesel::sql_query(format!(
        "SELECT datetime('now', '-{SETTLE_SECONDS} seconds') AS at_key"
    ))
    .get_result::<AtRow>(conn)
    .map_err(fault("read the database clock"))?
    .at_key)
}

fn count_after_sql() -> String {
    format!(
        "SELECT count(*) AS n FROM content \
         WHERE {AT_KEY} > ?1 OR ({AT_KEY} = ?1 AND id > ?2)"
    )
}

/// A diesel error, named by the read it failed.
fn fault(what: &'static str) -> impl Fn(diesel::result::Error) -> String {
    move |e| format!("fold: {what}: {e}")
}

/// A unit's chunks, by the declared rule: `head`, the body's sections, `tags`. Returns the unit
/// and how many body chunks the rule's per-unit cap dropped (counted, never silently lost).
pub fn unit_chunks(rule: &ChunkRule, row: &ContentRow, tags: &[String]) -> (UnitChunks, usize) {
    let mut chunks = Vec::new();
    let head = match row.description.as_deref().map(str::trim) {
        Some(description) if !description.is_empty() => format!("{}\n\n{description}", row.title),
        _ => row.title.clone(),
    };
    if !head.trim().is_empty() {
        chunks.push(ChunkRow {
            section: HEAD_SECTION.to_string(),
            text: head,
            vector: None,
        });
    }
    let mut dropped = 0;
    if let Some(body) = row.content_body.as_deref().filter(|b| !b.trim().is_empty()) {
        // The rule selects its splitter by extension: a markdown body splits at headings,
        // every other body is cut into line windows.
        let markdown = matches!(
            row.content_format.to_ascii_lowercase().as_str(),
            "markdown" | "md"
        );
        let name = if markdown { "body.md" } else { "body.txt" };
        let chunked = rule.chunk(name, body);
        dropped = chunked.dropped;
        chunks.extend(chunked.chunks.into_iter().map(|chunk| ChunkRow {
            section: chunk.section,
            text: chunk.text,
            vector: None,
        }));
    }
    if !tags.is_empty() {
        chunks.push(ChunkRow {
            section: TAGS_SECTION.to_string(),
            text: tags.join(", "),
            vector: None,
        });
    }
    let skipped = chunks.is_empty().then(|| "no text".to_string());
    let unit = UnitChunks {
        unit_id: row.id.clone(),
        fingerprint: fingerprint(row.blob_cid.as_deref(), row.content_body.as_deref()),
        stat: Some(head_stat(
            &row.title,
            row.description.as_deref(),
            &row.content_format,
            tags,
        )),
        skipped,
        chunks,
    };
    (unit, dropped)
}

/// What one fold run did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FoldReport {
    /// Content rows read past the watermark this run.
    pub seen: usize,
    /// Units (re)chunked and written live.
    pub folded: usize,
    /// Rows seen whose fingerprint and stat were unchanged.
    pub unchanged: usize,
    /// Units demoted because their row is gone.
    pub demoted: usize,
    /// Live units the demotion sweep compared against `content` this run (its bounded page).
    pub swept: usize,
    /// Rows in this batch whose `updated_at` SQLite's `datetime()` could not read (review m2).
    pub unparsed_at: usize,
    /// Rows the fold has read and could not date since the store was built — this batch's count
    /// OR'd into the one persisted in `meta` (ruling R-S11, review W3). This, and not this
    /// batch's own count, is what the run's attestation reads: a rebuild clears it, a quiet
    /// batch does not.
    pub unparsed_at_known: usize,
    /// Body chunks the per-unit cap dropped.
    pub dropped: usize,
    /// Content rows past the last row this run read.
    pub behind: u64,
    /// Whether the watermark moved.
    pub advanced: bool,
    /// Whether the watermark reached the last row this run read — the batch is behind it, so a
    /// backlog can be drained by running again straight away.
    pub passed_batch: bool,
    /// The attestation this run recorded, if it recorded one.
    pub attestation_cid: Option<String>,
}

/// What the fold last attested, for an answer to print.
#[derive(Debug, Clone, Default)]
pub struct FoldSnapshot {
    /// Fold runs this process completed (successfully or not).
    pub runs: u64,
    /// The latest attestation (from this process, or the one found beside the store at open).
    pub attestation: Option<FoldAttestation>,
    pub attestation_cid: Option<String>,
    /// Rows behind as of the last run; `None` until a run has measured it.
    pub behind: Option<u64>,
    /// The last run's error, cleared by a run that succeeds.
    pub last_error: Option<String>,
}

/// The content lexical fold: its declarations, its store, and the loop that keeps it current.
pub struct SearchIndex {
    store_dir: PathBuf,
    declared: Declared,
    attested_by: String,
    /// How the store was found at open: `created`, `reused` or `rebuilt (<why>)`.
    opened: String,
    store: Mutex<Store>,
    notify: tokio::sync::Notify,
    state: RwLock<FoldSnapshot>,
}

impl SearchIndex {
    /// Open (or create, or rebuild) the fold store for `declared` under `storage_dir`. A store that
    /// is missing is created; one that fails its integrity check, or was built under another schema
    /// or measure, is removed and created again — derived state is replaced, never repaired.
    pub fn open(
        storage_dir: &Path,
        declared: Declared,
        attested_by: impl Into<String>,
    ) -> Result<Self, String> {
        let dir = store_dir(storage_dir, &declared.measure_cid);
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("content search: create {}: {e}", dir.display()))?;
        let path = dir.join(STORE_FILE);
        let wanted: BTreeMap<String, String> = [
            (META_SCHEMA, SCHEMA_VERSION),
            (META_MEASURE, declared.measure_cid.as_str()),
            (META_UNIT, "content.id"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

        let (store, opened) = match Store::open_readable(&path, false, true) {
            Ok(Some(store)) => {
                let meta = store.meta().unwrap_or_default();
                let foreign = wanted.iter().find(|(k, v)| meta.get(*k) != Some(*v));
                match foreign {
                    None => (store, "reused".to_string()),
                    Some((key, _)) => {
                        drop(store);
                        let why = format!("built under another {key}");
                        (Self::recreate(&path, &wanted)?, format!("rebuilt ({why})"))
                    }
                }
            }
            Ok(None) => (Self::create(&path, &wanted)?, "created".to_string()),
            Err(why) => (Self::recreate(&path, &wanted)?, format!("rebuilt ({why})")),
        };

        let attestation = std::fs::read(dir.join(LATEST_FILE))
            .ok()
            .and_then(|raw| serde_json::from_slice::<FoldAttestation>(&raw).ok())
            .filter(|att| att.measure.to_string() == declared.measure_cid);
        let attestation_cid = attestation
            .as_ref()
            .and_then(|att| att.cid().ok())
            .map(|cid| cid.to_string());
        Ok(Self {
            store_dir: dir,
            declared,
            attested_by: attested_by.into(),
            opened,
            store: Mutex::new(store),
            notify: tokio::sync::Notify::new(),
            state: RwLock::new(FoldSnapshot {
                attestation,
                attestation_cid,
                ..FoldSnapshot::default()
            }),
        })
    }

    fn create(path: &Path, meta: &BTreeMap<String, String>) -> Result<Store, String> {
        Store::create(path, meta)
            .map_err(|e| format!("content search: create {}: {e}", path.display()))
    }

    fn recreate(path: &Path, meta: &BTreeMap<String, String>) -> Result<Store, String> {
        Store::remove(path)
            .map_err(|e| format!("content search: remove {}: {e}", path.display()))?;
        Self::create(path, meta)
    }

    /// The fold's directory (`<storage_dir>/index/<measure-cid>`).
    pub fn store_dir(&self) -> &Path {
        &self.store_dir
    }

    /// The fold store's path.
    pub fn store_path(&self) -> PathBuf {
        self.store_dir.join(STORE_FILE)
    }

    /// How the store was found at open: `created`, `reused` or `rebuilt (<why>)`.
    pub fn opened(&self) -> &str {
        &self.opened
    }

    /// The measure and recipe this fold runs under.
    pub fn declared(&self) -> &Declared {
        &self.declared
    }

    /// What the fold last attested.
    pub fn snapshot(&self) -> FoldSnapshot {
        self.state.read().map(|s| s.clone()).unwrap_or_default()
    }

    /// Read the store (a query, a tally) under the fold's lock.
    pub fn with_store<R>(&self, read: impl FnOnce(&Store) -> R) -> R {
        let store = self
            .store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        read(&store)
    }

    /// Content rows past the fold's watermark right now — what the fold has yet to read. A row
    /// inside the settle window counts until the watermark passes it.
    pub fn behind(&self, conn: &mut SqliteConnection) -> Result<u64, String> {
        let meta = self.with_store(|store| store.meta().map_err(|e| e.to_string()))?;
        let at = meta.get(META_WATERMARK_AT).cloned().unwrap_or_default();
        let id = meta.get(META_WATERMARK_ID).cloned().unwrap_or_default();
        Ok(diesel::sql_query(count_after_sql())
            .bind::<Text, _>(&at)
            .bind::<Text, _>(&id)
            .get_result::<CountRow>(conn)
            .map_err(fault("count behind"))?
            .n
            .max(0) as u64)
    }

    /// Wake the loop: something in `content` changed. A wake that arrives while a run is under
    /// way is kept (one permit), so the next run starts as soon as this one ends.
    pub fn notify(&self) {
        self.notify.notify_one();
    }

    /// One fold run over `conn` (see the module docs). Errors are recorded as a `failed`
    /// attestation (once per distinct reason) and returned.
    pub fn fold_once(&self, conn: &mut SqliteConnection) -> Result<FoldReport, String> {
        let result = self.fold_run(conn);
        let mut state = self.state.write().unwrap_or_else(|p| p.into_inner());
        state.runs += 1;
        match &result {
            Ok(report) => {
                state.behind = Some(report.behind);
                state.last_error = None;
            }
            Err(why) => {
                let repeated = matches!(
                    state.attestation.as_ref().map(|att| &att.state),
                    Some(FoldState::Failed { why: last }) if last == why
                );
                state.last_error = Some(why.clone());
                if !repeated {
                    if let Ok((att, cid)) = self.attest(FoldState::Failed { why: why.clone() }) {
                        state.attestation = Some(att);
                        state.attestation_cid = Some(cid);
                    }
                }
            }
        }
        result
    }

    fn fold_run(&self, conn: &mut SqliteConnection) -> Result<FoldReport, String> {
        let mut store = self.store.lock().unwrap_or_else(|p| p.into_inner());
        let meta = store.meta().map_err(|e| e.to_string())?;
        let mark = (
            meta.get(META_WATERMARK_AT).cloned().unwrap_or_default(),
            meta.get(META_WATERMARK_ID).cloned().unwrap_or_default(),
        );
        let cap = self.declared.fold_lag_limit().max(1) as i64;
        let rows: Vec<ContentRow> = diesel::sql_query(rows_after_sql())
            .bind::<Text, _>(&mark.0)
            .bind::<Text, _>(&mark.1)
            .bind::<BigInt, _>(cap)
            .load(conn)
            .map_err(fault("read content"))?;

        // Batch-scoped: only the units this run is about to compare, never the whole fold.
        let batch_ids: Vec<String> = rows.iter().map(|row| row.id.clone()).collect();
        let live: BTreeMap<String, LiveUnit> = store
            .live_units_for(&batch_ids)
            .map_err(|e| e.to_string())?;
        let mut report = FoldReport {
            seen: rows.len(),
            ..FoldReport::default()
        };
        let mut insert = Vec::new();
        for row in &rows {
            let tags: Vec<String> = diesel::sql_query(
                "SELECT tag FROM content_tags WHERE h_app_id = ?1 AND content_id = ?2 ORDER BY tag",
            )
            .bind::<Text, _>(&row.h_app_id)
            .bind::<Text, _>(&row.id)
            .load::<TagRow>(conn)
            .map_err(fault("read tags"))?
            .into_iter()
            .map(|t| t.tag)
            .collect();
            let (unit, dropped) = unit_chunks(&self.declared.chunk_rule, row, &tags);
            let unchanged = live
                .get(&row.id)
                .is_some_and(|held| held.fingerprint == unit.fingerprint && held.stat == unit.stat);
            if unchanged {
                report.unchanged += 1;
            } else {
                report.dropped += dropped;
                insert.push(unit);
            }
            report.unparsed_at += usize::from(row.at_unparsed != 0);
        }

        // A unit whose row is gone is demoted — swept ONE bounded page of the store's own ids per
        // run, from a cursor that drains across runs, and asked about in a single `IN` query.
        // Materialising every live unit and every content id each run made a no-op sweep cost the
        // whole corpus twice (ruling R-S10, reviews W1 and W3).
        let cursor = meta.get(META_DEMOTE_CURSOR).cloned().unwrap_or_default();
        let page = store
            .live_unit_ids_after(&cursor, cap as u32)
            .map_err(|e| e.to_string())?;
        report.swept = page.len();
        let present: BTreeSet<String> = if page.is_empty() {
            BTreeSet::new()
        } else {
            let places = std::iter::repeat_n("?", page.len())
                .collect::<Vec<_>>()
                .join(",");
            let mut query =
                diesel::sql_query(format!("SELECT id FROM content WHERE id IN ({places})"))
                    .into_boxed();
            for id in &page {
                query = query.bind::<Text, _>(id);
            }
            query
                .load::<IdRow>(conn)
                .map_err(fault("check swept ids"))?
                .into_iter()
                .map(|r| r.id)
                .collect()
        };
        let demote: Vec<String> = page
            .iter()
            .filter(|id| !present.contains(*id))
            .cloned()
            .collect();
        // A short page is the end of a cycle: the next run starts the walk again.
        let next_cursor = match page.last() {
            Some(last) if page.len() as i64 == cap => last.clone(),
            _ => String::new(),
        };

        // The watermark: the last row read, but never past the settle window — and the window is
        // measured by the database's own clock, the one the rows are stamped and compared on.
        let last = rows.last().map(|r| (r.at_key.clone(), r.id.clone()));
        let cutoff = settle_cutoff(conn)?;
        let next_mark = match &last {
            None => mark.clone(),
            Some(key) if key.0 < cutoff => key.clone(),
            Some(_) => std::cmp::max(mark.clone(), (cutoff, String::new())),
        };
        report.advanced = next_mark != mark;
        report.passed_batch = last.as_ref() == Some(&next_mark);
        let behind_from = last.clone().unwrap_or_else(|| mark.clone());
        report.behind = diesel::sql_query(count_after_sql())
            .bind::<Text, _>(&behind_from.0)
            .bind::<Text, _>(&behind_from.1)
            .get_result::<CountRow>(conn)
            .map_err(fault("count behind"))?
            .n
            .max(0) as u64;

        report.folded = insert.len();
        report.demoted = demote.len();
        if report.unparsed_at > 0 {
            let named: Vec<&str> = rows
                .iter()
                .filter(|row| row.at_unparsed != 0)
                .map(|row| row.id.as_str())
                .take(5)
                .collect();
            tracing::warn!(
                unparsed = report.unparsed_at,
                ids = ?named,
                "content search: rows whose updated_at datetime() cannot read — they order \
                 lexically and the fold attests degraded rather than complete"
            );
        }
        // The unparsed tally is sticky until a rebuild: this run's count, or the one the store
        // already carries, whichever is larger. A row the fold could not date sits at an unknown
        // place in the watermark's order, and the next run reading only datable rows does not
        // make that hole go away (ruling R-S11, review W3).
        let carried: usize = meta
            .get(META_UNPARSED_AT)
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        report.unparsed_at_known = carried.max(report.unparsed_at);
        let unparsed_moved = report.unparsed_at_known != carried;
        let changed = !insert.is_empty() || !demote.is_empty();
        let cursor_moved = next_cursor != cursor;
        if changed || report.advanced || cursor_moved || unparsed_moved {
            let at = chrono::Utc::now().timestamp();
            store
                .fold_txn(at, &demote, &insert, |tx| {
                    let unparsed = report.unparsed_at_known.to_string();
                    for (key, value) in [
                        (META_WATERMARK_AT, next_mark.0.as_str()),
                        (META_WATERMARK_ID, next_mark.1.as_str()),
                        (META_DEMOTE_CURSOR, next_cursor.as_str()),
                        (META_UNPARSED_AT, unparsed.as_str()),
                    ] {
                        tx.execute(
                            "INSERT INTO meta (key, value) VALUES (?1, ?2) \
                             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                            [key, value],
                        )?;
                    }
                    Ok(())
                })
                .map_err(|e| e.to_string())?;
        }
        drop(store);

        // A row whose `updated_at` cannot be read is unaccounted for — its place in the
        // watermark's order is a guess — so the run says degraded rather than claiming a complete
        // fold of rows it could not date. `FoldAttestation` carries no omissions list, so the
        // state is where this lands; the ids are named in the log above (review m2). The count is
        // the store's persisted one, not just this batch's: a degraded verdict earned by an
        // undated row stands until the fold is rebuilt (ruling R-S11, review W3).
        let state_now = if report.behind == 0 && report.unparsed_at_known == 0 {
            FoldState::Complete
        } else {
            FoldState::Degraded {
                retried: next_retry(&self.store_dir),
            }
        };
        let first_this_process = self.snapshot().runs == 0;
        let state_moved = self
            .snapshot()
            .attestation
            .map(|att| std::mem::discriminant(&att.state) != std::mem::discriminant(&state_now))
            .unwrap_or(true);
        if changed || first_this_process || state_moved {
            let (att, cid) = self.attest(state_now)?;
            report.attestation_cid = Some(cid.clone());
            let mut state = self.state.write().unwrap_or_else(|p| p.into_inner());
            state.attestation = Some(att);
            state.attestation_cid = Some(cid);
        }
        Ok(report)
    }

    /// Record an attestation of the store as it stands, in `state`.
    fn attest(&self, state: FoldState) -> Result<(FoldAttestation, String), String> {
        let store = self.store.lock().unwrap_or_else(|p| p.into_inner());
        let shard = store.shard().map_err(|e| e.to_string())?;
        let root = atom_cid(&SURFACE_ROOT).map_err(|e| e.to_string())?;
        let every = |_: &str| true;
        let heads_at = store.heads(&[(root, &every)]).map_err(|e| e.to_string())?;
        drop(store);
        let attestation = FoldAttestation {
            measure: self.declared.measure.cid().map_err(|e| e.to_string())?,
            shard,
            heads_at,
            state,
            attested_by: AgentRef(self.attested_by.clone()),
            at: chrono::Utc::now().timestamp(),
        };
        let cid = record(&self.store_dir, &attestation).map_err(|e| e.to_string())?;
        Ok((attestation, cid))
    }

    /// Fold at boot, then on every [`Self::notify`] and every `period`. A run that left rows
    /// behind and moved the watermark past its whole batch is followed straight away by the next, so a backlog drains
    /// at one batch per run rather than one batch per period.
    pub async fn run_loop(self: Arc<Self>, pool: DbPool, period: Duration) {
        // bounded-work: each run reads at most `fold_lag.limit` (200) rows in one transaction; an
        // immediate re-run happens only while a full batch moved the watermark, otherwise the
        // loop sleeps until a notify or `period`. A failing run never retries early.
        loop {
            let this = self.clone();
            let pool_for_run = pool.clone();
            let outcome = tokio::task::spawn_blocking(move || {
                let mut conn = pool_for_run.get().map_err(|e| format!("fold: pool: {e}"))?;
                this.fold_once(&mut conn)
            })
            .await;
            let drain_on = match outcome {
                Ok(Ok(report)) => {
                    if report.folded > 0 || report.demoted > 0 {
                        tracing::info!(
                            folded = report.folded,
                            demoted = report.demoted,
                            unchanged = report.unchanged,
                            behind = report.behind,
                            attestation = report.attestation_cid.as_deref().unwrap_or("-"),
                            "content search: fold"
                        );
                    }
                    report.behind > 0 && report.passed_batch
                }
                Ok(Err(why)) => {
                    tracing::warn!(error = %why, "content search: fold failed");
                    false
                }
                Err(join) => {
                    tracing::warn!(error = %join, "content search: fold task panicked");
                    false
                }
            };
            if drain_on {
                tokio::task::yield_now().await;
                continue;
            }
            tokio::select! {
                _ = self.notify.notified() => {}
                _ = tokio::time::sleep(period) => {}
            }
        }
    }
}
