---
id: "backlog-adam-storage-second-restart-unexplained"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "adam storage pod restarted twice in one edge deploy window (second restart 16:56:58Z unexplained — no panic/OOM captured)"
slug: "adam-storage-second-restart-unexplained"
written: "2026-07-04"
author: "notary-authority-land shift"
status: "open"
priority: "low"
ci_status: open
jobs: [elohim-edge]
tags: [alpha, adam, restart, stability, observability]
cites:
  - genesis/data/timeline/backlog/view-federation-request-flakiness-mesh-wide.md
---

# adam storage second restart (16:56:58Z 2026-07-04) unexplained

During edge #1153's deploy window, `elohim-adam-alpha-0`'s elohim-node
container initialized twice: ~16:36:29Z (the deploy restart) and again at
16:56:58Z (log file rolled 0.log → 1.log; `Initializing Diesel connection
pool` recurs). No panic/OOM/exit-signal evidence was captured in the Loki
queries run. Single occurrence so far; adam has no direct log streams beyond
the container logs (the observability symmetry gap, task #7).

## Next move

If it recurs: targeted search for exit-code/signal evidence around the restart
timestamp + k8s event correlation (operator seat). One more data point turns
this into a real investigation; alone it is an observation.
