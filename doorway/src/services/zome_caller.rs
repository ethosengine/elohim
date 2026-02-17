//! ZomeCaller - Generic zome call mechanism for federation and service registration
//!
//! Provides a single-connection client (like ImportClient) that can call any zome
//! function on a Holochain conductor. Used by:
//! - Federation service (register_doorway, record_heartbeat, find_publishers)
//! - Storage registration (register_content_server)
//!
//! ## Auth Flow
//! 1. Issue AppAuthenticationToken from admin interface
//! 2. Connect to app interface with token
//! 3. Discover cell_id for role_name via list_apps
//! 4. Build CallZome envelope (MessagePack), send, parse response

use rmpv::Value;
use serde::{de::DeserializeOwned, Serialize};
use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

use crate::projection::app_auth::{self};
use crate::worker::ConductorConnection;

/// Generic zome call client with single-connection lazy init
pub struct ZomeCaller {
    admin_url: String,
    app_url: String,
    installed_app_id: String,
    /// The single conductor connection (lazily initialized)
    connection: RwLock<Option<Arc<ConductorConnection>>>,
    /// Lock to prevent concurrent connection attempts
    connecting: Mutex<()>,
}

impl ZomeCaller {
    /// Create a new ZomeCaller
    pub fn new(admin_url: &str, app_url: &str, installed_app_id: &str) -> Self {
        info!(
            admin_url = %admin_url,
            app_url = %app_url,
            installed_app_id = %installed_app_id,
            "ZomeCaller created"
        );
        Self {
            admin_url: admin_url.to_string(),
            app_url: app_url.to_string(),
            installed_app_id: installed_app_id.to_string(),
            connection: RwLock::new(None),
            connecting: Mutex::new(()),
        }
    }

    /// Get or create the conductor connection (with app auth)
    async fn get_connection(&self) -> Result<Arc<ConductorConnection>, String> {
        // Fast path: check if we have a connection
        {
            let conn = self.connection.read().await;
            if let Some(ref c) = *conn {
                if c.is_connected().await {
                    return Ok(Arc::clone(c));
                }
            }
        }

        // Slow path: need to (re)connect
        let _lock = self.connecting.lock().await;

        // Double-check after acquiring lock
        {
            let conn = self.connection.read().await;
            if let Some(ref c) = *conn {
                if c.is_connected().await {
                    return Ok(Arc::clone(c));
                }
            }
        }

        // Get auth token from admin interface
        info!("ZomeCaller authenticating via admin interface");
        let token = app_auth::issue_app_token(
            &self.admin_url,
            &self.installed_app_id,
            300, // 5 minute token
        )
        .await?;

        // Connect to app interface with post-connection authentication
        info!("ZomeCaller connecting to app interface at {}", self.app_url);
        let conn = ConductorConnection::connect_with_auth(&self.app_url, Some(token.token.clone()))
            .await
            .map_err(|e| format!("ZomeCaller connection failed: {e}"))?;

        let conn = Arc::new(conn);

        // Store the connection
        {
            let mut write_conn = self.connection.write().await;
            *write_conn = Some(Arc::clone(&conn));
        }

        info!("ZomeCaller connected to conductor");
        Ok(conn)
    }

    /// Call a zome function with raw bytes payload, return raw bytes
    pub async fn call_zome(
        &self,
        role_name: &str,
        zome_name: &str,
        fn_name: &str,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, String> {
        let conn = self.get_connection().await?;

        // Build CallZome request envelope
        let call_zome_request = build_call_zome_request(role_name, zome_name, fn_name, payload);

        let request_id = 1u64;
        let envelope = build_request_envelope(request_id, &call_zome_request);

        debug!(
            role_name = %role_name,
            zome_name = %zome_name,
            fn_name = %fn_name,
            "ZomeCaller sending zome call ({} bytes)",
            envelope.len()
        );

        match conn.request(envelope, 30_000).await {
            Ok(response) => {
                debug!("ZomeCaller got response ({} bytes)", response.len());
                parse_zome_response(&response)
            }
            Err(e) => {
                warn!("ZomeCaller request failed: {}", e);
                // Clear connection so next call reconnects
                let mut write_conn = self.connection.write().await;
                *write_conn = None;
                Err(format!("Zome call failed: {e}"))
            }
        }
    }

    /// Typed wrapper: serialize input with MessagePack, deserialize output
    pub async fn call<I: Serialize, O: DeserializeOwned>(
        &self,
        role_name: &str,
        zome_name: &str,
        fn_name: &str,
        input: &I,
    ) -> Result<O, String> {
        let payload =
            rmp_serde::to_vec(input).map_err(|e| format!("Failed to serialize input: {e}"))?;

        let response_bytes = self
            .call_zome(role_name, zome_name, fn_name, payload)
            .await?;

        rmp_serde::from_slice(&response_bytes)
            .map_err(|e| format!("Failed to deserialize response: {e}"))
    }

    /// Check if currently connected
    pub async fn is_connected(&self) -> bool {
        let conn = self.connection.read().await;
        if let Some(ref c) = *conn {
            c.is_connected().await
        } else {
            false
        }
    }
}

/// Build a CallZome inner request (MessagePack)
fn build_call_zome_request(
    role_name: &str,
    zome_name: &str,
    fn_name: &str,
    payload: Vec<u8>,
) -> Vec<u8> {
    let data = Value::Map(vec![
        (
            Value::String("role_name".into()),
            Value::String(role_name.into()),
        ),
        (
            Value::String("zome_name".into()),
            Value::String(zome_name.into()),
        ),
        (
            Value::String("fn_name".into()),
            Value::String(fn_name.into()),
        ),
        (Value::String("payload".into()), Value::Binary(payload)),
    ]);

    let inner = Value::Map(vec![
        (
            Value::String("type".into()),
            Value::String("call_zome".into()),
        ),
        (Value::String("value".into()), data),
    ]);

    let mut buf = Vec::new();
    rmpv::encode::write_value(&mut buf, &inner).expect("Failed to encode call_zome request");
    buf
}

/// Build the request envelope (same pattern as app_auth.rs)
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

/// Parse zome call response, extracting the inner result bytes
fn parse_zome_response(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut cursor = Cursor::new(data);
    let value = rmpv::decode::read_value(&mut cursor)
        .map_err(|e| format!("Failed to decode response: {e}"))?;

    if let Value::Map(ref map) = value {
        // Check for error response
        // Error envelopes use "value" field: { type: "error", value: { type: "...", value: "..." } }
        if let Some(response_type) = get_string_field(map, "type") {
            if response_type == "error" {
                if let Some(Value::Map(ref err_data)) = get_field(map, "value") {
                    if let Some(msg) = get_string_field(err_data, "value") {
                        return Err(format!("Zome call error: {msg}"));
                    }
                    if let Some(msg) = get_string_field(err_data, "message") {
                        return Err(format!("Zome call error: {msg}"));
                    }
                }
                return Err("Unknown zome call error".to_string());
            }
        }

        // Parse success response
        // Response format: { id, type: "response", data: <bytes> }
        if let Some(Value::Binary(inner_bytes)) = get_field(map, "data") {
            // Inner response is the zome call result (also MessagePack)
            let mut inner_cursor = Cursor::new(inner_bytes.as_slice());
            let inner = rmpv::decode::read_value(&mut inner_cursor)
                .map_err(|e| format!("Failed to decode inner response: {e}"))?;

            // The zome call result is wrapped in { type: "...", value: <result bytes> }
            if let Value::Map(ref inner_map) = inner {
                if let Some(Value::Binary(result_bytes)) = get_field(inner_map, "value") {
                    return Ok(result_bytes.clone());
                }
                // Some responses may have the value directly as a map
                if let Some(Value::Map(ref result_map)) = get_field(inner_map, "value") {
                    let mut buf = Vec::new();
                    rmpv::encode::write_value(&mut buf, &Value::Map(result_map.clone()))
                        .map_err(|e| format!("Failed to re-encode result: {e}"))?;
                    return Ok(buf);
                }
            }

            // If inner is directly the result bytes, return them
            return Ok(inner_bytes.clone());
        }
    }

    Err("Unexpected zome call response format".to_string())
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
    fn test_build_call_zome_request() {
        let payload = rmp_serde::to_vec(&"test").unwrap();
        let request = build_call_zome_request(
            "infrastructure",
            "infrastructure",
            "register_doorway",
            payload,
        );
        assert!(!request.is_empty());

        // Verify it can be decoded
        let mut cursor = Cursor::new(&request);
        let decoded = rmpv::decode::read_value(&mut cursor).unwrap();
        assert!(matches!(decoded, Value::Map(_)));
    }

    #[test]
    fn test_build_request_envelope() {
        let inner = build_call_zome_request(
            "infrastructure",
            "infrastructure",
            "test_fn",
            vec![0xc0], // msgpack nil
        );
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
