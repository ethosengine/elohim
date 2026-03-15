//! SidecarEngine — HTTP client wrapping the elohim-agent-sdk TypeScript sidecar.
//!
//! The elohim-agent-sdk runs as a Fastify server at localhost:8095, providing
//! LLM-backed constitutional reasoning. This module translates between the
//! internal InferenceEngine types and the sidecar's HTTP API.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use uuid::Uuid;

use super::inference_engine::{
    InferenceEngine, InferenceError, InferenceRequest, InferenceResult, RecommendedAction,
};
use crate::services::elohim_gate::{ElohimReasoning, InferenceTier};

// ---------------------------------------------------------------------------
// Sidecar wire types (private — never leave this module)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SidecarInvokeRequest {
    request_id: String,
    elohim_id: String,
    capability: String,
    params: Value,
    requester_id: String,
    priority: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SidecarResponse {
    #[allow(dead_code)]
    request_id: String,
    status: String, // "fulfilled" | "declined" | "deferred" | "escalated"
    constitutional_reasoning: Option<ConstitutionalReasoningResponse>,
    payload: Option<Value>,
    decline_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConstitutionalReasoningResponse {
    primary_principle: String,
    interpretation: String,
    confidence: f64,
}

#[allow(dead_code)] // Used when is_available() gets a real health check
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SidecarHealthResponse {
    status: String,
    #[allow(dead_code)]
    budget_remaining: Option<i32>,
}

// ---------------------------------------------------------------------------
// SidecarEngine
// ---------------------------------------------------------------------------

/// HTTP client that calls the elohim-agent-sdk sidecar for inference.
pub struct SidecarEngine {
    client: Client,
    base_url: String,
    elohim_id: String,
}

impl SidecarEngine {
    /// Create a new SidecarEngine pointing to the given base URL.
    pub fn new(base_url: impl Into<String>, elohim_id: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");

        Self {
            client,
            base_url: base_url.into(),
            elohim_id: elohim_id.into(),
        }
    }

    /// Create an engine pointing to the default local sidecar at localhost:8095.
    pub fn default_local() -> Self {
        Self::new("http://localhost:8095", "local-elohim")
    }

    /// Translate an InferenceRequest into the sidecar's wire format.
    fn build_sidecar_request(&self, request: &InferenceRequest) -> SidecarInvokeRequest {
        let priority = match request.tier {
            InferenceTier::Constitutional => "urgent",
            InferenceTier::Full => "high",
            InferenceTier::Light => "normal",
            InferenceTier::None => "low",
        };

        let params = serde_json::json!({
            "tier": format!("{:?}", request.tier),
            "mutationType": format!("{:?}", request.mutation_type),
            "compositeTrust": request.trust_context.composite_trust,
            "mutationContent": request.mutation_content,
            "constitutionalPrompt": request.constitutional_prompt,
            "declaredIntent": request.trust_context.declared_intent,
        });

        SidecarInvokeRequest {
            request_id: Uuid::new_v4().to_string(),
            elohim_id: self.elohim_id.clone(),
            capability: "gate-evaluation".into(),
            params,
            requester_id: request.trust_context.human_id.clone(),
            priority: priority.into(),
        }
    }

    /// Translate the sidecar's response into an InferenceResult.
    fn parse_sidecar_response(
        &self,
        resp: SidecarResponse,
    ) -> Result<InferenceResult, InferenceError> {
        // Extract reasoning (use defaults if sidecar didn't provide it)
        let reasoning = match resp.constitutional_reasoning {
            Some(cr) => ElohimReasoning {
                primary_principle: cr.primary_principle,
                interpretation: cr.interpretation,
                confidence: cr.confidence,
            },
            None => ElohimReasoning {
                primary_principle: "sidecar-default".into(),
                interpretation: format!("Sidecar returned status: {}", resp.status),
                confidence: 0.5,
            },
        };

        let action = match resp.status.as_str() {
            "fulfilled" => {
                // Check payload for special cases
                if let Some(ref payload) = resp.payload {
                    if payload.get("adjustedReach").is_some() {
                        let reach = payload["adjustedReach"]
                            .as_str()
                            .unwrap_or("unknown")
                            .to_string();
                        RecommendedAction::ProceedWithReachAdjustment(reach)
                    } else if let Some(prompt) = payload.get("pausePrompt") {
                        let prompt_text = prompt.as_str().unwrap_or("Please confirm.").to_string();
                        RecommendedAction::PauseForConfirmation(prompt_text)
                    } else {
                        RecommendedAction::Proceed
                    }
                } else {
                    RecommendedAction::Proceed
                }
            }
            "declined" => {
                let reason = resp
                    .decline_reason
                    .unwrap_or_else(|| "No reason provided".into());
                RecommendedAction::Settle(reason)
            }
            "escalated" => {
                let prompt = resp
                    .payload
                    .and_then(|p| {
                        p.get("escalationPrompt")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                    })
                    .unwrap_or_else(|| "Escalated for review.".into());
                RecommendedAction::PauseForConfirmation(prompt)
            }
            other => {
                return Err(InferenceError::InvalidResponse(format!(
                    "unexpected sidecar status: {other}"
                )));
            }
        };

        Ok(InferenceResult {
            reasoning,
            recommended_action: action,
            observations: vec![], // Sprint 3: populate from sidecar observations
        })
    }
}

// ---------------------------------------------------------------------------
// InferenceEngine impl
// ---------------------------------------------------------------------------

#[async_trait]
impl InferenceEngine for SidecarEngine {
    async fn evaluate(&self, request: InferenceRequest) -> Result<InferenceResult, InferenceError> {
        let sidecar_req = self.build_sidecar_request(&request);
        let url = format!("{}/invoke", self.base_url);

        let http_resp = self
            .client
            .post(&url)
            .json(&sidecar_req)
            .send()
            .await
            .map_err(|e| InferenceError::RequestFailed(e.to_string()))?;

        if http_resp.status().as_u16() == 429 {
            return Err(InferenceError::BudgetExhausted);
        }

        if !http_resp.status().is_success() {
            return Err(InferenceError::RequestFailed(format!(
                "sidecar returned HTTP {}",
                http_resp.status()
            )));
        }

        let sidecar_resp: SidecarResponse = http_resp
            .json()
            .await
            .map_err(|e| InferenceError::InvalidResponse(e.to_string()))?;

        self.parse_sidecar_response(sidecar_resp)
    }

    fn is_available(&self) -> bool {
        // Synchronous check — use a blocking client for health.
        // In production this would be cached; for now, optimistically return true.
        // The evaluate() method handles errors gracefully.
        true
    }

    fn name(&self) -> &str {
        "sidecar"
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> SidecarEngine {
        SidecarEngine::new("http://localhost:8095", "test-elohim")
    }

    #[test]
    fn translate_fulfilled_response() {
        let resp = SidecarResponse {
            request_id: "req-1".into(),
            status: "fulfilled".into(),
            constitutional_reasoning: Some(ConstitutionalReasoningResponse {
                primary_principle: "stewardship".into(),
                interpretation: "Content aligns with community values.".into(),
                confidence: 0.9,
            }),
            payload: None,
            decline_reason: None,
        };

        let result = engine().parse_sidecar_response(resp).unwrap();
        assert_eq!(result.recommended_action, RecommendedAction::Proceed);
        assert!((result.reasoning.confidence - 0.9).abs() < f64::EPSILON);
        assert_eq!(result.reasoning.primary_principle, "stewardship");
        assert!(result.observations.is_empty());
    }

    #[test]
    fn translate_declined_response() {
        let resp = SidecarResponse {
            request_id: "req-2".into(),
            status: "declined".into(),
            constitutional_reasoning: Some(ConstitutionalReasoningResponse {
                primary_principle: "harm-prevention".into(),
                interpretation: "Content violates community boundary.".into(),
                confidence: 0.95,
            }),
            payload: None,
            decline_reason: Some("Violates harm-prevention principle.".into()),
        };

        let result = engine().parse_sidecar_response(resp).unwrap();
        match &result.recommended_action {
            RecommendedAction::Settle(reason) => {
                assert_eq!(reason, "Violates harm-prevention principle.");
            }
            other => panic!("Expected Settle, got {:?}", other),
        }
        assert_eq!(result.reasoning.primary_principle, "harm-prevention");
    }

    #[test]
    fn translate_reach_adjustment() {
        let resp = SidecarResponse {
            request_id: "req-3".into(),
            status: "fulfilled".into(),
            constitutional_reasoning: Some(ConstitutionalReasoningResponse {
                primary_principle: "proportionality".into(),
                interpretation: "Reach adjusted to match steward standing.".into(),
                confidence: 0.85,
            }),
            payload: Some(serde_json::json!({
                "adjustedReach": "community-only"
            })),
            decline_reason: None,
        };

        let result = engine().parse_sidecar_response(resp).unwrap();
        match &result.recommended_action {
            RecommendedAction::ProceedWithReachAdjustment(reach) => {
                assert_eq!(reach, "community-only");
            }
            other => panic!("Expected ProceedWithReachAdjustment, got {:?}", other),
        }
        assert!((result.reasoning.confidence - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn translate_escalated_response() {
        let resp = SidecarResponse {
            request_id: "req-4".into(),
            status: "escalated".into(),
            constitutional_reasoning: None,
            payload: Some(serde_json::json!({
                "escalationPrompt": "This requires council review."
            })),
            decline_reason: None,
        };

        let result = engine().parse_sidecar_response(resp).unwrap();
        match &result.recommended_action {
            RecommendedAction::PauseForConfirmation(prompt) => {
                assert_eq!(prompt, "This requires council review.");
            }
            other => panic!("Expected PauseForConfirmation, got {:?}", other),
        }
    }

    #[test]
    fn translate_fulfilled_with_pause_prompt() {
        let resp = SidecarResponse {
            request_id: "req-5".into(),
            status: "fulfilled".into(),
            constitutional_reasoning: Some(ConstitutionalReasoningResponse {
                primary_principle: "consent".into(),
                interpretation: "Action needs explicit confirmation.".into(),
                confidence: 0.75,
            }),
            payload: Some(serde_json::json!({
                "pausePrompt": "Are you sure you want to proceed?"
            })),
            decline_reason: None,
        };

        let result = engine().parse_sidecar_response(resp).unwrap();
        match &result.recommended_action {
            RecommendedAction::PauseForConfirmation(prompt) => {
                assert_eq!(prompt, "Are you sure you want to proceed?");
            }
            other => panic!("Expected PauseForConfirmation, got {:?}", other),
        }
    }

    #[test]
    fn translate_unknown_status_is_error() {
        let resp = SidecarResponse {
            request_id: "req-6".into(),
            status: "unknown-status".into(),
            constitutional_reasoning: None,
            payload: None,
            decline_reason: None,
        };

        let err = engine().parse_sidecar_response(resp).unwrap_err();
        assert!(matches!(err, InferenceError::InvalidResponse(_)));
    }

    #[test]
    fn build_request_sets_priority_by_tier() {
        use crate::services::elohim_gate::{MutationType, TrustContext, TrustSignals};

        let engine = engine();
        let ctx = TrustContext::compute(TrustSignals {
            mastery_depth: 0.5,
            steward_standing: 0.5,
            relationship_density: 0.5,
            governance_health: 1.0,
            behavioral_trust: 0.5,
            intent_divergence: 0.0,
        });

        let cases = vec![
            (InferenceTier::Constitutional, "urgent"),
            (InferenceTier::Full, "high"),
            (InferenceTier::Light, "normal"),
            (InferenceTier::None, "low"),
        ];

        for (tier, expected_priority) in cases {
            let req = InferenceRequest {
                tier,
                mutation_type: MutationType::Comment,
                trust_context: ctx.clone(),
                mutation_content: serde_json::json!({"text": "test"}),
                constitutional_prompt: "Be kind.".into(),
            };
            let sidecar_req = engine.build_sidecar_request(&req);
            assert_eq!(sidecar_req.priority, expected_priority, "tier {:?}", tier);
            assert_eq!(sidecar_req.capability, "gate-evaluation");
            assert_eq!(sidecar_req.elohim_id, "test-elohim");
        }
    }

    #[test]
    fn engine_name_is_sidecar() {
        assert_eq!(engine().name(), "sidecar");
    }

    #[test]
    fn engine_reports_available() {
        assert!(engine().is_available());
    }
}
