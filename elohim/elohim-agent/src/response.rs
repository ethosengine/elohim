//! Elohim response types.
//!
//! Matches TypeScript `ElohimResponse` in elohim-agent.model.ts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::capability::ElohimCapability;
use crate::types::ComputationCost;

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// Status of an Elohim response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "lowercase")]
pub enum ResponseStatus {
    /// Request was fulfilled successfully
    Fulfilled,
    /// Request was declined (capability not available, unauthorized, etc.)
    Declined,
    /// Request was deferred for later processing
    Deferred,
    /// Request was escalated to a higher layer
    Escalated,
}

/// Response from an Elohim agent invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct ElohimResponse {
    /// Unique response identifier
    pub response_id: String,
    /// Request ID this responds to
    pub request_id: String,
    /// Elohim agent that handled this
    pub elohim_id: String,
    /// Response status
    pub status: ResponseStatus,
    /// Constitutional reasoning (always provided for transparency)
    pub constitutional_reasoning: ConstitutionalReasoning,
    /// Response payload (capability-specific)
    pub payload: ResponsePayload,
    /// Computation cost
    pub cost: ComputationCost,
    /// When the response was generated
    pub responded_at: DateTime<Utc>,
}

impl ElohimResponse {
    /// Create a successful response.
    pub fn fulfilled(
        request_id: impl Into<String>,
        elohim_id: impl Into<String>,
        reasoning: ConstitutionalReasoning,
        payload: ResponsePayload,
        cost: ComputationCost,
    ) -> Self {
        Self {
            response_id: uuid::Uuid::new_v4().to_string(),
            request_id: request_id.into(),
            elohim_id: elohim_id.into(),
            status: ResponseStatus::Fulfilled,
            constitutional_reasoning: reasoning,
            payload,
            cost,
            responded_at: Utc::now(),
        }
    }

    /// Create a declined response.
    pub fn declined(
        request_id: impl Into<String>,
        elohim_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            response_id: uuid::Uuid::new_v4().to_string(),
            request_id: request_id.into(),
            elohim_id: elohim_id.into(),
            status: ResponseStatus::Declined,
            constitutional_reasoning: ConstitutionalReasoning::declined(reason),
            payload: ResponsePayload::None,
            cost: ComputationCost::default(),
            responded_at: Utc::now(),
        }
    }

    /// Create an escalated response.
    pub fn escalated(
        request_id: impl Into<String>,
        elohim_id: impl Into<String>,
        to_layer: constitution::ConstitutionalLayer,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            response_id: uuid::Uuid::new_v4().to_string(),
            request_id: request_id.into(),
            elohim_id: elohim_id.into(),
            status: ResponseStatus::Escalated,
            constitutional_reasoning: ConstitutionalReasoning::escalated(to_layer, reason),
            payload: ResponsePayload::Escalated {
                to_layer,
                reason: "".to_string(),
            },
            cost: ComputationCost::default(),
            responded_at: Utc::now(),
        }
    }
}

/// Constitutional reasoning audit trail.
///
/// Every Elohim response includes this for transparency.
/// Matches TypeScript `ConstitutionalReasoning`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct ConstitutionalReasoning {
    /// Primary constitutional principle applied
    pub primary_principle: String,
    /// How the principle was interpreted for this case
    pub interpretation: String,
    /// Values that were weighed in the decision
    pub values_weighed: Vec<ValueWeight>,
    /// Confidence in the decision (0.0 - 1.0)
    pub confidence: f32,
    /// Precedents referenced in reasoning
    pub precedents: Vec<String>,
    /// Whether this creates new precedent
    pub new_precedent: bool,
    /// Hash of the constitutional stack used
    pub stack_hash: String,
    /// Layer that made the final determination
    pub determining_layer: constitution::ConstitutionalLayer,
}

impl ConstitutionalReasoning {
    /// Create reasoning for a declined request.
    pub fn declined(reason: impl Into<String>) -> Self {
        Self {
            primary_principle: "Request handling boundaries".to_string(),
            interpretation: reason.into(),
            values_weighed: vec![],
            confidence: 1.0,
            precedents: vec![],
            new_precedent: false,
            stack_hash: String::new(),
            determining_layer: constitution::ConstitutionalLayer::Individual,
        }
    }

    /// Create reasoning for an escalated request.
    pub fn escalated(
        to_layer: constitution::ConstitutionalLayer,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            primary_principle: "Subsidiarity".to_string(),
            interpretation: format!(
                "Request requires {} layer authority: {}",
                to_layer.as_str(),
                reason.into()
            ),
            values_weighed: vec![],
            confidence: 0.9,
            precedents: vec![],
            new_precedent: false,
            stack_hash: String::new(),
            determining_layer: to_layer,
        }
    }

    /// Create default reasoning.
    pub fn default_for_capability(capability: ElohimCapability) -> Self {
        Self {
            primary_principle: "Capability fulfillment".to_string(),
            interpretation: format!(
                "Processed {} request per constitutional guidelines",
                capability.description()
            ),
            values_weighed: vec![],
            confidence: 0.8,
            precedents: vec![],
            new_precedent: false,
            stack_hash: String::new(),
            determining_layer: capability
                .required_layer()
                .unwrap_or(constitution::ConstitutionalLayer::Individual),
        }
    }
}

/// A value that was weighed in the decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct ValueWeight {
    /// Name of the value
    pub value: String,
    /// Weight given to this value (0.0 - 1.0)
    pub weight: f32,
    /// Whether this value argued for or against the decision
    pub direction: ValueDirection,
}

/// Direction a value argued.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "lowercase")]
pub enum ValueDirection {
    /// Value supports the decision
    For,
    /// Value opposes the decision
    Against,
}

/// Capability-specific response payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponsePayload {
    /// No payload (for declined requests, etc.)
    None,

    /// Content safety review result
    SafetyReview {
        safe: bool,
        issues: Vec<SafetyIssue>,
        recommendation: String,
    },

    /// Accuracy verification result
    AccuracyVerification {
        verified: bool,
        confidence: f32,
        issues: Vec<AccuracyIssue>,
    },

    /// Attestation recommendation
    AttestationRecommendation {
        recommend: bool,
        attestation_type: String,
        rationale: String,
    },

    /// Spiral detection result
    SpiralDetection {
        detected: bool,
        severity: Option<String>,
        signals: Vec<String>,
        suggested_response: Option<String>,
    },

    /// Path recommendation
    PathRecommendation {
        recommended_paths: Vec<RecommendedPath>,
    },

    /// Knowledge map synthesis
    KnowledgeMap {
        nodes: Vec<KnowledgeNode>,
        edges: Vec<KnowledgeEdge>,
    },

    /// Generic JSON payload
    Generic {
        data: serde_json::Value,
    },

    /// Escalated to another layer
    Escalated {
        to_layer: constitution::ConstitutionalLayer,
        reason: String,
    },
}

/// A safety issue found during review.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct SafetyIssue {
    pub category: String,
    pub severity: String,
    pub description: String,
    pub location: Option<String>,
}

/// An accuracy issue found during verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct AccuracyIssue {
    pub claim: String,
    pub issue: String,
    pub suggestion: Option<String>,
}

/// A recommended learning path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct RecommendedPath {
    pub path_id: String,
    pub title: String,
    pub relevance: f32,
    pub rationale: String,
}

/// A node in a knowledge map.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct KnowledgeNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
}

/// An edge in a knowledge map.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct KnowledgeEdge {
    pub source: String,
    pub target: String,
    pub relation: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fulfilled_response() {
        let response = ElohimResponse::fulfilled(
            "req-123",
            "elohim-456",
            ConstitutionalReasoning::default_for_capability(ElohimCapability::ContentSafetyReview),
            ResponsePayload::SafetyReview {
                safe: true,
                issues: vec![],
                recommendation: "Content is safe".to_string(),
            },
            ComputationCost::default(),
        );

        assert_eq!(response.status, ResponseStatus::Fulfilled);
        assert_eq!(response.request_id, "req-123");
    }

    #[test]
    fn test_declined_response() {
        let response = ElohimResponse::declined("req-123", "elohim-456", "Capability not available");

        assert_eq!(response.status, ResponseStatus::Declined);
    }
}
