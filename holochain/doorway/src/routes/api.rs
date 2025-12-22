//! REST API routes for cached zome calls
//!
//! Provides HTTP GET endpoints for cacheable zome functions.
//! Route pattern: GET /api/v1/{dna_hash}/{zome}/{fn}?{args}
//!
//! ## Read Path Modes
//!
//! Doorway supports two read path modes:
//!
//! 1. **Projection Mode** (production default):
//!    - Reads come from ProjectionStore (hot cache + MongoDB)
//!    - Conductor is never touched for reads
//!    - Data is populated via post_commit signals from DNA
//!
//! 2. **Conductor Mode** (dev/seeding):
//!    - Direct conductor calls for reads (DEV_MODE + CONDUCTOR_READS)
//!    - Used during database seeding and development
//!    - Falls back to projection if conductor call fails
//!
//! ## Caching Behavior
//!
//! DNAs can optionally implement `__doorway_cache_rules` to declare caching rules.
//! If not implemented, Doorway uses convention-based defaults:
//! - `get_*` and `list_*` functions are cached (5 min TTL, auth required)
//! - Other functions are not cached via REST API
//!
//! ## Public Access
//!
//! For a response to be publicly accessible (no auth), either:
//! - The cache rule has `public: true`, or
//! - The cache rule specifies a `reach_field` and the response contains that field
//!   with the required `reach_value` (e.g., `reach: "commons"`)

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use serde::Serialize;
use std::net::IpAddr;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::cache::{CacheKey, extract_reach_from_response, should_serve_response, extract_requester_context};
use crate::projection::ProjectionQuery;
use crate::server::AppState;

/// API error response
#[derive(Debug, Serialize)]
struct ApiError {
    error: String,
    code: &'static str,
}

/// Parsed API route components
#[derive(Debug)]
struct ApiRoute<'a> {
    dna_hash: &'a str,
    zome: &'a str,
    fn_name: &'a str,
}

impl<'a> ApiRoute<'a> {
    /// Parse route from path like "/api/v1/{dna}/{zome}/{fn}"
    fn parse(path: &'a str) -> Option<Self> {
        let stripped = path.strip_prefix("/api/v1/")?;
        let parts: Vec<&str> = stripped.splitn(3, '/').collect();

        if parts.len() != 3 {
            return None;
        }

        Some(Self {
            dna_hash: parts[0],
            zome: parts[1],
            fn_name: parts[2],
        })
    }

    /// Create cache key for this route
    fn cache_key(&self, args: &str) -> CacheKey {
        CacheKey::new(self.dna_hash, self.zome, self.fn_name, args)
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

/// Build a cached response with appropriate headers
fn cached_response(
    data: Vec<u8>,
    etag: &str,
    ttl_secs: u64,
    cache_hit: bool,
) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Cache-Control", format!("public, max-age={}, stale-while-revalidate=60", ttl_secs))
        .header("ETag", etag)
        .header("X-Cache", if cache_hit { "HIT" } else { "MISS" })
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(data)))
        .unwrap()
}

/// Handle GET /api/v1/{dna}/{zome}/{fn}
///
/// Query parameters:
/// - `_conductor=true` - Bypass projection cache, read directly from conductor (DEV_MODE only)
/// - Other params are passed as zome function arguments
///
/// Reach-aware serving gates content access by reach level:
/// - private: Only the beneficiary (content owner)
/// - local/neighborhood/municipal/bioregional/regional: Authenticated users
/// - commons: Everyone (public)
pub async fn handle_api_request(
    state: Arc<AppState>,
    path: &str,
    query: Option<&str>,
    remote_addr: Option<IpAddr>,
    auth_header: Option<String>,
) -> Response<Full<Bytes>> {
    // Parse route
    let route = match ApiRoute::parse(path) {
        Some(r) => r,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Invalid API route. Expected: /api/v1/{dna}/{zome}/{fn}",
                "INVALID_ROUTE",
            );
        }
    };

    // Get args from query string and check for conductor bypass
    let (args, conductor_bypass) = parse_query_args(query.unwrap_or(""));

    // Check if conductor bypass is requested and allowed
    let use_conductor = conductor_bypass && state.args.dev_mode;
    if conductor_bypass && !state.args.dev_mode {
        warn!("Conductor bypass requested but DEV_MODE is disabled");
    }

    // Extract requester context from auth header and IP
    let requester = extract_requester_context(auth_header.as_deref(), remote_addr);

    // Placeholder for beneficiary_id - would come from content metadata or request context
    // For now, we'll check reach-based access in the should_serve_response call
    let beneficiary_id = "unknown";

    // Create cache key (without the _conductor param)
    let cache_key = route.cache_key(&args);
    let storage_key = cache_key.to_storage_key();

    // =========================================================================
    // Read Path: Direct Conductor (per-request bypass, DEV_MODE only)
    // =========================================================================
    if use_conductor {
        debug!("Conductor bypass requested for {}", route.fn_name);

        // Direct conductor call would go here
        // For now, indicate that conductor reads are requested but bridge not implemented
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!(
                "Direct conductor read requested but bridge not yet implemented. \
                Route: {}/{}/{}. Use WebSocket for now.",
                route.dna_hash, route.zome, route.fn_name
            ),
            "CONDUCTOR_BRIDGE_PENDING",
        );
    }

    // =========================================================================
    // Read Path: Projection Cache (default for all requests)
    // =========================================================================
    debug!("Using projection read path for {}", route.fn_name);

    // Try legacy cache first (for backward compat during transition)
    if let Some(entry) = state.cache.get(&storage_key) {
        // Check if requester can access this reach level
        if should_serve_response(&entry.data, &requester, beneficiary_id) {
            let ttl = entry.remaining_ttl_secs();
            let etag = entry.etag.clone();
            let reach = entry.reach.clone();
            return reach_aware_cached_response(entry.data, &etag, ttl, true, reach);
        } else {
            debug!("Requester denied access to cached content due to reach level");
            return error_response(
                StatusCode::FORBIDDEN,
                "Content exists but is not accessible to you",
                "REACH_DENIED",
            );
        }
    }

    // Try projection store
    if let Some(ref projection) = state.projection {
        // Map zome function to projection query
        if let Some(response) = query_projection(&route, &args, projection.as_ref()).await {
            // Check if requester can access this reach level
            if !should_serve_response(&response, &requester, beneficiary_id) {
                debug!("Requester denied access to projection content due to reach level");
                return error_response(
                    StatusCode::FORBIDDEN,
                    "Content exists but is not accessible to you",
                    "REACH_DENIED",
                );
            }

            // Cache the response with reach-aware key if reach is present
            let reach_aware_key = if let Some(reach) = extract_reach_from_response(&response) {
                let mut key = cache_key.clone();
                key.reach = Some(reach.clone());
                key
            } else {
                cache_key.clone()
            };

            return reach_aware_projection_response(response, reach_aware_key.reach);
        }
    }

    // Projection cache miss
    error_response(
        StatusCode::NOT_FOUND,
        &format!(
            "Content not found in projection cache. It may not have been created yet. \
            Route: {}/{}/{}{}",
            route.dna_hash, route.zome, route.fn_name,
            if state.args.dev_mode { ". Add ?_conductor=true to bypass cache." } else { "" }
        ),
        "NOT_IN_PROJECTION",
    )
}

/// Parse query string, extracting `_conductor` flag and returning clean args
fn parse_query_args(query: &str) -> (String, bool) {
    if query.is_empty() {
        return (String::new(), false);
    }

    let mut conductor_bypass = false;
    let mut clean_params = Vec::new();

    for pair in query.split('&') {
        if pair == "_conductor=true" || pair == "_conductor=1" {
            conductor_bypass = true;
        } else if !pair.starts_with("_conductor=") {
            clean_params.push(pair);
        }
    }

    (clean_params.join("&"), conductor_bypass)
}

/// Query projection store based on zome function
async fn query_projection(
    route: &ApiRoute<'_>,
    args: &str,
    projection: &crate::projection::ProjectionStore,
) -> Option<Vec<u8>> {
    // Parse args to extract query parameters
    let args_json: serde_json::Value = serde_json::from_str(args).unwrap_or_default();

    // Map common zome functions to projection queries
    match route.fn_name {
        // Content queries
        "get_content" | "get_content_by_id" => {
            let id = args_json.get("id")
                .or_else(|| args_json.get("content_id"))
                .and_then(|v| v.as_str())?;

            let doc = projection.get("Content", id).await?;
            serde_json::to_vec(&doc.data).ok()
        }

        "get_content_by_type" => {
            let content_type = args_json.get("content_type")
                .and_then(|v| v.as_str())?;

            let query = ProjectionQuery::by_type("Content")
                .with_limit(100);
            // Note: We'd need to add filtering by content_type in the data
            // For now, return all content and filter client-side
            let docs = projection.query(query).await.ok()?;
            let data: Vec<_> = docs.iter()
                .filter(|d| d.data.get("content_type").and_then(|v| v.as_str()) == Some(content_type))
                .map(|d| &d.data)
                .collect();
            serde_json::to_vec(&data).ok()
        }

        // Path queries
        "get_all_paths" => {
            let query = ProjectionQuery::by_type("LearningPath")
                .with_limit(100);
            let docs = projection.query(query).await.ok()?;
            let data: Vec<_> = docs.iter().map(|d| &d.data).collect();
            serde_json::to_vec(&data).ok()
        }

        "get_path_overview" | "get_path_with_steps" | "get_path_full" => {
            let id = args_json.get("id")
                .or_else(|| args_json.get("path_id"))
                .and_then(|v| v.as_str())?;

            let doc = projection.get("LearningPath", id).await?;
            serde_json::to_vec(&doc.data).ok()
        }

        // Relationship queries
        "get_relationships" => {
            let source_id = args_json.get("source_id")
                .and_then(|v| v.as_str());

            let query = ProjectionQuery::by_type("Relationship")
                .with_limit(100);
            let docs = projection.query(query).await.ok()?;

            let data: Vec<_> = if let Some(src) = source_id {
                docs.iter()
                    .filter(|d| d.data.get("source_id").and_then(|v| v.as_str()) == Some(src))
                    .map(|d| &d.data)
                    .collect()
            } else {
                docs.iter().map(|d| &d.data).collect()
            };
            serde_json::to_vec(&data).ok()
        }

        // Default: function not mapped to projection
        _ => {
            warn!("Zome function '{}' not mapped to projection query", route.fn_name);
            None
        }
    }
}

/// Build reach-aware cached response with reach level header
fn reach_aware_cached_response(
    data: Vec<u8>,
    etag: &str,
    ttl_secs: u64,
    cache_hit: bool,
    reach: Option<String>,
) -> Response<Full<Bytes>> {
    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Cache-Control", format!("public, max-age={}, stale-while-revalidate=60", ttl_secs))
        .header("ETag", etag)
        .header("X-Cache", if cache_hit { "HIT" } else { "MISS" });

    if let Some(r) = reach {
        response = response.header("X-Reach", r);
    }

    response
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(data)))
        .unwrap()
}

/// Build response from projection data with reach awareness
fn reach_aware_projection_response(data: Vec<u8>, reach: Option<String>) -> Response<Full<Bytes>> {
    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("X-Source", "projection")
        .header("Cache-Control", "public, max-age=60");

    if let Some(r) = reach {
        response = response.header("X-Reach", r);
    }

    response
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(data)))
        .unwrap()
}

/// Build response from projection data
fn projection_response(data: Vec<u8>) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("X-Source", "projection")
        .header("Cache-Control", "public, max-age=60")
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(data)))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::RequesterContext;

    #[test]
    fn test_parse_route() {
        let route = ApiRoute::parse("/api/v1/uhC0k123abc/content_store/get_content").unwrap();
        assert_eq!(route.dna_hash, "uhC0k123abc");
        assert_eq!(route.zome, "content_store");
        assert_eq!(route.fn_name, "get_content");
    }

    #[test]
    fn test_parse_route_invalid() {
        assert!(ApiRoute::parse("/api/v1/dna/zome").is_none()); // Missing fn
        assert!(ApiRoute::parse("/api/v1/dna").is_none()); // Missing zome and fn
        assert!(ApiRoute::parse("/other/path").is_none()); // Wrong prefix
    }

    #[test]
    fn test_cache_key_generation() {
        let route = ApiRoute::parse("/api/v1/dna123/zome/get_thing").unwrap();
        let key = route.cache_key(r#"{"id":"abc"}"#);

        assert_eq!(key.dna_hash, "dna123");
        assert_eq!(key.zome, "zome");
        assert_eq!(key.fn_name, "get_thing");
        assert!(!key.args_hash.is_empty());
    }

    #[test]
    fn test_error_response() {
        let resp = error_response(StatusCode::NOT_FOUND, "Test error", "TEST_ERROR");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // ========================================================================
    // Reach-Aware Serving Tests
    // ========================================================================

    #[test]
    fn test_reach_aware_cached_response_with_reach() {
        let data = b"test content".to_vec();
        let etag = "\"abc123\"";
        let ttl_secs = 300;
        let reach = Some("commons".to_string());

        let resp = reach_aware_cached_response(data.clone(), etag, ttl_secs, true, reach);

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.headers().contains_key("X-Cache"));
        assert!(resp.headers().contains_key("X-Reach"));
    }

    #[test]
    fn test_reach_aware_cached_response_without_reach() {
        let data = b"test content".to_vec();
        let etag = "\"abc123\"";
        let ttl_secs = 300;

        let resp = reach_aware_cached_response(data, etag, ttl_secs, false, None);

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(!resp.headers().contains_key("X-Reach"));
    }

    #[test]
    fn test_reach_aware_projection_response_with_reach() {
        let data = br#"{"id":"test","reach":"local"}"#.to_vec();
        let reach = Some("local".to_string());

        let resp = reach_aware_projection_response(data, reach);

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get("X-Reach")
                .and_then(|h| h.to_str().ok()),
            Some("local")
        );
    }

    #[test]
    fn test_extract_requester_context_authenticated() {
        let auth_header = "Bearer alice-pubkey-abc123";
        let requester = extract_requester_context(Some(auth_header), None);

        assert_eq!(requester.agent_id, "alice-pubkey-abc123");
        assert!(requester.authenticated);
    }

    #[test]
    fn test_extract_requester_context_unauthenticated() {
        let requester = extract_requester_context(None, None);

        assert_eq!(requester.agent_id, "anonymous");
        assert!(!requester.authenticated);
    }

    #[test]
    fn test_should_serve_commons_unauthenticated() {
        let response = br#"{"id":"content1","reach":"commons","title":"Public"}"#.to_vec();
        let requester = RequesterContext {
            agent_id: "stranger".to_string(),
            location: None,
            authenticated: false,
        };

        assert!(should_serve_response(&response, &requester, "alice"));
    }

    #[test]
    fn test_should_not_serve_private_unauthenticated() {
        let response = br#"{"id":"content1","reach":"private","title":"Secret"}"#.to_vec();
        let requester = RequesterContext {
            agent_id: "bob".to_string(),
            location: None,
            authenticated: false,
        };

        assert!(!should_serve_response(&response, &requester, "alice"));
    }

    #[test]
    fn test_should_serve_private_to_owner() {
        let response = br#"{"id":"content1","reach":"private","title":"Secret"}"#.to_vec();
        let requester = RequesterContext {
            agent_id: "alice".to_string(),
            location: None,
            authenticated: true,
        };

        assert!(should_serve_response(&response, &requester, "alice"));
    }

    #[test]
    fn test_should_not_serve_private_to_other_authenticated() {
        let response = br#"{"id":"content1","reach":"private","title":"Secret"}"#.to_vec();
        let requester = RequesterContext {
            agent_id: "bob".to_string(),
            location: None,
            authenticated: true,
        };

        assert!(!should_serve_response(&response, &requester, "alice"));
    }

    #[test]
    fn test_should_serve_neighborhood_to_authenticated() {
        let response =
            br#"{"id":"content1","reach":"neighborhood","title":"Local"}"#.to_vec();
        let requester = RequesterContext {
            agent_id: "bob".to_string(),
            location: Some("37.7749,-122.4194".to_string()),
            authenticated: true,
        };

        assert!(should_serve_response(&response, &requester, "alice"));
    }

    #[test]
    fn test_should_not_serve_neighborhood_to_unauthenticated() {
        let response =
            br#"{"id":"content1","reach":"neighborhood","title":"Local"}"#.to_vec();
        let requester = RequesterContext {
            agent_id: "stranger".to_string(),
            location: None,
            authenticated: false,
        };

        assert!(!should_serve_response(&response, &requester, "alice"));
    }

    #[test]
    fn test_extract_reach_from_response_commons() {
        use serde_json::json;

        let response = json!({
            "id": "test-content",
            "title": "Public Content",
            "reach": "commons"
        });
        let bytes = serde_json::to_vec(&response).unwrap();

        let reach = extract_reach_from_response(&bytes);
        assert_eq!(reach, Some("commons".to_string()));
    }

    #[test]
    fn test_extract_reach_from_response_private() {
        use serde_json::json;

        let response = json!({
            "id": "test-content",
            "title": "Private Content",
            "reach": "private"
        });
        let bytes = serde_json::to_vec(&response).unwrap();

        let reach = extract_reach_from_response(&bytes);
        assert_eq!(reach, Some("private".to_string()));
    }

    #[test]
    fn test_extract_reach_from_array_response() {
        use serde_json::json;

        let response = json!([
            {"id": "item1", "reach": "commons"},
            {"id": "item2", "reach": "local"}
        ]);
        let bytes = serde_json::to_vec(&response).unwrap();

        let reach = extract_reach_from_response(&bytes);
        assert_eq!(reach, Some("commons".to_string())); // Should get from first item
    }

    #[test]
    fn test_extract_reach_missing() {
        use serde_json::json;

        let response = json!({
            "id": "test-content",
            "title": "No Reach Field"
        });
        let bytes = serde_json::to_vec(&response).unwrap();

        let reach = extract_reach_from_response(&bytes);
        assert_eq!(reach, None);
    }

    #[test]
    fn test_parse_query_args_with_conductor_flag() {
        let (args, conductor) = parse_query_args("id=abc&_conductor=true&name=test");
        assert!(conductor);
        assert!(!args.contains("_conductor"));
        assert!(args.contains("id=abc"));
        assert!(args.contains("name=test"));
    }

    #[test]
    fn test_parse_query_args_without_conductor_flag() {
        let (args, conductor) = parse_query_args("id=abc&name=test");
        assert!(!conductor);
        assert_eq!(args, "id=abc&name=test");
    }

    #[test]
    fn test_parse_query_args_empty() {
        let (args, conductor) = parse_query_args("");
        assert!(!conductor);
        assert_eq!(args, "");
    }
}
