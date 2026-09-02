//! ark-core — the pure half of the elohim compute envelope (tevah).
//!
//! Spec: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md §3, §5.1, §6, §8.
//! No I/O and no async runtime live here; `ark-supervisor` implements the traits in `sink` and
//! is the only ARK construction site for sidecar stores.

pub mod berth;
pub mod exit;
pub mod intent;
pub mod lifecycle;
pub mod manifest;
pub mod passport;
pub mod rea;
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
pub use lifecycle::{step, Action, ChildState, Event, IncidentCloseKind};
pub use manifest::{ArtifactRef, ChildPolicy, ChildSpec, Probe, ProcessKind, RuntimeManifest};
pub use passport::{EffectiveTier, Passport, ProcessPassport, PASSPORT_KIND};
pub use rea::{
    ReaProjectionError, RuntimeScope, INCIDENT_SPEC_ID, INCIDENT_SPEC_VERSION, RUNTIME_TAG_DEATH,
    RUNTIME_TAG_GIVE_UP, RUNTIME_TAG_INCIDENT_CLOSED, RUNTIME_TAG_KILL, RUNTIME_TAG_RESTART,
    RUNTIME_TAG_SPAWN, RUNTIME_TAG_STOP, UNIDENTIFIED_AGENT, UNIT_DEATHS, UNIT_PROCESS_MS,
};
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
    /// Every module's source, read at compile time so the scan below needs no reader.
    ///
    /// `boundary_scan_covers_every_module` keeps this list honest as modules are added.
    const SOURCES: &[(&str, &str)] = &[
        ("berth.rs", include_str!("berth.rs")),
        ("exit.rs", include_str!("exit.rs")),
        ("intent.rs", include_str!("intent.rs")),
        ("lib.rs", include_str!("lib.rs")),
        ("lifecycle.rs", include_str!("lifecycle.rs")),
        ("manifest.rs", include_str!("manifest.rs")),
        ("passport.rs", include_str!("passport.rs")),
        ("rea.rs", include_str!("rea.rs")),
        ("ring.rs", include_str!("ring.rs")),
        ("sample.rs", include_str!("sample.rs")),
        ("sink.rs", include_str!("sink.rs")),
        ("tally.rs", include_str!("tally.rs")),
        ("verdict.rs", include_str!("verdict.rs")),
        ("witness.rs", include_str!("witness.rs")),
    ];

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

    /// The pure core touches no filesystem — not directly, and not through the substrate
    /// crate it now depends on.
    ///
    /// `elohim-epr-rea` is taken `default-features = false`, which drops its `sidecar`
    /// feature, so its stores are not compiled here at all. This test refuses the reach at
    /// SOURCE level too, because a dependency-graph fact is only a boundary while nobody
    /// re-enables the feature — and because a source-level scan names the offending line.
    ///
    /// The needles are assembled at run time from split literals: this test's own module is
    /// one of the files it reads, so a contiguous literal here would make the guard fail on
    /// itself.
    #[test]
    fn no_fs_in_pure_core() {
        let denied: [(String, &str); 4] = [
            (
                format!("std{}fs", "::"),
                "the supervisor owns every byte that touches a disk",
            ),
            (
                format!("File{}", "::"),
                "opening a handle is I/O, and I/O is the supervisor's",
            ),
            (
                format!("{}{}", "Sidecar", "FlowStore"),
                "the spool is constructed in ark-supervisor and nowhere else",
            ),
            (
                format!("{}{}", "Sidecar", "ActorStore"),
                "the spool is constructed in ark-supervisor and nowhere else",
            ),
        ];

        for (file, source) in SOURCES {
            for (needle, why) in &denied {
                assert!(
                    !source.contains(needle.as_str()),
                    "ark-core src/{file} names `{needle}` — {why}"
                );
            }
        }
    }

    /// A module added without an entry in [`SOURCES`] would be invisible to the scan above,
    /// which is how a purity boundary quietly stops covering the code it exists for.
    #[test]
    fn boundary_scan_covers_every_module() {
        for line in include_str!("lib.rs")
            .lines()
            .filter(|line| line.starts_with("pub mod "))
        {
            let module = line
                .trim_start_matches("pub mod ")
                .trim_end_matches(';')
                .trim();
            let expected = format!("{module}.rs");
            assert!(
                SOURCES.iter().any(|(name, _)| *name == expected),
                "src/{expected} is not scanned by the purity boundary"
            );
        }
        assert!(SOURCES.iter().any(|(name, _)| *name == "lib.rs"));
    }
}
