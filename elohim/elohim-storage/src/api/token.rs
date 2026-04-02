//! Token API controller
//!
//! Routes:
//!   `GET  /api/v1/token/balance/{agent_id}`                         — all balances for an agent
//!   `GET  /api/v1/token/mints/{agent_id}`                           — mint history for an agent
//!   `GET  /api/v1/token/transfers/{agent_id}`                       — transfer history for an agent
//!   `POST /api/v1/token/transfer`                                   — create a peer-to-peer transfer
//!   `GET  /api/v1/token/config`                                     — list all demand curve configs
//!   `GET  /api/v1/token/config/{governance_layer}`                  — get config for a layer
//!   `POST /api/v1/token/config`                                     — create a demand curve config
//!   `GET  /api/v1/token/obligation/{agent_id}/{governance_layer}`   — evaluate agent obligation level
//!   `POST /api/v1/token/discernment-mint`                           — elohim Tier 2 discernment mint
//!   `POST /api/v1/token/apply-decay/{agent_id}/{governance_layer}`  — apply one decay period
//!   `GET  /api/v1/token/decay-history/{agent_id}`                   — decay audit log for an agent
//!   `GET  /api/v1/token/provenance/{agent_id}`                      — Merkle proof (individual layer)
//!   `GET  /api/v1/token/provenance/{agent_id}/{governance_layer}`   — Merkle proof for a layer
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
use crate::db::{
    responsibility_demand_configs, token_decay_events, token_mint_events, AppContext, DbPool,
};
use crate::error::StorageError;
use crate::services::provenance_service::ProvenanceService;
use crate::services::response::{self, from_create_result, from_option, from_result};
use crate::services::responsibility_demand_service::ResponsibilityDemandService;
use crate::services::token_decay_service::TokenDecayService;
use crate::services::token_ledger_service::TokenLedgerService;
use crate::services::token_mint_service::TokenMintService;
use crate::views::{
    CreateResponsibilityDemandConfigInputView, CreateTokenTransferInputView,
    DiscernmentMintInputView, ResponsibilityDemandConfigView, TokenDecayEventView,
    TokenMintEventView,
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

        // POST /api/v1/token/discernment-mint — elohim Tier 2 discernment mint
        (&Method::POST, "discernment-mint") => handle_discernment_mint(req, pool, ctx).await,

        // POST /api/v1/token/apply-decay/{agent_id}/{governance_layer} — apply one decay period
        (&Method::POST, p) if p.starts_with("apply-decay/") => {
            let rest = p.trim_start_matches("apply-decay/");
            // rest is "{agent_id}/{governance_layer}" — split on first '/'
            match rest.find('/') {
                Some(idx) => {
                    let agent_id = &rest[..idx];
                    let governance_layer = &rest[idx + 1..];
                    handle_apply_decay(agent_id, governance_layer, pool, ctx).await
                }
                None => Ok(response::bad_request(
                    "apply-decay route requires: /api/v1/token/apply-decay/{agent_id}/{governance_layer}",
                )),
            }
        }

        // GET /api/v1/token/decay-history/{agent_id} — decay audit log for an agent
        (&Method::GET, p) if p.starts_with("decay-history/") => {
            let agent_id = p.trim_start_matches("decay-history/");
            handle_get_decay_history(agent_id, pool, ctx).await
        }

        // GET /api/v1/token/provenance/{agent_id}/{governance_layer}
        // GET /api/v1/token/provenance/{agent_id}
        (&Method::GET, p) if p.starts_with("provenance/") => {
            let rest = p.trim_start_matches("provenance/");
            match rest.find('/') {
                Some(idx) => {
                    let agent_id = &rest[..idx];
                    let governance_layer = &rest[idx + 1..];
                    handle_get_provenance(agent_id, governance_layer, pool, ctx).await
                }
                None => {
                    // No slash — treat entire remainder as agent_id, default layer
                    handle_get_provenance(rest, "individual", pool, ctx).await
                }
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
        token_mint_events::get_mints_for_agent(&mut conn, ctx, agent_id).map(|events| {
            events
                .into_iter()
                .map(TokenMintEventView::from)
                .collect::<Vec<_>>()
        }),
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
        responsibility_demand_configs::get_all_configs(&mut conn, ctx).map(|configs| {
            configs
                .into_iter()
                .map(ResponsibilityDemandConfigView::from)
                .collect::<Vec<_>>()
        }),
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
        enforcement_active: if input.enforcement_active.unwrap_or(true) {
            1
        } else {
            0
        },
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

/// POST /api/v1/token/discernment-mint
///
/// Elohim Tier 2 minting for cross-domain patterns that no single REA event can
/// capture. Requires mandatory `elohim_attestation` and `reasoning_trace` fields
/// to form an auditable chain of custody for constitutional review.
///
/// The `governance_layer` field defaults to `"individual"` when absent.
/// The `source_epr_id` field is optional — when absent the mint is treated as
/// agent-level (no specific EPR reference).
async fn handle_discernment_mint(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input: DiscernmentMintInputView = parse_body(req).await?;
    let mut conn = get_conn(pool)?;

    let governance_layer = input.governance_layer.as_deref().unwrap_or("individual");

    Ok(from_create_result(TokenMintService::discernment_mint(
        &mut conn,
        ctx,
        &input.agent_id,
        governance_layer,
        input.amount,
        &input.elohim_attestation,
        &input.reasoning_trace,
        input.source_epr_id.as_deref(),
    )))
}

/// POST /api/v1/token/apply-decay/{agent_id}/{governance_layer}
///
/// Applies one decay period for an agent in a governance layer. The decay rate
/// scales with the agent's current obligation level. The dignity floor is always
/// protected. Returns a `DecayResult` describing what happened (including
/// early-exit cases where no decay was applied).
async fn handle_apply_decay(
    agent_id: &str,
    governance_layer: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    Ok(from_result(TokenDecayService::apply_decay(
        &mut conn,
        ctx,
        agent_id,
        governance_layer,
    )))
}

/// GET /api/v1/token/decay-history/{agent_id}
///
/// Returns the decay audit log for an agent across all governance layers,
/// ordered newest first. Each record shows the before/after balance, the decay
/// amount, and the obligation level that triggered the decay.
async fn handle_get_decay_history(
    agent_id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    Ok(from_result(
        token_decay_events::get_decay_events_for_agent(&mut conn, ctx, agent_id).map(|events| {
            events
                .into_iter()
                .map(TokenDecayEventView::from)
                .collect::<Vec<_>>()
        }),
    ))
}

/// GET /api/v1/token/provenance/{agent_id}/{governance_layer}
/// GET /api/v1/token/provenance/{agent_id}   (defaults governance_layer to "individual")
///
/// Returns a Merkle-tree provenance proof covering all mint events for the agent.
/// The `merkle_root` field is a hex-encoded SHA256 root over the set of mint event
/// leaf hashes, proving that every token was issued from a witnessed contribution.
///
/// When no mints exist the root is the all-zero 64-character string, event_count is 0,
/// and total_amount is 0 — this is a valid (empty) proof, not an error.
async fn handle_get_provenance(
    agent_id: &str,
    governance_layer: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    Ok(from_result(ProvenanceService::generate_proof(
        &mut conn,
        ctx,
        agent_id,
        governance_layer,
    )))
}
