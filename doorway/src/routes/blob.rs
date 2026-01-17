//! Content store streaming routes with HTTP 206 Range request support
//!
//! Provides efficient media delivery without blocking conductor threads:
//! - `GET /store/{address}` - Stream entire content or byte range
//! - Supports `Range: bytes=start-end` header for partial content
//! - Returns `206 Partial Content` for range requests
//! - Returns `200 OK` for full content requests
//!
//! ## Content Addressing
//!
//! Accepts multiple address formats for backward compatibility:
//! - CID (Content Identifier): `bafkreihdwdcefgh...` (IPFS-compatible, preferred)
//! - SHA256 prefixed: `sha256-a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a`
//! - Raw SHA256 hex: `a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a`
//!
//! All formats are normalized internally to SHA256 hex for cache lookups.
//!
//! ## Shard Resolution Fallback
//!
//! When content is not in the local cache, the handler can optionally use
//! a ShardResolver to fetch from elohim-storage nodes:
//! 1. Query projection store for ShardManifest by blob_hash
//! 2. Fetch shards from elohim-storage endpoints
//! 3. Reassemble and cache for future requests
//!
//! ## Example Usage
//!
//! ```bash
//! # CID format (preferred)
//! curl https://doorway.example.com/store/bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku
//!
//! # Legacy SHA256 format
//! curl https://doorway.example.com/store/sha256-abc123
//!
//! # Partial content (video seeking)
//! curl -H "Range: bytes=1000000-2000000" https://doorway.example.com/store/bafkreihdwdcefgh...
//! ```

use crate::cache::ContentCache;
use crate::projection::ProjectionStore;
use crate::services::{BlobResolution, ShardLocation, ShardManifest, ShardResolver};
use bytes::Bytes;
use cid::Cid;
use http_body_util::Full;
use hyper::{header, Method, Request, Response, StatusCode};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Error type for blob operations
#[derive(Debug)]
pub enum BlobError {
    NotFound,
    InvalidRange,
    InvalidAddress(String),
    MethodNotAllowed,
    InternalError(String),
}

impl std::fmt::Display for BlobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlobError::NotFound => write!(f, "Blob not found"),
            BlobError::InvalidRange => write!(f, "Invalid range"),
            BlobError::InvalidAddress(addr) => write!(f, "Invalid content address: {}", addr),
            BlobError::MethodNotAllowed => write!(f, "Method not allowed"),
            BlobError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

/// Parse a content address (CID or SHA256 hash) and return normalized SHA256 hex.
///
/// Accepts:
/// - CID (e.g., "bafkreihdwdcefgh...") - extracts SHA256 from multihash
/// - SHA256 prefixed (e.g., "sha256-abc123...") - strips prefix
/// - Raw SHA256 hex (64 char hex string) - returns as-is
///
/// Returns SHA256 hex string for cache lookups.
fn parse_content_address(addr: &str) -> Result<String, BlobError> {
    // Try CID first (starts with common CID prefixes)
    if addr.starts_with("baf") || addr.starts_with("Qm") || addr.starts_with("z") {
        match Cid::from_str(addr) {
            Ok(cid) => {
                // Extract the raw hash bytes from the multihash
                let hash_bytes = cid.hash().digest();
                // Verify it's SHA256 (32 bytes)
                if hash_bytes.len() == 32 {
                    return Ok(format!("sha256-{}", hex::encode(hash_bytes)));
                }
                return Err(BlobError::InvalidAddress(format!(
                    "CID uses unsupported hash algorithm (expected SHA256, got {} bytes)",
                    hash_bytes.len()
                )));
            }
            Err(e) => {
                return Err(BlobError::InvalidAddress(format!(
                    "Invalid CID format: {}",
                    e
                )));
            }
        }
    }

    // Try sha256- prefix
    if let Some(hex_hash) = addr.strip_prefix("sha256-") {
        // Validate it's valid hex of correct length
        if hex_hash.len() == 64 && hex_hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(addr.to_string());
        }
        return Err(BlobError::InvalidAddress(format!(
            "Invalid sha256 hash: expected 64 hex chars, got {}",
            hex_hash.len()
        )));
    }

    // Try raw hex (64 chars)
    if addr.len() == 64 && addr.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(format!("sha256-{}", addr));
    }

    Err(BlobError::InvalidAddress(format!(
        "Unrecognized address format: {}",
        addr
    )))
}

/// Parse HTTP Range header.
/// Supports formats: `bytes=start-end`, `bytes=start-`, `bytes=-suffix`
///
/// Returns (start, end) where end is exclusive.
fn parse_range_header(range_header: &str, total_size: usize) -> Option<(usize, usize)> {
    // Expected format: "bytes=start-end" or "bytes=start-" or "bytes=-suffix"
    let range_str = range_header.strip_prefix("bytes=")?;

    if range_str.starts_with('-') {
        // Suffix range: bytes=-500 means last 500 bytes
        let suffix: usize = range_str[1..].parse().ok()?;
        let start = total_size.saturating_sub(suffix);
        return Some((start, total_size));
    }

    let parts: Vec<&str> = range_str.split('-').collect();
    if parts.len() != 2 {
        return None;
    }

    let start: usize = parts[0].parse().ok()?;

    let end = if parts[1].is_empty() {
        // Open-ended range: bytes=1000-
        total_size
    } else {
        // Closed range: bytes=1000-2000 (end is inclusive in HTTP, we make it exclusive)
        let end: usize = parts[1].parse().ok()?;
        end + 1 // Convert to exclusive end
    };

    // Validate range
    if start >= total_size || end > total_size || start >= end {
        return None;
    }

    Some((start, end))
}

/// Handle content store requests with Range support.
///
/// # Routes
/// - `GET /store/{address}` - Get content (full or partial)
/// - `HEAD /store/{address}` - Get content metadata only
///
/// Address can be CID (bafkrei...), sha256-prefixed, or raw hex.
///
/// # Headers
/// - `Range: bytes=start-end` - Request partial content
/// - `If-None-Match: "etag"` - Conditional request
///
/// # Responses
/// - `200 OK` - Full content
/// - `206 Partial Content` - Range request fulfilled
/// - `304 Not Modified` - ETag matched
/// - `400 Bad Request` - Invalid address format
/// - `404 Not Found` - Content not in cache
/// - `416 Range Not Satisfiable` - Invalid range
pub async fn handle_blob_request(
    req: Request<hyper::body::Incoming>,
    cache: Arc<ContentCache>,
) -> Result<Response<Full<Bytes>>, BlobError> {
    // Extract address from path: /store/{address}
    let path = req.uri().path();
    let raw_address = path
        .strip_prefix("/store/")
        .ok_or(BlobError::NotFound)?;

    if raw_address.is_empty() {
        return Err(BlobError::NotFound);
    }

    // Normalize address to SHA256 format for cache lookup
    let hash = parse_content_address(raw_address)?;

    debug!(raw_address = %raw_address, hash = %hash, method = %req.method(), "Blob request");

    match *req.method() {
        Method::GET => handle_get_blob(req, cache, &hash).await,
        Method::HEAD => handle_head_blob(req, cache, &hash).await,
        _ => Err(BlobError::MethodNotAllowed),
    }
}

/// Handle GET /store/{hash}
async fn handle_get_blob(
    req: Request<hyper::body::Incoming>,
    cache: Arc<ContentCache>,
    hash: &str,
) -> Result<Response<Full<Bytes>>, BlobError> {
    // Check if blob exists
    let total_size = cache.blob_size(hash).ok_or(BlobError::NotFound)?;

    // Check If-None-Match header for conditional request
    if let Some(if_none_match) = req.headers().get(header::IF_NONE_MATCH) {
        if let Ok(etag_str) = if_none_match.to_str() {
            if let Some(true) = cache.check_etag(hash, etag_str) {
                debug!(hash = %hash, "ETag match, returning 304");
                return Ok(Response::builder()
                    .status(StatusCode::NOT_MODIFIED)
                    .body(Full::new(Bytes::new()))
                    .unwrap());
            }
        }
    }

    // Check for Range header
    if let Some(range_header) = req.headers().get(header::RANGE) {
        if let Ok(range_str) = range_header.to_str() {
            return handle_range_request(cache, hash, range_str, total_size).await;
        }
    }

    // Full content request
    handle_full_content(cache, hash).await
}

/// Handle full content request (200 OK)
async fn handle_full_content(
    cache: Arc<ContentCache>,
    hash: &str,
) -> Result<Response<Full<Bytes>>, BlobError> {
    let entry = cache.get(hash).ok_or(BlobError::NotFound)?;

    info!(
        hash = %hash,
        size = entry.data.len(),
        content_type = %entry.content_type,
        "Serving full blob"
    );

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, &entry.content_type)
        .header(header::CONTENT_LENGTH, entry.data.len())
        .header(header::ETAG, &entry.etag)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        // Required for COEP: require-corp in Angular app
        .header("Cross-Origin-Resource-Policy", "cross-origin")
        .body(Full::new(Bytes::from(entry.data)))
        .unwrap())
}

/// Handle range request (206 Partial Content)
async fn handle_range_request(
    cache: Arc<ContentCache>,
    hash: &str,
    range_str: &str,
    total_size: usize,
) -> Result<Response<Full<Bytes>>, BlobError> {
    let (start, end) = parse_range_header(range_str, total_size).ok_or_else(|| {
        warn!(hash = %hash, range = %range_str, "Invalid range header");
        BlobError::InvalidRange
    })?;

    let (data, total, etag) = cache
        .get_range(hash, start..end)
        .ok_or(BlobError::NotFound)?;

    let content_range = format!("bytes {}-{}/{}", start, end - 1, total);
    let content_length = data.len();

    info!(
        hash = %hash,
        range = format!("{}-{}", start, end - 1),
        size = content_length,
        "Serving partial content"
    );

    Ok(Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, content_length)
        .header(header::CONTENT_RANGE, content_range)
        .header(header::ETAG, &etag)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        // Required for COEP: require-corp in Angular app
        .header("Cross-Origin-Resource-Policy", "cross-origin")
        .body(Full::new(data))
        .unwrap())
}

/// Handle HEAD /store/{hash}
async fn handle_head_blob(
    _req: Request<hyper::body::Incoming>,
    cache: Arc<ContentCache>,
    hash: &str,
) -> Result<Response<Full<Bytes>>, BlobError> {
    let entry = cache.get(hash).ok_or(BlobError::NotFound)?;

    debug!(
        hash = %hash,
        size = entry.data.len(),
        "HEAD request"
    );

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, &entry.content_type)
        .header(header::CONTENT_LENGTH, entry.data.len())
        .header(header::ETAG, &entry.etag)
        .header(header::ACCEPT_RANGES, "bytes")
        // Required for COEP: require-corp in Angular app
        .header("Cross-Origin-Resource-Policy", "cross-origin")
        .body(Full::new(Bytes::new()))
        .unwrap())
}

/// Convert BlobError to HTTP response
pub fn error_response(err: BlobError) -> Response<Full<Bytes>> {
    let (status, message) = match err {
        BlobError::NotFound => (StatusCode::NOT_FOUND, "Blob not found"),
        BlobError::InvalidRange => (StatusCode::RANGE_NOT_SATISFIABLE, "Invalid range"),
        BlobError::InvalidAddress(_) => (StatusCode::BAD_REQUEST, "Invalid content address"),
        BlobError::MethodNotAllowed => (StatusCode::METHOD_NOT_ALLOWED, "Method not allowed"),
        BlobError::InternalError(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
        }
    };

    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Full::new(Bytes::from(message)))
        .unwrap()
}

// ============================================================================
// Storage Proxy Fallback
// ============================================================================

/// Handle blob request with elohim-storage proxy fallback.
///
/// On cache miss, proxies to `{storage_url}/store/{hash}` and caches the result.
/// This is the primary fallback mechanism for seeded blobs.
pub async fn handle_blob_request_with_storage_proxy(
    req: Request<hyper::body::Incoming>,
    cache: Arc<ContentCache>,
    storage_url: Option<String>,
) -> Result<Response<Full<Bytes>>, BlobError> {
    // Extract address from path: /store/{address}
    let path = req.uri().path();
    let raw_address = path
        .strip_prefix("/store/")
        .ok_or(BlobError::NotFound)?;

    if raw_address.is_empty() {
        return Err(BlobError::NotFound);
    }

    // Normalize address to SHA256 format for cache lookup
    let hash = parse_content_address(raw_address)?;

    debug!(raw_address = %raw_address, hash = %hash, method = %req.method(), "Blob request with storage proxy");

    // Check cache first (hot path)
    if let Some(_size) = cache.blob_size(&hash) {
        return match *req.method() {
            Method::GET => handle_get_blob(req, cache, &hash).await,
            Method::HEAD => handle_head_blob(req, cache, &hash).await,
            _ => Err(BlobError::MethodNotAllowed),
        };
    }

    // Cache miss - try fetching from elohim-storage
    if let Some(ref storage) = storage_url {
        debug!(hash = %hash, storage = %storage, "Cache miss, fetching from elohim-storage");

        if let Ok((data, content_type)) = fetch_from_storage(storage, &hash).await {
            // Cache the result with 1 hour TTL
            let ttl = std::time::Duration::from_secs(3600);
            cache.set(&hash, data.to_vec(), &content_type, ttl);
            info!(hash = %hash, size = data.len(), "Fetched and cached from elohim-storage");

            // Now serve from cache
            return match *req.method() {
                Method::GET => handle_get_blob(req, cache, &hash).await,
                Method::HEAD => handle_head_blob(req, cache, &hash).await,
                _ => Err(BlobError::MethodNotAllowed),
            };
        } else {
            warn!(hash = %hash, "Failed to fetch from elohim-storage");
        }
    }

    // All fallbacks failed
    Err(BlobError::NotFound)
}

/// Fetch blob from elohim-storage.
///
/// Returns (data, content_type) on success.
async fn fetch_from_storage(storage_url: &str, hash: &str) -> Result<(Bytes, String), String> {
    let url = format!("{}/store/{}", storage_url.trim_end_matches('/'), hash);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Storage returned {}", response.status()));
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let data = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read body: {}", e))?;

    Ok((data, content_type))
}

// ============================================================================
// Shard Resolution Fallback
// ============================================================================

/// Context for blob resolution with shard fallback
pub struct BlobContext {
    /// Content cache for hot path
    pub cache: Arc<ContentCache>,
    /// Shard resolver for cache misses
    pub resolver: Option<Arc<ShardResolver>>,
    /// Projection store for manifest lookups
    pub projection: Option<Arc<ProjectionStore>>,
}

impl BlobContext {
    /// Create context with just cache (no shard fallback)
    pub fn cache_only(cache: Arc<ContentCache>) -> Self {
        Self {
            cache,
            resolver: None,
            projection: None,
        }
    }

    /// Create context with full shard resolution support
    pub fn with_resolver(
        cache: Arc<ContentCache>,
        resolver: Arc<ShardResolver>,
        projection: Arc<ProjectionStore>,
    ) -> Self {
        Self {
            cache,
            resolver: Some(resolver),
            projection: Some(projection),
        }
    }
}

/// Handle content store requests with shard resolution fallback.
///
/// This is the enhanced handler that tries shard resolution when
/// content is not in the local cache.
///
/// Address can be CID (bafkrei...), sha256-prefixed, or raw hex.
///
/// # Resolution Order
/// 1. Check local ContentCache (hot path)
/// 2. If miss and resolver available, try shard resolution:
///    a. Query projection store for ShardManifest
///    b. Get shard locations
///    c. Fetch shards from elohim-storage
///    d. Reassemble and cache
/// 3. Return 404 if all methods fail
pub async fn handle_blob_request_with_fallback(
    req: Request<hyper::body::Incoming>,
    ctx: Arc<BlobContext>,
) -> Result<Response<Full<Bytes>>, BlobError> {
    // Extract address from path: /store/{address}
    let path = req.uri().path();
    let raw_address = path
        .strip_prefix("/store/")
        .ok_or(BlobError::NotFound)?;

    if raw_address.is_empty() {
        return Err(BlobError::NotFound);
    }

    // Normalize address to SHA256 format for cache lookup
    let hash = parse_content_address(raw_address)?;

    debug!(raw_address = %raw_address, hash = %hash, method = %req.method(), "Blob request with fallback");

    match *req.method() {
        Method::GET => handle_get_blob_with_fallback(req, ctx, &hash).await,
        Method::HEAD => handle_head_blob_with_fallback(ctx, &hash).await,
        _ => Err(BlobError::MethodNotAllowed),
    }
}

/// Handle GET with shard resolution fallback
async fn handle_get_blob_with_fallback(
    req: Request<hyper::body::Incoming>,
    ctx: Arc<BlobContext>,
    hash: &str,
) -> Result<Response<Full<Bytes>>, BlobError> {
    // Check If-None-Match first (works even if we need to resolve)
    if let Some(if_none_match) = req.headers().get(header::IF_NONE_MATCH) {
        if let Ok(etag_str) = if_none_match.to_str() {
            if let Some(true) = ctx.cache.check_etag(hash, etag_str) {
                debug!(hash = %hash, "ETag match, returning 304");
                return Ok(Response::builder()
                    .status(StatusCode::NOT_MODIFIED)
                    .body(Full::new(Bytes::new()))
                    .unwrap());
            }
        }
    }

    // Try cache first (hot path)
    if let Some(size) = ctx.cache.blob_size(hash) {
        // Check for Range header
        if let Some(range_header) = req.headers().get(header::RANGE) {
            if let Ok(range_str) = range_header.to_str() {
                return handle_range_request(ctx.cache.clone(), hash, range_str, size).await;
            }
        }
        return handle_full_content(ctx.cache.clone(), hash).await;
    }

    // Cache miss - try shard resolution if available
    if let (Some(ref resolver), Some(ref projection)) = (&ctx.resolver, &ctx.projection) {
        debug!(hash = %hash, "Cache miss, trying shard resolution");

        // Try to resolve from shards
        match try_resolve_from_shards(hash, resolver, projection).await {
            Ok(()) => {
                // Successfully resolved and cached, now serve from cache
                if let Some(size) = ctx.cache.blob_size(hash) {
                    if let Some(range_header) = req.headers().get(header::RANGE) {
                        if let Ok(range_str) = range_header.to_str() {
                            return handle_range_request(ctx.cache.clone(), hash, range_str, size).await;
                        }
                    }
                    return handle_full_content(ctx.cache.clone(), hash).await;
                }
            }
            Err(e) => {
                warn!(hash = %hash, error = %e, "Shard resolution failed");
            }
        }
    }

    // All resolution methods failed
    Err(BlobError::NotFound)
}

/// Handle HEAD with shard resolution fallback
async fn handle_head_blob_with_fallback(
    ctx: Arc<BlobContext>,
    hash: &str,
) -> Result<Response<Full<Bytes>>, BlobError> {
    // Try cache first
    if let Some(entry) = ctx.cache.get(hash) {
        debug!(hash = %hash, size = entry.data.len(), "HEAD request (cached)");
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, &entry.content_type)
            .header(header::CONTENT_LENGTH, entry.data.len())
            .header(header::ETAG, &entry.etag)
            .header(header::ACCEPT_RANGES, "bytes")
            // Required for COEP: require-corp in Angular app
            .header("Cross-Origin-Resource-Policy", "cross-origin")
            .body(Full::new(Bytes::new()))
            .unwrap());
    }

    // For HEAD, we can query projection for manifest metadata without fetching shards
    if let Some(ref projection) = ctx.projection {
        if let Some(manifest) = get_manifest_from_projection(hash, projection).await {
            debug!(hash = %hash, size = manifest.total_size, "HEAD request (from manifest)");
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, &manifest.mime_type)
                .header(header::CONTENT_LENGTH, manifest.total_size as usize)
                .header(header::ACCEPT_RANGES, "bytes")
                // Required for COEP: require-corp in Angular app
                .header("Cross-Origin-Resource-Policy", "cross-origin")
                .body(Full::new(Bytes::new()))
                .unwrap());
        }
    }

    Err(BlobError::NotFound)
}

/// Try to resolve a blob from shards
async fn try_resolve_from_shards(
    blob_hash: &str,
    resolver: &Arc<ShardResolver>,
    projection: &Arc<ProjectionStore>,
) -> Result<(), String> {
    // Get manifest from projection store
    let manifest = get_manifest_from_projection(blob_hash, projection)
        .await
        .ok_or_else(|| "Manifest not found in projection".to_string())?;

    // Get shard locations from projection
    let shard_locations = get_shard_locations_from_projection(&manifest.shard_hashes, projection).await;

    // Build BlobResolution
    let resolution = BlobResolution {
        manifest,
        shard_locations,
    };

    // Resolve via shard resolver (fetches shards and caches result)
    resolver
        .resolve(resolution)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get ShardManifest from projection store
async fn get_manifest_from_projection(
    blob_hash: &str,
    projection: &Arc<ProjectionStore>,
) -> Option<ShardManifest> {
    // Query projection for ShardManifest by blob_hash
    let doc = projection.get("ShardManifest", blob_hash).await?;

    // Parse the projected document data into ShardManifest
    // doc.data is JsonValue, check if it's null
    if doc.data.is_null() {
        return None;
    }
    serde_json::from_value(doc.data).ok()
}

/// Get shard locations from projection store
async fn get_shard_locations_from_projection(
    shard_hashes: &[String],
    projection: &Arc<ProjectionStore>,
) -> HashMap<String, Vec<ShardLocation>> {
    let mut locations = HashMap::new();

    for shard_hash in shard_hashes {
        // Query for ShardLocation entries
        if let Some(doc) = projection.get("ShardLocation", shard_hash).await {
            // doc.data is JsonValue, check if it's null before parsing
            if !doc.data.is_null() {
                // The projection may store an array of locations
                if let Ok(locs) = serde_json::from_value::<Vec<ShardLocation>>(doc.data.clone()) {
                    locations.insert(shard_hash.clone(), locs);
                } else if let Ok(loc) = serde_json::from_value::<ShardLocation>(doc.data) {
                    locations.insert(shard_hash.clone(), vec![loc]);
                }
            }
        }
    }

    locations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_range_header() {
        let total = 1000;

        // Standard range
        assert_eq!(parse_range_header("bytes=0-499", total), Some((0, 500)));
        assert_eq!(parse_range_header("bytes=500-999", total), Some((500, 1000)));

        // Open-ended range
        assert_eq!(parse_range_header("bytes=500-", total), Some((500, 1000)));

        // Suffix range
        assert_eq!(parse_range_header("bytes=-200", total), Some((800, 1000)));

        // Invalid ranges
        assert_eq!(parse_range_header("bytes=1000-1500", total), None);
        assert_eq!(parse_range_header("bytes=500-499", total), None);
        assert_eq!(parse_range_header("invalid", total), None);
    }

    #[test]
    fn test_parse_range_edge_cases() {
        // First byte
        assert_eq!(parse_range_header("bytes=0-0", 100), Some((0, 1)));

        // Last byte
        assert_eq!(parse_range_header("bytes=99-99", 100), Some((99, 100)));

        // Full file
        assert_eq!(parse_range_header("bytes=0-99", 100), Some((0, 100)));

        // Suffix larger than file
        assert_eq!(parse_range_header("bytes=-200", 100), Some((0, 100)));
    }

    #[test]
    fn test_parse_content_address_sha256_prefixed() {
        let hash = "sha256-a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a";
        let result = parse_content_address(hash).unwrap();
        assert_eq!(result, hash);
    }

    #[test]
    fn test_parse_content_address_raw_hex() {
        let hex = "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a";
        let result = parse_content_address(hex).unwrap();
        assert_eq!(
            result,
            "sha256-a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"
        );
    }

    #[test]
    fn test_parse_content_address_cid() {
        // CIDv1 with raw codec and SHA256 for empty data
        // This is the CID for empty content: bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku
        // But for testing, let's use a simpler CID we can verify
        use cid::Cid;
        use multihash_codetable::{Code, MultihashDigest};

        // Create a CID for known data
        let data = b"Hello, Elohim!";
        let hash = Code::Sha2_256.digest(data);
        let cid = Cid::new_v1(0x55, hash); // 0x55 = raw codec
        let cid_str = cid.to_string();

        // Parse should extract SHA256 and prefix with sha256-
        let result = parse_content_address(&cid_str).unwrap();
        assert!(result.starts_with("sha256-"));

        // Verify the hash matches what we computed directly
        let expected_hash = hex::encode(hash.digest());
        assert_eq!(result, format!("sha256-{}", expected_hash));
    }

    #[test]
    fn test_parse_content_address_invalid() {
        // Too short
        assert!(parse_content_address("abc123").is_err());

        // Invalid characters
        assert!(parse_content_address("sha256-gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg").is_err());

        // Wrong length
        assert!(parse_content_address("sha256-abc123").is_err());
    }
}
