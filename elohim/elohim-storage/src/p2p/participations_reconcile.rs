//! Cross-peer `collective_participations` reconcile arm — memberships converge
//! from the OWN conductor, with peers used as discovery only.
//!
//! ## Why this exists (the incident it cures)
//!
//! A household's members each affirm membership on their OWN conductor, and
//! `MembershipCommitted` `post_commit` fires only there. Nothing else ever
//! carried a participation row across peers: `projection_reconcile` grew arms
//! for `rea_commitments`, `content` and `collectives`, but never one for the
//! memberships those collectives are made of. So a Membership authored by
//! jessica's node was invisible on matthew's forever — the
//! `{"items": [], "count": 0}` the household-formation E2E measured against
//! `GET /db/collectives/family-dowell/participants` on 2026-08-20, on a fleet
//! where all three memberships plainly existed.
//!
//! ## The design contract
//!
//! Identical in shape to the collectives arm (see
//! [`crate::p2p::projection_reconcile`]'s module doc), on the same shared
//! [`GapTracker`] / [`MissLedger`] rails:
//!
//! - **Peer SQL is discovery ONLY.** Peers advertise the Membership ActionHashes
//!   they hold over `ViewKind::ProjectionInventory { table:
//!   "collective_participations" }`. For each anchor we do not hold, we call our
//!   OWN conductor's `imagodei::get_membership_by_action`. Peer bytes are NEVER
//!   written into the projection.
//! - **Upsert through the shared mapping.** Both the post-commit signal arm
//!   (`ReconcileController::on_membership_projected`) and this arm funnel
//!   through [`crate::db::memberships::project_membership`], so the
//!   `collective_id` / `human_id` resolution and the household backfill cannot
//!   drift between them.
//! - **The identity is the anchor.** The diesel row id is a peer-local UUID and
//!   never reaches the wire; the responder puts the anchor in both inventory
//!   slots. There is consequently NO divergence class here — a row either
//!   carries an anchor or it does not, so every admitted gap is healable.
//! - **Gap discipline.** Conductor-can't-see-it → `mark_failed`, retried on the
//!   NEXT sweep (never an immediate re-queue).
//!
//! `h_app_id` partition: participations live under the canonical
//! [`crate::db::PARTICIPATIONS_HAPP_ID`] (`qahal`), owned by the db-layer
//! functions themselves — they take no `AppContext`, so this arm cannot scope
//! them wrongly.

use std::sync::Arc;

use crate::db::DbPool;
use crate::hc_client::HcClient;
use crate::p2p::projection_reconcile::{
    call_with_retry, is_transient_conductor_error, Admission, HealCircuit, HealOutcomeKind,
    HealPacing, InventoryWindow, MissLedger, MAX_INVENTORY_WINDOW_TOTAL, MAX_RETRIES, PEER_TIMEOUT,
};
use crate::p2p::reconcile_rails::GapTracker;
use crate::p2p::view_federation::PROJECTION_INVENTORY_TABLE_PARTICIPATIONS;
use crate::p2p::P2PHandle;
use crate::views::{ProjectionInventoryPayload, ViewFederationRequest, ViewKind};

/// Discovery-side output for the participations arm. Mirrors
/// `CollectivesDiscovery`; the tracker is keyed by `dht_anchor_hash` (the
/// Membership ActionHash — the reconciliation identity), NOT by the diesel row
/// id (a peer-local UUID).
pub struct ParticipationsDiscovery {
    pub(crate) tracker: GapTracker,
    /// anchor → the first peer that advertised it (for the heal WARN).
    pub(crate) discovered_by: std::collections::HashMap<String, String>,
    /// Gap anchors the cross-sweep [`MissLedger`] held back this sweep.
    /// Published on
    /// `elohim_projection_reconcile_exhausted{stream="participations"}`.
    pub(crate) exhausted_persistent: usize,
    pub(crate) peers_asked: usize,
    pub(crate) ids_discovered: usize,
    pub(crate) local_anchored: usize,
    /// This arm actually OBSERVED the state it reports — see `ReaDiscovery`'s
    /// twin field. An unmeasured arm must never read as convergence.
    pub(crate) measured: bool,
}

impl ParticipationsDiscovery {
    /// Empty discovery (db unavailable this tick) — the heal leg has nothing to
    /// do. UNMEASURED by construction.
    fn empty(peers_asked: usize) -> Self {
        Self {
            tracker: GapTracker::new(MAX_RETRIES),
            discovered_by: std::collections::HashMap::new(),
            exhausted_persistent: 0,
            peers_asked,
            ids_discovered: 0,
            local_anchored: 0,
            measured: false,
        }
    }
}

/// How ONE advertised Membership anchor classifies against the local
/// `collective_participations` projection.
///
/// Deliberately two-valued. The collectives arm needs a `Divergent` case because
/// it reconciles a cid onto a peer-local ROUTING ALIAS, and two peers can hold
/// different cids under the same alias. Here the anchor IS the row key, so
/// "present with a different identity" is not a representable state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParticipationGap {
    /// No local row carries this anchor. Heal reads the Membership from the OWN
    /// conductor and projects it.
    AbsentLocal,
    /// A local row already carries this anchor — nothing to do.
    InSync,
}

/// Pure diff for ONE advertised anchor.
///
/// An empty/whitespace anchor is treated as NO EVIDENCE (`InSync`), never as a
/// gap: a responder never advertises an un-anchored row, so an empty value here
/// is defensive — and enqueuing it would burn a conductor round-trip on
/// something that can never resolve.
pub(crate) fn classify_participation_gap(
    anchor: &str,
    local_anchors: &std::collections::HashSet<String>,
) -> ParticipationGap {
    let trimmed = anchor.trim();
    if trimmed.is_empty() || local_anchors.contains(trimmed) {
        return ParticipationGap::InSync;
    }
    ParticipationGap::AbsentLocal
}

/// Discovery phase of the participations reconcile. No conductor call happens
/// here — [`heal_participations`] owns that.
///
/// 1. Ask every connected peer for its `ProjectionInventory
///    { collective_participations }`.
/// 2. One `participation_anchors_present` query resolves local presence for
///    every advertised anchor.
/// 3. Classify; absent anchors feed a per-sweep [`GapTracker`] keyed by anchor.
///
/// An anchor this node cannot even DECODE is dropped at discovery rather than
/// enqueued: it can never resolve, so enqueuing it would burn a conductor
/// round-trip every sweep forever (the same ruling the collectives arm makes on
/// an undecodable cid).
pub async fn discover_participations(
    p2p: &P2PHandle,
    pool: &DbPool,
    window: &mut InventoryWindow,
    misses: &mut MissLedger,
) -> ParticipationsDiscovery {
    let sweep_offset = window.offset_for(PROJECTION_INVENTORY_TABLE_PARTICIPATIONS);
    let mut max_peer_total: u64 = 0;

    // (1) Ask connected peers.
    let peers = p2p.list_peers().await;
    let mut peers_asked = 0usize;
    let mut ids_discovered = 0usize;
    // anchor → first peer that advertised it.
    let mut advertised_by: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for peer in &peers {
        let peer_id = match peer.peer_id.parse::<libp2p::PeerId>() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let request = ViewFederationRequest {
            view_kind: ViewKind::ProjectionInventory {
                table: PROJECTION_INVENTORY_TABLE_PARTICIPATIONS.to_string(),
            },
            agent_cid: p2p.agent_pubkey().to_string(),
            request_id: uuid::Uuid::new_v4().to_string(),
            inventory_offset: Some(sweep_offset),
            head_corpus_digest: None,
        };
        // Q11 atomic timing budget: the inventory-page RPC itself, per peer per
        // table — timed regardless of outcome (a timeout is a real cost).
        let inventory_page_started = std::time::Instant::now();
        let call_result = p2p.view_federate(peer_id, request, PEER_TIMEOUT).await;
        crate::metrics::observe_atom_duration(
            crate::metrics::ConvergenceAtom::InventoryPage,
            inventory_page_started.elapsed(),
        );
        let resp = match call_result {
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
                    "projection-reconcile[participations]: peer inventory payload undecodable; skipping peer"
                );
                continue;
            }
        };

        max_peer_total = max_peer_total.max(payload.total as u64);
        let windowed =
            (sweep_offset as usize).saturating_add(payload.entries.len()) < payload.total;

        tracing::info!(
            target: "elohim_storage::projection_reconcile",
            peer = %peer.peer_id,
            entries = payload.entries.len(),
            peer_total = payload.total,
            offset = sweep_offset,
            windowed = windowed,
            "projection-reconcile[participations]: peer inventory received"
        );

        ids_discovered += payload.entries.len();
        for entry in &payload.entries {
            let anchor = entry.dht_anchor_hash.trim();
            if anchor.is_empty() {
                continue; // no DHT identity, nothing to reconcile on
            }
            advertised_by
                .entry(anchor.to_string())
                .or_insert_with(|| peer.peer_id.clone());
        }
    }

    window.advance(
        PROJECTION_INVENTORY_TABLE_PARTICIPATIONS,
        sweep_offset,
        max_peer_total,
    );
    // Window-progress companions, measured-gated (see the REA arm's twin call).
    if peers_asked > 0 {
        crate::metrics::set_projection_reconcile_window_gauges(
            "participations",
            u64::from(sweep_offset),
            max_peer_total.min(MAX_INVENTORY_WINDOW_TOTAL),
        );
    }

    // (2) One presence query for the advertised anchors. The LOCAL side is read
    // whole (no window): the rotating window only bounds the PEER ask.
    let advertised: Vec<String> = advertised_by.keys().cloned().collect();
    let (present, local_anchored) = {
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile[participations]: db conn failed; skipping sweep");
                return ParticipationsDiscovery::empty(peers_asked);
            }
        };
        let local_anchored = match crate::db::collectives::list_participation_anchor_inventory(
            &mut conn,
            0,
            i64::MAX,
        ) {
            Ok((_rows, total)) => usize::try_from(total).unwrap_or(usize::MAX),
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile[participations]: local inventory failed; skipping sweep");
                return ParticipationsDiscovery::empty(peers_asked);
            }
        };
        // Chunk under SQLITE_MAX_VARIABLE_NUMBER (see `participation_anchors_present`).
        const PRESENCE_CHUNK: usize = 500;
        let mut acc = std::collections::HashSet::new();
        for chunk in advertised.chunks(PRESENCE_CHUNK) {
            match crate::db::collectives::participation_anchors_present(&mut conn, chunk) {
                Ok(s) => acc.extend(s),
                Err(e) => {
                    tracing::warn!(error = %e, "projection-reconcile[participations]: presence query failed; skipping heal");
                    return ParticipationsDiscovery::empty(peers_asked);
                }
            }
        }
        (acc, local_anchored)
    };

    // (3) Classify → gap set, keyed by anchor.
    let mut gap_anchors: Vec<String> = Vec::new();
    let mut discovered_by: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut undecodable = 0usize;

    for (anchor, peer) in &advertised_by {
        match classify_participation_gap(anchor, &present) {
            ParticipationGap::InSync => {}
            ParticipationGap::AbsentLocal => {
                // An anchor we cannot decode can never resolve — never enqueue it.
                if let Err(e) = crate::services::conductor_writes::decode_action_hash(anchor) {
                    undecodable += 1;
                    tracing::warn!(
                        target: "elohim_storage::projection_reconcile",
                        peer_anchor = %anchor,
                        error = %e,
                        "projection-reconcile[participations]: undecodable peer anchor; not enqueued"
                    );
                    continue;
                }
                discovered_by.insert(anchor.clone(), peer.clone());
                gap_anchors.push(anchor.clone());
            }
        }
    }

    if undecodable > 0 {
        tracing::warn!(
            target: "elohim_storage::projection_reconcile",
            undecodable,
            "projection-reconcile[participations]: dropped undecodable peer anchors this sweep"
        );
    }

    // Cross-sweep retry budget. The gap key IS the anchor, so "new evidence" can
    // only arrive as a NEW key — the ledger's evidence field is the anchor
    // itself and the cooldown is what re-admits. `divergent = false` always:
    // this arm has no divergence class (see [`ParticipationGap`]).
    let mut exhausted_persistent = 0usize;
    let mut admitted: Vec<String> = Vec::with_capacity(gap_anchors.len());
    for anchor in gap_anchors {
        match misses.admit(
            PROJECTION_INVENTORY_TABLE_PARTICIPATIONS,
            &anchor,
            &anchor,
            false,
        ) {
            Admission::Retry => admitted.push(anchor),
            Admission::Exhausted => exhausted_persistent += 1,
        }
    }

    let mut tracker = GapTracker::new(MAX_RETRIES);
    tracker.discover(admitted);

    ParticipationsDiscovery {
        tracker,
        discovered_by,
        exhausted_persistent,
        peers_asked,
        ids_discovered,
        local_anchored,
        // OBSERVED, not merely "no error thrown" — a peer-partition sweep must
        // not read as measured.
        measured: peers_asked > 0,
    }
}

/// What one participations heal leg produced.
pub struct ParticipationsHealOutcome {
    /// Rows that actually converged.
    pub(crate) healed: usize,
    /// Gaps the own conductor could not resolve at all.
    pub(crate) conductor_missing: usize,
}

/// Heal phase: for each discovered anchor, read the OWN conductor's
/// `imagodei::get_membership_by_action` and project through the SHARED mapping.
/// `None` → `mark_failed`, retried next sweep.
///
/// Row content comes EXCLUSIVELY from the own conductor's `Membership` entry;
/// no peer-derived value reaches the projection at all (unlike the collectives
/// arm, which carries a routing-alias merge candidate — there is no alias here).
pub async fn heal_participations(
    tracker: &mut GapTracker,
    discovered_by: &std::collections::HashMap<String, String>,
    hc: &Arc<HcClient>,
    pool: &DbPool,
    pacing: &HealPacing,
) -> ParticipationsHealOutcome {
    use crate::db::memberships::MembershipProjectionOutcome as Outcome;

    let leg_start = std::time::Instant::now();
    let mut healed = 0usize;
    let mut conductor_missing = 0usize;

    let mut circuit = HealCircuit::new(pacing.circuit_timeout_threshold);
    for anchor in tracker.pending_ids() {
        let attempt = call_with_retry(pacing, || {
            crate::services::conductor_writes::get_membership_by_anchor(hc, &anchor)
        })
        .await;
        circuit.record(&attempt.result);
        match attempt.result {
            Ok(Some(wire)) => match heal_participation_one(&anchor, &wire, pool) {
                Ok(Outcome::Projected) => {
                    tracker.mark_completed(&anchor);
                    healed += 1;
                    let peer = discovered_by
                        .get(&anchor)
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string());
                    tracing::warn!(
                        target: "elohim_storage::projection_reconcile",
                        dht_anchor_hash = %anchor,
                        discovered_via_peer = %peer,
                        retried = attempt.retried,
                        "projection-reconcile[participations]: HEALED membership from own conductor (peer discovery)"
                    );
                    crate::metrics::inc_projection_heal_outcome(
                        "participations",
                        if attempt.retried {
                            HealOutcomeKind::TimeoutRetried.label()
                        } else {
                            HealOutcomeKind::Healed.label()
                        },
                    );
                }
                Ok(Outcome::Refreshed) => {
                    // A row already carried this anchor (a live signal landed it
                    // between discovery and heal). Deliberately NOT counted as
                    // healed — a no-op counted as a cure hides a spinning heal
                    // plane.
                    tracker.mark_completed(&anchor);
                    tracing::info!(
                        target: "elohim_storage::projection_reconcile",
                        dht_anchor_hash = %anchor,
                        "projection-reconcile[participations]: row already anchored — refreshed, nothing converged"
                    );
                    crate::metrics::inc_projection_heal_outcome(
                        "participations",
                        HealOutcomeKind::Refreshed.label(),
                    );
                }
                Err(e) => {
                    tracing::warn!(dht_anchor_hash = %anchor, error = %e, "projection-reconcile[participations]: projection failed; retry next sweep");
                    tracker.mark_failed(&anchor);
                    crate::metrics::inc_projection_heal_outcome(
                        "participations",
                        HealOutcomeKind::Failed.label(),
                    );
                }
            },
            Ok(None) => {
                // Conductor can't see it yet (not gossiped in / foreign DHT) —
                // retried on the NEXT sweep via a fresh inventory diff.
                conductor_missing += 1;
                tracing::debug!(dht_anchor_hash = %anchor, "projection-reconcile[participations]: own conductor returned None; retry next sweep");
                tracker.mark_failed(&anchor);
                crate::metrics::inc_projection_heal_outcome(
                    "participations",
                    HealOutcomeKind::Missing.label(),
                );
            }
            Err(e) => {
                // A conductor whose imagodei coordinator predates
                // `get_membership_by_action` answers UnknownFunction for every
                // anchor in the leg. The DNA hash is blind to coordinator zomes,
                // so this is a real deploy state, not a bug — shed the leg
                // instead of burning a round-trip per gap until the hot-swap
                // lands.
                if crate::services::conductor_writes::is_unknown_function_error(&e) {
                    tracing::warn!(
                        target: "elohim_storage::projection_reconcile",
                        error = %e,
                        healed,
                        "projection-reconcile[participations]: own conductor does not serve \
                         get_membership_by_action (coordinator not yet hot-swapped) — shedding the \
                         rest of the leg; gaps resume next sweep"
                    );
                    tracker.mark_failed(&anchor);
                    crate::metrics::inc_projection_heal_outcome(
                        "participations",
                        HealOutcomeKind::Failed.label(),
                    );
                    break;
                }
                let transient = is_transient_conductor_error(&e);
                tracing::warn!(dht_anchor_hash = %anchor, error = %e, transient, "projection-reconcile[participations]: conductor resolve failed; retry next sweep");
                tracker.mark_failed(&anchor);
                crate::metrics::inc_projection_heal_outcome(
                    "participations",
                    if transient {
                        HealOutcomeKind::TimeoutExhausted.label()
                    } else {
                        HealOutcomeKind::Failed.label()
                    },
                );
            }
        }

        // Per-leg budget: yield so the leg recycles next sweep (see `heal_rea`).
        if leg_start.elapsed() >= pacing.participations_leg_budget {
            tracing::info!(
                target: "elohim_storage::projection_reconcile",
                budget_secs = pacing.participations_leg_budget.as_secs(),
                healed,
                "projection-reconcile[participations]: heal hit leg budget — yielding, remaining gaps resume next sweep"
            );
            break;
        }

        // Unresponsive-conductor circuit: shed the rest of the leg rather than
        // stack more abandoned-but-still-executing calls on it.
        if circuit.is_open() {
            tracing::warn!(
                target: "elohim_storage::projection_reconcile",
                consecutive_timeouts = circuit.consecutive_timeouts(),
                healed,
                "projection-reconcile[participations]: OPENED the unresponsive-conductor circuit — shedding the rest of the leg, remaining gaps resume next sweep"
            );
            break;
        }
    }

    // END-OF-ARM caught_up — an arm that skips this publishes a structurally
    // false bit (see the same call at the end of `heal_content`).
    tracker.update_caught_up();
    ParticipationsHealOutcome {
        healed,
        conductor_missing,
    }
}

/// Project ONE conductor-read `Membership` into local SQL via the SHARED
/// mapping the post-commit signal arm also uses.
fn heal_participation_one(
    anchor: &str,
    wire: &crate::services::conductor_writes::MembershipWire,
    pool: &DbPool,
) -> Result<crate::db::memberships::MembershipProjectionOutcome, crate::error::StorageError> {
    // The DNA carries a departure as a block height on an UPDATED entry; the
    // signal path maps that to an observation timestamp the same way
    // (`translate_imagodei`), because the projection column is an ISO string and
    // block heights are not comparable to it. Same shape, one meaning: "this
    // membership is no longer active, first observed now".
    let departed_at = wire
        .withdrawn_at_block_height
        .map(|_| chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());

    let mut conn = pool
        .get()
        .map_err(|e| crate::error::StorageError::Internal(format!("pool: {e}")))?;
    crate::db::memberships::project_membership(
        &mut conn,
        &crate::db::memberships::MembershipProjection {
            action_hash: anchor,
            collective_cid: &wire.collective_cid,
            member_cid: &wire.member_cid,
            member_kind: wire.member_kind.as_db_str(),
            role_context: wire.role.as_db_str(),
            departed_at: departed_at.as_deref(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const ANCHOR_A: &str = "uhCkkMembershipAlphaActionHash00000000000000";
    const ANCHOR_B: &str = "uhCkkMembershipBravoActionHash00000000000000";

    #[test]
    fn an_anchor_we_do_not_hold_is_a_gap() {
        let local: HashSet<String> = [ANCHOR_A.to_string()].into_iter().collect();
        assert_eq!(
            classify_participation_gap(ANCHOR_B, &local),
            ParticipationGap::AbsentLocal,
            "a membership this peer has never seen is healable — the own conductor answers for it"
        );
    }

    #[test]
    fn an_anchor_we_already_hold_is_in_sync() {
        let local: HashSet<String> = [ANCHOR_A.to_string()].into_iter().collect();
        assert_eq!(
            classify_participation_gap(ANCHOR_A, &local),
            ParticipationGap::InSync
        );
    }

    /// A responder never advertises an un-anchored row, so an empty anchor is
    /// no evidence — never a gap. Enqueuing it would burn a conductor
    /// round-trip every sweep on something that can never resolve.
    #[test]
    fn an_empty_anchor_is_no_evidence_not_a_gap() {
        let local: HashSet<String> = HashSet::new();
        for anchor in ["", "   "] {
            assert_eq!(
                classify_participation_gap(anchor, &local),
                ParticipationGap::InSync
            );
        }
    }

    /// The anchor is compared TRIMMED on both sides — a peer that pads its
    /// advertisement must not read as a permanent gap against a row we hold.
    #[test]
    fn a_padded_anchor_matches_the_local_holding() {
        let local: HashSet<String> = [ANCHOR_A.to_string()].into_iter().collect();
        assert_eq!(
            classify_participation_gap(&format!("  {ANCHOR_A} "), &local),
            ParticipationGap::InSync
        );
    }

    /// The participations leg budget is reserved and SMALLEST — it runs last
    /// and must never starve the arms that carry the fleet's real backlog.
    #[test]
    fn participations_leg_budget_is_the_smallest_reserved_slice() {
        let p = HealPacing::default();
        assert!(
            p.participations_leg_budget <= p.rea_leg_budget
                && p.participations_leg_budget <= p.content_leg_budget
                && p.participations_leg_budget <= p.collectives_leg_budget,
            "participations runs last with the smallest reserved budget"
        );
    }
}
