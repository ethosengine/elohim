---
title: "Commitment-to-dispatch puller"
id: commitment-dispatch-puller
status: Draft
class: process-meta
context-tier: disclosed
steward: cartographer
graduation-trigger: operator picks the puller into a shift AFTER the write-path (`note`) leg and the dev-system-equilibrium stocks measure have landed, and accepts the plane-typing contract in §3
created: 2026-08-13
domain: D9
topic: [agentic-harness, valueflows, commitment, dispatch, lease, delivery-stasis, symphony, plane-typing]
cites:
  - genesis/data/timeline/backlog/agentic-harness-borrows-backlog.md
  - genesis/research/context-engineering-primary-sources-cross-pollination-2026-08-13.md
  - genesis/manifests/habits.yaml
  - epr-rea-valueflow-fabric | EPR-REA ValueFlow Fabric | sha256:1cec32527dbff6d7 | path: genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md
  - elohim/epr-rea/src/model.rs
  - elohim/epr-rea/src/store.rs
  - elohim/eprfs/epr-cli/src/flow/walk.rs
  - elohim/eprfs/epr-cli/src/flow/project.rs
  - .claude/skills/delivery-stasis/SKILL.md
---

# Commitment-to-dispatch puller

**Decision requested — no implementation is authorized by this note.** May `/delivery-stasis` be
refined to read and write the REA commitment plane natively — polling `epr flow status`, holding an
*ephemeral claim lease* against a *durable commitment*, and dispatching bounded work against it —
rather than re-deriving what to do next from CI scoreboards? Proposed answer: **yes, but only as a
refinement of the existing conveyor, only with the lease and the commitment kept at strictly
different planes, and only after the write path and the stocks measure exist to make the loop
non-empty.**

## 1. Provenance and the habit served

This note is cluster row 4 of the agentic-harness borrows backlog
(`genesis/data/timeline/backlog/agentic-harness-borrows-backlog.md:42`, survey verdict **STUDY-5**),
which routes here explicitly: *"needs a design pass; do not build from this survey."*

Its two source sections:

- **Survey §1.6 — Symphony's two-loop architecture.** The *outer* orchestration loop polls the
  tracker (default 30s), reconciles, and re-checks issue state after every turn (up to
  `agent.max_turns`, default 20, on one live thread); the *model-visible* loop renders the full
  state block on the **first turn only**
  (`genesis/research/context-engineering-primary-sources-cross-pollination-2026-08-13.md:91-102`).
  Only the outer loop is borrowed here. Per-call state injection is **cluster row 1's** claim and
  its only evidence in the corpus is Arize's PlanMessage; the survey names the conflation of the two
  as WATCH-10 and this note does not commit it.
- **Survey §4.1a — plane-typing.** Symphony's `Unclaimed → Claimed → Running → RetryQueued →
  Released` are **ephemeral scheduler reservations** — anti-duplication leases owned by one
  orchestrator, not restored on restart; the service reconstructs by re-polling the tracker
  (`…cross-pollination-2026-08-13.md:97`). Ours are **durable REA promises**: `CommitmentState`
  is `Proposed | Active | Fulfilled | Revoked` (`elohim/epr-rea/src/model.rs:93`), minted `Active`
  only from a textually `CLAIMED` decomposed gap-item
  (`elohim/eprfs/epr-cli/src/flow/project.rs:548`), with fulfillment carried by economic events
  (`FlowEvent.fulfills: Vec<Cid>`, `elohim/epr-rea/src/model.rs:388`). §4.1a's correspondence table
  marks exactly one row ✅ *the actual gap*: "Poll (30s) → reconcile → dispatch loop … *(no puller)*"
  (`…cross-pollination-2026-08-13.md:230`).
- **`Human Review` is a successful handoff boundary, not a terminal state.** "A successful run can
  end at a workflow-defined handoff state (for example `Human Review`), not necessarily `Done`" —
  and by default it is *not* in the spec's terminal list, so moving there stops work without
  terminal-state cleanup (`…cross-pollination-2026-08-13.md:102`). Ours maps to
  ratification-at-dev-merge, not a mid-run interrupt.

**Habit served.** The primary is **`dev-system-equilibrium`** — the cluster's row 7 candidate
(`…agentic-harness-borrows-backlog.md:45`), whose measure is drain-rate ≥ inflow-rate per stock.
It is **not yet in the register**: `genesis/manifests/habits.yaml` holds 12 of 12 habits with none
carrying that id, so admission requires the operator's displace-or-wait call under covenant rule 1
(`genesis/manifests/habits.yaml:40`). Against that candidate habit the puller is the **outflow
actuator** — the stocks measure reads the level, the puller is the only thing that moves it. The
secondary is indirect: the register's single `active: true` habit today is `notary-authority`
(red, `genesis/manifests/habits.yaml:203-209`), and a puller that ranks habit-serving commitments
first drains toward whichever red is active, without ever writing a status flip itself (rule 4,
`genesis/manifests/habits.yaml:46-47`).

## 2. The design question this note settles

**`/delivery-stasis` is already the hand-cranked puller.** Its loop is measure → pick highest-leverage
pressure → dispatch the equipped station → re-measure → ceiling menu
(`.claude/skills/delivery-stasis/SKILL.md:25-80`). What it measures is CI verdicts, ledgers, and the
placement audit (`.claude/skills/delivery-stasis/SKILL.md:29-31`); `delivery-scoreboard.py` contains
no reference to `epr flow`, `flows.jsonl`, or commitments at all (verified by grep, 2026-08-13).
So the conveyor re-derives *what work exists* from downstream scoreboards while the durable
statement of what work exists — 556 active commitments, 539 unfulfilled, measured 2026-08-13
(`…cross-pollination-2026-08-13.md:222`) — sits unread beside it.

The refinement is therefore **not a new scheduler**. It is: the same loop, reading `epr flow status
--json` as a first-class scoreboard section and writing its outcomes back as flow events. The
survey's own words for why this is one problem and not two: *"this is where the System-2 gap and the
multi-agent coordination gap turn out to be one gap."*

Rejected alternatives: minting a scheduler beside `/delivery-stasis` (two conveyors, one of them
blind — the backlog row forbids it in as many words); storing claim state *in* the commitment
(collapses the planes, §3); adding a fifth register to hold run state (the cluster's own LEAVE-11,
`…agentic-harness-borrows-backlog.md:60`).

## 3. The plane-typed join (the core decision) — P2P design gate

**One join, two planes, no conflation.** The puller introduces an **ephemeral claim lease** that
*references* a durable commitment by CID and never mutates it.

- **Entity classification: (c) Ephemeral — by design.** Lost on restart is not a defect to be
  engineered away; it is the correct Symphony semantics (§1.6/§4.1a). A lease is an
  anti-duplication reservation, and a reservation held by a session that no longer exists is
  noise. The lease therefore lives in a **runtime directory outside git and is never committed**;
  it has no DHT entry type, no content-addressed identity requirement, and no notarization.
- **Lease shape (sketch, not a schema):**

  ```
  { commitment_cid, holder, claimed_at, ttl, attempt }
  ```

  `commitment_cid` is the existing `atom_cid` of the durable `Commitment` record; `holder` is a
  session/agent id; `attempt` mirrors Symphony's `attempt` integer so a re-lease after failure is
  distinguishable from a first claim.
- **Reconstruction after restart = re-read `flows.jsonl` + the lease dir.** This is Symphony's
  reconstruct-by-re-polling, with `.eprfs/status/flows.jsonl` (`elohim/epr-rea/src/store.rs:193,206`)
  as the tracker. No lease state is recovered; only live leases in the dir survive, and expired ones
  are reaped on the next reconcile.
- **The lease NEVER mutates the commitment.** `Proposed → Active → Fulfilled/Revoked` remains the
  durable lifecycle, and it moves only through the flow write path: today `epr flow fulfill`
  (`elohim/eprfs/epr-cli/src/flow/mod.rs:145`, `usage()` at `:306-318`), plus the proposed `note`
  leg from **cluster row 2** (`…agentic-harness-borrows-backlog.md:40`) — which does not exist yet
  (`epr flow` today is `project | walk | status | seal | reseal | hold | fulfill`,
  `elohim/eprfs/epr-cli/src/flow/mod.rs:306-318`). A lease is an intent to work; only an event is a
  claim of work done.

## 4. The loop (Symphony-shaped, adapted)

1. **Poll the commitment plane** — `epr flow status --json` (read-only; `walk.rs:405`). Note a real
   constraint for ranking: `top_unfulfilled` is truncated to 10 (`walk.rs:443`) while
   `unfulfilled_total` is the honest count; a ranking puller needs the full unfulfilled set from
   `store.unfulfilled_in_scope` (`elohim/epr-rea/src/store.rs:131`), not the display slice.
2. **Reconcile leases** — expire TTLs, release orphans (holder gone, TTL passed). Reaping is the
   only lease mutation the loop performs without dispatching.
3. **Rank — habit-serving first.** A commitment whose scope resolves to a spec that `cites:`
   `genesis/manifests/habits.yaml` for a habit currently `status: red` outranks everything else;
   within that, the active habit's commitments outrank the rest. This is covenant rule 5
   (`genesis/manifests/habits.yaml:48-49`) used as a ranking key rather than as after-the-fact
   admissibility.
4. **Dispatch bounded work** — one unit against one leased commitment, inside the existing station
   dispatch of `/delivery-stasis` (`.claude/skills/delivery-stasis/SKILL.md:46-58`). No new
   execution surface is minted.
5. **Re-check after each unit** — Symphony's re-check-after-every-turn, borrowed as the outer-loop
   cadence only.
6. **Handoff states:**
   - `fulfilled` — the work produced its evidence; the durable move is a `Produce` event via the
     flow write path, and the lease is released.
   - `blocked` — recorded as a correction on the commitment plane (row 2's `note` leg,
     `run:correction`), lease released, commitment left `Active` and honestly unfulfilled.
   - `human-review` — the **successful handoff boundary**: work stops, the item rides the ceiling
     menu (`.claude/skills/delivery-stasis/SKILL.md:77-81`), and the operator gate is
     ratification-at-dev-merge — peer acceptance at the branch rung, not a solo mid-run interrupt.
     It is not terminal and must never be counted as fulfilled.

## 5. Concern disposition

| Concern | Status | Design answer |
|---|---|---|
| C0 plane location | answered | Lease is Ephemeral (c), runtime dir outside git, never committed; the commitment stays the durable REA record in `.eprfs/status/flows.jsonl`. No DHT entry type, no HTTP route, no table. |
| C1 anti-self-election | partial | A session must not lease a commitment its own writes minted without re-verification — commitments are projected from `CLAIMED` gap-items (`project.rs:548`), so a session that marks its own item `CLAIMED` would otherwise mint and then lease its own work. Design floor: the lease predicate must require an independent verification leg (a gate run, a scenario, a foreign event) before the same holder may lease what it minted. The exact predicate is review-required, not settled here. |
| C2 monotonic authority | answered | Lease acquisition confers no authority over commitment state; `Proposed → Active → Fulfilled/Revoked` moves only through the flow write path (`fulfill`, plus row 2's `note`). |
| C3 liveness | answered | TTL expiry plus orphan release at every reconcile (§4 step 2); a dead holder cannot park a commitment, and lost-on-restart is the designed behaviour, not a stall. |
| C4 honest absence | answered | No lease ≠ no work. The truth is `unfulfilled_total` (`walk.rs:443`), never lease-dir cardinality; an empty lease dir after a crash means nothing was in flight, not that nothing is owed. The `top_unfulfilled` slice is display, not measure. |
| C6a bounded work | answered | One dispatched unit per leased commitment per round, inside `/delivery-stasis`'s existing one-station-per-round rule (`SKILL.md:57-58`); no same-round retry ladder. |
| C6b idempotent effect | answered | Re-acquisition after a crash is keyed by `(commitment_cid, holder)` with an incrementing `attempt`; replay mints no commitment and no event. Effects land only as flow events, which are content-addressed and deduped by CID on append. |
| C11 backpressure (WIP fence) | partial | Max concurrent leases mirrors the covenant's max-2-active fence (`habits.yaml:45`); the register today runs 1 of 2 (`habits.yaml:209`). Whether the puller's fence is literally the habit fence or a parallel constant scoped to commitments is a design choice this note leaves to the implementation review. |
| C14 witnessed residual | answered | Every lease outcome leaves a flow event: `fulfilled` → `Produce`; `blocked` → a `run:correction` note; `human-review` → a handoff note plus a ceiling-menu row. A lease that expires with no event is itself the residual signal and must be counted, never swept. |
| C9 identity lineage | not-applicable | The lease `holder` is a runtime session id with no agent-identity mapping; provenance of the durable record stays git-derived (`occurred_at` is never `now()`). |
| C10 contract evolution | not-applicable | No wire format, schema, or entry type changes; the lease is runtime-local and the commitment record is unchanged. |

## 6. Graduation trigger and sequencing

The puller is **third in its own chain**, and firing it early inherits an empty loop:

1. **Row 2 — write-path discipline / the `note` leg.** A puller over a plane nobody writes to
   projects nothing; the backlog row states the sequencing outright
   (`…agentic-harness-borrows-backlog.md:42`: "Sequence after row 2").
2. **Row 7 — dev-system equilibrium as rates-against-rates**, the stocks measure whose outflow this
   actuates; its primitives (`MeasureKind::Rate{per}`, `Stock{level, inflow, outflow}`) are landed,
   its habit admission is the operator's covenant call.
3. **This note.** Graduation = the operator picks it into a shift after (1) and (2) land, and
   accepts §3's plane-typing contract. Acceptance authorizes an implementation pass; it relaxes no
   evidence floor and mints no habit.

## 7. Open questions

- **Lease directory location.** `.eprfs/runtime/` (beside the sidecar the leases reference, at the
  cost of putting non-durable state under an `.eprfs/` root that has meant *durable content-addressed
  log* until now) versus `.claude/data/` (with the other runtime ledgers, at the cost of splitting
  the commitment plane's two halves across two trees). Unresolved.
- **Multi-session lease visibility and lock discipline.** Sessions share one worktree, so two
  sessions can reach for the same commitment. The one public precedent is the C-compiler run's
  `current_tasks/` lock-file stigmergy — 16 parallel Claudes, ~2,000 sessions, no central
  orchestrator, claim by writing a lock into the shared tree, release by removing it, and the whole
  claim history readable in git (survey §1.10,
  `…cross-pollination-2026-08-13.md:134-144`). Whether the lease dir needs that discipline — and
  whether "readable in git" survives the deliberate never-commit rule of §3 — is unresolved.
