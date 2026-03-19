//! Hazards API controller
//!
//! Routes: `/api/v1/hazards[/{id}]`
//!
//! Geospatial risk events attached to Places.
//! Category C (operational, SQLite-only). No DHT provenance.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use uuid::Uuid;

use crate::db::hazards::HazardQuery;
use crate::db::models::{current_timestamp, NewHazard};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::{CreateHazardInputView, HazardView};

use super::{get_conn, parse_body};

/// Handle `/api/v1/hazards*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    Ok(match (&method, path) {
        (&Method::POST, "") => handle_create(req, pool, ctx).await,
        (&Method::GET, "") => handle_list(req, pool, ctx).await,
        (&Method::GET, id) if !id.is_empty() => handle_get(id, pool, ctx).await,
        (&Method::PATCH, id) if !id.is_empty() => handle_update(req, id, pool, ctx).await,
        _ => response::not_found(&format!(
            "Unknown hazards route: {} /api/v1/hazards/{}",
            method, path
        )),
    })
}

async fn handle_create(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let v: CreateHazardInputView = match parse_body(req).await {
        Ok(v) => v,
        Err(_) => return response::bad_request("Invalid JSON body for create hazard"),
    };

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    let now = current_timestamp();
    let id = Uuid::new_v4().to_string();

    let affected_h3_cells = serde_json::to_string(&v.affected_h3_cells.unwrap_or_default())
        .unwrap_or_else(|_| "[]".to_string());

    let new = NewHazard {
        id,
        app_id: ctx.app_id().to_string(),
        place_id: v.place_id,
        hazard_type: v.hazard_type,
        severity: v.severity,
        title: v.title,
        description: v.description,
        reported_at: now.clone(),
        projected_onset: v.projected_onset,
        projected_end: v.projected_end,
        affected_h3_cells,
        radius_km: v.radius_km,
        source: v.source,
        source_reference: v.source_reference,
        metadata_json: v
            .metadata
            .map(|j| j.0.to_string())
            .unwrap_or_else(|| "{}".to_string()),
        status: "active".to_string(),
        created_at: now.clone(),
        updated_at: now,
    };

    match crate::db::hazards::create_hazard(&mut conn, ctx, new) {
        Ok(hazard) => response::created(&HazardView::from(hazard)),
        Err(e) => response::error_response(e),
    }
}

async fn handle_list(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let query_str = req.uri().query().unwrap_or("");
    let query: HazardQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match crate::db::hazards::list_hazards(&mut conn, ctx, &query) {
        Ok(items) => {
            let views: Vec<HazardView> = items.into_iter().map(HazardView::from).collect();
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

    match crate::db::hazards::get_hazard_by_id(&mut conn, ctx, id) {
        Ok(hazard) => response::ok(&HazardView::from(hazard)),
        Err(e) => response::error_response(e),
    }
}

/// Partial update — accepts status, severity, actual_onset, resolved_at
async fn handle_update(
    req: Request<Incoming>,
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let body: serde_json::Value = match parse_body(req).await {
        Ok(v) => v,
        Err(_) => return response::bad_request("Invalid JSON body for update hazard"),
    };

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    // Read the existing record, apply partial fields
    let existing = match crate::db::hazards::get_hazard_by_id(&mut conn, ctx, id) {
        Ok(h) => h,
        Err(e) => return response::error_response(e),
    };

    let new_status = body["status"]
        .as_str()
        .unwrap_or(&existing.status)
        .to_string();
    let resolved_at = body["resolvedAt"]
        .as_str()
        .map(|s| s.to_string())
        .or(existing.resolved_at);

    match crate::db::hazards::update_hazard_status(&mut conn, ctx, id, &new_status, resolved_at) {
        Ok(hazard) => response::ok(&HazardView::from(hazard)),
        Err(e) => response::error_response(e),
    }
}
