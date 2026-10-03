//! Error types for elohim-storage

#![forbid(unsafe_code)]

use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Diesel error: {0}")]
    Diesel(#[from] diesel::result::Error),

    #[error("Blob not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

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

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// Author-side reach earning refused (D.4 Stage 1).
    ///
    /// The signer has not earned the declared reach. The EPR was NOT persisted
    /// and no network publication (Kad/gossip) occurred. HTTP 403.
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Validation error: {0}")]
    Validation(String),

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

    #[error("Codec error: {0}")]
    Codec(String),

    /// Resolver-backed Ed25519 signature verification failed (Task A.7).
    ///
    /// Returned by `EprService::ingest_with_cache` when the signer's pubkey
    /// timeline cannot supply a key for `issued_at`, or when the signature
    /// does not verify under that key. The EPR is rejected and NOT persisted.
    #[error("EPR signature verification failed: {reason}")]
    VerifyFailed { reason: String },

    /// A chain write refused by the plane rule, before it was offered to the
    /// conductor (`elohim-storage`'s `chain_write_gate`).
    ///
    /// Storage signs as the steward's own agent, so every chain write is a
    /// permanent statement in the PERSON's voice. Two shapes are refused: a
    /// machine writing on a timer (`voice = "machine"`, `trigger = "timer"`),
    /// and a writer that never named itself (`voice`/`trigger` =
    /// `"undeclared"`). Nothing was committed, and re-offering the same call
    /// will be refused again: this is a defect in the write site, not a
    /// transient failure, so it maps to no retryable status.
    #[error(
        "Plane violation: writer `{writer}` (voice={voice}, trigger={trigger}) was refused \
         `{zome}::{fn_name}` before it reached the conductor. A person's source chain speaks in \
         the person's voice: a machine write on a timer is refused, and every chain write must \
         name its writer so its voice (person, delegate, machine) and trigger (act, transition, \
         period close, timer) are declared. See \
         genesis/docs/superpowers/specs/2026-10-03-plane-separation-design.md §1 and §12."
    )]
    PlaneViolation {
        writer: String,
        voice: String,
        trigger: String,
        zome: String,
        fn_name: String,
    },
}
