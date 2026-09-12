//! Cross-peer `humans` identity-BINDING reconcile arm — a divergent
//! `agent_pub_key` converges to the OWN conductor's membership truth, with peers
//! used as discovery only.
//!
//! ## Why this exists (the incident it cures)
//!
//! `humans` is an operational projection of DHT membership truth, and every
//! writer that touches `agent_pub_key` is deliberately NULL-only:
//! `reconcile::controller::on_membership_projected` must never regress a key on
//! an out-of-order signal, and `household_backfill` fills only what is missing.
//! The one arm that MOVES a key — `genesis_self_heal` — is hardcoded SELF-only.
//!
//! So when a peer re-keys (a non-prod DNA reinstall mints a new `AgentPubKey`
//! per pod; a runtime rotation does the same), every OTHER peer keeps the
//! FOSSIL binding forever. The fossil then poisons BOTH sides of the resilience
//! stewarding join — the holder side (`shard_locations.peer_id ==
//! humans.agent_pub_key`) and the commitment side (`rea_commitments.provider ==
//! humans.agent_pub_key`) — which is how three peers of ONE household testified
//! `intraHubPeers` 3 / 2 / 2 about identical custody facts
//! (dataplane-convergence DELTA 2026-09-12c, recorded there as "named, not
//! fixed": *`humans.agent_pub_key` has no reconcile arm*).
//!
//! `membership_identity_reconcile` already knows how to CURE this. What was
//! missing is the eye: nothing looked across peers, so nothing counted the
//! divergence, nothing reported it, and the cure fired only on its own 300 s
//! timer with no evidence that it was needed or that it had worked.
//!
//! ## The design contract
//!
//! Same shape as the collectives arm (see
//! [`crate::p2p::projection_reconcile`]'s module doc), on the same shared
//! [`GapTracker`] / [`MissLedger`] rails — with ONE deliberate difference,
//! stated plainly because it is the safety property of the whole arm:
//!
//! - **Peer SQL is discovery ONLY, and here it is discovery ONLY OF DISAGREEMENT.**
//!   Every other arm fetches the disputed object from its own conductor and
//!   writes it. This arm never writes anything a peer said, and never writes a
//!   key at all. A peer-advertised key can do exactly one thing: schedule a
//!   membership-truth pass ([`crate::services::membership_identity_reconcile`]),
//!   whose forced-bijection supersede reads the OWN conductor's `Membership`
//!   set and abstains whenever the pairing is ambiguous. Writing a peer's key
//!   directly would be an identity violation — mis-attributing one human's
//!   shards and commitments to another — strictly worse than a stale row.
//! - **One pass per sweep, not one per gap.** The cure is household-shaped and
//!   reads the whole conductor membership set; invoking it per divergent id
//!   would re-run the same read N times for one answer. The leg runs it ONCE,
//!   then re-reads the local bindings to see which ids actually converged.
//! - **`Missing` is classified, never enqueued.** A human this peer has never
//!   projected is not a binding divergence, and fabricating a `humans` row from
//!   a peer's inventory would invent an identity. It is counted so the asymmetry
//!   is visible, and left to the membership/humans projection paths that own
//!   creation.
//!
//! `h_app_id` partition: `humans` is matched scope-AGNOSTICALLY, matching every
//! other consumer of this table (see [`crate::db::humans::agent_key_inventory`]
//! and `membership_identity_reconcile`'s module doc). A scope filter on this
//! table alone ships a silent no-op.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::db::{AppContext, DbPool};
use crate::hc_client::HcClient;
use crate::p2p::projection_reconcile::{
    Admission, InventoryWindow, MissLedger, MAX_INVENTORY_WINDOW_TOTAL, MAX_RETRIES, PEER_TIMEOUT,
};
use crate::p2p::reconcile_peers::ReconcilePeers;
use crate::p2p::reconcile_rails::GapTracker;
use crate::p2p::view_federation::PROJECTION_INVENTORY_TABLE_HUMANS;
use crate::views::{ProjectionInventoryPayload, ViewFederationRequest, ViewKind};

/// How ONE advertised `(human_id, agent_pub_key)` binding classifies against the
/// local `humans` projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HumanBindingGap {
    /// Both peers bind this human to the SAME key — or the peer advertised no
    /// key at all, which is no evidence rather than a gap.
    InSync,
    /// Both peers bind this human, to DIFFERENT keys. Exactly one of them can be
    /// the live membership key; which one is a question only a conductor can
    /// answer, so this schedules a membership-truth pass and writes nothing.
    KeyDivergent,
    /// This peer has no row for the advertised human. NOT a binding divergence
    /// and NOT healable here — a `humans` row is created by the membership /
    /// humans projection paths, never fabricated from a peer's inventory.
    Missing,
}

/// Pure diff for ONE advertised binding.
///
/// An empty/whitespace peer key is NO EVIDENCE (`InSync`): the responder never
/// advertises an unbound row, so an empty value here is defensive — and treating
/// it as divergence would schedule a conductor pass over an absence.
pub(crate) fn classify_human_binding(peer_key: &str, local_key: Option<&str>) -> HumanBindingGap {
    let peer = peer_key.trim();
    if peer.is_empty() {
        return HumanBindingGap::InSync;
    }
    match local_key.map(str::trim).filter(|k| !k.is_empty()) {
        None => HumanBindingGap::Missing,
        Some(local) if local == peer => HumanBindingGap::InSync,
        Some(_) => HumanBindingGap::KeyDivergent,
    }
}

/// Discovery-side output for the humans arm. The tracker is keyed by
/// `human_id` — the stable slug, which is the ONE identifier both peers agree on
/// across a re-key (the keys are precisely what disagree).
pub struct HumansDiscovery {
    pub(crate) tracker: GapTracker,
    /// human_id → the first peer that advertised a divergent binding (for the
    /// heal WARN and the post-pass verdict log).
    pub(crate) discovered_by: HashMap<String, String>,
    /// human_id → the key the peer advertised. Carried for LOGGING and for the
    /// [`MissLedger`] evidence key ONLY — never written to the projection.
    pub(crate) peer_key: HashMap<String, String>,
    /// Bindings where the peer knows a human this peer has never projected.
    /// Counted and logged, never enqueued (see [`HumanBindingGap::Missing`]).
    pub(crate) missing_local: usize,
    /// Bindings diverging on the key — the actionable class.
    pub(crate) key_divergent: usize,
    /// Gap ids the cross-sweep [`MissLedger`] held back this sweep.
    pub(crate) exhausted_persistent: usize,
    pub(crate) peers_asked: usize,
    pub(crate) ids_discovered: usize,
    /// Locally-BOUND human rows (the honest denominator for the gauges).
    pub(crate) local_bound: usize,
    /// This arm actually OBSERVED the state it reports — see `ReaDiscovery`'s
    /// twin field. An unmeasured arm must never read as convergence.
    pub(crate) measured: bool,
}

impl HumansDiscovery {
    /// Empty discovery (db unavailable this tick). UNMEASURED by construction.
    fn empty(peers_asked: usize) -> Self {
        Self {
            tracker: GapTracker::new(MAX_RETRIES),
            discovered_by: HashMap::new(),
            peer_key: HashMap::new(),
            missing_local: 0,
            key_divergent: 0,
            exhausted_persistent: 0,
            peers_asked,
            ids_discovered: 0,
            local_bound: 0,
            measured: false,
        }
    }
}

/// Discovery phase of the humans reconcile. No conductor call happens here —
/// [`heal_humans`] owns that.
///
/// 1. Build the local `human_id → agent_pub_key` binding inventory (bound rows
///    only; the LOCAL side is read whole, the rotating window bounds the PEER
///    ask alone).
/// 2. Ask every connected peer for its `ProjectionInventory { humans }`.
/// 3. Classify each advertised binding; `KeyDivergent` ids feed a per-sweep
///    [`GapTracker`] keyed by `human_id`.
pub async fn discover_humans(
    p2p: &dyn ReconcilePeers,
    pool: &DbPool,
    window: &mut InventoryWindow,
    misses: &mut MissLedger,
) -> HumansDiscovery {
    let sweep_offset = window.offset_for(PROJECTION_INVENTORY_TABLE_HUMANS);
    let mut max_peer_total: u64 = 0;

    // (1) Local bindings, whole.
    let local_bindings: HashMap<String, String> = {
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile[humans]: db conn failed; skipping sweep");
                return HumansDiscovery::empty(0);
            }
        };
        match crate::db::humans::agent_key_inventory(&mut conn, 0, i64::MAX) {
            Ok((rows, _total)) => rows.into_iter().collect(),
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile[humans]: local inventory failed; skipping sweep");
                return HumansDiscovery::empty(0);
            }
        }
    };
    let local_bound = local_bindings.len();

    // (2) Ask connected peers.
    let peers = p2p.list_peers().await;
    let mut peers_asked = 0usize;
    let mut ids_discovered = 0usize;
    // human_id → first advertised key, and the peer that advertised it.
    let mut advertised: HashMap<String, String> = HashMap::new();
    let mut advertised_by: HashMap<String, String> = HashMap::new();

    for peer in &peers {
        let request = ViewFederationRequest {
            view_kind: ViewKind::ProjectionInventory {
                table: PROJECTION_INVENTORY_TABLE_HUMANS.to_string(),
            },
            agent_cid: p2p.agent_pubkey().to_string(),
            request_id: uuid::Uuid::new_v4().to_string(),
            inventory_offset: Some(sweep_offset),
            head_corpus_digest: None,
        };
        // Q11 atomic timing budget: the inventory-page RPC itself, per peer per
        // table — timed regardless of outcome (a timeout is a real cost).
        let inventory_page_started = std::time::Instant::now();
        let call_result = p2p
            .view_federate(&peer.peer_id, request, PEER_TIMEOUT)
            .await;
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
                    "projection-reconcile[humans]: peer inventory payload undecodable; skipping peer"
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
            "projection-reconcile[humans]: peer inventory received"
        );

        ids_discovered += payload.entries.len();
        for entry in &payload.entries {
            // The `dhtAnchorHash` slot carries the agent key for this table.
            let key = entry.dht_anchor_hash.trim();
            if key.is_empty() {
                continue; // unbound row — no binding to reconcile on
            }
            advertised
                .entry(entry.id.clone())
                .or_insert_with(|| key.to_string());
            advertised_by
                .entry(entry.id.clone())
                .or_insert_with(|| peer.peer_id.clone());
        }
    }

    window.advance(
        PROJECTION_INVENTORY_TABLE_HUMANS,
        sweep_offset,
        max_peer_total,
    );
    // Window-progress companions, measured-gated (see the REA arm's twin call).
    if peers_asked > 0 {
        crate::metrics::set_projection_reconcile_window_gauges(
            "humans",
            u64::from(sweep_offset),
            max_peer_total.min(MAX_INVENTORY_WINDOW_TOTAL),
        );
    }

    // (3) Classify.
    let mut gap_ids: Vec<String> = Vec::new();
    let mut discovered_by: HashMap<String, String> = HashMap::new();
    let mut peer_key: HashMap<String, String> = HashMap::new();
    let mut missing_local = 0usize;
    let mut key_divergent = 0usize;

    for (human_id, key) in &advertised {
        match classify_human_binding(key, local_bindings.get(human_id).map(String::as_str)) {
            HumanBindingGap::InSync => {
                misses.resolved(PROJECTION_INVENTORY_TABLE_HUMANS, human_id);
            }
            HumanBindingGap::Missing => {
                // NOT enqueued and NOT folded into the actionable count: this arm
                // cannot create a human, and counting an un-creatable gap would
                // pin `converged` false forever on any peer that simply has not
                // met someone yet.
                missing_local += 1;
            }
            HumanBindingGap::KeyDivergent => {
                key_divergent += 1;
                let local = local_bindings
                    .get(human_id)
                    .map(String::as_str)
                    .unwrap_or("");
                tracing::warn!(
                    target: "elohim_storage::projection_reconcile",
                    human_id = %human_id,
                    peer = %advertised_by.get(human_id).map(String::as_str).unwrap_or("unknown"),
                    local_key_prefix = %key_prefix(local),
                    peer_key_prefix = %key_prefix(key),
                    "projection-reconcile[humans]: KEY-DIVERGENT binding — this peer and \
                     the advertiser bind the same human to different agent keys; scheduling \
                     a membership-truth pass (no peer key is ever written)"
                );
                peer_key.insert(human_id.clone(), key.clone());
                if let Some(peer) = advertised_by.get(human_id) {
                    discovered_by.insert(human_id.clone(), peer.clone());
                }
                gap_ids.push(human_id.clone());
            }
        }
    }

    // Cross-sweep retry budget. The evidence is the peer-advertised KEY: a peer
    // that re-keys again is new evidence and re-admits an exhausted id.
    let mut exhausted_persistent = 0usize;
    let mut admitted: Vec<String> = Vec::with_capacity(gap_ids.len());
    for human_id in gap_ids {
        let evidence = peer_key.get(&human_id).map(String::as_str).unwrap_or("");
        match misses.admit(
            PROJECTION_INVENTORY_TABLE_HUMANS,
            &human_id,
            evidence,
            // Divergent by construction — every admitted id here is a binding
            // disagreement, never a plain absence.
            true,
        ) {
            Admission::Retry => admitted.push(human_id),
            Admission::Exhausted => exhausted_persistent += 1,
        }
    }

    let mut tracker = GapTracker::new(MAX_RETRIES);
    tracker.discover(admitted);

    HumansDiscovery {
        tracker,
        discovered_by,
        peer_key,
        missing_local,
        key_divergent,
        exhausted_persistent,
        peers_asked,
        ids_discovered,
        local_bound,
        // OBSERVED, not merely "no error thrown" — a peer-partition sweep must
        // not read as measured.
        measured: peers_asked > 0,
    }
}

/// Short, log-safe prefix of an agent key (mirrors
/// `membership_identity_reconcile::key_prefix`).
fn key_prefix(key: &str) -> String {
    key.chars().take(12).collect()
}

/// What one humans heal leg produced.
pub struct HumansHealOutcome {
    /// Divergent bindings the membership pass actually MOVED to a new key.
    pub(crate) healed: usize,
    /// Divergent bindings the pass left standing — the conductor's membership
    /// read was ambiguous, incomplete, or carried a withdrawal, so
    /// `membership_identity_reconcile` correctly ABSTAINED. Retried next sweep.
    pub(crate) unresolved: usize,
}

/// Heal phase: run ONE membership-truth pass against the OWN conductor, then
/// re-read the local bindings and mark each divergent id by whether its key
/// actually moved.
///
/// The pass ([`crate::services::membership_identity_reconcile::run_membership_pass`])
/// is the ONLY write path — this module contains no `humans` write of its own,
/// by design. It is correct-or-abstain: it supersedes only when a household's
/// fossil↔live pairing is a forced bijection, and skips otherwise. So an id that
/// does NOT converge here is not a failure to report as a defect; it is the
/// substrate declining to guess, and it is counted as `unresolved` rather than
/// healed so a spinning leg can never read as a cure.
pub async fn heal_humans(
    tracker: &mut GapTracker,
    discovered_by: &HashMap<String, String>,
    hc: &Arc<HcClient>,
    pool: &DbPool,
) -> HumansHealOutcome {
    use crate::services::holochain_humans_replayer::ConductorMembershipReader;
    use crate::services::membership_identity_reconcile;

    let pending: Vec<String> = tracker.pending_ids();
    if pending.is_empty() {
        tracker.update_caught_up();
        return HumansHealOutcome {
            healed: 0,
            unresolved: 0,
        };
    }

    // Snapshot the bindings BEFORE the pass: "did this id's key move" is not
    // answerable from the after-state alone.
    let before: HashMap<String, String> = match pool.get() {
        Ok(mut conn) => match crate::db::humans::agent_key_inventory(&mut conn, 0, i64::MAX) {
            Ok((rows, _)) => rows.into_iter().collect(),
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile[humans]: pre-pass inventory failed; retry next sweep");
                for id in &pending {
                    tracker.mark_failed(id);
                }
                tracker.update_caught_up();
                return HumansHealOutcome {
                    healed: 0,
                    unresolved: pending.len(),
                };
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, "projection-reconcile[humans]: db conn failed; retry next sweep");
            for id in &pending {
                tracker.mark_failed(id);
            }
            tracker.update_caught_up();
            return HumansHealOutcome {
                healed: 0,
                unresolved: pending.len(),
            };
        }
    };

    // ONE pass for the whole leg — the cure is household-shaped, not per-id.
    let ctx = AppContext::default_lamad();
    let reader = ConductorMembershipReader {
        hc_client: hc.clone(),
    };
    membership_identity_reconcile::run_membership_pass(pool, &ctx, &reader).await;

    let after: HashMap<String, String> = match pool.get() {
        Ok(mut conn) => match crate::db::humans::agent_key_inventory(&mut conn, 0, i64::MAX) {
            Ok((rows, _)) => rows.into_iter().collect(),
            Err(e) => {
                tracing::warn!(error = %e, "projection-reconcile[humans]: post-pass inventory failed; retry next sweep");
                for id in &pending {
                    tracker.mark_failed(id);
                }
                tracker.update_caught_up();
                return HumansHealOutcome {
                    healed: 0,
                    unresolved: pending.len(),
                };
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, "projection-reconcile[humans]: db conn failed post-pass; retry next sweep");
            for id in &pending {
                tracker.mark_failed(id);
            }
            tracker.update_caught_up();
            return HumansHealOutcome {
                healed: 0,
                unresolved: pending.len(),
            };
        }
    };

    let moved = moved_bindings(&before, &after);
    let mut healed = 0usize;
    let mut unresolved = 0usize;
    for id in &pending {
        if moved.contains(id) {
            tracker.mark_completed(id);
            healed += 1;
            tracing::warn!(
                target: "elohim_storage::projection_reconcile",
                human_id = %id,
                discovered_via_peer = %discovered_by.get(id).map(String::as_str).unwrap_or("unknown"),
                new_key_prefix = %key_prefix(after.get(id).map(String::as_str).unwrap_or("")),
                "projection-reconcile[humans]: HEALED binding — membership truth superseded \
                 the fossil key (peer discovery)"
            );
            crate::metrics::inc_projection_heal_outcome(
                "humans",
                crate::p2p::projection_reconcile::HealOutcomeKind::Healed.label(),
            );
        } else {
            tracker.mark_failed(id);
            unresolved += 1;
            tracing::info!(
                target: "elohim_storage::projection_reconcile",
                human_id = %id,
                "projection-reconcile[humans]: membership pass ABSTAINED on this binding \
                 (ambiguous / incomplete / withdrawal) — left stale on purpose, retry next sweep"
            );
            crate::metrics::inc_projection_heal_outcome(
                "humans",
                crate::p2p::projection_reconcile::HealOutcomeKind::RefusedDeclared.label(),
            );
        }
    }

    // END-OF-ARM caught_up — an arm that skips this publishes a structurally
    // false bit (see `heal_content`).
    tracker.update_caught_up();
    HumansHealOutcome { healed, unresolved }
}

/// Ids whose binding CHANGED between two inventory snapshots.
///
/// Pure + total so the "did the pass actually move anything" verdict is
/// testable without a conductor. An id that appears only in `after` counts as
/// moved (it went from unbound to bound); one that disappears does not (a row
/// losing its key is not convergence).
pub(crate) fn moved_bindings(
    before: &HashMap<String, String>,
    after: &HashMap<String, String>,
) -> HashSet<String> {
    after
        .iter()
        .filter(|(id, key)| before.get(*id).map(|b| b != *key).unwrap_or(true))
        .map(|(id, _)| id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY_OLD: &str = "uhCAkFossilAgentKey000000000000000000000000";
    const KEY_NEW: &str = "uhCAkLiveAgentKey00000000000000000000000000";

    #[test]
    fn identical_bindings_are_in_sync() {
        assert_eq!(
            classify_human_binding(KEY_NEW, Some(KEY_NEW)),
            HumanBindingGap::InSync
        );
    }

    /// THE 2026-09-12c class: matthew binds jessica to a previous-generation key
    /// while jessica's own peer advertises the live one.
    #[test]
    fn different_bindings_for_the_same_human_are_key_divergent() {
        assert_eq!(
            classify_human_binding(KEY_NEW, Some(KEY_OLD)),
            HumanBindingGap::KeyDivergent
        );
    }

    #[test]
    fn a_human_this_peer_has_never_projected_is_missing_not_divergent() {
        assert_eq!(
            classify_human_binding(KEY_NEW, None),
            HumanBindingGap::Missing
        );
        // An empty local key is an unbound row, which is the same absence.
        assert_eq!(
            classify_human_binding(KEY_NEW, Some("")),
            HumanBindingGap::Missing
        );
    }

    /// An empty peer key is NO EVIDENCE. A responder never advertises an unbound
    /// row, so this is defensive — but treating it as divergence would schedule
    /// a conductor pass over an absence, every sweep, forever.
    #[test]
    fn an_empty_peer_key_is_never_a_gap() {
        assert_eq!(
            classify_human_binding("", Some(KEY_OLD)),
            HumanBindingGap::InSync
        );
        assert_eq!(classify_human_binding("   ", None), HumanBindingGap::InSync);
    }

    #[test]
    fn moved_bindings_names_only_the_keys_that_actually_changed() {
        let before: HashMap<String, String> = [
            ("human-jessica".to_string(), KEY_OLD.to_string()),
            ("human-matthew".to_string(), KEY_NEW.to_string()),
            ("human-departed".to_string(), KEY_OLD.to_string()),
        ]
        .into_iter()
        .collect();
        let after: HashMap<String, String> = [
            // superseded by the pass
            ("human-jessica".to_string(), KEY_NEW.to_string()),
            // untouched
            ("human-matthew".to_string(), KEY_NEW.to_string()),
            // newly bound
            ("human-james".to_string(), KEY_NEW.to_string()),
            // "human-departed" dropped out entirely
        ]
        .into_iter()
        .collect();

        let moved = moved_bindings(&before, &after);
        assert!(moved.contains("human-jessica"), "a superseded key moved");
        assert!(moved.contains("human-james"), "unbound → bound is a move");
        assert!(
            !moved.contains("human-matthew"),
            "an unchanged binding is not a cure"
        );
        assert!(
            !moved.contains("human-departed"),
            "a row that LOST its key has not converged — never count it healed"
        );
        assert_eq!(moved.len(), 2);
    }

    /// The arm's inventory must advertise BOUND rows only: an unbound row
    /// carries no binding to disagree with, and advertising it would make every
    /// peer classify it `Missing`/`InSync` on noise.
    #[test]
    fn the_inventory_advertises_bound_rows_only() {
        use crate::db::models::NewHuman;
        use crate::db::{init_pool, run_migrations};
        use diesel::prelude::*;

        let pool = init_pool(":memory:").expect("pool");
        run_migrations(&pool).expect("migrations");
        let mut conn = pool.get().expect("conn");

        let row = |id: &str, key: Option<&str>| NewHuman {
            id: id.to_string(),
            agent_pub_key: key.map(str::to_string),
            display_name: id.to_string(),
            bio: None,
            affinities: "[]".to_string(),
            profile_reach: "collective".to_string(),
            location: None,
            profile_photo_url: None,
            h_app_id: crate::db::HUMANS_HAPP_ID.to_string(),
            household_id: None,
        };

        for h in [
            row("human-jessica", Some(KEY_OLD)),
            row("human-nobody", None),
        ] {
            diesel::insert_into(crate::db::diesel_schema::humans::table)
                .values(&h)
                .execute(&mut conn)
                .expect("seed human");
        }

        let (entries, total) =
            crate::db::humans::agent_key_inventory(&mut conn, 0, i64::MAX).expect("inventory");
        assert_eq!(total, 1, "only the BOUND row is advertisable");
        assert_eq!(
            entries,
            vec![("human-jessica".to_string(), KEY_OLD.to_string())]
        );
    }
}
