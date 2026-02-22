//! Protocol - P2P message types and wire format
//!
//! Defines the protocol for inter-pod communication.

use serde::{Deserialize, Serialize};

use super::models::*;

/// Protocol version
#[allow(dead_code)]
pub const PROTOCOL_VERSION: &str = "1.0.0";

/// Protocol ID for libp2p
#[allow(dead_code)]
pub const PROTOCOL_ID: &str = "/elohim/agent/1.0.0";

/// Maximum message size in bytes
#[allow(dead_code)]
pub const MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1MB

/// Wire message for pod-to-pod communication
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireMessage {
    /// Protocol version
    pub version: String,
    /// Message ID for request/response correlation
    pub message_id: String,
    /// Optional correlation ID for responses
    pub correlation_id: Option<String>,
    /// Timestamp
    pub timestamp: u64,
    /// Sender node ID
    pub sender: String,
    /// Message payload
    pub payload: AgentMessage,
}

#[allow(dead_code)]
impl WireMessage {
    pub fn new(sender: impl Into<String>, payload: AgentMessage) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            version: PROTOCOL_VERSION.to_string(),
            message_id: uuid::Uuid::new_v4().to_string(),
            correlation_id: None,
            timestamp: now,
            sender: sender.into(),
            payload,
        }
    }

    pub fn response(
        request: &WireMessage,
        sender: impl Into<String>,
        payload: AgentMessage,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            version: PROTOCOL_VERSION.to_string(),
            message_id: uuid::Uuid::new_v4().to_string(),
            correlation_id: Some(request.message_id.clone()),
            timestamp: now,
            sender: sender.into(),
            payload,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        rmp_serde::to_vec(self).map_err(|e| e.to_string())
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        rmp_serde::from_slice(bytes).map_err(|e| e.to_string())
    }

    /// Serialize to JSON (for debugging/logging)
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| e.to_string())
    }
}

/// Message envelope for framing on the wire
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageFrame {
    /// Length of the message in bytes
    pub length: u32,
    /// Checksum for integrity
    pub checksum: u32,
    /// The message bytes
    pub data: Vec<u8>,
}

#[allow(dead_code)]
impl MessageFrame {
    pub fn new(message: &WireMessage) -> Result<Self, String> {
        let data = message.to_bytes()?;

        if data.len() > MAX_MESSAGE_SIZE {
            return Err(format!(
                "Message too large: {} bytes (max {})",
                data.len(),
                MAX_MESSAGE_SIZE
            ));
        }

        let checksum = crc32fast::hash(&data);

        Ok(Self {
            length: data.len() as u32,
            checksum,
            data,
        })
    }

    pub fn verify(&self) -> bool {
        crc32fast::hash(&self.data) == self.checksum
    }

    pub fn into_message(self) -> Result<WireMessage, String> {
        if !self.verify() {
            return Err("Checksum mismatch".to_string());
        }
        WireMessage::from_bytes(&self.data)
    }
}

/// Utility for creating common messages
#[allow(dead_code)]
pub struct MessageBuilder;

#[allow(dead_code)]
impl MessageBuilder {
    /// Create a ping message
    pub fn ping(sender: &str) -> WireMessage {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        WireMessage::new(sender, AgentMessage::Ping { timestamp: now })
    }

    /// Create a pong response
    pub fn pong(request: &WireMessage, sender: &str, node_id: &str) -> WireMessage {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        WireMessage::response(
            request,
            sender,
            AgentMessage::Pong {
                timestamp: now,
                node_id: node_id.to_string(),
            },
        )
    }

    /// Create a share observation message
    pub fn share_observation(sender: &str, observation: Observation) -> WireMessage {
        WireMessage::new(sender, AgentMessage::ShareObservation(observation))
    }

    /// Create a request observations message
    pub fn request_observations(sender: &str, since: u64) -> WireMessage {
        WireMessage::new(sender, AgentMessage::RequestObservations { since })
    }

    /// Create an observation batch response
    pub fn observation_batch(
        request: &WireMessage,
        sender: &str,
        observations: Vec<Observation>,
    ) -> WireMessage {
        WireMessage::response(
            request,
            sender,
            AgentMessage::ObservationBatch { observations },
        )
    }

    /// Create a consensus request
    pub fn consensus_request(sender: &str, request: ConsensusRequest) -> WireMessage {
        WireMessage::new(sender, AgentMessage::ConsensusRequest(request))
    }

    /// Create a consensus response
    pub fn consensus_response(
        request: &WireMessage,
        sender: &str,
        response: ConsensusResponse,
    ) -> WireMessage {
        WireMessage::response(request, sender, AgentMessage::ConsensusResponse(response))
    }

    /// Create a capability advertisement
    pub fn advertise_capabilities(
        sender: &str,
        has_local_inference: bool,
        inference_model: Option<String>,
        available_compute: ComputeCapability,
    ) -> WireMessage {
        WireMessage::new(
            sender,
            AgentMessage::AdvertiseCapabilities {
                has_local_inference,
                inference_model,
                available_compute,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wire_message_roundtrip() {
        let msg = MessageBuilder::ping("test-node");

        let bytes = msg.to_bytes().unwrap();
        let decoded = WireMessage::from_bytes(&bytes).unwrap();

        assert_eq!(msg.message_id, decoded.message_id);
        assert_eq!(msg.sender, decoded.sender);
    }

    #[test]
    fn test_message_frame() {
        let msg = MessageBuilder::ping("test-node");
        let frame = MessageFrame::new(&msg).unwrap();

        assert!(frame.verify());

        let decoded = frame.into_message().unwrap();
        assert_eq!(msg.message_id, decoded.message_id);
    }

    #[test]
    fn test_correlation() {
        let request = MessageBuilder::ping("node-1");
        let response = MessageBuilder::pong(&request, "node-2", "node-2");

        assert_eq!(response.correlation_id, Some(request.message_id.clone()));
    }
}
