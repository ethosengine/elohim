//! WisdomInvokeExecutor — dev-context stub for elohim LLM wisdom calls.
//!
//! Phase 2: No actual LLM call. Writes a structured mock output to the
//! `output_key` in GateContext. The downstream `SynthesizeExecutor` reads this
//! key and emits the final `GateDecision`. Logs the invocation parameters for
//! observability so the call-shape is visible before Phase 6+ wires in a real LLM.

use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::info;

use crate::dag::executor::{StepExecutor, StepOutcome};
use crate::dag::{StepType, WisdomInvokeParams};
use crate::error::GateError;
use crate::events::RelationalImpactEvent;

use super::super::context::GateContext;

// ─── WisdomInvokeExecutor ─────────────────────────────────────────────────────

/// Executor for `StepType::WisdomInvoke`.
///
/// Phase 2 behaviour:
/// - Reads `params.context_keys` from `GateContext` to assemble the wisdom
///   context (so the context shape is correct for Phase 6+ real LLM calls).
/// - Logs the invocation (constitution_cid, framing_cid, context_keys).
/// - Writes a structured mock output to `params.output_key`.
/// - Returns `StepOutcome::Continue` — the DAG continues to a `Synthesize` step
///   which produces the final `GateDecision`.
pub struct WisdomInvokeExecutor;

impl WisdomInvokeExecutor {
    pub fn new() -> Self {
        Self
    }

    fn build_mock_output(params: &WisdomInvokeParams, wisdom_context: &Value) -> Value {
        json!({
            "decision": "allow",
            "reasoning": {
                "primary_principle": "dev-context-mock",
                "confidence": 0.0,
                "summary": format!(
                    "Phase 2 dev-context mock: wisdom-invoke for constitution '{}' framing '{}' \
                     returned Allow. No real LLM call was made.",
                    params.constitution_cid, params.framing_cid
                )
            },
            "wisdomContext": wisdom_context,
            "constitutionCid": params.constitution_cid,
            "framingCid": params.framing_cid,
            "phase": "dev-context"
        })
    }
}

impl Default for WisdomInvokeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StepExecutor for WisdomInvokeExecutor {
    async fn execute(
        &self,
        step: &StepType,
        ctx: &mut GateContext,
        _event: &RelationalImpactEvent,
    ) -> Result<StepOutcome, GateError> {
        let params = match step {
            StepType::WisdomInvoke { params } => params,
            _ => {
                return Err(GateError::DagExecution(
                    "WisdomInvokeExecutor received wrong step kind".to_string(),
                ))
            }
        };

        // Assemble wisdom context from declared input keys — mirrors the shape
        // that a real LLM call will receive in Phase 6+.
        let mut wisdom_context_map = serde_json::Map::new();
        for key in &params.context_keys {
            let value = ctx.get(key).cloned().unwrap_or(Value::Null);
            wisdom_context_map.insert(key.clone(), value);
        }
        let wisdom_context = Value::Object(wisdom_context_map);

        // Log the invocation for observability (spec §6.3).
        info!(
            wisdom.constitution_cid = %params.constitution_cid,
            wisdom.framing_cid = %params.framing_cid,
            wisdom.context_keys = ?params.context_keys,
            wisdom.output_key = %params.output_key,
            "WisdomInvoke: dev-context mock invocation"
        );

        let mock_output = Self::build_mock_output(params, &wisdom_context);
        ctx.insert(params.output_key.clone(), mock_output);

        // Test builds only: if a wisdom output override was injected for this
        // (constitution_cid, output_key) pair, overwrite the mock we just wrote.
        // This is a one-shot consume — subsequent calls see the default mock again.
        #[cfg(any(test, feature = "testing"))]
        {
            if let Some(override_value) = crate::__test_take_wisdom_output_override(
                &params.constitution_cid,
                &params.output_key,
            ) {
                ctx.insert(params.output_key.clone(), override_value);
            }
        }

        Ok(StepOutcome::Continue)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::RelationalImpactEvent;
    use serde_json::json;

    fn event() -> RelationalImpactEvent {
        RelationalImpactEvent::ContentPublish {
            content_cid: "cid".into(),
            declared_reach: "public".into(),
            author: "agent".into(),
        }
    }

    fn wisdom_step(context_keys: Vec<String>, output_key: &str) -> StepType {
        StepType::WisdomInvoke {
            params: WisdomInvokeParams {
                constitution_cid: "bafyconstitution".into(),
                framing_cid: "bafyframing".into(),
                context_keys,
                output_key: output_key.into(),
            },
        }
    }

    // ─── Happy path: writes mock output and continues ─────────────────────────

    #[tokio::test]
    async fn happy_path_writes_mock_output_and_continues() {
        let exec = WisdomInvokeExecutor::new();
        let step = wisdom_step(vec![], "wisdomOutput");
        let mut ctx = GateContext::new();

        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();

        assert!(matches!(outcome, StepOutcome::Continue));
        let output = ctx
            .get("wisdomOutput")
            .expect("wisdomOutput must be written");
        assert_eq!(
            output.get("decision").and_then(|v| v.as_str()),
            Some("allow")
        );
    }

    // ─── Output shape has required fields ────────────────────────────────────

    #[tokio::test]
    async fn output_shape_has_required_fields() {
        let exec = WisdomInvokeExecutor::new();
        let step = wisdom_step(vec![], "out");
        let mut ctx = GateContext::new();

        exec.execute(&step, &mut ctx, &event()).await.unwrap();

        let out = ctx.get("out").unwrap();
        // decision field
        assert!(out.get("decision").is_some(), "must have 'decision' field");
        // reasoning sub-object
        let reasoning = out.get("reasoning").expect("must have 'reasoning' field");
        assert_eq!(
            reasoning.get("primary_principle").and_then(|v| v.as_str()),
            Some("dev-context-mock")
        );
        assert_eq!(
            reasoning.get("confidence").and_then(|v| v.as_f64()),
            Some(0.0)
        );
        assert!(
            reasoning.get("summary").is_some(),
            "must have 'summary' in reasoning"
        );
        // phase marker
        assert_eq!(
            out.get("phase").and_then(|v| v.as_str()),
            Some("dev-context")
        );
    }

    // ─── Context keys are assembled into wisdomContext ────────────────────────

    #[tokio::test]
    async fn context_keys_assembled_into_wisdom_context() {
        let exec = WisdomInvokeExecutor::new();
        let step = wisdom_step(vec!["contentCid".into(), "author".into()], "wisdomOutput");

        let mut ctx = GateContext::new();
        ctx.insert("contentCid", json!("cid-abc"));
        ctx.insert("author", json!("agent-xyz"));

        exec.execute(&step, &mut ctx, &event()).await.unwrap();

        let out = ctx.get("wisdomOutput").unwrap();
        let wctx = out.get("wisdomContext").expect("must have wisdomContext");
        assert_eq!(
            wctx.get("contentCid").and_then(|v| v.as_str()),
            Some("cid-abc")
        );
        assert_eq!(
            wctx.get("author").and_then(|v| v.as_str()),
            Some("agent-xyz")
        );
    }

    // ─── Missing context key → null in wisdomContext (not an error) ──────────

    #[tokio::test]
    async fn missing_context_key_becomes_null_in_wisdom_context() {
        let exec = WisdomInvokeExecutor::new();
        let step = wisdom_step(vec!["missingKey".into()], "out");

        let mut ctx = GateContext::new();
        exec.execute(&step, &mut ctx, &event()).await.unwrap();

        let out = ctx.get("out").unwrap();
        let wctx = out.get("wisdomContext").unwrap();
        assert_eq!(wctx.get("missingKey"), Some(&Value::Null));
    }

    // ─── Wrong step kind → error ──────────────────────────────────────────────

    #[tokio::test]
    async fn wrong_step_kind_returns_error() {
        let exec = WisdomInvokeExecutor::new();
        let wrong = StepType::ContextAssemble {
            params: crate::dag::ContextAssembleParams { pulls: vec![] },
        };
        let mut ctx = GateContext::new();
        let result = exec.execute(&wrong, &mut ctx, &event()).await;
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("expected Err but got Ok"),
        };
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(msg.contains("wrong step kind"), "got: {msg}");
    }
}
