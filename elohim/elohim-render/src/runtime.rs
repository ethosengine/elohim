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
    /// Uses a raw `HandleScope` so there is no async overhead for simple
    /// expressions. Subsequent tasks will add module-loading paths that
    /// require async.
    pub async fn eval_string(&mut self, source: &str) -> Result<String> {
        let scope = &mut self.inner.handle_scope();
        let code = v8::String::new(scope, source)
            .ok_or_else(|| RenderError::ModuleLoad("v8 string alloc failed".into()))?;
        let script = v8::Script::compile(scope, code, None)
            .ok_or_else(|| RenderError::ModuleLoad("compile failed".into()))?;
        let result = script
            .run(scope)
            .ok_or_else(|| RenderError::Panic("run returned None".into()))?;
        let s = result
            .to_string(scope)
            .ok_or_else(|| RenderError::Panic("to_string failed".into()))?;
        Ok(s.to_rust_string_lossy(scope))
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new()
    }
}
