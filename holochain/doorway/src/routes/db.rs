//! Database Route Handler - Forwards /db/ requests to elohim-storage
//!
//! This is an infrastructure route that proxies database requests to elohim-storage,
//! similar to how /import/ routes proxy import requests.
//!
//! ## Architecture
//!
//! ```text
//! Browser → Doorway → elohim-storage
//!              │           │
//!         (proxy)    (SQLite content)
//! ```
//!
//! For browser clients, doorway must proxy to elohim-storage because:
//! 1. Different origin (CORS would block direct access)
//! 2. elohim-storage doesn't handle authentication
//! 3. Centralized logging and monitoring
//!
//! ## Endpoints (forwarded to storage)
//!
//! - GET /db/content - List content
//! - GET /db/content/{id} - Get content by ID
//! - POST /db/content/bulk - Bulk create content
//! - GET /db/paths - List paths
//! - GET /db/paths/{id} - Get path by ID with steps
//! - POST /db/paths/bulk - Bulk create paths
//! - GET /db/stats - Database statistics

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use tracing::{debug, info, warn};

/// Handle database proxy requests
///
/// Forwards all /db/* requests to elohim-storage
pub async fn handle_db_request(
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
            warn!("Database proxy called but STORAGE_URL not configured");
            return Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("Content-Type", "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(Full::new(Bytes::from(
                    r#"{"error": "Storage service not configured. Set STORAGE_URL env var."}"#
                )))
                .unwrap();
        }
    };

    // Forward the request to elohim-storage
    forward_db_request(req, &storage_url, path).await
}

/// Forward a /db/* request to elohim-storage
async fn forward_db_request(
    req: Request<Incoming>,
    storage_url: &str,
    path: &str,
) -> Response<Full<Bytes>> {
    // Build the storage endpoint URL
    // path is /db/... - forward as-is to storage
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), path);

    // Preserve query string
    let query = req.uri().query();
    let full_url = if let Some(q) = query {
        format!("{}?{}", storage_endpoint, q)
    } else {
        storage_endpoint
    };

    let method = req.method().clone();
    debug!(method = %method, url = %full_url, "Forwarding to elohim-storage");

    // Build the forwarded request
    let client = reqwest::Client::new();
    let mut builder = match method {
        Method::GET => client.get(&full_url),
        Method::POST => client.post(&full_url),
        Method::PUT => client.put(&full_url),
        Method::DELETE => client.delete(&full_url),
        Method::HEAD => client.head(&full_url),
        _ => {
            return Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header("Content-Type", "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(Full::new(Bytes::from(
                    r#"{"error": "Method not allowed"}"#
                )))
                .unwrap();
        }
    };

    // Forward content-type header if present
    if let Some(ct) = req.headers().get("content-type") {
        if let Ok(ct_str) = ct.to_str() {
            builder = builder.header("Content-Type", ct_str);
        }
    }

    // Forward authorization header if present
    if let Some(auth) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            builder = builder.header("Authorization", auth_str);
        }
    }

    // Forward body for POST/PUT
    if matches!(method, Method::POST | Method::PUT) {
        match req.collect().await {
            Ok(collected) => {
                let body_bytes = collected.to_bytes();
                builder = builder.body(body_bytes.to_vec());
            }
            Err(e) => {
                warn!(error = %e, "Failed to read request body");
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "application/json")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Failed to read request body: {}"}}"#,
                        e
                    ))))
                    .unwrap();
            }
        }
    }

    // Send the request
    match builder.send().await {
        Ok(response) => {
            let status = response.status();
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/json")
                .to_string();

            match response.bytes().await {
                Ok(body) => {
                    info!(
                        status = %status,
                        size = body.len(),
                        path = %path,
                        "Forwarded database response"
                    );

                    Response::builder()
                        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
                        .header("Content-Type", content_type)
                        .header("Access-Control-Allow-Origin", "*")
                        .body(Full::new(Bytes::from(body.to_vec())))
                        .unwrap()
                }
                Err(e) => {
                    warn!(error = %e, "Failed to read storage response body");
                    Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header("Content-Type", "application/json")
                        .header("Access-Control-Allow-Origin", "*")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error": "Failed to read storage response: {}"}}"#,
                            e
                        ))))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            warn!(error = %e, url = %full_url, "Failed to forward to storage");
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("Content-Type", "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to connect to storage: {}"}}"#,
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
        .header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type, Authorization")
        .header("Access-Control-Max-Age", "86400")
        .body(Full::new(Bytes::new()))
        .unwrap()
}
