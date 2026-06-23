---
id: "backlog-wasm-cache-core-404-persists-after-preferwasm-gate"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "WASM /wasm/elohim-cache-core 404 persists after the preferWasm gate landed — content-resolver path gated, but the request still fires from an untraced trigger (cosmetic, TS fallback works)"
slug: "wasm-cache-core-404-persists-after-preferwasm-gate-2026-06-23"
written: "2026-06-23"
author: "overnight deployment-shakeout shift — post-app-rebuild verification (look capture on alpha, bundle chunk-EJFZ262E)"
status: "open"
priority: "low"
tags: [frontend, wasm, cache-core, cosmetic, console-noise, preferWasm, harbor, runtime-debug]
relatedNodeIds:
  - backlog-handoff-sprawl-decompose-2026-06-23
cites:
  - app/elohim-app/src/app/elohim/services/data-loader.service.ts
  - app/elohim-library/projects/elohim-service/src/cache/content-resolver.ts
  - app/elohim-app/src/environments/environment.alpha.ts
---

# WASM elohim-cache-core 404 persists after the preferWasm gate

## State
`GET https://alpha.elohim.host/wasm/elohim-cache-core/elohim_cache_core.js → 404` still fires
on **every route** of the freshly-redeployed app bundle (`chunk-EJFZ262E`, app build #1555,
commit `2a11a1191`). **Cosmetic** — the TypeScript fallback is functionally complete (per the
WASM scout + the `Jenkinsfile:680` comment); nothing is broken, it's console noise.

## What landed (and is verified) this sprint
- `data-loader.service.ts:277` now threads `initialize({ preferWasm: environment.cache?.preferWasm })`.
- Deploy uses prod/alpha config (no Angular dev-mode banner in the capture → not the base env),
  both of which set `cache.preferWasm: false`. So `createContentResolver` IS gated (no WASM load
  on that path). The companion **map fix in the same commit verified live** (`data-testid=map-error`
  resolves), proving the commit is in the deployed bundle.

## Why it still fires — every STATIC path ruled out
- `createContentResolver` (content-resolver.ts:777) — gated by the fix (`preferWasm:false`). ✓
- `isWasmResolverAvailable()`/`checkWasmAvailable()` (content-resolver.service.ts:868) — **no callers**.
- `createReachAwareCache` (reach-aware-cache.ts) — **unused** in the app.
- `createWriteBuffer`/`SeedingService` (write-buffer) — **off the alpha boot path** (only the
  seeding flow injects it; nothing constructs SeedingService at bootstrap).
- `initializeForMode` (content-resolver.service.ts:337) — **test-only** caller.
- No eager `import`, no `modulepreload`/prefetch for the wasm in the deployed index.html, no
  service-worker (`apps-sw`) precache reference.

The `look` capture records the failed request (`net::ERR_ABORTED` + 404) but **no initiator**,
so the JS trigger can't be pinned from outside.

## Next step (when someone picks this up)
1. **Runtime DevTools initiator trace** on the deployed app: open `alpha.elohim.host`, Network
   tab → the `elohim_cache_core.js` request → Initiator stack → that names the exact call site
   the static sweep missed. Gate THAT site on `preferWasm`.
2. OR the operator/CI side (the *other* half, scoped out from the start): publish the
   `elohim-wasm-cache-core` artifact to Harbor for the app's `happVersion` (Jenkinsfile:669 `oras pull`;
   options at Jenkinsfile:680) so the asset simply exists and the request 200s — strictly better
   than suppressing it. This is the durable fix for the 404 itself.

Either way: **cosmetic, low priority, do not block on it.**
