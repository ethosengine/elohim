//! ark-core — the pure half of the elohim compute envelope (tevah).
//!
//! Spec: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md §3, §5.1, §6, §8.
//! No I/O and no async runtime live here; `ark-supervisor` implements the traits in `sink`.

pub mod berth;
pub mod exit;
pub mod intent;
pub mod lifecycle;
pub mod manifest;
pub mod passport;
pub mod ring;
pub mod sample;
pub mod sink;
pub mod tally;
pub mod verdict;
pub mod witness;

// Re-exports are uncommented by the task that creates each type.
pub use berth::{Berth, PassphraseSource};
pub use elohim_compute::{LimitOwner, Refusal};
pub use exit::{classify_readiness_outcome, ExitClass, ReadinessOutcome};
pub use intent::{Intent, IntentAction};
pub use manifest::{ArtifactRef, ChildPolicy, Probe, ProcessKind, ProcessSpec, RuntimeManifest};
pub use passport::{EffectiveTier, Passport, ProcessPassport, PASSPORT_KIND};
pub use ring::RingBuffer;
pub use sample::ProcessSample;
pub use sink::{Clock, SinkError, WitnessSink};
pub use tally::{DeathRecord, DeathTally};
pub use verdict::{
    BoundedBy, GiveUpReason, RestartContext, RestartGovernor, RestartGrant, RestartRequest,
    RestartVerdict,
};
pub use witness::{DeathWitness, Incident, IncidentClose, WitnessError, WITNESS_KIND};

#[cfg(test)]
mod boundary {
    /// The purity boundary, read from this crate's own manifest: a runtime or I/O
    /// dependency arriving here would let a pure decision do I/O.
    #[test]
    fn no_runtime_or_io_deps() {
        let toml = include_str!("../Cargo.toml");
        const DENIED: &[(&str, &str)] = &[
            (
                "tokio",
                "ark-core is plain data + decisions; the supervisor owns the runtime",
            ),
            ("nix", "syscalls are the supervisor's"),
            ("libc", "syscalls are the supervisor's"),
            ("diesel", "persistence is a storage concern"),
            ("rusqlite", "persistence is a storage concern"),
            (
                "libp2p",
                "the envelope has no swarm in v1 (spec §12 item 11)",
            ),
            ("iroh", "the envelope has no swarm in v1 (spec §12 item 11)"),
            ("reqwest", "no network in the envelope"),
            ("hyper", "no network in the envelope"),
            ("axum", "no network in the envelope"),
            ("hdk", "the envelope is below the DNA line"),
            ("hdi", "the envelope is below the DNA line"),
        ];
        for (pkg, why) in DENIED {
            let needle = format!("\n{pkg} ");
            let needle_eq = format!("\n{pkg}=");
            assert!(
                !toml.contains(&needle) && !toml.contains(&needle_eq),
                "elohim-ark-core declares `{pkg}` — {why}"
            );
        }
    }
}
