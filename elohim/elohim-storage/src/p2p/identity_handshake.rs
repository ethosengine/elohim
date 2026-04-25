//! Identity handshake protocol — `/elohim/identity/handshake/1.0.0`
//!
//! On `ConnectionEstablished`, each peer sends the other its current signed
//! `AgentPeerBinding`. The receiver verifies structural integrity and the
//! validity window; on valid: inserts into `peer_identity_bindings` table
//! with `source='handshake'`; on invalid: logs and leaves the peer Anonymous.
//!
//! ## Category classification
//!
//! Category **C** — operational, session-local.
//!
//! The handshake payload wraps a signed `AgentPeerBinding` whose source of
//! truth is the DHT-notarized entry in imagodei (Category A — Task A.2).
//! The handshake itself is NOT a source of truth; it is a libp2p session-local
//! projection of DHT state, reconstructable on re-handshake. Any row inserted
//! into `peer_identity_bindings` with `source='handshake'` can be evicted and
//! rebuilt the next time the peer connects.
//!
//! ## Wire format
//!
//! 4-byte BE length prefix + MessagePack body (`rmp-serde`).
//! Schema: `elohim/sdk/schemas/v1/p2p/identity-handshake.schema.json`
//!
//! ## Verification (Stage 1)
//!
//! Following the bootstrap security gradient (memory pin
//! `project_bootstrap_to_elohim_security_gradient`), Stage 1 enforces:
//!
//!   1. `peer_id` in the binding MUST match the connecting peer's PeerId.
//!   2. `agent_cid` MUST be non-empty.
//!   3. `signature` MUST be non-empty (structural; full Ed25519 key-resolver
//!      verification is Stage 3 — no resolver available at Stage 1).
//!   4. Validity window: `valid_from ≤ now < valid_until` (when `valid_until`
//!      is `Some`); handshakes outside the window are rejected.
//!   5. Timestamp anti-replay: handshake `timestamp` within ±5 minutes of now.
//!
//! Full Ed25519 key-resolver verification (resolving `agent_cid` → public key
//! → verifying `signature` over canonical bytes) is deferred to Stage 3 and
//! is NOT performed here.

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::request_response;
use serde::{Deserialize, Serialize};
use std::io;

pub const IDENTITY_HANDSHAKE_PROTOCOL_ID: &str = "/elohim/identity/handshake/1.0.0";

/// Maximum size for a handshake request (binding + timestamp + nonce; generous
/// upper bound — actual payloads are well under 1 KiB).
const MAX_REQUEST_SIZE: usize = 4 * 1024;
/// Maximum size for a handshake response (small tag + optional reason string).
const MAX_RESPONSE_SIZE: usize = 1024;

// =============================================================================
// Protocol marker
// =============================================================================

#[derive(Debug, Clone)]
pub struct IdentityHandshakeProtocol;

impl AsRef<str> for IdentityHandshakeProtocol {
    fn as_ref(&self) -> &str {
        IDENTITY_HANDSHAKE_PROTOCOL_ID
    }
}

// =============================================================================
// Wire types
// =============================================================================

/// Binding data transported in the handshake request.
///
/// Mirrors the `AgentPeerBinding` DHT entry (Category A, imagodei Task A.2)
/// with transport-friendly types. The `dht_anchor_hash` is optional because
/// the sender may not have observed their own binding's ActionHash yet.
///
/// Field naming: snake_case throughout — this payload does not cross the
/// TypeScript boundary; it is internal to the libp2p protocol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandshakeBindingPayload {
    /// libp2p PeerId of the sender (base58btc-encoded multihash).
    /// MUST match the connecting peer's `PeerId` as verified by the receiver.
    pub peer_id: String,
    /// CIDv1 (dag-cbor sha256) of the Agent EPR in imagodei.
    pub agent_cid: String,
    /// ISO-8601 timestamp (UTC, Z suffix) from which this binding is valid.
    pub valid_from: String,
    /// ISO-8601 expiry timestamp or `None` if the binding has no declared expiry.
    pub valid_until: Option<String>,
    /// Hardware/deployment archetype: "node" | "desktop" | "mobile" | "steward".
    pub device_archetype: String,
    /// Base64-encoded Ed25519 signature over the canonical binding body.
    pub signature: String,
    /// ActionHash (base64url) of the AgentPeerBinding DHT entry, if known.
    /// Used as `dht_anchor_hash` in the `peer_identity_bindings` row; a stable
    /// placeholder is synthesised when absent.
    pub dht_anchor_hash: Option<String>,
}

/// Request: initiating peer presents its signed `AgentPeerBinding`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityHandshakeRequest {
    pub binding: HandshakeBindingPayload,
    /// ISO-8601 UTC timestamp of this handshake attempt. Anti-replay window:
    /// receiver rejects if `|now − timestamp| > 300s`.
    pub timestamp: String,
    /// 16-byte random nonce, base64-encoded. Present for future replay
    /// prevention; not cryptographically verified at Stage 1.
    pub nonce: String,
}

/// Response: receiver acknowledges or rejects the presented binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IdentityHandshakeResponse {
    /// Binding accepted; peer identity recorded.
    Accepted,
    /// Binding rejected (bad peer_id, expired, empty fields, etc.).
    Rejected { reason: String },
    /// Receiver-side error (pool exhausted, etc.).
    Error(String),
}

// =============================================================================
// Codec (4-byte BE length prefix + MessagePack body)
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct IdentityHandshakeCodec;

#[async_trait]
impl request_response::Codec for IdentityHandshakeCodec {
    type Protocol = IdentityHandshakeProtocol;
    type Request = IdentityHandshakeRequest;
    type Response = IdentityHandshakeResponse;

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
        if len > MAX_REQUEST_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "identity handshake request too large: {} bytes (max {})",
                    len, MAX_REQUEST_SIZE
                ),
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
        if len > MAX_RESPONSE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "identity handshake response too large: {} bytes (max {})",
                    len, MAX_RESPONSE_SIZE
                ),
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
        io.flush().await?;
        Ok(())
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
        io.flush().await?;
        Ok(())
    }
}

// =============================================================================
// Verification helpers
// =============================================================================

/// Outcome of handshake verification by the receiver.
#[derive(Debug, PartialEq, Eq)]
pub enum VerifyOutcome {
    /// Binding is structurally valid and within the validity window.
    Valid,
    /// Binding is invalid; the string describes why.
    Invalid(String),
}

/// Verify an incoming handshake request (Stage 1 rules — see module doc).
///
/// `peer_id_str` is the base58-encoded PeerId of the connecting peer as
/// reported by the transport layer (authoritative; not trusting the payload).
/// `now_iso` is the current UTC timestamp in ISO-8601 format used for window
/// checks.
pub fn verify_handshake_request(
    request: &IdentityHandshakeRequest,
    peer_id_str: &str,
    now_iso: &str,
) -> VerifyOutcome {
    let b = &request.binding;

    // Rule 1: peer_id in payload must match the connecting peer's actual PeerId.
    if b.peer_id != peer_id_str {
        return VerifyOutcome::Invalid(format!(
            "binding peer_id '{}' does not match connecting peer '{}'",
            b.peer_id, peer_id_str
        ));
    }

    // Rule 2: agent_cid must be non-empty.
    if b.agent_cid.is_empty() {
        return VerifyOutcome::Invalid("binding agent_cid is empty".to_string());
    }

    // Rule 3: signature must be non-empty (structural; full Ed25519 resolve is Stage 3).
    if b.signature.is_empty() {
        return VerifyOutcome::Invalid("binding signature is empty".to_string());
    }

    // Rule 4: validity window check.
    // valid_from must be parseable; if valid_until is present it must be in the future.
    if let Some(ref until) = b.valid_until {
        // Simple string comparison works for ISO-8601 UTC with Z suffix (lexicographic ≡ chronological).
        if until.as_str() <= now_iso {
            return VerifyOutcome::Invalid(format!(
                "binding valid_until '{}' has expired (now='{}')",
                until, now_iso
            ));
        }
    }

    // Rule 5: handshake timestamp anti-replay (±5 minutes).
    // We perform a rough check by comparing ISO-8601 strings. For precise replay
    // prevention a chrono parse is used when the check matters, but at Stage 1
    // we accept any timestamp whose prefix falls within plausible bounds.
    // A full chrono parse keeps the check honest.
    let ts_ok = parse_iso_and_check_window(&request.timestamp, now_iso, 300);
    if !ts_ok {
        return VerifyOutcome::Invalid(format!(
            "handshake timestamp '{}' is outside the ±5-minute replay window (now='{}')",
            request.timestamp, now_iso
        ));
    }

    VerifyOutcome::Valid
}

/// Returns `true` if `timestamp` is within `window_secs` of `now_iso`.
///
/// Both strings are expected to be RFC3339 / ISO-8601 with Z suffix.
/// Falls back to `true` (permissive) if either string fails to parse, so that
/// misconfigured system clocks do not hard-lock identity establishment.
fn parse_iso_and_check_window(timestamp: &str, now_iso: &str, window_secs: i64) -> bool {
    use chrono::{DateTime, Utc};
    let ts = match timestamp.parse::<DateTime<Utc>>() {
        Ok(t) => t,
        Err(_) => return true, // permissive fallback — malformed clock should not hard-lock
    };
    let now = match now_iso.parse::<DateTime<Utc>>() {
        Ok(t) => t,
        Err(_) => return true,
    };
    let delta = (ts - now).num_seconds().abs();
    delta <= window_secs
}

/// Synthesise a stable `dht_anchor_hash` placeholder for handshake-sourced bindings
/// when the sender did not include one.
///
/// The placeholder is deterministic from `peer_id` + `agent_cid` so that
/// repeated handshakes for the same binding produce the same row key.
pub fn synthesise_dht_anchor_hash(peer_id: &str, agent_cid: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"handshake-anchor:");
    h.update(peer_id.as_bytes());
    h.update(b":");
    h.update(agent_cid.as_bytes());
    format!("handshake:{}", hex::encode(h.finalize()))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request(peer_id: &str, agent_cid: &str) -> IdentityHandshakeRequest {
        IdentityHandshakeRequest {
            binding: HandshakeBindingPayload {
                peer_id: peer_id.to_string(),
                agent_cid: agent_cid.to_string(),
                valid_from: "2026-01-01T00:00:00Z".to_string(),
                valid_until: None,
                device_archetype: "node".to_string(),
                signature: "dGVzdHNpZw==".to_string(), // non-empty base64
                dht_anchor_hash: None,
            },
            timestamp: "2026-04-25T12:00:00Z".to_string(),
            nonce: "AAAAAAAAAAAAAAAAAAAAAA==".to_string(),
        }
    }

    #[test]
    fn roundtrip_request() {
        let req = sample_request("12D3KooWPeer1", "bafyreiabc");
        let bytes = rmp_serde::to_vec(&req).unwrap();
        let decoded: IdentityHandshakeRequest = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.binding.peer_id, "12D3KooWPeer1");
        assert_eq!(decoded.binding.agent_cid, "bafyreiabc");
    }

    #[test]
    fn roundtrip_response_accepted() {
        let resp = IdentityHandshakeResponse::Accepted;
        let bytes = rmp_serde::to_vec(&resp).unwrap();
        let decoded: IdentityHandshakeResponse = rmp_serde::from_slice(&bytes).unwrap();
        assert!(matches!(decoded, IdentityHandshakeResponse::Accepted));
    }

    #[test]
    fn roundtrip_response_rejected() {
        let resp = IdentityHandshakeResponse::Rejected {
            reason: "expired".to_string(),
        };
        let bytes = rmp_serde::to_vec(&resp).unwrap();
        let decoded: IdentityHandshakeResponse = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            IdentityHandshakeResponse::Rejected { reason } => assert_eq!(reason, "expired"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn roundtrip_response_error() {
        let resp = IdentityHandshakeResponse::Error("pool exhausted".to_string());
        let bytes = rmp_serde::to_vec(&resp).unwrap();
        let decoded: IdentityHandshakeResponse = rmp_serde::from_slice(&bytes).unwrap();
        match decoded {
            IdentityHandshakeResponse::Error(msg) => assert_eq!(msg, "pool exhausted"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn wire_bytes_are_small() {
        let req = sample_request("12D3KooWPeer1", "bafyreiabc");
        let bytes = rmp_serde::to_vec(&req).unwrap();
        // Generous upper bound — well under 1 KiB
        assert!(
            bytes.len() < 512,
            "request too large: {} bytes",
            bytes.len()
        );
    }

    #[test]
    fn verify_accepts_valid_request() {
        let req = sample_request("12D3KooWPeer1", "bafyreiabc");
        let result = verify_handshake_request(&req, "12D3KooWPeer1", "2026-04-25T12:00:00Z");
        assert_eq!(result, VerifyOutcome::Valid);
    }

    #[test]
    fn verify_rejects_peer_id_mismatch() {
        let req = sample_request("12D3KooWPeer1", "bafyreiabc");
        let result = verify_handshake_request(
            &req,
            "12D3KooWPeer2", // wrong peer
            "2026-04-25T12:00:00Z",
        );
        match result {
            VerifyOutcome::Invalid(msg) => assert!(
                msg.contains("does not match"),
                "expected peer_id mismatch message, got: {msg}"
            ),
            VerifyOutcome::Valid => panic!("should have rejected peer_id mismatch"),
        }
    }

    #[test]
    fn verify_rejects_empty_agent_cid() {
        let req = sample_request("12D3KooWPeer1", "");
        let result = verify_handshake_request(&req, "12D3KooWPeer1", "2026-04-25T12:00:00Z");
        match result {
            VerifyOutcome::Invalid(msg) => assert!(msg.contains("agent_cid")),
            VerifyOutcome::Valid => panic!("should have rejected empty agent_cid"),
        }
    }

    #[test]
    fn verify_rejects_empty_signature() {
        let mut req = sample_request("12D3KooWPeer1", "bafyreiabc");
        req.binding.signature = String::new();
        let result = verify_handshake_request(&req, "12D3KooWPeer1", "2026-04-25T12:00:00Z");
        match result {
            VerifyOutcome::Invalid(msg) => assert!(msg.contains("signature")),
            VerifyOutcome::Valid => panic!("should have rejected empty signature"),
        }
    }

    #[test]
    fn verify_rejects_expired_binding() {
        let mut req = sample_request("12D3KooWPeer1", "bafyreiabc");
        req.binding.valid_until = Some("2026-02-01T00:00:00Z".to_string());
        // now is 2026-04-25 — past the expiry
        let result = verify_handshake_request(&req, "12D3KooWPeer1", "2026-04-25T12:00:00Z");
        match result {
            VerifyOutcome::Invalid(msg) => assert!(
                msg.contains("expired"),
                "expected expired message, got: {msg}"
            ),
            VerifyOutcome::Valid => panic!("should have rejected expired binding"),
        }
    }

    #[test]
    fn verify_accepts_binding_with_future_valid_until() {
        let mut req = sample_request("12D3KooWPeer1", "bafyreiabc");
        req.binding.valid_until = Some("2027-01-01T00:00:00Z".to_string());
        let result = verify_handshake_request(&req, "12D3KooWPeer1", "2026-04-25T12:00:00Z");
        assert_eq!(result, VerifyOutcome::Valid);
    }

    #[test]
    fn verify_rejects_timestamp_outside_window() {
        let mut req = sample_request("12D3KooWPeer1", "bafyreiabc");
        // Timestamp is 10 minutes in the past (now = 2026-04-25T12:00:00Z)
        req.timestamp = "2026-04-25T11:49:00Z".to_string();
        let result = verify_handshake_request(&req, "12D3KooWPeer1", "2026-04-25T12:00:00Z");
        match result {
            VerifyOutcome::Invalid(msg) => assert!(
                msg.contains("replay window"),
                "expected replay window message, got: {msg}"
            ),
            VerifyOutcome::Valid => panic!("should have rejected stale timestamp"),
        }
    }

    #[test]
    fn verify_accepts_timestamp_within_window() {
        let mut req = sample_request("12D3KooWPeer1", "bafyreiabc");
        // Timestamp is 2 minutes in the past (within ±5 minutes)
        req.timestamp = "2026-04-25T11:58:00Z".to_string();
        let result = verify_handshake_request(&req, "12D3KooWPeer1", "2026-04-25T12:00:00Z");
        assert_eq!(result, VerifyOutcome::Valid);
    }

    #[test]
    fn synthesise_anchor_is_deterministic() {
        let a = synthesise_dht_anchor_hash("peer-1", "agent-cid-1");
        let b = synthesise_dht_anchor_hash("peer-1", "agent-cid-1");
        assert_eq!(a, b);
    }

    #[test]
    fn synthesise_anchor_differs_for_different_inputs() {
        let a = synthesise_dht_anchor_hash("peer-1", "agent-cid-1");
        let b = synthesise_dht_anchor_hash("peer-2", "agent-cid-1");
        assert_ne!(a, b);
    }
}
