//! Account Route Handler - Forwards /account/ requests to elohim-storage
//!
//! Proxies account import/export requests for auth and localhost-binding support.
//!
//! ## Architecture
//!
//! ```text
//! Browser/Seeder → Doorway → elohim-storage
//!                    │           │
//!               (proxy+auth) (account operations)
//! ```
//!
//! ## Endpoints (forwarded to storage)
//!
//! - POST /account/import - Import an account package
//! - GET /account/export/{human_id} - Export an account package

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use tracing::{debug, info, warn};

/// Handle account proxy requests
///
/// Forwards all /account/* requests to elohim-storage
pub async fn handle_account_request(
    req: Request<Incoming>,
    storage_url: Option<String>,
    path: &str,
) -> Response<Full<Bytes>> {
    let storage_url = match storage_url {
        Some(url) => url,
        None => {
            warn!("Account proxy called but STORAGE_URL not configured");
            return Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error": "Storage service not configured. Set STORAGE_URL env var."}"#,
                )))
                .unwrap();
        }
    };

    forward_account_request(req, &storage_url, path).await
}

/// Forward an /account/* request to elohim-storage
async fn forward_account_request(
    req: Request<Incoming>,
    storage_url: &str,
    path: &str,
) -> Response<Full<Bytes>> {
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), path);

    let query = req.uri().query();
    let full_url = if let Some(q) = query {
        format!("{storage_endpoint}?{q}")
    } else {
        storage_endpoint
    };

    let method = req.method().clone();
    debug!(method = %method, url = %full_url, "Forwarding account request to elohim-storage");

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
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
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
                warn!(error = %e, "Failed to read account request body");
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Failed to read request body: {e}"}}"#
                    ))))
                    .unwrap();
            }
        }
    }

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
                        "Forwarded account response"
                    );

                    Response::builder()
                        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
                        .header("Content-Type", content_type)
                        .header("Cross-Origin-Resource-Policy", "cross-origin")
                        .body(Full::new(Bytes::from(body.to_vec())))
                        .unwrap()
                }
                Err(e) => {
                    warn!(error = %e, "Failed to read storage response body");
                    Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error": "Failed to read storage response: {e}"}}"#
                        ))))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            warn!(error = %e, url = %full_url, "Failed to forward account request to storage");
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to connect to storage: {e}"}}"#
                ))))
                .unwrap()
        }
    }
}
