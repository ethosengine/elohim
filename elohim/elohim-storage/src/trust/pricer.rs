//! `VerificationPricer` — prices verification depth against declared stage,
//! floor class, reach, and standing.
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-08-head-plane-trust-gradient-program-plan.md`
//! §3 L5. PURE module — no diesel, no tokio, no clock, no env.
//!
//! **THE safety invariant** (the keystone this program reviews before
//! accepting any change here): a [`FloorClass`] other than `None` forces
//! [`VerificationDepth::FullChain`] at EVERY [`NetworkStage`], including
//! `Simulacra`. [`tests::floor_forces_full_chain_across_the_entire_product_space`]
//! is the property test that pins it, iterating the full
//! `NetworkStage × FloorClass × Reach × Standing` product — not a sample.

use elohim_epr::Reach;
use seam_contracts::ReasonLabel;

use crate::services::standing::Standing;
use crate::trust::stage::NetworkStage;

/// How deep a verification must go. Ordered cheapest-to-most-thorough by
/// declaration, though no code in this module compares depths — the pricer
/// picks one outright.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationDepth {
    /// Trust the carried provenance outright — the cheapest path.
    AcceptWithProvenance,
    /// Verify only what changed since the last known-good state.
    DeltaVerify,
    /// Re-derive and verify everything. Today's (and INERT's) only answer.
    FullChain,
}

/// Floor classes never cheapen — a floor-classed decision point always gets
/// [`VerificationDepth::FullChain`], regardless of stage, reach, or standing.
/// `None` means no floor applies; the pricer is free to price normally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloorClass {
    /// No floor protection applies to this decision point.
    None,
    /// Constitutional kinds (Manifest, Attestation, Delegation) — see
    /// `services::floor_protections::is_constitutional_kind`.
    Constitutional,
    /// Local-relationship reach — unconditional regardless of standing. See
    /// `services::floor_protections::is_local_relationship_reach`.
    LocalRelationship,
    /// A correction/counter-evidence signal — corrections always reach the
    /// creator, un-filterable (brainstorm §3.3, "content creator" persona).
    CounterEvidence,
}

impl FloorClass {
    /// `true` for every variant except `None`.
    pub fn is_protected(self) -> bool {
        self != FloorClass::None
    }
}

/// Everything a [`VerificationPricer`] needs to price one decision point.
#[derive(Debug, Clone, Copy)]
pub struct PricingInput {
    pub stage: NetworkStage,
    pub floor: FloorClass,
    /// TYPED reach — never `&str`. The `&str` path
    /// (`epr_service::reach_level_index`) maps an unknown string to the
    /// MOST permissive tier; a pricer must never inherit that fail-open
    /// behavior, so it only ever sees the closed, DNA-notarized enum.
    pub reach: Reach,
    /// The derived standing view — `Unknown` everywhere until T19 lands the
    /// `standing_projector` writer (plan §2.1, evidence correction #1).
    pub standing: Standing,
}

/// The closed, countable outcome vocabulary for a pricing decision.
///
/// Single-variant today (`PricerInert`) — a legitimate, non-discriminating
/// vocabulary per `seam_contracts::ReasonLabel`'s own doc: "a decision point
/// may genuinely have one outcome today." Later tasks (T9/T19) add the
/// standing-aware reasons; this task never claims gradient behavior it does
/// not implement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PricingReason {
    /// The pricer is [`InertPricer`] — always `FullChain`, unconditionally.
    /// This IS today's behavior, restated as a typed reason.
    PricerInert,
}

impl ReasonLabel for PricingReason {
    const ALL: &'static [Self] = &[PricingReason::PricerInert];

    fn label(&self) -> &'static str {
        match self {
            PricingReason::PricerInert => "pricer_inert",
        }
    }
}

/// The priced verdict: how deep to verify, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PricedVerification {
    pub depth: VerificationDepth,
    pub reason: PricingReason,
}

/// Prices a [`PricingInput`] into a [`PricedVerification`].
///
/// Any implementor MUST uphold the floor invariant: `floor != FloorClass::None`
/// implies `depth == VerificationDepth::FullChain`, at every stage. See the
/// module-level doc and the property test.
pub trait VerificationPricer {
    fn price(&self, input: &PricingInput) -> PricedVerification;
}

/// Always `FullChain`, unconditionally — today's behavior, made a named,
/// swappable implementor instead of an inline default. The landing shape:
/// every caller that has not yet been given a real stage/standing-aware
/// pricer gets this one via `TrustGradient::inert()`.
#[derive(Debug, Clone, Copy, Default)]
pub struct InertPricer;

impl VerificationPricer for InertPricer {
    fn price(&self, _input: &PricingInput) -> PricedVerification {
        PricedVerification {
            depth: VerificationDepth::FullChain,
            reason: PricingReason::PricerInert,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::standing::StandingScore;

    const STAGES: [NetworkStage; 4] = NetworkStage::ALL;
    const FLOORS: [FloorClass; 4] = [
        FloorClass::None,
        FloorClass::Constitutional,
        FloorClass::LocalRelationship,
        FloorClass::CounterEvidence,
    ];
    const REACHES: [Reach; 8] = [
        Reach::Private,
        Reach::SelfScope,
        Reach::Intimate,
        Reach::Trusted,
        Reach::Familiar,
        Reach::Community,
        Reach::Public,
        Reach::Commons,
    ];
    const STANDINGS: [Standing; 6] = [
        Standing::Unknown,
        Standing::Computed {
            score: StandingScore::Floor,
        },
        Standing::Computed {
            score: StandingScore::Low,
        },
        Standing::Computed {
            score: StandingScore::Neutral,
        },
        Standing::Computed {
            score: StandingScore::High,
        },
        Standing::Computed {
            score: StandingScore::Trusted,
        },
    ];

    /// THE safety keystone. Every (stage, floor, reach, standing) quadruple
    /// in the full product space — 4 × 4 × 8 × 6 = 768 cases, not a sample —
    /// is priced, and wherever `floor != FloorClass::None` the result MUST
    /// be `FullChain`. This is what protects the floor guarantee across a
    /// future non-inert pricer swap: the test doesn't just check
    /// `InertPricer`'s trivial case, it fixes the invariant's shape so any
    /// later `VerificationPricer` impl can be dropped into this same loop.
    #[test]
    fn floor_forces_full_chain_across_the_entire_product_space() {
        let pricer = InertPricer;
        let mut cases_checked: u32 = 0;
        let mut floor_protected_cases: u32 = 0;

        for &stage in &STAGES {
            for &floor in &FLOORS {
                for &reach in &REACHES {
                    for &standing in &STANDINGS {
                        let input = PricingInput {
                            stage,
                            floor,
                            reach,
                            standing,
                        };
                        let priced = pricer.price(&input);
                        cases_checked += 1;

                        if floor.is_protected() {
                            floor_protected_cases += 1;
                            assert_eq!(
                                priced.depth,
                                VerificationDepth::FullChain,
                                "floor {floor:?} must force FullChain at EVERY stage, including \
                                 Simulacra — got {:?} at stage={stage:?}, reach={reach:?}, \
                                 standing={standing:?}",
                                priced.depth
                            );
                        }
                    }
                }
            }
        }

        assert_eq!(
            cases_checked,
            STAGES.len() as u32
                * FLOORS.len() as u32
                * REACHES.len() as u32
                * STANDINGS.len() as u32,
            "product space must be fully covered (4×4×8×6=768) — a silent shrink here weakens \
             the safety keystone"
        );
        // Sanity: the floor-protected slice is exactly 3/4 of the space
        // (Constitutional, LocalRelationship, CounterEvidence out of 4
        // FloorClass variants) — confirms the assertion above actually fired
        // on a non-trivial share of cases rather than vacuously passing.
        assert_eq!(floor_protected_cases, cases_checked * 3 / 4);
    }

    /// Explicit Simulacra-specific slice of the same invariant, named
    /// because Simulacra is the stage most tempted to cheapen (it's the
    /// "cheap dev/staging" stage) — this is the case the plan calls out by
    /// name ("EVERY stage including Simulacra").
    #[test]
    fn floor_forces_full_chain_even_at_simulacra() {
        let pricer = InertPricer;
        for &floor in &[
            FloorClass::Constitutional,
            FloorClass::LocalRelationship,
            FloorClass::CounterEvidence,
        ] {
            for &reach in &REACHES {
                for &standing in &STANDINGS {
                    let priced = pricer.price(&PricingInput {
                        stage: NetworkStage::Simulacra,
                        floor,
                        reach,
                        standing,
                    });
                    assert_eq!(priced.depth, VerificationDepth::FullChain);
                }
            }
        }
    }

    #[test]
    fn inert_pricer_always_returns_pricer_inert_reason() {
        let pricer = InertPricer;
        let priced = pricer.price(&PricingInput {
            stage: NetworkStage::Enforced,
            floor: FloorClass::None,
            reach: Reach::Commons,
            standing: Standing::Unknown,
        });
        assert_eq!(priced.reason, PricingReason::PricerInert);
        assert_eq!(priced.depth, VerificationDepth::FullChain);
    }

    #[test]
    fn floor_class_none_is_not_protected() {
        assert!(!FloorClass::None.is_protected());
    }

    #[test]
    fn every_non_none_floor_class_is_protected() {
        for floor in [
            FloorClass::Constitutional,
            FloorClass::LocalRelationship,
            FloorClass::CounterEvidence,
        ] {
            assert!(floor.is_protected());
        }
    }

    #[test]
    fn pricing_reason_is_reason_label_conformant() {
        seam_contracts::assert_reason_labels_conformant::<PricingReason>();
        seam_contracts::assert_reason_labels_stable::<PricingReason>(&["pricer_inert"]);
    }
}
