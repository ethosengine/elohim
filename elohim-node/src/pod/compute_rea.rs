//! REA compute accounting types.
//!
//! Every inference invocation produces an immutable economic event.
//! Commitments are created on admission (intent); events on fulfillment (fact).
//! Training-wheels: local storage via AuditLog. Production: Holochain DHT.

use serde::{Deserialize, Serialize};

/// Status of a compute commitment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommitmentStatus {
    Pending,
    Fulfilled,
    Cancelled,
}

/// REA Commitment — intent to serve a compute request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeCommitment {
    pub id: String,
    pub request_id: String,
    pub node_id: String,
    pub requester_id: String,
    pub capability: String,
    pub estimated_tokens: u32,
    pub status: CommitmentStatus,
    pub created_at: String,
    pub fulfilled_at: Option<String>,
    pub cancelled_at: Option<String>,
}

impl ComputeCommitment {
    pub fn new(
        request_id: String,
        node_id: String,
        requester_id: String,
        capability: String,
        estimated_tokens: u32,
    ) -> Self {
        Self {
            id: format!("commit-{}", uuid::Uuid::new_v4()),
            request_id,
            node_id,
            requester_id,
            capability,
            estimated_tokens,
            status: CommitmentStatus::Pending,
            created_at: chrono::Utc::now().to_rfc3339(),
            fulfilled_at: None,
            cancelled_at: None,
        }
    }

    pub fn fulfill(&mut self, tokens_used: u32, model: String, time_ms: u64) -> ComputeEvent {
        self.status = CommitmentStatus::Fulfilled;
        self.fulfilled_at = Some(chrono::Utc::now().to_rfc3339());

        ComputeEvent {
            id: format!("event-{}", uuid::Uuid::new_v4()),
            commitment_id: self.id.clone(),
            provider_id: self.node_id.clone(),
            receiver_id: self.requester_id.clone(),
            action: "use".into(),
            resource_type: "inference-tokens".into(),
            tokens_used,
            model,
            time_ms,
            capability: self.capability.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn cancel(&mut self) {
        self.status = CommitmentStatus::Cancelled;
        self.cancelled_at = Some(chrono::Utc::now().to_rfc3339());
    }
}

/// REA Economic Event — immutable record of compute consumed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeEvent {
    pub id: String,
    pub commitment_id: String,
    pub provider_id: String,
    pub receiver_id: String,
    pub action: String,
    pub resource_type: String,
    pub tokens_used: u32,
    pub model: String,
    pub time_ms: u64,
    pub capability: String,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment_lifecycle_fulfilled() {
        let mut commitment = ComputeCommitment::new(
            "req-1".into(),
            "node-A".into(),
            "user-1".into(),
            "path-recommendation".into(),
            1500,
        );
        assert_eq!(commitment.status, CommitmentStatus::Pending);
        assert!(commitment.fulfilled_at.is_none());

        let event = commitment.fulfill(1247, "claude-haiku-4-5".into(), 890);

        assert_eq!(commitment.status, CommitmentStatus::Fulfilled);
        assert!(commitment.fulfilled_at.is_some());
        assert_eq!(event.commitment_id, commitment.id);
        assert_eq!(event.tokens_used, 1247);
        assert_eq!(event.provider_id, "node-A");
        assert_eq!(event.receiver_id, "user-1");
        assert_eq!(event.action, "use");
        assert_eq!(event.resource_type, "inference-tokens");
    }

    #[test]
    fn test_commitment_lifecycle_cancelled() {
        let mut commitment = ComputeCommitment::new(
            "req-2".into(),
            "node-A".into(),
            "user-2".into(),
            "spiral-detection".into(),
            800,
        );
        commitment.cancel();

        assert_eq!(commitment.status, CommitmentStatus::Cancelled);
        assert!(commitment.cancelled_at.is_some());
        assert!(commitment.fulfilled_at.is_none());
    }

    #[test]
    fn test_event_links_commitment() {
        let mut commitment = ComputeCommitment::new(
            "req-3".into(),
            "node-B".into(),
            "user-3".into(),
            "content-safety-review".into(),
            2000,
        );
        let event = commitment.fulfill(1800, "claude-sonnet-4".into(), 1200);

        assert_eq!(event.commitment_id, commitment.id);
        assert_eq!(event.capability, "content-safety-review");
        assert_eq!(event.model, "claude-sonnet-4");
    }

    #[test]
    fn test_commitment_serialization() {
        let commitment = ComputeCommitment::new(
            "req-4".into(),
            "node-C".into(),
            "user-4".into(),
            "path-recommendation".into(),
            1000,
        );
        let json = serde_json::to_string(&commitment).unwrap();
        assert!(json.contains("requestId"));
        assert!(json.contains("nodeId"));
        assert!(json.contains("estimatedTokens"));
        assert!(json.contains("pending"));
    }

    #[test]
    fn test_event_serialization() {
        let mut commitment = ComputeCommitment::new(
            "req-5".into(),
            "node-D".into(),
            "user-5".into(),
            "spiral-detection".into(),
            500,
        );
        let event = commitment.fulfill(480, "llama-70b".into(), 3000);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("commitmentId"));
        assert!(json.contains("inference-tokens"));
        assert!(json.contains("llama-70b"));
    }
}
