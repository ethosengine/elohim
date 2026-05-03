//! Cluster view API controller.
//!
//! Routes:
//!   - `GET /api/v1/cluster` — federated agent-scoped cluster view (Phase 5 T30).
//!     Auth context determines scope; future hub variants (`/cluster/household/{id}`,
//!     `/cluster/hub/{id}`) will extend the path tree.
//!
//! ## Source of Truth
//!
//! Operational (Category C). Federated query result. Bindings are notarized in
//! the imagodei DHT (Category A — `AgentPeerBinding`); per-device live state is
//! federated via `/elohim/view-federation/1.0.0`. The DHT remains canonical;
//! no SQLite table is authoritative for the composed view.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::DbPool;
use crate::error::StorageError;
use crate::services::cluster_view::aggregate_my_cluster_view;
use crate::services::federator::Federator;
use crate::services::response;

use super::{account, get_conn};

/// Dispatch `/api/v1/cluster` (no path tail in Phase 5 — only the agent-scoped
/// self-view). Future hub-scoped variants will live alongside.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    pool: &DbPool,
    p2p_handle: Option<&crate::p2p::P2PHandle>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    if method != Method::GET {
        return Ok(response::method_not_allowed());
    }
    handle_cluster_self(req, pool, p2p_handle).await
}

/// `GET /api/v1/cluster`
///
/// Identity tier wiring:
///   - Session Visitor: no `X-Agent-Cid` and no active local_session → 401
///   - Hosted Human / Steward via doorway / Steward via Tauri:
///     agent_cid resolved → federated `aggregate_my_cluster_view` call
///   - No P2P feature / P2P unavailable: 503 (federation requires the swarm)
async fn handle_cluster_self(
    req: Request<Incoming>,
    pool: &DbPool,
    p2p_handle: Option<&crate::p2p::P2PHandle>,
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

    let p2p = match p2p_handle {
        Some(h) => h,
        None => return Ok(response::service_unavailable("p2p_not_configured")),
    };

    let federator = Federator::new(p2p.clone());
    match aggregate_my_cluster_view(pool, &federator, &agent_cid).await {
        Ok(view) => Ok(response::ok(&view)),
        Err(e) => Ok(response::internal_error(&format!(
            "aggregate_my_cluster_view failed: {}",
            e
        ))),
    }
}
