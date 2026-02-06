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
use serde_json::{json, Value as JsonValue};
use tokio::sync::broadcast;
use tokio::time::sleep;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{http::Request, Message},
};
use tracing::{debug, error, info, warn};

use super::app_auth::{issue_app_token, AppAuthToken};
use super::engine::ProjectionSignal;

// =============================================================================
// CacheSignal Support - for warm_cache and doorway-client signals
// =============================================================================

/// CacheSignal from doorway-client crate
/// Used by warm_cache to pre-populate doorway cache
#[derive(Debug, Clone, Deserialize)]
pub struct CacheSignal {
    pub signal_type: CacheSignalType,
    pub doc_type: String,
    pub doc_id: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub ttl_secs: Option<u64>,
    #[serde(default)]
    pub public: bool,
    #[serde(default)]
    pub reach: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CacheSignalType {
    Upsert,
    Delete,
    Invalidate,
}

/// DoorwaySignal wrapper from doorway-client crate
#[derive(Debug, Clone, Deserialize)]
pub struct DoorwaySignal {
    pub namespace: String,
    pub payload: CacheSignal,
}

// =============================================================================
// InfrastructureSignal Support - for ContentServer endpoint registration
// =============================================================================

/// Storage endpoint from infrastructure DNA
#[derive(Debug, Clone, Deserialize)]
pub struct StorageEndpointData {
    pub url: String,
    pub protocol: String,
    pub priority: u8,
}

/// ContentServer data from infrastructure DNA
#[derive(Debug, Clone, Deserialize)]
pub struct ContentServerData {
    pub content_hash: String,
    pub capability: String,
    pub serve_url: Option<String>,
    #[serde(default)]
    pub endpoints: Vec<StorageEndpointData>,
    pub online: bool,
    pub priority: u8,
    pub region: Option<String>,
    pub bandwidth_mbps: Option<u32>,
    pub registered_at: u64,
    pub last_heartbeat: u64,
}

/// Infrastructure signals from infrastructure DNA
/// Used by doorway to register content server endpoints for fallback fetching
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum InfrastructureSignal {
    /// ContentServer was registered or updated
    ContentServerCommitted {
        action_hash: String,
        entry_hash: String,
        server: ContentServerData,
        author: String,
    },
    // Other infrastructure signals (doorway heartbeats, etc.) can be added here
    // but we only care about ContentServerCommitted for blob routing
}

impl CacheSignal {
    /// Convert CacheSignal to ProjectionSignal format
    ///
    /// For cache warming, we don't have action_hash/author from the original
    /// commit, so we use placeholder values. The data is still valid and
    /// comes from the authoritative Holochain DHT.
    pub fn to_projection_signal(self) -> ProjectionSignal {
        let action = match self.signal_type {
            CacheSignalType::Upsert => "commit".to_string(),
            CacheSignalType::Delete => "delete".to_string(),
            CacheSignalType::Invalidate => "invalidate".to_string(),
        };

        ProjectionSignal {
            doc_type: self.doc_type,
            action,
            id: self.doc_id,
            data: self.data.unwrap_or(serde_json::Value::Null),
            // Placeholder values for cache-only signals
            // These signals come from warm_cache which reads from DHT
            // but doesn't have the original action metadata
            action_hash: "cache-warm".to_string(),
            entry_hash: None,
            author: "cache-warm".to_string(),
            search_tokens: vec![],
            invalidates: vec![],
            ttl_secs: self.ttl_secs,
        }
    }
}

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

/// Content server registration event for blob cache
#[derive(Debug, Clone)]
pub struct ContentServerRegistration {
    pub content_hash: String,
    pub endpoints: Vec<String>,
    pub capability: String,
    pub priority: u8,
    pub region: Option<String>,
    pub online: bool,
}

/// Signal Subscriber - receives signals from Holochain conductor
pub struct SignalSubscriber {
    config: SubscriberConfig,
    /// Channel to send parsed signals to the engine (content projection)
    signal_tx: broadcast::Sender<ProjectionSignal>,
    /// Channel to send content server registrations (blob routing)
    blob_registry_tx: broadcast::Sender<ContentServerRegistration>,
    /// Shutdown signal
    shutdown_tx: broadcast::Sender<()>,
}

impl SignalSubscriber {
    /// Create a new signal subscriber
    pub fn new(config: SubscriberConfig) -> Self {
        let (signal_tx, _) = broadcast::channel(1000);
        let (blob_registry_tx, _) = broadcast::channel(1000);
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            config,
            signal_tx,
            blob_registry_tx,
            shutdown_tx,
        }
    }

    /// Get a receiver for projection signals (content metadata → MongoDB)
    pub fn subscribe(&self) -> broadcast::Receiver<ProjectionSignal> {
        self.signal_tx.subscribe()
    }

    /// Get a receiver for content server registrations (blob endpoints → TieredBlobCache)
    pub fn subscribe_blob_registry(&self) -> broadcast::Receiver<ContentServerRegistration> {
        self.blob_registry_tx.subscribe()
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

    /// Process a signal value from the conductor.
    ///
    /// Attempts to parse the generic ProjectionSignal format from various
    /// Holochain wrapper formats.
    fn process_signal_value(&self, value: &JsonValue) {
        // Format 1: Direct generic signal object
        if let Ok(signal) = serde_json::from_value::<ProjectionSignal>(value.clone()) {
            self.emit_signal(signal);
            return;
        }

        // Format 2: DoorwaySignal (from warm_cache / doorway-client)
        // { "namespace": "doorway", "payload": { CacheSignal } }
        if let Ok(doorway_signal) = serde_json::from_value::<DoorwaySignal>(value.clone()) {
            if doorway_signal.namespace == "doorway" {
                let signal = doorway_signal.payload.to_projection_signal();
                info!(
                    doc_type = signal.doc_type,
                    id = signal.id,
                    "Received cache signal from warm_cache"
                );
                self.emit_signal(signal);
                return;
            }
        }

        // Format 3: InfrastructureSignal (from infrastructure DNA)
        // Used for ContentServer endpoint registration - goes to blob cache, NOT MongoDB
        if let Ok(infra_signal) = serde_json::from_value::<InfrastructureSignal>(value.clone()) {
            self.process_infrastructure_signal(infra_signal);
            return;
        }

        // Format 5: Wrapped in { "signal": ... }
        if let Some(signal_data) = value.get("signal") {
            self.process_signal_value(signal_data);
            return;
        }

        // Format 6: App signal wrapper { "type": "Signal", "data": ... }
        if value.get("type").and_then(|t| t.as_str()) == Some("Signal") {
            if let Some(data) = value.get("data") {
                self.process_signal_value(data);
                return;
            }
        }

        // Format 7: Holochain client format { "App": ... }
        if let Some(app) = value.get("App") {
            self.process_signal_value(app);
            return;
        }

        debug!("Unrecognized signal format: {:?}", value);
    }

    /// Process an infrastructure signal (ContentServer registrations, etc.)
    ///
    /// ContentServerCommitted signals now go to ProjectionStore via signal_tx,
    /// updating the blob_endpoints field on documents with matching blob_hash.
    /// This is part of the metadata layer merge (ProjectionStore is single source
    /// of truth for blob metadata, TieredBlobCache is bytes-only).
    fn process_infrastructure_signal(&self, signal: InfrastructureSignal) {
        match signal {
            InfrastructureSignal::ContentServerCommitted { server, author, action_hash, .. } => {
                // Build endpoint URLs from endpoints list (preferred) or serve_url (deprecated)
                let mut endpoint_urls: Vec<String> = server
                    .endpoints
                    .iter()
                    .filter(|e| e.protocol == "http" || e.protocol == "https")
                    .map(|e| e.url.clone())
                    .collect();

                // Fallback: add serve_url if endpoints is empty
                if endpoint_urls.is_empty() {
                    if let Some(url) = server.serve_url {
                        endpoint_urls.push(url);
                    }
                }

                // Only process if we have endpoints and server is online
                if !endpoint_urls.is_empty() && server.online {
                    // Clone content_hash before it's moved
                    let content_hash = server.content_hash.clone();

                    info!(
                        content_hash = %content_hash,
                        endpoints = ?endpoint_urls,
                        capability = %server.capability,
                        "ContentServer registered - updating projection store"
                    );

                    // Convert to ProjectionSignal with "update_endpoints" action
                    // This routes through the normal signal channel to ProjectionEngine,
                    // which updates blob_endpoints on documents with matching blob_hash
                    let signal = ProjectionSignal {
                        doc_type: "BlobEndpoint".to_string(), // Metadata only, not stored as doc
                        action: "update_endpoints".to_string(),
                        id: content_hash.clone(),             // blob_hash
                        data: json!(endpoint_urls.clone()), // Array of endpoint URLs
                        action_hash,
                        entry_hash: None,
                        author,
                        search_tokens: vec![],
                        invalidates: vec![],
                        ttl_secs: None,
                    };

                    if let Err(e) = self.signal_tx.send(signal) {
                        warn!("Failed to send endpoint update signal (no receivers?): {}", e);
                    }

                    // Also send to blob_registry_tx for backwards compatibility
                    // (in case any consumers still use it)
                    let registration = ContentServerRegistration {
                        content_hash,
                        endpoints: endpoint_urls,
                        capability: server.capability,
                        priority: server.priority,
                        region: server.region,
                        online: server.online,
                    };

                    // Ignore errors - old consumers may not exist
                    let _ = self.blob_registry_tx.send(registration);
                } else {
                    debug!(
                        content_hash = %server.content_hash,
                        online = server.online,
                        "Ignoring ContentServer (offline or no endpoints)"
                    );
                }
            }
        }
    }

    /// Emit a parsed signal to the engine.
    ///
    /// Type-agnostic logging - just logs doc_type, action, id.
    fn emit_signal(&self, signal: ProjectionSignal) {
        info!(
            doc_type = signal.doc_type,
            action = signal.action,
            id = signal.id,
            "Received projection signal"
        );

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

        // Test generic signal format (new format)
        let json = serde_json::json!({
            "doc_type": "Content",
            "action": "commit",
            "id": "test-content",
            "data": {
                "title": "Test Content",
                "description": "A test document"
            },
            "action_hash": "uhCkk...",
            "entry_hash": "uhCEk...",
            "author": "uhCAk...",
            "search_tokens": ["test", "content"]
        });

        // Just test that parsing doesn't panic
        subscriber.process_signal_value(&json);
    }

    #[test]
    fn test_wrapped_signal_parsing() {
        let subscriber = SignalSubscriber::new(SubscriberConfig::default());

        // Test wrapped format { "signal": ... }
        let json = serde_json::json!({
            "signal": {
                "doc_type": "LearningPath",
                "action": "commit",
                "id": "test-path",
                "data": {"title": "Test Path"},
                "action_hash": "uhCkk...",
                "author": "uhCAk..."
            }
        });

        subscriber.process_signal_value(&json);
    }

    #[test]
    fn test_doorway_cache_signal_parsing() {
        let subscriber = SignalSubscriber::new(SubscriberConfig::default());

        // Test DoorwaySignal format (from warm_cache / doorway-client)
        let json = serde_json::json!({
            "namespace": "doorway",
            "payload": {
                "signal_type": "upsert",
                "doc_type": "Content",
                "doc_id": "manifesto",
                "data": {
                    "id": "manifesto",
                    "title": "Elohim Protocol Manifesto",
                    "contentType": "manifesto"
                },
                "ttl_secs": 3600,
                "public": true,
                "reach": "commons"
            }
        });

        // Test that parsing doesn't panic
        subscriber.process_signal_value(&json);
    }

    #[test]
    fn test_cache_signal_to_projection_signal() {
        let cache_signal = CacheSignal {
            signal_type: CacheSignalType::Upsert,
            doc_type: "Content".to_string(),
            doc_id: "test-content".to_string(),
            data: Some(serde_json::json!({
                "title": "Test Content"
            })),
            ttl_secs: Some(3600),
            public: true,
            reach: Some("commons".to_string()),
        };

        let projection_signal = cache_signal.to_projection_signal();

        assert_eq!(projection_signal.doc_type, "Content");
        assert_eq!(projection_signal.action, "commit");
        assert_eq!(projection_signal.id, "test-content");
        assert_eq!(projection_signal.ttl_secs, Some(3600));
        assert_eq!(projection_signal.action_hash, "cache-warm");
        assert_eq!(projection_signal.author, "cache-warm");
    }

    #[test]
    fn test_cache_signal_delete_to_projection_signal() {
        let cache_signal = CacheSignal {
            signal_type: CacheSignalType::Delete,
            doc_type: "Content".to_string(),
            doc_id: "deleted-content".to_string(),
            data: None,
            ttl_secs: None,
            public: false,
            reach: None,
        };

        let projection_signal = cache_signal.to_projection_signal();

        assert_eq!(projection_signal.action, "delete");
        assert_eq!(projection_signal.id, "deleted-content");
        assert_eq!(projection_signal.data, serde_json::Value::Null);
    }

    #[test]
    fn test_infrastructure_signal_parsing() {
        // Test ContentServerCommitted signal parsing
        let json = serde_json::json!({
            "type": "ContentServerCommitted",
            "payload": {
                "action_hash": "uhCkkABC123",
                "entry_hash": "uhCEkDEF456",
                "server": {
                    "content_hash": "sha256-abc123def456",
                    "capability": "blob",
                    "serve_url": "http://localhost:8080/store",
                    "endpoints": [
                        {
                            "url": "http://192.168.1.100:8080/store",
                            "protocol": "http",
                            "priority": 100
                        },
                        {
                            "url": "https://my-node.example.com/api/blob",
                            "protocol": "https",
                            "priority": 50
                        }
                    ],
                    "online": true,
                    "priority": 80,
                    "region": "us-west",
                    "bandwidth_mbps": 100,
                    "registered_at": 1704067200,
                    "last_heartbeat": 1704067200
                },
                "author": "uhCAkXYZ789"
            }
        });

        let signal: InfrastructureSignal = serde_json::from_value(json).unwrap();

        match signal {
            InfrastructureSignal::ContentServerCommitted { server, author, .. } => {
                assert_eq!(server.content_hash, "sha256-abc123def456");
                assert_eq!(server.capability, "blob");
                assert_eq!(server.endpoints.len(), 2);
                assert_eq!(server.endpoints[0].url, "http://192.168.1.100:8080/store");
                assert_eq!(server.endpoints[0].priority, 100);
                assert!(server.online);
                assert_eq!(author, "uhCAkXYZ789");
            }
        }
    }

    #[test]
    fn test_infrastructure_signal_with_subscriber() {
        let subscriber = SignalSubscriber::new(SubscriberConfig::default());

        // Subscribe to both channels before processing
        let mut signal_rx = subscriber.subscribe();
        let mut blob_rx = subscriber.subscribe_blob_registry();

        // Test ContentServerCommitted signal
        let json = serde_json::json!({
            "type": "ContentServerCommitted",
            "payload": {
                "action_hash": "uhCkkABC123",
                "entry_hash": "uhCEkDEF456",
                "server": {
                    "content_hash": "sha256-test",
                    "capability": "blob",
                    "endpoints": [{
                        "url": "http://localhost:8080/store",
                        "protocol": "http",
                        "priority": 100
                    }],
                    "online": true,
                    "priority": 50,
                    "registered_at": 1704067200,
                    "last_heartbeat": 1704067200
                },
                "author": "uhCAk123"
            }
        });

        // Process the signal - should emit to both channels
        subscriber.process_signal_value(&json);

        // Check that projection signal was emitted (new behavior - routes to ProjectionStore)
        match signal_rx.try_recv() {
            Ok(signal) => {
                assert_eq!(signal.action, "update_endpoints");
                assert_eq!(signal.id, "sha256-test"); // blob_hash
                assert!(signal.data.as_array().is_some());
                let endpoints = signal.data.as_array().unwrap();
                assert_eq!(endpoints.len(), 1);
                assert_eq!(endpoints[0].as_str().unwrap(), "http://localhost:8080/store");
            }
            Err(_) => panic!("Expected ProjectionSignal to be emitted for update_endpoints"),
        }

        // Check that registration was also emitted to blob registry (backwards compatibility)
        match blob_rx.try_recv() {
            Ok(registration) => {
                assert_eq!(registration.content_hash, "sha256-test");
                assert_eq!(registration.endpoints.len(), 1);
                assert_eq!(registration.endpoints[0], "http://localhost:8080/store");
            }
            Err(_) => panic!("Expected ContentServerRegistration to be emitted"),
        }
    }
}
