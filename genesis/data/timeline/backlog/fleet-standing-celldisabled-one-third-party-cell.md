---
id: "backlog-fleet-standing-celldisabled-one-third-party-cell"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Fleet cells answer CellDisabled for hours after every conductor restart (first recovery ~108 min) and eve's own lamad cell has been disabled since 2026-09-18 — reads serve, writes 503, and this IS what fails app delivery"
slug: "fleet-standing-celldisabled-one-third-party-cell"
written: "2026-09-21"
author: "serving-edge failover-balance-stream campaign, 2026-09-21 pre-push fleet read"
status: "open"
priority: "high"
jobs: [elohim-edge]
tags: [fleet, conductor, cell-disabled, projection-reconcile, feedback-projector, alpha]
---

**DELTA 2026-09-22 (ci-wallclock lane): root cause of the hours-long post-restart window found; fix written and
unit-tested in the fork, NOT deployed. Status stays open until a conductor image carrying it rolls and the per-pod
recovery times are re-read.**

*Chain:* `Conductor::initialize_conductor` → per enabled app, `create_cells_and_startup`
(`crates/holochain/src/conductor/conductor.rs:1923`) awaits every cell's network `join` (`:1959`) and only then
adds the cells to `running_cells` → the first join of a DNA creates the kitsune2 space
(`crates/holochain_p2p/src/spawn/actor.rs:1699`) → `K2Gossip::create` (kitsune2_gossip 0.5.0 `gossip.rs:145`)
awaits `Dht::try_from_store` (kitsune2_dht 0.5.0-dev.6 `dht.rs:537`) → 512 sectors × `TimePartition::try_from_store`
(`hash.rs:107`, `time.rs:156`): per sector two `earliest_timestamp_in_arc` reads plus one
`retrieve_op_hashes_in_time_slice` per partial slice (~16; partial slices are never persisted, so every restart
recomputes all of them). About 9–10k SQL reads per DNA before any cell of the first app runs.

*Why each read is a full scan:* the 0.7 `holochain_data` DHT schema
(`migrations/dht/20260422120000_initial_schema.up.sql`) has **no secondary index at all**. 0.6's `DhtOp` had
`storage_center_loc` and `authored_timestamp` indexes. The arc filter in `dht/inner/sync_queries.rs`
(`:97-101`, `:180-184`, `:848-852` at pin `25dd2d0be`) is
`(?s<=?e AND loc>=?s AND loc<=?e) OR (?s>?e AND (loc<=?e OR loc>=?s))`, and its guard is only known at bind time.
So SQLite cannot use a loc index even if one exists. Measured with the exact pinned SQL: `SCAN ChainOp` with and
without an index. Cost is (reads × ChainOp rows), which is why it grows with each peer's store and runs from 56 min
(matthew) to 6 h (eve). The same reads back the 15-minute DHT update task and gossip ring diffs. That is a likely
(unmeasured) contributor to the week-long full-arc CPU peg in
`fleet-full-arc-conductor-saturation-and-coordinated-warmup-2026-09-11.md`.

*Fix, on local fork branch `perf/k2-dht-model-sargable-arc` (commit `e0bfc6c7a`, local only — not pushed, gitlink not moved; off pin `25dd2d0be`; `holochain_data`
only):* (1) Rust picks the arc shape: a plain range for non-wrapping arcs (every sector, and FULL), and the
two-sided OR for wrapping arcs. The rows selected are the same. (2) A covering index
`elohim_ChainOp_loc_sync_idx` and `elohim_WarrantOp_loc_idx`, created with `CREATE INDEX IF NOT EXISTS` at open.
This is deliberately not a sqlx migration: an unknown applied migration makes an older binary refuse the DB
(`VersionMissing`), and that would make a rollback an outage. Tests: 3 new unit tests (the plan uses the index and
never `SCAN ChainOp`; the rows match across non-wrap, wrap, FULL and empty arcs; the index is idempotent and absent
from `_sqlx_migrations`). `cargo test -p holochain_data` 146+8 pass, clippy `-D warnings` and fmt clean.
`holochain_p2p --test integration op_store` 10/10 pass.

*Measured (synthetic store, Python sqlite 3.46, exact pinned vs patched SQL, the startup read pattern for 512
sectors):* 300k ops 501 s → 23.5 s. 1.2M ops ~2632 s → 131 s (index build 3 s, one time). That is about 20×.
Fleet disks and encryption were not reproduced. On the fleet's per-peer scale that projects to roughly 3–20 min, not
56 min–6 h. The patched SQL still makes one `Action` primary-key probe per sector row per read.

*Next step:* bump the fork with this branch → `[build:conductor]` → pin move → one staggered roll, recording
`conductor app is RUNNING again` minus restart per pod. The structural follow-ups are separate, and each is a kitsune2
or holochain design change, not a patch. (a) Do not hold `running_cells` hostage to `join`: let cells run while
the space's DHT model builds in the background (gossip refuses rounds until it is ready). (b) Cut the ~16 partial
reads per sector to one. This needs the OpStore contract to return per-op timestamps, or `ChainOp` to carry the
authored timestamp so a `(loc, ts)` index serves the slice directly. That is a schema move, so it needs a rollback
story. Both belong to rung 3 (conductor-roll cost) of `upgrade-propagation-p2p-design-arc.md`.

---

**CORRECTED 2026-09-21 evening, after the first roll this item's own batch went through (edge #1472). Three
claims below were wrong; they are left in place and corrected here, because the wrong version already steered a
decision.**

- *"A third-party cell."* `uhCAkhsVVjkuw1D8…` is **eve's own agent** (`genesis bootstrap identity heal …
  human-eve-firstwoman`), and it appears on no storage pod but eve's. The four pods' sweeps were retrying against
  eve's cell, which makes it third-party to *them* — but the standing fact is "eve's own lamad cell has been
  disabled since 2026-09-18", not an unknown agent's.
- *"A holder's own cell heals 0–4 minutes after `Conductor ready.`"* That was one roll's reading of one role. Per
  role and per pod the window is far longer and it is NOT new: the `infrastructure` role answers own-agent
  `CellDisabled` at exactly the heartbeat rate (1/min, `record_peer_status`) in multi-hour episodes on every storage
  pod on 09-18, 09-19, 09-20 (susan: 8.5 h continuous on the OLD code) and 09-21 before the roll. After edge
  #1472 every pod entered it within minutes of its OWN conductor restart (sequential, 16:21Z→17:24Z); the first
  recovery in the fleet was susan/infrastructure at 18:09:47Z, **~108 minutes** after her conductor restarted.
  Conductor logs show no `enable_app`/`disable_app`/`update_coordinators`/reinstall on either roll, and the batch
  changed nothing under `elohim/holochain` — the condition pre-dates it and was invisible because the old error
  classifier counted a disabled cell as proof the path was live (82d06d902).
- *"Serving is unaffected, so medium."* Reads are unaffected (the public landing and `/lamad/` were 200 on every
  2-minute sample through the whole roll). WRITES are not: every write on a role in this state answers
  `503 {"error": …CellDisabled…, "cause": "conductor-app-disabled"}`, which is what has failed every app delivery
  since #1707. Priority raised to high.

**What would settle it.** Whether the cells converge by themselves in restart order (jessica, james, gertrude, eve,
adam, matthew after susan) or need a second conductor restart — read
`{namespace="elohim-alpha"} |= "conductor app is RUNNING again"` and `elohim_conductor_app_enabled` per pod/role.
If they converge: the cost driver to chase is the conductor's cold start (each conductor crash-loops once on
`AddrInUse`, then rebuilds the `CompiledWasm` cache) and the 10-minute sequential spacing of the conductor roll.
If they do not: `enable_app` is accepted and ineffective from storage (ladder at attempt 6+, "waiting on the
conductor"), so the only lever is operator-owned — restart the conductor pods, not storage.

**Also found:** `not_running_secs: 98` on susan's recovery line understates a ~108-minute episode — the episode
clock resets on repeated observations, so the field misleads exactly the triage it exists for.

---

**The fact (as first written, 2026-09-21 morning).** Two different things on the alpha fleet answer `CellDisabled`, and a raw grep conflates them.

1. *A holder's OWN cell during its own restart.* Bounded and self-healing: over three restarts in 72 h, matthew's
   and adam's own-cell `CellDisabled` lines stop within 0–4 minutes of that conductor's `Conductor ready.` line
   (matthew ready 2026-09-20T12:49:32Z, adam 13:11:40Z; one later matthew reconnect took ~35 min), and neither has
   logged one in 13–19 h since. This is the class that failed app builds #1709, #1712 and #1714 — they start
   seconds after an edge roll and author through that window. Cured on the CI side by the readiness wait in
   `scripts/ci/stage-spa-blob.sh`; not this item.
2. *One third-party cell, standing.* `CellDisabled(CellId(DnaHash(uhC0kZezl4k2nZa5ZyU5O5H-5vH5LYpvwkSwkVx4G1wS_sHB4GTOt),
   AgentPubKey(uhCAkhsVVjkuw1D8zGPU0uRV3zuPIR-HIUX5J2CubFyAQikOt--Ks)))` — the lamad DNA, an agent that is neither
   matthew (`uhCAkJH75…`) nor adam (`uhCAkCUzKl3…`). Present across the whole 72 h scan, identical before and after
   every pod restart, from at least 2026-09-18T13:57Z through 2026-09-21T07:54Z. **This item.**

**Evidence.** Loki, namespace `elohim-alpha`, 2026-09-21T07:54Z, last 60 min: `elohim-eve-alpha-0` 133 lines
(`feedback_projector`: "discovery enumeration failed — the member yields its turn and is retried"),
`elohim-gertrude-alpha-0` 60 lines (~1/min, heartbeat cadence), `elohim-doorway-alpha-…` 89 and
`elohim-doorway-alpha-b-…` 99 lines; zero on matthew and adam over 6 h (772 453 lines scanned). On the storage
side the caller is `elohim_storage::p2p::projection_reconcile` ("conductor get failed; retry next sweep") while
reconciling `provide-<matthew|adam>-content:commons` commitments — the sweeps depend on the disabled cell, they are
not the disabled cell. Fleet serving is unaffected: both doorways' `/health/serving` are 200 with
`storageServing: serving`, and matthew and adam publish ops normally (23 637/23 637 and 46 625/46 625 at 07:51Z).

**Why it matters.** It is the "fleet CellDisabled since 2026-09-18" that earlier notes recorded as one standing
red, and reading it that way sent the app-delivery diagnosis the wrong way for three days (be16ffbc1 declared
every CellDisabled structural). It also costs four pods a failed conductor round-trip per sweep, forever, and it
will keep every CellDisabled alert ambiguous until it is named.

**Not determined.** Whose cell it is and on which conductor it is installed (Loki cannot say; the conductor's
admin app listing can), and whether its app is genuinely disabled with a reason or is a hosted cell whose
conductor never finished initialising it. Storage's per-role health probe (e7858cc02) observes a holder's own
roles, not a third party's cell, so it will not surface this one.

**Smallest next step.** Read the owning conductor's app listing for that agent key (operator-owned: admin
interface, not kubectl from the dev environment) — `disabled` with a reason names the cure; `enabled` with a dead
cell is a conductor defect worth a specimen. Independently, `projection_reconcile` and `feedback_projector` should
stop paying a conductor round-trip per sweep for a dependency that has answered the same way for days: back off
per cell, and count it in a metric so the standing case is visible without a log grep.

**Links.** `scripts/ci/stage-spa-blob.sh` (the bounded class's CI cure). `elohim/elohim-storage/src/p2p/projection_reconcile.rs`
and the feedback projector (the retrying callers). Evidence run: `genesis/a2o/reports/recovery/serving-edge-20260921/`.
Sibling: `doorway-registry-ttl-unenforced-no-heartbeat.md`.
