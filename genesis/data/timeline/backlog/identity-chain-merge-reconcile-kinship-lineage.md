---
id: "backlog-identity-chain-merge-reconcile-kinship-lineage"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Identity chain fork/merge resolution — byte-minimal tiebreak in chain_head_of/chain_root_of is PROVISIONAL; real resolution is lineage-judgment (kinship-lineage merge-reconcile)"
slug: "identity-chain-merge-reconcile-kinship-lineage"
written: "2026-07-18"
author: "identity-head arc (Wave B minor, whole-arc review follow-up)"
status: "open"
priority: "medium"
area: "imagodei/identity-lineage"
domain: "D2"
jobs: [elohim-edge]
relatedNodeIds:
  - "memory:project_versioned_entity_head_is_declared_dependency"
cites:
  - genesis/docs/superpowers/plans/2026-07-17-identity-head-key-lineage-plan.md
  - genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md
  - genesis/data/timeline/backlog/keyrotation-mint-path-witness-backed.md
tags: [identity, key-rotation, lineage, chain-root, fork, merge, kinship-lineage, version-dag, tiebreak]
---

# Identity chain fork/merge resolution — the kinship-lineage follow-on

## Why this exists (Wave B minor, review-recorded)
The version-DAG walk in `elohim/holochain/dna/imagodei/zomes/imagodei/src/identity_lineage.rs`
resolves a chain's **root** (`chain_root_of` / `identity_chain_root`) and **head**
(`chain_head_of`) deterministically. On a **fork** (a key with >1 tip) or a **merge**
(a key with >1 version-parent — the recovery-reconcile case), the walk currently breaks
the tie by **byte-minimal key order** (`a.get_raw_39().cmp(b.get_raw_39())`). That is a
**provisional** heuristic chosen purely for determinism + append-stability — NOT a
principled resolution of which lineage branch is canonical.

## What "real resolution" means
Choosing the canonical head/root on a genuine fork or merge should be a judgment over
**lineage** (who authorized which branch, recovery-authority provenance, temporal +
controller-quorum evidence) — the "kinship-lineage merge-reconcile" — not raw key bytes.
This is the identity-instance of the declared-head-over-DAG principle
([[project_versioned_entity_head_is_declared_dependency]]): the head is *declared/judged*,
not derived by an arbitrary total order.

## Why it is DORMANT today (not a live bug)
Every identity is currently a **degenerate single-node chain** — no `KeyRotation` entry is
minted end-to-end (see `keyrotation-mint-path-witness-backed`), so no fork or merge can
exist and the byte-minimal branch is **never taken**. The tiebreak becomes reachable only
once (a) real rotation mints multi-node chains AND (b) a genuine fork/merge occurs
(concurrent rotations, or a recovery-reconcile joining branches). So this is gated behind
the mint path and can be designed alongside it.

## When picked up
- Replace the byte-minimal tiebreak in BOTH `chain_head_of` and `chain_root_of` (and the
  HDK `identity_chain_root` / `identity_head` walks) with lineage-judgment resolution.
- Route through `p2p-design-gate` (touches how a notarized chain resolves) — likely reuses
  the `RecoveryAuthority` provenance already on `KeyRotation` edges rather than a new entry.
- Keep chain-root **stability** the contract (the root cid must never change across a
  legitimate rotation) — property-test the fork/merge cases.
- Coordinator-only where possible (DNA-hash-neutral); if fork-resolution needs an integrity
  rule, treat it as a hash-move and follow the DNA-upgrade-governance path.

## Pairs with
`keyrotation-mint-path-witness-backed` (the mint that makes multi-node chains real) and the
identity-head plan Wave B. Until the mint lands, this stays dormant and the provisional
tiebreak is correct-by-degeneracy.
