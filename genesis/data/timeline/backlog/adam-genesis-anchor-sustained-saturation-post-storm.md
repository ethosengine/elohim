---
id: "backlog-adam-genesis-anchor-sustained-saturation-post-storm"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "adam (genesis anchor) sustained DB-read-pool saturation + conductor-unreachable + DHT non-convergence after the storm-pod deletion — the arc-shrink durable lever does NOT apply to a genesis anchor"
slug: "adam-genesis-anchor-sustained-saturation-post-storm"
written: "2026-07-07"
author: "pipeline-shakeout shift (overnight arc)"
status: "proposed"
priority: "high"
area: "substrate/conductor-authority-arc"
domain: "operator"
jobs: [elohim, elohim-genesis]
relatedNodeIds:
  - "memory:project_per_node_memory_is_conductor_authority_arc"
  - "memory:project_storage_metrics_surface_and_leak_verdict"
  - "memory:project_alpha_topology_bootstrap_pair"
  - "memory:project_prod_main_lag_vs_alpha_dev"
  - "memory:feedback_household_nodes_is_the_stable_floor"
cites:
  - genesis/data/timeline/backlog/ci-alpha-cluster-degraded-substrate.md
  - genesis/data/timeline/backlog/ci-rbac-jenkins-deployer.md
  - genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-memory-scaling.md
tags: [substrate, adam, genesis-anchor, arc-factor, db-read-pool-saturation, conductor-unreachable, dht-non-convergence, host-green-not-ci-green, operator-domain, post-storm]
---

# adam sustained saturation after the storm-pod deletion — no tree-fixable scale lever (genesis anchor can't arc-shrink)

## Context — this is a NEW phase, distinct from the inventory-snapshot storm

The 2026-07-06 inventory-snapshot storm was healed on the 7 active peers (receive-side snapshot
idempotency `b1ef627ed` + edge #1160 deploy). The operator then deleted the 7 suspended zombie
conductors (~2026-07-07T01:50Z). A first re-probe at ~02:06Z read a lull (storm-pod log volume →0,
DHT anchor sweep `caught_up=true`) but explicitly left **adam's content-sweep UNVERIFIED**. By
~05:50Z adam had re-saturated. This item captures that post-deletion adam-specific degradation — a
sibling of, but distinct from, the write-amplification storm in `ci-alpha-cluster-degraded-substrate.md`.

## Evidence (Loki/Prometheus, ns elohim-alpha, ~04:20–05:50Z 2026-07-07; CI: genesis #1263)

- **DB read-connection pool sustainedly saturated on adam.** `elohim_node_db_max_readers=8`
  (max_readers = `max(2*cpus, 8)`, so adam has ≤4 CPUs). Live `Database read connection is saturated`
  lines, per-DNA-cache Util samples ranging `650%`–`3300%` (07:05Z re-check; an earlier 05:50 read cited
  higher figures that a later pull couldn't reconcile — treat the exact peak as uncertain, but 650–3300%
  on 8 slots is heavy oversubscription regardless). Rate ~362–383 saturation msgs/sec, **flat across
  05:50→07:05Z (~75+ min, no downward trend)** — sustained, not a settling transient.
- **Inventory churn REBOUNDING** on adam: per-10-min applies dipped ~05:00–05:10 then climbed back
  toward storm levels by 05:40–05:50 (getting worse, not settling).
- **Conductor connection errors** (attribution corrected 07:05Z re-check): the `Failed to connect to
  conductor: Connection refused (os error 111)` loop at ~05:50 was against **susan's** conductor
  (`ws://elohim-susan-alpha-0…:8445`), and it **cleared at 06:57Z** — NOT confirmed to be adam's
  conductor (no doorway→`elohim-adam-alpha-0` websocket line could be grounded in the window). Adam's
  degradation is carried by the DB-read-pool + EPR-failover signals below, not this. adam's pod is UP
  (0 restarts across the whole window).
- **EPR router failing over off adam for 90+ min**: `EPR router DEGRADED: primary storage gave no
  projections … primary_url=…adam…:8090, primary_state:"empty", serving_url:…matthew…:8090`, repeating
  every ~30s since 04:21. `https://elohim.host/version` returns 200 ONLY because doorway fails over to
  matthew — textbook **host-green ≠ CI-green** (anti-patterns museum).
- **CI corroboration**: genesis #1263 `substrate-verify.sh propagation` failed — custody-blob DHT
  gossip `missing on: adam after 300s`. app #1596 UNSTABLE = the SPA-blob deploy-verify legs for
  `elohim.host` (adam-backed) 503'd. Both contemporaneous with the Loki evidence.

## Scale levers — arc-shrink is fenced off, but CPU/db-pool provisioning IS the lever (corrected 2026-07-07)

The documented durable fix for conductor-authority-arc overload (working set ∝ corpus → OOM /
saturation) is **arc-shrink** (`target_arc_factor` 1→0, leecher) — see the arc-memory spec and the
`deployments.json` `$arcFactorComment` history (jessica, james both flipped to leecher for exactly
this). **But adam is `genesisPeer:true`.** The deployments.json comments are explicit: the genesis
pair (adam+matthew) MUST stay full-arc or the DHT partitions. So:
- **arc-shrink (the one deployed lever, {0,1} only — fractional is kitsune2-blocked): NOT available
  for adam.** It must hold the full authority arc.
- **`max_readers` = `max(2*cpus, 8)` — CPU-driven, and adam was UNDER-PROVISIONED (this IS the lever).**
  At adam's 3000m limit the conductor detected 3 cores → floored at 8 readers. matthew got a 2026-06-15
  bump to 4000m + `STORAGE_DB_POOL_SIZE=20`; adam (a separate hand-maintained manifest, not the template)
  never received it and sat at 3000m + the unset default pool 10 — a manifest-propagation DRIFT, not an
  unfixable ceiling. Bumping adam's CPU limit >4000m lifts max_readers above the 8 floor (8000m → 16). See Resolution.
- Applying `target_arc_factor` at all **requires a conductor RESTART** (elohim-storage `http.rs:192`).

So adam is in the known hard tension: the anchor holds everything and saturates under
corpus+reconvergence+seed load, with the durable lever fenced off by its anchor role.

## Operator actions (this is operator/substrate-owned — agent cannot kubectl)

1. **Restart adam's conductor** — the acute symptom is `Connection refused` to adam's conductor while
   its pod stays up; a targeted conductor/pod restart is the most likely quick clear (also re-establishes
   the doorway→conductor bridge and drains the piled-up read queue). NOT the full edge deploy (which
   rollout-restarts every conductor); a targeted adam restart.
2. **Relieve load while it reconciles** — the inventory churn is rebounding; consider pausing further
   seeding against adam until it converges, and confirm the 7 deletions actually reduced gossip fan-in.
3. **Resource headroom (stopgap, not durable)** — adam at ≤4 CPUs / its current RAM is under-provisioned
   for full-arc anchor duty on the current corpus; a CPU/RAM bump raises `max_readers` and arc headroom
   but the deployments.json history is clear these are stopgaps.
4. **Durable (design)** — the real fix is the anchor-scale question the arc-memory spec opens: a genesis
   anchor cannot arc-shrink, so its scale ceiling must be met another way (dedicated resources, read-replica
   projection, or splitting anchor duty). Design-level; tracked for a future substrate arc.

## Resolution (2026-07-07) — the lever was CPU/db-pool provisioning drift, not an unfixable ceiling

The operator identified the real (tree-fixable) lever: adam was under-provisioned vs matthew, not hitting
an unfixable ceiling. Fix landed in `genesis/orchestrator/manifests/humans/adam-firstman.yaml`:
- **CPU limit 3000m → 8000m** (request 1000m → 1500m): 8 detected cores → conductor `max_readers` 8 → 16
  (2× the saturating pool) + doorway-B request-serving headroom.
- **`STORAGE_DB_POOL_SIZE` unset(10) → 20**: matches matthew; closes the storage content.db r2d2 pool gap.
- Corrects the propagation drift — matthew's 2026-06-15 4000m/pool-20 bump never reached adam's separate
  explicit manifest.

Applies via the **edge deploy** (`elohim/holochain/build-manifest.json` `deploy-manifests` sources include
`manifests/humans/**`): the Deploy Edge Node stage re-renders + `kubectl apply`s adam's StatefulSet with
the new resources and rollout-restarts adam — which is also the conductor restart it needed. (Operator may
`kubectl apply` the resized StatefulSet directly for a faster targeted restart.) **This supersedes the
earlier "no tree-fixable lever" framing above** for the immediate saturation; arc-shrink stays fenced off
(a separate long-term anchor-scale question), but provisioning was the operative fix.

## Shift disposition

app + genesis "reliably UNSTABLE→SUCCESS" was **BLOCKED-BY-SUBSTRATE** on adam's health — NOT a code-red
(the `alpha.elohim.host` legs already went green in app #1596). Originally read as not-tree-fixable and
escalated; the operator then identified the CPU/db-pool provisioning drift (see Resolution), which IS a
repo fix — now landed + pushed. **Falsifier:** post-apply, adam's `max_readers` metric →16, DB-read
saturation drains, EPR router serves from adam again (`primary_state` not "empty") → then a fresh
`[build:app,genesis]` should show the `elohim.host`/adam legs green.
