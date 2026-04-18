//! Core gate types — the decision shape returned from every gate invocation.
//!
//! Mirrors spec §1.3.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

use crate::phase::Phase;

/// The decision emitted from a gate invocation.
///
/// Every relational-impact write path receives one of these. The status
/// determines whether the caller may proceed; side effects declare work the
/// caller must perform (mint attestation, emit economic event, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../elohim-agent-sdk/src/gate-client/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct GateDecision {
    pub status: GateStatus,
    pub reasoning: ConstitutionalReasoningSummary,
    pub side_effects: Vec<SideEffect>,
    pub decision_attestation_cid: Option<String>,
    pub phase: Phase,
}

impl GateDecision {
    /// A mocked Allow returned during Phase 0 dev-context for boundary-crossing
    /// events. Carries a placeholder reasoning.
    pub fn allow_mocked(phase: Phase) -> Self {
        Self {
            status: GateStatus::Allow { exempt: false },
            reasoning: ConstitutionalReasoningSummary::mocked(),
            side_effects: Vec::new(),
            decision_attestation_cid: None,
            phase,
        }
    }

    /// An Allow returned for exempt interior spaces (offline, private drafting,
    /// play-interior). No wisdom invocation ran; the event never touched the
    /// gate's main path.
    pub fn allow_exempt(phase: Phase) -> Self {
        Self {
            status: GateStatus::Allow { exempt: true },
            reasoning: ConstitutionalReasoningSummary::exempt(),
            side_effects: Vec::new(),
            decision_attestation_cid: None,
            phase,
        }
    }

    /// Whether the caller may proceed with the relational-impact write.
    pub fn is_allowed(&self) -> bool {
        matches!(self.status, GateStatus::Allow { .. })
    }

    /// Whether this decision was exempt (gate did not fire because space was
    /// an interior).
    pub fn is_exempt(&self) -> bool {
        matches!(self.status, GateStatus::Allow { exempt: true })
    }
}

/// The four possible outcomes of a gate invocation.
///
/// Mirrors spec §1.3: Allow lets the caller proceed; Decline short-circuits;
/// Escalate routes to human review; Verdict emits a typed classification (e.g.,
/// StoryPointTag from the discernment gate).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../elohim-agent-sdk/src/gate-client/generated/")
)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum GateStatus {
    /// Caller may proceed. `exempt` indicates the gate did not fire (interior
    /// space); otherwise wisdom actively allowed the event.
    Allow { exempt: bool },

    /// Caller must not proceed. `grounds` carries the rationale for the decline.
    Decline { grounds: DeclineGrounds },

    /// Caller must not proceed; decision is routed to a reviewer. The caller
    /// typically returns a 202-with-review-link or similar to its upstream.
    Escalate {
        target: EscalationTarget,
        severity: Severity,
    },

    /// Evaluator-shape gates (like discernment-gate) emit a typed verdict.
    Verdict(GateTag),
}

/// Rationale for a Decline decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../elohim-agent-sdk/src/gate-client/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DeclineGrounds {
    pub category: String,
    pub summary: String,
    pub principle_refs: Vec<String>,
}

/// Where an escalated decision routes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../elohim-agent-sdk/src/gate-client/generated/")
)]
#[serde(tag = "kind", rename_all = "kebab-case", rename_all_fields = "camelCase")]
pub enum EscalationTarget {
    /// App-declared steward — fastest path, most context.
    AppSteward { steward_id: String },
    /// Qahal community review — persistent or cross-cutting concerns.
    Qahal { community_id: String },
    /// Existential-boundary enforcement — highest-reach protocol stewards.
    ExistentialBoundary,
}

/// Escalation severity tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../elohim-agent-sdk/src/gate-client/generated/")
)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Low,
    Medium,
    High,
    Existential,
}

/// A typed verdict from an evaluator-shape gate.
///
/// Example: `discernment-gate-v1-mechanical` emits a StoryPointTag variant
/// carrying valence + magnitude + evidenceType. `reach-gate` emits a ReachLevel
/// variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../elohim-agent-sdk/src/gate-client/generated/")
)]
#[serde(tag = "tag_kind", rename_all = "kebab-case", rename_all_fields = "camelCase")]
pub enum GateTag {
    /// Discernment gate output — carries the 7-valence classification.
    StoryPoint {
        valence: String,
        magnitude: String,
        evidence_type: String,
    },
    /// Reach gate output — computed reach level for a subject.
    ReachLevel { level: String },
    /// Content-safety gate output — coarse safety classification.
    ContentSafety { classification: String },
}

/// Side effects the caller must execute after a gate decision.
///
/// Per spec §1.3: "The caller executes side effects after the gate returns.
/// The gate library does not reach into conductor/DHT itself."
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../elohim-agent-sdk/src/gate-client/generated/")
)]
#[serde(tag = "effect", rename_all = "kebab-case", rename_all_fields = "camelCase")]
pub enum SideEffect {
    /// Mint an attestation on DHT.
    MintAttestation {
        shape: String,
        target_hash: String,
        tag_json: String,
    },
    /// Emit an economic event to the shefa REA economy.
    EmitEconomicEvent { event_json: String },
    /// Open a steward review for this event.
    OpenStewardReview {
        grounds_json: String,
        context_json: String,
    },
    /// Update reach aggregation for a subject.
    UpdateReachAggregation {
        subject_hash: String,
        delta_json: String,
    },
}

/// Placeholder for `ConstitutionalReasoning` until the elohim-agent crate
/// re-export is wired in Phase 1.
///
/// During DevContext, gate decisions carry a placeholder reasoning indicating
/// the rehearsal phase; during ElohimActive, this will be replaced by the
/// full `ConstitutionalReasoning` struct from `elohim-agent::response`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(
    feature = "typescript",
    ts(export, export_to = "../../elohim-agent-sdk/src/gate-client/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct ConstitutionalReasoningSummary {
    pub primary_principle: String,
    pub summary: String,
    pub confidence: f32,
    pub phase_note: String,
}

impl ConstitutionalReasoningSummary {
    pub fn mocked() -> Self {
        Self {
            primary_principle: "dev-context-mock".to_string(),
            summary: "Rehearsal phase: wisdom-invoke mocked to Allow.".to_string(),
            confidence: 0.0,
            phase_note: "This decision carries no reputation weight.".to_string(),
        }
    }

    pub fn exempt() -> Self {
        Self {
            primary_principle: "exempt-interior".to_string(),
            summary: "Event occurred in an exempt interior space; gate did not fire.".to_string(),
            confidence: 1.0,
            phase_note: "Architectural boundary, not a wisdom judgment.".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase::Phase;

    // ─── GateDecision builder invariants ─────────────────────────────────────

    #[test]
    fn allow_mocked_dev_context_shape() {
        let d = GateDecision::allow_mocked(Phase::DevContext);
        // GateStatus does not derive PartialEq; use matches! for structural check.
        assert!(
            matches!(d.status, GateStatus::Allow { exempt: false }),
            "expected Allow {{ exempt: false }}"
        );
        assert_eq!(d.phase, Phase::DevContext);
        assert!(d.side_effects.is_empty());
        assert!(d.decision_attestation_cid.is_none());
        assert!(d.is_allowed());
        assert!(!d.is_exempt());
    }

    #[test]
    fn allow_mocked_elohim_active_phase() {
        let d = GateDecision::allow_mocked(Phase::ElohimActive);
        assert!(
            matches!(d.status, GateStatus::Allow { exempt: false }),
            "expected Allow {{ exempt: false }}"
        );
        assert_eq!(d.phase, Phase::ElohimActive);
        assert!(d.is_allowed());
        assert!(!d.is_exempt());
    }

    #[test]
    fn allow_exempt_dev_context_shape() {
        let d = GateDecision::allow_exempt(Phase::DevContext);
        assert!(
            matches!(d.status, GateStatus::Allow { exempt: true }),
            "expected Allow {{ exempt: true }}"
        );
        assert_eq!(d.phase, Phase::DevContext);
        assert!(d.side_effects.is_empty());
        assert!(d.decision_attestation_cid.is_none());
        assert!(d.is_allowed());
        assert!(d.is_exempt());
    }

    #[test]
    fn allow_exempt_reasoning_uses_exempt_summary() {
        let d = GateDecision::allow_exempt(Phase::DevContext);
        assert_eq!(d.reasoning.primary_principle, "exempt-interior");
        // Exempt reasoning has confidence 1.0 — no wisdom judgment, just an
        // architectural short-circuit.
        assert!((d.reasoning.confidence - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn allow_mocked_reasoning_uses_mocked_summary() {
        let d = GateDecision::allow_mocked(Phase::DevContext);
        assert_eq!(d.reasoning.primary_principle, "dev-context-mock");
        // Mocked reasoning has confidence 0.0 — no real wisdom invocation ran.
        assert!((d.reasoning.confidence - 0.0).abs() < f32::EPSILON);
    }

    // ─── is_allowed() — true for all Allow variants ──────────────────────────

    #[test]
    fn is_allowed_true_for_allow_non_exempt() {
        let d = GateDecision::allow_mocked(Phase::DevContext);
        assert!(d.is_allowed());
    }

    #[test]
    fn is_allowed_true_for_allow_exempt() {
        let d = GateDecision::allow_exempt(Phase::DevContext);
        assert!(d.is_allowed());
    }

    #[test]
    fn is_allowed_false_for_decline() {
        let d = GateDecision {
            status: GateStatus::Decline {
                grounds: DeclineGrounds {
                    category: "test".to_string(),
                    summary: "test".to_string(),
                    principle_refs: Vec::new(),
                },
            },
            reasoning: ConstitutionalReasoningSummary::mocked(),
            side_effects: Vec::new(),
            decision_attestation_cid: None,
            phase: Phase::DevContext,
        };
        assert!(!d.is_allowed());
    }

    #[test]
    fn is_allowed_false_for_escalate() {
        let d = GateDecision {
            status: GateStatus::Escalate {
                target: EscalationTarget::ExistentialBoundary,
                severity: Severity::High,
            },
            reasoning: ConstitutionalReasoningSummary::mocked(),
            side_effects: Vec::new(),
            decision_attestation_cid: None,
            phase: Phase::DevContext,
        };
        assert!(!d.is_allowed());
    }

    #[test]
    fn is_allowed_false_for_verdict() {
        let d = GateDecision {
            status: GateStatus::Verdict(GateTag::ReachLevel {
                level: "local".to_string(),
            }),
            reasoning: ConstitutionalReasoningSummary::mocked(),
            side_effects: Vec::new(),
            decision_attestation_cid: None,
            phase: Phase::DevContext,
        };
        assert!(!d.is_allowed());
    }

    // ─── is_exempt() — true only for Allow { exempt: true } ──────────────────

    #[test]
    fn is_exempt_false_for_non_exempt_allow() {
        let d = GateDecision::allow_mocked(Phase::DevContext);
        assert!(!d.is_exempt());
    }

    #[test]
    fn is_exempt_false_for_decline() {
        let d = GateDecision {
            status: GateStatus::Decline {
                grounds: DeclineGrounds {
                    category: "test".to_string(),
                    summary: "test".to_string(),
                    principle_refs: Vec::new(),
                },
            },
            reasoning: ConstitutionalReasoningSummary::mocked(),
            side_effects: Vec::new(),
            decision_attestation_cid: None,
            phase: Phase::DevContext,
        };
        assert!(!d.is_exempt());
    }

    // ─── ConstitutionalReasoningSummary serde ─────────────────────────────────

    #[test]
    fn constitutional_reasoning_summary_mocked_round_trips() {
        let orig = ConstitutionalReasoningSummary::mocked();
        let json = serde_json::to_string(&orig).expect("serialize");
        let rt: ConstitutionalReasoningSummary = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(rt.primary_principle, orig.primary_principle);
        assert!((rt.confidence - orig.confidence).abs() < f32::EPSILON);
    }

    // ─── GateStatus serde discriminants ──────────────────────────────────────
    //
    // Confirm the `status` tag field uses kebab-case per the serde attribute.

    #[test]
    fn gate_status_allow_has_correct_tag() {
        let status = GateStatus::Allow { exempt: false };
        let json = serde_json::to_string(&status).expect("serialize");
        assert!(json.contains("\"status\":\"allow\""), "got: {json}");
    }

    #[test]
    fn gate_status_decline_has_correct_tag() {
        let status = GateStatus::Decline {
            grounds: DeclineGrounds {
                category: "c".to_string(),
                summary: "s".to_string(),
                principle_refs: Vec::new(),
            },
        };
        let json = serde_json::to_string(&status).expect("serialize");
        assert!(json.contains("\"status\":\"decline\""), "got: {json}");
    }

    #[test]
    fn gate_status_escalate_has_correct_tag() {
        let status = GateStatus::Escalate {
            target: EscalationTarget::ExistentialBoundary,
            severity: Severity::Low,
        };
        let json = serde_json::to_string(&status).expect("serialize");
        assert!(json.contains("\"status\":\"escalate\""), "got: {json}");
    }

    // ─── EscalationTarget serde tag variants ─────────────────────────────────

    #[test]
    fn escalation_target_app_steward_tag() {
        let t = EscalationTarget::AppSteward {
            steward_id: "s-1".to_string(),
        };
        let json = serde_json::to_string(&t).expect("serialize");
        assert!(json.contains("\"kind\":\"app-steward\""), "got: {json}");
        assert!(json.contains("stewardId"), "camelCase field expected, got: {json}");
    }

    #[test]
    fn escalation_target_qahal_tag() {
        let t = EscalationTarget::Qahal {
            community_id: "c-1".to_string(),
        };
        let json = serde_json::to_string(&t).expect("serialize");
        assert!(json.contains("\"kind\":\"qahal\""), "got: {json}");
        assert!(json.contains("communityId"), "camelCase field expected, got: {json}");
    }

    #[test]
    fn escalation_target_existential_boundary_tag() {
        let t = EscalationTarget::ExistentialBoundary;
        let json = serde_json::to_string(&t).expect("serialize");
        assert!(json.contains("\"kind\":\"existential-boundary\""), "got: {json}");
    }
}
