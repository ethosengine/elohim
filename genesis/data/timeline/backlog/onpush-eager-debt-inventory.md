---
id: "backlog-onpush-eager-debt-inventory"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "OnPush-Eager debt inventory — surviving ChangeDetectionStrategy.Eager stamps post-v22-migration"
slug: "onpush-eager-debt-inventory"
written: "2026-07-30"
author: "angular22-campaign"
status: "backlog"
priority: "low"
tags: [angular, change-detection, onpush, cleanup, v22-migration]
cites:
  - app/elohim-app/src/app/components/home/home.component.ts
  - app/lamad/src/app/components/search/search.component.ts
  - app/lamad/src/app/renderers/markdown-renderer/markdown-renderer.component.ts
  - app/lamad/src/app/components/content-viewer/content-viewer.component.ts
---

## What

The Angular 22 migration stamped `changeDetection: ChangeDetectionStrategy.Eager` on 145
components across five apps to preserve pre-v22 change-detection semantics (v22 made OnPush
the implicit default for components with no explicit strategy). Each stamp is legacy CD debt:
it opts the component back into whole-tree dirty-checking instead of the OnPush signal/input
push model v22 wants by default.

## The Eager-removal wave (2026-07-30)

Ran the full protocol per app — remove the stamp on every Eager-stamped component in the app
at once, run the app's full vitest suite at its pre-existing baseline count, run
`ng build` (AOT) as a second gate, restore (`git checkout --`) any component whose removal
broke a spec or the build. Order: imagodei-portal → doorway-app → elohim-library → lamad →
elohim-app (small to large).

**Result: 145/145 removed, 0/145 restored.** Every app held its exact test baseline and built
clean:

| App | Removed | Restored | Test baseline held |
|---|---|---|---|
| imagodei-portal | 1 | 0 | 29/29 |
| doorway-app | 14 | 0 | 32/32 |
| elohim-library | 5 | 0 | 261/261 |
| lamad | 22 | 0 | 2800/2800 |
| elohim-app | 103 | 0 | 4596/4596 |

Zero survivors means this backlog entry currently has no live debt to enumerate — it exists
as the inventory shape for the NEXT time a migration (or a new component) stamps Eager, and
as the record of one component investigated for a production-bug false alarm during the wave.

## Investigated, not restored: `app/elohim-app/src/app/components/home/home.component.ts`

The full-suite run surfaced one Vitest "Unhandled Error" (not a test failure — all 4596 tests
still passed) from `home.component.spec.ts`: `TypeError: this.intersectionObserver?.observe is
not a function`, thrown from a `setTimeout(..., 0)` callback in `setupIntersectionObserver()`
that fires after the test's `TestBed` fixture has torn down.

Per the wave's production-semantics rule this looked like exactly the class of thing to flag —
an OnPush timing change exposing a real stale-UI risk. It was investigated by isolating the
file: temporarily re-adding `ChangeDetectionStrategy.Eager` to this one component (Edit, not
git-stash — shared worktree) and re-running `home.component.spec.ts` alone. **The exception
reproduces identically under Eager.** It's a pre-existing jsdom/zone.js `IntersectionObserver`
mock-teardown flake in the spec's async timer handling, independent of change-detection
strategy. Component was left in the removed (OnPush) state. The flake itself is a separate,
pre-existing test-infra debt item (untracked here — it predates this wave and isn't
Eager-stamp-shaped).

## Why three components in lamad carry `NG0100`-under-Eager comments but were still removed

`search.component.spec.ts`, `markdown-renderer.component.spec.ts`, and
`content-viewer.component.spec.ts` each carry inline comments noting that mutating state
between `detectChanges()` calls "trips NG0100 under ChangeDetectionStrategy.Eager" — a signal
these specs are change-detection-timing-sensitive. `search.component.ts` and
`markdown-renderer.component.ts` *were* in the 22-file Eager-stamped set and were removed
cleanly (their specs stayed green at OnPush — NG0100 is an Eager-only failure mode, not an
OnPush one, so removing Eager can only help here). `content-viewer.component.ts` was never
Eager-stamped to begin with (already implicit-OnPush pre-wave, consistent with the
affinity-circle precedent) — its comment is inherited context from a shared test pattern, not
evidence of a stamp this wave touched.

## Recheck trigger

If a future migration re-introduces Eager stamps (framework major bump, a new component
scaffolded from an old template, a merge from a stale branch), rerun this wave's protocol and
update the table above. A non-zero "Restored" count is where this doc starts carrying live
debt — each restored file should be listed here with its one-line failure signature so a
future OnPush refactor pass has a starting list instead of re-deriving it from a repo-wide
grep.
