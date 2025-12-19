//! REST API routes for cached zome calls
//!
//! Provides HTTP GET endpoints for cacheable zome functions.
//! Route pattern: GET /api/v1/{dna_hash}/{zome}/{fn}?{args}
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

use crate::cache::{CacheKey, DefaultRules};
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

    // Get args from query string
    let args = query.unwrap_or("");

    // Look up cache rules for this function
    let rule = state.cache_rules.get_rule(route.dna_hash, route.fn_name);

    // Check if this function is cacheable
    let rule = match rule {
        Some(r) if r.cacheable => r,
        Some(_) => {
            return error_response(
                StatusCode::METHOD_NOT_ALLOWED,
                &format!("Function '{}' is not cacheable via REST API", route.fn_name),
                "NOT_CACHEABLE",
            );
        }
        None => {
            // No explicit rule - check convention
            match DefaultRules::for_function(route.fn_name) {
                Some(r) => r,
                None => {
                    return error_response(
                        StatusCode::METHOD_NOT_ALLOWED,
                        &format!(
                            "Function '{}' is not a get_*/list_* function and has no cache rules",
                            route.fn_name
                        ),
                        "NOT_CACHEABLE",
                    );
                }
            }
        }
    };

    // Create cache key
    let cache_key = route.cache_key(args);
    let storage_key = cache_key.to_storage_key();

    // Check cache first
    if let Some(entry) = state.cache.get(&storage_key) {
        let ttl = entry.remaining_ttl_secs();
        let etag = entry.etag.clone();
        return cached_response(
            entry.data,
            &etag,
            ttl,
            true, // cache hit
        );
    }

    // Cache miss - need to make zome call
    // TODO: Implement actual zome call to conductor
    // For now, return a placeholder indicating the conductor connection is needed

    // This is where we would:
    // 1. Connect to conductor app interface
    // 2. Make zome call: call_zome(dna_hash, zome, fn_name, args)
    // 3. Parse response
    // 4. Check reach_field if applicable
    // 5. Cache if public
    // 6. Return response

    error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        &format!(
            "Zome call bridge not yet implemented. Route: {}/{}/{}",
            route.dna_hash, route.zome, route.fn_name
        ),
        "NOT_IMPLEMENTED",
    )
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
