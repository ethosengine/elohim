//! Canonical signed-record boundary between doorway registrations and pkarr.
//!
//! The infrastructure DHT remains the source of truth. This crate owns the
//! deterministic bytes and Ed25519 verification that let the same notarized
//! record be projected to pkarr later without inventing a second record shape.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};

const RECORD_DOMAIN: &[u8] = b"elohim-doorway-endpoints-v1";
const MAX_TEXT_BYTES: usize = u16::MAX as usize;

/// Borrowed endpoint fields used to construct the canonical signed body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndpointRef<'a> {
    pub service: &'a str,
    pub url: &'a str,
    pub priority: u16,
    pub ttl_secs: u32,
}

/// Deterministic-record construction or verification failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordError {
    TooManyEndpoints(usize),
    TextTooLong { field: &'static str, bytes: usize },
    InvalidPublicKey,
    InvalidSignatureLength(usize),
    SignatureMismatch,
}

impl std::fmt::Display for RecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyEndpoints(count) => {
                write!(f, "endpoint record contains too many endpoints: {count}")
            }
            Self::TextTooLong { field, bytes } => {
                write!(f, "{field} exceeds {MAX_TEXT_BYTES} UTF-8 bytes: {bytes}")
            }
            Self::InvalidPublicKey => write!(f, "invalid Ed25519 public key"),
            Self::InvalidSignatureLength(bytes) => {
                write!(f, "Ed25519 signature must be 64 bytes, got {bytes}")
            }
            Self::SignatureMismatch => write!(f, "endpoint-record signature mismatch"),
        }
    }
}

impl std::error::Error for RecordError {}

/// Produce the exact bytes signed by a doorway's current identity-head key.
///
/// Strings are unsigned-length-prefixed UTF-8, integers are big-endian, and
/// endpoint order is preserved because priority ties are intentionally stable.
pub fn canonical_record_bytes(
    identity_root: &str,
    signing_key: &[u8; 32],
    serial: u64,
    endpoints: &[EndpointRef<'_>],
) -> Result<Vec<u8>, RecordError> {
    let endpoint_count: u16 = endpoints
        .len()
        .try_into()
        .map_err(|_| RecordError::TooManyEndpoints(endpoints.len()))?;

    let mut bytes = Vec::new();
    push_text(
        &mut bytes,
        "domain",
        std::str::from_utf8(RECORD_DOMAIN).expect("ASCII domain"),
    )?;
    push_text(&mut bytes, "identity_root", identity_root)?;
    bytes.extend_from_slice(signing_key);
    bytes.extend_from_slice(&serial.to_be_bytes());
    bytes.extend_from_slice(&endpoint_count.to_be_bytes());

    for endpoint in endpoints {
        push_text(&mut bytes, "endpoint.service", endpoint.service)?;
        push_text(&mut bytes, "endpoint.url", endpoint.url)?;
        bytes.extend_from_slice(&endpoint.priority.to_be_bytes());
        bytes.extend_from_slice(&endpoint.ttl_secs.to_be_bytes());
    }

    Ok(bytes)
}

/// Verify a detached Ed25519 signature over [`canonical_record_bytes`].
pub fn verify_record(
    signing_key: &[u8; 32],
    signature: &[u8],
    canonical_bytes: &[u8],
) -> Result<(), RecordError> {
    let verifying_key =
        VerifyingKey::from_bytes(signing_key).map_err(|_| RecordError::InvalidPublicKey)?;
    let signature_bytes: [u8; 64] = signature
        .try_into()
        .map_err(|_| RecordError::InvalidSignatureLength(signature.len()))?;
    let signature = Signature::from_bytes(&signature_bytes);
    verifying_key
        .verify(canonical_bytes, &signature)
        .map_err(|_| RecordError::SignatureMismatch)
}

fn push_text(target: &mut Vec<u8>, field: &'static str, value: &str) -> Result<(), RecordError> {
    let raw = value.as_bytes();
    let len: u16 = raw.len().try_into().map_err(|_| RecordError::TextTooLong {
        field,
        bytes: raw.len(),
    })?;
    target.extend_from_slice(&len.to_be_bytes());
    target.extend_from_slice(raw);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn endpoints() -> Vec<EndpointRef<'static>> {
        vec![
            EndpointRef {
                service: "gateway",
                url: "https://doorway.example",
                priority: 0,
                ttl_secs: 300,
            },
            EndpointRef {
                service: "gateway",
                url: "https://backup.example",
                priority: 10,
                ttl_secs: 120,
            },
        ]
    }

    #[test]
    fn canonical_bytes_are_stable_and_order_sensitive() {
        let key = [7_u8; 32];
        let first = canonical_record_bytes("uhCAk-root", &key, 42, &endpoints()).unwrap();
        let second = canonical_record_bytes("uhCAk-root", &key, 42, &endpoints()).unwrap();
        assert_eq!(first, second);

        let mut reversed = endpoints();
        reversed.reverse();
        assert_ne!(
            first,
            canonical_record_bytes("uhCAk-root", &key, 42, &reversed).unwrap()
        );
    }

    #[test]
    fn signature_round_trip_rejects_tampering() {
        let signing_key = SigningKey::from_bytes(&[11_u8; 32]);
        let public_key = signing_key.verifying_key().to_bytes();
        let bytes = canonical_record_bytes("uhCAk-root", &public_key, 9, &endpoints()).unwrap();
        let signature = signing_key.sign(&bytes).to_bytes();

        verify_record(&public_key, &signature, &bytes).unwrap();

        let mut tampered = bytes;
        tampered.push(0);
        assert_eq!(
            verify_record(&public_key, &signature, &tampered),
            Err(RecordError::SignatureMismatch)
        );
    }
}
