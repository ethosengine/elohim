//! Core contracts for EPR-governed filesystem projections.
//!
//! `eprfs-core` is deliberately storage-agnostic and domain-agnostic. It models
//! how distributed EPR-backed data is projected into a filesystem tree, not what
//! that tree means.

pub mod address;
pub mod attestation;
pub mod awareness;
pub mod error;
pub mod meta;
pub mod projection;
pub mod storage;

pub use address::{BlobCid, EprRef, ProjectionId};
pub use attestation::{AttestationDraft, AttestationKind};
pub use awareness::{
    BytePresence, EprCard, EprResiliency, LocalOverlayStatus, PeerVisibility, ProjectionAwareness,
    ProjectionAwarenessProvider, ProjectionEntryAwareness, VerificationStatus,
};
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
