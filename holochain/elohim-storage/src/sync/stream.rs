//! Stream Position Tracker - Matrix-inspired sync positions
//!
//! Tracks sync positions for incremental document synchronization.
//! Each peer maintains their position in each document's change stream.
//!
//! ## Design (Matrix-inspired)
//!
//! ```text
//! Stream: [change_0] → [change_1] → [change_2] → [change_3] → ...
//!                                        ↑
//!                              peer_A.position = 2
//! ```
//!
//! When peer_A syncs, they request changes since position 2,
//! receiving [change_3, ...] and updating their position.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::error::StorageError;

/// Stream position for a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamPosition {
    /// Document ID
    pub doc_id: String,
    /// Position in the change stream (index)
    pub position: u64,
    /// Last known head hash at this position
    pub last_head: String,
    /// Timestamp of last sync
    pub last_sync: u64,
    /// Peer ID this position is for (empty for local)
    pub peer_id: String,
}

/// Tracks sync positions for documents
pub struct StreamTracker {
    /// In-memory positions (peer_id:doc_id -> position)
    positions: RwLock<HashMap<String, StreamPosition>>,
    /// Persistent storage (optional sled tree)
    db: Option<sled::Tree>,
}

impl StreamTracker {
    /// Create a new in-memory stream tracker
    pub fn new() -> Self {
        Self {
            positions: RwLock::new(HashMap::new()),
            db: None,
        }
    }

    /// Create a stream tracker with persistence
    pub fn with_persistence(db: sled::Tree) -> Self {
        // Load existing positions from disk
        let mut positions = HashMap::new();
        for item in db.iter() {
            if let Ok((key, value)) = item {
                if let Ok(pos) = rmp_serde::from_slice::<StreamPosition>(&value) {
                    let key_str = String::from_utf8_lossy(&key).to_string();
                    positions.insert(key_str, pos);
                }
            }
        }

        info!(count = positions.len(), "StreamTracker loaded positions");

        Self {
            positions: RwLock::new(positions),
            db: Some(db),
        }
    }

    /// Get position for a peer and document
    pub async fn get_position(&self, peer_id: &str, doc_id: &str) -> Option<StreamPosition> {
        let key = Self::make_key(peer_id, doc_id);
        self.positions.read().await.get(&key).cloned()
    }

    /// Update position for a peer and document
    pub async fn set_position(
        &self,
        peer_id: &str,
        doc_id: &str,
        position: u64,
        last_head: &str,
    ) -> Result<(), StorageError> {
        let key = Self::make_key(peer_id, doc_id);
        let pos = StreamPosition {
            doc_id: doc_id.to_string(),
            position,
            last_head: last_head.to_string(),
            last_sync: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            peer_id: peer_id.to_string(),
        };

        // Update in memory
        self.positions.write().await.insert(key.clone(), pos.clone());

        // Persist if we have a database
        if let Some(ref db) = self.db {
            let bytes = rmp_serde::to_vec(&pos)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;
            db.insert(key.as_bytes(), bytes)
                .map_err(|e| StorageError::Database(e.to_string()))?;
        }

        debug!(
            peer_id = %peer_id,
            doc_id = %doc_id,
            position = position,
            "Position updated"
        );

        Ok(())
    }

    /// Get all positions for a peer
    pub async fn get_peer_positions(&self, peer_id: &str) -> Vec<StreamPosition> {
        let prefix = format!("{}:", peer_id);
        self.positions
            .read()
            .await
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(_, v)| v.clone())
            .collect()
    }

    /// Get all positions for a document
    pub async fn get_doc_positions(&self, doc_id: &str) -> Vec<StreamPosition> {
        let suffix = format!(":{}", doc_id);
        self.positions
            .read()
            .await
            .iter()
            .filter(|(k, _)| k.ends_with(&suffix))
            .map(|(_, v)| v.clone())
            .collect()
    }

    /// Remove position for a peer and document
    pub async fn remove_position(
        &self,
        peer_id: &str,
        doc_id: &str,
    ) -> Result<bool, StorageError> {
        let key = Self::make_key(peer_id, doc_id);
        let existed = self.positions.write().await.remove(&key).is_some();

        if let Some(ref db) = self.db {
            db.remove(key.as_bytes())
                .map_err(|e| StorageError::Database(e.to_string()))?;
        }

        Ok(existed)
    }

    /// Remove all positions for a peer
    pub async fn remove_peer(&self, peer_id: &str) -> Result<u64, StorageError> {
        let prefix = format!("{}:", peer_id);
        let mut positions = self.positions.write().await;
        let keys_to_remove: Vec<String> = positions
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .cloned()
            .collect();

        let count = keys_to_remove.len() as u64;

        for key in &keys_to_remove {
            positions.remove(key);
            if let Some(ref db) = self.db {
                db.remove(key.as_bytes())
                    .map_err(|e| StorageError::Database(e.to_string()))?;
            }
        }

        info!(peer_id = %peer_id, count = count, "Removed peer positions");
        Ok(count)
    }

    /// Get local position for a document (peer_id = "local")
    pub async fn get_local_position(&self, doc_id: &str) -> Option<StreamPosition> {
        self.get_position("local", doc_id).await
    }

    /// Set local position for a document
    pub async fn set_local_position(
        &self,
        doc_id: &str,
        position: u64,
        last_head: &str,
    ) -> Result<(), StorageError> {
        self.set_position("local", doc_id, position, last_head)
            .await
    }

    /// Flush to disk (if persistent)
    pub async fn flush(&self) -> Result<(), StorageError> {
        if let Some(ref db) = self.db {
            db.flush_async()
                .await
                .map_err(|e| StorageError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Make key for position lookup
    fn make_key(peer_id: &str, doc_id: &str) -> String {
        format!("{}:{}", peer_id, doc_id)
    }
}

impl Default for StreamTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stream_tracker() {
        let tracker = StreamTracker::new();

        // Set position
        tracker
            .set_position("peer1", "doc1", 5, "abc123")
            .await
            .unwrap();

        // Get position
        let pos = tracker.get_position("peer1", "doc1").await.unwrap();
        assert_eq!(pos.position, 5);
        assert_eq!(pos.last_head, "abc123");

        // Update position
        tracker
            .set_position("peer1", "doc1", 10, "def456")
            .await
            .unwrap();

        let pos = tracker.get_position("peer1", "doc1").await.unwrap();
        assert_eq!(pos.position, 10);
        assert_eq!(pos.last_head, "def456");

        // Remove position
        assert!(tracker.remove_position("peer1", "doc1").await.unwrap());
        assert!(tracker.get_position("peer1", "doc1").await.is_none());
    }

    #[tokio::test]
    async fn test_local_positions() {
        let tracker = StreamTracker::new();

        tracker
            .set_local_position("my-doc", 42, "head123")
            .await
            .unwrap();

        let pos = tracker.get_local_position("my-doc").await.unwrap();
        assert_eq!(pos.position, 42);
        assert_eq!(pos.peer_id, "local");
    }
}
