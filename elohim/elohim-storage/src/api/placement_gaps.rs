//! /api/v1/placement-gaps handler — structured shefa signal surface.
//!
//! Route: `GET /api/v1/placement-gaps`
//!
//! Serves the placement gaps projection as paged, filterable JSON.
//! Query params: `kind`, `contentId`, `limit`, `offset`.
//! Response: `{ items: PlacementGapView[], total: i32 }`
//!
//! Operational Category C — no DHT entry, projection only.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::Serialize;

use crate::db::{placement_gaps, AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::PlacementGapView;

use super::get_conn;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ListResponse {
    items: Vec<PlacementGapView>,
    total: i32,
}

pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    _resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    if method != Method::GET {
        return Ok(response::not_found(&format!(
            "Unknown placement-gaps method: {}",
            method
        )));
    }

    let query: std::collections::HashMap<String, String> = req
        .uri()
        .query()
        .map(|q| {
            url::form_urlencoded::parse(q.as_bytes())
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect()
        })
        .unwrap_or_default();

    let gap_q = placement_gaps::GapQuery {
        kind: query.get("kind").cloned(),
        content_id: query.get("contentId").cloned(),
        limit: query.get("limit").and_then(|s| s.parse().ok()),
        offset: query.get("offset").and_then(|s| s.parse().ok()),
    };

    let mut conn = get_conn(pool)?;
    let rows = placement_gaps::list_gaps(&mut conn, &ctx.h_app_id, gap_q)?;
    let items: Vec<PlacementGapView> = rows.into_iter().map(Into::into).collect();
    let total = items.len() as i32;

    Ok(response::ok(&ListResponse { items, total }))
}
