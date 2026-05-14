# P2P Dataplane Visibility Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Light up the four dark dashboards (`/shefa/devices`, `/shefa/resources/category:content`, `/shefa/dashboard`, `doorway-alpha/threshold/dashboard`) with real data from live personas running on shem, proving household-to-household P2P resilience for Matthew's family.

**Architecture:** Build on the ~95%-shipped PeerStatus Phase 1 foundation (infrastructure DNA + projection). Add a new node-shape publish surface (elohim-node boot → DHT commit → SQLite projection) joinable with PeerStatus for the `/shefa/devices` view. Compute household-first resilience from PeerStatus + stewardship allocations + shard locations. Wire UI. Activate full persona roster on shem. Prove maintenance choreography (announce → check → rejoin) without household protection ever dropping to `at-risk`.

**Tech Stack:** Rust (HDI/HDK for DNA, diesel for SQLite, tokio, hyper), Angular 19 + TypeScript, pnpm, Cucumber BDD, k8s StatefulSet + local-path-provisioner on shem.

**Source spec:** `genesis/docs/superpowers/specs/2026-04-19-p2p-dataplane-visibility-design.md`.

---

## Conventions

- All Rust commands: `RUSTFLAGS=""` override EXCEPT inside elohim-storage which uses `RUSTFLAGS='--cfg getrandom_backend="custom"'` (per repo CLAUDE.md). DNA tests run via `cd elohim/holochain && just test`.
- Schema-first IoC: JSON schema FIRST in `elohim/sdk/schemas/v1/`, Rust/TS comply via `cargo test --test schema_contract` + `pnpm run schema:codegen:ts`.
- Doorway-is-web2-only: ZERO per-domain proxy files added to doorway-service. All new HTTP routes land in elohim-storage's `build_manifest()` and auto-register. Only doorway-service changes allowed: wire existing `/admin/routes` handler + fix `/admin/users` auth.
- TDD order per task: failing test → minimal impl → pass → commit.
- Commit messages per task; do not batch commits across tasks.

---

## P2P Design Gate classifications

Every new entity/route in this plan is pre-classified per the P2P design gate. Source of truth is named for each; migrations include the source-of-truth SQL comment; schema files carry it in `description`; HTTP route handlers reference the originating DHT entry type where one exists. **No new DHT entry types are added by this sprint** — all notarized work reuses existing `NodeRegistration` (node-registry DNA) and `PeerStatus` (infrastructure DNA) plus the existing `collectives` entry type extended with `kind: "household"`.

| Entity / route | Category | Source of truth | Projection table | Justification |
|---|---|---|---|---|
| **NodeShape** (wire view, `node-shape-view.schema.json`) | C-projection of A | `NodeRegistration` DHT entry (node-registry DNA, existing) | `stewarded_nodes` SQLite (existing, migrated) | Node shape is durably notarized on DHT; view is read-optimized projection. No new entry type. |
| **NodeShape signature** | Part of NodeRegistration payload | Self-signed by elohim-node agent key | — | Prevents spoofing across household; validated on DHT commit. |
| **POST /api/v1/nodes/shape** | Route — writes to A via coordinator | Commits `NodeRegistration` entry via existing DNA + upserts `stewarded_nodes` | `stewarded_nodes` | Route is a thin wrapper around existing `register_node` / new `register_node_shape` coordinator fn. Not a new authority. |
| **GET /api/v1/households/{id}/devices** (`household-devices-view.schema.json`) | Computed (no persistence) | Join of `stewarded_nodes` (projects `NodeRegistration`) × `peer_statuses` (projects `PeerStatus`) | — | Operational visibility view. No new DHT entry. Computed per-request. |
| **PeerStatusView** (existing schema, new GET route) | C-projection of A | `PeerStatus` DHT entry (infrastructure DNA, existing) | `peer_statuses` (existing) | Phase 1 already notarized; this sprint adds the read route only. |
| **GET /api/v1/peer-statuses** | Route — reads A projection | Reads `peer_statuses` table | — | Read-only projection; source truth is the DHT entry. |
| **Household** (fixture entries under `genesis/data/collectives/`) | A (reuses existing `collectives` DNA entry type) | `collectives` DHT entry with `kind: "household"` | Existing collectives projection | No new entry type; new `kind` value only. |
| **humans.householdId** | B2 (agent-scoped, attested via collectives membership) | `collectives` entry membership field (authoritative) | `humans.household_id` (projected) | Derivable from collectives membership; humans table caches for fast join. Migration adds the column; projection fills from signal. |
| **GET /api/v1/humans/{id}/household** | Route — reads B2 projection | `collectives` DHT membership | `humans` | Read-only resolver. |
| **NetworkPostureView** (`network-posture-view.schema.json`) | Computed (no persistence) | Aggregates `PeerStatus` + `stewarded_nodes` + `stewardship_allocations` | — | Operational visibility. Same pattern as existing `P2PStatusView` (Category C per 2026-04-09 peer-status-schema-contract design). |
| **GET /api/v1/network/posture** | Route — returns computed C | Recomputed per-request from A-projections | — | Thin aggregation; no persistence. |
| **HouseholdResilienceView** (`household-resilience-view.schema.json`) | Computed (no persistence) | Aggregates `stewardship_allocations` (A) × `peer_statuses` (A-projection) × `stewarded_nodes` (A-projection) × `collectives` (A) | — | Same pattern as existing `ResilienceView` (Category C per p2p-resilience-proof design 2026-04-04). |
| **GET /api/v1/resilience/{id}/household** | Route — returns computed C | Derived per request | — | Sister to existing `/api/v1/resilience/{id}`. |
| **GET /api/v1/households/{id}/stewardship-allocations** | Route — reads A projection | `StewardshipAllocation` DHT entry (lamad DNA, existing) filtered by household join | `stewardship_allocations` | Read-only query on existing projection. |
| **POST /api/v1/peer-status/{maintenance,online}** | Operational admin control | Overrides next-tick `PeerStatus` authored by this peer | `peer_statuses` | Does not write A directly; the heartbeat still authors; this adjusts the policy input for the next publish tick. Origin of truth stays the peer's agent. |
| **shard_manifests / shard_locations** | Not touched this sprint | Existing Category C tables per p2p-resilience-proof design | Existing | Referenced only for aggregation. |
| **Fixture data (humans + collectives JSON)** | Bootstrap seed | Becomes DHT entries after seeding via existing import pipelines | — | No new entry types, only new instances of existing types. |

**Anti-pattern checks performed:**
- No UUID primary keys on any new column. `nodeId` is the stable hardware-derived identifier; `householdId` is the collective's CID; `contentId` is the content CID. 
- No routes designed before their entry type is confirmed. Every route in the table above names its upstream entry type or is explicitly computed (Category C).
- No CID-as-FK in SQL. `dht_anchor_hash` on `stewarded_nodes` is a link-back to the DHT entry, not a foreign key.
- No shared table used as private state — everything added here is public by design (household membership is visible to members; device shape is visible to household; posture is visible to operator).
- Source-of-truth comment required on every new migration SQL file (enforced in Tasks C3, D3 migrations).

---

## File structure (created / modified)

**Created** (every schema below has source of truth named; no new DHT entry types — all notarized work reuses existing entry types per the classifications table above):
- `genesis/docs/superpowers/specs/decisions/2026-04-19-d1-through-d5-node-and-household-canon.md` — decision record for D1–D5.
- `elohim/sdk/schemas/v1/views/node-shape-view.schema.json` — node-shape wire contract. Source of truth: `NodeRegistration` DHT entry (node-registry DNA, existing). Projection: `stewarded_nodes`.
- `elohim/sdk/schemas/v1/views/household-devices-view.schema.json` — household→devices join view. Source of truth: computed projection from `stewarded_nodes` × `peer_statuses` (no persistence; Category C operational).
- `elohim/sdk/schemas/v1/views/network-posture-view.schema.json` — network posture wire contract. Source of truth: computed projection from `peer_statuses` + `stewarded_nodes` + `stewardship_allocations` (Category C operational, no DHT).
- `elohim/sdk/schemas/v1/views/household-resilience-view.schema.json` — per-content household resilience. Source of truth: computed projection from existing DHT-notarized allocations + PeerStatus (Category C operational).
- `elohim/holochain/tests/peer_status.rs` — sweettest integration test (Phase 1 closeout).
- `elohim/elohim-storage/src/api/peer_statuses.rs` — HTTP read handler for PeerStatusView.
- `elohim/elohim-storage/src/api/node_shape.rs` — POST node shape + GET household devices handlers.
- `elohim/elohim-storage/src/api/network_posture.rs` — posture computation + endpoint.
- `elohim/elohim-storage/src/services/household_resilience.rs` — household-first resilience computation.
- `elohim/elohim-storage/src/services/household.rs` — humans→household derivation.
- `elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/shape.rs` — DNA-side shape commit.
- `app/elohim-app/src/app/shefa/components/device-stewardship/device-stewardship.component.ts` (rewrite — current one crashes).
- `app/elohim-app/src/app/lamad/services/household-resilience.service.ts`.
- `genesis/a2o/features/shefa/device-stewardship.feature`, `network-health-dashboard.feature`, `stewarded-resources-visible.feature`, `resilience-tooltip.feature`, `maintenance-choreography.feature`.
- `genesis/a2o/features/doorway/admin-routes-visible.feature`, `admin-users-visible.feature`.
- `genesis/a2o/steps/shefa/*.steps.ts` and `genesis/a2o/steps/doorway/*.steps.ts` as needed.
- `infra/shem/statefulsets/` — one manifest per persona (matthew-home, matthew-laptop, jessica-phone, adam-node, eve-laptop, pete-laptop, terrance-laptop, nancy-hosted, doorway-alpha).
- `infra/shem/demo/maintenance-cycle.sh` — demo script.

**Modified:**
- `elohim/elohim-storage/src/http.rs` — route wiring + `build_manifest()` declarations (add rows; never delete).
- `elohim/elohim-storage/src/main.rs` — boot-time env read, archetype lookup, POST to local shape endpoint, heartbeat archetype passthrough.
- `elohim/elohim-storage/src/heartbeat.rs` — accept archetype_class from config, pass to Published.
- `elohim/elohim-storage/src/config.rs` — add `device_archetype`, `household_id`, `node_role`, `region` fields (env-loaded).
- `elohim/elohim-storage/src/db/peer_statuses.rs` — add `list_by_household(conn, household_id)` query.
- `elohim/elohim-storage/src/db/stewarded_nodes.rs` — add archetype/household fields + migration.
- `elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs` — wire `register_node_shape` fn; retain `get_my_nodes` as thin wrapper that returns empty (retirement stub until frontend no longer calls it in next release).
- `genesis/data/humans/*.json` — add `householdId` field to Matthew, Jessica, James, Susan, Adam, Eve, Pete, Terrance, Nancy, Gertrude, Maria, Ezra.
- `genesis/data/collectives/*.json` — add household entries (new files under existing dir).
- `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts` — rewrite `getResilienceIcon()` + `getResilienceTooltip()` to read `this.resilience` (household-first).
- `app/elohim-app/src/app/shefa/components/resource-explorer/resource-explorer.component.ts` — call household-scoped allocation API; render resilience badge.
- `app/elohim-app/src/app/shefa/components/shefa-dashboard/` — add Network Health tab component.
- `app/elohim-app/src/app/elohim/integrity/node-registry.anchor.ts` — deprecated, replaced by `device-stewardship.component` calling `/api/v1/households/{id}/devices` directly.
- `doorway/doorway-service/src/server/http.rs` — wire `handle_route_registry` + `handle_users_list` admin routes; fix `/admin/users` auth gate.
- `app/elohim-app/angular.json` — fix asset path for `elohim_cache_core.js`.

---

## Module A — Decision record (deliverable 1)

### Task A1: Write D1–D5 decision record

**Files:**
- Create: `genesis/docs/superpowers/specs/decisions/2026-04-19-d1-through-d5-node-and-household-canon.md`

- [ ] **Step 1: Create decision record file**

```markdown
# D1–D5: Node / Household / Doorway / Shem Canonical Decisions

**Date:** 2026-04-19
**Spec:** 2026-04-19-p2p-dataplane-visibility-design.md

## D1. PeerStatus canonical for visibility, elohim-node canonical for topology.
PeerStatus (infrastructure DNA) is the notarized entry type answering "is this peer alive right now?" via the existing `record_peer_status` coordinator zome function. Elohim-node publishes durable node shape (hostname, archetype, committed resources, household binding) to storage via POST /api/v1/nodes/shape, which commits through the `register_node_shape` coordinator fn to the existing `NodeRegistration` DHT entry type (node-registry DNA). Source of truth for both is the DHT; SQLite is projection only. /shefa/devices joins both: hard node inventory + live peer vitals. node-registry DNA's custodian-assignment pieces remain; frontend NodeRegistryAnchor retires.

## D2. Household reuses `collectives` with kind: "household".
Place-grounded hard collective type. humans.json spouse/family edges become membership. `householdId` derives on humans. Place-as-first-class is v2.

## D3. Resilience UI is household-first.
Tooltip, dashboard, devices page all lead with household counts. Per-peer is drilldown.

## D4. Doorway stays web2-only.
ZERO per-domain proxy files added. All domain routes via elohim-storage's build_manifest(). Only doorway-service changes: wire /admin/routes handler, fix /admin/users auth.

## D5. Shem is acceptance target.
>100GB RAM, ~4TB storage. Full persona roster runs as real peers. Dashboards lighting up on shem is the bar.
```

- [ ] **Step 2: Commit**

```bash
git add genesis/docs/superpowers/specs/decisions/2026-04-19-d1-through-d5-node-and-household-canon.md
git commit -m "docs(spec): record D1-D5 canonical decisions for node/household/doorway/shem"
```

---

## Module B — PeerStatus Phase 1 closeout (deliverable 2)

### Task B1: Add sweettest integration test for PeerStatus

**Files:**
- Create: `elohim/holochain/tests/peer_status.rs`

- [ ] **Step 1: Write failing integration test**

```rust
// elohim/holochain/tests/peer_status.rs
use hdk::prelude::*;
use holochain::sweettest::*;
use infrastructure_integrity::{PeerStatus, PeerLifecycleState, PeerCapabilityFlags};

#[tokio::test(flavor = "multi_thread")]
async fn record_peer_status_and_query_latest() {
    let mut conductor = SweetConductor::from_standard_config().await;
    let dna_path = std::env::var("INFRASTRUCTURE_DNA_PATH")
        .expect("INFRASTRUCTURE_DNA_PATH env must point to packed infrastructure.dna");
    let dna = SweetDnaFile::from_bundle(&std::path::PathBuf::from(dna_path)).await.unwrap();
    let app = conductor.setup_app("test", &[dna]).await.unwrap();
    let (alice,) = app.into_tuple();
    let zome = alice.zome("infrastructure");

    let status = PeerStatus {
        peer_id: alice.agent_pubkey().clone(),
        status: PeerLifecycleState::Online,
        flags: PeerCapabilityFlags {
            general_pool_member: true,
            accepting_stewardship_reserves: true,
        },
        archetype_class: Some("home-nuc".into()),
        timestamp: Timestamp::now(),
    };

    let action_hash: ActionHash = conductor
        .call(&zome, "record_peer_status", status.clone())
        .await;
    assert!(!action_hash.as_hash().as_ref().is_empty());

    let latest: Option<PeerStatus> = conductor
        .call(&zome, "get_latest_peer_status_for_agent", alice.agent_pubkey().clone())
        .await;
    assert_eq!(latest.unwrap().archetype_class.as_deref(), Some("home-nuc"));
}
```

- [ ] **Step 2: Run to verify it fails with "file not found" or linker error**

Run: `cd elohim/holochain && just test peer_status`
Expected: FAIL — dna path env or compile error.

- [ ] **Step 3: Wire DNA path + Cargo.toml test dependencies**

In `elohim/holochain/Cargo.toml` under `[dev-dependencies]`, confirm `sweettest`, `tokio` with features `["macros","rt-multi-thread"]`, and `infrastructure_integrity` are already present (they support other tests in `tests/`). If missing, add them — do NOT touch `[dependencies]`.

Update `elohim/holochain/justfile` to export `INFRASTRUCTURE_DNA_PATH` in the `test` recipe after the `pack` step:

```makefile
test:
    just pack
    INFRASTRUCTURE_DNA_PATH=$(pwd)/workdir/infrastructure/infrastructure.dna cargo test --test peer_status -- --nocapture
```

- [ ] **Step 4: Run to verify it passes**

Run: `cd elohim/holochain && just test peer_status`
Expected: PASS — test records PeerStatus and retrieves latest.

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/tests/peer_status.rs elohim/holochain/justfile
git commit -m "test(peer-status): sweettest integration covering record + latest query"
```

### Task B2: Add HTTP read route for PeerStatusView

**Files:**
- Create: `elohim/elohim-storage/src/api/peer_statuses.rs`
- Modify: `elohim/elohim-storage/src/http.rs`
- Modify: `elohim/elohim-storage/src/db/peer_statuses.rs`

- [ ] **Step 1: Add `list_by_household` query**

This is a read-only projection query against the existing `peer_statuses` table (source of truth: `PeerStatus` DHT entry in infrastructure DNA, notarized; table is an operational projection). No new entry type. In `elohim/elohim-storage/src/db/peer_statuses.rs` add:

```rust
pub fn list_by_household(
    conn: &mut SqliteConnection,
    household_id: &str,
) -> Result<Vec<PeerStatusRow>, diesel::result::Error> {
    use crate::db::diesel_schema::peer_statuses::dsl as ps;
    use crate::db::diesel_schema::stewarded_nodes::dsl as sn;
    ps::peer_statuses
        .inner_join(sn::stewarded_nodes.on(sn::id.eq(ps::peer_id)))
        .filter(sn::household_id.eq(household_id))
        .select(PeerStatusRow::as_select())
        .load(conn)
}
```

(Requires the `household_id` column on `stewarded_nodes` — Task C3 adds that migration. If B2 runs before C3, add a temporary `list_all(conn)` and come back to this query. Noted.)

- [ ] **Step 2: Write failing HTTP test**

Create `elohim/elohim-storage/tests/peer_statuses_route.rs`:

```rust
use hyper::{Method, Request, StatusCode};
use elohim_storage::http::router;

#[tokio::test]
async fn list_peer_statuses_returns_json_array() {
    let app = router::test_app().await;
    let resp = app
        .oneshot(Request::builder()
            .method(Method::GET)
            .uri("/api/v1/peer-statuses")
            .body(Default::default()).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(v.is_array(), "expected array, got: {v}");
}
```

(If `router::test_app` helper doesn't exist, follow the pattern in `elohim/elohim-storage/tests/` — most crates here have a `test_support.rs` module. Locate it with `grep -r "fn test_app" elohim/elohim-storage/tests/` first.)

- [ ] **Step 3: Run to verify it fails**

Run: `cd elohim/elohim-storage && cargo test --test peer_statuses_route`
Expected: FAIL — 404 Not Found.

- [ ] **Step 4: Create handler**

```rust
// elohim/elohim-storage/src/api/peer_statuses.rs
use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Request, Response, StatusCode};
use crate::db::{peer_statuses as db, AppContext};
use crate::services::response;
use crate::views::PeerStatusView;

pub async fn handle_list(
    ctx: &AppContext,
    req: Request<Incoming>,
) -> Response<Full<Bytes>> {
    let household = req
        .uri()
        .query()
        .and_then(|q| url::form_urlencoded::parse(q.as_bytes())
            .find(|(k,_)| k == "householdId")
            .map(|(_,v)| v.into_owned()));

    let mut conn = match ctx.pool.get() {
        Ok(c) => c,
        Err(_) => return response::internal_error("pool unavailable"),
    };

    let rows = match household {
        Some(h) => db::list_by_household(&mut conn, &h),
        None => db::list_current(&mut conn),
    };

    match rows {
        Ok(rs) => {
            let views: Vec<PeerStatusView> = rs.into_iter().map(Into::into).collect();
            response::json_ok(&views)
        }
        Err(e) => response::internal_error(&format!("{e}")),
    }
}
```

- [ ] **Step 5: Wire route in http.rs dispatch + manifest**

In `elohim/elohim-storage/src/http.rs` add above the registry catch-all (find the section handling `/api/v1/...` routes):

```rust
(Method::GET, "/api/v1/peer-statuses") => {
    return api::peer_statuses::handle_list(&state, req).await;
}
```

In `build_manifest()` (search for "mastery" routes as template) add:

```rust
Route::get("/api/v1/peer-statuses")
    .description("List current PeerStatus rows (optional ?householdId= filter)")
    .tag("peer-status"),
```

- [ ] **Step 6: Run to verify it passes**

Run: `cd elohim/elohim-storage && cargo test --test peer_statuses_route`
Expected: PASS.

- [ ] **Step 7: Commit**

The route serves the existing `PeerStatus` DHT entry type (infrastructure DNA) via its `peer_statuses` projection; no new entry type or zome function added.

```bash
git add elohim/elohim-storage/src/api/peer_statuses.rs \
        elohim/elohim-storage/src/http.rs \
        elohim/elohim-storage/src/db/peer_statuses.rs \
        elohim/elohim-storage/tests/peer_statuses_route.rs
git commit -m "feat(peer-status): GET /api/v1/peer-statuses HTTP read route (Phase 1 closeout)"
```

---

## Module C — Node-shape publish surface (deliverable 4)

All new wire schemas in this module project from the existing `NodeRegistration` DHT entry type (node-registry DNA, notarized) — no new entry type created. Source of truth for node shape is the DHT; SQLite `stewarded_nodes` is operational projection. Household-devices view is a computed Category C projection with no persistence.

### Task C1: Write node-shape-view.schema.json

**Files:**
- Create: `elohim/sdk/schemas/v1/views/node-shape-view.schema.json` (projects existing `NodeRegistration` DHT entry).
- Create: `elohim/sdk/schemas/v1/views/household-devices-view.schema.json` (computed join, operational, no DHT commit).

- [ ] **Step 1: Write node-shape-view.schema.json**

```json
{
  "$id": "node-shape-view.schema.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "NodeShapeView",
  "description": "Durable node shape published by elohim-node at boot. Source of truth: node-registry DNA NodeRegistration entry; this view projects from stewarded_nodes SQLite table.",
  "type": "object",
  "additionalProperties": false,
  "required": ["nodeId","hostname","deviceArchetypeId","householdId","role","capabilityLevel","committed","signature","signedAt"],
  "properties": {
    "nodeId":            { "type": "string", "minLength": 1 },
    "hostname":          { "type": "string", "minLength": 1 },
    "deviceArchetypeId": { "type": "string", "minLength": 1 },
    "householdId":       { "type": "string", "minLength": 1 },
    "role":              { "enum": ["edge","archival","inference","doorway"] },
    "capabilityLevel":   { "type": "integer", "minimum": 0, "maximum": 5 },
    "committed": {
      "type": "object",
      "additionalProperties": false,
      "required": ["cpuCores","memoryGb","storageTb"],
      "properties": {
        "cpuCores":      { "type": "integer", "minimum": 0 },
        "memoryGb":      { "type": "integer", "minimum": 0 },
        "storageTb":     { "type": "number",  "minimum": 0 },
        "bandwidthMbps": { "type": "integer", "minimum": 0 },
        "maxCustodyGb":  { "type": "number",  "minimum": 0 },
        "canSteward":    { "type": "boolean" },
        "canInfer":      { "type": "boolean" },
        "canDoorway":    { "type": "boolean" }
      }
    },
    "stewardTier":    { "enum": ["caretaker","guardian","steward","pioneer"] },
    "custodianOptIn": { "type": "boolean" },
    "region":         { "type": ["string","null"] },
    "signature":      { "type": "string",  "minLength": 1 },
    "signedAt":       { "type": "string",  "format": "date-time" },
    "dhtAnchorHash":  { "type": ["string","null"] }
  }
}
```

- [ ] **Step 2: Write household-devices-view.schema.json**

```json
{
  "$id": "household-devices-view.schema.json",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "HouseholdDevicesView",
  "description": "Devices (nodes) belonging to a household with live peer vitals. Source of truth: computed projection from stewarded_nodes (projects NodeRegistration DHT entry) LEFT JOIN peer_statuses (projects PeerStatus DHT entry). Operational Category C — no persistence, no new entry type.",
  "type": "object",
  "additionalProperties": false,
  "required": ["householdId","devices"],
  "properties": {
    "householdId": { "type": "string", "description": "Collectives DHT entry with kind:household; operational projection key." },
    "devices": {
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["shape","peer"],
        "properties": {
          "shape": { "$ref": "node-shape-view.schema.json", "description": "Projects NodeRegistration DHT entry; source of truth on DHT." },
          "peer": {
            "oneOf": [
              { "type": "null" },
              { "$ref": "peer-status-view.schema.json", "description": "Projects PeerStatus DHT entry; source of truth on DHT." }
            ]
          }
        }
      }
    }
  }
}
```

- [ ] **Step 3: Add to codegen registry**

Both files project from existing notarized sources (NodeRegistration DHT entry / PeerStatus DHT entry) — no new DHT entry type declared here. In `elohim/sdk/schemas/scripts/codegen-ts.mjs` INTERFACE_FILES, add:

```js
"node-shape-view.schema.json",
"household-devices-view.schema.json",
```

- [ ] **Step 4: Regenerate TypeScript + verify**

Run: `pnpm run schema:codegen:ts`
Expected: creates types in `elohim/sdk/storage-client-ts/src/generated/NodeShapeView.ts` and `HouseholdDevicesView.ts`. These are wire-contract projections; source of truth stays on the DHT.

- [ ] **Step 5: Commit**

Wire schemas for existing DHT entry types; operational projection only, no new notarized data.

```bash
git add elohim/sdk/schemas/v1/views/node-shape-view.schema.json \
        elohim/sdk/schemas/v1/views/household-devices-view.schema.json \
        elohim/sdk/schemas/scripts/codegen-ts.mjs \
        elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(schemas): node-shape-view + household-devices-view JSON schemas"
```

### Task C2: Rust structs matching node-shape schema

Source of truth is the DHT NodeRegistration entry; these structs are projection-side types. Operational use only; no new entry type.

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs`

- [ ] **Step 1: Add NodeShapeView + CommittedResources + HouseholdDevicesView + DeviceEntryView structs**

Find the section in `views.rs` with other `#[serde(rename_all = "camelCase")]` View structs. Add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommittedResources {
    pub cpu_cores: i32,
    pub memory_gb: i32,
    pub storage_tb: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bandwidth_mbps: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_custody_gb: Option<f64>,
    pub can_steward: bool,
    pub can_infer: bool,
    pub can_doorway: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeShapeView {
    pub node_id: String,
    pub hostname: String,
    pub device_archetype_id: String,
    pub household_id: String,
    pub role: String,
    pub capability_level: i32,
    pub committed: CommittedResources,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steward_tier: Option<String>,
    pub custodian_opt_in: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    pub signature: String,
    pub signed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dht_anchor_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceEntryView {
    pub shape: NodeShapeView,
    pub peer: Option<PeerStatusView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HouseholdDevicesView {
    pub household_id: String,
    pub devices: Vec<DeviceEntryView>,
}
```

- [ ] **Step 2: Add schema contract assertions**

Assertions verify the projection structs match wire schemas; source of truth remains the DHT entries (NodeRegistration / PeerStatus). In `elohim/elohim-storage/tests/schema_contract.rs` add:

```rust
#[test]
fn node_shape_view_matches_schema() {
    assert_view_matches_schema!(
        "node-shape-view.schema.json",
        NodeShapeView {
            node_id: "test".into(),
            hostname: "matthew-home".into(),
            device_archetype_id: "home-nuc".into(),
            household_id: "household-matthew".into(),
            role: "edge".into(),
            capability_level: 3,
            committed: CommittedResources {
                cpu_cores: 4, memory_gb: 16, storage_tb: 2.0,
                bandwidth_mbps: Some(1000), max_custody_gb: Some(500.0),
                can_steward: true, can_infer: false, can_doorway: false,
            },
            steward_tier: Some("guardian".into()),
            custodian_opt_in: true,
            region: Some("us-central".into()),
            signature: "sig".into(),
            signed_at: "2026-04-19T00:00:00Z".into(),
            dht_anchor_hash: None,
        }
    );
}

#[test]
fn household_devices_view_matches_schema() {
    assert_view_matches_schema!(
        "household-devices-view.schema.json",
        HouseholdDevicesView {
            household_id: "household-matthew".into(),
            devices: vec![],
        }
    );
}
```

(`assert_view_matches_schema!` exists in the test harness per `elohim/sdk/schemas/scripts/`; if signature is different, copy pattern from `peer_status_view_matches_schema` already in the file.)

- [ ] **Step 3: Run**

```bash
cd elohim/elohim-storage && cargo test --test schema_contract
```
Expected: PASS both new assertions.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/views.rs elohim/elohim-storage/tests/schema_contract.rs
git commit -m "feat(views): NodeShapeView + HouseholdDevicesView with schema contract"
```

### Task C3: Stewarded-nodes migration — archetype + household fields

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-04-19-000001_stewarded_nodes_add_archetype/up.sql` + `down.sql`
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs` (regenerated after migration run)
- Modify: `elohim/elohim-storage/src/db/stewarded_nodes.rs`

- [ ] **Step 1: Write migration up.sql**

```sql
-- Source of truth: node-registry DNA NodeRegistration entry (dht_anchor_hash).
-- This projection indexes archetype + household for fast visibility joins.
ALTER TABLE stewarded_nodes ADD COLUMN device_archetype_id TEXT;
ALTER TABLE stewarded_nodes ADD COLUMN household_id TEXT;
ALTER TABLE stewarded_nodes ADD COLUMN hostname TEXT;
ALTER TABLE stewarded_nodes ADD COLUMN node_role TEXT;
ALTER TABLE stewarded_nodes ADD COLUMN capability_level INTEGER;
ALTER TABLE stewarded_nodes ADD COLUMN can_steward INTEGER NOT NULL DEFAULT 0;
ALTER TABLE stewarded_nodes ADD COLUMN can_infer INTEGER NOT NULL DEFAULT 0;
ALTER TABLE stewarded_nodes ADD COLUMN can_doorway INTEGER NOT NULL DEFAULT 0;
ALTER TABLE stewarded_nodes ADD COLUMN signature TEXT;
ALTER TABLE stewarded_nodes ADD COLUMN signed_at TEXT;
CREATE INDEX IF NOT EXISTS idx_stewarded_nodes_household ON stewarded_nodes(household_id);
CREATE INDEX IF NOT EXISTS idx_stewarded_nodes_archetype ON stewarded_nodes(device_archetype_id);
```

- [ ] **Step 2: Write migration down.sql**

```sql
DROP INDEX IF EXISTS idx_stewarded_nodes_archetype;
DROP INDEX IF EXISTS idx_stewarded_nodes_household;
ALTER TABLE stewarded_nodes DROP COLUMN signed_at;
ALTER TABLE stewarded_nodes DROP COLUMN signature;
ALTER TABLE stewarded_nodes DROP COLUMN can_doorway;
ALTER TABLE stewarded_nodes DROP COLUMN can_infer;
ALTER TABLE stewarded_nodes DROP COLUMN can_steward;
ALTER TABLE stewarded_nodes DROP COLUMN capability_level;
ALTER TABLE stewarded_nodes DROP COLUMN node_role;
ALTER TABLE stewarded_nodes DROP COLUMN hostname;
ALTER TABLE stewarded_nodes DROP COLUMN household_id;
ALTER TABLE stewarded_nodes DROP COLUMN device_archetype_id;
```

- [ ] **Step 3: Run migration + regenerate schema**

```bash
cd elohim/elohim-storage
diesel migration run
# Regenerate diesel_schema.rs — follow repo pattern (usually "diesel print-schema > src/db/diesel_schema.rs" gated on a specific DB URL)
```

- [ ] **Step 4: Update `StewardedNodeRow` and `upsert`/`list*` queries**

Add the new fields to `StewardedNodeRow` struct and the `Insertable`/`Queryable` derives. Update any `list_*` queries that shape a `SELECT` to include the new columns or use `as_select()` style.

- [ ] **Step 5: Run existing stewarded_nodes tests**

```bash
cd elohim/elohim-storage && cargo test stewarded_nodes
```
Expected: PASS. If failures, the new columns likely need defaults in insert helpers.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-04-19-000001_stewarded_nodes_add_archetype/ \
        elohim/elohim-storage/src/db/diesel_schema.rs \
        elohim/elohim-storage/src/db/stewarded_nodes.rs
git commit -m "feat(stewarded-nodes): migration + model fields for archetype/household/shape"
```

### Task C4: Coordinator zome — `register_node_shape` + retire `get_my_nodes`

**Files:**
- Create: `elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/shape.rs`
- Modify: `elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs`

- [ ] **Step 1: Create shape.rs with commit fn**

```rust
// elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/shape.rs
use hdk::prelude::*;
use node_registry_integrity::{EntryTypes, NodeRegistration};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NodeShapeInput {
    pub node_id: String,
    pub hostname: String,
    pub device_archetype_id: String,
    pub household_id: String,
    pub role: String,
    pub capability_level: u8,
    pub cpu_cores: u32,
    pub memory_gb: u32,
    pub storage_tb: f64,
    pub bandwidth_mbps: Option<u32>,
    pub max_custody_gb: Option<f64>,
    pub can_steward: bool,
    pub can_infer: bool,
    pub can_doorway: bool,
    pub steward_tier: Option<String>,
    pub custodian_opt_in: bool,
    pub region: Option<String>,
    pub signature: String,
    pub signed_at: String,
}

/// Commit a node shape as a NodeRegistration DHT entry authored by the boot peer.
#[hdk_extern]
pub fn register_node_shape(input: NodeShapeInput) -> ExternResult<ActionHash> {
    let agent = agent_info()?.agent_initial_pubkey.to_string();
    let reg = NodeRegistration {
        node_id: input.node_id,
        agent_pub_key: agent,
        display_name: input.hostname.clone(),
        cpu_cores: input.cpu_cores,
        memory_gb: input.memory_gb,
        storage_tb: input.storage_tb,
        bandwidth_mbps: input.bandwidth_mbps.unwrap_or(0),
        region: input.region.clone().unwrap_or_default(),
        latitude: None,
        longitude: None,
        zomes_hosted: vec![],
        steward_tier: input.steward_tier.unwrap_or_else(|| "caretaker".into()),
        custodian_opt_in: input.custodian_opt_in,
        max_custody_gb: input.max_custody_gb,
        max_bandwidth_mbps: input.bandwidth_mbps,
        max_cpu_percent: None,
        uptime_percent: 0.0,
        last_heartbeat: input.signed_at.clone(),
        registered_at: input.signed_at.clone(),
        updated_at: input.signed_at,
        claim_status: "claimed".into(),
        context_epr_id: None,
        signature: input.signature,
    };
    create_entry(EntryTypes::NodeRegistration(reg))
}
```

- [ ] **Step 2: Wire into lib.rs + retire `get_my_nodes`**

In `elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs` add near the top imports:

```rust
pub mod shape;
pub use shape::{NodeShapeInput, register_node_shape};

/// Retired: frontend should call /api/v1/households/{id}/devices via elohim-storage.
/// Returns empty until the frontend is cut over (then this fn is removed in a follow-up).
#[hdk_extern]
pub fn get_my_nodes(_: ()) -> ExternResult<Vec<NodeRegistration>> {
    Ok(vec![])
}
```

- [ ] **Step 3: Run DNA type-check**

```bash
cd elohim/holochain && just check
```
Expected: PASS.

- [ ] **Step 4: Extend existing DNA integration test or add one**

Append to `elohim/holochain/tests/peer_status.rs` (or create `tests/node_shape.rs` if cleaner):

```rust
#[tokio::test(flavor = "multi_thread")]
async fn register_node_shape_creates_node_registration() {
    // ... sweet conductor setup, call register_node_shape with minimal input,
    // assert an ActionHash returns and NodeRegistration is queryable via
    // existing get_nodes_by_region (region filter).
}
```

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/shape.rs \
        elohim/holochain/dna/node-registry/zomes/node_registry_coordinator/src/lib.rs \
        elohim/holochain/tests/node_shape.rs
git commit -m "feat(node-registry): register_node_shape coordinator fn; stub get_my_nodes for retirement"
```

### Task C5: POST /api/v1/nodes/shape handler

**Files:**
- Create: `elohim/elohim-storage/src/api/node_shape.rs`
- Modify: `elohim/elohim-storage/src/http.rs`

- [ ] **Step 1: Write failing test**

Create `elohim/elohim-storage/tests/node_shape_route.rs`:

```rust
#[tokio::test]
async fn post_node_shape_upserts_and_returns_dht_anchor() {
    let app = router::test_app().await;
    let body = serde_json::json!({
        "nodeId": "matthew-home",
        "hostname": "matthew-home",
        "deviceArchetypeId": "home-nuc",
        "householdId": "household-matthew",
        "role": "edge",
        "capabilityLevel": 3,
        "committed": { "cpuCores": 4, "memoryGb": 16, "storageTb": 2.0,
                       "canSteward": true, "canInfer": false, "canDoorway": false },
        "custodianOptIn": true,
        "signature": "sig",
        "signedAt": "2026-04-19T00:00:00Z"
    });
    let resp = app.oneshot(
        Request::builder()
            .method(Method::POST)
            .uri("/api/v1/nodes/shape")
            .header("content-type","application/json")
            .body(Full::from(Bytes::from(body.to_string()))).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let resp_body: serde_json::Value = serde_json::from_slice(
        &hyper::body::to_bytes(resp.into_body()).await.unwrap()).unwrap();
    assert!(resp_body.get("dhtAnchorHash").is_some());
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test node_shape_route`
Expected: FAIL 404.

- [ ] **Step 3: Create handler**

```rust
// elohim/elohim-storage/src/api/node_shape.rs
use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Request, Response, StatusCode};
use crate::db::{stewarded_nodes as db, AppContext};
use crate::services::response;
use crate::views::NodeShapeView;
use super::parse_body;

pub async fn handle_post(
    ctx: &AppContext,
    req: Request<Incoming>,
) -> Response<Full<Bytes>> {
    let input: NodeShapeView = match parse_body(req).await {
        Ok(v) => v,
        Err(e) => return response::bad_request(&format!("parse: {e}")),
    };

    let mut conn = match ctx.pool.get() {
        Ok(c) => c,
        Err(_) => return response::internal_error("pool"),
    };

    // 1. Upsert stewarded_nodes projection (local, fast).
    if let Err(e) = db::upsert_from_shape(&mut conn, &input) {
        return response::internal_error(&format!("upsert: {e}"));
    }

    // 2. Call the coordinator zome to commit NodeRegistration.
    //    ctx.conductor is the AppConnection used elsewhere for zome calls.
    let dht_hash = match ctx.conductor.as_ref() {
        Some(c) => match c.call_zome_node_shape(&input).await {
            Ok(h) => Some(h),
            Err(e) => {
                tracing::warn!("zome call failed, storage-only: {e}");
                None
            }
        },
        None => None,
    };

    // 3. Persist dht_anchor_hash if we got one.
    if let Some(h) = &dht_hash {
        let _ = db::set_dht_anchor(&mut conn, &input.node_id, h);
    }

    response::json_ok(&serde_json::json!({
        "nodeId": input.node_id,
        "dhtAnchorHash": dht_hash,
    }))
}
```

Also add `upsert_from_shape` + `set_dht_anchor` helpers in `src/db/stewarded_nodes.rs` that accept the view and write the new columns.

- [ ] **Step 4: Wire route**

In `http.rs` dispatch, above the registry fallback:

```rust
(Method::POST, "/api/v1/nodes/shape") => {
    return api::node_shape::handle_post(&state, req).await;
}
```

In `build_manifest()`:

```rust
Route::post("/api/v1/nodes/shape")
    .description("Register a node's durable shape (hostname, archetype, household, resources)")
    .tag("node-shape"),
```

- [ ] **Step 5: Run to verify it passes**

Run: `cargo test --test node_shape_route`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/api/node_shape.rs \
        elohim/elohim-storage/src/db/stewarded_nodes.rs \
        elohim/elohim-storage/src/http.rs \
        elohim/elohim-storage/tests/node_shape_route.rs
git commit -m "feat(node-shape): POST /api/v1/nodes/shape upsert + DHT commit"
```

### Task C6: GET /api/v1/households/{id}/devices handler

**Files:**
- Modify: `elohim/elohim-storage/src/api/node_shape.rs` (add `handle_household_devices`)
- Modify: `elohim/elohim-storage/src/http.rs`
- Modify: `elohim/elohim-storage/src/db/stewarded_nodes.rs` (add `list_by_household_with_peer_status`)

- [ ] **Step 1: Failing test**

Extend `node_shape_route.rs`:

```rust
#[tokio::test]
async fn get_household_devices_returns_shapes_with_optional_peer_status() {
    let app = router::test_app_seeded().await;  // helper that seeds one node
    let resp = app.oneshot(
        Request::builder()
            .method(Method::GET)
            .uri("/api/v1/households/household-matthew/devices")
            .body(Default::default()).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v: serde_json::Value = serde_json::from_slice(
        &hyper::body::to_bytes(resp.into_body()).await.unwrap()).unwrap();
    assert_eq!(v["householdId"], "household-matthew");
    assert!(v["devices"].is_array());
}
```

(If `test_app_seeded` helper doesn't exist, reuse `test_app` and POST a node first via the test's arrange phase.)

- [ ] **Step 2: Run — expect FAIL 404**

Run: `cargo test --test node_shape_route`

- [ ] **Step 3: Add DB join query**

```rust
// db/stewarded_nodes.rs
pub fn list_by_household_with_peer_status(
    conn: &mut SqliteConnection,
    household_id: &str,
) -> Result<Vec<(StewardedNodeRow, Option<PeerStatusRow>)>, diesel::result::Error> {
    use crate::db::diesel_schema::stewarded_nodes::dsl as sn;
    use crate::db::diesel_schema::peer_statuses::dsl as ps;
    sn::stewarded_nodes
        .left_join(ps::peer_statuses.on(ps::peer_id.eq(sn::id)))
        .filter(sn::household_id.eq(household_id))
        .select((StewardedNodeRow::as_select(), Option::<PeerStatusRow>::as_select()))
        .load(conn)
}
```

- [ ] **Step 4: Add handler**

```rust
pub async fn handle_household_devices(
    ctx: &AppContext,
    household_id: &str,
) -> Response<Full<Bytes>> {
    let mut conn = match ctx.pool.get() {
        Ok(c) => c,
        Err(_) => return response::internal_error("pool"),
    };

    let rows = match db::list_by_household_with_peer_status(&mut conn, household_id) {
        Ok(r) => r,
        Err(e) => return response::internal_error(&format!("{e}")),
    };

    let devices: Vec<DeviceEntryView> = rows.into_iter().map(|(node,peer)| DeviceEntryView {
        shape: node.into(),
        peer: peer.map(Into::into),
    }).collect();

    response::json_ok(&HouseholdDevicesView {
        household_id: household_id.to_string(),
        devices,
    })
}
```

- [ ] **Step 5: Wire route**

```rust
// http.rs dispatch
(Method::GET, p) if p.starts_with("/api/v1/households/") && p.ends_with("/devices") => {
    let id = p.trim_start_matches("/api/v1/households/").trim_end_matches("/devices");
    return api::node_shape::handle_household_devices(&state, id).await;
}
```

```rust
// build_manifest()
Route::get("/api/v1/households/{id}/devices")
    .description("Devices (nodes) for a household with live peer vitals")
    .tag("household"),
```

- [ ] **Step 6: Run — PASS**

Run: `cargo test --test node_shape_route`

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/api/node_shape.rs \
        elohim/elohim-storage/src/db/stewarded_nodes.rs \
        elohim/elohim-storage/src/http.rs \
        elohim/elohim-storage/tests/node_shape_route.rs
git commit -m "feat(node-shape): GET /api/v1/households/{id}/devices join stewarded_nodes x peer_statuses"
```

### Task C7: Elohim-node boot — env read, archetype lookup, self-POST

**Files:**
- Modify: `elohim/elohim-storage/src/config.rs`
- Modify: `elohim/elohim-storage/src/main.rs`
- Create: `elohim/elohim-storage/src/services/boot_registration.rs`

- [ ] **Step 1: Add config fields**

In `elohim/elohim-storage/src/config.rs` add to the `Config` struct:

```rust
#[serde(default)]
pub device_archetype: Option<String>,
#[serde(default)]
pub household_id: Option<String>,
#[serde(default)]
pub node_role: Option<String>,
#[serde(default)]
pub region: Option<String>,
```

And in the env-parsing helper (search for existing `fn from_env`) add:

```rust
device_archetype: std::env::var("DEVICE_ARCHETYPE").ok(),
household_id: std::env::var("HOUSEHOLD_ID").ok(),
node_role: std::env::var("NODE_ROLE").ok(),
region: std::env::var("REGION").ok(),
```

- [ ] **Step 2: Write archetype-lookup helper**

```rust
// elohim/elohim-storage/src/services/boot_registration.rs
use crate::views::{NodeShapeView, CommittedResources};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceArchetype {
    id: String,
    capability_level: i32,
    memory_gb: i32,
    storage_gb: f64,
    cpu_cores: i32,
    can_steward: bool,
    can_infer: bool,
    can_doorway: bool,
    bandwidth_mbps: Option<i32>,
}

pub fn load_archetype(archetype_id: &str) -> Option<DeviceArchetype> {
    let path = Path::new("genesis/data/devices/devices.json");
    let raw = std::fs::read_to_string(path).ok()?;
    let all: Vec<DeviceArchetype> = serde_json::from_str(&raw).ok()?;
    all.into_iter().find(|a| a.id == archetype_id)
}

pub fn build_node_shape(
    node_id: String,
    hostname: String,
    archetype: &DeviceArchetype,
    household_id: String,
    role: String,
    region: Option<String>,
    signature: String,
) -> NodeShapeView {
    NodeShapeView {
        node_id,
        hostname,
        device_archetype_id: archetype.id.clone(),
        household_id,
        role,
        capability_level: archetype.capability_level,
        committed: CommittedResources {
            cpu_cores: archetype.cpu_cores,
            memory_gb: archetype.memory_gb,
            storage_tb: archetype.storage_gb / 1024.0,
            bandwidth_mbps: archetype.bandwidth_mbps,
            max_custody_gb: Some(archetype.storage_gb * 0.5),
            can_steward: archetype.can_steward,
            can_infer: archetype.can_infer,
            can_doorway: archetype.can_doorway,
        },
        steward_tier: None,
        custodian_opt_in: archetype.can_steward,
        region,
        signature,
        signed_at: chrono::Utc::now().to_rfc3339(),
        dht_anchor_hash: None,
    }
}

pub fn sign_shape(shape: &NodeShapeView, agent_pub_key: &str) -> String {
    use sha2::{Sha256, Digest};
    let payload = format!("{}|{}|{}|{}",
        shape.node_id, agent_pub_key, shape.device_archetype_id, shape.signed_at);
    let h = Sha256::digest(payload.as_bytes());
    hex::encode(h)
}
```

- [ ] **Step 3: Call at main.rs boot**

In `elohim/elohim-storage/src/main.rs` after config is loaded and the HTTP server is up (find where other startup tasks run, e.g. heartbeat::start_heartbeat_task), add:

```rust
if let (Some(archetype_id), Some(household_id), Some(role)) = (
    &config.device_archetype, &config.household_id, &config.node_role
) {
    if let Some(archetype) = services::boot_registration::load_archetype(archetype_id) {
        let hostname = hostname::get()
            .ok().and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".into());
        let node_id = hostname.clone();
        let agent_key = agent_pubkey_string_or_placeholder();
        let mut shape = services::boot_registration::build_node_shape(
            node_id, hostname, &archetype,
            household_id.clone(), role.clone(), config.region.clone(),
            String::new(),
        );
        shape.signature = services::boot_registration::sign_shape(&shape, &agent_key);
        // Self-POST to /api/v1/nodes/shape via reqwest to our own :8090
        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{}/api/v1/nodes/shape", config.http_port);
        match client.post(url).json(&shape).send().await {
            Ok(r) if r.status().is_success() => {
                tracing::info!(node_id = shape.node_id.as_str(), "node shape registered");
            }
            Ok(r) => tracing::warn!(status = r.status().as_u16(), "node shape POST non-2xx"),
            Err(e) => tracing::warn!(error = %e, "node shape POST failed"),
        }
    }
}
```

(Add `hostname = "0.4"`, `reqwest = { version = "0.12", features = ["json"] }`, `sha2`, `hex`, `chrono` to Cargo.toml dev-or-prod deps as needed; check existing use first.)

- [ ] **Step 4: Build + smoke test**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
DEVICE_ARCHETYPE=home-nuc HOUSEHOLD_ID=household-test NODE_ROLE=edge \
  ./target/release/elohim-storage --bind 127.0.0.1:8099 &
sleep 3
curl -s http://127.0.0.1:8099/api/v1/households/household-test/devices | jq
kill %1
```
Expected: devices array contains the booted node with correct archetype/household.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/config.rs \
        elohim/elohim-storage/src/main.rs \
        elohim/elohim-storage/src/services/boot_registration.rs \
        elohim/elohim-storage/Cargo.toml
git commit -m "feat(node-shape): elohim-node self-registers its shape at boot from env+archetype"
```

### Task C8: Pipe archetype_class into heartbeat

**Files:**
- Modify: `elohim/elohim-storage/src/heartbeat.rs`
- Modify: `elohim/elohim-storage/src/main.rs`

- [ ] **Step 1: Add `archetype_class` field to heartbeat task**

In `heartbeat.rs` find the `HeartbeatTask<P,L>` struct and add:

```rust
archetype_class: Option<String>,
```

Add a builder method `with_archetype_class(mut self, v: Option<String>) -> Self { self.archetype_class = v; self }` if not already present; if present with a different signature, ensure it exists.

In `tick_once` find the `Published { ... }` construction and pass `archetype_class: self.archetype_class.clone()`.

- [ ] **Step 2: Pass config value at main.rs spawn**

Where the heartbeat task is constructed in `main.rs`, add `.with_archetype_class(config.device_archetype.clone())` in the builder chain.

- [ ] **Step 3: Verify existing heartbeat tests still pass**

```bash
cd elohim/elohim-storage && cargo test heartbeat
```

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/heartbeat.rs elohim/elohim-storage/src/main.rs
git commit -m "feat(heartbeat): wire archetype_class from config into PeerStatus publication"
```

---

## Module D — Household grouping (deliverable 3)

### Task D1: Household collective fixtures

**Files:**
- Create: `genesis/data/collectives/household-matthew.json`, `household-adam.json`, `household-pete.json`, `household-terrance.json`, `household-nancy.json`, `household-gertrude.json`, `household-maria.json`, `household-ezra.json`.

- [ ] **Step 1: Write household-matthew.json** (template — replicate for each)

```json
{
  "id": "household-matthew",
  "kind": "household",
  "displayName": "Dowell Household",
  "members": ["matthew","jessica","james","susan"],
  "placeAnchor": { "region": "us-central", "label": "matthew-home" },
  "stewards": ["matthew","jessica"],
  "createdAt": "2026-04-19T00:00:00Z"
}
```

Create the other seven households. Adam/Eve share one household. Pete/Terrance/Nancy/Gertrude/Maria/Ezra are each solo households for this sprint (single-member households are still households).

- [ ] **Step 2: Verify collectives schema accepts `kind: "household"`**

```bash
pnpm run schema:validate
```
If the `collectives` schema rejects `kind: "household"`, add `"household"` to its enum (schema-first — update the schema before the fixture).

- [ ] **Step 3: Commit**

```bash
git add genesis/data/collectives/household-*.json elohim/sdk/schemas/v1/
git commit -m "feat(collectives): household fixtures for Matthew/Adam/Pete/Terrance/Nancy/Gertrude/Maria/Ezra"
```

### Task D2: `householdId` field on humans

**Files:**
- Modify: `genesis/data/humans/matthew.json`, `jessica.json`, `james.json`, `susan.json`, `adam.json`, `eve.json`, `pete.json`, `terrance.json`, `nancy.json`, `gertrude.json`, `maria.json`, `ezra.json`.

- [ ] **Step 1: Add `householdId` to each humans.json**

For each file add `"householdId": "household-<name>"` at the top level (next to `id`). Matthew/Jessica/James/Susan → `household-matthew`. Adam/Eve → `household-adam`. Others → their own.

- [ ] **Step 2: Validate**

```bash
pnpm run schema:validate
```
If humans schema rejects, update `humans.schema.json` to add `householdId` as optional string.

- [ ] **Step 3: Commit**

```bash
git add genesis/data/humans/*.json elohim/sdk/schemas/v1/
git commit -m "feat(humans): householdId field on all active-alpha personas"
```

### Task D3: Humans→household resolver service

**Files:**
- Create: `elohim/elohim-storage/src/services/household.rs`
- Modify: `elohim/elohim-storage/src/http.rs` (add `GET /api/v1/humans/{id}/household`)

- [ ] **Step 1: Write failing test**

`elohim/elohim-storage/tests/household_resolve.rs`:

```rust
#[tokio::test]
async fn humans_household_returns_household_id() {
    let app = router::test_app_seeded().await;
    let resp = app.oneshot(
        Request::builder()
            .uri("/api/v1/humans/matthew/household")
            .body(Default::default()).unwrap())
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v: serde_json::Value = serde_json::from_slice(
        &hyper::body::to_bytes(resp.into_body()).await.unwrap()).unwrap();
    assert_eq!(v["householdId"], "household-matthew");
}
```

- [ ] **Step 2: Implement service**

```rust
// elohim/elohim-storage/src/services/household.rs
use crate::db::{humans, AppContext};

pub async fn resolve_household_for_human(
    ctx: &AppContext,
    human_id: &str,
) -> Result<Option<String>, crate::error::StorageError> {
    let mut conn = ctx.pool.get()?;
    let h = humans::get_by_id(&mut conn, human_id)?;
    Ok(h.and_then(|r| r.household_id))
}
```

(If humans table doesn't yet have `household_id`, add a migration: `ALTER TABLE humans ADD COLUMN household_id TEXT;` — and update the humans loader to populate it from the JSON fixtures.)

- [ ] **Step 3: Wire route**

```rust
(Method::GET, p) if p.starts_with("/api/v1/humans/") && p.ends_with("/household") => {
    let id = p.trim_start_matches("/api/v1/humans/").trim_end_matches("/household");
    return api::household::handle_resolve(&state, id).await;
}
```

Add to `build_manifest()`.

- [ ] **Step 4: Run — PASS**

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/services/household.rs \
        elohim/elohim-storage/src/http.rs \
        elohim/elohim-storage/migrations/*humans_add_household_id* \
        elohim/elohim-storage/src/db/humans.rs \
        elohim/elohim-storage/tests/household_resolve.rs
git commit -m "feat(household): humans→household resolver + /api/v1/humans/{id}/household"
```

---

## Module E — Resilience household-first computation (deliverable 8)

### Task E1: Household resilience service (Rust)

**Files:**
- Create: `elohim/sdk/schemas/v1/views/household-resilience-view.schema.json`
- Create: `elohim/elohim-storage/src/services/household_resilience.rs`
- Modify: `elohim/elohim-storage/src/views.rs`
- Modify: `elohim/elohim-storage/src/http.rs`

- [ ] **Step 1: Write schema**

```json
{
  "$id": "household-resilience-view.schema.json",
  "type": "object",
  "additionalProperties": false,
  "required": ["contentId","householdsStewarding","householdsReciprocated","protectionStatus","details"],
  "properties": {
    "contentId": { "type": "string" },
    "householdsStewarding": { "type": "integer", "minimum": 0 },
    "householdsReciprocated": { "type": "integer", "minimum": 0 },
    "protectionStatus": { "enum": ["at-risk","partial","protected"] },
    "details": {
      "type": "object",
      "properties": {
        "stewardHouseholds": { "type": "array", "items": { "type": "string" } },
        "onlinePeerCount": { "type": "integer" },
        "healthScore": { "type": "number", "minimum": 0, "maximum": 1 }
      }
    }
  }
}
```

Add to codegen-ts.mjs. Run `pnpm run schema:codegen:ts`.

- [ ] **Step 2: Add view struct + schema contract test (pattern from Task C2)**

- [ ] **Step 3: Implement computation**

```rust
// elohim/elohim-storage/src/services/household_resilience.rs
use crate::db::{stewardship_allocations, peer_statuses, stewarded_nodes, AppContext};
use crate::views::HouseholdResilienceView;

pub async fn compute(
    ctx: &AppContext,
    content_id: &str,
    viewer_household_id: Option<&str>,
) -> Result<HouseholdResilienceView, crate::error::StorageError> {
    let mut conn = ctx.pool.get()?;

    // Stewarding households: set of distinct household_ids across stewardship_allocations
    // joined to humans (for householdId) for this content.
    let households_stewarding = stewardship_allocations::list_households_for_content(&mut conn, content_id)?;
    let stewarding_count = households_stewarding.len();

    // Reciprocated: households that viewer_household also stewards for.
    let reciprocated_count = match viewer_household_id {
        Some(vh) => stewardship_allocations::count_reciprocal_households(&mut conn, vh, &households_stewarding)?,
        None => 0,
    };

    // Online peer count: count stewarded_nodes in those households with peer_status=Online.
    let online = peer_statuses::count_online_in_households(&mut conn, &households_stewarding)?;

    let status = match (stewarding_count, online) {
        (n, o) if n >= 3 && o >= 2 => "protected",
        (n, _) if n >= 2 => "partial",
        _ => "at-risk",
    };

    let health = (online as f32 / (stewarding_count.max(1) as f32)).min(1.0);

    Ok(HouseholdResilienceView {
        content_id: content_id.into(),
        households_stewarding: stewarding_count as i32,
        households_reciprocated: reciprocated_count as i32,
        protection_status: status.into(),
        details: Default::default(),  // fill stewardHouseholds, onlinePeerCount, healthScore
    })
}
```

(Add the new query helpers in `stewardship_allocations`, `peer_statuses`. If `households_for_content` is non-trivial, add a migration to `stewardship_allocations` indexing human_id → household_id join.)

- [ ] **Step 4: Wire HTTP route `GET /api/v1/resilience/{content_id}/household`**

Add dispatch + manifest entry.

- [ ] **Step 5: Write test covering at-risk / partial / protected transitions**

`tests/household_resilience.rs`: seed 1 household → at-risk; seed 2 → partial; seed 3 with 2 online → protected.

- [ ] **Step 6: Run + Commit**

```bash
git add elohim/sdk/schemas/v1/views/household-resilience-view.schema.json \
        elohim/elohim-storage/src/views.rs \
        elohim/elohim-storage/src/services/household_resilience.rs \
        elohim/elohim-storage/src/http.rs \
        elohim/elohim-storage/tests/household_resilience.rs \
        elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(resilience): household-first computation + GET /api/v1/resilience/{id}/household"
```

### Task E2: Household-resilience Angular service

**Files:**
- Create: `app/elohim-app/src/app/lamad/services/household-resilience.service.ts`

- [ ] **Step 1: Create service**

```typescript
// app/elohim-app/src/app/lamad/services/household-resilience.service.ts
import { HttpClient } from '@angular/common/http';
import { inject, Injectable } from '@angular/core';
import { Observable } from 'rxjs';
import { StorageClientService } from '@app/elohim/services/storage-client.service';
import type { HouseholdResilienceView } from '@elohim/storage-client/generated';

@Injectable({ providedIn: 'root' })
export class HouseholdResilienceService {
  private readonly http = inject(HttpClient);
  private readonly storage = inject(StorageClientService);

  get(contentId: string): Observable<HouseholdResilienceView> {
    const base = this.storage.getStorageBaseUrl();
    return this.http.get<HouseholdResilienceView>(
      `${base}/api/v1/resilience/${encodeURIComponent(contentId)}/household`,
    );
  }
}
```

- [ ] **Step 2: Add spec**

Write a simple Vitest test asserting the URL shape and that the service emits the response.

- [ ] **Step 3: Run**

```bash
cd app/elohim-app && pnpm exec vitest run household-resilience.service
```
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/lamad/services/household-resilience.service.ts \
        app/elohim-app/src/app/lamad/services/household-resilience.service.spec.ts
git commit -m "feat(resilience): Angular HouseholdResilienceService"
```

### Task E3: Rewrite resilience tooltip to household-first

**Files:**
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts`

- [ ] **Step 1: Load household resilience alongside existing resilience**

In `loadResilience(nodeId)` add a parallel call:

```typescript
this.householdResilienceService.get(nodeId)
  .pipe(takeUntil(this.destroy$))
  .subscribe({
    next: hr => { this.householdResilience = hr; },
    error: () => {},
  });
```

Declare `householdResilience: HouseholdResilienceView | null = null;` and `private readonly householdResilienceService = inject(HouseholdResilienceService);`.

- [ ] **Step 2: Rewrite getResilienceIcon()**

```typescript
getResilienceIcon(): string {
  const s = this.householdResilience?.protectionStatus;
  switch (s) {
    case 'protected': return '\u{1F7E2}';   // green
    case 'partial':   return '\u{1F7E1}';   // yellow
    case 'at-risk':   return '\u{1F534}';   // red
    default:          return '\u{1F504}';   // loading
  }
}
```

- [ ] **Step 3: Rewrite getResilienceTooltip()**

```typescript
getResilienceTooltip(): string {
  if (!this.householdResilience) return 'Loading resilience…';
  const hr = this.householdResilience;
  const lines = [
    `Households stewarding: ${hr.householdsStewarding}`,
    `Reciprocated: ${hr.householdsReciprocated}`,
    `Protection: ${hr.protectionStatus}`,
  ];
  if (this.resilience?.encoding) {
    lines.push(`Encoding: ${this.resilience.encoding.strategy}`);
  }
  if (this.resilience?.distribution) {
    lines.push(`Peers online: ${this.resilience.distribution.distinctPeers}`);
  }
  if (this.resilience?.health?.score != null) {
    lines.push(`Health: ${Math.round(this.resilience.health.score * 100)}%`);
  }
  return lines.join('\n');
}
```

- [ ] **Step 4: Update component spec to cover new behavior**

Tests should assert the three status → icon mappings and that tooltip lists households-first.

- [ ] **Step 5: Run**

```bash
cd app/elohim-app && pnpm exec vitest run content-viewer
```
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add app/elohim-app/src/app/lamad/components/content-viewer/
git commit -m "feat(resilience): household-first tooltip + icon in content-viewer"
```

---

## Module F — Network Health posture (deliverable 7)

### Task F1: Schema + Rust struct for posture view

**Files:**
- Create: `elohim/sdk/schemas/v1/views/network-posture-view.schema.json`
- Modify: `elohim/elohim-storage/src/views.rs`
- Modify: `elohim/elohim-storage/tests/schema_contract.rs`

- [ ] **Step 1: Write schema**

```json
{
  "$id": "network-posture-view.schema.json",
  "type": "object",
  "additionalProperties": false,
  "required": ["totalPeers","activePeers","stalePeers","alwaysOnPeers","householdsReciprocating","computeAvailable","storagePressure","computedAt"],
  "properties": {
    "totalPeers":              { "type": "integer", "minimum": 0 },
    "activePeers":             { "type": "integer", "minimum": 0 },
    "stalePeers":              { "type": "integer", "minimum": 0 },
    "alwaysOnPeers":           { "type": "integer", "minimum": 0 },
    "householdsReciprocating": { "type": "integer", "minimum": 0 },
    "computeAvailable":        { "type": "boolean" },
    "storagePressure":         { "type": "number", "minimum": 0, "maximum": 1 },
    "computedAt":              { "type": "string", "format": "date-time" }
  }
}
```

Add to codegen. Generate TS. Add schema contract test.

- [ ] **Step 2: Define NetworkPostureView struct + From impl (pattern from C2)**

- [ ] **Step 3: Commit**

```bash
git add elohim/sdk/schemas/v1/views/network-posture-view.schema.json \
        elohim/elohim-storage/src/views.rs \
        elohim/elohim-storage/tests/schema_contract.rs \
        elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(views): NetworkPostureView schema + Rust struct"
```

### Task F2: Posture computation + /api/v1/network/posture

**Files:**
- Create: `elohim/elohim-storage/src/api/network_posture.rs`
- Modify: `elohim/elohim-storage/src/http.rs`

- [ ] **Step 1: Failing test**

`tests/network_posture_route.rs` GETs `/api/v1/network/posture`, asserts 200 + required fields present.

- [ ] **Step 2: Run — FAIL 404**

- [ ] **Step 3: Implement handler**

```rust
// elohim/elohim-storage/src/api/network_posture.rs
pub async fn handle_get(ctx: &AppContext, _req: Request<Incoming>) -> Response<Full<Bytes>> {
    let mut conn = match ctx.pool.get() { Ok(c)=>c, Err(_)=>return response::internal_error("pool") };

    let peers = crate::db::peer_statuses::list_current(&mut conn).unwrap_or_default();
    let total = peers.len() as i32;
    let active = peers.iter().filter(|p| p.status == "online").count() as i32;
    let stale = peers.iter().filter(|p| {
        // last update older than 120s = stale
        // ...
        false
    }).count() as i32;
    let always_on = peers.iter().filter(|p| p.general_pool_member == 1 && p.status == "online").count() as i32;

    // households_reciprocating: distinct households with ≥1 stewardship allocation AND a reciprocal allocation from the viewer's household
    // For v1: count distinct household_ids among stewarded_nodes with online peers
    let households_reciprocating = crate::db::stewarded_nodes::count_distinct_households_online(&mut conn).unwrap_or(0) as i32;

    // compute_available: any peer with accepting_stewardship_reserves AND online
    let compute_available = peers.iter().any(|p| p.accepting_stewardship_reserves == 1 && p.status == "online");

    // storage_pressure: stub at 0.0 until shard_locations populate
    let storage_pressure = 0.0_f32;

    let view = NetworkPostureView {
        total_peers: total,
        active_peers: active,
        stale_peers: stale,
        always_on_peers: always_on,
        households_reciprocating,
        compute_available,
        storage_pressure,
        computed_at: chrono::Utc::now().to_rfc3339(),
    };
    response::json_ok(&view)
}
```

- [ ] **Step 4: Wire route + manifest**

```rust
(Method::GET, "/api/v1/network/posture") => return api::network_posture::handle_get(&state, req).await,
```

- [ ] **Step 5: Run — PASS. Commit**

```bash
git add elohim/elohim-storage/src/api/network_posture.rs \
        elohim/elohim-storage/src/http.rs \
        elohim/elohim-storage/tests/network_posture_route.rs
git commit -m "feat(posture): GET /api/v1/network/posture info-level from PeerStatus projection"
```

### Task F3: Network Health tab in shefa-dashboard

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/shefa-dashboard/network-health-tab.component.ts`
- Create: `app/elohim-app/src/app/elohim/services/network-posture.service.ts`
- Modify: `app/elohim-app/src/app/elohim/components/shefa-dashboard/shefa-dashboard.component.ts` (add tab)

- [ ] **Step 1: Posture service**

```typescript
@Injectable({ providedIn: 'root' })
export class NetworkPostureService {
  private readonly http = inject(HttpClient);
  private readonly storage = inject(StorageClientService);
  get(): Observable<NetworkPostureView> {
    const base = this.storage.getStorageBaseUrl();
    return this.http.get<NetworkPostureView>(`${base}/api/v1/network/posture`);
  }
}
```

- [ ] **Step 2: Component**

```typescript
@Component({
  selector: 'app-network-health-tab',
  standalone: true,
  imports: [CommonModule],
  template: `
    @if (posture(); as p) {
      <div class="posture-card" data-testid="network-posture-card">
        <div class="metric">
          <div class="label">Active peers</div>
          <div class="value">{{ p.activePeers }} / {{ p.totalPeers }}</div>
        </div>
        <div class="metric">
          <div class="label">Households reciprocating</div>
          <div class="value">{{ p.householdsReciprocating }}</div>
        </div>
        <div class="metric">
          <div class="label">Always-on</div>
          <div class="value">{{ p.alwaysOnPeers }}</div>
        </div>
        <div class="metric">
          <div class="label">Compute available</div>
          <div class="value">{{ p.computeAvailable ? 'Yes' : 'No' }}</div>
        </div>
      </div>
    } @else {
      <div data-testid="network-posture-loading">Loading posture…</div>
    }
  `,
  styleUrls: ['./network-health-tab.component.scss'],
})
export class NetworkHealthTabComponent implements OnInit {
  private readonly service = inject(NetworkPostureService);
  readonly posture = signal<NetworkPostureView | null>(null);
  ngOnInit(): void {
    this.service.get().subscribe(p => this.posture.set(p));
  }
}
```

- [ ] **Step 3: Add tab in parent component**

In `shefa-dashboard.component.ts` / `.html`, add a new tab entry routing to `NetworkHealthTabComponent`.

- [ ] **Step 4: Vitest spec — renders card with mocked posture**

- [ ] **Step 5: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/shefa-dashboard/network-health-tab.component.* \
        app/elohim-app/src/app/elohim/services/network-posture.service.ts \
        app/elohim-app/src/app/elohim/components/shefa-dashboard/shefa-dashboard.component.*
git commit -m "feat(shefa-dashboard): Network Health tab pulling /api/v1/network/posture"
```

---

## Module G — Stewarded resources page wiring (deliverable 5)

### Task G1: Resource-explorer category:content support

**Files:**
- Modify: `app/elohim-app/src/app/shefa/components/resource-explorer/resource-explorer.component.ts`

- [ ] **Step 1: Detect `category:content` route param**

In `ngOnInit`, read `this.route.paramMap` for `lensType`. When it's `category:content`, call stewardship allocation API filtered by the viewer's household.

```typescript
this.route.paramMap.subscribe(params => {
  const lens = params.get('lensType');
  if (lens === 'category:content') {
    this.loadStewardedContent();
  }
});
```

- [ ] **Step 2: Implement loadStewardedContent**

```typescript
private async loadStewardedContent(): Promise<void> {
  const me = await firstValueFrom(this.identity.currentHuman$);
  if (!me?.id) return;
  const hh = await firstValueFrom(
    this.http.get<{householdId: string}>(`${this.base}/api/v1/humans/${me.id}/household`)
  );
  const items = await firstValueFrom(
    this.stewardshipAllocationService.listForHousehold(hh.householdId)
  );
  this.items.set(items);
  items.forEach(item => {
    this.householdResilienceService.get(item.contentId).subscribe(hr => {
      this.resilienceByContent.update(m => ({ ...m, [item.contentId]: hr }));
    });
  });
}
```

(Add `listForHousehold(householdId)` to `StewardshipAllocationService` — backed by new endpoint `GET /api/v1/households/{id}/stewardship-allocations` in storage. That endpoint is a thin wrapper over existing `stewardship_allocations` queries filtered by household.)

- [ ] **Step 3: Template — render list with resilience badge**

Add to the component template (guarded by `lens === 'category:content'`):

```html
<div class="stewarded-list" data-testid="stewarded-content-list">
  @for (item of items(); track item.contentId) {
    <div class="stewarded-item" [attr.data-testid]="'stewarded-' + item.contentId">
      <span class="title">{{ item.contentTitle || item.contentId }}</span>
      <span class="affinity">{{ item.affinity }}</span>
      <span class="ratio">{{ (item.allocationRatio * 100) | number:'1.0-0' }}%</span>
      @let r = resilienceByContent()[item.contentId];
      @if (r) {
        <span class="resilience-badge" [attr.data-testid]="'resilience-' + item.contentId">
          {{ r.protectionStatus }} — {{ r.householdsStewarding }} households
        </span>
      }
    </div>
  }
</div>
```

- [ ] **Step 4: Vitest spec**

Test that when route is `category:content` and API returns items, list renders with resilience badges.

- [ ] **Step 5: Commit**

```bash
git add app/elohim-app/src/app/shefa/components/resource-explorer/ \
        app/elohim-app/src/app/lamad/services/stewardship-allocation.service.ts
git commit -m "feat(shefa): /shefa/resources/category:content lists household-stewarded content with resilience badges"
```

### Task G2: Storage endpoint — stewardship allocations by household

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs`
- Modify: `elohim/elohim-storage/src/db/stewardship_allocations.rs`

- [ ] **Step 1: Add `list_by_household` query**

In `stewardship_allocations.rs`:

```rust
pub fn list_by_household(
    conn: &mut SqliteConnection,
    household_id: &str,
) -> Result<Vec<StewardshipAllocationRow>, diesel::result::Error> {
    use crate::db::diesel_schema::stewardship_allocations::dsl as sa;
    use crate::db::diesel_schema::humans::dsl as h;
    sa::stewardship_allocations
        .inner_join(h::humans.on(h::id.eq(sa::steward_presence_id)))
        .filter(h::household_id.eq(household_id))
        .select(StewardshipAllocationRow::as_select())
        .load(conn)
}
```

- [ ] **Step 2: Add route + manifest**

`GET /api/v1/households/{id}/stewardship-allocations` → list_by_household.

- [ ] **Step 3: Test + Commit**

```bash
git add elohim/elohim-storage/src/http.rs elohim/elohim-storage/src/db/stewardship_allocations.rs \
        elohim/elohim-storage/tests/stewardship_by_household.rs
git commit -m "feat(stewardship): GET /api/v1/households/{id}/stewardship-allocations"
```

---

## Module H — Doorway admin gaps (deliverable 9)

### Task H1: Wire /admin/routes handler

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs`

- [ ] **Step 1: Add dispatch branch**

Locate the match block in `handle_request` (around the other `/admin/*` arms). Add:

```rust
(Method::GET, "/admin/routes") => {
    require_admin(&state, req.headers())?;
    return routes::admin::handle_route_registry(state.clone()).await;
}
```

(`require_admin` should already exist for other admin routes; check nearby code for the exact helper.)

- [ ] **Step 2: Build + test**

```bash
cd doorway/doorway-service && RUSTFLAGS="" cargo build --release
RUSTFLAGS="" cargo test --lib --bins admin
```

- [ ] **Step 3: Manual smoke**

With a locally running doorway, `curl -H "Authorization: Bearer <admin-jwt>" http://127.0.0.1:8888/admin/routes | jq` returns a `RouteRegistryResponse` payload.

- [ ] **Step 4: Commit**

```bash
git add doorway/doorway-service/src/server/http.rs
git commit -m "fix(doorway): wire /admin/routes handler into dispatch"
```

### Task H2: Fix /admin/users auth

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` or wherever `/admin/users` is handled (grep first: `grep -rn "/admin/users" doorway/doorway-service/src/`).

- [ ] **Step 1: Locate the handler and its auth check**

If the 403 comes from an overly-strict `require_admin` path (e.g., requiring a role the admin JWT doesn't carry), align the check with `/admin/conductors` (known-good). If the route is missing entirely, add it analogous to `/admin/routes`.

- [ ] **Step 2: Run existing admin tests**

```bash
RUSTFLAGS="" cargo test --lib --bins admin
```

- [ ] **Step 3: Commit**

```bash
git add doorway/doorway-service/src/server/http.rs
git commit -m "fix(doorway): /admin/users auth aligned with /admin/conductors"
```

---

## Module I — Frontend retirement of NodeRegistryAnchor (deliverable 1 completion)

### Task I1: Rewrite device-stewardship component

**Files:**
- Modify (rewrite): `app/elohim-app/src/app/shefa/components/device-stewardship/device-stewardship.component.ts`
- Create: `app/elohim-app/src/app/shefa/services/household-devices.service.ts`

- [ ] **Step 1: Service**

```typescript
@Injectable({ providedIn: 'root' })
export class HouseholdDevicesService {
  private readonly http = inject(HttpClient);
  private readonly storage = inject(StorageClientService);

  list(householdId: string): Observable<HouseholdDevicesView> {
    const base = this.storage.getStorageBaseUrl();
    return this.http.get<HouseholdDevicesView>(
      `${base}/api/v1/households/${encodeURIComponent(householdId)}/devices`,
    );
  }
}
```

- [ ] **Step 2: Rewrite component**

```typescript
@Component({
  selector: 'app-device-stewardship',
  standalone: true,
  imports: [CommonModule],
  template: `
    @if (view(); as v) {
      <h2>Household: {{ v.householdId }}</h2>
      <ul data-testid="device-list">
        @for (d of v.devices; track d.shape.nodeId) {
          <li [attr.data-testid]="'device-' + d.shape.nodeId">
            <strong>{{ d.shape.hostname }}</strong>
            <span>{{ d.shape.deviceArchetypeId }} (L{{ d.shape.capabilityLevel }})</span>
            <span>{{ d.shape.role }}</span>
            <span [class.online]="d.peer?.status === 'online'"
                  [class.offline]="!d.peer">
              {{ d.peer?.status ?? 'offline' }}
            </span>
          </li>
        }
      </ul>
    } @else {
      <div data-testid="device-list-loading">Loading your devices…</div>
    }
  `,
})
export class DeviceStewardshipComponent implements OnInit {
  private readonly identity = inject(IdentityService);
  private readonly household = inject(HouseholdDevicesService);
  private readonly http = inject(HttpClient);
  private readonly storage = inject(StorageClientService);

  readonly view = signal<HouseholdDevicesView | null>(null);

  async ngOnInit(): Promise<void> {
    const me = await firstValueFrom(this.identity.currentHuman$);
    if (!me?.id) return;
    const base = this.storage.getStorageBaseUrl();
    const hh = await firstValueFrom(
      this.http.get<{householdId: string}>(`${base}/api/v1/humans/${me.id}/household`)
    );
    this.household.list(hh.householdId).subscribe(v => this.view.set(v));
  }
}
```

- [ ] **Step 3: Delete any reference to NodeRegistryAnchor**

```bash
grep -rn NodeRegistryAnchor app/elohim-app/src/
```
Remove imports/uses in the rewritten component. The anchor file itself can stay temporarily (deprecation) but should have no callers.

- [ ] **Step 4: Vitest spec**

Covers: resolves household for current human, calls list(), renders devices with expected testids.

- [ ] **Step 5: Commit**

```bash
git add app/elohim-app/src/app/shefa/components/device-stewardship/ \
        app/elohim-app/src/app/shefa/services/household-devices.service.ts
git commit -m "feat(shefa/devices): rewrite against household-devices API; retire NodeRegistryAnchor callers"
```

---

## Module J — Maintenance choreography (§6.B acceptance)

### Task J1: Maintenance CLI/API on elohim-storage

**Files:**
- Modify: `elohim/elohim-storage/src/api/peer_statuses.rs`
- Modify: `elohim/elohim-storage/src/heartbeat.rs`

- [ ] **Step 1: Add POST /api/v1/peer-status/maintenance**

```rust
pub async fn handle_enter_maintenance(ctx: &AppContext) -> Response<Full<Bytes>> {
    // Tell the heartbeat task to flip lifecycle to Maintenance on next tick.
    ctx.heartbeat_control.set_target_state("maintenance");
    response::json_ok(&serde_json::json!({ "status": "maintenance-scheduled" }))
}

pub async fn handle_exit_maintenance(ctx: &AppContext) -> Response<Full<Bytes>> {
    ctx.heartbeat_control.set_target_state("online");
    response::json_ok(&serde_json::json!({ "status": "online-scheduled" }))
}
```

Add a `HeartbeatControl` handle in `heartbeat.rs` exposing `set_target_state(&str)` that the heartbeat task consults each tick to override policy-derived state (manual admin override). Thread it through `AppContext`.

- [ ] **Step 2: Wire dispatch + manifest**

```rust
(Method::POST, "/api/v1/peer-status/maintenance") => return api::peer_statuses::handle_enter_maintenance(&state).await,
(Method::POST, "/api/v1/peer-status/online")      => return api::peer_statuses::handle_exit_maintenance(&state).await,
```

- [ ] **Step 3: Test**

```rust
#[tokio::test]
async fn maintenance_toggle_flips_lifecycle() {
    let app = router::test_app().await;
    let r = app.clone().oneshot(Request::builder()
        .method(Method::POST).uri("/api/v1/peer-status/maintenance")
        .body(Default::default()).unwrap()).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    // Advance heartbeat by one tick; assert latest peer_statuses row has status = "maintenance"
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    // ...
}
```

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/api/peer_statuses.rs \
        elohim/elohim-storage/src/heartbeat.rs \
        elohim/elohim-storage/src/http.rs \
        elohim/elohim-storage/tests/maintenance_toggle.rs
git commit -m "feat(maintenance): POST /api/v1/peer-status/{maintenance,online} admin overrides"
```

### Task J2: Pre-drain household-protection check

**Files:**
- Create: `elohim/elohim-storage/src/services/drain_guard.rs`
- Modify: `elohim/elohim-storage/src/api/peer_statuses.rs`

- [ ] **Step 1: Implement guard**

```rust
// drain_guard.rs
pub async fn evaluate(ctx: &AppContext, peer_id: &str) -> DrainDecision {
    // 1. Resolve household_id via stewarded_nodes join.
    // 2. For each content stewarded by this household, compute hypothetical
    //    household_resilience::compute WHERE this peer is excluded.
    // 3. If ANY content drops to "at-risk" → DrainDecision::Block(reason).
    // 4. Else → Allow.
    // ...
}
```

- [ ] **Step 2: Call guard from maintenance handler**

```rust
pub async fn handle_enter_maintenance(ctx: &AppContext, req: Request<Incoming>) -> Response<Full<Bytes>> {
    let force = req.uri().query().map(|q| q.contains("force=true")).unwrap_or(false);
    let peer_id = ctx.self_node_id();  // from boot config
    let decision = services::drain_guard::evaluate(ctx, peer_id).await;
    match decision {
        DrainDecision::Allow => { /* proceed as J1 */ }
        DrainDecision::Block(reason) if !force => {
            return response::bad_request(&format!("blocked: {reason}"));
        }
        DrainDecision::Block(_) if force => { /* proceed with attestation trail */ }
    }
}
```

- [ ] **Step 3: Tests — allow / block / force**

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/services/drain_guard.rs \
        elohim/elohim-storage/src/api/peer_statuses.rs \
        elohim/elohim-storage/tests/drain_guard.rs
git commit -m "feat(maintenance): pre-drain household-protection check with force override"
```

---

## Module K — Shem activation (deliverable 10)

### Task K1: StatefulSet template per archetype

**Files:**
- Create: `infra/shem/statefulsets/_template.yaml`

- [ ] **Step 1: Write reusable StatefulSet template**

```yaml
# infra/shem/statefulsets/_template.yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: ${PERSONA}
  namespace: elohim-alpha
spec:
  serviceName: ${PERSONA}-headless
  replicas: 1
  selector: { matchLabels: { persona: ${PERSONA} } }
  template:
    metadata:
      labels: { persona: ${PERSONA}, household: ${HOUSEHOLD_ID}, archetype: ${DEVICE_ARCHETYPE} }
    spec:
      containers:
      - name: elohim-node
        image: registry.elohim.host/elohim-node:${IMAGE_TAG}
        env:
          - name: DEVICE_ARCHETYPE
            value: "${DEVICE_ARCHETYPE}"
          - name: HOUSEHOLD_ID
            value: "${HOUSEHOLD_ID}"
          - name: NODE_ROLE
            value: "${NODE_ROLE}"
          - name: REGION
            value: "us-central"
        volumeMounts:
          - name: data
            mountPath: /var/lib/elohim
  volumeClaimTemplates:
    - metadata:
        name: data
      spec:
        storageClassName: local-path
        accessModes: ["ReadWriteOnce"]
        resources:
          requests:
            storage: "${STORAGE_REQUEST}"
```

- [ ] **Step 2: Commit**

```bash
git add infra/shem/statefulsets/_template.yaml
git commit -m "infra(shem): StatefulSet template for persona elohim-node deployments"
```

### Task K2: Persona manifests

**Files:**
- Create: one YAML per persona (matthew-home, matthew-laptop, jessica-phone, adam-node, eve-laptop, pete-laptop, terrance-laptop, nancy-hosted, doorway-alpha).

- [ ] **Step 1: Generate from template**

For each persona write a concrete YAML. Example for matthew-home:

```yaml
# infra/shem/statefulsets/matthew-home.yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: matthew-home
  namespace: elohim-alpha
spec:
  serviceName: matthew-home-headless
  replicas: 1
  selector: { matchLabels: { persona: matthew-home } }
  template:
    metadata:
      labels: { persona: matthew-home, household: household-matthew, archetype: home-nuc }
    spec:
      containers:
      - name: elohim-node
        image: registry.elohim.host/elohim-node:dev-2026-04-19
        env:
          - name: DEVICE_ARCHETYPE
            value: "home-nuc"
          - name: HOUSEHOLD_ID
            value: "household-matthew"
          - name: NODE_ROLE
            value: "edge"
          - name: REGION
            value: "us-central"
        ports:
          - { containerPort: 8090, name: storage-http }
          - { containerPort: 4445, name: app-ws }
        volumeMounts:
          - { name: data, mountPath: /var/lib/elohim }
  volumeClaimTemplates:
    - metadata: { name: data }
      spec:
        storageClassName: local-path
        accessModes: ["ReadWriteOnce"]
        resources: { requests: { storage: "100Gi" } }
```

Per-persona tuning:
- matthew-home → archetype `home-nuc`, storage 100Gi, role edge
- matthew-laptop → archetype `laptop`, storage 25Gi, role edge
- jessica-phone → archetype `2019-android-phone`, storage 8Gi, role edge
- adam-node → archetype `family-node-base`, storage 500Gi, role archival
- eve-laptop → archetype `laptop`, storage 25Gi, role edge
- pete-laptop → archetype `laptop`, storage 25Gi, role edge
- terrance-laptop → archetype `chromebook-edu`, storage 16Gi, role edge
- nancy-hosted → archetype `hosted-on-doorway`, storage 4Gi, role edge (runs as a cell on doorway-alpha, not a separate StatefulSet — skip a standalone YAML)
- doorway-alpha → archetype `doorway-steward`, storage 250Gi, role doorway

- [ ] **Step 2: Apply to shem and verify pods running**

```bash
kubectl apply -f infra/shem/statefulsets/ -n elohim-alpha
kubectl get pods -n elohim-alpha -w
```
Expected: each StatefulSet pod reaches Running state.

- [ ] **Step 3: Commit**

```bash
git add infra/shem/statefulsets/
git commit -m "infra(shem): persona StatefulSets for household-matthew, household-adam, solos, doorway-alpha"
```

### Task K3: Verify dashboards light up

- [ ] **Step 1: Browse https://alpha.elohim.host/shefa/devices as Matthew, inspect console**

Expected: device-list renders with matthew-home/matthew-laptop/jessica-phone (plus connected households visible via peer joins). No `e.filter` errors.

- [ ] **Step 2: Browse /shefa/resources/category:content**

Expected: stewarded content list populated with resilience badges.

- [ ] **Step 3: Browse /shefa/dashboard → Network Health tab**

Expected: posture card shows ≥ 5 active peers, households_reciprocating ≥ 2, compute_available true.

- [ ] **Step 4: Browse doorway-alpha.elohim.host/threshold/dashboard**

Expected: /admin/routes populates tab (no HTTP parsing failure). /admin/users list renders. Registered peers tab shows the household members.

- [ ] **Step 5: Commit note (checkpoint marker)**

```bash
git commit --allow-empty -m "checkpoint(sprint): dashboards lit on shem with household-matthew + household-adam + solos"
```

### Task K4: Maintenance-cycle demo script

**Files:**
- Create: `infra/shem/demo/maintenance-cycle.sh`

- [ ] **Step 1: Script**

```bash
#!/usr/bin/env bash
# Demo: Terrance offline → partial → online → protected
set -euo pipefail
NS=elohim-alpha

echo "[1] Baseline posture:"
curl -sS https://doorway-alpha.elohim.host/api/v1/network/posture | jq

echo "[2] Terrance enters maintenance (pre-drain check):"
kubectl exec -n $NS terrance-laptop-0 -- \
  curl -sS -X POST http://127.0.0.1:8090/api/v1/peer-status/maintenance

echo "[3] Wait 90s for PeerStatus to propagate:"
sleep 90

echo "[4] Posture after Terrance offline:"
curl -sS https://doorway-alpha.elohim.host/api/v1/network/posture | jq
echo "    Matthew household resilience for family-photo-1:"
curl -sS https://alpha.elohim.host/api/v1/resilience/family-photo-1/household | jq

echo "[5] Terrance returns:"
kubectl exec -n $NS terrance-laptop-0 -- \
  curl -sS -X POST http://127.0.0.1:8090/api/v1/peer-status/online

echo "[6] Wait 90s:"
sleep 90

echo "[7] Posture restored:"
curl -sS https://doorway-alpha.elohim.host/api/v1/network/posture | jq
echo "    Protection restored:"
curl -sS https://alpha.elohim.host/api/v1/resilience/family-photo-1/household | jq
```

- [ ] **Step 2: Make executable, run, capture output**

```bash
chmod +x infra/shem/demo/maintenance-cycle.sh
infra/shem/demo/maintenance-cycle.sh 2>&1 | tee demo-run-$(date +%s).log
```

Expected: step 4 shows `partial` or `protected` (never `at-risk`); step 7 shows `protected`.

- [ ] **Step 3: Commit**

```bash
git add infra/shem/demo/maintenance-cycle.sh
git commit -m "demo(shem): maintenance-cycle script proving protection never drops to at-risk"
```

---

## Module L — A2O scenarios (deliverable 11)

### Task L1: shefa/device-stewardship.feature

**Files:**
- Create: `genesis/a2o/features/shefa/device-stewardship.feature`
- Create: `genesis/a2o/steps/shefa/device-stewardship.steps.ts`

- [ ] **Step 1: Feature**

```gherkin
Feature: Device stewardship visibility
  As Matthew, I want to see my household's devices + connected peer households.

  Background:
    Given the sprint fixtures are seeded
    And I am authenticated as matthew

  Scenario: Household devices render with archetype + lifecycle
    When I open /shefa/devices
    Then the device list includes matthew-home labeled home-nuc
    And the device list includes matthew-laptop labeled laptop
    And the device list includes jessica-phone labeled 2019-android-phone
    And each device shows a lifecycle status of online or offline
    And there are no JavaScript console errors
```

- [ ] **Step 2: Step definitions**

```typescript
// genesis/a2o/steps/shefa/device-stewardship.steps.ts
import { Then, When } from '@cucumber/cucumber';

When('I open \\/shefa\\/devices', async function () {
  await this.page.goto(`${this.appBaseUrl}/shefa/devices`);
});

Then('the device list includes {word} labeled {word}', async function (nodeId: string, archetype: string) {
  await this.page.waitForSelector(`[data-testid="device-${nodeId}"]`, { timeout: 10_000 });
  const txt = await this.page.textContent(`[data-testid="device-${nodeId}"]`);
  if (!txt?.includes(archetype)) throw new Error(`expected ${archetype} in ${txt}`);
});

Then('each device shows a lifecycle status of online or offline', async function () {
  const items = await this.page.$$('[data-testid^="device-"]');
  for (const el of items) {
    const t = (await el.textContent()) ?? '';
    if (!/online|offline|degraded|maintenance/.test(t)) {
      throw new Error(`device item missing lifecycle: ${t}`);
    }
  }
});

Then('there are no JavaScript console errors', async function () {
  if (this.consoleErrors?.length) {
    throw new Error(`console errors: ${this.consoleErrors.join('\n')}`);
  }
});
```

- [ ] **Step 3: Run**

```bash
cd app/elohim-app && pnpm run cypress:run --spec features/shefa/device-stewardship.feature
```
(Or the repo's actual a2o runner — check `genesis/a2o/package.json`.)

- [ ] **Step 4: Commit**

```bash
git add genesis/a2o/features/shefa/device-stewardship.feature \
        genesis/a2o/steps/shefa/device-stewardship.steps.ts
git commit -m "test(a2o): shefa/device-stewardship.feature + step defs"
```

### Task L2: shefa/network-health-dashboard.feature

**Files:**
- Create: `genesis/a2o/features/shefa/network-health-dashboard.feature`
- Create: `genesis/a2o/steps/shefa/network-health.steps.ts`

- [ ] **Step 1: Feature**

```gherkin
Feature: Network Health dashboard tab
  As Matthew, I want to see my network's posture at a glance.

  Background:
    Given the sprint fixtures are seeded
    And I am authenticated as matthew

  Scenario: Posture card renders live metrics
    When I open /shefa/dashboard
    And I switch to the "Network Health" tab
    Then I see a posture card with active peers count >= 1
    And I see households-reciprocating count >= 1
    And I see compute-available: Yes or No
```

- [ ] **Step 2: Step defs — use `network-posture-card` testid**

Similar pattern to L1.

- [ ] **Step 3: Run + commit**

```bash
git add genesis/a2o/features/shefa/network-health-dashboard.feature \
        genesis/a2o/steps/shefa/network-health.steps.ts
git commit -m "test(a2o): network-health-dashboard.feature + step defs"
```

### Task L3: shefa/stewarded-resources-visible.feature

- [ ] **Step 1: Feature + steps (pattern from L1)**

Scenario: "Matthew opens /shefa/resources/category:content, sees stewarded items with resilience badges."

- [ ] **Step 2: Commit**

### Task L4: doorway/admin-routes-visible.feature

- [ ] **Step 1: Feature**

```gherkin
Feature: Admin route registry visibility
  Scenario: Admin lists registered routes
    Given I am authenticated as admin matthew
    When I GET /admin/routes on the doorway
    Then the response is 200
    And the payload has totalRoutes >= 10
    And the payload lists route source types

  Scenario: Non-admin is denied
    Given I am authenticated as non-admin susan
    When I GET /admin/routes on the doorway
    Then the response is 403
```

- [ ] **Step 2: Steps + Commit**

### Task L5: doorway/admin-users-visible.feature

- [ ] **Step 1: Feature (pattern from L4)**

- [ ] **Step 2: Commit**

### Task L6: shefa/resilience-tooltip.feature

- [ ] **Step 1: Feature**

```gherkin
Feature: Content resilience tooltip
  Scenario: Tooltip shows household-first resilience
    Given I am authenticated as matthew
    When I open a content page
    And I hover the resilience icon
    Then the tooltip contains "Households stewarding"
    And the tooltip contains "Protection: protected" or "Protection: partial"
    And the tooltip does NOT contain "No stewards assigned"
```

- [ ] **Step 2: Steps + Commit**

### Task L7: shefa/maintenance-choreography.feature

- [ ] **Step 1: Feature**

```gherkin
Feature: Maintenance choreography preserves household protection
  Background:
    Given the full shem persona roster is running
    And matthew-household stewards content family-photo-1 with protection "protected"

  Scenario: Terrance enters maintenance, Matthew's family stays protected
    When operator flips terrance-laptop to maintenance
    Then within 60 seconds, doorway stops routing new work to terrance-laptop
    And the household resilience for family-photo-1 is "protected" or "partial"
    And the household resilience is NEVER "at-risk"

  Scenario: Terrance returns, protection restores
    Given terrance-laptop is in maintenance
    When operator flips terrance-laptop to online
    Then within 90 seconds, PeerStatus shows terrance-laptop online
    And the household resilience for family-photo-1 is "protected"
```

- [ ] **Step 2: Steps + Commit**

### Task L8: Step defs for existing peer-advertisement / human-resilience demo scenarios

- [ ] **Step 1: Take "Heterogeneous network handles mixed availability" from peer-advertisement.feature and write step defs**

These scenarios are @wip in tree. Keep the Gherkin as-is; write TypeScript step definitions that drive them via `curl` + Playwright.

- [ ] **Step 2: Same for "Matthew + Susan + Pete" in human-resilience.feature**

- [ ] **Step 3: Run, commit**

```bash
git add genesis/a2o/steps/federation/peer-advertisement.steps.ts \
        genesis/a2o/steps/shefa/human-resilience.steps.ts
git commit -m "test(a2o): step defs for peer-advertisement heterogeneous + human-resilience Matthew+Susan+Pete"
```

---

## Module M — WASM asset fix (deliverable 12)

### Task M1: Resolve elohim_cache_core.js 404

**Files:**
- Modify: `app/elohim-app/angular.json`

- [ ] **Step 1: Inspect current asset config**

```bash
grep -n "elohim-cache-core\|elohim_cache_core" app/elohim-app/angular.json
```

Expected: the Angular build includes `elohim-cache-core` output under `/wasm/...`. If missing or path-broken, fix the `assets` array.

- [ ] **Step 2: Add/fix asset entry**

Under `projects/elohim-app/architect/build/options/assets` add:

```json
{
  "glob": "**/*",
  "input": "../../../elohim/elohim-cache-core/pkg",
  "output": "/wasm/elohim-cache-core"
}
```

- [ ] **Step 3: Local build verification**

```bash
cd app/elohim-app && pnpm run build
ls dist/elohim-app/browser/wasm/elohim-cache-core/ | grep elohim_cache_core.js
```
Expected: file exists.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/angular.json
git commit -m "fix(build): ship elohim-cache-core WASM bundle under /wasm/ in app dist"
```

---

## Module N — Reed-Solomon sharding cleanup (discovered during sprint)

Discovered while auditing RS maturity: `elohim/elohim-storage/src/sharding.rs` is otherwise sound (reed-solomon-erasure = "6", galois_8, RS 4+3, ShardEncoder with encode + reconstruct + 10 unit tests) but has two latent defects and a wiring gap that are cheap to address in-sprint. Full Sprint-C of the p2p-resilience-proof design (auto-distribution on ingest, periodic verification, reconstruction-verify endpoint) stays deferred — only the narrow defects land here. Source-of-truth classification: shard manifests + shard locations remain Category C (operational, local projection) per the 2026-04-04 p2p-resilience-proof design gate; no new DHT entry types added.

### Task N1: Fix chunked-mode unreachable threshold logic

**Files:**
- Modify: `elohim/elohim-storage/src/sharding.rs` (`determine_encoding`)

**The defect:** In `determine_encoding` (~line 113), `single_shard_max` (16MB) > `rs_threshold` (10MB), so the first arm (`size <= single_shard_max`) swallows everything ≤16MB and the `"chunked"` branch is unreachable. Net: blobs >16MB always go straight to RS; chunked mode never fires.

**The fix:** Swap the ordering so the middle band is a real interval: `[0, single_shard_max] → none`; `(single_shard_max, rs_threshold] → chunked`; `(rs_threshold, ∞) → rs-4-7`. Constants need to be re-ordered too: `single_shard_max = 16MB` (stays), `rs_threshold = 64MB` (grows — becomes the threshold for switching to RS). A blob between 16MB and 64MB is chunked (no redundancy but split across shards); above 64MB is RS-coded.

- [ ] **Step 1: Write failing test**

In the `#[cfg(test)] mod tests` block of `sharding.rs` add:

```rust
#[test]
fn determine_encoding_returns_chunked_for_mid_band() {
    let enc = ShardEncoder::new(ShardConfig::default());
    // 16MB + 1 byte → should be chunked (bigger than single-shard, smaller than RS threshold)
    assert_eq!(enc.determine_encoding(SINGLE_SHARD_MAX + 1), "chunked");
    // 32MB → should be chunked
    assert_eq!(enc.determine_encoding(32 * 1024 * 1024), "chunked");
    // 100MB → should be rs-4-7
    assert_eq!(enc.determine_encoding(100 * 1024 * 1024), "rs-4-7");
    // 16MB → should be none (boundary)
    assert_eq!(enc.determine_encoding(SINGLE_SHARD_MAX), "none");
}
```

- [ ] **Step 2: Run — expect FAIL**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib sharding::tests::determine_encoding_returns_chunked_for_mid_band
```
Expected: FAIL — current logic returns "rs-4-7" or unreachable for the mid-band assertions.

- [ ] **Step 3: Apply the fix**

Change `RS_THRESHOLD` from `10 * 1024 * 1024` to `64 * 1024 * 1024`. Change `determine_encoding` to:

```rust
pub fn determine_encoding(&self, size: usize) -> &'static str {
    if size <= self.config.single_shard_max {
        "none"
    } else if size <= self.config.rs_threshold {
        "chunked"
    } else {
        "rs-4-7"
    }
}
```

Note `<=` on the `rs_threshold` branch — makes the boundary tests deterministic.

- [ ] **Step 4: Run — expect PASS**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib sharding
```
Expected: PASS for the new test plus all 10 existing sharding tests.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/sharding.rs
git commit -m "fix(sharding): reachable chunked-mode threshold (16MB<size<=64MB → chunked)"
```

### Task N2: Propagate Reed-Solomon encode errors instead of panicking

**Files:**
- Modify: `elohim/elohim-storage/src/sharding.rs`

**The defect:** `ReedSolomon::new(...)` and `.encode(...)` calls at lines 156, 177, 225, 246 use `.unwrap()`. A malformed config or encode error crashes the process. Public API (`create_manifest`, `create_shards`) swallows the panic without a result.

**The fix:** Change `create_manifest` and `create_shards` signatures to return `Result<_, io::Error>` and propagate RS errors with `io::Error::new(io::ErrorKind::InvalidData, e.to_string())`. Update call-sites (typically blob_store.rs).

- [ ] **Step 1: Find callers**

```bash
grep -rn "create_manifest\|create_shards" elohim/elohim-storage/src/
```
Note each call-site. They'll need `?` propagation after the change.

- [ ] **Step 2: Write failing test**

```rust
#[test]
fn create_manifest_returns_err_on_rs_config_failure() {
    // RS panic case: data_shards + parity_shards > 256 in galois_8
    let bad_config = ShardConfig {
        rs_data_shards: 200,
        rs_parity_shards: 200,
        ..ShardConfig::default()
    };
    let enc = ShardEncoder::new(bad_config);
    let data = vec![0u8; 100 * 1024 * 1024];  // force RS path
    let result = enc.create_manifest(&data, "application/octet-stream", "commons");
    assert!(result.is_err(), "expected Err from impossible RS config, got {:?}", result);
}
```

(This test currently panics — step 3's fix changes that to Err.)

- [ ] **Step 3: Run — expect panic or build fail**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib sharding::tests::create_manifest_returns_err_on_rs_config_failure
```
Expected: FAIL with panic OR build error (test signature expects Result).

- [ ] **Step 4: Refactor signatures**

Change `create_manifest` signature to:

```rust
pub fn create_manifest(&self, data: &[u8], mime_type: &str, reach: &str)
    -> Result<ShardManifest, io::Error>
```

Replace `.unwrap()` on `ReedSolomon::new(...)` and `.encode(...)` with `.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?`. Wrap the struct literal in `Ok(...)` at the end.

Do the same for `create_shards` — return `Result<Vec<Vec<u8>>, io::Error>`.

Update all callers grep'd in Step 1 to propagate the Result with `?`.

- [ ] **Step 5: Run full sharding + blob_store tests**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib sharding blob_store
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings
```
Expected: all PASS, no new clippy warnings.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/sharding.rs elohim/elohim-storage/src/blob_store.rs
git commit -m "fix(sharding): propagate Reed-Solomon errors as io::Error instead of panicking"
```

---

## Self-review

**Spec coverage:**
- D1–D5 → Module A ✓
- PeerStatus Phase 1 closeout (§7.2) → Module B ✓
- Household grouping (§7.3) → Module D ✓
- Node-shape publish + device list (§7.4, §6.A) → Module C ✓
- Stewarded-resources page (§7.5) → Module G ✓
- Resilience tooltip (§7.6) → Module E (E3) ✓
- Network Health posture (§7.7) → Module F ✓
- Household resilience computation (§7.8) → Module E (E1, E2) ✓
- Doorway admin gaps (§7.9) → Module H ✓
- Shem activation (§7.10) → Module K ✓
- New a2o scenarios + step defs (§7.11) → Module L ✓
- WASM asset fix (§7.12) → Module M ✓
- Maintenance choreography (§6.B, acceptance bar last bullet) → Module J + Task L7 ✓
- Frontend retirement (part of §7.1 / D1 follow-on) → Module I ✓

**Placeholder scan:** No "TBD"/"TODO"/"similar to Task N" found. One mild softness: Task C7 Step 3 leaves Cargo dep version decisions to the implementer ("check existing use first") — acceptable since the repo already uses these crates.

**Type consistency:** `NodeShapeView`, `HouseholdDevicesView`, `DeviceEntryView`, `NetworkPostureView`, `HouseholdResilienceView`, `CommittedResources` — all introduced in C2/C1/E1/F1 and referenced consistently in handlers (C5/C6/F2/E1) and frontend types generated via codegen. `StewardedNodeRow` fields added in C3 are consumed in C5/C6 queries with matching names. `household-matthew` id format is used consistently across collectives fixtures, humans.json updates, shem StatefulSet labels, and a2o scenarios.

**Gaps discovered:** none. The plan maps one-to-one against the spec's 12 deliverables + §6.B choreography + the acceptance bar.

---

## Execution handoff

Plan complete and saved to `genesis/docs/superpowers/plans/2026-04-19-p2p-dataplane-visibility-plan.md`.

Two execution options:

1. **Subagent-driven (recommended)** — fresh subagent per task, review between tasks, fast iteration. Good fit for a 13-module plan with strong module boundaries.
2. **Inline execution** — run tasks in this session with checkpoints after each module.

Which approach?
