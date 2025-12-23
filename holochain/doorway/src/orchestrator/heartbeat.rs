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
    shutdown_rx: Option<mpsc::Receiver<()>>,
}

impl HeartbeatLoop {
    /// Create new heartbeat loop
    pub fn new(config: HeartbeatConfig, state: Arc<OrchestratorState>) -> Self {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        Self {
            config,
            state,
            shutdown_tx: Some(shutdown_tx),
            shutdown_rx: Some(shutdown_rx),
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
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub storage_usage_tb: f64,
    pub active_connections: u32,
    pub custodied_content_gb: f64,
}

/// Run the heartbeat monitoring loop
async fn run_heartbeat_loop(config: HeartbeatConfig, state: Arc<OrchestratorState>) {
    let interval = std::time::Duration::from_secs(config.interval_secs);

    loop {
        tokio::time::sleep(interval).await;

        // Check for failed nodes
        let failed_nodes = state.get_failed_nodes().await;

        if !failed_nodes.is_empty() {
            warn!(
                count = failed_nodes.len(),
                "Detected failed nodes"
            );

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
        debug!(
            online = online_nodes.len(),
            "Heartbeat check complete"
        );
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
    /// Calculate health score from metrics
    pub fn calculate_health_score(&self) -> f64 {
        let total_pings = self.successful_pings + self.failed_pings;
        if total_pings == 0 {
            return 0.0;
        }

        let ping_ratio = self.successful_pings as f64 / total_pings as f64;

        // Weight: 70% peer attestations, 30% self-reported heartbeat
        let heartbeat_factor = if self.self_reported.is_some() { 0.3 } else { 0.0 };

        (ping_ratio * 0.7) + heartbeat_factor
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
            }),
            peer_attestations: 5,
            successful_pings: 9,
            failed_pings: 1,
            health_score: 0.0,
            status: NodeHealthStatus::Unknown,
        };

        let score = aggregation.calculate_health_score();
        assert!(score > 0.9); // 90% success + heartbeat bonus

        let status = aggregation.determine_status();
        assert_eq!(status, NodeHealthStatus::Healthy);
    }

    #[test]
    fn test_degraded_status() {
        let aggregation = NodeHealthAggregation {
            node_id: "test-node".to_string(),
            self_reported: None,
            peer_attestations: 3,
            successful_pings: 7,
            failed_pings: 3,
            health_score: 0.0,
            status: NodeHealthStatus::Unknown,
        };

        let status = aggregation.determine_status();
        assert_eq!(status, NodeHealthStatus::Degraded);
    }

    #[tokio::test]
    async fn test_heartbeat_loop_creation() {
        let config = HeartbeatConfig::default();
        let state = Arc::new(OrchestratorState::new(Default::default()));
        let _loop = HeartbeatLoop::new(config, state);
    }
}
