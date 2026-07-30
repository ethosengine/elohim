---
id: "backlog-elohim-render-isolate-reuse-trust-boundary"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-render: V8 isolate reuse across sequential renders is a cross-request bleed channel"
slug: "elohim-render-isolate-reuse-trust-boundary"
written: "2026-07-30"
author: "angular22-campaign"
status: "backlog"
priority: "low"
tags: [elohim-render, ssr, security, isolate, trust-boundary, deno_core]
cites:
  - elohim/elohim-render/src/runtime.rs
  - elohim/elohim-render/src/shim/mod.rs
  - elohim/elohim-render/src/shim/web_api.js
---

## What

`elohim_render::JsRuntime` reuses the same `deno_core` V8 isolate across sequential renders,
swapping only the `DataFetcher` per request. The JS-side globals (`fetch`, `Headers`,
`Response`, module-scope state a bundle may stash on `globalThis`, any per-app singleton the
Angular bundle constructs at bootstrap) persist across that swap unless something explicitly
clears them.

## Why this is a trust-boundary concern, not just a perf detail

If a future caller ever runs isolate reuse across renders for **different users** (a
per-user/per-tenant credentialed fetcher swapped in per request — the natural next step once
the render path carries auth context), the isolate becomes a channel for cross-request bleed:

- **JS global state** written by user A's render (module-level caches, memoized computed
  values, anything the Angular bundle or a `@defer` block stashes outside component state) is
  still live in `globalThis` when user B's render runs in the same isolate.
- **The web_api.js shims are install-guarded** (`if (typeof globalThis[name] === "undefined")`
  — see the file's `--- Install ---` section), which prevents re-installation from clobbering
  live state across renders, but that guard is exactly the mechanism that lets stale state
  survive: nothing resets `Headers`/`Response`/`ReadableStream` instances or app-bundle module
  state between requests.
- **The app bundle itself** is the larger surface: Angular DI singletons, RxJS `BehaviorSubject`
  state held in a provided-in-root service, anything memoized at module scope in the built
  `main.server.mjs` — none of that is isolate-reset between calls to `renderApplication()`.

## Pre-existing, not introduced by the v22 campaign

This is architecture that predates the Angular 22 migration; the v22 work touched web_api.js's
install guards but did not create the reuse pattern. Filed now because the review pass that
produced the web_api.js hardening (see the sibling Job-3 commit) surfaced it as an adjacent
concern while reading `src/shim/mod.rs`'s isolate lifecycle notes.

## Current mitigating factors (why this isn't urgent today)

- Every render today uses the SAME trust level: doorway's own service-to-service fetch,
  never a per-user credentialed fetcher. There is no live path where isolate reuse crosses a
  real trust boundary yet.
- Renders are stateless from the caller's perspective (URL in, HTML out) — no render result
  has ever depended on residue from a prior render, so no bug has manifested.

## Options (not yet decided)

1. **Document the trust assumption explicitly** in `src/runtime.rs` / `src/shim/mod.rs`:
   isolate reuse is safe ONLY while every render shares one fetcher trust level; the day a
   per-user credentialed fetcher is introduced, this doc is the tripwire that forces a design
   decision before shipping.
2. **Realm-per-render**: `deno_core` supports secondary V8 contexts (realms) within one
   isolate. A fresh realm per render gets a clean `globalThis` at near-zero cost (no new V8
   isolate spin-up), which would close the JS-global-state channel while keeping the
   perf win isolate reuse exists for. Needs a spike to confirm realm creation cost and
   whether `deno_core`'s extension/op wiring works per-realm here.
3. **Realm recycling cadence**: cheaper middle ground — recycle (destroy + recreate) the
   isolate after N renders or M minutes, bounding exposure window without paying per-render
   realm cost. Weaker guarantee than (2); easier to land first.

## Acceptance sketch (once picked up)

1. Decide (1)/(2)/(3) above — needs rust-architect judgment on `deno_core` realm API maturity
   and the actual perf budget elohim-render renders under.
2. If (2) or (3): a regression test that mutates a JS global in render N and asserts render
   N+1 does not observe it (mirrors the isolate-lifecycle intent, catches a future regression
   the same way `tests/shim.rs`'s ASCII-purity test catches a different structural drift).
3. Update `src/shim/mod.rs`'s isolate-lifecycle doc comment to state the resolved trust
   contract explicitly, whichever option is picked.

**Owner:** rust-architect.
