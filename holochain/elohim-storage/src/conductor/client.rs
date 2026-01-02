//! Conductor Client with Automatic Reconnection
//!
//! Single responsibility: Maintain a healthy session, reconnecting as needed.
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────┐
//! │                   ConductorClient                       │
//! │  - Exposes call_zome() to callers                       │
//! │  - Manages session lifecycle                            │
//! │  - Handles reconnection on failure                      │
//! └────────────────────────────────────────────────────────┘
//!                            │
//!                            ▼
//! ┌────────────────────────────────────────────────────────┐
//! │                      Session                            │
//! │  - An authenticated, ready-to-use connection            │
//! │  - Created via Session::establish()                     │
//! │  - Dies on disconnect (no auto-reconnect)               │
//! └────────────────────────────────────────────────────────┘
//!                            │
//!               ┌────────────┼────────────┐
//!               ▼            ▼            ▼
//!          Transport      Protocol       Auth
//! ```
//!
//! # Usage
//!
//! ```ignore
//! // Create a client - blocks until connected
//! let client = ConductorClient::connect(config).await?;
//!
//! // Make zome calls - automatically reconnects on failure
//! let result = client.call_zome(cell_id, "my_zome", "my_fn", payload).await?;
//! ```
//!
//! # Reconnection Policy
//!
//! When a session dies:
//! 1. Mark session as dead
//! 2. On next call_zome(), attempt to re-establish
//! 3. If re-establish fails, return error to caller
//! 4. Caller can retry, triggering another reconnection attempt
//!
//! This is a "lazy reconnection" strategy - we don't background-reconnect,
//! we reconnect on demand. This avoids:
//! - Wasted connections when the client isn't being used
//! - Complex background task management
//! - The original bug (race between background connect and caller)

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::session::{Session, SessionConfig};
use crate::error::StorageError;

/// Configuration for the conductor client.
#[derive(Debug, Clone)]
pub struct ConductorClientConfig {
    /// Admin interface URL for obtaining auth token
    pub admin_url: String,
    /// App interface URL for zome calls
    pub app_url: String,
    /// App ID for authentication
    pub app_id: String,
    /// Timeout for individual requests
    pub request_timeout: Duration,
    /// Delay between reconnection attempts
    pub reconnect_delay: Duration,
    /// Maximum reconnection attempts before giving up (0 = unlimited)
    pub max_reconnect_attempts: u32,
}

impl Default for ConductorClientConfig {
    fn default() -> Self {
        Self {
            admin_url: "ws://localhost:4444".to_string(),
            app_url: "ws://localhost:4445".to_string(),
            app_id: "elohim".to_string(),
            request_timeout: Duration::from_secs(60),
            reconnect_delay: Duration::from_secs(5),
            max_reconnect_attempts: 0, // Unlimited
        }
    }
}

/// A conductor client with automatic session management.
///
/// # Guarantees
///
/// - `connect()` only returns when we have a valid session
/// - `call_zome()` attempts to reconnect if the session died
/// - Thread-safe: can be wrapped in Arc and shared
///
/// # Non-Guarantees
///
/// - Sessions can die between calls
/// - Reconnection can fail
/// - Not all errors are recoverable
pub struct ConductorClient {
    config: ConductorClientConfig,
    /// The current session, if any.
    /// None means we need to reconnect.
    session: Arc<RwLock<Option<Session>>>,
}

impl ConductorClient {
    /// Connect to the conductor.
    ///
    /// This method blocks until we have a valid, authenticated session.
    /// There are no background tasks, no polling, no race conditions.
    ///
    /// # Errors
    /// Returns an error if we cannot establish a session.
    pub async fn connect(config: ConductorClientConfig) -> Result<Self, StorageError> {
        info!(
            app_url = %config.app_url,
            app_id = %config.app_id,
            "Creating conductor client"
        );

        // Create session config from client config
        let session_config = SessionConfig {
            admin_url: config.admin_url.clone(),
            app_url: config.app_url.clone(),
            app_id: config.app_id.clone(),
            request_timeout: config.request_timeout,
        };

        // Establish the initial session
        let session = Session::establish(session_config).await?;

        info!(app_url = %config.app_url, "Conductor client connected");

        Ok(Self {
            config,
            session: Arc::new(RwLock::new(Some(session))),
        })
    }

    /// Make a zome call.
    ///
    /// If the session is dead, attempts to reconnect first.
    ///
    /// # Arguments
    /// * `cell_id` - The cell to call
    /// * `zome_name` - Name of the zome
    /// * `fn_name` - Name of the function
    /// * `payload` - Msgpack-encoded arguments
    ///
    /// # Errors
    /// - Session is dead and reconnection failed
    /// - Request failed
    /// - Zome returned an error
    pub async fn call_zome(
        &self,
        cell_id: &[u8],
        zome_name: &str,
        fn_name: &str,
        payload: &[u8],
    ) -> Result<Vec<u8>, StorageError> {
        // Try to get a working session
        let mut attempts = 0u32;
        loop {
            // Check if we need to reconnect
            {
                let session_guard = self.session.read().await;
                if let Some(ref session) = *session_guard {
                    if session.is_alive() {
                        break; // Session is good
                    }
                }
            }

            // Need to reconnect
            if self.config.max_reconnect_attempts > 0
                && attempts >= self.config.max_reconnect_attempts
            {
                return Err(StorageError::Conductor(format!(
                    "Max reconnection attempts ({}) exceeded",
                    self.config.max_reconnect_attempts
                )));
            }

            attempts += 1;
            warn!(
                attempt = attempts,
                max = self.config.max_reconnect_attempts,
                "Session dead, attempting reconnection"
            );

            match self.reconnect().await {
                Ok(()) => {
                    info!("Reconnection successful");
                    break;
                }
                Err(e) => {
                    error!(error = %e, attempt = attempts, "Reconnection failed");
                    if self.config.max_reconnect_attempts > 0
                        && attempts >= self.config.max_reconnect_attempts
                    {
                        return Err(e);
                    }
                    tokio::time::sleep(self.config.reconnect_delay).await;
                }
            }
        }

        // Now make the actual call
        let mut session_guard = self.session.write().await;
        let session = session_guard
            .as_mut()
            .ok_or_else(|| StorageError::Conductor("No session available".into()))?;

        session.call_zome(cell_id, zome_name, fn_name, payload).await
    }

    /// Check if the client currently has a live session.
    pub async fn is_connected(&self) -> bool {
        let session_guard = self.session.read().await;
        session_guard
            .as_ref()
            .map(|s| s.is_alive())
            .unwrap_or(false)
    }

    /// Attempt to reconnect.
    ///
    /// This is called automatically by call_zome() when needed,
    /// but can also be called manually to pre-warm the connection.
    async fn reconnect(&self) -> Result<(), StorageError> {
        debug!("Reconnecting...");

        let session_config = SessionConfig {
            admin_url: self.config.admin_url.clone(),
            app_url: self.config.app_url.clone(),
            app_id: self.config.app_id.clone(),
            request_timeout: self.config.request_timeout,
        };

        let new_session = Session::establish(session_config).await?;

        // Replace the session
        let mut session_guard = self.session.write().await;
        *session_guard = Some(new_session);

        debug!("Reconnection complete");
        Ok(())
    }

    /// Force a reconnection, even if the current session is alive.
    ///
    /// Useful for:
    /// - Testing
    /// - Recovering from suspected corruption
    /// - Pre-warming after idle period
    #[allow(dead_code)]
    pub async fn force_reconnect(&self) -> Result<(), StorageError> {
        // Drop current session
        {
            let mut session_guard = self.session.write().await;
            *session_guard = None;
        }

        self.reconnect().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ConductorClientConfig::default();
        assert_eq!(config.admin_url, "ws://localhost:4444");
        assert_eq!(config.app_url, "ws://localhost:4445");
        assert_eq!(config.app_id, "elohim");
        assert_eq!(config.max_reconnect_attempts, 0); // Unlimited
    }
}
