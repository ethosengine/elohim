//! HTML5 App Route Handler - Forwards /apps/ requests to elohim-storage
//!
//! This is a proxy route that forwards app serving requests to elohim-storage,
//! similar to how /db/ routes proxy database requests.
//!
//! ## Architecture
//!
//! ```text
//! Browser → Doorway → elohim-storage
//!              │           │
//!         (proxy)    (ZIP extraction)
//! ```
//!
//! ## Endpoints (forwarded to storage)
//!
//! - GET /apps/{app_id}/{path} - Serve file from HTML5 app ZIP

use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use tracing::{debug, info, warn};

/// Maximum retries for transient failures (502/connection errors).
/// HTML5 apps load 30+ assets concurrently which can overwhelm storage
/// during extraction; a brief retry absorbs the back-pressure.
const MAX_RETRIES: u32 = 2;

/// Base delay between retries (doubles each attempt).
const RETRY_BASE_DELAY_MS: u64 = 100;

/// Handle app proxy requests
///
/// Forwards all /apps/* requests to elohim-storage
pub async fn handle_app_request(
    _req: Request<Incoming>,
    storage_url: Option<String>,
    path: &str,
) -> Response<Full<Bytes>> {
    let storage_url = match storage_url {
        Some(url) => url,
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

    // Forward the request to elohim-storage with retry for transient failures
    forward_app_request(&storage_url, path).await
}

/// Forward a /apps/* request to elohim-storage, retrying on transient errors.
async fn forward_app_request(storage_url: &str, path: &str) -> Response<Full<Bytes>> {
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
                            .header("Cross-Origin-Embedder-Policy", "credentialless");

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
        .body(Full::new(Bytes::from(
            r#"{"error": "Max retries exhausted"}"#,
        )))
        .unwrap()
}
