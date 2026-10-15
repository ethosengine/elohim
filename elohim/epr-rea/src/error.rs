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

    /// The line was written by a newer writer: its record carries fields this binary does not
    /// know, so the stored CID cannot be re-verified here. Refused, never read without them.
    #[error(
        "sidecar record {stored} carries field(s) [{fields}] that this reader (elohim-epr-rea \
         {reader}) does not know, so its CID cannot be re-verified here — the line was written by \
         a newer writer; upgrade this binary (refusing rather than reading the record without them)"
    )]
    NewerWriter {
        stored: String,
        fields: String,
        reader: &'static str,
    },

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
