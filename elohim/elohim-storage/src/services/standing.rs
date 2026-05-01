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
    ///
    /// Deprecated in favour of [`Standing::evaluate`], which reads from the
    /// standing_view projection table written by `standing_projector::project_signal`.
    #[deprecated(
        since = "0.0.0",
        note = "Phase 3.5: use Standing::evaluate(evaluator, subject, conn)"
    )]
    pub fn evaluate_placeholder(_agent_pubkey: &[u8]) -> Self {
        Standing::Unknown
    }

    /// Phase 3.5 evaluator. Reads from the standing_view projection table.
    ///
    /// Returns `Standing::Unknown` if no projection exists yet (cold-start / no
    /// FeedbackSignals for this (evaluator, subject) pair). NOT a stored authoritative
    /// score — the table is recomputable from the FeedbackSignal subgraph at any time.
    ///
    /// See: genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md §4.2
    pub fn evaluate(
        evaluator: &[u8],
        subject: &[u8],
        conn: &mut diesel::sqlite::SqliteConnection,
    ) -> Self {
        use crate::db::standing_view::fetch;
        use crate::services::standing_projector::deserialize_score;
        match fetch(conn, evaluator, subject) {
            Ok(Some(row)) => match deserialize_score(&row.score) {
                Some(score) => Standing::Computed { score },
                None => {
                    tracing::warn!(
                        score = %row.score,
                        evaluator_len = evaluator.len(),
                        subject_len = subject.len(),
                        "unrecognized standing score in DB row, defaulting to Unknown"
                    );
                    Standing::Unknown
                }
            },
            _ => Standing::Unknown,
        }
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

    #[allow(deprecated)]
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

    // -----------------------------------------------------------------------
    // Phase 3.5 evaluate() tests — require a real SQLite in-memory pool
    // -----------------------------------------------------------------------

    fn test_pool() -> crate::db::DbPool {
        use crate::db::run_migrations;
        use diesel::r2d2::{ConnectionManager, Pool};
        use diesel::sqlite::SqliteConnection;
        let url = format!(
            "file:standing_eval_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    #[test]
    fn evaluate_falls_back_to_unknown_for_no_projection() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let eval = [0xE0u8; 32];
        let subj = [0xABu8; 32];
        let result = Standing::evaluate(&eval, &subj, &mut conn);
        assert_eq!(result, Standing::Unknown, "cold-start should be Unknown");
    }

    #[test]
    fn evaluate_returns_computed_after_projection() {
        use crate::p2p::feedback_signal::{FeedbackSignal, SignalKind, StandingImpact};
        use crate::services::standing_projector::{project_signal, DefaultDebitWeightPolicy};
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let eval = [0xE0u8; 32];
        let subj_bytes = [0xABu8; 32];
        let subj_b64 = BASE64.encode(subj_bytes);

        // Apply a squelch advisory (sum=0, score=Neutral).
        let signal = FeedbackSignal {
            target_cid: "bafyreitarget".to_string(),
            signal_kind: SignalKind::Squelch,
            vouch_kind: None,
            evidence_cid: None,
            standing_impact: StandingImpact::Advisory,
            signed_by: subj_b64,
            signature: BASE64.encode([0xFFu8; 64]),
        };
        project_signal(
            &mut conn,
            &DefaultDebitWeightPolicy,
            &eval,
            &signal,
            "bafyreimanifest",
        )
        .expect("project");

        let result = Standing::evaluate(&eval, &subj_bytes, &mut conn);
        assert_eq!(
            result,
            Standing::Computed {
                score: StandingScore::Neutral
            },
            "after advisory projection should be Computed(Neutral)"
        );
    }
}
