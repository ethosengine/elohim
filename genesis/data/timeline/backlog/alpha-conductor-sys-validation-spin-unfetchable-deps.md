---
id: "backlog-alpha-conductor-sys-validation-spin-unfetchable-deps"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Every alpha storage pod is pegged at its CPU quota because the conductor's sys-validation spins on hundreds–thousands of unfetchable dependencies (NoPeersForLocation) — 40× read-pool saturation, ~1000 log lines/s per pod; the local mesh does not reproduce it"
slug: "alpha-conductor-sys-validation-spin-unfetchable-deps"
written: "2026-08-21"
author: "claude (operator asked whether the local mesh shows alpha's CPU spikes; Prometheus + Loki + local sampling)"
status: "refined"
priority: "critical"
jobs: [elohim-conductor, elohim-edge]
nodes: [elohim-adam-alpha, elohim-matthew-alpha, elohim-jessica-alpha, elohim-james-alpha, elohim-eve-alpha, elohim-susan-alpha, elohim-gertrude-alpha]
relatedNodeIds:
  - "memory:project_conductor_storm_starves_storage_reads"
  - "memory:project_full_arc_authority_disables_network_get"
  - "memory:project_ghost_declaration_deadlock_batch3"
  - "memory:feedback_mesh_is_the_proving_ground"
tags: [conductor, holochain, sys-validation, cpu, spin-loop, full-arc, observability, loki, alpha]
cites:
  - elohim/elohim-storage/src/conductor_admission.rs
  - genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
---

# Alpha conductors spin on unfetchable dependencies — the fleet-wide CPU peg (2026-08-21)

## What the fleet shows (Prometheus, instant at 16:00Z; 48 h range corroborates)

| pod | cpu limit | usage (10m rate) | CFS throttled periods |
|---|---|---|---|
| adam | 8 | 7.55 | 48 % |
| matthew | 4 | 3.94 | 99.9 % |
| susan / gertrude / eve | 3 | 2.9–3.0 | 97 % |
| jessica / james | 2 | 2.00 | 100 % |

**Chronic, not a spike:** every pod sits at its quota every hour for the last 48 h, whatever the quota is
(2, 3, 4, 8). A process that consumes exactly what it is given regardless of the limit is a spin, not a
workload. Storage's own activity is negligible: zome calls 0.04–0.10/s, reconcile sweeps every ~5 min,
heal outcomes < 1/s, `elohim_conductor_admission_in_flight` 0–2 of capacity 5 (13 on adam). The CPU is the
**conductor** in the same `elohim-node` container.

## The loop (Loki, matthew-alpha, 15 min window)

- **882,529 non-JSON (conductor) log lines / 15 min ≈ 980 lines/s** (jessica 347k, adam 85k).
- ~600–950/s: `holochain_sqlite::db::access: Database read connection is saturated. Util 4037.50%` —
  the DHT read pool (`db_max_readers` = 8; adam 16) is **~40× oversubscribed**, split across four DNAs
  (369k / 302k / 38k / 0.8k per 15 min).
- ~200/s: `holochain_cascade: No peers to fetch record from e=NoPeersForLocation("get", …)`.
- every 10 s: `sys_validation_workflow: Sys validation sleeping for 10s, with 0 fetched of 873 missing dependencies`.
- **Missing-dependency counts (max over 10 min): jessica 2849 · james 2666 · eve 1606 · matthew 1508 · adam 353**; never drained over 3 h (min per DNA still 109–768). `0 fetched` every cycle.

Mechanism: sys-validation wakes, attempts to fetch every missing dependency → each attempt is a cascade
`get` → on a full-arc fleet every get is local-only (`NoPeersForLocation`) → the attempt saturates the
read pool and logs at INFO → sleep 10 s → repeat, forever. The dependency set is permanently unfetchable
(the ghost/fossil class left by the 2026-07-24 DNA reinstall re-key is the prime suspect — ops that
reference actions no living chain holds). Cost scales with the missing set; the logging alone (~1000
lines/s × 7 pods) is a second CPU/I-O sink and is plausibly what has been OOM-killing Loki all day
(35 kills in `dmesg`, all Loki's pod).

## Local mesh: does NOT reproduce

Three conductors at 0.16–0.21 cores average (50 s window), storage 0.06 cores under a 120-request read
burst; **zero** `read connection is saturated` lines in `.sandbox_run_log` (3,145 lines in 204 min ≈
0.26 lines/s vs ~980/s on alpha). The local corpus was authored on the mesh, so no dependency is missing.
To make this Act I-provable, stage the class deliberately: author content on one peer, then re-key or
wipe that peer so its actions become unfetchable, and assert sys-validation's retry budget and log rate
(a `@requires:owned-substrate` chaos scenario).

## Cure directions (decide, then prove on the mesh first)

1. **Conductor fork (`elohim/holochain-conductor`)** — bounded retry with exponential backoff for missing
   deps in `sys_validation_workflow`; quarantine/abandon ops whose deps are unfetchable after a budget; log
   the saturation line at most once per window (or at DEBUG). Ships as a conductor image
   (`[conductor:…]`), no DNA change.
2. **Relief now, not a fix:** `RUST_LOG` directive in the edge manifest to drop
   `holochain_sqlite::db::access` / `holochain_cascade` below INFO — removes the logging sink and saves Loki;
   the fetch loop itself stays.
3. **Identify the missing set**: admin `dump_conductor_state` / `DumpFullState` on one pod; if no peer
   holds the referenced ops, they are fossils and a reinstall/migration of the affected DNA rows (with
   lineage, per the `ALLOW_DNA_REINSTALL` rules) is the honest cure for the data.
4. **Not a fix:** raising `db_max_readers` or CPU limits — the loop consumes whatever it is given.

## How this was found without a profiler (keep as the method)

Flat-at-quota CPU regardless of limit → storage activity counters near zero → log-rate spectroscopy
(lines/s by message) → a stuck counter (`missing dependencies` never reaching 0) with a fixed period
(10 s) and a saturation gauge (`Util 4000%`). A spin-loop is a constant amplitude with a constant period.

## Cure 1 implemented in the conductor fork (2026-08-21)

Branch `fix/sys-validation-unfetchable-deps-backoff` in `elohim/holochain-conductor`
(commits `b9c7458ae`, `c9a6c4439`; submodule pointer deliberately NOT bumped — that is the
integrator's move, and it dispatches a conductor image build).

- Per-dependency exponential backoff in `sys_validation_workflow`, with two independent schedules:
  local re-checks cap at 60s (so a dependency arriving by gossip is still noticed within a minute),
  network fetches cap at 1h. Both start due immediately, so a merely-late dependency is unaffected.
  After 12 failed network fetches a dependency is reported unfetchable and moves to a slow sweep —
  never dropped, never validated-as-good.
- The local dependency lookup fan-out was an unbounded `join_all`; it is now bounded to 10 in
  flight. That, not the network fetch, was the source of the `Util 4037%` read-pool saturation.
- The saturation log line is throttled to once per 30s per database with a suppressed count, and
  the true rate is now the `hc.db.connections.read_saturation` counter.
- New metrics: `hc.conductor.sys_validation.missing_dependencies`,
  `hc.conductor.sys_validation.unfetchable_dependencies`, `hc.db.connections.read_saturation`.

**What this does NOT do — still open:**

1. It stops the spin; it does not drain the missing set. Cure direction 3 (identify the fossil set
   via `DumpFullState`, and migrate or reinstall the affected DNA rows with lineage) is still the
   honest cure for the *data*. The new unfetchable-dependency metric is the measure for it: a
   non-zero, non-falling value names the fossil count directly.
2. Every pass still re-writes `AwaitingSysDeps` for each waiting op (`put_validation_limbo`), so N
   ops means N no-op UPDATEs every retry interval. Bounded and much smaller than the read storm,
   but real; skipping the write when the stage is unchanged is a separate small change.
3. Cure direction 2 (a `RUST_LOG` directive dropping `holochain_sqlite::db::access` /
   `holochain_cascade` below INFO) is now redundant for the saturation line but is still the
   zero-deploy relief for the `NoPeersForLocation` line until the fork image ships.
4. Not reproduced on the local mesh. The `@requires:owned-substrate` chaos scenario named above
   (author on one peer, re-key that peer, assert retry budget and log rate) is still unwritten and
   is what would make this Act I-provable.
