---
title: "Delivery Stasis Loop — the development cycle's ceremony against the developer docs"
id: delivery-stasis-loop-design
status: Draft
class: process-meta
process_subdomain: doc-lifecycle
topic: [delivery, stasis, ceremony, scoreboard, ceiling, floor, conveyor, shift, deliver, converge, gap-items, claimed-unverified, pilot]
cites:
  - unified-memory-loop-design | the parent loop shape — one scoreboard, measure→dispatch→re-measure to stasis — instantiated here one level up at the development cycle | sha256:99100efd20d10129 | path: genesis/docs/superpowers/specs/2026-06-01-unified-memory-loop-design.md
  - findings-sentinel-pattern-design | the sibling instantiation whose floor/ceiling rails vocabulary and self-draining finding classes this loop composes as already-handled pressures | sha256:c284074fe38e2450 | path: genesis/docs/superpowers/specs/2026-06-06-findings-sentinel-pattern-design.md
  - agentic-developer-loop-design | the /shift station this loop pre-authors Objectives for — the kickoff stays operator-fired (ceiling), the rails inside the shift hold the CI floor | sha256:42b461f7c0b7a870 | path: genesis/docs/superpowers/specs/2026-04-16-agentic-developer-loop-design.md
informed-by: [genesis/docs/superpowers/specs/2026-06-01-unified-memory-loop-design.md]
---

# Delivery Stasis Loop

Design session: 2026-06-06 (operator-directed). The third instantiation of
the stasis-loop shape, one level up: memory-stasis-loop drives the memory
disciplines; the findings sentinels drive finding classes; **this loop drives
the whole development cycle to stasis against the developer docs** — the role
the operator has been playing by hand ("honestly that's me right now").

## 1. The problem — operator-as-conveyor

Every station of the cycle exists (/converge ranks, /shift implements,
/deliver verifies, /close-loop captures, scope-reconcile aligns, the findings
sentinels self-drain, the memory loops tend the substrate) and the scoreboard
exists (`placement-audit --ledger`, `delivery-status-distribution`, the CI
ratio windows, the SessionStart gates). What does NOT exist is the loop body
between them: the thing that reads the whole scoreboard, picks the
highest-leverage pressure, dispatches the equipped station, re-measures, and
repeats. The operator is that loop body. The deliverable of this design is
the **role inversion**: operator stops being the conveyor and becomes the
ceiling.

## 2. Stasis definition (against the developer docs)

The developer docs (specs · plans · gap-items · a2o features · backlog ·
roadmap) are the setpoint; delivered reality (code · CI · the running app) is
the measured state. **Stasis** := every doc claim is exactly one of:
- **verified-delivered** (CLAIMED items proven; /deliver-grade evidence),
- **in-flight with a live trajectory** (an Objective, a wip backlog entry,
  a triaged finding awaiting disappearance),
- **held-by-env** (BLOCKED-BY-ENV under scope-reconcile),
- **on the ceiling menu** (a decision only the operator can make).

Nothing orphaned between stations waiting to be *noticed*.

## 3. The loop (measure → dispatch → re-measure)

**Floor (deterministic scoreboard, all local reads):** placement ledger
(orphan-OPEN, CLAIMED-unverified, no-status debt) · delivery-status
distribution · ci-findings ledger + `recent.<job>` ratios + deprecations
ledger · subject-focus/scope gates · decompose-due + roadmap/MAP currency ·
a2o coverage.

**Dispatch table (pressure → equipped station):**
| Pressure | Station |
|---|---|
| ranking stale / next-Objective unclear | /converge (cartographer) |
| CLAIMED-unverified | /deliver |
| dev-intent uncaptured / scenario drift | /close-loop, /story-harvest |
| scope drift (env↔plate) | scope-reconcile --apply |
| findings ledgers (dep/sec/CI) | already self-draining (sentinels, rails) |
| memory gates firing | /memory-stasis-loop, /memory-ceremony |
| OPEN gap-items with READY verdict | **pre-author** the /shift Objective |

**Autonomy envelope (operator decision, 2026-06-06):** the loop dispatches
all stations freely EXCEPT: /shift kickoffs (pre-authored, operator fires),
any `git push` (integrator-owned), env flips (`--set <cap>`), spend
commitments, and anything touching a judge. Those land on the **ceiling
menu** — each round's true deliverable: a shrinking scoreboard plus the 2–5
decisions only the operator can make, each with its evidence attached.

## 4. Wiring (operator decisions, 2026-06-06)

1. **Operator-invoked ceremony** (`/delivery-stasis` skill) — rounds until
   only ceiling items remain; present the menu.
2. **`/loop` self-paced** — overnight-capable; ceiling menu accumulates.
3. **SessionStart light advisory** (`delivery-gate.py` hook) — a ONE-LINE
   planning feed: counts + the single highest-leverage opportunity, framed
   to **inform session direction, surface what could be caught in flight —
   and NEVER disrupt or redirect the pilot's subject unless a listed item is
   a blocker to it**. No auto-dispatch from the gate; it is legibility, not
   a trigger.
4. Scheduled remote routine: deliberately deferred.

## 5. Testing

Gate script: local-file reads only, <1s, silent when at stasis, fail-silent
(never breaks session start); pipe-test counts against crafted fixtures.
Ceremony: first live round produces a scoreboard + ceiling menu without
dispatching anything ceiling-classed.

## 6. Captured follow-ups

1. Scheduled remote routine once cadence proves out.
2. A `delivery:` line merged into the main SessionStart MEMORY BUDGET
   headline (today it is a separate hook line; merging is cosmetic).
3. Round-journal artifact shape (sibling of the shift journal) if /loop
   usage shows the menu needs durable accumulation between sessions.
