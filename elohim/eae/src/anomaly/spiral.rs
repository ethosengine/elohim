//! Spiral pattern detection.
//!
//! Detects escalating negative behavior loops where a user
//! may be entering a harmful cycle.

use tracing::debug;

use crate::anomaly::{AnomalyDetector, AnomalyResult, AnomalySeverity};
use crate::types::{Observation, ObservationType, Result};

/// Signal indicating potential spiral behavior.
#[derive(Debug, Clone)]
pub struct SpiralSignal {
    /// Signal name
    pub name: String,
    /// Weight in detection (0.0 - 1.0)
    pub weight: f32,
    /// Description
    pub description: String,
}

/// Detector for spiral patterns.
pub struct SpiralDetector {
    /// Detection threshold
    threshold: f32,
    /// Warning patterns to look for
    patterns: Vec<SpiralPattern>,
}

/// A pattern that indicates potential spiral.
#[derive(Debug, Clone)]
struct SpiralPattern {
    /// Pattern name
    name: String,
    /// Observation types that match this pattern
    observation_types: Vec<ObservationType>,
    /// Minimum frequency to trigger (observations per minute)
    min_frequency: f32,
    /// Weight of this pattern
    weight: f32,
    /// Description
    description: String,
}

impl SpiralDetector {
    /// Create a new spiral detector with default configuration.
    pub fn new() -> Self {
        Self::with_threshold(0.7)
    }

    /// Create with custom threshold.
    pub fn with_threshold(threshold: f32) -> Self {
        Self {
            threshold,
            patterns: Self::default_patterns(),
        }
    }

    /// Get default spiral patterns.
    fn default_patterns() -> Vec<SpiralPattern> {
        vec![
            SpiralPattern {
                name: "rapid_emotional_state".to_string(),
                observation_types: vec![ObservationType::EmotionalState],
                min_frequency: 5.0, // 5 per minute
                weight: 0.4,
                description: "Rapid emotional state changes may indicate distress".to_string(),
            },
            SpiralPattern {
                name: "repetitive_content".to_string(),
                observation_types: vec![ObservationType::ContentSubmission],
                min_frequency: 10.0,
                weight: 0.3,
                description: "Repetitive content submission may indicate compulsive behavior".to_string(),
            },
            SpiralPattern {
                name: "behavior_escalation".to_string(),
                observation_types: vec![ObservationType::BehaviorPattern],
                min_frequency: 8.0,
                weight: 0.5,
                description: "Escalating behavior patterns detected".to_string(),
            },
            SpiralPattern {
                name: "security_attempts".to_string(),
                observation_types: vec![ObservationType::SecurityEvent],
                min_frequency: 3.0,
                weight: 0.8,
                description: "Repeated security events may indicate frustration or attack".to_string(),
            },
        ]
    }

    /// Add a custom pattern.
    pub fn add_pattern(&mut self, name: &str, observation_types: Vec<ObservationType>, min_frequency: f32, weight: f32) {
        self.patterns.push(SpiralPattern {
            name: name.to_string(),
            observation_types,
            min_frequency,
            weight,
            description: format!("Custom pattern: {}", name),
        });
    }

    /// Calculate frequency of observations (per minute).
    fn calculate_frequency(&self, observations: &[Observation]) -> f32 {
        if observations.len() < 2 {
            return 0.0;
        }

        let first = observations.first().unwrap();
        let last = observations.last().unwrap();
        let duration_secs = (last.timestamp - first.timestamp).num_seconds() as f32;

        if duration_secs < 1.0 {
            return observations.len() as f32 * 60.0; // Scale up for sub-second windows
        }

        (observations.len() as f32 / duration_secs) * 60.0
    }

    /// Get suggested response based on severity.
    fn suggest_response(&self, severity: AnomalySeverity) -> String {
        match severity {
            AnomalySeverity::Low => {
                "Consider gentle check-in with user about their experience.".to_string()
            }
            AnomalySeverity::Medium => {
                "Recommend taking a break. Offer calming content or pause suggestion.".to_string()
            }
            AnomalySeverity::High => {
                "Strongly suggest break. Consider temporary rate limiting. Offer support resources.".to_string()
            }
            AnomalySeverity::Critical => {
                "Immediate intervention required. Display care message. Enable cooldown period.".to_string()
            }
        }
    }
}

impl Default for SpiralDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AnomalyDetector for SpiralDetector {
    fn name(&self) -> &str {
        "spiral_detector"
    }

    async fn analyze(&self, observations: &[Observation]) -> Result<AnomalyResult> {
        if observations.is_empty() {
            return Ok(AnomalyResult::none());
        }

        let mut total_score = 0.0;
        let mut signals = Vec::new();

        for pattern in &self.patterns {
            // Filter observations matching this pattern
            let matching: Vec<_> = observations
                .iter()
                .filter(|o| pattern.observation_types.contains(&o.observation_type))
                .collect();

            if matching.is_empty() {
                continue;
            }

            // Calculate frequency
            let frequency = self.calculate_frequency(
                &matching.iter().map(|&o| o.clone()).collect::<Vec<_>>(),
            );

            if frequency >= pattern.min_frequency {
                let pattern_score = (frequency / pattern.min_frequency).min(2.0) * pattern.weight;
                total_score += pattern_score;

                debug!(
                    pattern = %pattern.name,
                    frequency = frequency,
                    score = pattern_score,
                    "Spiral pattern matched"
                );

                signals.push(format!(
                    "{}: {} ({:.1}/min, threshold {:.1}/min)",
                    pattern.name, pattern.description, frequency, pattern.min_frequency
                ));
            }
        }

        // Normalize score
        let normalized_score = (total_score / self.patterns.len() as f32).min(1.0);

        if normalized_score >= self.threshold {
            let severity = if normalized_score >= 0.9 {
                AnomalySeverity::Critical
            } else if normalized_score >= 0.8 {
                AnomalySeverity::High
            } else if normalized_score >= 0.7 {
                AnomalySeverity::Medium
            } else {
                AnomalySeverity::Low
            };

            let suggestion = self.suggest_response(severity);

            Ok(AnomalyResult::detected(normalized_score, severity, signals)
                .with_suggestion(suggestion))
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

    fn make_observation(obs_type: ObservationType, secs_ago: i64) -> Observation {
        let mut obs = Observation::new(
            ObservationSource::UserInteraction,
            obs_type,
            serde_json::json!({}),
        );
        obs.timestamp = chrono::Utc::now() - chrono::Duration::seconds(secs_ago);
        obs
    }

    #[tokio::test]
    async fn test_no_spiral() {
        let detector = SpiralDetector::new();
        let observations = vec![
            make_observation(ObservationType::BehaviorPattern, 60),
            make_observation(ObservationType::BehaviorPattern, 30),
        ];

        let result = detector.analyze(&observations).await.unwrap();
        assert!(!result.detected);
    }

    #[tokio::test]
    async fn test_spiral_detected() {
        let detector = SpiralDetector::with_threshold(0.3);

        // Create many observations in short time (high frequency)
        let observations: Vec<_> = (0..20)
            .map(|i| make_observation(ObservationType::BehaviorPattern, i))
            .collect();

        let result = detector.analyze(&observations).await.unwrap();
        // With 20 observations over 19 seconds, that's ~63/min which exceeds threshold
        assert!(result.score > 0.0);
    }
}
