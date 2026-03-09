//! CORS middleware for the Doorway gateway.
//!
//! Centralises all Cross-Origin Resource Sharing logic so that individual
//! route handlers never need to touch CORS headers directly.
//!
//! In **dev mode** every origin is reflected back (permissive).
//! In **production** only origins listed in `CORS_ORIGINS` are allowed;
//! the matched origin is reflected in `Access-Control-Allow-Origin`.

use bytes::Bytes;
use http_body_util::Full;
use hyper::header::{self, HeaderValue};
use hyper::{Response, StatusCode};
use std::collections::HashSet;
use tracing::debug;

use crate::config::Args;

type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

// ── Allowed methods & headers ────────────────────────────────────────

/// Methods permitted in CORS requests.
const ALLOWED_METHODS: &str = "GET, POST, PUT, DELETE, HEAD, OPTIONS";

/// Request headers the browser may send.
const ALLOWED_HEADERS: &str = "Content-Type, Authorization, Accept, X-Op, X-Requested-With";

/// Response headers the browser may read from JavaScript.
const EXPOSE_HEADERS: &str = "Content-Length, Content-Range, ETag";

/// Preflight cache lifetime (seconds). 24 hours.
const MAX_AGE_SECS: u32 = 86400;

// ── CorsConfig ───────────────────────────────────────────────────────

/// Runtime CORS configuration built once from CLI / environment.
#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// Origins that are allowed to make cross-origin requests.
    /// Ignored when `dev_mode` is true (all origins allowed).
    allowed_origins: HashSet<String>,
    /// When true every origin is permitted (development convenience).
    dev_mode: bool,
}

impl CorsConfig {
    /// Build from parsed CLI arguments.
    pub fn from_args(args: &Args) -> Self {
        let allowed_origins: HashSet<String> = args
            .cors_origins
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Self {
            allowed_origins,
            dev_mode: args.dev_mode,
        }
    }

    /// Determine the value for `Access-Control-Allow-Origin`.
    ///
    /// Returns `None` when the origin is not permitted (the browser will
    /// block the request).
    ///
    /// | Scenario                            | Result                   |
    /// |-------------------------------------|--------------------------|
    /// | No `Origin` header                  | `Some("*")`              |
    /// | Dev mode + any origin               | `Some(<reflected>)`      |
    /// | No allowlist configured + any origin | `Some("*")`              |
    /// | Allowlist + origin in list          | `Some(<reflected>)`      |
    /// | Allowlist + origin NOT in list      | `None`                   |
    pub fn resolve_origin(&self, request_origin: Option<&str>) -> Option<String> {
        match request_origin {
            // No Origin header → same-origin or non-browser client.
            None => Some("*".to_string()),

            Some(origin) if self.dev_mode => {
                // Dev mode: reflect whatever the caller sent.
                Some(origin.to_string())
            }

            Some(_) if self.allowed_origins.is_empty() => {
                // No CORS_ORIGINS configured → permissive (previous behaviour).
                // Operators opt into restriction by setting CORS_ORIGINS explicitly.
                Some("*".to_string())
            }

            Some(origin) => {
                if self.allowed_origins.contains(origin) {
                    Some(origin.to_string())
                } else {
                    debug!(origin = %origin, "CORS: origin not in allowlist");
                    None
                }
            }
        }
    }
}

// ── Preflight ────────────────────────────────────────────────────────

/// Build a preflight (`OPTIONS`) response.
///
/// Returns **204 No Content** with the full set of CORS headers.
pub fn preflight_response(
    config: &CorsConfig,
    request_origin: Option<&str>,
) -> Response<Full<Bytes>> {
    let mut builder = Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header("Access-Control-Allow-Methods", ALLOWED_METHODS)
        .header("Access-Control-Allow-Headers", ALLOWED_HEADERS)
        .header("Access-Control-Expose-Headers", EXPOSE_HEADERS)
        .header("Access-Control-Max-Age", MAX_AGE_SECS.to_string());

    if let Some(origin) = config.resolve_origin(request_origin) {
        builder = builder.header(header::ACCESS_CONTROL_ALLOW_ORIGIN, &origin);
        if origin != "*" {
            builder = builder.header(header::VARY, "Origin");
        }
    }

    builder.body(Full::new(Bytes::new())).unwrap()
}

// ── Response post-processor ──────────────────────────────────────────

/// Append CORS headers to an already-built response.
///
/// Skips:
/// - WebSocket upgrades (101 Switching Protocols)
/// - Responses that already carry `Access-Control-Allow-Origin`
///   (safety valve — should not happen once migration is complete)
pub fn apply_cors_headers(
    config: &CorsConfig,
    request_origin: Option<&str>,
    mut response: Response<BoxBody>,
) -> Response<BoxBody> {
    // WebSocket upgrades are not subject to CORS.
    if response.status() == StatusCode::SWITCHING_PROTOCOLS {
        return response;
    }

    let headers = response.headers_mut();

    // Don't double-set if a route already added the header.
    if headers.contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN) {
        return response;
    }

    if let Some(origin) = config.resolve_origin(request_origin) {
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            HeaderValue::from_str(&origin).unwrap_or(HeaderValue::from_static("*")),
        );
        if origin != "*" {
            headers.append(header::VARY, HeaderValue::from_static("Origin"));
        }
    }

    response
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    fn dev_config() -> CorsConfig {
        CorsConfig {
            allowed_origins: HashSet::new(),
            dev_mode: true,
        }
    }

    fn prod_config(origins: &[&str]) -> CorsConfig {
        CorsConfig {
            allowed_origins: origins.iter().map(|s| s.to_string()).collect(),
            dev_mode: false,
        }
    }

    // ── resolve_origin ───────────────────────────────────────────

    #[test]
    fn dev_mode_reflects_any_origin() {
        let config = dev_config();
        assert_eq!(
            config.resolve_origin(Some("https://random-site.com")),
            Some("https://random-site.com".to_string()),
        );
    }

    #[test]
    fn production_reflects_allowed_origin() {
        let config = prod_config(&["https://elohim.host"]);
        assert_eq!(
            config.resolve_origin(Some("https://elohim.host")),
            Some("https://elohim.host".to_string()),
        );
    }

    #[test]
    fn production_rejects_unknown_origin() {
        let config = prod_config(&["https://elohim.host"]);
        assert_eq!(config.resolve_origin(Some("https://evil.com")), None);
    }

    #[test]
    fn no_origin_header_returns_wildcard() {
        let config = prod_config(&["https://elohim.host"]);
        assert_eq!(config.resolve_origin(None), Some("*".to_string()));
    }

    #[test]
    fn tauri_origin_allowed() {
        let config = prod_config(&["tauri://localhost"]);
        assert_eq!(
            config.resolve_origin(Some("tauri://localhost")),
            Some("tauri://localhost".to_string()),
        );
    }

    #[test]
    fn empty_allowlist_is_permissive_in_production() {
        // No CORS_ORIGINS set → behave like pre-middleware (wildcard).
        // Operators opt into restriction by setting CORS_ORIGINS explicitly.
        let config = prod_config(&[]);
        assert_eq!(
            config.resolve_origin(Some("https://elohim.host")),
            Some("*".to_string()),
        );
    }

    // ── preflight_response ───────────────────────────────────────

    #[test]
    fn preflight_returns_204() {
        let resp = preflight_response(&dev_config(), Some("https://elohim.host"));
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[test]
    fn preflight_has_required_headers() {
        let resp = preflight_response(&dev_config(), Some("https://elohim.host"));
        assert!(resp.headers().contains_key("access-control-allow-methods"));
        assert!(resp.headers().contains_key("access-control-allow-headers"));
        assert!(resp.headers().contains_key("access-control-max-age"));
        assert!(resp.headers().contains_key("access-control-expose-headers"));
    }

    #[test]
    fn preflight_adds_vary_for_specific_origin() {
        let config = prod_config(&["https://elohim.host"]);
        let resp = preflight_response(&config, Some("https://elohim.host"));
        let vary = resp.headers().get("vary").unwrap().to_str().unwrap();
        assert!(vary.contains("Origin"));
    }

    #[test]
    fn preflight_no_vary_for_wildcard() {
        let config = prod_config(&["https://elohim.host"]);
        let resp = preflight_response(&config, None); // No Origin → wildcard
        assert!(!resp.headers().contains_key("vary"));
    }

    #[test]
    fn preflight_omits_acao_for_rejected_origin() {
        let config = prod_config(&["https://elohim.host"]);
        let resp = preflight_response(&config, Some("https://evil.com"));
        assert!(!resp.headers().contains_key("access-control-allow-origin"));
    }

    // ── apply_cors_headers ───────────────────────────────────────

    #[test]
    fn apply_skips_websocket_upgrade() {
        let config = dev_config();
        let response: Response<BoxBody> = Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .body(
                Full::new(Bytes::new())
                    .map_err(|never| match never {})
                    .boxed(),
            )
            .unwrap();
        let result = apply_cors_headers(&config, Some("https://x.com"), response);
        assert!(!result.headers().contains_key("access-control-allow-origin"));
    }

    #[test]
    fn apply_does_not_duplicate_existing_header() {
        let config = dev_config();
        let response: Response<BoxBody> = Response::builder()
            .status(StatusCode::OK)
            .header("Access-Control-Allow-Origin", "https://existing.com")
            .body(
                Full::new(Bytes::new())
                    .map_err(|never| match never {})
                    .boxed(),
            )
            .unwrap();
        let result = apply_cors_headers(&config, Some("https://other.com"), response);
        assert_eq!(
            result.headers().get("access-control-allow-origin").unwrap(),
            "https://existing.com",
        );
    }

    #[test]
    fn apply_adds_acao_to_normal_response() {
        let config = dev_config();
        let response: Response<BoxBody> = Response::builder()
            .status(StatusCode::OK)
            .body(
                Full::new(Bytes::new())
                    .map_err(|never| match never {})
                    .boxed(),
            )
            .unwrap();
        let result = apply_cors_headers(&config, Some("https://elohim.host"), response);
        assert_eq!(
            result.headers().get("access-control-allow-origin").unwrap(),
            "https://elohim.host",
        );
    }

    #[test]
    fn apply_adds_vary_for_specific_origin() {
        let config = prod_config(&["https://elohim.host"]);
        let response: Response<BoxBody> = Response::builder()
            .status(StatusCode::OK)
            .body(
                Full::new(Bytes::new())
                    .map_err(|never| match never {})
                    .boxed(),
            )
            .unwrap();
        let result = apply_cors_headers(&config, Some("https://elohim.host"), response);
        assert!(result.headers().contains_key("vary"));
    }

    #[test]
    fn apply_no_acao_for_rejected_origin() {
        let config = prod_config(&["https://elohim.host"]);
        let response: Response<BoxBody> = Response::builder()
            .status(StatusCode::OK)
            .body(
                Full::new(Bytes::new())
                    .map_err(|never| match never {})
                    .boxed(),
            )
            .unwrap();
        let result = apply_cors_headers(&config, Some("https://evil.com"), response);
        assert!(!result.headers().contains_key("access-control-allow-origin"));
    }
}
