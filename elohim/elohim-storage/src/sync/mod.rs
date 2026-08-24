//! Sync Module - CRDT document synchronization
//!
//! This module provides offline-first document sync using Automerge CRDTs.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                         Sync Engine                              │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  DocStore      - Automerge document persistence (sled)          │
//! │  StreamTracker - Position tracking for incremental sync         │
//! │  SyncManager   - Orchestrates sync with peers                   │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Document Types
//!
//! - **Graph subgraphs**: Per-scope edge collections (path, personal, community)
//! - **User state**: Subscriptions, progress, preferences
//! - **Content metadata**: Node stubs for fog-of-war visibility

pub mod doc_store;
pub mod projector;
pub mod stream;

pub use doc_store::{DocStore, DocStoreConfig, StoredDocument};
pub use stream::{StreamPosition, StreamTracker};

use crate::error::StorageError;
use automerge::Automerge;
use automerge::ReadDoc;
use std::sync::Arc;
use tracing::debug;

/// Sync manager coordinates document synchronization
pub struct SyncManager {
    /// Document store
    doc_store: Arc<DocStore>,
    /// Stream position tracker
    #[allow(dead_code)]
    stream_tracker: Arc<StreamTracker>,
}

/// Parse a hex change hash into an Automerge `ChangeHash`.
///
/// `None` (never an error) for anything that is not 32 hex-encoded bytes: the
/// hash arrives off the wire, and an unparseable one must degrade to "cannot
/// serve this change", never to a failed sync round.
fn parse_change_hash(hex_hash: &str) -> Option<automerge::ChangeHash> {
    let bytes = hex::decode(hex_hash).ok()?;
    let arr: [u8; 32] = bytes.try_into().ok()?;
    Some(automerge::ChangeHash(arr))
}

impl SyncManager {
    /// Create a new sync manager
    pub fn new(doc_store: Arc<DocStore>, stream_tracker: Arc<StreamTracker>) -> Self {
        Self {
            doc_store,
            stream_tracker,
        }
    }

    /// Get or create a document
    pub async fn get_or_create_doc(
        &self,
        h_app_id: &str,
        doc_id: &str,
    ) -> Result<Automerge, StorageError> {
        match self.doc_store.get(h_app_id, doc_id).await? {
            Some(stored) => {
                let doc = Automerge::load(&stored.data)
                    .map_err(|e| StorageError::Sync(format!("Failed to load doc: {}", e)))?;
                Ok(doc)
            }
            None => {
                let doc = Automerge::new();
                self.doc_store.save(h_app_id, doc_id, &doc).await?;
                Ok(doc)
            }
        }
    }

    /// Apply changes from a peer
    pub async fn apply_changes(
        &self,
        h_app_id: &str,
        doc_id: &str,
        changes: Vec<Vec<u8>>,
    ) -> Result<Vec<String>, StorageError> {
        let mut doc = self.get_or_create_doc(h_app_id, doc_id).await?;

        // Apply each change blob incrementally
        for change_bytes in changes {
            doc.load_incremental(&change_bytes)
                .map_err(|e| StorageError::Sync(format!("Failed to apply changes: {}", e)))?;
        }

        // Save updated document
        self.doc_store.save(h_app_id, doc_id, &doc).await?;

        // Return new heads
        let heads: Vec<String> = doc.get_heads().iter().map(|h| hex::encode(h.0)).collect();

        debug!(h_app_id = %h_app_id, doc_id = %doc_id, heads = ?heads, "Applied changes, new heads");
        Ok(heads)
    }

    /// Get changes since given heads
    pub async fn get_changes_since(
        &self,
        h_app_id: &str,
        doc_id: &str,
        have_heads: &[String],
    ) -> Result<(Vec<Vec<u8>>, Vec<String>), StorageError> {
        let doc = match self.doc_store.get(h_app_id, doc_id).await? {
            Some(stored) => Automerge::load(&stored.data)
                .map_err(|e| StorageError::Sync(format!("Failed to load doc: {}", e)))?,
            None => return Ok((vec![], vec![])),
        };

        // Parse heads from hex strings
        let heads: Vec<automerge::ChangeHash> = have_heads
            .iter()
            .filter_map(|h| {
                let bytes = hex::decode(h).ok()?;
                if bytes.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    Some(automerge::ChangeHash(arr))
                } else {
                    None
                }
            })
            .collect();

        // Get changes the peer doesn't have as a single blob
        // save_after returns all changes not in the given heads
        let changes_blob = doc.save_after(&heads);

        // For now, return as a single chunk (could be split for large docs)
        let changes: Vec<Vec<u8>> = if changes_blob.is_empty() {
            vec![]
        } else {
            vec![changes_blob]
        };

        // Current heads
        let new_heads: Vec<String> = doc.get_heads().iter().map(|h| hex::encode(h.0)).collect();

        Ok((changes, new_heads))
    }

    /// Raw bytes of ONE change, addressed by its hex hash, if this doc holds it.
    ///
    /// The announce path's payload source. A doorbell names the change it is
    /// announcing, so the eager push should carry THAT change — not
    /// `get_changes_since(.., &[])`, which is every change since genesis and
    /// therefore grows without bound as a doc accumulates history (a mature doc
    /// would overflow the announce bound on every local edit and burn a full-doc
    /// serialization to discover that).
    ///
    /// `Ok(None)` for an absent doc, an unparseable hash, or a change the doc
    /// does not hold (evicted, compacted, or never seen) — all of which mean
    /// "cannot push this eagerly", never an error, because the caller's fallback
    /// (pull, then the 60s round) is always available.
    ///
    /// Also the receive-side landing check: a single change whose dependencies
    /// the receiver lacks is QUEUED by Automerge rather than applied, silently
    /// leaving the doc unchanged — so "did the announced change actually land?"
    /// is exactly `get_change_by_hash(..).is_some()` after the apply.
    pub async fn get_change_by_hash(
        &self,
        h_app_id: &str,
        doc_id: &str,
        change_hash: &str,
    ) -> Result<Option<Vec<u8>>, StorageError> {
        let Some(hash) = parse_change_hash(change_hash) else {
            return Ok(None);
        };
        let Some(stored) = self.doc_store.get(h_app_id, doc_id).await? else {
            return Ok(None);
        };
        let doc = Automerge::load(&stored.data)
            .map_err(|e| StorageError::Sync(format!("Failed to load doc: {}", e)))?;
        Ok(doc
            .get_change_by_hash(&hash)
            .map(|c| c.raw_bytes().to_vec()))
    }

    /// Get current heads for a document
    pub async fn get_heads(
        &self,
        h_app_id: &str,
        doc_id: &str,
    ) -> Result<Vec<String>, StorageError> {
        match self.doc_store.get(h_app_id, doc_id).await? {
            Some(stored) => {
                let doc = Automerge::load(&stored.data)
                    .map_err(|e| StorageError::Sync(format!("Failed to load doc: {}", e)))?;
                let heads: Vec<String> = doc.get_heads().iter().map(|h| hex::encode(h.0)).collect();
                Ok(heads)
            }
            None => Ok(vec![]),
        }
    }

    /// Read a single top-level string field from a document.
    ///
    /// Read-only accessor used by `sync::projector` and the convergence tests
    /// to assert a doc's projected scalar fields. Returns an error if the
    /// document does not exist, the field is absent, or the field is not a
    /// string scalar.
    pub async fn get_doc_field(
        &self,
        h_app_id: &str,
        doc_id: &str,
        field: &str,
    ) -> Result<String, StorageError> {
        let stored = self
            .doc_store
            .get(h_app_id, doc_id)
            .await?
            .ok_or_else(|| StorageError::Sync(format!("doc not found: {h_app_id}/{doc_id}")))?;
        let doc = Automerge::load(&stored.data)
            .map_err(|e| StorageError::Sync(format!("Failed to load doc: {e}")))?;
        doc.get(automerge::ROOT, field)
            .map_err(|e| StorageError::Sync(format!("Failed to read field {field}: {e}")))?
            .and_then(|(v, _)| match v {
                automerge::Value::Scalar(s) => match s.as_ref() {
                    automerge::ScalarValue::Str(smol) => Some(smol.to_string()),
                    _ => None,
                },
                _ => None,
            })
            .ok_or_else(|| StorageError::Sync(format!("field not found or not a string: {field}")))
    }

    /// C3 read side (Plan C3 / notary-authority): resolve the blob hash of the
    /// notary-DECLARED head for a content doc, honoring REQ-F4 (serve the
    /// declared head, NEVER a peer's un-elected recency head — the SSL-strip /
    /// laundering hazard, §7.3).
    ///
    /// Returns `Some(blob)` ONLY when the converged doc's `headActionHash` hint
    /// MATCHES `declared_head_action_hash` (the doc IS projecting the row's
    /// declared head) AND a non-empty head blob resolves via
    /// [`projector::read_head_blob_hash`]. `None` degrades the caller to the
    /// row's own blob (current serve behavior).
    ///
    /// READ-ONLY: never creates the doc (unlike `get_or_create_doc`), so a serve
    /// probe for a never-synced id has no write side-effect.
    pub async fn declared_head_blob(
        &self,
        h_app_id: &str,
        doc_id: &str,
        declared_head_action_hash: &str,
    ) -> Option<String> {
        let stored = self.doc_store.get(h_app_id, doc_id).await.ok()??;
        let doc = Automerge::load(&stored.data).ok()?;
        // REQ-F4 / no-laundering gate: the doc's declared-head hint must match
        // the row's declared head before we adopt the doc's head blob.
        if projector::doc_head_action_hash(&doc).as_deref() != Some(declared_head_action_hash) {
            return None;
        }
        projector::read_head_blob_hash(&doc).filter(|b| !b.is_empty())
    }

    /// List documents for an app
    pub async fn list_documents(
        &self,
        h_app_id: &str,
        prefix: Option<&str>,
        offset: u32,
        limit: u32,
    ) -> Result<(Vec<StoredDocument>, u64), StorageError> {
        self.doc_store.list(h_app_id, prefix, offset, limit).await
    }

    /// Get document count for an app
    pub async fn count_documents(&self, h_app_id: &str) -> Result<u64, StorageError> {
        self.doc_store.count(h_app_id).await
    }

    /// Every document in the store, across all namespaces. This is what a
    /// "how many sync documents does this peer hold" status field means;
    /// `count_documents` answers the per-app question.
    pub async fn count_all(&self) -> Result<u64, StorageError> {
        self.doc_store.count_all().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use automerge::transaction::Transactable;
    use tempfile::TempDir;

    async fn test_sync_manager() -> (SyncManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let doc_store = Arc::new(
            DocStore::new(DocStoreConfig {
                db_path: temp_dir.path().join("sync-mod.sled"),
                ..Default::default()
            })
            .await
            .unwrap(),
        );
        let stream_tracker = Arc::new(StreamTracker::new());
        (SyncManager::new(doc_store, stream_tracker), temp_dir)
    }

    /// Author two changes, then ask for ONE by hash: the returned bytes must be
    /// that change alone — materially smaller than the whole document — and must
    /// apply into a peer that already holds the predecessor.
    ///
    /// This is the announce payload's contract. The reason it matters is scale:
    /// the doorbell used to carry `get_changes_since(.., &[])`, which grows with
    /// the doc, so a mature doc overflowed the announce bound on every edit.
    #[tokio::test]
    async fn one_change_by_hash_is_a_delta_that_applies_on_top_of_its_predecessor() {
        let (sync, _tmp) = test_sync_manager().await;
        let ns = "elohim";
        let doc = "node:concept-1";

        let mut first = Automerge::new();
        first
            .transact::<_, _, automerge::AutomergeError>(|tx| {
                tx.put(automerge::ROOT, "title", "v1")?;
                Ok(())
            })
            .unwrap();
        sync.apply_changes(ns, doc, vec![first.save()])
            .await
            .unwrap();
        let after_first = sync.get_heads(ns, doc).await.unwrap();

        let mut second = sync.get_or_create_doc(ns, doc).await.unwrap();
        second
            .transact::<_, _, automerge::AutomergeError>(|tx| {
                tx.put(automerge::ROOT, "title", "v2")?;
                Ok(())
            })
            .unwrap();
        sync.apply_changes(ns, doc, vec![second.save()])
            .await
            .unwrap();
        let head = sync.get_heads(ns, doc).await.unwrap()[0].clone();
        assert_ne!(head, after_first[0], "the second change must move the head");

        let delta = sync
            .get_change_by_hash(ns, doc, &head)
            .await
            .unwrap()
            .expect("the doc holds its own head change");
        let (whole, _) = sync.get_changes_since(ns, doc, &[]).await.unwrap();
        let whole_len: usize = whole.iter().map(|c| c.len()).sum();
        assert!(
            delta.len() < whole_len,
            "one change ({}) must be smaller than the whole doc ({whole_len})",
            delta.len()
        );

        // A peer holding only the FIRST change applies the delta cleanly.
        let (peer, _peer_tmp) = test_sync_manager().await;
        peer.apply_changes(ns, doc, vec![first.save()])
            .await
            .unwrap();
        peer.apply_changes(ns, doc, vec![delta]).await.unwrap();
        assert_eq!(peer.get_doc_field(ns, doc, "title").await.unwrap(), "v2");
        assert_eq!(peer.get_heads(ns, doc).await.unwrap(), vec![head]);
    }

    /// `/p2p/status.syncDocuments` read 0 on a store holding 5,356 docs
    /// (fleet, 2026-08-24) because `count_documents("_all")` scanned the key
    /// prefix `"_all:"` while every doc lives under `"elohim:"`. The status
    /// projection must count the whole store, and a namespace count must
    /// stay namespace-scoped — both pinned here so neither drifts again.
    #[tokio::test]
    async fn count_all_counts_every_namespace_and_count_documents_stays_scoped() {
        let (sync, _tmp) = test_sync_manager().await;
        for (ns, doc) in [
            ("elohim", "node:a"),
            ("elohim", "node:b"),
            ("other", "node:c"),
        ] {
            let mut d = Automerge::new();
            d.transact::<_, _, automerge::AutomergeError>(|tx| {
                tx.put(automerge::ROOT, "title", doc)?;
                Ok(())
            })
            .unwrap();
            sync.apply_changes(ns, doc, vec![d.save()]).await.unwrap();
        }
        assert_eq!(sync.count_all().await.unwrap(), 3, "whole store");
        assert_eq!(sync.count_documents("elohim").await.unwrap(), 2, "scoped");
        assert_eq!(
            sync.count_documents("_all").await.unwrap(),
            0,
            "\"_all\" is a namespace name, not a wildcard — this is the bug's shape"
        );
    }

    /// Every "cannot serve this change" case is `Ok(None)`, never an error: the
    /// hash arrives off the wire, and the caller's fallback (pull, then the 60s
    /// round) must not be turned into a failed sync round by a bad string.
    #[tokio::test]
    async fn unservable_change_hashes_are_none_not_errors() {
        let (sync, _tmp) = test_sync_manager().await;
        let ns = "elohim";
        let doc = "node:concept-1";

        // Absent doc.
        assert!(sync
            .get_change_by_hash(ns, doc, &"ab".repeat(32))
            .await
            .unwrap()
            .is_none());

        let mut d = Automerge::new();
        d.transact::<_, _, automerge::AutomergeError>(|tx| {
            tx.put(automerge::ROOT, "title", "v1")?;
            Ok(())
        })
        .unwrap();
        sync.apply_changes(ns, doc, vec![d.save()]).await.unwrap();

        // Not hex, wrong length, and a well-formed hash the doc does not hold.
        for bad in ["", "zzzz", "abcd", &"cd".repeat(32)] {
            assert!(
                sync.get_change_by_hash(ns, doc, bad)
                    .await
                    .unwrap()
                    .is_none(),
                "{bad} must be Ok(None)"
            );
        }
    }
}
