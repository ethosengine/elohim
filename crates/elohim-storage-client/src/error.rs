//! Error types for storage client

use thiserror::Error;

/// Storage client error
#[derive(Debug, Error)]
pub enum StorageError {
    /// HTTP request failed
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization/deserialization failed
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Server returned an error
    #[error("Server error {status}: {message}")]
    Server { status: u16, message: String },

    /// Document not found
    #[error("Document not found: {0}")]
    NotFound(String),

    /// Invalid response from server
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Automerge error
    #[error("Automerge error: {0}")]
    Automerge(String),

    /// Base64 decode error
    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
}

impl From<automerge::AutomergeError> for StorageError {
    fn from(e: automerge::AutomergeError) -> Self {
        StorageError::Automerge(e.to_string())
    }
}

/// Result type for storage operations
pub type Result<T> = std::result::Result<T, StorageError>;
