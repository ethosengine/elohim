//! Sync engine - Automerge-based CRDT synchronization
//!
//! Handles:
//! - Stream position tracking (Matrix-inspired)
//! - Document sync between peers
//! - CRDT conflict resolution via Automerge
//! - Sync coordination across multiple peers

pub mod coordinator;
pub mod merge;
pub mod protocol;
pub mod stream;

// Re-exports
pub use coordinator::SyncCoordinator;
pub use merge::SyncEngine;
pub use stream::SyncEvent;
