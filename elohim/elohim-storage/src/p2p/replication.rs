//! Identity-driven replication state tracking.
//!
//! Tracks what content this node should have vs what it has, and manages
//! the fetch queue for pulling missing content from peers.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::Serialize;

/// Replication progress exposed via /p2p/status
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplicationStatus {
    /// Content IDs discovered but not yet fetched
    pub pending: usize,
    /// Content IDs successfully replicated
    pub completed: usize,
    /// Content IDs that failed fetch (will retry)
    pub failed: usize,
    /// True when all discovered content has been fetched or failed with max retries
    pub caught_up: bool,
}

/// Internal replication state (not serialized directly)
#[derive(Debug, Default)]
struct ReplicationInner {
    /// Content IDs discovered from peers, not yet in local DB
    pending: HashSet<String>,
    /// Content IDs successfully written to local DB
    completed: HashSet<String>,
    /// Content IDs that failed with retry count
    failed: HashMap<String, u32>,
    /// Set after first successful discovery + fetch cycle with no remaining gaps
    caught_up: bool,
    /// Content IDs already known to be in local DB (skip during discovery)
    local_ids: HashSet<String>,
}

const MAX_RETRIES: u32 = 3;

/// Thread-safe replication state manager
#[derive(Debug, Clone)]
pub struct ReplicationState {
    inner: Arc<RwLock<ReplicationInner>>,
}

impl ReplicationState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(ReplicationInner::default())),
        }
    }

    /// Snapshot current status for API reporting
    pub async fn status(&self) -> ReplicationStatus {
        let inner = self.inner.read().await;
        ReplicationStatus {
            pending: inner.pending.len(),
            completed: inner.completed.len(),
            failed: inner.failed.len(),
            caught_up: inner.caught_up,
        }
    }

    /// Register content IDs already in local DB (call on startup)
    pub async fn set_local_ids(&self, ids: HashSet<String>) {
        let mut inner = self.inner.write().await;
        inner.local_ids = ids;
    }

    /// Discover content from a peer inventory. Returns IDs that are new gaps.
    pub async fn discover(&self, remote_ids: Vec<String>) -> Vec<String> {
        let mut inner = self.inner.write().await;
        let mut new_gaps = Vec::new();
        for id in remote_ids {
            if inner.local_ids.contains(&id)
                || inner.completed.contains(&id)
                || inner.pending.contains(&id)
            {
                continue;
            }
            // Skip if already failed max times
            if inner.failed.get(&id).copied().unwrap_or(0) >= MAX_RETRIES {
                continue;
            }
            inner.pending.insert(id.clone());
            new_gaps.push(id);
        }
        new_gaps
    }

    /// Mark a content ID as successfully replicated
    pub async fn mark_completed(&self, id: &str) {
        let mut inner = self.inner.write().await;
        inner.pending.remove(id);
        inner.failed.remove(id);
        inner.completed.insert(id.to_string());
        inner.local_ids.insert(id.to_string());
    }

    /// Mark a content ID as failed (will retry up to MAX_RETRIES)
    pub async fn mark_failed(&self, id: &str) {
        let mut inner = self.inner.write().await;
        inner.pending.remove(id);
        let count = inner.failed.entry(id.to_string()).or_insert(0);
        *count += 1;
        // Re-queue if under retry limit
        if *count < MAX_RETRIES {
            inner.pending.insert(id.to_string());
        }
    }

    /// Check if all discovered content is fetched or exhausted retries
    pub async fn update_caught_up(&self) {
        let mut inner = self.inner.write().await;
        inner.caught_up = inner.pending.is_empty();
    }
}

impl Default for ReplicationState {
    fn default() -> Self {
        Self::new()
    }
}
