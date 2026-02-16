//! Sync engine - Automerge-based CRDT synchronization
//!
//! Handles:
//! - Stream position tracking (Matrix-inspired)
//! - Document sync between peers
//! - CRDT conflict resolution via Automerge
//! - Sync coordination across multiple peers

pub mod stream;
pub mod merge;
pub mod protocol;
pub mod coordinator;

// Re-exports
pub use stream::{SyncState, SyncEvent, EventKind};
pub use merge::SyncEngine;
pub use coordinator::SyncCoordinator;
