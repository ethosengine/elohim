//! Acquisition reconcile stream state (spec §4) — the sibling of the
//! replication stream. Per-pin GapTrackers over the DECLARED desired set;
//! all state Category C (in-memory, recomputed on restart from active pins
//! × local inventory). Wire vocabulary is the unified set (spec §4.3):
//! {total, fetched, pending, failed, caughtUp}.

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use ts_rs::TS;

use super::reconcile_rails::GapTracker;

const MAX_RETRIES: u32 = 3;
/// Device-paced: deliberately below replication's 50 (R-E — the acquisition
/// stream serves a person's wants, not node policy; it must not starve the
/// node-level stream).
pub const MAX_ACQUISITION_INFLIGHT: usize = 25;

/// Pull-queue rollup. None on the wire means "cannot compute" = keep waiting
/// (the wait-for-drain tri-state contract, spec §4.3) — never caught-up.
#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PullStatusInfo {
    pub total: i32,
    pub fetched: i32,
    pub pending: i32,
    pub failed: i32,
    pub caught_up: bool,
}

/// Per-pin progress, served on GET /api/v1/pins (own node only).
#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PinPullStatus {
    pub pin_id: i32,
    pub total: i32,
    pub fetched: i32,
    pub pending: i32,
    pub failed: i32,
    pub caught_up: bool,
}

#[derive(Debug, Default)]
struct AcquisitionInner {
    /// pin row id → tracker over that pin's resolved item set
    trackers: HashMap<i32, GapTracker>,
    /// pin row id → resolved desired-set size (Slice 1: 1 for item pins)
    totals: HashMap<i32, usize>,
    /// content id → pin ids wanting it (completion fan-out)
    wanted_by: HashMap<String, Vec<i32>>,
}

#[derive(Debug, Clone, Default)]
pub struct AcquisitionState {
    inner: Arc<RwLock<AcquisitionInner>>,
}

impl AcquisitionState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reconcile the declared wants (spec §4.2 step 1–2). `pin_wants` maps
    /// each active pin to its resolved item ids; `local_has` is the set of
    /// ids already present in the local projection. Returns newly-pending
    /// content ids in pin-priority order (caller passes pins pre-sorted).
    pub async fn reconcile(
        &self,
        pin_wants: Vec<(i32, Vec<String>)>,
        local_has: &std::collections::HashSet<String>,
    ) -> Vec<String> {
        let mut inner = self.inner.write().await;
        // Drop trackers for pins no longer active (un-pinned → queue drains)
        let live: std::collections::HashSet<i32> = pin_wants.iter().map(|(id, _)| *id).collect();
        inner.trackers.retain(|id, _| live.contains(id));
        inner.totals.retain(|id, _| live.contains(id));
        inner.wanted_by.retain(|_, pins| {
            pins.retain(|p| live.contains(p));
            !pins.is_empty()
        });

        let mut to_dispatch = Vec::new();
        for (pin_id, want_ids) in pin_wants {
            inner.totals.insert(pin_id, want_ids.len());
            for id in &want_ids {
                let entry = inner.wanted_by.entry(id.clone()).or_default();
                if !entry.contains(&pin_id) {
                    entry.push(pin_id);
                }
            }
            let tracker = inner
                .trackers
                .entry(pin_id)
                .or_insert_with(|| GapTracker::new(MAX_RETRIES));
            // TODO(cluster-pins): clones local_has once per pin — fine for
            // Slice-1 item pins (tiny sets); revisit with a shared Arc or a
            // borrow variant when cluster closures bring large desired sets.
            tracker.set_local_ids(local_has.clone());
            let gaps = tracker.reconcile_desired(want_ids);
            to_dispatch.extend(gaps);
        }
        to_dispatch
    }

    /// Byte-arrival done-signal (R-A): called from the ContentData completion
    /// path AFTER bulk_create_content succeeds — never on inventory receipt.
    pub async fn mark_completed(&self, content_id: &str) {
        let mut inner = self.inner.write().await;
        if let Some(pin_ids) = inner.wanted_by.get(content_id).cloned() {
            for pin_id in pin_ids {
                if let Some(t) = inner.trackers.get_mut(&pin_id) {
                    t.mark_completed(content_id);
                    t.update_caught_up();
                }
            }
        }
    }

    pub async fn mark_failed(&self, content_id: &str) {
        // No update_caught_up here (unlike mark_completed): a failed item
        // removed from pending does not satisfy the drain contract — only a
        // byte-arrival completion earns caught_up.
        let mut inner = self.inner.write().await;
        if let Some(pin_ids) = inner.wanted_by.get(content_id).cloned() {
            for pin_id in pin_ids {
                if let Some(t) = inner.trackers.get_mut(&pin_id) {
                    t.mark_failed(content_id);
                }
            }
        }
    }

    /// True if any tracker still wants this id (dispatch filter).
    pub async fn wants(&self, content_id: &str) -> bool {
        let inner = self.inner.read().await;
        inner
            .wanted_by
            .get(content_id)
            .map(|pins| {
                pins.iter().any(|p| {
                    inner
                        .trackers
                        .get(p)
                        .map(|t| t.wants(content_id))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    }

    pub async fn rollup(&self) -> PullStatusInfo {
        let inner = self.inner.read().await;
        let mut s = PullStatusInfo::default();
        for (pin_id, t) in &inner.trackers {
            let c = t.counts();
            s.total += *inner.totals.get(pin_id).unwrap_or(&0) as i32;
            s.fetched += c.completed as i32;
            s.pending += c.pending as i32;
            s.failed += c.failed as i32;
        }
        // total == 0 is resolved-empty, a DISTINCT state — NOT caught_up
        // (spec §4.3/§10: a zero-item desired set must never false-complete).
        s.caught_up = s.pending == 0 && s.total > 0;
        s
    }

    pub async fn per_pin(&self) -> Vec<PinPullStatus> {
        let inner = self.inner.read().await;
        let mut out: Vec<PinPullStatus> = inner
            .trackers
            .iter()
            .map(|(pin_id, t)| {
                let c = t.counts();
                let total = *inner.totals.get(pin_id).unwrap_or(&0) as i32;
                PinPullStatus {
                    pin_id: *pin_id,
                    total,
                    fetched: c.completed as i32,
                    pending: c.pending as i32,
                    failed: c.failed as i32,
                    caught_up: c.pending == 0 && total > 0,
                }
            })
            .collect();
        out.sort_by_key(|p| p.pin_id);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[tokio::test]
    async fn reconcile_diffs_wants_and_rolls_up() {
        let acq = AcquisitionState::new();
        let local: HashSet<String> = ["have-1".to_string()].into_iter().collect();
        let dispatch = acq
            .reconcile(
                vec![
                    (1, vec!["have-1".into(), "need-1".into()]),
                    (2, vec!["need-2".into()]),
                ],
                &local,
            )
            .await;
        assert_eq!(dispatch.len(), 2);
        let r = acq.rollup().await;
        assert_eq!((r.total, r.fetched, r.pending), (3, 0, 2));
        assert!(!r.caught_up);
    }

    #[tokio::test]
    async fn byte_arrival_completes_every_wanting_pin() {
        let acq = AcquisitionState::new();
        let local = HashSet::new();
        acq.reconcile(
            vec![(1, vec!["shared".into()]), (2, vec!["shared".into()])],
            &local,
        )
        .await;
        acq.mark_completed("shared").await;
        let pins = acq.per_pin().await;
        assert!(pins.iter().all(|p| p.caught_up && p.fetched == 1));
    }

    #[tokio::test]
    async fn resolved_empty_desired_set_is_not_caught_up() {
        // A pin whose desired set resolves to zero items (e.g. an empty
        // cluster closure) must surface as resolved-empty, NOT caught_up
        // (spec §4.3/§10 — never silently false-complete).
        let acq = AcquisitionState::new();
        let local = std::collections::HashSet::new();
        acq.reconcile(vec![(1, vec![])], &local).await;
        let r = acq.rollup().await;
        assert_eq!(r.total, 0);
        assert!(!r.caught_up, "zero-item desired set must not be caught_up");
        let pins = acq.per_pin().await;
        assert_eq!(pins.len(), 1);
        assert!(!pins[0].caught_up);
    }

    #[tokio::test]
    async fn unpinned_pin_drains_out_of_state() {
        let acq = AcquisitionState::new();
        let local = HashSet::new();
        acq.reconcile(vec![(1, vec!["a".into()]), (2, vec!["b".into()])], &local)
            .await;
        acq.reconcile(vec![(1, vec!["a".into()])], &local).await;
        let pins = acq.per_pin().await;
        assert_eq!(pins.len(), 1);
        assert_eq!(pins[0].pin_id, 1);
        assert!(!acq.wants("b").await);
    }
}
