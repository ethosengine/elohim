//! Configuration for the EAE framework.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for an Elohim Autonomous Entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EaeConfig {
    /// Entity ID
    pub entity_id: String,
    /// Monitor configuration
    pub monitor: MonitorConfig,
    /// Analyzer configuration
    pub analyzer: AnalyzerConfig,
    /// Decider configuration
    pub decider: DeciderConfig,
    /// Executor configuration
    pub executor: ExecutorConfig,
    /// Consensus configuration
    pub consensus: ConsensusConfig,
    /// General settings
    pub general: GeneralConfig,
}

impl Default for EaeConfig {
    fn default() -> Self {
        Self {
            entity_id: uuid::Uuid::new_v4().to_string(),
            monitor: MonitorConfig::default(),
            analyzer: AnalyzerConfig::default(),
            decider: DeciderConfig::default(),
            executor: ExecutorConfig::default(),
            consensus: ConsensusConfig::default(),
            general: GeneralConfig::default(),
        }
    }
}

impl EaeConfig {
    /// Create a new config with entity ID.
    pub fn new(entity_id: impl Into<String>) -> Self {
        Self {
            entity_id: entity_id.into(),
            ..Default::default()
        }
    }

    /// Load config from YAML file.
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Serialize to YAML.
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

/// Monitor configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    /// Maximum observations to keep in buffer
    pub buffer_size: usize,
    /// Observation retention period (seconds)
    pub retention_secs: u64,
    /// Sampling rate for high-volume sources (0.0 - 1.0)
    pub sampling_rate: f32,
    /// Enable behavior pattern tracking
    pub track_behavior_patterns: bool,
    /// Enable emotional state monitoring
    pub track_emotional_state: bool,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            buffer_size: 10_000,
            retention_secs: 3600, // 1 hour
            sampling_rate: 1.0,
            track_behavior_patterns: true,
            track_emotional_state: true,
        }
    }
}

/// Analyzer configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerConfig {
    /// Analysis window size (seconds)
    pub window_secs: u64,
    /// Minimum observations for pattern detection
    pub min_observations: usize,
    /// Spiral detection threshold
    pub spiral_threshold: f32,
    /// Manipulation detection threshold
    pub manipulation_threshold: f32,
    /// Drift detection threshold
    pub drift_threshold: f32,
    /// Enable anomaly detection
    pub enable_anomaly_detection: bool,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            window_secs: 300, // 5 minutes
            min_observations: 5,
            spiral_threshold: 0.7,
            manipulation_threshold: 0.8,
            drift_threshold: 0.6,
            enable_anomaly_detection: true,
        }
    }
}

/// Decider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeciderConfig {
    /// Path to rules file
    pub rules_path: Option<String>,
    /// LLM fallback threshold (use LLM if confidence below this)
    pub llm_fallback_threshold: f32,
    /// Maximum decision time (ms)
    pub max_decision_time_ms: u64,
    /// Enable precedent lookup
    pub use_precedents: bool,
    /// Consensus threshold for risky decisions
    pub consensus_threshold: f32,
    /// Decisions requiring consensus
    pub require_consensus_for: Vec<String>,
}

impl Default for DeciderConfig {
    fn default() -> Self {
        Self {
            rules_path: None,
            llm_fallback_threshold: 0.6,
            max_decision_time_ms: 5000,
            use_precedents: true,
            consensus_threshold: 0.67,
            require_consensus_for: vec![
                "hard_intervention".to_string(),
                "block".to_string(),
            ],
        }
    }
}

/// Executor configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    /// Maximum concurrent actions
    pub max_concurrent: usize,
    /// Action timeout (ms)
    pub action_timeout_ms: u64,
    /// Retry count for failed actions
    pub retry_count: usize,
    /// Retry delay (ms)
    pub retry_delay_ms: u64,
    /// Enable action batching
    pub enable_batching: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            action_timeout_ms: 30_000,
            retry_count: 3,
            retry_delay_ms: 1000,
            enable_batching: true,
        }
    }
}

/// Consensus configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Minimum participants for valid consensus
    pub min_participants: usize,
    /// Consensus timeout (seconds)
    pub timeout_secs: u64,
    /// Default approval threshold
    pub default_threshold: f32,
    /// Peer agent URLs
    pub peer_agents: Vec<String>,
    /// Enable peer discovery
    pub enable_peer_discovery: bool,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            min_participants: 3,
            timeout_secs: 60,
            default_threshold: 0.67,
            peer_agents: vec![],
            enable_peer_discovery: true,
        }
    }
}

/// General configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Enable audit logging
    pub audit_enabled: bool,
    /// Log level
    pub log_level: String,
    /// Metrics endpoint
    pub metrics_endpoint: Option<String>,
    /// Health check interval (seconds)
    pub health_check_interval_secs: u64,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            audit_enabled: true,
            log_level: "info".to_string(),
            metrics_endpoint: None,
            health_check_interval_secs: 30,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EaeConfig::default();
        assert_eq!(config.monitor.buffer_size, 10_000);
        assert_eq!(config.analyzer.window_secs, 300);
        assert!(config.general.audit_enabled);
    }

    #[test]
    fn test_yaml_roundtrip() {
        let config = EaeConfig::new("test-entity");
        let yaml = config.to_yaml().unwrap();
        let parsed: EaeConfig = EaeConfig::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.entity_id, "test-entity");
    }
}
