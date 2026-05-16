---
id: "roadmap-signal-driven-ceremony-substrate-maturity"
kind: "roadmap"
contentType: "roadmap-item"
contentFormat: "markdown"
title: "Ceremonies migrate from cadence-triggered to signal-triggered over Q3 2026"
slug: "signal-driven-ceremony-substrate-maturity"
written: "2026-05-15"
author: "cartographer"
status: "proposed"
target_window: "2026-Q3"
themes: [memory-team, signal-infra, ceremony-shape]
relatedNodeIds:
  - project_signal_driven_audit_ceremonies
  - backlog-signal-infrastructure-bootstrap
  - backlog-audit-substrate-coverage-and-drift-fidelity
tags: [direction, ceremony-evolution]
---

# Body

The first six ceremony cycles ran on operator invocation — calendar-shaped,
not signal-shaped. The accumulator-shaped substrate exists in principle
(project_signal_driven_audit_ceremonies) but not in practice. The direction
this roadmap entry names: by end of Q3 2026, ceremonies fire because a
threshold crossed (CLAUDE.md budget regression accumulator hits the gate,
defer-budget-comfort accumulator hits 3 consecutive cycles, MEMORY.md size
crosses tolerance band) rather than because the operator typed
`/memory-ceremony`.

What it would feel like to have achieved it: a Tuesday morning where the
operator opens Claude Code, sees "memory-ceremony recommended: signal X
crossed threshold Y on date Z; estimated 30-minute cycle; no operator
gospel-edit pending," and decides whether to run it or defer. The decision
is informed by substrate, not by guessing whether enough has accumulated
since last cycle.

What lands first: the accumulator substrate (backlog-signal-infrastructure-bootstrap),
then the audit-fidelity gates (backlog-audit-substrate-coverage-and-drift-fidelity),
then one producer per accumulator wired into existing hooks, then the
signals-digest.py read path becomes a Wave 1 substrate input. The transition
is gradual; cadence-invocation stays available as the fallback.

Manifesto alignment: principle 6 (signal-driven feedback loops are how the
substrate stays trustworthy). The same shape underlies EPR feedback; bringing
it to memory ceremonies closes the loop.
