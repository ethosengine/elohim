//! Error types for the Elohim SDK

use thiserror::Error;

/// Result type for SDK operations
pub type Result<T> = std::result::Result<T, SdkError>;

/// SDK error types
#[derive(Error, Debug)]
pub enum SdkError {
    /// Content not found
    #[error("Content not found: {0}")]
    NotFound(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Sync error
    #[error("Sync error: {0}")]
    Sync(String),

    /// Write buffer full
    #[error("Write buffer full, backpressure at {0}%")]
    BackpressureFull(u8),

    /// Access denied
    #[error("Access denied: reach level {required} required, have {actual}")]
    AccessDenied { required: String, actual: String },

    /// Invalid mode
    #[error("Invalid client mode for operation: {0}")]
    InvalidMode(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
}

#[cfg(feature = "client")]
impl From<elohim_storage_client::StorageError> for SdkError {
    fn from(err: elohim_storage_client::StorageError) -> Self {
        SdkError::Storage(err.to_string())
    }
}

#[cfg(feature = "client")]
impl From<reqwest::Error> for SdkError {
    fn from(err: reqwest::Error) -> Self {
        SdkError::Network(err.to_string())
    }
}

impl From<serde_json::Error> for SdkError {
    fn from(err: serde_json::Error) -> Self {
        SdkError::Serialization(err.to_string())
    }
}
