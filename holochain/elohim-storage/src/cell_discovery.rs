//! Cell Discovery - Discover cell_id from Holochain conductor admin interface
//!
//! Used by ImportApi to find the correct cell for zome calls.
//!
//! ## Usage
//!
//! ```ignore
//! let cell_id = discover_cell_id(
//!     "ws://localhost:4444",  // admin URL
//!     "elohim",               // app ID
//!     Some("lamad"),          // role filter (optional)
//! ).await?;
//! ```

use crate::error::StorageError;
use futures_util::{SinkExt, StreamExt};
use rmpv::Value;
use std::io::Cursor;
use std::time::Duration;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{http::Request, protocol::Message},
};
use tracing::{debug, info};

/// Discover cell_id from conductor admin interface
///
/// Returns the serialized cell_id bytes (dna_hash + agent_pub_key concatenated)
/// suitable for passing to zome calls.
///
/// # Arguments
/// * `admin_url` - Conductor admin WebSocket URL (e.g., "ws://localhost:4444")
/// * `app_id` - Installed app ID to find
/// * `role_filter` - Optional role name to match (e.g., "lamad")
///
/// # Returns
/// * `Ok(Vec<u8>)` - Serialized cell_id bytes
/// * `Err(StorageError)` - If discovery fails
pub async fn discover_cell_id(
    admin_url: &str,
    app_id: &str,
    role_filter: Option<&str>,
) -> Result<Vec<u8>, StorageError> {
    info!(admin_url = %admin_url, app_id = %app_id, role = ?role_filter, "Discovering cell_id from conductor");

    // Connect to admin interface
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
        .map_err(|e| StorageError::Connection(format!("Failed to build request: {}", e)))?;

    let (ws_stream, _) = connect_async_with_config(request, None, false)
        .await
        .map_err(|e| StorageError::Connection(format!("Admin connection failed: {}", e)))?;

    let (mut write, mut read) = ws_stream.split();

    debug!("Connected to admin interface, sending list_apps");

    // Build list_apps request
    let inner = Value::Map(vec![
        (Value::String("type".into()), Value::String("list_apps".into())),
        (Value::String("data".into()), Value::Map(vec![])),
    ]);

    let response = send_admin_request(&mut write, &mut read, &inner, 1).await?;

    // Close connection
    let _ = write.close().await;

    // Parse cell_id from response
    parse_cell_id_from_apps(&response, app_id, role_filter)
}

/// Send admin request with proper envelope format
async fn send_admin_request<S, R>(
    write: &mut S,
    read: &mut R,
    inner_request: &Value,
    request_id: u64,
) -> Result<Value, StorageError>
where
    S: SinkExt<Message> + Unpin,
    S::Error: std::fmt::Display,
    R: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    // Encode inner request
    let mut inner_buf = Vec::new();
    rmpv::encode::write_value(&mut inner_buf, inner_request)
        .map_err(|e| StorageError::Parse(format!("Failed to encode inner request: {}", e)))?;

    // Build envelope: { id, type: "request", data: <inner bytes> }
    let envelope = Value::Map(vec![
        (Value::String("id".into()), Value::Integer(request_id.into())),
        (Value::String("type".into()), Value::String("request".into())),
        (Value::String("data".into()), Value::Binary(inner_buf)),
    ]);

    let mut envelope_buf = Vec::new();
    rmpv::encode::write_value(&mut envelope_buf, &envelope)
        .map_err(|e| StorageError::Parse(format!("Failed to encode envelope: {}", e)))?;

    // Send request
    write
        .send(Message::Binary(envelope_buf.into()))
        .await
        .map_err(|e| StorageError::Connection(format!("Failed to send: {}", e)))?;

    // Wait for response with matching ID (timeout 30s)
    let timeout = Duration::from_secs(30);
    let response = tokio::time::timeout(timeout, async {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    let mut cursor = Cursor::new(&data[..]);
                    let value = rmpv::decode::read_value(&mut cursor)
                        .map_err(|e| StorageError::Parse(format!("Failed to parse response: {}", e)))?;

                    if let Value::Map(ref map) = value {
                        // Check for error response
                        if let Some(resp_type) = get_string_field(map, "type") {
                            if resp_type == "error" {
                                let msg = get_error_message(map);
                                return Err(StorageError::Protocol(msg));
                            }
                        }

                        // Check ID matches
                        if let Some(Value::Integer(resp_id)) = get_field(map, "id") {
                            if resp_id.as_u64() == Some(request_id) {
                                // Parse inner response from data field
                                if let Some(Value::Binary(inner_bytes)) = get_field(map, "data") {
                                    let mut inner_cursor = Cursor::new(&inner_bytes[..]);
                                    let inner = rmpv::decode::read_value(&mut inner_cursor)
                                        .map_err(|e| {
                                            StorageError::Parse(format!(
                                                "Failed to parse inner response: {}",
                                                e
                                            ))
                                        })?;
                                    return Ok(inner);
                                }
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    return Err(StorageError::Connection("Connection closed".to_string()));
                }
                Err(e) => {
                    return Err(StorageError::Connection(format!("WebSocket error: {}", e)));
                }
                _ => continue,
            }
        }
        Err(StorageError::Connection("No response received".to_string()))
    })
    .await
    .map_err(|_| StorageError::Timeout("Request timed out".to_string()))??;

    Ok(response)
}

/// Parse cell_id from list_apps response
fn parse_cell_id_from_apps(
    response: &Value,
    app_id: &str,
    role_filter: Option<&str>,
) -> Result<Vec<u8>, StorageError> {
    // Response structure:
    // { type: "...", data: [{ installed_app_id, cell_info: { role_name: [{ cell_id: [dna, agent] }] } }] }
    if let Value::Map(map) = response {
        if let Some(Value::Array(apps)) = get_field(map, "data") {
            for app in apps {
                if let Value::Map(app_map) = app {
                    let is_our_app = get_string_field(app_map, "installed_app_id")
                        .map(|id| id == app_id)
                        .unwrap_or(false);

                    if is_our_app {
                        if let Some(Value::Map(cell_info)) = get_field(app_map, "cell_info") {
                            // Iterate roles to find matching one (or first if no filter)
                            for (role_key, cells) in cell_info {
                                let role_name = match role_key {
                                    Value::String(s) => s.as_str().map(|s| s.to_string()),
                                    _ => None,
                                };

                                // Check role filter if specified
                                if let Some(filter) = role_filter {
                                    if role_name.as_deref() != Some(filter) {
                                        continue;
                                    }
                                }

                                if let Value::Array(cell_arr) = cells {
                                    for cell in cell_arr {
                                        if let Value::Map(cell_map) = cell {
                                            if let Some(Value::Array(cell_id)) =
                                                get_field(cell_map, "cell_id")
                                            {
                                                if cell_id.len() >= 2 {
                                                    let dna = extract_bytes(&cell_id[0])?;
                                                    let agent = extract_bytes(&cell_id[1])?;

                                                    // Serialize cell_id as MessagePack for zome calls
                                                    let cell_id_value = Value::Array(vec![
                                                        Value::Binary(dna.clone()),
                                                        Value::Binary(agent.clone()),
                                                    ]);
                                                    let mut buf = Vec::new();
                                                    rmpv::encode::write_value(&mut buf, &cell_id_value)
                                                        .map_err(|e| {
                                                            StorageError::Parse(format!(
                                                                "Failed to serialize cell_id: {}",
                                                                e
                                                            ))
                                                        })?;

                                                    info!(
                                                        app_id = app_id,
                                                        role = ?role_name,
                                                        dna_hash = %hex::encode(&dna[..8.min(dna.len())]),
                                                        "Discovered cell_id"
                                                    );

                                                    return Ok(buf);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Err(StorageError::NotFound(format!(
        "App '{}' not found or has no cells{}",
        app_id,
        role_filter.map(|r| format!(" (role: {})", r)).unwrap_or_default()
    )))
}

// =============================================================================
// Helper Functions
// =============================================================================

fn get_field<'a>(map: &'a [(Value, Value)], key: &str) -> Option<&'a Value> {
    for (k, v) in map {
        if let Value::String(s) = k {
            if s.as_str() == Some(key) {
                return Some(v);
            }
        }
    }
    None
}

fn get_string_field(map: &[(Value, Value)], key: &str) -> Option<String> {
    get_field(map, key).and_then(|v| {
        if let Value::String(s) = v {
            s.as_str().map(|s| s.to_string())
        } else {
            None
        }
    })
}

fn get_error_message(map: &[(Value, Value)]) -> String {
    get_field(map, "data")
        .and_then(|v| {
            if let Value::String(s) = v {
                s.as_str().map(|s| s.to_string())
            } else if let Value::Map(data_map) = v {
                get_string_field(data_map, "message")
            } else {
                None
            }
        })
        .unwrap_or_else(|| "Unknown error".to_string())
}

fn extract_bytes(value: &Value) -> Result<Vec<u8>, StorageError> {
    match value {
        Value::Binary(b) => Ok(b.clone()),
        _ => Err(StorageError::Parse("Expected binary value".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cell_id() {
        // Mock response from list_apps
        let response = Value::Map(vec![
            (Value::String("type".into()), Value::String("apps_listed".into())),
            (
                Value::String("data".into()),
                Value::Array(vec![Value::Map(vec![
                    (Value::String("installed_app_id".into()), Value::String("elohim".into())),
                    (
                        Value::String("cell_info".into()),
                        Value::Map(vec![(
                            Value::String("lamad".into()),
                            Value::Array(vec![Value::Map(vec![(
                                Value::String("cell_id".into()),
                                Value::Array(vec![
                                    Value::Binary(vec![1, 2, 3, 4]),  // dna_hash
                                    Value::Binary(vec![5, 6, 7, 8]),  // agent_pub_key
                                ]),
                            )])]),
                        )]),
                    ),
                ])]),
            ),
        ]);

        let result = parse_cell_id_from_apps(&response, "elohim", Some("lamad"));
        assert!(result.is_ok());
        let cell_id = result.unwrap();
        assert!(!cell_id.is_empty());
    }
}
