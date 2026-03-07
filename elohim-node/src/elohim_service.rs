//! ElohimAgentService initialization, HTTP contract types, and axum handlers.
//!
//! This module wires the `elohim-agent` crate into elohim-node by:
//! 1. Defining the HTTP request/response contract (`InvokeRequest`, `InvokeResponse`)
//! 2. Providing `ElohimNodeState` — shared state for axum handlers
//! 3. Providing `initialize_agent_service()` — async startup that configures
//!    backends, constitutional stack, and capability registration
//! 4. Providing axum handlers for `POST /elohim/invoke` and `GET /elohim/health`

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use constitution::StackContext;
use elohim_agent::backend::MockBackend;
use elohim_agent::request::{ElohimRequest, RequestParams};
use elohim_agent::response::ResponseStatus;
use elohim_agent::service::{ElohimAgentService, ServiceConfig};
use elohim_agent::ElohimCapability;

use crate::pod::admission::{
    AdmissionConfig, AdmissionController, AdmissionDecision, DeferReason, RequestPriority,
};
use crate::pod::compute_rea::ComputeCommitment;

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
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum InvokeResult {
    /// Request was processed and a response is available.
    Fulfilled { response: serde_json::Value },
    /// Request cannot be served right now — try later or elsewhere.
    Deferred {
        defer_reason: String,
        mesh_hints: Vec<serde_json::Value>,
        retry_after_ms: u64,
    },
    /// Request was permanently declined.
    Declined { reason: String },
}

// ============================================================================
// Axum HTTP handlers
// ============================================================================

/// Axum handler for `POST /elohim/invoke`.
///
/// Orchestrates admission control, agent invocation, and REA accounting:
/// 1. Parse capability and priority from the request
/// 2. Evaluate admission (budget, queue, capability check)
/// 3. On Accepted: invoke agent service, record REA event, return result
/// 4. On Deferred: return defer reason with mesh hints
/// 5. On Declined: return decline reason
pub async fn handle_invoke(
    State(state): State<Arc<ElohimNodeState>>,
    Json(request): Json<InvokeRequest>,
) -> (StatusCode, Json<InvokeResponse>) {
    let request_id = request.request_id.clone();

    // 1. Evaluate admission
    let priority: RequestPriority = request.priority.into();
    let decision = state.admission.evaluate(
        &request.request_id,
        &request.requester_id,
        &request.capability,
        priority,
    );

    match decision {
        AdmissionDecision::Accepted { commitment_id, .. } => {
            // 2. Create REA commitment
            let mut commitment = ComputeCommitment::new(
                request.request_id.clone(),
                state.node_id.clone(),
                request.requester_id.clone(),
                request.capability.clone(),
                1500, // estimated tokens — training-wheels default
            );

            // 3. Parse capability string into ElohimCapability
            let capability_json = format!("\"{}\"", request.capability);
            let capability: ElohimCapability = match serde_json::from_str(&capability_json) {
                Ok(cap) => cap,
                Err(_) => {
                    commitment.cancel();
                    // Dequeue the request we just enqueued
                    state.admission.cancel(&request.request_id);
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(InvokeResponse {
                            request_id,
                            result: InvokeResult::Declined {
                                reason: format!("Unknown capability: '{}'", request.capability),
                            },
                        }),
                    );
                }
            };

            // 4. Build ElohimRequest from the HTTP request
            let mut params = RequestParams::default();
            // Map well-known fields from the opaque params JSON
            if let Some(content) = request.params.get("content").and_then(|v| v.as_str()) {
                params.content = Some(content.to_string());
            }
            if let Some(content_id) = request.params.get("contentId").and_then(|v| v.as_str()) {
                params.content_id = Some(content_id.to_string());
            }
            if let Some(path_id) = request.params.get("pathId").and_then(|v| v.as_str()) {
                params.path_id = Some(path_id.to_string());
            }
            if let Some(query) = request.params.get("query").and_then(|v| v.as_str()) {
                params.query = Some(query.to_string());
            }
            // Pass the full params object as extra for anything else
            params.extra = request.params.clone();

            let agent_request =
                ElohimRequest::new(capability, &request.requester_id).with_params(params);

            // 5. Mark active and invoke
            state.admission.mark_active();
            let invoke_result = state.agent_service.invoke(agent_request).await;
            state.admission.mark_complete();

            // Dequeue the processed request
            state.admission.dequeue();

            match invoke_result {
                Ok(response) => {
                    // 6. Determine outcome from ElohimResponse status
                    match response.status {
                        ResponseStatus::Fulfilled => {
                            let tokens_used =
                                response.cost.input_tokens + response.cost.output_tokens;
                            let time_ms = response.cost.processing_time_ms;

                            // Fulfill the REA commitment
                            let _event = commitment.fulfill(
                                tokens_used,
                                "mock".to_string(), // training-wheels model name
                                time_ms,
                            );

                            // Record budget usage
                            state.admission.record_usage(tokens_used);

                            info!(
                                request_id = %request_id,
                                commitment_id = %commitment_id,
                                tokens = tokens_used,
                                "Invoke fulfilled"
                            );

                            // Serialize the response payload as the response value
                            let response_value =
                                serde_json::to_value(&response.payload).unwrap_or_default();

                            (
                                StatusCode::OK,
                                Json(InvokeResponse {
                                    request_id,
                                    result: InvokeResult::Fulfilled {
                                        response: response_value,
                                    },
                                }),
                            )
                        }
                        ResponseStatus::Declined => {
                            commitment.cancel();
                            let reason = response.constitutional_reasoning.interpretation.clone();

                            warn!(
                                request_id = %request_id,
                                reason = %reason,
                                "Invoke declined by agent"
                            );

                            (
                                StatusCode::OK,
                                Json(InvokeResponse {
                                    request_id,
                                    result: InvokeResult::Declined { reason },
                                }),
                            )
                        }
                        _ => {
                            // Escalated or Deferred at agent level — treat as fulfilled
                            // with the full response serialized
                            let response_value =
                                serde_json::to_value(&response.payload).unwrap_or_default();

                            (
                                StatusCode::OK,
                                Json(InvokeResponse {
                                    request_id,
                                    result: InvokeResult::Fulfilled {
                                        response: response_value,
                                    },
                                }),
                            )
                        }
                    }
                }
                Err(e) => {
                    commitment.cancel();

                    error!(
                        request_id = %request_id,
                        error = %e,
                        "Invoke failed"
                    );

                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(InvokeResponse {
                            request_id,
                            result: InvokeResult::Declined {
                                reason: format!("Internal error: {}", e),
                            },
                        }),
                    )
                }
            }
        }

        AdmissionDecision::Deferred {
            reason,
            mesh_hints,
            retry_after_ms,
        } => {
            let defer_reason = match reason {
                DeferReason::BudgetExhausted => "budgetExhausted".to_string(),
                DeferReason::QueueFull => "queueFull".to_string(),
                DeferReason::CapabilityUnavailable => "capabilityUnavailable".to_string(),
                DeferReason::SystemPressure => "systemPressure".to_string(),
            };

            let hints: Vec<serde_json::Value> = mesh_hints
                .iter()
                .map(|h| serde_json::to_value(h).unwrap_or_default())
                .collect();

            info!(
                request_id = %request_id,
                reason = %defer_reason,
                retry_after_ms,
                "Invoke deferred"
            );

            // Return 200 with status:"deferred" in body — the JSON status field
            // discriminates all outcomes. Using 503 would cause NativeBackend
            // (which checks response.ok) to treat deferred as an error.
            (
                StatusCode::OK,
                Json(InvokeResponse {
                    request_id,
                    result: InvokeResult::Deferred {
                        defer_reason,
                        mesh_hints: hints,
                        retry_after_ms,
                    },
                }),
            )
        }

        AdmissionDecision::Declined { reason } => {
            warn!(
                request_id = %request_id,
                reason = %reason,
                "Invoke declined by admission"
            );

            // Return 200 with status:"declined" — same principle as deferred:
            // the JSON status field discriminates, not the HTTP status code.
            (
                StatusCode::OK,
                Json(InvokeResponse {
                    request_id,
                    result: InvokeResult::Declined { reason },
                }),
            )
        }
    }
}

/// Axum handler for `GET /elohim/health`.
///
/// Returns node health including admission controller metrics.
pub async fn handle_health(State(state): State<Arc<ElohimNodeState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "nodeId": state.node_id,
        "budgetRemaining": state.admission.budget_remaining(),
        "activeRequests": state.admission.active_requests(),
        "queueDepth": state.admission.queue_depth(),
    }))
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
        Box::new(std::io::Error::other(format!(
            "Failed to initialize agent service: {}",
            e
        ))) as Box<dyn std::error::Error + Send + Sync>
    })?;

    // 4. Register capabilities
    service
        .register_capabilities(config.capabilities.clone())
        .await;

    info!(
        capabilities = config.capabilities.len(),
        "Agent service initialized with capabilities"
    );

    // 5. Build admission controller
    let capability_strings: Vec<String> = config
        .capabilities
        .iter()
        .map(|c| {
            serde_json::to_value(c)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        })
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
        assert_eq!(
            RequestPriority::from(InvokePriority::Low),
            RequestPriority::Low
        );
        assert_eq!(
            RequestPriority::from(InvokePriority::Normal),
            RequestPriority::Normal
        );
        assert_eq!(
            RequestPriority::from(InvokePriority::High),
            RequestPriority::High
        );
        assert_eq!(
            RequestPriority::from(InvokePriority::Urgent),
            RequestPriority::Urgent
        );
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
