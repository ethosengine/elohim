//! Projector status controller — `/api/v1/status/projector`.
//!
//! Route table:
//!   GET  /api/v1/status/projector  → current cursor + reconciliation-lag per (pillar, kind)
//!
//! The handler is a thin wrapper around
//! [`elohim_storage::projector::status::compute_projector_status`]; all logic
//! lives in that builder function, which is tested directly (no HTTP server
//! required in tests).

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::projector::mapping::ManifestRegistry;
use crate::projector::status::compute_projector_status;
use crate::services::response;

use super::get_conn;

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

/// Handle `/api/v1/status/projector*` requests.
pub async fn handle(
    _req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    _ctx: &AppContext,
    registry: &ManifestRegistry,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // GET /api/v1/status/projector
        (&Method::GET, "") => get_projector_status(pool, registry).await,

        _ => Ok(response::not_found(&format!(
            "Unknown projector status route: {} /api/v1/status/projector/{}",
            method, path
        ))),
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// `GET /api/v1/status/projector`
///
/// Returns `ProjectorStatusView` — per-(pillar, kind) cursor positions and
/// reconciliation-lag in seconds. See
/// [`compute_projector_status`] for the full lag semantics.
async fn get_projector_status(
    pool: &DbPool,
    registry: &ManifestRegistry,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let view = compute_projector_status(&mut conn, registry)
        .map_err(|e| StorageError::Internal(format!("projector status query failed: {e}")))?;
    Ok(response::ok(&view))
}
