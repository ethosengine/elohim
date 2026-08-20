//! `VerificationPricer` — prices verification depth against declared stage,
//! floor class, reach, and standing.
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-08-head-plane-trust-gradient-program-plan.md`
//! §3 L5. PURE module — no diesel, no tokio, no clock, no env.
//!
//! **THE safety invariant** (the keystone this program reviews before
//! accepting any change here): a [`FloorClass`] other than `None` forces
//! [`VerificationDepth::FullChain`] at EVERY [`NetworkStage`], including
//! `Simulacra`. [`tests::every_in_crate_pricer_forces_full_chain_across_the_entire_product_space`]
//! is the property test that pins it, iterating the full
//! `NetworkStage × FloorClass × Reach × Standing × provenance_carried` product
//! — not a sample — for every implementation registered in
//! `tests::in_crate_pricers` (a new `VerificationPricer` impl MUST be added to
//! that registry; the harness's own teeth are proven by a
//! deliberately-cheapening mutant test).
//!
//! Two implementors ship: [`InertPricer`] (always `FullChain` — today's
//! behavior, and still the default at every call site that has not been given a
//! resolver) and [`DeclaredStakesPricer`] (Q7 — `AcceptWithProvenance` only
//! under an explicit [`NetworkStage::Simulacra`] declaration, on an unprotected
//! decision point that actually carries provenance). The consumption point that
//! turns a verdict into behavior is [`crate::trust::adoption_pricing`].

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
    /// (`epr_service::reach_level_index`) is a lossy projection of this enum;
    /// a pricer must never inherit a string parse, so it only ever sees the
    /// closed, DNA-notarized enum. (That path mapped an unknown string to the
    /// MOST permissive tier until 2026-08-20; it now fails closed to
    /// `u8::MAX`, but the argument for keeping this field typed is unchanged.)
    pub reach: Reach,
    /// The derived standing view — `Unknown` everywhere until T19 lands the
    /// `standing_projector` writer (plan §2.1, evidence correction #1).
    pub standing: Standing,
    /// Does this decision point actually HAVE provenance to accept?
    ///
    /// [`VerificationDepth::AcceptWithProvenance`] means "trust the carried
    /// provenance outright" — so a decision point carrying NO provenance can
    /// never be priced there, at any stage, however cheap the stage is. This
    /// field is what makes that a type-level obligation rather than a comment:
    /// a pricer cannot cheapen its way past an absent courier.
    ///
    /// For the head-plane adoption consumer (Q7,
    /// [`crate::trust::adoption_pricing`]) this is `true` exactly when a peer
    /// ADVERTISED a declared head for the id in this sweep's hint window AND
    /// the local row declares nothing — the `AdoptPeer` corner of
    /// `head_adoption::decide_head_action`, which is the only corner where an
    /// advertised declaration can be adopted directly.
    pub provenance_carried: bool,
}

/// The closed, countable outcome vocabulary for a pricing decision.
///
/// `PricerInert` is [`InertPricer`]'s only answer and remains the default
/// everywhere. The four reasons below belong to [`DeclaredStakesPricer`] (Q7)
/// and enumerate every way it can answer, so a "why is nothing cheapening?"
/// question is answered by a label rather than by reading the source: a fleet
/// sitting at `stage_requires_full_chain` has no Simulacra declaration, one at
/// `no_provenance_carried` has the declaration but no advertising peer, and one
/// at `floor_protected` is being held by the safety keystone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PricingReason {
    /// The pricer is [`InertPricer`] — always `FullChain`, unconditionally.
    /// This IS today's behavior, restated as a typed reason.
    PricerInert,
    /// THE safety keystone fired: `floor != FloorClass::None`, so the depth is
    /// `FullChain` regardless of stage, reach, standing, or provenance.
    FloorProtected,
    /// The declared stage is not [`NetworkStage::Simulacra`]. Bootstrap,
    /// Coordinated and Enforced all price `FullChain` — the gradient's whole
    /// point is that only an EXPLICIT low-stakes declaration buys the cheap
    /// path.
    StageRequiresFullChain,
    /// Simulacra is declared, but this decision point carries no provenance to
    /// accept ([`PricingInput::provenance_carried`] is `false`) — there is
    /// nothing to accept WITH, so the depth is `FullChain`.
    NoProvenanceCarried,
    /// Simulacra is declared, no floor applies, and provenance is carried:
    /// [`VerificationDepth::AcceptWithProvenance`].
    DeclaredSimulacraProvenance,
}

impl ReasonLabel for PricingReason {
    const ALL: &'static [Self] = &[
        PricingReason::PricerInert,
        PricingReason::FloorProtected,
        PricingReason::StageRequiresFullChain,
        PricingReason::NoProvenanceCarried,
        PricingReason::DeclaredSimulacraProvenance,
    ];

    fn label(&self) -> &'static str {
        match self {
            PricingReason::PricerInert => "pricer_inert",
            PricingReason::FloorProtected => "floor_protected",
            PricingReason::StageRequiresFullChain => "stage_requires_full_chain",
            PricingReason::NoProvenanceCarried => "no_provenance_carried",
            PricingReason::DeclaredSimulacraProvenance => "declared_simulacra_provenance",
        }
    }
}

/// The priced verdict: how deep to verify, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PricedVerification {
    pub depth: VerificationDepth,
    pub reason: PricingReason,
}

impl PricedVerification {
    /// Today's answer as a VALUE — `FullChain` / `PricerInert`, exactly what
    /// [`InertPricer`] returns.
    ///
    /// Exists so a call site that has not been given a pricer (the boot
    /// re-anchor pass, the ghost witness sweep) can pass the inert verdict
    /// explicitly instead of the parameter defaulting invisibly. Making the
    /// unpriced path SAY it is unpriced is what keeps "behavior-neutral
    /// everywhere but an explicit Simulacra declaration" checkable by reading
    /// the call sites.
    pub const fn inert() -> Self {
        Self {
            depth: VerificationDepth::FullChain,
            reason: PricingReason::PricerInert,
        }
    }

    /// `true` iff this verdict licenses the ceremony compression — the ONE
    /// predicate every consumer asks, so a consumer can never re-derive it
    /// from a depth comparison and drift.
    pub const fn accepts_with_provenance(&self) -> bool {
        matches!(self.depth, VerificationDepth::AcceptWithProvenance)
    }
}

/// Prices a [`PricingInput`] into a [`PricedVerification`].
///
/// Any implementor MUST uphold **two** invariants, both pinned registry-wide
/// over the full product space in `tests`:
///
/// 1. **FLOOR (the keystone).** `floor != FloorClass::None` implies
///    `depth == VerificationDepth::FullChain`, at every stage —
///    [`tests::every_in_crate_pricer_forces_full_chain_across_the_entire_product_space`].
/// 2. **STAGE.** `stage != NetworkStage::Simulacra` implies
///    `depth == VerificationDepth::FullChain`, **regardless of every other
///    field** —
///    [`tests::every_in_crate_pricer_is_full_chain_at_every_non_simulacra_stage`].
///
/// The second is not decoration and not implied by the first. It is the
/// obligation `crate::trust::adoption_pricing` RELIES on: only an explicit
/// low-stakes declaration may ever buy a cheaper depth, so a consumer may
/// reason about a non-Simulacra stage without consulting standing or reach.
/// An implementor that wants to cheapen at `Coordinated` must change this
/// contract and its test deliberately — it cannot arrive by accident through a
/// swapped `dyn` seam.
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
        PricedVerification::inert()
    }
}

/// The Q7 pricer: prices `AcceptWithProvenance` **only** under an explicit
/// [`NetworkStage::Simulacra`] declaration, and only for an unprotected
/// decision point that actually carries provenance.
///
/// Three gates, checked in this order, each with its own reason label:
///
/// 1. `floor.is_protected()` ⇒ `FullChain` / `FloorProtected`. THE keystone,
///    checked FIRST so no later gate can be reordered in front of it.
/// 2. `stage != Simulacra` ⇒ `FullChain` / `StageRequiresFullChain`. This is
///    what makes the swap-in behavior-neutral at Bootstrap / Coordinated /
///    Enforced: the pricer answers exactly what [`InertPricer`] answers, so a
///    fleet with no Simulacra declaration cannot tell the two apart.
/// 3. `!provenance_carried` ⇒ `FullChain` / `NoProvenanceCarried`. Accepting
///    "with provenance" when there is no provenance would not be a cheaper
///    verification, it would be an unverified one.
///
/// Otherwise `AcceptWithProvenance` / `DeclaredSimulacraProvenance`.
///
/// **PURE** — the `stage` arrives on [`PricingInput`], resolved upstream by a
/// [`crate::trust::stage::StakesResolver`]. This type reads no env, no
/// manifest, no clock, and holds no state, which is why it is a unit struct:
/// the declaration is data flowing in, never configuration baked into the
/// pricer.
///
/// **`DeltaVerify` is never produced.** The head-plane consumer has no
/// "verify only what changed" mode to route to — the depth exists in the
/// vocabulary for the snapshot/delta path (L2/T8) and this pricer honestly
/// declines to claim it. The `delta_verify` counter series therefore reads a
/// measured zero rather than being absent.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeclaredStakesPricer;

impl VerificationPricer for DeclaredStakesPricer {
    fn price(&self, input: &PricingInput) -> PricedVerification {
        // (1) THE KEYSTONE, first and unconditional. Constitutional,
        // LocalRelationship and CounterEvidence never cheapen at ANY stage,
        // Simulacra included.
        if input.floor.is_protected() {
            return PricedVerification {
                depth: VerificationDepth::FullChain,
                reason: PricingReason::FloorProtected,
            };
        }
        // (2) Only an EXPLICIT Simulacra declaration buys anything. Note this
        // is an equality test, not `<=`: a stage ORDER comparison would make a
        // future stage inserted below Simulacra silently inherit the cheap
        // path.
        if input.stage != NetworkStage::Simulacra {
            return PricedVerification {
                depth: VerificationDepth::FullChain,
                reason: PricingReason::StageRequiresFullChain,
            };
        }
        // (3) Nothing to accept ⇒ nothing to accept WITH.
        if !input.provenance_carried {
            return PricedVerification {
                depth: VerificationDepth::FullChain,
                reason: PricingReason::NoProvenanceCarried,
            };
        }
        PricedVerification {
            depth: VerificationDepth::AcceptWithProvenance,
            reason: PricingReason::DeclaredSimulacraProvenance,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::standing::StandingScore;

    type RegisteredPricer = (&'static str, &'static dyn VerificationPricer);

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
    /// The Q7 dimension: does the decision point carry provenance to accept?
    /// Both values are exercised so the keystone is proven on the side that
    /// CAN cheapen (`true`) as well as the side that cannot.
    const PROVENANCES: [bool; 2] = [false, true];
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

    /// 4 stages × 4 floors × 8 reaches × 6 standings × 2 provenances.
    const EXPECTED_PRODUCT_CASES: usize = 4 * 4 * 8 * 6 * 2;

    /// EXHAUSTIVENESS WITNESSES. Each `match` below is exhaustive by
    /// construction, so ADDING a variant to any of these enums is a COMPILE
    /// ERROR here — pointing straight at the dimension array that must grow.
    /// Without them a new `Reach`, `NetworkStage`, `FloorClass` or
    /// `StandingScore` would simply go untested, silently, forever. Paired with
    /// the literal `EXPECTED_PRODUCT_CASES`, this closes both halves: a variant
    /// cannot be added without noticing, and an array cannot be shrunk without
    /// failing.
    #[test]
    fn every_pricing_dimension_is_covered_by_its_array() {
        fn stage_witness(s: NetworkStage) -> usize {
            match s {
                NetworkStage::Simulacra => 0,
                NetworkStage::Bootstrap => 1,
                NetworkStage::Coordinated => 2,
                NetworkStage::Enforced => 3,
            }
        }
        fn floor_witness(f: FloorClass) -> usize {
            match f {
                FloorClass::None => 0,
                FloorClass::Constitutional => 1,
                FloorClass::LocalRelationship => 2,
                FloorClass::CounterEvidence => 3,
            }
        }
        fn reach_witness(r: Reach) -> usize {
            match r {
                Reach::Private => 0,
                Reach::SelfScope => 1,
                Reach::Intimate => 2,
                Reach::Trusted => 3,
                Reach::Familiar => 4,
                Reach::Community => 5,
                Reach::Public => 6,
                Reach::Commons => 7,
            }
        }
        fn standing_witness(s: Standing) -> usize {
            match s {
                Standing::Unknown => 0,
                Standing::Computed { score } => match score {
                    StandingScore::Floor => 1,
                    StandingScore::Low => 2,
                    StandingScore::Neutral => 3,
                    StandingScore::High => 4,
                    StandingScore::Trusted => 5,
                },
            }
        }
        fn depth_witness(d: VerificationDepth) -> usize {
            match d {
                VerificationDepth::AcceptWithProvenance => 0,
                VerificationDepth::DeltaVerify => 1,
                VerificationDepth::FullChain => 2,
            }
        }

        let mut seen: Vec<usize> = STAGES.iter().copied().map(stage_witness).collect();
        seen.sort_unstable();
        assert_eq!(
            seen,
            vec![0, 1, 2, 3],
            "STAGES must list every NetworkStage"
        );
        let mut seen: Vec<usize> = FLOORS.iter().copied().map(floor_witness).collect();
        seen.sort_unstable();
        assert_eq!(seen, vec![0, 1, 2, 3], "FLOORS must list every FloorClass");
        let mut seen: Vec<usize> = REACHES.iter().copied().map(reach_witness).collect();
        seen.sort_unstable();
        assert_eq!(
            seen,
            (0..8).collect::<Vec<_>>(),
            "REACHES must list every Reach"
        );
        let mut seen: Vec<usize> = STANDINGS.iter().copied().map(standing_witness).collect();
        seen.sort_unstable();
        assert_eq!(
            seen,
            (0..6).collect::<Vec<_>>(),
            "STANDINGS must list every Standing"
        );
        assert_eq!(PROVENANCES.len(), 2, "provenance_carried is a bool");
        // Depth is not a loop dimension, but the witness keeps a new depth
        // variant from slipping past the label mapping in `metrics.rs`.
        assert_eq!(depth_witness(VerificationDepth::FullChain), 2);

        assert_eq!(
            STAGES.len() * FLOORS.len() * REACHES.len() * STANDINGS.len() * PROVENANCES.len(),
            EXPECTED_PRODUCT_CASES,
            "the arrays and the literal case count must agree"
        );
    }

    #[derive(Debug, PartialEq, Eq)]
    struct FloorConformanceReport {
        cases_checked: usize,
        floor_protected_cases: usize,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct FloorConformanceViolation {
        stage: NetworkStage,
        floor: FloorClass,
        reach: Reach,
        standing: Standing,
        provenance_carried: bool,
        actual_depth: VerificationDepth,
    }

    /// Exercise one pricer through the complete pricing product space.
    ///
    /// Returning the violation instead of asserting inside the loop lets the
    /// mutation test below prove that this harness itself can go red.
    fn check_floor_conformance(
        pricer: &dyn VerificationPricer,
    ) -> Result<FloorConformanceReport, FloorConformanceViolation> {
        let mut cases_checked = 0;
        let mut floor_protected_cases = 0;

        for &stage in &STAGES {
            for &floor in &FLOORS {
                for &reach in &REACHES {
                    for &standing in &STANDINGS {
                        for &provenance_carried in &PROVENANCES {
                            let input = PricingInput {
                                stage,
                                floor,
                                reach,
                                standing,
                                provenance_carried,
                            };
                            let priced = pricer.price(&input);
                            cases_checked += 1;

                            if floor.is_protected() {
                                floor_protected_cases += 1;
                                if priced.depth != VerificationDepth::FullChain {
                                    return Err(FloorConformanceViolation {
                                        stage,
                                        floor,
                                        reach,
                                        standing,
                                        provenance_carried,
                                        actual_depth: priced.depth,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(FloorConformanceReport {
            cases_checked,
            floor_protected_cases,
        })
    }

    fn assert_floor_conformance(
        implementation_name: &str,
        pricer: &dyn VerificationPricer,
    ) -> FloorConformanceReport {
        check_floor_conformance(pricer).unwrap_or_else(|violation| {
            panic!(
                "VerificationPricer {implementation_name} cheapened protected floor \
                 {:?} at stage={:?}, reach={:?}, standing={:?}, provenance_carried={}: \
                 got {:?}, expected FullChain",
                violation.floor,
                violation.stage,
                violation.reach,
                violation.standing,
                violation.provenance_carried,
                violation.actual_depth,
            )
        })
    }

    /// Registry of every production `VerificationPricer` implementation in
    /// this crate. Adding an implementation means adding it here so the same
    /// conformance suite exercises it; tests below consume only this registry.
    fn in_crate_pricers() -> [RegisteredPricer; 2] {
        static INERT: InertPricer = InertPricer;
        static DECLARED: DeclaredStakesPricer = DeclaredStakesPricer;
        [("InertPricer", &INERT), ("DeclaredStakesPricer", &DECLARED)]
    }

    /// Test-only mutant: it violates the keystone exactly where future
    /// gradient pricing is most tempted to do so.
    struct CheapeningMutantPricer;

    impl VerificationPricer for CheapeningMutantPricer {
        fn price(&self, input: &PricingInput) -> PricedVerification {
            PricedVerification {
                depth: if input.floor.is_protected() {
                    VerificationDepth::DeltaVerify
                } else {
                    VerificationDepth::FullChain
                },
                reason: PricingReason::PricerInert,
            }
        }
    }

    /// THE safety keystone. Every (stage, floor, reach, standing) quadruple
    /// in the full product space — 4 × 4 × 8 × 6 = 768 cases, not a sample —
    /// is priced, and wherever `floor != FloorClass::None` the result MUST
    /// be `FullChain`. This is what protects the floor guarantee across a
    /// future non-inert pricer swap. Every in-crate implementation is supplied
    /// through `in_crate_pricers`, while the shared checker takes a trait
    /// object and therefore exercises the actual swappable interface.
    #[test]
    fn every_in_crate_pricer_forces_full_chain_across_the_entire_product_space() {
        for (implementation_name, pricer) in in_crate_pricers() {
            let report = assert_floor_conformance(implementation_name, pricer);
            // A LITERAL, deliberately not `STAGES.len() * ...`: re-deriving the
            // expectation from the same arrays the loop walks is a tautology
            // that can never fail, so shrinking any dimension array would pass
            // silently with a fraction of the coverage. The literal is the only
            // thing that actually notices.
            assert_eq!(
                report.cases_checked, EXPECTED_PRODUCT_CASES,
                "product space must be fully covered (4×4×8×6×2=1536) — a silent shrink here \
                 weakens the safety keystone for {implementation_name}"
            );
            // Sanity: the floor-protected slice is exactly 3/4 of the space
            // (Constitutional, LocalRelationship, CounterEvidence out of 4
            // FloorClass variants) — confirms the assertion above actually
            // fired on a non-trivial share rather than vacuously passing.
            assert_eq!(
                report.floor_protected_cases,
                report.cases_checked * 3 / 4,
                "protected slice unexpectedly changed for {implementation_name}"
            );
        }
    }

    #[test]
    fn conformance_harness_rejects_a_floor_cheapening_mutant() {
        let violation = check_floor_conformance(&CheapeningMutantPricer)
            .expect_err("the deliberately cheapening mutant must violate the floor invariant");

        assert!(violation.floor.is_protected());
        assert_eq!(violation.actual_depth, VerificationDepth::DeltaVerify);
    }

    /// Explicit Simulacra-specific slice of the same invariant, named
    /// because Simulacra is the stage most tempted to cheapen (it's the
    /// "cheap dev/staging" stage) — this is the case the plan calls out by
    /// name ("EVERY stage including Simulacra").
    #[test]
    fn floor_forces_full_chain_even_at_simulacra() {
        for (implementation_name, pricer) in in_crate_pricers() {
            for &floor in &[
                FloorClass::Constitutional,
                FloorClass::LocalRelationship,
                FloorClass::CounterEvidence,
            ] {
                for &reach in &REACHES {
                    for &standing in &STANDINGS {
                        // `provenance_carried: true` on purpose — this is the
                        // ONLY combination that could tempt a pricer to cheapen
                        // (cheapest stage, provenance in hand). The floor must
                        // still win.
                        let priced = pricer.price(&PricingInput {
                            stage: NetworkStage::Simulacra,
                            floor,
                            reach,
                            standing,
                            provenance_carried: true,
                        });
                        assert_eq!(
                            priced.depth,
                            VerificationDepth::FullChain,
                            "{implementation_name} cheapened {floor:?} at Simulacra"
                        );
                    }
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
            provenance_carried: false,
        });
        assert_eq!(priced.reason, PricingReason::PricerInert);
        assert_eq!(priced.depth, VerificationDepth::FullChain);
    }

    // -----------------------------------------------------------------------
    // DeclaredStakesPricer (Q7)
    // -----------------------------------------------------------------------

    fn declared(stage: NetworkStage, floor: FloorClass, provenance: bool) -> PricedVerification {
        DeclaredStakesPricer.price(&PricingInput {
            stage,
            floor,
            reach: Reach::Commons,
            standing: Standing::Unknown,
            provenance_carried: provenance,
        })
    }

    /// THE behavior-neutrality pin: at every stage that is not an explicit
    /// Simulacra declaration, the Q7 pricer answers exactly what
    /// [`InertPricer`] answers — same depth — over the whole
    /// floor × provenance product. This is what makes the swap-in provably a
    /// no-op on a fleet that declares nothing.
    #[test]
    fn declared_stakes_pricer_matches_inert_depth_at_every_non_simulacra_stage() {
        for &stage in &[
            NetworkStage::Bootstrap,
            NetworkStage::Coordinated,
            NetworkStage::Enforced,
        ] {
            for &floor in &FLOORS {
                for &provenance in &PROVENANCES {
                    let input = PricingInput {
                        stage,
                        floor,
                        reach: Reach::Commons,
                        standing: Standing::Unknown,
                        provenance_carried: provenance,
                    };
                    assert_eq!(
                        DeclaredStakesPricer.price(&input).depth,
                        InertPricer.price(&input).depth,
                        "DeclaredStakesPricer diverged from InertPricer at stage={stage:?}, \
                         floor={floor:?}, provenance_carried={provenance}"
                    );
                }
            }
        }
    }

    #[test]
    fn declared_simulacra_with_provenance_accepts_with_provenance() {
        let priced = declared(NetworkStage::Simulacra, FloorClass::None, true);
        assert_eq!(priced.depth, VerificationDepth::AcceptWithProvenance);
        assert_eq!(priced.reason, PricingReason::DeclaredSimulacraProvenance);
        assert!(priced.accepts_with_provenance());
    }

    #[test]
    fn declared_simulacra_without_provenance_stays_full_chain() {
        let priced = declared(NetworkStage::Simulacra, FloorClass::None, false);
        assert_eq!(priced.depth, VerificationDepth::FullChain);
        assert_eq!(priced.reason, PricingReason::NoProvenanceCarried);
        assert!(!priced.accepts_with_provenance());
    }

    /// The keystone, stated as its own named case with the reason label
    /// checked: at Simulacra, WITH provenance, every protected floor is still
    /// `FullChain` — and says `floor_protected`, not `stage_requires_full_chain`
    /// (a mislabelled hold would make a floor save look like an absent
    /// declaration on the meters).
    #[test]
    fn declared_stakes_pricer_labels_a_floor_save_as_floor_protected() {
        // Every STAGE, not just Simulacra: the floor gate must be checked
        // FIRST, and the only thing that proves the ordering is the LABEL. If
        // the stage gate ran first, a protected row at Bootstrap would report
        // `stage_requires_full_chain`, the `floor_protected` series would read
        // a permanent zero, and the operator story the reasons exist for would
        // be unreadable — with the depth invariant still perfectly intact.
        for &stage in &STAGES {
            for &floor in &[
                FloorClass::Constitutional,
                FloorClass::LocalRelationship,
                FloorClass::CounterEvidence,
            ] {
                for &provenance in &PROVENANCES {
                    let priced = declared(stage, floor, provenance);
                    assert_eq!(priced.depth, VerificationDepth::FullChain);
                    assert_eq!(
                        priced.reason,
                        PricingReason::FloorProtected,
                        "stage={stage:?} floor={floor:?} provenance={provenance}"
                    );
                }
            }
        }
    }

    /// THE STAGE OBLIGATION, registry-wide. `trust::adoption_pricing` relies on
    /// this to reason about a non-Simulacra stage without consulting reach or
    /// standing, and the `pricer` field is a `dyn` seam — so the property must
    /// hold for EVERY implementor, not just the one that happens to be wired
    /// today. A type-specific test would be a guarantee about a concrete type
    /// standing in for a guarantee about the interface.
    #[test]
    fn every_in_crate_pricer_is_full_chain_at_every_non_simulacra_stage() {
        for (implementation_name, pricer) in in_crate_pricers() {
            let mut checked = 0usize;
            for &stage in &STAGES {
                if stage == NetworkStage::Simulacra {
                    continue;
                }
                for &floor in &FLOORS {
                    for &reach in &REACHES {
                        for &standing in &STANDINGS {
                            for &provenance_carried in &PROVENANCES {
                                let priced = pricer.price(&PricingInput {
                                    stage,
                                    floor,
                                    reach,
                                    standing,
                                    provenance_carried,
                                });
                                assert_eq!(
                                    priced.depth,
                                    VerificationDepth::FullChain,
                                    "{implementation_name} cheapened at a non-Simulacra stage: \
                                     stage={stage:?} floor={floor:?} reach={reach:?} \
                                     standing={standing:?} provenance={provenance_carried}"
                                );
                                checked += 1;
                            }
                        }
                    }
                }
            }
            assert_eq!(
                checked,
                EXPECTED_PRODUCT_CASES / 4 * 3,
                "three of four stages must be covered for {implementation_name}"
            );
        }
    }

    #[test]
    fn declared_stakes_pricer_never_produces_delta_verify() {
        for &stage in &STAGES {
            for &floor in &FLOORS {
                for &provenance in &PROVENANCES {
                    assert_ne!(
                        declared(stage, floor, provenance).depth,
                        VerificationDepth::DeltaVerify,
                        "this pricer has no delta path to claim (stage={stage:?})"
                    );
                }
            }
        }
    }

    #[test]
    fn priced_verification_inert_is_exactly_the_inert_pricer_answer() {
        let inert = InertPricer.price(&PricingInput {
            stage: NetworkStage::Simulacra,
            floor: FloorClass::None,
            reach: Reach::Private,
            standing: Standing::Unknown,
            provenance_carried: true,
        });
        assert_eq!(PricedVerification::inert(), inert);
        assert!(!PricedVerification::inert().accepts_with_provenance());
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
        seam_contracts::assert_reason_labels_stable::<PricingReason>(&[
            "pricer_inert",
            "floor_protected",
            "stage_requires_full_chain",
            "no_provenance_carried",
            "declared_simulacra_provenance",
        ]);
    }
}
