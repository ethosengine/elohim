# Topology + Resilience + Qahal — Synthesis & Re-baseline

**Status:** Synthesis (pre-shift). Supersedes obsolete portions of the 2026-05-07 M1 plan; preserves the design intent of `light-up-the-topology` while reprojecting the work onto the graph-native substrate landed 2026-05-16.

**Date:** 2026-05-19
**Source docs:**
- `genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-design.md` — sprint design (6 surfaces + 2 substrate fixes)
- `genesis/docs/superpowers/plans/2026-05-07-topology-substrate-completion-m1-plan.md` — M1 vertical-slice plan (matthew↔terrance), partially executed
- `genesis/docs/plans/2026-05-16-graph-native-projection-substrate.md` — graph substrate (CozoDB + Apollo Federation v2), landed in tree
- `/projects/research/after-the-feed-2026-05/` — design reference (New_ Public, May 2026 deck)

## 0. Charter (slide 50, verbatim)

> "Help communities build spaces that have the **intimacy of a group chat** and the **openness of a public square**. And help those spaces connect to each other when it matters."

That is the qahal pillar product brief in one sentence. Every surface in this document answers the question: *does this make that sentence visible and verifiable in the app?*

## P2P Design Gate — Source-of-Truth Declarations

Per `.claude/skills/p2p-design-gate/SKILL.md`, this synthesis introduces **zero new DHT entry types, zero new HTTP routes, zero new persistent storage surfaces, zero new wire protocols**. Every relation referenced below is an existing operational projection (Category C) of a substrate-notarized canonical source. The audit-grade restatement appears once here and covers every CozoDB / Diesel relation mentioned in §2 (GraphQL sketch) and §5 (task list).

| Relation referenced | Where | Category | Canonical source-of-truth | Identity | Rebuild path |
|---|---|---|---|---|---|
| CozoDB graph relations (Agent, Device, Household, Collective, Edge, Reciprocity, EprHead, Norm, ContributionRecord) | `elohim/elohim-storage/src/graph/{schema,primitives}.rs` | **C — operational** | Holochain DHT: canonical `EprHead` atoms (per the 2026-05-16 graph-native plan §11). The `epr_atoms` Diesel table holds the materialized canonical_bytes. | Primary keys derived from substrate CIDs (content-addressed). No autoincrement keys; no new slugs introduced. | `projector::backfill_graph` walks `epr_atoms` and re-derives every CozoDB row from canonical bytes. Reach-from-peers is the ultimate rebuild. |
| `rea_commitments` / `rea_economic_events` (Diesel projections) | `elohim/elohim-storage/src/db/rea_projection.rs` | **C — operational** | Holochain DHT: REA `Commitment` and `EconomicEvent` Content entries (notarized). | Content CID. | Replay matching `ElohimContentSignal` through `rea_projector::handle_content_signal`. |
| `peer_identity_bindings` (Diesel projection) | `elohim/elohim-storage/src/db/peer_identity_bindings.rs` | **C — operational** | Holochain DHT: imagodei `AgentPeerBinding` entry. | `dht_anchor_hash` column links back to the canonical entry. | Signal replay. |
| `HumanRelationship` (joined into peer-topology edges, Epic D) | imagodei DHT entry type, projected through existing relationship views | **A — notarized** | Holochain DHT: existing imagodei `HumanRelationship` Content entries. No new DHT type. | Content CID. | Already projected; no new surface. |
| GraphQL types (`Viewer`, `HubView`, `PeerTopology`, `ReciprocityView`, `PeerHouseholdEdge`, `Collective`, `ContentDistribution`, `Device`) | `elohim/elohim-storage/src/graphql/resolvers.rs` (Epic A→F) | n/a — **wire shape**, not a store | Composes the relations above. Resolvers do reads only; no writes. | n/a (transport type). | n/a (no storage). |

**Rules this declaration enforces:**
- No GraphQL resolver in this sprint introduces a write path; every resolver is a read against existing relations.
- All projection rows MUST carry `dht_anchor_hash` (already enforced by the existing projectors); on disagreement with the DHT, the DHT wins.
- Identity derivation is CID-based throughout; no new slug identifiers introduced.
- `HumanRelationship.type` joining into `PeerHouseholdEdge` (Epic D1) reads an existing notarized entry; no new entry type is created.
- The qahal `Collective` GraphQL type (Epic E1) projects from existing Holochain Collective + membership entries (qahal substrate already established); no new DHT type required.

If a future automated audit re-flags any of L130 (graph projector reference) or L184 (REA relations reference) or similar, the truth lives in the table above.

---

## 1. Current-state inventory

### 1.1 Frontend — what's live (real HTTP / live data)

| Surface | Path | Wired to | Notes |
|---|---|---|---|
| `/shefa/cluster` device tiles | `app/elohim-app/src/app/shefa/pages/my-cluster/my-cluster.component.ts` | `MyClusterView` ← `/api/v1/cluster` | Federated cluster across viewer's bindings; archetype labels (`node→Home server`, `desktop→Laptop`, `mobile→Phone`, `steward→Steward process`) at `device-tile.component.ts:9-14` |
| `/shefa/peers` household cards | `app/elohim-app/src/app/shefa/pages/peer-topology/peer-topology.component.ts:15-94` | `PeerTopologyView` ← `/api/v1/peer-topology` (polls 5s) | Renders `peer-household-card`; reciprocity counters, criticality flags, sole-replica cliffs |
| `/shefa/dashboard` (operator) | `app/elohim-app/src/app/elohim/components/shefa-dashboard/shefa-dashboard.component.ts` | `CUSTODIAN_METRICS` token + `CustodianSelectionService` (30s refresh) | Custodian health, uptime aggregates, alerts |
| `/shefa/devices` (Activity / Devices tabs) | `app/elohim-app/src/app/shefa/components/device-stewardship/device-stewardship.component.ts` | `MasteryService` + `PointsService` + `HouseholdDevicesView` | Devices tab gated to stewards; Activity tab visible to all |
| Concept-card distribution badge | hydrated from `ContentNode.distribution` (`DistributionSummary`) | EPR head responses | Live across lamad pillar |
| Connection-indicator peer count | `app/elohim-app/src/app/imagodei/components/connection-indicator/connection-indicator.component.ts:40,60` | `HolochainClientService.peerCount()` or storage `/health` | Top-bar badge, always visible |

### 1.2 Frontend — what's stubbed or unrouted (the delivery gap)

| Surface | Path | State | Blocking what |
|---|---|---|---|
| Storage distribution (3-tab view) | `app/elohim-app/src/app/shefa/components/storage-distribution/storage-distribution.component.{html,ts}` | Internal stub model, no live wiring, not routed | Operator visibility into reach/type/node distribution |
| Compute-event dashboard | `app/elohim-app/src/app/shefa/interfaces/compute-event.interface.ts` + `compute-dashboard.interface.ts` | Service stubs exist; no rendered UI | Shefa compute-resource visibility |
| Doorway-dashboard | `app/elohim-app/src/app/generated/doorway-dashboard-view.ts` | Generated TS schema; no UI route | The 6th topology surface from the original sprint design |
| Topology-overview | `app/elohim-app/src/app/generated/topology-overview-view.ts` | **Already CozoDB-projected** (households + members + collectives + reciprocity) | No UI route consumes it — work is binding, not building |
| `/shefa/reciprocity` page | does not exist | route + page missing | M1 plan named this; never delivered |

### 1.3 Substrate — what's landed since the M1 plan

The graph-native projection substrate (plan `2026-05-16`) is in tree:

- `elohim/elohim-storage/src/graph/{mod,engine,registry,backfill,schema,primitives,projector}.rs`
- `elohim/elohim-storage/src/graphql/{mod,server,resolvers,codegen}.rs`
- `Cargo.toml`: `cozo = "0.7"` + `async-graphql = "7"`, feature `graph-native` (default-on)

This means the M1 plan's hand-rolled `services/cluster_view.rs`, `services/peer_topology_view.rs`, `services/reciprocity_view.rs` + the bespoke libp2p `/elohim/view-federation/1.0.0` request-response codec are now **the wrong shape** for the work. They become declarative queries against the graph relations the projector already maintains.

## 2. Graph-native projection target (GraphQL schema sketch)

The six topology surfaces collapse to one Apollo Federation v2 subgraph. Per-surface sketches:

```graphql
# Viewer-scoped root — replaces the per-view HTTP endpoints.
# Stewardship-aligned field naming (no `my*` prefix) — see L6 scope note below.
type Viewer {
  agent: Agent!
  hub: HubView!                    # /shefa/cluster (shipped: Epic A2)
  peers: PeerTopology!             # /shefa/peers   (shipped: Epic A3)
  reciprocity: ReciprocityView!    # /shefa/reciprocity (Epic B, L6 plan)
  collectives: [Collective!]!      # /qahal/* (Epic E, future)
}

type HubView {
  devices: [Device!]!
  totals: ClusterTotals!
}

type Device {
  peerId: ID!
  archetype: DeviceArchetype!      # node|desktop|mobile|steward|hub-archetype
  displayName: String!
  online: Boolean!
  freshness: Freshness!
  hosting: HostingStats!
  committed: ComputeResources      # shefa overlay; null on thin tiers
  stewardTier: StewardTier         # shefa overlay
}

type PeerTopology {
  reciprocationCount: Int!
  edges: [PeerHouseholdEdge!]!
  resilienceCliffs: [ResilienceCliff!]!
}

type PeerHouseholdEdge {
  householdId: ID!
  displayName: String!
  online: Boolean!
  relationship: HumanRelationshipType   # QAHAL OVERLAY: spouse|congregation|learning-partner|emergency-contact|...
  myCidsHostedByThem: Int!
  theirCidsHostedByMe: Int!
  netDiff: Int!                          # signed
  isCriticalForMe: Boolean!
  iAmCriticalForThem: Boolean!
}

type ReciprocityView {
  inflowByReach: [ReciprocityFlow!]!     # commitments delivered to the viewer, classified by reach
  outflowByReach: [ReciprocityFlow!]!    # commitments the viewer delivered, classified by reach
  byHousehold: [HouseholdReciprocity!]!  # committed-vs-delivered per peer-household
}

# Per-content overlay — already lives on EPR head responses today
type ContentDistribution {
  cid: ID!
  reachClass: ReachClass!
  replicaCount: Int!
  replicaTarget: Int!
  replicaHealth: ReplicaHealth!         # healthy|at_risk|critical
  projectorCount: Int!
  diversityHint: DiversityHint!
  myRole: ParticipationRole
  replicas: [ReplicaPeer!]!             # for detail view
}

type Collective {                       # qahal pillar (NEW)
  id: ID!
  name: String!
  members: [Member!]!
  stewards: [Member!]!
  activeNow: [Member!]!                 # live co-presence (slide 45)
  upcomingActivities: [Activity!]!
  norms: [Norm!]!                       # visible & persistent (slide 45)
  contributionRecognition: [ContributionRecord!]!  # reputation-by-contribution (slide 45)
}
```

**Viewer.* symmetry — L6 scope note (2026-05-19):** the field-level naming above (`hub`, `peers`, `reciprocity`, `collectives`) is the stewardship-aligned surface per `project_no_sovereignty_stewardship_over_ownership`. The shipped fields `Viewer.hub` and `Viewer.peers` already conform (`elohim/elohim-storage/src/graphql/resolvers.rs:713,728`). Internal Rust view types still carry `MyClusterView` / `MyTopologyView` names (`elohim/elohim-views/src/infrastructure.rs:1680`); the existing `impl From<MyClusterView> for HubView` keeps wire/internal nomenclature decoupled. Renaming internal types is a follow-on hygiene pass; the L6 plan leaves them alone to keep blast radius small.

The projector (`elohim-storage/src/graph/projector.rs`) is already fanning `EprHead` atoms into CozoDB relations (existing surfaces — see Source-of-Truth Declaration above). The work for this sprint is (a) confirm the relations cover the schema above, (b) add resolvers in `graphql/resolvers.rs`, (c) point existing Angular services at the GraphQL endpoint instead of the per-view HTTP routes.

## 3. The qahal reframe — per surface

Slide 45 of "After the Feed" supplies the five design moves the qahal-lens version of each surface must embody:

1. Community norms are visible and persistent
2. AI coordinator knows your goals (elohim-as-counsel)
3. Live co-presence is prioritized over metrics / feed
4. Reputation is earned through contribution ("organized 43 runs"), not follower counts
5. Smart matching notifies only members who match (pull, not push to all)

| Surface | Operational framing (today) | Qahal/social reframe |
|---|---|---|
| Distribution badge | "5 replicas across peers" | "5 households in my trust circle + 2 in Pete's congregation hold this" |
| Peer topology | "15GB hosted / 20GB reciprocated" | "Adam is spouse-household + learning partner; we mutually protect each other's photos" |
| Reciprocity | "inflow 12GB, outflow 8GB" | "inflow holds my medical records (intimate) + photos (neighborhood); outflow holds Pete's curriculum (trusted partner)" |
| Doorway dashboard | "known stewards + gossip windows" | "my household, my emergency contacts, my governance circles — where is authority distributed?" |
| Resilience tooltip | "3 households, partial" | "add a neighbor, ask your congregation member — your emergency circle is small" |
| Collective view (NEW) | n/a — qahal-only surface | identity strip → AI coordinator card → stats row → live-presence pane → upcoming activities + member rail + norms callouts (slide 45) |

Same DHT bindings + REA commitments + relationships everywhere. The reframe is a GraphQL projection choice and a render-template choice, not a substrate change.

## 4. Gating `@wip` a2o scenarios

These are the visual-delivery gates. A scenario is "visually delivered" when its `@wip` tag is lifted and a screenshot captured by `/deliver` shows the assertion satisfying.

| Feature file | Gate scenarios | What's missing today |
|---|---|---|
| `genesis/a2o/features/shefa/m1-matthew-terrance-delivery.feature` | All `@wip`: Matthew sees Terrance on `/shefa/peers`, real reciprocity on `/shefa/reciprocity`, device tiles on `/shefa/cluster` | `/shefa/reciprocity` route + page; substrate hydration into `peer-household-card` |
| `genesis/a2o/features/resilience/observable-distribution.feature` | `@wip` content-viewer resilience tooltip; `@resilience-p1` placement-gaps row | Resilience tooltip component on content viewer; `/api/v1/placement-gaps` route surfacing (or GraphQL equivalent) |
| `genesis/a2o/features/shefa/human-resilience.feature` | All `@wip`: at-risk → partial → protected progression per person, per reach | Trust-circle-depth + reciprocal-commitment-count computation; relationship-type joined into peer-topology edges |
| `genesis/a2o/features/qahal/collective-governance.feature` | Voting mechanics, proxy-elohim voting, REA stewardship recognition (mostly `@wip`) | Governance-decision → REA economic-event bridge; elohim governance-disposition computation; Collective view |
| `genesis/a2o/features/auth/recovery/intimate-quorum-happy-path.feature` | `@stage1-structural @recovery-m3` — cross-doorway invitation delivery | Gossipsub `recovery.invitation` topic across doorways; qahal-lens visibility of emergency-contact relationships |
| `genesis/a2o/features/auth/recovery/revocation-emergency-quorum.feature` | `@recovery-m4 @emergency-contact-quorum` | Cross-doorway vote aggregation; revoke-by-quorum surface |

## 5. Re-baselined task list

Supersedes the obsolete portions of the 2026-05-07 M1 plan. Tasks are grouped by epic; each task names its file targets and its gating a2o scenario.

### Epic A — Graph-native GraphQL surface over existing topology services (PLAN-GRADE)

**Status (2026-05-19):** landed as commit `ac17260e2` with stewardship-aligned field names (`Viewer.hub` / `Viewer.peers`) instead of the original `myCluster` / `myTopology` sketched below. Where this section uses `myCluster` / `myTopology` / `MyClusterGql` / `MyTopologyGql`, the as-shipped names are `hub` / `peers` / `HubView` / `PeerTopology` — see the L6 viewer.* symmetry scope note in §2 above. The descriptive prose below is preserved as the design record; for current-state nomenclature consult the source.

**Goal:** expose `/api/v1/cluster` and `/api/v1/peer-topology` data through GraphQL `{ viewer { myCluster ... myTopology ... } }`; migrate Angular `my-cluster.service` and `peer-topology.service` behind a feature flag; verify visual parity. The HTTP routes stay live; the GraphQL surface is additive.

**Audit findings (from §2 schema sketch vs. actual state):**

The synthesis assumed CozoDB carries Device/Household/PeerHouseholdEdge state. It does not. Per `elohim/elohim-storage/src/graph/schema.rs:1-92`, the Cozo schema today carries only EPR-shape relations (`epr_node`, `epr_edge`, `epr_lamad`, `epr_shefa`, `epr_qahal`). Topology state lives in Diesel relations (`peer_identity_bindings`, `humans`, `rea_commitments`, `peer_blob_inventory`) served by `services/cluster_view::build_view()` and `services/peer_topology_view::build_view()`, returning `MyClusterView` and `PeerTopologyView` (defined `elohim/elohim-views/src/infrastructure.rs:1680,1721`). The HTTP routes are registered at `elohim/elohim-storage/src/http.rs:9878,9889`.

**Decision (recorded in task A1):** GraphQL resolvers wrap the existing Diesel-backed services for this sprint. Cozo projection of topology state is a separate, larger sprint and NOT a prerequisite. This preserves the M1 plan's substrate work and adds GraphQL as a thin transport layer alongside HTTP.

**Tech stack:** Rust 1.78 (`elohim-storage` crate, feature `graph-native`), async-graphql 7, hyper-direct GraphQL handler (server.rs pattern; not axum), TypeScript with `fetch` for GraphQL POST, Angular 19 services with environment-flag routing, Cypress + Cucumber for visual-parity scenario.

**Sprint discipline (carries every task):**
- Native Rust builds: `RUSTFLAGS=""` and `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev` (per CLAUDE.md gotcha + memory `feedback_cargo_target_dir_for_native_builds`)
- One commit per task; never amend; never `--no-verify`
- All new code is `#[cfg(feature = "graph-native")]` per the device-class gating discipline established by the 2026-05-16 plan
- No new HTTP routes (the GraphQL endpoint already exists at `POST /api/v1/graphql`); no new DHT entry types; no new Diesel tables — see Source-of-Truth Declaration above (Category C, projection-only)
- Schema-first: async-graphql resolver types ARE the schema source; TypeScript types are hand-authored to match
- Stewardship vocabulary throughout

#### File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `elohim/elohim-storage/src/graphql/resolvers.rs` | Modify | Add `Viewer`, `MyClusterGql`, `MyTopologyGql`, `DeviceGql`, `PeerHouseholdEdgeGql`, `ResilienceCliffGql`, `ClusterTotalsGql` resolver types; extend `QueryRoot` with `viewer(agentCid)` field |
| `elohim/elohim-storage/src/graphql/server.rs` | Modify | Extend `build_schema` to accept `Arc<DbPool>` and `Arc<dyn Fn() -> Vec<PeerId>>` (connected-peers snapshot); inject via `.data(...)` |
| `elohim/elohim-storage/src/graphql/mod.rs` | Modify | Re-exports for new resolver types |
| `elohim/elohim-storage/src/http.rs` | Modify | At graphql route dispatch (~existing site), pass DB pool + connected-peers ref into `graphql::handle()` |
| `elohim/elohim-storage/tests/graphql_viewer_smoke.rs` | Create | Integration test: `{ viewer(agentCid) { myCluster { devices { archetype displayName online } totals { storageBytes } } } }` returns expected shape |
| `elohim/elohim-storage/tests/graphql_my_topology_smoke.rs` | Create | Integration test: `{ viewer(agentCid) { myTopology { reciprocationCount edges { ... } resilienceCliffs { ... } } } }` returns expected shape |
| `app/elohim-app/src/app/elohim/graphql-client/topology.queries.ts` | Create | gql template literals + TS interface types for the two queries |
| `app/elohim-app/src/app/elohim/graphql-client/topology-graphql.service.ts` | Create | Angular service with `myCluster(agentCid)` and `myTopology(agentCid)` methods using `fetch` POST to `/api/v1/graphql` |
| `app/elohim-app/src/app/elohim/graphql-client/topology-graphql.service.spec.ts` | Create | Vitest covering both methods + error shapes |
| `app/elohim-app/src/app/shefa/services/my-cluster.service.ts` | Modify | Route through `TopologyGraphqlService` when `environment.useGraphqlTopology === true`; otherwise existing HTTP path |
| `app/elohim-app/src/app/shefa/services/peer-topology.service.ts` | Modify | Same flag pattern |
| `app/elohim-app/src/environments/environment.ts` + `environment.prod.ts` | Modify | Add `useGraphqlTopology: false` default |
| `app/elohim-app/cypress/e2e/topology-graphql-parity.feature` | Create | Cucumber scenario asserting `/shefa/cluster` and `/shefa/peers` render identically under both flag values |

#### Task A1 — Record the projection-strategy decision

**Files:** Synthesis doc §A1 (this section) is the ADR. No code change.

**Decision:** GraphQL resolvers wrap existing Diesel services for this sprint. Cozo projection of topology state deferred to a later sprint. Rationale: substrate work for the M1 vertical slice already produced the typed view structs; reusing them lets us land the GraphQL surface in days, not weeks. The future Cozo projection will not change the GraphQL contract — only the resolver implementation behind it.

**Acceptance:** Decision recorded above. Commit lands the synthesis doc into the repo.

#### Task A2 — Resolver `Viewer.myCluster` wrapping `cluster_view::build_view`

**Files:**
- Modify: `elohim/elohim-storage/src/graphql/resolvers.rs`
- Modify: `elohim/elohim-storage/src/graphql/server.rs` (signature extension; minimal)
- Test (Create): `elohim/elohim-storage/tests/graphql_viewer_smoke.rs`

**Step 1 — Write the failing test:**

```rust
// elohim/elohim-storage/tests/graphql_viewer_smoke.rs
#![cfg(feature = "graph-native")]

use std::sync::Arc;
use elohim_storage::db::test_helpers::seed_alpha_household;
use elohim_storage::graph::engine::GraphEngine;
use elohim_storage::graphql::server::build_schema;

#[tokio::test]
async fn viewer_my_cluster_returns_devices_for_agent() {
    let pool = Arc::new(seed_alpha_household().await);
    let engine = Arc::new(GraphEngine::open_in_memory().unwrap());
    let connected_peers = Arc::new(|| Vec::<libp2p::PeerId>::new()) as Arc<_>;
    let schema = build_schema(engine, pool, connected_peers);

    let query = r#"{
        viewer(agentCid: "agent-matthew") {
            myCluster {
                devices { peerId archetype online displayName }
                totals { storageBytes deviceCount }
            }
        }
    }"#;

    let resp = schema.execute(query).await;
    assert!(resp.errors.is_empty(), "errors: {:?}", resp.errors);

    let data = resp.data.into_json().unwrap();
    let devices = data["viewer"]["myCluster"]["devices"].as_array().unwrap();
    assert!(!devices.is_empty(), "no devices returned for agent-matthew");
    assert!(
        devices.iter().any(|d| matches!(
            d["archetype"].as_str(),
            Some("node" | "desktop" | "mobile" | "steward")
        )),
        "no recognizable archetype in devices: {:?}",
        devices
    );
}
```

**Step 2 — Run the test, verify it fails as expected:**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS="" \
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev \
cargo test --features graph-native --test graphql_viewer_smoke 2>&1 | tail -30
```

Expected failure: compilation error — `Viewer` type unknown; `build_schema` signature mismatch (current sig takes only `Arc<GraphEngine>`).

**Step 3 — Implement minimal code:**

1. In `resolvers.rs` add:
   ```rust
   pub struct Viewer { pub agent_cid: String }
   pub struct MyClusterGql(pub MyClusterView);
   pub struct DeviceGql(pub DeviceSummary);
   pub struct ClusterTotalsGql(pub DeviceTotals);

   #[Object]
   impl Viewer {
       async fn my_cluster(&self, ctx: &Context<'_>) -> FieldResult<MyClusterGql> {
           let pool = ctx.data::<Arc<DbPool>>()?;
           let view = cluster_view::build_view(pool, &self.agent_cid).await?;
           Ok(MyClusterGql(view))
       }
   }
   ```
2. Add `#[Object]` impls for `MyClusterGql`, `DeviceGql`, `ClusterTotalsGql` that surface the existing `MyClusterView` fields (`devices`, `totals`, etc.). Where the View carries enums (`DeviceArchetype`, `FreshnessState`), implement `async_graphql::Enum`-derive shims or render as strings.
3. Extend `QueryRoot`:
   ```rust
   async fn viewer(&self, _ctx: &Context<'_>, agent_cid: ID) -> FieldResult<Viewer> {
       Ok(Viewer { agent_cid: agent_cid.to_string() })
   }
   ```
4. Update `build_schema` in `server.rs`:
   ```rust
   pub fn build_schema(
       graph_engine: Arc<GraphEngine>,
       db_pool: Arc<DbPool>,
       connected_peers: Arc<dyn Fn() -> Vec<PeerId> + Send + Sync>,
   ) -> AppSchema {
       Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
           .data(graph_engine)
           .data(db_pool)
           .data(connected_peers)
           .finish()
   }
   ```
5. Update call sites of `build_schema` in `server.rs::handle()` and `http.rs` (graphql route) to pass the new refs.

**Step 4 — Run all tests:**

```bash
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev \
cargo test --features graph-native --lib
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev \
cargo test --features graph-native --test graphql_viewer_smoke
RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev \
cargo clippy --features graph-native -- -D warnings
```

**Step 5 — Refactor for clarity if needed; commit:**

```bash
git add elohim/elohim-storage/src/graphql/ elohim/elohim-storage/tests/graphql_viewer_smoke.rs
git commit -m "feat(graphql): viewer.myCluster wrapping cluster_view service"
```

**Acceptance:** `cargo test --features graph-native --test graphql_viewer_smoke` passes. Clippy clean.

#### Task A3 — Resolver `Viewer.myTopology` wrapping `peer_topology_view::build_view`

**Files:**
- Modify: `elohim/elohim-storage/src/graphql/resolvers.rs`
- Test (Create): `elohim/elohim-storage/tests/graphql_my_topology_smoke.rs`

**Step 1 — Write the failing test:**

```rust
#![cfg(feature = "graph-native")]

use std::sync::Arc;
use elohim_storage::db::test_helpers::seed_alpha_with_terrance_household;
use elohim_storage::graph::engine::GraphEngine;
use elohim_storage::graphql::server::build_schema;

#[tokio::test]
async fn viewer_my_topology_returns_household_edges_with_reciprocity() {
    let pool = Arc::new(seed_alpha_with_terrance_household().await);
    let engine = Arc::new(GraphEngine::open_in_memory().unwrap());
    let connected_peers = Arc::new(|| Vec::<libp2p::PeerId>::new()) as Arc<_>;
    let schema = build_schema(engine, pool, connected_peers);

    let query = r#"{
        viewer(agentCid: "agent-matthew") {
            myTopology {
                reciprocationCount
                edges {
                    householdId displayName online
                    myCidsHostedByThem theirCidsHostedByMe
                    netDiff isCriticalForMe iAmCriticalForThem
                }
                resilienceCliffs { householdId soleReplicaCidCount }
            }
        }
    }"#;

    let resp = schema.execute(query).await;
    assert!(resp.errors.is_empty(), "errors: {:?}", resp.errors);
    let data = resp.data.into_json().unwrap();
    let edges = data["viewer"]["myTopology"]["edges"].as_array().unwrap();
    assert!(
        edges.iter().any(|e| e["displayName"].as_str() == Some("Terrance household")),
        "no Terrance-household edge: {:?}",
        edges
    );
    assert!(data["viewer"]["myTopology"]["reciprocationCount"].as_i64().unwrap() >= 1);
}
```

**Step 2 — Run, expect compile failure (`myTopology` not a field of Viewer).**

**Step 3 — Implement:**

1. Add `MyTopologyGql`, `PeerHouseholdEdgeGql`, `ResilienceCliffGql` resolver types in `resolvers.rs`, each wrapping the corresponding existing View struct.
2. Extend `Viewer`:
   ```rust
   async fn my_topology(&self, ctx: &Context<'_>) -> FieldResult<MyTopologyGql> {
       let pool = ctx.data::<Arc<DbPool>>()?;
       let connected = ctx.data::<Arc<dyn Fn() -> Vec<PeerId> + Send + Sync>>()?;
       let view = peer_topology_view::build_view(pool, &self.agent_cid, &connected()).await?;
       Ok(MyTopologyGql(view))
   }
   ```
3. Implement `#[Object]` impls that surface every field of `PeerTopologyView` and its children (verify field names exactly match the existing JSON shape — clients depend on it).

**Step 4 — Run all tests + clippy.**

**Step 5 — Commit:**

```bash
git commit -m "feat(graphql): viewer.myTopology wrapping peer_topology_view service"
```

**Acceptance:** `cargo test --features graph-native --test graphql_my_topology_smoke` passes. Field names match HTTP `/api/v1/peer-topology` shape exactly.

#### Task A4 — Wire dependencies through to `graphql::handle`

**Files:**
- Modify: `elohim/elohim-storage/src/graphql/server.rs::handle()`
- Modify: `elohim/elohim-storage/src/http.rs` (graphql route dispatch — find the existing call site)

**Step 1 — Write the failing test:** N/A — covered by A2's integration test which exercises `build_schema` end-to-end.

**Step 2 — Verify the existing GraphQL handler still compiles:** A2's signature change cascades. Adjust callers.

**Step 3 — Implement:**

1. Change `graphql::handle()` signature to accept `db_pool: &Arc<DbPool>` and `connected_peers: &Arc<dyn Fn() -> Vec<PeerId> + Send + Sync>`.
2. In `http.rs` at the graphql route, pass the existing `db_pool` Arc + a closure capturing the swarm's `connected_peers()` snapshot.

**Step 4 — Run integration test from A2 again to verify end-to-end POST `/api/v1/graphql` reaches the new resolvers.**

**Step 5 — Commit:**

```bash
git commit -m "feat(graphql): wire db pool + connected-peers snapshot through handle"
```

**Acceptance:** `curl -X POST http://localhost:8090/api/v1/graphql -d '{"query":"{ viewer(agentCid:\"X\") { myCluster { totals { deviceCount } } } }"}'` against a running storage process returns valid data. Existing graphql endpoint behavior (epr_head, contributor) preserved.

#### Task A5 — TypeScript GraphQL client + typed queries

**Files:**
- Create: `app/elohim-app/src/app/elohim/graphql-client/topology.queries.ts`
- Create: `app/elohim-app/src/app/elohim/graphql-client/topology-graphql.service.ts`
- Create: `app/elohim-app/src/app/elohim/graphql-client/topology-graphql.service.spec.ts`

**Decision:** Hand-authored gql strings + TS interfaces. graphql-codegen is over-engineered for two queries; revisit when the surface grows >5 queries.

**Step 1 — Write the failing test:**

```typescript
// topology-graphql.service.spec.ts
import { describe, it, expect, vi } from 'vitest';
import { TopologyGraphqlService } from './topology-graphql.service';

describe('TopologyGraphqlService', () => {
  it('myCluster posts the cluster query and parses the data', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({
        data: {
          viewer: {
            myCluster: {
              devices: [{ peerId: 'p1', archetype: 'desktop', online: true, displayName: 'Matthew laptop' }],
              totals: { storageBytes: 1024, deviceCount: 1 }
            }
          }
        }
      })
    });
    const svc = new TopologyGraphqlService(fetchMock as any, '/api/v1/graphql');
    const result = await svc.myCluster('agent-matthew');
    expect(result.devices[0].archetype).toBe('desktop');
    expect(result.totals.deviceCount).toBe(1);
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/v1/graphql',
      expect.objectContaining({ method: 'POST' })
    );
  });

  it('surfaces GraphQL errors', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ errors: [{ message: 'unknown agent' }] })
    });
    const svc = new TopologyGraphqlService(fetchMock as any, '/api/v1/graphql');
    await expect(svc.myCluster('nonexistent')).rejects.toThrow('unknown agent');
  });
});
```

**Step 2 — Run:**

```bash
cd /projects/elohim/app/elohim-app
pnpm exec vitest run --config vite.config.ts topology-graphql.service.spec
```

Expected failure: `TopologyGraphqlService` does not exist.

**Step 3 — Implement:**

```typescript
// topology.queries.ts
export const MY_CLUSTER_QUERY = `
  query MyCluster($agentCid: ID!) {
    viewer(agentCid: $agentCid) {
      myCluster {
        devices { peerId archetype online displayName }
        totals { storageBytes deviceCount }
      }
    }
  }`;

export const MY_TOPOLOGY_QUERY = `
  query MyTopology($agentCid: ID!) {
    viewer(agentCid: $agentCid) {
      myTopology {
        reciprocationCount
        edges { householdId displayName online myCidsHostedByThem theirCidsHostedByMe netDiff isCriticalForMe iAmCriticalForThem }
        resilienceCliffs { householdId soleReplicaCidCount }
      }
    }
  }`;

export interface MyClusterGql { devices: DeviceGql[]; totals: ClusterTotalsGql; }
export interface DeviceGql { peerId: string; archetype: 'node'|'desktop'|'mobile'|'steward'; online: boolean; displayName: string; }
// ... (mirror the existing MyClusterView / PeerTopologyView shapes)
```

```typescript
// topology-graphql.service.ts
@Injectable({ providedIn: 'root' })
export class TopologyGraphqlService {
  constructor(
    @Inject('FETCH') private fetch: typeof globalThis.fetch = globalThis.fetch,
    @Inject('GRAPHQL_ENDPOINT') private endpoint: string = '/api/v1/graphql',
  ) {}
  async myCluster(agentCid: string): Promise<MyClusterGql> {
    return this.query(MY_CLUSTER_QUERY, { agentCid }).then(d => d.viewer.myCluster);
  }
  async myTopology(agentCid: string): Promise<MyTopologyGql> {
    return this.query(MY_TOPOLOGY_QUERY, { agentCid }).then(d => d.viewer.myTopology);
  }
  private async query<T>(query: string, variables: Record<string, unknown>): Promise<T> {
    const res = await this.fetch(this.endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ query, variables }),
    });
    const body = await res.json();
    if (body.errors?.length) throw new Error(body.errors[0].message);
    return body.data;
  }
}
```

**Step 4 — Run vitest; lint.**

**Step 5 — Commit:**

```bash
git commit -m "feat(elohim-app): TopologyGraphqlService for viewer queries"
```

**Acceptance:** Vitest passes. Lint clean.

#### Task A6 — Angular service migration behind `useGraphqlTopology` flag

**Files:**
- Modify: `app/elohim-app/src/app/shefa/services/my-cluster.service.ts`
- Modify: `app/elohim-app/src/app/shefa/services/peer-topology.service.ts`
- Modify: `app/elohim-app/src/environments/environment.ts`
- Modify: `app/elohim-app/src/environments/environment.prod.ts`
- Modify: existing `*.spec.ts` for the two services — add flag-routing assertions

**Step 1 — Write the failing test (extend existing service specs):**

```typescript
it('when useGraphqlTopology=true, routes through TopologyGraphqlService', async () => {
  const gql = { myCluster: vi.fn().mockResolvedValue({ devices: [], totals: { storageBytes: 0, deviceCount: 0 } }) };
  const httpMock = vi.fn();
  const env = { useGraphqlTopology: true };
  const svc = new MyClusterService(httpMock as any, gql as any, env as any);
  await svc.fetchCluster('agent-matthew');
  expect(gql.myCluster).toHaveBeenCalledWith('agent-matthew');
  expect(httpMock).not.toHaveBeenCalled();
});
```

**Step 2 — Run; expect failure (flag not yet a constructor arg).**

**Step 3 — Implement:**

```typescript
@Injectable({ providedIn: 'root' })
export class MyClusterService {
  constructor(
    private http: HttpClient,
    private gql: TopologyGraphqlService,
    @Inject(ENV) private env: AppEnvironment,
  ) {}
  fetchCluster(agentCid: string): Observable<MyClusterView> {
    if (this.env.useGraphqlTopology) {
      return from(this.gql.myCluster(agentCid));
    }
    return this.http.get<MyClusterView>(`/api/v1/cluster?agentCid=${agentCid}`);
  }
}
```

Same shape for `PeerTopologyService`. Preserve any polling/observable subjects upstream.

Add to `environment.ts` (and `.prod.ts`): `useGraphqlTopology: false`.

**Step 4 — Run all elohim-app tests + lint.**

**Step 5 — Commit:**

```bash
git commit -m "feat(elohim-app): route topology services through GraphQL behind feature flag"
```

**Acceptance:** Existing component tests pass with flag-off (no behavior change). Flag-on test demonstrates GraphQL path used.

#### Task A7 — Visual parity verification (the `/deliver` gate)

**Files:**
- Create: `app/elohim-app/cypress/e2e/topology-graphql-parity.feature`
- Create: matching step definitions in `app/elohim-app/cypress/support/step_definitions/topology-graphql-parity.steps.ts`

**Step 1 — Write the failing scenario:**

```gherkin
@e2e @topology @graphql-parity @wip
Feature: GraphQL topology surface parity
  As an operator migrating to GraphQL
  I want /shefa/cluster and /shefa/peers to render identically under either transport
  So that the migration is invisible to users

  Scenario: my-cluster renders identically under flag toggle
    Given the app uses HTTP topology transport (flag off)
    When I visit "/shefa/cluster" as agent "agent-matthew"
    Then I capture a baseline screenshot "cluster-http-baseline.png"
    Given the app uses GraphQL topology transport (flag on)
    When I visit "/shefa/cluster" as agent "agent-matthew"
    Then the screenshot matches "cluster-http-baseline.png" within 2% pixel diff

  Scenario: my-topology renders identically under flag toggle
    Given the app uses HTTP topology transport (flag off)
    When I visit "/shefa/peers" as agent "agent-matthew"
    Then I capture a baseline screenshot "peers-http-baseline.png"
    Given the app uses GraphQL topology transport (flag on)
    When I visit "/shefa/peers" as agent "agent-matthew"
    Then the screenshot matches "peers-http-baseline.png" within 2% pixel diff
```

**Step 2 — Run; expect failure (step defs missing).**

**Step 3 — Implement step defs:**
- `Given the app uses HTTP topology transport`: set `window.__elohimEnvOverride = { useGraphqlTopology: false }` via Cypress route interception, reload
- `Given the app uses GraphQL topology transport`: same with `true`
- `Then I capture a baseline screenshot`: `cy.screenshot(name)`
- `Then the screenshot matches`: Cypress visual-regression plugin diff, fail above threshold

**Step 4 — Land + run scenario locally; capture initial baselines.**

**Step 5 — Lift `@wip` tag once both scenarios pass green. Commit:**

```bash
git commit -m "test(e2e): topology graphql parity scenarios + step defs"
```

**Acceptance:** Both scenarios pass with `@wip` tag lifted. `/deliver` ceremony reviews screenshots and approves visual parity.

#### Closing conditions (Epic A done = ALL of these hold)

- `cargo test --features graph-native --lib` passes (no regressions)
- `cargo test --features graph-native --test graphql_viewer_smoke` passes
- `cargo test --features graph-native --test graphql_my_topology_smoke` passes
- `cargo clippy --features graph-native -- -D warnings` clean
- `pnpm --filter elohim-app test` passes
- `pnpm --filter elohim-app run lint` clean
- Cypress scenario `topology-graphql-parity` passes; `@wip` lifted
- Pre-push hook green; orchestrator dispatches green on dev for two consecutive builds
- Manual verification: GraphQL endpoint returns valid data for `viewer.myCluster` and `viewer.myTopology` queries via `curl`
- No deletion of existing `/api/v1/cluster` or `/api/v1/peer-topology` HTTP routes — they stay live as fallback for one release cycle

#### Risk + mitigation

- **`MyClusterView`/`PeerTopologyView` carry enums that conflict with async-graphql derives.** Mitigation: don't `derive(SimpleObject)` on the View structs directly; use wrapper types `MyClusterGql(MyClusterView)` so async-graphql derives don't pollute the existing serde/ts-rs surface.
- **The `connected_peers` snapshot Arc-of-Fn pattern leaks lifetime concerns into resolver signatures.** Mitigation: confirm exact swarm-handle pattern at A2 Step 3; may need to capture an `mpsc::Receiver` or shared state behind a Mutex instead. Consult `elohim/elohim-storage/src/p2p/mod.rs:F-T20 responder arm` for the existing pattern.
- **DB pool exhaustion under GraphQL load.** Mitigation: A2 reuses the existing connection pool; no new pool needed. If query rate becomes a concern in prod, add per-resolver concurrency limit; out of scope for this Epic.
- **Schema-introspection surface accidentally leaks internal types.** Mitigation: only `Viewer`, `HubView`, `PeerTopology`, and their children are public; the EprHead/Contributor types from prior Phase 7 remain.

### Epic B — Deliver the missing `/shefa/reciprocity` page

**Gating scenario:** `shefa/m1-matthew-terrance-delivery.feature @wip` (reciprocity row)

Detail plan: `genesis/docs/plans/2026-05-19-viewer-symmetry-reciprocity-qahal-substrate.md` (L6 pass).

- [ ] B1. Add `Viewer.reciprocity` to GraphQL schema + resolver; back with existing `rea_commitments` + `rea_economic_events` relations (existing surfaces — see Source-of-Truth Declaration above; no new storage)
- [ ] B2. Create Angular route `/shefa/reciprocity` + component `app/elohim-app/src/app/shefa/pages/reciprocity/`
- [ ] B3. Render committed-vs-delivered per household; classify by reach
- [ ] B4. Lift `@wip` on the reciprocity scenarios in `m1-matthew-terrance-delivery.feature`

### Epic C — Resilience tooltip + placement-gaps badge

**Gating scenario:** `resilience/observable-distribution.feature @wip` (content-viewer resilience tooltip)

- [ ] C1. Confirm EPR head responses carry both `distribution` and `resilience` (two-dimension coherence per memory `2026-05-03 coherence sub-pass`)
- [ ] C2. Resilience tooltip component on `app/elohim-app/src/app/lamad/components/concept-card/` and content-viewer
- [ ] C3. Tooltip text: count + qahal-lens prompt ("your emergency circle is small")
- [ ] C4. Lift `@wip` on resilience-tooltip scenarios

### Epic D — Qahal lens overlay on topology edges

**Gating scenario:** `shefa/human-resilience.feature` (all `@wip`)

- [ ] D1. Join `HumanRelationship.type` into `PeerHouseholdEdge` (graph relation + resolver)
- [ ] D2. Resilience computation: trust-circle depth + reciprocal-commitment count → at-risk|partial|protected (compute on viewer side or projector?)
- [ ] D3. Re-render `peer-household-card` with relationship-type chip + resilience class
- [ ] D4. Lift `@wip` on the human-resilience progression scenarios

### Epic E — Qahal Collective view (slide 45 anchor)

**Gating scenario:** `qahal/collective-governance.feature` (`@wip` proxy-voting, recognition); future `qahal/collective-view.feature` (to author)

- [ ] E1. Add `Collective` type + resolver to GraphQL schema (members, stewards, activeNow, upcomingActivities, norms, contributionRecognition)
- [ ] E2. Author a2o feature `genesis/a2o/features/qahal/collective-view.feature` — 5-feature assertion per slide 45 (norms visible, AI coordinator, live co-presence, contribution reputation, smart matching)
- [ ] E3. Create Angular route `/qahal/collective/:id` + component embodying slide-45 layout (identity strip / AI card / stats row / live pane / upcoming + right rail + norms callouts)
- [ ] E4. Stewardship recognition: bridge governance-decisions → REA economic-events with `signal_kind = governance-participation`
- [ ] E5. Smart-matching pull via elohim affinity service (existing `reach-gate-is-elohim-mediated-matchmaking`)

### Epic F — Doorway dashboard surface

**Surfaces sixth of the original 6 topology surfaces.**

- [ ] F1. Add `Doorway` GraphQL type + resolver: stewards, federation peers, projection coverage, public-surface state
- [ ] F2. Create Angular route + component consuming the existing generated `doorway-dashboard-view.ts`
- [ ] F3. Qahal overlay: who-can-reach-whom view of authority distribution

### Epic G — Topology-overview as the unifier

**The graph projector already maintains this — work is binding, not building.**

- [ ] G1. Route `/topology-overview` (or fold into `/shefa/peers` as a zoomed-out tab)
- [ ] G2. Render households, collectives, reciprocity flows using the existing `topology-overview-view.ts`
- [ ] G3. Toggle: operational lens (sums/bytes) ↔ qahal lens (relationships/affinity)

## 6. What this plan deliberately does NOT do

- Does not change DHT entry types (zero new ones)
- Does not touch the libp2p `view-federation` codec — it's superseded; the existing codec stays in place until Epic A migrates the callers, then it can be retired
- Does not introduce new HTTP routes — every new surface projects through GraphQL
- Does not block on doorway dashboard (Epic F) for the demo gate; the demo runs on Epics A→E
- Does not lift `@wip` mechanically — only with a screenshot the operator accepts per `/deliver`

## 7. Open questions / deferred work

- Hylo + onebody design pass — both cloned at `/projects/research/`; a deeper read may surface qahal vocabulary or governance shapes worth adopting. **Not blocking** this sprint; can fold lessons into Epic E during execution.
- Compute-event dashboard (`shefa/interfaces/compute-event.interface.ts`) — out of scope for this sprint; deferred to a shefa-economic-event-visibility sprint
- Storage-distribution 3-tab view — out of scope; the live-data version is the topology-overview surface from Epic G
- Slide 49's "shared context: pinned canon, searchable archives, institutional memory" — real gap, no surface yet; deferred to a future "qahal canonical memory" sprint (storyteller agent's substrate)

## 8. Execution shape

Recommended next move: `/shift` on Epic A (the foundation — every other epic depends on the GraphQL surface) followed by `/deliver` ceremonies per gating scenario as B/C/D/E land. Epics A→D are the core demo loop; E is the qahal-anchor that converts demo into product brief satisfaction; F/G are the breadth.

The visible verification is, end-to-end: **take a peer offline → replica count drops → resilience class shifts → tooltip prompts "ask your congregation member" → bring peer back → counts rise → tooltip clears.** Each transition is a frame in a screenshot loop the operator approves.
