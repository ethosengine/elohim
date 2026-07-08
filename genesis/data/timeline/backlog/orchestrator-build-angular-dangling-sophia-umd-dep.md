---
id: "backlog-orchestrator-build-angular-dangling-sophia-umd-dep"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Orchestrator manifest validator: build-angular depends on nonexistent step 'elohim-sophia:build-sophia-umd' (pre-existing dangling cross-manifest dependency)"
slug: "orchestrator-build-angular-dangling-sophia-umd-dep"
written: "2026-06-29"
author: "pipeline-shakeout shift"
status: "open"
priority: "low"
ci_status: backlog
jobs: [elohim-orchestrator, elohim]
tags: [ci, orchestrator, build-manifest, validate-manifests, dangling-dependency, sophia]
cites:
  - app/elohim-app/build-manifest.json
  - genesis/orchestrator/validate-manifests.mjs
---

# build-angular declares a dependency on a step that no longer exists

## Observed (2026-06-29, while landing the pnpm-lock trigger-glob fix)

`node genesis/orchestrator/validate-manifests.mjs` reports exactly **1 error**, stable
across `dev` (reproduced with the trigger-glob edits stashed AND applied — so it is
**pre-existing**, not introduced by that change):

```
✗ ./app/elohim-app/build-manifest.json: step 'build-angular' depends on
  'elohim-sophia:build-sophia-umd' which does not exist
=== Summary: 10 manifests, 27 steps, 1 errors ===
```

`build-angular` carries a cross-pipeline `depends` on `elohim-sophia:build-sophia-umd`,
but no manifest in the validated set declares an `elohim-sophia` project with a
`build-sophia-umd` step (sophia is a submodule with its own pnpm workspace; the UMD
prebuild is enforced by the app's `prebuild`/`check-sophia.sh` script, not by an
orchestrator manifest step).

## Why it's non-blocking (so far)

The app pipeline builds fine — the sophia UMD prereq is satisfied by the app's
`prebuild` script (`scripts/check-sophia.sh` + `pnpm build:umd`), not by the
orchestrator dependency graph. The validator error is advisory; the orchestrator's
dispatch logic tolerates the dangling edge today. But a dangling `depends` is latent
risk: if dependency resolution ever becomes strict, or if someone keys ordering off
that edge, build-angular's sophia prereq silently has no graph enforcement.

## Fix options (deferred — not in scope for the trigger-glob fix)

1. **Remove the dead edge** — drop the `elohim-sophia:build-sophia-umd` entry from
   build-angular's `depends` if the prebuild script is the intended enforcement (most
   likely correct; sophia is a submodule, deliberately outside the manifest graph).
2. **Declare the missing step** — add an `elohim-sophia` manifest / `build-sophia-umd`
   step if the intent was genuine orchestrator-level ordering (heavier; only if the
   submodule should be a first-class pipeline node).

Decide which intent is correct before touching it — this is an orchestrator
dependency-graph decision, not a mechanical edit. Captured here so the dangling edge
is owned, not lost.
