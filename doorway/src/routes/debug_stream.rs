//! Debug Stream - WebSocket endpoint for real-time debugging
//!
//! Provides a WebSocket stream of debug events from:
//! - Doorway (zome calls, routing, errors)
//! - elohim-storage (import processing, conductor client, cell discovery)
//!
//! ## Usage
//!
//! ```bash
//! # Connect via websocat
//! websocat 'wss://doorway-dev.elohim.host/debug/stream?apiKey=...'
//!
//! # Or use the seeder debug command
//! npm run debug:stream
//! ```
//!
//! ## Event Types
//!
//! - `doorway:zome_call` - Zome call request/response
//! - `doorway:import` - Import request forwarded
//! - `storage:import` - Import processing in elohim-storage
//! - `storage:conductor` - Conductor client events
//! - `storage:cell` - Cell discovery events
//! - `error` - Error events from any source

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::{Request, Response, StatusCode};
use hyper::body::Incoming;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

/// Debug event that can be streamed to clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugEvent {
    /// Event timestamp (ISO 8601)
    pub timestamp: String,
    /// Event source: "doorway", "storage", "conductor"
    pub source: String,
    /// Event type: "zome_call", "import", "cell_discovery", "error", etc.
    pub event_type: String,
    /// Log level: "debug", "info", "warn", "error"
    pub level: String,
    /// Human-readable message
    pub message: String,
    /// Optional structured data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl DebugEvent {
    pub fn new(source: &str, event_type: &str, level: &str, message: &str) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            source: source.to_string(),
            event_type: event_type.to_string(),
            level: level.to_string(),
            message: message.to_string(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn info(source: &str, event_type: &str, message: &str) -> Self {
        Self::new(source, event_type, "info", message)
    }

    pub fn warn(source: &str, event_type: &str, message: &str) -> Self {
        Self::new(source, event_type, "warn", message)
    }

    pub fn error(source: &str, event_type: &str, message: &str) -> Self {
        Self::new(source, event_type, "error", message)
    }

    pub fn debug(source: &str, event_type: &str, message: &str) -> Self {
        Self::new(source, event_type, "debug", message)
    }
}

/// Debug event hub - broadcasts events to all connected clients
pub struct DebugHub {
    /// Broadcast channel for events
    tx: broadcast::Sender<DebugEvent>,
    /// Whether debug streaming is enabled
    enabled: bool,
}

impl DebugHub {
    pub fn new(enabled: bool) -> Self {
        let (tx, _) = broadcast::channel(1000);
        Self { tx, enabled }
    }

    /// Emit a debug event to all connected clients
    pub fn emit(&self, event: DebugEvent) {
        if self.enabled {
            let _ = self.tx.send(event);
        }
    }

    /// Subscribe to debug events
    pub fn subscribe(&self) -> broadcast::Receiver<DebugEvent> {
        self.tx.subscribe()
    }

    /// Check if hub is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    // Convenience methods for common events

    pub fn zome_call_start(&self, zome: &str, fn_name: &str, cell_role: &str) {
        self.emit(DebugEvent::info("doorway", "zome_call_start",
            &format!("→ {}::{} (role: {})", zome, fn_name, cell_role)));
    }

    pub fn zome_call_complete(&self, zome: &str, fn_name: &str, duration_ms: u64) {
        self.emit(DebugEvent::info("doorway", "zome_call_complete",
            &format!("← {}::{} completed in {}ms", zome, fn_name, duration_ms)));
    }

    pub fn zome_call_error(&self, zome: &str, fn_name: &str, error: &str) {
        self.emit(DebugEvent::error("doorway", "zome_call_error",
            &format!("✗ {}::{} failed: {}", zome, fn_name, error)));
    }

    pub fn import_forwarded(&self, batch_type: &str, batch_id: &str) {
        self.emit(DebugEvent::info("doorway", "import_forward",
            &format!("→ Forwarding {} import: {}", batch_type, batch_id)));
    }

    pub fn storage_event(&self, event_type: &str, message: &str) {
        self.emit(DebugEvent::info("storage", event_type, message));
    }
}

impl Default for DebugHub {
    fn default() -> Self {
        Self::new(true)
    }
}

/// Handle WebSocket upgrade for debug stream
pub async fn handle_debug_stream(
    req: Request<Incoming>,
    debug_hub: Arc<DebugHub>,
    storage_url: Option<String>,
) -> Response<Full<Bytes>> {
    // Check if debug is enabled
    if !debug_hub.is_enabled() {
        return Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(r#"{"error": "Debug streaming not enabled"}"#)))
            .unwrap();
    }

    // Upgrade to WebSocket
    match hyper_tungstenite::upgrade(req, None) {
        Ok((response, ws_future)) => {
            // Spawn handler task
            tokio::spawn(async move {
                match ws_future.await {
                    Ok(ws_stream) => {
                        handle_debug_client(ws_stream, debug_hub, storage_url).await;
                    }
                    Err(e) => {
                        error!(error = %e, "WebSocket upgrade failed");
                    }
                }
            });

            // Return upgrade response
            let (parts, _) = response.into_parts();
            Response::from_parts(parts, Full::new(Bytes::new()))
        }
        Err(e) => {
            warn!(error = %e, "Failed to upgrade to WebSocket");
            Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(r#"{{"error": "WebSocket upgrade failed: {}"}}"#, e))))
                .unwrap()
        }
    }
}

/// Handle a connected debug client
async fn handle_debug_client(
    ws_stream: hyper_tungstenite::WebSocketStream<hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>>,
    debug_hub: Arc<DebugHub>,
    storage_url: Option<String>,
) {
    let (mut ws_write, mut ws_read) = ws_stream.split();

    // Subscribe to local debug events
    let mut local_rx = debug_hub.subscribe();

    // Send welcome message
    let welcome = DebugEvent::info("doorway", "connected", "Debug stream connected")
        .with_data(serde_json::json!({
            "storage_url": storage_url,
            "features": ["doorway_events", "storage_proxy"]
        }));

    if let Ok(json) = serde_json::to_string(&welcome) {
        let _ = ws_write.send(Message::Text(json.into())).await;
    }

    // Optionally connect to storage debug stream
    let storage_stream = if let Some(ref url) = storage_url {
        connect_storage_debug_stream(url).await
    } else {
        None
    };

    // Create merged stream
    let mut storage_rx = storage_stream.map(|(_, rx)| rx);

    loop {
        tokio::select! {
            // Local debug events
            event = local_rx.recv() => {
                match event {
                    Ok(evt) => {
                        if let Ok(json) = serde_json::to_string(&evt) {
                            if ws_write.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        let lag_event = DebugEvent::warn("doorway", "lag",
                            &format!("Dropped {} events due to slow client", n));
                        if let Ok(json) = serde_json::to_string(&lag_event) {
                            let _ = ws_write.send(Message::Text(json.into())).await;
                        }
                    }
                    Err(_) => break,
                }
            }

            // Storage debug events (if connected)
            Some(msg) = async {
                if let Some(ref mut rx) = storage_rx {
                    rx.recv().await
                } else {
                    // Type must match rx.recv().await which returns Option<Option<Result<...>>>
                    std::future::pending::<Option<Option<Result<Message, tokio_tungstenite::tungstenite::Error>>>>().await
                }
            } => {
                if let Some(Ok(Message::Text(text))) = msg {
                    // Parse and re-emit storage events
                    if let Ok(mut evt) = serde_json::from_str::<DebugEvent>(&text) {
                        evt.source = "storage".to_string();
                        if let Ok(json) = serde_json::to_string(&evt) {
                            if ws_write.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    } else {
                        // Forward raw message
                        if ws_write.send(Message::Text(text)).await.is_err() {
                            break;
                        }
                    }
                }
            }

            // Client messages (for commands)
            msg = ws_read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // Handle commands like {"command": "ping"}
                        if let Ok(cmd) = serde_json::from_str::<serde_json::Value>(&text) {
                            if cmd.get("command").and_then(|c| c.as_str()) == Some("ping") {
                                let pong = DebugEvent::debug("doorway", "pong", "pong");
                                if let Ok(json) = serde_json::to_string(&pong) {
                                    let _ = ws_write.send(Message::Text(json.into())).await;
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }

    debug!("Debug client disconnected");
}

/// Connect to elohim-storage debug stream
async fn connect_storage_debug_stream(
    storage_url: &str,
) -> Option<((), tokio::sync::mpsc::Receiver<Option<Result<Message, tokio_tungstenite::tungstenite::Error>>>)> {
    let ws_url = format!("{}/debug/stream", storage_url.replace("http://", "ws://").replace("https://", "wss://"));

    info!(url = %ws_url, "Connecting to storage debug stream");

    match tokio_tungstenite::connect_async(&ws_url).await {
        Ok((ws_stream, _)) => {
            let (_, mut read) = ws_stream.split();
            let (tx, rx) = tokio::sync::mpsc::channel(100);

            // Spawn task to forward messages
            tokio::spawn(async move {
                while let Some(msg) = read.next().await {
                    if tx.send(Some(msg)).await.is_err() {
                        break;
                    }
                }
            });

            Some(((), rx))
        }
        Err(e) => {
            warn!(error = %e, "Failed to connect to storage debug stream");
            None
        }
    }
}
