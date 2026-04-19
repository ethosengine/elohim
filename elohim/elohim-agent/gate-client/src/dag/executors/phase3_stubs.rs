//! Pull resolvers for `ContextAssembleExecutor`.
//!
//! Two resolvers are real (HTTP-based, honest-degradation):
//! - [`ElohimStorageResolver`] — HTTP GET to an elohim-storage endpoint.
//! - [`ManifestResolver`] — HTTP GET to the elohim-storage manifest endpoint.
//!
//! Two resolvers remain stubs pending deeper host-binding work:
//! - [`SourceChainResolver`] — Phase 10+ (requires conductor-bridge or HDK integration).
//! - [`DhtResolver`] — Phase 10+ (requires conductor-bridge for DHT queries).
//!
//! # Honest degradation
//!
//! All four resolvers follow the same contract: **resolver failures return
//! `Value::Null`, they do not fail the gate**. This mirrors the
//! `WisdomInvokeExecutor`'s pattern — if a resolver is unavailable, the gate
//! still completes; downstream rules interpret null data as "no prior evidence"
//! or "unknown", which is the correct honest-degraded posture.

use async_trait::async_trait;
use serde_json::Value;
use tracing::{debug, warn};

use crate::error::GateError;

use super::context_assemble::PullResolver;

// ─── ElohimStorageResolver (real, HTTP-based) ─────────────────────────────────

/// Pull resolver that issues HTTP GET requests to an elohim-storage instance.
///
/// # Query convention
///
/// The `query` parameter is treated as a URL path fragment appended to
/// `base_url`. Examples:
/// - `/db/content/{cid}` → full content record from the projection table
/// - `/db/gate-decisions/{cid}` → prior gate decisions for a content CID
/// - `/api/v1/cache/{type}/{id}` → cached operational records
///
/// These paths correspond to real elohim-storage endpoints from Phase 4 Task 4.3.
///
/// # Honest degradation
///
/// | HTTP outcome          | Resolver response                                  |
/// |-----------------------|----------------------------------------------------|
/// | 200 with valid JSON   | `Ok(parsed_value)`                                 |
/// | 200 with bad JSON     | `Err(GateError::ContextAssembly)` — caller logs    |
/// | 404                   | `Ok(Value::Null)` with `tracing::debug`            |
/// | Other 4xx / 5xx       | `Ok(Value::Null)` with `tracing::warn`             |
/// | Network / TLS error   | `Ok(Value::Null)` with `tracing::warn`             |
///
/// The gate **never fails** due to an elohim-storage outage; it degrades to
/// reasoning with null context.
pub struct ElohimStorageResolver {
    base_url: String,
    http_client: reqwest::Client,
}

impl ElohimStorageResolver {
    /// Construct a resolver pointing at `base_url`.
    ///
    /// `base_url` should be a bare origin, e.g. `"http://localhost:8090"`.
    /// A trailing slash is stripped; the `query` parameter provides the path.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http_client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PullResolver for ElohimStorageResolver {
    async fn resolve(&self, query: &str) -> Result<Value, GateError> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), query);

        match self.http_client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                response.json::<Value>().await.map_err(|e| {
                    GateError::ContextAssembly(format!(
                        "elohim-storage resolver: JSON parse error for {url}: {e}"
                    ))
                })
            }
            Ok(response) if response.status() == reqwest::StatusCode::NOT_FOUND => {
                debug!(
                    pull.source = "elohim-storage",
                    pull.query = query,
                    url = %url,
                    "elohim-storage: 404 not found; returning null (normal for absent records)"
                );
                Ok(Value::Null)
            }
            Ok(response) => {
                warn!(
                    pull.source = "elohim-storage",
                    pull.query = query,
                    url = %url,
                    status = %response.status(),
                    "elohim-storage resolver: non-success status; returning null (honest degradation)"
                );
                Ok(Value::Null)
            }
            Err(e) => {
                warn!(
                    pull.source = "elohim-storage",
                    pull.query = query,
                    url = %url,
                    error = %e,
                    "elohim-storage resolver: network error; returning null (honest degradation)"
                );
                Ok(Value::Null)
            }
        }
    }
}

// ─── ManifestResolver (real, HTTP-based) ──────────────────────────────────────

/// Pull resolver that issues HTTP GET requests to the elohim-storage manifest
/// endpoint.
///
/// # Query convention
///
/// The `query` parameter is treated as a manifest ID or key. The resolver
/// fetches `{base_url}/api/v1/manifest/{query}`.
///
/// Manifests are served from the same elohim-storage origin as content, so
/// `base_url` is typically the same value used by [`ElohimStorageResolver`].
///
/// # Honest degradation
///
/// Same degradation contract as [`ElohimStorageResolver`]: failures return
/// `Value::Null` without failing the gate.
pub struct ManifestResolver {
    base_url: String,
    http_client: reqwest::Client,
}

impl ManifestResolver {
    /// Construct a resolver pointing at `base_url`.
    ///
    /// `base_url` should be a bare origin, e.g. `"http://localhost:8090"`.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http_client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PullResolver for ManifestResolver {
    async fn resolve(&self, query: &str) -> Result<Value, GateError> {
        let url = format!(
            "{}/api/v1/manifest/{}",
            self.base_url.trim_end_matches('/'),
            query.trim_start_matches('/')
        );

        match self.http_client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                response.json::<Value>().await.map_err(|e| {
                    GateError::ContextAssembly(format!(
                        "manifest resolver: JSON parse error for {url}: {e}"
                    ))
                })
            }
            Ok(response) if response.status() == reqwest::StatusCode::NOT_FOUND => {
                debug!(
                    pull.source = "manifest",
                    pull.query = query,
                    url = %url,
                    "manifest resolver: 404 not found; returning null"
                );
                Ok(Value::Null)
            }
            Ok(response) => {
                warn!(
                    pull.source = "manifest",
                    pull.query = query,
                    url = %url,
                    status = %response.status(),
                    "manifest resolver: non-success status; returning null (honest degradation)"
                );
                Ok(Value::Null)
            }
            Err(e) => {
                warn!(
                    pull.source = "manifest",
                    pull.query = query,
                    url = %url,
                    error = %e,
                    "manifest resolver: network error; returning null (honest degradation)"
                );
                Ok(Value::Null)
            }
        }
    }
}

// ─── SourceChainResolver (stub, Phase 10+) ────────────────────────────────────

/// Source-chain resolver (stub — Phase 10+ deferred).
///
/// Reading from a persona's source-chain requires one of:
/// - HDK integration from inside a zome (i.e., running gate-client inside a
///   Holochain WASM context, which is the `gate-client-zome` pure-sync bridge
///   work deferred to Phase 10+), OR
/// - A conductor-bridge HTTP endpoint that exposes source-chain queries over
///   HTTP (e.g., an `elohim-agent-service` or sidecar that holds an open
///   app-WS connection to the conductor and accepts `GET /source-chain/{query}`
///   requests).
///
/// Neither bridge exists yet. This resolver returns `Value::Null` with a
/// `tracing::warn` so the gate degrades honestly rather than failing.
///
/// # Phase 10+ migration path
///
/// Option A — Conductor-bridge HTTP endpoint:
/// Replace this struct with an HTTP-based resolver (similar to
/// [`ElohimStorageResolver`]) that calls the conductor bridge at a configurable
/// URL. Add `source_chain_bridge_url: Option<String>` to [`GateClientConfig`]
/// and register the real resolver from `configure_runner_with_config`.
///
/// Option B — Zome-side (gate-client-zome):
/// Run gate-client inside a Holochain coordinator zome using the pure-sync
/// bridge. This resolver becomes irrelevant; the HDK provides source-chain
/// access natively.
pub(super) struct SourceChainResolver;

#[async_trait]
impl PullResolver for SourceChainResolver {
    async fn resolve(&self, query: &str) -> Result<Value, GateError> {
        warn!(
            pull.source = "source-chain",
            pull.query = query,
            "source-chain resolver is a Phase 10+ stub (requires conductor-bridge \
             or HDK integration); returning null — gate degrades honestly"
        );
        Ok(Value::Null)
    }
}

// ─── DhtResolver (stub, Phase 10+) ────────────────────────────────────────────

/// DHT resolver (stub — Phase 10+ deferred).
///
/// Reading from the DHT requires a conductor-bridge endpoint that exposes DHT
/// entry lookups over HTTP (i.e., an `elohim-agent-service` or sidecar that
/// holds an open app-WS connection to the Holochain conductor and accepts DHT
/// GET requests). This is distinct from the Holochain zome path — it is a
/// native Rust HTTP server that proxies DHT reads from the app-WS.
///
/// This resolver returns `Value::Null` with a `tracing::warn` so the gate
/// degrades honestly rather than failing.
///
/// # Phase 10+ migration path
///
/// Add a `dht_bridge_url: Option<String>` to [`GateClientConfig`] and
/// replace this stub with an HTTP-based resolver (similar to
/// [`ElohimStorageResolver`]) that calls the conductor bridge. Register the
/// real resolver from `configure_runner_with_config` when the URL is present.
pub(super) struct DhtResolver;

#[async_trait]
impl PullResolver for DhtResolver {
    async fn resolve(&self, query: &str) -> Result<Value, GateError> {
        warn!(
            pull.source = "dht",
            pull.query = query,
            "DHT resolver is a Phase 10+ stub (requires conductor-bridge HTTP \
             endpoint over app-WS); returning null — gate degrades honestly"
        );
        Ok(Value::Null)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ─── SourceChainResolver: stub returns null without error ─────────────────

    #[tokio::test]
    async fn source_chain_stub_returns_null_without_error() {
        let resolver = SourceChainResolver;
        let result = resolver.resolve("/any/query").await.unwrap();
        assert_eq!(result, Value::Null);
    }

    // ─── DhtResolver: stub returns null without error ─────────────────────────

    #[tokio::test]
    async fn dht_stub_returns_null_without_error() {
        let resolver = DhtResolver;
        let result = resolver.resolve("/dht/entry/hash").await.unwrap();
        assert_eq!(result, Value::Null);
    }

    // ─── ElohimStorageResolver: unreachable host → Value::Null (honest degradation)

    #[tokio::test]
    async fn elohim_storage_resolver_network_error_returns_null() {
        // Use a port on localhost that is guaranteed to be unreachable
        // (nothing listens on the standard ephemeral-range reject port in tests).
        // The resolver must return Null, not propagate the error.
        let resolver = ElohimStorageResolver::new("http://127.0.0.1:19999");
        let result = resolver.resolve("/db/content/some-cid").await.unwrap();
        assert_eq!(
            result,
            Value::Null,
            "network error must degrade to Null, not fail the gate"
        );
    }

    // ─── ManifestResolver: unreachable host → Value::Null (honest degradation)

    #[tokio::test]
    async fn manifest_resolver_network_error_returns_null() {
        let resolver = ManifestResolver::new("http://127.0.0.1:19999");
        let result = resolver.resolve("lamad").await.unwrap();
        assert_eq!(
            result,
            Value::Null,
            "network error must degrade to Null, not fail the gate"
        );
    }

    // ─── ElohimStorageResolver: 200 with valid JSON → parsed Value ────────────

    #[cfg(feature = "testing")]
    #[tokio::test]
    async fn elohim_storage_resolver_200_returns_parsed_value() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/db/content/cid-abc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"id": "cid-abc", "title": "Hello"})),
            )
            .mount(&server)
            .await;

        let resolver = ElohimStorageResolver::new(server.uri());
        let result = resolver.resolve("/db/content/cid-abc").await.unwrap();
        assert_eq!(
            result,
            json!({"id": "cid-abc", "title": "Hello"}),
            "200 with JSON body must return parsed Value"
        );
    }

    // ─── ElohimStorageResolver: 404 → Value::Null ────────────────────────────

    #[cfg(feature = "testing")]
    #[tokio::test]
    async fn elohim_storage_resolver_404_returns_null() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/db/content/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let resolver = ElohimStorageResolver::new(server.uri());
        let result = resolver.resolve("/db/content/missing").await.unwrap();
        assert_eq!(result, Value::Null, "404 must degrade to Null");
    }

    // ─── ElohimStorageResolver: 500 → Value::Null ────────────────────────────

    #[cfg(feature = "testing")]
    #[tokio::test]
    async fn elohim_storage_resolver_500_returns_null() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/db/content/error"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&server)
            .await;

        let resolver = ElohimStorageResolver::new(server.uri());
        let result = resolver.resolve("/db/content/error").await.unwrap();
        assert_eq!(result, Value::Null, "5xx must degrade to Null");
    }

    // ─── ElohimStorageResolver: 200 with invalid JSON → Err(GateError) ───────

    #[cfg(feature = "testing")]
    #[tokio::test]
    async fn elohim_storage_resolver_200_invalid_json_returns_err() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/db/content/bad-json"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("not valid json {{{{")
                    .append_header("Content-Type", "application/json"),
            )
            .mount(&server)
            .await;

        let resolver = ElohimStorageResolver::new(server.uri());
        let result = resolver.resolve("/db/content/bad-json").await;
        assert!(
            matches!(result, Err(GateError::ContextAssembly(_))),
            "200 with invalid JSON must return Err(ContextAssembly), got: {:?}",
            result
        );
    }

    // ─── ManifestResolver: 200 with valid JSON → parsed Value ────────────────

    #[cfg(feature = "testing")]
    #[tokio::test]
    async fn manifest_resolver_200_returns_parsed_value() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/manifest/lamad"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"id": "lamad", "version": "1.0.0"})),
            )
            .mount(&server)
            .await;

        let resolver = ManifestResolver::new(server.uri());
        let result = resolver.resolve("lamad").await.unwrap();
        assert_eq!(
            result,
            json!({"id": "lamad", "version": "1.0.0"}),
            "200 with JSON body must return parsed Value"
        );
    }

    // ─── ManifestResolver: 404 → Value::Null ─────────────────────────────────

    #[cfg(feature = "testing")]
    #[tokio::test]
    async fn manifest_resolver_404_returns_null() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/manifest/unknown"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let resolver = ManifestResolver::new(server.uri());
        let result = resolver.resolve("unknown").await.unwrap();
        assert_eq!(result, Value::Null, "manifest 404 must degrade to Null");
    }

    // ─── ManifestResolver: path normalization (leading slash on query) ────────

    #[cfg(feature = "testing")]
    #[tokio::test]
    async fn manifest_resolver_strips_leading_slash_from_query() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/manifest/lamad"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
            .mount(&server)
            .await;

        let resolver = ManifestResolver::new(server.uri());
        // Both "/lamad" and "lamad" must resolve to the same URL.
        let result = resolver.resolve("/lamad").await.unwrap();
        assert_eq!(result, json!({"ok": true}));

        let result2 = resolver.resolve("lamad").await.unwrap();
        assert_eq!(result2, json!({"ok": true}));
    }
}
