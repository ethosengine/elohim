//! Comments API controller
//!
//! Routes: `/api/v1/comments[/{id}]`
//!
//! POST is gated through ElohimGate (MutationType::Comment).
//! GET endpoints are ungated.

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::comments::{CommentQuery, CreateCommentInput};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::elohim_gate::{GateResult, MutationType};
use crate::services::response;
use crate::services::Services;
use crate::views::{CommentView, CreateCommentInputView};

use super::{get_conn, parse_body};

// =============================================================================
// Main dispatcher
// =============================================================================

/// Handle `/api/v1/comments*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
    services: Option<Arc<Services>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    Ok(match (&method, path) {
        // POST /api/v1/comments — gated create
        (&Method::POST, "") => handle_create_comment(req, pool, ctx, services).await,

        // GET /api/v1/comments — list with query params
        (&Method::GET, "") => handle_list_comments(req, pool, ctx).await,

        // GET /api/v1/comments/{id}
        (&Method::GET, id) if !id.is_empty() => handle_get_comment(id, pool, ctx).await,

        _ => response::not_found(&format!(
            "Unknown comments route: {} /api/v1/comments/{}",
            method, path
        )),
    })
}

// =============================================================================
// Handlers
// =============================================================================

/// Compute reach level from composite trust score.
fn reach_from_trust(composite_trust: f64) -> &'static str {
    if composite_trust < 0.3 {
        "close"
    } else if composite_trust < 0.6 {
        "community"
    } else if composite_trust < 0.85 {
        "network"
    } else {
        "constitutional"
    }
}

async fn handle_create_comment(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
    services: Option<Arc<Services>>,
) -> Response<Full<Bytes>> {
    let input_view: CreateCommentInputView = match parse_body(req).await {
        Ok(v) => v,
        Err(_) => return response::bad_request("Invalid JSON body for create comment"),
    };

    if input_view.body.trim().is_empty() {
        return response::bad_request("body must not be empty");
    }

    // In v0, human_id defaults to "self" (no auth context yet)
    let human_id = "self".to_string();

    let (gate_result, gate_view) = super::evaluate_gate(
        &services,
        pool,
        ctx,
        MutationType::Comment,
        serde_json::json!({
            "contentId": input_view.content_id,
            "body": input_view.body,
        }),
        Some(&human_id),
    )
    .await;

    match &gate_result {
        GateResult::Pause {
            prompt,
            confirm_token,
            ..
        } => {
            return response::json_response(
                hyper::StatusCode::CONFLICT,
                &serde_json::json!({
                    "gate": gate_view,
                    "pausePrompt": prompt,
                    "confirmToken": confirm_token,
                }),
            );
        }
        GateResult::Settlement {
            boundary,
            appeal_path,
            ..
        } => {
            return response::forbidden(&serde_json::json!({
                "gate": gate_view,
                "boundary": boundary,
                "appealPath": appeal_path,
            }));
        }
        _ => {}
    }

    // Compute reach from trust context
    let reach = match &gate_view {
        Some(gv) => reach_from_trust(gv.trust_context.composite_trust),
        None => "community", // default when no gate evaluation
    };

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    let input = CreateCommentInput {
        content_id: input_view.content_id,
        human_id,
        body: input_view.body,
        reach: reach.to_string(),
    };

    match crate::db::comments::create_comment(&mut conn, ctx, &input) {
        Ok(comment) => response::created(&serde_json::json!({
            "data": CommentView::from(comment),
            "gate": gate_view,
        })),
        Err(e) => response::error_response(e),
    }
}

async fn handle_list_comments(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let query_str = req.uri().query().unwrap_or("");
    let query: CommentQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match crate::db::comments::list_comments(&mut conn, ctx, &query) {
        Ok(items) => {
            let views: Vec<CommentView> = items.into_iter().map(|c| c.into()).collect();
            response::ok(&views)
        }
        Err(e) => response::error_response(e),
    }
}

async fn handle_get_comment(id: &str, pool: &DbPool, ctx: &AppContext) -> Response<Full<Bytes>> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match crate::db::comments::get_comment(&mut conn, ctx, id) {
        Ok(comment) => response::ok(&CommentView::from(comment)),
        Err(e) => response::error_response(e),
    }
}
