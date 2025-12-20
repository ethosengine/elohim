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
use std::sync::Arc;
use tracing::{debug, warn};

use crate::cache::{CacheKey, DefaultRules};
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
        .body(Full::new(Bytes::from(data)))
        .unwrap()
}

/// Handle GET /api/v1/{dna}/{zome}/{fn}
///
/// Query parameters:
/// - `_conductor=true` - Bypass projection cache, read directly from conductor (DEV_MODE only)
/// - Other params are passed as zome function arguments
pub async fn handle_api_request(
    state: Arc<AppState>,
    path: &str,
    query: Option<&str>,
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
        let ttl = entry.remaining_ttl_secs();
        let etag = entry.etag.clone();
        return cached_response(entry.data, &etag, ttl, true);
    }

    // Try projection store
    if let Some(ref projection) = state.projection {
        // Map zome function to projection query
        if let Some(response) = query_projection(&route, &args, projection.as_ref()).await {
            return projection_response(response);
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

/// Build response from projection data
fn projection_response(data: Vec<u8>) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("X-Source", "projection")
        .header("Cache-Control", "public, max-age=60")
        .body(Full::new(Bytes::from(data)))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
