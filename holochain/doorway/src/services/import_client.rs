//! Import Client - Single connection for import operations
//!
//! Provides a dedicated single WebSocket connection for import operations.
//! Unlike WorkerPool which manages multiple connections for concurrent reads,
//! ImportClient uses ONE connection to avoid overwhelming the conductor
//! during batch import operations.
//!
//! ## Design Rationale
//!
//! Import operations (queue_import, get_import_status) should:
//! 1. Use a single connection to avoid conductor connection churn
//! 2. Be fast (just storing manifest, not processing items)
//! 3. Let elohim-storage handle batch writes via WriteBuffer
//!
//! The zome's queue_import stores the manifest and emits ImportBatchQueued signal.
//! elohim-storage listens for this signal and processes chunks using WriteBuffer.

use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

use crate::worker::ConductorConnection;
use crate::types::Result;

/// Configuration for the import client
#[derive(Debug, Clone)]
pub struct ImportClientConfig {
    /// Conductor app interface URL (NOT admin interface)
    pub app_url: String,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Whether to auto-reconnect on failure
    pub auto_reconnect: bool,
}

impl Default for ImportClientConfig {
    fn default() -> Self {
        Self {
            app_url: "ws://localhost:4445".to_string(),
            timeout_ms: 60_000, // 60s for import operations (longer than default)
            auto_reconnect: true,
        }
    }
}

/// Single-connection client for import operations
pub struct ImportClient {
    config: ImportClientConfig,
    /// The single conductor connection (lazily initialized)
    connection: RwLock<Option<Arc<ConductorConnection>>>,
    /// Lock to prevent concurrent connection attempts
    connecting: Mutex<()>,
}

impl ImportClient {
    /// Create a new import client
    pub fn new(config: ImportClientConfig) -> Self {
        info!(
            app_url = %config.app_url,
            timeout_ms = config.timeout_ms,
            "ImportClient created (single connection for imports)"
        );

        Self {
            config,
            connection: RwLock::new(None),
            connecting: Mutex::new(()),
        }
    }

    /// Create with default configuration
    pub fn with_defaults(app_url: String) -> Self {
        Self::new(ImportClientConfig {
            app_url,
            ..Default::default()
        })
    }

    /// Get or create the conductor connection
    async fn get_connection(&self) -> Result<Arc<ConductorConnection>> {
        // Fast path: check if we have a connection
        {
            let conn = self.connection.read().await;
            if let Some(ref c) = *conn {
                if c.is_connected().await {
                    return Ok(Arc::clone(c));
                }
            }
        }

        // Slow path: need to (re)connect
        let _lock = self.connecting.lock().await;

        // Double-check after acquiring lock
        {
            let conn = self.connection.read().await;
            if let Some(ref c) = *conn {
                if c.is_connected().await {
                    return Ok(Arc::clone(c));
                }
            }
        }

        // Create new connection
        info!("ImportClient connecting to {}", self.config.app_url);

        let conn = ConductorConnection::connect(&self.config.app_url).await?;
        let conn = Arc::new(conn);

        // Store the connection
        {
            let mut write_conn = self.connection.write().await;
            *write_conn = Some(Arc::clone(&conn));
        }

        info!("ImportClient connected to conductor");
        Ok(conn)
    }

    /// Send a request to the conductor
    pub async fn request(&self, payload: Vec<u8>) -> Result<Vec<u8>> {
        let conn = self.get_connection().await?;

        debug!("ImportClient sending request ({} bytes)", payload.len());

        match conn.request(payload, self.config.timeout_ms).await {
            Ok(response) => {
                debug!("ImportClient got response ({} bytes)", response.len());
                Ok(response)
            }
            Err(e) => {
                warn!("ImportClient request failed: {}", e);

                // Clear the connection so next request will reconnect
                if self.config.auto_reconnect {
                    let mut write_conn = self.connection.write().await;
                    *write_conn = None;
                }

                Err(e)
            }
        }
    }

    /// Check if currently connected
    pub async fn is_connected(&self) -> bool {
        let conn = self.connection.read().await;
        if let Some(ref c) = *conn {
            c.is_connected().await
        } else {
            false
        }
    }

    /// Force reconnection
    pub async fn reconnect(&self) -> Result<()> {
        // Clear existing connection
        {
            let mut write_conn = self.connection.write().await;
            *write_conn = None;
        }

        // Get new connection
        let _ = self.get_connection().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ImportClientConfig::default();
        assert_eq!(config.app_url, "ws://localhost:4445");
        assert_eq!(config.timeout_ms, 60_000);
        assert!(config.auto_reconnect);
    }
}
