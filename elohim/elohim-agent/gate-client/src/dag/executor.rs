//! StepExecutor trait, StepOutcome enum, and StepKind discriminator.
//!
//! The interpreter holds a `HashMap<StepKind, Arc<dyn StepExecutor>>` and
//! dispatches each step to the right executor by calling `step.kind()`.
//! Task 2.2 adds the concrete implementations; Task 2.1 ships the trait +
//! a `TestExecutor` for unit tests only.

use std::sync::Arc;

use crate::error::GateError;
use crate::events::RelationalImpactEvent;
use crate::types::{GateDecision, SideEffect};
use async_trait::async_trait;

use super::context::GateContext;
use super::StepType;

// ─── StepKind ────────────────────────────────────────────────────────────────

/// A discriminator enum that mirrors the variants of [`StepType`] without
/// carrying any data. Used as the key in the executor dispatch table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StepKind {
    ContextAssemble,
    WisdomInvoke,
    MechanicalRuleset,
    AggregateAttestations,
    SkillInvoke,
    Synthesize,
    EscalateToReview,
}

impl StepType {
    /// Return the kind discriminator for this step variant.
    pub fn kind(&self) -> StepKind {
        match self {
            StepType::ContextAssemble { .. } => StepKind::ContextAssemble,
            StepType::WisdomInvoke { .. } => StepKind::WisdomInvoke,
            StepType::MechanicalRuleset { .. } => StepKind::MechanicalRuleset,
            StepType::AggregateAttestations { .. } => StepKind::AggregateAttestations,
            StepType::SkillInvoke { .. } => StepKind::SkillInvoke,
            StepType::Synthesize { .. } => StepKind::Synthesize,
            StepType::EscalateToReview { .. } => StepKind::EscalateToReview,
        }
    }
}

// ─── StepOutcome ─────────────────────────────────────────────────────────────

/// What a step executor returns to the interpreter after executing.
#[allow(clippy::large_enum_variant)]
pub enum StepOutcome {
    /// The DAG should continue walking edges to find the next step or terminal.
    Continue,

    /// The executor has determined the final decision; short-circuit and return.
    /// Boxed-vs-inline is a Phase 3+ optimization — the wrapped `GateDecision`
    /// is small enough today and inlining keeps the trait signature simpler.
    Terminate(GateDecision, Vec<SideEffect>),
}

// ─── StepExecutor ────────────────────────────────────────────────────────────

/// A pluggable executor for a specific [`StepKind`].
///
/// The interpreter calls `execute()` with the full [`StepType`] (carrying all
/// params) so each executor only needs to match the variant it owns.
///
/// Task 2.2 implements the seven concrete executors. This trait is the only
/// thing the interpreter knows about.
#[async_trait]
pub trait StepExecutor: Send + Sync {
    async fn execute(
        &self,
        step: &StepType,
        ctx: &mut GateContext,
        event: &RelationalImpactEvent,
    ) -> Result<StepOutcome, GateError>;
}

/// Convenience alias used throughout the interpreter.
pub type ArcExecutor = Arc<dyn StepExecutor>;

// ─── ContextValue helper (used inside test executor) ─────────────────────────

/// A test-only executor that writes a fixed `Value` to a fixed context key
/// and always returns `Continue`. Used to drive the interpreter in unit tests
/// without implementing real business logic.
///
/// Only available under `#[cfg(test)]`.
#[cfg(test)]
pub struct TestExecutor {
    /// The key to write into the context on each call.
    pub output_key: String,
    /// The value to write.
    pub output_value: serde_json::Value,
}

#[cfg(test)]
#[async_trait]
impl StepExecutor for TestExecutor {
    async fn execute(
        &self,
        _step: &StepType,
        ctx: &mut GateContext,
        _event: &RelationalImpactEvent,
    ) -> Result<StepOutcome, GateError> {
        ctx.insert(self.output_key.clone(), self.output_value.clone());
        Ok(StepOutcome::Continue)
    }
}

/// A test-only executor that immediately terminates with a provided decision.
#[cfg(test)]
pub struct TerminateExecutor {
    pub decision: GateDecision,
}

#[cfg(test)]
#[async_trait]
impl StepExecutor for TerminateExecutor {
    async fn execute(
        &self,
        _step: &StepType,
        _ctx: &mut GateContext,
        _event: &RelationalImpactEvent,
    ) -> Result<StepOutcome, GateError> {
        Ok(StepOutcome::Terminate(self.decision.clone(), vec![]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::RelationalImpactEvent;
    use crate::phase::Phase;
    use serde_json::json;

    fn sample_event() -> RelationalImpactEvent {
        RelationalImpactEvent::ContentPublish {
            content_cid: "cid".into(),
            declared_reach: "public".into(),
            author: "agent".into(),
        }
    }

    fn sample_step() -> StepType {
        StepType::ContextAssemble {
            params: crate::dag::ContextAssembleParams { pulls: vec![] },
        }
    }

    #[tokio::test]
    async fn test_executor_writes_to_context_and_continues() {
        let exec = TestExecutor {
            output_key: "result".into(),
            output_value: json!("hello"),
        };
        let mut ctx = GateContext::new();
        let outcome = exec
            .execute(&sample_step(), &mut ctx, &sample_event())
            .await
            .unwrap();
        assert!(matches!(outcome, StepOutcome::Continue));
        assert_eq!(ctx.get("result"), Some(&json!("hello")));
    }

    #[tokio::test]
    async fn terminate_executor_short_circuits() {
        let exec = TerminateExecutor {
            decision: GateDecision::allow_mocked(Phase::DevContext),
        };
        let mut ctx = GateContext::new();
        let outcome = exec
            .execute(&sample_step(), &mut ctx, &sample_event())
            .await
            .unwrap();
        assert!(matches!(outcome, StepOutcome::Terminate(_, _)));
    }

    #[test]
    fn step_kind_covers_all_variants() {
        use crate::dag::{
            AggregateAttestationsParams, ContextAssembleParams, EscalateToReviewParams,
            MechanicalRulesetParams, SkillInvokeParams, SynthesizeParams, WisdomInvokeParams,
        };

        let cases = vec![
            (
                StepType::ContextAssemble {
                    params: ContextAssembleParams { pulls: vec![] },
                },
                StepKind::ContextAssemble,
            ),
            (
                StepType::WisdomInvoke {
                    params: WisdomInvokeParams {
                        constitution_cid: "c".into(),
                        framing_cid: "f".into(),
                        context_keys: vec![],
                        output_key: "o".into(),
                    },
                },
                StepKind::WisdomInvoke,
            ),
            (
                StepType::MechanicalRuleset {
                    params: MechanicalRulesetParams {
                        rules_cid: "r".into(),
                        input_keys: vec![],
                        output_key: "o".into(),
                    },
                },
                StepKind::MechanicalRuleset,
            ),
            (
                StepType::AggregateAttestations {
                    params: AggregateAttestationsParams {
                        aggregation_spec_cid: "a".into(),
                        subject_key: "s".into(),
                        output_key: "o".into(),
                    },
                },
                StepKind::AggregateAttestations,
            ),
            (
                StepType::SkillInvoke {
                    params: SkillInvokeParams {
                        capability: "cap".into(),
                        request_from_keys: vec![],
                        output_key: "o".into(),
                    },
                },
                StepKind::SkillInvoke,
            ),
            (
                StepType::Synthesize {
                    params: SynthesizeParams {
                        input_keys: vec![],
                        decision_builder: "b".into(),
                        side_effects: vec![],
                    },
                },
                StepKind::Synthesize,
            ),
            (
                StepType::EscalateToReview {
                    params: EscalateToReviewParams {
                        target_spec_cid: "t".into(),
                        severity: "high".into(),
                    },
                },
                StepKind::EscalateToReview,
            ),
        ];

        for (step, expected_kind) in cases {
            assert_eq!(step.kind(), expected_kind);
        }
    }
}
