---
id: "backlog-fleet-full-arc-conductor-saturation-and-coordinated-warmup"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Every full-arc alpha conductor is CPU-pegged (three for a week, adam for a day) and nothing sheds for it — kitsune2 publish has no sender back-off, gossip runs upstream defaults, and the fleet rolls all peers at once; design a demand-driven warm-up at the seams we own"
slug: "fleet-full-arc-conductor-saturation-and-coordinated-warmup-2026-09-11"
written: "2026-09-11"
author: "shift 2026-09-11T09-00-land-batch-2c338124a (integrator) — operator question 'are the reactive-streams systems working?'"
status: "open"
priority: "high"
needs_brainstorm: true
tags: [conductor, kitsune2, backpressure, arc, gossip, publish, warm-up, edge-pipeline, D8, risk]
relatedNodeIds: []
cites:
  - elohim/elohim-storage/src/services/arc_actuator.rs
  - elohim/kitsune2/crates/core/src/factories/core_publish.rs
  - elohim/kitsune2/crates/gossip/src/config.rs
  - genesis/data/timeline/backlog/dataplane-reanchor-dead-remaining-rekeyed-peer.md
---

# The seams we own shed; the conductor underneath does not

## Measured (2026-09-11 13:00Z, Prometheus + Loki, read-only)

| conductor | CPU limit | 30 min rate | CFS throttled | for how long |
|---|---|---|---|---|
| adam-alpha (genesis pair, full arc, shem) | 4 | 4.00 | 100 % | ≈24 h pegged; climbed 0.6 → 4.0 over 7 days |
| gertrude / susan / eve (full arc, shem) | 1.5 | 1.50 | 100 % | the entire 7-day window |
| matthew (genesis pair, full arc, ethosengine) | 2 | 1.7–1.9 | 55 % | steady |
| james / jessica (leechers, arc 0) | 1 | 0.5–0.8 | 43–79 % | healthy |

- adam's conductor log: a wall of `kitsune2_transport_iroh … could not insert incoming publish ops request into queue (Full)` / `Error in recv_module_msg, peer connection will be closed` from ONE remote (`a85fd573…` = a full-arc agent behind `relay.alpha.elohim.host`, i.e. the ethosengine side, most likely matthew).
- The 2026-08-21 sys-validation cure IS live on the 0.7 line (`0 fetched of 3 missing dependencies (3 of 3 tracked … on a slow sweep)`) — this is NOT that spin.
- Storage-side backpressure is behaving: admission gate cap 5 (adam 3/5 in flight, shed 54 in 6 h, p90 wait ≈1 ms), doorway sheds writes with `503 catching-up + retryAfter` while reads stay 200, 0 pod restarts. The apex doorway therefore cannot forward blobs to adam's storage, which is why app builds #1704/#1705 failed — env-red, not code.

## What is missing (code facts)

1. **No demand signal in kitsune2 publish.** `core_publish.rs`: incoming publish ops go to `fetch.request_ops`; on a full queue the receiver warns and drops, the transport closes the connection, the sender warns (`could not send publish ops`) and reconnects. No per-peer back-off anywhere.
2. **Gossip knobs unset.** `gossip/src/config.rs` exposes `initiate_interval_ms`, `initiate_jitter_ms`, `min_initiate_interval_ms`, `max_concurrent_accepted_rounds`, `max_gossip_op_bytes`, `max_request_gossip_op_bytes`; no conductor config template in this repo sets any of them (every peer runs upstream defaults regardless of saturation). `arc_actuator.rs` already renders a `k2:` block — the render path exists.
3. **Binary arc lever.** `target_arc_factor ∈ {0,1}`; the genesis pair must stay full, so the only proven shed (leecher flip) cannot apply to adam/matthew. Fractional arcs remain upstream-blocked.
4. **Simultaneous fleet roll.** `elohim/holochain/Jenkinsfile` rolls all seven peers together; the quiesce gate's four legs are used only to MEASURE, never to SEQUENCE a deploy.

## Recommended design (one summary; brainstorm to confirm, then plan)

Demand-driven ramp at the three seams we own, smallest slice first:

1. **Fork patch — sender back-off** in `patches/kitsune2_transport_iroh` (already ours): per-peer exponential back-off on publish refusal; a closed connection is "no demand", not "reconnect and republish". Stops the storm at its source.
2. **Conductor boot profiles** rendered by `arc_actuator`'s `k2:` block and flipped by the rung-4 runtime-config watcher: `recovering` (long initiate interval, few concurrent rounds, small op budgets) → `steady` after `caughtUp` and CFS throttle < 0.7 hold for N minutes. AIMD: additive increase, multiplicative decrease on any shed.
3. **Storage reconcile ramp**: sweeps (projection reconcile, reanchor backfill, provide loop, sync fetch) start at concurrency 1 and expand while admission wait/shed stay low; halve on shed. Sensors = the existing `elohim_conductor_admission_*` histograms.
4. **Sequenced roll** in the edge pipeline: leechers first, then one full-arc holder at a time, each gated on its own caught-up + throttle reads (the quiesce legs as a deploy predicate).

Caveat: the present condition is steady-state saturation of full-arc holders, not only post-roll churn — the ramp removes spikes and reconnect storms; capacity needs sharding (upstream) or fewer full holders. Immediate operator lever (cluster-owned, not the repo's): flip one shem full-arc conductor to leecher, as james/jessica were.

## Progress 2026-09-12
- The "immediate operator lever" is repo-declared after all: `edgenodeArcFactor` per human in `genesis/orchestrator/data/deployments.json`, rendered into `target_arc_factor` by `elohim/holochain/Jenkinsfile` (`TARGET_ARC_FACTOR_PLACEHOLDER`). Pulled: **susan 1 → 0 (leecher)** — shem-only, non-genesis, recycled-laptop class, the fleet's highest actionable divergence with zero heal outcomes, CFS-throttled 1.0 at her 3000m bump. Rationale beside the jessica/james precedents in her `$arcFactorComment`; reversible by setting `"1"`. 6 full holders remain (adam, matthew, jessica, james, gertrude, eve). Lands with the next edge deploy; measure = her throttle ratio and adam's publish-queue-Full rate afterwards, which the new `ConductorCfsThrottleSustained` rule now watches.
- Sensing legs of the sibling atom landed the same night (alert rules + harvester hook), so the next week of saturation cannot go unwatched.

## Done when

- `rate(container_cpu_cfs_throttled_periods_total)/rate(container_cpu_cfs_periods_total)` < 0.9 on every alpha conductor for 24 h after a fleet roll; adam's publish-queue-Full lines drop to zero.
- An edge roll shows per-peer sequencing in its log and no `503 catching-up` on the apex during app staging.
