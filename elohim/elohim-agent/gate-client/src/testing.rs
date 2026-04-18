//! Testing helpers for downstream crates that need to mock gate decisions.
//!
//! Per spec §6.1: "Testing helpers in `gate-client::testing`".

use crate::phase::Phase;
use crate::types::{
    ConstitutionalReasoningSummary, DeclineGrounds, EscalationTarget, GateDecision, GateStatus,
    GateTag, Severity,
};

pub fn mock_allow() -> GateDecision {
    GateDecision::allow_mocked(Phase::DevContext)
}

pub fn mock_decline(category: &str, summary: &str) -> GateDecision {
    GateDecision {
        status: GateStatus::Decline {
            grounds: DeclineGrounds {
                category: category.to_string(),
                summary: summary.to_string(),
                principle_refs: Vec::new(),
            },
        },
        reasoning: ConstitutionalReasoningSummary::mocked(),
        side_effects: Vec::new(),
        decision_attestation_cid: None,
        phase: Phase::DevContext,
    }
}

pub fn mock_escalate(target: EscalationTarget, severity: Severity) -> GateDecision {
    GateDecision {
        status: GateStatus::Escalate { target, severity },
        reasoning: ConstitutionalReasoningSummary::mocked(),
        side_effects: Vec::new(),
        decision_attestation_cid: None,
        phase: Phase::DevContext,
    }
}

pub fn mock_verdict(tag: GateTag) -> GateDecision {
    GateDecision {
        status: GateStatus::Verdict(tag),
        reasoning: ConstitutionalReasoningSummary::mocked(),
        side_effects: Vec::new(),
        decision_attestation_cid: None,
        phase: Phase::DevContext,
    }
}
