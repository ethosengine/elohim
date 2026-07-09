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
    PROJECTION_INVENTORY_TABLE_CONTENT, PROJECTION_INVENTORY_TABLE_REA_COMMITMENTS,
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
    /// True when every discovered gap was healed or exhausted retries.
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

/// One discovery pass over BOTH reconcile arms (REA + content). Conductor-free:
/// this is the per-tick outbound view-federation ask that must fire from boot,
/// independent of the lamad bridge. Returns the [`SweepPlan`] the heal leg
/// consumes; the heal leg is scheduled separately (single-flight) so a multi-hour
/// heal never starves this cadence.
pub async fn run_discovery(p2p: &P2PHandle, pool: &DbPool) -> SweepPlan {
    let rea = discover_rea(p2p, pool).await;
    let content = discover_content(p2p, pool).await;

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
    let ReaDiscovery {
        mut tracker,
        discovered_by,
        peers_asked,
        ids_discovered,
        divergent_anchor: rea_divergent,
        local_total,
    } = rea;
    let counts = heal_rea(&mut tracker, &discovered_by, hc, pool).await;

    let ContentDiscovery {
        tracker: mut content_tracker,
        discovered_by: content_discovered_by,
        divergent_anchor: content_divergent,
        peers_asked: content_peers_asked,
        ids_discovered: content_ids_discovered,
        local_anchored,
    } = content;
    let (content_healed, content_missing) =
        heal_content(&mut content_tracker, &content_discovered_by, hc, pool).await;

    // GAP 1.5: green the un-witnessed seeded corpus. Composed onto (not forked
    // from) the content arm — same conductor gate + single-flight guard.
    witness_bootstrap(hc, pool, provide_state).await;

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

/// Discovery phase of the REA-commitment reconcile (steps 1–3): build the local
/// `(id → anchor)` inventory, ask every connected peer for its
/// `ProjectionInventory { rea_commitments }`, and diff into a per-sweep
/// [`GapTracker`] (an id missing locally, OR present with a different anchor, is a
/// gap). No conductor call happens here — [`heal_rea`] owns that.
async fn discover_rea(p2p: &P2PHandle, pool: &DbPool) -> ReaDiscovery {
    let app_ctx = crate::db::AppContext::default_lamad();

    // (1) Local inventory: id → anchor (anchor "" when un-anchored).
    let (local_pairs, local_total) = {
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile: db conn failed; skipping sweep");
                return ReaDiscovery::empty();
            }
        };
        match crate::db::rea_commitments::inventory_for_reconcile(&mut conn, &app_ctx, i64::MAX) {
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

        // Per-peer INFO: makes the discovery leg observable end-to-end
        // (which peers answered, and with how much).
        tracing::info!(
            target: "elohim_storage::projection_reconcile",
            peer = %peer.peer_id,
            entries = payload.entries.len(),
            peer_total = payload.total,
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
) -> crate::p2p::reconcile_rails::GapCounts {
    let app_ctx = crate::db::AppContext::default_lamad();

    // (3+4) Heal each gap from the OWN conductor.
    let gap_ids = tracker.pending_ids();
    for id in gap_ids {
        match crate::services::conductor_writes::get_rea_commitment(hc, &id).await {
            Ok(Some(output)) => {
                let healed = heal_one(&output, pool, &app_ctx);
                match healed {
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
                            "projection-reconcile: HEALED rea_commitment from own conductor (peer discovery)"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(commitment_id = %id, error = %e, "projection-reconcile: upsert failed; retry next sweep");
                        tracker.mark_failed(&id);
                    }
                }
            }
            Ok(None) => {
                // Conductor can't see it — retry on NEXT sweep, never immediate.
                tracing::debug!(commitment_id = %id, "projection-reconcile: own conductor returned None; retry next sweep");
                tracker.mark_failed(&id);
            }
            Err(e) => {
                tracing::warn!(commitment_id = %id, error = %e, "projection-reconcile: conductor get failed; retry next sweep");
                tracker.mark_failed(&id);
            }
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
async fn discover_content(p2p: &P2PHandle, pool: &DbPool) -> ContentDiscovery {
    let app_ctx = crate::db::AppContext::default_lamad();

    // (1) Local anchored inventory: id → anchor. Only anchored + distribution-
    // safe rows (the same set this node advertises). Absent / un-anchored rows
    // are resolved via presence below.
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
            i64::MAX,
        ) {
            Ok(v) => v.into_iter().collect(),
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

        tracing::info!(
            target: "elohim_storage::projection_reconcile",
            peer = %peer.peer_id,
            entries = payload.entries.len(),
            peer_total = payload.total,
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
/// sweep. Returns `(healed, conductor_missing)`. Runs only once the lamad bridge
/// is up; scheduled single-flight OFF the discovery ticker (see `main.rs`).
/// `stamp_declared_head` is existing-row-only and idempotent, so a heal is safe
/// under duplicate delivery. A heal logs WARN naming the id and discovering peer.
async fn heal_content(
    tracker: &mut GapTracker,
    discovered_by: &std::collections::HashMap<String, String>,
    hc: &Arc<HcClient>,
    pool: &DbPool,
) -> (usize, usize) {
    let app_ctx = crate::db::AppContext::default_lamad();

    // (5) Heal each gap from the OWN conductor (verified stamp).
    let mut healed = 0usize;
    let mut conductor_missing = 0usize;
    for id in tracker.pending_ids() {
        match crate::services::conductor_writes::call_resolve_content_head(hc, &id).await {
            Ok(Some(head)) => match heal_content_one(&head, pool, &app_ctx) {
                Ok(true) => {
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
                        "projection-reconcile[content]: HEALED content anchor from own conductor (peer discovery)"
                    );
                }
                Ok(false) => {
                    // Row vanished between presence check and stamp (rare race).
                    // Nothing to stamp — resolved, not a conductor miss.
                    tracker.mark_completed(&id);
                    tracing::debug!(content_id = %id, "projection-reconcile[content]: stamp found no local row; nothing to do");
                }
                Err(e) => {
                    tracing::warn!(content_id = %id, error = %e, "projection-reconcile[content]: stamp failed; retry next sweep");
                    tracker.mark_failed(&id);
                }
            },
            Ok(None) => {
                // Conductor can't see it yet (catch-up) — retry on the NEXT
                // sweep via a fresh inventory diff, never an immediate re-queue.
                conductor_missing += 1;
                tracing::debug!(content_id = %id, "projection-reconcile[content]: own conductor returned None; retry next sweep");
                tracker.mark_failed(&id);
            }
            Err(e) => {
                tracing::warn!(content_id = %id, error = %e, "projection-reconcile[content]: conductor resolve failed; retry next sweep");
                tracker.mark_failed(&id);
            }
        }
    }

    (healed, conductor_missing)
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
) -> Result<bool, crate::error::StorageError> {
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
    crate::db::content_diesel::stamp_declared_head(
        &mut conn,
        app_ctx,
        &c.id,
        head.head_action_hash.as_str(),
        Some(patch),
    )
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
}
