//! STORE — the fold store: one SQLite file per fold, derived and rebuildable, never synced.
//!
//! Schema v3 (post-station-4 sprint, ruling R-S1) keys every row by an opaque `unit_id` — a
//! repository-relative path to the recall executor, a content id to the storage peer — where v2
//! keyed them by `path`. A store of any other schema is not migrated: its owner rebuilds it.
//!
//! - `units` is the fold's manifest of what it has seen: each unit's fingerprint, the `stat` the
//!   owner compares to skip re-hashing (opaque here — the executor stores `"size:mtime_ns"`;
//!   `NULL` when the owner has none), how many chunks it yielded, and why it was skipped if it
//!   yielded none. A unit that yields no chunk is still seen, so it is not behind forever.
//! - `chunks` holds each chunk's text and, for a fold that embeds, its vector (little-endian
//!   `f32` × the pinned dims; `NULL` for a lexical-only fold). A row's text, unit and section are
//!   fixed once written; only `demoted_at` moves, and only once.
//! - `chunks_fts` is external-content FTS5 over `text` holding LIVE chunks only: a demotion issues
//!   FTS5's `'delete'` for that row, so a lexical ranking never meets a demoted chunk.
//!
//! Demotion, never deletion: `Retention` has no delete, and the store's own triggers refuse one.
//! Every write a fold makes lands in ONE transaction ([`Store::fold_txn`]).
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use cid::Cid;
use elohim_epr_rea::{atom_cid, ShardManifest};
use rusqlite::{params, Connection, OpenFlags, Transaction};

use crate::error::{IndexError, Result};

/// Bumped when the tables below change shape; a store of another version is rebuilt.
pub const SCHEMA_VERSION: &str = "3";

/// How long a reader waits on a fold's write transaction before reporting the store busy.
pub const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// The whole store, in one place.
pub const SCHEMA: &str = "
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE units (
  unit_id TEXT PRIMARY KEY,
  fingerprint TEXT NOT NULL,
  stat TEXT,
  chunks INTEGER NOT NULL,
  skipped TEXT,
  folded_at INTEGER NOT NULL,
  demoted_at INTEGER
);
CREATE TABLE chunks (
  id INTEGER PRIMARY KEY,
  unit_id TEXT NOT NULL,
  section TEXT NOT NULL,
  ordinal INTEGER NOT NULL,
  fingerprint TEXT NOT NULL,
  text TEXT NOT NULL,
  vector BLOB,
  folded_at INTEGER NOT NULL,
  demoted_at INTEGER
);
CREATE INDEX chunks_by_unit ON chunks(unit_id, demoted_at);
CREATE VIRTUAL TABLE chunks_fts USING fts5(
  text, unit_id UNINDEXED, section UNINDEXED, content='chunks', content_rowid='id',
  tokenize='unicode61'
);
CREATE TRIGGER chunks_fts_insert AFTER INSERT ON chunks BEGIN
  INSERT INTO chunks_fts(rowid, text, unit_id, section)
    VALUES (new.id, new.text, new.unit_id, new.section);
END;
CREATE TRIGGER chunks_fts_demote AFTER UPDATE OF demoted_at ON chunks
WHEN old.demoted_at IS NULL AND new.demoted_at IS NOT NULL BEGIN
  INSERT INTO chunks_fts(chunks_fts, rowid, text, unit_id, section)
    VALUES ('delete', old.id, old.text, old.unit_id, old.section);
END;
CREATE TRIGGER chunks_demoted_once BEFORE UPDATE OF demoted_at ON chunks
WHEN old.demoted_at IS NOT NULL BEGIN
  SELECT RAISE(ABORT, 'a demoted chunk stays demoted; a returning unit folds new rows');
END;
CREATE TRIGGER chunks_identity_fixed BEFORE UPDATE OF text, unit_id, section ON chunks BEGIN
  SELECT RAISE(ABORT, 'a chunk''s text, unit and section are fixed; demote it and fold another');
END;
CREATE TRIGGER chunks_never_deleted BEFORE DELETE ON chunks BEGIN
  SELECT RAISE(ABORT, 'demotion, never deletion');
END;
CREATE TRIGGER units_never_deleted BEFORE DELETE ON units BEGIN
  SELECT RAISE(ABORT, 'demotion, never deletion');
END;
";

/// A live unit as the store holds it: its fingerprint and the stat its owner recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveUnit {
    pub fingerprint: String,
    pub stat: Option<String>,
}

/// One chunk of a unit a fold writes; its ordinal is its position in [`UnitChunks::chunks`].
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkRow {
    pub section: String,
    pub text: String,
    /// The embedded vector's bytes (see [`crate::rank::encode`]); `None` for a lexical-only fold.
    pub vector: Option<Vec<u8>>,
}

/// A unit a fold (re)writes: its fingerprint (every chunk carries it), its stat, why it yielded
/// no chunk if it was skipped, and its chunks in order.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitChunks {
    pub unit_id: String,
    pub fingerprint: String,
    pub stat: Option<String>,
    pub skipped: Option<String>,
    pub chunks: Vec<ChunkRow>,
}

/// What a store holds, counted: live chunks, demoted chunks, and the live text's bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tally {
    pub chunks: u64,
    pub demoted: u64,
    pub bytes: u64,
}

/// A declared root a fold's heads are taken per: its own CID, and which unit ids it holds.
pub type Root<'a> = (Cid, &'a dyn Fn(&str) -> bool);

/// An open fold store.
pub struct Store {
    conn: Connection,
}

impl Store {
    /// A NEW store at `path` (no file may be there — see [`Store::remove`]): the schema and the
    /// identity rows `meta`, written in one transaction.
    pub fn create(path: &Path, meta: &BTreeMap<String, String>) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        conn.busy_timeout(BUSY_TIMEOUT)?;
        let tx = conn.transaction()?;
        tx.execute_batch(SCHEMA)?;
        for (key, value) in meta {
            tx.execute(
                "INSERT INTO meta (key, value) VALUES (?1, ?2)",
                params![key, value],
            )?;
        }
        tx.commit()?;
        Ok(Self { conn })
    }

    /// Remove a store that cannot serve — its file and SQLite's journal, WAL and shared-memory
    /// siblings — so the next [`Store::create`] starts clean. Replaced, never patched.
    pub fn remove(path: &Path) -> std::io::Result<()> {
        for suffix in ["", "-journal", "-wal", "-shm"] {
            let stale = PathBuf::from(format!("{}{suffix}", path.display()));
            if stale.exists() {
                std::fs::remove_file(&stale)?;
            }
        }
        Ok(())
    }

    /// A store file that opens — and, when `check`, passes its integrity check — whatever it was
    /// built under: `Ok(None)` when there is no file, `Err(why)` when there is one that cannot
    /// serve. A query path may skip the check: integrity is the fold's job (it rebuilds a store
    /// that fails it), and a corrupt store a query touches still surfaces as the SQLite error of
    /// the read that meets it.
    pub fn open_readable(
        path: &Path,
        read_only: bool,
        check: bool,
    ) -> std::result::Result<Option<Self>, String> {
        if !path.is_file() {
            return Ok(None);
        }
        let flags = if read_only {
            OpenFlags::SQLITE_OPEN_READ_ONLY
        } else {
            OpenFlags::SQLITE_OPEN_READ_WRITE
        };
        let unreadable = |error: rusqlite::Error| format!("store unreadable: {error}");
        let conn = Connection::open_with_flags(path, flags).map_err(unreadable)?;
        conn.busy_timeout(BUSY_TIMEOUT).map_err(unreadable)?;
        let store = Self { conn };
        if check {
            let verdict: String = store
                .conn
                .query_row("PRAGMA quick_check", [], |row| row.get(0))
                .map_err(unreadable)?;
            if verdict != "ok" {
                return Err(format!("store fails its integrity check: {verdict}"));
            }
        }
        Ok(Some(store))
    }

    /// The identity rows the store was built under (and any running tallies beside them).
    pub fn meta(&self) -> Result<BTreeMap<String, String>> {
        let mut statement = self.conn.prepare("SELECT key, value FROM meta")?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Every unit the fold currently holds, with its fingerprint and recorded stat.
    pub fn live_units(&self) -> Result<BTreeMap<String, LiveUnit>> {
        let mut statement = self
            .conn
            .prepare("SELECT unit_id, fingerprint, stat FROM units WHERE demoted_at IS NULL")?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                LiveUnit {
                    fingerprint: row.get(1)?,
                    stat: row.get(2)?,
                },
            ))
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Live chunks, demoted chunks, and the live text's bytes.
    pub fn tally(&self) -> Result<Tally> {
        let count = |sql: &str| -> Result<u64> {
            Ok(self
                .conn
                .query_row(sql, [], |row| row.get::<_, i64>(0))
                .map(|n| n.max(0) as u64)?)
        };
        Ok(Tally {
            chunks: count("SELECT count(*) FROM chunks WHERE demoted_at IS NULL")?,
            demoted: count("SELECT count(*) FROM chunks WHERE demoted_at IS NOT NULL")?,
            bytes: count(
                "SELECT coalesce(sum(length(CAST(text AS BLOB))), 0) FROM chunks \
                 WHERE demoted_at IS NULL",
            )?,
        })
    }

    /// What the fold holds: live chunk count, their text bytes, and the CID of the sorted
    /// `(unit_id, section, fingerprint)` manifest.
    pub fn shard(&self) -> Result<ShardManifest> {
        let mut statement = self.conn.prepare(
            "SELECT unit_id, section, fingerprint, length(CAST(text AS BLOB)) FROM chunks \
             WHERE demoted_at IS NULL",
        )?;
        let mut rows: Vec<(String, String, String)> = Vec::new();
        let mut bytes = 0u64;
        let mut query = statement.query([])?;
        while let Some(row) = query.next()? {
            rows.push((row.get(0)?, row.get(1)?, row.get(2)?));
            bytes += row.get::<_, i64>(3)?.max(0) as u64;
        }
        rows.sort();
        Ok(ShardManifest {
            arc: None,
            atoms: rows.len() as u64,
            bytes,
            manifest: atom_cid(&rows)?,
        })
    }

    /// One `(root, head)` pair per declared root: the root's own CID, and the CID of the sorted
    /// `(unit_id, fingerprint)` list of the live units it `matches` — so a later fold can name
    /// which root moved.
    pub fn heads(&self, roots: &[Root<'_>]) -> Result<Vec<(Cid, Cid)>> {
        let units = self.live_units()?;
        let mut heads = Vec::new();
        for (root, matches) in roots {
            let under: Vec<(&String, &String)> = units
                .iter()
                .filter(|(unit_id, _)| matches(unit_id))
                .map(|(unit_id, row)| (unit_id, &row.fingerprint))
                .collect();
            heads.push((*root, atom_cid(&under)?));
        }
        Ok(heads)
    }

    /// One fold run's writes, in ONE transaction, at `at` (Unix seconds): each unit in `insert`
    /// has its live chunks demoted, its row (re)written live, and its chunks inserted in order;
    /// each unit in `demote` has its live chunks and its row demoted; then `within` makes the
    /// owner's own writes (a running tally, a recorded stat) inside the same transaction. Any
    /// error — the owner's included — rolls every write back.
    pub fn fold_txn(
        &mut self,
        at: i64,
        demote: &[String],
        insert: &[UnitChunks],
        within: impl FnOnce(&Transaction<'_>) -> Result<()>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        let demote_chunks = |tx: &Transaction<'_>, unit_id: &str| -> Result<()> {
            tx.execute(
                "UPDATE chunks SET demoted_at = ?2 WHERE unit_id = ?1 AND demoted_at IS NULL",
                params![unit_id, at],
            )?;
            Ok(())
        };
        for unit in insert {
            demote_chunks(&tx, &unit.unit_id)?;
            tx.execute(
                "INSERT INTO units (unit_id, fingerprint, stat, chunks, skipped, folded_at,
                   demoted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL)
                 ON CONFLICT(unit_id) DO UPDATE SET fingerprint = ?2, stat = ?3, chunks = ?4,
                   skipped = ?5, folded_at = ?6, demoted_at = NULL",
                params![
                    unit.unit_id,
                    unit.fingerprint,
                    unit.stat,
                    unit.chunks.len() as i64,
                    unit.skipped,
                    at
                ],
            )?;
        }
        for unit in insert {
            for (ordinal, chunk) in unit.chunks.iter().enumerate() {
                tx.execute(
                    "INSERT INTO chunks (unit_id, section, ordinal, fingerprint, text, vector,
                       folded_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        unit.unit_id,
                        chunk.section,
                        ordinal as i64,
                        unit.fingerprint,
                        chunk.text,
                        chunk.vector,
                        at
                    ],
                )?;
            }
        }
        for unit_id in demote {
            demote_chunks(&tx, unit_id)?;
            tx.execute(
                "UPDATE units SET demoted_at = ?2 WHERE unit_id = ?1 AND demoted_at IS NULL",
                params![unit_id, at],
            )?;
        }
        within(&tx)?;
        tx.commit()?;
        Ok(())
    }

    /// Visit every LIVE chunk the FTS5 `expression` matches, with its `bm25()` (lower is a better
    /// match); returns how many matched. The FTS table holds live chunks only (a demotion deletes
    /// its row), and the join to `chunks` keeps a demoted row out even if one ever lingered.
    pub fn lexical(&self, expression: &str, mut visit: impl FnMut(i64, &str, f64)) -> Result<u64> {
        let mut statement = self.conn.prepare(
            "SELECT chunks.id, chunks.unit_id, bm25(chunks_fts) FROM chunks_fts \
             JOIN chunks ON chunks.id = chunks_fts.rowid \
             WHERE chunks_fts MATCH ?1 AND chunks.demoted_at IS NULL",
        )?;
        let mut rows = statement.query(params![expression])?;
        let mut matched = 0u64;
        while let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let unit_id: String = row.get(1)?;
            let rank: f64 = row.get(2)?;
            visit(id, &unit_id, rank);
            matched += 1;
        }
        Ok(matched)
    }

    /// Visit every live chunk's `(id, unit_id, vector bytes)` — `None` where the fold embedded
    /// nothing; returns how many were visited.
    pub fn scan(&self, mut visit: impl FnMut(i64, &str, Option<&[u8]>)) -> Result<u64> {
        let mut statement = self
            .conn
            .prepare("SELECT id, unit_id, vector FROM chunks WHERE demoted_at IS NULL")?;
        let mut rows = statement.query([])?;
        let mut seen = 0u64;
        while let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let unit_id: String = row.get(1)?;
            let vector = row.get_ref(2)?.as_blob_or_null().map_err(|error| {
                IndexError::Io(std::io::Error::other(format!("fold store: {error}")))
            })?;
            visit(id, &unit_id, vector);
            seen += 1;
        }
        Ok(seen)
    }

    /// One chunk's `(section, text)`.
    pub fn chunk(&self, id: i64) -> Result<(String, String)> {
        Ok(self.conn.query_row(
            "SELECT section, text FROM chunks WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?)
    }
}
