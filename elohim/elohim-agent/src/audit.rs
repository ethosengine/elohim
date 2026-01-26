//! Audit trail for Elohim agent invocations.
//!
//! Provides transparency by logging all requests and responses.

use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::capability::ElohimCapability;
use crate::request::ElohimRequest;
use crate::response::{ConstitutionalReasoning, ElohimResponse, ResponseStatus};
use crate::types::ComputationCost;

/// Maximum entries in the audit log before pruning.
const MAX_AUDIT_ENTRIES: usize = 10_000;

/// An entry in the audit log.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// Unique entry ID
    pub entry_id: String,
    /// Request ID
    pub request_id: String,
    /// Response ID (if available)
    pub response_id: Option<String>,
    /// Capability invoked
    pub capability: ElohimCapability,
    /// Who made the request
    pub requester_id: String,
    /// Elohim that handled the request
    pub elohim_id: Option<String>,
    /// Response status
    pub status: Option<ResponseStatus>,
    /// Constitutional reasoning
    pub reasoning: Option<ConstitutionalReasoning>,
    /// Computation cost
    pub cost: Option<ComputationCost>,
    /// When the request was made
    pub requested_at: DateTime<Utc>,
    /// When the response was generated
    pub responded_at: Option<DateTime<Utc>>,
    /// Processing duration in ms
    pub duration_ms: Option<u64>,
}

impl AuditEntry {
    /// Create an entry from a request.
    pub fn from_request(request: &ElohimRequest) -> Self {
        Self {
            entry_id: uuid::Uuid::new_v4().to_string(),
            request_id: request.request_id.clone(),
            response_id: None,
            capability: request.capability,
            requester_id: request.requester_id.clone(),
            elohim_id: None,
            status: None,
            reasoning: None,
            cost: None,
            requested_at: request.requested_at,
            responded_at: None,
            duration_ms: None,
        }
    }

    /// Update with response data.
    pub fn with_response(mut self, response: &ElohimResponse) -> Self {
        self.response_id = Some(response.response_id.clone());
        self.elohim_id = Some(response.elohim_id.clone());
        self.status = Some(response.status);
        self.reasoning = Some(response.constitutional_reasoning.clone());
        self.cost = Some(response.cost.clone());
        self.responded_at = Some(response.responded_at);
        self.duration_ms = Some(
            (response.responded_at - self.requested_at)
                .num_milliseconds()
                .max(0) as u64,
        );
        self
    }
}

/// Audit log for tracking all Elohim invocations.
pub struct AuditLog {
    /// Log entries (newest first)
    entries: Arc<RwLock<VecDeque<AuditEntry>>>,
    /// Maximum entries to retain
    max_entries: usize,
}

impl AuditLog {
    /// Create a new audit log.
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(VecDeque::new())),
            max_entries: MAX_AUDIT_ENTRIES,
        }
    }

    /// Create with custom max entries.
    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(VecDeque::new())),
            max_entries,
        }
    }

    /// Log a request (before processing).
    pub async fn log_request(&self, request: &ElohimRequest) -> String {
        let entry = AuditEntry::from_request(request);
        let entry_id = entry.entry_id.clone();

        let mut entries = self.entries.write().await;
        entries.push_front(entry);

        // Prune if over limit
        while entries.len() > self.max_entries {
            entries.pop_back();
        }

        entry_id
    }

    /// Update entry with response.
    pub async fn log_response(&self, entry_id: &str, response: &ElohimResponse) {
        let mut entries = self.entries.write().await;

        if let Some(entry) = entries.iter_mut().find(|e| e.entry_id == entry_id) {
            entry.response_id = Some(response.response_id.clone());
            entry.elohim_id = Some(response.elohim_id.clone());
            entry.status = Some(response.status);
            entry.reasoning = Some(response.constitutional_reasoning.clone());
            entry.cost = Some(response.cost.clone());
            entry.responded_at = Some(response.responded_at);
            entry.duration_ms = Some(
                (response.responded_at - entry.requested_at)
                    .num_milliseconds()
                    .max(0) as u64,
            );
        }
    }

    /// Get recent entries.
    pub async fn recent(&self, limit: usize) -> Vec<AuditEntry> {
        let entries = self.entries.read().await;
        entries.iter().take(limit).cloned().collect()
    }

    /// Get entry by request ID.
    pub async fn get_by_request(&self, request_id: &str) -> Option<AuditEntry> {
        let entries = self.entries.read().await;
        entries.iter().find(|e| e.request_id == request_id).cloned()
    }

    /// Get entries for a requester.
    pub async fn get_by_requester(&self, requester_id: &str, limit: usize) -> Vec<AuditEntry> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| e.requester_id == requester_id)
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get entries for a capability.
    pub async fn get_by_capability(
        &self,
        capability: ElohimCapability,
        limit: usize,
    ) -> Vec<AuditEntry> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| e.capability == capability)
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get statistics.
    pub async fn stats(&self) -> AuditStats {
        let entries = self.entries.read().await;

        let total = entries.len();
        let fulfilled = entries
            .iter()
            .filter(|e| e.status == Some(ResponseStatus::Fulfilled))
            .count();
        let declined = entries
            .iter()
            .filter(|e| e.status == Some(ResponseStatus::Declined))
            .count();
        let escalated = entries
            .iter()
            .filter(|e| e.status == Some(ResponseStatus::Escalated))
            .count();

        let avg_duration_ms = if total > 0 {
            entries
                .iter()
                .filter_map(|e| e.duration_ms)
                .sum::<u64>()
                / total as u64
        } else {
            0
        };

        AuditStats {
            total_requests: total,
            fulfilled,
            declined,
            escalated,
            avg_duration_ms,
        }
    }

    /// Clear the log.
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }

    /// Get count.
    pub async fn count(&self) -> usize {
        let entries = self.entries.read().await;
        entries.len()
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics from the audit log.
#[derive(Debug, Clone)]
pub struct AuditStats {
    /// Total requests logged
    pub total_requests: usize,
    /// Fulfilled requests
    pub fulfilled: usize,
    /// Declined requests
    pub declined: usize,
    /// Escalated requests
    pub escalated: usize,
    /// Average processing duration
    pub avg_duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::{ElohimRequest, RequestParams};
    use crate::response::{ElohimResponse, ResponsePayload};
    use crate::types::ComputationCost;

    #[tokio::test]
    async fn test_audit_log() {
        let log = AuditLog::new();

        let request = ElohimRequest::new(ElohimCapability::ContentSafetyReview, "user-123")
            .with_params(RequestParams::with_content("Test content"));

        let entry_id = log.log_request(&request).await;

        // Check entry was created
        let entry = log.get_by_request(&request.request_id).await.unwrap();
        assert_eq!(entry.capability, ElohimCapability::ContentSafetyReview);
        assert!(entry.status.is_none());

        // Log response
        let response = ElohimResponse::fulfilled(
            &request.request_id,
            "elohim-1",
            crate::response::ConstitutionalReasoning::default_for_capability(
                ElohimCapability::ContentSafetyReview,
            ),
            ResponsePayload::SafetyReview {
                safe: true,
                issues: vec![],
                recommendation: "Safe".to_string(),
            },
            ComputationCost::default(),
        );

        log.log_response(&entry_id, &response).await;

        // Check entry was updated
        let entry = log.get_by_request(&request.request_id).await.unwrap();
        assert_eq!(entry.status, Some(ResponseStatus::Fulfilled));
        assert!(entry.duration_ms.is_some());
    }

    #[tokio::test]
    async fn test_audit_stats() {
        let log = AuditLog::new();

        // Log a few requests
        for i in 0..5 {
            let request =
                ElohimRequest::new(ElohimCapability::ContentSafetyReview, format!("user-{}", i));
            log.log_request(&request).await;
        }

        let stats = log.stats().await;
        assert_eq!(stats.total_requests, 5);
    }
}
