//! Status endpoint for Doorway
//!
//! Provides runtime status information including active connections,
//! cluster health, and orchestration metrics.

use askama::Template;
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::{Request, Response, StatusCode};
use serde::Serialize;
use std::sync::Arc;

use crate::auth::{extract_token_from_header, JwtValidator, PermissionLevel};
use crate::orchestrator::NodeHealthStatus;
use crate::server::AppState;

/// Bootstrap service stats
#[derive(Debug, Serialize)]
pub struct BootstrapStats {
    /// Whether bootstrap is enabled
    pub enabled: bool,
    /// Number of registered agents
    pub agents: usize,
    /// Number of active spaces
    pub spaces: usize,
}

/// Cache service stats
#[derive(Debug, Serialize)]
pub struct CacheStats {
    /// Number of cached entries
    pub entries: usize,
    /// Cache hit count
    pub hits: u64,
    /// Cache miss count
    pub misses: u64,
    /// Hit rate percentage
    pub hit_rate: f64,
}

/// Orchestrator cluster stats
#[derive(Debug, Serialize)]
pub struct OrchestratorStats {
    /// Whether orchestrator is enabled
    pub enabled: bool,
    /// Region for this doorway
    pub region: String,
    /// Total nodes known to orchestrator
    pub total_nodes: usize,
    /// Online nodes
    pub online_nodes: usize,
    /// Degraded nodes (heartbeat issues)
    pub degraded_nodes: usize,
    /// Failed/offline nodes
    pub failed_nodes: usize,
    /// Nodes with health breakdown
    pub nodes: Vec<NodeSummary>,
}

/// Summary of a single node
#[derive(Debug, Serialize)]
pub struct NodeSummary {
    pub node_id: String,
    pub status: String,
    pub nats_provisioned: bool,
    pub last_heartbeat_secs_ago: Option<u64>,
    /// Combined health + impact score (0.0 - 1.0)
    pub combined_score: f64,
    /// Steward tier (caretaker, guardian, steward, pioneer)
    pub steward_tier: Option<String>,
    /// Trust score from peer attestations (0.0 - 1.0)
    pub trust_score: Option<f64>,
    /// Human-scale impact score (0.0 - 1.0)
    pub impact_score: Option<f64>,
}

/// Storage (elohim-storage) stats for import debugging
#[derive(Debug, Serialize)]
pub struct StorageStats {
    /// Whether storage URL is configured
    pub configured: bool,
    /// Storage URL (if configured)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Whether storage is reachable
    pub reachable: bool,
    /// Storage healthy (from health check)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub healthy: Option<bool>,
    /// Whether import API is enabled on storage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import_enabled: Option<bool>,
    /// Number of active import batches
    pub active_batches: usize,
    /// Recent import batches (last 5)
    pub recent_batches: Vec<ImportBatchSummary>,
    /// Error message if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Summary of an import batch
#[derive(Debug, Serialize)]
pub struct ImportBatchSummary {
    pub batch_id: String,
    pub status: String,
    pub progress: String,
}

/// Conductor connection stats
#[derive(Debug, Serialize)]
pub struct ConductorStats {
    /// Whether conductor is connected
    pub connected: bool,
    /// Number of connected workers
    pub connected_workers: usize,
    /// Total number of workers
    pub total_workers: usize,
}

/// Federation health stats from peer cache
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FederationHealthStats {
    pub enabled: bool,
    pub self_id: Option<String>,
    pub self_url: Option<String>,
    pub self_tier: Option<String>,
    pub self_uptime_7d: Option<f32>,
    pub peer_count: usize,
    pub peers: Vec<PeerHealthSummary>,
}

/// Summary of a single federation peer's health
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerHealthSummary {
    pub doorway_id: String,
    pub url: String,
    pub self_reported_status: Option<String>,
    pub peer_attestations: Vec<AttestationSummary>,
    pub peers_agree: String,
    pub consensus_status: String,
}

/// Summary of a single attestation from a peer
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttestationSummary {
    pub attestor_id: String,
    pub observed_status: String,
    pub response_time_ms: Option<u32>,
    pub conductor_healthy: Option<bool>,
    pub timestamp: i64,
}

/// Diagnostic recommendations
#[derive(Debug, Serialize)]
pub struct Diagnostics {
    /// Overall health status
    pub status: String,
    /// List of recommendations for fixing issues
    pub recommendations: Vec<String>,
}

/// Doorway's implementation of the shared HealthReporter trait.
struct DoorwayHealthReporter<'a> {
    state: &'a AppState,
    conductor_connected: bool,
    storage_reachable: bool,
}

impl elohim_compute::HealthReporter for DoorwayHealthReporter<'_> {
    fn service_id(&self) -> &str {
        "doorway"
    }

    fn health(&self) -> elohim_compute::ServiceHealth {
        if self.conductor_connected && self.storage_reachable {
            elohim_compute::ServiceHealth::Healthy
        } else if self.state.args.dev_mode {
            elohim_compute::ServiceHealth::Degraded
        } else if !self.conductor_connected && !self.storage_reachable {
            elohim_compute::ServiceHealth::Offline
        } else {
            // One subsystem connected, one not — partial availability in production
            elohim_compute::ServiceHealth::Degraded
        }
    }

    fn health_reason(&self) -> String {
        let conductor = if self.conductor_connected {
            "connected"
        } else {
            "disconnected"
        };
        let storage = if self.storage_reachable {
            "reachable"
        } else {
            "unreachable"
        };
        let subscribers = self.state.peer_health.active_count();
        format!(
            "conductor {}, storage {}, {} active subscriber(s)",
            conductor, storage, subscribers
        )
    }

    fn started_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.state.started_at
    }
}

/// Status response payload
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    /// Service name
    pub service: &'static str,
    /// Service version
    pub version: &'static str,
    /// Node ID
    pub node_id: String,
    /// Whether dev mode is enabled
    pub dev_mode: bool,
    /// Number of available hosts in router
    pub available_hosts: usize,
    /// MongoDB connection status
    pub mongodb_connected: bool,
    /// NATS connection status
    pub nats_connected: bool,
    /// Conductor connection stats
    pub conductor: ConductorStats,
    /// Storage (elohim-storage) stats
    pub storage: StorageStats,
    /// Bootstrap service stats
    pub bootstrap: BootstrapStats,
    /// Cache service stats
    pub cache: CacheStats,
    /// Orchestrator cluster stats
    pub orchestrator: OrchestratorStats,
    /// Federation health stats
    pub federation: FederationHealthStats,
    /// Diagnostic information and recommendations
    pub diagnostics: Diagnostics,
    /// Uniform compute report (shared fleet contract)
    pub compute: elohim_compute::ComputeReport,
}

// =============================================================================
// Askama HTML Template Types
// =============================================================================

/// A single component row in the status page
pub struct ComponentView {
    pub name: String,
    pub dot_color: String,
    pub detail: String,
}

/// A single federation peer card
pub struct PeerView {
    pub doorway_id: String,
    pub dot_color: String,
    pub peers_agree: String,
    pub consensus_status: String,
    pub self_reported_status: Option<String>,
    pub response_time_ms: Option<u32>,
}

/// A projection subscriber peer card for the status page
pub struct ProjectionPeerView {
    pub conductor_id: String,
    pub dot_color: String,
    pub health: String,
    pub reason: String,
    pub signals_received: String,
    pub last_signal: String,
    pub reconnect_attempts: u32,
}

/// Resource usage summary for the status page
pub struct ResourceUsageView {
    pub projection_storage: String,
    pub projection_documents: String,
    pub hot_cache_entries: String,
    pub total_requests: String,
    pub active_subscribers: String,
}

/// Shefa compute contribution (placeholder for future)
#[allow(dead_code)]
pub struct ShefaView {
    pub cpu_hours: String,
    pub storage: String,
    pub bandwidth: String,
    pub tokens_earned: String,
    pub steward_tier: String,
    pub trust_score: String,
}

/// Askama template for the /status HTML page
#[derive(Template)]
#[template(path = "status.html")]
pub struct StatusPageTemplate {
    pub doorway_id: String,
    pub status_label: String,
    pub self_tier: Option<String>,
    pub uptime_7d: Option<String>,
    pub version: String,
    pub node_id: String,
    pub uptime_segments: Vec<String>,
    pub peer_count: usize,
    pub cache_hit_rate: String,
    pub available_hosts: usize,
    pub region: String,
    pub components: Vec<ComponentView>,
    pub federation_enabled: bool,
    pub peers: Vec<PeerView>,
    pub shefa: Option<ShefaView>,
    pub projection_peers: Vec<ProjectionPeerView>,
    pub resource_usage: ResourceUsageView,
    pub is_operator: bool,
    pub attestation_log: Vec<String>,
}

// =============================================================================
// Shared Data Gathering
// =============================================================================

async fn fetch_projection_stats(state: &Arc<AppState>) -> (u64, u64) {
    let mut cached = state.projection_stats_cache.lock().await;
    if cached.0.elapsed().as_secs() < 30 {
        return (cached.1, cached.2);
    }

    let result = if let Some(ref mongo) = state.mongo {
        let db = mongo.inner().database(mongo.db_name());
        match db
            .run_command(bson::doc! { "collStats": "projected_entries" })
            .await
        {
            Ok(doc) => {
                let bytes = doc.get_i64("size").unwrap_or(0).max(0) as u64;
                let docs = doc.get_i64("count").unwrap_or(0).max(0) as u64;
                (bytes, docs)
            }
            Err(e) => {
                tracing::warn!("collStats query failed (deprecated in MongoDB 6.2+): {e}");
                (0, 0)
            }
        }
    } else {
        (0, 0)
    };

    *cached = (std::time::Instant::now(), result.0, result.1);
    result
}

/// Build the StatusResponse from the current AppState.
/// Shared by both the JSON endpoint and the HTML page handler.
async fn build_status_data(state: &Arc<AppState>) -> StatusResponse {
    let available_hosts = state.router.available_count().await;
    let mut recommendations = Vec::new();

    // Get conductor stats
    let (conductor_connected, connected_workers, total_workers) = match &state.pool {
        Some(pool) => {
            let connected = pool.connected_count();
            let total = pool.worker_count();
            (pool.is_healthy(), connected, total)
        }
        None => (false, 0, 0),
    };

    if !conductor_connected && !state.args.dev_mode {
        recommendations.push(
            "Conductor not connected - seeding will fail. Check worker pool health.".to_string(),
        );
    }

    let conductor = ConductorStats {
        connected: if state.args.dev_mode {
            true
        } else {
            conductor_connected
        },
        connected_workers,
        total_workers,
    };

    // Get storage stats
    let storage = if let Some(ref storage_url) = state.args.storage_url {
        match fetch_storage_status(storage_url).await {
            Ok(info) => StorageStats {
                configured: true,
                url: Some(storage_url.clone()),
                reachable: true,
                healthy: Some(info.healthy),
                import_enabled: Some(info.import_enabled),
                active_batches: info.batches.len(),
                recent_batches: info.batches,
                error: None,
            },
            Err(e) => {
                recommendations.push(format!(
                    "Cannot reach elohim-storage: {e} - imports will fail"
                ));
                StorageStats {
                    configured: true,
                    url: Some(storage_url.clone()),
                    reachable: false,
                    healthy: None,
                    import_enabled: None,
                    active_batches: 0,
                    recent_batches: vec![],
                    error: Some(e),
                }
            }
        }
    } else {
        recommendations.push("STORAGE_URL not configured - import API will fail".to_string());
        StorageStats {
            configured: false,
            url: None,
            reachable: false,
            healthy: None,
            import_enabled: None,
            active_batches: 0,
            recent_batches: vec![],
            error: Some("STORAGE_URL not configured".to_string()),
        }
    };

    // Get bootstrap stats
    let bootstrap = match &state.bootstrap {
        Some(store) => {
            let stats = store.stats();
            BootstrapStats {
                enabled: true,
                agents: stats.total_agents,
                spaces: stats.total_spaces,
            }
        }
        None => BootstrapStats {
            enabled: false,
            agents: 0,
            spaces: 0,
        },
    };

    // Get cache stats
    let cache_stats = state.cache.stats();
    let cache = CacheStats {
        entries: cache_stats.entries,
        hits: cache_stats.hits,
        misses: cache_stats.misses,
        hit_rate: cache_stats.hit_rate(),
    };

    // Get orchestrator stats
    let orchestrator = match &state.orchestrator {
        Some(orch_state) => {
            let config = orch_state.config();
            let all_nodes = orch_state.all_nodes().await;

            let mut online = 0;
            let mut degraded = 0;
            let mut failed = 0;
            let mut node_summaries = Vec::new();

            for node_status in &all_nodes {
                let status_str = match &node_status.status {
                    NodeHealthStatus::Discovered => "discovered",
                    NodeHealthStatus::Registering => "registering",
                    NodeHealthStatus::Online => {
                        online += 1;
                        "online"
                    }
                    NodeHealthStatus::Degraded => {
                        degraded += 1;
                        "degraded"
                    }
                    NodeHealthStatus::Offline => {
                        failed += 1;
                        "offline"
                    }
                    NodeHealthStatus::Failed => {
                        failed += 1;
                        "failed"
                    }
                };

                let last_heartbeat_secs_ago =
                    node_status.last_heartbeat.map(|t| t.elapsed().as_secs());

                // Extract social metrics if available
                let (steward_tier, trust_score, impact_score) = match &node_status.social_metrics {
                    Some(metrics) => (
                        Some(metrics.steward_tier.clone()),
                        Some(metrics.trust_score),
                        Some(metrics.impact_score()),
                    ),
                    None => (None, None, None),
                };

                node_summaries.push(NodeSummary {
                    node_id: node_status.node_id.clone(),
                    status: status_str.to_string(),
                    nats_provisioned: node_status.nats_provisioned,
                    last_heartbeat_secs_ago,
                    combined_score: node_status.combined_score,
                    steward_tier,
                    trust_score,
                    impact_score,
                });
            }

            OrchestratorStats {
                enabled: true,
                region: config.region.clone(),
                total_nodes: all_nodes.len(),
                online_nodes: online,
                degraded_nodes: degraded,
                failed_nodes: failed,
                nodes: node_summaries,
            }
        }
        None => OrchestratorStats {
            enabled: false,
            region: state
                .args
                .region
                .clone()
                .unwrap_or_else(|| "default".to_string()),
            total_nodes: 0,
            online_nodes: 0,
            degraded_nodes: 0,
            failed_nodes: 0,
            nodes: vec![],
        },
    };

    // Determine overall diagnostic status
    let diag_status = if conductor_connected && storage.reachable {
        "healthy".to_string()
    } else if state.args.dev_mode {
        "degraded (dev mode)".to_string()
    } else {
        "degraded".to_string()
    };

    // Build federation health stats from peer cache
    let federation = {
        let peer_cache = state.peer_cache.read().await;
        let peers: Vec<PeerHealthSummary> = peer_cache
            .iter()
            .map(|peer| PeerHealthSummary {
                doorway_id: peer.id.clone(),
                url: peer.url.clone(),
                self_reported_status: None, // TODO: from DHT heartbeats
                peer_attestations: vec![],  // TODO: from DHT attestations
                peers_agree: "0/0".to_string(),
                consensus_status: "unknown".to_string(),
            })
            .collect();
        let peer_count = peers.len();

        FederationHealthStats {
            enabled: peer_count > 0 || !state.args.federation_peers.is_empty(),
            self_id: state
                .args
                .doorway_id
                .clone()
                .or_else(|| Some(state.args.node_id.to_string())),
            self_url: state.args.doorway_url.clone(),
            self_tier: None,
            self_uptime_7d: None,
            peer_count,
            peers,
        }
    };

    let diagnostics = Diagnostics {
        status: diag_status,
        recommendations,
    };

    // Build uniform compute report
    let (projection_bytes, projection_documents) = fetch_projection_stats(state).await;
    let hot_cache_entries = state
        .projection
        .as_ref()
        .map(|p| p.hot_cache_stats().total_entries)
        .unwrap_or(0);

    let reporter = DoorwayHealthReporter {
        state,
        conductor_connected,
        storage_reachable: storage.reachable,
    };
    let resources = elohim_compute::ResourceSnapshot {
        timestamp: chrono::Utc::now(),
        requests: state.request_counters.snapshot(),
        active_connections: state.peer_health.active_count(),
        managed_storage_bytes: projection_bytes,
        managed_document_count: projection_documents,
    };
    let compute_peers = state.peer_health.snapshot();
    let extensions = serde_json::json!({
        "hotCacheEntries": hot_cache_entries,
        "cacheHitRate": cache_stats.hit_rate(),
    });
    let build_info = elohim_compute::BuildInfo::new("elohim-doorway");
    let compute = elohim_compute::ComputeReport::build(
        &reporter,
        &build_info,
        resources,
        compute_peers,
        extensions,
    );

    StatusResponse {
        service: "doorway",
        version: env!("CARGO_PKG_VERSION"),
        node_id: state.args.node_id.to_string(),
        dev_mode: state.args.dev_mode,
        available_hosts,
        mongodb_connected: state.mongo.is_some(),
        nats_connected: state.nats.is_some(),
        conductor,
        storage,
        bootstrap,
        cache,
        orchestrator,
        federation,
        diagnostics,
        compute,
    }
}

// =============================================================================
// JSON Handler
// =============================================================================

/// Handle status request — returns JSON
pub async fn status_check(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let status = build_status_data(&state).await;

    match serde_json::to_string_pretty(&status) {
        Ok(body) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body)))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::new(Bytes::from("Failed to build response")))
                    .unwrap()
            }),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::new(Bytes::from("Failed to serialize status")))
            .unwrap(),
    }
}

// =============================================================================
// HTML Page Handler
// =============================================================================

/// Format bytes as human-readable (KB/MB/GB)
fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

/// Format a number with comma separators
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// Handle status page request — returns server-rendered HTML via Askama
///
/// When a valid operator JWT (Admin-level) is present in the Authorization header
/// or `doorway_token` cookie, the page includes expanded diagnostics such as
/// the route registry summary and attestation log.
pub async fn status_page(req: Request<Incoming>, state: Arc<AppState>) -> Response<Full<Bytes>> {
    let is_operator = check_operator_auth(&req, &state);
    let data = build_status_data(&state).await;

    // Determine overall status label
    let status_label = if data.diagnostics.status.starts_with("healthy") {
        "Operational".to_string()
    } else if data.diagnostics.status.contains("degraded") {
        "Degraded".to_string()
    } else {
        "Offline".to_string()
    };

    // Build component list
    let mut components = Vec::new();

    // Gateway (always present)
    components.push(ComponentView {
        name: "Gateway".to_string(),
        dot_color: "green".to_string(),
        detail: format!("{} hosts available", data.available_hosts),
    });

    // Conductor Pool
    components.push(ComponentView {
        name: "Conductor Pool".to_string(),
        dot_color: if data.conductor.connected {
            "green".to_string()
        } else {
            "red".to_string()
        },
        detail: format!(
            "{}/{} workers",
            data.conductor.connected_workers, data.conductor.total_workers
        ),
    });

    // Projection Cache
    components.push(ComponentView {
        name: "Projection Cache".to_string(),
        dot_color: "green".to_string(),
        detail: format!(
            "{} entries, {:.0}% hit rate",
            data.cache.entries, data.cache.hit_rate
        ),
    });

    // Storage
    components.push(ComponentView {
        name: "Storage".to_string(),
        dot_color: if data.storage.reachable {
            "green".to_string()
        } else if data.storage.configured {
            "red".to_string()
        } else {
            "gray".to_string()
        },
        detail: if data.storage.reachable {
            "Connected".to_string()
        } else if let Some(ref err) = data.storage.error {
            err.clone()
        } else {
            "Not configured".to_string()
        },
    });

    // Build peer view list
    let peers: Vec<PeerView> = data
        .federation
        .peers
        .iter()
        .map(|p| {
            let dot_color = match p.consensus_status.as_str() {
                "healthy" => "green",
                "degraded" => "yellow",
                "unhealthy" | "offline" => "red",
                _ => "gray",
            };
            // Find the first attestation with a response time
            let response_time_ms = p.peer_attestations.iter().find_map(|a| a.response_time_ms);

            PeerView {
                doorway_id: p.doorway_id.clone(),
                dot_color: dot_color.to_string(),
                peers_agree: p.peers_agree.clone(),
                consensus_status: p.consensus_status.clone(),
                self_reported_status: p.self_reported_status.clone(),
                response_time_ms,
            }
        })
        .collect();

    // 168 hourly segments, all "nodata" for now (placeholder)
    let uptime_segments: Vec<String> = vec!["nodata".to_string(); 168];

    let doorway_id = data
        .federation
        .self_id
        .clone()
        .unwrap_or_else(|| data.node_id.clone());

    // Build projection peer views from compute report
    let projection_peers: Vec<ProjectionPeerView> = data
        .compute
        .peers
        .iter()
        .map(|p| {
            let dot_color = match p.health {
                elohim_compute::ServiceHealth::Healthy => "green",
                elohim_compute::ServiceHealth::Degraded => "yellow",
                elohim_compute::ServiceHealth::Offline => "red",
            };
            ProjectionPeerView {
                conductor_id: p.peer_id.clone(),
                dot_color: dot_color.to_string(),
                health: p.health.to_string(),
                reason: p.reason.clone(),
                signals_received: format_number(p.signals_received),
                last_signal: p
                    .last_signal_at
                    .map(|t| {
                        let secs = (chrono::Utc::now() - t).num_seconds().max(0);
                        if secs < 60 {
                            format!("{}s ago", secs)
                        } else if secs < 3600 {
                            format!("{}m ago", secs / 60)
                        } else {
                            format!("{}h ago", secs / 3600)
                        }
                    })
                    .unwrap_or_else(|| "never".to_string()),
                reconnect_attempts: p.reconnect_attempts,
            }
        })
        .collect();

    let resource_usage = ResourceUsageView {
        projection_storage: format_bytes(data.compute.resources.managed_storage_bytes),
        projection_documents: format_number(data.compute.resources.managed_document_count),
        hot_cache_entries: format_number(
            data.compute.extensions["hotCacheEntries"]
                .as_u64()
                .unwrap_or(0),
        ),
        total_requests: format_number(data.compute.resources.requests.total),
        active_subscribers: format_number(data.compute.resources.active_connections as u64),
    };

    let template = StatusPageTemplate {
        doorway_id,
        status_label,
        self_tier: data.federation.self_tier.clone(),
        uptime_7d: data.federation.self_uptime_7d.map(|u| format!("{:.1}", u)),
        version: data.version.to_string(),
        node_id: data.node_id.clone(),
        uptime_segments,
        peer_count: data.federation.peer_count,
        cache_hit_rate: format!("{:.0}", data.cache.hit_rate),
        available_hosts: data.available_hosts,
        region: data.orchestrator.region.clone(),
        components,
        federation_enabled: data.federation.enabled,
        peers,
        shefa: None,
        projection_peers,
        resource_usage,
        is_operator,
        attestation_log: if is_operator {
            build_operator_attestation_log(&state).await
        } else {
            vec![]
        },
    };

    match template.render() {
        Ok(html) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/html; charset=utf-8")
            .body(Full::new(Bytes::from(html)))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::new(Bytes::from("Failed to build response")))
                    .unwrap()
            }),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::new(Bytes::from(format!(
                "Template render error: {e}"
            ))))
            .unwrap(),
    }
}

// =============================================================================
// Operator Auth Helpers
// =============================================================================

/// Check whether the request carries a valid Admin-level JWT.
///
/// Looks in the `Authorization: Bearer <token>` header first, then falls back
/// to a `doorway_token` cookie.  Returns `true` only when the token is valid
/// and the permission level is at least `Admin`.
fn check_operator_auth(req: &Request<Incoming>, state: &AppState) -> bool {
    // Try Authorization header first
    let auth_header = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let token = extract_token_from_header(auth_header)
        .map(|t| t.to_string())
        // Fallback: try doorway_token cookie
        .or_else(|| {
            req.headers()
                .get("cookie")
                .and_then(|v| v.to_str().ok())
                .and_then(|cookies| {
                    cookies.split(';').find_map(|c| {
                        let c = c.trim();
                        c.strip_prefix("doorway_token=").map(|t| t.to_string())
                    })
                })
        });

    let Some(token) = token else {
        return false;
    };

    let jwt = if state.args.dev_mode {
        JwtValidator::new_dev()
    } else {
        match state.args.jwt_secret.as_ref() {
            Some(secret) => {
                match JwtValidator::new(secret.clone(), state.args.jwt_expiry_seconds) {
                    Ok(v) => v,
                    Err(_) => return false,
                }
            }
            None => return false,
        }
    };

    let result = jwt.verify_token(&token);
    if result.valid {
        result
            .claims
            .map(|c| c.permission_level >= PermissionLevel::Admin)
            .unwrap_or(false)
    } else {
        false
    }
}

/// Build the attestation log entries shown in the operator panel.
///
/// Currently surfaces a route registry summary. As the federation attestation
/// protocol matures, this will include recent probe results.
async fn build_operator_attestation_log(state: &Arc<AppState>) -> Vec<String> {
    let mut log = Vec::new();

    // Route registry summary
    let stats = state.route_registry.stats().await;
    log.push(format!(
        "Route registry: {} total routes ({} DNA sources, {} external agents, {} blob proxies, {} stream proxies)",
        stats.total_routes,
        stats.dna_route_sources,
        stats.external_agents,
        stats.blob_proxies,
        stats.stream_proxies,
    ));

    // Steward storage registration
    if state.args.storage_url.is_some() {
        log.push("Steward storage: registered".to_string());
    } else {
        log.push("Steward storage: not configured".to_string());
    }

    log
}

// =============================================================================
// Storage Status Helpers
// =============================================================================

/// Response from elohim-storage health endpoint
/// Note: elohim-storage returns { "status": "ok", ... } not { "healthy": bool }
#[derive(serde::Deserialize)]
struct StorageHealthResponse {
    status: String,
    #[serde(default)]
    _blobs: u64,
    #[serde(default)]
    _bytes: u64,
    #[serde(default)]
    _manifests: u64,
    #[serde(default)]
    import_enabled: bool,
}

/// Response from elohim-storage import batches endpoint
#[derive(serde::Deserialize)]
struct StorageBatchesResponse {
    #[serde(default)]
    batches: Vec<StorageBatchInfo>,
}

#[derive(serde::Deserialize)]
struct StorageBatchInfo {
    batch_id: String,
    status: String,
    processed_count: u32,
    total_items: u32,
}

/// Storage status info from elohim-storage health endpoint
struct StorageStatusInfo {
    healthy: bool,
    import_enabled: bool,
    batches: Vec<ImportBatchSummary>,
}

/// Fetch health and batch status from elohim-storage
async fn fetch_storage_status(storage_url: &str) -> Result<StorageStatusInfo, String> {
    let base_url = storage_url.trim_end_matches('/');

    // Create HTTP client with timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    // Fetch health
    let health_url = format!("{base_url}/health");
    let health_resp = client
        .get(&health_url)
        .send()
        .await
        .map_err(|e| format!("Connection failed: {e}"))?;

    if !health_resp.status().is_success() {
        return Err(format!(
            "Health check failed: HTTP {}",
            health_resp.status()
        ));
    }

    let health: StorageHealthResponse = health_resp
        .json()
        .await
        .map_err(|e| format!("Invalid health response: {e}"))?;

    // Derive healthy from status field (elohim-storage returns status: "ok")
    let healthy = health.status == "ok";
    let import_enabled = health.import_enabled;

    // Try to fetch batches (may not be available if import API disabled)
    let batches: Vec<ImportBatchSummary> = fetch_import_batches(&client, base_url)
        .await
        .unwrap_or_default();

    Ok(StorageStatusInfo {
        healthy,
        import_enabled,
        batches,
    })
}

/// Fetch import batches from elohim-storage
async fn fetch_import_batches(
    client: &reqwest::Client,
    base_url: &str,
) -> Result<Vec<ImportBatchSummary>, String> {
    let url = format!("{base_url}/import/batches");
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let data: StorageBatchesResponse = resp
        .json()
        .await
        .map_err(|e| format!("Invalid response: {e}"))?;

    // Take only last 5 batches
    let batches: Vec<ImportBatchSummary> = data
        .batches
        .into_iter()
        .take(5)
        .map(|b| ImportBatchSummary {
            batch_id: b.batch_id,
            status: b.status,
            progress: format!("{}/{}", b.processed_count, b.total_items),
        })
        .collect();

    Ok(batches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_serialization() {
        let status = StatusResponse {
            service: "doorway",
            version: "0.1.0",
            node_id: "test-node".to_string(),
            dev_mode: true,
            available_hosts: 3,
            mongodb_connected: true,
            nats_connected: true,
            conductor: ConductorStats {
                connected: true,
                connected_workers: 4,
                total_workers: 4,
            },
            storage: StorageStats {
                configured: true,
                url: Some("http://localhost:8090".to_string()),
                reachable: true,
                healthy: Some(true),
                import_enabled: Some(true),
                active_batches: 1,
                recent_batches: vec![ImportBatchSummary {
                    batch_id: "test-batch-1".to_string(),
                    status: "completed".to_string(),
                    progress: "100/100".to_string(),
                }],
                error: None,
            },
            bootstrap: BootstrapStats {
                enabled: true,
                agents: 10,
                spaces: 2,
            },
            cache: CacheStats {
                entries: 100,
                hits: 50,
                misses: 10,
                hit_rate: 83.33,
            },
            orchestrator: OrchestratorStats {
                enabled: true,
                region: "us-west".to_string(),
                total_nodes: 5,
                online_nodes: 4,
                degraded_nodes: 1,
                failed_nodes: 0,
                nodes: vec![NodeSummary {
                    node_id: "node-1".to_string(),
                    status: "online".to_string(),
                    nats_provisioned: true,
                    last_heartbeat_secs_ago: Some(5),
                    combined_score: 0.85,
                    steward_tier: Some("guardian".to_string()),
                    trust_score: Some(0.9),
                    impact_score: Some(0.75),
                }],
            },
            federation: FederationHealthStats {
                enabled: true,
                self_id: Some("test-doorway".to_string()),
                self_url: Some("https://test.elohim.host".to_string()),
                self_tier: None,
                self_uptime_7d: None,
                peer_count: 1,
                peers: vec![PeerHealthSummary {
                    doorway_id: "peer-1".to_string(),
                    url: "https://peer1.elohim.host".to_string(),
                    self_reported_status: None,
                    peer_attestations: vec![AttestationSummary {
                        attestor_id: "node-2".to_string(),
                        observed_status: "healthy".to_string(),
                        response_time_ms: Some(42),
                        conductor_healthy: Some(true),
                        timestamp: 1710000000,
                    }],
                    peers_agree: "1/1".to_string(),
                    consensus_status: "healthy".to_string(),
                }],
            },
            diagnostics: Diagnostics {
                status: "healthy".to_string(),
                recommendations: vec![],
            },
            compute: elohim_compute::ComputeReport {
                service_id: "doorway".to_string(),
                build: elohim_compute::BuildInfo {
                    version: "0.1.0".to_string(),
                    commit: "unknown".to_string(),
                    commit_full: "unknown".to_string(),
                    build_time: "unknown".to_string(),
                    rustc_version: "unknown".to_string(),
                    service: "elohim-doorway".to_string(),
                },
                health: elohim_compute::ServiceHealth::Healthy,
                health_reason: "all systems go".to_string(),
                started_at: chrono::Utc::now(),
                uptime_seconds: 60,
                resources: elohim_compute::ResourceSnapshot {
                    timestamp: chrono::Utc::now(),
                    requests: elohim_compute::RequestCounterSnapshot {
                        total: 0,
                        by_category: std::collections::HashMap::new(),
                    },
                    active_connections: 0,
                    managed_storage_bytes: 0,
                    managed_document_count: 0,
                },
                peers: vec![],
                extensions: serde_json::json!({}),
            },
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("doorway"));
        assert!(json.contains("test-node"));
        assert!(json.contains("orchestrator"));
        assert!(json.contains("us-west"));
        assert!(json.contains("bootstrap"));
        assert!(json.contains("cache"));
        assert!(json.contains("conductor"));
        assert!(json.contains("storage"));
        assert!(json.contains("diagnostics"));
        assert!(json.contains("federation"));
        assert!(json.contains("peerCount"));
        assert!(json.contains("selfId"));
        assert!(json.contains("consensusStatus"));
        assert!(json.contains("\"compute\""));
        assert!(json.contains("\"serviceId\""));
        assert!(json.contains("\"uptimeSeconds\""));
    }
}
