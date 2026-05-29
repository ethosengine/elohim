---
name: project-orchestrator-build-tag-syntax
description: "Orchestrator's [build:X] commit-tag syntax strips the elohim- prefix. Valid aliases per the 2026-05-28 pipelines-unstable shift investigation: edge, dna, app, genesis, sophia, steward, all. CAVEAT 2026-05-28: tag parsing is gated on env.BUILD_TRIGGER == 'WEBHOOK' (Jenkinsfile L1611) — timer-triggered, manual-rebuild, and replay builds SKIP tag parsing entirely even if HEAD commit message contains the tag. If the webhook build is NOT_BUILT/aborted and a later timer build picks up the same commit, the tag is silently lost. Wiring to build-graph dispatch IS correct (L1019-1028) once parsing fires."
metadata: 
  node_type: memory
  type: project
  originSessionId: 5ed7452d-de73-43b1-814f-3b1742a3b1b8
---

The orchestrator's `[build:<name>]` commit-tag syntax (per `genesis/orchestrator/Jenkinsfile`) uses **short aliases**, not full pipeline names.

**Valid tags** (discovered from orchestrator #1075 log during 2026-05-28 shift):

| Tag | Maps to pipeline |
|---|---|
| `[build:edge]` | elohim-edge |
| `[build:dna]` | elohim-holochain |
| `[build:app]` | elohim |
| `[build:genesis]` | elohim-genesis |
| `[build:sophia]` | elohim-sophia |
| `[build:steward]` | elohim-steward |
| `[build:all]` | all pipelines |

**Invalid (silently ignored with warning):** `[build:elohim-edge]`, `[build:elohim-app]`, `[build:elohim-holochain]`, `[build:elohim-storybook]` (no storybook short alias visible), `[build:epr]` (no epr alias visible).

When the orchestrator sees an unknown tag, it logs:
```
⚠️  Unknown [build:<unrecognized>] — valid: edge, dna, app, genesis, sophia, steward, all
```
…then falls back to changeset analysis (which may or may not dispatch the intended pipeline depending on what paths changed).

**Why:** The orchestrator's PIPELINES map uses `elohim`, `elohim-edge`, `elohim-holochain`, etc. — but the build-tag parser is built around the short names (probably for typing convenience). The shift's iteration 5 lost a wake cycle waiting for edge to redispatch under `[build:elohim-edge]` which was silently ignored.

**How to apply:** When forcing a specific pipeline dispatch via empty commit, use the short alias. Two confirmation points: (1) `[build:edge]` not `[build:elohim-edge]`; (2) check the orchestrator log's "Determine Build Plan" stage for "✓ Tag [build:X] applied" or "⚠️ Unknown [build:X]" to confirm the tag landed. There's no short alias for `elohim-storybook` or `elohim-epr` — to force those, use `[build:all]` or push a path change matching their changePatterns.

Related: [[feedback_jenkins_token_strictly_guarded]] (authenticated builds via curl when tag override insufficient).

## 2026-05-28 — RESOLVED

Webhook-gate silent-drop fixed in commit `beba218b6` (Task 1 of plan
`genesis/docs/superpowers/plans/2026-05-28-orchestrator-clean-build-triggers.md`).
Tag parsing now runs on every trigger type (webhook, timer, manual, replay).
`[deploy-only]` remains webhook-gated by design.

Full legacy PIPELINES deprecation landed in the same plan (Tasks 2-10).
Pipeline metadata now lives in per-project `build-manifest.json` files;
the in-Jenkinsfile PIPELINES map and the `orchestrator-strategy.mjs`
JS mirror are deleted. `pipeline-registry.mjs` now exposes the metadata
from manifests to JavaScript consumers.
