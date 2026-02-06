//! Elohim SDK - P2P Application Development Kit
//!
//! SDK for building offline-first P2P applications on the Elohim Protocol.
//!
//! # Architecture
//!
//! This SDK provides mode-aware content access that works across:
//! - **Browser**: Doorway → Projection Store (no offline, doorway-dependent)
//! - **Native (Tauri)**: Local SQLite → P2P sync (full offline)
//! - **Node (elohim-node)**: Local SQLite → P2P sync → serves to doorways
//!
//! # Phase A: No DHT
//!
//! This version uses SQLite/Projection as the authority for content.
//! Holochain DHT will be added in Phase B for agent-centric data only
//! (attestations, identity, points, consent).
//!
//! # Example
//!
//! ```rust,ignore
//! use elohim_sdk::{ContentClient, ClientMode, ContentReadable};
//!
//! // Browser mode - uses doorway projection
//! let client = ContentClient::new(ClientMode::Browser {
//!     doorway_url: "https://doorway.example.com".into(),
//! });
//!
//! // Get content
//! let content = client.get::<Content>("manifesto").await?;
//!
//! // Native mode - uses local SQLite
//! let client = ContentClient::new(ClientMode::Native {
//!     storage_path: "/data/elohim".into(),
//! });
//!
//! // Same API, different backend
//! let content = client.get::<Content>("manifesto").await?;
//! ```

// Core traits for content types
pub mod traits;

// Content client with mode-aware resolution
#[cfg(feature = "client")]
pub mod client;

// Caching primitives
pub mod cache;

// Sync operations (Automerge CRDT)
#[cfg(feature = "sync")]
pub mod sync;

// Reach-level access control
pub mod reach;

// Error types
pub mod error;

// Re-export core traits
pub use traits::{ContentReadable, ContentWriteable, Syncable};

// Re-export client types
#[cfg(feature = "client")]
pub use client::{ContentClient, ClientMode};

// Re-export cache types
pub use cache::{WriteBuffer, WritePriority, WriteOp};

// Re-export reach types
pub use reach::{ReachLevel, ReachEnforcer};

// Re-export error types
pub use error::{SdkError, Result};

// Re-export from underlying crates
pub use doorway_client::{
    Cacheable, CacheSignal, CacheSignalType, CacheRule, CacheRuleBuilder,
};

#[cfg(feature = "client")]
pub use elohim_storage_client::{
    StorageClient, StorageConfig, AutomergeSync, SyncResult,
};
