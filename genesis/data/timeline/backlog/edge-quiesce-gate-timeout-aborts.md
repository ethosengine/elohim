---
id: "backlog-edge-quiesce-gate-timeout-aborts"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Edge fleet-quiesce gate rides the build to global-timeout ABORT during catch-up windows — bound it warn-only; also sequence the DNA coordswap stage around in-flight edge rolls"
slug: "edge-quiesce-gate-timeout-aborts"
written: "2026-09-01"
author: "shift velocity-rungs-overnight"
status: "backlog"
priority: "medium"
jobs: [elohim-edge, elohim-holochain]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
tags: [ci, quiesce, cycle-time]
---

Observed 2026-09-01: edge #1406/#1407/#1408 all completed their DEPLOY
stages then died ABORTED ("Timeout has been exceeded") inside the
fleet-quiesce measurement, which cannot pass mid-catch-up — so a healthy
deploy reads as ABORTED (the NOT_BUILT/lossy-measure museum family). Bound
the quiesce leg with its own timeout + warn-only verdict (report the last
measurement instead of eating the build), keeping strict mode for
[edge:validate-only] runs. Second, both DNA builds that raced an in-flight
edge roll (#1412 gertrude, #1413 adam) hit connect-refused mid-roll peers
and halted (correct, but noisy) — the COORDSWAP stage should defer or
skip-with-verdict while an edge deploy is rolling storage pods.

2026-09-02: both halves landed in-tree. `runDataplaneValidation()` (top-level
def beside `runMeshQuiesceMeasure()`, `elohim/holochain/Jenkinsfile`) gives
the quiesce leg its own 55-minute `timeout{}` — inside the pipeline-global
120 min — so a healthy deploy can no longer read as ABORTED; warn-only
(UNSTABLE) on ordinary deploy builds, strict (FAILURE) on
`[edge:validate-only]` recording runs. `fleet-coordswap.sh`'s
`run_rolling_apply` now records a peer that refuses the connection
(`LAST_HTTP_CODE=000`) as `deferred` and continues the rollout instead of
halting it, returning `4` when any peer deferred and none failed;
`fleet-coordswap-dispatch.sh` prints `COORDSWAP: DEFERRED — …` on that rc
and still exits 0 (warn-only policy unchanged). Fleet-unproven until the
next edge build shows UNSTABLE-not-ABORTED and the next DNA build racing an
in-flight edge roll shows DEFERRED-not-halted.

2026-09-19: reopened — the claim above is falsified by edge/dev 1463. The quiesce leg's own
55-minute bound only protects a build that REACHES validation with 55 minutes left. 1463 spent
62 minutes building (doorway quality gate 1162s, storage build 2002s — both rebuilt because the
image tag is commit-derived) and 56 minutes on `Deploy Edge Node - Alpha` (SUCCESS, every peer and
both doorways rolled), entered Dataplane Validation at minute 118, and was killed at 120 by the
pipeline-global `timeout(time: 120)` — `Timeout has been exceeded`, result ABORTED, orchestrator
FAILURE, app and genesis never dispatched. The stage bounds sum past the global one by design
arithmetic: build ~60 + roll budget 45 + storage/doorway rollouts ~15 + validation 55.

- chain / between "alpha deploy succeeded" → "orchestrator dispatches app and genesis" / missing node
  "a deploy that succeeded is reported as succeeded whether or not validation had time to measure":
  assertion — a full cold build + full roll finishes UNSTABLE-or-better with the global limit untouched;
  probe — edge build duration vs. `options { timeout }`. Remedy shape: raise the global limit to cover
  the stage sum (≥ 200 min), or start validation's clock from a budget that subtracts elapsed time and
  reports no-measure instead of being interrupted. State: **not built** (held out of the 09-19 shift's
  pushes on purpose — any change under the edge watch globs rebuilds and re-rolls the whole fleet).

