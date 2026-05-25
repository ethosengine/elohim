//! Identity-driven replication state tracking.
//!
//! Tracks what content this node should have vs what it has, and manages
//! the fetch queue for pulling missing content from peers.

use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use ts_rs::TS;

/// Replication progress exposed via /p2p/status
#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ReplicationStatus {
    /// Content IDs discovered but not yet fetched
    #[ts(type = "number")]
    pub pending: usize,
    /// Content IDs successfully replicated
    #[ts(type = "number")]
    pub completed: usize,
    /// Content IDs that failed fetch (will retry)
    #[ts(type = "number")]
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

    /// Register content IDs already in local DB (call on startup).
    ///
    /// A pod that restarts with content already replicated locally is caught
    /// up to the extent of its knowledge — there are no known gaps. We surface
    /// that immediately so the seeder's caughtUp poll doesn't hang for 600s
    /// waiting on a peer-discovery event that may not arrive (matthew might
    /// also be mid-restart, the periodic ListContent tick is on a long
    /// cadence, etc). When a peer subsequently advertises new gaps via
    /// `discover()`, caught_up correctly resets to false until those drain.
    ///
    /// Fresh pods (empty `ids`) stay `caught_up=false` until at least one
    /// inventory exchange completes — otherwise the seeder would pass before
    /// any content has actually replicated.
    pub async fn set_local_ids(&self, ids: HashSet<String>) {
        let had_content = !ids.is_empty();
        let mut inner = self.inner.write().await;
        inner.local_ids = ids;
        if had_content && inner.pending.is_empty() {
            inner.caught_up = true;
        }
    }

    /// Discover content from a peer inventory. Returns IDs that are new gaps.
    ///
    /// All remote IDs are scanned; already-known items (local, completed,
    /// in-flight pending, or exhausted retries) are skipped. New gaps are added
    /// to `pending` and returned so the caller can enqueue them for dispatch.
    /// Throttling is handled by `drain_gap_queue` — not here.
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
            if inner.failed.get(&id).copied().unwrap_or(0) >= MAX_RETRIES {
                continue;
            }
            inner.pending.insert(id.clone());
            new_gaps.push(id);
        }
        // Maintain the invariant: caught_up=false whenever pending is non-empty.
        // `set_local_ids` can flip caught_up=true on a restored pod; without
        // this guard, a subsequent inventory exchange that uncovers new gaps
        // would leave caught_up stuck at true until a mark_completed/mark_failed
        // path called update_caught_up.
        if !new_gaps.is_empty() {
            inner.caught_up = false;
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

    /// Mark a content ID as failed (will retry up to MAX_RETRIES).
    ///
    /// Items are removed from pending but NOT re-queued here. `discover()` will
    /// re-include them on the next replication cycle (when the peer re-advertises
    /// its inventory) if `fail_count < MAX_RETRIES`. This prevents a burst of
    /// timed-out requests from being immediately re-dispatched, which was the
    /// mechanism that caused replication to freeze at a partial completion count.
    pub async fn mark_failed(&self, id: &str) {
        let mut inner = self.inner.write().await;
        inner.pending.remove(id);
        let count = inner.failed.entry(id.to_string()).or_insert(0);
        *count += 1;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn replication_state_discovers_gaps() {
        let state = ReplicationState::new();

        // Simulate local node has items a, b, c
        let local: HashSet<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        state.set_local_ids(local).await;

        // Peer advertises items a–e
        let remote: Vec<String> = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let gaps = state.discover(remote).await;

        assert_eq!(gaps.len(), 2);
        assert!(gaps.contains(&"d".to_string()));
        assert!(gaps.contains(&"e".to_string()));

        let status = state.status().await;
        assert_eq!(status.pending, 2);
        assert_eq!(status.completed, 0);
        assert!(!status.caught_up);
    }

    #[tokio::test]
    async fn replication_state_marks_completed() {
        let state = ReplicationState::new();
        state.set_local_ids(HashSet::new()).await;

        state.discover(vec!["x".to_string()]).await;
        assert_eq!(state.status().await.pending, 1);

        state.mark_completed("x").await;
        let status = state.status().await;
        assert_eq!(status.pending, 0);
        assert_eq!(status.completed, 1);

        state.update_caught_up().await;
        assert!(state.status().await.caught_up);
    }

    #[tokio::test]
    async fn replication_state_retries_failures() {
        let state = ReplicationState::new();
        state.set_local_ids(HashSet::new()).await;

        state.discover(vec!["y".to_string()]).await;
        assert_eq!(state.status().await.pending, 1);

        // First failure: removed from pending, NOT re-queued immediately.
        // discover() on the next cycle will re-add it (fail_count=1 < MAX_RETRIES=3).
        state.mark_failed("y").await;
        let s = state.status().await;
        assert_eq!(s.pending, 0);
        assert_eq!(s.failed, 1);

        // Simulate next cycle: discover re-queues the item (count < MAX_RETRIES)
        let gaps = state.discover(vec!["y".to_string()]).await;
        assert_eq!(gaps.len(), 1);
        assert_eq!(state.status().await.pending, 1);

        // Second failure
        state.mark_failed("y").await;
        assert_eq!(state.status().await.pending, 0);

        // Simulate next cycle again
        let gaps = state.discover(vec!["y".to_string()]).await;
        assert_eq!(gaps.len(), 1);

        // Third failure: exhausted MAX_RETRIES
        state.mark_failed("y").await;

        // discover() no longer re-queues (fail_count=3 >= MAX_RETRIES=3)
        let gaps = state.discover(vec!["y".to_string()]).await;
        assert_eq!(gaps.len(), 0);
        let s = state.status().await;
        assert_eq!(s.pending, 0);
        assert_eq!(s.failed, 1);
    }

    #[tokio::test]
    async fn restored_pod_with_local_content_is_caught_up_on_startup() {
        // A pod that restarts with content already replicated locally must
        // report caught_up=true immediately so the seeder's 600s poll doesn't
        // hang waiting on a peer-discovery event that may not fire in time.
        let state = ReplicationState::new();
        assert!(
            !state.status().await.caught_up,
            "initial state is not caught up"
        );

        let local: HashSet<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        state.set_local_ids(local).await;

        let status = state.status().await;
        assert!(
            status.caught_up,
            "restored pod with local content must be caught up"
        );
        assert_eq!(status.pending, 0);
        assert_eq!(status.completed, 0);
    }

    #[tokio::test]
    async fn fresh_pod_with_no_local_content_stays_not_caught_up() {
        // A fresh pod (no local content) must NOT claim caught_up until at
        // least one inventory exchange completes — otherwise the seeder
        // would pass before any content has actually replicated.
        let state = ReplicationState::new();
        state.set_local_ids(HashSet::new()).await;

        let status = state.status().await;
        assert!(
            !status.caught_up,
            "fresh pod must wait for inventory exchange"
        );
    }

    #[tokio::test]
    async fn pending_gaps_keep_restored_pod_not_caught_up() {
        // If pending gaps already exist (somehow) when set_local_ids runs,
        // we must not falsely flip caught_up.
        let state = ReplicationState::new();
        state.discover(vec!["new-item".to_string()]).await;
        assert_eq!(state.status().await.pending, 1);

        let local: HashSet<String> = ["existing".to_string()].into_iter().collect();
        state.set_local_ids(local).await;

        assert!(
            !state.status().await.caught_up,
            "pending gaps must keep caught_up=false even with local content"
        );
    }

    #[tokio::test]
    async fn replication_state_skips_known_local_ids() {
        let state = ReplicationState::new();
        let local: HashSet<String> = ["a", "b"].iter().map(|s| s.to_string()).collect();
        state.set_local_ids(local).await;

        // Discover only items already held locally — no gaps expected
        let gaps = state.discover(vec!["a".to_string(), "b".to_string()]).await;
        assert_eq!(gaps.len(), 0);
        assert_eq!(state.status().await.pending, 0);
    }
}
