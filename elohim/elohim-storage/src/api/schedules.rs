//! Schedules API controller (Kairos temporal dimension)
//!
//! Routes: `/api/v1/schedules[/{id}]`
//!
//! No gate evaluation — schedules are metadata.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::schedules::{CreateScheduleInput, ScheduleQuery, UpdateScheduleInput};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::{CreateScheduleInputView, ScheduleView, UpdateScheduleInputView};

use super::{get_conn, parse_body};

// =============================================================================
// Main dispatcher
// =============================================================================

/// Handle `/api/v1/schedules*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    Ok(match (&method, path) {
        // POST /api/v1/schedules — create schedule
        (&Method::POST, "") => handle_create_schedule(req, pool, ctx).await,

        // GET /api/v1/schedules — list with query params
        (&Method::GET, "") => handle_list_schedules(req, pool, ctx).await,

        // POST /api/v1/schedules/{id}/advance — advance occurrence
        (&Method::POST, id) if id.ends_with("/advance") => {
            let schedule_id = id.strip_suffix("/advance").unwrap_or("");
            handle_advance_occurrence(schedule_id, pool, ctx).await
        }

        // GET /api/v1/schedules/{id} — get by ID
        (&Method::GET, id) if !id.is_empty() => handle_get_schedule(id, pool, ctx).await,

        // PATCH /api/v1/schedules/{id} — update schedule
        (&Method::PATCH, id) if !id.is_empty() => handle_update_schedule(req, id, pool, ctx).await,

        _ => response::not_found(&format!(
            "Unknown schedules route: {} /api/v1/schedules/{}",
            method, path
        )),
    })
}

// =============================================================================
// Handlers
// =============================================================================

async fn handle_create_schedule(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let input_view: CreateScheduleInputView = match parse_body(req).await {
        Ok(v) => v,
        Err(_) => return response::bad_request("Invalid JSON body for create schedule"),
    };

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    let input = CreateScheduleInput {
        entity_type: input_view.entity_type,
        entity_id: input_view.entity_id,
        scheduled_at: input_view.scheduled_at,
        expires_at: input_view.expires_at,
        rrule: input_view.rrule,
    };

    match crate::db::schedules::create_schedule(&mut conn, ctx, input) {
        Ok(schedule) => response::created(&ScheduleView::from(schedule)),
        Err(e) => response::error_response(e),
    }
}

async fn handle_list_schedules(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let query_str = req.uri().query().unwrap_or("");
    let query: ScheduleQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match crate::db::schedules::list_schedules(&mut conn, ctx, &query) {
        Ok(items) => {
            let views: Vec<ScheduleView> = items.into_iter().map(|s| s.into()).collect();
            response::ok(&views)
        }
        Err(e) => response::error_response(e),
    }
}

async fn handle_get_schedule(id: &str, pool: &DbPool, ctx: &AppContext) -> Response<Full<Bytes>> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match crate::db::schedules::get_schedule_by_id(&mut conn, ctx, id) {
        Ok(schedule) => response::ok(&ScheduleView::from(schedule)),
        Err(e) => response::error_response(e),
    }
}

async fn handle_update_schedule(
    req: Request<Incoming>,
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let input_view: UpdateScheduleInputView = match parse_body(req).await {
        Ok(v) => v,
        Err(_) => return response::bad_request("Invalid JSON body for update schedule"),
    };

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    // Convert flat Option<String> from view to Option<Option<String>> for PATCH semantics.
    // Present field = Some(Some(v)), absent field = None (no change).
    // To clear a field, client sends null → serde gives Some(None) at the view level,
    // but our view uses Option<String> so null → None. We treat None as "no change".
    let input = UpdateScheduleInput {
        scheduled_at: input_view.scheduled_at.map(Some),
        expires_at: input_view.expires_at.map(Some),
        rrule: input_view.rrule.map(Some),
    };

    match crate::db::schedules::update_schedule(&mut conn, ctx, id, input) {
        Ok(schedule) => response::ok(&ScheduleView::from(schedule)),
        Err(e) => response::error_response(e),
    }
}

async fn handle_advance_occurrence(
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match crate::db::schedules::advance_occurrence(&mut conn, ctx, id) {
        Ok(schedule) => response::ok(&ScheduleView::from(schedule)),
        Err(e) => response::error_response(e),
    }
}
