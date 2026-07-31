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
    /// This SWEEP is over: every discovered gap was healed OR exhausted its
    /// retry budget. Load-bearing for the existing `/health`, `/p2p/status`
    /// and a2o surfaces; deliberately NOT renamed. It does not mean the peer
    /// converged — see [`GapCounts::converged`].
    pub caught_up: bool,
    /// Gaps abandoned at `max_retries` — counted, never silently dropped.
    /// `enqueue_missing` refuses to re-queue these, so they are permanently
    /// absent from `pending`; that absence is why `pending.is_empty()`
    /// overstates convergence.
    pub exhausted: usize,
    /// This PEER holds what its peers advertised: nothing pending AND nothing
    /// abandoned. Strictly stronger than `caught_up`; this is the field an SLO
    /// may be offered over.
    pub converged: bool,
}

impl GapTracker {
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }

    /// Raise (or lower) the retry budget on a LIVE tracker.
    ///
    /// The acquisition stream sizes its budget from the CONNECTED PEER COUNT —
    /// `MAX_RETRIES = 3` on a 6-peer fabric probes at most 3 of 6 possible
    /// providers before declaring an item unsatisfiable, so "exhausted" meant
    /// "half the fabric was never asked". Peer count changes between reconcile
    /// cycles and trackers outlive a cycle, so the budget must be adjustable in
    /// place. RAISING it re-admits ids whose failed-count is below the new bound
    /// on the next `reconcile_desired` — which is the point.
    pub fn set_max_retries(&mut self, max_retries: u32) {
        self.max_retries = max_retries;
    }

    /// Ids whose retry budget is spent. `enqueue_missing` refuses to re-queue
    /// these, so they leave `pending` permanently — the exact reason
    /// `pending.is_empty()` (i.e. `caught_up`) overstates convergence.
    pub fn exhausted_count(&self) -> usize {
        self.failed
            .values()
            .filter(|&&n| n >= self.max_retries)
            .count()
    }

    pub fn counts(&self) -> GapCounts {
        let exhausted = self.exhausted_count();
        GapCounts {
            pending: self.pending.len(),
            completed: self.completed.len(),
            failed: self.failed.len(),
            caught_up: self.caught_up,
            exhausted,
            converged: self.pending.is_empty() && exhausted == 0,
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

    /// Acquisition-only: count wanted ids that are ALREADY present locally as
    /// completed, so the acquisition rollup's `total == fetched` holds for
    /// content the node already holds. `enqueue_missing` skips already-local
    /// ids (neither pending nor completed), so without this they inflate
    /// `total` but never `fetched` → the pull stream reports `caughtUp=false`
    /// forever on a content-holder node (the live matthew case).
    ///
    /// This is NOT a false-complete: R-A's "never claim done before bytes
    /// ARRIVE" is satisfied — the bytes ARE present locally (that is what
    /// `local_ids` means). Distinct from `mark_completed` (the cross-stream
    /// byte-ARRIVAL signal). Idempotent; leaves `pending`/`failed` untouched.
    pub fn mark_local_wants_satisfied(&mut self, want_ids: &[String]) {
        for id in want_ids {
            if self.local_ids.contains(id) {
                self.completed.insert(id.clone());
            }
        }
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
    fn retry_exhausted_gaps_are_caught_up_but_not_converged() {
        // The live lie: a sweep whose gaps all failed past max_retries drains
        // `pending` (mark_failed removes without re-queue) and reports
        // caught_up=true while nothing was healed. That is "this sweep is
        // over", not "this peer holds what its peers hold" — the shape behind
        // caughtUp:true over 1860 divergent anchors.
        let mut t = GapTracker::new(2);
        t.discover(vec!["a".into(), "b".into(), "c".into()]);
        for _ in 0..2 {
            for id in ["a", "b", "c"] {
                t.mark_failed(id);
            }
            // Retry-on-NEXT-cycle: re-discovery is what re-enqueues (R-E).
            t.discover(vec!["a".into(), "b".into(), "c".into()]);
        }
        t.update_caught_up();

        let c = t.counts();
        assert_eq!(c.completed, 0, "nothing was healed");
        assert!(
            c.caught_up,
            "existing semantics preserved: the sweep is over"
        );
        assert_eq!(c.exhausted, 3, "all three exhausted their retry budget");
        assert!(
            !c.converged,
            "converged must be FALSE when gaps were abandoned, not healed"
        );
    }

    #[test]
    fn a_fully_healed_sweep_is_both_caught_up_and_converged() {
        let mut t = GapTracker::new(2);
        t.discover(vec!["a".into(), "b".into()]);
        t.mark_completed("a");
        t.mark_completed("b");
        t.update_caught_up();

        let c = t.counts();
        assert!(c.caught_up);
        assert!(c.converged, "healed gaps converge");
        assert_eq!(c.exhausted, 0);
    }

    #[test]
    fn a_gap_still_retrying_is_neither_caught_up_nor_converged() {
        // Guards the other direction: a failure that has budget left is still
        // in flight, so `exhausted` must not count it.
        let mut t = GapTracker::new(3);
        t.discover(vec!["a".into()]);
        t.mark_failed("a");
        t.discover(vec!["a".into()]); // re-queued: 1 < 3
        t.update_caught_up();

        let c = t.counts();
        assert!(!c.caught_up);
        assert!(!c.converged);
        assert_eq!(c.exhausted, 0, "retryable != exhausted");
    }

    #[test]
    fn raising_the_budget_re_admits_an_exhausted_id() {
        // Peer-breadth: an item asked of 3 peers on a 6-peer fabric has not been
        // proven unsatisfiable — five of six providers may never have been
        // probed. Raising the budget to the peer count must let it back in.
        let mut t = GapTracker::new(3);
        t.reconcile_desired(vec!["x".into()]);
        for _ in 0..3 {
            t.mark_failed("x");
            t.reconcile_desired(vec!["x".into()]);
        }
        assert_eq!(t.counts().exhausted, 1, "spent at the 3-retry budget");

        t.set_max_retries(6);
        let gaps = t.reconcile_desired(vec!["x".into()]);
        assert_eq!(
            gaps,
            vec!["x".to_string()],
            "re-admitted under the new bound"
        );
        assert_eq!(
            t.counts().exhausted,
            0,
            "not exhausted while unprobed providers remain"
        );
    }

    #[test]
    fn dispatch_budget_caps_inflight() {
        assert_eq!(DispatchBudget::new(50).available(47), 3);
        assert_eq!(DispatchBudget::new(50).available(50), 0);
        assert_eq!(DispatchBudget::new(50).available(60), 0); // saturating
    }
}
