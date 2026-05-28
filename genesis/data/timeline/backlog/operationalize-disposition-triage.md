---
id: "backlog-operationalize-disposition-triage"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Operationalize storyteller disposition-triage (graduate/memorialize/hold) as a standing hygiene-sweep step"
slug: "operationalize-disposition-triage"
written: "2026-05-28"
author: "librarian"
status: "proposed"
priority: "medium"
relatedNodeIds:
  - backlog-cleanup-scan-disposition-mechanization
  - backlog-cleanup-scan-disposition-taxonomy
  - "memory:project_memory_lifecycle_comet_shape"
  - "memory:project_subconscious_memory_tier"
  - "memory:feedback_first_memory_team_ceremony"
tags: [memory-team, storyteller-substrate, disposition, comet-tail, orphaned-primitive]
shift_objective: |
  The storyteller's three-disposition primitive (graduate / memorialize / hold) — the
  wisdom-graduation pass over the comet's aging tail — lost its orchestrating flow when
  run-6 (commit dfa4c7121, 2026-05-15) pivoted /memory-ceremony from "memory sprint
  disposition triage" to "substrate-currency rewrite." The redesign note recorded that
  disposition hygiene "moved OUT to the librarian-solo /hygiene-sweep," but the move was
  documented (LIFECYCLE.md still owns the matrix) and never operationalized: no script or
  skill step actually invokes the storyteller for graduate/memorialize/hold today. The
  storyteller agent definition now points at the librarian's hygiene-sweep handoff for
  this triage, but the handoff is prose, not a scripted step.
  This objective closes that gap. Add a disposition-triage step to the /hygiene-sweep
  cadence: after cleanup-scan surfaces archive candidates, the librarian hands the
  graduation-eligible subset (lessons that may warrant a canonical story, vs. pure
  staleness) to the storyteller, who returns COVERED / NEEDS-MEMORIALIZATION / HOLD per
  its agent definition. Keep the hard rules (tiny-delete two-signature; graduate requires
  an existing canonical story; memorialize requires historian forensic-value confirm).
  Distinct from backlog-cleanup-scan-disposition-mechanization, which mechanizes the
  LIBRARIAN's archive/dedupe/tiny-delete flag taxonomy — this is the STORYTELLER's
  graduate/memorialize/hold wisdom axis. Done when a hygiene-sweep run produces a
  disposition-triage output without anyone hand-wiring the storyteller dispatch.
---

# Operationalize disposition-triage as a hygiene-sweep step

## Why this matters

The comet model (`project_memory_lifecycle_comet_shape`) promises a dwindling tail and a
memorialized core — but a comet only works if something decides, per cycle, what graduates
to story, what gets memorialized to the Isildur's-diary tier
(`project_subconscious_memory_tier`), and what is held. That decision is the storyteller's
graduate/memorialize/hold primitive. Right now the primitive is *defined* (storyteller
agent definition) and *owned in the lifecycle matrix* (LIFECYCLE.md) but has **no flow that
fires it**. Memory ages without graduation; the tail thickens instead of dwindling.

## What changed (the orphaning)

- Run-6 (`dfa4c7121`, 2026-05-15) repurposed `/memory-ceremony` to substrate-currency
  rewrites and deleted the old "Wave 2 disposition debate."
- `project_substrate_currency_ceremony_redesign` records the intent: disposition hygiene
  moves to the librarian-solo `/hygiene-sweep`.
- The reconciliation was partial — the storyteller and LIFECYCLE.md kept describing a
  "Wave 2" ceremony that no longer exists. The 2026-05-28 memory-coherence review fixed
  the prompt/spec drift (re-homed the storyteller's section onto the librarian handoff and
  pointed here) but the **operational step itself is still unbuilt**.

## Shape of the fix

1. `/hygiene-sweep` cadence (memory-kit SKILL.md) gains an optional disposition-triage
   step after cleanup-scan + judge: librarian classifies which archive candidates are
   graduation-eligible (carry a transferable lesson) vs. pure staleness.
2. Graduation-eligible subset is handed to the storyteller (the handoff the librarian
   already documents). Storyteller returns COVERED / NEEDS-MEMORIALIZATION / HOLD.
3. NEEDS-MEMORIALIZATION items flow to the cartographer's converge as candidate "write the
   story of X" Objectives (the existing path).
4. Decide: scripted librarian-judgment dispatch (like cleanup-judge) vs. a documented
   manual sub-flow. Lean scripted so the comet's graduation is continuous, not heroic.

## Acceptance

A `/hygiene-sweep` run surfaces a disposition-triage output (the three lists) without the
operator hand-wiring a storyteller dispatch; hard rules preserved; cross-linked to the
librarian's flag-taxonomy work so the two disposition axes don't collide.
