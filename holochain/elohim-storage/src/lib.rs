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

// Core modules (always available)
pub mod blob_store;
pub mod metadata;
pub mod config;
pub mod signals;
pub mod error;
pub mod sharding;
pub mod http;
pub mod import_handler;
pub mod conductor;              // New: well-structured conductor connection
pub mod conductor_client;       // Legacy: kept for backward compatibility during migration
pub mod import_api;
pub mod progress_hub;
pub mod progress_ws;
pub mod cell_discovery;
pub mod debug_stream;

// P2P identity and discovery (always available, but some types require p2p feature)
pub mod identity;
pub mod content_server;

// P2P network modules (require p2p feature)
#[cfg(feature = "p2p")]
pub mod p2p;

// Sovereignty and cluster modules
pub mod sovereignty;
#[cfg(feature = "p2p")]
pub mod cluster;

// Re-exports
pub use blob_store::BlobStore;
pub use metadata::MetadataDb;
pub use config::Config;
pub use error::StorageError;
pub use sharding::{ShardEncoder, ShardManifest, ShardConfig};
pub use http::HttpServer;
pub use import_handler::{ImportHandler, ImportHandlerConfig, ImportProgress};
pub use progress_hub::{ProgressHub, ProgressHubConfig, ProgressMessage};
pub use debug_stream::{DebugBroadcaster, DebugEvent};

// P2P re-exports
pub use identity::{NodeCapabilities, NodeIdentityInfo};
pub use content_server::{ContentServerBridge, ContentServerConfig, PublisherInfo};

#[cfg(feature = "p2p")]
pub use identity::NodeIdentity;
