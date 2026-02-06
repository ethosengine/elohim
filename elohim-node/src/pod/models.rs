//! Pod models - shared types for cluster orchestration
//!
//! Core types for observations, actions, consensus, and agent communication.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Unique identifier for an action
pub type ActionId = String;

/// Unique identifier for an agent in the network
pub type AgentId = String;

/// Unique identifier for a peer node
pub type PeerId = String;

/// Unique identifier for a request
pub type RequestId = String;

//=============================================================================
// OBSERVATIONS
//=============================================================================

/// A point-in-time observation of system state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// When this observation was made
    pub timestamp: u64,
    /// Which node made the observation
    pub node_id: String,
    /// The type of observation
    pub kind: ObservationKind,
    /// Observation-specific data
    pub data: serde_json::Value,
}

/// Categories of observations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ObservationKind {
    /// System resource metrics
    SystemMetrics,
    /// Node health conditions
    NodeConditions,
    /// Service status changes
    ServiceHealth,
    /// Peer discovery/loss
    PeerEvent,
    /// Sync state changes
    SyncEvent,
    /// Storage state changes
    StorageEvent,
    /// Cache statistics
    CacheStats,
    /// Network traffic patterns
    NetworkPattern,
    /// Error or anomaly detected
    Anomaly,
}

/// System metrics observation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetricsData {
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub disk_percent: f32,
    pub disk_available_bytes: u64,
    pub memory_available_bytes: u64,
    pub load_average: [f64; 3],
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}

/// Node condition change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionChangeData {
    pub condition: String,
    pub previous: bool,
    pub current: bool,
    pub reason: String,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatsData {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f32,
    pub size_bytes: u64,
    pub entry_count: usize,
}

/// Anomaly detection data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyData {
    pub anomaly_type: AnomalyType,
    pub severity: Severity,
    pub description: String,
    pub suggested_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyType {
    ResourceSpike,
    ServiceFailure,
    PeerLoss,
    SyncStall,
    StorageCorruption,
    UnusualTraffic,
    ConfigDrift,
}

//=============================================================================
// ACTIONS
//=============================================================================

/// An action the pod can take
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Unique action ID
    pub id: ActionId,
    /// When the action was created
    pub created_at: u64,
    /// The type of action
    pub kind: ActionKind,
    /// Risk level determines consensus requirements
    pub risk: ActionRisk,
    /// Current status
    pub status: ActionStatus,
    /// Human-readable description
    pub description: String,
    /// Action-specific parameters
    pub params: serde_json::Value,
    /// Result after execution
    pub result: Option<ActionResult>,
}

/// Categories of actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    // Storage actions
    ReplicateBlob,
    EvictBlob,
    RebuildShard,
    RebalanceStorage,

    // Cache actions
    ResizeCache,
    WarmCache,
    FlushCache,
    ChangeCachePolicy,

    // Config actions
    SetLogLevel,
    EnableTracing,
    UpdateSetting,
    ReloadConfig,

    // Debug actions
    CaptureHeapDump,
    CollectDiagnostics,
    ReportBug,

    // Recovery actions
    RestartService,
    ReconnectPeer,
    FailoverService,
    QuarantineNode,

    // Workload actions
    RedirectClients,
    ThrottleSync,
    ShardQuery,
}

/// Risk level of an action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionRisk {
    /// Execute immediately, no approval needed
    /// Examples: adjust log level, warm cache
    Safe,

    /// Requires N-of-M independent agent evaluations to approve
    /// Examples: evict data, quarantine node, failover
    Risky {
        required_approvals: u8,
        total_evaluators: u8,
    },
}

/// Current status of an action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    /// Waiting for consensus (risky actions)
    PendingConsensus,
    /// Approved and queued for execution
    Queued,
    /// Currently executing
    InProgress,
    /// Successfully completed
    Completed,
    /// Failed to execute
    Failed,
    /// Rejected by consensus
    Rejected,
    /// Cancelled by operator
    Cancelled,
}

/// Result of an action execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub success: bool,
    pub message: String,
    pub duration_ms: u64,
    pub details: Option<serde_json::Value>,
}

//=============================================================================
// CONSENSUS
//=============================================================================

/// Request for consensus on a risky action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusRequest {
    /// The action requiring consensus
    pub action: Action,
    /// Context about the cluster state
    pub context: ClusterContext,
    /// Agent proposing the action
    pub proposing_agent: AgentId,
    /// Deadline for responses
    pub deadline: u64,
}

/// Response from an agent evaluating a consensus request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResponse {
    /// Agent that evaluated
    pub evaluator: AgentId,
    /// Whether the agent approves
    pub approved: bool,
    /// Natural language explanation of the decision
    pub reasoning: String,
    /// Confidence in the decision (0.0-1.0)
    pub confidence: f32,
}

/// Context about cluster state for consensus decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterContext {
    /// Cluster name
    pub cluster_name: String,
    /// Number of nodes in cluster
    pub node_count: usize,
    /// Healthy node count
    pub healthy_nodes: usize,
    /// Recent observations across cluster
    pub recent_observations: Vec<Observation>,
    /// Current resource utilization
    pub resource_summary: ResourceSummary,
    /// Active issues/anomalies
    pub active_issues: Vec<String>,
}

/// Summary of cluster resource utilization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSummary {
    pub avg_cpu_percent: f32,
    pub avg_memory_percent: f32,
    pub avg_disk_percent: f32,
    pub total_storage_bytes: u64,
    pub used_storage_bytes: u64,
    pub total_blob_count: u64,
    pub connected_clients: usize,
}

//=============================================================================
// AGENT PROTOCOL
//=============================================================================

/// Messages for inter-agent P2P communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentMessage {
    // Observations (always shared)
    ShareObservation(Observation),
    RequestObservations {
        since: u64,
    },
    ObservationBatch {
        observations: Vec<Observation>,
    },

    // Inference requests (LLM - stubbed)
    RequestInference {
        prompt: String,
        context: ClusterContext,
        response_to: PeerId,
    },
    InferenceResult {
        request_id: RequestId,
        response: String,
        model_used: String,
    },

    // Consensus for risky actions
    ConsensusRequest(ConsensusRequest),
    ConsensusResponse(ConsensusResponse),

    // Capability discovery
    AdvertiseCapabilities {
        has_local_inference: bool,
        inference_model: Option<String>,
        available_compute: ComputeCapability,
    },

    // Heartbeat / liveness
    Ping {
        timestamp: u64,
    },
    Pong {
        timestamp: u64,
        node_id: String,
    },
}

/// Compute capabilities of a node (for agent placement)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeCapability {
    /// Available GPU memory in bytes (0 if none)
    pub gpu_memory_bytes: u64,
    /// GPU model name if available
    pub gpu_model: Option<String>,
    /// NPU available (Apple Silicon, etc.)
    pub has_npu: bool,
    /// CPU cores available for inference
    pub inference_cpu_cores: u8,
    /// Estimated tokens/second for local inference
    pub estimated_tokens_per_sec: Option<f32>,
}

impl Default for ComputeCapability {
    fn default() -> Self {
        Self {
            gpu_memory_bytes: 0,
            gpu_model: None,
            has_npu: false,
            inference_cpu_cores: 2,
            estimated_tokens_per_sec: None,
        }
    }
}

//=============================================================================
// RULES
//=============================================================================

/// A rule for automatic decision making
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Rule name
    pub name: String,
    /// Whether rule is enabled
    pub enabled: bool,
    /// Condition that triggers the rule
    pub condition: Condition,
    /// Action to take when triggered
    pub action_template: ActionTemplate,
    /// Rule priority (higher = evaluated first)
    pub priority: u8,
    /// Minimum seconds between rule activations
    pub cooldown_secs: u64,
    /// Last time this rule was triggered
    #[serde(skip)]
    pub last_triggered: Option<u64>,
}

/// Conditions for rule evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    /// Metric exceeds threshold
    MetricAbove {
        metric: MetricName,
        threshold: f64,
    },
    /// Metric below threshold
    MetricBelow {
        metric: MetricName,
        threshold: f64,
    },
    /// Service in specific state
    ServiceState {
        service: String,
        state: ServiceState,
    },
    /// Node condition active
    ConditionActive {
        condition: String,
    },
    /// Multiple conditions (all must be true)
    And {
        conditions: Vec<Condition>,
    },
    /// Multiple conditions (any must be true)
    Or {
        conditions: Vec<Condition>,
    },
    /// Negate a condition
    Not {
        condition: Box<Condition>,
    },
}

/// Metrics that can be used in conditions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetricName {
    CpuPercent,
    MemoryPercent,
    DiskPercent,
    DiskAvailableGb,
    MemoryAvailableMb,
    LoadAverage1m,
    LoadAverage5m,
    CacheHitRate,
    BlobAccessRate,
    SyncLagSeconds,
    PeerCount,
    ClientCount,
}

/// Service states for conditions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceState {
    Running,
    Stopped,
    Failed,
    Degraded,
}

/// Template for generating an action from a rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionTemplate {
    pub kind: ActionKind,
    pub risk: ActionRisk,
    pub description: String,
    pub params: serde_json::Value,
}

//=============================================================================
// SEVERITY
//=============================================================================

/// Severity levels for anomalies and alerts
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

//=============================================================================
// POD STATUS
//=============================================================================

/// Current status of the pod
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodStatus {
    /// Whether the pod is active
    pub active: bool,
    /// Node ID this pod runs on
    pub node_id: String,
    /// When the pod started
    pub started_at: u64,
    /// Number of actions executed
    pub actions_executed: u64,
    /// Number of actions pending
    pub actions_pending: usize,
    /// Connected peer pods
    pub peer_pods: Vec<PeerPodInfo>,
    /// Whether local inference is available
    pub has_local_inference: bool,
    /// Remote inference endpoint if using
    pub remote_inference_endpoint: Option<String>,
    /// Last decision timestamp
    pub last_decision_at: Option<u64>,
    /// Active rules count
    pub active_rules: usize,
    /// Current mode
    pub mode: PodMode,
}

/// Information about a peer pod
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerPodInfo {
    pub node_id: String,
    pub peer_id: String,
    pub last_seen: u64,
    pub compute_capability: ComputeCapability,
}

/// Operating mode of the pod
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PodMode {
    /// Normal operation
    Active,
    /// Only safe actions, no inference
    Degraded,
    /// Manual approval required for all actions
    Manual,
    /// Pod is disabled
    Disabled,
}

//=============================================================================
// HELPERS
//=============================================================================

impl Action {
    pub fn new(kind: ActionKind, description: impl Into<String>, params: serde_json::Value) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: now,
            kind,
            risk: ActionRisk::Safe,
            status: ActionStatus::Queued,
            description: description.into(),
            params,
            result: None,
        }
    }

    pub fn with_risk(mut self, risk: ActionRisk) -> Self {
        let is_risky = matches!(risk, ActionRisk::Risky { .. });
        self.risk = risk;
        if is_risky {
            self.status = ActionStatus::PendingConsensus;
        }
        self
    }
}

impl Observation {
    pub fn new(node_id: impl Into<String>, kind: ObservationKind, data: serde_json::Value) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            timestamp: now,
            node_id: node_id.into(),
            kind,
            data,
        }
    }
}

impl Default for PodStatus {
    fn default() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            active: false,
            node_id: String::new(),
            started_at: now,
            actions_executed: 0,
            actions_pending: 0,
            peer_pods: Vec::new(),
            has_local_inference: false,
            remote_inference_endpoint: None,
            last_decision_at: None,
            active_rules: 0,
            mode: PodMode::Disabled,
        }
    }
}

impl Rule {
    /// Check if the rule is on cooldown
    pub fn on_cooldown(&self, now: u64) -> bool {
        if let Some(last) = self.last_triggered {
            now < last + self.cooldown_secs
        } else {
            false
        }
    }
}
