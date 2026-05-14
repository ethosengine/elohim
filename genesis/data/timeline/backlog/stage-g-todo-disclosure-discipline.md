---
id: "backlog-stage-g-todo-disclosure-discipline"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Annotate observation-layer plan with explicit Stage-G follow-up TODO markers"
slug: "stage-g-todo-disclosure-discipline"
written: "2026-05-14"
author: "cartographer"
status: "proposed"
priority: "medium"
relatedNodeIds:
  - "memory:feedback_pause_sprint_when_substrate_in_flight"
  - "memory:feedback_signature_changes_grep_callers"
  - "memory:project_principle_p1_reconciliation_controller"
  - "memory:project_signal_driven_audit_ceremonies"
tags: [sprint-discipline, TODO-disclosure, observation-layer, scope-creep-prevention]
shift_objective: |
  Two concrete TODO markers from prior sprint scope-creep remain in-tree:
  `imagodei/zomes/imagodei/src/lib.rs:2925+` and `elohim-storage/src/p2p/shamir_transport.rs`.
  Historian Wave 1 of memory ceremony Run #2 flagged the precedent and suggested the
  observation-layer plan needs Stage-G-style follow-up disclosure baked in. Outcome:
  introduce a `TODO(observation-followup)` marker convention; document it in the
  observation-layer plan front-matter; sweep the two existing markers and either resolve
  or tag with the new convention; add a grep-counter to the signal accumulator so future
  Stage-G-shape work is surfaced as a metric. Done when (a) convention documented in the
  observation-layer plan front-matter; (b) two existing markers either resolved or retagged;
  (c) accumulator tracks `TODO(observation-followup)` counter as a drift signal.
---

# Stage-G TODO disclosure discipline

## Why this matters

Per historian Wave 1 (Run #2): scope-creep TODOs from prior sprints become invisible drift
if not disclosed. The two specific markers are evidence that the discipline hasn't been
codified. Pairs naturally with `observation-vocabulary-collision-disambiguate` — both are
about the observation subsystem getting structural cleanup as it stabilizes.

The attestation consolidation sprint subdivided Stage G into G.1/G.2/G.3 mid-flight when
Shamir transport hit `ElohimStorageBehaviour` registration; the recovery migration is
genuinely deferred (cross-DNA Content deserialization migration must precede the bridge).
The observation/event layer is the next sprint at risk of the same shape — its reads
already outpacing the substrate's writes via graduation-evaluator's `observation_refs`.

## What's blocking

Nothing technical. Convention design is judgment-shape.

## What's ready

- Two concrete markers in-tree at `imagodei/zomes/imagodei/src/lib.rs:2925+` and
  `elohim-storage/src/p2p/shamir_transport.rs`
- Signal-accumulator pattern landed (`project_signal_driven_audit_ceremonies`)
- `feedback_pause_sprint_when_substrate_in_flight` provides the framing
- Historian Run #2 surfaced the load-bearing breadcrumbs (the specific file:line refs that
  Run #1 missed)

## Convergence

- Historian Wave 1: precedent 1 (sprint scope-creep across stage boundaries)
- Cartographer Wave 3: convergence with observation theme; cluster-pair with vocabulary disambiguation

## Definition of done

1. `TODO(observation-followup)` (or similar) convention documented in observation plan
2. Two existing markers swept (resolved or retagged with convention)
3. Signal accumulator tracks the counter as a `cleanup-scan` flag class
