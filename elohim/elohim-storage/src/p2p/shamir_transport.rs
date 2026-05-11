//! libp2p request-response protocol for Shamir share delivery.
//!
//! Authorization is the recovery-approval attestation on the DHT: when a custodian
//! sends a share, the recovery agent verifies the corresponding
//! `attestation:recovery-approval` exists and is current. The DHT carries the
//! authorization (cheap, durable, witnessed); libp2p carries the share material
//! (expensive, ephemeral, point-to-point).
//!
//! This protocol is the OPTIONAL cryptographic proof layer that sits on top of
//! the attestation-DHT-driven recovery flow. Low-security recovery completes via
//! social-threshold quorum alone; only key-material recovery invokes this transport.
//!
//! ## Wire format
//!
//! 4-byte BE length prefix + MessagePack body (matches all elohim request-response
//! protocol conventions). Protocol ID: `/elohim/shamir-share/1.0.0`.
//!
//! ## Swarm registration
//!
//! TODO(G.1-swarm-wiring): the `ShamirShareCodec` is ready for registration but
//! adding a new field to `ElohimStorageBehaviour` requires touching the behaviour
//! struct + `From` impls + the swarm event loop match arm — a multi-file change
//! deferred so G.1 lands cleanly on its own commit boundary. Registration follows
//! the exact same pattern as `trust_protocol` (see `behaviour.rs:88` and
//! `mod.rs:2292`). Track under the share-custody epic.

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::request_response;
use serde::{Deserialize, Serialize};
use std::io;

// ─────────────────────────────────────────────────────────────────────────────
// Protocol identifier
// ─────────────────────────────────────────────────────────────────────────────

/// Protocol ID for Shamir share transport.
///
/// Versioned at 1.0.0 so future wire-breaking changes increment to 1.1.0 or 2.0.0
/// without confusing old peers (libp2p negotiate-on-connect guards the version).
pub const SHAMIR_SHARE_PROTOCOL_ID: &str = "/elohim/shamir-share/1.0.0";

// ─────────────────────────────────────────────────────────────────────────────
// Protocol struct (needed for `AsRef<str>` negotiation)
// ─────────────────────────────────────────────────────────────────────────────

/// Marker type that libp2p uses for protocol negotiation.
///
/// Implements `AsRef<str>` to return [`SHAMIR_SHARE_PROTOCOL_ID`].
#[derive(Debug, Clone)]
pub struct ShamirShareProtocol;

impl AsRef<str> for ShamirShareProtocol {
    fn as_ref(&self) -> &str {
        SHAMIR_SHARE_PROTOCOL_ID
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Request / Response types
// ─────────────────────────────────────────────────────────────────────────────

/// Request: the recovery agent asks a custodian for its share of a key.
///
/// The custodian MUST verify that an `attestation:recovery-approval` entry
/// exists on the DHT for this recovery governance action AND that the requesting
/// agent is the recovery subject (or a nominated delegate) before sending the
/// share. This request carries only identifiers; the custodian does the
/// authorization check against the DHT.
///
/// The DHT carries authorization (cheap, durable, witnessed); this wire message
/// carries only the identifiers needed to do the DHT lookup. No credential
/// material travels in the request direction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShamirShareRequest {
    /// ActionHash (base64url) of the `governance-action:recovery-request` entry
    /// that governs this recovery. The custodian resolves this CID against the
    /// DHT to confirm the action is open and authorizes key material release.
    pub recovery_governance_action_cid: String,

    /// Content-addressed identifier (CID) of the custodian being asked.
    ///
    /// Prevents replay: a custodian MUST verify this matches their own identity
    /// before responding. A share request directed at the wrong custodian is
    /// silently rejected (no error response that could be used for enumeration).
    pub custodian_cid: String,
}

/// Response: the custodian delivers its share shard to the recovery agent.
///
/// The custodian MUST have verified the DHT authorization before sending this.
/// The recovery agent MUST verify:
///   1. `attestation_cid` resolves to a real `attestation:recovery-approval` row
///      in the local `attestations` DB projection.
///   2. `signature` verifies against the custodian's known public key.
///   3. The signed payload matches `(recovery_governance_action_cid ‖ share_data ‖
///      share_index_le4)` where `‖` is concatenation and `share_index_le4` is the
///      4-byte little-endian encoding of `share_index`.
///
/// Only after both checks pass does the share contribute to the assembler tally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShamirShareResponse {
    /// Encrypted share bytes.
    ///
    /// Opaque to the transport. The recovery agent decrypts with the session key
    /// established during the recovery-request phase (out of scope for G.1).
    ///
    /// NOTE: share material is intentionally kept off the DHT. The integrity
    /// validator (`attestation_validator.rs`) rejects any `attestation:recovery-approval`
    /// entry that carries `share_data`, `share_index`, or `share_blob` fields in its
    /// `metadata.evidence_json`. See G.3.
    pub share_data: Vec<u8>,

    /// Index of this share within the (m,n) Shamir scheme.
    ///
    /// Indexes are 1-based: the first custodian holds share 1, the second holds
    /// share 2, etc. The assembler uses this to avoid double-counting the same
    /// share from two different transport paths.
    pub share_index: u32,

    /// ActionHash (base64url) of the `attestation:recovery-approval` entry on the
    /// DHT that authorizes this share release.
    ///
    /// The recovery agent looks this up in the local `attestations` SQLite projection
    /// to confirm the custodian actually issued a signed approval before accepting
    /// the share bytes. This ties the ephemeral libp2p delivery back to the durable
    /// DHT social-proof chain.
    pub attestation_cid: String,

    /// Ed25519 signature over the canonical message:
    ///
    /// ```text
    /// message = recovery_governance_action_cid_bytes
    ///         ‖ share_data
    ///         ‖ share_index.to_le_bytes()
    /// ```
    ///
    /// The custodian signs with the same key used to author the
    /// `attestation:recovery-approval` DHT entry. Verification uses
    /// `ed25519_dalek::VerifyingKey::verify_strict`.
    pub signature: Vec<u8>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Wire codec — 4-byte BE length prefix + MessagePack body
// ─────────────────────────────────────────────────────────────────────────────

/// Codec for the Shamir share request-response protocol.
///
/// Wire format: identical to all other elohim request-response codecs —
/// 4-byte big-endian length prefix followed by MessagePack-encoded body.
/// This ensures rolling upgrades can negotiate the protocol version cleanly.
///
/// Maximum message size is deliberately conservative (1 MiB) because share
/// material should never be large — Shamir splits operate on 32-64 byte seeds,
/// not on arbitrarily large blobs.
#[derive(Debug, Clone, Default)]
pub struct ShamirShareCodec;

/// Maximum allowed size for a single Shamir share message (request or response).
///
/// Share seeds are at most 64 bytes; the response envelope adds signatures,
/// attestation CIDs, and MessagePack framing overhead. 1 MiB is generous and
/// guards against accidentally sending large material over this channel.
pub const MAX_SHAMIR_MESSAGE_SIZE: usize = 1 << 20; // 1 MiB

#[async_trait]
impl request_response::Codec for ShamirShareCodec {
    type Protocol = ShamirShareProtocol;
    type Request = ShamirShareRequest;
    type Response = ShamirShareResponse;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > MAX_SHAMIR_MESSAGE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("shamir request too large: {} bytes", len),
            ));
        }
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        rmp_serde::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > MAX_SHAMIR_MESSAGE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("shamir response too large: {} bytes", len),
            ));
        }
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        rmp_serde::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
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
        let data = rmp_serde::to_vec(&request)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len_buf = (data.len() as u32).to_be_bytes();
        io.write_all(&len_buf).await?;
        io.write_all(&data).await?;
        io.flush().await
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
        let data = rmp_serde::to_vec(&response)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let len_buf = (data.len() as u32).to_be_bytes();
        io.write_all(&len_buf).await?;
        io.write_all(&data).await?;
        io.flush().await
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Signature verification helper
// ─────────────────────────────────────────────────────────────────────────────

/// Verify a custodian's signature on a [`ShamirShareResponse`].
///
/// The canonical message is:
/// ```text
/// message = recovery_governance_action_cid.as_bytes()
///         ‖ share_data
///         ‖ share_index.to_le_bytes()
/// ```
///
/// `verifying_key_bytes` must be the 32-byte compressed Ed25519 public key of
/// the custodian (obtained from the `attestation:recovery-approval` DHT entry
/// or the `attestations` SQLite projection).
///
/// Returns `Ok(())` if the signature is valid, `Err(msg)` otherwise.
pub fn verify_share_response(
    response: &ShamirShareResponse,
    recovery_governance_action_cid: &str,
    verifying_key_bytes: &[u8; 32],
) -> Result<(), String> {
    use ed25519_dalek::{Signature, VerifyingKey};

    let key = VerifyingKey::from_bytes(verifying_key_bytes)
        .map_err(|e| format!("verify_share_response: invalid verifying key: {}", e))?;

    let sig_bytes: [u8; 64] = response.signature.as_slice().try_into().map_err(|_| {
        format!(
            "verify_share_response: signature must be 64 bytes, got {}",
            response.signature.len()
        )
    })?;
    let sig = Signature::from_bytes(&sig_bytes);

    // Canonical message: cid_bytes ‖ share_data ‖ share_index_le4
    let mut message = Vec::with_capacity(
        recovery_governance_action_cid.len() + response.share_data.len() + 4,
    );
    message.extend_from_slice(recovery_governance_action_cid.as_bytes());
    message.extend_from_slice(&response.share_data);
    message.extend_from_slice(&response.share_index.to_le_bytes());

    key.verify_strict(&message, &sig)
        .map_err(|e| format!("verify_share_response: signature invalid: {}", e))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip encode/decode of a ShamirShareRequest via MessagePack.
    #[test]
    fn request_roundtrip_msgpack() {
        let req = ShamirShareRequest {
            recovery_governance_action_cid: "uhCkkAbcdef1234567890".to_string(),
            custodian_cid: "uhCqkXyz9876543210".to_string(),
        };
        let encoded = rmp_serde::to_vec(&req).expect("encode request");
        let decoded: ShamirShareRequest =
            rmp_serde::from_slice(&encoded).expect("decode request");
        assert_eq!(decoded.recovery_governance_action_cid, req.recovery_governance_action_cid);
        assert_eq!(decoded.custodian_cid, req.custodian_cid);
    }

    /// Round-trip encode/decode of a ShamirShareResponse via MessagePack.
    #[test]
    fn response_roundtrip_msgpack() {
        let resp = ShamirShareResponse {
            share_data: vec![0xDE, 0xAD, 0xBE, 0xEF],
            share_index: 2,
            attestation_cid: "uhCkkAttestation9999".to_string(),
            signature: vec![0u8; 64],
        };
        let encoded = rmp_serde::to_vec(&resp).expect("encode response");
        let decoded: ShamirShareResponse =
            rmp_serde::from_slice(&encoded).expect("decode response");
        assert_eq!(decoded.share_data, resp.share_data);
        assert_eq!(decoded.share_index, resp.share_index);
        assert_eq!(decoded.attestation_cid, resp.attestation_cid);
        assert_eq!(decoded.signature, resp.signature);
    }

    /// Wire bytes for a minimal request must be small (< 150 bytes).
    #[test]
    fn request_wire_bytes_are_small() {
        let req = ShamirShareRequest {
            recovery_governance_action_cid: "uhCkkAbcdef1234567890".to_string(),
            custodian_cid: "uhCqkXyz9876543210".to_string(),
        };
        let encoded = rmp_serde::to_vec(&req).expect("encode");
        // Heuristic: small identifiers + MessagePack overhead should be < 150 bytes.
        assert!(
            encoded.len() < 150,
            "request unexpectedly large: {} bytes",
            encoded.len()
        );
    }

    /// `verify_share_response` rejects tampered share_data.
    #[test]
    fn verify_rejects_tampered_share_data() {
        use ed25519_dalek::{Signer, SigningKey};

        let mut rng_seed = [42u8; 32];
        let signing_key = SigningKey::from_bytes(&rng_seed);
        let verifying_key: [u8; 32] = signing_key.verifying_key().to_bytes();

        let cid = "uhCkkRecoveryAction001";
        let share_data = b"secret-seed-bytes-32bytes-long!1".to_vec();
        let share_index: u32 = 1;

        // Build canonical message and sign
        let mut message = Vec::new();
        message.extend_from_slice(cid.as_bytes());
        message.extend_from_slice(&share_data);
        message.extend_from_slice(&share_index.to_le_bytes());
        let sig = signing_key.sign(&message);

        let mut resp = ShamirShareResponse {
            share_data: share_data.clone(),
            share_index,
            attestation_cid: "uhCkkAttest001".to_string(),
            signature: sig.to_bytes().to_vec(),
        };

        // Valid signature should pass
        assert!(verify_share_response(&resp, cid, &verifying_key).is_ok());

        // Tamper with share_data — should fail
        resp.share_data[0] ^= 0xFF;
        assert!(verify_share_response(&resp, cid, &verifying_key).is_err());

        // Reset and tamper with share_index — should fail
        rng_seed[0] = 42; // reset
        let signing_key2 = SigningKey::from_bytes(&rng_seed);
        let verifying_key2: [u8; 32] = signing_key2.verifying_key().to_bytes();
        let mut resp2 = ShamirShareResponse {
            share_data: share_data.clone(),
            share_index,
            attestation_cid: "uhCkkAttest001".to_string(),
            signature: sig.to_bytes().to_vec(),
        };
        resp2.share_index = 2; // wrong index
        assert!(verify_share_response(&resp2, cid, &verifying_key2).is_err());
    }
}
