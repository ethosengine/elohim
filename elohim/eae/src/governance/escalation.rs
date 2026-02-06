//! Escalation management.
//!
//! Handles moving decisions to higher constitutional authority layers.

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use constitution::ConstitutionalLayer;
use crate::types::{Decision, EaeError, Result};

/// Reason for escalation.
#[derive(Debug, Clone)]
pub enum EscalationReason {
    /// Decision requires higher authority
    InsufficientAuthority,
    /// Constitutional conflict at current layer
    ConstitutionalConflict,
    /// Novel situation without precedent
    NovelSituation,
    /// High-impact decision
    HighImpact,
    /// Consensus not reached
    ConsensusFailure,
    /// Explicit request from user
    UserRequest,
    /// System safety concern
    SafetyConcern,
    /// Other reason
    Other(String),
}

impl EscalationReason {
    /// Get description.
    pub fn description(&self) -> &str {
        match self {
            EscalationReason::InsufficientAuthority => "Current layer lacks authority for this decision",
            EscalationReason::ConstitutionalConflict => "Conflict between constitutional principles at this layer",
            EscalationReason::NovelSituation => "Novel situation without established precedent",
            EscalationReason::HighImpact => "Decision has high impact requiring higher authority",
            EscalationReason::ConsensusFailure => "Failed to reach consensus at current layer",
            EscalationReason::UserRequest => "User explicitly requested escalation",
            EscalationReason::SafetyConcern => "Safety concerns require higher authority review",
            EscalationReason::Other(desc) => desc,
        }
    }
}

/// Request for escalation to a higher layer.
#[derive(Debug, Clone)]
pub struct EscalationRequest {
    /// Request ID
    pub id: String,
    /// Original decision
    pub decision: Decision,
    /// Current layer
    pub from_layer: ConstitutionalLayer,
    /// Target layer
    pub to_layer: ConstitutionalLayer,
    /// Reason for escalation
    pub reason: EscalationReason,
    /// Additional context
    pub context: String,
    /// When the request was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Status
    pub status: EscalationStatus,
}

/// Status of an escalation request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscalationStatus {
    /// Pending review
    Pending,
    /// Accepted by higher layer
    Accepted,
    /// Rejected by higher layer
    Rejected,
    /// Resolved
    Resolved,
    /// Expired
    Expired,
}

/// Manages escalation of decisions to higher layers.
pub struct EscalationManager {
    /// Pending escalation requests
    pending: Arc<RwLock<VecDeque<EscalationRequest>>>,
    /// Maximum pending requests
    max_pending: usize,
    /// Escalation timeout (seconds)
    timeout_secs: u64,
}

impl EscalationManager {
    /// Create a new escalation manager.
    pub fn new() -> Self {
        Self {
            pending: Arc::new(RwLock::new(VecDeque::new())),
            max_pending: 100,
            timeout_secs: 3600, // 1 hour
        }
    }

    /// Create with custom limits.
    pub fn with_limits(max_pending: usize, timeout_secs: u64) -> Self {
        Self {
            pending: Arc::new(RwLock::new(VecDeque::new())),
            max_pending,
            timeout_secs,
        }
    }

    /// Escalate a decision to a higher layer.
    pub async fn escalate(
        &self,
        decision: Decision,
        from_layer: ConstitutionalLayer,
        to_layer: ConstitutionalLayer,
        reason: EscalationReason,
        context: impl Into<String>,
    ) -> Result<EscalationRequest> {
        // Validate escalation direction
        if to_layer <= from_layer {
            return Err(EaeError::ConstitutionalViolation(
                "Cannot escalate to lower or equal layer".to_string(),
            ));
        }

        let request = EscalationRequest {
            id: uuid::Uuid::new_v4().to_string(),
            decision,
            from_layer,
            to_layer,
            reason: reason.clone(),
            context: context.into(),
            created_at: chrono::Utc::now(),
            status: EscalationStatus::Pending,
        };

        info!(
            request_id = %request.id,
            from = %from_layer.as_str(),
            to = %to_layer.as_str(),
            reason = %reason.description(),
            "Escalation request created"
        );

        // Store pending request
        {
            let mut pending = self.pending.write().await;
            pending.push_back(request.clone());

            // Prune if over limit
            while pending.len() > self.max_pending {
                pending.pop_front();
            }
        }

        Ok(request)
    }

    /// Get the next layer for escalation.
    pub fn next_layer(&self, current: ConstitutionalLayer) -> Option<ConstitutionalLayer> {
        match current {
            ConstitutionalLayer::Individual => Some(ConstitutionalLayer::Family),
            ConstitutionalLayer::Family => Some(ConstitutionalLayer::Community),
            ConstitutionalLayer::Community => Some(ConstitutionalLayer::Provincial),
            ConstitutionalLayer::Provincial => Some(ConstitutionalLayer::NationState),
            ConstitutionalLayer::NationState => Some(ConstitutionalLayer::Bioregional),
            ConstitutionalLayer::Bioregional => Some(ConstitutionalLayer::Global),
            ConstitutionalLayer::Global => None, // Can't escalate beyond global
        }
    }

    /// Check if escalation is needed based on decision.
    pub fn should_escalate(&self, decision: &Decision) -> Option<(ConstitutionalLayer, EscalationReason)> {
        use crate::types::DecisionType;

        // Check confidence threshold
        if decision.confidence < 0.5 {
            let next = self.next_layer(decision.reasoning.determining_layer)?;
            return Some((next, EscalationReason::NovelSituation));
        }

        // Check decision type
        match decision.decision_type {
            DecisionType::HardIntervention | DecisionType::Block => {
                // High-impact decisions may need escalation
                if decision.reasoning.determining_layer < ConstitutionalLayer::Community {
                    let next = self.next_layer(decision.reasoning.determining_layer)?;
                    return Some((next, EscalationReason::HighImpact));
                }
            }
            DecisionType::Escalate => {
                let next = self.next_layer(decision.reasoning.determining_layer)?;
                return Some((next, EscalationReason::InsufficientAuthority));
            }
            _ => {}
        }

        None
    }

    /// Update escalation status.
    pub async fn update_status(&self, request_id: &str, status: EscalationStatus) -> Result<()> {
        let mut pending = self.pending.write().await;

        if let Some(request) = pending.iter_mut().find(|r| r.id == request_id) {
            request.status = status;
            info!(
                request_id = %request_id,
                status = ?status,
                "Escalation status updated"
            );
            Ok(())
        } else {
            Err(EaeError::ConfigError(format!(
                "Escalation request {} not found",
                request_id
            )))
        }
    }

    /// Get pending escalations.
    pub async fn pending_requests(&self) -> Vec<EscalationRequest> {
        let pending = self.pending.read().await;
        pending
            .iter()
            .filter(|r| r.status == EscalationStatus::Pending)
            .cloned()
            .collect()
    }

    /// Clean up expired escalations.
    pub async fn cleanup_expired(&self) {
        let cutoff = chrono::Utc::now() - chrono::Duration::seconds(self.timeout_secs as i64);

        let mut pending = self.pending.write().await;
        let mut expired_count = 0;

        for request in pending.iter_mut() {
            if request.status == EscalationStatus::Pending && request.created_at < cutoff {
                request.status = EscalationStatus::Expired;
                expired_count += 1;
            }
        }

        if expired_count > 0 {
            warn!(count = expired_count, "Escalation requests expired");
        }
    }
}

impl Default for EscalationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Action, ActionType, DecisionReasoning, DecisionType};

    fn make_decision(layer: ConstitutionalLayer) -> Decision {
        Decision {
            id: "test".to_string(),
            timestamp: chrono::Utc::now(),
            event_id: "event".to_string(),
            decision_type: DecisionType::Allow,
            actions: vec![],
            reasoning: DecisionReasoning {
                primary_principle: "Test".to_string(),
                interpretation: "Test".to_string(),
                matched_rules: vec![],
                llm_assisted: false,
                precedents_considered: vec![],
                determining_layer: layer,
                stack_hash: "".to_string(),
            },
            confidence: 0.8,
            requires_consensus: false,
            consensus_status: None,
        }
    }

    #[tokio::test]
    async fn test_escalation() {
        let manager = EscalationManager::new();

        let decision = make_decision(ConstitutionalLayer::Individual);
        let request = manager
            .escalate(
                decision,
                ConstitutionalLayer::Individual,
                ConstitutionalLayer::Family,
                EscalationReason::HighImpact,
                "Test escalation",
            )
            .await
            .unwrap();

        assert_eq!(request.status, EscalationStatus::Pending);
        assert_eq!(request.to_layer, ConstitutionalLayer::Family);
    }

    #[test]
    fn test_next_layer() {
        let manager = EscalationManager::new();

        assert_eq!(
            manager.next_layer(ConstitutionalLayer::Individual),
            Some(ConstitutionalLayer::Family)
        );
        assert_eq!(manager.next_layer(ConstitutionalLayer::Global), None);
    }
}
