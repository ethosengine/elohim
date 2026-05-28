//! Inventory gossip wire types and structural verification.
//!
//! Topic: `elohim/inventory/blob` (gossipsub).
//! Wire format: MessagePack via `rmp_serde`.
//!
//! ## Why two messages
//!
//! - `BlobInventorySnapshot` is the authoritative full-state replacement.
//!   Receivers replace their per-peer entries with the snapshot's set.
//! - `BlobInventoryDelta` is the event-driven add/remove.
//!   Receivers track per-peer sequence; gap-detect requests a snapshot.
//!
//! ## Stage 1 signature
//!
//! Both messages carry a `signature: Vec<u8>` field that is structurally
//! non-empty (a single null byte is sufficient at Stage 1). Stage 2 will
//! enforce Ed25519 verification over canonical bytes; the structural-non-empty
//! gate is a forward-compatible placeholder.

use serde::{Deserialize, Serialize};

/// Gossipsub topic for blob inventory broadcasts. Wire-level keeps the
/// `blob` identifier even though the broader vocabulary uses `quilt`/`pantry`
/// per the storage-vocabulary memory pin.
pub const INVENTORY_TOPIC: &str = "elohim/inventory/blob";

/// Canonical wire-format blob address. Wraps a `sha256-<64-lower-hex>`
/// string and constructor-validates the shape. Once a `BlobAddress`
/// exists, every downstream consumer can rely on the format without
/// re-checking.
///
/// ## Stage trajectory
/// This is a Stage-2 hardening that co-exists with Stage-1 structural checks.
/// The destination is Stage-4 of the REA-compute-substrate roadmap, where
/// hosting becomes a `Mishpat::Commitment` with `action="serve-url-projection"`
/// and `BlobAddress` becomes the address type referenced in the Commitment's
/// scope field. The newtype survives the graduation unchanged.
///
/// See `genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlobAddress(String);

impl BlobAddress {
    /// Construct from a canonical wire string. Returns Err if the shape
    /// is not `sha256-<64 lowercase hex>`.
    pub fn new(s: impl Into<String>) -> Result<Self, VerifyError> {
        let s = s.into();
        if is_blob_hash_shaped(&s) {
            Ok(Self(s))
        } else {
            Err(VerifyError::InvalidHashFormat(s))
        }
    }

    /// Borrow the inner wire string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for BlobAddress {
    type Error = VerifyError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl From<BlobAddress> for String {
    fn from(b: BlobAddress) -> Self {
        b.0
    }
}

impl std::fmt::Display for BlobAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Periodic full-state snapshot. Replaces the receiver's per-peer entries
/// with the snapshot's set. Accepted regardless of sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobInventorySnapshot {
    /// Multibase-encoded libp2p PeerId of the broadcaster.
    pub peer_id: String,
    /// Set of blob hashes the peer currently hosts. Each entry is a validated
    /// `sha256-<64 lowercase hex>` address; malformed strings are rejected at
    /// deserialize time by `BlobAddress`'s `TryFrom<String>` impl.
    pub hashes: Vec<BlobAddress>,
    /// Microseconds since epoch — when the snapshot was computed.
    pub snapshot_at: i64,
    /// Per-peer monotonic counter. Snapshots advance the receiver's high-watermark.
    pub sequence: u64,
    /// Structural non-empty signature (Stage 1). Ed25519 in Stage 2.
    pub signature: Vec<u8>,
}

/// Event-driven add/remove. Receivers apply against their per-peer set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobInventoryDelta {
    pub peer_id: String,
    /// Hashes added to this peer's inventory. Each entry is a validated
    /// `sha256-<64 lowercase hex>` address.
    pub added: Vec<BlobAddress>,
    /// Hashes removed from this peer's inventory. Each entry is a validated
    /// `sha256-<64 lowercase hex>` address.
    pub removed: Vec<BlobAddress>,
    /// Microseconds since epoch — when the delta was emitted.
    pub emitted_at: i64,
    /// Per-peer monotonic counter. Receivers gap-detect on `expected_next` mismatch.
    pub sequence: u64,
    /// Structural non-empty signature (Stage 1). Ed25519 in Stage 2.
    pub signature: Vec<u8>,
}

/// Reasons a wire message can fail structural verification.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VerifyError {
    #[error("peer ID cannot be empty")]
    EmptyPeerId,
    #[error("signature cannot be empty")]
    EmptySignature,
    #[error("delta must contain at least one add or remove")]
    EmptyDelta,
    #[error("invalid blob hash format: {0}")]
    InvalidHashFormat(String),
}

impl BlobInventorySnapshot {
    /// Encode to MessagePack bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }

    /// Decode from MessagePack bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(bytes)
    }

    /// Structural verification — Stage 1 gate.
    ///
    /// Enforces:
    /// - peer_id is non-empty
    /// - signature is non-empty (Stage 1; Ed25519 in Stage 2)
    ///
    /// Hash-format validation is now enforced at deserialize time by `BlobAddress`'s
    /// `TryFrom<String>` impl, so this method no longer needs to loop over hashes.
    pub fn verify_structural(&self) -> Result<(), VerifyError> {
        if self.peer_id.is_empty() {
            return Err(VerifyError::EmptyPeerId);
        }
        if self.signature.is_empty() {
            return Err(VerifyError::EmptySignature);
        }
        Ok(())
    }
}

impl BlobInventoryDelta {
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(bytes)
    }

    /// Structural verification — Stage 1 gate.
    ///
    /// In addition to the snapshot rules, deltas must carry at least one
    /// add or remove. Empty deltas are protocol violations.
    ///
    /// Hash-format validation is now enforced at deserialize time by `BlobAddress`'s
    /// `TryFrom<String>` impl, so this method no longer needs to loop over hashes.
    pub fn verify_structural(&self) -> Result<(), VerifyError> {
        if self.peer_id.is_empty() {
            return Err(VerifyError::EmptyPeerId);
        }
        if self.signature.is_empty() {
            return Err(VerifyError::EmptySignature);
        }
        if self.added.is_empty() && self.removed.is_empty() {
            return Err(VerifyError::EmptyDelta);
        }
        Ok(())
    }
}

/// Sha256 wire shape check: canonical `sha256-<64 lowercase hex>` per
/// `elohim-storage/CLAUDE.md` ("Wire-level identifiers — `sha256-{hex}` —
/// keep their existing names"). Every producer in the crate emits this
/// shape (BlobStore::store → list_hashes → StoreAdapter::current_hashes);
/// this verifier matches.
fn is_blob_hash_shaped(s: &str) -> bool {
    s.strip_prefix("sha256-").is_some_and(|hex| {
        hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Canonical wire-format hash fixture — matches `BlobStore::store`
    /// output. Production producer always emits this prefix; the verifier
    /// enforces it. Do NOT replace with bare hex.
    fn sha256_wire(byte: char) -> String {
        format!(
            "sha256-{}",
            std::iter::repeat_n(byte, 64).collect::<String>()
        )
    }

    fn sample_snapshot() -> BlobInventorySnapshot {
        BlobInventorySnapshot {
            peer_id: "12D3KooWtest1".to_string(),
            hashes: vec![
                BlobAddress::new(sha256_wire('a')).unwrap(),
                BlobAddress::new(sha256_wire('b')).unwrap(),
            ],
            snapshot_at: 1_700_000_000_000_000,
            sequence: 42,
            signature: vec![0x00],
        }
    }

    fn sample_delta() -> BlobInventoryDelta {
        BlobInventoryDelta {
            peer_id: "12D3KooWtest1".to_string(),
            added: vec![BlobAddress::new(sha256_wire('c')).unwrap()],
            removed: vec![BlobAddress::new(sha256_wire('a')).unwrap()],
            emitted_at: 1_700_000_001_000_000,
            sequence: 43,
            signature: vec![0x00],
        }
    }

    #[test]
    fn snapshot_round_trips() {
        let snapshot = sample_snapshot();
        let bytes = snapshot.to_bytes().unwrap();
        let decoded = BlobInventorySnapshot::from_bytes(&bytes).unwrap();
        assert_eq!(snapshot, decoded);
    }

    #[test]
    fn delta_round_trips() {
        let delta = sample_delta();
        let bytes = delta.to_bytes().unwrap();
        let decoded = BlobInventoryDelta::from_bytes(&bytes).unwrap();
        assert_eq!(delta, decoded);
    }

    #[test]
    fn snapshot_verify_passes_well_formed() {
        assert_eq!(sample_snapshot().verify_structural(), Ok(()));
    }

    #[test]
    fn snapshot_verify_rejects_empty_peer_id() {
        let mut s = sample_snapshot();
        s.peer_id = String::new();
        assert_eq!(s.verify_structural(), Err(VerifyError::EmptyPeerId));
    }

    #[test]
    fn snapshot_verify_rejects_empty_signature() {
        let mut s = sample_snapshot();
        s.signature.clear();
        assert_eq!(s.verify_structural(), Err(VerifyError::EmptySignature));
    }

    /// Wire-path rejection: a snapshot whose `hashes` array contains a bare hex
    /// string (missing `sha256-` prefix) must fail to decode, because
    /// `BlobAddress`'s `TryFrom<String>` impl rejects non-canonical strings at
    /// deserialize time — before `verify_structural` ever runs.
    #[test]
    fn snapshot_decode_rejects_bare_hex_without_prefix() {
        let bad = serde_json::json!({
            "peer_id": "12D3KooWtest1",
            "hashes": ["a".repeat(64)],
            "snapshot_at": 1_700_000_000_000_000_i64,
            "sequence": 42_u64,
            "signature": [0_u8],
        });
        let bytes = rmp_serde::to_vec_named(&bad).unwrap();
        assert!(BlobInventorySnapshot::from_bytes(&bytes).is_err());
    }

    /// Wire-path rejection: wrong algorithm prefix (`sha512-` instead of `sha256-`).
    #[test]
    fn snapshot_decode_rejects_wrong_prefix() {
        let bad = serde_json::json!({
            "peer_id": "12D3KooWtest1",
            "hashes": [format!("sha512-{}", "a".repeat(64))],
            "snapshot_at": 1_700_000_000_000_000_i64,
            "sequence": 42_u64,
            "signature": [0_u8],
        });
        let bytes = rmp_serde::to_vec_named(&bad).unwrap();
        assert!(BlobInventorySnapshot::from_bytes(&bytes).is_err());
    }

    /// Wire-path rejection: correct prefix but wrong hex length (32 instead of 64).
    #[test]
    fn snapshot_decode_rejects_wrong_hex_length() {
        let bad = serde_json::json!({
            "peer_id": "12D3KooWtest1",
            "hashes": [format!("sha256-{}", "a".repeat(32))],
            "snapshot_at": 1_700_000_000_000_000_i64,
            "sequence": 42_u64,
            "signature": [0_u8],
        });
        let bytes = rmp_serde::to_vec_named(&bad).unwrap();
        assert!(BlobInventorySnapshot::from_bytes(&bytes).is_err());
    }

    #[test]
    fn delta_verify_passes_well_formed() {
        assert_eq!(sample_delta().verify_structural(), Ok(()));
    }

    #[test]
    fn delta_verify_rejects_empty_payload() {
        let mut d = sample_delta();
        d.added.clear();
        d.removed.clear();
        assert_eq!(d.verify_structural(), Err(VerifyError::EmptyDelta));
    }

    #[test]
    fn topic_constant_matches_spec() {
        assert_eq!(INVENTORY_TOPIC, "elohim/inventory/blob");
    }

    #[test]
    fn blob_address_accepts_canonical_wire() {
        let addr = BlobAddress::new(sha256_wire('a')).unwrap();
        assert_eq!(addr.as_str(), sha256_wire('a'));
    }

    #[test]
    fn blob_address_rejects_bare_hex() {
        assert!(matches!(
            BlobAddress::new("a".repeat(64)),
            Err(VerifyError::InvalidHashFormat(_))
        ));
    }

    #[test]
    fn blob_address_round_trips_via_serde() {
        let addr = BlobAddress::new(sha256_wire('b')).unwrap();
        let json = serde_json::to_string(&addr).unwrap();
        let decoded: BlobAddress = serde_json::from_str(&json).unwrap();
        assert_eq!(addr, decoded);
    }

    #[test]
    fn blob_address_deserialize_rejects_invalid_shape() {
        let json = "\"not-a-hash\"";
        let result: Result<BlobAddress, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
