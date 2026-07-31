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
    let hc_lamad = hc_registry.and_then(|r| r.lamad_client());

    match (&method, path) {
        // GET /api/v1/commitments
        (&Method::GET, "") => handle_list(req, pool, ctx).await,

        // POST /api/v1/commitments
        (&Method::POST, "") => handle_create(req, pool, ctx, services, hc_lamad.as_ref()).await,

        // POST /api/v1/commitments/capacity — explicit steward consent to
        // notarize a commons byte-budget pledge. Matched before the bare-id
        // arms so the capacity primitive stays a named protocol operation.
        (&Method::POST, "capacity") => {
            handle_create_capacity_pledge(req, pool, ctx, hc_lamad.as_ref()).await
        }

        // GET /api/v1/commitments/capacity/ratios — consent-legibility read:
        // exposes the server-side `constitutional_ratio_registry::effective_ratios()`
        // values a caller MUST see before it can construct a `ratioAttestation`
        // that will pass `replicates_commons_validator`'s exact-match donut
        // check. Category C operational config, no new DHT entity — no auth.
        (&Method::GET, "capacity/ratios") => handle_get_effective_ratios(),

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
            handle_update_state(req, id, pool, ctx, services, hc_lamad.as_ref()).await
        }

        _ => Ok(response::not_found(&format!(
            "Unknown commitments route: {} /api/v1/commitments/{}",
            method, path
        ))),
    }
}

/// GET /api/v1/commitments/capacity/ratios
///
/// Consent-legibility read for the capacity-pledge attestation: a caller
/// building a `POST /api/v1/commitments/capacity` body must attest ratio
/// values that EXACTLY match the server's
/// `constitutional_ratio_registry::effective_ratios()` (including the
/// `effective_ratio_cid == manifest_cid` provenance check —
/// `replicates_commons_validator::validate_typed_for_creation_commons`
/// stage 2(d)). Before that consent can be given it must be seen; this
/// route is what you must SEE, never a new notarized entity, so it stays
/// Category C operational config — unauthenticated, uncached-by-DHT,
/// derived from the same manifest-backed registry the write path enforces
/// against.
///
/// Response shape reuses the existing `CommonsRatioAttestation` ts-rs view
/// (already camelCase, already the exact shape a `ratioAttestation` needs):
/// `{"commonsPct","dwellingPct","collectivePct","freePct","effectiveRatioCid"}`.
fn handle_get_effective_ratios() -> Result<Response<Full<Bytes>>, StorageError> {
    let provenance = crate::services::constitutional_ratio_registry::effective_ratios();
    let ratios = provenance.ratios;
    let attestation = elohim_views::replicates_commons::CommonsRatioAttestation {
        commons_pct: ratios.commons_pct,
        dwelling_pct: ratios.dwelling_pct,
        collective_pct: ratios.collective_pct,
        free_pct: ratios.free_pct,
        effective_ratio_cid: provenance.manifest_cid,
    };
    Ok(response::ok(&attestation))
}

async fn handle_create_capacity_pledge(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
    hc_lamad: Option<&Arc<HcClient>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let payload: elohim_views::replicates_commons::ReplicatesContentPayload =
        parse_body(req).await?;
    let hc = hc_lamad.ok_or_else(|| {
        StorageError::Conductor(
            "lamad/mishpat bridge unavailable — capacity pledges require notarization".to_string(),
        )
    })?;
    let mut conn = get_conn(pool)?;
    Ok(from_create_result(
        crate::services::capacity_pledge_author::author_capacity_pledge(
            &mut conn, ctx, payload, hc,
        )
        .await,
    ))
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    /// GET /api/v1/commitments/capacity/ratios must echo the server's
    /// `effective_ratios()` verbatim — this is the consent-legibility read a
    /// caller needs before it can construct a `ratioAttestation` that will
    /// pass `replicates_commons_validator`'s exact-match donut check.
    #[tokio::test]
    async fn effective_ratios_response_matches_registry_and_sums_to_100() {
        let response = handle_get_effective_ratios().expect("handler must succeed");
        assert_eq!(response.status(), hyper::StatusCode::OK);

        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        let provenance = crate::services::constitutional_ratio_registry::effective_ratios();
        let ratios = provenance.ratios;

        assert_eq!(body["commonsPct"], ratios.commons_pct);
        assert_eq!(body["dwellingPct"], ratios.dwelling_pct);
        assert_eq!(body["collectivePct"], ratios.collective_pct);
        assert_eq!(body["freePct"], ratios.free_pct);
        assert_eq!(body["effectiveRatioCid"], provenance.manifest_cid);

        let cid = body["effectiveRatioCid"].as_str().unwrap();
        assert!(!cid.is_empty(), "effectiveRatioCid must be non-empty");

        let sum = body["commonsPct"].as_u64().unwrap()
            + body["dwellingPct"].as_u64().unwrap()
            + body["collectivePct"].as_u64().unwrap()
            + body["freePct"].as_u64().unwrap();
        assert_eq!(sum, 100, "ratio pcts must sum to 100, got {body:?}");
    }
}
