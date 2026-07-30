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
  - app/elohim-app/src/app/qahal/components/opinion-cluster/opinion-cluster.component.ts
  - app/lamad/src/app/components/learner-dashboard/learner-dashboard.component.ts
  - app/lamad/src/app/components/profile-page/profile-page.component.ts
  - app/lamad/src/app/components/content-editor-page/content-editor-page.component.ts
  - app/elohim-app/src/app/components/theme-toggle/theme-toggle.component.ts
  - doorway/doorway-app/src/app/components/theme-toggle/theme-toggle.component.ts
  - app/elohim-app/src/app/shared/components/alert-banner/alert-banner.component.ts
  - app/elohim-app/src/app/elohim/components/elohim-navigator/elohim-navigator.component.ts
  - app/elohim-app/src/app/imagodei/components/profile/sections/profile-header/profile-header.component.ts
  - app/elohim-library/projects/lamad-ui/src/lib/components/observer-diagram/observer-diagram.component.ts
  - app/elohim-library/projects/lamad-ui/src/lib/components/value-scanner-diagram/value-scanner-diagram.component.ts
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

Zero survivors at wave time meant this backlog entry initially had no live debt to enumerate.
That changed at the post-wave code review (below) — this doc now carries the live restore list.

## Post-review restoration (2026-07-30): 12 of 145 restored

Code review of the removal wave found the wave's own verification method — full-suite
`fixture.detectChanges()` runs — is structurally blind to a specific staleness class: a
component that mutates template-bound state from a subscribe callback, a `setInterval`/
`setTimeout` handler, or a raw DOM event listener OUTSIDE Angular's zone-triggered change
detection cycle. Under OnPush, that mutation never schedules a re-render; under a spec that
calls `fixture.detectChanges()` explicitly (forcing a check regardless of strategy), the same
mutation reads as "passing" because the test itself drives the check the runtime would skip.
Zero test regressions across 4596+2800+261+32+29 runs proved nothing about this class.

The review traced 12 components with this exact shape and reclassified them: **restore the
Eager stamp, do not attempt an OnPush conversion in this pass** (that's a real refactor —
signal-izing the mutating state or funneling it through `markForCheck()` — deliberately
deferred to a future OnPush-conversion campaign, not bundled into this remediation).

**Updated net count: 145 removed at wave time → 12 restored post-review = 133 net removed.**

| App | Removed (wave) | Restored (review) | Net removed |
|---|---|---|---|
| imagodei-portal | 1 | 0 | 1 |
| doorway-app | 14 | 1 | 13 |
| elohim-library | 5 | 2 | 3 |
| lamad | 22 | 4 | 18 |
| elohim-app | 103 | 5 | 98 |
| **Total** | **145** | **12** | **133** |

### The 12 restored components and their OnPush-unsafe reason

| Component | Failure mode |
|---|---|
| `app/elohim-app/src/app/qahal/components/opinion-cluster/opinion-cluster.component.ts` | Canvas event listeners + subscribe mutation outside change detection |
| `app/lamad/src/app/components/learner-dashboard/learner-dashboard.component.ts` | `ngOnInit` subscribe mutation |
| `app/lamad/src/app/components/profile-page/profile-page.component.ts` | `ngOnInit` subscribe mutation |
| `app/lamad/src/app/components/content-editor-page/content-editor-page.component.ts` | `ngOnInit` subscribe mutation |
| `app/lamad/src/app/renderers/markdown-renderer/markdown-renderer.component.ts` | Fire-and-forget async `ngOnChanges` + scroll listener |
| `app/elohim-app/src/app/components/theme-toggle/theme-toggle.component.ts` | Cross-tab theme sync subscribe mutation |
| `doorway/doorway-app/src/app/components/theme-toggle/theme-toggle.component.ts` | Cross-tab theme sync subscribe mutation |
| `app/elohim-app/src/app/shared/components/alert-banner/alert-banner.component.ts` | `setTimeout` auto-dismiss mutation |
| `app/elohim-app/src/app/elohim/components/elohim-navigator/elohim-navigator.component.ts` | Raw subscribes masked by router co-trigger (router navigation already forces a check, hiding the gap in specs and in most real nav-heavy sessions) |
| `app/elohim-app/src/app/imagodei/components/profile/sections/profile-header/profile-header.component.ts` | Clipboard write + `setTimeout` flag mutation |
| `app/elohim-library/projects/lamad-ui/src/lib/components/observer-diagram/observer-diagram.component.ts` | `setInterval` field mutation |
| `app/elohim-library/projects/lamad-ui/src/lib/components/value-scanner-diagram/value-scanner-diagram.component.ts` | `setInterval` field mutation |

Each restored file carries a one-line `// OnPush-unsafe: <reason> — see backlog-onpush-eager-debt-inventory`
comment directly above its `changeDetection: ChangeDetectionStrategy.Eager` line, so a future
reader hits the reasoning at the point of use, not just in this doc.

### Structural lesson

`fixture.detectChanges()` in a spec is not a proxy for "OnPush would have re-rendered here" —
it's an unconditional check that erases the exact distinction OnPush exists to enforce. A
removal wave (or any future CD-strategy change) that verifies itself only through
`detectChanges()`-driven suites will systematically miss subscribe/callback/timer mutation
staleness. The real gate for that class is one of: (a) eyes-on-render verification (`pnpm look`
against a live route, watching for state that visibly fails to update after an out-of-zone
event), or (b) converting the component to signals / `markForCheck()` discipline so there's no
window where OnPush can silently skip a needed check. The 12 restored components above are
candidates for (b) in a future OnPush-conversion campaign; until then they stay on Eager.

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
cleanly at wave time (their specs stayed green at OnPush — NG0100 is an Eager-only failure
mode, not an OnPush one, so removing Eager could only help there). `markdown-renderer` was
subsequently one of the 12 post-review restorations (see above) for an unrelated reason —
fire-and-forget async `ngOnChanges` + scroll-listener mutation, not NG0100 — so it carries
Eager again; `search.component.ts` was not flagged by review and remains OnPush.
`content-viewer.component.ts` was never Eager-stamped to begin with (already implicit-OnPush
pre-wave, consistent with the affinity-circle precedent) — its comment is inherited context
from a shared test pattern, not evidence of a stamp this wave touched.

## Recheck trigger

If a future migration re-introduces Eager stamps (framework major bump, a new component
scaffolded from an old template, a merge from a stale branch), rerun this wave's protocol and
update the table above. A non-zero "Restored" count is where this doc starts carrying live
debt — each restored file should be listed here with its one-line failure signature so a
future OnPush refactor pass has a starting list instead of re-deriving it from a repo-wide
grep.
