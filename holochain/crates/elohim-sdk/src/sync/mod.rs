//! Sync operations for P2P content synchronization
//!
//! Provides Automerge CRDT-based synchronization between P2P nodes.
//! This module is only available when the `sync` feature is enabled.

#[cfg(feature = "sync")]
mod automerge_sync;

#[cfg(feature = "sync")]
pub use automerge_sync::*;

// Re-export from elohim-storage-client when available
#[cfg(all(feature = "sync", feature = "client"))]
pub use elohim_storage_client::{AutomergeSync, SyncResult};
