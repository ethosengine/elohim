use std::path::PathBuf;

use seam_contracts::Unanswered;

use crate::address::{BlobCid, EprRef};
use crate::tier::ItemAbsence;

pub type Result<T> = std::result::Result<T, EprfsError>;

/// Non-exhaustive from `0.2.0`, the first registry publish that carries the walk and the tier
/// chain (interfaces spec §0 finding 2, decision 20): remote tiers add refusals without breaking a
/// caller. Marking it is itself a break from `0.1.2`, in the same release as `TreeError`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EprfsError {
    #[error("invalid projection path: {0}")]
    InvalidProjectionPath(String),

    #[error("invalid projection manifest: {0}")]
    InvalidProjectionManifest(String),

    #[error("invalid composition graph: {0}")]
    InvalidCompositionGraph(String),

    #[error("EPR record not found: {0:?}")]
    EprNotFound(EprRef),

    #[error("blob not found: {0:?}")]
    BlobNotFound(BlobCid),

    #[error("blob is not local and fetch was not requested: {0:?}")]
    BlobNotLocal(BlobCid),

    #[error("projection entry is missing blob for path: {0:?}")]
    MissingBlob(PathBuf),

    #[error("filesystem I/O error at {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("storage adapter error: {0}")]
    Storage(String),

    /// The store holds bytes under `cid` that fail their address. Typed, so a reader maps it to
    /// `ItemAbsence::Integrity` and `TreeError::Integrity` without reading an error message, and
    /// no other storage failure can be mistaken for it (B4).
    #[error("stored object is corrupt: {0:?}")]
    Corrupt(BlobCid),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// A tier did not answer for `cid`, with ruling 7's three parts (interfaces spec §A.1).
    /// Boxed so every `Result` carrying this error stays small (clippy `result_large_err`).
    #[error("unanswered: {cid}")]
    Unanswered {
        cid: BlobCid,
        why: Box<Unanswered<ItemAbsence>>,
    },

    /// A pathless fetch asked a chain for more than its own store (A18): outer tiers are reached
    /// only through a root-carrying call, so the chain says so instead of a false absence.
    #[error("root required for a fetch beyond the own store: {0:?}")]
    RootRequired(BlobCid),
}
