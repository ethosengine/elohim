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
tags: [conductor, holochain, sys-validation, cpu, spin-loop, full-arc, observability, loki, alpha, performance, perf-cpu, trustful-declared, friction-mechanical, friction-wait, plane-notary, fused-planes, unit-item, phase-steady, lane-borrowed]
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

## Mesh reproduction — two attempts, neither reproduced, and why (2026-08-21)

Baseline, before any staging: three conductors at **0.10 cores** each, `.sandbox_run_log` at
**0.02 lines/s total**, zero `read connection is saturated`, zero sys-validation lines. Alpha, for
contrast, runs ~980 lines/s per pod with missing-dependency counts of 1508–2849 that never drain.
Instruments: `app/elohim-app/scripts/hc-mesh-spin-detector.sh` (the measure) and
`hc-mesh-chaos-rekey.sh` (the staging). Summaries under `/tmp/elohim-local-mesh/spin/`.

**v1 — author on one peer, let the others fetch it, then re-key that peer.** 5 nodes authored on
james's storage, each re-authored onto **james's own conductor** by `reanchor_backfill`
(`dhtAnchorHash uhCkk…`, `trust: notarized`, 5/5 in 45 s); matthew and jessica each came to hold 4/5;
james re-keyed in 13 s (`uhCAkOZ2b…` → `uhCAkuV2p0…`), storage DB kept on purpose. **Verdict QUIET.**
CPU never left baseline; the re-keyed peer spiked to 0.67 cores, fell to 0.29, and settled at 0.08
within four cycles — a peer re-syncing, decreasing, not a spin.

*Why it could not reproduce:* the neighbours had already **fetched and validated** james's ops before
the re-key. Validated ops are integrated, so nothing is pending and there is no dependency to chase.
Re-keying does not retroactively un-validate work already done. Alpha's condition is the opposite —
the reinstall left ops **in the validation queue** whose dependencies were never fetched and now never
can be.

**v2 — deny the neighbours the chain's head.** Both neighbour conductors frozen with `SIGSTOP`
(trap-guaranteed thaw), james authored a base node plus **24 dependent revisions** onto his source
chain, 8 s thaw, then re-key (`uhCAkuV2p0…` → `uhCAkMgQJn…`). After the thaw the neighbours had
received **nothing**: `Content not found: chain-v2-stock060` on both, while james held it at rev 24,
notarized. The 8 s window did not deliver a partial tail; it delivered no tail. **Not measured** — with
the neighbours holding nothing that references james, a measure would have scored the staging, not the
conductor.

## Staging v3 — the design (not yet run)

Two changes, and one assertion that must pass before any measurement is believed.

1. **Widen the delivery window, and drive it by observation rather than by a timer.** 8 s was chosen
   arbitrarily and produced nothing. Either widen to tens of seconds, or — better — invert the control:
   author the chain, then **wait until the neighbours' conductors have actually received ops** from it,
   and only THEN re-key. The re-key is the instant the dependencies become unproducible, so it must
   come after partial delivery, not race it.
2. **Read the conductor, not the projection.** v2's "did they get it?" check queried
   `/db/content/<id>` — a *storage projection*, synced on its own reconcile cadence and not what
   sys-validation holds. The op store and validation queue are conductor state. Read them via the admin
   interface (`dump_state` / `DumpFullState`, or `hc sandbox call dump-conductor-state`), or off the
   **per-conductor log**, which is now genuinely attributable: `MESH_CONDUCTOR_LAUNCH=direct` gives each
   conductor its own `.sandbox_run_log.<peer>` instead of one multiplexed prefix-less stream, and
   `MESH_RUST_LOG` puts the three diagnostic modules at INFO so the lines exist at all.

**The staging-worked assertion (gate the measure on this):**

> A neighbour conductor holds **≥1 op whose dependency is absent from every peer.**

Until that holds, a QUIET verdict measures the staging and says nothing about the conductor. This is
the same discipline the detector already enforces on its log leg: a measure that cannot see must say
so rather than report a confident zero. Verify the assertion first; only then start the detector.

*Prerequisite, learned the hard way:* the mesh must be running the **fork conductor line**, and the
`hc` CLI must match the conductor's config schema — a 0.6.0 `hc` rewrites `conductor-config.yaml` in
its own schema and a 0.6.3 conductor then refuses to boot. See the Prologue addendum.

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

**RECONCILED 2026-10-02** (shem, fork 2b334df7973d, superproject 4a80267f3, code read only, nothing measured): spin cured in code; the unfetchable set is data, not determinable here. `sys_validation_workflow/missing_dep_backoff.rs:44-62`, `sys_validation_workflow.rs:615,817` (fork c08a851a7, not b9c7458ae). The saturation counter and throttled log line named above are not at this pin. Same mechanism as: none. Confirming measurement: conductor log `Sys validation sleeping … (K of T tracked dependencies are unfetchable and on a slow sweep)`; needs the fleet or the v3 staging to be non-trivial.

**MEASURED ON THE FLEET 2026-10-04** (alpha, pin `901b02607`, the conductor's own `hc_*` series, 60 min and 30 min windows ending 01:29Z, at least three hours after the last restart; hosted actors are fixtures). The backoff is in the pin (`c08a851a7` is an ancestor) and the gauges it reads now exist, but the workflow is not idle. `sys_validation_consumer` time per second of wall clock (`rate(hc_conductor_workflow_duration_sum[60m])`) is 1.00–1.02 on all seven conductors: about 7.5–8.2 runs a minute at a mean of 7.3–8.0 s each on matthew and jessica, so one run is always in progress. It is the largest workflow by time on every conductor except eve. On the one DNA that carries the corpus (`uhC0k5a385O0…`), `hc_conductor_sys_validation_missing_dependencies` reads gertrude 10,000, eve 8,734, james 2,055, jessica 963, matthew 810, adam 205, susan 1; `…_unfetchable_dependencies` reads james 608, gertrude 454, jessica 445, eve 23, matthew 12, adam 6, susan 1. The other four DNAs read 0 on every conductor. *What it does not show:* that this is CPU. The duration is wall clock; susan carries one missing dependency and still reads 1.00, at 0.5 cores, so a run's eight seconds may be mostly waiting. Gertrude's 10,000 is a cap, not a count: each run selects at most 10,000 ops pending sys validation (`sys_validation_workflow.rs:257`, `ops_pending_sys_validation(10_000)`), so gertrude's pending set is at least that large and every run re-reads the whole selection. Gertrude, eve and james are also the conductors held at their CPU limit (throttled in 93–98% of CFS periods), and the first two carry the largest missing sets; whether one causes the other is not established. Eve was integrating 311–622 ops a minute in these windows against 4 the evening before, so its figures are from a burst. *Measurement that would settle it:* a CPU profile of one conductor split by workflow, or the workflow's own counters for dependencies fetched per run against dependencies retried. Evidence (gitignored, on shem): `genesis/a2o/reports/recovery/shem-perf-deep-dive-20261002/fleet-hc-settled-901b02607/README.md` §2 and §8.
