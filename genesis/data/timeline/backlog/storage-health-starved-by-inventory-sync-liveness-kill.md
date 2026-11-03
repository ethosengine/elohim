---
id: "backlog-storage-health-starved-by-inventory-sync-liveness-kill"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Content-inventory sync starves the storage container's /health, so liveness kills busy-but-healthy shem peers (eve, gertrude, susan: 14–17 restarts in 24 h)"
slug: "storage-health-starved-by-inventory-sync-liveness-kill"
written: "2026-11-03"
author: "claude (row-5 precursor session a340d2e2), from the k8s-side read of 2026-11-03"
status: "backlog"
priority: "high"
tags: [elohim-storage, liveness, health-probe, inventory-sync, cpu-throttle, shem, alpha, design-signal, resource-limit-raise, performance, perf-queue, friction-mechanical, plane-custody, unit-peer, phase-steady, lane-background]
relatedNodeIds:
  - "backlog-edge-deploy-ready-gate-liveness-only"
  - "backlog-steward-key-chain-fork-warrant-exile"
cites:
  - elohim/elohim-storage/src/http.rs
  - elohim/elohim-storage/src/p2p/inventory_gossip.rs
  - genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml
  - scripts/ci/conductor-split-budget.sh
  - genesis/orchestrator/data/deployments.json
---

# Inventory sync starves /health, and liveness kills a healthy pod

## What was observed (k8s side, 2026-11-03 ~21:30Z; nothing captured before that)

- Over 24 h, eve, gertrude and susan's `-alpha-0` pods (storage container `elohim-node`)
  restarted 14–17 times. The most recent restart was about 20:38Z: eve and susan both exited
  Error/137, 8 seconds apart.
- Susan's last log line is `stopping cpu profiler`, a graceful shutdown cut off by SIGKILL. That
  is a liveness kill. `container_oom_events_total` is 0 for all three. Gertrude's previous exit
  was OOMKilled at 1152Mi, so some restarts are memory.
- CFS throttling of the storage container:

  | Pod | periods throttled |
  |---|---|
  | eve | 19,796 of 29,544 (67 %) |
  | susan | 19,469 of 29,460 (66 %) |
  | gertrude | 16,499 of 25,496 (65 %) |

  Each sits at about 600–630m of a 750m limit.
- The stalls line up with content-inventory and head-hint sync bursts
  (`Received content inventory from peer … total 3776`,
  `retained content-head hint queued …`). The liveness probe allows 20 s timeout × 10 failures ×
  30 s period, so roughly 5 minutes of no answer kills the pod.
- shem ran at 93 % CPU (22.4 of 24 cores). Adam's compute worker took about 4.7 cores of that,
  retrying a grant that the chain-fork exile makes impossible.

## Why the limit is 750m

`scripts/ci/conductor-split-budget.sh` splits a human's `edgenodeCpuLimit`: 3/4 to the conductor
and 1/4 to storage, on the premise that the conductor is the side that throttles. The 2026-08-09
raise to 3000m on these three was made for the conductor, before the rung-2 split. Since the split,
storage has inherited 750m, and storage is the side throttling.

## The design question (the raise is the signal, not the cure)

This unit carries three earlier raise stamps (`$cpuLimitBump`, `$cpuThrottleBumpComment`,
`$memoryBumpComment`), and its liveness probe was already loosened once (timeout 15 → 20 s,
failures 5 → 10). The operator-directed raise of 2026-11-03 (`$storageSplitRaiseComment`: storage
1500m, conductor pinned at 2250m) buys time. It does not attenuate the disturbance.

- **Disturbance:** one inventory burst (thousands of hints at once) occupies storage's tokio
  workers long enough that `/health` cannot be scheduled within the probe window.
  - `handle_health` itself does nothing slow. It uses `try_lock` on the conductor manager, atomic
    counters and blob-store stats (`http.rs` `handle_health`).
  - So the starvation is in scheduling, not in the handler. **Unverified:** this has not been
    measured. Measure it before acting on it.
- **Seam we own:** elohim-storage. Inventory work and the liveness answer share one runtime.
- **Candidate cures (source side, pick by measurement):**
  1. Pace the inventory burst. Process received hints in bounded batches that yield, rather than
     all at once (`p2p/inventory_gossip.rs`).
  2. Serve liveness from a path that sync cannot starve, such as a dedicated thread or small
     runtime that answers only "the process is alive". Readiness stays on the full `/health`.
  3. Read whether the burst size (3,776 hints) is expected for this corpus, or itself a sign of a
     re-announce loop. The 2026-09-13 lesson applies: a small corpus with a large burn points to a
     loop, not to missing headroom.
- **Probe for the cure:** over 24 h at the split share (storage back at 750m),
  `kube_pod_container_status_restarts_total` on the three storage containers stays at 0, and
  `/health` p99 latency during a logged inventory burst stays under 1 s.

## Restoration

When the cure lands and the probe above holds, remove `storageCpuLimit` and `conductorCpuLimit` from
eve, gertrude and susan, and return `edgenodeCpuLimit` and `resourceOverride.cpuLimit` to 3000m.

**Falsifier for the 2026-11-03 raise:** storage pins at about 1500m and liveness restarts continue.
That would show the burn is a loop. Revert and do this design pass first.

## Memory (not raised)

Gertrude OOMKilled once at 1152Mi; susan read 918Mi of 1152Mi. Memory is incompressible and has no
capacity ratification, so the gate refers any memory raise to the operator. Two options:
- Rebalance the 3Gi split budget-neutrally. This needs a conductor working-set read first, since
  the conductor gets 1920Mi.
- Have the operator ratify a raise.
