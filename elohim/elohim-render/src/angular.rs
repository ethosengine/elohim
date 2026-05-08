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

use crate::{RenderContext, RenderError, RenderOutput, Renderer, Result};

/// A single unit of work sent to the background render thread.
struct StringWorkItem {
    /// The JS expression/IIFE driver to evaluate via `eval_string`.
    script: String,
    /// Channel to return the raw string result (or error) back to the caller.
    reply: tokio::sync::oneshot::Sender<Result<String>>,
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
    /// The fetcher is fixed at construction time. Per-request `RenderContext.
    /// data_fetcher` is currently ignored by this renderer — the constructor's
    /// fetcher is used for every render. This matches the doorway use case
    /// where one renderer instance always forwards to a single elohim-storage
    /// endpoint. Per-request DataFetcher swapping via OpState is a future
    /// enhancement (Task 14+).
    ///
    /// # Errors
    ///
    /// Returns `RenderError::ModuleLoad` if the bundle path does not exist or the
    /// background thread cannot be spawned.
    pub fn new(bundle: PathBuf, fetcher: Arc<dyn crate::DataFetcher>) -> Result<Self> {
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
                    // supplied DataFetcher. Without fetch, Angular SSR bootstrap
                    // hangs forever waiting on HttpClient calls (services like
                    // ConfigService, AuthService, ContentService).
                    let mut runtime = crate::runtime::JsRuntime::with_full_shims(fetcher);

                    for StringWorkItem { script, reply } in rx {
                        let result = runtime.eval_string(&script).await;
                        // Ignore send error — caller may have dropped its receiver
                        // (e.g. due to a timeout cancellation).
                        let _ = reply.send(result);
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
    /// a future task. The next work item sent to the worker will block until
    /// the current render completes, so sustained timeout storms will exhaust the
    /// channel queue. Mitigation (isolate interrupt) is tracked as Task 14+.
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
        let driver = format!(
            r#"(async () => {{
                // Suppress unhandled promise rejections — SSR fetch failures are
                // expected (our fetch shim rejects) and must not block the event loop.
                // Without this, op_dispatch_exception is called on each unhandled
                // rejection, which causes RenderError::Panic instead of rendering.
                if (typeof Deno !== 'undefined' && Deno.core && Deno.core.setUnhandledPromiseRejectionHandler) {{
                    Deno.core.setUnhandledPromiseRejectionHandler(() => true);
                }}
                const mod = await import({bundle_lit});
                const html = await mod.renderApplication(mod.default, {{ url: {url_lit} }});
                return html;
            }})()"#
        );

        // Send the work item to the background thread.
        // SyncSender::send is a blocking call and must not run on a tokio worker
        // thread directly — wrap it in spawn_blocking so tokio can schedule other
        // tasks while we wait for channel capacity.
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let tx = self.tx.clone();
        let work_item = StringWorkItem {
            script: driver,
            reply: reply_tx,
        };
        tokio::task::spawn_blocking(move || {
            tx.send(work_item)
                .map_err(|_| RenderError::Panic("angular render worker: channel closed".into()))
        })
        .await
        .map_err(|_| RenderError::Panic("spawn_blocking join failed".into()))??;

        // Await the result from the background thread, subject to wall-time limit.
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(ctx.limits.wall_time_ms),
            reply_rx,
        )
        .await
        .map_err(|_| RenderError::Timeout {
            limit_ms: ctx.limits.wall_time_ms,
        })?
        .map_err(|_| RenderError::Panic("angular render worker: reply channel dropped".into()))??;

        let html = result;

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
