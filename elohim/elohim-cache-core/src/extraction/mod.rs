//! Extraction Cache — disk-backed cache for rendered/extracted content
//!
//! The CACHE requires the `native` feature (tokio; not available in WASM
//! builds). Its CONFIG does not, and is always available: `ExtractionCacheConfig`
//! is a field of the storage node's boot `Config`, which lives in
//! `elohim-settings` — a crate that is runtime-free by construction and whose
//! lockfile boundary test denies `tokio`, so it cannot take `native` to reach
//! the type. Splitting the gate here rather than widening that boundary keeps a
//! plain serde struct from dragging a runtime down the crate stack.
//!
//! Every public path is unchanged: a `native` build re-exports exactly what it
//! always did, from exactly where it always did.

mod config;
pub use config::ExtractionCacheConfig;

#[cfg(feature = "native")]
mod backend;
#[cfg(feature = "native")]
mod cache;
#[cfg(feature = "native")]
mod disk;
#[cfg(feature = "native")]
mod error;

#[cfg(feature = "native")]
pub use backend::CacheBackend;
#[cfg(feature = "native")]
pub use cache::{AppCacheEntry, ExtractionCache, ExtractionCacheStats, ExtractionGuard};
#[cfg(feature = "native")]
pub use disk::DiskBackend;
#[cfg(feature = "native")]
pub use error::CacheError;
