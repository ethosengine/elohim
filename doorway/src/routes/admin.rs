//! Admin API endpoints for Shefa compute resources dashboard
//!
//! ## Endpoints
//!
//! - `GET /admin/nodes` - List all nodes with detailed resource info
//! - `GET /admin/nodes/{id}` - Get specific node details
//! - `GET /admin/cluster` - Cluster-wide aggregated metrics
//! - `GET /admin/resources` - Resource utilization summary
//! - `GET /admin/custodians` - Custodian network overview
//!
//! ## Human-Scale Metrics
//!
//! These endpoints expose both technical metrics (CPU, memory, storage)
//! and human-scale metrics (trust, reach, stewardship tier) to give
//! operators a holistic view of their network.

use bson::doc;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};
use serde::Serialize;
use std::sync::Arc;

use crate::db::schemas::{UserDoc, USER_COLLECTION};
use crate::orchestrator::{NodeHealthStatus, SocialMetrics};
use crate::server::AppState;

// ============================================================================
// Node Details Response
// ============================================================================

/// Detailed node information for dashboard
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeDetails {
    /// Node identifier
    pub node_id: String,
    /// Current health status
    pub status: String,
    /// Whether NATS credentials are provisioned
    pub nats_provisioned: bool,
    /// Seconds since last heartbeat (None if never received)
    pub last_heartbeat_secs_ago: Option<u64>,

    // Technical resource metrics
    /// CPU cores available
    pub cpu_cores: Option<u32>,
    /// Memory in GB
    pub memory_gb: Option<u32>,
    /// Storage in TB
    pub storage_tb: Option<f64>,
    /// Bandwidth in Mbps
    pub bandwidth_mbps: Option<u32>,
    /// CPU utilization percent (from last heartbeat)
    pub cpu_usage_percent: Option<f64>,
    /// Memory utilization percent (from last heartbeat)
    pub memory_usage_percent: Option<f64>,
    /// Storage used in TB
    pub storage_usage_tb: Option<f64>,
    /// Active connections
    pub active_connections: Option<u32>,
    /// Content being custodied in GB
    pub custodied_content_gb: Option<f64>,

    // Human-scale metrics
    /// Steward tier (caretaker, guardian, steward, pioneer)
    pub steward_tier: Option<String>,
    /// Maximum reach level this node serves
    pub max_reach_level: Option<u8>,
    /// Actively serving reach levels
    pub active_reach_levels: Option<Vec<u8>>,
    /// Trust score from peer attestations (0.0 - 1.0)
    pub trust_score: Option<f64>,
    /// Number of humans served in last period
    pub humans_served: Option<u32>,
    /// Content pieces custodied
    pub content_custodied: Option<u32>,
    /// Successful deliveries in period
    pub successful_deliveries: Option<u32>,
    /// Failed deliveries in period
    pub failed_deliveries: Option<u32>,
    /// Delivery success rate (0.0 - 1.0)
    pub delivery_success_rate: Option<f64>,
    /// Human-scale impact score (0.0 - 1.0)
    pub impact_score: Option<f64>,
    /// Combined health + impact score (0.0 - 1.0)
    pub combined_score: f64,

    // Location/region
    /// Geographic region
    pub region: Option<String>,
}

/// Response for /admin/nodes
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodesResponse {
    /// Total count of nodes
    pub total: usize,
    /// Nodes by status
    pub by_status: NodeStatusCounts,
    /// All node details
    pub nodes: Vec<NodeDetails>,
}

/// Node counts by status
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeStatusCounts {
    pub online: usize,
    pub degraded: usize,
    pub offline: usize,
    pub failed: usize,
    pub discovering: usize,
    pub registering: usize,
}

// ============================================================================
// Cluster Metrics Response
// ============================================================================

/// Cluster-wide aggregated metrics
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterMetrics {
    /// Region this doorway serves
    pub region: String,
    /// Total nodes in cluster
    pub total_nodes: usize,
    /// Nodes currently online
    pub online_nodes: usize,
    /// Cluster health ratio (online/total)
    pub health_ratio: f64,

    // Aggregate resource capacity
    pub total_cpu_cores: u32,
    pub total_memory_gb: u32,
    pub total_storage_tb: f64,
    pub total_bandwidth_mbps: u32,

    // Aggregate resource usage (from online nodes with metrics)
    pub avg_cpu_usage_percent: f64,
    pub avg_memory_usage_percent: f64,
    pub total_storage_used_tb: f64,
    pub total_active_connections: u32,
    pub total_custodied_content_gb: f64,

    // Human-scale aggregates
    pub avg_trust_score: f64,
    pub avg_impact_score: f64,
    pub total_humans_served: u32,
    pub total_content_custodied: u32,
    pub cluster_delivery_success_rate: f64,

    // Steward distribution
    pub steward_counts: StewardCounts,
    // Reach coverage
    pub reach_coverage: ReachCoverage,
}

/// Count of nodes by steward tier
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StewardCounts {
    pub pioneers: usize,
    pub stewards: usize,
    pub guardians: usize,
    pub caretakers: usize,
}

/// Reach level coverage (how many nodes serve each level)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct ReachCoverage {
    /// Nodes serving private content (reach 0)
    pub private: usize,
    /// Nodes serving invited content (reach 1)
    pub invited: usize,
    /// Nodes serving local content (reach 2)
    pub local: usize,
    /// Nodes serving neighborhood content (reach 3)
    pub neighborhood: usize,
    /// Nodes serving municipal content (reach 4)
    pub municipal: usize,
    /// Nodes serving bioregional content (reach 5)
    pub bioregional: usize,
    /// Nodes serving regional content (reach 6)
    pub regional: usize,
    /// Nodes serving commons content (reach 7)
    pub commons: usize,
}

// ============================================================================
// Resource Summary Response
// ============================================================================

/// Resource utilization summary
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSummary {
    /// CPU utilization breakdown
    pub cpu: ResourceUtilization,
    /// Memory utilization breakdown
    pub memory: ResourceUtilization,
    /// Storage utilization breakdown
    pub storage: StorageUtilization,
    /// Bandwidth utilization
    pub bandwidth: BandwidthUtilization,
    /// Cache performance
    pub cache: CachePerformance,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceUtilization {
    /// Total capacity
    pub total: f64,
    /// Currently used
    pub used: f64,
    /// Available
    pub available: f64,
    /// Utilization percentage
    pub utilization_percent: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageUtilization {
    /// Total capacity in TB
    pub total_tb: f64,
    /// Used storage in TB
    pub used_tb: f64,
    /// Available in TB
    pub available_tb: f64,
    /// Utilization percentage
    pub utilization_percent: f64,
    /// Content custodied in GB
    pub custodied_content_gb: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BandwidthUtilization {
    /// Total bandwidth capacity in Mbps
    pub total_mbps: u32,
    /// Active connections using bandwidth
    pub active_connections: u32,
    /// Estimated per-connection bandwidth (rough)
    pub avg_bandwidth_per_connection_mbps: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachePerformance {
    /// Cache entries count
    pub entries: usize,
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Hit rate percentage
    pub hit_rate: f64,
}

// ============================================================================
// Custodian Network Response
// ============================================================================

/// Custodian network overview
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustodianNetworkResponse {
    /// Registered custodians
    pub registered_custodians: usize,
    /// Tracked blobs
    pub tracked_blobs: usize,
    /// Total commitments
    pub total_commitments: usize,
    /// Health probe stats
    pub total_probes: u64,
    pub successful_probes: u64,
    pub probe_success_rate: f64,
    /// Selection stats
    pub total_selections: u64,
    /// Healthy custodian count
    pub healthy_custodians: usize,
}

// ============================================================================
// Handlers
// ============================================================================

/// Handle GET /admin/nodes
pub async fn handle_nodes(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let Some(ref orchestrator) = state.orchestrator else {
        return json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({"error": "Orchestrator not enabled"}),
        );
    };

    let all_nodes = orchestrator.all_nodes().await;
    let mut nodes = Vec::with_capacity(all_nodes.len());
    let mut by_status = NodeStatusCounts::default();

    for node in all_nodes {
        // Count by status
        match &node.status {
            NodeHealthStatus::Online => by_status.online += 1,
            NodeHealthStatus::Degraded => by_status.degraded += 1,
            NodeHealthStatus::Offline => by_status.offline += 1,
            NodeHealthStatus::Failed => by_status.failed += 1,
            NodeHealthStatus::Discovered => by_status.discovering += 1,
            NodeHealthStatus::Registering => by_status.registering += 1,
        }

        // Extract inventory metrics
        let (cpu_cores, memory_gb, storage_tb, bandwidth_mbps, region) = match &node.inventory {
            Some(inv) => (
                Some(inv.capacity.cpu_cores),
                Some(inv.capacity.memory_gb),
                Some(inv.capacity.storage_tb),
                Some(inv.capacity.bandwidth_mbps),
                Some(inv.region.clone()),
            ),
            None => (None, None, None, None, None),
        };

        // Extract social metrics
        let (
            steward_tier,
            max_reach_level,
            active_reach_levels,
            trust_score,
            humans_served,
            content_custodied,
            successful_deliveries,
            failed_deliveries,
            delivery_success_rate,
            impact_score,
        ) = extract_social_metrics(&node.social_metrics);

        nodes.push(NodeDetails {
            node_id: node.node_id.clone(),
            status: status_to_string(&node.status),
            nats_provisioned: node.nats_provisioned,
            last_heartbeat_secs_ago: node.last_heartbeat.map(|t| t.elapsed().as_secs()),
            cpu_cores,
            memory_gb,
            storage_tb,
            bandwidth_mbps,
            cpu_usage_percent: None,    // Would come from heartbeat message
            memory_usage_percent: None, // Would come from heartbeat message
            storage_usage_tb: None,     // Would come from heartbeat message
            active_connections: None,   // Would come from heartbeat message
            custodied_content_gb: None, // Would come from heartbeat message
            steward_tier,
            max_reach_level,
            active_reach_levels,
            trust_score,
            humans_served,
            content_custodied,
            successful_deliveries,
            failed_deliveries,
            delivery_success_rate,
            impact_score,
            combined_score: node.combined_score,
            region,
        });
    }

    // Sort by combined score descending
    nodes.sort_by(|a, b| {
        b.combined_score
            .partial_cmp(&a.combined_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let response = NodesResponse {
        total: nodes.len(),
        by_status,
        nodes,
    };

    json_response(StatusCode::OK, response)
}

/// Handle GET /admin/nodes/{id}
pub async fn handle_node_by_id(state: Arc<AppState>, node_id: &str) -> Response<Full<Bytes>> {
    let Some(ref orchestrator) = state.orchestrator else {
        return json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({"error": "Orchestrator not enabled"}),
        );
    };

    let Some(node) = orchestrator.get_node(node_id).await else {
        return json_response(
            StatusCode::NOT_FOUND,
            serde_json::json!({"error": "Node not found", "node_id": node_id}),
        );
    };

    let (cpu_cores, memory_gb, storage_tb, bandwidth_mbps, region) = match &node.inventory {
        Some(inv) => (
            Some(inv.capacity.cpu_cores),
            Some(inv.capacity.memory_gb),
            Some(inv.capacity.storage_tb),
            Some(inv.capacity.bandwidth_mbps),
            Some(inv.region.clone()),
        ),
        None => (None, None, None, None, None),
    };

    let (
        steward_tier,
        max_reach_level,
        active_reach_levels,
        trust_score,
        humans_served,
        content_custodied,
        successful_deliveries,
        failed_deliveries,
        delivery_success_rate,
        impact_score,
    ) = extract_social_metrics(&node.social_metrics);

    let details = NodeDetails {
        node_id: node.node_id.clone(),
        status: status_to_string(&node.status),
        nats_provisioned: node.nats_provisioned,
        last_heartbeat_secs_ago: node.last_heartbeat.map(|t| t.elapsed().as_secs()),
        cpu_cores,
        memory_gb,
        storage_tb,
        bandwidth_mbps,
        cpu_usage_percent: None,
        memory_usage_percent: None,
        storage_usage_tb: None,
        active_connections: None,
        custodied_content_gb: None,
        steward_tier,
        max_reach_level,
        active_reach_levels,
        trust_score,
        humans_served,
        content_custodied,
        successful_deliveries,
        failed_deliveries,
        delivery_success_rate,
        impact_score,
        combined_score: node.combined_score,
        region,
    };

    json_response(StatusCode::OK, details)
}

/// Handle GET /admin/cluster
pub async fn handle_cluster_metrics(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let Some(ref orchestrator) = state.orchestrator else {
        return json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({"error": "Orchestrator not enabled"}),
        );
    };

    let all_nodes = orchestrator.all_nodes().await;
    let config = orchestrator.config();

    let mut metrics = ClusterMetrics {
        region: config.region.clone(),
        total_nodes: all_nodes.len(),
        online_nodes: 0,
        health_ratio: 0.0,
        total_cpu_cores: 0,
        total_memory_gb: 0,
        total_storage_tb: 0.0,
        total_bandwidth_mbps: 0,
        avg_cpu_usage_percent: 0.0,
        avg_memory_usage_percent: 0.0,
        total_storage_used_tb: 0.0,
        total_active_connections: 0,
        total_custodied_content_gb: 0.0,
        avg_trust_score: 0.0,
        avg_impact_score: 0.0,
        total_humans_served: 0,
        total_content_custodied: 0,
        cluster_delivery_success_rate: 0.0,
        steward_counts: StewardCounts::default(),
        reach_coverage: ReachCoverage::default(),
    };

    let mut trust_sum = 0.0;
    let mut impact_sum = 0.0;
    let mut nodes_with_metrics = 0;
    let mut total_successful = 0u32;
    let mut total_failed = 0u32;

    for node in &all_nodes {
        // Count online nodes
        if node.status == NodeHealthStatus::Online {
            metrics.online_nodes += 1;
        }

        // Aggregate capacity from inventory
        if let Some(ref inv) = node.inventory {
            metrics.total_cpu_cores += inv.capacity.cpu_cores;
            metrics.total_memory_gb += inv.capacity.memory_gb;
            metrics.total_storage_tb += inv.capacity.storage_tb;
            metrics.total_bandwidth_mbps += inv.capacity.bandwidth_mbps;
        }

        // Aggregate social metrics
        if let Some(ref social) = node.social_metrics {
            nodes_with_metrics += 1;
            trust_sum += social.trust_score;
            impact_sum += social.impact_score();
            metrics.total_humans_served += social.humans_served;
            metrics.total_content_custodied += social.content_custodied;
            total_successful += social.successful_deliveries;
            total_failed += social.failed_deliveries;

            // Count steward tiers
            match social.steward_tier.as_str() {
                "pioneer" => metrics.steward_counts.pioneers += 1,
                "steward" => metrics.steward_counts.stewards += 1,
                "guardian" => metrics.steward_counts.guardians += 1,
                _ => metrics.steward_counts.caretakers += 1,
            }

            // Count reach coverage
            for &level in &social.active_reach_levels {
                match level {
                    0 => metrics.reach_coverage.private += 1,
                    1 => metrics.reach_coverage.invited += 1,
                    2 => metrics.reach_coverage.local += 1,
                    3 => metrics.reach_coverage.neighborhood += 1,
                    4 => metrics.reach_coverage.municipal += 1,
                    5 => metrics.reach_coverage.bioregional += 1,
                    6 => metrics.reach_coverage.regional += 1,
                    7 => metrics.reach_coverage.commons += 1,
                    _ => {}
                }
            }
        }
    }

    // Calculate averages
    if metrics.total_nodes > 0 {
        metrics.health_ratio = metrics.online_nodes as f64 / metrics.total_nodes as f64;
    }
    if nodes_with_metrics > 0 {
        metrics.avg_trust_score = trust_sum / nodes_with_metrics as f64;
        metrics.avg_impact_score = impact_sum / nodes_with_metrics as f64;
    }
    if total_successful + total_failed > 0 {
        metrics.cluster_delivery_success_rate =
            total_successful as f64 / (total_successful + total_failed) as f64;
    }

    json_response(StatusCode::OK, metrics)
}

/// Handle GET /admin/resources
pub async fn handle_resources(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let Some(ref orchestrator) = state.orchestrator else {
        return json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({"error": "Orchestrator not enabled"}),
        );
    };

    let all_nodes = orchestrator.all_nodes().await;

    let mut total_cpu: f64 = 0.0;
    let mut total_memory: f64 = 0.0;
    let mut total_storage: f64 = 0.0;
    let mut total_bandwidth: u32 = 0;
    let total_connections: u32 = 0;
    let custodied_content: f64 = 0.0;

    for node in &all_nodes {
        if let Some(ref inv) = node.inventory {
            total_cpu += inv.capacity.cpu_cores as f64;
            total_memory += inv.capacity.memory_gb as f64;
            total_storage += inv.capacity.storage_tb;
            total_bandwidth += inv.capacity.bandwidth_mbps;
        }
    }

    // Get cache stats
    let cache_stats = state.cache.stats();

    let summary = ResourceSummary {
        cpu: ResourceUtilization {
            total: total_cpu,
            used: 0.0, // Would need heartbeat data
            available: total_cpu,
            utilization_percent: 0.0,
        },
        memory: ResourceUtilization {
            total: total_memory,
            used: 0.0,
            available: total_memory,
            utilization_percent: 0.0,
        },
        storage: StorageUtilization {
            total_tb: total_storage,
            used_tb: 0.0,
            available_tb: total_storage,
            utilization_percent: 0.0,
            custodied_content_gb: custodied_content,
        },
        bandwidth: BandwidthUtilization {
            total_mbps: total_bandwidth,
            active_connections: total_connections,
            avg_bandwidth_per_connection_mbps: if total_connections > 0 {
                total_bandwidth as f64 / total_connections as f64
            } else {
                0.0
            },
        },
        cache: CachePerformance {
            entries: cache_stats.entries,
            hits: cache_stats.hits,
            misses: cache_stats.misses,
            hit_rate: cache_stats.hit_rate(),
        },
    };

    json_response(StatusCode::OK, summary)
}

/// Handle GET /admin/custodians
pub async fn handle_custodians(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let stats = state.custodian.stats();
    let healthy = state.custodian.healthy_custodian_count();

    let response = CustodianNetworkResponse {
        registered_custodians: stats.registered_custodians,
        tracked_blobs: stats.tracked_blobs,
        total_commitments: stats.total_commitments,
        total_probes: stats.total_probes,
        successful_probes: stats.successful_probes,
        probe_success_rate: stats.probe_success_rate(),
        total_selections: stats.total_selections,
        healthy_custodians: healthy,
    };

    json_response(StatusCode::OK, response)
}

// ============================================================================
// Helpers
// ============================================================================

fn status_to_string(status: &NodeHealthStatus) -> String {
    match status {
        NodeHealthStatus::Discovered => "discovered".to_string(),
        NodeHealthStatus::Registering => "registering".to_string(),
        NodeHealthStatus::Online => "online".to_string(),
        NodeHealthStatus::Degraded => "degraded".to_string(),
        NodeHealthStatus::Offline => "offline".to_string(),
        NodeHealthStatus::Failed => "failed".to_string(),
    }
}

#[allow(clippy::type_complexity)]
fn extract_social_metrics(
    social: &Option<SocialMetrics>,
) -> (
    Option<String>,
    Option<u8>,
    Option<Vec<u8>>,
    Option<f64>,
    Option<u32>,
    Option<u32>,
    Option<u32>,
    Option<u32>,
    Option<f64>,
    Option<f64>,
) {
    match social {
        Some(m) => (
            Some(m.steward_tier.clone()),
            Some(m.max_reach_level),
            Some(m.active_reach_levels.clone()),
            Some(m.trust_score),
            Some(m.humans_served),
            Some(m.content_custodied),
            Some(m.successful_deliveries),
            Some(m.failed_deliveries),
            Some(m.delivery_success_rate()),
            Some(m.impact_score()),
        ),
        None => (None, None, None, None, None, None, None, None, None, None),
    }
}

// ============================================================================
// Pipeline (User Lifecycle Funnel)
// ============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineResponse {
    pub registered_total: u64,
    pub registered_active: u64,
    pub hosted_total: u64,
    pub graduating_count: u64,
    pub steward_count: u64,
}

/// Handle GET /admin/pipeline
pub async fn handle_admin_pipeline(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let mongo = match &state.mongo {
        Some(m) => m,
        None => {
            return json_response(
                StatusCode::SERVICE_UNAVAILABLE,
                serde_json::json!({"error": "Database not available"}),
            )
        }
    };

    let collection = match mongo.collection::<UserDoc>(USER_COLLECTION).await {
        Ok(c) => c,
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({"error": format!("Database error: {e}")}),
            )
        }
    };

    let inner = collection.inner();

    let registered_total = inner.count_documents(doc! {}).await.unwrap_or(0);
    let registered_active = inner
        .count_documents(doc! { "is_active": true })
        .await
        .unwrap_or(0);
    let hosted_total = inner
        .count_documents(doc! { "conductor_id": { "$ne": null } })
        .await
        .unwrap_or(0);
    let graduating_count = inner
        .count_documents(doc! {
            "custodial_key.exported": true,
            "is_steward": false,
        })
        .await
        .unwrap_or(0);
    let steward_count = inner
        .count_documents(doc! { "is_steward": true })
        .await
        .unwrap_or(0);

    json_response(
        StatusCode::OK,
        PipelineResponse {
            registered_total,
            registered_active,
            hosted_total,
            graduating_count,
            steward_count,
        },
    )
}

// ============================================================================
// Capabilities
// ============================================================================

/// Feature flags derived from optional AppState fields
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitiesResponse {
    /// Orchestrator is available (cluster management, node health, provisioning)
    pub orchestrator: bool,
    /// Federation is available (peer doorways, NATS messaging)
    pub federation: bool,
    /// Conductor pool is available (hosted agent connections)
    pub conductor_pool: bool,
    /// NATS messaging is available
    pub nats: bool,
}

/// Handle GET /admin/capabilities
///
/// Returns feature flags derived from optional AppState fields so the
/// dashboard can discover available features upfront instead of hitting
/// 503 errors on individual endpoints.
///
/// This endpoint does NOT require authentication.
pub async fn handle_capabilities(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let response = CapabilitiesResponse {
        orchestrator: state.orchestrator.is_some(),
        federation: state.nats.is_some(),
        conductor_pool: state.pool.is_some() || state.admin_pool.is_some(),
        nats: state.nats.is_some(),
    };
    json_response(StatusCode::OK, response)
}

fn json_response<T: Serialize>(status: StatusCode, body: T) -> Response<Full<Bytes>> {
    match serde_json::to_string_pretty(&body) {
        Ok(json) => Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .header("Access-Control-Allow-Origin", "*")
            .body(Full::new(Bytes::from(json)))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::new(Bytes::from("Failed to build response")))
                    .unwrap()
            }),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::new(Bytes::from("Failed to serialize response")))
            .unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_to_string() {
        assert_eq!(status_to_string(&NodeHealthStatus::Online), "online");
        assert_eq!(status_to_string(&NodeHealthStatus::Degraded), "degraded");
        assert_eq!(status_to_string(&NodeHealthStatus::Failed), "failed");
    }

    #[test]
    fn test_reach_coverage_default() {
        let coverage = ReachCoverage::default();
        assert_eq!(coverage.commons, 0);
        assert_eq!(coverage.private, 0);
    }
}
