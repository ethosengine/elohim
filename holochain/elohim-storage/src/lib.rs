//! Elohim Storage - Blob storage sidecar for Elohim nodes
//!
//! Runs alongside Holochain conductor to provide scalable blob storage.
//!
//! ## Architecture
//!
//! - **Holochain DNA**: Stores metadata/provenance (BlobEntry)
//! - **elohim-storage**: Stores actual blob data locally
//! - **P2P Signals**: Request/transfer blobs between nodes
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
//! └── config.toml            # Configuration
//! ```

pub mod blob_store;
pub mod metadata;
pub mod config;
pub mod signals;
pub mod error;
pub mod sharding;
pub mod http;
pub mod import_handler;
pub mod conductor_client;
pub mod import_api;

pub use blob_store::BlobStore;
pub use metadata::MetadataDb;
pub use config::Config;
pub use error::StorageError;
pub use sharding::{ShardEncoder, ShardManifest, ShardConfig};
pub use http::HttpServer;
pub use import_handler::{ImportHandler, ImportHandlerConfig, ImportProgress};
