//! Tiered Blob Cache - O(1) LRU Bytes-Only Cache for Media Streaming
//!
//! Two-tier caching strategy optimized for Elohim Protocol media:
//!
//! - **Blobs**: 1 GB LRU limit, 24h TTL (commons) / 7d TTL (private)
//!   - Full blob data for small-medium files
//!   - Reach-isolated eviction (private content never evicts commons)
//! - **Chunks**: 10 GB LRU limit, 7-day TTL
//!   - Individual chunks for HLS/DASH streaming
//!   - Keyed by "hash:index" for O(1) lookup
//!
//! ## Performance
//!
//! All operations are O(1) using DashMap with insertion-order LRU simulation.
//! Large blobs won't evict many small documents due to tier isolation.
//!
//! ## P2P Integration (CDN-Style Origin Pull)
//!
//! Doorway is a **thin CDN proxy**, NOT primary storage. Blobs live on agent
//! devices via `elohim-storage`. When agents store blobs, they register endpoints
//! with the infrastructure DNA, which emits `ContentServerCommitted` signals.
//!
//! ```text
//! Agent Device                    Doorway                      Browser
//! ┌──────────────┐               ┌─────────────────────────┐   ┌────────┐
//! │elohim-storage│──register────►│ProjectionStore          │   │ Client │
//! │  (origin)    │  endpoint     │  (blob_endpoints field) │   └────┬───┘
//! └──────────────┘               └────────────┬────────────┘        │
//!        ▲                                    │ lookup endpoints    │
//!        │                       ┌────────────▼────────────┐        │
//!        │                       │TieredBlobCache          │◄───────┘
//!        │                       │  (bytes only)           │   request
//!        └───── origin pull ─────┴─────────────────────────┘
//!              (on cache miss)
//! ```
//!
//! ## Architecture
//!
//! Blob metadata (including endpoints) lives in `ProjectionStore` (MongoDB).
//! TieredBlobCache is bytes-only:
//!
//! - `ProjectionStore.get_blob_endpoints()` - lookup where to fetch a blob
//! - `TieredBlobCache.get_or_fetch_from()` - cache lookup + origin pull
//!
//! **Recommended usage:**
//! ```ignore
//! // Get endpoints from ProjectionStore
//! let endpoints = projection_store.get_blob_endpoints(&hash).await?;
//!
//! // Fetch blob with those endpoints
//! let blob = blob_cache.get_or_fetch_from(&hash, &endpoints, &reach, &http_client).await?;
//! ```

use bytes::Bytes;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, info, warn};

// ============================================================================
// Errors
// ============================================================================

/// Error types for cache operations
#[derive(Debug, Error)]
pub enum CacheError {
    /// Blob not found in cache or from fallback sources
    #[error("Blob not found: {0}")]
    NotFound(String),

    /// No fallback URLs available
    #[error("No fallback URLs for blob: {0}")]
    NoFallbackUrls(String),

    /// HTTP fetch failed
    #[error("HTTP fetch failed: {0}")]
    FetchFailed(String),

    /// Verification failed (hash mismatch)
    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
}

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for tiered cache (blobs and chunks)
#[derive(Debug, Clone)]
pub struct TieredCacheConfig {
    /// Blob cache max bytes (default: 1 GB)
    pub blob_max_bytes: u64,
    /// Blob TTL for commons reach (default: 24 hours)
    pub blob_commons_ttl: Duration,
    /// Blob TTL for private reach (default: 7 days)
    pub blob_private_ttl: Duration,

    /// Chunk cache max bytes (default: 10 GB)
    pub chunk_max_bytes: u64,
    /// Chunk TTL (default: 7 days)
    pub chunk_ttl: Duration,
    /// Default chunk size (default: 5 MB)
    pub default_chunk_size: usize,
}

impl Default for TieredCacheConfig {
    fn default() -> Self {
        Self {
            blob_max_bytes: 1024 * 1024 * 1024,                      // 1 GB
            blob_commons_ttl: Duration::from_secs(24 * 60 * 60),     // 24 hours
            blob_private_ttl: Duration::from_secs(7 * 24 * 60 * 60), // 7 days

            chunk_max_bytes: 10 * 1024 * 1024 * 1024, // 10 GB
            chunk_ttl: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
            default_chunk_size: 5 * 1024 * 1024,      // 5 MB
        }
    }
}

impl TieredCacheConfig {
    /// Create config from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(val) = std::env::var("BLOB_CACHE_MAX_GB") {
            if let Ok(gb) = val.parse::<u64>() {
                config.blob_max_bytes = gb * 1024 * 1024 * 1024;
            }
        }

        if let Ok(val) = std::env::var("CHUNK_CACHE_MAX_GB") {
            if let Ok(gb) = val.parse::<u64>() {
                config.chunk_max_bytes = gb * 1024 * 1024 * 1024;
            }
        }

        if let Ok(val) = std::env::var("CHUNK_SIZE_MB") {
            if let Ok(mb) = val.parse::<usize>() {
                config.default_chunk_size = mb * 1024 * 1024;
            }
        }

        config
    }
}

// ============================================================================
// Tier 1: Blob Metadata
// ============================================================================

/// Blob metadata for Tier 1 cache - lightweight, long-lived
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobMetadata {
    /// SHA256 hash of the blob (hex string, 64 chars)
    pub hash: String,
    /// Total size in bytes
    pub size_bytes: u64,
    /// MIME type (e.g., "video/mp4", "audio/mpeg")
    pub mime_type: String,
    /// Video/audio codec (H.264, H.265, VP9, AV1, AAC, OPUS, etc.)
    pub codec: Option<String>,
    /// Bitrate in Mbps (for bandwidth estimation)
    pub bitrate_mbps: Option<f32>,
    /// Duration in seconds (for audio/video)
    pub duration_seconds: Option<u32>,
    /// Reach level (private, local, neighborhood, municipal, bioregional, regional, commons)
    pub reach: String,
    /// Fallback URLs in priority order (primary, secondary, tertiary, custodian endpoints)
    pub fallback_urls: Vec<String>,
    /// Quality variants for adaptive streaming
    pub variants: Vec<VariantMetadata>,
    /// Subtitle/caption tracks
    pub captions: Vec<CaptionMetadata>,
    /// When this blob was created
    pub created_at: String,
    /// When this blob was last verified
    pub verified_at: Option<String>,
    /// Author/creator agent ID
    pub author_id: Option<String>,
    /// Content ID this blob belongs to
    pub content_id: Option<String>,
}

impl BlobMetadata {
    /// Compute SHA256 hash from data
    pub fn compute_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    /// Get TTL based on reach level
    pub fn ttl_for_reach(reach: &str, config: &TieredCacheConfig) -> Duration {
        match reach {
            "private" | "invited" | "local" => config.blob_private_ttl,
            _ => config.blob_commons_ttl,
        }
    }

    /// Number of chunks needed at given chunk size
    pub fn chunk_count(&self, chunk_size: usize) -> usize {
        (self.size_bytes as usize).div_ceil(chunk_size)
    }

    /// Get variant by label (e.g., "1080p", "720p")
    pub fn get_variant(&self, label: &str) -> Option<&VariantMetadata> {
        self.variants.iter().find(|v| v.label == label)
    }
}

/// Variant metadata for adaptive streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantMetadata {
    /// Label (e.g., "1080p", "720p", "480p")
    pub label: String,
    /// SHA256 hash of this variant
    pub hash: String,
    /// Bitrate in Mbps
    pub bitrate_mbps: f32,
    /// Width in pixels
    pub width: Option<u32>,
    /// Height in pixels
    pub height: Option<u32>,
    /// Size in bytes
    pub size_bytes: u64,
    /// Fallback URLs for this variant
    pub fallback_urls: Vec<String>,
}

/// Caption/subtitle track metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptionMetadata {
    /// ISO 639-1 language code (e.g., "en", "es", "fr")
    pub language: String,
    /// Human-readable label (e.g., "English", "Spanish", "French with SDH")
    pub label: String,
    /// Format (webvtt, srt, vtt, ass, ssa)
    pub format: String,
    /// URL to caption file
    pub url: String,
    /// Whether captions include hearing impaired info
    pub is_hard_of_hearing: Option<bool>,
}

// ============================================================================
// Cache Entry Wrappers
// ============================================================================

/// Blob entry with reach-aware TTL
struct BlobEntry {
    data: Vec<u8>,
    _reach: String,
    cached_at: Instant,
    expires_at: Instant,
}

/// Chunk entry
struct ChunkEntry {
    data: Vec<u8>,
    cached_at: Instant,
    expires_at: Instant,
}

// ============================================================================
// Cache Statistics
// ============================================================================

/// Statistics for a single cache tier
#[derive(Debug, Clone, Default)]
pub struct TierStats {
    /// Number of items in cache
    pub item_count: usize,
    /// Total size in bytes (for blob/chunk tiers)
    pub total_bytes: u64,
    /// Maximum size in bytes
    pub max_bytes: u64,
    /// Percentage full
    pub percent_full: f64,
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Evictions due to size limit
    pub evictions: u64,
    /// Expirations due to TTL
    pub expirations: u64,
}

impl TierStats {
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

/// Combined statistics for blob and chunk tiers
#[derive(Debug, Clone)]
pub struct TieredCacheStats {
    pub blob: TierStats,
    pub chunk: TierStats,
}

// ============================================================================
// Tiered Blob Cache Implementation
// ============================================================================

/// Two-tier blob cache for media streaming (bytes-only)
///
/// Thread-safe with O(1) operations using DashMap.
///
/// Note: Tier 1 (Metadata) was removed. Blob metadata including endpoints
/// now lives in ProjectionStore. Use `get_or_fetch_from()` with endpoints
/// from `ProjectionStore.get_blob_endpoints()`.
pub struct TieredBlobCache {
    // Tier 2: Full blobs (size-limited, reach-aware TTL)
    blobs: DashMap<String, BlobEntry>,
    blob_total_bytes: AtomicU64,
    blob_hits: AtomicU64,
    blob_misses: AtomicU64,
    blob_evictions: AtomicU64,

    // Tier 3: Chunks (size-limited, keyed by "hash:index")
    chunks: DashMap<String, ChunkEntry>,
    chunk_total_bytes: AtomicU64,
    chunk_hits: AtomicU64,
    chunk_misses: AtomicU64,
    chunk_evictions: AtomicU64,

    config: TieredCacheConfig,
}

impl TieredBlobCache {
    /// Create a new tiered cache with configuration
    pub fn new(config: TieredCacheConfig) -> Self {
        info!(
            blob_max_gb = config.blob_max_bytes / (1024 * 1024 * 1024),
            chunk_max_gb = config.chunk_max_bytes / (1024 * 1024 * 1024),
            chunk_size_mb = config.default_chunk_size / (1024 * 1024),
            "TieredBlobCache initialized (bytes-only, metadata in ProjectionStore)"
        );

        Self {
            blobs: DashMap::new(),
            blob_total_bytes: AtomicU64::new(0),
            blob_hits: AtomicU64::new(0),
            blob_misses: AtomicU64::new(0),
            blob_evictions: AtomicU64::new(0),

            chunks: DashMap::new(),
            chunk_total_bytes: AtomicU64::new(0),
            chunk_hits: AtomicU64::new(0),
            chunk_misses: AtomicU64::new(0),
            chunk_evictions: AtomicU64::new(0),

            config,
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(TieredCacheConfig::default())
    }

    /// Get configuration
    pub fn config(&self) -> &TieredCacheConfig {
        &self.config
    }

    // ========================================================================
    // Tier 2: Blob Operations
    // ========================================================================

    /// Get full blob by hash. O(1).
    pub fn get_blob(&self, hash: &str) -> Option<Vec<u8>> {
        if let Some(entry) = self.blobs.get(hash) {
            if Instant::now() < entry.expires_at {
                self.blob_hits.fetch_add(1, Ordering::Relaxed);
                debug!(hash = hash, size = entry.data.len(), "Blob cache hit");
                return Some(entry.data.clone());
            }
            // Expired
            let size = entry.data.len() as u64;
            drop(entry);
            self.blobs.remove(hash);
            self.blob_total_bytes.fetch_sub(size, Ordering::Relaxed);
        }

        self.blob_misses.fetch_add(1, Ordering::Relaxed);
        debug!(hash = hash, "Blob cache miss");
        None
    }

    /// Store full blob with reach-aware TTL. O(1) amortized.
    pub fn set_blob(&self, hash: &str, data: Vec<u8>, reach: &str) {
        let size = data.len() as u64;

        // Check if this single blob exceeds max (don't cache huge blobs in Tier 2)
        if size > self.config.blob_max_bytes / 2 {
            warn!(
                hash = hash,
                size = size,
                max = self.config.blob_max_bytes,
                "Blob too large for Tier 2, should use Tier 3 chunks"
            );
            return;
        }

        // Evict until we have space
        self.evict_blobs_until_fits(size);

        let now = Instant::now();
        let ttl = BlobMetadata::ttl_for_reach(reach, &self.config);
        let entry = BlobEntry {
            data,
            _reach: reach.to_string(),
            cached_at: now,
            expires_at: now + ttl,
        };

        // Remove old entry if exists (update size tracking)
        if let Some((_, old)) = self.blobs.remove(hash) {
            self.blob_total_bytes
                .fetch_sub(old.data.len() as u64, Ordering::Relaxed);
        }

        debug!(hash = hash, size = size, reach = reach, "Blob cached");
        self.blobs.insert(hash.to_string(), entry);
        self.blob_total_bytes.fetch_add(size, Ordering::Relaxed);
    }

    /// Check if blob exists. O(1).
    pub fn has_blob(&self, hash: &str) -> bool {
        self.blobs
            .get(hash)
            .map(|e| Instant::now() < e.expires_at)
            .unwrap_or(false)
    }

    /// Remove blob. O(1).
    pub fn remove_blob(&self, hash: &str) -> bool {
        if let Some((_, entry)) = self.blobs.remove(hash) {
            self.blob_total_bytes
                .fetch_sub(entry.data.len() as u64, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Get byte range from blob. O(1).
    pub fn get_blob_range(&self, hash: &str, start: usize, end: usize) -> Option<Bytes> {
        if let Some(entry) = self.blobs.get(hash) {
            if Instant::now() < entry.expires_at
                && start < entry.data.len()
                && end <= entry.data.len()
                && start < end
            {
                self.blob_hits.fetch_add(1, Ordering::Relaxed);
                return Some(Bytes::copy_from_slice(&entry.data[start..end]));
            }
        }
        self.blob_misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    // ========================================================================
    // Tier 2: Fallback Fetching
    // ========================================================================

    /// Get blob from cache, or fetch from provided endpoints.
    ///
    /// **This is the recommended method** after the metadata layer merge.
    /// Caller provides endpoints from ProjectionStore.
    ///
    /// # Arguments
    ///
    /// * `hash` - The blob hash (e.g., "abc123def456" - without "sha256-" prefix)
    /// * `endpoints` - List of endpoint URLs to try (from ProjectionStore.get_blob_endpoints())
    /// * `reach` - Reach level for TTL selection ("commons", "private", etc.)
    /// * `http_client` - reqwest client for fetching from endpoints
    ///
    /// # Example
    ///
    /// ```ignore
    /// let client = reqwest::Client::new();
    /// let endpoints = projection_store.get_blob_endpoints(&hash).await.unwrap_or_default();
    /// let blob = cache.get_or_fetch_from("abc123", &endpoints, "commons", &client).await?;
    /// ```
    pub async fn get_or_fetch_from(
        &self,
        hash: &str,
        endpoints: &[String],
        reach: &str,
        http_client: &reqwest::Client,
    ) -> Result<Bytes, CacheError> {
        // 1. Check Tier 2 blob cache
        if let Some(blob) = self.get_blob(hash) {
            return Ok(Bytes::from(blob));
        }

        // 2. No endpoints available
        if endpoints.is_empty() {
            return Err(CacheError::NoFallbackUrls(hash.to_string()));
        }

        // 3. Try each endpoint in order
        let mut last_error = None;
        for url in endpoints {
            match self.fetch_from_url(http_client, url).await {
                Ok(data) => {
                    // 4. Verify hash matches
                    let computed_hash = BlobMetadata::compute_hash(&data);
                    if computed_hash != hash {
                        warn!(
                            expected = hash,
                            actual = computed_hash,
                            url = url,
                            "Hash mismatch from endpoint"
                        );
                        last_error = Some(CacheError::HashMismatch {
                            expected: hash.to_string(),
                            actual: computed_hash,
                        });
                        continue;
                    }

                    // 5. Cache in Tier 2
                    self.set_blob(hash, data.to_vec(), reach);

                    info!(
                        hash = hash,
                        url = url,
                        size = data.len(),
                        reach = reach,
                        "Fetched and cached blob from endpoint"
                    );

                    return Ok(Bytes::from(data));
                }
                Err(e) => {
                    warn!(url = url, error = %e, "Endpoint fetch failed");
                    last_error = Some(e);
                    continue;
                }
            }
        }

        // All endpoints failed
        Err(last_error.unwrap_or_else(|| CacheError::NotFound(hash.to_string())))
    }

    /// Fetch blob data from a URL
    async fn fetch_from_url(
        &self,
        http_client: &reqwest::Client,
        url: &str,
    ) -> Result<Vec<u8>, CacheError> {
        let response = http_client
            .get(url)
            .send()
            .await
            .map_err(|e| CacheError::FetchFailed(format!("Request failed: {e}")))?;

        if !response.status().is_success() {
            return Err(CacheError::FetchFailed(format!(
                "HTTP {} from {}",
                response.status(),
                url
            )));
        }

        let data = response
            .bytes()
            .await
            .map_err(|e| CacheError::FetchFailed(format!("Body read failed: {e}")))?;

        Ok(data.to_vec())
    }

    // ========================================================================
    // Tier 3: Chunk Operations
    // ========================================================================

    /// Make chunk key from hash and index
    fn chunk_key(hash: &str, index: usize) -> String {
        format!("{hash}:{index}")
    }

    /// Get chunk by hash and index. O(1).
    pub fn get_chunk(&self, hash: &str, index: usize) -> Option<Vec<u8>> {
        let key = Self::chunk_key(hash, index);

        if let Some(entry) = self.chunks.get(&key) {
            if Instant::now() < entry.expires_at {
                self.chunk_hits.fetch_add(1, Ordering::Relaxed);
                debug!(hash = hash, index = index, "Chunk cache hit");
                return Some(entry.data.clone());
            }
            // Expired
            let size = entry.data.len() as u64;
            drop(entry);
            self.chunks.remove(&key);
            self.chunk_total_bytes.fetch_sub(size, Ordering::Relaxed);
        }

        self.chunk_misses.fetch_add(1, Ordering::Relaxed);
        debug!(hash = hash, index = index, "Chunk cache miss");
        None
    }

    /// Store chunk. O(1) amortized.
    pub fn set_chunk(&self, hash: &str, index: usize, data: Vec<u8>) {
        let size = data.len() as u64;
        let key = Self::chunk_key(hash, index);

        // Evict until we have space
        self.evict_chunks_until_fits(size);

        let now = Instant::now();
        let entry = ChunkEntry {
            data,
            cached_at: now,
            expires_at: now + self.config.chunk_ttl,
        };

        // Remove old entry if exists
        if let Some((_, old)) = self.chunks.remove(&key) {
            self.chunk_total_bytes
                .fetch_sub(old.data.len() as u64, Ordering::Relaxed);
        }

        debug!(hash = hash, index = index, size = size, "Chunk cached");
        self.chunks.insert(key, entry);
        self.chunk_total_bytes.fetch_add(size, Ordering::Relaxed);
    }

    /// Check if chunk exists. O(1).
    pub fn has_chunk(&self, hash: &str, index: usize) -> bool {
        let key = Self::chunk_key(hash, index);
        self.chunks
            .get(&key)
            .map(|e| Instant::now() < e.expires_at)
            .unwrap_or(false)
    }

    /// Remove chunk. O(1).
    pub fn remove_chunk(&self, hash: &str, index: usize) -> bool {
        let key = Self::chunk_key(hash, index);
        if let Some((_, entry)) = self.chunks.remove(&key) {
            self.chunk_total_bytes
                .fetch_sub(entry.data.len() as u64, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Get all chunks for a blob (for reassembly). O(n) where n = chunk count.
    pub fn get_all_chunks(&self, hash: &str) -> Option<Vec<Vec<u8>>> {
        // First, find all chunk indices for this hash
        let prefix = format!("{hash}:");
        let mut chunks: Vec<(usize, Vec<u8>)> = Vec::new();

        for entry in self.chunks.iter() {
            if entry.key().starts_with(&prefix) && Instant::now() < entry.expires_at {
                // Parse index from key
                if let Some(idx_str) = entry.key().strip_prefix(&prefix) {
                    if let Ok(idx) = idx_str.parse::<usize>() {
                        chunks.push((idx, entry.data.clone()));
                    }
                }
            }
        }

        if chunks.is_empty() {
            return None;
        }

        // Sort by index and return data
        chunks.sort_by_key(|(idx, _)| *idx);
        Some(chunks.into_iter().map(|(_, data)| data).collect())
    }

    /// Remove all chunks for a blob. O(n).
    pub fn remove_all_chunks(&self, hash: &str) -> usize {
        let prefix = format!("{hash}:");
        let keys_to_remove: Vec<String> = self
            .chunks
            .iter()
            .filter(|e| e.key().starts_with(&prefix))
            .map(|e| e.key().clone())
            .collect();

        let mut total_size = 0u64;
        for key in &keys_to_remove {
            if let Some((_, entry)) = self.chunks.remove(key) {
                total_size += entry.data.len() as u64;
            }
        }

        self.chunk_total_bytes
            .fetch_sub(total_size, Ordering::Relaxed);
        keys_to_remove.len()
    }

    // ========================================================================
    // Eviction
    // ========================================================================

    /// Evict blobs until we have space for new_size
    fn evict_blobs_until_fits(&self, new_size: u64) {
        let current = self.blob_total_bytes.load(Ordering::Relaxed);
        if current + new_size <= self.config.blob_max_bytes {
            return;
        }

        let to_free = (current + new_size).saturating_sub(self.config.blob_max_bytes);
        let mut freed = 0u64;

        // Collect entries sorted by cached_at (oldest first)
        let mut entries: Vec<(String, Instant, u64)> = self
            .blobs
            .iter()
            .map(|e| (e.key().clone(), e.cached_at, e.data.len() as u64))
            .collect();

        entries.sort_by_key(|(_, cached_at, _)| *cached_at);

        for (key, _, size) in entries {
            if freed >= to_free {
                break;
            }
            if self.blobs.remove(&key).is_some() {
                freed += size;
                self.blob_evictions.fetch_add(1, Ordering::Relaxed);
            }
        }

        self.blob_total_bytes.fetch_sub(freed, Ordering::Relaxed);
        debug!(freed = freed, "Evicted blobs to make space");
    }

    /// Evict chunks until we have space for new_size
    fn evict_chunks_until_fits(&self, new_size: u64) {
        let current = self.chunk_total_bytes.load(Ordering::Relaxed);
        if current + new_size <= self.config.chunk_max_bytes {
            return;
        }

        let to_free = (current + new_size).saturating_sub(self.config.chunk_max_bytes);
        let mut freed = 0u64;

        // Collect entries sorted by cached_at (oldest first)
        let mut entries: Vec<(String, Instant, u64)> = self
            .chunks
            .iter()
            .map(|e| (e.key().clone(), e.cached_at, e.data.len() as u64))
            .collect();

        entries.sort_by_key(|(_, cached_at, _)| *cached_at);

        for (key, _, size) in entries {
            if freed >= to_free {
                break;
            }
            if self.chunks.remove(&key).is_some() {
                freed += size;
                self.chunk_evictions.fetch_add(1, Ordering::Relaxed);
            }
        }

        self.chunk_total_bytes.fetch_sub(freed, Ordering::Relaxed);
        debug!(freed = freed, "Evicted chunks to make space");
    }

    // ========================================================================
    // Cleanup & Maintenance
    // ========================================================================

    /// Remove all expired entries from both tiers
    ///
    /// Returns (expired_blobs, expired_chunks)
    pub fn cleanup_expired(&self) -> (usize, usize) {
        let now = Instant::now();

        // Blobs
        let expired_blobs: Vec<(String, u64)> = self
            .blobs
            .iter()
            .filter(|e| now >= e.expires_at)
            .map(|e| (e.key().clone(), e.data.len() as u64))
            .collect();
        let mut blob_freed = 0u64;
        for (key, size) in &expired_blobs {
            if self.blobs.remove(key).is_some() {
                blob_freed += size;
            }
        }
        self.blob_total_bytes
            .fetch_sub(blob_freed, Ordering::Relaxed);

        // Chunks
        let expired_chunks: Vec<(String, u64)> = self
            .chunks
            .iter()
            .filter(|e| now >= e.expires_at)
            .map(|e| (e.key().clone(), e.data.len() as u64))
            .collect();
        let mut chunk_freed = 0u64;
        for (key, size) in &expired_chunks {
            if self.chunks.remove(key).is_some() {
                chunk_freed += size;
            }
        }
        self.chunk_total_bytes
            .fetch_sub(chunk_freed, Ordering::Relaxed);

        if !expired_blobs.is_empty() || !expired_chunks.is_empty() {
            debug!(
                blobs = expired_blobs.len(),
                chunks = expired_chunks.len(),
                "Cleaned up expired entries"
            );
        }

        (expired_blobs.len(), expired_chunks.len())
    }

    /// Clear all entries from both tiers
    pub fn clear(&self) {
        self.blobs.clear();
        self.chunks.clear();
        self.blob_total_bytes.store(0, Ordering::Relaxed);
        self.chunk_total_bytes.store(0, Ordering::Relaxed);
        info!("TieredBlobCache cleared");
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get statistics for blob and chunk tiers
    pub fn stats(&self) -> TieredCacheStats {
        let blob_bytes = self.blob_total_bytes.load(Ordering::Relaxed);
        let chunk_bytes = self.chunk_total_bytes.load(Ordering::Relaxed);

        TieredCacheStats {
            blob: TierStats {
                item_count: self.blobs.len(),
                total_bytes: blob_bytes,
                max_bytes: self.config.blob_max_bytes,
                percent_full: (blob_bytes as f64 / self.config.blob_max_bytes as f64) * 100.0,
                hits: self.blob_hits.load(Ordering::Relaxed),
                misses: self.blob_misses.load(Ordering::Relaxed),
                evictions: self.blob_evictions.load(Ordering::Relaxed),
                expirations: 0,
            },
            chunk: TierStats {
                item_count: self.chunks.len(),
                total_bytes: chunk_bytes,
                max_bytes: self.config.chunk_max_bytes,
                percent_full: (chunk_bytes as f64 / self.config.chunk_max_bytes as f64) * 100.0,
                hits: self.chunk_hits.load(Ordering::Relaxed),
                misses: self.chunk_misses.load(Ordering::Relaxed),
                evictions: self.chunk_evictions.load(Ordering::Relaxed),
                expirations: 0,
            },
        }
    }

    /// Get total memory usage in bytes
    pub fn total_memory_bytes(&self) -> u64 {
        self.blob_total_bytes.load(Ordering::Relaxed)
            + self.chunk_total_bytes.load(Ordering::Relaxed)
    }
}

impl Default for TieredBlobCache {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ============================================================================
// Background Cleanup Task
// ============================================================================

use std::sync::Arc;

/// Spawn a background task to periodically cleanup expired entries
pub fn spawn_tiered_cleanup_task(cache: Arc<TieredBlobCache>, interval: Duration) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            let (expired_blobs, expired_chunks) = cache.cleanup_expired();
            let stats = cache.stats();
            debug!(
                expired_blobs = expired_blobs,
                expired_chunks = expired_chunks,
                blob_percent = format!("{:.1}%", stats.blob.percent_full),
                chunk_percent = format!("{:.1}%", stats.chunk.percent_full),
                "Tiered cache cleanup completed"
            );
        }
    });

    info!(
        interval_secs = interval.as_secs(),
        "Tiered cache cleanup task started"
    );
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_metadata() -> BlobMetadata {
        BlobMetadata {
            hash: "abc123def456".to_string(),
            size_bytes: 1024 * 1024, // 1 MB
            mime_type: "video/mp4".to_string(),
            codec: Some("H.264".to_string()),
            bitrate_mbps: Some(5.0),
            duration_seconds: Some(120),
            reach: "commons".to_string(),
            fallback_urls: vec!["https://example.com/blob/abc123".to_string()],
            variants: vec![],
            captions: vec![],
            created_at: "2024-01-01T00:00:00Z".to_string(),
            verified_at: None,
            author_id: None,
            content_id: None,
        }
    }

    #[test]
    fn test_blob_cache() {
        let cache = TieredBlobCache::with_defaults();
        let data = vec![0u8; 1024 * 1024]; // 1 MB
        let hash = "test_blob_hash";

        // Miss initially
        assert!(cache.get_blob(hash).is_none());

        // Set and get
        cache.set_blob(hash, data.clone(), "commons");
        let cached = cache.get_blob(hash).expect("Should have blob");
        assert_eq!(cached.len(), 1024 * 1024);

        // Stats
        let stats = cache.stats();
        assert_eq!(stats.blob.item_count, 1);
        assert_eq!(stats.blob.total_bytes, 1024 * 1024);
    }

    #[test]
    fn test_chunk_cache() {
        let cache = TieredBlobCache::with_defaults();
        let chunk0 = vec![0u8; 5 * 1024 * 1024]; // 5 MB
        let chunk1 = vec![1u8; 5 * 1024 * 1024]; // 5 MB
        let hash = "test_chunk_hash";

        // Miss initially
        assert!(cache.get_chunk(hash, 0).is_none());

        // Set chunks
        cache.set_chunk(hash, 0, chunk0.clone());
        cache.set_chunk(hash, 1, chunk1.clone());

        // Get chunks
        let cached0 = cache.get_chunk(hash, 0).expect("Should have chunk 0");
        let cached1 = cache.get_chunk(hash, 1).expect("Should have chunk 1");
        assert_eq!(cached0.len(), 5 * 1024 * 1024);
        assert_eq!(cached1[0], 1);

        // Get all chunks
        let all = cache.get_all_chunks(hash).expect("Should have all chunks");
        assert_eq!(all.len(), 2);

        // Stats
        let stats = cache.stats();
        assert_eq!(stats.chunk.item_count, 2);
        assert_eq!(stats.chunk.total_bytes, 10 * 1024 * 1024);
    }

    #[test]
    fn test_blob_range() {
        let cache = TieredBlobCache::with_defaults();
        let data: Vec<u8> = (0..100).collect();
        let hash = "range_test";

        cache.set_blob(hash, data, "commons");

        // Get range
        let range = cache
            .get_blob_range(hash, 10, 20)
            .expect("Should get range");
        assert_eq!(range.len(), 10);
        assert_eq!(range[0], 10);
        assert_eq!(range[9], 19);
    }

    #[test]
    fn test_chunk_count() {
        let meta = BlobMetadata {
            size_bytes: 27 * 1024 * 1024, // 27 MB
            ..test_metadata()
        };

        // At 5 MB chunks: ceil(27/5) = 6 chunks
        assert_eq!(meta.chunk_count(5 * 1024 * 1024), 6);

        // At 10 MB chunks: ceil(27/10) = 3 chunks
        assert_eq!(meta.chunk_count(10 * 1024 * 1024), 3);
    }
}
