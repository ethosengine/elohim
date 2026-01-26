//! Cache store implementation
//!
//! In-memory LRU cache with TTL support, ETag generation, and pattern-based invalidation.
//! Uses holochain-cache-core for O(log n) eviction operations.
//!
//! ## Streaming Support
//!
//! Provides async blob streaming to avoid blocking conductor threads:
//! - `stream_blob()` - Stream entire blob as async chunks
//! - `get_range()` - Get byte range for HTTP 206 Partial Content
//! - `blob_size()` - Get blob size without loading data

use super::CacheConfig;
use bytes::Bytes;
use dashmap::DashMap;
use futures::stream::{self, Stream};
use holochain_cache_core::BlobCache;
use sha2::{Digest, Sha256};
use std::ops::Range;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// A cached entry with metadata
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// The cached data (typically JSON or MessagePack bytes)
    pub data: Vec<u8>,
    /// ETag for HTTP caching (SHA256 of data)
    pub etag: String,
    /// When this entry was created
    pub created_at: Instant,
    /// When this entry expires
    pub expires_at: Instant,
    /// Content-Type header value
    pub content_type: String,
    /// Reach level of the cached content (private, local, municipal, commons, etc.)
    pub reach: Option<String>,
    /// Cache priority (0-100, higher = serve sooner)
    pub cache_priority: u32,
    /// Bandwidth classification (low, medium, high, ultra)
    pub bandwidth_class: Option<String>,
    /// Geographic affinity hint for source prioritization
    pub geographic_affinity: Option<String>,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(data: Vec<u8>, ttl: Duration, content_type: &str) -> Self {
        let etag = Self::compute_etag(&data);
        let now = Instant::now();
        Self {
            data,
            etag,
            created_at: now,
            expires_at: now + ttl,
            content_type: content_type.to_string(),
            reach: None,
            cache_priority: 50, // Default priority
            bandwidth_class: None,
            geographic_affinity: None,
        }
    }

    /// Create a new cache entry with reach and performance hints
    pub fn with_reach(
        data: Vec<u8>,
        ttl: Duration,
        content_type: &str,
        reach: &str,
        cache_priority: u32,
        bandwidth_class: Option<&str>,
        geographic_affinity: Option<&str>,
    ) -> Self {
        let etag = Self::compute_etag(&data);
        let now = Instant::now();
        Self {
            data,
            etag,
            created_at: now,
            expires_at: now + ttl,
            content_type: content_type.to_string(),
            reach: Some(reach.to_string()),
            cache_priority: cache_priority.clamp(0, 100),
            bandwidth_class: bandwidth_class.map(|s| s.to_string()),
            geographic_affinity: geographic_affinity.map(|s| s.to_string()),
        }
    }

    /// Compute ETag from data using SHA256
    fn compute_etag(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        format!("\"{}\"", hex::encode(&hash[..16])) // Use first 16 bytes for shorter ETag
    }

    /// Check if this entry has expired
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    /// Get remaining TTL in seconds
    pub fn remaining_ttl_secs(&self) -> u64 {
        self.expires_at
            .saturating_duration_since(Instant::now())
            .as_secs()
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

impl CacheStats {
    /// Calculate hit rate as percentage
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }
}

/// In-memory content cache with O(log n) LRU eviction.
///
/// Uses holochain-cache-core's BlobCache for efficient eviction decisions
/// while maintaining DashMap for concurrent access to actual content.
pub struct ContentCache {
    /// The cache storage: storage_key -> entry
    entries: DashMap<String, CacheEntry>,
    /// LRU index for O(log n) eviction (tracks keys and sizes only)
    lru_index: RwLock<BlobCache>,
    /// Configuration
    config: CacheConfig,
    /// Hit counter
    hits: AtomicU64,
    /// Miss counter
    misses: AtomicU64,
    /// Eviction counter
    evictions: AtomicU64,
}

impl ContentCache {
    /// Create a new content cache with configuration.
    /// Initializes holochain-cache-core's BlobCache for O(log n) eviction.
    pub fn new(config: CacheConfig) -> Self {
        // Calculate max size: assume average entry is ~10KB
        let estimated_max_bytes = (config.max_entries as u64) * 10 * 1024;
        let lru_index = BlobCache::new(estimated_max_bytes);

        info!(
            max_entries = config.max_entries,
            "ContentCache initialized with holochain-cache-core O(log n) eviction"
        );

        Self {
            entries: DashMap::new(),
            lru_index: RwLock::new(lru_index),
            config,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(CacheConfig::default())
    }

    /// Get an entry from the cache by storage key
    pub fn get(&self, storage_key: &str) -> Option<CacheEntry> {
        if let Some(entry) = self.entries.get(storage_key) {
            if !entry.is_expired() {
                self.hits.fetch_add(1, Ordering::Relaxed);
                debug!(key = storage_key, "Cache hit");
                return Some(entry.clone());
            }
            // Entry expired, remove it
            drop(entry); // Release the reference before removing
            self.entries.remove(storage_key);
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        debug!(key = storage_key, "Cache miss");
        None
    }

    /// Check if an ETag matches the cached entry
    pub fn check_etag(&self, storage_key: &str, etag: &str) -> Option<bool> {
        self.entries.get(storage_key).map(|entry| {
            if entry.is_expired() {
                false
            } else {
                entry.etag == etag
            }
        })
    }

    /// Store an entry in the cache with explicit TTL
    pub fn set(&self, storage_key: &str, data: Vec<u8>, content_type: &str, ttl: Duration) {
        let entry = CacheEntry::new(data, ttl, content_type);
        debug!(key = storage_key, ttl_secs = ttl.as_secs(), "Cache set");
        self.entries.insert(storage_key.to_string(), entry);

        // Check if we need to evict entries
        self.maybe_evict();
    }

    /// Remove an entry from the cache
    pub fn remove(&self, storage_key: &str) -> Option<CacheEntry> {
        self.entries.remove(storage_key).map(|(_, entry)| entry)
    }

    /// Invalidate entries matching a pattern (prefix match)
    pub fn invalidate_pattern(&self, pattern: &str) -> usize {
        let keys_to_remove: Vec<String> = self
            .entries
            .iter()
            .filter(|entry| entry.key().starts_with(pattern))
            .map(|entry| entry.key().clone())
            .collect();

        let count = keys_to_remove.len();
        for key in keys_to_remove {
            self.entries.remove(&key);
        }

        if count > 0 {
            debug!(pattern = pattern, count = count, "Invalidated cache entries");
        }
        count
    }

    /// Invalidate all entries for a specific DNA
    pub fn invalidate_dna(&self, dna_hash: &str) -> usize {
        self.invalidate_pattern(&format!("{}:", dna_hash))
    }

    /// Invalidate all entries for a specific zome function
    pub fn invalidate_function(&self, dna_hash: &str, zome: &str, fn_name: &str) -> usize {
        self.invalidate_pattern(&format!("{}:{}:{}:", dna_hash, zome, fn_name))
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.entries.clear();
        info!("Cache cleared");
    }

    /// Remove expired entries
    pub fn cleanup(&self) -> usize {
        let expired: Vec<String> = self
            .entries
            .iter()
            .filter(|entry| entry.is_expired())
            .map(|entry| entry.key().clone())
            .collect();

        let count = expired.len();
        for key in expired {
            self.entries.remove(&key);
        }

        if count > 0 {
            debug!(count = count, "Cleaned up expired cache entries");
        }
        count
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.entries.len(),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
        }
    }

    /// Get configuration
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Get default TTL from config
    pub fn default_ttl(&self) -> Duration {
        self.config.content_ttl
    }

    /// Evict entries if over capacity (oldest first)
    fn maybe_evict(&self) {
        if self.entries.len() <= self.config.max_entries {
            return;
        }

        // Find oldest entries to evict
        let to_evict = self.entries.len() - self.config.max_entries + 100; // Evict 100 extra to avoid thrashing

        let mut entries: Vec<(String, Instant)> = self
            .entries
            .iter()
            .map(|entry| (entry.key().clone(), entry.created_at))
            .collect();

        entries.sort_by_key(|(_, created)| *created);

        for (key, _) in entries.into_iter().take(to_evict) {
            self.entries.remove(&key);
            self.evictions.fetch_add(1, Ordering::Relaxed);
        }

        debug!(evicted = to_evict, "Evicted cache entries");
    }

    // =========================================================================
    // Blob Streaming Operations (NEW - for HTTP 206 Range requests)
    // =========================================================================

    /// Get the size of a cached blob without loading the data.
    /// Returns None if the entry doesn't exist or is expired.
    pub fn blob_size(&self, storage_key: &str) -> Option<usize> {
        self.entries.get(storage_key).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(entry.data.len())
            }
        })
    }

    /// Get a byte range from a cached blob.
    /// Used for HTTP 206 Partial Content responses.
    ///
    /// # Arguments
    /// * `storage_key` - The cache key
    /// * `range` - Byte range (start..end, exclusive end)
    ///
    /// # Returns
    /// * `Some((data, total_size, etag))` if found and valid
    /// * `None` if not found, expired, or range invalid
    pub fn get_range(
        &self,
        storage_key: &str,
        range: Range<usize>,
    ) -> Option<(Bytes, usize, String)> {
        let entry = self.entries.get(storage_key)?;

        if entry.is_expired() {
            drop(entry);
            self.entries.remove(storage_key);
            self.misses.fetch_add(1, Ordering::Relaxed);
            return None;
        }

        let total_size = entry.data.len();

        // Validate range
        if range.start >= total_size || range.end > total_size || range.start >= range.end {
            warn!(
                key = storage_key,
                range_start = range.start,
                range_end = range.end,
                total_size = total_size,
                "Invalid byte range requested"
            );
            return None;
        }

        self.hits.fetch_add(1, Ordering::Relaxed);
        debug!(
            key = storage_key,
            range = format!("{}-{}", range.start, range.end - 1),
            "Cache range hit"
        );

        let data = Bytes::copy_from_slice(&entry.data[range]);
        let etag = entry.etag.clone();

        Some((data, total_size, etag))
    }

    /// Stream a blob in chunks without blocking.
    /// Returns an async stream of Bytes chunks.
    ///
    /// # Arguments
    /// * `storage_key` - The cache key
    /// * `chunk_size` - Size of each chunk (default 64KB if 0)
    ///
    /// # Returns
    /// * `Some(stream)` if blob exists and is valid
    /// * `None` if not found or expired
    pub fn stream_blob(
        &self,
        storage_key: &str,
        chunk_size: usize,
    ) -> Option<(
        Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>,
        usize,
        String,
        String,
    )> {
        let entry = self.entries.get(storage_key)?;

        if entry.is_expired() {
            drop(entry);
            self.entries.remove(storage_key);
            self.misses.fetch_add(1, Ordering::Relaxed);
            return None;
        }

        let data = entry.data.clone();
        let total_size = data.len();
        let etag = entry.etag.clone();
        let content_type = entry.content_type.clone();
        drop(entry); // Release lock before spawning

        self.hits.fetch_add(1, Ordering::Relaxed);
        debug!(
            key = storage_key,
            size = total_size,
            chunk_size = chunk_size,
            "Streaming blob"
        );

        // Use 64KB chunks if not specified
        let chunk_size = if chunk_size == 0 { 64 * 1024 } else { chunk_size };

        // Create chunked stream
        let stream = stream::iter((0..total_size).step_by(chunk_size).map(move |start| {
            let end = std::cmp::min(start + chunk_size, total_size);
            Ok(Bytes::copy_from_slice(&data[start..end]))
        }));

        Some((Box::pin(stream), total_size, etag, content_type))
    }

    /// Stream a byte range of a blob.
    /// Used for HTTP 206 with streaming response.
    ///
    /// # Arguments
    /// * `storage_key` - The cache key
    /// * `range` - Byte range to stream
    /// * `chunk_size` - Size of each chunk
    pub fn stream_range(
        &self,
        storage_key: &str,
        range: Range<usize>,
        chunk_size: usize,
    ) -> Option<(
        Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>,
        usize,
        String,
        String,
    )> {
        let entry = self.entries.get(storage_key)?;

        if entry.is_expired() {
            drop(entry);
            self.entries.remove(storage_key);
            self.misses.fetch_add(1, Ordering::Relaxed);
            return None;
        }

        let total_size = entry.data.len();

        // Validate range
        if range.start >= total_size || range.end > total_size || range.start >= range.end {
            return None;
        }

        // Extract the range data
        let range_data = entry.data[range.clone()].to_vec();
        let range_size = range_data.len();
        let etag = entry.etag.clone();
        let content_type = entry.content_type.clone();
        drop(entry);

        self.hits.fetch_add(1, Ordering::Relaxed);
        debug!(
            key = storage_key,
            range = format!("{}-{}", range.start, range.end - 1),
            "Streaming range"
        );

        let chunk_size = if chunk_size == 0 { 64 * 1024 } else { chunk_size };

        let stream = stream::iter((0..range_size).step_by(chunk_size).map(move |start| {
            let end = std::cmp::min(start + chunk_size, range_size);
            Ok(Bytes::copy_from_slice(&range_data[start..end]))
        }));

        Some((Box::pin(stream), total_size, etag, content_type))
    }

    /// Store a blob with explicit size tracking for large content.
    /// This method is optimized for media files.
    pub fn set_blob(
        &self,
        storage_key: &str,
        data: Vec<u8>,
        content_type: &str,
        ttl: Duration,
        reach: Option<&str>,
        priority: Option<u32>,
    ) {
        let size = data.len();
        let entry = if let Some(r) = reach {
            CacheEntry::with_reach(
                data,
                ttl,
                content_type,
                r,
                priority.unwrap_or(50),
                None,
                None,
            )
        } else {
            CacheEntry::new(data, ttl, content_type)
        };

        debug!(
            key = storage_key,
            size = size,
            content_type = content_type,
            ttl_secs = ttl.as_secs(),
            "Blob cached"
        );

        self.entries.insert(storage_key.to_string(), entry);

        // Update LRU index with size
        if let Ok(mut lru) = self.lru_index.write() {
            lru.put(
                storage_key.to_string(),
                size as u64,
                7, // Default to commons reach
                "blob".to_string(),
                "media".to_string(),
                priority.unwrap_or(50) as i32,
            );
        }

        self.maybe_evict();
    }
}

impl Default for ContentCache {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Spawn a background task to periodically cleanup expired entries
pub fn spawn_cleanup_task(cache: Arc<ContentCache>) {
    let interval = cache.config.cleanup_interval;

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            let removed = cache.cleanup();
            let stats = cache.stats();
            debug!(
                removed = removed,
                entries = stats.entries,
                hit_rate = format!("{:.1}%", stats.hit_rate()),
                "Cache cleanup completed"
            );
        }
    });

    info!("Cache cleanup task started");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry_etag() {
        let data = b"test data".to_vec();
        let entry = CacheEntry::new(data.clone(), Duration::from_secs(60), "application/json");

        // ETag should be consistent
        let entry2 = CacheEntry::new(data, Duration::from_secs(60), "application/json");
        assert_eq!(entry.etag, entry2.etag);

        // Different data should have different ETag
        let entry3 =
            CacheEntry::new(b"different".to_vec(), Duration::from_secs(60), "application/json");
        assert_ne!(entry.etag, entry3.etag);
    }

    #[test]
    fn test_cache_get_set() {
        let cache = ContentCache::with_defaults();
        let key = "dna123:zome:get_thing:abc";

        // Miss initially
        assert!(cache.get(key).is_none());

        // Set and get
        cache.set(key, b"test data".to_vec(), "application/json", Duration::from_secs(300));
        let entry = cache.get(key).expect("Should have entry");
        assert_eq!(entry.data, b"test data");

        // Stats
        let stats = cache.stats();
        assert_eq!(stats.entries, 1);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_cache_expiry() {
        let config = CacheConfig::default();
        let cache = ContentCache::new(config);
        let key = "dna:zome:fn:args";

        cache.set(key, b"will expire".to_vec(), "application/json", Duration::from_millis(10));

        // Should exist immediately
        assert!(cache.get(key).is_some());

        // Wait for expiry
        std::thread::sleep(Duration::from_millis(20));

        // Should be gone
        assert!(cache.get(key).is_none());
    }

    #[test]
    fn test_invalidate_pattern() {
        let cache = ContentCache::with_defaults();
        let ttl = Duration::from_secs(300);

        cache.set("dna1:zome:get_a:x", b"a".to_vec(), "application/json", ttl);
        cache.set("dna1:zome:get_b:y", b"b".to_vec(), "application/json", ttl);
        cache.set("dna2:zome:get_c:z", b"c".to_vec(), "application/json", ttl);

        assert_eq!(cache.stats().entries, 3);

        // Invalidate all dna1 entries
        let removed = cache.invalidate_pattern("dna1:");
        assert_eq!(removed, 2);
        assert_eq!(cache.stats().entries, 1);
    }

    #[test]
    fn test_invalidate_function() {
        let cache = ContentCache::with_defaults();
        let ttl = Duration::from_secs(300);

        cache.set("dna:zome:get_content:a", b"1".to_vec(), "application/json", ttl);
        cache.set("dna:zome:get_content:b", b"2".to_vec(), "application/json", ttl);
        cache.set("dna:zome:get_path:c", b"3".to_vec(), "application/json", ttl);

        assert_eq!(cache.stats().entries, 3);

        // Invalidate get_content function
        let removed = cache.invalidate_function("dna", "zome", "get_content");
        assert_eq!(removed, 2);
        assert_eq!(cache.stats().entries, 1);
    }

    #[test]
    fn test_invalidate_dna() {
        let cache = ContentCache::with_defaults();
        let ttl = Duration::from_secs(300);

        cache.set("dna_abc:z1:fn:x", b"1".to_vec(), "application/json", ttl);
        cache.set("dna_abc:z2:fn:y", b"2".to_vec(), "application/json", ttl);
        cache.set("dna_xyz:z1:fn:z", b"3".to_vec(), "application/json", ttl);

        let removed = cache.invalidate_dna("dna_abc");
        assert_eq!(removed, 2);
        assert_eq!(cache.stats().entries, 1);
    }
}
