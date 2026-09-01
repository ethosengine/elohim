---
id: "backlog-ci-genesis-epr-lens-composite-outline-not-deployed"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "EPR Slice-1 lens-complete /epr/{claimed} E2E red on alpha — PathViewerComponent (composite-outline keystone) absent from the deployed app bundle"
slug: "ci-genesis-epr-lens-composite-outline-not-deployed"
written: "2026-06-08"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: blocked
fingerprints: [a33fea18bb6b]
jobs: [elohim-genesis]
relatedNodeIds: []
tags: [ci, genesis, e2e, lamad, epr, lens-complete, deploy-coherence, host-green-ne-ci-green]
cites:
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1108/
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1107/
  - genesis/a2o/features/lms/deep-link-delivery.feature
  - genesis/a2o/steps/lamad/deep-link-delivery.steps.ts
  - genesis/a2o/src/framework/pages/selectors.ts
  - app/lamad/src/app/renderers/path-viewer/path-viewer.component.html
  - app/lamad/src/app/renderers/path-viewer/path-viewer.component.ts
  - app/lamad/src/app/renderers/renderer-initializer.service.ts
  - app/lamad/src/app/components/content-viewer/content-viewer.component.ts
  - app/lamad/src/app/generated/manifest-types.ts
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# EPR Slice-1 lens-complete render — composite outline not deployed to alpha (elohim-genesis)

The newly-inverted EPR Slice-1 scenario asserts the lens-complete render at the
universal address. The SOURCE is correct and unit-green; the bundle deployed to
`alpha.elohim.host` is missing the keystone renderer. This is a deploy-coherence
gap, not a code bug — the genesis pipeline TESTS live alpha, it does not build or
deploy the app.

## The failure

`a33fea18bb6b` — Scenario "Universal EPR address renders a claimed type
lens-complete, not a 302" (`features/lms/deep-link-delivery.feature:54`,
`@browser-only @regression`), step "Then the lamad composite outline renders"
(`steps/lamad/deep-link-delivery.steps.ts:327`):

```
✔ Given a learner opens the deep link "/epr/foundations-christian-technology"
✖ Then the lamad composite outline renders
AssertionError [ERR_ASSERTION]: Expected the focal epr-composite outline to render
  (data-testid="epr-composite-outline"); URL is
  "https://alpha.elohim.host/epr/foundations-christian-technology"
- false
+ true
- And the Open in pillar lens is offered   (cascade-skipped after the failed assert)
```

**Occurrence evidence:** ledger `seen: 2`, `first_build: 1107`, `last_build: 1108`.
This is a NEWLY-INVERTED scenario — commit `ee51d429c` (2026-06-08 03:03 UTC, EPR
Slice 1 task 4) flipped it from asserting the Slice-3 claims-302 to asserting the
lens-complete render, so 1107 is the first build the inverted form ran. The
onset is the scenario inversion itself, not a behavior regression of previously-
green code.

## Verdict

**Real deploy-coherence failure — not a flake, not a code bug, not a museum
measure-trap.** The URL stays at `/epr/foundations-christian-technology` (HTTP
200, no redirect — confirmed live), so the doorway-side demotion of the claims-302
(task 1) IS deployed and working; the failure is purely that the focal lens does
not render. Deterministic across 1107→1108. Not flagged flaky.

This maps to the museum's host-green ≠ CI-green cluster (the load-bearing reading,
`…/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md` §"The load-bearing
reading", and trap **#3 build-manifest/context completeness** — a NEW source input
the deployed artifact under-covers). It CITES the museum; it does not re-derive a
new trap. (Adjacent lineage: the EprRouter-empties concern
`ci-genesis-lamad-shell-routing-regression.md` was a doorway-data poison; THIS is
an app-bundle deploy gap — a sibling surface, kept as a separate concern.)

## Root cause (VERIFIED by direct probe of the deployed alpha bundle)

The source tree is coherent and correct:
- `app/lamad/src/app/renderers/path-viewer/path-viewer.component.html:1` carries
  `data-testid="epr-composite-outline"` (template correct).
- `renderer-initializer.service.ts` statically imports `PathViewerComponent`
  (line 9) and registers it in `RENDERER_COMPONENTS` (line 21) — the TODO was
  uncommented in `4e9388c44`.
- `manifest-types.ts:88` maps `'epr-composite': 'PathViewerComponent'`.
- `ContentViewerComponent` (the shell's `/epr/:resourceId` `loadComponent`)
  instantiates the registered renderer via
  `rendererRegistry.getRenderer(node)` → `rendererHost.createComponent(...)`
  (content-viewer.component.ts:387,397). For a claimed `epr-composite` (a path)
  this resolves to `PathViewerComponent`, rendering the outline.

The DEPLOYED `alpha.elohim.host` bundle, by contrast, is INCOHERENT. Probing every
served JS chunk (shell `main` + 70 lazy chunks) shows:

| Marker | Source commit | Deployed on alpha? |
|---|---|---|
| `epr-raw-node` (Slice 0) | `988a1b0c1` | YES |
| `/epr/{claimed}` → 200 no-redirect (Slice 1 **task 1**, 302 demotion) | `81994237d` | YES |
| `epr-open-in-pillar` / `open-in-pillar-link` (Slice 1 **task 3**) | `52eba1323` | YES (chunk-3YA2DW4S) |
| `epr-composite-outline` / `path-outline` (Slice 1 **task 2**, KEYSTONE) | `4e9388c44` | **NO — 0 chunks** |

The four OTHER registry renderers (`markdown-renderer`, `gherkin-renderer`,
`sophia-renderer`, `iframe-renderer`) ARE deployed, instantiated through the
IDENTICAL `getRenderer → createComponent` path — so the registry mechanism is NOT
tree-shaken. PathViewerComponent is simply absent.

The contradiction that pins the diagnosis: task 3 (`52eba1323`, the LATER commit,
03:01 UTC) is deployed while task 2 (`4e9388c44`, the EARLIER ancestor, 02:51 UTC)
is not. A clean linear build from any commit ≥ task 3 would necessarily contain
task 2. The only way to deploy task 3 without task 2 is a **stale/incremental app
build that picked up the modified `content-viewer.component.html` (task 3) but
never emitted the NEW `renderers/path-viewer/` directory (task 2)** — the same
shape as museum trap #3 (a new source input the build context under-covers), on
the app/lamad bundle deploy. The genesis pipeline only runs cucumber against
already-deployed alpha (checkout genesis → seed humans → cucumber vs
`E2E_DOORWAY_ALPHA`; no app/edge build stage), so it cannot self-heal this.

## Current decision

`ci_status: blocked` — **operator/app-pipeline move, not a code fix.** The source
is correct and unit-green; nothing in the tree is wrong, so there is no bounded
code change to land. What unblocks it: a **clean (cache-defeating) app rebuild +
redeploy to alpha** from a commit ≥ `4e9388c44`, so the deployed bundle contains
the `renderers/path-viewer/` chunk. Because the symptom is a stale/partial build
artifact (task-3-without-task-2), an incremental rebuild may reproduce the gap —
the redeploy must force a clean app build (defeat the bundler/layer cache for the
lamad bundle). The app pipeline is the root `Jenkinsfile`; triggering it is an
integrator push (`[build:app]` or `[build:edge]` ride a push) — this agent is
commit-only and cannot trigger builds.

Recurrence reference: `last_build` at triage = 1108. The sweep confirms by
disappearance (elohim-genesis green-streak ≥3 with no recurrence of
`a33fea18bb6b`) once the coherent bundle is deployed. No `decompose_on_confirm`
stamp: the lesson (a partial/stale app-bundle deploy can ship a later commit's
template change while dropping an earlier commit's whole new renderer directory,
and a live-alpha E2E will read it as a render regression) is museum-worthy if it
recurs — graduate-then-decompose on confirmation rather than silent delete.

## Fix trail

No code commit (source is coherent). Verification performed this run:
- Live probe `GET https://alpha.elohim.host/epr/foundations-christian-technology`
  → HTTP 200, no redirect, shell served (`<base href="/">`, title `elohim.host`).
- Fetched the served shell `main` + all 70 lazy chunks; grepped string-literal
  markers (survive minification). `epr-composite-outline`/`path-outline`: 0
  chunks. `epr-open-in-pillar`: 1 chunk. Four sibling renderers: present.
- Confirmed the genesis-tested commit (`c762aae4c`, build 1108) is a descendant
  of `4e9388c44` — the source under test HAD PathViewerComponent; the gap is the
  deployed artifact.
