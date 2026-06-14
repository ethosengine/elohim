---
id: "backlog-ci-app-routes-count-canary-stale-debug-shell"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-app route-count canary stale after /debug shell added (15→16)"
slug: "ci-app-routes-count-canary-stale-debug-shell"
written: "2026-06-14"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: in-progress
fingerprints: [995092e07d67]
jobs: [elohim]
relatedNodeIds: []
tags: [ci, elohim-app, vitest, route-count, canary, stale-count, debug-surface, self-healing-control-plane]
cites:
  - app/elohim-app/src/app/app.routes.spec.ts
  - app/elohim-app/src/app/app.routes.ts
  - app/elohim-app/src/app/debug/debug-shell.component.ts
---

# elohim-app route-count canary stale after /debug shell added (15→16)

## The failure

`elohim` #1539 (vitest unit stage), one occurrence (seen 1, first/last build 1539):

```
AssertionError: expected 16 to be 15 // Object.is equality
```

Failing assertion: `app/elohim-app/src/app/app.routes.spec.ts:35`
`expect(routes.length).toBe(15)` — the shell's top-level route-count canary.
`routes.length` is now **16** (actual), the canary still expected 15.

Cascade impact: this is a real unit-test failure upstream of the E2E-alpha
gate, so it correctly was NOT masked by the 2b CI fix. It tripped the
orchestrator/dev #1242 Level-0 fail-fast, which skipped elohim-edge and
elohim-genesis (both target pipelines stayed red). Last green: elohim #1529.

## Verdict — real (stale-count test), bounded

Not a flake, not infra, not a regression in the route table. The 42-commit
feat→dev integration (HEAD 67545ae4b: self-healing control plane + protocol
debug surface) legitimately added a 16th top-level route. The author updated
`app.routes.ts` but did not bump the count canary or its enumerating comment.

Evidence the 16th route is intentional and fully wired:
- `ee1e41f08` "feat(elohim-app): /debug shell + lens registry + ConnectionLens
  + gated nav" added the `debug` route (`app.routes.ts:97-102`,
  `path: 'debug'` → `DebugShellComponent`); ancestor of HEAD on
  `feat/frontend-eyes-sprint`.
- `app/elohim-app/src/app/debug/` holds a complete component
  (`debug-shell.component.{ts,html,scss,spec.ts}`, `debug.types.ts`, a
  `lenses/` registry) — not a half-wired stray route.
- Independent count of `app.routes.ts`: '', community, shefa, identity,
  account, doorway, avodah, auth/callback, deliver/:slug,
  resource/:resourceId, epr/:resourceId, epr/:resourceId/raw, debug, map,
  resolve, ** = 16. The canary's own enumerating comment listed 15 and
  omitted `debug`.

Museum gate: none of the 11 CI/orchestrator traps apply — this is an
app-layer vitest count drift, not an orchestrator/sccache/sweettest/CPS
class. No new recurring trap (a forgotten count canary is narration, not
structure).

## Root cause

Count-canary maintenance gap: a new top-level route landed without bumping
the `app.routes.spec.ts` count assertion + its self-documenting comment.
The canary did its job — it caught the unaccounted-for route shape change.

## Current decision

Bounded fix LANDED, locally verified. Awaiting disappearance-confirmation
by the harvester (elohim job green-streak ≥3, no recurrence of fp
995092e07d67). Ledger stamped `decompose_on_confirm: true` — a forgotten
count canary carries no museum-worthy lesson, so the harvester fully
auto-cleans (ledger line + this backlog entry) once the streak confirms.
Re-trigger is the shift orchestrator's feat→dev push.

## Fix trail

- `0b8bed01f` test(elohim-app): bump app.routes count canary 15→16 for
  /debug shell — `app/elohim-app/src/app/app.routes.spec.ts`
  (expected count 15→16; added `debug` to the enumerating comment in
  file-order position so the canary stays self-documenting).
- Local verification: `cd app/elohim-app && pnpm exec vitest run --config
  vite.config.ts src/app/app.routes.spec.ts` → 14/14 passed (sibling
  ordering assertions — catch-all-last, raw-before-catch-all — all green).
- Committed on `feat/frontend-eyes-sprint`, not pushed (shift orchestrator
  owns the dev push / retrigger).
