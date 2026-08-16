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

use crate::auth::{extract_http_permission, PermissionLevel};
use crate::server::AppState;

// =============================================================================
// Authorization gate — Stage A (CI seed authority)
// =============================================================================

/// Require seed/cache-mutation authority for an incoming request.
///
/// Stage A — resolves to the `seed-content` operator capability at Stage C
/// (`auth/operator.rs`); transition-window Admin check per `operator.rs`
/// sanction. Until the capability-scoped resolver is wired for the seed path,
/// this gate accepts any request that resolves to [`PermissionLevel::Admin`]
/// (legacy Admin JWT or admin API key) and, in dev mode, the local-stack
/// fallback (so `pnpm run hc:start:seed` keeps working with no credential).
///
/// Returns:
/// - `Ok(())` if authorized (dev_mode, or resolved level >= Admin).
/// - `Err(401)` if no/invalid bearer resolved to [`PermissionLevel::Public`].
/// - `Err(403)` if authenticated but below Admin.
///
/// Generic over the request body type so the gate runs on request **headers
/// only** — a rejected request never has its blob body consumed.
// Matches the established `require_admin`/`get_jwt_validator` pattern in
// admin_users.rs: rejection responses carry a full `Response` in the Err arm.
#[allow(clippy::result_large_err)]
pub(crate) fn require_seed_authority<B>(
    state: &AppState,
    req: &Request<B>,
) -> Result<(), Response<Full<Bytes>>> {
    // Local-stack safety: dev mode is the trusted single-operator context
    // (`pnpm run hc:start:seed`). Pass before any auth resolution so seeding
    // works with no credential locally.
    if state.args.dev_mode {
        return Ok(());
    }

    let level = extract_http_permission(state, req);
    if level >= PermissionLevel::Admin {
        Ok(())
    } else if level >= PermissionLevel::Authenticated {
        // Real identity, but not operator-grade: distinguish from anonymous.
        Err(error_response(
            StatusCode::FORBIDDEN,
            "Admin permission required to seed content",
        ))
    } else {
        Err(error_response(
            StatusCode::UNAUTHORIZED,
            "Authentication required to seed content",
        ))
    }
}

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
#[allow(dead_code)]
struct StorageResponse {
    hash: Option<String>,
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
    // Stage A authorization — gate BEFORE reading the body so a rejected
    // request never streams the blob to cache/storage. Headers-only check.
    if let Err(resp) = require_seed_authority(&state, &req) {
        return resp;
    }

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

        return json_response(
            StatusCode::OK,
            &BlobUploadResponse {
                success: true,
                hash: expected_hash,
                already_cached: true,
                forwarded_to_storage: forwarded,
                size: cached_size,
                error: None,
            },
        );
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
        let error_msg = format!("Hash mismatch: expected {expected_hash}, got {computed_hash}");
        return json_response(
            StatusCode::CONFLICT,
            &BlobUploadResponse {
                success: false,
                hash: expected_hash,
                already_cached: false,
                forwarded_to_storage: false,
                size: blob_size,
                error: Some(error_msg),
            },
        );
    }

    // Store in local cache (1 hour TTL - acts as write-through cache)
    let seed_blob_ttl = std::time::Duration::from_secs(3600);
    state
        .cache
        .set(&expected_hash, body.to_vec(), &content_type, seed_blob_ttl);
    info!(hash = %expected_hash, size = blob_size, "Blob cached locally");

    // Forward to elohim-storage (authoritative store)
    let forwarded = forward_to_storage(&state, &expected_hash, Some(&body)).await;

    if forwarded {
        info!(hash = %expected_hash, "Blob forwarded to elohim-storage");
    } else {
        warn!(hash = %expected_hash, "Failed to forward blob to elohim-storage (will retry on read)");
    }

    json_response(
        StatusCode::OK,
        &BlobUploadResponse {
            success: true,
            hash: expected_hash,
            already_cached: false,
            forwarded_to_storage: forwarded,
            size: blob_size,
            error: None,
        },
    )
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
                // Read-back verification: a 2xx PUT alone has been observed
                // reporting `forwarded_to_storage: true` while the bytes never
                // became durably servable (2026-08-16 local mesh: an 18MB SPA
                // bundle vanished while three sibling blobs from the same run
                // survived; storage accepts the same bytes fine when PUT
                // directly). Until that class has a root cause, "forwarded"
                // means storage SERVES the blob, not that a PUT returned 200.
                let verify_url = format!("{}/blob/{}", storage_url.trim_end_matches('/'), hash);
                match client
                    .get(&verify_url)
                    .timeout(std::time::Duration::from_secs(30))
                    .send()
                    .await
                {
                    Ok(check) if check.status().is_success() => true,
                    Ok(check) => {
                        error!(
                            hash = %hash,
                            status = %check.status(),
                            "seed-blob forward read-back FAILED — storage accepted the PUT \
                             but does not serve the blob; reporting forwarded=false"
                        );
                        false
                    }
                    Err(e) => {
                        error!(
                            hash = %hash,
                            error = %e,
                            "seed-blob forward read-back unreachable — reporting forwarded=false"
                        );
                        false
                    }
                }
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
pub async fn handle_check_blob(hash: &str, state: Arc<AppState>) -> Response<Full<Bytes>> {
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
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    // Use hex encoding to match seeder (blob-manager.ts) convention
    format!("sha256-{result:x}")
}

/// Create JSON response
fn json_response<T: Serialize>(status: StatusCode, data: &T) -> Response<Full<Bytes>> {
    let body = serde_json::to_string(data)
        .unwrap_or_else(|_| r#"{"error":"Serialization failed"}"#.to_string());

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
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
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{JwtValidator, TokenInput};
    use crate::config::Args;
    use clap::Parser;
    use http_body_util::Empty;

    const TEST_JWT_SECRET: &str = "test-secret-that-is-at-least-32-characters-long";

    #[test]
    fn test_compute_sha256() {
        let data = b"hello world";
        let hash = compute_sha256(data);
        // SHA256 of "hello world" is well-known
        assert!(hash.starts_with("sha256-"));
    }

    /// Build an AppState with the given dev_mode and a configured prod JWT
    /// secret so non-dev JWT validation works.
    fn test_state(dev_mode: bool) -> AppState {
        let mut args = Args::parse_from(["doorway", "--listen", "127.0.0.1:0"]);
        args.dev_mode = dev_mode;
        args.api_key_admin = Some("test-admin-key".to_string());
        args.jwt_secret = Some(TEST_JWT_SECRET.to_string());
        AppState::new(args)
    }

    /// Bearer JWT signed with the same prod secret, carrying `level`.
    fn bearer_jwt(level: PermissionLevel) -> String {
        let validator = JwtValidator::new(TEST_JWT_SECRET.into(), 3600).unwrap();
        let input = TokenInput {
            human_id: "human-123".into(),
            agent_pub_key: "uhCAk...".into(),
            identifier: "ci@example.com".into(),
            permission_level: level,
            session_id: None,
            doorway_id: None,
            doorway_url: None,
            conductor_id: None,
            installed_app_id: None,
            is_steward: false,
            has_local_conductor: false,
        };
        validator.generate_token(input).unwrap()
    }

    fn seed_req(bearer: Option<&str>) -> Request<Empty<Bytes>> {
        let mut builder = Request::builder()
            .method("PUT")
            .uri("/admin/seed/blob")
            .header("X-Blob-Hash", "sha256-deadbeef");
        if let Some(t) = bearer {
            builder = builder.header(hyper::header::AUTHORIZATION, format!("Bearer {t}"));
        }
        builder.body(Empty::<Bytes>::new()).unwrap()
    }

    /// (a) PROVE FIRST: dev_mode passes the gate with NO auth header — the
    /// local-stack seeding safety invariant (`pnpm run hc:start:seed`).
    #[test]
    fn dev_mode_no_auth_passes_gate() {
        let state = test_state(true);
        let req = seed_req(None);
        assert!(
            require_seed_authority(&state, &req).is_ok(),
            "dev_mode must pass the seed gate with no credential"
        );
    }

    /// (b) non-dev + no bearer ⇒ 401. The gate returns before the body is read,
    /// so the blob is never forwarded to cache/storage.
    #[test]
    fn non_dev_no_bearer_is_unauthorized() {
        let state = test_state(false);
        let req = seed_req(None);
        let err =
            require_seed_authority(&state, &req).expect_err("no bearer in prod must be rejected");
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    /// (c) non-dev + Admin JWT ⇒ passes the gate.
    #[test]
    fn non_dev_admin_jwt_passes_gate() {
        let state = test_state(false);
        let token = bearer_jwt(PermissionLevel::Admin);
        let req = seed_req(Some(&token));
        assert!(
            require_seed_authority(&state, &req).is_ok(),
            "Admin JWT must pass the seed gate"
        );
    }

    /// (d) non-dev + Authenticated (non-admin) JWT ⇒ 403 (real identity, but
    /// below operator grade).
    #[test]
    fn non_dev_authenticated_jwt_is_forbidden() {
        let state = test_state(false);
        let token = bearer_jwt(PermissionLevel::Authenticated);
        let req = seed_req(Some(&token));
        let err = require_seed_authority(&state, &req)
            .expect_err("non-admin identity must be forbidden from seeding");
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
    }

    /// Admin API key (X-API-Key) also passes — the operator credential CI uses
    /// during the transition window before per-cap commitments land.
    #[test]
    fn non_dev_admin_api_key_passes_gate() {
        let state = test_state(false);
        let req = Request::builder()
            .method("PUT")
            .uri("/admin/seed/blob")
            .header("X-Blob-Hash", "sha256-deadbeef")
            .header("x-api-key", "test-admin-key")
            .body(Empty::<Bytes>::new())
            .unwrap();
        assert!(
            require_seed_authority(&state, &req).is_ok(),
            "Admin API key must pass the seed gate"
        );
    }
}
