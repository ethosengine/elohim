//! ElohimAgentService initialization and HTTP contract types.
//!
//! This module wires the `elohim-agent` crate into elohim-node by:
//! 1. Defining the HTTP request/response contract (`InvokeRequest`, `InvokeResponse`)
//! 2. Providing `ElohimNodeState` — shared state for axum handlers
//! 3. Providing `initialize_agent_service()` — async startup that configures
//!    backends, constitutional stack, and capability registration
//!
//! The actual HTTP handler wiring lives in a separate module (Task 7).

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::info;

use constitution::StackContext;
use elohim_agent::backend::MockBackend;
use elohim_agent::service::{ElohimAgentService, ServiceConfig};
use elohim_agent::ElohimCapability;

use crate::pod::admission::{AdmissionController, AdmissionConfig, RequestPriority};

// ============================================================================
// Shared state for axum handlers
// ============================================================================

/// Shared state available to HTTP handlers via axum's State extractor.
pub struct ElohimNodeState {
    /// The initialized agent service (handles invoke pipeline).
    pub agent_service: Arc<ElohimAgentService>,
    /// Admission controller (budget, queue, priority).
    pub admission: Arc<AdmissionController>,
    /// Node ID for tracing / mesh hints.
    pub node_id: String,
}

// ============================================================================
// HTTP contract types — what the invoke endpoint accepts and returns
// ============================================================================

/// Incoming invoke request (deserialized from JSON body).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvokeRequest {
    /// Client-generated request ID for correlation.
    pub request_id: String,
    /// Capability to invoke (kebab-case, e.g. "path-recommendation").
    pub capability: String,
    /// Capability-specific parameters (opaque JSON object).
    pub params: serde_json::Value,
    /// Identity of the requester.
    pub requester_id: String,
    /// Priority level for queue ordering.
    #[serde(default)]
    pub priority: InvokePriority,
}

/// Priority as seen in the HTTP request.
/// Mirrors `pod::admission::RequestPriority` but decoupled for the wire format.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InvokePriority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}

impl From<InvokePriority> for RequestPriority {
    fn from(p: InvokePriority) -> Self {
        match p {
            InvokePriority::Low => RequestPriority::Low,
            InvokePriority::Normal => RequestPriority::Normal,
            InvokePriority::High => RequestPriority::High,
            InvokePriority::Urgent => RequestPriority::Urgent,
        }
    }
}

/// Outgoing invoke response (serialized to JSON body).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvokeResponse {
    /// Echoed request ID for correlation.
    pub request_id: String,
    /// Tagged result discriminator.
    #[serde(flatten)]
    pub result: InvokeResult,
}

/// Outcome of an invoke request — three-variant tagged enum.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum InvokeResult {
    /// Request was processed and a response is available.
    Fulfilled {
        response: serde_json::Value,
    },
    /// Request cannot be served right now — try later or elsewhere.
    Deferred {
        defer_reason: String,
        mesh_hints: Vec<serde_json::Value>,
        retry_after_ms: u64,
    },
    /// Request was permanently declined.
    Declined {
        reason: String,
    },
}

// ============================================================================
// Service initialization
// ============================================================================

/// Configuration for the agent service initialization.
#[derive(Debug, Clone)]
pub struct AgentInitConfig {
    /// Node ID used as the agent ID.
    pub node_id: String,
    /// Capabilities to register.
    pub capabilities: Vec<ElohimCapability>,
    /// Budget limit for admission controller.
    pub budget_limit: u32,
    /// Max queue depth for admission controller.
    pub max_queue_depth: u32,
    /// Max concurrent requests.
    pub max_concurrent: u32,
}

impl Default for AgentInitConfig {
    fn default() -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
            capabilities: ElohimCapability::all(),
            budget_limit: 1000,
            max_queue_depth: 50,
            max_concurrent: 10,
        }
    }
}

/// Initialize the Elohim agent service and admission controller.
///
/// Training-wheels: uses MockBackend so the pipeline works end-to-end
/// without requiring a live LLM. Swap in AnthropicBackend / vLLM
/// backend via config when real inference is available.
pub async fn initialize_agent_service(
    config: AgentInitConfig,
) -> Result<ElohimNodeState, Box<dyn std::error::Error + Send + Sync>> {
    info!(node_id = %config.node_id, "Initializing Elohim agent service");

    // 1. Create backend — MockBackend for training-wheels phase
    let backend = Arc::new(
        MockBackend::default()
            .with_response(r#"{"status": "ok", "message": "Mock response from elohim-node"}"#),
    );

    // 2. Create the agent service with configured backend
    let service_config = ServiceConfig {
        agent_id: config.node_id.clone(),
        default_timeout_ms: 30_000,
        max_concurrent: config.max_concurrent as usize,
        audit_enabled: true,
    };

    let service = ElohimAgentService::new(vec![backend]).with_config(service_config);

    // 3. Initialize with constitutional stack (agent-only context for now)
    let context = StackContext::agent_only(&config.node_id);
    service.initialize(context).await.map_err(|e| {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to initialize agent service: {}", e),
        )) as Box<dyn std::error::Error + Send + Sync>
    })?;

    // 4. Register capabilities
    service.register_capabilities(config.capabilities.clone()).await;

    info!(
        capabilities = config.capabilities.len(),
        "Agent service initialized with capabilities"
    );

    // 5. Build admission controller
    let capability_strings: Vec<String> = config
        .capabilities
        .iter()
        .map(|c| serde_json::to_value(c).unwrap().as_str().unwrap().to_string())
        .collect();

    let admission_config = AdmissionConfig {
        budget_limit: config.budget_limit,
        max_queue_depth: config.max_queue_depth,
        max_concurrent: config.max_concurrent,
        capabilities: capability_strings,
        node_id: config.node_id.clone(),
    };

    let admission = Arc::new(AdmissionController::new(admission_config));

    info!(
        budget = config.budget_limit,
        queue_depth = config.max_queue_depth,
        "Admission controller initialized"
    );

    Ok(ElohimNodeState {
        agent_service: Arc::new(service),
        admission,
        node_id: config.node_id,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invoke_request_deserializes() {
        let json = r#"{
            "requestId": "req-123",
            "capability": "path-recommendation",
            "params": {"pathId": "know-thyself"},
            "requesterId": "user-1",
            "priority": "normal"
        }"#;
        let req: InvokeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.request_id, "req-123");
        assert_eq!(req.capability, "path-recommendation");
        assert_eq!(req.requester_id, "user-1");
        assert_eq!(req.priority, InvokePriority::Normal);
        assert_eq!(req.params["pathId"], "know-thyself");
    }

    #[test]
    fn test_invoke_response_fulfilled_serializes() {
        let resp = InvokeResponse {
            request_id: "req-123".into(),
            result: InvokeResult::Fulfilled {
                response: serde_json::json!({"payload": "test"}),
            },
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("fulfilled"));
        assert!(json.contains("requestId"));
        assert!(json.contains("payload"));
    }

    #[test]
    fn test_invoke_response_deferred_serializes() {
        let resp = InvokeResponse {
            request_id: "req-456".into(),
            result: InvokeResult::Deferred {
                defer_reason: "budgetExhausted".into(),
                mesh_hints: vec![],
                retry_after_ms: 10000,
            },
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("deferred"));
        assert!(json.contains("retryAfterMs"));
        assert!(json.contains("budgetExhausted"));
    }

    #[test]
    fn test_invoke_response_declined_serializes() {
        let resp = InvokeResponse {
            request_id: "req-789".into(),
            result: InvokeResult::Declined {
                reason: "Unknown capability".into(),
            },
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("declined"));
        assert!(json.contains("Unknown capability"));
    }

    #[test]
    fn test_invoke_request_defaults_priority() {
        let json = r#"{
            "requestId": "req-999",
            "capability": "spiral-detection",
            "params": {},
            "requesterId": "user-2"
        }"#;
        let req: InvokeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.priority, InvokePriority::Normal);
    }

    #[test]
    fn test_priority_conversion() {
        assert_eq!(RequestPriority::from(InvokePriority::Low), RequestPriority::Low);
        assert_eq!(RequestPriority::from(InvokePriority::Normal), RequestPriority::Normal);
        assert_eq!(RequestPriority::from(InvokePriority::High), RequestPriority::High);
        assert_eq!(RequestPriority::from(InvokePriority::Urgent), RequestPriority::Urgent);
    }

    #[tokio::test]
    async fn test_initialize_agent_service() {
        let config = AgentInitConfig {
            node_id: "test-node-123".into(),
            capabilities: vec![
                ElohimCapability::PathRecommendation,
                ElohimCapability::ContentSafetyReview,
            ],
            budget_limit: 500,
            max_queue_depth: 10,
            max_concurrent: 5,
        };

        let state = initialize_agent_service(config).await.unwrap();

        assert_eq!(state.node_id, "test-node-123");
        assert!(state.agent_service.is_initialized().await);
        assert_eq!(state.admission.budget_remaining(), 500);
        assert_eq!(state.admission.queue_depth(), 0);
    }
}
