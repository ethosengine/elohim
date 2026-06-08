//! Shared reconcile-stream rails (spec §4.1, R-E): the gap state machine and
//! dispatch budget used by BOTH the replication stream (whole-inventory,
//! node-policy) and the acquisition stream (desired-set, user-declared).
//! ONE controller pattern governs all reconcile streams — a parallel bespoke
//! fetcher is a coherence violation.

use std::collections::{HashMap, HashSet};

/// Generic gap state machine: known-local / pending / completed / failed(retries).
/// Retry discipline is retry-on-NEXT-cycle (never immediate re-queue — the
/// freeze-at-partial battle-scar, see replication.rs mark_failed docs).
#[derive(Debug, Default)]
pub struct GapTracker {
    local_ids: HashSet<String>,
    pending: HashSet<String>,
    completed: HashSet<String>,
    failed: HashMap<String, u32>,
    caught_up: bool,
    max_retries: u32,
}

/// Snapshot counts for status surfaces.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GapCounts {
    pub pending: usize,
    pub completed: usize,
    pub failed: usize,
    pub caught_up: bool,
}

impl GapTracker {
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }

    pub fn counts(&self) -> GapCounts {
        GapCounts {
            pending: self.pending.len(),
            completed: self.completed.len(),
            failed: self.failed.len(),
            caught_up: self.caught_up,
        }
    }

    /// Register IDs already present locally. `flip_caught_up_if_restored`
    /// preserves replication.rs's restored-pod semantics; the acquisition
    /// stream passes `false` (a pin is never caught-up before reconcile).
    /// Does NOT drain `pending` of ids now present in `local_ids`; callers
    /// pair this with `mark_completed` for cross-stream byte-arrivals.
    pub fn set_local_ids_with(&mut self, ids: HashSet<String>, flip_caught_up_if_restored: bool) {
        let had_content = !ids.is_empty();
        self.local_ids = ids;
        if flip_caught_up_if_restored && had_content && self.pending.is_empty() {
            self.caught_up = true;
        }
    }

    pub fn set_local_ids(&mut self, ids: HashSet<String>) {
        self.set_local_ids_with(ids, false);
    }

    /// Inventory-driven entry (replication): remote advertises, we diff.
    pub fn discover(&mut self, remote_ids: Vec<String>) -> Vec<String> {
        self.enqueue_missing(remote_ids)
    }

    /// Desired-set-driven entry (acquisition): WE declare, then diff.
    /// Identical machine, different direction of declaration (spec §4.2).
    pub fn reconcile_desired(&mut self, want_ids: Vec<String>) -> Vec<String> {
        self.enqueue_missing(want_ids)
    }

    fn enqueue_missing(&mut self, ids: Vec<String>) -> Vec<String> {
        let mut new_gaps = Vec::new();
        for id in ids {
            if self.local_ids.contains(&id)
                || self.completed.contains(&id)
                || self.pending.contains(&id)
            {
                continue;
            }
            if self.failed.get(&id).copied().unwrap_or(0) >= self.max_retries {
                continue;
            }
            self.pending.insert(id.clone());
            new_gaps.push(id);
        }
        if !new_gaps.is_empty() {
            self.caught_up = false;
        }
        new_gaps
    }

    pub fn mark_completed(&mut self, id: &str) {
        self.pending.remove(id);
        self.failed.remove(id);
        self.completed.insert(id.to_string());
        self.local_ids.insert(id.to_string());
    }

    /// Removed from pending, NOT re-queued — the next reconcile/discover
    /// cycle re-includes it while fail_count < max_retries (R-E).
    pub fn mark_failed(&mut self, id: &str) {
        self.pending.remove(id);
        *self.failed.entry(id.to_string()).or_insert(0) += 1;
    }

    pub fn update_caught_up(&mut self) {
        self.caught_up = self.pending.is_empty();
    }

    /// True if this tracker currently has `id` in flight (in `pending`).
    /// Does NOT return true for retryable-next-cycle items — a failed-but-
    /// retryable id was removed from `pending` and re-enters only on the next
    /// `reconcile_desired`/`discover` call. Used as a dispatch-time filter.
    pub fn wants(&self, id: &str) -> bool {
        self.pending.contains(id)
    }

    /// Distinct ids currently in flight (in `pending`). Used by the per-EPR
    /// rollup to union across pins of the same head_ref (shared ids dedupe).
    pub fn pending_ids(&self) -> Vec<String> {
        self.pending.iter().cloned().collect()
    }

    /// Distinct ids byte-arrival complete (in `completed`).
    pub fn completed_ids(&self) -> Vec<String> {
        self.completed.iter().cloned().collect()
    }

    /// Distinct ids that have failed at least once (keys of `failed`).
    pub fn failed_ids(&self) -> Vec<String> {
        self.failed.keys().cloned().collect()
    }
}

/// Slot-backpressure helper (R-E): dispatch rate becomes a natural function
/// of peer response speed — mirrors drain_gap_queue's MAX_REPLICATION_INFLIGHT.
#[derive(Debug, Clone, Copy)]
pub struct DispatchBudget {
    max_inflight: usize,
}

impl DispatchBudget {
    pub fn new(max_inflight: usize) -> Self {
        Self { max_inflight }
    }
    pub fn available(&self, in_flight: usize) -> usize {
        self.max_inflight.saturating_sub(in_flight)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desired_set_reconcile_diffs_wants_against_local() {
        let mut t = GapTracker::new(3);
        t.set_local_ids(["a".into()].into_iter().collect());
        let gaps = t.reconcile_desired(vec!["a".into(), "b".into(), "c".into()]);
        assert_eq!(gaps.len(), 2);
        assert!(gaps.contains(&"b".to_string()) && gaps.contains(&"c".to_string()));
        let s = t.counts();
        assert_eq!((s.pending, s.completed, s.failed), (2, 0, 0));
        assert!(!s.caught_up);
    }

    #[test]
    fn exhausted_retries_drop_out_of_reconcile() {
        let mut t = GapTracker::new(1);
        t.reconcile_desired(vec!["x".into()]);
        t.mark_failed("x");
        // next cycle: fail_count=1 >= max_retries=1 → not re-queued
        let gaps = t.reconcile_desired(vec!["x".into()]);
        assert!(gaps.is_empty());
        assert_eq!(t.counts().failed, 1);
    }

    #[test]
    fn dispatch_budget_caps_inflight() {
        assert_eq!(DispatchBudget::new(50).available(47), 3);
        assert_eq!(DispatchBudget::new(50).available(50), 0);
        assert_eq!(DispatchBudget::new(50).available(60), 0); // saturating
    }
}
