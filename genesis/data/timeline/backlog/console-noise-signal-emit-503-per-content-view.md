---
id: "backlog-console-noise-signal-emit-503-per-content-view"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Every content view POSTs /api/v1/signal/emit and takes a by-design 503 while write-through is OFF — one browser console error per page that the a2o console-cleanliness hook fails every content scenario on"
slug: "console-noise-signal-emit-503-per-content-view"
written: "2026-08-25"
author: "epr-card-nav shift (integrator)"
status: "backlog"
priority: "medium"
tags: [console-noise, signal-harness, write-through, rea-runtime, a2o-console-cleanliness, content-viewer, lamad, capability-discovery]
cites:
  - app/elohim-library/projects/elohim-rea-runtime/src/lib/signal-emit.service.ts
  - app/lamad/src/app/services/signal-harness.service.ts
  - genesis/a2o/src/framework/utils/console-filters.ts
  - genesis/a2o/steps/common.steps.ts
  - genesis/a2o/features/elohim-core/epr-link-hypercard.feature
---

## What

`SignalHarnessService.onRendererComplete` (fired on every content view) calls
`SignalEmitService.tryEmit`, whose contract is: HTTP 503 ⇒ `{status:'fallback'}` ⇒ POST the legacy
`/api/v1/economic-events`. On alpha write-through is OFF for shefa/EconomicEvent, so every
`/epr/{id}` and `/resource/{id}` view logs `Failed to load resource: … 503` for `/api/v1/signal/emit`
— handled by contract, but the browser reports it, and `collectDeviceErrors` (a2o After hook) fails
any otherwise-passing content scenario on it (`isSpaRoutingNoise` whitelists 404/403/0, not 503).
Observed 2026-08-25 on the new `Following the card …` regression scenario: 7/7 steps pass on the
deployed alpha, the scenario is failed by this single console line (flaky by timing — a run that
finishes before the harness POST lands passes).

## Fix shape (client-side; no wire change)

Discover write-through state ONCE instead of probing with a failing POST per view: e.g. read the
doorway capability surface (`/admin/capabilities` already exists) or cache the first `fallback`
result per session in `SignalEmitService` and short-circuit to the legacy path until a capability
change is observed. Either removes the console error at the source. Do NOT whitelist 503 in
`console-filters.ts` — a blanket 503 would also hide real catching-up sheds from the cleanliness
contract; the by-design 503 has to stop being emitted, not stop being seen.
