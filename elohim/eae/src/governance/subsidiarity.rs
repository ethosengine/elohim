//! Subsidiarity principle implementation.
//!
//! Ensures decisions are made at the lowest appropriate level,
//! only escalating when truly necessary.

use constitution::ConstitutionalLayer;
use crate::types::{Decision, DecisionType};

/// Result of subsidiarity check.
#[derive(Debug, Clone)]
pub struct SubsidiarityResult {
    /// Whether the decision level is appropriate
    pub appropriate: bool,
    /// Suggested layer if not appropriate
    pub suggested_layer: Option<ConstitutionalLayer>,
    /// Reason for the determination
    pub reason: String,
}

impl SubsidiarityResult {
    /// Create an appropriate result.
    pub fn appropriate() -> Self {
        Self {
            appropriate: true,
            suggested_layer: None,
            reason: "Decision is at appropriate level".to_string(),
        }
    }

    /// Create a result suggesting escalation.
    pub fn escalate(to: ConstitutionalLayer, reason: impl Into<String>) -> Self {
        Self {
            appropriate: false,
            suggested_layer: Some(to),
            reason: reason.into(),
        }
    }

    /// Create a result suggesting devolution (moving to lower layer).
    pub fn devolve(to: ConstitutionalLayer, reason: impl Into<String>) -> Self {
        Self {
            appropriate: false,
            suggested_layer: Some(to),
            reason: reason.into(),
        }
    }
}

/// Checks if decisions adhere to subsidiarity principle.
pub struct SubsidiarityChecker {
    /// Impact threshold for escalation
    impact_threshold: f32,
    /// Minimum confidence for local decision
    min_local_confidence: f32,
}

impl SubsidiarityChecker {
    /// Create a new checker with default settings.
    pub fn new() -> Self {
        Self {
            impact_threshold: 0.7,
            min_local_confidence: 0.6,
        }
    }

    /// Create with custom thresholds.
    pub fn with_thresholds(impact_threshold: f32, min_local_confidence: f32) -> Self {
        Self {
            impact_threshold,
            min_local_confidence,
        }
    }

    /// Check if a decision adheres to subsidiarity.
    pub fn check(&self, decision: &Decision, context_layer: ConstitutionalLayer) -> SubsidiarityResult {
        let decision_layer = decision.reasoning.determining_layer;

        // Check if decision layer matches context
        if decision_layer > context_layer {
            // Decision is being made at higher layer than context warrants
            return SubsidiarityResult::devolve(
                context_layer,
                format!(
                    "Decision is at {} but context only warrants {}",
                    decision_layer.as_str(),
                    context_layer.as_str()
                ),
            );
        }

        // Check decision type requirements
        match decision.decision_type {
            DecisionType::Block | DecisionType::HardIntervention => {
                // High-impact decisions should have community-level authority minimum
                if decision_layer < ConstitutionalLayer::Community {
                    return SubsidiarityResult::escalate(
                        ConstitutionalLayer::Community,
                        "Blocking/intervention decisions require at least community-level authority",
                    );
                }
            }
            DecisionType::Escalate => {
                // Already requesting escalation
                if let Some(next) = self.next_layer(decision_layer) {
                    return SubsidiarityResult::escalate(
                        next,
                        "Decision explicitly requests escalation",
                    );
                }
            }
            DecisionType::DeferToHuman => {
                // Human deferral is appropriate at any level
                return SubsidiarityResult::appropriate();
            }
            _ => {}
        }

        // Check confidence
        if decision.confidence < self.min_local_confidence {
            // Low confidence might warrant escalation
            if let Some(next) = self.next_layer(decision_layer) {
                return SubsidiarityResult::escalate(
                    next,
                    format!(
                        "Confidence {:.0}% is below threshold {:.0}%",
                        decision.confidence * 100.0,
                        self.min_local_confidence * 100.0
                    ),
                );
            }
        }

        // Check for novel precedent
        if decision.reasoning.precedents_considered.is_empty() && decision.reasoning.llm_assisted {
            // Novel decision with LLM assistance might warrant review
            if decision_layer < ConstitutionalLayer::Community {
                return SubsidiarityResult::escalate(
                    ConstitutionalLayer::Community,
                    "Novel decision without precedent should be reviewed at community level",
                );
            }
        }

        // Decision is at appropriate level
        SubsidiarityResult::appropriate()
    }

    /// Get next higher layer.
    fn next_layer(&self, current: ConstitutionalLayer) -> Option<ConstitutionalLayer> {
        match current {
            ConstitutionalLayer::Individual => Some(ConstitutionalLayer::Family),
            ConstitutionalLayer::Family => Some(ConstitutionalLayer::Community),
            ConstitutionalLayer::Community => Some(ConstitutionalLayer::Provincial),
            ConstitutionalLayer::Provincial => Some(ConstitutionalLayer::NationState),
            ConstitutionalLayer::NationState => Some(ConstitutionalLayer::Bioregional),
            ConstitutionalLayer::Bioregional => Some(ConstitutionalLayer::Global),
            ConstitutionalLayer::Global => None,
        }
    }

    /// Check if action scope matches layer.
    pub fn scope_matches_layer(&self, action_scope: ActionScope, layer: ConstitutionalLayer) -> bool {
        match action_scope {
            ActionScope::Self_ => true, // Self-actions are always appropriate
            ActionScope::Individual => layer >= ConstitutionalLayer::Individual,
            ActionScope::Family => layer >= ConstitutionalLayer::Family,
            ActionScope::Community => layer >= ConstitutionalLayer::Community,
            ActionScope::Regional => layer >= ConstitutionalLayer::Provincial,
            ActionScope::National => layer >= ConstitutionalLayer::NationState,
            ActionScope::Global => layer >= ConstitutionalLayer::Global,
        }
    }
}

impl Default for SubsidiarityChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Scope of an action's impact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionScope {
    /// Affects only self
    Self_,
    /// Affects individuals
    Individual,
    /// Affects family members
    Family,
    /// Affects community members
    Community,
    /// Affects regional population
    Regional,
    /// Affects national population
    National,
    /// Affects global population
    Global,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Action, ActionType, DecisionReasoning};

    fn make_decision(
        decision_type: DecisionType,
        layer: ConstitutionalLayer,
        confidence: f32,
    ) -> Decision {
        Decision {
            id: "test".to_string(),
            timestamp: chrono::Utc::now(),
            event_id: "event".to_string(),
            decision_type,
            actions: vec![],
            reasoning: DecisionReasoning {
                primary_principle: "Test".to_string(),
                interpretation: "Test".to_string(),
                matched_rules: vec![],
                llm_assisted: false,
                precedents_considered: vec!["precedent-1".to_string()],
                determining_layer: layer,
                stack_hash: "".to_string(),
            },
            confidence,
            requires_consensus: false,
            consensus_status: None,
        }
    }

    #[test]
    fn test_appropriate_level() {
        let checker = SubsidiarityChecker::new();

        let decision = make_decision(
            DecisionType::Allow,
            ConstitutionalLayer::Individual,
            0.9,
        );

        let result = checker.check(&decision, ConstitutionalLayer::Individual);
        assert!(result.appropriate);
    }

    #[test]
    fn test_block_needs_community() {
        let checker = SubsidiarityChecker::new();

        let decision = make_decision(
            DecisionType::Block,
            ConstitutionalLayer::Individual,
            0.9,
        );

        let result = checker.check(&decision, ConstitutionalLayer::Community);
        assert!(!result.appropriate);
        assert_eq!(result.suggested_layer, Some(ConstitutionalLayer::Community));
    }
}
