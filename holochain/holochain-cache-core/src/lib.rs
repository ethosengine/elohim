//! Holochain Cache Core - High-Performance Content-Reach Aware Cache
//!
//! Provides O(log n) eviction and cleanup operations instead of O(n).
//! Compiled to WebAssembly for use in Holochain/Elohim client applications.
//!
//! # Key Features
//! - Reach-level isolation (private → commons): 8 independent LRU caches
//! - Domain/Epic content organization
//! - Custodian-aware distribution with geographic awareness
//! - Mastery-based TTL with freshness decay
//! - O(log n) operations via BTreeMap indices

use std::collections::{BTreeMap, HashMap};
use wasm_bindgen::prelude::*;

// ============================================================================
// Elohim Protocol Constants
// ============================================================================

/// Reach levels: 0=Private, 1=Invited, 2=Local, 3=Neighborhood,
/// 4=Municipal, 5=Bioregional, 6=Regional, 7=Commons
pub const REACH_PRIVATE: u8 = 0;
pub const REACH_INVITED: u8 = 1;
pub const REACH_LOCAL: u8 = 2;
pub const REACH_NEIGHBORHOOD: u8 = 3;
pub const REACH_MUNICIPAL: u8 = 4;
pub const REACH_BIOREGIONAL: u8 = 5;
pub const REACH_REGIONAL: u8 = 6;
pub const REACH_COMMONS: u8 = 7;

/// Mastery levels with decay rates (per day):
/// 0=NotStarted(0.0), 1=Seen(0.05), 2=Remember(0.03), 3=Understand(0.02),
/// 4=Apply(0.015), 5=Analyze(0.01), 6=Evaluate(0.008), 7=Create(0.005)
pub const MASTERY_NOT_STARTED: u8 = 0;
pub const MASTERY_SEEN: u8 = 1;
pub const MASTERY_REMEMBER: u8 = 2;
pub const MASTERY_UNDERSTAND: u8 = 3;
pub const MASTERY_APPLY: u8 = 4;
pub const MASTERY_ANALYZE: u8 = 5;
pub const MASTERY_EVALUATE: u8 = 6;
pub const MASTERY_CREATE: u8 = 7;

// ============================================================================
// Internal Data Structures (not exported to JS)
// ============================================================================

/// Internal cache entry - stores all metadata
#[derive(Clone, Debug)]
struct CacheEntryInternal {
    hash: String,
    size_bytes: u64,
    created_at: u64,
    last_accessed_at: u64,
    access_count: u32,
    reach_level: u8,
    domain: String,
    epic: String,
    priority: i32,
}

// ============================================================================
// Cache Statistics (exported to JS)
// ============================================================================

/// Cache statistics - immutable snapshot of cache state
#[wasm_bindgen]
pub struct CacheStats {
    item_count: u32,
    total_size_bytes: u64,
    eviction_count: u64,
    hit_count: u64,
    miss_count: u64,
}

#[wasm_bindgen]
impl CacheStats {
    /// Number of items in cache
    #[wasm_bindgen(getter)]
    pub fn item_count(&self) -> u32 {
        self.item_count
    }

    /// Total size of cached items in bytes
    #[wasm_bindgen(getter)]
    pub fn total_size_bytes(&self) -> u64 {
        self.total_size_bytes
    }

    /// Number of items evicted since creation
    #[wasm_bindgen(getter)]
    pub fn eviction_count(&self) -> u64 {
        self.eviction_count
    }

    /// Cache hits
    #[wasm_bindgen(getter)]
    pub fn hit_count(&self) -> u64 {
        self.hit_count
    }

    /// Cache misses
    #[wasm_bindgen(getter)]
    pub fn miss_count(&self) -> u64 {
        self.miss_count
    }

    /// Hit rate as percentage (0-100)
    #[wasm_bindgen]
    pub fn hit_rate(&self) -> f64 {
        let total = self.hit_count + self.miss_count;
        if total == 0 {
            0.0
        } else {
            (self.hit_count as f64 / total as f64) * 100.0
        }
    }
}

// ============================================================================
// LRU Blob Cache - O(log n) operations
// ============================================================================

/// High-performance LRU cache with O(log n) eviction.
///
/// Uses BTreeMap for temporal ordering, enabling efficient LRU eviction
/// without scanning the entire cache.
///
/// # Example
/// ```javascript
/// const cache = new BlobCache(1024 * 1024 * 1024); // 1GB
/// cache.put("hash123", 1024, 7, "elohim-protocol", "governance", 50);
/// const exists = cache.has("hash123");
/// cache.touch("hash123"); // Update access time
/// ```
#[wasm_bindgen]
pub struct BlobCache {
    entries: HashMap<String, CacheEntryInternal>,
    time_index: BTreeMap<u64, Vec<String>>,
    total_size: u64,
    max_size: u64,
    hit_count: u64,
    miss_count: u64,
    eviction_count: u64,
}

#[wasm_bindgen]
impl BlobCache {
    /// Create a new LRU cache with specified maximum size in bytes.
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

    /// Add or update an entry. Returns number of items evicted.
    ///
    /// # Arguments
    /// * `hash` - Content hash (unique identifier)
    /// * `size_bytes` - Size of the content
    /// * `reach_level` - 0-7 (private → commons)
    /// * `domain` - Content domain (e.g., "elohim-protocol")
    /// * `epic` - Content epic (e.g., "governance")
    /// * `priority` - Priority score (higher = less likely to evict)
    #[wasm_bindgen]
    pub fn put(
        &mut self,
        hash: String,
        size_bytes: u64,
        reach_level: u8,
        domain: String,
        epic: String,
        priority: i32,
    ) -> u32 {
        let now = current_time_ms();

        // Remove existing entry if present
        if self.entries.contains_key(&hash) {
            self.delete(&hash);
        }

        // Evict LRU items if necessary
        let evicted = self.evict_lru(size_bytes);

        // Create entry
        let entry = CacheEntryInternal {
            hash: hash.clone(),
            size_bytes,
            created_at: now,
            last_accessed_at: now,
            access_count: 0,
            reach_level: reach_level.min(7),
            domain,
            epic,
            priority,
        };

        // Insert into storage
        self.entries.insert(hash.clone(), entry);
        self.total_size += size_bytes;

        // Update time index
        self.time_index
            .entry(now)
            .or_insert_with(Vec::new)
            .push(hash);

        evicted
    }

    /// Check if an entry exists. O(1).
    #[wasm_bindgen]
    pub fn has(&self, hash: &str) -> bool {
        self.entries.contains_key(hash)
    }

    /// Update access time for an entry (touch). Returns true if found.
    #[wasm_bindgen]
    pub fn touch(&mut self, hash: &str) -> bool {
        if let Some(entry) = self.entries.get_mut(hash) {
            let old_time = entry.last_accessed_at;
            let new_time = current_time_ms();

            entry.last_accessed_at = new_time;
            entry.access_count += 1;
            self.hit_count += 1;

            // Update time index
            if let Some(hashes) = self.time_index.get_mut(&old_time) {
                hashes.retain(|h| h != hash);
                if hashes.is_empty() {
                    self.time_index.remove(&old_time);
                }
            }
            self.time_index
                .entry(new_time)
                .or_insert_with(Vec::new)
                .push(hash.to_string());

            true
        } else {
            self.miss_count += 1;
            false
        }
    }

    /// Get entry metadata as JSON string. Returns empty string if not found.
    #[wasm_bindgen]
    pub fn get_json(&mut self, hash: &str) -> String {
        if let Some(entry) = self.entries.get_mut(hash) {
            self.hit_count += 1;
            format!(
                r#"{{"hash":"{}","sizeBytes":{},"createdAt":{},"lastAccessedAt":{},"accessCount":{},"reachLevel":{},"domain":"{}","epic":"{}","priority":{}}}"#,
                entry.hash,
                entry.size_bytes,
                entry.created_at,
                entry.last_accessed_at,
                entry.access_count,
                entry.reach_level,
                entry.domain,
                entry.epic,
                entry.priority
            )
        } else {
            self.miss_count += 1;
            String::new()
        }
    }

    /// Delete an entry. Returns true if found and deleted.
    #[wasm_bindgen]
    pub fn delete(&mut self, hash: &str) -> bool {
        if let Some(entry) = self.entries.remove(hash) {
            self.total_size -= entry.size_bytes;

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

    /// Evict least recently used items until space is available.
    /// Returns number of items evicted. O(k log n) where k = evictions.
    fn evict_lru(&mut self, required_bytes: u64) -> u32 {
        let mut evicted = 0;

        while self.total_size + required_bytes > self.max_size && !self.entries.is_empty() {
            // Get oldest timestamp
            let oldest_time = match self.time_index.keys().next().copied() {
                Some(t) => t,
                None => break,
            };

            // Get hashes at that time
            if let Some(hashes) = self.time_index.remove(&oldest_time) {
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
        }

        evicted
    }

    /// Get current cache size in bytes.
    #[wasm_bindgen]
    pub fn size(&self) -> u64 {
        self.total_size
    }

    /// Get number of items in cache.
    #[wasm_bindgen]
    pub fn count(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Get maximum cache size in bytes.
    #[wasm_bindgen]
    pub fn max_size(&self) -> u64 {
        self.max_size
    }

    /// Get cache statistics snapshot.
    #[wasm_bindgen]
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            item_count: self.entries.len() as u32,
            total_size_bytes: self.total_size,
            eviction_count: self.eviction_count,
            hit_count: self.hit_count,
            miss_count: self.miss_count,
        }
    }

    /// Clear all entries.
    #[wasm_bindgen]
    pub fn clear(&mut self) {
        self.entries.clear();
        self.time_index.clear();
        self.total_size = 0;
    }
}

// ============================================================================
// TTL Chunk Cache - O(k) cleanup where k = expired items
// ============================================================================

/// Time-based cache with efficient O(k) cleanup of expired items.
///
/// Uses BTreeMap keyed by creation time for fast expiration.
///
/// # Example
/// ```javascript
/// const cache = new ChunkCache(10 * 1024 * 1024 * 1024, 7 * 24 * 60 * 60 * 1000); // 10GB, 7 days
/// cache.put("chunk-hash", 65536);
/// cache.cleanup(); // Remove expired items
/// ```
#[wasm_bindgen]
pub struct ChunkCache {
    entries: HashMap<String, CacheEntryInternal>,
    expiry_index: BTreeMap<u64, Vec<String>>,
    total_size: u64,
    max_size: u64,
    ttl_millis: u64,
    hit_count: u64,
    miss_count: u64,
    eviction_count: u64,
}

#[wasm_bindgen]
impl ChunkCache {
    /// Create a new TTL cache.
    ///
    /// # Arguments
    /// * `max_size_bytes` - Maximum cache size
    /// * `ttl_millis` - Time-to-live in milliseconds
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
        }
    }

    /// Add a chunk. Returns number of items evicted.
    #[wasm_bindgen]
    pub fn put(&mut self, hash: String, size_bytes: u64) -> u32 {
        let now = current_time_ms();

        // Remove existing
        if self.entries.contains_key(&hash) {
            self.delete(&hash);
        }

        // Cleanup expired first
        self.cleanup_internal(now);

        // Evict if over capacity
        let evicted = self.evict_oldest(size_bytes);

        // Create entry
        let entry = CacheEntryInternal {
            hash: hash.clone(),
            size_bytes,
            created_at: now,
            last_accessed_at: now,
            access_count: 0,
            reach_level: 7,
            domain: String::new(),
            epic: String::new(),
            priority: 0,
        };

        self.entries.insert(hash.clone(), entry);
        self.total_size += size_bytes;

        // Index by creation time for expiry
        self.expiry_index
            .entry(now)
            .or_insert_with(Vec::new)
            .push(hash);

        evicted
    }

    /// Check if chunk exists and is not expired.
    #[wasm_bindgen]
    pub fn has(&self, hash: &str) -> bool {
        if let Some(entry) = self.entries.get(hash) {
            let age = current_time_ms().saturating_sub(entry.created_at);
            age <= self.ttl_millis
        } else {
            false
        }
    }

    /// Touch a chunk (update access time). Returns true if found and valid.
    #[wasm_bindgen]
    pub fn touch(&mut self, hash: &str) -> bool {
        if let Some(entry) = self.entries.get_mut(hash) {
            let now = current_time_ms();
            let age = now.saturating_sub(entry.created_at);

            if age > self.ttl_millis {
                self.miss_count += 1;
                self.delete(hash);
                return false;
            }

            entry.last_accessed_at = now;
            entry.access_count += 1;
            self.hit_count += 1;
            true
        } else {
            self.miss_count += 1;
            false
        }
    }

    /// Delete a chunk.
    #[wasm_bindgen]
    pub fn delete(&mut self, hash: &str) -> bool {
        if let Some(entry) = self.entries.remove(hash) {
            self.total_size -= entry.size_bytes;

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

    /// Cleanup expired items. O(k) where k = expired items.
    /// Returns number of items removed.
    #[wasm_bindgen]
    pub fn cleanup(&mut self) -> u32 {
        self.cleanup_internal(current_time_ms())
    }

    fn cleanup_internal(&mut self, now: u64) -> u32 {
        let cutoff = now.saturating_sub(self.ttl_millis);
        let mut cleaned = 0;

        // Get all expired time buckets
        let expired_times: Vec<u64> = self
            .expiry_index
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

        cleaned
    }

    /// Evict oldest items if over capacity.
    fn evict_oldest(&mut self, required_bytes: u64) -> u32 {
        let mut evicted = 0;

        while self.total_size + required_bytes > self.max_size && !self.entries.is_empty() {
            let oldest_time = match self.expiry_index.keys().next().copied() {
                Some(t) => t,
                None => break,
            };

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
        }

        evicted
    }

    /// Get cache size in bytes.
    #[wasm_bindgen]
    pub fn size(&self) -> u64 {
        self.total_size
    }

    /// Get number of items.
    #[wasm_bindgen]
    pub fn count(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Get statistics.
    #[wasm_bindgen]
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            item_count: self.entries.len() as u32,
            total_size_bytes: self.total_size,
            eviction_count: self.eviction_count,
            hit_count: self.hit_count,
            miss_count: self.miss_count,
        }
    }

    /// Clear all entries.
    #[wasm_bindgen]
    pub fn clear(&mut self) {
        self.entries.clear();
        self.expiry_index.clear();
        self.total_size = 0;
    }
}

// ============================================================================
// Reach-Aware Cache - Isolated caches per reach level
// ============================================================================

/// Multi-reach cache maintaining separate LRU caches per reach level (0-7).
///
/// Ensures content at different reach levels never evict each other.
/// Private content (reach=0) is isolated from commons content (reach=7).
///
/// # Example
/// ```javascript
/// const cache = new ReachAwareCache(128 * 1024 * 1024); // 128MB per reach
/// cache.put("hash", 1024, 7, "elohim-protocol", "governance", 50);
/// cache.put("private", 512, 0, "personal", "notes", 10);
/// // Private content won't evict commons content
/// ```
#[wasm_bindgen]
pub struct ReachAwareCache {
    reach_caches: Vec<BlobCache>,
    domain_index: HashMap<String, Vec<(String, u8)>>, // domain -> [(hash, reach)]
}

#[wasm_bindgen]
impl ReachAwareCache {
    /// Create reach-aware cache with specified size per reach level.
    #[wasm_bindgen(constructor)]
    pub fn new(max_size_per_reach: u64) -> ReachAwareCache {
        let mut reach_caches = Vec::with_capacity(8);
        for _ in 0..8 {
            reach_caches.push(BlobCache::new(max_size_per_reach));
        }

        ReachAwareCache {
            reach_caches,
            domain_index: HashMap::new(),
        }
    }

    /// Add entry to appropriate reach cache.
    #[wasm_bindgen]
    pub fn put(
        &mut self,
        hash: String,
        size_bytes: u64,
        reach_level: u8,
        domain: String,
        epic: String,
        priority: i32,
    ) -> u32 {
        let reach = (reach_level as usize).min(7);

        // Update domain index
        let key = domain.clone();
        self.domain_index
            .entry(key)
            .or_insert_with(Vec::new)
            .push((hash.clone(), reach_level));

        self.reach_caches[reach].put(hash, size_bytes, reach_level, domain, epic, priority)
    }

    /// Check if entry exists at specified reach level.
    #[wasm_bindgen]
    pub fn has(&self, hash: &str, reach_level: u8) -> bool {
        let reach = (reach_level as usize).min(7);
        self.reach_caches[reach].has(hash)
    }

    /// Touch entry at specified reach level.
    #[wasm_bindgen]
    pub fn touch(&mut self, hash: &str, reach_level: u8) -> bool {
        let reach = (reach_level as usize).min(7);
        self.reach_caches[reach].touch(hash)
    }

    /// Delete entry from specified reach level.
    #[wasm_bindgen]
    pub fn delete(&mut self, hash: &str, reach_level: u8) -> bool {
        let reach = (reach_level as usize).min(7);
        let deleted = self.reach_caches[reach].delete(hash);

        if deleted {
            // Clean domain index
            for entries in self.domain_index.values_mut() {
                entries.retain(|(h, _)| h != hash);
            }
        }

        deleted
    }

    /// Get statistics for a specific reach level.
    #[wasm_bindgen]
    pub fn stats_for_reach(&self, reach_level: u8) -> CacheStats {
        let reach = (reach_level as usize).min(7);
        self.reach_caches[reach].stats()
    }

    /// Get total items across all reach levels.
    #[wasm_bindgen]
    pub fn total_count(&self) -> u32 {
        self.reach_caches.iter().map(|c| c.count()).sum()
    }

    /// Get total size across all reach levels.
    #[wasm_bindgen]
    pub fn total_size(&self) -> u64 {
        self.reach_caches.iter().map(|c| c.size()).sum()
    }

    /// Clear all caches.
    #[wasm_bindgen]
    pub fn clear(&mut self) {
        for cache in &mut self.reach_caches {
            cache.clear();
        }
        self.domain_index.clear();
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Calculate priority score for content.
///
/// Priority = reach_level × 12 + proximity + bandwidth_bonus + steward_bonus + affinity × 10 - age_penalty
///
/// # Arguments
/// * `reach_level` - 0-7 (private → commons)
/// * `proximity_score` - -100 to +100 (geographic proximity to custodian)
/// * `bandwidth_class` - 1-4 (low → ultra)
/// * `steward_tier` - 1-4 (caretaker → pioneer)
/// * `affinity_match` - 0.0-1.0 (content relevance to user)
/// * `age_penalty` - Penalty for aged content
#[wasm_bindgen]
pub fn calculate_priority(
    reach_level: u8,
    proximity_score: i32,
    bandwidth_class: u8,
    steward_tier: u8,
    affinity_match: f64,
    age_penalty: i32,
) -> i32 {
    let mut score: i32 = 0;

    // Base reach (0-84 points)
    score += (reach_level.min(7) as i32) * 12;

    // Proximity (-100 to +100)
    score += proximity_score.clamp(-100, 100);

    // Bandwidth bonus
    score += match bandwidth_class {
        4 => 20,  // Ultra
        3 => 10,  // High
        2 => 5,   // Medium
        1 => -5,  // Low
        _ => 0,
    };

    // Steward bonus
    score += match steward_tier {
        4 => 50, // Pioneer
        3 => 30, // Expert
        2 => 15, // Curator
        1 => 5,  // Caretaker
        _ => 0,
    };

    // Affinity (0-10 points)
    score += (affinity_match.clamp(0.0, 1.0) * 10.0) as i32;

    // Age penalty
    score -= age_penalty;

    score.clamp(0, 200)
}

/// Calculate mastery freshness (0.0-1.0) based on age and mastery level.
///
/// Higher mastery levels decay slower.
#[wasm_bindgen]
pub fn calculate_freshness(mastery_level: u8, age_seconds: f64) -> f64 {
    let decay_per_day = match mastery_level {
        0 => 0.0,   // NotStarted - no decay
        1 => 0.05,  // Seen
        2 => 0.03,  // Remember
        3 => 0.02,  // Understand
        4 => 0.015, // Apply
        5 => 0.01,  // Analyze
        6 => 0.008, // Evaluate
        7 => 0.005, // Create
        _ => 0.0,
    };

    let decay_per_second = decay_per_day / 86400.0;
    (1.0 - decay_per_second * age_seconds).max(0.0)
}

// ============================================================================
// Time Utility
// ============================================================================

fn current_time_ms() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as u64
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_cache_basic() {
        let mut cache = BlobCache::new(1000);

        // Add items
        cache.put("a".into(), 100, 7, "test".into(), "gov".into(), 50);
        cache.put("b".into(), 200, 7, "test".into(), "gov".into(), 50);

        assert!(cache.has("a"));
        assert!(cache.has("b"));
        assert_eq!(cache.size(), 300);
        assert_eq!(cache.count(), 2);
    }

    #[test]
    fn test_blob_cache_eviction() {
        let mut cache = BlobCache::new(500);

        // Fill cache
        for i in 0..10 {
            cache.put(format!("item-{}", i), 100, 7, "test".into(), "gov".into(), i);
        }

        // Should have evicted to stay under 500 bytes
        assert!(cache.size() <= 500);
        assert!(cache.count() < 10);
    }

    #[test]
    fn test_reach_aware_isolation() {
        let mut cache = ReachAwareCache::new(1000);

        // Add to different reaches
        cache.put("private".into(), 100, 0, "p".into(), "e".into(), 10);
        cache.put("commons".into(), 100, 7, "p".into(), "e".into(), 10);

        // Both should exist independently
        assert!(cache.has("private", 0));
        assert!(cache.has("commons", 7));

        // Cross-reach should not find
        assert!(!cache.has("private", 7));
        assert!(!cache.has("commons", 0));
    }

    #[test]
    fn test_priority_calculation() {
        // Commons + high proximity + ultra bandwidth + pioneer + high affinity
        let priority = calculate_priority(7, 80, 4, 4, 0.9, 0);
        assert!(priority > 150);

        // Private + low proximity + low bandwidth + caretaker + low affinity
        let low_priority = calculate_priority(0, -50, 1, 1, 0.1, 20);
        assert!(low_priority < 50);
    }

    #[test]
    fn test_freshness_decay() {
        // NotStarted should never decay
        assert_eq!(calculate_freshness(0, 86400.0 * 30.0), 1.0);

        // Seen should decay fast (0.05/day)
        let seen_30_days = calculate_freshness(1, 86400.0 * 30.0);
        assert!(seen_30_days < 0.5);

        // Create should decay slow (0.005/day)
        let create_30_days = calculate_freshness(7, 86400.0 * 30.0);
        assert!(create_30_days > 0.8);
    }
}
