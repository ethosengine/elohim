//! Analyzer component - detects patterns and anomalies.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::config::AnalyzerConfig;
use crate::types::{
    AnalysisEvent, AnalysisEventType, EventPriority, Observation, ObservationType, Result,
};

/// Pattern definition for detection.
#[derive(Debug, Clone)]
pub struct Pattern {
    /// Unique pattern ID
    pub id: String,
    /// Pattern name
    pub name: String,
    /// Observation types that contribute to this pattern
    pub observation_types: Vec<ObservationType>,
    /// Minimum observations to trigger
    pub threshold_count: usize,
    /// Time window in seconds
    pub window_secs: u64,
    /// Event type to generate
    pub event_type: AnalysisEventType,
    /// Priority of generated events
    pub priority: EventPriority,
    /// Optional condition function name
    pub condition: Option<String>,
}

/// A detected pattern match.
#[derive(Debug, Clone)]
pub struct PatternMatch {
    /// Pattern that matched
    pub pattern: Pattern,
    /// Matching observations
    pub observations: Vec<Observation>,
    /// Confidence score
    pub confidence: f32,
    /// Timestamp of match
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Analyzer for detecting patterns in observations.
pub struct Analyzer {
    /// Configuration
    config: AnalyzerConfig,
    /// Registered patterns
    patterns: Arc<RwLock<Vec<Pattern>>>,
    /// Recent observations for analysis
    observation_window: Arc<RwLock<Vec<Observation>>>,
    /// Pattern match history
    match_history: Arc<RwLock<Vec<PatternMatch>>>,
}

impl Analyzer {
    /// Create a new analyzer with default configuration.
    pub fn new() -> Self {
        Self::with_config(AnalyzerConfig::default())
    }

    /// Create with custom configuration.
    pub fn with_config(config: AnalyzerConfig) -> Self {
        Self {
            config,
            patterns: Arc::new(RwLock::new(Vec::new())),
            observation_window: Arc::new(RwLock::new(Vec::new())),
            match_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a builder.
    pub fn builder() -> AnalyzerBuilder {
        AnalyzerBuilder::new()
    }

    /// Register a pattern for detection.
    pub async fn register_pattern(&self, pattern: Pattern) {
        let mut patterns = self.patterns.write().await;
        patterns.push(pattern);
    }

    /// Analyze a new observation and return any triggered events.
    pub async fn analyze(&self, observation: Observation) -> Result<Vec<AnalysisEvent>> {
        // Add to window
        {
            let mut window = self.observation_window.write().await;
            window.push(observation.clone());

            // Prune old observations outside window
            let cutoff = chrono::Utc::now()
                - chrono::Duration::seconds(self.config.window_secs as i64);
            window.retain(|o| o.timestamp > cutoff);
        }

        // Check patterns
        let mut events = Vec::new();
        let patterns = self.patterns.read().await;
        let window = self.observation_window.read().await;

        for pattern in patterns.iter() {
            if let Some(event) = self.check_pattern(pattern, &window).await {
                events.push(event);
            }
        }

        // Check anomalies if enabled
        if self.config.enable_anomaly_detection {
            if let Some(event) = self.check_anomalies(&window).await {
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Check if a pattern matches.
    async fn check_pattern(
        &self,
        pattern: &Pattern,
        observations: &[Observation],
    ) -> Option<AnalysisEvent> {
        // Filter observations by type
        let matching: Vec<&Observation> = observations
            .iter()
            .filter(|o| pattern.observation_types.contains(&o.observation_type))
            .collect();

        if matching.len() < pattern.threshold_count {
            return None;
        }

        debug!(
            pattern_id = %pattern.id,
            match_count = matching.len(),
            "Pattern threshold reached"
        );

        // Record match
        let pattern_match = PatternMatch {
            pattern: pattern.clone(),
            observations: matching.iter().map(|&o| o.clone()).collect(),
            confidence: (matching.len() as f32 / pattern.threshold_count as f32).min(1.0),
            timestamp: chrono::Utc::now(),
        };

        {
            let mut history = self.match_history.write().await;
            history.push(pattern_match);

            // Keep history bounded
            while history.len() > 1000 {
                history.remove(0);
            }
        }

        // Generate event
        Some(AnalysisEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            event_type: pattern.event_type,
            observations: matching.iter().map(|&o| o.clone()).collect(),
            priority: pattern.priority,
            layer_context: None,
        })
    }

    /// Check for anomalies in the observation window.
    async fn check_anomalies(&self, observations: &[Observation]) -> Option<AnalysisEvent> {
        if observations.len() < self.config.min_observations {
            return None;
        }

        // Simple anomaly detection: look for unusual patterns
        let mut type_counts: HashMap<ObservationType, usize> = HashMap::new();
        for obs in observations {
            *type_counts.entry(obs.observation_type).or_insert(0) += 1;
        }

        // Check for unusual concentration of a single type
        let total = observations.len() as f32;
        for (obs_type, count) in &type_counts {
            let ratio = *count as f32 / total;

            // Flag if >80% of observations are the same type (might indicate spiral)
            if ratio > 0.8 && *count > 10 {
                warn!(
                    observation_type = ?obs_type,
                    ratio = ratio,
                    "Potential anomaly detected: high concentration of single type"
                );

                return Some(AnalysisEvent {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: chrono::Utc::now(),
                    event_type: AnalysisEventType::AnomalyDetected,
                    observations: observations
                        .iter()
                        .filter(|o| o.observation_type == *obs_type)
                        .take(10)
                        .cloned()
                        .collect(),
                    priority: EventPriority::High,
                    layer_context: None,
                });
            }
        }

        None
    }

    /// Get recent pattern matches.
    pub async fn recent_matches(&self, limit: usize) -> Vec<PatternMatch> {
        let history = self.match_history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Clear analysis state.
    pub async fn clear(&self) {
        let mut window = self.observation_window.write().await;
        window.clear();

        let mut history = self.match_history.write().await;
        history.clear();
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for Analyzer configuration.
pub struct AnalyzerBuilder {
    config: AnalyzerConfig,
    patterns: Vec<Pattern>,
}

impl AnalyzerBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            config: AnalyzerConfig::default(),
            patterns: Vec::new(),
        }
    }

    /// Set analysis window.
    pub fn window_secs(mut self, secs: u64) -> Self {
        self.config.window_secs = secs;
        self
    }

    /// Set minimum observations.
    pub fn min_observations(mut self, min: usize) -> Self {
        self.config.min_observations = min;
        self
    }

    /// Enable/disable anomaly detection.
    pub fn anomaly_detection(mut self, enabled: bool) -> Self {
        self.config.enable_anomaly_detection = enabled;
        self
    }

    /// Add a pattern.
    pub fn with_pattern(mut self, pattern: Pattern) -> Self {
        self.patterns.push(pattern);
        self
    }

    /// Build the analyzer.
    pub async fn build(self) -> Analyzer {
        let analyzer = Analyzer::with_config(self.config);
        for pattern in self.patterns {
            analyzer.register_pattern(pattern).await;
        }
        analyzer
    }
}

impl Default for AnalyzerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ObservationSource;

    #[tokio::test]
    async fn test_pattern_detection() {
        let analyzer = Analyzer::new();

        // Register a pattern
        analyzer
            .register_pattern(Pattern {
                id: "test-pattern".to_string(),
                name: "Test Pattern".to_string(),
                observation_types: vec![ObservationType::BehaviorPattern],
                threshold_count: 3,
                window_secs: 300,
                event_type: AnalysisEventType::ThresholdCrossed,
                priority: EventPriority::Normal,
                condition: None,
            })
            .await;

        // Add observations
        for i in 0..5 {
            let observation = Observation::new(
                ObservationSource::UserInteraction,
                ObservationType::BehaviorPattern,
                serde_json::json!({"index": i}),
            );
            let events = analyzer.analyze(observation).await.unwrap();

            // Should trigger after 3rd observation
            if i >= 2 {
                assert!(!events.is_empty(), "Should detect pattern at index {}", i);
            }
        }
    }
}
