---
id: "backlog-conductor-slow-batch-starvation-jessica-class"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Conductor slow-batch starvation (jessica-class): circuit breaker sheds the content heal leg before per-item classification, starving every adopt/contest arm"
slug: "conductor-slow-batch-starvation-jessica-class"
written: "2026-08-15"
author: "claude (shift 2026-08-15T00-54-verify-spin-discharge-live, iteration 4b investigation)"
status: "open"
ci_status: "open"
priority: "medium"
tags: [dataplane, conductor, circuit-breaker, heal-pacing, starvation, saga-06-heads-converge, jessica]
cites:
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
  - genesis/data/timeline/backlog/susan-conductor-ws-dead-heal-pacing-blind-to-instant-errors.md
  - genesis/data/timeline/backlog/spin-divergent-undeclared-rows-block-a-convergence.md
---

# Conductor slow-batch starvation (jessica-class)

**Symptom (live, 2026-08-15 01:25–04:30Z, post-edge-#1350 restart):** jessica's
known_divergent{content} stayed flat at 13 for 3+ hours while matthew (13→2) and
james (14→1) drained the same SPIN-class rows. Not an algebra failure — the SPIN
discharge arm (9132e6d28) is proven reachable on jessica's vantage (one
Refreshed classification at 02:47:49Z).

**Mechanism (ci-investigator, Loki + code, quoted evidence in the shift
journal):** jessica's conductor times out the batched head-resolve calls (15s
per-attempt, fanout=2, batch=8) → `OPENED the unresponsive-conductor circuit`
fired ~28× in the window → the content leg is shed BEFORE per-item Answers
exist → all 5 adopt-candidate branches in `heal_content`
(projection_reconcile.rs:3696–4008) are unreachable (each requires an Answer)
→ `adopt_deferred_heads` receives zero candidates and early-returns before its
metric (adopt_sweep_total=0 across 16 reconcile sweeps). Comparable CALL-level
batch-failure counts on matthew (110 vs 99) but matthew still lands ~10× the
successful classifications (129 vs 13) — the pods are not on the same effective
clock.

**Open questions:**
- WHY jessica's conductor is slower per-batch (CPU/mem/DB contention vs
  post-restart catch-up load). Pyroscope now ingests the fleet (profiler eyes
  open 2026-08-14) — a CPU profile comparison jessica-vs-matthew during a sweep
  window is the natural first probe.
- Whether the circuit's shed-whole-leg behavior should degrade to
  shed-remaining-batch (partial progress per sweep instead of none) — relates
  to the heal-pacing-blind-to-instant-errors thread on susan.
- Whether starved pods self-recover when catch-up completes (watch jessica
  after the fleet quiesces) — if yes, this is a transient-churn class; if no,
  a standing per-pod outage class invisible to the liveness table (an arm can
  be modeled live and scheduled never — the 2026-08-03 lesson, new variant:
  modeled live and STARVED always).

**Probe that would make this class visible without Loki archaeology:** a
counter for circuit-open events per stream
(e.g. `elohim_projection_reconcile_circuit_opened_total{stream}`) + a gauge for
consecutive-sheds; today the circuit is WARN-log-only.

Claimable by any agent; read the shift journal
(.claude/shifts/2026-08-15T00-54-verify-spin-discharge-live.journal.md,
iteration 4b) for the full quoted evidence before starting.

## RECLASSIFIED 2026-08-15 ~11:40Z — same root cause as the fleet-wide contest failure volume

Overnight id-level attribution (shift iteration 7-8) unified this with matthew's
contest failures: the throttle is the conductor admission ceiling
(`conductor_admission.rs` — `content_store`, `class=interactive`,
`capacity = max(2*cpus,8) - CONDUCTOR_RESERVE(3) = 5` on the ~4-CPU household
pods, 5s interactive shed), hit identically on matthew/jessica/james with the
verbatim error string. Jessica's "slower conductor" is the same ceiling seen
from the batch head-resolve side. Levers, ranked: (1) DEMAND — head-plane L1–L3
batching (arch-dataplane-refactor-backlog / DATAPLANE-SDK-PATH critical-path #2)
collapses per-id round-trips; tonight's evidence gives it hard numbers
(106 shed/timeout failure lines in ~7h on one pod). (2) SUPPLY —
`ELOHIM_CONDUCTOR_PERMITS` env bump (OPERATOR ceiling decision; cautioned:
11/27 declare errors were conductor-side websocket timeouts, so the conductor
itself saturates — over-admission trades shed-at-gate for timeout-in-flight).
(3) Class audit — contest declares ride `class=interactive` (5s hold burns
permit time); whether they belong in `Background` (1s defer, cheap retry) is a
bounded code question.
