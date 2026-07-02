---
index: false
name: project_resilience_tier_content_declared_floor
title: Resilience tier = content-declared floor, not reach
description: "Content self-declares its durability FLOOR (tier), orthogonal to reach; deriving tier from reach (reach_to_resilience_tier) is the conflation to correct."
metadata: 
  node_type: memory
  type: project
  originSessionId: 8e95c106-8653-4c93-904c-845e7c85b0d7
---

Operator architecture (2026-06-14): content must **express its own resilience floor** — the durability tier it signals it needs. A will / mortgage / wedding photos / medical records / bridged fiat-bank-statement data express a HIGH (vault) tier; a map that populates a dropdown is EPHEMERAL/cache tier (regenerable, holds only a reference into a commons archive). "What something is should have self-awareness as to where it thinks it should be."

**Resilience tier ⟂ reach.** Reach = who may see it (visibility). Resilience tier = how durably the OWNER needs it held (value/irreplaceability). My will is `private` reach but `vault` tier. They correlate loosely but must NOT be derived from each other.

**The live drift this corrects:** `epr_nav_context_view.rs:31 reach_to_resilience_tier()` derives tier from reach breadth (`commons→high`, `private→unknown`) — backwards for the personal-vault case. Capture: `backlog/resilience-tier-content-declared-floor.md` (NEEDS /brainstorm — 5 design questions: where the tier lives [notarized field vs `requires-resilience` Mishpat::Commitment], vocabulary, who sets the self-aware default [content-type default, owner-overridable — NOT operator, NOT reach], how it drives placement, ephemeral=pointer-not-payload).

**Why:** measuring achieved resilience against a FLAT floor lies both ways — false-alarms ephemeral content and falsely-reassures under-protected vault content. The content-declared floor is the honest denominator AND anti-capture (households declare what matters; no central SLA-setter). Declared tier = Cat-A author truth; achieved = Cat-C projection.

**How to apply:** the slots already exist — `PlacementGapView.requested_steward_count` vs `achieved_steward_count`, `household_resilience.rs:191 desired=7 "// Per-content override deferred to Plan 3"`, the `resilienceTier` field. The grandma felt-status shift (feat/frontend-eyes-sprint) lands a floor-aware SEAM: `FeltStatusView.floor { tier, tierDeclared, wantsHouseholds, hasHouseholds }` with tier→floor mapping built+tested, defaulting `tier:"standard"`/`tierDeclared:false`. The primitive flips `tierDeclared:true` and sets the real floor; the felt surface lights automatically. Do NOT wire the felt floor to the reach heuristic — that bakes in the conflation. Relates to [[project_rea_compute_commitment_primitive]] (tier-as-commitment), [[project_reach_enum_drift_reconciliation]] (sibling axis drift), [[project_epr_router_empties_on_poisoned_scope]] (felt surface must degrade honestly per-row).
