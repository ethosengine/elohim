---
name: project_risk_discipline_timeline_tag
title: Risk discipline — the timeline risk tag
description: "Project risks are backlog rows tagged `risk` in a cluster (first: arch-scale-risk-backlog), wired into five recall surfaces — check the cluster before designing on a scale-sensitive plane"
metadata:
  type: project
---

Established 2026-09-04 (operator-directed): a project risk is a `backlog` cluster row with the tag `risk`,
a measurable trigger, a horizon and the change that retires it — a VIEW over the timeline collection, not a
fourth kind and not a register. Discipline: `genesis/data/timeline/CONVENTIONS.md` §Risks. First cluster:
`genesis/data/timeline/backlog/arch-scale-risk-backlog.md` (six rows from the Holochain Evolution Epic code
read: quadratic export walk, O(W²) witness validation, chain doubling + held-carry fan-out, dual-cell window
RAM, per-entry idempotency reads, controller sweep load).

**Why:** the epic's stations went green at node_registry scale (tens of records) while four shapes in the
same code grow quadratically or ∝ peers × records; nothing surfaced them because "risk" had no home and no
reader.

**How to apply:** file a risk row at LANDING time (when the shape is visible in the diff), in the same pass
wire it into the surfaces it deserves — an `inject` + `dedupe-of` rule in the nearest `.epr-meta`, a numbered
entry in the threatened habit's `guard:`, the trigger number on the a2o receipt — and run the MemPalace sync.
A fired row flips to `regression` + a chronicle entry; retire only when the mitigation lands. Before
designing on the lineage / carry / sweep plane, read the cluster first. See [[project_holochain_evolution_epic]].
