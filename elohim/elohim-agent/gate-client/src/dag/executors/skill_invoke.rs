//! SkillInvokeExecutor — dispatches to a named ElohimCapability as a sub-step.
//!
//! Phase 6: `CapabilityDispatcher` is an injection point. The
//! `MockCapabilityDispatcher` is used in DevContext / tests and returns a
//! structured Allow-shape response. Phase 7+ activation wires in the real
//! elohim-agent-service capability handler.
//!
//! The step is intentionally a sub-step: it writes its output to `output_key`
//! in GateContext and returns `StepOutcome::Continue`. The containing DAG's
//! `synthesize` step reads that key and emits the final `GateDecision`.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Map, Value};
use tracing::info;

use crate::dag::executor::{StepExecutor, StepOutcome};
use crate::dag::{SkillInvokeParams, StepType};
use crate::error::GateError;
use crate::events::RelationalImpactEvent;

use super::super::context::GateContext;

// ─── CapabilityDispatcher ────────────────────────────────────────────────────

/// Dispatch seam for `skill-invoke` steps.
///
/// Phase 6: `MockCapabilityDispatcher` is the only implementation.
/// Phase 7+ activation: wire in elohim-agent-service's capability handler.
///
/// The trait is `Send + Sync` so implementations can be held in an `Arc` and
/// shared across DAG runs without per-call cloning.
#[async_trait]
pub trait CapabilityDispatcher: Send + Sync {
    /// Invoke a named capability with the assembled request payload.
    ///
    /// - `capability`: the capability name string (no enum constraint in Phase 6;
    ///   the registry validates at Phase 7+).
    /// - `request`: the JSON object assembled from `request_from_keys` in
    ///   GateContext. May be an empty object `{}` if no keys were declared.
    ///
    /// Returns a JSON `Value` written verbatim to `output_key` in GateContext.
    async fn invoke(&self, capability: &str, request: Value) -> Result<Value, GateError>;
}

// ─── MockCapabilityDispatcher ────────────────────────────────────────────────

/// Dev-context mock dispatcher.
///
/// Returns a structured Allow-shape response:
/// ```json
/// { "status": "fulfilled", "capability": "<name>", "mock": true }
/// ```
///
/// Used in DevContext and unit tests. No network call is made.
pub struct MockCapabilityDispatcher;

#[async_trait]
impl CapabilityDispatcher for MockCapabilityDispatcher {
    async fn invoke(&self, capability: &str, _request: Value) -> Result<Value, GateError> {
        Ok(json!({
            "status": "fulfilled",
            "capability": capability,
            "mock": true
        }))
    }
}

// ─── SkillInvokeExecutor ──────────────────────────────────────────────────────

/// Executor for `StepType::SkillInvoke`.
///
/// Phase 6 behaviour:
/// - Reads `params.request_from_keys` from `GateContext` to assemble the
///   capability request payload (so the request shape is correct for Phase 7+).
/// - Invokes `dispatcher.invoke(&params.capability, request)`.
/// - Writes the response to `params.output_key` in GateContext.
/// - Returns `StepOutcome::Continue` — the containing DAG's `synthesize` step
///   produces the final `GateDecision`.
pub struct SkillInvokeExecutor {
    /// Phase 6: `MockCapabilityDispatcher` by default.
    /// Phase 7+: replace with the real capability handler via
    /// `DagRunner::with_capability_dispatcher`.
    dispatcher: Arc<dyn CapabilityDispatcher>,
}

impl SkillInvokeExecutor {
    /// Create an executor with the given dispatcher.
    pub fn new(dispatcher: Arc<dyn CapabilityDispatcher>) -> Self {
        Self { dispatcher }
    }

    fn build_request(params: &SkillInvokeParams, ctx: &GateContext) -> Value {
        let mut map = Map::new();
        for key in &params.request_from_keys {
            let value = ctx.get(key).cloned().unwrap_or(Value::Null);
            map.insert(key.clone(), value);
        }
        Value::Object(map)
    }
}

#[async_trait]
impl StepExecutor for SkillInvokeExecutor {
    async fn execute(
        &self,
        step: &StepType,
        ctx: &mut GateContext,
        _event: &RelationalImpactEvent,
    ) -> Result<StepOutcome, GateError> {
        let params = match step {
            StepType::SkillInvoke { params } => params,
            _ => {
                return Err(GateError::DagExecution(
                    "SkillInvokeExecutor received wrong step kind".to_string(),
                ))
            }
        };

        let request = Self::build_request(params, ctx);

        info!(
            skill.capability = %params.capability,
            skill.request_from_keys = ?params.request_from_keys,
            skill.output_key = %params.output_key,
            "SkillInvoke: dispatching to capability"
        );

        let response = self.dispatcher.invoke(&params.capability, request).await?;

        ctx.insert(params.output_key.clone(), response);

        Ok(StepOutcome::Continue)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::SkillInvokeParams;
    use crate::events::RelationalImpactEvent;
    use serde_json::json;

    fn event() -> RelationalImpactEvent {
        RelationalImpactEvent::ContentPublish {
            content_cid: "cid".into(),
            declared_reach: "public".into(),
            author: "agent".into(),
        }
    }

    fn skill_step(capability: &str, request_from_keys: Vec<String>, output_key: &str) -> StepType {
        StepType::SkillInvoke {
            params: SkillInvokeParams {
                capability: capability.into(),
                request_from_keys,
                output_key: output_key.into(),
            },
        }
    }

    fn mock_executor() -> SkillInvokeExecutor {
        SkillInvokeExecutor::new(Arc::new(MockCapabilityDispatcher))
    }

    // ─── MockCapabilityDispatcher returns the expected Allow-shape ────────────

    #[tokio::test]
    async fn mock_dispatcher_returns_expected_shape() {
        let dispatcher = MockCapabilityDispatcher;
        let response = dispatcher.invoke("translate", json!({})).await.unwrap();
        assert_eq!(
            response.get("status").and_then(|v| v.as_str()),
            Some("fulfilled")
        );
        assert_eq!(
            response.get("capability").and_then(|v| v.as_str()),
            Some("translate")
        );
        assert_eq!(response.get("mock").and_then(|v| v.as_bool()), Some(true));
    }

    // ─── MockCapabilityDispatcher echoes the capability name ──────────────────

    #[tokio::test]
    async fn mock_dispatcher_echoes_capability_name() {
        let dispatcher = MockCapabilityDispatcher;
        for name in [
            "translate",
            "reach-negotiation",
            "content-safety",
            "summarize",
        ] {
            let response = dispatcher.invoke(name, json!({})).await.unwrap();
            assert_eq!(
                response.get("capability").and_then(|v| v.as_str()),
                Some(name),
                "capability name not echoed for '{name}'"
            );
        }
    }

    // ─── Happy path: context keys read, response written to output_key ────────

    #[tokio::test]
    async fn happy_path_reads_keys_and_writes_output() {
        let exec = mock_executor();
        let step = skill_step(
            "translate",
            vec!["contentCid".into(), "author".into()],
            "skillOutput",
        );

        let mut ctx = GateContext::new();
        ctx.insert("contentCid", json!("cid-abc"));
        ctx.insert("author", json!("agent-xyz"));

        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();

        assert!(matches!(outcome, StepOutcome::Continue));
        let output = ctx.get("skillOutput").expect("skillOutput must be written");
        assert_eq!(
            output.get("status").and_then(|v| v.as_str()),
            Some("fulfilled")
        );
        assert_eq!(
            output.get("capability").and_then(|v| v.as_str()),
            Some("translate")
        );
    }

    // ─── Empty request_from_keys → request is empty {} (not an error) ─────────

    #[tokio::test]
    async fn empty_request_from_keys_sends_empty_object() {
        let exec = mock_executor();
        let step = skill_step("some-capability", vec![], "out");

        let mut ctx = GateContext::new();
        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();

        assert!(matches!(outcome, StepOutcome::Continue));
        assert!(
            ctx.get("out").is_some(),
            "output_key must be written even with no request keys"
        );
    }

    // ─── output_key is written with the dispatcher response ──────────────────

    #[tokio::test]
    async fn output_key_contains_dispatcher_response() {
        let exec = mock_executor();
        let step = skill_step("check-content-safety", vec![], "safetyResult");

        let mut ctx = GateContext::new();
        exec.execute(&step, &mut ctx, &event()).await.unwrap();

        let result = ctx
            .get("safetyResult")
            .expect("safetyResult must be present");
        assert_eq!(
            result.get("mock").and_then(|v| v.as_bool()),
            Some(true),
            "mock flag must be present in output"
        );
    }

    // ─── Missing context key → null in request (not an error) ────────────────

    #[tokio::test]
    async fn missing_context_key_becomes_null_in_request() {
        // Custom dispatcher that captures the request payload.
        use std::sync::Mutex;

        struct CapturingDispatcher {
            captured: Mutex<Option<Value>>,
        }

        #[async_trait::async_trait]
        impl CapabilityDispatcher for CapturingDispatcher {
            async fn invoke(&self, _capability: &str, request: Value) -> Result<Value, GateError> {
                *self.captured.lock().unwrap() = Some(request);
                Ok(json!({"status": "fulfilled", "mock": true}))
            }
        }

        let dispatcher = Arc::new(CapturingDispatcher {
            captured: Mutex::new(None),
        });
        let exec = SkillInvokeExecutor::new(dispatcher.clone());
        let step = skill_step("test-cap", vec!["missingKey".into()], "out");

        let mut ctx = GateContext::new();
        exec.execute(&step, &mut ctx, &event()).await.unwrap();

        let captured = dispatcher.captured.lock().unwrap().clone().unwrap();
        assert_eq!(
            captured.get("missingKey"),
            Some(&Value::Null),
            "missing context key must become null in request"
        );
    }

    // ─── request_from_keys builds correct request object ─────────────────────

    #[tokio::test]
    async fn request_from_keys_builds_correct_request_object() {
        use std::sync::Mutex;

        struct CapturingDispatcher {
            captured: Mutex<Option<Value>>,
        }

        #[async_trait::async_trait]
        impl CapabilityDispatcher for CapturingDispatcher {
            async fn invoke(&self, _capability: &str, request: Value) -> Result<Value, GateError> {
                *self.captured.lock().unwrap() = Some(request);
                Ok(json!({"status": "fulfilled", "mock": true}))
            }
        }

        let dispatcher = Arc::new(CapturingDispatcher {
            captured: Mutex::new(None),
        });
        let exec = SkillInvokeExecutor::new(dispatcher.clone());
        let step = skill_step("translate", vec!["lang".into(), "text".into()], "out");

        let mut ctx = GateContext::new();
        ctx.insert("lang", json!("es"));
        ctx.insert("text", json!("Hello"));

        exec.execute(&step, &mut ctx, &event()).await.unwrap();

        let captured = dispatcher.captured.lock().unwrap().clone().unwrap();
        assert_eq!(captured.get("lang"), Some(&json!("es")));
        assert_eq!(captured.get("text"), Some(&json!("Hello")));
    }

    // ─── Wrong step kind → DagExecution error ────────────────────────────────

    #[tokio::test]
    async fn wrong_step_kind_returns_error() {
        let exec = mock_executor();
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

    // ─── Named-capability parity: any String is accepted (no enum) ───────────

    #[tokio::test]
    async fn named_capability_any_string_is_accepted() {
        let exec = mock_executor();
        let capabilities = [
            "translate",
            "reach-negotiation",
            "content-safety",
            "summarize",
            "an-unknown-future-capability",
        ];
        for cap in capabilities {
            let step = skill_step(cap, vec![], "out");
            let mut ctx = GateContext::new();
            let result = exec.execute(&step, &mut ctx, &event()).await;
            assert!(
                result.is_ok(),
                "capability '{cap}' must not error, got: {:?}",
                result.err()
            );
            let output = ctx.get("out").unwrap();
            assert_eq!(
                output.get("capability").and_then(|v| v.as_str()),
                Some(cap),
                "capability '{cap}' must be echoed in output"
            );
        }
    }

    // ─── Return value is always StepOutcome::Continue ────────────────────────

    #[tokio::test]
    async fn always_returns_continue_not_terminate() {
        let exec = mock_executor();
        let step = skill_step("any-cap", vec![], "out");
        let mut ctx = GateContext::new();
        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();
        assert!(
            matches!(outcome, StepOutcome::Continue),
            "skill-invoke is a sub-step and must always return Continue"
        );
    }

    // ─── Dispatcher error propagates ─────────────────────────────────────────

    #[tokio::test]
    async fn dispatcher_error_propagates_as_gate_error() {
        struct FailingDispatcher;

        #[async_trait::async_trait]
        impl CapabilityDispatcher for FailingDispatcher {
            async fn invoke(&self, _capability: &str, _request: Value) -> Result<Value, GateError> {
                Err(GateError::DagExecution(
                    "capability dispatcher returned error".to_string(),
                ))
            }
        }

        let exec = SkillInvokeExecutor::new(Arc::new(FailingDispatcher));
        let step = skill_step("bad-cap", vec![], "out");
        let mut ctx = GateContext::new();
        let result = exec.execute(&step, &mut ctx, &event()).await;
        assert!(result.is_err(), "dispatcher error must propagate");
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("expected Err but got Ok"),
        };
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(
            msg.contains("capability dispatcher returned error"),
            "got: {msg}"
        );
    }
}
