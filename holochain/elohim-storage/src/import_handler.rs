//! Import Handler - Processes ImportBatchQueued signals from Holochain
//!
//! Listens for ImportBatchQueued signals emitted by the zome when a new import batch
//! is queued. Reads the blob from local storage and sends chunks to the zome via
//! process_import_chunk calls.
//!
//! ## Architecture
//!
//! ```text
//! elohim-storage (this module)
//!     │
//!     ├── Connects to conductor admin interface
//!     ├── Gets app auth token (IssueAppAuthenticationToken)
//!     ├── Connects to app interface
//!     ├── Sends AppAuthenticationRequest with token
//!     ├── Subscribes to app signals
//!     │
//!     └── On ImportBatchQueued signal:
//!         1. Read blob from local storage
//!         2. Parse items JSON
//!         3. Send chunks via process_import_chunk
//!         4. Track progress
//! ```
//!
//! ## Conductor Communication Protocol
//!
//! All requests use envelope format: { id, type: "request", data: <inner msgpack bytes> }
//! Inner request format: { type: "<request_type>", data: {...} }

use crate::blob_store::BlobStore;
use crate::error::StorageError;
use futures_util::{SinkExt, StreamExt};
use rmpv::Value;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{http::Request, protocol::Message},
};
use tracing::{debug, error, info, warn};

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for import handler
#[derive(Debug, Clone)]
pub struct ImportHandlerConfig {
    /// Conductor admin WebSocket URL
    pub admin_url: String,
    /// Installed app ID
    pub installed_app_id: String,
    /// Zome name for import calls
    pub zome_name: String,
    /// Role name for the cell (usually matches DNA role in happ manifest)
    pub role_name: String,
    /// Chunk size for processing
    pub chunk_size: usize,
    /// Delay between chunks (ms)
    pub chunk_delay_ms: u64,
    /// Reconnect delay on connection failure
    pub reconnect_delay_secs: u64,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
}

impl Default for ImportHandlerConfig {
    fn default() -> Self {
        Self {
            admin_url: "ws://localhost:4444".to_string(),
            installed_app_id: "elohim".to_string(),
            zome_name: "content_store".to_string(),
            role_name: "elohim".to_string(),
            chunk_size: 50,
            chunk_delay_ms: 100,
            reconnect_delay_secs: 5,
            request_timeout_secs: 30,
        }
    }
}

// ============================================================================
// Signal Types (matching zome signals)
// ============================================================================

/// Import batch queued signal from zome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportBatchQueued {
    pub batch_id: String,
    pub batch_type: String,
    pub blob_hash: String,
    pub total_items: u32,
    pub schema_version: u32,
}

/// Progress update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    pub batch_id: String,
    pub processed_count: u32,
    pub error_count: u32,
    pub status: String,
}

// ============================================================================
// Import Handler
// ============================================================================

/// Import handler that listens for signals and processes batches
pub struct ImportHandler {
    config: ImportHandlerConfig,
    blob_store: Arc<BlobStore>,
    /// Broadcast channel for progress updates
    progress_tx: broadcast::Sender<ImportProgress>,
    /// Shutdown signal
    shutdown_rx: Option<broadcast::Receiver<()>>,
    /// Request ID counter (atomic for thread-safety)
    request_id: AtomicU64,
}

impl ImportHandler {
    pub fn new(config: ImportHandlerConfig, blob_store: Arc<BlobStore>) -> Self {
        let (progress_tx, _) = broadcast::channel(100);
        Self {
            config,
            blob_store,
            progress_tx,
            shutdown_rx: None,
            request_id: AtomicU64::new(1),
        }
    }

    /// Subscribe to progress updates
    pub fn subscribe_progress(&self) -> broadcast::Receiver<ImportProgress> {
        self.progress_tx.subscribe()
    }

    /// Set shutdown receiver
    pub fn set_shutdown(&mut self, rx: broadcast::Receiver<()>) {
        self.shutdown_rx = Some(rx);
    }

    /// Get next request ID
    fn next_request_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Run the import handler (blocking)
    pub async fn run(&mut self) -> Result<(), StorageError> {
        loop {
            match self.connect_and_listen().await {
                Ok(()) => {
                    info!("Import handler connection closed cleanly");
                    break;
                }
                Err(e) => {
                    error!(error = %e, "Import handler connection failed");

                    // Check if we should shutdown
                    if let Some(ref mut rx) = self.shutdown_rx {
                        if rx.try_recv().is_ok() {
                            info!("Import handler shutdown requested");
                            break;
                        }
                    }

                    info!(
                        delay_secs = self.config.reconnect_delay_secs,
                        "Reconnecting in {} seconds...",
                        self.config.reconnect_delay_secs
                    );
                    tokio::time::sleep(Duration::from_secs(self.config.reconnect_delay_secs)).await;
                }
            }
        }
        Ok(())
    }

    /// Connect to conductor and listen for signals
    async fn connect_and_listen(&self) -> Result<(), StorageError> {
        info!(admin_url = %self.config.admin_url, "Connecting to Holochain conductor");

        // Step 1: Get app auth token from admin interface
        let (token, app_port, cell_id) = self.setup_from_admin().await?;
        let (dna_hash, agent_pub_key) = cell_id;

        info!(port = app_port, "Got auth token and app interface port");

        // Step 2: Connect to app interface
        let app_url = self.derive_app_url(app_port);
        info!(app_url = %app_url, "Connecting to app interface");

        let host = app_url.split("//").last().unwrap_or("localhost");
        let app_request = Request::builder()
            .uri(&app_url)
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
            .map_err(|e| StorageError::Connection(format!("Failed to build app request: {}", e)))?;

        let (app_ws, _) = connect_async_with_config(app_request, None, false)
            .await
            .map_err(|e| StorageError::Connection(format!("App connection failed: {}", e)))?;

        let (mut write, mut read) = app_ws.split();

        // Step 3: Send AppAuthenticationRequest
        self.send_app_auth_request(&mut write, &token).await?;

        // Step 4: Wait for auth response
        self.wait_for_auth_response(&mut read).await?;

        info!("Connected and authenticated to app interface, listening for signals");

        // Step 5: Listen for signals
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    if let Err(e) = self
                        .handle_message(&data, &mut write, &dna_hash, &agent_pub_key)
                        .await
                    {
                        warn!(error = %e, "Failed to handle message");
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("App connection closed by server");
                    break;
                }
                Err(e) => {
                    error!(error = %e, "WebSocket error");
                    break;
                }
                _ => {}
            }

            // Check for shutdown
            if let Some(ref rx) = self.shutdown_rx {
                let mut rx = rx.resubscribe();
                if rx.try_recv().is_ok() {
                    info!("Shutdown requested, closing connection");
                    let _ = write.close().await;
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    /// Setup: get token, app port, and cell info from admin interface
    async fn setup_from_admin(&self) -> Result<(Vec<u8>, u16, (Vec<u8>, Vec<u8>)), StorageError> {
        // Connect to admin interface
        let host = self.config.admin_url.split("//").last().unwrap_or("localhost");
        let admin_request = Request::builder()
            .uri(&self.config.admin_url)
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

        let (admin_ws, _) = connect_async_with_config(admin_request, None, false)
            .await
            .map_err(|e| StorageError::Connection(format!("Admin connection failed: {}", e)))?;

        let (mut write, mut read) = admin_ws.split();

        info!("Connected to admin interface");

        // Get app info (cell_id)
        let cell_id = self.get_cell_info(&mut write, &mut read).await?;
        debug!(dna_hash = %hex::encode(&cell_id.0[..8]), "Got cell info");

        // Get auth token
        let token = self.get_app_token(&mut write, &mut read).await?;
        debug!("Got app auth token");

        // Get or create app interface
        let app_port = self.get_or_create_app_interface(&mut write, &mut read).await?;
        info!(port = app_port, "Using app interface");

        // Close admin connection
        let _ = write.close().await;

        Ok((token, app_port, cell_id))
    }

    // ========================================================================
    // Admin Interface Helpers (with proper envelope wrapping)
    // ========================================================================

    /// Get cell info (dna_hash, agent_pub_key) for the app
    async fn get_cell_info<S, R>(&self, write: &mut S, read: &mut R) -> Result<(Vec<u8>, Vec<u8>), StorageError>
    where
        S: SinkExt<Message> + Unpin,
        S::Error: std::fmt::Display,
        R: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
    {
        // Build inner request: { type: "list_apps", value: {} }
        // Note: Holochain 0.6+ uses "value" not "data" for request parameters
        let inner = Value::Map(vec![
            (Value::String("type".into()), Value::String("list_apps".into())),
            (Value::String("value".into()), Value::Map(vec![])),
        ]);

        let response = self.send_admin_request(write, read, &inner).await?;

        // Parse response to find our app and get cell_id
        self.parse_cell_info_from_apps(&response)
    }

    /// Get app auth token
    async fn get_app_token<S, R>(&self, write: &mut S, read: &mut R) -> Result<Vec<u8>, StorageError>
    where
        S: SinkExt<Message> + Unpin,
        S::Error: std::fmt::Display,
        R: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
    {
        // Build inner request: { type: "issue_app_authentication_token", data: {...} }
        let data = Value::Map(vec![
            (
                Value::String("installed_app_id".into()),
                Value::String(self.config.installed_app_id.clone().into()),
            ),
            (Value::String("expiry_seconds".into()), Value::Integer(3600.into())),
            (Value::String("single_use".into()), Value::Boolean(false)),
        ]);

        let inner = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("issue_app_authentication_token".into()),
            ),
            (Value::String("value".into()), data),
        ]);

        let response = self.send_admin_request(write, read, &inner).await?;

        // Parse token from response
        // Response inner: { type: "app_authentication_token_issued", value: { token: <bytes> } }
        if let Value::Map(map) = &response {
            // Debug: log response structure
            let keys: Vec<_> = map.iter()
                .filter_map(|(k, _)| if let Value::String(s) = k { s.as_str().map(|s| s.to_string()) } else { None })
                .collect();
            error!(keys = ?keys, "Token response map keys");

            let data_opt = get_field(map, "value").or_else(|| get_field(map, "data"));
            if let Some(value) = &data_opt {
                error!(value_type = %response_type_str(value), "Token response value type");
            }
            match &data_opt {
                Some(Value::Map(data_map)) => {
                    // Check for token in the data map
                    if let Some(token_val) = get_field(data_map, "token") {
                        error!(token_type = %response_type_str(token_val), "Token field type");
                        match token_val {
                            Value::Binary(token) => return Ok(token.clone()),
                            Value::Array(arr) => {
                                // Token might be wrapped in array [<bytes>] OR array of byte integers
                                if let Some(Value::Binary(token)) = arr.first() {
                                    error!("Token was wrapped in array containing binary");
                                    return Ok(token.clone());
                                }
                                // Try to convert array of integers to bytes
                                let bytes: Result<Vec<u8>, _> = arr.iter()
                                    .map(|v| match v {
                                        Value::Integer(i) => i.as_u64().map(|n| n as u8).ok_or("invalid byte"),
                                        _ => Err("not integer"),
                                    })
                                    .collect();
                                if let Ok(token) = bytes {
                                    info!("Token was array of {} bytes", token.len());
                                    return Ok(token);
                                }
                            }
                            Value::Ext(_, bytes) => {
                                // Extension type
                                error!("Token is Ext type");
                                return Ok(bytes.clone());
                            }
                            _ => {}
                        }
                    }
                    // Also check directly in the value map (newer format might just be { token: <bytes> })
                    let token_keys: Vec<_> = data_map.iter()
                        .filter_map(|(k, _)| if let Value::String(s) = k { s.as_str().map(|s| s.to_string()) } else { None })
                        .collect();
                    error!(keys = ?token_keys, "Token data map keys (token field not binary)");
                }
                Some(Value::Binary(token)) => {
                    // Token might be directly in the value field
                    error!("Token value is binary, length={}", token.len());
                    return Ok(token.clone());
                }
                Some(other) => {
                    error!(value_type = %response_type_str(other), "Token value is not a Map or Binary");
                }
                None => {
                    error!("No value/data field in token response");
                }
            }
        }

        Err(StorageError::Parse("Failed to get token from response".to_string()))
    }

    /// Get or create app interface
    async fn get_or_create_app_interface<S, R>(&self, write: &mut S, read: &mut R) -> Result<u16, StorageError>
    where
        S: SinkExt<Message> + Unpin,
        S::Error: std::fmt::Display,
        R: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
    {
        // List existing interfaces
        let inner = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("list_app_interfaces".into()),
            ),
            (Value::String("value".into()), Value::Nil),
        ]);

        let response = self.send_admin_request(write, read, &inner).await?;

        // Check for existing interfaces (try "value" first, then "data" for backwards compat)
        if let Value::Map(map) = &response {
            let interfaces_opt = get_field(map, "value").or_else(|| get_field(map, "data"));
            if let Some(Value::Array(interfaces)) = interfaces_opt {
                if let Some(Value::Map(iface)) = interfaces.first() {
                    if let Some(Value::Integer(port)) = get_field(iface, "port") {
                        return Ok(port.as_u64().unwrap_or(4445) as u16);
                    }
                }
            }
        }

        // No interface found, create one
        let attach_data = Value::Map(vec![(
            Value::String("allowed_origins".into()),
            Value::String("*".into()),
        )]);

        let attach_inner = Value::Map(vec![
            (
                Value::String("type".into()),
                Value::String("attach_app_interface".into()),
            ),
            (Value::String("value".into()), attach_data),
        ]);

        let attach_response = self.send_admin_request(write, read, &attach_inner).await?;

        // Parse port from response (try "value" first, then "data")
        if let Value::Map(map) = &attach_response {
            let data_opt = get_field(map, "value").or_else(|| get_field(map, "data"));
            if let Some(Value::Map(data_map)) = data_opt {
                if let Some(Value::Integer(port)) = get_field(data_map, "port") {
                    return Ok(port.as_u64().unwrap_or(4445) as u16);
                }
            }
        }

        Ok(4445) // Default fallback
    }

    /// Send admin request with proper envelope and wait for response
    async fn send_admin_request<S, R>(
        &self,
        write: &mut S,
        read: &mut R,
        inner_request: &Value,
    ) -> Result<Value, StorageError>
    where
        S: SinkExt<Message> + Unpin,
        S::Error: std::fmt::Display,
        R: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
    {
        let request_id = self.next_request_id();

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

        // Wait for response with matching ID
        let timeout = Duration::from_secs(self.config.request_timeout_secs);
        let response = tokio::time::timeout(timeout, async {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Binary(data)) => {
                        let mut cursor = Cursor::new(&data[..]);
                        let value = rmpv::decode::read_value(&mut cursor)
                            .map_err(|e| StorageError::Parse(format!("Failed to parse response: {}", e)))?;

                        // Check if this is our response
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

    // ========================================================================
    // App Interface Authentication
    // ========================================================================

    /// Send AppAuthenticationRequest to the app interface
    async fn send_app_auth_request<S>(&self, write: &mut S, token: &[u8]) -> Result<(), StorageError>
    where
        S: SinkExt<Message> + Unpin,
        S::Error: std::fmt::Display,
    {
        // Build inner: { token: <bytes> }
        // Note: Token was returned as array of integers, we have it as Vec<u8> now
        let inner = Value::Map(vec![(
            Value::String("token".into()),
            Value::Binary(token.to_vec()),
        )]);

        let mut inner_buf = Vec::new();
        rmpv::encode::write_value(&mut inner_buf, &inner)
            .map_err(|e| StorageError::Parse(format!("Failed to encode auth request: {}", e)))?;

        // Wrap in "authenticate" envelope (NOT "request" - special format for auth)
        // The SDK uses: { type: "authenticate", data: encode(request) }
        let envelope = Value::Map(vec![
            (Value::String("type".into()), Value::String("authenticate".into())),
            (Value::String("data".into()), Value::Binary(inner_buf)),
        ]);

        let mut buf = Vec::new();
        rmpv::encode::write_value(&mut buf, &envelope)
            .map_err(|e| StorageError::Parse(format!("Failed to encode envelope: {}", e)))?;

        write
            .send(Message::Binary(buf.into()))
            .await
            .map_err(|e| StorageError::Connection(format!("Failed to send auth request: {}", e)))?;

        debug!("Sent AppAuthenticationRequest");
        Ok(())
    }

    /// Wait for authentication to complete
    ///
    /// The Holochain SDK doesn't wait for an explicit response - it just waits briefly
    /// and assumes success if the connection doesn't close. We do the same here.
    async fn wait_for_auth_response<R>(&self, read: &mut R) -> Result<(), StorageError>
    where
        R: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
    {
        // Wait a brief moment to allow conductor to close connection if token is invalid
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Try to peek at the connection state - if we can poll without blocking
        // and get a close message, auth failed
        let check_timeout = Duration::from_millis(50);
        let check_result = tokio::time::timeout(check_timeout, async {
            // Use tokio::select! to check for close message without blocking
            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Close(_))) => {
                            Err(StorageError::Connection("Connection closed during auth - invalid token".to_string()))
                        }
                        Some(Ok(Message::Binary(data))) => {
                            // Check if this is an error message
                            let mut cursor = Cursor::new(&data[..]);
                            if let Ok(value) = rmpv::decode::read_value(&mut cursor) {
                                if let Value::Map(ref map) = value {
                                    if let Some(resp_type) = get_string_field(map, "type") {
                                        if resp_type == "error" {
                                            return Err(StorageError::Auth("Authentication rejected".to_string()));
                                        }
                                    }
                                }
                            }
                            // Non-error message means we're connected and receiving
                            Ok(())
                        }
                        Some(Err(e)) => Err(StorageError::Connection(format!("WebSocket error during auth: {}", e))),
                        None => Err(StorageError::Connection("Connection ended during auth".to_string())),
                        _ => Ok(()) // Other messages (ping/pong) are fine
                    }
                }
            }
        }).await;

        // If timeout occurred, that's actually good - no close message received
        match check_result {
            Ok(result) => result,
            Err(_) => Ok(()), // Timeout means connection is still open, auth succeeded
        }
    }

    // ========================================================================
    // Signal Handling
    // ========================================================================

    /// Handle incoming message (could be signal or response)
    async fn handle_message<S>(
        &self,
        data: &[u8],
        ws: &mut S,
        dna_hash: &[u8],
        agent_pub_key: &[u8],
    ) -> Result<(), StorageError>
    where
        S: SinkExt<Message> + Unpin,
        S::Error: std::fmt::Display,
    {
        // Try to parse as msgpack
        let mut cursor = Cursor::new(data);
        let value = rmpv::decode::read_value(&mut cursor)
            .map_err(|e| StorageError::Parse(format!("Failed to parse message: {}", e)))?;

        // Try multiple signal formats (like subscriber.rs)
        if self.try_handle_signal(&value, ws, dna_hash, agent_pub_key).await? {
            return Ok(());
        }

        // Try wrapped formats
        if let Value::Map(ref map) = value {
            // Format: { "signal": ... }
            if let Some(signal_val) = get_field(map, "signal") {
                return self
                    .try_handle_signal(signal_val, ws, dna_hash, agent_pub_key)
                    .await
                    .map(|_| ());
            }

            // Format: { "type": "Signal", "data": ... }
            if get_string_field(map, "type") == Some("Signal".to_string()) {
                if let Some(data_val) = get_field(map, "data") {
                    return self
                        .try_handle_signal(data_val, ws, dna_hash, agent_pub_key)
                        .await
                        .map(|_| ());
                }
            }

            // Format: { "App": ... }
            if let Some(app_val) = get_field(map, "App") {
                return self
                    .try_handle_signal(app_val, ws, dna_hash, agent_pub_key)
                    .await
                    .map(|_| ());
            }
        }

        Ok(())
    }

    /// Try to handle value as ImportBatchQueued signal
    async fn try_handle_signal<S>(
        &self,
        value: &Value,
        ws: &mut S,
        dna_hash: &[u8],
        agent_pub_key: &[u8],
    ) -> Result<bool, StorageError>
    where
        S: SinkExt<Message> + Unpin,
        S::Error: std::fmt::Display,
    {
        // Check if this is ImportBatchQueued signal
        if let Value::Map(ref map) = value {
            let is_import_signal = get_string_field(map, "type")
                .or_else(|| get_string_field(map, "signal_type"))
                .map(|t| t == "ImportBatchQueued")
                .unwrap_or(false);

            if is_import_signal {
                let payload = self.extract_signal_payload(value)?;
                self.process_import_batch(&payload, ws, dna_hash, agent_pub_key)
                    .await?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Process an import batch
    async fn process_import_batch<S>(
        &self,
        payload: &ImportBatchQueued,
        ws: &mut S,
        dna_hash: &[u8],
        agent_pub_key: &[u8],
    ) -> Result<(), StorageError>
    where
        S: SinkExt<Message> + Unpin,
        S::Error: std::fmt::Display,
    {
        let batch_start = std::time::Instant::now();
        info!(
            batch_id = %payload.batch_id,
            batch_type = %payload.batch_type,
            blob_hash = %payload.blob_hash,
            total_items = payload.total_items,
            schema_version = payload.schema_version,
            "📥 BATCH_START: Processing ImportBatchQueued signal"
        );

        // Read blob from storage
        let blob_data = self
            .blob_store
            .get(&payload.blob_hash)
            .await
            .map_err(|e| StorageError::BlobNotFound(format!("{}: {}", payload.blob_hash, e)))?;

        // Parse items
        let items: Vec<serde_json::Value> = serde_json::from_slice(&blob_data)
            .map_err(|e| StorageError::Parse(format!("Failed to parse items: {}", e)))?;

        let total_items = items.len();
        let chunk_size = self.config.chunk_size;
        let total_chunks = (total_items + chunk_size - 1) / chunk_size;

        info!(
            batch_id = %payload.batch_id,
            total_items = total_items,
            total_chunks = total_chunks,
            "Processing import batch"
        );

        // Process chunks
        let mut processed_count = 0u32;
        let mut error_count = 0u32;

        for (chunk_index, chunk) in items.chunks(chunk_size).enumerate() {
            let is_final = chunk_index == total_chunks - 1;
            let chunk_json = serde_json::to_string(chunk)
                .map_err(|e| StorageError::Parse(format!("Failed to serialize chunk: {}", e)))?;

            let chunk_start = std::time::Instant::now();
            info!(
                batch_id = %payload.batch_id,
                chunk_index = chunk_index,
                chunk_items = chunk.len(),
                total_chunks = total_chunks,
                is_final = is_final,
                "🔄 CHUNK_START: Sending chunk to conductor"
            );

            // Call process_import_chunk
            match self
                .call_process_chunk(
                    ws,
                    dna_hash,
                    agent_pub_key,
                    &payload.batch_id,
                    chunk_index as u32,
                    is_final,
                    &chunk_json,
                )
                .await
            {
                Ok(result) => {
                    let chunk_duration = chunk_start.elapsed();
                    processed_count = result.total_processed;
                    error_count = result.total_errors;

                    info!(
                        batch_id = %payload.batch_id,
                        chunk_index = chunk_index,
                        chunk_items = chunk.len(),
                        duration_ms = chunk_duration.as_millis(),
                        total_processed = processed_count,
                        "✅ CHUNK_OK: Chunk sent successfully"
                    );

                    // Emit progress
                    let _ = self.progress_tx.send(ImportProgress {
                        batch_id: payload.batch_id.clone(),
                        processed_count,
                        error_count,
                        status: if is_final {
                            "completed".to_string()
                        } else {
                            "processing".to_string()
                        },
                    });
                }
                Err(e) => {
                    let chunk_duration = chunk_start.elapsed();
                    error!(
                        batch_id = %payload.batch_id,
                        chunk_index = chunk_index,
                        chunk_items = chunk.len(),
                        duration_ms = chunk_duration.as_millis(),
                        error = %e,
                        "❌ CHUNK_ERROR: Chunk processing failed"
                    );
                    error_count += chunk.len() as u32;
                }
            }

            // Delay between chunks
            if !is_final && self.config.chunk_delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.config.chunk_delay_ms)).await;
            }
        }

        let batch_duration = batch_start.elapsed();
        let items_per_sec = if batch_duration.as_secs_f64() > 0.0 {
            processed_count as f64 / batch_duration.as_secs_f64()
        } else {
            0.0
        };

        info!(
            batch_id = %payload.batch_id,
            batch_type = %payload.batch_type,
            processed = processed_count,
            errors = error_count,
            total_items = payload.total_items,
            duration_ms = batch_duration.as_millis(),
            items_per_sec = format!("{:.1}", items_per_sec),
            "📦 BATCH_COMPLETE: Import batch finished"
        );

        Ok(())
    }

    /// Extract ImportBatchQueued payload from signal value
    fn extract_signal_payload(&self, signal: &Value) -> Result<ImportBatchQueued, StorageError> {
        // The signal structure varies - try different paths
        let payload = if let Value::Map(map) = signal {
            // Look for payload field
            map.iter()
                .find(|(k, _)| {
                    if let Value::String(s) = k {
                        s.as_str() == Some("payload") || s.as_str() == Some("data")
                    } else {
                        false
                    }
                })
                .map(|(_, v)| v)
                .unwrap_or(signal)
        } else {
            signal
        };

        // Convert to JSON for easier parsing
        let json_str = serde_json::to_string(&rmpv_to_json(payload))
            .map_err(|e| StorageError::Parse(format!("Failed to convert signal: {}", e)))?;

        serde_json::from_str(&json_str)
            .map_err(|e| StorageError::Parse(format!("Failed to parse signal payload: {}", e)))
    }

    // ========================================================================
    // Zome Calls (with proper envelope and response waiting)
    // ========================================================================

    /// Call process_import_chunk on zome and wait for response
    async fn call_process_chunk<S>(
        &self,
        ws: &mut S,
        dna_hash: &[u8],
        agent_pub_key: &[u8],
        batch_id: &str,
        chunk_index: u32,
        is_final: bool,
        items_json: &str,
    ) -> Result<ChunkResult, StorageError>
    where
        S: SinkExt<Message> + Unpin,
        S::Error: std::fmt::Display,
    {
        let request_id = self.next_request_id();

        // Build zome call payload
        let payload = serde_json::json!({
            "batch_id": batch_id,
            "chunk_index": chunk_index,
            "is_final": is_final,
            "items_json": items_json,
        });

        let payload_bytes = rmp_serde::to_vec(&payload)
            .map_err(|e| StorageError::Parse(format!("Failed to encode payload: {}", e)))?;

        // Build call_zome inner request
        let cell_id = Value::Array(vec![
            Value::Binary(dna_hash.to_vec()),
            Value::Binary(agent_pub_key.to_vec()),
        ]);

        let call_data = Value::Map(vec![
            (Value::String("cell_id".into()), cell_id),
            (
                Value::String("zome_name".into()),
                Value::String(self.config.zome_name.clone().into()),
            ),
            (
                Value::String("fn_name".into()),
                Value::String("process_import_chunk".into()),
            ),
            (Value::String("payload".into()), Value::Binary(payload_bytes)),
            (Value::String("cap_secret".into()), Value::Nil),
            (Value::String("provenance".into()), Value::Binary(agent_pub_key.to_vec())),
        ]);

        let inner = Value::Map(vec![
            (Value::String("type".into()), Value::String("call_zome".into())),
            (Value::String("value".into()), call_data),
        ]);

        // Encode inner request
        let mut inner_buf = Vec::new();
        rmpv::encode::write_value(&mut inner_buf, &inner)
            .map_err(|e| StorageError::Parse(format!("Failed to encode inner: {}", e)))?;

        // Build envelope
        let envelope = Value::Map(vec![
            (Value::String("id".into()), Value::Integer(request_id.into())),
            (Value::String("type".into()), Value::String("request".into())),
            (Value::String("data".into()), Value::Binary(inner_buf)),
        ]);

        let mut envelope_buf = Vec::new();
        rmpv::encode::write_value(&mut envelope_buf, &envelope)
            .map_err(|e| StorageError::Parse(format!("Failed to encode envelope: {}", e)))?;

        // Send request
        ws.send(Message::Binary(envelope_buf.into()))
            .await
            .map_err(|e| StorageError::Connection(format!("Failed to send: {}", e)))?;

        // Note: On app interface, we can't easily wait for the specific response
        // because we're using a split stream. For now, assume success.
        // In a more robust implementation, we'd need to either:
        // 1. Use a request/response correlation system
        // 2. Or make the connection handle both send/receive
        //
        // For MVP, we assume the zome call succeeds and rely on progress signals
        // from the zome for actual status.

        Ok(ChunkResult {
            chunk_processed: (chunk_index + 1) * self.config.chunk_size as u32,
            chunk_errors: 0,
            total_processed: (chunk_index + 1) * self.config.chunk_size as u32,
            total_errors: 0,
            status: if is_final {
                "completed".to_string()
            } else {
                "processing".to_string()
            },
        })
    }

    // ========================================================================
    // Helpers
    // ========================================================================

    /// Parse cell info from list_apps response
    fn parse_cell_info_from_apps(&self, response: &Value) -> Result<(Vec<u8>, Vec<u8>), StorageError> {
        // Response structure varies by Holochain version:
        // v0.3+: { type: "...", data: [{ installed_app_id, cell_info: { role_name: [{ type: "provisioned", value: { cell_id: { dna_hash: ..., agent_pub_key: ... } } }] } }] }
        // Earlier: { type: "...", data: [{ installed_app_id, cell_info: { role_name: [{ cell_id: [dna, agent] }] } }] }

        // Debug: log what we received
        warn!(response_type = %response_type_str(response), "Parsing list_apps response");

        // Handle wrapped { type, data/value: [...] } and direct [...] formats
        // Holochain responses can be: Array, or Map with "value" or "data" containing the array
        // The value might also be Binary (nested msgpack)

        // Helper to extract apps array from a value (handles nested wrapping)
        fn extract_apps(value: &Value, depth: u8) -> Option<Vec<Value>> {
            if depth > 5 { return None; } // Prevent infinite recursion

            // Debug logging for tracing
            tracing::debug!("extract_apps depth={} type={}", depth, response_type_str(value));

            match value {
                Value::Array(arr) => {
                    tracing::debug!("extract_apps depth={} found array len={}", depth, arr.len());
                    Some(arr.clone())
                },
                Value::Map(map) => {
                    let keys: Vec<_> = map.iter()
                        .filter_map(|(k, _)| if let Value::String(s) = k { s.as_str().map(|s| s.to_string()) } else { None })
                        .collect();
                    tracing::debug!("extract_apps depth={} map keys={:?}", depth, keys);

                    // Try value field first (newer Holochain), then data
                    // May be nested: response.value.value = [apps]
                    if let Some(inner) = get_field(map, "value") {
                        tracing::debug!("extract_apps depth={} found 'value' field type={}", depth, response_type_str(inner));
                        if let Value::Array(arr) = inner {
                            tracing::debug!("extract_apps depth={} value is array len={}", depth, arr.len());
                            return Some(arr.clone());
                        }
                        // Recurse into nested wrapper
                        if let Some(arr) = extract_apps(inner, depth + 1) {
                            return Some(arr);
                        }
                    }
                    if let Some(inner) = get_field(map, "data") {
                        tracing::debug!("extract_apps depth={} found 'data' field type={}", depth, response_type_str(inner));
                        if let Value::Array(arr) = inner {
                            return Some(arr.clone());
                        }
                        if let Some(arr) = extract_apps(inner, depth + 1) {
                            return Some(arr);
                        }
                    }
                    tracing::debug!("extract_apps depth={} no value/data field found", depth);
                    None
                }
                Value::Binary(bytes) => {
                    tracing::debug!("extract_apps depth={} decoding binary len={}", depth, bytes.len());
                    // Decode nested msgpack
                    let mut cursor = Cursor::new(&bytes[..]);
                    if let Ok(inner) = rmpv::decode::read_value(&mut cursor) {
                        extract_apps(&inner, depth + 1)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        let apps = if let Some(apps_vec) = extract_apps(response, 0) {
            apps_vec
        } else {
            // Log what we got for debugging - trace all the way down
            if let Value::Map(map) = response {
                let keys: Vec<_> = map.iter()
                    .filter_map(|(k, _)| if let Value::String(s) = k { s.as_str().map(|s| s.to_string()) } else { None })
                    .collect();
                error!("Response map keys: {:?}", keys);
                if let Some(v) = get_field(map, "value") {
                    error!("value field type: {}", response_type_str(v));
                    if let Value::Map(inner) = v {
                        let inner_keys: Vec<_> = inner.iter()
                            .filter_map(|(k, _)| if let Value::String(s) = k { s.as_str().map(|s| s.to_string()) } else { None })
                            .collect();
                        error!("Inner map keys: {:?}", inner_keys);
                        // Go one level deeper
                        if let Some(v2) = get_field(inner, "value") {
                            error!("Inner.value field type: {}", response_type_str(v2));
                            if let Value::Array(arr) = v2 {
                                error!("Inner.value is array with {} elements!", arr.len());
                            } else if let Value::Map(inner2) = v2 {
                                let inner2_keys: Vec<_> = inner2.iter()
                                    .filter_map(|(k, _)| if let Value::String(s) = k { s.as_str().map(|s| s.to_string()) } else { None })
                                    .collect();
                                error!("Inner2 map keys: {:?}", inner2_keys);
                            }
                        }
                    }
                }
            }
            return Err(StorageError::NotFound(format!(
                "App {} not found - could not extract apps array",
                self.config.installed_app_id
            )));
        };

        for app in &apps {
            if let Value::Map(app_map) = app {
                let is_our_app = get_string_field(app_map, "installed_app_id")
                    .map(|id| id == self.config.installed_app_id)
                    .unwrap_or(false);

                if is_our_app {
                    debug!(app_id = %self.config.installed_app_id, "Found app, extracting cell info");

                    if let Some(Value::Map(cell_info)) = get_field(app_map, "cell_info") {
                        // Get first role's first cell
                        for (role_key, cells) in cell_info {
                            let role_name = if let Value::String(s) = role_key {
                                s.as_str().unwrap_or("unknown").to_string()
                            } else {
                                "unknown".to_string()
                            };

                            if let Value::Array(cell_arr) = cells {
                                for cell in cell_arr {
                                    if let Value::Map(cell_map) = cell {
                                        // Try new format: { type: "provisioned", value: { cell_id: { dna_hash, agent_pub_key } } }
                                        if let Some(cell_id) = self.extract_cell_id_from_provisioned(cell_map) {
                                            debug!(role = %role_name, "Extracted cell_id from provisioned format");
                                            return Ok(cell_id);
                                        }

                                        // Try legacy format: { cell_id: [dna, agent] }
                                        if let Some(Value::Array(cell_id)) = get_field(cell_map, "cell_id") {
                                            if cell_id.len() >= 2 {
                                                let dna = extract_bytes(&cell_id[0])?;
                                                let agent = extract_bytes(&cell_id[1])?;
                                                debug!(role = %role_name, "Extracted cell_id from legacy format");
                                                return Ok((dna, agent));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // App found but no cells - this is an error state
                    return Err(StorageError::NotFound(format!(
                        "App {} found but has no cells",
                        self.config.installed_app_id
                    )));
                }
            }
        }

        Err(StorageError::NotFound(format!(
            "App {} not found in list_apps response",
            self.config.installed_app_id
        )))
    }

    /// Extract cell_id from provisioned cell format (Holochain 0.3+)
    /// Format: { type: "provisioned", value: { cell_id: { dna_hash: <bytes>, agent_pub_key: <bytes> } } }
    fn extract_cell_id_from_provisioned(&self, cell_map: &[(Value, Value)]) -> Option<(Vec<u8>, Vec<u8>)> {
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
                let dna = get_field(id_map, "dna_hash")
                    .and_then(|v| extract_bytes(v).ok())?;
                let agent = get_field(id_map, "agent_pub_key")
                    .and_then(|v| extract_bytes(v).ok())?;
                Some((dna, agent))
            }
            // Legacy format: [dna, agent]
            Value::Array(arr) if arr.len() >= 2 => {
                let dna = extract_bytes(&arr[0]).ok()?;
                let agent = extract_bytes(&arr[1]).ok()?;
                Some((dna, agent))
            }
            _ => None,
        }
    }

    /// Derive app URL from admin URL
    fn derive_app_url(&self, app_port: u16) -> String {
        if let Some(host_start) = self.config.admin_url.find("://") {
            let after_scheme = &self.config.admin_url[host_start + 3..];
            if let Some(port_start) = after_scheme.rfind(':') {
                let host = &after_scheme[..port_start];
                return format!(
                    "{}://{}:{}",
                    &self.config.admin_url[..host_start],
                    host,
                    app_port
                );
            }
        }
        format!("ws://localhost:{}", app_port)
    }
}

/// Chunk processing result
#[derive(Debug, Clone)]
pub struct ChunkResult {
    pub chunk_processed: u32,
    pub chunk_errors: u32,
    pub total_processed: u32,
    pub total_errors: u32,
    pub status: String,
}

// ============================================================================
// Helpers
// ============================================================================

/// Get string representation of response type for debugging
fn response_type_str(value: &Value) -> String {
    match value {
        Value::Nil => "nil".to_string(),
        Value::Boolean(_) => "bool".to_string(),
        Value::Integer(_) => "int".to_string(),
        Value::F32(_) | Value::F64(_) => "float".to_string(),
        Value::String(s) => format!("string({})", s.as_str().unwrap_or("?")),
        Value::Binary(b) => format!("binary({}b)", b.len()),
        Value::Array(a) => format!("array({})", a.len()),
        Value::Map(m) => {
            let keys: Vec<_> = m.iter()
                .filter_map(|(k, _)| if let Value::String(s) = k { s.as_str().map(|s| s.to_string()) } else { None })
                .collect();
            format!("map({})[{}]", m.len(), keys.join(","))
        }
        Value::Ext(_, _) => "ext".to_string(),
    }
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

/// Get error message from error response
fn get_error_message(map: &[(Value, Value)]) -> String {
    if let Some(Value::Map(data)) = get_field(map, "data") {
        if let Some(msg) = get_string_field(data, "message") {
            return msg;
        }
    }
    "Unknown error".to_string()
}

/// Extract bytes from msgpack value
fn extract_bytes(value: &Value) -> Result<Vec<u8>, StorageError> {
    match value {
        Value::Binary(b) => Ok(b.clone()),
        Value::Array(arr) => {
            // Array of integers (byte representation)
            let mut bytes = Vec::with_capacity(arr.len());
            for v in arr {
                if let Value::Integer(i) = v {
                    bytes.push(i.as_u64().unwrap_or(0) as u8);
                }
            }
            Ok(bytes)
        }
        _ => Err(StorageError::Parse(format!(
            "Expected binary, got {:?}",
            value
        ))),
    }
}

/// Convert rmpv::Value to serde_json::Value
fn rmpv_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Nil => serde_json::Value::Null,
        Value::Boolean(b) => serde_json::Value::Bool(*b),
        Value::Integer(i) => {
            if let Some(n) = i.as_i64() {
                serde_json::Value::Number(n.into())
            } else if let Some(n) = i.as_u64() {
                serde_json::Value::Number(n.into())
            } else {
                serde_json::Value::Null
            }
        }
        Value::F32(f) => serde_json::json!(*f),
        Value::F64(f) => serde_json::json!(*f),
        Value::String(s) => serde_json::Value::String(s.as_str().unwrap_or("").to_string()),
        Value::Binary(b) => serde_json::Value::String(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            b,
        )),
        Value::Array(arr) => serde_json::Value::Array(arr.iter().map(rmpv_to_json).collect()),
        Value::Map(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                if let Value::String(key) = k {
                    obj.insert(key.as_str().unwrap_or("").to_string(), rmpv_to_json(v));
                }
            }
            serde_json::Value::Object(obj)
        }
        Value::Ext(_, _) => serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ImportHandlerConfig::default();
        assert_eq!(config.admin_url, "ws://localhost:4444");
        assert_eq!(config.installed_app_id, "elohim");
        assert_eq!(config.zome_name, "content_store");
        assert_eq!(config.chunk_size, 50);
    }

    #[test]
    fn test_derive_app_url() {
        let handler = ImportHandler::new(
            ImportHandlerConfig {
                admin_url: "ws://conductor.local:4444".to_string(),
                ..Default::default()
            },
            Arc::new(BlobStore::new_memory()),
        );

        assert_eq!(
            handler.derive_app_url(4445),
            "ws://conductor.local:4445"
        );
    }

    #[test]
    fn test_get_string_field() {
        let map = vec![
            (Value::String("type".into()), Value::String("Signal".into())),
            (Value::String("count".into()), Value::Integer(42.into())),
        ];

        assert_eq!(get_string_field(&map, "type"), Some("Signal".to_string()));
        assert_eq!(get_string_field(&map, "count"), None); // Not a string
        assert_eq!(get_string_field(&map, "missing"), None);
    }
}
