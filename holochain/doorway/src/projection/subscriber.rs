//! Signal Subscriber - connects to Holochain conductor for real-time signals
//!
//! Subscribes to the conductor's app WebSocket interface and receives
//! post_commit signals from the DNA, feeding them to the Projection Engine.

use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::sync::broadcast;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

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
    /// Conductor app WebSocket URL
    pub app_url: String,
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
            app_url: "ws://localhost:4445".to_string(),
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
            app_url: std::env::var("HOLOCHAIN_APP_URL")
                .unwrap_or_else(|_| "ws://localhost:4445".to_string()),
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

            info!("Connecting to conductor at {}", self.config.app_url);

            match self.connect_and_listen().await {
                Ok(()) => {
                    // Clean disconnect, reset counter
                    reconnect_attempts = 0;
                }
                Err(e) => {
                    error!("Conductor connection error: {}", e);
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
    async fn connect_and_listen(&self) -> Result<(), String> {
        let (ws_stream, _) = connect_async(&self.config.app_url)
            .await
            .map_err(|e| format!("WebSocket connect failed: {}", e))?;

        info!("Connected to conductor app interface");

        let (mut write, mut read) = ws_stream.split();
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
        assert_eq!(config.app_url, "ws://localhost:4445");
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
