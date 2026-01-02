//! Conductor Session
//!
//! Single responsibility: An authenticated, ready-to-use connection to the conductor.
//!
//! # The Key Abstraction
//!
//! A `Session` can ONLY be created via `Session::establish()`, which:
//! 1. Obtains an auth token from the admin interface
//! 2. Connects to the app interface
//! 3. Authenticates the connection
//! 4. Only THEN returns a Session
//!
//! This makes it **impossible** to have a Session that isn't ready.
//! If you have a Session, you can make zome calls.
//!
//! # Why This Pattern Matters
//!
//! The original bug was caused by:
//! - Polling a "connected" boolean flag
//! - Returning a handle before the connection was ready
//! - Race condition between polling and the actual connection
//!
//! With this pattern:
//! - `Session::establish()` is async and blocks until fully ready
//! - No polling, no flags, no race conditions
//! - The type system enforces the invariant

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, Mutex};
use tracing::{debug, error, info};

use super::auth::AuthToken;
use super::protocol::{decode_response, encode_zome_call};
use super::transport::{Transport, WsSink, WsStream};
use crate::error::StorageError;

/// Configuration for establishing a session.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Admin interface URL for obtaining auth token
    pub admin_url: String,
    /// App interface URL for zome calls
    pub app_url: String,
    /// App ID for authentication
    pub app_id: String,
    /// Timeout for individual requests
    pub request_timeout: Duration,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            admin_url: "ws://localhost:4444".to_string(),
            app_url: "ws://localhost:4445".to_string(),
            app_id: "elohim".to_string(),
            request_timeout: Duration::from_secs(60),
        }
    }
}

/// An authenticated session with the Holochain conductor.
///
/// # Guarantees
///
/// If you have a `Session`, you have:
/// - A valid WebSocket connection to the app interface
/// - A valid authentication token
/// - The ability to make zome calls
///
/// # Lifecycle
///
/// Sessions are created via `Session::establish()` and remain valid until:
/// - The conductor closes the connection
/// - A network error occurs
/// - The session is explicitly closed
///
/// Sessions do NOT automatically reconnect. If the connection dies,
/// the session is gone. Use `ConductorClient` for automatic reconnection.
pub struct Session {
    config: SessionConfig,
    /// Send half of the WebSocket
    sink: WsSink,
    /// Request ID counter
    next_id: AtomicU64,
    /// Pending response channels, keyed by request ID
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Vec<u8>, StorageError>>>>>,
    /// Handle to the receiver task
    recv_task: tokio::task::JoinHandle<()>,
}

impl Session {
    /// Establish a new authenticated session.
    ///
    /// This method blocks until the session is fully ready to use.
    /// If any step fails, an error is returned and no session exists.
    ///
    /// # Steps
    /// 1. Obtain auth token from admin interface
    /// 2. Connect to app interface
    /// 3. Authenticate connection
    /// 4. Start receiver task
    /// 5. Return ready session
    ///
    /// # Errors
    /// - Admin interface connection fails
    /// - Auth token request fails
    /// - App interface connection fails
    /// - Authentication is rejected
    pub async fn establish(config: SessionConfig) -> Result<Self, StorageError> {
        info!(
            admin_url = %config.admin_url,
            app_url = %config.app_url,
            app_id = %config.app_id,
            "Establishing conductor session"
        );

        // Step 1: Get auth token from admin interface
        let token = AuthToken::obtain(&config.admin_url, &config.app_id).await?;
        debug!("Auth token obtained");

        // Step 2: Connect to app interface
        let mut transport = Transport::connect(&config.app_url).await?;
        debug!("Connected to app interface");

        // Step 3: Authenticate
        token.authenticate(&mut transport).await?;
        debug!("Authenticated successfully");

        // Step 4: Split transport for concurrent send/receive
        let (sink, stream) = transport.split();

        // Step 5: Set up pending response tracking
        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Vec<u8>, StorageError>>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // Step 6: Spawn receiver task
        let pending_for_recv = Arc::clone(&pending);
        let recv_task = tokio::spawn(async move {
            receiver_loop(stream, pending_for_recv).await;
        });

        info!("Conductor session established");

        Ok(Self {
            config,
            sink,
            next_id: AtomicU64::new(1),
            pending,
            recv_task,
        })
    }

    /// Make a zome call on this session.
    ///
    /// # Arguments
    /// * `dna_hash` - DNA hash bytes
    /// * `agent_pub_key` - Agent public key bytes
    /// * `zome_name` - Name of the zome
    /// * `fn_name` - Name of the function
    /// * `payload` - Msgpack-encoded function arguments
    ///
    /// # Returns
    /// The msgpack-encoded response from the zome function.
    ///
    /// # Errors
    /// - Connection is closed
    /// - Request times out
    /// - Zome returns an error
    pub async fn call_zome(
        &mut self,
        dna_hash: &[u8],
        agent_pub_key: &[u8],
        zome_name: &str,
        fn_name: &str,
        payload: &[u8],
    ) -> Result<Vec<u8>, StorageError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        debug!(
            id = id,
            zome = %zome_name,
            fn_name = %fn_name,
            "Making zome call"
        );

        // Encode the request
        let encoded = encode_zome_call(id, dna_hash, agent_pub_key, zome_name, fn_name, payload)?;

        // Set up response channel
        let (response_tx, response_rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            pending.insert(id, response_tx);
        }

        // Send request
        use futures_util::SinkExt;
        use tokio_tungstenite::tungstenite::protocol::Message;

        self.sink
            .send(Message::Binary(encoded.bytes))
            .await
            .map_err(|e| {
                // Clean up pending on send failure
                let pending = self.pending.clone();
                tokio::spawn(async move {
                    pending.lock().await.remove(&id);
                });
                StorageError::Conductor(format!("Failed to send: {}", e))
            })?;

        // Wait for response with timeout
        match tokio::time::timeout(self.config.request_timeout, response_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => {
                // Channel was closed - receiver task died or cleaned up
                Err(StorageError::Conductor("Response channel closed".into()))
            }
            Err(_) => {
                // Timeout - clean up pending
                self.pending.lock().await.remove(&id);
                Err(StorageError::Conductor("Request timeout".into()))
            }
        }
    }

    /// Check if the session is still alive.
    ///
    /// Note: This checks if the receiver task is still running.
    /// It doesn't guarantee the next zome call will succeed.
    pub fn is_alive(&self) -> bool {
        !self.recv_task.is_finished()
    }

    /// Get the app URL this session is connected to.
    pub fn app_url(&self) -> &str {
        &self.config.app_url
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // Abort the receiver task when session is dropped
        self.recv_task.abort();
        debug!("Session dropped, receiver task aborted");
    }
}

/// Receiver loop - runs in a spawned task.
///
/// Receives responses from the conductor and routes them to the
/// appropriate pending request channel.
async fn receiver_loop(
    mut stream: WsStream,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Vec<u8>, StorageError>>>>>,
) {
    use futures_util::StreamExt;
    use tokio_tungstenite::tungstenite::protocol::Message;

    debug!("Receiver loop started");

    while let Some(msg) = stream.next().await {
        match msg {
            Ok(Message::Binary(data)) => {
                // Try to decode and route the response
                match decode_response(&data) {
                    Ok(response) => {
                        let mut pending = pending.lock().await;
                        if let Some(tx) = pending.remove(&response.id) {
                            let result = response
                                .result
                                .map_err(|e| StorageError::Conductor(e));
                            let _ = tx.send(result);
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to decode response");
                    }
                }
            }
            Ok(Message::Ping(_)) => {
                // Pong is handled automatically by tungstenite
                debug!("Received ping");
            }
            Ok(Message::Close(frame)) => {
                info!(frame = ?frame, "Conductor closed connection");
                break;
            }
            Err(e) => {
                error!(error = %e, "WebSocket error");
                break;
            }
            _ => {
                // Ignore other message types
            }
        }
    }

    debug!("Receiver loop ended");

    // Clean up any remaining pending requests
    let mut pending = pending.lock().await;
    for (id, tx) in pending.drain() {
        debug!(id = id, "Cleaning up pending request");
        let _ = tx.send(Err(StorageError::Conductor("Session closed".into())));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_config_default() {
        let config = SessionConfig::default();
        assert_eq!(config.admin_url, "ws://localhost:4444");
        assert_eq!(config.app_url, "ws://localhost:4445");
        assert_eq!(config.app_id, "elohim");
    }
}
