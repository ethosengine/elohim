//! Gateway NATS publisher for JetStream-based request routing
//!
//! Publishes Holochain requests to JetStream and waits for responses from workers.
//! This replaces direct WebSocket proxy connections for better scalability.

use async_nats::jetstream::{self, stream::Stream};
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, RwLock};
use tracing::{debug, error, info, warn};

use crate::types::{DoorwayError, Result};
use crate::worker::processor::{WorkerRequest, WorkerResponse, STREAM_NAME, SUBJECT_PREFIX};

/// Gateway configuration
pub struct GatewayConfig {
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
    /// Gateway ID for tracking
    pub gateway_id: String,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            request_timeout_ms: 30000,
            gateway_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

/// Gateway publisher that routes requests through NATS JetStream
pub struct GatewayPublisher {
    config: GatewayConfig,
    nats_client: async_nats::Client,
    jetstream: jetstream::Context,
    /// Pending responses indexed by request_id
    pending: Arc<RwLock<HashMap<String, oneshot::Sender<WorkerResponse>>>>,
}

impl GatewayPublisher {
    /// Create a new gateway publisher
    pub async fn new(nats_url: &str, config: GatewayConfig) -> Result<Self> {
        info!(
            "Gateway {} connecting to NATS at {}",
            config.gateway_id, nats_url
        );

        let nats_client = async_nats::connect(nats_url)
            .await
            .map_err(|e| DoorwayError::Nats(format!("Failed to connect to NATS: {}", e)))?;

        let jetstream = jetstream::new(nats_client.clone());

        let publisher = Self {
            config,
            nats_client,
            jetstream,
            pending: Arc::new(RwLock::new(HashMap::new())),
        };

        // Ensure the stream exists
        publisher.ensure_stream().await?;

        // Start the response listener
        publisher.start_response_listener().await?;

        info!("Gateway publisher initialized");
        Ok(publisher)
    }

    /// Ensure the request stream exists
    async fn ensure_stream(&self) -> Result<Stream> {
        let stream = self
            .jetstream
            .get_or_create_stream(jetstream::stream::Config {
                name: STREAM_NAME.to_string(),
                subjects: vec![format!("{}.>", SUBJECT_PREFIX)],
                max_age: Duration::from_secs(3600), // 1 hour
                max_bytes: 500 * 1024 * 1024,       // 500 MiB
                storage: jetstream::stream::StorageType::Memory,
                ..Default::default()
            })
            .await
            .map_err(|e| DoorwayError::Nats(format!("Failed to create stream: {}", e)))?;

        info!("Using stream {} for request routing", STREAM_NAME);
        Ok(stream)
    }

    /// Start listening for responses on a reply subject
    async fn start_response_listener(&self) -> Result<()> {
        // Create a unique reply subject for this gateway
        let reply_subject = format!("_INBOX.gateway.{}", self.config.gateway_id);

        let mut subscriber = self
            .nats_client
            .subscribe(reply_subject.clone())
            .await
            .map_err(|e| DoorwayError::Nats(format!("Failed to subscribe to replies: {}", e)))?;

        info!("Listening for responses on {}", reply_subject);

        let pending = Arc::clone(&self.pending);

        // Spawn response handler task
        tokio::spawn(async move {
            while let Some(msg) = subscriber.next().await {
                // Parse the response
                match serde_json::from_slice::<WorkerResponse>(&msg.payload) {
                    Ok(response) => {
                        debug!(
                            "Received response for request {} from worker {}",
                            response.request_id, response.worker_id
                        );

                        // Find and notify the waiting sender
                        let sender = {
                            let mut pending = pending.write().await;
                            pending.remove(&response.request_id)
                        };

                        if let Some(sender) = sender {
                            let _ = sender.send(response);
                        } else {
                            warn!("No pending request for response {}", response.request_id);
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse worker response: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Get the reply subject for this gateway
    fn reply_subject(&self) -> String {
        format!("_INBOX.gateway.{}", self.config.gateway_id)
    }

    /// Send a request to the conductor via NATS and wait for response
    pub async fn request(
        &self,
        payload: Vec<u8>,
        interface: &str,
        app_port: Option<u16>,
    ) -> Result<Vec<u8>> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let reply_subject = self.reply_subject();

        // Create the request
        let request = WorkerRequest {
            request_id: request_id.clone(),
            reply_subject: reply_subject.clone(),
            payload,
            interface: interface.to_string(),
            app_port,
        };

        // Create response channel
        let (tx, rx) = oneshot::channel();

        // Register pending request
        {
            let mut pending = self.pending.write().await;
            pending.insert(request_id.clone(), tx);
        }

        // Serialize and publish the request
        let request_json = serde_json::to_vec(&request)
            .map_err(|e| DoorwayError::Nats(format!("Failed to serialize request: {}", e)))?;

        // Determine subject based on interface
        let subject = format!("{}.{}", SUBJECT_PREFIX, interface);

        self.jetstream
            .publish(subject.clone(), request_json.into())
            .await
            .map_err(|e| DoorwayError::Nats(format!("Failed to publish request: {}", e)))?
            .await
            .map_err(|e| DoorwayError::Nats(format!("Failed to confirm publish: {}", e)))?;

        debug!(
            "Published request {} to {} (reply: {})",
            request_id, subject, reply_subject
        );

        // Wait for response with timeout
        let timeout_duration = Duration::from_millis(self.config.request_timeout_ms);

        match tokio::time::timeout(timeout_duration, rx).await {
            Ok(Ok(response)) => {
                if response.success {
                    Ok(response.payload)
                } else {
                    Err(DoorwayError::Holochain(
                        response
                            .error
                            .unwrap_or_else(|| "Unknown error".to_string()),
                    ))
                }
            }
            Ok(Err(_)) => {
                // Clean up pending
                self.pending.write().await.remove(&request_id);
                Err(DoorwayError::Nats("Response channel closed".to_string()))
            }
            Err(_) => {
                // Clean up pending
                self.pending.write().await.remove(&request_id);
                Err(DoorwayError::Nats(format!(
                    "Request {} timed out after {}ms",
                    request_id, self.config.request_timeout_ms
                )))
            }
        }
    }

    /// Send a request and return immediately (fire-and-forget)
    pub async fn publish_only(
        &self,
        payload: Vec<u8>,
        interface: &str,
        app_port: Option<u16>,
    ) -> Result<String> {
        let request_id = uuid::Uuid::new_v4().to_string();

        let request = WorkerRequest {
            request_id: request_id.clone(),
            reply_subject: String::new(), // No reply expected
            payload,
            interface: interface.to_string(),
            app_port,
        };

        let request_json = serde_json::to_vec(&request)
            .map_err(|e| DoorwayError::Nats(format!("Failed to serialize request: {}", e)))?;

        let subject = format!("{}.{}", SUBJECT_PREFIX, interface);

        self.jetstream
            .publish(subject, request_json.into())
            .await
            .map_err(|e| DoorwayError::Nats(format!("Failed to publish request: {}", e)))?
            .await
            .map_err(|e| DoorwayError::Nats(format!("Failed to confirm publish: {}", e)))?;

        Ok(request_id)
    }

    /// Get count of pending requests
    pub async fn pending_count(&self) -> usize {
        self.pending.read().await.len()
    }
}

/// Session-based gateway for handling a WebSocket connection
///
/// This maintains a bidirectional NATS channel for a single client session,
/// allowing the worker to send signals/responses back asynchronously.
pub struct SessionGateway {
    publisher: Arc<GatewayPublisher>,
    session_id: String,
    interface: String,
    app_port: Option<u16>,
}

impl SessionGateway {
    /// Create a new session gateway
    pub fn new(publisher: Arc<GatewayPublisher>, interface: &str, app_port: Option<u16>) -> Self {
        Self {
            publisher,
            session_id: uuid::Uuid::new_v4().to_string(),
            interface: interface.to_string(),
            app_port,
        }
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Send a request through this session
    pub async fn request(&self, payload: Vec<u8>) -> Result<Vec<u8>> {
        self.publisher
            .request(payload, &self.interface, self.app_port)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GatewayConfig::default();
        assert_eq!(config.request_timeout_ms, 30000);
        assert!(!config.gateway_id.is_empty());
    }
}
