//! Core contracts for EPR-governed filesystem projections.
//!
//! `eprfs-core` is deliberately storage-agnostic and domain-agnostic. It models
//! how distributed EPR-backed data is projected into a filesystem tree, not what
//! that tree means.
//!
//! It is a pure value crate: no I/O, no runtime, no clock (rails R-E1 and R-E2
//! in the `boundary` tests below). Async functions only await an
//! [`EprfsStorage`], `DerivationIndex` or `Derive` port.

// Every `TreeError` variant that names an object carries its 96-byte inline CID; the
// `Unanswered` variant adds the object's path beside its boxed three parts and lands at 128
// bytes, one word over clippy's default. Boxing the CID would make every refusal allocate.
#![allow(
    clippy::result_large_err,
    reason = "TreeError retains its inline CID and path; boxing the CID would allocate on every refusal"
)]

pub mod address;
pub mod attestation;
pub mod awareness;
pub mod composition;
pub mod derive;
pub mod error;
pub mod level_walk;
pub mod meta;
pub mod projection;
pub mod reach;
pub mod storage;
pub mod tier;
pub mod tree;
pub mod view_decl;
pub mod walk;

#[cfg(test)]
mod test_support;

pub use address::{BlobCid, BlobLink, EprRef, ProjectionId};
pub use attestation::{AttestationDraft, AttestationKind};
pub use awareness::{
    BytePresence, EprCard, EprResiliency, LocalOverlayStatus, PeerVisibility, ProjectionAwareness,
    ProjectionAwarenessProvider, ProjectionEntryAwareness, VerificationStatus,
};
pub use composition::{CompositionGraph, CompositionNode, DerivationEdge, DerivationKind};
pub use derive::{
    AdoptedHead, DerivationBasis, DerivationBundle, DerivationIndex, DerivationKey,
    DerivationLookup, DerivationRecord, DerivationUnanswered, Derive, DeriveOutcome, Derived,
    DomainRoster, KeyInput, MethodDescriptor, ProjectedRoot, TrustPolicy, Witness,
};
pub use error::{EprfsError, Result};
pub use level_walk::{
    check_membership, resolve_links, walk_batched, LevelOutcome, LevelWalk, Membership,
};
pub use meta::{
    EprHeadCoupling, EprMetaGovernance, EprMetaRecord, EprMetaResolution, EprMetaSource,
    EprMetaSubject, GovernanceBinding, GovernancePolicyBinding, GovernanceRule,
    GovernanceRuleClass, GovernanceRulePredicate, GovernanceScope, GovernanceValidator,
};
pub use projection::{
    EntryKind, MaterializationPolicy, ProjectionEntry, ProjectionManifest, ProjectionPath,
    ProjectionRoot, ProjectionSource, ProjectionSourceKind, ProjectionStatus,
};
pub use reach::{
    AskPolicy, KeyCustody, OpennessAsk, ReachGate, ReachLevel, ReachReason, ReachRefusal, Requester,
};
pub use storage::{
    BatchAnswer, BatchItem, BatchOptions, BatchReport, BlobHandle, BlobPresence, BlobSink,
    CollectSink, EprRecord, EprfsStorage, FetchPolicy, SinkControl,
};
pub use tier::{
    ChainLimits, ChainStatsSnapshot, ForwardGate, ForwardPermit, ItemAbsence, OnBehalf, OverBudget,
    Proximity, ProximityOrder, TierAddress, TierChain, TierClass, TierConnector, TierCounts,
    TierId, TierOrder, TierRecord, TierSource,
};
pub use tree::{
    decode_tree, encode_tree, load_verified_tree, TreeEntry, TreeEntryKind, TreeError, TreeLimits,
    TreeNode, TreeResult, VerifiedTree,
};
pub use view_decl::TreeView;
pub use walk::{
    diff_trees, list_dir, read_path, walk, walk_collect, AbsentReason, Coverage, Depth, DirListing,
    LeafKind, ReadLeaf, TreeChange, TreePath, TreeWalk, View, WalkItem, WalkStats, WalkedNode,
    WalkedView,
};

/// Rails R-E1 and R-E2 (interfaces spec §1.3): eprfs-core's frozen dependency set, and no I/O,
/// runtime or clock in its source.
#[cfg(test)]
mod boundary {
    /// The manifest reader R-E1 here and R-E3 in eprfs-storage share (`boundary/manifest_deps.rs`).
    mod manifest_deps;

    /// Every module's source, read at compile time so the scan below needs no reader.
    /// `scan_covers_every_module` keeps this list honest as modules are added.
    const SOURCES: &[(&str, &str)] = &[
        ("address.rs", include_str!("address.rs")),
        (
            "boundary/manifest_deps.rs",
            include_str!("boundary/manifest_deps.rs"),
        ),
        ("attestation.rs", include_str!("attestation.rs")),
        ("awareness.rs", include_str!("awareness.rs")),
        ("composition.rs", include_str!("composition.rs")),
        ("derive.rs", include_str!("derive.rs")),
        ("error.rs", include_str!("error.rs")),
        ("level_walk.rs", include_str!("level_walk.rs")),
        ("lib.rs", include_str!("lib.rs")),
        ("meta.rs", include_str!("meta.rs")),
        ("projection.rs", include_str!("projection.rs")),
        ("reach.rs", include_str!("reach.rs")),
        ("storage.rs", include_str!("storage.rs")),
        ("tier.rs", include_str!("tier.rs")),
        ("tree.rs", include_str!("tree.rs")),
        ("view_decl.rs", include_str!("view_decl.rs")),
        ("walk.rs", include_str!("walk.rs")),
    ];

    /// The exact set eprfs-core may name, read from its own manifest. An exact set, not a
    /// deny-list: a needle scan misses `tokio.workspace = true` and trips on the `tokio`
    /// dev-dependency. Every form of a normal dependency counts, through the reader R-E3 shares:
    /// the `[dependencies]` table, a `[target.<cfg>.dependencies]` table, a dotted table
    /// `[dependencies.<name>]` in either form, and a rename by its package. Dev- and
    /// build-dependencies are not R-E1's. `elohim-seam-contracts` is admitted because ruling 7's
    /// shared type must be nameable by every seam, and it is a std/alloc leaf with zero
    /// first-party dependencies whose own boundary test keeps it one.
    ///
    /// Open licence question (C4 review, for the operator before 0.2.0 publishes): seam-contracts
    /// is AGPL-3.0 and this crate is Apache-2.0, and brit (MIT OR Apache-2.0) pins this crate from
    /// the registry. The admission above rests on the shared type, not on licence compatibility;
    /// the ruling (relicense the leaf, or move `Unanswered`/`ComeBack`/`Meanwhile` into a
    /// permissive one) is recorded here when it is made.
    #[test]
    fn frozen_dependency_set() {
        let manifest = include_str!("../Cargo.toml");
        let mut found: Vec<String> = manifest_deps::dependencies(manifest)
            .iter()
            .filter(|dep| dep.kind == manifest_deps::DepKind::Normal)
            .map(|dep| dep.package_name().to_string())
            .collect();
        found.sort();
        let mut expected = vec![
            "async-trait",
            "bytes",
            "cid",
            "elohim-seam-contracts",
            "multihash-codetable",
            "serde",
            "serde_bytes",
            "serde_ipld_dagcbor",
            "serde_json",
            "thiserror",
        ];
        expected.sort();
        assert_eq!(
            found, expected,
            "eprfs-core's normal dependencies left their frozen set (rail R-E1)"
        );
        assert!(
            manifest
                .lines()
                .any(|l| l.starts_with("elohim-seam-contracts") && l.contains("\"serde\"")),
            "seam-contracts is admitted with its serde feature only"
        );
    }

    /// R-E1 cannot be passed by the form of a dependency: a target table and a dotted table
    /// each bring their dependency into the set the rail compares.
    #[test]
    fn the_frozen_set_counts_target_and_dotted_tables() {
        let manifest = "[dependencies]\nbytes = \"1\"\n\
                        [target.'cfg(unix)'.dependencies]\ntokio = \"1\"\n\
                        [dependencies.mio]\nversion = \"1\"\n\
                        [dev-dependencies]\nproptest = \"1\"\n";
        let normal: Vec<String> = manifest_deps::dependencies(manifest)
            .iter()
            .filter(|dep| dep.kind == manifest_deps::DepKind::Normal)
            .map(|dep| dep.package_name().to_string())
            .collect();
        assert_eq!(normal, vec!["bytes", "tokio", "mio"]);
    }

    /// No I/O, no runtime, no clock. The needles are assembled at run time from split literals:
    /// this module is one of the files it reads, so a contiguous literal would trip on itself.
    #[test]
    fn no_io_in_core() {
        let denied: [(String, &str); 7] = [
            (format!("std{}fs", "::"), "the filesystem is an adapter's"),
            (format!("File{}", "::"), "opening a handle is I/O"),
            (format!("std{}net", "::"), "the network is an adapter's"),
            (format!("std{}thread", "::"), "threads are a runtime's"),
            (format!("{}(", "spawn"), "spawning is a runtime's"),
            (format!("Instant{}now", "::"), "the core reads no clock"),
            (format!("SystemTime{}now", "::"), "the core reads no clock"),
        ];
        for (file, source) in SOURCES {
            for (needle, why) in &denied {
                assert!(
                    !source.contains(needle.as_str()),
                    "eprfs-core src/{file} names `{needle}` — {why} (rail R-E2)"
                );
            }
        }
    }

    /// A module added without an entry in [`SOURCES`] would be invisible to the scan above.
    #[test]
    fn scan_covers_every_module() {
        for line in include_str!("lib.rs")
            .lines()
            .filter(|line| line.starts_with("pub mod ") || line.starts_with("mod "))
        {
            let module = line
                .trim_start_matches("pub mod ")
                .trim_start_matches("mod ")
                .trim_end_matches(';')
                .trim();
            if module == "test_support" || module.ends_with('{') {
                continue;
            }
            let expected = format!("{module}.rs");
            assert!(
                SOURCES.iter().any(|(name, _)| *name == expected),
                "src/{expected} is not scanned by the purity boundary"
            );
        }
    }
}
