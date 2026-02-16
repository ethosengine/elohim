//! NATS message types for Holochain WebSocket proxying
//!
//! Defines the request/response messages used for routing WebSocket
//! connections through NATS to backend Holochain hosts.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Subject prefix for Holochain WebSocket requests
pub const HC_WS_SUBJECT_PREFIX: &str = "HC.WS";

/// Request to establish a WebSocket connection via NATS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HcWsRequest {
    /// Unique request ID
    pub request_id: String,

    /// Target host node ID (if known, for sticky sessions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_host: Option<String>,

    /// Geographic region preference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_region: Option<String>,

    /// The operation being requested (for routing decisions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,

    /// Original client origin
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,

    /// User identifier (for user-to-host affinity)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    /// The WebSocket message payload (binary as base64)
    pub payload: String,
}

impl HcWsRequest {
    /// Create a new request with a generated ID
    pub fn new(payload: Vec<u8>) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            target_host: None,
            preferred_region: None,
            operation: None,
            origin: None,
            user_id: None,
            payload: base64_encode(&payload),
        }
    }

    /// Set the target host for sticky sessions
    pub fn with_target_host(mut self, host: String) -> Self {
        self.target_host = Some(host);
        self
    }

    /// Set the preferred region
    pub fn with_region(mut self, region: String) -> Self {
        self.preferred_region = Some(region);
        self
    }

    /// Set the operation type
    pub fn with_operation(mut self, operation: String) -> Self {
        self.operation = Some(operation);
        self
    }

    /// Set the origin
    pub fn with_origin(mut self, origin: String) -> Self {
        self.origin = Some(origin);
        self
    }

    /// Set the user ID
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Get the NATS subject for this request
    pub fn subject(&self) -> String {
        if let Some(ref host) = self.target_host {
            format!("{HC_WS_SUBJECT_PREFIX}.{host}")
        } else if let Some(ref region) = self.preferred_region {
            format!("{HC_WS_SUBJECT_PREFIX}.REGION.{region}")
        } else {
            format!("{HC_WS_SUBJECT_PREFIX}.ANY")
        }
    }

    /// Decode the payload from base64
    pub fn decode_payload(&self) -> Result<Vec<u8>, base64::DecodeError> {
        base64_decode(&self.payload)
    }

    /// Serialize to JSON bytes
    pub fn to_bytes(&self) -> Result<bytes::Bytes, serde_json::Error> {
        serde_json::to_vec(self).map(Into::into)
    }

    /// Deserialize from JSON bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}

/// Response from a Holochain WebSocket request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HcWsResponse {
    /// Original request ID
    pub request_id: String,

    /// Whether the request succeeded
    pub success: bool,

    /// Host node ID that handled the request
    pub host_id: String,

    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// The response payload (binary as base64)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
}

impl HcWsResponse {
    /// Create a successful response
    pub fn success(request_id: String, host_id: String, payload: Vec<u8>) -> Self {
        Self {
            request_id,
            success: true,
            host_id,
            error: None,
            payload: Some(base64_encode(&payload)),
        }
    }

    /// Create an error response
    pub fn error(request_id: String, host_id: String, error: String) -> Self {
        Self {
            request_id,
            success: false,
            host_id,
            error: Some(error),
            payload: None,
        }
    }

    /// Decode the payload from base64
    pub fn decode_payload(&self) -> Option<Vec<u8>> {
        self.payload.as_ref().and_then(|p| base64_decode(p).ok())
    }

    /// Serialize to JSON bytes
    pub fn to_bytes(&self) -> Result<bytes::Bytes, serde_json::Error> {
        serde_json::to_vec(self).map(Into::into)
    }

    /// Deserialize from JSON bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}

/// Host heartbeat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostHeartbeat {
    /// Host node ID
    pub host_id: String,

    /// Host status
    pub status: String,

    /// Current active connections
    pub active_connections: i32,

    /// Maximum connections
    pub max_connections: i32,

    /// Geographic region
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,

    /// Software version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

impl HostHeartbeat {
    /// Create a new heartbeat
    pub fn new(host_id: String, active_connections: i32, max_connections: i32) -> Self {
        Self {
            host_id,
            status: "online".to_string(),
            active_connections,
            max_connections,
            region: None,
            version: None,
        }
    }

    /// Subject for heartbeat messages
    pub fn subject() -> &'static str {
        "HC.HOST.HEARTBEAT"
    }

    /// Serialize to JSON bytes
    pub fn to_bytes(&self) -> Result<bytes::Bytes, serde_json::Error> {
        serde_json::to_vec(self).map(Into::into)
    }
}

// Base64 encoding helpers using the base64 crate
fn base64_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD.encode(data)
}

fn base64_decode(data: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD.decode(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_subject_any() {
        let req = HcWsRequest::new(vec![1, 2, 3]);
        assert_eq!(req.subject(), "HC.WS.ANY");
    }

    #[test]
    fn test_request_subject_host() {
        let req = HcWsRequest::new(vec![1, 2, 3]).with_target_host("host-123".to_string());
        assert_eq!(req.subject(), "HC.WS.host-123");
    }

    #[test]
    fn test_request_subject_region() {
        let req = HcWsRequest::new(vec![1, 2, 3]).with_region("us-west".to_string());
        assert_eq!(req.subject(), "HC.WS.REGION.us-west");
    }

    #[test]
    fn test_request_roundtrip() {
        let original = HcWsRequest::new(vec![1, 2, 3, 4, 5])
            .with_operation("list_apps".to_string())
            .with_origin("https://example.com".to_string());

        let bytes = original.to_bytes().unwrap();
        let decoded = HcWsRequest::from_bytes(&bytes).unwrap();

        assert_eq!(original.request_id, decoded.request_id);
        assert_eq!(original.operation, decoded.operation);
        assert_eq!(original.payload, decoded.payload);
    }

    #[test]
    fn test_response_success() {
        let resp =
            HcWsResponse::success("req-123".to_string(), "host-456".to_string(), vec![1, 2, 3]);

        assert!(resp.success);
        assert!(resp.error.is_none());
        assert!(resp.payload.is_some());
    }

    #[test]
    fn test_response_error() {
        let resp = HcWsResponse::error(
            "req-123".to_string(),
            "host-456".to_string(),
            "oops".to_string(),
        );

        assert!(!resp.success);
        assert_eq!(resp.error, Some("oops".to_string()));
        assert!(resp.payload.is_none());
    }

    #[test]
    fn test_payload_roundtrip() {
        let original_payload = vec![0u8, 1, 2, 255, 128, 64];
        let req = HcWsRequest::new(original_payload.clone());
        let decoded = req.decode_payload().unwrap();
        assert_eq!(original_payload, decoded);
    }
}
