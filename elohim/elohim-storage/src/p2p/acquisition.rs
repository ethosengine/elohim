//! Acquisition reconcile stream state (spec §4) — the sibling of the
//! replication stream. Per-pin GapTrackers over the DECLARED desired set;
//! all state Category C (in-memory, recomputed on restart from active pins
//! × local inventory). Wire vocabulary is the unified set (spec §4.3):
//! {total, fetched, pending, failed, caughtUp}.

use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use ts_rs::TS;

use super::reconcile_rails::GapTracker;
use elohim_views::acquisition::EprPullStatusView;

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
            let want_ids_for_local = want_ids.clone();
            let gaps = tracker.reconcile_desired(want_ids);
            // Already-local wants are satisfied (bytes present), not pending —
            // count them as fetched so the rollup can reach caught_up on a
            // content-holder node. See GapTracker::mark_local_wants_satisfied.
            tracker.mark_local_wants_satisfied(&want_ids_for_local);
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
                    // keeps the tracker's own bit fresh for symmetry with replication;
                    // the acquisition rollup recomputes independently (see rollup()).
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
        // caught_up is recomputed here from byte-arrival (fetched == total), NOT read from
        // the per-tracker GapTracker.caught_up bit (which the replication stream maintains
        // on a pending-based formula). The two are intentionally independent.
        // Caught-up means BYTE-ARRIVAL complete: every wanted item fetched
        // (spec R-A — never false-complete). A failed item (fetched < total
        // with pending transiently 0 before re-queue) must NOT report caught_up.
        // total == 0 (resolved-empty) is likewise not caught_up.
        s.caught_up = s.total > 0 && s.fetched == s.total;
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
                    caught_up: total > 0 && (c.completed as i32) == total,
                }
            })
            .collect();
        out.sort_by_key(|p| p.pin_id);
        out
    }

    /// Per-EPR rollup grouped by `head_ref`. `pin_heads` maps each live pin id
    /// to its `head_ref` (supplied by the HTTP handler from the DB — the
    /// AcquisitionState never learns head_ref itself). Returns `None` when no
    /// active tracker belongs to `epr_id` (handler maps that to 404).
    ///
    /// Shared content is counted ONCE: ids are deduped across all pins of the
    /// EPR before counting. `total == 0` (resolved-empty) surfaces as None
    /// total / None caughtUp — never false-complete (spec R-A, §4.3).
    pub async fn per_epr(
        &self,
        epr_id: &str,
        pin_heads: &HashMap<i32, String>,
    ) -> Option<EprPullStatusView> {
        let inner = self.inner.read().await;
        // Which live pins belong to this EPR?
        let group: Vec<i32> = inner
            .trackers
            .keys()
            .copied()
            .filter(|pid| pin_heads.get(pid).map(|h| h == epr_id).unwrap_or(false))
            .collect();
        if group.is_empty() {
            return None;
        }
        // Union the desired set and per-status id sets across the group's pins.
        let mut desired: HashSet<String> = HashSet::new();
        let mut completed: HashSet<String> = HashSet::new();
        let mut pending: HashSet<String> = HashSet::new();
        let mut failed: HashSet<String> = HashSet::new();
        for pid in &group {
            let t = match inner.trackers.get(pid) {
                Some(t) => t,
                None => continue,
            };
            for id in t.completed_ids() {
                desired.insert(id.clone());
                completed.insert(id);
            }
            for id in t.pending_ids() {
                desired.insert(id.clone());
                pending.insert(id);
            }
            for id in t.failed_ids() {
                desired.insert(id.clone());
                failed.insert(id);
            }
        }
        // Status precedence: a completed id is fetched even if another pin's
        // tracker still has it pending/failed. Subtract higher-precedence sets.
        pending.retain(|id| !completed.contains(id));
        failed.retain(|id| !completed.contains(id) && !pending.contains(id));

        let total_n = desired.len() as u64;
        let fetched = completed.len() as u64;
        // total == 0 resolved-empty → tri-state None (never caught up); matches
        // rollup()'s `s.total > 0` guard in this same file.
        let (total, caught_up) = if total_n == 0 {
            (None, None)
        } else {
            (Some(total_n), Some(fetched == total_n))
        };
        Some(EprPullStatusView {
            epr_id: epr_id.to_string(),
            total,
            fetched,
            pending: pending.len() as u64,
            failed: failed.len() as u64,
            caught_up,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // have-1 is already local → now counted as fetched (satisfied), so
        // fetched=1; need-1/need-2 are still pending. Not caught_up (3 != 1).
        assert_eq!((r.total, r.fetched, r.pending), (3, 1, 2));
        assert!(!r.caught_up);
    }

    #[tokio::test]
    async fn all_pinned_items_already_local_is_caught_up() {
        // The content-holder case (matthew holds the seeded corpus): a pin
        // whose every item is already present locally must report caught_up.
        // Today total counts already-local items but fetched does not, so the
        // pull stream is pull=false forever and Verify Projection Sync fails.
        let acq = AcquisitionState::new();
        let local: HashSet<String> = ["have-1".to_string()].into_iter().collect();
        let dispatch = acq
            .reconcile(vec![(1, vec!["have-1".into()])], &local)
            .await;
        assert!(
            dispatch.is_empty(),
            "already-local item needs no fetch dispatch"
        );
        let r = acq.rollup().await;
        assert_eq!((r.total, r.fetched, r.pending), (1, 1, 0));
        assert!(
            r.caught_up,
            "a pin whose every item is already local must be caught_up"
        );
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
    async fn failed_item_is_not_caught_up_despite_empty_pending() {
        // R-A: an item that failed its fetch (removed from pending until the
        // next reconcile re-queues it) leaves fetched < total — the pull must
        // NOT report caught_up even though pending is transiently 0.
        let acq = AcquisitionState::new();
        let local = std::collections::HashSet::new();
        acq.reconcile(vec![(1, vec!["need".into()])], &local).await;
        acq.mark_failed("need").await;
        let r = acq.rollup().await;
        assert_eq!((r.total, r.fetched, r.pending, r.failed), (1, 0, 0, 1));
        assert!(
            !r.caught_up,
            "a failed (unfetched) item must not be caught_up (R-A)"
        );
        let pins = acq.per_pin().await;
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

    #[tokio::test]
    async fn per_epr_groups_by_head_ref_counting_shared_content_once() {
        let acq = AcquisitionState::new();
        // pin 1 and pin 2 both belong to EPR "head-A" and both want "shared";
        // pin 1 additionally wants "only1", pin 2 additionally wants "only2".
        // pin 3 belongs to a different EPR "head-B".
        let local = HashSet::new();
        acq.reconcile(
            vec![
                (1, vec!["shared".into(), "only1".into()]),
                (2, vec!["shared".into(), "only2".into()]),
                (3, vec!["b1".into()]),
            ],
            &local,
        )
        .await;
        // byte-arrival completes the shared id (fans out to BOTH pins 1 & 2).
        acq.mark_completed("shared").await;

        let mut heads: HashMap<i32, String> = HashMap::new();
        heads.insert(1, "head-A".into());
        heads.insert(2, "head-A".into());
        heads.insert(3, "head-B".into());

        let a = acq.per_epr("head-A", &heads).await.expect("head-A present");
        // distinct desired ids for head-A = {shared, only1, only2} = 3, NOT 4.
        assert_eq!(a.total, Some(3), "shared id counted once across pins");
        // "shared" fetched once; "only1"/"only2" still pending.
        assert_eq!(a.fetched, 1);
        assert_eq!(a.pending, 2);
        assert_eq!(a.failed, 0);
        assert_eq!(a.caught_up, Some(false));
        assert_eq!(a.epr_id, "head-A");

        let b = acq.per_epr("head-B", &heads).await.expect("head-B present");
        assert_eq!(b.total, Some(1));
        assert_eq!(b.pending, 1);

        // Unknown EPR → None (the handler maps None → 404).
        assert!(acq.per_epr("head-Z", &heads).await.is_none());
    }

    #[tokio::test]
    async fn per_epr_all_shared_fetched_is_caught_up() {
        let acq = AcquisitionState::new();
        let local = HashSet::new();
        acq.reconcile(vec![(1, vec!["x".into()]), (2, vec!["x".into()])], &local)
            .await;
        acq.mark_completed("x").await;
        let mut heads: HashMap<i32, String> = HashMap::new();
        heads.insert(1, "head-A".into());
        heads.insert(2, "head-A".into());
        let a = acq.per_epr("head-A", &heads).await.unwrap();
        assert_eq!(a.total, Some(1)); // single distinct id across both pins
        assert_eq!(a.fetched, 1);
        assert_eq!(a.caught_up, Some(true));
    }

    #[tokio::test]
    async fn per_epr_resolved_empty_is_none_total_not_caught_up() {
        // A pin whose desired set resolves to zero items → tri-state None total
        // / None caughtUp (never false-complete), matching rollup()'s guard.
        let acq = AcquisitionState::new();
        let local = HashSet::new();
        acq.reconcile(vec![(1, vec![])], &local).await;
        let mut heads: HashMap<i32, String> = HashMap::new();
        heads.insert(1, "head-A".into());
        let a = acq.per_epr("head-A", &heads).await.expect("head-A present");
        assert_eq!(a.total, None, "zero-item desired set → None total");
        assert_eq!(a.caught_up, None, "None total must never be caught_up");
        assert_eq!(a.fetched, 0);
    }

    #[tokio::test]
    async fn per_epr_failed_item_is_not_caught_up() {
        // R-A: a failed (unfetched) item leaves fetched < total — the per-EPR
        // rollup must not report caught_up even though pending is transiently 0.
        let acq = AcquisitionState::new();
        let local = HashSet::new();
        acq.reconcile(vec![(1, vec!["need".into()])], &local).await;
        acq.mark_failed("need").await;
        let mut heads: HashMap<i32, String> = HashMap::new();
        heads.insert(1, "head-A".into());
        let a = acq.per_epr("head-A", &heads).await.expect("head-A present");
        assert_eq!(a.total, Some(1));
        assert_eq!(a.fetched, 0);
        assert_eq!(a.failed, 1);
        assert_eq!(
            a.caught_up,
            Some(false),
            "failed item must not be caught_up"
        );
    }
}
