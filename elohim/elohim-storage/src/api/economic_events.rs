//! Economic Events API controller
//!
//! Routes: `/api/v1/economic-events[/{id}][/from-staged|/bulk|/agent/{id}|/content/{id}]`
//!
//! Delegates to `EconomicEventService` for business logic, which calls
//! `db::economic_events` for Diesel queries.
//!
//! ## Collab share-routing (M1 Task 14 deepening)
//!
//! When `POST /api/v1/economic-events` carries a `scopeCollabCid` field,
//! `handle_create` performs async Collab+Agreement resolution at the handler
//! boundary (which is already async, per Hyper), then calls the pure
//! `route_economic_event_under_collab` evaluator and bulk-persists Settlement
//! events.  `EconomicEventService` itself remains sync throughout.

use std::collections::HashSet;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::Serialize;

use crate::db::economic_events::{CreateEconomicEventInput, EconomicEventQuery};
use crate::db::models::rea_actions;
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::hc_client_registry::HcClientRegistry;
use crate::services::economic_event_service::{
    route_economic_event_under_collab, StagedTransaction,
};
use crate::services::qahal_service::QahalService;
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
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Normalize path: strip leading slash, split into segments
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // GET /api/v1/economic-events
        (&Method::GET, "") => handle_list(req, pool, ctx).await,

        // POST /api/v1/economic-events
        (&Method::POST, "") => handle_create(req, pool, ctx, hc_registry).await,

        // POST /api/v1/economic-events/bulk
        (&Method::POST, "bulk") => handle_bulk_create(req, pool, ctx).await,

        // POST /api/v1/economic-events/from-staged
        (&Method::POST, "from-staged") => handle_from_staged(req, pool, ctx).await,

        // GET /api/v1/economic-events/{id}  (must not match known sub-paths)
        (&Method::GET, id) if !id.contains('/') && id != "appreciations" => {
            handle_get_by_id(id, pool, ctx).await
        }

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

        // GET /api/v1/economic-events/appreciations?for={id}&by={id}
        (&Method::GET, p) if p.starts_with("appreciations") => {
            handle_appreciations_query(req, pool, ctx).await
        }

        // POST /api/v1/economic-events/appreciations
        (&Method::POST, "appreciations") => handle_create_appreciation(req, pool, ctx).await,

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
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
    // Currently null for direct storage writes. Backfill needed for pre-coherence data.
    let input_view: CreateEconomicEventInputView = parse_body(req).await?;

    // If the event was authored under a Collab scope, route through share-allocation
    // BEFORE persisting.  The primary event records scope_collab_cid so downstream
    // queries can correlate Settlement events with their source Collab.
    if let Some(ref collab_cid) = input_view.scope_collab_cid.clone() {
        // Build QahalService from the registry — returns 503 if conductor offline.
        let svc = match build_qahal_service(hc_registry) {
            Ok(s) => s,
            Err(resp) => return Ok(resp),
        };

        // Fetch the Collab-Qahal to get the active member set.
        let collab = svc.fetch_collab_qahal(collab_cid).await.map_err(|e| {
            StorageError::Internal(format!("fetch_collab_qahal failed for {collab_cid}: {e}"))
        })?;

        // Resolve the anchor Agreement CID from the Collab-Qahal view.
        let agreement_cid = &collab.anchor_agreement_cid;
        if agreement_cid.is_empty() {
            return Err(StorageError::InvalidInput(format!(
                "Collab-Qahal {collab_cid} has no anchor_agreement_cid — cannot route"
            )));
        }

        // Fetch the CollabAgreement to get share_allocation.
        // Full Agreement entry decode is active (M1 final): fetch_agreement_by_action_bytes
        // now decodes the CollabAgreement entry body from the conductor Record and parses
        // share_allocation_json into a real ShareAllocation.
        let agreement = svc
            .fetch_collab_agreement(agreement_cid)
            .await
            .map_err(|e| {
                StorageError::Internal(format!(
                    "fetch_collab_agreement failed for {agreement_cid}: {e}"
                ))
            })?;

        // Defensive trust-boundary check: a properly-formed CollabAgreement always
        // carries declared shares (validated by the coordinator pre-commit gate).
        // This guard surfaces data-integrity violations rather than silently routing
        // with zero shares.
        if agreement.share_allocation.shares.is_none() {
            return Err(StorageError::InvalidInput(
                "agreement share_allocation.shares missing on Agreement entry — \
                 this should never happen with a properly-formed CollabAgreement; \
                 check coordinator pre-commit validation or DHT data integrity"
                    .into(),
            ));
        }

        // Active member set = non-withdrawn Collective CIDs from the Collab-Qahal.
        let active_set: HashSet<String> = collab
            .member_collectives
            .iter()
            .map(|c| c.cid.clone())
            .collect();

        let at_block_height = input_view.at_block_height;

        // Build the primary CreateEconomicEventInput (scope_collab_cid is threaded through).
        let primary_input: CreateEconomicEventInput = input_view.into();

        // Persist the primary event first — its ID is the source_event_id for Settlements.
        let mut conn = get_conn(pool)?;
        let primary = EconomicEventService::create_event(&mut conn, ctx, primary_input)
            .map_err(|e| StorageError::Internal(format!("primary event insert failed: {e}")))?;

        // Generate Settlement inputs via the pure evaluator.
        let settlement_inputs = route_economic_event_under_collab(
            &agreement,
            primary.resource_quantity_value.unwrap_or(0.0) as f64,
            at_block_height,
            &active_set,
            &primary.id,
        )?;

        // Bulk-persist Settlement events.
        let settlement_result =
            EconomicEventService::bulk_create_events(&mut conn, ctx, settlement_inputs)?;

        let resp_body = serde_json::json!({
            "primary": primary,
            "settlements": {
                "created": settlement_result.created,
                "errors": settlement_result.errors,
            }
        });
        return Ok(response::json_response(
            hyper::StatusCode::CREATED,
            &resp_body,
        ));
    }

    // Non-Collab path: single event, original behavior.
    let input: CreateEconomicEventInput = input_view.into();
    let mut conn = get_conn(pool)?;
    Ok(from_create_result(EconomicEventService::create_event(
        &mut conn, ctx, input,
    )))
}

// ---------------------------------------------------------------------------
// QahalService helper
// ---------------------------------------------------------------------------

/// Build a `QahalService` from the `HcClientRegistry`, returning a 503 response
/// if the registry or imagodei slot is absent.  Mirrors the pattern in
/// `api/qahal.rs::require_qahal_service` but returns a `StorageError`-safe
/// response rather than panicking.
#[allow(clippy::result_large_err)]
fn build_qahal_service(
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<QahalService, Response<Full<Bytes>>> {
    let registry = match hc_registry {
        Some(r) => r,
        None => return Err(response_503_qahal_offline()),
    };
    let hc = match registry.imagodei_client() {
        Some(h) => h,
        None => return Err(response_503_qahal_offline()),
    };
    Ok(QahalService::new(hc))
}

/// 503 body emitted when Collab share-routing is requested but the imagodei
/// conductor bridge is offline.
fn response_503_qahal_offline() -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "error": "imagodei bridge offline",
        "code": "IMAGODEI_BRIDGE_OFFLINE",
        "message": "scopeCollabCid was set but the imagodei conductor bridge is not \
                    connected. EconomicEvent Collab routing requires the imagodei DNA.",
    });
    response::json_response(hyper::StatusCode::SERVICE_UNAVAILABLE, &body)
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

/// GET /api/v1/economic-events/appreciations
///
/// Query params:
/// - `?for={id}` — appreciations directed at an entity (receiver = id)
/// - `?by={id}` — appreciations sent by an agent (provider = id)
///
/// Both params may be combined. action='appreciate' is always applied.
async fn handle_appreciations_query(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    #[derive(serde::Deserialize, Default)]
    struct AppreciationQueryParams {
        #[serde(rename = "for")]
        for_id: Option<String>,
        by: Option<String>,
        limit: Option<i64>,
        offset: Option<i64>,
    }

    let raw = req.uri().query().unwrap_or("");
    let params: AppreciationQueryParams = serde_urlencoded::from_str(raw).unwrap_or_default();

    let query = EconomicEventQuery {
        action: Some(rea_actions::APPRECIATE.to_string()),
        receiver: params.for_id,
        provider: params.by,
        limit: params.limit.unwrap_or(100),
        offset: params.offset.unwrap_or(0),
        ..Default::default()
    };

    let mut conn = get_conn(pool)?;
    Ok(from_result(EconomicEventService::list_events(
        &mut conn, ctx, &query,
    )))
}

/// POST /api/v1/economic-events/appreciations
///
/// Creates an economic event with action='appreciate'. The request body
/// follows `CreateEconomicEventInputView` — `action` is forced to 'appreciate'
/// server-side so callers may omit it.
async fn handle_create_appreciation(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut input_view: CreateEconomicEventInputView = parse_body(req).await?;
    // Force action regardless of what the client sent
    input_view.action = rea_actions::APPRECIATE.to_string();
    let input: CreateEconomicEventInput = input_view.into();
    let mut conn = get_conn(pool)?;
    Ok(from_create_result(EconomicEventService::create_event(
        &mut conn, ctx, input,
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
