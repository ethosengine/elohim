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
    /// A refusal from [`crate::algedonic`] — a signal whose kind disagrees with its evidence,
    /// or evidence that names no bound. Pain has its own refusal class because an algedonic
    /// signal is not an envelope: it is a report against a promise.
    #[error("algedonic signal: {0}")]
    Algedonic(String),
}

pub type Result<T> = std::result::Result<T, EprError>;
