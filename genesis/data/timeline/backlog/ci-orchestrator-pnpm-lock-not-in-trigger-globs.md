---
id: "backlog-ci-orchestrator-pnpm-lock-not-in-trigger-globs"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Orchestrator under-dispatches on a root pnpm-lock.yaml change — edge/app/storybook trigger globs omit the lockfile (a frozen-lockfile fix can't re-trigger the jobs it fixes)"
slug: "ci-orchestrator-pnpm-lock-not-in-trigger-globs"
written: "2026-06-28"
author: "pipeline-shakeout shift"
status: "wip"
priority: "medium"
ci_status: in-progress
jobs: [elohim-orchestrator, elohim-edge, elohim, elohim-storybook]
tags: [ci, orchestrator, change-detection, build-manifest, pnpm-lock, frozen-lockfile, principle-7, under-dispatch]
cites:
  - app/elohim-app/build-manifest.json
  - elohim/holochain/build-manifest.json
  - app/elohim-library/build-manifest.json
  - genesis/orchestrator/graph-walker.mjs
---

# Orchestrator under-dispatches on a root `pnpm-lock.yaml`-only change

## The failure (observed 2026-06-28, orchestrator/dev #1338)

A push whose changeset was a single root `pnpm-lock.yaml` line (commit `0cd103b06`,
the fix for `ERR_PNPM_OUTDATED_LOCKFILE`) triggered orchestrator #1338, which dispatched
**only `elohim-genesis`** (`description: auto: elohim-genesis`). It did NOT dispatch
`elohim-edge`, `elohim` (app), or `elohim-storybook` — the exact pnpm-consuming pipelines
that were hard-FAILING at `pnpm install --frozen-lockfile` and that the lockfile fix was
authored to unblock. Result: the fix landed on `dev` but the failing jobs never re-ran to
pick it up (edge stayed at #1128 FAILURE, app at #1569, storybook at #187).

## Root cause

Build-manifest trigger globs live under `steps.*.inputs.sources`. None of the three
pnpm-consumer manifests list the **root `pnpm-lock.yaml`** (or root `package.json`) as a
source:
- `app/elohim-library/build-manifest.json` lists `app/elohim-library/package.json` but not root `pnpm-lock.yaml`.
- `app/elohim-app/build-manifest.json` and `elohim/holochain/build-manifest.json` (edge) likewise scope to their own subtrees.

So a workspace-root lockfile change matches none of their globs → graph-walker doesn't
select them. Yet all three run `pnpm install --frozen-lockfile` against that root lockfile,
so a lockfile change is exactly a change they depend on. This is a change-detection
**under-build** (principle 7): the dependency graph omits the workspace lockfile edge.

## Fix

Add `pnpm-lock.yaml` (and root `package.json`) to the `inputs.sources` of the
install/build step in each pnpm-consumer manifest:
- `app/elohim-library/build-manifest.json` → `build-storybook.inputs.sources` (pipeline `elohim-storybook`).
- `app/elohim-app/build-manifest.json` → `build-angular.inputs.sources` (pipeline `elohim`).
- `doorway/doorway-app/build-manifest.json` → `build-doorway-app.inputs.sources` — doorway-app is its **own** pipeline (`elohim-doorway-app`), NOT the edge manifest. (2026-07-06 pipeline-shakeout re-grounding corrected the original third bullet, which wrongly named `elohim/holochain`.)
- **Edge caveat (separate concern, not a glob edit):** the edge manifest (`elohim/holochain`) has no doorway-app step — its doorway-app `pnpm install --frozen-lockfile` runs *embedded* in `elohim/holochain/Jenkinsfile:1320-1362`, unmodeled as a manifest step, so a root-lockfile change can't change-detect it via graph-walker. Do NOT bolt `pnpm-lock.yaml` onto a Rust step; model it as a manifest step or let `elohim-doorway-app` own the doorway-app build — a separate decision.

Editing each manifest also self-triggers its pipeline (each manifest lists its own
`build-manifest.json` in sources), so the fix landing re-validates each.

## Interim mitigation (this shift)

Re-dispatched edge+app via a `[build:edge,app]` commit tag (`f5815c703`); the documented
force-tags have no `storybook` variant, so storybook was left to recover on its next
trigger / this fix. The durable fix above removes the need for the manual tag next time.

## Status

`ci_status: in-progress` (updated 2026-06-29) — the durable manifest-glob fix **LANDED for
app + storybook** this commit:
- `app/elohim-app/build-manifest.json` `build-angular.inputs.sources` += `pnpm-lock.yaml`,
  `package.json`, **and `app/elohim-app/build-manifest.json`** — the app manifest did NOT
  list its own manifest in sources, so it was not genuinely self-triggering (the app
  pipeline fired here only because the co-edited library manifest matches build-angular's
  `app/elohim-library/**` glob). That self-trigger gap is now closed too, matching
  storybook's already-correct self-reference.
- `app/elohim-library/build-manifest.json` `build-storybook.inputs.sources` +=
  `pnpm-lock.yaml`, `package.json`.

**doorway-app glob LANDED (2026-07-07 pipeline-shakeout).** The "edge" residual named in the
2026-06-29 update was a mis-attribution: the third pnpm consumer is the doorway admin app, which
has its **own** pipeline + manifest (`elohim-doorway-app` / `doorway/doorway-app/build-manifest.json`),
NOT the edge manifest. That manifest's `build-doorway-app.inputs.sources` now carries
`pnpm-lock.yaml` + `package.json` (this commit), so all three *modeled* pnpm consumers
(app, storybook, doorway-app) change-detect on a root-lockfile change.

**One residual remains, re-scoped:** edge's OWN doorway-app build is *embedded* in
`elohim/holochain/Jenkinsfile:1320-1362` (not a manifest step), so graph-walker cannot
change-detect it from a root-lockfile-only change. That is a separate modeling decision (model it
as a manifest step, or let `elohim-doorway-app` own the doorway-app build), tracked here but not a
glob edit. This item stays `wip` until that embedded build is modeled.

Confirms by disappearance: a future root-`pnpm-lock.yaml`-only change should now dispatch
app+storybook+doorway-app without a force-tag (edge's embedded build remains the known gap).
