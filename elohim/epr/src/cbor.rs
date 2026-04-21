//! Canonical CBOR wrapper using serde_ipld_dagcbor (dag-cbor).
//!
//! dag-cbor implements RFC 8949 §4.2.1 ("Core Deterministic Encoding Requirements"):
//! sorted map keys, shortest-form integers, no indefinite-length items.

use crate::error::{EprError, Result};
use ipld_core::ipld::Ipld;

/// Encode an Ipld value to canonical dag-cbor bytes.
pub fn encode(value: &Ipld) -> Result<Vec<u8>> {
    serde_ipld_dagcbor::to_vec(value).map_err(|e| EprError::Encode(e.to_string()))
}

/// Decode dag-cbor bytes to an Ipld value.
pub fn decode(bytes: &[u8]) -> Result<Ipld> {
    serde_ipld_dagcbor::from_slice(bytes).map_err(|e| EprError::Decode(e.to_string()))
}

/// Decode dag-cbor bytes with strict canonical-form enforcement:
/// the decoded value, re-encoded, must produce byte-identical output.
pub fn decode_strict(bytes: &[u8]) -> Result<Ipld> {
    let decoded = decode(bytes)?;
    let re_encoded = encode(&decoded)?;
    if re_encoded != bytes {
        return Err(EprError::Decode(format!(
            "input is not canonical dag-cbor (got {} bytes, re-encoded to {} bytes)",
            bytes.len(),
            re_encoded.len()
        )));
    }
    Ok(decoded)
}
