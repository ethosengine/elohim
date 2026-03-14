//! ElohimGate — mutation interceptor for protocol-level agent reasoning.
//!
//! Every mutation passes through the gate. The gate computes a TrustContext,
//! classifies an InferenceTier, and returns a GateResult that determines
//! how the mutation settles.

use constitution::{ConstitutionalLayer, ConstitutionalStack, PromptAssembler, StackContext};
use serde::Serialize;

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

/// Result of gate evaluation.
#[derive(Debug, Clone)]
pub enum GateResult {
    /// No inference needed, or inference not yet available (Sprint 1).
    PassThrough { tier: InferenceTier },

    /// Elohim evaluated. Mutation proceeds with adjustments.
    Enriched {
        tier: InferenceTier,
        reasoning: ElohimReasoning,
        adjusted_reach: Option<String>,
        observations: Vec<ObservationDraft>,
        session_intent_note: Option<String>,
    },

    /// Friction moment. Human must confirm to proceed.
    Pause {
        tier: InferenceTier,
        reasoning: ElohimReasoning,
        prompt: String,
        confirm_token: String,
    },

    /// Constitutional settlement. Appeal path exists.
    Settlement {
        tier: InferenceTier,
        reasoning: ElohimReasoning,
        boundary: String,
        appeal_path: Option<String>,
    },
}

impl GateResult {
    pub fn tier(&self) -> InferenceTier {
        match self {
            Self::PassThrough { tier } => *tier,
            Self::Enriched { tier, .. } => *tier,
            Self::Pause { tier, .. } => *tier,
            Self::Settlement { tier, .. } => *tier,
        }
    }
}

/// Placeholder for Sprint 2 — will carry LLM reasoning.
#[derive(Debug, Clone, Serialize)]
pub struct ElohimReasoning {
    pub primary_principle: String,
    pub interpretation: String,
    pub confidence: f64,
}

/// Draft observation to be stored in imagodei_observations.
#[derive(Debug, Clone)]
pub struct ObservationDraft {
    pub observation_type: String,
    pub content: String,
    pub structured_signals: Option<serde_json::Value>,
    pub trust_delta: f64,
    pub visibility_layer: String,
}

/// The gate itself. Sprint 1: classify-only skeleton.
pub struct ElohimGate {
    // Sprint 2: inference_router: Option<InferenceRouter>,
    // Sprint 3: observation_store: ObservationStore,
}

impl ElohimGate {
    /// Create a skeleton gate with no inference capability.
    pub fn new_skeleton() -> Self {
        Self {}
    }

    /// Evaluate a mutation against trust context.
    /// Sprint 1: always returns PassThrough with correct tier classification.
    pub fn evaluate(&self, mutation: MutationType, ctx: &TrustContext) -> GateResult {
        let tier = InferenceTier::classify(mutation, ctx);
        GateResult::PassThrough { tier }
    }

    /// Build constitutional prompt for a given trust context and mutation.
    pub fn build_constitutional_prompt(
        &self,
        ctx: &TrustContext,
        mutation: MutationType,
        tier: InferenceTier,
    ) -> String {
        let mut stack_ctx = StackContext::agent_only(&ctx.human_id);
        if let Some(ref family_id) = ctx.family_id {
            stack_ctx = stack_ctx.with_family(family_id);
        }
        if let Some(ref community_id) = ctx.community_id {
            stack_ctx = stack_ctx.with_community(community_id);
        }

        let stack = ConstitutionalStack::build_defaults(stack_ctx);

        let query = format!(
            "Evaluate {:?} mutation (tier: {:?}, composite_trust: {:.2}). {}",
            mutation,
            tier,
            ctx.composite_trust,
            ctx.declared_intent.as_deref().unwrap_or("No declared intent."),
        );

        match tier {
            InferenceTier::Constitutional => {
                PromptAssembler::build_reasoning_prompt(&stack, &query)
            }
            _ => PromptAssembler::build_system_prompt(&stack),
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
    fn gate_evaluate_returns_pass_through() {
        let gate = ElohimGate::new_skeleton();
        let ctx = TrustContext::compute(TrustSignals {
            mastery_depth: 0.5,
            steward_standing: 0.5,
            relationship_density: 0.5,
            governance_health: 1.0,
            behavioral_trust: 0.5,
            intent_divergence: 0.0,
        });
        let result = gate.evaluate(MutationType::Comment, &ctx);
        // Sprint 1: always PassThrough (no inference engine yet)
        let GateResult::PassThrough { tier } = result else {
            panic!("Expected PassThrough, got {:?}", result);
        };
        // But tier is correctly classified (composite=0.575 < 0.7 → Full for Comment)
        assert_eq!(tier, InferenceTier::Full);
    }

    #[test]
    fn prompt_assembly_light_tier() {
        let gate = ElohimGate::new_skeleton();
        let ctx = TrustContext::compute(TrustSignals {
            mastery_depth: 0.8,
            steward_standing: 0.7,
            relationship_density: 0.9,
            governance_health: 1.0,
            behavioral_trust: 0.85,
            intent_divergence: 0.0,
        });
        let prompt =
            gate.build_constitutional_prompt(&ctx, MutationType::Comment, InferenceTier::Light);
        assert!(!prompt.is_empty());
    }

    #[test]
    fn prompt_assembly_constitutional_tier() {
        let gate = ElohimGate::new_skeleton();
        let ctx = TrustContext::compute(TrustSignals {
            mastery_depth: 0.5,
            steward_standing: 0.5,
            relationship_density: 0.5,
            governance_health: 0.5,
            behavioral_trust: 0.5,
            intent_divergence: 0.0,
        });
        let prompt = gate.build_constitutional_prompt(
            &ctx,
            MutationType::ReachChange,
            InferenceTier::Constitutional,
        );
        assert!(!prompt.is_empty());
        // Constitutional tier uses reasoning prompt which includes the query
        assert!(prompt.contains("Evaluate"));
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
