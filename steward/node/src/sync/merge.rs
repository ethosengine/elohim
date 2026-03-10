//! CRDT merge logic using Automerge
//!
//! SQLite-backed Automerge document management for sync between peers.
//! Documents are stored as binary blobs and loaded on demand.

use std::path::Path;

use anyhow::{Context, Result};
use automerge::AutoCommit;
use rusqlite::Connection;
use tracing::{debug, info};

use super::stream::{SyncEvent, SyncState};

/// Automerge sync engine backed by SQLite for document persistence.
pub struct SyncEngine {
    db: Connection,
    state: SyncState,
}

impl SyncEngine {
    /// Create a new sync engine, opening or creating the SQLite database.
    pub fn new(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir).context("creating data directory")?;
        let db_path = data_dir.join("documents.db");
        let db = Connection::open(&db_path)
            .with_context(|| format!("opening database at {}", db_path.display()))?;

        // Enable WAL mode for concurrent read access
        db.execute_batch("PRAGMA journal_mode=WAL;")?;

        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS documents (
                doc_id TEXT PRIMARY KEY,
                data BLOB NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            );",
        )?;

        info!(path = %db_path.display(), "Sync engine initialized");

        Ok(Self {
            db,
            state: SyncState::new(),
        })
    }

    /// Load an Automerge document from the database.
    pub fn load_doc(&self, doc_id: &str) -> Result<Option<AutoCommit>> {
        let mut stmt = self
            .db
            .prepare_cached("SELECT data FROM documents WHERE doc_id = ?1")?;

        let result = stmt.query_row([doc_id], |row| {
            let data: Vec<u8> = row.get(0)?;
            Ok(data)
        });

        match result {
            Ok(data) => {
                let doc =
                    AutoCommit::load(&data).with_context(|| format!("loading doc {}", doc_id))?;
                Ok(Some(doc))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Save an Automerge document to the database.
    pub fn save_doc(&self, doc_id: &str, doc: &mut AutoCommit) -> Result<()> {
        let data = doc.save();
        self.db.execute(
            "INSERT INTO documents (doc_id, data, updated_at)
             VALUES (?1, ?2, strftime('%s', 'now'))
             ON CONFLICT(doc_id) DO UPDATE SET data = ?2, updated_at = strftime('%s', 'now')",
            rusqlite::params![doc_id, data],
        )?;
        debug!(doc_id, bytes = data.len(), "Saved document");
        Ok(())
    }

    /// Apply remote changes to a document (create if it doesn't exist).
    pub fn apply_remote_changes(&mut self, doc_id: &str, changes: &[Vec<u8>]) -> Result<()> {
        let mut doc = self.load_doc(doc_id)?.unwrap_or_else(AutoCommit::new);

        for change_bytes in changes {
            doc.load_incremental(change_bytes)
                .with_context(|| format!("applying change to doc {}", doc_id))?;
        }

        self.save_doc(doc_id, &mut doc)?;

        // Record in sync state
        let heads = doc.get_heads();
        if let Some(head) = heads.first() {
            let hash_str = format!("{}", head);
            self.state.record_local(doc_id.to_string(), hash_str);
        }

        debug!(
            doc_id,
            num_changes = changes.len(),
            "Applied remote changes"
        );
        Ok(())
    }

    /// Get changes that a peer doesn't have based on their known heads.
    pub fn get_changes_for_peer(
        &self,
        doc_id: &str,
        peer_heads: &[String],
    ) -> Result<Vec<Vec<u8>>> {
        let mut doc = match self.load_doc(doc_id)? {
            Some(doc) => doc,
            None => return Ok(vec![]),
        };

        if peer_heads.is_empty() {
            // Peer has nothing — send all changes
            return Ok(vec![doc.save()]);
        }

        // Parse peer heads into ChangeHash
        let mut heads = Vec::new();
        for h in peer_heads {
            if let Ok(bytes) = hex::decode(h) {
                if bytes.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    heads.push(automerge::ChangeHash(arr));
                }
            }
        }

        if heads.is_empty() {
            // Couldn't parse heads — send full doc
            return Ok(vec![doc.save()]);
        }

        let changes = doc.save_after(&heads);
        if changes.is_empty() {
            Ok(vec![])
        } else {
            Ok(vec![changes])
        }
    }

    /// Record a local change and produce a SyncEvent.
    #[allow(dead_code)]
    pub fn record_local_change(&mut self, doc_id: String, change_hash: String) -> SyncEvent {
        self.state.record_local(doc_id, change_hash)
    }

    /// Get events since a stream position.
    pub fn events_since(&self, position: u64) -> Vec<SyncEvent> {
        self.state.events_since(position)
    }

    /// Get the current local stream position.
    #[allow(dead_code)]
    pub fn local_position(&self) -> u64 {
        self.state.local_position
    }
}

/// Hex encoding helper (avoids adding hex crate dependency).
mod hex {
    pub fn decode(s: &str) -> Result<Vec<u8>, ()> {
        if !s.len().is_multiple_of(2) {
            return Err(());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use automerge::{transaction::Transactable, ReadDoc};
    use tempfile::TempDir;

    #[test]
    fn test_create_save_load_document() {
        let dir = TempDir::new().unwrap();
        let mut engine = SyncEngine::new(dir.path()).unwrap();

        // Create and save a document
        let mut doc = AutoCommit::new();
        doc.put(automerge::ROOT, "title", "Hello").unwrap();
        engine.save_doc("doc-1", &mut doc).unwrap();

        // Load it back
        let loaded = engine.load_doc("doc-1").unwrap().unwrap();
        let (val, _id) = loaded.get(automerge::ROOT, "title").unwrap().unwrap();
        assert_eq!(val.to_str().unwrap(), "Hello");
    }

    #[test]
    fn test_load_nonexistent_returns_none() {
        let dir = TempDir::new().unwrap();
        let engine = SyncEngine::new(dir.path()).unwrap();
        assert!(engine.load_doc("nope").unwrap().is_none());
    }

    #[test]
    fn test_apply_remote_changes() {
        let dir = TempDir::new().unwrap();
        let mut engine = SyncEngine::new(dir.path()).unwrap();

        // Create a doc on a "remote" node
        let mut remote_doc = AutoCommit::new();
        remote_doc.put(automerge::ROOT, "key", "value").unwrap();
        let saved = remote_doc.save();

        // Apply as remote changes
        engine.apply_remote_changes("doc-1", &[saved]).unwrap();

        // Verify doc exists
        let doc = engine.load_doc("doc-1").unwrap().unwrap();
        let (val, _id) = doc.get(automerge::ROOT, "key").unwrap().unwrap();
        assert_eq!(val.to_str().unwrap(), "value");
    }

    #[test]
    fn test_events_since() {
        let dir = TempDir::new().unwrap();
        let mut engine = SyncEngine::new(dir.path()).unwrap();

        let e1 = engine.record_local_change("doc-1".into(), "hash-a".into());
        let e2 = engine.record_local_change("doc-2".into(), "hash-b".into());

        assert_eq!(e1.position, 1);
        assert_eq!(e2.position, 2);

        let events = engine.events_since(0);
        assert_eq!(events.len(), 2);

        let events = engine.events_since(1);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].doc_id, "doc-2");
    }
}
