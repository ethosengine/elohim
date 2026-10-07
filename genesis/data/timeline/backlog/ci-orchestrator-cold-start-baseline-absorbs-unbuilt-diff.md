---
id: "backlog-ci-orchestrator-cold-start-baseline-absorbs-unbuilt-diff"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "orchestrator: a cold-start run that then FAILS absorbs the unbuilt diff — the baseline-hold restores the LOADED baseline, and on a cold start there was none to restore"
slug: "ci-orchestrator-cold-start-baseline-absorbs-unbuilt-diff"
written: "2026-10-07"
author: "release-retention-cleanup shift (Claude Fable 5.1)"
status: "backlog"
priority: "medium"
ci_status: open
fingerprints: []
jobs: [elohim-orchestrator]
relatedNodeIds: []
tags: [ci, orchestrator, baseline, cold-start, under-build, change-detection, principle-7]
cites:
  - https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/1989/
  - https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/1990/
  - https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/1991/
  - https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/1992/
  - genesis/orchestrator/Jenkinsfile
  - genesis/orchestrator/baseline-hold.test.mjs
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# A cold start that fails absorbs the diff it never built

## Seen 2026-10-07 (alpha CI, during a GitHub egress stall)

1. `#1989` (a0601e49d: storage, scripts/ci, holochain Jenkinsfile, sdk/schemas script) died at
   its own checkout after 30 min — no plan, no baseline archived.
2. `#1990` (251a1929d, a .epr-meta-only push) logged `Loaded pipeline-baselines.json but it is
   empty — treating as cold start` and `No baseline commit available - diffing HEAD~1 (latest
   push only)`: it analysed 2 files, dispatched edge (died at clone) and genesis (skipped by
   cascade), and archived `[baseline:plan] __global__=251a1929, per-pipeline=0`. At post it
   logged `currentBuild.result=FAILURE — NOT advancing __global__ (preserving prior 251a1929)`:
   the "prior" it preserved was the value its own plan checkpoint wrote, because the run had
   LOADED nothing.
3. `#1991` (9ac7625aa, `[build:app]`) and `#1992` (a980fc681, untagged): `Analyzing 0 changed
   files`, `dispatched: (none)`. The a0601e49d changes sat behind the baseline with edge,
   genesis and app all unbuilt since before them. Only an explicit `[build:edge] [build:genesis]
   [build:app]` push (`#1993`, e68c909f5) dispatched the carrying builds.

The rule the orchestrator already states — "a failed orchestrator run must not absorb its own
diff" (`baseline-hold.test.mjs`, the #1875 lesson) — holds only when there was a loaded baseline
to fall back to. A cold start has none, so the failed-post branch preserves the plan checkpoint's
HEAD and the diff is absorbed exactly as in #1875.

## What to change

In the failed-post branch, when the run cold-started (loaded baseline empty), do not archive
`__global__` at all (leave the next run to cold-start too) OR, better, fall back to the last
SUCCESSFUL orchestrator build's revision as the baseline — `HEAD~1` is the wrong floor on a cold
start whenever more than one push arrived since the last success (here: three). Extend
`baseline-hold.test.mjs` with the cold-start case. Also worth reading: why `#1985`'s archived
`pipeline-baselines.json` was empty for `#1990` to load (a run that dispatches nothing may be
archiving an empty file; unverified).

## Probe

Replay the shape: a push whose orchestrator run dies before planning, then a docs-only push →
the next run must still dispatch the pipelines the first push touched, without a `[build:*]` tag.
