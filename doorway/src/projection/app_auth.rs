//! App Interface Authentication
//!
//! Handles the Holochain 0.3+ app interface authentication flow:
//! 1. Request an AppAuthenticationToken from admin interface
//! 2. Connect to app interface
//! 3. Send AppAuthenticationRequest with the token

use futures_util::{SinkExt, StreamExt};
use rmpv::Value;
use std::io::Cursor;
use std::time::Duration;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{http::Request, Message},
};
use tracing::{debug, error, info};

/// Token issued by admin interface for app authentication
#[derive(Debug, Clone)]
pub struct AppAuthToken {
    /// The raw token bytes
    pub token: Vec<u8>,
}

/// Issue an app authentication token from the admin interface
///
/// Connects to the admin interface, sends IssueAppAuthenticationToken,
/// and returns the token to use for app interface authentication.
pub async fn issue_app_token(
    admin_url: &str,
    installed_app_id: &str,
    expiry_seconds: u64,
) -> Result<AppAuthToken, String> {
    info!(
        "Requesting app auth token for '{}' from {}",
        installed_app_id, admin_url
    );

    // Build WebSocket request with proper headers
    let host = admin_url.split("//").last().unwrap_or("localhost");
    let request = Request::builder()
        .uri(admin_url)
        .header("Host", host)
        .header("Origin", "http://localhost")
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header(
            "Sec-WebSocket-Key",
            tokio_tungstenite::tungstenite::handshake::client::generate_key(),
        )
        .body(())
        .map_err(|e| format!("Failed to build request: {}", e))?;

    // Connect to admin interface
    let (ws_stream, _) = connect_async_with_config(request, None, false)
        .await
        .map_err(|e| format!("Admin WebSocket connect failed: {}", e))?;

    let (mut write, mut read) = ws_stream.split();

    // Build IssueAppAuthenticationToken request
    // Format: { type: "issue_app_authentication_token", data: { installed_app_id, expiry_seconds, single_use } }
    let request_id = 1u64;
    let inner_request = build_issue_token_request(installed_app_id, expiry_seconds);
    let envelope = build_request_envelope(request_id, &inner_request);

    debug!("Sending IssueAppAuthenticationToken request");

    // Send request
    write
        .send(Message::Binary(envelope))
        .await
        .map_err(|e| format!("Failed to send token request: {}", e))?;

    // Wait for response with timeout
    let response = tokio::time::timeout(Duration::from_secs(10), async {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    return parse_token_response(&data);
                }
                Ok(Message::Close(_)) => {
                    return Err("Admin connection closed".to_string());
                }
                Err(e) => {
                    return Err(format!("WebSocket error: {}", e));
                }
                _ => continue,
            }
        }
        Err("No response received".to_string())
    })
    .await
    .map_err(|_| "Timeout waiting for token response".to_string())??;

    // Close the admin connection
    let _ = write.close().await;

    info!("Successfully obtained app auth token");
    Ok(response)
}

// Note: App authentication after connecting is done inline in the subscriber
// since the generic stream types make a shared helper complex

/// Build the IssueAppAuthenticationToken inner request
fn build_issue_token_request(installed_app_id: &str, expiry_seconds: u64) -> Vec<u8> {
    // Inner request: { type: "issue_app_authentication_token", data: { installed_app_id, expiry_seconds, single_use } }
    let data = Value::Map(vec![
        (
            Value::String("installed_app_id".into()),
            Value::String(installed_app_id.into()),
        ),
        (
            Value::String("expiry_seconds".into()),
            Value::Integer(expiry_seconds.into()),
        ),
        (
            Value::String("single_use".into()),
            Value::Boolean(false), // Allow reuse for reconnection
        ),
    ]);

    let inner = Value::Map(vec![
        (
            Value::String("type".into()),
            Value::String("issue_app_authentication_token".into()),
        ),
        (Value::String("value".into()), data),
    ]);

    let mut buf = Vec::new();
    rmpv::encode::write_value(&mut buf, &inner).expect("Failed to encode inner request");
    buf
}

/// Build the request envelope
fn build_request_envelope(id: u64, inner_data: &[u8]) -> Vec<u8> {
    // Envelope: { id, type: "request", data: <inner bytes> }
    let envelope = Value::Map(vec![
        (Value::String("id".into()), Value::Integer(id.into())),
        (
            Value::String("type".into()),
            Value::String("request".into()),
        ),
        (
            Value::String("data".into()),
            Value::Binary(inner_data.to_vec()),
        ),
    ]);

    let mut buf = Vec::new();
    rmpv::encode::write_value(&mut buf, &envelope).expect("Failed to encode envelope");
    buf
}

/// Build AppAuthenticationRequest message
#[allow(dead_code)]
fn build_app_auth_request(token: &[u8]) -> Vec<u8> {
    // AppAuthenticationRequest: { token: <bytes> }
    // Wrapped in envelope for consistency
    let inner = Value::Map(vec![(
        Value::String("token".into()),
        Value::Binary(token.to_vec()),
    )]);

    let mut inner_buf = Vec::new();
    rmpv::encode::write_value(&mut inner_buf, &inner).expect("Failed to encode auth request");

    build_request_envelope(0, &inner_buf)
}

/// Parse the token response from admin interface
fn parse_token_response(data: &[u8]) -> Result<AppAuthToken, String> {
    let mut cursor = Cursor::new(data);
    let value = rmpv::decode::read_value(&mut cursor)
        .map_err(|e| format!("Failed to decode response: {}", e))?;

    // Response format: { id, type: "response", data: <bytes> }
    if let Value::Map(ref map) = value {
        // Check for error response
        // Error envelopes use "value" field: { type: "error", value: { type: "...", value: "..." } }
        if let Some(response_type) = get_string_field(map, "type") {
            if response_type == "error" {
                if let Some(Value::Map(ref err_data)) = get_field(map, "value") {
                    if let Some(msg) = get_string_field(err_data, "value") {
                        return Err(format!("Admin error: {}", msg));
                    }
                    if let Some(msg) = get_string_field(err_data, "message") {
                        return Err(format!("Admin error: {}", msg));
                    }
                }
                return Err("Unknown admin error".to_string());
            }
        }

        // Parse success response
        if let Some(Value::Binary(inner_bytes)) = get_field(map, "data") {
            let mut inner_cursor = Cursor::new(inner_bytes.as_slice());
            let inner = rmpv::decode::read_value(&mut inner_cursor)
                .map_err(|e| format!("Failed to decode inner response: {}", e))?;

            // Inner: { type: "app_authentication_token_issued", value: { token: <bytes> } }
            if let Value::Map(ref inner_map) = inner {
                if let Some(Value::Map(ref token_data)) = get_field(inner_map, "value") {
                    if let Some(Value::Binary(token_bytes)) = get_field(token_data, "token") {
                        return Ok(AppAuthToken {
                            token: token_bytes.clone(),
                        });
                    }
                }
            }
        }
    }

    error!("Unexpected token response format: {:?}", value);
    Err("Unexpected token response format".to_string())
}

/// Parse the authentication response
#[allow(dead_code)]
fn parse_auth_response(data: &[u8]) -> Result<(), String> {
    let mut cursor = Cursor::new(data);
    let value = rmpv::decode::read_value(&mut cursor)
        .map_err(|e| format!("Failed to decode auth response: {}", e))?;

    // Check for error
    if let Value::Map(ref map) = value {
        if let Some(response_type) = get_string_field(map, "type") {
            if response_type == "error" {
                if let Some(Value::Map(ref err_data)) = get_field(map, "data") {
                    if let Some(msg) = get_string_field(err_data, "message") {
                        return Err(format!("Auth error: {}", msg));
                    }
                }
                return Err("Authentication failed".to_string());
            }

            // Any non-error response means success
            // (Holochain sends app_authenticated or similar)
            return Ok(());
        }
    }

    // Empty or unexpected response might still be OK
    debug!("Auth response: {:?}", value);
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_issue_token_request() {
        let request = build_issue_token_request("elohim", 300);
        assert!(!request.is_empty());

        // Verify it can be decoded
        let mut cursor = Cursor::new(&request);
        let decoded = rmpv::decode::read_value(&mut cursor).unwrap();
        assert!(matches!(decoded, Value::Map(_)));
    }

    #[test]
    fn test_build_request_envelope() {
        let inner = build_issue_token_request("test-app", 60);
        let envelope = build_request_envelope(42, &inner);

        let mut cursor = Cursor::new(&envelope);
        let decoded = rmpv::decode::read_value(&mut cursor).unwrap();

        if let Value::Map(map) = decoded {
            let id = get_field(&map, "id");
            assert!(matches!(id, Some(Value::Integer(_))));

            let msg_type = get_string_field(&map, "type");
            assert_eq!(msg_type.as_deref(), Some("request"));
        } else {
            panic!("Expected map");
        }
    }
}
