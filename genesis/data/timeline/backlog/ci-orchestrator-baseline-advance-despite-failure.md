---
id: "backlog-ci-orchestrator-baseline-advance-despite-failure"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Orchestrator baselines advance at plan time even when the dispatched build FAILS — failed work goes invisible to every later run (under-build hole)"
slug: "ci-orchestrator-baseline-advance-despite-failure"
written: "2026-06-11"
author: "agentic-developer (EPR durability arc, pipeline shakeout)"
status: "backlog"
priority: "high"
ci_status: red
jobs: [elohim-orchestrator]
tags: [ci, orchestrator, baseline, graph-walker, dispatch, anti-pattern]
cites:
  - genesis/orchestrator/Jenkinsfile
  - genesis/orchestrator/graph-walker.mjs
---

# Baseline advance despite failure — the under-build hole

## Evidence (2026-06-11 overnight, quoted from build logs)

- Orchestrator **#1213** (294-file push, ends `bb9b3776`): graph-walker
  correctly planned `[elohim + …] → elohim-edge → genesis` (edge BUILD from
  `elohim/elohim-storage/src/http.rs`, genesis BUILD). elohim-app **#1528
  FAILED** at Level 0 → edge and genesis never dispatched.
- Orchestrator **#1215** (next push, 2 files): log shows
  `Using stored global baseline: bb9b3776` and
  `Loaded per-pipeline baselines from previous successful build`, then a
  decision matrix with **every app/edge step `SKIP no changes`** and
  `[baseline:plan] archived — __global__=c56ee4b1, per-pipeline=4`.

The 294-file changeset's app/edge work became **invisible**: the baseline
advanced to `bb9b3776` even though the only build that carried those
changes FAILED (app) or never ran (edge, genesis Level-2 steps). The
sibling anti-pattern in the museum is baseline-ROLLBACK over-build; this is
baseline-ADVANCE under-build — strictly worse, because nothing ever
rebuilds the lost work without manual intervention.

## Mechanism (to confirm in the fix)

`[baseline:plan] archived` strings suggest per-pipeline baselines are
archived at PLAN time (optimistic), not at dispatch-result time. Optimistic
advance is the documented intent for fire-and-forget longRunning pipelines;
applying it to wait-for-result pipelines that then FAIL (or to planned
pipelines never reached because of a cascade abort) silently drops work.

## Workaround (used 2026-06-11)

Force tags in the HEAD commit: `[build:app,edge]` (comma syntax per
`genesis/orchestrator/commit-tag-parser.mjs`).

## Fix shape

On plan archive, only advance a pipeline's baseline when its dispatched
build completes SUCCESS/UNSTABLE; pipelines that fail, are aborted, or are
never reached keep their old baseline so the next run re-plans them.
Museum-worthy once fixed.

shift_objective: |
  Close the under-build hole: per-pipeline baselines must not advance for
  pipelines whose builds failed or never ran. Prove it with an orchestrator
  run where a Level-0 failure leaves edge/genesis baselines untouched and
  the NEXT run re-plans them without [build:*] tags.

## 2026-09-19 — half closed, and the other half now bites the other way

**Closed (34f652bdb, verified live):** the plan checkpoint advanced `__global__` to the run's own commit and the
failed-post branch "preserved prior" from that already-overwritten value, so orchestrator/dev 1875 logged "baseline
held so the next push re-dispatches" while archiving its own head. Auto mode now records the baseline it loaded and a
failed post restores it. Proof on fresh triggers: 1878 and 1880 both print `[baseline:plan] archived — __global__=<own
head>` then `[baseline:post] … preserving prior 6ab3c790` / `archived — __global__=6ab3c790`.
`genesis/orchestrator/baseline-hold.test.mjs` guards both halves.

**Still open — per-pipeline.** Routing reads only `__global__` (`autoModeAnalyze` → `analyzeChangeset`), and every
archive since 1875 says `per-pipeline=0`. So in orchestrator/dev 1880 — `✅ elohim-edge: SUCCESS (7472s)`,
`❌ elohim: FAILURE` — the held global baseline means the NEXT push re-dispatches edge too: a full rebuild and a
whole-fleet re-roll to retry an app stage. That is worse than waste here: the re-roll re-saturates conductor admission
for hours (backlog `conductor-admission-saturated-for-hours-after-restart`), which is what failed the app stage — a
limit cycle. Until this is fixed, **any push to dev, docs included, re-rolls alpha** unless its head commit says
skip-ci or the run is started manually for app + genesis only.

Two pieces are needed, and the first already exists uncommitted:

1. Persist what succeeded before the fail-fast throw. The index of the shared checkout has held staged work since
   2026-09-14 that does exactly this in Execute Builds (`env.PIPELINE_BASELINES = …pipelineBaselines` then
   `archivePipelineBaselines('execute')` before throwing, plus the stale-controller-workspace backstop and
   `orchestrator-integration.test.mjs` +67). It passes `just gate orchestrator` in the tree. It needs its owner's review
   and a commit; this shift committed around it and left it staged as found.
2. Consult it when routing: a pipeline whose recorded success sha already contains every changed file that routes to
   it (graph-walker over `<its sha>..HEAD`) is dropped from the dispatch set.
