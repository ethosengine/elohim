//! REA Commitments API controller
//!
//! Routes: `/api/v1/commitments[/{id}][/agent/{id}]`

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use std::sync::Arc;

use crate::db::rea_commitments::{
    CreateReaCommitmentInput, ReaCommitmentQuery, UpdateReaCommitmentState,
};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::hc_client_registry::HcClientRegistry;
use crate::services::rea_commitment_service::ReaCommitmentService;
use crate::services::response::{self, from_create_result, from_option, from_result};
use crate::services::Services;

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
    services: Option<Arc<Services>>,
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');
    let hc_lamad = hc_registry.and_then(|r| r.lamad.as_ref());

    match (&method, path) {
        // GET /api/v1/commitments
        (&Method::GET, "") => handle_list(req, pool, ctx).await,

        // POST /api/v1/commitments
        (&Method::POST, "") => handle_create(req, pool, ctx, services, hc_lamad).await,

        // GET /api/v1/commitments/agent/{agent_id}
        (&Method::GET, agent_path) if agent_path.starts_with("agent/") => {
            let agent_id = agent_path.trim_start_matches("agent/");
            handle_get_by_agent(req, agent_id, pool, ctx).await
        }

        // GET /api/v1/commitments/facing/rea — REA economic facing per-commitment
        // read surface over the mishpat compute/recovery-class ledger (Wave 4.2).
        // `facing/rea` contains '/', so it can never be shadowed by the
        // `(GET, id) if !id.contains('/')` arm below — but it is matched
        // explicitly here for clarity and is the route-shadow guard registered
        // in the http.rs manifest (asserted by test_manifest_builds).
        (&Method::GET, "facing/rea") => handle_facing_rea(pool).await,

        // GET /api/v1/commitments/{id}
        (&Method::GET, id) if !id.contains('/') => handle_get_by_id(id, pool, ctx).await,

        // PATCH /api/v1/commitments/{id}
        (&Method::PATCH, id) if !id.contains('/') => {
            handle_update_state(req, id, pool, ctx, services, hc_lamad).await
        }

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
    services: Option<Arc<Services>>,
    hc_lamad: Option<&Arc<HcClient>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // dht_anchor_hash is now populated end-to-end for project-epr commitments
    // via the conductor-first write path in ReaCommitmentService::create
    // (substrate-rea-replication-fix plan, Task 4). Other actions still take
    // the legacy diesel-direct path.
    let raw: serde_json::Value = parse_body(req).await?;
    let input = normalize_create_input(raw)?;
    let mut conn = get_conn(pool)?;
    let events = services.as_ref().map(|s| s.events.as_ref());
    Ok(from_create_result(
        ReaCommitmentService::create(&mut conn, ctx, input, events, hc_lamad).await,
    ))
}

/// Normalize a create body into the DB-layer input, honoring BOTH live wire
/// shapes (shift honest-held 2026-06-06 — the route previously parsed the
/// DB-layer struct directly, silently swallowing the canonical
/// `CreateReaCommitmentInputView` fields: `metadata` objects were dropped,
/// `resourceQuantity` Measure objects were dropped, and plain-string
/// `resourceClassifiedAs` was stored un-JSON-encoded so the output view's
/// parse-as-array nulled it on every read):
///
/// - **View shape** (canonical, `CreateReaCommitmentInputView` — seeder, a2o
///   storage-client): `metadata` object, `resourceQuantity`/`effortQuantity`
///   `{hasNumericalValue, hasUnit}`, `resourceClassifiedAs`/`inScopeOf` arrays.
/// - **Legacy flat shape** (older steps + seed-projections): `metadataJson`
///   string, `resourceQuantityValue`/`resourceQuantityUnit`, plain strings for
///   classifiedAs/inScopeOf.
///
/// Strategy: pre-wrap plain-string array fields, parse the view (all-default
/// fields tolerate legacy bodies), then merge legacy keys for anything the
/// view shape didn't carry. List-ish columns are stored as JSON-array strings
/// so they round-trip through `ReaCommitmentView`'s parse.
fn normalize_create_input(
    mut raw: serde_json::Value,
) -> Result<CreateReaCommitmentInput, StorageError> {
    use elohim_views::shefa::CreateReaCommitmentInputView;

    // Pre-normalize: plain string → one-element array (both fields are
    // array-typed in the view and array-JSON in storage).
    for key in ["resourceClassifiedAs", "inScopeOf"] {
        if let Some(s) = raw.get(key).and_then(|v| v.as_str()).map(String::from) {
            raw[key] = serde_json::Value::Array(vec![serde_json::Value::String(s)]);
        }
    }

    let view: CreateReaCommitmentInputView = serde_json::from_value(raw.clone())
        .map_err(|e| StorageError::InvalidInput(format!("Invalid commitment body: {e}")))?;

    // The canonical conversion already lives in views_convert/inputs.rs —
    // the bug was that this route never used it. Delegate, then merge the
    // legacy flat keys for anything the view shape didn't carry (view fields
    // win; seed-projections sends both forms and the object form is
    // authoritative there too).
    let mut input: CreateReaCommitmentInput = view.into();
    if input.metadata_json.is_none() {
        input.metadata_json = raw
            .get("metadataJson")
            .and_then(|v| v.as_str())
            .map(String::from);
    }
    if input.resource_quantity_value.is_none() {
        input.resource_quantity_value = raw
            .get("resourceQuantityValue")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32);
        input.resource_quantity_unit = raw
            .get("resourceQuantityUnit")
            .and_then(|v| v.as_str())
            .map(String::from);
    }
    if input.effort_quantity_value.is_none() {
        input.effort_quantity_value = raw
            .get("effortQuantityValue")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32);
        input.effort_quantity_unit = raw
            .get("effortQuantityUnit")
            .and_then(|v| v.as_str())
            .map(String::from);
    }
    Ok(input)
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

/// GET /api/v1/commitments/facing/rea — REA economic facing per-commitment read
/// surface over the mishpat compute/recovery-class commitment ledger (Wave 4.2).
///
/// Viewer-less + node-scoped (no auth, no app-id), mirroring the operational-weave
/// `build_weave_view` dispatch: the lens reflects the whole node's commitment
/// ledger, not a single app or agent. Read-only over an already-notarized ledger
/// (Operational Category C — no new DHT entry type, no POST; the write path lives
/// on the existing mishpat coordinator). Returns `Vec<MishpatCommitmentView>`.
async fn handle_facing_rea(pool: &DbPool) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let views =
        crate::services::mishpat_commitment_facing::build_mishpat_commitment_view(&mut conn);
    Ok(response::ok(&views))
}

async fn handle_update_state(
    req: Request<Incoming>,
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
    services: Option<Arc<Services>>,
    hc_lamad: Option<&Arc<HcClient>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Accept both the canonical view shape (`metadata` object — see
    // UpdateReaCommitmentStateView) and the legacy flat `metadataJson` string.
    let raw: serde_json::Value = parse_body(req).await?;
    let view: elohim_views::shefa::UpdateReaCommitmentStateView =
        serde_json::from_value(raw.clone())
            .map_err(|e| StorageError::InvalidInput(format!("Invalid state update body: {e}")))?;
    let update = UpdateReaCommitmentState {
        state: view.state,
        finished: view.finished,
        metadata_json: view
            .metadata
            .as_ref()
            .and_then(|m| serde_json::to_string(m).ok())
            .or_else(|| {
                raw.get("metadataJson")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            }),
    };
    let mut conn = get_conn(pool)?;
    let events = services.as_ref().map(|s| s.events.as_ref());
    Ok(from_result(
        ReaCommitmentService::update_state(&mut conn, ctx, id, &update, events, hc_lamad).await,
    ))
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
