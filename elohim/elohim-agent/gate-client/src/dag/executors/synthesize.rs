//! SynthesizeExecutor — composes prior step outputs into a final GateDecision.
//!
//! Reads `params.input_keys` from GateContext, applies the named
//! `decision_builder`, and returns `StepOutcome::Terminate` with the composed
//! `GateDecision` + converted side effects.

use async_trait::async_trait;
use tracing::debug;

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

        let status = build_status(params, ctx)?;
        let side_effects = convert_side_effect_specs(&params.side_effects, ctx)?;

        let decision = GateDecision {
            status,
            reasoning: ConstitutionalReasoningSummary::mocked(),
            side_effects,
            decision_attestation_cid: None,
            phase: Phase::DevContext,
        };

        Ok(StepOutcome::Terminate(decision, vec![]))
    }
}

/// Apply the `decision_builder` selector and return the appropriate `GateStatus`.
fn build_status(params: &SynthesizeParams, ctx: &GateContext) -> Result<GateStatus, GateError> {
    match params.decision_builder.as_str() {
        "allow-passthrough" => Ok(GateStatus::Allow { exempt: false }),

        "allow-with-wisdom" => {
            // Read wisdom output — default key is "wisdomOutput".
            let wisdom_key = params
                .input_keys
                .first()
                .map(String::as_str)
                .unwrap_or("wisdomOutput");
            let _wisdom = ctx.get(wisdom_key); // presence is best-effort in dev-context
            Ok(GateStatus::Allow { exempt: false })
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

                Ok(GateStatus::Decline {
                    grounds: DeclineGrounds {
                        category: "wisdom-decline".to_string(),
                        summary,
                        principle_refs: vec![],
                    },
                })
            } else {
                Ok(GateStatus::Allow { exempt: false })
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
            Ok(GateStatus::Verdict(tag))
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
        assert!(matches!(decision.status, GateStatus::Allow { exempt: false }));
        assert!(decision.side_effects.is_empty());
    }

    // ─── allow-with-wisdom → Allow ────────────────────────────────────────────

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

    // ─── decline-from-wisdom when wisdom says allow → Allow ──────────────────

    #[tokio::test]
    async fn decline_from_wisdom_when_allow_returns_allow() {
        let exec = SynthesizeExecutor::new();
        let step = synthesize_step("decline-from-wisdom", vec!["wisdomOutput".into()]);

        let mut ctx = GateContext::new();
        ctx.insert("wisdomOutput", json!({"decision": "allow", "reasoning": {"summary": "ok"}}));

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
        ctx.insert("wisdomOutput", json!({
            "decision": "decline",
            "reasoning": {"summary": "content violates safety principle"}
        }));

        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();
        let (decision, _) = match outcome {
            StepOutcome::Terminate(d, se) => (d, se),
            _ => panic!("expected Terminate"),
        };
        assert!(!decision.is_allowed());
        assert!(
            matches!(&decision.status, GateStatus::Decline { grounds } if grounds.category == "wisdom-decline"),
            "got: {:?}", decision.status
        );
    }

    // ─── verdict-from-rule → Verdict(StoryPoint) ─────────────────────────────

    #[tokio::test]
    async fn verdict_from_rule_returns_verdict() {
        let exec = SynthesizeExecutor::new();
        let step = synthesize_step("verdict-from-rule", vec!["ruleDecision".into()]);

        let mut ctx = GateContext::new();
        ctx.insert("ruleDecision", json!({
            "tag_kind": "story-point",
            "valence": "constructive",
            "magnitude": "medium",
            "evidenceType": "direct"
        }));

        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();
        let (decision, _) = match outcome {
            StepOutcome::Terminate(d, se) => (d, se),
            _ => panic!("expected Terminate"),
        };
        assert!(
            matches!(&decision.status,
                GateStatus::Verdict(GateTag::StoryPoint { valence, .. }) if valence == "constructive"
            ),
            "got: {:?}", decision.status
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
        assert!(msg.contains("ruleDecision"), "error should name the missing key, got: {msg}");
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
                    params_from_keys: vec![
                        "momentEntryHash".into(),
                        "ruleDecision".into(),
                    ],
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
