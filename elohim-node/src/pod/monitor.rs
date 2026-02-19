//! Monitor - Observation collection from local node
//!
//! Collects system metrics, conditions, and events into observations
//! that the pod can analyze and act upon.

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, trace};

use super::models::*;
use crate::dashboard::metrics::{self, NodeConditions, NodeMetrics};

/// Maximum observations to keep in history
const MAX_OBSERVATION_HISTORY: usize = 1000;

/// Monitor collects observations from the local node
pub struct Monitor {
    node_id: String,
    observations: Arc<RwLock<VecDeque<Observation>>>,
    previous_conditions: Option<NodeConditions>,
}

impl Monitor {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            observations: Arc::new(RwLock::new(VecDeque::with_capacity(
                MAX_OBSERVATION_HISTORY,
            ))),
            previous_conditions: None,
        }
    }

    /// Collect all current observations
    pub async fn collect(&mut self, setup_complete: bool) -> Vec<Observation> {
        let mut new_observations = Vec::new();

        // Collect system metrics
        let metrics = metrics::collect_metrics(&self.node_id, setup_complete);

        // Always record system metrics
        new_observations.push(self.observe_system_metrics(&metrics));

        // Check for condition changes
        if let Some(prev) = &self.previous_conditions {
            new_observations.extend(self.detect_condition_changes(prev, &metrics.conditions));
        }
        self.previous_conditions = Some(metrics.conditions.clone());

        // Check service health
        new_observations.push(self.observe_service_health(&metrics));

        // Store observations
        let mut obs = self.observations.write().await;
        for observation in &new_observations {
            obs.push_back(observation.clone());
            // Trim to max size
            while obs.len() > MAX_OBSERVATION_HISTORY {
                obs.pop_front();
            }
        }

        trace!(
            count = new_observations.len(),
            total = obs.len(),
            "Collected observations"
        );

        new_observations
    }

    /// Get observations since a given timestamp
    #[allow(dead_code)]
    pub async fn get_observations_since(&self, since: u64) -> Vec<Observation> {
        let obs = self.observations.read().await;
        obs.iter()
            .filter(|o| o.timestamp > since)
            .cloned()
            .collect()
    }

    /// Get recent observations (last N)
    pub async fn get_recent(&self, count: usize) -> Vec<Observation> {
        let obs = self.observations.read().await;
        obs.iter().rev().take(count).cloned().collect()
    }

    /// Get the latest metrics observation
    pub async fn get_latest_metrics(&self) -> Option<SystemMetricsData> {
        let obs = self.observations.read().await;
        obs.iter()
            .rev()
            .find(|o| o.kind == ObservationKind::SystemMetrics)
            .and_then(|o| serde_json::from_value(o.data.clone()).ok())
    }

    fn observe_system_metrics(&self, metrics: &NodeMetrics) -> Observation {
        let data = SystemMetricsData {
            cpu_percent: metrics.cpu.usage_percent,
            memory_percent: metrics.memory.usage_percent,
            disk_percent: metrics.disk.usage_percent,
            disk_available_bytes: metrics.disk.available_bytes,
            memory_available_bytes: metrics.memory.available_bytes,
            load_average: metrics.cpu.load_average,
            network_rx_bytes: metrics.network.rx_bytes,
            network_tx_bytes: metrics.network.tx_bytes,
        };

        Observation::new(
            &self.node_id,
            ObservationKind::SystemMetrics,
            serde_json::to_value(data).unwrap(),
        )
    }

    fn detect_condition_changes(
        &self,
        prev: &NodeConditions,
        curr: &NodeConditions,
    ) -> Vec<Observation> {
        let mut observations = Vec::new();

        // Check each condition for changes
        let checks = [
            (
                "memory_pressure",
                prev.memory_pressure.status,
                curr.memory_pressure.status,
                &curr.memory_pressure.reason,
            ),
            (
                "disk_pressure",
                prev.disk_pressure.status,
                curr.disk_pressure.status,
                &curr.disk_pressure.reason,
            ),
            (
                "pid_pressure",
                prev.pid_pressure.status,
                curr.pid_pressure.status,
                &curr.pid_pressure.reason,
            ),
            (
                "network_ready",
                prev.network_ready.status,
                curr.network_ready.status,
                &curr.network_ready.reason,
            ),
            (
                "ready",
                prev.ready.status,
                curr.ready.status,
                &curr.ready.reason,
            ),
        ];

        for (condition, prev_status, curr_status, reason) in checks {
            if prev_status != curr_status {
                debug!(
                    condition,
                    prev = prev_status,
                    curr = curr_status,
                    "Condition changed"
                );

                let data = ConditionChangeData {
                    condition: condition.to_string(),
                    previous: prev_status,
                    current: curr_status,
                    reason: reason.clone(),
                };

                observations.push(Observation::new(
                    &self.node_id,
                    ObservationKind::NodeConditions,
                    serde_json::to_value(data).unwrap(),
                ));
            }
        }

        observations
    }

    fn observe_service_health(&self, metrics: &NodeMetrics) -> Observation {
        let services = &metrics.services;

        // Create a summary of service health
        let data = serde_json::json!({
            "holochain": {
                "running": services.holochain.running,
                "healthy": services.holochain.healthy,
                "message": services.holochain.message,
            },
            "sync": {
                "running": services.sync.running,
                "healthy": services.sync.healthy,
            },
            "storage": {
                "running": services.storage.running,
                "healthy": services.storage.healthy,
            },
            "p2p": {
                "running": services.p2p.running,
                "healthy": services.p2p.healthy,
            },
            "api": {
                "running": services.api.running,
                "healthy": services.api.healthy,
            },
        });

        Observation::new(&self.node_id, ObservationKind::ServiceHealth, data)
    }

    /// Record an external observation (from peer or user)
    #[allow(dead_code)]
    pub async fn record(&self, observation: Observation) {
        let mut obs = self.observations.write().await;
        obs.push_back(observation);
        while obs.len() > MAX_OBSERVATION_HISTORY {
            obs.pop_front();
        }
    }

    /// Record an anomaly
    #[allow(dead_code)]
    pub async fn record_anomaly(
        &self,
        anomaly_type: AnomalyType,
        severity: Severity,
        description: impl Into<String>,
        suggested_action: Option<String>,
    ) {
        let data = AnomalyData {
            anomaly_type,
            severity,
            description: description.into(),
            suggested_action,
        };

        let observation = Observation::new(
            &self.node_id,
            ObservationKind::Anomaly,
            serde_json::to_value(data).unwrap(),
        );

        self.record(observation).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitor_collect() {
        let mut monitor = Monitor::new("test-node".to_string());

        let observations = monitor.collect(false).await;

        // Should have at least system metrics and service health
        assert!(observations.len() >= 2);
        assert!(observations
            .iter()
            .any(|o| o.kind == ObservationKind::SystemMetrics));
        assert!(observations
            .iter()
            .any(|o| o.kind == ObservationKind::ServiceHealth));
    }

    #[tokio::test]
    async fn test_observations_history() {
        let monitor = Monitor::new("test-node".to_string());

        // Record some observations
        for i in 0..10 {
            let obs = Observation::new(
                "test-node",
                ObservationKind::SystemMetrics,
                serde_json::json!({"iteration": i}),
            );
            monitor.record(obs).await;
        }

        let recent = monitor.get_recent(5).await;
        assert_eq!(recent.len(), 5);
    }
}
