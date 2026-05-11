//! ## Source of Truth
//!
//! Wire payload (Category C operational protocol) for the libp2p
//! IntegrityNotify direct-notify path. Carries projections of the
//! DHT-notarized KeyRotation entry (imagodei zome). The DHT entry is
//! authoritative; this message is delivery-optimistic — peers can also
//! discover the same rotation via the local conductor's signal stream.
//!
//! Field names mirror [`crate::signals::KeyRotationPayload`] exactly so the
//! wire format is consistent across the direct-notify and signal-stream paths.
//!
//! Coordinated with Recovery M4 producer per
//! project_epr2b_recovery_m4_convergence memory pin.

use serde::{Deserialize, Serialize};

/// MessagePack wire payload for the `IntegrityNotify { kind: "KeyRotation" }`
/// direct-notify path.
///
/// Field names match `KeyRotationPayload` in `signals.rs` (the canonical
/// signal-stream shape) so that Recovery M4 can assemble this message
/// directly from DNA signal fields without renaming.
///
/// `rotation_id` is a synthetic deduplication key (carried as
/// `recovery_request_hash` on the signal side) — stable, coordinator-assigned,
/// and unique per rotation event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryRotationMessage {
    /// Synthetic deduplication key for IntegrityNotify dedup.
    /// Populated with `recovery_request_hash` by the M4 producer.
    pub rotation_id: String,
    /// Base64-encoded AgentPubKey of the human whose key is rotating.
    /// Matches `KeyRotationPayload.human_agent_pubkey`.
    pub human_agent_pubkey: String,
    /// Base64-encoded new AgentPubKey replacing the superseded one.
    /// Matches `KeyRotationPayload.new_agent_pubkey`.
    pub new_agent_pubkey: String,
    /// Base64-encoded AgentPubKey being superseded by this rotation.
    /// Matches `KeyRotationPayload.superseded_agent_pubkey`.
    pub superseded_agent_pubkey: String,
    /// Hash of the recovery request that authorized this rotation.
    /// Matches `KeyRotationPayload.recovery_request_hash`.
    pub recovery_request_hash: String,
    /// ISO-8601 timestamp when the rotation became effective.
    /// Matches `KeyRotationPayload.rotated_at` (converted from Holochain μs).
    pub rotated_at: String,
    /// Base58 PeerId of the publishing node.
    pub sender_peer_id: String,
    /// ISO-8601 timestamp when this message was assembled for publishing.
    pub sent_at: String,
}

impl RecoveryRotationMessage {
    /// Encode to MessagePack bytes for IntegrityNotify direct-notify.
    ///
    /// Named-field MessagePack (struct-as-map) so adding/reordering
    /// fields doesn't silently break across rolling upgrades.
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }

    /// Decode from IntegrityNotify-received bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> RecoveryRotationMessage {
        RecoveryRotationMessage {
            rotation_id: "uhCAkRecoveryRequest-2026-05-11".into(),
            human_agent_pubkey: "uhCAkHumanAgentPubkeyBase64AAAA".into(),
            new_agent_pubkey: "uhCAkNewAgentPubkeyBase64BBBBB".into(),
            superseded_agent_pubkey: "uhCAkOldAgentPubkeyBase64CCCCC".into(),
            recovery_request_hash: "uhCAkRecoveryRequest-2026-05-11".into(),
            rotated_at: "2026-05-11T12:00:00Z".into(),
            sender_peer_id: "12D3KooWtest1234".into(),
            sent_at: "2026-05-11T12:00:01Z".into(),
        }
    }

    #[test]
    fn roundtrip() {
        let msg = sample();
        let bytes = msg.to_bytes().unwrap();
        let decoded = RecoveryRotationMessage::from_bytes(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn wire_bytes_are_small() {
        // MessagePack should keep this well under 400 bytes for typical payloads.
        assert!(sample().to_bytes().unwrap().len() < 400);
    }
}
