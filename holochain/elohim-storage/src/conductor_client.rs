//! Conductor Client - Connection pool for Holochain conductor
//!
//! Provides a single managed connection to the Holochain conductor's app interface.
//! Handles reconnection, request/response correlation, and timeouts.
//!
//! ## Why Single Connection?
//!
//! Unlike doorway which needs multiple connections for concurrent reads,
//! elohim-storage uses ONE connection with internal batching:
//!
//! 1. WriteBuffer batches operations before sending
//! 2. Conductor processes batches atomically
//! 3. Single connection avoids conductor connection pressure
//! 4. Backpressure signals when conductor is overwhelmed

use futures_util::{SinkExt, StreamExt};
use rmpv::Value;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use tokio::time::timeout;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{http::Request, protocol::Message},
};
use tracing::{debug, error, info, warn};

use crate::error::StorageError;

/// Configuration for conductor client
#[derive(Debug, Clone)]
pub struct ConductorClientConfig {
    /// Admin interface WebSocket URL (for auth token)
    pub admin_url: String,
    /// App interface WebSocket URL
    pub app_url: String,
    /// App ID for authentication
    pub app_id: String,
    /// Request timeout
    pub request_timeout: Duration,
    /// Reconnect delay on failure
    pub reconnect_delay: Duration,
    /// Maximum pending requests
    pub max_pending_requests: usize,
}

impl Default for ConductorClientConfig {
    fn default() -> Self {
        Self {
            admin_url: "ws://localhost:4444".to_string(),
            app_url: "ws://localhost:4445".to_string(),
            app_id: "elohim".to_string(),
            request_timeout: Duration::from_secs(60),
            reconnect_delay: Duration::from_secs(5),
            max_pending_requests: 100,
        }
    }
}

/// Single-connection conductor client with batching support
pub struct ConductorClient {
    config: ConductorClientConfig,
    /// Request sender channel
    request_tx: mpsc::Sender<PendingRequest>,
    /// Connection status
    connected: Arc<RwLock<bool>>,
    /// Request ID counter
    next_id: AtomicU64,
}

struct PendingRequest {
    id: u64,
    payload: Vec<u8>,
    response_tx: oneshot::Sender<Result<Vec<u8>, StorageError>>,
}

impl ConductorClient {
    /// Create and connect a new conductor client
    pub async fn connect(config: ConductorClientConfig) -> Result<Self, StorageError> {
        let (request_tx, request_rx) = mpsc::channel(config.max_pending_requests);
        let connected = Arc::new(RwLock::new(false));

        let client = Self {
            config: config.clone(),
            request_tx,
            connected: Arc::clone(&connected),
            next_id: AtomicU64::new(1),
        };

        // Spawn connection manager
        let conn_config = config.clone();
        let conn_connected = Arc::clone(&connected);
        tokio::spawn(async move {
            connection_loop(conn_config, request_rx, conn_connected).await;
        });

        // Wait for initial connection
        for _ in 0..50 {
            if *client.connected.read().await {
                info!(app_url = %config.app_url, "ConductorClient connected");
                return Ok(client);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err(StorageError::Conductor("Connection timeout".into()))
    }

    /// Send a zome call request
    pub async fn call_zome(
        &self,
        cell_id: &[u8],
        zome_name: &str,
        fn_name: &str,
        payload: &[u8],
    ) -> Result<Vec<u8>, StorageError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        // Build zome call request
        let call_payload = build_zome_call(cell_id, zome_name, fn_name, payload)?;

        let (response_tx, response_rx) = oneshot::channel();

        self.request_tx
            .send(PendingRequest {
                id,
                payload: call_payload,
                response_tx,
            })
            .await
            .map_err(|_| StorageError::Conductor("Connection closed".into()))?;

        match timeout(self.config.request_timeout, response_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(StorageError::Conductor("Response channel closed".into())),
            Err(_) => Err(StorageError::Conductor("Request timeout".into())),
        }
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }
}

/// Build a zome call payload (MessagePack envelope format)
fn build_zome_call(
    cell_id: &[u8],
    zome_name: &str,
    fn_name: &str,
    payload: &[u8],
) -> Result<Vec<u8>, StorageError> {
    use rmpv::encode::write_value;

    // Inner call data
    let call_data = Value::Map(vec![
        (Value::String("cell_id".into()), Value::Binary(cell_id.to_vec())),
        (Value::String("zome_name".into()), Value::String(zome_name.into())),
        (Value::String("fn_name".into()), Value::String(fn_name.into())),
        (Value::String("payload".into()), Value::Binary(payload.to_vec())),
        (Value::String("cap_secret".into()), Value::Nil),
        (Value::String("provenance".into()), Value::Nil),
    ]);

    // Inner request (Holochain 0.6+ uses 'value' not 'data')
    let inner_request = Value::Map(vec![
        (Value::String("type".into()), Value::String("call_zome".into())),
        (Value::String("value".into()), call_data),
    ]);

    // Serialize inner request
    let mut inner_bytes = Vec::new();
    write_value(&mut inner_bytes, &inner_request)
        .map_err(|e| StorageError::Internal(format!("Failed to encode inner request: {}", e)))?;

    Ok(inner_bytes)
}

/// Connection loop with reconnection logic
async fn connection_loop(
    config: ConductorClientConfig,
    mut request_rx: mpsc::Receiver<PendingRequest>,
    connected: Arc<RwLock<bool>>,
) {
    loop {
        info!(url = %config.app_url, "Connecting to conductor app interface");

        // Step 1: Get auth token from admin interface
        let auth_token = match get_auth_token(&config.admin_url, &config.app_id).await {
            Ok(token) => {
                info!("Got auth token from admin interface");
                token
            }
            Err(e) => {
                error!(error = %e, "Failed to get auth token");
                tokio::time::sleep(config.reconnect_delay).await;
                continue;
            }
        };

        // Step 2: Connect to app interface
        match connect_to_conductor(&config.app_url).await {
            Ok((mut ws_sink, mut ws_stream)) => {
                // Step 3: Authenticate
                if let Err(e) = authenticate_connection(&mut ws_sink, &mut ws_stream, &auth_token).await {
                    error!(error = %e, "Failed to authenticate");
                    tokio::time::sleep(config.reconnect_delay).await;
                    continue;
                }

                *connected.write().await = true;
                info!("Connected and authenticated to conductor");

                // Pending responses by ID
                let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Vec<u8>, StorageError>>>>> =
                    Arc::new(Mutex::new(HashMap::new()));
                let pending_for_recv = Arc::clone(&pending);

                // Spawn receiver task
                let recv_task = tokio::spawn(async move {
                    while let Some(msg) = ws_stream.next().await {
                        match msg {
                            Ok(Message::Binary(data)) => {
                                // Parse response and extract ID
                                if let Some((id, result)) = parse_response(&data) {
                                    let mut pending = pending_for_recv.lock().await;
                                    if let Some(tx) = pending.remove(&id) {
                                        let _ = tx.send(result);
                                    }
                                }
                            }
                            Ok(Message::Ping(_data)) => {
                                debug!("Received ping");
                                // Pong is handled by tungstenite automatically
                            }
                            Ok(Message::Close(_)) => {
                                info!("Conductor closed connection");
                                break;
                            }
                            Err(e) => {
                                error!(error = %e, "WebSocket error");
                                break;
                            }
                            _ => {}
                        }
                    }
                });

                // Process requests
                let mut request_id_counter = 1u64;
                while let Some(req) = request_rx.recv().await {
                    // Build envelope with ID
                    let envelope = build_envelope(request_id_counter, &req.payload);

                    // Store pending response
                    {
                        let mut pending = pending.lock().await;
                        pending.insert(request_id_counter, req.response_tx);
                    }
                    request_id_counter += 1;

                    // Send to conductor
                    if let Err(e) = ws_sink.send(Message::Binary(envelope)).await {
                        error!(error = %e, "Failed to send to conductor");
                        break;
                    }
                }

                recv_task.abort();
                *connected.write().await = false;
            }
            Err(e) => {
                error!(error = %e, "Failed to connect to conductor");
            }
        }

        warn!(delay = ?config.reconnect_delay, "Reconnecting to conductor...");
        tokio::time::sleep(config.reconnect_delay).await;
    }
}

/// Build request envelope with ID
/// NOTE: Outer envelope uses "data" for binary payload, inner request uses "value" for Holochain 0.6+
fn build_envelope(id: u64, inner_bytes: &[u8]) -> Vec<u8> {
    use rmpv::encode::write_value;

    let envelope = Value::Map(vec![
        (Value::String("id".into()), Value::Integer(id.into())),
        (Value::String("type".into()), Value::String("request".into())),
        (Value::String("data".into()), Value::Binary(inner_bytes.to_vec())),
    ]);

    let mut bytes = Vec::new();
    let _ = write_value(&mut bytes, &envelope);
    bytes
}

/// Parse response envelope and extract result
fn parse_response(data: &[u8]) -> Option<(u64, Result<Vec<u8>, StorageError>)> {
    use rmpv::decode::read_value;

    let mut cursor = Cursor::new(data);
    let value = read_value(&mut cursor).ok()?;

    let map = value.as_map()?;

    // Extract ID
    let id = map.iter()
        .find(|(k, _)| k.as_str() == Some("id"))?
        .1.as_u64()?;

    // Extract type
    let resp_type = map.iter()
        .find(|(k, _)| k.as_str() == Some("type"))?
        .1.as_str()?;

    // Extract data
    let data = map.iter()
        .find(|(k, _)| k.as_str() == Some("data"))?
        .1.clone();

    if resp_type == "response" {
        // Success - data is the zome call result
        if let Some(bytes) = data.as_slice() {
            Some((id, Ok(bytes.to_vec())))
        } else {
            Some((id, Ok(Vec::new())))
        }
    } else if resp_type == "error" {
        // Error
        let msg = data.as_str().unwrap_or("Unknown error");
        Some((id, Err(StorageError::Conductor(msg.to_string()))))
    } else {
        None
    }
}

/// Connect to conductor with proper headers
async fn connect_to_conductor(
    url: &str,
) -> Result<(
    futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
), StorageError> {
    let request = Request::builder()
        .uri(url)
        .header("Host", url.split("//").last().unwrap_or("localhost"))
        .header("Origin", "http://localhost")
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header(
            "Sec-WebSocket-Key",
            tokio_tungstenite::tungstenite::handshake::client::generate_key(),
        )
        .body(())
        .map_err(|e| StorageError::Conductor(format!("Failed to build request: {}", e)))?;

    let (ws, _) = connect_async_with_config(request, None, false)
        .await
        .map_err(|e| StorageError::Conductor(format!("WebSocket connect failed: {}", e)))?;

    Ok(ws.split())
}

/// Get auth token from admin interface (Holochain 0.6+ format)
async fn get_auth_token(admin_url: &str, app_id: &str) -> Result<Vec<u8>, StorageError> {
    use rmpv::{decode::read_value, encode::write_value};
    use std::io::Cursor;

    debug!(admin_url = %admin_url, app_id = %app_id, "Getting auth token from admin interface");

    let (mut sink, mut stream) = connect_to_conductor(admin_url).await?;
    debug!("Connected to admin interface for auth token");

    // Build issue_app_authentication_token request (Holochain 0.6+ format)
    // Type name has underscores: "issue_app_authentication_token"
    let data = Value::Map(vec![
        (Value::String("installed_app_id".into()), Value::String(app_id.into())),
        (Value::String("expiry_seconds".into()), Value::Integer(3600.into())),
        (Value::String("single_use".into()), Value::Boolean(false)),
    ]);

    let inner = Value::Map(vec![
        (Value::String("type".into()), Value::String("issue_app_authentication_token".into())),
        (Value::String("value".into()), data),
    ]);

    let mut inner_buf = Vec::new();
    write_value(&mut inner_buf, &inner)
        .map_err(|e| StorageError::Internal(format!("Failed to encode inner request: {}", e)))?;

    // NOTE: Outer envelope uses "data" for binary payload, but inner structure uses "value"
    let envelope = Value::Map(vec![
        (Value::String("id".into()), Value::Integer(1.into())),
        (Value::String("type".into()), Value::String("request".into())),
        (Value::String("data".into()), Value::Binary(inner_buf)),
    ]);

    let mut envelope_buf = Vec::new();
    write_value(&mut envelope_buf, &envelope)
        .map_err(|e| StorageError::Internal(format!("Failed to encode envelope: {}", e)))?;

    sink.send(Message::Binary(envelope_buf)).await
        .map_err(|e| StorageError::Conductor(format!("Failed to send auth request: {}", e)))?;

    // Wait for response
    let timeout_result = timeout(Duration::from_secs(10), async {
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    return extract_token_from_response(&data);
                }
                Ok(Message::Close(_)) => {
                    return Err(StorageError::Conductor("Connection closed".into()));
                }
                Ok(_) => continue,
                Err(e) => {
                    return Err(StorageError::Conductor(format!("WebSocket error: {}", e)));
                }
            }
        }
        Err(StorageError::Conductor("Stream ended without response".into()))
    }).await;

    match timeout_result {
        Ok(result) => result,
        Err(_) => Err(StorageError::Conductor("Auth token request timeout".into())),
    }
}

/// Extract token from admin response
fn extract_token_from_response(data: &[u8]) -> Result<Vec<u8>, StorageError> {
    use rmpv::decode::read_value;
    use std::io::Cursor;

    let mut cursor = Cursor::new(data);
    let value = read_value(&mut cursor)
        .map_err(|e| StorageError::Internal(format!("Failed to decode response: {}", e)))?;

    // Navigate: envelope.data -> inner.value -> {token, expires_at}
    // NOTE: Response envelope uses "data" field (not "value")
    let envelope = value.as_map()
        .ok_or_else(|| StorageError::Internal("Response is not a map".into()))?;

    // Get the 'data' field from envelope (response uses "data" not "value")
    let envelope_value = envelope.iter()
        .find(|(k, _)| k.as_str() == Some("data"))
        .map(|(_, v)| v)
        .ok_or_else(|| StorageError::Internal("No data in envelope".into()))?;

    // Parse inner response from binary
    let inner_bytes = envelope_value.as_slice()
        .ok_or_else(|| StorageError::Internal("Envelope value is not binary".into()))?;

    let mut inner_cursor = Cursor::new(inner_bytes);
    let inner_value = read_value(&mut inner_cursor)
        .map_err(|e| StorageError::Internal(format!("Failed to decode inner response: {}", e)))?;

    // Inner is {type, value} where value contains {token, expires_at}
    let inner_map = inner_value.as_map()
        .ok_or_else(|| StorageError::Internal("Inner response is not a map".into()))?;

    let token_struct = inner_map.iter()
        .find(|(k, _)| k.as_str() == Some("value"))
        .map(|(_, v)| v)
        .ok_or_else(|| StorageError::Internal("No value in inner response".into()))?;

    // Get token field from value (map with token, expires_at)
    let token_map = token_struct.as_map()
        .ok_or_else(|| StorageError::Internal("Token struct is not a map".into()))?;

    let token_value = token_map.iter()
        .find(|(k, _)| k.as_str() == Some("token"))
        .map(|(_, v)| v)
        .ok_or_else(|| StorageError::Internal("No token field".into()))?;

    // Token might be binary or array of integers
    match token_value {
        Value::Binary(bytes) => Ok(bytes.clone()),
        Value::Array(arr) => {
            // Convert array of integers to bytes
            arr.iter()
                .map(|v| match v {
                    Value::Integer(i) => i.as_u64().map(|n| n as u8).ok_or("Invalid byte"),
                    _ => Err("Not an integer"),
                })
                .collect::<Result<Vec<u8>, _>>()
                .map_err(|e| StorageError::Internal(format!("Failed to convert token array: {}", e)))
        }
        _ => Err(StorageError::Internal(format!("Unexpected token type: {:?}", token_value))),
    }
}

/// Authenticate on app interface connection
async fn authenticate_connection<S, R>(
    sink: &mut S,
    stream: &mut R,
    token: &[u8],
) -> Result<(), StorageError>
where
    S: futures_util::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
    R: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    use rmpv::encode::write_value;

    debug!("Authenticating with conductor");

    // Build authenticate envelope (Holochain 0.6+ uses 'authenticate' type)
    let inner = Value::Map(vec![
        (Value::String("token".into()), Value::Binary(token.to_vec())),
    ]);

    let mut inner_buf = Vec::new();
    write_value(&mut inner_buf, &inner)
        .map_err(|e| StorageError::Internal(format!("Failed to encode auth inner: {}", e)))?;

    let envelope = Value::Map(vec![
        (Value::String("type".into()), Value::String("authenticate".into())),
        (Value::String("data".into()), Value::Binary(inner_buf)),
    ]);

    let mut envelope_buf = Vec::new();
    write_value(&mut envelope_buf, &envelope)
        .map_err(|e| StorageError::Internal(format!("Failed to encode auth envelope: {}", e)))?;

    sink.send(Message::Binary(envelope_buf)).await
        .map_err(|e| StorageError::Conductor(format!("Failed to send auth: {}", e)))?;

    // Wait briefly - conductor doesn't send explicit auth response, just closes if invalid
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Check if connection is still open
    let check_result = timeout(Duration::from_millis(50), async {
        tokio::select! {
            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) => {
                        Err(StorageError::Conductor("Auth rejected - connection closed".into()))
                    }
                    Some(Err(e)) => {
                        Err(StorageError::Conductor(format!("WebSocket error during auth: {}", e)))
                    }
                    _ => Ok(()),
                }
            }
        }
    }).await;

    match check_result {
        Ok(result) => result,
        Err(_) => Ok(()), // Timeout means connection still open, auth succeeded
    }
}
