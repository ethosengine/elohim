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
use elohim_render::{AngularRenderer, DataFetcher, FetchRequest, FetchResponse, RenderContext};
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
    pub fn from_env() -> Option<Self> {
        let path = std::env::var("SSR_BUNDLE_PATH").ok()?;
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
// render_url — shared render path for both endpoints
// =============================================================================

/// Render the given URL via the SSR runtime.
///
/// Used by both `GET /spa/*` and `POST /render`.
pub async fn render_url(state: &SsrState, url: String) -> Result<String, RenderError> {
    let ctx = RenderContext {
        spec: RenderSpec::AngularSsr,
        url,
        data_fetcher: Arc::new(LocalFetcher),
        limits: Default::default(),
    };
    state.renderer.render(ctx).await.map(|out| out.html)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    #[test]
    fn ssr_feature_compiles() {
        // This test exists solely to verify the module compiles cleanly when
        // the ssr feature is ON.  The default build profile (feature off) is
        // covered by ssr_direct.rs's `ssr_disabled_compiles_without_renderer`.
    }
}
