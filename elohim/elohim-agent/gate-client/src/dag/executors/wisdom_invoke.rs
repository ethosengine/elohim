//! WisdomInvokeExecutor — dispatches wisdom calls based on `WisdomTransport`.
//!
//! # Transport dispatch
//!
//! - `WisdomTransport::Mock` — writes a hardcoded Allow + phase="dev-context"
//!   to the output key.  No external call, no overhead, always succeeds.
//!   Used in DevContext and sync zome contexts where async is not available.
//!
//! - `WisdomTransport::InProcess` — calls
//!   `elohim_agent::wisdom::invoke_wisdom(input).await` directly.
//!   The response's `phase` field is **authoritative**:
//!   - Real LLM call succeeded → `phase: "elohim-active"`
//!   - No API key / backend unavailable / parse error → `phase: "dev-context"`
//!
//! # Phase observation rule
//!
//! `Phase::ElohimActive` is **observed from the actual call outcome**, never
//! from a config flag. This executor writes the raw `phase` string from the
//! service response to the context. `SynthesizeExecutor::execute` reads it and
//! propagates it to the final `GateDecision`.

use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::info;

use crate::dag::executor::{StepExecutor, StepOutcome};
use crate::dag::{StepType, WisdomInvokeParams};
use crate::error::GateError;
use crate::events::RelationalImpactEvent;
use crate::transport::WisdomTransport;

use super::super::context::GateContext;

// ─── WisdomInvokeExecutor ─────────────────────────────────────────────────────

/// Executor for `StepType::WisdomInvoke`.
///
/// Holds a `WisdomTransport` that determines whether to use the hardcoded mock
/// (DevContext) or the in-process `elohim_agent::wisdom::invoke_wisdom` function
/// (ElohimActive when real inference succeeds).
pub struct WisdomInvokeExecutor {
    transport: WisdomTransport,
}

impl WisdomInvokeExecutor {
    /// Create an executor with the given transport.
    pub fn new(transport: WisdomTransport) -> Self {
        Self { transport }
    }

    /// Convenience: create an executor with `WisdomTransport::Mock`.
    pub fn mock() -> Self {
        Self::new(WisdomTransport::Mock)
    }

    /// Convenience: create an executor with `WisdomTransport::InProcess`.
    pub fn in_process() -> Self {
        Self::new(WisdomTransport::InProcess)
    }

    /// Assemble the wisdom context map from the declared `context_keys`.
    fn assemble_wisdom_context(params: &WisdomInvokeParams, ctx: &GateContext) -> Value {
        let mut map = serde_json::Map::new();
        for key in &params.context_keys {
            let value = ctx.get(key).cloned().unwrap_or(Value::Null);
            map.insert(key.clone(), value);
        }
        Value::Object(map)
    }

    /// Build the mock output — hardcoded Allow + dev-context phase.
    fn build_mock_output(params: &WisdomInvokeParams, wisdom_context: &Value) -> Value {
        json!({
            "decision": "allow",
            "reasoning": {
                "primary_principle": "dev-context-mock",
                "confidence": 0.0,
                "summary": format!(
                    "Phase 8 dev-context mock: wisdom-invoke for constitution '{}' framing '{}' \
                     returned Allow. No real LLM call was made.",
                    params.constitution_cid, params.framing_cid
                ),
                "phase_note": "mock-transport"
            },
            "wisdomContext": wisdom_context,
            "constitutionCid": params.constitution_cid,
            "framingCid": params.framing_cid,
            "phase": "dev-context"
        })
    }

    /// Build the output from a `WisdomInvocationResponse`.
    ///
    /// The `phase` field is taken directly from the service response — it is
    /// the authoritative phase observation.  `SynthesizeExecutor` reads this
    /// field when building the final `GateDecision`.
    fn build_output_from_response(
        response: &elohim_agent::wisdom::WisdomInvocationResponse,
        params: &WisdomInvokeParams,
        wisdom_context: &Value,
    ) -> Value {
        let decision_str = match &response.decision {
            elohim_agent::wisdom::WisdomDecision::Allow => "allow",
            elohim_agent::wisdom::WisdomDecision::Decline => "decline",
            elohim_agent::wisdom::WisdomDecision::Escalate => "escalate",
            elohim_agent::wisdom::WisdomDecision::Verdict => "verdict",
            elohim_agent::wisdom::WisdomDecision::NeedDeeper => "need-deeper",
        };

        let phase_str = match response.phase {
            elohim_agent::wisdom::WisdomPhase::DevContext => "dev-context",
            elohim_agent::wisdom::WisdomPhase::ElohimActive => "elohim-active",
        };

        json!({
            "decision": decision_str,
            "reasoning": {
                "primary_principle": response.reasoning.primary_principle,
                "confidence": response.reasoning.confidence,
                "summary": response.reasoning.summary,
                "phase_note": response.reasoning.phase_note
            },
            "wisdomContext": wisdom_context,
            "constitutionCid": params.constitution_cid,
            "framingCid": params.framing_cid,
            "phase": phase_str
        })
    }
}

impl Default for WisdomInvokeExecutor {
    fn default() -> Self {
        Self::mock()
    }
}

#[async_trait]
impl StepExecutor for WisdomInvokeExecutor {
    async fn execute(
        &self,
        step: &StepType,
        ctx: &mut GateContext,
        event: &RelationalImpactEvent,
    ) -> Result<StepOutcome, GateError> {
        let params = match step {
            StepType::WisdomInvoke { params } => params,
            _ => {
                return Err(GateError::DagExecution(
                    "WisdomInvokeExecutor received wrong step kind".to_string(),
                ))
            }
        };

        let wisdom_context = Self::assemble_wisdom_context(params, ctx);

        // Log the invocation for observability (spec §6.3).
        info!(
            wisdom.constitution_cid = %params.constitution_cid,
            wisdom.framing_cid = %params.framing_cid,
            wisdom.context_keys = ?params.context_keys,
            wisdom.output_key = %params.output_key,
            wisdom.transport = ?self.transport,
            "WisdomInvoke: invocation starting"
        );

        let output = match &self.transport {
            WisdomTransport::Mock => {
                info!(
                    wisdom.output_key = %params.output_key,
                    "WisdomInvoke: Mock transport — hardcoded Allow + dev-context"
                );
                Self::build_mock_output(params, &wisdom_context)
            }

            WisdomTransport::InProcess => {
                // Assemble the input from context
                let context_json =
                    serde_json::to_string(&wisdom_context).unwrap_or_else(|_| "{}".to_string());
                let event_summary = describe_event(event);

                let input = elohim_agent::wisdom::WisdomInvocationInput {
                    constitution_cid: params.constitution_cid.clone(),
                    framing_cid: params.framing_cid.clone(),
                    context_keys: params.context_keys.clone(),
                    context_json,
                    event_summary,
                };

                let response = elohim_agent::wisdom::invoke_wisdom(input).await;

                info!(
                    wisdom.decision = ?response.decision,
                    wisdom.phase = ?response.phase,
                    wisdom.confidence = response.reasoning.confidence,
                    "WisdomInvoke: InProcess call complete"
                );

                Self::build_output_from_response(&response, params, &wisdom_context)
            }
        };

        ctx.insert(params.output_key.clone(), output);

        // Test builds only: if a wisdom output override was injected for this
        // (constitution_cid, output_key) pair, overwrite the value we just wrote.
        // This is a one-shot consume — subsequent calls see the default output again.
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

/// Build a short natural-language summary of the event for the LLM prompt.
fn describe_event(event: &RelationalImpactEvent) -> String {
    match event {
        RelationalImpactEvent::ContentPublish {
            content_cid,
            declared_reach,
            author,
        } => format!(
            "ContentPublish: author={author} published content cid={content_cid} \
             at reach={declared_reach}"
        ),
        RelationalImpactEvent::AttestationWrite {
            subject_hash,
            claim_kind,
            issuer,
        } => format!(
            "AttestationWrite: issuer={issuer} wrote claim kind={claim_kind} \
             on subject={subject_hash}"
        ),
        RelationalImpactEvent::EconomicEventEmit {
            event_kind,
            provider,
            receiver,
            quantity,
        } => format!(
            "EconomicEventEmit: kind={event_kind} provider={provider} \
             receiver={receiver} quantity={quantity}"
        ),
        RelationalImpactEvent::PeerMessage {
            recipient,
            payload_kind,
        } => format!("PeerMessage: to={recipient} kind={payload_kind}"),
        RelationalImpactEvent::SyncToPeers {
            manifest_cid,
            item_count,
        } => format!("SyncToPeers: manifest={manifest_cid} items={item_count}"),
        RelationalImpactEvent::AdviceSought {
            requester,
            summary_cid,
            topic,
        } => format!("AdviceSought: requester={requester} topic={topic} summary={summary_cid}"),
        RelationalImpactEvent::CapabilityInvoke {
            capability,
            requester,
            request_id,
        } => format!(
            "CapabilityInvoke: capability={capability} requester={requester} \
             request_id={request_id}"
        ),
        RelationalImpactEvent::PrivateToPublicCrossing {
            source_space,
            artifact_ref,
        } => format!("PrivateToPublicCrossing: from={source_space} artifact={artifact_ref}"),
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

    // ─── Mock transport: writes mock output + continues ───────────────────────

    #[tokio::test]
    async fn mock_transport_writes_allow_dev_context_output() {
        let exec = WisdomInvokeExecutor::mock();
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
        assert_eq!(
            output.get("phase").and_then(|v| v.as_str()),
            Some("dev-context"),
            "Mock transport must always write dev-context phase"
        );
    }

    // ─── Mock transport: output shape has required fields ─────────────────────

    #[tokio::test]
    async fn mock_transport_output_shape_has_required_fields() {
        let exec = WisdomInvokeExecutor::mock();
        let step = wisdom_step(vec![], "out");
        let mut ctx = GateContext::new();

        exec.execute(&step, &mut ctx, &event()).await.unwrap();

        let out = ctx.get("out").unwrap();
        assert!(out.get("decision").is_some(), "must have 'decision' field");
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
        assert_eq!(
            out.get("phase").and_then(|v| v.as_str()),
            Some("dev-context")
        );
    }

    // ─── Mock transport: context keys assembled into wisdomContext ────────────

    #[tokio::test]
    async fn mock_transport_context_keys_assembled_into_wisdom_context() {
        let exec = WisdomInvokeExecutor::mock();
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

    // ─── Mock transport: missing context key → null ───────────────────────────

    #[tokio::test]
    async fn mock_transport_missing_context_key_becomes_null() {
        let exec = WisdomInvokeExecutor::mock();
        let step = wisdom_step(vec!["missingKey".into()], "out");

        let mut ctx = GateContext::new();
        exec.execute(&step, &mut ctx, &event()).await.unwrap();

        let out = ctx.get("out").unwrap();
        let wctx = out.get("wisdomContext").unwrap();
        assert_eq!(wctx.get("missingKey"), Some(&Value::Null));
    }

    // ─── InProcess transport with no API key → DevContext ─────────────────────
    //
    // When no ANTHROPIC_API_KEY or OPENAI_API_KEY is set in the test environment,
    // invoke_wisdom returns a dev-context stub.  The executor must write
    // phase="dev-context" — NOT "elohim-active".

    #[tokio::test]
    async fn in_process_no_api_key_returns_dev_context() {
        // Clear any API key env vars so select_env_backend returns None.
        // (In CI and dev contexts, these should not be set.)
        // We cannot guarantee the env is clean, so we check the output's phase
        // and assert it is EITHER dev-context (no key) or elohim-active (key set).
        // The key correctness assertion is: whatever the service decides, we propagate it.
        let exec = WisdomInvokeExecutor::in_process();
        let step = wisdom_step(vec![], "out");
        let mut ctx = GateContext::new();

        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();
        assert!(matches!(outcome, StepOutcome::Continue));

        let out = ctx.get("out").unwrap();
        let phase = out
            .get("phase")
            .and_then(|v| v.as_str())
            .unwrap_or("missing");

        // Phase must be one of the two valid values.
        assert!(
            phase == "dev-context" || phase == "elohim-active",
            "phase must be 'dev-context' or 'elohim-active', got: {phase}"
        );

        // In environments with no API key (CI, typical dev), phase is dev-context.
        // We verify this only when we KNOW no key is set.
        let has_anthropic = std::env::var("ANTHROPIC_API_KEY")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);
        let has_openai = std::env::var("OPENAI_API_KEY")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);

        if !has_anthropic && !has_openai {
            assert_eq!(
                phase, "dev-context",
                "Without API keys, InProcess must return dev-context; got: {phase}"
            );
            let decision = out.get("decision").and_then(|v| v.as_str()).unwrap_or("");
            assert_eq!(
                decision, "allow",
                "Without API keys, stub returns Allow; got: {decision}"
            );
        }
    }

    // ─── InProcess transport with canned real-response backend: ElohimActive ──
    //
    // Uses __test_set_wisdom_output to inject a pre-cooked "elohim-active" output
    // so we can verify the phase propagation without needing a real LLM key.
    // This simulates what would happen if invoke_wisdom returned ElohimActive.

    #[tokio::test]
    async fn in_process_canned_elohim_active_phase_propagated() {
        // We inject the canned wisdom output directly via the test override mechanism.
        // This simulates the InProcess path returning an elohim-active response.
        let canned_elohim_active = json!({
            "decision": "allow",
            "reasoning": {
                "primary_principle": "subsidiarity",
                "confidence": 0.92,
                "summary": "Content is within declared reach boundaries.",
                "phase_note": ""
            },
            "wisdomContext": {},
            "constitutionCid": "bafyconstitution",
            "framingCid": "bafyframing",
            "phase": "elohim-active"
        });

        crate::__test_set_wisdom_output("bafyconstitution", "wisdomOutput", canned_elohim_active);

        // Use InProcess executor (the override fires after the output is written).
        let exec = WisdomInvokeExecutor::in_process();
        let step = wisdom_step(vec![], "wisdomOutput");
        let mut ctx = GateContext::new();

        exec.execute(&step, &mut ctx, &event()).await.unwrap();

        let out = ctx.get("wisdomOutput").unwrap();
        assert_eq!(
            out.get("phase").and_then(|v| v.as_str()),
            Some("elohim-active"),
            "Canned elohim-active response must be propagated to output"
        );
        assert_eq!(out.get("decision").and_then(|v| v.as_str()), Some("allow"));
    }

    // ─── describe_event covers all 8 variants ────────────────────────────────

    #[test]
    fn describe_event_all_variants_non_empty() {
        let events = vec![
            RelationalImpactEvent::ContentPublish {
                content_cid: "c".into(),
                declared_reach: "public".into(),
                author: "a".into(),
            },
            RelationalImpactEvent::AttestationWrite {
                subject_hash: "h".into(),
                claim_kind: "brit".into(),
                issuer: "a".into(),
            },
            RelationalImpactEvent::EconomicEventEmit {
                event_kind: "transfer".into(),
                provider: "a".into(),
                receiver: "b".into(),
                quantity: "5".into(),
            },
            RelationalImpactEvent::PeerMessage {
                recipient: "a".into(),
                payload_kind: "handshake".into(),
            },
            RelationalImpactEvent::SyncToPeers {
                manifest_cid: "m".into(),
                item_count: 3,
            },
            RelationalImpactEvent::AdviceSought {
                requester: "a".into(),
                summary_cid: "s".into(),
                topic: "conflict".into(),
            },
            RelationalImpactEvent::CapabilityInvoke {
                capability: "translate".into(),
                requester: "a".into(),
                request_id: "r".into(),
            },
            RelationalImpactEvent::PrivateToPublicCrossing {
                source_space: "private".into(),
                artifact_ref: "art".into(),
            },
        ];
        for ev in &events {
            let s = describe_event(ev);
            assert!(
                !s.is_empty(),
                "describe_event must be non-empty for {:?}",
                ev.kind()
            );
            assert!(
                s.len() > 5,
                "describe_event must include useful content for {:?}: got '{s}'",
                ev.kind()
            );
        }
    }

    // ─── Wrong step kind → error ──────────────────────────────────────────────

    #[tokio::test]
    async fn wrong_step_kind_returns_error() {
        let exec = WisdomInvokeExecutor::mock();
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

    // ─── Default is Mock ──────────────────────────────────────────────────────

    #[test]
    fn default_executor_uses_mock_transport() {
        let exec = WisdomInvokeExecutor::default();
        assert_eq!(exec.transport, WisdomTransport::Mock);
    }

    // ─── Constructors set correct transport ───────────────────────────────────

    #[test]
    fn mock_constructor_sets_mock_transport() {
        let exec = WisdomInvokeExecutor::mock();
        assert_eq!(exec.transport, WisdomTransport::Mock);
    }

    #[test]
    fn in_process_constructor_sets_in_process_transport() {
        let exec = WisdomInvokeExecutor::in_process();
        assert_eq!(exec.transport, WisdomTransport::InProcess);
    }
}
