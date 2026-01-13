//! Core types for the EAE framework.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use constitution::ConstitutionalLayer;

/// An observation from the monitoring system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// Unique observation ID
    pub id: String,
    /// When the observation was made
    pub timestamp: DateTime<Utc>,
    /// Source of the observation
    pub source: ObservationSource,
    /// Type of observation
    pub observation_type: ObservationType,
    /// Raw data
    pub data: serde_json::Value,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f32,
    /// Related entity IDs
    pub related_entities: Vec<String>,
}

impl Observation {
    /// Create a new observation.
    pub fn new(source: ObservationSource, observation_type: ObservationType, data: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            source,
            observation_type,
            data,
            confidence: 1.0,
            related_entities: vec![],
        }
    }

    /// Set confidence level.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }
}

/// Source of an observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationSource {
    /// Direct user interaction
    UserInteraction,
    /// Content analysis
    ContentAnalysis,
    /// Network signal (from other agents)
    NetworkSignal,
    /// System metric
    SystemMetric,
    /// Scheduled check
    ScheduledCheck,
    /// External event
    ExternalEvent,
}

/// Type of observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationType {
    /// Behavior pattern detected
    BehaviorPattern,
    /// Content submitted for review
    ContentSubmission,
    /// Emotional state indicator
    EmotionalState,
    /// Constitutional compliance check
    ComplianceCheck,
    /// Resource usage metric
    ResourceUsage,
    /// Security event
    SecurityEvent,
    /// Governance proposal
    GovernanceProposal,
}

/// An event that triggers analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisEvent {
    /// Unique event ID
    pub id: String,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event_type: AnalysisEventType,
    /// Observations that triggered this event
    pub observations: Vec<Observation>,
    /// Priority level
    pub priority: EventPriority,
    /// Constitutional layer context
    pub layer_context: Option<ConstitutionalLayer>,
}

/// Type of analysis event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisEventType {
    /// Pattern threshold crossed
    ThresholdCrossed,
    /// Anomaly detected
    AnomalyDetected,
    /// Care flag raised
    CareFlagRaised,
    /// Constitutional boundary approached
    BoundaryApproached,
    /// Governance action required
    GovernanceRequired,
    /// Routine check
    RoutineCheck,
}

/// Priority level for events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventPriority {
    /// Background processing
    Low = 0,
    /// Standard priority
    Normal = 1,
    /// Should be processed soon
    High = 2,
    /// Immediate attention required
    Urgent = 3,
    /// Critical - may require intervention
    Critical = 4,
}

impl Default for EventPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// A decision made by the decider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    /// Unique decision ID
    pub id: String,
    /// When the decision was made
    pub timestamp: DateTime<Utc>,
    /// Analysis event that led to this decision
    pub event_id: String,
    /// Decision type
    pub decision_type: DecisionType,
    /// Actions to execute
    pub actions: Vec<Action>,
    /// Constitutional reasoning
    pub reasoning: DecisionReasoning,
    /// Confidence in the decision
    pub confidence: f32,
    /// Whether consensus is required
    pub requires_consensus: bool,
    /// Consensus status if required
    pub consensus_status: Option<ConsensusStatus>,
}

/// Type of decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionType {
    /// Allow without intervention
    Allow,
    /// Allow with monitoring
    AllowWithMonitoring,
    /// Intervene with soft guidance
    SoftIntervention,
    /// Hard intervention required
    HardIntervention,
    /// Block the action
    Block,
    /// Escalate to higher layer
    Escalate,
    /// Defer for human review
    DeferToHuman,
}

/// An action to be executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Unique action ID
    pub id: String,
    /// Action type
    pub action_type: ActionType,
    /// Target entity
    pub target: String,
    /// Action parameters
    pub params: HashMap<String, serde_json::Value>,
    /// Priority
    pub priority: EventPriority,
    /// Deadline (if any)
    pub deadline: Option<DateTime<Utc>>,
}

impl Action {
    /// Create a new action.
    pub fn new(action_type: ActionType, target: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            action_type,
            target: target.into(),
            params: HashMap::new(),
            priority: EventPriority::Normal,
            deadline: None,
        }
    }

    /// Add a parameter.
    pub fn with_param(mut self, key: &str, value: impl Serialize) -> Self {
        self.params.insert(
            key.to_string(),
            serde_json::to_value(value).unwrap_or_default(),
        );
        self
    }
}

/// Type of action to execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// Send a notification
    Notify,
    /// Log for audit
    Log,
    /// Invoke an Elohim capability
    InvokeCapability,
    /// Trigger a governance process
    TriggerGovernance,
    /// Apply content filter
    FilterContent,
    /// Flag for human review
    FlagForReview,
    /// Update user metrics
    UpdateMetrics,
    /// Store precedent
    StorePrecedent,
    /// Broadcast to network
    Broadcast,
    /// Request consensus
    RequestConsensus,
}

/// Reasoning behind a decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionReasoning {
    /// Primary constitutional principle applied
    pub primary_principle: String,
    /// How the principle was interpreted
    pub interpretation: String,
    /// Rules that matched (from rule engine)
    pub matched_rules: Vec<String>,
    /// Whether LLM was used for decision
    pub llm_assisted: bool,
    /// Precedents considered
    pub precedents_considered: Vec<String>,
    /// Constitutional layer that made the decision
    pub determining_layer: ConstitutionalLayer,
    /// Stack hash at decision time
    pub stack_hash: String,
}

/// Status of consensus gathering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusStatus {
    /// Required threshold (0.0 - 1.0)
    pub threshold: f32,
    /// Current agreement level
    pub current_agreement: f32,
    /// Participating agents
    pub participants: Vec<String>,
    /// Votes received
    pub votes: HashMap<String, ConsensusVote>,
    /// Deadline for consensus
    pub deadline: DateTime<Utc>,
    /// Whether consensus is reached
    pub reached: bool,
}

/// A vote in consensus gathering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusVote {
    /// Agent ID
    pub agent_id: String,
    /// Vote decision
    pub vote: VoteDecision,
    /// Reasoning for vote
    pub reasoning: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Vote decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoteDecision {
    /// Approve the action
    Approve,
    /// Reject the action
    Reject,
    /// Abstain from voting
    Abstain,
    /// Request more information
    NeedMoreInfo,
}

/// Result of action execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Action ID
    pub action_id: String,
    /// Whether execution succeeded
    pub success: bool,
    /// Result data
    pub result: Option<serde_json::Value>,
    /// Error message if failed
    pub error: Option<String>,
    /// Execution duration in ms
    pub duration_ms: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Error types for EAE.
#[derive(Debug, thiserror::Error)]
pub enum EaeError {
    /// Monitor error
    #[error("Monitor error: {0}")]
    MonitorError(String),

    /// Analysis error
    #[error("Analysis error: {0}")]
    AnalysisError(String),

    /// Decision error
    #[error("Decision error: {0}")]
    DecisionError(String),

    /// Execution error
    #[error("Execution error: {0}")]
    ExecutionError(String),

    /// Consensus error
    #[error("Consensus error: {0}")]
    ConsensusError(String),

    /// Constitutional violation
    #[error("Constitutional violation: {0}")]
    ConstitutionalViolation(String),

    /// Agent service error
    #[error("Agent service error: {0}")]
    AgentServiceError(#[from] elohim_agent::service::ServiceError),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, EaeError>;
