//! Health check endpoints
//!
//! Provides Kubernetes-style health probes:
//! - /health, /healthz - Liveness probe (is the service running?)
//! - /ready, /readyz - Readiness probe (is the service ready for traffic?)
//!
//! Liveness probes return 200 if doorway is running, regardless of conductor status.
//! Readiness probes return 200 only if at least one worker is connected to conductor,
//! UNLESS dev_mode is enabled (conductor connection is optional in dev mode).
//!
//! Dev mode: Conductor connection is optional. Doorway can operate as a pure
//! HTTP bridge to elohim-storage without direct conductor access. The conductor
//! path is only needed for web-based development (Eclipse Che) scenarios.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use serde::Serialize;
use std::sync::Arc;

use crate::server::AppState;

/// Health response for seeder compatibility and doorway-picker UI
///
/// The seeder expects this format:
/// - healthy: boolean - overall health status
/// - version: string - service version
/// - cacheEnabled: boolean - whether projection cache is available
/// - cache.enabled: boolean - backwards compat for seeder's data.cache?.enabled
/// - conductor.connected: boolean - whether conductor is available (seeder should check this!)
///
/// The doorway-picker UI expects:
/// - status: 'online' | 'degraded' | 'offline' | 'maintenance'
/// - registrationOpen: boolean - whether new users can register
#[derive(Serialize)]
pub struct HealthResponse {
    /// Overall health status (true if service is running)
    pub healthy: bool,
    /// Doorway status for UI display: 'online', 'degraded', 'offline', 'maintenance'
    pub status: &'static str,
    /// Whether new user registration is open
    #[serde(rename = "registrationOpen")]
    pub registration_open: bool,
    /// Service version
    pub version: &'static str,
    /// Uptime in seconds
    pub uptime: u64,
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
    /// Projection role (writer or reader)
    pub projection: ProjectionRole,
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
    /// Number of conductors in the pool
    pub pool_size: usize,
    /// Per-conductor pools with at least one connected worker
    pub pools_healthy: usize,
    /// Total per-conductor pools (one per conductor in CONDUCTOR_URLS)
    pub pools_total: usize,
}

/// Projection role details
#[derive(Serialize)]
pub struct ProjectionRole {
    /// Whether this instance is the projection writer
    pub writer: bool,
}

/// Build health response with current state
fn build_health_response(state: &AppState) -> HealthResponse {
    let args = &state.args;

    // Check worker pool health
    let (actual_conductor_connected, connected_workers, total_workers) = match &state.pool {
        Some(pool) => {
            let connected = pool.connected_count();
            let total = pool.worker_count();
            (pool.is_healthy(), connected, total)
        }
        None => {
            // No pool - can't verify conductor status
            (false, 0, 0)
        }
    };

    // Conductor pool size from registry
    let pool_size = state
        .conductor_registry
        .as_ref()
        .map(|r| r.conductor_count())
        .unwrap_or(0);

    // Per-conductor pool health from router
    let (pools_healthy, pools_total) = state
        .conductor_router
        .as_ref()
        .map(|r| (r.pools().healthy_count(), r.pools().total_count()))
        .unwrap_or((0, 0));

    // In dev mode, conductor connection is optional
    // Doorway can operate as pure HTTP bridge to elohim-storage
    let conductor_connected = if args.dev_mode {
        // Dev mode: always report healthy for conductor
        // (actual status still shown in connected_workers/total_workers)
        true
    } else {
        actual_conductor_connected
    };

    // Check if projection/cache is enabled
    let cache_enabled = state.projection.is_some();

    // Include conductor status info in error field if not connected (but not blocking in dev mode)
    let error = if !actual_conductor_connected && !args.dev_mode {
        Some(format!(
            "No workers connected to conductor ({connected_workers}/{total_workers} workers) - seeding will fail"
        ))
    } else if !actual_conductor_connected && args.dev_mode {
        Some(format!(
            "Dev mode: conductor not connected ({connected_workers}/{total_workers} workers) - using elohim-storage path"
        ))
    } else {
        None
    };

    // Determine status for doorway-picker UI
    // - 'online': fully operational
    // - 'degraded': running but conductor not connected (limited functionality)
    // - 'offline': would return early if service truly down
    // - 'maintenance': reserved for planned maintenance
    let status = if conductor_connected || args.dev_mode {
        "online"
    } else {
        "degraded"
    };

    // Registration is always open for now (can be made configurable via args later)
    let registration_open = true;

    // Calculate uptime in seconds (approximate - from process start)
    let uptime = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    HealthResponse {
        healthy: true, // Service is running
        status,
        registration_open,
        version: env!("CARGO_PKG_VERSION"),
        uptime,
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
            pool_size,
            pools_healthy,
            pools_total,
        },
        projection: ProjectionRole {
            writer: args.projection_writer,
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
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// Handle readiness probe (/ready, /readyz)
///
/// Returns 200 OK only if doorway can accept traffic.
/// In production: requires conductor connection.
/// In dev mode: conductor is optional (doorway bridges to elohim-storage).
/// Use this endpoint for load balancer health checks and seeder pre-flight checks.
pub fn readiness_check(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let response = build_health_response(&state);

    // Readiness depends on role:
    // - Writer instances: need conductor connection (to write projections)
    // - Reader instances: only need MongoDB (conductor is optional)
    // - Dev mode: always ready (conductor.connected is forced true)
    let is_ready = if !state.args.projection_writer {
        // Read replicas are ready if they have a projection store (MongoDB)
        state.projection.is_some() || state.args.dev_mode
    } else {
        response.conductor.connected
    };

    let body = serde_json::to_string(&response)
        .unwrap_or_else(|_| r#"{"healthy":false,"error":"Serialization failed"}"#.to_string());

    let status = if is_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// Version information for deployment verification
#[derive(Serialize)]
pub struct VersionResponse {
    /// Cargo package version
    pub version: &'static str,
    /// Git commit hash (short)
    pub commit: &'static str,
    /// Git commit hash (full)
    pub commit_full: &'static str,
    /// Build timestamp
    pub build_time: &'static str,
    /// Service name
    pub service: &'static str,
}

/// Handle version endpoint (/version)
///
/// Returns build information for deployment verification.
/// The orchestrator uses this to verify deployments match expected commits.
pub fn version_info() -> Response<Full<Bytes>> {
    let response = VersionResponse {
        version: env!("CARGO_PKG_VERSION"),
        commit: option_env!("GIT_COMMIT_SHORT").unwrap_or("unknown"),
        commit_full: option_env!("GIT_COMMIT_FULL").unwrap_or("unknown"),
        build_time: option_env!("BUILD_TIMESTAMP").unwrap_or("unknown"),
        service: "elohim-doorway",
    };

    let body = serde_json::to_string(&response)
        .unwrap_or_else(|_| r#"{"version":"unknown","commit":"unknown"}"#.to_string());

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}
