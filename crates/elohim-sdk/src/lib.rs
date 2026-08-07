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

/// View types — re-exported from `elohim-views` for the consumer-friendly facade.
///
/// Consumers should prefer `elohim_sdk::views::*` over depending on
/// `elohim-views` directly — this insulates them from any future
/// reorganization of the underlying types crate.
pub mod views {
    pub use elohim_views::*;
}

/// Seam contracts — the concern canon as compile shapes and property harnesses,
/// re-exported from `elohim-seam-contracts`.
///
/// This is the **SDK inheritance surface**: an external peer runtime receives
/// the protocol's hard-won concern contracts instead of re-deriving them one
/// production incident at a time. The two you will reach for first:
///
/// - [`contracts::Answer`] — a three-way boundary answer
///   (`Present` / `Absent` / `Unreachable`). Absence that was *observed* and
///   absence that was never *established* are different facts; collapsing them
///   into `Option<T>` is how a node claims authority over content it merely
///   never received. On a full-arc fleet a local `get` miss is `Unreachable`.
/// - [`contracts::ReasonLabel`] — a closed, countable outcome vocabulary, so
///   every decision increments a labeled counter through a typed reason rather
///   than a raw string.
///
/// The `Arbitrated` and `Quiescent` property harnesses live behind that crate's
/// default-off `harness` feature; enable it in your own `[dev-dependencies]`
/// rather than through the SDK, so nothing links a test harness at runtime.
///
/// Design:
/// `genesis/docs/superpowers/plans/2026-08-02-seam-concern-contract-architecture-plan.md`.
pub mod contracts {
    // Package name is `elohim-seam-contracts`; the LIB name is `seam_contracts`
    // (declared in that crate's `[lib]` table), and the lib name is what a
    // dependent imports. `elohim_seam_contracts::` does not resolve.
    pub use seam_contracts::*;
}

// Re-export core traits
pub use traits::{ContentReadable, ContentWriteable};

// Re-export client types
#[cfg(feature = "client")]
pub use client::{ClientMode, ContentClient};

// Re-export cache types
pub use cache::{WriteBuffer, WriteOp, WritePriority};

// Re-export reach types
pub use reach::{ParseReachLevelError, ReachEnforcer, ReachLevel};

// Re-export error types
pub use error::{Result, SdkError};

// Re-export from underlying crates
pub use doorway_client::{CacheRule, CacheRuleBuilder, CacheSignal, CacheSignalType, Cacheable};

#[cfg(feature = "client")]
pub use elohim_storage_client::{AutomergeSync, StorageClient, StorageConfig, SyncResult};
