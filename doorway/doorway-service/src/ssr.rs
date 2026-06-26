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
// Test-only fault injection (env-gated)
// =============================================================================

/// Wrap `inner` with stall fault injection when `DOORWAY_SSR_FAULT_STALL_PATH`
/// is set (off by default — zero production impact). Any outbound SSR fetch whose
/// URL contains the configured substring then never settles, exercising the
/// render-trace `stalled` terminal + fast soft-deadline fallback end-to-end.
///
/// The match is on the *fetch* URL (which carries the content slug), because the
/// renderer's fetcher is fixed at construction and sees every render's fetches —
/// so a single shared fetcher can fault just the configured route. Used by the
/// `render-trace-distinguishes-stall-from-empty` a2o scenario.
pub fn maybe_inject_stall_fault(inner: Arc<dyn DataFetcher>) -> Arc<dyn DataFetcher> {
    match std::env::var("DOORWAY_SSR_FAULT_STALL_PATH") {
        Ok(substr) if !substr.is_empty() => {
            tracing::warn!(
                target: "doorway::ssr",
                fault = %substr,
                "SSR stall fault injection ENABLED (DOORWAY_SSR_FAULT_STALL_PATH — test-only)"
            );
            Arc::new(StallFaultFetcher {
                inner,
                fault_substr: substr,
            })
        }
        _ => inner,
    }
}

/// Decorator that stalls fetches whose URL contains `fault_substr`, delegating
/// everything else to `inner`. See [`maybe_inject_stall_fault`].
struct StallFaultFetcher {
    inner: Arc<dyn DataFetcher>,
    fault_substr: String,
}

#[async_trait]
impl DataFetcher for StallFaultFetcher {
    async fn fetch(&self, request: FetchRequest) -> Result<FetchResponse> {
        if request.url.contains(self.fault_substr.as_str()) {
            // Never settle — the TracingFetcher's per-fetch soft-deadline converts
            // this into a recorded `stalled` and a fast fallback.
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            return Err(RenderError::DataFetch("ssr stall fault (test-only)".into()));
        }
        self.inner.fetch(request).await
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
// DoorwayBundleSource — blocking BundleSource over the storage HTTP API
// =============================================================================

/// Implements `elohim_render::BundleSource` by hitting the storage HTTP API
/// synchronously (blocking reqwest).
///
/// ## Async-caller safety
///
/// `init_renderer()` is called from FOUR `AppState` constructors. Three are plain
/// sync fns — fine. The fourth, `AppState::with_projection`, is an **async fn**.
/// `reqwest::blocking` internally creates a new tokio runtime and calls
/// `block_on`; doing that while already inside a tokio worker thread panics with
/// "Cannot start a runtime from within a runtime."
///
/// To handle both callers safely each blocking HTTP call is dispatched to a
/// **dedicated OS thread** via `std::thread::spawn(...).join()`. The spawned
/// thread has no ambient tokio context, so `reqwest::blocking` can create its
/// own runtime freely. The brief join is acceptable for a one-shot boot
/// materialize (this runs at most once per doorway startup, not per request).
pub struct DoorwayBundleSource {
    storage_base: String,
}

impl DoorwayBundleSource {
    pub fn new(storage_base_url: String) -> Self {
        Self {
            storage_base: storage_base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Build a fresh blocking client. Each call site spawns its own thread, so
    /// sharing a client across threads is not needed — a fresh per-call client
    /// keeps the implementation simple and avoids `Arc` across the spawn boundary.
    fn make_client() -> reqwest::blocking::Client {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default()
    }
}

impl DoorwayBundleSource {
    /// Shared thread-isolated `GET /db/content/{slug}` returning the raw body.
    ///
    /// Both `resolve_blob_hash` and `resolve_server_blob_hash` hit the same
    /// endpoint and differ only in which JSON field they parse, so the blocking
    /// HTTP call (and its dedicated-OS-thread isolation) lives here once.
    fn fetch_content_body(&self, slug: &str) -> elohim_render::Result<String> {
        let url = format!("{}/db/content/{}", self.storage_base, slug);
        // Run on a dedicated OS thread so `reqwest::blocking` can create its own
        // tokio runtime without conflicting with an ambient tokio context (the
        // `with_projection` async constructor calls `init_renderer`).
        std::thread::spawn(move || {
            let client = DoorwayBundleSource::make_client();
            let resp = client
                .get(&url)
                .send()
                .map_err(|e| RenderError::Bootstrap(format!("content GET failed: {e}")))?;
            let status = resp.status();
            resp.error_for_status()
                .map_err(|_| {
                    RenderError::Bootstrap(format!("content GET: HTTP {status} for slug `{url}`"))
                })?
                .text()
                .map_err(|e| RenderError::Bootstrap(format!("content GET read body: {e}")))
        })
        .join()
        .map_err(|_| RenderError::Bootstrap("content GET: thread panicked".into()))?
    }
}

impl elohim_render::BundleSource for DoorwayBundleSource {
    fn resolve_blob_hash(&self, slug: &str) -> elohim_render::Result<String> {
        let body = self.fetch_content_body(slug)?;
        parse_blob_hash(&body)
    }

    fn resolve_server_blob_hash(&self, slug: &str) -> elohim_render::Result<String> {
        let body = self.fetch_content_body(slug)?;
        parse_server_blob_hash(&body)
    }

    fn fetch_blob(&self, hash: &str) -> elohim_render::Result<Vec<u8>> {
        let url = format!("{}/blob/{}", self.storage_base, hash);
        // Same thread-isolation rationale as resolve_blob_hash above.
        std::thread::spawn(move || {
            let client = DoorwayBundleSource::make_client();
            let resp = client
                .get(&url)
                .send()
                .map_err(|e| RenderError::Bootstrap(format!("fetch_blob GET failed: {e}")))?;
            let status = resp.status();
            resp.error_for_status()
                .map_err(|_| {
                    RenderError::Bootstrap(format!("fetch_blob: HTTP {status} for hash `{url}`"))
                })?
                .bytes()
                .map_err(|e| RenderError::Bootstrap(format!("fetch_blob read bytes: {e}")))
                .map(|b| b.to_vec())
        })
        .join()
        .map_err(|_| RenderError::Bootstrap("fetch_blob: thread panicked".into()))?
    }
}

/// Parse the `blobHash` field from a JSON body returned by `GET /db/content/{slug}`.
///
/// The storage boundary is camelCase, so the field is `blobHash`. This is the
/// **browser** bundle pointer.
pub fn parse_blob_hash(body: &str) -> elohim_render::Result<String> {
    let v: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| RenderError::Bootstrap(format!("parse_blob_hash: invalid JSON: {e}")))?;
    v.get("blobHash")
        .and_then(|h| h.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| RenderError::Bootstrap("parse_blob_hash: missing `blobHash` field".into()))
}

/// Parse the `serverBlobHash` field from a JSON body returned by
/// `GET /db/content/{slug}`.
///
/// The storage boundary is camelCase, so the field is `serverBlobHash`. This is
/// the **server** bundle pointer — the deploy-time hash of the Angular SSR
/// *server* bundle on the one EPR node. The SSR runtime materializes this, not
/// `blobHash`. Absence is an honest, non-fatal condition (the caller treats it
/// as "SSR not materializable → CSR fallback"), so a missing field is a clean
/// `Err`, never a panic.
pub fn parse_server_blob_hash(body: &str) -> elohim_render::Result<String> {
    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| {
        RenderError::Bootstrap(format!("parse_server_blob_hash: invalid JSON: {e}"))
    })?;
    v.get("serverBlobHash")
        .and_then(|h| h.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            RenderError::Bootstrap("parse_server_blob_hash: missing `serverBlobHash` field".into())
        })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_render::BundleSource;

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

    // ── parse_blob_hash ─────────────────────────────────────────────────────

    #[test]
    fn parse_blob_hash_valid_returns_hash() {
        let body = r#"{"blobHash":"sha256-abc"}"#;
        let result = parse_blob_hash(body);
        assert_eq!(result.unwrap(), "sha256-abc");
    }

    #[test]
    fn parse_blob_hash_missing_field_returns_err() {
        let body = r#"{"someOtherField":"value"}"#;
        let result = parse_blob_hash(body);
        assert!(result.is_err(), "expected Err on missing blobHash field");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("blobHash"),
            "error should mention missing field"
        );
    }

    #[test]
    fn parse_blob_hash_invalid_json_returns_err() {
        let body = "not json at all";
        let result = parse_blob_hash(body);
        assert!(result.is_err(), "expected Err on invalid JSON");
    }

    #[test]
    fn parse_blob_hash_blob_hash_not_string_returns_err() {
        let body = r#"{"blobHash":42}"#;
        let result = parse_blob_hash(body);
        assert!(
            result.is_err(),
            "expected Err when blobHash is not a string"
        );
    }

    // ── parse_server_blob_hash ──────────────────────────────────────────────

    #[test]
    fn parse_server_blob_hash_returns_server_not_browser() {
        // A body carrying BOTH pointers — must select the SERVER one. This is
        // the exact shape `GET /db/content/elohim-host-landing` returns after
        // the row collapse (one EPR node, both hashes).
        let body = r#"{"blobHash":"sha256-browser","serverBlobHash":"sha256-server"}"#;
        let result = parse_server_blob_hash(body);
        assert_eq!(
            result.unwrap(),
            "sha256-server",
            "must return serverBlobHash, not blobHash"
        );
    }

    #[test]
    fn parse_server_blob_hash_missing_field_returns_err() {
        // `blobHash` present but `serverBlobHash` absent (the migration-safety
        // shape: SSR simply isn't materializable yet → caller does CSR fallback).
        let body = r#"{"blobHash":"sha256-browser"}"#;
        let result = parse_server_blob_hash(body);
        assert!(
            result.is_err(),
            "expected Err when serverBlobHash field is absent"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("serverBlobHash"),
            "error should mention the missing field; got: {msg}"
        );
    }

    #[test]
    fn parse_server_blob_hash_invalid_json_returns_err() {
        let result = parse_server_blob_hash("not json at all");
        assert!(result.is_err(), "expected Err on invalid JSON");
    }

    #[test]
    fn parse_server_blob_hash_not_string_returns_err() {
        let body = r#"{"serverBlobHash":42}"#;
        let result = parse_server_blob_hash(body);
        assert!(
            result.is_err(),
            "expected Err when serverBlobHash is not a string"
        );
    }

    #[tokio::test]
    async fn bundle_source_resolve_server_blob_hash_ok_from_async_context() {
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/db/content/elohim-host-landing"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"{"blobHash":"sha256-browser","serverBlobHash":"sha256-server"}"#,
            ))
            .mount(&server)
            .await;

        let src = DoorwayBundleSource::new(server.uri());
        // Thread-isolation regression: must not panic from within a tokio runtime.
        let result = src.resolve_server_blob_hash("elohim-host-landing");
        assert_eq!(
            result.expect("resolve_server_blob_hash must succeed from async context"),
            "sha256-server"
        );
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

    // ── DoorwayBundleSource async-caller regression ─────────────────────────
    //
    // These tests run from WITHIN a tokio runtime (via #[tokio::test]) to prove
    // that `DoorwayBundleSource::resolve_blob_hash` does NOT panic with "Cannot
    // start a runtime from within a runtime." Before the thread-isolation fix the
    // blocking reqwest call would create its own tokio runtime on the tokio
    // worker thread — a nested-runtime panic.  After the fix the blocking call
    // runs on a dedicated OS thread with no ambient tokio context.

    #[tokio::test]
    async fn bundle_source_resolve_blob_hash_ok_from_async_context() {
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

        // Start a wiremock server (runs its own tokio task — fine; we just call
        // the blocking DoorwayBundleSource from within this async test).
        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/db/content/my-bundle-slug"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(r#"{"blobHash":"sha256-deadbeef"}"#),
            )
            .mount(&server)
            .await;

        let src = DoorwayBundleSource::new(server.uri());
        // This call PANICS before the thread-isolation fix; it must return Ok after.
        let result = src.resolve_blob_hash("my-bundle-slug");
        assert_eq!(
            result.expect("resolve_blob_hash must succeed from async context"),
            "sha256-deadbeef"
        );
    }

    #[tokio::test]
    async fn bundle_source_resolve_blob_hash_404_returns_http_status_err() {
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/db/content/missing-slug"))
            .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
            .mount(&server)
            .await;

        let src = DoorwayBundleSource::new(server.uri());
        let err = src
            .resolve_blob_hash("missing-slug")
            .expect_err("404 must produce an Err");
        let msg = err.to_string();
        // The error must mention the HTTP status so callers get a useful message
        // instead of a misleading "missing blobHash field".
        assert!(
            msg.contains("404"),
            "error message must mention the HTTP status; got: {msg}"
        );
    }
}
