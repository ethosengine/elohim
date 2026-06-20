---
id: "backlog-operational-weave-tier-region-occupancy"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Operational-weave lens: tier_occupancy + region_occupancy folds (deferred from Slice 4 — no clean source in the viewer-less operational relation)"
slug: "operational-weave-tier-region-occupancy"
written: "2026-06-20"
author: "subagent-driven execution of the operational-weave lens (Wave A of the Weave Epic); operator-approved Slice-4 scope deferral"
status: "open"
priority: "medium"
domain: D5
tags: [operational-weave, facings, lens, tier-occupancy, region-occupancy, declared-tier, weave-epic, deferred]
cites:
  - genesis/docs/superpowers/specs/2026-06-19-operational-weave-facing-lens-design.md
  - genesis/docs/superpowers/specs/2026-06-20-weave-epic-arc-design.md
  - genesis/docs/superpowers/plans/2026-06-20-operational-weave-lens-plan.md
  - elohim/elohim-facings/src/relation.rs
  - elohim/elohim-facings/src/folds/resiliency.rs
  - elohim/elohim-views/src/infrastructure.rs
  - genesis/data/timeline/backlog/resilience-tier-content-declared-floor.md
---

# Operational-weave: tier_occupancy + region_occupancy (deferred)

The operational-weave lens (`WeaveView`) shipped v1 with `placementGapCount`, `rsCoverage`, and
`clusterCapacity` lit (Slices 1–3) and a `GET /api/v1/weave` route (Slice 4). The charter's two remaining
folds — `tier_occupancy` and `region_occupancy` — were **deliberately deferred**: the `WeaveView` fields
`tierOccupancy?`/`regionOccupancy?` are present-but-absent on the wire via the not-selected-field contract,
so the lens is honest, not broken.

## Why deferred (the design gap)

Both folds were specced (`operational-weave-facing-lens-design.md`) as `bucket_by` over the framework
`HolderRow`, but that relation does not carry what they need:

1. **`tier_occupancy(holders) -> RiskTierDistribution` has no tier source.** `HolderRow` is
   `{hub_id, agent_id, region}` — no tier. And `RiskTierDistribution` (low/moderate/elevated/critical/unknown)
   is a **per-content RISK classification** owned by the resiliency facing (`floor_for_tier`/`build_felt_status`
   take the content's declared tier as a parameter), NOT a holder/custodian attribute. `custodian_metrics.tier`
   exists but is a **storage-tier integer**, a different concept whose distribution would need a different
   output type than `RiskTierDistribution`.
2. **`region_occupancy -> RegionalDistributionView` is viewer-relative.** `local`/`regional`/`global` are
   derived from a *viewer* (resiliency's `regional_distribution` splits local-vs-regional against the viewer's
   region). The operational weave is cluster-scoped / viewer-less, so there is no anchor for that split.

## What lighting them needs

- **tier_occupancy:** decide the meaning for a cluster-scoped operational view. Either (a) custodian
  storage-tier occupancy over `custodian_metrics.tier` with a NEW output type (a count-per-storage-tier, not
  `RiskTierDistribution`), or (b) wait for the declared-tier primitive
  (`resilience-tier-content-declared-floor.md`) and aggregate per-content declared tiers. Pick one;
  `RiskTierDistribution` is probably the wrong output type either way.
- **region_occupancy:** a viewer-less region model — e.g. distinct-holders-per-region (a `BTreeMap<region,
  count>`), NOT the viewer-relative `RegionalDistributionView`. Add the fold over `HolderRow.region` and a new
  cluster-region output type.

Both then add their `WeaveView` fields (already optional) + gauges + the schema, following the same
select→fold→aggregate recipe the v1 folds used (`elohim/elohim-facings/CLAUDE.md` §11). No new DHT entry types
(Operational-C, unchanged).
