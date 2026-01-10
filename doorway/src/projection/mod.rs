//! Projection Engine for Doorway
//!
//! Projects DHT entries into MongoDB for fast reads. The projection layer
//! provides a one-way data flow from Holochain → MongoDB → Clients.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    ProjectionStore                       │
//! ├─────────────────────────────────────────────────────────┤
//! │  ┌──────────────────┐    ┌──────────────────────────┐   │
//! │  │   HotCache       │◄───│    MongoDB Collections   │   │
//! │  │   (DashMap)      │    │                          │   │
//! │  │   - LRU eviction │    │  - projected_entries     │   │
//! │  │   - 10k entries  │    │  - projected_content     │   │
//! │  │   - Fast reads   │    │  - projected_paths       │   │
//! │  └──────────────────┘    └──────────────────────────┘   │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```ignore
//! let store = ProjectionStore::new(mongo_client).await?;
//!
//! // Project an entry
//! store.set("content", doc).await?;
//!
//! // Read from projection (hot cache → MongoDB)
//! let content = store.get("content", "content-abc123").await;
//!
//! // Query with filters
//! let results = store.query("content", ContentQuery::by_type("concept")).await;
//! ```

pub mod app_auth;
pub mod collections;
pub mod document;
pub mod engine;
pub mod store;
pub mod subscriber;

// Re-export main types
pub use document::{ProjectedDocument, ProjectionQuery};
pub use engine::{EngineConfig, ProjectionEngine, ProjectionSignal, spawn_engine_task};
pub use store::{ProjectionConfig, ProjectionStore};
pub use subscriber::{ContentServerRegistration, SignalSubscriber, SubscriberConfig, spawn_subscriber};
