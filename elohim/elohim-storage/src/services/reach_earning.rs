//! Reach-earning gate — Phase 3.5 author-side compose substrate.
//!
//! Pure deterministic evaluator. Returns ReachVerdict; never persists. The
//! verdict shape is forward-compatible with a future elohim-mediated discernment
//! layer that consumes Pending and produces sponsor suggestions.
//!
//! See: genesis/docs/superpowers/specs/2026-05-01-light-up-the-graph-design.md §Components::ReachVerdict

use diesel::SqliteConnection;
use serde::{Deserialize, Serialize};

use crate::services::epr_kind::{Reach, ReachStandingExt};
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

/// Reach joins the verdict spine. A [`ReachVerdict`] is one axis's answer, so it
/// projects onto the protocol-owned [`elohim_epr::verdict::Verdict`] shape:
/// `Allowed → Permit`, `Blocked → Refuse`, `Pending → Refer`.
///
/// **Ceiling law.** `Pending` is a not-yet-decidable question for the discernment
/// layer — it maps to [`Decision::Refer`] (escalate to the named layer), NEVER to
/// [`Decision::Refuse`] (which would silently swallow a question a human was owed).
/// The [`StandingEvidence`] rides into the witness as `observed` values (positive
/// evidence), so the verdict carries the standing it was computed from.
impl ReachVerdict {
    /// Narrow onto the verdict spine, citing the policy that produced this answer.
    ///
    /// `policy_ref` should be [`ManifestRegistry::standing_policy_ref`] — the CID of
    /// the standing-policy manifest that parameterized [`evaluate`]. Reach is a
    /// closed-verdict axis (`epr:registry:axis:reach`), and a closed verdict owes its
    /// counterparty the answer to *which policy decided this*: with the CID the
    /// receiver fetches the exact bytes and checks our reasoning without sharing our
    /// code. A name or a revision integer cannot do that — two peers can hold
    /// different bytes under the same `revision: 1`.
    ///
    /// `None` is honest, not a default: it says this verdict came from a policy that
    /// is not content-addressed, so no citation can be backed.
    pub fn into_verdict(self, policy_ref: Option<String>) -> elohim_epr::verdict::Verdict {
        use elohim_epr::verdict::{
            CheckOutcome, CheckWitness, Decision, ReferQuestion, ReferReason, Verdict, Witness,
        };

        let axis = "reach".to_string();
        match self {
            ReachVerdict::Allowed {
                floor_class_match,
                evidence,
            } => {
                let checks = vec![
                    CheckWitness {
                        check_id: "reach-standing".into(),
                        outcome: CheckOutcome::Passed,
                        summary: "author standing meets the requested reach".into(),
                        observed: serde_json::to_value(evidence.standing).ok(),
                    },
                    CheckWitness {
                        check_id: "floor-class".into(),
                        outcome: CheckOutcome::Passed,
                        summary: match floor_class_match {
                            Some(_) => "allowed under a constitutional floor class".into(),
                            None => "allowed on earned standing".into(),
                        },
                        observed: serde_json::to_value(floor_class_match).ok(),
                    },
                ];
                Verdict {
                    axis,
                    subject: None,
                    decision: Decision::Permit,
                    witness: Witness { checks },
                    policy_ref,
                }
            }
            ReachVerdict::Blocked { reason, evidence } => {
                let checks = vec![
                    CheckWitness {
                        check_id: "reach-standing".into(),
                        outcome: CheckOutcome::Failed,
                        summary: format!("blocked: {reason:?}"),
                        observed: serde_json::to_value(reason).ok(),
                    },
                    CheckWitness {
                        check_id: "standing-evidence".into(),
                        outcome: CheckOutcome::Skipped,
                        summary: "standing snapshot at evaluation time".into(),
                        observed: serde_json::to_value(evidence.standing).ok(),
                    },
                ];
                Verdict {
                    axis,
                    subject: None,
                    decision: Decision::Refuse,
                    witness: Witness { checks },
                    policy_ref,
                }
            }
            ReachVerdict::Pending { reason, evidence } => {
                // Ceiling law: escalate, never collapse. UnknownAuthor lacks the standing
                // to earn the reach (InsufficientAuthority); a new voice is an unseen
                // participant the layer must frame (NovelSituation).
                let refer_reason = match reason {
                    PendingReason::UnknownAuthorAtNonFloorReach => {
                        ReferReason::InsufficientAuthority
                    }
                    PendingReason::NewVoiceWithoutSponsor => ReferReason::NovelSituation,
                };
                let checks = vec![CheckWitness {
                    check_id: "standing-evidence".into(),
                    outcome: CheckOutcome::Skipped,
                    summary: "standing snapshot at evaluation time".into(),
                    observed: serde_json::to_value(evidence.standing).ok(),
                }];
                Verdict {
                    axis,
                    subject: None,
                    decision: Decision::Refer(ReferQuestion {
                        layer: "community".into(),
                        reason: refer_reason,
                        note: Some(format!("{reason:?}")),
                    }),
                    witness: Witness { checks },
                    policy_ref,
                }
            }
        }
    }
}

/// Convenience conversion that emits an **uncitable** verdict (`policy_ref: None`).
///
/// Kept so the spine conversion stays available where no registry is in hand (tests,
/// pure-shape assertions). Production paths that hold a [`ManifestRegistry`] should
/// call [`ReachVerdict::into_verdict`] with [`ManifestRegistry::standing_policy_ref`]
/// instead — a verdict a counterparty cannot trace to its policy is unauditable, and
/// this impl cannot supply the trace.
impl From<ReachVerdict> for elohim_epr::verdict::Verdict {
    fn from(v: ReachVerdict) -> Self {
        v.into_verdict(None)
    }
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
            Reach::Private | Reach::SelfScope | Reach::Intimate => FloorClass::CidTargetedLookup,
            Reach::Trusted | Reach::Familiar => FloorClass::LocalRelationshipReach,
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
    let required = match registry.reach_threshold(requested_reach.as_manifest_key()) {
        Some(t) => t,
        None => {
            // Manifest missing entry — use safe-by-default conservative table.
            // Both top rungs of the canonical vocabulary (Public, Commons) are
            // the "maximally open" case the old kebab-8 single top rung meant.
            match requested_reach {
                Reach::Public | Reach::Commons => "high".to_string(),
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
                "private":"any","self":"any","intimate":"any","trusted":"any","familiar":"any",
                "community":"neutral","public":"high","commons":"high"
            }
        }"#;
        ManifestRegistry::from_payload_json(json).expect("parse")
    }

    #[test]
    fn floor_reach_always_allowed_unknown_author() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let r = registry_with_full_policy();
        let v = evaluate(&[0; 32], &[1; 32], Reach::SelfScope, &mut conn, &r);
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
    fn floor_reach_trusted_returns_local_relationship_reach_class() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let r = registry_with_full_policy();
        let v = evaluate(&[0; 32], &[1; 32], Reach::Trusted, &mut conn, &r);
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

    #[test]
    fn every_reach_verdict_maps_to_spine_and_pending_never_refuses() {
        use elohim_epr::verdict::{Decision, Verdict};

        let evidence = StandingEvidence {
            standing: Standing::Unknown,
        };

        // Allowed → Permit
        let allowed: Verdict = ReachVerdict::Allowed {
            floor_class_match: Some(FloorClass::CidTargetedLookup),
            evidence: evidence.clone(),
        }
        .into();
        assert_eq!(allowed.axis, "reach");
        assert!(matches!(allowed.decision, Decision::Permit), "{allowed:?}");

        // Blocked → Refuse
        let blocked: Verdict = ReachVerdict::Blocked {
            reason: BlockReason::StandingBelowThreshold,
            evidence: evidence.clone(),
        }
        .into();
        assert!(matches!(blocked.decision, Decision::Refuse), "{blocked:?}");

        // Pending → Refer, NEVER Refuse (the ceiling law) — for every PendingReason.
        for reason in [
            PendingReason::UnknownAuthorAtNonFloorReach,
            PendingReason::NewVoiceWithoutSponsor,
        ] {
            let pending: Verdict = ReachVerdict::Pending {
                reason,
                evidence: evidence.clone(),
            }
            .into();
            assert!(
                matches!(pending.decision, Decision::Refer(_)),
                "Pending must map to Refer, got {pending:?}"
            );
            assert!(
                !matches!(pending.decision, Decision::Refuse),
                "Pending must NEVER collapse to Refuse"
            );
        }
    }

    // Reach is a closed-verdict axis (epr:registry:axis:reach). A closed verdict owes
    // its counterparty the answer to "which policy decided this?" — and the answer has
    // to be a content address, because two peers can hold different bytes under the
    // same `revision: 1`. These pin the citation on EVERY decision arm, not just the
    // happy one: a refusal is exactly the verdict a counterparty most wants to audit.
    #[test]
    fn every_decision_arm_cites_the_policy_that_produced_it() {
        use elohim_epr::verdict::Decision;

        let evidence = StandingEvidence {
            standing: Standing::Unknown,
        };
        let pin = Some("bafyreipolicypin".to_string());

        let cases = vec![
            ReachVerdict::Allowed {
                floor_class_match: Some(FloorClass::CidTargetedLookup),
                evidence: evidence.clone(),
            },
            ReachVerdict::Blocked {
                reason: BlockReason::StandingBelowThreshold,
                evidence: evidence.clone(),
            },
            ReachVerdict::Pending {
                reason: PendingReason::UnknownAuthorAtNonFloorReach,
                evidence: evidence.clone(),
            },
        ];

        for case in cases {
            let v = case.into_verdict(pin.clone());
            assert_eq!(
                v.policy_ref, pin,
                "every arm must cite its policy — including {:?}",
                v.decision
            );
            // The citation must not quietly replace the evidence.
            assert!(
                !v.witness.checks.is_empty(),
                "policy_ref supplements the witness, never substitutes for it"
            );
        }

        // Refer carries the citation too: the layer receiving the escalation needs to
        // know which policy could not decide it.
        let referred = ReachVerdict::Pending {
            reason: PendingReason::NewVoiceWithoutSponsor,
            evidence,
        }
        .into_verdict(pin.clone());
        assert!(matches!(referred.decision, Decision::Refer(_)));
        assert_eq!(referred.policy_ref, pin);
    }

    // An unpinned policy must emit NO ref rather than a name it cannot back. Silence
    // here is honest; a placeholder would be a citation that resolves to nothing.
    #[test]
    fn unpinned_policy_emits_no_citation() {
        use elohim_epr::verdict::Verdict;

        let v: Verdict = ReachVerdict::Allowed {
            floor_class_match: None,
            evidence: StandingEvidence {
                standing: Standing::Unknown,
            },
        }
        .into();
        assert_eq!(
            v.policy_ref, None,
            "the registry-less From impl cannot supply a trace, and must not invent one"
        );

        // A hand-built policy is not content-addressed, so it has no ref to give.
        let hand_built = registry_with_full_policy();
        assert_eq!(
            hand_built.standing_policy_ref(),
            None,
            "from_payload_json has no CID — an unpinned policy cites nothing"
        );
    }

    #[test]
    fn registry_loaded_from_db_carries_the_policy_cid() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let payload = r#"{"manifestKind":"standing-policy","revision":1,
            "reachThresholds":{"public":"high"}}"#;
        crate::db::manifests::insert_manifest(
            &mut conn,
            &crate::db::manifests::ManifestRow {
                cid: "bafyreistandingpolicy".to_string(),
                manifest_kind: "standing-policy".to_string(),
                pillar: None,
                payload_json: payload.to_string(),
                schema_ref: None,
                signer_pubkey: vec![1, 2, 3],
                created_at: "2026-07-24T00:00:00Z".to_string(),
                verified_at: None,
                revision: 1,
            },
        )
        .expect("upsert standing-policy");

        let registry = ManifestRegistry::new();
        registry.load_from_db(&mut conn).expect("load");

        assert_eq!(
            registry.standing_policy_ref(),
            Some("bafyreistandingpolicy".to_string()),
            "the row's CID must survive into the registry — discarding it is what made \
             every derived verdict uncitable"
        );
    }

    #[test]
    fn legacy_vocabulary_manifest_still_evaluates_floor_reach() {
        // Data-aware migration (spec §7.5): a live standing-policy manifest
        // payload may still carry the retired kebab-8 key "household" (now
        // Reach::Trusted) in its reachThresholds map. parse_reach_key covers
        // legacy manifest ingestion generally, and — independently — the
        // floor-allowed short-circuit means Reach::Trusted evaluates to
        // Allowed regardless of how the manifest spells its threshold keys.
        // Together these prove a legacy-vocabulary manifest keeps evaluating
        // rather than 404ing or panicking.
        let json = r#"{"manifestKind":"standing-policy","revision":1,
            "floor":{"classes":[]},
            "newVoiceBaseline":{"score":"floor","vulnerableClassLift":"low"},
            "debitWeights":{"squelch":{"advisory":0,"debit-soft":1,"debit-firm":3},"correction":{"advisory":0,"debit-soft":10,"debit-firm":20},"retraction":{"advisory":0,"debit-soft":-5,"debit-firm":-10},"quarantine":{"advisory":0,"debit-soft":12,"debit-firm":30},"vouch":{"advisory":0,"debit-soft":-3,"debit-firm":-8}},
            "unknownTreatment":{"default":"conservative","evidenceSources":[]},
            "reachThresholds":{"household":"any"}
        }"#;
        let r = ManifestRegistry::from_payload_json(json).expect("parse");
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let v = evaluate(&[0; 32], &[1; 32], Reach::Trusted, &mut conn, &r);
        assert!(matches!(v, ReachVerdict::Allowed { .. }), "{v:?}");
        assert_eq!(
            crate::services::epr_kind::parse_reach_key("household"),
            Some(Reach::Trusted)
        );
    }
}
