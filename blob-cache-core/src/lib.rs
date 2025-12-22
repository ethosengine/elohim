/// Blob Cache Core - High-Performance LRU Cache in Rust
///
/// Provides O(log n) eviction and cleanup operations instead of O(n).
/// Compiled to WebAssembly for use in Angular applications.

use std::collections::{BTreeMap, HashMap};
use wasm_bindgen::prelude::*;

// Log macro for debugging (uses console.log in WASM)
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Cache entry metadata
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct CacheEntry {
    pub hash: String,
    pub size_bytes: u64,
    pub created_at: u64,
    pub last_accessed_at: u64,
    pub access_count: u32,
}

/// Statistics for cache tier
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct CacheStats {
    pub item_count: u32,
    pub total_size_bytes: u64,
    pub eviction_count: u64,
    pub hit_count: u64,
    pub miss_count: u64,
}

/// High-performance LRU cache with O(log n) operations
///
/// Uses a BTreeMap to maintain temporal ordering of access times,
/// allowing efficient LRU eviction without scanning entire cache.
#[wasm_bindgen]
pub struct BlobCache {
    // Primary storage: hash -> entry
    entries: HashMap<String, CacheEntry>,

    // Time index: last_accessed_at -> vec of hashes with that timestamp
    // Allows O(log n) finding of LRU item
    time_index: BTreeMap<u64, Vec<String>>,

    // Size tracking
    total_size: u64,
    max_size: u64,

    // Statistics
    hit_count: u64,
    miss_count: u64,
    eviction_count: u64,
}

#[wasm_bindgen]
impl BlobCache {
    /// Create new blob cache with size limit
    #[wasm_bindgen(constructor)]
    pub fn new(max_size_bytes: u64) -> BlobCache {
        BlobCache {
            entries: HashMap::new(),
            time_index: BTreeMap::new(),
            total_size: 0,
            max_size: max_size_bytes,
            hit_count: 0,
            miss_count: 0,
            eviction_count: 0,
        }
    }

    /// Put entry in cache, evicting LRU items if necessary
    /// O(log n) amortized time
    #[wasm_bindgen]
    pub fn put(&mut self, entry: CacheEntry) -> u32 {
        // Ensure space
        let evicted = self.evict_lru(entry.size_bytes);

        // Insert into primary storage
        self.entries.insert(entry.hash.clone(), entry.clone());
        self.total_size += entry.size_bytes;

        // Update time index
        self.time_index
            .entry(entry.last_accessed_at)
            .or_insert_with(Vec::new)
            .push(entry.hash);

        evicted
    }

    /// Get entry from cache, updating access time
    /// O(1) lookup
    #[wasm_bindgen]
    pub fn get(&mut self, hash: &str) -> Option<CacheEntry> {
        if let Some(entry) = self.entries.get_mut(hash) {
            // Update stats
            self.hit_count += 1;

            // Update access time (lazy - don't update index immediately)
            // Index will be corrected during eviction
            let old_time = entry.last_accessed_at;
            entry.last_accessed_at = current_time_ms();
            entry.access_count += 1;

            // Clone for return
            let updated = entry.clone();

            // Clean up old time index entry (remove hash from old time bucket)
            if let Some(hashes) = self.time_index.get_mut(&old_time) {
                hashes.retain(|h| h != hash);
                if hashes.is_empty() {
                    self.time_index.remove(&old_time);
                }
            }

            // Add to new time index
            self.time_index
                .entry(updated.last_accessed_at)
                .or_insert_with(Vec::new)
                .push(hash.to_string());

            Some(updated)
        } else {
            self.miss_count += 1;
            None
        }
    }

    /// Check if entry exists
    /// O(1)
    #[wasm_bindgen]
    pub fn contains(&self, hash: &str) -> bool {
        self.entries.contains_key(hash)
    }

    /// Delete entry from cache
    /// O(log n)
    #[wasm_bindgen]
    pub fn delete(&mut self, hash: &str) -> bool {
        if let Some(entry) = self.entries.remove(hash) {
            self.total_size -= entry.size_bytes;
            self.eviction_count += 1;

            // Remove from time index
            if let Some(hashes) = self.time_index.get_mut(&entry.last_accessed_at) {
                hashes.retain(|h| h != hash);
                if hashes.is_empty() {
                    self.time_index.remove(&entry.last_accessed_at);
                }
            }

            true
        } else {
            false
        }
    }

    /// Evict LRU items until space available
    /// O(k) where k = number of items evicted, O(log n) per item
    fn evict_lru(&mut self, required_bytes: u64) -> u32 {
        let mut evicted = 0;

        // Keep evicting until we have space
        while self.total_size + required_bytes > self.max_size && !self.entries.is_empty() {
            // Get the oldest access time
            if let Some((&oldest_time, _)) = self.time_index.iter().next() {
                // Get hashes with this time
                if let Some(hashes) = self.time_index.remove(&oldest_time) {
                    for hash in hashes {
                        if let Some(entry) = self.entries.remove(&hash) {
                            self.total_size -= entry.size_bytes;
                            self.eviction_count += 1;
                            evicted += 1;

                            // Check if we have enough space now
                            if self.total_size + required_bytes <= self.max_size {
                                return evicted;
                            }
                        }
                    }
                }
            } else {
                break;
            }
        }

        evicted
    }

    /// Get current cache size
    #[wasm_bindgen]
    pub fn get_total_size(&self) -> u64 {
        self.total_size
    }

    /// Get number of items
    #[wasm_bindgen]
    pub fn get_item_count(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Get cache statistics
    #[wasm_bindgen]
    pub fn get_stats(&self) -> CacheStats {
        CacheStats {
            item_count: self.entries.len() as u32,
            total_size_bytes: self.total_size,
            eviction_count: self.eviction_count,
            hit_count: self.hit_count,
            miss_count: self.miss_count,
        }
    }

    /// Clear entire cache
    #[wasm_bindgen]
    pub fn clear(&mut self) {
        self.entries.clear();
        self.time_index.clear();
        self.total_size = 0;
    }
}

/// High-performance time-based cache with O(k) cleanup
///
/// Uses BTreeMap keyed by creation time, allowing efficient
/// batch cleanup of expired items.
#[wasm_bindgen]
pub struct ChunkCache {
    // Primary storage: hash -> entry
    entries: HashMap<String, CacheEntry>,

    // Expiry index: created_at -> vec of hashes
    // Allows O(k) cleanup of expired items instead of O(n)
    expiry_index: BTreeMap<u64, Vec<String>>,

    // Size and TTL
    total_size: u64,
    max_size: u64,
    ttl_millis: u64,

    // Statistics
    hit_count: u64,
    miss_count: u64,
    eviction_count: u64,
    cleanup_count: u64,
}

#[wasm_bindgen]
impl ChunkCache {
    /// Create new chunk cache with size limit and TTL
    #[wasm_bindgen(constructor)]
    pub fn new(max_size_bytes: u64, ttl_millis: u64) -> ChunkCache {
        ChunkCache {
            entries: HashMap::new(),
            expiry_index: BTreeMap::new(),
            total_size: 0,
            max_size: max_size_bytes,
            ttl_millis,
            hit_count: 0,
            miss_count: 0,
            eviction_count: 0,
            cleanup_count: 0,
        }
    }

    /// Put entry in cache
    /// O(log n)
    #[wasm_bindgen]
    pub fn put(&mut self, entry: CacheEntry) -> u32 {
        // First clean up expired items
        let _ = self.cleanup_expired(current_time_ms());

        // Evict if necessary
        let evicted = self.evict_lru(entry.size_bytes);

        // Insert
        self.entries.insert(entry.hash.clone(), entry.clone());
        self.total_size += entry.size_bytes;

        // Index by creation time for fast cleanup
        self.expiry_index
            .entry(entry.created_at)
            .or_insert_with(Vec::new)
            .push(entry.hash);

        evicted
    }

    /// Get entry from cache
    /// O(1) lookup
    #[wasm_bindgen]
    pub fn get(&mut self, hash: &str) -> Option<CacheEntry> {
        if let Some(entry) = self.entries.get(hash) {
            // Check TTL
            let age_millis = current_time_ms() - entry.created_at;
            if age_millis > self.ttl_millis {
                // Expired - remove
                self.delete(hash);
                self.miss_count += 1;
                return None;
            }

            self.hit_count += 1;
            Some(entry.clone())
        } else {
            self.miss_count += 1;
            None
        }
    }

    /// Delete entry from cache
    /// O(log n)
    #[wasm_bindgen]
    pub fn delete(&mut self, hash: &str) -> bool {
        if let Some(entry) = self.entries.remove(hash) {
            self.total_size -= entry.size_bytes;
            self.eviction_count += 1;

            // Remove from expiry index
            if let Some(hashes) = self.expiry_index.get_mut(&entry.created_at) {
                hashes.retain(|h| h != hash);
                if hashes.is_empty() {
                    self.expiry_index.remove(&entry.created_at);
                }
            }

            true
        } else {
            false
        }
    }

    /// Clean up expired items
    /// O(k) where k = number of expired items (typically much smaller than n)
    #[wasm_bindgen]
    pub fn cleanup_expired(&mut self, now_millis: u64) -> u32 {
        let mut cleaned = 0;
        let cutoff = now_millis.saturating_sub(self.ttl_millis);

        // Use range() to efficiently get only expired buckets
        let expired_times: Vec<u64> = self.expiry_index
            .range(0..=cutoff)
            .map(|(&t, _)| t)
            .collect();

        for time in expired_times {
            if let Some(hashes) = self.expiry_index.remove(&time) {
                for hash in hashes {
                    if let Some(entry) = self.entries.remove(&hash) {
                        self.total_size -= entry.size_bytes;
                        self.eviction_count += 1;
                        cleaned += 1;
                    }
                }
            }
        }

        self.cleanup_count += cleaned as u64;
        cleaned
    }

    /// Evict LRU items if over capacity
    /// O(k log n)
    fn evict_lru(&mut self, required_bytes: u64) -> u32 {
        let mut evicted = 0;

        while self.total_size + required_bytes > self.max_size && !self.entries.is_empty() {
            // Evict oldest items
            if let Some((&oldest_time, _)) = self.expiry_index.iter().next() {
                if let Some(hashes) = self.expiry_index.remove(&oldest_time) {
                    for hash in hashes {
                        if let Some(entry) = self.entries.remove(&hash) {
                            self.total_size -= entry.size_bytes;
                            self.eviction_count += 1;
                            evicted += 1;

                            if self.total_size + required_bytes <= self.max_size {
                                return evicted;
                            }
                        }
                    }
                }
            } else {
                break;
            }
        }

        evicted
    }

    /// Get statistics
    #[wasm_bindgen]
    pub fn get_stats(&self) -> CacheStats {
        CacheStats {
            item_count: self.entries.len() as u32,
            total_size_bytes: self.total_size,
            eviction_count: self.eviction_count,
            hit_count: self.hit_count,
            miss_count: self.miss_count,
        }
    }

    /// Get current size
    #[wasm_bindgen]
    pub fn get_total_size(&self) -> u64 {
        self.total_size
    }

    /// Get item count
    #[wasm_bindgen]
    pub fn get_item_count(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Clear cache
    #[wasm_bindgen]
    pub fn clear(&mut self) {
        self.entries.clear();
        self.expiry_index.clear();
        self.total_size = 0;
    }
}

/// Get current time in milliseconds
fn current_time_ms() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::Date;
        Date::now() as u64
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_cache_eviction() {
        let mut cache = BlobCache::new(1000);

        // Add items
        for i in 0..10 {
            let entry = CacheEntry {
                hash: format!("hash-{}", i),
                size_bytes: 150,
                created_at: i as u64,
                last_accessed_at: i as u64,
                access_count: 0,
            };
            cache.put(entry);
        }

        // Cache should have evicted oldest items to stay under 1000 bytes
        assert!(cache.get_total_size() <= 1000);
        assert!(cache.get_item_count() < 10);
    }

    #[test]
    fn test_chunk_cache_cleanup() {
        let mut cache = ChunkCache::new(10000, 5000); // 5 second TTL

        // Add old item (1000 ms old)
        let old_entry = CacheEntry {
            hash: "old".to_string(),
            size_bytes: 100,
            created_at: 0,
            last_accessed_at: 0,
            access_count: 0,
        };
        cache.put(old_entry);

        // Clean up (6000 ms elapsed)
        let cleaned = cache.cleanup_expired(6000);

        // Old item should be cleaned
        assert_eq!(cleaned, 1);
        assert!(!cache.get("old").is_some());
    }
}
