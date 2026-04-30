//! Standing — agent property in the EPR graph substrate.
//!
//! Phase 3 introduces standing-aware code paths with a placeholder signal.
//! Phase 3.5 lights up the gradient via FeedbackSignal back-prop and
//! AttentionTending filter aggregation.
//!
//! See: genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md §4

use serde::{Deserialize, Serialize};

/// Continuous standing signal for an agent in the network.
///
/// Standing is a graph-derived view, not a stored score. This enum is the
/// shape that gradient-relevant code paths consume; the actual computation
/// is deferred to Phase 3.5 substrate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Standing {
    /// Phase 3 placeholder — signal not yet computed.
    /// Floor protections still apply; gradient-modulated paths fall back to
    /// safe defaults (e.g. cache-priority neutral, full validation, full
    /// schemaRef depth).
    Unknown,
    /// Phase 3.5+ — computed from attestation/correction/restitution
    /// subgraph through the evaluator's constitutional manifests.
    Computed { score: StandingScore },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StandingScore {
    Floor, // new-voice baseline, vulnerable-class, recent debit
    Low,
    Neutral,
    High,
    Trusted, // long-running good-faith stewardship
}

impl Standing {
    /// Phase 3 placeholder evaluator. Returns Unknown.
    /// Phase 3.5 replaces this with real graph traversal.
    pub fn evaluate_placeholder(_agent_pubkey: &[u8]) -> Self {
        Standing::Unknown
    }

    /// Modulation policy for cache priority. Returns priority weight in [0, 100].
    /// Unknown returns neutral (50). Phase 3.5 lights up the gradient.
    pub fn cache_priority_weight(self) -> u8 {
        match self {
            Standing::Unknown => 50,
            Standing::Computed { score } => match score {
                StandingScore::Floor => 25,
                StandingScore::Low => 35,
                StandingScore::Neutral => 50,
                StandingScore::High => 75,
                StandingScore::Trusted => 95,
            },
        }
    }

    /// SchemaRef walk depth limit. Floor protection: protocol-load-bearing
    /// types bypass this — see `floor_protections::is_protocol_load_bearing_schemaref`.
    pub fn schemaref_depth_limit(self) -> usize {
        match self {
            Standing::Unknown => 8, // Phase 3 default
            Standing::Computed { score } => match score {
                StandingScore::Floor | StandingScore::Low => 3,
                StandingScore::Neutral => 5,
                StandingScore::High | StandingScore::Trusted => 8,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_evaluator_returns_unknown() {
        let pk = [0u8; 32];
        assert_eq!(Standing::evaluate_placeholder(&pk), Standing::Unknown);
    }

    #[test]
    fn unknown_uses_neutral_cache_priority() {
        assert_eq!(Standing::Unknown.cache_priority_weight(), 50);
    }

    #[test]
    fn unknown_uses_default_schemaref_depth() {
        assert_eq!(Standing::Unknown.schemaref_depth_limit(), 8);
    }

    #[test]
    fn computed_floor_clips_schemaref_depth() {
        let standing = Standing::Computed {
            score: StandingScore::Floor,
        };
        assert_eq!(standing.schemaref_depth_limit(), 3);
    }

    #[test]
    fn computed_trusted_widens_cache_priority() {
        let standing = Standing::Computed {
            score: StandingScore::Trusted,
        };
        assert_eq!(standing.cache_priority_weight(), 95);
    }

    #[test]
    fn standing_serializes_round_trip() {
        let standing = Standing::Computed {
            score: StandingScore::High,
        };
        let json = serde_json::to_string(&standing).unwrap();
        let back: Standing = serde_json::from_str(&json).unwrap();
        assert_eq!(standing, back);
    }
}
