//! Generic Cache API
//!
//! Doorway provides a generic cache serving layer. Applications register
//! content to be cached, and doorway serves it.
//!
//! ## Routes
//!
//! - `GET /api/v1/cache/{type}/{id}` - Get cached document by type and ID
//! - `GET /api/v1/cache/{type}` - Query cached documents by type
//!
//! ## Architecture
//!
//! The app (elohim-app) owns all route definitions and content structure.
//! Doorway is a dumb cache that:
//! 1. Stores content in ProjectionStore (populated via DHT signals)
//! 2. Serves cached content when requested
//!
//! Doorway does NOT define app-specific routes or content mappings.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use serde::Serialize;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::cache::{extract_reach_from_response, should_serve_response, extract_requester_context};
use crate::projection::ProjectionQuery;
use crate::server::AppState;

/// API error response
#[derive(Debug, Serialize)]
struct ApiError {
    error: String,
    code: &'static str,
}

/// Parsed cache route components
#[derive(Debug)]
struct CacheRoute<'a> {
    /// Document type (e.g., "Content", "LearningPath")
    doc_type: &'a str,
    /// Optional document ID for single lookups
    doc_id: Option<&'a str>,
}

impl<'a> CacheRoute<'a> {
    /// Parse route from path like "/api/v1/cache/{type}" or "/api/v1/cache/{type}/{id}"
    fn parse(path: &'a str) -> Option<Self> {
        let stripped = path.strip_prefix("/api/v1/cache/")?;
        let parts: Vec<&str> = stripped.splitn(2, '/').collect();

        if parts.is_empty() || parts[0].is_empty() {
            return None;
        }

        Some(Self {
            doc_type: parts[0],
            doc_id: parts.get(1).copied().filter(|s| !s.is_empty()),
        })
    }
}

/// Build a JSON error response
fn error_response(status: StatusCode, message: &str, code: &'static str) -> Response<Full<Bytes>> {
    let error = ApiError {
        error: message.to_string(),
        code,
    };
    let body = serde_json::to_vec(&error).unwrap_or_default();

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Cache-Control", "no-cache")
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(body)))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(r#"{"error":"Internal error"}"#)))
                .unwrap()
        })
}

/// Build response from projection data with optional reach header
fn projection_response(data: Vec<u8>, reach: Option<String>) -> Response<Full<Bytes>> {
    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Cache-Control", "public, max-age=60");

    if let Some(r) = reach {
        response = response.header("X-Reach", r);
    }

    response
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(data)))
        .unwrap()
}

/// Parse query string into key-value map
fn parse_query_params(query: &str) -> HashMap<String, String> {
    if query.is_empty() {
        return HashMap::new();
    }

    query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?;
            let value = parts.next().unwrap_or("");
            Some((key.to_string(), value.to_string()))
        })
        .collect()
}

/// Handle GET /api/v1/cache/{type}/{id} or /api/v1/cache/{type}
///
/// Generic cache serving endpoint. The app defines what content exists
/// and how to access it. Doorway just serves cached documents.
pub async fn handle_api_request(
    state: Arc<AppState>,
    path: &str,
    query: Option<&str>,
    remote_addr: Option<IpAddr>,
    auth_header: Option<String>,
) -> Response<Full<Bytes>> {
    // Parse cache route
    let route = match CacheRoute::parse(path) {
        Some(r) => r,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Invalid route. Expected: /api/v1/cache/{type} or /api/v1/cache/{type}/{id}",
                "INVALID_ROUTE",
            );
        }
    };

    // Extract requester context from auth header and IP
    let requester = extract_requester_context(auth_header.as_deref(), remote_addr);
    let beneficiary_id = "unknown"; // Would come from content metadata

    // Get projection store
    let projection = match &state.projection {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "Projection store not available",
                "PROJECTION_UNAVAILABLE",
            );
        }
    };

    debug!("Cache request: type={}, id={:?}", route.doc_type, route.doc_id);

    // Single document lookup
    if let Some(id) = route.doc_id {
        match projection.get(route.doc_type, id).await {
            Some(doc) => {
                let response = serde_json::to_vec(&doc.data).unwrap_or_default();

                // Check reach-based access
                if !should_serve_response(&response, &requester, beneficiary_id) {
                    return error_response(
                        StatusCode::FORBIDDEN,
                        "Content not accessible",
                        "REACH_DENIED",
                    );
                }

                let reach = extract_reach_from_response(&response);
                return projection_response(response, reach);
            }
            None => {
                return error_response(
                    StatusCode::NOT_FOUND,
                    &format!("Not found: {}/{}", route.doc_type, id),
                    "NOT_FOUND",
                );
            }
        }
    }

    // Collection query
    let params = parse_query_params(query.unwrap_or(""));
    let limit = params.get("limit")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(100)
        .min(1000);

    let proj_query = ProjectionQuery::by_type(route.doc_type)
        .with_limit(limit);

    match projection.query(proj_query).await {
        Ok(docs) => {
            // Filter by reach and collect accessible documents
            let accessible: Vec<_> = docs.iter()
                .filter(|doc| {
                    let bytes = serde_json::to_vec(&doc.data).unwrap_or_default();
                    should_serve_response(&bytes, &requester, beneficiary_id)
                })
                .map(|doc| &doc.data)
                .collect();

            let response = serde_json::to_vec(&accessible).unwrap_or_default();
            projection_response(response, None)
        }
        Err(e) => {
            warn!("Projection query failed: {}", e);
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Query failed",
                "QUERY_FAILED",
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cache_route_with_id() {
        let route = CacheRoute::parse("/api/v1/cache/Content/manifesto").unwrap();
        assert_eq!(route.doc_type, "Content");
        assert_eq!(route.doc_id, Some("manifesto"));
    }

    #[test]
    fn test_parse_cache_route_collection() {
        let route = CacheRoute::parse("/api/v1/cache/LearningPath").unwrap();
        assert_eq!(route.doc_type, "LearningPath");
        assert_eq!(route.doc_id, None);
    }

    #[test]
    fn test_parse_cache_route_invalid() {
        assert!(CacheRoute::parse("/api/v1/cache/").is_none());
        assert!(CacheRoute::parse("/api/v1/cache").is_none());
        assert!(CacheRoute::parse("/other/path").is_none());
    }

    #[test]
    fn test_parse_query_params() {
        let params = parse_query_params("limit=50&skip=10");
        assert_eq!(params.get("limit"), Some(&"50".to_string()));
        assert_eq!(params.get("skip"), Some(&"10".to_string()));
    }

    #[test]
    fn test_parse_query_params_empty() {
        let params = parse_query_params("");
        assert!(params.is_empty());
    }

    #[test]
    fn test_error_response() {
        let resp = error_response(StatusCode::NOT_FOUND, "Test error", "TEST_ERROR");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
