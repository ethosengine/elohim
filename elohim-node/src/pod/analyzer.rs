//! Analyzer - Pattern detection and anomaly finding
//!
//! Analyzes observations to detect patterns, trends, and anomalies
//! that may require action.

use std::collections::HashMap;
use tracing::{debug, info, warn};

use super::models::*;

/// Window size for trend analysis (number of observations)
const TREND_WINDOW_SIZE: usize = 10;

/// Threshold for spike detection (standard deviations)
const SPIKE_THRESHOLD: f64 = 2.5;

/// Analyzer for observation patterns
pub struct Analyzer {
    node_id: String,
    /// Historical metrics for trend analysis
    metric_history: HashMap<String, Vec<f64>>,
}

impl Analyzer {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            metric_history: HashMap::new(),
        }
    }

    /// Analyze a set of observations and return detected anomalies
    pub fn analyze(&mut self, observations: &[Observation]) -> Vec<AnomalyData> {
        let mut anomalies = Vec::new();

        for obs in observations {
            match obs.kind {
                ObservationKind::SystemMetrics => {
                    if let Ok(metrics) = serde_json::from_value::<SystemMetricsData>(obs.data.clone()) {
                        anomalies.extend(self.analyze_system_metrics(&metrics));
                    }
                }
                ObservationKind::NodeConditions => {
                    if let Ok(change) = serde_json::from_value::<ConditionChangeData>(obs.data.clone()) {
                        if let Some(anomaly) = self.analyze_condition_change(&change) {
                            anomalies.push(anomaly);
                        }
                    }
                }
                ObservationKind::ServiceHealth => {
                    anomalies.extend(self.analyze_service_health(&obs.data));
                }
                _ => {}
            }
        }

        anomalies
    }

    fn analyze_system_metrics(&mut self, metrics: &SystemMetricsData) -> Vec<AnomalyData> {
        let mut anomalies = Vec::new();

        // Track CPU and detect spikes
        self.record_metric("cpu", metrics.cpu_percent as f64);
        if let Some(spike) = self.detect_spike("cpu", metrics.cpu_percent as f64) {
            anomalies.push(AnomalyData {
                anomaly_type: AnomalyType::ResourceSpike,
                severity: if spike > 90.0 { Severity::Critical } else { Severity::Warning },
                description: format!("CPU spike detected: {:.1}%", spike),
                suggested_action: Some("Consider throttling sync or redirecting clients".to_string()),
            });
        }

        // Track memory and detect pressure
        self.record_metric("memory", metrics.memory_percent as f64);
        if metrics.memory_percent > 90.0 {
            anomalies.push(AnomalyData {
                anomaly_type: AnomalyType::ResourceSpike,
                severity: Severity::Critical,
                description: format!(
                    "Memory pressure: {:.1}% used, {} MB available",
                    metrics.memory_percent,
                    metrics.memory_available_bytes / 1024 / 1024
                ),
                suggested_action: Some("Flush caches or reduce concurrent operations".to_string()),
            });
        }

        // Check disk space
        if metrics.disk_percent > 85.0 {
            let severity = if metrics.disk_percent > 95.0 {
                Severity::Critical
            } else if metrics.disk_percent > 90.0 {
                Severity::Error
            } else {
                Severity::Warning
            };

            anomalies.push(AnomalyData {
                anomaly_type: AnomalyType::ResourceSpike,
                severity,
                description: format!(
                    "Disk space low: {:.1}% used, {} GB available",
                    metrics.disk_percent,
                    metrics.disk_available_bytes / 1024 / 1024 / 1024
                ),
                suggested_action: Some("Evict low-priority blobs".to_string()),
            });
        }

        // Check load average
        if metrics.load_average[0] > 10.0 {
            anomalies.push(AnomalyData {
                anomaly_type: AnomalyType::ResourceSpike,
                severity: Severity::Warning,
                description: format!("High system load: {:.2}", metrics.load_average[0]),
                suggested_action: Some("Throttle operations or investigate processes".to_string()),
            });
        }

        anomalies
    }

    fn analyze_condition_change(&self, change: &ConditionChangeData) -> Option<AnomalyData> {
        // Only alert on negative changes (condition becoming true for pressure, or ready becoming false)
        let is_negative = match change.condition.as_str() {
            "memory_pressure" | "disk_pressure" | "pid_pressure" => !change.previous && change.current,
            "network_ready" | "ready" => change.previous && !change.current,
            _ => false,
        };

        if !is_negative {
            return None;
        }

        let severity = match change.condition.as_str() {
            "ready" => Severity::Critical,
            "memory_pressure" | "disk_pressure" => Severity::Error,
            _ => Severity::Warning,
        };

        Some(AnomalyData {
            anomaly_type: AnomalyType::ResourceSpike,
            severity,
            description: format!("Condition changed: {} - {}", change.condition, change.reason),
            suggested_action: Some(format!("Address {} condition", change.condition)),
        })
    }

    fn analyze_service_health(&self, data: &serde_json::Value) -> Vec<AnomalyData> {
        let mut anomalies = Vec::new();

        // Check each service
        let services = ["holochain", "sync", "storage", "p2p", "api"];
        for service in services {
            if let Some(svc) = data.get(service) {
                let running = svc.get("running").and_then(|v| v.as_bool()).unwrap_or(true);
                let healthy = svc.get("healthy").and_then(|v| v.as_bool()).unwrap_or(true);

                if !running {
                    anomalies.push(AnomalyData {
                        anomaly_type: AnomalyType::ServiceFailure,
                        severity: Severity::Critical,
                        description: format!("Service '{}' is not running", service),
                        suggested_action: Some(format!("Restart {} service", service)),
                    });
                } else if !healthy {
                    anomalies.push(AnomalyData {
                        anomaly_type: AnomalyType::ServiceFailure,
                        severity: Severity::Error,
                        description: format!("Service '{}' is unhealthy", service),
                        suggested_action: Some(format!("Check {} service logs", service)),
                    });
                }
            }
        }

        anomalies
    }

    /// Record a metric value for trend analysis
    fn record_metric(&mut self, name: &str, value: f64) {
        let history = self.metric_history.entry(name.to_string()).or_insert_with(Vec::new);
        history.push(value);

        // Keep only the window size
        if history.len() > TREND_WINDOW_SIZE * 2 {
            history.drain(0..TREND_WINDOW_SIZE);
        }
    }

    /// Detect if a value is a spike compared to recent history
    fn detect_spike(&self, name: &str, value: f64) -> Option<f64> {
        let history = self.metric_history.get(name)?;

        if history.len() < 5 {
            return None;
        }

        // Calculate mean and std dev of recent history (excluding last value)
        let window = &history[..history.len().saturating_sub(1)];
        let mean = window.iter().sum::<f64>() / window.len() as f64;
        let variance = window.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / window.len() as f64;
        let std_dev = variance.sqrt();

        // Check if current value is a spike
        if std_dev > 0.0 && (value - mean).abs() > SPIKE_THRESHOLD * std_dev {
            Some(value)
        } else {
            None
        }
    }

    /// Get trend direction for a metric (-1 = decreasing, 0 = stable, 1 = increasing)
    pub fn get_trend(&self, name: &str) -> Option<i8> {
        let history = self.metric_history.get(name)?;

        if history.len() < 5 {
            return None;
        }

        // Compare first half to second half average
        let mid = history.len() / 2;
        let first_half: f64 = history[..mid].iter().sum::<f64>() / mid as f64;
        let second_half: f64 = history[mid..].iter().sum::<f64>() / (history.len() - mid) as f64;

        let diff = second_half - first_half;
        let threshold = first_half * 0.1; // 10% change threshold

        Some(if diff > threshold {
            1
        } else if diff < -threshold {
            -1
        } else {
            0
        })
    }

    /// Analyze patterns across multiple observations
    pub fn analyze_patterns(&self, observations: &[Observation]) -> Vec<String> {
        let mut patterns = Vec::new();

        // Count observation types
        let mut type_counts: HashMap<ObservationKind, usize> = HashMap::new();
        for obs in observations {
            *type_counts.entry(obs.kind.clone()).or_insert(0) += 1;
        }

        // Detect unusual patterns
        if let Some(&anomaly_count) = type_counts.get(&ObservationKind::Anomaly) {
            if anomaly_count > 5 {
                patterns.push(format!(
                    "High anomaly rate: {} anomalies in observation window",
                    anomaly_count
                ));
            }
        }

        // Check for metric trends
        for (name, trend) in [("cpu", self.get_trend("cpu")), ("memory", self.get_trend("memory"))] {
            if let Some(1) = trend {
                patterns.push(format!("{} usage is trending upward", name));
            }
        }

        patterns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_spike() {
        let mut analyzer = Analyzer::new("test".to_string());

        // Record stable values
        for _ in 0..10 {
            analyzer.record_metric("test", 50.0);
        }

        // Should not detect spike for normal value
        assert!(analyzer.detect_spike("test", 52.0).is_none());

        // Should detect spike for abnormal value
        assert!(analyzer.detect_spike("test", 100.0).is_some());
    }

    #[test]
    fn test_get_trend() {
        let mut analyzer = Analyzer::new("test".to_string());

        // Record increasing values
        for i in 0..10 {
            analyzer.record_metric("increasing", i as f64 * 10.0);
        }
        assert_eq!(analyzer.get_trend("increasing"), Some(1));

        // Record decreasing values
        for i in (0..10).rev() {
            analyzer.record_metric("decreasing", i as f64 * 10.0);
        }
        assert_eq!(analyzer.get_trend("decreasing"), Some(-1));

        // Record stable values
        for _ in 0..10 {
            analyzer.record_metric("stable", 50.0);
        }
        assert_eq!(analyzer.get_trend("stable"), Some(0));
    }
}
