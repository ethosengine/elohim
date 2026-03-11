//! Zome Call Builder - Build MessagePack payloads for Holochain zome calls
//!
//! Provides utilities to construct proper Holochain zome call requests
//! for the conductor's app WebSocket interface.
//!
//! ## DNA Contract
//!
//! For conductor fallback to work, the DNA must implement a `__doorway_get`
//! function that accepts `{ doc_type: String, id: String }` and returns
//! the document data or null.
//!
//! ```rust,ignore
//! // In DNA zome:
//! #[hdk_extern]
//! pub fn __doorway_get(input: DoorwayGetInput) -> ExternResult<Option<DoorwayGetOutput>> {
//!     // Dispatch based on doc_type to actual getters
//!     match input.doc_type.as_str() {
//!         "Content" => get_content_by_id(&input.id),
//!         "LearningPath" => get_path_by_id(&input.id),
//!         _ => Ok(None),  // Unknown type
//!     }
//! }
//! ```

use rmpv::Value;
use serde::{Deserialize, Serialize};

use crate::types::{DoorwayError, Result};

/// Requester identity for access control (passed to DNA)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequesterIdentity {
    /// Requester's agent public key (if authenticated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Requester's geographic location (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// Whether requester is authenticated
    #[serde(default)]
    pub authenticated: bool,
}

/// Input for the DNA's `__doorway_get` function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorwayGetInput {
    /// Document type (e.g., "Content", "LearningPath")
    pub doc_type: String,
    /// Document ID
    pub id: String,
    /// Requester identity for access control (DNA decides based on this)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requester: Option<RequesterIdentity>,
}

/// Input for the DNA's `__doorway_write` function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorwayWriteInput {
    /// Document type (e.g., "Content", "LearningPath")
    pub doc_type: String,
    /// Operation type ("create", "update", "delete")
    pub op_type: String,
    /// Document ID
    pub id: String,
    /// Document data (for create/update)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Entry hash (for update/delete operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_hash: Option<String>,
}

/// Input for the DNA's `__doorway_batch` function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorwayBatchInput {
    /// List of write operations
    pub operations: Vec<DoorwayWriteInput>,
}

/// Configuration for zome calls
#[derive(Debug, Clone)]
pub struct ZomeCallConfig {
    /// DNA hash (base64 encoded)
    pub dna_hash: String,
    /// Agent public key (base64 encoded)
    pub agent_pub_key: String,
    /// Zome name (default: "content_store")
    pub zome_name: String,
    /// Installed app ID
    pub app_id: String,
    /// Role name from hApp manifest (e.g., "lamad", "imagodei", "infrastructure")
    /// Used to identify which DNA this config belongs to in multi-DNA apps
    pub role_name: String,
}

impl Default for ZomeCallConfig {
    fn default() -> Self {
        Self {
            dna_hash: String::new(),
            agent_pub_key: String::new(),
            zome_name: "content_store".to_string(),
            app_id: "elohim".to_string(),
            role_name: "lamad".to_string(),
        }
    }
}

/// Build a zome call payload for conductor
pub struct ZomeCallBuilder {
    config: ZomeCallConfig,
}

impl ZomeCallBuilder {
    /// Create a new builder with the given config
    pub fn new(config: ZomeCallConfig) -> Self {
        Self { config }
    }

    /// Build a `__doorway_get` call payload
    ///
    /// This calls the DNA's generic getter function that doorway uses
    /// for conductor fallback resolution.
    ///
    /// Identity is passed through for DNA to make access control decisions.
    /// Doorway doesn't enforce access - DNA does.
    pub fn build_doorway_get(
        &self,
        doc_type: &str,
        id: &str,
        requester: Option<RequesterIdentity>,
    ) -> Result<Vec<u8>> {
        let input = DoorwayGetInput {
            doc_type: doc_type.to_string(),
            id: id.to_string(),
            requester,
        };

        self.build_zome_call("__doorway_get", &input)
    }

    /// Build a `__doorway_write` call payload
    ///
    /// This calls the DNA's generic write function for single operations.
    pub fn build_doorway_write(
        &self,
        doc_type: &str,
        op_type: &str,
        id: &str,
        data: Option<serde_json::Value>,
        entry_hash: Option<String>,
    ) -> Result<Vec<u8>> {
        let input = DoorwayWriteInput {
            doc_type: doc_type.to_string(),
            op_type: op_type.to_string(),
            id: id.to_string(),
            data,
            entry_hash,
        };

        self.build_zome_call("__doorway_write", &input)
    }

    /// Build a `__doorway_batch` call payload
    ///
    /// This calls the DNA's batch write function for multiple operations.
    /// More efficient than individual calls for bulk operations.
    pub fn build_doorway_batch(&self, operations: Vec<DoorwayWriteInput>) -> Result<Vec<u8>> {
        let input = DoorwayBatchInput { operations };
        self.build_zome_call("__doorway_batch", &input)
    }

    /// Build a generic zome call payload
    pub fn build_zome_call<T: Serialize>(&self, fn_name: &str, payload: &T) -> Result<Vec<u8>> {
        // Serialize the payload to MessagePack
        let payload_bytes = rmp_serde::to_vec(payload)
            .map_err(|e| DoorwayError::Internal(format!("Failed to serialize payload: {e}")))?;

        // Build the zome call request
        // Format: { type: "call_zome", data: { cell_id, zome_name, fn_name, payload, provenance, cap_secret } }
        let call_data = Value::Map(vec![
            (
                Value::String("cell_id".into()),
                Value::Array(vec![
                    Value::Binary(decode_base64(&self.config.dna_hash)?),
                    Value::Binary(decode_base64(&self.config.agent_pub_key)?),
                ]),
            ),
            (
                Value::String("zome_name".into()),
                Value::String(self.config.zome_name.clone().into()),
            ),
            (
                Value::String("fn_name".into()),
                Value::String(fn_name.into()),
            ),
            (
                Value::String("payload".into()),
                Value::Binary(payload_bytes),
            ),
            (
                Value::String("provenance".into()),
                Value::Binary(decode_base64(&self.config.agent_pub_key)?),
            ),
            (
                Value::String("cap_secret".into()),
                Value::Nil, // No capability secret for public functions
            ),
        ]);

        let request = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("call_zome".into()),
            ),
            (Value::String("data".into()), call_data),
        ]);

        let mut buf = Vec::new();
        rmpv::encode::write_value(&mut buf, &request)
            .map_err(|e| DoorwayError::Internal(format!("Failed to encode request: {e}")))?;

        Ok(buf)
    }

    /// Parse a zome call response
    pub fn parse_response<T: for<'de> Deserialize<'de>>(
        &self,
        response: &[u8],
    ) -> Result<Option<T>> {
        // Decode the response envelope
        let mut cursor = std::io::Cursor::new(response);
        let value = rmpv::decode::read_value(&mut cursor)
            .map_err(|e| DoorwayError::Holochain(format!("Failed to decode response: {e}")))?;

        // Extract the response data
        if let Value::Map(ref map) = value {
            // Check for error response
            if let Some(Value::String(ref type_str)) = get_field(map, "type") {
                if type_str.as_str() == Some("error") {
                    if let Some(Value::Map(ref error_map)) = get_field(map, "data") {
                        if let Some(Value::String(ref msg)) = get_field(error_map, "message") {
                            return Err(DoorwayError::Holochain(
                                msg.as_str().unwrap_or("Unknown error").to_string(),
                            ));
                        }
                    }
                    return Err(DoorwayError::Holochain("Unknown zome error".into()));
                }
            }

            // Extract successful response data
            if let Some(Value::Binary(ref data)) = get_field(map, "data") {
                // The data is MessagePack-encoded ExternIO
                // Holochain wraps the actual return value
                let result: Option<T> = rmp_serde::from_slice(data)
                    .map_err(|e| DoorwayError::Holochain(format!("Failed to parse result: {e}")))?;
                return Ok(result);
            }
        }

        // Nil/null response means not found
        Ok(None)
    }
}

/// Decode base64 string to bytes
fn decode_base64(s: &str) -> Result<Vec<u8>> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD
        .decode(s)
        .map_err(|e| DoorwayError::Internal(format!("Invalid base64: {e}")))
}

/// Get a field from a MessagePack map
fn get_field<'a>(map: &'a [(Value, Value)], key: &str) -> Option<&'a Value> {
    for (k, v) in map {
        if let Value::String(k_str) = k {
            if k_str.as_str() == Some(key) {
                return Some(v);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doorway_get_input_serialization() {
        let input = DoorwayGetInput {
            doc_type: "Content".to_string(),
            id: "manifesto".to_string(),
            requester: None,
        };

        let bytes = rmp_serde::to_vec(&input).unwrap();
        let decoded: DoorwayGetInput = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(decoded.doc_type, "Content");
        assert_eq!(decoded.id, "manifesto");
    }

    #[test]
    fn test_doorway_get_input_with_identity() {
        let input = DoorwayGetInput {
            doc_type: "Content".to_string(),
            id: "private-doc".to_string(),
            requester: Some(RequesterIdentity {
                agent_id: Some("uhCAk...".to_string()),
                location: None,
                authenticated: true,
            }),
        };

        // Test JSON serialization (used at API boundary)
        let json = serde_json::to_string(&input).unwrap();
        let decoded: DoorwayGetInput = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.doc_type, "Content");
        assert!(decoded.requester.is_some());
        assert!(decoded.requester.unwrap().authenticated);
    }

    #[test]
    fn test_requester_identity_default() {
        let identity = RequesterIdentity::default();
        assert!(!identity.authenticated);
        assert!(identity.agent_id.is_none());
    }

    #[test]
    fn test_config_default() {
        let config = ZomeCallConfig::default();
        assert_eq!(config.zome_name, "content_store");
        assert_eq!(config.app_id, "elohim");
        assert_eq!(config.role_name, "lamad");
    }
}
