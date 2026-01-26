//! Consensus component - gathers agreement from peer agents.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::config::ConsensusConfig;
use crate::types::{
    ConsensusStatus, ConsensusVote, Decision, EaeError, Result, VoteDecision,
};

/// Request for consensus from peers.
#[derive(Debug, Clone)]
pub struct ConsensusRequest {
    /// Request ID
    pub id: String,
    /// Decision requiring consensus
    pub decision: Decision,
    /// Requesting agent ID
    pub requester_id: String,
    /// When the request was made
    pub requested_at: chrono::DateTime<chrono::Utc>,
    /// Deadline for responses
    pub deadline: chrono::DateTime<chrono::Utc>,
}

impl ConsensusRequest {
    /// Create a new consensus request.
    pub fn new(decision: Decision, requester_id: impl Into<String>, timeout_secs: u64) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            decision,
            requester_id: requester_id.into(),
            requested_at: chrono::Utc::now(),
            deadline: chrono::Utc::now() + chrono::Duration::seconds(timeout_secs as i64),
        }
    }
}

/// Manages consensus gathering for risky decisions.
pub struct ConsensusManager {
    /// Configuration
    config: ConsensusConfig,
    /// Pending consensus requests
    pending: Arc<RwLock<HashMap<String, ConsensusRequest>>>,
    /// Collected votes
    votes: Arc<RwLock<HashMap<String, Vec<ConsensusVote>>>>,
    /// Completed consensus results
    completed: Arc<RwLock<Vec<ConsensusResult>>>,
}

/// Result of a consensus gathering.
#[derive(Debug, Clone)]
pub struct ConsensusResult {
    /// Request ID
    pub request_id: String,
    /// Decision ID
    pub decision_id: String,
    /// Whether consensus was reached
    pub reached: bool,
    /// Final vote tally
    pub approve_count: usize,
    /// Reject count
    pub reject_count: usize,
    /// Abstain count
    pub abstain_count: usize,
    /// Agreement ratio
    pub agreement_ratio: f32,
    /// Timestamp
    pub completed_at: chrono::DateTime<chrono::Utc>,
}

impl ConsensusManager {
    /// Create a new consensus manager with default configuration.
    pub fn new() -> Self {
        Self::with_config(ConsensusConfig::default())
    }

    /// Create with custom configuration.
    pub fn with_config(config: ConsensusConfig) -> Self {
        Self {
            config,
            pending: Arc::new(RwLock::new(HashMap::new())),
            votes: Arc::new(RwLock::new(HashMap::new())),
            completed: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Request consensus for a decision.
    pub async fn request_consensus(&self, decision: Decision) -> Result<ConsensusRequest> {
        let request = ConsensusRequest::new(
            decision.clone(),
            "local-agent", // TODO: Get actual agent ID
            self.config.timeout_secs,
        );

        info!(
            request_id = %request.id,
            decision_id = %decision.id,
            "Requesting consensus"
        );

        // Store pending request
        {
            let mut pending = self.pending.write().await;
            pending.insert(request.id.clone(), request.clone());
        }

        // Initialize votes collection
        {
            let mut votes = self.votes.write().await;
            votes.insert(request.id.clone(), Vec::new());
        }

        // In a real implementation, we would broadcast to peer agents here
        // For now, we just return the request

        Ok(request)
    }

    /// Submit a vote for a consensus request.
    pub async fn submit_vote(&self, request_id: &str, vote: ConsensusVote) -> Result<()> {
        debug!(
            request_id = %request_id,
            agent_id = %vote.agent_id,
            vote = ?vote.vote,
            "Vote submitted"
        );

        let mut votes = self.votes.write().await;
        if let Some(vote_list) = votes.get_mut(request_id) {
            // Check for duplicate vote
            if vote_list.iter().any(|v| v.agent_id == vote.agent_id) {
                return Err(EaeError::ConsensusError(format!(
                    "Agent {} already voted on request {}",
                    vote.agent_id, request_id
                )));
            }
            vote_list.push(vote);
        } else {
            return Err(EaeError::ConsensusError(format!(
                "No pending request with ID {}",
                request_id
            )));
        }

        Ok(())
    }

    /// Check if consensus is reached for a request.
    pub async fn check_consensus(&self, request_id: &str) -> Result<Option<ConsensusResult>> {
        let pending = self.pending.read().await;
        let request = pending.get(request_id).ok_or_else(|| {
            EaeError::ConsensusError(format!("No pending request with ID {}", request_id))
        })?;

        let votes = self.votes.read().await;
        let vote_list = votes.get(request_id).ok_or_else(|| {
            EaeError::ConsensusError(format!("No votes for request {}", request_id))
        })?;

        // Check if we have minimum participants
        if vote_list.len() < self.config.min_participants {
            // Check if deadline passed
            if chrono::Utc::now() > request.deadline {
                warn!(
                    request_id = %request_id,
                    votes_received = vote_list.len(),
                    min_required = self.config.min_participants,
                    "Consensus deadline passed without minimum participants"
                );
                return Ok(Some(self.finalize_consensus(request_id, vote_list, false).await));
            }
            return Ok(None); // Still waiting for votes
        }

        // Calculate consensus
        let approve_count = vote_list.iter().filter(|v| v.vote == VoteDecision::Approve).count();
        let reject_count = vote_list.iter().filter(|v| v.vote == VoteDecision::Reject).count();
        let total_decisive = approve_count + reject_count;

        if total_decisive == 0 {
            return Ok(None); // No decisive votes yet
        }

        let agreement_ratio = approve_count as f32 / total_decisive as f32;
        let reached = agreement_ratio >= self.config.default_threshold;

        info!(
            request_id = %request_id,
            approve = approve_count,
            reject = reject_count,
            ratio = agreement_ratio,
            reached = reached,
            "Consensus check completed"
        );

        Ok(Some(self.finalize_consensus(request_id, vote_list, reached).await))
    }

    /// Finalize consensus and move to completed.
    async fn finalize_consensus(
        &self,
        request_id: &str,
        votes: &[ConsensusVote],
        reached: bool,
    ) -> ConsensusResult {
        let pending = self.pending.read().await;
        let request = pending.get(request_id).unwrap();

        let approve_count = votes.iter().filter(|v| v.vote == VoteDecision::Approve).count();
        let reject_count = votes.iter().filter(|v| v.vote == VoteDecision::Reject).count();
        let abstain_count = votes.iter().filter(|v| v.vote == VoteDecision::Abstain).count();

        let total_decisive = approve_count + reject_count;
        let agreement_ratio = if total_decisive > 0 {
            approve_count as f32 / total_decisive as f32
        } else {
            0.0
        };

        let result = ConsensusResult {
            request_id: request_id.to_string(),
            decision_id: request.decision.id.clone(),
            reached,
            approve_count,
            reject_count,
            abstain_count,
            agreement_ratio,
            completed_at: chrono::Utc::now(),
        };

        // Store in completed
        {
            let mut completed = self.completed.write().await;
            completed.push(result.clone());

            // Keep history bounded
            while completed.len() > 1000 {
                completed.remove(0);
            }
        }

        result
    }

    /// Get pending requests.
    pub async fn pending_requests(&self) -> Vec<ConsensusRequest> {
        let pending = self.pending.read().await;
        pending.values().cloned().collect()
    }

    /// Get completed results.
    pub async fn completed_results(&self, limit: usize) -> Vec<ConsensusResult> {
        let completed = self.completed.read().await;
        completed.iter().rev().take(limit).cloned().collect()
    }

    /// Cancel a pending request.
    pub async fn cancel_request(&self, request_id: &str) -> Result<()> {
        let mut pending = self.pending.write().await;
        pending
            .remove(request_id)
            .ok_or_else(|| EaeError::ConsensusError(format!("No pending request with ID {}", request_id)))?;

        let mut votes = self.votes.write().await;
        votes.remove(request_id);

        info!(request_id = %request_id, "Consensus request cancelled");
        Ok(())
    }

    /// Simulate votes for testing (would be replaced by network calls in production).
    #[cfg(test)]
    pub async fn simulate_votes(&self, request_id: &str, approvals: usize, rejections: usize) {
        for i in 0..approvals {
            let vote = ConsensusVote {
                agent_id: format!("agent-approve-{}", i),
                vote: VoteDecision::Approve,
                reasoning: "Test approval".to_string(),
                timestamp: chrono::Utc::now(),
            };
            let _ = self.submit_vote(request_id, vote).await;
        }

        for i in 0..rejections {
            let vote = ConsensusVote {
                agent_id: format!("agent-reject-{}", i),
                vote: VoteDecision::Reject,
                reasoning: "Test rejection".to_string(),
                timestamp: chrono::Utc::now(),
            };
            let _ = self.submit_vote(request_id, vote).await;
        }
    }
}

impl Default for ConsensusManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Action, ActionType, DecisionReasoning, DecisionType};
    use constitution::ConstitutionalLayer;

    fn make_test_decision() -> Decision {
        Decision {
            id: "test-decision".to_string(),
            timestamp: chrono::Utc::now(),
            event_id: "test-event".to_string(),
            decision_type: DecisionType::HardIntervention,
            actions: vec![Action::new(ActionType::FilterContent, "test")],
            reasoning: DecisionReasoning {
                primary_principle: "Test".to_string(),
                interpretation: "Test interpretation".to_string(),
                matched_rules: vec![],
                llm_assisted: false,
                precedents_considered: vec![],
                determining_layer: ConstitutionalLayer::Community,
                stack_hash: "test-hash".to_string(),
            },
            confidence: 0.8,
            requires_consensus: true,
            consensus_status: None,
        }
    }

    #[tokio::test]
    async fn test_consensus_flow() {
        let mut config = ConsensusConfig::default();
        config.min_participants = 3;
        config.default_threshold = 0.67;

        let manager = ConsensusManager::with_config(config);

        let decision = make_test_decision();
        let request = manager.request_consensus(decision).await.unwrap();

        // Simulate votes: 3 approvals, 1 rejection = 75% >= 67% threshold
        manager.simulate_votes(&request.id, 3, 1).await;

        // Check consensus
        let result = manager.check_consensus(&request.id).await.unwrap();
        assert!(result.is_some());

        let result = result.unwrap();
        assert!(result.reached); // 3/4 = 0.75 >= 0.67 threshold
        assert_eq!(result.approve_count, 3);
        assert_eq!(result.reject_count, 1);
    }
}
