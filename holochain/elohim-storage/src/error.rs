//! Error types for elohim-storage

use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Blob not found: {0}")]
    NotFound(String),

    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("Chunk missing: blob {hash}, chunk {index}")]
    ChunkMissing { hash: String, index: u32 },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    Database(#[from] sled::Error),

    #[error("Holochain client error: {0}")]
    HolochainClient(String),

    #[error("Signal error: {0}")]
    Signal(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Internal error: {0}")]
    Internal(String),
}
