//! Admission control types for elohim compute coordination.
//!
//! These types define the contract between request admission and response.
//! Training-wheels: single-node admission with correct shapes for mesh routing.

use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RequestPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Urgent = 3,
}

impl Default for RequestPriority {
    fn default() -> Self {
        Self::Normal
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
}
