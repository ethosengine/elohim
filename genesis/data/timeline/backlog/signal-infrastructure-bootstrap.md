---
id: "backlog-signal-infrastructure-bootstrap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Bootstrap .claude/scripts/_signals/ accumulator substrate for ceremony triggers"
slug: "signal-infrastructure-bootstrap"
written: "2026-05-15"
author: "cartographer"
status: "envisioned"
priority: "medium"
relatedNodeIds:
  - project_signal_driven_audit_ceremonies
  - feedback_first_memory_team_ceremony
  - feedback_memory_balance_sheet_pattern
tags: [memory-team, signal-infra, ceremony-trigger, run6-surface]
shift_objective: |
  Land `.claude/scripts/_signals/` as a first-class accumulator directory with three
  initial accumulators: (a) `claude-md-budget-regression.jsonl` — one append per
  pre-push hook detection of a CLAUDE.md crossing budget; (b) `defer-budget-comfort.jsonl`
  — one append per ceremony Wave 3 where defer count is below ceiling AND any
  dimension has a forced-attempt-clock entry, recording cartographer's
  substrate-truth-or-convenience self-reckoning verdict; (c) `convergence-collapse-counter.jsonl`
  — one append per ceremony Wave 5 where the same area shows 3+ consecutive cycles
  of converging agent consensus, signaling de-rate-confidence. Wire each
  accumulator with the existing `.claude/scripts/_lib/` walk-up pattern; expose a
  read-only `signals-digest.py` that ceremonies invoke in Wave 1 to surface
  thresholds crossed since last run.
---

# Body

The first ceremony cycle (chronicle:2026-05-14) and Run #5 retro both surfaced the same gap:
ceremonies have no persistent signal store outside the dated `memory-kit/<date>/` reports.
This means every cycle re-reads symptoms from scratch and can't see "this is the third
consecutive cycle where defer-budget felt comfortable while a dimension regressed."

This is the bootstrap entry. It doesn't try to be complete — three initial accumulators are
enough to start. The shape follows the EPR feedback pattern: each accumulator is append-only,
each line is a signal event with a timestamp, threshold breaches emit ceremony-trigger
recommendations rather than auto-triggering. Manifesto-aligned (signal not cadence).

Acceptance: `.claude/scripts/_signals/` exists with three jsonl files initialized; one
producer wired (the pre-push hook for claude-md-budget-regression is the simplest first
producer); `signals-digest.py` reads all three and outputs a Wave-1-ready summary.
The other two producers wire in follow-up cycles as their consumers (ceremony Wave 3
self-reckoning; ceremony Wave 5 convergence detection) come online.
