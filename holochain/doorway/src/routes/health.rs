//! Health check endpoints
//!
//! Provides /health and /healthz endpoints for Kubernetes probes.
//!
//! The health check verifies:
//! - Doorway service is running
//! - Worker pool is connected to conductor (if pool exists)
//!
//! Returns HTTP 200 if healthy, HTTP 503 if not.

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
#[derive(Serialize)]
pub struct HealthResponse {
    /// Overall health status
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
    /// Conductor connection status
    pub conductor: ConductorHealth,
    /// Error message if unhealthy
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
    /// Whether conductor is connected
    pub connected: bool,
    /// Number of connected workers
    pub connected_workers: usize,
    /// Total number of workers
    pub total_workers: usize,
}

/// Handle health check request
///
/// Checks conductor connectivity through the worker pool and returns
/// appropriate HTTP status code:
/// - 200 OK: All systems healthy, at least one worker connected to conductor
/// - 503 Service Unavailable: No workers connected to conductor
pub fn health_check(state: Arc<AppState>) -> Response<Full<Bytes>> {
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

    // Determine overall health
    let healthy = conductor_connected;
    let error = if !healthy {
        Some(format!(
            "No workers connected to conductor (0/{} workers)",
            total_workers
        ))
    } else {
        None
    };

    let response = HealthResponse {
        healthy,
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
    };

    let body = serde_json::to_string(&response)
        .unwrap_or_else(|_| r#"{"healthy":false,"error":"Serialization failed"}"#.to_string());

    // Return 503 if not healthy, 200 if healthy
    let status = if healthy {
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
