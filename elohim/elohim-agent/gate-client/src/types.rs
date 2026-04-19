//! Core gate types — the decision shape returned from every gate invocation.
//!
//! All type definitions live in `gate-types::types`; this module re-exports
//! them for backward compatibility with any code importing
//! `gate_client::types::*`.
//!
//! Tests remain here to exercise the types from the gate-client consumer
//! perspective.

pub use gate_types::types::{
    ConstitutionalReasoningSummary, DeclineGrounds, EscalationTarget, GateDecision, GateStatus,
    GateTag, Severity, SideEffect,
};

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

    #[test]
    fn constitutional_reasoning_summary_exempt_round_trips() {
        let original = ConstitutionalReasoningSummary::exempt();
        let json = serde_json::to_string(&original).unwrap();
        let restored: ConstitutionalReasoningSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.primary_principle, original.primary_principle);
        assert_eq!(restored.summary, original.summary);
        assert_eq!(restored.confidence, original.confidence);
        assert_eq!(restored.phase_note, original.phase_note);
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
        assert!(
            json.contains("stewardId"),
            "camelCase field expected, got: {json}"
        );
    }

    #[test]
    fn escalation_target_qahal_tag() {
        let t = EscalationTarget::Qahal {
            community_id: "c-1".to_string(),
        };
        let json = serde_json::to_string(&t).expect("serialize");
        assert!(json.contains("\"kind\":\"qahal\""), "got: {json}");
        assert!(
            json.contains("communityId"),
            "camelCase field expected, got: {json}"
        );
    }

    #[test]
    fn escalation_target_existential_boundary_tag() {
        let t = EscalationTarget::ExistentialBoundary;
        let json = serde_json::to_string(&t).expect("serialize");
        assert!(
            json.contains("\"kind\":\"existential-boundary\""),
            "got: {json}"
        );
    }
}
