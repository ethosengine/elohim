//! Reciprocity view API controller.
//!
//! Routes:
//!   - `GET /api/v1/reciprocity` — agent-scoped reciprocity ledger (Phase 5 T32).
//!     Composes net-flow per peer over the agent's REA commitment / economic-event
//!     stream. Auth context determines scope.
//!
//! ## Source of Truth
//!
//! Operational (Category C). Composes over notarized REA state:
//!   - `rea_commitments` — agreement-anchored commitments (Category A from
//!     imagodei/shefa REA notarization)
//!   - `economic_events` — fulfillment events (Category A)
//!
//! No new entity is introduced here. The DHT remains canonical; this view is a
//! read-optimized roll-up.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::{peer_identity_bindings, DbPool};
use crate::error::StorageError;
use crate::services::reciprocity_view::aggregate_reciprocity_view;
use crate::services::response;

use super::{account, get_conn};

/// Dispatch `/api/v1/reciprocity` (no path tail in Phase 5 — only the
/// agent-scoped self-view).
///
/// `graph_engine` is `None` when the `graph-native` feature is off or the engine
/// failed to open at startup. When `Some`, the graph-backed path is preferred.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    pool: &DbPool,
    graph_engine: Option<&std::sync::Arc<crate::graph::engine::GraphEngine>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    if method != Method::GET {
        return Ok(response::method_not_allowed());
    }
    handle_reciprocity_self(req, pool, graph_engine).await
}

/// `GET /api/v1/reciprocity`
///
/// Identity tier wiring:
///   - Session Visitor: no `X-Agent-Cid` and no active local_session → 401
///   - Hosted Human / Steward via doorway / Steward via Tauri:
///     agent_cid resolved → `aggregate_reciprocity_view` call
///
/// Note: reciprocity is a **local** SQLite roll-up over the steward's own REA
/// projection — no P2P federation required (each storage node owns its own
/// commitment/event tables). This is the only Phase 5 self-view that does not
/// gate on `p2p_handle`.
///
/// NOTE: graph-backed path returns graph-derived fields with placeholders
/// for system_metrics / blob_inventory / REA event fields. Full composition
/// lands in the follow-on sprint per Phase 5 architectural decision.
async fn handle_reciprocity_self(
    req: Request<Incoming>,
    pool: &DbPool,
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

    // graph-native branch: derive reciprocity from RECIPROCATES_WITH edges.
    #[cfg(feature = "graph-native")]
    if let Some(engine) = graph_engine {
        return match crate::graph_views::shefa::reciprocity::build(engine, &agent_cid) {
            Ok(view) => Ok(response::ok(&view)),
            Err(e) => Ok(response::internal_error(&format!(
                "graph reciprocity build failed: {e}"
            ))),
        };
    }

    // Legacy local relational path.
    let _ = graph_engine;
    let now_iso = chrono::Utc::now().to_rfc3339();
    let bindings = peer_identity_bindings::list_active_for_agent(&mut conn, &agent_cid, &now_iso)
        .map_err(|e| StorageError::Internal(format!("bindings lookup failed: {}", e)))?;

    // HTTP callers have no live P2P swarm — pass empty snapshot.
    // Per Phase 4 T10 design: online will be Some(false) for all rows,
    // which is correct semantics for cold-cache HTTP reads.
    let connected_peers = std::collections::HashSet::new();
    match aggregate_reciprocity_view(pool, &agent_cid, &bindings, &connected_peers).await {
        Ok(view) => Ok(response::ok(&view)),
        Err(e) => Ok(response::internal_error(&format!(
            "aggregate_reciprocity_view failed: {}",
            e
        ))),
    }
}
