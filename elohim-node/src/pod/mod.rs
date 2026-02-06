//! Pod - Cluster Operator for elohim-node
//!
//! The pod is an autonomous cluster intelligence layer that monitors, analyzes,
//! decides, and acts to optimize the local cluster. Think "Claude Code for P2P
//! cluster operations".
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                          POD ORCHESTRATOR                        │
//! │                                                                   │
//! │   ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
//! │   │ Monitor │──│ Analyze │──│ Decide  │──│ Execute │           │
//! │   └─────────┘  └─────────┘  └─────────┘  └─────────┘           │
//! │                                  │                               │
//! │                          ┌───────┴───────┐                       │
//! │                          │   Consensus   │                       │
//! │                          └───────────────┘                       │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Responsibilities
//!
//! - **Storage Orchestration**: Replicate, evict, rebuild blobs
//! - **Workload Balancing**: Redirect clients, throttle sync
//! - **Cache Management**: Resize, warm, flush caches
//! - **Debug & Diagnostics**: Log levels, heap dumps, bug reports
//! - **Health Recovery**: Restart services, failover, quarantine

pub mod models;
pub mod monitor;
pub mod analyzer;
pub mod decider;
pub mod executor;
pub mod actions;
pub mod protocol;
pub mod consensus;
pub mod cli;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use models::*;
use monitor::Monitor;
use analyzer::Analyzer;
use decider::Decider;
use executor::Executor;
use consensus::ConsensusManager;

/// Pod configuration
#[derive(Debug, Clone)]
pub struct PodConfig {
    /// Whether the pod is enabled
    pub enabled: bool,
    /// Decision interval in seconds
    pub decision_interval_secs: u64,
    /// Path to rules file
    pub rules_file: Option<String>,
    /// Maximum actions per hour
    pub max_actions_per_hour: u32,
    /// Dry run mode (don't execute actions)
    pub dry_run: bool,
}

impl Default for PodConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            decision_interval_secs: 10,
            rules_file: None,
            max_actions_per_hour: 20,
            dry_run: false,
        }
    }
}

/// The Pod orchestrator
pub struct Pod {
    config: PodConfig,
    node_id: String,
    monitor: Monitor,
    analyzer: Analyzer,
    decider: Decider,
    executor: Executor,
    consensus: ConsensusManager,
    status: Arc<RwLock<PodStatus>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    setup_complete: bool,
}

impl Pod {
    /// Create a new Pod instance
    pub fn new(node_id: String, config: PodConfig) -> Self {
        Self {
            node_id: node_id.clone(),
            config,
            monitor: Monitor::new(node_id.clone()),
            analyzer: Analyzer::new(node_id.clone()),
            decider: Decider::new(node_id.clone()),
            executor: Executor::new(node_id.clone()),
            consensus: ConsensusManager::new(node_id.clone()),
            status: Arc::new(RwLock::new(PodStatus::default())),
            shutdown_tx: None,
            setup_complete: false,
        }
    }

    /// Get current pod status
    pub async fn status(&self) -> PodStatus {
        let mut status = self.status.read().await.clone();
        status.actions_pending = self.executor.pending_count().await;
        status.actions_executed = self.executor.executed_count().await;
        status.peer_pods = self.consensus.peer_agents().await;
        status.active_rules = self.decider.rules().len();
        status
    }

    /// Load rules from file
    pub fn load_rules(&mut self, path: &str) -> Result<(), String> {
        self.decider.load_rules(path)
    }

    /// Set setup complete flag
    pub fn set_setup_complete(&mut self, complete: bool) {
        self.setup_complete = complete;
    }

    /// Start the pod orchestration loop
    pub async fn start(&mut self) -> Result<(), String> {
        if !self.config.enabled {
            info!("Pod is disabled, not starting");
            return Ok(());
        }

        // Load rules if configured
        if let Some(path) = self.config.rules_file.clone() {
            if let Err(e) = self.load_rules(&path) {
                warn!(path = %path, error = %e, "Failed to load rules file, using defaults");
            }
        }

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        // Update status
        {
            let mut status = self.status.write().await;
            status.active = true;
            status.node_id = self.node_id.clone();
            status.mode = if self.config.dry_run {
                PodMode::Manual
            } else {
                PodMode::Active
            };
            status.started_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }

        info!(
            node_id = %self.node_id,
            interval_secs = self.config.decision_interval_secs,
            dry_run = self.config.dry_run,
            "Pod started"
        );

        // Main loop
        let mut tick = interval(Duration::from_secs(self.config.decision_interval_secs));

        loop {
            tokio::select! {
                _ = tick.tick() => {
                    if let Err(e) = self.tick().await {
                        error!(error = %e, "Pod tick failed");
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Pod shutting down");
                    break;
                }
            }
        }

        // Update status
        {
            let mut status = self.status.write().await;
            status.active = false;
            status.mode = PodMode::Disabled;
        }

        Ok(())
    }

    /// Stop the pod
    pub async fn stop(&self) {
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(()).await;
        }
    }

    /// Execute one tick of the orchestration loop
    async fn tick(&mut self) -> Result<(), String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        debug!("Pod tick starting");

        // 1. Collect observations
        let observations = self.monitor.collect(self.setup_complete).await;

        // 2. Analyze for patterns and anomalies
        let anomalies = self.analyzer.analyze(&observations);

        if !anomalies.is_empty() {
            info!(count = anomalies.len(), "Detected anomalies");
            for anomaly in &anomalies {
                warn!(
                    anomaly_type = ?anomaly.anomaly_type,
                    severity = ?anomaly.severity,
                    description = %anomaly.description,
                    "Anomaly detected"
                );
            }
        }

        // 3. Get latest metrics for decision making
        let latest_metrics = self.monitor.get_latest_metrics().await;

        // 4. Build service health map
        let service_health = self.get_service_health();

        // 5. Make decisions (evaluate rules)
        let actions = self.decider.evaluate(
            latest_metrics.as_ref(),
            &service_health,
            &anomalies,
        );

        if !actions.is_empty() {
            info!(count = actions.len(), "Generated actions from rules");
        }

        // 6. Queue actions
        for action in actions {
            if self.config.dry_run {
                info!(
                    action_id = %action.id,
                    kind = ?action.kind,
                    "Would execute action (dry run)"
                );
                continue;
            }

            // Check if action needs consensus
            match &action.risk {
                ActionRisk::Safe => {
                    // Queue directly
                    if let Err(e) = self.executor.queue(action.clone()).await {
                        warn!(error = %e, "Failed to queue action");
                    }
                }
                ActionRisk::Risky { required_approvals, total_evaluators } => {
                    // Request consensus
                    let context = self.build_cluster_context().await;

                    match self.consensus.request_consensus(action.clone(), context).await {
                        Ok(outcome) => {
                            if outcome.is_approved() {
                                if let Err(e) = self.executor.queue(action).await {
                                    warn!(error = %e, "Failed to queue approved action");
                                }
                            } else {
                                info!(
                                    action_id = %action.id,
                                    outcome = ?outcome,
                                    "Action not approved by consensus"
                                );
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, "Consensus request failed");
                        }
                    }
                }
            }
        }

        // 7. Execute queued actions
        let results = self.executor.process_queue().await;

        if !results.is_empty() {
            info!(
                executed = results.len(),
                succeeded = results.iter().filter(|r| r.success).count(),
                "Processed action queue"
            );
        }

        // 8. Update status
        {
            let mut status = self.status.write().await;
            status.last_decision_at = Some(now);
        }

        debug!("Pod tick complete");
        Ok(())
    }

    /// Get current service health as a map
    fn get_service_health(&self) -> HashMap<String, bool> {
        // In a real implementation, this would query actual service states
        // For now, assume all services are healthy if setup is complete
        let mut health = HashMap::new();
        health.insert("holochain".to_string(), self.setup_complete);
        health.insert("sync".to_string(), self.setup_complete);
        health.insert("storage".to_string(), true);
        health.insert("p2p".to_string(), self.setup_complete);
        health.insert("api".to_string(), true);
        health
    }

    /// Build cluster context for consensus requests
    async fn build_cluster_context(&self) -> ClusterContext {
        let peers = self.consensus.peer_agents().await;
        let observations = self.monitor.get_recent(20).await;
        let latest_metrics = self.monitor.get_latest_metrics().await;

        ClusterContext {
            cluster_name: "default".to_string(), // Would come from config
            node_count: peers.len() + 1,
            healthy_nodes: peers.len() + 1, // Simplified
            recent_observations: observations,
            resource_summary: ResourceSummary {
                avg_cpu_percent: latest_metrics.as_ref().map(|m| m.cpu_percent).unwrap_or(0.0),
                avg_memory_percent: latest_metrics.as_ref().map(|m| m.memory_percent).unwrap_or(0.0),
                avg_disk_percent: latest_metrics.as_ref().map(|m| m.disk_percent).unwrap_or(0.0),
                total_storage_bytes: 0,
                used_storage_bytes: 0,
                total_blob_count: 0,
                connected_clients: 0,
            },
            active_issues: vec![],
        }
    }

    // =========================================================================
    // CLI INTERFACE
    // =========================================================================

    /// Execute a manual action (from CLI)
    pub async fn execute_manual_action(&mut self, action: Action) -> Result<ActionResult, String> {
        info!(
            action_id = %action.id,
            kind = ?action.kind,
            "Manual action requested"
        );

        // Queue and execute immediately
        self.executor.queue(action).await?;

        match self.executor.execute_next().await {
            Some(result) => Ok(result),
            None => Err("No action to execute".to_string()),
        }
    }

    /// Get action history
    pub async fn get_action_history(&self, count: usize) -> Vec<Action> {
        self.executor.get_history(count).await
    }

    /// Get recent observations
    pub async fn get_observations(&self, count: usize) -> Vec<Observation> {
        self.monitor.get_recent(count).await
    }

    /// Get active rules
    pub fn get_rules(&self) -> Vec<Rule> {
        self.decider.rules().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pod_creation() {
        let pod = Pod::new("test-node".to_string(), PodConfig::default());
        let status = pod.status().await;

        assert_eq!(status.node_id, "");  // Not started yet
        assert!(!status.active);
    }

    #[tokio::test]
    async fn test_pod_status() {
        let mut pod = Pod::new("test-node".to_string(), PodConfig {
            enabled: false, // Don't start the loop
            ..Default::default()
        });

        let status = pod.status().await;
        assert!(!status.active);
        assert_eq!(status.mode, PodMode::Disabled);
    }
}
