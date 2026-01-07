//! Sync engine - Automerge-based CRDT synchronization
//!
//! Handles:
//! - Stream position tracking (Matrix-inspired)
//! - Document sync between peers
//! - CRDT conflict resolution via Automerge

pub mod stream;
pub mod merge;
pub mod protocol;

// Re-exports
pub use stream::{SyncState, SyncEvent, EventKind};
