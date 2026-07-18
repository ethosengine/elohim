//! Per-app SSR renderer registry — the multi-app generalization of the single
//! `SSR_BUNDLE_PATH`/`SSR_BUNDLE_SLUG` renderer.
//!
//! A serving peer projects MANY apps (elohim-host-landing, lamad-spa, …) but
//! holds renderers only for the apps whose server bundles it materialized. The
//! registry keys renderers by app slug and exposes ONE selection function used
//! by BOTH the dispatch gate (may this route SSR at all?) and renderer lookup
//! (which isolate renders it?) — a single seam, so the gate and the selection
//! can never drift apart.
//!
//! Framework note: entries hold `Arc<dyn elohim_render::Renderer>` — the
//! framework-agnostic trait. Today every entry is an `AngularRenderer` because
//! the current clients are Angular; a react-ssr (Sophia) app plugs in as
//! another `Renderer` impl selected by its route's `RenderSpec`, with no change
//! to this registry's shape.
//!
//! Env contract (boot):
//!   SSR_BUNDLE_PATH   — filesystem path of the DEFAULT app's server entry
//!                       (e.g. /opt/elohim-render/main.server.mjs)
//!   SSR_BUNDLE_SLUGS  — comma-separated app slugs to materialize+load; the
//!                       FIRST is the default app (materialized over
//!                       SSR_BUNDLE_PATH exactly like the legacy single-slug
//!                       path); the rest land in `<parent>/apps/<slug>/`.
//!   SSR_BUNDLE_SLUG   — legacy singular; used when SSR_BUNDLE_SLUGS is unset.
//!
//! Each loaded app is its own V8 isolate (its own render worker thread) —
//! renderer count is bounded by the configured slug list, and the shared
//! render semaphore still bounds CONCURRENT renders across all of them.

use std::collections::HashMap;
use std::sync::Arc;

/// Renderers keyed by the app (EPR node slug) they can produce markup for.
pub struct RendererRegistry {
    by_app: HashMap<String, Arc<dyn elohim_render::Renderer>>,
    /// Renderer used for dispatches with NO projection app (legacy Registry
    /// manifest routes) — the first configured slug's renderer, or the
    /// image-baked bundle when no slug is configured.
    default_renderer: Option<Arc<dyn elohim_render::Renderer>>,
    /// The app the default renderer serves. `None` means the bundle's app is
    /// unknown (no slug env — image-baked bundle): selection then allows ANY
    /// app through the default renderer, preserving legacy behavior, and the
    /// compose step's typed cross-app refusal remains the backstop.
    default_app: Option<String>,
}

impl RendererRegistry {
    /// A registry with nothing loaded — SSR fully disabled (every dispatch
    /// takes the renderer-absent fallback).
    pub fn empty() -> Self {
        Self {
            by_app: HashMap::new(),
            default_renderer: None,
            default_app: None,
        }
    }

    /// Test/embedding constructor: explicit entries, no env or materialization.
    pub fn with_entries(
        default_renderer: Option<Arc<dyn elohim_render::Renderer>>,
        default_app: Option<String>,
        extra: Vec<(String, Arc<dyn elohim_render::Renderer>)>,
    ) -> Self {
        let mut by_app: HashMap<String, Arc<dyn elohim_render::Renderer>> =
            extra.into_iter().collect();
        if let (Some(app), Some(r)) = (default_app.as_ref(), default_renderer.as_ref()) {
            by_app.entry(app.clone()).or_insert_with(|| Arc::clone(r));
        }
        Self {
            by_app,
            default_renderer,
            default_app,
        }
    }

    /// Build the registry from the boot environment: materialize each
    /// configured app's server bundle from the substrate and load it. A
    /// failed extra app is logged and skipped (partial registry — the other
    /// apps still serve; the mismatch gate sheds the missing one, named).
    /// Mirrors the legacy `init_renderer` behavior exactly for the first slug.
    pub fn from_env() -> Self {
        let Ok(bundle_path) = std::env::var("SSR_BUNDLE_PATH") else {
            return Self::empty();
        };
        let storage_url = std::env::var("SSR_STORAGE_URL")
            .or_else(|_| std::env::var("STORAGE_URL"))
            .unwrap_or_else(|_| "http://localhost:8090".to_string());
        // Per-fetch soft budget for SSR data fetches (the TracingFetcher SLA):
        // parameter-bearing — too low flaps a slow-but-healthy storage peer,
        // too high lets a stalled fetch hold a sequential isolate.
        let soft_budget_ms = std::env::var("DOORWAY_SSR_FETCH_SOFT_BUDGET_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|&v| v > 0)
            .unwrap_or(elohim_render::DEFAULT_SOFT_BUDGET_MS);

        // Slug list: SSR_BUNDLE_SLUGS (comma) → legacy SSR_BUNDLE_SLUG → none.
        let slugs: Vec<String> = std::env::var("SSR_BUNDLE_SLUGS")
            .ok()
            .map(|s| {
                s.split(',')
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect()
            })
            .filter(|v: &Vec<String>| !v.is_empty())
            .or_else(|| std::env::var("SSR_BUNDLE_SLUG").ok().map(|s| vec![s]))
            .unwrap_or_default();

        let bundle_parent = std::path::Path::new(&bundle_path)
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let src = crate::ssr::DoorwayBundleSource::new(storage_url.clone());

        // Bootstrap default fetcher per isolate — every render swaps in its own
        // per-request fetcher via ctx.data_fetcher, so this one is never used
        // for a real render; it still gets a hard timeout (a bootstrap fetch
        // must never be an unbounded await on an SSR thread).
        let make_fetcher = || -> Arc<dyn elohim_render::DataFetcher> {
            let bootstrap_client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default();
            Arc::new(crate::ssr::ResolverFetcher::new(
                Arc::new(bootstrap_client),
                storage_url.clone(),
            ))
        };

        // Default app (first slug): materialize over the legacy SSR_BUNDLE_PATH
        // location, exactly like the retired single-slug init. No slug at all →
        // load the image-baked bundle as an app-unknown default.
        let default_app = slugs.first().cloned();
        if let Some(slug) = &default_app {
            // The server bundle is resolved from the EPR node's
            // `serverBlobHash` field (not the browser `blobHash`). On resolve
            // failure (e.g. serverBlobHash absent mid-migration) SSR for this
            // app stays off → CSR fallback, never a crash.
            match elohim_render::materialize_server_bundle(&src, slug, &bundle_parent) {
                Ok(materialized) => {
                    tracing::info!(
                        target: "doorway::ssr",
                        slug = %slug,
                        path = %materialized.display(),
                        "SSR server bundle materialized from substrate"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        target: "doorway::ssr",
                        slug = %slug,
                        "SSR server bundle materialization failed: {}",
                        e
                    );
                    return Self::empty();
                }
            }
        }
        let default_renderer = match elohim_render::AngularRenderer::with_soft_budget(
            std::path::PathBuf::from(&bundle_path),
            make_fetcher(),
            soft_budget_ms,
        ) {
            Ok(r) => {
                tracing::info!(
                    target: "doorway::ssr",
                    bundle = %bundle_path,
                    app = default_app.as_deref().unwrap_or("<unknown>"),
                    storage = %storage_url,
                    "SSR renderer ready"
                );
                Some(Arc::new(r) as Arc<dyn elohim_render::Renderer>)
            }
            Err(e) => {
                tracing::warn!(target: "doorway::ssr", "SSR disabled: {}", e);
                return Self::empty();
            }
        };

        // Extra apps (slugs beyond the first): each materializes into its own
        // `<parent>/apps/<slug>/` dir and gets its own isolate. A failure skips
        // ONLY that app.
        let mut extra: Vec<(String, Arc<dyn elohim_render::Renderer>)> = Vec::new();
        for slug in slugs.iter().skip(1) {
            let app_dir = bundle_parent.join("apps").join(slug);
            if let Err(e) = std::fs::create_dir_all(&app_dir) {
                tracing::warn!(
                    target: "doorway::ssr",
                    slug = %slug,
                    "SSR extra app dir create failed: {} — app skipped",
                    e
                );
                continue;
            }
            let materialized = match elohim_render::materialize_server_bundle(&src, slug, &app_dir)
            {
                Ok(p) => {
                    tracing::info!(
                        target: "doorway::ssr",
                        slug = %slug,
                        path = %p.display(),
                        "SSR server bundle materialized from substrate"
                    );
                    p
                }
                Err(e) => {
                    tracing::warn!(
                        target: "doorway::ssr",
                        slug = %slug,
                        "SSR server bundle materialization failed: {} — app skipped",
                        e
                    );
                    continue;
                }
            };
            match elohim_render::AngularRenderer::with_soft_budget(
                materialized,
                make_fetcher(),
                soft_budget_ms,
            ) {
                Ok(r) => {
                    tracing::info!(
                        target: "doorway::ssr",
                        app = %slug,
                        storage = %storage_url,
                        "SSR renderer ready"
                    );
                    extra.push((
                        slug.clone(),
                        Arc::new(r) as Arc<dyn elohim_render::Renderer>,
                    ));
                }
                Err(e) => {
                    tracing::warn!(
                        target: "doorway::ssr",
                        slug = %slug,
                        "SSR renderer load failed: {} — app skipped",
                        e
                    );
                }
            }
        }

        Self::with_entries(default_renderer, default_app, extra)
    }

    /// The ONE selection seam. `Some(app)` — a projected route: the app's own
    /// renderer, or (only when the default bundle's app is unknown) the legacy
    /// allow-through default. `None` — a no-projection legacy Registry route:
    /// the default renderer.
    pub fn select(&self, app: Option<&str>) -> Option<Arc<dyn elohim_render::Renderer>> {
        match app {
            Some(app) => self.by_app.get(app).cloned().or_else(|| {
                if self.default_app.is_none() {
                    self.default_renderer.clone()
                } else {
                    None
                }
            }),
            None => self.default_renderer.clone(),
        }
    }

    /// True when a projected app must be SHED as `renderer-app-mismatch`:
    /// renderers ARE loaded, but none serves this app. (With nothing loaded
    /// the dispatch takes the renderer-absent path instead — not a mismatch.)
    pub fn mismatch(&self, app: &str) -> bool {
        self.any_loaded() && self.select(Some(app)).is_none()
    }

    /// Whether any renderer is loaded at all (the registry-level analog of the
    /// old `state.renderer.is_some()`).
    pub fn any_loaded(&self) -> bool {
        self.default_renderer.is_some() || !self.by_app.is_empty()
    }

    /// Loaded app names for observability lines (`<unknown>` marks an
    /// app-unknown image-baked default).
    pub fn app_names(&self) -> String {
        if !self.any_loaded() {
            return "<none>".to_string();
        }
        let mut names: Vec<&str> = self.by_app.keys().map(String::as_str).collect();
        names.sort_unstable();
        if self.default_app.is_none() && self.default_renderer.is_some() {
            names.push("<unknown>");
        }
        names.join(",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct StubRenderer(&'static str);

    #[async_trait]
    impl elohim_render::Renderer for StubRenderer {
        async fn render(
            &self,
            _ctx: elohim_render::RenderContext,
        ) -> elohim_render::Result<elohim_render::RenderOutput> {
            Ok(elohim_render::RenderOutput {
                html: self.0.to_string(),
                ..Default::default()
            })
        }
    }

    fn stub(tag: &'static str) -> Arc<dyn elohim_render::Renderer> {
        Arc::new(StubRenderer(tag))
    }

    #[test]
    fn selects_each_apps_own_renderer() {
        let reg = RendererRegistry::with_entries(
            Some(stub("landing")),
            Some("elohim-host-landing".into()),
            vec![("lamad-spa".into(), stub("lamad"))],
        );
        assert!(reg.select(Some("elohim-host-landing")).is_some());
        assert!(reg.select(Some("lamad-spa")).is_some());
        // The gate and the selection share one seam: an unserved app is a
        // mismatch, never a wrong-app render.
        assert!(reg.select(Some("qahal-spa")).is_none());
        assert!(reg.mismatch("qahal-spa"));
        assert!(!reg.mismatch("lamad-spa"));
    }

    #[test]
    fn no_projection_uses_the_default_renderer() {
        let reg = RendererRegistry::with_entries(
            Some(stub("landing")),
            Some("elohim-host-landing".into()),
            vec![],
        );
        assert!(reg.select(None).is_some());
    }

    #[test]
    fn unknown_default_app_allows_any_projection_through() {
        // Image-baked bundle, no slug env: the bundle's app is unknown, so the
        // legacy behavior holds — allow, and compose's typed cross-app refusal
        // stays the backstop.
        let reg = RendererRegistry::with_entries(Some(stub("baked")), None, vec![]);
        assert!(reg.select(Some("lamad-spa")).is_some());
        assert!(!reg.mismatch("lamad-spa"));
    }

    #[test]
    fn empty_registry_is_absent_not_mismatched() {
        let reg = RendererRegistry::empty();
        assert!(!reg.any_loaded());
        assert!(reg.select(Some("lamad-spa")).is_none());
        // Nothing loaded → renderer-absent path, NOT a mismatch shed.
        assert!(!reg.mismatch("lamad-spa"));
        assert_eq!(reg.app_names(), "<none>");
    }

    #[test]
    fn app_names_lists_loaded_apps() {
        let reg = RendererRegistry::with_entries(
            Some(stub("landing")),
            Some("elohim-host-landing".into()),
            vec![("lamad-spa".into(), stub("lamad"))],
        );
        assert_eq!(reg.app_names(), "elohim-host-landing,lamad-spa");
    }
}
