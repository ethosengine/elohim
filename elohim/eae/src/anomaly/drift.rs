//! Constitutional drift detection.
//!
//! Detects gradual departure from constitutional principles over time,
//! which might indicate value misalignment or corruption.

use std::collections::HashMap;
use tracing::debug;

use crate::anomaly::{AnomalyDetector, AnomalyResult, AnomalySeverity};
use crate::types::{Observation, ObservationType, Result};

/// Signal indicating constitutional drift.
#[derive(Debug, Clone)]
pub struct DriftSignal {
    /// Signal name
    pub name: String,
    /// Drift magnitude (0.0 - 1.0)
    pub magnitude: f32,
    /// Direction of drift
    pub direction: DriftDirection,
    /// Description
    pub description: String,
}

/// Direction of constitutional drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftDirection {
    /// Drifting toward more permissive behavior
    Permissive,
    /// Drifting toward more restrictive behavior
    Restrictive,
    /// Inconsistent direction
    Inconsistent,
}

/// Detector for constitutional drift.
pub struct DriftDetector {
    /// Detection threshold
    threshold: f32,
    /// Baseline metrics for comparison
    baseline: HashMap<String, f32>,
    /// Weight for compliance drift
    compliance_weight: f32,
    /// Weight for boundary drift
    boundary_weight: f32,
    /// Weight for governance drift
    governance_weight: f32,
}

impl DriftDetector {
    /// Create a new drift detector.
    pub fn new() -> Self {
        Self {
            threshold: 0.6,
            baseline: Self::default_baseline(),
            compliance_weight: 0.4,
            boundary_weight: 0.3,
            governance_weight: 0.3,
        }
    }

    /// Create with custom threshold.
    pub fn with_threshold(threshold: f32) -> Self {
        Self {
            threshold,
            ..Self::new()
        }
    }

    /// Get default baseline metrics.
    fn default_baseline() -> HashMap<String, f32> {
        let mut baseline = HashMap::new();
        baseline.insert("compliance_rate".to_string(), 0.95); // Expected 95% compliance
        baseline.insert("boundary_violations".to_string(), 0.02); // Expected <2% violations
        baseline.insert("governance_participation".to_string(), 0.5); // Expected 50% participation
        baseline.insert("escalation_rate".to_string(), 0.05); // Expected 5% escalations
        baseline
    }

    /// Set a custom baseline metric.
    pub fn set_baseline(&mut self, metric: &str, value: f32) {
        self.baseline.insert(metric.to_string(), value);
    }

    /// Analyze compliance drift.
    fn analyze_compliance(&self, observations: &[Observation]) -> Option<DriftSignal> {
        let compliance_checks: Vec<_> = observations
            .iter()
            .filter(|o| o.observation_type == ObservationType::ComplianceCheck)
            .collect();

        if compliance_checks.is_empty() {
            return None;
        }

        // Calculate compliance rate from observations
        let compliant_count = compliance_checks
            .iter()
            .filter(|o| {
                o.data
                    .get("compliant")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true)
            })
            .count();

        let current_rate = compliant_count as f32 / compliance_checks.len() as f32;
        let baseline_rate = self.baseline.get("compliance_rate").copied().unwrap_or(0.95);

        let drift = (baseline_rate - current_rate).abs();
        let direction = if current_rate < baseline_rate {
            DriftDirection::Permissive // Less compliance = more permissive
        } else {
            DriftDirection::Restrictive
        };

        if drift > 0.1 {
            Some(DriftSignal {
                name: "compliance_drift".to_string(),
                magnitude: drift,
                direction,
                description: format!(
                    "Compliance rate drifted from {:.1}% to {:.1}% ({})",
                    baseline_rate * 100.0,
                    current_rate * 100.0,
                    if direction == DriftDirection::Permissive {
                        "more permissive"
                    } else {
                        "more restrictive"
                    }
                ),
            })
        } else {
            None
        }
    }

    /// Analyze boundary violation drift.
    fn analyze_boundaries(&self, observations: &[Observation]) -> Option<DriftSignal> {
        let total = observations.len();
        if total == 0 {
            return None;
        }

        // Count boundary-related observations
        let boundary_observations = observations
            .iter()
            .filter(|o| {
                o.data
                    .get("boundary_check")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            })
            .count();

        let violations = observations
            .iter()
            .filter(|o| {
                o.data
                    .get("boundary_violated")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            })
            .count();

        if boundary_observations == 0 {
            return None;
        }

        let current_rate = violations as f32 / boundary_observations as f32;
        let baseline_rate = self.baseline.get("boundary_violations").copied().unwrap_or(0.02);

        let drift = (current_rate - baseline_rate).max(0.0);

        if drift > 0.05 {
            Some(DriftSignal {
                name: "boundary_drift".to_string(),
                magnitude: drift,
                direction: DriftDirection::Permissive, // More violations = more permissive
                description: format!(
                    "Boundary violations increased from {:.1}% to {:.1}%",
                    baseline_rate * 100.0,
                    current_rate * 100.0
                ),
            })
        } else {
            None
        }
    }

    /// Analyze governance participation drift.
    fn analyze_governance(&self, observations: &[Observation]) -> Option<DriftSignal> {
        let governance_observations: Vec<_> = observations
            .iter()
            .filter(|o| o.observation_type == ObservationType::GovernanceProposal)
            .collect();

        if governance_observations.is_empty() {
            return None;
        }

        // Calculate participation from observations
        let participated = governance_observations
            .iter()
            .filter(|o| {
                o.data
                    .get("participated")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            })
            .count();

        let current_rate = participated as f32 / governance_observations.len() as f32;
        let baseline_rate = self
            .baseline
            .get("governance_participation")
            .copied()
            .unwrap_or(0.5);

        let drift = (baseline_rate - current_rate).abs();
        let direction = if current_rate < baseline_rate {
            DriftDirection::Permissive // Less participation could indicate disengagement
        } else {
            DriftDirection::Restrictive // More participation
        };

        if drift > 0.2 {
            Some(DriftSignal {
                name: "governance_drift".to_string(),
                magnitude: drift,
                direction,
                description: format!(
                    "Governance participation changed from {:.1}% to {:.1}%",
                    baseline_rate * 100.0,
                    current_rate * 100.0
                ),
            })
        } else {
            None
        }
    }

    /// Get suggested response for drift.
    fn suggest_response(&self, severity: AnomalySeverity, direction: DriftDirection) -> String {
        let direction_note = match direction {
            DriftDirection::Permissive => "toward more permissive behavior",
            DriftDirection::Restrictive => "toward more restrictive behavior",
            DriftDirection::Inconsistent => "in inconsistent directions",
        };

        match severity {
            AnomalySeverity::Low => format!(
                "Monitor drift {}. Review recent decisions for alignment.",
                direction_note
            ),
            AnomalySeverity::Medium => format!(
                "Significant drift detected {}. Schedule constitutional review.",
                direction_note
            ),
            AnomalySeverity::High => format!(
                "Major drift {}. Initiate governance discussion. Consider temporary corrections.",
                direction_note
            ),
            AnomalySeverity::Critical => format!(
                "Critical drift {}. Escalate to higher constitutional layer. Enable protective measures.",
                direction_note
            ),
        }
    }
}

impl Default for DriftDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AnomalyDetector for DriftDetector {
    fn name(&self) -> &str {
        "drift_detector"
    }

    async fn analyze(&self, observations: &[Observation]) -> Result<AnomalyResult> {
        if observations.is_empty() {
            return Ok(AnomalyResult::none());
        }

        let mut signals = Vec::new();
        let mut total_drift = 0.0;
        let mut primary_direction = DriftDirection::Inconsistent;

        // Check compliance drift
        if let Some(signal) = self.analyze_compliance(observations) {
            debug!(
                signal = %signal.name,
                magnitude = signal.magnitude,
                "Compliance drift detected"
            );
            total_drift += signal.magnitude * self.compliance_weight;
            primary_direction = signal.direction;
            signals.push(signal.description);
        }

        // Check boundary drift
        if let Some(signal) = self.analyze_boundaries(observations) {
            debug!(
                signal = %signal.name,
                magnitude = signal.magnitude,
                "Boundary drift detected"
            );
            total_drift += signal.magnitude * self.boundary_weight;
            if signals.is_empty() {
                primary_direction = signal.direction;
            } else if primary_direction != signal.direction {
                primary_direction = DriftDirection::Inconsistent;
            }
            signals.push(signal.description);
        }

        // Check governance drift
        if let Some(signal) = self.analyze_governance(observations) {
            debug!(
                signal = %signal.name,
                magnitude = signal.magnitude,
                "Governance drift detected"
            );
            total_drift += signal.magnitude * self.governance_weight;
            if signals.is_empty() {
                primary_direction = signal.direction;
            } else if primary_direction != signal.direction {
                primary_direction = DriftDirection::Inconsistent;
            }
            signals.push(signal.description);
        }

        let normalized_score = total_drift.min(1.0);

        if normalized_score >= self.threshold {
            let severity = if normalized_score >= 0.9 {
                AnomalySeverity::Critical
            } else if normalized_score >= 0.75 {
                AnomalySeverity::High
            } else if normalized_score >= 0.6 {
                AnomalySeverity::Medium
            } else {
                AnomalySeverity::Low
            };

            let suggestion = self.suggest_response(severity, primary_direction);

            Ok(
                AnomalyResult::detected(normalized_score, severity, signals)
                    .with_suggestion(suggestion),
            )
        } else {
            Ok(AnomalyResult {
                score: normalized_score,
                detected: false,
                severity: None,
                signals,
                suggested_response: None,
            })
        }
    }

    fn threshold(&self) -> f32 {
        self.threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ObservationSource;

    #[tokio::test]
    async fn test_no_drift() {
        let detector = DriftDetector::new();
        let observations = vec![
            Observation::new(
                ObservationSource::SystemMetric,
                ObservationType::ComplianceCheck,
                serde_json::json!({"compliant": true}),
            ),
            Observation::new(
                ObservationSource::SystemMetric,
                ObservationType::ComplianceCheck,
                serde_json::json!({"compliant": true}),
            ),
        ];

        let result = detector.analyze(&observations).await.unwrap();
        assert!(!result.detected);
    }

    #[tokio::test]
    async fn test_compliance_drift() {
        let detector = DriftDetector::with_threshold(0.1);

        // Create observations with low compliance (drift from 95% to ~50%)
        let observations: Vec<_> = (0..10)
            .map(|i| {
                Observation::new(
                    ObservationSource::SystemMetric,
                    ObservationType::ComplianceCheck,
                    serde_json::json!({"compliant": i % 2 == 0}), // 50% compliant
                )
            })
            .collect();

        let result = detector.analyze(&observations).await.unwrap();
        assert!(result.score > 0.0);
        assert!(result.signals.iter().any(|s| s.contains("Compliance")));
    }
}
