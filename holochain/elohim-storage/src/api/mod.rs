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

pub mod compute;
pub mod custodians;
pub mod economic_events;
pub mod exchange;
pub mod flow_planning;
pub mod identity;
pub mod mastery;
pub mod presence;
pub mod resources;
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
    if sub_path.starts_with("presence") {
        let resource_path = sub_path.strip_prefix("presence").unwrap_or("");
        presence::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("stewardship") {
        let resource_path = sub_path.strip_prefix("stewardship").unwrap_or("");
        stewardship::handle(req, method, resource_path, &pool, &app_ctx).await
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
    } else if sub_path.starts_with("compute") {
        let resource_path = sub_path.strip_prefix("compute").unwrap_or("");
        compute::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("custodians") {
        let resource_path = sub_path.strip_prefix("custodians").unwrap_or("");
        custodians::handle(req, method, resource_path, &pool, &app_ctx).await
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
