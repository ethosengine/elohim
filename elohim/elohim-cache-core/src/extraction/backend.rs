//! CacheBackend trait — pluggable storage for extraction cache

use super::CacheError;
use async_trait::async_trait;

/// Pluggable storage backend for cached content.
///
/// Keys are forward-slash-separated paths (e.g., `"app_id/js/main.js"`).
/// Implementations must be Send + Sync for use in async HTTP handlers.
#[async_trait]
pub trait CacheBackend: Send + Sync {
    /// Get cached bytes by key. Returns None on miss.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError>;

    /// Store bytes at key. Returns true if this was a new entry (not overwrite).
    async fn put(&self, key: &str, data: Vec<u8>) -> Result<bool, CacheError>;

    /// Delete a single entry. Returns true if it existed.
    async fn delete(&self, key: &str) -> Result<bool, CacheError>;

    /// Delete all entries whose key starts with prefix.
    async fn delete_prefix(&self, prefix: &str) -> Result<u64, CacheError>;

    /// Check if key exists without reading bytes.
    async fn exists(&self, key: &str) -> Result<bool, CacheError>;

    /// Total size of all cached data in bytes.
    async fn total_size(&self) -> Result<u64, CacheError>;
}
