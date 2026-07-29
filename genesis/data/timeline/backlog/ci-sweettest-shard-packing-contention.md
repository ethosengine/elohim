---
id: "backlog-ci-sweettest-shard-packing-contention"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Sweettest 4-way shards pack 2-3 per node (PVC pinning) — 95%+ CPU blows 30s gossip-visibility deadlines"
slug: "ci-sweettest-shard-packing-contention"
written: "2026-07-29"
author: "deliver-the-saga morning sprint"
status: "wip"
priority: "medium"
ci_status: in-progress
jobs: [elohim-holochain]
tags: [ci, sweettest, flake, contention, scheduling]
---

# Sweettest shard packing saturates one node and fails timing-sensitive tests

`content_visible_across_agents` failed elohim-holochain #1376/#1379/#1380 and
passed #1377/#1378. Prometheus correlation (2026-07-29) refuted the co-running
edge/genesis hypothesis (their driver pods run on `intel-nuc`; sweettest shards
on `thinkc-p1s`/`thinkc-p0h`) and identified intra-job self-contention:

- Every FAIL: shard-2 shared `thinkc-p1s` with 1-2 sibling shards; node CPU
  95-99% during the visibility window (#1379 had 3-of-4 shards packed, 98.8%).
- PASS #1377: same co-location, peak 85%. PASS #1378: shard-2 avoided p1s
  entirely (p0h, 78%).

Shards are node-pinned by design — hostpath cache PVCs (see
`project_ci_storage_topology`: pin or thrash on volume binding) — so plain pod
anti-affinity would break cache locality. Applied mitigation (this sprint):
`content_visible_across_agents`'s poll deadline 30s → 120s (breaks on first
success; healthy runs pay nothing).

## Remaining work (claimable)

1. Confirm the mechanism at container level: query
   `container_cpu_cfs_throttled_seconds_total` for `sweettest-shard-*` pods over
   the fail windows; also confirm the IP→node mapping via `kube_node_info`
   (the current mapping is inferred, flagged by the investigation).
2. Sweep the other ~10 `Duration::from_secs(30)` deadlines in
   `tests/sweettest/src/tests/*.rs` — widen any that are poll-until-success
   loops (zero healthy-run cost); leave hard timeouts that bound genuine hangs.
3. Structural option if recurrence continues: stagger shard start (or 4→2
   shards per node) in the DNA Jenkinsfile pod templates, respecting the PVC
   node pins; or provision a second sweettest-target PVC on another node.

## Evidence

Correlation session 2026-07-29 (session_01F7sMjfrAaADtgYHv9XM2nA): Prometheus
`kube_pod_info{namespace="jenkins"}` placement + `instance:node_cpu_utilisation:rate5m`
peaks 0.95/0.988/0.95 (fails) vs 0.85/0.78 (passes).
