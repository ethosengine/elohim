---
name: feedback_onpush_implicit_default_harness_blindness
title: Angular 22 implicit-OnPush freeze + harness blindness
description: Angular 22 implicit-OnPush freezes components mutating plain fields from callbacks; overrideComponent(CUT) and fixture-root autoDetect hide it — verify with an Eager host or eyes-on.
metadata:
  type: feedback
---

Angular 22 made OnPush the implicit default. A component with NO `changeDetection` that flips
template-bound plain fields from `.subscribe()` / timers / raw listeners freezes in the browser
(state correct, DOM stale — `ng.getComponent` shows the new state, `ng.applyChanges` renders it).
The 2026-07-30 Eager-removal wave + restore list missed `ContentViewerComponent` because the lamad
walk never saw the SHELL's `/epr/:id` route (cross-bundle mount) — shipped as "Loading content..."
on every EPR-card click (fixed 54dedb119, 2026-08-25).

**Why:** two harness blind spots make spec suites structurally unable to see this class:
(1) `TestBed.overrideComponent(CUT, …)` recompiles the component with JIT defaults → `ɵcmp.onPush`
flips true→false, so the spec tests an Eager copy; (2) a `ComponentFixture` whose root IS the CUT
forces `detectChanges()` on that root under autoDetect, checking an OnPush root unconditionally.

**How to apply:** a regression spec for this class must keep the production definition (override
CHILDREN, never the CUT) and mount the CUT as a child of an explicitly `Eager` host, with
`provideZoneChangeDetection()`, `autoDetectChanges(true)`, the emission inside `NgZone.run()`, and
NO `detectChanges()` after — verify it FAILS with the stamp removed. When triaging a "spinner never
clears / data loaded but nothing renders" report, check `X.ɵcmp.onPush` in the deployed bundle
before reading any data-layer code. Inventory: `genesis/data/timeline/backlog/onpush-eager-debt-inventory.md`.
Related: [[project_angular22_node24_campaign]].
