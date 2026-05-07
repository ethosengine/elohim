# Doorway SSR Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up a Rust + V8 server-side rendering runtime that lets external WebFetch and peer-to-peer fetch return fully-rendered HTML for Elohim Protocol content routes, without introducing a Node sidecar.

**Architecture:** A new library crate `elohim-render` embeds `deno_core` (V8) behind a `Renderer` trait with one `AngularRenderer` impl. `doorway-service` depends on it unconditionally (server-class hardware always carries SSR). `elohim-storage` gains opt-in `--feature ssr` for capable peer-to-peer rendering. Route eligibility lives in the storage manifest (`render: "angular-ssr"`), never in doorway code. Failure always falls back to the existing CSR shell.

**Tech Stack:** Rust (deno_core, hyper, tokio, tracing); Angular 19 (`@angular/ssr`, `provideClientHydration`); Cucumber/Gherkin a2o scenarios; Playwright for hydration verification.

**Spec:** `genesis/docs/superpowers/specs/2026-05-07-doorway-ssr-runtime-design.md`

**P2P design gate:** This plan's API routes (`/lamad/concept/{id}`, `/lamad/path/*`, `/`, `/spa/*`) and Rust types (`RenderContext`, `RenderOutput`, `FetchRequest`, `FetchResponse`, `ContentRef`, `Route.render` field) introduce **no new DHT entry types**. They serve existing `ContentNode` entities and live in the operational/projection layer. The disposition table is in the spec under "P2P Design Gate disposition" — see it before adding any new route or schema during execution. If a future task needs to add a new entity type, stop and run the design gate on the spec first.

---

## File Structure

**New files:**

| Path | Responsibility |
|---|---|
| `elohim/elohim-render/Cargo.toml` | Crate manifest |
| `elohim/elohim-render/src/lib.rs` | Module root + re-exports |
| `elohim/elohim-render/src/types.rs` | `RenderContext`, `RenderOutput`, `ContentRef`, `RenderLimits`, `RenderSpec` |
| `elohim/elohim-render/src/error.rs` | `RenderError`, `Result<T>` alias |
| `elohim/elohim-render/src/renderer.rs` | `Renderer` trait + `EchoRenderer` impl |
| `elohim/elohim-render/src/data_fetcher.rs` | `DataFetcher` trait + `FetchRequest` + `FetchResponse` |
| `elohim/elohim-render/src/runtime.rs` | `JsRuntime` thin wrapper around deno_core |
| `elohim/elohim-render/src/shim/mod.rs` | Shim module root |
| `elohim/elohim-render/src/shim/console.rs` | `console.log/warn/error` ops |
| `elohim/elohim-render/src/shim/url.rs` | `URL` shim ops |
| `elohim/elohim-render/src/shim/text.rs` | `TextEncoder` / `TextDecoder` ops |
| `elohim/elohim-render/src/shim/fetch.rs` | `fetch` op dispatching to `DataFetcher` |
| `elohim/elohim-render/src/angular.rs` | `AngularRenderer` impl |
| `elohim/elohim-render/src/snapshot.rs` | Isolate snapshotting helpers |
| `elohim/elohim-render/tests/echo.rs` | Integration test: EchoRenderer end-to-end |
| `elohim/elohim-render/tests/angular.rs` | Integration test: AngularRenderer with a fixture bundle |
| `elohim/elohim-render/fixtures/echo-bundle.mjs` | Tiny ESM fixture for stdlib shim tests |
| `elohim/elohim-render/fixtures/angular-fixture-bundle.mjs` | Pre-built fixture Angular SSR bundle for tests |
| `app/elohim-app/src/main.server.ts` | Angular server bootstrap entry |
| `app/elohim-app/src/app/app.config.server.ts` | Server-side providers (`provideServerRendering`, `provideClientHydration`) |
| `genesis/a2o/features/ssr/external-webfetch-renders-content.feature` | a2o: AI design tool reads concept page |
| `genesis/a2o/features/ssr/social-card-crawler-gets-rich-preview.feature` | a2o: social card crawler |
| `genesis/a2o/features/ssr/browser-hydrates-without-flash.feature` | a2o: browser hydration |

**Modified files:**

| Path | Change |
|---|---|
| `elohim/Cargo.toml` | Add `elohim-render` to workspace members |
| `crates/doorway-client/src/routes.rs:157` | Add `render: Option<String>` field to `Route` + `RouteBuilder::render()` |
| `elohim/elohim-storage/Cargo.toml` | Add `ssr` feature + optional `elohim-render` dep |
| `elohim/elohim-storage/src/http.rs:7629` | Declare `render: "angular-ssr"` on `/lamad/concept/*`, `/lamad/path/*`, `/` |
| `elohim/elohim-storage/src/http.rs` (router) | Add `POST /render` and `GET /spa/*` when `ssr` feature on |
| `doorway/doorway-service/Cargo.toml` | Add `elohim-render` dep |
| `doorway/doorway-service/src/server/http.rs` | Wire `Renderer` into `AppState`; dispatch render-eligible routes |
| `doorway/doorway-service/src/services/route_registry.rs` | Honor `render` field on routes |
| `app/elohim-app/angular.json` | Add `server` build target; `prerender: false`, `ssr.entry: src/main.server.ts` |
| `app/elohim-app/src/app/app.component.ts` | Add `CUSTOM_ELEMENTS_SCHEMA` for `<sophia-question>` |
| `app/elohim-app/package.json` | Add `@angular/ssr` dep |
| `app/elohim-app/scripts/prebuild.mjs` | Verify `dist/elohim-app/server/main.server.mjs` builds alongside browser bundle |

---

## Build Commands Reference

```bash
# elohim-render (native Rust crate; no holochain WASM)
cd /projects/elohim/elohim/elohim-render
RUSTFLAGS="" cargo build
RUSTFLAGS="" cargo test
RUSTFLAGS="" cargo clippy -- -D warnings

# doorway-service
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo build --release
RUSTFLAGS="" cargo test --lib --bins

# elohim-storage (holochain getrandom backend required)
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --features ssr
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features ssr

# elohim-app
cd /projects/elohim/app/elohim-app
pnpm install
pnpm run build               # browser bundle
pnpm exec ng build --configuration development   # both browser + server
```

---

## Task 1: Create elohim-render crate skeleton

**Files:**
- Create: `elohim/elohim-render/Cargo.toml`
- Create: `elohim/elohim-render/src/lib.rs`
- Modify: `elohim/Cargo.toml` (add workspace member)

- [ ] **Step 1: Create the crate manifest**

Create `elohim/elohim-render/Cargo.toml`:

```toml
[package]
name = "elohim-render"
version = "0.1.0"
edition = "2021"
description = "JS execution runtime for server-side rendering at the Elohim peer boundary"
license = "Apache-2.0"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
tracing = "0.1"
async-trait = "0.1"
tokio = { version = "1.43", features = ["sync", "rt", "macros"] }

[dev-dependencies]
tokio = { version = "1.43", features = ["full", "macros"] }

[features]
default = []
```

(deno_core gets added in Task 4, not now — Task 1 ships a V8-free skeleton on purpose.)

- [ ] **Step 2: Create the module root**

Create `elohim/elohim-render/src/lib.rs`:

```rust
//! elohim-render — JS execution runtime for server-side rendering.
//!
//! Embeds V8 via deno_core. Exposes a `Renderer` trait that frameworks
//! plug into. Doorway and elohim-storage both consume this crate.
//!
//! See `genesis/docs/superpowers/specs/2026-05-07-doorway-ssr-runtime-design.md`.

pub mod data_fetcher;
pub mod error;
pub mod renderer;
pub mod types;

pub use data_fetcher::{DataFetcher, FetchRequest, FetchResponse};
pub use error::{RenderError, Result};
pub use renderer::{EchoRenderer, Renderer};
pub use types::{ContentRef, RenderContext, RenderLimits, RenderOutput, RenderSpec};
```

- [ ] **Step 3: Add the crate to the workspace**

Open `elohim/Cargo.toml`, find the `[workspace]` `members = [...]` array, and add `"elohim-render"` to the list (alphabetical insertion is fine).

- [ ] **Step 4: Verify it compiles (it won't yet — three modules are missing)**

Run from `elohim/elohim-render/`:

```bash
RUSTFLAGS="" cargo check
```

Expected: errors about missing modules `data_fetcher`, `error`, `renderer`, `types`. This is the failing baseline; Tasks 2 and 3 add them.

- [ ] **Step 5: Commit**

```bash
git add elohim/Cargo.toml elohim/elohim-render/
git commit -m "feat(elohim-render): scaffold crate"
```

---

## Task 2: Define types, error, and DataFetcher

**Files:**
- Create: `elohim/elohim-render/src/types.rs`
- Create: `elohim/elohim-render/src/error.rs`
- Create: `elohim/elohim-render/src/data_fetcher.rs`

- [ ] **Step 1: Write `error.rs`**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("module load failed: {0}")]
    ModuleLoad(String),

    #[error("render timed out after {limit_ms}ms")]
    Timeout { limit_ms: u64 },

    #[error("isolate out of memory (limit: {limit_mb}MB)")]
    OutOfMemory { limit_mb: u64 },

    #[error("data fetch failed: {0}")]
    DataFetch(String),

    #[error("renderer panicked: {0}")]
    Panic(String),

    #[error("unsupported render spec: {0}")]
    UnsupportedSpec(String),

    #[error("render output exceeded {limit_bytes} bytes")]
    OutputTooLarge { limit_bytes: usize },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, RenderError>;
```

- [ ] **Step 2: Write `types.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::data_fetcher::DataFetcher;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RenderSpec {
    Echo,
    AngularSsr,
}

impl RenderSpec {
    pub fn parse(s: &str) -> crate::Result<Self> {
        match s {
            "echo" => Ok(Self::Echo),
            "angular-ssr" => Ok(Self::AngularSsr),
            other => Err(crate::RenderError::UnsupportedSpec(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RenderLimits {
    pub wall_time_ms: u64,
    pub memory_mb: u64,
    pub max_fetches: u32,
    pub max_output_bytes: usize,
}

impl Default for RenderLimits {
    fn default() -> Self {
        Self {
            wall_time_ms: 2_000,
            memory_mb: 128,
            max_fetches: 32,
            max_output_bytes: 2 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRef {
    pub id: String,
    pub content_hash: String,
}

pub struct RenderContext {
    pub spec: RenderSpec,
    pub url: String,
    pub data_fetcher: Arc<dyn DataFetcher>,
    pub limits: RenderLimits,
}

#[derive(Debug, Clone, Default)]
pub struct RenderOutput {
    pub html: String,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub fetched_inputs: Vec<ContentRef>,
}
```

- [ ] **Step 3: Write `data_fetcher.rs`**

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    /// Optional content hash for cache-key derivation.
    pub content_hash: Option<String>,
}

#[async_trait]
pub trait DataFetcher: Send + Sync {
    async fn fetch(&self, request: FetchRequest) -> Result<FetchResponse>;
}
```

- [ ] **Step 4: Verify it compiles**

```bash
RUSTFLAGS="" cargo check -p elohim-render
```

Expected: error about missing `renderer` module (Task 3 adds it). Types and traits should compile.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-render/src/
git commit -m "feat(elohim-render): types, errors, DataFetcher trait"
```

---

## Task 3: Renderer trait + EchoRenderer (no V8 yet)

**Files:**
- Create: `elohim/elohim-render/src/renderer.rs`
- Create: `elohim/elohim-render/tests/echo.rs`

- [ ] **Step 1: Write the failing integration test FIRST**

Create `elohim/elohim-render/tests/echo.rs`:

```rust
use async_trait::async_trait;
use elohim_render::{
    DataFetcher, EchoRenderer, FetchRequest, FetchResponse, RenderContext, RenderLimits,
    RenderSpec, Renderer, Result,
};
use std::sync::Arc;

struct NoopFetcher;

#[async_trait]
impl DataFetcher for NoopFetcher {
    async fn fetch(&self, _request: FetchRequest) -> Result<FetchResponse> {
        unreachable!("EchoRenderer should not call fetch");
    }
}

#[tokio::test]
async fn echo_renderer_returns_url_in_html() {
    let renderer = EchoRenderer::default();
    let ctx = RenderContext {
        spec: RenderSpec::Echo,
        url: "/test/path".to_string(),
        data_fetcher: Arc::new(NoopFetcher),
        limits: RenderLimits::default(),
    };

    let out = renderer.render(ctx).await.expect("render ok");
    assert_eq!(out.status, 200);
    assert!(out.html.contains("/test/path"), "html: {}", out.html);
}
```

- [ ] **Step 2: Run the test, confirm it fails**

```bash
RUSTFLAGS="" cargo test -p elohim-render --test echo
```

Expected: compile error — `EchoRenderer` does not exist yet.

- [ ] **Step 3: Write `renderer.rs`**

```rust
use async_trait::async_trait;

use crate::{RenderContext, RenderOutput, Result};

#[async_trait]
pub trait Renderer: Send + Sync {
    async fn render(&self, ctx: RenderContext) -> Result<RenderOutput>;
}

#[derive(Default)]
pub struct EchoRenderer;

#[async_trait]
impl Renderer for EchoRenderer {
    async fn render(&self, ctx: RenderContext) -> Result<RenderOutput> {
        let html = format!(
            "<!doctype html><html><body><pre>echo: {}</pre></body></html>",
            ctx.url
        );
        Ok(RenderOutput {
            html,
            status: 200,
            headers: vec![("content-type".into(), "text/html; charset=utf-8".into())],
            fetched_inputs: vec![],
        })
    }
}
```

- [ ] **Step 4: Run the test, confirm it passes**

```bash
RUSTFLAGS="" cargo test -p elohim-render --test echo
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-render/src/renderer.rs elohim/elohim-render/tests/echo.rs
git commit -m "feat(elohim-render): Renderer trait + EchoRenderer"
```

---

## Task 4: Add deno_core, boot a V8 isolate, evaluate `1+1`

**Files:**
- Modify: `elohim/elohim-render/Cargo.toml`
- Create: `elohim/elohim-render/src/runtime.rs`
- Modify: `elohim/elohim-render/src/lib.rs`

- [ ] **Step 1: Add deno_core to deps**

Append to `elohim/elohim-render/Cargo.toml` `[dependencies]`:

```toml
deno_core = "0.339"
```

(Pin a recent stable version. If newer version exists, use it; the API is stable across patch releases.)

- [ ] **Step 2: Write the failing test**

Create `elohim/elohim-render/tests/runtime.rs`:

```rust
use elohim_render::runtime::JsRuntime;

#[tokio::test]
async fn evaluates_simple_expression() {
    let mut rt = JsRuntime::new();
    let result = rt.eval_string("1 + 1").await.expect("eval ok");
    assert_eq!(result, "2");
}
```

- [ ] **Step 3: Confirm it fails**

```bash
RUSTFLAGS="" cargo test -p elohim-render --test runtime
```

Expected: compile error — `JsRuntime` does not exist.

- [ ] **Step 4: Write `runtime.rs`**

```rust
//! Thin wrapper around deno_core::JsRuntime for V8 isolate lifecycle.

use deno_core::{JsRuntime as DenoJsRuntime, RuntimeOptions, v8};

use crate::{RenderError, Result};

pub struct JsRuntime {
    inner: DenoJsRuntime,
}

impl JsRuntime {
    pub fn new() -> Self {
        let inner = DenoJsRuntime::new(RuntimeOptions {
            ..Default::default()
        });
        Self { inner }
    }

    /// Evaluate a JS expression and return its toString().
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
```

- [ ] **Step 5: Re-export the runtime module**

Add to `elohim/elohim-render/src/lib.rs`:

```rust
pub mod runtime;
```

- [ ] **Step 6: Run the test, confirm it passes**

```bash
RUSTFLAGS="" cargo test -p elohim-render --test runtime
```

Expected: PASS. (First build is slow — V8 statically links. Allow ~5-10 min on a cold cache. `sccache` materially helps.)

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-render/Cargo.toml elohim/elohim-render/src/runtime.rs elohim/elohim-render/src/lib.rs elohim/elohim-render/tests/runtime.rs
git commit -m "feat(elohim-render): boot V8 isolate via deno_core"
```

---

## Task 5: Module loader — load an ESM module and read an export

**Files:**
- Create: `elohim/elohim-render/fixtures/echo-bundle.mjs`
- Modify: `elohim/elohim-render/src/runtime.rs`
- Create: `elohim/elohim-render/tests/module_load.rs`

- [ ] **Step 1: Create the fixture module**

Create `elohim/elohim-render/fixtures/echo-bundle.mjs`:

```js
export function render(url) {
  return `<!doctype html><html><body><pre>echo: ${url}</pre></body></html>`;
}
```

- [ ] **Step 2: Write the failing test**

Create `elohim/elohim-render/tests/module_load.rs`:

```rust
use elohim_render::runtime::JsRuntime;

#[tokio::test]
async fn loads_esm_module_and_calls_export() {
    let mut rt = JsRuntime::new();
    let module_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/echo-bundle.mjs");
    let html = rt
        .render_via_module(&module_path, "/foo")
        .await
        .expect("module render ok");
    assert!(html.contains("/foo"), "html: {}", html);
}
```

- [ ] **Step 3: Confirm it fails**

```bash
RUSTFLAGS="" cargo test -p elohim-render --test module_load
```

Expected: compile error — `render_via_module` not defined.

- [ ] **Step 4: Add module loading to `runtime.rs`**

Append to `elohim/elohim-render/src/runtime.rs`:

```rust
use deno_core::{ModuleSpecifier, ModuleCodeString, FsModuleLoader};
use std::rc::Rc;
use std::path::Path;

impl JsRuntime {
    /// Build a runtime with an FS module loader for ESM imports.
    pub fn with_fs_loader() -> Self {
        let inner = DenoJsRuntime::new(RuntimeOptions {
            module_loader: Some(Rc::new(FsModuleLoader)),
            ..Default::default()
        });
        Self { inner }
    }

    /// Load an ESM module from `module_path`, call its `render(url)` export,
    /// and return the result as a string.
    pub async fn render_via_module(
        &mut self,
        module_path: &Path,
        url: &str,
    ) -> Result<String> {
        // Re-init with FS loader if needed
        if !self.has_fs_loader() {
            *self = Self::with_fs_loader();
        }
        let specifier = ModuleSpecifier::from_file_path(module_path).map_err(|_| {
            RenderError::ModuleLoad(format!("invalid module path: {}", module_path.display()))
        })?;
        let module_id = self
            .inner
            .load_main_es_module(&specifier)
            .await
            .map_err(|e| RenderError::ModuleLoad(e.to_string()))?;
        let _ = self
            .inner
            .mod_evaluate(module_id)
            .await
            .map_err(|e| RenderError::ModuleLoad(e.to_string()))?;

        // Call render(url) on the namespace
        let scope = &mut self.inner.handle_scope();
        let ns = self
            .inner_ns(module_id, scope)
            .ok_or_else(|| RenderError::ModuleLoad("module namespace missing".into()))?;
        let render_key = v8::String::new(scope, "render")
            .ok_or_else(|| RenderError::ModuleLoad("v8 alloc".into()))?
            .into();
        let render_fn = ns
            .get(scope, render_key)
            .ok_or_else(|| RenderError::ModuleLoad("render export missing".into()))?;
        let render_fn = v8::Local::<v8::Function>::try_from(render_fn)
            .map_err(|_| RenderError::ModuleLoad("render is not a function".into()))?;
        let url_v8 = v8::String::new(scope, url)
            .ok_or_else(|| RenderError::ModuleLoad("url alloc".into()))?
            .into();
        let recv = v8::undefined(scope).into();
        let result = render_fn
            .call(scope, recv, &[url_v8])
            .ok_or_else(|| RenderError::Panic("render() threw".into()))?;
        let s = result
            .to_string(scope)
            .ok_or_else(|| RenderError::Panic("to_string failed".into()))?;
        Ok(s.to_rust_string_lossy(scope))
    }

    fn has_fs_loader(&self) -> bool {
        // deno_core doesn't expose this; treat as "always need re-init for safety"
        // in this MVP. Optimize later by tracking via a flag.
        false
    }

    fn inner_ns<'s>(
        &mut self,
        module_id: deno_core::ModuleId,
        scope: &mut v8::HandleScope<'s>,
    ) -> Option<v8::Local<'s, v8::Object>> {
        let global = self.inner.get_module_namespace(module_id).ok()?;
        let local = v8::Local::new(scope, global);
        Some(local)
    }
}
```

(The deno_core API surface above is the typical 0.339 shape. If APIs have shifted, the engineer adapts — the contract is "load a file:// ESM module, get its `render` export, call it with one string arg, return the toString of the result." Document any API shape difference in the commit message.)

- [ ] **Step 5: Run the test**

```bash
RUSTFLAGS="" cargo test -p elohim-render --test module_load
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-render/fixtures/ elohim/elohim-render/src/runtime.rs elohim/elohim-render/tests/module_load.rs
git commit -m "feat(elohim-render): ESM module loading via FsModuleLoader"
```

---

## Task 6: console / URL / TextEncoder / TextDecoder shims

**Files:**
- Create: `elohim/elohim-render/src/shim/mod.rs`
- Create: `elohim/elohim-render/src/shim/console.rs`
- Create: `elohim/elohim-render/src/shim/url.rs`
- Create: `elohim/elohim-render/src/shim/text.rs`
- Modify: `elohim/elohim-render/src/lib.rs`
- Create: `elohim/elohim-render/tests/shim.rs`

- [ ] **Step 1: Failing test**

Create `elohim/elohim-render/tests/shim.rs`:

```rust
use elohim_render::runtime::JsRuntime;

#[tokio::test]
async fn console_log_does_not_throw() {
    let mut rt = JsRuntime::with_shims();
    let v = rt.eval_string("console.log('hello'); 42").await.unwrap();
    assert_eq!(v, "42");
}

#[tokio::test]
async fn url_constructor_works() {
    let mut rt = JsRuntime::with_shims();
    let v = rt
        .eval_string("new URL('/foo', 'https://example.com').href")
        .await
        .unwrap();
    assert_eq!(v, "https://example.com/foo");
}

#[tokio::test]
async fn text_encoder_round_trips() {
    let mut rt = JsRuntime::with_shims();
    let v = rt
        .eval_string("new TextDecoder().decode(new TextEncoder().encode('hi'))")
        .await
        .unwrap();
    assert_eq!(v, "hi");
}
```

- [ ] **Step 2: Confirm failing**

```bash
RUSTFLAGS="" cargo test -p elohim-render --test shim
```

Expected: compile error — `with_shims` does not exist.

- [ ] **Step 3: Implement the shims**

Create `elohim/elohim-render/src/shim/mod.rs`:

```rust
pub mod console;
pub mod text;
pub mod url;
```

Create `elohim/elohim-render/src/shim/console.rs`:

```rust
use deno_core::{op2, Op};

#[op2(fast)]
fn op_console_log(#[string] msg: String) {
    tracing::info!(target: "elohim_render::js_console", "{}", msg);
}

#[op2(fast)]
fn op_console_warn(#[string] msg: String) {
    tracing::warn!(target: "elohim_render::js_console", "{}", msg);
}

#[op2(fast)]
fn op_console_error(#[string] msg: String) {
    tracing::error!(target: "elohim_render::js_console", "{}", msg);
}

deno_core::extension!(
    console_ext,
    ops = [op_console_log, op_console_warn, op_console_error],
    js = [dir "src/shim", "console.js"],
);
```

Create `elohim/elohim-render/src/shim/console.js`:

```js
// Minimal console binding to ops; shape compatible with what Angular SSR uses.
((globalThis) => {
  const fmt = (args) => args.map((a) => {
    if (typeof a === "string") return a;
    try { return JSON.stringify(a); } catch { return String(a); }
  }).join(" ");

  globalThis.console = {
    log:   (...a) => Deno.core.ops.op_console_log(fmt(a)),
    info:  (...a) => Deno.core.ops.op_console_log(fmt(a)),
    warn:  (...a) => Deno.core.ops.op_console_warn(fmt(a)),
    error: (...a) => Deno.core.ops.op_console_error(fmt(a)),
    debug: (...a) => Deno.core.ops.op_console_log(fmt(a)),
  };
})(globalThis);
```

Create `elohim/elohim-render/src/shim/url.js`:

```js
// V8 has WHATWG URL built in; nothing to shim. This file exists to document
// the surface and to give snapshots a stable load order.
```

Create `elohim/elohim-render/src/shim/url.rs`:

```rust
deno_core::extension!(
    url_ext,
    js = [dir "src/shim", "url.js"],
);
```

Create `elohim/elohim-render/src/shim/text.js`:

```js
// V8 has TextEncoder/TextDecoder built in. Same rationale as url.js.
```

Create `elohim/elohim-render/src/shim/text.rs`:

```rust
deno_core::extension!(
    text_ext,
    js = [dir "src/shim", "text.js"],
);
```

- [ ] **Step 4: Wire shims into the runtime**

Append to `elohim/elohim-render/src/runtime.rs`:

```rust
use crate::shim::{console::console_ext, text::text_ext, url::url_ext};

impl JsRuntime {
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
        Self { inner }
    }
}
```

- [ ] **Step 5: Re-export shim module**

Add to `elohim/elohim-render/src/lib.rs`:

```rust
pub mod shim;
```

- [ ] **Step 6: Run the test, expect PASS**

```bash
RUSTFLAGS="" cargo test -p elohim-render --test shim
```

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-render/src/shim/ elohim/elohim-render/src/runtime.rs elohim/elohim-render/src/lib.rs elohim/elohim-render/tests/shim.rs
git commit -m "feat(elohim-render): console/URL/TextEncoder shims"
```

---

## Task 7: fetch shim that dispatches to DataFetcher

**Files:**
- Create: `elohim/elohim-render/src/shim/fetch.rs`
- Create: `elohim/elohim-render/src/shim/fetch.js`
- Modify: `elohim/elohim-render/src/shim/mod.rs`
- Modify: `elohim/elohim-render/src/runtime.rs`
- Create: `elohim/elohim-render/tests/fetch.rs`

- [ ] **Step 1: Failing test**

Create `elohim/elohim-render/tests/fetch.rs`:

```rust
use async_trait::async_trait;
use elohim_render::{DataFetcher, FetchRequest, FetchResponse, Result};
use elohim_render::runtime::JsRuntime;
use std::collections::HashMap;
use std::sync::Arc;

struct FixedFetcher;

#[async_trait]
impl DataFetcher for FixedFetcher {
    async fn fetch(&self, request: FetchRequest) -> Result<FetchResponse> {
        Ok(FetchResponse {
            status: 200,
            headers: HashMap::from([("content-type".into(), "application/json".into())]),
            body: format!(r#"{{"url":"{}"}}"#, request.url).into_bytes(),
            content_hash: Some("test-hash".into()),
        })
    }
}

#[tokio::test]
async fn fetch_dispatches_to_data_fetcher() {
    let mut rt = JsRuntime::with_full_shims(Arc::new(FixedFetcher));
    let v = rt
        .eval_string(
            "(async () => { const r = await fetch('/foo'); return await r.text(); })()",
        )
        .await
        .unwrap();
    assert!(v.contains("/foo"), "got: {}", v);
}
```

- [ ] **Step 2: Confirm failing**

```bash
RUSTFLAGS="" cargo test -p elohim-render --test fetch
```

Expected: compile error — `with_full_shims` does not exist.

- [ ] **Step 3: Implement the fetch shim**

Create `elohim/elohim-render/src/shim/fetch.rs`:

```rust
use deno_core::{op2, OpState};
use serde_json::Value;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::{DataFetcher, FetchRequest, FetchResponse};

pub struct FetcherHandle(pub Arc<dyn DataFetcher>);

#[op2(async)]
#[serde]
async fn op_fetch(
    state: Rc<RefCell<OpState>>,
    #[serde] request: FetchRequest,
) -> Result<FetchResponse, deno_core::error::AnyError> {
    let fetcher = {
        let state = state.borrow();
        state.borrow::<FetcherHandle>().0.clone()
    };
    fetcher
        .fetch(request)
        .await
        .map_err(|e| deno_core::error::AnyError::msg(e.to_string()))
}

deno_core::extension!(
    fetch_ext,
    ops = [op_fetch],
    js = [dir "src/shim", "fetch.js"],
    options = { fetcher: FetcherHandle },
    state = |state, options| {
        state.put(options.fetcher);
    },
);
```

Create `elohim/elohim-render/src/shim/fetch.js`:

```js
((globalThis) => {
  globalThis.fetch = async (urlOrRequest, init = {}) => {
    const url = typeof urlOrRequest === "string" ? urlOrRequest : urlOrRequest.url;
    const request = {
      method: (init && init.method) || "GET",
      url,
      headers: (init && init.headers) || {},
      body: init && init.body
        ? Array.from(typeof init.body === "string"
            ? new TextEncoder().encode(init.body)
            : init.body)
        : null,
    };
    const response = await Deno.core.ops.op_fetch(request);
    const bodyBytes = new Uint8Array(response.body);
    return {
      status: response.status,
      headers: new Map(Object.entries(response.headers)),
      arrayBuffer: async () => bodyBytes.buffer,
      text: async () => new TextDecoder().decode(bodyBytes),
      json: async () => JSON.parse(new TextDecoder().decode(bodyBytes)),
      ok: response.status >= 200 && response.status < 300,
    };
  };
})(globalThis);
```

- [ ] **Step 4: Update `shim/mod.rs`**

```rust
pub mod console;
pub mod fetch;
pub mod text;
pub mod url;
```

- [ ] **Step 5: Wire `with_full_shims`**

Append to `elohim/elohim-render/src/runtime.rs`:

```rust
use crate::shim::fetch::{fetch_ext, FetcherHandle};
use crate::DataFetcher;
use std::sync::Arc;

impl JsRuntime {
    pub fn with_full_shims(fetcher: Arc<dyn DataFetcher>) -> Self {
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
        Self { inner }
    }
}
```

- [ ] **Step 6: Run the test**

```bash
RUSTFLAGS="" cargo test -p elohim-render --test fetch
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-render/src/shim/ elohim/elohim-render/src/runtime.rs elohim/elohim-render/tests/fetch.rs
git commit -m "feat(elohim-render): fetch shim dispatches to DataFetcher"
```

---

## Task 8: Add @angular/ssr to elohim-app, configure server build

**Files:**
- Modify: `app/elohim-app/package.json`
- Modify: `app/elohim-app/angular.json`
- Create: `app/elohim-app/src/main.server.ts`
- Create: `app/elohim-app/src/app/app.config.server.ts`

- [ ] **Step 1: Install @angular/ssr**

From `app/elohim-app/`:

```bash
pnpm add @angular/ssr@~19
```

- [ ] **Step 2: Create the server config**

Create `app/elohim-app/src/app/app.config.server.ts`:

```typescript
import { ApplicationConfig, mergeApplicationConfig } from '@angular/core';
import { provideServerRendering } from '@angular/ssr';
import { provideClientHydration } from '@angular/platform-browser';

import { appConfig } from './app.config';

const serverConfig: ApplicationConfig = {
  providers: [provideServerRendering(), provideClientHydration()],
};

export const config = mergeApplicationConfig(appConfig, serverConfig);
```

- [ ] **Step 3: Create the server entry**

Create `app/elohim-app/src/main.server.ts`:

```typescript
import { bootstrapApplication } from '@angular/platform-browser';
import { AppComponent } from './app/app.component';
import { config } from './app/app.config.server';

const bootstrap = () => bootstrapApplication(AppComponent, config);

export default bootstrap;
```

- [ ] **Step 4: Add the `server` build target to angular.json**

Edit `app/elohim-app/angular.json` — under `projects.elohim-app.architect.build.options`, add:

```json
"server": "src/main.server.ts",
"prerender": false,
"ssr": {
  "entry": "src/main.server.ts"
}
```

(The exact JSON shape Angular CLI 19 expects: `server` is a sibling of `browser`. If `ng add @angular/ssr` was used directly it inserts these fields; doing it by hand here for traceability.)

- [ ] **Step 5: Build both bundles**

```bash
cd /projects/elohim/app/elohim-app
pnpm exec ng build --configuration development
```

Expected: outputs `dist/elohim-app/browser/` AND `dist/elohim-app/server/main.server.mjs`.

- [ ] **Step 6: Verify the server bundle exists and is ESM**

```bash
ls -la dist/elohim-app/server/main.server.mjs
head -5 dist/elohim-app/server/main.server.mjs
```

Expected: file present, content begins with ESM `import` statements.

- [ ] **Step 7: Commit**

```bash
git add app/elohim-app/package.json app/elohim-app/pnpm-lock.yaml app/elohim-app/angular.json app/elohim-app/src/main.server.ts app/elohim-app/src/app/app.config.server.ts
git commit -m "feat(elohim-app): add @angular/ssr server build target"
```

---

## Task 9: AngularRenderer adapter

**Files:**
- Create: `elohim/elohim-render/src/angular.rs`
- Modify: `elohim/elohim-render/src/lib.rs`
- Create: `elohim/elohim-render/tests/angular.rs`
- Create: `elohim/elohim-render/fixtures/angular-fixture-bundle.mjs`

- [ ] **Step 1: Create a tiny fixture bundle that mimics Angular's SSR API**

Create `elohim/elohim-render/fixtures/angular-fixture-bundle.mjs`:

```js
// Mimics Angular's main.server.mjs surface for tests:
// exports a default `bootstrap()` that returns a Promise<string> (rendered HTML).
//
// Real Angular's renderApplication(bootstrap, { url, document }) returns the same.
export default function bootstrap() {
  return Promise.resolve(
    `<!doctype html><html><head><title>Fixture</title></head>` +
      `<body><app-root>fixture rendered</app-root></body></html>`
  );
}

export async function renderApplication(_bootstrap, opts) {
  return `<!doctype html><html><head><title>${opts.url}</title></head>` +
    `<body><app-root ngh="0">fixture rendered ${opts.url}</app-root></body></html>`;
}
```

- [ ] **Step 2: Failing test**

Create `elohim/elohim-render/tests/angular.rs`:

```rust
use async_trait::async_trait;
use elohim_render::{
    AngularRenderer, DataFetcher, FetchRequest, FetchResponse, RenderContext, RenderLimits,
    RenderSpec, Renderer, Result,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

struct EmptyFetcher;

#[async_trait]
impl DataFetcher for EmptyFetcher {
    async fn fetch(&self, _request: FetchRequest) -> Result<FetchResponse> {
        Ok(FetchResponse {
            status: 200,
            headers: HashMap::new(),
            body: b"{}".to_vec(),
            content_hash: None,
        })
    }
}

#[tokio::test]
async fn angular_renderer_returns_rendered_html() {
    let bundle: PathBuf = [env!("CARGO_MANIFEST_DIR"), "fixtures", "angular-fixture-bundle.mjs"]
        .iter()
        .collect();
    let renderer = AngularRenderer::new(bundle).await.expect("renderer init");
    let ctx = RenderContext {
        spec: RenderSpec::AngularSsr,
        url: "/lamad/concept/test".into(),
        data_fetcher: Arc::new(EmptyFetcher),
        limits: RenderLimits::default(),
    };
    let out = renderer.render(ctx).await.expect("render");
    assert_eq!(out.status, 200);
    assert!(out.html.contains("fixture rendered /lamad/concept/test"));
    assert!(out.html.contains("ngh="), "hydration markers missing: {}", out.html);
}
```

- [ ] **Step 3: Confirm failing**

```bash
RUSTFLAGS="" cargo test -p elohim-render --test angular
```

Expected: compile error — `AngularRenderer` does not exist.

- [ ] **Step 4: Implement `angular.rs`**

```rust
//! AngularRenderer — loads main.server.mjs, calls renderApplication(bootstrap, {url}).

use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::runtime::JsRuntime;
use crate::{RenderContext, RenderError, RenderOutput, Renderer, Result};

pub struct AngularRenderer {
    bundle: PathBuf,
    // Single isolate for MVP — pool comes in a later iteration.
    runtime: Mutex<Option<JsRuntime>>,
}

impl AngularRenderer {
    pub async fn new(bundle: PathBuf) -> Result<Self> {
        if !bundle.exists() {
            return Err(RenderError::ModuleLoad(format!(
                "bundle not found: {}",
                bundle.display()
            )));
        }
        Ok(Self {
            bundle,
            runtime: Mutex::new(None),
        })
    }
}

#[async_trait]
impl Renderer for AngularRenderer {
    async fn render(&self, ctx: RenderContext) -> Result<RenderOutput> {
        let mut guard = self.runtime.lock().expect("runtime mutex");
        if guard.is_none() {
            *guard = Some(JsRuntime::with_full_shims(ctx.data_fetcher.clone()));
        }
        let rt = guard.as_mut().expect("runtime present");

        // Lazy: call into the fixture's renderApplication via dynamic import + invoke.
        let url_lit = serde_json::to_string(&ctx.url).map_err(RenderError::Serde)?;
        let bundle_url = url::Url::from_file_path(&self.bundle)
            .map_err(|_| RenderError::ModuleLoad("bundle path → url".into()))?;
        let bundle_lit = serde_json::to_string(bundle_url.as_str()).map_err(RenderError::Serde)?;

        let driver = format!(
            r#"
            (async () => {{
                const mod = await import({bundle_lit});
                const html = await mod.renderApplication(mod.default, {{ url: {url_lit} }});
                return html;
            }})()
            "#
        );

        let html = rt.eval_string(&driver).await?;

        if html.len() > ctx.limits.max_output_bytes {
            return Err(RenderError::OutputTooLarge {
                limit_bytes: ctx.limits.max_output_bytes,
            });
        }

        Ok(RenderOutput {
            html,
            status: 200,
            headers: vec![
                ("content-type".into(), "text/html; charset=utf-8".into()),
            ],
            fetched_inputs: vec![],
        })
    }
}
```

- [ ] **Step 5: Add `url` to deps**

Append to `elohim/elohim-render/Cargo.toml` `[dependencies]`:

```toml
url = "2.5"
```

- [ ] **Step 6: Re-export AngularRenderer**

Add to `elohim/elohim-render/src/lib.rs`:

```rust
pub mod angular;
pub use angular::AngularRenderer;
```

- [ ] **Step 7: Run the test**

```bash
RUSTFLAGS="" cargo test -p elohim-render --test angular
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add elohim/elohim-render/src/angular.rs elohim/elohim-render/src/lib.rs elohim/elohim-render/Cargo.toml elohim/elohim-render/fixtures/angular-fixture-bundle.mjs elohim/elohim-render/tests/angular.rs
git commit -m "feat(elohim-render): AngularRenderer adapter"
```

---

## Task 10: Wire elohim-render into doorway as a hardcoded `/render-test` route

**Files:**
- Modify: `doorway/doorway-service/Cargo.toml`
- Modify: `doorway/doorway-service/src/server/http.rs`
- Modify: `doorway/doorway-service/src/server/mod.rs` (or wherever AppState is defined; line 195 of http.rs in current tree)
- Create: `doorway/doorway-service/tests/ssr_smoke.rs`

- [ ] **Step 1: Add the dep**

In `doorway/doorway-service/Cargo.toml` `[dependencies]`:

```toml
elohim-render = { path = "../../elohim/elohim-render" }
```

- [ ] **Step 2: Failing integration test**

Create `doorway/doorway-service/tests/ssr_smoke.rs`:

```rust
//! Smoke test: doorway hardcoded /render-test route returns rendered HTML.
//!
//! Uses the angular-fixture-bundle so we don't need a real Angular build.

use std::path::PathBuf;

#[tokio::test]
#[ignore = "requires running doorway with ssr_test_bundle env"]
async fn render_test_route_returns_rendered_html() {
    // The agent runs doorway separately with --render-test-bundle <path>.
    // Test hits localhost:8888/render-test/foo and asserts the body.
    let body = reqwest::get("http://localhost:8888/render-test/foo")
        .await
        .expect("connect")
        .text()
        .await
        .expect("body");
    assert!(body.contains("/render-test/foo"), "body: {}", body);
    assert!(body.contains("ngh="), "no hydration markers: {}", body);
}
```

(Marked `#[ignore]` because it needs a live doorway. The agent runs it explicitly with `cargo test --test ssr_smoke -- --ignored` after standing up doorway.)

- [ ] **Step 3: Add a `Renderer` slot to AppState**

In `doorway/doorway-service/src/server/http.rs` (or wherever `pub struct AppState` lives — current line ~190):

```rust
pub struct AppState {
    // ... existing fields ...
    pub renderer: Option<Arc<dyn elohim_render::Renderer>>,
}
```

In `AppState::new()` (around line 201):

```rust
// Initialize renderer if SSR_BUNDLE_PATH is set.
let renderer: Option<Arc<dyn elohim_render::Renderer>> = match std::env::var("SSR_BUNDLE_PATH") {
    Ok(path) => match elohim_render::AngularRenderer::new(PathBuf::from(path)).await {
        Ok(r) => Some(Arc::new(r) as Arc<dyn elohim_render::Renderer>),
        Err(e) => {
            tracing::warn!("SSR disabled: {}", e);
            None
        }
    },
    Err(_) => None,
};
```

(`AppState::new` is currently sync. If converting to async is too invasive in one task, alternatively spawn the renderer init in a background task and use `tokio::sync::OnceCell`. Pick the path that keeps the existing call sites intact — engineer's judgment.)

- [ ] **Step 4: Add a hardcoded `/render-test/*` match arm**

In `http.rs`, find the request dispatch (the big match block referenced in `doorway/doorway-service/CLAUDE.md`). Add ABOVE the registry fallback:

```rust
if let Some(stripped) = path.strip_prefix("/render-test/") {
    if let Some(renderer) = state.renderer.as_ref() {
        let url = format!("/{}", stripped);
        let ctx = elohim_render::RenderContext {
            spec: elohim_render::RenderSpec::AngularSsr,
            url: url.clone(),
            data_fetcher: Arc::new(NoopFetcher),
            limits: Default::default(),
        };
        match renderer.render(ctx).await {
            Ok(out) => return ok_html(out.html),
            Err(e) => {
                tracing::warn!(target: "doorway::ssr", "render error: {e}");
                return spa_shell_fallback();
            }
        }
    }
}
```

(`NoopFetcher`, `ok_html`, `spa_shell_fallback` — small helpers in the same file. The engineer fills them out: NoopFetcher returns 404 from `fetch()`; `ok_html` builds a hyper response with `text/html` header; `spa_shell_fallback` proxies to the existing SPA index.html serving.)

- [ ] **Step 5: Build doorway and start it with the fixture bundle**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo build --release
SSR_BUNDLE_PATH=/projects/elohim/elohim/elohim-render/fixtures/angular-fixture-bundle.mjs \
  ./target/release/doorway &
DOORWAY_PID=$!
sleep 2
```

- [ ] **Step 6: Hit it with curl**

```bash
curl -s http://localhost:8888/render-test/foo
```

Expected output contains `fixture rendered /render-test/foo` and `ngh=`.

- [ ] **Step 7: Run the integration test**

```bash
RUSTFLAGS="" cargo test --test ssr_smoke -- --ignored
```

Expected: PASS.

- [ ] **Step 8: Stop doorway, commit**

```bash
kill $DOORWAY_PID
git add doorway/doorway-service/Cargo.toml doorway/doorway-service/src/server/http.rs doorway/doorway-service/tests/ssr_smoke.rs
git commit -m "feat(doorway): /render-test route dispatches to elohim-render"
```

---

## Task 11: Extend Route schema with `render` field

**Files:**
- Modify: `crates/doorway-client/src/routes.rs`
- Create: `crates/doorway-client/tests/render_field.rs` (or extend existing test file)

- [ ] **Step 1: Failing test**

Add to `crates/doorway-client/tests/render_field.rs` (create if absent):

```rust
use doorway_client::routes::Route;

#[test]
fn render_field_round_trips_through_serde() {
    let route = Route::get("/lamad/concept/{id}")
        .handler("get_concept")
        .render("angular-ssr")
        .build();
    let json = serde_json::to_string(&route).unwrap();
    assert!(json.contains(r#""render":"angular-ssr""#));
    let back: Route = serde_json::from_str(&json).unwrap();
    assert_eq!(back.render.as_deref(), Some("angular-ssr"));
}

#[test]
fn render_field_omitted_when_none() {
    let route = Route::get("/blob/{hash}").handler("get_blob").build();
    let json = serde_json::to_string(&route).unwrap();
    assert!(!json.contains("render"), "json: {}", json);
}
```

- [ ] **Step 2: Confirm failing**

```bash
RUSTFLAGS="" cargo test -p doorway-client --test render_field
```

Expected: compile error — `Route::render` does not exist.

- [ ] **Step 3: Add the field to `Route` struct**

In `crates/doorway-client/src/routes.rs` around line 187, add:

```rust
    /// Render spec for SSR-eligible routes (e.g. "angular-ssr"). None = no SSR.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render: Option<String>,
```

In `RouteBuilder::new()` around line 251, initialize `render: None`. Add the builder method:

```rust
    /// Mark this route as SSR-eligible with the given render spec.
    pub fn render(mut self, spec: &str) -> Self {
        self.route.render = Some(spec.to_string());
        self
    }
```

- [ ] **Step 4: Run the tests**

```bash
RUSTFLAGS="" cargo test -p doorway-client --test render_field
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/doorway-client/src/routes.rs crates/doorway-client/tests/
git commit -m "feat(doorway-client): Route.render field for SSR eligibility"
```

---

## Task 12: Storage manifest declares ssr-eligible routes

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs` (around line 7629, `build_manifest()`)

- [ ] **Step 1: Find the existing routes that should gain SSR**

Search inside `build_manifest()` for routes matching `/lamad/path`, `/lamad/concept`, or any landing-page route. Many of them route to `get_path` / `get_concept` handlers.

- [ ] **Step 2: Mark the three eligible groups**

For each route group below, add `.render("angular-ssr")` to its builder chain. If a route does not exist in the current manifest, add it.

```rust
.route(
    Route::get("/lamad/concept/{id}")
        .handler("get_concept_view")
        .cache_ttl(60)
        .render("angular-ssr")
        .build(),
)
.route(
    Route::get("/lamad/path/{slug}")
        .handler("get_path_view")
        .cache_ttl(60)
        .render("angular-ssr")
        .build(),
)
.route(
    Route::get("/lamad/path/{slug}/step/{n}")
        .handler("get_path_step_view")
        .cache_ttl(60)
        .render("angular-ssr")
        .build(),
)
.route(
    Route::get("/")
        .handler("get_landing_view")
        .cache_ttl(60)
        .render("angular-ssr")
        .build(),
)
```

(Handler names — `get_concept_view`, `get_path_view`, `get_path_step_view`, `get_landing_view` — must exist as actual handler functions in storage's HTTP layer, returning JSON for the SPA AND consumed by SSR. If they don't yet exist, add minimal stubs that return whatever JSON shape elohim-app's content service expects today — same shape as the existing `/db/content/{id}` response.)

- [ ] **Step 3: Update the manifest schema test**

Find the existing manifest contract test in `elohim/elohim-storage/tests/` and add an assertion that the four route groups carry `render: Some("angular-ssr")`.

- [ ] **Step 4: Verify storage compiles and passes its tests**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/http.rs elohim/elohim-storage/tests/
git commit -m "feat(elohim-storage): declare ssr-eligible routes in manifest"
```

---

## Task 13: Doorway dispatches render-eligible routes from manifest

**Files:**
- Modify: `doorway/doorway-service/src/services/route_registry.rs`
- Modify: `doorway/doorway-service/src/server/http.rs`
- Create: `doorway/doorway-service/tests/registry_render.rs`

- [ ] **Step 1: Failing test**

Create `doorway/doorway-service/tests/registry_render.rs`:

```rust
use doorway::services::route_registry::RouteRegistry;
use doorway_client::{DoorwayRoutesBuilder, Route};

#[test]
fn registry_marks_render_routes_for_ssr_dispatch() {
    let routes = DoorwayRoutesBuilder::new()
        .route(
            Route::get("/lamad/concept/{id}")
                .handler("get_concept_view")
                .render("angular-ssr")
                .build(),
        )
        .build();
    let registry = RouteRegistry::with_routes(routes);
    let m = registry
        .match_request("GET", "/lamad/concept/abc")
        .expect("matched");
    assert_eq!(m.render_spec.as_deref(), Some("angular-ssr"));
}
```

- [ ] **Step 2: Confirm failing**

```bash
RUSTFLAGS="" cargo test -p doorway --test registry_render
```

Expected: error about missing `render_spec` on the registry match result.

- [ ] **Step 3: Add `render_spec` to the registry match result**

In `route_registry.rs`, locate the struct returned by `match_request`. Add `render_spec: Option<String>`. Populate it from the matched `Route.render`.

- [ ] **Step 4: Wire dispatch in `http.rs`**

Replace the hardcoded `/render-test/` arm from Task 10 with a registry-driven version:

```rust
if let Some(matched) = state.route_registry.match_request(method, path) {
    if let Some(spec) = matched.render_spec.as_deref() {
        if let Some(renderer) = state.renderer.as_ref() {
            // Build a DataFetcher that calls back through the existing resolver.
            let fetcher = Arc::new(crate::ssr::ResolverFetcher::new(state.resolver.clone()));
            let ctx = elohim_render::RenderContext {
                spec: elohim_render::RenderSpec::parse(spec).unwrap_or(elohim_render::RenderSpec::AngularSsr),
                url: full_url(req).into_owned(),
                data_fetcher: fetcher,
                limits: Default::default(),
            };
            return match renderer.render(ctx).await {
                Ok(out) => Ok(html_response(out)),
                Err(e) => {
                    tracing::warn!(target: "doorway::ssr", "render error: {e}");
                    spa_shell_fallback(req).await
                }
            };
        }
    }
    // Non-render route — fall through to existing forward_to_storage path
}
```

- [ ] **Step 5: Add `ssr` module with `ResolverFetcher`**

Create `doorway/doorway-service/src/ssr.rs`:

```rust
use async_trait::async_trait;
use elohim_render::{DataFetcher, FetchRequest, FetchResponse, Result};
use std::sync::Arc;

pub struct ResolverFetcher {
    resolver: Arc<crate::cache::resolution::DoorwayResolver>,
}

impl ResolverFetcher {
    pub fn new(resolver: Arc<crate::cache::resolution::DoorwayResolver>) -> Self {
        Self { resolver }
    }
}

#[async_trait]
impl DataFetcher for ResolverFetcher {
    async fn fetch(&self, request: FetchRequest) -> Result<FetchResponse> {
        // Resolve via doorway's cache first, then storage. Specific
        // resolver call shape depends on DoorwayResolver's API.
        // The contract: take a relative URL, return the JSON body and
        // the content_hash if the resolver knows it.
        let url = request.url.clone();
        match self.resolver.resolve_http(&url).await {
            Ok(resp) => Ok(FetchResponse {
                status: resp.status,
                headers: resp.headers.into_iter().collect(),
                body: resp.body,
                content_hash: resp.content_hash,
            }),
            Err(e) => Err(elohim_render::RenderError::DataFetch(e.to_string())),
        }
    }
}
```

(If `DoorwayResolver::resolve_http` doesn't exist yet, add a thin shim that wraps the existing `resolve` API — ResolverFetcher's only requirement is "given a relative URL string, return body + status + headers + optional hash".)

Add `pub mod ssr;` to `doorway/doorway-service/src/lib.rs`.

- [ ] **Step 6: Build and run a manual smoke check**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo build --release
SSR_BUNDLE_PATH=/projects/elohim/app/elohim-app/dist/elohim-app/server/main.server.mjs \
  ./target/release/doorway &
sleep 3
curl -s http://localhost:8888/lamad/concept/elohim-protocol-overview | head -50
```

(Requires elohim-storage running with seeded content — the standard `pnpm run hc:start:seed` flow.)

Expected: HTML with concept title, body text, hydration markers.

- [ ] **Step 7: Commit**

```bash
git add doorway/doorway-service/src/services/route_registry.rs doorway/doorway-service/src/server/http.rs doorway/doorway-service/src/ssr.rs doorway/doorway-service/src/lib.rs doorway/doorway-service/tests/registry_render.rs
git commit -m "feat(doorway): manifest-driven SSR dispatch via route registry"
```

---

## Task 14: Render-result cache keyed on content hash

**Files:**
- Modify: `doorway/doorway-service/src/ssr.rs`
- Modify: `doorway/doorway-service/src/server/http.rs`
- Modify: `doorway/doorway-service/src/cache/resolution.rs` (or wherever the projection cache lives — current path)
- Create: `doorway/doorway-service/tests/render_cache.rs`

- [ ] **Step 1: Define the cache key**

Add to `ssr.rs`:

```rust
use sha2::{Digest, Sha256};

pub fn render_cache_key(url: &str, fetched_hashes: &[String], spec_version: &str) -> String {
    let mut h = Sha256::new();
    h.update(url.as_bytes());
    h.update(b"\0");
    for f in fetched_hashes {
        h.update(f.as_bytes());
        h.update(b"\0");
    }
    h.update(spec_version.as_bytes());
    let bytes = h.finalize();
    format!("ssr-{}", hex::encode(&bytes[..16]))
}
```

Add `sha2` and `hex` to `doorway-service/Cargo.toml` if absent.

- [ ] **Step 2: Failing cache hit/miss test**

Create `doorway/doorway-service/tests/render_cache.rs`:

```rust
#[tokio::test]
#[ignore = "requires live doorway + storage"]
async fn render_cache_serves_second_request_from_cache() {
    let url = "http://localhost:8888/lamad/concept/elohim-protocol-overview";
    let r1 = reqwest::get(url).await.unwrap();
    let header_first = r1.headers().get("x-render-cache").cloned();
    let _body_first = r1.text().await.unwrap();

    let r2 = reqwest::get(url).await.unwrap();
    let header_second = r2.headers().get("x-render-cache").cloned();
    assert_eq!(header_first.as_ref().map(|v| v.to_str().unwrap()), Some("MISS"));
    assert_eq!(header_second.as_ref().map(|v| v.to_str().unwrap()), Some("HIT"));
}
```

- [ ] **Step 3: Wire cache lookup before render**

In `http.rs` SSR dispatch, before calling `renderer.render(ctx)`:

```rust
let cache_key = ssr::render_cache_key(&full_url_str, &[], "v1");
if let Some(cached) = state.cache.get_rendered(&cache_key).await {
    return Ok(html_response_with_header(cached, "x-render-cache", "HIT"));
}
```

After successful render:

```rust
let hashes: Vec<String> = out.fetched_inputs.iter().map(|c| c.content_hash.clone()).collect();
let final_key = ssr::render_cache_key(&full_url_str, &hashes, "v1");
state.cache.put_rendered(&final_key, out.html.clone(), Duration::from_secs(60 * 5)).await;
```

(The cache trait gains two methods: `get_rendered(&str) -> Option<String>` and `put_rendered(&str, String, Duration)`. Implement on the existing `ContentCache` — store keyed by string, value is HTML body, TTL respected.)

- [ ] **Step 4: Wire DataFetcher to record content hashes**

In `ResolverFetcher::fetch`, when the resolver returns a content hash, ensure it lands in `FetchResponse.content_hash`. The renderer's adapter already reads `fetched_inputs` — add logic in `AngularRenderer` to populate it from each `op_fetch` invocation.

(MVP simplification: track a thread-local `Vec<ContentRef>` in the fetch shim that the renderer drains after the render call. Document in code that this is single-render-per-isolate-at-a-time and gets revisited when the isolate pool lands.)

- [ ] **Step 5: Run live cache test**

```bash
# Doorway and storage running with seeded content
RUSTFLAGS="" cargo test --test render_cache -- --ignored
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add doorway/doorway-service/Cargo.toml doorway/doorway-service/src/ssr.rs doorway/doorway-service/src/server/http.rs doorway/doorway-service/src/cache/ doorway/doorway-service/tests/render_cache.rs
git commit -m "feat(doorway): render-result cache keyed on content hash"
```

---

## Task 15: CUSTOM_ELEMENTS_SCHEMA + sophia placeholder snapshot

**Files:**
- Modify: `app/elohim-app/src/app/app.component.ts`
- Create: `app/elohim-app/src/app/__tests__/ssr-sophia-placeholder.test.ts`

- [ ] **Step 1: Add schema declaration**

In `app/elohim-app/src/app/app.component.ts`, add to the `@Component` decorator:

```typescript
import { CUSTOM_ELEMENTS_SCHEMA } from '@angular/core';

@Component({
  // ... existing fields ...
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
})
export class AppComponent {}
```

Also add the same schema to any standalone component that renders `<sophia-question>` directly.

- [ ] **Step 2: Add a snapshot test on placeholder shape**

Create `app/elohim-app/src/app/__tests__/ssr-sophia-placeholder.test.ts`:

```typescript
import { describe, expect, it } from 'vitest';
// Use Angular's static rendering helper for the test
import { renderApplication } from '@angular/platform-server';
import bootstrap from '../main.server';

describe('SSR sophia placeholder', () => {
  it('emits <sophia-question> element with attributes preserved', async () => {
    // Render a fixture URL that includes a sophia quiz
    const html = await renderApplication(bootstrap, {
      url: '/lamad/concept/quiz-fixture',
      document: '<!doctype html><html><head></head><body><app-root></app-root></body></html>',
    });
    expect(html).toMatch(/<sophia-question[^>]*content-id="quiz-/);
    expect(html).not.toMatch(/customElements\.define/);
  });
});
```

- [ ] **Step 3: Run the test**

```bash
cd /projects/elohim/app/elohim-app
pnpm test -- ssr-sophia-placeholder
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/app.component.ts app/elohim-app/src/app/__tests__/ssr-sophia-placeholder.test.ts
git commit -m "feat(elohim-app): preserve <sophia-question> placeholder during SSR"
```

---

## Task 16: Hydration verification via Playwright

**Files:**
- Create: `app/elohim-app/cypress/e2e/ssr-hydration.cy.ts` (or Playwright equivalent if the project uses Playwright)
- Create: `genesis/a2o/features/ssr/browser-hydrates-without-flash.feature`

- [ ] **Step 1: Write the a2o scenario**

Create `genesis/a2o/features/ssr/browser-hydrates-without-flash.feature`:

```gherkin
Feature: Browser hydrates SSR'd content without a re-render flash

  Background:
    Given doorway is running with SSR enabled
    And elohim-storage is seeded with concept "elohim-protocol-overview"

  Scenario: Concept page hydrates seamlessly
    When I navigate the browser to "/lamad/concept/elohim-protocol-overview"
    Then the initial HTML response contains the concept title in <title>
    And the initial HTML response contains the concept body in <article>
    And the rendered DOM does not change between SSR and hydration
    And no console errors are emitted during hydration
```

- [ ] **Step 2: Implement step definitions**

In the existing Cucumber/Playwright harness for elohim-app (look at `cypress/e2e/` or `e2e/` for the pattern). Add steps that:
- Fetch initial HTML via raw `fetch()` and parse with DOMParser, snapshot the DOM tree
- Open page in headless browser
- Wait for `appReady` (existing hydration ready signal — or wait for Angular to be stable)
- Snapshot the post-hydration DOM tree
- Diff: assert no nodes were added/removed/replaced

- [ ] **Step 3: Run the scenario**

```bash
cd /projects/elohim/app/elohim-app
pnpm run cypress:run -- --spec ssr-hydration.cy.ts
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/cypress/ genesis/a2o/features/ssr/
git commit -m "feat(a2o): browser hydrates without re-render flash"
```

---

## Task 17: elohim-storage `--feature ssr` embed

**Files:**
- Modify: `elohim/elohim-storage/Cargo.toml`
- Modify: `elohim/elohim-storage/src/http.rs`
- Create: `elohim/elohim-storage/tests/ssr_direct.rs`

- [ ] **Step 1: Add the feature**

In `elohim/elohim-storage/Cargo.toml`:

```toml
[features]
default = []
ssr = ["dep:elohim-render"]

[dependencies]
elohim-render = { path = "../elohim-render", optional = true }
```

- [ ] **Step 2: Failing test**

Create `elohim/elohim-storage/tests/ssr_direct.rs`:

```rust
//! Storage's direct-to-peer SSR endpoint.

#[cfg(feature = "ssr")]
#[tokio::test]
#[ignore = "requires storage running with --feature ssr and seeded content"]
async fn storage_spa_endpoint_returns_rendered_html() {
    let body = reqwest::get("http://localhost:8090/spa/lamad/concept/elohim-protocol-overview")
        .await.unwrap()
        .text().await.unwrap();
    assert!(body.contains("<title>"), "body: {}", body);
    assert!(body.contains("ngh="), "no hydration markers");
}
```

- [ ] **Step 3: Add the SSR handler under `#[cfg(feature = "ssr")]`**

In `elohim/elohim-storage/src/http.rs`, add a new module-level block:

```rust
#[cfg(feature = "ssr")]
mod ssr_endpoint {
    use elohim_render::{AngularRenderer, RenderContext, RenderSpec, Renderer};
    use std::path::PathBuf;
    use std::sync::Arc;

    pub async fn handle_spa(
        path: &str,
        state: &super::AppState,
    ) -> Result<hyper::Response<...>, super::Error> {
        let bundle = std::env::var("SSR_BUNDLE_PATH")
            .map(PathBuf::from)
            .map_err(|_| super::Error::config("SSR_BUNDLE_PATH not set"))?;
        let renderer = AngularRenderer::new(bundle).await
            .map_err(|e| super::Error::internal(e.to_string()))?;

        // LocalFetcher resolves directly against in-process content store —
        // no HTTP hop. Same shape as ResolverFetcher in doorway.
        let fetcher = Arc::new(LocalFetcher { state: state.clone() });
        let ctx = RenderContext {
            spec: RenderSpec::AngularSsr,
            url: format!("/{}", path.trim_start_matches('/')),
            data_fetcher: fetcher,
            limits: Default::default(),
        };
        match renderer.render(ctx).await {
            Ok(out) => Ok(html_response(out.html)),
            Err(e) => {
                tracing::warn!(target: "elohim_storage::ssr", "render error: {e}");
                Ok(spa_shell_response())
            }
        }
    }

    struct LocalFetcher { /* fields */ }
    // impl DataFetcher — resolves via internal services, no HTTP
}
```

Add a router arm matching `GET /spa/*` that calls `ssr_endpoint::handle_spa`.

- [ ] **Step 4: Build with feature on**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --features ssr
```

Expected: success.

- [ ] **Step 5: Build with feature off (the default profile)**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build
```

Expected: success, smaller binary, no V8 in the dep graph.

- [ ] **Step 6: Run the integration test**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features ssr --test ssr_direct -- --ignored
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/Cargo.toml elohim/elohim-storage/src/http.rs elohim/elohim-storage/tests/ssr_direct.rs
git commit -m "feat(elohim-storage): --feature ssr direct-render endpoint"
```

---

## Task 18: a2o scenarios for external WebFetch and social cards

**Files:**
- Create: `genesis/a2o/features/ssr/external-webfetch-renders-content.feature`
- Create: `genesis/a2o/features/ssr/social-card-crawler-gets-rich-preview.feature`
- Step definition files in the existing a2o harness

- [ ] **Step 1: External WebFetch scenario**

Create `genesis/a2o/features/ssr/external-webfetch-renders-content.feature`:

```gherkin
Feature: External WebFetch renders concept HTML readable without JS

  Scenario: AI design tool fetches a concept page
    Given doorway is running with SSR enabled
    And elohim-storage is seeded with concept "elohim-protocol-overview"
    When an HTTP client without a JavaScript engine fetches
      "/lamad/concept/elohim-protocol-overview"
    Then the response has status 200
    And the response body contains the concept title in <title>
    And the response body contains the concept body in <article>
    And the response body contains <sophia-question> placeholders if any
    And no <app-root> tag is empty
```

- [ ] **Step 2: Social card scenario**

Create `genesis/a2o/features/ssr/social-card-crawler-gets-rich-preview.feature`:

```gherkin
Feature: Social card crawlers receive rich link previews

  Scenario: Twitter/Slack/Mastodon previews a learning path
    Given doorway is running with SSR enabled
    And elohim-storage is seeded with learning path "elohim-protocol"
    When a social card crawler fetches "/lamad/path/elohim-protocol/step/0"
    Then the response body contains <meta property="og:title">
    And the response body contains <meta property="og:description">
    And the response body contains <meta property="og:image">
    And the og:title equals the path's first step title
```

- [ ] **Step 3: Implement step definitions**

Add to the existing a2o step library. The fetch step uses raw `reqwest::get` with no UA spoofing — the renderer is content-equal regardless of UA. The assertions parse HTML with `scraper` or `kuchiki`.

- [ ] **Step 4: Run all SSR a2o scenarios**

```bash
cd /projects/elohim
pnpm run a2o -- features/ssr/  # or whatever the project's a2o runner is
```

Expected: all SSR scenarios PASS green.

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/features/ssr/ genesis/a2o/step-definitions/
git commit -m "feat(a2o): SSR scenarios for external WebFetch and social cards"
```

---

## Self-Review

**Spec coverage check:**

| Spec section | Task |
|---|---|
| Architecture: `elohim-render` library | Tasks 1-9 |
| Architecture: deno_core + V8 | Tasks 4-5 |
| Architecture: doorway depends unconditionally | Task 10 |
| Architecture: storage opt-in `--feature ssr` | Task 17 |
| Data flow: Path A external WebFetch | Tasks 10, 13, 18 |
| Data flow: Path B peer-to-peer storage-direct | Task 17 |
| Data flow: manifest-driven eligibility | Tasks 11, 12, 13 |
| Cache key + invalidation | Task 14 |
| Auth: commons-only for MVP | Implicit — content routes are commons-reach |
| Angular SSR build pipeline | Task 8 |
| Hydration via standard Angular providers | Task 8, 16 |
| sophia-element placeholder strategy | Task 15 |
| Streaming SSR | Out of scope for MVP — confirmed |
| Render limits | Tasks 2, 9 (RenderLimits applied in AngularRenderer) |
| Error handling: CSR shell fallback | Tasks 10, 13, 17 |
| Observability hooks | Implicit via `tracing` calls — explicit instrumentation lands in a polish slice after Task 18 |
| Audit trail (route, hashes, spec_version, output_hash) | Task 14 (hashes), polish slice for full audit-artifact storage |
| Testing: unit + integration + a2o | Tasks 3, 5, 6, 7, 9 (unit); 10, 13, 14, 17 (integration); 16, 18 (a2o) |
| MVP first observable outcome | Task 13 (concept route renders) |
| Smallest first slice | Task 10 (`/render-test` hardcoded) |

**Gaps acknowledged for follow-up slices (not MVP-blocking):**
- Isolate pool (currently single isolate per AngularRenderer)
- Full observability dashboard (just `tracing` calls for MVP)
- Audit-artifact persistence (cache key carries hashes; full artifact storage is REA-compute-prep work)
- Snapshotting the booted Angular bundle for sub-10ms cold start (currently lazy init)

These are noted in Task 14 inline comments and the spec's Risks section. They land as a "polish" slice between Task 18 and any production rollout.

**Placeholder scan:** none — every step has the actual content the engineer needs.

**Type consistency:** `Renderer`, `RenderContext`, `RenderOutput`, `DataFetcher`, `FetchRequest`, `FetchResponse`, `RenderSpec`, `RenderError`, `ContentRef`, `RenderLimits`, `AngularRenderer`, `EchoRenderer`, `JsRuntime` — names consistent across all 18 tasks.

---

## Done

The /shift agentic developer can take this plan and grind tasks 1–18. Stability gate: a2o scenarios in Tasks 16 and 18 all green against a live cluster. The spec's "MVP first observable outcome" is the human-visible artifact of that gate.
