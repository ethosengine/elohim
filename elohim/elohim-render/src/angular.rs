//! AngularRenderer — loads main.server.mjs, calls renderApplication(bootstrap, {url}).
//!
//! # Thread model
//!
//! `deno_core::JsRuntime` is `!Send` (it contains `Rc<JsRuntimeState>` internals and
//! a V8 isolate pointer that must not cross thread boundaries). The `Renderer` trait
//! requires `Send + Sync`. These two constraints are bridged by a **dedicated background
//! thread** that owns the `JsRuntime` for its entire lifetime.
//!
//! The background thread runs its own single-threaded Tokio runtime so that
//! `JsRuntime::eval_string` (which is `async`) can drive the deno event loop on the
//! correct OS thread.
//!
//! Communication uses a `std::sync::mpsc::SyncSender<StringWorkItem>` where each item
//! carries the JS driver script and a `tokio::sync::oneshot::Sender<Result<String>>`
//! for the response. This keeps `AngularRenderer: Send + Sync` while the isolate never
//! leaves its owning thread.
//!
//! For the MVP a single isolate handles requests sequentially (channel capacity = 1).
//! A pool of background threads (one isolate each) is the natural scaling step — the
//! channel approach already supports it via a `Vec<SyncSender<...>>` with round-robin.

use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::{mpsc, Arc};

use crate::traced_fetcher::TracingFetcher;
use crate::types::{FetchEvent, RenderTrace};
use crate::{RenderContext, RenderError, RenderOutput, Renderer, Result};

/// Safety ceiling on `ctx.limits.wall_time_ms`, independent of whatever the
/// caller requests (doorway currently asks for 60_000ms to cover a cold-start
/// worst case). The isolate is a SINGLE sequential worker (capacity-1 channel,
/// see `render()` below) — a route that genuinely stalls (idle-waiting on a
/// promise that never settles, not a slow-but-finishing render) occupies that
/// one isolate for the FULL requested wall time, shedding every other request
/// with `RenderError::Busy` for the duration. Measured healthy renders
/// (`examples/render_url.rs`, cold isolate, no JIT warmup) complete in
/// 300–550ms; this ceiling leaves ~20x headroom over that while cutting a
/// stalling route's worst-case isolate-occupancy from 60s to this value.
/// Override with `ELOHIM_RENDER_MAX_WALL_TIME_MS` (e.g. for a slower host or a
/// deliberately content-heavy cold-start scenario that needs more headroom).
/// This bounds impact; it does NOT fix a genuine root-cause stall — see the
/// investigation notes in `genesis/data/timeline/backlog/` for the '/' route
/// class this was introduced for.
const DEFAULT_MAX_WALL_TIME_MS: u64 = 10_000;

fn max_wall_time_ms() -> u64 {
    std::env::var("ELOHIM_RENDER_MAX_WALL_TIME_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MAX_WALL_TIME_MS)
}

/// Scans `dir` (non-recursive — the server bundle's chunks are flat) for a
/// `.mjs` file whose SOURCE TEXT contains every substring in `signatures`.
///
/// This is how the `ELOHIM_RENDER_DEBUG_HOOKS` PendingTasks/Router probes
/// (see [`render`](AngularRenderer::render)) relocate a specific Angular
/// framework chunk across rebuilds: esbuild mints a fresh content-hash
/// filename for every chunk on every build (`chunk-HBGZH5O4.mjs` today, some
/// other hash tomorrow), so hard-coding a filename would silently break the
/// probe on the next `ng build`. Public method/getter names invoked via dot
/// notation elsewhere in the module graph (`hasPendingTasksObservable`,
/// `navigateByUrl`, `initialNavigation`) are NOT mangled by the minifier
/// (only local variable/class names are), so grepping for them is a stable
/// anchor. Returns the matching file's `file://` URL pre-encoded as a JSON
/// string literal (quotes included) ready to splice directly into generated
/// JS; `None` if no chunk matches — callers degrade the probe to a no-op
/// rather than failing the render.
fn find_module_chunk_by_signatures(dir: &std::path::Path, signatures: &[&str]) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("mjs") {
            continue;
        }
        let Ok(contents) = std::fs::read_to_string(&path) else {
            continue;
        };
        if signatures.iter().all(|sig| contents.contains(sig)) {
            if let Ok(url) = url::Url::from_file_path(&path) {
                if let Ok(lit) = serde_json::to_string(url.as_str()) {
                    return Some(lit);
                }
            }
        }
    }
    None
}

/// Builds the `ELOHIM_RENDER_DEBUG_HOOKS` PendingTasks + Router instrumentation
/// block (empty string if the relevant chunk isn't found — a no-op, not a
/// render failure).
///
/// # Why PendingTasks and Router
///
/// Under `provideZonelessChangeDetection()` (elohim-app's config),
/// `ApplicationRef.whenStable()` — which `renderApplication` awaits internally
/// before serializing — resolves only when Angular's `PendingTasks` registry
/// is empty. The two contributors are HttpClient requests and Router
/// navigation: `Router`'s navigation path calls `pendingTasks.add()` before
/// dispatching, and only calls `remove()` once its `events` stream emits
/// `NavigationEnd` / `NavigationCancel` / `NavigationError` for that
/// navigation (Angular's real, un-mangled source — verified by reading the
/// bundle directly). If navigation never reaches one of those terminal
/// events, `remove()` never fires and `whenStable()` hangs forever with NO
/// component constructor having to run — this is the prime suspect for the
/// silent `/` stall recorded in
/// `genesis/data/timeline/backlog/elohim-render-v22-elohim-app-stall.md`.
///
/// # Mechanism
///
/// Neither class is reachable from `globalThis` (Angular resolves them via
/// DI). Instead this dynamically `import()`s the SAME chunk file the main
/// bundle will later import (by resolved `file://` URL, so the ES module
/// cache returns the identical, singleton module record) BEFORE the bundle
/// loads, duck-types the exported class by its distinctive
/// prototype members, and monkey-patches the prototype in place. When
/// Angular's DI later instantiates the (now-patched) class, every instance
/// carries the instrumentation automatically — no bundle file is ever
/// edited.
fn pending_tasks_debug_probe_js(bundle_dir: &std::path::Path) -> String {
    let mut js = String::new();

    if let Some(pt_url) =
        find_module_chunk_by_signatures(bundle_dir, &["hasPendingTasksObservable"])
    {
        js.push_str(&format!(
            r#"
            try {{
                const __ptUrl = {pt_url};
                const __ptNs = await import(__ptUrl);
                let __ptFound = false;
                for (const __ptKey of Object.keys(__ptNs)) {{
                    const __ptCls = __ptNs[__ptKey];
                    if (typeof __ptCls === 'function' && __ptCls.prototype &&
                        Object.getOwnPropertyDescriptor(__ptCls.prototype, 'hasPendingTasks')) {{
                        const __ptProto = __ptCls.prototype;
                        const __ptOrigAdd = __ptProto.add;
                        const __ptOrigRemove = __ptProto.remove;
                        __ptProto.add = function (...args) {{
                            const id = __ptOrigAdd.apply(this, args);
                            globalThis.__eprLastPendingTasks = this;
                            console.log(`[PendingTasks.add] key=${{__ptKey}} id=${{id}} outstanding=${{this.pendingTasks.size}} t=${{Date.now()}}\n${{new Error().stack}}`);
                            return id;
                        }};
                        __ptProto.remove = function (...args) {{
                            console.log(`[PendingTasks.remove] key=${{__ptKey}} id=${{args[0]}} outstanding-before=${{this.pendingTasks.size}} t=${{Date.now()}}`);
                            return __ptOrigRemove.apply(this, args);
                        }};
                        console.log(`[PendingTasks probe] installed on export "${{__ptKey}}" from ${{__ptUrl}}`);
                        __ptFound = true;
                        break;
                    }}
                }}
                if (!__ptFound) {{
                    console.log('[PendingTasks probe] chunk imported but no matching class export found');
                }}
            }} catch (e) {{
                console.log(`[PendingTasks probe FAILED] ${{e && e.stack ? e.stack : e}}`);
            }}
            "#
        ));
    } else {
        js.push_str(
            "console.log('[PendingTasks probe] no chunk in bundle dir matched the signature');\n",
        );
    }

    if let Some(router_url) =
        find_module_chunk_by_signatures(bundle_dir, &["navigateByUrl", "initialNavigation"])
    {
        js.push_str(&format!(
            r#"
            try {{
                const __rtUrl = {router_url};
                const __rtNs = await import(__rtUrl);
                let __rtFound = false;
                for (const __rtKey of Object.keys(__rtNs)) {{
                    const __rtCls = __rtNs[__rtKey];
                    if (typeof __rtCls === 'function' && __rtCls.prototype &&
                        typeof __rtCls.prototype.navigateByUrl === 'function' &&
                        typeof __rtCls.prototype.initialNavigation === 'function') {{
                        const __rtProto = __rtCls.prototype;
                        const __rtOrigNav = __rtProto.navigateByUrl;
                        __rtProto.navigateByUrl = function (...args) {{
                            if (!this.__eprEventsHooked && this.events && typeof this.events.subscribe === 'function') {{
                                this.__eprEventsHooked = true;
                                this.events.subscribe({{
                                    next: (ev) => console.log(`[Router.events] type=${{ev && ev.type}} t=${{Date.now()}} ${{ev && ev.toString ? ev.toString() : ''}}`),
                                    error: (err) => console.log(`[Router.events ERROR] ${{err}}`),
                                }});
                            }}
                            console.log(`[Router.navigateByUrl] url=${{args[0]}} t=${{Date.now()}}`);
                            return __rtOrigNav.apply(this, args);
                        }};
                        console.log(`[Router probe] installed on export "${{__rtKey}}" from ${{__rtUrl}}`);
                        __rtFound = true;
                        break;
                    }}
                }}
                if (!__rtFound) {{
                    console.log('[Router probe] chunk imported but no matching class export found');
                }}
            }} catch (e) {{
                console.log(`[Router probe FAILED] ${{e && e.stack ? e.stack : e}}`);
            }}
            "#
        ));
    } else {
        js.push_str(
            "console.log('[Router probe] no chunk in bundle dir matched the signature');\n",
        );
    }

    js
}

/// A single unit of work sent to the background render thread.
struct StringWorkItem {
    /// The JS expression/IIFE driver to evaluate via `eval_string`.
    script: String,
    /// The per-request `DataFetcher` for this render — swapped into the isolate
    /// before evaluation so each request renders against its own fetcher (e.g.
    /// one carrying the originating user's session credential).
    fetcher: Arc<dyn crate::DataFetcher>,
    /// Channel to return the raw string result (or error) and the fetch
    /// breadcrumbs recorded during this render back to the caller. Events are
    /// returned alongside the result so the caller can assemble the render trace
    /// even when the render itself errored.
    reply: tokio::sync::oneshot::Sender<(Result<String>, Vec<FetchEvent>)>,
}

/// Angular SSR renderer.
///
/// Loads `main.server.mjs` (or any ESM bundle that exports `renderApplication`
/// and a default `bootstrap` function) and calls Angular's SSR API to produce a
/// full HTML document for the requested URL.
///
/// Internally owns a background thread with a single V8 isolate. `render()` is
/// safe to call from any async context and from multiple threads concurrently
/// (requests are serialised by the channel).
pub struct AngularRenderer {
    bundle: PathBuf,
    /// Sender to the background worker thread.
    ///
    /// `SyncSender` is `Send + Sync`, satisfying the `Renderer: Send + Sync` bound
    /// even though the `JsRuntime` on the other end is `!Send`.
    tx: mpsc::SyncSender<StringWorkItem>,
    /// Handle to the background OS thread. Stored so we can join on drop and
    /// surface any worker panic rather than silently losing it.
    worker: Option<std::thread::JoinHandle<()>>,
}

impl AngularRenderer {
    /// Creates a new `AngularRenderer`. Spawns a dedicated OS thread that
    /// owns a V8 isolate. The isolate is initialized with the standard
    /// runtime shims: `console`, `URL` (with `URLSearchParams`),
    /// `TextEncoder`/`TextDecoder`, the `fs` module loader, and a `fetch`
    /// global backed by the supplied `DataFetcher`.
    ///
    /// The construction-time `fetcher` is the bootstrap default. Each call to
    /// [`render`](AngularRenderer::render) swaps in that request's
    /// `RenderContext.data_fetcher` via `JsRuntime::set_fetcher` before evaluating,
    /// so an authenticated render fetches with the originating user's credential
    /// rather than as anonymous (the SSR reach-aware contract).
    ///
    /// # The swap authorizes; it does not isolate
    ///
    /// **One isolate serves every render for the life of this renderer.** The
    /// fetcher is the only thing swapped between them — `globalThis`, the ESM
    /// module map, and the Angular bundle's DI singletons and module-scope
    /// caches all persist. So a per-user credentialed fetcher makes render N+1
    /// *authorized* as user B, but does not stop it from observing whatever
    /// user A's render left in the JS heap. `JsRuntime::set_fetcher` records
    /// that crossing (see its docs and
    /// [`JsRuntime`](crate::runtime::JsRuntime)); closing it requires a cold
    /// start per principal, which this reuse exists precisely to avoid. Verdict
    /// and evidence:
    /// `genesis/data/timeline/backlog/elohim-render-isolate-reuse-trust-boundary.md`.
    ///
    /// # Errors
    ///
    /// Returns `RenderError::ModuleLoad` if the bundle path does not exist or the
    /// background thread cannot be spawned.
    pub fn new(bundle: PathBuf, fetcher: Arc<dyn crate::DataFetcher>) -> Result<Self> {
        Self::with_soft_budget(
            bundle,
            fetcher,
            crate::traced_fetcher::DEFAULT_SOFT_BUDGET_MS,
        )
    }

    /// Like [`AngularRenderer::new`], but with an explicit per-fetch soft budget
    /// (milliseconds). A data fetch outstanding longer than this is recorded as
    /// `Stalled` and rejected so the render falls back fast — the per-peer fetch
    /// SLA whose breach feeds compute-commitment tuning. Consumers thread this
    /// from their render limits; `new` uses [`DEFAULT_SOFT_BUDGET_MS`].
    ///
    /// [`DEFAULT_SOFT_BUDGET_MS`]: crate::traced_fetcher::DEFAULT_SOFT_BUDGET_MS
    pub fn with_soft_budget(
        bundle: PathBuf,
        fetcher: Arc<dyn crate::DataFetcher>,
        soft_budget_ms: u64,
    ) -> Result<Self> {
        if !bundle.exists() {
            return Err(RenderError::ModuleLoad(format!(
                "bundle not found: {}",
                bundle.display()
            )));
        }

        // Bounded channel — capacity 1 for sequential MVP isolate.
        let (tx, rx) = mpsc::sync_channel::<StringWorkItem>(1);

        // Spawn the background thread that owns the JsRuntime.
        let worker = std::thread::Builder::new()
            .name("angular-renderer".into())
            .spawn(move || {
                // Each worker thread runs its own single-threaded Tokio runtime.
                // This is required because JsRuntime::eval_string is async and must
                // be driven on the same OS thread that owns the V8 isolate.
                let local_rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("angular render worker: tokio runtime init failed");

                local_rt.block_on(async move {
                    // JsRuntime is created here — it stays on this thread forever.
                    // with_full_shims() wires console, URL, TextEncoder/TextDecoder,
                    // FsModuleLoader, AND a `fetch` global that dispatches into the
                    // current DataFetcher. Without fetch, Angular SSR bootstrap
                    // hangs forever waiting on HttpClient calls (services like
                    // ConfigService, AuthService, ContentService). The construction-
                    // time `fetcher` is the bootstrap default — every render swaps in
                    // its own per-request fetcher below before evaluating.
                    let mut runtime = crate::runtime::JsRuntime::with_full_shims(fetcher);

                    for StringWorkItem {
                        script,
                        fetcher,
                        reply,
                    } in rx
                    {
                        // Wrap THIS request's fetcher in a fresh TracingFetcher so
                        // every `fetch()` it issues is recorded below the framework
                        // (status/timing/empty classification + soft-deadline stall
                        // detection), then swap it into the isolate. A fresh instance
                        // per render scopes the trace and the offset clock cleanly;
                        // the isolate is sequential so there is no cross-render race.
                        let traced =
                            Arc::new(TracingFetcher::with_soft_budget(fetcher, soft_budget_ms));
                        runtime.set_fetcher(traced.clone());
                        let result = runtime.eval_string(&script).await;
                        let events = traced.take_events();
                        // Ignore send error — caller may have dropped its receiver
                        // (e.g. due to a timeout cancellation).
                        let _ = reply.send((result, events));
                    }
                });
            })
            .map_err(|e| RenderError::ModuleLoad(format!("spawn worker thread: {e}")))?;

        Ok(Self {
            bundle,
            tx,
            worker: Some(worker),
        })
    }
}

#[async_trait]
impl Renderer for AngularRenderer {
    /// Renders `ctx.url` via Angular SSR and returns the full HTML document.
    ///
    /// # Timeout
    ///
    /// The caller-side timeout is `ctx.limits.wall_time_ms`. If the worker does
    /// not reply within this window, `RenderError::Timeout` is returned and the
    /// caller resumes immediately. The worker continues executing in its V8
    /// isolate until the in-flight JS resolves or the isolate OOMs —
    /// V8-level termination (interrupt + `TerminateExecution`) is deferred to
    /// a future task.
    ///
    /// # Load shedding (bounded queue)
    ///
    /// The isolate is sequential (capacity-1 channel). Work is handed off with a
    /// non-blocking `try_send`: if the queue is saturated (a render in flight and
    /// one already buffered behind it), this returns `RenderError::Busy` so the
    /// caller sheds to a fast fallback (CSR shell) rather than queuing behind a
    /// possibly-stuck render. This bounds the queue so a stuck render cannot
    /// back-pressure onto the tokio runtime (2026-06-13 freeze RCA). Pooling more
    /// than one isolate (round-robin over `Vec<SyncSender<…>>`) and V8-level
    /// interrupt are the natural follow-ups.
    async fn render(&self, ctx: RenderContext) -> Result<RenderOutput> {
        // Build a file:// URL for the bundle so dynamic import() can locate it.
        let bundle_url = url::Url::from_file_path(&self.bundle).map_err(|_| {
            RenderError::ModuleLoad(format!(
                "bundle path cannot be converted to a file:// URL \
                 (path must be absolute): {}",
                self.bundle.display()
            ))
        })?;
        let bundle_lit = serde_json::to_string(bundle_url.as_str()).map_err(RenderError::Serde)?;
        let url_lit = serde_json::to_string(&ctx.url).map_err(RenderError::Serde)?;

        // Diagnostic-only instrumentation, OFF by default and never shipped in a
        // production driver: ELOHIM_RENDER_DEBUG_HOOKS=1 installs (a) a
        // console.log heartbeat via Deno.core's own timer facility so a genuine
        // V8 spin vs. an idle-wait can be told apart from wall-clock alone, and
        // (b) a trap on the FIRST assignment to `globalThis.customElements`
        // (Lit's own SSR CustomElementRegistry shim self-installs via
        // `globalThis.customElements ??= new Registry()` the first time a Lit
        // chunk evaluates) that wraps `.define()` / `.whenDefined()` to log
        // ENTER/RESOLVE with a caller stack — the direct test for "some
        // component awaits customElements.whenDefined(tag) for a tag that is
        // never defined server-side, and hangs forever." See the root-cause
        // investigation in genesis/data/timeline/backlog/2026-07-30-*stall*.md.
        let debug_hooks = std::env::var("ELOHIM_RENDER_DEBUG_HOOKS").is_ok();
        let debug_js = if debug_hooks {
            r#"
                (() => {
                    if (typeof Deno === 'undefined' || !Deno.core) return;
                    // V8's default Error.stackTraceLimit (10) truncates every
                    // caller-stack capture below before it reaches the actual
                    // application-level caller (a component/service several
                    // RxJS operators + interceptor-chain frames deep) — every
                    // captured stack in this probe bottoms out at generic
                    // RxJS Subscriber machinery instead of naming who issued
                    // the request. Raise it for the duration of the render.
                    Error.stackTraceLimit = 50;
                    const core = Deno.core;
                    const tick = () => {
                        // If the PendingTasks probe below installed itself and
                        // Angular's DI has instantiated the (patched) class, the
                        // last instance that called add()/remove() is stashed on
                        // globalThis — dump its outstanding task ids on every
                        // beat so a stuck-forever task is visible well before
                        // any fixed T+2s/T+5s checkpoint.
                        const pt = globalThis.__eprLastPendingTasks;
                        const ptInfo = pt ? ` pendingTasks=[${[...pt.pendingTasks].join(',')}]` : '';
                        console.log(`[heartbeat] t=${Date.now()}${ptInfo}`);
                        core.queueUserTimer(1, false, 500, tick);
                    };
                    core.queueUserTimer(1, false, 500, tick);

                    // Wrap setTimeout/setInterval to log EVERY scheduled timer
                    // (delay + caller stack) — names the concrete callback still
                    // outstanding when the render stalls (e.g. a long/huge-delay
                    // timer, a retry backoff, an rxjs `timeout()` operator).
                    const origSetTimeout = globalThis.setTimeout;
                    const origSetInterval = globalThis.setInterval;
                    globalThis.setTimeout = function (cb, delay, ...args) {
                        console.log(`[setTimeout SCHEDULE] delay=${delay}\n${new Error().stack}`);
                        return origSetTimeout(function (...a) {
                            console.log(`[setTimeout FIRE] delay=${delay}`);
                            return cb.apply(this, a);
                        }, delay, ...args);
                    };
                    globalThis.setInterval = function (cb, delay, ...args) {
                        console.log(`[setInterval SCHEDULE] delay=${delay}\n${new Error().stack}`);
                        return origSetInterval(cb, delay, ...args);
                    };

                    let backing = undefined;
                    Object.defineProperty(globalThis, 'customElements', {
                        configurable: true,
                        get() { return backing; },
                        set(registry) {
                            console.log('[customElements] registry installed');
                            try {
                                const origWhenDefined = registry.whenDefined.bind(registry);
                                registry.whenDefined = function (name) {
                                    console.log(`[whenDefined ENTER] ${name}\n${new Error().stack}`);
                                    return origWhenDefined(name).then(
                                        (r) => { console.log(`[whenDefined RESOLVE] ${name}`); return r; },
                                        (e) => { console.log(`[whenDefined REJECT] ${name} ${e}`); throw e; }
                                    );
                                };
                                const origDefine = registry.define.bind(registry);
                                registry.define = function (name, ctor) {
                                    console.log(`[customElements.define] ${name}`);
                                    return origDefine(name, ctor);
                                };
                            } catch (e) {
                                console.log(`[customElements trap install FAILED] ${e}`);
                            }
                            backing = registry;
                        },
                    });
                })();
            "#
        } else {
            ""
        };

        // JS driver: dynamic import the bundle, then call Angular's SSR API.
        // The `await import(...)` is required because eval_string executes in a
        // *script* context (not a module context). Dynamic import() is the bridge
        // from script to ESM module space.
        //
        // Unhandled promise rejection suppression: deno_core tracks unhandled
        // rejections in EventLoopPendingState.has_pending_promise_events. If the
        // handler returns false (default), op_dispatch_exception is called, which
        // causes poll_event_loop to return an error. By registering a handler that
        // returns true (handled), rejections are consumed without throwing. This is
        // needed because Angular services make HTTP calls during SSR (ConfigService,
        // etc.) which hit our stub fetch and produce rejected Promises that propagate
        // up the RxJS chain without a subscriber-level error handler.
        // Diagnostic-only: under ELOHIM_RENDER_DEBUG_HOOKS, LOG every unhandled
        // rejection (reason + a best-effort stack) before still suppressing it —
        // behavior is unchanged (still returns true/handled), but a rejection
        // that today vanishes silently (and may be leaving Angular-internal
        // bookkeeping, e.g. a PendingTask, forever unresolved) becomes visible.
        let rejection_handler_js = if debug_hooks {
            r#"Deno.core.setUnhandledPromiseRejectionHandler((promise, reason) => {
                console.log(`[unhandled rejection] ${reason && reason.stack ? reason.stack : reason}`);
                return true;
            });"#
        } else {
            "Deno.core.setUnhandledPromiseRejectionHandler(() => true);"
        };

        // Diagnostic-only, under ELOHIM_RENDER_DEBUG_HOOKS: reflection-based
        // PendingTasks + Router instrumentation — see
        // `pending_tasks_debug_probe_js` for the full rationale. Must run
        // BEFORE the bundle's own module graph is imported below so the
        // prototype patches are in place before Angular's DI instantiates
        // these classes. Empty string (no-op) when disabled or when the
        // bundle's directory doesn't contain a matching chunk.
        let pending_tasks_probe_js = if debug_hooks {
            let bundle_dir = self
                .bundle
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."));
            pending_tasks_debug_probe_js(bundle_dir)
        } else {
            String::new()
        };

        let driver = format!(
            r#"(async () => {{
                // Suppress unhandled promise rejections — SSR fetch failures are
                // expected (our fetch shim rejects) and must not block the event loop.
                // Without this, op_dispatch_exception is called on each unhandled
                // rejection, which causes RenderError::Panic instead of rendering.
                if (typeof Deno !== 'undefined' && Deno.core && Deno.core.setUnhandledPromiseRejectionHandler) {{
                    {rejection_handler_js}
                }}
                {debug_js}
                {pending_tasks_probe_js}
                // Eagerly evaluate the crypto shim BEFORE the bundle graph. The
                // bundle bundles `ws` (CommonJS), which reaches for a synchronous
                // `require("crypto")` at module-init; the module shim's require
                // serves that from the global the crypto shim registers on eval
                // (src/shim/node_crypto.js). The bundle's own static crypto import
                // evaluates LATER in the graph than `ws`, so without this pre-warm
                // the require would fire before crypto registers. `node:crypto`
                // resolves to the SAME `elohim-builtin:crypto` module instance the
                // bundle imports, so this is a one-time pre-eval, not a duplicate.
                await import('node:crypto');
                const mod = await import({bundle_lit});
                const html = await mod.renderApplication(mod.default, {{ url: {url_lit} }});
                return html;
            }})()"#
        );

        // Hand the work item to the background render thread WITHOUT blocking.
        //
        // The isolate is sequential (capacity-1 channel). A blocking `send` would
        // park the caller until channel capacity frees — and if the in-flight
        // render is stuck (e.g. a JS infinite loop the wall-time timeout cannot
        // free, since the worker keeps running), that block back-pressures onto
        // the tokio runtime. That is a load-bearing contributor to the 2026-06-13
        // freeze. Instead we `try_send`: if the queue is saturated, shed this
        // request with `RenderError::Busy` so the caller falls back fast (CSR
        // shell) rather than queuing behind a possibly-stuck render.
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let work_item = StringWorkItem {
            script: driver,
            // Render against THIS request's fetcher (carries the originating user's
            // session credential for reach-aware SSR), not a fixed construction-time
            // fetcher — the worker swaps it into the isolate before evaluating.
            fetcher: Arc::clone(&ctx.data_fetcher),
            reply: reply_tx,
        };
        self.tx.try_send(work_item).map_err(|e| match e {
            mpsc::TrySendError::Full(_) => RenderError::Busy,
            mpsc::TrySendError::Disconnected(_) => {
                RenderError::Panic("angular render worker: channel closed".into())
            }
        })?;

        // Await the result from the background thread, subject to wall-time limit.
        // A hard-timeout here is the backstop for a true hang (e.g. an infinite
        // loop in JS); the per-fetch soft-deadline already converts a stalled
        // fetch into a fast-completing render below this limit.
        //
        // The effective limit is the caller's request CLAMPED to a safety
        // ceiling (see `max_wall_time_ms` above) — the isolate is a shared,
        // sequential resource, so no single caller gets to hold it hostage
        // for longer than the ceiling regardless of what it asks for.
        let effective_wall_time_ms = ctx.limits.wall_time_ms.min(max_wall_time_ms());
        if effective_wall_time_ms < ctx.limits.wall_time_ms {
            tracing::debug!(
                target: "elohim_render::angular",
                requested_ms = ctx.limits.wall_time_ms,
                effective_ms = effective_wall_time_ms,
                "clamped RenderLimits::wall_time_ms to the elohim-render safety ceiling"
            );
        }
        let started = std::time::Instant::now();
        let (render_result, fetch_events) = tokio::time::timeout(
            std::time::Duration::from_millis(effective_wall_time_ms),
            reply_rx,
        )
        .await
        .map_err(|_| RenderError::Timeout {
            limit_ms: effective_wall_time_ms,
        })?
        .map_err(|_| RenderError::Panic("angular render worker: reply channel dropped".into()))?;

        // The render completed within the wall-time limit. Propagate a render
        // error if the JS threw; otherwise assemble the trace from the recorded
        // fetch breadcrumbs. `trace_id` is left empty here — the consumer stamps
        // its `X-Observation-Id` correlation token onto the trace.
        let html = render_result?;
        let wall_ms = started.elapsed().as_millis() as u64;
        let terminal = RenderTrace::classify(&fetch_events, false);

        if html.len() > ctx.limits.max_output_bytes {
            return Err(RenderError::OutputTooLarge {
                limit_bytes: ctx.limits.max_output_bytes,
                actual_bytes: html.len(),
            });
        }

        Ok(RenderOutput {
            html,
            status: 200,
            headers: vec![("content-type".into(), "text/html; charset=utf-8".into())],
            fetched_inputs: vec![],
            trace: RenderTrace {
                trace_id: String::new(),
                fetches: fetch_events,
                terminal,
                wall_ms,
            },
        })
    }
}

impl Drop for AngularRenderer {
    fn drop(&mut self) {
        // Close our send half by replacing it with a freshly-closed sender,
        // then dropping the original. This signals the worker's rx iterator to
        // exit (it returns `Err(RecvError)` once all senders are gone), which
        // lets the worker's `for item in rx` loop terminate cleanly.
        let (closed_tx, _closed_rx) = mpsc::sync_channel(0);
        // _closed_rx drops here, so closed_tx is already disconnected on the
        // receive side — but that doesn't matter. What matters is that we
        // drop the *original* tx so the worker's rx sees the disconnect.
        let original_tx = std::mem::replace(&mut self.tx, closed_tx);
        drop(original_tx);

        // Wait for the worker to finish its current render and tear down V8.
        // Joining here surfaces any worker panic so it isn't silently lost.
        if let Some(handle) = self.worker.take() {
            if let Err(panic) = handle.join() {
                let msg = panic
                    .downcast_ref::<&'static str>()
                    .map(|s| (*s).to_string())
                    .or_else(|| panic.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "<unknown panic>".into());
                tracing::error!(
                    target: "elohim_render::angular",
                    "AngularRenderer worker panicked: {msg}"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RenderError;

    /// The load-shedding invariant (2026-06-13 freeze): once the capacity-1 queue
    /// is saturated (one item buffered while no consumer drains), `try_send` must
    /// return `Full`, and the renderer maps that to `RenderError::Busy` — a fast
    /// shed, NOT a block. This exercises the exact mapping `render()` applies
    /// without booting a V8 isolate (which needs a real bundle).
    #[test]
    fn saturated_queue_sheds_with_busy_not_block() {
        // No receiver loop is draining — model a worker stuck on an in-flight
        // render. Capacity 1 means the first try_send buffers, the next is Full.
        let (tx, _rx) = mpsc::sync_channel::<u32>(1);

        // First send buffers into the single slot.
        assert!(tx.try_send(1).is_ok(), "first item fills the single slot");

        // Second send finds the slot full — this is the shed point.
        let err = tx.try_send(2).expect_err("saturated queue must reject");
        let mapped = match err {
            mpsc::TrySendError::Full(_) => RenderError::Busy,
            mpsc::TrySendError::Disconnected(_) => RenderError::Panic("channel closed".into()),
        };
        assert!(
            matches!(mapped, RenderError::Busy),
            "a full queue maps to Busy (fast fallback), never a block"
        );
    }

    /// A disconnected worker (the rx half dropped) must map to `Panic`, not
    /// `Busy` — the two failure modes are distinct and the caller treats them
    /// differently (Busy → fall back fast; Panic → genuine error).
    #[test]
    fn disconnected_worker_maps_to_panic_not_busy() {
        let (tx, rx) = mpsc::sync_channel::<u32>(1);
        drop(rx); // worker gone

        let err = tx
            .try_send(1)
            .expect_err("disconnected channel must reject");
        let mapped = match err {
            mpsc::TrySendError::Full(_) => RenderError::Busy,
            mpsc::TrySendError::Disconnected(_) => RenderError::Panic("channel closed".into()),
        };
        assert!(
            matches!(mapped, RenderError::Panic(_)),
            "a disconnected worker is a Panic, not a Busy shed"
        );
    }
}
