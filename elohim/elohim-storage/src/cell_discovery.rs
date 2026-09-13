//! Cell Discovery - Discover cell_id from Holochain conductor admin interface
//!
//! Used by ImportApi to find the correct cell for zome calls.
//!
//! ## Usage
//!
//! ```ignore
//! let (dna_hash, agent_pub_key) = discover_cell_id(
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

/// Cell ID components returned by discovery
#[derive(Debug, Clone)]
pub struct CellIdComponents {
    /// DNA hash bytes (typically 39 bytes including type prefix)
    pub dna_hash: Vec<u8>,
    /// Agent public key bytes (typically 39 bytes including type prefix)
    pub agent_pub_key: Vec<u8>,
}

/// Discover cell_id from conductor admin interface
///
/// Returns the cell_id components (dna_hash and agent_pub_key) separately
/// for proper wire protocol encoding.
///
/// # Arguments
/// * `admin_url` - Conductor admin WebSocket URL (e.g., "ws://localhost:4444")
/// * `app_id` - Installed app ID to find
/// * `role_filter` - Optional role name to match (e.g., "lamad")
///
/// # Returns
/// * `Ok(CellIdComponents)` - DNA hash and agent pubkey
/// * `Err(StorageError)` - If discovery fails
pub async fn discover_cell_id(
    admin_url: &str,
    app_id: &str,
    role_filter: Option<&str>,
) -> Result<CellIdComponents, StorageError> {
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
    // Note: Holochain 0.6+ uses "value" not "data" for request parameters
    let inner = Value::Map(vec![
        (
            Value::String("type".into()),
            Value::String("list_apps".into()),
        ),
        (Value::String("value".into()), Value::Map(vec![])),
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
        (
            Value::String("id".into()),
            Value::Integer(request_id.into()),
        ),
        (
            Value::String("type".into()),
            Value::String("request".into()),
        ),
        (Value::String("data".into()), Value::Binary(inner_buf)),
    ]);

    let mut envelope_buf = Vec::new();
    rmpv::encode::write_value(&mut envelope_buf, &envelope)
        .map_err(|e| StorageError::Parse(format!("Failed to encode envelope: {}", e)))?;

    // Send request
    write
        .send(Message::Binary(envelope_buf))
        .await
        .map_err(|e| StorageError::Connection(format!("Failed to send: {}", e)))?;

    // Wait for response with matching ID (timeout 30s)
    let timeout = Duration::from_secs(30);
    let response = tokio::time::timeout(timeout, async {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    let mut cursor = Cursor::new(&data[..]);
                    let value = rmpv::decode::read_value(&mut cursor).map_err(|e| {
                        StorageError::Parse(format!("Failed to parse response: {}", e))
                    })?;

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

/// An explicit role.clone selector never resolves to a provisioned cell.
/// Clone suffixes match either the human name or the conductor's role.N id.
pub(crate) fn split_cell_target(target: &str) -> Result<(&str, Option<&str>), StorageError> {
    let mut parts = target.split('.');
    let role = parts.next().unwrap_or_default();
    let clone = parts.next();
    if role.is_empty()
        || clone == Some("")
        || parts.next().is_some()
        || target.chars().any(char::is_whitespace)
    {
        return Err(StorageError::Parse(format!(
            "Invalid cell target '{target}': expected role or role.clone"
        )));
    }
    Ok((role, clone))
}

pub(crate) fn select_target_cell(
    cells: &[holochain_client::CellInfo],
    target: &str,
) -> Result<holochain_client::CellId, StorageError> {
    let (_, clone) = split_cell_target(target)?;
    let matches: Vec<_> = cells
        .iter()
        .filter_map(|cell| match (clone, cell) {
            (None, holochain_client::CellInfo::Provisioned(p)) => Some(p.cell_id.clone()),
            (Some(name), holochain_client::CellInfo::Cloned(c))
                if c.enabled && (c.name == name || c.clone_id.to_string() == target) =>
            {
                Some(c.cell_id.clone())
            }
            _ => None,
        })
        .collect();
    if matches.len() != 1 {
        return Err(StorageError::NotFound(format!(
            "Cell target '{target}': expected one enabled cell, found {}; no fallback",
            matches.len()
        )));
    }
    Ok(matches[0].clone())
}

/// Parse cell_id from list_apps response
fn parse_cell_id_from_apps(
    response: &Value,
    app_id: &str,
    role_filter: Option<&str>,
) -> Result<CellIdComponents, StorageError> {
    let selector = role_filter.map(split_cell_target).transpose()?;
    // Log response structure for debugging
    debug!(
        app_id = %app_id,
        role_filter = ?role_filter,
        response_type = ?std::mem::discriminant(response),
        "Parsing list_apps response"
    );

    // Response structure can be:
    // - Wrapped: { type: "apps_listed", data: [...] }
    // - Wrapped (0.6+): { type: "apps_listed", value: [...] }
    // - Direct: [...]

    // Handle both wrapped and direct array formats
    // Holochain uses "value" in newer API versions
    let apps = match response {
        Value::Array(arr) => {
            debug!(apps_count = arr.len(), "Direct array response");
            arr
        }
        Value::Map(map) => {
            // Log available keys for debugging
            let keys: Vec<_> = map
                .iter()
                .filter_map(|(k, _)| {
                    if let Value::String(s) = k {
                        s.as_str().map(String::from)
                    } else {
                        None
                    }
                })
                .collect();
            debug!(keys = ?keys, "Map response with keys");

            if let Some(Value::Array(arr)) = get_field(map, "value") {
                debug!(apps_count = arr.len(), "Found apps in 'value' field");
                arr
            } else if let Some(Value::Array(arr)) = get_field(map, "data") {
                debug!(apps_count = arr.len(), "Found apps in 'data' field");
                arr
            } else {
                // Log the actual structure for debugging
                info!(
                    keys = ?keys,
                    "Cell discovery failed: response has no value/data array field"
                );
                return Err(StorageError::NotFound(format!(
                    "App '{}' not found - no value/data field in response (keys: {:?})",
                    app_id, keys
                )));
            }
        }
        _ => {
            info!(
                response_type = ?std::mem::discriminant(response),
                "Cell discovery failed: unexpected response type"
            );
            return Err(StorageError::NotFound(format!(
                "App '{}' not found - unexpected response format: {:?}",
                app_id,
                std::mem::discriminant(response)
            )));
        }
    };

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
                        if let Some((filter, _)) = selector {
                            if role_name.as_deref() != Some(filter) {
                                continue;
                            }
                        }

                        if let Value::Array(cell_arr) = cells {
                            if let Some((_, Some(clone_name))) = selector {
                                let mut matches = Vec::new();
                                for cell in cell_arr {
                                    let Value::Map(map) = cell else { continue };
                                    let value = if get_string_field(map, "type").as_deref()
                                        == Some("cloned")
                                    {
                                        get_field(map, "value")
                                    } else {
                                        get_field(map, "cloned")
                                    };
                                    let Some(Value::Map(cloned)) = value else {
                                        continue;
                                    };
                                    if get_field(cloned, "enabled") != Some(&Value::Boolean(true)) {
                                        continue;
                                    }
                                    if get_string_field(cloned, "name").as_deref()
                                        != Some(clone_name)
                                        && get_string_field(cloned, "clone_id").as_deref()
                                            != role_filter
                                    {
                                        continue;
                                    }
                                    // Reuse the existing cell-id decoding for both tuple/map encodings.
                                    let wrapped = vec![(
                                        Value::from("provisioned"),
                                        Value::Map(cloned.clone()),
                                    )];
                                    if let Some((dna_hash, agent_pub_key)) =
                                        extract_js_provisioned_cell_id(&wrapped)
                                    {
                                        matches.push(CellIdComponents {
                                            dna_hash,
                                            agent_pub_key,
                                        });
                                    }
                                }
                                if matches.len() != 1 {
                                    return Err(StorageError::NotFound(format!(
                                        "Cell target '{}': expected one enabled clone, found {}; no fallback",
                                        role_filter.unwrap_or_default(), matches.len()
                                    )));
                                }
                                return Ok(matches.remove(0));
                            }
                            for cell in cell_arr {
                                if let Value::Map(cell_map) = cell {
                                    // Try Holochain 0.3+ provisioned format first
                                    // Format: { type: "provisioned", value: { cell_id: { dna_hash, agent_pub_key } } }
                                    if let Some((dna, agent)) =
                                        extract_provisioned_cell_id(cell_map)
                                    {
                                        info!(
                                            app_id = app_id,
                                            role = ?role_name,
                                            dna_hash = %hex::encode(&dna[..8.min(dna.len())]),
                                            "Discovered cell_id (provisioned format)"
                                        );

                                        return Ok(CellIdComponents {
                                            dna_hash: dna,
                                            agent_pub_key: agent,
                                        });
                                    }

                                    // Try JS client format: { provisioned: { cell_id: [dna, agent] } }
                                    // This is the format returned by @holochain/client JS library
                                    if let Some((dna, agent)) =
                                        extract_js_provisioned_cell_id(cell_map)
                                    {
                                        info!(
                                            app_id = app_id,
                                            role = ?role_name,
                                            dna_hash = %hex::encode(&dna[..8.min(dna.len())]),
                                            "Discovered cell_id (JS client format)"
                                        );

                                        return Ok(CellIdComponents {
                                            dna_hash: dna,
                                            agent_pub_key: agent,
                                        });
                                    }

                                    // Fall back to legacy format: { cell_id: [dna, agent] }
                                    if let Some(Value::Array(cell_id)) =
                                        get_field(cell_map, "cell_id")
                                    {
                                        if cell_id.len() >= 2 {
                                            let dna = extract_bytes(&cell_id[0])?;
                                            let agent = extract_bytes(&cell_id[1])?;

                                            info!(
                                                app_id = app_id,
                                                role = ?role_name,
                                                dna_hash = %hex::encode(&dna[..8.min(dna.len())]),
                                                "Discovered cell_id (legacy format)"
                                            );

                                            return Ok(CellIdComponents {
                                                dna_hash: dna,
                                                agent_pub_key: agent,
                                            });
                                        }
                                    }

                                    // No format matched - log cell structure for debugging
                                    let cell_keys: Vec<_> = cell_map
                                        .iter()
                                        .filter_map(|(k, _)| {
                                            if let Value::String(s) = k {
                                                s.as_str().map(String::from)
                                            } else {
                                                None
                                            }
                                        })
                                        .collect();
                                    debug!(
                                        role = ?role_name,
                                        cell_keys = ?cell_keys,
                                        "Cell found but format not recognized"
                                    );
                                }
                            }
                        }
                    }
                }

                // App found but no provisioned cells
                info!(
                    app_id = %app_id,
                    role_filter = ?role_filter,
                    "App found but no provisioned cells matched"
                );
            }
        }
    }

    Err(StorageError::NotFound(format!(
        "App '{}' not found or has no cells{}",
        app_id,
        role_filter
            .map(|r| format!(" (role: {})", r))
            .unwrap_or_default()
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

/// Extract cell_id from Holochain 0.3+ provisioned cell format
/// Format: { type: "provisioned", value: { cell_id: { dna_hash: <bytes>, agent_pub_key: <bytes> } } }
fn extract_provisioned_cell_id(cell_map: &[(Value, Value)]) -> Option<(Vec<u8>, Vec<u8>)> {
    // Check if this is a provisioned cell
    let cell_type = get_string_field(cell_map, "type");
    if cell_type.as_deref() != Some("provisioned") {
        return None;
    }

    // Get the value field
    let value = get_field(cell_map, "value")?;
    let value_map = match value {
        Value::Map(m) => m,
        _ => return None,
    };

    // Get cell_id from value
    let cell_id = get_field(value_map, "cell_id")?;

    match cell_id {
        // New format: { dna_hash: <bytes>, agent_pub_key: <bytes> }
        Value::Map(id_map) => {
            let dna = get_field(id_map, "dna_hash").and_then(|v| {
                if let Value::Binary(b) = v {
                    Some(b.clone())
                } else {
                    None
                }
            })?;
            let agent = get_field(id_map, "agent_pub_key").and_then(|v| {
                if let Value::Binary(b) = v {
                    Some(b.clone())
                } else {
                    None
                }
            })?;
            Some((dna, agent))
        }
        // Also handle legacy format just in case: [dna, agent]
        Value::Array(arr) if arr.len() >= 2 => {
            let dna = if let Value::Binary(b) = &arr[0] {
                Some(b.clone())
            } else {
                None
            }?;
            let agent = if let Value::Binary(b) = &arr[1] {
                Some(b.clone())
            } else {
                None
            }?;
            Some((dna, agent))
        }
        _ => None,
    }
}

/// Extract cell_id from JS client format
/// Format: { provisioned: { cell_id: [dna, agent] } }
/// This is returned by @holochain/client when calling list_apps
fn extract_js_provisioned_cell_id(cell_map: &[(Value, Value)]) -> Option<(Vec<u8>, Vec<u8>)> {
    // Check if "provisioned" is a key in the map (not a type value)
    let provisioned = get_field(cell_map, "provisioned")?;
    let provisioned_map = match provisioned {
        Value::Map(m) => m,
        _ => return None,
    };

    // Get cell_id from provisioned object
    let cell_id = get_field(provisioned_map, "cell_id")?;

    match cell_id {
        // Format: [dna, agent] as binary arrays
        Value::Array(arr) if arr.len() >= 2 => {
            let dna = if let Value::Binary(b) = &arr[0] {
                Some(b.clone())
            } else {
                None
            }?;
            let agent = if let Value::Binary(b) = &arr[1] {
                Some(b.clone())
            } else {
                None
            }?;
            Some((dna, agent))
        }
        // Also handle map format: { dna_hash, agent_pub_key }
        Value::Map(id_map) => {
            let dna = get_field(id_map, "dna_hash").and_then(|v| {
                if let Value::Binary(b) = v {
                    Some(b.clone())
                } else {
                    None
                }
            })?;
            let agent = get_field(id_map, "agent_pub_key").and_then(|v| {
                if let Value::Binary(b) = v {
                    Some(b.clone())
                } else {
                    None
                }
            })?;
            Some((dna, agent))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_clone_selection_is_strict() {
        use holochain_client::{CellId, CellInfo};
        use holochain_types::prelude::{AgentPubKey, CloneId, DnaHash, DnaModifiers};
        let base_id = CellId::new(
            DnaHash::from_raw_32(vec![1; 32]),
            AgentPubKey::from_raw_32(vec![2; 32]),
        );
        let clone_id = CellId::new(
            DnaHash::from_raw_32(vec![3; 32]),
            base_id.agent_pubkey().clone(),
        );
        let modifiers = DnaModifiers {
            network_seed: "fixture-test".into(),
            properties: ().try_into().unwrap(),
        };
        let base = CellInfo::new_provisioned(base_id.clone(), modifiers.clone(), "lamad".into());
        let clone = CellInfo::new_cloned(
            clone_id.clone(),
            CloneId::new(&"lamad".into(), 0),
            base_id.dna_hash().clone(),
            modifiers,
            "fixtures".into(),
            true,
        );
        let cells = [base.clone(), clone.clone()];
        assert_eq!(select_target_cell(&cells, "lamad").unwrap(), base_id);
        for target in ["lamad.fixtures", "lamad.0"] {
            assert_eq!(select_target_cell(&cells, target).unwrap(), clone_id);
        }
        assert!(select_target_cell(&cells, "lamad.missing").is_err());
        assert!(select_target_cell(&cells[..1], "lamad.fixtures").is_err());
        assert!(select_target_cell(&[clone.clone(), clone.clone()], "lamad.fixtures").is_err());
        let CellInfo::Cloned(mut disabled) = clone else {
            unreachable!()
        };
        disabled.enabled = false;
        assert!(select_target_cell(&[base, CellInfo::Cloned(disabled)], "lamad.fixtures").is_err());
    }

    fn clone_response(keyed: bool, enabled: bool, copies: usize) -> Value {
        let clone = Value::Map(vec![
            (
                Value::from("cell_id"),
                Value::Array(vec![Value::Binary(vec![3; 39]), Value::Binary(vec![4; 39])]),
            ),
            (Value::from("clone_id"), Value::from("lamad.0")),
            (Value::from("name"), Value::from("fixtures")),
            (Value::from("enabled"), Value::Boolean(enabled)),
        ]);
        let cloned = if keyed {
            Value::Map(vec![(Value::from("cloned"), clone)])
        } else {
            Value::Map(vec![
                (Value::from("type"), Value::from("cloned")),
                (Value::from("value"), clone),
            ])
        };
        let mut cells = vec![Value::Map(vec![(
            Value::from("cell_id"),
            Value::Array(vec![Value::Binary(vec![1; 39]), Value::Binary(vec![2; 39])]),
        )])];
        cells.extend(std::iter::repeat_n(cloned, copies));
        Value::Array(vec![Value::Map(vec![
            (Value::from("installed_app_id"), Value::from("elohim")),
            (
                Value::from("cell_info"),
                Value::Map(vec![(Value::from("lamad"), Value::Array(cells))]),
            ),
        ])])
    }

    #[test]
    fn clone_targets_never_fall_back() {
        for keyed in [true, false] {
            let response = clone_response(keyed, true, 1);
            for target in ["lamad.fixtures", "lamad.0"] {
                assert_eq!(
                    parse_cell_id_from_apps(&response, "elohim", Some(target))
                        .unwrap()
                        .dna_hash,
                    vec![3; 39]
                );
            }
            assert_eq!(
                parse_cell_id_from_apps(&response, "elohim", None)
                    .unwrap()
                    .dna_hash,
                vec![1; 39]
            );
            for target in ["lamad.missing", "imagodei.fixtures", "lamad.", ""] {
                assert!(parse_cell_id_from_apps(&response, "elohim", Some(target)).is_err());
            }
            for (enabled, copies) in [(false, 1), (true, 0), (true, 2)] {
                assert!(parse_cell_id_from_apps(
                    &clone_response(keyed, enabled, copies),
                    "elohim",
                    Some("lamad.fixtures")
                )
                .is_err());
            }
        }
    }

    #[test]
    fn test_parse_cell_id() {
        // Mock response from list_apps
        let response = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("apps_listed".into()),
            ),
            (
                Value::String("data".into()),
                Value::Array(vec![Value::Map(vec![
                    (
                        Value::String("installed_app_id".into()),
                        Value::String("elohim".into()),
                    ),
                    (
                        Value::String("cell_info".into()),
                        Value::Map(vec![(
                            Value::String("lamad".into()),
                            Value::Array(vec![Value::Map(vec![(
                                Value::String("cell_id".into()),
                                Value::Array(vec![
                                    Value::Binary(vec![1, 2, 3, 4]), // dna_hash
                                    Value::Binary(vec![5, 6, 7, 8]), // agent_pub_key
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
        assert_eq!(cell_id.dna_hash, vec![1, 2, 3, 4]);
        assert_eq!(cell_id.agent_pub_key, vec![5, 6, 7, 8]);
    }

    #[test]
    fn test_parse_js_client_format() {
        // JS client format: { provisioned: { cell_id: [dna, agent] } }
        let response = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("apps_listed".into()),
            ),
            (
                Value::String("data".into()),
                Value::Array(vec![Value::Map(vec![
                    (
                        Value::String("installed_app_id".into()),
                        Value::String("elohim".into()),
                    ),
                    (
                        Value::String("cell_info".into()),
                        Value::Map(vec![(
                            Value::String("lamad".into()),
                            Value::Array(vec![Value::Map(vec![(
                                Value::String("provisioned".into()),
                                Value::Map(vec![(
                                    Value::String("cell_id".into()),
                                    Value::Array(vec![
                                        Value::Binary(vec![1, 2, 3, 4]), // dna_hash
                                        Value::Binary(vec![5, 6, 7, 8]), // agent_pub_key
                                    ]),
                                )]),
                            )])]),
                        )]),
                    ),
                ])]),
            ),
        ]);

        let result = parse_cell_id_from_apps(&response, "elohim", Some("lamad"));
        assert!(
            result.is_ok(),
            "Should parse JS client format: {:?}",
            result
        );
        let cell_id = result.unwrap();
        assert_eq!(cell_id.dna_hash, vec![1, 2, 3, 4]);
        assert_eq!(cell_id.agent_pub_key, vec![5, 6, 7, 8]);
    }
}
