//! Caching primitives for content operations
//!
//! Provides write buffering with backpressure to protect backend services
//! from write storms during bulk operations.

mod write_buffer;

pub use write_buffer::{WriteBuffer, WriteBufferConfig, WriteOp, WritePriority, WriteResult};
