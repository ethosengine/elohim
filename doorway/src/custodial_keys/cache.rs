//! In-memory cache for decrypted signing keys.
//!
//! Keys are cached during active sessions to avoid repeated decryption
//! on every zome call. The cache enforces TTL and zeroizes keys on eviction.
//!
//! # Security
//!
//! - Keys are zeroized when dropped (memory cleared)
//! - TTL enforcement prevents stale keys
//! - Max entries limit prevents memory exhaustion
//! - LRU eviction when at capacity

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use ed25519_dalek::SigningKey;
use zeroize::Zeroize;

// =============================================================================
// Configuration
// =============================================================================

/// Configuration for the signing key cache.
#[derive(Debug, Clone)]
pub struct SigningKeyCacheConfig {
    /// Default TTL for cached keys (how long before they expire)
    pub default_ttl: Duration,

    /// Maximum number of cached keys (prevents memory exhaustion)
    pub max_entries: usize,

    /// How often to run cleanup (remove expired entries)
    pub cleanup_interval: Duration,
}

impl Default for SigningKeyCacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: Duration::from_secs(3600),       // 1 hour
            max_entries: 10_000,                          // 10k concurrent sessions
            cleanup_interval: Duration::from_secs(60),   // Every minute
        }
    }
}

// =============================================================================
// Cached Signing Key
// =============================================================================

/// A cached signing key with expiration metadata.
///
/// The key is zeroized (memory cleared) when this struct is dropped.
pub struct CachedSigningKey {
    /// The decrypted Ed25519 signing key (sensitive!)
    key_bytes: [u8; 32],

    /// Human ID this key belongs to
    pub human_id: String,

    /// When this cache entry was created
    pub created_at: Instant,

    /// When this cache entry expires
    pub expires_at: Instant,

    /// Last time this key was used (for LRU eviction)
    last_used: Instant,
}

impl CachedSigningKey {
    /// Create a new cached signing key.
    pub fn new(key: SigningKey, human_id: String, ttl: Duration) -> Self {
        let now = Instant::now();
        Self {
            key_bytes: key.to_bytes(),
            human_id,
            created_at: now,
            expires_at: now + ttl,
            last_used: now,
        }
    }

    /// Check if this cache entry has expired.
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    /// Update the last used timestamp (for LRU tracking).
    pub fn touch(&mut self) {
        self.last_used = Instant::now();
    }

    /// Get the signing key (reconstructed from bytes).
    pub fn signing_key(&self) -> SigningKey {
        SigningKey::from_bytes(&self.key_bytes)
    }

    /// Get the last used instant (for LRU comparison).
    pub fn last_used(&self) -> Instant {
        self.last_used
    }
}

impl Drop for CachedSigningKey {
    fn drop(&mut self) {
        // Zeroize key material when dropped
        self.key_bytes.zeroize();
    }
}

// =============================================================================
// Cache Statistics
// =============================================================================

/// Statistics for the signing key cache.
#[derive(Debug, Default)]
pub struct CacheStats {
    /// Total number of cache hits
    pub hits: AtomicU64,

    /// Total number of cache misses
    pub misses: AtomicU64,

    /// Total number of keys inserted
    pub inserts: AtomicU64,

    /// Total number of keys evicted (TTL or LRU)
    pub evictions: AtomicU64,
}

impl CacheStats {
    /// Record a cache hit.
    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache miss.
    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an insertion.
    pub fn record_insert(&self) {
        self.inserts.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an eviction.
    pub fn record_eviction(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    /// Get snapshot of current stats.
    pub fn snapshot(&self) -> CacheStatsSnapshot {
        CacheStatsSnapshot {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            inserts: self.inserts.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStatsSnapshot {
    pub hits: u64,
    pub misses: u64,
    pub inserts: u64,
    pub evictions: u64,
}

// =============================================================================
// Signing Key Cache
// =============================================================================

/// In-memory cache for decrypted signing keys.
///
/// Keys are stored by session_id and automatically expire after TTL.
pub struct SigningKeyCache {
    /// Cache entries indexed by session_id
    cache: DashMap<String, CachedSigningKey>,

    /// Cache configuration
    config: SigningKeyCacheConfig,

    /// Cache statistics
    stats: CacheStats,
}

impl SigningKeyCache {
    /// Create a new signing key cache with the given configuration.
    pub fn new(config: SigningKeyCacheConfig) -> Self {
        Self {
            cache: DashMap::new(),
            config,
            stats: CacheStats::default(),
        }
    }

    /// Create a new cache with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(SigningKeyCacheConfig::default())
    }

    /// Store a signing key in the cache.
    ///
    /// If the cache is at capacity, the least recently used entry is evicted.
    pub fn insert(&self, session_id: String, key: SigningKey, human_id: String) {
        // Evict if at capacity
        if self.cache.len() >= self.config.max_entries {
            self.evict_lru();
        }

        let entry = CachedSigningKey::new(key, human_id, self.config.default_ttl);
        self.cache.insert(session_id, entry);
        self.stats.record_insert();
    }

    /// Get a signing key from the cache.
    ///
    /// Returns None if the key doesn't exist or has expired.
    /// Updates the last_used timestamp on hit.
    pub fn get(&self, session_id: &str) -> Option<SigningKey> {
        if let Some(mut entry) = self.cache.get_mut(session_id) {
            if entry.is_expired() {
                // Entry expired, remove it
                drop(entry);
                self.cache.remove(session_id);
                self.stats.record_miss();
                self.stats.record_eviction();
                return None;
            }

            entry.touch();
            self.stats.record_hit();
            return Some(entry.signing_key());
        }

        self.stats.record_miss();
        None
    }

    /// Check if a session has a cached key (without updating last_used).
    pub fn contains(&self, session_id: &str) -> bool {
        self.cache
            .get(session_id)
            .map(|e| !e.is_expired())
            .unwrap_or(false)
    }

    /// Remove a key from the cache (logout).
    ///
    /// Returns true if a key was removed.
    pub fn remove(&self, session_id: &str) -> bool {
        self.cache.remove(session_id).is_some()
    }

    /// Remove all keys for a specific human (logout all sessions).
    ///
    /// Returns the number of sessions removed.
    pub fn remove_human(&self, human_id: &str) -> usize {
        let mut removed = 0;
        self.cache.retain(|_, v| {
            if v.human_id == human_id {
                removed += 1;
                false
            } else {
                true
            }
        });
        removed
    }

    /// Remove all expired entries.
    ///
    /// Returns the number of entries removed.
    pub fn cleanup(&self) -> usize {
        let mut removed = 0;
        self.cache.retain(|_, v| {
            if v.is_expired() {
                removed += 1;
                self.stats.record_eviction();
                false
            } else {
                true
            }
        });
        removed
    }

    /// Get the current number of cached keys.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStatsSnapshot {
        self.stats.snapshot()
    }

    /// Evict the least recently used entry.
    fn evict_lru(&self) {
        // Find the oldest entry by last_used
        let oldest_key = self
            .cache
            .iter()
            .min_by_key(|e| e.last_used())
            .map(|e| e.key().clone());

        if let Some(key) = oldest_key {
            self.cache.remove(&key);
            self.stats.record_eviction();
        }
    }
}

impl Default for SigningKeyCache {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::custodial_keys::crypto::generate_keypair;

    #[test]
    fn test_cache_insert_and_get() {
        let cache = SigningKeyCache::with_defaults();
        let (signing_key, _) = generate_keypair();
        let original_bytes = signing_key.to_bytes();

        cache.insert("session-1".to_string(), signing_key, "human-1".to_string());

        let retrieved = cache.get("session-1").unwrap();
        assert_eq!(retrieved.to_bytes(), original_bytes);
    }

    #[test]
    fn test_cache_miss() {
        let cache = SigningKeyCache::with_defaults();
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_cache_remove() {
        let cache = SigningKeyCache::with_defaults();
        let (signing_key, _) = generate_keypair();

        cache.insert("session-1".to_string(), signing_key, "human-1".to_string());
        assert!(cache.contains("session-1"));

        cache.remove("session-1");
        assert!(!cache.contains("session-1"));
    }

    #[test]
    fn test_cache_remove_human() {
        let cache = SigningKeyCache::with_defaults();

        // Add multiple sessions for same human
        for i in 0..3 {
            let (key, _) = generate_keypair();
            cache.insert(format!("session-{}", i), key, "human-1".to_string());
        }

        // Add session for different human
        let (key, _) = generate_keypair();
        cache.insert("other-session".to_string(), key, "human-2".to_string());

        assert_eq!(cache.len(), 4);

        // Remove all sessions for human-1
        let removed = cache.remove_human("human-1");
        assert_eq!(removed, 3);
        assert_eq!(cache.len(), 1);
        assert!(cache.contains("other-session"));
    }

    #[test]
    fn test_cache_expiry() {
        let config = SigningKeyCacheConfig {
            default_ttl: Duration::from_millis(10), // Very short TTL
            ..Default::default()
        };
        let cache = SigningKeyCache::new(config);
        let (signing_key, _) = generate_keypair();

        cache.insert("session-1".to_string(), signing_key, "human-1".to_string());

        // Key should be available immediately
        assert!(cache.get("session-1").is_some());

        // Wait for expiry
        std::thread::sleep(Duration::from_millis(20));

        // Key should be expired and removed
        assert!(cache.get("session-1").is_none());
    }

    #[test]
    fn test_cache_lru_eviction() {
        let config = SigningKeyCacheConfig {
            max_entries: 3,
            ..Default::default()
        };
        let cache = SigningKeyCache::new(config);

        // Insert 3 keys
        for i in 0..3 {
            let (key, _) = generate_keypair();
            cache.insert(format!("session-{}", i), key, format!("human-{}", i));
            std::thread::sleep(Duration::from_millis(10)); // Ensure different timestamps
        }

        // Access session-0 to make it recently used
        cache.get("session-0");

        // Insert 4th key - should evict session-1 (LRU)
        let (key, _) = generate_keypair();
        cache.insert("session-3".to_string(), key, "human-3".to_string());

        assert_eq!(cache.len(), 3);
        assert!(cache.contains("session-0")); // Recently used
        assert!(!cache.contains("session-1")); // Should be evicted (LRU)
        assert!(cache.contains("session-2"));
        assert!(cache.contains("session-3"));
    }

    #[test]
    fn test_cache_stats() {
        let cache = SigningKeyCache::with_defaults();
        let (key, _) = generate_keypair();

        cache.insert("session-1".to_string(), key, "human-1".to_string());
        cache.get("session-1"); // Hit
        cache.get("session-1"); // Hit
        cache.get("nonexistent"); // Miss

        let stats = cache.stats();
        assert_eq!(stats.inserts, 1);
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
    }
}
