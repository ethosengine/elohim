//! HTML5 App Route Handler — Projection Cache with Storage Fallback
//!
//! Serves HTML5 app assets (JS, CSS, images) from a MongoDB projection cache
//! with transparent fallback to elohim-storage for cache misses. When a browser
//! loads an HTML5 app (e.g., a Sophia quiz), it fires 30+ concurrent requests
//! for assets — the cache absorbs this web2 traffic pattern so storage stays
//! focused on P2P.
//!
//! ## Cache-First Flow
//!
//! ```text
//! Browser → Doorway
//!              ├─ Cache HIT  → serve from MongoDB         (X-Cache: HIT)
//!              ├─ Coalesced  → wait for in-flight fetch    (X-Cache: HIT-COALESCED)
//!              ├─ Cache MISS → fetch from storage, cache   (X-Cache: MISS)
//!              └─ No cache   → direct proxy to storage     (X-Cache: BYPASS)
//! ```
//!
//! ## Endpoints
//!
//! - GET /apps/{app_id}/{path} - Serve file from HTML5 app ZIP

use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::server::AppState;

/// Maximum retries for transient failures (502/connection errors).
/// HTML5 apps load 30+ assets concurrently which can overwhelm storage
/// during extraction; a brief retry absorbs the back-pressure.
const MAX_RETRIES: u32 = 2;

/// Base delay between retries (doubles each attempt).
const RETRY_BASE_DELAY_MS: u64 = 100;

/// Handle app requests with cache-first resolution.
///
/// 1. Parse path into app_id and file_path
/// 2. Resolve blob_hash from app index (projection store lookup)
/// 3. If cache available AND blob_hash known: try cache → coalesce → fetch
/// 4. Otherwise: fall through to direct proxy (BYPASS)
pub async fn handle_app_request(
    _req: Request<Incoming>,
    state: Arc<AppState>,
    path: &str,
) -> Response<Full<Bytes>> {
    let storage_url = match &state.args.storage_url {
        Some(url) => url.clone(),
        None => {
            warn!("Apps proxy called but STORAGE_URL not configured");
            return Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error": "Storage service not configured. Set STORAGE_URL env var."}"#,
                )))
                .unwrap();
        }
    };

    // Parse /apps/{app_id}/{file_path...}
    let (app_id, file_path) = match parse_app_path(path) {
        Some(parsed) => parsed,
        None => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "application/json")
                .header("X-Cache", "BYPASS")
                .body(Full::new(Bytes::from(
                    r#"{"error": "Invalid app path. Expected /apps/{app_id}/{file_path}"}"#,
                )))
                .unwrap();
        }
    };

    // Cache-first path: only active when cache service AND blob_hash are available.
    // blob_hash is resolved from the app index (populated from projection store).
    if let Some(ref cache) = state.app_file_cache {
        let blob_hash = cache.resolve_blob_hash(app_id).await;

        if let Some(hash) = blob_hash {
            // --- Try cache lookup ---
            if let Some(cached) = cache.get(app_id, file_path, &hash).await {
                debug!(app_id = %app_id, file_path = %file_path, "App file cache HIT");
                return build_app_response(&cached.data, &cached.content_type, "HIT");
            }

            // --- Try in-flight coalescing ---
            match cache.begin_fetch(app_id, file_path) {
                Some(mut rx) => {
                    // Another task is already fetching this file — wait for it
                    debug!(
                        app_id = %app_id,
                        file_path = %file_path,
                        "Coalescing app file fetch (waiting for leader)"
                    );
                    match rx.recv().await {
                        Ok(Some(cached)) => {
                            return build_app_response(
                                &cached.data,
                                &cached.content_type,
                                "HIT-COALESCED",
                            );
                        }
                        _ => {
                            // Leader failed or channel closed — fall through to proxy
                            warn!(
                                app_id = %app_id,
                                file_path = %file_path,
                                "Coalesced fetch failed, falling through to proxy"
                            );
                            return forward_app_request_with_header(&storage_url, path, "MISS")
                                .await;
                        }
                    }
                }
                None => {
                    // We are the leader — fetch from storage, cache, broadcast
                    let response =
                        fetch_and_cache(cache, &storage_url, path, app_id, file_path, &hash).await;
                    return response;
                }
            }
        }
    }

    // No cache or no blob_hash — direct proxy (existing behaviour)
    forward_app_request_with_header(&storage_url, path, "BYPASS").await
}

/// Parse `/apps/{app_id}/{file_path...}` into (app_id, file_path).
///
/// Returns `None` if the path doesn't have at least an app_id and one file
/// path segment.
fn parse_app_path(path: &str) -> Option<(&str, &str)> {
    // Strip leading "/apps/"
    let rest = path.strip_prefix("/apps/")?;
    if rest.is_empty() {
        return None;
    }

    // Split into app_id and file_path at the first '/'
    let (app_id, file_path) = match rest.find('/') {
        Some(idx) => {
            let (id, remainder) = rest.split_at(idx);
            // remainder starts with '/', strip it
            (id, &remainder[1..])
        }
        None => {
            // No file path segment — just an app_id
            return None;
        }
    };

    if app_id.is_empty() || file_path.is_empty() {
        return None;
    }

    Some((app_id, file_path))
}

/// Fetch a file from storage, cache it, broadcast to waiters, and return the response.
async fn fetch_and_cache(
    cache: &crate::cache::AppFileCacheService,
    storage_url: &str,
    full_path: &str,
    app_id: &str,
    file_path: &str,
    blob_hash: &str,
) -> Response<Full<Bytes>> {
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), full_path);
    let client = reqwest::Client::new();

    let result = client.get(&storage_endpoint).send().await;

    match result {
        Ok(response) => {
            let status = response.status();

            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string();

            match response.bytes().await {
                Ok(body) => {
                    if status.is_success() {
                        // Cache the file
                        cache
                            .put(app_id, file_path, blob_hash, &content_type, body.to_vec())
                            .await;

                        // Broadcast to coalesced waiters
                        let cached_file = crate::cache::CachedFile {
                            data: body.to_vec(),
                            content_type: content_type.clone(),
                            blob_hash: blob_hash.to_string(),
                        };
                        cache.finish_fetch(app_id, file_path, Some(cached_file));

                        info!(
                            app_id = %app_id,
                            file_path = %file_path,
                            size = body.len(),
                            "App file cached (MISS)"
                        );

                        build_app_response(&body, &content_type, "MISS")
                    } else {
                        // Non-success status — don't cache, broadcast failure
                        cache.finish_fetch(app_id, file_path, None);

                        Response::builder()
                            .status(
                                StatusCode::from_u16(status.as_u16())
                                    .unwrap_or(StatusCode::BAD_GATEWAY),
                            )
                            .header("Content-Type", &content_type)
                            .header("X-Cache", "MISS")
                            .body(Full::new(Bytes::from(body.to_vec())))
                            .unwrap()
                    }
                }
                Err(e) => {
                    cache.finish_fetch(app_id, file_path, None);
                    warn!(error = %e, "Failed to read storage response body during cache fetch");
                    Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header("Content-Type", "application/json")
                        .header("X-Cache", "MISS")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error": "Failed to read storage response: {e}"}}"#
                        ))))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            cache.finish_fetch(app_id, file_path, None);
            warn!(error = %e, "Failed to connect to storage during cache fetch");
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("Content-Type", "application/json")
                .header("X-Cache", "MISS")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to connect to storage: {e}"}}"#
                ))))
                .unwrap()
        }
    }
}

/// Build an HTTP response for an app file with standard headers.
///
/// Sets COEP/CORP headers (required for cross-origin embedding in Angular),
/// cache control, and the X-Cache diagnostic header.
fn build_app_response(
    data: &[u8],
    content_type: &str,
    cache_status: &str,
) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", content_type)
        .header("Cross-Origin-Resource-Policy", "cross-origin")
        .header("Cross-Origin-Embedder-Policy", "credentialless")
        .header("X-Cache", cache_status)
        .header("Cache-Control", "public, max-age=3600")
        .body(Full::new(Bytes::from(data.to_vec())))
        .unwrap()
}

/// Forward a /apps/* request to elohim-storage with retry for transient errors.
///
/// This is the existing proxy path, preserved for BYPASS and fallback scenarios.
/// Adds the X-Cache header to the response.
async fn forward_app_request_with_header(
    storage_url: &str,
    path: &str,
    cache_status: &str,
) -> Response<Full<Bytes>> {
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), path);

    debug!(url = %storage_endpoint, "Forwarding app request to elohim-storage");

    let client = reqwest::Client::new();

    for attempt in 0..=MAX_RETRIES {
        let result = client.get(&storage_endpoint).send().await;

        match result {
            Ok(response) => {
                let status = response.status();

                // Retry on 502 (storage overloaded/restarting)
                if status == reqwest::StatusCode::BAD_GATEWAY && attempt < MAX_RETRIES {
                    let delay = RETRY_BASE_DELAY_MS * 2u64.pow(attempt);
                    warn!(
                        attempt = attempt + 1,
                        delay_ms = delay,
                        path = %path,
                        "Storage returned 502, retrying"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    continue;
                }

                let content_type = response
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("application/octet-stream")
                    .to_string();

                let cache_control = response
                    .headers()
                    .get("cache-control")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());

                let etag = response
                    .headers()
                    .get("etag")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());

                match response.bytes().await {
                    Ok(body) => {
                        info!(
                            status = %status,
                            size = body.len(),
                            path = %path,
                            "Forwarded app response"
                        );

                        let mut builder = Response::builder()
                            .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
                            .header("Content-Type", content_type)
                            // Required for COEP: require-corp in Angular app
                            .header("Cross-Origin-Resource-Policy", "cross-origin")
                            // Required for iframes embedded in COEP pages
                            .header("Cross-Origin-Embedder-Policy", "credentialless")
                            .header("X-Cache", cache_status);

                        if let Some(cc) = cache_control {
                            builder = builder.header("Cache-Control", cc);
                        }

                        if let Some(et) = etag {
                            builder = builder.header("ETag", et);
                        }

                        return builder.body(Full::new(Bytes::from(body.to_vec()))).unwrap();
                    }
                    Err(e) if attempt < MAX_RETRIES => {
                        let delay = RETRY_BASE_DELAY_MS * 2u64.pow(attempt);
                        warn!(
                            attempt = attempt + 1,
                            delay_ms = delay,
                            error = %e,
                            "Failed to read storage body, retrying"
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        continue;
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to read storage response body");
                        return Response::builder()
                            .status(StatusCode::BAD_GATEWAY)
                            .header("Content-Type", "application/json")
                            .header("X-Cache", cache_status)
                            .body(Full::new(Bytes::from(format!(
                                r#"{{"error": "Failed to read storage response: {e}"}}"#
                            ))))
                            .unwrap();
                    }
                }
            }
            Err(e) if attempt < MAX_RETRIES => {
                let delay = RETRY_BASE_DELAY_MS * 2u64.pow(attempt);
                warn!(
                    attempt = attempt + 1,
                    delay_ms = delay,
                    error = %e,
                    "Failed to connect to storage, retrying"
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                continue;
            }
            Err(e) => {
                warn!(error = %e, url = %storage_endpoint, "Failed to forward to storage");
                return Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .header("Content-Type", "application/json")
                    .header("X-Cache", cache_status)
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Failed to connect to storage: {e}"}}"#
                    ))))
                    .unwrap();
            }
        }
    }

    // Unreachable, but satisfy the compiler
    Response::builder()
        .status(StatusCode::BAD_GATEWAY)
        .header("Content-Type", "application/json")
        .header("X-Cache", cache_status)
        .body(Full::new(Bytes::from(
            r#"{"error": "Max retries exhausted"}"#,
        )))
        .unwrap()
}

/// Parse `/apps/{app_id}/_capability` into the app_id.
///
/// Returns `None` if the path doesn't match the expected pattern.
fn parse_capability_path(path: &str) -> Option<&str> {
    let app_id = path
        .strip_prefix("/apps/")
        .and_then(|s| s.strip_suffix("/_capability"))?;
    if app_id.is_empty() || app_id.contains('/') {
        return None;
    }
    Some(app_id)
}

/// Handle delivery capability probe for an HTML5 app.
///
/// Route: HEAD /apps/{app_id}/_capability
///
/// Returns an empty body with headers describing the delivery readiness
/// of this doorway for the given app. If doorway has a projection cache,
/// reports its own capability (projection tier). Otherwise, proxies the
/// HEAD request to elohim-storage for its extraction-tier capability.
pub async fn handle_app_capability(state: Arc<AppState>, path: &str) -> Response<Full<Bytes>> {
    let app_id = match parse_capability_path(path) {
        Some(id) => id,
        None => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Length", "0")
                .body(Full::new(Bytes::new()))
                .unwrap();
        }
    };

    // If we have projection cache, report our own capability
    if let Some(ref cache) = state.app_file_cache {
        let blob_hash = cache.resolve_blob_hash(app_id).await;
        let ready = blob_hash.is_some();

        let mut builder = Response::builder()
            .status(StatusCode::OK)
            .header("Content-Length", "0")
            .header("X-Delivery-Mode", "extracted")
            .header("X-Cache-Tier", "projection")
            .header("X-Ready", if ready { "true" } else { "false" });

        if let Some(hash) = &blob_hash {
            builder = builder.header("X-Blob-Hash", hash.as_str());
        }

        return builder.body(Full::new(Bytes::new())).unwrap();
    }

    // No projection cache — proxy HEAD to storage for its capability
    let storage_url = match &state.args.storage_url {
        Some(url) => url,
        None => {
            return Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("Content-Length", "0")
                .body(Full::new(Bytes::new()))
                .unwrap();
        }
    };

    let endpoint = format!("{}{}", storage_url.trim_end_matches('/'), path);
    match reqwest::Client::new().head(&endpoint).send().await {
        Ok(resp) => {
            let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::OK);
            let mut builder = Response::builder().status(status);

            // Forward X-* headers from storage
            for (name, value) in resp.headers() {
                if name.as_str().starts_with("x-") {
                    builder = builder.header(name, value);
                }
            }

            builder
                .header("Content-Length", "0")
                .body(Full::new(Bytes::new()))
                .unwrap()
        }
        Err(e) => {
            warn!(error = %e, "Failed to proxy capability probe to storage");
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("Content-Length", "0")
                .body(Full::new(Bytes::new()))
                .unwrap()
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_app_path_normal() {
        let (app_id, file_path) = parse_app_path("/apps/my-app/index.html").unwrap();
        assert_eq!(app_id, "my-app");
        assert_eq!(file_path, "index.html");
    }

    #[test]
    fn test_parse_app_path_nested() {
        let (app_id, file_path) = parse_app_path("/apps/quiz-123/js/vendor/pixi.min.js").unwrap();
        assert_eq!(app_id, "quiz-123");
        assert_eq!(file_path, "js/vendor/pixi.min.js");
    }

    #[test]
    fn test_parse_app_path_no_file() {
        assert!(parse_app_path("/apps/my-app").is_none());
    }

    #[test]
    fn test_parse_app_path_no_app_id() {
        assert!(parse_app_path("/apps/").is_none());
    }

    #[test]
    fn test_parse_app_path_empty() {
        assert!(parse_app_path("/apps/my-app/").is_none());
    }

    #[test]
    fn test_parse_app_path_not_apps() {
        assert!(parse_app_path("/other/my-app/file.js").is_none());
    }

    #[test]
    fn test_build_app_response_headers() {
        let resp = build_app_response(b"hello", "text/html", "HIT");
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get("Content-Type").unwrap(), "text/html");
        assert_eq!(resp.headers().get("X-Cache").unwrap(), "HIT");
        assert_eq!(
            resp.headers().get("Cross-Origin-Resource-Policy").unwrap(),
            "cross-origin"
        );
        assert_eq!(
            resp.headers().get("Cross-Origin-Embedder-Policy").unwrap(),
            "credentialless"
        );
        assert_eq!(
            resp.headers().get("Cache-Control").unwrap(),
            "public, max-age=3600"
        );
    }

    #[test]
    fn test_build_app_response_bypass() {
        let resp = build_app_response(b"data", "application/javascript", "BYPASS");
        assert_eq!(resp.headers().get("X-Cache").unwrap(), "BYPASS");
    }

    #[test]
    fn test_build_app_response_coalesced() {
        let resp = build_app_response(b"data", "text/css", "HIT-COALESCED");
        assert_eq!(resp.headers().get("X-Cache").unwrap(), "HIT-COALESCED");
    }

    // =========================================================================
    // Capability path parsing tests
    // =========================================================================

    #[test]
    fn test_parse_capability_path_normal() {
        let app_id = parse_capability_path("/apps/my-app/_capability").unwrap();
        assert_eq!(app_id, "my-app");
    }

    #[test]
    fn test_parse_capability_path_with_hash() {
        let app_id = parse_capability_path("/apps/bafkrei-abc123/_capability").unwrap();
        assert_eq!(app_id, "bafkrei-abc123");
    }

    #[test]
    fn test_parse_capability_path_empty_app_id() {
        assert!(parse_capability_path("/apps//_capability").is_none());
    }

    #[test]
    fn test_parse_capability_path_no_capability_suffix() {
        assert!(parse_capability_path("/apps/my-app/index.html").is_none());
    }

    #[test]
    fn test_parse_capability_path_nested_slashes() {
        // app_id should not contain slashes
        assert!(parse_capability_path("/apps/my/nested/app/_capability").is_none());
    }

    #[test]
    fn test_parse_capability_path_wrong_prefix() {
        assert!(parse_capability_path("/other/my-app/_capability").is_none());
    }

    #[test]
    fn test_parse_capability_path_only_prefix() {
        assert!(parse_capability_path("/apps/_capability").is_none());
    }
}
