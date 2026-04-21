//! Error types for the elohim-epr crate.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EprError {
    #[error("cbor encode error: {0}")]
    Encode(String),
    #[error("cbor decode error: {0}")]
    Decode(String),
    #[error("invalid cid: expected {expected}, got {actual}")]
    InvalidCid { expected: String, actual: String },
    #[error("signature error: {0}")]
    Signature(String),
    #[error("coupling requirement not met: {0}")]
    Coupling(String),
    #[error("invalid envelope: {0}")]
    InvalidEnvelope(String),
}

pub type Result<T> = std::result::Result<T, EprError>;
