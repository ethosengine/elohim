---
id: "backlog-observation-vocabulary-collision-disambiguate"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Disambiguate observation-session (a2o diagnostic) vs observation-event (substrate witness)"
slug: "observation-vocabulary-collision-disambiguate"
written: "2026-05-14"
author: "cartographer"
status: "proposed"
priority: "medium"
relatedNodeIds:
  - "memory:project_storage_vocabulary_quilt"
  - "memory:project_quilt_pantry_vocabulary"
  - "memory:project_three_layer_truth_model"
tags: [vocabulary, observation, disambiguation, naming-collision]
shift_objective: |
  Two terms have collided in the codebase under the root "observation": (a) April's
  `observation-session` from a2o diagnostic scaffolding (learner-experience capture during
  scenarios), and (b) May's `observation-event` from substrate consolidation (peer_blob_inventory
  + system_metrics reclassified as observation projections). Same root word, two subsystems,
  zero cross-reference. Outcome: write a new memory entry
  `feedback_observation_vocabulary_collision_disambiguate.md` naming both subsystems and
  their boundary; register the disambiguation at `genesis/graphos/vocabulary.md`
  (per project_quilt_pantry_vocabulary pattern); grep both terms across specs/plans/a2o
  and pin each hit to its subsystem. Done when (a) memory entry exists with the resolution;
  (b) vocabulary register lists both with sentence-length definitions; (c) every use of
  bare "observation" in recent specs/a2o is qualified with the subsystem name.
---

# Observation-vocabulary disambiguation

## Why this matters

Storage already had this exact failure mode (quilt vs weave vs lattice, resolved 2026-04-30).
The pattern: when two unrelated subsystems land within ~30 days of each other reaching for
the same root word, the protocol loses its naming discipline silently. Future implementers
read a spec referencing "observation" and choose the wrong subsystem. The fix is cheap if
done now (memory entry + vocabulary register + grep-pin), expensive after another wave of
features.

## What's blocking

Nothing. Memory entry is a single-file write; vocabulary register is append-only; grep-pin
is mechanical.

## What's ready

- `project_quilt_pantry_vocabulary` registered the precedent process
- `project_storage_vocabulary_quilt` shows the resolution template
- Both colliding terms are still localized — `observation-session` in genesis/a2o/,
  `observation-event` in elohim-storage + carrier memory entry for attestation Stages A→G

## Who knows the area

Historian (surfaced the collision via cascade-unmask — Run #2). Cartographer dispatches;
can be Sonnet-shaped.

## Convergence

- Historian Wave 1: precedent 3 (vocabulary collision; April 2026 plans vs May 2026 specs,
  visible only via cross-wing grep)
- Storyteller Wave 2: memory-entry-tier proposal (`feedback_observation_vocabulary_collision_disambiguate.md`)

## Definition of done

1. `feedback_observation_vocabulary_collision_disambiguate.md` written in `.claude/memory/`
2. `genesis/graphos/vocabulary.md` updated with both terms
3. Grep over `genesis/a2o/features/` and `genesis/docs/plans/` for bare "observation" — each pinned
4. May 2026 observation-event spec annotated with cross-reference to April observation-session
