---
name: Household is the resilience unit
description: Resilience computation is household-to-household, not peer-to-peer; households are the mutual-aid graph
type: project
originSessionId: 9c3a2266-4f19-410d-b4d4-30d06366e38d
---
Resilience is modeled household-to-household, not peer-to-peer. A household bundles humans + their devices (a couple, a parent + wards, a multi-generation home). The resilience graph asks "how many households hold a copy of my content, how many households do I reciprocally steward for, can my household survive without any one peer" — not "how many peers hold shards."

**Why:** peers are ephemeral (phones die, laptops close, nodes drain). Households persist and hold the actual stewardship commitments. Shard distribution within a household is implementation detail; resilience across households is the protection claim that matters.

**How to apply:** When designing resilience UIs, compute/display primary grouping by household. Per-peer detail is available as drilldown, but the top-level card is "N households steward this" and "your household stewards for M households." In a2o scenarios (human-resilience.feature), `protection_status` transitions (at-risk → partial → protected) are driven by household reciprocation count, not peer count. Private content is household-scoped by default.
