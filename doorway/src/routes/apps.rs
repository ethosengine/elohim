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
use hyper::{Method, Request, Response, StatusCode};
use tracing::{debug, info, warn};

/// Handle app proxy requests
///
/// Forwards all /apps/* requests to elohim-storage
pub async fn handle_app_request(
    req: Request<Incoming>,
    storage_url: Option<String>,
    path: &str,
) -> Response<Full<Bytes>> {
    // Handle CORS preflight requests
    if req.method() == Method::OPTIONS {
        return cors_preflight();
    }

    let storage_url = match storage_url {
        Some(url) => url,
        None => {
            warn!("Apps proxy called but STORAGE_URL not configured");
            return Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("Content-Type", "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(Full::new(Bytes::from(
                    r#"{"error": "Storage service not configured. Set STORAGE_URL env var."}"#,
                )))
                .unwrap();
        }
    };

    // Forward the request to elohim-storage
    forward_app_request(&storage_url, path).await
}

/// Forward a /apps/* request to elohim-storage
async fn forward_app_request(storage_url: &str, path: &str) -> Response<Full<Bytes>> {
    // Build the storage endpoint URL
    // path is /apps/... - forward as-is to storage
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), path);

    debug!(url = %storage_endpoint, "Forwarding app request to elohim-storage");

    // Build the forwarded request
    let client = reqwest::Client::new();
    let builder = client.get(&storage_endpoint);

    // Send the request
    match builder.send().await {
        Ok(response) => {
            let status = response.status();
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string();

            // Get cache headers if present
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
                        .header("Access-Control-Allow-Origin", "*")
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

                    builder.body(Full::new(Bytes::from(body.to_vec()))).unwrap()
                }
                Err(e) => {
                    warn!(error = %e, "Failed to read storage response body");
                    Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header("Content-Type", "application/json")
                        .header("Access-Control-Allow-Origin", "*")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error": "Failed to read storage response: {e}"}}"#
                        ))))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            warn!(error = %e, url = %storage_endpoint, "Failed to forward to storage");
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("Content-Type", "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to connect to storage: {e}"}}"#
                ))))
                .unwrap()
        }
    }
}

/// CORS preflight response
fn cors_preflight() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type")
        .header("Access-Control-Max-Age", "86400")
        .body(Full::new(Bytes::new()))
        .unwrap()
}
