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
install/build step in each of the three manifests:
- `app/elohim-library/build-manifest.json` → `build-storybook.inputs.sources`
- `app/elohim-app/build-manifest.json` → the install/build step's sources
- `elohim/holochain/build-manifest.json` → the "Build Doorway App" / install step's sources

Editing each manifest also self-triggers its pipeline (each manifest lists its own
`build-manifest.json` in sources), so the fix landing re-validates all three.

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

**Edge (`elohim/holochain/build-manifest.json`) deliberately NOT edited in this commit** —
editing it would re-dispatch edge, which redeploys/churns alpha and would race the app
pipeline's Upload SPA Blob leg (the leg recovering the stale elohim.host mount onto
now-healthy alpha). Edge's lockfile glob is the one remaining residual; add it once
elohim.host is confirmed recovered (low urgency — edge is normally triggered by a storage
changeset; the root-lockfile-only case is the gap). **This item stays `wip` until edge's
glob lands.**

Confirms by disappearance: a future root-`pnpm-lock.yaml`-only change should dispatch
app+storybook now (and, once the edge glob lands, edge) without a force-tag.
