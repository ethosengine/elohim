//! Generic Cache API
//!
//! Doorway provides a generic anonymous projection serving layer. Applications
//! register content to be cached, and doorway serves exact `public` or
//! `commons` declarations.
//!
//! ## Routes
//!
//! - `GET /api/v1/cache/{type}/{id}` - Get cached document by type and ID
//! - `GET /api/v1/cache/{type}` - Query cached documents by type
//!
//! ## Architecture
//!
//! Doorway is a **thin HTTP gateway**. It does not:
//! - Define content types (DNA defines those)
//! - Interpret domain-specific response fields
//! - Author reach or governance decisions
//!
//! Doorway:
//! - Routes HTTP requests to projection/conductor
//! - Serves only anonymous-open rows from its local projection
//! - Passes validated bearer identity to conductor (for access control)
//! - Translates success/error to HTTP status codes
//!
//! Access control happens in the DNA layer which has the full context
//! of reach levels, governance rules, and identity relationships.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use serde::Serialize;
use tracing::{debug, info, warn};

use crate::auth::{extract_token_from_header, JwtValidator};
use crate::projection::cache_refresh::is_cacheable_reach;
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
    /// Document type (e.g., "Content", "Human")
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
/// Supports `Bearer <JWT>` by extracting the agent public key from validated
/// claims. Caller-supplied agent strings are not identity evidence.
fn parse_requester_identity(
    auth_header: Option<&str>,
    state: &AppState,
) -> Option<RequesterIdentity> {
    let header = auth_header?;

    if header.starts_with("Bearer ") {
        // JWT token - validate and extract agent_pub_key from claims
        let token = extract_token_from_header(Some(header))?;

        // Build validator from state (mirrors get_jwt_validator in auth_routes.rs).
        // Keyed on JWT_SECRET presence, never dev_mode (JwtValidator::from_config).
        let validator = JwtValidator::from_config(
            state.args.configured_jwt_secret(),
            state.args.jwt_expiry_seconds,
        )
        .ok()?;

        let result = validator.verify_token(token);
        if result.valid {
            let claims = result.claims?;
            Some(RequesterIdentity {
                agent_id: Some(claims.agent_pub_key),
                location: None,
                authenticated: true,
            })
        } else {
            debug!(
                error = ?result.error,
                "JWT validation failed for API request"
            );
            // Return unauthenticated identity rather than blocking the request;
            // the DNA layer enforces access control based on authenticated = false.
            Some(RequesterIdentity {
                agent_id: None,
                location: None,
                authenticated: false,
            })
        }
    } else {
        None
    }
}

/// Handle GET /api/v1/cache/{type}/{id} or /api/v1/cache/{type}
///
/// Thin HTTP gateway. Doorway does not interpret domain-specific response
/// fields or author access rules. Doorway:
/// 1. Routes to projection/conductor
/// 2. Limits projection responses to anonymous-open declarations
/// 3. Passes validated requester identity for conductor checks
/// 4. Translates success/error to HTTP
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

    // Request accounting happens ONCE, in the dispatcher
    // (`server::http::handle_request` → `request_category`). Counting again here
    // would double-count every /api/v1/cache request against the total.

    // Parse verified requester identity for conductor access control.
    let requester = parse_requester_identity(auth_header.as_deref(), &state);

    debug!(
        "Cache request: type={}, id={:?}, has_identity={}",
        route.doc_type,
        route.doc_id,
        requester.is_some()
    );

    // Single document lookup - use DoorwayResolver for tiered fallback
    // Doorway is type-agnostic: passes type string through to projection/conductor.
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

                // Return the admitted projection or conductor response.
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
    let limit = match params.get("limit").and_then(|v| v.parse::<i64>().ok()) {
        Some(limit) if limit < 0 => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Limit must be non-negative",
                "INVALID_LIMIT",
            );
        }
        Some(limit) => limit.min(1000),
        None => 100,
    };

    if limit == 0 {
        return json_response(b"[]".to_vec());
    }

    // Mongo applies the reach predicate before its limit. The memory-only store
    // ignores custom BSON filters, so it retains the in-process eligibility
    // check over its already bounded hot cache.
    let proj_query = ProjectionQuery {
        doc_type: Some(route.doc_type.to_string()),
        filter: Some(cacheable_reach_filter()),
        limit: projection.has_mongodb().then_some(limit),
        ..Default::default()
    };

    match projection.query(proj_query).await {
        Ok(docs) => {
            let data: Vec<_> = docs
                .iter()
                .filter(|doc| is_cacheable_reach(&doc.data))
                .take(limit as usize)
                .map(|doc| &doc.data)
                .collect();
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

/// The Mongo filter matching exactly the rows `is_cacheable_reach` serves: a scalar
/// string reach of `public` or `commons`. `$in` alone also matches an array holding
/// one of them, which `is_cacheable_reach` refuses, so the limit would count rows
/// the response then drops.
fn cacheable_reach_filter() -> bson::Document {
    bson::doc! {
        "data.reach": { "$type": "string", "$in": ["public", "commons"] }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn the_mongo_filter_requires_a_scalar_string_reach() {
        let filter = cacheable_reach_filter();
        let reach = filter.get_document("data.reach").expect("reach clause");
        assert_eq!(reach.get_str("$type").expect("type clause"), "string");
        let allowed: Vec<_> = reach
            .get_array("$in")
            .expect("in clause")
            .iter()
            .filter_map(|value| value.as_str())
            .collect();
        assert_eq!(allowed, ["public", "commons"]);
    }

    use super::*;
    use crate::config::Args;
    use crate::projection::{EngineConfig, ProjectedDocument, ProjectionEngine, ProjectionSignal};
    use clap::Parser;
    use http_body_util::BodyExt;

    fn test_state() -> Arc<AppState> {
        Arc::new(AppState::new(Args::parse_from([
            "doorway",
            "--listen",
            "127.0.0.1:0",
        ])))
    }

    #[test]
    fn test_parse_cache_route_with_id() {
        let route = CacheRoute::parse("/api/v1/cache/Content/manifesto").unwrap();
        assert_eq!(route.doc_type, "Content");
        assert_eq!(route.doc_id, Some("manifesto"));
    }

    #[test]
    fn test_parse_cache_route_collection() {
        let route = CacheRoute::parse("/api/v1/cache/Human").unwrap();
        assert_eq!(route.doc_type, "Human");
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

    #[test]
    fn caller_supplied_agent_header_is_anonymous() {
        let state = test_state();

        assert!(parse_requester_identity(None, &state).is_none());
        assert!(parse_requester_identity(Some("Agent uhCAkForged"), &state).is_none());
    }

    #[tokio::test]
    async fn intimate_signal_document_is_not_served_to_anonymous_caller() {
        let state = test_state();
        let projection = state.projection.clone().expect("test projection");
        let engine = ProjectionEngine::new(projection, EngineConfig::default());
        engine
            .process_signal(ProjectionSignal {
                doc_type: "Content".to_string(),
                action: "commit".to_string(),
                id: "intimate-signal".to_string(),
                data: serde_json::json!({
                    "title": "Intimate document",
                    "reach": "intimate"
                }),
                action_hash: "action-intimate".to_string(),
                entry_hash: None,
                author: "uhCAkAuthor".to_string(),
                search_tokens: vec![],
                invalidates: vec![],
                ttl_secs: None,
            })
            .await
            .unwrap();

        let response = handle_api_request(
            state,
            "/api/v1/cache/Content/intimate-signal",
            None,
            None,
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn collection_query_serves_only_exact_anonymous_open_reach() {
        let state = test_state();
        let projection = state.projection.clone().expect("test projection");
        for (id, reach) in [
            ("public", Some("public")),
            ("commons", Some("commons")),
            ("intimate", Some("intimate")),
            ("absent", None),
        ] {
            let data = match reach {
                Some(reach) => serde_json::json!({"id": id, "reach": reach}),
                None => serde_json::json!({"id": id}),
            };
            let doc =
                ProjectedDocument::new("Content", id, format!("action-{id}"), "uhCAkAuthor", data);
            assert!(doc.reach.is_none());
            projection.set(doc).await.unwrap();
        }

        let response = handle_api_request(state, "/api/v1/cache/Content", None, None, None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let rows: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        let mut ids: Vec<&str> = rows
            .iter()
            .filter_map(|row| row.get("id").and_then(serde_json::Value::as_str))
            .collect();
        ids.sort_unstable();
        assert_eq!(ids, vec!["commons", "public"]);
    }

    #[tokio::test]
    async fn collection_limit_is_applied_after_reach_filtering() {
        let state = test_state();
        let projection = state.projection.clone().expect("test projection");
        let mut public_doc = ProjectedDocument::new(
            "Content",
            "older-public",
            "action-public",
            "uhCAkAuthor",
            serde_json::json!({"id": "older-public", "reach": "public"}),
        );
        public_doc.projected_at = bson::DateTime::from_millis(0);
        projection.set(public_doc).await.unwrap();

        for index in 0..100 {
            projection
                .set(ProjectedDocument::new(
                    "Content",
                    format!("newer-intimate-{index}"),
                    format!("action-intimate-{index}"),
                    "uhCAkAuthor",
                    serde_json::json!({
                        "id": format!("newer-intimate-{index}"),
                        "reach": "intimate"
                    }),
                ))
                .await
                .unwrap();
        }

        let response = handle_api_request(state, "/api/v1/cache/Content", None, None, None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let rows: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            rows,
            vec![serde_json::json!({
                "id": "older-public",
                "reach": "public"
            })]
        );
    }

    #[tokio::test]
    async fn collection_query_rejects_negative_limit() {
        let state = test_state();

        let response =
            handle_api_request(state, "/api/v1/cache/Content", Some("limit=-1"), None, None).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["code"], "INVALID_LIMIT");
    }
}
