//! Usage logging for Unyt-compatible billing
//!
//! Logs usage events in JSONL format for consumption by Unyt's billing system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::auth::PermissionLevel;

/// Usage event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// WebSocket connection opened
    ConnectionOpened,
    /// WebSocket connection closed
    ConnectionClosed,
    /// Message sent through proxy
    MessageProxied,
    /// Authentication attempt
    AuthAttempt,
    /// Admin operation executed
    AdminOperation,
}

/// Usage event for billing/analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event_type: EventType,
    /// Host/node that handled the request
    pub host_id: String,
    /// User identifier (if authenticated)
    pub user_id: Option<String>,
    /// Human identifier (if authenticated)
    pub human_id: Option<String>,
    /// Permission level of the request
    pub permission_level: PermissionLevel,
    /// Operation name (for admin operations)
    pub operation: Option<String>,
    /// Message size in bytes (for proxied messages)
    pub bytes: Option<u64>,
    /// Duration in milliseconds (for connection close events)
    pub duration_ms: Option<u64>,
    /// Request origin/region
    pub region: Option<String>,
    /// Additional metadata
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl UsageEvent {
    /// Create a new usage event
    pub fn new(event_type: EventType, host_id: String) -> Self {
        Self {
            timestamp: Utc::now(),
            event_type,
            host_id,
            user_id: None,
            human_id: None,
            permission_level: PermissionLevel::Public,
            operation: None,
            bytes: None,
            duration_ms: None,
            region: None,
            metadata: None,
        }
    }

    /// Set the user ID
    pub fn with_user(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Set the human ID
    pub fn with_human(mut self, human_id: String) -> Self {
        self.human_id = Some(human_id);
        self
    }

    /// Set the permission level
    pub fn with_permission(mut self, level: PermissionLevel) -> Self {
        self.permission_level = level;
        self
    }

    /// Set the operation name
    pub fn with_operation(mut self, operation: String) -> Self {
        self.operation = Some(operation);
        self
    }

    /// Set the byte count
    pub fn with_bytes(mut self, bytes: u64) -> Self {
        self.bytes = Some(bytes);
        self
    }

    /// Set the duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Set the region
    pub fn with_region(mut self, region: String) -> Self {
        self.region = Some(region);
        self
    }

    /// Convert to JSONL line
    pub fn to_jsonl(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Usage logger that writes events to JSONL file
#[derive(Clone)]
pub struct UsageLogger {
    inner: Arc<Mutex<UsageLoggerInner>>,
    host_id: String,
}

struct UsageLoggerInner {
    writer: Option<BufWriter<File>>,
    path: Option<PathBuf>,
}

impl UsageLogger {
    /// Create a new usage logger
    pub fn new(host_id: String) -> Self {
        Self {
            inner: Arc::new(Mutex::new(UsageLoggerInner {
                writer: None,
                path: None,
            })),
            host_id,
        }
    }

    /// Initialize file logging to the specified path
    pub async fn init_file(&self, path: PathBuf) -> std::io::Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        let writer = BufWriter::new(file);

        let mut inner = self.inner.lock().await;
        inner.writer = Some(writer);
        inner.path = Some(path.clone());

        info!("Usage logging initialized to {}", path.display());
        Ok(())
    }

    /// Log a usage event
    pub async fn log(&self, event: UsageEvent) {
        let jsonl = match event.to_jsonl() {
            Ok(line) => line,
            Err(e) => {
                error!("Failed to serialize usage event: {}", e);
                return;
            }
        };

        let mut inner = self.inner.lock().await;

        if let Some(ref mut writer) = inner.writer {
            if let Err(e) = writeln!(writer, "{}", jsonl) {
                error!("Failed to write usage event: {}", e);
            }
            // Flush periodically for durability
            if let Err(e) = writer.flush() {
                error!("Failed to flush usage log: {}", e);
            }
        }
    }

    /// Log a connection opened event
    pub async fn log_connection_opened(&self, permission: PermissionLevel, user_id: Option<&str>) {
        let mut event = UsageEvent::new(EventType::ConnectionOpened, self.host_id.clone())
            .with_permission(permission);

        if let Some(uid) = user_id {
            event = event.with_user(uid.to_string());
        }

        self.log(event).await;
    }

    /// Log a connection closed event
    pub async fn log_connection_closed(
        &self,
        permission: PermissionLevel,
        user_id: Option<&str>,
        duration_ms: u64,
    ) {
        let mut event = UsageEvent::new(EventType::ConnectionClosed, self.host_id.clone())
            .with_permission(permission)
            .with_duration(duration_ms);

        if let Some(uid) = user_id {
            event = event.with_user(uid.to_string());
        }

        self.log(event).await;
    }

    /// Log a proxied message
    pub async fn log_message(&self, permission: PermissionLevel, bytes: u64) {
        let event = UsageEvent::new(EventType::MessageProxied, self.host_id.clone())
            .with_permission(permission)
            .with_bytes(bytes);

        self.log(event).await;
    }

    /// Log an admin operation
    pub async fn log_admin_operation(
        &self,
        operation: &str,
        permission: PermissionLevel,
        user_id: Option<&str>,
    ) {
        let mut event = UsageEvent::new(EventType::AdminOperation, self.host_id.clone())
            .with_permission(permission)
            .with_operation(operation.to_string());

        if let Some(uid) = user_id {
            event = event.with_user(uid.to_string());
        }

        self.log(event).await;
    }

    /// Log an authentication attempt
    pub async fn log_auth_attempt(&self, success: bool, user_id: Option<&str>) {
        let mut event = UsageEvent::new(EventType::AuthAttempt, self.host_id.clone());

        if let Some(uid) = user_id {
            event = event.with_user(uid.to_string());
        }

        event.metadata = Some(serde_json::json!({
            "success": success
        }));

        self.log(event).await;
    }

    /// Get the host ID
    pub fn host_id(&self) -> &str {
        &self.host_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = UsageEvent::new(EventType::ConnectionOpened, "test-host".to_string())
            .with_user("user-123".to_string())
            .with_permission(PermissionLevel::Authenticated);

        let jsonl = event.to_jsonl().unwrap();
        assert!(jsonl.contains("connection_opened"));
        assert!(jsonl.contains("user-123"));
        assert!(jsonl.contains("AUTHENTICATED")); // PermissionLevel serializes uppercase
    }

    #[test]
    fn test_message_event() {
        let event = UsageEvent::new(EventType::MessageProxied, "test-host".to_string())
            .with_bytes(1024)
            .with_permission(PermissionLevel::Admin);

        let jsonl = event.to_jsonl().unwrap();
        assert!(jsonl.contains("message_proxied"));
        assert!(jsonl.contains("1024"));
    }

    #[test]
    fn test_admin_operation_event() {
        let event = UsageEvent::new(EventType::AdminOperation, "test-host".to_string())
            .with_operation("list_apps".to_string())
            .with_permission(PermissionLevel::Authenticated);

        let jsonl = event.to_jsonl().unwrap();
        assert!(jsonl.contains("admin_operation"));
        assert!(jsonl.contains("list_apps"));
    }
}
