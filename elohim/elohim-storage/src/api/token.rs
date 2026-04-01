//! Token API controller
//!
//! Routes:
//!   `GET  /api/v1/token/balance/{agent_id}`                     — all balances for an agent
//!   `GET  /api/v1/token/mints/{agent_id}`                       — mint history for an agent
//!   `GET  /api/v1/token/transfers/{agent_id}`                   — transfer history for an agent
//!   `POST /api/v1/token/transfer`                               — create a peer-to-peer transfer
//!   `GET  /api/v1/token/config`                                 — list all demand curve configs
//!   `GET  /api/v1/token/config/{governance_layer}`              — get config for a layer
//!   `POST /api/v1/token/config`                                 — create a demand curve config
//!   `GET  /api/v1/token/obligation/{agent_id}/{governance_layer}` — evaluate agent obligation level
//!
//! Delegates to `TokenLedgerService` for balance/transfer logic, to
//! `db::token_mint_events` for mint history reads, to
//! `db::responsibility_demand_configs` for config CRUD, and to
//! `ResponsibilityDemandService` for obligation evaluation.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use uuid::Uuid;

use crate::db::models::NewResponsibilityDemandConfig;
use crate::db::{responsibility_demand_configs, token_mint_events, AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response::{self, from_create_result, from_option, from_result};
use crate::services::responsibility_demand_service::ResponsibilityDemandService;
use crate::services::token_ledger_service::TokenLedgerService;
use crate::views::{
    CreateResponsibilityDemandConfigInputView, CreateTokenTransferInputView,
    ResponsibilityDemandConfigView, TokenMintEventView,
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

        // GET /api/v1/token/config — list all demand curve configs
        (&Method::GET, "config") => handle_list_configs(pool, ctx).await,

        // POST /api/v1/token/config — create a demand curve config
        (&Method::POST, "config") => handle_create_config(req, pool, ctx).await,

        // GET /api/v1/token/config/{governance_layer} — get config for a layer
        (&Method::GET, p) if p.starts_with("config/") => {
            let governance_layer = p.trim_start_matches("config/");
            handle_get_config(governance_layer, pool, ctx).await
        }

        // GET /api/v1/token/obligation/{agent_id}/{governance_layer} — evaluate obligation
        (&Method::GET, p) if p.starts_with("obligation/") => {
            let rest = p.trim_start_matches("obligation/");
            // rest is "{agent_id}/{governance_layer}" — split on first '/'
            match rest.find('/') {
                Some(idx) => {
                    let agent_id = &rest[..idx];
                    let governance_layer = &rest[idx + 1..];
                    handle_get_obligation(agent_id, governance_layer, pool, ctx).await
                }
                None => Ok(response::bad_request(
                    "obligation route requires: /api/v1/token/obligation/{agent_id}/{governance_layer}",
                )),
            }
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

/// GET /api/v1/token/config
///
/// Returns all responsibility demand curve configs for this app, ordered by
/// governance layer name.
async fn handle_list_configs(
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    Ok(from_result(
        responsibility_demand_configs::get_all_configs(&mut conn, ctx)
            .map(|configs| configs.into_iter().map(ResponsibilityDemandConfigView::from).collect::<Vec<_>>()),
    ))
}

/// GET /api/v1/token/config/{governance_layer}
///
/// Returns the demand curve config for the specified governance layer, or 404
/// if no config has been set for that layer yet.
async fn handle_get_config(
    governance_layer: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    Ok(from_option(
        responsibility_demand_configs::get_config_for_layer(&mut conn, ctx, governance_layer)
            .map(|opt| opt.map(ResponsibilityDemandConfigView::from)),
        &format!("No config found for governance layer: {}", governance_layer),
    ))
}

/// POST /api/v1/token/config
///
/// Creates a new responsibility demand curve config. Applies protocol defaults
/// for any optional curve parameters not provided:
/// - dignity_floor: 100.0
/// - median_estimate: 1000.0
/// - soft_ceiling_multiplier: 10.0
/// - hard_ceiling_multiplier: 20.0
/// - social_contract_health: 0.5
/// - enforcement_active: true
async fn handle_create_config(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input: CreateResponsibilityDemandConfigInputView = parse_body(req).await?;
    let mut conn = get_conn(pool)?;

    let id = Uuid::new_v4().to_string();
    let new = NewResponsibilityDemandConfig {
        id: id.as_str(),
        h_app_id: ctx.h_app_id.as_str(),
        governance_layer: input.governance_layer.as_str(),
        dignity_floor: input.dignity_floor.unwrap_or(100.0),
        median_estimate: input.median_estimate.unwrap_or(1000.0),
        soft_ceiling_multiplier: input.soft_ceiling_multiplier.unwrap_or(10.0),
        hard_ceiling_multiplier: input.hard_ceiling_multiplier.unwrap_or(20.0),
        social_contract_health: input.social_contract_health.unwrap_or(0.5),
        enforcement_active: if input.enforcement_active.unwrap_or(true) { 1 } else { 0 },
        ratified_by: None,
        ratified_at: None,
        dht_anchor_hash: None,
    };

    Ok(from_create_result(
        responsibility_demand_configs::create_config(&mut conn, ctx, new)
            .map(ResponsibilityDemandConfigView::from),
    ))
}

/// GET /api/v1/token/obligation/{agent_id}/{governance_layer}
///
/// Evaluates and returns the agent's obligation level for the specified
/// governance layer. This is the primary consumer-facing query for "where am I
/// on the responsibility demand curve?".
///
/// Returns `Normal` when no config exists or enforcement is inactive, so this
/// route always succeeds (never 404) — the absence of config has a defined
/// meaning.
async fn handle_get_obligation(
    agent_id: &str,
    governance_layer: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    Ok(from_result(ResponsibilityDemandService::evaluate(
        &mut conn,
        ctx,
        agent_id,
        governance_layer,
    )))
}
