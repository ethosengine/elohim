//! Recognition Pipeline API controller
//!
//! Routes: `/api/v1/recognition[/distribute]`
//!
//! Delegates to `recognition_pipeline_service` for business logic.

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::elohim_gate::{GateResult, MutationType};
use crate::services::recognition_pipeline_service;
use crate::services::response;
use crate::services::Services;
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
    services: Option<Arc<Services>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // POST /api/v1/recognition/distribute
        (&Method::POST, "distribute") => handle_distribute(req, pool, ctx, services).await,

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
    services: Option<Arc<Services>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input: RecognitionTriggerInputView = parse_body(req).await?;

    let (gate_result, gate_view) = super::evaluate_gate(
        &services,
        MutationType::ReachChange,
        serde_json::json!({
            "contentId": input.content_id,
            "eventType": input.event_type,
            "rawAmount": input.raw_amount,
        }),
    )
    .await;

    match &gate_result {
        GateResult::Pause {
            prompt,
            confirm_token,
            ..
        } => {
            return Ok(response::json_response(
                hyper::StatusCode::CONFLICT,
                &serde_json::json!({
                    "gate": gate_view,
                    "pausePrompt": prompt,
                    "confirmToken": confirm_token,
                }),
            ));
        }
        GateResult::Settlement {
            boundary,
            appeal_path,
            ..
        } => {
            return Ok(response::forbidden(&serde_json::json!({
                "gate": gate_view,
                "boundary": boundary,
                "appealPath": appeal_path,
            })));
        }
        _ => {}
    }

    let trigger = input.into();
    let mut conn = get_conn(pool)?;
    let result = recognition_pipeline_service::distribute(&mut conn, ctx, trigger)?;
    let view = RecognitionDistributionResultView::from(result);
    Ok(response::created(&serde_json::json!({
        "data": view,
        "gate": gate_view,
    })))
}
