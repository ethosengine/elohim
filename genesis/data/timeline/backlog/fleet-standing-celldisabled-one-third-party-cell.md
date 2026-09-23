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

**SETTLED 2026-09-22 02:50Z (conductor logs + the fork's source).** Two mechanisms, both now named.
(1) *Startup is the window, and it is hours.* A cell is absent from the conductor's in-memory `running_cells` until
startup finishes `create_cells_and_startup` for its app, and startup blocks inside the kitsune space join on
`Dht::try_from_store` — a sequential walk of 512 sectors over the peer's persistent op store. On 2026-09-21 each
conductor logged its `DHT model initialised in …` lines and `Conductor startup: apps enabled.` 0–20 s after the
slowest one: matthew 56 min, susan 1 h 47, james 1 h 34, jessica 2 h 09, adam 3 h 29, gertrude 5 h 51, eve 6 h 09
after its restart. Interfaces open first, so every call in between answers `CellDisabled`; `enable_app` on an
already-Enabled app short-circuits without touching cells (conductor.rs ~2043), so storage's enable ladder cannot
shorten it. The window scales with stored history, which is why the household (fresh cast) sees ~11 min.
(2) *Stranded after startup — cause still open.* gertrude, eve and adam (infrastructure) and james (lamad) kept
answering `CellDisabled` for hours AFTER their conductor's `apps enabled`, with zero disable/teardown lines in any
conductor log (`DISABLING the following apps` / `Cell cleaned up and removed`: 0 on all 7). A conductor restart is
therefore NOT a reliable cure for this class. The decisive reading is the conductor's own running-cell map
(`ListCellIds` / `DumpConductorState`) against the failing `(DNA, agent)` — an admin-interface read storage can
make and the fleet's `/db/p2p/conductor-diagnostics` (503, PROBE-BROKEN) should carry.
Also found: a second, coordinated restart of the three conductors on node `ethosengine` (jessica, james, matthew)
at ~19:53Z — a node event, not a deploy; james's lamad cell went down with it and never came back. And the
`AddrInUse` crash-loop on every restart is the app-interface port 4445 still held by the predecessor while storage
attaches too early.
**Cure order (Astra, source-verified):** storage observes actual cell membership via `ListCellIds` and stops
spending `enable_app` on Enabled apps (household-verifiable) → a scoped disable→enable only for a confirmed
stranded app → the conductor's `enable_app` reconciles missing cells → startup/attachment ordering. Restarting
the conductor retries construction and clears a stuck attempt, but repeats the hours-long load.

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

**2026-09-23 — mechanism (1) has a fix on a buildable branch; the fleet has not run it.** The fork's
`perf/k2-dht-model-sargable-arc` (`e0bfc6c7a`: ArcBounds chooses a sargable predicate shape in
Rust, covering `elohim_ChainOp_loc_sync_idx` / `elohim_WarrantOp_loc_idx` created `IF NOT EXISTS`
after migrations so an older binary still opens the store) is now `8ccffdedb` on
`int/2026-09-23-diagnostics-throttle-perf`, with the cap-grant fix and the private diagnostics.
Its three tests pass in the fork (`sector_arc_reads_use_loc_index_not_chain_op_scan`,
`arc_shapes_select_the_same_rows`, `sync_indexes_are_created_idempotently_outside_migrations`).
Household (36 GB Dowell fixture, three conductors, `hc-fork-61565f320d0e`): processes started
18:43:57Z; storage first read every role's cell ABSENT at 18:44:25Z; matthew's first successful
zome call landed at about 18:46:24Z (its `/health` at 18:48:30Z showed `lastZomeCallAgeSecs` 126
on node_registry) and all fifteen role cells on the three peers were serving by 18:49Z — a
window of roughly two and a half minutes on this store. No same-store baseline with the old
binary exists (the household's conductors log at `warn`, so the `DHT model initialised in` line is
not captured here), so this is "boots and serves", not a controlled before/after; the fleet's
per-pod `DHT model initialised in` lines (Loki, `container="elohim-conductor"`) are the measurement
that closes this row once a conductor image carrying `61565f320` rolls. Today's fleet reading for
scale: gertrude 24 176 s, eve 15 138 s, susan 10 078 s, each a single space join.
Mechanism (2), stranded after startup, is untouched by this.
