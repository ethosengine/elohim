//! `NetworkStage` — the trust-gradient's declared-stakes axis.
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-08-head-plane-trust-gradient-program-plan.md`
//! §3 L5; canonical principle:
//! `genesis/docs/content/elohim-protocol/architecture/trust-as-efficiency-signal.md`.
//!
//! PURE module — no diesel, no tokio, no clock, no env. A stage is a value a
//! [`StakesResolver`] hands you; nothing in this file reads config, a manifest,
//! or a wall clock to produce one.

use std::sync::OnceLock;

/// The network's declared operating stage — the axis every verification-cost
/// decision in `pricer.rs` is priced against.
///
/// Ordering is SEMANTIC: `Simulacra < Bootstrap < Coordinated < Enforced`,
/// from cheapest/most-permissive to strictest/most-guarded. The derived
/// `Ord` below relies on declaration order matching that semantic order —
/// pinned independently (not just self-consistently) by
/// [`tests::stage_ordering_matches_declared_semantics`], which checks the
/// full 4×4 product against a hand-written reference ranking.
///
/// `Simulacra` (dev/staging/genesis fixtures, per the head-plane
/// trust-gradient program plan §1) is **never** reached by inference or
/// fallback — only an explicit, positive declaration selects it (T10's
/// `ManifestStakesResolver` / `ELOHIM_NETWORK_STAKES` reader, out of scope
/// here). Absent/unknown config resolves to [`NetworkStage::Bootstrap`]
/// (fail-closed: toward MORE scrutiny, never less — see plan §2.2, evidence
/// correction #2).
///
/// `NetworkStage` must never derive from any `DEV_MODE` flag — neither
/// elohim-storage's inert one (`p2p/mod.rs`) nor doorway's live,
/// auth-permissive one (`config.rs`). Both are orthogonal runtime toggles;
/// neither is the network's declared trust stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NetworkStage {
    /// Dev/staging/genesis fixtures — cheapest, most permissive. Reached
    /// ONLY by explicit declaration; see the type doc.
    Simulacra,
    /// The fail-closed default. Structural validators only (Stage 1 per
    /// `project_bootstrap_to_elohim_security_gradient`).
    Bootstrap,
    /// Elohim-coordinated trust (Stage 2).
    Coordinated,
    /// Full elohim enforcement (Stage 3) — the live, generational,
    /// human-centric design the gradient exists to price for.
    Enforced,
}

impl Default for NetworkStage {
    /// The fail-closed default is `Bootstrap`, never `Simulacra`. Written by
    /// hand (not `#[derive(Default)]`, which would require an explicit
    /// `#[default]` variant marker anyway) so the choice is a decision this
    /// file states, not an accident of declaration order.
    fn default() -> Self {
        NetworkStage::Bootstrap
    }
}

impl NetworkStage {
    /// All four stages, in their declared (== semantic) order.
    pub const ALL: [NetworkStage; 4] = [
        NetworkStage::Simulacra,
        NetworkStage::Bootstrap,
        NetworkStage::Coordinated,
        NetworkStage::Enforced,
    ];
}

/// EXACT lowercase-variant-name parsing — the single vocabulary both
/// `ManifestStakesResolver` (T10, `trust::manifest_resolver`) and the
/// `PUT /admin/seed/network-stakes` route validator parse against, so the
/// wire contract can never drift between "what the resolver accepts" and
/// "what the route accepts." No case-folding, no aliasing ("dev" is not
/// `Simulacra`) — an unrecognized string is `Err`, never a best-effort
/// guess. Still PURE: no I/O, just string matching.
impl std::str::FromStr for NetworkStage {
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

/// Provenance of a resolved [`NetworkStage`] — travels with every stage
/// verdict so a downstream consumer (metrics, audit trail, `.epr-meta`
/// review) can distinguish "declared in a manifest" from "declared via
/// operator config" from "nothing declared anywhere, fail-closed default
/// applied."
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StakesProvenance {
    /// Declared in a standing-policy manifest, keyed by its CID. T10 wires
    /// the `ManifestRegistry` read that produces this; the variant exists
    /// now so the type is stable across that landing.
    Manifest { cid: String },
    /// Declared via operator config (`ELOHIM_NETWORK_STAKES` — T10).
    OperatorConfig,
    /// No declaration found anywhere — [`NetworkStage::Bootstrap`] (the
    /// fail-closed default) was applied.
    BootstrapDefault,
}

/// Resolves the declared [`NetworkStage`] for a scope, with the
/// [`StakesProvenance`] that produced it.
///
/// `ManifestStakesResolver` (reading the standing-policy manifest's stakes
/// field plus `ELOHIM_NETWORK_STAKES`) is T10 — out of scope here.
/// [`FixedStakesResolver`] is the only implementor this task ships, for
/// tests and for [`crate::trust::TrustGradient::inert`].
pub trait StakesResolver {
    /// `scope` names whatever a real resolver keys stakes by (an
    /// `h_app_id`, a manifest CID, ...). This task declares no obligation on
    /// its shape; T10's resolver refines it against the real manifest key.
    fn stage_for(&self, scope: &str) -> (NetworkStage, StakesProvenance);
}

/// A fixed-answer test double — always returns the same declared stage
/// regardless of `scope`. Used by tests and by the inert landing shape.
#[derive(Debug, Clone)]
pub struct FixedStakesResolver {
    pub stage: NetworkStage,
    pub provenance: StakesProvenance,
}

impl FixedStakesResolver {
    /// The inert-landing constructor: `Bootstrap` via `BootstrapDefault` —
    /// today's behavior, stated as data instead of inferred.
    pub fn bootstrap_default() -> Self {
        Self {
            stage: NetworkStage::Bootstrap,
            provenance: StakesProvenance::BootstrapDefault,
        }
    }
}

impl StakesResolver for FixedStakesResolver {
    fn stage_for(&self, _scope: &str) -> (NetworkStage, StakesProvenance) {
        (self.stage, self.provenance.clone())
    }
}

/// Process-lifetime singleton backing [`crate::trust::TrustGradient::inert`]
/// — avoids allocating a fresh resolver on every call while keeping the
/// borrowed-context (`&dyn StakesResolver`) shape `AdoptContext<'a>` uses.
pub(crate) fn inert_stakes_resolver() -> &'static FixedStakesResolver {
    static RESOLVER: OnceLock<FixedStakesResolver> = OnceLock::new();
    RESOLVER.get_or_init(FixedStakesResolver::bootstrap_default)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The stage-ordering property test: the full 4×4 product of
    /// `NetworkStage` against itself, checked against a hand-written
    /// reference ranking that is NOT derived from `Ord` — so this catches a
    /// future reordering of the enum's declaration (which would silently
    /// change the derived `Ord`) as well as any accidental use of a
    /// different comparison elsewhere.
    #[test]
    fn stage_ordering_matches_declared_semantics() {
        fn reference_rank(stage: NetworkStage) -> u8 {
            match stage {
                NetworkStage::Simulacra => 0,
                NetworkStage::Bootstrap => 1,
                NetworkStage::Coordinated => 2,
                NetworkStage::Enforced => 3,
            }
        }

        let mut pairs_checked = 0u32;
        for &a in &NetworkStage::ALL {
            for &b in &NetworkStage::ALL {
                let expected = reference_rank(a).cmp(&reference_rank(b));
                assert_eq!(
                    a.cmp(&b),
                    expected,
                    "NetworkStage::cmp disagrees with the semantic reference ranking for {a:?} vs {b:?}"
                );
                pairs_checked += 1;
            }
        }
        assert_eq!(
            pairs_checked, 16,
            "must cover the full 4×4 product, not a sample"
        );
    }

    #[test]
    fn simulacra_is_strictly_cheapest() {
        for stage in [
            NetworkStage::Bootstrap,
            NetworkStage::Coordinated,
            NetworkStage::Enforced,
        ] {
            assert!(NetworkStage::Simulacra < stage);
        }
    }

    #[test]
    fn enforced_is_strictly_strictest() {
        for stage in [
            NetworkStage::Simulacra,
            NetworkStage::Bootstrap,
            NetworkStage::Coordinated,
        ] {
            assert!(stage < NetworkStage::Enforced);
        }
    }

    #[test]
    fn default_is_bootstrap_never_simulacra() {
        assert_eq!(NetworkStage::default(), NetworkStage::Bootstrap);
        assert_ne!(NetworkStage::default(), NetworkStage::Simulacra);
    }

    #[test]
    fn fixed_resolver_reports_bootstrap_default_provenance() {
        let resolver = FixedStakesResolver::bootstrap_default();
        let (stage, provenance) = resolver.stage_for("any-scope");
        assert_eq!(stage, NetworkStage::Bootstrap);
        assert_eq!(provenance, StakesProvenance::BootstrapDefault);
    }

    #[test]
    fn fixed_resolver_ignores_scope_by_design() {
        // A "fixed" resolver's whole point is a constant answer — this
        // pins that it does not accidentally start reading `scope`.
        let resolver = FixedStakesResolver {
            stage: NetworkStage::Enforced,
            provenance: StakesProvenance::Manifest {
                cid: "bafyreitest".into(),
            },
        };
        let (a, _) = resolver.stage_for("scope-a");
        let (b, _) = resolver.stage_for("scope-b");
        assert_eq!(a, b);
    }

    #[test]
    fn inert_stakes_resolver_is_bootstrap() {
        let (stage, provenance) = inert_stakes_resolver().stage_for("ignored");
        assert_eq!(stage, NetworkStage::Bootstrap);
        assert_eq!(provenance, StakesProvenance::BootstrapDefault);
    }

    #[test]
    fn from_str_accepts_exact_lowercase_variant_names() {
        assert_eq!("simulacra".parse(), Ok(NetworkStage::Simulacra));
        assert_eq!("bootstrap".parse(), Ok(NetworkStage::Bootstrap));
        assert_eq!("coordinated".parse(), Ok(NetworkStage::Coordinated));
        assert_eq!("enforced".parse(), Ok(NetworkStage::Enforced));
    }

    #[test]
    fn from_str_rejects_case_variants_and_aliases() {
        // No case-folding, no "dev"/"staging" aliasing — an unrecognized
        // string is Err, never a best-effort guess.
        for bad in ["Simulacra", "SIMULACRA", "dev", "staging", "dev-mode", ""] {
            assert!(
                bad.parse::<NetworkStage>().is_err(),
                "{bad:?} must not parse as a NetworkStage"
            );
        }
    }
}
