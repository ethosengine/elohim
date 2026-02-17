//! Worker processor - NATS JetStream consumer for Holochain requests
//!
//! Subscribes to request streams, processes via conductor, publishes responses.

use async_nats::jetstream::{self, consumer::PullConsumer, stream::Stream};
use futures_util::StreamExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::conductor::ConductorConnection;
use crate::types::{DoorwayError, Result};

/// NATS subjects for Holochain requests
pub const STREAM_NAME: &str = "HC_REQUESTS";
pub const SUBJECT_PREFIX: &str = "hc.request";
pub const CONSUMER_NAME_PREFIX: &str = "hc_worker";

/// Request message from gateway
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct WorkerRequest {
    /// Unique request ID for correlation
    pub request_id: String,
    /// Reply subject to publish response to
    pub reply_subject: String,
    /// Raw Holochain MessagePack payload
    #[serde(with = "base64_bytes")]
    pub payload: Vec<u8>,
    /// Interface type (admin or app)
    pub interface: String,
    /// App port (for app interface)
    pub app_port: Option<u16>,
}

/// Response message to gateway
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct WorkerResponse {
    /// Request ID for correlation
    pub request_id: String,
    /// Success or error
    pub success: bool,
    /// Raw Holochain MessagePack response
    #[serde(with = "base64_bytes")]
    pub payload: Vec<u8>,
    /// Error message if failed
    pub error: Option<String>,
    /// Worker ID that processed this request
    pub worker_id: String,
}

/// Base64 encoding for binary payloads
mod base64_bytes {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        STANDARD.encode(bytes).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        STANDARD
            .decode(&s)
            .map_err(|e| serde::de::Error::custom(format!("base64 decode error: {e}")))
    }
}

/// Worker configuration
pub struct WorkerConfig {
    /// Unique worker ID
    pub worker_id: String,
    /// NATS server URL
    pub nats_url: String,
    /// Conductor admin URL
    pub conductor_url: String,
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
    /// Maximum concurrent requests
    pub max_concurrent: usize,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            worker_id: uuid::Uuid::new_v4().to_string(),
            nats_url: "nats://127.0.0.1:4222".to_string(),
            conductor_url: "ws://localhost:4444".to_string(),
            request_timeout_ms: 30000,
            max_concurrent: 10,
        }
    }
}

/// Backend worker that processes NATS requests
pub struct Worker {
    config: WorkerConfig,
    nats_client: async_nats::Client,
    jetstream: jetstream::Context,
    conductor: ConductorConnection,
    running: Arc<RwLock<bool>>,
}

impl Worker {
    /// Create and start a new worker
    pub async fn new(config: WorkerConfig) -> Result<Self> {
        info!(
            "Starting worker {} connecting to {}",
            config.worker_id, config.nats_url
        );

        // Connect to NATS
        let nats_client = async_nats::connect(&config.nats_url)
            .await
            .map_err(|e| DoorwayError::Nats(format!("Failed to connect to NATS: {e}")))?;

        let jetstream = jetstream::new(nats_client.clone());

        // Connect to conductor
        let conductor = ConductorConnection::connect(&config.conductor_url).await?;

        info!(
            "Worker {} connected to NATS and conductor",
            config.worker_id
        );

        Ok(Self {
            config,
            nats_client,
            jetstream,
            conductor,
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Run the worker processing loop
    pub async fn run(&self) -> Result<()> {
        *self.running.write().await = true;

        // Ensure the stream exists
        let stream = self.ensure_stream().await?;

        // Create a durable consumer
        let consumer = self.ensure_consumer(&stream).await?;

        info!(
            "Worker {} starting request processing loop",
            self.config.worker_id
        );

        // Process messages
        while *self.running.read().await {
            match self.process_batch(&consumer).await {
                Ok(count) => {
                    if count > 0 {
                        debug!("Processed {} requests", count);
                    }
                }
                Err(e) => {
                    error!("Error processing batch: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }

        info!("Worker {} stopped", self.config.worker_id);
        Ok(())
    }

    /// Stop the worker
    pub async fn stop(&self) {
        *self.running.write().await = false;
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
            .map_err(|e| DoorwayError::Nats(format!("Failed to create stream: {e}")))?;

        info!(
            "Using stream {} with subjects {}.>",
            STREAM_NAME, SUBJECT_PREFIX
        );
        Ok(stream)
    }

    /// Ensure the consumer exists
    async fn ensure_consumer(&self, stream: &Stream) -> Result<PullConsumer> {
        let consumer_name = format!("{}_{}", CONSUMER_NAME_PREFIX, self.config.worker_id);

        let consumer = stream
            .get_or_create_consumer(
                &consumer_name,
                jetstream::consumer::pull::Config {
                    durable_name: Some(consumer_name.clone()),
                    ack_policy: jetstream::consumer::AckPolicy::Explicit,
                    filter_subject: format!("{SUBJECT_PREFIX}.>"),
                    max_ack_pending: self.config.max_concurrent as i64,
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| DoorwayError::Nats(format!("Failed to create consumer: {e}")))?;

        info!("Using consumer {}", consumer_name);
        Ok(consumer)
    }

    /// Process a batch of messages
    async fn process_batch(&self, consumer: &PullConsumer) -> Result<usize> {
        let mut messages = consumer
            .fetch()
            .max_messages(self.config.max_concurrent)
            .expires(Duration::from_secs(5))
            .messages()
            .await
            .map_err(|e| DoorwayError::Nats(format!("Failed to fetch messages: {e}")))?;

        let mut count = 0;

        while let Some(msg_result) = messages.next().await {
            match msg_result {
                Ok(msg) => {
                    count += 1;
                    self.process_message(msg).await;
                }
                Err(e) => {
                    warn!("Error receiving message: {}", e);
                }
            }
        }

        Ok(count)
    }

    /// Process a single message
    async fn process_message(&self, msg: jetstream::Message) {
        let payload = msg.payload.to_vec();

        // Parse the request
        let request: WorkerRequest = match serde_json::from_slice(&payload) {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to parse request: {}", e);
                if let Err(e) = msg.ack().await {
                    warn!("Failed to ack malformed message: {}", e);
                }
                return;
            }
        };

        debug!(
            "Processing request {} for {} interface",
            request.request_id, request.interface
        );

        // Process the request via conductor
        let response = match self
            .conductor
            .request(request.payload, self.config.request_timeout_ms)
            .await
        {
            Ok(response_payload) => WorkerResponse {
                request_id: request.request_id.clone(),
                success: true,
                payload: response_payload,
                error: None,
                worker_id: self.config.worker_id.clone(),
            },
            Err(e) => WorkerResponse {
                request_id: request.request_id.clone(),
                success: false,
                payload: vec![],
                error: Some(e.to_string()),
                worker_id: self.config.worker_id.clone(),
            },
        };

        // Publish response to reply subject
        let response_json = match serde_json::to_vec(&response) {
            Ok(j) => j,
            Err(e) => {
                error!("Failed to serialize response: {}", e);
                let _ = msg.ack().await;
                return;
            }
        };

        if let Err(e) = self
            .nats_client
            .publish(request.reply_subject.clone(), response_json.into())
            .await
        {
            error!(
                "Failed to publish response to {}: {}",
                request.reply_subject, e
            );
        } else {
            debug!(
                "Published response for {} to {}",
                request.request_id, request.reply_subject
            );
        }

        // Acknowledge the message
        if let Err(e) = msg.ack().await {
            warn!("Failed to ack message: {}", e);
        }
    }
}
