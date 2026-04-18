//! EscalateToReviewExecutor — terminal step routing to steward/qahal/existential review.
//!
//! Phase 2: Parses `severity` from the step params, constructs a
//! `GateStatus::Escalate` with `EscalationTarget::ExistentialBoundary` (real
//! target resolution via `target_spec_cid` is Phase 3+), and terminates the DAG.

use async_trait::async_trait;
use tracing::warn;

use crate::dag::executor::{StepExecutor, StepOutcome};
use crate::dag::{EscalateToReviewParams, StepType};
use crate::error::GateError;
use crate::events::RelationalImpactEvent;
use crate::phase::Phase;
use crate::types::{
    ConstitutionalReasoningSummary, EscalationTarget, GateDecision, GateStatus, Severity,
};

use super::super::context::GateContext;

// ─── EscalateToReviewExecutor ─────────────────────────────────────────────────

/// Executor for `StepType::EscalateToReview`.
///
/// Phase 2 behaviour:
/// - Parses `params.severity` into the `Severity` enum.
/// - Constructs `GateStatus::Escalate` with `EscalationTarget::ExistentialBoundary`
///   (real target resolution via `target_spec_cid` arrives in Phase 3+).
/// - Logs the escalation via `tracing::warn!`.
/// - Returns `StepOutcome::Terminate`.
pub struct EscalateToReviewExecutor;

impl EscalateToReviewExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EscalateToReviewExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a severity string into the `Severity` enum.
///
/// Accepted values (case-insensitive): `low`, `medium`, `high`, `existential`.
/// All other strings return `GateError::DagExecution`.
fn parse_severity(s: &str) -> Result<Severity, GateError> {
    match s.to_lowercase().as_str() {
        "low" => Ok(Severity::Low),
        "medium" => Ok(Severity::Medium),
        "high" => Ok(Severity::High),
        "existential" => Ok(Severity::Existential),
        unknown => Err(GateError::DagExecution(format!(
            "EscalateToReview: unknown severity '{unknown}'; \
             expected one of: low, medium, high, existential"
        ))),
    }
}

#[async_trait]
impl StepExecutor for EscalateToReviewExecutor {
    async fn execute(
        &self,
        step: &StepType,
        _ctx: &mut GateContext,
        _event: &RelationalImpactEvent,
    ) -> Result<StepOutcome, GateError> {
        let params = match step {
            StepType::EscalateToReview { params } => params,
            _ => {
                return Err(GateError::DagExecution(
                    "EscalateToReviewExecutor received wrong step kind".to_string(),
                ))
            }
        };

        let severity = parse_severity(&params.severity)?;

        // Phase 3+: resolve params.target_spec_cid to a concrete EscalationTarget.
        // Phase 2: always ExistentialBoundary — the CID is carried through but not resolved.
        let target = EscalationTarget::ExistentialBoundary;

        warn!(
            escalate.target_spec_cid = %params.target_spec_cid,
            escalate.severity = ?severity,
            "EscalateToReview: routing to existential-boundary review (Phase 2 — target CID not resolved)"
        );

        let decision = GateDecision {
            status: GateStatus::Escalate { target, severity },
            reasoning: ConstitutionalReasoningSummary {
                primary_principle: "escalate-to-review".to_string(),
                summary: format!(
                    "Event escalated for review. Severity: {severity:?}. \
                     Target spec CID: {} (not resolved in Phase 2).",
                    params.target_spec_cid
                ),
                confidence: 0.0,
                phase_note: "Phase 2 escalation stub — target resolution in Phase 3+.".to_string(),
            },
            side_effects: vec![],
            decision_attestation_cid: None,
            phase: Phase::DevContext,
        };

        Ok(StepOutcome::Terminate(decision, vec![]))
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

impl EscalateToReviewParams {
    /// Parse the severity string field. Exposed for use in tests.
    pub fn parsed_severity(&self) -> Result<Severity, GateError> {
        parse_severity(&self.severity)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::EscalateToReviewParams;
    use crate::events::RelationalImpactEvent;
    use crate::types::{GateStatus, Severity};

    fn event() -> RelationalImpactEvent {
        RelationalImpactEvent::ContentPublish {
            content_cid: "cid".into(),
            declared_reach: "public".into(),
            author: "agent".into(),
        }
    }

    fn escalate_step(severity: &str) -> StepType {
        StepType::EscalateToReview {
            params: EscalateToReviewParams {
                target_spec_cid: "bafytargetspec".into(),
                severity: severity.into(),
            },
        }
    }

    // ─── Happy path: each severity variant ───────────────────────────────────

    #[tokio::test]
    async fn escalate_low_severity_terminates_with_escalate_status() {
        let exec = EscalateToReviewExecutor::new();
        let mut ctx = GateContext::new();
        let outcome = exec
            .execute(&escalate_step("low"), &mut ctx, &event())
            .await
            .unwrap();

        let decision = match outcome {
            StepOutcome::Terminate(d, _) => d,
            _ => panic!("expected Terminate"),
        };
        assert!(
            matches!(
                &decision.status,
                GateStatus::Escalate {
                    severity: Severity::Low,
                    ..
                }
            ),
            "got: {:?}",
            decision.status
        );
    }

    #[tokio::test]
    async fn escalate_medium_severity() {
        let exec = EscalateToReviewExecutor::new();
        let mut ctx = GateContext::new();
        let outcome = exec
            .execute(&escalate_step("medium"), &mut ctx, &event())
            .await
            .unwrap();
        let decision = match outcome {
            StepOutcome::Terminate(d, _) => d,
            _ => panic!("expected Terminate"),
        };
        assert!(matches!(
            &decision.status,
            GateStatus::Escalate {
                severity: Severity::Medium,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn escalate_high_severity() {
        let exec = EscalateToReviewExecutor::new();
        let mut ctx = GateContext::new();
        let outcome = exec
            .execute(&escalate_step("high"), &mut ctx, &event())
            .await
            .unwrap();
        let decision = match outcome {
            StepOutcome::Terminate(d, _) => d,
            _ => panic!("expected Terminate"),
        };
        assert!(matches!(
            &decision.status,
            GateStatus::Escalate {
                severity: Severity::High,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn escalate_existential_severity() {
        let exec = EscalateToReviewExecutor::new();
        let mut ctx = GateContext::new();
        let outcome = exec
            .execute(&escalate_step("existential"), &mut ctx, &event())
            .await
            .unwrap();
        let decision = match outcome {
            StepOutcome::Terminate(d, _) => d,
            _ => panic!("expected Terminate"),
        };
        assert!(matches!(
            &decision.status,
            GateStatus::Escalate {
                severity: Severity::Existential,
                ..
            }
        ));
    }

    // ─── Case-insensitive severity parsing ───────────────────────────────────

    #[tokio::test]
    async fn severity_parsing_is_case_insensitive() {
        let exec = EscalateToReviewExecutor::new();
        let mut ctx = GateContext::new();
        let outcome = exec
            .execute(&escalate_step("HIGH"), &mut ctx, &event())
            .await
            .unwrap();
        let decision = match outcome {
            StepOutcome::Terminate(d, _) => d,
            _ => panic!("expected Terminate"),
        };
        assert!(matches!(
            &decision.status,
            GateStatus::Escalate {
                severity: Severity::High,
                ..
            }
        ));
    }

    // ─── Unknown severity → error ─────────────────────────────────────────────

    #[tokio::test]
    async fn unknown_severity_returns_dag_execution_error() {
        let exec = EscalateToReviewExecutor::new();
        let mut ctx = GateContext::new();
        let result = exec
            .execute(&escalate_step("ultra"), &mut ctx, &event())
            .await;
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("expected Err but got Ok"),
        };
        assert!(matches!(err, GateError::DagExecution(_)));
        let msg = err.to_string();
        assert!(
            msg.contains("ultra"),
            "error should name the unknown severity, got: {msg}"
        );
    }

    // ─── ExistentialBoundary is the Phase 2 stub target ──────────────────────

    #[tokio::test]
    async fn phase2_stub_target_is_existential_boundary() {
        let exec = EscalateToReviewExecutor::new();
        let mut ctx = GateContext::new();
        let outcome = exec
            .execute(&escalate_step("high"), &mut ctx, &event())
            .await
            .unwrap();
        let decision = match outcome {
            StepOutcome::Terminate(d, _) => d,
            _ => panic!("expected Terminate"),
        };
        assert!(
            matches!(
                &decision.status,
                GateStatus::Escalate {
                    target: EscalationTarget::ExistentialBoundary,
                    ..
                }
            ),
            "Phase 2 must use ExistentialBoundary as stub target, got: {:?}",
            decision.status
        );
    }

    // ─── Target spec CID is included in reasoning summary ────────────────────

    #[tokio::test]
    async fn target_spec_cid_appears_in_reasoning_summary() {
        let exec = EscalateToReviewExecutor::new();
        let step = StepType::EscalateToReview {
            params: EscalateToReviewParams {
                target_spec_cid: "bafyspecific-cid".into(),
                severity: "high".into(),
            },
        };
        let mut ctx = GateContext::new();
        let outcome = exec.execute(&step, &mut ctx, &event()).await.unwrap();
        let decision = match outcome {
            StepOutcome::Terminate(d, _) => d,
            _ => panic!("expected Terminate"),
        };
        assert!(
            decision.reasoning.summary.contains("bafyspecific-cid"),
            "CID should appear in reasoning summary, got: {}",
            decision.reasoning.summary
        );
    }

    // ─── Wrong step kind → error ──────────────────────────────────────────────

    #[tokio::test]
    async fn wrong_step_kind_returns_error() {
        let exec = EscalateToReviewExecutor::new();
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
