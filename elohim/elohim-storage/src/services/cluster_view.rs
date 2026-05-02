//! Cluster view service — federated `MyClusterView` aggregator.
//!
//! ## Source of Truth
//!
//! Operational (Category C) per the p2p-design-gate output in
//! `genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-design.md`.
//! Federated query result. Bindings notarized in DHT (Category A); per-device
//! live state federated via `/elohim/view-federation/1.0.0`. The DHT remains canonical.
//! No SQLite table here is authoritative.

use std::time::Duration;

use chrono::Utc;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{Float, Nullable};
use thiserror::Error;

use crate::db::models::PeerIdentityBindingRow;
use crate::db::DbPool;
use crate::services::federator::{FederationResult, Federator};
use crate::views::{
    DeviceArchetype, DeviceSummary, DeviceTotals, Freshness, FreshnessState, MyClusterView,
    ViewKind,
};

const FEDERATION_TIMEOUT_MS: u64 = 3000;

#[derive(Debug, Error)]
pub enum ClusterViewError {
    #[error("db error: {0}")]
    Db(#[from] diesel::result::Error),
    #[error("storage error: {0}")]
    Storage(#[from] crate::error::StorageError),
    #[error("pool error: {0}")]
    Pool(String),
}

/// Federated `MyClusterView` aggregator.
///
/// Steps:
/// 1. Resolve `agent_cid → Vec<PeerIdentityBindingRow>` via
///    `db::peer_identity_bindings::list_active_for_agent`.
/// 2. Fan out via `Federator::query(ViewKind::Cluster, agent_cid, &bindings, timeout)`.
/// 3. For each `FederationResult`: build a `DeviceSummary` from the binding row + slice payload.
/// 4. Compose `DeviceTotals` from the live `DeviceSummary` rows + a sum over `rea_commitments`
///    (custody-blob commitments where this agent's peers are the provider).
/// 5. Roll up `freshness`: `AllOffline` if no device is `Live`; otherwise `Live`.
///    For an empty bindings list (no devices), default to `Live`.
pub async fn aggregate_my_cluster_view(
    pool: &DbPool,
    federator: &Federator,
    agent_cid: &str,
) -> Result<MyClusterView, ClusterViewError> {
    let now_iso = Utc::now().to_rfc3339();

    // 1) Resolve bindings.
    let bindings: Vec<PeerIdentityBindingRow> = {
        let mut conn = pool
            .get()
            .map_err(|e| ClusterViewError::Pool(e.to_string()))?;
        crate::db::peer_identity_bindings::list_active_for_agent(&mut conn, agent_cid, &now_iso)?
    };

    if bindings.is_empty() {
        return Ok(MyClusterView {
            agent_cid: agent_cid.to_string(),
            devices: vec![],
            totals: DeviceTotals {
                storage_used_bytes: 0,
                storage_total_bytes: 0,
                external_committed_bytes: 0,
                reciprocity_net_bytes: 0,
            },
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
            ViewKind::Cluster,
            agent_cid,
            &bindings,
            Duration::from_millis(timeout_ms),
        )
        .await;

    // 4) Build per-device summaries.
    let devices: Vec<DeviceSummary> = results
        .iter()
        .map(|r| device_summary_from_result(r, &bindings))
        .collect();

    // 5) Totals + freshness rollup.
    let totals = {
        let mut conn = pool
            .get()
            .map_err(|e| ClusterViewError::Pool(e.to_string()))?;
        compose_totals(&mut conn, &devices, &bindings)
    };

    let any_live = devices
        .iter()
        .any(|d| matches!(d.freshness.state, FreshnessState::Live));
    let freshness = if !any_live {
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

    Ok(MyClusterView {
        agent_cid: agent_cid.to_string(),
        devices,
        totals,
        freshness,
    })
}

/// Compose a `DeviceSummary` from a single peer's `FederationResult`.
fn device_summary_from_result(
    r: &FederationResult,
    bindings: &[PeerIdentityBindingRow],
) -> DeviceSummary {
    let archetype = bindings
        .iter()
        .find(|b| b.peer_id == r.peer_id)
        .map(|b| parse_archetype(&b.device_archetype))
        .unwrap_or(DeviceArchetype::Node);

    match (&r.freshness.state, &r.slice) {
        // Live + slice present → fill in fields from payload.
        (FreshnessState::Live, Some(slice)) => {
            let payload = &slice.payload.0;
            DeviceSummary {
                peer_id: r.peer_id.clone(),
                archetype,
                display_name: payload
                    .get("display_name")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                online: true,
                freshness: r.freshness.clone(),
                storage_used_bytes: payload.get("storage_used_bytes").and_then(|v| v.as_u64()),
                storage_total_bytes: payload.get("storage_total_bytes").and_then(|v| v.as_u64()),
                memory_used_bytes: payload.get("memory_used_bytes").and_then(|v| v.as_u64()),
                memory_total_bytes: payload.get("memory_total_bytes").and_then(|v| v.as_u64()),
                hosting_count: payload
                    .get("hosting_count")
                    .and_then(|v| v.as_u64())
                    .map(|x| x as u32),
                projecting_count: payload
                    .get("projecting_count")
                    .and_then(|v| v.as_u64())
                    .map(|x| x as u32),
                beacon_age_ms: payload.get("beacon_age_ms").and_then(|v| v.as_u64()),
            }
        }
        // Anything else → offline-shape with the freshness state preserved.
        _ => DeviceSummary {
            peer_id: r.peer_id.clone(),
            archetype,
            display_name: None,
            online: false,
            freshness: r.freshness.clone(),
            storage_used_bytes: None,
            storage_total_bytes: None,
            memory_used_bytes: None,
            memory_total_bytes: None,
            hosting_count: None,
            projecting_count: None,
            beacon_age_ms: None,
        },
    }
}

fn parse_archetype(s: &str) -> DeviceArchetype {
    match s {
        "desktop" => DeviceArchetype::Desktop,
        "mobile" => DeviceArchetype::Mobile,
        "steward" => DeviceArchetype::Steward,
        _ => DeviceArchetype::Node,
    }
}

/// Compose `DeviceTotals` from live device summaries + a SUM over `rea_commitments`
/// (provider-side custody-blob commitments by this agent's peers).
fn compose_totals(
    conn: &mut diesel::SqliteConnection,
    devices: &[DeviceSummary],
    bindings: &[PeerIdentityBindingRow],
) -> DeviceTotals {
    let storage_used_bytes: u64 = devices.iter().filter_map(|d| d.storage_used_bytes).sum();
    let storage_total_bytes: u64 = devices.iter().filter_map(|d| d.storage_total_bytes).sum();

    let my_peer_ids: Vec<&str> = bindings.iter().map(|b| b.peer_id.as_str()).collect();
    let external_committed_bytes = if my_peer_ids.is_empty() {
        0
    } else {
        use crate::db::diesel_schema::rea_commitments::dsl as rc;
        let outflow: Option<f32> = rc::rea_commitments
            .filter(rc::action.eq("custody-blob"))
            .filter(rc::provider.eq_any(&my_peer_ids))
            .select(sql::<Nullable<Float>>("SUM(resource_quantity_value)"))
            .first::<Option<f32>>(conn)
            .unwrap_or(None);
        outflow.unwrap_or(0.0).max(0.0) as u64
    };

    DeviceTotals {
        storage_used_bytes,
        storage_total_bytes,
        external_committed_bytes,
        // T27 wires the full reciprocity view; this stays 0 for T25.
        reciprocity_net_bytes: 0,
    }
}

/// Build the per-peer slice payload that this node returns when asked for its
/// own cluster slice via the F-T20 responder.
///
/// Returns a `serde_json::Value` shaped like:
/// ```json
/// {
///   "display_name": "Matthew's Desktop",
///   "storage_used_bytes": 0,
///   "storage_total_bytes": 0,
///   "hosting_count": 0,
///   "projecting_count": 0,
///   "beacon_age_ms": 0
/// }
/// ```
///
/// Real values for storage / hosting / projecting are stubbed in T25 (TODOs)
/// and replaced as those substrate sources land. T26 wires the F-T20 responder
/// to call this function (today the responder still returns the F-T20 stub).
pub async fn build_local_slice(_pool: &DbPool) -> serde_json::Value {
    // TODO(Phase 4 follow-up): wire real values from system metrics + db counts.
    serde_json::json!({
        "display_name": std::env::var("ELOHIM_DISPLAY_NAME").unwrap_or_default(),
        "storage_used_bytes": 0u64,
        "storage_total_bytes": 0u64,
        "hosting_count": 0u32,
        "projecting_count": 0u32,
        "beacon_age_ms": 0u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_archetype_known() {
        assert_eq!(parse_archetype("desktop"), DeviceArchetype::Desktop);
        assert_eq!(parse_archetype("mobile"), DeviceArchetype::Mobile);
        assert_eq!(parse_archetype("node"), DeviceArchetype::Node);
        assert_eq!(parse_archetype("steward"), DeviceArchetype::Steward);
    }

    #[test]
    fn parse_archetype_unknown_defaults_to_node() {
        assert_eq!(parse_archetype("server"), DeviceArchetype::Node);
        assert_eq!(parse_archetype(""), DeviceArchetype::Node);
    }
}
