//! Shard Resolver Service - reassembling a payload from its custody units
//!
//! Resolves blobs from a DNA `ShardManifest` (as projected into the doorway's
//! projection store) by fetching the manifest's custody units from elohim-storage
//! (`GET /shard/{cid}`) and reassembling them.
//!
//! ## The manifest (re-genesis 1 byte profile)
//!
//! Every address is a byte CID (CIDv1 raw + BLAKE3-256, `bafkr4i…`), admitted only by
//! full decode ([`elohim_address::ByteCid::parse`], here through serde). A manifest
//! declares its chunker, its content-defined `chunks`, its erasure code and — when the
//! code is not `none` — its `stripes`: contiguous runs of original bytes, each with
//! `data + parity` shards over `rs-gf8-0x11d-v1` (GF(2^8), polynomial 0x11d,
//! systematic Vandermonde as `reed-solomon-erasure` 6 computes it, index order, the
//! last data shard zero-padded and the stripe truncated to its declared length).
//!
//! - erasure `none`: the chunks are the custody units, fetched and concatenated in
//!   manifest order.
//! - erasure `rs-gf8-0x11d-v1`: each stripe needs `data` good shards; missing data
//!   shards are reconstructed from parity.
//!
//! ## Verify before serve
//!
//! Every unit is checked against its own CID before it is used (a unit that fails
//! its address counts as absent, never as data). The reassembled payload is then
//! hashed **whole** and must equal the manifest's `blob_cid` before it is cached or
//! returned: per-unit checks alone admit a manifest that lists the right units in
//! the wrong order. Unknown chunker or erasure ids fail closed.

use bytes::Bytes;
use elohim_address::{compute_bytes_cid, ByteCid};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

use crate::cache::ContentCache;

// ============================================================================
// Types (matching the DNA `ShardManifest`, elohim-storage `sharding.rs`)
// ============================================================================

/// The chunker id every epoch-0 manifest declares.
pub const CHUNKER_FASTCDC_2020: &str = "fastcdc-2020";
/// Erasure id: no erasure; the chunks are the custody units.
pub const ERASURE_NONE: &str = "none";
/// Erasure id: Reed-Solomon over GF(2^8), polynomial 0x11d, version 1.
pub const ERASURE_RS_GF8_0X11D_V1: &str = "rs-gf8-0x11d-v1";

/// The chunker a manifest declares (DNA `ChunkerDecl`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkerDecl {
    pub id: String,
    pub min: u32,
    pub avg: u32,
    pub max: u32,
    pub level: u8,
}

/// One content-defined chunk of the payload (DNA `ChunkRef`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkRef {
    pub cid: ByteCid,
    pub len: u32,
}

/// The erasure code a manifest declares (DNA `ErasureDecl`); `none` declares 0/0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErasureDecl {
    pub id: String,
    pub data: u8,
    pub parity: u8,
}

/// One erasure-coded shard of a stripe (DNA `ShardRef`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShardRef {
    pub cid: ByteCid,
    /// The shard's stored (padded) length.
    pub len: u32,
    /// Position in the stripe: `0..data` data, `data..data+parity` parity.
    pub index: u8,
}

/// One stripe: a contiguous run of original bytes and its `data + parity` shards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stripe {
    pub index: u32,
    /// Original bytes this stripe covers (before padding).
    pub byte_len: u64,
    pub shards: Vec<ShardRef>,
}

/// Shard manifest (mirrors the DNA `content_store_integrity::ShardManifest`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShardManifest {
    /// The byte CID of the complete payload.
    pub blob_cid: ByteCid,
    /// Total size of the payload in bytes.
    pub byte_len: u64,
    /// MIME type of the payload.
    pub mime_type: String,
    pub chunker: ChunkerDecl,
    pub chunks: Vec<ChunkRef>,
    pub erasure: ErasureDecl,
    /// Empty iff `erasure.id == "none"`.
    #[serde(default)]
    pub stripes: Vec<Stripe>,
    /// Reach level for access control.
    pub reach: String,
    /// Author agent public key.
    pub author_id: Option<String>,
    /// Creation timestamp (ISO 8601).
    pub created_at: String,
    #[serde(default)]
    pub verified_at: Option<String>,
}

/// The erasure a manifest declares, once checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclaredErasure {
    None,
    Rs { data: u8, parity: u8 },
}

impl ShardManifest {
    /// Check a manifest before acting on it: known chunker and erasure ids, chunks cover
    /// `byte_len`, stripes are contiguous, non-empty, cover `byte_len`, and carry exactly
    /// `data + parity` shards of `ceil(stripe_len / data)` bytes in index order.
    pub fn validate(&self) -> Result<DeclaredErasure, ShardResolverError> {
        let bad = |why: String| Err(ShardResolverError::InvalidManifest(why));
        if self.chunker.id != CHUNKER_FASTCDC_2020 {
            return bad(format!("unsupported chunker {}", self.chunker.id));
        }
        let chunk_total: u64 = self.chunks.iter().map(|c| u64::from(c.len)).sum();
        if chunk_total != self.byte_len {
            return bad(format!(
                "chunks cover {chunk_total} of {} bytes",
                self.byte_len
            ));
        }
        let erasure = match (
            self.erasure.id.as_str(),
            self.erasure.data,
            self.erasure.parity,
        ) {
            (ERASURE_NONE, 0, 0) => DeclaredErasure::None,
            (ERASURE_RS_GF8_0X11D_V1, data, parity) if data > 0 && parity > 0 => {
                DeclaredErasure::Rs { data, parity }
            }
            (id, data, parity) => return bad(format!("unsupported erasure {id}/{data}+{parity}")),
        };
        let DeclaredErasure::Rs { data, parity } = erasure else {
            if !self.stripes.is_empty() {
                return bad("stripes under erasure none".into());
            }
            return Ok(erasure);
        };
        let mut covered = 0u64;
        for (position, stripe) in self.stripes.iter().enumerate() {
            if stripe.index as usize != position {
                return bad(format!("stripe {} at position {position}", stripe.index));
            }
            if stripe.byte_len == 0 {
                return bad(format!("stripe {} covers no bytes", stripe.index));
            }
            if stripe.shards.len() != usize::from(data) + usize::from(parity) {
                return bad(format!(
                    "stripe {} has {} shards, the code needs {}",
                    stripe.index,
                    stripe.shards.len(),
                    u16::from(data) + u16::from(parity)
                ));
            }
            let shard_len = stripe.byte_len.div_ceil(u64::from(data));
            for (i, shard) in stripe.shards.iter().enumerate() {
                if usize::from(shard.index) != i || u64::from(shard.len) != shard_len {
                    return bad(format!(
                        "stripe {} shard {i}: index {} len {}, expected len {shard_len}",
                        stripe.index, shard.index, shard.len
                    ));
                }
            }
            covered += stripe.byte_len;
        }
        if covered != self.byte_len {
            return bad(format!(
                "stripes cover {covered} of {} bytes",
                self.byte_len
            ));
        }
        Ok(erasure)
    }

    /// The units custody holds: shard CIDs when erasure-coded, else chunk CIDs.
    pub fn custody_units(&self) -> Vec<ByteCid> {
        if self.stripes.is_empty() {
            self.chunks.iter().map(|c| c.cid).collect()
        } else {
            self.stripes
                .iter()
                .flat_map(|stripe| stripe.shards.iter().map(|s| s.cid))
                .collect()
        }
    }
}

/// Shard location from DNA (where to fetch a custody unit)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardLocation {
    /// Byte CID of the unit
    pub shard_cid: ByteCid,
    /// Agent public key of the holder
    pub holder_id: String,
    /// Base URL of the elohim-storage HTTP endpoint
    pub endpoint_url: String,
    /// When this location was registered
    pub registered_at: String,
    /// Whether this location is active
    pub active: bool,
}

/// Complete blob resolution input: the manifest and where its units live.
#[derive(Debug, Clone)]
pub struct BlobResolution {
    /// The shard manifest
    pub manifest: ShardManifest,
    /// Location info for each custody unit
    pub shard_locations: HashMap<ByteCid, Vec<ShardLocation>>,
}

/// Result of resolving and fetching a blob
#[derive(Debug)]
pub struct ResolvedBlob {
    /// The reassembled payload, verified whole against the manifest's `blob_cid`
    pub data: Bytes,
    /// MIME type from manifest
    pub mime_type: String,
    /// Reach level from manifest
    pub reach: String,
    /// Resolution timing
    pub resolution_time: Duration,
    /// Number of units fetched
    pub shards_fetched: usize,
}

/// Error types for shard resolution
#[derive(Debug)]
pub enum ShardResolverError {
    /// Blob not found in DNA
    NotFound,
    /// Failed to query DNA
    DnaError(String),
    /// Failed to fetch a unit from storage
    FetchError { shard_cid: String, error: String },
    /// Not enough good shards for a stripe
    InsufficientShards { needed: usize, available: usize },
    /// The manifest is not one this doorway reads
    InvalidManifest(String),
    /// Failed to reassemble blob from shards
    ReassemblyError(String),
    /// The reassembled payload does not hash to the manifest's `blob_cid`
    PayloadMismatch { expected: String, actual: String },
    /// Internal error
    Internal(String),
}

impl std::fmt::Display for ShardResolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShardResolverError::NotFound => write!(f, "Blob not found"),
            ShardResolverError::DnaError(e) => write!(f, "DNA error: {e}"),
            ShardResolverError::FetchError { shard_cid, error } => {
                write!(f, "Failed to fetch shard {shard_cid}: {error}")
            }
            ShardResolverError::InsufficientShards { needed, available } => {
                write!(f, "Need {needed} shards but only {available} available")
            }
            ShardResolverError::InvalidManifest(e) => write!(f, "Invalid manifest: {e}"),
            ShardResolverError::ReassemblyError(e) => write!(f, "Reassembly failed: {e}"),
            ShardResolverError::PayloadMismatch { expected, actual } => write!(
                f,
                "reassembled payload hashes to {actual}, manifest declares {expected}"
            ),
            ShardResolverError::Internal(e) => write!(f, "Internal error: {e}"),
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

    /// Resolve a blob from its manifest.
    ///
    /// Takes a pre-resolved [`BlobResolution`] (the manifest and its unit locations)
    /// and fetches the unit bytes to reassemble the payload. The payload is returned
    /// (and cached) only after it hashes whole to `manifest.blob_cid`.
    pub async fn resolve(
        &self,
        resolution: BlobResolution,
    ) -> Result<ResolvedBlob, ShardResolverError> {
        let start = Instant::now();
        let blob_cid = resolution.manifest.blob_cid;
        let key = blob_cid.to_string();

        self.stats
            .resolutions_attempted
            .fetch_add(1, Ordering::Relaxed);

        debug!(
            blob_cid = %blob_cid,
            erasure = %resolution.manifest.erasure.id,
            stripes = resolution.manifest.stripes.len(),
            chunks = resolution.manifest.chunks.len(),
            "Resolving blob from custody units"
        );

        // Check cache first
        if let Some(ref cache) = self.cache {
            if let Some(entry) = cache.get(&key) {
                self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
                debug!(blob_cid = %blob_cid, "Cache hit for resolved blob");
                return Ok(ResolvedBlob {
                    data: Bytes::from(entry.data),
                    mime_type: entry.content_type,
                    reach: entry.reach.unwrap_or_else(|| "commons".to_string()),
                    resolution_time: start.elapsed(),
                    shards_fetched: 0,
                });
            }
        }

        // Fetch units, reassemble and verify the whole payload
        let result = match self.fetch_and_reassemble(&resolution).await {
            Ok(result) => result,
            Err(error) => {
                self.stats
                    .resolutions_failed
                    .fetch_add(1, Ordering::Relaxed);
                return Err(error);
            }
        };

        // Cache the result
        if self.config.enable_caching {
            if let Some(ref cache) = self.cache {
                cache.set_blob(
                    &key,
                    result.data.to_vec(),
                    &result.mime_type,
                    self.config.cache_ttl,
                    Some(&result.reach),
                    Some(50), // Default priority
                );
                debug!(blob_cid = %blob_cid, "Cached resolved blob");
            }
        }

        self.stats
            .resolutions_successful
            .fetch_add(1, Ordering::Relaxed);
        self.stats
            .bytes_resolved
            .fetch_add(result.data.len() as u64, Ordering::Relaxed);

        info!(
            blob_cid = %blob_cid,
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

    /// Fetch the manifest's custody units, reassemble, and verify the whole payload.
    async fn fetch_and_reassemble(
        &self,
        resolution: &BlobResolution,
    ) -> Result<ResolvedBlob, ShardResolverError> {
        let manifest = &resolution.manifest;
        let erasure = manifest.validate()?;
        let (data, shards_fetched) = match erasure {
            DeclaredErasure::None => self.fetch_chunks(resolution).await?,
            DeclaredErasure::Rs { data, parity } => {
                self.fetch_and_decode_stripes(resolution, data, parity)
                    .await?
            }
        };
        verify_whole(manifest, &data)?;
        Ok(ResolvedBlob {
            data: Bytes::from(data),
            mime_type: manifest.mime_type.clone(),
            reach: manifest.reach.clone(),
            resolution_time: Duration::ZERO, // Set by caller
            shards_fetched,
        })
    }

    /// Under erasure `none`: fetch every chunk in manifest order and concatenate.
    async fn fetch_chunks(
        &self,
        resolution: &BlobResolution,
    ) -> Result<(Vec<u8>, usize), ShardResolverError> {
        let manifest = &resolution.manifest;
        let mut all_data = Vec::with_capacity(usize::try_from(manifest.byte_len).unwrap_or(0));
        for chunk in &manifest.chunks {
            let locations = resolution
                .shard_locations
                .get(&chunk.cid)
                .cloned()
                .unwrap_or_default();
            let bytes = self
                .fetch_shard_with_fallback(&chunk.cid, &locations)
                .await?;
            all_data.extend_from_slice(&bytes);
        }
        Ok((all_data, manifest.chunks.len()))
    }

    /// Under `rs-gf8-0x11d-v1`: per stripe, fetch until `data` units verify, reconstruct
    /// missing data shards from parity, truncate to the stripe's declared length.
    async fn fetch_and_decode_stripes(
        &self,
        resolution: &BlobResolution,
        data: u8,
        parity: u8,
    ) -> Result<(Vec<u8>, usize), ShardResolverError> {
        let manifest = &resolution.manifest;
        let k = usize::from(data);
        let mut all_data = Vec::with_capacity(usize::try_from(manifest.byte_len).unwrap_or(0));
        let mut shards_fetched = 0;
        for stripe in &manifest.stripes {
            let mut fetched: Vec<Option<Vec<u8>>> = vec![None; stripe.shards.len()];
            let mut have = 0;
            for (i, shard) in stripe.shards.iter().enumerate() {
                if have >= k {
                    break;
                }
                let locations = resolution
                    .shard_locations
                    .get(&shard.cid)
                    .cloned()
                    .unwrap_or_default();
                match self.fetch_shard_with_fallback(&shard.cid, &locations).await {
                    Ok(bytes) => {
                        fetched[i] = Some(bytes.to_vec());
                        have += 1;
                        shards_fetched += 1;
                    }
                    Err(e) => warn!(
                        shard_cid = %shard.cid,
                        stripe = stripe.index,
                        index = i,
                        error = %e,
                        "Shard unavailable, will reconstruct without it"
                    ),
                }
            }
            all_data.extend_from_slice(&decode_stripe(stripe, fetched, data, parity)?);
        }
        Ok((all_data, shards_fetched))
    }

    /// Fetch a shard with fallback to multiple locations
    async fn fetch_shard_with_fallback(
        &self,
        shard_cid: &ByteCid,
        locations: &[ShardLocation],
    ) -> Result<Bytes, ShardResolverError> {
        // Filter to active locations
        let active_locations: Vec<_> = locations.iter().filter(|l| l.active).collect();

        // If no locations, try default storage URL
        if active_locations.is_empty() {
            if let Some(ref default_url) = self.config.default_storage_url {
                return self.fetch_shard_from_url(shard_cid, default_url).await;
            }
            return Err(ShardResolverError::FetchError {
                shard_cid: shard_cid.to_string(),
                error: "No active storage locations".into(),
            });
        }

        // Try each location in order
        let mut last_error = None;
        for location in &active_locations {
            match self
                .fetch_shard_from_url(shard_cid, &location.endpoint_url)
                .await
            {
                Ok(data) => {
                    self.stats.shards_fetched.fetch_add(1, Ordering::Relaxed);
                    return Ok(data);
                }
                Err(e) => {
                    warn!(
                        shard_cid = %shard_cid,
                        endpoint = %location.endpoint_url,
                        error = %e,
                        "Failed to fetch shard, trying next location"
                    );
                    last_error = Some(e);
                }
            }
        }

        self.stats
            .shard_fetch_errors
            .fetch_add(1, Ordering::Relaxed);

        Err(
            last_error.unwrap_or_else(|| ShardResolverError::FetchError {
                shard_cid: shard_cid.to_string(),
                error: "All locations failed".into(),
            }),
        )
    }

    /// Fetch a shard from a specific storage URL
    async fn fetch_shard_from_url(
        &self,
        shard_cid: &ByteCid,
        base_url: &str,
    ) -> Result<Bytes, ShardResolverError> {
        let url = format!("{}/shard/{}", base_url.trim_end_matches('/'), shard_cid);

        let mut attempts = 0;
        loop {
            attempts += 1;

            match self.http_client.get(&url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        let bytes =
                            response
                                .bytes()
                                .await
                                .map_err(|e| ShardResolverError::FetchError {
                                    shard_cid: shard_cid.to_string(),
                                    error: format!("Failed to read response body: {e}"),
                                })?;

                        // Verify before use: a unit that fails its own address is
                        // treated as absent at this location, never as data.
                        if compute_bytes_cid(&bytes) != *shard_cid {
                            return Err(ShardResolverError::FetchError {
                                shard_cid: shard_cid.to_string(),
                                error: format!(
                                    "{url} served bytes that do not hash to the unit's cid"
                                ),
                            });
                        }

                        debug!(
                            shard_cid = %shard_cid,
                            size = bytes.len(),
                            url = %url,
                            "Fetched shard successfully"
                        );

                        return Ok(bytes);
                    }

                    if attempts >= self.config.fetch_retries {
                        return Err(ShardResolverError::FetchError {
                            shard_cid: shard_cid.to_string(),
                            error: format!("HTTP {}", response.status()),
                        });
                    }
                }
                Err(e) => {
                    if attempts >= self.config.fetch_retries {
                        return Err(ShardResolverError::FetchError {
                            shard_cid: shard_cid.to_string(),
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

/// Reconstruct one stripe from the shards supplied (`None` = missing; every `Some` has
/// already verified against its CID) and truncate it to its declared length.
fn decode_stripe(
    stripe: &Stripe,
    mut shards: Vec<Option<Vec<u8>>>,
    data: u8,
    parity: u8,
) -> Result<Vec<u8>, ShardResolverError> {
    let have = shards.iter().filter(|s| s.is_some()).count();
    if have < usize::from(data) {
        return Err(ShardResolverError::InsufficientShards {
            needed: usize::from(data),
            available: have,
        });
    }
    let rs =
        reed_solomon_erasure::galois_8::ReedSolomon::new(usize::from(data), usize::from(parity))
            .map_err(|e| ShardResolverError::ReassemblyError(e.to_string()))?;
    rs.reconstruct_data(&mut shards)
        .map_err(|e| ShardResolverError::ReassemblyError(e.to_string()))?;
    let stripe_len = usize::try_from(stripe.byte_len)
        .map_err(|_| ShardResolverError::ReassemblyError("stripe too large".into()))?;
    let mut out = Vec::with_capacity(stripe_len);
    for shard in shards.into_iter().take(usize::from(data)) {
        out.extend_from_slice(&shard.unwrap_or_default());
    }
    out.truncate(stripe_len);
    Ok(out)
}

/// The whole-artifact check: the reassembled payload must hash (BLAKE3, byte profile)
/// to the manifest's `blob_cid`. Per-unit checks cannot see a reordered unit list.
fn verify_whole(manifest: &ShardManifest, payload: &[u8]) -> Result<(), ShardResolverError> {
    let actual = compute_bytes_cid(payload);
    if actual == manifest.blob_cid {
        Ok(())
    } else {
        Err(ShardResolverError::PayloadMismatch {
            expected: manifest.blob_cid.to_string(),
            actual: actual.to_string(),
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Deterministic payload bytes (xorshift), so every CID below is minted from real bytes.
    fn payload(len: usize) -> Vec<u8> {
        let mut x: u64 = 0x454c_4f48_494d;
        (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                (x & 0xff) as u8
            })
            .collect()
    }

    fn chunker() -> ChunkerDecl {
        ChunkerDecl {
            id: CHUNKER_FASTCDC_2020.into(),
            min: 262_144,
            avg: 1_048_576,
            max: 4_194_304,
            level: 1,
        }
    }

    /// Cut `bytes` into `pieces` contiguous chunks addressed by their own byte CIDs.
    fn chunks_of(bytes: &[u8], pieces: usize) -> Vec<(ChunkRef, Vec<u8>)> {
        let size = bytes.len().div_ceil(pieces);
        bytes
            .chunks(size)
            .map(|c| {
                (
                    ChunkRef {
                        cid: compute_bytes_cid(c),
                        len: c.len() as u32,
                    },
                    c.to_vec(),
                )
            })
            .collect()
    }

    /// Erasure-code `bytes` the way elohim-storage's `sharding.rs` does
    /// (`rs-gf8-0x11d-v1`, stripes of at most `stripe_max`, padded data shards).
    fn stripes_of(
        bytes: &[u8],
        data: u8,
        parity: u8,
        stripe_max: usize,
    ) -> Vec<(Stripe, Vec<Vec<u8>>)> {
        let rs =
            reed_solomon_erasure::galois_8::ReedSolomon::new(data.into(), parity.into()).unwrap();
        bytes
            .chunks(stripe_max)
            .enumerate()
            .map(|(index, original)| {
                let shard_len = original.len().div_ceil(usize::from(data));
                let mut shards: Vec<Vec<u8>> = (0..usize::from(data))
                    .map(|i| {
                        let start = (i * shard_len).min(original.len());
                        let end = ((i + 1) * shard_len).min(original.len());
                        let mut shard = original[start..end].to_vec();
                        shard.resize(shard_len, 0);
                        shard
                    })
                    .collect();
                shards.extend((0..parity).map(|_| vec![0u8; shard_len]));
                rs.encode(&mut shards).unwrap();
                let stripe = Stripe {
                    index: index as u32,
                    byte_len: original.len() as u64,
                    shards: shards
                        .iter()
                        .enumerate()
                        .map(|(i, s)| ShardRef {
                            cid: compute_bytes_cid(s),
                            len: shard_len as u32,
                            index: i as u8,
                        })
                        .collect(),
                };
                (stripe, shards)
            })
            .collect()
    }

    fn manifest(
        bytes: &[u8],
        chunks: Vec<ChunkRef>,
        erasure: ErasureDecl,
        stripes: Vec<Stripe>,
    ) -> ShardManifest {
        ShardManifest {
            blob_cid: compute_bytes_cid(bytes),
            byte_len: bytes.len() as u64,
            mime_type: "application/octet-stream".into(),
            chunker: chunker(),
            chunks,
            erasure,
            stripes,
            reach: "commons".into(),
            author_id: None,
            created_at: "2026-11-05T00:00:00Z".into(),
            verified_at: None,
        }
    }

    async fn serve(server: &MockServer, cid: &ByteCid, body: Vec<u8>) {
        Mock::given(method("GET"))
            .and(path(format!("/shard/{cid}")))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
            .mount(server)
            .await;
    }

    fn resolver_for(server: &MockServer, cache: Arc<ContentCache>) -> ShardResolver {
        let config = ShardResolverConfig {
            fetch_timeout: Duration::from_secs(5),
            fetch_retries: 1,
            default_storage_url: Some(server.uri()),
            ..ShardResolverConfig::default()
        };
        ShardResolver::with_cache(config, cache)
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
        assert_eq!(format!("{err}"), "Blob not found");

        let err = ShardResolverError::InsufficientShards {
            needed: 4,
            available: 2,
        };
        assert!(format!("{err}").contains('4'));
        assert!(format!("{err}").contains('2'));
    }

    /// The manifest reads from the DNA's JSON shape; every address goes through the
    /// full-decode parser, so a legacy spelling fails to deserialize at all.
    #[test]
    fn manifest_parses_the_dna_shape_and_refuses_legacy_addresses() {
        let bytes = payload(1000);
        let chunks: Vec<ChunkRef> = chunks_of(&bytes, 2).into_iter().map(|(c, _)| c).collect();
        let m = manifest(
            &bytes,
            chunks,
            ErasureDecl {
                id: ERASURE_NONE.into(),
                data: 0,
                parity: 0,
            },
            vec![],
        );
        let json = serde_json::to_value(&m).unwrap();
        assert!(json["blob_cid"].as_str().unwrap().starts_with("bafkr4i"));
        let back: ShardManifest = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(back, m);
        assert_eq!(back.validate().unwrap(), DeclaredErasure::None);

        let mut legacy = json;
        legacy["blob_cid"] = serde_json::json!(format!("sha256-{}", "ab".repeat(32)));
        assert!(serde_json::from_value::<ShardManifest>(legacy).is_err());
    }

    #[test]
    fn manifest_with_unknown_chunker_or_erasure_fails_closed() {
        let bytes = payload(100);
        let chunks: Vec<ChunkRef> = chunks_of(&bytes, 1).into_iter().map(|(c, _)| c).collect();
        let mut m = manifest(
            &bytes,
            chunks,
            ErasureDecl {
                id: ERASURE_NONE.into(),
                data: 0,
                parity: 0,
            },
            vec![],
        );
        m.chunker.id = "fixed-1mib".into();
        assert!(matches!(
            m.validate(),
            Err(ShardResolverError::InvalidManifest(_))
        ));
        m.chunker = chunker();
        m.erasure.id = "reed-solomon-4-3".into();
        assert!(matches!(
            m.validate(),
            Err(ShardResolverError::InvalidManifest(_))
        ));
    }

    /// Three stripes (the last short), 4+3 shards each; in every stripe two data shards
    /// are missing (and a parity shard in stripes 1 and 2), and in stripe 0 one served
    /// shard carries the wrong bytes. The payload is rebuilt from parity, the bad shard is never used, and
    /// the result is checked whole against `blob_cid` before it is cached.
    #[tokio::test]
    async fn shard_fallback_reassembles_from_stripes_and_verifies_whole() {
        let bytes = payload(50_000);
        let encoded = stripes_of(&bytes, 4, 3, 20_000);
        assert_eq!(encoded.len(), 3);
        let server = MockServer::start().await;
        for (stripe, shards) in &encoded {
            for (i, shard) in shards.iter().enumerate() {
                let cid = stripe.shards[i].cid;
                match (stripe.index, i) {
                    // Stripe 0: shards 0 and 2 missing, shard 1 served with the wrong
                    // bytes — four good shards remain (3, 4, 5, 6).
                    (0, 0) | (0, 2) => {}
                    (0, 1) => serve(&server, &cid, b"not this shard".to_vec()).await,
                    // Stripes 1 and 2: two data shards and one parity shard missing.
                    (s, 0 | 2 | 4) if s > 0 => {}
                    _ => serve(&server, &cid, shard.clone()).await,
                }
            }
        }
        let chunks: Vec<ChunkRef> = chunks_of(&bytes, 3).into_iter().map(|(c, _)| c).collect();
        let m = manifest(
            &bytes,
            chunks,
            ErasureDecl {
                id: ERASURE_RS_GF8_0X11D_V1.into(),
                data: 4,
                parity: 3,
            },
            encoded.iter().map(|(s, _)| s.clone()).collect(),
        );
        let cache = Arc::new(ContentCache::with_defaults());
        let resolver = resolver_for(&server, Arc::clone(&cache));

        let resolved = resolver
            .resolve(BlobResolution {
                manifest: m.clone(),
                shard_locations: HashMap::new(),
            })
            .await
            .expect("reassembles from parity");

        assert_eq!(resolved.data.as_ref(), bytes.as_slice());
        assert_eq!(compute_bytes_cid(&resolved.data), m.blob_cid);
        assert_eq!(cache.blob_size(&m.blob_cid.to_string()), Some(bytes.len()));
    }

    /// A stripe with fewer than `data` good shards cannot be rebuilt; nothing is cached.
    #[tokio::test]
    async fn too_few_good_shards_is_refused() {
        let bytes = payload(8_000);
        let encoded = stripes_of(&bytes, 4, 3, 20_000);
        let server = MockServer::start().await;
        let (stripe, shards) = &encoded[0];
        for i in 0..3 {
            serve(&server, &stripe.shards[i].cid, shards[i].clone()).await;
        }
        let chunks: Vec<ChunkRef> = chunks_of(&bytes, 1).into_iter().map(|(c, _)| c).collect();
        let m = manifest(
            &bytes,
            chunks,
            ErasureDecl {
                id: ERASURE_RS_GF8_0X11D_V1.into(),
                data: 4,
                parity: 3,
            },
            vec![stripe.clone()],
        );
        let cache = Arc::new(ContentCache::with_defaults());
        let err = resolver_for(&server, Arc::clone(&cache))
            .resolve(BlobResolution {
                manifest: m.clone(),
                shard_locations: HashMap::new(),
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ShardResolverError::InsufficientShards {
                needed: 4,
                available: 3
            }
        ));
        assert_eq!(cache.blob_size(&m.blob_cid.to_string()), None);
    }

    /// Every chunk verifies against its own CID, but the manifest lists them in the wrong
    /// order: only the whole-artifact check sees it. Refused, never cached.
    #[tokio::test]
    async fn reordered_chunks_are_refused() {
        let bytes = payload(30_000);
        let pieces = chunks_of(&bytes, 3);
        let server = MockServer::start().await;
        for (chunk, body) in &pieces {
            serve(&server, &chunk.cid, body.clone()).await;
        }
        let mut chunks: Vec<ChunkRef> = pieces.iter().map(|(c, _)| *c).collect();
        chunks.swap(0, 2);
        let m = manifest(
            &bytes,
            chunks,
            ErasureDecl {
                id: ERASURE_NONE.into(),
                data: 0,
                parity: 0,
            },
            vec![],
        );
        let cache = Arc::new(ContentCache::with_defaults());
        let resolver = resolver_for(&server, Arc::clone(&cache));

        let err = resolver
            .resolve(BlobResolution {
                manifest: m.clone(),
                shard_locations: HashMap::new(),
            })
            .await
            .unwrap_err();

        assert!(
            matches!(err, ShardResolverError::PayloadMismatch { .. }),
            "{err}"
        );
        assert_eq!(cache.blob_size(&m.blob_cid.to_string()), None);
        assert_eq!(resolver.stats().resolutions_failed, 1);
    }

    /// In manifest order the same chunks reassemble and verify.
    #[tokio::test]
    async fn chunks_in_manifest_order_reassemble() {
        let bytes = payload(30_000);
        let pieces = chunks_of(&bytes, 3);
        let server = MockServer::start().await;
        for (chunk, body) in &pieces {
            serve(&server, &chunk.cid, body.clone()).await;
        }
        let chunks: Vec<ChunkRef> = pieces.iter().map(|(c, _)| *c).collect();
        let m = manifest(
            &bytes,
            chunks,
            ErasureDecl {
                id: ERASURE_NONE.into(),
                data: 0,
                parity: 0,
            },
            vec![],
        );
        let resolved = resolver_for(&server, Arc::new(ContentCache::with_defaults()))
            .resolve(BlobResolution {
                manifest: m,
                shard_locations: HashMap::new(),
            })
            .await
            .expect("chunks in order");
        assert_eq!(resolved.data.as_ref(), bytes.as_slice());
    }
}
