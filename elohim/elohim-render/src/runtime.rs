//! Thin wrapper around deno_core::JsRuntime for V8 isolate lifecycle.
//!
//! Exposes a minimal synchronous eval surface used by AngularRenderer and
//! other consumers. Module loading, stdlib shims, and fetch dispatch are added
//! in subsequent tasks.

use std::path::Path;
use std::rc::Rc;

use deno_core::{
    v8, FsModuleLoader, JsRuntime as DenoJsRuntime, ModuleSpecifier, PollEventLoopOptions,
    RuntimeOptions,
};

use crate::shim::loader::NodeShimLoader;
use crate::shim::node_buffer::node_buffer_ext;
use crate::shim::node_crypto::node_crypto_ext;
use crate::shim::node_globals::node_globals_ext;
use crate::shim::web_api::web_api_ext;
use crate::shim::{console::console_ext, text::text_ext, url::url_ext};

use crate::{RenderError, Result};

/// A V8 isolate managed by deno_core.
///
/// `JsRuntime::new()` boots the isolate with no module loader — suitable for
/// `eval_string` only. `JsRuntime::with_fs_loader()` adds `FsModuleLoader`
/// so that `render_via_module` can load ESM files from the filesystem.
///
/// # Trust contract: this isolate is reused across renders
///
/// One `JsRuntime` serves many sequential renders (see
/// [`crate::angular::AngularRenderer`], which owns exactly one for the life of
/// its worker thread). Only the [`DataFetcher`](crate::DataFetcher) is swapped
/// between renders — via [`set_fetcher`](Self::set_fetcher). **Nothing resets
/// the JS heap.** `globalThis`, the ESM module map, Angular's DI singletons and
/// every module-scope cache in the server bundle survive from one render into
/// the next, by design: re-evaluating a ~51 MB bundle graph per render is the
/// cold start this reuse exists to avoid.
///
/// That is safe exactly while every render shares one trust level, and unsafe
/// the moment two renders run under different principals. `set_fetcher` tracks
/// which of those two worlds this isolate is in; see its docs and
/// [`isolate_hosted_principal_fetcher`](Self::isolate_hosted_principal_fetcher).
///
/// deno_core 0.339 offers no way to make this cheap and safe: multi-realm
/// support is gone from the public API (`JsRealm` is `pub(crate)`,
/// `JsRuntime::create_realm` does not exist, and the exported
/// `CreateRealmOptions` is a vestige with no consumer), so "fresh `globalThis`
/// per render at near-zero cost" is not purchasable here. The verdict and its
/// evidence live in
/// `genesis/data/timeline/backlog/elohim-render-isolate-reuse-trust-boundary.md`.
pub struct JsRuntime {
    inner: DenoJsRuntime,
    has_fs_loader: bool,
    /// Sticky: set once any [`FetcherTrust::Principal`] fetcher has served a
    /// render on this isolate. Never cleared — nothing resets the JS heap, so
    /// once a principal's render has run here, its residue is present for
    /// every subsequent render.
    hosted_principal_fetcher: bool,
    /// Warn-once latch so a busy authenticated doorway emits one line per
    /// isolate rather than one per render.
    reported_principal_reuse: bool,
}

impl JsRuntime {
    /// Boot a new V8 isolate with default runtime options.
    ///
    /// This runtime does **not** have a module loader. Calling
    /// `render_via_module` on it will return `RenderError::ModuleLoad`.
    /// Use `JsRuntime::with_fs_loader()` when module loading is required.
    pub fn new() -> Self {
        let inner = DenoJsRuntime::new(RuntimeOptions {
            ..Default::default()
        });
        Self::wrap(inner, false)
    }

    /// Common tail of every constructor: a freshly booted isolate has hosted no
    /// fetcher at all, so its trust bookkeeping starts clean.
    fn wrap(inner: DenoJsRuntime, has_fs_loader: bool) -> Self {
        Self {
            inner,
            has_fs_loader,
            hosted_principal_fetcher: false,
            reported_principal_reuse: false,
        }
    }

    /// Boot a new V8 isolate with `FsModuleLoader` enabled.
    ///
    /// Required for `render_via_module`. The filesystem loader reads `.mjs` /
    /// `.js` files synchronously from disk on first load; subsequent imports
    /// are served from V8's module map cache.
    pub fn with_fs_loader() -> Self {
        let inner = DenoJsRuntime::new(RuntimeOptions {
            module_loader: Some(Rc::new(FsModuleLoader)),
            ..Default::default()
        });
        Self::wrap(inner, true)
    }

    /// Boot a new V8 isolate with console, URL, and TextEncoder/TextDecoder shims.
    ///
    /// `console.*` calls dispatch into Rust's `tracing` system at the
    /// appropriate level (info/warn/error). `URL` is a minimal WHATWG-compatible
    /// JS implementation. `TextEncoder`/`TextDecoder` are implemented on top of
    /// deno_core's `op_encode`/`op_decode` builtins. None of these are available
    /// in a bare deno_core runtime without a snapshot; the extensions inject them.
    ///
    /// Also injects the runtime globals the Node-targeted Angular bundle expects
    /// (`node_globals_ext`): `process` (nextTick, env, versions, platform, ...),
    /// the timer family (`setTimeout`/`setInterval`/`setImmediate` + clears),
    /// `performance` (op-backed monotonic `now()` + a minimal User Timing store),
    /// and the `global` alias. See [`crate::shim::node_globals`]. `node_buffer_ext`
    /// installs the global `Buffer` (a real `Uint8Array` subclass) the bundled
    /// `ws`/multiformats code touches at module-eval time; see
    /// [`crate::shim::node_buffer`].
    ///
    /// Includes the [`NodeShimLoader`] so that `render_via_module` works from the
    /// same runtime instance AND the bundle's bare Node-builtin imports (`crypto`
    /// / `node:crypto`) resolve to injected shims instead of panicking; the
    /// `node_crypto_ext` op backs the crypto shim's `createHash`.
    pub fn with_shims() -> Self {
        let inner = DenoJsRuntime::new(RuntimeOptions {
            module_loader: Some(Rc::new(NodeShimLoader::new())),
            extensions: vec![
                console_ext::init_ops_and_esm(),
                url_ext::init_ops_and_esm(),
                text_ext::init_ops_and_esm(),
                node_globals_ext::init_ops_and_esm(),
                node_buffer_ext::init_ops_and_esm(),
                web_api_ext::init_ops_and_esm(),
                node_crypto_ext::init_ops_and_esm(),
            ],
            ..Default::default()
        });
        Self::wrap(inner, true)
    }

    /// Boot a new V8 isolate with all shims plus a `fetch` global backed by
    /// the provided [`DataFetcher`].
    ///
    /// The `fetch(url, init?)` global dispatches to `fetcher.fetch(...)` via
    /// an async deno_core op. The runtime drives the event loop when
    /// `eval_string` receives a Promise result, so callers can `await fetch()`
    /// inside an async IIFE evaluated through `eval_string`.
    ///
    /// Also injects the runtime globals the Node-targeted Angular bundle expects
    /// (`node_globals_ext`): `process`, the timer family, `performance`, and the
    /// `global` alias. See [`crate::shim::node_globals`]. `node_buffer_ext`
    /// installs the global `Buffer` (a real `Uint8Array` subclass); see
    /// [`crate::shim::node_buffer`].
    ///
    /// Includes the [`NodeShimLoader`] so that `render_via_module` works from the
    /// same runtime instance AND the Angular server bundle's bare Node-builtin
    /// imports (`crypto` / `node:crypto`) resolve to injected shims instead of
    /// panicking; the `node_crypto_ext` op backs the crypto shim's `createHash`.
    pub fn with_full_shims(fetcher: std::sync::Arc<dyn crate::DataFetcher>) -> Self {
        use crate::shim::fetch::{fetch_ext, FetcherHandle};
        let inner = DenoJsRuntime::new(RuntimeOptions {
            module_loader: Some(Rc::new(NodeShimLoader::new())),
            extensions: vec![
                console_ext::init_ops_and_esm(),
                url_ext::init_ops_and_esm(),
                text_ext::init_ops_and_esm(),
                node_globals_ext::init_ops_and_esm(),
                node_buffer_ext::init_ops_and_esm(),
                // MUST precede fetch_ext: fetch.js builds a `Response` and reads
                // `Headers` from the globals web_api.js installs.
                web_api_ext::init_ops_and_esm(),
                fetch_ext::init_ops_and_esm(FetcherHandle(fetcher)),
                node_crypto_ext::init_ops_and_esm(),
            ],
            ..Default::default()
        });
        Self::wrap(inner, true)
    }

    /// Replace the `fetch` global's backing [`DataFetcher`](crate::DataFetcher)
    /// for all subsequent fetches on this isolate.
    ///
    /// `op_fetch` reads the `FetcherHandle` from `OpState` on every call, so
    /// `put`-ing a new handle here redirects every fetch issued by the next
    /// render. This is the per-request fetcher swap: a reused isolate renders
    /// each request against that request's own fetcher (carrying, e.g., the
    /// originating user's session credential) rather than a fixed construction-
    /// time fetcher. No-op if the runtime was built without the fetch shim.
    ///
    /// # This swap does NOT reset the isolate — read before relying on it
    ///
    /// **The fetcher is the only thing that changes.** Swapping in a different
    /// principal's fetcher does not clear `globalThis`, the ESM module map,
    /// Angular's DI singletons, or any module-scope cache in the server bundle;
    /// all of it carries over from the previous render. A per-user credentialed
    /// fetcher therefore does not make a render *isolated* — it only makes it
    /// *authorized*. Those are different properties and this method provides
    /// only the second.
    ///
    /// Consequently: whenever a [`FetcherTrust::Principal`] fetcher renders
    /// here, every render that follows on this isolate is running on top of
    /// that principal's residue. This method records that as a sticky fact
    /// ([`isolate_hosted_principal_fetcher`](Self::isolate_hosted_principal_fetcher))
    /// and emits one `WARN` on `elohim_render::trust` the first time a render
    /// follows a principal render, so the crossing is visible in logs rather
    /// than assumed away in a comment.
    ///
    /// A caller that must guarantee isolation between principals cannot get it
    /// from this method; it has to drop the whole `JsRuntime` and pay a cold
    /// start. See the type-level docs on [`JsRuntime`] and the backlog record
    /// `genesis/data/timeline/backlog/elohim-render-isolate-reuse-trust-boundary.md`.
    ///
    /// [`FetcherTrust::Principal`]: crate::FetcherTrust::Principal
    pub fn set_fetcher(&mut self, fetcher: std::sync::Arc<dyn crate::DataFetcher>) {
        use crate::shim::fetch::FetcherHandle;
        self.note_isolate_trust_transition(fetcher.trust_scope());
        self.inner
            .op_state()
            .borrow_mut()
            .put(FetcherHandle(fetcher));
    }

    /// Whether a [`FetcherTrust::Principal`](crate::FetcherTrust::Principal)
    /// fetcher has ever served a render on this isolate.
    ///
    /// Sticky and deliberately conservative: it never returns to `false`,
    /// because nothing in this runtime resets the JS heap. Once true, treat the
    /// isolate as carrying that principal's residue for the rest of its life.
    pub fn isolate_hosted_principal_fetcher(&self) -> bool {
        self.hosted_principal_fetcher
    }

    /// Record the trust scope of the fetcher about to serve the next render,
    /// and surface a trust-boundary crossing the first time one occurs.
    ///
    /// The rule is intentionally conservative: any render that *follows* a
    /// principal render on this isolate is a crossing, whatever its own trust
    /// scope — an ambient render after a principal render can still observe
    /// that principal's data through leftover JS state, and a second principal
    /// render obviously can.
    fn note_isolate_trust_transition(&mut self, incoming: crate::FetcherTrust) {
        if self.hosted_principal_fetcher && !self.reported_principal_reuse {
            self.reported_principal_reuse = true;
            tracing::warn!(
                target: "elohim_render::trust",
                incoming_trust = ?incoming,
                "V8 isolate reuse crossed a trust boundary: a render is starting on an \
                 isolate that has already served a credentialed (FetcherTrust::Principal) \
                 render. Swapping the fetcher does not reset globalThis, the module map, \
                 or Angular DI singletons, so the previous principal's residue is live in \
                 this render. Isolating principals requires dropping the JsRuntime (cold \
                 start); deno_core 0.339 exposes no realm API to do it cheaply."
            );
        }
        if incoming == crate::FetcherTrust::Principal {
            self.hosted_principal_fetcher = true;
        }
    }

    /// Evaluate a JS expression and return its `toString()`.
    ///
    /// Requires `&mut self` because V8 operations require exclusive access
    /// to the isolate.
    ///
    /// If the expression evaluates to a `Promise` (e.g. an async IIFE), the
    /// event loop is driven until the promise settles, and the resolved value's
    /// `toString()` is returned. This is necessary for `fetch(...)` and other
    /// async ops registered via extensions.
    ///
    /// Drives the V8 event loop via `with_event_loop_promise` so that
    /// async expressions (Promises, awaited fetches) resolve before
    /// returning. Sync values pass through with a no-op event-loop spin.
    ///
    /// # Error mapping
    ///
    /// Both compile-time syntax errors and runtime JS exceptions
    /// (`throw new Error(...)`) map to `RenderError::ModuleLoad` with
    /// the exception message preserved via deno_core's TryCatch handling.
    /// Operators looking for the actual cause should read the
    /// `RenderError::ModuleLoad(msg)` payload, not just the variant.
    ///
    /// `RenderError::Panic` is reserved for failures driving the
    /// event loop (op panics, unhandled promise rejections from the
    /// runtime layer) and for `to_string()` failures on the resolved value.
    ///
    /// Semantic differentiation between syntax errors and thrown exceptions
    /// (via stack-frame inspection on `JsError`) is deferred to a future
    /// refinement — the current mapping is intentional and documented here.
    pub async fn eval_string(&mut self, source: &str) -> Result<String> {
        // execute_script returns a Global<Value>. For a Promise result (e.g.
        // an async IIFE), we must drive the event loop before reading the value.
        let global_val = self
            .inner
            .execute_script("<eval>", source.to_string())
            .map_err(|e| RenderError::ModuleLoad(e.to_string()))?;

        // resolve() creates an RcPromiseFuture. For non-Promise values it
        // resolves immediately; for Promises it registers a V8 callback and
        // resolves when the promise settles. with_event_loop_promise drives the
        // event loop concurrently so async ops (fetch, timers) can complete.
        let resolve_fut = self.inner.resolve(global_val);
        let resolved = self
            .inner
            .with_event_loop_promise(resolve_fut, PollEventLoopOptions::default())
            .await
            .map_err(|e| RenderError::Panic(e.to_string()))?;

        // Convert the final value to a Rust String.
        let scope = &mut self.inner.handle_scope();
        let local = v8::Local::new(scope, &resolved);
        let s = local
            .to_string(scope)
            .ok_or_else(|| RenderError::Panic("to_string failed".into()))?;
        Ok(s.to_rust_string_lossy(scope))
    }

    /// Load an ESM module from `module_path`, call its `render(url)` export,
    /// and return the resulting HTML string.
    ///
    /// # Module contract
    ///
    /// The file at `module_path` must be an ESM module that exports a function
    /// named `render` with the signature `(url: string) -> string`. Any other
    /// shape (missing export, non-function, wrong arity) produces a
    /// `RenderError::ModuleLoad` with a descriptive message.
    ///
    /// Module loading caches per-runtime: re-calling with the same path on
    /// the same `JsRuntime` instance returns the cached module (or errors if
    /// deno_core's loader doesn't deduplicate — verify behavior in tests if
    /// re-loading is exercised).
    ///
    /// # Errors
    ///
    /// Returns `RenderError::ModuleLoad("runtime constructed without FS loader...")` if
    /// this runtime was created via `JsRuntime::new()` instead of
    /// `JsRuntime::with_fs_loader()`.
    ///
    /// Returns `RenderError::ModuleLoad` for any failure during load, module
    /// evaluation, or JS exception thrown by `render()`. This includes the case
    /// where the module exists but does not export a `render` function (missing
    /// export), which is distinguished from "export exists but is not a function"
    /// by separate guard branches.
    pub async fn render_via_module(&mut self, module_path: &Path, url: &str) -> Result<String> {
        if !self.has_fs_loader {
            return Err(RenderError::ModuleLoad(
                "runtime constructed without FS loader; use JsRuntime::with_fs_loader()".into(),
            ));
        }

        // Convert filesystem path to a file:// ModuleSpecifier.
        let specifier = ModuleSpecifier::from_file_path(module_path).map_err(|_| {
            RenderError::ModuleLoad(format!(
                "cannot convert path to file URL: {}",
                module_path.display()
            ))
        })?;

        // Load the module (resolves imports recursively via FsModuleLoader).
        let module_id = self
            .inner
            .load_main_es_module(&specifier)
            .await
            .map_err(|e| RenderError::ModuleLoad(e.to_string()))?;

        // mod_evaluate schedules the top-level module body for execution.
        // We must run the event loop BEFORE awaiting the evaluate future.
        let evaluate_fut = self.inner.mod_evaluate(module_id);
        self.inner
            .run_event_loop(PollEventLoopOptions::default())
            .await
            .map_err(|e| RenderError::ModuleLoad(format!("event loop failed: {e}")))?;
        evaluate_fut
            .await
            .map_err(|e| RenderError::ModuleLoad(format!("module evaluate failed: {e}")))?;

        // Retrieve the module's namespace object (its exports).
        let namespace = self
            .inner
            .get_module_namespace(module_id)
            .map_err(|e| RenderError::ModuleLoad(format!("get_module_namespace failed: {e}")))?;

        // Open a handle scope and a TryCatch to surface JS exceptions cleanly.
        let scope = &mut self.inner.handle_scope();
        let tc = &mut v8::TryCatch::new(scope);

        // Get the `render` export from the namespace object.
        let ns_local = v8::Local::new(tc, &namespace);
        let render_key = v8::String::new(tc, "render")
            .ok_or_else(|| RenderError::ModuleLoad("v8 string alloc failed for 'render'".into()))?;
        let render_val = ns_local.get(tc, render_key.into()).ok_or_else(|| {
            let msg = tc
                .exception()
                .and_then(|e| e.to_string(tc))
                .map(|s| s.to_rust_string_lossy(tc))
                .unwrap_or_else(|| "namespace.get('render') returned None".into());
            RenderError::ModuleLoad(msg)
        })?;

        // Guard: distinguish "export missing" from "export exists but wrong type".
        // Angular bundles may ship with a different entry-point name; both cases
        // deserve a clear message rather than a confusing type-cast error.
        if render_val.is_undefined() {
            return Err(RenderError::ModuleLoad(
                "module does not export a 'render' function".into(),
            ));
        }

        // Cast to v8::Function.
        let render_fn = v8::Local::<v8::Function>::try_from(render_val)
            .map_err(|_| RenderError::ModuleLoad("'render' export is not a function".into()))?;

        // Build the `url` argument string.
        let url_arg = v8::String::new(tc, url)
            .ok_or_else(|| RenderError::ModuleLoad("v8 string alloc failed for url".into()))?;
        let this = v8::undefined(tc).into();
        let args = [v8::Local::<v8::Value>::from(url_arg)];

        // Call render(url).
        let result = render_fn.call(tc, this, &args).ok_or_else(|| {
            let msg = tc
                .exception()
                .and_then(|e| e.to_string(tc))
                .map(|s| s.to_rust_string_lossy(tc))
                .unwrap_or_else(|| "render() call returned None".into());
            RenderError::ModuleLoad(format!("render() threw: {msg}"))
        })?;

        // Convert result to Rust String.
        let result_str = result
            .to_string(tc)
            .ok_or_else(|| RenderError::Panic("render() result to_string failed".into()))?;
        Ok(result_str.to_rust_string_lossy(tc))
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new()
    }
}
