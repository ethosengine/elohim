//! Pull resolvers for `ContextAssembleExecutor`.
//!
//! All four resolvers are real HTTP-based implementations with honest degradation:
//! - [`ElohimStorageResolver`] — HTTP GET to an elohim-storage endpoint.
//! - [`ManifestResolver`] — HTTP GET to the elohim-storage manifest endpoint.
//! - [`SourceChainResolver`] — HTTP GET to the conductor-bridge source-chain endpoint.
//! - [`DhtResolver`] — HTTP GET to the conductor-bridge DHT endpoint.
//!
//! All four are registered by `configure_runner_with_config` when
//! `elohim_storage_base_url` is set — they share the same origin as the
//! storage/manifest resolvers.
//!
//! # Conductor-bridge status (Phase 10)
//!
//! `SourceChainResolver` and `DhtResolver` target real HTTP endpoints in
//! elohim-storage (`GET /db/source-chain/{agent_id}/entries` and
//! `GET /db/dht/{entry_hash}`).  The HTTP pipes are wired end-to-end.
//! The conductor query layer (HcClient wired into HttpServer) is deferred to
//! Phase 11.  Until then, elohim-storage returns empty arrays / 404 responses,
//! which these resolvers degrade to `Value::Null` — gates complete with null
//! context rather than failing.
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

// ─── SourceChainResolver (real, HTTP-based) ───────────────────────────────────

/// Pull resolver that issues HTTP GET requests to the elohim-storage
/// conductor-bridge source-chain endpoint.
///
/// # Query convention
///
/// The `query` parameter is treated as a URL path fragment appended to
/// `{base_url}/db/source-chain`. Examples:
/// - `/{agent_id}/entries` → all source-chain entries for an agent
/// - `/{agent_id}/entries?filter=content-type:concept` → filtered entries
///
/// These paths correspond to `GET /db/source-chain/{agent_id}/entries` in
/// elohim-storage (added Task 10.2 Phase 10).
///
/// # Conductor-bridge status (Phase 10)
///
/// The HTTP pipe is wired end-to-end.  The conductor query layer (zome call
/// via HcClient) is deferred to Phase 11 — elohim-storage currently returns
/// an empty `entries: []` array.  This resolver interprets that as a JSON
/// object (success) and returns it; callers that need a non-null value will
/// see `{"agentId": ..., "entries": [], "phase": "10-stub"}`.
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
pub struct SourceChainResolver {
    base_url: String,
    http_client: reqwest::Client,
}

impl SourceChainResolver {
    /// Construct a resolver pointing at `base_url`.
    ///
    /// `base_url` should be a bare origin, e.g. `"http://localhost:8090"`.
    /// A trailing slash is stripped; the `query` parameter provides the path
    /// fragment after `/db/source-chain`.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http_client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PullResolver for SourceChainResolver {
    async fn resolve(&self, query: &str) -> Result<Value, GateError> {
        let url = format!(
            "{}/db/source-chain{}",
            self.base_url.trim_end_matches('/'),
            query
        );

        match self.http_client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                response.json::<Value>().await.map_err(|e| {
                    GateError::ContextAssembly(format!(
                        "source-chain resolver: JSON parse error for {url}: {e}"
                    ))
                })
            }
            Ok(response) if response.status() == reqwest::StatusCode::NOT_FOUND => {
                debug!(
                    pull.source = "source-chain",
                    pull.query = query,
                    url = %url,
                    "source-chain: 404 not found; returning null"
                );
                Ok(Value::Null)
            }
            Ok(response) => {
                warn!(
                    pull.source = "source-chain",
                    pull.query = query,
                    url = %url,
                    status = %response.status(),
                    "source-chain resolver: non-success status; returning null (honest degradation)"
                );
                Ok(Value::Null)
            }
            Err(e) => {
                warn!(
                    pull.source = "source-chain",
                    pull.query = query,
                    url = %url,
                    error = %e,
                    "source-chain resolver: network error; returning null (honest degradation)"
                );
                Ok(Value::Null)
            }
        }
    }
}

// ─── DhtResolver (real, HTTP-based) ───────────────────────────────────────────

/// Pull resolver that issues HTTP GET requests to the elohim-storage
/// conductor-bridge DHT endpoint.
///
/// # Query convention
///
/// The `query` parameter is treated as a URL path fragment appended to
/// `{base_url}/db/dht`. Examples:
/// - `/{entry_hash}` → fetch a DHT entry by hash (ActionHash or EntryHash)
///
/// These paths correspond to `GET /db/dht/{entry_hash}` in elohim-storage
/// (added Task 10.2 Phase 10).
///
/// # Conductor-bridge status (Phase 10)
///
/// The HTTP pipe is wired end-to-end.  The conductor query layer (zome call
/// via HcClient) is deferred to Phase 11 — elohim-storage currently returns
/// 404 for all DHT queries.  This resolver maps 404 → `Value::Null` (debug
/// log), so gates degrade honestly rather than failing.
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
pub struct DhtResolver {
    base_url: String,
    http_client: reqwest::Client,
}

impl DhtResolver {
    /// Construct a resolver pointing at `base_url`.
    ///
    /// `base_url` should be a bare origin, e.g. `"http://localhost:8090"`.
    /// A trailing slash is stripped; the `query` parameter provides the path
    /// fragment after `/db/dht`.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http_client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PullResolver for DhtResolver {
    async fn resolve(&self, query: &str) -> Result<Value, GateError> {
        let url = format!("{}/db/dht{}", self.base_url.trim_end_matches('/'), query);

        match self.http_client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                response.json::<Value>().await.map_err(|e| {
                    GateError::ContextAssembly(format!(
                        "dht resolver: JSON parse error for {url}: {e}"
                    ))
                })
            }
            Ok(response) if response.status() == reqwest::StatusCode::NOT_FOUND => {
                debug!(
                    pull.source = "dht",
                    pull.query = query,
                    url = %url,
                    "dht resolver: 404 not found; returning null (normal for absent entries)"
                );
                Ok(Value::Null)
            }
            Ok(response) => {
                warn!(
                    pull.source = "dht",
                    pull.query = query,
                    url = %url,
                    status = %response.status(),
                    "dht resolver: non-success status; returning null (honest degradation)"
                );
                Ok(Value::Null)
            }
            Err(e) => {
                warn!(
                    pull.source = "dht",
                    pull.query = query,
                    url = %url,
                    error = %e,
                    "dht resolver: network error; returning null (honest degradation)"
                );
                Ok(Value::Null)
            }
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ─── SourceChainResolver: unreachable host → Value::Null (honest degradation)

    #[tokio::test]
    async fn source_chain_resolver_network_error_returns_null() {
        let resolver = SourceChainResolver::new("http://127.0.0.1:19998");
        let result = resolver.resolve("/agent-xyz/entries").await.unwrap();
        assert_eq!(
            result,
            Value::Null,
            "network error must degrade to Null, not fail the gate"
        );
    }

    // ─── DhtResolver: unreachable host → Value::Null (honest degradation) ─────

    #[tokio::test]
    async fn dht_resolver_network_error_returns_null() {
        let resolver = DhtResolver::new("http://127.0.0.1:19998");
        let result = resolver.resolve("/uhC0k-some-entry-hash").await.unwrap();
        assert_eq!(
            result,
            Value::Null,
            "network error must degrade to Null, not fail the gate"
        );
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

    // ─── SourceChainResolver: 200 with valid JSON → parsed Value ─────────────

    #[cfg(feature = "testing")]
    #[tokio::test]
    async fn source_chain_resolver_200_returns_parsed_value() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/db/source-chain/agent-abc/entries"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "agentId": "agent-abc",
                "entries": [],
                "phase": "10-stub"
            })))
            .mount(&server)
            .await;

        let resolver = SourceChainResolver::new(server.uri());
        let result = resolver.resolve("/agent-abc/entries").await.unwrap();
        assert_eq!(
            result,
            json!({"agentId": "agent-abc", "entries": [], "phase": "10-stub"}),
            "200 with JSON body must return parsed Value"
        );
    }

    // ─── SourceChainResolver: 404 → Value::Null ──────────────────────────────

    #[cfg(feature = "testing")]
    #[tokio::test]
    async fn source_chain_resolver_404_returns_null() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/db/source-chain/missing-agent/entries"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let resolver = SourceChainResolver::new(server.uri());
        let result = resolver.resolve("/missing-agent/entries").await.unwrap();
        assert_eq!(result, Value::Null, "source-chain 404 must degrade to Null");
    }

    // ─── SourceChainResolver: 500 → Value::Null ──────────────────────────────

    #[cfg(feature = "testing")]
    #[tokio::test]
    async fn source_chain_resolver_500_returns_null() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/db/source-chain/agent-err/entries"))
            .respond_with(ResponseTemplate::new(500).set_body_string("conductor unavailable"))
            .mount(&server)
            .await;

        let resolver = SourceChainResolver::new(server.uri());
        let result = resolver.resolve("/agent-err/entries").await.unwrap();
        assert_eq!(result, Value::Null, "5xx must degrade to Null");
    }

    // ─── SourceChainResolver: 200 with invalid JSON → Err(GateError) ─────────

    #[cfg(feature = "testing")]
    #[tokio::test]
    async fn source_chain_resolver_200_invalid_json_returns_err() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/db/source-chain/agent-bad/entries"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("not valid json {{{{")
                    .append_header("Content-Type", "application/json"),
            )
            .mount(&server)
            .await;

        let resolver = SourceChainResolver::new(server.uri());
        let result = resolver.resolve("/agent-bad/entries").await;
        assert!(
            matches!(result, Err(GateError::ContextAssembly(_))),
            "200 with invalid JSON must return Err(ContextAssembly), got: {:?}",
            result
        );
    }

    // ─── DhtResolver: 200 with valid JSON → parsed Value ─────────────────────

    #[cfg(feature = "testing")]
    #[tokio::test]
    async fn dht_resolver_200_returns_parsed_value() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/db/dht/uhC0k-abc123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "entryHash": "uhC0k-abc123",
                "entryType": "Content",
                "content": {"id": "content-1", "title": "Test"}
            })))
            .mount(&server)
            .await;

        let resolver = DhtResolver::new(server.uri());
        let result = resolver.resolve("/uhC0k-abc123").await.unwrap();
        assert_eq!(
            result,
            json!({
                "entryHash": "uhC0k-abc123",
                "entryType": "Content",
                "content": {"id": "content-1", "title": "Test"}
            }),
            "200 with JSON body must return parsed Value"
        );
    }

    // ─── DhtResolver: 404 → Value::Null ──────────────────────────────────────

    #[cfg(feature = "testing")]
    #[tokio::test]
    async fn dht_resolver_404_returns_null() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/db/dht/uhC0k-missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let resolver = DhtResolver::new(server.uri());
        let result = resolver.resolve("/uhC0k-missing").await.unwrap();
        assert_eq!(result, Value::Null, "dht 404 must degrade to Null");
    }

    // ─── DhtResolver: 500 → Value::Null ──────────────────────────────────────

    #[cfg(feature = "testing")]
    #[tokio::test]
    async fn dht_resolver_500_returns_null() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/db/dht/uhC0k-err"))
            .respond_with(ResponseTemplate::new(500).set_body_string("conductor down"))
            .mount(&server)
            .await;

        let resolver = DhtResolver::new(server.uri());
        let result = resolver.resolve("/uhC0k-err").await.unwrap();
        assert_eq!(result, Value::Null, "dht 5xx must degrade to Null");
    }

    // ─── DhtResolver: 200 with invalid JSON → Err(GateError) ─────────────────

    #[cfg(feature = "testing")]
    #[tokio::test]
    async fn dht_resolver_200_invalid_json_returns_err() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/db/dht/uhC0k-bad"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("not valid json {{{{")
                    .append_header("Content-Type", "application/json"),
            )
            .mount(&server)
            .await;

        let resolver = DhtResolver::new(server.uri());
        let result = resolver.resolve("/uhC0k-bad").await;
        assert!(
            matches!(result, Err(GateError::ContextAssembly(_))),
            "200 with invalid JSON must return Err(ContextAssembly), got: {:?}",
            result
        );
    }
}
