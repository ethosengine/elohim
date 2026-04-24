//! Recovery invitation wire contract — the payload published on the
//! `recovery.invitation` gossipsub topic when a new RecoveryRequest is
//! committed to the DHT.
//!
//! Encoding: MessagePack via `rmp-serde`. Matches the EPR-2C codec
//! convention (see `epr_protocol.rs`). Payloads are small (~100 bytes);
//! no length prefix is needed — gossipsub frames the message.

use serde::{Deserialize, Serialize};

/// Broadcast announcement that a recovery request has been committed.
/// Subscribers filter (M5) for invitations relevant to humans their
/// elohim represents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryInvitation {
    /// ActionHash of the RecoveryRequest, base64-string.
    pub request_hash: String,
    /// Legacy String id of the human being recovered.
    pub human_id: String,
    /// ISO-8601 timestamp of the request commit (forwarded from the signal).
    pub created_at: String,
}

impl RecoveryInvitation {
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

    #[test]
    fn roundtrip() {
        let inv = RecoveryInvitation {
            request_hash: "R1".into(),
            human_id: "H1".into(),
            created_at: "2026-04-24T00:00:00Z".into(),
        };
        let bytes = inv.to_bytes().unwrap();
        let decoded = RecoveryInvitation::from_bytes(&bytes).unwrap();
        assert_eq!(inv, decoded);
    }

    #[test]
    fn wire_bytes_are_small() {
        let inv = RecoveryInvitation {
            request_hash: "R1".into(),
            human_id: "H1".into(),
            created_at: "2026-04-24T00:00:00Z".into(),
        };
        // Heuristic — messagepack should keep this under 150 bytes.
        assert!(inv.to_bytes().unwrap().len() < 150);
    }
}
