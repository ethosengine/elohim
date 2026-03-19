//! Spatial Dashboard API controller (Sprint 8 — Planet-Scale Governance Dashboard)
//!
//! Routes: `/api/v1/dashboard/spatial[?...]`
//!
//! Read-only aggregate endpoint. Composes places, hazards, alerts,
//! vulnerability, and weather into a single response. No mutations.
//!
//! Query parameters (all optional):
//! - `constitutionalLayer` — filter by governance layer
//! - `h3Index` — filter by H3 cell index
//! - `parentPlaceId` — filter by parent place
//! - `status` — place lifecycle status (default: "active")
//! - `include` — comma-separated enrichment layers (default: all)
//!   values: capacity, hazards, vulnerability, weather, alerts, routes
//! - `limit` — max places returned (default: 200)

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::services::spatial_dashboard::{build_dashboard, DashboardQuery};

// =============================================================================
// Main dispatcher
// =============================================================================

/// Handle `/api/v1/dashboard*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    Ok(match (&method, path) {
        // GET /api/v1/dashboard/spatial
        (&Method::GET, "spatial") | (&Method::GET, "spatial/") | (&Method::GET, "") => {
            handle_spatial_dashboard(req, pool, ctx).await
        }

        _ => response::not_found(&format!(
            "Unknown dashboard route: {} /api/v1/dashboard/{}",
            method, path
        )),
    })
}

// =============================================================================
// Handlers
// =============================================================================

async fn handle_spatial_dashboard(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let query_str = req.uri().query().unwrap_or("");
    let query: DashboardQuery = match serde_urlencoded::from_str(query_str) {
        Ok(q) => q,
        Err(e) => {
            return response::bad_request(&format!("Invalid query parameters: {}", e));
        }
    };

    match build_dashboard(pool, ctx, &query).await {
        Ok(dashboard) => response::ok(&dashboard),
        Err(e) => response::error_response(e),
    }
}
