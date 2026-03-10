//! Rust client SDK for elohim-storage sync API
//!
//! # Example
//!
//! ```rust,no_run
//! use elohim_storage_client::{StorageClient, StorageConfig, AutomergeSync};
//! use automerge::transaction::Transactable;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create client
//! let client = StorageClient::new(StorageConfig {
//!     base_url: "http://localhost:8080".into(),
//!     app_id: "lamad".into(),
//!     ..Default::default()
//! });
//!
//! // List documents
//! let response = client.list_documents(Default::default()).await?;
//!
//! // Use Automerge sync helper
//! let mut sync = AutomergeSync::new(client);
//! let mut doc = sync.load("graph:my-doc").await?;
//!
//! doc.transact::<_, _, automerge::AutomergeError>(|tx| {
//!     tx.put(automerge::ROOT, "title", "Updated")?;
//!     Ok(())
//! })?;
//!
//! sync.save("graph:my-doc", &doc).await?;
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod error;
pub mod sync;
pub mod types;

// Re-export main types
pub use client::StorageClient;
pub use error::{Result, StorageError};
pub use sync::{AutomergeSync, SyncResult};
pub use types::*;
