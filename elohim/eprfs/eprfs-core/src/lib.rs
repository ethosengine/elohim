//! Core contracts for EPR-governed filesystem projections.
//!
//! `eprfs-core` is deliberately storage-agnostic and domain-agnostic. It models
//! how distributed EPR-backed data is projected into a filesystem tree, not what
//! that tree means.

// `async_trait` marks its generated futures `#[must_use]`; clippy 1.99 reads
// that as doubled on methods returning `Result`. The attribute is the macro's.
#![allow(clippy::double_must_use)]

pub mod address;
pub mod attestation;
pub mod awareness;
pub mod composition;
pub mod error;
pub mod meta;
pub mod projection;
pub mod storage;
pub mod tree;

pub use address::{BlobCid, BlobLink, EprRef, ProjectionId};
pub use attestation::{AttestationDraft, AttestationKind};
pub use awareness::{
    BytePresence, EprCard, EprResiliency, LocalOverlayStatus, PeerVisibility, ProjectionAwareness,
    ProjectionAwarenessProvider, ProjectionEntryAwareness, VerificationStatus,
};
pub use composition::{CompositionGraph, CompositionNode, DerivationEdge, DerivationKind};
pub use error::{EprfsError, Result};
pub use meta::{
    EprHeadCoupling, EprMetaGovernance, EprMetaRecord, EprMetaResolution, EprMetaSource,
    EprMetaSubject, GovernanceBinding, GovernancePolicyBinding, GovernanceRule,
    GovernanceRuleClass, GovernanceRulePredicate, GovernanceScope, GovernanceValidator,
};
pub use projection::{
    EntryKind, MaterializationPolicy, ProjectionEntry, ProjectionManifest, ProjectionPath,
    ProjectionRoot, ProjectionSource, ProjectionSourceKind, ProjectionStatus,
};
pub use storage::{BlobHandle, BlobPresence, EprRecord, EprfsStorage, FetchPolicy};
pub use tree::{
    decode_tree, encode_tree, load_verified_tree, TreeEntry, TreeEntryKind, TreeError, TreeLimits,
    TreeNode, TreeResult, VerifiedTree,
};
