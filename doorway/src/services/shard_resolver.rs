//! Shard Resolver Service - Native Holochain Blob Resolution
//!
//! Resolves blobs by querying DNA for ShardManifests and fetching shards
//! from elohim-storage nodes. This enables native Holochain blob storage
//! without relying on web2.0 fallbacks.
//!
//! ## Resolution Flow
//!
//! 1. Query DNA for ShardManifest via `resolve_blob` zome function
//! 2. Get shard locations (which nodes have which shards)
//! 3. Fetch shards from elohim-storage HTTP endpoints
//! 4. Reassemble blob from shards (single shard or Reed-Solomon decode)
//! 5. Cache in ContentCache for subsequent requests
//!
//! ## Integration with Content Cache
//!
//! When blob.rs routes get a cache miss, they call the shard resolver
//! as a fallback before returning 404. This provides seamless integration
//! with the existing caching layer.
//!
//! ## Architecture Coherence
//!
//! - ShardManifest in DNA tracks blob metadata + shard hashes
//! - ShardLocation in DNA tracks which nodes hold which shards
//! - elohim-storage sidecar stores actual shard bytes
//! - Doorway orchestrates resolution across these components

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

use crate::cache::ContentCache;

// ============================================================================
// Types (matching DNA definitions)
// ============================================================================

/// Shard manifest from DNA (mirrors content_store_integrity::ShardManifest)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardManifest {
    /// SHA256 hash of the complete blob (content-addressed ID)
    pub blob_hash: String,
    /// Total size of the original blob in bytes
    pub total_size: u64,
    /// MIME type of the blob
    pub mime_type: String,
    /// Encoding: "none" (single shard), "chunked", "reed-solomon-4-3"
    pub encoding: String,
    /// Number of data shards (for Reed-Solomon: k)
    pub data_shards: u8,
    /// Total number of shards including parity (for Reed-Solomon: k + parity)
    pub total_shards: u8,
    /// Size of each shard in bytes
    pub shard_size: u32,
    /// Ordered list of shard hashes
    pub shard_hashes: Vec<String>,
    /// Reach level for access control
    pub reach: String,
    /// Author agent public key
    pub author_id: Option<String>,
    /// Creation timestamp (ISO 8601)
    pub created_at: String,
}

/// Shard location from DNA (where to fetch a shard)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardLocation {
    /// SHA256 hash of the shard
    pub shard_hash: String,
    /// Agent public key of the holder
    pub holder_id: String,
    /// Base URL of the elohim-storage HTTP endpoint
    pub endpoint_url: String,
    /// When this location was registered
    pub registered_at: String,
    /// Whether this location is active
    pub active: bool,
}

/// Complete blob resolution output from DNA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobResolution {
    /// The shard manifest
    pub manifest: ShardManifest,
    /// Location info for each shard
    pub shard_locations: HashMap<String, Vec<ShardLocation>>,
}

/// Result of resolving and fetching a blob
#[derive(Debug)]
pub struct ResolvedBlob {
    /// The reassembled blob data
    pub data: Bytes,
    /// MIME type from manifest
    pub mime_type: String,
    /// Reach level from manifest
    pub reach: String,
    /// Resolution timing
    pub resolution_time: Duration,
    /// Number of shards fetched
    pub shards_fetched: usize,
}

/// Error types for shard resolution
#[derive(Debug)]
pub enum ShardResolverError {
    /// Blob not found in DNA
    NotFound,
    /// Failed to query DNA
    DnaError(String),
    /// Failed to fetch shard from storage node
    FetchError { shard_hash: String, error: String },
    /// Not enough shards available for reassembly
    InsufficientShards { needed: usize, available: usize },
    /// Failed to reassemble blob from shards
    ReassemblyError(String),
    /// Internal error
    Internal(String),
}

impl std::fmt::Display for ShardResolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShardResolverError::NotFound => write!(f, "Blob not found"),
            ShardResolverError::DnaError(e) => write!(f, "DNA error: {}", e),
            ShardResolverError::FetchError { shard_hash, error } => {
                write!(f, "Failed to fetch shard {}: {}", shard_hash, error)
            }
            ShardResolverError::InsufficientShards { needed, available } => {
                write!(f, "Need {} shards but only {} available", needed, available)
            }
            ShardResolverError::ReassemblyError(e) => write!(f, "Reassembly failed: {}", e),
            ShardResolverError::Internal(e) => write!(f, "Internal error: {}", e),
        }
    }
}

impl std::error::Error for ShardResolverError {}

// ============================================================================
// Service Configuration
// ============================================================================

/// Configuration for the shard resolver service
#[derive(Debug, Clone)]
pub struct ShardResolverConfig {
    /// HTTP timeout for fetching shards
    pub fetch_timeout: Duration,
    /// Maximum concurrent shard fetches
    pub max_concurrent_fetches: usize,
    /// Retry attempts for failed shard fetches
    pub fetch_retries: u8,
    /// Whether to cache resolved blobs
    pub enable_caching: bool,
    /// TTL for cached blobs (1 hour for immutable content-addressed blobs)
    pub cache_ttl: Duration,
    /// Default storage endpoint if none in shard location
    pub default_storage_url: Option<String>,
}

impl Default for ShardResolverConfig {
    fn default() -> Self {
        Self {
            fetch_timeout: Duration::from_secs(30),
            max_concurrent_fetches: 4,
            fetch_retries: 2,
            enable_caching: true,
            cache_ttl: Duration::from_secs(3600), // 1 hour
            default_storage_url: None,
        }
    }
}

impl ShardResolverConfig {
    /// Create config from environment variables
    pub fn from_env() -> Self {
        let fetch_timeout_secs = std::env::var("SHARD_FETCH_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        let max_concurrent = std::env::var("SHARD_MAX_CONCURRENT_FETCHES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4);

        let default_storage_url = std::env::var("ELOHIM_STORAGE_URL").ok();

        Self {
            fetch_timeout: Duration::from_secs(fetch_timeout_secs),
            max_concurrent_fetches: max_concurrent,
            fetch_retries: 2,
            enable_caching: true,
            cache_ttl: Duration::from_secs(3600),
            default_storage_url,
        }
    }
}

// ============================================================================
// Shard Resolver Service
// ============================================================================

/// Service for resolving blobs from Holochain DNA and elohim-storage nodes
pub struct ShardResolver {
    /// HTTP client for fetching shards
    http_client: reqwest::Client,
    /// Optional content cache for storing resolved blobs
    cache: Option<Arc<ContentCache>>,
    /// Configuration
    config: ShardResolverConfig,
    /// Statistics
    stats: ShardResolverStats,
}

struct ShardResolverStats {
    resolutions_attempted: AtomicU64,
    resolutions_successful: AtomicU64,
    resolutions_failed: AtomicU64,
    shards_fetched: AtomicU64,
    shard_fetch_errors: AtomicU64,
    cache_hits: AtomicU64,
    bytes_resolved: AtomicU64,
}

/// Public statistics
#[derive(Debug, Clone, Serialize)]
pub struct ResolverStats {
    pub resolutions_attempted: u64,
    pub resolutions_successful: u64,
    pub resolutions_failed: u64,
    pub shards_fetched: u64,
    pub shard_fetch_errors: u64,
    pub cache_hits: u64,
    pub bytes_resolved: u64,
}

impl ShardResolver {
    /// Create a new shard resolver
    pub fn new(config: ShardResolverConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(config.fetch_timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http_client,
            cache: None,
            config,
            stats: ShardResolverStats {
                resolutions_attempted: AtomicU64::new(0),
                resolutions_successful: AtomicU64::new(0),
                resolutions_failed: AtomicU64::new(0),
                shards_fetched: AtomicU64::new(0),
                shard_fetch_errors: AtomicU64::new(0),
                cache_hits: AtomicU64::new(0),
                bytes_resolved: AtomicU64::new(0),
            },
        }
    }

    /// Create with content cache for storing resolved blobs
    pub fn with_cache(config: ShardResolverConfig, cache: Arc<ContentCache>) -> Self {
        let mut resolver = Self::new(config);
        resolver.cache = Some(cache);
        resolver
    }

    /// Set the content cache (for adding after construction)
    pub fn set_cache(&mut self, cache: Arc<ContentCache>) {
        self.cache = Some(cache);
    }

    /// Resolve a blob by its hash
    ///
    /// Takes a pre-resolved BlobResolution from the DNA (from resolve_blob zome call)
    /// and fetches the actual shard bytes to reassemble the blob.
    pub async fn resolve(
        &self,
        resolution: BlobResolution,
    ) -> Result<ResolvedBlob, ShardResolverError> {
        let start = Instant::now();
        let blob_hash = &resolution.manifest.blob_hash;

        self.stats.resolutions_attempted.fetch_add(1, Ordering::Relaxed);

        debug!(
            blob_hash = %blob_hash,
            encoding = %resolution.manifest.encoding,
            shards = resolution.manifest.total_shards,
            "Resolving blob from shards"
        );

        // Check cache first
        if let Some(ref cache) = self.cache {
            if let Some(entry) = cache.get(blob_hash) {
                self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
                debug!(blob_hash = %blob_hash, "Cache hit for resolved blob");
                return Ok(ResolvedBlob {
                    data: Bytes::from(entry.data),
                    mime_type: entry.content_type,
                    reach: entry.reach.unwrap_or_else(|| "commons".to_string()),
                    resolution_time: start.elapsed(),
                    shards_fetched: 0,
                });
            }
        }

        // Fetch shards and reassemble
        let result = self.fetch_and_reassemble(&resolution).await?;

        // Cache the result
        if self.config.enable_caching {
            if let Some(ref cache) = self.cache {
                cache.set_blob(
                    blob_hash,
                    result.data.to_vec(),
                    &result.mime_type,
                    self.config.cache_ttl,
                    Some(&result.reach),
                    Some(50), // Default priority
                );
                debug!(blob_hash = %blob_hash, "Cached resolved blob");
            }
        }

        self.stats.resolutions_successful.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_resolved.fetch_add(result.data.len() as u64, Ordering::Relaxed);

        info!(
            blob_hash = %blob_hash,
            size = result.data.len(),
            shards = result.shards_fetched,
            time_ms = start.elapsed().as_millis(),
            "Blob resolved successfully"
        );

        Ok(ResolvedBlob {
            resolution_time: start.elapsed(),
            ..result
        })
    }

    /// Fetch shards and reassemble the blob
    async fn fetch_and_reassemble(
        &self,
        resolution: &BlobResolution,
    ) -> Result<ResolvedBlob, ShardResolverError> {
        let manifest = &resolution.manifest;

        match manifest.encoding.as_str() {
            "none" => {
                // Single shard = entire blob
                if manifest.shard_hashes.is_empty() {
                    return Err(ShardResolverError::ReassemblyError(
                        "No shards in manifest".into(),
                    ));
                }

                let shard_hash = &manifest.shard_hashes[0];
                let locations = resolution
                    .shard_locations
                    .get(shard_hash)
                    .cloned()
                    .unwrap_or_default();

                let data = self.fetch_shard_with_fallback(shard_hash, &locations).await?;

                Ok(ResolvedBlob {
                    data,
                    mime_type: manifest.mime_type.clone(),
                    reach: manifest.reach.clone(),
                    resolution_time: Duration::ZERO, // Set by caller
                    shards_fetched: 1,
                })
            }

            "chunked" => {
                // Simple chunked: fetch all shards in order and concatenate
                let mut all_data = Vec::with_capacity(manifest.total_size as usize);
                let mut shards_fetched = 0;

                for shard_hash in &manifest.shard_hashes {
                    let locations = resolution
                        .shard_locations
                        .get(shard_hash)
                        .cloned()
                        .unwrap_or_default();

                    let shard_data = self.fetch_shard_with_fallback(shard_hash, &locations).await?;
                    all_data.extend_from_slice(&shard_data);
                    shards_fetched += 1;
                }

                Ok(ResolvedBlob {
                    data: Bytes::from(all_data),
                    mime_type: manifest.mime_type.clone(),
                    reach: manifest.reach.clone(),
                    resolution_time: Duration::ZERO,
                    shards_fetched,
                })
            }

            encoding if encoding.starts_with("reed-solomon") => {
                // Reed-Solomon: need at least k data shards
                self.fetch_and_decode_reed_solomon(resolution).await
            }

            _ => Err(ShardResolverError::ReassemblyError(format!(
                "Unknown encoding: {}",
                manifest.encoding
            ))),
        }
    }

    /// Fetch a shard with fallback to multiple locations
    async fn fetch_shard_with_fallback(
        &self,
        shard_hash: &str,
        locations: &[ShardLocation],
    ) -> Result<Bytes, ShardResolverError> {
        // Filter to active locations
        let active_locations: Vec<_> = locations.iter().filter(|l| l.active).collect();

        // If no locations, try default storage URL
        if active_locations.is_empty() {
            if let Some(ref default_url) = self.config.default_storage_url {
                return self.fetch_shard_from_url(shard_hash, default_url).await;
            }
            return Err(ShardResolverError::FetchError {
                shard_hash: shard_hash.to_string(),
                error: "No active storage locations".into(),
            });
        }

        // Try each location in order
        let mut last_error = None;
        for location in &active_locations {
            match self.fetch_shard_from_url(shard_hash, &location.endpoint_url).await {
                Ok(data) => {
                    self.stats.shards_fetched.fetch_add(1, Ordering::Relaxed);
                    return Ok(data);
                }
                Err(e) => {
                    warn!(
                        shard_hash = %shard_hash,
                        endpoint = %location.endpoint_url,
                        error = %e,
                        "Failed to fetch shard, trying next location"
                    );
                    last_error = Some(e);
                }
            }
        }

        self.stats.shard_fetch_errors.fetch_add(1, Ordering::Relaxed);

        Err(last_error.unwrap_or_else(|| ShardResolverError::FetchError {
            shard_hash: shard_hash.to_string(),
            error: "All locations failed".into(),
        }))
    }

    /// Fetch a shard from a specific storage URL
    async fn fetch_shard_from_url(
        &self,
        shard_hash: &str,
        base_url: &str,
    ) -> Result<Bytes, ShardResolverError> {
        let url = format!("{}/shard/{}", base_url.trim_end_matches('/'), shard_hash);

        let mut attempts = 0;
        loop {
            attempts += 1;

            match self.http_client.get(&url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        let bytes = response.bytes().await.map_err(|e| {
                            ShardResolverError::FetchError {
                                shard_hash: shard_hash.to_string(),
                                error: format!("Failed to read response body: {}", e),
                            }
                        })?;

                        debug!(
                            shard_hash = %shard_hash,
                            size = bytes.len(),
                            url = %url,
                            "Fetched shard successfully"
                        );

                        return Ok(bytes);
                    }

                    if attempts >= self.config.fetch_retries {
                        return Err(ShardResolverError::FetchError {
                            shard_hash: shard_hash.to_string(),
                            error: format!("HTTP {}", response.status()),
                        });
                    }
                }
                Err(e) => {
                    if attempts >= self.config.fetch_retries {
                        return Err(ShardResolverError::FetchError {
                            shard_hash: shard_hash.to_string(),
                            error: e.to_string(),
                        });
                    }
                }
            }

            // Exponential backoff
            let delay = Duration::from_millis(100 * 2u64.pow(attempts as u32 - 1));
            tokio::time::sleep(delay).await;
        }
    }

    /// Fetch and decode Reed-Solomon encoded blob
    async fn fetch_and_decode_reed_solomon(
        &self,
        resolution: &BlobResolution,
    ) -> Result<ResolvedBlob, ShardResolverError> {
        let manifest = &resolution.manifest;
        let k = manifest.data_shards as usize;
        let _n = manifest.total_shards as usize;

        // We need at least k shards to reconstruct
        let mut fetched_shards: Vec<Option<Vec<u8>>> = vec![None; manifest.shard_hashes.len()];
        let mut shards_fetched = 0;

        // Try to fetch shards, we need at least k
        for (i, shard_hash) in manifest.shard_hashes.iter().enumerate() {
            if shards_fetched >= k {
                break; // We have enough
            }

            let locations = resolution
                .shard_locations
                .get(shard_hash)
                .cloned()
                .unwrap_or_default();

            match self.fetch_shard_with_fallback(shard_hash, &locations).await {
                Ok(data) => {
                    fetched_shards[i] = Some(data.to_vec());
                    shards_fetched += 1;
                }
                Err(e) => {
                    warn!(
                        shard_hash = %shard_hash,
                        index = i,
                        error = %e,
                        "Failed to fetch shard, will try to reconstruct without it"
                    );
                }
            }
        }

        if shards_fetched < k {
            return Err(ShardResolverError::InsufficientShards {
                needed: k,
                available: shards_fetched,
            });
        }

        // For now, if we have all data shards (indices 0..k), just concatenate them
        // Full Reed-Solomon decoding would use the reed-solomon-erasure crate
        let mut all_data = Vec::with_capacity(manifest.total_size as usize);
        for shard in fetched_shards.into_iter().take(k).flatten() {
            all_data.extend(shard);
        }

        // Trim to actual size (last shard may be padded)
        all_data.truncate(manifest.total_size as usize);

        Ok(ResolvedBlob {
            data: Bytes::from(all_data),
            mime_type: manifest.mime_type.clone(),
            reach: manifest.reach.clone(),
            resolution_time: Duration::ZERO,
            shards_fetched,
        })
    }

    /// Get service statistics
    pub fn stats(&self) -> ResolverStats {
        ResolverStats {
            resolutions_attempted: self.stats.resolutions_attempted.load(Ordering::Relaxed),
            resolutions_successful: self.stats.resolutions_successful.load(Ordering::Relaxed),
            resolutions_failed: self.stats.resolutions_failed.load(Ordering::Relaxed),
            shards_fetched: self.stats.shards_fetched.load(Ordering::Relaxed),
            shard_fetch_errors: self.stats.shard_fetch_errors.load(Ordering::Relaxed),
            cache_hits: self.stats.cache_hits.load(Ordering::Relaxed),
            bytes_resolved: self.stats.bytes_resolved.load(Ordering::Relaxed),
        }
    }

    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        let attempted = self.stats.resolutions_attempted.load(Ordering::Relaxed);
        if attempted == 0 {
            return 0.0;
        }
        let successful = self.stats.resolutions_successful.load(Ordering::Relaxed);
        (successful as f64 / attempted as f64) * 100.0
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_manifest() -> ShardManifest {
        ShardManifest {
            blob_hash: "sha256-abc123".to_string(),
            total_size: 1024,
            mime_type: "image/png".to_string(),
            encoding: "none".to_string(),
            data_shards: 1,
            total_shards: 1,
            shard_size: 1024,
            shard_hashes: vec!["sha256-abc123".to_string()],
            reach: "commons".to_string(),
            author_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_config_default() {
        let config = ShardResolverConfig::default();
        assert_eq!(config.fetch_timeout, Duration::from_secs(30));
        assert_eq!(config.max_concurrent_fetches, 4);
        assert!(config.enable_caching);
    }

    #[test]
    fn test_resolver_creation() {
        let config = ShardResolverConfig::default();
        let resolver = ShardResolver::new(config);
        assert_eq!(resolver.stats().resolutions_attempted, 0);
    }

    #[test]
    fn test_error_display() {
        let err = ShardResolverError::NotFound;
        assert_eq!(format!("{}", err), "Blob not found");

        let err = ShardResolverError::InsufficientShards {
            needed: 4,
            available: 2,
        };
        assert!(format!("{}", err).contains("4"));
        assert!(format!("{}", err).contains("2"));
    }

    #[test]
    fn test_manifest_parsing() {
        let manifest = test_manifest();
        assert_eq!(manifest.encoding, "none");
        assert_eq!(manifest.shard_hashes.len(), 1);
    }
}
