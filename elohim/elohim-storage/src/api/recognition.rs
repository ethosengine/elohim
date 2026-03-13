//! Recognition Pipeline API controller
//!
//! Routes: `/api/v1/recognition[/distribute]`
//!
//! Delegates to `recognition_pipeline_service` for business logic.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::recognition_pipeline_service;
use crate::services::response;
use crate::views::{RecognitionDistributionResultView, RecognitionTriggerInputView};

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Route dispatcher
// ---------------------------------------------------------------------------

/// Handle `/api/v1/recognition*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // POST /api/v1/recognition/distribute
        (&Method::POST, "distribute") => handle_distribute(req, pool, ctx).await,

        _ => Ok(response::not_found(&format!(
            "Unknown recognition route: {} /api/v1/recognition/{}",
            method, path
        ))),
    }
}

// ---------------------------------------------------------------------------
// Handler implementations
// ---------------------------------------------------------------------------

async fn handle_distribute(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input: RecognitionTriggerInputView = parse_body(req).await?;
    let trigger = input.into();
    let mut conn = get_conn(pool)?;
    let result = recognition_pipeline_service::distribute(&mut conn, ctx, trigger)?;
    let view = RecognitionDistributionResultView::from(result);
    Ok(response::created(&view))
}
