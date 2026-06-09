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
all stations freely EXCEPT: /shift kickoffs (pre-authored, operator fires
— except under wiring-decision-5 build authorization),
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
5. **Ambition mode — the build leg (operator decision, 2026-06-09).** The
   maintenance loop alone never builds; the operator observed the agent can
   carry full explore→plan→execute→integrate→push arcs between rounds toward
   *visionary delivery*. With an explicit launch-time BUILD authorization
   (overnight /loop prompt), each cycle adds one build leg: fire the top
   vision×readiness OPEN Objective as an agentic-developer arc (the shift
   discipline owns the arc's rails), integrated under an **evidence-gated
   push lease** (targeted local gates green · single-dispatcher verified ·
   wave watched, fix-or-revert on traced code-red). Selecting from the
   operator's standing ranking is execution; RE-ranking stays ceiling.
   Stall rule: 2 no-progress EXECUTE-phase iterations (arc-phase artifacts
   count as progress) → capture trajectory, fall back to maintenance. Both
   grants live in `.claude/data/push-lease.json` (separate `build`/`push`
   fields, operator-verbatim grant text, revocable at any hour by editing
   the file); absent/unverifiable, commit-only stands and legs suspend.
   Human-visible done-claims require the /deliver two-render rule —
   render-less green re-enters the board as CLAIMED-unverified.
   Paired discipline: **research-before-bail** — unattended legs must
   exhaust the corpus (spec/plan, journals + git trajectory, prior-art
   recall, ledgers) and forge ahead on the best-evidenced reading with a
   journaled interpretive decision, rather than bailing on a question the
   repo can answer; bail stays correct only for judge/ceiling-classed or
   irreversible divergence (gate text: agentic-developer skill, principle 3).

## Watch-outs (close-interval, 2026-06-09 shift evidence)

- **A bare pass_ratio is failure-class-blind.** SUCCESS-only counting read
  "0%" all day at jobs whose only residual red was substrate-gated, steering
  dispatch at ceiling-track work. The floor metric is now the per-job VERDICT
  ladder in `delivery-scoreboard.py` (code-red / attention dispatchable;
  env-gated / fix-landed / clearing are ceiling-or-wait). Never steer by
  ratio alone.
- **/deliver is the most expensive station — bring-up paths are an asset.**
  The first render-proof of a surface cost a full local-stack bring-up
  (stale binary on the port, provenance-gate 404s, two-bundle render
  topology). The verified ladder lives in `hc-dev-orchestrator`
  §"Verified bring-up ladder"; /deliver rounds hand it over and write new
  bring-up facts back. Render verification is also irreplaceable: the
  shipped desktop sidebar was unreachable (display:none panel AND toggle)
  while every deterministic instrument read green.
- **Build-leg hazards (wiring decision 5 — the documented overnight traps).**
  (1) *Concurrent-push mutual abort*: two dispatchers alternately abort each
  other's orchestrator runs with silent webhook loss after failure storms —
  the lease's single-dispatcher reads + SPAWN confirmation exist for this.
  (2) *Only {PR-*, dev} are orchestrator-indexed*: a work-branch push spawns
  nothing — the leased push is the local ff-merge to dev, never a branch
  push expecting a wave. (3) *Overnight permission stalls*: an idle-looking
  session may be blocked on an approval prompt, not done — the unattended
  kickoff variant + durable-palette-only rule exist for this. (4) *The CPS
  64KB Jenkinsfile limit fails at COMPILE time* (#1519/#1520: zero stages
  ran; the red looks total and stageless) — helpers stay heredoc-free per
  the CLAUDE.md gotcha.
- **Instrument liveness.** The SessionStart scope gate crashed silently for
  days (malformed gap-item) and its line VANISHED — a dead instrument read
  as "nothing to report." All headline gates + scoreboard sections now emit
  `⚠ gate-error (…)` on failure (`_gate_subprocess` in placement-audit.py;
  `section()` in delivery-scoreboard.py). A vanished gate line is itself a
  finding; repairing a blind instrument outranks every other dispatch.

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
