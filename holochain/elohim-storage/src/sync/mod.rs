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
pub mod stream;

pub use doc_store::{DocStore, DocStoreConfig, StoredDocument};
pub use stream::{StreamPosition, StreamTracker};

use crate::error::StorageError;
use automerge::Automerge;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Sync manager coordinates document synchronization
pub struct SyncManager {
    /// Document store
    doc_store: Arc<DocStore>,
    /// Stream position tracker
    stream_tracker: Arc<StreamTracker>,
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
    pub async fn get_or_create_doc(&self, app_id: &str, doc_id: &str) -> Result<Automerge, StorageError> {
        match self.doc_store.get(app_id, doc_id).await? {
            Some(stored) => {
                let doc = Automerge::load(&stored.data)
                    .map_err(|e| StorageError::Sync(format!("Failed to load doc: {}", e)))?;
                Ok(doc)
            }
            None => {
                let doc = Automerge::new();
                self.doc_store.save(app_id, doc_id, &doc).await?;
                Ok(doc)
            }
        }
    }

    /// Apply changes from a peer
    pub async fn apply_changes(
        &self,
        app_id: &str,
        doc_id: &str,
        changes: Vec<Vec<u8>>,
    ) -> Result<Vec<String>, StorageError> {
        let mut doc = self.get_or_create_doc(app_id, doc_id).await?;

        // Apply each change blob incrementally
        for change_bytes in changes {
            doc.load_incremental(&change_bytes)
                .map_err(|e| StorageError::Sync(format!("Failed to apply changes: {}", e)))?;
        }

        // Save updated document
        self.doc_store.save(app_id, doc_id, &doc).await?;

        // Return new heads
        let heads: Vec<String> = doc
            .get_heads()
            .iter()
            .map(|h| hex::encode(h.0))
            .collect();

        debug!(app_id = %app_id, doc_id = %doc_id, heads = ?heads, "Applied changes, new heads");
        Ok(heads)
    }

    /// Get changes since given heads
    pub async fn get_changes_since(
        &self,
        app_id: &str,
        doc_id: &str,
        have_heads: &[String],
    ) -> Result<(Vec<Vec<u8>>, Vec<String>), StorageError> {
        let doc = match self.doc_store.get(app_id, doc_id).await? {
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
        let new_heads: Vec<String> = doc
            .get_heads()
            .iter()
            .map(|h| hex::encode(h.0))
            .collect();

        Ok((changes, new_heads))
    }

    /// Get current heads for a document
    pub async fn get_heads(&self, app_id: &str, doc_id: &str) -> Result<Vec<String>, StorageError> {
        match self.doc_store.get(app_id, doc_id).await? {
            Some(stored) => {
                let doc = Automerge::load(&stored.data)
                    .map_err(|e| StorageError::Sync(format!("Failed to load doc: {}", e)))?;
                let heads: Vec<String> = doc
                    .get_heads()
                    .iter()
                    .map(|h| hex::encode(h.0))
                    .collect();
                Ok(heads)
            }
            None => Ok(vec![]),
        }
    }

    /// List documents for an app
    pub async fn list_documents(
        &self,
        app_id: &str,
        prefix: Option<&str>,
        offset: u32,
        limit: u32,
    ) -> Result<(Vec<StoredDocument>, u64), StorageError> {
        self.doc_store.list(app_id, prefix, offset, limit).await
    }

    /// Get document count for an app
    pub async fn count_documents(&self, app_id: &str) -> Result<u64, StorageError> {
        self.doc_store.count(app_id).await
    }
}
