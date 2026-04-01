//! Token API controller
//!
//! Routes:
//!   `GET  /api/v1/token/balance/{agent_id}`   — all balances for an agent
//!   `GET  /api/v1/token/mints/{agent_id}`     — mint history for an agent
//!   `GET  /api/v1/token/transfers/{agent_id}` — transfer history for an agent
//!   `POST /api/v1/token/transfer`             — create a peer-to-peer transfer
//!
//! Delegates to `TokenLedgerService` for balance/transfer logic and to
//! `db::token_mint_events` for mint history reads.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::{token_mint_events, AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response::{self, from_create_result, from_result};
use crate::services::token_ledger_service::TokenLedgerService;
use crate::views::{
    CreateTokenTransferInputView, TokenMintEventView,
};

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Route dispatcher
// ---------------------------------------------------------------------------

/// Handle `/api/v1/token*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // POST /api/v1/token/transfer
        (&Method::POST, "transfer") => handle_create_transfer(req, pool, ctx).await,

        // GET /api/v1/token/balance/{agent_id}
        (&Method::GET, p) if p.starts_with("balance/") => {
            let agent_id = p.trim_start_matches("balance/");
            handle_get_balances(agent_id, pool, ctx).await
        }

        // GET /api/v1/token/mints/{agent_id}
        (&Method::GET, p) if p.starts_with("mints/") => {
            let agent_id = p.trim_start_matches("mints/");
            handle_get_mints(agent_id, pool, ctx).await
        }

        // GET /api/v1/token/transfers/{agent_id}
        (&Method::GET, p) if p.starts_with("transfers/") => {
            let agent_id = p.trim_start_matches("transfers/");
            handle_get_transfers(agent_id, pool, ctx).await
        }

        _ => Ok(response::not_found(&format!(
            "Unknown token route: {} /api/v1/token/{}",
            method, path
        ))),
    }
}

// ---------------------------------------------------------------------------
// Handler implementations
// ---------------------------------------------------------------------------

/// GET /api/v1/token/balance/{agent_id}
///
/// Returns all balance rows for the agent across every governance layer.
async fn handle_get_balances(
    agent_id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    Ok(from_result(TokenLedgerService::get_all_balances(
        &mut conn, ctx, agent_id,
    )))
}

/// GET /api/v1/token/mints/{agent_id}
///
/// Returns mint history for the agent in descending chronological order.
async fn handle_get_mints(
    agent_id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    Ok(from_result(
        token_mint_events::get_mints_for_agent(&mut conn, ctx, agent_id)
            .map(|events| events.into_iter().map(TokenMintEventView::from).collect::<Vec<_>>()),
    ))
}

/// GET /api/v1/token/transfers/{agent_id}
///
/// Returns all transfers where the agent is either sender or receiver.
async fn handle_get_transfers(
    agent_id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    Ok(from_result(TokenLedgerService::get_transfers(
        &mut conn, ctx, agent_id,
    )))
}

/// POST /api/v1/token/transfer
///
/// Accepts a `CreateTokenTransferInputView` JSON body and executes an atomic
/// debit/credit between two agents in the specified governance layer.
async fn handle_create_transfer(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input: CreateTokenTransferInputView = parse_body(req).await?;
    let mut conn = get_conn(pool)?;
    Ok(from_create_result(TokenLedgerService::transfer(
        &mut conn,
        ctx,
        &input.from_agent,
        &input.to_agent,
        input.amount,
        &input.governance_layer,
        input.note.as_deref(),
    )))
}
