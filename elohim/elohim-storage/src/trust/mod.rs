//! The trust-gradient seam (L5) — landed **INERT**.
//!
//! Design: `genesis/docs/superpowers/plans/2026-08-08-head-plane-trust-gradient-program-plan.md`
//! §3 L5. Canonical principle this composes from (do not fork):
//! `genesis/docs/content/elohim-protocol/architecture/trust-as-efficiency-signal.md`.
//!
//! "Inert" is the contract: every caller of `authorize_reach_for_human`
//! passes [`TrustGradient::inert()`] and gets EXACTLY today's behavior —
//! `NetworkStage::Bootstrap`, [`pricer::InertPricer`] (always `FullChain`),
//! no memo store (always re-verify, never cached). `standing_projector`
//! (T19) is the writer that eventually lights up live gradient behavior;
//! until it lands, standing is `Unknown` everywhere and this module MUST
//! NOT claim otherwise (plan §2, evidence correction #1).
//!
//! Module map:
//! - [`stage`] — `NetworkStage`, `StakesProvenance`, `StakesResolver` (PURE).
//! - [`pricer`] — `VerificationPricer`, the floor safety invariant (PURE).
//! - [`snapshot`] — `HeadSetSnapshot`: mint, verify (C5 re-derivation + the
//!   C2 epoch guard), delta, and the dag-cbor CID.
//! - [`snapshot_source`] — where a mint's inputs come from, and the (honestly
//!   refusing) signer seam.
//! - [`memo`] — `VerificationMemo` type, `VerificationMemoStore` trait, and
//!   [`memo::InMemoryVerificationMemoStore`] (process-lifetime plumbing only,
//!   T7 — nothing calls `get`/`put` yet; T9 wires consumption).

pub mod memo;
pub mod pricer;
pub mod snapshot;
pub mod snapshot_source;
pub mod stage;

pub use memo::{InMemoryVerificationMemoStore, VerificationMemo, VerificationMemoStore};
pub use pricer::{
    FloorClass, InertPricer, PricedVerification, PricingInput, PricingReason, VerificationDepth,
    VerificationPricer,
};
pub use snapshot::{
    compare_epochs, head_set_delta, mint_snapshot, verify_snapshot, verify_wire_snapshot,
    verify_wire_snapshot_for_readiness, CarriedSnapshot, EpochOrdering, HeadDivergence,
    HeadSetDelta, HeadSetEntry, HeadSetSnapshot, MintedSnapshot, SignerEdgeSet, SnapshotAcceptance,
    SnapshotProof, SnapshotRefusal, SnapshotVerdict, TrustEpoch, VerifyInput,
};
pub use snapshot_source::{
    FixedSnapshotSource, LocalHeadPlaneSnapshotSource, LocalKeySnapshotSigner, SnapshotInputs,
    SnapshotSigner, SnapshotSource, UnavailableSnapshotSigner,
};
pub use stage::{FixedStakesResolver, NetworkStage, StakesProvenance, StakesResolver};

/// Borrowed-context bundle for the trust-gradient seam — modeled on
/// [`crate::services::head_adoption::AdoptContext`]. IoC here is NOT via the
/// `Services` registry (all-concrete); callers borrow exactly the seam
/// pieces they need for the duration of one call, same as `AdoptContext`.
pub struct TrustGradient<'a> {
    pub stakes: &'a dyn StakesResolver,
    pub pricer: &'a dyn VerificationPricer,
    /// `None` means "no memo store wired" — every lookup misses, so the
    /// caller always falls through to full verification. This is the
    /// INERT default; a real store arrives `Arc`'d process-lifetime from
    /// `main.rs` in T7.
    pub memo: Option<&'a dyn VerificationMemoStore>,
}

impl TrustGradient<'_> {
    /// The behavior-neutral landing shape: `Bootstrap` stage (never
    /// `Simulacra` — see [`stage::NetworkStage`]'s doc), [`InertPricer`]
    /// (always `FullChain`), no memo store. Every caller of
    /// `authorize_reach_for_human` passes this today; swapping any caller to
    /// a non-inert `TrustGradient` is explicitly a LATER task's job (T9
    /// wires memo, T10 wires real stakes, T19 wires live standing).
    pub fn inert() -> TrustGradient<'static> {
        static PRICER: InertPricer = InertPricer;
        TrustGradient {
            stakes: stage::inert_stakes_resolver(),
            pricer: &PRICER,
            memo: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inert_resolves_bootstrap_stage_with_bootstrap_default_provenance() {
        let trust = TrustGradient::inert();
        let (stage, provenance) = trust.stakes.stage_for("any-scope");
        assert_eq!(stage, NetworkStage::Bootstrap);
        assert_eq!(provenance, StakesProvenance::BootstrapDefault);
    }

    #[test]
    fn inert_prices_full_chain_with_pricer_inert_reason() {
        use crate::services::standing::Standing;
        use elohim_epr::Reach;

        let trust = TrustGradient::inert();
        let priced = trust.pricer.price(&PricingInput {
            stage: NetworkStage::Bootstrap,
            floor: FloorClass::None,
            reach: Reach::Commons,
            standing: Standing::Unknown,
        });
        assert_eq!(priced.depth, VerificationDepth::FullChain);
        assert_eq!(priced.reason, PricingReason::PricerInert);
    }

    #[test]
    fn inert_has_no_memo_store() {
        let trust = TrustGradient::inert();
        assert!(trust.memo.is_none());
    }
}
