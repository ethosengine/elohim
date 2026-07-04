//! Core contracts for EPR-governed filesystem projections.
//!
//! `eprfs-core` is deliberately storage-agnostic and domain-agnostic. It models
//! how distributed EPR-backed data is projected into a filesystem tree, not what
//! that tree means.

pub mod address;
pub mod attestation;
pub mod error;
pub mod projection;
pub mod storage;

pub use address::{BlobCid, EprRef, ProjectionId};
pub use attestation::{AttestationDraft, AttestationKind};
pub use error::{EprfsError, Result};
pub use projection::{
    EntryKind, MaterializationPolicy, ProjectionEntry, ProjectionManifest, ProjectionPath,
    ProjectionRoot, ProjectionSource, ProjectionSourceKind, ProjectionStatus,
};
pub use storage::{BlobHandle, BlobPresence, EprRecord, EprfsStorage, FetchPolicy};
