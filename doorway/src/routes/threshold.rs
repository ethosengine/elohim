//! Threshold Route Handler - Forwards /threshold/* requests to doorway-app
//!
//! This is a proxy route that serves the operator dashboard Angular app.
//! The doorway-app container serves at `/threshold/` path via nginx.
//!
//! ## Architecture
//!
//! ```text
//! Browser → Doorway → doorway-app (nginx)
//!              │           │
//!         (proxy)    (Angular SPA)
//! ```
//!
//! ## Endpoints (forwarded to doorway-app)
//!
//! - GET /threshold/* - Serve Angular operator dashboard

use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use tracing::{debug, warn};

/// Handle threshold proxy requests
///
/// Forwards all /threshold/* requests to doorway-app container
pub async fn handle_threshold_request(
    req: Request<Incoming>,
    threshold_url: &str,
    path: &str,
) -> Response<Full<Bytes>> {
    // Handle CORS preflight requests
    if req.method() == Method::OPTIONS {
        return cors_preflight();
    }

    // Forward the request to doorway-app
    forward_threshold_request(threshold_url, path).await
}

/// Forward a /threshold/* request to doorway-app
async fn forward_threshold_request(
    threshold_url: &str,
    path: &str,
) -> Response<Full<Bytes>> {
    // Build the doorway-app endpoint URL
    // path is /threshold/... - forward as-is to doorway-app
    let target_url = format!("{}{}", threshold_url.trim_end_matches('/'), path);

    debug!(url = %target_url, "Forwarding threshold request to doorway-app");

    // Build the forwarded request
    let client = reqwest::Client::new();
    let builder = client.get(&target_url);

    // Send the request
    match builder.send().await {
        Ok(response) => {
            let status = response.status();
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("text/html")
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
                    debug!(
                        status = %status,
                        size = body.len(),
                        path = %path,
                        "Forwarded threshold response"
                    );

                    let mut builder = Response::builder()
                        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
                        .header("Content-Type", content_type);

                    if let Some(cc) = cache_control {
                        builder = builder.header("Cache-Control", cc);
                    }

                    if let Some(et) = etag {
                        builder = builder.header("ETag", et);
                    }

                    builder
                        .body(Full::new(Bytes::from(body.to_vec())))
                        .unwrap()
                }
                Err(e) => {
                    warn!(error = %e, "Failed to read doorway-app response body");
                    Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error": "Failed to read doorway-app response: {}"}}"#,
                            e
                        ))))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            warn!(error = %e, url = %target_url, "Failed to forward to doorway-app");
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to connect to doorway-app: {}"}}"#,
                    e
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
