//! Peer-topology view API controller.
//!
//! Routes:
//!   - `GET /api/v1/peer-topology` — federated agent-scoped peer-household
//!     topology (Phase 5 T31).
//!     Auth context determines scope; future hub variants (`/peer-topology/hub/{id}`)
//!     will extend the path tree alongside this self-view.
//!
//! ## Source of Truth
//!
//! Operational (Category C). Federated query result. Per-peer bindings are
//! notarized in the imagodei DHT (Category A — `AgentPeerBinding`); per-peer
//! live state and connected-household edges are federated via
//! `/elohim/view-federation/1.0.0`. The DHT remains canonical; no SQLite table
//! is authoritative for the composed view.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::DbPool;
use crate::error::StorageError;
use crate::services::federator::Federator;
use crate::services::peer_topology_view::aggregate_peer_topology_view;
use crate::services::response;

use super::{account, get_conn};

/// Dispatch `/api/v1/peer-topology` (no path tail in Phase 5 — only the
/// agent-scoped self-view). Future hub-scoped variants will live alongside.
///
/// `graph_engine` is `None` when the `graph-native` feature is off or the engine
/// failed to open at startup. When `Some`, the graph-backed path is preferred.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    pool: &DbPool,
    p2p_handle: Option<&crate::p2p::P2PHandle>,
    graph_engine: Option<&std::sync::Arc<crate::graph::engine::GraphEngine>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    if method != Method::GET {
        return Ok(response::method_not_allowed());
    }
    handle_peer_topology_self(req, pool, p2p_handle, graph_engine).await
}

/// `GET /api/v1/peer-topology`
///
/// Identity tier wiring (mirrors `/cluster`):
///   - Session Visitor: no `X-Agent-Cid` and no active local_session → 401
///   - Hosted Human / Steward via doorway / Steward via Tauri:
///     agent_cid resolved → federated `aggregate_peer_topology_view` call
///   - No P2P feature / P2P unavailable: 503 (federation requires the swarm)
///
/// NOTE: graph-backed path returns graph-derived fields with placeholders
/// for system_metrics / blob_inventory / REA event fields. Full composition
/// lands in the follow-on sprint per Phase 5 architectural decision.
async fn handle_peer_topology_self(
    req: Request<Incoming>,
    pool: &DbPool,
    p2p_handle: Option<&crate::p2p::P2PHandle>,
    graph_engine: Option<&std::sync::Arc<crate::graph::engine::GraphEngine>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;

    let agent_cid = match account::extract_agent_cid(&req, &mut conn)? {
        Some(cid) => cid,
        None => {
            return Ok(response::json_response(
                hyper::StatusCode::UNAUTHORIZED,
                &serde_json::json!({ "reason": "auth_required" }),
            ));
        }
    };

    // graph-native branch: derive topology from RECIPROCATES_WITH edges.
    #[cfg(feature = "graph-native")]
    if let Some(engine) = graph_engine {
        return match crate::graph_views::shefa::peer_topology::build(engine, &agent_cid) {
            Ok(view) => Ok(response::ok(&view)),
            Err(e) => Ok(response::internal_error(&format!(
                "graph peer_topology build failed: {e}"
            ))),
        };
    }

    // Legacy federated path.
    let _ = graph_engine;
    let p2p = match p2p_handle {
        Some(h) => h,
        None => return Ok(response::service_unavailable("p2p_not_configured")),
    };

    let federator = Federator::new(p2p.clone());
    match aggregate_peer_topology_view(pool, &federator, &agent_cid).await {
        Ok(view) => Ok(response::ok(&view)),
        Err(e) => Ok(response::internal_error(&format!(
            "aggregate_peer_topology_view failed: {}",
            e
        ))),
    }
}
