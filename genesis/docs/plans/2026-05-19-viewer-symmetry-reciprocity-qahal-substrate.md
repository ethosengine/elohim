# Viewer.* Symmetry + Reciprocity + Qahal Substrate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the viewer.* GraphQL surface as a consistent stewardship-aligned set (`hub` / `peers` / `reciprocity` / `collectives`), retire the REST topology fallback, deliver the `/shefa/reciprocity` page, and author the Qahal Collective+Membership DHT design spec as substrate groundwork for the deferred hREA / R&O interop brainstorm.

**Architecture:**
- **L6 — GraphQL viewer.* symmetry** (executes): touches the synthesis-plan schema sketch, flips the `useGraphqlTopology` flag now that Vitest parity (`topology-parity.spec.ts`) is green, adds `Viewer.reciprocity` resolver + matching wire schema + Angular service + page. New code uses stewardship-aligned names from day one; existing internal Rust types (`MyClusterView`, `MyTopologyView`) keep their names and are flagged as a follow-on hygiene pass — out of scope here to keep blast radius small.
- **L7 — Qahal substrate design** (design only): authors a P2P-design-gate-compliant spec for `Collective` and `Membership` DHT entry types in the elohim DNA, including hREA `Organization` / `AgentRelationship` mapping notes. No implementation in this sprint — the spec is the deliverable, feeding the deferred Wave 3 brainstorm.

**Tech Stack:** Rust `async-graphql` 7, Diesel projections, JSON-Schema codegen via `pnpm run schema:codegen:ts`, Angular 19 with signals, Cucumber a2o features.

**Sprint scope reminder (per `project_no_sovereignty_stewardship_over_ownership`):** field names avoid `my`/`own`. The `Viewer.X` namespace already encodes "from this viewer's perspective" — `my` prefixes are redundant ownership coding. Shipped fields `Viewer.hub` and `Viewer.peers` (`elohim/elohim-storage/src/graphql/resolvers.rs:713,728`) already follow this; the deferred surfaces (`reciprocity`, `collectives`) inherit the pattern.

**P2P Design Gate declaration (L6):** zero new DHT entry types, zero new HTTP routes, zero new persistent storage. `ReciprocityView` is **Category C — operational projection** over the existing `rea_commitments` + `rea_economic_events` Diesel relations (which are themselves notarized projections of REA `Commitment` and `EconomicEvent` DHT entries — see the parent synthesis plan, `2026-05-19-topology-resilience-qahal-synthesis.md`, §"P2P Design Gate" table). Identity is CID-derived throughout; rebuild path is signal replay through `rea_projector::handle_content_signal`.

**P2P Design Gate declaration (L7):** the deliverable is a *design document*, not an implementation. The doc itself answers the gate questions (Category, entry type, identity, coordinator function) for `Collective` and `Membership`. Nothing is committed to the DNA until a follow-on plan turns the spec into code.

---

## File Structure

### L6 — viewer.* symmetry + reciprocity

**Create:**
- `elohim/sdk/schemas/v1/views/reciprocity-view.schema.json` — JSON-Schema wire shape (Category-C-compliant, camelCase)
- `elohim/elohim-storage/tests/graphql_viewer_reciprocity.rs` — resolver integration test (mirrors `graphql_viewer_peers.rs`)
- `app/elohim-app/src/app/shefa/services/reciprocity.service.ts` — Angular service (GraphQL-only; no REST fallback for new surface)
- `app/elohim-app/src/app/shefa/services/reciprocity.service.spec.ts` — service unit test
- `app/elohim-app/src/app/shefa/pages/reciprocity/reciprocity.component.ts` — page component
- `app/elohim-app/src/app/shefa/pages/reciprocity/reciprocity.component.html` — template
- `app/elohim-app/src/app/shefa/pages/reciprocity/reciprocity.component.scss` — styles
- `app/elohim-app/src/app/shefa/pages/reciprocity/reciprocity.component.spec.ts` — component spec
- `genesis/a2o/features/shefa/steps/ui/reciprocity.steps.ts` — Cucumber step bindings for the reciprocity scenario

**Modify:**
- `genesis/docs/plans/2026-05-19-topology-resilience-qahal-synthesis.md` — §2 GraphQL schema sketch + §5 Epic B narrative renamed for viewer.* symmetry
- `elohim/elohim-views/src/shefa.rs` — add `ReciprocityView`, `ReciprocityFlow`, `HouseholdReciprocity` view structs
- `elohim/elohim-storage/src/views_convert/shefa.rs` — add `ReciprocityView` builder over `rea_commitments` + `rea_economic_events`
- `elohim/elohim-storage/src/graphql/resolvers.rs` — register `async fn reciprocity(...)` on `ViewerImpl`; register `ReciprocityView`/`ReciprocityFlow`/`HouseholdReciprocity` GraphQL objects
- `elohim/elohim-storage/tests/schema_contract.rs` — add `validate_against_schema("views/reciprocity-view.schema.json", &json)` test
- `elohim/sdk/schemas/scripts/codegen-ts.mjs` — register `reciprocity-view.schema.json` in `INTERFACE_FILES`
- `elohim/sdk/storage-client-ts/src/graphql/queries.ts` — add `ViewerReciprocityQuery` document + response interface
- `elohim/sdk/storage-client-ts/tests/graphql-queries.test.ts` — coverage for the new query shape
- `app/elohim-app/src/environments/environment.ts:29` — `useGraphqlTopology: true`
- `app/elohim-app/src/environments/environment.prod.ts` — `useGraphqlTopology: true`
- `app/elohim-app/src/app/shefa/shefa.routes.ts` (or equivalent loader; verify path at execution time) — register `/reciprocity` route
- `app/elohim-app/src/app/shefa/components/shefa-sidenav/shefa-sidenav.component.{ts,html,spec.ts}` — add nav entry for Reciprocity (currently modified per `git status`; verify wiring)
- `genesis/a2o/features/shefa/m1-matthew-terrance-delivery.feature` — lift `@wip` on cluster + peers scenarios (post flag-flip) and reciprocity scenario (post Epic B landing)

### L7 — Qahal substrate design (design only)

**Create:**
- `genesis/docs/superpowers/specs/2026-05-19-qahal-collective-membership-dht-design.md` — design spec

No source code touched in L7.

---

## Tasks

### Task L6.0: Touch up synthesis plan §2 schema sketch + §5 Epic B

**Files:**
- Modify: `genesis/docs/plans/2026-05-19-topology-resilience-qahal-synthesis.md`

- [ ] **Step 1: Update §2 schema sketch — replace `Viewer.myCluster/myTopology/myReciprocity/myCollectives` with `hub/peers/reciprocity/collectives`**

In `genesis/docs/plans/2026-05-19-topology-resilience-qahal-synthesis.md`, lines 80–86, replace:

```graphql
type Viewer {
  agent: Agent!
  myCluster: MyCluster!            # /shefa/cluster
  myTopology: MyTopology!          # /shefa/peers
  myReciprocity: MyReciprocity!    # /shefa/reciprocity (NEW)
  myCollectives: [Collective!]!    # /qahal/* (NEW, qahal lens)
}
```

with:

```graphql
type Viewer {
  agent: Agent!
  hub: HubView!                    # /shefa/cluster (shipped: Epic A2)
  peers: PeerTopology!             # /shefa/peers   (shipped: Epic A3)
  reciprocity: ReciprocityView!    # /shefa/reciprocity (Epic B, this plan)
  collectives: [Collective!]!      # /qahal/* (Epic E, future)
}
```

- [ ] **Step 2: Rename the type-body blocks accordingly**

Lines 88–150: rename `type MyCluster` → `type HubView`, `type MyTopology` → `type PeerTopology`, `type MyReciprocity` → `type ReciprocityView`. The internal field names inside those types are out of scope for this rename (deferred, see Step 3).

- [ ] **Step 3: Add a deferred-debt callout after the schema block**

After the closing ``` of the GraphQL block (line 151), add this paragraph:

```markdown
**Viewer.* symmetry — scope note (2026-05-19, L6 pass):** the field-level naming above (`hub`, `peers`, `reciprocity`, `collectives`) is the stewardship-aligned surface per `project_no_sovereignty_stewardship_over_ownership`. The shipped fields `Viewer.hub` and `Viewer.peers` already conform (`elohim/elohim-storage/src/graphql/resolvers.rs:713,728`). Internal Rust view types still carry `MyClusterView` / `MyTopologyView` names (`elohim/elohim-views/src/infrastructure.rs:1680`); the existing `impl From<MyClusterView> for HubView` keeps wire/internal nomenclature decoupled. Renaming internal types is a follow-on hygiene pass; this plan leaves them alone to keep L6 blast radius small.
```

- [ ] **Step 4: Update §5 Epic B language**

Lines 719–727 in the same file: replace every occurrence of `Viewer.myReciprocity` with `Viewer.reciprocity` and `/shefa/reciprocity (NEW)` with `/shefa/reciprocity`.

- [ ] **Step 5: Commit**

```bash
git add genesis/docs/plans/2026-05-19-topology-resilience-qahal-synthesis.md
git commit -m "docs(plan): viewer.* symmetry — rename schema sketch + Epic B for L6"
```

---

### Task L6.1: Add ReciprocityView wire schema

**Files:**
- Create: `elohim/sdk/schemas/v1/views/reciprocity-view.schema.json`
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs`

- [ ] **Step 1: Read the existing convention doc + a representative schema**

```bash
cat elohim/sdk/schemas/v1/views/CONVENTIONS.md
cat elohim/sdk/schemas/v1/views/peer-topology-view.schema.json
```

The 10 conventions must be followed verbatim (camelCase, `$id`, required-field discipline, etc.). The peer-topology schema is the closest structural sibling.

- [ ] **Step 2: Write `reciprocity-view.schema.json`**

Create `elohim/sdk/schemas/v1/views/reciprocity-view.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://elohim.host/schemas/v1/views/reciprocity-view.schema.json",
  "title": "ReciprocityView",
  "description": "Viewer-scoped reciprocity surface — committed and delivered REA flows classified by reach and counterparty household. Category-C projection over rea_commitments + rea_economic_events.",
  "type": "object",
  "additionalProperties": false,
  "required": ["agentCid", "inflowByReach", "outflowByReach", "byHousehold"],
  "properties": {
    "agentCid": {
      "type": "string",
      "description": "Viewer agent CID."
    },
    "inflowByReach": {
      "type": "array",
      "description": "Flows the viewer received, classified by reach.",
      "items": { "$ref": "#/$defs/ReciprocityFlow" }
    },
    "outflowByReach": {
      "type": "array",
      "description": "Flows the viewer delivered, classified by reach.",
      "items": { "$ref": "#/$defs/ReciprocityFlow" }
    },
    "byHousehold": {
      "type": "array",
      "description": "Committed-vs-delivered totals per peer household.",
      "items": { "$ref": "#/$defs/HouseholdReciprocity" }
    }
  },
  "$defs": {
    "ReciprocityFlow": {
      "type": "object",
      "additionalProperties": false,
      "required": ["reach", "committedBytes", "deliveredBytes", "commitmentCount"],
      "properties": {
        "reach": {
          "type": "string",
          "enum": ["self", "intimate", "qahal", "community", "global"],
          "description": "Reach classification of the flow."
        },
        "committedBytes": { "type": "integer", "minimum": 0 },
        "deliveredBytes": { "type": "integer", "minimum": 0 },
        "commitmentCount": { "type": "integer", "minimum": 0 }
      }
    },
    "HouseholdReciprocity": {
      "type": "object",
      "additionalProperties": false,
      "required": ["householdId", "committedBytes", "deliveredBytes"],
      "properties": {
        "householdId": { "type": "string" },
        "displayName": { "type": ["string", "null"] },
        "committedBytes": { "type": "integer", "minimum": 0 },
        "deliveredBytes": { "type": "integer", "minimum": 0 }
      }
    }
  }
}
```

If the existing reach enum values in `elohim/sdk/schemas/v1/enums/reach.schema.json` differ from `["self","intimate","qahal","community","global"]`, replace the inline enum with `{ "$ref": "../enums/reach.schema.json" }`. Verify before writing:

```bash
ls elohim/sdk/schemas/v1/enums/ 2>&1 | grep -i reach
cat elohim/sdk/schemas/v1/enums/reach.schema.json 2>&1 | head -30
```

- [ ] **Step 3: Register schema in codegen-ts.mjs**

Open `elohim/sdk/schemas/scripts/codegen-ts.mjs` and find the `INTERFACE_FILES` array (or equivalent — verify the exact constant name at execution time). Add `'views/reciprocity-view.schema.json'`. Also add any `refMap` entries required per memory `feedback_codegen_relative_ref_keys` — all relative-path key forms must be present (bare, `"./"`, `"../views/"`, `$id`).

- [ ] **Step 4: Run codegen and verify TS output**

```bash
cd /projects/elohim
pnpm run schema:codegen:ts 2>&1 | tail -20
```

Expected: a new file `app/elohim-app/src/app/generated/reciprocity-view.ts` (and matching `elohim/sdk/storage-client-ts/src/generated/ReciprocityView.ts`) appears. Inspect:

```bash
head -40 app/elohim-app/src/app/generated/reciprocity-view.ts
```

- [ ] **Step 5: Validate the schema itself**

```bash
pnpm run schema:validate 2>&1 | tail -10
```

Expected: `✓ all schemas valid` or equivalent.

- [ ] **Step 6: Commit**

```bash
git add elohim/sdk/schemas/v1/views/reciprocity-view.schema.json \
        elohim/sdk/schemas/scripts/codegen-ts.mjs \
        app/elohim-app/src/app/generated/reciprocity-view.ts \
        elohim/sdk/storage-client-ts/src/generated/ReciprocityView.ts
git commit -m "feat(schemas): ReciprocityView wire schema + TS codegen"
```

---

### Task L6.2: Add Rust ReciprocityView struct + view builder

**Files:**
- Modify: `elohim/elohim-views/src/shefa.rs`
- Modify: `elohim/elohim-storage/src/views_convert/shefa.rs`

- [ ] **Step 1: Read existing patterns**

```bash
sed -n '1,40p' elohim/elohim-views/src/shefa.rs
sed -n '1,40p' elohim/elohim-storage/src/views_convert/shefa.rs
```

The existing `EconomicEventView` / `ReaCommitmentView` / `MeasureView` patterns govern serde discipline and the `#[derive(TS)]` export discipline.

- [ ] **Step 2: Add `ReciprocityView`, `ReciprocityFlow`, `HouseholdReciprocity` to `shefa.rs`**

Append to `elohim/elohim-views/src/shefa.rs` (preserve existing imports + module structure):

```rust
/// Viewer-scoped reciprocity surface. Category-C projection over
/// rea_commitments + rea_economic_events. Stewardship-aligned name —
/// no "my" prefix per project_no_sovereignty_stewardship_over_ownership.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "export-bindings", derive(TS))]
#[cfg_attr(feature = "export-bindings", ts(export))]
pub struct ReciprocityView {
    pub agent_cid: String,
    pub inflow_by_reach: Vec<ReciprocityFlow>,
    pub outflow_by_reach: Vec<ReciprocityFlow>,
    pub by_household: Vec<HouseholdReciprocity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "export-bindings", derive(TS))]
#[cfg_attr(feature = "export-bindings", ts(export))]
pub struct ReciprocityFlow {
    pub reach: Reach,
    pub committed_bytes: u64,
    pub delivered_bytes: u64,
    pub commitment_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "export-bindings", derive(TS))]
#[cfg_attr(feature = "export-bindings", ts(export))]
pub struct HouseholdReciprocity {
    pub household_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub committed_bytes: u64,
    pub delivered_bytes: u64,
}
```

If the `Reach` enum lives elsewhere (e.g. `elohim_views::common::Reach`), import via `use crate::common::Reach;` — verify at execution time by grepping `^pub enum Reach`.

- [ ] **Step 3: Add `build_reciprocity_view` to views_convert/shefa.rs**

Append a builder that aggregates from existing Diesel models. Reference the schema for shape; reference `rea_commitments.rs` + `rea_economic_events.rs` for field names. Sketch:

```rust
pub fn build_reciprocity_view(
    conn: &mut PgConnection,
    agent_cid: &str,
) -> Result<ReciprocityView, ViewBuildError> {
    use crate::db::schema::{rea_commitments::dsl as c, rea_economic_events::dsl as e};

    // 1. Pull commitments where receiver_agent = agent_cid (inflow) and provider_agent = agent_cid (outflow).
    // 2. Pull matching economic_events (fulfillment receipts) for delivered totals.
    // 3. Group inflow by reach, outflow by reach.
    // 4. Group by counterparty household (joined via peer_identity_bindings).

    todo!("walk rea_commitments + rea_economic_events; classify by reach; group by household")
}
```

Replace the `todo!` with the actual aggregation using Diesel `.filter(...).group_by(...).select(...)` patterns from neighboring builders (e.g. `build_peer_topology_view`). The builder must:
- Filter `rea_commitments` by `agent_cid` on both `provider_agent_cid` (outflow) and `receiver_agent_cid` (inflow) — verify exact column names at execution time.
- Join `rea_economic_events` via `commitment_id` for `delivered_bytes`.
- Resolve household identity via `peer_identity_bindings` for `byHousehold`.
- Use `displayName: None` when binding has no display name; never invent a default.

- [ ] **Step 4: Add a unit test for the builder**

Append to `elohim/elohim-storage/tests/views_shefa.rs` (or its sibling — verify location):

```rust
#[test]
fn reciprocity_view_classifies_by_reach() {
    let mut conn = test_conn();
    seed_reciprocity_fixture(&mut conn); // committed: intimate=100, qahal=50; delivered: intimate=100, qahal=25
    let view = build_reciprocity_view(&mut conn, "agent-matthew-cid").unwrap();
    assert_eq!(view.inflow_by_reach.len(), 2);
    let intimate = view.inflow_by_reach.iter().find(|f| f.reach == Reach::Intimate).unwrap();
    assert_eq!(intimate.committed_bytes, 100);
    assert_eq!(intimate.delivered_bytes, 100);
}
```

Define `seed_reciprocity_fixture` inline near other test fixtures in the file. Use existing `test_conn()` helpers from neighboring tests.

- [ ] **Step 5: Run the test**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo nextest run reciprocity_view_classifies_by_reach 2>&1 | tail -20
```

Expected: PASS.

- [ ] **Step 6: Schema-contract sample test in `tests/schema_contract.rs`**

Find the existing `validate_against_schema("views/peer-topology-view.schema.json", ...)` test (`tests/schema_contract.rs:~1978`) and add a sibling for ReciprocityView:

```rust
#[test]
fn reciprocity_view_matches_schema() {
    let sample = ReciprocityView {
        agent_cid: "agent-sample".to_string(),
        inflow_by_reach: vec![ReciprocityFlow {
            reach: Reach::Intimate,
            committed_bytes: 100,
            delivered_bytes: 80,
            commitment_count: 2,
        }],
        outflow_by_reach: vec![],
        by_household: vec![HouseholdReciprocity {
            household_id: "household-terrance".to_string(),
            display_name: Some("Terrance".to_string()),
            committed_bytes: 50,
            delivered_bytes: 50,
        }],
    };
    let json = serde_json::to_value(&sample).unwrap();
    validate_against_schema("views/reciprocity-view.schema.json", &json);
}
```

- [ ] **Step 7: Run the contract test**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo nextest run reciprocity_view_matches_schema 2>&1 | tail -20
```

Expected: PASS.

- [ ] **Step 8: Run `cargo test export_bindings` to regenerate TS bindings**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo test export_bindings 2>&1 | tail -20
```

Verify `ReciprocityView.ts`, `ReciprocityFlow.ts`, `HouseholdReciprocity.ts` appear in `elohim/sdk/storage-client-ts/src/generated/`.

- [ ] **Step 9: Commit**

```bash
git add elohim/elohim-views/src/shefa.rs \
        elohim/elohim-storage/src/views_convert/shefa.rs \
        elohim/elohim-storage/tests/views_shefa.rs \
        elohim/elohim-storage/tests/schema_contract.rs \
        elohim/sdk/storage-client-ts/src/generated/Reciprocity*.ts
git commit -m "feat(shefa): ReciprocityView Rust struct + builder + schema contract"
```

---

### Task L6.3: Add Viewer.reciprocity GraphQL resolver

**Files:**
- Modify: `elohim/elohim-storage/src/graphql/resolvers.rs`
- Create: `elohim/elohim-storage/tests/graphql_viewer_reciprocity.rs`

- [ ] **Step 1: Study the shipped `hub` and `peers` resolver shape**

```bash
sed -n '700,760p' elohim/elohim-storage/src/graphql/resolvers.rs
```

The pattern: `async fn <field>(&self, ctx: &Context<'_>) -> FieldResult<<Type>>` that pulls the DB pool from `ctx`, calls the view builder, and converts via `From`. Mirror this pattern.

- [ ] **Step 2: Register `ReciprocityView` / `ReciprocityFlow` / `HouseholdReciprocity` as async-graphql objects**

In `resolvers.rs`, add object impls. Two options: (a) derive `#[derive(SimpleObject)]` on the view structs in `elohim-views`, or (b) define wrapper structs in `resolvers.rs` and `impl From<crate::views::shefa::ReciprocityView>` for the wrapper. Option (b) is the precedent set by `HubView` (line 487: `impl From<crate::views::MyClusterView> for HubView`). Use option (b) for symmetry with the existing pattern.

Define wrapper structs at the top of `resolvers.rs` near `HubView`:

```rust
#[derive(SimpleObject)]
pub struct ReciprocityViewGql {
    pub agent_cid: String,
    pub inflow_by_reach: Vec<ReciprocityFlowGql>,
    pub outflow_by_reach: Vec<ReciprocityFlowGql>,
    pub by_household: Vec<HouseholdReciprocityGql>,
}

#[derive(SimpleObject)]
pub struct ReciprocityFlowGql {
    pub reach: ReachEnum,
    pub committed_bytes: u64,
    pub delivered_bytes: u64,
    pub commitment_count: u32,
}

#[derive(SimpleObject)]
pub struct HouseholdReciprocityGql {
    pub household_id: String,
    pub display_name: Option<String>,
    pub committed_bytes: u64,
    pub delivered_bytes: u64,
}

impl From<crate::views::shefa::ReciprocityView> for ReciprocityViewGql {
    fn from(v: crate::views::shefa::ReciprocityView) -> Self {
        Self {
            agent_cid: v.agent_cid,
            inflow_by_reach: v.inflow_by_reach.into_iter().map(Into::into).collect(),
            outflow_by_reach: v.outflow_by_reach.into_iter().map(Into::into).collect(),
            by_household: v.by_household.into_iter().map(Into::into).collect(),
        }
    }
}
// (mirror From<...> for ReciprocityFlowGql, HouseholdReciprocityGql)
```

If `ReachEnum` doesn't exist yet in the GraphQL surface, look for how Reach is encoded in `PeerHouseholdEdgeGql` (peers resolver) and reuse that encoding.

- [ ] **Step 3: Add `async fn reciprocity(...)` to ViewerImpl**

After `async fn peers(...)` at `resolvers.rs:728`, append:

```rust
async fn reciprocity(&self, ctx: &Context<'_>) -> FieldResult<ReciprocityViewGql> {
    let pool = ctx.data::<DbPool>()?;
    let agent_cid = self.agent_cid.clone();
    let view = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get()?;
        crate::views_convert::shefa::build_reciprocity_view(&mut conn, &agent_cid)
    })
    .await??;
    Ok(view.into())
}
```

The exact `DbPool` type + connection-acquisition pattern + error-conversion shape must match `peers` line-for-line. Copy that resolver's body and substitute `build_reciprocity_view` for `build_peer_topology_view`.

- [ ] **Step 4: Add resolver integration test**

Create `elohim/elohim-storage/tests/graphql_viewer_reciprocity.rs` mirroring `tests/graphql_viewer_peers.rs`:

```rust
//! GraphQL integration: Viewer.reciprocity end-to-end.

mod common;
use common::{seed_reciprocity_fixture, test_schema};
use serde_json::json;

#[tokio::test]
async fn viewer_reciprocity_returns_classified_flows() {
    let (schema, _pool) = test_schema().await;
    seed_reciprocity_fixture(&schema).await;

    let query = r#"
        query {
            viewer(agentCid: "agent-matthew-cid") {
                reciprocity {
                    inflowByReach { reach committedBytes deliveredBytes commitmentCount }
                    outflowByReach { reach committedBytes deliveredBytes }
                    byHousehold { householdId displayName committedBytes deliveredBytes }
                }
            }
        }
    "#;

    let result = schema.execute(query).await;
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    let data = result.data.into_json().unwrap();
    let inflow = &data["viewer"]["reciprocity"]["inflowByReach"];
    assert!(inflow.as_array().unwrap().len() >= 1);
}
```

Add `seed_reciprocity_fixture` helper to `tests/common/mod.rs` (or wherever `seed_peer_fixture` lives — verify location).

- [ ] **Step 5: Run the GraphQL test**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo nextest run --test graphql_viewer_reciprocity 2>&1 | tail -30
```

Expected: PASS.

- [ ] **Step 6: Verify the SDL contains `reciprocity` field on Viewer**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
cargo run --bin export-graphql-sdl 2>&1 | grep -A2 "type Viewer" | head -20
```

If no `export-graphql-sdl` binary exists, run the existing SDL-snapshot test in `tests/graphql_*` instead. Expected output should show `reciprocity: ReciprocityViewGql!` as a field.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/graphql/resolvers.rs \
        elohim/elohim-storage/tests/graphql_viewer_reciprocity.rs \
        elohim/elohim-storage/tests/common/mod.rs
git commit -m "feat(graphql): Viewer.reciprocity resolver with seeded integration test"
```

---

### Task L6.4: Storage-client TS query document + types

**Files:**
- Modify: `elohim/sdk/storage-client-ts/src/graphql/queries.ts`
- Modify: `elohim/sdk/storage-client-ts/tests/graphql-queries.test.ts`

- [ ] **Step 1: Add the `viewerReciprocity` document**

Append to `elohim/sdk/storage-client-ts/src/graphql/queries.ts`:

```ts
export const VIEWER_RECIPROCITY_QUERY = /* GraphQL */ `
  query ViewerReciprocity($agentCid: ID!) {
    viewer(agentCid: $agentCid) {
      reciprocity {
        agentCid
        inflowByReach {
          reach
          committedBytes
          deliveredBytes
          commitmentCount
        }
        outflowByReach {
          reach
          committedBytes
          deliveredBytes
          commitmentCount
        }
        byHousehold {
          householdId
          displayName
          committedBytes
          deliveredBytes
        }
      }
    }
  }
`;

export interface ViewerReciprocityResponse {
  viewer: {
    reciprocity: ReciprocityViewWire;
  };
}

export interface ReciprocityViewWire {
  agentCid: string;
  inflowByReach: ReciprocityFlowWire[];
  outflowByReach: ReciprocityFlowWire[];
  byHousehold: HouseholdReciprocityWire[];
}

export interface ReciprocityFlowWire {
  reach: 'self' | 'intimate' | 'qahal' | 'community' | 'global';
  committedBytes: number;
  deliveredBytes: number;
  commitmentCount: number;
}

export interface HouseholdReciprocityWire {
  householdId: string;
  displayName: string | null;
  committedBytes: number;
  deliveredBytes: number;
}
```

- [ ] **Step 2: Add query-shape test**

Mirror the existing `graphql-queries.test.ts` pattern (verify exact assertion shape at execution time):

```ts
describe('VIEWER_RECIPROCITY_QUERY', () => {
  it('selects all reciprocity fields', () => {
    const sdl = VIEWER_RECIPROCITY_QUERY;
    ['agentCid', 'inflowByReach', 'outflowByReach', 'byHousehold',
     'reach', 'committedBytes', 'deliveredBytes', 'commitmentCount',
     'householdId', 'displayName'].forEach(field => {
      expect(sdl).toContain(field);
    });
  });
});
```

- [ ] **Step 3: Run TS tests**

```bash
cd /projects/elohim
pnpm --filter @elohim/storage-client test 2>&1 | tail -20
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add elohim/sdk/storage-client-ts/src/graphql/queries.ts \
        elohim/sdk/storage-client-ts/tests/graphql-queries.test.ts
git commit -m "feat(sdk): VIEWER_RECIPROCITY_QUERY document + response types"
```

---

### Task L6.5: Flip `useGraphqlTopology` flag

**Files:**
- Modify: `app/elohim-app/src/environments/environment.ts`
- Modify: `app/elohim-app/src/environments/environment.prod.ts`

- [ ] **Step 1: Verify Vitest parity is green**

```bash
cd /projects/elohim/app/elohim-app
pnpm exec vitest run --config vite.config.ts shefa/services/topology-parity 2>&1 | tail -20
```

Expected: PASS. This is the green-gate that lets us retire REST.

- [ ] **Step 2: Flip the flag in dev environment**

In `app/elohim-app/src/environments/environment.ts:29`, change:

```ts
useGraphqlTopology: false,
```

to:

```ts
useGraphqlTopology: true,
```

- [ ] **Step 3: Flip the flag in prod environment**

In `app/elohim-app/src/environments/environment.prod.ts`, change the same line.

- [ ] **Step 4: Run the cluster + peer-topology service specs to verify they pass with GraphQL transport**

```bash
cd /projects/elohim/app/elohim-app
pnpm exec vitest run --config vite.config.ts \
  shefa/services/cluster.service.spec \
  shefa/services/peer-topology.service.spec 2>&1 | tail -30
```

Expected: PASS for both. (These specs branch on the flag; they must already handle `true`.)

- [ ] **Step 5: Run the full Angular test suite**

```bash
cd /projects/elohim/app/elohim-app
pnpm test 2>&1 | tail -30
```

Expected: PASS. Note any regressions; if any spec depended on REST-mode behavior that's no longer reachable, that spec is dead code — file a follow-up issue rather than fix here.

- [ ] **Step 6: Commit**

```bash
git add app/elohim-app/src/environments/environment.ts \
        app/elohim-app/src/environments/environment.prod.ts
git commit -m "feat(shefa): flip useGraphqlTopology=true — retire REST topology fallback"
```

---

### Task L6.6: Angular reciprocity service

**Files:**
- Create: `app/elohim-app/src/app/shefa/services/reciprocity.service.ts`
- Create: `app/elohim-app/src/app/shefa/services/reciprocity.service.spec.ts`

- [ ] **Step 1: Study the cluster.service.ts post-flag pattern**

```bash
sed -n '1,80p' app/elohim-app/src/app/shefa/services/cluster.service.ts
```

`reciprocity.service.ts` will NOT carry a REST fallback — Viewer.reciprocity is GraphQL-only from day one. Drop the `if (useGraphqlTopology)` branch entirely.

- [ ] **Step 2: Write the service**

Create `app/elohim-app/src/app/shefa/services/reciprocity.service.ts`:

```ts
import { Injectable, inject, signal } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import type { ReciprocityViewWire, ViewerReciprocityResponse } from '@elohim/storage-client/graphql';
import { VIEWER_RECIPROCITY_QUERY } from '@elohim/storage-client/graphql';
import { SessionService } from '@app/imagodei';

@Injectable({ providedIn: 'root' })
export class ReciprocityService {
  private readonly http = inject(HttpClient);
  private readonly session = inject(SessionService);

  readonly reciprocity = signal<ReciprocityViewWire | null>(null);
  readonly loading = signal(false);
  readonly error = signal<string | null>(null);

  async load(): Promise<ReciprocityViewWire> {
    this.loading.set(true);
    this.error.set(null);
    try {
      const agentCid = this.session.requireAgentCid();
      const response = await firstValueFrom(
        this.http.post<{ data: ViewerReciprocityResponse }>('/api/v1/graphql', {
          query: VIEWER_RECIPROCITY_QUERY,
          variables: { agentCid },
        }),
      );
      const view = response.data.viewer.reciprocity;
      this.reciprocity.set(view);
      return view;
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      this.error.set(msg);
      throw e;
    } finally {
      this.loading.set(false);
    }
  }
}
```

If `@app/imagodei` doesn't export `SessionService.requireAgentCid()`, verify the exact name at execution time by grepping `^export class SessionService` and look for the agent-cid accessor — could be `currentAgentCid`, `agentCid()`, etc. Use whichever the cluster.service.ts uses.

- [ ] **Step 3: Write the spec**

Create `app/elohim-app/src/app/shefa/services/reciprocity.service.spec.ts`:

```ts
import { TestBed } from '@angular/core/testing';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';
import { ReciprocityService } from './reciprocity.service';
import { SessionService } from '@app/imagodei';

describe('ReciprocityService', () => {
  let service: ReciprocityService;
  let httpMock: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: SessionService, useValue: { requireAgentCid: () => 'agent-matthew-cid' } },
      ],
    });
    service = TestBed.inject(ReciprocityService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => httpMock.verify());

  it('posts the GraphQL query and stores the response', async () => {
    const promise = service.load();
    const req = httpMock.expectOne('/api/v1/graphql');
    expect(req.request.method).toBe('POST');
    expect(req.request.body.variables).toEqual({ agentCid: 'agent-matthew-cid' });
    req.flush({
      data: {
        viewer: {
          reciprocity: {
            agentCid: 'agent-matthew-cid',
            inflowByReach: [
              { reach: 'intimate', committedBytes: 100, deliveredBytes: 100, commitmentCount: 1 },
            ],
            outflowByReach: [],
            byHousehold: [
              { householdId: 'household-terrance', displayName: 'Terrance', committedBytes: 100, deliveredBytes: 100 },
            ],
          },
        },
      },
    });
    const view = await promise;
    expect(view.inflowByReach.length).toBe(1);
    expect(service.reciprocity()).toEqual(view);
  });

  it('captures error state on HTTP failure', async () => {
    const promise = service.load();
    httpMock.expectOne('/api/v1/graphql').error(new ProgressEvent('network'));
    await expect(promise).rejects.toBeDefined();
    expect(service.error()).not.toBeNull();
  });
});
```

- [ ] **Step 4: Run the spec**

```bash
cd /projects/elohim/app/elohim-app
pnpm exec vitest run --config vite.config.ts shefa/services/reciprocity.service.spec 2>&1 | tail -20
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add app/elohim-app/src/app/shefa/services/reciprocity.service.ts \
        app/elohim-app/src/app/shefa/services/reciprocity.service.spec.ts
git commit -m "feat(shefa): ReciprocityService — GraphQL-only viewer.reciprocity client"
```

---

### Task L6.7: /shefa/reciprocity page component + route

**Files:**
- Create: `app/elohim-app/src/app/shefa/pages/reciprocity/reciprocity.component.ts`
- Create: `app/elohim-app/src/app/shefa/pages/reciprocity/reciprocity.component.html`
- Create: `app/elohim-app/src/app/shefa/pages/reciprocity/reciprocity.component.scss`
- Create: `app/elohim-app/src/app/shefa/pages/reciprocity/reciprocity.component.spec.ts`
- Modify: shefa route table + `shefa-sidenav.component`

- [ ] **Step 1: Locate the shefa routes file**

```bash
find app/elohim-app/src/app/shefa -name "*routes*" -o -name "*module*" 2>&1 | head
grep -rn "/shefa/cluster" app/elohim-app/src/app/shefa 2>&1 | grep -iE "(route|path)" | head
```

The route table is wherever `/shefa/cluster` and `/shefa/peers` are declared. Add `/reciprocity` alongside.

- [ ] **Step 2: Write the page component**

Create `app/elohim-app/src/app/shefa/pages/reciprocity/reciprocity.component.ts`:

```ts
import { Component, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ReciprocityService } from '../../services/reciprocity.service';

@Component({
  selector: 'app-reciprocity-page',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './reciprocity.component.html',
  styleUrl: './reciprocity.component.scss',
})
export class ReciprocityComponent implements OnInit {
  readonly reciprocityService = inject(ReciprocityService);

  ngOnInit(): void {
    void this.reciprocityService.load();
  }
}
```

- [ ] **Step 3: Write the template**

Create `app/elohim-app/src/app/shefa/pages/reciprocity/reciprocity.component.html`:

```html
<section class="reciprocity-page" data-testid="reciprocity-page">
  <header>
    <h1>Reciprocity</h1>
    <p class="lede">Flows you've delivered to peers and flows peers have delivered to you, grouped by reach and by household.</p>
  </header>

  @if (reciprocityService.loading()) {
    <p class="loading" data-testid="reciprocity-loading">Loading…</p>
  } @else if (reciprocityService.error(); as err) {
    <p class="error" data-testid="reciprocity-error">{{ err }}</p>
  } @else if (reciprocityService.reciprocity(); as view) {
    <h2>Inflow — what peers have delivered to you</h2>
    <table data-testid="reciprocity-inflow-table">
      <thead>
        <tr>
          <th>Counterparty</th>
          <th>Committed (bytes)</th>
          <th>Delivered (bytes)</th>
        </tr>
      </thead>
      <tbody>
        @for (row of view.byHousehold; track row.householdId) {
          <tr data-testid="reciprocity-inflow-row">
            <td data-testid="reciprocity-counterparty">{{ row.displayName ?? row.householdId }}</td>
            <td data-testid="reciprocity-committed">{{ row.committedBytes }}</td>
            <td data-testid="reciprocity-delivered">{{ row.deliveredBytes }}</td>
          </tr>
        }
      </tbody>
    </table>

    <h2>By reach</h2>
    <ul class="by-reach">
      @for (flow of view.inflowByReach; track flow.reach) {
        <li>
          <strong>{{ flow.reach }}</strong>: {{ flow.deliveredBytes }} / {{ flow.committedBytes }} bytes delivered ({{ flow.commitmentCount }} commitment(s))
        </li>
      }
    </ul>
  }
</section>
```

The `data-testid` attributes are required for a2o step bindings (see `page-model` skill).

- [ ] **Step 4: Write a minimal stylesheet**

Create `app/elohim-app/src/app/shefa/pages/reciprocity/reciprocity.component.scss`:

```scss
.reciprocity-page {
  padding: 1.5rem;

  header {
    margin-bottom: 1.5rem;

    .lede { color: var(--text-secondary, #666); margin: 0.25rem 0 0; }
  }

  table {
    border-collapse: collapse;
    width: 100%;
    margin-bottom: 2rem;

    th, td { padding: 0.5rem 0.75rem; text-align: left; border-bottom: 1px solid var(--border, #eee); }
    th { font-weight: 600; }
  }

  .by-reach { list-style: none; padding: 0; }
  .by-reach li { padding: 0.25rem 0; }

  .loading, .error { padding: 1rem; }
  .error { color: var(--error, #c00); }
}
```

- [ ] **Step 5: Write the component spec**

Create `app/elohim-app/src/app/shefa/pages/reciprocity/reciprocity.component.spec.ts`:

```ts
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { signal } from '@angular/core';
import { ReciprocityComponent } from './reciprocity.component';
import { ReciprocityService } from '../../services/reciprocity.service';

describe('ReciprocityComponent', () => {
  let fixture: ComponentFixture<ReciprocityComponent>;
  let mockService: Partial<ReciprocityService>;

  beforeEach(async () => {
    mockService = {
      reciprocity: signal({
        agentCid: 'agent-matthew-cid',
        inflowByReach: [{ reach: 'intimate' as const, committedBytes: 100, deliveredBytes: 80, commitmentCount: 1 }],
        outflowByReach: [],
        byHousehold: [{ householdId: 'household-terrance', displayName: 'Terrance', committedBytes: 100, deliveredBytes: 80 }],
      }),
      loading: signal(false),
      error: signal(null),
      load: async () => mockService.reciprocity!()!,
    };

    await TestBed.configureTestingModule({
      imports: [ReciprocityComponent],
      providers: [{ provide: ReciprocityService, useValue: mockService }],
    }).compileComponents();

    fixture = TestBed.createComponent(ReciprocityComponent);
    fixture.detectChanges();
  });

  it('renders the inflow table with one row per household', () => {
    const rows = fixture.nativeElement.querySelectorAll('[data-testid="reciprocity-inflow-row"]');
    expect(rows.length).toBe(1);
    expect(fixture.nativeElement.querySelector('[data-testid="reciprocity-counterparty"]').textContent.trim()).toBe('Terrance');
  });

  it('falls back to householdId when displayName is null', () => {
    mockService.reciprocity = signal({
      agentCid: 'agent-x',
      inflowByReach: [],
      outflowByReach: [],
      byHousehold: [{ householdId: 'household-anon', displayName: null, committedBytes: 0, deliveredBytes: 0 }],
    });
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [ReciprocityComponent],
      providers: [{ provide: ReciprocityService, useValue: mockService }],
    });
    const f = TestBed.createComponent(ReciprocityComponent);
    f.detectChanges();
    expect(f.nativeElement.querySelector('[data-testid="reciprocity-counterparty"]').textContent.trim()).toBe('household-anon');
  });
});
```

- [ ] **Step 6: Register the route**

In the shefa routes file (located in Step 1), add an entry mirroring the `/shefa/cluster` pattern:

```ts
{
  path: 'reciprocity',
  loadComponent: () =>
    import('./pages/reciprocity/reciprocity.component').then(m => m.ReciprocityComponent),
}
```

- [ ] **Step 7: Add sidenav entry**

`app/elohim-app/src/app/shefa/components/shefa-sidenav/shefa-sidenav.component.{ts,html}` already shows recent modifications per `git status`. Add an entry for Reciprocity alongside Cluster and Peers, and update the matching `.spec.ts` to expect 3 items where it previously expected 2 (or whatever the current count is).

- [ ] **Step 8: Run all the new specs + sidenav spec**

```bash
cd /projects/elohim/app/elohim-app
pnpm exec vitest run --config vite.config.ts \
  shefa/pages/reciprocity \
  shefa/components/shefa-sidenav 2>&1 | tail -30
```

Expected: PASS.

- [ ] **Step 9: Start dev server and screenshot the page**

```bash
cd /projects/elohim/app/elohim-app
pnpm start 2>&1 &
# Wait ~30s for compilation; open http://localhost:4200/shefa/reciprocity in browser
```

Confirm the page renders with seeded fixture data (or `(empty)` placeholders if no data). Capture the screenshot in `/projects/elohim/.claude/screenshots/2026-05-19-reciprocity-page.png` for `/deliver`.

- [ ] **Step 10: Commit**

```bash
git add app/elohim-app/src/app/shefa/pages/reciprocity/ \
        app/elohim-app/src/app/shefa/shefa.routes.ts \
        app/elohim-app/src/app/shefa/components/shefa-sidenav/
git commit -m "feat(shefa): /shefa/reciprocity page + route + sidenav entry"
```

---

### Task L6.8: a2o step bindings + lift @wip

**Files:**
- Create: `genesis/a2o/features/shefa/steps/ui/reciprocity.steps.ts`
- Modify: `genesis/a2o/features/shefa/m1-matthew-terrance-delivery.feature`

- [ ] **Step 1: Inspect existing shefa step bindings for pattern**

```bash
ls genesis/a2o/features/shefa/steps/ 2>&1
cat genesis/a2o/features/shefa/steps/ui/peer-topology.steps.ts 2>/dev/null | head -60
```

If `steps/ui/` doesn't exist for shefa yet, mirror the pattern from `genesis/a2o/features/lamad/steps/ui/` or whichever pillar already uses Playwright steps.

- [ ] **Step 2: Write `reciprocity.steps.ts`**

Create the file with three step bindings for the scenario at `m1-matthew-terrance-delivery.feature:26-31`:

```ts
import { Then, When } from '@cucumber/cucumber';
import { E2EWorld } from '../../../framework/e2e-world';
import { requirePlaywright } from '../../../framework/playwright-device';

When('Matthew opens the reciprocity page at {string}', async function (this: E2EWorld, path: string) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  await device.goto(path);
});

Then('he sees at least one inflow row whose counterparty is {word}', async function (this: E2EWorld, expected: string) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  const counterparties = await device.page.locator('[data-testid="reciprocity-counterparty"]').allTextContents();
  if (!counterparties.some(c => c.trim() === expected)) {
    throw new Error(`expected counterparty ${expected}; got [${counterparties.join(', ')}]`);
  }
});

Then('the committed bytes column shows a non-zero value for that row', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  const cells = await device.page.locator('[data-testid="reciprocity-committed"]').allTextContents();
  if (!cells.some(c => Number(c) > 0)) throw new Error(`no non-zero committed bytes: [${cells.join(', ')}]`);
});

Then('the delivered bytes column shows a non-zero value once the cross-pod fetch has completed', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  await device.page.waitForFunction(() => {
    const cells = Array.from(document.querySelectorAll('[data-testid="reciprocity-delivered"]'));
    return cells.some(c => Number(c.textContent) > 0);
  }, { timeout: 30_000 });
});
```

Verify exact import paths for `E2EWorld` and `requirePlaywright` from a neighboring step file — the conventions are not absolute, they're learned by example.

- [ ] **Step 3: Lift @wip on the reciprocity scenario**

In `genesis/a2o/features/shefa/m1-matthew-terrance-delivery.feature` line 26, delete the `@wip` tag on the "Reciprocity page shows inflow from Terrance" scenario. Cluster + peers scenarios remain `@wip` for now — those depend on the seeder having real `household-terrance` data; that gate is delivery work, not part of this plan.

- [ ] **Step 4: Run the a2o scenario locally (if Playwright stack is up)**

```bash
cd /projects/elohim/app/elohim-app
pnpm run cypress:run -- --spec 'genesis/a2o/features/shefa/m1-matthew-terrance-delivery.feature' 2>&1 | tail -30
```

If the local stack isn't running, document the gate as "pending CI green on Jenkins genesis pipeline" and proceed to commit. (Per memory `feedback_shift_measure_jenkins`, the truth measure is Jenkins, not local.)

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/features/shefa/steps/ui/reciprocity.steps.ts \
        genesis/a2o/features/shefa/m1-matthew-terrance-delivery.feature
git commit -m "test(a2o): reciprocity scenario steps + lift @wip on m1 reciprocity row"
```

---

### Task L7.1: Author Qahal Collective+Membership DHT design spec

**Files:**
- Create: `genesis/docs/superpowers/specs/2026-05-19-qahal-collective-membership-dht-design.md`

This task is design-only. No code is written. The output is one design document that the deferred Wave 3 hREA interop brainstorm will consume.

- [ ] **Step 1: Confirm zome inventory headroom**

The P2P design gate (`.claude/skills/p2p-design-gate/SKILL.md`) flags that the lamad DNA is at ~73/100 entry types and mishpat at ~11/100. Verify before assuming headroom for *elohim* DNA (where qahal lives):

```bash
grep -rn "EntryTypes\|entry_def\b\|#\[hdk_entry_helper" elohim/holochain/dna/elohim/zomes/ 2>&1 | grep -iE "(content_store|relationship|collective|membership)" | head -30
```

The spec MUST quote the current entry-type count for the elohim DNA and the headroom available.

- [ ] **Step 2: Survey what already exists for qahal-shaped entities**

```bash
grep -rn "Collective\|Membership\|Affiliation\|Group" elohim/holochain/dna/elohim/zomes/ 2>&1 | grep -v "test\|target/" | head -30
```

If `Collective`/`Membership` partial stubs already exist, the spec must note the existing state and propose evolution rather than green-field creation.

- [ ] **Step 3: Write the spec**

Create `genesis/docs/superpowers/specs/2026-05-19-qahal-collective-membership-dht-design.md` with this skeleton — every section must be filled with concrete content, not placeholders:

```markdown
# Qahal Collective + Membership DHT Design

**Status:** Design (pre-implementation). Implementation deferred to a follow-on plan that consumes this spec.
**Date:** 2026-05-19
**Author:** [from session]
**Source docs:**
- `genesis/docs/plans/2026-05-19-topology-resilience-qahal-synthesis.md` §Epic E
- `/projects/research/vf-graphql/lib/schemas/agent.gql` — canonical ValueFlows Agent / AgentRelationship
- `.claude/skills/p2p-design-gate/SKILL.md`
- `.claude/memory/project_collective_is_stewardship_unit.md` (note: load if exists; if not, omit)

## 1. Problem statement

Epic E of the synthesis plan promises a `/qahal/collective/:id` page with members, stewards, live co-presence, upcoming activities, norms, and contribution-recognition (slide 45). All five facets must be backed by P2P-native DHT entries. Today's DHT does not have first-class `Collective` or `Membership` entries; the topology view derives household groupings from `HumanRelationship` edges, which doesn't compose for arbitrary collectives (congregations, learning cohorts, civic groups). This spec proposes the missing entries.

## 2. P2P Design Gate — answers

(Per the gate skill, this section is mandatory and answers four questions before any HTTP route is sketched.)

### 2.1 Source-of-truth classification

| Entity | Category | Rationale |
|---|---|---|
| `Collective` | A — notarized | Identity-of-record; must be discoverable; rejoinable after key recovery. |
| `Membership` | A — notarized | Authority-bearing (a steward designation is a governance act). |
| `MembershipRole` (steward, contributor, observer) | derived via link kind | Encoded as link type from Person → Collective; no separate entry. |
| `ActivityPresence` (live co-presence) | C — operational | Heartbeat-shaped; rebuilt from libp2p signals; not notarized. |

### 2.2 Existing entry types — does a fit already exist?

Quote the audit from Step 1. State whether `Membership` can collapse onto an existing relationship entry or needs its own type.

### 2.3 Identity scheme

- `Collective.id` = content CID derived from `{founder_agent_cid, charter_text, created_at_block_height}`. No slugs. Display name is mutable; id is immutable.
- `Membership.id` = content CID over `{person_cid, collective_cid, role, joined_at, sponsor_cid_or_self}`.

### 2.4 Coordinator function map

| Operation | Coordinator zome | Validation gate |
|---|---|---|
| `create_collective(charter)` | `qahal_coordinator` | Charter must be non-empty, founder must sign, no collisions on CID. |
| `request_membership(collective_cid, role)` | `qahal_coordinator` | Person CID must match call origin; role ∈ {contributor, observer}; steward role requires sponsor. |
| `attest_membership(membership_cid)` | `qahal_coordinator` | Caller must be an existing steward of the collective. |
| `revoke_membership(membership_cid, reason)` | `qahal_coordinator` | Caller must be steward; reason must be non-empty; emits ContentSignal for projection. |

## 3. Entry shapes (HDK)

(Sketch the Rust struct + `#[hdk_entry_helper]` + validation function. Reference `elohim/holochain/dna/elohim/zomes/imagodei_integrity/src/relationship.rs` for the relationship-entry pattern.)

```rust
#[hdk_entry_helper]
#[derive(Clone, Debug, PartialEq)]
pub struct Collective {
    pub founder_agent_cid: String,
    pub charter: String,              // markdown; max 16 KiB
    pub display_name: String,         // mutable via update_collective
    pub created_at_block_height: u64,
}

#[hdk_entry_helper]
#[derive(Clone, Debug, PartialEq)]
pub struct Membership {
    pub person_cid: String,
    pub collective_cid: String,
    pub role: MembershipRole,
    pub sponsor_cid: Option<String>,  // None ⇒ self-joined (open collective)
    pub joined_at_block_height: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MembershipRole {
    Steward,
    Contributor,
    Observer,
}
```

## 4. Link types

- `Person → Collective` (link kind: `MemberOf`) — outbound discovery from person.
- `Collective → Person` (link kind: `HasMember`) — inbound member iteration.
- `Collective → Membership` (link kind: `HasMembership`) — for ordered iteration with role + timestamp.

## 5. Validation rules

Quote `feedback_serde_json_value_breaks_zome_boundary` — no `serde_json::Value` at the SerializedBytes boundary. Quote `project_hdi_no_get_links_in_validators` — integrity uses `must_get_*` only.

| Rule | Where enforced |
|---|---|
| `Collective.charter` non-empty | `validate_create_collective` |
| Membership steward requires sponsor | `validate_create_membership` |
| Revoke caller must be steward | coordinator-side; integrity defers to `must_get_agent_activity` |

## 6. hREA / ValueFlows mapping

Cross-reference `/projects/research/vf-graphql/lib/schemas/agent.gql`:

| qahal entity | VF/hREA counterpart | Mapping notes |
|---|---|---|
| `Collective` | `Organization implements Agent` | `Organization.id` ↔ Collective CID; `Organization.name` ↔ `display_name`; `Organization.note` ↔ `charter`. |
| `Membership` (steward role) | `AgentRelationship { object → Organization, subject → Person, relationship: AgentRelationshipRole(Steward) }` | Reuse VF `AgentRelationshipRole` vocabulary; steward = "manages" or "governs" role. |
| `Membership` (contributor role) | `AgentRelationship { ..., relationship: AgentRelationshipRole(Contributor) }` | Plain participation. |
| `MembershipRole` enum | `AgentRelationshipRole` instances | We define our enum but expose role names that line up with VF. |

The doorway VF-GraphQL projection (deferred to Wave 3) would query the qahal coordinator for collectives + memberships and present them as `Organization` + `AgentRelationship`.

## 7. What this spec deliberately does NOT do

- Does not propose a `Group` entry separate from `Collective` — one notarized type covers congregations, cohorts, civic clusters, households-as-collectives.
- Does not introduce a coordinator function for `merge_collective` / `split_collective` — governance scope, deferred.
- Does not specify the on-wire shape for live co-presence (that's libp2p ephemeral state, not DHT).
- Does not commit to an Angular surface — that's Epic E.

## 8. Open questions for follow-on planning

- Should `MembershipRole::Steward` carry an attestation chain (sponsored by N existing stewards, threshold M)? Quorum design deferred.
- Charter immutability vs mutability via update-entry — what's the integrity invariant?
- Inheritance: does a collective have a parent? (Households-as-collectives may want a Household ⊃ neighborhood-collective relation.)
- Open / invitation-only / application-only collective gates — encode as `Collective.access_policy` field, or as a separate `AccessPolicy` entry?

## 9. Implementation handoff

Implementation lands in a follow-on plan dispatched after the deferred Wave 3 hREA interop brainstorm. That brainstorm validates the VF-GraphQL mapping above and decides whether to ship the coordinator zome on its own or co-bundle with the VF-GraphQL projection.
```

The skeleton above is fully filled. Adjust any field names or zome paths against actual repo state at execution time. Do not leave a "TBD" anywhere.

- [ ] **Step 4: Validate the spec against the P2P design gate skill**

```bash
cat .claude/skills/p2p-design-gate/SKILL.md | head -80
```

Read through the four questions the gate requires answered (Category, existing entry type fit, identity, coordinator function). The spec §2 must answer all four. If any is missing, fill it before commit.

- [ ] **Step 5: Commit**

```bash
git add genesis/docs/superpowers/specs/2026-05-19-qahal-collective-membership-dht-design.md
git commit -m "docs(spec): qahal Collective + Membership DHT design — substrate groundwork for Epic E + Wave 3 hREA interop"
```

---

## Out of scope (call-outs)

- Renaming internal Rust view types (`MyClusterView` → `HubViewInternal`, `MyTopologyView` → `PeerTopologyInternal`, plus rename of internal fields like `myCidsHostedByThem` → `cidsHostedByPeer`). Blast radius: ~10 files, many test fixtures. Filed as a follow-on hygiene pass; no behavior change so it's cheap to do once stewardship-aligned naming is settled by use.
- The Cypress feature `topology-graphql-parity.feature` referenced in the parent synthesis plan §A7 does not exist in tree; the Vitest unit-parity at `app/elohim-app/src/app/shefa/services/topology-parity.spec.ts` is the green-gate this plan relies on. If a Cypress E2E gate is later wanted, that's its own task.
- Reciprocity historical rollup / time-series view — out of scope; the page renders current totals only.
- Implementation of L7's `Collective`/`Membership` entry types — design-only this sprint.
- Wave 3 hREA / VF-GraphQL interop brainstorm — deferred per the user's instruction; happens after this sprint closes.

## Self-review

Spec coverage: every numbered item in the user's three-item ask is mapped:
1. Flip `useGraphqlTopology` → Task L6.5.
2. Land Epic B (`Viewer.reciprocity` resolver + `/shefa/reciprocity` page) → Tasks L6.1 through L6.8.
3. Wave 3 framing decision → explicitly deferred; L7 (Task L7.1) lays substrate groundwork that the deferred brainstorm will need.

Placeholder scan: no `TODO`/`TBD`/"fill in" markers. The one `todo!()` in L6.2 Step 3 is annotated with the exact thing to write (Diesel filter/group_by/select using existing patterns from neighboring builders).

Type consistency: `ReciprocityView` / `ReciprocityFlow` / `HouseholdReciprocity` are the three view types used throughout L6. Field names are stable across schema (L6.1), Rust struct (L6.2), resolver (L6.3), TS client (L6.4), service (L6.6), component (L6.7), and steps (L6.8).

## Execution shape

Recommended: subagent-driven execution, one task per dispatch, review between L6.X tasks. L7.1 can run in parallel with L6.5 (flag flip) and L6.8 (a2o) since L7 touches no shared files.
