//! SSR-specific glue between elohim-render and doorway's resolver.
//!
//! ## ResolverFetcher
//!
//! `ResolverFetcher` implements `elohim_render::DataFetcher` by forwarding fetch
//! requests to the configured elohim-storage endpoint via `reqwest`. This lets
//! Angular SSR components call `fetch()` against the local storage sidecar during
//! server-side rendering without needing network access to the public doorway URL.
//!
//! The backing method is a direct `reqwest::Client` GET against `storage_base_url`
//! because `DoorwayResolver::resolve(content_type, id)` is scoped to
//! projection-cache / conductor lookups — it has no concept of arbitrary HTTP paths.
//!
//! ## Render-result cache
//!
//! `render_cache_key` produces a stable key for an SSR render result.
//!
//! MVP cache semantics: key is `(url, spec_version)` only. `fetched_inputs` is
//! recorded in `RenderOutput` for audit but not factored into the lookup key.
//! Invalidation is TTL-based (5-minute default). Hash-aware invalidation (keyed on
//! fetched-content hashes) is deferred until a DHT signal subscriber drives
//! evictions — at that point callers pass the real hash list rather than `&[]`.

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;

use elohim_render::{DataFetcher, FetchRequest, FetchResponse, RenderError, Result};

// =============================================================================
// Cache key
// =============================================================================

/// Build a stable cache key for an SSR render result.
///
/// Inputs:
/// - `url` — the requested URL (path + query)
/// - `fetched_hashes` — content hashes of every resource fetched during
///   rendering; pass `&[]` for MVP TTL-only invalidation
/// - `spec_version` — version tag for the renderer behaviour; bump when
///   the renderer's output changes (e.g. new shim added) to invalidate
///   all cached renders without touching content
///
/// MVP trade-off: the lookup key uses `&[]` so the first request is always MISS
/// and subsequent requests (within TTL) hit. Hash-aware invalidation requires
/// a DHT signal subscriber, which is a follow-up task.
pub fn render_cache_key(url: &str, fetched_hashes: &[String], spec_version: &str) -> String {
    let mut h = Sha256::new();
    h.update(url.as_bytes());
    h.update(b"\0");
    for hash in fetched_hashes {
        h.update(hash.as_bytes());
        h.update(b"\0");
    }
    h.update(spec_version.as_bytes());
    let bytes = h.finalize();
    format!("ssr-{}", hex::encode(&bytes[..16]))
}

// =============================================================================
// ResolverFetcher
// =============================================================================

/// Opaque user credential the V8 fetch shim attaches to outbound storage
/// fetches. Constructed by doorway's session layer; the shim doesn't decode
/// or interpret it. Allows framework-agnostic auth threading: any V8-based
/// renderer can pass through whatever credential shape doorway's session
/// produced (JWT, steward attestation, opaque session cookie, etc.).
#[derive(Debug, Clone)]
pub struct UserCredential {
    pub header_name: String,
    pub header_value: String,
}

/// Forwards Angular SSR `fetch()` calls to the local elohim-storage sidecar.
///
/// Constructed with a shared `reqwest::Client` (from `AppState::ssr_http_client`)
/// and the storage base URL (e.g. `http://localhost:8090`). Any `fetch()` call
/// from the Angular bundle is translated to a `reqwest::Client` request against
/// that base, stripping any scheme/host from the incoming URL first (the bundle's
/// fetch calls use relative paths or doorway-originating absolute URLs).
///
/// The client is injected rather than built per-request so the connection pool is
/// shared across all SSR renders on this doorway instance.
///
/// When `user_credential` is `Some`, the shim attaches it as an HTTP header
/// to every outbound storage fetch, so authenticated SSR renders see the
/// originating user's auth context (commons + reach-aware content).
pub struct ResolverFetcher {
    storage_base: String,
    client: Arc<reqwest::Client>,
    user_credential: Option<UserCredential>,
}

impl ResolverFetcher {
    /// Create a new `ResolverFetcher` using a shared HTTP client.
    ///
    /// `client` should be the doorway-level shared client (from `AppState`).
    /// `storage_base_url` should be the root of the elohim-storage HTTP API,
    /// e.g. `http://localhost:8090` (no trailing slash).
    pub fn new(client: Arc<reqwest::Client>, storage_base_url: String) -> Self {
        Self {
            storage_base: storage_base_url.trim_end_matches('/').to_string(),
            client,
            user_credential: None,
        }
    }

    /// Builder: attach a per-render user credential. The shim will add this
    /// as a header on every outbound storage fetch.
    pub fn with_user_credential(mut self, credential: UserCredential) -> Self {
        self.user_credential = Some(credential);
        self
    }

    /// Convenience: apply credential only if `Some`.
    pub fn maybe_with_user_credential(self, cred: Option<UserCredential>) -> Self {
        match cred {
            Some(c) => self.with_user_credential(c),
            None => self,
        }
    }
}

#[async_trait]
impl DataFetcher for ResolverFetcher {
    async fn fetch(&self, request: FetchRequest) -> Result<FetchResponse> {
        let path = strip_to_path(&request.url);
        let url = format!("{}{}", self.storage_base, path);

        let method = request.method.to_ascii_uppercase();
        let req_builder = match method.as_str() {
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            "PATCH" => self.client.patch(&url),
            _ => self.client.get(&url),
        };

        // Forward request headers
        let mut req_builder = req_builder;
        for (k, v) in &request.headers {
            req_builder = req_builder.header(k.as_str(), v.as_str());
        }

        // Attach the originating user's credential if the shim was given one.
        // The header value is opaque to the shim — doorway's session layer
        // constructed it from the live request's auth context.
        if let Some(cred) = &self.user_credential {
            req_builder = req_builder.header(&cred.header_name, &cred.header_value);
        }

        // Forward body if present
        if let Some(body) = request.body {
            req_builder = req_builder.body(body);
        }

        let resp = req_builder
            .send()
            .await
            .map_err(|e| RenderError::DataFetch(format!("storage fetch error: {e}")))?;

        let status = resp.status().as_u16();

        // Collect response headers into a flat HashMap (last value wins on dupe keys)
        let mut headers: HashMap<String, String> = HashMap::new();
        for (name, value) in resp.headers() {
            if let Ok(v) = value.to_str() {
                headers.insert(name.to_string(), v.to_string());
            }
        }

        let body = resp
            .bytes()
            .await
            .map_err(|e| RenderError::DataFetch(format!("storage read body: {e}")))?
            .to_vec();

        Ok(FetchResponse {
            status,
            headers,
            body,
            content_hash: None,
        })
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Strip scheme and host from a URL, returning just the path (and query).
///
/// `https://example.com/api/v1/foo?bar=1` → `/api/v1/foo?bar=1`
/// `/api/v1/foo` → `/api/v1/foo`
fn strip_to_path(url: &str) -> &str {
    if let Some(idx) = url.find("://") {
        let after_scheme = &url[idx + 3..];
        if let Some(slash_idx) = after_scheme.find('/') {
            return &after_scheme[slash_idx..];
        }
        return "/";
    }
    url
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_absolute_url_to_path() {
        assert_eq!(
            strip_to_path("https://example.com/api/v1/foo?bar=1"),
            "/api/v1/foo?bar=1"
        );
    }

    // ── ResolverFetcher user_credential threading ───────────────────────────

    #[tokio::test]
    async fn fetcher_forwards_user_credential_header() {
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::header("authorization", "Bearer user-token"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&server)
            .await;
        let client = Arc::new(reqwest::Client::new());
        let fetcher =
            ResolverFetcher::new(client, server.uri()).with_user_credential(UserCredential {
                header_name: "Authorization".into(),
                header_value: "Bearer user-token".into(),
            });
        let req = FetchRequest {
            url: "/api/private".into(),
            method: "GET".into(),
            headers: HashMap::new(),
            body: None,
        };
        let resp = fetcher.fetch(req).await.expect("fetch ok");
        assert_eq!(resp.status, 200);
    }

    #[tokio::test]
    async fn fetcher_omits_credential_when_none() {
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/api/public"))
            .respond_with(ResponseTemplate::new(200).set_body_string("public"))
            .mount(&server)
            .await;
        let client = Arc::new(reqwest::Client::new());
        let fetcher = ResolverFetcher::new(client, server.uri());
        let req = FetchRequest {
            url: "/api/public".into(),
            method: "GET".into(),
            headers: HashMap::new(),
            body: None,
        };
        let resp = fetcher.fetch(req).await.expect("fetch ok");
        assert_eq!(resp.status, 200);
        let received = server.received_requests().await.unwrap();
        let auth_header = received[0].headers.get("authorization");
        assert!(
            auth_header.is_none(),
            "no credential → no Authorization header on outbound fetch"
        );
    }

    #[tokio::test]
    async fn fetcher_maybe_with_user_credential_none_is_noop() {
        let client = Arc::new(reqwest::Client::new());
        let fetcher = ResolverFetcher::new(client, "http://localhost:8090".to_string())
            .maybe_with_user_credential(None);
        // Just verify it compiles and doesn't panic.
        // Behavioral coverage is in fetcher_omits_credential_when_none.
        assert_eq!(fetcher.storage_base, "http://localhost:8090");
    }

    #[test]
    fn strip_absolute_url_no_path() {
        assert_eq!(strip_to_path("https://example.com"), "/");
    }

    #[test]
    fn strip_relative_url_unchanged() {
        assert_eq!(strip_to_path("/api/v1/foo"), "/api/v1/foo");
    }

    #[test]
    fn strip_relative_url_with_query() {
        assert_eq!(strip_to_path("/api/v1/foo?x=1"), "/api/v1/foo?x=1");
    }

    // ── render_cache_key ────────────────────────────────────────────────────

    #[test]
    fn render_cache_key_same_inputs_produce_same_key() {
        let k1 = render_cache_key("/lamad/concept/abc", &[], "v1");
        let k2 = render_cache_key("/lamad/concept/abc", &[], "v1");
        assert_eq!(k1, k2, "identical inputs must produce identical keys");
    }

    #[test]
    fn render_cache_key_different_url_produces_different_key() {
        let k1 = render_cache_key("/lamad/concept/abc", &[], "v1");
        let k2 = render_cache_key("/lamad/concept/xyz", &[], "v1");
        assert_ne!(k1, k2, "different URLs must produce different keys");
    }

    #[test]
    fn render_cache_key_different_fetched_hashes_produces_different_key() {
        let k1 = render_cache_key("/lamad/concept/abc", &[], "v1");
        let k2 = render_cache_key("/lamad/concept/abc", &["sha256-deadbeef".to_string()], "v1");
        assert_ne!(
            k1, k2,
            "different fetched hashes must produce different keys"
        );
    }

    #[test]
    fn render_cache_key_different_spec_version_produces_different_key() {
        let k1 = render_cache_key("/lamad/concept/abc", &[], "v1");
        let k2 = render_cache_key("/lamad/concept/abc", &[], "v2");
        assert_ne!(
            k1, k2,
            "different spec versions must produce different keys"
        );
    }

    #[test]
    fn render_cache_key_empty_hashes_still_produces_a_key() {
        let k = render_cache_key("/", &[], "v1");
        assert!(
            k.starts_with("ssr-"),
            "key must start with 'ssr-' prefix; got: {k}"
        );
        // 32 hex chars for 16 bytes + 4 for "ssr-" prefix = 36 total
        assert_eq!(k.len(), 36, "key must be exactly 36 characters; got: {k}");
    }

    #[test]
    fn render_cache_key_hash_ordering_matters() {
        let hashes_a = vec!["hash1".to_string(), "hash2".to_string()];
        let hashes_b = vec!["hash2".to_string(), "hash1".to_string()];
        let k1 = render_cache_key("/url", &hashes_a, "v1");
        let k2 = render_cache_key("/url", &hashes_b, "v1");
        assert_ne!(
            k1, k2,
            "hash ordering must affect the key (list is ordered)"
        );
    }
}
