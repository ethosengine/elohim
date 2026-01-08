//! Error types for elohim-storage

use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Diesel error: {0}")]
    Diesel(#[from] diesel::result::Error),

    #[error("Blob not found: {0}")]
    NotFound(String),

    #[error("Blob not found in storage: {0}")]
    BlobNotFound(String),

    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("Chunk missing: blob {hash}, chunk {index}")]
    ChunkMissing { hash: String, index: u32 },

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Request timeout: {0}")]
    Timeout(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    DatabaseSled(#[from] sled::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Holochain client error: {0}")]
    HolochainClient(String),

    #[error("Conductor error: {0}")]
    Conductor(String),

    #[error("Signal error: {0}")]
    Signal(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Internal error: {0}")]
    Internal(String),

    // P2P-related errors (feature: p2p)
    #[error("Peer not found: {0}")]
    PeerNotFound(String),

    #[error("No providers found for content: {0}")]
    NoProviders(String),

    #[error("P2P network error: {0}")]
    P2PNetwork(String),

    #[error("Identity error: {0}")]
    Identity(String),

    #[error("Cluster error: {0}")]
    Cluster(String),

    #[error("Replication failed: {0}")]
    Replication(String),

    #[error("Invalid content address: {0}")]
    InvalidContentAddress(String),

    // Sync-related errors
    #[error("Sync error: {0}")]
    Sync(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}
