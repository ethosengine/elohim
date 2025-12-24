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
    /// Bootstrap service stats
    pub bootstrap: BootstrapStats,
    /// Cache service stats
    pub cache: CacheStats,
    /// Orchestrator cluster stats
    pub orchestrator: OrchestratorStats,
}

/// Handle status request
pub async fn status_check(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let available_hosts = state.router.available_count().await;

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
                    NodeHealthStatus::Online => { online += 1; "online" },
                    NodeHealthStatus::Degraded => { degraded += 1; "degraded" },
                    NodeHealthStatus::Offline => { failed += 1; "offline" },
                    NodeHealthStatus::Failed => { failed += 1; "failed" },
                };

                let last_heartbeat_secs_ago = node_status.last_heartbeat.map(|t| t.elapsed().as_secs());

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
            region: state.args.region.clone().unwrap_or_else(|| "default".to_string()),
            total_nodes: 0,
            online_nodes: 0,
            degraded_nodes: 0,
            failed_nodes: 0,
            nodes: vec![],
        },
    };

    let status = StatusResponse {
        service: "doorway",
        version: env!("CARGO_PKG_VERSION"),
        node_id: state.args.node_id.to_string(),
        dev_mode: state.args.dev_mode,
        available_hosts,
        mongodb_connected: state.mongo.is_some(),
        nats_connected: state.nats.is_some(),
        bootstrap,
        cache,
        orchestrator,
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
                nodes: vec![
                    NodeSummary {
                        node_id: "node-1".to_string(),
                        status: "online".to_string(),
                        nats_provisioned: true,
                        last_heartbeat_secs_ago: Some(5),
                        combined_score: 0.85,
                        steward_tier: Some("guardian".to_string()),
                        trust_score: Some(0.9),
                        impact_score: Some(0.75),
                    },
                ],
            },
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("doorway"));
        assert!(json.contains("test-node"));
        assert!(json.contains("orchestrator"));
        assert!(json.contains("us-west"));
        assert!(json.contains("bootstrap"));
        assert!(json.contains("cache"));
    }
}
