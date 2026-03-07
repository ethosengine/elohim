//! Capacity announcement types for gossipsub compute discovery.
//!
//! Nodes periodically broadcast their available compute capacity.
//! Training-wheels: broadcast only, no neighbor table consumption yet.

use serde::{Deserialize, Serialize};

/// Gossipsub topic for compute capacity announcements.
pub const CAPACITY_TOPIC: &str = "/elohim/compute/capacity/1.0.0";

/// Broadcast interval in seconds.
pub const CAPACITY_BROADCAST_INTERVAL_SECS: u64 = 30;

/// A node's compute capacity announcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapacityAnnouncement {
    pub node_id: String,
    pub timestamp: u64,
    pub budget_remaining: u32,
    pub active_requests: u32,
    pub queue_depth: u32,
    pub estimated_tokens_per_sec: f32,
    pub capabilities: Vec<String>,
    pub ready: bool,
}

impl CapacityAnnouncement {
    /// Encode to MessagePack bytes (4-byte BE length prefix + msgpack).
    pub fn encode(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        let msgpack = rmp_serde::encode::to_vec(self)?;
        let len = (msgpack.len() as u32).to_be_bytes();
        let mut buf = Vec::with_capacity(4 + msgpack.len());
        buf.extend_from_slice(&len);
        buf.extend_from_slice(&msgpack);
        Ok(buf)
    }

    /// Decode from MessagePack bytes (skip 4-byte BE length prefix).
    pub fn decode(bytes: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        if bytes.len() < 4 {
            return Err(rmp_serde::decode::Error::LengthMismatch(4));
        }
        rmp_serde::decode::from_slice(&bytes[4..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_announcement() -> CapacityAnnouncement {
        CapacityAnnouncement {
            node_id: "node-123".into(),
            timestamp: 1709827200,
            budget_remaining: 42,
            active_requests: 2,
            queue_depth: 5,
            estimated_tokens_per_sec: 150.0,
            capabilities: vec![
                "path-recommendation".into(),
                "content-safety-review".into(),
            ],
            ready: true,
        }
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let original = sample_announcement();
        let encoded = original.encode().unwrap();
        let decoded = CapacityAnnouncement::decode(&encoded).unwrap();

        assert_eq!(decoded.node_id, "node-123");
        assert_eq!(decoded.budget_remaining, 42);
        assert_eq!(decoded.active_requests, 2);
        assert_eq!(decoded.capabilities.len(), 2);
        assert!(decoded.ready);
    }

    #[test]
    fn test_encode_has_length_prefix() {
        let announcement = sample_announcement();
        let encoded = announcement.encode().unwrap();

        // First 4 bytes are BE length of the msgpack payload
        let len = u32::from_be_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]);
        assert_eq!(len as usize, encoded.len() - 4);
    }

    #[test]
    fn test_json_serialization() {
        let announcement = sample_announcement();
        let json = serde_json::to_string(&announcement).unwrap();
        assert!(json.contains("nodeId"));
        assert!(json.contains("budgetRemaining"));
        assert!(json.contains("estimatedTokensPerSec"));
    }

    #[test]
    fn test_topic_constant() {
        assert_eq!(CAPACITY_TOPIC, "/elohim/compute/capacity/1.0.0");
    }
}
