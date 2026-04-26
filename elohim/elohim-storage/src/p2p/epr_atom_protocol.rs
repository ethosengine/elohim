//! EPR Atom Protocol — Request-response protocol for signed EPR atoms.
//!
//! Carries CBOR-encoded `elohim_epr::Envelope` bytes between peers. Coexists
//! with the legacy `/elohim/epr/1.0.0` (which serves EprHead via MessagePack).
//!
//! Wire format: 4-byte BE length prefix + CBOR body.
//!
//! This module defines transient wire types — they live only during a single
//! request-response exchange. No persistent source of truth is introduced.
//! The notarized source of truth remains the ed25519-signed CBOR Envelope
//! (content-addressed by CID). The `epr_atoms` table (Phase 2a) is its
//! operational projection.
//!
//! Spec: `genesis/docs/superpowers/specs/2026-04-23-epr-phase-2c-libp2p-federation-design.md`

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::request_response;
use serde::{Deserialize, Serialize};
use std::io;

/// Protocol identifier for EPR atom federation.
pub const EPR_ATOM_PROTOCOL_ID: &str = "/elohim/epr-atom/1.0.0";

/// Max request size: 256 KB (accommodates FetchBatch of ~100 CIDs).
pub const MAX_REQUEST_SIZE: usize = 256 * 1024;

/// Max response size: 2 MB (headroom for future atom payload growth).
pub const MAX_RESPONSE_SIZE: usize = 2 * 1024 * 1024;

/// Max CIDs per batch request.
pub const MAX_BATCH_CIDS: usize = 128;

/// Protocol marker for libp2p negotiation.
#[derive(Debug, Clone)]
pub struct EprAtomProtocol;

impl AsRef<str> for EprAtomProtocol {
    fn as_ref(&self) -> &str {
        EPR_ATOM_PROTOCOL_ID
    }
}

/// Request variants — transient wire types (no persistent source of truth).
/// `tag` is the CBOR discriminator; shape matches the wire contract in
/// `elohim/sdk/schemas/v1/p2p/epr-atom-message.schema.json`.
// TODO(Z.1 / D.5 schema-first): update elohim/sdk/schemas/v1/p2p/epr-atom-message.schema.json
// with integrity_notify request variant + integrity_ack response variant.
// Tracked by Phase 2B Batch D Z.1 (TODO sweep).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tag", rename_all = "snake_case")]
pub enum EprAtomRequest {
    /// Fetch a single atom by CID.
    Fetch { cid: String },
    /// Announce a new atom (push). Body is raw CBOR envelope bytes.
    Announce {
        #[serde(with = "serde_bytes")]
        envelope_bytes: Vec<u8>,
    },
    /// Fetch multiple atoms in one request. Bounded by `MAX_BATCH_CIDS`.
    FetchBatch { cids: Vec<String> },
    /// D.5: direct-notify of integrity events (KeyRevocation, KeyRotation,
    /// RevocationAttestation, AgentPeerBinding). The `kind` field discriminates
    /// the encoding of `payload_bytes`. For `KeyRevocation` the payload is a
    /// MessagePack-encoded `RecoveryRevocationMessage`. Receivers decode by
    /// kind and route to the same handler the gossipsub receive path uses.
    ///
    /// Stage 1: only KeyRevocation is implemented on the receive side; other
    /// integrity kinds are accepted but logged as not-yet-handled.
    IntegrityNotify {
        kind: String,
        #[serde(with = "serde_bytes")]
        payload_bytes: Vec<u8>,
    },
}

/// Response variants — transient wire types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tag", rename_all = "snake_case")]
pub enum EprAtomResponse {
    /// Single atom (raw CBOR envelope bytes).
    Atom {
        #[serde(with = "serde_bytes")]
        envelope_bytes: Vec<u8>,
    },
    /// Batch response — one entry per requested CID, `None` for missing/unauthorized.
    AtomBatch {
        #[serde(with = "serde_bytes_vec")]
        atoms: Vec<Option<Vec<u8>>>,
    },
    /// Ack for AnnounceAtom.
    Announced {
        accepted: bool,
        reason: Option<String>,
    },
    /// Atom missing OR reach gate failed (leak-free — caller can't distinguish).
    NotFound,
    /// Protocol-level error (malformed request, batch too large, etc.).
    Error { message: String },
    /// D.5: ack for IntegrityNotify. `received: true` means the payload was
    /// successfully decoded and routed; `false` means decode or routing failed,
    /// with `reason` carrying a non-secret diagnostic string.
    IntegrityAck {
        received: bool,
        reason: Option<String>,
    },
}

/// Helper for `#[serde(with = "...")]` over `Vec<Option<Vec<u8>>>`.
/// Each `Some(bytes)` is serialized as a CBOR byte string; `None` as null.
mod serde_bytes_vec {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_bytes::ByteBuf;

    pub fn serialize<S>(v: &[Option<Vec<u8>>], s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mapped: Vec<Option<ByteBuf>> = v
            .iter()
            .map(|o| o.as_ref().map(|b| ByteBuf::from(b.clone())))
            .collect();
        mapped.serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Vec<Option<Vec<u8>>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mapped: Vec<Option<ByteBuf>> = Vec::deserialize(d)?;
        Ok(mapped
            .into_iter()
            .map(|o| o.map(|b| b.into_vec()))
            .collect())
    }
}

/// Codec for the EPR atom protocol. CBOR body + 4-byte BE length prefix.
#[derive(Debug, Clone, Default)]
pub struct EprAtomCodec;

#[async_trait]
impl request_response::Codec for EprAtomCodec {
    type Protocol = EprAtomProtocol;
    type Request = EprAtomRequest;
    type Response = EprAtomResponse;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_cbor(io, MAX_REQUEST_SIZE).await
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_cbor(io, MAX_RESPONSE_SIZE).await
    }

    async fn write_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        request: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_cbor(io, &request, MAX_REQUEST_SIZE).await
    }

    async fn write_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        response: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_cbor(io, &response, MAX_RESPONSE_SIZE).await
    }
}

async fn read_cbor<T, V>(io: &mut T, max_size: usize) -> io::Result<V>
where
    T: AsyncRead + Unpin + Send,
    V: serde::de::DeserializeOwned,
{
    let mut len_buf = [0u8; 4];
    io.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > max_size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "epr-atom message too large: {} bytes (max {})",
                len, max_size
            ),
        ));
    }
    let mut buf = vec![0u8; len];
    io.read_exact(&mut buf).await?;
    ciborium::de::from_reader(&buf[..])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("cbor decode: {}", e)))
}

async fn write_cbor<T, V>(io: &mut T, value: &V, max_size: usize) -> io::Result<()>
where
    T: AsyncWrite + Unpin + Send,
    V: serde::Serialize,
{
    let mut buf = Vec::new();
    ciborium::ser::into_writer(value, &mut buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("cbor encode: {}", e)))?;
    if buf.len() > max_size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "outgoing epr-atom message too large: {} bytes (max {})",
                buf.len(),
                max_size
            ),
        ));
    }
    let len_buf = (buf.len() as u32).to_be_bytes();
    io.write_all(&len_buf).await?;
    io.write_all(&buf).await?;
    io.flush().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Inbound envelope verification
// ---------------------------------------------------------------------------

/// Decode wire bytes as `elohim_epr::Epr`, recompute CID from canonical bytes,
/// verify the CID matches, run structural signature checks (algorithm + length),
/// and run the coupling validator. Returns the verified `Epr` and its `Cid`,
/// or a `VerifyError` describing which stage failed.
///
/// This helper does NOT verify the ed25519 signature under a public key —
/// that requires a resolver we don't have at this layer. `EprService::ingest`
/// will re-run the same structural pass before persistence; this helper lets
/// handlers reject obviously-malformed bytes early without touching the DB.
pub fn verify_incoming_epr(wire_bytes: &[u8]) -> Result<(elohim_epr::Epr, cid::Cid), VerifyError> {
    // Stage 1: CBOR decode to Epr (envelope + payload together).
    let epr: elohim_epr::Epr = ciborium::de::from_reader(wire_bytes)
        .map_err(|e| VerifyError::CborDecode(e.to_string()))?;

    // Stage 2: canonicalize + CID recompute.
    let canonical = epr
        .envelope
        .canonical_bytes(&epr.payload)
        .map_err(|e| VerifyError::Canonicalize(e.to_string()))?;
    let computed_cid = elohim_epr::cid::compute_cid(&canonical);
    if computed_cid != epr.envelope.cid {
        return Err(VerifyError::CidMismatch {
            claimed: epr.envelope.cid.to_string(),
            computed: computed_cid.to_string(),
        });
    }

    // Stage 3: structural signature check (algorithm + length only).
    if epr.envelope.proof.algorithm != "ed25519" {
        return Err(VerifyError::Signature(format!(
            "unsupported algorithm: {}",
            epr.envelope.proof.algorithm
        )));
    }
    if epr.envelope.proof.signature.len() != 64 {
        return Err(VerifyError::Signature(
            "ed25519 signature must be 64 bytes".to_string(),
        ));
    }

    // Stage 4: coupling validator chain.
    elohim_epr::validate_coupling(&epr.envelope)
        .map_err(|e| VerifyError::Validation(e.to_string()))?;

    let cid = epr.envelope.cid;
    Ok((epr, cid))
}

/// Error returned by `verify_incoming_epr`, tagged by the stage that failed.
#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("cbor decode: {0}")]
    CborDecode(String),
    #[error("canonicalize: {0}")]
    Canonicalize(String),
    #[error("cid mismatch: claimed={claimed} computed={computed}")]
    CidMismatch { claimed: String, computed: String },
    #[error("signature: {0}")]
    Signature(String),
    #[error("validation: {0}")]
    Validation(String),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::request_response::Codec as _;

    /// Helper: encode a request to bytes via the codec, then decode from those bytes.
    async fn encode_decode_request(req: EprAtomRequest) -> EprAtomRequest {
        use futures::io::Cursor;
        let mut codec = EprAtomCodec;
        let proto = EprAtomProtocol;

        // Write into a Vec<u8> buffer.
        let mut buf: Vec<u8> = Vec::new();
        codec
            .write_request(&proto, &mut buf, req)
            .await
            .expect("write_request");

        // Read back from a Cursor over the buffer.
        let mut cursor = Cursor::new(buf);
        codec
            .read_request(&proto, &mut cursor)
            .await
            .expect("read_request")
    }

    /// Helper: encode a response to bytes via the codec, then decode from those bytes.
    async fn encode_decode_response(resp: EprAtomResponse) -> EprAtomResponse {
        use futures::io::Cursor;
        let mut codec = EprAtomCodec;
        let proto = EprAtomProtocol;

        let mut buf: Vec<u8> = Vec::new();
        codec
            .write_response(&proto, &mut buf, resp)
            .await
            .expect("write_response");

        let mut cursor = Cursor::new(buf);
        codec
            .read_response(&proto, &mut cursor)
            .await
            .expect("read_response")
    }

    /// D.5: IntegrityNotify request roundtrips through the CBOR codec with
    /// kind and payload_bytes preserved exactly.
    #[tokio::test]
    async fn integrity_notify_request_roundtrip() {
        let original = EprAtomRequest::IntegrityNotify {
            kind: "KeyRevocation".to_string(),
            payload_bytes: b"test-revocation-payload-bytes".to_vec(),
        };

        let decoded = encode_decode_request(original.clone()).await;

        match (original, decoded) {
            (
                EprAtomRequest::IntegrityNotify {
                    kind: k1,
                    payload_bytes: p1,
                },
                EprAtomRequest::IntegrityNotify {
                    kind: k2,
                    payload_bytes: p2,
                },
            ) => {
                assert_eq!(k1, k2, "kind must round-trip");
                assert_eq!(p1, p2, "payload_bytes must round-trip");
            }
            _ => panic!("decoded variant did not match IntegrityNotify"),
        }
    }

    /// D.5: IntegrityAck response (received=true) roundtrips through the CBOR codec.
    #[tokio::test]
    async fn integrity_ack_response_roundtrip() {
        let original = EprAtomResponse::IntegrityAck {
            received: true,
            reason: None,
        };

        let decoded = encode_decode_response(original.clone()).await;

        match (original, decoded) {
            (
                EprAtomResponse::IntegrityAck {
                    received: r1,
                    reason: reason1,
                },
                EprAtomResponse::IntegrityAck {
                    received: r2,
                    reason: reason2,
                },
            ) => {
                assert_eq!(r1, r2, "received must round-trip");
                assert_eq!(reason1, reason2, "reason must round-trip");
            }
            _ => panic!("decoded variant did not match IntegrityAck"),
        }
    }

    /// D.5: IntegrityAck with reason preserves the reason string.
    #[tokio::test]
    async fn integrity_ack_with_reason_roundtrip() {
        let original = EprAtomResponse::IntegrityAck {
            received: false,
            reason: Some("decode failed: unexpected EOF".to_string()),
        };

        let decoded = encode_decode_response(original).await;

        match decoded {
            EprAtomResponse::IntegrityAck {
                received: false,
                reason: Some(r),
            } => {
                assert!(r.contains("decode failed"), "reason must be preserved");
            }
            other => panic!("unexpected decoded variant: {other:?}"),
        }
    }
}
