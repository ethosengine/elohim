//! Places API controller
//!
//! Routes: `/api/v1/places[/{id}]`
//!
//! Governed spatial entities — storage projections of Mishpat DHT entries.
//! In production, Places are created via DHT post-commit signals.
//! This API provides direct creation for dev/testing and query access.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use uuid::Uuid;

use crate::db::models::{current_timestamp, NewPlace};
use crate::db::places::PlaceQuery;
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::{CreatePlaceInputView, PlaceView};

use super::{get_conn, parse_body};

/// Handle `/api/v1/places*` requests
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
        _ => response::not_found(&format!(
            "Unknown places route: {} /api/v1/places/{}",
            method, path
        )),
    })
}

async fn handle_create(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let v: CreatePlaceInputView = match parse_body(req).await {
        Ok(v) => v,
        Err(_) => return response::bad_request("Invalid JSON body for create place"),
    };

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    let now = current_timestamp();
    let id = Uuid::new_v4().to_string();
    // In dev/testing, use a placeholder DHT hash. In production, the projection
    // signal provides the real ActionHash.
    let dht_hash = format!("dev-placeholder-{}", &id[..8]);

    let new = NewPlace {
        id,
        app_id: ctx.app_id().to_string(),
        dht_anchor_hash: dht_hash,
        name: v.name,
        place_type: v.place_type,
        constitutional_layer: v.constitutional_layer,
        h3_index: v.h3_index,
        h3_resolution: v.h3_resolution,
        geometry_json: serde_json::to_string(&v.geometry_json).unwrap_or_default(),
        centroid_lat: v.centroid_lat,
        centroid_lng: v.centroid_lng,
        parent_place_id: v.parent_place_id,
        osm_reference_json: v
            .osm_reference
            .map(|j| serde_json::to_string(&j).unwrap_or_default()),
        carrying_capacity_json: v
            .carrying_capacity
            .map(|j| serde_json::to_string(&j).unwrap_or_default())
            .unwrap_or_else(|| "[]".to_string()),
        governing_collective_id: v.governing_collective_id,
        status: v.status.unwrap_or_else(|| "proposed".to_string()),
        created_by: "api".to_string(),
        created_at: now.clone(),
        updated_at: now,
        metadata_json: v
            .metadata
            .map(|j| serde_json::to_string(&j).unwrap_or_default())
            .unwrap_or_else(|| "{}".to_string()),
    };

    match crate::db::places::upsert_place(&mut conn, ctx, new) {
        Ok(place) => response::created(&PlaceView::from(place)),
        Err(e) => response::error_response(e),
    }
}

async fn handle_list(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let query_str = req.uri().query().unwrap_or("");
    let query: PlaceQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match crate::db::places::list_places(&mut conn, ctx, &query) {
        Ok(items) => {
            let views: Vec<PlaceView> = items.into_iter().map(|p| p.into()).collect();
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

    match crate::db::places::get_place_by_id(&mut conn, ctx, id) {
        Ok(place) => response::ok(&PlaceView::from(place)),
        Err(e) => response::error_response(e),
    }
}
