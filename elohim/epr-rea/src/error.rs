use thiserror::Error;

#[derive(Debug, Error)]
pub enum FabricError {
    #[error("canonical encode failed: {0}")]
    Encode(String),

    #[error("decode failed: {0}")]
    Decode(String),

    #[error("sidecar io: {0}")]
    Io(#[from] std::io::Error),

    #[error("integrity: stored cid {stored} does not match recomputed {computed}")]
    Integrity { stored: String, computed: String },
}

pub type Result<T> = std::result::Result<T, FabricError>;
