---
title: "Story 3.3 design input — admission as receiver-granted lanes (what TCP and Homa teach the serving edge)"
id: admission-receiver-granted-lanes-design
status: Draft
class: architecture
serves: conductor-capacity-represented
date: 2026-09-23
context-tier: disclosed
steward: agent:orchestrator@claude-opus-5-5
graduation-trigger: the story 3.3 brainstorm adopts, reshapes or rejects §4 and its plan is written; this note is then superseded by that plan and retires to history with the one-line lesson
cites:
  - "serving-edge-failover-balance-stream-campaign-plan | the campaign whose story 3.3 (admission degrades as a curve) this note frames; the plan orders the stories, this note seeds the 3.3 brainstorm | sha256:7d6ad91f145361b8 | path: genesis/docs/superpowers/plans/2026-09-19-serving-edge-failover-balance-stream-campaign-plan.md"
  - genesis/data/timeline/backlog/conductor-admission-saturated-for-hours-after-restart.md
  - genesis/data/timeline/backlog/arch-dataplane-borrows-backlog.md
  - genesis/data/timeline/backlog/arch-scale-risk-backlog.md
  - elohim/elohim-storage/.epr-meta/conductor-capacity-represented.habit.md
  - elohim/elohim-storage/src/conductor_admission.rs
---

# Story 3.3 design input — admission as receiver-granted lanes

This is the brainstorm's **starting frame**, not the design. Story 3.3 ("admission degrades as a curve") is gated
on a brainstorm; this note hands that brainstorm one recommended direction, grounded in the 2026-09-23 incident
and in code, so it does not start from a blank page. Line references are to `origin/dev` at `328efba7b`; fork
references are to `elohim/holochain-conductor` at `25dd2d0be` and `elohim/kitsune2` at `22de6e4`.

## 1. The incident, 2026-09-23 (alpha)

After repeated node crashes, matthew's conductor republished about 1.13 M DHT ops per 15 minutes, in rounds of
about 24 k ops: roughly 10× james and jessica and 18× adam. Every alpha conductor sat at its CPU limit. SQLite
median latency was 1.5 s, with a maximum of 210 s. Zome calls from the doorways timed out at 10 s
(`doorway-service/src/services/zome_caller.rs:99`). Storage's conductor-admission gate read in-flight/capacity
adam 100 %, matthew 80 %, james 60 %, yet storage offered the conductor only ~0.2 calls/s per pod. By Little's
law (`L = λW`, the frame `conductor_admission.rs:51-59` already uses), five permits held at 0.2/s means each call
held ~25 s. The gate was full because the conductor was slow, not because we offered much. **The load was
conductor-internal publish and gossip, which none of our telemetry sees.**

Separately, one doorway re-sent the same 3–12 MB `PUT /blob` about 100 times in 3 h. Storage's blob PUT awaited
one Node Registry zome call per shard before replying (`elohim-storage/src/http.rs:3417-3445`). That call is
advisory, and it sat behind the saturated conductor. Moving it off the request path into the Background lane is
change (1) below, in flight on another branch.

## 2. The mapping (agreed with the operator)

| Network lesson | Where we did it | Code |
|---|---|---|
| **Sender-driven retransmit under congestion → collapse** (TCP before Jacobson) | Conductor: every authored op without `receipts_complete` is republished each `min_publish_interval` (300 s default), in one unbounded batch | fork `holochain_data/src/dht/inner/chain_op_publish.rs:115-150` (no `LIMIT`); `holochain_conductor_api/src/config/conductor.rs:702-704` |
| | Doorway: a re-PUT hitting the cache re-forwards the whole payload; a *timeout* is terminal, so the caller re-offers from scratch; nothing dedupes an in-flight forward of the same hash | `doorway-service/src/routes/seed.rs:259-264`, `:535-555` |
| **Incast** (many senders converge on one receiver) | After crashes, every peer catches up with every peer at once; the edge pipeline rolls all peers together (scale-risk row 8) | `elohim/holochain/Jenkinsfile` |
| **ACKs queued behind data** | Completion needs `required_validations`, or 5 receipts by default. Receipts come from validators whose CPU the republished ops are consuming. *Working theory, unconfirmed:* receipts starve → ops stay incomplete → republished → saturation holds | fork `publish_dht_ops_workflow.rs:28`; `holochain/src/conductor/cell.rs:606-651` |
| **No message priority** | The admission gate's classes are wait bounds, not priority: one FIFO pool (`conductor_admission.rs:180-183`). Storage's request pools split read from write only, so a 12 MB PUT takes the same write permit as a small PATCH and holds it for the whole transfer (`http.rs:254-267`, `:1760-1790`) | as cited |

What we already have that is the right shape:

- **Lanes, for our own calls.** `AdmissionClass::{Interactive, Background}` (`conductor_admission.rs:184-189`), with
  5 s vs 1 s wait bounds (`:156`, `:164`). A shed dispatches nothing (`:343-353`). Occupancy and hold time are
  measured (`elohim_conductor_admission_in_flight` / `_hold_ms` / `_wait_ms`, `metrics.rs:2299-2320`). But the
  in-flight gauge carries **no class label**, so how much each lane occupies is not observable today.
- **A reserve, in one place.** `CONDUCTOR_RESERVE = 3` (`:150`) holds three reader permits back for the
  conductor's own keeper (`admissible_permits`, `:258-260`). Reserved capacity is already our idiom; it is just
  not applied between our own lanes.
- **The receiver says how full it is.** Storage sheds with `503 + Retry-After + X-Available-Permits`
  (`services/response.rs:113-128`, `http.rs:1734-1790`). The conductor gate's `Retry-After` is a constant 2 s
  (`SHED_RETRY_AFTER_SECS`, `conductor_admission.rs:171`). It is not derived from the measured hold.
- **Senders that honour it.** The doorway storage proxy surfaces storage's `Retry-After` (`storage_proxy.rs:806-822`).
  Story 3.1's name routing honours a holder's window, clamped to 5–300 s (`name_routing.rs:679-693`, `:995`).
  The seed-blob forward *does* re-offer on a declared shed: clamped 1–5 s, 3 attempts (`seed.rs:400-444`). The
  honest gap is narrower than "blob forward ignores Retry-After". The forward cannot tell a slow receiver from a
  lost transfer, and every retry re-sends every byte.
- **Receiver-driven by design.** Story 4.2's doorbell carries no data. The receiver pulls, with at most one pull
  in flight per peer plus one dirty bit, and a 503's `Retry-After` mutes the sender (`story-4.2-design.md` §2
  C6a/C11, §3.2, in the `doorbell-4.2` worktree, slice 1 `af64f0f07`). kitsune2's publish also sends only op
  ids, and the receiver fetches the data (`kitsune2 crates/core/src/factories/core_publish.rs:162-178`,
  `:344-356`). The sender-driven part is the *republish timer*, not the data transfer.

## 3. Limits of the analogy

We control the application layer and our Holochain/kitsune2 forks. We do not control kernels, NICs or
switches, so Homa's in-network priority queues have no literal home here. A "message" here is a zome call, a
blob, or a publish batch. Homa's lessons land as **grants, lanes and backoff in our own protocols**. Homa is also
tuned for datacenter RTTs, while our slow path is a conductor that answers in 25–60 s, so "shortest first" must
use *expected hold time*, not bytes.

## 4. Recommended direction

**Admission becomes a receiver-issued grant over reserved lanes, taken shortest-first, with limited
overcommitment for background work.** Four rules, all inside `conductor_admission.rs` and its callers, with no
conductor change:

1. **Reserve, don't just bound.** Split the one semaphore into a reserved interactive floor `r` that
   background work can never take, and a background ceiling of `N − r`. Interactive calls may also borrow idle
   background permits. This is `CONDUCTOR_RESERVE` applied between our own lanes. It answers the backlog's
   "a dozen cold background loops hold all five permits indefinitely". Heartbeat call sites re-classed to
   `Background` (the module's own contract) ride the ceiling.
2. **Shortest-expected-first inside a lane (SRPT).** Order each lane's waiters by the expected hold of the zome
   function they call, not by arrival. The estimate is an EWMA per `(zome, fn)` of the hold we already measure.
   An aging bound keeps a long call from starving; Homa gives a fixed share to the oldest message for the same
   reason. A 4 ms `content_store` read should never queue behind a 56 s chain walk.
3. **Limited overcommitment for background (slow start).** After a restart, background concurrency starts at
   1 and grows additively while the hold stays under a target. It halves on a shed or on a hold spike (AIMD).
   This is Homa's "grant to at most k senders at once" and scale-risk row 8's mitigation (3), and it gives the
   backlog's cold-start pacing a mechanism rather than a sleep.
4. **The grant is legible, so no sender retries blind.** Every shed carries a `Retry-After` computed from
   measured hold × queue depth (Little's law), replacing the constant 2 s. Every shed also carries per-lane
   `X-Available-Permits`, and the in-flight gauge gains a `class` label. A caller that holds no window does not
   re-offer. This is the rule the doorway blob forward and the conductor republish loop both break.

**Why this and not the alternatives the backlog named.** A background ceiling alone still lets interactive
calls queue FIFO behind slow interactive calls. Cold-start pacing alone is a timer and does not track the
receiver. Raising `ELOHIM_CONDUCTOR_PERMITS` is a capacity costume on a design signal. The recommended shape
subsumes all three, and every input it needs (hold, wait, occupancy, class) is already measured or one label
away.

**What 3.3 does not cure.** The 2026-09-23 driver was conductor-internal. Rules 1–4 guarantee that when the
conductor answers anything, a person's call is answered first, and that our own offers stop adding to the load.
They cannot reduce republish traffic we do not see. That is change (4) below, in the fork.

## 5. The ordered changes this frames

1. **Bulk off the small lanes.** Node Registry shard registration leaves the blob PUT path for the Background
   admission lane. *In flight on another branch.*
2. **Receiver-granted doorway→storage blob forward.** Storage grants a bulk slot (bounded concurrent large
   bodies per peer, apart from the write pool) before bytes move. The doorway single-flights each hash and asks
   `HEAD /blob/{hash}` before re-sending. Backlog: `arch-dataplane-borrows-backlog` row 16.
3. **This story: reserved control capacity + shortest-first** (§4). Plan follows the brainstorm.
4. **Conductor fork.** Surface the publish rate and the pending-receipt count in telemetry *first*. Then add
   receipt-aware republish backoff and per-peer outbound publish caps (receiver-paced gossip). Backlog:
   `arch-dataplane-borrows-backlog` row 17, graduating scale-risk row 8's mitigation (1).

(1) and (2) stop the doorway pegging storage. (3) protects people inside storage. (4) is the only one that
touches the incident's driver, and its first step is a measurement, not a tune.

## 6. What the brainstorm still owns

- The size of `r` on a 5-permit gate (one permit? two?). It should be derived from the interactive
  arrival rate measured once the gauge carries a class label.
- Whether shortest-first needs its own waiter queue. `tokio::sync::Semaphore` is FIFO-fair, so the smallest
  version is two semaphores plus an ordered waiter list in front of the interactive one.
- Which sites are *control* traffic, whose loss makes the system lie about itself: liveness, head-declare
  authority, shed answers. Should they get a third, tiny lane or sit in interactive?
- The household proof. It needs a fixture that saturates background work (a reconcile flood) while a scripted
  person-facing read measures p99 admission wait. The habit it moves is `conductor-capacity-represented`.
  Fleet readiness: interactive sheds stay at zero while background sheds, and matthew/adam
  `elohim_conductor_admission_in_flight{class="interactive"}` never reaches its floor.
- The p2p-design-gate does not apply: there is no new entity, DHT entry type or route. The lane split is
  process-local state, and a sibling's lane state is observed and never synced (plan A:107).
