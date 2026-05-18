---
name: reach-enum-drift-reconciliation
description: The reach taxonomy is in three drifted forms across the substrate — the schema enum (8 values), the Rust enum used in reach_earning.rs (8 values, different shape), and the resilience-epic Part V vocabulary (5 values). Reconciliation gates the storage-stewardship-summary route landing because the route's bucket filters depend on which reach taxonomy is canonical. Named in the resilience epic Part IX gap matrix and roadmap item 13.
metadata:
  type: project
---

The reach taxonomy is in **three drifted forms** across the substrate. The drift is real and gates downstream UX work.

**The three vocabularies:**

1. **Schema enum** (`elohim/sdk/schemas/v1/enums/reach.schema.json:7`) — 8 values: `private`, `self`, `intimate`, `trusted`, `familiar`, `community`, `public`, `commons`.

2. **Rust enum** in `elohim/elohim-storage/src/services/reach_earning.rs:90-93` — 8 values, different shape: `Reach::Personal | Reach::Intimate | Reach::Household | Reach::Neighborhood | Reach::Collective | Reach::Community | Reach::District | Reach::Public`. Also `reach_earning.rs:241-244` test JSON with `personal/intimate/household/neighborhood/collective/community/district/public`.

3. **Resilience epic Part V vocabulary** (in `genesis/docs/content/elohim-protocol/resilience/README.md`) — 5 values implied by the social-tier custody examples: `household / neighborhood / community / organization / commons`.

**Why this matters:** The storage-stewardship-summary HTTP route (Part V's roadmap item; named in the gap matrix at Part IX) will return a three-bucket breakdown (encrypted / social / commons) where the **social** bucket is `reach IN (...some-set-of-reach-values)`. Which reach-enum is canonical determines the bucket filter. The Angular shefa-pillar widget that renders the bar depends on the same decision.

**The work to close:**

1. Decide which vocabulary is canonical (likely the schema enum, since `feedback_schema_first_ioc` says schemas are protocol-law).
2. Migrate the Rust enum + tests in `reach_earning.rs` to match.
3. Update the resilience epic Part V text to use canonical vocabulary (or document the divergence explicitly if intentional).
4. Audit downstream consumers (validator at `content_store_integrity/lib.rs:514`; doorway `cache/reach_aware_serving.rs`; steward-node `storage/reach.rs`; elohim-storage `p2p/reach_authorization.rs`) for any hardcoded reach strings.
5. Add a schema-contract test that fails if Rust enum and schema enum disagree.

**How to apply:**
- Cartographer: this is a sized substrate-wide cleanup task; ranks roadmap item 13 in the resilience epic (substrate-wide foundational work, not resilience-surface-specific). Worth a dedicated sprint or rolling-into a graph-native / storage-stewardship-summary sprint.
- Memory-ceremony: when reviewing the reach vocabulary, anchor to the schema as canonical, not to the Rust enum. The Rust enum diverged because it was authored to match an older brainstorm; the schema is what protocol consumers depend on.
- For anyone touching reach-related code, grep all three forms before introducing new values. The drift is the kind of thing that compounds quickly.
