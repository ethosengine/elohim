//! ElohimGate — mutation interceptor for protocol-level agent reasoning.
//!
//! Every mutation passes through the gate. The gate computes a TrustContext,
//! classifies an InferenceTier, and returns a GateResult that determines
//! how the mutation settles.

use constitution::ConstitutionalLayer;

/// Raw trust signals gathered from DB queries.
#[derive(Debug, Clone)]
pub struct TrustSignals {
    pub mastery_depth: f64,
    pub steward_standing: f64,
    pub relationship_density: f64,
    pub governance_health: f64,
    pub behavioral_trust: f64,
    pub intent_divergence: f64,
}

/// What kind of mutation is being attempted.
/// Extensible — SDK developers register new types here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationType {
    // Internal bookkeeping — never gated
    MasteryUpdate,
    SessionHeartbeat,
    InternalSync,

    // Curation — light gate for trusted stewards
    CurationEvent,
    AllocationUpdate,

    // Human boundary — full gate for untrusted
    Comment,
    Reaction,
    ContentPublish,

    // Recognition pipeline — usually pass-through
    RecognitionTrigger,

    // Governance — always elevated
    DisputeFiling,
    GovernanceVote,
    ReachChange,

    // Catch-all for SDK extensions
    Custom(u32),
}

/// How much ceremony this mutation receives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceTier {
    None,
    Light,
    Full,
    Constitutional,
}

impl InferenceTier {
    /// Escalate one level.
    pub fn escalate(self) -> Self {
        match self {
            Self::None => Self::Light,
            Self::Light => Self::Full,
            Self::Full => Self::Constitutional,
            Self::Constitutional => Self::Constitutional,
        }
    }

    /// Classify the inference tier for a mutation given trust context.
    pub fn classify(mutation: MutationType, ctx: &TrustContext) -> Self {
        let base = Self::base_tier(mutation, ctx.composite_trust);

        // Intent divergence escalates
        if ctx.intent_divergence > 0.5 {
            base.escalate()
        } else {
            base
        }
    }

    fn base_tier(mutation: MutationType, trust: f64) -> Self {
        use MutationType::*;

        match mutation {
            // Never gated
            MasteryUpdate | SessionHeartbeat | InternalSync => Self::None,

            // Curation
            CurationEvent | AllocationUpdate => {
                if trust > 0.6 {
                    Self::Light
                } else {
                    Self::Full
                }
            }

            // Human boundary
            Comment | Reaction | ContentPublish => {
                if trust > 0.7 {
                    Self::Light
                } else {
                    Self::Full
                }
            }

            // Recognition
            RecognitionTrigger => {
                if trust > 0.4 {
                    Self::None
                } else {
                    Self::Light
                }
            }

            // Governance — always elevated
            DisputeFiling => Self::Full,
            GovernanceVote => {
                if trust > 0.8 {
                    Self::Full
                } else {
                    Self::Constitutional
                }
            }
            ReachChange => Self::Constitutional,

            // Unknown SDK types — conservative
            Custom(_) => Self::Full,
        }
    }
}

/// Pre-computed trust context for a session.
#[derive(Debug, Clone)]
pub struct TrustContext {
    pub human_id: String,
    pub session_id: String,
    pub mastery_depth: f64,
    pub steward_standing: f64,
    pub relationship_density: f64,
    pub governance_health: f64,
    pub behavioral_trust: f64,
    pub intent_divergence: f64,
    pub composite_trust: f64,
    pub constitutional_layer: ConstitutionalLayer,
    pub community_id: Option<String>,
    pub family_id: Option<String>,
    pub declared_intent: Option<String>,
    pub computed_at: String,
}

// Signal weights — constitutional-layer-dependent in the future,
// flat weights for Sprint 1.
const W_MASTERY: f64 = 0.20;
const W_STEWARD: f64 = 0.20;
const W_RELATIONSHIP: f64 = 0.25;
const W_GOVERNANCE: f64 = 0.15;
const W_BEHAVIORAL: f64 = 0.20;
const INTENT_DIVERGENCE_PENALTY: f64 = 0.3;

impl TrustContext {
    /// Compute composite trust from raw signals.
    pub fn compute(signals: TrustSignals) -> Self {
        let raw = signals.mastery_depth * W_MASTERY
            + signals.steward_standing * W_STEWARD
            + signals.relationship_density * W_RELATIONSHIP
            + signals.governance_health * W_GOVERNANCE
            + signals.behavioral_trust * W_BEHAVIORAL;

        // Intent divergence penalizes composite trust
        let penalty = signals.intent_divergence * INTENT_DIVERGENCE_PENALTY;
        let composite = (raw - penalty).clamp(0.0, 1.0);

        Self {
            human_id: String::new(),
            session_id: String::new(),
            mastery_depth: signals.mastery_depth,
            steward_standing: signals.steward_standing,
            relationship_density: signals.relationship_density,
            governance_health: signals.governance_health,
            behavioral_trust: signals.behavioral_trust,
            intent_divergence: signals.intent_divergence,
            composite_trust: composite,
            constitutional_layer: ConstitutionalLayer::Individual,
            community_id: None,
            family_id: None,
            declared_intent: None,
            computed_at: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_context_high_trust() {
        let ctx = TrustContext::compute(TrustSignals {
            mastery_depth: 0.8,
            steward_standing: 0.7,
            relationship_density: 0.9,
            governance_health: 1.0,
            behavioral_trust: 0.85,
            intent_divergence: 0.0,
        });
        assert!(ctx.composite_trust > 0.7);
        assert!(ctx.composite_trust <= 1.0);
    }

    #[test]
    fn trust_context_low_trust() {
        let ctx = TrustContext::compute(TrustSignals {
            mastery_depth: 0.1,
            steward_standing: 0.0,
            relationship_density: 0.05,
            governance_health: 0.5,
            behavioral_trust: 0.3,
            intent_divergence: 0.0,
        });
        assert!(ctx.composite_trust < 0.3);
    }

    #[test]
    fn mastery_update_always_none() {
        let high_trust = TrustContext::compute(TrustSignals {
            mastery_depth: 0.9,
            steward_standing: 0.9,
            relationship_density: 0.9,
            governance_health: 1.0,
            behavioral_trust: 0.9,
            intent_divergence: 0.0,
        });
        assert_eq!(
            InferenceTier::classify(MutationType::MasteryUpdate, &high_trust),
            InferenceTier::None
        );
    }

    #[test]
    fn comment_high_trust_is_light() {
        let high_trust = TrustContext::compute(TrustSignals {
            mastery_depth: 0.8,
            steward_standing: 0.7,
            relationship_density: 0.9,
            governance_health: 1.0,
            behavioral_trust: 0.85,
            intent_divergence: 0.0,
        });
        assert_eq!(
            InferenceTier::classify(MutationType::Comment, &high_trust),
            InferenceTier::Light
        );
    }

    #[test]
    fn comment_low_trust_is_full() {
        let low_trust = TrustContext::compute(TrustSignals {
            mastery_depth: 0.1,
            steward_standing: 0.0,
            relationship_density: 0.05,
            governance_health: 0.5,
            behavioral_trust: 0.3,
            intent_divergence: 0.0,
        });
        assert_eq!(
            InferenceTier::classify(MutationType::Comment, &low_trust),
            InferenceTier::Full
        );
    }

    #[test]
    fn governance_vote_is_at_least_full() {
        let high_trust = TrustContext::compute(TrustSignals {
            mastery_depth: 0.9,
            steward_standing: 0.9,
            relationship_density: 0.9,
            governance_health: 1.0,
            behavioral_trust: 0.9,
            intent_divergence: 0.0,
        });
        let tier = InferenceTier::classify(MutationType::GovernanceVote, &high_trust);
        assert!(tier == InferenceTier::Full || tier == InferenceTier::Constitutional);
    }

    #[test]
    fn reach_change_always_constitutional() {
        let high_trust = TrustContext::compute(TrustSignals {
            mastery_depth: 0.9,
            steward_standing: 0.9,
            relationship_density: 0.9,
            governance_health: 1.0,
            behavioral_trust: 0.9,
            intent_divergence: 0.0,
        });
        assert_eq!(
            InferenceTier::classify(MutationType::ReachChange, &high_trust),
            InferenceTier::Constitutional
        );
    }

    #[test]
    fn intent_divergence_escalates_tier() {
        let diverged = TrustContext::compute(TrustSignals {
            mastery_depth: 0.8,
            steward_standing: 0.7,
            relationship_density: 0.9,
            governance_health: 1.0,
            behavioral_trust: 0.85,
            intent_divergence: 0.8,
        });
        // Comment would normally be Light for high trust, but divergence escalates
        let tier = InferenceTier::classify(MutationType::Comment, &diverged);
        assert!(tier == InferenceTier::Full || tier == InferenceTier::Constitutional);
    }

    #[test]
    fn intent_divergence_lowers_trust() {
        let base = TrustContext::compute(TrustSignals {
            mastery_depth: 0.8,
            steward_standing: 0.7,
            relationship_density: 0.9,
            governance_health: 1.0,
            behavioral_trust: 0.85,
            intent_divergence: 0.0,
        });
        let diverged = TrustContext::compute(TrustSignals {
            mastery_depth: 0.8,
            steward_standing: 0.7,
            relationship_density: 0.9,
            governance_health: 1.0,
            behavioral_trust: 0.85,
            intent_divergence: 0.8,
        });
        assert!(diverged.composite_trust < base.composite_trust);
    }
}
