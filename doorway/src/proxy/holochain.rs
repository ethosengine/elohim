//! Holochain MessagePack message parsing
//!
//! Parses the double-encoded message format used by @holochain/client:
//! - Outer envelope: { id, type: "request", data: <encoded inner> }
//! - Inner request: { type: "list_apps", data: {...} }
//!
//! Ported from admin-proxy/src/message-parser.ts

use rmpv::Value;
use std::io::Cursor;

use crate::types::DoorwayError;

/// Parsed Holochain admin message
#[derive(Debug, Clone)]
pub struct ParsedMessage {
    /// The operation type (e.g., "list_apps", "install_app")
    pub operation: String,
    /// The operation data (may be Nil for operations without data)
    pub data: Value,
}

/// Parse a MessagePack-encoded Holochain admin message.
///
/// The @holochain/client double-encodes messages:
/// - Outer: { id, type: "request", data: <encoded inner> }
/// - Inner: { type: "list_apps", data: {...} }
pub fn parse_message(data: &[u8]) -> Result<ParsedMessage, DoorwayError> {
    let mut cursor = Cursor::new(data);

    // Decode outer envelope
    let envelope = rmpv::decode::read_value(&mut cursor)
        .map_err(|e| DoorwayError::Holochain(format!("Failed to decode envelope: {}", e)))?;

    // Try client envelope format first
    if let Value::Map(ref map) = envelope {
        if let Some(parsed) = try_parse_client_envelope(map)? {
            return Ok(parsed);
        }

        // Fallback: try direct tagged message format
        if let Some(parsed) = try_parse_direct_message(map)? {
            return Ok(parsed);
        }
    }

    Err(DoorwayError::Holochain(
        "Invalid message format: expected map with 'type' field".into(),
    ))
}

/// Try to parse as client envelope format: { id, type: "request", data: <bytes> }
fn try_parse_client_envelope(
    map: &[(Value, Value)],
) -> Result<Option<ParsedMessage>, DoorwayError> {
    let msg_type = get_string_field(map, "type");
    let data_field = get_field(map, "data");

    // Check for envelope format
    if msg_type.as_deref() == Some("request") {
        if let Some(Value::Binary(inner_bytes)) = data_field {
            // Decode the inner AdminRequest
            let mut inner_cursor = Cursor::new(inner_bytes.as_slice());
            let inner = rmpv::decode::read_value(&mut inner_cursor).map_err(|e| {
                DoorwayError::Holochain(format!("Failed to decode inner request: {}", e))
            })?;

            if let Value::Map(ref inner_map) = inner {
                if let Some(operation) = get_string_field(inner_map, "type") {
                    let data = get_field(inner_map, "data").cloned().unwrap_or(Value::Nil);
                    return Ok(Some(ParsedMessage { operation, data }));
                }
            }
        }
    }

    Ok(None)
}

/// Try to parse as direct tagged message: { type: "list_apps", data: {...} }
fn try_parse_direct_message(map: &[(Value, Value)]) -> Result<Option<ParsedMessage>, DoorwayError> {
    if let Some(operation) = get_string_field(map, "type") {
        // Skip if this looks like an envelope (type: "request")
        if operation == "request" || operation == "response" {
            return Ok(None);
        }

        let data = get_field(map, "data").cloned().unwrap_or(Value::Nil);
        return Ok(Some(ParsedMessage { operation, data }));
    }

    Ok(None)
}

/// Get a string field from a MessagePack map
fn get_string_field(map: &[(Value, Value)], key: &str) -> Option<String> {
    for (k, v) in map {
        if let Value::String(k_str) = k {
            if k_str.as_str() == Some(key) {
                if let Value::String(v_str) = v {
                    return v_str.as_str().map(|s| s.to_string());
                }
            }
        }
    }
    None
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

/// Encode an error response in MessagePack format
pub fn encode_error(message: &str) -> Vec<u8> {
    let error_map = Value::Map(vec![
        (Value::String("type".into()), Value::String("error".into())),
        (
            Value::String("data".into()),
            Value::Map(vec![(
                Value::String("message".into()),
                Value::String(message.into()),
            )]),
        ),
    ]);

    let mut buf = Vec::new();
    rmpv::encode::write_value(&mut buf, &error_map).unwrap_or_default();
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_direct_message() {
        // Create a simple { type: "list_apps", data: null } message
        let msg = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("list_apps".into()),
            ),
            (Value::String("data".into()), Value::Nil),
        ]);

        let mut buf = Vec::new();
        rmpv::encode::write_value(&mut buf, &msg).unwrap();

        let parsed = parse_message(&buf).unwrap();
        assert_eq!(parsed.operation, "list_apps");
    }

    #[test]
    fn test_encode_error() {
        let encoded = encode_error("Test error");
        assert!(!encoded.is_empty());

        // Verify it can be decoded
        let mut cursor = Cursor::new(&encoded);
        let decoded = rmpv::decode::read_value(&mut cursor).unwrap();

        if let Value::Map(map) = decoded {
            let msg_type = get_string_field(&map, "type");
            assert_eq!(msg_type.as_deref(), Some("error"));
        } else {
            panic!("Expected map");
        }
    }
}
