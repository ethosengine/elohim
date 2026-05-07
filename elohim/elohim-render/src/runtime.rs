//! Thin wrapper around deno_core::JsRuntime for V8 isolate lifecycle.
//!
//! Exposes a minimal synchronous eval surface used by AngularRenderer and
//! other consumers. Module loading, stdlib shims, and fetch dispatch are added
//! in subsequent tasks.

use deno_core::{v8, JsRuntime as DenoJsRuntime, RuntimeOptions};

use crate::{RenderError, Result};

/// A V8 isolate managed by deno_core.
///
/// `JsRuntime::new()` boots the isolate. `eval_string` evaluates a JS
/// expression and returns its `toString()` as a Rust `String`.
pub struct JsRuntime {
    inner: DenoJsRuntime,
}

impl JsRuntime {
    /// Boot a new V8 isolate with default runtime options.
    pub fn new() -> Self {
        let inner = DenoJsRuntime::new(RuntimeOptions {
            ..Default::default()
        });
        Self { inner }
    }

    /// Evaluate a JS expression and return its `toString()`.
    ///
    /// Requires `&mut self` because `handle_scope` requires exclusive access
    /// to the V8 isolate.
    ///
    /// Wraps compile + run in a `v8::TryCatch` scope so that JS exceptions
    /// surface their actual error message instead of being swallowed.
    ///
    /// The function is intentionally `async` to keep the contract stable for
    /// future tasks that require event-loop turning (module loading, fetch
    /// dispatch). The allow attribute silences `clippy::unused_async` without
    /// removing the forward-compatible signature.
    #[allow(clippy::unused_async)]
    pub async fn eval_string(&mut self, source: &str) -> Result<String> {
        let scope = &mut self.inner.handle_scope();
        let tc = &mut v8::TryCatch::new(scope);
        let code = v8::String::new(tc, source)
            .ok_or_else(|| RenderError::ModuleLoad("v8 string alloc failed".into()))?;
        let script = v8::Script::compile(tc, code, None).ok_or_else(|| {
            let msg = tc
                .exception()
                .and_then(|e| e.to_string(tc))
                .map(|s| s.to_rust_string_lossy(tc))
                .unwrap_or_else(|| "compile failed".into());
            RenderError::ModuleLoad(msg)
        })?;
        let result = script.run(tc).ok_or_else(|| {
            let msg = tc
                .exception()
                .and_then(|e| e.to_string(tc))
                .map(|s| s.to_rust_string_lossy(tc))
                .unwrap_or_else(|| "run returned None".into());
            RenderError::Panic(msg)
        })?;
        let s = result
            .to_string(tc)
            .ok_or_else(|| RenderError::Panic("to_string failed".into()))?;
        Ok(s.to_rust_string_lossy(tc))
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new()
    }
}
