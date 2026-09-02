---
id: "backlog-edge-quiesce-gate-timeout-aborts"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Edge fleet-quiesce gate rides the build to global-timeout ABORT during catch-up windows — bound it warn-only; also sequence the DNA coordswap stage around in-flight edge rolls"
slug: "edge-quiesce-gate-timeout-aborts"
written: "2026-09-01"
author: "shift velocity-rungs-overnight"
status: "in-tree"
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
