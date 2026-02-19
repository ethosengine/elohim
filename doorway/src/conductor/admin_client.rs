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

/// Info about an installed app discovered on a conductor.
#[derive(Debug, Clone)]
pub struct InstalledAppInfo {
    /// The installed app ID (e.g. "elohim", "elohim-conductor-0-abc123")
    pub installed_app_id: String,
    /// Raw 39-byte Holochain agent public key
    pub agent_pub_key: Vec<u8>,
}

/// A cell ID: (dna_hash, agent_pub_key) — both 39 bytes.
pub type CellIdPair = (Vec<u8>, Vec<u8>);

/// Detailed app info including cell IDs per role.
///
/// Used by the Chaperone endpoint to grant capabilities per cell.
#[derive(Debug, Clone)]
pub struct AppInfoDetailed {
    /// The installed app ID
    pub installed_app_id: String,
    /// Raw 39-byte Holochain agent public key
    pub agent_pub_key: Vec<u8>,
    /// Cell IDs keyed by role name
    pub cell_ids: Vec<(String, CellIdPair)>,
}

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

    /// Get the admin WebSocket URL.
    pub fn admin_url(&self) -> &str {
        &self.admin_url
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
                    if let Some(err_value) = get_field(map, "value") {
                        return Err(format!(
                            "Admin error (key generation): unstructured error: {err_value:?}"
                        ));
                    }
                    return Err(
                        "Admin error (key generation): empty error (no value field)".to_string()
                    );
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

    /// List all installed apps on the conductor.
    ///
    /// Returns the installed_app_id and raw 39-byte agent public key for each app.
    /// Used at startup to discover pre-existing agent→conductor mappings.
    pub async fn list_apps(&self) -> Result<Vec<InstalledAppInfo>, String> {
        // Build inner request: { type: "list_apps", value: { status_filter: null } }
        let data = Value::Map(vec![(Value::String("status_filter".into()), Value::Nil)]);

        let inner = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("list_apps".into()),
            ),
            (Value::String("value".into()), data),
        ]);

        let response = self.send_request(&inner).await?;
        self.check_error_response(&response, "list_apps")?;

        // Response: { type: "apps_listed", value: [AppInfo, ...] }
        let mut apps = Vec::new();
        if let Value::Map(ref map) = response {
            if let Some(Value::Array(ref app_list)) = get_field(map, "value") {
                for app_info in app_list {
                    if let Value::Map(ref info) = app_info {
                        let installed_app_id =
                            get_string_field(info, "installed_app_id").unwrap_or_default();

                        // Try top-level agent_pub_key (Holochain 0.4+)
                        let agent_key =
                            if let Some(Value::Binary(key)) = get_field(info, "agent_pub_key") {
                                Some(key.clone())
                            } else {
                                // Fallback: extract from cell_info
                                extract_agent_from_cell_info(info)
                            };

                        if let Some(key) = agent_key {
                            if !installed_app_id.is_empty() {
                                apps.push(InstalledAppInfo {
                                    installed_app_id,
                                    agent_pub_key: key,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(apps)
    }

    /// Get detailed app info including cell IDs for each role.
    ///
    /// Extends `list_apps` by parsing `cell_info` to return all provisioned cells
    /// with their role names. Used by the Chaperone to grant caps per cell.
    pub async fn get_app_info(&self, installed_app_id: &str) -> Result<AppInfoDetailed, String> {
        let data = Value::Map(vec![(Value::String("status_filter".into()), Value::Nil)]);

        let inner = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("list_apps".into()),
            ),
            (Value::String("value".into()), data),
        ]);

        let response = self.send_request(&inner).await?;
        self.check_error_response(&response, "list_apps (get_app_info)")?;

        if let Value::Map(ref map) = response {
            if let Some(Value::Array(ref app_list)) = get_field(map, "value") {
                for app_info in app_list {
                    if let Value::Map(ref info) = app_info {
                        let app_id = get_string_field(info, "installed_app_id").unwrap_or_default();
                        if app_id != installed_app_id {
                            continue;
                        }

                        // Extract agent key
                        let agent_pub_key =
                            if let Some(Value::Binary(key)) = get_field(info, "agent_pub_key") {
                                key.clone()
                            } else {
                                extract_agent_from_cell_info(info).unwrap_or_default()
                            };

                        // Extract all cell IDs from cell_info
                        let cell_ids = extract_all_cells_from_cell_info(info);

                        return Ok(AppInfoDetailed {
                            installed_app_id: app_id,
                            agent_pub_key,
                            cell_ids,
                        });
                    }
                }
            }
        }

        Err(format!("App '{installed_app_id}' not found on conductor"))
    }

    /// Grant zome call capability for a cell.
    ///
    /// Allows the given `signing_key` to make zome calls on `cell_id`
    /// using the provided `cap_secret`.
    pub async fn grant_zome_call_capability(
        &self,
        cell_id: (&[u8], &[u8]),
        cap_secret: &[u8],
        signing_key: &[u8],
        tag: &str,
    ) -> Result<(), String> {
        let cell_id_value = Value::Array(vec![
            Value::Binary(cell_id.0.to_vec()),
            Value::Binary(cell_id.1.to_vec()),
        ]);

        let cap_grant = Value::Map(vec![
            (Value::String("tag".into()), Value::String(tag.into())),
            (
                Value::String("functions".into()),
                Value::Map(vec![(
                    Value::String("type".into()),
                    Value::String("all".into()),
                )]),
            ),
            (
                Value::String("access".into()),
                Value::Map(vec![
                    (
                        Value::String("type".into()),
                        Value::String("assigned".into()),
                    ),
                    (
                        Value::String("value".into()),
                        Value::Map(vec![
                            (
                                Value::String("secret".into()),
                                Value::Binary(cap_secret.to_vec()),
                            ),
                            (
                                Value::String("assignees".into()),
                                Value::Array(vec![Value::Binary(signing_key.to_vec())]),
                            ),
                        ]),
                    ),
                ]),
            ),
        ]);

        let data = Value::Map(vec![
            (Value::String("cell_id".into()), cell_id_value),
            (Value::String("cap_grant".into()), cap_grant),
        ]);

        let inner = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("grant_zome_call_capability".into()),
            ),
            (Value::String("value".into()), data),
        ]);

        let response = self.send_request(&inner).await?;
        self.check_error_response(&response, "grant_zome_call_capability")?;

        Ok(())
    }

    /// Authorize signing credentials for a cell.
    ///
    /// Tells the conductor to accept zome calls signed by `signing_key`
    /// for the given cell, using the provided `cap_secret`.
    pub async fn authorize_signing_credentials(
        &self,
        cell_id: (&[u8], &[u8]),
        signing_key: &[u8],
        cap_secret: &[u8],
    ) -> Result<(), String> {
        let cell_id_value = Value::Array(vec![
            Value::Binary(cell_id.0.to_vec()),
            Value::Binary(cell_id.1.to_vec()),
        ]);

        let data = Value::Map(vec![
            (Value::String("cell_id".into()), cell_id_value),
            (
                Value::String("signing_key".into()),
                Value::Binary(signing_key.to_vec()),
            ),
            (
                Value::String("cap_secret".into()),
                Value::Binary(cap_secret.to_vec()),
            ),
        ]);

        let inner = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("authorize_signing_credentials".into()),
            ),
            (Value::String("value".into()), data),
        ]);

        let response = self.send_request(&inner).await?;
        self.check_error_response(&response, "authorize_signing_credentials")?;

        Ok(())
    }

    /// Issue an app authentication token for AppWebsocket connections.
    ///
    /// Returns the raw token bytes that the client passes to `AppWebsocket.connect()`.
    pub async fn issue_app_authentication_token(
        &self,
        installed_app_id: &str,
        expiry_seconds: u64,
    ) -> Result<Vec<u8>, String> {
        let data = Value::Map(vec![
            (
                Value::String("installed_app_id".into()),
                Value::String(installed_app_id.into()),
            ),
            (
                Value::String("expiry_seconds".into()),
                Value::Integer(expiry_seconds.into()),
            ),
            (Value::String("single_use".into()), Value::Boolean(false)),
        ]);

        let inner = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("issue_app_authentication_token".into()),
            ),
            (Value::String("value".into()), data),
        ]);

        let response = self.send_request(&inner).await?;
        self.check_error_response(&response, "issue_app_authentication_token")?;

        // Response: { type: "app_authentication_token_issued", value: { token: <bytes> } }
        if let Value::Map(ref map) = response {
            if let Some(Value::Map(ref token_map)) = get_field(map, "value") {
                if let Some(Value::Binary(token)) = get_field(token_map, "token") {
                    return Ok(token.clone());
                }
            }
            // Some conductor versions return token directly in value
            if let Some(Value::Binary(token)) = get_field(map, "value") {
                return Ok(token.clone());
            }
        }

        Err(format!(
            "Unexpected issue_app_authentication_token response: {response:?}"
        ))
    }

    /// List app interface ports on the conductor.
    ///
    /// Returns the ports that app WebSocket connections can connect to.
    pub async fn list_app_interfaces(&self) -> Result<Vec<u16>, String> {
        let inner = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("list_app_interfaces".into()),
            ),
            (Value::String("value".into()), Value::Nil),
        ]);

        let response = self.send_request(&inner).await?;
        self.check_error_response(&response, "list_app_interfaces")?;

        // Response: { type: "app_interfaces_listed", value: [{ port: N }, ...] }
        let mut ports = Vec::new();
        if let Value::Map(ref map) = response {
            if let Some(Value::Array(ref interfaces)) = get_field(map, "value") {
                for iface in interfaces {
                    if let Value::Map(ref iface_map) = iface {
                        if let Some(Value::Integer(port)) = get_field(iface_map, "port") {
                            if let Some(p) = port.as_u64() {
                                ports.push(p as u16);
                            }
                        }
                    }
                    // Some versions return just integers
                    if let Value::Integer(port) = iface {
                        if let Some(p) = port.as_u64() {
                            ports.push(p as u16);
                        }
                    }
                }
            }
        }

        Ok(ports)
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
    // Retry helpers
    // =========================================================================

    /// Retry an admin operation that may fail due to source chain contention.
    ///
    /// When multiple cap grants or credential authorizations happen in sequence,
    /// the source chain head can move between read and commit. This helper retries
    /// with exponential backoff on known conflict errors.
    pub async fn with_source_chain_retry<F, Fut>(
        op: F,
        desc: &str,
        max_retries: u32,
    ) -> Result<(), String>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<(), String>>,
    {
        for attempt in 0..max_retries {
            match op().await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    let is_retryable =
                        e.contains("HeadMoved") || e.contains("source chain head has moved");

                    if is_retryable && attempt + 1 < max_retries {
                        let delay = 100 * (1 << attempt); // 100ms, 200ms, 400ms
                        tracing::warn!(
                            attempt = attempt + 1,
                            max_retries,
                            delay_ms = delay,
                            "{desc}: source chain conflict, retrying"
                        );
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        continue;
                    }

                    return Err(e);
                }
            }
        }

        Err(format!("{desc}: max retries ({max_retries}) exceeded"))
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
                    // Include raw error value for diagnosability
                    if let Some(err_value) = get_field(map, "value") {
                        return Err(format!(
                            "Admin error ({operation}): unstructured error: {err_value:?}"
                        ));
                    }
                    return Err(format!(
                        "Admin error ({operation}): empty error (no value field)"
                    ));
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
                if let Some(err_value) = get_field(map, "value") {
                    return Err(format!(
                        "Admin error (envelope): unstructured error: {err_value:?}"
                    ));
                }
                return Err("Admin error (envelope): empty error (no value field)".to_string());
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

/// Extract all provisioned cells from cell_info, returning (role_name, (dna_hash, agent_key)).
///
/// Digs into cell_info → each role → Provisioned cells → cell_id [DnaHash, AgentPubKey].
fn extract_all_cells_from_cell_info(app_info: &[(Value, Value)]) -> Vec<(String, CellIdPair)> {
    let mut cells = Vec::new();
    let Some(cell_info) = get_field(app_info, "cell_info") else {
        return cells;
    };
    if let Value::Map(ref roles) = cell_info {
        for (role_key, role_cells) in roles {
            let role_name = if let Value::String(ref s) = role_key {
                s.as_str().unwrap_or("unknown").to_string()
            } else {
                continue;
            };
            if let Value::Array(ref cell_list) = role_cells {
                for cell in cell_list {
                    if let Value::Map(ref cell_map) = cell {
                        if let Some(Value::Array(ref cell_id)) = get_field(cell_map, "cell_id") {
                            if cell_id.len() >= 2 {
                                if let (Value::Binary(ref dna), Value::Binary(ref agent)) =
                                    (&cell_id[0], &cell_id[1])
                                {
                                    cells.push((role_name.clone(), (dna.clone(), agent.clone())));
                                    break; // First provisioned cell per role
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    cells
}

/// Extract agent pub key from cell_info when not available at top level.
///
/// Handles older Holochain versions where AppInfo lacks a top-level agent_pub_key.
/// Digs into cell_info → first role → first Provisioned cell → cell_id[1] (AgentPubKey).
fn extract_agent_from_cell_info(app_info: &[(Value, Value)]) -> Option<Vec<u8>> {
    let cell_info = get_field(app_info, "cell_info")?;
    if let Value::Map(ref roles) = cell_info {
        for (_, cells) in roles {
            if let Value::Array(ref cell_list) = cells {
                for cell in cell_list {
                    if let Value::Map(ref cell_map) = cell {
                        // CellInfo::Provisioned contains cell_id: [DnaHash, AgentPubKey]
                        if let Some(Value::Array(ref cell_id)) = get_field(cell_map, "cell_id") {
                            if cell_id.len() >= 2 {
                                if let Value::Binary(ref key) = cell_id[1] {
                                    return Some(key.clone());
                                }
                            }
                        }
                    }
                }
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

    #[test]
    fn test_extract_agent_from_cell_info() {
        let agent_key = vec![
            0x84u8, 0x20, 0x24, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
            20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36,
        ];

        // Simulate cell_info: { "role_name": [{ "cell_id": [dna_hash, agent_key] }] }
        let app_info = vec![
            (
                Value::String("installed_app_id".into()),
                Value::String("elohim".into()),
            ),
            (
                Value::String("cell_info".into()),
                Value::Map(vec![(
                    Value::String("content_store".into()),
                    Value::Array(vec![Value::Map(vec![(
                        Value::String("cell_id".into()),
                        Value::Array(vec![
                            Value::Binary(vec![0u8; 39]),     // DnaHash
                            Value::Binary(agent_key.clone()), // AgentPubKey
                        ]),
                    )])]),
                )]),
            ),
        ];

        let result = extract_agent_from_cell_info(&app_info);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), agent_key);
    }

    #[test]
    fn test_extract_agent_from_cell_info_missing() {
        let app_info = vec![(
            Value::String("installed_app_id".into()),
            Value::String("elohim".into()),
        )];

        assert!(extract_agent_from_cell_info(&app_info).is_none());
    }

    #[test]
    fn test_grant_zome_call_capability_encoding() {
        // Verify the MessagePack structure for grant_zome_call_capability
        let dna_hash = vec![0x84u8; 39];
        let agent_key = vec![0x85u8; 39];
        let cap_secret = vec![0xAAu8; 64];
        let signing_key = vec![0xBBu8; 39];

        let cell_id_value = Value::Array(vec![Value::Binary(dna_hash), Value::Binary(agent_key)]);

        let cap_grant = Value::Map(vec![
            (
                Value::String("tag".into()),
                Value::String("test-tag".into()),
            ),
            (
                Value::String("functions".into()),
                Value::Map(vec![(
                    Value::String("type".into()),
                    Value::String("all".into()),
                )]),
            ),
            (
                Value::String("access".into()),
                Value::Map(vec![
                    (
                        Value::String("type".into()),
                        Value::String("assigned".into()),
                    ),
                    (
                        Value::String("value".into()),
                        Value::Map(vec![
                            (Value::String("secret".into()), Value::Binary(cap_secret)),
                            (
                                Value::String("assignees".into()),
                                Value::Array(vec![Value::Binary(signing_key)]),
                            ),
                        ]),
                    ),
                ]),
            ),
        ]);

        let data = Value::Map(vec![
            (Value::String("cell_id".into()), cell_id_value),
            (Value::String("cap_grant".into()), cap_grant),
        ]);

        let inner = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("grant_zome_call_capability".into()),
            ),
            (Value::String("value".into()), data),
        ]);

        let bytes = encode_msgpack(&inner).unwrap();
        let envelope = build_request_envelope(1, &bytes);

        // Verify it's valid MessagePack
        let mut cursor = Cursor::new(&envelope);
        let decoded = rmpv::decode::read_value(&mut cursor).unwrap();
        assert!(matches!(decoded, Value::Map(_)));
    }

    #[test]
    fn test_authorize_signing_credentials_encoding() {
        let dna_hash = vec![0x84u8; 39];
        let agent_key = vec![0x85u8; 39];
        let signing_key = vec![0xBBu8; 39];
        let cap_secret = vec![0xAAu8; 64];

        let cell_id_value = Value::Array(vec![Value::Binary(dna_hash), Value::Binary(agent_key)]);

        let data = Value::Map(vec![
            (Value::String("cell_id".into()), cell_id_value),
            (
                Value::String("signing_key".into()),
                Value::Binary(signing_key),
            ),
            (
                Value::String("cap_secret".into()),
                Value::Binary(cap_secret),
            ),
        ]);

        let inner = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("authorize_signing_credentials".into()),
            ),
            (Value::String("value".into()), data),
        ]);

        let bytes = encode_msgpack(&inner).unwrap();
        assert!(!bytes.is_empty());

        // Round-trip: decode and verify type field
        let mut cursor = Cursor::new(&bytes);
        let decoded = rmpv::decode::read_value(&mut cursor).unwrap();
        if let Value::Map(map) = decoded {
            assert_eq!(
                get_string_field(&map, "type"),
                Some("authorize_signing_credentials".to_string())
            );
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn test_issue_app_authentication_token_encoding() {
        let data = Value::Map(vec![
            (
                Value::String("installed_app_id".into()),
                Value::String("elohim".into()),
            ),
            (
                Value::String("expiry_seconds".into()),
                Value::Integer(3600.into()),
            ),
            (Value::String("single_use".into()), Value::Boolean(false)),
        ]);

        let inner = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("issue_app_authentication_token".into()),
            ),
            (Value::String("value".into()), data),
        ]);

        let bytes = encode_msgpack(&inner).unwrap();
        let mut cursor = Cursor::new(&bytes);
        let decoded = rmpv::decode::read_value(&mut cursor).unwrap();
        if let Value::Map(map) = decoded {
            assert_eq!(
                get_string_field(&map, "type"),
                Some("issue_app_authentication_token".to_string())
            );
        } else {
            panic!("Expected Map");
        }
    }

    #[test]
    fn test_extract_all_cells_from_cell_info() {
        let dna1 = vec![0x01u8; 39];
        let dna2 = vec![0x02u8; 39];
        let agent = vec![0x84u8; 39];

        let app_info = vec![(
            Value::String("cell_info".into()),
            Value::Map(vec![
                (
                    Value::String("lamad".into()),
                    Value::Array(vec![Value::Map(vec![(
                        Value::String("cell_id".into()),
                        Value::Array(vec![
                            Value::Binary(dna1.clone()),
                            Value::Binary(agent.clone()),
                        ]),
                    )])]),
                ),
                (
                    Value::String("imagodei".into()),
                    Value::Array(vec![Value::Map(vec![(
                        Value::String("cell_id".into()),
                        Value::Array(vec![
                            Value::Binary(dna2.clone()),
                            Value::Binary(agent.clone()),
                        ]),
                    )])]),
                ),
            ]),
        )];

        let cells = extract_all_cells_from_cell_info(&app_info);
        assert_eq!(cells.len(), 2);

        let role_names: Vec<&str> = cells.iter().map(|(name, _)| name.as_str()).collect();
        assert!(role_names.contains(&"lamad"));
        assert!(role_names.contains(&"imagodei"));
    }

    #[tokio::test]
    async fn test_with_source_chain_retry_succeeds_immediately() {
        let result = AdminClient::with_source_chain_retry(|| async { Ok(()) }, "test op", 3).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_with_source_chain_retry_non_retryable_error() {
        let result = AdminClient::with_source_chain_retry(
            || async { Err("some other error".to_string()) },
            "test op",
            3,
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("some other error"));
    }

    #[tokio::test]
    async fn test_with_source_chain_retry_exhausts_retries() {
        let result = AdminClient::with_source_chain_retry(
            || async { Err("HeadMoved: source chain conflict".to_string()) },
            "test op",
            2,
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("HeadMoved"));
    }
}
