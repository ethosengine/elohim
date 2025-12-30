//! Health check endpoints
//!
//! Provides Kubernetes-style health probes:
//! - /health, /healthz - Liveness probe (is the service running?)
//! - /ready, /readyz - Readiness probe (is the service ready for traffic?)
//!
//! Liveness probes return 200 if doorway is running, regardless of conductor status.
//! Readiness probes return 200 only if at least one worker is connected to conductor.
//!
//! The seeder should check the `conductor.connected` field in the response body
//! to determine if it's safe to begin seeding operations.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use serde::Serialize;
use std::sync::Arc;

use crate::server::AppState;

/// Health response for seeder compatibility
///
/// The seeder expects this format:
/// - healthy: boolean - overall health status
/// - version: string - service version
/// - cacheEnabled: boolean - whether projection cache is available
/// - cache.enabled: boolean - backwards compat for seeder's data.cache?.enabled
/// - conductor.connected: boolean - whether conductor is available (seeder should check this!)
#[derive(Serialize)]
pub struct HealthResponse {
    /// Overall health status (true if service is running)
    pub healthy: bool,
    /// Service version
    pub version: &'static str,
    /// Whether cache/projection is enabled (top-level for new clients)
    #[serde(rename = "cacheEnabled")]
    pub cache_enabled: bool,
    /// Cache status object (backwards compat for seeder: data.cache?.enabled)
    pub cache: CacheStatus,
    /// Current timestamp
    pub timestamp: String,
    /// Operating mode
    pub mode: String,
    /// Node identifier
    pub node_id: String,
    /// Conductor connection status - IMPORTANT: seeder should check conductor.connected!
    pub conductor: ConductorHealth,
    /// Error message if conductor not connected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Cache status for backwards compatibility
#[derive(Serialize)]
pub struct CacheStatus {
    /// Whether cache is enabled
    pub enabled: bool,
}

/// Conductor connection health details
#[derive(Serialize)]
pub struct ConductorHealth {
    /// Whether conductor is connected - seeder MUST check this before seeding!
    pub connected: bool,
    /// Number of connected workers
    pub connected_workers: usize,
    /// Total number of workers
    pub total_workers: usize,
}

/// Build health response with current state
fn build_health_response(state: &AppState) -> HealthResponse {
    let args = &state.args;

    // Check worker pool health
    let (conductor_connected, connected_workers, total_workers) = match &state.pool {
        Some(pool) => {
            let connected = pool.connected_count();
            let total = pool.worker_count();
            (pool.is_healthy(), connected, total)
        }
        None => {
            // No pool means dev mode with direct proxy - consider healthy
            // as we can't verify conductor status without pool
            (true, 0, 0)
        }
    };

    // Check if projection/cache is enabled
    let cache_enabled = state.projection.is_some();

    // Include conductor status warning in error field if not connected
    let error = if !conductor_connected {
        Some(format!(
            "No workers connected to conductor (0/{} workers) - seeding will fail",
            total_workers
        ))
    } else {
        None
    };

    HealthResponse {
        healthy: true, // Service is running
        version: env!("CARGO_PKG_VERSION"),
        cache_enabled,
        cache: CacheStatus {
            enabled: cache_enabled,
        },
        timestamp: chrono::Utc::now().to_rfc3339(),
        mode: if args.dev_mode {
            "development".to_string()
        } else {
            "production".to_string()
        },
        node_id: args.node_id.to_string(),
        conductor: ConductorHealth {
            connected: conductor_connected,
            connected_workers,
            total_workers,
        },
        error,
    }
}

/// Handle liveness probe (/health, /healthz)
///
/// Returns 200 OK if doorway service is running.
/// The response body includes conductor status for informational purposes.
/// Callers that need to verify conductor connectivity should check `conductor.connected`.
pub fn health_check(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let response = build_health_response(&state);

    let body = serde_json::to_string(&response)
        .unwrap_or_else(|_| r#"{"healthy":true,"error":"Serialization failed"}"#.to_string());

    // Liveness probe: always return 200 if service is running
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// Handle readiness probe (/ready, /readyz)
///
/// Returns 200 OK only if doorway can accept traffic (conductor connected).
/// Returns 503 Service Unavailable if conductor is not connected.
/// Use this endpoint for load balancer health checks and seeder pre-flight checks.
pub fn readiness_check(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let response = build_health_response(&state);
    let conductor_connected = response.conductor.connected;

    let body = serde_json::to_string(&response)
        .unwrap_or_else(|_| r#"{"healthy":false,"error":"Serialization failed"}"#.to_string());

    // Readiness probe: return 503 if conductor not connected
    let status = if conductor_connected {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}
