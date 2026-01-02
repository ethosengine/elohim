//! Holochain Wire Protocol
//!
//! Single responsibility: Encode and decode messages in Holochain's msgpack wire format.
//!
//! # Wire Format
//!
//! Holochain uses MessagePack with a specific envelope structure:
//!
//! ## Request Envelope (outer)
//! ```text
//! {
//!     "id": <u64>,           // Request ID for correlation
//!     "type": "request",     // Always "request" for outgoing
//!     "data": <binary>,      // Inner request as msgpack bytes
//! }
//! ```
//!
//! ## Inner Request (for zome calls)
//! ```text
//! {
//!     "type": "call_zome",
//!     "value": {
//!         "cell_id": <binary>,
//!         "zome_name": <string>,
//!         "fn_name": <string>,
//!         "payload": <binary>,
//!         "cap_secret": null,
//!         "provenance": null,
//!     }
//! }
//! ```
//!
//! ## Response Envelope
//! ```text
//! {
//!     "id": <u64>,           // Matches request ID
//!     "type": "response" | "error",
//!     "data": <binary>,      // Result or error message
//! }
//! ```

use rmpv::Value;
use std::io::Cursor;

use crate::error::StorageError;

/// A request ready to be sent over the wire.
pub struct EncodedRequest {
    pub id: u64,
    pub bytes: Vec<u8>,
}

/// A decoded response from the conductor.
pub struct DecodedResponse {
    pub id: u64,
    pub result: Result<Vec<u8>, String>,
}

/// Encode a zome call into wire format.
///
/// # Arguments
/// * `id` - Request ID for correlating the response
/// * `cell_id` - The cell to call (DNA hash + agent pubkey)
/// * `zome_name` - Name of the zome
/// * `fn_name` - Name of the function
/// * `payload` - Msgpack-encoded function arguments
///
/// # Returns
/// An `EncodedRequest` containing the ID and the bytes to send.
pub fn encode_zome_call(
    id: u64,
    cell_id: &[u8],
    zome_name: &str,
    fn_name: &str,
    payload: &[u8],
) -> Result<EncodedRequest, StorageError> {
    use rmpv::encode::write_value;

    // Build the inner call_zome request
    // Note: Holochain 0.6+ uses "value" not "data" for the inner structure
    let call_data = Value::Map(vec![
        (Value::String("cell_id".into()), Value::Binary(cell_id.to_vec())),
        (Value::String("zome_name".into()), Value::String(zome_name.into())),
        (Value::String("fn_name".into()), Value::String(fn_name.into())),
        (Value::String("payload".into()), Value::Binary(payload.to_vec())),
        (Value::String("cap_secret".into()), Value::Nil),
        (Value::String("provenance".into()), Value::Nil),
    ]);

    let inner_request = Value::Map(vec![
        (Value::String("type".into()), Value::String("call_zome".into())),
        (Value::String("value".into()), call_data),
    ]);

    // Serialize inner request to bytes
    let mut inner_bytes = Vec::new();
    write_value(&mut inner_bytes, &inner_request)
        .map_err(|e| StorageError::Internal(format!("Failed to encode inner request: {}", e)))?;

    // Build outer envelope
    // Note: Outer envelope uses "data" for the binary payload
    let envelope = Value::Map(vec![
        (Value::String("id".into()), Value::Integer(id.into())),
        (Value::String("type".into()), Value::String("request".into())),
        (Value::String("data".into()), Value::Binary(inner_bytes)),
    ]);

    let mut bytes = Vec::new();
    write_value(&mut bytes, &envelope)
        .map_err(|e| StorageError::Internal(format!("Failed to encode envelope: {}", e)))?;

    Ok(EncodedRequest { id, bytes })
}

/// Decode a response from wire format.
///
/// # Arguments
/// * `data` - Raw bytes received from the conductor
///
/// # Returns
/// A `DecodedResponse` with the request ID and either the result bytes or an error message.
pub fn decode_response(data: &[u8]) -> Result<DecodedResponse, StorageError> {
    use rmpv::decode::read_value;

    let mut cursor = Cursor::new(data);
    let value = read_value(&mut cursor)
        .map_err(|e| StorageError::Internal(format!("Failed to decode response: {}", e)))?;

    let map = value
        .as_map()
        .ok_or_else(|| StorageError::Internal("Response is not a map".into()))?;

    // Extract ID (required)
    let id = map
        .iter()
        .find(|(k, _)| k.as_str() == Some("id"))
        .and_then(|(_, v)| v.as_u64())
        .ok_or_else(|| StorageError::Internal("Response missing 'id' field".into()))?;

    // Extract type (required)
    let resp_type = map
        .iter()
        .find(|(k, _)| k.as_str() == Some("type"))
        .and_then(|(_, v)| v.as_str())
        .ok_or_else(|| StorageError::Internal("Response missing 'type' field".into()))?;

    // Extract data (required)
    let data_value = map
        .iter()
        .find(|(k, _)| k.as_str() == Some("data"))
        .map(|(_, v)| v.clone())
        .ok_or_else(|| StorageError::Internal("Response missing 'data' field".into()))?;

    let result = match resp_type {
        "response" => {
            // Success - data contains the zome call result
            let bytes = data_value.as_slice().map(|s| s.to_vec()).unwrap_or_default();
            Ok(bytes)
        }
        "error" => {
            // Error - data contains the error message
            let msg = data_value.as_str().unwrap_or("Unknown error").to_string();
            Err(msg)
        }
        other => {
            return Err(StorageError::Internal(format!(
                "Unknown response type: {}",
                other
            )))
        }
    };

    Ok(DecodedResponse { id, result })
}

/// Encode an admin API request (for auth token).
///
/// Admin API uses the same envelope format but different inner request types.
pub fn encode_admin_request(id: u64, request_type: &str, value: Value) -> Result<Vec<u8>, StorageError> {
    use rmpv::encode::write_value;

    let inner = Value::Map(vec![
        (Value::String("type".into()), Value::String(request_type.into())),
        (Value::String("value".into()), value),
    ]);

    let mut inner_bytes = Vec::new();
    write_value(&mut inner_bytes, &inner)
        .map_err(|e| StorageError::Internal(format!("Failed to encode inner request: {}", e)))?;

    let envelope = Value::Map(vec![
        (Value::String("id".into()), Value::Integer(id.into())),
        (Value::String("type".into()), Value::String("request".into())),
        (Value::String("data".into()), Value::Binary(inner_bytes)),
    ]);

    let mut bytes = Vec::new();
    write_value(&mut bytes, &envelope)
        .map_err(|e| StorageError::Internal(format!("Failed to encode envelope: {}", e)))?;

    Ok(bytes)
}

/// Encode an authentication message for the app interface.
pub fn encode_authenticate(token: &[u8]) -> Result<Vec<u8>, StorageError> {
    use rmpv::encode::write_value;

    let inner = Value::Map(vec![(
        Value::String("token".into()),
        Value::Binary(token.to_vec()),
    )]);

    let mut inner_bytes = Vec::new();
    write_value(&mut inner_bytes, &inner)
        .map_err(|e| StorageError::Internal(format!("Failed to encode auth inner: {}", e)))?;

    let envelope = Value::Map(vec![
        (Value::String("type".into()), Value::String("authenticate".into())),
        (Value::String("data".into()), Value::Binary(inner_bytes)),
    ]);

    let mut bytes = Vec::new();
    write_value(&mut bytes, &envelope)
        .map_err(|e| StorageError::Internal(format!("Failed to encode auth envelope: {}", e)))?;

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_zome_call_has_required_fields() {
        let result = encode_zome_call(1, b"cell123", "my_zome", "my_fn", b"payload");
        assert!(result.is_ok());
        let encoded = result.unwrap();
        assert_eq!(encoded.id, 1);
        assert!(!encoded.bytes.is_empty());
    }
}
