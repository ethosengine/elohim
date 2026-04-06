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

use holochain_conductor_api::AppResponse;
use holochain_serialized_bytes::prelude::*;
use holochain_websocket::WireMessage;
use rmpv::Value;
use serde::{de::DeserializeOwned, Serialize};
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
        let payload = rmp_serde::to_vec_named(input)
            .map_err(|e| format!("Failed to serialize input: {e}"))?;

        let response_bytes = self
            .call_zome(role_name, zome_name, fn_name, payload)
            .await?;

        rmp_serde::from_slice(&response_bytes).map_err(|e| {
            // Dump response structure on failure for debugging
            let structure = match rmpv::decode::read_value(&mut std::io::Cursor::new(&response_bytes)) {
                Ok(val) => format!("{val:?}"),
                Err(decode_err) => format!("<raw {} bytes, decode err: {decode_err}>", response_bytes.len()),
            };
            format!(
                "Failed to deserialize response: {e} | response_bytes({} bytes) structure: {structure}",
                response_bytes.len()
            )
        })
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

/// Parse zome call response using official Holochain types.
///
/// The conductor WebSocket response has three layers:
/// 1. `WireMessage::Response { id, data }` — the outer envelope
/// 2. `AppResponse::ZomeCalled(Box<ExternIO>)` — the app-level response
/// 3. `ExternIO(Vec<u8>)` — the raw msgpack bytes of the zome return value
fn parse_zome_response(data: &[u8]) -> Result<Vec<u8>, String> {
    // Layer 1: Deserialize WireMessage envelope
    let sb = SerializedBytes::from(UnsafeBytes::from(data.to_vec()));
    let wire_msg: WireMessage = sb
        .try_into()
        .map_err(|e: SerializedBytesError| format!("Failed to decode WireMessage: {e}"))?;

    match wire_msg {
        WireMessage::Response { id: _, data } => {
            let response_bytes =
                data.ok_or_else(|| "Conductor returned empty response (None)".to_string())?;

            // Layer 2: Deserialize AppResponse
            let sb = SerializedBytes::from(UnsafeBytes::from(response_bytes));
            let app_response: AppResponse = sb
                .try_into()
                .map_err(|e: SerializedBytesError| format!("Failed to decode AppResponse: {e}"))?;

            match app_response {
                AppResponse::ZomeCalled(extern_io) => {
                    // Layer 3: ExternIO contains the raw bytes of the zome return value
                    debug!(
                        bytes_len = extern_io.as_bytes().len(),
                        "parse_zome_response: extracted ExternIO from ZomeCalled"
                    );
                    Ok(extern_io.into_vec())
                }
                AppResponse::Error(e) => Err(format!("Conductor AppResponse error: {e:?}")),
                other => Err(format!("Unexpected AppResponse variant: {other:?}")),
            }
        }
        WireMessage::Signal { .. } => Err("Received Signal instead of Response".to_string()),
        other => Err(format!("Unexpected WireMessage type: {other:?}")),
    }
}

#[cfg(test)]
mod tests {
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

    use super::*;
    use std::io::Cursor;

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

    #[test]
    fn test_parse_zome_response_roundtrip() {
        use holochain_zome_types::prelude::ExternIO;

        // Build mock zome return data (what the zome function would return)
        let action_hash_bytes: Vec<u8> = vec![0u8; 39];
        let test_data = rmp_serde::to_vec_named(&serde_json::json!({
            "action_hash": action_hash_bytes,
            "human": {
                "id": "test-123",
                "display_name": "Test",
                "bio": null,
                "affinities": [],
                "profile_reach": "public",
                "location": null,
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            }
        }))
        .unwrap();

        // Layer 3: Wrap in ExternIO
        let extern_io = ExternIO::from(test_data.clone());

        // Layer 2: Wrap in AppResponse::ZomeCalled
        let app_response = AppResponse::ZomeCalled(Box::new(extern_io));
        let app_sb: SerializedBytes = app_response.try_into().unwrap();

        // Layer 1: Wrap in WireMessage::Response
        let wire_msg = WireMessage::Response {
            id: 1,
            data: Some(UnsafeBytes::from(app_sb).into()),
        };
        let wire_sb: SerializedBytes = wire_msg.try_into().unwrap();

        // Parse using our function
        let result = parse_zome_response(wire_sb.bytes()).unwrap();
        assert_eq!(result, test_data);
    }

    #[test]
    fn test_parse_zome_response_error() {
        use holochain_conductor_api::ExternalApiWireError;

        // Build an error response
        let app_response =
            AppResponse::Error(ExternalApiWireError::InternalError("test error".into()));
        let app_sb: SerializedBytes = app_response.try_into().unwrap();

        let wire_msg = WireMessage::Response {
            id: 1,
            data: Some(UnsafeBytes::from(app_sb).into()),
        };
        let wire_sb: SerializedBytes = wire_msg.try_into().unwrap();

        let result = parse_zome_response(wire_sb.bytes());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Conductor AppResponse error"));
    }
}
