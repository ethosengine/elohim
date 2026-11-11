//! What the store refuses or fails with.

use std::io;
use std::path::PathBuf;

/// Every failure of a [`crate::BlobStore`] operation.
///
/// The first five variants are the ones the store has always produced (elohim-storage maps
/// them onto its own `StorageError` variant for variant); the remaining variants belong to the
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

    /// A shared instance found a held copy it could not read: it does not know what it would
    /// overwrite, so it writes nothing and leaves the copy for the owner.
    #[error("the held copy of {cid} could not be read ({reason}); left for the store's owner")]
    HeldCopyUnreadable { cid: String, reason: String },

    /// An owner or shared open found a root that is neither empty nor marked
    /// [`crate::marker::BLOB_STORE_V1`]: a store written before the format marker (the unlocked
    /// store's representation, a directory store, anything else). Owner and shared stores are
    /// only ever created fresh, so nothing was written to it.
    #[error("the payload store at {} holds no {} marker and is not empty; an owner or shared store is created fresh", path.display(), crate::marker::FILE)]
    LegacyLayout { path: PathBuf },

    /// An owner or shared open found a format marker naming another format, or one it could not
    /// read. Nothing was written.
    #[error("the payload store marker {} says `{found}`, not `{}`", path.display(), crate::marker::BLOB_STORE_V1)]
    Format { path: PathBuf, found: String },

    /// [`crate::BlobStore::new`] (the unlocked store) found a format marker in its root: the root
    /// belongs to an owner/shared store, whose files the unlocked representation must never
    /// touch. Nothing was written.
    #[error("the unlocked store does not open {}: it carries a {} marker (`{found}`)", path.display(), crate::marker::FILE)]
    MarkedRoot { path: PathBuf, found: String },

    /// The filesystem refused `link(2)`, the only way an owner or shared store publishes a
    /// chunk file (no rename fallback, which could replace a concurrent winner).
    #[error("this filesystem cannot hold an owner or shared payload store: {reason}")]
    UnsupportedFilesystem { reason: String },

    /// A streamed publication paused for a repair and a chunk it had written left the store
    /// meanwhile; nothing naming it was published. Store the payload again.
    #[error("the publication of {cid} lost a chunk: {reason}")]
    PublicationLost { cid: String, reason: String },

    /// The root of an owner or shared store is not a private, real directory.
    #[error("payload store root {} refused: {reason}", root.display())]
    UnsafeRoot { root: PathBuf, reason: String },
}

/// The store's result type.
pub type Result<T> = std::result::Result<T, BlobStoreError>;
