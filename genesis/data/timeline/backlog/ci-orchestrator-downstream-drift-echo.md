---
id: "backlog-ci-orchestrator-downstream-drift-echo"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Orchestrator Reconcile-Build-Graph UNSTABLE — downstream-drift echo of genesis/elohim/edge UNSTABLE+FAILURE (museum trap #1 working as designed)"
slug: "ci-orchestrator-downstream-drift-echo"
written: "2026-06-06"
author: "ci-failure-triage"
status: "backlog"
priority: "low"
ci_status: blocked
fingerprints: [4508f1172d15, b169c3b9034c]
jobs: [elohim-orchestrator]
relatedNodeIds: []
tags: [ci, elohim-orchestrator, reconcile-build-graph, downstream-echo, museum-trap-1, not-a-root-cause]
cites:
  - https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/1167/
  - https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/1168/
  - genesis/orchestrator/README.md
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# Orchestrator UNSTABLE is a downstream echo, not an independent concern

## The failure

```
4508f1172d15  elohim-orchestrator — red build, stage:Reconcile Build Graph   (1164–1167)
b169c3b9034c  elohim-orchestrator — red build, stage:elohim-genesis          (1168)
```

Both builds are **UNSTABLE** (the harvester's "red build" label is its
classifier token; the actual result is UNSTABLE — confirmed via the build API).

- #1167, **Reconcile Build Graph**: `📊 BUILD GRAPH RECONCILIATION — verdict:
  DRIFT / drift: 3 unstable, 1 unknown-result`, with
  `elohim=UNSTABLE, elohim-storybook=DISPATCHED, elohim-edge=UNSTABLE,
  elohim-genesis=UNSTABLE`. Stage warns `Build graph drift detected — see
  build-graph-reconciliation.json`.
- #1168, **elohim-genesis** stage: `Build elohim-genesis » dev #1101 completed:
  FAILURE`; `Results: elohim=UNSTABLE, …, elohim-genesis=FAILURE`;
  `Investigation pointers: • elohim-genesis FAILURE — …/elohim-genesis/job/dev/
  1101/`. Orchestrator's own status UNSTABLE: "Genesis failed - seeding or
  tests may have issues."

## Verdict

**Not a root cause — a downstream echo, surfacing correctly.** This is **museum
trap #1** in its intended-behavior form: the orchestrator's Reconcile Build
Graph / per-child stages exist precisely TO surface downstream UNSTABLE/FAILURE
as orchestrator UNSTABLE (never FAILURE — the fail-regime boundary in the museum
record, `2026-06-02-…-museum.md` lines 86–91: post-dispatch stages must
`catchError(…UNSTABLE…)`, which they do here). The orchestrator is reporting the
truth about its children, not failing itself.

## Root cause

The children's states, already canonicalized:

- `elohim-genesis` UNSTABLE (1100) / FAILURE (1101) → the TS2739 concern
  (`ci-genesis-projectionspec-ts2739.md`, already fixed) + the degraded-substrate
  concern (`ci-alpha-cluster-degraded-substrate.md`).
- `elohim` UNSTABLE → Upload SPA Blob against degraded alpha
  (`ci-alpha-cluster-degraded-substrate.md`) + the lamad bundle build
  (`ci-lamad-attention-flow-null-contentid.md`, already fixed).
- `elohim-edge` UNSTABLE → alpha-doorway deploy against degraded cluster
  (`ci-alpha-cluster-degraded-substrate.md`) + the doorway image fixture gate
  (`ci-doorway-dockerfile-fixture-context.md`, already fixed).

The orchestrator carries **no independent defect**. There is nothing to fix in
the orchestrator Jenkinsfile or graph-walker.

## Current decision

**BLOCKED-as-echo — resolves automatically when the upstream concerns resolve.**
No tree change: per the museum record, an agent reading a red/UNSTABLE
orchestrator as its own bug and "fixing" the orchestrator is the canonical
anti-pattern (the deepest trap). The correct move is to let the four real
concerns close (one substrate-blocked, three already-fixed-await-confirm); the
orchestrator's drift verdict clears on the same green streak.

Both fingerprints set `status: blocked` in the ledger (blocker: upstream
children — see the four cited backlog entries). No `triaged_at_build` (nothing
landed in the orchestrator). Recurrence tracks the children and is expected until
they clear — not an orchestrator re-fire bug.

## Fix trail

- No change (intentional — orchestrator is reporting correctly; the fix surface
  is the four upstream concerns).
- This entry exists so the next sentinel run does not re-investigate the
  orchestrator as a novel concern: it is, and will remain, an echo until the
  children are green.
