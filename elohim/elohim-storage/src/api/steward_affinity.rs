//! Steward Affinity API controller
//!
//! Routes: `/api/v1/steward-affinity[/*]`

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::steward_affinity::{self, AffinityQuery};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::{
    BulkCreateStewardAffinityInputView, CreateStewardAffinityInputView, StewardAffinityView,
};

use super::{get_conn, parse_body};

/// Handle `/api/v1/steward-affinity*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        (&Method::GET, "") => handle_list(req, pool, ctx).await,
        (&Method::POST, "") => handle_create(req, pool, ctx).await,
        (&Method::POST, "bulk") => handle_bulk_create(req, pool, ctx).await,
        (&Method::GET, id) if !id.contains('/') => handle_get_by_id(id, pool, ctx).await,

        _ => Ok(response::not_found(&format!(
            "Unknown steward-affinity route: {} /api/v1/steward-affinity/{}",
            method, path
        ))),
    }
}

async fn handle_list(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let query: AffinityQuery =
        serde_urlencoded::from_str(req.uri().query().unwrap_or("")).unwrap_or_default();
    let mut conn = get_conn(pool)?;
    let affinities = steward_affinity::list_affinities(&mut conn, ctx, &query)?;
    let views: Vec<StewardAffinityView> = affinities.into_iter().map(Into::into).collect();
    Ok(response::ok(&views))
}

async fn handle_create(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input: CreateStewardAffinityInputView = parse_body(req).await?;
    let db_input = input.into();
    let mut conn = get_conn(pool)?;
    let affinity = steward_affinity::create_affinity(&mut conn, ctx, &db_input)?;
    let view = StewardAffinityView::from(affinity);
    Ok(response::created(&view))
}

async fn handle_bulk_create(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input: BulkCreateStewardAffinityInputView = parse_body(req).await?;
    let db_inputs: Vec<_> = input.affinities.into_iter().map(Into::into).collect();
    let mut conn = get_conn(pool)?;
    let (created, errors) = steward_affinity::bulk_create_affinities(&mut conn, ctx, &db_inputs)?;
    Ok(response::ok(&serde_json::json!({
        "created": created,
        "errors": errors,
    })))
}

async fn handle_get_by_id(
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let affinity = steward_affinity::get_affinity_by_id(&mut conn, ctx, id)?;
    let view = StewardAffinityView::from(affinity);
    Ok(response::ok(&view))
}
