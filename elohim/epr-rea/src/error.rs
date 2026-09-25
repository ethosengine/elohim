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

    #[error("invalid edge: {0}")]
    InvalidEdge(String),

    #[error("invalid bound: {0}")]
    InvalidBound(String),

    #[error("invalid scope relation: {0}")]
    InvalidScopeRelation(String),

    #[error("invalid external claim: {0}")]
    InvalidExternalClaim(String),

    #[error("invalid offer: {0}")]
    InvalidOffer(String),
}

pub type Result<T> = std::result::Result<T, FabricError>;
