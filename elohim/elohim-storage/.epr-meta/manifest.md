---
epr-meta-version: 1
id: elohim-storage-governance
purpose: >
  The truth layer: domain services, diesel persistence, and the dual P2P transport (libp2p and
  iroh) where offline-correct, P2P-native state actually lives. This manifest exists to host the
  habit atoms declared here — six of the register's twelve, which is not an accident of placement
  but a measurement: convergence, blob custody, cross-signed attribution, the operator's runtime
  verbs, sync cost, and conductor capacity are all promises this crate keeps or breaks. It
  carries no author-time rule yet; the gates that hold this tree are structural (the schema
  contract harness, `just gate elohim-storage`, and the cargo-pool CARGO_TARGET_DIR rail), and a
  rule with no lived drift behind it is furniture.
cites:
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
---
# elohim-storage — governance package

Habits declared here (`*.habit.md`) describe THIS crate's behaviour; their checks live wherever
the evidence does — a2o scenarios, cargo tests, live fleet probes — because a habit is the
practice and a suite is only ever evidence for it.

The register is projected from these atoms into `genesis/manifests/habits.yaml` by
`.claude/scripts/habits-project.py`. Edit the atom, never the projection.
