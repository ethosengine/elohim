//! SynthesizeExecutor — composes prior step outputs into a final GateDecision.
//!
//! Reads `params.input_keys` from GateContext, applies the named
//! `decision_builder`, and returns `StepOutcome::Terminate` with the composed
//! `GateDecision` + converted side effects.

use async_trait::async_trait;
use tracing::{debug, warn};

use crate::dag::executor::{StepExecutor, StepOutcome};
use crate::dag::{StepType, SynthesizeParams};
use crate::error::GateError;
use crate::events::RelationalImpactEvent;
use crate::phase::Phase;
use crate::types::{
    ConstitutionalReasoningSummary, DeclineGrounds, GateDecision, GateStatus, GateTag,
};

use super::super::context::GateContext;
use super::convert_side_effect_specs;

// ─── SynthesizeExecutor ───────────────────────────────────────────────────────

/// Executor for `StepType::Synthesize`.
///
/// Applies a `decision_builder` string selector to produce a `GateStatus`,
/// converts `side_effects` specs to concrete `SideEffect`s, and terminates
/// the DAG with `StepOutcome::Terminate`.
///
/// Supported `decision_builder` values:
/// - `"allow-passthrough"` — always Allow { exempt: false } with mocked reasoning.
/// - `"allow-with-wisdom"` — Allow with reasoning pulled from the wisdom output
///   (reads `"wisdomOutput"` key by default).
/// - `"decline-from-wisdom"` — Decline if wisdom output's `"decision"` == `"decline"`;
///   otherwise falls through to Allow.
/// - `"verdict-from-rule"` — reads `"ruleDecision"` and emits a Verdict.
/// - `"verdict-from-reach"` — reads `"reachData.level"` and emits
///   `Verdict(ReachLevel { level })`. Defaults to `"self-reach"` when level is null.
pub struct SynthesizeExecutor;

impl SynthesizeExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SynthesizeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StepExecutor for SynthesizeExecutor {
    async fn execute(
        &self,
        step: &StepType,
        ctx: &mut GateContext,
        _event: &RelationalImpactEvent,
    ) -> Result<StepOutcome, GateError> {
        let params = match step {
            StepType::Synthesize { params } => params,
            _ => {
                return Err(GateError::DagExecution(
                    "SynthesizeExecutor received wrong step kind".to_string(),
                ))
            }
        };

        debug!(
            synthesize.builder = %params.decision_builder,
            synthesize.input_keys = ?params.input_keys,
            "SynthesizeExecutor: building decision"
        );

        let (status, reasoning_override) = build_status(params, ctx)?;
        let side_effects = convert_side_effect_specs(&params.side_effects, ctx)?;

        let decision = GateDecision {
            status,
            reasoning: reasoning_override.unwrap_or_else(ConstitutionalReasoningSummary::mocked),
            side_effects,
            decision_attestation_cid: None,
            phase: Phase::DevContext,
        };

        Ok(StepOutcome::Terminate(decision, vec![]))
    }
}

/// Apply the `decision_builder` selector and return the appropriate `GateStatus`
/// plus an optional `ConstitutionalReasoningSummary` override.
///
/// When the override is `None`, the caller uses `ConstitutionalReasoningSummary::mocked()`.
fn build_status(
    params: &SynthesizeParams,
    ctx: &GateContext,
) -> Result<(GateStatus, Option<ConstitutionalReasoningSummary>), GateError> {
    match params.decision_builder.as_str() {
        "allow-passthrough" => Ok((GateStatus::Allow { exempt: false }, None)),

        "allow-with-wisdom" => {
            // Read wisdom output — default key is "wisdomOutput".
            let wisdom_key = params
                .input_keys
                .first()
                .map(String::as_str)
                .unwrap_or("wisdomOutput");
            let wisdom = ctx.get(wisdom_key).ok_or_else(|| {
                GateError::DagExecution(format!(
                    "allow-with-wisdom builder: missing required context key '{wisdom_key}'"
                ))
            })?;

            // Build ConstitutionalReasoningSummary from wisdom reasoning sub-object.
            // If the reasoning sub-shape is absent, fall back to mocked() with a warning.
            let reasoning_val = wisdom.get("reasoning");
            if reasoning_val.is_none() {
                warn!(
                    wisdom_key,
                    "allow-with-wisdom: wisdom output missing 'reasoning' sub-object; \
                     falling back to mocked ConstitutionalReasoningSummary"
                );
            }
            let reasoning_override = reasoning_val.map(|r| ConstitutionalReasoningSummary {
                primary_principle: r
                    .get("primary_principle")
                    .and_then(|v| v.as_str())
                    .unwrap_or("dev-context-mock")
                    .to_string(),
                confidence: r
                    .get("confidence")
                    .and_then(|v| v.as_f64())
                    .map(|f| f as f32)
                    .unwrap_or(0.0),
                summary: r
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("wisdom-sourced")
                    .to_string(),
                phase_note: "wisdom-sourced".to_string(),
            });

            Ok((GateStatus::Allow { exempt: false }, reasoning_override))
        }

        "decline-from-wisdom" => {
            let wisdom_key = params
                .input_keys
                .first()
                .map(String::as_str)
                .unwrap_or("wisdomOutput");
            let wisdom = ctx.get(wisdom_key);

            // If wisdom output says "decline", emit Decline; otherwise Allow.
            let is_decline = wisdom
                .and_then(|v| v.get("decision"))
                .and_then(|v| v.as_str())
                .map(|s| s == "decline")
                .unwrap_or(false);

            if is_decline {
                let summary = wisdom
                    .and_then(|v| v.get("reasoning"))
                    .and_then(|v| v.get("summary"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("declined by wisdom")
                    .to_string();

                Ok((
                    GateStatus::Decline {
                        grounds: DeclineGrounds {
                            category: "wisdom-decline".to_string(),
                            summary,
                            principle_refs: vec![],
                        },
                    },
                    None,
                ))
            } else {
                Ok((GateStatus::Allow { exempt: false }, None))
            }
        }

        "verdict-from-rule" => {
            let rule_key = params
                .input_keys
                .first()
                .map(String::as_str)
                .unwrap_or("ruleDecision");
            let rule = ctx.get(rule_key).ok_or_else(|| {
                GateError::DagExecution(format!(
                    "synthesize step 'verdict-from-rule' missing required context key '{rule_key}'"
                ))
            })?;

            // The rule decision JSON must deserialize into a GateTag.
            let tag: GateTag = serde_json::from_value(rule.clone()).map_err(|e| {
                GateError::DagExecution(format!(
                    "synthesize step 'verdict-from-rule': could not parse ruleDecision as GateTag: {e}"
                ))
            })?;
            Ok((GateStatus::Verdict(tag), None))
        }

        "verdict-from-reach" => {
            // Read the aggregation output key — defaults to "reachData".
            let reach_key = params
                .input_keys
                .first()
                .map(String::as_str)
                .unwrap_or("reachData");
            let reach_data = ctx.get(reach_key).ok_or_else(|| {
                GateError::DagExecution(format!(
                    "synthesize step 'verdict-from-reach' missing required context key '{reach_key}'"
                ))
            })?;

            // Extract the `level` string from the aggregation output object.
            // When no tier was met, `level` is null — default to "self-reach".
            let level = reach_data
                .get("level")
                .and_then(|v| v.as_str())
                .unwrap_or("self-reach")
                .to_string();

            Ok((GateStatus::Verdict(GateTag::ReachLevel { level }), None))
        }

        unknown => Err(GateError::DagExecution(format!(
            "synthesize step: unknown decision_builder '{unknown}'"
        ))),
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::{SideEffectSpec, SynthesizeParams};
    use crate::events::RelationalImpactEvent;
    use crate::types::GateStatus;
    use serde_json::json;

    fn event() -> RelationalImpactEvent {
        RelationalImpactEvent::ContentPublish {
            content_cid: "cid".into(),
            declared_reach: "public".into(),
            author: "agent".into(),
        }
    }

    fn synthesize_step(builder: &str, input_keys: Vec<String>) -> StepType {
        StepType::Synthesize {
            params: SynthesizeParams {
                input_keys,
                decision_builder: builder.into(),
                side_effects: vec![],
            },
        }
    }

    // ─── allow-passthrough → Allow ────────────────────────────────────────────

    #[tokio::test]
    async fn allow_passthrough_returns_allow() {
        let exec = SynthesizeExecutor::new();
        let step = synthesize_step("allow-passthrough", vec![]);
        let mut ctx = GateContext::new();

        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();
        let (decision, _) = match outcome {
            StepOutcome::Terminate(d, se) => (d, se),
            _ => panic!("expected Terminate"),
        };
        assert!(matches!(
            decision.status,
            GateStatus::Allow { exempt: false }
        ));
        assert!(decision.side_effects.is_empty());
    }

    // ─── allow-with-wisdom → Allow with propagated reasoning ─────────────────

    #[tokio::test]
    async fn allow_with_wisdom_returns_allow() {
        let exec = SynthesizeExecutor::new();
        let step = synthesize_step("allow-with-wisdom", vec!["wisdomOutput".into()]);

        let mut ctx = GateContext::new();
        ctx.insert("wisdomOutput", json!({
            "decision": "allow",
            "reasoning": {"primary_principle": "dev-context-mock", "confidence": 0.0, "summary": "mock"}
        }));

        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();
        let (decision, _) = match outcome {
            StepOutcome::Terminate(d, se) => (d, se),
            _ => panic!("expected Terminate"),
        };
        assert!(decision.is_allowed());
    }

    // ─── allow-with-wisdom propagates wisdom reasoning.summary ───────────────

    #[tokio::test]
    async fn allow_with_wisdom_propagates_reasoning_summary() {
        let exec = SynthesizeExecutor::new();
        let step = synthesize_step("allow-with-wisdom", vec!["wisdomOutput".into()]);

        let mut ctx = GateContext::new();
        ctx.insert(
            "wisdomOutput",
            json!({
                "decision": "allow",
                "reasoning": {
                    "primary_principle": "test-principle",
                    "confidence": 0.9,
                    "summary": "unique-test-marker"
                }
            }),
        );

        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();
        let (decision, _) = match outcome {
            StepOutcome::Terminate(d, se) => (d, se),
            _ => panic!("expected Terminate"),
        };
        assert!(decision.is_allowed());
        assert_eq!(
            decision.reasoning.summary, "unique-test-marker",
            "reasoning.summary should be propagated from wisdom output, got: {:?}",
            decision.reasoning.summary
        );
        assert_eq!(decision.reasoning.phase_note, "wisdom-sourced");
        assert_eq!(decision.reasoning.primary_principle, "test-principle");
    }

    // ─── allow-with-wisdom missing key → error ────────────────────────────────

    #[tokio::test]
    async fn allow_with_wisdom_missing_key_returns_error() {
        let exec = SynthesizeExecutor::new();
        let step = synthesize_step("allow-with-wisdom", vec!["wisdomOutput".into()]);
        let mut ctx = GateContext::new(); // wisdomOutput not inserted

        let result = exec.execute(&step, &mut ctx, &event()).await;
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("expected Err but got Ok"),
        };
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(
            msg.contains("wisdomOutput"),
            "error should name the missing key, got: {msg}"
        );
    }

    // ─── allow-with-wisdom missing reasoning sub-object → mocked fallback ────

    #[tokio::test]
    async fn allow_with_wisdom_missing_reasoning_falls_back_to_mocked() {
        let exec = SynthesizeExecutor::new();
        let step = synthesize_step("allow-with-wisdom", vec!["wisdomOutput".into()]);

        let mut ctx = GateContext::new();
        ctx.insert("wisdomOutput", json!({"decision": "allow"})); // no "reasoning" key

        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();
        let (decision, _) = match outcome {
            StepOutcome::Terminate(d, se) => (d, se),
            _ => panic!("expected Terminate"),
        };
        assert!(decision.is_allowed());
        // Falls back to mocked — phase_note is NOT "wisdom-sourced"
        assert_ne!(
            decision.reasoning.phase_note, "wisdom-sourced",
            "missing reasoning should fall back to mocked, not wisdom-sourced"
        );
    }

    // ─── decline-from-wisdom when wisdom says allow → Allow ──────────────────

    #[tokio::test]
    async fn decline_from_wisdom_when_allow_returns_allow() {
        let exec = SynthesizeExecutor::new();
        let step = synthesize_step("decline-from-wisdom", vec!["wisdomOutput".into()]);

        let mut ctx = GateContext::new();
        ctx.insert(
            "wisdomOutput",
            json!({"decision": "allow", "reasoning": {"summary": "ok"}}),
        );

        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();
        let (decision, _) = match outcome {
            StepOutcome::Terminate(d, se) => (d, se),
            _ => panic!("expected Terminate"),
        };
        assert!(decision.is_allowed());
    }

    // ─── decline-from-wisdom when wisdom says decline → Decline ─────────────

    #[tokio::test]
    async fn decline_from_wisdom_when_decline_returns_decline() {
        let exec = SynthesizeExecutor::new();
        let step = synthesize_step("decline-from-wisdom", vec!["wisdomOutput".into()]);

        let mut ctx = GateContext::new();
        ctx.insert(
            "wisdomOutput",
            json!({
                "decision": "decline",
                "reasoning": {"summary": "content violates safety principle"}
            }),
        );

        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();
        let (decision, _) = match outcome {
            StepOutcome::Terminate(d, se) => (d, se),
            _ => panic!("expected Terminate"),
        };
        assert!(!decision.is_allowed());
        assert!(
            matches!(&decision.status, GateStatus::Decline { grounds } if grounds.category == "wisdom-decline"),
            "got: {:?}",
            decision.status
        );
    }

    // ─── verdict-from-rule → Verdict(StoryPoint) ─────────────────────────────

    #[tokio::test]
    async fn verdict_from_rule_returns_verdict() {
        let exec = SynthesizeExecutor::new();
        let step = synthesize_step("verdict-from-rule", vec!["ruleDecision".into()]);

        let mut ctx = GateContext::new();
        ctx.insert(
            "ruleDecision",
            json!({
                "tag_kind": "story-point",
                "valence": "constructive",
                "magnitude": "medium",
                "evidenceType": "direct"
            }),
        );

        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();
        let (decision, _) = match outcome {
            StepOutcome::Terminate(d, se) => (d, se),
            _ => panic!("expected Terminate"),
        };
        assert!(
            matches!(&decision.status,
                GateStatus::Verdict(GateTag::StoryPoint { valence, .. }) if valence == "constructive"
            ),
            "got: {:?}",
            decision.status
        );
    }

    // ─── verdict-from-rule with missing key → error ───────────────────────────

    #[tokio::test]
    async fn verdict_from_rule_missing_key_returns_error() {
        let exec = SynthesizeExecutor::new();
        let step = synthesize_step("verdict-from-rule", vec!["ruleDecision".into()]);
        let mut ctx = GateContext::new(); // ruleDecision not inserted

        let result = exec.execute(&step, &mut ctx, &event()).await;
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("expected Err but got Ok"),
        };
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(
            msg.contains("ruleDecision"),
            "error should name the missing key, got: {msg}"
        );
    }

    // ─── Unknown decision_builder → error ────────────────────────────────────

    #[tokio::test]
    async fn unknown_decision_builder_returns_error() {
        let exec = SynthesizeExecutor::new();
        let step = synthesize_step("no-such-builder", vec![]);
        let mut ctx = GateContext::new();

        let result = exec.execute(&step, &mut ctx, &event()).await;
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("expected Err but got Ok"),
        };
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(msg.contains("no-such-builder"), "got: {msg}");
    }

    // ─── Wrong step kind → error ──────────────────────────────────────────────

    #[tokio::test]
    async fn wrong_step_kind_returns_error() {
        let exec = SynthesizeExecutor::new();
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

    // ─── verdict-from-reach → Verdict(ReachLevel) ────────────────────────────

    #[tokio::test]
    async fn verdict_from_reach_returns_reach_level_verdict() {
        let exec = SynthesizeExecutor::new();
        let step = synthesize_step("verdict-from-reach", vec!["reachData".into()]);

        let mut ctx = GateContext::new();
        ctx.insert(
            "reachData",
            json!({ "level": "community", "count": 7, "edge_case": false, "reduction_kind": "threshold-ladder" }),
        );

        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();
        let (decision, _) = match outcome {
            StepOutcome::Terminate(d, se) => (d, se),
            _ => panic!("expected Terminate"),
        };
        assert!(
            matches!(
                &decision.status,
                GateStatus::Verdict(GateTag::ReachLevel { level }) if level == "community"
            ),
            "expected Verdict(ReachLevel{{community}}), got: {:?}",
            decision.status
        );
    }

    #[tokio::test]
    async fn verdict_from_reach_protocol_level() {
        let exec = SynthesizeExecutor::new();
        let step = synthesize_step("verdict-from-reach", vec!["reachData".into()]);

        let mut ctx = GateContext::new();
        ctx.insert(
            "reachData",
            json!({ "level": "protocol", "count": 55, "edge_case": false, "reduction_kind": "threshold-ladder" }),
        );

        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();
        let (decision, _) = match outcome {
            StepOutcome::Terminate(d, se) => (d, se),
            _ => panic!("expected Terminate"),
        };
        assert!(
            matches!(
                &decision.status,
                GateStatus::Verdict(GateTag::ReachLevel { level }) if level == "protocol"
            ),
            "got: {:?}",
            decision.status
        );
    }

    #[tokio::test]
    async fn verdict_from_reach_null_level_defaults_to_self_reach() {
        let exec = SynthesizeExecutor::new();
        let step = synthesize_step("verdict-from-reach", vec!["reachData".into()]);

        let mut ctx = GateContext::new();
        // level is null — no tier met
        ctx.insert(
            "reachData",
            json!({ "level": null, "count": 2, "edge_case": false, "reduction_kind": "threshold-ladder" }),
        );

        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();
        let (decision, _) = match outcome {
            StepOutcome::Terminate(d, se) => (d, se),
            _ => panic!("expected Terminate"),
        };
        assert!(
            matches!(
                &decision.status,
                GateStatus::Verdict(GateTag::ReachLevel { level }) if level == "self-reach"
            ),
            "null level must default to self-reach, got: {:?}",
            decision.status
        );
    }

    #[tokio::test]
    async fn verdict_from_reach_missing_key_returns_error() {
        let exec = SynthesizeExecutor::new();
        let step = synthesize_step("verdict-from-reach", vec!["reachData".into()]);
        let mut ctx = GateContext::new(); // reachData not inserted

        let result = exec.execute(&step, &mut ctx, &event()).await;
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("expected Err but got Ok"),
        };
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(
            msg.contains("reachData"),
            "error should name the missing key, got: {msg}"
        );
    }

    #[tokio::test]
    async fn verdict_from_reach_missing_level_field_defaults_to_self_reach() {
        let exec = SynthesizeExecutor::new();
        let step = synthesize_step("verdict-from-reach", vec!["reachData".into()]);

        let mut ctx = GateContext::new();
        // reachData object present but no "level" key
        ctx.insert("reachData", json!({ "count": 0, "edge_case": false }));

        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();
        let (decision, _) = match outcome {
            StepOutcome::Terminate(d, se) => (d, se),
            _ => panic!("expected Terminate"),
        };
        assert!(
            matches!(
                &decision.status,
                GateStatus::Verdict(GateTag::ReachLevel { level }) if level == "self-reach"
            ),
            "missing level field must default to self-reach, got: {:?}",
            decision.status
        );
    }

    // ─── Side effects spec with MintAttestation ──────────────────────────────

    #[tokio::test]
    async fn side_effects_are_converted_and_returned_in_decision() {
        let exec = SynthesizeExecutor::new();
        let step = StepType::Synthesize {
            params: SynthesizeParams {
                input_keys: vec![],
                decision_builder: "allow-passthrough".into(),
                side_effects: vec![SideEffectSpec {
                    effect_type: "MintAttestation".into(),
                    params_from_keys: vec!["momentEntryHash".into(), "ruleDecision".into()],
                    shape: None,
                }],
            },
        };

        let mut ctx = GateContext::new();
        ctx.insert("momentEntryHash", json!("hash-abc"));
        ctx.insert("ruleDecision", json!({"tag_kind": "story-point", "valence": "constructive", "magnitude": "medium", "evidenceType": "direct"}));

        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();
        let decision = match outcome {
            StepOutcome::Terminate(d, _) => d,
            _ => panic!("expected Terminate"),
        };
        // The decision's side_effects field carries the converted side effects.
        assert_eq!(decision.side_effects.len(), 1, "should have 1 side effect");
    }
}
