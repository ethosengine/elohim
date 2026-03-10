//! Agreements API controller
//!
//! Routes: `/api/v1/agreements[/{id}]`

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::agreements::AgreementQuery;
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::agreement_service::AgreementService;
use crate::services::response::{self, from_create_result, from_option, from_result};
use crate::views::CreateAgreementInputView;

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Route dispatcher
// ---------------------------------------------------------------------------

/// Handle `/api/v1/agreements*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // GET /api/v1/agreements
        (&Method::GET, "") => handle_list(req, pool, ctx).await,

        // POST /api/v1/agreements
        (&Method::POST, "") => handle_create(req, pool, ctx).await,

        // GET /api/v1/agreements/{id}
        (&Method::GET, id) if !id.contains('/') => handle_get_by_id(id, pool, ctx).await,

        _ => Ok(response::not_found(&format!(
            "Unknown agreements route: {} /api/v1/agreements/{}",
            method, path
        ))),
    }
}

// ---------------------------------------------------------------------------
// Handler implementations
// ---------------------------------------------------------------------------

async fn handle_list(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let query: AgreementQuery =
        serde_urlencoded::from_str(req.uri().query().unwrap_or("")).unwrap_or_default();
    let mut conn = get_conn(pool)?;
    Ok(from_result(AgreementService::list(&mut conn, ctx, &query)))
}

async fn handle_create(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input_view: CreateAgreementInputView = parse_body(req).await?;
    let input = input_view.into();
    let mut conn = get_conn(pool)?;
    Ok(from_create_result(AgreementService::create(
        &mut conn, ctx, input,
    )))
}

async fn handle_get_by_id(
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    Ok(from_option(
        AgreementService::get_by_id(&mut conn, ctx, id),
        &format!("Agreement not found: {}", id),
    ))
}
