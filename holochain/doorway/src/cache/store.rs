//! Cache store implementation
//!
//! In-memory LRU cache with TTL support, ETag generation, and pattern-based invalidation.

use super::CacheConfig;
use dashmap::DashMap;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info};

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

/// In-memory content cache
pub struct ContentCache {
    /// The cache storage: storage_key -> entry
    entries: DashMap<String, CacheEntry>,
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
    /// Create a new content cache with configuration
    pub fn new(config: CacheConfig) -> Self {
        Self {
            entries: DashMap::new(),
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
