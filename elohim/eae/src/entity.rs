//! ElohimAutonomousEntity - the main autonomous agent.
//!
//! Orchestrates all MACE components, anomaly detection, governance,
//! and precedent tracking into a cohesive autonomous entity.

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use constitution::{ConstitutionalStack, StackContext};
use elohim_agent::service::ElohimAgentService;

use crate::anomaly::{AnomalyDetector, DriftDetector, ManipulationDetector, SpiralDetector};
use crate::config::EaeConfig;
use crate::governance::{EscalationManager, EscalationReason, LayerContext, SubsidiarityChecker};
use crate::mace::{Analyzer, ConsensusManager, Decider, Executor, MacePipeline, Monitor};
use crate::precedent::PrecedentTracker;
use crate::types::{
    AnalysisEvent, Decision, EaeError, ExecutionResult, Observation, Result,
};

/// The Elohim Autonomous Entity.
///
/// A self-governing agent that:
/// - Monitors observations from various sources
/// - Analyzes patterns and detects anomalies
/// - Makes decisions using rules + LLM fallback
/// - Executes actions with appropriate authority
/// - Gathers consensus for high-impact decisions
/// - Learns from precedents
pub struct ElohimAutonomousEntity {
    /// Configuration
    config: EaeConfig,
    /// MACE pipeline
    mace: MacePipeline,
    /// Constitutional stack
    stack: Arc<RwLock<Option<ConstitutionalStack>>>,
    /// Agent service for LLM capabilities
    agent_service: Option<Arc<ElohimAgentService>>,
    /// Anomaly detectors
    anomaly_detectors: Vec<Arc<dyn AnomalyDetector>>,
    /// Escalation manager
    escalation: EscalationManager,
    /// Subsidiarity checker
    subsidiarity: SubsidiarityChecker,
    /// Precedent tracker
    precedents: Arc<PrecedentTracker>,
    /// Current layer context
    layer_context: Arc<RwLock<LayerContext>>,
    /// Whether entity is running
    running: Arc<RwLock<bool>>,
}

impl ElohimAutonomousEntity {
    /// Create a new entity with default configuration.
    pub fn new(entity_id: impl Into<String>) -> Self {
        let config = EaeConfig::new(entity_id);
        Self::with_config(config)
    }

    /// Create with custom configuration.
    pub fn with_config(config: EaeConfig) -> Self {
        let entity_id = config.entity_id.clone();

        // Create MACE components
        let monitor = Monitor::with_config(config.monitor.clone());
        let analyzer = Analyzer::with_config(config.analyzer.clone());
        let decider = Decider::with_config(config.decider.clone());
        let executor = Executor::with_config(config.executor.clone());
        let consensus = ConsensusManager::with_config(config.consensus.clone());

        let mace = MacePipeline::with_components(monitor, analyzer, decider, executor, consensus);

        // Create anomaly detectors
        let anomaly_detectors: Vec<Arc<dyn AnomalyDetector>> = vec![
            Arc::new(SpiralDetector::with_threshold(config.analyzer.spiral_threshold)),
            Arc::new(ManipulationDetector::with_threshold(config.analyzer.manipulation_threshold)),
            Arc::new(DriftDetector::with_threshold(config.analyzer.drift_threshold)),
        ];

        Self {
            config,
            mace,
            stack: Arc::new(RwLock::new(None)),
            agent_service: None,
            anomaly_detectors,
            escalation: EscalationManager::new(),
            subsidiarity: SubsidiarityChecker::new(),
            precedents: Arc::new(PrecedentTracker::new()),
            layer_context: Arc::new(RwLock::new(LayerContext::individual(&entity_id))),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Get entity ID.
    pub fn id(&self) -> &str {
        &self.config.entity_id
    }

    /// Set the agent service for LLM capabilities.
    pub fn with_agent_service(mut self, service: Arc<ElohimAgentService>) -> Self {
        self.agent_service = Some(service);
        self
    }

    /// Initialize the entity with a constitutional stack.
    pub async fn initialize(&self, context: StackContext) -> Result<()> {
        info!(entity_id = %self.config.entity_id, "Initializing ElohimAutonomousEntity");

        // Build constitutional stack
        let stack = ConstitutionalStack::build_defaults(context);
        let stack_hash = stack.stack_hash().to_string();

        {
            let mut s = self.stack.write().await;
            *s = Some(stack);
        }

        // Configure decider with stack hash
        self.mace.decider.set_stack_hash(stack_hash).await;

        // Initialize agent service if available
        if let Some(service) = &self.agent_service {
            if !service.is_initialized().await {
                // Agent service should be initialized separately
                warn!("Agent service not initialized - LLM fallback will not be available");
            }
        }

        {
            let mut running = self.running.write().await;
            *running = true;
        }

        info!("ElohimAutonomousEntity initialized");
        Ok(())
    }

    /// Set the layer context.
    pub async fn set_layer_context(&self, context: LayerContext) {
        let mut ctx = self.layer_context.write().await;
        *ctx = context;
    }

    /// Process an observation through the full MACE pipeline.
    pub async fn process(&self, observation: Observation) -> Result<Vec<ExecutionResult>> {
        if !*self.running.read().await {
            return Err(EaeError::ConfigError("Entity not initialized".to_string()));
        }

        debug!(
            entity_id = %self.config.entity_id,
            observation_id = %observation.id,
            "Processing observation"
        );

        // 1. Monitor: Record observation
        self.mace.monitor.observe(observation.clone()).await?;

        // 2. Check anomaly detectors
        let recent = self.mace.monitor.recent(100).await;
        for detector in &self.anomaly_detectors {
            let result = detector.analyze(&recent).await?;
            if result.detected {
                warn!(
                    detector = detector.name(),
                    score = result.score,
                    severity = ?result.severity,
                    "Anomaly detected"
                );

                // Create analysis event for anomaly
                let anomaly_event = AnalysisEvent {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: chrono::Utc::now(),
                    event_type: crate::types::AnalysisEventType::AnomalyDetected,
                    observations: recent.clone(),
                    priority: crate::types::EventPriority::High,
                    layer_context: Some(self.layer_context.read().await.layer()),
                };

                // Process the anomaly
                return self.process_event(anomaly_event).await;
            }
        }

        // 3. Analyze: Detect patterns
        let events = self.mace.analyzer.analyze(observation).await?;

        // 4. Process each event
        let mut all_results = Vec::new();
        for event in events {
            let results = self.process_event(event).await?;
            all_results.extend(results);
        }

        Ok(all_results)
    }

    /// Process an analysis event (decide and execute).
    async fn process_event(&self, event: AnalysisEvent) -> Result<Vec<ExecutionResult>> {
        debug!(
            event_id = %event.id,
            event_type = ?event.event_type,
            "Processing analysis event"
        );

        // 5. Decide: Make decision
        let decision = self.mace.decider.decide(event.clone()).await?;

        // 6. Check subsidiarity
        let layer_context = self.layer_context.read().await;
        let subsidiarity_result = self.subsidiarity.check(&decision, layer_context.layer());

        if !subsidiarity_result.appropriate {
            if let Some(suggested_layer) = subsidiarity_result.suggested_layer {
                info!(
                    reason = %subsidiarity_result.reason,
                    suggested_layer = %suggested_layer.as_str(),
                    "Subsidiarity check suggests different layer"
                );

                // Escalate if needed
                if suggested_layer > layer_context.layer() {
                    self.escalation
                        .escalate(
                            decision.clone(),
                            layer_context.layer(),
                            suggested_layer,
                            EscalationReason::InsufficientAuthority,
                            &subsidiarity_result.reason,
                        )
                        .await?;
                }
            }
        }
        drop(layer_context);

        // 7. Check if consensus required
        if decision.requires_consensus {
            info!(
                decision_id = %decision.id,
                "Decision requires consensus - requesting"
            );

            let request = self.mace.consensus.request_consensus(decision.clone()).await?;

            // In a real implementation, we would wait for consensus
            // For now, we continue with execution

            debug!(consensus_request_id = %request.id, "Consensus request created");
        }

        // 8. Execute: Run actions
        let results = self.mace.executor.execute(&decision).await;

        // 9. Store precedent
        self.precedents
            .store_from_decision(&decision, event.event_type)
            .await;

        Ok(results)
    }

    /// Get recent observations.
    pub async fn recent_observations(&self, limit: usize) -> Vec<Observation> {
        self.mace.monitor.recent(limit).await
    }

    /// Get recent pattern matches.
    pub async fn recent_matches(&self, limit: usize) -> Vec<crate::mace::PatternMatch> {
        self.mace.analyzer.recent_matches(limit).await
    }

    /// Get precedent statistics.
    pub async fn precedent_stats(&self) -> crate::precedent::PrecedentStats {
        self.precedents.stats().await
    }

    /// Get pending escalations.
    pub async fn pending_escalations(&self) -> Vec<crate::governance::EscalationRequest> {
        self.escalation.pending_requests().await
    }

    /// Get pending consensus requests.
    pub async fn pending_consensus(&self) -> Vec<crate::mace::ConsensusRequest> {
        self.mace.consensus.pending_requests().await
    }

    /// Shutdown the entity.
    pub async fn shutdown(&self) {
        info!(entity_id = %self.config.entity_id, "Shutting down ElohimAutonomousEntity");

        {
            let mut running = self.running.write().await;
            *running = false;
        }

        // Clean up resources
        self.mace.monitor.clear().await;
        self.mace.analyzer.clear().await;
    }

    /// Check if entity is running.
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

/// Builder for ElohimAutonomousEntity.
pub struct EaeBuilder {
    config: EaeConfig,
    agent_service: Option<Arc<ElohimAgentService>>,
}

impl EaeBuilder {
    /// Create a new builder.
    pub fn new(entity_id: impl Into<String>) -> Self {
        Self {
            config: EaeConfig::new(entity_id),
            agent_service: None,
        }
    }

    /// Set agent service.
    pub fn with_agent_service(mut self, service: Arc<ElohimAgentService>) -> Self {
        self.agent_service = Some(service);
        self
    }

    /// Set monitor config.
    pub fn monitor_buffer_size(mut self, size: usize) -> Self {
        self.config.monitor.buffer_size = size;
        self
    }

    /// Set analyzer window.
    pub fn analyzer_window_secs(mut self, secs: u64) -> Self {
        self.config.analyzer.window_secs = secs;
        self
    }

    /// Set spiral detection threshold.
    pub fn spiral_threshold(mut self, threshold: f32) -> Self {
        self.config.analyzer.spiral_threshold = threshold;
        self
    }

    /// Enable/disable anomaly detection.
    pub fn anomaly_detection(mut self, enabled: bool) -> Self {
        self.config.analyzer.enable_anomaly_detection = enabled;
        self
    }

    /// Enable/disable audit.
    pub fn audit_enabled(mut self, enabled: bool) -> Self {
        self.config.general.audit_enabled = enabled;
        self
    }

    /// Build the entity.
    pub fn build(self) -> ElohimAutonomousEntity {
        let mut entity = ElohimAutonomousEntity::with_config(self.config);
        if let Some(service) = self.agent_service {
            entity = entity.with_agent_service(service);
        }
        entity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ObservationSource, ObservationType};

    #[tokio::test]
    async fn test_entity_lifecycle() {
        let entity = ElohimAutonomousEntity::new("test-entity");

        assert!(!entity.is_running().await);

        entity
            .initialize(StackContext::agent_only("test-entity"))
            .await
            .unwrap();

        assert!(entity.is_running().await);

        entity.shutdown().await;

        assert!(!entity.is_running().await);
    }

    #[tokio::test]
    async fn test_process_observation() {
        let entity = ElohimAutonomousEntity::new("test-entity");
        entity
            .initialize(StackContext::agent_only("test-entity"))
            .await
            .unwrap();

        let observation = Observation::new(
            ObservationSource::UserInteraction,
            ObservationType::BehaviorPattern,
            serde_json::json!({"action": "click"}),
        );

        let results = entity.process(observation).await.unwrap();
        // Should complete without error (may or may not have results depending on patterns)

        let recent = entity.recent_observations(10).await;
        assert_eq!(recent.len(), 1);
    }

    #[tokio::test]
    async fn test_builder() {
        let entity = EaeBuilder::new("builder-test")
            .monitor_buffer_size(100)
            .spiral_threshold(0.5)
            .build();

        assert_eq!(entity.config.monitor.buffer_size, 100);
        assert_eq!(entity.config.analyzer.spiral_threshold, 0.5);
    }
}
