---
id: "backlog-edge-quiesce-gate-timeout-aborts"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Edge fleet-quiesce gate rides the build to global-timeout ABORT during catch-up windows — bound it warn-only; also sequence the DNA coordswap stage around in-flight edge rolls"
slug: "edge-quiesce-gate-timeout-aborts"
written: "2026-09-01"
author: "shift velocity-rungs-overnight"
status: "open"
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
