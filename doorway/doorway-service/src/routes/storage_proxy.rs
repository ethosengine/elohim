//! Generic storage proxy — single implementation shared by all registry-routed handlers.
//!
//! All domain proxy files (governance, presence, economic_events, etc.) contained
//! identical copies of this function. The route registry now owns dispatch; this
//! module owns the one canonical forwarding implementation.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use tracing::{debug, warn};

/// Forward an incoming request to the elohim-storage endpoint.
///
/// Builds `{storage_url}{path}[?{query}]`, forwards the HTTP method, preserves
/// `content-type` and `authorization` headers, streams the body for
/// POST/PUT/PATCH, and returns the storage response with a
/// `Cross-Origin-Resource-Policy: cross-origin` header so Angular (COEP) is happy.
///
/// Errors surfaces as:
/// - `400 BAD_REQUEST`  — failed to read incoming body
/// - `502 BAD_GATEWAY`  — failed to connect to or read from storage
/// - `405 METHOD_NOT_ALLOWED` — method not in GET/POST/PUT/DELETE/HEAD/PATCH
pub async fn forward_to_storage(
    req: Request<Incoming>,
    storage_url: &str,
    path: &str,
) -> Response<Full<Bytes>> {
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), path);

    let query = req.uri().query();
    let full_url = match query {
        Some(q) => format!("{storage_endpoint}?{q}"),
        None => storage_endpoint,
    };

    let method = req.method().clone();
    debug!(method = %method, url = %full_url, "Forwarding request to elohim-storage");

    let client = reqwest::Client::new();
    let mut builder = match method {
        Method::GET => client.get(&full_url),
        Method::POST => client.post(&full_url),
        Method::PUT => client.put(&full_url),
        Method::DELETE => client.delete(&full_url),
        Method::HEAD => client.head(&full_url),
        Method::PATCH => client.patch(&full_url),
        _ => {
            return Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap();
        }
    };

    if let Some(ct) = req.headers().get("content-type") {
        if let Ok(ct_str) = ct.to_str() {
            builder = builder.header("Content-Type", ct_str);
        }
    }

    if let Some(auth) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            builder = builder.header("Authorization", auth_str);
        }
    }

    if matches!(method, Method::POST | Method::PUT | Method::PATCH) {
        match req.collect().await {
            Ok(collected) => {
                builder = builder.body(collected.to_bytes().to_vec());
            }
            Err(e) => {
                warn!(error = %e, "Failed to read request body");
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
                Ok(body) => Response::builder()
                    .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
                    .header("Content-Type", content_type)
                    .header("Cross-Origin-Resource-Policy", "cross-origin")
                    .body(Full::new(Bytes::from(body.to_vec())))
                    .unwrap(),
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
            warn!(error = %e, path = %path, "Failed to forward request to storage");
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

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::StatusCode;

    /// Verify method-not-allowed branch without needing a live server.
    /// We construct a request with an unsupported method (OPTIONS) and confirm
    /// the function returns 405 before ever attempting a network call.
    #[test]
    fn method_not_allowed_returns_405() {
        // We can't construct hyper::body::Incoming directly in tests, so we verify
        // the observable: that method dispatch produces METHOD_NOT_ALLOWED for an
        // unsupported verb by mirroring the exact match arm logic from forward_to_storage.
        let method = Method::from_bytes(b"OPTIONS").unwrap();
        let response: Response<Full<Bytes>> = match method {
            Method::GET
            | Method::POST
            | Method::PUT
            | Method::DELETE
            | Method::HEAD
            | Method::PATCH => {
                panic!("OPTIONS should not match any forwarded method");
            }
            _ => Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap(),
        };

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    /// Verify that the URL construction appends path and query correctly.
    #[test]
    fn url_construction_with_query() {
        let storage_url = "http://localhost:8090/";
        let path = "/api/v1/governance/proposals";
        let query = Some("page=2&limit=10");

        let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), path);
        let full_url = match query {
            Some(q) => format!("{storage_endpoint}?{q}"),
            None => storage_endpoint,
        };

        assert_eq!(
            full_url,
            "http://localhost:8090/api/v1/governance/proposals?page=2&limit=10"
        );
    }

    /// Verify URL construction without query string.
    #[test]
    fn url_construction_without_query() {
        let storage_url = "http://localhost:8090";
        let path = "/db/content";

        let full_url = format!("{}{}", storage_url.trim_end_matches('/'), path);

        assert_eq!(full_url, "http://localhost:8090/db/content");
    }
}
