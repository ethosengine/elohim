---
id: "backlog-elohim-render-v22-elohim-app-stall"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-render SSR: elohim-app '/' route stalls to the wall-time limit (HomeComponent subtree, idle-wait)"
slug: "elohim-render-v22-elohim-app-stall"
written: "2026-07-30"
author: "angular22-campaign"
status: "backlog"
priority: "medium"
tags: [elohim-render, ssr, deno-core, angular22, zoneless, whenstable]
cites:
  - elohim/elohim-render/src/shim/web_api.js
  - elohim/elohim-render/src/shim/fetch.js
  - elohim/elohim-render/examples/render_url.rs
  - app/elohim-app/src/app/app.routes.ts
  - app/elohim-app/src/app/components/home/home.component.ts
  - app/elohim-app/src/app/app.config.server.ts
---

## What

Rendering elohim-app's `/` route through `elohim-render`'s deno_core isolate never completes:
`renderApplication` rides to the wall-time limit (60s in `examples/render_url.rs`). Every other
route measured renders correctly and fast. This is the residual after the Angular 22
FetchBackend web-API polyfill landed (`fix(render): polyfill web APIs for Angular 22
FetchBackend in the deno_core isolate`, 977d17814) — that commit fixed the whole-runtime
breakage; this entry records the one route that still hangs.

## Measured state (2026-07-30, commit 977d17814, debug build)

Bundles: `app/{elohim-app,lamad}/dist/*/server/main.server.mjs` (v22 production server builds).
Harness: `RUSTFLAGS="" cargo run --example render_url -- <bundle> <url>`, whose `FailFetcher`
**rejects** every fetch — byte-identical semantics to the Node ground-truth stub.

| bundle | url | terminal | fetches | wall_ms | html bytes | Node ground truth |
|---|---|---|---|---|---|---|
| lamad | `/lamad` | errored | 2 | 307 | 4208 | 4208 B / 2 fetches / ~260ms |
| elohim-app | `/` | **TIMEOUT** | **1** | 60000 | 0 | 60740 B / 30 fetches / 321ms |
| elohim-app | `/identity/login` | errored | 1 | 338 | 1376 | (not measured) |
| elohim-app | `/zzz-nonexistent` | errored | 1 | 312 | 5739 | (not measured) |

Before the polyfill commit, for comparison: lamad `/lamad` panicked with "Promise resolution is
still pending but the event loop has already resolved"; **all three** elohim-app routes timed out
at 60s with **zero** fetches. So the polyfill moved 3 of 4 measured renders from broken to
matching-Node, and moved `/` from "0 fetches, dead at the AbortController constructor" to
"1 fetch, dies later".

## Root-cause evidence — what is RULED OUT

A heartbeat probe (a pristine `Deno.core.queueUserTimer` repeating timer installed in the driver
script before the bundle import) **kept firing for the entire stall** — 49 beats over 25s, evenly
spaced at 500ms. Combined with `user 0m0.319s` of CPU against `real 1m0.017s`, this establishes:

- **NOT a synchronous V8 spin.** The event loop ticks normally throughout. An earlier
  investigation pass concluded "V8 spinning" from a heartbeat that stopped; that reading does not
  reproduce and should be discarded.
- **NOT event-loop starvation / exhaustion.** deno_core is idle-waiting on a promise that never
  settles, with live timers (the app's 3 `setInterval`s) keeping `poll_event_loop` pending. That
  is also why `/` times out where lamad used to panic: lamad has no live timers, so its loop
  drained and deno_core raised "Promise resolution is still pending".

Three hypotheses about the new fetch/Response shim are **ruled out by fetch accounting**: the
render issues exactly ONE fetch, and it is `GET https://doorway.elohim.host/health` from
`AppComponent.testDoorwayConnection` — a **raw `globalThis.fetch`**, not HttpClient. Verified by
wrapping `globalThis.fetch` in the driver and printing the caller stack:

```
at globalThis.fetch (ext:fetch_ext/fetch.js)
at polyfills.server.mjs (zone.js fetch patch)
at i.testDoorwayConnection (chunk-HK72D2A5.mjs)
```

Therefore, in the failing render:

- **Zero `Response` objects are ever constructed** — the single fetch rejects at the Rust
  `DataFetcher`. So `ReadableStream`/`text()`/`json()`/`Headers` completion semantics cannot be
  the blocker.
- **Zero `FetchBackend` requests are ever issued** — no `HttpClient` traffic at all, so no
  rxjs `timeout`/`retry` chain and no `AbortSignal` event is in play on the stalling path.
- **The fetcher's rejection semantics match Node exactly** (`[fetch → FAIL]` is printed, then the
  promise rejects), so a resolve-vs-reject divergence is not it.

Route-specificity is the decisive discriminator: `/identity/login` and `/zzz-nonexistent` render
in ~330ms **with the identical single rejecting fetch**. Only the `/` route — i.e. the
`HomeComponent` subtree — hangs.

## Root-cause evidence — what IS established

`/` is the only route whose `loadComponent` is `import('./components/home/home.component')`
(`app/elohim-app/src/app/app.routes.ts`). Instrumenting `NodeShimLoader::{resolve,load}` with a
temporary `ELOHIM_RENDER_TRACE_MODULES` env gate shows 61 module loads during the stalling
render, ending in this exact order:

```
[load] dyn=true .../angular-app-manifest.mjs
[load] dyn=true .../chunk-O7TGWBOU.mjs        <- the HomeComponent chunk
[load] dyn=true .../chunk-KNLKBGAS.mjs
[resolve OK]  /wasm/elohim-cache-core/elohim_cache_core.js -> file:///wasm/...   (x4)
[load] dyn=true file:///wasm/elohim-cache-core/elohim_cache_core.js
<no further module activity for the remaining 59s>
```

So: the HomeComponent chunk **does** load, and the DI graph gets far enough to reach the
ContentResolver's WASM probe (`app/elohim-library/projects/elohim-service/src/cache/content-resolver.ts`
`loadWasmModule`). Zero `resolve ERR` lines — no unresolved bare specifier.

The failing `/wasm/...` dynamic import is **not** the blocker, on two independent grounds:

1. In an isolated probe, a dynamic import of a nonexistent `file://` module **rejects cleanly**
   ("Failed to load … No such file or directory (os error 2)") within one tick and the loop stays
   healthy. The `xhr2`-era note in `src/angular.rs` claiming failed dynamic imports "leave pending
   dynamic module evaluations in the V8 event loop" does not reproduce on the current deno_core.
2. Materializing a stand-in `/wasm/elohim-cache-core/elohim_cache_core.js` so the import RESOLVES
   made things strictly worse — the stall then happened *earlier*, before the health fetch. (The
   stand-in was removed; it is a global-filesystem side effect, do not leave one behind.)

No `console.*` output and no unhandled promise rejection is emitted during the stall (verified
with console re-pointed at `Deno.core.print` and a printing
`setUnhandledPromiseRejectionHandler`). The stall is silent.

## Where to look next

The app is configured with `provideZonelessChangeDetection()` (see the long rationale in
`app/elohim-app/src/app/app.config.server.ts`). Under zoneless, `ApplicationRef.whenStable()`
resolves on `PendingTasks` alone, and the only contributors are **router navigation** and
**HttpClient requests**. Since zero HttpClient requests are ever issued, the prime suspect is a
**router initial-navigation `PendingTask` that never completes** for the `''` route,
somewhere between `loadComponent` resolving (proven to happen) and `HomeComponent` being
constructed (proven NOT to happen — neither `ConfigService`'s `/assets/config.json` nor
`FooterComponent`'s `/version.json`, which are Node's fetches #2 and #3, ever fire).

Suggested next probes, in order of expected yield:

1. Instrument Angular's `PendingTasks` (or wrap `setTimeout`/`queueUserTimer` scheduling with
   caller stacks) and dump what is outstanding at T+5s — this should name the stuck task directly.
2. Bisect `HomeComponent`'s child set (Hero, Crisis, Vision, ElohimHost, DesignPrinciples,
   LearningSuccess, PathForward, CallToAction, Footer) against a trimmed local build. The
   `<elohim-*>` Lit custom elements under `ElohimHostComponent` are the most environment-sensitive
   of these — note `whenDefined` appears in `chunk-HC2CPYH5.mjs`, which IS loaded during the
   render, and `customElements` is absent from the isolate's globals.
3. Compare the module-load trace of a Node run against the isolate run for the same URL; the first
   divergence after `chunk-KNLKBGAS.mjs` localizes it.

Reusable probe recipe (all temp examples were deleted; `examples/` must stay at
`compose_check.rs` + `render_url.rs`): driver script that (a) re-points `console.*` at
`Deno.core.print`, (b) installs a repeating `Deno.core.queueUserTimer` heartbeat, (c) wraps
`globalThis.fetch` to print ENTER/OK/THROW plus caller stack, (d) prints from
`setUnhandledPromiseRejectionHandler`; plus an env-gated `eprintln!` in
`NodeShimLoader::{resolve,load}`.

## Blast radius / why this is `medium` not `high`

The doorway falls back to a CSR shell per-app on render failure, so `/` degrades to
client-rendered rather than erroring — the same shape as the historical `fetches=0` production
signature. lamad SSR is fully restored and byte-matches Node. The cost is (a) no SSR content for
elohim-app's landing page, and (b) each `/` render occupies the single sequential isolate for the
full wall-time budget, so a burst of landing-page requests sheds with `RenderError::Busy`. Lowering
`RenderLimits::wall_time_ms` for this surface would bound that independently of the root cause.

## Second investigation pass (2026-07-30, overnight — commit f2306603a)

Root cause still OPEN, but the search space is materially narrower and two prior readings are
now **disproven by direct evidence rather than argument**. Read this before re-probing.

### Ruled OUT (do not re-run these)

- **`customElements.whenDefined` is NOT the blocker.** A live trap on `globalThis.customElements`
  (`ELOHIM_RENDER_DEBUG_HOOKS=1`) shows the Lit SSR registry installs and `define()`s **all 19
  `<elohim-*>` elements within ~10ms of isolate start** — before the stall, before the single
  fetch. `whenDefined` is **never called** during the render. This retires suggested-probe #2's
  lead above.
- **The `elohim-cache-core` wasm resolve/load pattern is NOT `/`-specific.** Re-running
  `ELOHIM_RENDER_TRACE_MODULES=1` against the *healthy* `/identity/login` (380ms) shows the
  **identical** 4×resolve / 1×load signature. The trace above reads as if it localized the stall;
  it does not. It is a universal, harmless cache probe.
- **No swallowed rejection**: unhandled-rejection suppression was made loud (log-then-suppress) —
  nothing ever fires.
- **No dangling timer**: `setTimeout`/`setInterval` wrapped with caller stacks — every scheduled
  timer is legitimate (known background timers + AppComponent's 3s abort timer).
- **Not a module-graph top-level-await deadlock**: a raw `import()` of HomeComponent's own lazy
  chunk resolves fast (clean `TypeError`, not a hang).

### Established this pass

- Node ground truth for `/` (harness matching elohim-render's fetch-reject semantics exactly):
  **~524ms, 30 fetches, 60740B** — config.json, version.json, then 14× `epr-head`+`resilience`
  pairs from the `EprRelationshipCardComponent` instances under Vision / DesignPrinciples /
  LearningSuccess / PathForward. HomeComponent's **entire** subtree constructs fine in Node.
- The isolate never gets past AppComponent's own health-check fetch (1 fetch total), so the block
  **precedes every HomeComponent descendant** — it is not one flaky child component.

### The load-bearing next probe

`app/elohim-app/src/app/app.config.server.ts`'s own comments record that under
`provideZonelessChangeDetection()`, **both router navigation and HTTP requests** are explicit
`PendingTasks` contributors. Since zero HttpClient requests are ever issued, the prime suspect is
now the **Router's per-navigation PendingTask for the `''` route never completing** — which would
stall `whenStable()` independently of any component constructor running (consistent with
HomeComponent never being constructed). Instrument `PendingTasks` directly and dump what is
outstanding at T+5s.

### Mitigation LANDED (independent of root cause)

`AngularRenderer::render()` now clamps caller-requested wall time to
`DEFAULT_MAX_WALL_TIME_MS = 10_000` (override: `ELOHIM_RENDER_MAX_WALL_TIME_MS`). The doorway's
hardcoded `wall_time_ms: 60_000` is bounded without editing that surface. A stalling route now
occupies the single sequential isolate for 10s rather than 60s, cutting the `RenderError::Busy`
burst hazard 6×. Healthy routes verified unchanged (`/identity/login` 538ms, `/zzz-nonexistent`
548ms, lamad `/lamad` 464ms / 2 fetches / 4208B); `cargo test --lib --bins` 110/110.

Diagnostics are now permanent and opt-in (off by default, zero behavior change when unset):
`ELOHIM_RENDER_DEBUG_HOOKS`, `ELOHIM_RENDER_TRACE_MODULES`, `ELOHIM_RENDER_WALL_MS`, plus a wired
`tracing_subscriber` so `console.*` from the bundle is visible in the harness. The probe recipe
above no longer needs hand-rebuilding.

### Adjacent bug found, not fixed (deserves its own entry)

`/epr/elohim-host-landing` renders real content fetches and then panics on `localStorage is not
defined` in mastery-tracking code — pre-existing, unrelated to v22, out of scope here.
