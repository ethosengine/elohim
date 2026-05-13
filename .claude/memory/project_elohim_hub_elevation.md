---
name: elohim-hub elevation — emergent scaling primitive for Tier 3
description: elohim-node graduates from passive deployment wrapper into the active runtime composition primitive for HouseholdHub/CollectiveHub; federation between hubs is the Tier 3 substrate scaling story
type: project
originSessionId: 155036b0-387a-441c-91c5-7a1333fb2f07
---
elohim-node is being elevated from "passive deployment wrapper" (per project_elohim_node_role) into the **active runtime composition primitive** for hubs. A hub (HouseholdHub | CollectiveHub per project_hub_archetype_abstraction) is what scales the P2P dataplane while keeping the whole human-scale invariants (deep inclusivity, integrity, care for system limits).

**Why:** project_substrate_scale_ceiling commits the substrate to Tier 3 family nodes federating, but did not name *what a Tier 3 node composes*. The hub IS the composition. Each hub stays governable by the people inside it (qahal/mishpat-bounded); federation between hubs reaches global scale without flattening any single hub. project_household_horizontal_scaling already says "more blades = more elohim-node instances with different roles" — that's the inside of one hub; the outside is hub-to-hub federation.

**How to apply:**
- New deployment-shape work (conductor wiring, blade orchestration, operator surface per project_household_fabric) carries "what hub does this serve" as a first-class question.
- Module boundary direction (TBD this sprint): elohim-storage = state + projections; elohim-node = runtime composition / hub instantiation; elohim-hub = the abstract composition primitive (possibly a new crate, possibly a trait inside elohim-node).
- Federation work on top of substrate (Phase 3 view-federation, future Bloom inventory, household aggregation per the spec's scale-ceiling section) should be designed with hub-to-hub topology, not undifferentiated peer mesh.
- "Whole human scale" rule: growth = more hubs, never bigger hubs. Federation density determines reach; hub size is bounded by what humans inside can govern.

Open this sprint (light-up-topology Phase 3 kickoff, 2026-05-02): scaffold the elohim-hub / elohim-node / elohim-storage boundary and update the plan to reflect it before continuing federation work.
