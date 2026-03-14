//! InferenceRouter — tries inference engines in priority order with fallback.
//!
//! The router iterates through a list of engines, skipping unavailable ones
//! and falling back on errors (including budget exhaustion). If no engine
//! can serve the request, it returns `InferenceError::Unavailable`.

use std::sync::Arc;

use super::inference_engine::{InferenceEngine, InferenceError, InferenceRequest, InferenceResult};

/// Routes inference requests to the first available engine, falling back on failure.
pub struct InferenceRouter {
    engines: Vec<Arc<dyn InferenceEngine>>,
}

impl InferenceRouter {
    pub fn new(engines: Vec<Arc<dyn InferenceEngine>>) -> Self {
        Self { engines }
    }

    /// Try each engine in priority order. Skip unavailable engines and
    /// fall back to the next on any error (including budget exhaustion).
    pub async fn route(
        &self,
        request: InferenceRequest,
    ) -> Result<InferenceResult, InferenceError> {
        for engine in &self.engines {
            if engine.is_available() {
                tracing::info!(engine = engine.name(), "Routing inference to engine");
                match engine.evaluate(request.clone()).await {
                    Ok(result) => return Ok(result),
                    Err(InferenceError::BudgetExhausted) => {
                        tracing::warn!(engine = engine.name(), "Budget exhausted, trying next");
                        continue;
                    }
                    Err(e) => {
                        tracing::warn!(
                            engine = engine.name(),
                            error = %e,
                            "Engine failed, trying next"
                        );
                        continue;
                    }
                }
            }
        }
        Err(InferenceError::Unavailable(
            "No inference engine available".to_string(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::elohim_gate::{TrustContext, TrustSignals};
    use crate::services::inference_engine::{MockEngine, RecommendedAction};
    use serde_json::json;

    fn test_request() -> InferenceRequest {
        use crate::services::elohim_gate::{InferenceTier, MutationType};

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
    async fn routes_to_first_available() {
        let router = InferenceRouter::new(vec![Arc::new(MockEngine::always_proceed())]);

        let result = router.route(test_request()).await.unwrap();
        assert_eq!(result.recommended_action, RecommendedAction::Proceed);
    }

    #[tokio::test]
    async fn skips_unavailable_engines() {
        let router = InferenceRouter::new(vec![
            Arc::new(MockEngine::unavailable()),
            Arc::new(MockEngine::always_proceed()),
        ]);

        let result = router.route(test_request()).await.unwrap();
        assert_eq!(result.recommended_action, RecommendedAction::Proceed);
    }

    #[tokio::test]
    async fn all_unavailable_returns_error() {
        let router = InferenceRouter::new(vec![Arc::new(MockEngine::unavailable())]);

        let err = router.route(test_request()).await.unwrap_err();
        assert!(matches!(err, InferenceError::Unavailable(_)));
    }
}
