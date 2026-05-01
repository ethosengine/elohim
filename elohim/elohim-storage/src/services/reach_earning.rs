//! Reach-earning gate — Phase 3.5 author-side compose substrate.
//!
//! Pure deterministic evaluator. Returns ReachVerdict; never persists. The
//! verdict shape is forward-compatible with a future elohim-mediated discernment
//! layer that consumes Pending and produces sponsor suggestions.
//!
//! See: genesis/docs/superpowers/specs/2026-05-01-light-up-the-graph-design.md §Components::ReachVerdict

use diesel::SqliteConnection;
use serde::{Deserialize, Serialize};

use crate::services::epr_kind::Reach;
use crate::services::manifest_registry::{ManifestRegistry, UnknownTreatment};
use crate::services::standing::{Standing, StandingScore};

/// One of the five constitutional floor classes (brainstorm §2.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FloorClass {
    CidTargetedLookup,
    NewVoiceBaseline,
    VulnerableClassElevation,
    LocalRelationshipReach,
    ConstitutionalFloorSignatures,
}

/// Snapshot evidence attached to a verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandingEvidence {
    pub standing: Standing,
}

/// Reason a compose attempt was blocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockReason {
    QuarantineActive,
    FloorBreach {
        class: FloorClass,
    },
    StandingBelowThreshold,
    /// Reach value not in the manifest's reachThresholds map — fail-closed.
    UnknownReach,
}

/// Reason a compose attempt is pending (gate defers to discernment layer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PendingReason {
    UnknownAuthorAtNonFloorReach,
    NewVoiceWithoutSponsor,
}

/// Verdict returned by [`evaluate`].
///
/// `Pending` collapses to `Blocked` for substrate-only callers in this sprint.
/// The verdict shape is forward-compatible for a future elohim-mediated
/// discernment layer that may consume `Pending` and produce sponsor suggestions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReachVerdict {
    Allowed {
        floor_class_match: Option<FloorClass>,
        evidence: StandingEvidence,
    },
    Blocked {
        reason: BlockReason,
        evidence: StandingEvidence,
    },
    Pending {
        reason: PendingReason,
        evidence: StandingEvidence,
    },
}

/// Pure substrate evaluator. Does not persist; returns ephemeral verdict.
///
/// # Parameters
/// - `local_agent` — the evaluating peer's pubkey bytes (used to query the
///   local standing_view projection for the (local_agent, author) pair)
/// - `author` — the composing agent's pubkey bytes
/// - `requested_reach` — the scope the author wants to publish at
/// - `conn` — SQLite connection for standing_view lookup
/// - `registry` — loaded ManifestRegistry (may be empty/default)
pub fn evaluate(
    local_agent: &[u8],
    author: &[u8],
    requested_reach: Reach,
    conn: &mut SqliteConnection,
    registry: &ManifestRegistry,
) -> ReachVerdict {
    // 1. Floor class allow: cid-targeted-lookup, local-relationship-reach
    if requested_reach.is_floor_allowed() {
        let floor_class = match requested_reach {
            Reach::Personal | Reach::Intimate => FloorClass::CidTargetedLookup,
            Reach::Household | Reach::Neighborhood => FloorClass::LocalRelationshipReach,
            _ => FloorClass::CidTargetedLookup,
        };
        return ReachVerdict::Allowed {
            floor_class_match: Some(floor_class),
            evidence: StandingEvidence {
                standing: Standing::Unknown,
            },
        };
    }

    // 2. Quarantine check
    if registry.is_quarantined(author) {
        return ReachVerdict::Blocked {
            reason: BlockReason::QuarantineActive,
            evidence: StandingEvidence {
                standing: Standing::Unknown,
            },
        };
    }

    // 3. Vulnerable-class lift
    let lift = registry.vulnerable_class_lift(author);

    // 4. Standing evaluation (evaluator = local_agent)
    let raw_standing = Standing::evaluate(local_agent, author, conn);
    let effective = raw_standing.with_lift(lift);
    let evidence = StandingEvidence {
        standing: raw_standing,
    };

    // 5. Required threshold from manifest (with safe-by-default fallback)
    let required = match registry.reach_threshold(requested_reach.as_kebab()) {
        Some(t) => t,
        None => {
            // Manifest missing entry — use safe-by-default conservative table.
            match requested_reach {
                Reach::Public => "high".to_string(),
                _ => "neutral".to_string(),
            }
        }
    };

    // 6. Apply UnknownTreatment policy
    match (effective, required.as_str()) {
        (Standing::Unknown, _) => match registry.unknown_treatment() {
            UnknownTreatment::Conservative => ReachVerdict::Pending {
                reason: PendingReason::UnknownAuthorAtNonFloorReach,
                evidence,
            },
            UnknownTreatment::NewVoiceBaseline => {
                let baseline = registry
                    .new_voice_baseline()
                    .unwrap_or(StandingScore::Floor);
                evaluate_with_score(baseline, &required, evidence)
            }
            UnknownTreatment::Neutral => {
                evaluate_with_score(StandingScore::Neutral, &required, evidence)
            }
        },
        (Standing::Computed { score }, threshold) => {
            evaluate_with_score(score, threshold, evidence)
        }
    }
}

fn evaluate_with_score(
    score: StandingScore,
    threshold: &str,
    evidence: StandingEvidence,
) -> ReachVerdict {
    if threshold == "any" {
        return ReachVerdict::Allowed {
            floor_class_match: None,
            evidence,
        };
    }
    let needed = match threshold {
        "floor" => StandingScore::Floor,
        "low" => StandingScore::Low,
        "neutral" => StandingScore::Neutral,
        "high" => StandingScore::High,
        "trusted" => StandingScore::Trusted,
        _ => {
            return ReachVerdict::Blocked {
                reason: BlockReason::UnknownReach,
                evidence,
            }
        }
    };
    if score_rank(score) >= score_rank(needed) {
        ReachVerdict::Allowed {
            floor_class_match: None,
            evidence,
        }
    } else {
        ReachVerdict::Blocked {
            reason: BlockReason::StandingBelowThreshold,
            evidence,
        }
    }
}

fn score_rank(s: StandingScore) -> u8 {
    match s {
        StandingScore::Floor => 0,
        StandingScore::Low => 1,
        StandingScore::Neutral => 2,
        StandingScore::High => 3,
        StandingScore::Trusted => 4,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::sqlite::SqliteConnection;

    fn test_pool() -> DbPool {
        let url = format!(
            "file:reach_earn_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn registry_with_full_policy() -> ManifestRegistry {
        let json = r#"{
            "manifestKind":"standing-policy","revision":1,
            "floor":{"classes":[]},
            "newVoiceBaseline":{"score":"floor","vulnerableClassLift":"low"},
            "debitWeights":{
                "squelch":{"advisory":0,"debit-soft":1,"debit-firm":3},
                "correction":{"advisory":0,"debit-soft":10,"debit-firm":20},
                "retraction":{"advisory":0,"debit-soft":-5,"debit-firm":-10},
                "quarantine":{"advisory":0,"debit-soft":12,"debit-firm":30},
                "vouch":{"advisory":0,"debit-soft":-3,"debit-firm":-8}
            },
            "unknownTreatment":{"default":"conservative","evidenceSources":[]},
            "reachThresholds":{
                "personal":"any","intimate":"any","household":"any","neighborhood":"any",
                "collective":"neutral","community":"neutral","district":"neutral","public":"high"
            }
        }"#;
        ManifestRegistry::from_payload_json(json).expect("parse")
    }

    #[test]
    fn floor_reach_always_allowed_unknown_author() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let r = registry_with_full_policy();
        let v = evaluate(&[0; 32], &[1; 32], Reach::Personal, &mut conn, &r);
        assert!(matches!(v, ReachVerdict::Allowed { .. }), "{v:?}");
    }

    #[test]
    fn unknown_author_at_public_with_conservative_treatment_pending() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let r = registry_with_full_policy();
        let v = evaluate(&[0; 32], &[1; 32], Reach::Public, &mut conn, &r);
        assert!(
            matches!(
                v,
                ReachVerdict::Pending {
                    reason: PendingReason::UnknownAuthorAtNonFloorReach,
                    ..
                }
            ),
            "{v:?}"
        );
    }

    #[test]
    fn unknown_author_at_public_with_neutral_treatment_blocked_below_high() {
        let json = r#"{"manifestKind":"standing-policy","revision":1,
            "floor":{"classes":[]},
            "newVoiceBaseline":{"score":"floor","vulnerableClassLift":"low"},
            "debitWeights":{"squelch":{"advisory":0,"debit-soft":1,"debit-firm":3},"correction":{"advisory":0,"debit-soft":10,"debit-firm":20},"retraction":{"advisory":0,"debit-soft":-5,"debit-firm":-10},"quarantine":{"advisory":0,"debit-soft":12,"debit-firm":30},"vouch":{"advisory":0,"debit-soft":-3,"debit-firm":-8}},
            "unknownTreatment":{"default":"neutral","evidenceSources":[]},
            "reachThresholds":{"public":"high"}
        }"#;
        let r = ManifestRegistry::from_payload_json(json).unwrap();
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        // Unknown standing → Neutral treatment → score=Neutral; threshold=high → Neutral < High → Blocked
        let v = evaluate(&[0; 32], &[1; 32], Reach::Public, &mut conn, &r);
        assert!(
            matches!(
                v,
                ReachVerdict::Blocked {
                    reason: BlockReason::StandingBelowThreshold,
                    ..
                }
            ),
            "{v:?}"
        );
    }

    #[test]
    fn computed_high_at_public_allowed() {
        use crate::p2p::feedback_signal::{FeedbackSignal, SignalKind, StandingImpact, VouchKind};
        use crate::services::standing_projector::{project_signal, ManifestDebitWeightPolicy};
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let r = registry_with_full_policy();
        let policy = ManifestDebitWeightPolicy::from_registry(&r);
        let evaluator = [0u8; 32];
        let author_bytes = [0xABu8; 32];
        let author_b64 = BASE64.encode(author_bytes);

        // Two vouch/debit-firm signals: weight = -8 each → sum = -16 → Trusted
        for _ in 0..2 {
            let sig = FeedbackSignal {
                target_cid: format!("bafyreitarget{}", uuid::Uuid::new_v4().simple()),
                signal_kind: SignalKind::Vouch,
                vouch_kind: Some(VouchKind::AcceptCorrection),
                evidence_cid: None,
                standing_impact: StandingImpact::DebitFirm,
                signed_by: author_b64.clone(),
                signature: BASE64.encode([0xFFu8; 64]),
            };
            project_signal(&mut conn, &policy, &evaluator, &sig, "bafyreimanifest").unwrap();
        }

        let v = evaluate(&evaluator, &author_bytes, Reach::Public, &mut conn, &r);
        assert!(matches!(v, ReachVerdict::Allowed { .. }), "{v:?}");
    }

    #[test]
    fn manifest_absent_falls_back_to_conservative_table() {
        let r = ManifestRegistry::default();
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        // Unknown standing + Conservative treatment → Pending
        let v = evaluate(&[0; 32], &[1; 32], Reach::Public, &mut conn, &r);
        assert!(matches!(v, ReachVerdict::Pending { .. }), "{v:?}");
    }

    #[test]
    fn floor_reach_household_returns_local_relationship_reach_class() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let r = registry_with_full_policy();
        let v = evaluate(&[0; 32], &[1; 32], Reach::Household, &mut conn, &r);
        if let ReachVerdict::Allowed {
            floor_class_match: Some(class),
            ..
        } = v
        {
            assert_eq!(class, FloorClass::LocalRelationshipReach);
        } else {
            panic!("expected Allowed with LocalRelationshipReach, got {v:?}");
        }
    }
}
