use std::path::PathBuf;

use crate::address::{BlobCid, EprRef};

pub type Result<T> = std::result::Result<T, EprfsError>;

#[derive(Debug, thiserror::Error)]
pub enum EprfsError {
    #[error("invalid projection path: {0}")]
    InvalidProjectionPath(String),

    #[error("invalid projection manifest: {0}")]
    InvalidProjectionManifest(String),

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

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
