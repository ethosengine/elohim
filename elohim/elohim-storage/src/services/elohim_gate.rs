//! ElohimGate — mutation interceptor for protocol-level agent reasoning.
//!
//! Every mutation passes through the gate. The gate computes a TrustContext,
//! classifies an InferenceTier, and returns a GateResult that determines
//! how the mutation settles.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use constitution::{ConstitutionalLayer, ConstitutionalStack, PromptAssembler, StackContext};
use serde::Serialize;

use super::inference_engine::{InferenceRequest, InferenceResult, RecommendedAction};
use super::inference_router::InferenceRouter;

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

/// Per-session cache entry for computed TrustContext
struct CacheEntry {
    context: TrustContext,
    cached_at: Instant,
}

/// Session-scoped cache for TrustContext with TTL-based expiry.
/// Avoids recomputing trust signals on every mutation within a session.
pub struct TrustContextCache {
    entries: RwLock<HashMap<String, CacheEntry>>,
    ttl: Duration,
}

impl TrustContextCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    /// Get cached TrustContext if within TTL
    pub fn get(&self, session_id: &str) -> Option<TrustContext> {
        let entries = self.entries.read().ok()?;
        let entry = entries.get(session_id)?;
        if entry.cached_at.elapsed() < self.ttl {
            Some(entry.context.clone())
        } else {
            None
        }
    }

    /// Cache a computed TrustContext for a session
    pub fn set(&self, session_id: &str, context: TrustContext) {
        if let Ok(mut entries) = self.entries.write() {
            entries.insert(
                session_id.to_string(),
                CacheEntry {
                    context,
                    cached_at: Instant::now(),
                },
            );
        }
    }

    /// Invalidate a specific session's cache (e.g., on session intent change)
    pub fn invalidate(&self, session_id: &str) {
        if let Ok(mut entries) = self.entries.write() {
            entries.remove(session_id);
        }
    }

    /// Invalidate all cached entries (e.g., on system-wide trust recalibration)
    pub fn invalidate_all(&self) {
        if let Ok(mut entries) = self.entries.write() {
            entries.clear();
        }
    }
}

/// Cached pending confirmation — stores original mutation context
/// so it can be validated when the user confirms a paused mutation.
pub struct PendingConfirmation {
    pub mutation: MutationType,
    pub mutation_content: serde_json::Value,
    pub human_id: String,
    pub created_at: Instant,
}

/// Short-lived cache for pending confirmations (Pause flow).
/// Token -> PendingConfirmation, with TTL expiry.
pub struct PendingConfirmationCache {
    entries: RwLock<HashMap<String, PendingConfirmation>>,
    ttl: Duration,
}

impl PendingConfirmationCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    /// Store a pending confirmation for a token.
    pub fn store(&self, token: &str, confirmation: PendingConfirmation) {
        if let Ok(mut entries) = self.entries.write() {
            entries.insert(token.to_string(), confirmation);
        }
    }

    /// Take (remove and return) a pending confirmation if within TTL.
    /// Returns None if token not found or expired.
    pub fn take(&self, token: &str) -> Option<PendingConfirmation> {
        let mut entries = self.entries.write().ok()?;
        let entry = entries.remove(token)?;
        if entry.created_at.elapsed() < self.ttl {
            Some(entry)
        } else {
            None
        }
    }
}

/// The gate itself. Sprint 2: routes to InferenceRouter when available.
pub struct ElohimGate {
    router: Option<Arc<InferenceRouter>>,
    trust_cache: TrustContextCache,
    pending_confirmations: PendingConfirmationCache,
}

impl ElohimGate {
    /// Create a skeleton gate with no inference capability.
    pub fn new_skeleton() -> Self {
        Self {
            router: None,
            trust_cache: TrustContextCache::new(300),
            pending_confirmations: PendingConfirmationCache::new(300),
        }
    }

    /// Create a gate backed by an inference router.
    pub fn new(router: Arc<InferenceRouter>) -> Self {
        Self {
            router: Some(router),
            trust_cache: TrustContextCache::new(300),
            pending_confirmations: PendingConfirmationCache::new(300),
        }
    }

    /// Access the trust context cache.
    pub fn trust_cache(&self) -> &TrustContextCache {
        &self.trust_cache
    }

    /// Access the pending confirmations cache.
    pub fn pending_confirmations(&self) -> &PendingConfirmationCache {
        &self.pending_confirmations
    }

    /// Evaluate a mutation against trust context.
    /// Routes to inference when a router is available and tier is not None.
    /// Falls back to PassThrough on missing router or inference failure.
    pub async fn evaluate(
        &self,
        mutation: MutationType,
        ctx: &TrustContext,
        mutation_content: serde_json::Value,
    ) -> GateResult {
        let tier = InferenceTier::classify(mutation, ctx);

        // None tier: always pass through
        if tier == InferenceTier::None {
            return GateResult::PassThrough { tier };
        }

        // No router: fall back to PassThrough
        let router = match &self.router {
            Some(r) => r,
            None => return GateResult::PassThrough { tier },
        };

        // Build prompt and request
        let prompt = self.build_constitutional_prompt(ctx, mutation, tier);
        let request = InferenceRequest {
            tier,
            mutation_type: mutation,
            trust_context: ctx.clone(),
            mutation_content,
            constitutional_prompt: prompt,
        };

        // Route to inference
        match router.route(request).await {
            Ok(result) => self.map_inference_result(tier, result),
            Err(e) => {
                tracing::warn!(error = %e, "Inference failed, falling back to PassThrough");
                GateResult::PassThrough { tier }
            }
        }
    }

    /// Map an inference result to the appropriate GateResult variant.
    fn map_inference_result(&self, tier: InferenceTier, result: InferenceResult) -> GateResult {
        match result.recommended_action {
            RecommendedAction::Proceed => GateResult::Enriched {
                tier,
                reasoning: result.reasoning,
                adjusted_reach: None,
                observations: result.observations,
                session_intent_note: None,
            },
            RecommendedAction::ProceedWithReachAdjustment(reach) => GateResult::Enriched {
                tier,
                reasoning: result.reasoning,
                adjusted_reach: Some(reach),
                observations: result.observations,
                session_intent_note: None,
            },
            RecommendedAction::PauseForConfirmation(prompt) => GateResult::Pause {
                tier,
                reasoning: result.reasoning,
                prompt,
                confirm_token: uuid::Uuid::new_v4().to_string(),
            },
            RecommendedAction::Settle(boundary) => GateResult::Settlement {
                tier,
                reasoning: result.reasoning,
                boundary,
                appeal_path: Some("/api/v1/governance/appeal".to_string()),
            },
        }
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
            ctx.declared_intent
                .as_deref()
                .unwrap_or("No declared intent."),
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

    #[tokio::test]
    async fn gate_evaluate_returns_pass_through() {
        let gate = ElohimGate::new_skeleton();
        let ctx = TrustContext::compute(TrustSignals {
            mastery_depth: 0.5,
            steward_standing: 0.5,
            relationship_density: 0.5,
            governance_health: 1.0,
            behavioral_trust: 0.5,
            intent_divergence: 0.0,
        });
        let result = gate
            .evaluate(MutationType::Comment, &ctx, serde_json::json!({}))
            .await;
        // No router: always PassThrough
        let GateResult::PassThrough { tier } = result else {
            panic!("Expected PassThrough, got {:?}", result);
        };
        // But tier is correctly classified (composite=0.575 < 0.7 → Full for Comment)
        assert_eq!(tier, InferenceTier::Full);
    }

    #[tokio::test]
    async fn gate_with_mock_returns_enriched() {
        use crate::services::inference_engine::MockEngine;
        use crate::services::inference_router::InferenceRouter;

        let engine = Arc::new(MockEngine::always_proceed());
        let router = Arc::new(InferenceRouter::new(vec![
            engine as Arc<dyn crate::services::inference_engine::InferenceEngine>,
        ]));
        let gate = ElohimGate::new(router);

        let ctx = TrustContext::compute(TrustSignals {
            mastery_depth: 0.8,
            steward_standing: 0.7,
            relationship_density: 0.9,
            governance_health: 1.0,
            behavioral_trust: 0.85,
            intent_divergence: 0.0,
        });
        let result = gate
            .evaluate(MutationType::Comment, &ctx, serde_json::json!({}))
            .await;
        assert!(matches!(result, GateResult::Enriched { .. }));
    }

    #[tokio::test]
    async fn gate_without_router_returns_passthrough() {
        let gate = ElohimGate::new_skeleton();
        let ctx = TrustContext::compute(TrustSignals {
            mastery_depth: 0.5,
            steward_standing: 0.5,
            relationship_density: 0.5,
            governance_health: 0.5,
            behavioral_trust: 0.5,
            intent_divergence: 0.0,
        });
        let result = gate
            .evaluate(MutationType::Comment, &ctx, serde_json::json!({}))
            .await;
        assert!(matches!(result, GateResult::PassThrough { .. }));
    }

    #[tokio::test]
    async fn gate_none_tier_always_passthrough() {
        use crate::services::inference_engine::MockEngine;
        use crate::services::inference_router::InferenceRouter;

        let engine = Arc::new(MockEngine::always_proceed());
        let router = Arc::new(InferenceRouter::new(vec![
            engine as Arc<dyn crate::services::inference_engine::InferenceEngine>,
        ]));
        let gate = ElohimGate::new(router);

        let ctx = TrustContext::compute(TrustSignals {
            mastery_depth: 0.5,
            steward_standing: 0.5,
            relationship_density: 0.5,
            governance_health: 0.5,
            behavioral_trust: 0.5,
            intent_divergence: 0.0,
        });
        let result = gate
            .evaluate(MutationType::MasteryUpdate, &ctx, serde_json::json!({}))
            .await;
        assert!(matches!(result, GateResult::PassThrough { .. }));
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

    #[test]
    fn cache_hit_returns_context() {
        let cache = TrustContextCache::new(300);
        let ctx = TrustContext::compute(TrustSignals {
            mastery_depth: 0.7,
            steward_standing: 0.8,
            relationship_density: 0.6,
            governance_health: 0.5,
            behavioral_trust: 0.9,
            intent_divergence: 0.0,
        });
        cache.set("session-1", ctx.clone());
        let cached = cache.get("session-1");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().composite_trust, ctx.composite_trust);
    }

    #[test]
    fn cache_miss_returns_none() {
        let cache = TrustContextCache::new(300);
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn invalidation_removes_entry() {
        let cache = TrustContextCache::new(300);
        let ctx = TrustContext::compute(TrustSignals {
            mastery_depth: 0.5,
            steward_standing: 0.5,
            relationship_density: 0.5,
            governance_health: 0.5,
            behavioral_trust: 0.5,
            intent_divergence: 0.0,
        });
        cache.set("session-1", ctx);
        cache.invalidate("session-1");
        assert!(cache.get("session-1").is_none());
    }

    #[test]
    fn pending_confirmation_store_and_take() {
        let cache = PendingConfirmationCache::new(300);
        cache.store(
            "token-1",
            PendingConfirmation {
                mutation: MutationType::CurationEvent,
                mutation_content: serde_json::json!({"test": true}),
                human_id: "human-1".to_string(),
                created_at: Instant::now(),
            },
        );
        let taken = cache.take("token-1");
        assert!(taken.is_some());
        assert_eq!(taken.unwrap().human_id, "human-1");
    }

    #[test]
    fn pending_confirmation_take_removes_entry() {
        let cache = PendingConfirmationCache::new(300);
        cache.store(
            "token-1",
            PendingConfirmation {
                mutation: MutationType::CurationEvent,
                mutation_content: serde_json::json!({}),
                human_id: "human-1".to_string(),
                created_at: Instant::now(),
            },
        );
        let _ = cache.take("token-1");
        assert!(cache.take("token-1").is_none()); // second take returns None
    }

    #[test]
    fn pending_confirmation_invalid_token_returns_none() {
        let cache = PendingConfirmationCache::new(300);
        assert!(cache.take("nonexistent").is_none());
    }

    #[test]
    fn invalidate_all_clears_cache() {
        let cache = TrustContextCache::new(300);
        let ctx = TrustContext::compute(TrustSignals {
            mastery_depth: 0.5,
            steward_standing: 0.5,
            relationship_density: 0.5,
            governance_health: 0.5,
            behavioral_trust: 0.5,
            intent_divergence: 0.0,
        });
        cache.set("session-1", ctx.clone());
        cache.set("session-2", ctx);
        cache.invalidate_all();
        assert!(cache.get("session-1").is_none());
        assert!(cache.get("session-2").is_none());
    }
}
