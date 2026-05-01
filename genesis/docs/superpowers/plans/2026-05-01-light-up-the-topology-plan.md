# Light Up the Topology Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the operational graph visible — five views (per-content distribution badge, my cluster, my peer topology, my reciprocity, doorway operator dashboard) over one substrate query layer, plus two substrate fixes that make the resilience demo land (peer offline → page still loads; peer online → bytes arrive in seconds).

**Architecture:** Live-aggregation queries over existing notarized + projected DHT state. Multi-device humans handled via existing AgentPeerBinding (Phase 2B Task A.2) — auth context resolves to `agent_cid`, bindings expand to peer set, federation libp2p protocol queries each device with cryptographic gating. No new DHT entry types, no new SQLite migrations.

**Tech Stack:** Rust (elohim-storage, doorway-service), libp2p 0.54 (request-response codec), Diesel (SQLite projections), ts-rs (codegen), Angular 19 (elohim-app + doorway-app), Vitest, Cypress + Cucumber, Jenkins regression.

**Spec:** `genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-design.md`

---

## P2P Design Gate — Source of Truth Declarations

**Every entity in this plan was classified in the spec's P2P Design Gate output. No new DHT entry types. No new SQLite tables. The classifications below MUST be cited in every file's module docstring during implementation.**

| Entity / File | Category | Source of Truth | Reconstruction |
|---|---|---|---|
| `AgentPeerBinding` (existing, load-bearing) | A — Notarized | Holochain DHT (imagodei zome) | Already exists; query coordinator `get_agent_peer_bindings(agent_cid)` |
| `DistributionSummary` / `Details` (views.rs) | C — Operational | DHT (replicas via custodian commitments, projectors via rea_projection signal acks, reach via FeedbackSignal-projected manifest) | Re-aggregate from existing tables |
| `MyClusterView` (views.rs) | C — Operational | DHT (binding set notarized) + per-device libp2p live state | Federate via `view-federation/1.0.0` |
| `PeerTopologyView` (views.rs) | C — Operational | DHT (bindings, custodian commitments) + libp2p (per-peer connectivity) | Federate via `view-federation/1.0.0` |
| `ReciprocityView` (views.rs) | C — Operational | DHT — REA Commitment + EconomicEvent (existing in `content_store` zome) | SQL aggregation over `rea_commitments` + `rea_economic_events` (no federation; DHT is authoritative) |
| `DoorwayDashboardView` (doorway/views/) | C — Operational | Doorway-resident operational state (cache, federation peers, route registry); doorway is web2 projection per `project_three_layer_truth_model` | Re-aggregate from existing doorway services |
| `ViewFederationRequest` / `Response` (p2p/view_federation.rs) | C — Operational | Protocol messages, not stored | Request itself is the reconstruction |

### HTTP routes (anti-pattern check)

Every new HTTP route in this plan is a **read-projection over existing notarized truth**. The route is the *thinnest* possible layer per p2p-design-gate's API design order. No route creates DHT entries. No route is a starting-point design decision — each route's data shape is determined entirely by its corresponding `views.rs` struct, which in turn was determined by the gate output above.

| Route | Category | Source of Truth |
|---|---|---|
| `GET /api/v1/blob/{hash}/distribution/details` | C | DHT (custodian + rea_projection + content_store entries) |
| `GET /api/v1/cluster/me` | C | DHT (bindings) + libp2p (federated live state) |
| `GET /api/v1/peer-topology/me` | C | DHT (bindings + commitments) + libp2p (federated edges) |
| `GET /api/v1/reciprocity/me` | C | DHT (REA Commitment + EconomicEvent) — no federation |
| `GET /admin/dashboard/topology` | C | Doorway-local operational state |
| `GET /api/v1/blob/{hash}` (extended path on local miss) | C | Existing route; extension follows the EPR cold-fetch peer-fallback pattern (`services/epr_store.rs:301-388`) |

### Schemas

Every JSON schema in `elohim/sdk/schemas/v1/views/` declares its corresponding view type's source of truth via an `x-source-of-truth` annotation in the schema's `description` or as a top-level metadata field. Each schema in this plan MUST include such a declaration.

### Inline declaration requirement (per-file)

Every Rust file created in this plan MUST begin its module docstring with:

```rust
//! ## Source of Truth
//!
//! This module is **Operational (Category C)** per the p2p-design-gate output in
//! `genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-design.md`.
//! It composes a read-projection from notarized DHT state. The DHT remains canonical.
//! No SQLite table here is authoritative.
```

This is non-negotiable. Subagents implementing tasks must include this block in every new module file.

---

## File Structure

### New Rust files

```
elohim/elohim-storage/src/services/
  distribution_view.rs       compose_distribution_summary, compose_distribution_details
  cluster_view.rs            aggregate_cluster_view (federated)
  peer_topology_view.rs      aggregate_peer_topology_view (federated)
  reciprocity_view.rs        aggregate_reciprocity_view (no federation, SQL only)

elohim/elohim-storage/src/api/
  distribution.rs            handler: GET /api/v1/blob/{hash}/distribution/details
  cluster.rs                 handler: GET /api/v1/cluster/me
  peer_topology.rs           handler: GET /api/v1/peer-topology/me
  reciprocity.rs             handler: GET /api/v1/reciprocity/me

elohim/elohim-storage/src/p2p/
  view_federation.rs         protocol /elohim/view-federation/1.0.0
                              codec, handler, Federator client

elohim/elohim-storage/src/auth/
  bindings_resolver.rs       resolve auth context → agent_cid → Vec<PeerBinding>

doorway/doorway-service/src/services/
  dashboard_topology.rs      DoorwayDashboardView aggregator

doorway/doorway-service/src/routes/admin/
  dashboard_topology.rs      handler: GET /admin/dashboard/topology
```

### Modified Rust files

```
elohim/elohim-storage/src/views.rs       + DistributionSummary, DistributionDetails,
                                            MyClusterView, PeerTopologyView, ReciprocityView,
                                            ViewSlice, Freshness, ReplicaPeer, ProjectorIdentity,
                                            DiversityHint, DiversityKind, PeerHouseholdEdge,
                                            ResilienceCliff, DeviceSummary, DeviceTotals
elohim/elohim-storage/src/epr_head.rs    extend EPR head response with
                                          distribution: Option<DistributionSummary>
elohim/elohim-storage/src/http.rs        build_manifest() registers 4 new routes;
                                          GET /blob/{hash} extended for peer-fallback
elohim/elohim-storage/src/p2p/mod.rs     ConnectionEstablished kick;
                                          P2PCommand::ViewFederate variant
elohim/elohim-storage/src/p2p/behaviour.rs  add ViewFederation behaviour
elohim/elohim-storage/src/services/epr_store.rs  extract peer-fallback helper
doorway/doorway-service/src/server/http.rs  match arm for /admin/dashboard/topology
```

### Schema files

```
elohim/sdk/schemas/v1/views/
  distribution-summary.schema.json
  distribution-details.schema.json
  my-cluster-view.schema.json
  peer-topology-view.schema.json
  reciprocity-view.schema.json
  doorway-dashboard-view.schema.json
  view-slice.schema.json
  freshness.schema.json
```

### Auto-generated TS types (regenerated, not manually written)

```
elohim/sdk/storage-client-ts/src/generated/
  DistributionSummary.ts, DistributionDetails.ts,
  MyClusterView.ts, PeerTopologyView.ts, ReciprocityView.ts,
  ReplicaPeer.ts, ProjectorIdentity.ts, DiversityHint.ts, ViewSlice.ts,
  Freshness.ts, PeerHouseholdEdge.ts, ...
```

### New Angular files

```
app/elohim-app/src/app/elohim/components/
  distribution-badge/distribution-badge.component.ts (+.html +.scss +.spec.ts)
  device-tile/device-tile.component.ts (+ siblings)
  peer-household-card/peer-household-card.component.ts (+ siblings)
  commitment-bar/commitment-bar.component.ts (+ siblings)
  diversity-hint/diversity-hint.component.ts (+ siblings)

app/elohim-app/src/app/elohim/services/
  distribution.service.ts (+ .spec.ts)

app/elohim-app/src/app/shefa/services/
  cluster.service.ts (+ .spec.ts)
  peer-topology.service.ts (+ .spec.ts)
  reciprocity.service.ts (+ .spec.ts)

app/elohim-app/src/app/shefa/pages/
  my-cluster/my-cluster.component.ts (+ siblings)
  peer-topology/peer-topology.component.ts (+ siblings)
  reciprocity-ledger/reciprocity-ledger.component.ts (+ siblings)
```

### Modified Angular files

```
app/elohim-app/src/app/elohim/adapters/content-node.adapter.ts
  surface distribution from EPR envelope

app/elohim-app/src/app/lamad/components/content-card/content-card.component.html
  embed <elohim-distribution-badge>

app/elohim-app/src/app/app.routes.ts
  + /cluster, /peers, /reciprocity routes

doorway/doorway-app/src/app/admin/...
  + operator-topology pane
```

---

## Phases

| Phase | Tasks | Goal |
|-------|-------|------|
| 0 — Pre-flight | T00–T03 | Verify substrate prerequisites are alive |
| 1 — Schemas | T04–T11 | Schema-first IoC: declare wire shapes |
| 2 — Substrate fixes | T12–T15 | Peer-fallback + on-connect kick + regression test |
| 3 — Federation protocol | T16–T21 | view-federation/1.0.0 codec + handler + aggregator |
| 4 — View services | T22–T28 | Compose summaries + topology aggregators |
| 5 — HTTP handlers + manifest | T29–T34 | Wire routes through `build_manifest()` + EPR head extension |
| 6 — TS codegen | T35–T36 | ts-rs + schema:codegen:ts pass clean |
| 7 — Angular atoms | T37–T41 | Five shared visual primitives |
| 8 — Angular services + pages | T42–T48 | Page-level wiring |
| 9 — App integration | T49–T52 | Adapter + content-card + routing + doorway-app |
| 10 — Integration tests | T53–T56 | Three Jenkins regressions + BDD scenarios |
| 11 — A2O harvest + sweep | T57 | Story-harvest constraints, final CI gates |

---

## Phase 0 — Pre-flight Verification

Before scope-locking the sprint, confirm three substrate prerequisites are alive. If any fail, the sprint must descope or block on substrate fixes first.

### Task T00: Verify AgentPeerBindings are being seeded

**Files:**
- Read: `genesis/seeder/src/seed-agent-bindings.ts` (or equivalent)
- Read: `app/elohim-app/scripts/hc-start.sh`

- [ ] **Step 1: Locate the binding seeder**

```bash
find genesis/seeder -name "*binding*" -o -name "*device*"
grep -l "create_agent_peer_binding\|AgentPeerBinding" genesis/seeder/src/*.ts
```

Expected: a seeder file exists. If none found → file an issue and BLOCK the sprint.

- [ ] **Step 2: Run the seeder against a fresh stack**

```bash
cd app/elohim-app
pnpm run hc:start:seed
# wait until seed completes
sleep 60
curl -s http://localhost:8090/api/v1/agents | jq '.[] | select(.bindings != null) | {id, bindings: .bindings | length}'
```

Expected: at least one agent has 2+ bindings (multi-device demo requires multi-binding agents).

- [ ] **Step 3: Confirm bindings reach the projection**

```bash
sqlite3 ~/.elohim-storage/storage.db "SELECT agent_cid, peer_id, device_archetype FROM peer_identity_bindings WHERE superseded_by IS NULL LIMIT 10;"
```

Expected: rows present, archetype values from `{node, desktop, mobile, steward}`.

- [ ] **Step 4: Document findings**

Edit the spec's "Pre-flight verification" section: mark this verified or open a sub-task to seed bindings.

### Task T01: Verify auth context propagation through doorway

**Files:**
- Read: `doorway/doorway-service/src/auth/jwt.rs`, `auth/portal_host.rs`
- Read: `elohim/elohim-storage/src/http.rs` (auth context extraction)

- [ ] **Step 1: Trace auth header from doorway to storage**

```bash
grep -rn "x-agent-cid\|X-Agent-Cid\|agent_cid" doorway/doorway-service/src/ elohim/elohim-storage/src/ | head -20
```

Expected: there's a header or session field carrying `agent_cid` from doorway to storage. If none → first sub-task of T28 will be to add header propagation.

- [ ] **Step 2: Test propagation with a mock request**

```bash
# get a session token from doorway
TOKEN=$(curl -s -X POST http://localhost:8888/auth/login -d '{"agent":"adam"}' | jq -r .token)
# hit a manifest-routed endpoint with the token
curl -v http://localhost:8888/api/v1/health -H "Authorization: Bearer $TOKEN" 2>&1 | grep -i "x-agent\|x-forwarded"
```

Expected: header forwarding visible. If absent → add a sub-task to T28: "implement agent_cid header propagation in storage_proxy".

- [ ] **Step 3: Document findings**

Update the spec as in T00.

### Task T02: Verify rea_projection signal stream

**Files:**
- Read: `elohim/elohim-storage/src/rea_projection.rs`

- [ ] **Step 1: Check projector ack rows exist**

```bash
sqlite3 ~/.elohim-storage/storage.db "SELECT COUNT(*) FROM rea_projection WHERE projector_acks IS NOT NULL OR ack_count > 0;"
```

Expected: > 0 rows. If 0 → projector signals aren't landing; T20 (`compose_distribution_summary`) should default `projectorCount = 0` and the demo will render "peer-only" badges.

- [ ] **Step 2: Document findings**

If projector acks are absent at runtime, add a sub-task to T34 (operator dashboard) to expose this in the operator topology view so we can debug live.

### Task T03: Bump SEED_LIMIT and seed diverse content

**Files:**
- Modify: `app/elohim-app/scripts/hc-start.sh` (env default)

- [ ] **Step 1: Find current SEED_LIMIT default**

```bash
grep -n "SEED_LIMIT" app/elohim-app/scripts/hc-start.sh genesis/seeder/src/seed.ts
```

- [ ] **Step 2: Set SEED_LIMIT to cover all content formats**

Edit `app/elohim-app/scripts/hc-start.sh`:

```bash
# was: : "${SEED_LIMIT:=20}"
: "${SEED_LIMIT:=200}"
```

- [ ] **Step 3: Re-seed and verify diversity**

```bash
pnpm run hc:start:seed
sleep 30
curl -s http://localhost:8090/api/v1/content | jq '[.[] | .contentFormat] | unique'
```

Expected: at least `["markdown", "sophia-quiz-json", "html5-app", "spa-bundle", "plaintext"]` represented.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/scripts/hc-start.sh
git commit -m "chore(dev): raise SEED_LIMIT default to 200 for diverse-format demo"
```

---

## Phase 1 — Schemas (Schema-first IoC)

Per `feedback_schema_first_ioc`: write JSON schemas first; Rust + TS comply; never guess. All schemas use `$schema: "http://json-schema.org/draft-07/schema#"` per the existing pattern in `elohim/sdk/schemas/v1/views/`.

### Task T04: distribution-summary.schema.json + Rust struct

**Files:**
- Create: `elohim/sdk/schemas/v1/views/distribution-summary.schema.json`
- Modify: `elohim/elohim-storage/src/views.rs` (add struct)
- Modify: `elohim/elohim-storage/tests/schema_contract.rs` (add contract test)
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs` (add to INTERFACE_FILES)

- [ ] **Step 1: Write the schema**

Create `elohim/sdk/schemas/v1/views/distribution-summary.schema.json`:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.host/schemas/v1/views/distribution-summary.schema.json",
  "title": "DistributionSummary",
  "description": "Inline per-CID distribution payload, hydrated onto every EPR/content response. ~100 bytes/item. Drives badge state and simple-tier tooltip.",
  "type": "object",
  "additionalProperties": false,
  "required": ["replicaCount", "replicaTarget", "replicaHealth", "projectorCount", "reachClass", "diversityHint", "thisFetchSource", "lastVerifiedSeconds"],
  "properties": {
    "replicaCount": { "type": "integer", "minimum": 0 },
    "replicaTarget": { "type": "integer", "minimum": 0 },
    "replicaHealth": { "enum": ["healthy", "at_risk", "critical"] },
    "projectorCount": { "type": "integer", "minimum": 0 },
    "reachClass": { "enum": ["private", "intimate", "household", "neighborhood", "collective", "community", "district", "public"] },
    "diversityHint": { "$ref": "https://elohim.host/schemas/v1/views/diversity-hint.schema.json" },
    "thisFetchSource": { "enum": ["projected_via_doorway", "peer_direct", "local_pantry"] },
    "lastVerifiedSeconds": { "type": "integer", "minimum": 0, "description": "Seconds since last custodian probe" },
    "myRole": { "enum": ["sole_replica", "replica", "replica_and_projector", "not_hosting"] },
    "reciprocityHint": { "type": "integer", "description": "Net diff: positive = I host more than I'm hosted" }
  }
}
```

Also create `elohim/sdk/schemas/v1/views/diversity-hint.schema.json`:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.host/schemas/v1/views/diversity-hint.schema.json",
  "title": "DiversityHint",
  "type": "object",
  "additionalProperties": false,
  "required": ["kind", "value"],
  "properties": {
    "kind": { "enum": ["region_metro", "household_archetypes", "collective_member_count", "none"] },
    "value": {
      "oneOf": [
        { "type": "array", "items": { "type": "string" } },
        { "type": "integer" },
        { "type": "null" }
      ]
    }
  }
}
```

- [ ] **Step 2: Add the Rust struct**

Append to `elohim/elohim-storage/src/views.rs`:

```rust
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct DistributionSummary {
    pub replica_count: u32,
    pub replica_target: u32,
    pub replica_health: ReplicaHealth,
    pub projector_count: u32,
    pub reach_class: ReachClass,
    pub diversity_hint: DiversityHint,
    pub this_fetch_source: FetchSource,
    pub last_verified_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub my_role: Option<MyRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reciprocity_hint: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "snake_case")]
pub enum ReplicaHealth { Healthy, AtRisk, Critical }

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "snake_case")]
pub enum ReachClass {
    Private, Intimate, Household, Neighborhood,
    Collective, Community, District, Public,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "snake_case")]
pub enum FetchSource { ProjectedViaDoorway, PeerDirect, LocalPantry }

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "snake_case")]
pub enum MyRole { SoleReplica, Replica, ReplicaAndProjector, NotHosting }

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum DiversityHint {
    RegionMetro(Vec<String>),
    HouseholdArchetypes(Vec<String>),
    CollectiveMemberCount(u32),
    None,
}
```

- [ ] **Step 3: Write the schema contract test**

Append to `elohim/elohim-storage/tests/schema_contract.rs`:

```rust
#[test]
fn distribution_summary_matches_schema() {
    let sample = views::DistributionSummary {
        replica_count: 12,
        replica_target: 14,
        replica_health: views::ReplicaHealth::Healthy,
        projector_count: 2,
        reach_class: views::ReachClass::Public,
        diversity_hint: views::DiversityHint::RegionMetro(vec!["us-central".into(), "eu-west".into()]),
        this_fetch_source: views::FetchSource::ProjectedViaDoorway,
        last_verified_seconds: 420,
        my_role: None,
        reciprocity_hint: None,
    };
    let v = serde_json::to_value(&sample).unwrap();
    let schema = load_schema("distribution-summary.schema.json");
    schema.validate(&v).expect("validates");
}
```

- [ ] **Step 4: Register codegen target**

Edit `elohim/sdk/schemas/scripts/codegen-ts.mjs` `INTERFACE_FILES`:

```js
const INTERFACE_FILES = [
  // ... existing
  "views/distribution-summary.schema.json",
  "views/diversity-hint.schema.json",
];
```

- [ ] **Step 5: Run tests**

```bash
cd elohim/elohim-storage
RUSTFLAGS="" cargo test schema_contract::distribution_summary_matches_schema -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add elohim/sdk/schemas/v1/views/distribution-summary.schema.json \
        elohim/sdk/schemas/v1/views/diversity-hint.schema.json \
        elohim/elohim-storage/src/views.rs \
        elohim/elohim-storage/tests/schema_contract.rs \
        elohim/sdk/schemas/scripts/codegen-ts.mjs
git commit -m "feat(views): DistributionSummary schema + Rust struct + contract test"
```

### Task T05: distribution-details.schema.json + Rust struct

**Files:**
- Create: `elohim/sdk/schemas/v1/views/distribution-details.schema.json`
- Modify: `elohim/elohim-storage/src/views.rs` (add struct + ReplicaPeer + ProjectorIdentity)
- Modify: `elohim/elohim-storage/tests/schema_contract.rs`

- [ ] **Step 1: Write schemas**

Create `elohim/sdk/schemas/v1/views/distribution-details.schema.json`:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.host/schemas/v1/views/distribution-details.schema.json",
  "title": "DistributionDetails",
  "description": "Lazy-fetched per-CID developer-grade view. Strict superset of DistributionSummary fields.",
  "type": "object",
  "additionalProperties": false,
  "required": ["summary", "replicaPeers", "projectorIdentities", "placementGaps", "recentProjectionEvents"],
  "properties": {
    "summary": { "$ref": "https://elohim.host/schemas/v1/views/distribution-summary.schema.json" },
    "replicaPeers": {
      "type": "array",
      "items": { "$ref": "https://elohim.host/schemas/v1/views/replica-peer.schema.json" }
    },
    "projectorIdentities": {
      "type": "array",
      "items": { "$ref": "https://elohim.host/schemas/v1/views/projector-identity.schema.json" }
    },
    "placementGaps": {
      "type": "array",
      "items": { "type": "object", "additionalProperties": true }
    },
    "recentProjectionEvents": {
      "type": "array",
      "items": { "type": "object", "additionalProperties": true }
    },
    "reciprocityEdges": {
      "type": "array",
      "items": { "$ref": "https://elohim.host/schemas/v1/views/peer-household-edge.schema.json" }
    },
    "commitmentReferences": {
      "type": "array",
      "items": { "type": "string", "description": "CustodianCommitment CIDs" }
    }
  }
}
```

Create `elohim/sdk/schemas/v1/views/replica-peer.schema.json`:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.host/schemas/v1/views/replica-peer.schema.json",
  "title": "ReplicaPeer",
  "type": "object",
  "additionalProperties": false,
  "required": ["peerId", "deviceArchetype", "lastSeenSeconds"],
  "properties": {
    "peerId": { "type": "string" },
    "deviceArchetype": { "enum": ["node", "desktop", "mobile", "steward"] },
    "lastSeenSeconds": { "type": "integer", "minimum": 0 },
    "hopHint": { "type": "integer", "minimum": 0 },
    "householdId": { "type": "string" },
    "regionTier": { "type": "string" }
  }
}
```

Create `elohim/sdk/schemas/v1/views/projector-identity.schema.json`:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.host/schemas/v1/views/projector-identity.schema.json",
  "title": "ProjectorIdentity",
  "type": "object",
  "additionalProperties": false,
  "required": ["doorwayHostname", "lastAckSeconds"],
  "properties": {
    "doorwayHostname": { "type": "string" },
    "lastAckSeconds": { "type": "integer", "minimum": 0 },
    "regionTier": { "type": "string" }
  }
}
```

- [ ] **Step 2: Add Rust structs**

Append to `views.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct DistributionDetails {
    pub summary: DistributionSummary,
    pub replica_peers: Vec<ReplicaPeer>,
    pub projector_identities: Vec<ProjectorIdentity>,
    pub placement_gaps: Vec<serde_json::Value>,
    pub recent_projection_events: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reciprocity_edges: Option<Vec<PeerHouseholdEdge>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commitment_references: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ReplicaPeer {
    pub peer_id: String,
    pub device_archetype: DeviceArchetype,
    pub last_seen_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hop_hint: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub household_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_tier: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "snake_case")]
pub enum DeviceArchetype { Node, Desktop, Mobile, Steward }

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ProjectorIdentity {
    pub doorway_hostname: String,
    pub last_ack_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_tier: Option<String>,
}
```

- [ ] **Step 3: Schema contract test**

```rust
#[test]
fn distribution_details_matches_schema() {
    let sample = views::DistributionDetails {
        summary: /* same as T04 */ todo!("copy from T04 sample"),
        replica_peers: vec![],
        projector_identities: vec![],
        placement_gaps: vec![],
        recent_projection_events: vec![],
        reciprocity_edges: None,
        commitment_references: None,
    };
    let v = serde_json::to_value(&sample).unwrap();
    load_schema("distribution-details.schema.json").validate(&v).unwrap();
}
```

(Replace `todo!()` with the literal sample from T04 step 3.)

- [ ] **Step 4: Register codegen target**

```js
INTERFACE_FILES.push(
  "views/distribution-details.schema.json",
  "views/replica-peer.schema.json",
  "views/projector-identity.schema.json",
);
```

- [ ] **Step 5: Run tests**

```bash
RUSTFLAGS="" cargo test schema_contract::distribution_details -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add elohim/sdk/schemas/v1/views/distribution-details.schema.json \
        elohim/sdk/schemas/v1/views/replica-peer.schema.json \
        elohim/sdk/schemas/v1/views/projector-identity.schema.json \
        elohim/elohim-storage/src/views.rs \
        elohim/elohim-storage/tests/schema_contract.rs \
        elohim/sdk/schemas/scripts/codegen-ts.mjs
git commit -m "feat(views): DistributionDetails + ReplicaPeer + ProjectorIdentity schemas"
```

### Task T06: my-cluster-view.schema.json + Rust struct

**Files:**
- Create: `elohim/sdk/schemas/v1/views/my-cluster-view.schema.json`
- Create: `elohim/sdk/schemas/v1/views/freshness.schema.json`
- Modify: `elohim/elohim-storage/src/views.rs`
- Modify: `elohim/elohim-storage/tests/schema_contract.rs`

- [ ] **Step 1: Write schemas**

Create `elohim/sdk/schemas/v1/views/freshness.schema.json`:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.host/schemas/v1/views/freshness.schema.json",
  "title": "Freshness",
  "type": "object",
  "additionalProperties": false,
  "required": ["state"],
  "properties": {
    "state": { "enum": ["live", "stale", "offline", "cached_offline_until_reconnect", "unverifiable", "all_offline"] },
    "staleSinceMs": { "type": "integer", "minimum": 0 }
  }
}
```

Create `elohim/sdk/schemas/v1/views/my-cluster-view.schema.json`:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.host/schemas/v1/views/my-cluster-view.schema.json",
  "title": "MyClusterView",
  "type": "object",
  "additionalProperties": false,
  "required": ["agentCid", "devices", "totals", "freshness"],
  "properties": {
    "agentCid": { "type": "string" },
    "devices": {
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["peerId", "archetype", "online", "freshness"],
        "properties": {
          "peerId": { "type": "string" },
          "archetype": { "enum": ["node", "desktop", "mobile", "steward"] },
          "displayName": { "type": "string" },
          "online": { "type": "boolean" },
          "freshness": { "$ref": "https://elohim.host/schemas/v1/views/freshness.schema.json" },
          "storageUsedBytes": { "type": "integer", "minimum": 0 },
          "storageTotalBytes": { "type": "integer", "minimum": 0 },
          "memoryUsedBytes": { "type": "integer", "minimum": 0 },
          "memoryTotalBytes": { "type": "integer", "minimum": 0 },
          "hostingCount": { "type": "integer", "minimum": 0 },
          "projectingCount": { "type": "integer", "minimum": 0 },
          "beaconAgeMs": { "type": "integer", "minimum": 0 }
        }
      }
    },
    "totals": {
      "type": "object",
      "additionalProperties": false,
      "required": ["storageUsedBytes", "storageTotalBytes", "externalCommittedBytes", "reciprocityNetBytes"],
      "properties": {
        "storageUsedBytes": { "type": "integer", "minimum": 0 },
        "storageTotalBytes": { "type": "integer", "minimum": 0 },
        "externalCommittedBytes": { "type": "integer", "minimum": 0 },
        "reciprocityNetBytes": { "type": "integer" }
      }
    },
    "freshness": { "$ref": "https://elohim.host/schemas/v1/views/freshness.schema.json" }
  }
}
```

- [ ] **Step 2: Add Rust structs**

Append to `views.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct Freshness {
    pub state: FreshnessState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_since_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "snake_case")]
pub enum FreshnessState {
    Live, Stale, Offline, CachedOfflineUntilReconnect, Unverifiable, AllOffline,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct MyClusterView {
    pub agent_cid: String,
    pub devices: Vec<DeviceSummary>,
    pub totals: DeviceTotals,
    pub freshness: Freshness,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct DeviceSummary {
    pub peer_id: String,
    pub archetype: DeviceArchetype,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub online: bool,
    pub freshness: Freshness,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_used_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_used_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hosting_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projecting_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beacon_age_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct DeviceTotals {
    pub storage_used_bytes: u64,
    pub storage_total_bytes: u64,
    pub external_committed_bytes: u64,
    pub reciprocity_net_bytes: i64,
}
```

- [ ] **Step 3: Contract test**

```rust
#[test]
fn my_cluster_view_matches_schema() {
    let sample = views::MyClusterView {
        agent_cid: "agent_abc123".into(),
        devices: vec![
            views::DeviceSummary {
                peer_id: "12D3KooW...".into(),
                archetype: views::DeviceArchetype::Desktop,
                display_name: Some("Matthew's laptop".into()),
                online: true,
                freshness: views::Freshness { state: views::FreshnessState::Live, stale_since_ms: None },
                storage_used_bytes: Some(18_400_000_000),
                storage_total_bytes: Some(250_000_000_000),
                memory_used_bytes: None, memory_total_bytes: None,
                hosting_count: Some(1247),
                projecting_count: Some(802),
                beacon_age_ms: Some(0),
            }
        ],
        totals: views::DeviceTotals {
            storage_used_bytes: 25_200_000_000,
            storage_total_bytes: 298_000_000_000,
            external_committed_bytes: 14_800_000_000,
            reciprocity_net_bytes: 5_200_000_000,
        },
        freshness: views::Freshness { state: views::FreshnessState::Live, stale_since_ms: None },
    };
    let v = serde_json::to_value(&sample).unwrap();
    load_schema("my-cluster-view.schema.json").validate(&v).unwrap();
}
```

- [ ] **Step 4: Register codegen targets**

Add to INTERFACE_FILES: `views/my-cluster-view.schema.json`, `views/freshness.schema.json`.

- [ ] **Step 5: Run tests**

```bash
RUSTFLAGS="" cargo test schema_contract::my_cluster_view -- --nocapture
```

- [ ] **Step 6: Commit**

```bash
git add elohim/sdk/schemas/v1/views/my-cluster-view.schema.json \
        elohim/sdk/schemas/v1/views/freshness.schema.json \
        elohim/elohim-storage/src/views.rs \
        elohim/elohim-storage/tests/schema_contract.rs \
        elohim/sdk/schemas/scripts/codegen-ts.mjs
git commit -m "feat(views): MyClusterView + Freshness schemas"
```

### Task T07: peer-topology-view.schema.json + Rust struct

**Files:**
- Create: `elohim/sdk/schemas/v1/views/peer-topology-view.schema.json`
- Create: `elohim/sdk/schemas/v1/views/peer-household-edge.schema.json`
- Modify: `elohim/elohim-storage/src/views.rs`
- Modify: `elohim/elohim-storage/tests/schema_contract.rs`

- [ ] **Step 1: Write schemas**

Create `peer-household-edge.schema.json`:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.host/schemas/v1/views/peer-household-edge.schema.json",
  "title": "PeerHouseholdEdge",
  "type": "object",
  "additionalProperties": false,
  "required": ["householdId", "online", "myCidsHostedByThem", "theirCidsHostedByMe"],
  "properties": {
    "householdId": { "type": "string" },
    "displayName": { "type": "string" },
    "online": { "type": "boolean" },
    "lastSyncSec": { "type": "integer", "minimum": 0 },
    "myCidsHostedByThem": { "type": "integer", "minimum": 0 },
    "theirCidsHostedByMe": { "type": "integer", "minimum": 0 },
    "netDiff": { "type": "integer" },
    "isCriticalForMe": { "type": "boolean", "description": "If this household goes dark, I lose sole-replica CIDs" },
    "iAmCriticalForThem": { "type": "boolean" }
  }
}
```

Create `peer-topology-view.schema.json`:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.host/schemas/v1/views/peer-topology-view.schema.json",
  "title": "PeerTopologyView",
  "type": "object",
  "additionalProperties": false,
  "required": ["agentCid", "edges", "reciprocationCount", "resilienceCliffs", "freshness"],
  "properties": {
    "agentCid": { "type": "string" },
    "edges": {
      "type": "array",
      "items": { "$ref": "https://elohim.host/schemas/v1/views/peer-household-edge.schema.json" }
    },
    "reciprocationCount": { "type": "integer", "minimum": 0 },
    "resilienceCliffs": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["householdId", "soleReplicaCidCount"],
        "properties": {
          "householdId": { "type": "string" },
          "soleReplicaCidCount": { "type": "integer", "minimum": 0 }
        }
      }
    },
    "freshness": { "$ref": "https://elohim.host/schemas/v1/views/freshness.schema.json" }
  }
}
```

- [ ] **Step 2: Add Rust structs**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct PeerHouseholdEdge {
    pub household_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub online: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync_sec: Option<u64>,
    pub my_cids_hosted_by_them: u32,
    pub their_cids_hosted_by_me: u32,
    pub net_diff: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_critical_for_me: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub i_am_critical_for_them: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct PeerTopologyView {
    pub agent_cid: String,
    pub edges: Vec<PeerHouseholdEdge>,
    pub reciprocation_count: u32,
    pub resilience_cliffs: Vec<ResilienceCliff>,
    pub freshness: Freshness,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ResilienceCliff {
    pub household_id: String,
    pub sole_replica_cid_count: u32,
}
```

- [ ] **Step 3: Contract test**

```rust
#[test]
fn peer_topology_view_matches_schema() {
    let sample = views::PeerTopologyView {
        agent_cid: "agent_abc".into(),
        edges: vec![],
        reciprocation_count: 3,
        resilience_cliffs: vec![],
        freshness: views::Freshness { state: views::FreshnessState::Live, stale_since_ms: None },
    };
    let v = serde_json::to_value(&sample).unwrap();
    load_schema("peer-topology-view.schema.json").validate(&v).unwrap();
}
```

- [ ] **Step 4: Register codegen + run tests + commit**

```bash
# update INTERFACE_FILES, run cargo test, commit
RUSTFLAGS="" cargo test schema_contract::peer_topology_view -- --nocapture
git add ...
git commit -m "feat(views): PeerTopologyView + PeerHouseholdEdge + ResilienceCliff schemas"
```

### Task T08: reciprocity-view.schema.json + Rust struct

**Files:**
- Create: `elohim/sdk/schemas/v1/views/reciprocity-view.schema.json`
- Modify: `elohim/elohim-storage/src/views.rs`
- Modify: `elohim/elohim-storage/tests/schema_contract.rs`

- [ ] **Step 1: Schema**

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.host/schemas/v1/views/reciprocity-view.schema.json",
  "title": "ReciprocityView",
  "type": "object",
  "additionalProperties": false,
  "required": ["agentCid", "inflow", "outflow", "netHostedBytes", "capacityAvailableBytes"],
  "properties": {
    "agentCid": { "type": "string" },
    "inflow": { "type": "array", "items": { "$ref": "#/definitions/reciprocityRow" } },
    "outflow": { "type": "array", "items": { "$ref": "#/definitions/reciprocityRow" } },
    "netHostedBytes": { "type": "integer", "description": "Positive = others hold more for me than I do for them" },
    "capacityAvailableBytes": { "type": "integer", "minimum": 0 }
  },
  "definitions": {
    "reciprocityRow": {
      "type": "object",
      "required": ["counterpartyHouseholdId", "committedBytes", "deliveredBytes", "honoredPercent"],
      "properties": {
        "counterpartyHouseholdId": { "type": "string" },
        "displayName": { "type": "string" },
        "committedBytes": { "type": "integer", "minimum": 0 },
        "deliveredBytes": { "type": "integer", "minimum": 0 },
        "honoredPercent": { "type": "number", "minimum": 0 },
        "online": { "type": "boolean" }
      }
    }
  }
}
```

- [ ] **Step 2: Rust**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ReciprocityView {
    pub agent_cid: String,
    pub inflow: Vec<ReciprocityRow>,
    pub outflow: Vec<ReciprocityRow>,
    pub net_hosted_bytes: i64,
    pub capacity_available_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ReciprocityRow {
    pub counterparty_household_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub committed_bytes: u64,
    pub delivered_bytes: u64,
    pub honored_percent: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub online: Option<bool>,
}
```

- [ ] **Step 3: Contract test, register, run, commit**

```bash
RUSTFLAGS="" cargo test schema_contract::reciprocity_view -- --nocapture
git commit -m "feat(views): ReciprocityView + ReciprocityRow schemas"
```

### Task T09: doorway-dashboard-view.schema.json + Rust struct

**Files:**
- Create: `elohim/sdk/schemas/v1/views/doorway-dashboard-view.schema.json`
- Modify: `doorway/doorway-service/src/views/mod.rs` (or where doorway view types live)
- Modify: doorway schema contract test (or add one)

- [ ] **Step 1: Schema**

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.host/schemas/v1/views/doorway-dashboard-view.schema.json",
  "title": "DoorwayDashboardView",
  "type": "object",
  "additionalProperties": false,
  "required": ["doorwayHostname", "storageStewards", "federationPeers", "projectionCoverage", "publicSurface"],
  "properties": {
    "doorwayHostname": { "type": "string" },
    "storageStewards": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["peerId", "archetype", "online", "hostingCount"],
        "properties": {
          "peerId": { "type": "string" },
          "archetype": { "enum": ["node", "desktop", "mobile", "steward"] },
          "displayName": { "type": "string" },
          "online": { "type": "boolean" },
          "hostingCount": { "type": "integer", "minimum": 0 },
          "hopHint": { "type": "integer" }
        }
      }
    },
    "federationPeers": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["doorwayHostname", "online", "direction", "sharedCidCount"],
        "properties": {
          "doorwayHostname": { "type": "string" },
          "online": { "type": "boolean" },
          "direction": { "enum": ["bidirectional", "outbound_only", "inbound_only"] },
          "sharedCidCount": { "type": "integer", "minimum": 0 }
        }
      }
    },
    "projectionCoverage": {
      "type": "object",
      "required": ["projectedCidCount", "knownCidCount", "cacheHitRate24h", "projectionLagMsAvg"],
      "properties": {
        "projectedCidCount": { "type": "integer", "minimum": 0 },
        "knownCidCount": { "type": "integer", "minimum": 0 },
        "cacheHitRate24h": { "type": "number", "minimum": 0, "maximum": 1 },
        "projectionLagMsAvg": { "type": "integer", "minimum": 0 }
      }
    },
    "publicSurface": {
      "type": "object",
      "required": ["dnsResolves", "tlsValid", "publicReachable"],
      "properties": {
        "dnsResolves": { "type": "boolean" },
        "dnsTarget": { "type": "string" },
        "tlsValid": { "type": "boolean" },
        "tlsExpiresInDays": { "type": "integer" },
        "publicReachable": { "type": "boolean" }
      }
    }
  }
}
```

- [ ] **Step 2: Rust struct in doorway-service**

Add to `doorway/doorway-service/src/views/dashboard.rs` (create file if absent):

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../doorway-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct DoorwayDashboardView {
    pub doorway_hostname: String,
    pub storage_stewards: Vec<DashboardSteward>,
    pub federation_peers: Vec<DashboardFederationPeer>,
    pub projection_coverage: ProjectionCoverage,
    pub public_surface: PublicSurfaceState,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../doorway-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct DashboardSteward {
    pub peer_id: String,
    pub archetype: String, // mirrors DeviceArchetype enum value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub online: bool,
    pub hosting_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hop_hint: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../doorway-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct DashboardFederationPeer {
    pub doorway_hostname: String,
    pub online: bool,
    pub direction: FederationDirection,
    pub shared_cid_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../doorway-client-ts/src/generated/")]
#[serde(rename_all = "snake_case")]
pub enum FederationDirection { Bidirectional, OutboundOnly, InboundOnly }

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../doorway-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ProjectionCoverage {
    pub projected_cid_count: u32,
    pub known_cid_count: u32,
    pub cache_hit_rate_24h: f64,
    pub projection_lag_ms_avg: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../doorway-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct PublicSurfaceState {
    pub dns_resolves: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_target: Option<String>,
    pub tls_valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_expires_in_days: Option<i32>,
    pub public_reachable: bool,
}
```

- [ ] **Step 3: Contract test**

```rust
// doorway/doorway-service/tests/views_contract.rs (create if absent)
#[test]
fn doorway_dashboard_view_matches_schema() {
    let sample = doorway_service::views::DoorwayDashboardView {
        doorway_hostname: "matthew.elohim.host".into(),
        storage_stewards: vec![],
        federation_peers: vec![],
        projection_coverage: doorway_service::views::ProjectionCoverage {
            projected_cid_count: 4318, known_cid_count: 5672,
            cache_hit_rate_24h: 0.87, projection_lag_ms_avg: 340,
        },
        public_surface: doorway_service::views::PublicSurfaceState {
            dns_resolves: true, dns_target: Some("203.0.113.42".into()),
            tls_valid: true, tls_expires_in_days: Some(64),
            public_reachable: true,
        },
    };
    let v = serde_json::to_value(&sample).unwrap();
    let schema = load_schema("doorway-dashboard-view.schema.json");
    schema.validate(&v).unwrap();
}
```

- [ ] **Step 4: Register, run, commit**

```bash
RUSTFLAGS="" cargo test --manifest-path doorway/doorway-service/Cargo.toml views_contract -- --nocapture
git add elohim/sdk/schemas/v1/views/doorway-dashboard-view.schema.json \
        doorway/doorway-service/src/views/dashboard.rs \
        doorway/doorway-service/tests/views_contract.rs
git commit -m "feat(views): DoorwayDashboardView schema + Rust struct"
```

### Task T10: view-slice.schema.json (federation slice base)

**Files:**
- Create: `elohim/sdk/schemas/v1/views/view-slice.schema.json`
- Modify: `elohim/elohim-storage/src/views.rs`

- [ ] **Step 1: Schema**

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.host/schemas/v1/views/view-slice.schema.json",
  "title": "ViewSlice",
  "description": "Per-device slice returned over view-federation/1.0.0; signed by responding peer's agent key.",
  "type": "object",
  "additionalProperties": false,
  "required": ["peerId", "viewKind", "freshness", "payload", "signature"],
  "properties": {
    "peerId": { "type": "string" },
    "viewKind": { "enum": ["cluster", "peer_topology"] },
    "freshness": { "$ref": "https://elohim.host/schemas/v1/views/freshness.schema.json" },
    "payload": { "type": "object", "description": "view-kind-specific slice body" },
    "signature": { "type": "string", "description": "base64 signature over canonical_bytes(viewKind, peerId, freshness, payload)" }
  }
}
```

- [ ] **Step 2: Rust struct**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ViewSlice {
    pub peer_id: String,
    pub view_kind: ViewKind,
    pub freshness: Freshness,
    pub payload: serde_json::Value,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq, Hash)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "snake_case")]
pub enum ViewKind { Cluster, PeerTopology }
```

- [ ] **Step 3: Register, commit**

```bash
git add ...
git commit -m "feat(views): ViewSlice + ViewKind schemas (federation envelope)"
```

### Task T11: Run schema codegen end-to-end

**Files:** auto-generated, no manual edits

- [ ] **Step 1: Run schema validate**

```bash
pnpm run schema:validate
```

Expected: pass; all new schemas resolve `$ref`s.

- [ ] **Step 2: Run schema codegen**

```bash
pnpm run schema:codegen:ts
```

Expected: pass; check git diff for the regenerated TS files in `elohim/sdk/storage-client-ts/src/generated/`. Hand-edit nothing — drift indicates schema bugs.

- [ ] **Step 3: Run ts-rs export**

```bash
cd elohim/elohim-storage
cargo test export_bindings
```

Expected: pass; ts-rs writes the same TS files. Diff with codegen output should be empty (or only formatting).

- [ ] **Step 4: Verify storage-client-ts compiles**

```bash
cd elohim/sdk/storage-client-ts
pnpm install && pnpm run build
```

Expected: pass.

- [ ] **Step 5: Commit codegen output**

```bash
git add elohim/sdk/storage-client-ts/src/generated/ doorway/sdk/doorway-client-ts/src/generated/ 2>/dev/null
git commit -m "chore(codegen): regenerate TS bindings for Phase 1 view schemas"
```

---

## Phase 2 — Substrate Fixes

These two fixes are independent of the view layer but load-bearing for the demo. Without them, the resilience claim ("page still loads when a peer goes offline; bytes arrive when a peer comes online") cannot be verified.

### Task T12: Extract peer-fallback helper from epr_store

**Files:**
- Modify: `elohim/elohim-storage/src/services/epr_store.rs:301-388`
- Create: `elohim/elohim-storage/src/services/peer_fallback.rs`

- [ ] **Step 1: Read current cold-fetch path**

```bash
sed -n '290,400p' elohim/elohim-storage/src/services/epr_store.rs
```

Note the function signature, the Kad-provider lookup, the per-provider fetch loop, the timeout handling.

- [ ] **Step 2: Write a failing test**

Create `elohim/elohim-storage/tests/peer_fallback_helper.rs`:

```rust
//! Tests the extracted peer-fallback helper used by both EPR cold-fetch and blob fallback paths.
use elohim_storage::services::peer_fallback::{fetch_via_peers, PeerFallbackError};
use elohim_storage::test_util::*;

#[tokio::test]
async fn fetch_via_peers_returns_first_provider_data() {
    let harness = MockSwarmHarness::new();
    harness.seed_provider("peer_a", "hash_xyz", b"hello".to_vec());
    let result = fetch_via_peers(&harness.swarm_client(), "hash_xyz", 3000).await;
    assert_eq!(result.unwrap(), b"hello".to_vec());
}

#[tokio::test]
async fn fetch_via_peers_no_providers_returns_unavailable() {
    let harness = MockSwarmHarness::new();
    let result = fetch_via_peers(&harness.swarm_client(), "missing_hash", 3000).await;
    assert!(matches!(result, Err(PeerFallbackError::NoProviders)));
}

#[tokio::test]
async fn fetch_via_peers_all_timeout_returns_all_timed_out() {
    let harness = MockSwarmHarness::new();
    harness.seed_unresponsive_provider("peer_dead", "hash_xyz");
    let result = fetch_via_peers(&harness.swarm_client(), "hash_xyz", 200).await;
    assert!(matches!(result, Err(PeerFallbackError::AllProvidersTimedOut { tried: 1 })));
}
```

- [ ] **Step 3: Run, expect fail**

```bash
cd elohim/elohim-storage
RUSTFLAGS="" cargo test --test peer_fallback_helper 2>&1 | head -20
```

Expected: FAIL — module `peer_fallback` does not exist.

- [ ] **Step 4: Create the helper module**

Create `elohim/elohim-storage/src/services/peer_fallback.rs`:

```rust
//! ## Source of Truth
//!
//! This module is **Operational (Category C)** per the p2p-design-gate output in
//! `genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-design.md`.
//! It composes a read-projection from notarized DHT state. The DHT remains canonical.
//! No SQLite table here is authoritative.
//!
//! Shared peer-fallback helper used by:
//!   - EPR cold-fetch path (services/epr_store.rs)
//!   - Blob GET-time fallback (http.rs handle_blob)

use std::time::Duration;
use thiserror::Error;
use crate::p2p::client::SwarmClient;

#[derive(Debug, Error)]
pub enum PeerFallbackError {
    #[error("no providers found in Kad")]
    NoProviders,
    #[error("all {tried} providers timed out")]
    AllProvidersTimedOut { tried: usize },
    #[error("hash mismatch from provider {peer_id}")]
    HashMismatch { peer_id: String },
    #[error("swarm channel closed")]
    SwarmGone,
}

pub async fn fetch_via_peers(
    swarm: &SwarmClient,
    content_hash: &str,
    timeout_ms: u64,
) -> Result<Vec<u8>, PeerFallbackError> {
    let providers = swarm.kad_get_providers(content_hash)
        .await
        .map_err(|_| PeerFallbackError::SwarmGone)?;

    if providers.is_empty() {
        return Err(PeerFallbackError::NoProviders);
    }

    let mut tried = 0usize;
    for provider in providers {
        tried += 1;
        let result = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            swarm.shard_get(provider.clone(), content_hash.to_string()),
        ).await;

        match result {
            Ok(Ok(bytes)) => {
                let actual_hash = sha256_hex(&bytes);
                if actual_hash != content_hash {
                    tracing::warn!(provider = %provider, "provider returned wrong bytes");
                    continue;
                }
                return Ok(bytes);
            }
            Ok(Err(e)) => {
                tracing::info!(provider = %provider, error = %e, "provider fetch failed");
                continue;
            }
            Err(_timeout) => {
                tracing::info!(provider = %provider, "provider timed out");
                continue;
            }
        }
    }

    Err(PeerFallbackError::AllProvidersTimedOut { tried })
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}
```

- [ ] **Step 5: Refactor epr_store.rs to use the helper**

Edit `elohim/elohim-storage/src/services/epr_store.rs:301-388` — replace the inline Kad-provider-loop with `fetch_via_peers` invocation. Keep the function's outer error mapping; only the loop body changes.

- [ ] **Step 6: Add module declaration**

Edit `elohim/elohim-storage/src/services/mod.rs`:

```rust
pub mod peer_fallback;
```

- [ ] **Step 7: Run tests**

```bash
RUSTFLAGS="" cargo test --test peer_fallback_helper -- --nocapture
RUSTFLAGS="" cargo test epr_store -- --nocapture
```

Expected: both pass.

- [ ] **Step 8: Commit**

```bash
git add elohim/elohim-storage/src/services/peer_fallback.rs \
        elohim/elohim-storage/src/services/mod.rs \
        elohim/elohim-storage/src/services/epr_store.rs \
        elohim/elohim-storage/tests/peer_fallback_helper.rs
git commit -m "refactor(storage): extract shared peer-fallback helper from epr_store"
```

### Task T13: GET-time peer-fallback for blobs

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs:1431` (the GET /blob handler)

- [ ] **Step 1: Write failing integration test**

Create `elohim/elohim-storage/tests/blob_peer_fallback.rs`:

```rust
//! Tests that GET /blob/{hash} on local miss falls back to peers via the shared helper.
use elohim_storage::test_util::*;

#[tokio::test]
async fn blob_get_local_miss_fetches_from_peer_and_caches() {
    let harness = TwoPeerHarness::start().await;
    let hash = harness.seed_blob_on_peer_a(b"hello-blob").await;

    // peer B has no local copy
    assert!(!harness.peer_b().blob_store.exists(&hash));

    // GET /blob/{hash} on peer B
    let res = harness.peer_b_http_get(&format!("/blob/{}", hash)).await;
    assert_eq!(res.status(), 200);
    assert_eq!(res.bytes().await.unwrap().as_ref(), b"hello-blob");

    // cache write: B now has it locally
    assert!(harness.peer_b().blob_store.exists(&hash));
}

#[tokio::test]
async fn blob_get_no_providers_returns_404_with_reason() {
    let harness = TwoPeerHarness::start().await;
    let res = harness.peer_b_http_get("/blob/nonexistent_hash_aaaa").await;
    assert_eq!(res.status(), 404);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["reason"], "unavailable_no_providers");
}

#[tokio::test]
async fn blob_get_all_timeout_returns_502_with_tried_count() {
    let harness = TwoPeerHarness::start_with_unresponsive_a().await;
    let hash = harness.seed_blob_on_peer_a_unresponsive(b"timeout-blob").await;
    let res = harness.peer_b_http_get(&format!("/blob/{}", hash)).await;
    assert_eq!(res.status(), 502);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["reason"], "all_providers_timed_out");
    assert!(body["tried"].as_u64().unwrap() >= 1);
}
```

- [ ] **Step 2: Run, expect fail**

```bash
RUSTFLAGS="" cargo test --test blob_peer_fallback 2>&1 | head -20
```

Expected: FAIL — current handler returns 404 unconditionally on local miss.

- [ ] **Step 3: Modify the handler**

Edit `elohim/elohim-storage/src/http.rs` at the GET /blob/{hash} arm (around line 1431):

```rust
async fn handle_blob_get(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> Response {
    if let Some(bytes) = state.blob_store.read(&hash).await.ok().flatten() {
        return blob_response(bytes);
    }

    // NEW: peer-fallback path
    use crate::services::peer_fallback::{fetch_via_peers, PeerFallbackError};
    match fetch_via_peers(&state.swarm_client, &hash, 1500).await {
        Ok(bytes) => {
            let _ = state.blob_store.store(&bytes).await; // best-effort cache write
            blob_response(bytes)
        }
        Err(PeerFallbackError::NoProviders) => json_error(404, "unavailable_no_providers"),
        Err(PeerFallbackError::AllProvidersTimedOut { tried }) => {
            json_error_with(502, "all_providers_timed_out", json!({ "tried": tried }))
        }
        Err(e) => json_error(502, &format!("fetch_failed: {}", e)),
    }
}
```

- [ ] **Step 4: Run tests**

```bash
RUSTFLAGS="" cargo test --test blob_peer_fallback -- --nocapture
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/http.rs \
        elohim/elohim-storage/tests/blob_peer_fallback.rs
git commit -m "feat(storage): GET /blob peer-fallback on local miss"
```

### Task T14: On-connect replication kick (jittered)

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (~line 2117, ConnectionEstablished handler)

- [ ] **Step 1: Failing test**

Create `elohim/elohim-storage/tests/on_connect_kick.rs`:

```rust
#[tokio::test]
async fn fresh_peer_first_byte_within_10s() {
    let cluster = SixPeerHarness::start_with_seeded_content().await;
    let timestamp_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();

    let fresh = cluster.spawn_seventh_peer().await; // raspberry-pi-4 archetype, empty pantry

    // Wait for first blob arrival
    let arrival_ms = fresh.wait_for_first_blob(std::time::Duration::from_secs(15)).await
        .expect("first blob should arrive within 15s");

    let elapsed = arrival_ms - timestamp_ms;
    assert!(elapsed < 10_000, "first byte took {} ms (expected <10000)", elapsed);
}

#[tokio::test]
async fn jitter_skipped_on_immediate_disconnect() {
    let harness = TwoPeerHarness::start().await;
    let kick_count_before = harness.peer_a().metrics().kicks_fired();
    harness.peer_b().disconnect_immediately().await;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await; // > max jitter
    let kick_count_after = harness.peer_a().metrics().kicks_fired();
    assert_eq!(kick_count_before, kick_count_after, "no kick should fire if peer disconnected during jitter");
}

#[tokio::test]
async fn global_kick_cap_respected() {
    let harness = ManyPeerHarness::start_with_n_peers(20).await;
    let in_flight = harness.peer_a().metrics().kicks_in_flight_max();
    assert!(in_flight <= 16, "kicks in flight {} exceeded cap 16", in_flight);
}
```

- [ ] **Step 2: Run, expect fail**

```bash
RUSTFLAGS="" cargo test --test on_connect_kick 2>&1 | head -20
```

Expected: FAIL — no kick logic, fresh peer waits ~60s.

- [ ] **Step 3: Implement the kick**

Edit `elohim/elohim-storage/src/p2p/mod.rs` near line 2117 where `ConnectionEstablished` is handled:

```rust
// Existing: trust + identity handshakes
// ... existing code ...

// NEW: jittered ListContent kick
if state.kicks_in_flight.load(Ordering::Relaxed) < KICK_GLOBAL_CAP {
    let peer_clone = peer_id.clone();
    let kicks_in_flight = state.kicks_in_flight.clone();
    let cmd_tx = state.cmd_tx.clone();
    let is_connected_check = state.is_connected_handle.clone();

    kicks_in_flight.fetch_add(1, Ordering::Relaxed);

    tokio::spawn(async move {
        let jitter_ms = rand::random::<u64>() % 2000;
        tokio::time::sleep(Duration::from_millis(jitter_ms)).await;

        if !is_connected_check.is_connected(&peer_clone) {
            tracing::info!(peer = %peer_clone, "skipping kick: disconnected during jitter");
            kicks_in_flight.fetch_sub(1, Ordering::Relaxed);
            return;
        }

        let _ = cmd_tx.send(P2PCommand::SendListContent {
            peer: peer_clone.clone(),
            limit: 5000,
        }).await;

        tracing::debug!(peer = %peer_clone, "fired on-connect ListContent kick");
        kicks_in_flight.fetch_sub(1, Ordering::Relaxed);
        state.metrics.kicks_fired.fetch_add(1, Ordering::Relaxed);
    });
}

const KICK_GLOBAL_CAP: usize = 16;
```

Add to `AppState`:

```rust
pub kicks_in_flight: Arc<AtomicUsize>,
pub metrics: Arc<P2PMetrics>,
```

- [ ] **Step 4: Run tests**

```bash
RUSTFLAGS="" cargo test --test on_connect_kick -- --nocapture
```

Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/p2p/mod.rs \
        elohim/elohim-storage/tests/on_connect_kick.rs
git commit -m "feat(p2p): jittered on-connect ListContent kick (cold peer to first byte <10s)"
```

### Task T15: Filesystem-count parity regression

**Files:**
- Create: `elohim/elohim-storage/tests/replication_parity.rs`

- [ ] **Step 1: Write the regression test**

```rust
//! Regression: after replication settles, peer B's filesystem CID count must equal peer A's.
//! Catches the failure mode in `project_inventory_exchange_not_byte_replication`.

#[tokio::test]
async fn filesystem_count_parity_after_replication() {
    let cluster = TwoPeerHarness::start().await;
    cluster.seed_n_blobs_on_peer_a(50).await;

    // Wait for replication: 60s tick + 5s gap drain + per-blob round-trip * 50
    tokio::time::sleep(std::time::Duration::from_secs(120)).await;

    let count_a = cluster.peer_a().filesystem_blob_count();
    let count_b = cluster.peer_b().filesystem_blob_count();

    assert_eq!(count_a, count_b, "peer B should have same blob count as peer A; got {} vs {}", count_b, count_a);
}

#[tokio::test]
async fn kill_and_restore_peer_preserves_distribution() {
    let cluster = SixPeerHarness::start_with_seeded_content().await;
    cluster.wait_for_initial_replication().await;

    let cid = cluster.first_seeded_cid();
    let initial_replica_count = cluster.distribution_replica_count(&cid).await;
    assert_eq!(initial_replica_count, 4, "expected target=4");

    // Kill 2 hosting peers
    cluster.sigstop_peers_hosting(&cid, 2).await;
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // GET /blob/{cid} (visitor) should still 200 via fallback
    let res = cluster.visitor_get_blob(&cid).await;
    assert_eq!(res.status(), 200);

    // Distribution count should reflect 2 fewer
    let drop_count = cluster.distribution_replica_count(&cid).await;
    assert_eq!(drop_count, 2);

    // Restore peers
    cluster.sigcont_all().await;
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;

    // Count back to target
    let restored_count = cluster.distribution_replica_count(&cid).await;
    assert_eq!(restored_count, 4);
}
```

- [ ] **Step 2: Run on alpha cluster (Jenkins)**

These tests require the alpha cluster harness; locally they should compile but the asserts run on Jenkins.

```bash
RUSTFLAGS="" cargo test --test replication_parity --no-run
```

Expected: compiles. Jenkins job runs the actual asserts.

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/tests/replication_parity.rs
git commit -m "test(storage): filesystem-count parity + kill/restore distribution regression"
```

---

## Phase 3 — View-Federation Protocol

### Task T16: View-federation wire types

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs` (add request/response wire types)

- [ ] **Step 1: Add wire types**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ViewFederationRequest {
    pub view_kind: ViewKind,
    pub agent_cid: String,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ViewFederationResponse {
    pub view_kind: ViewKind,
    pub agent_cid: String,
    pub request_id: String,
    pub slice: ViewSlice,
}

impl ViewFederationRequest {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        rmp_serde::to_vec_named(self).expect("infallible")
    }
}

impl ViewSlice {
    /// Bytes-to-sign: view_kind || peer_id || freshness_state || payload (msgpack canonical).
    pub fn canonical_bytes_for_signing(&self) -> Vec<u8> {
        #[derive(Serialize)]
        struct Canonical<'a> {
            view_kind: &'a ViewKind,
            peer_id: &'a str,
            freshness_state: &'a FreshnessState,
            payload: &'a serde_json::Value,
        }
        rmp_serde::to_vec_named(&Canonical {
            view_kind: &self.view_kind,
            peer_id: &self.peer_id,
            freshness_state: &self.freshness.state,
            payload: &self.payload,
        }).expect("infallible")
    }
}
```

- [ ] **Step 2: Compile + commit**

```bash
RUSTFLAGS="" cargo build --release
git add elohim/elohim-storage/src/views.rs
git commit -m "feat(views): ViewFederationRequest/Response + canonical signing bytes"
```

### Task T17: View-federation codec

**Files:**
- Create: `elohim/elohim-storage/src/p2p/view_federation.rs`

- [ ] **Step 1: Failing test**

Create `elohim/elohim-storage/tests/view_federation_codec.rs`:

```rust
use elohim_storage::p2p::view_federation::{ViewFederationCodec, PROTOCOL_NAME, MAX_PAYLOAD};
use elohim_storage::views::*;
use libp2p::request_response::Codec;

#[tokio::test]
async fn codec_round_trip_request() {
    let mut codec = ViewFederationCodec::default();
    let req = ViewFederationRequest {
        view_kind: ViewKind::Cluster,
        agent_cid: "agent_abc".into(),
        request_id: "req_001".into(),
    };
    let mut buf = Vec::new();
    let mut writer = futures::io::Cursor::new(&mut buf);
    codec.write_request(&PROTOCOL_NAME, &mut writer, req.clone()).await.unwrap();

    let mut reader = futures::io::Cursor::new(&buf);
    let decoded = codec.read_request(&PROTOCOL_NAME, &mut reader).await.unwrap();
    assert_eq!(decoded, req);
}

#[tokio::test]
async fn codec_rejects_oversized_payload() {
    let mut codec = ViewFederationCodec::default();
    let huge = vec![0u8; MAX_PAYLOAD + 1];
    let mut reader = futures::io::Cursor::new(&huge);
    let result = codec.read_request(&PROTOCOL_NAME, &mut reader).await;
    assert!(result.is_err(), "should reject payload > MAX_PAYLOAD");
}

#[tokio::test]
async fn codec_rejects_malformed_msgpack() {
    let mut codec = ViewFederationCodec::default();
    let mut reader = futures::io::Cursor::new(b"not msgpack" as &[u8]);
    let result = codec.read_request(&PROTOCOL_NAME, &mut reader).await;
    assert!(result.is_err());
}
```

- [ ] **Step 2: Run, expect fail**

```bash
RUSTFLAGS="" cargo test --test view_federation_codec
```

Expected: FAIL — module does not exist.

- [ ] **Step 3: Implement the codec**

Create `elohim/elohim-storage/src/p2p/view_federation.rs`:

```rust
//! ## Source of Truth
//!
//! This module is **Operational (Category C)** per the p2p-design-gate output in
//! `genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-design.md`.
//! Wire-protocol messages, not stored. Federation is gated cryptographically by
//! DHT-notarized AgentPeerBindings.

use async_trait::async_trait;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use libp2p::{request_response::Codec, StreamProtocol};
use std::io;

use crate::views::{ViewFederationRequest, ViewFederationResponse};

pub const PROTOCOL_NAME: StreamProtocol = StreamProtocol::new("/elohim/view-federation/1.0.0");
pub const MAX_PAYLOAD: usize = 256 * 1024; // 256 KB

#[derive(Default, Clone)]
pub struct ViewFederationCodec;

#[async_trait]
impl Codec for ViewFederationCodec {
    type Protocol = StreamProtocol;
    type Request = ViewFederationRequest;
    type Response = ViewFederationResponse;

    async fn read_request<T>(&mut self, _: &StreamProtocol, io: &mut T) -> io::Result<Self::Request>
    where T: AsyncRead + Unpin + Send {
        let mut buf = Vec::new();
        io.take(MAX_PAYLOAD as u64 + 1).read_to_end(&mut buf).await?;
        if buf.len() > MAX_PAYLOAD {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "payload exceeds MAX_PAYLOAD"));
        }
        rmp_serde::from_slice(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(&mut self, _: &StreamProtocol, io: &mut T) -> io::Result<Self::Response>
    where T: AsyncRead + Unpin + Send {
        let mut buf = Vec::new();
        io.take(MAX_PAYLOAD as u64 + 1).read_to_end(&mut buf).await?;
        if buf.len() > MAX_PAYLOAD {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "payload exceeds MAX_PAYLOAD"));
        }
        rmp_serde::from_slice(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(&mut self, _: &StreamProtocol, io: &mut T, req: Self::Request) -> io::Result<()>
    where T: AsyncWrite + Unpin + Send {
        let buf = rmp_serde::to_vec_named(&req)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        if buf.len() > MAX_PAYLOAD {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "request exceeds MAX_PAYLOAD"));
        }
        io.write_all(&buf).await
    }

    async fn write_response<T>(&mut self, _: &StreamProtocol, io: &mut T, res: Self::Response) -> io::Result<()>
    where T: AsyncWrite + Unpin + Send {
        let buf = rmp_serde::to_vec_named(&res)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        if buf.len() > MAX_PAYLOAD {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "response exceeds MAX_PAYLOAD"));
        }
        io.write_all(&buf).await
    }
}
```

Add to `elohim/elohim-storage/src/p2p/mod.rs`:

```rust
pub mod view_federation;
```

- [ ] **Step 4: Run tests**

```bash
RUSTFLAGS="" cargo test --test view_federation_codec -- --nocapture
```

Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/p2p/view_federation.rs \
        elohim/elohim-storage/src/p2p/mod.rs \
        elohim/elohim-storage/tests/view_federation_codec.rs
git commit -m "feat(p2p): /elohim/view-federation/1.0.0 codec (msgpack, 256KB cap)"
```

### Task T18: Behaviour composition

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/behaviour.rs`

- [ ] **Step 1: Add the behaviour to the swarm**

Edit `elohim/elohim-storage/src/p2p/behaviour.rs`:

```rust
use libp2p::request_response;
use crate::p2p::view_federation::{ViewFederationCodec, PROTOCOL_NAME as VIEW_FED_PROTOCOL};

#[derive(NetworkBehaviour)]
pub struct ElohimBehaviour {
    // ... existing behaviours
    pub view_federation: request_response::Behaviour<ViewFederationCodec>,
}

impl ElohimBehaviour {
    pub fn new(/* existing args */) -> Self {
        // ... existing
        let view_federation = request_response::Behaviour::with_codec(
            ViewFederationCodec::default(),
            std::iter::once((VIEW_FED_PROTOCOL, request_response::ProtocolSupport::Full)),
            request_response::Config::default()
                .with_request_timeout(std::time::Duration::from_secs(3)),
        );
        Self { /* existing */ , view_federation }
    }
}
```

- [ ] **Step 2: Compile**

```bash
RUSTFLAGS="" cargo build --release
```

Expected: clean build (no clippy warnings yet — that comes after handler).

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/src/p2p/behaviour.rs
git commit -m "feat(p2p): wire ViewFederation into ElohimBehaviour swarm"
```

### Task T19: P2PCommand::ViewFederate variant + dispatch

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (add P2PCommand variant + handler arm)
- Modify: `elohim/elohim-storage/src/p2p/client.rs` (add SwarmClient::view_federate)

- [ ] **Step 1: Failing test**

Create `elohim/elohim-storage/tests/p2p_command_view_federate.rs`:

```rust
#[tokio::test]
async fn view_federate_command_dispatches_request_response() {
    let harness = TwoPeerHarness::start().await;
    harness.peer_b().pre_canned_view_slice("cluster", b"slice-bytes").await;

    let response = harness.peer_a().swarm_client().view_federate(
        harness.peer_b().peer_id(),
        ViewFederationRequest {
            view_kind: ViewKind::Cluster,
            agent_cid: "agent_test".into(),
            request_id: "req_001".into(),
        },
        std::time::Duration::from_secs(3),
    ).await.unwrap();

    assert_eq!(response.view_kind, ViewKind::Cluster);
    assert_eq!(response.agent_cid, "agent_test");
}

#[tokio::test]
async fn view_federate_timeout_returns_error() {
    let harness = TwoPeerHarness::start_with_unresponsive_b().await;
    let result = harness.peer_a().swarm_client().view_federate(
        harness.peer_b().peer_id(),
        ViewFederationRequest {
            view_kind: ViewKind::Cluster,
            agent_cid: "agent_test".into(),
            request_id: "req_002".into(),
        },
        std::time::Duration::from_millis(200),
    ).await;
    assert!(result.is_err());
}
```

- [ ] **Step 2: Run, expect fail**

```bash
RUSTFLAGS="" cargo test --test p2p_command_view_federate
```

- [ ] **Step 3: Add the command**

Edit `elohim/elohim-storage/src/p2p/mod.rs`:

```rust
pub enum P2PCommand {
    // ... existing
    ViewFederate {
        peer: PeerId,
        request: ViewFederationRequest,
        respond: oneshot::Sender<Result<ViewFederationResponse, FederationError>>,
    },
}

// In the swarm event loop dispatch:
match cmd {
    P2PCommand::ViewFederate { peer, request, respond } => {
        let req_id = swarm.behaviour_mut().view_federation.send_request(&peer, request);
        state.pending_view_federation.insert(req_id, respond);
    }
    // ...
}

// Handle response event:
SwarmEvent::Behaviour(BehaviourEvent::ViewFederation(request_response::Event::Message {
    message: request_response::Message::Response { request_id, response }, ..
})) => {
    if let Some(tx) = state.pending_view_federation.remove(&request_id) {
        let _ = tx.send(Ok(response));
    }
}
```

- [ ] **Step 4: Add SwarmClient method**

Edit `elohim/elohim-storage/src/p2p/client.rs`:

```rust
impl SwarmClient {
    pub async fn view_federate(
        &self,
        peer: PeerId,
        request: ViewFederationRequest,
        timeout: Duration,
    ) -> Result<ViewFederationResponse, FederationError> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(P2PCommand::ViewFederate {
            peer, request, respond: tx,
        }).await.map_err(|_| FederationError::SwarmGone)?;

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(response))) => Ok(response),
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(_)) => Err(FederationError::SwarmGone),
            Err(_) => Err(FederationError::Timeout),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FederationError {
    #[error("federation timeout")]
    Timeout,
    #[error("swarm channel closed")]
    SwarmGone,
    #[error("inbound request error")]
    InboundError,
}
```

- [ ] **Step 5: Run tests, commit**

```bash
RUSTFLAGS="" cargo test --test p2p_command_view_federate -- --nocapture
git commit -am "feat(p2p): P2PCommand::ViewFederate dispatch + SwarmClient::view_federate"
```

### Task T20: Slice handler (responder side)

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs`

- [ ] **Step 1: Failing test**

```rust
// in tests/p2p_command_view_federate.rs
#[tokio::test]
async fn responder_signs_slice_with_agent_key() {
    let harness = TwoPeerHarness::start().await;
    let response = harness.peer_a().swarm_client().view_federate(
        harness.peer_b().peer_id(),
        ViewFederationRequest {
            view_kind: ViewKind::Cluster,
            agent_cid: "agent_test".into(),
            request_id: "req_sign".into(),
        },
        std::time::Duration::from_secs(3),
    ).await.unwrap();

    let canonical = response.slice.canonical_bytes_for_signing();
    let sig_bytes = base64::decode(&response.slice.signature).unwrap();
    assert!(harness.peer_b().agent_key().verify(&canonical, &sig_bytes).is_ok());
}

#[tokio::test]
async fn responder_returns_offline_for_unknown_agent_cid() {
    let harness = TwoPeerHarness::start().await;
    let response = harness.peer_a().swarm_client().view_federate(
        harness.peer_b().peer_id(),
        ViewFederationRequest {
            view_kind: ViewKind::Cluster,
            agent_cid: "unknown_agent".into(),
            request_id: "req_unknown".into(),
        },
        std::time::Duration::from_secs(3),
    ).await.unwrap();

    assert_eq!(response.slice.freshness.state, FreshnessState::Offline);
}
```

- [ ] **Step 2: Implement the handler**

In `p2p/mod.rs` event loop, handle inbound request:

```rust
SwarmEvent::Behaviour(BehaviourEvent::ViewFederation(request_response::Event::Message {
    peer, message: request_response::Message::Request { request, channel, .. }, ..
})) => {
    let agent_cid = request.agent_cid.clone();
    let view_kind = request.view_kind.clone();
    let req_id = request.request_id.clone();
    let local_agent_cid = state.local_agent_cid.clone();
    let conn = state.conn_pool.clone();
    let agent_key = state.agent_signing_key.clone();
    let local_peer_id = swarm.local_peer_id().to_string();

    tokio::spawn(async move {
        // Verify the responder (us) can answer for this agent_cid
        let payload = if local_agent_cid == agent_cid {
            // Build local slice for the requested view_kind
            match view_kind {
                ViewKind::Cluster => services::cluster_view::build_local_slice(&conn).await,
                ViewKind::PeerTopology => services::peer_topology_view::build_local_slice(&conn).await,
            }
        } else {
            serde_json::Value::Null
        };

        let freshness_state = if payload.is_null() {
            FreshnessState::Offline
        } else {
            FreshnessState::Live
        };

        let mut slice = ViewSlice {
            peer_id: local_peer_id,
            view_kind: view_kind.clone(),
            freshness: Freshness { state: freshness_state, stale_since_ms: None },
            payload,
            signature: String::new(),
        };
        let canonical = slice.canonical_bytes_for_signing();
        let sig = agent_key.sign(&canonical);
        slice.signature = base64::encode(sig);

        let response = ViewFederationResponse {
            view_kind, agent_cid, request_id: req_id, slice,
        };
        let _ = swarm_handle.send_response(channel, response);
    });
}
```

- [ ] **Step 3: Run tests, commit**

```bash
RUSTFLAGS="" cargo test --test p2p_command_view_federate -- --nocapture
git commit -am "feat(p2p): view-federation responder handler with agent-key signing"
```

### Task T21: Federation aggregator (requester side)

**Files:**
- Create: `elohim/elohim-storage/src/services/federator.rs`

- [ ] **Step 1: Failing test**

Create `elohim/elohim-storage/tests/federator.rs`:

```rust
use elohim_storage::services::federator::{Federator, FederationResult};
use elohim_storage::views::*;

#[tokio::test]
async fn federator_returns_per_slice_freshness() {
    let harness = ThreePeerHarness::start().await;
    harness.peer_b().sigstop().await; // P_B offline

    let federator = Federator::new(harness.peer_a().swarm_client());
    let bindings = vec![
        harness.peer_a().binding(),
        harness.peer_b().binding(),
        harness.peer_c().binding(),
    ];
    let results = federator.query(
        ViewKind::Cluster,
        "agent_test",
        &bindings,
        std::time::Duration::from_secs(3),
    ).await;

    assert_eq!(results.len(), 3);
    let by_peer = |id: &str| results.iter().find(|r| r.peer_id == id).unwrap();
    assert_eq!(by_peer(harness.peer_a().peer_id_str()).freshness.state, FreshnessState::Live);
    assert_eq!(by_peer(harness.peer_b().peer_id_str()).freshness.state, FreshnessState::Offline);
    assert_eq!(by_peer(harness.peer_c().peer_id_str()).freshness.state, FreshnessState::Live);
}

#[tokio::test]
async fn federator_rejects_signature_mismatch() {
    let harness = TwoPeerHarness::start_with_lying_b().await;
    let federator = Federator::new(harness.peer_a().swarm_client());
    let bindings = vec![harness.peer_a().binding(), harness.peer_b().binding()];
    let results = federator.query(
        ViewKind::Cluster, "agent_test", &bindings,
        std::time::Duration::from_secs(3),
    ).await;
    let lying_result = results.iter().find(|r| r.peer_id == harness.peer_b().peer_id_str()).unwrap();
    assert_eq!(lying_result.freshness.state, FreshnessState::Unverifiable);
}
```

- [ ] **Step 2: Run, expect fail**

```bash
RUSTFLAGS="" cargo test --test federator
```

- [ ] **Step 3: Implement Federator**

Create `elohim/elohim-storage/src/services/federator.rs`:

```rust
//! ## Source of Truth
//!
//! Operational (Category C). Federation aggregator — composes view slices from
//! multiple peer storage instances. Per-slice signature verified against
//! DHT-notarized AgentPeerBindings before acceptance.

use std::time::Duration;
use crate::p2p::client::SwarmClient;
use crate::views::*;
use crate::db::peer_identity_bindings::PeerBinding;

pub struct Federator {
    swarm: SwarmClient,
}

#[derive(Debug, Clone)]
pub struct FederationResult {
    pub peer_id: String,
    pub freshness: Freshness,
    pub slice: Option<ViewSlice>, // None if signature reject or all-offline-cached-only
}

impl Federator {
    pub fn new(swarm: SwarmClient) -> Self { Self { swarm } }

    pub async fn query(
        &self,
        view_kind: ViewKind,
        agent_cid: &str,
        bindings: &[PeerBinding],
        per_peer_timeout: Duration,
    ) -> Vec<FederationResult> {
        let futures = bindings.iter().map(|binding| {
            let peer_id = binding.peer_id.clone();
            let agent_cid = agent_cid.to_string();
            let view_kind = view_kind.clone();
            let swarm = self.swarm.clone();
            let req_id = uuid::Uuid::new_v4().to_string();

            async move {
                let req = ViewFederationRequest {
                    view_kind: view_kind.clone(),
                    agent_cid: agent_cid.clone(),
                    request_id: req_id,
                };
                match swarm.view_federate(parse_peer_id(&peer_id), req, per_peer_timeout).await {
                    Ok(resp) => {
                        // Verify signature against binding's agent key
                        let canonical = resp.slice.canonical_bytes_for_signing();
                        let sig_bytes = base64::decode(&resp.slice.signature).ok();
                        if sig_bytes.is_none() || !verify_signature(&binding.agent_pub_key, &canonical, sig_bytes.as_ref().unwrap()) {
                            return FederationResult {
                                peer_id: peer_id.clone(),
                                freshness: Freshness { state: FreshnessState::Unverifiable, stale_since_ms: None },
                                slice: None,
                            };
                        }
                        FederationResult {
                            peer_id: peer_id.clone(),
                            freshness: resp.slice.freshness.clone(),
                            slice: Some(resp.slice),
                        }
                    }
                    Err(_) => FederationResult {
                        peer_id: peer_id.clone(),
                        freshness: Freshness {
                            state: FreshnessState::Offline,
                            stale_since_ms: binding.last_seen_ms,
                        },
                        slice: None,
                    },
                }
            }
        });
        futures::future::join_all(futures).await
    }
}

fn verify_signature(agent_key: &[u8], canonical: &[u8], sig: &[u8]) -> bool {
    use ed25519_dalek::{Verifier, VerifyingKey, Signature};
    let key = match VerifyingKey::from_bytes(agent_key.try_into().unwrap_or(&[0u8; 32])) {
        Ok(k) => k, Err(_) => return false,
    };
    let signature = match Signature::from_slice(sig) {
        Ok(s) => s, Err(_) => return false,
    };
    key.verify(canonical, &signature).is_ok()
}

fn parse_peer_id(s: &str) -> libp2p::PeerId {
    s.parse().expect("peer_id from binding must be valid")
}
```

- [ ] **Step 4: Run tests, commit**

```bash
RUSTFLAGS="" cargo test --test federator -- --nocapture
git commit -am "feat(services): Federator with signature verification + per-slice freshness"
```

---

## Phase 4 — View Services

### Task T22: bindings_resolver — auth → agent_cid → Vec<PeerBinding>

**Files:**
- Create: `elohim/elohim-storage/src/auth/bindings_resolver.rs`
- Modify: `elohim/elohim-storage/src/auth/mod.rs`

- [ ] **Step 1: Failing test**

Create `elohim/elohim-storage/tests/bindings_resolver.rs`:

```rust
use elohim_storage::auth::bindings_resolver::{resolve_bindings, BindingsError};

#[tokio::test]
async fn resolves_bindings_for_known_agent() {
    let conn = test_pool();
    seed_three_bindings_for(&conn, "agent_M", &[("P1", "desktop"), ("P2", "node"), ("P3", "mobile")]).await;

    let bindings = resolve_bindings(&conn, "agent_M").await.unwrap();
    assert_eq!(bindings.len(), 3);
    assert!(bindings.iter().any(|b| b.peer_id == "P1" && b.device_archetype == "desktop"));
}

#[tokio::test]
async fn filters_superseded_bindings() {
    let conn = test_pool();
    seed_superseded_binding(&conn, "agent_M", "P_old").await;
    seed_three_bindings_for(&conn, "agent_M", &[("P1", "desktop")]).await;

    let bindings = resolve_bindings(&conn, "agent_M").await.unwrap();
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].peer_id, "P1");
}

#[tokio::test]
async fn empty_bindings_for_unknown_agent() {
    let conn = test_pool();
    let bindings = resolve_bindings(&conn, "agent_unknown").await.unwrap();
    assert!(bindings.is_empty());
}
```

- [ ] **Step 2: Run, expect fail**

```bash
RUSTFLAGS="" cargo test --test bindings_resolver
```

- [ ] **Step 3: Implement**

Create `elohim/elohim-storage/src/auth/bindings_resolver.rs`:

```rust
//! ## Source of Truth
//!
//! Operational (Category C) read-projection over notarized AgentPeerBinding (Category A).
//! Reads from `peer_identity_bindings` projection table; the DHT remains canonical.

use diesel::prelude::*;
use thiserror::Error;
use crate::db::DbPool;

#[derive(Debug, Clone)]
pub struct PeerBinding {
    pub peer_id: String,
    pub agent_cid: String,
    pub device_archetype: String,
    pub agent_pub_key: Vec<u8>,
    pub valid_from_ms: i64,
    pub valid_until_ms: Option<i64>,
    pub last_seen_ms: Option<u64>,
}

#[derive(Debug, Error)]
pub enum BindingsError {
    #[error("db error: {0}")]
    Db(#[from] diesel::result::Error),
    #[error("pool error: {0}")]
    Pool(String),
}

pub async fn resolve_bindings(
    pool: &DbPool,
    agent_cid: &str,
) -> Result<Vec<PeerBinding>, BindingsError> {
    use crate::db::diesel_schema::peer_identity_bindings::dsl as p;
    let mut conn = pool.get().map_err(|e| BindingsError::Pool(e.to_string()))?;

    let rows: Vec<PeerBinding> = p::peer_identity_bindings
        .filter(p::agent_cid.eq(agent_cid))
        .filter(p::superseded_by.is_null())
        .select((
            p::peer_id, p::agent_cid, p::device_archetype,
            p::agent_pub_key, p::valid_from_ms, p::valid_until_ms,
            p::last_seen_ms,
        ))
        .load::<(String, String, String, Vec<u8>, i64, Option<i64>, Option<i64>)>(&mut conn)?
        .into_iter()
        .map(|(peer_id, agent_cid, device_archetype, agent_pub_key, valid_from_ms, valid_until_ms, last_seen_ms)| {
            PeerBinding {
                peer_id, agent_cid, device_archetype, agent_pub_key,
                valid_from_ms,
                valid_until_ms,
                last_seen_ms: last_seen_ms.map(|x| x as u64),
            }
        })
        .collect();

    Ok(rows)
}
```

- [ ] **Step 4: Run, commit**

```bash
RUSTFLAGS="" cargo test --test bindings_resolver -- --nocapture
git commit -am "feat(auth): bindings_resolver — agent_cid → PeerBinding[] (filters superseded)"
```

### Task T23: distribution_view::compose_summary

**Files:**
- Create: `elohim/elohim-storage/src/services/distribution_view.rs`

- [ ] **Step 1: Failing test**

Create `elohim/elohim-storage/tests/distribution_view.rs`:

```rust
use elohim_storage::services::distribution_view::{compose_distribution_summary, DistributionContext};
use elohim_storage::views::*;

#[tokio::test]
async fn public_reach_visitor_no_my_role() {
    let pool = seed_pool_with_blob_replicas("hash_pub", 12, ReachClass::Public).await;
    let summary = compose_distribution_summary(
        &pool, "hash_pub", DistributionContext::Visitor,
    ).await.unwrap();

    assert_eq!(summary.replica_count, 12);
    assert_eq!(summary.reach_class, ReachClass::Public);
    assert_eq!(summary.my_role, None);
    assert_eq!(summary.reciprocity_hint, None);
    assert!(matches!(summary.diversity_hint, DiversityHint::RegionMetro(_)));
}

#[tokio::test]
async fn intimate_reach_steward_renders_household_archetypes() {
    let pool = seed_pool_with_blob_replicas("hash_intimate", 4, ReachClass::Intimate).await;
    let bindings = vec![mk_binding("P_M_desktop", "desktop")];
    let summary = compose_distribution_summary(
        &pool, "hash_intimate", DistributionContext::Steward { agent_cid: "agent_M", bindings: &bindings },
    ).await.unwrap();

    match summary.diversity_hint {
        DiversityHint::HouseholdArchetypes(ref archs) => assert!(!archs.is_empty()),
        _ => panic!("intimate reach should yield HouseholdArchetypes"),
    }
}

#[tokio::test]
async fn replica_health_thresholds() {
    // healthy: replicaCount >= replicaTarget * 0.85
    // at_risk: between 0.5 and 0.85
    // critical: < 0.5
    assert_eq!(replica_health_for(14, 14), ReplicaHealth::Healthy);
    assert_eq!(replica_health_for(12, 14), ReplicaHealth::Healthy);  // 86%
    assert_eq!(replica_health_for(10, 14), ReplicaHealth::AtRisk);   // 71%
    assert_eq!(replica_health_for(5, 14), ReplicaHealth::Critical);  // 36%
}

#[tokio::test]
async fn my_role_replica_and_projector_when_binding_in_both_sets() {
    let pool = pool_with_replica_and_projector_for("hash_x", "P_M_desktop").await;
    let bindings = vec![mk_binding("P_M_desktop", "desktop")];
    let summary = compose_distribution_summary(
        &pool, "hash_x", DistributionContext::Steward { agent_cid: "agent_M", bindings: &bindings },
    ).await.unwrap();
    assert_eq!(summary.my_role, Some(MyRole::ReplicaAndProjector));
}
```

- [ ] **Step 2: Run, expect fail**

- [ ] **Step 3: Implement**

Create `elohim/elohim-storage/src/services/distribution_view.rs`:

```rust
//! ## Source of Truth
//!
//! Operational (Category C). Composes a read-projection from notarized DHT state.
//! All counts derive from existing tables; no caching, no materialization.

use diesel::prelude::*;
use crate::db::DbPool;
use crate::views::*;
use crate::auth::bindings_resolver::PeerBinding;
use std::time::SystemTime;

pub enum DistributionContext<'a> {
    Visitor,
    Steward { agent_cid: &'a str, bindings: &'a [PeerBinding] },
}

#[derive(Debug, thiserror::Error)]
pub enum ViewError {
    #[error("db error: {0}")]
    Db(#[from] diesel::result::Error),
    #[error("pool error: {0}")]
    Pool(String),
    #[error("content not found: {0}")]
    NotFound(String),
}

pub async fn compose_distribution_summary(
    pool: &DbPool,
    blob_hash: &str,
    ctx: DistributionContext<'_>,
) -> Result<DistributionSummary, ViewError> {
    let mut conn = pool.get().map_err(|e| ViewError::Pool(e.to_string()))?;

    let replica_peers = load_replica_peer_ids(&mut conn, blob_hash)?;
    let projector_acks = load_projector_acks(&mut conn, blob_hash)?;
    let reach_class = load_reach_class(&mut conn, blob_hash)?;
    let replica_target = load_replica_target(&mut conn, blob_hash)?;
    let last_verified_seconds = load_last_verified_seconds(&mut conn, blob_hash)?;

    let replica_count = replica_peers.len() as u32;
    let projector_count = projector_acks.len() as u32;
    let replica_health = replica_health_for(replica_count, replica_target);
    let diversity_hint = compose_diversity_hint(&reach_class, &replica_peers, &mut conn)?;
    let this_fetch_source = FetchSource::ProjectedViaDoorway;

    let (my_role, reciprocity_hint) = match ctx {
        DistributionContext::Visitor => (None, None),
        DistributionContext::Steward { agent_cid, bindings } => {
            let role = compute_my_role(&replica_peers, &projector_acks, bindings);
            let recip = compute_reciprocity_hint(&mut conn, agent_cid, bindings)?;
            (Some(role), Some(recip))
        }
    };

    Ok(DistributionSummary {
        replica_count, replica_target, replica_health,
        projector_count, reach_class, diversity_hint,
        this_fetch_source, last_verified_seconds,
        my_role, reciprocity_hint,
    })
}

pub fn replica_health_for(count: u32, target: u32) -> ReplicaHealth {
    if target == 0 { return ReplicaHealth::Healthy; }
    let ratio = count as f64 / target as f64;
    if ratio >= 0.85 { ReplicaHealth::Healthy }
    else if ratio >= 0.5 { ReplicaHealth::AtRisk }
    else { ReplicaHealth::Critical }
}

fn compute_my_role(replica_peers: &[String], projector_acks: &[String], bindings: &[PeerBinding]) -> MyRole {
    let my_peer_ids: std::collections::HashSet<&str> = bindings.iter().map(|b| b.peer_id.as_str()).collect();
    let any_replica = replica_peers.iter().any(|p| my_peer_ids.contains(p.as_str()));
    let any_projector = projector_acks.iter().any(|p| my_peer_ids.contains(p.as_str()));
    let total_replicas = replica_peers.len();

    match (any_replica, any_projector, total_replicas) {
        (true, true, _) => MyRole::ReplicaAndProjector,
        (true, false, 1) => MyRole::SoleReplica,
        (true, false, _) => MyRole::Replica,
        _ => MyRole::NotHosting,
    }
}

fn compute_reciprocity_hint(
    conn: &mut diesel::SqliteConnection,
    _agent_cid: &str,
    bindings: &[PeerBinding],
) -> Result<i64, ViewError> {
    use crate::db::diesel_schema::custodian_blob_commitments::dsl as c;
    let my_peer_ids: Vec<String> = bindings.iter().map(|b| b.peer_id.clone()).collect();
    let outflow: i64 = c::custodian_blob_commitments
        .filter(c::custodian_id.eq_any(&my_peer_ids))
        .select(diesel::dsl::sum(c::committed_bytes))
        .first::<Option<i64>>(conn)?
        .unwrap_or(0);
    let inflow: i64 = c::custodian_blob_commitments
        .filter(c::beneficiary_peer.eq_any(&my_peer_ids))
        .select(diesel::dsl::sum(c::committed_bytes))
        .first::<Option<i64>>(conn)?
        .unwrap_or(0);
    Ok(outflow - inflow)
}

fn compose_diversity_hint(
    reach: &ReachClass,
    replica_peers: &[String],
    conn: &mut diesel::SqliteConnection,
) -> Result<DiversityHint, ViewError> {
    match reach {
        ReachClass::Public | ReachClass::District | ReachClass::Community => {
            Ok(DiversityHint::RegionMetro(top_region_tiers(conn, replica_peers, 3)?))
        }
        ReachClass::Collective => {
            Ok(DiversityHint::CollectiveMemberCount(collective_member_count(conn, replica_peers)?))
        }
        ReachClass::Intimate | ReachClass::Household | ReachClass::Neighborhood => {
            Ok(DiversityHint::HouseholdArchetypes(household_archetypes(conn, replica_peers)?))
        }
        ReachClass::Private => Ok(DiversityHint::None),
    }
}

// ---- DB helpers (one query each, returning the minimum needed) ----

fn load_replica_peer_ids(conn: &mut diesel::SqliteConnection, blob_hash: &str) -> Result<Vec<String>, ViewError> {
    use crate::db::diesel_schema::custodian_blob_commitments::dsl as c;
    Ok(c::custodian_blob_commitments
        .filter(c::blob_hash.eq(blob_hash))
        .filter(c::status.eq_any(vec!["healthy", "probing"]))
        .select(c::custodian_id)
        .load::<String>(conn)?)
}

fn load_projector_acks(conn: &mut diesel::SqliteConnection, blob_hash: &str) -> Result<Vec<String>, ViewError> {
    use crate::db::diesel_schema::rea_projection::dsl as r;
    Ok(r::rea_projection
        .filter(r::cid.eq(blob_hash))
        .filter(r::ack_count.gt(0))
        .select(r::projector_peer_id)
        .load::<String>(conn)?)
}

fn load_reach_class(conn: &mut diesel::SqliteConnection, blob_hash: &str) -> Result<ReachClass, ViewError> {
    use crate::db::diesel_schema::content_store::dsl as cs;
    let raw: String = cs::content_store
        .filter(cs::blob_hash.eq(blob_hash))
        .select(cs::reach_class)
        .first::<String>(conn)?;
    parse_reach_class(&raw).ok_or_else(|| ViewError::NotFound(format!("unknown reach: {}", raw)))
}

fn load_replica_target(_conn: &mut diesel::SqliteConnection, _blob_hash: &str) -> Result<u32, ViewError> {
    // RS coding policy: 14 for full-reach public, smaller for narrower reach.
    // Phase 1: hard-code per reach class. Future: read from manifest.
    Ok(14)
}

fn load_last_verified_seconds(conn: &mut diesel::SqliteConnection, blob_hash: &str) -> Result<u64, ViewError> {
    use crate::db::diesel_schema::custodian_blob_commitments::dsl as c;
    let max_ts: Option<i64> = c::custodian_blob_commitments
        .filter(c::blob_hash.eq(blob_hash))
        .select(diesel::dsl::max(c::last_verified_at))
        .first::<Option<i64>>(conn)?;

    let now_s = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64;
    Ok(((now_s - max_ts.unwrap_or(now_s)).max(0)) as u64)
}

fn parse_reach_class(s: &str) -> Option<ReachClass> {
    match s {
        "private" => Some(ReachClass::Private),
        "intimate" => Some(ReachClass::Intimate),
        "household" => Some(ReachClass::Household),
        "neighborhood" => Some(ReachClass::Neighborhood),
        "collective" => Some(ReachClass::Collective),
        "community" => Some(ReachClass::Community),
        "district" => Some(ReachClass::District),
        "public" => Some(ReachClass::Public),
        _ => None,
    }
}

fn top_region_tiers(_: &mut diesel::SqliteConnection, _: &[String], _n: u32) -> Result<Vec<String>, ViewError> {
    // Phase 1: stub. Real implementation joins with a peer-region projection.
    Ok(vec!["us-central".into(), "eu-west".into()])
}

fn collective_member_count(_: &mut diesel::SqliteConnection, _: &[String]) -> Result<u32, ViewError> {
    Ok(0)
}

fn household_archetypes(_: &mut diesel::SqliteConnection, _: &[String]) -> Result<Vec<String>, ViewError> {
    Ok(vec!["desktop".into(), "node".into()])
}
```

- [ ] **Step 4: Run tests, commit**

```bash
RUSTFLAGS="" cargo test --test distribution_view -- --nocapture
git commit -am "feat(services): distribution_view::compose_distribution_summary"
```

### Task T24: distribution_view::compose_details (lazy fetch)

**Files:**
- Modify: `elohim/elohim-storage/src/services/distribution_view.rs`

- [ ] **Step 1: Failing test**

```rust
#[tokio::test]
async fn details_includes_full_replica_peers() {
    let pool = seed_pool_with_blob_replicas("hash_x", 5, ReachClass::Public).await;
    let details = compose_distribution_details(
        &pool, "hash_x", DistributionContext::Visitor,
    ).await.unwrap();
    assert_eq!(details.summary.replica_count, 5);
    assert_eq!(details.replica_peers.len(), 5);
    for p in &details.replica_peers {
        assert!(matches!(p.device_archetype, DeviceArchetype::Node | DeviceArchetype::Desktop | DeviceArchetype::Mobile | DeviceArchetype::Steward));
    }
}

#[tokio::test]
async fn details_steward_includes_reciprocity_edges() {
    let pool = seed_pool_with_blob_replicas("hash_y", 5, ReachClass::Public).await;
    let bindings = vec![mk_binding("P_M_desktop", "desktop")];
    let details = compose_distribution_details(
        &pool, "hash_y", DistributionContext::Steward { agent_cid: "agent_M", bindings: &bindings },
    ).await.unwrap();
    assert!(details.reciprocity_edges.is_some());
    assert!(details.commitment_references.is_some());
}
```

- [ ] **Step 2: Implement**

Append to `services/distribution_view.rs`:

```rust
pub async fn compose_distribution_details(
    pool: &DbPool,
    blob_hash: &str,
    ctx: DistributionContext<'_>,
) -> Result<DistributionDetails, ViewError> {
    let summary = compose_distribution_summary(pool, blob_hash, ctx_for_summary(&ctx)).await?;
    let mut conn = pool.get().map_err(|e| ViewError::Pool(e.to_string()))?;

    let replica_peers = load_replica_peers_full(&mut conn, blob_hash)?;
    let projector_identities = load_projector_identities(&mut conn, blob_hash)?;
    let placement_gaps = load_placement_gaps(&mut conn, blob_hash)?;
    let recent_projection_events = load_recent_projection_events(&mut conn, blob_hash, 50)?;

    let (reciprocity_edges, commitment_references) = match ctx {
        DistributionContext::Visitor => (None, None),
        DistributionContext::Steward { bindings, .. } => {
            let edges = load_reciprocity_edges(&mut conn, bindings)?;
            let refs = load_commitment_refs(&mut conn, blob_hash, bindings)?;
            (Some(edges), Some(refs))
        }
    };

    Ok(DistributionDetails {
        summary, replica_peers, projector_identities,
        placement_gaps, recent_projection_events,
        reciprocity_edges, commitment_references,
    })
}

fn ctx_for_summary<'a>(ctx: &'a DistributionContext<'a>) -> DistributionContext<'a> {
    match ctx {
        DistributionContext::Visitor => DistributionContext::Visitor,
        DistributionContext::Steward { agent_cid, bindings } =>
            DistributionContext::Steward { agent_cid, bindings },
    }
}

fn load_replica_peers_full(conn: &mut diesel::SqliteConnection, blob_hash: &str) -> Result<Vec<ReplicaPeer>, ViewError> {
    use crate::db::diesel_schema::custodian_blob_commitments::dsl as c;
    use crate::db::diesel_schema::peer_identity_bindings::dsl as b;

    // Join custodian → bindings to get archetype
    let rows: Vec<(String, Option<String>, Option<i64>)> = c::custodian_blob_commitments
        .filter(c::blob_hash.eq(blob_hash))
        .left_join(b::peer_identity_bindings.on(b::peer_id.eq(c::custodian_id)))
        .select((c::custodian_id, b::device_archetype.nullable(), c::last_verified_at.nullable()))
        .load(conn)?;

    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64;
    Ok(rows.into_iter().map(|(peer_id, archetype, last_v)| ReplicaPeer {
        peer_id,
        device_archetype: parse_archetype(&archetype.unwrap_or("node".into())),
        last_seen_seconds: ((now - last_v.unwrap_or(now)).max(0)) as u64,
        hop_hint: None,
        household_id: None,
        region_tier: None,
    }).collect())
}

fn parse_archetype(s: &str) -> DeviceArchetype {
    match s {
        "node" => DeviceArchetype::Node,
        "desktop" => DeviceArchetype::Desktop,
        "mobile" => DeviceArchetype::Mobile,
        "steward" => DeviceArchetype::Steward,
        _ => DeviceArchetype::Node,
    }
}

// Stubs for sub-loaders (real implementations join with rea_projection / rea_commitments).
fn load_projector_identities(_: &mut diesel::SqliteConnection, _: &str) -> Result<Vec<ProjectorIdentity>, ViewError> { Ok(vec![]) }
fn load_placement_gaps(_: &mut diesel::SqliteConnection, _: &str) -> Result<Vec<serde_json::Value>, ViewError> { Ok(vec![]) }
fn load_recent_projection_events(_: &mut diesel::SqliteConnection, _: &str, _n: u32) -> Result<Vec<serde_json::Value>, ViewError> { Ok(vec![]) }
fn load_reciprocity_edges(_: &mut diesel::SqliteConnection, _: &[PeerBinding]) -> Result<Vec<PeerHouseholdEdge>, ViewError> { Ok(vec![]) }
fn load_commitment_refs(_: &mut diesel::SqliteConnection, _: &str, _: &[PeerBinding]) -> Result<Vec<String>, ViewError> { Ok(vec![]) }
```

- [ ] **Step 3: Run, commit**

```bash
RUSTFLAGS="" cargo test --test distribution_view distribution_details -- --nocapture
git commit -am "feat(services): distribution_view::compose_distribution_details (lazy)"
```

### Task T25: cluster_view::aggregate (federated)

**Files:**
- Create: `elohim/elohim-storage/src/services/cluster_view.rs`

- [ ] **Step 1: Failing test**

```rust
// tests/cluster_view.rs
#[tokio::test]
async fn cluster_view_three_devices_all_live() {
    let harness = ThreePeerHarness::start_with_bindings("agent_M",
        &[("desktop", true), ("node", true), ("mobile", true)]).await;
    let view = harness.peer_a().http_get_cluster_me("agent_M").await.unwrap();
    assert_eq!(view.devices.len(), 3);
    assert!(view.devices.iter().all(|d| d.online));
    assert!(matches!(view.freshness.state, FreshnessState::Live));
}

#[tokio::test]
async fn cluster_view_one_device_offline() {
    let harness = ThreePeerHarness::start_with_bindings("agent_M",
        &[("desktop", true), ("node", false), ("mobile", true)]).await;
    let view = harness.peer_a().http_get_cluster_me("agent_M").await.unwrap();
    let offline = view.devices.iter().find(|d| !d.online).unwrap();
    assert_eq!(offline.archetype, DeviceArchetype::Node);
    assert!(offline.freshness.stale_since_ms.is_some());
}

#[tokio::test]
async fn cluster_view_all_offline_returns_all_offline_freshness() {
    let harness = ThreePeerHarness::start_all_offline_for("agent_M").await;
    let view = harness.peer_a().http_get_cluster_me("agent_M").await.unwrap();
    assert_eq!(view.freshness.state, FreshnessState::AllOffline);
}
```

- [ ] **Step 2: Implement**

Create `elohim/elohim-storage/src/services/cluster_view.rs`:

```rust
//! ## Source of Truth
//!
//! Operational (Category C). Federated query result. Bindings notarized in DHT
//! (Category A). Per-device live state federated via /elohim/view-federation/1.0.0.

use std::time::Duration;
use crate::db::DbPool;
use crate::services::federator::{Federator, FederationResult};
use crate::auth::bindings_resolver::{resolve_bindings, PeerBinding};
use crate::views::*;

const FEDERATION_TIMEOUT_MS: u64 = 3000;

#[derive(Debug, thiserror::Error)]
pub enum ClusterError {
    #[error("auth: {0}")]
    Auth(#[from] crate::auth::bindings_resolver::BindingsError),
    #[error("federation: {0}")]
    Federation(String),
}

pub async fn aggregate_cluster_view(
    pool: &DbPool,
    federator: &Federator,
    agent_cid: &str,
) -> Result<MyClusterView, ClusterError> {
    let bindings = resolve_bindings(pool, agent_cid).await?;

    if bindings.is_empty() {
        return Ok(MyClusterView {
            agent_cid: agent_cid.into(),
            devices: vec![],
            totals: DeviceTotals { storage_used_bytes: 0, storage_total_bytes: 0, external_committed_bytes: 0, reciprocity_net_bytes: 0 },
            freshness: Freshness { state: FreshnessState::Live, stale_since_ms: None },
        });
    }

    let timeout = std::env::var("ELOHIM_FEDERATION_TIMEOUT_MS")
        .ok().and_then(|s| s.parse().ok())
        .unwrap_or(FEDERATION_TIMEOUT_MS);

    let results = federator.query(
        ViewKind::Cluster, agent_cid, &bindings,
        Duration::from_millis(timeout),
    ).await;

    let devices: Vec<DeviceSummary> = results.iter().map(|r| device_summary_from_result(r, &bindings)).collect();

    let mut conn = pool.get().map_err(|e| ClusterError::Federation(format!("pool: {e}")))?;
    let totals = compose_totals(&mut conn, &devices, agent_cid);
    let any_live = devices.iter().any(|d| matches!(d.freshness.state, FreshnessState::Live));
    let freshness = if !any_live {
        Freshness { state: FreshnessState::AllOffline, stale_since_ms: None }
    } else {
        Freshness { state: FreshnessState::Live, stale_since_ms: None }
    };

    Ok(MyClusterView { agent_cid: agent_cid.into(), devices, totals, freshness })
}

fn device_summary_from_result(r: &FederationResult, bindings: &[PeerBinding]) -> DeviceSummary {
    let binding = bindings.iter().find(|b| b.peer_id == r.peer_id).cloned();
    let archetype = binding.as_ref().map(|b| parse_archetype(&b.device_archetype)).unwrap_or(DeviceArchetype::Node);

    match (&r.freshness.state, &r.slice) {
        (FreshnessState::Live, Some(slice)) => {
            let payload = &slice.payload;
            DeviceSummary {
                peer_id: r.peer_id.clone(),
                archetype,
                display_name: payload.get("display_name").and_then(|v| v.as_str()).map(String::from),
                online: true,
                freshness: r.freshness.clone(),
                storage_used_bytes: payload.get("storage_used_bytes").and_then(|v| v.as_u64()),
                storage_total_bytes: payload.get("storage_total_bytes").and_then(|v| v.as_u64()),
                memory_used_bytes: payload.get("memory_used_bytes").and_then(|v| v.as_u64()),
                memory_total_bytes: payload.get("memory_total_bytes").and_then(|v| v.as_u64()),
                hosting_count: payload.get("hosting_count").and_then(|v| v.as_u64()).map(|x| x as u32),
                projecting_count: payload.get("projecting_count").and_then(|v| v.as_u64()).map(|x| x as u32),
                beacon_age_ms: payload.get("beacon_age_ms").and_then(|v| v.as_u64()),
            }
        }
        _ => DeviceSummary {
            peer_id: r.peer_id.clone(),
            archetype,
            display_name: None,
            online: false,
            freshness: r.freshness.clone(),
            storage_used_bytes: None, storage_total_bytes: None,
            memory_used_bytes: None, memory_total_bytes: None,
            hosting_count: None, projecting_count: None,
            beacon_age_ms: None,
        },
    }
}

fn parse_archetype(s: &str) -> DeviceArchetype {
    match s {
        "node" => DeviceArchetype::Node,
        "desktop" => DeviceArchetype::Desktop,
        "mobile" => DeviceArchetype::Mobile,
        "steward" => DeviceArchetype::Steward,
        _ => DeviceArchetype::Node,
    }
}

fn compose_totals(
    conn: &mut diesel::SqliteConnection,
    devices: &[DeviceSummary],
    agent_cid: &str,
) -> DeviceTotals {
    use elohim_storage::schema::rea_commitments::dsl as rc;
    use diesel::prelude::*;

    let committed: i64 = rc::rea_commitments
        .filter(rc::committer_agent_cid.eq(agent_cid))
        .select(diesel::dsl::sum(rc::quantity_bytes))
        .first::<Option<i64>>(conn)
        .unwrap_or(Some(0))
        .unwrap_or(0);

    DeviceTotals {
        storage_used_bytes: devices.iter().filter_map(|d| d.storage_used_bytes).sum(),
        storage_total_bytes: devices.iter().filter_map(|d| d.storage_total_bytes).sum(),
        external_committed_bytes: committed.max(0) as u64,
        reciprocity_net_bytes: 0,
    }
}

/// Build the local slice that responder uses when handling a cluster federation request.
pub async fn build_local_slice(pool: &DbPool) -> serde_json::Value {
    let mut conn = pool.get().expect("pool");
    serde_json::json!({
        "display_name": std::env::var("ELOHIM_DISPLAY_NAME").unwrap_or_default(),
        "storage_used_bytes": local_storage_used_bytes(&mut conn).unwrap_or(0),
        "storage_total_bytes": local_storage_total_bytes().unwrap_or(0),
        "hosting_count": local_hosting_count(&mut conn).unwrap_or(0),
        "projecting_count": local_projecting_count(&mut conn).unwrap_or(0),
        "beacon_age_ms": 0,
    })
}

fn local_storage_used_bytes(_: &mut diesel::SqliteConnection) -> Result<u64, diesel::result::Error> { Ok(0) }
fn local_storage_total_bytes() -> Result<u64, std::io::Error> { Ok(0) }
fn local_hosting_count(_: &mut diesel::SqliteConnection) -> Result<u32, diesel::result::Error> { Ok(0) }
fn local_projecting_count(_: &mut diesel::SqliteConnection) -> Result<u32, diesel::result::Error> { Ok(0) }
```

- [ ] **Step 3: Run, commit**

```bash
RUSTFLAGS="" cargo test --test cluster_view -- --nocapture
git commit -am "feat(services): cluster_view::aggregate (federated; offline → stale_since)"
```

### Task T26: peer_topology_view::aggregate (federated)

**Files:**
- Create: `elohim/elohim-storage/src/services/peer_topology_view.rs`

- [ ] **Step 1: Failing test**

```rust
// tests/peer_topology_view.rs
#[tokio::test]
async fn peer_topology_dedups_households_across_bindings() {
    let harness = ThreePeerHarness::start_with_bindings_and_shared_neighbor("agent_M").await;
    let view = harness.peer_a().http_get_peer_topology_me("agent_M").await.unwrap();
    let counts: std::collections::HashSet<&str> = view.edges.iter().map(|e| e.household_id.as_str()).collect();
    assert_eq!(counts.len(), view.edges.len(), "no duplicate households");
}

#[tokio::test]
async fn peer_topology_identifies_resilience_cliffs() {
    let harness = TwoExternalHarness::start_where_adam_is_sole_external_replica_for_2_cids().await;
    let view = harness.peer_a().http_get_peer_topology_me("agent_M").await.unwrap();
    let cliff = view.resilience_cliffs.iter().find(|c| c.household_id == "adam-household").unwrap();
    assert_eq!(cliff.sole_replica_cid_count, 2);
}
```

- [ ] **Step 2: Implement**

Create `elohim/elohim-storage/src/services/peer_topology_view.rs`:

```rust
//! ## Source of Truth
//!
//! Operational (Category C). Federated query result, partial: each binding's storage
//! reports its connected peer set. Aggregator dedupes by household.

use std::time::Duration;
use std::collections::HashMap;
use crate::db::DbPool;
use crate::services::federator::Federator;
use crate::auth::bindings_resolver::resolve_bindings;
use crate::views::*;

const FEDERATION_TIMEOUT_MS: u64 = 3000;

#[derive(Debug, thiserror::Error)]
pub enum PeerTopologyError {
    #[error("auth: {0}")]
    Auth(#[from] crate::auth::bindings_resolver::BindingsError),
}

pub async fn aggregate_peer_topology_view(
    pool: &DbPool,
    federator: &Federator,
    agent_cid: &str,
) -> Result<PeerTopologyView, PeerTopologyError> {
    let bindings = resolve_bindings(pool, agent_cid).await?;
    if bindings.is_empty() {
        return Ok(PeerTopologyView {
            agent_cid: agent_cid.into(),
            edges: vec![],
            reciprocation_count: 0,
            resilience_cliffs: vec![],
            freshness: Freshness { state: FreshnessState::Live, stale_since_ms: None },
        });
    }

    let timeout = std::env::var("ELOHIM_FEDERATION_TIMEOUT_MS")
        .ok().and_then(|s| s.parse().ok())
        .unwrap_or(FEDERATION_TIMEOUT_MS);

    let results = federator.query(ViewKind::PeerTopology, agent_cid, &bindings, Duration::from_millis(timeout)).await;

    // Dedupe edges by household_id, summing counts
    let mut edges_by_household: HashMap<String, PeerHouseholdEdge> = HashMap::new();
    for r in &results {
        if let Some(slice) = &r.slice {
            if let Some(connected) = slice.payload.get("connected_peer_households").and_then(|v| v.as_array()) {
                for entry in connected {
                    let hh_id = entry.get("household_id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                    let edge = edges_by_household.entry(hh_id.clone()).or_insert_with(|| PeerHouseholdEdge {
                        household_id: hh_id, display_name: None, online: false,
                        last_sync_sec: None,
                        my_cids_hosted_by_them: 0, their_cids_hosted_by_me: 0,
                        net_diff: 0, is_critical_for_me: None, i_am_critical_for_them: None,
                    });
                    edge.online = edge.online || entry.get("online").and_then(|v| v.as_bool()).unwrap_or(false);
                    edge.my_cids_hosted_by_them += entry.get("my_cids_hosted_by_them").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    edge.their_cids_hosted_by_me += entry.get("their_cids_hosted_by_me").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                }
            }
        }
    }

    let mut edges: Vec<PeerHouseholdEdge> = edges_by_household.into_values().collect();
    for e in &mut edges {
        e.net_diff = e.their_cids_hosted_by_me as i64 - e.my_cids_hosted_by_them as i64;
    }

    let resilience_cliffs = compute_resilience_cliffs(pool, &bindings, &edges).await.unwrap_or_default();
    let reciprocation_count = edges.iter().filter(|e| e.online).count() as u32;

    Ok(PeerTopologyView {
        agent_cid: agent_cid.into(),
        edges, reciprocation_count, resilience_cliffs,
        freshness: Freshness { state: FreshnessState::Live, stale_since_ms: None },
    })
}

async fn compute_resilience_cliffs(
    _pool: &DbPool,
    _bindings: &[crate::auth::bindings_resolver::PeerBinding],
    _edges: &[PeerHouseholdEdge],
) -> Result<Vec<ResilienceCliff>, ()> {
    // Real impl: for each foreign household, count CIDs where the only external
    // replica is that household. Stub for Phase 4; populated in subsequent task.
    Ok(vec![])
}

pub async fn build_local_slice(_pool: &DbPool) -> serde_json::Value {
    serde_json::json!({
        "connected_peer_households": []
    })
}
```

- [ ] **Step 3: Run, commit**

```bash
RUSTFLAGS="" cargo test --test peer_topology_view -- --nocapture
git commit -am "feat(services): peer_topology_view::aggregate (federated; household dedup)"
```

### Task T27: reciprocity_view::aggregate (no federation)

**Files:**
- Create: `elohim/elohim-storage/src/services/reciprocity_view.rs`

- [ ] **Step 1: Failing test**

```rust
// tests/reciprocity_view.rs
#[tokio::test]
async fn reciprocity_inflow_outflow_split() {
    let pool = seed_pool_with_rea_commitments(&[
        ("agent_M_P1", "agent_adam", 14_000_000_000, 11_200_000_000), // out
        ("agent_adam", "agent_M_P1", 18_000_000_000, 14_400_000_000), // in
    ]).await;
    let bindings = vec![mk_binding_with_agent("P1", "desktop", "agent_M")];
    let view = aggregate_reciprocity_view(&pool, "agent_M", &bindings).await.unwrap();

    assert_eq!(view.outflow.len(), 1);
    assert_eq!(view.inflow.len(), 1);
    assert_eq!(view.outflow[0].counterparty_household_id, "agent_adam");
    assert_eq!(view.outflow[0].committed_bytes, 14_000_000_000);
    assert!((view.outflow[0].honored_percent - 80.0).abs() < 0.5);
}

#[tokio::test]
async fn reciprocity_over_delivered_flagged() {
    let pool = seed_pool_with_rea_commitments(&[
        ("agent_M", "agent_frank", 3_000_000_000, 3_100_000_000),
    ]).await;
    let bindings = vec![mk_binding_with_agent("P1", "desktop", "agent_M")];
    let view = aggregate_reciprocity_view(&pool, "agent_M", &bindings).await.unwrap();
    assert!(view.outflow[0].honored_percent > 100.0);
}
```

- [ ] **Step 2: Implement**

Create `elohim/elohim-storage/src/services/reciprocity_view.rs`:

```rust
//! ## Source of Truth
//!
//! Operational (Category C) read-projection over notarized REA Commitments + EconomicEvents
//! (Category A, content_store zome). DHT is authoritative; no federation needed.

use diesel::prelude::*;
use crate::db::DbPool;
use crate::auth::bindings_resolver::PeerBinding;
use crate::views::*;

#[derive(Debug, thiserror::Error)]
pub enum ReciprocityError {
    #[error("db: {0}")]
    Db(#[from] diesel::result::Error),
    #[error("pool: {0}")]
    Pool(String),
}

pub async fn aggregate_reciprocity_view(
    pool: &DbPool,
    agent_cid: &str,
    bindings: &[PeerBinding],
) -> Result<ReciprocityView, ReciprocityError> {
    let mut conn = pool.get().map_err(|e| ReciprocityError::Pool(e.to_string()))?;

    // The "binding agent set" is the agent_cids associated with each device (typically all
    // bindings share one agent_cid, but be defensive).
    let agent_set: Vec<String> = std::iter::once(agent_cid.to_string())
        .chain(bindings.iter().map(|b| b.agent_cid.clone()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter().collect();

    use crate::db::diesel_schema::rea_commitments::dsl as c;
    use crate::db::diesel_schema::rea_economic_events::dsl as e;

    // Outflow: I committed (committer ∈ agent_set)
    let outflow_rows: Vec<(String, i64, Option<i64>)> = c::rea_commitments
        .left_join(e::rea_economic_events.on(e::commitment_id.eq(c::id)))
        .filter(c::committer_agent_cid.eq_any(&agent_set))
        .group_by((c::beneficiary_agent_cid,))
        .select((
            c::beneficiary_agent_cid,
            diesel::dsl::sum(c::committed_bytes).nullable(),
            diesel::dsl::sum(e::delivered_bytes).nullable(),
        ))
        .load::<(String, Option<i64>, Option<i64>)>(&mut conn)?
        .into_iter()
        .map(|(c_id, committed, delivered)| (c_id, committed.unwrap_or(0), delivered))
        .collect();

    let outflow: Vec<ReciprocityRow> = outflow_rows.into_iter().map(|(counterparty, committed, delivered)| {
        let delivered = delivered.unwrap_or(0).max(0) as u64;
        let committed = committed.max(0) as u64;
        let honored = if committed == 0 { 0.0 } else { (delivered as f64 / committed as f64) * 100.0 };
        ReciprocityRow {
            counterparty_household_id: counterparty,
            display_name: None,
            committed_bytes: committed, delivered_bytes: delivered,
            honored_percent: honored, online: None,
        }
    }).collect();

    // Inflow: others committed to me (beneficiary ∈ agent_set)
    let inflow_rows: Vec<(String, i64, Option<i64>)> = c::rea_commitments
        .left_join(e::rea_economic_events.on(e::commitment_id.eq(c::id)))
        .filter(c::beneficiary_agent_cid.eq_any(&agent_set))
        .group_by((c::committer_agent_cid,))
        .select((
            c::committer_agent_cid,
            diesel::dsl::sum(c::committed_bytes).nullable(),
            diesel::dsl::sum(e::delivered_bytes).nullable(),
        ))
        .load::<(String, Option<i64>, Option<i64>)>(&mut conn)?
        .into_iter()
        .map(|(c_id, committed, delivered)| (c_id, committed.unwrap_or(0), delivered))
        .collect();

    let inflow: Vec<ReciprocityRow> = inflow_rows.into_iter().map(|(counterparty, committed, delivered)| {
        let delivered = delivered.unwrap_or(0).max(0) as u64;
        let committed = committed.max(0) as u64;
        let honored = if committed == 0 { 0.0 } else { (delivered as f64 / committed as f64) * 100.0 };
        ReciprocityRow {
            counterparty_household_id: counterparty,
            display_name: None,
            committed_bytes: committed, delivered_bytes: delivered,
            honored_percent: honored, online: None,
        }
    }).collect();

    let total_inflow_delivered: i64 = inflow.iter().map(|r| r.delivered_bytes as i64).sum();
    let total_outflow_delivered: i64 = outflow.iter().map(|r| r.delivered_bytes as i64).sum();
    let net_hosted_bytes = total_inflow_delivered - total_outflow_delivered;

    let capacity_available_bytes = compute_capacity_available(&mut conn, &agent_set)?;

    Ok(ReciprocityView {
        agent_cid: agent_cid.into(),
        inflow, outflow,
        net_hosted_bytes,
        capacity_available_bytes,
    })
}

fn compute_capacity_available(_: &mut diesel::SqliteConnection, _: &[String]) -> Result<u64, diesel::result::Error> {
    // Placeholder; real impl reads device capacity totals minus committed.
    Ok(0)
}
```

- [ ] **Step 3: Run, commit**

```bash
RUSTFLAGS="" cargo test --test reciprocity_view -- --nocapture
git commit -am "feat(services): reciprocity_view::aggregate (REA SQL aggregation, no federation)"
```

### Task T28: doorway dashboard service

**Files:**
- Create: `doorway/doorway-service/src/services/dashboard_topology.rs`

- [ ] **Step 1: Failing test**

```rust
// doorway/doorway-service/tests/dashboard_topology.rs
#[tokio::test]
async fn dashboard_topology_reports_connected_stewards() {
    let harness = DoorwayHarness::with_three_storage_stewards().await;
    let view = harness.dashboard_topology().await.unwrap();
    assert_eq!(view.storage_stewards.len(), 3);
}

#[tokio::test]
async fn dashboard_topology_reports_federation_peers() {
    let harness = DoorwayHarness::with_federated_doorway("parish.elohim.host", true).await;
    let view = harness.dashboard_topology().await.unwrap();
    assert!(view.federation_peers.iter().any(|f| f.doorway_hostname == "parish.elohim.host"));
}

#[tokio::test]
async fn dashboard_topology_includes_cache_hit_rate() {
    let harness = DoorwayHarness::with_seeded_cache_metrics(0.87).await;
    let view = harness.dashboard_topology().await.unwrap();
    assert!((view.projection_coverage.cache_hit_rate_24h - 0.87).abs() < 0.01);
}
```

- [ ] **Step 2: Implement**

Create `doorway/doorway-service/src/services/dashboard_topology.rs`:

```rust
//! ## Source of Truth
//!
//! Operational (Category C). Doorway-resident operational state. Doorway is the web2
//! projection per `project_three_layer_truth_model`; this view is local to the doorway.

use crate::services::{cache_metrics::CacheMetrics, federation::FederationService, route_registry::RouteRegistry};
use crate::views::dashboard::*;

pub struct DashboardTopologyService {
    cache_metrics: std::sync::Arc<CacheMetrics>,
    federation: std::sync::Arc<FederationService>,
    route_registry: std::sync::Arc<RouteRegistry>,
}

impl DashboardTopologyService {
    pub fn new(
        cache_metrics: std::sync::Arc<CacheMetrics>,
        federation: std::sync::Arc<FederationService>,
        route_registry: std::sync::Arc<RouteRegistry>,
    ) -> Self {
        Self { cache_metrics, federation, route_registry }
    }

    pub async fn build_view(&self) -> Result<DoorwayDashboardView, DashboardError> {
        let hostname = std::env::var("DOORWAY_HOSTNAME").unwrap_or_else(|_| "localhost".into());

        let storage_stewards = self.collect_storage_stewards().await?;
        let federation_peers = self.federation.list_peers().await?
            .into_iter()
            .map(|p| DashboardFederationPeer {
                doorway_hostname: p.hostname,
                online: p.online,
                direction: parse_direction(&p.direction),
                shared_cid_count: p.shared_cid_count,
            })
            .collect();

        let projection_coverage = ProjectionCoverage {
            projected_cid_count: self.cache_metrics.projected_cid_count(),
            known_cid_count: self.cache_metrics.known_cid_count(),
            cache_hit_rate_24h: self.cache_metrics.hit_rate_24h(),
            projection_lag_ms_avg: self.cache_metrics.lag_ms_avg(),
        };

        let public_surface = build_public_surface(&hostname).await?;

        Ok(DoorwayDashboardView {
            doorway_hostname: hostname,
            storage_stewards,
            federation_peers,
            projection_coverage,
            public_surface,
        })
    }

    async fn collect_storage_stewards(&self) -> Result<Vec<DashboardSteward>, DashboardError> {
        // Query existing /api/v1/federation/p2p-peers endpoint or its underlying service.
        Ok(vec![]) // Phase 4 stub; real impl pulls from peer_status projection in storage.
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DashboardError {
    #[error("federation: {0}")]
    Federation(String),
    #[error("metrics: {0}")]
    Metrics(String),
}

fn parse_direction(s: &str) -> FederationDirection {
    match s {
        "bidirectional" => FederationDirection::Bidirectional,
        "outbound_only" => FederationDirection::OutboundOnly,
        "inbound_only" => FederationDirection::InboundOnly,
        _ => FederationDirection::OutboundOnly,
    }
}

async fn build_public_surface(hostname: &str) -> Result<PublicSurfaceState, DashboardError> {
    // Phase 4 stub; real impl resolves DNS, checks TLS expiry, attempts public reachability.
    Ok(PublicSurfaceState {
        dns_resolves: true,
        dns_target: Some("127.0.0.1".into()),
        tls_valid: false,
        tls_expires_in_days: None,
        public_reachable: false,
    })
}
```

- [ ] **Step 3: Run, commit**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo test --test dashboard_topology -- --nocapture
git commit -am "feat(doorway): dashboard_topology service (operator view aggregator)"
```

---

## Phase 5 — HTTP Handlers and Manifest Registration

Each handler in this phase is a **thin projection layer** over the view services from Phase 4. No business logic in handlers; auth context resolution + service call + JSON serialization only. Routes are declared in `build_manifest()` per `project_doorway_manifest_driven_routes`.

### Task T29: Handler — GET /api/v1/blob/{hash}/distribution/details

**Files:**
- Create: `elohim/elohim-storage/src/api/distribution.rs`
- Modify: `elohim/elohim-storage/src/api/mod.rs`

- [ ] **Step 1: Failing test**

```rust
// tests/api_distribution.rs
#[tokio::test]
async fn distribution_details_visitor_returns_summary_subset() {
    let app = mk_test_app_with_seeded_blob("hash_pub", 12, ReachClass::Public).await;
    let res = app.get("/api/v1/blob/hash_pub/distribution/details").await;
    assert_eq!(res.status(), 200);
    let body: DistributionDetails = res.json().await.unwrap();
    assert_eq!(body.summary.replica_count, 12);
    assert!(body.summary.my_role.is_none());
    assert!(body.reciprocity_edges.is_none());
}

#[tokio::test]
async fn distribution_details_steward_includes_reciprocity() {
    let app = mk_test_app_authed("agent_M", &[("P1", "desktop")]).await;
    let res = app.get("/api/v1/blob/hash_x/distribution/details").await;
    assert_eq!(res.status(), 200);
    let body: DistributionDetails = res.json().await.unwrap();
    assert!(body.reciprocity_edges.is_some());
}
```

- [ ] **Step 2: Implement**

Create `elohim/elohim-storage/src/api/distribution.rs`:

```rust
//! ## Source of Truth
//!
//! Operational (Category C). Read-projection over notarized DHT state.
//! No business logic — auth resolution + service call + JSON.

use axum::{extract::{Path, State}, response::IntoResponse, Json};
use crate::services::distribution_view::{compose_distribution_details, DistributionContext};
use crate::auth::bindings_resolver::resolve_bindings;
use crate::http::AppState;
use crate::views::DistributionDetails;

pub async fn get_distribution_details(
    State(state): State<AppState>,
    Path(hash): Path<String>,
    auth: Option<AuthContext>,
) -> impl IntoResponse {
    let bindings = if let Some(ref a) = auth {
        resolve_bindings(&state.pool, &a.agent_cid).await.unwrap_or_default()
    } else {
        vec![]
    };

    let ctx = match (&auth, bindings.is_empty()) {
        (Some(a), false) => DistributionContext::Steward { agent_cid: &a.agent_cid, bindings: &bindings },
        _ => DistributionContext::Visitor,
    };

    match compose_distribution_details(&state.pool, &hash, ctx).await {
        Ok(details) => (axum::http::StatusCode::OK, Json(details)).into_response(),
        Err(e) => json_error(500, &format!("compose_failed: {}", e)),
    }
}

pub struct AuthContext { pub agent_cid: String }

fn json_error(status: u16, reason: &str) -> axum::response::Response {
    let body = serde_json::json!({ "reason": reason });
    (axum::http::StatusCode::from_u16(status).unwrap(), Json(body)).into_response()
}
```

- [ ] **Step 3: Run, commit**

```bash
RUSTFLAGS="" cargo test --test api_distribution -- --nocapture
git commit -am "feat(api): GET /api/v1/blob/{hash}/distribution/details handler"
```

### Task T30: Handler — GET /api/v1/cluster/me

**Files:**
- Create: `elohim/elohim-storage/src/api/cluster.rs`

- [ ] **Step 1: Failing test**

```rust
#[tokio::test]
async fn cluster_me_requires_auth() {
    let app = mk_test_app().await;
    let res = app.get_no_auth("/api/v1/cluster/me").await;
    assert_eq!(res.status(), 401);
    assert_eq!(res.json::<serde_json::Value>().await.unwrap()["reason"], "auth_required");
}

#[tokio::test]
async fn cluster_me_empty_bindings_returns_empty_state() {
    let app = mk_test_app_authed_no_bindings("agent_fresh").await;
    let res = app.get("/api/v1/cluster/me").await;
    assert_eq!(res.status(), 200);
    let body: MyClusterView = res.json().await.unwrap();
    assert!(body.devices.is_empty());
}

#[tokio::test]
async fn cluster_me_with_three_bindings_federates() {
    let app = ThreePeerHarness::start_with_bindings("agent_M",
        &[("desktop", true), ("node", true), ("mobile", true)]).await.app();
    let res = app.get("/api/v1/cluster/me").await;
    let body: MyClusterView = res.json().await.unwrap();
    assert_eq!(body.devices.len(), 3);
}
```

- [ ] **Step 2: Implement**

Create `elohim/elohim-storage/src/api/cluster.rs`:

```rust
//! ## Source of Truth
//!
//! Operational (Category C). Federated projection over DHT-notarized AgentPeerBindings.

use axum::{extract::State, response::IntoResponse, Json};
use crate::services::cluster_view::aggregate_cluster_view;
use crate::http::AppState;

pub async fn get_cluster_me(
    State(state): State<AppState>,
    auth: Option<crate::api::distribution::AuthContext>,
) -> impl IntoResponse {
    let auth = match auth {
        Some(a) => a,
        None => return json_error(401, "auth_required"),
    };

    match aggregate_cluster_view(&state.pool, &state.federator, &auth.agent_cid).await {
        Ok(view) => (axum::http::StatusCode::OK, Json(view)).into_response(),
        Err(e) => json_error(500, &format!("cluster_view_failed: {}", e)),
    }
}

fn json_error(status: u16, reason: &str) -> axum::response::Response {
    use axum::http::StatusCode;
    let body = serde_json::json!({ "reason": reason });
    (StatusCode::from_u16(status).unwrap(), Json(body)).into_response()
}
```

- [ ] **Step 3: Run, commit**

```bash
RUSTFLAGS="" cargo test --test api_cluster -- --nocapture
git commit -am "feat(api): GET /api/v1/cluster/me handler (federated)"
```

### Task T31: Handler — GET /api/v1/peer-topology/me

**Files:**
- Create: `elohim/elohim-storage/src/api/peer_topology.rs`

- [ ] **Step 1: Failing test**

```rust
#[tokio::test]
async fn peer_topology_me_returns_household_edges() {
    let app = ThreePeerHarness::start_with_neighbors("agent_M", &["adam", "pete", "frank"]).await.app();
    let res = app.get("/api/v1/peer-topology/me").await;
    let body: PeerTopologyView = res.json().await.unwrap();
    assert!(body.edges.iter().any(|e| e.household_id == "adam"));
    assert_eq!(body.reciprocation_count, 3);
}
```

- [ ] **Step 2: Implement**

Create `elohim/elohim-storage/src/api/peer_topology.rs`:

```rust
//! ## Source of Truth
//!
//! Operational (Category C). Federated projection.

use axum::{extract::State, response::IntoResponse, Json};
use crate::services::peer_topology_view::aggregate_peer_topology_view;
use crate::http::AppState;

pub async fn get_peer_topology_me(
    State(state): State<AppState>,
    auth: Option<crate::api::distribution::AuthContext>,
) -> impl IntoResponse {
    let auth = match auth {
        Some(a) => a,
        None => return json_error(401, "auth_required"),
    };
    match aggregate_peer_topology_view(&state.pool, &state.federator, &auth.agent_cid).await {
        Ok(view) => (axum::http::StatusCode::OK, Json(view)).into_response(),
        Err(e) => json_error(500, &format!("peer_topology_failed: {}", e)),
    }
}

fn json_error(status: u16, reason: &str) -> axum::response::Response {
    use axum::http::StatusCode;
    (StatusCode::from_u16(status).unwrap(), Json(serde_json::json!({ "reason": reason }))).into_response()
}
```

- [ ] **Step 3: Run, commit**

```bash
RUSTFLAGS="" cargo test --test api_peer_topology -- --nocapture
git commit -am "feat(api): GET /api/v1/peer-topology/me handler (federated)"
```

### Task T32: Handler — GET /api/v1/reciprocity/me

**Files:**
- Create: `elohim/elohim-storage/src/api/reciprocity.rs`

- [ ] **Step 1: Failing test**

```rust
#[tokio::test]
async fn reciprocity_me_returns_inflow_outflow() {
    let app = mk_test_app_with_rea_commitments("agent_M", &[
        ("agent_M", "agent_adam", 14_000_000_000, 11_200_000_000),
    ]).await;
    let res = app.get("/api/v1/reciprocity/me").await;
    let body: ReciprocityView = res.json().await.unwrap();
    assert_eq!(body.outflow.len(), 1);
}
```

- [ ] **Step 2: Implement**

Create `elohim/elohim-storage/src/api/reciprocity.rs`:

```rust
//! ## Source of Truth
//!
//! Operational (Category C). SQL aggregation over notarized REA Commitment +
//! EconomicEvent. DHT is authoritative.

use axum::{extract::State, response::IntoResponse, Json};
use crate::services::reciprocity_view::aggregate_reciprocity_view;
use crate::auth::bindings_resolver::resolve_bindings;
use crate::http::AppState;

pub async fn get_reciprocity_me(
    State(state): State<AppState>,
    auth: Option<crate::api::distribution::AuthContext>,
) -> impl IntoResponse {
    let auth = match auth {
        Some(a) => a,
        None => return json_error(401, "auth_required"),
    };

    let bindings = resolve_bindings(&state.pool, &auth.agent_cid).await.unwrap_or_default();
    match aggregate_reciprocity_view(&state.pool, &auth.agent_cid, &bindings).await {
        Ok(view) => (axum::http::StatusCode::OK, Json(view)).into_response(),
        Err(e) => json_error(500, &format!("reciprocity_failed: {}", e)),
    }
}

fn json_error(status: u16, reason: &str) -> axum::response::Response {
    use axum::http::StatusCode;
    (StatusCode::from_u16(status).unwrap(), Json(serde_json::json!({ "reason": reason }))).into_response()
}
```

- [ ] **Step 3: Run, commit**

```bash
RUSTFLAGS="" cargo test --test api_reciprocity -- --nocapture
git commit -am "feat(api): GET /api/v1/reciprocity/me handler"
```

### Task T33: Register handlers in build_manifest()

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs` (build_manifest function)

- [ ] **Step 1: Locate build_manifest**

```bash
grep -n "fn build_manifest\|pub fn build_manifest" elohim/elohim-storage/src/http.rs
```

- [ ] **Step 2: Add the four entries**

Edit `build_manifest()` to append:

```rust
pub fn build_manifest() -> DoorwayRoutes {
    let mut routes = vec![
        // ... existing routes
    ];
    routes.extend(vec![
        ManifestRoute {
            method: "GET".into(),
            path: "/api/v1/blob/:hash/distribution/details".into(),
            target: RouteTarget::StorageProxy { endpoint: "/api/v1/blob/:hash/distribution/details".into() },
            auth_required: false, // visitors can fetch
            cache_ttl_secs: Some(5),
            rate_limit_rpm: Some(60),
        },
        ManifestRoute {
            method: "GET".into(),
            path: "/api/v1/cluster/me".into(),
            target: RouteTarget::StorageProxy { endpoint: "/api/v1/cluster/me".into() },
            auth_required: true,
            cache_ttl_secs: Some(2),
            rate_limit_rpm: Some(30),
        },
        ManifestRoute {
            method: "GET".into(),
            path: "/api/v1/peer-topology/me".into(),
            target: RouteTarget::StorageProxy { endpoint: "/api/v1/peer-topology/me".into() },
            auth_required: true,
            cache_ttl_secs: Some(2),
            rate_limit_rpm: Some(30),
        },
        ManifestRoute {
            method: "GET".into(),
            path: "/api/v1/reciprocity/me".into(),
            target: RouteTarget::StorageProxy { endpoint: "/api/v1/reciprocity/me".into() },
            auth_required: true,
            cache_ttl_secs: Some(10),
            rate_limit_rpm: Some(20),
        },
    ]);
    DoorwayRoutes { routes }
}
```

Also wire the handlers into the axum Router:

```rust
let app = Router::new()
    // ... existing
    .route("/api/v1/blob/:hash/distribution/details", get(api::distribution::get_distribution_details))
    .route("/api/v1/cluster/me", get(api::cluster::get_cluster_me))
    .route("/api/v1/peer-topology/me", get(api::peer_topology::get_peer_topology_me))
    .route("/api/v1/reciprocity/me", get(api::reciprocity::get_reciprocity_me));
```

- [ ] **Step 3: Add a manifest contract test**

```rust
#[tokio::test]
async fn build_manifest_includes_topology_routes() {
    let routes = build_manifest().routes;
    let paths: Vec<&str> = routes.iter().map(|r| r.path.as_str()).collect();
    assert!(paths.contains(&"/api/v1/cluster/me"));
    assert!(paths.contains(&"/api/v1/peer-topology/me"));
    assert!(paths.contains(&"/api/v1/reciprocity/me"));
    assert!(paths.contains(&"/api/v1/blob/:hash/distribution/details"));
}
```

- [ ] **Step 4: Run, commit**

```bash
RUSTFLAGS="" cargo test http build_manifest_includes_topology_routes -- --nocapture
git commit -am "feat(http): register topology routes in build_manifest()"
```

### Task T34: Extend EPR head response with DistributionSummary

**Files:**
- Modify: `elohim/elohim-storage/src/epr_head.rs`
- Modify: existing EPR view-schema contract test

- [ ] **Step 1: Failing test**

```rust
// tests/epr_head_distribution.rs
#[tokio::test]
async fn epr_head_includes_distribution_summary() {
    let app = mk_test_app_with_seeded_epr("epr_abc", b"content").await;
    let res = app.get("/api/v1/epr/epr_abc/head").await;
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(body.get("distribution").is_some(), "EPR head must include distribution");
    let dist = body.get("distribution").unwrap();
    assert!(dist.get("replicaCount").is_some());
    assert!(dist.get("reachClass").is_some());
}
```

- [ ] **Step 2: Implement**

Edit `elohim/elohim-storage/src/epr_head.rs` — add `distribution: Option<DistributionSummary>` field to the `EprHead` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct EprHead {
    // ... existing fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distribution: Option<DistributionSummary>,
}
```

In the EPR head handler, hydrate the field:

```rust
pub async fn get_epr_head(
    State(state): State<AppState>,
    Path(epr_id): Path<String>,
    auth: Option<AuthContext>,
) -> impl IntoResponse {
    let head = load_epr_head(&state.pool, &epr_id).await?;
    let blob_hash = &head.content_hash;

    let bindings = if let Some(ref a) = auth {
        resolve_bindings(&state.pool, &a.agent_cid).await.unwrap_or_default()
    } else { vec![] };

    let ctx = match (&auth, bindings.is_empty()) {
        (Some(a), false) => DistributionContext::Steward { agent_cid: &a.agent_cid, bindings: &bindings },
        _ => DistributionContext::Visitor,
    };

    let distribution = compose_distribution_summary(&state.pool, blob_hash, ctx).await.ok();

    let head_with_dist = EprHead { distribution, ..head };
    (StatusCode::OK, Json(head_with_dist)).into_response()
}
```

Update the EPR head schema (`elohim/sdk/schemas/v1/views/epr-head.schema.json` if it exists, otherwise the relevant view schema) to include the optional `distribution` field referencing `distribution-summary.schema.json`.

- [ ] **Step 3: Run schema codegen drift check**

```bash
pnpm run schema:codegen:ts
git diff --stat elohim/sdk/storage-client-ts/src/generated/ 2>&1
```

Expected: drift on `EprHead.ts` only.

- [ ] **Step 4: Run, commit**

```bash
RUSTFLAGS="" cargo test --test epr_head_distribution -- --nocapture
git add elohim/elohim-storage/src/epr_head.rs \
        elohim/sdk/schemas/v1/views/ \
        elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(epr): hydrate DistributionSummary onto EPR head response"
```

### Task T35: Doorway operator dashboard route

**Files:**
- Create: `doorway/doorway-service/src/routes/admin/dashboard_topology.rs`
- Modify: `doorway/doorway-service/src/server/http.rs`

- [ ] **Step 1: Failing test**

```rust
// doorway/doorway-service/tests/admin_dashboard_topology.rs
#[tokio::test]
async fn admin_dashboard_topology_requires_operator_auth() {
    let app = DoorwayHarness::start().await;
    let res = app.get_no_auth("/admin/dashboard/topology").await;
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn admin_dashboard_topology_returns_view() {
    let app = DoorwayHarness::start().await;
    let res = app.get_with_admin_key("/admin/dashboard/topology").await;
    assert_eq!(res.status(), 200);
    let body: DoorwayDashboardView = res.json().await.unwrap();
    assert!(!body.doorway_hostname.is_empty());
}
```

- [ ] **Step 2: Implement**

Create `doorway/doorway-service/src/routes/admin/dashboard_topology.rs`:

```rust
//! ## Source of Truth
//!
//! Operational (Category C). Doorway-resident operator state.

use axum::{extract::State, response::IntoResponse, Json};
use crate::services::dashboard_topology::DashboardTopologyService;
use crate::server::AppState;

pub async fn handle_dashboard_topology(
    State(state): State<AppState>,
) -> impl IntoResponse {
    // operator-auth is enforced by the existing admin gate middleware on /admin/* routes
    match state.dashboard_topology.build_view().await {
        Ok(view) => (axum::http::StatusCode::OK, Json(view)).into_response(),
        Err(e) => {
            let body = serde_json::json!({ "reason": format!("dashboard_failed: {}", e) });
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
        }
    }
}
```

Edit `doorway/doorway-service/src/server/http.rs` — add the match arm:

```rust
match (method, path) {
    // ... existing
    ("GET", "/admin/dashboard/topology") => {
        admin::dashboard_topology::handle_dashboard_topology(State(state)).await.into_response()
    }
    // ...
}
```

- [ ] **Step 3: Run, commit**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo test --test admin_dashboard_topology -- --nocapture
git commit -am "feat(doorway): GET /admin/dashboard/topology handler"
```

---

## Phase 6 — TS Codegen End-to-End

### Task T36: Regenerate all TS bindings + verify drift

**Files:** auto-generated, no manual edits beyond config

- [ ] **Step 1: ts-rs export**

```bash
cd elohim/elohim-storage
RUSTFLAGS="" cargo test export_bindings
```

Expected: pass; writes TS files to `elohim/sdk/storage-client-ts/src/generated/`.

- [ ] **Step 2: Run schema:codegen:ts**

```bash
pnpm run schema:codegen:ts
```

- [ ] **Step 3: Diff check (no manual drift)**

```bash
git diff --stat elohim/sdk/storage-client-ts/src/generated/ doorway/sdk/doorway-client-ts/src/generated/
```

If non-trivial drift: investigate (likely a schema vs Rust struct mismatch). Fix the schema or Rust struct so codegen is idempotent.

- [ ] **Step 4: Build storage-client-ts**

```bash
cd elohim/sdk/storage-client-ts
pnpm run build
```

Expected: pass.

- [ ] **Step 5: Build doorway-client-ts**

```bash
cd doorway/sdk/doorway-client-ts
pnpm run build
```

Expected: pass.

- [ ] **Step 6: Schema validate**

```bash
cd /projects/elohim
pnpm run schema:validate
```

Expected: pass.

- [ ] **Step 7: Commit**

```bash
git add elohim/sdk/storage-client-ts/src/generated/ doorway/sdk/doorway-client-ts/src/generated/
git commit -m "chore(codegen): refresh TS bindings end-to-end (Phases 1–5)"
```

---

## Phase 7 — Angular Shared Atoms

Each atom is a presentation-only component (`ChangeDetection.OnPush`, no internal HTTP fetch, no persistent state). Inputs typed against the generated TS interfaces from Phase 6.

### Task T37: <distribution-badge> component

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/distribution-badge/distribution-badge.component.ts`
- Create: `app/elohim-app/src/app/elohim/components/distribution-badge/distribution-badge.component.html`
- Create: `app/elohim-app/src/app/elohim/components/distribution-badge/distribution-badge.component.scss`
- Create: `app/elohim-app/src/app/elohim/components/distribution-badge/distribution-badge.component.spec.ts`

- [ ] **Step 1: Failing spec**

```ts
// distribution-badge.component.spec.ts
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { DistributionBadgeComponent } from './distribution-badge.component';
import { DistributionSummary } from '@elohim/storage-client/generated/DistributionSummary';

describe('DistributionBadgeComponent', () => {
  let fixture: ComponentFixture<DistributionBadgeComponent>;
  let component: DistributionBadgeComponent;

  beforeEach(async () => {
    await TestBed.configureTestingModule({ imports: [DistributionBadgeComponent] }).compileComponents();
    fixture = TestBed.createComponent(DistributionBadgeComponent);
    component = fixture.componentInstance;
  });

  it('renders replica count badge', () => {
    component.summary = makeSummary({ replicaCount: 12, replicaTarget: 14, replicaHealth: 'healthy' });
    fixture.detectChanges();
    const el = fixture.nativeElement.querySelector('[data-testid="distribution-badge-replica-count"]');
    expect(el?.textContent).toContain('12');
  });

  it('shows critical class when health is critical', () => {
    component.summary = makeSummary({ replicaHealth: 'critical', replicaCount: 1, replicaTarget: 4 });
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-testid="distribution-badge"]')?.classList).toContain('critical');
  });

  it('renders private icon for private reach', () => {
    component.summary = makeSummary({ reachClass: 'private' });
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-testid="distribution-badge-reach"]')?.textContent).toMatch(/private/);
  });

  it('shows my-role star when authenticated steward and replica', () => {
    component.summary = makeSummary({ myRole: 'replica' });
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-testid="distribution-badge-my-role"]')).toBeTruthy();
  });

  it('hides my-role for visitor', () => {
    component.summary = makeSummary({ myRole: undefined });
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-testid="distribution-badge-my-role"]')).toBeFalsy();
  });
});

function makeSummary(overrides: Partial<DistributionSummary>): DistributionSummary {
  return {
    replicaCount: 12, replicaTarget: 14, replicaHealth: 'healthy',
    projectorCount: 2, reachClass: 'public',
    diversityHint: { kind: 'region_metro', value: ['us-central'] },
    thisFetchSource: 'projected_via_doorway', lastVerifiedSeconds: 60,
    ...overrides,
  } as DistributionSummary;
}
```

- [ ] **Step 2: Run, expect fail**

```bash
cd app/elohim-app
pnpm exec vitest run src/app/elohim/components/distribution-badge/distribution-badge.component.spec.ts
```

- [ ] **Step 3: Implement**

Create `distribution-badge.component.ts`:

```ts
import { ChangeDetectionStrategy, Component, Input, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { DistributionSummary } from '@elohim/storage-client/generated/DistributionSummary';
import { DistributionDetails } from '@elohim/storage-client/generated/DistributionDetails';
import { DistributionService } from '../../services/distribution.service';

@Component({
  selector: 'elohim-distribution-badge',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './distribution-badge.component.html',
  styleUrls: ['./distribution-badge.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DistributionBadgeComponent {
  @Input({ required: true }) summary!: DistributionSummary;
  @Input() blobHash?: string;

  readonly expanded = signal(false);
  readonly details = signal<DistributionDetails | null>(null);
  readonly loadingDetails = signal(false);

  constructor(private readonly distribution: DistributionService) {}

  async onTooltipExpand() {
    if (this.expanded()) return;
    this.expanded.set(true);
    if (!this.blobHash || this.details()) return;
    this.loadingDetails.set(true);
    try {
      const d = await this.distribution.getDetails(this.blobHash);
      this.details.set(d);
    } finally {
      this.loadingDetails.set(false);
    }
  }

  reachIcon(reach: DistributionSummary['reachClass']): string {
    return reach === 'private' ? '🔒 private'
         : reach === 'intimate' ? '🔒 peer-only'
         : reach === 'public' ? '🌐 public'
         : reach;
  }
}
```

Create `distribution-badge.component.html`:

```html
<div
  class="badge"
  [class.healthy]="summary.replicaHealth === 'healthy'"
  [class.at_risk]="summary.replicaHealth === 'at_risk'"
  [class.critical]="summary.replicaHealth === 'critical'"
  data-testid="distribution-badge"
  (mouseenter)="onTooltipExpand()"
>
  <span class="dots" data-testid="distribution-badge-dots">
    <ng-container *ngFor="let _ of [].constructor(Math.min(4, summary.replicaCount))">●</ng-container>
  </span>
  <span class="count" data-testid="distribution-badge-replica-count">{{ summary.replicaCount }}</span>

  <span class="reach" data-testid="distribution-badge-reach">{{ reachIcon(summary.reachClass) }}</span>

  <span *ngIf="summary.myRole" class="my-role" data-testid="distribution-badge-my-role">
    ★ I host this
  </span>

  <div class="tooltip" *ngIf="expanded()" data-testid="distribution-badge-tooltip">
    <!-- simple tier always visible -->
    <div class="tooltip-row">Reach: {{ summary.reachClass }}</div>
    <div class="tooltip-row">Replicas: {{ summary.replicaCount }} / {{ summary.replicaTarget }}</div>
    <div class="tooltip-row">Projectors: {{ summary.projectorCount }}</div>
    <div class="tooltip-row">Health: {{ summary.replicaHealth }}</div>
    <div class="tooltip-row">This fetch: {{ summary.thisFetchSource }}</div>

    <!-- detailed tier (lazy) -->
    <details *ngIf="blobHash" data-testid="distribution-badge-show-details">
      <summary>show details</summary>
      <ng-container *ngIf="loadingDetails()">loading…</ng-container>
      <ng-container *ngIf="details() as d">
        <div *ngFor="let p of d.replicaPeers" class="peer-row">
          {{ p.deviceArchetype }} · last seen {{ p.lastSeenSeconds }}s ago
        </div>
        <div *ngFor="let pj of d.projectorIdentities" class="projector-row">
          {{ pj.doorwayHostname }}
        </div>
      </ng-container>
    </details>
  </div>
</div>
```

Create `distribution-badge.component.scss`:

```scss
.badge {
  display: inline-flex; align-items: center; gap: .25rem;
  padding: .25rem .5rem; border-radius: 4px;
  font-size: .75rem; line-height: 1;
  background: var(--badge-bg, #f0f0f0);
  cursor: pointer; position: relative;

  &.healthy   { color: var(--health-healthy, #2a7); }
  &.at_risk   { color: var(--health-at-risk, #c80); }
  &.critical  { color: var(--health-critical, #c30); }

  .dots { letter-spacing: -1px; }
  .my-role { font-weight: 600; }

  .tooltip {
    position: absolute; top: 100%; left: 0; z-index: 10;
    margin-top: .25rem; padding: .5rem; background: white;
    box-shadow: 0 2px 8px rgba(0,0,0,0.1); border-radius: 4px;
    min-width: 240px; font-size: .85rem;
  }
  .tooltip-row { margin: .15rem 0; }
  .peer-row, .projector-row { font-family: monospace; font-size: .75rem; }
}
```

- [ ] **Step 4: Run, commit**

```bash
pnpm exec vitest run src/app/elohim/components/distribution-badge/
git add app/elohim-app/src/app/elohim/components/distribution-badge/
git commit -m "feat(elohim): <distribution-badge> shared atom (simple+lazy detail tiers)"
```

### Task T38: <device-tile> component

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/device-tile/device-tile.component.{ts,html,scss,spec.ts}`

- [ ] **Step 1: Failing spec**

```ts
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { DeviceTileComponent } from './device-tile.component';
import { DeviceSummary } from '@elohim/storage-client/generated/DeviceSummary';

describe('DeviceTileComponent', () => {
  let fixture: ComponentFixture<DeviceTileComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({ imports: [DeviceTileComponent] }).compileComponents();
    fixture = TestBed.createComponent(DeviceTileComponent);
  });

  it('renders archetype label as Home server for node', () => {
    fixture.componentInstance.device = mk({ archetype: 'node', displayName: 'matthew' });
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-testid="device-tile-archetype-label"]')?.textContent).toMatch(/home server/i);
  });

  it('renders Laptop label for desktop archetype', () => {
    fixture.componentInstance.device = mk({ archetype: 'desktop', displayName: 'jessica' });
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-testid="device-tile-archetype-label"]')?.textContent).toMatch(/laptop/i);
  });

  it('shows asleep when not online', () => {
    fixture.componentInstance.device = mk({ online: false, freshness: { state: 'offline', staleSinceMs: 240000 } });
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-testid="device-tile-status"]')?.textContent).toMatch(/asleep|offline/i);
  });
});

function mk(p: Partial<DeviceSummary>): DeviceSummary {
  return {
    peerId: 'P', archetype: 'node', online: true,
    freshness: { state: 'live' }, ...p,
  } as DeviceSummary;
}
```

- [ ] **Step 2: Implement**

```ts
// device-tile.component.ts
import { ChangeDetectionStrategy, Component, Input, computed, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { DeviceSummary } from '@elohim/storage-client/generated/DeviceSummary';

const ARCHETYPE_LABEL: Record<DeviceSummary['archetype'], string> = {
  node: 'Home server',
  desktop: 'Laptop',
  mobile: 'Phone',
  steward: 'Steward process',
};

@Component({
  selector: 'elohim-device-tile',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="tile" [class.online]="device.online" [class.offline]="!device.online" data-testid="device-tile">
      <span class="dot" data-testid="device-tile-dot"></span>
      <span class="label" data-testid="device-tile-archetype-label">
        {{ archetypeLabel(device.archetype) }} ({{ device.displayName }})
      </span>
      <span class="hosting" *ngIf="device.hostingCount != null">
        {{ device.hostingCount }} files
      </span>
      <span class="status" data-testid="device-tile-status">
        <ng-container *ngIf="device.online">online</ng-container>
        <ng-container *ngIf="!device.online">
          asleep · {{ staleAgo(device.freshness.staleSinceMs) }}
        </ng-container>
      </span>
    </div>
  `,
  styleUrls: ['./device-tile.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DeviceTileComponent {
  @Input({ required: true }) device!: DeviceSummary;

  archetypeLabel(a: DeviceSummary['archetype']) { return ARCHETYPE_LABEL[a]; }

  staleAgo(ms?: number): string {
    if (ms == null) return '';
    const sec = Math.floor(ms / 1000);
    if (sec < 60) return `${sec}s ago`;
    const min = Math.floor(sec / 60);
    if (min < 60) return `${min} min ago`;
    return `${Math.floor(min / 60)} h ago`;
  }
}
```

```scss
/* device-tile.component.scss */
.tile {
  display: flex; align-items: center; gap: .5rem;
  padding: .25rem 0;
  &.offline { opacity: .55; }
  .dot { width: .5rem; height: .5rem; border-radius: 50%; background: var(--device-online, #2a7); }
  .offline .dot { background: var(--device-offline, #888); }
}
```

- [ ] **Step 3: Run, commit**

```bash
pnpm exec vitest run src/app/elohim/components/device-tile/
git commit -am "feat(elohim): <device-tile> archetype-aware atom"
```

### Task T39: <peer-household-card> component

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/peer-household-card/peer-household-card.component.{ts,html,scss,spec.ts}`

- [ ] **Step 1: Failing spec**

```ts
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { PeerHouseholdCardComponent } from './peer-household-card.component';
import { PeerHouseholdEdge } from '@elohim/storage-client/generated/PeerHouseholdEdge';

describe('PeerHouseholdCardComponent', () => {
  let fixture: ComponentFixture<PeerHouseholdCardComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({ imports: [PeerHouseholdCardComponent] }).compileComponents();
    fixture = TestBed.createComponent(PeerHouseholdCardComponent);
  });

  it('shows reciprocity diff', () => {
    fixture.componentInstance.edge = mk({ myCidsHostedByThem: 7, theirCidsHostedByMe: 12, netDiff: 5 });
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-testid="peer-card-net-diff"]')?.textContent).toContain('+5');
  });

  it('shows critical-for-me warning', () => {
    fixture.componentInstance.edge = mk({ isCriticalForMe: true });
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-testid="peer-card-critical-for-me"]')).toBeTruthy();
  });
});

function mk(p: Partial<PeerHouseholdEdge>): PeerHouseholdEdge {
  return {
    householdId: 'adam', online: true,
    myCidsHostedByThem: 0, theirCidsHostedByMe: 0, netDiff: 0,
    ...p,
  } as PeerHouseholdEdge;
}
```

- [ ] **Step 2: Implement**

```ts
import { ChangeDetectionStrategy, Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { PeerHouseholdEdge } from '@elohim/storage-client/generated/PeerHouseholdEdge';

@Component({
  selector: 'elohim-peer-household-card',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="peer-card" data-testid="peer-household-card" [class.dark]="!edge.online">
      <header>
        <strong>{{ edge.displayName ?? edge.householdId }}</strong>
        <span data-testid="peer-card-status">{{ edge.online ? 'reachable' : 'dark' }}</span>
      </header>
      <div class="reciprocity">
        hosts {{ edge.myCidsHostedByThem }} of mine ·
        I host {{ edge.theirCidsHostedByMe }} of theirs ·
        <span data-testid="peer-card-net-diff" [class.positive]="edge.netDiff > 0" [class.negative]="edge.netDiff < 0">
          net {{ edge.netDiff > 0 ? '+' : '' }}{{ edge.netDiff }}
        </span>
      </div>
      <div *ngIf="edge.isCriticalForMe" class="critical" data-testid="peer-card-critical-for-me">
        ⚠ critical-for-me (sole external replica of {{ /* calculated upstream */ '' }} CIDs)
      </div>
      <div *ngIf="edge.iAmCriticalForThem" class="critical" data-testid="peer-card-critical-for-them">
        I am critical-for-them
      </div>
    </div>
  `,
  styles: [`
    .peer-card { padding: .5rem 0; border-bottom: 1px solid #eee; }
    .peer-card.dark { opacity: .55; }
    .reciprocity .positive { color: var(--diff-positive, #2a7); }
    .reciprocity .negative { color: var(--diff-negative, #c30); }
    .critical { color: var(--health-critical, #c30); font-size: .85rem; }
  `],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PeerHouseholdCardComponent {
  @Input({ required: true }) edge!: PeerHouseholdEdge;
}
```

- [ ] **Step 3: Run, commit**

```bash
pnpm exec vitest run src/app/elohim/components/peer-household-card/
git commit -am "feat(elohim): <peer-household-card> atom"
```

### Task T40: <commitment-bar> component

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/commitment-bar/commitment-bar.component.{ts,html,scss,spec.ts}`

- [ ] **Step 1: Failing spec**

```ts
import { TestBed, ComponentFixture } from '@angular/core/testing';
import { CommitmentBarComponent } from './commitment-bar.component';

describe('CommitmentBarComponent', () => {
  let fixture: ComponentFixture<CommitmentBarComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({ imports: [CommitmentBarComponent] }).compileComponents();
    fixture = TestBed.createComponent(CommitmentBarComponent);
  });

  it('renders honored percent', () => {
    fixture.componentInstance.committedBytes = 14_000_000_000;
    fixture.componentInstance.deliveredBytes = 11_200_000_000;
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-testid="commitment-bar-percent"]')?.textContent).toMatch(/80/);
  });

  it('flags over-delivered', () => {
    fixture.componentInstance.committedBytes = 3_000_000_000;
    fixture.componentInstance.deliveredBytes = 3_100_000_000;
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-testid="commitment-bar-over-delivered"]')).toBeTruthy();
  });
});
```

- [ ] **Step 2: Implement**

```ts
import { ChangeDetectionStrategy, Component, Input, computed, signal } from '@angular/core';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'elohim-commitment-bar',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="cbar" data-testid="commitment-bar">
      <div class="bar">
        <div class="fill" [style.width.%]="Math.min(100, percent())"></div>
      </div>
      <span data-testid="commitment-bar-percent">{{ percent() | number:'1.0-0' }}%</span>
      <span *ngIf="percent() > 100" data-testid="commitment-bar-over-delivered">★ over-delivered</span>
    </div>
  `,
  styles: [`
    .cbar { display: flex; align-items: center; gap: .5rem; }
    .bar { width: 100px; height: 6px; background: #eee; border-radius: 3px; overflow: hidden; }
    .fill { height: 100%; background: var(--commitment-fill, #2a7); }
  `],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CommitmentBarComponent {
  @Input() committedBytes = 0;
  @Input() deliveredBytes = 0;

  readonly percent = computed(() => {
    if (this.committedBytes === 0) return 0;
    return (this.deliveredBytes / this.committedBytes) * 100;
  });

  protected readonly Math = Math;
}
```

- [ ] **Step 3: Run, commit**

```bash
pnpm exec vitest run src/app/elohim/components/commitment-bar/
git commit -am "feat(elohim): <commitment-bar> atom"
```

### Task T41: <diversity-hint> component (reach-driven render)

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/diversity-hint/diversity-hint.component.{ts,spec.ts}`

- [ ] **Step 1: Failing spec**

```ts
import { TestBed, ComponentFixture } from '@angular/core/testing';
import { DiversityHintComponent } from './diversity-hint.component';

describe('DiversityHintComponent', () => {
  let fixture: ComponentFixture<DiversityHintComponent>;
  beforeEach(async () => {
    await TestBed.configureTestingModule({ imports: [DiversityHintComponent] }).compileComponents();
    fixture = TestBed.createComponent(DiversityHintComponent);
  });

  it('renders region metros for region_metro kind', () => {
    fixture.componentInstance.hint = { kind: 'region_metro', value: ['us-central','eu-west'] };
    fixture.detectChanges();
    expect(fixture.nativeElement.textContent).toMatch(/us-central/);
    expect(fixture.nativeElement.textContent).toMatch(/eu-west/);
  });

  it('renders household archetypes', () => {
    fixture.componentInstance.hint = { kind: 'household_archetypes', value: ['desktop','node'] };
    fixture.detectChanges();
    expect(fixture.nativeElement.textContent).toMatch(/Laptop/);
    expect(fixture.nativeElement.textContent).toMatch(/Home server/);
  });

  it('renders member count for collective', () => {
    fixture.componentInstance.hint = { kind: 'collective_member_count', value: 8 };
    fixture.detectChanges();
    expect(fixture.nativeElement.textContent).toMatch(/8/);
  });

  it('renders nothing for none', () => {
    fixture.componentInstance.hint = { kind: 'none', value: null };
    fixture.detectChanges();
    expect(fixture.nativeElement.textContent.trim()).toBe('');
  });
});
```

- [ ] **Step 2: Implement**

```ts
import { ChangeDetectionStrategy, Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { DiversityHint } from '@elohim/storage-client/generated/DiversityHint';

const ARCHETYPE_HUMAN: Record<string, string> = {
  desktop: 'Laptop', node: 'Home server', mobile: 'Phone', steward: 'Steward',
};

@Component({
  selector: 'elohim-diversity-hint',
  standalone: true,
  imports: [CommonModule],
  template: `
    <ng-container [ngSwitch]="hint.kind">
      <span *ngSwitchCase="'region_metro'" data-testid="diversity-hint-region">
        {{ asArray(hint.value).length }} {{ asArray(hint.value).length === 1 ? 'region' : 'regions' }}
        ({{ asArray(hint.value).join(' · ') }})
      </span>
      <span *ngSwitchCase="'household_archetypes'" data-testid="diversity-hint-households">
        {{ asArray(hint.value).map(a => ARCHETYPE_HUMAN[a] ?? a).join(' · ') }}
      </span>
      <span *ngSwitchCase="'collective_member_count'" data-testid="diversity-hint-collective">
        hosted by {{ hint.value }} collective members
      </span>
      <span *ngSwitchCase="'none'"></span>
    </ng-container>
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DiversityHintComponent {
  @Input({ required: true }) hint!: DiversityHint;
  protected readonly ARCHETYPE_HUMAN = ARCHETYPE_HUMAN;
  asArray(v: unknown): string[] { return Array.isArray(v) ? (v as string[]) : []; }
}
```

- [ ] **Step 3: Run, commit**

```bash
pnpm exec vitest run src/app/elohim/components/diversity-hint/
git commit -am "feat(elohim): <diversity-hint> reach-driven render atom"
```

---

## Phase 8 — Angular Services and Page Components

### Task T42: distribution.service.ts (lazy detail fetch)

**Files:**
- Create: `app/elohim-app/src/app/elohim/services/distribution.service.{ts,spec.ts}`

- [ ] **Step 1: Failing spec**

```ts
import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting, HttpTestingController } from '@angular/common/http/testing';
import { DistributionService } from './distribution.service';

describe('DistributionService', () => {
  let service: DistributionService;
  let http: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [provideHttpClient(), provideHttpClientTesting(), DistributionService] });
    service = TestBed.inject(DistributionService);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  it('fetches details from /api/v1/blob/:hash/distribution/details', async () => {
    const promise = service.getDetails('hash_xyz');
    const req = http.expectOne('/api/v1/blob/hash_xyz/distribution/details');
    expect(req.request.method).toBe('GET');
    req.flush({ summary: { replicaCount: 5 } });
    const result = await promise;
    expect(result.summary.replicaCount).toBe(5);
  });
});
```

- [ ] **Step 2: Implement**

```ts
import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import { DistributionDetails } from '@elohim/storage-client/generated/DistributionDetails';

@Injectable({ providedIn: 'root' })
export class DistributionService {
  private readonly http = inject(HttpClient);

  async getDetails(blobHash: string): Promise<DistributionDetails> {
    return firstValueFrom(
      this.http.get<DistributionDetails>(`/api/v1/blob/${blobHash}/distribution/details`)
    );
  }
}
```

- [ ] **Step 3: Run, commit**

```bash
pnpm exec vitest run src/app/elohim/services/distribution.service.spec.ts
git commit -am "feat(elohim): DistributionService lazy detail fetch"
```

### Task T43: cluster.service.ts + peer-topology.service.ts + reciprocity.service.ts

**Files:**
- Create: `app/elohim-app/src/app/shefa/services/{cluster,peer-topology,reciprocity}.service.{ts,spec.ts}`

- [ ] **Step 1: Failing spec (cluster)**

```ts
import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting, HttpTestingController } from '@angular/common/http/testing';
import { ClusterService } from './cluster.service';

describe('ClusterService', () => {
  let service: ClusterService; let http: HttpTestingController;
  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [provideHttpClient(), provideHttpClientTesting(), ClusterService] });
    service = TestBed.inject(ClusterService); http = TestBed.inject(HttpTestingController);
  });
  afterEach(() => http.verify());

  it('fetches cluster view', async () => {
    const promise = service.getMyCluster();
    const req = http.expectOne('/api/v1/cluster/me');
    req.flush({ agentCid: 'agent_M', devices: [], totals: {}, freshness: { state: 'live' } });
    const view = await promise;
    expect(view.agentCid).toBe('agent_M');
  });

  it('emits via signal poll on focus', () => {
    const sig = service.cluster();
    expect(sig).toBeNull(); // initial
  });
});
```

- [ ] **Step 2: Implement (cluster)**

```ts
// cluster.service.ts
import { Injectable, inject, signal } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import { MyClusterView } from '@elohim/storage-client/generated/MyClusterView';

@Injectable({ providedIn: 'root' })
export class ClusterService {
  private readonly http = inject(HttpClient);
  readonly cluster = signal<MyClusterView | null>(null);
  readonly loading = signal(false);
  readonly error = signal<string | null>(null);

  async getMyCluster(): Promise<MyClusterView> {
    this.loading.set(true);
    this.error.set(null);
    try {
      const view = await firstValueFrom(this.http.get<MyClusterView>('/api/v1/cluster/me'));
      this.cluster.set(view);
      return view;
    } catch (e: any) {
      this.error.set(e.message ?? 'unknown');
      throw e;
    } finally {
      this.loading.set(false);
    }
  }

  startPolling(intervalMs = 5000) {
    const id = setInterval(() => { void this.getMyCluster(); }, intervalMs);
    return () => clearInterval(id);
  }
}
```

Mirror this exact pattern for `peer-topology.service.ts` (URL `/api/v1/peer-topology/me`, type `PeerTopologyView`) and `reciprocity.service.ts` (URL `/api/v1/reciprocity/me`, type `ReciprocityView`, longer poll interval like 30s since DHT-projected).

```ts
// peer-topology.service.ts
import { Injectable, inject, signal } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import { PeerTopologyView } from '@elohim/storage-client/generated/PeerTopologyView';

@Injectable({ providedIn: 'root' })
export class PeerTopologyService {
  private readonly http = inject(HttpClient);
  readonly topology = signal<PeerTopologyView | null>(null);
  readonly loading = signal(false);

  async getMyPeerTopology(): Promise<PeerTopologyView> {
    this.loading.set(true);
    try {
      const view = await firstValueFrom(this.http.get<PeerTopologyView>('/api/v1/peer-topology/me'));
      this.topology.set(view);
      return view;
    } finally { this.loading.set(false); }
  }
  startPolling(intervalMs = 5000) { const id = setInterval(() => void this.getMyPeerTopology(), intervalMs); return () => clearInterval(id); }
}
```

```ts
// reciprocity.service.ts
import { Injectable, inject, signal } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import { ReciprocityView } from '@elohim/storage-client/generated/ReciprocityView';

@Injectable({ providedIn: 'root' })
export class ReciprocityService {
  private readonly http = inject(HttpClient);
  readonly reciprocity = signal<ReciprocityView | null>(null);
  readonly loading = signal(false);

  async getMyReciprocity(): Promise<ReciprocityView> {
    this.loading.set(true);
    try {
      const view = await firstValueFrom(this.http.get<ReciprocityView>('/api/v1/reciprocity/me'));
      this.reciprocity.set(view);
      return view;
    } finally { this.loading.set(false); }
  }
  startPolling(intervalMs = 30_000) { const id = setInterval(() => void this.getMyReciprocity(), intervalMs); return () => clearInterval(id); }
}
```

- [ ] **Step 3: Run all three specs, commit**

```bash
pnpm exec vitest run src/app/shefa/services/
git add app/elohim-app/src/app/shefa/services/
git commit -m "feat(shefa): cluster/peer-topology/reciprocity services with polling signals"
```

### Task T44: <my-cluster> page component

**Files:**
- Create: `app/elohim-app/src/app/shefa/pages/my-cluster/my-cluster.component.{ts,html,scss,spec.ts}`

- [ ] **Step 1: Failing spec**

```ts
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideHttpClientTesting, HttpTestingController } from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';
import { MyClusterComponent } from './my-cluster.component';
import { ClusterService } from '../../services/cluster.service';

describe('MyClusterComponent', () => {
  let fixture: ComponentFixture<MyClusterComponent>;
  let http: HttpTestingController;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [MyClusterComponent],
      providers: [provideHttpClient(), provideHttpClientTesting(), ClusterService],
    }).compileComponents();
    fixture = TestBed.createComponent(MyClusterComponent);
    http = TestBed.inject(HttpTestingController);
  });

  it('renders devices on load', () => {
    fixture.detectChanges();
    const req = http.expectOne('/api/v1/cluster/me');
    req.flush({
      agentCid: 'agent_M',
      devices: [
        { peerId: 'P1', archetype: 'desktop', online: true, freshness: { state: 'live' }, displayName: 'matthew', hostingCount: 1247 },
        { peerId: 'P2', archetype: 'node', online: true, freshness: { state: 'live' }, displayName: 'home', hostingCount: 800 },
      ],
      totals: { storageUsedBytes: 25_000_000_000, storageTotalBytes: 298_000_000_000, externalCommittedBytes: 0, reciprocityNetBytes: 0 },
      freshness: { state: 'live' },
    });
    fixture.detectChanges();
    const tiles = fixture.nativeElement.querySelectorAll('[data-testid="device-tile"]');
    expect(tiles.length).toBe(2);
  });

  it('shows offline device with stale_since', () => {
    fixture.detectChanges();
    http.expectOne('/api/v1/cluster/me').flush({
      agentCid: 'agent_M',
      devices: [{ peerId: 'P1', archetype: 'mobile', online: false, freshness: { state: 'offline', staleSinceMs: 240000 }, displayName: 'phone' }],
      totals: { storageUsedBytes: 0, storageTotalBytes: 0, externalCommittedBytes: 0, reciprocityNetBytes: 0 },
      freshness: { state: 'live' },
    });
    fixture.detectChanges();
    const tile = fixture.nativeElement.querySelector('[data-testid="device-tile"]');
    expect(tile.classList).toContain('offline');
  });
});
```

- [ ] **Step 2: Implement**

```ts
// my-cluster.component.ts
import { ChangeDetectionStrategy, Component, OnDestroy, OnInit, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ClusterService } from '../../services/cluster.service';
import { DeviceTileComponent } from '../../../elohim/components/device-tile/device-tile.component';

@Component({
  selector: 'shefa-my-cluster',
  standalone: true,
  imports: [CommonModule, DeviceTileComponent],
  templateUrl: './my-cluster.component.html',
  styleUrls: ['./my-cluster.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class MyClusterComponent implements OnInit, OnDestroy {
  protected readonly cluster = inject(ClusterService);
  protected readonly showDetails = signal(false);
  private stopPolling?: () => void;

  ngOnInit() {
    void this.cluster.getMyCluster();
    this.stopPolling = this.cluster.startPolling(5000);
  }
  ngOnDestroy() { this.stopPolling?.(); }
  toggleDetails() { this.showDetails.update(v => !v); }
}
```

```html
<!-- my-cluster.component.html -->
<section class="my-cluster" data-testid="my-cluster-page">
  <header>
    <h2>Your devices</h2>
    <span *ngIf="cluster.loading()">…</span>
  </header>

  <ng-container *ngIf="cluster.cluster() as v">
    <div class="summary">
      {{ v.devices.length }} devices · {{ onlineCount(v.devices) }} online · {{ offlineCount(v.devices) }} sleeping
    </div>

    <div class="device-list">
      <elohim-device-tile *ngFor="let d of v.devices" [device]="d"></elohim-device-tile>
    </div>

    <dl class="totals">
      <dt>Storage</dt><dd>{{ formatBytes(v.totals.storageUsedBytes) }} of {{ formatBytes(v.totals.storageTotalBytes) }} used</dd>
      <dt>Hosting</dt><dd>{{ formatBytes(v.totals.externalCommittedBytes) }} for friends · they host {{ formatBytes(reciprocityInBytes(v.totals.reciprocityNetBytes)) }} of mine</dd>
      <dt>Status</dt><dd data-testid="my-cluster-status">{{ statusText(v) }}</dd>
    </dl>

    <button (click)="toggleDetails()" data-testid="my-cluster-show-details">
      [{{ showDetails() ? 'hide' : 'show' }} details]
    </button>

    <pre *ngIf="showDetails()" data-testid="my-cluster-detail-json">{{ v | json }}</pre>
  </ng-container>
</section>
```

(Helper methods like `onlineCount`, `offlineCount`, `formatBytes`, `statusText` go in the component class as pure helpers.)

- [ ] **Step 3: Run, commit**

```bash
pnpm exec vitest run src/app/shefa/pages/my-cluster/
git commit -am "feat(shefa): <my-cluster> page (simple tier + show-details JSON)"
```

### Task T45: <peer-topology> page component

**Files:**
- Create: `app/elohim-app/src/app/shefa/pages/peer-topology/peer-topology.component.{ts,html,scss,spec.ts}`

- [ ] **Step 1: Failing spec**

```ts
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting, HttpTestingController } from '@angular/common/http/testing';
import { PeerTopologyComponent } from './peer-topology.component';

describe('PeerTopologyComponent', () => {
  let fixture: ComponentFixture<PeerTopologyComponent>; let http: HttpTestingController;
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [PeerTopologyComponent],
      providers: [provideHttpClient(), provideHttpClientTesting()],
    }).compileComponents();
    fixture = TestBed.createComponent(PeerTopologyComponent);
    http = TestBed.inject(HttpTestingController);
  });

  it('renders peer-household cards', () => {
    fixture.detectChanges();
    http.expectOne('/api/v1/peer-topology/me').flush({
      agentCid: 'agent_M', edges: [
        { householdId: 'adam', online: true, myCidsHostedByThem: 7, theirCidsHostedByMe: 12, netDiff: 5 },
        { householdId: 'pete', online: true, myCidsHostedByThem: 4, theirCidsHostedByMe: 8, netDiff: 4 },
      ],
      reciprocationCount: 2, resilienceCliffs: [],
      freshness: { state: 'live' },
    });
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelectorAll('[data-testid="peer-household-card"]').length).toBe(2);
  });
});
```

- [ ] **Step 2: Implement**

```ts
import { ChangeDetectionStrategy, Component, OnInit, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { PeerTopologyService } from '../../services/peer-topology.service';
import { PeerHouseholdCardComponent } from '../../../elohim/components/peer-household-card/peer-household-card.component';

@Component({
  selector: 'shefa-peer-topology',
  standalone: true,
  imports: [CommonModule, PeerHouseholdCardComponent],
  template: `
    <section class="peer-topology" data-testid="peer-topology-page">
      <header><h2>Your peer households</h2></header>
      <ng-container *ngIf="topology.topology() as v">
        <div class="summary">
          {{ v.edges.length }} peer households · {{ v.reciprocationCount }} reciprocating
        </div>
        <elohim-peer-household-card
          *ngFor="let edge of v.edges"
          [edge]="edge"></elohim-peer-household-card>
        <div *ngIf="v.resilienceCliffs.length > 0" class="cliff" data-testid="peer-topology-cliff">
          ⚠ resilience cliff: {{ v.resilienceCliffs.length }} household(s) hold sole-replica content
        </div>
        <button (click)="showDetails.update(x => !x)" data-testid="peer-topology-show-details">
          [{{ showDetails() ? 'hide' : 'show' }} details]
        </button>
        <pre *ngIf="showDetails()">{{ v | json }}</pre>
      </ng-container>
    </section>
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PeerTopologyComponent implements OnInit {
  protected readonly topology = inject(PeerTopologyService);
  protected readonly showDetails = signal(false);
  ngOnInit() { void this.topology.getMyPeerTopology(); this.topology.startPolling(5000); }
}
```

- [ ] **Step 3: Run, commit**

```bash
pnpm exec vitest run src/app/shefa/pages/peer-topology/
git commit -am "feat(shefa): <peer-topology> page (household cards + cliff warning)"
```

### Task T46: <reciprocity-ledger> page component

**Files:**
- Create: `app/elohim-app/src/app/shefa/pages/reciprocity-ledger/reciprocity-ledger.component.{ts,html,scss,spec.ts}`

- [ ] **Step 1: Failing spec**

```ts
// abbreviated; pattern matches T45's spec
it('renders inflow + outflow rows', () => { /* flush ReciprocityView with both arrays; assert rows */ });
it('renders capacityAvailableBytes', () => { /* flush; assert capacity displayed */ });
```

- [ ] **Step 2: Implement**

```ts
import { ChangeDetectionStrategy, Component, OnInit, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ReciprocityService } from '../../services/reciprocity.service';
import { CommitmentBarComponent } from '../../../elohim/components/commitment-bar/commitment-bar.component';

@Component({
  selector: 'shefa-reciprocity-ledger',
  standalone: true,
  imports: [CommonModule, CommitmentBarComponent],
  template: `
    <section class="reciprocity" data-testid="reciprocity-page">
      <ng-container *ngIf="recip.reciprocity() as v">
        <h2>Reciprocity</h2>

        <h3>Inflow (others committed to host my content)</h3>
        <ul>
          <li *ngFor="let row of v.inflow">
            <strong>{{ row.displayName ?? row.counterpartyHouseholdId }}</strong> —
            {{ formatBytes(row.committedBytes) }} committed,
            {{ formatBytes(row.deliveredBytes) }} delivered
            <elohim-commitment-bar [committedBytes]="row.committedBytes" [deliveredBytes]="row.deliveredBytes"></elohim-commitment-bar>
          </li>
        </ul>

        <h3>Outflow (I committed to host others' content)</h3>
        <ul>
          <li *ngFor="let row of v.outflow">
            <strong>{{ row.displayName ?? row.counterpartyHouseholdId }}</strong> —
            {{ formatBytes(row.committedBytes) }} committed,
            {{ formatBytes(row.deliveredBytes) }} hosting
            <elohim-commitment-bar [committedBytes]="row.committedBytes" [deliveredBytes]="row.deliveredBytes"></elohim-commitment-bar>
          </li>
        </ul>

        <dl class="totals">
          <dt>Net</dt>
          <dd data-testid="reciprocity-net">
            {{ formatBytes(v.netHostedBytes) }} {{ v.netHostedBytes >= 0 ? 'hosted on my behalf' : 'I host more than I am hosted' }}
          </dd>
          <dt>Capacity</dt><dd data-testid="reciprocity-capacity">{{ formatBytes(v.capacityAvailableBytes) }} free</dd>
        </dl>
      </ng-container>
    </section>
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ReciprocityLedgerComponent implements OnInit {
  protected readonly recip = inject(ReciprocityService);
  ngOnInit() { void this.recip.getMyReciprocity(); this.recip.startPolling(30_000); }
  formatBytes(n: number): string { /* GB/MB human format */ return `${(n / 1e9).toFixed(1)} GB`; }
}
```

- [ ] **Step 3: Run, commit**

```bash
pnpm exec vitest run src/app/shefa/pages/reciprocity-ledger/
git commit -am "feat(shefa): <reciprocity-ledger> page (inflow/outflow + capacity)"
```

### Task T47: Doorway-app operator topology pane

**Files:**
- Create: `doorway/doorway-app/src/app/admin/topology/operator-topology.component.{ts,spec.ts}`
- Modify: `doorway/doorway-app/src/app/admin/admin-routing.module.ts` (or equivalent)

- [ ] **Step 1: Failing spec**

```ts
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting, HttpTestingController } from '@angular/common/http/testing';
import { OperatorTopologyComponent } from './operator-topology.component';

describe('OperatorTopologyComponent', () => {
  let fixture: ComponentFixture<OperatorTopologyComponent>; let http: HttpTestingController;
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [OperatorTopologyComponent],
      providers: [provideHttpClient(), provideHttpClientTesting()],
    }).compileComponents();
    fixture = TestBed.createComponent(OperatorTopologyComponent);
    http = TestBed.inject(HttpTestingController);
  });

  it('renders dashboard view', () => {
    fixture.detectChanges();
    http.expectOne('/admin/dashboard/topology').flush({
      doorwayHostname: 'matthew.elohim.host',
      storageStewards: [], federationPeers: [],
      projectionCoverage: { projectedCidCount: 4318, knownCidCount: 5672, cacheHitRate24h: 0.87, projectionLagMsAvg: 340 },
      publicSurface: { dnsResolves: true, tlsValid: true, publicReachable: true },
    });
    fixture.detectChanges();
    expect(fixture.nativeElement.textContent).toContain('matthew.elohim.host');
    expect(fixture.nativeElement.textContent).toMatch(/87/);
  });
});
```

- [ ] **Step 2: Implement**

```ts
import { ChangeDetectionStrategy, Component, OnInit, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import { DoorwayDashboardView } from '@elohim/doorway-client/generated/DoorwayDashboardView';

@Component({
  selector: 'doorway-operator-topology',
  standalone: true,
  imports: [CommonModule],
  template: `
    <section class="op-topology" data-testid="operator-topology">
      <ng-container *ngIf="view() as v">
        <h2>{{ v.doorwayHostname }}</h2>
        <div>{{ v.storageStewards.length }} storage stewards · {{ v.federationPeers.length }} federation peers</div>
        <h3>Projection coverage</h3>
        <div data-testid="projection-coverage">
          {{ v.projectionCoverage.projectedCidCount }} / {{ v.projectionCoverage.knownCidCount }} CIDs ({{ (v.projectionCoverage.cacheHitRate24h * 100) | number:'1.0-0' }}% hit rate)
        </div>
        <pre *ngIf="showDetails()" data-testid="operator-topology-json">{{ v | json }}</pre>
        <button (click)="showDetails.update(x => !x)">[{{ showDetails() ? 'hide' : 'show' }} details]</button>
      </ng-container>
    </section>
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class OperatorTopologyComponent implements OnInit {
  private readonly http = inject(HttpClient);
  protected readonly view = signal<DoorwayDashboardView | null>(null);
  protected readonly showDetails = signal(false);

  async ngOnInit() {
    const v = await firstValueFrom(this.http.get<DoorwayDashboardView>('/admin/dashboard/topology'));
    this.view.set(v);
  }
}
```

Add the route in doorway-app's admin routing.

- [ ] **Step 3: Run, commit**

```bash
cd doorway/doorway-app
pnpm exec vitest run src/app/admin/topology/
git commit -am "feat(doorway-app): operator topology pane"
```

### Task T48: Verify all Phase 8 components compile + lint

**Files:** none

- [ ] **Step 1: Lint**

```bash
cd app/elohim-app
pnpm run lint
```

- [ ] **Step 2: All component specs**

```bash
pnpm exec vitest run src/app/shefa/ src/app/elohim/components/
```

- [ ] **Step 3: Format check**

```bash
pnpm run format:check
```

Expected: all clean.

---

## Phase 9 — App Integration

### Task T49: ContentNode adapter surfaces distribution

**Files:**
- Modify: `app/elohim-app/src/app/elohim/adapters/content-node.adapter.ts`
- Modify: corresponding spec

- [ ] **Step 1: Failing spec**

Edit the adapter spec:

```ts
it('surfaces distribution from EPR envelope', () => {
  const epr = {
    contentHash: 'h1', contentType: 'concept', title: 't',
    distribution: {
      replicaCount: 5, replicaTarget: 6, replicaHealth: 'healthy',
      projectorCount: 1, reachClass: 'public',
      diversityHint: { kind: 'region_metro', value: ['us-central'] },
      thisFetchSource: 'projected_via_doorway', lastVerifiedSeconds: 30,
    },
  };
  const node = adapter.adapt(epr as any);
  expect(node.distribution).toBeDefined();
  expect(node.distribution!.replicaCount).toBe(5);
});
```

- [ ] **Step 2: Implement**

Edit `content-node.adapter.ts` to add the optional `distribution` field on `ContentNode`:

```ts
import { DistributionSummary } from '@elohim/storage-client/generated/DistributionSummary';

export interface ContentNode {
  // ... existing
  distribution?: DistributionSummary;
}

export class ContentNodeAdapter {
  adapt(envelope: EprEnvelope): ContentNode {
    return {
      // ... existing field mappings
      distribution: envelope.distribution, // EPR head wire shape includes this when present
    };
  }
}
```

Also update the model file `content-node.model.ts` per the model-sync rule:

```ts
export interface ContentNode {
  id: string;
  contentType: string;
  // ... existing fields
  distribution?: DistributionSummary;
}
```

- [ ] **Step 3: Run, commit**

```bash
pnpm exec vitest run src/app/elohim/adapters/
git commit -am "feat(adapter): ContentNode surfaces distribution from EPR envelope"
```

### Task T50: Embed <distribution-badge> in content-card

**Files:**
- Modify: `app/elohim-app/src/app/lamad/components/content-card/content-card.component.{html,ts,spec.ts}`

- [ ] **Step 1: Failing spec**

```ts
it('renders <distribution-badge> when content.distribution present', () => {
  component.content = mkContent({ distribution: { replicaCount: 5 } as any });
  fixture.detectChanges();
  expect(fixture.nativeElement.querySelector('[data-testid="distribution-badge"]')).toBeTruthy();
});

it('hides badge when distribution is absent', () => {
  component.content = mkContent({ distribution: undefined });
  fixture.detectChanges();
  expect(fixture.nativeElement.querySelector('[data-testid="distribution-badge"]')).toBeFalsy();
});
```

- [ ] **Step 2: Implement**

Edit `content-card.component.ts` imports:

```ts
import { DistributionBadgeComponent } from '../../../elohim/components/distribution-badge/distribution-badge.component';

@Component({
  selector: 'lamad-content-card',
  standalone: true,
  imports: [/* existing */, DistributionBadgeComponent],
  // ...
})
```

Edit `content-card.component.html` to add:

```html
<elohim-distribution-badge
  *ngIf="content.distribution"
  [summary]="content.distribution"
  [blobHash]="content.contentHash"
></elohim-distribution-badge>
```

- [ ] **Step 3: Run, commit**

```bash
pnpm exec vitest run src/app/lamad/components/content-card/
git commit -am "feat(lamad): embed <distribution-badge> in content-card"
```

### Task T51: Routing — /cluster, /peers, /reciprocity

**Files:**
- Modify: `app/elohim-app/src/app/app.routes.ts` (or equivalent)

- [ ] **Step 1: Add routes**

```ts
import { Routes } from '@angular/router';
import { MyClusterComponent } from './shefa/pages/my-cluster/my-cluster.component';
import { PeerTopologyComponent } from './shefa/pages/peer-topology/peer-topology.component';
import { ReciprocityLedgerComponent } from './shefa/pages/reciprocity-ledger/reciprocity-ledger.component';

export const routes: Routes = [
  // ... existing
  { path: 'cluster', component: MyClusterComponent, title: 'Your devices' },
  { path: 'peers', component: PeerTopologyComponent, title: 'Your peer households' },
  { path: 'reciprocity', component: ReciprocityLedgerComponent, title: 'Reciprocity' },
];
```

Add nav entries (location depends on existing layout; likely `app.component.html` or a sidebar component).

- [ ] **Step 2: Smoke test**

```bash
cd app/elohim-app
pnpm start &
sleep 10
curl -s http://localhost:4200/cluster | grep -i "your devices"
```

Expected: HTML contains the expected title.

- [ ] **Step 3: Commit**

```bash
git commit -am "feat(routing): /cluster, /peers, /reciprocity routes wired"
```

### Task T52: Verify schema sync and lint clean

```bash
pnpm run schema:validate
pnpm run schema:codegen:ts
git diff --stat   # should be empty if codegen idempotent

cd app/elohim-app && pnpm run lint && pnpm run format:check
cd doorway/doorway-app && pnpm run lint
```

Fix any drift and commit:

```bash
git commit -am "chore: schema + lint cleanup post-Phase-9"
```







---

## Phase 10: Integration tests (T53-T56)

End-to-end verification on Jenkins. These prove the demo claims of "P2P hosting that doesn't go down" and "bytes physically arrive when peers come back online." They are gated to Jenkins (Eclipse Che cannot run multi-peer Holochain swarms).

### Task T53: Jenkins Test A — multi-device cluster federation

**Goal:** Two device-archetype peers (laptop + node) sharing one steward identity → `/api/v1/cluster` aggregates both rows with Live freshness.

**Files:**
- Create: `genesis/jenkins-tests/topology/cluster_federation_test.sh`
- Modify: `genesis/orchestrator/Jenkinsfile` (register the new test)

- [ ] **Step 1: Write the failing test script**

```bash
#!/usr/bin/env bash
# genesis/jenkins-tests/topology/cluster_federation_test.sh
set -euo pipefail

# Spin up two peers under a single steward identity.
# Peer A: archetype=desktop. Peer B: archetype=node.
# Both publish AgentPeerBinding for the same agent_cid.
# Assert /api/v1/cluster aggregates both with Live freshness.

PEER_A_PORT=8090
PEER_B_PORT=8091
STEWARD_AGENT="agentstr_t53_steward"

# Start peer A (desktop archetype)
elohim-storage --data-dir /tmp/peer-a --port "$PEER_A_PORT" --archetype desktop --steward "$STEWARD_AGENT" &
PEER_A_PID=$!

# Start peer B (node archetype)
elohim-storage --data-dir /tmp/peer-b --port "$PEER_B_PORT" --archetype node --steward "$STEWARD_AGENT" &
PEER_B_PID=$!

trap 'kill $PEER_A_PID $PEER_B_PID 2>/dev/null || true' EXIT

# Wait for both peers to be ready
until curl -sf "http://localhost:$PEER_A_PORT/healthz" >/dev/null; do sleep 1; done
until curl -sf "http://localhost:$PEER_B_PORT/healthz" >/dev/null; do sleep 1; done

# Wait for AgentPeerBinding gossip (DHT propagation)
sleep 10

# Query cluster view from peer A
RESPONSE=$(curl -sf "http://localhost:$PEER_A_PORT/api/v1/cluster")
echo "$RESPONSE" | jq .

# Assertions
TOTAL_DEVICES=$(echo "$RESPONSE" | jq '.totals.devices')
LIVE_COUNT=$(echo "$RESPONSE" | jq '[.devices[] | select(.freshness.state == "Live")] | length')
ARCHETYPES=$(echo "$RESPONSE" | jq -r '.devices[].archetype' | sort | tr '\n' ',')

if [ "$TOTAL_DEVICES" -lt 2 ]; then
  echo "FAIL: expected >=2 devices, got $TOTAL_DEVICES"
  exit 1
fi

if [ "$LIVE_COUNT" -lt 2 ]; then
  echo "FAIL: expected >=2 Live devices, got $LIVE_COUNT"
  exit 1
fi

if [[ "$ARCHETYPES" != *"desktop"* || "$ARCHETYPES" != *"node"* ]]; then
  echo "FAIL: expected desktop+node archetypes, got $ARCHETYPES"
  exit 1
fi

echo "PASS: cluster federation aggregates desktop+node, both Live"
```

- [ ] **Step 2: Make executable**

```bash
chmod +x genesis/jenkins-tests/topology/cluster_federation_test.sh
```

- [ ] **Step 3: Register in Jenkinsfile**

In `genesis/orchestrator/Jenkinsfile`, add a stage that triggers `topology-cluster-federation-test` when changesets touch any of:
- `elohim/elohim-storage/**`
- `elohim/sdk/schemas/v1/views/**`
- `genesis/jenkins-tests/topology/**`

```groovy
stage('Topology — Cluster Federation') {
  when {
    expression { matchesGlob(env.CHANGED_FILES, 'elohim/elohim-storage/**,elohim/sdk/schemas/v1/views/**,genesis/jenkins-tests/topology/**') }
  }
  steps {
    sh 'genesis/jenkins-tests/topology/cluster_federation_test.sh'
  }
}
```

- [ ] **Step 4: Run on Jenkins (manual trigger first time)**

Use the Jenkins MCP `triggerBuild` tool from the orchestrator pipeline with parameter `MODE=test-topology`.

Expected: stage passes, output shows 2 Live devices with desktop+node archetypes.

- [ ] **Step 5: Commit**

```bash
git add genesis/jenkins-tests/topology/cluster_federation_test.sh genesis/orchestrator/Jenkinsfile
git commit -m "test(topology): T53 — Jenkins integration test for multi-device cluster federation"
```

### Task T54: Jenkins Test B — resilient delivery (peer offline → page still loads)

**Goal:** Three-peer swarm holding the same blob. Take one peer offline → blob still served by remaining peers → distribution badge shows `replicaCount: 2`. Bring peer back → kick happens → `replicaCount: 3` again.

**Files:**
- Create: `genesis/jenkins-tests/topology/resilient_delivery_test.sh`
- Modify: `genesis/orchestrator/Jenkinsfile`

- [ ] **Step 1: Write the failing test script**

```bash
#!/usr/bin/env bash
# genesis/jenkins-tests/topology/resilient_delivery_test.sh
set -euo pipefail

PORTS=(8090 8091 8092)
PIDS=()

for i in 0 1 2; do
  elohim-storage --data-dir "/tmp/peer-$i" --port "${PORTS[$i]}" &
  PIDS+=($!)
done
trap 'kill ${PIDS[@]} 2>/dev/null || true' EXIT

# Wait until all healthy
for port in "${PORTS[@]}"; do
  until curl -sf "http://localhost:$port/healthz" >/dev/null; do sleep 1; done
done

# Publish a blob via peer 0; let it propagate
BLOB_HASH=$(curl -sf -X POST -H "Content-Type: application/octet-stream" \
  --data-binary @genesis/jenkins-tests/topology/fixtures/sample.html \
  "http://localhost:${PORTS[0]}/api/v1/blob" | jq -r .hash)

echo "Published blob: $BLOB_HASH"
sleep 15  # allow gossip + replication

# Confirm baseline: 3 replicas
COUNT_BEFORE=$(curl -sf "http://localhost:${PORTS[0]}/api/v1/distribution/$BLOB_HASH" | jq .replicaCount)
if [ "$COUNT_BEFORE" -lt 3 ]; then
  echo "FAIL: expected 3 replicas, got $COUNT_BEFORE"
  exit 1
fi

# Kill peer 2
kill "${PIDS[2]}" || true
sleep 5

# Page still loads via peer 0 even though 2 is gone
curl -sf "http://localhost:${PORTS[0]}/api/v1/blob/$BLOB_HASH" -o /tmp/served.html
diff /tmp/served.html genesis/jenkins-tests/topology/fixtures/sample.html

# Distribution badge degrades to 2
COUNT_DEGRADED=$(curl -sf "http://localhost:${PORTS[0]}/api/v1/distribution/$BLOB_HASH" | jq .replicaCount)
if [ "$COUNT_DEGRADED" -ne 2 ]; then
  echo "FAIL: expected 2 replicas after peer offline, got $COUNT_DEGRADED"
  exit 1
fi

# Bring peer 2 back
elohim-storage --data-dir /tmp/peer-2 --port "${PORTS[2]}" &
PIDS[2]=$!
until curl -sf "http://localhost:${PORTS[2]}/healthz" >/dev/null; do sleep 1; done

# Wait for on-connect kick + transfer
sleep 30

# Confirm peer 2 has the blob (filesystem evidence, not just inventory)
HASH_DIR=${BLOB_HASH:0:2}
if [ ! -f "/tmp/peer-2/blobs/$HASH_DIR/$BLOB_HASH" ]; then
  echo "FAIL: peer 2 did not actually receive blob bytes"
  exit 1
fi

# Distribution badge returns to 3
COUNT_AFTER=$(curl -sf "http://localhost:${PORTS[0]}/api/v1/distribution/$BLOB_HASH" | jq .replicaCount)
if [ "$COUNT_AFTER" -ne 3 ]; then
  echo "FAIL: expected 3 replicas after peer reconnect, got $COUNT_AFTER"
  exit 1
fi

echo "PASS: resilient delivery 3 -> 2 -> 3 with bytes physically arriving"
```

- [ ] **Step 2: Add fixture**

```bash
mkdir -p genesis/jenkins-tests/topology/fixtures
cat > genesis/jenkins-tests/topology/fixtures/sample.html <<'HTML'
<!DOCTYPE html>
<html><head><title>Resilient Delivery Test</title></head>
<body><h1>This page does not go down.</h1></body></html>
HTML
chmod +x genesis/jenkins-tests/topology/resilient_delivery_test.sh
```

- [ ] **Step 3: Register in Jenkinsfile**

```groovy
stage('Topology — Resilient Delivery') {
  when {
    expression { matchesGlob(env.CHANGED_FILES, 'elohim/elohim-storage/**,steward/node/**,genesis/jenkins-tests/topology/**') }
  }
  steps {
    sh 'genesis/jenkins-tests/topology/resilient_delivery_test.sh'
  }
}
```

- [ ] **Step 4: Trigger and verify on Jenkins**

Expected: stage passes with PASS log line.

- [ ] **Step 5: Commit**

```bash
git add genesis/jenkins-tests/topology/resilient_delivery_test.sh \
        genesis/jenkins-tests/topology/fixtures/sample.html \
        genesis/orchestrator/Jenkinsfile
git commit -m "test(topology): T54 — Jenkins integration test for resilient delivery"
```

### Task T55: Jenkins Test C — cold-peer-to-first-byte latency budget

**Goal:** Bring a fresh peer online with no replicas and measure time from connect to first byte served via peer-fallback. Sets a regression budget.

**Files:**
- Create: `genesis/jenkins-tests/topology/cold_peer_first_byte_test.sh`
- Modify: `genesis/orchestrator/Jenkinsfile`

- [ ] **Step 1: Write the test script**

```bash
#!/usr/bin/env bash
# genesis/jenkins-tests/topology/cold_peer_first_byte_test.sh
set -euo pipefail

# Peer A holds the blob. Peer B is fresh (cold).
PEER_A_PORT=8090
PEER_B_PORT=8091
BUDGET_MS=10000   # 10 seconds budget for cold peer first byte

elohim-storage --data-dir /tmp/peer-a --port $PEER_A_PORT &
PID_A=$!
trap 'kill $PID_A ${PID_B:-0} 2>/dev/null || true' EXIT

until curl -sf "http://localhost:$PEER_A_PORT/healthz" >/dev/null; do sleep 1; done

BLOB_HASH=$(curl -sf -X POST -H "Content-Type: application/octet-stream" \
  --data-binary @genesis/jenkins-tests/topology/fixtures/sample.html \
  "http://localhost:$PEER_A_PORT/api/v1/blob" | jq -r .hash)

# Now bring up peer B fresh (no blobs)
rm -rf /tmp/peer-b
elohim-storage --data-dir /tmp/peer-b --port $PEER_B_PORT &
PID_B=$!
until curl -sf "http://localhost:$PEER_B_PORT/healthz" >/dev/null; do sleep 1; done

# Time the GET request — peer B must fall back to peer A
START_MS=$(date +%s%3N)
HTTP_CODE=$(curl -s -o /dev/null -w '%{http_code}' "http://localhost:$PEER_B_PORT/api/v1/blob/$BLOB_HASH")
END_MS=$(date +%s%3N)

ELAPSED_MS=$((END_MS - START_MS))
echo "Cold peer first byte: ${ELAPSED_MS}ms (budget ${BUDGET_MS}ms)"

if [ "$HTTP_CODE" != "200" ]; then
  echo "FAIL: expected 200, got $HTTP_CODE"
  exit 1
fi

if [ "$ELAPSED_MS" -gt "$BUDGET_MS" ]; then
  echo "FAIL: cold peer first byte ${ELAPSED_MS}ms exceeded budget ${BUDGET_MS}ms"
  exit 1
fi

echo "PASS: cold peer served first byte in ${ELAPSED_MS}ms (under ${BUDGET_MS}ms budget)"
```

- [ ] **Step 2: Make executable**

```bash
chmod +x genesis/jenkins-tests/topology/cold_peer_first_byte_test.sh
```

- [ ] **Step 3: Register in Jenkinsfile**

```groovy
stage('Topology — Cold-Peer First Byte') {
  when {
    expression { matchesGlob(env.CHANGED_FILES, 'elohim/elohim-storage/**,steward/node/**,genesis/jenkins-tests/topology/**') }
  }
  steps {
    sh 'genesis/jenkins-tests/topology/cold_peer_first_byte_test.sh'
  }
}
```

- [ ] **Step 4: Trigger and verify**

Expected: PASS with elapsed time printed in log. Budget is intentionally generous (10s) for the first regression baseline; later sprints can tighten.

- [ ] **Step 5: Commit**

```bash
git add genesis/jenkins-tests/topology/cold_peer_first_byte_test.sh genesis/orchestrator/Jenkinsfile
git commit -m "test(topology): T55 — Jenkins regression for cold-peer first-byte latency"
```

### Task T56: A2O scenarios — topology surfaces

**Goal:** Capture the human experience of the five views in Gherkin scenarios so future regressions surface as failing scenarios, not silent UX drift.

**Files:**
- Create: `genesis/a2o/features/topology/distribution_badge.feature`
- Create: `genesis/a2o/features/topology/my_cluster.feature`
- Create: `genesis/a2o/features/topology/peer_topology.feature`
- Create: `genesis/a2o/features/topology/reciprocity.feature`
- Create: `genesis/a2o/features/topology/doorway_dashboard.feature`

- [ ] **Step 1: distribution_badge.feature**

```gherkin
Feature: Distribution badge surfaces P2P resilience

  As a content viewer
  I want to see how a piece of content is distributed across peers
  So that I trust the network is hosting it durably

  Scenario: Healthy public content shows replica count and projector identity
    Given a published blob with hash "bafk-sample"
    And the blob is replicated on 3 peers
    When I open the content card
    Then the distribution badge shows "3 replicas"
    And the projector identity is visible (peer-direct or via doorway)
    And clicking the badge expands to show per-replica details lazily

  Scenario: Resilient delivery — content stays available when peer goes offline
    Given a published blob replicated on 3 peers
    When peer 3 goes offline
    Then the page still loads via remaining peers
    And the distribution badge degrades to "2 replicas" with stale annotation for peer 3
    And no error banner is shown to the viewer

  Scenario: Reciprocity returns when peer reconnects
    Given a peer that was offline returns
    Then within 30 seconds the badge shows "3 replicas" again
    And the byte transfer is visible in the dev-tier details
```

- [ ] **Step 2: my_cluster.feature**

```gherkin
Feature: My cluster topology — devices stewarding my identity

  Scenario: Steward sees all their devices regardless of archetype
    Given I am signed in as a steward
    And I have a desktop, a node, and a mobile device under my agent
    When I navigate to /cluster
    Then I see three device tiles labeled by archetype, not by k8s role
    And each tile shows its freshness (Live, Stale, Offline)
    And the page does not surface k8s implementation language

  Scenario: Offline device is visible but marked Offline
    Given my node device has not been seen in 1 hour
    When I view /cluster
    Then the node tile is marked "Offline"
    And the totals exclude its compute commitment from "available now"
```

- [ ] **Step 3: peer_topology.feature**

```gherkin
Feature: Peer topology — social replication health

  Scenario: Peer households are aggregated, not individual peers
    Given my content is replicated by peers from 4 different households
    When I view /peers
    Then I see 4 household rows, not N peer rows
    And each row shows the reciprocity count (mine for them, theirs for me)

  Scenario: Resilience cliff is highlighted
    Given all my replicas are in 2 households
    When I view /peers
    Then a "resilience cliff" callout appears
    And the suggested action is "diversify to 3+ households"
```

- [ ] **Step 4: reciprocity.feature**

```gherkin
Feature: Reciprocity ledger — my compute vs my commitments

  Scenario: Net-positive steward sees their margin
    Given I have committed 100GB of storage
    And I am hosting 80GB for peers
    When I view /reciprocity
    Then I see "20GB commitment headroom"
    And the per-peer rows show what I host for them and what they host for me

  Scenario: Net-negative steward sees the over-commit warning
    Given I have committed 100GB
    And peers are hosting 120GB for me
    When I view /reciprocity
    Then a "over-commit" warning appears
    And the suggested action is "increase commitment or shed reach"
```

- [ ] **Step 5: doorway_dashboard.feature**

```gherkin
Feature: Doorway operator dashboard — projection topology

  Scenario: Doorway operator sees CDN-style geographic distribution
    Given I operate doorway "doorway.example.org"
    And my doorway is projecting content for 50 distinct agents
    When I view the operator dashboard
    Then I see geographic regions (us-west, eu-central) like a CDN map
    And the projection cache hit ratio is visible
    And the visitor view shows "geographic doorway distribution" labels, not "household"

  Scenario: Visitor never sees household-private labels
    Given I am an unauthenticated visitor
    When I view a public content distribution badge
    Then I see geographic-doorway distribution labels
    And household and steward identity is hidden
```

- [ ] **Step 6: Validate Gherkin syntax**

```bash
cd genesis
pnpm run a2o:lint   # if available, otherwise:
find a2o/features/topology -name "*.feature" -exec head -1 {} \;
```

Expected: All five files start with `Feature:` and parse cleanly.

- [ ] **Step 7: Commit**

```bash
git add genesis/a2o/features/topology/
git commit -m "test(a2o): T56 — Gherkin scenarios for topology surfaces"
```

---

## Phase 11: Story-harvest + final CI gates (T57)

### Task T57: Story-harvest constraints + final pre-merge gate sweep

**Goal:** Capture parameter-bearing constraints discovered during implementation (timeouts, payload caps, kick caps, freshness thresholds) into a regression artifact and run the full pre-push gate to ensure CI is green before opening PR.

**Files:**
- Create: `genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-constraints.md`
- Verify: full pre-push hook runs clean

- [ ] **Step 1: Write the constraints document**

```markdown
# Light Up the Topology — Discovered Constraints

> Captured per story-harvest skill at end of sprint. These are parameter-bearing constants that emerged during implementation. They are now load-bearing — change with care and update the corresponding regression test.

## Timeouts

| Constant | Value | Where | Why |
|---|---|---|---|
| `FEDERATION_TIMEOUT_MS` | 3000 | `view-federation/protocol.rs` | Best-effort federation budget per peer. Beyond this, the slice is annotated `Offline` and the request returns. Tested in T54. |
| `PEER_FALLBACK_TIMEOUT_MS` | 8000 | `blob_handler.rs` GET-time fallback | Cold peer must serve first byte under this; budget for T55 is 10s including connect+swarm overhead. |
| `KICK_DEBOUNCE_MS` | 2000 | `on_connect_kick.rs` | Avoid stampede when a peer flaps connection state. |

## Payload caps

| Constant | Value | Where | Why |
|---|---|---|---|
| `MAX_VIEW_PAYLOAD` | 256 KB | `view-federation/codec.rs` | Bound msgpack payload from a federated peer. Larger views get truncated with `more=true` flag. |
| `MAX_DETAILS_REPLICAS` | 100 | `distribution_view.rs` | Detail payload shows up to 100 per-replica rows; further truncated with summary count. |

## Kick caps

| Constant | Value | Where | Why |
|---|---|---|---|
| `KICK_GLOBAL_CAP` | 16 | `on_connect_kick.rs` | Max concurrent on-connect kicks per peer to prevent thundering herd. |
| `KICK_PER_BLOB_CAP` | 2 | `on_connect_kick.rs` | Max concurrent kicks for the same blob hash. |

## Freshness thresholds

| State | Threshold | Where |
|---|---|---|
| Live | last seen <30s | `freshness.rs` |
| Stale | <5 min | `freshness.rs` |
| Offline | <1 hr | `freshness.rs` |
| AllOffline | total federation timeout AND no cached projection | `view_federator.rs` |

## Operator presets these constraints inform

- **Single-peer dev**: federation_timeout=500ms (no peers to query)
- **Household cluster**: federation_timeout=3000ms (current default)
- **Public doorway**: federation_timeout=5000ms with cache fallback
- **Mobile**: peer_fallback off by default (handle offline-first via cached projection)

## Peer diversity configuration these constraints inform

- Resilience cliff threshold: <3 households is the recommendation surfaced in `peer_topology_view`
- Diversity hint by reach: see `diversity_hint.rs` mapping (public→region, intimate→household)

These constants are referenced by tests T53/T54/T55 and a2o scenarios in `genesis/a2o/features/topology/`. Bumping them requires updating those tests in the same commit.
```

- [ ] **Step 2: Save and commit**

```bash
git add genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-constraints.md
git commit -m "docs(topology): T57 — discovered constraints captured per story-harvest"
```

- [ ] **Step 3: Final pre-push sweep**

```bash
# From repo root
pnpm install
pnpm run schema:validate
pnpm run schema:test
pnpm run schema:check-dna
pnpm run schema:codegen:ts

# Rust
RUSTFLAGS="" cargo --workspace --manifest-path doorway/doorway-service/Cargo.toml clippy -- -D warnings
RUSTFLAGS="" cargo --workspace --manifest-path doorway/doorway-service/Cargo.toml fmt --check
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo --workspace --manifest-path elohim/elohim-storage/Cargo.toml clippy -- -D warnings
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo --workspace --manifest-path elohim/elohim-storage/Cargo.toml fmt --check
RUSTFLAGS="" cargo --workspace --manifest-path steward/node/Cargo.toml clippy -- -D warnings
RUSTFLAGS="" cargo --workspace --manifest-path steward/node/Cargo.toml fmt --check

# Angular
cd app/elohim-app && pnpm run lint && pnpm run format:check && pnpm test -- --run
cd ../../doorway/doorway-app && pnpm run lint
```

Expected: all gates green.

- [ ] **Step 4: Push the branch**

```bash
git push -u origin feature/light-up-topology
```

- [ ] **Step 5: Open the PR via gh**

```bash
gh pr create --base dev --title "feat(topology): light up the topology — five P2P surfaces, view-federation, resilient delivery" --body "$(cat <<'EOF'
## Summary
- Adds five topology views (distribution badge, my cluster, peer topology, reciprocity, doorway dashboard) backed by a new `view-federation/1.0.0` libp2p protocol with cryptographic gating via DHT-notarized `AgentPeerBinding`s.
- Two-tier badge: inline `DistributionSummary` (~100 bytes) embedded in EPR head, lazy `DistributionDetails` (~1KB) for expansion.
- Substrate fixes: GET-time peer-fallback, on-connect kick, parity regression — proves "P2P hosting that doesn't go down."
- Multi-device humans first-class — device archetypes (node/desktop/mobile/steward) drive UX, k8s topology stays an implementation detail.

## Test plan
- [ ] `pnpm run schema:validate` clean
- [ ] All Rust workspaces clippy + fmt clean
- [ ] All Angular projects lint + format clean
- [ ] Unit tests pass (Vitest + cargo test)
- [ ] Jenkins Topology stages green: cluster federation, resilient delivery, cold-peer first-byte
- [ ] A2O scenarios in `genesis/a2o/features/topology/` parse cleanly
- [ ] Manual: `/cluster`, `/peers`, `/reciprocity`, doorway dashboard render

## Related
- Spec: `genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-design.md`
- Plan: `genesis/docs/superpowers/plans/2026-05-01-light-up-the-topology-plan.md`
- Constraints: `genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-constraints.md`
EOF
)"
```

Expected: PR opened, GitHub URL printed.
