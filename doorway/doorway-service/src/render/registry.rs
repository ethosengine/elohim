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
//! ## Bundle-head reconciliation (Track-4 T4-1)
//!
//! The server bundle is materialized once at boot; the declared head (the
//! storage content row's `serverBlobHash`) moves whenever CI re-deploys the
//! app. Without reconciliation a running doorway serves the boot bundle
//! forever — a restart is the only refresh. This registry now:
//!
//! 1. records a per-slug [`BundleHead`] attestation at materialize time
//!    ({slug, serverBlobHash, materializedAt, status}), surfaced on
//!    `/health/startup` as `servedBundleHeads`;
//! 2. re-resolves each slug's declared `serverBlobHash` on a background tick
//!    ([`reconcile`]) and, on mismatch, re-materializes into a FRESH directory,
//!    builds a new isolate, and HOT-SWAPS the registry entry in place —
//!    converge-to-declared-head without a process restart.
//!
//! Hot-swap is safe because each `AngularRenderer` owns its own V8 isolate on
//! its own OS thread (the isolate never crosses threads): building a new one
//! spawns a fresh thread/isolate, and dropping the old `Arc` tears down the old
//! one on `Drop`. The swap itself is an `Arc` pointer replacement under a brief
//! write lock; the heavy materialize + isolate construction happens OUTSIDE the
//! lock on a blocking thread.
//!
//! ### Safe-degrade invariants (each individually tested via [`decide_reconcile`]
//! and the swap/mark helpers)
//! - declared-head read fails (storage unreachable) → keep serving current,
//!   status unchanged. Never adopt an unverifiable head.
//! - re-materialize / isolate construction fails → keep serving the OLD
//!   renderer, status `failed` (error retained), retry next tick.
//! - blob hash-verify mismatch → surfaces as a materialize error →
//!   treated exactly as the construction-failure arm (materialize verifies the
//!   fetched bytes against the declared hash before it ever returns Ok).
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

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

/// Lifecycle status of a served bundle head. Serialized lowercase on the
/// `servedBundleHeads` health field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadStatus {
    /// Materialized bundle matches the last-observed declared head.
    Current,
    /// A newer declared head was observed but not yet materialized/swapped.
    Stale,
    /// A re-materialize + swap is actively in progress.
    Refreshing,
    /// The last re-materialize attempt failed; the OLD renderer is still served.
    Failed,
    /// The renderer loaded (bytes ARE on disk and serving) but the boot-time
    /// `serverBlobHash` attestation could not be CONFIRMED — the declared-head
    /// read failed, or the head MOVED across the materialize window so we cannot
    /// prove which hash the on-disk bytes verified against. The slug is kept in
    /// the head table (NOT dropped) precisely so `reconcile` retries it every
    /// tick and converges to a confirmed head; the health surface shows it as
    /// `unresolved` so a probe/operator can distinguish it from a slug that is
    /// genuinely absent.
    Unresolved,
}

impl HeadStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            HeadStatus::Current => "current",
            HeadStatus::Stale => "stale",
            HeadStatus::Refreshing => "refreshing",
            HeadStatus::Failed => "failed",
            HeadStatus::Unresolved => "unresolved",
        }
    }
}

/// Per-slug bundle-identity attestation — the doorway's honest answer to "which
/// server bundle am I serving for this app, and how does it compare to the
/// declared head?". Projected onto `/health/startup` as one `servedBundleHeads`
/// entry.
#[derive(Debug, Clone)]
pub struct BundleHead {
    /// The app slug (EPR node slug) this head belongs to.
    pub slug: String,
    /// The `serverBlobHash` of the bundle CURRENTLY materialized + serving.
    pub server_blob_hash: String,
    /// When this bundle was materialized (boot or last successful swap).
    pub materialized_at: chrono::DateTime<chrono::Utc>,
    /// Lifecycle status.
    pub status: HeadStatus,
    /// The declared `serverBlobHash` observed on the last reconcile check
    /// (`None` until the first check runs).
    pub declared_server_blob_hash: Option<String>,
    /// The error retained from the last failed re-materialize attempt.
    pub last_error: Option<String>,
}

impl BundleHead {
    fn to_json(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        obj.insert("slug".into(), serde_json::Value::String(self.slug.clone()));
        obj.insert(
            "serverBlobHash".into(),
            serde_json::Value::String(self.server_blob_hash.clone()),
        );
        obj.insert(
            "materializedAt".into(),
            serde_json::Value::String(self.materialized_at.to_rfc3339()),
        );
        obj.insert(
            "status".into(),
            serde_json::Value::String(self.status.as_str().to_string()),
        );
        // declaredServerBlobHash only when known (from a reconcile check).
        if let Some(ref d) = self.declared_server_blob_hash {
            obj.insert(
                "declaredServerBlobHash".into(),
                serde_json::Value::String(d.clone()),
            );
        }
        // error is additive — present only when the last attempt failed.
        if let Some(ref e) = self.last_error {
            obj.insert("error".into(), serde_json::Value::String(e.clone()));
        }
        serde_json::Value::Object(obj)
    }
}

/// Per-slug outcome of one reconcile pass — the `POST /admin/ssr-bundle/refresh`
/// response shape.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlugOutcome {
    pub slug: String,
    /// `"current"` (already matched), `"refreshed"` (swapped to a new head),
    /// `"failed"` (re-materialize failed, old kept), or `"unreachable"`
    /// (declared-head read failed, old kept).
    pub outcome: String,
    /// The `serverBlobHash` now being served after this pass.
    pub server_blob_hash: Option<String>,
    /// The declared `serverBlobHash` observed this pass (`None` when the read
    /// failed).
    pub declared_server_blob_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Boot-captured context needed to re-resolve + re-materialize a declared head
/// without a restart. `None` on registries that loaded no slug (image-baked
/// bundle, or SSR disabled) — reconcile is then a no-op.
#[derive(Debug, Clone)]
struct ReconcileCtx {
    storage_url: String,
    bundle_parent: PathBuf,
    soft_budget_ms: u64,
}

/// Mutable registry contents guarded by one `RwLock`. Reads (the SSR hot path:
/// `select`/`mismatch`/`app_names`) take the read lock; a reconcile swap takes
/// the write lock only for the `Arc`-pointer replacement (never across `.await`
/// or blocking work).
struct RegistryInner {
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
    /// Per-slug bundle-head attestations, keyed by slug. Only slugs whose
    /// server bundle materialized successfully at boot appear here.
    heads: HashMap<String, BundleHead>,
    /// Slugs with an in-progress re-materialize. The per-slug concurrency guard:
    /// a concurrent reconcile (background tick + admin refresh, or two admin
    /// calls) must not double-enter the Stale branch and race two materialize
    /// passes into the same deterministic `.reconcile/<slug>/<hash16>` dir.
    refreshing: HashSet<String>,
}

/// A holding pen for renderers displaced by a hot-swap. A displaced
/// `AngularRenderer` owns a V8 isolate whose `Drop` closes its channel and
/// **joins** the worker thread (`angular.rs`), which can BLOCK if a render is
/// still in flight — so the drop must never run on a tokio worker (the
/// 2026-06-13 freeze class). The graveyard keeps `strong_count >= 1` on every
/// displaced renderer, so an in-flight request holding its own `Arc` clone is
/// NEVER the last owner; a dedicated off-runtime reaper drops each one only
/// once it is the sole remaining owner (`strong_count == 1`), guaranteeing the
/// terminal `Drop` (the join) runs off the async runtime.
#[derive(Clone)]
struct Graveyard {
    plots: Arc<Mutex<Vec<Arc<dyn elohim_render::Renderer>>>>,
}

impl Graveyard {
    fn new() -> Self {
        Self {
            plots: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Bury a displaced renderer. The graveyard now holds a strong ref, so the
    /// request path can never be the renderer's last owner.
    fn bury(&self, renderer: Arc<dyn elohim_render::Renderer>) {
        self.plots
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(renderer);
    }

    /// Drop every buried renderer whose ONLY remaining owner is the graveyard
    /// (`strong_count == 1`); leave the rest (still referenced by an in-flight
    /// request) for a later pass. MUST run off the async runtime — the terminal
    /// drop runs `AngularRenderer::Drop`, which joins a worker thread. The Arcs
    /// are drained under the lock but DROPPED after releasing it, so a join can
    /// never block `bury`. Returns the count reaped.
    fn reap(&self) -> usize {
        let to_drop: Vec<Arc<dyn elohim_render::Renderer>> = {
            let mut plots = self.plots.lock().unwrap_or_else(|e| e.into_inner());
            let mut drained = Vec::new();
            let mut i = 0;
            while i < plots.len() {
                if Arc::strong_count(&plots[i]) == 1 {
                    drained.push(plots.swap_remove(i));
                } else {
                    i += 1; // still referenced by an in-flight render — reap next pass
                }
            }
            drained
        };
        let n = to_drop.len();
        drop(to_drop); // terminal Drops (the joins) run here, lock already released
        n
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.plots.lock().unwrap_or_else(|e| e.into_inner()).len()
    }
}

/// Renderers keyed by the app (EPR node slug) they can produce markup for.
pub struct RendererRegistry {
    inner: Arc<RwLock<RegistryInner>>,
    /// Boot context for reconcile; `None` disables reconcile for this registry.
    reconcile_ctx: Option<ReconcileCtx>,
    /// Off-runtime teardown queue for renderers displaced by a hot-swap.
    graveyard: Graveyard,
}

impl RendererRegistry {
    /// A registry with nothing loaded — SSR fully disabled (every dispatch
    /// takes the renderer-absent fallback).
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RegistryInner {
                by_app: HashMap::new(),
                default_renderer: None,
                default_app: None,
                heads: HashMap::new(),
                refreshing: HashSet::new(),
            })),
            reconcile_ctx: None,
            graveyard: Graveyard::new(),
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
            inner: Arc::new(RwLock::new(RegistryInner {
                by_app,
                default_renderer,
                default_app,
                heads: HashMap::new(),
                refreshing: HashSet::new(),
            })),
            reconcile_ctx: None,
            graveyard: Graveyard::new(),
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

        let mut heads: HashMap<String, BundleHead> = HashMap::new();

        // Default app (first slug): materialize over the legacy SSR_BUNDLE_PATH
        // location, exactly like the retired single-slug init. No slug at all →
        // load the image-baked bundle as an app-unknown default.
        let default_app = slugs.first().cloned();
        if let Some(slug) = &default_app {
            // The server bundle is resolved from the EPR node's
            // `serverBlobHash` field (not the browser `blobHash`). On resolve
            // failure (e.g. serverBlobHash absent mid-migration) SSR for this
            // app stays off → CSR fallback, never a crash.
            //
            // Single-resolve TOCTOU guard (Track-4): capture the declared head
            // BEFORE materialize so `boot_head` can re-resolve AFTER and attest
            // a `current` head ONLY when the declared head was UNCHANGED across
            // the whole materialize window (materialize's own internal resolve
            // then provably read that same stable hash, so the bytes on disk
            // verified against the hash we attest). On drift/failure the slug is
            // registered `unresolved` (never dropped) so reconcile converges.
            let pre = {
                use elohim_render::BundleSource;
                src.resolve_server_blob_hash(slug)
            };
            match elohim_render::materialize_server_bundle(&src, slug, &bundle_parent) {
                Ok(materialized) => {
                    tracing::info!(
                        target: "doorway::ssr",
                        slug = %slug,
                        path = %materialized.display(),
                        "SSR server bundle materialized from substrate"
                    );
                    heads.insert(slug.clone(), boot_head(&src, slug, pre));
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
            // Single-resolve TOCTOU guard (see the default-app branch): capture
            // the declared head before materialize so `boot_head` can bracket the
            // materialize window and only attest a confirmed hash.
            let pre = {
                use elohim_render::BundleSource;
                src.resolve_server_blob_hash(slug)
            };
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
                    heads.insert(slug.clone(), boot_head(&src, slug, pre));
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

        // Assemble inner (mirrors `with_entries` — the default app's renderer is
        // also keyed under its slug so a projected default route resolves it).
        let mut by_app: HashMap<String, Arc<dyn elohim_render::Renderer>> =
            extra.into_iter().collect();
        if let (Some(app), Some(r)) = (default_app.as_ref(), default_renderer.as_ref()) {
            by_app.entry(app.clone()).or_insert_with(|| Arc::clone(r));
        }

        // Reconcile is enabled only when at least one slug was configured (there
        // is a declared head to converge toward). An image-baked default with no
        // slug has no `serverBlobHash` source, so it never reconciles.
        let reconcile_ctx = if slugs.is_empty() {
            None
        } else {
            Some(ReconcileCtx {
                storage_url,
                bundle_parent,
                soft_budget_ms,
            })
        };

        Self {
            inner: Arc::new(RwLock::new(RegistryInner {
                by_app,
                default_renderer,
                default_app,
                heads,
                refreshing: HashSet::new(),
            })),
            reconcile_ctx,
            graveyard: Graveyard::new(),
        }
    }

    /// The ONE selection seam. `Some(app)` — a projected route: the app's own
    /// renderer, or (only when the default bundle's app is unknown) the legacy
    /// allow-through default. `None` — a no-projection legacy Registry route:
    /// the default renderer.
    pub fn select(&self, app: Option<&str>) -> Option<Arc<dyn elohim_render::Renderer>> {
        let inner = self.inner.read().unwrap_or_else(|e| e.into_inner());
        match app {
            Some(app) => inner.by_app.get(app).cloned().or_else(|| {
                if inner.default_app.is_none() {
                    inner.default_renderer.clone()
                } else {
                    None
                }
            }),
            None => inner.default_renderer.clone(),
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
        let inner = self.inner.read().unwrap_or_else(|e| e.into_inner());
        inner.default_renderer.is_some() || !inner.by_app.is_empty()
    }

    /// Loaded app names for observability lines (`<unknown>` marks an
    /// app-unknown image-baked default).
    pub fn app_names(&self) -> String {
        let inner = self.inner.read().unwrap_or_else(|e| e.into_inner());
        if inner.default_renderer.is_none() && inner.by_app.is_empty() {
            return "<none>".to_string();
        }
        let mut names: Vec<&str> = inner.by_app.keys().map(String::as_str).collect();
        names.sort_unstable();
        if inner.default_app.is_none() && inner.default_renderer.is_some() {
            names.push("<unknown>");
        }
        names.join(",")
    }

    // ── Bundle-head attestation + reconciliation ─────────────────────────────

    /// Snapshot of every served bundle head (for the `servedBundleHeads` health
    /// field), sorted by slug for stable output.
    pub fn heads_snapshot(&self) -> Vec<BundleHead> {
        let inner = self.inner.read().unwrap_or_else(|e| e.into_inner());
        let mut heads: Vec<BundleHead> = inner.heads.values().cloned().collect();
        heads.sort_by(|a, b| a.slug.cmp(&b.slug));
        heads
    }

    /// The `servedBundleHeads` JSON array for `/health/startup`.
    pub fn heads_json(&self) -> serde_json::Value {
        serde_json::Value::Array(
            self.heads_snapshot()
                .iter()
                .map(BundleHead::to_json)
                .collect(),
        )
    }

    /// Whether reconcile is enabled (a declared-head source exists). `false` for
    /// test/image-baked registries.
    pub fn reconcile_enabled(&self) -> bool {
        self.reconcile_ctx.is_some()
    }

    /// Run ONE reconcile pass over every served slug: re-resolve its declared
    /// `serverBlobHash`, and on mismatch re-materialize + hot-swap. Returns a
    /// per-slug outcome. No-op (empty) when reconcile is disabled.
    ///
    /// Concurrency: heavy work (declared-head resolve, materialize, isolate
    /// construction) runs on `spawn_blocking`; the registry write lock is taken
    /// only for the brief `Arc` swap / status update and is NEVER held across an
    /// `.await` or blocking call.
    pub async fn reconcile(&self, cache: &crate::cache::ContentCache) -> Vec<SlugOutcome> {
        let Some(ctx) = self.reconcile_ctx.clone() else {
            return Vec::new();
        };

        // Snapshot the reconcile targets (slug, currently-materialized hash,
        // whether this slug is the default app) without holding the lock across
        // the async work below.
        let targets: Vec<(String, String, bool)> = {
            let inner = self.inner.read().unwrap_or_else(|e| e.into_inner());
            inner
                .heads
                .values()
                .map(|h| {
                    let is_default = inner.default_app.as_deref() == Some(h.slug.as_str());
                    (h.slug.clone(), h.server_blob_hash.clone(), is_default)
                })
                .collect()
        };

        let mut outcomes = Vec::new();
        for (slug, materialized_hash, is_default) in targets {
            let declared = resolve_declared(&ctx.storage_url, &slug).await;
            match decide_reconcile(&materialized_hash, declared) {
                ReconcileDecision::Unreachable(err) => {
                    // Safe-degrade: keep serving current, status unchanged. Never
                    // adopt an unverifiable head.
                    tracing::debug!(
                        target: "doorway::ssr",
                        slug = %slug,
                        error = %err,
                        "bundle reconcile: declared-head read failed — keeping current"
                    );
                    outcomes.push(SlugOutcome {
                        slug,
                        outcome: "unreachable".into(),
                        server_blob_hash: Some(materialized_hash),
                        declared_server_blob_hash: None,
                        error: Some(err),
                    });
                }
                ReconcileDecision::UpToDate(declared_hash) => {
                    self.mark_current(&slug, &declared_hash);
                    outcomes.push(SlugOutcome {
                        slug,
                        outcome: "current".into(),
                        server_blob_hash: Some(materialized_hash),
                        declared_server_blob_hash: Some(declared_hash),
                        error: None,
                    });
                }
                ReconcileDecision::Stale(declared_hash) => {
                    // Per-slug in-progress guard: a concurrent reconcile (the
                    // background tick + POST /admin/ssr-bundle/refresh, or two
                    // admin calls) must not double-enter this branch and race two
                    // materialize passes writing the SAME deterministic
                    // `.reconcile/<slug>/<hash16>` directory. try_begin_refresh
                    // checks-and-sets under the write lock; the loser skips.
                    if !self.try_begin_refresh(&slug, &declared_hash) {
                        tracing::debug!(
                            target: "doorway::ssr",
                            slug = %slug,
                            "bundle reconcile: already refreshing — skipping concurrent pass"
                        );
                        outcomes.push(SlugOutcome {
                            slug,
                            outcome: "refreshing".into(),
                            server_blob_hash: Some(materialized_hash),
                            declared_server_blob_hash: Some(declared_hash),
                            error: None,
                        });
                        continue;
                    }

                    tracing::info!(
                        target: "doorway::ssr",
                        slug = %slug,
                        from = %materialized_hash,
                        to = %declared_hash,
                        "bundle reconcile: declared head moved — re-materializing"
                    );
                    let built = tokio::task::spawn_blocking({
                        let storage_url = ctx.storage_url.clone();
                        let bundle_parent = ctx.bundle_parent.clone();
                        let slug = slug.clone();
                        let declared_hash = declared_hash.clone();
                        let soft_budget_ms = ctx.soft_budget_ms;
                        move || {
                            materialize_and_build(
                                &storage_url,
                                &bundle_parent,
                                &slug,
                                &declared_hash,
                                soft_budget_ms,
                            )
                        }
                    })
                    .await;
                    let outcome = match built {
                        Ok(Ok(new_renderer)) => {
                            // Attestation TOCTOU guard: materialize_server_bundle
                            // did its OWN independent resolve+verify of the bytes.
                            // Re-resolve the declared head now; ONLY if it is
                            // unchanged across the entire materialize window can we
                            // be sure the hash we are about to attest is the one
                            // the bytes verified against. If it moved, discard this
                            // build and converge next tick — never attest a hash we
                            // did not verify against (the exact drift this feature
                            // exists to catch).
                            let post = resolve_declared(&ctx.storage_url, &slug).await;
                            match post {
                                Ok(ref h2) if h2 == &declared_hash => {
                                    let old = self.swap_renderer(
                                        &slug,
                                        new_renderer,
                                        &declared_hash,
                                        is_default,
                                    );
                                    // Clear the SSR render-result cache so no
                                    // pre-swap HTML leaks (keys are `ssr-<hash>`
                                    // and not slug-recoverable, so clear the whole
                                    // SSR namespace — a superset, safe: it only
                                    // forces re-renders).
                                    let cleared = cache.invalidate_pattern("ssr-");
                                    tracing::info!(
                                        target: "doorway::ssr",
                                        slug = %slug,
                                        head = %declared_hash,
                                        cache_entries_cleared = cleared,
                                        "bundle reconcile: hot-swapped to declared head"
                                    );
                                    // Retire the old isolate via the graveyard so
                                    // its blocking Drop (join) never runs on a
                                    // tokio worker — a later reap drops it off the
                                    // runtime once no request still holds it.
                                    if let Some(old) = old {
                                        self.graveyard.bury(old);
                                    }
                                    SlugOutcome {
                                        slug: slug.clone(),
                                        outcome: "refreshed".into(),
                                        server_blob_hash: Some(declared_hash.clone()),
                                        declared_server_blob_hash: Some(declared_hash.clone()),
                                        error: None,
                                    }
                                }
                                other => {
                                    // Head moved during materialize (or the re-read
                                    // failed): the freshly-built renderer may not
                                    // match declared_hash, so we cannot attest it.
                                    // Discard the build (bury — it too owns an
                                    // isolate) and leave the slug Stale; the next
                                    // tick converges to the settled head.
                                    self.graveyard.bury(new_renderer);
                                    let observed = other.ok();
                                    let declared_for_status =
                                        observed.clone().unwrap_or_else(|| declared_hash.clone());
                                    self.mark_stale(&slug, &declared_for_status);
                                    tracing::info!(
                                        target: "doorway::ssr",
                                        slug = %slug,
                                        "bundle reconcile: declared head moved during \
                                         materialize — deferring attestation to next tick"
                                    );
                                    SlugOutcome {
                                        slug: slug.clone(),
                                        outcome: "stale".into(),
                                        server_blob_hash: Some(materialized_hash.clone()),
                                        declared_server_blob_hash: observed
                                            .or(Some(declared_hash.clone())),
                                        error: None,
                                    }
                                }
                            }
                        }
                        Ok(Err(e)) => {
                            tracing::warn!(
                                target: "doorway::ssr",
                                slug = %slug,
                                "bundle reconcile: re-materialize failed — keeping old renderer: {}",
                                e
                            );
                            self.mark_failed(&slug, &e);
                            SlugOutcome {
                                slug: slug.clone(),
                                outcome: "failed".into(),
                                server_blob_hash: Some(materialized_hash.clone()),
                                declared_server_blob_hash: Some(declared_hash.clone()),
                                error: Some(e),
                            }
                        }
                        Err(join_e) => {
                            let e = format!("reconcile task panicked: {join_e}");
                            self.mark_failed(&slug, &e);
                            SlugOutcome {
                                slug: slug.clone(),
                                outcome: "failed".into(),
                                server_blob_hash: Some(materialized_hash.clone()),
                                declared_server_blob_hash: Some(declared_hash.clone()),
                                error: Some(e),
                            }
                        }
                    };
                    // Release the in-progress guard in EVERY terminal arm (the
                    // status was set by the swap/mark helper above; end_refresh
                    // only clears the concurrency marker).
                    self.end_refresh(&slug);
                    outcomes.push(outcome);
                }
            }
        }

        // Reap isolates displaced this pass (and any whose in-flight requests
        // have since drained) off the async runtime — the terminal Drop joins a
        // worker thread and must never run on a tokio worker.
        let graveyard = self.graveyard.clone();
        tokio::task::spawn_blocking(move || {
            graveyard.reap();
        });

        outcomes
    }

    // ── Head-mutation helpers (brief write lock, never across await) ─────────

    /// Record that a slug's declared head matches what we serve.
    fn mark_current(&self, slug: &str, declared: &str) {
        let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
        if let Some(h) = inner.heads.get_mut(slug) {
            h.status = HeadStatus::Current;
            h.declared_server_blob_hash = Some(declared.to_string());
            h.last_error = None;
        }
    }

    /// Try to claim the per-slug refresh guard. Returns `true` if THIS caller
    /// may proceed to re-materialize, `false` if a concurrent reconcile already
    /// holds it (the loser skips). On success also sets status `Refreshing` and
    /// records the declared head. The check-and-set is atomic under the write
    /// lock, so two concurrent callers can never both proceed.
    fn try_begin_refresh(&self, slug: &str, declared: &str) -> bool {
        let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
        if inner.refreshing.contains(slug) {
            return false;
        }
        inner.refreshing.insert(slug.to_string());
        if let Some(h) = inner.heads.get_mut(slug) {
            h.status = HeadStatus::Refreshing;
            h.declared_server_blob_hash = Some(declared.to_string());
        }
        true
    }

    /// Release the per-slug refresh guard. Clears ONLY the in-progress marker —
    /// the terminal status is set by the swap/mark helper. Called in every
    /// terminal arm of the Stale branch so the guard cannot leak.
    fn end_refresh(&self, slug: &str) {
        let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
        inner.refreshing.remove(slug);
    }

    /// Record that a newer declared head exists but is not yet materialized
    /// (e.g. the head moved mid-materialize) — the next tick converges to it.
    fn mark_stale(&self, slug: &str, declared: &str) {
        let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
        if let Some(h) = inner.heads.get_mut(slug) {
            h.status = HeadStatus::Stale;
            h.declared_server_blob_hash = Some(declared.to_string());
        }
    }

    /// Mark a slug's last re-materialize as failed (old renderer retained).
    fn mark_failed(&self, slug: &str, error: &str) {
        let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
        if let Some(h) = inner.heads.get_mut(slug) {
            h.status = HeadStatus::Failed;
            h.last_error = Some(error.to_string());
        }
    }

    /// Swap in a freshly-built renderer for `slug`, updating the head to the new
    /// declared hash and status `Current`. Returns the OLD renderer `Arc` (for
    /// off-runtime teardown), if one was present.
    fn swap_renderer(
        &self,
        slug: &str,
        new_renderer: Arc<dyn elohim_render::Renderer>,
        new_hash: &str,
        is_default: bool,
    ) -> Option<Arc<dyn elohim_render::Renderer>> {
        let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
        let old = inner
            .by_app
            .insert(slug.to_string(), Arc::clone(&new_renderer));
        if is_default {
            inner.default_renderer = Some(new_renderer);
        }
        if let Some(h) = inner.heads.get_mut(slug) {
            h.server_blob_hash = new_hash.to_string();
            h.materialized_at = chrono::Utc::now();
            h.status = HeadStatus::Current;
            h.declared_server_blob_hash = Some(new_hash.to_string());
            h.last_error = None;
        }
        old
    }

    /// Test-only: install an explicit head record. Lets swap/mark transitions be
    /// exercised without a live storage backend or a V8 isolate.
    #[cfg(test)]
    fn insert_head_for_test(&self, head: BundleHead) {
        let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
        inner.heads.insert(head.slug.clone(), head);
    }
}

/// Boot-time head record for a slug whose bundle just materialized.
///
/// TOCTOU-guarded single-resolve attestation. `pre` is the declared
/// `serverBlobHash` resolved BEFORE `materialize_server_bundle` ran; this
/// re-resolves it AFTER and delegates the decision to [`decide_boot_head`]:
/// a `current` head is attested ONLY when the declared head was unchanged
/// across the whole materialize window (so the bytes on disk provably verified
/// against the hash we attest). On drift or resolve failure the slug is returned
/// as an `unresolved` head — NEVER dropped — so `reconcile` retries it each tick
/// and the health surface can distinguish "unresolved" from "absent".
fn boot_head(
    src: &crate::ssr::DoorwayBundleSource,
    slug: &str,
    pre: elohim_render::Result<String>,
) -> BundleHead {
    use elohim_render::BundleSource;
    let post = src.resolve_server_blob_hash(slug);
    let head = decide_boot_head(
        slug,
        pre.map_err(|e| e.to_string()),
        post.map_err(|e| e.to_string()),
    );
    if head.status == HeadStatus::Unresolved {
        tracing::warn!(
            target: "doorway::ssr",
            slug = %slug,
            error = %head.last_error.as_deref().unwrap_or("unknown"),
            "bundle head UNRESOLVED at boot — renderer serving, but attested head \
             not confirmed; registered for reconcile retry"
        );
    }
    head
}

/// Pure boot-head decision (no I/O) — the TOCTOU comparison of the pre- and
/// post-materialize declared-head resolves. Extracted so the attest-only-on-
/// stable and register-unresolved-on-drift arms are unit-testable without a
/// storage backend or a V8 isolate.
///
/// - `pre == post` (both Ok, equal): the declared head held steady across the
///   entire materialize window, so materialize's own internal resolve read the
///   same hash and the on-disk bytes verified against it → attest `current`.
/// - any resolve failed, or `pre != post` (head moved mid-materialize): we
///   cannot prove which hash the served bytes match → `unresolved` head with an
///   empty `server_blob_hash` (unknown until reconcile confirms), retained so
///   reconcile converges.
fn decide_boot_head(
    slug: &str,
    pre: std::result::Result<String, String>,
    post: std::result::Result<String, String>,
) -> BundleHead {
    match (pre, post) {
        (Ok(pre_hash), Ok(post_hash)) if pre_hash == post_hash => BundleHead {
            slug: slug.to_string(),
            server_blob_hash: pre_hash,
            materialized_at: chrono::Utc::now(),
            status: HeadStatus::Current,
            declared_server_blob_hash: None,
            last_error: None,
        },
        (pre_r, post_r) => {
            let err = match (&pre_r, &post_r) {
                (Err(e), _) => format!("declared-head resolve failed at boot: {e}"),
                (_, Err(e)) => format!("declared-head re-resolve failed at boot: {e}"),
                (Ok(a), Ok(b)) => {
                    format!("declared head moved during boot materialize: {a} -> {b}")
                }
            };
            BundleHead {
                slug: slug.to_string(),
                // Unknown: materialize does not return the hash it verified, and
                // the bracketing resolves disagree — leave empty until reconcile
                // materializes + confirms a head.
                server_blob_hash: String::new(),
                materialized_at: chrono::Utc::now(),
                status: HeadStatus::Unresolved,
                // Surface the post-resolve when we HAVE it, so an operator sees
                // the head reconcile will converge toward.
                declared_server_blob_hash: post_r.ok(),
                last_error: Some(err),
            }
        }
    }
}

/// Re-resolve a slug's declared `serverBlobHash` off the async runtime.
/// Returns `Err(msg)` on any failure (storage unreachable, field absent, task
/// panic) — the caller treats every `Err` as the safe-degrade "keep current" arm.
async fn resolve_declared(storage_url: &str, slug: &str) -> std::result::Result<String, String> {
    let storage_url = storage_url.to_string();
    let slug = slug.to_string();
    match tokio::task::spawn_blocking(move || {
        use elohim_render::BundleSource;
        crate::ssr::DoorwayBundleSource::new(storage_url).resolve_server_blob_hash(&slug)
    })
    .await
    {
        Ok(Ok(hash)) => Ok(hash),
        Ok(Err(e)) => Err(e.to_string()),
        Err(e) => Err(format!("declared-head resolve task panicked: {e}")),
    }
}

/// Materialize a slug's declared server bundle into a FRESH directory and build
/// a new renderer over it. Runs on a blocking thread (materialize + isolate
/// construction are blocking). The materialize step verifies the fetched bytes
/// against the declared hash, so a hash mismatch surfaces here as `Err` — the
/// caller then keeps the old renderer.
fn materialize_and_build(
    storage_url: &str,
    bundle_parent: &Path,
    slug: &str,
    declared_hash: &str,
    soft_budget_ms: u64,
) -> std::result::Result<Arc<dyn elohim_render::Renderer>, String> {
    let src = crate::ssr::DoorwayBundleSource::new(storage_url.to_string());

    // Fresh directory keyed by the declared hash so we NEVER overwrite the live
    // bundle the current isolate loaded. `<parent>/.reconcile/<slug>/<hash16>`.
    let short = declared_hash
        .strip_prefix("sha256-")
        .unwrap_or(declared_hash);
    // Panic-proof slice: `get(..16)` yields None (→ full string) if shorter.
    let short = short.get(..16).unwrap_or(short);
    let dir = bundle_parent.join(".reconcile").join(slug).join(short);
    std::fs::create_dir_all(&dir).map_err(|e| format!("reconcile dir create failed: {e}"))?;

    let materialized = elohim_render::materialize_server_bundle(&src, slug, &dir)
        .map_err(|e| format!("materialize failed: {e}"))?;

    let fetcher: Arc<dyn elohim_render::DataFetcher> = {
        let bootstrap_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Arc::new(crate::ssr::ResolverFetcher::new(
            Arc::new(bootstrap_client),
            storage_url.to_string(),
        ))
    };

    let renderer =
        elohim_render::AngularRenderer::with_soft_budget(materialized, fetcher, soft_budget_ms)
            .map_err(|e| format!("renderer construction failed: {e}"))?;
    Ok(Arc::new(renderer) as Arc<dyn elohim_render::Renderer>)
}

/// Decision arm of one reconcile check — pure, so the current/stale/safe-degrade
/// transitions are unit-testable without a storage backend or a V8 isolate.
#[derive(Debug, PartialEq, Eq)]
pub enum ReconcileDecision {
    /// Declared-head read failed — keep serving current, status unchanged.
    Unreachable(String),
    /// Declared head == materialized head — mark current.
    UpToDate(String),
    /// Declared head != materialized head — re-materialize + swap.
    Stale(String),
}

/// Compare the currently-materialized head against a freshly-resolved declared
/// head. `Err` (read failure) is the safe-degrade arm; equal is up-to-date;
/// different is stale (needs converge).
pub fn decide_reconcile(
    materialized: &str,
    declared: std::result::Result<String, String>,
) -> ReconcileDecision {
    match declared {
        Err(e) => ReconcileDecision::Unreachable(e),
        Ok(d) if d == materialized => ReconcileDecision::UpToDate(d),
        Ok(d) => ReconcileDecision::Stale(d),
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

    fn test_head(slug: &str, hash: &str, status: HeadStatus) -> BundleHead {
        BundleHead {
            slug: slug.to_string(),
            server_blob_hash: hash.to_string(),
            materialized_at: chrono::Utc::now(),
            status,
            declared_server_blob_hash: None,
            last_error: None,
        }
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

    // ── decide_reconcile: current / stale / safe-degrade arms ────────────────

    #[test]
    fn decide_reconcile_equal_is_up_to_date() {
        let d = decide_reconcile("sha256-aaa", Ok("sha256-aaa".to_string()));
        assert_eq!(d, ReconcileDecision::UpToDate("sha256-aaa".to_string()));
    }

    #[test]
    fn decide_reconcile_different_is_stale() {
        let d = decide_reconcile("sha256-aaa", Ok("sha256-bbb".to_string()));
        assert_eq!(d, ReconcileDecision::Stale("sha256-bbb".to_string()));
    }

    #[test]
    fn decide_reconcile_read_failure_is_unreachable_safe_degrade() {
        // Storage unreachable / declared-head read failed → never adopt an
        // unverifiable head; keep serving current.
        let d = decide_reconcile("sha256-aaa", Err("storage GET failed".to_string()));
        assert_eq!(
            d,
            ReconcileDecision::Unreachable("storage GET failed".to_string())
        );
    }

    // ── head status transitions (swap / mark helpers, no V8 needed) ──────────

    #[test]
    fn swap_renderer_updates_head_to_current_and_returns_old() {
        let reg = RendererRegistry::with_entries(Some(stub("v1")), Some("app".into()), vec![]);
        reg.insert_head_for_test(test_head("app", "sha256-old", HeadStatus::Stale));

        let old = reg.swap_renderer("app", stub("v2"), "sha256-new", true);
        assert!(old.is_some(), "swap must return the displaced renderer");

        let heads = reg.heads_snapshot();
        assert_eq!(heads.len(), 1);
        assert_eq!(heads[0].server_blob_hash, "sha256-new");
        assert_eq!(heads[0].status, HeadStatus::Current);
        assert_eq!(
            heads[0].declared_server_blob_hash.as_deref(),
            Some("sha256-new")
        );
        assert!(heads[0].last_error.is_none());
    }

    #[test]
    fn mark_failed_retains_error_and_keeps_old_hash() {
        let reg = RendererRegistry::with_entries(Some(stub("v1")), Some("app".into()), vec![]);
        reg.insert_head_for_test(test_head("app", "sha256-old", HeadStatus::Refreshing));

        reg.mark_failed("app", "materialize failed: hash mismatch");

        let heads = reg.heads_snapshot();
        assert_eq!(heads[0].status, HeadStatus::Failed);
        // Old renderer kept — head hash unchanged.
        assert_eq!(heads[0].server_blob_hash, "sha256-old");
        assert_eq!(
            heads[0].last_error.as_deref(),
            Some("materialize failed: hash mismatch")
        );
    }

    #[test]
    fn mark_current_records_declared_and_clears_error() {
        let reg = RendererRegistry::with_entries(Some(stub("v1")), Some("app".into()), vec![]);
        let mut h = test_head("app", "sha256-x", HeadStatus::Failed);
        h.last_error = Some("prior failure".into());
        reg.insert_head_for_test(h);

        reg.mark_current("app", "sha256-x");

        let heads = reg.heads_snapshot();
        assert_eq!(heads[0].status, HeadStatus::Current);
        assert_eq!(
            heads[0].declared_server_blob_hash.as_deref(),
            Some("sha256-x")
        );
        assert!(heads[0].last_error.is_none());
    }

    // ── servedBundleHeads JSON serialization contract (exact camelCase) ──────

    #[test]
    fn heads_json_serialization_shape_exact_fields() {
        let reg = RendererRegistry::with_entries(Some(stub("v1")), Some("app".into()), vec![]);
        let mut h = test_head("app", "sha256-served", HeadStatus::Current);
        h.declared_server_blob_hash = Some("sha256-served".into());
        reg.insert_head_for_test(h);

        let json = reg.heads_json();
        let arr = json.as_array().expect("servedBundleHeads is an array");
        assert_eq!(arr.len(), 1);
        let entry = &arr[0];
        assert_eq!(entry.get("slug").unwrap(), "app");
        assert_eq!(entry.get("serverBlobHash").unwrap(), "sha256-served");
        assert_eq!(entry.get("status").unwrap(), "current");
        assert_eq!(
            entry.get("declaredServerBlobHash").unwrap(),
            "sha256-served"
        );
        // materializedAt is an RFC3339 string.
        let mat = entry.get("materializedAt").unwrap().as_str().unwrap();
        assert!(
            chrono::DateTime::parse_from_rfc3339(mat).is_ok(),
            "materializedAt must be rfc3339; got {mat}"
        );
        // No error key on a healthy head.
        assert!(entry.get("error").is_none());
    }

    #[test]
    fn heads_json_omits_declared_when_unknown_and_includes_error_when_failed() {
        let reg = RendererRegistry::with_entries(Some(stub("v1")), Some("app".into()), vec![]);
        let mut h = test_head("app", "sha256-served", HeadStatus::Failed);
        h.declared_server_blob_hash = None; // never checked yet
        h.last_error = Some("boom".into());
        reg.insert_head_for_test(h);

        let json = reg.heads_json();
        let entry = &json.as_array().unwrap()[0];
        // declaredServerBlobHash omitted when unknown.
        assert!(entry.get("declaredServerBlobHash").is_none());
        // error present when failed.
        assert_eq!(entry.get("error").unwrap(), "boom");
        assert_eq!(entry.get("status").unwrap(), "failed");
    }

    #[test]
    fn head_status_as_str_vocabulary() {
        assert_eq!(HeadStatus::Current.as_str(), "current");
        assert_eq!(HeadStatus::Stale.as_str(), "stale");
        assert_eq!(HeadStatus::Refreshing.as_str(), "refreshing");
        assert_eq!(HeadStatus::Failed.as_str(), "failed");
        assert_eq!(HeadStatus::Unresolved.as_str(), "unresolved");
    }

    // ── boot-head TOCTOU: attest a `current` head ONLY on a stable resolve ────
    // window; NEVER attest a hash we didn't verify the served bytes against.

    #[test]
    fn boot_head_attests_current_only_when_resolve_window_is_stable() {
        // Declared head unchanged across the materialize window (pre == post):
        // materialize's own internal resolve provably read this same hash, so the
        // on-disk bytes verified against it — safe to attest as the served head.
        let head = decide_boot_head(
            "app",
            Ok("sha256-materialized".to_string()),
            Ok("sha256-materialized".to_string()),
        );
        assert_eq!(head.status, HeadStatus::Current);
        assert_eq!(
            head.server_blob_hash, "sha256-materialized",
            "attested head must be the hash whose bytes were materialized"
        );
        assert!(head.last_error.is_none());
    }

    #[test]
    fn boot_head_drift_never_attests_the_unverified_hash() {
        // The declared head MOVED between the pre- and post-materialize resolves.
        // We cannot prove which hash the on-disk bytes verified against, so we
        // must NOT attest a served head — the exact stale-certification this
        // guard exists to prevent. The slug becomes `unresolved` (retained), and
        // crucially its server_blob_hash is NOT set to the drifted post-value.
        let head = decide_boot_head(
            "app",
            Ok("sha256-first".to_string()),
            Ok("sha256-moved".to_string()),
        );
        assert_eq!(head.status, HeadStatus::Unresolved);
        assert_eq!(
            head.server_blob_hash, "",
            "an unconfirmed head must NOT attest any served hash"
        );
        assert_ne!(
            head.server_blob_hash, "sha256-moved",
            "must never certify the hash whose bytes were never materialized"
        );
        // The observed post-value is surfaced only as the declared head to
        // converge toward — never as the served head.
        assert_eq!(
            head.declared_server_blob_hash.as_deref(),
            Some("sha256-moved")
        );
        assert!(head.last_error.is_some());
    }

    // ── failed boot resolve produces an UNRESOLVED head (not absent) so ───────
    // reconcile retries it and the probe/operator can see it.

    #[test]
    fn boot_head_resolve_failure_is_unresolved_not_absent() {
        // Pre-resolve failed (storage blip at boot). Old behavior dropped the
        // slug entirely (returned None), excluding it from reconcile forever and
        // making the CI probe read the absence as a PASS. New behavior: a
        // retained `unresolved` head.
        let head = decide_boot_head(
            "app",
            Err("storage GET failed".to_string()),
            Err("storage GET failed".to_string()),
        );
        assert_eq!(head.status, HeadStatus::Unresolved);
        assert_eq!(head.server_blob_hash, "");
        assert!(head.last_error.is_some());
    }

    #[test]
    fn unresolved_boot_slug_is_retained_and_reconcile_targets_it() {
        // A slug whose boot resolve failed is REGISTERED (not dropped), so it is
        // included in the head table that reconcile iterates …
        let reg = RendererRegistry::with_entries(Some(stub("v1")), Some("app".into()), vec![]);
        let unresolved = decide_boot_head("app", Err("boom".into()), Err("boom".into()));
        reg.insert_head_for_test(unresolved);

        let heads = reg.heads_snapshot();
        assert_eq!(
            heads.len(),
            1,
            "unresolved slug must be present, not absent"
        );
        assert_eq!(heads[0].status, HeadStatus::Unresolved);

        // … and once storage recovers, reconcile's decision for that slug (empty
        // materialized hash vs a freshly-resolved declared head) is Stale — i.e.
        // it re-materializes + swaps, self-healing to a confirmed head on the
        // 300s tick. This is the link that closes the "excluded forever" gap.
        let decision = decide_reconcile(&heads[0].server_blob_hash, Ok("sha256-real".to_string()));
        assert_eq!(
            decision,
            ReconcileDecision::Stale("sha256-real".to_string())
        );
    }

    #[test]
    fn unresolved_head_serializes_distinguishably_from_absent() {
        // The health surface must let the probe/operator tell "unresolved" from
        // "absent": the slug appears with status `unresolved` (not missing).
        let reg = RendererRegistry::with_entries(Some(stub("v1")), Some("app".into()), vec![]);
        reg.insert_head_for_test(decide_boot_head(
            "app",
            Err("boom".into()),
            Err("boom".into()),
        ));

        let json = reg.heads_json();
        let entry = &json.as_array().unwrap()[0];
        assert_eq!(entry.get("slug").unwrap(), "app");
        assert_eq!(entry.get("status").unwrap(), "unresolved");
        // serverBlobHash present but empty — an honest "not yet confirmed",
        // never a fabricated served head.
        assert_eq!(entry.get("serverBlobHash").unwrap(), "");
    }

    #[test]
    fn test_registry_has_reconcile_disabled() {
        // with_entries / empty registries have no declared-head source.
        let reg = RendererRegistry::with_entries(Some(stub("v1")), Some("app".into()), vec![]);
        assert!(!reg.reconcile_enabled());
        assert!(RendererRegistry::empty().heads_snapshot().is_empty());
    }

    #[tokio::test]
    async fn reconcile_is_noop_when_disabled() {
        let reg = RendererRegistry::with_entries(Some(stub("v1")), Some("app".into()), vec![]);
        let cache = crate::cache::ContentCache::default();
        let outcomes = reg.reconcile(&cache).await;
        assert!(outcomes.is_empty(), "disabled reconcile must be a no-op");
    }

    // ── graveyard: displaced isolates never drop on the request path ─────────

    #[test]
    fn graveyard_reaps_only_when_sole_owner_and_drains() {
        let gy = Graveyard::new();

        // Simulate an in-flight request still holding its own Arc clone of a
        // displaced renderer (strong_count == 2 after burying).
        let in_flight = stub("still-rendering");
        gy.bury(Arc::clone(&in_flight));
        // A fully-displaced renderer whose only owner is the graveyard.
        gy.bury(stub("displaced"));
        assert_eq!(gy.len(), 2);

        // First reap: only the sole-owner (count == 1) entry drains; the one an
        // in-flight request still references is retained (its terminal Drop must
        // never run on the request path).
        assert_eq!(gy.reap(), 1);
        assert_eq!(gy.len(), 1);

        // Once the in-flight request releases its clone, the next reap drains
        // it — the terminal Drop runs here (off-runtime by contract), never on
        // the request thread that dropped its clone.
        drop(in_flight);
        assert_eq!(gy.reap(), 1);
        assert_eq!(gy.len(), 0);
        // Idempotent: reaping an empty graveyard is a no-op.
        assert_eq!(gy.reap(), 0);
    }

    // ── in-progress guard: concurrent reconcile passes don't double-enter ────

    #[test]
    fn try_begin_refresh_is_exclusive_until_end_refresh() {
        let reg = RendererRegistry::with_entries(Some(stub("v1")), Some("app".into()), vec![]);
        reg.insert_head_for_test(test_head("app", "sha256-old", HeadStatus::Current));

        // First caller claims the guard and sets Refreshing.
        assert!(reg.try_begin_refresh("app", "sha256-new"));
        assert_eq!(reg.heads_snapshot()[0].status, HeadStatus::Refreshing);
        // A concurrent caller is refused while the first holds it.
        assert!(!reg.try_begin_refresh("app", "sha256-new"));

        // After the first releases, a subsequent pass may claim it again.
        reg.end_refresh("app");
        assert!(reg.try_begin_refresh("app", "sha256-new"));
        reg.end_refresh("app");
    }
}
