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

/// Periodic full-state snapshot. Replaces the receiver's per-peer entries
/// with the snapshot's set. Accepted regardless of sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobInventorySnapshot {
    /// Multibase-encoded libp2p PeerId of the broadcaster.
    pub peer_id: String,
    /// Set of blob hashes the peer currently hosts.
    pub hashes: Vec<String>,
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
    pub added: Vec<String>,
    pub removed: Vec<String>,
    /// Microseconds since epoch — when the delta was emitted.
    pub emitted_at: i64,
    /// Per-peer monotonic counter. Receivers gap-detect on `expected_next` mismatch.
    pub sequence: u64,
    /// Structural non-empty signature (Stage 1). Ed25519 in Stage 2.
    pub signature: Vec<u8>,
}

/// Reasons a wire message can fail structural verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    EmptyPeerId,
    EmptySignature,
    EmptyDelta,
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
    /// - blob hashes look like sha256 hex (64 hex chars) — defensive only
    pub fn verify_structural(&self) -> Result<(), VerifyError> {
        if self.peer_id.is_empty() {
            return Err(VerifyError::EmptyPeerId);
        }
        if self.signature.is_empty() {
            return Err(VerifyError::EmptySignature);
        }
        for hash in &self.hashes {
            if !is_blob_hash_shaped(hash) {
                return Err(VerifyError::InvalidHashFormat(hash.clone()));
            }
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
        for hash in self.added.iter().chain(self.removed.iter()) {
            if !is_blob_hash_shaped(hash) {
                return Err(VerifyError::InvalidHashFormat(hash.clone()));
            }
        }
        Ok(())
    }
}

/// Sha256 hex shape check: 64 lowercase hex chars (defensive structural rule).
fn is_blob_hash_shaped(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_snapshot() -> BlobInventorySnapshot {
        BlobInventorySnapshot {
            peer_id: "12D3KooWtest1".to_string(),
            hashes: vec!["a".repeat(64), "b".repeat(64)],
            snapshot_at: 1_700_000_000_000_000,
            sequence: 42,
            signature: vec![0x00],
        }
    }

    fn sample_delta() -> BlobInventoryDelta {
        BlobInventoryDelta {
            peer_id: "12D3KooWtest1".to_string(),
            added: vec!["c".repeat(64)],
            removed: vec!["a".repeat(64)],
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

    #[test]
    fn snapshot_verify_rejects_malformed_hash() {
        let mut s = sample_snapshot();
        s.hashes.push("notahex!".to_string());
        assert!(matches!(
            s.verify_structural(),
            Err(VerifyError::InvalidHashFormat(_))
        ));
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
    fn delta_verify_rejects_malformed_hash() {
        let mut d = sample_delta();
        d.added.push("notahex!".to_string());
        assert!(matches!(
            d.verify_structural(),
            Err(VerifyError::InvalidHashFormat(_))
        ));
    }

    #[test]
    fn topic_constant_matches_spec() {
        assert_eq!(INVENTORY_TOPIC, "elohim/inventory/blob");
    }
}
