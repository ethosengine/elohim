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

/// FLOOR on the per-item retry budget, not the budget itself.
///
/// `drain_acquisition_queue` rotates each retry to a different connected peer,
/// so the budget is really "how many providers do we probe before calling an
/// item unsatisfiable". Three retries on a 6-peer fabric probes at most half of
/// it — and then RETIRES the pin as if the bytes were proven unavailable, which
/// is a false negative, not a bounded one. The live budget is
/// [`retry_budget_for_peers`]: `max(MIN_RETRIES, connected peers)`. Retirement
/// must mean actually-unsatisfiable.
const MIN_RETRIES: u32 = 3;

/// The per-item retry budget for a fabric of `connected_peers` peers: enough
/// attempts that the rotation can reach EVERY connected provider at least once,
/// never fewer than [`MIN_RETRIES`].
pub fn retry_budget_for_peers(connected_peers: usize) -> u32 {
    MIN_RETRIES.max(u32::try_from(connected_peers).unwrap_or(u32::MAX))
}

/// Device-paced: deliberately below replication's 50 (R-E — the acquisition
/// stream serves a person's wants, not node policy; it must not starve the
/// node-level stream).
pub const MAX_ACQUISITION_INFLIGHT: usize = 25;

/// Pin status for a pin whose every wanted item exhausted its retry budget
/// against every connected provider.
///
/// NOT a new column: `acquisition_pins.status` is TEXT and
/// [`crate::views::PinView`]`::status` is a `String`, so this value is
/// additive on the wire. It DOES need a migration, though: `status` carries a
/// CHECK constraint enumerating the admitted values, and the original
/// migration (`2026-06-07-000000_acquisition_pins/up.sql`) did not include
/// `'retired'` — writing it failed with a CheckViolation at the SQLite engine
/// until `2026-07-31-200000_acquisition_pins_admit_retired` widened the CHECK.
/// `updated_at` (already stamped by `set_pin_status`) is the retirement clock.
///
/// Distinct from `removed`, which is a PERSON's revocation. `retired` is the
/// SUBSTRATE saying "no peer can supply these bytes right now" — a pin the
/// person still wants, held back so it stops pinning `pull.caughtUp` false
/// forever (alpha-A: 73 total / 3 fetched / 70 failed, caughtUp false for 12h+).
/// It is re-admitted the moment a peer advertises the content again, or after
/// [`PIN_READMIT_COOLDOWN`].
pub const PIN_STATUS_RETIRED: &str = "retired";

/// Backstop re-admission for a retired pin when no inventory ever names it
/// again. The primary path is inventory-driven (a peer advertising the head_ref
/// re-activates it immediately); this only bounds the worst case where the pin's
/// content simply is not being advertised, so the substrate re-checks a few
/// times a day instead of never.
pub const PIN_READMIT_COOLDOWN: chrono::Duration = chrono::Duration::hours(6);

/// Has a retired pin's hold lasted longer than [`PIN_READMIT_COOLDOWN`]?
///
/// `updated_at` is the retirement clock — `set_pin_status` stamps it, which is
/// why retirement needs no migration and no new column. The format is
/// `current_timestamp()`'s RFC 3339 (`%Y-%m-%dT%H:%M:%SZ`).
///
/// An UNPARSEABLE timestamp returns `true` — fail toward re-admission. A pin is
/// something a person asked for; a redundant retry costs one round of fetches,
/// whereas treating a malformed clock as "not yet cooled" would strand the pin
/// permanently, which is the exact failure this whole leg exists to remove.
pub fn pin_cooled_down(updated_at: &str, now: chrono::DateTime<chrono::Utc>) -> bool {
    match chrono::DateTime::parse_from_rfc3339(updated_at) {
        Ok(t) => now.signed_duration_since(t.with_timezone(&chrono::Utc)) > PIN_READMIT_COOLDOWN,
        Err(_) => true,
    }
}

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
    /// `max_retries` is the live per-item budget (see [`retry_budget_for_peers`])
    /// — applied to every tracker each cycle, so a fabric that grows re-admits
    /// items that were exhausted against a smaller one.
    pub async fn reconcile(
        &self,
        pin_wants: Vec<(i32, Vec<String>)>,
        local_has: &std::collections::HashSet<String>,
        max_retries: u32,
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
                .or_insert_with(|| GapTracker::new(max_retries));
            // Re-apply every cycle: the fabric's peer count moves, and a tracker
            // that outlives the cycle must not keep probing against a stale
            // (smaller) budget.
            tracker.set_max_retries(max_retries);
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

    /// Pins whose desired set is UNSATISFIABLE against the current fabric: every
    /// remaining want spent its retry budget (`exhausted > 0`), nothing is still
    /// in flight (`pending == 0`), and the pin never completed (`completed <
    /// total`).
    ///
    /// The caller flips these to [`PIN_STATUS_RETIRED`]. All three conditions are
    /// load-bearing:
    /// - `exhausted > 0` — a budget was actually SPENT. With
    ///   [`retry_budget_for_peers`] that means every connected provider was
    ///   asked, so this is a measured absence, not an unprobed one.
    /// - `pending == 0` — nothing is still being fetched; retiring an in-flight
    ///   pin would abandon bytes that are arriving.
    /// - `completed < total` — a satisfied pin is caught-up, not exhausted.
    ///   (`total == 0`, the resolved-empty case, is excluded for the same reason
    ///   `rollup` refuses to call it caught_up: nothing was ever asked.)
    ///
    /// No eviction code is needed on the tracker side: once the pin leaves
    /// `list_active_pins`, the `trackers.retain` at the top of [`Self::reconcile`]
    /// drops it on the next cycle, which is what removes it from BOTH `total` and
    /// `fetched` in [`Self::rollup`].
    pub async fn exhausted_pin_ids(&self) -> Vec<i32> {
        let inner = self.inner.read().await;
        let mut out: Vec<i32> = inner
            .trackers
            .iter()
            .filter(|(pin_id, t)| {
                let c = t.counts();
                let total = *inner.totals.get(pin_id).unwrap_or(&0);
                total > 0 && c.exhausted > 0 && c.pending == 0 && c.completed < total
            })
            .map(|(pin_id, _)| *pin_id)
            .collect();
        out.sort_unstable();
        out
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
                MIN_RETRIES,
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
            .reconcile(vec![(1, vec!["have-1".into()])], &local, MIN_RETRIES)
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
            MIN_RETRIES,
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
        acq.reconcile(vec![(1, vec![])], &local, MIN_RETRIES).await;
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
        acq.reconcile(vec![(1, vec!["need".into()])], &local, MIN_RETRIES)
            .await;
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
        acq.reconcile(
            vec![(1, vec!["a".into()]), (2, vec!["b".into()])],
            &local,
            MIN_RETRIES,
        )
        .await;
        acq.reconcile(vec![(1, vec!["a".into()])], &local, MIN_RETRIES)
            .await;
        let pins = acq.per_pin().await;
        assert_eq!(pins.len(), 1);
        assert_eq!(pins[0].pin_id, 1);
        assert!(!acq.wants("b").await);
    }

    /// Spend a pin's whole retry budget the way the live loop does: fail, then
    /// re-reconcile (retry-on-NEXT-cycle, R-E) until `enqueue_missing` refuses.
    async fn exhaust(acq: &AcquisitionState, pin_wants: Vec<(i32, Vec<String>)>, budget: u32) {
        let local = HashSet::new();
        for _ in 0..budget {
            acq.reconcile(pin_wants.clone(), &local, budget).await;
            for (_, ids) in &pin_wants {
                for id in ids {
                    acq.mark_failed(id).await;
                }
            }
        }
        acq.reconcile(pin_wants, &local, budget).await;
    }

    #[tokio::test]
    async fn exhausted_pin_retires_and_rollup_reaches_caught_up() {
        // The live alpha-A shape: 73 total / 3 fetched / 70 failed, and
        // `pull.caughtUp` false for 12h+. Nothing in the runtime ever flipped a
        // pin's status (the only status-flip caller was the HTTP DELETE route),
        // so an unsatisfiable want held the queue open forever.
        let acq = AcquisitionState::new();
        let local: HashSet<String> = ["have-1".to_string()].into_iter().collect();

        // pin 1 is satisfiable (already local); pin 2 is not.
        acq.reconcile(vec![(1, vec!["have-1".into()])], &local, MIN_RETRIES)
            .await;
        for _ in 0..MIN_RETRIES {
            acq.reconcile(
                vec![(1, vec!["have-1".into()]), (2, vec!["gone".into()])],
                &local,
                MIN_RETRIES,
            )
            .await;
            acq.mark_failed("gone").await;
        }
        acq.reconcile(
            vec![(1, vec!["have-1".into()]), (2, vec!["gone".into()])],
            &local,
            MIN_RETRIES,
        )
        .await;

        let r = acq.rollup().await;
        assert!(
            !r.caught_up,
            "before retirement the unsatisfiable want holds the queue open"
        );

        assert_eq!(
            acq.exhausted_pin_ids().await,
            vec![2],
            "only the unsatisfiable pin is exhausted — the satisfied one is caught-up"
        );

        // The caller flips pin 2 to `retired`, so it leaves `list_active_pins`
        // and the NEXT reconcile's `trackers.retain` drops it — from BOTH total
        // and fetched. No separate eviction code exists, or is needed.
        acq.reconcile(vec![(1, vec!["have-1".into()])], &local, MIN_RETRIES)
            .await;
        let r = acq.rollup().await;
        assert_eq!((r.total, r.fetched), (1, 1));
        assert!(
            r.caught_up,
            "with the unsatisfiable pin retired the pull queue can finally drain"
        );
    }

    #[tokio::test]
    async fn retired_pin_readmitted_on_new_inventory() {
        // Retirement is a HOLD, not a delete. When the pin comes back as active
        // (a peer advertised its head_ref again), the tracker is rebuilt and the
        // item re-enters the pull queue — caught_up correctly goes false again,
        // because the bytes still have not arrived.
        let acq = AcquisitionState::new();
        let local = HashSet::new();
        exhaust(&acq, vec![(1, vec!["want".into()])], MIN_RETRIES).await;
        assert_eq!(acq.exhausted_pin_ids().await, vec![1]);

        // Retired: absent from `list_active_pins` → tracker dropped, rollup empty.
        acq.reconcile(vec![], &local, MIN_RETRIES).await;
        assert!(acq.exhausted_pin_ids().await.is_empty());

        // Re-admitted: back in `list_active_pins` with a CLEAN budget.
        let dispatch = acq
            .reconcile(vec![(1, vec!["want".into()])], &local, MIN_RETRIES)
            .await;
        assert_eq!(
            dispatch,
            vec!["want".to_string()],
            "a re-admitted pin re-enters the pull queue"
        );
        let r = acq.rollup().await;
        assert_eq!((r.total, r.fetched, r.pending), (1, 0, 1));
        assert!(
            !r.caught_up,
            "re-admission must not false-complete — the bytes still have not arrived (R-A)"
        );
    }

    #[tokio::test]
    async fn all_pins_retired_is_not_caught_up() {
        // The failure mode retirement could EASILY introduce: retire every pin,
        // the tracker map empties, `total` goes 0 — and a naive `pending == 0`
        // rollup would call that caught-up. `rollup`'s `s.total > 0` guard is
        // what stops "we gave up on everything" from reading as "we have
        // everything"; this test is that guard's regression fence.
        let acq = AcquisitionState::new();
        let local = HashSet::new();
        exhaust(
            &acq,
            vec![(1, vec!["a".into()]), (2, vec!["b".into()])],
            MIN_RETRIES,
        )
        .await;
        assert_eq!(acq.exhausted_pin_ids().await, vec![1, 2]);

        acq.reconcile(vec![], &local, MIN_RETRIES).await;
        let r = acq.rollup().await;
        assert_eq!((r.total, r.fetched), (0, 0));
        assert!(
            !r.caught_up,
            "every pin retired is NOT caught up — an empty desired set never false-completes"
        );
        assert!(acq.per_pin().await.is_empty());
    }

    #[test]
    fn cooldown_readmission_uses_updated_at_and_never_strands_a_pin() {
        // `updated_at` is the retirement clock — this is why Leg B needs no
        // migration. A pin retired 1h ago is still held; one retired 7h ago is
        // re-admitted.
        let now = chrono::Utc::now();
        let fresh = (now - chrono::Duration::hours(1))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let stale = (now - chrono::Duration::hours(7))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        assert!(!pin_cooled_down(&fresh, now), "still within the hold");
        assert!(pin_cooled_down(&stale, now), "cooled down — re-admit");

        // A malformed clock must never strand a pin forever: fail TOWARD
        // re-admission. Permanent silent hold is the failure this leg removes.
        assert!(pin_cooled_down("not-a-timestamp", now));
        assert!(
            pin_cooled_down("2026-07-31 12:00:00", now),
            "the SQL-default format (no T/Z) is unparseable here — re-admit, never strand"
        );
    }

    #[test]
    fn retry_budget_follows_the_fabric_never_below_the_floor() {
        // A 6-peer fabric must probe 6 providers before calling an item
        // unsatisfiable — retirement is only honest if every peer was asked.
        assert_eq!(retry_budget_for_peers(6), 6);
        assert_eq!(retry_budget_for_peers(25), 25);
        // Small/empty fabrics keep the floor (a 1-peer fabric still deserves
        // retries against transient failures).
        assert_eq!(retry_budget_for_peers(0), MIN_RETRIES);
        assert_eq!(retry_budget_for_peers(1), MIN_RETRIES);
        assert_eq!(retry_budget_for_peers(3), MIN_RETRIES);
    }

    #[tokio::test]
    async fn an_in_flight_pin_is_never_retired() {
        // `pending == 0` is load-bearing: retiring a pin whose bytes are still
        // arriving would abandon a fetch in progress.
        let acq = AcquisitionState::new();
        let local = HashSet::new();
        acq.reconcile(vec![(1, vec!["a".into(), "b".into()])], &local, MIN_RETRIES)
            .await;
        // "a" exhausts; "b" stays pending.
        for _ in 0..MIN_RETRIES {
            acq.mark_failed("a").await;
            acq.reconcile(vec![(1, vec!["a".into(), "b".into()])], &local, MIN_RETRIES)
                .await;
        }
        assert!(
            acq.exhausted_pin_ids().await.is_empty(),
            "a pin with an item still in flight must not retire"
        );
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
            MIN_RETRIES,
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
        acq.reconcile(
            vec![(1, vec!["x".into()]), (2, vec!["x".into()])],
            &local,
            MIN_RETRIES,
        )
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
        acq.reconcile(vec![(1, vec![])], &local, MIN_RETRIES).await;
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
        acq.reconcile(vec![(1, vec!["need".into()])], &local, MIN_RETRIES)
            .await;
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
