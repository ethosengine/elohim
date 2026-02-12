//! Status endpoint for Doorway
//!
//! Provides runtime status information including active connections,
//! cluster health, and orchestration metrics.

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};
use serde::Serialize;
use std::sync::Arc;

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

/// Diagnostic recommendations
#[derive(Debug, Serialize)]
pub struct Diagnostics {
    /// Overall health status
    pub status: String,
    /// List of recommendations for fixing issues
    pub recommendations: Vec<String>,
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
    /// Diagnostic information and recommendations
    pub diagnostics: Diagnostics,
}

/// Handle status request
pub async fn status_check(state: Arc<AppState>) -> Response<Full<Bytes>> {
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
                    "Cannot reach elohim-storage: {} - imports will fail",
                    e
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

    let diagnostics = Diagnostics {
        status: diag_status,
        recommendations,
    };

    let status = StatusResponse {
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
        diagnostics,
    };

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
        .map_err(|e| format!("HTTP client error: {}", e))?;

    // Fetch health
    let health_url = format!("{}/health", base_url);
    let health_resp = client
        .get(&health_url)
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    if !health_resp.status().is_success() {
        return Err(format!(
            "Health check failed: HTTP {}",
            health_resp.status()
        ));
    }

    let health: StorageHealthResponse = health_resp
        .json()
        .await
        .map_err(|e| format!("Invalid health response: {}", e))?;

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
    let url = format!("{}/import/batches", base_url);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let data: StorageBatchesResponse = resp
        .json()
        .await
        .map_err(|e| format!("Invalid response: {}", e))?;

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
            diagnostics: Diagnostics {
                status: "healthy".to_string(),
                recommendations: vec![],
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
    }
}
