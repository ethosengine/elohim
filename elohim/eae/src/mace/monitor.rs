//! Monitor component - collects and buffers observations.

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, trace};

use crate::config::MonitorConfig;
use crate::types::{Observation, ObservationSource, ObservationType, Result};

/// Monitor for collecting observations.
pub struct Monitor {
    /// Configuration
    config: MonitorConfig,
    /// Observation buffer
    buffer: Arc<RwLock<VecDeque<Observation>>>,
    /// Observers to notify when new observations arrive
    observers: Arc<RwLock<Vec<Box<dyn ObservationHandler + Send + Sync>>>>,
}

impl Monitor {
    /// Create a new monitor with default configuration.
    pub fn new() -> Self {
        Self::with_config(MonitorConfig::default())
    }

    /// Create with custom configuration.
    pub fn with_config(config: MonitorConfig) -> Self {
        Self {
            config,
            buffer: Arc::new(RwLock::new(VecDeque::new())),
            observers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a builder for custom configuration.
    pub fn builder() -> MonitorBuilder {
        MonitorBuilder::new()
    }

    /// Record a new observation.
    pub async fn observe(&self, observation: Observation) -> Result<()> {
        // Apply sampling if configured
        if self.config.sampling_rate < 1.0 {
            let sample: f32 = rand_sample();
            if sample > self.config.sampling_rate {
                trace!(observation_id = %observation.id, "Observation sampled out");
                return Ok(());
            }
        }

        debug!(
            observation_id = %observation.id,
            source = ?observation.source,
            observation_type = ?observation.observation_type,
            "Recording observation"
        );

        // Add to buffer
        {
            let mut buffer = self.buffer.write().await;
            buffer.push_back(observation.clone());

            // Prune if over limit
            while buffer.len() > self.config.buffer_size {
                buffer.pop_front();
            }
        }

        // Notify observers
        let observers = self.observers.read().await;
        for observer in observers.iter() {
            observer.on_observation(&observation).await;
        }

        Ok(())
    }

    /// Record a user interaction observation.
    pub async fn observe_user_interaction(&self, data: serde_json::Value) -> Result<()> {
        let observation = Observation::new(
            ObservationSource::UserInteraction,
            ObservationType::BehaviorPattern,
            data,
        );
        self.observe(observation).await
    }

    /// Record a content submission observation.
    pub async fn observe_content(&self, content_id: &str, data: serde_json::Value) -> Result<()> {
        let mut observation = Observation::new(
            ObservationSource::ContentAnalysis,
            ObservationType::ContentSubmission,
            data,
        );
        observation.related_entities.push(content_id.to_string());
        self.observe(observation).await
    }

    /// Record a network signal observation.
    pub async fn observe_network_signal(&self, source_agent: &str, data: serde_json::Value) -> Result<()> {
        let mut observation = Observation::new(
            ObservationSource::NetworkSignal,
            ObservationType::BehaviorPattern,
            data,
        );
        observation.related_entities.push(source_agent.to_string());
        self.observe(observation).await
    }

    /// Get recent observations.
    pub async fn recent(&self, limit: usize) -> Vec<Observation> {
        let buffer = self.buffer.read().await;
        buffer.iter().rev().take(limit).cloned().collect()
    }

    /// Get observations by source.
    pub async fn by_source(&self, source: ObservationSource, limit: usize) -> Vec<Observation> {
        let buffer = self.buffer.read().await;
        buffer
            .iter()
            .rev()
            .filter(|o| o.source == source)
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get observations by type.
    pub async fn by_type(&self, observation_type: ObservationType, limit: usize) -> Vec<Observation> {
        let buffer = self.buffer.read().await;
        buffer
            .iter()
            .rev()
            .filter(|o| o.observation_type == observation_type)
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get observations related to an entity.
    pub async fn by_entity(&self, entity_id: &str, limit: usize) -> Vec<Observation> {
        let buffer = self.buffer.read().await;
        buffer
            .iter()
            .rev()
            .filter(|o| o.related_entities.contains(&entity_id.to_string()))
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get observation count.
    pub async fn count(&self) -> usize {
        let buffer = self.buffer.read().await;
        buffer.len()
    }

    /// Clear all observations.
    pub async fn clear(&self) {
        let mut buffer = self.buffer.write().await;
        buffer.clear();
    }

    /// Register an observation handler.
    pub async fn register_handler(&self, handler: Box<dyn ObservationHandler + Send + Sync>) {
        let mut observers = self.observers.write().await;
        observers.push(handler);
    }
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Handler trait for observation notifications.
#[async_trait::async_trait]
pub trait ObservationHandler {
    /// Called when a new observation is recorded.
    async fn on_observation(&self, observation: &Observation);
}

/// Builder for Monitor configuration.
pub struct MonitorBuilder {
    config: MonitorConfig,
}

impl MonitorBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            config: MonitorConfig::default(),
        }
    }

    /// Set buffer size.
    pub fn buffer_size(mut self, size: usize) -> Self {
        self.config.buffer_size = size;
        self
    }

    /// Set retention period.
    pub fn retention_secs(mut self, secs: u64) -> Self {
        self.config.retention_secs = secs;
        self
    }

    /// Set sampling rate.
    pub fn sampling_rate(mut self, rate: f32) -> Self {
        self.config.sampling_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Build the monitor.
    pub fn build(self) -> Monitor {
        Monitor::with_config(self.config)
    }
}

impl Default for MonitorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple pseudo-random sampling (deterministic for testing).
fn rand_sample() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos % 1000) as f32 / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitor_observe() {
        let monitor = Monitor::new();

        let observation = Observation::new(
            ObservationSource::UserInteraction,
            ObservationType::BehaviorPattern,
            serde_json::json!({"action": "click"}),
        );

        monitor.observe(observation).await.unwrap();
        assert_eq!(monitor.count().await, 1);
    }

    #[tokio::test]
    async fn test_monitor_buffer_limit() {
        let monitor = Monitor::builder()
            .buffer_size(5)
            .build();

        for i in 0..10 {
            let observation = Observation::new(
                ObservationSource::UserInteraction,
                ObservationType::BehaviorPattern,
                serde_json::json!({"index": i}),
            );
            monitor.observe(observation).await.unwrap();
        }

        assert_eq!(monitor.count().await, 5);
    }
}
