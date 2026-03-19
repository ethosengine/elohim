//! Weather API controller
//!
//! Routes:
//! - `GET /api/v1/weather/{placeId}` — get weather forecast for a Place

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;

use super::get_conn;

/// Handle `/api/v1/weather*` requests
pub async fn handle(
    _req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    Ok(match (&method, path) {
        (&Method::GET, place_id) if !place_id.is_empty() => {
            handle_forecast(place_id, pool, ctx).await
        }
        _ => response::not_found(&format!(
            "Unknown weather route: {} /api/v1/weather/{}",
            method, path
        )),
    })
}

async fn handle_forecast(place_id: &str, pool: &DbPool, ctx: &AppContext) -> Response<Full<Bytes>> {
    // Look up the Place to get its coordinates
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    let place = match crate::db::places::get_place_by_id(&mut conn, ctx, place_id) {
        Ok(p) => p,
        Err(e) => return response::error_response(e),
    };

    let forecast =
        crate::services::weather::get_forecast(place_id, place.centroid_lat, place.centroid_lng)
            .await;
    response::ok(&forecast)
}
