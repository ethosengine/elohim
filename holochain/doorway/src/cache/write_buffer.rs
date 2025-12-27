//! Write Buffer - Batched conductor writes with priority queues
//!
//! Wraps holochain-cache-core's WriteBuffer to protect the conductor from
//! heavy write loads during seeding, sync, and recovery operations.
//!
//! ## Purpose
//!
//! When doorway receives bulk writes (e.g., seeding, recovery, sync),
//! individual zome calls would overwhelm the conductor. This module batches
//! writes together and flushes them with proper priority ordering.
//!
//! ## Priority Levels
//!
//! 1. **High** - Identity, authentication, consent (flushed immediately)
//! 2. **Normal** - Regular content updates (batched moderately)
//! 3. **Bulk** - Seeding, imports, recovery (heavily batched, throttled)
//!
//! ## Usage
//!
//! ```rust,ignore
//! let buffer = DoorwayWriteBuffer::new(worker_pool);
//!
//! // Queue writes
//! buffer.queue_content_create("content-123", content_json, WritePriority::Bulk);
//! buffer.queue_link_create("link-456", link_json, WritePriority::Bulk);
//!
//! // Auto-flush runs in background, or force flush:
//! buffer.flush_all().await?;
//! ```

use std::sync::Arc;
use std::time::Duration;
use holochain_cache_core::{WriteBuffer, WriteBatch, WriteBufferStats, WriteOperation, BatchResult};
pub use holochain_cache_core::{WritePriority, WriteOpType};
use serde::Serialize;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::worker::{WorkerPool, ZomeCallBuilder, ZomeCallConfig};
use crate::types::{DoorwayError, Result};

// =============================================================================
// Buffer Configuration
// =============================================================================

/// Configuration for doorway write buffer
#[derive(Debug, Clone)]
pub struct WriteBufferConfig {
    /// Batch size before flush
    pub batch_size: u32,
    /// Auto-flush interval in milliseconds
    pub flush_interval_ms: u64,
    /// Max retry attempts for failed writes
    pub max_retries: u8,
    /// Max queue size before backpressure
    pub max_queue_size: u32,
}

impl Default for WriteBufferConfig {
    fn default() -> Self {
        Self {
            batch_size: 50,
            flush_interval_ms: 100,
            max_retries: 3,
            max_queue_size: 5000,
        }
    }
}

impl WriteBufferConfig {
    /// Config optimized for bulk seeding operations
    pub fn for_seeding() -> Self {
        Self {
            batch_size: 100,
            flush_interval_ms: 50,
            max_retries: 5,
            max_queue_size: 10000,
        }
    }

    /// Config optimized for interactive use
    pub fn for_interactive() -> Self {
        Self {
            batch_size: 20,
            flush_interval_ms: 100,
            max_retries: 3,
            max_queue_size: 1000,
        }
    }

    /// Config optimized for recovery/sync
    pub fn for_recovery() -> Self {
        Self {
            batch_size: 200,
            flush_interval_ms: 25,
            max_retries: 10,
            max_queue_size: 50000,
        }
    }
}

// =============================================================================
// Flush Result
// =============================================================================

/// Result of a flush operation
#[derive(Debug, Clone, Serialize)]
pub struct FlushResult {
    /// Number of operations committed
    pub committed: u64,
    /// Number of operations failed
    pub failed: u64,
    /// Number of batches processed
    pub batches: u64,
    /// Duration in milliseconds
    pub duration_ms: f64,
}

// =============================================================================
// Doorway Write Buffer
// =============================================================================

/// Write buffer for doorway conductor operations.
///
/// Provides batching, priority queuing, and backpressure for writes.
pub struct DoorwayWriteBuffer {
    /// holochain-cache-core write buffer
    buffer: RwLock<WriteBuffer>,
    /// Worker pool for conductor requests
    worker_pool: Arc<WorkerPool>,
    /// Configuration
    config: WriteBufferConfig,
    /// Zome call configuration (discovered at runtime)
    zome_config: RwLock<Option<ZomeCallConfig>>,
    /// Whether auto-flush is running
    auto_flush_running: std::sync::atomic::AtomicBool,
}

impl DoorwayWriteBuffer {
    /// Create a new write buffer with the given worker pool.
    pub fn new(worker_pool: Arc<WorkerPool>, config: WriteBufferConfig) -> Self {
        let buffer = WriteBuffer::new(
            config.batch_size,
            config.flush_interval_ms,
            config.max_retries,
        );

        info!(
            batch_size = config.batch_size,
            flush_interval_ms = config.flush_interval_ms,
            max_queue_size = config.max_queue_size,
            "DoorwayWriteBuffer initialized"
        );

        Self {
            buffer: RwLock::new(buffer),
            worker_pool,
            config,
            zome_config: RwLock::new(None),
            auto_flush_running: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Create with default configuration.
    pub fn with_defaults(worker_pool: Arc<WorkerPool>) -> Self {
        Self::new(worker_pool, WriteBufferConfig::default())
    }

    /// Create optimized for seeding.
    pub fn for_seeding(worker_pool: Arc<WorkerPool>) -> Self {
        Self::new(worker_pool, WriteBufferConfig::for_seeding())
    }

    /// Create optimized for recovery.
    pub fn for_recovery(worker_pool: Arc<WorkerPool>) -> Self {
        Self::new(worker_pool, WriteBufferConfig::for_recovery())
    }

    /// Set the zome call configuration.
    ///
    /// Call this after discovering cell info from the conductor.
    pub async fn set_zome_config(&self, config: ZomeCallConfig) {
        info!(
            dna_hash = config.dna_hash,
            zome = config.zome_name,
            "Zome config set for write buffer"
        );
        let mut zome_config = self.zome_config.write().await;
        *zome_config = Some(config);
    }

    /// Check if zome config is available.
    pub async fn has_zome_config(&self) -> bool {
        self.zome_config.read().await.is_some()
    }

    // =========================================================================
    // Queuing Operations
    // =========================================================================

    /// Queue a content creation.
    pub async fn queue_content_create(
        &self,
        content_id: &str,
        payload: &str,
        priority: WritePriority,
    ) -> Result<bool> {
        let mut buffer = self.buffer.write().await;
        let queued = buffer.queue_write(
            format!("content:{}", content_id),
            WriteOpType::CreateEntry,
            payload.to_string(),
            priority,
        );

        if !queued {
            warn!(content_id = content_id, "Write rejected due to backpressure");
        }

        Ok(queued)
    }

    /// Queue a content update with deduplication.
    pub async fn queue_content_update(
        &self,
        content_id: &str,
        entry_hash: &str,
        payload: &str,
        priority: WritePriority,
    ) -> Result<bool> {
        let mut buffer = self.buffer.write().await;
        let queued = buffer.queue_write_with_dedup(
            format!("content:{}", content_id),
            WriteOpType::UpdateEntry,
            payload.to_string(),
            priority,
            Some(entry_hash.to_string()),
        );

        Ok(queued)
    }

    /// Queue a path creation.
    pub async fn queue_path_create(
        &self,
        path_id: &str,
        payload: &str,
        priority: WritePriority,
    ) -> Result<bool> {
        let mut buffer = self.buffer.write().await;
        let queued = buffer.queue_write(
            format!("path:{}", path_id),
            WriteOpType::CreateEntry,
            payload.to_string(),
            priority,
        );

        Ok(queued)
    }

    /// Queue a link creation.
    pub async fn queue_link_create(
        &self,
        link_id: &str,
        payload: &str,
        priority: WritePriority,
    ) -> Result<bool> {
        let mut buffer = self.buffer.write().await;
        let queued = buffer.queue_write(
            format!("link:{}", link_id),
            WriteOpType::CreateLink,
            payload.to_string(),
            priority,
        );

        Ok(queued)
    }

    /// Queue a high-priority identity/auth write.
    pub async fn queue_identity_write(
        &self,
        op_id: &str,
        op_type: WriteOpType,
        payload: &str,
    ) -> Result<bool> {
        let mut buffer = self.buffer.write().await;
        // High priority always succeeds (bypasses backpressure)
        let queued = buffer.queue_write(
            op_id.to_string(),
            op_type,
            payload.to_string(),
            WritePriority::High,
        );

        Ok(queued)
    }

    // =========================================================================
    // Flushing
    // =========================================================================

    /// Check if buffer should be flushed.
    pub async fn should_flush(&self) -> bool {
        let buffer = self.buffer.read().await;
        buffer.should_flush()
    }

    /// Flush a single batch to the conductor.
    pub async fn flush_batch(&self) -> Result<Option<FlushResult>> {
        let start = std::time::Instant::now();

        // Get next batch
        let batch_json = {
            let mut buffer = self.buffer.write().await;
            buffer.get_pending_batch()
        };

        let batch_result: BatchResult =
            serde_json::from_str(&batch_json).map_err(|e| {
                DoorwayError::Internal(format!("Failed to parse batch result: {}", e))
            })?;

        if !batch_result.has_batch {
            return Ok(None);
        }

        let batch = batch_result.batch.ok_or_else(|| {
            DoorwayError::Internal("Batch result missing batch data".into())
        })?;

        debug!(
            batch_id = batch.batch_id,
            op_count = batch.operations.len(),
            "Flushing batch to conductor"
        );

        // Send batch to conductor
        let result = self.send_batch_to_conductor(&batch).await;

        // Update buffer with result
        {
            let mut buffer = self.buffer.write().await;
            match &result {
                Ok(()) => {
                    buffer.mark_batch_committed(&batch.batch_id);
                }
                Err(e) => {
                    buffer.mark_batch_failed(&batch.batch_id, &e.to_string());
                }
            }
        }

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

        match result {
            Ok(()) => Ok(Some(FlushResult {
                committed: batch.operations.len() as u64,
                failed: 0,
                batches: 1,
                duration_ms,
            })),
            Err(e) => {
                error!(batch_id = batch.batch_id, error = ?e, "Batch flush failed");
                Ok(Some(FlushResult {
                    committed: 0,
                    failed: batch.operations.len() as u64,
                    batches: 1,
                    duration_ms,
                }))
            }
        }
    }

    /// Flush all pending batches.
    pub async fn flush_all(&self) -> Result<FlushResult> {
        let start = std::time::Instant::now();
        let mut total_committed = 0u64;
        let mut total_failed = 0u64;
        let mut batch_count = 0u64;

        loop {
            if !self.should_flush().await {
                break;
            }

            match self.flush_batch().await? {
                Some(result) => {
                    total_committed += result.committed;
                    total_failed += result.failed;
                    batch_count += 1;
                }
                None => break,
            }

            // Small delay between batches
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

        info!(
            committed = total_committed,
            failed = total_failed,
            batches = batch_count,
            duration_ms = duration_ms,
            "Flush all completed"
        );

        Ok(FlushResult {
            committed: total_committed,
            failed: total_failed,
            batches: batch_count,
            duration_ms,
        })
    }

    /// Send a batch to the conductor.
    async fn send_batch_to_conductor(&self, batch: &WriteBatch) -> Result<()> {
        // Check if zome config is available
        if !self.has_zome_config().await {
            return Err(DoorwayError::Internal(
                "Cannot send batch: zome config not set".into()
            ));
        }

        // TODO: Optimize with batch zome function (__doorway_batch)
        // For now, send operations individually
        for op in &batch.operations {
            // Build zome call payload based on operation type
            let payload = self.build_zome_payload(op).await?;

            // Send to conductor via worker pool
            match self.worker_pool.request(payload).await {
                Ok(_) => {
                    debug!(op_id = op.op_id, "Operation committed");
                }
                Err(e) => {
                    error!(op_id = op.op_id, error = ?e, "Operation failed");
                    // For now, fail the entire batch on any error
                    // TODO: Track partial failures
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// Build zome call payload for an operation.
    ///
    /// Parses the operation and builds a MessagePack payload for the conductor.
    /// The op_id format is "{doc_type}:{id}" (e.g., "content:manifesto").
    async fn build_zome_payload(&self, op: &WriteOperation) -> Result<Vec<u8>> {
        // Get zome config
        let zome_config_guard = self.zome_config.read().await;
        let zome_config = zome_config_guard.as_ref().ok_or_else(|| {
            DoorwayError::Internal("Zome config not set - cannot build payload".into())
        })?;

        let builder = ZomeCallBuilder::new(zome_config.clone());

        // Parse op_id to extract doc_type and id (format: "doc_type:id")
        let (doc_type, id) = op.op_id.split_once(':').ok_or_else(|| {
            DoorwayError::Internal(format!("Invalid op_id format: {}", op.op_id))
        })?;

        // Map WriteOpType to operation string
        let op_type = match op.op_type {
            WriteOpType::CreateEntry => "create",
            WriteOpType::UpdateEntry => "update",
            WriteOpType::DeleteEntry => "delete",
            WriteOpType::CreateLink => "create_link",
            WriteOpType::DeleteLink => "delete_link",
        };

        // Parse payload JSON (for create/update)
        let data = if op.op_type == WriteOpType::DeleteEntry || op.op_type == WriteOpType::DeleteLink {
            None
        } else {
            Some(serde_json::from_str(&op.payload).map_err(|e| {
                DoorwayError::Internal(format!("Failed to parse payload JSON: {}", e))
            })?)
        };

        // Build the zome call
        builder.build_doorway_write(doc_type, op_type, id, data, op.dedup_key.clone())
    }

    // =========================================================================
    // Auto-Flush Background Task
    // =========================================================================

    /// Start auto-flush background task.
    ///
    /// Spawns a task that periodically checks and flushes the buffer.
    pub fn start_auto_flush(self: &Arc<Self>) {
        if self.auto_flush_running.swap(true, std::sync::atomic::Ordering::SeqCst) {
            warn!("Auto-flush already running");
            return;
        }

        let buffer = Arc::clone(self);
        let interval_ms = self.config.flush_interval_ms;

        tokio::spawn(async move {
            info!(interval_ms = interval_ms, "Auto-flush task started");

            loop {
                tokio::time::sleep(Duration::from_millis(interval_ms)).await;

                if !buffer.auto_flush_running.load(std::sync::atomic::Ordering::SeqCst) {
                    info!("Auto-flush task stopping");
                    break;
                }

                if buffer.should_flush().await {
                    match buffer.flush_batch().await {
                        Ok(Some(result)) => {
                            debug!(
                                committed = result.committed,
                                failed = result.failed,
                                "Auto-flush batch completed"
                            );
                        }
                        Ok(None) => {}
                        Err(e) => {
                            error!(error = ?e, "Auto-flush batch failed");
                        }
                    }
                }
            }
        });
    }

    /// Stop auto-flush background task.
    pub fn stop_auto_flush(&self) {
        self.auto_flush_running.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    // =========================================================================
    // Status and Statistics
    // =========================================================================

    /// Get total queued operations.
    pub async fn total_queued(&self) -> u32 {
        let buffer = self.buffer.read().await;
        buffer.total_queued()
    }

    /// Get in-flight batch count.
    pub async fn in_flight_count(&self) -> u32 {
        let buffer = self.buffer.read().await;
        buffer.in_flight_count()
    }

    /// Get current backpressure level (0-100).
    pub async fn backpressure(&self) -> u8 {
        let buffer = self.buffer.read().await;
        buffer.backpressure()
    }

    /// Check if under backpressure.
    pub async fn is_backpressured(&self) -> bool {
        let buffer = self.buffer.read().await;
        buffer.is_backpressured()
    }

    /// Get buffer statistics.
    pub async fn get_stats(&self) -> WriteBufferStats {
        let buffer = self.buffer.read().await;
        let stats_json = buffer.get_stats();
        serde_json::from_str(&stats_json).unwrap_or_else(|_| WriteBufferStats {
            high_queue_count: 0,
            normal_queue_count: 0,
            bulk_queue_count: 0,
            retry_queue_count: 0,
            batches_flushed: 0,
            ops_committed: 0,
            ops_failed: 0,
            ops_deduplicated: 0,
            backpressure: 0,
        })
    }

    /// Clear all queued operations.
    ///
    /// Warning: This drops all pending writes!
    pub async fn clear(&self) {
        let mut buffer = self.buffer.write().await;
        buffer.clear();
        warn!("Write buffer cleared - all pending writes dropped");
    }

    /// Drain all queued operations for graceful shutdown.
    pub async fn drain_all(&self) -> String {
        let mut buffer = self.buffer.write().await;
        buffer.drain_all()
    }

    /// Restore operations from a previous drain.
    pub async fn restore(&self, operations_json: &str) {
        let mut buffer = self.buffer.write().await;
        buffer.restore(operations_json);
        info!("Write buffer restored");
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = WriteBufferConfig::default();
        assert_eq!(config.batch_size, 50);
        assert_eq!(config.flush_interval_ms, 100);
    }

    #[test]
    fn test_config_presets() {
        let seeding = WriteBufferConfig::for_seeding();
        assert_eq!(seeding.batch_size, 100);

        let recovery = WriteBufferConfig::for_recovery();
        assert_eq!(recovery.batch_size, 200);

        let interactive = WriteBufferConfig::for_interactive();
        assert_eq!(interactive.batch_size, 20);
    }
}
