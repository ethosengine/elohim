//! REA Commitments API controller
//!
//! Routes: `/api/v1/commitments[/{id}][/agent/{id}]`

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::rea_commitments::{
    CreateReaCommitmentInput, ReaCommitmentQuery, UpdateReaCommitmentState,
};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::rea_commitment_service::ReaCommitmentService;
use crate::services::response::{self, from_create_result, from_option, from_result};

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Route dispatcher
// ---------------------------------------------------------------------------

/// Handle `/api/v1/commitments*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // GET /api/v1/commitments
        (&Method::GET, "") => handle_list(req, pool, ctx).await,

        // POST /api/v1/commitments
        (&Method::POST, "") => handle_create(req, pool, ctx).await,

        // GET /api/v1/commitments/agent/{agent_id}
        (&Method::GET, agent_path) if agent_path.starts_with("agent/") => {
            let agent_id = agent_path.trim_start_matches("agent/");
            handle_get_by_agent(req, agent_id, pool, ctx).await
        }

        // GET /api/v1/commitments/{id}
        (&Method::GET, id) if !id.contains('/') => handle_get_by_id(id, pool, ctx).await,

        // PATCH /api/v1/commitments/{id}
        (&Method::PATCH, id) if !id.contains('/') => handle_update_state(req, id, pool, ctx).await,

        _ => Ok(response::not_found(&format!(
            "Unknown commitments route: {} /api/v1/commitments/{}",
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
    let query: ReaCommitmentQuery =
        serde_urlencoded::from_str(req.uri().query().unwrap_or("")).unwrap_or_default();
    let mut conn = get_conn(pool)?;
    Ok(from_result(ReaCommitmentService::list(
        &mut conn, ctx, &query,
    )))
}

async fn handle_create(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input: CreateReaCommitmentInput = parse_body(req).await?;
    let mut conn = get_conn(pool)?;
    Ok(from_create_result(ReaCommitmentService::create(
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
        ReaCommitmentService::get_by_id(&mut conn, ctx, id),
        &format!("Commitment not found: {}", id),
    ))
}

async fn handle_update_state(
    req: Request<Incoming>,
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let update: UpdateReaCommitmentState = parse_body(req).await?;
    let mut conn = get_conn(pool)?;
    Ok(from_result(ReaCommitmentService::update_state(
        &mut conn, ctx, id, &update,
    )))
}

async fn handle_get_by_agent(
    req: Request<Incoming>,
    agent_id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let limit = extract_limit(req.uri().query().unwrap_or(""));
    let mut conn = get_conn(pool)?;
    Ok(from_result(ReaCommitmentService::get_by_agent(
        &mut conn, ctx, agent_id, limit,
    )))
}

// ---------------------------------------------------------------------------
// Query parsing helpers
// ---------------------------------------------------------------------------

fn extract_limit(query_str: &str) -> i64 {
    #[derive(serde::Deserialize, Default)]
    struct LimitQuery {
        limit: Option<i64>,
    }
    let q: LimitQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();
    q.limit.unwrap_or(100)
}
