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

use crate::shim::{console::console_ext, text::text_ext, url::url_ext};

use crate::{RenderError, Result};

/// A V8 isolate managed by deno_core.
///
/// `JsRuntime::new()` boots the isolate with no module loader — suitable for
/// `eval_string` only. `JsRuntime::with_fs_loader()` adds `FsModuleLoader`
/// so that `render_via_module` can load ESM files from the filesystem.
pub struct JsRuntime {
    inner: DenoJsRuntime,
    has_fs_loader: bool,
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
        Self {
            inner,
            has_fs_loader: false,
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
        Self {
            inner,
            has_fs_loader: true,
        }
    }

    /// Boot a new V8 isolate with console, URL, and TextEncoder/TextDecoder shims.
    ///
    /// `console.*` calls dispatch into Rust's `tracing` system at the
    /// appropriate level (info/warn/error). `URL` is a minimal WHATWG-compatible
    /// JS implementation. `TextEncoder`/`TextDecoder` are implemented on top of
    /// deno_core's `op_encode`/`op_decode` builtins. None of these are available
    /// in a bare deno_core runtime without a snapshot; the extensions inject them.
    ///
    /// Includes `FsModuleLoader` so that `render_via_module` works from the
    /// same runtime instance.
    pub fn with_shims() -> Self {
        let inner = DenoJsRuntime::new(RuntimeOptions {
            module_loader: Some(Rc::new(FsModuleLoader)),
            extensions: vec![
                console_ext::init_ops_and_esm(),
                url_ext::init_ops_and_esm(),
                text_ext::init_ops_and_esm(),
            ],
            ..Default::default()
        });
        Self {
            inner,
            has_fs_loader: true,
        }
    }

    /// Boot a new V8 isolate with all shims plus a `fetch` global backed by
    /// the provided [`DataFetcher`].
    ///
    /// The `fetch(url, init?)` global dispatches to `fetcher.fetch(...)` via
    /// an async deno_core op. The runtime drives the event loop when
    /// `eval_string` receives a Promise result, so callers can `await fetch()`
    /// inside an async IIFE evaluated through `eval_string`.
    ///
    /// Includes `FsModuleLoader` so that `render_via_module` works from the
    /// same runtime instance.
    pub fn with_full_shims(fetcher: std::sync::Arc<dyn crate::DataFetcher>) -> Self {
        use crate::shim::fetch::{fetch_ext, FetcherHandle};
        let inner = DenoJsRuntime::new(RuntimeOptions {
            module_loader: Some(Rc::new(FsModuleLoader)),
            extensions: vec![
                console_ext::init_ops_and_esm(),
                url_ext::init_ops_and_esm(),
                text_ext::init_ops_and_esm(),
                fetch_ext::init_ops_and_esm(FetcherHandle(fetcher)),
            ],
            ..Default::default()
        });
        Self {
            inner,
            has_fs_loader: true,
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
    /// Wraps execution in a try/catch so that JS exceptions surface their
    /// actual error message instead of being swallowed.
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
    /// # Errors
    ///
    /// Returns `RenderError::ModuleLoad("runtime constructed without FS loader...")` if
    /// this runtime was created via `JsRuntime::new()` instead of
    /// `JsRuntime::with_fs_loader()`.
    ///
    /// Returns `RenderError::ModuleLoad` for any failure during load, module
    /// evaluation, or JS exception thrown by `render()`.
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
