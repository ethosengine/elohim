//! Holochain Admin API client for agent provisioning
//!
//! Reusable admin API client using short-lived WebSocket connections + raw MessagePack.
//! Follows the envelope pattern from `projection/app_auth.rs:107-156`.
//!
//! Each method opens a fresh WebSocket, sends one request, reads one response,
//! and closes. Provisioning is infrequent — no need for persistent admin pools.

use futures_util::{SinkExt, StreamExt};
use rmpv::Value;
use std::io::Cursor;
use std::time::Duration;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{http::Request, Message},
};

/// Default timeout for admin WebSocket operations
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);

/// Holochain admin API client using short-lived WebSocket connections.
///
/// Each call opens a fresh connection, sends one request, reads one response,
/// and closes. This matches the pattern from `projection/app_auth.rs`.
pub struct AdminClient {
    admin_url: String,
    timeout: Duration,
}

impl AdminClient {
    /// Create a new admin client targeting the given admin WebSocket URL.
    pub fn new(admin_url: String) -> Self {
        Self {
            admin_url,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Set a custom timeout for WebSocket operations.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Generate a new agent public key on the conductor.
    ///
    /// Returns the raw 39-byte agent key from the conductor.
    pub async fn generate_agent_pub_key(&self) -> Result<Vec<u8>, String> {
        // Build inner request: { type: "generate_agent_pub_key", value: null }
        let inner = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("generate_agent_pub_key".into()),
            ),
            (Value::String("value".into()), Value::Nil),
        ]);

        let response = self.send_request(&inner).await?;

        // Response inner: { type: "agent_pub_key_generated", value: <39 bytes> }
        if let Value::Map(ref map) = response {
            // Check for error
            if let Some(err_type) = get_string_field(map, "type") {
                if err_type == "error" {
                    if let Some(Value::Map(ref err_data)) = get_field(map, "value") {
                        if let Some(msg) = get_string_field(err_data, "message") {
                            return Err(format!("Admin error: {msg}"));
                        }
                    }
                    return Err("Unknown admin error during key generation".to_string());
                }
            }

            // Extract agent key from value field
            if let Some(Value::Binary(key_bytes)) = get_field(map, "value") {
                return Ok(key_bytes.clone());
            }
        }

        Err(format!(
            "Unexpected generate_agent_pub_key response: {response:?}"
        ))
    }

    /// Install an app on the conductor with the given agent key.
    pub async fn install_app(
        &self,
        installed_app_id: &str,
        agent_key: &[u8],
        bundle_path: &str,
    ) -> Result<(), String> {
        // Build inner request:
        // { type: "install_app", data: { installed_app_id, agent_key, path: bundle_path } }
        let data = Value::Map(vec![
            (
                Value::String("installed_app_id".into()),
                Value::String(installed_app_id.into()),
            ),
            (
                Value::String("agent_key".into()),
                Value::Binary(agent_key.to_vec()),
            ),
            (
                Value::String("path".into()),
                Value::String(bundle_path.into()),
            ),
        ]);

        let inner = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("install_app".into()),
            ),
            (Value::String("value".into()), data),
        ]);

        let response = self.send_request(&inner).await?;
        self.check_error_response(&response, "install_app")?;

        Ok(())
    }

    /// Enable an installed app on the conductor.
    pub async fn enable_app(&self, installed_app_id: &str) -> Result<(), String> {
        // Build inner request: { type: "enable_app", value: { installed_app_id } }
        let data = Value::Map(vec![(
            Value::String("installed_app_id".into()),
            Value::String(installed_app_id.into()),
        )]);

        let inner = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("enable_app".into()),
            ),
            (Value::String("value".into()), data),
        ]);

        let response = self.send_request(&inner).await?;
        self.check_error_response(&response, "enable_app")?;

        Ok(())
    }

    /// Uninstall an app from the conductor (cleanup).
    pub async fn uninstall_app(&self, installed_app_id: &str) -> Result<(), String> {
        // Build inner request: { type: "uninstall_app", value: { installed_app_id } }
        let data = Value::Map(vec![(
            Value::String("installed_app_id".into()),
            Value::String(installed_app_id.into()),
        )]);

        let inner = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("uninstall_app".into()),
            ),
            (Value::String("value".into()), data),
        ]);

        let response = self.send_request(&inner).await?;
        self.check_error_response(&response, "uninstall_app")?;

        Ok(())
    }

    // =========================================================================
    // Private helpers
    // =========================================================================

    /// Open a WebSocket connection, send a request, read the response, close.
    ///
    /// Follows the pattern from `discovery.rs:252-282` (connect) and
    /// `app_auth.rs:60-95` (send/receive cycle).
    async fn send_request(&self, inner: &Value) -> Result<Value, String> {
        // Encode inner request
        let inner_bytes = encode_msgpack(inner)?;

        // Build envelope: { id: 1, type: "request", data: <inner bytes> }
        let envelope = build_request_envelope(1, &inner_bytes);

        // Connect (pattern from discovery.rs:252-282)
        let host = self
            .admin_url
            .split("//")
            .last()
            .unwrap_or("localhost:4444");

        let request = Request::builder()
            .uri(&self.admin_url)
            .header("Host", host)
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header(
                "Sec-WebSocket-Key",
                tokio_tungstenite::tungstenite::handshake::client::generate_key(),
            )
            .header("Origin", "http://localhost:8080")
            .body(())
            .map_err(|e| format!("Failed to build request: {e}"))?;

        let (ws_stream, _) = tokio::time::timeout(
            self.timeout,
            connect_async_with_config(request, None, false),
        )
        .await
        .map_err(|_| "Timeout connecting to admin interface".to_string())?
        .map_err(|e| format!("Admin WebSocket connect failed: {e}"))?;

        let (mut write, mut read) = ws_stream.split();

        // Send request
        write
            .send(Message::Binary(envelope))
            .await
            .map_err(|e| format!("Failed to send admin request: {e}"))?;

        // Read response with timeout
        let response_bytes = tokio::time::timeout(self.timeout, async {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Binary(data)) => return Ok(data),
                    Ok(Message::Close(_)) => {
                        return Err("Admin connection closed".to_string());
                    }
                    Err(e) => return Err(format!("WebSocket error: {e}")),
                    _ => continue,
                }
            }
            Err("No response received".to_string())
        })
        .await
        .map_err(|_| "Timeout waiting for admin response".to_string())??;

        // Close connection
        let _ = write.close().await;

        // Parse response envelope
        parse_response_envelope(&response_bytes)
    }

    /// Check if the parsed inner response is an error and return Err if so.
    fn check_error_response(&self, response: &Value, operation: &str) -> Result<(), String> {
        if let Value::Map(ref map) = response {
            if let Some(resp_type) = get_string_field(map, "type") {
                if resp_type == "error" {
                    if let Some(Value::Map(ref err_data)) = get_field(map, "value") {
                        if let Some(msg) = get_string_field(err_data, "message") {
                            return Err(format!("Admin error ({operation}): {msg}"));
                        }
                    }
                    return Err(format!("Unknown admin error during {operation}"));
                }
            }
        }
        Ok(())
    }
}

// =============================================================================
// MessagePack helpers (pattern from app_auth.rs)
// =============================================================================

/// Encode a rmpv::Value to MessagePack bytes.
fn encode_msgpack(value: &Value) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    rmpv::encode::write_value(&mut buf, value)
        .map_err(|e| format!("Failed to encode MessagePack: {e}"))?;
    Ok(buf)
}

/// Build the request envelope (pattern from app_auth.rs:138-156).
///
/// Format: `{ id, type: "request", data: <inner bytes> }`
fn build_request_envelope(id: u64, inner_data: &[u8]) -> Vec<u8> {
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

/// Parse the response envelope, extract the inner data.
///
/// Format: `{ id, type: "response"|"error", data: <inner bytes> }`
/// Pattern from app_auth.rs:174-215.
fn parse_response_envelope(data: &[u8]) -> Result<Value, String> {
    let mut cursor = Cursor::new(data);
    let value = rmpv::decode::read_value(&mut cursor)
        .map_err(|e| format!("Failed to decode response: {e}"))?;

    if let Value::Map(ref map) = value {
        // Check for error at envelope level
        // Error envelopes use "value" field: { type: "error", value: { type: "...", value: "..." } }
        if let Some(resp_type) = get_string_field(map, "type") {
            if resp_type == "error" {
                if let Some(Value::Map(ref err_data)) = get_field(map, "value") {
                    if let Some(msg) = get_string_field(err_data, "value") {
                        return Err(format!("Admin error: {msg}"));
                    }
                    if let Some(msg) = get_string_field(err_data, "message") {
                        return Err(format!("Admin error: {msg}"));
                    }
                }
                return Err("Unknown admin error".to_string());
            }
        }

        // Parse success response — inner data is Binary
        if let Some(Value::Binary(inner_bytes)) = get_field(map, "data") {
            let mut inner_cursor = Cursor::new(inner_bytes.as_slice());
            let inner = rmpv::decode::read_value(&mut inner_cursor)
                .map_err(|e| format!("Failed to decode inner response: {e}"))?;
            return Ok(inner);
        }
    }

    Err(format!("Unexpected response format: {value:?}"))
}

/// Get a string field from a MessagePack map.
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

/// Get a field from a MessagePack map.
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
    fn test_admin_client_creation() {
        let client = AdminClient::new("ws://localhost:4444".to_string());
        assert_eq!(client.admin_url, "ws://localhost:4444");
        assert_eq!(client.timeout, DEFAULT_TIMEOUT);

        let client = client.with_timeout(Duration::from_secs(30));
        assert_eq!(client.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_build_request_envelope() {
        let inner = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("generate_agent_pub_key".into()),
            ),
            (Value::String("value".into()), Value::Nil),
        ]);
        let inner_bytes = encode_msgpack(&inner).unwrap();
        let envelope = build_request_envelope(1, &inner_bytes);

        // Should be valid MessagePack
        let mut cursor = Cursor::new(&envelope);
        let decoded = rmpv::decode::read_value(&mut cursor).unwrap();
        assert!(matches!(decoded, Value::Map(_)));

        if let Value::Map(map) = decoded {
            assert_eq!(get_string_field(&map, "type"), Some("request".to_string()));
        }
    }

    #[test]
    fn test_parse_response_envelope_success() {
        // Build a mock success response (Holochain 0.6 uses "value" for inner payload)
        let inner = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("agent_pub_key_generated".into()),
            ),
            (Value::String("value".into()), Value::Binary(vec![0u8; 39])),
        ]);
        let inner_bytes = encode_msgpack(&inner).unwrap();

        let envelope = Value::Map(vec![
            (Value::String("id".into()), Value::Integer(1.into())),
            (
                Value::String("type".into()),
                Value::String("response".into()),
            ),
            (Value::String("data".into()), Value::Binary(inner_bytes)),
        ]);
        let envelope_bytes = encode_msgpack(&envelope).unwrap();

        let result = parse_response_envelope(&envelope_bytes).unwrap();
        if let Value::Map(map) = result {
            assert_eq!(
                get_string_field(&map, "type"),
                Some("agent_pub_key_generated".to_string())
            );
        } else {
            panic!("Expected Map response");
        }
    }

    #[test]
    fn test_parse_response_envelope_error() {
        // Holochain 0.6 error format: { type: "error", value: { type: "...", value: "..." } }
        let err_data = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("deserialization".into()),
            ),
            (
                Value::String("value".into()),
                Value::String("something went wrong".into()),
            ),
        ]);

        let envelope = Value::Map(vec![
            (Value::String("id".into()), Value::Integer(1.into())),
            (Value::String("type".into()), Value::String("error".into())),
            (Value::String("value".into()), err_data),
        ]);
        let envelope_bytes = encode_msgpack(&envelope).unwrap();

        let result = parse_response_envelope(&envelope_bytes);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("something went wrong"));
    }

    #[test]
    fn test_check_error_response() {
        let client = AdminClient::new("ws://localhost:4444".to_string());

        // Non-error response should be OK
        let ok_response = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("app_installed".into()),
            ),
            (Value::String("value".into()), Value::Nil),
        ]);
        assert!(client.check_error_response(&ok_response, "test").is_ok());

        // Error response should return Err (Holochain 0.6 error format)
        let err_response = Value::Map(vec![
            (Value::String("type".into()), Value::String("error".into())),
            (
                Value::String("value".into()),
                Value::Map(vec![(
                    Value::String("message".into()),
                    Value::String("app not found".into()),
                )]),
            ),
        ]);
        let result = client.check_error_response(&err_response, "install_app");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("app not found"));
    }
}
