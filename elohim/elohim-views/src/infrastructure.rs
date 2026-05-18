//! infrastructure view types — migrated from elohim-storage/src/views.rs (VIEWS.T2).

use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use serde_json::Value;
use ts_rs::TS;
use crate::shared::*;
#[allow(unused_imports)]
use crate::imagodei::*;
#[allow(unused_imports)]
use crate::lamad::*;
#[allow(unused_imports)]
use crate::shefa::*;
#[allow(unused_imports)]
use crate::qahal::*;
#[allow(unused_imports)]
use crate::epr::*;

/// Extension capability map — keyed by capability name, value is a structured entry.
/// Used by `ElohimCapabilityProfile` for Tier-2 extensibility.
pub type CapabilityExtensions = std::collections::BTreeMap<String, CapabilityExtensionEntry>;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AppView {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub enabled: bool,
}

/// Health metrics for a custodian node
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianHealthView {
    pub uptime_percent: f64,
    pub availability: bool,
    pub response_time_p50_ms: f64,
    pub response_time_p95_ms: f64,
    pub response_time_p99_ms: f64,
    pub error_rate: f64,
    pub sla_compliance: bool,
}

/// Storage metrics for a custodian node
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianStorageMetricsView {
    pub total_capacity_bytes: i64,
    pub used_bytes: i64,
    pub free_bytes: i64,
    pub utilization_percent: f64,
    /// Parsed by-domain map (was JSON string in storage)
    pub by_domain: Option<JsonVal>,
    pub full_replica_bytes: i64,
    pub threshold_bytes: i64,
    pub erasure_coded_bytes: i64,
}

/// Bandwidth metrics for a custodian node
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianBandwidthView {
    pub declared_mbps: f64,
    pub current_usage_mbps: f64,
    pub peak_usage_mbps: f64,
    pub average_usage_mbps: f64,
    pub utilization_percent: f64,
    pub inbound_mbps: f64,
    pub outbound_mbps: f64,
    /// Parsed by-domain map (was JSON string in storage)
    pub by_domain: Option<JsonVal>,
}

/// Computation metrics for a custodian node
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianComputationView {
    pub cpu_cores: u32,
    pub cpu_usage_percent: f64,
    pub memory_gb: f64,
    pub memory_usage_percent: f64,
    pub zome_ops_per_second: f64,
    pub reconstruction_workload_percent: f64,
}

/// Reputation metrics for a custodian node
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianReputationView {
    pub reliability_rating: f64,
    pub speed_rating: f64,
    pub reputation_score: f64,
    pub specialization_bonus: f64,
    pub commitment_fulfillment: f64,
}

/// Economic metrics for a custodian node
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianEconomicView {
    pub steward_tier: u32,
    pub price_per_gb: f64,
    pub monthly_earnings: f64,
    pub lifetime_earnings: f64,
    pub active_commitments: u32,
    pub total_committed_bytes: i64,
}

/// Complete metrics snapshot for a single custodian node.
///
/// Assembled from the custodian_metrics table (reported by the node) and
/// used by the shefa dashboard and operator tooling.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianMetricsView {
    pub custodian_id: String,
    pub tier: u32,
    pub health: CustodianHealthView,
    pub storage: CustodianStorageMetricsView,
    pub bandwidth: CustodianBandwidthView,
    pub computation: CustodianComputationView,
    pub reputation: CustodianReputationView,
    pub economic: CustodianEconomicView,
    /// Unix timestamp (milliseconds) when metrics were collected
    pub collected_at: i64,
    /// Unix timestamp (milliseconds) of last update
    pub last_updated_at: i64,
}

/// Alert for custodian operators.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianAlertView {
    pub custodian_id: String,
    /// "warning" | "critical"
    pub severity: String,
    pub category: String,
    pub message: String,
    pub suggestion: Option<String>,
}

/// Recommendation for custodian operators.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianRecommendationView {
    pub custodian_id: String,
    pub category: String,
    pub opportunity: String,
    pub potential_revenue: Option<f64>,
}

/// Input view for a node reporting its own metrics snapshot.
///
/// The sub-metric groups are accepted as raw JSON values and stored verbatim
/// (serialised back to String for the `UpsertCustodianMetrics` insertable).
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ReportCustodianMetricsInputView {
    pub custodian_id: String,
    pub tier: u32,
    pub health: CustodianHealthView,
    pub storage: CustodianStorageMetricsView,
    pub bandwidth: CustodianBandwidthView,
    pub computation: CustodianComputationView,
    pub reputation: CustodianReputationView,
    pub economic: CustodianEconomicView,
    /// Unix timestamp (ms) when metrics were collected — defaults to now if absent
    pub collected_at: Option<i64>,
}

/// A node protecting the operator's data.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianNodeView {
    pub id: String,
    pub name: String,
    /// "family" | "friend" | "community" | "professional" | "institution"
    pub custodian_type: String,
    pub location_region: Option<String>,
    pub location_country: Option<String>,
    // dataStored
    pub data_stored_total_gb: f64,
    pub data_stored_shard_count: u32,
    pub data_stored_redundancy_level: u32,
    // health
    pub health_up_percent: f64,
    pub health_last_heartbeat: String,
    pub health_response_time_ms: f64,
    // commitment
    pub commitment_id: String,
    /// "active" | "pending" | "breached" | "expired"
    pub commitment_status: String,
    pub commitment_start_date: String,
    pub commitment_expiry_date: String,
    /// "auto-renew" | "manual" | "expired"
    pub commitment_renewal_status: String,
    pub trust_level: f64,
    pub relationship: String,
}

/// Geographic distribution of custodians in a region.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RegionalPresenceView {
    pub region: String,
    pub custodian_count: u32,
    pub data_shards: u32,
    pub redundancy: u32,
    pub risk_factors: Vec<String>,
}

/// Trust relationship in the trust graph.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct TrustRelationshipView {
    pub from: String,
    pub to: String,
    /// "family-member" | "friend" | "community-peer" | "professional" | "institution"
    pub relationship_type: String,
    pub trust_score: f64,
    pub depth: u32,
    /// "weak" | "moderate" | "strong"
    pub strength: String,
}

/// Complete family-community protection status.
///
/// Assembled from CustodianCommitment entries and DHT health data.
/// No dedicated DB table — assembled by the data protection service.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct FamilyCommunityProtectionStatusView {
    // Redundancy model
    /// "full_replica" | "threshold_split" | "erasure_coded"
    pub redundancy_strategy: String,
    pub redundancy_factor: f64,
    pub recovery_threshold: u32,
    // Custodian network
    pub custodians: Vec<CustodianNodeView>,
    pub total_custodians: u32,
    // Geographic distribution
    pub geographic_regions: Vec<RegionalPresenceView>,
    /// "centralized" | "distributed" | "geo-redundant"
    pub geographic_risk_profile: String,
    // Trust graph
    pub trust_graph: Vec<TrustRelationshipView>,
    // Overall protection status
    /// "vulnerable" | "protected" | "highly-protected"
    pub protection_level: String,
    pub estimated_recovery_time: String,
    pub last_verification: String,
    /// "verified" | "pending" | "failed"
    pub verification_status: String,
}

/// Time-series data point for metric history charts.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct MetricHistoryView {
    pub timestamp: String,
    pub value: f64,
}

/// Node availability tracking.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct UpTimeMetricsView {
    pub up_percent: f64,
    pub downtime_hours_24: f64,
    pub downtime_hours_7d: f64,
    pub downtime_hours_30d: f64,
    pub last_failure: Option<String>,
    pub consecutive_uptime: String,
    /// "excellent" | "good" | "fair" | "poor"
    pub reliability: String,
}

/// Real-time node performance and capacity.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ComputeMetricsView {
    // CPU
    pub cpu_total_cores: u32,
    pub cpu_available: u32,
    pub cpu_usage_percent: f64,
    pub cpu_usage_history: Vec<MetricHistoryView>,
    pub cpu_temperature: Option<f64>,
    // Memory
    pub memory_total_gb: f64,
    pub memory_used_gb: f64,
    pub memory_available_gb: f64,
    pub memory_usage_percent: f64,
    pub memory_usage_history: Vec<MetricHistoryView>,
    // Storage
    pub storage_total_gb: f64,
    pub storage_used_gb: f64,
    pub storage_available_gb: f64,
    pub storage_usage_percent: f64,
    pub storage_usage_history: Vec<MetricHistoryView>,
    pub storage_breakdown_holochain_gb: f64,
    pub storage_breakdown_cache_gb: f64,
    pub storage_breakdown_custodian_data_gb: f64,
    pub storage_breakdown_user_applications_gb: f64,
    // Network
    pub network_upstream_mbps: f64,
    pub network_downstream_mbps: f64,
    pub network_used_upstream_mbps: f64,
    pub network_used_downstream_mbps: f64,
    pub network_latency_p50: f64,
    pub network_latency_p95: f64,
    pub network_latency_p99: f64,
    pub network_connections_total: u32,
    pub network_connections_holochain: u32,
    pub network_connections_cache: u32,
    pub network_connections_custodian: u32,
    // Load
    pub load_average_one_minute: f64,
    pub load_average_five_minutes: f64,
    pub load_average_fifteen_minutes: f64,
    // Power (optional)
    pub power_consumption_watts: Option<f64>,
    pub power_thermal_output: Option<f64>,
}

/// A node in the user's cluster.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct OwnedNodeView {
    pub node_id: String,
    pub display_name: String,
    /// "holoport" | "holoport-plus" | "holoport-nano" | "self-hosted" | "cloud"
    pub node_type: String,
    /// "online" | "offline" | "degraded" | "maintenance" | "provisioning" | "unknown"
    pub status: String,
    pub last_heartbeat: String,
    pub consecutive_uptime: String,
    pub location_label: Option<String>,
    pub location_region: Option<String>,
    pub location_country: Option<String>,
    pub roles: Vec<NodeRoleView>,
    pub resources_cpu_percent: f64,
    pub resources_memory_percent: f64,
    pub resources_storage_used_gb: f64,
    pub resources_storage_total_gb: f64,
    pub resources_bandwidth_mbps: f64,
    pub custodian_activity_items_custodied: u32,
    pub custodian_activity_items_being_custodied: u32,
    pub custodian_activity_total_custodied_gb: f64,
    pub is_primary: bool,
}

/// A role a node is playing in the cluster.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct NodeRoleView {
    /// "storage" | "compute" | "gateway" | "custodian" | "archive"
    pub role: String,
    pub description: String,
    pub utilization_percent: f64,
}

/// Node topology overview for an operator.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct NodeTopologyStateView {
    pub nodes: Vec<OwnedNodeView>,
    pub total_nodes: u32,
    pub online_nodes: u32,
    pub offline_nodes: u32,
    pub degraded_nodes: u32,
    pub primary_node_id: Option<String>,
    pub primary_node_status: Option<String>,
    pub primary_node_is_online: Option<bool>,
    /// "healthy" | "degraded" | "critical" | "offline"
    pub cluster_health: String,
    pub alerts: Vec<OfflineNodeAlertView>,
    pub last_updated: String,
}

/// Alert when a node goes offline.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct OfflineNodeAlertView {
    pub id: String,
    /// "info" | "warning" | "critical"
    pub severity: String,
    pub node_id: String,
    pub node_name: String,
    pub is_primary_node: bool,
    /// "went-offline" | "degraded" | "heartbeat-missed" | "recovery-needed"
    pub event_type: String,
    pub message: String,
    pub detected_at: String,
    pub last_seen_online: String,
    pub offline_duration: String,
    pub impact_affected_content: u32,
    pub impact_affected_custodians: u32,
    pub impact_compute_gap_percent: f64,
    pub impact_storage_gap_percent: f64,
    pub recommended_actions: Vec<String>,
    pub help_flow_url: Option<String>,
    pub dismissed_at: Option<String>,
}

/// A single custodian relationship (helping or being helped).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CustodianRelationshipView {
    pub agent_id: String,
    pub display_name: String,
    /// "family" | "friend" | "community" | "professional"
    pub relationship_type: String,
    pub trust_score: f64,
    /// "i-help-them" | "they-help-me"
    pub direction: String,
    pub content_summary_total_items: u32,
    pub content_summary_total_gb: f64,
    /// Parsed content types breakdown (was JSON string in storage)
    pub content_summary_content_types: Vec<JsonVal>,
    /// "active" | "pending" | "at-risk" | "expired"
    pub status: String,
    pub last_activity: String,
    pub reliability: f64,
}

/// Bidirectional custodian view (who I help vs who helps me).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct BidirectionalCustodianView {
    pub helping: Vec<CustodianRelationshipView>,
    pub helping_count: u32,
    pub helping_total_gb: f64,
    pub being_helped_by: Vec<CustodianRelationshipView>,
    pub being_helped_by_count: u32,
    pub being_helped_by_total_gb: f64,
    pub mutual_aid_balance_ratio: f64,
    /// "giving-more" | "balanced" | "receiving-more"
    pub mutual_aid_balance_status: String,
    pub mutual_aid_balance_message: String,
    /// "strong" | "moderate" | "weak"
    pub community_strength: String,
}

/// Storage breakdown by content type.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentTypeStorageView {
    /// "video" | "audio" | "image" | "document" | "application" | "learning" | "other"
    pub content_type: String,
    pub display_label: String,
    pub icon: Option<String>,
    pub item_count: u32,
    pub size_gb: f64,
    pub percent_of_total: f64,
    pub fully_replicated: u32,
    pub under_replicated: u32,
    pub average_replicas: f64,
}

/// Storage breakdown by reach level (0–7).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ReachLevelStorageView {
    pub reach_level: u32,
    pub reach_label: String,
    pub item_count: u32,
    pub size_gb: f64,
    pub target_replicas: u32,
    pub current_replicas: f64,
    /// "met" | "under" | "over"
    pub replication_status: String,
}

/// Storage breakdown for a single node.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct NodeStorageBreakdownView {
    pub node_id: String,
    pub node_name: String,
    pub node_status: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub available_gb: f64,
    pub my_content_gb: f64,
    pub custodied_content_gb: f64,
    pub cache_content_gb: f64,
    /// Parsed content-type list (was JSON string in storage)
    pub content_types: Vec<JsonVal>,
}

/// What types of content are stored where.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StorageContentDistributionView {
    pub by_content_type: Vec<ContentTypeStorageView>,
    pub by_reach_level: Vec<ReachLevelStorageView>,
    pub by_node: Vec<NodeStorageBreakdownView>,
    pub total_items: u32,
    pub total_size_gb: f64,
    pub total_replica_count: u32,
}

/// A specific compute deficiency.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ComputeGapView {
    /// "cpu" | "memory" | "storage" | "bandwidth" | "redundancy"
    pub resource: String,
    pub current_value: f64,
    pub target_value: f64,
    pub gap_percent: f64,
    /// "minor" | "moderate" | "critical"
    pub severity: String,
    pub description: String,
    pub impact: String,
}

/// Suggested node to address compute gaps.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct NodeRecommendationView {
    /// "holoport" | "holoport-plus" | "holoport-nano" | "self-hosted" | "cloud"
    pub node_type: String,
    pub display_name: String,
    pub description: String,
    pub addresses_gaps: Vec<String>,
    pub improvement_percent: f64,
    pub estimated_cost_value: Option<f64>,
    pub estimated_cost_currency: Option<String>,
    pub estimated_cost_period: Option<String>,
    pub order_url: Option<String>,
    /// "recommended" | "optional" | "future"
    pub priority: String,
}

/// Compute needs assessment — gaps and recommendations for the help-flow.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ComputeNeedsAssessmentView {
    pub current_capacity_cpu_cores: u32,
    pub current_capacity_memory_gb: f64,
    pub current_capacity_storage_gb: f64,
    pub current_capacity_bandwidth_mbps: f64,
    pub gaps: Vec<ComputeGapView>,
    pub has_gaps: bool,
    /// "none" | "minor" | "moderate" | "critical"
    pub overall_gap_severity: String,
    pub recommendations: Vec<NodeRecommendationView>,
    pub help_flow_url: String,
    pub help_flow_cta: String,
}

/// Trust context summary for client observability.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct TrustContextView {
    pub composite_trust: f64,
    pub mastery_depth: f64,
    pub steward_standing: f64,
    pub relationship_density: f64,
    pub governance_health: f64,
    pub behavioral_trust: f64,
    pub intent_divergence: f64,
    pub declared_intent: Option<String>,
}

/// Gate evaluation result for client.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct GateEvaluationView {
    pub tier: String,
    pub trust_context: TrustContextView,
    /// Present when gate pauses the mutation
    pub pause_prompt: Option<String>,
    /// Token to confirm a paused mutation
    pub confirm_token: Option<String>,
    /// Present when gate settles (constitutional boundary)
    pub settlement_boundary: Option<String>,
    pub appeal_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ScheduleView {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub scheduled_at: Option<String>,
    pub expires_at: Option<String>,
    pub rrule: Option<String>,
    pub next_occurrence_at: Option<String>,
    pub occurrence_count: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateScheduleInputView {
    pub entity_type: String,
    pub entity_id: String,
    pub scheduled_at: Option<String>,
    pub expires_at: Option<String>,
    pub rrule: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct UpdateScheduleInputView {
    pub scheduled_at: Option<String>,
    pub expires_at: Option<String>,
    pub rrule: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SpatialContextView {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
    pub accuracy: Option<f64>,
    pub h3_res5: Option<String>,
    pub h3_res7: Option<String>,
    pub h3_res9: Option<String>,
    pub place_id: Option<String>,
    pub osm_type: Option<String>,
    pub osm_id: Option<i32>,
    pub label: Option<String>,
    pub context_type: String,
    pub geometry_json: Option<JsonVal>,
    pub metadata: Option<JsonVal>,
    pub observed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateSpatialContextInputView {
    pub entity_type: String,
    pub entity_id: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
    pub accuracy: Option<f64>,
    pub place_id: Option<String>,
    pub osm_type: Option<String>,
    pub osm_id: Option<i32>,
    pub label: Option<String>,
    pub context_type: Option<String>,
    pub geometry_json: Option<JsonVal>,
    pub metadata: Option<JsonVal>,
    pub observed_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct UpdateSpatialContextInputView {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
    pub accuracy: Option<f64>,
    pub place_id: Option<String>,
    pub label: Option<String>,
    pub context_type: Option<String>,
    pub geometry_json: Option<JsonVal>,
    pub metadata: Option<JsonVal>,
    pub observed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PlaceView {
    pub id: String,
    pub dht_anchor_hash: String,
    pub name: String,
    pub place_type: String,
    pub constitutional_layer: String,
    pub h3_index: String,
    pub h3_resolution: i32,
    pub geometry_json: Option<JsonVal>,
    pub centroid_lat: f64,
    pub centroid_lng: f64,
    pub parent_place_id: Option<String>,
    pub osm_reference: Option<JsonVal>,
    pub carrying_capacity: Option<JsonVal>,
    pub governing_collective_id: Option<String>,
    pub status: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata: Option<JsonVal>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreatePlaceInputView {
    pub name: String,
    pub place_type: String,
    pub constitutional_layer: String,
    pub h3_index: String,
    pub h3_resolution: i32,
    pub geometry_json: JsonVal,
    pub centroid_lat: f64,
    pub centroid_lng: f64,
    pub parent_place_id: Option<String>,
    pub osm_reference: Option<JsonVal>,
    pub carrying_capacity: Option<JsonVal>,
    pub governing_collective_id: Option<String>,
    pub status: Option<String>,
    pub metadata: Option<JsonVal>,
}

/// View for a geospatial hazard event
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct HazardView {
    pub id: String,
    pub h_app_id: String,
    pub place_id: String,
    pub hazard_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub reported_at: String,
    pub projected_onset: Option<String>,
    pub projected_end: Option<String>,
    pub actual_onset: Option<String>,
    pub resolved_at: Option<String>,
    pub affected_h3_cells: JsonVal,
    pub radius_km: Option<f64>,
    pub source: String,
    pub source_reference: Option<String>,
    pub metadata: Option<JsonVal>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating a hazard
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateHazardInputView {
    pub place_id: String,
    pub hazard_type: String,
    pub severity: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub projected_onset: Option<String>,
    #[serde(default)]
    pub projected_end: Option<String>,
    #[serde(default)]
    pub affected_h3_cells: Option<Vec<String>>,
    #[serde(default)]
    pub radius_km: Option<f64>,
    #[serde(default = "default_hazard_source")]
    pub source: String,
    #[serde(default)]
    pub source_reference: Option<String>,
    #[serde(default)]
    pub metadata: Option<JsonVal>,
}

/// View for a generated risk alert
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RiskAlertView {
    pub id: String,
    pub h_app_id: String,
    pub place_id: String,
    pub alert_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub trigger_hazard_id: Option<String>,
    pub trigger_data: Option<JsonVal>,
    pub triggered_at: String,
    pub lead_time_hours: Option<f64>,
    pub expires_at: Option<String>,
    pub status: String,
    pub acknowledged_by: Option<String>,
    pub acknowledged_at: Option<String>,
    pub resolved_at: Option<String>,
    pub escalated_to: Option<String>,
    pub metadata: Option<JsonVal>,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for updating a risk alert (PATCH semantics)
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct UpdateRiskAlertInputView {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub acknowledged_by: Option<String>,
    #[serde(default)]
    pub escalated_to: Option<String>,
}

/// Carrying capacity summary for the dashboard tile
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CapacitySummaryView {
    pub resource_count: u32,
    pub worst_utilization: f64,
    pub worst_category: String,
    pub trigger_governance: bool,
}

/// Hazard summary for the dashboard tile
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct HazardEntrySummaryView {
    pub active_count: u32,
    pub worst_severity: String,
    pub nearest_onset_hours: Option<f64>,
    pub types: Vec<String>,
}

/// Vulnerability summary for the dashboard tile
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct VulnerabilitySummaryView {
    pub overall_score: f64,
    pub risk_tier: String,
    pub preparation_status: String,
}

/// Weather summary for the dashboard tile
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct WeatherEntrySummaryView {
    pub risk_level: String,
    pub temperature_c: f64,
    pub precipitation_mm: f64,
    pub wind_speed_kmh: f64,
}

/// Alert summary for the dashboard tile
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AlertEntrySummaryView {
    pub active_count: u32,
    pub worst_severity: String,
    pub unacknowledged_count: u32,
    pub types: Vec<String>,
}

/// Route summary for the dashboard tile
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RouteEntrySummaryView {
    pub active_route_count: u32,
    pub total_distance_km: f64,
}

/// Single place entry in the planet-scale dashboard
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PlaceDashboardEntry {
    // Core (always present)
    pub id: String,
    pub name: String,
    pub place_type: String,
    pub constitutional_layer: String,
    pub h3_index: String,
    pub centroid_lat: f64,
    pub centroid_lng: f64,
    pub geometry_json: Option<JsonVal>,
    pub status: String,
    pub parent_place_id: Option<String>,
    // Optional enrichments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<CapacitySummaryView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hazards: Option<HazardEntrySummaryView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vulnerability: Option<VulnerabilitySummaryView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weather: Option<WeatherEntrySummaryView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alerts: Option<AlertEntrySummaryView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routes: Option<RouteEntrySummaryView>,
}

/// Risk tier distribution counts across all places
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RiskTierDistribution {
    pub low: u32,
    pub moderate: u32,
    pub elevated: u32,
    pub critical: u32,
    pub unknown: u32,
}

/// Summary statistics for the full dashboard
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct DashboardSummaryView {
    pub total_places: u32,
    pub places_by_risk_tier: RiskTierDistribution,
    pub total_active_hazards: u32,
    pub total_active_alerts: u32,
    pub total_unacknowledged_alerts: u32,
    pub worst_overall_risk: String,
}

/// Full spatial dashboard response
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SpatialDashboardView {
    pub places: Vec<PlaceDashboardEntry>,
    pub summary: DashboardSummaryView,
    pub queried_at: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ResilienceView {
    pub content_id: String,
    pub encoding: EncodingInfoView,
    pub distribution: DistributionView,
    pub stewardship: ResilienceStewardshipView,
    pub commitments: CommitmentHealthView,
    pub health: HealthScoreView,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EncodingInfoView {
    pub strategy: String,
    pub data_shards: i32,
    pub parity_shards: i32,
    pub total_size_bytes: i64,
    pub shard_size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct DistributionView {
    pub total_shards: i32,
    pub shards_with_locations: i32,
    pub distinct_peers: i32,
    pub shards: Vec<ShardInfoView>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ShardInfoView {
    pub hash: String,
    pub shard_type: String,
    pub peer_ids: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ResilienceStewardshipView {
    pub steward_count: i32,
    pub allocations: Vec<StewardshipAllocationView>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CommitmentHealthView {
    pub active_peers: i32,
    pub total_committed_bytes: i64,
    pub total_used_bytes: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct HealthScoreView {
    pub score: f32,
    pub can_survive_failures: i32,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct VerificationResultView {
    pub content_id: String,
    pub verified: bool,
    pub encoding: String,
    pub shards_available: i32,
    pub shards_needed: i32,
    pub shards_located: i32,
    pub shards_missing: i32,
    pub reconstruction_time_ms: u64,
    pub original_hash: String,
    pub reconstructed_hash: String,
    pub hash_match: bool,
    pub error: Option<String>,
}

/// Wire view for a notarized gate decision attestation.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct GateDecisionAttestationView {
    /// CID of this attestation (self-addressing, globally unique).
    pub decision_id: String,
    /// Deployment phase: "dev-context" | "elohim-active".
    pub phase: String,
    /// AgentPubKey (base64) of the deciding elohim agent.
    pub elohim_id: String,
    /// CID of the substance declaration active at decision time.
    pub elohim_substance_cid: String,
    /// Name of the gate that evaluated the request.
    pub gate_name: String,
    /// CID of the GateProcessDeclaration DAG that was executed.
    pub gate_process_cid: String,
    /// Serialised RequestRef (opaque JSON string).
    pub request_ref_json: String,
    /// Decision outcome: "allow" | "decline" | "escalate" | "verdict".
    pub decision: String,
    /// Full ConstitutionalReasoning (opaque JSON string).
    pub reasoning_json: String,
    /// CID of the privacy-respecting GateContext snapshot.
    pub context_summary_cid: String,
    /// ISO 8601 timestamp of when the decision was made.
    pub decided_at: String,
    /// CID of the universal-band DAG declaration active at decision time.
    pub universal_band_cid: String,
    /// ActionHash (base64) of the upstream DHT entry — provenance anchor.
    pub dht_anchor_hash: String,
    /// ISO 8601 timestamp of when this projection row was inserted.
    pub created_at: String,
}

/// Wire view for a notarized gate decision challenge.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct GateDecisionChallengeView {
    /// CID of this challenge (self-addressing, globally unique).
    pub challenge_id: String,
    /// CID of the GateDecisionAttestation being challenged.
    pub challenged_decision_cid: String,
    /// AgentPubKey (base64) of the challenger.
    pub challenger_id: String,
    /// Grounds: "factual-error" | "safety" | "policy" | "constitutional" | "indemnification-request".
    pub grounds: String,
    /// Challenger's articulation of the grievance.
    pub summary: String,
    /// Comma-separated CIDs of evidence refs (empty string if none).
    pub evidence_refs: String,
    /// ISO 8601 timestamp of when the challenge was filed.
    pub filed_at: String,
    /// Reach level: "self" | "intimate" | "community" | "commons".
    pub reach: String,
    /// ActionHash (base64) of the upstream DHT entry — provenance anchor.
    pub dht_anchor_hash: String,
    /// ISO 8601 timestamp of when this projection row was inserted.
    pub created_at: String,
}

/// Multi-dimensional reputation profile for an elohim agent over a time window.
///
/// Reputation emerges from the accumulated public record — no elohim can assert
/// its own reputation; no protocol steward can assign it.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ElohimReputationProfileView {
    /// AgentPubKey (base64) of the queried elohim agent.
    pub elohim_id: String,
    /// ISO 8601 timestamp — start of the query window (inclusive).
    pub window_start: String,
    /// ISO 8601 timestamp — end of the query window (inclusive).
    pub window_end: String,
    /// CID of the most-recently-used ElohimSubstance in the window.
    /// None if no decisions exist in the window.
    pub current_substance_cid: Option<String>,
    /// Count of GateDecisionAttestations where phase=elohim-active in the window.
    /// Dev-context decisions are excluded; they carry no reputation weight.
    pub total_decisions: i64,
    /// Count of distinct decisions that attracted at least one GateDecisionChallenge.
    pub challenged_count: i64,
    /// Count of ChallengeOutcomes with verdict=upheld.
    pub upheld_count: i64,
    /// Count of ChallengeOutcomes with verdict=dismissed.
    pub dismissed_count: i64,
    /// Count of ChallengeOutcomes with verdict=superseded.
    pub superseded_count: i64,
    /// Count of challenges with no ChallengeOutcome yet (still open).
    pub pending_count: i64,
    /// Histogram of ChallengeGrounds values to count.
    /// Keys: "factual-error" | "safety" | "policy" | "constitutional" | "indemnification-request".
    /// Missing keys indicate zero challenges on those grounds.
    pub challenges_by_grounds: JsonVal,
    /// Histogram of ChallengeVerdict values to count.
    /// Keys: "upheld" | "dismissed" | "superseded".
    /// Missing keys indicate zero outcomes of that verdict.
    pub outcomes_by_verdict: JsonVal,
}

/// Renderer kind a bundle targets. Mirrors `enums/renderer-kind.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum RendererKind {
    AngularSsr,
    ReactRsc,
    VueSsr,
    SvelteSsr,
    LitSsr,
    StaticHtml,
}

/// One bundle a doorway carries (mirrors `bundles[]` items in the profile schema).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct BundleEntry {
    pub name: String,
    pub version: String,
    pub renderer: RendererKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
}

/// Tier-1 render capability profile. View-layer Category C operational state,
/// layered into PeerStatusView post-construction. NOT a DHT entry.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RenderCapabilityProfile {
    pub bundles: Vec<BundleEntry>,
    pub renderers: Vec<RendererKind>,
    pub auth_modes: Vec<String>,
    pub max_concurrent_renders: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_budget_mib: Option<u32>,
}

/// Tier-2 extension capability claim (one entry in the extensions map).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CapabilityExtensionEntry {
    pub schema_ref: String,
    pub profile: JsonVal,
}

/// Advertised model-level capabilities of an elohim-agent running at this peer.
///
/// Wire format: `elohim/sdk/schemas/v1/views/elohim-capability-profile.schema.json`
///
/// Operator-configured at startup. Phase 10+ may auto-detect from
/// elohim-agent-service. All string arrays default to empty.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ElohimCapabilityProfile {
    /// Fully-qualified model name. E.g. `claude-opus-4-7`, `llama-3.1-70b-q4`.
    pub model_name: String,
    /// Model family/vendor. E.g. `claude`, `llama`, `gpt`, `mistral`.
    pub model_family: String,
    /// Maximum input context in tokens for this model instance.
    #[ts(type = "number")]
    pub context_window_tokens: u64,
    /// CID of the constitution document. Null when no specific constitution is applied.
    pub constitution_cid: Option<String>,
    /// Quantization descriptor. E.g. `q4_K_M`, `bf16`, `f32`. Null for hosted/closed models.
    pub quantization_spec: Option<String>,
    /// Free-form host descriptor. E.g. `tauri-desktop-mac-arm64`, `elohim-node-linux-x86_64`.
    pub deployment_context: Option<String>,
    /// Domains this elohim is primed for. Free-form in Phase 9.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub specialties: Vec<String>,
    /// Named gate/service capabilities this elohim can dispatch.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<String>,
    /// Observed strengths accrued from attestation history.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub strengths: Vec<String>,
    /// ISO 8601 timestamp when this elohim instance became active at this peer.
    pub active_since: String,
    /// Reach of this elohim instance. Distinct from the node's overall reach. Null until attestations accrue.
    pub reach_level: Option<String>,
}

/// Wire view for a peer's notarized status, as projected from the infrastructure DNA DHT.
///
/// Wire format: `elohim/sdk/schemas/v1/views/peer-status-view.schema.json`
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PeerStatusView {
    /// Agent pubkey of the peer (base64url encoded).
    pub peer_id: String,
    /// Peer availability status: "online" | "degraded" | "leaving" | "offline".
    pub status: String,
    /// True when this peer is available as a general-pool request target.
    pub general_pool_member: bool,
    /// True when this peer is accepting new stewardship reserve allocations.
    pub accepting_stewardship_reserves: bool,
    /// Operator-declared node archetype. E.g. home-nuc, blade, desktop, cloud-vps.
    pub archetype_class: Option<String>,
    /// Holochain action timestamp in microseconds since Unix epoch (u64 as string).
    pub timestamp: String,
    /// ActionHash of the upstream PeerStatus DHT entry — provenance anchor.
    pub dht_anchor_hash: String,
    /// Unix microseconds when this projection row was last written (u64 as string).
    pub updated_at: String,
    /// Model-level capabilities if an elohim-agent runs at this peer. None for pure storage/relay nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elohim_capability: Option<ElohimCapabilityProfile>,
    /// Render capability profile if a doorway co-located with this peer can SSR.
    /// Layered post-construction via build_peer_status_view; NOT in DHT entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_capability: Option<RenderCapabilityProfile>,
    /// Tier-2 extension capabilities. Layered post-construction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<CapabilityExtensions>,
}

/// Committed hardware resources declared by a node in its NodeRegistration DHT entry.
///
/// Wire format: embedded in `NodeShapeView`.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CommittedResources {
    pub cpu_cores: i32,
    pub memory_gb: i32,
    pub storage_tb: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bandwidth_mbps: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_custody_gb: Option<f64>,
    pub can_steward: bool,
    pub can_infer: bool,
    pub can_doorway: bool,
}

/// Wire view for a node's declared shape, as projected from the infrastructure DNA DHT.
///
/// Wire format: `elohim/sdk/schemas/v1/views/node-shape-view.schema.json`
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct NodeShapeView {
    pub node_id: String,
    pub hostname: String,
    pub device_archetype_id: String,
    pub household_id: String,
    pub role: String,
    pub capability_level: i32,
    pub committed: CommittedResources,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steward_tier: Option<String>,
    pub custodian_opt_in: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    pub signature: String,
    pub signed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dht_anchor_hash: Option<String>,
}

/// A single device entry combining node shape with live peer status (if online).
///
/// Wire format: embedded in `HouseholdDevicesView`.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct DeviceEntryView {
    pub shape: NodeShapeView,
    /// Live peer status; present when online, null when offline/unknown.
    pub peer: Option<PeerStatusView>,
}

/// Wire view listing all devices registered under a household.
///
/// Wire format: `elohim/sdk/schemas/v1/views/household-devices-view.schema.json`
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct HouseholdDevicesView {
    pub household_id: String,
    pub devices: Vec<DeviceEntryView>,
}

/// Aggregate network posture: counts over PeerStatus projection and
/// household reciprocation. Operational Category C — computed per-request,
/// no persistence. Source of truth: PeerStatus DHT entries (infrastructure
/// DNA) and stewarded_nodes / stewardship_allocations projections.
///
/// Wire format: `elohim/sdk/schemas/v1/views/network-posture-view.schema.json`
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct NetworkPostureView {
    pub total_peers: i32,
    pub active_peers: i32,
    pub stale_peers: i32,
    pub always_on_peers: i32,
    pub households_reciprocating: i32,
    pub compute_available: bool,
    pub storage_pressure: f32,
    pub computed_at: String,
}

/// Per-content household-first resilience claim. Source of truth:
/// computed projection over stewardship_allocations (Category A2 link
/// metadata on Agreement DHT entries), peer_statuses (PeerStatus DHT
/// entries), and stewarded_nodes (NodeRegistration projection). Category C
/// operational view — no persistence, no new entry types.
///
/// Wire format: `elohim/sdk/schemas/v1/views/household-resilience-view.schema.json`
#[derive(Debug, Clone, Serialize, Default, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct HouseholdResilienceView {
    pub content_id: String,
    pub households_stewarding: i32,
    pub households_reciprocated: i32,
    pub protection_status: String,
    pub details: HouseholdResilienceDetails,
}

#[derive(Debug, Clone, Serialize, Default, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct HouseholdResilienceDetails {
    #[serde(default)]
    pub steward_households: Vec<String>,
    pub online_peer_count: i32,
    pub health_score: f32,
}

/// Structured shefa signal: a content item's achieved placement falls short of
/// its requested stewarding-collective diversity. Source of truth: computed
/// projection from shard_locations + rea_commitments + humans → collectives.
/// Operational Category C — no DHT entry.
///
/// Wire format: `elohim/sdk/schemas/v1/views/placement-gap-view.schema.json`
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PlacementGapView {
    pub id: String,
    pub content_id: String,
    pub shard_hash: String,
    pub requested_steward_count: i32,
    pub achieved_steward_count: i32,
    pub contract_coverage: f32,
    pub gap_kind: String,
    pub first_seen_at: String,
    pub last_seen_at: String,
}

/// Regional distribution of steward peers across geographic tiers.
///
/// Wire format: embedded in `resilience-snapshot-view.schema.json`
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RegionalDistributionView {
    pub local: i32,
    pub regional: i32,
    pub global: i32,
    pub unknown: i32,
}

/// Per-content collective-general resilience snapshot. Collective-general:
/// stewards can be any collective kind (household, church, patron-circle,
/// DAO, …) — any group that holds DHT-notarized REA commitments.
/// Source of truth: computed projection. Operational Category C.
///
/// Wire format: `elohim/sdk/schemas/v1/views/resilience-snapshot-view.schema.json`
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ResilienceSnapshotView {
    pub content_id: String,
    pub stewarding_collectives: i32,
    pub commitment_backed_collectives: i32,
    pub diversity_score: f32,
    pub regional_distribution: RegionalDistributionView,
    pub placement_gaps: Vec<PlacementGapView>,
    pub protection_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reciprocating_collectives: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<ResilienceSnapshotDetailsView>,
}

/// A collective (of any kind) currently stewarding a content item.
///
/// Wire format: embedded in `resilience-snapshot-view.schema.json` details
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StewardingCollectiveEntry {
    pub id: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Optional detail layer for `ResilienceSnapshotView`. Source of truth:
/// computed projection. Operational Category C.
///
/// Wire format: embedded in `resilience-snapshot-view.schema.json` details
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ResilienceSnapshotDetailsView {
    pub stewarding_collectives: Vec<StewardingCollectiveEntry>,
    pub online_peer_count: i32,
    pub health_score: f32,
}

/// Replica health bucket for a CID's distribution summary.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "snake_case")]
pub enum ReplicaHealth {
    Healthy,
    AtRisk,
    Critical,
}

/// Reach class for a CID — mirrors the protocol Reach enum's content-distribution
/// classes. Drives target replica count and badge tier.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "snake_case")]
pub enum ReachClass {
    Private,
    Intimate,
    Household,
    Neighborhood,
    Collective,
    Community,
    District,
    Public,
}

/// Where the bytes for the current fetch were sourced from.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "snake_case")]
pub enum FetchSource {
    ProjectedViaDoorway,
    PeerDirect,
    LocalPantry,
}

/// Viewer's role for a CID.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "snake_case")]
pub enum MyRole {
    SoleReplica,
    Replica,
    ReplicaAndProjector,
    NotHosting,
}

/// Tagged peer-diversity hint for a CID's replica set. Tag/content layout
/// matches the schema's `{ kind, value }` envelope so downstream consumers can
/// branch on `kind` without untagged-enum heuristics. The `None` unit variant
/// serializes as `{"kind":"none","value":null}`.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum DiversityHint {
    /// Replicas span these region-metro tiers.
    RegionMetro(Vec<String>),
    /// Replicas span these household device archetypes.
    HouseholdArchetypes(Vec<String>),
    /// Replicas live within a single collective; carries member count.
    CollectiveMemberCount(u32),
    /// No diversity hint available.
    None,
}

/// Inline per-CID distribution payload hydrated onto EPR/content responses.
/// Operational (Category C) projection; not persisted.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct DistributionSummary {
    pub replica_count: u32,
    pub replica_target: u32,
    pub replica_health: ReplicaHealth,
    pub projector_count: u32,
    pub reach_class: ReachClass,
    pub diversity_hint: DiversityHint,
    pub this_fetch_source: FetchSource,
    pub last_verified_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub my_role: Option<MyRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reciprocity_hint: Option<i64>,
}

/// Hardware/deployment archetype carried on an AgentPeerBinding (Category A
/// from imagodei DHT). Mirrors the protocol enum at
/// `elohim/sdk/schemas/v1/enums/device-archetype.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq, Hash)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "snake_case")]
pub enum DeviceArchetype {
    Node,
    Desktop,
    Mobile,
    Steward,
}

/// Per-replica row in a CID's distribution-details view.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ReplicaPeer {
    pub peer_id: String,
    pub device_archetype: DeviceArchetype,
    pub last_seen_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hop_hint: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub household_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_tier: Option<String>,
}

/// Per-doorway projector row in a CID's distribution-details view.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ProjectorIdentity {
    pub doorway_hostname: String,
    pub last_ack_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_tier: Option<String>,
}

/// Lazy-fetched per-CID developer-grade distribution view. Strict superset of
/// `DistributionSummary`. Operational (Category C) projection; not persisted.
///
/// `reciprocityEdges` is intentionally omitted at T05 — it references
/// `PeerHouseholdEdge` which lands in T07. T07 (or a follow-up) extends both
/// the schema and this struct with the optional field.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct DistributionDetails {
    pub summary: DistributionSummary,
    pub replica_peers: Vec<ReplicaPeer>,
    pub projector_identities: Vec<ProjectorIdentity>,
    /// Open-shape placement-gap records during bring-up; will graduate to a
    /// typed schema once stable.
    pub placement_gaps: Vec<JsonVal>,
    /// Open-shape rea projection-event records relevant to this CID.
    pub recent_projection_events: Vec<JsonVal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commitment_references: Option<Vec<String>>,
}

/// Per-device summary in `MyClusterView`.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct DeviceSummary {
    pub peer_id: String,
    pub archetype: DeviceArchetype,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub online: bool,
    pub freshness: Freshness,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_used_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_used_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hosting_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projecting_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beacon_age_ms: Option<u64>,
}

/// Aggregated totals across the agent's devices.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct DeviceTotals {
    pub storage_used_bytes: u64,
    pub storage_total_bytes: u64,
    pub external_committed_bytes: u64,
    pub reciprocity_net_bytes: i64,
}

/// Federated cluster view of an agent's devices.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct MyClusterView {
    pub agent_cid: String,
    pub devices: Vec<DeviceSummary>,
    pub totals: DeviceTotals,
    pub freshness: Freshness,
}

/// Per-household reciprocation edge in a peer topology view.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct PeerHouseholdEdge {
    pub household_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub online: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync_sec: Option<u64>,
    pub my_cids_hosted_by_them: u32,
    pub their_cids_hosted_by_me: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_diff: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_critical_for_me: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub i_am_critical_for_them: Option<bool>,
}

/// Sole-replica risk row in a peer topology view.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ResilienceCliff {
    pub household_id: String,
    pub sole_replica_cid_count: u32,
}

/// Per-agent peer topology — reciprocation edges + resilience cliffs.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct PeerTopologyView {
    pub agent_cid: String,
    pub edges: Vec<PeerHouseholdEdge>,
    pub reciprocation_count: u32,
    pub resilience_cliffs: Vec<ResilienceCliff>,
    pub freshness: Freshness,
}

/// Direction of byte flow across a doorway federation edge.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "snake_case")]
pub enum FederationDirection {
    Bidirectional,
    OutboundOnly,
    InboundOnly,
}

/// Per-doorway federation row in a doorway dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct DashboardFederationPeer {
    pub doorway_hostname: String,
    pub online: bool,
    pub direction: FederationDirection,
    pub shared_cid_count: u32,
}

/// Projection cache + lag aggregate for a doorway.
///
/// `cache_hit_rate_24h` is `f64` so PartialEq only.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ProjectionCoverage {
    pub projected_cid_count: u32,
    pub known_cid_count: u32,
    pub cache_hit_rate_24h: f64,
    pub projection_lag_ms_avg: u64,
}

/// DNS/TLS/reachability surface for a doorway hostname.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct PublicSurfaceState {
    pub dns_resolves: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_target: Option<String>,
    pub tls_valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_expires_in_days: Option<i32>,
    pub public_reachable: bool,
}

/// Doorway operator dashboard view — composed at request time from cluster +
/// reciprocity views over view-federation/1.0.0.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct DoorwayDashboardView {
    pub doorway_hostname: String,
    pub storage_stewards: Vec<DashboardSteward>,
    pub federation_peers: Vec<DashboardFederationPeer>,
    pub projection_coverage: ProjectionCoverage,
    pub public_surface: PublicSurfaceState,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct Libp2pTransportProfileView {
    pub peer_id: String,
    pub addrs: Vec<String>,
    pub supports: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct IrohTransportProfileView {
    pub node_id: String,
    pub relays: Vec<String>,
    pub supports: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PeerTransportManifestView {
    pub agent_cid: String,
    pub libp2p: Option<Libp2pTransportProfileView>,
    pub iroh: Option<IrohTransportProfileView>,
    pub discovery: Vec<String>,
    pub capability_level: u8,
    pub last_observed: i64,
}

/// Response view for `PUT /blob/{hash}`.
///
/// Wire-compatible superset of the legacy `ShardManifest` JSON (all pre-existing
/// fields retained). Adds `blake3_hash` as an optional field populated by the
/// iroh dual-write path. Clients on the libp2p-only path receive `blake3_hash: null`.
///
/// Source of truth: `BlobStore` filesystem (SHA256-keyed, Category C operational)
/// and `IrohBlobStore` filesystem (BLAKE3-keyed, Category C operational). This
/// view is a projection of both for the single PUT response.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PutBlobResponseView {
    pub blob_hash: String,
    pub total_size: u64,
    pub mime_type: String,
    pub encoding: String,
    pub data_shards: u8,
    pub total_shards: u8,
    pub shard_size: u64,
    pub shard_hashes: Vec<String>,
    pub reach: String,
    pub author_id: Option<String>,
    pub created_at: String,
    pub verified_at: Option<String>,
    /// BLAKE3 hash the same bytes were written to in `IrohBlobStore`.
    /// `None` (serialises as `null`) when the iroh side is not configured or
    /// the dual-write failed — the SHA256 write still succeeded in that case.
    pub blake3_hash: Option<String>,
}

/// HTTP view of a single row from the `observations` table.
///
/// Source of truth for any observation is the per-observer iroh-blob log;
/// this projection is rebuildable by log replay. Classification: Category C
/// (operational, reconstructable from log_cid + log_offset).
///
/// `payloadJson` is forwarded as a raw JSON string — clients parse according
/// to `observationKind`. This preserves forward-compatibility: new payload
/// shapes do not require a Rust redeploy.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ObservationView {
    pub observer_cid: String,
    pub log_cid: String,
    pub log_offset: i64,
    pub observed_at: i64,
    pub seq: i64,
    pub observation_kind: String,
    pub subject_cid: Option<String>,
    pub subject_kind: Option<String>,
    pub payload_json: String,
    pub observer_household_cid: Option<String>,
    pub observer_collective_cid: Option<String>,
    pub observer_region: Option<String>,
    pub observer_archetype: Option<String>,
    pub observer_compute_class: Option<String>,
    pub signature_b64: String,
}

/// HTTP view of a row from the `observation_diversity_summary` SQLite view.
///
/// Diversity rollup over `observations` — one row per (subject_cid,
/// observation_kind) with distinct-counts across diversity dimensions. Used
/// by the graduation evaluator (Stage 5) to gate attestation graduation.
/// Classification: Category C (aggregation; reconstructable by re-querying
/// the observations table).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ObservationDiversitySummaryView {
    pub subject_cid: String,
    pub observation_kind: String,
    pub distinct_agents: i64,
    pub distinct_households: i64,
    pub distinct_collectives: i64,
    pub distinct_regions: i64,
    pub distinct_archetypes: i64,
    pub distinct_compute_classes: i64,
    pub total_count: i64,
    pub first_observed_at: i64,
    pub last_observed_at: i64,
}

