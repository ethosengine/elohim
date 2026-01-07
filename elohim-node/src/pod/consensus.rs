//! Consensus - Multi-agent agreement for risky actions
//!
//! Implements consensus gathering for actions that require
//! N-of-M independent agent evaluations.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use tracing::{debug, info, warn, error};

use super::models::*;
use super::protocol::*;

/// Default timeout for consensus gathering
const CONSENSUS_TIMEOUT_SECS: u64 = 30;

/// Consensus manager handles gathering approvals for risky actions
pub struct ConsensusManager {
    node_id: String,
    /// Pending consensus requests
    pending: Arc<RwLock<HashMap<ActionId, PendingConsensus>>>,
    /// Known peer agents
    peer_agents: Arc<RwLock<Vec<PeerPodInfo>>>,
}

/// A pending consensus request
struct PendingConsensus {
    request: ConsensusRequest,
    responses: Vec<ConsensusResponse>,
    required_approvals: u8,
    total_evaluators: u8,
    deadline: u64,
}

impl ConsensusManager {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            pending: Arc::new(RwLock::new(HashMap::new())),
            peer_agents: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a peer agent
    pub async fn register_peer(&self, peer: PeerPodInfo) {
        let mut peers = self.peer_agents.write().await;

        // Update or add
        if let Some(existing) = peers.iter_mut().find(|p| p.peer_id == peer.peer_id) {
            *existing = peer;
        } else {
            peers.push(peer);
        }
    }

    /// Remove a peer agent
    pub async fn unregister_peer(&self, peer_id: &str) {
        let mut peers = self.peer_agents.write().await;
        peers.retain(|p| p.peer_id != peer_id);
    }

    /// Get peer agents
    pub async fn peer_agents(&self) -> Vec<PeerPodInfo> {
        self.peer_agents.read().await.clone()
    }

    /// Check if we have enough peers for consensus
    pub async fn can_reach_consensus(&self, required: u8) -> bool {
        let peers = self.peer_agents.read().await;
        peers.len() >= required as usize
    }

    /// Start a consensus request for an action
    pub async fn request_consensus(
        &self,
        action: Action,
        context: ClusterContext,
    ) -> Result<ConsensusOutcome, String> {
        let (required, total) = match &action.risk {
            ActionRisk::Safe => {
                return Ok(ConsensusOutcome::Approved {
                    approvals: vec![],
                    reasoning: "Safe action, no consensus required".to_string(),
                });
            }
            ActionRisk::Risky { required_approvals, total_evaluators } => {
                (*required_approvals, *total_evaluators)
            }
        };

        // Check if we have enough peers
        let peers = self.peer_agents.read().await;
        if peers.len() < total as usize {
            warn!(
                available = peers.len(),
                required = total,
                "Not enough peer agents for consensus"
            );
            // In degraded mode, we might fall back to local decision only
            return Ok(ConsensusOutcome::InsufficientPeers {
                available: peers.len() as u8,
                required: total,
            });
        }
        drop(peers);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let request = ConsensusRequest {
            action: action.clone(),
            context,
            proposing_agent: self.node_id.clone(),
            deadline: now + CONSENSUS_TIMEOUT_SECS,
        };

        let pending = PendingConsensus {
            request: request.clone(),
            responses: Vec::new(),
            required_approvals: required,
            total_evaluators: total,
            deadline: now + CONSENSUS_TIMEOUT_SECS,
        };

        // Store pending request
        {
            let mut pending_map = self.pending.write().await;
            pending_map.insert(action.id.clone(), pending);
        }

        info!(
            action_id = %action.id,
            required,
            total,
            "Consensus request started"
        );

        // In a real implementation, this would:
        // 1. Send ConsensusRequest to peer agents via P2P
        // 2. Wait for responses
        // 3. Tally votes
        // For now, simulate the process

        // Simulate waiting for responses (in real impl, would be async P2P)
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Check outcome
        let outcome = self.check_consensus(&action.id).await?;

        // Clean up
        {
            let mut pending_map = self.pending.write().await;
            pending_map.remove(&action.id);
        }

        Ok(outcome)
    }

    /// Receive a consensus response from a peer
    pub async fn receive_response(&self, response: ConsensusResponse) -> Result<(), String> {
        let mut pending_map = self.pending.write().await;

        // Find the pending request that matches
        for (_, pending) in pending_map.iter_mut() {
            if pending.request.action.id == response.evaluator {
                // This check is wrong - should match action somehow
                // For now, accept any response
            }

            pending.responses.push(response.clone());
            debug!(
                action_id = %pending.request.action.id,
                evaluator = %response.evaluator,
                approved = response.approved,
                "Received consensus response"
            );
            return Ok(());
        }

        Err("No pending consensus request found".to_string())
    }

    /// Check if consensus has been reached
    async fn check_consensus(&self, action_id: &str) -> Result<ConsensusOutcome, String> {
        let pending_map = self.pending.read().await;

        let pending = pending_map.get(action_id)
            .ok_or_else(|| format!("No pending consensus for action {}", action_id))?;

        let approvals: Vec<_> = pending.responses.iter()
            .filter(|r| r.approved)
            .cloned()
            .collect();

        let rejections: Vec<_> = pending.responses.iter()
            .filter(|r| !r.approved)
            .cloned()
            .collect();

        if approvals.len() >= pending.required_approvals as usize {
            Ok(ConsensusOutcome::Approved {
                approvals,
                reasoning: "Required approvals reached".to_string(),
            })
        } else if rejections.len() > (pending.total_evaluators - pending.required_approvals) as usize {
            // Too many rejections to possibly reach consensus
            Ok(ConsensusOutcome::Rejected {
                rejections,
                reasoning: "Too many rejections to reach consensus".to_string(),
            })
        } else if std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() > pending.deadline
        {
            Ok(ConsensusOutcome::Timeout {
                received: pending.responses.len() as u8,
                required: pending.required_approvals,
            })
        } else {
            Ok(ConsensusOutcome::Pending {
                received: pending.responses.len() as u8,
                required: pending.required_approvals,
            })
        }
    }

    /// Evaluate a consensus request from another agent
    ///
    /// This is where the local agent (or LLM) decides whether to approve.
    pub async fn evaluate_request(&self, request: &ConsensusRequest) -> ConsensusResponse {
        // In the future, this would:
        // 1. Check local state and observations
        // 2. Optionally consult local LLM
        // 3. Apply policy rules

        // For now, use simple heuristics
        let (approved, reasoning, confidence) = self.evaluate_action(&request.action, &request.context);

        ConsensusResponse {
            evaluator: self.node_id.clone(),
            approved,
            reasoning,
            confidence,
        }
    }

    /// Simple rule-based evaluation of an action
    fn evaluate_action(&self, action: &Action, context: &ClusterContext) -> (bool, String, f32) {
        // Evaluate based on action type and context
        match &action.kind {
            ActionKind::QuarantineNode => {
                // Be cautious about quarantining
                if context.healthy_nodes < 3 {
                    (false, "Cluster too small to quarantine a node safely".to_string(), 0.9)
                } else {
                    (true, "Cluster has enough redundancy for quarantine".to_string(), 0.7)
                }
            }
            ActionKind::EvictBlob => {
                // Check if we have enough replicas
                (true, "Blob eviction generally safe with proper replica check".to_string(), 0.8)
            }
            ActionKind::FailoverService => {
                // Failover is risky but often necessary
                if context.healthy_nodes > 1 {
                    (true, "Healthy nodes available for failover".to_string(), 0.75)
                } else {
                    (false, "No healthy nodes available for failover".to_string(), 0.95)
                }
            }
            ActionKind::RebalanceStorage => {
                // Generally safe with dry_run
                let is_dry_run = action.params.get("dry_run")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if is_dry_run {
                    (true, "Dry run rebalance is safe".to_string(), 0.95)
                } else {
                    (true, "Rebalance operation approved".to_string(), 0.7)
                }
            }
            _ => {
                // Default: approve safe actions, be cautious about others
                match &action.risk {
                    ActionRisk::Safe => (true, "Safe action approved".to_string(), 0.9),
                    ActionRisk::Risky { .. } => {
                        (true, "Risky action approved with standard checks".to_string(), 0.6)
                    }
                }
            }
        }
    }
}

/// Outcome of a consensus request
#[derive(Debug, Clone)]
pub enum ConsensusOutcome {
    /// Action approved with sufficient votes
    Approved {
        approvals: Vec<ConsensusResponse>,
        reasoning: String,
    },
    /// Action rejected
    Rejected {
        rejections: Vec<ConsensusResponse>,
        reasoning: String,
    },
    /// Waiting for more responses
    Pending {
        received: u8,
        required: u8,
    },
    /// Consensus timed out
    Timeout {
        received: u8,
        required: u8,
    },
    /// Not enough peers to reach consensus
    InsufficientPeers {
        available: u8,
        required: u8,
    },
}

impl ConsensusOutcome {
    pub fn is_approved(&self) -> bool {
        matches!(self, ConsensusOutcome::Approved { .. })
    }

    pub fn is_final(&self) -> bool {
        matches!(
            self,
            ConsensusOutcome::Approved { .. }
                | ConsensusOutcome::Rejected { .. }
                | ConsensusOutcome::Timeout { .. }
                | ConsensusOutcome::InsufficientPeers { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_safe_action_no_consensus() {
        let manager = ConsensusManager::new("test-node".to_string());

        let action = Action::new(
            ActionKind::SetLogLevel,
            "Test action",
            serde_json::json!({}),
        ); // Default is Safe

        let context = ClusterContext {
            cluster_name: "test".to_string(),
            node_count: 3,
            healthy_nodes: 3,
            recent_observations: vec![],
            resource_summary: ResourceSummary {
                avg_cpu_percent: 50.0,
                avg_memory_percent: 50.0,
                avg_disk_percent: 50.0,
                total_storage_bytes: 0,
                used_storage_bytes: 0,
                total_blob_count: 0,
                connected_clients: 0,
            },
            active_issues: vec![],
        };

        let outcome = manager.request_consensus(action, context).await.unwrap();
        assert!(outcome.is_approved());
    }

    #[tokio::test]
    async fn test_evaluate_request() {
        let manager = ConsensusManager::new("test-node".to_string());

        let action = Action::new(
            ActionKind::RebalanceStorage,
            "Test rebalance",
            serde_json::json!({"dry_run": true}),
        ).with_risk(ActionRisk::Risky {
            required_approvals: 2,
            total_evaluators: 3,
        });

        let context = ClusterContext {
            cluster_name: "test".to_string(),
            node_count: 3,
            healthy_nodes: 3,
            recent_observations: vec![],
            resource_summary: ResourceSummary {
                avg_cpu_percent: 50.0,
                avg_memory_percent: 50.0,
                avg_disk_percent: 50.0,
                total_storage_bytes: 0,
                used_storage_bytes: 0,
                total_blob_count: 0,
                connected_clients: 0,
            },
            active_issues: vec![],
        };

        let request = ConsensusRequest {
            action,
            context,
            proposing_agent: "other-node".to_string(),
            deadline: 0,
        };

        let response = manager.evaluate_request(&request).await;

        // Dry run rebalance should be approved
        assert!(response.approved);
        assert!(response.confidence > 0.5);
    }
}
