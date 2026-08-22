---
name: feedback_sprint_cadence_local_batch_then_jenkins
title: "Sprint cadence — batch locally, one Jenkins pass"
description: Cadence target 2026-08-21 — sprint of 10 verified on the local mesh, then ONE Jenkins pass; Jenkins confirms a batch, it never discovers.
metadata:
  type: feedback
---

Before: sprint(1h) → Jenkins verify (2h) → sprint(1h) → Jenkins (2h) = **6 h for n=2** (3 h/item).
Target: sprint of 10 items (2h) → each item reproduced, fixed, gated and re-measured on `just mesh`
INSIDE the sprint → ONE Jenkins pass for the batch (2h) = **4 h for n=10** (0.4 h/item).

**Why:** the fleet measure costs a 7-pod deploy plus catch-up and legitimately no-measures in churn, so
per-item Jenkins round-trips cap throughput at ~2 items/6 h regardless of how fast the fixes are. The
mesh owns its substrate, runs the Act I profile in minutes (194 eligible scenarios ≈ 5.5 min; the saga in
order ≈ 3 min) and surfaced 25 reds in one afternoon; five disjoint fix agents landed in ~90 min.

**How to apply:**
- An item is DONE when it is green on the mesh (scoped re-measure) with its gate green — not when pushed.
- Batch the gates: one cargo gate per tree per sprint where possible (serialize cargo across agents;
  cap link parallelism); the CPU limit (10.5 cores), not RAM, bounds the loop.
- One push per sprint, one `[build:edge]`; Dataplane Validation narrowed to Act II so the Jenkins pass
  measures what only the fleet can (HA through churn, global projection) and stays ~2 h.
- Mesh bring-up must stay one verb and ≤ ~15 min: `just mesh start` → `just mesh prologue`; anything the
  Prologue lacks is a bug in the Prologue, not a reason to test on alpha.
- Disjoint write-sets per agent (doorway / storage-by-module / seeder / a2o steps / conductor fork);
  reproduce on the mesh BEFORE touching code; re-measure scoped (never `-p local <files>`).
Related: [[feedback_mesh_is_the_proving_ground]], [[project_tests_layered_as_acts_of_one_story]],
[[feedback_concurrent_push_mutual_abort]].
