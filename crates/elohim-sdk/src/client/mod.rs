//! Content client with mode-aware resolution
//!
//! Provides unified content access across different deployment modes:
//! - Browser: via Doorway → Projection Store
//! - Native: via local SQLite
//! - Node: via local SQLite with P2P sync

mod content_client;
mod projection_warmer;

pub use content_client::{ClientMode, ContentClient};
pub use projection_warmer::ProjectionWarmer;
