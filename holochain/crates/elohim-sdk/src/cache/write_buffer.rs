//! Write buffer with backpressure protection
//!
//! Protects backend services from write storms by:
//! - Batching writes by priority
//! - Deduplicating writes (last-write-wins within batch)
//! - Signaling backpressure when buffer is full
//! - Auto-flushing background task

use crate::error::{Result, SdkError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Write operation priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum WritePriority {
    /// High priority - identity/auth operations, flush immediately
    High = 0,
    /// Normal priority - regular content, moderate batching
    Normal = 1,
    /// Bulk priority - seeding/recovery, aggressive batching
    Bulk = 2,
}

impl Default for WritePriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// A single write operation in the buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteOp {
    /// Content type (e.g., "content", "path")
    pub content_type: String,
    /// Content ID
    pub id: String,
    /// The data to write (JSON)
    pub data: serde_json::Value,
    /// Priority level
    pub priority: WritePriority,
    /// Timestamp when queued (ms since epoch)
    pub queued_at: u64,
}

impl WriteOp {
    pub fn new(
        content_type: impl Into<String>,
        id: impl Into<String>,
        data: serde_json::Value,
        priority: WritePriority,
    ) -> Self {
        Self {
            content_type: content_type.into(),
            id: id.into(),
            data,
            priority,
            queued_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    /// Create a cache key for deduplication
    pub fn cache_key(&self) -> String {
        format!("{}:{}", self.content_type, self.id)
    }
}

/// Result of a flush operation
#[derive(Debug, Clone, Default)]
pub struct WriteResult {
    /// Number of operations successfully written
    pub succeeded: usize,
    /// Number of operations that failed
    pub failed: usize,
    /// Error messages for failed operations
    pub errors: Vec<String>,
}

/// Configuration for the write buffer
#[derive(Debug, Clone)]
pub struct WriteBufferConfig {
    /// Maximum operations in buffer before blocking
    pub max_size: usize,
    /// High watermark for backpressure signaling (0-100)
    pub high_watermark: u8,
    /// Batch size for flush operations
    pub batch_size: usize,
    /// Auto-flush interval in milliseconds
    pub auto_flush_ms: u64,
}

impl Default for WriteBufferConfig {
    fn default() -> Self {
        Self {
            max_size: 1000,
            high_watermark: 80,
            batch_size: 50,
            auto_flush_ms: 1000,
        }
    }
}

impl WriteBufferConfig {
    /// Configuration for interactive use (small batches, fast flush)
    pub fn for_interactive() -> Self {
        Self {
            max_size: 100,
            high_watermark: 50,
            batch_size: 10,
            auto_flush_ms: 100,
        }
    }

    /// Configuration for seeding (large batches, aggressive batching)
    pub fn for_seeding() -> Self {
        Self {
            max_size: 5000,
            high_watermark: 90,
            batch_size: 500,
            auto_flush_ms: 5000,
        }
    }

    /// Configuration for recovery (moderate settings)
    pub fn for_recovery() -> Self {
        Self {
            max_size: 2000,
            high_watermark: 75,
            batch_size: 100,
            auto_flush_ms: 2000,
        }
    }
}

/// Write buffer with backpressure protection
///
/// # Example
///
/// ```rust,ignore
/// use elohim_sdk::{WriteBuffer, WriteOp, WritePriority};
///
/// let buffer = WriteBuffer::new(WriteBufferConfig::default());
///
/// // Queue a write
/// buffer.queue(WriteOp::new(
///     "content",
///     "manifesto",
///     serde_json::json!({"title": "Manifesto"}),
///     WritePriority::Normal,
/// ))?;
///
/// // Check backpressure
/// if buffer.backpressure() > 80 {
///     println!("Buffer getting full, slow down writes");
/// }
///
/// // Flush when ready
/// let result = buffer.flush().await?;
/// ```
pub struct WriteBuffer {
    config: WriteBufferConfig,
    /// Operations by priority, deduplicated by cache_key
    queues: Arc<Mutex<HashMap<WritePriority, HashMap<String, WriteOp>>>>,
}

impl WriteBuffer {
    /// Create a new write buffer with the given configuration
    pub fn new(config: WriteBufferConfig) -> Self {
        let mut queues = HashMap::new();
        queues.insert(WritePriority::High, HashMap::new());
        queues.insert(WritePriority::Normal, HashMap::new());
        queues.insert(WritePriority::Bulk, HashMap::new());

        Self {
            config,
            queues: Arc::new(Mutex::new(queues)),
        }
    }

    /// Create a write buffer with default config
    pub fn default_buffer() -> Self {
        Self::new(WriteBufferConfig::default())
    }

    /// Queue a write operation
    ///
    /// If an operation with the same key exists, it will be replaced (last-write-wins).
    pub async fn queue(&self, op: WriteOp) -> Result<()> {
        let mut queues = self.queues.lock().await;
        let total_size: usize = queues.values().map(|q| q.len()).sum();

        if total_size >= self.config.max_size {
            return Err(SdkError::BackpressureFull(100));
        }

        let key = op.cache_key();
        let priority = op.priority;

        if let Some(queue) = queues.get_mut(&priority) {
            queue.insert(key, op);
        }

        Ok(())
    }

    /// Get current backpressure level (0-100)
    pub async fn backpressure(&self) -> u8 {
        let queues = self.queues.lock().await;
        let total_size: usize = queues.values().map(|q| q.len()).sum();
        let percentage = (total_size as f64 / self.config.max_size as f64 * 100.0) as u8;
        percentage.min(100)
    }

    /// Check if buffer is above high watermark
    pub async fn should_flush(&self) -> bool {
        self.backpressure().await >= self.config.high_watermark
    }

    /// Get pending operations count by priority
    pub async fn pending_counts(&self) -> HashMap<WritePriority, usize> {
        let queues = self.queues.lock().await;
        queues.iter().map(|(k, v)| (*k, v.len())).collect()
    }

    /// Take a batch of operations for flushing (highest priority first)
    pub async fn take_batch(&self) -> Vec<WriteOp> {
        let mut queues = self.queues.lock().await;
        let mut batch = Vec::new();

        // Process by priority: High, Normal, Bulk
        for priority in [WritePriority::High, WritePriority::Normal, WritePriority::Bulk] {
            if let Some(queue) = queues.get_mut(&priority) {
                let keys: Vec<String> = queue.keys().take(self.config.batch_size - batch.len()).cloned().collect();
                for key in keys {
                    if let Some(op) = queue.remove(&key) {
                        batch.push(op);
                    }
                    if batch.len() >= self.config.batch_size {
                        break;
                    }
                }
            }
            if batch.len() >= self.config.batch_size {
                break;
            }
        }

        batch
    }

    /// Clear all pending operations
    pub async fn clear(&self) {
        let mut queues = self.queues.lock().await;
        for queue in queues.values_mut() {
            queue.clear();
        }
    }

    /// Get the buffer configuration
    pub fn config(&self) -> &WriteBufferConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_queue_and_take() {
        let buffer = WriteBuffer::new(WriteBufferConfig::default());

        buffer.queue(WriteOp::new(
            "content",
            "id1",
            serde_json::json!({"title": "Test 1"}),
            WritePriority::Normal,
        )).await.unwrap();

        buffer.queue(WriteOp::new(
            "content",
            "id2",
            serde_json::json!({"title": "Test 2"}),
            WritePriority::High,
        )).await.unwrap();

        let batch = buffer.take_batch().await;
        assert_eq!(batch.len(), 2);

        // High priority should come first
        assert_eq!(batch[0].id, "id2");
    }

    #[tokio::test]
    async fn test_deduplication() {
        let buffer = WriteBuffer::new(WriteBufferConfig::default());

        buffer.queue(WriteOp::new(
            "content",
            "id1",
            serde_json::json!({"title": "Version 1"}),
            WritePriority::Normal,
        )).await.unwrap();

        buffer.queue(WriteOp::new(
            "content",
            "id1",
            serde_json::json!({"title": "Version 2"}),
            WritePriority::Normal,
        )).await.unwrap();

        let batch = buffer.take_batch().await;
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].data["title"], "Version 2"); // Last write wins
    }

    #[tokio::test]
    async fn test_backpressure() {
        let buffer = WriteBuffer::new(WriteBufferConfig {
            max_size: 10,
            high_watermark: 50,
            ..Default::default()
        });

        for i in 0..5 {
            buffer.queue(WriteOp::new(
                "content",
                format!("id{}", i),
                serde_json::json!({}),
                WritePriority::Normal,
            )).await.unwrap();
        }

        assert_eq!(buffer.backpressure().await, 50);
        assert!(buffer.should_flush().await);
    }
}
