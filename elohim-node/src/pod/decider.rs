//! Decider - Rule engine and decision making
//!
//! Evaluates rules against current state and generates actions.
//! LLM integration is stubbed for future implementation.

use std::collections::HashMap;
use tracing::{debug, info, warn};

use super::models::*;

/// Decider makes decisions based on rules and observations
pub struct Decider {
    node_id: String,
    /// Active rules
    rules: Vec<Rule>,
    /// Last trigger times for cooldown tracking
    last_triggered: HashMap<String, u64>,
}

impl Decider {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            rules: default_rules(),
            last_triggered: HashMap::new(),
        }
    }

    /// Load rules from a YAML file
    pub fn load_rules(&mut self, path: &str) -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read rules file: {}", e))?;

        let rules: Vec<Rule> = serde_yaml::from_str(&content)
            .map_err(|e| format!("Failed to parse rules: {}", e))?;

        self.rules = rules;
        info!(count = self.rules.len(), path, "Loaded rules");
        Ok(())
    }

    /// Get active rules
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Add a rule
    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    /// Evaluate rules and return actions to take
    pub fn evaluate(
        &mut self,
        latest_metrics: Option<&SystemMetricsData>,
        service_health: &HashMap<String, bool>,
        anomalies: &[AnomalyData],
    ) -> Vec<Action> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut actions = Vec::new();

        // Sort rules by priority (higher first)
        let mut sorted_rules: Vec<_> = self.rules.iter().enumerate().collect();
        sorted_rules.sort_by(|a, b| b.1.priority.cmp(&a.1.priority));

        for (idx, rule) in sorted_rules {
            if !rule.enabled {
                continue;
            }

            // Check cooldown
            if let Some(&last) = self.last_triggered.get(&rule.name) {
                if now < last + rule.cooldown_secs {
                    debug!(rule = %rule.name, "Rule on cooldown");
                    continue;
                }
            }

            // Evaluate condition
            let triggered = self.evaluate_condition(&rule.condition, latest_metrics, service_health);

            if triggered {
                info!(rule = %rule.name, "Rule triggered");

                // Create action from template
                let action = Action::new(
                    rule.action_template.kind.clone(),
                    &rule.action_template.description,
                    rule.action_template.params.clone(),
                ).with_risk(rule.action_template.risk.clone());

                actions.push(action);

                // Update last triggered
                self.last_triggered.insert(rule.name.clone(), now);
            }
        }

        // Also generate actions from anomalies with suggestions
        for anomaly in anomalies {
            if let Some(suggestion) = &anomaly.suggested_action {
                // Convert suggestion to action if severity is high enough
                if matches!(anomaly.severity, Severity::Error | Severity::Critical) {
                    if let Some(action) = self.anomaly_to_action(anomaly) {
                        actions.push(action);
                    }
                }
            }
        }

        actions
    }

    fn evaluate_condition(
        &self,
        condition: &Condition,
        metrics: Option<&SystemMetricsData>,
        services: &HashMap<String, bool>,
    ) -> bool {
        match condition {
            Condition::MetricAbove { metric, threshold } => {
                if let Some(m) = metrics {
                    let value = self.get_metric_value(metric, m);
                    value > *threshold
                } else {
                    false
                }
            }
            Condition::MetricBelow { metric, threshold } => {
                if let Some(m) = metrics {
                    let value = self.get_metric_value(metric, m);
                    value < *threshold
                } else {
                    false
                }
            }
            Condition::ServiceState { service, state } => {
                let running = services.get(service).copied().unwrap_or(true);
                match state {
                    ServiceState::Running => running,
                    ServiceState::Stopped | ServiceState::Failed => !running,
                    ServiceState::Degraded => false, // Would need more info
                }
            }
            Condition::ConditionActive { condition: _ } => {
                // Would check node conditions
                false
            }
            Condition::And { conditions } => {
                conditions.iter().all(|c| self.evaluate_condition(c, metrics, services))
            }
            Condition::Or { conditions } => {
                conditions.iter().any(|c| self.evaluate_condition(c, metrics, services))
            }
            Condition::Not { condition } => {
                !self.evaluate_condition(condition, metrics, services)
            }
        }
    }

    fn get_metric_value(&self, metric: &MetricName, data: &SystemMetricsData) -> f64 {
        match metric {
            MetricName::CpuPercent => data.cpu_percent as f64,
            MetricName::MemoryPercent => data.memory_percent as f64,
            MetricName::DiskPercent => data.disk_percent as f64,
            MetricName::DiskAvailableGb => (data.disk_available_bytes / 1024 / 1024 / 1024) as f64,
            MetricName::MemoryAvailableMb => (data.memory_available_bytes / 1024 / 1024) as f64,
            MetricName::LoadAverage1m => data.load_average[0],
            MetricName::LoadAverage5m => data.load_average[1],
            _ => 0.0, // Other metrics would need additional data sources
        }
    }

    fn anomaly_to_action(&self, anomaly: &AnomalyData) -> Option<Action> {
        // Map anomaly types to actions
        match anomaly.anomaly_type {
            AnomalyType::ServiceFailure => {
                // Extract service name from description if possible
                if anomaly.description.contains("holochain") {
                    Some(Action::new(
                        ActionKind::RestartService,
                        "Restart holochain service",
                        serde_json::json!({"service": "holochain"}),
                    ).with_risk(ActionRisk::Risky {
                        required_approvals: 2,
                        total_evaluators: 3,
                    }))
                } else if anomaly.description.contains("sync") {
                    Some(Action::new(
                        ActionKind::RestartService,
                        "Restart sync service",
                        serde_json::json!({"service": "sync"}),
                    ).with_risk(ActionRisk::Safe))
                } else {
                    None
                }
            }
            AnomalyType::ResourceSpike => {
                if anomaly.description.contains("Disk space") {
                    Some(Action::new(
                        ActionKind::RebalanceStorage,
                        "Rebalance storage to free disk space",
                        serde_json::json!({"dry_run": true}),
                    ).with_risk(ActionRisk::Safe))
                } else if anomaly.description.contains("Memory") {
                    Some(Action::new(
                        ActionKind::FlushCache,
                        "Flush caches to reduce memory pressure",
                        serde_json::json!({"cache": "content"}),
                    ).with_risk(ActionRisk::Safe))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    // =========================================================================
    // LLM INTEGRATION (STUBBED)
    // =========================================================================

    /// Request LLM evaluation for a complex decision
    ///
    /// This is stubbed - in the future, this would:
    /// 1. Connect to local llama.cpp or remote Claude API
    /// 2. Send the context and observations
    /// 3. Get back a structured decision
    pub async fn request_llm_evaluation(
        &self,
        context: &ClusterContext,
        observations: &[Observation],
        question: &str,
    ) -> LlmEvaluationResult {
        info!(question, "LLM evaluation requested (stubbed)");

        // Stubbed response - in the future would call actual LLM
        LlmEvaluationResult {
            available: false,
            response: None,
            reasoning: Some("LLM integration not yet implemented".to_string()),
            confidence: 0.0,
            model: None,
        }
    }

    /// Check if LLM inference is available
    pub fn has_llm(&self) -> bool {
        // Stubbed - would check if llama.cpp or API is configured
        false
    }
}

/// Result of an LLM evaluation request
#[derive(Debug, Clone)]
pub struct LlmEvaluationResult {
    /// Whether LLM was available
    pub available: bool,
    /// The LLM's response
    pub response: Option<String>,
    /// Reasoning behind the response
    pub reasoning: Option<String>,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Model used
    pub model: Option<String>,
}

/// Create default rules
fn default_rules() -> Vec<Rule> {
    vec![
        // Disk pressure rule
        Rule {
            name: "disk-pressure-evict".to_string(),
            enabled: true,
            condition: Condition::MetricAbove {
                metric: MetricName::DiskPercent,
                threshold: 85.0,
            },
            action_template: ActionTemplate {
                kind: ActionKind::RebalanceStorage,
                risk: ActionRisk::Risky {
                    required_approvals: 2,
                    total_evaluators: 3,
                },
                description: "Evict LRU blobs until disk usage is below 75%".to_string(),
                params: serde_json::json!({
                    "target_usage_percent": 75.0,
                }),
            },
            priority: 80,
            cooldown_secs: 300, // 5 minutes
            last_triggered: None,
        },

        // High CPU rule
        Rule {
            name: "high-cpu-throttle".to_string(),
            enabled: true,
            condition: Condition::MetricAbove {
                metric: MetricName::CpuPercent,
                threshold: 90.0,
            },
            action_template: ActionTemplate {
                kind: ActionKind::ThrottleSync,
                risk: ActionRisk::Safe,
                description: "Throttle sync operations due to high CPU".to_string(),
                params: serde_json::json!({
                    "max_concurrent": 2,
                    "duration_secs": 60,
                }),
            },
            priority: 70,
            cooldown_secs: 60,
            last_triggered: None,
        },

        // Memory pressure rule
        Rule {
            name: "memory-pressure-flush".to_string(),
            enabled: true,
            condition: Condition::MetricAbove {
                metric: MetricName::MemoryPercent,
                threshold: 90.0,
            },
            action_template: ActionTemplate {
                kind: ActionKind::FlushCache,
                risk: ActionRisk::Safe,
                description: "Flush caches due to memory pressure".to_string(),
                params: serde_json::json!({
                    "cache": "content",
                }),
            },
            priority: 85,
            cooldown_secs: 120,
            last_triggered: None,
        },

        // Service restart rule
        Rule {
            name: "service-restart-p2p".to_string(),
            enabled: true,
            condition: Condition::ServiceState {
                service: "p2p".to_string(),
                state: ServiceState::Failed,
            },
            action_template: ActionTemplate {
                kind: ActionKind::RestartService,
                risk: ActionRisk::Safe,
                description: "Restart failed P2P service".to_string(),
                params: serde_json::json!({
                    "service": "p2p",
                    "grace_period_secs": 5,
                }),
            },
            priority: 90,
            cooldown_secs: 180, // 3 minutes
            last_triggered: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_metric_above() {
        let mut decider = Decider::new("test".to_string());

        let metrics = SystemMetricsData {
            cpu_percent: 95.0,
            memory_percent: 50.0,
            disk_percent: 40.0,
            disk_available_bytes: 100 * 1024 * 1024 * 1024,
            memory_available_bytes: 8 * 1024 * 1024 * 1024,
            load_average: [1.0, 0.5, 0.3],
            network_rx_bytes: 0,
            network_tx_bytes: 0,
        };

        let services = HashMap::new();
        let anomalies = Vec::new();

        let actions = decider.evaluate(Some(&metrics), &services, &anomalies);

        // Should trigger high-cpu-throttle rule
        assert!(actions.iter().any(|a| matches!(a.kind, ActionKind::ThrottleSync)));
    }

    #[test]
    fn test_rule_cooldown() {
        let mut decider = Decider::new("test".to_string());

        let metrics = SystemMetricsData {
            cpu_percent: 95.0,
            memory_percent: 50.0,
            disk_percent: 40.0,
            disk_available_bytes: 100 * 1024 * 1024 * 1024,
            memory_available_bytes: 8 * 1024 * 1024 * 1024,
            load_average: [1.0, 0.5, 0.3],
            network_rx_bytes: 0,
            network_tx_bytes: 0,
        };

        let services = HashMap::new();
        let anomalies = Vec::new();

        // First evaluation should trigger
        let actions1 = decider.evaluate(Some(&metrics), &services, &anomalies);
        assert!(!actions1.is_empty());

        // Second evaluation should be on cooldown
        let actions2 = decider.evaluate(Some(&metrics), &services, &anomalies);
        // Same rule shouldn't trigger again
        assert!(actions2.len() < actions1.len());
    }
}
