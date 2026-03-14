//! Enriched API endpoints for pillar-level business logic
//!
//! These `/api/v1/*` routes encapsulate domain logic that was previously
//! in doorway route handlers. By living in storage, both browser (via doorway
//! proxy) and Tauri (direct to :8090) get identical behavior.
//!
//! ## Architecture (Controller → Service → Model)
//!
//! ```text
//! api/*.rs (controllers)      — HTTP handlers, serde request/response types
//!     ↓
//! services/*_service.rs       — Business logic, validation, compound operations
//!     ↓
//! db/*.rs (models)            — Diesel queries, ORM models
//! ```

pub mod agreements;
pub mod attestations;
pub mod compute;
pub mod contributors;
pub mod custodians;
pub mod economic_events;
pub mod exchange;
pub mod flow_planning;
pub mod governance;
pub mod identity;
pub mod mastery;
pub mod presence;
pub mod rea_commitments;
pub mod recognition;
pub mod resources;
pub mod steward;
pub mod steward_affinity;
pub mod stewardship;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::{response, Services};

use std::sync::Arc;

/// Handle all `/api/v1/*` requests by dispatching to domain controllers
pub async fn handle_api_request(
    req: Request<Incoming>,
    method: Method,
    path: &str,
    pool: DbPool,
    services: Option<Arc<Services>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Strip /api/v1/ prefix
    let sub_path = path.strip_prefix("/api/v1/").unwrap_or("");

    // Extract app context from X-App-Id header, default to "lamad"
    let app_ctx = extract_app_context(&req);

    // Dispatch to domain controllers
    if sub_path.starts_with("agreements") {
        let resource_path = sub_path.strip_prefix("agreements").unwrap_or("");
        agreements::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("presence") {
        let resource_path = sub_path.strip_prefix("presence").unwrap_or("");
        presence::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("stewardship") {
        let resource_path = sub_path.strip_prefix("stewardship").unwrap_or("");
        stewardship::handle(req, method, resource_path, &pool, &app_ctx, services).await
    } else if sub_path.starts_with("economic-events") {
        let resource_path = sub_path.strip_prefix("economic-events").unwrap_or("");
        economic_events::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("resources") {
        let resource_path = sub_path.strip_prefix("resources").unwrap_or("");
        resources::handle(req, method, resource_path, &pool, &app_ctx, services).await
    } else if sub_path.starts_with("exchange") {
        let resource_path = sub_path.strip_prefix("exchange").unwrap_or("");
        exchange::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("identity") {
        let resource_path = sub_path.strip_prefix("identity").unwrap_or("");
        identity::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("mastery") {
        let resource_path = sub_path.strip_prefix("mastery").unwrap_or("");
        mastery::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("flow-planning") {
        let resource_path = sub_path.strip_prefix("flow-planning").unwrap_or("");
        flow_planning::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("governance") {
        let resource_path = sub_path.strip_prefix("governance").unwrap_or("");
        governance::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("compute") {
        let resource_path = sub_path.strip_prefix("compute").unwrap_or("");
        compute::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("custodians") {
        let resource_path = sub_path.strip_prefix("custodians").unwrap_or("");
        custodians::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("commitments") {
        let resource_path = sub_path.strip_prefix("commitments").unwrap_or("");
        rea_commitments::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("attestations") {
        let resource_path = sub_path.strip_prefix("attestations").unwrap_or("");
        attestations::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("contributors") {
        let resource_path = sub_path.strip_prefix("contributors").unwrap_or("");
        contributors::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("recognition") {
        let resource_path = sub_path.strip_prefix("recognition").unwrap_or("");
        recognition::handle(req, method, resource_path, &pool, &app_ctx, services).await
    } else if sub_path.starts_with("steward-affinity") {
        let resource_path = sub_path.strip_prefix("steward-affinity").unwrap_or("");
        steward_affinity::handle(req, method, resource_path, &pool, &app_ctx, services).await
    } else if sub_path.starts_with("steward") && !sub_path.starts_with("stewardship") {
        let resource_path = sub_path.strip_prefix("steward").unwrap_or("");
        steward::handle(req, method, resource_path, &pool, &app_ctx).await
    } else {
        Ok(response::not_found(&format!(
            "Unknown API route: /api/v1/{}",
            sub_path
        )))
    }
}

/// Extract app context from request headers, defaulting to "lamad"
fn extract_app_context(req: &Request<Incoming>) -> AppContext {
    let app_id = req
        .headers()
        .get("X-App-Id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("lamad")
        .to_string();
    AppContext { app_id }
}

/// Helper to get a Diesel connection from the pool
pub fn get_conn(pool: &DbPool) -> Result<crate::db::PooledConn, StorageError> {
    pool.get()
        .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))
}

// =============================================================================
// ElohimGate evaluation helper
// =============================================================================

use crate::services::elohim_gate::{
    GateResult, InferenceTier, MutationType, TrustContext, TrustSignals,
};
use crate::views::{GateEvaluationView, TrustContextView};

/// Evaluate a mutation through the ElohimGate.
/// Returns (GateResult, Option<GateEvaluationView>) for inclusion in response.
/// If services unavailable, returns PassThrough with no view.
pub async fn evaluate_gate(
    services: &Option<Arc<Services>>,
    mutation: MutationType,
    mutation_content: serde_json::Value,
) -> (GateResult, Option<GateEvaluationView>) {
    let Some(svc) = services else {
        return (
            GateResult::PassThrough {
                tier: InferenceTier::None,
            },
            None,
        );
    };

    // Sprint 2: placeholder TrustContext — Sprint 3 gathers real signals from DB
    let trust_ctx = TrustContext::compute(TrustSignals {
        mastery_depth: 0.5,
        steward_standing: 0.5,
        relationship_density: 0.5,
        governance_health: 0.5,
        behavioral_trust: 0.5,
        intent_divergence: 0.0,
    });

    let result = svc
        .gate
        .evaluate(mutation, &trust_ctx, mutation_content)
        .await;
    let view = build_gate_view(&result, &trust_ctx);
    (result, Some(view))
}

/// Build a GateEvaluationView from a GateResult + TrustContext
fn build_gate_view(result: &GateResult, ctx: &TrustContext) -> GateEvaluationView {
    let trust_view = TrustContextView {
        composite_trust: ctx.composite_trust,
        mastery_depth: ctx.mastery_depth,
        steward_standing: ctx.steward_standing,
        relationship_density: ctx.relationship_density,
        governance_health: ctx.governance_health,
        behavioral_trust: ctx.behavioral_trust,
        intent_divergence: ctx.intent_divergence,
        declared_intent: ctx.declared_intent.clone(),
    };
    match result {
        GateResult::PassThrough { tier } => GateEvaluationView {
            tier: format!("{:?}", tier),
            trust_context: trust_view,
            pause_prompt: None,
            confirm_token: None,
            settlement_boundary: None,
            appeal_path: None,
        },
        GateResult::Enriched { tier, .. } => GateEvaluationView {
            tier: format!("{:?}", tier),
            trust_context: trust_view,
            pause_prompt: None,
            confirm_token: None,
            settlement_boundary: None,
            appeal_path: None,
        },
        GateResult::Pause {
            tier,
            prompt,
            confirm_token,
            ..
        } => GateEvaluationView {
            tier: format!("{:?}", tier),
            trust_context: trust_view,
            pause_prompt: Some(prompt.clone()),
            confirm_token: Some(confirm_token.clone()),
            settlement_boundary: None,
            appeal_path: None,
        },
        GateResult::Settlement {
            tier,
            boundary,
            appeal_path,
            ..
        } => GateEvaluationView {
            tier: format!("{:?}", tier),
            trust_context: trust_view,
            pause_prompt: None,
            confirm_token: None,
            settlement_boundary: Some(boundary.clone()),
            appeal_path: appeal_path.clone(),
        },
    }
}

/// Helper to parse JSON request body
pub async fn parse_body<T: serde::de::DeserializeOwned>(
    req: Request<Incoming>,
) -> Result<T, StorageError> {
    use http_body_util::BodyExt;
    let body_bytes = req
        .into_body()
        .collect()
        .await
        .map_err(|e| StorageError::InvalidInput(format!("Failed to read body: {}", e)))?
        .to_bytes();
    serde_json::from_slice(&body_bytes)
        .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))
}
