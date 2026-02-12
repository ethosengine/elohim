//! Conductor connection manager
//!
//! Maintains a persistent WebSocket connection to the Holochain conductor.
//! Handles reconnection and provides a thread-safe interface for sending requests.

use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use tokio::time::timeout;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{http::Request, protocol::Message},
};
use tracing::{debug, error, info, warn};

use crate::types::{DoorwayError, Result};

/// Conductor connection manager
pub struct ConductorConnection {
    /// URL of the conductor
    #[allow(dead_code)]
    conductor_url: String,
    /// Channel for sending messages to the conductor
    tx: mpsc::Sender<(Vec<u8>, oneshot::Sender<Vec<u8>>)>,
    /// Whether the connection is alive
    connected: Arc<RwLock<bool>>,
}

impl ConductorConnection {
    /// Create a new conductor connection (no authentication)
    pub async fn connect(conductor_url: &str) -> Result<Self> {
        Self::connect_with_auth(conductor_url, None).await
    }

    /// Create a new conductor connection with optional app authentication token.
    ///
    /// If `auth_token` is provided, the connection sends an `authenticate` message
    /// after each WebSocket connect (including reconnects), matching the Holochain 0.6
    /// app interface protocol used by elohim-storage.
    pub async fn connect_with_auth(
        conductor_url: &str,
        auth_token: Option<Vec<u8>>,
    ) -> Result<Self> {
        let (tx, rx) = mpsc::channel::<(Vec<u8>, oneshot::Sender<Vec<u8>>)>(1000);
        let connected = Arc::new(RwLock::new(false));

        let conn = Self {
            conductor_url: conductor_url.to_string(),
            tx,
            connected: Arc::clone(&connected),
        };

        // Start the connection manager task
        let url = conductor_url.to_string();
        let connected_flag = Arc::clone(&connected);
        tokio::spawn(async move {
            connection_loop(url, auth_token, rx, connected_flag).await;
        });

        // Wait for initial connection
        for _ in 0..50 {
            if *conn.connected.read().await {
                return Ok(conn);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err(DoorwayError::Holochain(
            "Timeout waiting for conductor connection".into(),
        ))
    }

    /// Send a request to the conductor and wait for response
    pub async fn request(&self, data: Vec<u8>, timeout_ms: u64) -> Result<Vec<u8>> {
        let (response_tx, response_rx) = oneshot::channel();

        self.tx
            .send((data, response_tx))
            .await
            .map_err(|_| DoorwayError::Holochain("Conductor connection closed".into()))?;

        match timeout(Duration::from_millis(timeout_ms), response_rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(DoorwayError::Holochain("Response channel closed".into())),
            Err(_) => Err(DoorwayError::Holochain("Request timeout".into())),
        }
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }
}

/// Main connection loop with reconnection logic
async fn connection_loop(
    conductor_url: String,
    auth_token: Option<Vec<u8>>,
    mut rx: mpsc::Receiver<(Vec<u8>, oneshot::Sender<Vec<u8>>)>,
    connected: Arc<RwLock<bool>>,
) {
    let mut reconnect_delay = Duration::from_millis(100);
    let max_reconnect_delay = Duration::from_secs(30);

    loop {
        info!("Connecting to conductor at {}", conductor_url);

        match connect_to_conductor(&conductor_url).await {
            Ok((mut ws_sink, ws_stream)) => {
                // Authenticate if token provided (Holochain 0.6 app interface)
                if let Some(ref token) = auth_token {
                    match send_authenticate(&mut ws_sink, token).await {
                        Ok(()) => {
                            debug!("Authenticated with conductor");
                        }
                        Err(e) => {
                            error!("Failed to authenticate with conductor: {}", e);
                            *connected.write().await = false;
                            warn!("Reconnecting to conductor in {:?}...", reconnect_delay);
                            tokio::time::sleep(reconnect_delay).await;
                            reconnect_delay = (reconnect_delay * 2).min(max_reconnect_delay);
                            continue;
                        }
                    }
                }

                *connected.write().await = true;
                reconnect_delay = Duration::from_millis(100);
                info!("Connected to conductor");

                // Run the message handling loop
                if let Err(e) = handle_messages(ws_sink, ws_stream, &mut rx).await {
                    error!("Conductor connection error: {}", e);
                }

                *connected.write().await = false;
            }
            Err(e) => {
                error!("Failed to connect to conductor: {}", e);
            }
        }

        // Wait before reconnecting
        warn!("Reconnecting to conductor in {:?}...", reconnect_delay);
        tokio::time::sleep(reconnect_delay).await;
        reconnect_delay = (reconnect_delay * 2).min(max_reconnect_delay);
    }
}

/// Send authenticate message after WebSocket connect.
///
/// Holochain 0.6 app interface format: { type: "authenticate", data: <binary {token: <bytes>}> }
async fn send_authenticate(
    ws_sink: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    token: &[u8],
) -> Result<()> {
    let inner = rmpv::Value::Map(vec![(
        rmpv::Value::String("token".into()),
        rmpv::Value::Binary(token.to_vec()),
    )]);

    let mut inner_buf = Vec::new();
    rmpv::encode::write_value(&mut inner_buf, &inner)
        .map_err(|e| DoorwayError::Holochain(format!("Failed to encode auth: {}", e)))?;

    let envelope = rmpv::Value::Map(vec![
        (
            rmpv::Value::String("type".into()),
            rmpv::Value::String("authenticate".into()),
        ),
        (
            rmpv::Value::String("data".into()),
            rmpv::Value::Binary(inner_buf),
        ),
    ]);

    let mut buf = Vec::new();
    rmpv::encode::write_value(&mut buf, &envelope)
        .map_err(|e| DoorwayError::Holochain(format!("Failed to encode auth envelope: {}", e)))?;

    ws_sink
        .send(Message::Binary(buf))
        .await
        .map_err(|e| DoorwayError::Holochain(format!("Failed to send auth: {}", e)))?;

    // Brief pause — if conductor rejects, it closes the connection
    tokio::time::sleep(Duration::from_millis(50)).await;

    Ok(())
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
)> {
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
        .map_err(|e| DoorwayError::Holochain(format!("Failed to build request: {}", e)))?;

    let (ws, _) = connect_async_with_config(request, None, false)
        .await
        .map_err(|e| DoorwayError::Holochain(format!("WebSocket connect failed: {}", e)))?;

    Ok(ws.split())
}

/// Handle messages between request channel and conductor WebSocket
async fn handle_messages(
    ws_sink: futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    mut ws_stream: futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    rx: &mut mpsc::Receiver<(Vec<u8>, oneshot::Sender<Vec<u8>>)>,
) -> Result<()> {
    // Pending responses indexed by request ID
    // Holochain uses MessagePack with request IDs embedded in messages
    // For now, we use a simple queue since Holochain responses are ordered
    let pending: Arc<Mutex<Vec<oneshot::Sender<Vec<u8>>>>> = Arc::new(Mutex::new(Vec::new()));
    let pending_for_send = Arc::clone(&pending);

    // Wrap sink in Arc<Mutex> for sharing
    let ws_sink = Arc::new(Mutex::new(ws_sink));
    let ws_sink_for_rx = Arc::clone(&ws_sink);

    // Task to handle incoming requests
    let request_handler = async {
        while let Some((data, response_tx)) = rx.recv().await {
            // Queue the response handler
            {
                let mut pending = pending_for_send.lock().await;
                pending.push(response_tx);
            }

            // Send to conductor
            let mut sink = ws_sink_for_rx.lock().await;
            if let Err(e) = sink.send(Message::Binary(data)).await {
                error!("Failed to send to conductor: {}", e);
                // Remove the pending response
                let mut pending = pending_for_send.lock().await;
                pending.pop();
                break;
            }
        }
    };

    // Task to handle responses from conductor
    let response_handler = async {
        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    // Get the next pending response handler
                    let maybe_sender = {
                        let mut pending = pending.lock().await;
                        if !pending.is_empty() {
                            Some(pending.remove(0))
                        } else {
                            None
                        }
                    };

                    if let Some(sender) = maybe_sender {
                        let _ = sender.send(data.to_vec());
                    } else {
                        warn!("Received response with no pending request");
                    }
                }
                Ok(Message::Ping(data)) => {
                    let mut sink = ws_sink.lock().await;
                    let _ = sink.send(Message::Pong(data)).await;
                }
                Ok(Message::Close(frame)) => {
                    info!("Conductor closed connection: {:?}", frame);
                    break;
                }
                Err(e) => {
                    error!("Conductor WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    };

    // Run both handlers concurrently
    tokio::select! {
        _ = request_handler => {
            debug!("Request handler ended");
        }
        _ = response_handler => {
            debug!("Response handler ended");
        }
    }

    Ok(())
}
