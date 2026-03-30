//! Extraction Cache — disk-backed cache for rendered/extracted content
//!
//! Requires the `native` feature (not available in WASM builds).

mod backend;
mod cache;
mod disk;
mod error;

pub use backend::CacheBackend;
pub use cache::{
    AppCacheEntry, ExtractionCache, ExtractionCacheConfig, ExtractionCacheStats, ExtractionGuard,
};
pub use disk::DiskBackend;
pub use error::CacheError;
