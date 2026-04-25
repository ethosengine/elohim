//! Recovery revocation wire contract — the payload published on the
//! `recovery.revocation` gossipsub topic when a KeyRevocation is committed
//! (either requested or becomes effective).
//!
//! Encoding: MessagePack via `rmp-serde`. Matches the EPR-2C codec
//! convention (see `epr_protocol.rs`). Payloads are small (~150 bytes);
//! no length prefix is needed — gossipsub frames the message.
//!
//! M4 wire contract per spec §7.2. Active consumer logic (M5 elohim defender)
//! will subscribe to this module's type. M4 only subscribes and logs.

use serde::{Deserialize, Serialize};

/// MessagePack wire payload for the `recovery.revocation` gossipsub topic.
///
/// Published for `KeyRevocationRequested` and `KeyRevocationEffective` signals.
/// `RevocationVoteSubmitted` is NOT published here — votes are DHT-authoritative
/// and not gossiped separately (spec §7.1 decision #6).
///
/// Subscribers filter on `revocation_id` for events relevant to their humans.
/// The `status` field is "pending" | "effective".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryRevocationMessage {
    /// Coordinator-assigned revocation ID (stable deduplication key).
    pub revocation_id: String,
    /// Legacy String human id of the human whose key is being revoked.
    pub human_id: String,
    /// Base64-encoded AgentPubKey being revoked.
    pub revoked_key: String,
    /// "voluntary" | "steward_vote" | "challenge". May be empty string on
    /// Effective-only signals where trigger_type is not re-carried.
    pub trigger_type: String,
    /// REVOCATION_REASONS member. May be empty string on Effective-only signals.
    pub reason: String,
    /// "pending" | "effective"
    pub status: String,
    /// Base58 PeerId of the publishing node.
    pub sender_peer_id: String,
    /// ISO-8601 timestamp when this message was assembled for publishing.
    pub sent_at: String,
}

impl RecoveryRevocationMessage {
    /// Encode to MessagePack bytes for gossipsub publish.
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(self)
    }

    /// Decode from gossipsub-received bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> RecoveryRevocationMessage {
        RecoveryRevocationMessage {
            revocation_id: "rev-human-matthew-2026-04-24T00:00:00Z".into(),
            human_id: "human-matthew".into(),
            revoked_key: "uhCAkABCD1234567890".into(),
            trigger_type: "voluntary".into(),
            reason: "compromised".into(),
            status: "effective".into(),
            sender_peer_id: "12D3KooW...".into(),
            sent_at: "2026-04-24T00:00:00Z".into(),
        }
    }

    #[test]
    fn roundtrip() {
        let msg = sample();
        let bytes = msg.to_bytes().unwrap();
        let decoded = RecoveryRevocationMessage::from_bytes(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn wire_bytes_are_small() {
        // MessagePack should keep this well under 300 bytes for typical payloads.
        assert!(sample().to_bytes().unwrap().len() < 300);
    }

    #[test]
    fn pending_message_roundtrips() {
        let mut msg = sample();
        msg.status = "pending".into();
        msg.trigger_type = "steward_vote".into();
        let bytes = msg.to_bytes().unwrap();
        let decoded = RecoveryRevocationMessage::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.status, "pending");
    }
}
