//! Node-shape publish + household-devices read API controllers.
//!
//! Routes:
//! - `POST /api/v1/nodes/shape` — Elohim-node publishes its durable shape
//!   (hostname, archetype, committed resources, household binding) at boot.
//!   Storage upserts into `stewarded_nodes`. The DHT commit via the
//!   `register_node_shape` coordinator zome function happens on the node's
//!   own agent key (Task C7 elohim-node boot path); post-commit signal then
//!   fills in `dht_anchor_hash` on the stewarded_nodes projection row.
//!   Source of truth: `NodeRegistration` DHT entry (node-registry DNA,
//!   existing entry type — no new entry types introduced).
//!
//! - `GET /api/v1/households/{id}/devices` — Returns the operational join of
//!   `stewarded_nodes` LEFT JOIN `peer_statuses` for a household, shaped as
//!   `HouseholdDevicesView`. Category C computed projection, no persistence.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::{stewarded_nodes as nodes_db, AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::{DeviceEntryView, HouseholdDevicesView, NodeShapeView, PeerStatusView};

use super::{get_conn, parse_body};

/// Dispatch for `/api/v1/nodes/*` — currently just `POST /nodes/shape`.
pub async fn handle_nodes(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');
    Ok(match (&method, path) {
        (&Method::POST, "shape") => handle_post_shape(req, pool).await,
        _ => response::not_found(&format!(
            "Unknown nodes route: {} /api/v1/nodes/{}",
            method, path
        )),
    })
}

/// Dispatch for `/api/v1/households/*` — currently just `GET /{id}/devices`.
pub async fn handle_households(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');
    let _ = req;

    // Parse `{id}/devices` — the household id may contain any URL-safe chars.
    if let Some(id) = path.strip_suffix("/devices") {
        if method == Method::GET && !id.is_empty() {
            return Ok(handle_get_household_devices(id, pool).await);
        }
    }

    Ok(response::not_found(&format!(
        "Unknown households route: {} /api/v1/households/{}",
        method, path
    )))
}

async fn handle_post_shape(req: Request<Incoming>, pool: &DbPool) -> Response<Full<Bytes>> {
    let shape: NodeShapeView = match parse_body(req).await {
        Ok(v) => v,
        Err(e) => return response::bad_request(&format!("parse: {e}")),
    };

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(_) => return response::internal_error("pool unavailable"),
    };

    match nodes_db::upsert_from_shape(&mut conn, &shape) {
        Ok(row) => response::ok(&serde_json::json!({
            "nodeId": row.id,
            "dhtAnchorHash": row.dht_anchor_hash,
            "stored": true,
        })),
        Err(e) => response::internal_error(&format!("upsert_from_shape: {e}")),
    }
}

async fn handle_get_household_devices(household_id: &str, pool: &DbPool) -> Response<Full<Bytes>> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(_) => return response::internal_error("pool unavailable"),
    };

    let rows = match nodes_db::list_by_household_with_peer_status(&mut conn, household_id) {
        Ok(r) => r,
        Err(e) => return response::internal_error(&format!("{e}")),
    };

    let devices: Vec<DeviceEntryView> = rows
        .into_iter()
        .map(|(node, peer)| {
            let shape = stewarded_node_to_shape_view(&node);
            let peer_view: Option<PeerStatusView> = peer.map(Into::into);
            DeviceEntryView {
                shape,
                peer: peer_view,
            }
        })
        .collect();

    response::ok(&HouseholdDevicesView {
        household_id: household_id.to_string(),
        devices,
    })
}

/// Project a `StewardedNode` projection row back onto the wire `NodeShapeView`.
///
/// Mirror of the upsert in `upsert_from_shape`. Columns added by the
/// archetype migration (Task C3) are all nullable on the row but required
/// on the view — missing fields fall back to safe defaults so legacy rows
/// (pre-C3) still render without panicking.
fn stewarded_node_to_shape_view(n: &crate::db::models::StewardedNode) -> NodeShapeView {
    use crate::views::CommittedResources;
    NodeShapeView {
        node_id: n.id.clone(),
        hostname: n.hostname.clone().unwrap_or_else(|| n.display_name.clone()),
        device_archetype_id: n.device_archetype_id.clone().unwrap_or_default(),
        household_id: n.household_id.clone().unwrap_or_default(),
        role: n.node_role.clone().unwrap_or_else(|| "edge".into()),
        capability_level: n.capability_level.unwrap_or(0),
        committed: CommittedResources {
            cpu_cores: n.cpu_cores,
            memory_gb: n.memory_gb,
            storage_tb: n.storage_tb,
            bandwidth_mbps: Some(n.bandwidth_mbps),
            max_custody_gb: None,
            can_steward: n.can_steward != 0,
            can_infer: n.can_infer != 0,
            can_doorway: n.can_doorway != 0,
        },
        steward_tier: Some(n.steward_tier.clone()),
        custodian_opt_in: n.custodian_opt_in != 0,
        region: n.region.clone(),
        signature: n.signature.clone().unwrap_or_default(),
        signed_at: n.signed_at.clone().unwrap_or_else(|| n.updated_at.clone()),
        dht_anchor_hash: n.dht_anchor_hash.clone(),
    }
}
