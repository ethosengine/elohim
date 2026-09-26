---
name: feedback_local_mesh_first_cadence_and_measure_offload
title: Local-mesh-first cadence, verification & offload (umbrella)
id: feedback-local-mesh-first-cadence-and-measure-offload
description: "Household mesh proves first; fleet confirms, never discovers. Batch ~10, then one Jenkins pass; DoD runs the gate; measure-class runs go to a peer."
supersedes: [feedback_local_mesh_first_cadence, feedback_measure_class_runs_off_dev_berth]
metadata:
  node_type: memory
  type: feedback
cites:
  - app/elohim-app/scripts/hc-mesh.sh
---

# Local-mesh-first cadence, verification & measure offload (umbrella)

Supersedes [[feedback_local_mesh_first_cadence]] (carried in full below) and folds
[[feedback_measure_class_runs_off_dev_berth]] into it (curated 2026-09-26; takes effect on a
distinct Steward's verdict).
Folds the prove-locally-first development cadence and its verification rails. Members:

- [[feedback_mesh_is_the_proving_ground]] — Operator rule 2026-08-21 — drive design and development on the local mesh (`just mesh`, Act I) and prove there; the fleet CONFIRMS delivery, it does not discover. Land the valueflow chain locally first.
- [[feedback_household_nodes_is_the_stable_floor]] — Degraded-6peer/shem-offline ≠ content work blocked: M/J/J household is a live multi-peer mesh; prove deep there — only cross-node discovery needs @requires:shem.
- [[feedback_sprint_cadence_local_batch_then_jenkins]] — Cadence target 2026-08-21 — sprint of 10 verified on the local mesh, then ONE Jenkins pass; Jenkins confirms a batch, it never discovers.
- [[feedback_sprint_dod_includes_prepush_gates]] — Task DoD must run the touched tree's gate clauses (lint/format:check/typecheck), not just unit tests; graphos sprint went 142-green with a red a2o gate.
- [[feedback_swarm_composition_fresh_tree_build]] — just check on a DNA worktree doesn't verify elohim-storage; swarm edits need a clean-tree cargo build + fmt/clippy pre-push — parallel sessions hide missing code.
- [[project_local_pair_failover_validation_rail]] — How to validate doorway-failover / saga ch04 locally on `just mesh` (two doorways + 3 peers + mongod) before any [build:edge] — the seeding order, the shed drill, and the traps that cost time on 2026-08-21.
- [[feedback_measure_class_runs_off_dev_berth]] — (folded 2026-09-26) Operator 2026-09-25: long measured runs (A/B windows, soaks, proofs on grown stores, anything > 30 min of mesh or cargo lease) never hold the workspace a human develops in — send them to a measurement peer (`just measure`) or queue them until one exists; the dev berth runs verify-class lanes bounded to their budget. If a subagent loops restarts/sleeps "waiting for convergence", stop it and record the window. Contributed with human:matthew as steward of record: no agent author was witnessed writing it, and the record does not say human:matthew wrote it.
