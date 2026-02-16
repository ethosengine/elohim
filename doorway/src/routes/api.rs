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
//! Doorway is a **thin HTTP gateway**. It does NOT:
//! - Define content types (DNA defines those)
//! - Parse response bodies (DNA returns what it returns)
//! - Enforce access control (DNA/conductor checks reach/governance)
//!
//! Doorway ONLY:
//! - Routes HTTP requests to projection/conductor
//! - Passes requester identity to conductor (for access control)
//! - Translates success/error to HTTP status codes
//!
//! Access control happens in the DNA layer which has the full context
//! of reach levels, governance rules, and identity relationships.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use serde::Serialize;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::projection::ProjectionQuery;
use crate::server::AppState;
use crate::worker::RequesterIdentity;

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

/// Build successful JSON response
fn json_response(data: Vec<u8>) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Cache-Control", "public, max-age=60")
        .header("Access-Control-Allow-Origin", "*")
        // Required for COEP: require-corp in Angular app
        .header("Cross-Origin-Resource-Policy", "cross-origin")
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

/// Parse requester identity from auth header
///
/// Supports formats:
/// - "Bearer <JWT>" - Extract agent ID from JWT claims
/// - "Agent <pubkey>" - Direct agent public key
fn parse_requester_identity(auth_header: Option<&str>) -> Option<RequesterIdentity> {
    let header = auth_header?;

    // Simple parsing - in production would validate JWT properly
    if header.starts_with("Bearer ") {
        // JWT token - extract agent_id from claims (simplified)
        let _token = header.strip_prefix("Bearer ")?;
        // TODO: Properly decode JWT and extract agent_id
        Some(RequesterIdentity {
            agent_id: None, // Would come from JWT claims
            location: None,
            authenticated: true,
        })
    } else if header.starts_with("Agent ") {
        // Direct agent pubkey
        let agent_id = header.strip_prefix("Agent ")?.to_string();
        Some(RequesterIdentity {
            agent_id: Some(agent_id),
            location: None,
            authenticated: true,
        })
    } else {
        None
    }
}

/// Handle GET /api/v1/cache/{type}/{id} or /api/v1/cache/{type}
///
/// Thin HTTP gateway. Doorway does NOT interpret response bodies or enforce
/// access control - that's the DNA's job. Doorway only:
/// 1. Routes to projection/conductor
/// 2. Passes requester identity (for conductor to check access)
/// 3. Translates success/error to HTTP
pub async fn handle_api_request(
    state: Arc<AppState>,
    path: &str,
    query: Option<&str>,
    _remote_addr: Option<IpAddr>,
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

    // Parse requester identity from auth header (passed to DNA for access control)
    let requester = parse_requester_identity(auth_header.as_deref());

    debug!(
        "Cache request: type={}, id={:?}, has_identity={}",
        route.doc_type,
        route.doc_id,
        requester.is_some()
    );

    // Single document lookup - use DoorwayResolver for tiered fallback
    // Doorway is type-agnostic: passes type string through to projection/conductor
    // Access control happens in the DNA layer, not here
    if let Some(id) = route.doc_id {
        // Generic resolution for any type (tiered: projection → conductor)
        // Identity passed through for DNA-level access control
        let result = state
            .resolver
            .resolve_with_identity(route.doc_type, id, requester)
            .await;

        return match result {
            Ok(resolution) => {
                info!(
                    doc_type = route.doc_type,
                    id = id,
                    source = resolution.source_id,
                    duration_ms = resolution.duration_ms,
                    "Document resolved"
                );

                // Return whatever the DNA returned - no interpretation
                let response = serde_json::to_vec(&resolution.data).unwrap_or_default();
                json_response(response)
            }
            Err(e) => {
                debug!(doc_type = route.doc_type, id = id, error = ?e, "Resolution failed");
                // TODO: Distinguish NotFound vs AccessDenied from conductor errors
                error_response(
                    StatusCode::NOT_FOUND,
                    &format!("Not found: {}/{}", route.doc_type, id),
                    "NOT_FOUND",
                )
            }
        };
    }

    // Collection query - use projection directly
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

    let params = parse_query_params(query.unwrap_or(""));
    let limit = params
        .get("limit")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(100)
        .min(1000);

    // TODO: Pass requester identity for access-filtered queries
    let proj_query = ProjectionQuery::by_type(route.doc_type).with_limit(limit);

    match projection.query(proj_query).await {
        Ok(docs) => {
            // Return whatever projection returned - no filtering here
            // Access control should happen at projection query level
            let data: Vec<_> = docs.iter().map(|doc| &doc.data).collect();
            let response = serde_json::to_vec(&data).unwrap_or_default();
            json_response(response)
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
