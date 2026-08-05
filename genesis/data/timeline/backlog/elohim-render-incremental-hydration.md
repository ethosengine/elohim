---
id: "backlog-elohim-render-incremental-hydration"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-render: support Angular incremental hydration + event replay (opted out at v22 migration)"
slug: "elohim-render-incremental-hydration"
written: "2026-07-30"
author: "angular22-campaign"
status: "backlog"
priority: "low"
tags: [elohim-render, ssr, hydration, angular, low-power-devices, performance]
cites:
  - app/elohim-app/src/app/app.config.server.ts
  - app/lamad/src/app/app.config.server.ts
  - genesis/docs/content/elohim-protocol/architecture/2026-06-02-doorway-ssr-runtime.md
---

## What

Angular 22 ships incremental hydration (`withIncrementalHydration()` + `@defer (hydrate on …)`)
— per-fragment hydration with event replay, so a server-rendered page becomes interactive
piecewise instead of hydrating the whole component tree up front. The v22 migration explicitly
opted BOTH SSR apps out (`withNoIncrementalHydration()` in each `app.config.server.ts`),
because our SSR is not Angular CLI SSR: it is the custom elohim-render deno_core runtime
driving `renderApplication()` and splicing output into the browser shell.

## Why it fits the protocol (when it's ready)

The seam map's device spectrum runs smartwatch → home storage rack. Full-tree hydration cost
scales with page complexity and is paid on the *client* — exactly the resource human-scale
hardware doesn't have. Incremental hydration moves that cost to on-demand per-fragment work,
which is the right shape for low-power household devices consuming doorway-projected SSR
surfaces. This is a UI-render gradient concern (NOT hardware tiering — see the seam map's
misrouting warning).

## Why it is a feature request, not a config flip

Incremental hydration requires the serving layer to preserve Angular's hydration annotations
and serve the client bundles such that deferred fragments can load + replay events. The
elohim-render compose step (root-tag-derived splice into the browser shell) and the doorway
blob-serving path have never been validated against `@defer`-block hydration boundaries.
Turning the flag on without renderer support would at best no-op and at worst break
hydration entirely (NG05xx class errors client-side).

## Acceptance sketch

1. elohim-render compose preserves hydration annotation comments/attributes through the splice.
2. A `@defer (hydrate on viewport)` block in a test route hydrates lazily against a
   doorway-served bundle (a2o scenario under `genesis/a2o/features/ssr/`).
3. Event replay verified: click during pre-hydration window is replayed post-hydration.
4. Flip `withNoIncrementalHydration()` → `withIncrementalHydration()` per app, measure
   hydration cost delta on a low-power fixture device.

## Current state

Opted out at both SSR sites (v22 migration, 2026-07-30, commit a7da08924). No renderer work
started. Re-evaluate when elohim-render next takes SSR feature work.
