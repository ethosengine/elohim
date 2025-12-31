//! Global progress aggregator for import batch streaming
//!
//! The ProgressHub collects progress updates from all active import batches
//! and broadcasts them to WebSocket subscribers.
//!
//! ## Architecture
//!
//! ```text
//! ImportApi (per-batch progress_tx)
//!     │
//!     └─► ProgressHub (global broadcast)
//!              │
//!              └─► WebSocket clients (subscribed to specific batches)
//! ```

use crate::import_api::ImportStatusResponse;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info, warn};

/// Configuration for the progress hub
#[derive(Debug, Clone)]
pub struct ProgressHubConfig {
    /// How long to retain completed batch states (default: 5 minutes)
    pub batch_retention: Duration,
    /// Broadcast channel capacity (default: 256)
    pub channel_capacity: usize,
    /// Heartbeat interval for WebSocket connections (default: 30s)
    pub heartbeat_interval: Duration,
}

impl Default for ProgressHubConfig {
    fn default() -> Self {
        Self {
            batch_retention: Duration::from_secs(300), // 5 minutes
            channel_capacity: 256,
            heartbeat_interval: Duration::from_secs(30),
        }
    }
}

/// Message types sent to WebSocket clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProgressMessage {
    /// Initial state sent on connection/subscribe
    InitialState {
        batches: Vec<BatchState>,
    },
    /// Progress update for a batch
    Progress {
        batch_id: String,
        status: String,
        total_items: u32,
        processed_count: u32,
        error_count: u32,
        items_per_second: f64,
        elapsed_ms: u64,
    },
    /// Batch completed successfully
    Complete {
        batch_id: String,
        status: String,
        total_items: u32,
        processed_count: u32,
        error_count: u32,
        errors: Vec<String>,
        elapsed_ms: u64,
        items_per_second: f64,
    },
    /// Batch failed
    Error {
        batch_id: String,
        message: String,
        errors: Vec<String>,
    },
    /// Periodic heartbeat
    Heartbeat {
        timestamp: String,
    },
}

/// State of a single batch (for initial state)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchState {
    pub batch_id: String,
    pub batch_type: String,
    pub status: String,
    pub total_items: u32,
    pub processed_count: u32,
    pub error_count: u32,
    pub elapsed_ms: u64,
    pub items_per_second: f64,
}

/// Internal batch tracking
struct TrackedBatch {
    batch_id: String,
    batch_type: String,
    status: String,
    total_items: u32,
    processed_count: u32,
    error_count: u32,
    errors: Vec<String>,
    started_at: Instant,
    completed_at: Option<Instant>,
}

impl TrackedBatch {
    fn to_state(&self) -> BatchState {
        let elapsed = self.started_at.elapsed();
        let items_per_second = if elapsed.as_secs_f64() > 0.0 {
            self.processed_count as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        BatchState {
            batch_id: self.batch_id.clone(),
            batch_type: self.batch_type.clone(),
            status: self.status.clone(),
            total_items: self.total_items,
            processed_count: self.processed_count,
            error_count: self.error_count,
            elapsed_ms: elapsed.as_millis() as u64,
            items_per_second,
        }
    }

    fn to_progress_message(&self) -> ProgressMessage {
        let elapsed = self.started_at.elapsed();
        let items_per_second = if elapsed.as_secs_f64() > 0.0 {
            self.processed_count as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        if self.status == "completed" || self.status == "completedwitherrors" {
            ProgressMessage::Complete {
                batch_id: self.batch_id.clone(),
                status: self.status.clone(),
                total_items: self.total_items,
                processed_count: self.processed_count,
                error_count: self.error_count,
                errors: self.errors.clone(),
                elapsed_ms: elapsed.as_millis() as u64,
                items_per_second,
            }
        } else if self.status == "failed" {
            ProgressMessage::Error {
                batch_id: self.batch_id.clone(),
                message: "Import batch failed".to_string(),
                errors: self.errors.clone(),
            }
        } else {
            ProgressMessage::Progress {
                batch_id: self.batch_id.clone(),
                status: self.status.clone(),
                total_items: self.total_items,
                processed_count: self.processed_count,
                error_count: self.error_count,
                items_per_second,
                elapsed_ms: elapsed.as_millis() as u64,
            }
        }
    }
}

/// Global progress hub for broadcasting import progress
pub struct ProgressHub {
    config: ProgressHubConfig,
    /// Broadcast channel for progress updates
    progress_tx: broadcast::Sender<ProgressMessage>,
    /// Active and recently completed batches
    batches: Arc<RwLock<HashMap<String, TrackedBatch>>>,
}

impl ProgressHub {
    /// Create a new progress hub
    pub fn new(config: ProgressHubConfig) -> Self {
        let (progress_tx, _) = broadcast::channel(config.channel_capacity);

        let hub = Self {
            config,
            progress_tx,
            batches: Arc::new(RwLock::new(HashMap::new())),
        };

        // Start cleanup task for expired batches
        hub.start_cleanup_task();

        info!("ProgressHub initialized");
        hub
    }

    /// Subscribe to progress updates
    pub fn subscribe(&self) -> broadcast::Receiver<ProgressMessage> {
        self.progress_tx.subscribe()
    }

    /// Get current state of all tracked batches
    pub async fn get_batch_states(&self) -> Vec<BatchState> {
        let batches = self.batches.read().await;
        batches.values().map(|b| b.to_state()).collect()
    }

    /// Get state of specific batches
    pub async fn get_batch_states_filtered(&self, batch_ids: &[String]) -> Vec<BatchState> {
        let batches = self.batches.read().await;
        batch_ids
            .iter()
            .filter_map(|id| batches.get(id).map(|b| b.to_state()))
            .collect()
    }

    /// Register a new batch (called when import is queued)
    pub async fn register_batch(
        &self,
        batch_id: &str,
        batch_type: &str,
        total_items: u32,
    ) {
        let mut batches = self.batches.write().await;
        batches.insert(
            batch_id.to_string(),
            TrackedBatch {
                batch_id: batch_id.to_string(),
                batch_type: batch_type.to_string(),
                status: "queued".to_string(),
                total_items,
                processed_count: 0,
                error_count: 0,
                errors: Vec::new(),
                started_at: Instant::now(),
                completed_at: None,
            },
        );

        debug!(batch_id = %batch_id, total_items = total_items, "Batch registered with ProgressHub");
    }

    /// Update batch progress (called from ImportApi)
    pub async fn update_progress(&self, status: &ImportStatusResponse) {
        let message = {
            let mut batches = self.batches.write().await;

            if let Some(batch) = batches.get_mut(&status.batch_id) {
                // Update tracked state
                batch.status = format!("{:?}", status.status).to_lowercase();
                batch.processed_count = status.processed_count;
                batch.error_count = status.error_count;
                batch.total_items = status.total_items;

                // Merge errors (keep last N)
                for err in &status.errors {
                    if !batch.errors.contains(err) && batch.errors.len() < 100 {
                        batch.errors.push(err.clone());
                    }
                }

                // Check if completed
                let is_complete = matches!(
                    batch.status.as_str(),
                    "completed" | "completedwitherrors" | "failed"
                );
                if is_complete && batch.completed_at.is_none() {
                    batch.completed_at = Some(Instant::now());
                }

                batch.to_progress_message()
            } else {
                // Batch not registered - create ad-hoc message
                let elapsed_secs = status.elapsed_ms as f64 / 1000.0;
                let items_per_second = if elapsed_secs > 0.0 {
                    status.processed_count as f64 / elapsed_secs
                } else {
                    0.0
                };

                ProgressMessage::Progress {
                    batch_id: status.batch_id.clone(),
                    status: format!("{:?}", status.status).to_lowercase(),
                    total_items: status.total_items,
                    processed_count: status.processed_count,
                    error_count: status.error_count,
                    items_per_second,
                    elapsed_ms: status.elapsed_ms,
                }
            }
        };

        // Broadcast to all subscribers
        if self.progress_tx.receiver_count() > 0 {
            if let Err(e) = self.progress_tx.send(message) {
                warn!(error = %e, "Failed to broadcast progress update");
            }
        }
    }

    /// Broadcast a heartbeat to all subscribers
    pub fn send_heartbeat(&self) {
        if self.progress_tx.receiver_count() > 0 {
            let message = ProgressMessage::Heartbeat {
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            let _ = self.progress_tx.send(message);
        }
    }

    /// Get heartbeat interval
    pub fn heartbeat_interval(&self) -> Duration {
        self.config.heartbeat_interval
    }

    /// Start background task to clean up expired batches
    fn start_cleanup_task(&self) {
        let batches = Arc::clone(&self.batches);
        let retention = self.config.batch_retention;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));

            loop {
                interval.tick().await;

                let mut batches = batches.write().await;
                let now = Instant::now();
                let before_count = batches.len();

                batches.retain(|_, batch| {
                    match batch.completed_at {
                        Some(completed) => now.duration_since(completed) < retention,
                        None => true, // Keep in-progress batches
                    }
                });

                let removed = before_count - batches.len();
                if removed > 0 {
                    debug!(removed = removed, "Cleaned up expired batches from ProgressHub");
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_progress_hub_basic() {
        let hub = ProgressHub::new(ProgressHubConfig::default());

        // Register a batch
        hub.register_batch("test-batch-1", "content", 100).await;

        // Get initial state
        let states = hub.get_batch_states().await;
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].batch_id, "test-batch-1");
        assert_eq!(states[0].total_items, 100);
    }

    #[tokio::test]
    async fn test_progress_message_serialization() {
        let msg = ProgressMessage::Progress {
            batch_id: "test-1".to_string(),
            status: "processing".to_string(),
            total_items: 100,
            processed_count: 50,
            error_count: 0,
            items_per_second: 25.5,
            elapsed_ms: 2000,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"progress\""));
        assert!(json.contains("\"batch_id\":\"test-1\""));
    }
}
