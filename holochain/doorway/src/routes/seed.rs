//! Admin Seed Routes - Blob upload for seeding operations
//!
//! Provides endpoints for uploading blobs during initial seeding:
//! - `PUT /admin/seed/blob` - Upload a blob, forwarded to elohim-storage
//!
//! ## Security
//!
//! These endpoints require admin authentication (API key or JWT).
//! In dev mode, authentication may be disabled.
//!
//! ## Flow
//!
//! ```text
//! Seeder → PUT /admin/seed/blob → Doorway
//!                                   ├── Local cache (fast, write-through)
//!                                   └── Forward to elohim-storage (authoritative)
//! ```
//!
//! Doorway acts as a write-cache (like SSD cache for HDD).
//! elohim-storage is the authoritative blob store.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::server::AppState;

// =============================================================================
// Types
// =============================================================================

/// Response from blob upload
#[derive(Debug, Serialize)]
pub struct BlobUploadResponse {
    pub success: bool,
    pub hash: String,
    /// Whether the blob was already present (in doorway cache)
    pub already_cached: bool,
    /// Whether blob was forwarded to elohim-storage
    pub forwarded_to_storage: bool,
    /// Size in bytes
    pub size: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response from elohim-storage
#[derive(Debug, Deserialize)]
struct StorageResponse {
    #[allow(dead_code)]
    hash: Option<String>,
    #[allow(dead_code)]
    size: Option<u64>,
    error: Option<String>,
}

// =============================================================================
// Route Handler
// =============================================================================

/// Handle PUT /admin/seed/blob
///
/// Expects:
/// - Body: Raw blob data
/// - Header `X-Blob-Hash`: Expected hash of the blob
/// - Header `Content-Type`: MIME type of the blob
/// - Header `X-Blob-Size`: Optional size hint
///
/// Flow:
/// 1. Verify hash matches body
/// 2. Cache locally (fast write-through cache)
/// 3. Forward to elohim-storage (authoritative store)
///
/// Returns:
/// - 200 OK with BlobUploadResponse on success
/// - 400 Bad Request if missing required headers
/// - 409 Conflict if hash mismatch
pub async fn handle_seed_blob(
    req: Request<Incoming>,
    state: Arc<AppState>,
) -> Response<Full<Bytes>> {
    // Extract required headers
    let expected_hash = match req.headers().get("X-Blob-Hash") {
        Some(h) => match h.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid X-Blob-Hash header"),
        },
        None => return error_response(StatusCode::BAD_REQUEST, "Missing X-Blob-Hash header"),
    };

    let content_type = req
        .headers()
        .get("Content-Type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    debug!(
        hash = %expected_hash,
        content_type = %content_type,
        "Processing seed blob upload"
    );

    // Check if already cached locally
    if let Some(cached_size) = state.cache.blob_size(&expected_hash) {
        info!(hash = %expected_hash, "Blob already in local cache");

        // Still try to forward to storage in case it's missing there
        let forwarded = forward_to_storage(&state, &expected_hash, None).await;

        return json_response(StatusCode::OK, &BlobUploadResponse {
            success: true,
            hash: expected_hash,
            already_cached: true,
            forwarded_to_storage: forwarded,
            size: cached_size,
            error: None,
        });
    }

    // Read request body
    let body = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            warn!(error = %e, "Failed to read blob body");
            return error_response(StatusCode::BAD_REQUEST, "Failed to read request body");
        }
    };

    let blob_size = body.len();
    debug!(hash = %expected_hash, size = blob_size, "Received blob data");

    // Verify hash matches
    let computed_hash = compute_sha256(&body);
    if computed_hash != expected_hash {
        warn!(
            expected = %expected_hash,
            computed = %computed_hash,
            "Blob hash mismatch"
        );
        let error_msg = format!("Hash mismatch: expected {}, got {}", expected_hash, computed_hash);
        return json_response(StatusCode::CONFLICT, &BlobUploadResponse {
            success: false,
            hash: expected_hash,
            already_cached: false,
            forwarded_to_storage: false,
            size: blob_size,
            error: Some(error_msg),
        });
    }

    // Store in local cache (1 hour TTL - acts as write-through cache)
    let seed_blob_ttl = std::time::Duration::from_secs(3600);
    state.cache.set(&expected_hash, body.to_vec(), &content_type, seed_blob_ttl);
    info!(hash = %expected_hash, size = blob_size, "Blob cached locally");

    // Forward to elohim-storage (authoritative store)
    let forwarded = forward_to_storage(&state, &expected_hash, Some(&body)).await;

    if forwarded {
        info!(hash = %expected_hash, "Blob forwarded to elohim-storage");
    } else {
        warn!(hash = %expected_hash, "Failed to forward blob to elohim-storage (will retry on read)");
    }

    json_response(StatusCode::OK, &BlobUploadResponse {
        success: true,
        hash: expected_hash,
        already_cached: false,
        forwarded_to_storage: forwarded,
        size: blob_size,
        error: None,
    })
}

/// Forward a blob to elohim-storage
///
/// If body is None, reads from local cache.
async fn forward_to_storage(state: &AppState, hash: &str, body: Option<&Bytes>) -> bool {
    let storage_url = match &state.args.storage_url {
        Some(url) => url.clone(),
        None => {
            debug!("No storage_url configured, skipping forward");
            return false;
        }
    };

    // Get body from cache if not provided
    let data = match body {
        Some(b) => b.to_vec(),
        None => match state.cache.get(hash) {
            Some(entry) => entry.data.clone(),
            None => {
                warn!(hash = %hash, "Blob not in cache, cannot forward");
                return false;
            }
        },
    };

    // Build URL: PUT /blob/{hash}
    let url = format!("{}/blob/{}", storage_url.trim_end_matches('/'), hash);

    debug!(url = %url, size = data.len(), "Forwarding blob to elohim-storage");

    // Use reqwest client from state or create one
    let client = reqwest::Client::new();

    match client
        .put(&url)
        .header("Content-Type", "application/octet-stream")
        .body(data)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                true
            } else {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                error!(
                    hash = %hash,
                    status = %status,
                    body = %body,
                    "elohim-storage returned error"
                );
                false
            }
        }
        Err(e) => {
            error!(hash = %hash, error = %e, "Failed to connect to elohim-storage");
            false
        }
    }
}

/// Check if a blob exists in the cache
pub async fn handle_check_blob(
    hash: &str,
    state: Arc<AppState>,
) -> Response<Full<Bytes>> {
    if state.cache.blob_size(hash).is_some() {
        Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::new()))
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::new()))
            .unwrap()
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Compute SHA256 hash of data (hex-encoded to match seeder convention)
fn compute_sha256(data: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    // Use hex encoding to match seeder (blob-manager.ts) convention
    format!("sha256-{:x}", result)
}

/// Create JSON response
fn json_response<T: Serialize>(status: StatusCode, data: &T) -> Response<Full<Bytes>> {
    let body = serde_json::to_string(data).unwrap_or_else(|_| r#"{"error":"Serialization failed"}"#.to_string());

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// Create error response
fn error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "success": false,
        "error": message,
    });

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_sha256() {
        let data = b"hello world";
        let hash = compute_sha256(data);
        // SHA256 of "hello world" is well-known
        assert!(hash.starts_with("sha256-"));
    }
}
