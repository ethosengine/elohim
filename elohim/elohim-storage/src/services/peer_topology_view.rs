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

use crate::db::models::PeerIdentityBindingRow;
use crate::db::DbPool;
use crate::services::federator::Federator;
use crate::views::{Freshness, FreshnessState, PeerHouseholdEdge, PeerTopologyView, ViewKind};

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
/// 6. `resilience_cliffs` stubbed `vec![]` — TODO Phase 4 follow-up.
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
        return Ok(PeerTopologyView {
            agent_cid: agent_cid.to_string(),
            edges: vec![],
            reciprocation_count: 0,
            resilience_cliffs: vec![],
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

    // 7) TODO(Phase 4 follow-up): compute resilience_cliffs from sole-replica
    // analysis once the quilt distribution layer surfaces per-CID replica sets.
    let resilience_cliffs = vec![];

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

/// Build the per-peer slice payload that this node returns when asked for its
/// own peer topology slice via the F-T20 responder.
///
/// Returns a `serde_json::Value` with an empty `connected_peer_households` array.
///
/// TODO(Phase 4 follow-up): populate from libp2p's connected-peer set joined
/// with each peer's binding's `household_id`. Real CID counts come from the
/// quilt distribution layer once it surfaces per-peer hosted-CID sets.
pub async fn build_local_slice(_pool: &DbPool) -> serde_json::Value {
    serde_json::json!({ "connected_peer_households": [] })
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
