//! Admission control types for elohim compute coordination.
//!
//! These types define the contract between request admission and response.
//! Training-wheels: single-node admission with correct shapes for mesh routing.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

/// Result of admission evaluation for an incoming compute request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "decision",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AdmissionDecision {
    /// Request accepted — will be queued and processed.
    Accepted {
        commitment_id: String,
        queue_position: u32,
        estimated_wait_ms: u64,
    },
    /// Request deferred — node is at capacity, try later or elsewhere.
    Deferred {
        reason: DeferReason,
        mesh_hints: Vec<MeshHint>,
        retry_after_ms: u64,
    },
    /// Request declined — cannot serve this request at all.
    Declined { reason: String },
}

/// Why a request was deferred (not declined — it could be served later).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum DeferReason {
    BudgetExhausted,
    QueueFull,
    CapabilityUnavailable,
    SystemPressure,
}

/// Hint about a neighbor node that might have capacity.
/// Training-wheels: empty vec, populated when gossip neighbor table is built.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeshHint {
    pub node_id: String,
    pub budget_remaining: u32,
    pub estimated_wait_ms: u64,
    pub capabilities: Vec<String>,
}

/// Priority levels for queue ordering. Maps to ElohimRequest.priority.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RequestPriority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
    Urgent = 3,
}

/// Configuration for the admission controller.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Training-wheels: max_concurrent and node_id used when mesh routing is wired
pub struct AdmissionConfig {
    pub budget_limit: u32,
    pub max_queue_depth: u32,
    pub max_concurrent: u32,
    pub capabilities: Vec<String>,
    pub node_id: String,
}

/// Queued request entry.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Training-wheels: fields used when dequeue processing is wired
pub struct QueuedRequest {
    pub request_id: String,
    pub requester_id: String,
    pub capability: String,
    pub priority: RequestPriority,
    pub commitment_id: String,
    pub enqueued_at: String,
}

/// Admission controller — evaluates, queues, and tracks compute requests.
pub struct AdmissionController {
    config: AdmissionConfig,
    budget_remaining: AtomicU32,
    queue: Mutex<Vec<QueuedRequest>>,
    active_count: AtomicU32,
}

impl AdmissionController {
    pub fn new(config: AdmissionConfig) -> Self {
        let budget = config.budget_limit;
        Self {
            config,
            budget_remaining: AtomicU32::new(budget),
            queue: Mutex::new(Vec::new()),
            active_count: AtomicU32::new(0),
        }
    }

    pub fn evaluate(
        &self,
        request_id: &str,
        requester_id: &str,
        capability: &str,
        priority: RequestPriority,
    ) -> AdmissionDecision {
        if !self.config.capabilities.iter().any(|c| c == capability) {
            return AdmissionDecision::Declined {
                reason: format!("Capability '{}' not available on this node", capability),
            };
        }

        // Acquire queue lock before budget check to prevent TOCTOU race
        // where concurrent requests all see budget > 0 before any enqueues
        let mut queue = self.queue.lock().unwrap();

        if self.budget_remaining.load(Ordering::Acquire) == 0 {
            return AdmissionDecision::Deferred {
                reason: DeferReason::BudgetExhausted,
                mesh_hints: vec![],
                retry_after_ms: 30_000,
            };
        }

        if queue.len() as u32 >= self.config.max_queue_depth {
            return AdmissionDecision::Deferred {
                reason: DeferReason::QueueFull,
                mesh_hints: vec![],
                retry_after_ms: 10_000,
            };
        }

        let commitment_id = format!("commit-{}", uuid::Uuid::new_v4());

        queue.push(QueuedRequest {
            request_id: request_id.into(),
            requester_id: requester_id.into(),
            capability: capability.into(),
            priority,
            commitment_id: commitment_id.clone(),
            enqueued_at: chrono::Utc::now().to_rfc3339(),
        });

        // Sort by priority (urgent first), then find actual position post-sort
        queue.sort_by(|a, b| b.priority.cmp(&a.priority));
        let position = queue
            .iter()
            .position(|r| r.commitment_id == commitment_id)
            .unwrap_or(0) as u32;

        AdmissionDecision::Accepted {
            commitment_id,
            queue_position: position,
            estimated_wait_ms: (position as u64 + 1) * 2000,
        }
    }

    pub fn record_usage(&self, tokens: u32) {
        // CAS loop to prevent underflow — saturates at 0
        loop {
            let current = self.budget_remaining.load(Ordering::Acquire);
            let new = current.saturating_sub(tokens);
            if self
                .budget_remaining
                .compare_exchange_weak(current, new, Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
        }
    }

    pub fn dequeue(&self) -> Option<QueuedRequest> {
        let mut queue = self.queue.lock().unwrap();
        if queue.is_empty() {
            None
        } else {
            Some(queue.remove(0))
        }
    }

    pub fn cancel(&self, request_id: &str) -> bool {
        let mut queue = self.queue.lock().unwrap();
        let before = queue.len();
        queue.retain(|r| r.request_id != request_id);
        queue.len() < before
    }

    pub fn budget_remaining(&self) -> u32 {
        self.budget_remaining.load(Ordering::Relaxed)
    }

    pub fn queue_depth(&self) -> u32 {
        self.queue.lock().unwrap().len() as u32
    }

    pub fn active_requests(&self) -> u32 {
        self.active_count.load(Ordering::Relaxed)
    }

    pub fn mark_active(&self) {
        self.active_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn mark_complete(&self) {
        self.active_count.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admission_accepted_serializes() {
        let decision = AdmissionDecision::Accepted {
            commitment_id: "commit-123".into(),
            queue_position: 2,
            estimated_wait_ms: 5000,
        };
        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("accepted"));
        assert!(json.contains("commitmentId"));
    }

    #[test]
    fn test_admission_deferred_with_empty_hints() {
        let decision = AdmissionDecision::Deferred {
            reason: DeferReason::BudgetExhausted,
            mesh_hints: vec![],
            retry_after_ms: 10000,
        };
        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("deferred"));
        assert!(json.contains("meshHints"));
        assert!(json.contains("[]"));
    }

    #[test]
    fn test_admission_deferred_with_mesh_hints() {
        let decision = AdmissionDecision::Deferred {
            reason: DeferReason::QueueFull,
            mesh_hints: vec![MeshHint {
                node_id: "node-456".into(),
                budget_remaining: 50,
                estimated_wait_ms: 2000,
                capabilities: vec!["path-recommendation".into()],
            }],
            retry_after_ms: 5000,
        };
        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("node-456"));
        assert!(json.contains("path-recommendation"));
    }

    #[test]
    fn test_priority_ordering() {
        assert!(RequestPriority::Urgent > RequestPriority::High);
        assert!(RequestPriority::High > RequestPriority::Normal);
        assert!(RequestPriority::Normal > RequestPriority::Low);
    }

    #[test]
    fn test_defer_reason_equality() {
        assert_eq!(DeferReason::BudgetExhausted, DeferReason::BudgetExhausted);
        assert_ne!(DeferReason::QueueFull, DeferReason::SystemPressure);
    }

    #[test]
    fn test_accept_within_budget() {
        let controller = AdmissionController::new(AdmissionConfig {
            budget_limit: 100,
            max_queue_depth: 10,
            max_concurrent: 5,
            capabilities: vec!["path-recommendation".into()],
            node_id: "node-test".into(),
        });
        let decision = controller.evaluate(
            "req-1",
            "user-1",
            "path-recommendation",
            RequestPriority::Normal,
        );
        assert!(matches!(decision, AdmissionDecision::Accepted { .. }));
    }

    #[test]
    fn test_defer_budget_exhausted() {
        let controller = AdmissionController::new(AdmissionConfig {
            budget_limit: 0,
            max_queue_depth: 10,
            max_concurrent: 5,
            capabilities: vec!["path-recommendation".into()],
            node_id: "node-test".into(),
        });
        let decision = controller.evaluate(
            "req-1",
            "user-1",
            "path-recommendation",
            RequestPriority::Normal,
        );
        match decision {
            AdmissionDecision::Deferred {
                reason, mesh_hints, ..
            } => {
                assert_eq!(reason, DeferReason::BudgetExhausted);
                assert!(mesh_hints.is_empty());
            }
            other => panic!("Expected Deferred, got {:?}", other),
        }
    }

    #[test]
    fn test_decline_unknown_capability() {
        let controller = AdmissionController::new(AdmissionConfig {
            budget_limit: 100,
            max_queue_depth: 10,
            max_concurrent: 5,
            capabilities: vec!["path-recommendation".into()],
            node_id: "node-test".into(),
        });
        let decision = controller.evaluate(
            "req-1",
            "user-1",
            "unknown-capability",
            RequestPriority::Normal,
        );
        assert!(matches!(decision, AdmissionDecision::Declined { .. }));
    }

    #[test]
    fn test_defer_queue_full() {
        let controller = AdmissionController::new(AdmissionConfig {
            budget_limit: 100,
            max_queue_depth: 2,
            max_concurrent: 5,
            capabilities: vec!["path-recommendation".into()],
            node_id: "node-test".into(),
        });
        controller.evaluate(
            "req-1",
            "user-1",
            "path-recommendation",
            RequestPriority::Normal,
        );
        controller.evaluate(
            "req-2",
            "user-2",
            "path-recommendation",
            RequestPriority::Normal,
        );
        let decision = controller.evaluate(
            "req-3",
            "user-3",
            "path-recommendation",
            RequestPriority::Normal,
        );
        match decision {
            AdmissionDecision::Deferred { reason, .. } => {
                assert_eq!(reason, DeferReason::QueueFull);
            }
            other => panic!("Expected Deferred, got {:?}", other),
        }
    }

    #[test]
    fn test_budget_decrements_on_fulfill() {
        let controller = AdmissionController::new(AdmissionConfig {
            budget_limit: 10,
            max_queue_depth: 10,
            max_concurrent: 5,
            capabilities: vec!["path-recommendation".into()],
            node_id: "node-test".into(),
        });
        assert_eq!(controller.budget_remaining(), 10);
        controller.record_usage(3);
        assert_eq!(controller.budget_remaining(), 7);
    }
}
