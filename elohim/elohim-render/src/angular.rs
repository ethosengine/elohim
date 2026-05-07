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
use std::sync::mpsc;

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
}

impl AngularRenderer {
    /// Create a new renderer backed by `bundle`.
    ///
    /// Validates that the bundle path exists, then spawns the background thread
    /// and initialises the V8 isolate with all stdlib shims (console, URL,
    /// TextEncoder, fetch).
    ///
    /// # Errors
    ///
    /// Returns `RenderError::ModuleLoad` if the bundle path does not exist or the
    /// background thread cannot be spawned.
    pub async fn new(bundle: PathBuf) -> Result<Self> {
        if !bundle.exists() {
            return Err(RenderError::ModuleLoad(format!(
                "bundle not found: {}",
                bundle.display()
            )));
        }

        // Bounded channel — capacity 1 for sequential MVP isolate.
        let (tx, rx) = mpsc::sync_channel::<StringWorkItem>(1);

        // Spawn the background thread that owns the JsRuntime.
        std::thread::Builder::new()
            .name("angular-render-worker".into())
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
                    // with_fs_loader gives us dynamic import() support. The fetch
                    // shim is not connected here (MVP); if Angular tries to call
                    // fetch() during SSR it will hit a missing-global error that
                    // surfaces as RenderError::Panic.
                    let mut runtime = crate::runtime::JsRuntime::with_fs_loader();

                    for StringWorkItem { script, reply } in rx {
                        let result = runtime.eval_string(&script).await;
                        // Ignore send error — caller may have dropped its receiver
                        // (e.g. due to a timeout cancellation).
                        let _ = reply.send(result);
                    }
                });
            })
            .map_err(|e| RenderError::Panic(format!("angular render worker: spawn failed: {e}")))?;

        Ok(Self { bundle, tx })
    }
}

#[async_trait]
impl Renderer for AngularRenderer {
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
        let driver = format!(
            r#"(async () => {{
                const mod = await import({bundle_lit});
                const html = await mod.renderApplication(mod.default, {{ url: {url_lit} }});
                return html;
            }})()"#
        );

        // Send the driver to the background thread and await the response.
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();

        self.tx
            .send(StringWorkItem {
                script: driver,
                reply: reply_tx,
            })
            .map_err(|_| RenderError::Panic("angular render worker: channel closed".into()))?;

        // Await the result from the background thread.
        let html = reply_rx.await.map_err(|_| {
            RenderError::Panic("angular render worker: reply channel dropped".into())
        })??;

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
