---
id: shem-performance-deep-dive-report-20261002
status: draft
date: 2026-10-02
author: "claude-opus-5-5 (shem workspace, reconcile session), from the measuring session's evidence and its own code reading"
habits: [zome-call-cost-bounded, runtime-performance, idle-is-free, dataplane-convergence]
cites:
  - "performance-deep-dive-shem-handoff-20261002 | 2026-10-02-performance-deep-dive-shem-handoff | sha256:a51158f038519bad | path: genesis/docs/superpowers/sprints/2026-10-02-performance-deep-dive-shem-handoff.md"
  - genesis/data/timeline/backlog/performance-concern-index.md
  - genesis/data/timeline/backlog/conductor-cap-grant-scan-per-zome-call.md
  - genesis/data/timeline/backlog/conductor-residual-cpu-full-chain-read-and-perpetual-republish.md
  - genesis/data/timeline/backlog/arch-dataplane-borrows-backlog.md
  - genesis/data/timeline/backlog/head-authority-carried-with-content-sync-unit.md
---

# Shem performance deep dive: report

**Draft.** The before/after of the patched conductor (no-op, writes and idle CPU), the
receipts-per-op measurements, option E's before/after and the household serving receipt are in
(findings 1, 2, 4 and 5). Still owed: any figure at 45,000 actions or on the fleet, and the cause of
the one serving miss (finding 4). No habit status moves on this page.

This answers the handoff `2026-10-02-performance-deep-dive-shem-handoff.md` ("What to send back").
Two shem sessions did the work by operator direction: a measuring session that owns the mesh, the
cargo lane and a local fork branch, and a reconcile session that read code and wrote the backlog
side. All actors on the mesh were test fixtures.

## Identity of what was measured

| | |
|---|---|
| Superproject | `4a80267f3` (handoff ref = `dev` `dd1b55ac2` plus the handoff pages), branch `shem/performance-deep-dive` |
| Conductor | fork `2b334df7973d05eab4d72a75006275bcec770321`, production features (`encryption,wasmer-sys-cranelift,jemalloc`) plus frame pointers and line tables; receipt `/projects/.claude-config/tools/hc-fork-2b334df7973d/BUILD-RECEIPT.txt`, holochain sha256 `7bf139d1…` |
| hApp | built and packed from this ref at 20:10:58Z; `elohim.happ` sha256 `2b291342…`, `content_store.wasm` `8f506c92…`, lamad DNA `uhC0kQE2sPmD7EuxoNLzDVVR9l0rGLnaS2nZhKJcc4rzJa9LV8lwZ` (`genesis/local-dev/perf-deep-dive/happ-identity.txt` on shem) |
| Storage, doorway | built from `4a80267f3` (pool `mesh-bin` and `dev/debug` slots) |
| Mesh | disposable three-peer mesh `genesis/local-dev/perf-deep-dive/mesh-a`, three doorways, no prologue, no seeded content, no hosted cast |
| Machine | shem workspace, 2× Xeon E5-2667 (24 threads, no AVX2), ZFS-backed volume |
| Fleet | alpha runs `dd1b55ac2` with conductor `conductor-2b334df7973d` (edge #1523 SUCCESS, all seven statefulsets configured); #1524 dataplane validation `DID NOT MEASURE` (fleet still churning) |
| Workload | `genesis/a2o/scripts/chain-cost-probe.ts`: grows matthew's lamad chain with `content_store::issue_attestation` (3 actions per write) and times `export_schema_version` (no host call) signed by a signing credential, as storage and doorway call |

Evidence lives under `genesis/a2o/reports/recovery/shem-perf-deep-dive-20261002/` and
`genesis/local-dev/perf-deep-dive/` on shem. Both are gitignored, so the figures below are copied in.
The measuring session's summary there is `FINDINGS.md`.

## The answer to the handoff's first question, in one paragraph

At a long source chain, a trivial zome call is slow because the conductor walks the caller's whole
chain several times per call before the zome function runs. The capability-grant lookup and the
init check each scan every action the author ever wrote through the `(author, seq)` index, about
13 µs per action, to return a handful of rows. The cost follows the caller's chain, not the store,
the content or the grant count: in the same conductor process, a short-chain agent answers a no-op
in about 15 ms while matthew, at about 19,100 actions, takes 480–990 ms. Machine writers share the
person's chain and make it longer, and they collide with foreground writes, which then repeat the
whole call. A two-line fork change removes the walks: with it, matthew's no-op falls from about 400 ms to
about 8 ms, the same as the short-chain agent. Whether the walks are the whole of campaign 1.4's
9–21 s is not proved; by straight-line extrapolation they would be about 0.8 s per call at
45,000 actions (see finding 1).

## Ranked findings

Ranked by measured cost on a path someone waits on. **Measured** means a timing or count taken on
the mesh or an offline store; **observed** means seen in logs without a controlled timing;
**inferred** means read from code.

### 1. The capability-grant lookup walks the caller's chain on every call

- **Measured.** No-op call p50 on matthew rose with chain length (growth runs `growth-g1.jsonl`,
  `growth-g2.jsonl`; writes running in the background, so these are under load):

  | chain actions (approx) | no-op p50 ms | 3-action write p50 ms |
  |---|---|---|
  | 120 | 7.1 | — |
  | 1,600 | 24.8 | 102 |
  | 7,800 | 198 | 451 |
  | 10,800 | 265 | 679 |
  | 15,300 | 330 | 1,075 |
  | 18,300 | 469 | 1,441 |

- **Measured, attributed.** With sqlx statement logging (conductor restarted in direct mode, chain
  19,081 actions), 20 no-op calls spent 8.53 s in SQL over a 9.0 s window, so statement time is
  nearly all of the wall time (`sqlwin-noop.json`). The joined grant lookup was 4.80 s of it
  (56.3%): 44 executions, 109 ms mean, at most 8 rows returned. In a five-write window it was 6.21 s
  of 15.4 s (40.3%), up to 747 ms per execution.
- **Measured, cause.** Offline on matthew's stopped store (`offline-matthew-seq18k.json`: 19,081
  actions by that author, 9 CapGrant rows, 0 UpdatedRecord, 0 DeletedRecord) the statement's plan is
  `SEARCH a USING INDEX elohim_Action_author_seq_idx (author=?)` then
  `SEARCH cg USING PRIMARY KEY (action_hash=?)`: it walks every action and probes the grant table
  once per action. 249–270 ms per execution, for each access class, including the two that return
  no rows. The revocation subqueries were suspected from code reading; the census rules them out.
- **Measured, control.** A second five-cell agent (`perf-control`) installed in matthew's own
  conductor, same process and store, unpatched binary: no-op p50 14.6 / 15.1 ms (best 7.8) against
  matthew's 482 / 986 ms (best 284) over two runs of 40. The other build on the box was competing for
  CPU during this run.
- **Code.** Statement `holochain_data/src/dht/inner/cap_grant.rs:133-171`; call chain
  `holochain/src/core/ribosome.rs:397-402` → `holochain_state/src/source_chain.rs:815-834`. The
  caller-is-author shortcut does not apply to a signing credential. The single-statement form came
  from `db2f5bf37`; the original per-grant form is upstream's.
- **Why the existing check missed it.** `zome-call-cost-bounded`'s passing test seeds 600 grants on
  a fresh chain. It never holds grants fixed and grows the chain, and the cost harness counts rows
  returned, which stay at 9 at any length.
- **Arithmetic toward campaign 1.4 (extrapolated, not measured).** In the conductor at 19,100
  actions a grant execution averaged 109 ms and an init execution 76 ms, and the no-op window shows
  about 2.2 grant and 1.35 init executions per call: about 340 ms per call. Straight-line to 45,000
  actions that is about 0.8 s per zome call before any work. (The offline figure of about 13 µs per
  action is slower than the conductor's own and should not be multiplied by the in-conductor counts.)
  An approval that makes nested calls and host reads pays it per call. This makes the walks a
  substantial part of the 9–21 s, not provably all of it.
- **Fork change under test (not on dev, not pushed).** The measuring session's branch
  `shem/per-call-chain-scans` makes CapGrant the outer loop; a plan test
  (`live_cap_grant_lookup_is_driven_by_the_grant_table`) failed on the old statement with exactly the
  plan above and passes on the new one; `holochain_data` lib 155/155.
  - **Measured before/after, committed change** (fork `ab31ecf2c` on `shem/per-call-chain-scans`,
    local only; `ab-patched-ab31ecf2c-r2/`, `ab-unpatched-2b334-r2/`). Quiet box, doorways stopped,
    relay up, four minutes settle, same chain (about 19,400 actions), arms back to back:

    | measure | unpatched `2b334df7973d` | patched `ab31ecf2c` |
    |---|---|---|
    | matthew no-op p50, three runs of 40 | 397 / 425 / 449 ms | 8.8 / 7.8 / 7.7 ms |
    | short-chain control, same conductor | 19.8 / 22.2 / 18.5 ms | 8.1 / 7.4 / 7.7 ms |
    | matthew 3-action write p50 / p90 (100 writes) | 1,867 / 2,298 ms | 141 / 212 ms |
    | control write p50 / p90 (100 writes) | 258 / 401 ms | 119 / 151 ms |
    | matthew idle CPU, 60 s average | 0.729 cores | 0.379 cores |

    Offline on the stopped store the grant lookup went from 270–347 ms to 0.18–0.69 ms, same rows
    (`offline-m1m2-grant-ab.json`, earlier revision of the change). The committed revision adds an
    access-class index and per-grant revocation probes from an independent review, so the lookup
    no longer reads every grant or every update and delete in the store. This measures both
    changes together. Diagnostic builds (frame pointers, line tables) on both sides.
- **Confidence:** high that the walk is the dominant per-call SQL cost at this pin on this mesh.
- **Unproved:** the figure at 45,000 actions (growth stopped a little above 19,100 by operator
  decision); the ranking on the fleet at real chain lengths (no fleet log access from shem).
- **Smallest next measurement:** the same before/after on a stopped copy of the campaign 1.4
  household store, at its real chain length. (Write latency on the patched binary is now in the table
  above.)
- **Home:** `conductor-cap-grant-scan-per-zome-call.md`, RECONCILED 2026-10-02.

### 2. The init check walks the caller's chain on every call

- **Measured.** 2.04 s of the 20-call no-op window (23.9%): 27 executions, 76 ms mean, 1 row each.
  Offline: same `(author, seq)` index walk, 249–275 ms for one row.
- **Code.** `check_or_run_zome_init` builds a fresh `SourceChain` per call whose "init done" flag
  starts false (`holochain/src/conductor/cell.rs:815-842`, `holochain_state/src/source_chain.rs:664`),
  so every call queries `Action WHERE author AND record_validity AND action_type IN (?)` with no
  range, under the per-cell init mutex. No index carries `action_type`. Upstream 0.7.0 has the same
  flag and query.
- **Fork change under test.** A per-cell latch in `check_or_run_zome_init`. The three
  `DELETE FROM Action` paths cannot remove `InitZomesComplete` under a live cell; no code rewinding a
  chain under a live cell was found, which is "none found", not proof.
  - Measured together with finding 1's change (the before/after there). The latch alone was not
    measured separately.
  - Tests on fork `ab31ecf2c`: `initialised_cell_skips_the_init_check` (an initialised cell
    returns while the init mutex is held) and `failed_init_leaves_the_latch_open` pass. The fork's
    older init tests in `src/conductor/tests/` never compile, because no module declares that
    directory; that is a pre-existing gap, not part of this change.
- **Confidence:** high for the mechanism and the combined effect.
- **Home:** `conductor-residual-cpu-full-chain-read-and-perpetual-republish.md`.

### 3. Machine writers share the person's chain and collide with foreground writes

- **Observed.** Growth leg g1 died at chain seq 6,299 on `source chain head has moved since the
  bundle began`; leg g2 counted 14 such refusals in 4,000 writes. The doorway logged its side of the
  same collisions: `Failed to record health attestation` at 20:28:58 (twice), 20:38:59 and
  21:14:06 in `doorway-restart-a.log`, once in `doorway-restart-b.log`.
- **Code.** On an idle mesh the only timed writer on the lamad chain is the doorway peer-health
  probe: every 300 s, per live sibling, `doorway-service/src/services/federation.rs:687-695` calls
  `infrastructure::record_health_attestation`, which bridges to the elohim role's
  `content_store::issue_attestation` (`dna/infrastructure/zomes/infrastructure/src/lib.rs:905,937`):
  1 Create + 2 CreateLink, about 72 actions an hour per peer with nobody using the mesh. The probed
  chain's census fits (12,712 links = 2 × 6,356 app entries; no Update or Delete). Storage wrote
  nothing to that chain; its 60 s heartbeat lands on the infrastructure cell.
- **Why it costs a person.** Every machine action lengthens the chain that findings 1 and 2 walk.
  No zome uses relaxed chain-top ordering, so a collision discards the whole zome call and repeats
  both walks. Storage's head-moved retry has a flat 2 s budget, so once a write costs more than about
  a second it gets one attempt. The bridge crosses cells, so storage's chain-write gate cannot
  serialize it, and the doorway does not retry.
- **Gradient reading (a design input, not decided).** The device-health observation is custody-plane,
  latest-wins, between a doorway and its own node; by the design gate it is Ephemeral. It could leave
  the person's chain for the observation substrate, or go to the node_registry cell, which already
  has `HealthAttestation` and `attest_health`. Either is a doorway- or coordinator-only change; what
  is given up is a peer-signed history of who saw whom, which nothing reads today. Machine head
  nominations must stay in the lamad DHT, and their honest author is a node agent.
- **Confidence:** high for the mesh; the campaign 1.4 household's writer mix is not established.
- **Smallest next measurement:** on a stopped long household store, the share of the top author's
  actions that a person or ceremony authored (census SQL in
  `genesis/local-dev/perf-deep-dive/background-writers-and-planes.md` §5).
- **Home:** `arch-dataplane-borrows-backlog.md` §Plane-separation pass, observations 1 and 2.

### 4. One peer can miss the 75 s publication deadline when its adoption trigger stays silent

This is the deadline that stopped campaign 1.4, measured as the household serving story
(`features/dataplane/epr-app-deliverability.feature`, five stations) on mesh-a.

- **Measured, serving receipt for the pin move** (FINDINGS.md F11, F12):

  | Run (UTC 2026-10-03) | Conductor | Result |
  |---|---|---|
  | 02:16Z, Chromium installed | patched `ab31ecf2c` | 4 of 5: station 3 failed on "within 75 seconds both doorways serve the new browser bundle by content address" (elohim.host's pointer still old at 76,953 ms) |
  | control, same story | unpatched `2b334df7973d` | failed the same scenario one step earlier ("every household peer answers with the same server pointer"), jessica not adopted |
  | 03:04:38Z | patched `ab31ecf2c`, jessica's trigger at debug | 5 of 5 |
  | 03:11:16Z | patched `ab31ecf2c`, jessica's trigger at debug | 5 of 5 |

  Sprint reports `sprint-report-household-20261003T021620Z-4a80267f`, `…T030438Z…`, `…T031116Z…`.
  The failure occurs on both binaries and every conductor call on its path took 33–58 ms, so it is
  independent of findings 1 and 2's change.
- **Observed, where the 75 s went in the failed run** (doorway, storage and step logs): the scenario
  runs its own doorway pair. alpha-A is backed by matthew and passed on its first poll; elohim.host
  is backed by jessica. The second declaration (02:19:55.8Z, through matthew) reached jessica and
  james over content sync at 02:19:55.65. James's adoption trigger adopted it in 4.8 s (attempt 1).
  Jessica's trigger logged nothing at INFO, so jessica waited for the projection-reconcile heal
  sweep, which healed at 02:21:17.19 (about 81 s). Doorway B hot-swapped at 02:21:22.5. The step's
  last poll was at 76.95 s.
- **Code, why the fallback cannot meet the deadline.** On this mesh the heal sweep runs every 30 s
  over a windowed inventory (2,000 rows a page of 5,081; offsets 0 → 2000 → 4000), so a given slug's
  page recurs about every 90 s, plus a tick. That is structurally above 75 s. A peer whose trigger
  stays silent therefore passes or fails depending on sweep phase: the same slug's first declaration
  healed in 25 s because offset 0 came round next.
- **Observed, the passing reruns at debug.** On jessica the second declaration was adopted by the
  trigger on attempt 0 in 338 ms and 425 ms, one of them 41 s after the first, inside the 60 s
  per-id cooldown. So the cooldown does not suppress a second declaration; that candidate is ruled
  out. The same window shows the trigger's single serial worker loaded with machine bookkeeping:
  - 724 of 730 "cannot yet walk the head" / "re-probe ladder spent" lines were for 404 distinct
    device-health attestation documents, nearly all written to matthew's chain by this deep dive's
    own growth probe.
  - The retained-hint pass re-offers them through the same gate and 256-deep queue as sync-apply
    offers, about 60 a minute (`services/head_adoption_trigger.rs:960-996`, queue at `:180`).
  - A full queue drops an offer with no log line (`DroppedFull`, `:738-743`).
- **Leading cause, not proven:** jessica's offer for the person's publication was dropped or
  delayed behind that machine-written backlog. **The failed run's own trigger lines are lost**: it
  ran with the trigger at info, and the 02:59Z storage restart that enabled debug started a new
  `jessica.log`. A probe exit that logs only at debug (head not yet walkable) remains the second
  candidate.
- **Smallest next measurement:** `elohim_head_adoption_trigger_total{outcome}` (`metrics.rs:1579`,
  including `dropped_full`) scraped on jessica during a station-3 run with the backlog present. A
  rising `dropped_full` at the declaration confirms the leading cause.
- **Gradient reading (a design input, not decided).** Machine-written attestations share the
  adoption fast path's queue with a person's publication: the third place observation 1 (finding 3)
  costs a person. Narrow fixes: a priority lane for sync-apply offers over retained hints, or
  excluding attestation kinds from the retained-hint pass. The 75 s deadline is not to be raised.
- **Measurement hygiene.** The growth probe's ~12,000 attestations are now standing load on every
  mesh-a peer's adoption trigger; any later serving result on mesh-a carries it.
- **Confidence:** high for the timeline and the structural sweep worst case; medium for the leading
  cause.
- **Home:** `head-authority-carried-with-content-sync-unit.md` (2026-10-03 paragraphs); detail on
  shem in `genesis/local-dev/perf-deep-dive/serving-elohim-host-miss.md`.

### 5. The publish loop re-reads the author's op set, and on a small network it never stops

- **Measured.** In the five-write window the publish selector was 5.63 s of 15.4 s (36.5%):
  4 executions, up to 1.82 s, 2,119 rows. At idle (30 s window): selector 0.48 s over 5 runs
  (10,937 rows in one), and `UPDATE ChainOpPublish SET last_publish_time` 0.58 s over 8,454
  statements, so at idle the write-back costs slightly more than the read. 57,240 of 62,720 chain ops
  have `receipts_complete IS NULL`. It is the only one of the measured SQL costs that runs with nobody
  calling.
- **Code.** No `LIMIT` on the selector (`holochain_data/src/dht/inner/chain_op_publish.rs:98-123`).
  Completion needs 5 receipts (`publish_dht_ops_workflow.rs:28`; no DNA declares
  `required_validations`), which three peers cannot supply. The 300 s interval limits each op, not
  the set, so every incomplete op comes due every five minutes. A republish to a peer that already
  holds the op cannot earn a receipt: kitsune2 sends op ids and the receiver drops ids it holds
  (`kitsune2_core-0.5.0/src/factories/core_fetch.rs:169-172`), and `require_receipt` is set only on
  first store and is cleared without sending when the author looks offline
  (`validation_receipt_workflow.rs:84-89`). So on a settled mesh the republish is pure cost, and
  some receipts are unrecoverable rather than late.
- **Options:** E, runs between sweeps select only never-published ops and the full sweep runs at
  most once per interval; then A + D, per-op back-off by age and retirement after age T, guarded by
  "sent to at least one peer" (a guarantee change, the operator's call, not approved); then C, index
  plus `LIMIT`; B, completing at reachable authorities, only after upstream discussion. None needs a
  wire, DNA or schema change. Detail: `genesis/local-dev/perf-deep-dive/publish-reselect-design.md`.
- **Option E: approved by the operator 2026-10-02, implemented, measured.** Fork commit `cd568819e`
  on local branch `shem/publish-new-only` (parent `ab31ecf2c`, the grant/init change); not pushed,
  not pinned. Between sweeps a run reads never-published ops whose action sequence is above the chain
  head read just before the last recorded sweep (an `(author, seq)` range), so it costs the actions
  committed since that sweep and never revisits private `CreateEntry` rows; a sweep that hits a
  routing error is not recorded. Tests: plan test, selector-equivalence and floor tests, two
  workflow tests; clippy clean. Plan and conditions: `genesis/local-dev/perf-deep-dive/publish-option-e-plan.md`.
  - **Measured before/after** (FINDINGS.md F9; `e-before-ab31ecf2c/`, `e-after-cd568819e/`): quiet
    box, relay up, doorways stopped, direct launch with statement logging, 4 min settle, 20 writes on
    matthew at about 19,700 actions; before = `ab31ecf2c`, after = `cd568819e`.

    | Over the 20-write window | before | after |
    |---|---|---|
    | All SQL | 2.105 s | 0.897 s |
    | Publish selector | 3 runs, 1.497 s, max 0.783 s, 72 rows (71.1% of SQL) | 20 runs, 0.200 s, max 0.035 s, ≤ 9 rows each (22.3%) |
    | `publishing N ops` lines | 3 (9, 27, 36 ops) | 20, of 9 ops each |
    | Write p50 / p90 | 99 / 131 ms | 102 / 153 ms |

    Every write's ops now go out on its own run instead of being batched behind a whole-chain read.
    Write latency is unchanged, as expected: the selector runs in the publish consumer, not in the
    zome call. The head read `ORDER BY seq DESC LIMIT 1` ran 50 times in both windows, so it is the
    zome-call path's own head read, not the new pre-sweep read (at most one per sweep). Idle SQL per
    60 s fell from 0.98 s to 0.52 s, not attributed to E. Not covered: a window long enough to contain
    the once-per-interval full sweep, and the idle `last_publish_time` write-back, which E does not
    change by design. Shem reads its clock through HPET, which inflates clock-heavy code against a
    TSC host; these are relative figures on one box.
  - **Measured M1** (`offline-m1m2-grant-ab.json`, matthew's author ops, stopped store): 58,017
    publish rows, every one with `receipts_complete IS NULL`. Receipts per op: 2 for 49,801 (86%),
    1 for 7,524 (13%), 0 for 692 (1%); none above 2. No op on this mesh has ever completed.
  - **Measured M2:** all 107,126 receipts arrived less than a minute after integration; every later
    bucket is empty. Every receipt came from the first publish, and no republish earned one.
  - Reading: B alone would complete 86% and leave 14% pending, so it does not bound the cost; the
    measurement favours E, then A + D. The earlier "82–100% idle CPU" was a single `top` snapshot
    and is withdrawn. Averaged over 60 s, matthew's conductor idles at 0.73 cores unpatched and
    0.38 cores patched (finding 1); a relay-down pair (0.61 cores) rules out the relay outage.
    The two idle profiles (FINDINGS.md F10) put about half of the unpatched idle CPU in SQL, roughly
    0.36 cores, nearly all removed by findings 1 and 2's change: the mesh's own background zome
    calls each paid the two chain walks. Of the remaining ~0.38 cores, SQL is about 0.06, wasm
    calls about 0.06, gossip about 0.03 and the HPET clock about 0.03; the publish loop is not
    shown to be the bulk of it, and the rest is not yet attributed per caller.
- **Confidence:** high for the mechanism, its idle cost, and that republish earns nothing here.
- **Home:** `conductor-residual-cpu-full-chain-read-and-perpetual-republish.md`;
  `arch-dataplane-borrows-backlog.md` row 17.

### 6. Storage's head-moving writes can fail at once with `database is locked`

- **Observed.** The mesh logs hold 51, 24 and 30 `database is locked` lines on matthew, jessica and
  james over the session (44/20/25 in the first 48 minutes), at `apply_snapshot` and the REA
  projection's `Update anchor failed` / `Insert failed`. The campaign's head stamp failed the same
  way.
- **Code.** Both head-moving writes read and then write in a deferred transaction; on the author
  path they are nested in an outer deferred transaction (`services/content_service.rs:869-875`).
  In WAL mode such a transaction fails immediately on upgrade if another writer committed after its
  read, whatever the 30 s busy timeout. Diesel reports both busy kinds as
  `DatabaseErrorKind::Unknown`, and storage answers 500 (`services/response.rs:262`). The REA lines
  run without a transaction, so they are plain contention after the full wait, and this mechanism
  does not explain them.
- **Proposal (not routed or decided):** start the author-path transaction `IMMEDIATE`; read, then
  `IMMEDIATE`, then re-check for standalone stamps; map the busy class to 503 with Retry-After; a
  two-connection WAL test that needs no mesh. Detail:
  `genesis/local-dev/perf-deep-dive/storage-begin-immediate-design.md`.
- **Confidence:** medium. The code shape is verified; no trace has shown the upgrade failure itself.
- **Smallest next measurement:** time-to-error of each `database is locked` line. Near-immediate
  means the upgrade; about 30 s means plain contention.
- **Home:** `storage-sqlite-locked-surfaces-as-500-despite-busy-timeout.md`.

### 7. Conductor memory grows by about 700 MB per hosted agent

- **Measured, one snapshot (ethosengine fresh mesh, 2026-10-02 about 16:21Z).** matthew hosting
  16 apps / 80 cells: 12.47 GB resident, 99.8% anonymous; jessica and james with one app each:
  1.73–1.92 GB. About 704 MB per hosted agent, about 137 MiB per cell. One snapshot is not a growth
  law, and the binary's allocator on ethosengine is not known.
- **Code (inferred).** Compiled modules, database pools and the module cache are per conductor or
  per DNA, not per cell, so per-cell structures do not explain 137 MiB. The leading candidate: every
  app install evicts that app's compiled modules, and so does an idle hour; the next call reloads
  them into one shared wasmer engine, which adds a code-memory mapping each time, and nothing found
  releases them. If so, the cost is per install and per idle-hour reload, and no allocator change
  cures it. The code memory is an anonymous private mapping switched to `r-x` for code pages, and
  nothing in the engine frees it; a full reload set is estimated at 0.3–0.5 GB from `wasm.db`'s
  555 MB of serialized artifacts against 54.7 MB of wasm source, the same order as the per-agent
  figure (an estimate, not a measurement). Detail: `genesis/local-dev/perf-deep-dive/reconcile-conductor-b.md` §1.
- **Ranked here** because it is a capacity cost (how many people a conductor can host), not latency
  on a path someone waits on.
- **Smallest next measurement:** executable versus writable anonymous memory before and after one
  install, after one call per role, and after an idle hour. Plan:
  `genesis/local-dev/perf-deep-dive/memory-per-agent-plan.md`. The `perf-control` install on
  matthew may already give one data point without installing anything.
- **Home:** `arch-scale-risk-backlog.md` row 7; `arch-dataplane-borrows-backlog.md` row 18 priority 3.

### 8. SHA-512 in the profile is SQLCipher's page HMAC

- **Measured, one sample.** `perf-matthew-growth-dwarf.data` (10 s, DWARF unwinding):
  `sha512_block_data_order_avx` 14.5% of samples, reached through `readDbPage → sqlite3Codec →
  sqlcipher_page_cipher → sqlcipher_page_hmac → SHA512_Update` from `sqlite3BtreeIndexMoveto` on the
  sqlx worker thread. At about 8,000 actions, 56% of samples were under `sqlite3_step`. Frame-pointer
  unwinding does not recover this caller.
- **Reading.** It is the price of each page that findings 1 and 2 walk, not a separate cost. It
  shrinks with them. Not run through `runtime-performance.ts check`; not an acceptance pass.

## Instrument findings

- **The pinned conductor exports no `hc_*` series.** The exporter commit `0f26f6703` and the rest of
  `int/2026-09-23-diagnostics-throttle-perf` (`61565f320`, `cb61633c2`, …) are not ancestors of
  `2b334df7973d`. Four cures from that branch were re-derived on the pin's own line; the receive
  throttle and the exporter were not. Root `CLAUDE.md`'s promise of `hc_*` on `:9464` under direct
  launch, `zome-call-cost-bounded` DELTA 2026-09-23b and `conductor-capacity-represented` DELTA
  2026-09-23 describe a lineage the dev pin is not on. Attribution here came from the sqlx statement
  log instead (about 18 MB per conductor-minute; bounded windows only).
- **Harness gaps** (`household-mesh-harness-honest-readiness.md` items 8–14): preflight passes with
  no packed hApp and no wasm; `MESH_CONDUCTOR_LAUNCH=direct` applies only on restart; the launcher
  advertises a metrics port the binary cannot serve; browser lanes launch without Chromium installed;
  the prologue's build instruction fails on a fresh box (the working order is CI's); root
  `CLAUDE.md`'s sophia recipe names a script that does not exist; an exported `RUST_LOG` makes
  `mesh start` refuse a matching toolchain.
- **Wasm frames carry no symbols**, so the profiles cannot split idle wasm time between validation
  and coordinator calls. Settling the remaining ~0.06 cores of idle wasm needs per-function counters
  or a wasm symbol map (`genesis/local-dev/perf-deep-dive/idle-zome-callers.md`). With the doorways
  stopped, each storage peer makes about 2 zome calls a minute, only one of them a write (the 60 s
  heartbeat), so storage's own calls cannot explain it.

## Reconciliation of the labelled concerns

53 of the 74 indexed entries now end in a dated `RECONCILED 2026-10-02` line (conductor 20,
kitsune2 2, storage 22, doorway 4, harness 5); 14 entries had tags corrected where their own text
contradicted them; `performance-concern-index.md` has a reconciliation section and recomputed
tables. The 21 app, CI, devspace and cluster entries are not reconciled, held as off the critical
path. Things the pass found beyond the ranked list:

- Cures in code that their entries never recorded: `feedback-discovery-sweep-is-o-n-in-history`,
  `heal-pointer-bytes-ordering-blocking-serve`, `head-authority-carried-with-content-sync-unit`
  item 7, the susan heal-pacing leg. None has a fleet reading after the cure.
- Inventory gossip's receive-side de-duplication is defeated for any publisher with more than one
  page: every delta clears the snapshot fingerprint (`db/peer_blob_inventory.rs:273-277`), so each
  60 s refresh would delete, re-insert and re-score the set inline on the network loop. Inferred;
  the mesh's inventories were empty.
- The household gossip wedge reads as "a 60 s whole-round deadline is not enough at about
  120,000 ops"; peers with an empty arc are dropped from read routing.
- Several storage and doorway entries are probably downstream of findings 1–2, since a storage
  peer's agent authors the seeded corpus and so has the long chain. Each says so as inference.

## The two design hypotheses

- **Machine bookkeeping shares a chain with human acts: confirmed on the mesh** (finding 3), with the
  correction that the writer was the doorway, not storage. The share on the campaign household is
  not measured. It also turned up in a third place: machine-written attestations share the
  adoption trigger's serial queue with a person's publication, the leading (unproven) cause of the
  one serving miss (finding 4).
- **Authority is paid per connection: corrected.** Storage reuses persisted credentials while its
  closed-chain fence is armed (two paths still mint directly without it); the doorway mints once per
  cell per process. The authority cost that remains is per call and priced by chain length
  (finding 1), not grant count. The two compound: every machine write raises the price of every
  later call's authorisation.

## What remains unproved

- Any figure at 45,000 actions; any figure on the campaign 1.4 household or the fleet.
- That findings 1 and 2 are the whole of the campaign's 9–21 s.
- The patched conductor's effect on the fleet (the no-op, write and idle before/after on mesh-a is
  measured: finding 1). The init latch was measured only together with the grant change.
- Why jessica's adoption trigger stayed silent in the one failed serving run: the leading cause is
  an offer dropped behind machine-written attestations, but that run's own trigger lines were lost
  to a restart (finding 4).
- Option E over a window that contains the once-per-interval full sweep; the idle republish
  write-back, which E does not change.
- What the remaining idle CPU on a patched conductor is (finding 5; instrument findings).
- The per-agent heap's composition and the allocator on ethosengine's binary.
- The `database is locked` upgrade mechanism (no trace).

## Decisions that are the operator's

- The pin move for the grant/init change (`ab31ecf2c`). The checks the measuring session named have
  results: the no-op, write and idle before/after (finding 1) and a household serving receipt (two
  5-of-5 runs, finding 4); route in `genesis/local-dev/perf-deep-dive/pin-move-route.md`. Push
  responsibility was handed to the measuring session as a shift; the pin move itself stays the
  operator's call.
- Option E (`cd568819e`) was approved on 2026-10-02 and is measured; it pins separately, after the
  grant/init pin has landed and been measured on the fleet.
- Whether device-health attestations leave the person's chain, and where they go.
- Whether sync-apply offers get priority over retained hints in the adoption trigger, or
  attestation kinds leave the retained-hint pass (finding 4).
- Whether publish may retire an op after an age with incomplete receipts (options A + D): not
  approved.
- Whether to route the storage `IMMEDIATE` and 503 proposal.
- Whether `CLAUDE.md`'s `hc_*` line is corrected, and whether the exporter is re-derived onto the pin.
