//! Heartbeat loop for node liveness monitoring
//!
//! ## Overview
//!
//! The heartbeat system provides:
//! 1. Periodic liveness checks for all registered nodes
//! 2. Health status aggregation from peer attestations
//! 3. Failure detection after missed heartbeats
//! 4. Disaster recovery triggering for failed nodes
//!
//! ## Protocol
//!
//! Nodes send heartbeats via NATS to `WORKLOAD.orchestrator.heartbeat`
//! containing current resource usage metrics.
//!
//! Orchestrator:
//! 1. Tracks last heartbeat time per node
//! 2. Marks nodes as degraded after 1 missed heartbeat
//! 3. Marks nodes as failed after N missed heartbeats
//! 4. Triggers disaster recovery for failed nodes

use super::OrchestratorState;
use crate::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Heartbeat configuration
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    /// Interval between heartbeat checks (seconds)
    pub interval_secs: u64,
    /// Number of missed heartbeats before marking node as failed
    pub failure_threshold: u32,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval_secs: 30,
            failure_threshold: 3,
        }
    }
}

/// Heartbeat loop service
pub struct HeartbeatLoop {
    config: HeartbeatConfig,
    state: Arc<OrchestratorState>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    _shutdown_rx: Option<mpsc::Receiver<()>>,
}

impl HeartbeatLoop {
    /// Create new heartbeat loop
    pub fn new(config: HeartbeatConfig, state: Arc<OrchestratorState>) -> Self {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        Self {
            config,
            state,
            shutdown_tx: Some(shutdown_tx),
            _shutdown_rx: Some(shutdown_rx),
        }
    }

    /// Start the heartbeat monitoring loop
    pub async fn start(&self) -> Result<()> {
        info!(
            interval_secs = self.config.interval_secs,
            failure_threshold = self.config.failure_threshold,
            "Starting heartbeat monitor"
        );

        let state = Arc::clone(&self.state);
        let config = self.config.clone();

        // Spawn heartbeat monitor task
        tokio::spawn(async move {
            run_heartbeat_loop(config, state).await;
        });

        Ok(())
    }

    /// Stop the heartbeat loop
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping heartbeat monitor");
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(()).await;
        }
        Ok(())
    }

    /// Process an incoming heartbeat from a node
    pub async fn process_heartbeat(&self, heartbeat: HeartbeatMessage) -> Result<()> {
        debug!(
            node_id = %heartbeat.node_id,
            cpu = %heartbeat.cpu_usage_percent,
            memory = %heartbeat.memory_usage_percent,
            "Processing heartbeat"
        );

        // Update last heartbeat time in state
        self.state.update_heartbeat(&heartbeat.node_id).await?;

        // Store heartbeat in DNA
        // In production, call heartbeat() zome function

        Ok(())
    }
}

/// Heartbeat message from nodes
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatMessage {
    pub node_id: String,
    pub timestamp: String,
    // Resource metrics
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub storage_usage_tb: f64,
    pub active_connections: u32,
    pub custodied_content_gb: f64,
    // Human-scale metrics (optional for backward compat)
    #[serde(default)]
    pub social_metrics: Option<SocialMetrics>,
}

/// Human-scale metrics for nodes
///
/// These metrics connect technical infrastructure to social relationships:
/// - Steward tier reflects commitment level to the network
/// - Reach levels indicate what content this node can serve
/// - Trust score reflects reliability based on peer attestations
/// - Humans served shows impact in the network
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SocialMetrics {
    /// Steward tier: caretaker (0) → guardian (1) → steward (2) → pioneer (3)
    pub steward_tier: String,
    /// Maximum reach level this node serves (0=private, 7=commons)
    pub max_reach_level: u8,
    /// Reach levels actively being served
    pub active_reach_levels: Vec<u8>,
    /// Trust score from peer attestations (0.0 - 1.0)
    pub trust_score: f64,
    /// Number of unique humans served in last period
    pub humans_served: u32,
    /// Content pieces actively custodied
    pub content_custodied: u32,
    /// Successful content deliveries in last period
    pub successful_deliveries: u32,
    /// Failed content deliveries in last period
    pub failed_deliveries: u32,
}

impl Default for SocialMetrics {
    fn default() -> Self {
        Self {
            steward_tier: "caretaker".to_string(),
            max_reach_level: 7, // commons by default
            active_reach_levels: vec![7],
            trust_score: 0.5, // neutral starting point
            humans_served: 0,
            content_custodied: 0,
            successful_deliveries: 0,
            failed_deliveries: 0,
        }
    }
}

impl SocialMetrics {
    /// Calculate delivery success rate
    pub fn delivery_success_rate(&self) -> f64 {
        let total = self.successful_deliveries + self.failed_deliveries;
        if total == 0 {
            return 1.0; // No deliveries = no failures
        }
        self.successful_deliveries as f64 / total as f64
    }

    /// Calculate human-scale impact score (0.0 - 1.0)
    ///
    /// Weighs multiple factors:
    /// - 30% steward tier (commitment)
    /// - 25% trust score (reliability)
    /// - 25% delivery success rate (performance)
    /// - 20% reach breadth (accessibility)
    pub fn impact_score(&self) -> f64 {
        let tier_score = match self.steward_tier.as_str() {
            "pioneer" => 1.0,
            "steward" => 0.75,
            "guardian" => 0.5,
            _ => 0.25, // caretaker
        };

        let reach_score = self.active_reach_levels.len() as f64 / 8.0;
        let delivery_score = self.delivery_success_rate();

        (tier_score * 0.30)
            + (self.trust_score * 0.25)
            + (delivery_score * 0.25)
            + (reach_score * 0.20)
    }
}

/// Run the heartbeat monitoring loop
async fn run_heartbeat_loop(config: HeartbeatConfig, state: Arc<OrchestratorState>) {
    let interval = std::time::Duration::from_secs(config.interval_secs);

    loop {
        tokio::time::sleep(interval).await;

        // Check for failed nodes
        let failed_nodes = state.get_failed_nodes().await;

        if !failed_nodes.is_empty() {
            warn!(count = failed_nodes.len(), "Detected failed nodes");

            for node_id in failed_nodes {
                error!(node_id = %node_id, "Node marked as failed - triggering disaster recovery");

                // Trigger disaster recovery
                // In production, call trigger_disaster_recovery() on Node Registry DNA
                if let Err(e) = trigger_disaster_recovery_for_node(&node_id, &state).await {
                    error!(
                        node_id = %node_id,
                        error = %e,
                        "Failed to trigger disaster recovery"
                    );
                }
            }
        }

        // Log health summary
        let online_nodes = state.get_online_nodes().await;
        debug!(online = online_nodes.len(), "Heartbeat check complete");
    }
}

/// Trigger disaster recovery for a failed node
async fn trigger_disaster_recovery_for_node(
    node_id: &str,
    _state: &OrchestratorState,
) -> Result<()> {
    info!(node_id = %node_id, "Triggering disaster recovery");

    // In production:
    // 1. Call trigger_disaster_recovery(node_id) on Node Registry DNA
    // 2. DNA will find content custodied by this node
    // 3. DNA emits ReplicateContent signals for each piece of content
    // 4. DisasterRecoveryCoordinator handles actual replication

    // Placeholder for DNA call
    // let app_ws = connect_to_holochain().await?;
    // app_ws.call_zome({
    //     cell_id: node_registry_cell_id,
    //     zome_name: "node_registry_coordinator",
    //     fn_name: "trigger_disaster_recovery",
    //     payload: node_id,
    // }).await?;

    debug!(node_id = %node_id, "Disaster recovery triggered (simulated)");
    Ok(())
}

/// Health status aggregation from multiple sources
///
/// Combines technical metrics with human-scale impact metrics
/// to create a holistic view of node health and contribution.
#[derive(Debug, Clone)]
pub struct NodeHealthAggregation {
    pub node_id: String,
    /// Self-reported status from heartbeats
    pub self_reported: Option<HeartbeatMessage>,
    /// Peer attestations count
    pub peer_attestations: u32,
    /// Successful peer pings
    pub successful_pings: u32,
    /// Failed peer pings
    pub failed_pings: u32,
    /// Calculated health score (0.0 - 1.0)
    pub health_score: f64,
    /// Human-scale impact score (0.0 - 1.0)
    pub impact_score: f64,
    /// Final status determination
    pub status: NodeHealthStatus,
}

/// Health status enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeHealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl NodeHealthAggregation {
    /// Calculate technical health score from metrics (0.0 - 1.0)
    pub fn calculate_health_score(&self) -> f64 {
        let total_pings = self.successful_pings + self.failed_pings;
        if total_pings == 0 {
            return 0.0;
        }

        let ping_ratio = self.successful_pings as f64 / total_pings as f64;

        // Weight: 70% peer attestations, 30% self-reported heartbeat
        let heartbeat_factor = if self.self_reported.is_some() {
            0.3
        } else {
            0.0
        };

        (ping_ratio * 0.7) + heartbeat_factor
    }

    /// Calculate human-scale impact score (0.0 - 1.0)
    pub fn calculate_impact_score(&self) -> f64 {
        match &self.self_reported {
            Some(hb) => match &hb.social_metrics {
                Some(social) => social.impact_score(),
                None => 0.5, // No social metrics = neutral
            },
            None => 0.0,
        }
    }

    /// Calculate combined score (health + impact)
    ///
    /// The combined score weighs both technical reliability
    /// and human-scale contribution:
    /// - 60% technical health (uptime, ping success)
    /// - 40% human-scale impact (trust, reach, stewardship)
    pub fn combined_score(&self) -> f64 {
        let health = self.calculate_health_score();
        let impact = self.calculate_impact_score();
        (health * 0.60) + (impact * 0.40)
    }

    /// Determine status from health score
    pub fn determine_status(&self) -> NodeHealthStatus {
        let score = self.calculate_health_score();

        if score >= 0.9 {
            NodeHealthStatus::Healthy
        } else if score >= 0.6 {
            NodeHealthStatus::Degraded
        } else if score > 0.0 {
            NodeHealthStatus::Unhealthy
        } else {
            NodeHealthStatus::Unknown
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_score_calculation() {
        let aggregation = NodeHealthAggregation {
            node_id: "test-node".to_string(),
            self_reported: Some(HeartbeatMessage {
                node_id: "test-node".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                cpu_usage_percent: 50.0,
                memory_usage_percent: 60.0,
                storage_usage_tb: 0.5,
                active_connections: 10,
                custodied_content_gb: 100.0,
                social_metrics: Some(SocialMetrics {
                    steward_tier: "guardian".to_string(),
                    max_reach_level: 7,
                    active_reach_levels: vec![5, 6, 7],
                    trust_score: 0.9,
                    humans_served: 50,
                    content_custodied: 100,
                    successful_deliveries: 950,
                    failed_deliveries: 50,
                }),
            }),
            peer_attestations: 5,
            successful_pings: 9,
            failed_pings: 1,
            health_score: 0.0,
            impact_score: 0.0,
            status: NodeHealthStatus::Unknown,
        };

        let score = aggregation.calculate_health_score();
        assert!(score > 0.9); // 90% success + heartbeat bonus

        let impact = aggregation.calculate_impact_score();
        assert!(impact > 0.5); // Guardian tier with good metrics

        let combined = aggregation.combined_score();
        assert!(combined > 0.7); // Combined health + impact

        let status = aggregation.determine_status();
        assert_eq!(status, NodeHealthStatus::Healthy);
    }

    #[test]
    fn test_degraded_status() {
        let aggregation = NodeHealthAggregation {
            node_id: "test-node".to_string(),
            self_reported: None,
            peer_attestations: 3,
            successful_pings: 9,
            failed_pings: 1,
            health_score: 0.0,
            impact_score: 0.0,
            status: NodeHealthStatus::Unknown,
        };

        let status = aggregation.determine_status();
        assert_eq!(status, NodeHealthStatus::Degraded);
    }

    #[test]
    fn test_social_metrics_impact_score() {
        // Pioneer with perfect metrics
        let pioneer = SocialMetrics {
            steward_tier: "pioneer".to_string(),
            max_reach_level: 7,
            active_reach_levels: vec![0, 1, 2, 3, 4, 5, 6, 7], // All reach levels
            trust_score: 1.0,
            humans_served: 1000,
            content_custodied: 500,
            successful_deliveries: 10000,
            failed_deliveries: 0,
        };
        assert!(pioneer.impact_score() > 0.9);

        // Caretaker with minimal metrics
        let caretaker = SocialMetrics::default();
        assert!(caretaker.impact_score() < 0.5);
    }

    #[tokio::test]
    async fn test_heartbeat_loop_creation() {
        let config = HeartbeatConfig::default();
        let state = Arc::new(OrchestratorState::new(Default::default()));
        let _loop = HeartbeatLoop::new(config, state);
    }
}
