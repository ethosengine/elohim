//! What the store refuses or fails with.

use std::io;
use std::path::PathBuf;

/// Every failure of a [`crate::BlobStore`] operation.
///
/// The first five variants are the ones the store has always produced (elohim-storage maps
/// them onto its own `StorageError` variant for variant); the last three belong to the
/// owner/shared split and to opening a store a peer shares with another process.
#[derive(Debug, thiserror::Error)]
pub enum BlobStoreError {
    /// No payload, chunk or record by that CID is held.
    #[error("blob not found: {0}")]
    BlobNotFound(String),

    /// Bytes read back do not hash to the CID they are filed under.
    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    /// A CID of the wrong profile for the lane it was asked of.
    #[error("invalid content address: {0}")]
    InvalidContentAddress(String),

    /// The filesystem refused.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Anything else (an encode failure, a task that died).
    #[error("internal error: {0}")]
    Internal(String),

    /// [`crate::BlobStore::open_owner`] found another instance holding `owner.lock`.
    #[error("the payload store at {} already has an owner", root.display())]
    OwnerHeld { root: PathBuf },

    /// A destructive operation asked of a shared instance: only the owner moves or removes
    /// files.
    #[error("a shared payload-store instance may not {op}: only the store's owner moves or removes files")]
    SharedInstance { op: &'static str },

    /// The root of an owner or shared store is not a private, real directory.
    #[error("payload store root {} refused: {reason}", root.display())]
    UnsafeRoot { root: PathBuf, reason: String },
}

/// The store's result type.
pub type Result<T> = std::result::Result<T, BlobStoreError>;
