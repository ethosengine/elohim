//! Elohim request types.
//!
//! Matches TypeScript `ElohimRequest` in elohim-agent.model.ts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::capability::ElohimCapability;

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// Priority levels for requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "lowercase")]
pub enum RequestPriority {
    /// Background processing, can be delayed
    Low,
    /// Standard priority
    #[default]
    Normal,
    /// Should be processed soon
    High,
    /// Needs immediate attention (care situations, safety)
    Urgent,
}

/// Request to invoke an Elohim agent capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct ElohimRequest {
    /// Unique request identifier
    pub request_id: String,
    /// Target Elohim agent ID, or "auto" for automatic selection
    pub target_elohim_id: String,
    /// Capability being invoked
    pub capability: ElohimCapability,
    /// Request parameters
    pub params: RequestParams,
    /// Who is making the request
    pub requester_id: String,
    /// Request priority
    pub priority: RequestPriority,
    /// When the request was made
    pub requested_at: DateTime<Utc>,
    /// Optional context about the request
    pub context: Option<RequestContext>,
}

impl ElohimRequest {
    /// Create a new request.
    pub fn new(
        capability: ElohimCapability,
        requester_id: impl Into<String>,
    ) -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            target_elohim_id: "auto".to_string(),
            capability,
            params: RequestParams::default(),
            requester_id: requester_id.into(),
            priority: RequestPriority::Normal,
            requested_at: Utc::now(),
            context: None,
        }
    }

    /// Set target Elohim agent.
    pub fn with_target(mut self, target_id: impl Into<String>) -> Self {
        self.target_elohim_id = target_id.into();
        self
    }

    /// Set priority.
    pub fn with_priority(mut self, priority: RequestPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set parameters.
    pub fn with_params(mut self, params: RequestParams) -> Self {
        self.params = params;
        self
    }

    /// Set context.
    pub fn with_context(mut self, context: RequestContext) -> Self {
        self.context = Some(context);
        self
    }
}

/// Parameters for a request (capability-specific).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct RequestParams {
    /// Content to analyze (for content operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Content ID (for content operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_id: Option<String>,
    /// Agent ID (for agent operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Path ID (for path operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_id: Option<String>,
    /// Query string (for search/analysis)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// Additional parameters as JSON
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub extra: serde_json::Value,
}

impl RequestParams {
    /// Create params with content.
    pub fn with_content(content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            ..Default::default()
        }
    }

    /// Create params with content ID.
    pub fn for_content_id(content_id: impl Into<String>) -> Self {
        Self {
            content_id: Some(content_id.into()),
            ..Default::default()
        }
    }

    /// Create params with agent ID.
    pub fn for_agent(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: Some(agent_id.into()),
            ..Default::default()
        }
    }

    /// Create params with path ID.
    pub fn for_path(path_id: impl Into<String>) -> Self {
        Self {
            path_id: Some(path_id.into()),
            ..Default::default()
        }
    }

    /// Add extra parameters.
    pub fn with_extra(mut self, key: &str, value: impl Serialize) -> Self {
        if self.extra.is_null() {
            self.extra = serde_json::json!({});
        }
        if let Some(obj) = self.extra.as_object_mut() {
            obj.insert(key.to_string(), serde_json::to_value(value).unwrap_or_default());
        }
        self
    }
}

/// Context for a request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct RequestContext {
    /// Why this request is being made
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Previous related request IDs
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_requests: Vec<String>,
    /// Constitutional layer context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer: Option<constitution::ConstitutionalLayer>,
    /// Community context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub community_id: Option<String>,
    /// Family context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_builder() {
        let request = ElohimRequest::new(ElohimCapability::ContentSafetyReview, "user-123")
            .with_priority(RequestPriority::High)
            .with_params(RequestParams::with_content("Test content"));

        assert_eq!(request.capability, ElohimCapability::ContentSafetyReview);
        assert_eq!(request.priority, RequestPriority::High);
        assert_eq!(request.params.content, Some("Test content".to_string()));
    }

    #[test]
    fn test_params_extra() {
        let params = RequestParams::default()
            .with_extra("threshold", 0.8)
            .with_extra("max_results", 10);

        assert_eq!(params.extra["threshold"], 0.8);
        assert_eq!(params.extra["max_results"], 10);
    }
}
