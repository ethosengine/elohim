//! Elohim Storage - Blob storage sidecar for Elohim nodes
//!
//! Runs alongside Holochain conductor to provide scalable blob storage.
//!
//! ## Architecture
//!
//! - **Holochain DNA**: Stores metadata/provenance (BlobEntry)
//! - **elohim-storage**: Stores actual blob data locally
//! - **P2P Network**: Request/transfer blobs between nodes via rust-libp2p
//!
//! ## Why Separate from Holochain DHT?
//!
//! | Problem with DHT | Solution with Sidecar |
//! |------------------|----------------------|
//! | 16MB entry limit | Unlimited local storage |
//! | 10-50x gossip amplification | Direct P2P transfer |
//! | Memory pressure on conductor | Separate process |
//! | Slow for large files | Optimized streaming |
//!
//! ## Storage Layout
//!
//! ```text
//! ~/.elohim-storage/
//! ├── blobs/                 # Content-addressed blob storage
//! │   ├── sha256-abc123...   # First 2 chars of hash as subdirs
//! │   └── sha256-def456...
//! ├── meta.sled/             # Metadata database
//! ├── identity.key           # libp2p keypair (if p2p feature enabled)
//! └── config.toml            # Configuration
//! ```
//!
//! ## Features
//!
//! - `p2p` - Enable rust-libp2p for P2P shard transfer

// Custom getrandom v0.3 backend for holochain dependencies
// (must be declared before any modules that use randomness)
mod getrandom_custom;

// Protocol enum constants — AUTO-GENERATED from JSON schemas
pub mod generated_enums;

// Enriched API endpoints (controller layer for /api/v1/*)
pub mod api;

// Core modules (always available)
pub mod blob_store;
pub mod conductor; // Custom conductor connection (unsigned - deprecated)
pub mod conductor_client; // Legacy: kept for backward compatibility during migration
pub mod config;
pub mod dag_store;
pub mod db; // SQLite content/path storage
pub mod epr_codec;
pub mod error;
pub mod hc_client;
pub mod http;
pub mod import_handler;
pub mod metadata;
pub mod rea_projection; // REA projection signal handler (DHT → SQLite sync)
pub mod sharding;
pub mod signals; // Official holochain_client wrapper with signing support
pub use hc_client::{ConductorHealth, HcClient, HcClientConfig, NetworkHealth, StorageHealth};
pub mod cell_discovery;
pub mod debug_stream;
pub mod import_api;
pub mod node_registry_api;
pub mod progress_hub;
pub mod progress_ws;
pub mod services;
pub mod sse;

// Tally strategies for multi-mechanism voting
pub mod tally;

// Sensemaking: opinion clustering for governance deliberation
pub mod sensemaking;

// View types for HTTP API responses (camelCase serialization for TypeScript)
pub mod views;

// P2P identity and discovery (always available, but some types require p2p feature)
pub mod content_server;
pub mod identity;

// P2P network modules (require p2p feature)
#[cfg(feature = "p2p")]
pub mod p2p;

// CRDT sync module (always available, P2P transport requires p2p feature)
pub mod sync;

// Sovereignty and cluster modules
#[cfg(feature = "p2p")]
pub mod cluster;
pub mod sovereignty;

// Re-exports
pub use blob_store::BlobStore;
pub use config::Config;
pub use debug_stream::{DebugBroadcaster, DebugEvent};
pub use error::StorageError;
pub use http::HttpServer;
pub use import_handler::{ImportHandler, ImportHandlerConfig, ImportProgress};
pub use metadata::MetadataDb;
pub use node_registry_api::{NodeRegistryApi, ShardAssignment, ShardStatus, ShardingStrategy};
pub use progress_hub::{ProgressHub, ProgressHubConfig, ProgressMessage};
pub use sharding::{ShardConfig, ShardEncoder, ShardManifest};

// P2P re-exports
pub use content_server::{ContentServerBridge, ContentServerConfig, PublisherInfo};
pub use identity::{NodeCapabilities, NodeIdentityInfo};

#[cfg(feature = "p2p")]
pub use identity::NodeIdentity;
#[cfg(feature = "p2p")]
pub use p2p::{P2PConfig, P2PHandle, P2PNode, P2PStatusInfo, RelayMode};

// Sync re-exports
pub use sync::{
    DocStore, DocStoreConfig, StoredDocument, StreamPosition, StreamTracker, SyncManager,
};

// SQLite re-exports
pub use db::DbStats;

// Policy cache re-exports
pub use db::policy_cache::{
    CachedPolicy, ContentMetadata, PolicyCache, PolicyDecision, PolicyEnforcement, PolicyEvent,
    PolicyEventType, TimeAccessDecision, TimeWindow,
};

// Service re-exports
pub use services::{
    ContentService, EventBus, KnowledgeService, PathService, RelationshipService, Services,
    StorageEvent,
};
