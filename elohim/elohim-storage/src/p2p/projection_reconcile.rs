//! P1 projection reconciliation stream — REA commitments converge from the
//! OWN conductor, with peers used as discovery only.
//!
//! ## Why this exists (the incident it cures)
//!
//! Edge-triggered `ReaProjectionSignal`s are the ONLY thing that lands REA
//! commitments into a peer's local SQL projection. If a peer misses the signal
//! (stale binary at signal time, restart window, gossip race), its projection
//! stays divergent — and a reseed on the originating peer collapses to a 409,
//! so the signal never re-fires. adam's storage stayed divergent for 10 days
//! this way (`.claude/deliver/journal-resilient-dual-doorway.md`, root cause
//! #2). Edge-triggered projection with no reconciliation is a P1 gap.
//!
//! ## The controller, on the shared rails
//!
//! This is a NEW STREAM on `reconcile_rails::GapTracker` — the ONE controller
//! pattern. A parallel bespoke fetcher would be a coherence violation. The
//! tracker is reconstructed per sweep (Category C operational state; the durable
//! truth is the DHT, the projection is the index).
//!
//! ## The design contract (binding, from the p2p-design-gate output)
//!
//! - **Peer SQL is discovery ONLY.** We ask connected peers for their
//!   `(id, dht_anchor_hash)` inventory of REA commitments (the extended
//!   `ViewKind::ProjectionInventory` over `/elohim/view-federation/1.0.0`). For
//!   each id missing locally OR present with a DIFFERENT anchor, we call our OWN
//!   conductor's `content_store::get_rea_commitment(id)`. Row content comes
//!   EXCLUSIVELY from the conductor's DHT notary view. Peer bytes are NEVER
//!   written into the projection.
//! - **Upsert through the shared mapping.** Both the post-commit signal handler
//!   and this reconciler funnel the wire Commitment through
//!   `rea_projection::project_commitment_from_wire` → `upsert_with_anchor`.
//! - **Gap discipline.** Conductor-can't-see-it (`get` returns `None`) →
//!   `mark_failed`, retried on the NEXT sweep (never an immediate re-queue — the
//!   freeze-at-partial battle-scar). Counts are observable on `/p2p/status`.
//! - **v1 scope: `rea_commitments` only.** The table discriminator on
//!   `ProjectionInventory` is the seam for agreements / economic_events; this
//!   sweep asks only for `rea_commitments`.
//!
//! ## The content arm (notary-authority Leg 4)
//!
//! Alongside the REA arm, the `content` arm ([`discover_content`] +
//! [`heal_content`]) runs the SAME pattern for the `content` projection — the
//! cross-peer content-anchor reconcile arm that flips scenario 2: a peer (e.g.
//! `elohim.host`) reaches `trust="notarized"` for content whose DHT anchor exists
//! on an authoring conductor but whose `ContentCommitted` signal it never saw
//! (`post_commit` fires only on the authoring conductor). It shares the cadence
//! (both arms run from [`run_discovery`]/[`run_heal`] on the same
//! `PROJECTION_RECONCILE_SECS` tick) and the shared `GapTracker` rails, but
//! keeps its OWN tracker — the id space is disjoint from REA. Its heal
//! entrypoint is the conductor-VERIFIED [`content_diesel::stamp_declared_head`];
//! the anchor value comes EXCLUSIVELY from the node's own
//! `content_store::resolve_content_head`, never from the peer-advertised pair.
//! Its `divergent_anchor` folds into the shared `/p2p/status` counter (the one
//! cross-arm health signal); its heal/miss detail is log-observable, because
//! extending the ts-rs-exported [`ProjectionReconcileStatus`] with content
//! fields would change the `p2p-status` wire shape (owned elsewhere).

use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::RwLock;
use ts_rs::TS;

use crate::db::DbPool;
use crate::hc_client::HcClient;
use crate::p2p::reconcile_rails::GapTracker;
use crate::p2p::view_federation::{
    PROJECTION_INVENTORY_CAP, PROJECTION_INVENTORY_TABLE_CONTENT,
    PROJECTION_INVENTORY_TABLE_REA_COMMITMENTS,
};
use crate::p2p::P2PHandle;
use crate::services::provide_loop_status::ProvideLoopState;
use crate::views::{ProjectionInventoryPayload, ViewFederationRequest, ViewKind};

/// Per-tick cap for the sweep-driven witness-bootstrap authoring step (GAP 1.5).
/// Keeps conductor load bounded on a large seeded corpus: a ~6k un-witnessed
/// corpus greens over ~30 ticks rather than storming a saturated conductor in a
/// single sweep (the F-T19 evidence — adam's conductor already sits at its
/// read-pool ceiling). Do not raise without weighing conductor saturation.
const WITNESS_MAX_PER_TICK: i64 = 200;

/// Per-item spacing inside a witness sweep (each item is a conductor round-trip).
const WITNESS_ITEM_DELAY: Duration = Duration::from_millis(25);

/// Wall-clock budget for one witness sweep. `HcClient::call_zome` awaits with no
/// timeout of its own, so a hung/stuck conductor call would otherwise hold the
/// heal leg's single-flight guard forever (the RAII `HealFlag` covers panic and
/// cancellation, but not an infinite await). Bounding the whole sweep releases
/// the guard normally on the worst case and resumes next tick (the sweep is
/// idempotent). Derivation: `WITNESS_MAX_PER_TICK` (200) × `WITNESS_ITEM_DELAY`
/// (25ms) = 5s of spacing, plus generous conductor-latency headroom for 200
/// round-trips on a healthy node.
const WITNESS_SWEEP_BUDGET: Duration = Duration::from_secs(120);

/// Per-sweep retry budget for conductor-can't-see-it gaps. A gap that the
/// conductor still can't resolve after this many sweeps drops out (it is almost
/// certainly an id this DHT view legitimately does not carry — a foreign-app or
/// not-yet-gossiped entry). The next sweep that re-discovers it from a peer
/// resets nothing; the failed-count persists for the life of THIS tracker, but
/// the tracker is rebuilt each sweep, so a transient miss self-heals.
const MAX_RETRIES: u32 = 3;

/// Per-peer deadline for a single `ProjectionInventory` federation request.
const PEER_TIMEOUT: Duration = Duration::from_secs(10);

// ── Heal-leg pacing (the saturated-conductor cure) ──────────────────────────
//
// The incident: adam (shem node) sits on a saturated conductor with steady
// ~1/min WS-timeout zome calls (1.5–1.8× matthew's rate). The heal leg is
// single-flight and, pre-fix, UNBOUNDED — it walked every pending row calling
// the conductor with no per-row retry and no per-leg wall-clock bound. On a
// slow conductor one leg could grind for hours (1,956 content rows × a ~60s WS
// timeout each), so the leg never recycled: the next discovery tick found the
// leg still in flight (`SkipInFlight`) and the small REA backlog (62 rows) that
// COULD have landed was starved behind the content queue. Result: rea_local_total
// stuck at 0 for 3h while matthew (healthy conductor) healed the same backlog.
//
// The cure is three-part, all bounded and observable:
//   1. Per-row transient retry — a WS timeout on ONE row gets a bounded in-leg
//      retry (timeout-class errors only) so an intermittently-saturated conductor
//      lands rows it would otherwise drop for the whole sweep.
//   2. Per-leg wall-clock budget — each arm processes rows until its budget
//      elapses, then YIELDS the single-flight guard so the leg recycles every
//      sweep instead of one multi-hour grind. Un-attempted rows are re-discovered
//      next sweep (the tracker is per-sweep), so nothing is lost.
//   3. REA priority — REA heals FIRST with its own reserved budget, so its small
//      backlog is never starved behind the large content queue.

/// Bounded in-leg retry attempts for a transient (timeout-class) conductor error
/// on a single row. 2 retries ⇒ up to 3 attempts. A row that exhausts these stays
/// a `mark_failed` gap (re-discovered next sweep), exactly as before — the retry
/// only adds chances to catch a saturated conductor's free window WITHIN a sweep.
const MAX_ROW_RETRIES: u32 = 2;

/// Per-attempt deadline for one heal conductor call. Deliberately TIGHTER than the
/// conductor client's own ~60s WS timeout: a wedged call is abandoned fast and
/// retried (or the next row attempted) rather than burning ~60s of the leg budget
/// on a single hung row. `HcClient::call_zome` has no timeout of its own.
const HEAL_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(15);

/// In-leg retry backoff floor / span for a transient row (jittered in `[min, min+span)`).
const HEAL_BACKOFF_MIN: Duration = Duration::from_secs(2);
const HEAL_BACKOFF_SPAN: Duration = Duration::from_secs(3); // → 2–5s jittered

/// Per-leg wall-clock budget for the REA arm (small backlog, prioritized — runs
/// first with this reserved slice so it is never starved behind content).
const REA_LEG_BUDGET: Duration = Duration::from_secs(45);

/// Per-leg wall-clock budget for the content arm (the large backlog; recycles each
/// sweep so a saturated conductor lands SOME rows per tick instead of one grind).
const CONTENT_LEG_BUDGET: Duration = Duration::from_secs(120);

/// Injectable pacing for the heal legs (retry + budget). Defaults come from the
/// consts above; tests override with a fast profile (no real sleeps, generous
/// budgets) so the retry/outcome logic is exercised without wall-clock waits.
#[derive(Debug, Clone)]
pub struct HealPacing {
    pub max_row_retries: u32,
    pub attempt_timeout: Duration,
    pub backoff_min: Duration,
    pub backoff_span: Duration,
    pub rea_leg_budget: Duration,
    pub content_leg_budget: Duration,
}

impl Default for HealPacing {
    fn default() -> Self {
        Self {
            max_row_retries: MAX_ROW_RETRIES,
            attempt_timeout: HEAL_ATTEMPT_TIMEOUT,
            backoff_min: HEAL_BACKOFF_MIN,
            backoff_span: HEAL_BACKOFF_SPAN,
            rea_leg_budget: REA_LEG_BUDGET,
            content_leg_budget: CONTENT_LEG_BUDGET,
        }
    }
}

impl HealPacing {
    /// Test profile: no backoff, a generous per-attempt timeout, and budgets large
    /// enough that a small test set never trips the yield (the retry/outcome logic
    /// is what these tests exercise, not the wall-clock bound).
    #[cfg(test)]
    fn test_fast() -> Self {
        Self {
            max_row_retries: 2,
            attempt_timeout: Duration::from_secs(30),
            backoff_min: Duration::ZERO,
            backoff_span: Duration::ZERO,
            rea_leg_budget: Duration::from_secs(3600),
            content_leg_budget: Duration::from_secs(3600),
        }
    }

    /// Jittered backoff in `[backoff_min, backoff_min + backoff_span)`. Dependency-
    /// free jitter from the wall clock's sub-second nanos (spreading concurrent
    /// retries across the fleet is the only property needed — not cryptographic
    /// randomness). A zero span (tests) yields exactly `backoff_min`.
    fn backoff(&self) -> Duration {
        let span_ms = self.backoff_span.as_millis() as u64;
        if span_ms == 0 {
            return self.backoff_min;
        }
        let jitter_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(0)
            % span_ms;
        self.backoff_min + Duration::from_millis(jitter_ms)
    }
}

/// True when a conductor error is a TRANSIENT class worth a bounded in-leg retry —
/// a websocket timeout from a saturated conductor (adam's steady ~1/min WS
/// timeouts), NOT a definitive miss (`Ok(None)`, retried next SWEEP) nor a
/// decode/logic error (never retried, no free-window will fix it). The conductor
/// client preserves the error text verbatim (`Zome call failed: Websocket error:
/// Timeout`), so this is a substring match on that text plus the explicit
/// [`StorageError::Timeout`] variant.
fn is_transient_conductor_error(err: &crate::error::StorageError) -> bool {
    use crate::error::StorageError;
    match err {
        StorageError::Timeout(_) => true,
        StorageError::Conductor(m)
        | StorageError::HolochainClient(m)
        | StorageError::Connection(m) => {
            let m = m.to_ascii_lowercase();
            m.contains("timeout") || m.contains("timed out")
        }
        _ => false,
    }
}

/// The classified result of healing ONE row, for the `/metrics` outcome counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HealOutcomeKind {
    /// Conductor answered and the projection write succeeded on the first attempt.
    Healed,
    /// Conductor answered and the write succeeded, but only after ≥1 transient
    /// retry — the signal that the bounded retry is doing real work on a saturated
    /// conductor (vs `timeout_exhausted`, where it could not recover the row).
    TimeoutRetried,
    /// Every attempt hit a transient (timeout-class) error — row stays a gap,
    /// re-discovered next sweep.
    TimeoutExhausted,
    /// Conductor definitively could not see the row (`Ok(None)`) — retried next
    /// sweep, never immediately.
    Missing,
    /// A non-transient error (decode/logic) or a projection-write failure — retried
    /// next sweep, but no in-leg retry (no free window fixes it).
    Failed,
    /// Content arm, `StampMode::GapFill`: the conductor gave a FALLBACK (non-
    /// canonical) answer and the row already carries a DIFFERENT declared head, so
    /// heal left it to the canonical channels ([`StampOutcome::SkippedDeclared`]).
    /// Not an error and not progress — the heal is REFUSING CORRECTLY.
    RefusedDeclared,
    /// Content arm, `StampMode::HealCanonical`: the conductor's CANONICAL answer is
    /// not provably newer than the adopted head, so the heal kept the adopted head
    /// ([`StampOutcome::SkippedStale`]). Also a correct refusal.
    RefusedStale,
    /// The local row vanished between the presence check and the stamp
    /// ([`StampOutcome::NoRow`]) — resolved, not a conductor miss.
    NoRow,
    /// The stamp wrote, but the declared HEAD did not move: the row already held
    /// the action the own conductor answered with
    /// ([`StampOutcome::Refreshed`]). Real work (patch refresh + ordering
    /// backfill) but NOT convergence — the peer-advertised anchor this row was
    /// enqueued for is still divergent, and will be re-enqueued next sweep.
    ///
    /// A sustained high `refreshed` rate against a non-zero `divergent_anchor` is
    /// the SPIN signature: the two peers hold genuinely different roots, the own
    /// conductor keeps answering with its own, and no amount of healing can
    /// converge them — only a canonical channel can. That diagnosis was
    /// previously indistinguishable from real healing.
    Refreshed,
}

impl HealOutcomeKind {
    fn label(self) -> &'static str {
        match self {
            HealOutcomeKind::Healed => "healed",
            HealOutcomeKind::TimeoutRetried => "timeout_retried",
            HealOutcomeKind::TimeoutExhausted => "timeout_exhausted",
            HealOutcomeKind::Missing => "missing",
            HealOutcomeKind::Failed => "failed",
            HealOutcomeKind::RefusedDeclared => "refused_declared",
            HealOutcomeKind::RefusedStale => "refused_stale",
            HealOutcomeKind::NoRow => "no_row",
            HealOutcomeKind::Refreshed => "refreshed",
        }
    }
}

/// Outcome of the bounded-retry conductor call for one row: the final result plus
/// whether any transient retry was taken (so a success-after-retry is countable).
struct RetryResult<T> {
    result: Result<T, crate::error::StorageError>,
    retried: bool,
}

/// Call the conductor for one row with a per-attempt timeout and bounded transient
/// retry. Non-transient errors and successes return immediately; a transient
/// (timeout-class) error or a per-attempt timeout backs off (jittered) and retries
/// up to `pacing.max_row_retries`. Generic over the call closure so it is unit-
/// testable with a fake op (no conductor).
async fn call_with_retry<T, F, Fut>(pacing: &HealPacing, mut op: F) -> RetryResult<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, crate::error::StorageError>>,
{
    let mut retried = false;
    let mut attempt: u32 = 0;
    loop {
        let outcome = match tokio::time::timeout(pacing.attempt_timeout, op()).await {
            Ok(r) => r,
            Err(_elapsed) => Err(crate::error::StorageError::Timeout(format!(
                "heal conductor call exceeded per-attempt timeout {:?}",
                pacing.attempt_timeout
            ))),
        };
        match outcome {
            Ok(v) => {
                return RetryResult {
                    result: Ok(v),
                    retried,
                }
            }
            Err(e) => {
                // Only timeout-class errors are worth another attempt within the
                // leg; anything else returns immediately (no free window fixes it).
                if attempt < pacing.max_row_retries && is_transient_conductor_error(&e) {
                    attempt += 1;
                    retried = true;
                    tokio::time::sleep(pacing.backoff()).await;
                    continue;
                }
                return RetryResult {
                    result: Err(e),
                    retried,
                };
            }
        }
    }
}

/// Reconciliation progress for the REA-commitment projection stream, exposed
/// via `/p2p/status` (the same surface `replication.rs` uses). Mirrors
/// `ReplicationStatus` and adds reconcile-specific observability.
///
/// Wire format: the `projectionReconcile` property of
/// `elohim/sdk/schemas/v1/views/p2p-status-view.schema.json` (an inline object,
/// mirroring the `pull` precedent). Schema contract test: the
/// `p2p_status_view_*` cases in `tests/schema_contract.rs`.
#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ProjectionReconcileStatus {
    /// Gap ids discovered but not yet healed (in flight this sweep).
    #[ts(type = "number")]
    pub pending: usize,
    /// Gap ids healed from the own conductor (this sweep).
    #[ts(type = "number")]
    pub completed: usize,
    /// Gap ids the own conductor could not see (`get` returned None) — retried
    /// next sweep until MAX_RETRIES.
    #[ts(type = "number")]
    pub failed: usize,
    /// True when this SWEEP ended: every discovered gap was healed **or**
    /// exhausted retries. Deliberately NOT renamed — `/health`, `/p2p/status`
    /// and a live a2o gate consume it. It is not a convergence signal; see
    /// [`ProjectionReconcileStatus::converged`].
    pub caught_up: bool,
    /// Peers asked for an inventory in the last completed sweep.
    #[ts(type = "number")]
    pub peers_asked: usize,
    /// Gaps in the last sweep that were present locally but with a DIFFERENT
    /// anchor than a peer advertised (anchor-divergence, not just absence).
    #[ts(type = "number")]
    pub divergent_anchor: usize,
    /// Cumulative gaps healed across all sweeps this process lifetime.
    #[ts(type = "number")]
    pub healed_total: usize,
    /// Sweeps completed this process lifetime.
    #[ts(type = "number")]
    pub sweeps: usize,
    /// Gaps abandoned at MAX_RETRIES this sweep — healed nothing, retried no
    /// more. `enqueue_missing` refuses to re-queue them, so they leave
    /// `pending` permanently; that is why `caught_up` alone overstates.
    #[ts(type = "number")]
    pub exhausted: usize,
    /// True when this peer holds what its peers advertised: nothing pending,
    /// nothing abandoned, nothing divergent. `caught_up` says only that the
    /// sweep ended — an SLO may be offered over THIS field, not that one.
    pub converged: bool,
}

/// Thread-safe holder for the latest sweep's status snapshot. The `GapTracker`
/// itself is per-sweep (rebuilt each cycle); this carries only the published
/// snapshot the status surface reads, plus the cumulative counters.
#[derive(Debug, Clone, Default)]
pub struct ProjectionReconcileState {
    inner: Arc<RwLock<ProjectionReconcileStatus>>,
}

impl ProjectionReconcileState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot the latest sweep status for `/p2p/status`.
    pub async fn status(&self) -> ProjectionReconcileStatus {
        self.inner.read().await.clone()
    }

    /// Publish the result of a completed sweep, advancing cumulative counters.
    async fn publish_sweep(
        &self,
        counts: crate::p2p::reconcile_rails::GapCounts,
        peers_asked: usize,
        divergent_anchor: usize,
    ) {
        let mut s = self.inner.write().await;
        s.pending = counts.pending;
        s.completed = counts.completed;
        s.failed = counts.failed;
        s.caught_up = counts.caught_up;
        s.peers_asked = peers_asked;
        s.divergent_anchor = divergent_anchor;
        s.healed_total = s.healed_total.saturating_add(counts.completed);
        s.sweeps = s.sweeps.saturating_add(1);
        s.exhausted = counts.exhausted;
        // The gap ledger converging is necessary but not sufficient: rows held
        // locally under an anchor no peer advertises are divergence this sweep
        // did not resolve, so they defeat convergence on their own.
        s.converged = counts.converged && divergent_anchor == 0;
        // Same call site as the wire field on purpose: metric and /p2p/status
        // are written together, so they cannot drift apart the way /health and
        // /p2p/status did (1860 vs 148 in the same minute, 2026-07-25).
        crate::metrics::record_reconcile_sweep(&counts, divergent_anchor);
    }
}

/// Discovery-side output for the REA arm: the gap set (as a per-sweep
/// [`GapTracker`]) plus the observability numbers, carried to the heal leg.
/// Discovery needs no conductor, so it runs every tick even before the lamad
/// bridge lands.
pub struct ReaDiscovery {
    tracker: GapTracker,
    discovered_by: std::collections::HashMap<String, String>,
    peers_asked: usize,
    ids_discovered: usize,
    divergent_anchor: usize,
    local_total: usize,
}

impl ReaDiscovery {
    /// Empty discovery (db unavailable this tick) — the heal leg has nothing to do.
    fn empty() -> Self {
        Self {
            tracker: GapTracker::new(MAX_RETRIES),
            discovered_by: std::collections::HashMap::new(),
            peers_asked: 0,
            ids_discovered: 0,
            divergent_anchor: 0,
            local_total: 0,
        }
    }
}

/// The discovery-side plan both reconcile arms produce, consumed by the heal leg.
pub struct SweepPlan {
    rea: ReaDiscovery,
    content: ContentDiscovery,
}

/// What the per-tick heal scheduler should do, given whether the lamad bridge is
/// up and whether a heal leg is already running. Keeps the single-flight decision
/// pure and unit-testable, off the `main.rs` boot loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealAction {
    /// Bridge up and no heal in flight — spawn the heal leg for this tick's plan.
    Spawn,
    /// A heal leg from an earlier tick is still running — skip (never two
    /// concurrent heal legs). Discovery already ran this tick.
    SkipInFlight,
    /// The lamad bridge has not connected yet — the conductor-dependent heal is
    /// deferred. Discovery already ran this tick.
    SkipNoBridge,
}

/// Decide the per-tick heal action. Bridge-absence takes precedence over the
/// single-flight guard: with no conductor there is nothing to spawn regardless.
pub fn heal_decision(bridge_up: bool, heal_in_flight: bool) -> HealAction {
    if !bridge_up {
        HealAction::SkipNoBridge
    } else if heal_in_flight {
        HealAction::SkipInFlight
    } else {
        HealAction::Spawn
    }
}

/// Ceiling on the peer-reported inventory total the rotating window trusts.
///
/// A buggy or adversarial peer that reports a huge `total` would otherwise pin the
/// window offset ever upward: `requested + page` (bounded by `u32`) can never
/// reach a `u64::MAX`-ish claim, so the wrap-to-0 test never fires — the offset
/// climbs to the `u32::MAX` saturation plateau and every peer is queried past its
/// real corpus forever, silently recreating the whole-table invisibility this
/// cursor exists to close. Clamping `max_total` to this ceiling bounds window
/// progress independent of any single peer's claim, and the wrap still fires at
/// the clamp. Mirrors [`crate::p2p::sync_protocol::MAX_SYNC_LIST_OFFSET`], which
/// bounds a lying always-`has_more` peer on the ListDocuments chain. A genuine
/// corpus larger than this windows its first N rows per rotation (raise
/// deliberately if a real projection ever approaches it).
const MAX_INVENTORY_WINDOW_TOTAL: u64 = 100_000;

/// Rotating per-table window cursor for the `ProjectionInventory` reconcile.
///
/// The responder caps each inventory at [`PROJECTION_INVENTORY_CAP`] rows ordered
/// hot-set-first; a corpus larger than the cap needs successive sweeps to advance
/// a window across the whole table, or its cold tail past the cap stays
/// permanently invisible to this arm (the structural non-convergence the honest
/// `total` exposes and this cursor closes). Lives for the discovery task's
/// lifetime (one per process, owned by the single discovery loop — no locking).
/// Advances by one page each sweep and wraps at the largest peer-reported total
/// (clamped by [`MAX_INVENTORY_WINDOW_TOTAL`]), so a modest corpus that fits one
/// page never leaves offset 0 and no single peer's claim can strand the window.
#[derive(Debug, Default)]
pub struct InventoryWindow {
    /// table → next offset to request on the coming sweep.
    offsets: std::collections::HashMap<String, u32>,
}

impl InventoryWindow {
    pub fn new() -> Self {
        Self::default()
    }

    /// The offset to request for `table` this sweep (0 until advanced).
    fn offset_for(&self, table: &str) -> u32 {
        self.offsets.get(table).copied().unwrap_or(0)
    }

    /// Advance the window for `table` after a sweep that requested `requested` and
    /// saw `max_total` as the largest peer-reported corpus size. Advances by one
    /// page ([`PROJECTION_INVENTORY_CAP`]); wraps to 0 once the window has covered
    /// the whole corpus (or no peer reported a corpus larger than one page — the
    /// common small-corpus case, which then stays pinned at offset 0).
    ///
    /// `max_total` is clamped to [`MAX_INVENTORY_WINDOW_TOTAL`] BEFORE the wrap
    /// test, so a single peer's inflated `total` can never push the offset onto
    /// the `u32` saturation plateau where it would never wrap (see the const).
    fn advance(&mut self, table: &str, requested: u32, max_total: u64) {
        let page = u32::try_from(PROJECTION_INVENTORY_CAP).unwrap_or(u32::MAX);
        let bounded_total = max_total.min(MAX_INVENTORY_WINDOW_TOTAL);
        let next = requested.saturating_add(page);
        let wrapped = if u64::from(next) >= bounded_total {
            0
        } else {
            next
        };
        self.offsets.insert(table.to_string(), wrapped);
    }
}

#[cfg(test)]
mod inventory_window_tests {
    use super::*;

    /// The documented page size the conversion path must yield (Fix 2b review).
    #[test]
    fn page_size_conversion_matches_cap() {
        let page = u32::try_from(PROJECTION_INVENTORY_CAP).unwrap_or(u32::MAX);
        assert_eq!(
            page, 2000,
            "the CAP→u32 conversion yields the documented page"
        );
        let mut w = InventoryWindow::new();
        // A corpus of many pages advances by exactly one page.
        w.advance("content", 0, u64::from(page) * 10);
        assert_eq!(w.offset_for("content"), page);
    }

    #[test]
    fn new_window_starts_at_zero() {
        let w = InventoryWindow::new();
        assert_eq!(w.offset_for("content"), 0);
        assert_eq!(w.offset_for("rea_commitments"), 0);
    }

    #[test]
    fn advances_by_page_then_wraps_at_corpus_end() {
        let mut w = InventoryWindow::new();
        let page = u32::try_from(PROJECTION_INVENTORY_CAP).unwrap();
        let total = u64::from(page) * 2 + 5; // just over two pages
        w.advance("content", 0, total);
        assert_eq!(w.offset_for("content"), page, "0 -> one page");
        w.advance("content", page, total);
        assert_eq!(w.offset_for("content"), page * 2, "page -> two pages");
        // next (3*page) >= total -> wrap to 0 (whole corpus covered).
        w.advance("content", page * 2, total);
        assert_eq!(w.offset_for("content"), 0, "past corpus end -> wrap");
    }

    #[test]
    fn small_corpus_stays_pinned_at_zero() {
        let mut w = InventoryWindow::new();
        // Fits one page: next(=page) >= total -> stays at 0 forever.
        w.advance("content", 0, 10);
        assert_eq!(w.offset_for("content"), 0);
    }

    #[test]
    fn zero_total_peer_never_advances() {
        let mut w = InventoryWindow::new();
        // No peer answered (max_total 0): bounded_total 0, next >= 0 -> wrap to 0.
        w.advance("content", 0, 0);
        assert_eq!(w.offset_for("content"), 0);
        // Even from a non-zero offset, a zero-total sweep resets to 0.
        w.advance("content", 4000, 0);
        assert_eq!(w.offset_for("content"), 0);
    }

    #[test]
    fn adversarial_huge_total_is_clamped_so_window_still_wraps() {
        let mut w = InventoryWindow::new();
        let page = u32::try_from(PROJECTION_INVENTORY_CAP).unwrap();
        let clamp = u32::try_from(MAX_INVENTORY_WINDOW_TOTAL).unwrap();
        // Two pages below the ceiling with a u64::MAX claim: still advances (the
        // clamp does not wrap us early inside the trusted window).
        let below = clamp - page - page;
        w.advance("content", below, u64::MAX);
        assert_eq!(
            w.offset_for("content"),
            below + page,
            "a clamped huge total still advances below the ceiling"
        );
        // At the ceiling boundary the clamp forces the wrap a lying peer's raw
        // total would otherwise prevent (the u32-plateau bug this const fixes).
        w.advance("content", clamp - page, u64::MAX);
        assert_eq!(
            w.offset_for("content"),
            0,
            "clamp forces wrap-to-0 at the ceiling despite a u64::MAX claim"
        );
    }

    #[test]
    fn saturating_add_plateau_self_heals_via_clamp() {
        let mut w = InventoryWindow::new();
        // Simulate an offset already stuck near the u32 plateau (e.g. from a
        // pre-clamp binary). A huge claim now WRAPS instead of pinning, because
        // saturating_add lands at u32::MAX which is >= the clamped total.
        w.advance("content", u32::MAX - 1, u64::MAX);
        assert_eq!(
            w.offset_for("content"),
            0,
            "a plateaued offset self-heals to 0 under the clamp"
        );
    }

    #[test]
    fn separate_tables_have_independent_offsets() {
        let mut w = InventoryWindow::new();
        let page = u32::try_from(PROJECTION_INVENTORY_CAP).unwrap();
        w.advance("content", 0, u64::from(page) * 3);
        assert_eq!(w.offset_for("content"), page);
        assert_eq!(
            w.offset_for("rea_commitments"),
            0,
            "advancing one table must not move another"
        );
    }
}

/// One discovery pass over BOTH reconcile arms (REA + content). Conductor-free:
/// this is the per-tick outbound view-federation ask that must fire from boot,
/// independent of the lamad bridge. Returns the [`SweepPlan`] the heal leg
/// consumes; the heal leg is scheduled separately (single-flight) so a multi-hour
/// heal never starves this cadence.
///
/// `window` carries the rotating per-table inventory offset ACROSS sweeps (owned
/// by the caller's discovery loop), so successive sweeps window across a corpus
/// larger than the responder's page cap rather than re-diffing only the hot set.
pub async fn run_discovery(
    p2p: &P2PHandle,
    pool: &DbPool,
    window: &mut InventoryWindow,
) -> SweepPlan {
    let rea = discover_rea(p2p, pool, window).await;
    let content = discover_content(p2p, pool, window).await;

    tracing::info!(
        target: "elohim_storage::projection_reconcile",
        rea_peers_asked = rea.peers_asked,
        rea_ids_discovered = rea.ids_discovered,
        rea_gaps = rea.tracker.counts().pending,
        rea_divergent_anchor = rea.divergent_anchor,
        rea_local_total = rea.local_total,
        content_peers_asked = content.peers_asked,
        content_ids_discovered = content.ids_discovered,
        content_gaps = content.tracker.counts().pending,
        content_divergent_anchor = content.divergent_anchor,
        content_local_anchored = content.local_anchored,
        "projection-reconcile: discovery complete (heal scheduled separately)"
    );

    SweepPlan { rea, content }
}

/// One heal pass over BOTH arms, consuming a [`SweepPlan`] from [`run_discovery`].
/// Requires the lamad bridge (`hc`); the caller only invokes this once the bridge
/// is up and no other heal is in flight (see [`heal_decision`]). Publishes the
/// sweep status snapshot. Row content comes EXCLUSIVELY from the own conductor;
/// both upsert paths are idempotent, so a heal is safe under duplicate delivery.
///
/// The content arm also runs the sweep-driven [`witness_bootstrap`] step (GAP
/// 1.5): it authors a notarized head for local rows born un-witnessed
/// (bulk-seeded, `dht_anchor_hash` NULL) so they can green. It rides this leg's
/// single-flight guard + OnceLock conductor gate — never running bridge-absent
/// or concurrently — and publishes its progress to `provide_state`.
pub async fn run_heal(
    plan: SweepPlan,
    hc: &Arc<HcClient>,
    pool: &DbPool,
    state: &ProjectionReconcileState,
    provide_state: &ProvideLoopState,
) {
    let SweepPlan { rea, content } = plan;
    // Bounded heal pacing (per-row transient retry + per-leg wall-clock budget).
    // REA runs FIRST with its own reserved budget so its small backlog is never
    // starved behind the large content queue (the incident).
    let pacing = HealPacing::default();
    let ReaDiscovery {
        mut tracker,
        discovered_by,
        peers_asked,
        ids_discovered,
        divergent_anchor: rea_divergent,
        local_total,
    } = rea;
    // Publish the last-sweep gauges BEFORE heal (discovered gaps + local rows), so
    // convergence is watchable on `/metrics` without tailing Loki.
    crate::metrics::set_projection_reconcile_gauges(
        "rea",
        tracker.counts().pending as u64,
        local_total as u64,
        tracker.counts().exhausted as u64,
        rea_divergent as u64,
    );
    let counts = heal_rea(&mut tracker, &discovered_by, hc, pool, &pacing).await;

    let ContentDiscovery {
        tracker: mut content_tracker,
        discovered_by: content_discovered_by,
        divergent_anchor: content_divergent,
        peers_asked: content_peers_asked,
        ids_discovered: content_ids_discovered,
        local_anchored,
    } = content;
    crate::metrics::set_projection_reconcile_gauges(
        "content",
        content_tracker.counts().pending as u64,
        local_anchored as u64,
        content_tracker.counts().exhausted as u64,
        content_divergent as u64,
    );
    let ContentHealOutcome {
        healed: content_healed,
        conductor_missing: content_missing,
        ghost_candidates,
    } = heal_content(
        &mut content_tracker,
        &content_discovered_by,
        hc,
        pool,
        &pacing,
    )
    .await;

    // GAP 1.5: green the un-witnessed seeded corpus. Composed onto (not forked
    // from) the content arm — same conductor gate + single-flight guard.
    witness_bootstrap(hc, pool, provide_state).await;

    // Ghost-anchor witness: the NULL-anchor sweep above cannot see rows whose
    // anchor string outlived its conductor incarnation. Runs on the same leg,
    // fed by the conductor answers the heal already paid for.
    witness_ghost_anchors(hc, pool, &ghost_candidates).await;

    // Publish mirrors the pre-decoupling contract: REA counts + peers_asked, with
    // the divergent-anchor counter folding in BOTH arms (the one cross-arm signal).
    state
        .publish_sweep(counts, peers_asked, rea_divergent + content_divergent)
        .await;

    tracing::info!(
        target: "elohim_storage::projection_reconcile",
        peers_asked,
        ids_discovered,
        healed = counts.completed,
        conductor_missing = counts.failed,
        divergent_anchor = rea_divergent,
        local_total,
        caught_up = counts.caught_up,
        content_peers_asked,
        content_ids_discovered,
        content_healed,
        content_missing,
        content_divergent_anchor = content_divergent,
        content_local_anchored = local_anchored,
        "projection-reconcile: heal complete"
    );
}

/// Witness-bootstrap (GAP 1.5): author a notarized head through the conductor for
/// local content rows born un-witnessed — bulk-seeded diesel-direct rows with
/// `dht_anchor_hash IS NULL` and no conductor record, which can otherwise never
/// reach `trust=green`.
///
/// Composes the proven [`reanchor_backfill::run_once`] mechanism rather than
/// forking a new authoring path (the backlog's "compose, don't fork"):
/// - **Once-per-id guard.** `run_once` authors via `create_content`, which the
///   `content_store` zome REFUSES for a duplicate id; the already-exists branch
///   recovers and stamps the EXISTING anchor instead of minting a second head.
///   So a re-run over an already-witnessed row stamps (not authors), and a
///   transient/bridge error stays a retryable failure — never a fabricated or
///   duplicate head. (The classifier is [`reanchor_backfill::decide_outcome`].)
/// - **Eligibility.** Honors the existing heal/stamp path's reach filter
///   (`CORE_REACH_LEVELS`) — un-widened; non-canonical reach is skipped, not
///   authored.
/// - **Pacing.** Bounded to [`WITNESS_MAX_PER_TICK`] rows per tick with a
///   per-item delay, so a large corpus greens over many ticks. No concurrency.
///
/// Runs only inside [`run_heal`], so the OnceLock conductor gate + single-flight
/// guard already guarantee it never fires bridge-absent or concurrently.
async fn witness_bootstrap(hc: &Arc<HcClient>, pool: &DbPool, provide_state: &ProvideLoopState) {
    // A lamad-scoped ContentService drives the canonical re-anchor path
    // (`update_via_conductor` null-anchor branch). The EventBus is a throwaway:
    // the only event this path emits is `ContentUpdated` (cache invalidation);
    // re-anchoring is a projection write, and content bytes are unchanged, so a
    // dropped invalidation only defers a trust-label refresh to the next read.
    let content_service = crate::services::ContentService::new(
        pool.clone(),
        crate::db::AppContext::default_lamad(),
        Arc::new(crate::services::events::EventBus::new()),
    );
    let cfg = crate::services::reanchor_backfill::ReanchorConfig {
        max_per_sweep: WITNESS_MAX_PER_TICK,
        item_delay: WITNESS_ITEM_DELAY,
    };
    // Wall-clock bound (see WITNESS_SWEEP_BUDGET): on elapse the run_once future
    // is dropped, cancelling any in-flight (possibly hung) conductor call, so the
    // heal leg's single-flight guard always releases. The sweep is idempotent and
    // resumes next tick.
    let sweep = crate::services::reanchor_backfill::run_once(
        pool,
        &content_service,
        hc,
        provide_state,
        &cfg,
    );
    match tokio::time::timeout(WITNESS_SWEEP_BUDGET, sweep).await {
        Ok(Ok(report)) if report.candidates > 0 => {
            crate::metrics::add_content_witness_authored(report.reanchored as u64);
            tracing::info!(
                target: "elohim_storage::projection_reconcile",
                candidates = report.candidates,
                authored = report.reanchored,
                already_witnessed = report.already_anchored,
                skipped = report.skipped,
                failed = report.failed,
                remaining = report.remaining,
                "projection-reconcile[witness]: authored notarized heads for un-witnessed seeded content"
            );
        }
        Ok(Ok(_)) => {
            // No un-witnessed rows — the seeded corpus is fully witnessed.
        }
        Ok(Err(e)) => {
            tracing::warn!(
                target: "elohim_storage::projection_reconcile",
                error = %e,
                "projection-reconcile[witness]: sweep failed (non-fatal, retried next tick)"
            );
        }
        Err(_elapsed) => {
            tracing::warn!(
                target: "elohim_storage::projection_reconcile",
                budget_secs = WITNESS_SWEEP_BUDGET.as_secs(),
                "projection-reconcile[witness]: sweep exceeded wall-clock budget \
                 (likely a slow/saturated conductor) — abandoned, single-flight guard \
                 released, resumes next tick"
            );
        }
    }
}

/// What one content heal leg produced: convergence counts plus the
/// ghost-anchor candidate set the leg's own conductor answers exposed.
struct ContentHealOutcome {
    /// Rows whose head actually MOVED to the own conductor's answer.
    healed: usize,
    /// Gaps the own conductor could not resolve at all (`Ok(None)`).
    conductor_missing: usize,
    /// Ids the own conductor could not resolve — the ghost-anchor candidate
    /// set, narrowed to the truly-anchored rows by [`witness_ghost_anchors`].
    ghost_candidates: Vec<String>,
}

/// Ghost-anchor witness: author a local notarized head for rows whose SQL
/// projection claims a `dht_anchor_hash` that this node's OWN conductor cannot
/// resolve.
///
/// ## The blind spot this closes
///
/// [`witness_bootstrap`] (GAP 1.5) greens rows born un-witnessed — `dht_anchor_
/// hash IS NULL`. But an anchor string can also outlive the conductor
/// incarnation that authored it: a DHT reset / reinstall / re-key wipes
/// conductor state while the SQLite projection persists on its PVC. The row
/// then carries a NON-NULL anchor that resolves to nothing locally — a GHOST.
/// It is invisible to the NULL-anchor sweep, so it can never green, and every
/// canonical-head declaration against it is refused by the zome with
/// `declare_canonical_head: no content found for id '<id>'` (the first gate,
/// `gather_content_chain`, needs a LOCAL `IdToContent` link — and on a full-arc
/// fleet every `get_links` is local-only, so a gossip gap reads as absence).
///
/// ## Why `Ok(None)` from the own conductor is the honest classifier
///
/// `content_store::resolve_content_head` returns `Ok(None)` ONLY when BOTH the
/// cross-root canonical link AND the per-root chain are locally absent — it
/// never fabricates (unlike `get_content_by_id`, which has a v1-healing
/// fallback). Combined with "the row is present AND anchored" (the focused
/// [`anchored_content_reaches_for_ids`] query), that is exactly the ghost class:
///
/// - present + NULL anchor  → [`witness_bootstrap`] already owns it (excluded).
/// - absent                 → the acquisition plane's job; never fabricated.
/// - present + real anchor  → the conductor answers `Some`; never a candidate.
///
/// **Safety property (heal-fills-never-moves).** A row that has genuinely
/// ADOPTED a live canonical head can never reach this path: its conductor holds
/// the canonical link, so `resolve_content_head` answers `Some(canonical)` and
/// the id is healed, not classified as a ghost. The write this path performs is
/// `create_content` through the OWN conductor — the own-authored Declare-class
/// channel `upsert_with_anchor` already implements for `witness_bootstrap`, not
/// a heal stamp. It replaces an unverifiable hash with one this node authored
/// and can prove; a later canonical declaration (always Declare mode) still
/// wins outright.
///
/// ## Bounds
///
/// Reuses [`WITNESS_MAX_PER_TICK`] / [`WITNESS_ITEM_DELAY`] /
/// [`WITNESS_SWEEP_BUDGET`], and the same `CORE_REACH_LEVELS` guard the
/// re-anchor path uses (a non-canonical reach is not re-authorable and would
/// fail every sweep forever). Self-terminating: once authored, the conductor
/// resolves the id and it is never a candidate again.
async fn witness_ghost_anchors(hc: &Arc<HcClient>, pool: &DbPool, candidates: &[String]) {
    if candidates.is_empty() {
        return;
    }
    let app_ctx = crate::db::AppContext::default_lamad();

    // Narrow the conductor-missing set to rows that actually CLAIM an anchor.
    let ghosts: Vec<(String, String)> = {
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile[ghost-witness]: db conn failed; skipping");
                return;
            }
        };
        match crate::db::content_diesel::anchored_content_reaches_for_ids(
            &mut conn, &app_ctx, candidates,
        ) {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile[ghost-witness]: candidate query failed; skipping");
                return;
            }
        }
    };
    if ghosts.is_empty() {
        return;
    }

    let content_service = crate::services::ContentService::new(
        pool.clone(),
        app_ctx,
        Arc::new(crate::services::events::EventBus::new()),
    );

    let total = ghosts.len();
    let sweep = async {
        let mut authored = 0usize;
        let mut skipped = 0usize;
        let mut failed = 0usize;
        for (id, reach) in ghosts.iter().take(WITNESS_MAX_PER_TICK as usize) {
            // Same guard as the re-anchor path: a reach outside the DNA-notarized
            // vocabulary can never be re-authored, so it would burn a conductor
            // round-trip every sweep forever.
            if !crate::generated_enums::CORE_REACH_LEVELS.contains(&reach.as_str()) {
                skipped += 1;
                tracing::warn!(
                    content_id = %id,
                    reach = %reach,
                    "projection-reconcile[ghost-witness]: skipping row with non-canonical reach (not re-authorable)"
                );
                continue;
            }
            // Empty patch on an ANCHORED row takes `update_via_conductor`'s
            // update branch; the zome refuses with "no Content entry found"
            // (there is no local chain), which trips that method's STALE-ANCHOR
            // HEAL: re-publish the full entry from the SQL row via
            // `create_content`. Composed, not forked.
            let empty_patch = crate::views::UpdateContentInputView {
                title: None,
                description: None,
                content_body: None,
                content_format: None,
                metadata: None,
                tags: None,
                reach: None,
                blob_hash: None,
                server_blob_hash: None,
                p2p_published_at: None,
            };
            match content_service
                .update_via_conductor(hc, id, empty_patch)
                .await
            {
                Ok(_) => authored += 1,
                Err(e) => {
                    failed += 1;
                    tracing::warn!(
                        content_id = %id,
                        error = %e,
                        "projection-reconcile[ghost-witness]: re-author failed (non-fatal, retried next sweep)"
                    );
                }
            }
            tokio::time::sleep(WITNESS_ITEM_DELAY).await;
        }
        (authored, skipped, failed)
    };

    match tokio::time::timeout(WITNESS_SWEEP_BUDGET, sweep).await {
        Ok((authored, skipped, failed)) => {
            crate::metrics::add_content_witness_authored(authored as u64);
            tracing::warn!(
                target: "elohim_storage::projection_reconcile",
                candidates = total,
                authored,
                skipped,
                failed,
                "projection-reconcile[ghost-witness]: authored local heads for rows whose claimed \
                 dht_anchor_hash this conductor cannot resolve (stale-anchor class)"
            );
        }
        Err(_elapsed) => {
            tracing::warn!(
                target: "elohim_storage::projection_reconcile",
                budget_secs = WITNESS_SWEEP_BUDGET.as_secs(),
                candidates = total,
                "projection-reconcile[ghost-witness]: sweep exceeded wall-clock budget \
                 (likely a slow/saturated conductor) — abandoned, resumes next sweep"
            );
        }
    }
}

/// Discovery phase of the REA-commitment reconcile (steps 1–3): build the local
/// `(id → anchor)` inventory, ask every connected peer for its
/// `ProjectionInventory { rea_commitments }`, and diff into a per-sweep
/// [`GapTracker`] (an id missing locally, OR present with a different anchor, is a
/// gap). No conductor call happens here — [`heal_rea`] owns that.
async fn discover_rea(
    p2p: &P2PHandle,
    pool: &DbPool,
    window: &mut InventoryWindow,
) -> ReaDiscovery {
    let app_ctx = crate::db::AppContext::default_lamad();
    // Rotating window offset for this sweep (advanced after the peer loop).
    let sweep_offset = window.offset_for(PROJECTION_INVENTORY_TABLE_REA_COMMITMENTS);
    // Largest peer-reported corpus size this sweep — drives the wrap decision.
    let mut max_peer_total: u64 = 0;

    // (1) Local inventory: id → anchor (anchor "" when un-anchored).
    let (local_pairs, local_total) = {
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile: db conn failed; skipping sweep");
                return ReaDiscovery::empty();
            }
        };
        // Offset 0 / i64::MAX: the LOCAL diff needs the WHOLE local set (the
        // rotating window only bounds the PEER ask below).
        match crate::db::rea_commitments::inventory_for_reconcile(&mut conn, &app_ctx, 0, i64::MAX)
        {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile: local inventory failed; skipping sweep");
                return ReaDiscovery::empty();
            }
        }
    };
    let local_anchors: std::collections::HashMap<String, String> =
        local_pairs.iter().cloned().collect();

    // (2) Ask connected peers for their inventory. Collect all peer entries
    // first (one pass), THEN build the tracker — so anchor-divergent ids (present
    // locally but with a different anchor) can be excluded from the tracker's
    // local set and thus admitted as gaps by `discover()`. This keeps ONE
    // tracker on the shared rails without reaching into its internals.
    let peers = p2p.list_peers().await;
    let mut peers_asked = 0usize;
    let mut ids_discovered = 0usize;
    let mut divergent_anchor = 0usize;
    // The union of ids any peer advertised, with the FIRST peer that did so
    // (for the heal WARN log). Anchor-divergent ids are recorded the same way.
    let mut discovered_by: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    // Ids present locally but with a peer-advertised non-empty anchor that
    // disagrees with ours — excluded from the tracker's local set so they heal.
    let mut divergent_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    for peer in &peers {
        let peer_id = match peer.peer_id.parse::<libp2p::PeerId>() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let request = ViewFederationRequest {
            view_kind: ViewKind::ProjectionInventory {
                table: PROJECTION_INVENTORY_TABLE_REA_COMMITMENTS.to_string(),
            },
            // Carries the local agent; the responder ignores ownership for
            // ProjectionInventory (it returns what IT holds, not an agent view).
            agent_cid: p2p.agent_pubkey().to_string(),
            request_id: uuid::Uuid::new_v4().to_string(),
            // Rotating window: this sweep asks for the page at `sweep_offset` so
            // successive sweeps cover a corpus larger than the responder's cap.
            inventory_offset: Some(sweep_offset),
        };
        let resp = match p2p.view_federate(peer_id, request, PEER_TIMEOUT).await {
            Ok(r) => r,
            Err(_) => continue, // peer offline/timeout — discovery is best-effort
        };
        peers_asked += 1;

        let payload: ProjectionInventoryPayload =
            match serde_json::from_value(resp.slice.payload.0.clone()) {
                Ok(p) => p,
                Err(e) => {
                    // WARN, not debug: an undecodable inventory is a protocol
                    // break (version skew), and at debug it is invisible in
                    // Loki — the 2026-06-10 Phase-0 read could not tell
                    // "peers advertise nothing" from "responses don't decode".
                    tracing::warn!(
                        target: "elohim_storage::projection_reconcile",
                        peer = %peer.peer_id,
                        error = %e,
                        "projection-reconcile: peer inventory payload undecodable; skipping peer"
                    );
                    continue;
                }
            };

        // Track the largest corpus any peer reports so the window knows when it
        // has covered the whole table and should wrap. Honesty (Fix 2c): when
        // `total > entries.len()` the peer's inventory is WINDOWED — ids outside
        // this page are unknown-this-sweep, NOT absent. This arm only classifies
        // ADVERTISED ids (see below), so it never concludes absence/in-sync about
        // a non-advertised id; the rotating window guarantees eventual coverage.
        max_peer_total = max_peer_total.max(payload.total as u64);
        // Offset-aware: rows remain BEYOND this page only when
        // offset + served < total. Without the offset term the FINAL page of a
        // rotating window (which reaches the corpus end) would false-log
        // "windowed" merely because served < total.
        let windowed =
            (sweep_offset as usize).saturating_add(payload.entries.len()) < payload.total;

        // Per-peer INFO: makes the discovery leg observable end-to-end
        // (which peers answered, and with how much, and whether truncated).
        tracing::info!(
            target: "elohim_storage::projection_reconcile",
            peer = %peer.peer_id,
            entries = payload.entries.len(),
            peer_total = payload.total,
            offset = sweep_offset,
            windowed = windowed,
            "projection-reconcile: peer inventory received"
        );

        ids_discovered += payload.entries.len();
        for entry in &payload.entries {
            discovered_by
                .entry(entry.id.clone())
                .or_insert_with(|| peer.peer_id.clone());
            // Anchor-divergence: both present, peer carries a non-empty anchor
            // that disagrees with ours. An empty remote anchor is not evidence
            // of divergence (the peer is itself un-anchored).
            if let Some(local_anchor) = local_anchors.get(&entry.id) {
                if !entry.dht_anchor_hash.is_empty()
                    && *local_anchor != entry.dht_anchor_hash
                    && divergent_ids.insert(entry.id.clone())
                {
                    divergent_anchor += 1;
                }
            }
        }
    }

    // Advance the rotating window for the next sweep (wraps at the largest total).
    window.advance(
        PROJECTION_INVENTORY_TABLE_REA_COMMITMENTS,
        sweep_offset,
        max_peer_total,
    );

    // Build the tracker: local set EXCLUDES anchor-divergent ids so `discover()`
    // admits them alongside genuinely-absent ids. All discovered ids flow
    // through the one gap state machine (absence + divergence, unified).
    let tracker_local: std::collections::HashSet<String> = local_anchors
        .keys()
        .filter(|id| !divergent_ids.contains(*id))
        .cloned()
        .collect();
    let mut tracker = GapTracker::new(MAX_RETRIES);
    tracker.set_local_ids(tracker_local);
    let all_discovered: Vec<String> = discovered_by.keys().cloned().collect();
    tracker.discover(all_discovered);

    ReaDiscovery {
        tracker,
        discovered_by,
        peers_asked,
        ids_discovered,
        divergent_anchor,
        local_total,
    }
}

/// Heal phase of the REA-commitment reconcile (step 4): for each discovered gap,
/// read the OWN conductor's `get_rea_commitment(id)` and upsert through the shared
/// mapping (`mark_completed`), or `mark_failed` (retried next sweep) when the
/// conductor can't see it. Runs only once the lamad bridge is up; may span many
/// discovery ticks on a saturated conductor, so it is scheduled single-flight OFF
/// the discovery ticker (see `main.rs`). A heal logs WARN naming the id and the
/// peer that discovered it (a visible mutual-aid event).
async fn heal_rea(
    tracker: &mut GapTracker,
    discovered_by: &std::collections::HashMap<String, String>,
    hc: &Arc<HcClient>,
    pool: &DbPool,
    pacing: &HealPacing,
) -> crate::p2p::reconcile_rails::GapCounts {
    let app_ctx = crate::db::AppContext::default_lamad();

    // (3+4) Heal each gap from the OWN conductor, bounded by the REA leg budget.
    // REA runs FIRST (in `run_heal`) with its own reserved budget, so a small REA
    // backlog is never starved behind the large content queue (the incident).
    let leg_start = std::time::Instant::now();
    let gap_ids = tracker.pending_ids();
    for id in gap_ids {
        let attempt = call_with_retry(pacing, || {
            crate::services::conductor_writes::get_rea_commitment(hc, &id)
        })
        .await;
        let kind = match attempt.result {
            Ok(Some(output)) => match heal_one(&output, pool, &app_ctx) {
                Ok(()) => {
                    tracker.mark_completed(&id);
                    let peer = discovered_by
                        .get(&id)
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string());
                    tracing::warn!(
                        target: "elohim_storage::projection_reconcile",
                        commitment_id = %id,
                        discovered_via_peer = %peer,
                        retried = attempt.retried,
                        "projection-reconcile: HEALED rea_commitment from own conductor (peer discovery)"
                    );
                    if attempt.retried {
                        HealOutcomeKind::TimeoutRetried
                    } else {
                        HealOutcomeKind::Healed
                    }
                }
                Err(e) => {
                    tracing::warn!(commitment_id = %id, error = %e, "projection-reconcile: upsert failed; retry next sweep");
                    tracker.mark_failed(&id);
                    HealOutcomeKind::Failed
                }
            },
            Ok(None) => {
                // Conductor can't see it — retry on NEXT sweep, never immediate.
                tracing::debug!(commitment_id = %id, "projection-reconcile: own conductor returned None; retry next sweep");
                tracker.mark_failed(&id);
                HealOutcomeKind::Missing
            }
            Err(e) => {
                let transient = is_transient_conductor_error(&e);
                tracing::warn!(commitment_id = %id, error = %e, transient, "projection-reconcile: conductor get failed; retry next sweep");
                tracker.mark_failed(&id);
                if transient {
                    HealOutcomeKind::TimeoutExhausted
                } else {
                    HealOutcomeKind::Failed
                }
            }
        };
        crate::metrics::inc_projection_heal_outcome("rea", kind.label());

        // Per-leg budget: yield the single-flight guard so the leg recycles next
        // sweep. Un-attempted rows are re-discovered (the tracker is per-sweep), so
        // nothing is lost — the next leg re-attempts them fresh.
        if leg_start.elapsed() >= pacing.rea_leg_budget {
            tracing::info!(
                target: "elohim_storage::projection_reconcile",
                budget_secs = pacing.rea_leg_budget.as_secs(),
                "projection-reconcile: rea heal hit leg budget — yielding, remaining gaps resume next sweep"
            );
            break;
        }
    }

    tracker.update_caught_up();
    tracker.counts()
}

/// Project one conductor-read Commitment into local SQL via the SHARED mapping.
/// Row content comes exclusively from the own conductor's `ReaCommitmentOutput`.
fn heal_one(
    output: &shefa_types::ReaCommitmentOutput,
    pool: &DbPool,
    app_ctx: &crate::db::AppContext,
) -> Result<(), crate::error::StorageError> {
    let c = &output.commitment;
    let action_hash = format!("{}", output.action_hash);
    let input = crate::rea_projection::project_commitment_from_wire(
        &crate::rea_projection::CommitmentWireFields {
            id: &c.id,
            action: &c.action,
            provider: &c.provider,
            receiver: &c.receiver,
            resource_conforms_to: c.resource_conforms_to.as_deref(),
            // shefa_types::Commitment carries `_json` as non-optional String;
            // an empty string parses to an empty Vec in the shared mapping.
            resource_classified_as_json: Some(c.resource_classified_as_json.as_str()),
            resource_quantity_value: c.resource_quantity_value,
            resource_quantity_unit: c.resource_quantity_unit.as_deref(),
            effort_quantity_value: c.effort_quantity_value,
            effort_quantity_unit: c.effort_quantity_unit.as_deref(),
            has_beginning: c.has_beginning.as_deref(),
            has_end: c.has_end.as_deref(),
            due: c.due.as_deref(),
            clause_of: c.clause_of.as_deref(),
            in_scope_of_json: Some(c.in_scope_of_json.as_str()),
            note: c.note.as_deref(),
            metadata_json: Some(c.metadata_json.as_str()),
        },
    );
    let mut conn = pool
        .get()
        .map_err(|e| crate::error::StorageError::Internal(format!("pool: {e}")))?;
    crate::db::rea_commitments::upsert_with_anchor(&mut conn, app_ctx, input, Some(&action_hash))?;
    Ok(())
}

// ============================================================================
// Content-anchor reconcile arm (notary-authority Leg 4)
// ============================================================================

/// Discovery-side output for the content arm (notary-authority Leg 4): the gap
/// set (as a per-sweep [`GapTracker`]) + observability numbers, carried to the
/// heal leg. Mirrors [`ReaDiscovery`]. Only `divergent_anchor` folds into the
/// shared [`ProjectionReconcileStatus`] (the one cross-arm counter the status
/// surface carries); the rest is log-observable — extending the ts-rs-exported
/// status struct with content fields would change the `p2p-status` wire shape
/// (owned elsewhere).
pub struct ContentDiscovery {
    tracker: GapTracker,
    discovered_by: std::collections::HashMap<String, String>,
    divergent_anchor: usize,
    peers_asked: usize,
    ids_discovered: usize,
    local_anchored: usize,
}

impl ContentDiscovery {
    /// Empty discovery (db unavailable this tick) — the heal leg has nothing to
    /// do. `peers_asked` records how many peers answered before the db failure so
    /// the discovery log stays honest.
    fn empty(peers_asked: usize) -> Self {
        Self {
            tracker: GapTracker::new(MAX_RETRIES),
            discovered_by: std::collections::HashMap::new(),
            divergent_anchor: 0,
            peers_asked,
            ids_discovered: 0,
            local_anchored: 0,
        }
    }
}

/// How ONE advertised content id classifies against the local projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContentGap {
    /// Not present locally — SKIP. Absence is the shard/acquisition plane's job;
    /// the content reconcile NEVER fabricates a row (`stamp_declared_head` is
    /// existing-row-only by construction).
    AbsentLocal,
    /// Present but un-anchored (`dht_anchor_hash` NULL — not in the local
    /// anchored set). Heal: the own conductor stamps the notary anchor. This is
    /// scenario 2 — the bulk-seeded row that never saw its `ContentCommitted`.
    AnchorGap,
    /// Present + anchored, but the local anchor disagrees with a NON-EMPTY peer
    /// anchor. Verify-gap: the own conductor decides who is right (we re-stamp
    /// OUR conductor's head; we never adopt the peer's value). Counts as a
    /// divergence.
    Divergent,
    /// Anchors agree, or the peer advertised no anchor — nothing to do.
    InSync,
}

/// Pure diff for ONE advertised content id (the notary invariant, Leg 4).
///
/// `present` is reach-agnostic local presence (`content_ids_present`);
/// `local_anchors` is the local anchored + distribution-safe set
/// (`list_content_anchor_inventory`); `peer_anchor` is the anchor a peer
/// advertised for the id (`None`/empty ⇒ the peer is itself un-anchored, which
/// is never divergence evidence).
fn classify_content_gap(
    id: &str,
    present: &std::collections::HashSet<String>,
    local_anchors: &std::collections::HashMap<String, String>,
    peer_anchor: Option<&str>,
) -> ContentGap {
    if !present.contains(id) {
        return ContentGap::AbsentLocal;
    }
    match local_anchors.get(id) {
        None => ContentGap::AnchorGap,
        Some(local) => match peer_anchor {
            Some(pa) if !pa.is_empty() && pa != local.as_str() => ContentGap::Divergent,
            _ => ContentGap::InSync,
        },
    }
}

/// Discovery phase of the `content` reconcile (Leg 4, steps 1–4). No conductor
/// call happens here — [`heal_content`] owns step 5.
///
/// 1. Build the local anchored+distribution-safe inventory (`id → anchor`).
/// 2. Ask every connected peer for its `ProjectionInventory { content }`.
/// 3. One `content_ids_present` query resolves reach-agnostic local presence for
///    every advertised id.
/// 4. Diff each advertised `(id, peer_anchor)` via [`classify_content_gap`]:
///    absent → SKIP; un-anchored → anchor-gap; anchor-divergent → verify-gap +
///    divergence count. Anchor-gap ∪ divergent ids feed a per-sweep
///    [`GapTracker`] on the shared rails.
///
/// **Re-detect semantics.** The tracker is rebuilt each sweep, so its per-sweep
/// `MAX_RETRIES` never permanently drops a gap: a divergence or anchor-gap that
/// persists in SQL is recomputed from the inventory diff on the NEXT sweep and
/// re-enqueued afresh. `MAX_RETRIES` only bounds within-sweep churn (and the heal
/// leg attempts each gap once per sweep, so it is effectively a floor).
async fn discover_content(
    p2p: &P2PHandle,
    pool: &DbPool,
    window: &mut InventoryWindow,
) -> ContentDiscovery {
    let app_ctx = crate::db::AppContext::default_lamad();
    // Rotating window offset for this sweep (advanced after the peer loop).
    let sweep_offset = window.offset_for(PROJECTION_INVENTORY_TABLE_CONTENT);
    let mut max_peer_total: u64 = 0;

    // (1) Local anchored inventory: id → anchor. Only anchored + distribution-
    // safe rows (the same set this node advertises). Absent / un-anchored rows
    // are resolved via presence below. Offset 0 / i64::MAX: the LOCAL diff needs
    // the WHOLE local set (the rotating window only bounds the PEER ask).
    let local_anchors: std::collections::HashMap<String, String> = {
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile[content]: db conn failed; skipping sweep");
                return ContentDiscovery::empty(0);
            }
        };
        match crate::db::content_diesel::list_content_anchor_inventory(
            &mut conn,
            &app_ctx,
            0,
            i64::MAX,
        ) {
            Ok((rows, _total)) => rows.into_iter().collect(),
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile[content]: local inventory failed; skipping sweep");
                return ContentDiscovery::empty(0);
            }
        }
    };
    let local_anchored = local_anchors.len();

    // (2) Ask connected peers for their content inventory. Collect all entries
    // first, then diff once presence is known.
    let peers = p2p.list_peers().await;
    let mut peers_asked = 0usize;
    let mut ids_discovered = 0usize;
    // id → first peer that advertised it (for the heal WARN).
    let mut discovered_by: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    // id → first NON-EMPTY advertised anchor (for divergence diffing).
    let mut advertised_anchor: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for peer in &peers {
        let peer_id = match peer.peer_id.parse::<libp2p::PeerId>() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let request = ViewFederationRequest {
            view_kind: ViewKind::ProjectionInventory {
                table: PROJECTION_INVENTORY_TABLE_CONTENT.to_string(),
            },
            agent_cid: p2p.agent_pubkey().to_string(),
            request_id: uuid::Uuid::new_v4().to_string(),
            // Rotating window: this sweep asks for the page at `sweep_offset`.
            inventory_offset: Some(sweep_offset),
        };
        let resp = match p2p.view_federate(peer_id, request, PEER_TIMEOUT).await {
            Ok(r) => r,
            Err(_) => continue, // peer offline/timeout — discovery is best-effort
        };
        peers_asked += 1;

        let payload: ProjectionInventoryPayload = match serde_json::from_value(
            resp.slice.payload.0.clone(),
        ) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(
                    target: "elohim_storage::projection_reconcile",
                    peer = %peer.peer_id,
                    error = %e,
                    "projection-reconcile[content]: peer inventory payload undecodable; skipping peer"
                );
                continue;
            }
        };

        // Honest total (Fix 2a) drives the window and truncation observability:
        // when `total > entries.len()` the peer inventory is WINDOWED — ids
        // outside this page are unknown-this-sweep, not absent. This arm classifies
        // only ADVERTISED ids (classify_content_gap below), so it never concludes
        // absence/in-sync about a non-advertised id; the window covers the rest.
        max_peer_total = max_peer_total.max(payload.total as u64);
        // Offset-aware (see discover_rea): rows remain beyond this page only when
        // offset + served < total, so the final page of a window is not
        // mislabelled "windowed".
        let windowed =
            (sweep_offset as usize).saturating_add(payload.entries.len()) < payload.total;

        tracing::info!(
            target: "elohim_storage::projection_reconcile",
            peer = %peer.peer_id,
            entries = payload.entries.len(),
            peer_total = payload.total,
            offset = sweep_offset,
            windowed = windowed,
            "projection-reconcile[content]: peer inventory received"
        );

        ids_discovered += payload.entries.len();
        for entry in &payload.entries {
            discovered_by
                .entry(entry.id.clone())
                .or_insert_with(|| peer.peer_id.clone());
            if !entry.dht_anchor_hash.is_empty() {
                advertised_anchor
                    .entry(entry.id.clone())
                    .or_insert_with(|| entry.dht_anchor_hash.clone());
            }
        }
    }

    // Advance the rotating window for the next sweep (wraps at the largest total).
    window.advance(
        PROJECTION_INVENTORY_TABLE_CONTENT,
        sweep_offset,
        max_peer_total,
    );

    // (3) One presence query for the whole advertised union (reach-agnostic).
    let advertised_ids: Vec<String> = discovered_by.keys().cloned().collect();
    let present: std::collections::HashSet<String> = {
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile[content]: db conn failed for presence; skipping heal");
                return ContentDiscovery::empty(peers_asked);
            }
        };
        // Chunk under SQLite's bound-variable limit (SQLITE_MAX_VARIABLE_NUMBER,
        // ~999 on older builds) — `content_ids_present`'s own doc requires callers
        // to chunk large id sets (content_diesel.rs:996). A >limit federation
        // inventory would otherwise error the WHOLE presence query and silently
        // drop the sweep into a no-heal tick. Merge per-chunk results.
        const PRESENCE_CHUNK: usize = 500;
        let mut acc = std::collections::HashSet::new();
        for chunk in advertised_ids.chunks(PRESENCE_CHUNK) {
            match crate::db::content_diesel::content_ids_present(&mut conn, &app_ctx, chunk) {
                Ok(s) => acc.extend(s),
                Err(e) => {
                    tracing::warn!(error = %e, "projection-reconcile[content]: presence query failed; skipping heal");
                    return ContentDiscovery::empty(peers_asked);
                }
            }
        }
        acc
    };

    // (4) Classify → gap set (anchor-gap ∪ divergent). Absent + in-sync are dropped.
    let mut gap_ids: Vec<String> = Vec::new();
    let mut divergent_anchor = 0usize;
    for id in &advertised_ids {
        match classify_content_gap(
            id,
            &present,
            &local_anchors,
            advertised_anchor.get(id).map(String::as_str),
        ) {
            ContentGap::AbsentLocal | ContentGap::InSync => {}
            ContentGap::AnchorGap => gap_ids.push(id.clone()),
            ContentGap::Divergent => {
                divergent_anchor += 1;
                gap_ids.push(id.clone());
            }
        }
    }

    // Feed the gap set through a fresh per-sweep tracker on the shared rails
    // (empty local set → every gap id becomes pending, under MAX_RETRIES).
    let mut tracker = GapTracker::new(MAX_RETRIES);
    tracker.discover(gap_ids);

    ContentDiscovery {
        tracker,
        discovered_by,
        divergent_anchor,
        peers_asked,
        ids_discovered,
        local_anchored,
    }
}

/// Heal phase of the `content` reconcile (Leg 4, step 5): for each discovered
/// gap, `content_store::resolve_content_head(id)` on the OWN conductor →
/// [`stamp_declared_head`] (verified stamp); `None` → `mark_failed`, retried next
/// sweep. Returns a [`ContentHealOutcome`] — the convergence counts plus the
/// ids the own conductor could not resolve at all, which
/// [`witness_ghost_anchors`] narrows to the ghost-anchor class. Runs only once the lamad bridge
/// is up; scheduled single-flight OFF the discovery ticker (see `main.rs`).
/// `stamp_declared_head` is existing-row-only and idempotent, so a heal is safe
/// under duplicate delivery. A heal logs WARN naming the id and discovering peer.
async fn heal_content(
    tracker: &mut GapTracker,
    discovered_by: &std::collections::HashMap<String, String>,
    hc: &Arc<HcClient>,
    pool: &DbPool,
    pacing: &HealPacing,
) -> ContentHealOutcome {
    let app_ctx = crate::db::AppContext::default_lamad();
    let mut ghost_candidates: Vec<String> = Vec::new();

    // (5) Heal each gap from the OWN conductor (verified stamp), bounded by the
    // content leg budget so a saturated conductor lands SOME rows per tick and the
    // leg recycles instead of grinding for hours (the incident). Runs AFTER `heal_rea`
    // so the small REA backlog is never starved behind this large content queue.
    let leg_start = std::time::Instant::now();
    let mut healed = 0usize;
    let mut conductor_missing = 0usize;
    for id in tracker.pending_ids() {
        let attempt = call_with_retry(pacing, || {
            crate::services::conductor_writes::call_resolve_content_head(hc, &id)
        })
        .await;
        match attempt.result {
            Ok(Some(head)) => match heal_content_one(&head, pool, &app_ctx) {
                Ok(crate::db::content_diesel::StampOutcome::Stamped) => {
                    tracker.mark_completed(&id);
                    healed += 1;
                    let peer = discovered_by
                        .get(&id)
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string());
                    tracing::warn!(
                        target: "elohim_storage::projection_reconcile",
                        content_id = %id,
                        discovered_via_peer = %peer,
                        retried = attempt.retried,
                        "projection-reconcile[content]: HEALED content anchor from own conductor (peer discovery)"
                    );
                    crate::metrics::inc_projection_heal_outcome(
                        "content",
                        if attempt.retried {
                            HealOutcomeKind::TimeoutRetried.label()
                        } else {
                            HealOutcomeKind::Healed.label()
                        },
                    );
                }
                Ok(crate::db::content_diesel::StampOutcome::Refreshed) => {
                    // The own conductor answered with the head this row ALREADY
                    // holds: the write refreshed value fields and backfilled the
                    // declaration ordering, but the HEAD did not move — so the
                    // peer-advertised divergence that enqueued this row is
                    // UNCHANGED and will re-enqueue next sweep. Deliberately NOT
                    // counted in `healed` and NOT logged as HEALED: conflating
                    // this no-op with convergence is what made a spinning heal
                    // plane read as a working one (see `StampOutcome::Refreshed`).
                    tracker.mark_completed(&id);
                    tracing::info!(
                        target: "elohim_storage::projection_reconcile",
                        content_id = %id,
                        "projection-reconcile[content]: head unchanged — refreshed row, divergence NOT resolved (own conductor answered the head this row already holds)"
                    );
                    crate::metrics::inc_projection_heal_outcome(
                        "content",
                        HealOutcomeKind::Refreshed.label(),
                    );
                }
                Ok(crate::db::content_diesel::StampOutcome::SkippedDeclared) => {
                    // Row already carries a DIFFERENT declared head — the heal
                    // must not move it (a fresh-boot conductor resolve can fall
                    // through to the root-author election and would resurrect a
                    // superseded head over an adopted canonical one — the
                    // 2026-07-11 20:42:40 regression). Canonical channels own it.
                    tracker.mark_completed(&id);
                    tracing::info!(content_id = %id, "projection-reconcile[content]: row already declared — heal left it to the canonical channels");
                    crate::metrics::inc_projection_heal_outcome(
                        "content",
                        HealOutcomeKind::RefusedDeclared.label(),
                    );
                }
                Ok(crate::db::content_diesel::StampOutcome::SkippedStale) => {
                    // The conductor's CANONICAL answer is not provably newer
                    // than the declared head this row already adopted — a
                    // conductor that has not yet integrated the newer canonical
                    // link answers with the OLD canonical record, and stamping
                    // it would move the head BACKWARDS (the 2026-07-12
                    // regression: converged at edge #1187's seam-smoke, healed
                    // back to the superseded head by #1188). Completed, not
                    // failed: retrying yields the same stale answer until the
                    // conductor integrates the newer link, at which point the
                    // next sweep's answer becomes provably newer and stamps.
                    tracker.mark_completed(&id);
                    tracing::info!(content_id = %id, "projection-reconcile[content]: conductor canonical answer not provably newer — heal kept the adopted head");
                    crate::metrics::inc_projection_heal_outcome(
                        "content",
                        HealOutcomeKind::RefusedStale.label(),
                    );
                }
                Ok(crate::db::content_diesel::StampOutcome::NoRow) => {
                    // Row vanished between presence check and stamp (rare race).
                    // Nothing to stamp — resolved, not a conductor miss.
                    tracker.mark_completed(&id);
                    tracing::debug!(content_id = %id, "projection-reconcile[content]: stamp found no local row; nothing to do");
                    crate::metrics::inc_projection_heal_outcome(
                        "content",
                        HealOutcomeKind::NoRow.label(),
                    );
                }
                Err(e) => {
                    tracing::warn!(content_id = %id, error = %e, "projection-reconcile[content]: stamp failed; retry next sweep");
                    tracker.mark_failed(&id);
                    crate::metrics::inc_projection_heal_outcome(
                        "content",
                        HealOutcomeKind::Failed.label(),
                    );
                } // SkippedDeclared / SkippedStale / NoRow are benign resolutions
                  // (row already correct or absent): `mark_completed`, no stamp, and
                  // deliberately NOT counted as `healed` so the cure signal (real
                  // stamps) is not inflated by no-op resolutions.
            },
            Ok(None) => {
                // Conductor can't see it yet (catch-up) — retry on the NEXT
                // sweep via a fresh inventory diff, never an immediate re-queue.
                conductor_missing += 1;
                // GHOST-ANCHOR CANDIDATE (see `witness_ghost_anchors`). This is
                // the ONLY place the substrate learns "my own conductor has no
                // chain for an id my projection carries" — `resolve_content_head`
                // returns `Ok(None)` exactly when BOTH the canonical link and the
                // per-root chain are locally absent. Collect it here rather than
                // re-probing later: the answer is already paid for.
                ghost_candidates.push(id.clone());
                tracing::debug!(content_id = %id, "projection-reconcile[content]: own conductor returned None; retry next sweep");
                tracker.mark_failed(&id);
                crate::metrics::inc_projection_heal_outcome(
                    "content",
                    HealOutcomeKind::Missing.label(),
                );
            }
            Err(e) => {
                let transient = is_transient_conductor_error(&e);
                tracing::warn!(content_id = %id, error = %e, transient, "projection-reconcile[content]: conductor resolve failed; retry next sweep");
                tracker.mark_failed(&id);
                crate::metrics::inc_projection_heal_outcome(
                    "content",
                    if transient {
                        HealOutcomeKind::TimeoutExhausted.label()
                    } else {
                        HealOutcomeKind::Failed.label()
                    },
                );
            }
        }

        // Per-leg budget: yield so the leg recycles next sweep (see `heal_rea`).
        if leg_start.elapsed() >= pacing.content_leg_budget {
            tracing::info!(
                target: "elohim_storage::projection_reconcile",
                budget_secs = pacing.content_leg_budget.as_secs(),
                healed,
                "projection-reconcile[content]: heal hit leg budget — yielding, remaining gaps resume next sweep"
            );
            break;
        }
    }

    ContentHealOutcome {
        healed,
        conductor_missing,
        ghost_candidates,
    }
}

/// Project ONE conductor-resolved content HEAD into local SQL via the VERIFIED
/// stamp path. Row content comes exclusively from the own conductor's resolved
/// [`ContentHeadWire`]; the field mapping mirrors the `ContentCommitted` signal
/// arm (`rea_projection.rs`). Returns `stamp_declared_head`'s bool (false ⇒ no
/// local row to stamp).
fn heal_content_one(
    head: &crate::services::conductor_writes::ContentHeadWire,
    pool: &DbPool,
    app_ctx: &crate::db::AppContext,
) -> Result<crate::db::content_diesel::StampOutcome, crate::error::StorageError> {
    let c = &head.content;
    // u64 → i32 saturating cast — identical to the ContentCommitted arm.
    let size_i32 = c
        .content_size_bytes
        .map(|n| i32::try_from(n).unwrap_or(i32::MAX));
    let patch = crate::db::content_diesel::ContentProjectionPatch {
        blob_cid: c.blob_cid.clone(),
        content_size_bytes: size_i32,
        title: Some(c.title.clone()),
        description: Some(c.description.clone()),
        content_type: Some(c.content_type.clone()),
        content_format: Some(c.content_format.clone()),
        reach: Some(c.reach.clone()),
        metadata_json: Some(c.metadata_json.clone()),
    };
    let mut conn = pool
        .get()
        .map_err(|e| crate::error::StorageError::Internal(format!("pool: {e}")))?;
    // Canonical-aware stamp mode: a CANONICAL answer (the conductor verified
    // the cross-root canonical record) may fill an undeclared row, refresh the
    // same head, or MOVE a declared row FORWARD (provably newer declared_at) —
    // this is exactly how a peer converges when the canonical link gossips in
    // between deploys. It must NOT move a declared row otherwise: a conductor
    // that has not yet integrated a newer canonical link answers with the OLD
    // canonical record — canonical, yet stale — and an unconditional Declare
    // stamp moved the head BACKWARDS (live regression 2026-07-12,
    // elohim-host-landing). A FALLBACK answer (cold conductor, root-author
    // election) may only FILL an undeclared row — never resurrect a
    // superseded head over an adopted canonical one.
    let mode = if head.canonical {
        crate::db::content_diesel::StampMode::HealCanonical
    } else {
        crate::db::content_diesel::StampMode::GapFill
    };
    crate::db::content_diesel::stamp_declared_head_mode(
        &mut conn,
        app_ctx,
        &c.id,
        head.head_action_hash.as_str(),
        Some(head.declared_at),
        Some(patch),
        mode,
    )
}

// ---------------------------------------------------------------------------
// Shard-location catch-up arm (Category C — custody convergence, cold path)
// ---------------------------------------------------------------------------

/// Catch-up reconcile for the `shard_locations` custody projection: fetch each
/// connected peer's custody inventory over `/elohim/view-federation/1.0.0` and
/// project every advertised claim as a `peer-announced` row via the SAME
/// never-overwrite-local rule the gossip hot path uses.
///
/// ## Why this differs from the REA / content arms
///
/// Custody is Category C: there is NO DHT notary for who-holds-what. The
/// announcing peer's own projection IS the heal source, so — unlike
/// [`discover_rea`] / [`discover_content`], which advertise only `(id, anchor)`
/// and heal row content from the OWN conductor — a custody inventory entry
/// carries the FULL [`CustodyAnnouncement`] and is applied directly. No conductor
/// is involved.
///
/// ## DORMANT until the responder serves the table
///
/// This is deliberately NOT wired into [`run_discovery`]/[`run_heal`] yet. The
/// view-federation responder (`p2p::view_federation::build_inventory_payload`)
/// returns an honest empty inventory for any table it does not know, and
/// `shard_locations` is not yet a known table there. Wiring this into the sweep
/// before the responder serves it would ship a guaranteed no-op that burns a
/// federation round-trip per peer every tick. Activate it once the responder
/// serves `table = "shard_locations"` with a `CustodyInventoryPayload` (see the
/// slice report for the exact responder + `run_discovery` insertions). Kept
/// `pub` so it compiles clean (no dead-code) as landed-but-dormant.
pub async fn reconcile_shard_locations_from_peers(p2p: &P2PHandle, pool: &DbPool) {
    use crate::p2p::custody_announce::{
        CustodyInventoryPayload, PROJECTION_INVENTORY_TABLE_SHARD_LOCATIONS,
    };

    // Resolve THIS node's own agent_cid once for the self-drop guard.
    let self_agent_cid = pool
        .get()
        .ok()
        .and_then(|mut conn| crate::reconcile::custody::resolve_self_agent_cid(&mut conn, None));

    let peers = p2p.list_peers().await;
    let mut applied = 0usize;
    let mut dropped_weaker = 0usize;
    let mut dropped_self = 0usize;
    let mut peers_asked = 0usize;

    for peer in &peers {
        let peer_id = match peer.peer_id.parse::<libp2p::PeerId>() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let request = ViewFederationRequest {
            view_kind: ViewKind::ProjectionInventory {
                table: PROJECTION_INVENTORY_TABLE_SHARD_LOCATIONS.to_string(),
            },
            agent_cid: p2p.agent_pubkey().to_string(),
            request_id: uuid::Uuid::new_v4().to_string(),
            inventory_offset: None,
        };
        let resp = match p2p.view_federate(peer_id, request, PEER_TIMEOUT).await {
            Ok(r) => r,
            Err(_) => continue, // peer offline/timeout — catch-up is best-effort
        };
        peers_asked += 1;

        let payload: CustodyInventoryPayload =
            match serde_json::from_value(resp.slice.payload.0.clone()) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(
                        target: "elohim_storage::custody",
                        peer = %peer.peer_id,
                        error = %e,
                        "custody catch-up: peer inventory payload undecodable; skipping peer"
                    );
                    continue;
                }
            };

        let Ok(mut conn) = pool.get() else { continue };
        let stats = crate::db::shard_locations::apply_custody_inventory(
            &mut conn,
            &payload.entries,
            self_agent_cid.as_deref(),
        );
        applied += stats.applied();
        dropped_weaker += stats.dropped_weaker;
        dropped_self += stats.dropped_self;
    }

    for _ in 0..applied {
        crate::metrics::inc_custody_announce("applied");
    }
    for _ in 0..dropped_weaker {
        crate::metrics::inc_custody_announce("dropped_weaker");
    }
    for _ in 0..dropped_self {
        crate::metrics::inc_custody_announce("dropped_self");
    }

    tracing::info!(
        target: "elohim_storage::custody",
        peers_asked,
        applied,
        dropped_weaker,
        dropped_self,
        "custody catch-up: shard-location reconcile complete"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::reconcile_rails::GapCounts;

    #[tokio::test]
    async fn state_publishes_and_accumulates_across_sweeps() {
        let state = ProjectionReconcileState::new();
        // Initial: nothing.
        let s0 = state.status().await;
        assert_eq!((s0.healed_total, s0.sweeps), (0, 0));

        // Sweep 1: healed 2, failed 1.
        state
            .publish_sweep(
                GapCounts {
                    pending: 0,
                    completed: 2,
                    failed: 1,
                    caught_up: true,
                    // The failed gap spent its budget: the sweep ended
                    // (caught_up) without healing it, so it did NOT converge.
                    exhausted: 1,
                    converged: false,
                },
                3,
                1,
            )
            .await;
        let s1 = state.status().await;
        assert_eq!(s1.completed, 2);
        assert_eq!(s1.failed, 1);
        assert_eq!(s1.peers_asked, 3);
        assert_eq!(s1.divergent_anchor, 1);
        assert_eq!(s1.healed_total, 2);
        assert_eq!(s1.sweeps, 1);
        assert!(s1.caught_up);

        // Sweep 2: healed 1 more — cumulative healed_total advances.
        state
            .publish_sweep(
                GapCounts {
                    pending: 0,
                    completed: 1,
                    failed: 0,
                    caught_up: true,
                    exhausted: 0,
                    converged: true,
                },
                2,
                0,
            )
            .await;
        let s2 = state.status().await;
        assert_eq!(s2.healed_total, 3);
        assert_eq!(s2.sweeps, 2);
        assert_eq!(s2.completed, 1);
        assert_eq!(s2.divergent_anchor, 0);
    }

    #[tokio::test]
    async fn a_sweep_with_divergent_anchors_is_not_converged() {
        // The live beta shape: every gap healed and the retry budget untouched,
        // but rows sit locally under an anchor no peer advertises. `caught_up`
        // says the sweep ended; it does NOT say this peer holds what its peers
        // hold. divergent_anchor alone must defeat convergence.
        let state = ProjectionReconcileState::new();
        state
            .publish_sweep(
                GapCounts {
                    pending: 0,
                    completed: 4,
                    failed: 0,
                    caught_up: true,
                    exhausted: 0,
                    converged: true, // the GAP LEDGER converged...
                },
                3,
                1860, // ...but 1860 rows diverge, so the PEER did not.
            )
            .await;

        let s = state.status().await;
        assert!(s.caught_up, "the sweep did finish");
        assert_eq!(s.divergent_anchor, 1860);
        assert_eq!(s.exhausted, 0);
        assert!(
            !s.converged,
            "divergent anchors mean this peer does NOT hold what its peers hold"
        );
    }

    #[tokio::test]
    async fn a_sweep_that_abandoned_every_gap_is_caught_up_but_not_converged() {
        // The live beta shape from the other end: healedTotal stays 0 across
        // sweeps while gaps exhaust their retry budget. caught_up flips true;
        // converged must not.
        let state = ProjectionReconcileState::new();
        state
            .publish_sweep(
                GapCounts {
                    pending: 0,
                    completed: 0,
                    failed: 61,
                    caught_up: true,
                    exhausted: 61,
                    converged: false,
                },
                3,
                0,
            )
            .await;

        let s = state.status().await;
        assert!(s.caught_up);
        assert_eq!(s.exhausted, 61);
        assert_eq!(
            s.healed_total, 0,
            "22 sweeps, healedTotal 0 — the live shape"
        );
        assert!(!s.converged);
    }

    #[test]
    fn status_serializes_converged_and_exhausted_as_camel_case() {
        // Wire contract: p2p-status-view.schema.json sets
        // additionalProperties:false on projectionReconcile, so these names
        // must match the schema exactly or the contract test rejects them.
        let s = ProjectionReconcileStatus {
            exhausted: 61,
            converged: false,
            ..Default::default()
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["exhausted"], serde_json::json!(61));
        assert_eq!(v["converged"], serde_json::json!(false));
    }

    #[test]
    fn heal_decision_covers_bridge_and_single_flight() {
        // Bridge up, nothing running → spawn the heal leg this tick.
        assert_eq!(heal_decision(true, false), HealAction::Spawn);
        // Bridge up, a heal already in flight → skip (single-flight; discovery ran).
        assert_eq!(heal_decision(true, true), HealAction::SkipInFlight);
        // Bridge down → defer heal regardless of the in-flight flag (nothing to
        // spawn without a conductor); discovery still ran.
        assert_eq!(heal_decision(false, false), HealAction::SkipNoBridge);
        assert_eq!(heal_decision(false, true), HealAction::SkipNoBridge);
    }

    #[test]
    fn ghost_witness_classifier_is_conductor_miss_intersect_anchored() {
        // The ghost class is the INTERSECTION of two independently-owned facts,
        // and neither alone is sufficient:
        //   - the own conductor could not resolve the id (`Ok(None)`), and
        //   - the local row CLAIMS a dht_anchor_hash.
        // This test locks the classifier's shape at the seam where the two meet
        // (the SQL half is proved by
        // `anchored_content_reaches_for_ids_selects_only_the_ghost_candidate_class`).
        // Only `Ok(None)` may enter the candidate list: a resolvable id — whether
        // canonical or a root-author fallback — is healed, never re-authored, so a
        // row that has genuinely adopted a canonical head can never be re-rooted
        // underneath it.
        let classify = |resolved: Option<bool>| resolved.is_none();
        assert!(classify(None), "conductor miss is the only candidate arm");
        assert!(
            !classify(Some(true)),
            "a CANONICAL answer heals — it must never be re-authored"
        );
        assert!(
            !classify(Some(false)),
            "a FALLBACK answer still proves a local chain exists — not a ghost"
        );
    }

    #[test]
    fn ghost_witness_reuses_the_witness_bounds() {
        // The ghost sweep runs on the SAME heal leg as `witness_bootstrap`, so it
        // must not introduce a second, unbounded conductor load: it reuses the
        // per-tick cap, the per-item spacing, and the wall-clock budget. Locking
        // this keeps a future tuning change from bounding one sweep and not the
        // other.
        assert!(WITNESS_MAX_PER_TICK > 0);
        assert!(!WITNESS_ITEM_DELAY.is_zero());
        assert!(WITNESS_SWEEP_BUDGET > WITNESS_ITEM_DELAY * (WITNESS_MAX_PER_TICK as u32));
    }

    #[test]
    fn witness_per_tick_cap_is_bounded_for_pacing() {
        // (d) per-tick cap: bounded so a large un-witnessed corpus (live alpha
        // shows thousands of rows) greens over many ticks instead of storming a
        // saturated conductor in one sweep.
        assert!(WITNESS_MAX_PER_TICK > 0, "must author some per tick");
        assert!(
            WITNESS_MAX_PER_TICK <= 500,
            "must stay small enough to pace a multi-thousand-row corpus across ticks"
        );
    }

    #[test]
    fn witness_sweep_budget_exceeds_paced_floor() {
        // The wall-clock budget must exceed the unavoidable per-item spacing floor
        // (cap × delay) with headroom for the conductor round-trips, so a HEALTHY
        // sweep never trips the timeout — the budget only fires on a hung/saturated
        // conductor, releasing the single-flight guard instead of holding it forever.
        let paced_floor = WITNESS_ITEM_DELAY * (WITNESS_MAX_PER_TICK as u32);
        assert!(
            WITNESS_SWEEP_BUDGET > paced_floor,
            "budget {WITNESS_SWEEP_BUDGET:?} must exceed the paced floor {paced_floor:?}"
        );
        // And it must be a real bound (not effectively infinite).
        assert!(WITNESS_SWEEP_BUDGET <= Duration::from_secs(600));
    }

    #[test]
    fn witness_guard_is_the_reanchor_once_per_id_classifier() {
        // The witness step's once-per-id guard IS `reanchor_backfill::decide_outcome`
        // (composed, not forked). Assert the three cases the task calls out, so the
        // guarantee is legible at the composition site.
        use crate::error::StorageError;
        use crate::services::reanchor_backfill::{
            decide_outcome, is_already_anchored_error, RowOutcome,
        };

        // (a) A candidate whose head the conductor already holds: create_content is
        // refused ("already exists") and the existing anchor is recovered+stamped →
        // AlreadyAnchored (stamped, NOT a second authored head).
        let already: Result<(), StorageError> = Err(StorageError::Conductor(
            "Zome call failed: Guest(\"Content with id 'seed-1' already exists. \
             Use update_content to modify existing entries.\")"
                .to_string(),
        ));
        assert!(is_already_anchored_error(already.as_ref().unwrap_err()));
        assert_eq!(
            decide_outcome(&already, Some(&Ok(true))),
            RowOutcome::AlreadyAnchored
        );

        // (b) Definitive not-found → authored exactly once (Reanchored). On the
        // NEXT tick the conductor holds it, so create is refused → AlreadyAnchored
        // (authored zero the second time — idempotent).
        let authored: Result<(), StorageError> = Ok(());
        assert_eq!(decide_outcome(&authored, None), RowOutcome::Reanchored);
        assert_eq!(
            decide_outcome(&already, Some(&Ok(true))),
            RowOutcome::AlreadyAnchored
        );

        // (c) Transient/bridge error → Failed (skipped, retried next tick; never a
        // fabricated or duplicate head).
        let transient: Result<(), StorageError> =
            Err(StorageError::Conductor("read plane down".into()));
        assert!(!is_already_anchored_error(transient.as_ref().unwrap_err()));
        assert_eq!(decide_outcome(&transient, None), RowOutcome::Failed);
    }

    #[test]
    fn content_gap_classification_absent_null_divergent() {
        use std::collections::{HashMap, HashSet};

        // present: b (un-anchored), c (anchored=X), d (anchored=X). a is absent.
        let present: HashSet<String> = ["b", "c", "d"].iter().map(|s| s.to_string()).collect();
        // local anchored set (list_content_anchor_inventory): only c and d.
        let mut local_anchors: HashMap<String, String> = HashMap::new();
        local_anchors.insert("c".into(), "anchor-X".into());
        local_anchors.insert("d".into(), "anchor-X".into());

        // (a) advertised but absent locally → SKIP.
        assert_eq!(
            classify_content_gap("a", &present, &local_anchors, Some("anchor-Z")),
            ContentGap::AbsentLocal
        );
        // (b) present but un-anchored (not in local_anchors) → anchor-gap.
        assert_eq!(
            classify_content_gap("b", &present, &local_anchors, Some("anchor-Y")),
            ContentGap::AnchorGap
        );
        // (c) present + anchored, peer anchor disagrees → divergent.
        assert_eq!(
            classify_content_gap("c", &present, &local_anchors, Some("anchor-Y")),
            ContentGap::Divergent
        );
        // (d) present + anchored, peer anchor agrees → in sync.
        assert_eq!(
            classify_content_gap("d", &present, &local_anchors, Some("anchor-X")),
            ContentGap::InSync
        );
        // (c) present + anchored, peer advertised EMPTY anchor → NOT divergence
        // (an un-anchored peer is not evidence our anchor is wrong).
        assert_eq!(
            classify_content_gap("c", &present, &local_anchors, Some("")),
            ContentGap::InSync
        );
        // (c) present + anchored, peer advertised NO anchor → in sync.
        assert_eq!(
            classify_content_gap("c", &present, &local_anchors, None),
            ContentGap::InSync
        );
    }

    // ── Heal-leg pacing: the saturated-conductor cure (retry + budget) ──

    #[test]
    fn transient_classifier_splits_timeout_from_logic() {
        use crate::error::StorageError;
        // The exact verbatim conductor text adam's Loki shows is transient.
        assert!(is_transient_conductor_error(&StorageError::Conductor(
            "Zome call failed: Websocket error: Timeout".into()
        )));
        assert!(is_transient_conductor_error(&StorageError::Timeout(
            "per-attempt".into()
        )));
        assert!(is_transient_conductor_error(&StorageError::Connection(
            "read timed out".into()
        )));
        // A logic/decode error is NOT transient — no free window fixes it, so no
        // in-leg retry (it would just burn budget).
        assert!(!is_transient_conductor_error(&StorageError::Conductor(
            "Guest(\"not the author\")".into()
        )));
        assert!(!is_transient_conductor_error(&StorageError::Validation(
            "bad enum".into()
        )));
    }

    #[test]
    fn heal_pacing_prioritizes_rea_and_bounds_attempts() {
        let p = HealPacing::default();
        assert!(
            p.max_row_retries >= 1,
            "a transient row must get at least one in-leg retry"
        );
        assert!(
            p.attempt_timeout < Duration::from_secs(60),
            "per-attempt timeout must be tighter than the conductor's ~60s WS timeout"
        );
        assert!(
            p.rea_leg_budget <= p.content_leg_budget,
            "rea's reserved budget must not exceed content's (rea is prioritized, small backlog)"
        );
        assert!(p.rea_leg_budget > Duration::ZERO && p.content_leg_budget > Duration::ZERO);
        // Jittered backoff stays within [min, min+span).
        let b = p.backoff();
        assert!(b >= p.backoff_min && b < p.backoff_min + p.backoff_span);
    }

    #[test]
    fn heal_outcome_labels_are_stable() {
        // The `/metrics` label vocabulary is a wire contract — pin it.
        assert_eq!(HealOutcomeKind::Healed.label(), "healed");
        assert_eq!(HealOutcomeKind::TimeoutRetried.label(), "timeout_retried");
        assert_eq!(
            HealOutcomeKind::TimeoutExhausted.label(),
            "timeout_exhausted"
        );
        assert_eq!(HealOutcomeKind::Missing.label(), "missing");
        assert_eq!(HealOutcomeKind::Failed.label(), "failed");
    }

    #[tokio::test]
    async fn call_with_retry_recovers_after_one_transient() {
        use crate::error::StorageError;
        let pacing = HealPacing::test_fast();
        let calls = std::cell::Cell::new(0u32);
        let r = call_with_retry(&pacing, || {
            let n = calls.get();
            calls.set(n + 1);
            async move {
                if n == 0 {
                    // First attempt: a WS timeout (the adam signature).
                    Err::<i32, StorageError>(StorageError::Conductor(
                        "Websocket error: Timeout".into(),
                    ))
                } else {
                    Ok(7)
                }
            }
        })
        .await;
        assert_eq!(r.result.expect("recovered"), 7);
        assert!(r.retried, "success came after a transient retry");
        assert_eq!(calls.get(), 2, "one retry after the first transient");
    }

    #[tokio::test]
    async fn call_with_retry_exhausts_on_persistent_transient() {
        use crate::error::StorageError;
        let pacing = HealPacing::test_fast(); // max_row_retries = 2 → 3 attempts
        let calls = std::cell::Cell::new(0u32);
        let r = call_with_retry(&pacing, || {
            calls.set(calls.get() + 1);
            async { Err::<i32, StorageError>(StorageError::Timeout("wedged".into())) }
        })
        .await;
        assert!(
            r.result.is_err(),
            "a persistently-wedged row stays a failure"
        );
        assert!(r.retried);
        assert_eq!(calls.get(), 3, "1 initial + max_row_retries(2) attempts");
    }

    #[tokio::test]
    async fn call_with_retry_no_retry_on_non_transient() {
        use crate::error::StorageError;
        let pacing = HealPacing::test_fast();
        let calls = std::cell::Cell::new(0u32);
        let r = call_with_retry(&pacing, || {
            calls.set(calls.get() + 1);
            async { Err::<i32, StorageError>(StorageError::Validation("bad".into())) }
        })
        .await;
        assert!(r.result.is_err());
        assert!(!r.retried, "a non-transient error is never retried in-leg");
        assert_eq!(calls.get(), 1, "exactly one attempt for a logic error");
    }
}
