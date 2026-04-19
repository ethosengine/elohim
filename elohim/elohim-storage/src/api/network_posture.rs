//! Network posture aggregation controller.
//!
//! Route: `GET /api/v1/network/posture`
//!
//! Computes `NetworkPostureView` per-request from the current `peer_statuses`
//! projection (notarized PeerStatus DHT entries, infrastructure DNA) joined
//! lightly with `stewarded_nodes` for household reciprocation counts. No
//! persistence, no new DHT entry types — operational Category C projection.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::{peer_statuses, stewarded_nodes as nodes, AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::NetworkPostureView;

use super::get_conn;

/// Stale threshold: 120s since last update = stale (3 missed 60s heartbeats).
const STALE_THRESHOLD_SECS: i64 = 120;

pub async fn handle(
    _req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');
    Ok(match (&method, path) {
        (&Method::GET, "posture") => handle_get_posture(pool).await,
        _ => response::not_found(&format!(
            "Unknown network route: {} /api/v1/network/{}",
            method, path
        )),
    })
}

async fn handle_get_posture(pool: &DbPool) -> Response<Full<Bytes>> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(_) => return response::internal_error("pool unavailable"),
    };

    let peers = match peer_statuses::list_current(&mut conn) {
        Ok(p) => p,
        Err(e) => return response::internal_error(&format!("list_current: {e}")),
    };

    let now_secs = chrono::Utc::now().timestamp();
    let total_peers = peers.len() as i32;
    let mut active = 0i32;
    let mut stale = 0i32;
    let mut always_on = 0i32;
    let mut compute_available = false;

    for p in &peers {
        let age = now_secs - p.updated_at;
        let is_stale = age > STALE_THRESHOLD_SECS;
        let is_active = matches!(p.status.as_str(), "online" | "degraded") && !is_stale;

        if is_active {
            active += 1;
            if p.general_pool_member != 0 {
                always_on += 1;
            }
            if p.accepting_stewardship_reserves != 0 {
                compute_available = true;
            }
        }
        if is_stale {
            stale += 1;
        }
    }

    // households_reciprocating: distinct households among nodes with an active
    // peer status. If stewarded_nodes lacks household_id (pre-C3), this stays
    // zero — honest absence is better than a fake count.
    let households_reciprocating =
        match nodes::distinct_active_households(&mut conn, STALE_THRESHOLD_SECS, now_secs) {
            Ok(n) => n as i32,
            Err(_) => 0,
        };

    // storage_pressure: placeholder 0.0 until shard_locations populate.
    // Operational signal; real computation lands with the resilience proof
    // Sprint C (deferred from this sprint).
    let storage_pressure = 0.0_f32;

    response::ok(&NetworkPostureView {
        total_peers,
        active_peers: active,
        stale_peers: stale,
        always_on_peers: always_on,
        households_reciprocating,
        compute_available,
        storage_pressure,
        computed_at: chrono::Utc::now().to_rfc3339(),
    })
}
