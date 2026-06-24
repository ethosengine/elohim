//! Salvage-capacity advertisement wire type and structural verification.
//!
//! Topic: `elohim/storage/salvage` (gossipsub).
//! Wire format: MessagePack via `rmp_serde` (map-keyed via `to_vec_named`, so
//! additive fields stay backward-compatible).
//!
//! ## What this is
//!
//! An opt-in, always-on peer (`node`/`steward`) periodically gossips a
//! [`SalvageCapacityAd`] declaring its spare capacity. Receivers project it
//! into the `salvage_capacity` reality table (Category C operational, like
//! `peer_blob_inventory`). The Phase-3 salvage candidate pool = fresh, opted-in
//! entries; `salvage_pass` ranks that pool with the placement-strategy seam to
//! self-select the deterministic closest-N holders for an under-replicated blob.
//!
//! Identity namespace is `agent_cid` throughout — the candidate pool the XOR
//! placement metric ranks over never crosses identity namespaces.
//!
//! ## Stage 1 signature
//!
//! The ad carries a `signature: Vec<u8>` that is structurally non-empty (a
//! single null byte is sufficient at Stage 1). Stage 2 will enforce Ed25519
//! verification over canonical bytes; the structural-non-empty gate is a
//! forward-compatible placeholder. Mirrors the inventory-gossip Stage trajectory.

use serde::{Deserialize, Serialize};

/// Gossipsub topic for salvage-capacity advertisements.
pub const SALVAGE_CAPACITY_TOPIC: &str = "elohim/storage/salvage";

/// An opt-in peer's advertisement of spare salvage capacity. Class C
/// operational; gossiped like inventory. Populates the candidate pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalvageCapacityAd {
    /// Canonical agent identity (`uhCAk…`) of the advertising peer. The pool is
    /// `agent_cid`-keyed so the XOR placement metric never crosses namespaces.
    pub agent_cid: String,
    /// Best-effort spare bytes this peer offers for salvage custody.
    pub spare_bytes: u64,
    /// Always-on class (`node` / `steward`). Only always-on archetypes advertise
    /// by default; carried so diversity/standing strategies (P3-8) can read it.
    pub archetype: String,
    /// Per-peer monotonic counter (advertisement freshness ordering).
    pub seq: u64,
    /// Structural non-empty signature (Stage 1). Ed25519 in Stage 2.
    pub signature: Vec<u8>,
}

/// Reasons a salvage-capacity ad can fail structural verification.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VerifyError {
    #[error("agent_cid cannot be empty")]
    EmptyAgentCid,
    #[error("signature cannot be empty")]
    EmptySignature,
}

impl SalvageCapacityAd {
    /// Encode to MessagePack bytes (map-keyed; additive-field compatible).
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }

    /// Decode from MessagePack bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(bytes)
    }

    /// Structural verification — Stage 1 gate. Rejects empty `agent_cid`
    /// (would corrupt the candidate pool / XOR metric) and empty signature
    /// (Stage 1 non-empty placeholder; Ed25519 in Stage 2).
    pub fn verify_structural(&self) -> Result<(), VerifyError> {
        if self.agent_cid.is_empty() {
            return Err(VerifyError::EmptyAgentCid);
        }
        if self.signature.is_empty() {
            return Err(VerifyError::EmptySignature);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SalvageCapacityAd {
        SalvageCapacityAd {
            agent_cid: "uhCAk-self".to_string(),
            spare_bytes: 8_192,
            archetype: "node".to_string(),
            seq: 7,
            signature: vec![0x00],
        }
    }

    #[test]
    fn roundtrips_through_to_bytes_from_bytes() {
        let ad = sample();
        let bytes = ad.to_bytes().expect("encode");
        let decoded = SalvageCapacityAd::from_bytes(&bytes).expect("decode");
        assert_eq!(ad, decoded);
    }

    #[test]
    fn verify_structural_accepts_well_formed() {
        assert!(sample().verify_structural().is_ok());
    }

    #[test]
    fn verify_structural_rejects_empty_signature() {
        let mut ad = sample();
        ad.signature.clear();
        assert_eq!(
            ad.verify_structural(),
            Err(VerifyError::EmptySignature),
            "empty signature must be rejected (Stage 1 non-empty gate)"
        );
    }

    #[test]
    fn verify_structural_rejects_empty_agent_cid() {
        let mut ad = sample();
        ad.agent_cid.clear();
        assert_eq!(ad.verify_structural(), Err(VerifyError::EmptyAgentCid));
    }
}
