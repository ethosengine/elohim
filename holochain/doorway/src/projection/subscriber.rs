//! Signal Subscriber - connects to Holochain conductor for real-time signals
//!
//! Subscribes to the conductor's app WebSocket interface and receives
//! post_commit signals from the DNA, feeding them to the Projection Engine.
//!
//! ## Authentication Flow (Holochain 0.3+)
//!
//! 1. Connect to admin interface
//! 2. Request AppAuthenticationToken for the installed app
//! 3. Connect to app interface
//! 4. Send AppAuthenticationRequest with token
//! 5. Receive signals

use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::sync::broadcast;
use tokio::time::sleep;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{http::Request, Message},
};
use tracing::{debug, error, info, warn};

use super::app_auth::{issue_app_token, AppAuthToken};
use super::engine::ProjectionSignal;

/// Holochain WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HolochainMessage {
    /// Signal from a zome
    Signal {
        cell_id: Vec<Vec<u8>>,
        signal: JsonValue,
    },
    /// Response to a request
    Response {
        id: u64,
        data: JsonValue,
    },
}

/// Signal from Holochain conductor (app interface format)
#[derive(Debug, Clone, Deserialize)]
pub struct AppSignal {
    /// The cell that emitted the signal
    #[serde(default)]
    pub cell_id: Option<Vec<Vec<u8>>>,
    /// The signal payload (our ProjectionSignal)
    pub signal: JsonValue,
}

/// Subscriber configuration
#[derive(Debug, Clone)]
pub struct SubscriberConfig {
    /// Conductor admin WebSocket URL (for requesting auth tokens)
    pub admin_url: String,
    /// Conductor app WebSocket URL (for receiving signals)
    pub app_url: String,
    /// Installed app ID to authenticate as
    pub installed_app_id: String,
    /// Token expiry in seconds (0 = no expiry)
    pub token_expiry_seconds: u64,
    /// Reconnection delay on disconnect
    pub reconnect_delay: Duration,
    /// Maximum reconnection attempts (0 = infinite)
    pub max_reconnect_attempts: u32,
    /// Ping interval for keepalive
    pub ping_interval: Duration,
}

impl Default for SubscriberConfig {
    fn default() -> Self {
        Self {
            admin_url: "ws://localhost:4444".to_string(),
            app_url: "ws://localhost:4445".to_string(),
            installed_app_id: "elohim".to_string(),
            token_expiry_seconds: 0, // No expiry - we'll refresh on reconnect
            reconnect_delay: Duration::from_secs(5),
            max_reconnect_attempts: 0, // Infinite
            ping_interval: Duration::from_secs(30),
        }
    }
}

impl SubscriberConfig {
    /// Create config from environment
    pub fn from_env() -> Self {
        Self {
            admin_url: std::env::var("HOLOCHAIN_ADMIN_URL")
                .unwrap_or_else(|_| "ws://localhost:4444".to_string()),
            app_url: std::env::var("HOLOCHAIN_APP_URL")
                .unwrap_or_else(|_| "ws://localhost:4445".to_string()),
            installed_app_id: std::env::var("HOLOCHAIN_APP_ID")
                .unwrap_or_else(|_| "elohim".to_string()),
            token_expiry_seconds: std::env::var("HOLOCHAIN_TOKEN_EXPIRY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            reconnect_delay: Duration::from_secs(
                std::env::var("PROJECTION_RECONNECT_DELAY")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(5),
            ),
            max_reconnect_attempts: std::env::var("PROJECTION_MAX_RECONNECT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            ping_interval: Duration::from_secs(30),
        }
    }
}

/// Signal Subscriber - receives signals from Holochain conductor
pub struct SignalSubscriber {
    config: SubscriberConfig,
    /// Channel to send parsed signals to the engine
    signal_tx: broadcast::Sender<ProjectionSignal>,
    /// Shutdown signal
    shutdown_tx: broadcast::Sender<()>,
}

impl SignalSubscriber {
    /// Create a new signal subscriber
    pub fn new(config: SubscriberConfig) -> Self {
        let (signal_tx, _) = broadcast::channel(1000);
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            config,
            signal_tx,
            shutdown_tx,
        }
    }

    /// Get a receiver for projection signals
    pub fn subscribe(&self) -> broadcast::Receiver<ProjectionSignal> {
        self.signal_tx.subscribe()
    }

    /// Get a shutdown receiver
    pub fn shutdown_receiver(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Signal shutdown
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }

    /// Start the subscriber (blocking)
    pub async fn run(&self) {
        let mut reconnect_attempts = 0u32;
        let mut shutdown_rx = self.shutdown_receiver();

        loop {
            // Check for shutdown
            if shutdown_rx.try_recv().is_ok() {
                info!("Signal subscriber shutting down");
                break;
            }

            // Step 1: Get auth token from admin interface
            info!(
                "Requesting app auth token for '{}' from {}",
                self.config.installed_app_id, self.config.admin_url
            );

            let token = match issue_app_token(
                &self.config.admin_url,
                &self.config.installed_app_id,
                self.config.token_expiry_seconds,
            )
            .await
            {
                Ok(t) => {
                    info!("Obtained app authentication token");
                    t
                }
                Err(e) => {
                    error!("Failed to get app auth token: {}", e);
                    reconnect_attempts += 1;

                    if self.config.max_reconnect_attempts > 0
                        && reconnect_attempts >= self.config.max_reconnect_attempts
                    {
                        error!("Max reconnection attempts reached, stopping subscriber");
                        break;
                    }

                    info!(
                        "Retrying in {:?} (attempt {})",
                        self.config.reconnect_delay, reconnect_attempts
                    );
                    sleep(self.config.reconnect_delay).await;
                    continue;
                }
            };

            // Step 2: Connect to app interface and authenticate
            info!("Connecting to app interface at {}", self.config.app_url);

            match self.connect_and_listen(&token).await {
                Ok(()) => {
                    // Clean disconnect, reset counter
                    reconnect_attempts = 0;
                }
                Err(e) => {
                    error!("App interface error: {}", e);
                    reconnect_attempts += 1;

                    if self.config.max_reconnect_attempts > 0
                        && reconnect_attempts >= self.config.max_reconnect_attempts
                    {
                        error!(
                            "Max reconnection attempts ({}) reached, stopping subscriber",
                            self.config.max_reconnect_attempts
                        );
                        break;
                    }
                }
            }

            // Wait before reconnecting
            info!(
                "Reconnecting in {:?} (attempt {})",
                self.config.reconnect_delay, reconnect_attempts
            );

            tokio::select! {
                _ = sleep(self.config.reconnect_delay) => {}
                _ = shutdown_rx.recv() => {
                    info!("Shutdown received during reconnect wait");
                    break;
                }
            }
        }

        info!("Signal subscriber stopped");
    }

    /// Connect to conductor and listen for signals
    async fn connect_and_listen(&self, token: &AppAuthToken) -> Result<(), String> {
        // Build request with proper headers (like conductor.rs)
        // This ensures the app interface accepts the connection
        let host = self
            .config
            .app_url
            .split("//")
            .last()
            .unwrap_or("localhost:4445");

        let request = Request::builder()
            .uri(&self.config.app_url)
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
            .map_err(|e| format!("Failed to build request: {}", e))?;

        let (ws_stream, _) = connect_async_with_config(request, None, false)
            .await
            .map_err(|e| format!("WebSocket connect failed: {}", e))?;

        debug!("WebSocket connected, sending authentication...");

        let (mut write, mut read) = ws_stream.split();

        // Step 3: Send AppAuthenticationRequest
        self.send_auth_request(&mut write, token).await?;

        // Step 4: Wait for auth response
        self.wait_for_auth_response(&mut read).await?;

        info!("Connected and authenticated to app interface");

        // Continue with the same split streams
        let mut shutdown_rx = self.shutdown_receiver();
        let mut ping_interval = tokio::time::interval(self.config.ping_interval);

        loop {
            tokio::select! {
                // Check for shutdown
                _ = shutdown_rx.recv() => {
                    info!("Shutdown signal received, closing connection");
                    let _ = write.close().await;
                    return Ok(());
                }

                // Send periodic ping
                _ = ping_interval.tick() => {
                    if let Err(e) = write.send(Message::Ping(vec![])).await {
                        return Err(format!("Ping failed: {}", e));
                    }
                }

                // Receive messages
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            self.handle_message(&text);
                        }
                        Some(Ok(Message::Binary(data))) => {
                            // Holochain often sends msgpack, try to decode
                            self.handle_binary(&data);
                        }
                        Some(Ok(Message::Pong(_))) => {
                            debug!("Received pong");
                        }
                        Some(Ok(Message::Close(_))) => {
                            info!("Conductor closed connection");
                            return Ok(());
                        }
                        Some(Err(e)) => {
                            return Err(format!("WebSocket error: {}", e));
                        }
                        None => {
                            return Err("WebSocket stream ended".to_string());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Send AppAuthenticationRequest to the app interface
    async fn send_auth_request<S>(
        &self,
        write: &mut futures_util::stream::SplitSink<S, Message>,
        token: &AppAuthToken,
    ) -> Result<(), String>
    where
        S: futures_util::Sink<Message> + Unpin,
        <S as futures_util::Sink<Message>>::Error: std::fmt::Display,
    {
        // Build AppAuthenticationRequest message
        // Format: { token: <bytes> } wrapped in request envelope
        let inner = rmpv::Value::Map(vec![(
            rmpv::Value::String("token".into()),
            rmpv::Value::Binary(token.token.clone()),
        )]);

        let mut inner_buf = Vec::new();
        rmpv::encode::write_value(&mut inner_buf, &inner)
            .map_err(|e| format!("Failed to encode auth request: {}", e))?;

        // Wrap in request envelope with id=0
        let envelope = rmpv::Value::Map(vec![
            (
                rmpv::Value::String("id".into()),
                rmpv::Value::Integer(0.into()),
            ),
            (
                rmpv::Value::String("type".into()),
                rmpv::Value::String("request".into()),
            ),
            (
                rmpv::Value::String("data".into()),
                rmpv::Value::Binary(inner_buf),
            ),
        ]);

        let mut buf = Vec::new();
        rmpv::encode::write_value(&mut buf, &envelope)
            .map_err(|e| format!("Failed to encode envelope: {}", e))?;

        write
            .send(Message::Binary(buf))
            .await
            .map_err(|e| format!("Failed to send auth request: {}", e))?;

        debug!("Sent AppAuthenticationRequest");
        Ok(())
    }

    /// Wait for authentication response
    async fn wait_for_auth_response<S>(
        &self,
        read: &mut futures_util::stream::SplitStream<S>,
    ) -> Result<(), String>
    where
        S: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
            + Unpin,
    {
        let response = tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Binary(data)) => {
                        // Parse response to check for errors
                        if let Ok(value) = rmpv::decode::read_value(&mut std::io::Cursor::new(&data))
                        {
                            if let rmpv::Value::Map(ref map) = value {
                                // Check for error response
                                for (k, v) in map {
                                    if let rmpv::Value::String(key) = k {
                                        if key.as_str() == Some("type") {
                                            if let rmpv::Value::String(val) = v {
                                                if val.as_str() == Some("error") {
                                                    return Err("Authentication rejected".to_string());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // Any non-error response means success
                        return Ok(());
                    }
                    Ok(Message::Close(_)) => {
                        return Err("Connection closed during auth".to_string());
                    }
                    Err(e) => {
                        return Err(format!("WebSocket error: {}", e));
                    }
                    _ => continue,
                }
            }
            Err("No auth response received".to_string())
        })
        .await
        .map_err(|_| "Timeout waiting for auth response".to_string())??;

        Ok(response)
    }

    /// Handle a text message from conductor
    fn handle_message(&self, text: &str) {
        // Try to parse as JSON signal
        match serde_json::from_str::<JsonValue>(text) {
            Ok(value) => {
                self.process_signal_value(&value);
            }
            Err(e) => {
                debug!("Failed to parse message as JSON: {}", e);
            }
        }
    }

    /// Handle a binary message from conductor (msgpack)
    fn handle_binary(&self, data: &[u8]) {
        // Try to decode msgpack
        match rmp_serde::from_slice::<JsonValue>(data) {
            Ok(value) => {
                self.process_signal_value(&value);
            }
            Err(e) => {
                debug!("Failed to decode msgpack: {}", e);
                // Try JSON as fallback
                if let Ok(text) = std::str::from_utf8(data) {
                    self.handle_message(text);
                }
            }
        }
    }

    /// Process a signal value from the conductor
    fn process_signal_value(&self, value: &JsonValue) {
        // Holochain signals come in different formats depending on version
        // Try to extract the actual signal payload

        // Format 1: Direct signal object
        if let Ok(signal) = serde_json::from_value::<ProjectionSignal>(value.clone()) {
            self.emit_signal(signal);
            return;
        }

        // Format 2: Wrapped in { "signal": ... }
        if let Some(signal_data) = value.get("signal") {
            if let Ok(signal) = serde_json::from_value::<ProjectionSignal>(signal_data.clone()) {
                self.emit_signal(signal);
                return;
            }

            // Format 3: Double-wrapped { "signal": { "payload": ... } }
            if let Some(payload) = signal_data.get("payload") {
                if let Ok(signal) = serde_json::from_value::<ProjectionSignal>(payload.clone()) {
                    self.emit_signal(signal);
                    return;
                }
            }
        }

        // Format 4: App signal wrapper { "type": "Signal", "data": ... }
        if value.get("type").and_then(|t| t.as_str()) == Some("Signal") {
            if let Some(data) = value.get("data") {
                self.process_signal_value(data);
                return;
            }
        }

        // Format 5: Holochain client signal format
        if let Some(app) = value.get("App") {
            self.process_signal_value(app);
            return;
        }

        debug!("Unrecognized signal format: {:?}", value);
    }

    /// Emit a parsed signal to the engine
    fn emit_signal(&self, signal: ProjectionSignal) {
        match &signal {
            ProjectionSignal::ContentCommitted { content, .. } => {
                info!("Received content signal: {}", content.title);
            }
            ProjectionSignal::PathCommitted { path, .. } => {
                info!("Received path signal: {}", path.title);
            }
            ProjectionSignal::StepCommitted { step, .. } => {
                debug!("Received step signal: {}", step.id);
            }
            ProjectionSignal::ChapterCommitted { chapter, .. } => {
                debug!("Received chapter signal: {}", chapter.id);
            }
            ProjectionSignal::RelationshipCommitted { relationship, .. } => {
                debug!("Received relationship signal: {}", relationship.id);
            }
            _ => {
                debug!("Received signal: {:?}", std::mem::discriminant(&signal));
            }
        }

        if let Err(e) = self.signal_tx.send(signal) {
            warn!("Failed to send signal to engine (no receivers?): {}", e);
        }
    }
}

/// Spawn the signal subscriber as a background task
pub fn spawn_subscriber(
    config: SubscriberConfig,
) -> (Arc<SignalSubscriber>, tokio::task::JoinHandle<()>) {
    let subscriber = Arc::new(SignalSubscriber::new(config));
    let subscriber_clone = subscriber.clone();

    let handle = tokio::spawn(async move {
        subscriber_clone.run().await;
    });

    (subscriber, handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = SubscriberConfig::default();
        assert_eq!(config.admin_url, "ws://localhost:4444");
        assert_eq!(config.app_url, "ws://localhost:4445");
        assert_eq!(config.installed_app_id, "elohim");
        assert_eq!(config.reconnect_delay, Duration::from_secs(5));
    }

    #[test]
    fn test_signal_parsing() {
        let subscriber = SignalSubscriber::new(SubscriberConfig::default());

        // Test direct format
        let json = serde_json::json!({
            "type": "ContentCommitted",
            "payload": {
                "action_hash": "uhCkk...",
                "entry_hash": "uhCEk...",
                "content": {
                    "id": "test",
                    "content_type": "concept",
                    "title": "Test Content",
                    "description": "",
                    "summary": null,
                    "content": "",
                    "content_format": "markdown",
                    "tags": [],
                    "source_path": null,
                    "related_node_ids": [],
                    "author_id": null,
                    "reach": "public",
                    "trust_score": 1.0,
                    "estimated_minutes": null,
                    "thumbnail_url": null,
                    "metadata_json": "{}",
                    "created_at": "",
                    "updated_at": ""
                },
                "author": "uhCAk..."
            }
        });

        // Just test that parsing doesn't panic
        subscriber.process_signal_value(&json);
    }
}
