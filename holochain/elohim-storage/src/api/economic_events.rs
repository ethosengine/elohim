//! Economic Events API controller
//!
//! Routes: `/api/v1/economic-events[/{id}][/from-staged|/bulk|/agent/{id}|/content/{id}]`
//!
//! Delegates to `EconomicEventService` for business logic, which calls
//! `db::economic_events` for Diesel queries.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::Serialize;

use crate::db::economic_events::{CreateEconomicEventInput, EconomicEventQuery};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::economic_event_service::StagedTransaction;
use crate::services::response::{self, from_create_result, from_option, from_result};
use crate::services::EconomicEventService;
use crate::views::{CreateEconomicEventInputView, EconomicEventView};

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkCreateResponse {
    pub events: Vec<EconomicEventView>,
    pub submitted_count: u64,
    pub created_count: u64,
    pub skipped_count: u64,
}

// ---------------------------------------------------------------------------
// Route dispatcher
// ---------------------------------------------------------------------------

/// Handle `/api/v1/economic-events*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Normalize path: strip leading slash, split into segments
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // GET /api/v1/economic-events
        (&Method::GET, "") => handle_list(req, pool, ctx).await,

        // POST /api/v1/economic-events
        (&Method::POST, "") => handle_create(req, pool, ctx).await,

        // POST /api/v1/economic-events/bulk
        (&Method::POST, "bulk") => handle_bulk_create(req, pool, ctx).await,

        // POST /api/v1/economic-events/from-staged
        (&Method::POST, "from-staged") => handle_from_staged(req, pool, ctx).await,

        // GET /api/v1/economic-events/{id}  (must not match known sub-paths)
        (&Method::GET, id) if !id.contains('/') => handle_get_by_id(id, pool, ctx).await,

        // GET /api/v1/economic-events/agent/{agent_id}
        (&Method::GET, agent_path) if agent_path.starts_with("agent/") => {
            let agent_id = agent_path.trim_start_matches("agent/");
            handle_events_for_agent(req, agent_id, pool, ctx).await
        }

        // GET /api/v1/economic-events/content/{content_id}
        (&Method::GET, content_path) if content_path.starts_with("content/") => {
            let content_id = content_path.trim_start_matches("content/");
            handle_events_for_content(content_id, pool, ctx).await
        }

        _ => Ok(response::not_found(&format!(
            "Unknown economic-events route: {} /api/v1/economic-events/{}",
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
    let query = extract_query(req.uri().query().unwrap_or(""));
    let mut conn = get_conn(pool)?;
    Ok(from_result(EconomicEventService::list_events(
        &mut conn, ctx, &query,
    )))
}

async fn handle_get_by_id(
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    Ok(from_option(
        EconomicEventService::get_event(&mut conn, ctx, id),
        &format!("Economic event not found: {}", id),
    ))
}

async fn handle_create(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input_view: CreateEconomicEventInputView = parse_body(req).await?;
    let input: CreateEconomicEventInput = input_view.into();
    let mut conn = get_conn(pool)?;
    Ok(from_create_result(EconomicEventService::create_event(
        &mut conn, ctx, input,
    )))
}

async fn handle_bulk_create(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input_views: Vec<CreateEconomicEventInputView> = parse_body(req).await?;
    let inputs: Vec<CreateEconomicEventInput> = input_views.into_iter().map(|v| v.into()).collect();
    let mut conn = get_conn(pool)?;
    Ok(from_result(EconomicEventService::bulk_create_events(
        &mut conn, ctx, inputs,
    )))
}

async fn handle_from_staged(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Check if body is a single staged transaction or an array (bulk)
    use http_body_util::BodyExt;
    let body_bytes = req
        .into_body()
        .collect()
        .await
        .map_err(|e| StorageError::InvalidInput(format!("Failed to read body: {}", e)))?
        .to_bytes();

    // Try array first, fall back to single
    let is_array = body_bytes
        .iter()
        .find(|&&b| !b.is_ascii_whitespace())
        .map(|&b| b == b'[')
        .unwrap_or(false);

    let mut conn = get_conn(pool)?;

    if is_array {
        // Bulk from-staged
        let staged_list: Vec<StagedTransaction> = serde_json::from_slice(&body_bytes)
            .map_err(|e| StorageError::InvalidInput(format!("Invalid request body: {}", e)))?;

        let (events, submitted, skipped) =
            EconomicEventService::bulk_from_staged(&mut conn, ctx, staged_list)?;
        let created_count = events.len() as u64;
        let resp = BulkCreateResponse {
            submitted_count: submitted,
            created_count,
            skipped_count: skipped,
            events,
        };
        Ok(response::ok(&resp))
    } else {
        // Single from-staged
        let staged: StagedTransaction = serde_json::from_slice(&body_bytes)
            .map_err(|e| StorageError::InvalidInput(format!("Invalid request body: {}", e)))?;

        Ok(from_create_result(
            EconomicEventService::build_event_from_staged(&mut conn, ctx, &staged),
        ))
    }
}

async fn handle_events_for_agent(
    req: Request<Incoming>,
    agent_id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let limit = extract_limit(req.uri().query().unwrap_or(""));
    let mut conn = get_conn(pool)?;
    Ok(from_result(EconomicEventService::events_for_agent(
        &mut conn, ctx, agent_id, limit,
    )))
}

async fn handle_events_for_content(
    content_id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    Ok(from_result(EconomicEventService::events_for_content(
        &mut conn, ctx, content_id,
    )))
}

// ---------------------------------------------------------------------------
// Query parsing helpers
// ---------------------------------------------------------------------------

fn extract_query(query_str: &str) -> EconomicEventQuery {
    serde_urlencoded::from_str(query_str).unwrap_or_default()
}

fn extract_limit(query_str: &str) -> i64 {
    #[derive(serde::Deserialize, Default)]
    struct LimitQuery {
        limit: Option<i64>,
    }
    let q: LimitQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();
    q.limit.unwrap_or(100)
}
