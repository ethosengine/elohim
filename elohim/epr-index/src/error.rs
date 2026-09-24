//! The crate's one error type. Every variant renders exactly as the recall executor's own error
//! for the same fault did before the lift, so a message that reaches a screen, an attestation's
//! `why` or a golden rendering is byte-identical whichever side of the crate boundary raised it.

/// What an index step refuses or fails on.
#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    /// A declaration or a caller's request this crate will not run: an undeclared chunk-rule
    /// shape, a batch over its budget. The executor's `invalid arguments`.
    #[error("invalid arguments: {0}")]
    Refused(String),

    /// A declared route that cannot run here and now (an embedder whose pinned bytes are absent,
    /// a reply of the wrong width). Not a fault in the question.
    #[error("unavailable: {0}")]
    Unavailable(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    /// A content address that could not be minted (`atom_cid`).
    #[error("fabric: {0}")]
    Fabric(#[from] elohim_epr_rea::FabricError),

    /// The fold store's SQLite said no. Rendered as the executor always rendered it: an I/O
    /// error whose message is `fold store: <sqlite error>`.
    #[error("io: fold store: {0}")]
    Store(#[from] rusqlite::Error),
}

/// A result whose error is an [`IndexError`].
pub type Result<T> = std::result::Result<T, IndexError>;
