//! `freshness` — the PURE predicate behind "freshness graded by declared stakes".
//!
//! Operator decision, 2026-08-21: a doorway's uniform honest-shed contract (every
//! data read 503 `catching-up` whenever the upstream circuit is open) is replaced
//! by a freshness REQUIREMENT graded by the read's declared stakes. Being behind
//! becomes an amber trust signal on the wire, not a blocker. Memory:
//! `project_freshness_graded_by_declared_stakes`.
//!
//! ## Two axes, never mixed
//!
//! [`Available`] is what the serving node CAN answer with — green (a live
//! upstream 200, the DHT-witnessed current projection), amber (true,
//! content-addressed last-good bytes the node itself stocked earlier), or
//! nothing at all. [`Requirement`] is POLICY: what a read of a given
//! [`ReadClass`] at a given [`NetworkStage`] is permitted to be answered with.
//! [`verdict`] is their product. `Shed` is an OUTCOME of that product (the
//! existing 503 `catching-up` contract), never a freshness LEVEL — there is no
//! such thing as "red freshness" here.
//!
//! ## Concerns
//!
//! - **C4 honest absence** ([`crate::canon::PIN_C4_HONEST_ABSENCE`]): amber is
//!   the honest name for "I hold real bytes but no current witness." The whole
//!   point of the [`FreshnessVerdict::ServeAmber`] arm is that stale-but-true
//!   is never dressed as green — the caller learns which one it got.
//! - **C13 graduated authority** ([`crate::canon::PIN_C13_GRADUATED_AUTHORITY`]):
//!   the requirement is GRADUATED by declared stakes rather than uniform, and
//!   the graduation is bounded by FLOOR rows ([`ReadClass::is_floor`]) that no
//!   stage may cheapen.
//! - **C8 observability-per-decision**: [`FreshnessVerdict`] implements
//!   [`crate::ReasonLabel`], so every verdict is countable by name.
//!
//! ## Floor rows are never stage-priced
//!
//! [`ReadClass::Authority`] and [`ReadClass::CounterEvidence`] are floor rows,
//! mirroring the `FloorClass` semantics of elohim-storage's
//! `trust/pricer.rs` (a protected floor forces `FullChain` at EVERY stage,
//! including `Simulacra`). Here the analogous invariant is: a floor row's
//! requirement is [`Requirement::GreenOnly`] at every stage, so its verdict is
//! never `ServeAmber`/`ServeAmberWarn` for ANY available colour.
//! [`tests::floor_classes_never_yield_amber_over_the_full_product`] pins that
//! over the full `ReadClass × NetworkStage × Available` product rather than a
//! sample.
//!
//! ## Purity
//!
//! No clock, no env, no I/O. `Available::Amber { age }` is an age the CALLER
//! computed against its own stocked-at stamp; this module never asks what time
//! it is. That is the same inversion-of-control seam every other predicate in
//! this crate keeps.

use core::str::FromStr;
use core::time::Duration;

use crate::reason::ReasonLabel;

/// How long knowledge-class last-good bytes stay servable. One day: a content
/// read answered from yesterday's true bytes is a better answer than a 503,
/// and content addressing makes "true" checkable.
pub const KNOWLEDGE_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// How long value-coupled last-good bytes stay servable at the permissive
/// stages (`Simulacra`/`Bootstrap`). Same 24h as knowledge: before the network
/// coordinates, a reciprocity view is informational.
pub const VALUE_COUPLED_MAX_AGE_PERMISSIVE: Duration = Duration::from_secs(24 * 60 * 60);

/// How long value-coupled last-good bytes stay servable once the network
/// declares `Coordinated`/`Enforced` — and they are served WARNED. Fifteen
/// minutes: a stewardship or shefa figure that drives a person's decision may
/// be a few minutes behind, never a day.
pub const VALUE_COUPLED_MAX_AGE_COORDINATED: Duration = Duration::from_secs(15 * 60);

/// The network's declared operating stage.
///
/// **This type is a MIRROR.** `elohim/elohim-storage/src/trust/stage.rs` is the
/// authority: same variants, same semantic `Ord`
/// (`Simulacra < Bootstrap < Coordinated < Enforced`), same exact-lowercase
/// [`FromStr`], same fail-closed [`Default`] of `Bootstrap`. A divergence
/// between the two is a BUG, not a local dialect. The mirror exists because
/// this crate is a leaf (zero first-party deps) and doorway — which needs the
/// stage but does not depend on `elohim-epr` or `elohim-storage` — must be able
/// to name it. Drift is caught by
/// [`tests::network_stage_variant_names_match_storage_trust_stage`], which
/// reads the authority's source text rather than trusting this comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NetworkStage {
    /// Dev/staging/genesis fixtures — cheapest, most permissive. Reached ONLY
    /// by explicit positive declaration, never by inference or fallback.
    Simulacra,
    /// The fail-closed default: structural validators only.
    Bootstrap,
    /// Elohim-coordinated trust.
    Coordinated,
    /// Full elohim enforcement.
    Enforced,
}

impl Default for NetworkStage {
    /// Fail-closed: `Bootstrap`, never `Simulacra`. Written by hand so the
    /// choice is a decision this file states, not an accident of declaration
    /// order (mirrors the authority's own reasoning).
    fn default() -> Self {
        NetworkStage::Bootstrap
    }
}

impl NetworkStage {
    /// All four stages in declared (== semantic) order.
    pub const ALL: [NetworkStage; 4] = [
        NetworkStage::Simulacra,
        NetworkStage::Bootstrap,
        NetworkStage::Coordinated,
        NetworkStage::Enforced,
    ];

    /// The metric/wire-safe lowercase label — the SAME vocabulary [`FromStr`]
    /// accepts, so a stage can round-trip through a label without an alias
    /// table growing between the two.
    pub fn label(self) -> &'static str {
        match self {
            NetworkStage::Simulacra => "simulacra",
            NetworkStage::Bootstrap => "bootstrap",
            NetworkStage::Coordinated => "coordinated",
            NetworkStage::Enforced => "enforced",
        }
    }
}

/// EXACT lowercase-variant-name parsing — no case folding, no aliasing ("dev"
/// is not `Simulacra`). An unrecognized string is `Err`, never a best-effort
/// guess; the CALLER decides what an unknown declaration means (doorway warns
/// and falls back to the fail-closed [`NetworkStage::default`]).
impl FromStr for NetworkStage {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "simulacra" => Ok(NetworkStage::Simulacra),
            "bootstrap" => Ok(NetworkStage::Bootstrap),
            "coordinated" => Ok(NetworkStage::Coordinated),
            "enforced" => Ok(NetworkStage::Enforced),
            _ => Err(()),
        }
    }
}

/// The declared stakes of ONE read — what it would cost to answer it with
/// bytes that no live witness confirms.
///
/// This first cut is derived from the route (doorway's
/// `routes::catching_up::read_class`). The EPR-coupling refinement (deriving
/// the class from the envelope's kind × reach × coupling) is the declared next
/// rung and is why the vocabulary, not the route table, lives here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadClass {
    /// Content, blobs, app bundles — the corpus. Stale-but-true beats absent.
    Knowledge,
    /// Reciprocity, stewardship, shefa, economic events — reads a person may
    /// act economically on.
    ValueCoupled,
    /// Auth, session, identity, governance, head-declare, canonical-head, and
    /// EVERY non-GET. A FLOOR row.
    Authority,
    /// A read that carries or resolves counter-evidence (a correction, a
    /// dispute, a revocation). A FLOOR row: a correction answered from stale
    /// bytes is worse than no answer, because it reads as "still uncorrected."
    CounterEvidence,
}

impl ReadClass {
    /// All four classes, in declared order.
    pub const ALL: [ReadClass; 4] = [
        ReadClass::Knowledge,
        ReadClass::ValueCoupled,
        ReadClass::Authority,
        ReadClass::CounterEvidence,
    ];

    /// FLOOR rows are never stage-priced. Mirrors `FloorClass::is_protected`
    /// in `elohim/elohim-storage/src/trust/pricer.rs`: a protected class gets
    /// the strictest treatment at EVERY stage, including the cheapest.
    pub fn is_floor(self) -> bool {
        matches!(self, ReadClass::Authority | ReadClass::CounterEvidence)
    }

    /// The metric/wire-safe label. `value` (not `value_coupled`) is the wire
    /// vocabulary the `x-elohim-freshness-class` header advertises.
    ///
    /// `counter_evidence` is presently UNREACHABLE from a route-derived class
    /// (recognizing counter-evidence needs the envelope, not the path) — it is
    /// named here because the predicate is the vocabulary's home, and the
    /// EPR-coupling rung will produce it.
    pub fn label(self) -> &'static str {
        match self {
            ReadClass::Knowledge => "knowledge",
            ReadClass::ValueCoupled => "value",
            ReadClass::Authority => "authority",
            ReadClass::CounterEvidence => "counter_evidence",
        }
    }
}

/// What the serving node CAN answer with right now — the availability axis.
/// Never a policy statement; policy is [`Requirement`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Available {
    /// A live 200 from the upstream — the DHT-witnessed current projection.
    Green,
    /// True, content-addressed last-good bytes this node stocked earlier,
    /// `age` old. The caller computes `age`; this module owns no clock.
    Amber {
        /// How long ago these bytes were stocked.
        age: Duration,
    },
    /// Nothing servable at all.
    None,
}

/// POLICY: what a read of a given class, at a given stage, may be answered with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    /// Amber is acceptable up to `max_age`, unremarked.
    AmberOk {
        /// Oldest amber this requirement will accept.
        max_age: Duration,
    },
    /// Amber is acceptable up to `max_age`, but the answer must carry a
    /// WARNING so the consumer knows it acted on unwitnessed bytes.
    AmberWarn {
        /// Oldest amber this requirement will accept.
        max_age: Duration,
    },
    /// Only a live, witnessed answer will do. The floor.
    GreenOnly,
}

/// The product of [`Requirement`] and [`Available`].
///
/// **Concerns:** C8 — implements [`ReasonLabel`], so `{class,stage,verdict}` is
/// a countable triple and a dashboard can enumerate the outcome space without a
/// hand-maintained legend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FreshnessVerdict {
    /// Serve the live, witnessed answer.
    Serve,
    /// Serve last-good bytes, labelled amber on the wire.
    ServeAmber,
    /// Serve last-good bytes, labelled amber AND warned.
    ServeAmberWarn,
    /// Refuse: the existing 503 `catching-up` contract.
    Shed,
}

impl ReasonLabel for FreshnessVerdict {
    const ALL: &'static [Self] = &[
        FreshnessVerdict::Serve,
        FreshnessVerdict::ServeAmber,
        FreshnessVerdict::ServeAmberWarn,
        FreshnessVerdict::Shed,
    ];

    fn label(&self) -> &'static str {
        match self {
            FreshnessVerdict::Serve => "serve",
            FreshnessVerdict::ServeAmber => "serve_amber",
            FreshnessVerdict::ServeAmberWarn => "serve_amber_warn",
            FreshnessVerdict::Shed => "shed",
        }
    }
}

impl FreshnessVerdict {
    /// True for the two amber arms — the ones a floor row must never reach.
    pub fn is_amber(self) -> bool {
        matches!(
            self,
            FreshnessVerdict::ServeAmber | FreshnessVerdict::ServeAmberWarn
        )
    }
}

/// The policy matrix, as one total function.
///
/// | ReadClass      | Simulacra / Bootstrap | Coordinated / Enforced |
/// |----------------|-----------------------|------------------------|
/// | Knowledge      | `AmberOk 24h`         | `AmberOk 24h`          |
/// | ValueCoupled   | `AmberOk 24h`         | `AmberWarn 15m`        |
/// | Authority      | `GreenOnly`           | `GreenOnly`            |
/// | CounterEvidence| `GreenOnly`           | `GreenOnly`            |
///
/// The floor rows are written as a single stage-blind arm ON PURPOSE: there is
/// no place in this function where a stage could cheapen them, so the
/// invariant is structural and the property test is a proof rather than a
/// reminder.
///
/// **Contract test:** [`tests::matrix_table_matches_the_operator_confirmed_defaults`].
pub fn requirement(class: ReadClass, stage: NetworkStage) -> Requirement {
    match class {
        // FLOOR: stage is not consulted. See ReadClass::is_floor.
        ReadClass::Authority | ReadClass::CounterEvidence => Requirement::GreenOnly,
        ReadClass::Knowledge => Requirement::AmberOk {
            max_age: KNOWLEDGE_MAX_AGE,
        },
        ReadClass::ValueCoupled => match stage {
            NetworkStage::Simulacra | NetworkStage::Bootstrap => Requirement::AmberOk {
                max_age: VALUE_COUPLED_MAX_AGE_PERMISSIVE,
            },
            NetworkStage::Coordinated | NetworkStage::Enforced => Requirement::AmberWarn {
                max_age: VALUE_COUPLED_MAX_AGE_COORDINATED,
            },
        },
    }
}

/// `requirement(class, stage) × available` → the verdict.
///
/// - `Available::None` sheds for EVERY class and stage — there is nothing to
///   serve, and inventing one is exactly the dishonesty C4 names.
/// - `Available::Green` always serves: a live witness satisfies every
///   requirement, floor rows included.
/// - `Available::Amber { age }` serves only when the requirement admits amber
///   AND `age` has not passed `max_age`. The boundary is INCLUSIVE
///   (`age == max_age` still serves); strictly past it sheds.
///
/// **Contract tests:** [`tests::floor_classes_never_yield_amber_over_the_full_product`],
/// [`tests::amber_strictly_past_max_age_sheds`],
/// [`tests::nothing_available_sheds_for_every_class_and_stage`].
pub fn verdict(class: ReadClass, stage: NetworkStage, available: Available) -> FreshnessVerdict {
    match available {
        Available::None => FreshnessVerdict::Shed,
        Available::Green => FreshnessVerdict::Serve,
        Available::Amber { age } => match requirement(class, stage) {
            Requirement::GreenOnly => FreshnessVerdict::Shed,
            Requirement::AmberOk { max_age } => {
                if age > max_age {
                    FreshnessVerdict::Shed
                } else {
                    FreshnessVerdict::ServeAmber
                }
            }
            Requirement::AmberWarn { max_age } => {
                if age > max_age {
                    FreshnessVerdict::Shed
                } else {
                    FreshnessVerdict::ServeAmberWarn
                }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reason::{
        assert_reason_labels_conformant, assert_reason_labels_discriminating,
        assert_reason_labels_stable,
    };

    const DAY: Duration = Duration::from_secs(86_400);
    const FIFTEEN_MIN: Duration = Duration::from_secs(900);

    /// The operator-confirmed matrix, written as a TABLE of independently
    /// stated literals — not derived from `requirement` itself, so this
    /// measures the policy rather than mirroring it.
    #[test]
    fn matrix_table_matches_the_operator_confirmed_defaults() {
        let table: &[(ReadClass, NetworkStage, Requirement)] = &[
            // Knowledge — 24h amber at every stage.
            (
                ReadClass::Knowledge,
                NetworkStage::Simulacra,
                Requirement::AmberOk { max_age: DAY },
            ),
            (
                ReadClass::Knowledge,
                NetworkStage::Bootstrap,
                Requirement::AmberOk { max_age: DAY },
            ),
            (
                ReadClass::Knowledge,
                NetworkStage::Coordinated,
                Requirement::AmberOk { max_age: DAY },
            ),
            (
                ReadClass::Knowledge,
                NetworkStage::Enforced,
                Requirement::AmberOk { max_age: DAY },
            ),
            // ValueCoupled — 24h unremarked below Coordinated, 15m WARNED at
            // and above it. The one genuinely stage-priced row.
            (
                ReadClass::ValueCoupled,
                NetworkStage::Simulacra,
                Requirement::AmberOk { max_age: DAY },
            ),
            (
                ReadClass::ValueCoupled,
                NetworkStage::Bootstrap,
                Requirement::AmberOk { max_age: DAY },
            ),
            (
                ReadClass::ValueCoupled,
                NetworkStage::Coordinated,
                Requirement::AmberWarn {
                    max_age: FIFTEEN_MIN,
                },
            ),
            (
                ReadClass::ValueCoupled,
                NetworkStage::Enforced,
                Requirement::AmberWarn {
                    max_age: FIFTEEN_MIN,
                },
            ),
            // Authority — GreenOnly, floor.
            (
                ReadClass::Authority,
                NetworkStage::Simulacra,
                Requirement::GreenOnly,
            ),
            (
                ReadClass::Authority,
                NetworkStage::Bootstrap,
                Requirement::GreenOnly,
            ),
            (
                ReadClass::Authority,
                NetworkStage::Coordinated,
                Requirement::GreenOnly,
            ),
            (
                ReadClass::Authority,
                NetworkStage::Enforced,
                Requirement::GreenOnly,
            ),
            // CounterEvidence — GreenOnly, floor.
            (
                ReadClass::CounterEvidence,
                NetworkStage::Simulacra,
                Requirement::GreenOnly,
            ),
            (
                ReadClass::CounterEvidence,
                NetworkStage::Bootstrap,
                Requirement::GreenOnly,
            ),
            (
                ReadClass::CounterEvidence,
                NetworkStage::Coordinated,
                Requirement::GreenOnly,
            ),
            (
                ReadClass::CounterEvidence,
                NetworkStage::Enforced,
                Requirement::GreenOnly,
            ),
        ];

        for &(class, stage, expected) in table {
            assert_eq!(
                requirement(class, stage),
                expected,
                "matrix row {class:?} x {stage:?} does not match the operator-confirmed default"
            );
        }
        assert_eq!(
            table.len(),
            ReadClass::ALL.len() * NetworkStage::ALL.len(),
            "the table must cover the FULL class x stage product, not a sample"
        );
    }

    /// THE floor property. Over the entire `ReadClass × NetworkStage ×
    /// Available` product (with the amber axis sampled across the whole age
    /// range that matters, including zero and beyond every configured
    /// `max_age`), a floor row never yields an amber verdict.
    ///
    /// This is the mirror of `pricer.rs`'s
    /// `every_in_crate_pricer_forces_full_chain_across_the_entire_product_space`:
    /// a floor is only a floor if no point in the product space escapes it.
    #[test]
    fn floor_classes_never_yield_amber_over_the_full_product() {
        let ages = [
            Duration::ZERO,
            Duration::from_secs(1),
            FIFTEEN_MIN - Duration::from_secs(1),
            FIFTEEN_MIN,
            FIFTEEN_MIN + Duration::from_secs(1),
            DAY - Duration::from_secs(1),
            DAY,
            DAY + Duration::from_secs(1),
            Duration::from_secs(10 * 365 * 86_400),
        ];
        let mut floor_points = 0u32;
        let mut checked = 0u32;

        for &class in &ReadClass::ALL {
            for &stage in &NetworkStage::ALL {
                let mut availables = vec![Available::Green, Available::None];
                availables.extend(ages.iter().map(|&age| Available::Amber { age }));
                for available in availables {
                    let v = verdict(class, stage, available);
                    checked += 1;
                    if class.is_floor() {
                        floor_points += 1;
                        assert!(
                            !v.is_amber(),
                            "FLOOR VIOLATION: {class:?} at {stage:?} with {available:?} \
                             yielded {v:?} — a floor row must never be answered amber"
                        );
                        // And the floor's requirement is stage-blind.
                        assert_eq!(
                            requirement(class, stage),
                            Requirement::GreenOnly,
                            "floor row {class:?} was stage-priced at {stage:?}"
                        );
                    }
                }
            }
        }

        assert_eq!(
            checked,
            (ReadClass::ALL.len() * NetworkStage::ALL.len() * (2 + ages.len())) as u32,
            "must walk the full product, not a sample"
        );
        assert_eq!(
            floor_points,
            (2 * NetworkStage::ALL.len() * (2 + ages.len())) as u32,
            "both floor rows must be exercised at every stage and availability"
        );
    }

    /// A floor row with a live witness still SERVES — `GreenOnly` is a floor on
    /// freshness, not a refusal to answer. Stated separately so the property
    /// above cannot be satisfied by a predicate that just sheds everything.
    #[test]
    fn floor_classes_still_serve_a_live_green_answer() {
        for &class in &[ReadClass::Authority, ReadClass::CounterEvidence] {
            for &stage in &NetworkStage::ALL {
                assert_eq!(
                    verdict(class, stage, Available::Green),
                    FreshnessVerdict::Serve
                );
            }
        }
    }

    /// Amber expires. The boundary is inclusive: `age == max_age` still serves,
    /// one second past it sheds.
    #[test]
    fn amber_strictly_past_max_age_sheds() {
        // Knowledge, 24h.
        assert_eq!(
            verdict(
                ReadClass::Knowledge,
                NetworkStage::Bootstrap,
                Available::Amber { age: DAY }
            ),
            FreshnessVerdict::ServeAmber
        );
        assert_eq!(
            verdict(
                ReadClass::Knowledge,
                NetworkStage::Bootstrap,
                Available::Amber {
                    age: DAY + Duration::from_secs(1)
                }
            ),
            FreshnessVerdict::Shed
        );

        // ValueCoupled at Coordinated, 15m, WARNED up to the boundary.
        assert_eq!(
            verdict(
                ReadClass::ValueCoupled,
                NetworkStage::Coordinated,
                Available::Amber { age: FIFTEEN_MIN }
            ),
            FreshnessVerdict::ServeAmberWarn
        );
        assert_eq!(
            verdict(
                ReadClass::ValueCoupled,
                NetworkStage::Coordinated,
                Available::Amber {
                    age: FIFTEEN_MIN + Duration::from_secs(1)
                }
            ),
            FreshnessVerdict::Shed
        );

        // The same age that is FINE at Bootstrap sheds at Enforced — the
        // graduation is real, not decorative.
        let sixteen_min = Duration::from_secs(16 * 60);
        assert_eq!(
            verdict(
                ReadClass::ValueCoupled,
                NetworkStage::Bootstrap,
                Available::Amber { age: sixteen_min }
            ),
            FreshnessVerdict::ServeAmber
        );
        assert_eq!(
            verdict(
                ReadClass::ValueCoupled,
                NetworkStage::Enforced,
                Available::Amber { age: sixteen_min }
            ),
            FreshnessVerdict::Shed
        );
    }

    /// Nothing available sheds everywhere — including for the classes whose
    /// requirement is the most permissive. `Available::None` is not a
    /// freshness level; it is the absence of anything to be fresh about.
    #[test]
    fn nothing_available_sheds_for_every_class_and_stage() {
        let mut checked = 0u32;
        for &class in &ReadClass::ALL {
            for &stage in &NetworkStage::ALL {
                assert_eq!(
                    verdict(class, stage, Available::None),
                    FreshnessVerdict::Shed,
                    "{class:?} at {stage:?} must shed when nothing is available"
                );
                checked += 1;
            }
        }
        assert_eq!(checked, 16, "full 4x4 product, not a sample");
    }

    /// Green serves for EVERY class and stage — the other total row.
    #[test]
    fn a_live_green_answer_serves_for_every_class_and_stage() {
        for &class in &ReadClass::ALL {
            for &stage in &NetworkStage::ALL {
                assert_eq!(
                    verdict(class, stage, Available::Green),
                    FreshnessVerdict::Serve
                );
            }
        }
    }

    #[test]
    fn network_stage_parses_exact_lowercase_only() {
        assert_eq!(
            "simulacra".parse::<NetworkStage>(),
            Ok(NetworkStage::Simulacra)
        );
        assert_eq!(
            "bootstrap".parse::<NetworkStage>(),
            Ok(NetworkStage::Bootstrap)
        );
        assert_eq!(
            "coordinated".parse::<NetworkStage>(),
            Ok(NetworkStage::Coordinated)
        );
        assert_eq!(
            "enforced".parse::<NetworkStage>(),
            Ok(NetworkStage::Enforced)
        );

        // No case folding, no aliasing, no whitespace tolerance.
        for bad in [
            "Simulacra",
            "BOOTSTRAP",
            "dev",
            "prod",
            " enforced",
            "enforced ",
            "",
            "coordinated\n",
        ] {
            assert_eq!(
                bad.parse::<NetworkStage>(),
                Err(()),
                "{bad:?} must NOT parse — an unrecognized declaration is an error, not a guess"
            );
        }
    }

    #[test]
    fn network_stage_default_is_bootstrap_not_simulacra() {
        assert_eq!(NetworkStage::default(), NetworkStage::Bootstrap);
    }

    #[test]
    fn network_stage_label_round_trips_through_from_str() {
        for &stage in &NetworkStage::ALL {
            assert_eq!(stage.label().parse::<NetworkStage>(), Ok(stage));
        }
    }

    /// Ordering is semantic, checked against a hand-written reference ranking
    /// (NOT derived from `Ord`), exactly as the authority does — so a
    /// reordering of either enum shows up here.
    #[test]
    fn network_stage_ordering_matches_declared_semantics() {
        fn reference_rank(stage: NetworkStage) -> u8 {
            match stage {
                NetworkStage::Simulacra => 0,
                NetworkStage::Bootstrap => 1,
                NetworkStage::Coordinated => 2,
                NetworkStage::Enforced => 3,
            }
        }
        for &a in &NetworkStage::ALL {
            for &b in &NetworkStage::ALL {
                assert_eq!(a.cmp(&b), reference_rank(a).cmp(&reference_rank(b)));
            }
        }
    }

    /// The mirror is checked, not asserted. `elohim-storage`'s
    /// `trust/stage.rs` is the AUTHORITY for `NetworkStage`; this crate cannot
    /// depend on it (leaf boundary — see `boundary_tests`), so the parity claim
    /// is verified by reading the authority's source text for its variant list.
    ///
    /// Out-of-tree (a vendored/published checkout of just this crate) the
    /// authority is absent and the test returns early rather than failing — the
    /// claim is about THIS repo's coherence, and a published leaf crate has no
    /// sibling to diverge from.
    #[test]
    fn network_stage_variant_names_match_storage_trust_stage() {
        let storage_crate =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../elohim/elohim-storage");
        if !storage_crate.is_dir() {
            // Out-of-tree checkout: nothing to mirror.
            return;
        }
        let stage_rs = storage_crate.join("src/trust/stage.rs");
        let src = std::fs::read_to_string(&stage_rs).unwrap_or_else(|e| {
            panic!(
                "the authority for NetworkStage must exist in-tree at {}: {e}",
                stage_rs.display()
            )
        });

        // Take the body of `pub enum NetworkStage { ... }` and collect the
        // capitalized identifiers that head a line — the variant names.
        let body = src
            .split_once("pub enum NetworkStage {")
            .and_then(|(_, rest)| rest.split_once("\n}"))
            .map(|(body, _)| body.to_string())
            .expect("could not locate `pub enum NetworkStage {` in the authority source");

        let mut authority_variants: Vec<String> = Vec::new();
        for line in body.lines() {
            let t = line.trim().trim_end_matches(',');
            if t.is_empty() || t.starts_with("//") || t.starts_with("#[") {
                continue;
            }
            if t.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                && t.chars().all(|c| c.is_ascii_alphanumeric())
            {
                authority_variants.push(t.to_string());
            }
        }

        let mirror_variants: Vec<String> =
            NetworkStage::ALL.iter().map(|s| format!("{s:?}")).collect();

        assert_eq!(
            authority_variants, mirror_variants,
            "NetworkStage has DRIFTED from its authority \
             (elohim/elohim-storage/src/trust/stage.rs). The authority wins: \
             fix this mirror, not the assertion."
        );
    }

    /// C8 conformance for the verdict vocabulary, plus the pinned label set —
    /// a rename or reorder is a dashboard-legend break and must be a decision.
    #[test]
    fn freshness_verdict_is_reason_label_conformant() {
        assert_reason_labels_conformant::<FreshnessVerdict>();
        assert_reason_labels_discriminating::<FreshnessVerdict>();
        assert_reason_labels_stable::<FreshnessVerdict>(&[
            "serve",
            "serve_amber",
            "serve_amber_warn",
            "shed",
        ]);
    }

    /// The class labels the wire and the metric share. `value` (not
    /// `value_coupled`) is deliberate: it is the token
    /// `x-elohim-freshness-class` carries.
    #[test]
    fn read_class_labels_are_the_wire_vocabulary() {
        assert_eq!(ReadClass::Knowledge.label(), "knowledge");
        assert_eq!(ReadClass::ValueCoupled.label(), "value");
        assert_eq!(ReadClass::Authority.label(), "authority");
        assert_eq!(ReadClass::CounterEvidence.label(), "counter_evidence");
        // Metric-safe: the conservative intersection Prometheus/Loki/grep carry.
        for &c in &ReadClass::ALL {
            assert!(c
                .label()
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_'));
        }
    }

    /// `is_floor` and the matrix must agree — the predicate and its
    /// self-description cannot drift apart.
    #[test]
    fn is_floor_agrees_with_the_matrix() {
        for &class in &ReadClass::ALL {
            for &stage in &NetworkStage::ALL {
                let green_only = requirement(class, stage) == Requirement::GreenOnly;
                assert_eq!(
                    green_only,
                    class.is_floor(),
                    "{class:?} at {stage:?}: is_floor()={} but GreenOnly={}",
                    class.is_floor(),
                    green_only
                );
            }
        }
    }
}
