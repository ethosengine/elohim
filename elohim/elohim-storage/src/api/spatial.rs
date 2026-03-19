//! Spatial Context API controller
//!
//! Routes: `/api/v1/spatial-contexts[/{id}]`
//!
//! Polymorphic geospatial attachment — attach location to any entity.
//! Auto-computes H3 hexagonal cells from lat/lng on create/update.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::spatial_contexts::{
    CreateSpatialContextInput, SpatialContextQuery, UpdateSpatialContextInput,
};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::{
    CreateSpatialContextInputView, SpatialContextView, UpdateSpatialContextInputView,
};

use super::{get_conn, parse_body};

// =============================================================================
// Main dispatcher
// =============================================================================

/// Handle `/api/v1/spatial-contexts*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    Ok(match (&method, path) {
        // POST /api/v1/spatial-contexts — create
        (&Method::POST, "") => handle_create(req, pool, ctx).await,

        // GET /api/v1/spatial-contexts — list with query params (including H3 spatial queries)
        (&Method::GET, "") => handle_list(req, pool, ctx).await,

        // GET /api/v1/spatial-contexts/history/{entityType}/{entityId} — full history
        (&Method::GET, id) if id.starts_with("history/") => {
            let rest = id.strip_prefix("history/").unwrap_or("");
            if let Some((et, eid)) = rest.split_once('/') {
                handle_history(et, eid, pool, ctx).await
            } else {
                response::bad_request(
                    "Usage: /api/v1/spatial-contexts/history/{entityType}/{entityId}",
                )
            }
        }

        // GET /api/v1/spatial-contexts/{id} — get by ID
        (&Method::GET, id) if !id.is_empty() => handle_get(id, pool, ctx).await,

        // PATCH /api/v1/spatial-contexts/{id} — update (recomputes H3 if coords change)
        (&Method::PATCH, id) if !id.is_empty() => handle_update(req, id, pool, ctx).await,

        // DELETE /api/v1/spatial-contexts/{id} — delete
        (&Method::DELETE, id) if !id.is_empty() => handle_delete(id, pool, ctx).await,

        _ => response::not_found(&format!(
            "Unknown spatial route: {} /api/v1/spatial-contexts/{}",
            method, path
        )),
    })
}

// =============================================================================
// Handlers
// =============================================================================

async fn handle_create(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let v: CreateSpatialContextInputView = match parse_body(req).await {
        Ok(v) => v,
        Err(_) => return response::bad_request("Invalid JSON body for create spatial context"),
    };

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    let input = CreateSpatialContextInput {
        entity_type: v.entity_type,
        entity_id: v.entity_id,
        latitude: v.latitude,
        longitude: v.longitude,
        altitude: v.altitude,
        accuracy: v.accuracy,
        place_id: v.place_id,
        osm_type: v.osm_type,
        osm_id: v.osm_id,
        label: v.label,
        context_type: v.context_type,
        geometry_json: v
            .geometry_json
            .map(|j| serde_json::to_string(&j).unwrap_or_default()),
        metadata_json: v
            .metadata
            .map(|j| serde_json::to_string(&j).unwrap_or_default()),
        observed_at: v.observed_at,
    };

    match crate::db::spatial_contexts::create_spatial_context(&mut conn, ctx, input) {
        Ok(sc) => response::created(&SpatialContextView::from(sc)),
        Err(e) => response::error_response(e),
    }
}

async fn handle_list(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let query_str = req.uri().query().unwrap_or("");
    let query: SpatialContextQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match crate::db::spatial_contexts::list_spatial_contexts(&mut conn, ctx, &query) {
        Ok(items) => {
            let views: Vec<SpatialContextView> = items.into_iter().map(|s| s.into()).collect();
            response::ok(&views)
        }
        Err(e) => response::error_response(e),
    }
}

async fn handle_get(id: &str, pool: &DbPool, ctx: &AppContext) -> Response<Full<Bytes>> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match crate::db::spatial_contexts::get_spatial_context_by_id(&mut conn, ctx, id) {
        Ok(sc) => response::ok(&SpatialContextView::from(sc)),
        Err(e) => response::error_response(e),
    }
}

async fn handle_history(
    entity_type: &str,
    entity_id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match crate::db::spatial_contexts::get_spatial_context_history(
        &mut conn,
        ctx,
        entity_type,
        entity_id,
    ) {
        Ok(items) => {
            let views: Vec<SpatialContextView> = items.into_iter().map(|s| s.into()).collect();
            response::ok(&views)
        }
        Err(e) => response::error_response(e),
    }
}

async fn handle_update(
    req: Request<Incoming>,
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let v: UpdateSpatialContextInputView = match parse_body(req).await {
        Ok(v) => v,
        Err(_) => return response::bad_request("Invalid JSON body for update spatial context"),
    };

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    let input = UpdateSpatialContextInput {
        latitude: v.latitude.map(Some),
        longitude: v.longitude.map(Some),
        altitude: v.altitude.map(Some),
        accuracy: v.accuracy.map(Some),
        place_id: v.place_id.map(Some),
        label: v.label.map(Some),
        context_type: v.context_type,
        geometry_json: v
            .geometry_json
            .map(|j| Some(serde_json::to_string(&j).unwrap_or_default())),
        metadata_json: v
            .metadata
            .map(|j| Some(serde_json::to_string(&j).unwrap_or_default())),
        observed_at: v.observed_at.map(Some),
    };

    match crate::db::spatial_contexts::update_spatial_context(&mut conn, ctx, id, input) {
        Ok(sc) => response::ok(&SpatialContextView::from(sc)),
        Err(e) => response::error_response(e),
    }
}

async fn handle_delete(id: &str, pool: &DbPool, ctx: &AppContext) -> Response<Full<Bytes>> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match crate::db::spatial_contexts::delete_spatial_context(&mut conn, ctx, id) {
        Ok(()) => response::no_content(),
        Err(e) => response::error_response(e),
    }
}
