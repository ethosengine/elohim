//! NATS JetStream client wrapper
//!
//! Provides connection management with reconnection, TLS support,
//! and request/response patterns with timeouts.

use async_nats::{Client, ConnectOptions, HeaderMap, HeaderValue};
use bytes::Bytes;
use futures_util::StreamExt;
use std::str::FromStr;
use std::time::Duration;
use tracing::info;

use crate::config::NatsArgs;
use crate::types::DoorwayError;

/// Default request timeout for RPC-style calls
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Default ping interval for keep-alive
const DEFAULT_PING_INTERVAL: Duration = Duration::from_secs(120);

/// NATS client wrapper with JetStream support
#[derive(Clone)]
pub struct NatsClient {
    /// Underlying NATS client
    client: Client,
    /// Request timeout for RPC calls
    request_timeout: Duration,
    /// Client name for logging
    name: String,
}

impl NatsClient {
    /// Create a new NATS client
    pub async fn new(args: &NatsArgs, name: &str) -> Result<Self, DoorwayError> {
        info!("Connecting to NATS at {}", args.nats_url);

        // Don't use retry_on_initial_connect() - we want fast failure if NATS isn't available
        // Reconnection will still work after initial successful connection
        let mut options = ConnectOptions::new()
            .name(name)
            .ping_interval(DEFAULT_PING_INTERVAL)
            .connection_timeout(Duration::from_secs(5));

        // Add credentials if provided
        if let (Some(user), Some(pass)) = (&args.nats_user, &args.nats_password) {
            options = options.user_and_password(user.clone(), pass.clone());
        }

        let client = options
            .connect(&args.nats_url)
            .await
            .map_err(|e| DoorwayError::Nats(format!("Failed to connect: {}", e)))?;

        info!("Connected to NATS at {}", args.nats_url);

        Ok(Self {
            client,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            name: name.to_string(),
        })
    }

    /// Set the request timeout for RPC calls
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Get the underlying NATS client
    pub fn inner(&self) -> &Client {
        &self.client
    }

    /// Publish a message to a subject
    pub async fn publish(&self, subject: &str, payload: Bytes) -> Result<(), DoorwayError> {
        self.client
            .publish(subject.to_string(), payload)
            .await
            .map_err(|e| DoorwayError::Nats(format!("Publish failed: {}", e)))
    }

    /// Publish a message with headers
    pub async fn publish_with_headers(
        &self,
        subject: &str,
        headers: HeaderMap,
        payload: Bytes,
    ) -> Result<(), DoorwayError> {
        self.client
            .publish_with_headers(subject.to_string(), headers, payload)
            .await
            .map_err(|e| DoorwayError::Nats(format!("Publish failed: {}", e)))
    }

    /// Request/response pattern with timeout
    pub async fn request(
        &self,
        subject: &str,
        payload: Bytes,
    ) -> Result<async_nats::Message, DoorwayError> {
        tokio::time::timeout(
            self.request_timeout,
            self.client.request(subject.to_string(), payload),
        )
        .await
        .map_err(|_| DoorwayError::Nats(format!("Request to {} timed out", subject)))?
        .map_err(|e| DoorwayError::Nats(format!("Request failed: {}", e)))
    }

    /// Request with custom headers
    pub async fn request_with_headers(
        &self,
        subject: &str,
        headers: HeaderMap,
        payload: Bytes,
    ) -> Result<async_nats::Message, DoorwayError> {
        // Create a unique inbox for the reply
        let inbox = self.client.new_inbox();

        // Subscribe to the inbox before publishing
        let mut subscription = self
            .client
            .subscribe(inbox.clone())
            .await
            .map_err(|e| DoorwayError::Nats(format!("Subscribe failed: {}", e)))?;

        // Add reply header
        let mut headers = headers;
        headers.insert(
            "Nats-Reply-To",
            HeaderValue::from_str(&inbox)
                .map_err(|e| DoorwayError::Nats(format!("Invalid header: {}", e)))?,
        );

        // Publish the request
        self.publish_with_headers(subject, headers, payload).await?;

        // Wait for response with timeout
        tokio::time::timeout(self.request_timeout, subscription.next())
            .await
            .map_err(|_| DoorwayError::Nats(format!("Request to {} timed out", subject)))?
            .ok_or_else(|| DoorwayError::Nats("No response received".into()))
    }

    /// Subscribe to a subject
    pub async fn subscribe(&self, subject: &str) -> Result<async_nats::Subscriber, DoorwayError> {
        self.client
            .subscribe(subject.to_string())
            .await
            .map_err(|e| DoorwayError::Nats(format!("Subscribe failed: {}", e)))
    }

    /// Flush pending messages
    pub async fn flush(&self) -> Result<(), DoorwayError> {
        self.client
            .flush()
            .await
            .map_err(|e| DoorwayError::Nats(format!("Flush failed: {}", e)))
    }

    /// Get the client name
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Create NATS headers with reply subject
pub fn headers_with_reply(reply_subject: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if let Ok(value) = HeaderValue::from_str(reply_subject) {
        headers.insert("Nats-Reply-To", value);
    }
    headers
}

#[cfg(test)]
mod tests {
    // Integration tests would require a running NATS server
    // See docker-compose.dev.yml for local testing
}
