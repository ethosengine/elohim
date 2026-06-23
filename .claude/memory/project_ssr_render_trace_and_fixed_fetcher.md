---
name: project_ssr_render_trace_and_fixed_fetcher
description: "elohim-render is the framework-agnostic p2p-native SSR core; AngularRenderer's fetcher is fixed at construction (ctx.data_fetcher ignored) → doorway per-request SSR user-credential is a latent no-op"
metadata: 
  node_type: memory
  type: project
  originSessionId: a83484a6-6492-4e83-8c53-5b01785aa065
---

The SSR core is **`elohim/elohim-render/`** — a framework-agnostic V8/deno_core runtime (`Renderer` trait; AngularRenderer/EchoRenderer plug in), consumed by BOTH doorway AND elohim-storage (`src/ssr.rs` in each). SSR is p2p-native: a capable storage peer renders its own content, doorway is just the web2 edge. Instrument SSR concerns in the core crate, not in doorway.

**Render-trace instrument** (shipped 2026-06-12, commits b9f12dab9 / 504324318 / 5fcd8380c on feat/frontend-eyes-sprint): `RenderOutput.trace` carries a `RenderTerminal` (`rendered`/`rendered-empty`/`stalled`/`timed-out`/`errored`) — the cut that distinguishes a truthful empty (404/204/`[]`) from a degenerate stall (the "blank page, no error" class: EprRouter-empties, DHT-anchor-gap 404s). `TracingFetcher` decorates any `DataFetcher` below the V8 boundary; a per-fetch **soft-deadline** (`DEFAULT_SOFT_BUDGET_MS=1200`, overridable via `AngularRenderer::with_soft_budget`) converts a hang into a recorded `Stalled` + fast fallback. doorway emits `x-ssr-terminal`/`x-ssr-fetches`/`x-ssr-wall-ms`/`x-ssr-trace-id` headers + `doorway::ssr::trace` Loki log; `RenderTraceStats` aggregates per-peer terminal+latency, served at `GET /admin/render-stats` (the feed-forward signal for compute-commitment diversity tuning, feeding existing [[project_rea_compute_commitment_primitive]] — no new DHT entity).

**Per-request fetcher swap (FIXED 2026-06-12, commit a89f255f1).** Originally `AngularRenderer` baked the fetcher in at construction and IGNORED `ctx.data_fetcher` → doorway's per-request user-credential fetcher was dropped → authenticated SSR fetched as anonymous (reach-aware content → public). Now `JsRuntime::set_fetcher` swaps the fetch op's backing `DataFetcher` in OpState (op_fetch reads the handle per call); the render worker wraps each request's `ctx.data_fetcher` in a fresh `TracingFetcher` and swaps it in before eval. **render() uses `ctx.data_fetcher`**; the `new(bundle, fetcher)` arg is just the isolate bootstrap default (never used for a real render). Consumers: doorway builds the per-request fetcher with the user credential at the SSR call site (`http.rs`, `build_ssr_user_credential`) — it now flows. Per-request fault injection wraps that per-request fetcher (`ssr.rs::maybe_inject_stall_fault`, env `DOORWAY_SSR_FAULT_STALL_PATH`), NOT the bootstrap. A marker-fetcher integration test pins "per-request fetcher used, construction one not."

Build elohim-render native: `RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/elohim-render-tgt cargo test` (deno_core is native; the WASM getrandom-custom flag would break the link). Doorway/storage warm targets: `/tmp/doorway-target`, `/tmp/elohim-storage-target` (the pool slots hit the `/projects-volume` fingerprint-ENOENT quirk).
