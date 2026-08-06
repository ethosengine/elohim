---
id: "backlog-eprfs-stale-edge-backlog"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "87 stale sealed contract edges today, unattended — the design's own scoreboard/push-gate legs are still unbuilt so nothing surfaces or drains the count"
slug: "eprfs-stale-edge-backlog"
written: "2026-07-25"
author: "claude (saga-status.py implementation session)"
status: "backlog"
priority: "medium"
relatedNodeIds:
  - "genesis/docs/superpowers/specs/2026-07-21-sealed-contract-edges-governor-frontier-design.md"
  - "elohim/eprfs/epr-cli/src/flow/walk.rs"
tags: [eprfs, epr-rea, sealed-edges, stale, governance, stasis]
---

Measured directly (`epr flow status`, 2026-07-25, T6): **`edges: 290 sealed · 0 governed · 87
stale · 0 held · 4 dangling`**. The sealed-contract-edges design (2026-07-21) defines
`stale(e) ⇔ e.governor = cite-seal ∧ body_cid(e.to) ≠ e.sealed_cid` and states its own stasis
target explicitly: "Stasis = 0 stale · unsealed only shrinking · every hold reasoned." At 87
stale out of 290 sealed (30%), the corpus is far from that target, and — checked directly against
the design doc's task checklist — the two mechanisms that would either PREVENT further growth or
surface it in the ambient scoreboard are both still unchecked TODOs: the "Push-gate leg: red
while cite-seal-class stale edges touching pushed paths are neither resealed nor held (BACK-hard)"
and the "Scoreboard + governance wiring: `placement-audit.py` `edges:` headline + `memory-
stasis-loop` edges discipline" items. In their absence, a stale edge can accumulate indefinitely
without ever becoming visible at session-start or blocking a push — this backlog's title says
"unattended growth" because that's the structural situation the unchecked items describe, not
because a specific prior count was independently re-verified this session (no historical 87-vs-79
snapshot was found in-repo to confirm the exact delta; treat that specific number as unverified
and the 87-today figure as the only confirmed data point).

Candidate next step: land the design's own §5 scoreboard leg first (cheapest, read-only) so the
count becomes ambient pressure the same way `habits`/`saga` headlines already are, before attempting
the push-gate leg (which needs the reseal/hold UX to be comfortable enough not to become pure
friction).
