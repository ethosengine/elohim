//! Decider component - makes decisions using rules and LLM fallback.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use constitution::ConstitutionalLayer;
use elohim_agent::capability::ElohimCapability;
use elohim_agent::request::{ElohimRequest, RequestParams};
use elohim_agent::service::ElohimAgentService;

use crate::config::DeciderConfig;
use crate::types::{
    Action, ActionType, AnalysisEvent, AnalysisEventType, Decision, DecisionReasoning,
    DecisionType, EaeError, EventPriority, Result,
};

/// A rule in the rule engine.
#[derive(Debug, Clone)]
pub struct Rule {
    /// Unique rule ID
    pub id: String,
    /// Rule name
    pub name: String,
    /// Event types this rule applies to
    pub event_types: Vec<AnalysisEventType>,
    /// Priority (higher = evaluated first)
    pub priority: i32,
    /// Condition expression
    pub condition: RuleCondition,
    /// Decision to make if condition matches
    pub decision_type: DecisionType,
    /// Actions to take
    pub actions: Vec<ActionTemplate>,
    /// Constitutional layer this rule operates at
    pub layer: ConstitutionalLayer,
}

/// Condition for rule matching.
#[derive(Debug, Clone)]
pub enum RuleCondition {
    /// Always matches
    Always,
    /// Matches if event priority >= threshold
    PriorityAtLeast(EventPriority),
    /// Matches if observation count >= threshold
    ObservationCountAtLeast(usize),
    /// Matches on specific event type
    EventType(AnalysisEventType),
    /// Logical AND of conditions
    And(Vec<RuleCondition>),
    /// Logical OR of conditions
    Or(Vec<RuleCondition>),
    /// Custom expression (evaluated as JSON path)
    Custom(String),
}

impl RuleCondition {
    /// Evaluate the condition against an event.
    pub fn evaluate(&self, event: &AnalysisEvent) -> bool {
        match self {
            RuleCondition::Always => true,
            RuleCondition::PriorityAtLeast(threshold) => event.priority >= *threshold,
            RuleCondition::ObservationCountAtLeast(count) => event.observations.len() >= *count,
            RuleCondition::EventType(event_type) => event.event_type == *event_type,
            RuleCondition::And(conditions) => conditions.iter().all(|c| c.evaluate(event)),
            RuleCondition::Or(conditions) => conditions.iter().any(|c| c.evaluate(event)),
            RuleCondition::Custom(_) => {
                // Custom expressions would need a proper expression evaluator
                // For now, always return true
                true
            }
        }
    }
}

/// Template for generating actions.
#[derive(Debug, Clone)]
pub struct ActionTemplate {
    /// Action type
    pub action_type: ActionType,
    /// Target (can include variables like {{entity_id}})
    pub target_template: String,
    /// Parameters
    pub params: HashMap<String, serde_json::Value>,
}

impl ActionTemplate {
    /// Create an action from the template.
    pub fn to_action(&self, context: &HashMap<String, String>) -> Action {
        let mut target = self.target_template.clone();
        for (key, value) in context {
            target = target.replace(&format!("{{{{{}}}}}", key), value);
        }

        let mut action = Action::new(self.action_type, target);
        for (key, value) in &self.params {
            action.params.insert(key.clone(), value.clone());
        }
        action
    }
}

/// Simple rule engine for pattern-based decisions.
pub struct RuleEngine {
    rules: Vec<Rule>,
}

impl RuleEngine {
    /// Create a new empty rule engine.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a rule.
    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
        // Sort by priority (descending)
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Evaluate rules against an event.
    pub fn evaluate(&self, event: &AnalysisEvent) -> Option<(Rule, Vec<Action>)> {
        for rule in &self.rules {
            // Check if rule applies to this event type
            if !rule.event_types.is_empty() && !rule.event_types.contains(&event.event_type) {
                continue;
            }

            // Evaluate condition
            if rule.condition.evaluate(event) {
                debug!(
                    rule_id = %rule.id,
                    rule_name = %rule.name,
                    "Rule matched"
                );

                // Generate actions
                let context = self.build_context(event);
                let actions: Vec<Action> = rule
                    .actions
                    .iter()
                    .map(|t| t.to_action(&context))
                    .collect();

                return Some((rule.clone(), actions));
            }
        }

        None
    }

    /// Build context for action templates.
    fn build_context(&self, event: &AnalysisEvent) -> HashMap<String, String> {
        let mut context = HashMap::new();
        context.insert("event_id".to_string(), event.id.clone());
        context.insert("event_type".to_string(), format!("{:?}", event.event_type));

        // Add entity IDs from observations
        for (i, obs) in event.observations.iter().enumerate() {
            for (j, entity_id) in obs.related_entities.iter().enumerate() {
                context.insert(format!("entity_{}_{}", i, j), entity_id.clone());
            }
        }

        context
    }

    /// Get rule count.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Decider component - makes decisions using rules + LLM fallback.
pub struct Decider {
    /// Configuration
    config: DeciderConfig,
    /// Rule engine
    rule_engine: Arc<RwLock<RuleEngine>>,
    /// Optional LLM agent service for fallback
    agent_service: Option<Arc<ElohimAgentService>>,
    /// Constitutional stack hash
    stack_hash: Arc<RwLock<String>>,
}

impl Decider {
    /// Create a new decider with default configuration.
    pub fn new() -> Self {
        Self::with_config(DeciderConfig::default())
    }

    /// Create with custom configuration.
    pub fn with_config(config: DeciderConfig) -> Self {
        Self {
            config,
            rule_engine: Arc::new(RwLock::new(RuleEngine::new())),
            agent_service: None,
            stack_hash: Arc::new(RwLock::new(String::new())),
        }
    }

    /// Create a builder.
    pub fn builder() -> DeciderBuilder {
        DeciderBuilder::new()
    }

    /// Set the LLM agent service for fallback decisions.
    pub fn with_agent_service(mut self, service: Arc<ElohimAgentService>) -> Self {
        self.agent_service = Some(service);
        self
    }

    /// Set the constitutional stack hash.
    pub async fn set_stack_hash(&self, hash: String) {
        let mut stack_hash = self.stack_hash.write().await;
        *stack_hash = hash;
    }

    /// Add a rule to the engine.
    pub async fn add_rule(&self, rule: Rule) {
        let mut engine = self.rule_engine.write().await;
        engine.add_rule(rule);
    }

    /// Make a decision for an analysis event.
    pub async fn decide(&self, event: AnalysisEvent) -> Result<Decision> {
        let start = std::time::Instant::now();

        // First, try the rule engine
        let rule_engine = self.rule_engine.read().await;
        if let Some((rule, actions)) = rule_engine.evaluate(&event) {
            let stack_hash = self.stack_hash.read().await.clone();

            let decision = Decision {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: chrono::Utc::now(),
                event_id: event.id.clone(),
                decision_type: rule.decision_type,
                actions,
                reasoning: DecisionReasoning {
                    primary_principle: rule.name.clone(),
                    interpretation: format!("Rule '{}' matched event {:?}", rule.name, event.event_type),
                    matched_rules: vec![rule.id.clone()],
                    llm_assisted: false,
                    precedents_considered: vec![],
                    determining_layer: rule.layer,
                    stack_hash,
                },
                confidence: 1.0,
                requires_consensus: self.requires_consensus(&rule.decision_type),
                consensus_status: None,
            };

            info!(
                decision_id = %decision.id,
                decision_type = ?decision.decision_type,
                rule_id = %rule.id,
                duration_ms = start.elapsed().as_millis() as u64,
                "Decision made via rule engine"
            );

            return Ok(decision);
        }
        drop(rule_engine);

        // Fall back to LLM if available and confidence threshold not met
        if let Some(agent_service) = &self.agent_service {
            return self.decide_with_llm(&event, agent_service).await;
        }

        // Default decision if no rule matched and no LLM available
        let stack_hash = self.stack_hash.read().await.clone();
        Ok(Decision {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            event_id: event.id.clone(),
            decision_type: DecisionType::AllowWithMonitoring,
            actions: vec![Action::new(ActionType::Log, "default-monitor")],
            reasoning: DecisionReasoning {
                primary_principle: "Default allowance".to_string(),
                interpretation: "No specific rule matched; allowing with monitoring".to_string(),
                matched_rules: vec![],
                llm_assisted: false,
                precedents_considered: vec![],
                determining_layer: ConstitutionalLayer::Individual,
                stack_hash,
            },
            confidence: 0.5,
            requires_consensus: false,
            consensus_status: None,
        })
    }

    /// Make a decision using the LLM agent service.
    async fn decide_with_llm(
        &self,
        event: &AnalysisEvent,
        agent_service: &ElohimAgentService,
    ) -> Result<Decision> {
        info!(event_id = %event.id, "Falling back to LLM for decision");

        // Build request for spiral detection (or general analysis)
        let content = serde_json::to_string_pretty(&event)
            .map_err(|e| EaeError::DecisionError(e.to_string()))?;

        let request = ElohimRequest::new(ElohimCapability::SpiralDetection, agent_service.agent_id())
            .with_params(RequestParams::with_content(&content));

        let response = agent_service.invoke(request).await?;

        let stack_hash = self.stack_hash.read().await.clone();

        // Parse response to determine decision
        let (decision_type, actions) = self.parse_llm_response(&response);

        Ok(Decision {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            event_id: event.id.clone(),
            decision_type,
            actions,
            reasoning: DecisionReasoning {
                primary_principle: response.constitutional_reasoning.primary_principle.clone(),
                interpretation: response.constitutional_reasoning.interpretation.clone(),
                matched_rules: vec![],
                llm_assisted: true,
                precedents_considered: response.constitutional_reasoning.precedents.clone(),
                determining_layer: response.constitutional_reasoning.determining_layer,
                stack_hash,
            },
            confidence: response.constitutional_reasoning.confidence,
            requires_consensus: false,
            consensus_status: None,
        })
    }

    /// Parse LLM response to determine decision type and actions.
    fn parse_llm_response(
        &self,
        response: &elohim_agent::response::ElohimResponse,
    ) -> (DecisionType, Vec<Action>) {
        use elohim_agent::response::ResponsePayload;

        match &response.payload {
            ResponsePayload::SpiralDetection {
                detected,
                severity,
                suggested_response,
                ..
            } => {
                if *detected {
                    let decision_type = match severity.as_deref() {
                        Some("high") | Some("critical") => DecisionType::HardIntervention,
                        Some("medium") => DecisionType::SoftIntervention,
                        _ => DecisionType::AllowWithMonitoring,
                    };

                    let mut actions = vec![Action::new(ActionType::Log, "spiral-detected")];

                    if let Some(suggestion) = suggested_response {
                        actions.push(
                            Action::new(ActionType::Notify, "spiral-response")
                                .with_param("message", suggestion.clone()),
                        );
                    }

                    (decision_type, actions)
                } else {
                    (DecisionType::Allow, vec![])
                }
            }
            ResponsePayload::SafetyReview {
                safe,
                recommendation,
                ..
            } => {
                if *safe {
                    (DecisionType::Allow, vec![])
                } else {
                    (
                        DecisionType::Block,
                        vec![Action::new(ActionType::FilterContent, "safety-block")
                            .with_param("reason", recommendation.clone())],
                    )
                }
            }
            _ => (
                DecisionType::AllowWithMonitoring,
                vec![Action::new(ActionType::Log, "llm-decision")],
            ),
        }
    }

    /// Check if a decision type requires consensus.
    fn requires_consensus(&self, decision_type: &DecisionType) -> bool {
        let decision_str = format!("{:?}", decision_type).to_lowercase();
        self.config
            .require_consensus_for
            .iter()
            .any(|s| decision_str.contains(s))
    }
}

impl Default for Decider {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for Decider configuration.
pub struct DeciderBuilder {
    config: DeciderConfig,
    rules: Vec<Rule>,
    agent_service: Option<Arc<ElohimAgentService>>,
}

impl DeciderBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            config: DeciderConfig::default(),
            rules: Vec::new(),
            agent_service: None,
        }
    }

    /// Set LLM fallback threshold.
    pub fn llm_fallback_threshold(mut self, threshold: f32) -> Self {
        self.config.llm_fallback_threshold = threshold;
        self
    }

    /// Enable precedent lookup.
    pub fn use_precedents(mut self, enabled: bool) -> Self {
        self.config.use_precedents = enabled;
        self
    }

    /// Add a rule.
    pub fn with_rule(mut self, rule: Rule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Set agent service.
    pub fn with_agent_service(mut self, service: Arc<ElohimAgentService>) -> Self {
        self.agent_service = Some(service);
        self
    }

    /// Build the decider.
    pub async fn build(self) -> Decider {
        let mut decider = Decider::with_config(self.config);
        if let Some(service) = self.agent_service {
            decider = decider.with_agent_service(service);
        }
        for rule in self.rules {
            decider.add_rule(rule).await;
        }
        decider
    }
}

impl Default for DeciderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_condition_evaluation() {
        let event = AnalysisEvent {
            id: "test".to_string(),
            timestamp: chrono::Utc::now(),
            event_type: AnalysisEventType::ThresholdCrossed,
            observations: vec![],
            priority: EventPriority::High,
            layer_context: None,
        };

        assert!(RuleCondition::Always.evaluate(&event));
        assert!(RuleCondition::PriorityAtLeast(EventPriority::Normal).evaluate(&event));
        assert!(!RuleCondition::PriorityAtLeast(EventPriority::Critical).evaluate(&event));
    }

    #[tokio::test]
    async fn test_decider_default_decision() {
        let decider = Decider::new();

        let event = AnalysisEvent {
            id: "test".to_string(),
            timestamp: chrono::Utc::now(),
            event_type: AnalysisEventType::RoutineCheck,
            observations: vec![],
            priority: EventPriority::Low,
            layer_context: None,
        };

        let decision = decider.decide(event).await.unwrap();
        assert_eq!(decision.decision_type, DecisionType::AllowWithMonitoring);
    }
}
