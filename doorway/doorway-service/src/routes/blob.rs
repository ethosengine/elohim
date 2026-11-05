//! Blob streaming routes with HTTP 206 Range request support
//!
//! Provides efficient media delivery without blocking conductor threads:
//! - `GET /blob/{address}` - Stream entire content or byte range
//! - Supports `Range: bytes=start-end` header for partial content
//! - Returns `206 Partial Content` for range requests
//! - Returns `200 OK` for full content requests
//!
//! Note: These handlers are not currently dispatched from `http.rs`. The
//! canonical `/blob/{hash}` path is registry-routed via storage's manifest.
//! These handlers exist as a custom-dispatch alternative if doorway-specific
//! caching/range logic ever needs to live above the registry layer.
//!
//! ## Content Addressing
//!
//! One spelling: a byte CID (CIDv1 raw `0x55` + BLAKE3-256 `0x1e`, `bafkr4i…`, 59
//! characters), admitted only by full decode ([`BlobAddress::parse`], which is
//! [`elohim_address::ByteCid::parse`]). There is no second spelling: `sha256-<hex>`,
//! `blake3-<hex>`, bare hex, `bafkrei…` (raw + sha2-256), CIDv0 and upper-case text
//! are all refused with the typed 400 elohim-storage answers,
//! `{"error":"invalid-byte-cid","check","detail"}`. The cache key is the canonical
//! CID text; every byte response names its address in `X-Content-Address`.
//!
//! ## Shard Resolution Fallback
//!
//! When content is not in the local cache, the handler can optionally use
//! a ShardResolver to fetch from elohim-storage nodes:
//! 1. Query projection store for the ShardManifest by `blob_cid`
//! 2. Fetch the manifest's custody units (chunks, or stripe shards) from storage
//! 3. Reassemble, verify the WHOLE payload against `blob_cid`, then cache
//!
//! ## Example Usage
//!
//! ```bash
//! curl https://doorway.example.com/blob/bafkr4igxjga67jykbseaxdmmdgc5a5o3zp3htom2l6mrjznk7fvyggu6eq
//!
//! # Partial content (video seeking)
//! curl -H "Range: bytes=1000000-2000000" https://doorway.example.com/blob/bafkr4i…
//! ```

use crate::cache::ContentCache;
use crate::projection::ProjectionStore;
use crate::services::{BlobResolution, ShardLocation, ShardManifest, ShardResolver};
use bytes::Bytes;
use elohim_address::{compute_bytes_cid, AddressError, ByteCid};
use http_body_util::Full;
use hyper::{header, Method, Request, Response, StatusCode};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// The header every byte response names its address in (the only content header).
pub const CONTENT_ADDRESS_HEADER: &str = "X-Content-Address";

/// Error type for blob operations
#[derive(Debug)]
pub enum BlobError {
    NotFound,
    InvalidRange,
    InvalidAddress(AddressError),
    MethodNotAllowed,
    InternalError(String),
}

impl std::fmt::Display for BlobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlobError::NotFound => write!(f, "Blob not found"),
            BlobError::InvalidRange => write!(f, "Invalid range"),
            BlobError::InvalidAddress(error) => write!(f, "Invalid byte CID: {error}"),
            BlobError::MethodNotAllowed => write!(f, "Method not allowed"),
            BlobError::InternalError(msg) => write!(f, "Internal error: {msg}"),
        }
    }
}

/// The doorway's admission of a byte address.
///
/// `BlobAddress::parse` IS [`ByteCid::parse`]: full decode (length 59, multibase `b`,
/// base32-lower alphabet, zero padding bits, 36 decoded bytes, header `01 55 1e 20`,
/// canonical re-encoding). The doorway writes no prefix or length check of its own,
/// so a 32-byte digest under another multihash is never admitted by its length.
pub struct BlobAddress;

impl BlobAddress {
    /// Admit `text` as a byte CID, or name the first check it fails.
    pub fn parse(text: &str) -> Result<ByteCid, AddressError> {
        ByteCid::parse(text)
    }
}

/// The typed 400 for a path segment that is not a byte CID — the same body
/// elohim-storage answers, so a refusal reads the same at either edge:
/// `{"error":"invalid-byte-cid","check":<failed check>,"detail":<reason>}`.
pub fn invalid_byte_cid_response(error: &AddressError) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "error": "invalid-byte-cid",
        "check": error.check(),
        "detail": error.to_string(),
    });
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

/// The address of a `/blob/{address}` path, admitted by full decode.
fn blob_path_address(path: &str) -> Result<ByteCid, BlobError> {
    let raw_address = path.strip_prefix("/blob/").ok_or(BlobError::NotFound)?;
    if raw_address.is_empty() {
        return Err(BlobError::NotFound);
    }
    BlobAddress::parse(raw_address).map_err(BlobError::InvalidAddress)
}

/// Parse HTTP Range header.
/// Supports formats: `bytes=start-end`, `bytes=start-`, `bytes=-suffix`
///
/// Returns (start, end) where end is exclusive.
fn parse_range_header(range_header: &str, total_size: usize) -> Option<(usize, usize)> {
    // Expected format: "bytes=start-end" or "bytes=start-" or "bytes=-suffix"
    let range_str = range_header.strip_prefix("bytes=")?;

    if let Some(suffix_str) = range_str.strip_prefix('-') {
        // Suffix range: bytes=-500 means last 500 bytes
        let suffix: usize = suffix_str.parse().ok()?;
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

/// Handle blob requests with Range support.
///
/// # Routes
/// - `GET /blob/{address}` - Get content (full or partial)
/// - `HEAD /blob/{address}` - Get content metadata only
///
/// The address must be a byte CID (`bafkr4i…`); anything else is a typed 400.
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
    let cid = blob_path_address(req.uri().path())?;

    debug!(cid = %cid, method = %req.method(), "Blob request");

    match *req.method() {
        Method::GET => handle_get_blob(req, cache, &cid).await,
        Method::HEAD => handle_head_blob(req, cache, &cid).await,
        _ => Err(BlobError::MethodNotAllowed),
    }
}

/// Handle GET /blob/{cid}
async fn handle_get_blob(
    req: Request<hyper::body::Incoming>,
    cache: Arc<ContentCache>,
    cid: &ByteCid,
) -> Result<Response<Full<Bytes>>, BlobError> {
    let key = cid.to_string();
    let hash = key.as_str();
    // Check if blob exists
    let total_size = cache.blob_size(hash).ok_or(BlobError::NotFound)?;

    // Check If-None-Match header for conditional request
    if let Some(if_none_match) = req.headers().get(header::IF_NONE_MATCH) {
        if let Ok(etag_str) = if_none_match.to_str() {
            if let Some(true) = cache.check_etag(hash, etag_str) {
                debug!(cid = %hash, "ETag match, returning 304");
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
            return handle_range_request(cache, cid, range_str, total_size).await;
        }
    }

    // Full content request
    handle_full_content(cache, cid).await
}

/// Handle full content request (200 OK)
async fn handle_full_content(
    cache: Arc<ContentCache>,
    cid: &ByteCid,
) -> Result<Response<Full<Bytes>>, BlobError> {
    let hash = cid.to_string();
    let entry = cache.get(&hash).ok_or(BlobError::NotFound)?;

    info!(
        cid = %hash,
        size = entry.data.len(),
        content_type = %entry.content_type,
        "Serving full blob"
    );

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, &entry.content_type)
        .header(header::CONTENT_LENGTH, entry.data.len())
        .header(header::ETAG, &entry.etag)
        .header(CONTENT_ADDRESS_HEADER, &hash)
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
    cid: &ByteCid,
    range_str: &str,
    total_size: usize,
) -> Result<Response<Full<Bytes>>, BlobError> {
    let key = cid.to_string();
    let hash = key.as_str();
    let (start, end) = parse_range_header(range_str, total_size).ok_or_else(|| {
        warn!(cid = %hash, range = %range_str, "Invalid range header");
        BlobError::InvalidRange
    })?;

    let (data, total, etag) = cache
        .get_range(hash, start..end)
        .ok_or(BlobError::NotFound)?;

    let content_range = format!("bytes {}-{}/{}", start, end - 1, total);
    let content_length = data.len();

    info!(
        cid = %hash,
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
        .header(CONTENT_ADDRESS_HEADER, hash)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        // Required for COEP: require-corp in Angular app
        .header("Cross-Origin-Resource-Policy", "cross-origin")
        .body(Full::new(data))
        .unwrap())
}

/// Handle HEAD /blob/{cid}
async fn handle_head_blob(
    _req: Request<hyper::body::Incoming>,
    cache: Arc<ContentCache>,
    cid: &ByteCid,
) -> Result<Response<Full<Bytes>>, BlobError> {
    let hash = cid.to_string();
    let entry = cache.get(&hash).ok_or(BlobError::NotFound)?;

    debug!(
        cid = %hash,
        size = entry.data.len(),
        "HEAD request"
    );

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, &entry.content_type)
        .header(header::CONTENT_LENGTH, entry.data.len())
        .header(header::ETAG, &entry.etag)
        .header(CONTENT_ADDRESS_HEADER, &hash)
        .header(header::ACCEPT_RANGES, "bytes")
        // Required for COEP: require-corp in Angular app
        .header("Cross-Origin-Resource-Policy", "cross-origin")
        .body(Full::new(Bytes::new()))
        .unwrap())
}

/// Convert BlobError to HTTP response
pub fn error_response(err: BlobError) -> Response<Full<Bytes>> {
    let (status, message) = match err {
        BlobError::InvalidAddress(error) => return invalid_byte_cid_response(&error),
        BlobError::NotFound => (StatusCode::NOT_FOUND, "Blob not found"),
        BlobError::InvalidRange => (StatusCode::RANGE_NOT_SATISFIABLE, "Invalid range"),
        BlobError::MethodNotAllowed => (StatusCode::METHOD_NOT_ALLOWED, "Method not allowed"),
        BlobError::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
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
/// On cache miss (and only for full-content, non-Range requests), proxies to
/// `{storage_url}/blob/{hash}`, writes the result into the local `ContentCache`,
/// and serves from cache.  This is the primary fallback mechanism for seeded
/// blobs and the reason `appFileCache.cachedFiles` should rise above zero after
/// the first request for a given blob.
///
/// ## Caching contract
///
/// | Condition | Cache written? |
/// |-----------|---------------|
/// | Request has `Range` header | No — we only cache full-content fetches |
/// | Storage returns non-2xx | No |
/// | Storage returns 206 Partial Content | No — we only have part of the blob |
/// | Blob bytes exceed `blob_max_bytes` | No (ContentCache.set is a no-op there) |
/// | Storage returns 200 OK, no Range header | Yes — `cache.set(hash, body, content_type, 1h)` |
///
/// Cache-write failures are non-fatal: the response is still returned.
///
/// ## Cache key and verification
///
/// Keys are the canonical byte-CID text (admitted by [`BlobAddress::parse`]).
/// Bytes fetched from storage are hashed and must equal the path CID before they
/// are cached or served (verify before serve); a mismatch is a 404, never bytes.
pub async fn handle_blob_request_with_storage_proxy(
    req: Request<hyper::body::Incoming>,
    cache: Arc<ContentCache>,
    storage_url: Option<String>,
) -> Result<Response<Full<Bytes>>, BlobError> {
    let cid = blob_path_address(req.uri().path())?;
    let hash = cid.to_string();

    // Detect Range header early — we skip the cache-write path for partial reads.
    let has_range_header = req.headers().contains_key(header::RANGE);

    debug!(
        cid = %hash,
        method = %req.method(),
        has_range = has_range_header,
        "Blob request with storage proxy",
    );

    // Check cache first (hot path) — serves both full and range requests from cache.
    if let Some(_size) = cache.blob_size(&hash) {
        return match *req.method() {
            Method::GET => handle_get_blob(req, cache, &cid).await,
            Method::HEAD => handle_head_blob(req, cache, &cid).await,
            _ => Err(BlobError::MethodNotAllowed),
        };
    }

    // Cache miss — fetch from elohim-storage.
    if let Some(ref storage) = storage_url {
        debug!(cid = %hash, storage = %storage, "Cache miss, fetching from elohim-storage");

        match fetch_from_storage(storage, &cid).await {
            Ok((data, content_type)) => {
                // Write to cache only for full-content responses from storage.
                // Skip if the client sent a Range header (partial-read intent)
                // or if content type signals a known-partial response format.
                if !has_range_header {
                    let ttl = std::time::Duration::from_secs(3600);
                    cache.set(&hash, data.to_vec(), &content_type, ttl);
                    debug!(cid = %hash, size = data.len(), "Blob written to cache");
                } else {
                    debug!(cid = %hash, "Skipping cache write for Range request");
                }

                info!(cid = %hash, size = data.len(), "Fetched from elohim-storage");

                // If the cache accepted the bytes (full request, blob within budget),
                // serve from cache — gives ETag, range support, immutable cache headers.
                // If the cache write didn't hold (oversized → evicted, or this is a
                // Range request that bypassed cache), serve the upstream bytes
                // directly. The bytes are real and authoritative; never drop a 200
                // on the floor just because the cache couldn't keep it.
                if !has_range_header && cache.blob_size(&hash).is_some() {
                    return match *req.method() {
                        Method::GET => handle_get_blob(req, cache, &cid).await,
                        Method::HEAD => handle_head_blob(req, cache, &cid).await,
                        _ => Err(BlobError::MethodNotAllowed),
                    };
                }

                // Direct response: Range request, OR full request whose cache write
                // didn't take effect (e.g., oversized blob exceeding blob_max_bytes).
                // For HEAD: send headers, empty body, but Content-Length reports
                // what GET would return. The client's HTTP stack handles slicing
                // for Range; we don't cache partial bodies.
                let is_head = *req.method() == Method::HEAD;
                let body = if is_head {
                    Full::new(Bytes::new())
                } else {
                    Full::new(data.clone())
                };
                return Ok(Response::builder()
                    .status(hyper::StatusCode::OK)
                    .header(header::CONTENT_TYPE, &content_type)
                    .header(header::CONTENT_LENGTH, data.len())
                    .header(CONTENT_ADDRESS_HEADER, &hash)
                    .header("Cross-Origin-Resource-Policy", "cross-origin")
                    .body(body)
                    .unwrap());
            }
            Err(e) => {
                warn!(cid = %hash, error = %e, "Failed to fetch from elohim-storage");
            }
        }
    }

    // All fallbacks failed
    Err(BlobError::NotFound)
}

/// Fetch blob from elohim-storage.
///
/// Makes a plain (non-Range) GET to `{storage_url}/blob/{cid}`.
/// Returns `(body_bytes, content_type)` on 200 OK whose body hashes to `cid`.
/// Returns an error for non-200 responses, including 206 Partial Content
/// (which indicates the storage peer returned a partial body — we refuse to
/// cache it to avoid serving truncated blobs), and for a body that is not the
/// bytes the address names.
async fn fetch_from_storage(storage_url: &str, cid: &ByteCid) -> Result<(Bytes, String), String> {
    let url = format!("{}/blob/{}", storage_url.trim_end_matches('/'), cid);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = response.status();

    // Accept only 200 OK — reject 206 Partial Content and all other status codes.
    // Caching a 206 would store an incomplete blob and corrupt subsequent full reads.
    if status != reqwest::StatusCode::OK {
        return Err(format!("Storage returned {status} (expected 200 OK)"));
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
        .map_err(|e| format!("Failed to read body: {e}"))?;

    let actual = compute_bytes_cid(&data);
    if actual != *cid {
        return Err(format!(
            "storage served bytes addressed {actual} for {cid}; refused (verify before serve)"
        ));
    }

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

/// Handle blob requests with shard resolution fallback.
///
/// This is the enhanced handler that tries shard resolution when
/// content is not in the local cache.
///
/// The address must be a byte CID (`bafkr4i…`).
///
/// # Resolution Order
/// 1. Check local ContentCache (hot path)
/// 2. If miss and resolver available, try shard resolution:
///    a. Query projection store for ShardManifest
///    b. Get custody-unit locations
///    c. Fetch units from elohim-storage (each verified against its CID)
///    d. Reassemble, verify the whole payload against the address, and cache
/// 3. Return 404 if all methods fail
pub async fn handle_blob_request_with_fallback(
    req: Request<hyper::body::Incoming>,
    ctx: Arc<BlobContext>,
) -> Result<Response<Full<Bytes>>, BlobError> {
    let cid = blob_path_address(req.uri().path())?;

    debug!(cid = %cid, method = %req.method(), "Blob request with fallback");

    match *req.method() {
        Method::GET => handle_get_blob_with_fallback(req, ctx, &cid).await,
        Method::HEAD => handle_head_blob_with_fallback(ctx, &cid).await,
        _ => Err(BlobError::MethodNotAllowed),
    }
}

/// Handle GET with shard resolution fallback
async fn handle_get_blob_with_fallback(
    req: Request<hyper::body::Incoming>,
    ctx: Arc<BlobContext>,
    cid: &ByteCid,
) -> Result<Response<Full<Bytes>>, BlobError> {
    let key = cid.to_string();
    let hash = key.as_str();
    // Check If-None-Match first (works even if we need to resolve)
    if let Some(if_none_match) = req.headers().get(header::IF_NONE_MATCH) {
        if let Ok(etag_str) = if_none_match.to_str() {
            if let Some(true) = ctx.cache.check_etag(hash, etag_str) {
                debug!(cid = %hash, "ETag match, returning 304");
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
                return handle_range_request(ctx.cache.clone(), cid, range_str, size).await;
            }
        }
        return handle_full_content(ctx.cache.clone(), cid).await;
    }

    // Cache miss - try shard resolution if available
    if let (Some(ref resolver), Some(ref projection)) = (&ctx.resolver, &ctx.projection) {
        debug!(cid = %hash, "Cache miss, trying shard resolution");

        // Try to resolve from shards
        match try_resolve_from_shards(cid, resolver, projection).await {
            Ok(()) => {
                // Successfully resolved and cached, now serve from cache
                if let Some(size) = ctx.cache.blob_size(hash) {
                    if let Some(range_header) = req.headers().get(header::RANGE) {
                        if let Ok(range_str) = range_header.to_str() {
                            return handle_range_request(ctx.cache.clone(), cid, range_str, size)
                                .await;
                        }
                    }
                    return handle_full_content(ctx.cache.clone(), cid).await;
                }
            }
            Err(e) => {
                warn!(cid = %hash, error = %e, "Shard resolution failed");
            }
        }
    }

    // All resolution methods failed
    Err(BlobError::NotFound)
}

/// Handle HEAD with shard resolution fallback
async fn handle_head_blob_with_fallback(
    ctx: Arc<BlobContext>,
    cid: &ByteCid,
) -> Result<Response<Full<Bytes>>, BlobError> {
    let key = cid.to_string();
    let hash = key.as_str();
    // Try cache first
    if let Some(entry) = ctx.cache.get(hash) {
        debug!(cid = %hash, size = entry.data.len(), "HEAD request (cached)");
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, &entry.content_type)
            .header(header::CONTENT_LENGTH, entry.data.len())
            .header(header::ETAG, &entry.etag)
            .header(CONTENT_ADDRESS_HEADER, hash)
            .header(header::ACCEPT_RANGES, "bytes")
            // Required for COEP: require-corp in Angular app
            .header("Cross-Origin-Resource-Policy", "cross-origin")
            .body(Full::new(Bytes::new()))
            .unwrap());
    }

    // For HEAD, we can query projection for manifest metadata without fetching shards
    if let Some(ref projection) = ctx.projection {
        if let Some(manifest) = get_manifest_from_projection(cid, projection).await {
            debug!(cid = %hash, size = manifest.byte_len, "HEAD request (from manifest)");
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, &manifest.mime_type)
                .header(header::CONTENT_LENGTH, manifest.byte_len)
                .header(CONTENT_ADDRESS_HEADER, hash)
                .header(header::ACCEPT_RANGES, "bytes")
                // Required for COEP: require-corp in Angular app
                .header("Cross-Origin-Resource-Policy", "cross-origin")
                .body(Full::new(Bytes::new()))
                .unwrap());
        }
    }

    Err(BlobError::NotFound)
}

/// Try to resolve a blob from its manifest's custody units.
///
/// The resolver checks every unit against its own CID and the reassembled payload
/// against `blob_cid` before caching; a manifest whose `blob_cid` is not the
/// requested address is refused here before any unit is fetched.
async fn try_resolve_from_shards(
    blob_cid: &ByteCid,
    resolver: &Arc<ShardResolver>,
    projection: &Arc<ProjectionStore>,
) -> Result<(), String> {
    // Get manifest from projection store
    let manifest = get_manifest_from_projection(blob_cid, projection)
        .await
        .ok_or_else(|| "Manifest not found in projection".to_string())?;
    if manifest.blob_cid != *blob_cid {
        return Err(format!(
            "projected manifest names {}, not {blob_cid}",
            manifest.blob_cid
        ));
    }

    // Get custody-unit locations from projection
    let shard_locations =
        get_shard_locations_from_projection(&manifest.custody_units(), projection).await;

    // Build BlobResolution
    let resolution = BlobResolution {
        manifest,
        shard_locations,
    };

    // Resolve via shard resolver (fetches units, verifies whole, caches result)
    resolver
        .resolve(resolution)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get ShardManifest from projection store. A projected document that does not
/// parse as the recast manifest (a legacy `total_size`/`shard_hashes` shape, or an
/// address that fails full decode) is absent, not misread.
async fn get_manifest_from_projection(
    blob_cid: &ByteCid,
    projection: &Arc<ProjectionStore>,
) -> Option<ShardManifest> {
    let doc = projection
        .get("ShardManifest", &blob_cid.to_string())
        .await?;

    // doc.data is JsonValue, check if it's null
    if doc.data.is_null() {
        return None;
    }
    serde_json::from_value(doc.data).ok()
}

/// Get custody-unit locations from projection store
async fn get_shard_locations_from_projection(
    units: &[ByteCid],
    projection: &Arc<ProjectionStore>,
) -> HashMap<ByteCid, Vec<ShardLocation>> {
    let mut locations = HashMap::new();

    for unit in units {
        // Query for ShardLocation entries
        if let Some(doc) = projection.get("ShardLocation", &unit.to_string()).await {
            // doc.data is JsonValue, check if it's null before parsing
            if !doc.data.is_null() {
                // The projection may store an array of locations
                if let Ok(locs) = serde_json::from_value::<Vec<ShardLocation>>(doc.data.clone()) {
                    locations.insert(*unit, locs);
                } else if let Ok(loc) = serde_json::from_value::<ShardLocation>(doc.data) {
                    locations.insert(*unit, vec![loc]);
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
        assert_eq!(
            parse_range_header("bytes=500-999", total),
            Some((500, 1000))
        );

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

    /// Every spelling a byte address has ever had, and the malformed ones, beside the
    /// one that is admitted. The kernel vector pins the admitted spelling.
    pub(super) fn legacy_and_malformed_spellings(bytes: &[u8]) -> Vec<(&'static str, String)> {
        let admitted = compute_bytes_cid(bytes).to_string();
        let sha256_hex = {
            use sha2::{Digest, Sha256};
            format!("{:x}", Sha256::digest(bytes))
        };
        // A header that declares a 33-byte digest over the same 32 digest bytes: 36
        // decoded bytes and a `bafkr4…`-looking text, refused by full decode only.
        let digest = *compute_bytes_cid(bytes).digest();
        let mut header_33 = vec![0x01, 0x55, 0x1e, 0x21];
        header_33.extend_from_slice(&digest);
        let header_33 = cid::multibase::encode(cid::multibase::Base::Base32Lower, header_33);
        vec![
            ("sha256 hex", format!("sha256-{sha256_hex}")),
            ("blake3-hex", format!("blake3-{}", hex::encode(digest))),
            ("bare-hex", sha256_hex),
            (
                "bafkrei (raw + sha2-256)",
                elohim_address::compute_raw_sha2_cid(bytes).to_string(),
            ),
            ("33-byte digest header", header_33),
            ("upper-case", admitted.to_uppercase()),
            (
                "upper-case body",
                format!("b{}", admitted[1..].to_uppercase()),
            ),
            ("truncated", admitted[..admitted.len() - 1].to_string()),
        ]
    }

    #[test]
    fn blob_address_admits_the_kernel_vector() {
        let cid = BlobAddress::parse("bafkr4igxjga67jykbseaxdmmdgc5a5o3zp3htom2l6mrjznk7fvyggu6eq")
            .expect("the kernel's \"hello world\" byte CID");
        assert_eq!(cid, compute_bytes_cid(b"hello world"));
    }

    /// No spelling but the byte CID is admitted, and the refusal names its check.
    #[test]
    fn blob_address_refuses_legacy_and_malformed_spellings() {
        for (label, text) in legacy_and_malformed_spellings(b"hello world") {
            let error = BlobAddress::parse(&text)
                .err()
                .unwrap_or_else(|| panic!("{label} ({text}) must be refused"));
            assert!(!error.check().is_empty(), "{label}");
        }
    }

    /// The 33-byte header carries a 32-byte digest and decodes to 36 bytes: only the
    /// header check refuses it — never a length or prefix heuristic.
    #[test]
    fn a_33_byte_digest_header_is_refused_by_its_header() {
        let (_, text) = legacy_and_malformed_spellings(b"hello world")
            .into_iter()
            .find(|(label, _)| *label == "33-byte digest header")
            .unwrap();
        assert_eq!(text.len(), 59);
        assert_eq!(
            BlobAddress::parse(&text).unwrap_err().check(),
            "digest-length"
        );
    }

    #[test]
    fn invalid_address_answers_the_typed_400() {
        let error = BlobAddress::parse("sha256-abc").unwrap_err();
        let response = error_response(BlobError::InvalidAddress(error));
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }
}

// =============================================================================
// Blob proxy cache tests
// =============================================================================
//
// These tests verify the cache-check / cache-write contract of
// `handle_blob_request_with_storage_proxy` using an in-process mock HTTP
// server built with hyper so no external process is required.
//
// Test map:
//   blob_proxy_200_writes_cache            — upstream 200 → cache.set called
//   blob_proxy_206_skips_cache             — upstream 206 → cache NOT written
//   blob_proxy_range_header_skips_cache    — Range header present → cache NOT written
//   blob_proxy_cache_write_failure_serves  — oversized blob evicted → response still served
//   blob_proxy_second_request_hits_cache   — two GETs, second hits cache (no upstream call)

#[cfg(test)]
mod proxy_cache_tests {
    use super::*;

    use crate::cache::ContentCache;
    use bytes::Bytes;
    use http_body_util::Full;
    use hyper::body::Incoming;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper::{Request, Response};
    use hyper_util::rt::TokioIo;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;

    const TEST_BODY: &[u8] = b"hello elohim blob data";

    /// The byte CID of [`TEST_BODY`], minted from its bytes.
    fn test_cid() -> String {
        compute_bytes_cid(TEST_BODY).to_string()
    }

    /// Spawn a minimal in-process mock HTTP server.
    ///
    /// Responds to every request with `status_code` and `body`.
    /// `call_count` is incremented atomically on each received request.
    /// The server exits after handling `max_requests` connections.
    ///
    /// Returns the bound `SocketAddr` and a shutdown sender (drop to stop early).
    async fn spawn_mock_storage(
        status_code: u16,
        body: &'static [u8],
        call_count: Arc<std::sync::atomic::AtomicUsize>,
        max_requests: usize,
    ) -> (SocketAddr, oneshot::Sender<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

        tokio::spawn(async move {
            let mut handled = 0;
            loop {
                if handled >= max_requests {
                    break;
                }
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    accept = listener.accept() => {
                        let Ok((stream, _)) = accept else { break };
                        let count_clone = Arc::clone(&call_count);
                        handled += 1;
                        tokio::spawn(async move {
                            let io = TokioIo::new(stream);
                            let _ = http1::Builder::new()
                                .serve_connection(
                                    io,
                                    service_fn(move |_req: Request<Incoming>| {
                                        count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                        let resp = Response::builder()
                                            .status(status_code)
                                            .header("Content-Type", "application/octet-stream")
                                            .body(Full::new(Bytes::from(body)))
                                            .unwrap();
                                        async move { Ok::<_, std::convert::Infallible>(resp) }
                                    }),
                                )
                                .await;
                        });
                    }
                }
            }
        });

        (addr, shutdown_tx)
    }

    /// Drive a real GET against `mock_addr` routed through
    /// `handle_blob_request_with_storage_proxy` connected to the mock server.
    ///
    /// Returns whether the response was 200 OK and the cache blob_size after
    /// the call.
    async fn drive_request(
        cache: Arc<ContentCache>,
        mock_addr: SocketAddr,
        hash: &str,
        range_header: Option<&str>,
    ) -> (u16, Option<usize>) {
        // Build a real HTTP/1.1 connection to the mock server so we get a
        // genuine Request<Incoming> to feed the handler.  We spin up a tiny
        // loopback proxy: the proxy receives the hyper Request<Incoming> and
        // passes it directly to the handler.

        // Loopback listener for the proxy side
        let proxy_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = proxy_listener.local_addr().unwrap();
        let storage_url = Some(format!("http://{mock_addr}"));
        let cache_for_proxy = Arc::clone(&cache);

        // Spawn the proxy server that calls our handler
        tokio::spawn(async move {
            if let Ok((stream, _)) = proxy_listener.accept().await {
                let io = TokioIo::new(stream);
                let _ = http1::Builder::new()
                    .serve_connection(
                        io,
                        service_fn(move |req: Request<Incoming>| {
                            let cache = Arc::clone(&cache_for_proxy);
                            let storage = storage_url.clone();
                            async move {
                                let result =
                                    handle_blob_request_with_storage_proxy(req, cache, storage)
                                        .await;
                                Ok::<_, std::convert::Infallible>(match result {
                                    Ok(r) => r,
                                    Err(e) => error_response(e),
                                })
                            }
                        }),
                    )
                    .await;
            }
        });

        // Make the client request to the proxy
        let mut request_builder =
            reqwest::Client::new().get(format!("http://{proxy_addr}/blob/{hash}"));

        if let Some(range) = range_header {
            request_builder = request_builder.header("Range", range);
        }

        let resp = request_builder
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .expect("request to proxy");

        let status = resp.status().as_u16();
        let size_after = cache.blob_size(hash);
        (status, size_after)
    }

    // --------------------------------------------------------------------------
    // Test 1: upstream 200 → cache.set called
    // --------------------------------------------------------------------------

    #[tokio::test]
    async fn blob_proxy_200_writes_cache() {
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (mock_addr, _shutdown) =
            spawn_mock_storage(200, TEST_BODY, Arc::clone(&call_count), 1).await;

        let cache = Arc::new(ContentCache::with_defaults());
        assert!(
            cache.blob_size(&test_cid()).is_none(),
            "Cache should be empty before the request"
        );

        let (status, size_after) = drive_request(
            Arc::clone(&cache),
            mock_addr,
            &test_cid(),
            None, // no Range header
        )
        .await;

        assert_eq!(status, 200, "Expected 200 from proxy");
        assert_eq!(
            call_count.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "Mock storage should have been called exactly once"
        );
        assert!(
            size_after.is_some(),
            "Cache must be populated after a successful 200 fetch; got {:?}",
            size_after
        );
        assert_eq!(
            size_after.unwrap(),
            TEST_BODY.len(),
            "Cached size must match upstream body length"
        );
    }

    // --------------------------------------------------------------------------
    // Test 2: upstream 206 → cache NOT written
    // --------------------------------------------------------------------------

    #[tokio::test]
    async fn blob_proxy_206_skips_cache() {
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (mock_addr, _shutdown) =
            spawn_mock_storage(206, TEST_BODY, Arc::clone(&call_count), 1).await;

        let cache = Arc::new(ContentCache::with_defaults());

        let (status, size_after) =
            drive_request(Arc::clone(&cache), mock_addr, &test_cid(), None).await;

        // Storage returned 206, which fetch_from_storage rejects as non-200.
        // The proxy returns 404 (not found / all fallbacks failed).
        assert_eq!(
            status, 404,
            "Proxy must return 404 when storage returns 206"
        );
        assert!(
            size_after.is_none(),
            "Cache must NOT be written when storage returns 206"
        );
    }

    // --------------------------------------------------------------------------
    // Test 3: Range header → cache NOT written
    // --------------------------------------------------------------------------

    #[tokio::test]
    async fn blob_proxy_range_header_skips_cache() {
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (mock_addr, _shutdown) =
            spawn_mock_storage(200, TEST_BODY, Arc::clone(&call_count), 1).await;

        let cache = Arc::new(ContentCache::with_defaults());

        let (_status, size_after) = drive_request(
            Arc::clone(&cache),
            mock_addr,
            &test_cid(),
            Some("bytes=0-9"), // Range header present
        )
        .await;

        assert!(
            size_after.is_none(),
            "Cache must NOT be written when client sends a Range header; got {:?}",
            size_after
        );
    }

    // --------------------------------------------------------------------------
    // Test 4: blob exceeds cache budget → cache write is a no-op, response served
    // --------------------------------------------------------------------------

    #[tokio::test]
    async fn blob_proxy_oversized_blob_serves_anyway() {
        // Create a tiny cache that will reject blobs above 1 byte max
        use crate::cache::CacheConfig;
        let tiny_config = CacheConfig {
            max_entries: 0, // evict everything immediately
            content_ttl: std::time::Duration::from_secs(60),
            list_ttl: std::time::Duration::from_secs(60),
            user_ttl: std::time::Duration::from_secs(60),
            cleanup_interval: std::time::Duration::from_secs(3600),
        };
        let cache = Arc::new(ContentCache::new(tiny_config));

        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (mock_addr, _shutdown) =
            spawn_mock_storage(200, TEST_BODY, Arc::clone(&call_count), 1).await;

        let (status, _size_after) =
            drive_request(Arc::clone(&cache), mock_addr, &test_cid(), None).await;

        // The response must still be 200 even if the cache entry was immediately evicted.
        assert_eq!(
            status, 200,
            "Response must be served even when cache write is evicted immediately"
        );
        assert_eq!(
            call_count.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "Storage must have been called exactly once"
        );
    }

    // --------------------------------------------------------------------------
    // Test 5: second request hits cache, storage NOT called a second time
    // --------------------------------------------------------------------------

    #[tokio::test]
    async fn blob_proxy_second_request_hits_cache() {
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        // Allow up to 2 connections, but we expect only 1 to actually be used.
        let (mock_addr, _shutdown) =
            spawn_mock_storage(200, TEST_BODY, Arc::clone(&call_count), 2).await;

        let cache = Arc::new(ContentCache::with_defaults());

        // First request — cache miss, fetches from storage
        let (status1, size_after_1) =
            drive_request(Arc::clone(&cache), mock_addr, &test_cid(), None).await;
        assert_eq!(status1, 200, "First request must succeed");
        assert!(
            size_after_1.is_some(),
            "Cache must be populated after first request"
        );

        let calls_after_first = call_count.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            calls_after_first, 1,
            "Storage called once after first request"
        );

        // Second request — cache hit, storage must NOT be called again.
        // The cache check is done inside the handler before fetch_from_storage,
        // so storage sees 0 new calls.
        let (status2, _) = drive_request(Arc::clone(&cache), mock_addr, &test_cid(), None).await;
        assert_eq!(
            status2, 200,
            "Second request must succeed (served from cache)"
        );

        let calls_after_second = call_count.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            calls_after_second, 1,
            "Storage must NOT be called for the second request (cache hit expected); \
             got {} calls (expected 1)",
            calls_after_second
        );
    }

    // --------------------------------------------------------------------------
    // Test 6: legacy and malformed spellings are refused before storage is asked
    // --------------------------------------------------------------------------

    /// The brief named this `blob_route_404s_legacy_spellings`; the storage contract it
    /// mirrors answers the typed 400 `invalid-byte-cid`, so the doorway does too.
    #[tokio::test]
    async fn blob_route_refuses_legacy_spellings_400() {
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (mock_addr, _shutdown) =
            spawn_mock_storage(200, TEST_BODY, Arc::clone(&call_count), 16).await;
        let cache = Arc::new(ContentCache::with_defaults());

        for (label, address) in super::tests::legacy_and_malformed_spellings(TEST_BODY) {
            let (status, body) = drive_request_body(Arc::clone(&cache), mock_addr, &address).await;
            assert_eq!(status, 400, "{label} ({address}) must be refused: {body}");
            let v: serde_json::Value = serde_json::from_str(&body)
                .unwrap_or_else(|e| panic!("{label}: typed JSON body ({e}): {body}"));
            assert_eq!(v["error"], "invalid-byte-cid", "{label}");
            assert!(v["check"].is_string(), "{label}: {body}");
        }
        assert_eq!(
            call_count.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "a refused address never reaches storage"
        );

        // The byte CID of the same bytes is served, under its address.
        let (status, _) = drive_request_body(Arc::clone(&cache), mock_addr, &test_cid()).await;
        assert_eq!(status, 200);
    }

    // --------------------------------------------------------------------------
    // Test 7: bytes that do not hash to the address are neither served nor cached
    // --------------------------------------------------------------------------

    #[tokio::test]
    async fn blob_proxy_refuses_bytes_that_miss_their_address() {
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (mock_addr, _shutdown) =
            spawn_mock_storage(200, b"some other bytes", Arc::clone(&call_count), 1).await;
        let cache = Arc::new(ContentCache::with_defaults());

        let (status, size_after) =
            drive_request(Arc::clone(&cache), mock_addr, &test_cid(), None).await;

        assert_eq!(status, 404, "bytes that miss their address are not served");
        assert!(size_after.is_none(), "and never cached");
    }

    /// Like [`drive_request`], returning the response body instead of the cache size.
    async fn drive_request_body(
        cache: Arc<ContentCache>,
        mock_addr: SocketAddr,
        address: &str,
    ) -> (u16, String) {
        let proxy_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = proxy_listener.local_addr().unwrap();
        let storage_url = Some(format!("http://{mock_addr}"));
        tokio::spawn(async move {
            if let Ok((stream, _)) = proxy_listener.accept().await {
                let io = TokioIo::new(stream);
                let _ = http1::Builder::new()
                    .serve_connection(
                        io,
                        service_fn(move |req: Request<Incoming>| {
                            let cache = Arc::clone(&cache);
                            let storage = storage_url.clone();
                            async move {
                                let result =
                                    handle_blob_request_with_storage_proxy(req, cache, storage)
                                        .await;
                                Ok::<_, std::convert::Infallible>(match result {
                                    Ok(r) => r,
                                    Err(e) => error_response(e),
                                })
                            }
                        }),
                    )
                    .await;
            }
        });
        let resp = reqwest::Client::new()
            .get(format!("http://{proxy_addr}/blob/{address}"))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .expect("request to proxy");
        let status = resp.status().as_u16();
        (status, resp.text().await.unwrap_or_default())
    }
}
