//! Extraction Cache — disk-backed cache for rendered/extracted content
//!
//! Requires the `native` feature (not available in WASM builds).

mod backend;
mod disk;
mod cache;
mod error;

pub use backend::CacheBackend;
pub use disk::DiskBackend;
pub use cache::{ExtractionCache, ExtractionCacheConfig, AppCacheEntry, ExtractionCacheStats};
pub use error::CacheError;
