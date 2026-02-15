//! Heartbeat service for host health monitoring
//!
//! Publishes heartbeats to NATS and monitors for stale hosts.

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::nats::messages::HostHeartbeat;
use crate::nats::NatsClient;
use crate::types::DoorwayError;

/// Default heartbeat interval
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

/// Heartbeat service for publishing and monitoring health
pub struct HeartbeatService {
    /// Our node ID
    node_id: String,
    /// NATS client for publishing
    nats: Option<NatsClient>,
    /// Current active connection count
    active_connections: Arc<AtomicI32>,
    /// Maximum connections
    max_connections: i32,
    /// Geographic region
    region: Option<String>,
    /// Software version
    version: Option<String>,
    /// Whether the service is running
    running: Arc<RwLock<bool>>,
}

impl HeartbeatService {
    /// Create a new heartbeat service
    pub fn new(node_id: String, nats: Option<NatsClient>, max_connections: i32) -> Self {
        Self {
            node_id,
            nats,
            active_connections: Arc::new(AtomicI32::new(0)),
            max_connections,
            region: None,
            version: None,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Set the region
    pub fn with_region(mut self, region: String) -> Self {
        self.region = Some(region);
        self
    }

    /// Set the version
    pub fn with_version(mut self, version: String) -> Self {
        self.version = Some(version);
        self
    }

    /// Increment connection count
    pub fn connection_opened(&self) {
        let count = self.active_connections.fetch_add(1, Ordering::SeqCst);
        debug!("Connection opened, total: {}", count + 1);
    }

    /// Decrement connection count
    pub fn connection_closed(&self) {
        let count = self.active_connections.fetch_sub(1, Ordering::SeqCst);
        debug!("Connection closed, total: {}", count - 1);
    }

    /// Get current connection count
    pub fn connection_count(&self) -> i32 {
        self.active_connections.load(Ordering::SeqCst)
    }

    /// Get a handle to the connection counter
    pub fn connection_counter(&self) -> Arc<AtomicI32> {
        Arc::clone(&self.active_connections)
    }

    /// Create a heartbeat message
    fn create_heartbeat(&self) -> HostHeartbeat {
        let mut heartbeat = HostHeartbeat::new(
            self.node_id.clone(),
            self.active_connections.load(Ordering::SeqCst),
            self.max_connections,
        );
        heartbeat.region = self.region.clone();
        heartbeat.version = self.version.clone();
        heartbeat
    }

    /// Publish a single heartbeat
    pub async fn publish_heartbeat(&self) -> Result<(), DoorwayError> {
        let nats = match &self.nats {
            Some(n) => n,
            None => {
                debug!("NATS not configured, skipping heartbeat");
                return Ok(());
            }
        };

        let heartbeat = self.create_heartbeat();
        let payload = heartbeat
            .to_bytes()
            .map_err(|e| DoorwayError::Nats(format!("Failed to serialize heartbeat: {e}")))?;

        nats.publish(HostHeartbeat::subject(), payload).await?;
        debug!(
            "Published heartbeat: {} connections",
            heartbeat.active_connections
        );

        Ok(())
    }

    /// Start the heartbeat publishing loop
    pub async fn start(self: Arc<Self>) -> Result<(), DoorwayError> {
        // Check if already running
        {
            let mut running = self.running.write().await;
            if *running {
                warn!("Heartbeat service already running");
                return Ok(());
            }
            *running = true;
        }

        info!(
            "Starting heartbeat service for node {} (interval: {:?})",
            self.node_id, HEARTBEAT_INTERVAL
        );

        // Clone for the spawned task
        let service = Arc::clone(&self);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);

            loop {
                interval.tick().await;

                // Check if we should stop
                if !*service.running.read().await {
                    info!("Heartbeat service stopped");
                    break;
                }

                if let Err(e) = service.publish_heartbeat().await {
                    error!("Failed to publish heartbeat: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Stop the heartbeat service
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("Stopping heartbeat service for node {}", self.node_id);
    }

    /// Check if the service is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_counting() {
        let service = HeartbeatService::new("test-node".to_string(), None, 100);

        assert_eq!(service.connection_count(), 0);

        service.connection_opened();
        service.connection_opened();
        assert_eq!(service.connection_count(), 2);

        service.connection_closed();
        assert_eq!(service.connection_count(), 1);
    }

    #[test]
    fn test_heartbeat_creation() {
        let service = HeartbeatService::new("test-node".to_string(), None, 100)
            .with_region("us-west".to_string())
            .with_version("1.0.0".to_string());

        service.connection_opened();
        service.connection_opened();

        let heartbeat = service.create_heartbeat();
        assert_eq!(heartbeat.host_id, "test-node");
        assert_eq!(heartbeat.active_connections, 2);
        assert_eq!(heartbeat.max_connections, 100);
        assert_eq!(heartbeat.region, Some("us-west".to_string()));
        assert_eq!(heartbeat.version, Some("1.0.0".to_string()));
    }
}
