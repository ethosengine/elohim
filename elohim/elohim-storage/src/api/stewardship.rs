//! Stewardship API controller
//!
//! Routes: `/api/v1/stewardship[/grants|/appeals|/policy|/activity|/allocations]`
//!
//! Delegates to `StewardshipService` for business logic, which calls
//! `db::stewardship_allocations` and `db::policy_cache` for Diesel queries.

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::db::stewardship_allocations::AllocationQuery;
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::elohim_gate::{GateResult, MutationType};
use crate::services::response;
use crate::services::Services;
use crate::services::StewardshipService;
use crate::views::{
    ContentStewardshipView, CreateAllocationInputView, DevicePolicyView, StewardshipAllocationView,
    UpdateAllocationInputView, UpsertPolicyInputView,
};

use super::{get_conn, parse_body};

// =============================================================================
// Request types (controller-local, not exported to TS)
// =============================================================================

/// Request to create a stewardship grant (acknowledged-only, data lives in Holochain zome)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct CreateGrantRequest {
    pub subject_id: String,
    pub authority_basis: String,
    #[serde(default)]
    pub evidence_hash: Option<String>,
    #[serde(default)]
    pub verified_by: Option<String>,
    #[serde(default)]
    pub content_filtering: bool,
    #[serde(default)]
    pub time_limits: bool,
    #[serde(default)]
    pub feature_restrictions: bool,
    #[serde(default)]
    pub activity_monitoring: bool,
    #[serde(default)]
    pub policy_delegation: bool,
    #[serde(default)]
    pub delegatable: bool,
    #[serde(default = "default_30u32")]
    pub expires_in_days: u32,
    #[serde(default = "default_30u32")]
    pub review_in_days: u32,
}

fn default_30u32() -> u32 {
    30
}

/// Request to delegate a grant to another steward (acknowledged-only)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct DelegateGrantRequest {
    pub parent_grant_id: String,
    pub new_steward_id: String,
    #[serde(default)]
    pub content_filtering: Option<bool>,
    #[serde(default)]
    pub time_limits: Option<bool>,
    #[serde(default)]
    pub feature_restrictions: Option<bool>,
    #[serde(default)]
    pub activity_monitoring: Option<bool>,
    #[serde(default)]
    pub policy_delegation: Option<bool>,
    #[serde(default = "default_30u32")]
    pub expires_in_days: u32,
}

/// Request to file an appeal (acknowledged-only)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct FileAppealRequest {
    pub grant_id: String,
    #[serde(default)]
    pub policy_id: Option<String>,
    pub appeal_type: String,
    pub grounds: Vec<String>,
    #[serde(default)]
    pub evidence_json: Option<String>,
    #[serde(default)]
    pub advocate_id: Option<String>,
}

/// Request to log session activity (acknowledged-only)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct LogActivityRequest {
    pub session_id: String,
    pub session_duration_minutes: u32,
    #[serde(default)]
    pub categories_accessed: Vec<String>,
    #[serde(default)]
    pub policy_events: Vec<Value>,
}

/// Request to file a dispute on an allocation
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileDisputeRequest {
    pub disputed_by: String,
    pub reason: String,
}

/// Request to resolve a dispute
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResolveDisputeRequest {
    pub ratifier_id: String,
    #[serde(default = "default_active")]
    pub new_state: String,
}

fn default_active() -> String {
    "active".to_string()
}

// =============================================================================
// Response types
// =============================================================================

/// Minimal "acknowledged" response for grant/appeal/activity endpoints
/// where the underlying data lives in the Holochain zome, not local SQLite.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AcknowledgedResponse {
    pub acknowledged: bool,
    pub message: String,
}

// =============================================================================
// Main dispatcher
// =============================================================================

/// Handle `/api/v1/stewardship*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
    services: Option<Arc<Services>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // resource_path arrives without the "stewardship" prefix:
    //   ""                          → /api/v1/stewardship
    //   "/policy"                   → /api/v1/stewardship/policy
    //   "/grants"                   → /api/v1/stewardship/grants
    //   "/allocations"              → /api/v1/stewardship/allocations
    //   "/allocations/{id}"         → /api/v1/stewardship/allocations/{id}
    //   "/allocations/{id}/dispute" → …/dispute
    // etc.

    let path = resource_path.trim_start_matches('/');

    Ok(match (&method, path) {
        // -----------------------------------------------------------------
        // Policy check
        // -----------------------------------------------------------------
        (&Method::GET, "policy") => handle_get_policy(req, pool, ctx).await,

        // -----------------------------------------------------------------
        // Grants (acknowledged only — data lives in Holochain zome)
        // -----------------------------------------------------------------
        (&Method::GET, "grants") => handle_list_grants(req).await,

        (&Method::POST, "grants") => handle_create_grant(req).await,

        (&Method::GET, p) if p.starts_with("grants/") && !p.ends_with("/delegate") => {
            let id = p.trim_start_matches("grants/");
            handle_get_grant(id).await
        }

        (&Method::POST, p) if p.starts_with("grants/") && p.ends_with("/delegate") => {
            handle_delegate_grant(req).await
        }

        // -----------------------------------------------------------------
        // Appeals (acknowledged only)
        // -----------------------------------------------------------------
        (&Method::POST, "appeals") => handle_file_appeal(req).await,

        // -----------------------------------------------------------------
        // Activity logging (acknowledged only)
        // -----------------------------------------------------------------
        (&Method::POST, "activity") => handle_log_activity(req).await,

        // -----------------------------------------------------------------
        // Allocations (backed by local SQLite via stewardship_allocations)
        // -----------------------------------------------------------------
        (&Method::GET, "allocations") => handle_list_allocations(req, pool, ctx).await,

        (&Method::POST, "allocations") => {
            handle_create_allocation(req, pool, ctx, services.clone()).await
        }

        (&Method::GET, p) if p.starts_with("allocations/content/") => {
            let content_id = p.trim_start_matches("allocations/content/");
            handle_get_allocations_for_content(content_id, pool, ctx).await
        }

        (&Method::GET, p) if p.starts_with("allocations/steward/") => {
            let steward_id = p.trim_start_matches("allocations/steward/");
            handle_get_allocations_for_steward(steward_id, pool, ctx).await
        }

        (&Method::POST, p) if p.starts_with("allocations/") && p.ends_with("/dispute") => {
            let inner = p
                .trim_start_matches("allocations/")
                .trim_end_matches("/dispute");
            handle_file_dispute(req, inner, pool, ctx, services.clone()).await
        }

        (&Method::POST, p) if p.starts_with("allocations/") && p.ends_with("/resolve") => {
            let inner = p
                .trim_start_matches("allocations/")
                .trim_end_matches("/resolve");
            handle_resolve_dispute(req, inner, pool, ctx, services.clone()).await
        }

        (&Method::GET, p) if p.starts_with("allocations/") => {
            let id = p.trim_start_matches("allocations/");
            handle_get_allocation(id, pool, ctx).await
        }

        (&Method::PUT, p) if p.starts_with("allocations/") => {
            let id = p.trim_start_matches("allocations/");
            handle_update_allocation(req, id, pool, ctx, services.clone()).await
        }

        (&Method::DELETE, p) if p.starts_with("allocations/") => {
            let id = p.trim_start_matches("allocations/");
            handle_delete_allocation(id, pool, ctx, services.clone()).await
        }

        // -----------------------------------------------------------------
        // Device Policies (backed by device_policies table)
        // -----------------------------------------------------------------
        (&Method::POST, "policies") => handle_upsert_policy(req, pool).await,

        (&Method::GET, "policies/me/chain") => handle_my_policy_chain(req, pool).await,

        (&Method::GET, p) if p.starts_with("policies/") && p.ends_with("/parent") => {
            let subject_id = p
                .trim_start_matches("policies/")
                .trim_end_matches("/parent");
            handle_get_parent_policy(subject_id, pool).await
        }

        (&Method::GET, p) if p.starts_with("policies/") && p.ends_with("/chain") => {
            let subject_id = p.trim_start_matches("policies/").trim_end_matches("/chain");
            handle_get_policy_chain(subject_id, pool).await
        }

        (&Method::GET, "policies") => handle_list_policies(req, pool).await,

        (&Method::GET, p) if p.starts_with("policies/") => {
            let subject_id = p.trim_start_matches("policies/");
            handle_get_subject_policy(subject_id, pool).await
        }

        // -----------------------------------------------------------------
        // Time access check (uses existing PolicyEnforcement)
        // -----------------------------------------------------------------
        (&Method::GET, "access/time") => handle_check_time_access(req, pool).await,

        _ => response::not_found(&format!(
            "Unknown stewardship route: {} /api/v1/stewardship/{}",
            method, path
        )),
    })
}

// =============================================================================
// Policy handlers
// =============================================================================

async fn handle_get_policy(
    req: Request<Incoming>,
    pool: &DbPool,
    _ctx: &AppContext,
) -> Response<Full<Bytes>> {
    // Extract agentId query param
    let agent_id = extract_query_param(req.uri().query(), "agentId");
    let agent_id = match agent_id {
        Some(id) if !id.is_empty() => id,
        _ => return response::bad_request("agentId query parameter is required"),
    };

    // Try to load from policy cache
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    // Attempt to look up from cached_policies via raw query
    use diesel::prelude::*;
    let policy_json_result: Result<Option<String>, StorageError> = {
        use crate::db::policy_cache::cached_policies;
        cached_policies::table
            .filter(cached_policies::agent_id.eq(&agent_id))
            .select(cached_policies::policy_json)
            .first::<String>(&mut conn)
            .optional()
            .map_err(|e| StorageError::Internal(format!("Failed to query policy cache: {}", e)))
    };

    match policy_json_result {
        Ok(Some(json)) => {
            let transformed = StewardshipService::transform_cached_policy(&agent_id, &json);
            response::ok(&transformed)
        }
        Ok(None) => {
            // Return default (permissive) policy when no cached policy exists
            let default_policy = StewardshipService::transform_cached_policy(
                &agent_id,
                &serde_json::json!({
                    "computedAt": chrono::Utc::now().to_rfc3339(),
                    "blockedCategories": [],
                    "blockedHashes": [],
                    "disabledFeatures": [],
                    "disabledRoutes": [],
                    "requireApproval": [],
                    "timeWindowsJson": "[]",
                    "logSessions": false,
                    "logCategories": false,
                    "logPolicyEvents": true,
                    "retentionDays": 30,
                    "subjectCanView": true,
                })
                .to_string(),
            );
            response::ok(&default_policy)
        }
        Err(e) => response::error_response(e),
    }
}

// =============================================================================
// Grant handlers (acknowledged — no local SQL storage for grants themselves)
// =============================================================================

async fn handle_list_grants(_req: Request<Incoming>) -> Response<Full<Bytes>> {
    // Grants are stored in the Holochain zome, not local SQLite.
    // Return empty list with acknowledgement note.
    response::ok(&serde_json::json!({
        "grants": [],
        "note": "Grant data is managed by the Holochain zome. Use the zome RPC for full grant listings."
    }))
}

async fn handle_get_grant(id: &str) -> Response<Full<Bytes>> {
    response::not_found(&format!("Grant {} is managed by the Holochain zome", id))
}

async fn handle_create_grant(req: Request<Incoming>) -> Response<Full<Bytes>> {
    let body: CreateGrantRequest = match parse_body(req).await {
        Ok(b) => b,
        Err(_) => return response::bad_request("Invalid JSON body for create grant"),
    };

    if body.subject_id.trim().is_empty() {
        return response::bad_request("subjectId is required");
    }
    if body.authority_basis.trim().is_empty() {
        return response::bad_request("authorityBasis is required");
    }

    response::ok(&AcknowledgedResponse {
        acknowledged: true,
        message: format!(
            "Grant request for subject '{}' acknowledged. Submit to Holochain zome for execution.",
            body.subject_id
        ),
    })
}

async fn handle_delegate_grant(req: Request<Incoming>) -> Response<Full<Bytes>> {
    let body: DelegateGrantRequest = match parse_body(req).await {
        Ok(b) => b,
        Err(_) => return response::bad_request("Invalid JSON body for delegate grant"),
    };

    if body.parent_grant_id.trim().is_empty() {
        return response::bad_request("parentGrantId is required");
    }
    if body.new_steward_id.trim().is_empty() {
        return response::bad_request("newStewardId is required");
    }

    response::ok(&AcknowledgedResponse {
        acknowledged: true,
        message: format!(
            "Delegation of grant '{}' to '{}' acknowledged.",
            body.parent_grant_id, body.new_steward_id
        ),
    })
}

// =============================================================================
// Appeal handlers (acknowledged)
// =============================================================================

async fn handle_file_appeal(req: Request<Incoming>) -> Response<Full<Bytes>> {
    let body: FileAppealRequest = match parse_body(req).await {
        Ok(b) => b,
        Err(_) => return response::bad_request("Invalid JSON body for file appeal"),
    };

    if body.grant_id.trim().is_empty() {
        return response::bad_request("grantId is required");
    }
    if body.grounds.is_empty() {
        return response::bad_request("grounds must not be empty");
    }
    if body.appeal_type.trim().is_empty() {
        return response::bad_request("appealType is required");
    }

    response::created(&AcknowledgedResponse {
        acknowledged: true,
        message: format!(
            "Appeal for grant '{}' (type: {}) filed. Submit to Holochain zome for arbitration.",
            body.grant_id, body.appeal_type
        ),
    })
}

// =============================================================================
// Activity logging handler (acknowledged)
// =============================================================================

async fn handle_log_activity(req: Request<Incoming>) -> Response<Full<Bytes>> {
    let body: LogActivityRequest = match parse_body(req).await {
        Ok(b) => b,
        Err(_) => return response::bad_request("Invalid JSON body for log activity"),
    };

    if body.session_id.trim().is_empty() {
        return response::bad_request("sessionId is required");
    }

    response::ok(&AcknowledgedResponse {
        acknowledged: true,
        message: format!(
            "Activity for session '{}' ({} min) logged.",
            body.session_id, body.session_duration_minutes
        ),
    })
}

// =============================================================================
// Allocation CRUD handlers
// =============================================================================

async fn handle_list_allocations(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let query_str = req.uri().query().unwrap_or("");
    let query: AllocationQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match StewardshipService::list_allocations(&mut conn, ctx, &query) {
        Ok(items) => {
            let views: Vec<StewardshipAllocationView> =
                items.into_iter().map(|a| a.into()).collect();
            response::ok(&views)
        }
        Err(e) => response::error_response(e),
    }
}

async fn handle_get_allocation(id: &str, pool: &DbPool, ctx: &AppContext) -> Response<Full<Bytes>> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match StewardshipService::get_allocation(&mut conn, ctx, id) {
        Ok(a) => response::ok(&StewardshipAllocationView::from(a)),
        Err(e) => response::error_response(e),
    }
}

async fn handle_create_allocation(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
    services: Option<Arc<Services>>,
) -> Response<Full<Bytes>> {
    // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
    // Currently null for direct storage writes. Backfill needed for pre-coherence data.
    // Note: NewStewardshipAllocation also omits the dht_anchor_hash field entirely —
    // both the insert struct and an upsert_with_anchor function need to be added.
    let input_view: CreateAllocationInputView = match parse_body(req).await {
        Ok(v) => v,
        Err(_) => return response::bad_request("Invalid JSON body for create allocation"),
    };

    let (gate_result, gate_view) = super::evaluate_gate(
        &services,
        pool,
        ctx,
        MutationType::AllocationUpdate,
        serde_json::json!({
            "contentId": input_view.content_id,
            "stewardPresenceId": input_view.steward_presence_id,
        }),
        Some(&input_view.steward_presence_id),
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

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    let input = input_view.into();
    match StewardshipService::create_allocation(&mut conn, ctx, input) {
        Ok(a) => response::created(&serde_json::json!({
            "data": StewardshipAllocationView::from(a),
            "gate": gate_view,
        })),
        Err(e) => response::error_response(e),
    }
}

async fn handle_update_allocation(
    req: Request<Incoming>,
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
    services: Option<Arc<Services>>,
) -> Response<Full<Bytes>> {
    let input_view: UpdateAllocationInputView = match parse_body(req).await {
        Ok(v) => v,
        Err(_) => return response::bad_request("Invalid JSON body for update allocation"),
    };

    let (gate_result, gate_view) = super::evaluate_gate(
        &services,
        pool,
        ctx,
        MutationType::AllocationUpdate,
        serde_json::json!({ "allocationId": id }),
        None,
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

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    let input = input_view.into();
    match StewardshipService::update_allocation(&mut conn, ctx, id, input) {
        Ok(a) => response::ok(&serde_json::json!({
            "data": StewardshipAllocationView::from(a),
            "gate": gate_view,
        })),
        Err(e) => response::error_response(e),
    }
}

async fn handle_delete_allocation(
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
    services: Option<Arc<Services>>,
) -> Response<Full<Bytes>> {
    let (gate_result, gate_view) = super::evaluate_gate(
        &services,
        pool,
        ctx,
        MutationType::AllocationUpdate,
        serde_json::json!({ "allocationId": id }),
        None,
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

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match StewardshipService::delete_allocation(&mut conn, ctx, id) {
        Ok(()) => response::no_content(),
        Err(e) => response::error_response(e),
    }
}

async fn handle_get_allocations_for_content(
    content_id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match StewardshipService::get_content_stewardship(&mut conn, ctx, content_id) {
        Ok(s) => response::ok(&ContentStewardshipView::from(s)),
        Err(e) => response::error_response(e),
    }
}

async fn handle_get_allocations_for_steward(
    steward_id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match StewardshipService::get_allocations_for_steward(&mut conn, ctx, steward_id) {
        Ok(items) => {
            let views: Vec<StewardshipAllocationView> =
                items.into_iter().map(|a| a.into()).collect();
            response::ok(&views)
        }
        Err(e) => response::error_response(e),
    }
}

// =============================================================================
// Dispute handlers
// =============================================================================

async fn handle_file_dispute(
    req: Request<Incoming>,
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
    services: Option<Arc<Services>>,
) -> Response<Full<Bytes>> {
    let body: FileDisputeRequest = match parse_body(req).await {
        Ok(b) => b,
        Err(_) => return response::bad_request("Invalid JSON body for file dispute"),
    };

    let (gate_result, gate_view) = super::evaluate_gate(
        &services,
        pool,
        ctx,
        MutationType::DisputeFiling,
        serde_json::json!({
            "allocationId": id,
            "disputedBy": body.disputed_by,
            "reason": body.reason,
        }),
        Some(&body.disputed_by),
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

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match StewardshipService::file_dispute(&mut conn, ctx, id, &body.disputed_by, &body.reason) {
        Ok(a) => response::ok(&serde_json::json!({
            "data": StewardshipAllocationView::from(a),
            "gate": gate_view,
        })),
        Err(e) => response::error_response(e),
    }
}

async fn handle_resolve_dispute(
    req: Request<Incoming>,
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
    services: Option<Arc<Services>>,
) -> Response<Full<Bytes>> {
    let body: ResolveDisputeRequest = match parse_body(req).await {
        Ok(b) => b,
        Err(_) => return response::bad_request("Invalid JSON body for resolve dispute"),
    };

    let (gate_result, gate_view) = super::evaluate_gate(
        &services,
        pool,
        ctx,
        MutationType::GovernanceVote,
        serde_json::json!({
            "allocationId": id,
            "ratifierId": body.ratifier_id,
            "newState": body.new_state,
        }),
        Some(&body.ratifier_id),
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

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match StewardshipService::resolve_dispute(
        &mut conn,
        ctx,
        id,
        &body.ratifier_id,
        &body.new_state,
    ) {
        Ok(a) => response::ok(&serde_json::json!({
            "data": StewardshipAllocationView::from(a),
            "gate": gate_view,
        })),
        Err(e) => response::error_response(e),
    }
}

// =============================================================================
// Internal helpers
// =============================================================================

/// Extract a query parameter value from an optional query string.
fn extract_query_param(query: Option<&str>, key: &str) -> Option<String> {
    let q = query?;
    for pair in q.split('&') {
        let mut parts = pair.splitn(2, '=');
        let k = parts.next()?;
        if k == key {
            return parts.next().map(percent_decode);
        }
    }
    None
}

// =============================================================================
// Device Policy handlers
// =============================================================================

async fn handle_upsert_policy(req: Request<Incoming>, pool: &DbPool) -> Response<Full<Bytes>> {
    let input_view: UpsertPolicyInputView = match parse_body(req).await {
        Ok(v) => v,
        Err(_) => return response::bad_request("Invalid JSON body for upsert policy"),
    };

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    // In v0, author_id defaults to "self" (no auth context yet)
    let db_input = crate::views::upsert_policy_to_db_input(input_view, "self", "self");

    match StewardshipService::upsert_device_policy(&mut conn, &db_input) {
        Ok(policy) => response::ok(&DevicePolicyView::from(policy)),
        Err(e) => response::error_response(e),
    }
}

async fn handle_list_policies(req: Request<Incoming>, pool: &DbPool) -> Response<Full<Bytes>> {
    let subject_id = extract_query_param(req.uri().query(), "subjectId");
    let subject_id = match subject_id {
        Some(id) if !id.is_empty() => id,
        _ => return response::bad_request("subjectId query parameter is required"),
    };

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match StewardshipService::get_policies_for_subject(&mut conn, &subject_id) {
        Ok(policies) => {
            let views: Vec<DevicePolicyView> = policies.into_iter().map(|p| p.into()).collect();
            response::ok(&views)
        }
        Err(e) => response::error_response(e),
    }
}

async fn handle_get_subject_policy(subject_id: &str, pool: &DbPool) -> Response<Full<Bytes>> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match StewardshipService::get_subject_policy(&mut conn, subject_id) {
        Ok(Some(policy)) => response::ok(&DevicePolicyView::from(policy)),
        Ok(None) => response::not_found(&format!("No policy found for subject {}", subject_id)),
        Err(e) => response::error_response(e),
    }
}

async fn handle_get_parent_policy(subject_id: &str, pool: &DbPool) -> Response<Full<Bytes>> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match StewardshipService::get_parent_policy(&mut conn, subject_id) {
        Ok(Some(policy)) => response::ok(&policy),
        Ok(None) => response::ok(&serde_json::json!(null)),
        Err(e) => response::error_response(e),
    }
}

async fn handle_get_policy_chain(subject_id: &str, pool: &DbPool) -> Response<Full<Bytes>> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match StewardshipService::get_policy_chain(&mut conn, subject_id) {
        Ok(chain) => response::ok(&chain),
        Err(e) => response::error_response(e),
    }
}

async fn handle_my_policy_chain(req: Request<Incoming>, pool: &DbPool) -> Response<Full<Bytes>> {
    let agent_id = extract_query_param(req.uri().query(), "agentId");
    let agent_id = match agent_id {
        Some(id) if !id.is_empty() => id,
        _ => return response::bad_request("agentId query parameter is required"),
    };

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match StewardshipService::get_policy_chain(&mut conn, &agent_id) {
        Ok(chain) => response::ok(&chain),
        Err(e) => response::error_response(e),
    }
}

async fn handle_check_time_access(req: Request<Incoming>, pool: &DbPool) -> Response<Full<Bytes>> {
    let agent_id = extract_query_param(req.uri().query(), "agentId");
    let agent_id = match agent_id {
        Some(id) if !id.is_empty() => id,
        _ => return response::bad_request("agentId query parameter is required"),
    };

    // Use existing PolicyEnforcement from policy_cache
    let policy_cache = crate::db::policy_cache::PolicyCache::new(pool.clone());
    let enforcement = crate::db::policy_cache::PolicyEnforcement::new(policy_cache);

    match enforcement.check_time_access(&agent_id) {
        Ok(decision) => {
            let view = match decision {
                crate::db::policy_cache::TimeAccessDecision::Allowed {
                    remaining_session,
                    remaining_daily,
                } => {
                    serde_json::json!({ "status": "allowed", "remainingSession": remaining_session, "remainingDaily": remaining_daily })
                }
                crate::db::policy_cache::TimeAccessDecision::OutsideWindow => {
                    serde_json::json!({ "status": "outside_window" })
                }
                crate::db::policy_cache::TimeAccessDecision::SessionLimit => {
                    serde_json::json!({ "status": "session_limit" })
                }
                crate::db::policy_cache::TimeAccessDecision::DailyLimit => {
                    serde_json::json!({ "status": "daily_limit" })
                }
            };
            response::ok(&view)
        }
        Err(_) => {
            // Fail open — return allowed if check fails
            response::ok(&serde_json::json!({ "status": "allowed" }))
        }
    }
}

/// Minimal percent-decode for query parameter values (replaces %XX and + → space).
fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '+' {
            out.push(' ');
        } else if c == '%' {
            let h1 = chars.next().unwrap_or('0');
            let h2 = chars.next().unwrap_or('0');
            if let Ok(byte) = u8::from_str_radix(&format!("{}{}", h1, h2), 16) {
                out.push(byte as char);
            } else {
                out.push('%');
                out.push(h1);
                out.push(h2);
            }
        } else {
            out.push(c);
        }
    }
    out
}
