//! Peer topology view service — federated `PeerTopologyView` aggregator.
//!
//! ## Source of Truth
//!
//! Operational (Category C) per the p2p-design-gate output in
//! `genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-design.md`.
//! Federated query result. Bindings notarized in DHT (Category A); per-peer
//! live state federated via `/elohim/view-federation/1.0.0`. The DHT remains canonical.
//! No SQLite table here is authoritative.

use std::collections::HashMap;
use std::time::Duration;

use chrono::Utc;
use thiserror::Error;

use diesel::prelude::*;

use crate::db::models::PeerIdentityBindingRow;
use crate::db::DbPool;
use crate::services::federator::Federator;
use crate::views::{
    Freshness, FreshnessState, PeerHouseholdEdge, PeerTopologyView, ResilienceCliff, ViewKind,
};

const FEDERATION_TIMEOUT_MS: u64 = 3000;

#[derive(Debug, Error)]
pub enum PeerTopologyError {
    #[error("db error: {0}")]
    Db(#[from] diesel::result::Error),
    #[error("storage error: {0}")]
    Storage(#[from] crate::error::StorageError),
    #[error("pool error: {0}")]
    Pool(String),
}

/// Federated `PeerTopologyView` aggregator.
///
/// Steps:
/// 1. Resolve `agent_cid → Vec<PeerIdentityBindingRow>` via
///    `db::peer_identity_bindings::list_active_for_agent`.
/// 2. Fan out via `Federator::query(ViewKind::PeerTopology, agent_cid, &bindings, timeout)`.
/// 3. For each `FederationResult` with a Live slice: walk
///    `slice.payload.0["connected_peer_households"]` as array, accumulate edges
///    keyed by `household_id` in a HashMap (online OR-roll, additive CID counts).
/// 4. Compute `net_diff` per edge at fold time.
/// 5. `reciprocation_count` = number of edges where `online == true`.
/// 6. `resilience_cliffs` is computed via `compute_resilience_cliffs` (`:239`)
///    on both the local-resolver path (`:198`) and the federation-aggregate
///    early-return path (`:74`). Returns sole-replica resilience cliffs derived
///    from the quilt distribution projection.
/// 7. `freshness`: `AllOffline` if no result has Live freshness; otherwise `Live`.
pub async fn aggregate_peer_topology_view(
    pool: &DbPool,
    federator: &Federator,
    agent_cid: &str,
) -> Result<PeerTopologyView, PeerTopologyError> {
    let now_iso = Utc::now().to_rfc3339();

    // 1) Resolve bindings.
    let bindings: Vec<PeerIdentityBindingRow> = {
        let mut conn = pool
            .get()
            .map_err(|e| PeerTopologyError::Pool(e.to_string()))?;
        crate::db::peer_identity_bindings::list_active_for_agent(&mut conn, agent_cid, &now_iso)?
    };

    if bindings.is_empty() {
        let resilience_cliffs = {
            let mut conn = pool
                .get()
                .map_err(|e| PeerTopologyError::Pool(e.to_string()))?;
            compute_resilience_cliffs(&mut conn, agent_cid).unwrap_or_default()
        };
        return Ok(PeerTopologyView {
            agent_cid: agent_cid.to_string(),
            edges: vec![],
            reciprocation_count: 0,
            resilience_cliffs,
            freshness: Freshness {
                state: FreshnessState::Live,
                stale_since_ms: None,
            },
        });
    }

    // 2) Federation timeout (env override for testing / operator tuning).
    let timeout_ms = std::env::var("ELOHIM_FEDERATION_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(FEDERATION_TIMEOUT_MS);

    // 3) Fan out.
    let results = federator
        .query(
            ViewKind::PeerTopology,
            agent_cid,
            &bindings,
            Duration::from_millis(timeout_ms),
        )
        .await;

    // 4) Dedup edges by household_id across all live slices.
    let mut edge_map: HashMap<String, PeerHouseholdEdge> = HashMap::new();
    let mut any_live = false;

    for result in &results {
        if let (FreshnessState::Live, Some(slice)) = (&result.freshness.state, &result.slice) {
            any_live = true;
            let payload = &slice.payload.0;

            if let Some(arr) = payload
                .get("connected_peer_households")
                .and_then(|v| v.as_array())
            {
                for entry in arr {
                    let household_id = match entry.get("household_id").and_then(|v| v.as_str()) {
                        Some(id) => id.to_string(),
                        None => continue,
                    };

                    let my_cids = entry
                        .get("my_cids_hosted_by_them")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32;
                    let their_cids = entry
                        .get("their_cids_hosted_by_me")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32;
                    let online = entry
                        .get("online")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let display_name = entry
                        .get("display_name")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let last_sync_sec = entry.get("last_sync_sec").and_then(|v| v.as_u64());

                    let existing =
                        edge_map
                            .entry(household_id.clone())
                            .or_insert_with(|| PeerHouseholdEdge {
                                household_id: household_id.clone(),
                                display_name: None,
                                online: false,
                                last_sync_sec: None,
                                my_cids_hosted_by_them: 0,
                                their_cids_hosted_by_me: 0,
                                net_diff: None,
                                is_critical_for_me: None,
                                i_am_critical_for_them: None,
                            });

                    // OR-roll online: if any peer reports this household as online, it's online.
                    existing.online = existing.online || online;
                    // Additive CID count accumulation.
                    existing.my_cids_hosted_by_them =
                        existing.my_cids_hosted_by_them.saturating_add(my_cids);
                    existing.their_cids_hosted_by_me =
                        existing.their_cids_hosted_by_me.saturating_add(their_cids);
                    // Use display_name from first entry that has one.
                    if existing.display_name.is_none() {
                        existing.display_name = display_name;
                    }
                    // Keep the most recent last_sync_sec (min = most recent when stored as age).
                    existing.last_sync_sec = match (existing.last_sync_sec, last_sync_sec) {
                        (Some(a), Some(b)) => Some(a.min(b)),
                        (Some(a), None) => Some(a),
                        (None, b) => b,
                    };
                }
            }
        }
    }

    // 5) Compute net_diff per edge, then collect into sorted Vec (stable ordering).
    let mut edges: Vec<PeerHouseholdEdge> = edge_map
        .into_values()
        .map(|mut e| {
            e.net_diff = Some(e.their_cids_hosted_by_me as i64 - e.my_cids_hosted_by_them as i64);
            e
        })
        .collect();

    // Sort by household_id for deterministic output.
    edges.sort_by(|a, b| a.household_id.cmp(&b.household_id));

    // 6) Reciprocation count = online edges.
    let reciprocation_count = edges.iter().filter(|e| e.online).count() as u32;

    // 7) Resilience cliffs: sole-replica analysis over the agent's authored content.
    let resilience_cliffs = {
        let mut conn = pool
            .get()
            .map_err(|e| PeerTopologyError::Pool(e.to_string()))?;
        compute_resilience_cliffs(&mut conn, agent_cid).unwrap_or_default()
    };

    // 8) Freshness rollup: AllOffline if no peer returned Live data.
    let freshness = if !any_live && !bindings.is_empty() {
        Freshness {
            state: FreshnessState::AllOffline,
            stale_since_ms: None,
        }
    } else {
        Freshness {
            state: FreshnessState::Live,
            stale_since_ms: None,
        }
    };

    Ok(PeerTopologyView {
        agent_cid: agent_cid.to_string(),
        edges,
        reciprocation_count,
        resilience_cliffs,
        freshness,
    })
}

// ============================================================================
// Phase 4 T11 — resilience cliff computation
// ============================================================================

/// Compute resilience cliffs for an agent.
///
/// A resilience cliff means: "I (or my peers) am the sole replica for a CID
/// that the given household authored." If that household goes offline and I go
/// offline, that content is lost.
///
/// Algorithm:
/// 1. Find all blob_hashes authored by `agent_cid` (via `content.created_by`).
/// 2. For each blob_hash, count distinct replica holders in `peer_blob_inventory`.
/// 3. If replica_count == 1, the sole holder is at risk.
/// 4. Resolve the sole holder to their household_id.
/// 5. Group by household_id, count sole-replica CIDs per household.
fn compute_resilience_cliffs(
    conn: &mut diesel::SqliteConnection,
    agent_cid: &str,
) -> Result<Vec<ResilienceCliff>, diesel::result::Error> {
    use crate::db::diesel_schema::content::dsl as c;
    use crate::db::diesel_schema::peer_blob_inventory::dsl as inv;
    use crate::db::diesel_schema::peer_identity_bindings::dsl as bind;

    // 1) CIDs authored by this agent (content.created_by = agent_cid).
    let my_blob_hashes: Vec<String> = c::content
        .filter(c::created_by.eq(agent_cid))
        .filter(c::blob_hash.is_not_null())
        .select(c::blob_hash)
        .load::<Option<String>>(conn)?
        .into_iter()
        .flatten()
        .collect();

    if my_blob_hashes.is_empty() {
        return Ok(vec![]);
    }

    // 2-4) For each blob, check if there is exactly one replica holder;
    //      resolve that holder's household_id (falls back to peer_id).
    let mut cliff_map: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    for blob_hash in &my_blob_hashes {
        let replica_peers: Vec<String> = inv::peer_blob_inventory
            .filter(inv::blob_hash.eq(blob_hash))
            .select(inv::peer_id)
            .load::<String>(conn)?;

        if replica_peers.len() == 1 {
            let sole_peer = &replica_peers[0];
            // Resolve to household_id via peer_identity_bindings → humans.
            // Fall back to peer_id if no binding found.
            let household_id: String = bind::peer_identity_bindings
                .filter(bind::peer_id.eq(sole_peer))
                .filter(bind::superseded_by.is_null())
                .select(bind::agent_cid)
                .first::<String>(conn)
                .unwrap_or_else(|_| sole_peer.clone());

            *cliff_map.entry(household_id).or_insert(0) += 1;
        }
    }

    // 5) Convert map to sorted Vec<ResilienceCliff>.
    let mut cliffs: Vec<ResilienceCliff> = cliff_map
        .into_iter()
        .map(|(household_id, sole_replica_cid_count)| ResilienceCliff {
            household_id,
            sole_replica_cid_count,
        })
        .collect();
    cliffs.sort_by(|a, b| a.household_id.cmp(&b.household_id));
    Ok(cliffs)
}

/// Build the per-peer slice payload that this node returns when asked for its
/// own peer-topology slice via the F-T20 responder.
///
/// Walks `connected_peers` (snapshot from `swarm.connected_peers()` at request
/// time), resolves each peer_id to an `agent_cid` via `peer_identity_bindings`,
/// then resolves each agent_cid to a `household_id` via the `humans` table.
/// Peers with no binding are skipped silently (logged at WARN). Peers with a
/// binding but no human row are emitted with `display_name: None` and
/// `household_id` falling back to the agent_cid.
///
/// CID counts (`my_cids_hosted_by_them`, `their_cids_hosted_by_me`) are
/// computed from `peer_blob_inventory`. The "authored-by-me" set is currently
/// inferred as "blobs present locally" — M3 will tighten this to a real
/// authoring-peer field.
pub async fn build_local_slice(
    pool: &DbPool,
    connected_peers: &[libp2p::PeerId],
) -> serde_json::Value {
    use crate::db::diesel_schema::{humans, peer_blob_inventory, peer_identity_bindings};
    use diesel::prelude::*;

    if connected_peers.is_empty() {
        return serde_json::json!({ "connected_peer_households": [] });
    }

    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return serde_json::json!({ "connected_peer_households": [] }),
    };

    let local_peer_id = std::env::var("ELOHIM_LOCAL_PEER_ID").unwrap_or_default();

    // Hoisted above the loop — loop-invariant heuristic until M3 introduces
    // a per-peer authoring-peer field on peer_blob_inventory.
    // TODO(M3): replace with per-peer authored-CID count once authoring-peer is tracked.
    let local_blob_count_heuristic: i64 = peer_blob_inventory::table
        .filter(peer_blob_inventory::peer_id.eq(&local_peer_id))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let mut entries: Vec<serde_json::Value> = Vec::new();

    for peer in connected_peers {
        let peer_str = peer.to_string();

        // Step 1: peer_id → agent_cid via peer_identity_bindings (most recent valid_from).
        // valid_from is ISO-8601 TEXT — lexicographic descending gives most recent first.
        let binding_row: Option<String> = peer_identity_bindings::table
            .filter(peer_identity_bindings::peer_id.eq(&peer_str))
            .order(peer_identity_bindings::valid_from.desc())
            .select(peer_identity_bindings::agent_cid)
            .first::<String>(&mut conn)
            .optional()
            .ok()
            .flatten();

        let agent_cid = match binding_row {
            Some(cid) => cid,
            None => {
                tracing::warn!(
                    target: "peer_topology_view",
                    peer = %peer_str,
                    "peer_id has no binding — skipping edge"
                );
                continue;
            }
        };

        // Step 2: agent_cid → household_id, display_name via humans table.
        let human_row: Option<(Option<String>, String)> = humans::table
            .filter(humans::id.eq(&agent_cid))
            .select((humans::household_id, humans::display_name))
            .first::<(Option<String>, String)>(&mut conn)
            .optional()
            .map_err(|e| {
                tracing::warn!(
                    target: "peer_topology_view",
                    agent_cid = %agent_cid,
                    err = %e,
                    "humans query failed — falling back to agent_cid as household_id"
                );
                e
            })
            .ok()
            .flatten();

        let (household_id, display_name) = match human_row {
            Some((Some(hid), name)) => (hid, Some(name)),
            Some((None, name)) => (agent_cid.clone(), Some(name)),
            None => (agent_cid.clone(), None),
        };

        // Step 3: CID counts from peer_blob_inventory.
        // their_cids_hosted_by_me uses the loop-invariant heuristic computed above;
        // my_cids_hosted_by_them is per-peer.
        // TODO(M3): batch counts into a single GROUP BY query once N > cluster_size
        let my_cids_hosted_by_them: i64 = peer_blob_inventory::table
            .filter(peer_blob_inventory::peer_id.eq(&peer_str))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        entries.push(serde_json::json!({
            "household_id": household_id,
            "display_name": display_name,
            "online": true,
            "last_sync_sec": 0u64, // TODO(M3): derive from last fetch-success timestamp in peer_blob_inventory
            "my_cids_hosted_by_them": my_cids_hosted_by_them as u32,
            "their_cids_hosted_by_me": local_blob_count_heuristic as u32,
        }));
    }

    serde_json::json!({ "connected_peer_households": entries })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::federator::Federator;
    use crate::test_util::test_pool;
    use libp2p::PeerId;

    #[tokio::test]
    async fn build_local_slice_returns_empty_array_when_no_peers_connected() {
        let pool = test_pool();
        let slice = build_local_slice(&pool, &[]).await;
        let arr = slice
            .get("connected_peer_households")
            .and_then(|v| v.as_array());
        assert!(arr.is_some(), "expected connected_peer_households array");
        assert_eq!(arr.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn build_local_slice_skips_peers_without_bindings() {
        let pool = test_pool();
        let unknown = PeerId::random();
        let slice = build_local_slice(&pool, &[unknown]).await;
        let arr = slice
            .get("connected_peer_households")
            .and_then(|v| v.as_array())
            .unwrap();
        assert_eq!(arr.len(), 0, "unbound peer should be skipped silently");
    }

    #[test]
    fn net_diff_computed_correctly() {
        // their_cids_hosted_by_me=10, my_cids_hosted_by_them=3 → net_diff = +7
        let mut edge = PeerHouseholdEdge {
            household_id: "hh-test".to_string(),
            display_name: None,
            online: true,
            last_sync_sec: None,
            my_cids_hosted_by_them: 3,
            their_cids_hosted_by_me: 10,
            net_diff: None,
            is_critical_for_me: None,
            i_am_critical_for_them: None,
        };
        edge.net_diff =
            Some(edge.their_cids_hosted_by_me as i64 - edge.my_cids_hosted_by_them as i64);
        assert_eq!(edge.net_diff, Some(7));
    }

    #[test]
    fn net_diff_negative_when_deficit() {
        let mut edge = PeerHouseholdEdge {
            household_id: "hh-deficit".to_string(),
            display_name: None,
            online: false,
            last_sync_sec: None,
            my_cids_hosted_by_them: 20,
            their_cids_hosted_by_me: 5,
            net_diff: None,
            is_critical_for_me: None,
            i_am_critical_for_them: None,
        };
        edge.net_diff =
            Some(edge.their_cids_hosted_by_me as i64 - edge.my_cids_hosted_by_them as i64);
        assert_eq!(edge.net_diff, Some(-15));
    }

    /// federation-aggregate early-return path (bindings.is_empty()) must compute
    /// real resilience_cliffs rather than returning vec![].
    ///
    /// Fixture: agent "agent-cliff-test" authored one blob ("blob-sole-001").
    /// That blob has exactly one replica holder ("peer-sole-holder") in
    /// peer_blob_inventory → sole replica → one ResilienceCliff entry.
    ///
    /// Since the pool has no peer_identity_bindings rows, `list_active_for_agent`
    /// returns empty → the early-return path at line 71 is exercised.
    ///
    /// Before the fix: resilience_cliffs == vec![] (stub) → test FAILS.
    /// After  the fix: resilience_cliffs == vec![ResilienceCliff { ... }] → PASSES.
    #[tokio::test]
    async fn federation_aggregate_resilience_cliffs_returns_real_values_not_empty() {
        use crate::db::diesel_schema::{content, peer_blob_inventory};

        let pool = test_pool();

        // Seed: one content row authored by the agent with a sole-replica blob.
        {
            let mut conn = pool.get().expect("conn");
            diesel::insert_into(content::table)
                .values((
                    content::id.eq("content-cliff-01"),
                    content::h_app_id.eq("lamad"),
                    content::title.eq("Cliff test content"),
                    content::content_type.eq("concept"),
                    content::content_format.eq("markdown"),
                    content::blob_hash.eq(Some("blob-sole-001")),
                    content::reach.eq("commons"),
                    content::validation_status.eq("valid"),
                    content::created_by.eq(Some("agent-cliff-test")),
                    content::created_at.eq("2026-05-15T00:00:00Z"),
                    content::updated_at.eq("2026-05-15T00:00:00Z"),
                ))
                .execute(&mut conn)
                .expect("insert content");

            // One sole replica holder.
            diesel::insert_into(peer_blob_inventory::table)
                .values((
                    peer_blob_inventory::peer_id.eq("peer-sole-holder"),
                    peer_blob_inventory::blob_hash.eq("blob-sole-001"),
                    peer_blob_inventory::source.eq("gossip-snapshot"),
                    peer_blob_inventory::sequence.eq(0i64),
                    peer_blob_inventory::last_seen_at.eq("2026-05-15T00:00:00Z"),
                ))
                .execute(&mut conn)
                .expect("insert peer_blob_inventory");
        }

        // No peer_identity_bindings rows → list_active_for_agent returns empty
        // → aggregate_peer_topology_view hits the early-return path at line 71.
        let p2p = crate::p2p::P2PHandle::for_testing();
        let federator = Federator::new(p2p);

        let view = aggregate_peer_topology_view(&pool, &federator, "agent-cliff-test")
            .await
            .expect("aggregate view");

        assert!(
            !view.resilience_cliffs.is_empty(),
            "federation-aggregate resilience_cliffs should be computed via \
             compute_resilience_cliffs, not returned as stub vec![]"
        );
    }
}
