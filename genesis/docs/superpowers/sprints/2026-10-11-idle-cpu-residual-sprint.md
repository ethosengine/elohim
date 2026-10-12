---
title: "Idle-CPU residual sprint 2026-10-11 — the reading the conductor cures never got, and one bounded cure chosen from it"
id: idle-cpu-residual-sprint-20261011
date: 2026-10-11
status: in-flight
author: "claude-fable-5-1 (orchestrator); readers claude-sonnet-5-5 and Explore; cure design claude-opus-5-5; at the operator's direction"
habits: [idle-is-free, zome-call-cost-bounded, runtime-performance]
cites:
  - genesis/data/timeline/backlog/performance-concern-index.md
  - genesis/data/timeline/backlog/conductor-residual-cpu-full-chain-read-and-perpetual-republish.md
  - genesis/data/timeline/backlog/alpha-conductor-sys-validation-spin-unfetchable-deps.md
  - "gradient-sprint-20261008 | the sprint whose Lane 4A (lock-deferral probe) this one closes for the current pods and whose Lane 4D (residual idle CPU) it takes up | sha256:4fec108f82d70b3d | path: genesis/docs/superpowers/sprints/2026-10-08-gradient-sprint.md"
  - "shem-performance-deep-dive-report-20261002 | the deep dive that named the per-read SQLCipher floor (F10) and the publish/validation loops this sprint re-reads with the latch on | sha256:df11f4c346d960e2 | path: genesis/docs/superpowers/sprints/2026-10-02-shem-performance-deep-dive-report.md"
  - elohim/elohim-storage/.epr-meta/idle-is-free.habit.md
---

# Idle-CPU residual sprint 2026-10-11

This is a plan. Results go to each habit's own atom as a DELTA line and to the backlog entry, never here.

## Why this item

The performance index's reading order starts with the trustful-self paths paying to re-verify. Its first entry,
`conductor-cap-grant-scan-per-zome-call`, was cured on the fleet (settled 2026-10-04, read from storage's side
2026-10-08). The next, `conductor-residual-cpu-full-chain-read-and-perpetual-republish`, is `cured-unverified`:
the publish-time lock cure rolled (c916eddbe, edge #1561) with its 90-minute window unclosed, Option E shipped and
moved publish time but not CPU, and "residual idle CPU on a patched conductor" is the open question the deep dive
left. The habit it serves, `idle-is-free`, is red with a runnable check that has not been run since 2026-09-20,
before every one of those cures. Stabilising this item means taking the reading; finishing it means naming the
residual and curing the bounded part.

## What was known before the household ran (read 2026-10-11, fleet, Prometheus and Loki)

- **Fleet pin:** all seven conductors on `c916eddbed02` since the 2026-10-09 restarts (image
  `elohim-storage:1.0.0-dev-fd7dd12e`, held byte-stable by edge #1588/#1590). No roll since.
- **Publish-lock probe (Lane 4A of the gradient sprint):** Loki holds nothing before 2026-10-08 22:06Z, so the
  original 2026-10-06 window cannot be read. On the current pods' first 90 minutes (adam from 11:01:10Z, matthew
  from 09:16:36Z on 2026-10-09) and adam's last 24 h: `could not record publish time` 0, `recording publish times
  failed for a chunk` 0, `write lock was not available` 0, `code: 778` 0; `database is locked` 2 (adam) and 7
  (matthew). The probe reads closed for the current pods; the deferral path itself has never fired on the fleet.
- **Residual CPU, 6 h, flat on every pod:** adam 0.77 of 4 cores, matthew 0.99 of 3, susan 0.53 of 2.25, jessica
  1.14 of 1.5 (53% of periods throttled), james 1.35 of 1.5 (76%), gertrude 2.23 of 2.25 (97%), eve 2.23 of 2.25
  (98%).
- **Largest workflow by wall time on every conductor:** `sys_validation_consumer` at 0.995–1.005 s per second,
  0.05–0.10 runs a second, so one run is always in flight at 10–20 s each. Second: `publish_dht_ops_consumer`
  at 0.17–0.48 s/s. No other workflow above 0.02 s/s. The sys-validation entry already names the mechanism:
  each run selects at most 10,000 ops pending sys validation and re-reads the selection
  (`sys_validation_workflow.rs:257`, `ops_pending_sys_validation(10_000)`; retry delay 10 s); wall time is not
  yet shown to be CPU.
- **Storage-side idle calls, 6 h:** `app_info` 0.217 calls/s on every pod (13 a minute, the ungated bridge probe:
  over the 6-a-minute budget by itself), `get_rea_commitment` 0.04–0.23 (eve 0.89 at one 30 m read),
  `resolve_content_head` 0.02–0.04, `my_consent_standing` 0.03, `record_peer_status` 0.017 (the 60 s heartbeat).
  Admission hold p50: adam 1,875 ms (outlier, unexplained), the rest 13–81 ms.
- **Fork loops that run with nobody acting** (code read at c916eddbe): the scheduler ticks every 100 ms and, for
  every running cell, runs `reschedule_expired_persisted` and `live_scheduled_functions` (two reads per cell
  per tick; the unconditional delete was already removed by an ELOHIM patch); WAL maintenance every 60 s per
  space; the publish trigger loop 60→300 s per cell until `has_ops_still_needing_publish` is false (never, on a
  mesh that cannot complete receipts); sys-validation retry every 10 s while dependencies are missing; app
  validation retry 100–3000 ms. No elohim zome calls `schedule()`, so every scheduler read on this fleet returns
  nothing.

## Lanes

### Lane M — the household reading (`idle-is-free`, the check as declared)

Household-dowell resumed in place (identities and the long chains preserved), `MESH_CONDUCTOR_LAUNCH=direct` so
each conductor serves its own `hc_*` series on :9464–:9466, `MESH_PORTAL=0`. Settle at least 20 minutes after
`just mesh wait`. Then, bracketing the same window: `genesis/local-dev/perf-idle-residual/metrics-window-diff.ts
snapshot` at t0, `MESH_ALLOW_NO_PROLOGUE=1 BERTH_CLASS=verify just test mesh
features/dataplane/household-at-rest.feature`, snapshot at t1, diff. Done-when the three budgets are read (calls
a minute per peer by `fn`, refused permits, household CPU s/min), and conductor time is attributed per workflow
(`hc_conductor_workflow_duration`), per database (`hc_db_connections_use_time`, `hc_db_write_txn_duration`) and
per wasm function (`hc_ribosome_zome_call_duration`) for the same 300 s. Evidence under
`genesis/a2o/reports/recovery/idle-residual-20261011/`. The lane's verdict is the DELTA; the budgets are the
target, not a reading.

### Lane D — cure design (Opus, read-only until Lane M names the bulk)

Two candidates, each with the smallest change, a test shape and the invariant it must not break:

- **D1 scheduler latch.** A per-cell "may have scheduled functions" flag: true at start (persisted schedules
  survive restarts), cleared when both tick reads return empty, set by the `schedule` host function for that
  cell. At idle on a fleet whose zomes never schedule, the 100 ms loop stops touching the database.
- **D2 sys-validation re-selection.** A run whose whole selection is already tracked as missing or unfetchable
  and whose back-offs have not expired should not re-read the capped 10,000 rows; push the back-off into the
  selection or hold a watermark. Correctness-sensitive: upstream's workflow.

One is built, not both: the one Lane M (household) and the fleet read together name as the larger idle share.
A fork change lands as a local fork pair (`/projects/.claude-config/tools/hc-fork-<sha>-proof`), the household
reading repeats on it as the after half, and the pin moves only on that pair. Reviewed by code-reviewer and
the orchestrator; the roll is the operator's.

**Lane D state (2026-10-12 01:20Z).** Lane M named the scheduler loop as the largest *nameable* share (≈65% of
matthew's DB checkouts, ≈20 of its 63.7 CPU s/min; sys-validation 0.002 s/s on the household), so D1 was built.
Opus implemented it on fork branch `perf/scheduler-latch` (worktree `.worktrees/hc-lock-deferral`), commit
`bef264f46` on `c916eddbe`: `ScheduleHint` (per-author latch + generation counter) on `DhtStore`, `touch` after
the source-chain flush commit and in `upsert_scheduled_function`, `peek`/`try_clear` in
`Cell::dispatch_scheduled_fns` with one new `SELECT EXISTS … WHERE author = ?` probe before a clear. Tests:
holochain_state 210 pass (3 new), schedule integration 13 pass (2 new), clippy clean. Sonnet review (read-only,
adversarial on the race): no ordering leaves a committed row behind a cleared latch; `reschedule_expired_persisted`
only updates existing rows; one `DhtStore` per DNA, hint keyed by agent. Verdict SAFE TO MEASURE; one hardening
suggested (also `touch` before the commit, against a dropped flush future). Codex, second independent review
(`codex-review-bef264f46.md`, ~01:40Z): FIX FIRST — a future dropped between a durable commit and the post-commit
`touch` hides a committed row behind a cleared latch until restart, and a pre-commit touch alone does not close
it (reader peeks the new generation, sees zero rows, clears, then the commit lands); `reschedule_expired_persisted`
is a third unhooked writer. Accepted cure: an in-flight write guard (RAII; held across the write and the commit,
blocks clears while held, bumps the generation on drop, so cancellation is covered), on all three writers. Opus
built it as `c8c296c56` (`perf/scheduler-latch-v2` on bef264f46, worktree `.worktrees/hc-latch-v2`; +244/−28 in
four files; holochain_state 216 pass incl. 6 new, schedule integration 14 pass incl. 1 new, clippy clean on
holochain_state + holochain_data + holochain). `perf/scheduler-latch` was fast-forwarded onto it. The release
pair for bef264f46 (`hc-fork-bef264f46bee-proof`, 34m56s cold) came out first, and because the guard changes
only the write path (no elohim zome calls `schedule`; the measured tick path is byte-for-byte the same
decision), the after-half reading ran on that pair rather than idle an hour; Codex re-reviewed the guarded
commit (`codex-review-c8c296c56.md`, ~02:15Z) and found the residual: sqlx-sqlite runs queued statements on a
worker thread, so a dropped future's already-queued COMMIT still lands AFTER the guard's Drop (sqlx-sqlite
0.9.0 `connection/worker.rs` ~264-282); ordering guard → COMMIT queued → drop → peek → zero → clear → commit
hides a row. Cure accepted: `complete()` on the guard after the awaited Ok, and a Drop without `complete()` sets
a per-author `clear_not_before` quarantine longer than any queued statement can wait (derived from the SQLite
busy timeout, floor 30 s). Opus added it as `fbdf961df` (third commit on
`perf/scheduler-latch-v2`; `UNACKED_WRITE_QUARANTINE` = 60 s = 2 × max(BUSY_TIMEOUT 15 s, pool acquire 30 s);
holochain_state 220 pass, schedule integration 14 pass, clippy and fmt clean). `perf/scheduler-latch` was
fast-forwarded onto it; it is the pin candidate, re-reviewed by Codex (`codex-review-fbdf961df.md`) and
release-built after the reading for a confirmation window. Codex round three (`codex-review-fbdf961df.md`,
~02:45Z): FIX FIRST again, two findings — (1) the reader may peek and probe INSIDE the quarantine and call
`try_clear` after it expires, with the queued commit landing in between (gen unchanged, quarantine over → clear
succeeds); cure: `peek` returns a token carrying its `Instant`, and `try_clear` refuses a token taken before
`clear_not_before`; (2) 60 s is a practical bound, not a derived one (sqlx-sqlite's serial worker has no
configured maximum from queue to execution; a WAL checkpoint or long prior statement can hold it), so the
quarantine is raised to 600 s and documented as practical. Opus added both as `501b5bf71` (`PeekToken { gen, at }`
taken under the map lock; `try_clear` refuses a token taken before `clear_not_before`; 600 s; `checked_add` with a
`quarantined_forever` flag on overflow; holochain_state 222 pass; one flake seen once in `schedule_same_agent`, a
`*/3` wall-clock cron, green on three reruns and not attributed), and the key finding below as `96bcd2bee`
(`log_slow_statements(LevelFilter::Debug, 1 s)` in holochain_data's connect options; holochain_data 201 pass). Codex
round four (`codex-review-96bcd2bee.md`, ~03:00Z): **SAFE TO MEASURE** — every ordering with a commit inside the
quarantine is refused (in-flight count, generation, token timestamp, or the committed row is visible to the
observations); the key now prints only at DEBUG or through `Debug`-formatting `SqliteConnectOptions` (no
establishment path does); test gaps named (no test cancels a real queued COMMIT). The residual by design is a
worker stalled longer than ten minutes with a queued COMMIT, after which a row can sit behind a cleared latch until
the next write or restart. Nothing in this fleet schedules, so the exposure is upstream's. Final branch:
`perf/scheduler-latch` = `96bcd2bee` (five commits on c916eddbe), release pair `hc-fork-96bcd2bee…-proof`. D2 not built: the fleet's
sys-validation spin does not reproduce on the household, and the selection re-read found a likely starvation
(`sys_validation_attempts` is never incremented; oldest-first at the 10,000 cap) that needs pending counts
before any cure is chosen.

### Lane R — register

DELTAs: `idle-is-free` (the reading, before and if reached after), the residual-cpu entry (4A read closed for
the current pods; sys-validation named as the largest wall-time workflow; the scheduler loop named), the
sys-validation entry (fleet 2026-10-11 figures), the index snapshot line. `habits-project.py` re-run; `just
gate` for each touched tree; land on `dev`.

## What the statement window named (2026-10-12 02:41:39–02:42:39Z, latch binary, settled 5 min after restart)

`genesis/local-dev/perf-idle-residual/run-stmt-window.sh` (restart with `RUST_LOG=warn,sqlx::query=debug`,
which sqlx-core 0.9.0 honours without a rebuild) and `stmt-log-summary.ts` over the three conductors'
`.sandbox_run_log.*` regions, keys redacted (`stmt-bef264f46-*.log`, `stmt-summary-bef264f46.md`): 840 / 937 /
820 statements a second on james (5 cells) / jessica (10) / matthew (100); DB-busy 0.30 / 0.31 / 0.27 of a core.
The same four shapes on all three, in the same proportions — cell count does not enter:

| share of DB time | share of statements | rate | statement | who issues it (Explore trace, read-only) |
|---:|---:|---:|---|---|
| 72–79% | 0.3% | ~2.3/s at 86–105 ms | `SELECT op_hash … FROM (SELECT ChainOp.hash …)` — `op_ids_since_time_batch_sql` (`holochain_data/src/dht/inner/sync_queries.rs:173-237`), NOT the publish sweep (the analyzer's keyword class and my first reading had it wrong) | kitsune2 gossip `retrieve_new_op_ids` (`kitsune2_gossip-0.5.0/src/respond.rs:315-343`) via `HolochainOpStore::retrieve_op_ids_bounded` (`holochain_p2p/src/op_store.rs:288-335`), once per side per round. Full arc makes the predicate walk the whole `elohim_ChainOp_loc_sync_idx` (`when_integrated` is its LAST column) and sort, whatever `new_since` is. Two fork bugs ride on it: the page loop stops only when a page adds nothing and re-reads the cursor row (`>=`), so every call with data costs two statements; and when the limit is not hit the fork returns the last op's `when_integrated` instead of the call's start time (kitsune2_api `op_store.rs:134-135`), so the peer's bookmark sticks and every round re-sends the last op id |
| 8–11% | 88–92% | ~750–830/s at 33 µs | `SELECT slice_index, hash FROM SliceHash WHERE arc_start = ? AND arc_end = ?` | `HashPartition::disc_top_hash` (`kitsune2_dht-0.5.0-dev.6/src/hash.rs:266-286`) reads one sector at a time; 2³²/2²³ = 512 sectors per minimal snapshot, a snapshot per side per round, nothing cached; elohim-storage's health `dump_network_metrics` adds 512 per space per call |
| 4–8% | 0.1–0.2% | ~1.3/s at 12–15 ms | `SELECT hash, basis_hash FROM LimboChainOp WHERE hash IN (?) UNION …` — `check_op_hashes_present` (`sync_queries.rs:254-301`) | `CoreFetch::request_ops` (`kitsune2_core-0.5.0/src/factories/core_fetch.rs:172`), once per round in which the peer sent op ids — nearly every round because of the stuck bookmark; 12–15 ms is too slow for primary-key `IN` lookups and is unexplained (EXPLAIN QUERY PLAN owed) |
| ~1–3% | 6–8% | 57/s reads, 7/s writes | `peer_meta` get/put | kitsune2 gossip peer-meta: `select_responsive_and_least_recently_gossiped` costs two reads per candidate agent per initiation (`initiate.rs:255-290`), plus the bookmark, timestamp and round counters — ~30 candidate agents per space implied |

Every one of these is a gossip ROUND cost, and the household fires rounds every 1 s on purpose:
`app/elohim-app/scripts/hc-mesh.sh:2638-2650` patches `k2Gossip.initiateIntervalMs / minInitiateIntervalMs /
initialInitiateIntervalMs = 1000` into every sandbox at start ("dev-tier gossip acceleration, declared
preproduction stakes", `:5014-5030`), against upstream defaults of 120 000 / 300 000 ms
(`kitsune2_gossip-0.5.0/src/config.rs:145-151`). The fleet template sets only `roundTimeoutMs` and
`maxConcurrentAcceptedRounds` (`genesis/orchestrator/manifests/humans/_edgenode-conductor.template.yaml:136-139`;
adam 15 000 / 10), so alpha runs the default cadence — roughly a hundred times fewer rounds. Three consequences:
(1) the household's cell-independent base is the dev profile's, not alpha's; alpha's residual (sys-validation
1.0 s/s, publish 0.2–0.5 s/s) does not reproduce here and this base does not reproduce there; (2) `idle-is-free`
AS DECLARED cannot meet a 10 CPU s/min household budget while that acceleration is on — the check is confounded
by the fixture, and the register must say so; (3) the per-round cost is real and upstream-worthy anyway (the
bookmark contract violation, the double page, the unindexed `when_integrated`, 512 uncached slice reads) —
alpha pays it rarely today, but it scales with database size (the new-ops scan is a full index walk) and with
hosted-agent count (`peer_meta` reads per candidate), which is adam's shape. A second household reading at
production cadence (`gossip-cadence.sh prod`, same latch binary) is the lane that reads what the latch leaves
when gossip is not artificially hot. First attempt (restart 02:53Z, settle 20 min, lane 03:16Z): the PRECONDITION
step refused — "the household is not at rest yet — jessica: pull not caught up" — so the budget step never ran:
at production cadence the household takes longer than the fixture's documented ≈20 min restart churn to reach
"at rest" (that figure was measured under one-second gossip). The 10 s window the aborted lane did bracket read
matthew 6.8, jessica 6.5, james 6.6 CPU s/min, household 19.9 — against 147 at one-second cadence, and with the
100-cell conductor costing the same as the 5-cell one. The lane was re-run at 03:18Z after 40 min of settle
(`lane-after-bef264f46-prodgossip2.log`): precondition passed, budget step FAILED — household CPU 65.8 s/min (per
process 21.3 / 22.2 / 20.5), DB checkouts 214–300/s per conductor (from ~1,000), refused permits 0, calls a minute
matthew 502, jessica 569, james 810, almost all `get_rea_commitment` Background (×2,386 / ×2,742 / ×3,946 in 300 s).
First read as the trigger courier; CORRECTED at ~05:00Z by an Opus read of the storage metrics
snapshots: it is the REA heal leg's miss re-admission — calls = 3 × `miss_readmissions_total{rung="2"}` to within
one, a ≈1,316-id stale cohort lapsing out of dormancy together and re-read three times per id (the quiet
`to_read 0` summary I quoted sat between bursts); the courier is bounded to ≤28 reads per 60 s claim and accounted
for ≤1 call. Levers: one read per re-admission (3× fewer, permanently), a deterministic +0–50% spread of the
dormancy window (flattens the burst); neither reaches the 6/min budget while ≈1,300 ids stay stale, and that stale
set (10× the 2026-09-20 count) is a gossip-convergence question at production cadence. So at production cadence
the three conductors' own idle cost is ≈20 CPU s/min (twice the budget, with 115 cells), the heal leg's
re-admission burst is the top caller, and at the dev cadence the budget is unreachable by construction. The `idle-is-free` DELTA 2026-10-12b carries
the three readings. The household was restored to its declared dev gossip profile afterwards and
restarted on the final pair `hc-fork-96bcd2bee215-proof` (03:25Z); a metrics-only 300 s window after 10 min
(`smoke-96bcd2bee.log`, `diff-smoke-96bcd2bee-*.md`): checkouts matthew 1,093 / jessica 1,055 / james 1,128 a
second (the latch holds on the final commit; dev cadence, so CPU is the gossip figure, 154 s/min), and zero
`PRAGMA key` lines after the restart — with zero slow-statement lines of any kind, so that path was not exercised
live; its evidence is the unit test and Codex's read of sqlx's establish path. The household is left running the
candidate pair at its declared dev profile; `just mesh status` names the pin as NEXT LAUNCH.

Sprint status: **stabilized, not finished.** The chosen item's cell-proportional share is cured on a reviewed fork
branch awaiting the operator's roll; the fleet's own residual (sys-validation spin) is re-read and its selection
defect verified from source but not cured; the household check is shown to be confounded by its fixture and read
once without the confounder; one security defect was found and cured on the same branch.

## Shift 2026-10-12T04-10 — the pin on the pipeline overnight

Operator's call at ~03:50Z: push rather than keep measuring locally ("you find it you fix it… commit and push
/shift on the pipeline overnight"; pin-first framing confirmed). Shift `roll-scheduler-latch-pin-edge-green`,
bring-up mode (edge red since Oct 9), 9 iterations, 04:10–~11:00Z; journal gitignored under `.claude/shifts/`.

- **Delivery leg — DONE on the fleet.** Fork branch `perf/scheduler-latch` pushed to ethosengine/holochain;
  gitlink moved `c916eddbe → 96bcd2bee` (`5e4cab277`); orchestrator #2038 built `elohim-edgenode/main` #44
  (`HC_REF=96bcd2bee215…`, 20.9 min) and ran edge #1608 as a real deploy (105 min, SUCCESS): storage image FROM
  the new conductor image, seven conductor StatefulSets rolled on the pin-moved rule (`conductor fork pin moved
  (conductor-c916eddbed02 -> conductor-96bcd2bee215)` ×7; roll gate: six RELEASED, jessica DEADLINE/continued).
  The measure (last completed deploy run green and naming the pin) read 2 from a fresh trigger. Alpha runs the
  scheduler latch and no longer prints the SQLCipher key at WARN.
- **Bring-up leg — the red was the harness, fixed twice, proven live once.** Every edge red since #1591 was a
  validate-only run turning 14 born-red undefined scenarios into FAILURE. `run-dataplane-validation.sh` now
  downgrades ONLY the pending-only shape (raw cucumber statuses ⊆ {undefined, pending}, every undefined
  scenario under a concern declared in `scripts/ci/dataplane-born-red-concerns.txt`, fresh evidence files,
  console witnesses) and prints `PENDING-ONLY refused: <predicate> — <item>` otherwise (`2059054b5`,
  `70ed2957f`; Sonnet ×3, Codex ×3; the first fleet run #1610 refused silently on cucumber's benign "Switching to
  'progress' formatter" notice — fixed). #1612 and #1613 proved the refusing direction live (one real failed scenario — `inventory-convergence`,
  `p2p.caughtUp` false on storage-B then storage-A — named beside the 14 undefined). The green direction
  (banner, exit 0) is unobserved live at close because the fleet has that real red; it shows on the first
  validate-only run after the storage peers catch up.
- **Complementary leg — the heal-leg re-admission burst, cured in code.** `4d9cde774`: one read per
  re-admission spent at execution, declared-exception replays, 8-sweep re-offer cap, rung ≥2 spread; four Opus
  passes, four Codex rounds; `cargo test --lib --bins` ALL CLEAR twice; household receipt
  `epr-app-deliverability` 5/5 on the rebuilt storage (the pre-push NO-SERVING-RECEIPT leg demanded it); landed
  validate-only, carried by the next deploy. Residual named in idle-is-free DELTA 2026-10-12c.
- **Found on the fleet, captured:** storage `caughtUp` flapped for ~4 h after the roll (quiesce no-measures,
  `inventory-convergence` red) — appended to `ci-edge-inventory-convergence-caughtup-sawtooth-flap`; a
  `HUSKY=0` push bypass was denied by policy and not retried (the gate's wait was the right answer).

## Boundaries

- No budget is raised. Options A and D for publish (retiring ops with incomplete receipts) stay unapproved.
- Codex was requested as a tier and is installed (0.159.3); it was not logged in when the sprint opened, so the
  first review leg ran on Sonnet. The operator logged it in at ~01:35Z 2026-10-12; from then it carries two
  read-only legs: a second, independent review of the fork commit and a verification of the sys-validation
  selection claims (D2). Receipts: `codex-review-bef264f46.md`, `codex-d2-sysval-selection.md` under the
  evidence dir.
- Status flips only on evidence; a household green does not flip the fleet.
