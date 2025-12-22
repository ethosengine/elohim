/// Holochain Cache Core - High-Performance Content-Reach Aware Cache
///
/// Provides O(log n) eviction and cleanup operations instead of O(n).
/// Compiled to WebAssembly for use in Holochain client applications.
///
/// Optimized for Elohim Protocol's content-reach distributed caching strategy:
/// - Reach-level isolation (private → commons)
/// - Domain/Epic content organization
/// - Custodian-aware distribution with geographic awareness
/// - Mastery-based TTL with freshness decay
/// - Affinity-based content prioritization
/// - Steward economy support

use std::collections::{BTreeMap, HashMap};
use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

// Log macro for debugging (uses console.log in WASM)
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// =========================================================================
// Elohim Protocol Enums & Types
// =========================================================================

/// Content reach levels - determines who can access content
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ReachLevel {
    Private = 0,      // Only beneficiary
    Invited = 1,      // Explicitly invited individuals
    Local = 2,        // Family/household
    Neighborhood = 3, // Street block
    Municipal = 4,    // City/town
    Bioregional = 5,  // Watershed/ecosystem
    Regional = 6,     // State/province
    Commons = 7,      // Global/public
}

/// Content domains in Elohim Protocol
#[wasm_bindgen]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Domain {
    #[serde(rename = "elohim-protocol")]
    ElohimProtocol,
    #[serde(rename = "fct")]
    FCT, // Foundations for Christian Technology
    #[serde(rename = "ethosengine")]
    EthosEngine,
    #[serde(rename = "other")]
    Other,
}

/// Epic categories - content organization
#[wasm_bindgen]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Epic {
    #[serde(rename = "governance")]
    Governance,
    #[serde(rename = "autonomous_entity")]
    AutonomousEntity,
    #[serde(rename = "public_observer")]
    PublicObserver,
    #[serde(rename = "social_medium")]
    SocialMedium,
    #[serde(rename = "value_scanner")]
    ValueScanner,
    #[serde(rename = "economic_coordination")]
    EconomicCoordination,
    #[serde(rename = "lamad")]
    LAMAD,
    #[serde(rename = "other")]
    Other,
}

/// Mastery levels with associated freshness decay rates
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MasteryLevel {
    NotStarted = 0,  // decay: 0.0 (no decay, no mastery)
    Seen = 1,        // decay: 0.05 (passive viewing)
    Remember = 2,    // decay: 0.03 (moderate)
    Understand = 3,  // decay: 0.02
    Apply = 4,       // decay: 0.015 (demonstrated application)
    Analyze = 5,     // decay: 0.01
    Evaluate = 6,    // decay: 0.008 (ongoing participation)
    Create = 7,      // decay: 0.005 (slowest, creating maintains mastery)
}

impl MasteryLevel {
    /// Get the freshness decay rate per second for this mastery level
    pub fn decay_rate_per_second(&self) -> f64 {
        match self {
            MasteryLevel::NotStarted => 0.0,
            MasteryLevel::Seen => 0.05 / 86400.0,      // daily rate to per-second
            MasteryLevel::Remember => 0.03 / 86400.0,
            MasteryLevel::Understand => 0.02 / 86400.0,
            MasteryLevel::Apply => 0.015 / 86400.0,
            MasteryLevel::Analyze => 0.01 / 86400.0,
            MasteryLevel::Evaluate => 0.008 / 86400.0,
            MasteryLevel::Create => 0.005 / 86400.0,
        }
    }

    /// Calculate freshness (0.0-1.0) at a given time
    /// freshness = max(0, 1.0 - (decay_rate * age_seconds))
    pub fn calculate_freshness(&self, age_seconds: f64) -> f64 {
        let decay = self.decay_rate_per_second() * age_seconds;
        (1.0 - decay).max(0.0)
    }

    /// Get freshness categories
    pub fn freshness_status(&self, age_seconds: f64) -> &'static str {
        let freshness = self.calculate_freshness(age_seconds);
        if freshness >= 0.7 {
            "fresh"
        } else if freshness >= 0.4 {
            "stale"
        } else {
            "critical"
        }
    }
}

/// Custodian bandwidth class
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BandwidthClass {
    Ultra = 4,    // +20 points
    High = 3,     // +10 points
    Medium = 2,   // +5 points
    Low = 1,      // -5 points
}

impl BandwidthClass {
    pub fn score_bonus(&self) -> i32 {
        match self {
            BandwidthClass::Ultra => 20,
            BandwidthClass::High => 10,
            BandwidthClass::Medium => 5,
            BandwidthClass::Low => -5,
        }
    }
}

/// Custodian health status
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CustodianHealth {
    Healthy,    // All replicas present
    Degraded,   // 50%+ replicas present
    Critical,   // <50% replicas present
}

/// Steward tier for content curation
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum StewardTier {
    Caretaker = 1,  // Basic stewardship
    Curator = 2,    // Active curation
    Expert = 3,     // Domain expertise
    Pioneer = 4,    // Original research
}

impl StewardTier {
    pub fn priority_multiplier(&self) -> f64 {
        match self {
            StewardTier::Caretaker => 1.0,
            StewardTier::Curator => 1.2,
            StewardTier::Expert => 1.5,
            StewardTier::Pioneer => 2.0,
        }
    }
}

// =========================================================================
// Cache Entry with Elohim Protocol Metadata
// =========================================================================

/// Cache entry metadata with reach-aware and custodian support
#[wasm_bindgen]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub hash: String,
    pub size_bytes: u64,
    pub created_at: u64,
    pub last_accessed_at: u64,
    pub access_count: u32,

    // Elohim Protocol metadata
    pub reach_level: u8, // ReachLevel as u8 (0-7)
    pub domain: String, // "elohim-protocol", "fct", "ethosengine", "other"
    pub epic: String,   // "governance", "autonomous_entity", etc.
    pub custodian_id: Option<String>, // Agent ID of custodian
    pub steward_tier: u8, // StewardTier as u8 (1-4)
    pub mastery_level: u8, // MasteryLevel as u8 (0-7)

    // Geographic and performance data
    pub custodian_proximity_score: i32, // -100 to +100
    pub bandwidth_class: u8, // BandwidthClass as u8 (1-4)
    pub custodian_health: u8, // CustodianHealth as u8
    pub content_age_penalty: i32, // Penalty for aged content
    pub affinity_match: f64, // 0.0-1.0 affinity relevance
}

#[wasm_bindgen]
impl CacheEntry {
    /// Calculate total priority score for this entry
    /// Priority = base_reach + proximity + bandwidth + steward + freshness - penalties
    pub fn calculate_priority(&self) -> i32 {
        let mut score: i32 = 0;

        // Base reach level (commons = +100, private = 0)
        score += (self.reach_level as i32) * 12; // 0-84

        // Custodian proximity (-100 to +100)
        score += self.custodian_proximity_score;

        // Bandwidth class bonus
        let bandwidth = match self.bandwidth_class {
            4 => 20,   // Ultra
            3 => 10,   // High
            2 => 5,    // Medium
            1 => -5,   // Low
            _ => 0,
        };
        score += bandwidth;

        // Steward tier multiplier (as bonus points)
        let steward_bonus = match self.steward_tier {
            1 => 5,    // Caretaker
            2 => 15,   // Curator
            3 => 30,   // Expert
            4 => 50,   // Pioneer
            _ => 0,
        };
        score += steward_bonus;

        // Affinity bonus (0-10 points)
        score += (self.affinity_match * 10.0) as i32;

        // Penalties
        score -= self.content_age_penalty;

        // Clamp to reasonable range
        score.clamp(0, 200)
    }
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

// =========================================================================
// Reach-Aware Cache for Elohim Protocol
// =========================================================================

/// Multi-reach blob cache - maintains separate caches per reach level
/// This ensures content at different reach levels doesn't evict each other
#[wasm_bindgen]
pub struct ReachAwareBlobCache {
    // Separate cache per reach level (0-7)
    // reach_caches[i] = cache for reach level i
    reach_caches: Vec<BlobCache>,

    // Global stats across all reaches
    total_hits: u64,
    total_misses: u64,
    total_evictions: u64,

    // Domain/Epic index for faster queries
    // Key: "domain:epic" -> Vec of hashes
    domain_epic_index: HashMap<String, Vec<String>>,

    // Custodian index for replication tracking
    // Key: custodian_id -> Vec of (hash, reach_level)
    custodian_index: HashMap<String, Vec<(String, u8)>>,
}

#[wasm_bindgen]
impl ReachAwareBlobCache {
    /// Create new reach-aware blob cache
    /// max_size_per_reach: bytes limit per reach level
    #[wasm_bindgen(constructor)]
    pub fn new(max_size_per_reach: u64) -> ReachAwareBlobCache {
        let mut reach_caches = Vec::with_capacity(8);
        for _ in 0..8 {
            reach_caches.push(BlobCache::new(max_size_per_reach));
        }

        ReachAwareBlobCache {
            reach_caches,
            total_hits: 0,
            total_misses: 0,
            total_evictions: 0,
            domain_epic_index: HashMap::new(),
            custodian_index: HashMap::new(),
        }
    }

    /// Put entry in appropriate reach cache
    /// O(log n) amortized
    #[wasm_bindgen]
    pub fn put(&mut self, entry: CacheEntry) -> u32 {
        let reach = (entry.reach_level as usize).min(7);

        // Insert into reach cache
        let evicted = self.reach_caches[reach].put(entry.clone());

        // Update domain/epic index
        let key = format!("{}:{}", entry.domain, entry.epic);
        self.domain_epic_index
            .entry(key)
            .or_insert_with(Vec::new)
            .push(entry.hash.clone());

        // Update custodian index
        if let Some(ref custodian_id) = entry.custodian_id {
            self.custodian_index
                .entry(custodian_id.clone())
                .or_insert_with(Vec::new)
                .push((entry.hash, entry.reach_level));
        }

        evicted
    }

    /// Get entry from appropriate reach cache
    /// O(1) lookup
    #[wasm_bindgen]
    pub fn get(&mut self, hash: &str, reach_level: u8) -> Option<CacheEntry> {
        let reach = (reach_level as usize).min(7);

        match self.reach_caches[reach].get(hash) {
            Some(entry) => {
                self.total_hits += 1;
                Some(entry)
            }
            None => {
                self.total_misses += 1;
                None
            }
        }
    }

    /// Query entries by domain and epic
    /// Returns hashes of cached items matching criteria
    #[wasm_bindgen]
    pub fn query_by_domain_epic(&self, domain: &str, epic: &str) -> Vec<JsValue> {
        let key = format!("{}:{}", domain, epic);
        match self.domain_epic_index.get(&key) {
            Some(hashes) => hashes.iter().map(|h| JsValue::from_str(h)).collect(),
            None => Vec::new(),
        }
    }

    /// Query entries by custodian
    /// Returns list of (hash, reach_level) tuples
    #[wasm_bindgen]
    pub fn query_by_custodian(&self, custodian_id: &str) -> Vec<JsValue> {
        match self.custodian_index.get(custodian_id) {
            Some(entries) => entries
                .iter()
                .map(|(hash, reach)| {
                    JsValue::from_str(&format!("{}:{}", hash, reach))
                })
                .collect(),
            None => Vec::new(),
        }
    }

    /// Delete entry from cache
    /// O(log n)
    #[wasm_bindgen]
    pub fn delete(&mut self, hash: &str, reach_level: u8) -> bool {
        let reach = (reach_level as usize).min(7);
        let deleted = self.reach_caches[reach].delete(hash);

        // Clean up indices if entry was actually deleted
        if deleted {
            // Remove from domain/epic index
            for index_hashes in self.domain_epic_index.values_mut() {
                index_hashes.retain(|h| h != hash);
            }

            // Remove from custodian index
            for custodian_entries in self.custodian_index.values_mut() {
                custodian_entries.retain(|(h, _)| h != hash);
            }
        }

        deleted
    }

    /// Get all entries for a specific reach level
    /// Useful for reach-level-specific cleanup
    #[wasm_bindgen]
    pub fn get_reach_stats(&self, reach_level: u8) -> CacheStats {
        let reach = (reach_level as usize).min(7);
        self.reach_caches[reach].get_stats()
    }

    /// Get aggregated stats across all reaches
    #[wasm_bindgen]
    pub fn get_global_stats(&self) -> CacheStats {
        let mut total_items = 0u32;
        let mut total_size = 0u64;
        let mut total_evictions = 0u64;

        for cache in &self.reach_caches {
            let stats = cache.get_stats();
            total_items += stats.item_count;
            total_size += stats.total_size_bytes;
            total_evictions += stats.eviction_count;
        }

        CacheStats {
            item_count: total_items,
            total_size_bytes: total_size,
            eviction_count: total_evictions,
            hit_count: self.total_hits,
            miss_count: self.total_misses,
        }
    }

    /// Clean up expired items across all reach levels
    #[wasm_bindgen]
    pub fn cleanup_all_reaches(&mut self, now_millis: u64) -> u32 {
        let mut cleaned = 0;
        // Note: This is a placeholder - actual cleanup depends on entry TTLs
        // which are handled at the application level
        cleaned
    }

    /// Clear entire cache
    #[wasm_bindgen]
    pub fn clear_all(&mut self) {
        for cache in &mut self.reach_caches {
            cache.clear();
        }
        self.domain_epic_index.clear();
        self.custodian_index.clear();
        self.total_hits = 0;
        self.total_misses = 0;
        self.total_evictions = 0;
    }

    /// Get total cache size across all reaches
    #[wasm_bindgen]
    pub fn get_total_size(&self) -> u64 {
        self.reach_caches.iter().map(|c| c.get_total_size()).sum()
    }
}

/// Query builder for content-reach caching
#[wasm_bindgen]
pub struct CacheQuery {
    // Query filters
    pub reach_levels: Vec<u8>,
    pub domains: Vec<String>,
    pub epics: Vec<String>,
    pub custodians: Vec<String>,
    pub min_priority: i32,
    pub mastery_levels: Vec<u8>,
}

#[wasm_bindgen]
impl CacheQuery {
    /// Create new cache query builder
    #[wasm_bindgen(constructor)]
    pub fn new() -> CacheQuery {
        CacheQuery {
            reach_levels: Vec::new(),
            domains: Vec::new(),
            epics: Vec::new(),
            custodians: Vec::new(),
            min_priority: 0,
            mastery_levels: Vec::new(),
        }
    }

    /// Filter by reach level
    pub fn with_reach(&mut self, reach: u8) -> &mut Self {
        self.reach_levels.push(reach);
        self
    }

    /// Filter by domain
    pub fn with_domain(&mut self, domain: String) -> &mut Self {
        self.domains.push(domain);
        self
    }

    /// Filter by epic
    pub fn with_epic(&mut self, epic: String) -> &mut Self {
        self.epics.push(epic);
        self
    }

    /// Filter by custodian
    pub fn with_custodian(&mut self, custodian_id: String) -> &mut Self {
        self.custodians.push(custodian_id);
        self
    }

    /// Filter by minimum priority
    pub fn with_min_priority(&mut self, priority: i32) -> &mut Self {
        self.min_priority = priority;
        self
    }

    /// Filter by mastery level
    pub fn with_mastery(&mut self, level: u8) -> &mut Self {
        self.mastery_levels.push(level);
        self
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

    /// Helper to create test cache entries with sensible defaults
    fn test_entry(
        hash: &str,
        size_bytes: u64,
        reach_level: u8,
        domain: &str,
        epic: &str,
    ) -> CacheEntry {
        CacheEntry {
            hash: hash.to_string(),
            size_bytes,
            created_at: current_time_ms(),
            last_accessed_at: current_time_ms(),
            access_count: 0,
            reach_level,
            domain: domain.to_string(),
            epic: epic.to_string(),
            custodian_id: None,
            steward_tier: 1, // Caretaker
            mastery_level: 0, // NotStarted
            custodian_proximity_score: 0,
            bandwidth_class: 2, // Medium
            custodian_health: 0, // Healthy
            content_age_penalty: 0,
            affinity_match: 0.5,
        }
    }

    #[test]
    fn test_blob_cache_eviction() {
        let mut cache = BlobCache::new(1000);

        // Add items
        for i in 0..10 {
            let entry = test_entry(
                &format!("hash-{}", i),
                150,
                7, // commons
                "elohim-protocol",
                "governance",
            );
            cache.put(entry);
        }

        // Cache should have evicted oldest items to stay under 1000 bytes
        assert!(cache.get_total_size() <= 1000);
        assert!(cache.get_item_count() < 10);
    }

    #[test]
    fn test_reach_aware_cache() {
        let mut cache = ReachAwareBlobCache::new(5000);

        // Add items at different reach levels
        let commons_entry = test_entry("commons-hash", 1000, 7, "elohim-protocol", "governance");
        let private_entry = test_entry("private-hash", 1000, 0, "fct", "autonomous_entity");

        cache.put(commons_entry);
        cache.put(private_entry);

        // Both should be retrievable
        assert!(cache.get("commons-hash", 7).is_some());
        assert!(cache.get("private-hash", 0).is_some());

        // Query by domain/epic
        let governance_items = cache.query_by_domain_epic("elohim-protocol", "governance");
        assert!(!governance_items.is_empty());
    }

    #[test]
    fn test_chunk_cache_cleanup() {
        let mut cache = ChunkCache::new(10000, 5000); // 5 second TTL

        // Add old item (created at 0, now is 6000ms)
        let old_entry = test_entry("old", 100, 7, "fct", "governance");
        let mut old_with_time = old_entry;
        old_with_time.created_at = 0; // Very old

        cache.put(old_with_time);

        // Clean up (6000 ms elapsed)
        let cleaned = cache.cleanup_expired(6000);

        // Old item should be cleaned (TTL is 5000ms, age is 6000ms)
        assert_eq!(cleaned, 1);
        assert!(cache.get("old").is_none());
    }

    #[test]
    fn test_priority_calculation() {
        let mut entry = test_entry("test", 100, 7, "elohim-protocol", "governance");

        // Base reach level: 7 * 12 = 84
        // Proximity: 0
        // Bandwidth: 5 (medium)
        // Steward: 5 (caretaker)
        // Affinity: 5 (0.5 * 10)
        // Total: 84 + 0 + 5 + 5 + 5 = 99
        let priority = entry.calculate_priority();
        assert!(priority > 80 && priority < 120);
    }

    #[test]
    fn test_mastery_freshness() {
        // Create entry with Evaluate mastery
        let entry = test_entry("test", 100, 7, "elohim-protocol", "governance");

        // Freshness immediately after creation should be near 1.0
        let freshness = MasteryLevel::Evaluate.calculate_freshness(0.0);
        assert!(freshness > 0.99);

        // After 1 day, should decay
        let freshness_1day = MasteryLevel::Evaluate.calculate_freshness(86400.0);
        assert!(freshness_1day < 0.99);
        assert!(freshness_1day > 0.90);
    }
}
