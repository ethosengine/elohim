//! InferenceEngine trait — the contract all inference providers implement.
//!
//! Sprint 2 introduces real inference into the ElohimGate. This module defines
//! the async trait that backends (mock, local LLM, remote API) must satisfy,
//! plus a MockEngine for deterministic testing.

use async_trait::async_trait;
use serde_json::Value;
use thiserror::Error;

use crate::services::elohim_gate::{
    ElohimReasoning, InferenceTier, MutationType, ObservationDraft, TrustContext,
};

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Errors that can occur during inference evaluation.
#[derive(Debug, Error)]
pub enum InferenceError {
    /// The engine is not currently available (model loading, network down, etc.)
    #[error("inference engine unavailable: {0}")]
    Unavailable(String),

    /// The underlying request to the inference provider failed.
    #[error("inference request failed: {0}")]
    RequestFailed(String),

    /// The provider returned a response we could not parse.
    #[error("invalid inference response: {0}")]
    InvalidResponse(String),

    /// The session or community inference budget is exhausted.
    #[error("inference budget exhausted")]
    BudgetExhausted,
}

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

/// What the elohim recommends after evaluating a mutation.
#[derive(Debug, Clone, PartialEq)]
pub enum RecommendedAction {
    /// Mutation proceeds unmodified.
    Proceed,

    /// Mutation proceeds but reach should be adjusted (reason attached).
    ProceedWithReachAdjustment(String),

    /// Pause for human confirmation before proceeding.
    PauseForConfirmation(String),

    /// Constitutional settlement — mutation blocked with explanation.
    Settle(String),
}

/// Everything the inference engine needs to evaluate a mutation.
#[derive(Debug, Clone)]
pub struct InferenceRequest {
    /// How much ceremony this mutation should receive.
    pub tier: InferenceTier,

    /// What kind of mutation is being attempted.
    pub mutation_type: MutationType,

    /// Pre-computed trust context for the session.
    pub trust_context: TrustContext,

    /// The mutation payload (serialised to JSON Value).
    pub mutation_content: Value,

    /// Constitutional prompt text appropriate for the tier.
    pub constitutional_prompt: String,
}

/// The inference engine's evaluation of a mutation.
#[derive(Debug, Clone)]
pub struct InferenceResult {
    /// The elohim's reasoning chain.
    pub reasoning: ElohimReasoning,

    /// What action the elohim recommends.
    pub recommended_action: RecommendedAction,

    /// Observations to store in the imagodei layer.
    pub observations: Vec<ObservationDraft>,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Async trait for inference providers.
///
/// Implementors: MockEngine (testing), local LLM sidecar, remote API gateway.
#[async_trait]
pub trait InferenceEngine: Send + Sync {
    /// Evaluate a mutation and return reasoning + recommended action.
    async fn evaluate(&self, request: InferenceRequest) -> Result<InferenceResult, InferenceError>;

    /// Whether this engine is currently operational.
    fn is_available(&self) -> bool;

    /// Human-readable name for logging and diagnostics.
    fn name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// MockEngine
// ---------------------------------------------------------------------------

/// Deterministic mock engine for testing.
///
/// Returns a fixed action with confidence 1.0 and no observations.
/// Can be configured as unavailable to test fallback paths.
pub struct MockEngine {
    available: bool,
    default_action: RecommendedAction,
}

impl MockEngine {
    /// Create an engine that always returns `Proceed` with confidence 1.0.
    pub fn always_proceed() -> Self {
        Self {
            available: true,
            default_action: RecommendedAction::Proceed,
        }
    }

    /// Create an engine that reports itself as unavailable.
    pub fn unavailable() -> Self {
        Self {
            available: false,
            default_action: RecommendedAction::Proceed,
        }
    }
}

#[async_trait]
impl InferenceEngine for MockEngine {
    async fn evaluate(&self, request: InferenceRequest) -> Result<InferenceResult, InferenceError> {
        if !self.available {
            return Err(InferenceError::Unavailable(
                "MockEngine configured as unavailable".into(),
            ));
        }

        Ok(InferenceResult {
            reasoning: ElohimReasoning {
                primary_principle: "mock-principle".into(),
                interpretation: format!(
                    "Mock evaluation for {:?} at tier {:?}",
                    request.mutation_type, request.tier
                ),
                confidence: 1.0,
            },
            recommended_action: self.default_action.clone(),
            observations: vec![],
        })
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn name(&self) -> &str {
        "MockEngine"
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::elohim_gate::{TrustContext, TrustSignals};
    use serde_json::json;

    fn test_request() -> InferenceRequest {
        let ctx = TrustContext::compute(TrustSignals {
            mastery_depth: 0.5,
            steward_standing: 0.5,
            relationship_density: 0.5,
            governance_health: 1.0,
            behavioral_trust: 0.5,
            intent_divergence: 0.0,
        });
        InferenceRequest {
            tier: InferenceTier::Light,
            mutation_type: MutationType::Comment,
            trust_context: ctx,
            mutation_content: json!({"text": "hello"}),
            constitutional_prompt: "Be kind.".into(),
        }
    }

    #[tokio::test]
    async fn mock_engine_proceed() {
        let engine = MockEngine::always_proceed();
        assert!(engine.is_available());
        assert_eq!(engine.name(), "MockEngine");

        let result = engine.evaluate(test_request()).await.unwrap();
        assert_eq!(result.recommended_action, RecommendedAction::Proceed);
        assert!((result.reasoning.confidence - 1.0).abs() < f64::EPSILON);
        assert!(result.observations.is_empty());
    }

    #[tokio::test]
    async fn mock_engine_unavailable() {
        let engine = MockEngine::unavailable();
        assert!(!engine.is_available());

        let err = engine.evaluate(test_request()).await.unwrap_err();
        assert!(matches!(err, InferenceError::Unavailable(_)));
    }
}
