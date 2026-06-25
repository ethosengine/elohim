//! SSR endpoint handlers.
//!
//! Compiled only when the `ssr` feature is enabled. Provides:
//! - `POST /render` — internal endpoint for doorway co-location forwarding
//! - `GET /spa/*` — peer-to-peer libp2p-routed direct rendering
//!
//! Both call into elohim-render's `AngularRenderer` with a `LocalFetcher`
//! that resolves content directly via storage's in-process HTTP layer
//! (no round-trip through the network; the fixture bundle does not fetch).
//!
//! ## Design notes
//!
//! The renderer is built once at startup from `SSR_BUNDLE_PATH` and reused
//! across requests. `SsrState::from_env()` returns `None` when the env var
//! is absent or the bundle fails to load — callers fall through to 503.
//!
//! `LocalFetcher` is a stub today. When elohim-storage's content service
//! exposes a path-based method, switch it to forward fetches there without
//! an HTTP hop back to self.

#![cfg(feature = "ssr")]

use async_trait::async_trait;
use elohim_render::{materialize_bundle, AngularRenderer, BundleSource, DataFetcher};
use elohim_render::{FetchRequest, FetchResponse, RenderContext};
use elohim_render::{RenderError, RenderSpec, Renderer};
use std::path::PathBuf;
use std::sync::Arc;

// =============================================================================
// SsrState — renderer lifecycle
// =============================================================================

/// Holds the renderer instance for the lifetime of the server.
///
/// Constructed once at startup via `SsrState::from_env()`. If `SSR_BUNDLE_PATH`
/// is unset or the bundle fails to load, the field is `None` and every
/// SSR-gated handler returns 503.
pub struct SsrState {
    pub(crate) renderer: Arc<dyn Renderer>,
}

impl SsrState {
    /// Build from `SSR_BUNDLE_PATH` environment variable.
    ///
    /// Returns `None` when the env var is absent or the bundle cannot be loaded
    /// (e.g. file missing, V8 snapshot corrupt). A `tracing::warn!` is emitted
    /// in that case so the operator knows SSR is degraded without crashing.
    ///
    /// ## Materialize-first (substrate self-serve)
    ///
    /// When `SSR_BUNDLE_SLUG` is also set, the bundle is fetched from this peer's
    /// OWN content + blob surface (resolve slug → `blobHash`, fetch the blob,
    /// verify integrity, unzip) into the parent directory of `SSR_BUNDLE_PATH`
    /// BEFORE the renderer is built. This lets a peer render its own SPA directly
    /// without a pre-baked bundle on disk — the p2p-native analogue of doorway's
    /// `init_renderer` materialize step. On any materialization error we `warn!`
    /// and return `None` (SSR degraded, never a crash) — same graceful-degrade
    /// contract as a missing/corrupt bundle.
    pub fn from_env() -> Option<Self> {
        let path = std::env::var("SSR_BUNDLE_PATH").ok()?;

        // Materialize-first: if a slug is configured, pull the bundle from this
        // peer's own storage surface before loading the renderer.
        if let Ok(slug) = std::env::var("SSR_BUNDLE_SLUG") {
            let target_dir = PathBuf::from(&path);
            let target_dir = target_dir
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."));
            let storage_base = std::env::var("SSR_STORAGE_URL")
                .or_else(|_| std::env::var("STORAGE_URL"))
                .unwrap_or_else(|_| "http://localhost:8090".to_string());
            let src = LocalBundleSource::new(storage_base);
            match materialize_bundle(&src, &slug, target_dir) {
                Ok(materialized) => {
                    tracing::info!(
                        target: "elohim_storage::ssr",
                        slug = %slug,
                        path = %materialized.display(),
                        "SSR bundle materialized from local substrate"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        target: "elohim_storage::ssr",
                        slug = %slug,
                        error = %e,
                        "SSR disabled: bundle materialization failed"
                    );
                    return None;
                }
            }
        }

        let bundle = PathBuf::from(path);
        let fetcher: Arc<dyn DataFetcher> = Arc::new(LocalFetcher);
        match AngularRenderer::new(bundle, fetcher) {
            Ok(r) => Some(Self {
                renderer: Arc::new(r),
            }),
            Err(e) => {
                tracing::warn!(
                    target: "elohim_storage::ssr",
                    error = %e,
                    "SSR disabled: bundle load failed"
                );
                None
            }
        }
    }
}

// =============================================================================
// LocalFetcher — in-process content resolution (stub for MVP)
// =============================================================================

/// Resolves Angular SSR `fetch()` calls against storage's in-process services.
///
/// MVP: stub returning 404. The fixture bundle does not issue fetches, so this
/// is sufficient for the direct-peer SSR use case today. When storage's content
/// service exposes a path-addressable method, wire it here to avoid the HTTP
/// hop back to self.
///
/// TODO: hold a reference to storage's in-process content service so fetches
/// resolve against the local DHT projection without any HTTP round-trip.
pub struct LocalFetcher;

#[async_trait]
impl DataFetcher for LocalFetcher {
    async fn fetch(&self, _request: FetchRequest) -> elohim_render::Result<FetchResponse> {
        Ok(FetchResponse {
            status: 404,
            headers: Default::default(),
            body: b"local fetcher: stub".to_vec(),
            content_hash: None,
        })
    }
}

// =============================================================================
// LocalBundleSource — blocking BundleSource over this peer's own storage HTTP API
// =============================================================================

/// Implements `elohim_render::BundleSource` by hitting this peer's OWN storage
/// HTTP surface synchronously (blocking reqwest): resolve a deployment slug to a
/// content-addressed `blobHash` via `GET /db/content/{slug}`, then fetch the raw
/// bytes via `GET /blob/{hash}`.
///
/// ## v1 limitation — loopback HTTP, not in-process
///
/// This is a deliberate v1: storage IS the service answering :8090, so the
/// in-process content-DB + blob-store handles already exist — but they are not
/// threaded into the arg-less `SsrState::from_env()` boot seam, and doing so is a
/// larger refactor than this materialize step warrants. The brief sanctions a
/// blocking loopback `GET` to `:8090` as the acceptable v1. The cost is one
/// localhost round-trip at boot (at most once per process startup, never per
/// render), and it requires the HTTP server to be accepting on the base URL when
/// `from_env()` runs. When an ergonomic in-process accessor is exposed, switch
/// `resolve_blob_hash`/`fetch_blob` to call it and drop the HTTP hop.
///
/// ## Async-caller safety
///
/// Each blocking HTTP call runs on a dedicated OS thread via
/// `std::thread::spawn(...).join()`. `reqwest::blocking` creates its own tokio
/// runtime and `block_on`s; doing that from within an ambient tokio worker would
/// panic with "Cannot start a runtime from within a runtime." The spawned thread
/// has no ambient tokio context, so it is safe regardless of whether
/// `from_env()` is called inside or outside a runtime. (Mirrors doorway's
/// `DoorwayBundleSource`.)
pub struct LocalBundleSource {
    storage_base: String,
}

impl LocalBundleSource {
    pub fn new(storage_base_url: String) -> Self {
        Self {
            storage_base: storage_base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Build a fresh blocking client. Each call site spawns its own thread, so a
    /// per-call client keeps the impl simple and avoids `Arc` across the spawn.
    fn make_client() -> reqwest::blocking::Client {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default()
    }
}

impl BundleSource for LocalBundleSource {
    fn resolve_blob_hash(&self, slug: &str) -> elohim_render::Result<String> {
        let url = format!("{}/db/content/{}", self.storage_base, slug);
        let result = std::thread::spawn(move || {
            let client = LocalBundleSource::make_client();
            let resp = client.get(&url).send().map_err(|e| {
                RenderError::Bootstrap(format!("resolve_blob_hash GET failed: {e}"))
            })?;
            let status = resp.status();
            resp.error_for_status()
                .map_err(|_| {
                    RenderError::Bootstrap(format!(
                        "resolve_blob_hash: HTTP {status} for slug `{url}`"
                    ))
                })?
                .text()
                .map_err(|e| RenderError::Bootstrap(format!("resolve_blob_hash read body: {e}")))
        })
        .join()
        .map_err(|_| RenderError::Bootstrap("resolve_blob_hash: thread panicked".into()))?;
        parse_blob_hash(&result?)
    }

    fn fetch_blob(&self, hash: &str) -> elohim_render::Result<Vec<u8>> {
        let url = format!("{}/blob/{}", self.storage_base, hash);
        std::thread::spawn(move || {
            let client = LocalBundleSource::make_client();
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

/// Parse the `blobHash` field from the JSON body of `GET /db/content/{slug}`.
///
/// The storage boundary is camelCase (see `views.rs`), so the field is `blobHash`.
pub fn parse_blob_hash(body: &str) -> elohim_render::Result<String> {
    let v: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| RenderError::Bootstrap(format!("parse_blob_hash: invalid JSON: {e}")))?;
    v.get("blobHash")
        .and_then(|h| h.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| RenderError::Bootstrap("parse_blob_hash: missing `blobHash` field".into()))
}

// =============================================================================
// render_url — shared render path for both endpoints
// =============================================================================

/// Render the given URL via the SSR runtime.
///
/// Used by both `GET /spa/*` and `POST /render`.
pub async fn render_url(state: &SsrState, url: String) -> Result<String, RenderError> {
    let ctx = RenderContext {
        spec: RenderSpec::AngularSsr,
        url: url.clone(),
        data_fetcher: Arc::new(LocalFetcher),
        limits: Default::default(),
    };
    let out = state.renderer.render(ctx).await?;
    // p2p-native render-trace telemetry — the same signal doorway emits, but from
    // a peer rendering its own content directly (no web2 hop). The terminal
    // classification (rendered-empty vs stalled) + per-render latency is the
    // feedback that tunes the diversity of peer compute commitments.
    tracing::info!(
        target: "elohim_storage::ssr::trace",
        url = %url,
        terminal = out.trace.terminal.as_str(),
        fetches = out.trace.fetches.len(),
        wall_ms = out.trace.wall_ms,
        "SSR render trace"
    );
    Ok(out.html)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssr_feature_compiles() {
        // This test exists solely to verify the module compiles cleanly when
        // the ssr feature is ON.  The default build profile (feature off) is
        // covered by ssr_direct.rs's `ssr_disabled_compiles_without_renderer`.
    }

    // ── parse_blob_hash ──────────────────────────────────────────────────────
    // The pure resolve-parse piece of LocalBundleSource (mirrors Task 2's
    // doorway parse_blob_hash). The HTTP transport itself is exercised at the
    // doorway boundary (DoorwayBundleSource wiremock tests) and shares the same
    // materialize_bundle path; here we lock down the camelCase field contract.

    #[test]
    fn parse_blob_hash_valid_returns_hash() {
        let body = r#"{"id":"my-spa","blobHash":"sha256-abc123","contentType":"epr-composite"}"#;
        let result = parse_blob_hash(body);
        assert_eq!(result.unwrap(), "sha256-abc123");
    }

    #[test]
    fn parse_blob_hash_missing_field_returns_err() {
        // snake_case `blob_hash` must NOT satisfy the camelCase boundary contract.
        let body = r#"{"id":"my-spa","blob_hash":"sha256-abc123"}"#;
        let result = parse_blob_hash(body);
        assert!(result.is_err(), "missing camelCase blobHash should be Err");
    }

    #[test]
    fn parse_blob_hash_invalid_json_returns_err() {
        let body = "not json at all";
        let result = parse_blob_hash(body);
        assert!(result.is_err(), "invalid JSON should be Err, not panic");
    }

    #[test]
    fn local_bundle_source_trims_trailing_slash() {
        // The base URL must not double up slashes when composing endpoint URLs.
        let src = LocalBundleSource::new("http://localhost:8090/".to_string());
        assert_eq!(src.storage_base, "http://localhost:8090");
    }
}
