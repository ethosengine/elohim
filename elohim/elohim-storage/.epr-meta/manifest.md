---
epr-meta-version: 1
id: elohim-storage-governance
purpose: >
  The truth layer: domain services, diesel persistence, and the dual P2P transport (libp2p and
  iroh) where offline-correct, P2P-native state actually lives. This manifest exists to host the
  habit atoms declared here — six of the register's twelve, which is not an accident of placement
  but a measurement: convergence, blob custody, cross-signed attribution, the operator's runtime
  verbs, sync cost, and conductor capacity are all promises this crate keeps or breaks. It
  carries one author-time rule — the scale-risk pointer on the lineage bridge sweep, a measured
  shape (fan-out ∝ peers × records) rather than lived drift; every other gate that holds this tree
  is structural (the schema contract harness, `just gate elohim-storage`, and the cargo-pool
  CARGO_TARGET_DIR rail), and a rule with no lived drift or measured risk behind it is furniture.
rules:
  - id: scale-risk-lineage-bridge
    class: inject
    when:
      write: "lineage_bridge.rs"
    dedupe-of: genesis/data/timeline/backlog/arch-scale-risk-backlog.md
    retire-when: >
      when row 3 of the scale-risk cluster reads `retired` — a courier election (one courier per
      neighbour) or a compaction of carried v1 facts has landed and the Station 5 receipt carries
      its catch-up minutes.
    why: >
      SCALE RISK ON THIS PATH — row 3 of the scale-risk cluster: the sweep has EVERY crossed peer
      held-carry EVERY neighbour's v1 records (DHT writes ∝ peers × records), one 16-record page
      per 30 s tick, so a 3.5k-record neighbour is ≈110 min per courier and the window stays open
      at least that long. Three peers on node_registry hides it. Prefer changes that elect a
      courier or compact what is carried over changes that only tune the page size, and put the
      catch-up minutes on the receipt. Advisory only.
cites:
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
---
# elohim-storage — governance package

Habits declared here (`*.habit.md`) describe THIS crate's behaviour; their checks live wherever
the evidence does — a2o scenarios, cargo tests, live fleet probes — because a habit is the
practice and a suite is only ever evidence for it.

The register is projected from these atoms into `genesis/manifests/habits.yaml` by
`.claude/scripts/habits-project.py`. Edit the atom, never the projection.
