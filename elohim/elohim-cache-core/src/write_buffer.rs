//! Write Buffer - Batched write operations with priority queues
//!
//! Protects the conductor from heavy write loads during seeding, sync,
//! and recovery operations. Provides backpressure signaling and retry logic.
//!
//! # Priority Levels
//!
//! 1. **High** - Identity, authentication, critical state (flushed immediately)
//! 2. **Normal** - Regular content updates (batched)
//! 3. **Bulk** - Seeding, imports, recovery sync (heavily batched, throttled)
//!
//! # Example (JavaScript)
//!
//! ```javascript
//! const buffer = new WriteBuffer(50, 100, 3); // batch:50, interval:100ms, retries:3
//!
//! // Queue writes
//! buffer.queue_write('content-123', 'CreateEntry', '{"type":"ContentNode",...}', WritePriority.Bulk);
//! buffer.queue_write('link-456', 'CreateLink', '{"base":"...", "target":"..."}', WritePriority.Bulk);
//!
//! // Check if flush needed
//! if (buffer.should_flush()) {
//!   const batch = buffer.get_pending_batch();
//!   // Send batch to conductor...
//!   // On success:
//!   buffer.mark_batch_committed(batch.batch_id);
//!   // On failure:
//!   buffer.mark_batch_failed(batch.batch_id, 'conductor_unavailable');
//! }
//! ```

use std::collections::{HashMap, VecDeque};
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

use crate::current_time_ms;

// =============================================================================
// Write Priority - Determines queue and flush behavior
// =============================================================================

/// Priority level for write operations.
///
/// Higher priority = flushed sooner, smaller batches.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum WritePriority {
    /// Critical writes: identity, auth, consent - flush immediately
    High = 0,
    /// Normal content updates - batch moderately
    Normal = 1,
    /// Bulk operations: seeding, sync, recovery - batch aggressively
    Bulk = 2,
}

impl Default for WritePriority {
    fn default() -> Self {
        WritePriority::Normal
    }
}

// =============================================================================
// Write Operation Types
// =============================================================================

/// Type of write operation.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum WriteOpType {
    /// Create a new entry
    CreateEntry = 0,
    /// Update an existing entry
    UpdateEntry = 1,
    /// Delete an entry
    DeleteEntry = 2,
    /// Create a link between entries
    CreateLink = 3,
    /// Delete a link
    DeleteLink = 4,
}

// =============================================================================
// Write Operation - Single queued write
// =============================================================================

/// A single write operation waiting to be flushed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteOperation {
    /// Unique operation ID (for deduplication and tracking)
    pub op_id: String,
    /// Type of write operation
    pub op_type: WriteOpType,
    /// Serialized payload (entry data, link data, etc.)
    pub payload: String,
    /// Priority level
    pub priority: WritePriority,
    /// When this operation was queued
    pub queued_at: u64,
    /// Number of retry attempts
    pub retry_count: u8,
    /// Deduplication key (e.g., entry hash for updates)
    pub dedup_key: Option<String>,
}

// =============================================================================
// Batch - Group of operations to flush together
// =============================================================================

/// A batch of write operations ready to send to conductor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteBatch {
    /// Unique batch ID
    pub batch_id: String,
    /// Operations in this batch
    pub operations: Vec<WriteOperation>,
    /// When this batch was created
    pub created_at: u64,
    /// Priority of this batch (highest priority of contained ops)
    pub priority: WritePriority,
}

/// Result of getting a pending batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    /// Whether a batch is available
    pub has_batch: bool,
    /// The batch (if available)
    pub batch: Option<WriteBatch>,
    /// Number of remaining operations across all queues
    pub remaining_count: u32,
}

// =============================================================================
// Buffer Statistics
// =============================================================================

/// Statistics about write buffer state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteBufferStats {
    /// Operations in high priority queue
    pub high_queue_count: u32,
    /// Operations in normal priority queue
    pub normal_queue_count: u32,
    /// Operations in bulk priority queue
    pub bulk_queue_count: u32,
    /// Operations waiting for retry
    pub retry_queue_count: u32,
    /// Total batches flushed
    pub batches_flushed: u64,
    /// Total operations committed
    pub ops_committed: u64,
    /// Total operations failed (after all retries)
    pub ops_failed: u64,
    /// Total operations deduplicated (collapsed)
    pub ops_deduplicated: u64,
    /// Current backpressure level (0-100)
    pub backpressure: u8,
}

// =============================================================================
// Write Buffer - Main buffering engine
// =============================================================================

/// Write buffer with priority queues and batching.
///
/// Protects conductor from heavy write loads by:
/// - Batching operations together
/// - Priority-based flushing (critical writes go first)
/// - Deduplication within batch window
/// - Retry logic with backoff
/// - Backpressure signaling
#[wasm_bindgen]
pub struct WriteBuffer {
    // Priority queues
    high_queue: VecDeque<WriteOperation>,
    normal_queue: VecDeque<WriteOperation>,
    bulk_queue: VecDeque<WriteOperation>,

    // Retry queue (operations that failed, waiting for retry)
    retry_queue: VecDeque<WriteOperation>,

    // Deduplication map: dedup_key -> op_id (last write wins)
    dedup_index: HashMap<String, String>,

    // In-flight batches: batch_id -> batch
    in_flight: HashMap<String, WriteBatch>,

    // Configuration
    batch_size: u32,
    flush_interval_ms: u64,
    max_retries: u8,
    max_queue_size: u32,

    // Timing
    last_flush_at: u64,

    // Statistics
    batches_flushed: u64,
    ops_committed: u64,
    ops_failed: u64,
    ops_deduplicated: u64,

    // Batch ID counter
    next_batch_id: u64,
}

#[wasm_bindgen]
impl WriteBuffer {
    /// Create a new write buffer.
    ///
    /// # Arguments
    /// * `batch_size` - Max operations per batch (default: 50)
    /// * `flush_interval_ms` - Auto-flush interval in ms (default: 100)
    /// * `max_retries` - Max retry attempts for failed ops (default: 3)
    #[wasm_bindgen(constructor)]
    pub fn new(batch_size: u32, flush_interval_ms: u64, max_retries: u8) -> WriteBuffer {
        WriteBuffer {
            high_queue: VecDeque::new(),
            normal_queue: VecDeque::new(),
            bulk_queue: VecDeque::new(),
            retry_queue: VecDeque::new(),
            dedup_index: HashMap::new(),
            in_flight: HashMap::new(),
            batch_size: batch_size.max(1),
            flush_interval_ms,
            max_retries,
            max_queue_size: batch_size * 100, // Default: 100 batches worth
            last_flush_at: current_time_ms(),
            batches_flushed: 0,
            ops_committed: 0,
            ops_failed: 0,
            ops_deduplicated: 0,
            next_batch_id: 0,
        }
    }

    /// Create with default settings optimized for seeding.
    #[wasm_bindgen]
    pub fn for_seeding() -> WriteBuffer {
        WriteBuffer::new(100, 50, 5) // Larger batches, faster flush, more retries
    }

    /// Create with default settings optimized for interactive use.
    #[wasm_bindgen]
    pub fn for_interactive() -> WriteBuffer {
        WriteBuffer::new(20, 100, 3) // Smaller batches, responsive
    }

    /// Create with default settings optimized for recovery/sync.
    #[wasm_bindgen]
    pub fn for_recovery() -> WriteBuffer {
        WriteBuffer::new(200, 25, 10) // Large batches, fast flush, many retries
    }

    /// Set maximum queue size for backpressure.
    #[wasm_bindgen]
    pub fn set_max_queue_size(&mut self, size: u32) {
        self.max_queue_size = size.max(self.batch_size);
    }

    // =========================================================================
    // Queuing Operations
    // =========================================================================

    /// Queue a write operation.
    ///
    /// # Arguments
    /// * `op_id` - Unique operation ID
    /// * `op_type` - Type of operation (CreateEntry, CreateLink, etc.)
    /// * `payload` - Serialized operation data (JSON)
    /// * `priority` - Priority level
    ///
    /// # Returns
    /// `true` if queued, `false` if backpressure is full (caller should wait)
    #[wasm_bindgen]
    pub fn queue_write(
        &mut self,
        op_id: String,
        op_type: WriteOpType,
        payload: String,
        priority: WritePriority,
    ) -> bool {
        self.queue_write_with_dedup(op_id, op_type, payload, priority, None)
    }

    /// Queue a write operation with deduplication key.
    ///
    /// If another operation with the same dedup_key is already queued,
    /// the old one is replaced (last write wins).
    #[wasm_bindgen]
    pub fn queue_write_with_dedup(
        &mut self,
        op_id: String,
        op_type: WriteOpType,
        payload: String,
        priority: WritePriority,
        dedup_key: Option<String>,
    ) -> bool {
        // Check backpressure (but always allow high priority)
        if priority != WritePriority::High && self.total_queued() >= self.max_queue_size {
            return false;
        }

        // Handle deduplication
        if let Some(ref key) = dedup_key {
            if let Some(old_op_id) = self.dedup_index.get(key).cloned() {
                // Remove old operation from its queue
                self.remove_from_queues(&old_op_id);
                self.ops_deduplicated += 1;
            }
            self.dedup_index.insert(key.clone(), op_id.clone());
        }

        let op = WriteOperation {
            op_id,
            op_type,
            payload,
            priority,
            queued_at: current_time_ms(),
            retry_count: 0,
            dedup_key,
        };

        // Add to appropriate queue
        match priority {
            WritePriority::High => self.high_queue.push_back(op),
            WritePriority::Normal => self.normal_queue.push_back(op),
            WritePriority::Bulk => self.bulk_queue.push_back(op),
        }

        true
    }

    /// Remove an operation from queues (for deduplication).
    fn remove_from_queues(&mut self, op_id: &str) {
        self.high_queue.retain(|op| op.op_id != op_id);
        self.normal_queue.retain(|op| op.op_id != op_id);
        self.bulk_queue.retain(|op| op.op_id != op_id);
    }

    // =========================================================================
    // Flushing
    // =========================================================================

    /// Check if buffer should be flushed.
    ///
    /// Returns true if:
    /// - High priority queue has any items
    /// - Any queue exceeds batch size
    /// - Flush interval has elapsed
    #[wasm_bindgen]
    pub fn should_flush(&self) -> bool {
        // High priority always flushes immediately
        if !self.high_queue.is_empty() {
            return true;
        }

        // Check if any queue exceeds batch size
        if self.normal_queue.len() >= self.batch_size as usize
            || self.bulk_queue.len() >= self.batch_size as usize
        {
            return true;
        }

        // Check flush interval
        let now = current_time_ms();
        if now - self.last_flush_at >= self.flush_interval_ms {
            return self.total_queued() > 0;
        }

        // Check retry queue
        !self.retry_queue.is_empty()
    }

    /// Get the next batch of operations to send to conductor.
    ///
    /// Returns JSON with batch info or empty result if no batch ready.
    /// Priority order: High → Retry → Normal → Bulk
    #[wasm_bindgen]
    pub fn get_pending_batch(&mut self) -> String {
        let now = current_time_ms();
        self.last_flush_at = now;

        // Determine which queue to drain
        let (operations, priority) = if !self.high_queue.is_empty() {
            // High priority: drain all
            let ops: Vec<_> = self.high_queue.drain(..).collect();
            (ops, WritePriority::High)
        } else if !self.retry_queue.is_empty() {
            // Retry queue: take up to batch_size
            let count = (self.retry_queue.len() as u32).min(self.batch_size);
            let ops: Vec<_> = self.retry_queue.drain(..count as usize).collect();
            (ops, WritePriority::Normal)
        } else if !self.normal_queue.is_empty() {
            // Normal queue: take up to batch_size
            let count = (self.normal_queue.len() as u32).min(self.batch_size);
            let ops: Vec<_> = self.normal_queue.drain(..count as usize).collect();
            (ops, WritePriority::Normal)
        } else if !self.bulk_queue.is_empty() {
            // Bulk queue: take up to batch_size
            let count = (self.bulk_queue.len() as u32).min(self.batch_size);
            let ops: Vec<_> = self.bulk_queue.drain(..count as usize).collect();
            (ops, WritePriority::Bulk)
        } else {
            // Nothing to flush
            return serde_json::to_string(&BatchResult {
                has_batch: false,
                batch: None,
                remaining_count: 0,
            }).unwrap_or_else(|_| r#"{"has_batch":false}"#.to_string());
        };

        // Clean up dedup index for operations being flushed
        for op in &operations {
            if let Some(ref key) = op.dedup_key {
                self.dedup_index.remove(key);
            }
        }

        // Create batch
        let batch_id = format!("batch-{}", self.next_batch_id);
        self.next_batch_id += 1;

        let batch = WriteBatch {
            batch_id: batch_id.clone(),
            operations,
            created_at: now,
            priority,
        };

        // Track in-flight
        self.in_flight.insert(batch_id, batch.clone());
        self.batches_flushed += 1;

        serde_json::to_string(&BatchResult {
            has_batch: true,
            batch: Some(batch),
            remaining_count: self.total_queued(),
        }).unwrap_or_else(|_| r#"{"has_batch":false}"#.to_string())
    }

    // =========================================================================
    // Batch Result Reporting
    // =========================================================================

    /// Mark a batch as successfully committed.
    ///
    /// Call this after conductor confirms all operations in batch succeeded.
    #[wasm_bindgen]
    pub fn mark_batch_committed(&mut self, batch_id: &str) {
        if let Some(batch) = self.in_flight.remove(batch_id) {
            self.ops_committed += batch.operations.len() as u64;
        }
    }

    /// Mark a batch as failed, queuing operations for retry.
    ///
    /// Operations that haven't exceeded max_retries go to retry queue.
    /// Operations that have exceeded max_retries are dropped (counted as failed).
    #[wasm_bindgen]
    pub fn mark_batch_failed(&mut self, batch_id: &str, _error: &str) {
        if let Some(batch) = self.in_flight.remove(batch_id) {
            for mut op in batch.operations {
                op.retry_count += 1;
                if op.retry_count <= self.max_retries {
                    self.retry_queue.push_back(op);
                } else {
                    self.ops_failed += 1;
                }
            }
        }
    }

    /// Mark specific operations within a batch as failed.
    ///
    /// Use when some operations in a batch succeeded but others failed.
    /// Pass JSON array of failed operation IDs.
    #[wasm_bindgen]
    pub fn mark_operations_failed(&mut self, batch_id: &str, failed_op_ids_json: &str) {
        let failed_ids: Vec<String> = serde_json::from_str(failed_op_ids_json)
            .unwrap_or_default();

        if let Some(batch) = self.in_flight.remove(batch_id) {
            for mut op in batch.operations {
                if failed_ids.contains(&op.op_id) {
                    op.retry_count += 1;
                    if op.retry_count <= self.max_retries {
                        self.retry_queue.push_back(op);
                    } else {
                        self.ops_failed += 1;
                    }
                } else {
                    self.ops_committed += 1;
                }
            }
        }
    }

    // =========================================================================
    // Status and Statistics
    // =========================================================================

    /// Get total number of queued operations.
    #[wasm_bindgen]
    pub fn total_queued(&self) -> u32 {
        (self.high_queue.len() + self.normal_queue.len() +
         self.bulk_queue.len() + self.retry_queue.len()) as u32
    }

    /// Get number of in-flight batches (sent but not confirmed).
    #[wasm_bindgen]
    pub fn in_flight_count(&self) -> u32 {
        self.in_flight.len() as u32
    }

    /// Get current backpressure level (0-100).
    ///
    /// 0 = empty, 100 = full (should pause queuing)
    #[wasm_bindgen]
    pub fn backpressure(&self) -> u8 {
        let ratio = self.total_queued() as f64 / self.max_queue_size as f64;
        (ratio * 100.0).min(100.0) as u8
    }

    /// Check if buffer is under backpressure.
    #[wasm_bindgen]
    pub fn is_backpressured(&self) -> bool {
        self.backpressure() >= 80
    }

    /// Get statistics as JSON.
    #[wasm_bindgen]
    pub fn get_stats(&self) -> String {
        let stats = WriteBufferStats {
            high_queue_count: self.high_queue.len() as u32,
            normal_queue_count: self.normal_queue.len() as u32,
            bulk_queue_count: self.bulk_queue.len() as u32,
            retry_queue_count: self.retry_queue.len() as u32,
            batches_flushed: self.batches_flushed,
            ops_committed: self.ops_committed,
            ops_failed: self.ops_failed,
            ops_deduplicated: self.ops_deduplicated,
            backpressure: self.backpressure(),
        };

        serde_json::to_string(&stats).unwrap_or_else(|_| "{}".to_string())
    }

    /// Reset statistics (but keep queued operations).
    #[wasm_bindgen]
    pub fn reset_stats(&mut self) {
        self.batches_flushed = 0;
        self.ops_committed = 0;
        self.ops_failed = 0;
        self.ops_deduplicated = 0;
    }

    /// Clear all queued operations.
    ///
    /// Warning: This drops all pending writes!
    #[wasm_bindgen]
    pub fn clear(&mut self) {
        self.high_queue.clear();
        self.normal_queue.clear();
        self.bulk_queue.clear();
        self.retry_queue.clear();
        self.dedup_index.clear();
        // Note: in_flight batches are NOT cleared (they're already sent)
    }

    /// Drain all queues and return remaining operations as JSON.
    ///
    /// Use this for graceful shutdown to persist pending writes.
    #[wasm_bindgen]
    pub fn drain_all(&mut self) -> String {
        let mut all_ops: Vec<WriteOperation> = Vec::new();
        all_ops.extend(self.high_queue.drain(..));
        all_ops.extend(self.normal_queue.drain(..));
        all_ops.extend(self.bulk_queue.drain(..));
        all_ops.extend(self.retry_queue.drain(..));
        self.dedup_index.clear();

        serde_json::to_string(&all_ops).unwrap_or_else(|_| "[]".to_string())
    }

    /// Restore operations from JSON (e.g., after restart).
    ///
    /// Use with drain_all() for graceful shutdown/restart.
    #[wasm_bindgen]
    pub fn restore(&mut self, operations_json: &str) {
        let ops: Vec<WriteOperation> = serde_json::from_str(operations_json)
            .unwrap_or_default();

        for op in ops {
            // Restore dedup index
            if let Some(ref key) = op.dedup_key {
                self.dedup_index.insert(key.clone(), op.op_id.clone());
            }

            // Add to appropriate queue
            if op.retry_count > 0 {
                self.retry_queue.push_back(op);
            } else {
                match op.priority {
                    WritePriority::High => self.high_queue.push_back(op),
                    WritePriority::Normal => self.normal_queue.push_back(op),
                    WritePriority::Bulk => self.bulk_queue.push_back(op),
                }
            }
        }
    }
}

impl Default for WriteBuffer {
    fn default() -> Self {
        Self::new(50, 100, 3)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_queuing() {
        let mut buffer = WriteBuffer::new(10, 1000, 3);

        assert!(buffer.queue_write(
            "op1".into(),
            WriteOpType::CreateEntry,
            r#"{"data":"test"}"#.into(),
            WritePriority::Normal,
        ));

        assert_eq!(buffer.total_queued(), 1);
        assert_eq!(buffer.backpressure(), 0); // 1/1000 = ~0%
    }

    #[test]
    fn test_priority_ordering() {
        let mut buffer = WriteBuffer::new(10, 1000, 3);

        // Queue in reverse priority order
        buffer.queue_write("bulk1".into(), WriteOpType::CreateEntry, "{}".into(), WritePriority::Bulk);
        buffer.queue_write("normal1".into(), WriteOpType::CreateEntry, "{}".into(), WritePriority::Normal);
        buffer.queue_write("high1".into(), WriteOpType::CreateEntry, "{}".into(), WritePriority::High);

        // High priority should come out first
        let result: BatchResult = serde_json::from_str(&buffer.get_pending_batch()).unwrap();
        assert!(result.has_batch);
        let batch = result.batch.unwrap();
        assert_eq!(batch.operations[0].op_id, "high1");
        assert_eq!(batch.priority, WritePriority::High);
    }

    #[test]
    fn test_deduplication() {
        let mut buffer = WriteBuffer::new(10, 1000, 3);

        // Queue two writes with same dedup key
        buffer.queue_write_with_dedup(
            "op1".into(),
            WriteOpType::UpdateEntry,
            r#"{"value":1}"#.into(),
            WritePriority::Normal,
            Some("entry-123".into()),
        );
        buffer.queue_write_with_dedup(
            "op2".into(),
            WriteOpType::UpdateEntry,
            r#"{"value":2}"#.into(),
            WritePriority::Normal,
            Some("entry-123".into()),
        );

        // Only one operation should be queued (last write wins)
        assert_eq!(buffer.total_queued(), 1);
        assert_eq!(buffer.ops_deduplicated, 1);

        // Get batch and verify it's the second operation
        let result: BatchResult = serde_json::from_str(&buffer.get_pending_batch()).unwrap();
        let batch = result.batch.unwrap();
        assert_eq!(batch.operations[0].op_id, "op2");
        assert!(batch.operations[0].payload.contains("value\":2"));
    }

    #[test]
    fn test_batch_size_limit() {
        let mut buffer = WriteBuffer::new(5, 1000, 3);

        // Queue 10 operations
        for i in 0..10 {
            buffer.queue_write(
                format!("op{}", i),
                WriteOpType::CreateEntry,
                "{}".into(),
                WritePriority::Normal,
            );
        }

        // First batch should have 5 operations
        let result: BatchResult = serde_json::from_str(&buffer.get_pending_batch()).unwrap();
        let batch = result.batch.unwrap();
        assert_eq!(batch.operations.len(), 5);
        assert_eq!(result.remaining_count, 5);

        // Second batch should have remaining 5
        let result2: BatchResult = serde_json::from_str(&buffer.get_pending_batch()).unwrap();
        let batch2 = result2.batch.unwrap();
        assert_eq!(batch2.operations.len(), 5);
        assert_eq!(result2.remaining_count, 0);
    }

    #[test]
    fn test_retry_logic() {
        let mut buffer = WriteBuffer::new(10, 1000, 2); // max 2 retries

        buffer.queue_write("op1".into(), WriteOpType::CreateEntry, "{}".into(), WritePriority::Normal);

        // Get batch
        let result: BatchResult = serde_json::from_str(&buffer.get_pending_batch()).unwrap();
        let batch = result.batch.unwrap();
        let batch_id = batch.batch_id.clone();

        // Mark as failed
        buffer.mark_batch_failed(&batch_id, "conductor_error");

        // Operation should be in retry queue
        assert_eq!(buffer.retry_queue.len(), 1);
        assert_eq!(buffer.retry_queue[0].retry_count, 1);

        // Get next batch (should be from retry queue)
        let result2: BatchResult = serde_json::from_str(&buffer.get_pending_batch()).unwrap();
        let batch2 = result2.batch.unwrap();

        // Fail again
        buffer.mark_batch_failed(&batch2.batch_id, "conductor_error");
        assert_eq!(buffer.retry_queue[0].retry_count, 2);

        // Get and fail one more time (exceeds max_retries)
        let result3: BatchResult = serde_json::from_str(&buffer.get_pending_batch()).unwrap();
        buffer.mark_batch_failed(&result3.batch.unwrap().batch_id, "conductor_error");

        // Operation should be dropped (failed count incremented)
        assert_eq!(buffer.retry_queue.len(), 0);
        assert_eq!(buffer.ops_failed, 1);
    }

    #[test]
    fn test_backpressure() {
        let mut buffer = WriteBuffer::new(10, 1000, 3);
        buffer.set_max_queue_size(100);

        // Fill to 80%
        for i in 0..80 {
            buffer.queue_write(format!("op{}", i), WriteOpType::CreateEntry, "{}".into(), WritePriority::Bulk);
        }

        assert!(buffer.is_backpressured());
        assert_eq!(buffer.backpressure(), 80);

        // High priority should still be allowed
        assert!(buffer.queue_write("high".into(), WriteOpType::CreateEntry, "{}".into(), WritePriority::High));

        // Bulk should be rejected when at max
        for i in 80..100 {
            buffer.queue_write(format!("op{}", i), WriteOpType::CreateEntry, "{}".into(), WritePriority::Bulk);
        }
        assert!(!buffer.queue_write("rejected".into(), WriteOpType::CreateEntry, "{}".into(), WritePriority::Bulk));
    }

    #[test]
    fn test_drain_and_restore() {
        let mut buffer = WriteBuffer::new(10, 1000, 3);

        buffer.queue_write("op1".into(), WriteOpType::CreateEntry, r#"{"a":1}"#.into(), WritePriority::Normal);
        buffer.queue_write("op2".into(), WriteOpType::CreateLink, r#"{"b":2}"#.into(), WritePriority::Bulk);

        // Drain all
        let drained = buffer.drain_all();
        assert_eq!(buffer.total_queued(), 0);

        // Restore
        buffer.restore(&drained);
        assert_eq!(buffer.total_queued(), 2);

        // Verify operations are in correct queues
        assert_eq!(buffer.normal_queue.len(), 1);
        assert_eq!(buffer.bulk_queue.len(), 1);
    }

    #[test]
    fn test_should_flush() {
        let mut buffer = WriteBuffer::new(10, 100, 3);

        // Empty buffer shouldn't flush
        assert!(!buffer.should_flush());

        // High priority always triggers flush
        buffer.queue_write("high".into(), WriteOpType::CreateEntry, "{}".into(), WritePriority::High);
        assert!(buffer.should_flush());
    }

    #[test]
    fn test_batch_commit() {
        let mut buffer = WriteBuffer::new(10, 1000, 3);

        buffer.queue_write("op1".into(), WriteOpType::CreateEntry, "{}".into(), WritePriority::Normal);
        buffer.queue_write("op2".into(), WriteOpType::CreateEntry, "{}".into(), WritePriority::Normal);

        let result: BatchResult = serde_json::from_str(&buffer.get_pending_batch()).unwrap();
        let batch_id = result.batch.unwrap().batch_id;

        buffer.mark_batch_committed(&batch_id);

        assert_eq!(buffer.ops_committed, 2);
        assert_eq!(buffer.in_flight.len(), 0);
    }

    #[test]
    fn test_partial_batch_failure() {
        let mut buffer = WriteBuffer::new(10, 1000, 3);

        buffer.queue_write("op1".into(), WriteOpType::CreateEntry, "{}".into(), WritePriority::Normal);
        buffer.queue_write("op2".into(), WriteOpType::CreateEntry, "{}".into(), WritePriority::Normal);
        buffer.queue_write("op3".into(), WriteOpType::CreateEntry, "{}".into(), WritePriority::Normal);

        let result: BatchResult = serde_json::from_str(&buffer.get_pending_batch()).unwrap();
        let batch_id = result.batch.unwrap().batch_id;

        // Only op2 failed
        buffer.mark_operations_failed(&batch_id, r#"["op2"]"#);

        assert_eq!(buffer.ops_committed, 2); // op1 and op3
        assert_eq!(buffer.retry_queue.len(), 1); // op2 in retry
        assert_eq!(buffer.retry_queue[0].op_id, "op2");
    }

    #[test]
    fn test_presets() {
        let seeding = WriteBuffer::for_seeding();
        assert_eq!(seeding.batch_size, 100);

        let interactive = WriteBuffer::for_interactive();
        assert_eq!(interactive.batch_size, 20);

        let recovery = WriteBuffer::for_recovery();
        assert_eq!(recovery.batch_size, 200);
    }
}
