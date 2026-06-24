---
id: "backlog-intentional-placement-strategy-beyond-xor"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Intentional placement strategies behind the seam — diversity/affinity/standing/governance beyond XOR-distance (the P3-8 door)"
slug: "intentional-placement-strategy-beyond-xor"
written: "2026-06-24"
author: "claude (blob-custody Phase-3 brainstorm — operator-directed: XOR is MVP convenience, leave the intentional-placement door open)"
status: "open"
priority: "medium"
domain: D5
tags: [blob, custody, placement, placement-strategy, diversity, household-resilience, affinity, rea-standing, governance, byte-mobility]
cites:
  - genesis/docs/superpowers/specs/2026-06-24-blob-custody-phase3-xor-salvage-placement-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-02-blob-custody-reconciliation-design.md
  - elohim/elohim-storage/src/services/peer_selection.rs
---

# Intentional placement strategies beyond XOR (the P3-8 door)

Phase 3 (`2026-06-24-blob-custody-phase3-xor-salvage-placement-design.md`) ships **XOR-distance as the
MVP placement strategy for developer convenience**, deliberately behind a `PlacementStrategy` seam so the
heuristic is not sealed in. This item tracks the *purposeful* replication that XOR's uniform spread is
blind to. Operator directive (2026-06-24): "have purposeful replication, intentionally… leave that door
open to be more intentional about XOR placement later."

**What XOR is blind to (ranked):**
1. **Failure-domain / household diversity (highest).** Resilience is household-to-household
   (`project_household_is_resilience_unit`); XOR-closest can co-locate replicas in one household → one
   household loss takes them all. Compose with the existing `peer_selection.rs` household/archetype
   diversity multi-pass.
2. **Affinity / relationship-following** — replicate near peers who care about the content.
3. **Capacity- / standing-weighted** — prefer real spare capacity + good REA custodial standing
   (`PlacementCandidate.spare_bytes` already carried).
4. **Governance- / reach-directed** — qahal / content-steward-directed, reach-scoped.
5. **Geographic / latency-aware** for read-locality.

**The named fork to resolve here:** several of these break the determinism XOR relies on for
coordination-free self-selection. Decide the **authoring model** — keep deterministic self-selection
(strategies must agree across peers) vs move to coordinated/governed authoring (planner / steward /
quorum authors placement intentionally). Phase 3 commits to neither.

**Unblocks / depends on:** the Phase-3 seam (P3-1) landing. This item is the P3-8 row of that spec.
