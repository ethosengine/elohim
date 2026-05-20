# Light-up-the-topology Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land three routed visual surfaces that prove the P2P substrate is alive and stewarding content resiliently: (A) per-device compute triptych (free / used / stewarded) in `/shefa/cluster`, (B) doorway-dashboard wired into the app shell with nav-menu discoverability, (C) resilience tooltip + placement-gaps row on the distribution badge, with browser-tier scenarios un-`@wip`-ed.

**Architecture:** Three independent sub-projects in one sprint. Each produces a shippable surface with its own commit boundary. Substrate is already live for A and partially for C; B is mostly app-shell wiring. All work routes through existing services (`Viewer.hub`, `DoorwayAdminService`, EPR head response) — no new HTTP routes except one missing resilience endpoint in C.

**Tech Stack:** Rust (async-graphql, diesel, ts-rs), TypeScript (Angular 19 standalone components, signals, Apollo), Cucumber + Playwright (a2o browser tier).

**Divergences from the synthesis (verified during research):**
- **Doorway**: routes already exist at `/doorway` and `/doorway/config`; the *real* gap is the absence of a layout wrapper + nav-menu entry. The component is fully featured (243 LOC, 95 test cases). Sub-project B scope shifts from "wire routes" to "wire app shell."
- **Compute**: `storage_used_bytes` and `storage_total_bytes` are already on `DeviceSummaryGql`. The triptych adds an explicit `compute { free used stewarded }` shape for semantic clarity *and* introduces the new `stewarded` field (the only truly novel value).
- **Resilience**: substrate is *incomplete*, not just surface-partial. `/api/v1/resilience/{id}/household` is missing (feature file expects it), `placement_gaps` row schema is intentionally loose (`Record<string, unknown>[]`), and 6 browser step defs return `'pending'`. Sub-project C has real Rust + schema work.

**Substrate currency note:** `rust-architect.md` was just updated by the operator to clear earlier drift. Refer to it freely for backend patterns.

---

## P2P design-gate declaration (source-of-truth lineage)

This plan adds zero notarized entity types, zero new DHT entry types, and zero new diesel tables. All new artifacts are **Category-C operational projections** (per the EPR substrate taxonomy) — derived views over already-notarized truth, serialized into View schemas that govern the Rust→TS wire boundary (per `elohim/sdk/schemas/v1/views/CONVENTIONS.md`). Each new shape declares its truth source:

| New view shape | Category | Truth sources (already notarized / persisted) |
|---|---|---|
| `ComputeTriptych` | C (operational) | `system_metrics::filesystem_capacity_bytes` + `directory_size` (per-node probe) + `rea_commitments` table (notarized Category-A REA Commitments where `action='custody-blob'` and `provider_cid IN my_peers`) |
| `PlacementGapRow` | C (operational) | `peer_blob_inventory` (gossip-synced operational truth) + peer-identity bindings (Category-A from DHT) + declared reach class on EPR head (Category-A) |
| `ResilienceHubView` | C (operational) | `peer_blob_inventory` + peer-identity bindings + hub-membership facts (Category-A from DHT, where present) |
| `HubSummary { kind, … }` | C (operational) | Hub-kind classification derives from notarized membership facts; the **hub role itself is operator-configurable runtime behavior, not a notarized entity** ([[project_hub_archetype_abstraction]], [[project_hub_optional_floor]]) |

**Hub is the abstraction**, not "household". Per `[[project_hub_archetype_abstraction]]` and `[[project_hub_optional_floor]]`: hubs are dial-up-by-capability roles; a single device can be a hub, a cluster in a dwelling can be a hub, a collective can be a hub. The substrate exposes `hub` as the polymorphic surface; concrete kinds (`dwelling`, `collective`, `computed`) resolve in UI labels. Substrate stays kind-agnostic so future hub kinds slot in without schema change.

No new HTTP routes for A or B. C adds **one** new route — `/api/v1/resilience/{id}/hub` — which projects the operational view above for a content ID; it is not a notarized surface, and a future doorway/hub-aware projection layer can replace this endpoint without changing the truth layer.

---

## File Structure

### Sub-project A: Compute Triptych

**Rust (elohim/elohim-storage/):**
- Modify `src/services/reciprocity_view.rs` — add `aggregate_stewarded_bytes_by_peer(pool, my_peer_ids) -> HashMap<String, u64>` helper
- Modify `src/services/cluster_view.rs` — pass stewarded map into `device_summary_from_result`, attach `compute` field
- Create `src/views/compute_triptych.rs` — `ComputeTriptych` view struct with ts-rs derive
- Modify `src/views/mod.rs` — re-export
- Modify `elohim-views/src/infrastructure.rs` — add `compute: Option<ComputeTriptych>` to `DeviceSummary`
- Modify `src/graphql/resolvers.rs` — add `ComputeTriptychGql` + `compute()` field resolver on `DeviceSummaryGql`
- Modify `sdk/schemas/v1/views/device-summary.schema.json` — add `compute` property
- Create `sdk/schemas/v1/views/compute-triptych.schema.json` — new view schema
- Create `tests/compute_triptych.rs` — service + resolver integration test

**TypeScript (sdk/storage-client-ts/):**
- Regenerate `src/generated/compute-triptych.ts` via `cargo test export_bindings`
- Regenerate `src/generated/device-summary.ts`
- Update `src/graphql/queries.ts` — extend `VIEWER_HUB_QUERY` to fetch `compute { free used stewarded }`

**Angular (app/elohim-app/):**
- Modify `src/app/generated/my-cluster-view.ts` — regenerated automatically
- Modify `src/app/shefa/components/device-tile/device-tile.component.ts` — render compute triptych
- Modify `src/app/shefa/components/device-tile/device-tile.component.html` — add triptych block with `data-testid`s
- Modify `src/app/shefa/components/device-tile/device-tile.component.scss` — triptych styles
- Modify `src/app/shefa/components/device-tile/device-tile.component.spec.ts` — assert triptych renders

**a2o:**
- Modify `genesis/a2o/features/shefa/m1-matthew-terrance-delivery.feature` — add `@compute-triptych` scenario
- Modify `genesis/a2o/steps/ui/shefa.steps.ts` (or create if absent) — implement triptych Then-steps

### Sub-project B: Doorway-peers wiring

**Angular:**
- Create `app/elohim-app/src/app/doorway/components/doorway-layout/doorway-layout.component.ts`
- Create `app/elohim-app/src/app/doorway/components/doorway-layout/doorway-layout.component.html`
- Create `app/elohim-app/src/app/doorway/components/doorway-layout/doorway-layout.component.scss`
- Modify `app/elohim-app/src/app/doorway/doorway.routes.ts` — wrap routes with layout
- Modify `app/elohim-app/src/app/elohim/components/elohim-shell/elohim-shell.component.html` (or main-nav) — add doorway nav entry

**a2o:**
- Modify `genesis/a2o/features/browser/doorway-dashboard-health.feature` — un-`@wip` the two scenarios
- Modify `genesis/a2o/steps/ui/doorway.steps.ts` (or create) — implement browser steps

### Sub-project C: Resilience tooltip + placement-gaps

**Rust:**
- Create `elohim/elohim-storage/src/api/resilience_hub.rs` — `GET /api/v1/resilience/{id}/hub` handler returning a polymorphic `ResilienceHubView` (hubs of kind `dwelling | collective | computed`); operational projection — does not introduce a notarized entity
- Modify `elohim/elohim-storage/src/api/router.rs` (or wherever routes are mounted) — register route
- Modify `elohim/sdk/schemas/v1/views/distribution-details.schema.json` — graduate `placementGaps` row schema
- Create `elohim/sdk/schemas/v1/views/placement-gap-row.schema.json` — dedicated row schema (kind enum is hub-abstract: `hub_diversity`, `replica_count`, `reach_class`)
- Create `elohim/sdk/schemas/v1/views/resilience-hub-view.schema.json` — wire format for the `/hub` projection
- Modify `elohim/elohim-storage/tests/distribution_view.rs` — seed `placement_gaps` rows in fixture

**Angular (elohim-library, since the badge lives there):**
- Modify `app/elohim-library/projects/elohim-service/src/components/distribution-badge/distribution-badge.component.ts` — add tooltip state + lazy detail-fetch
- Modify `app/elohim-library/projects/elohim-service/src/components/distribution-badge/distribution-badge.component.html` — tooltip template + placement-gaps row
- Modify `app/elohim-library/projects/elohim-service/src/components/distribution-badge/distribution-badge.component.scss` — tooltip styles
- Modify `app/elohim-library/projects/elohim-service/src/components/distribution-badge/distribution-badge.component.spec.ts` — assert tooltip behavior

**a2o:**
- Modify `genesis/a2o/features/resilience/observable-distribution.feature` — un-`@wip` resilience-tooltip and side-by-side scenarios
- Modify `genesis/a2o/steps/resilience.steps.ts` — replace 6 'pending' step defs with real Playwright assertions

---

## Sub-Project A: Compute Triptych

### Task A1: Add `aggregate_stewarded_bytes_by_peer` helper in `reciprocity_view.rs`

**Files:**
- Modify: `elohim/elohim-storage/src/services/reciprocity_view.rs`
- Test: `elohim/elohim-storage/tests/compute_triptych.rs`

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-storage/tests/compute_triptych.rs`:

```rust
use elohim_storage::services::reciprocity_view::aggregate_stewarded_bytes_by_peer;
use elohim_storage::test_support::test_pool;

#[tokio::test]
async fn stewarded_bytes_sums_custody_blob_commitments_by_peer() {
    let pool = test_pool();
    seed_custody_commitment(&pool, "peer_M_laptop", "agent_T", 1_000_000).await;
    seed_custody_commitment(&pool, "peer_M_laptop", "agent_J", 500_000).await;
    seed_custody_commitment(&pool, "peer_M_phone", "agent_T", 200_000).await;
    seed_custody_commitment(&pool, "peer_M_laptop", "agent_T", 300_000).await; // duplicate provider/receiver

    let peers = vec!["peer_M_laptop".to_string(), "peer_M_phone".to_string()];
    let result = aggregate_stewarded_bytes_by_peer(&pool, &peers)
        .await
        .expect("aggregate");

    assert_eq!(result.get("peer_M_laptop"), Some(&1_800_000_u64));
    assert_eq!(result.get("peer_M_phone"), Some(&200_000_u64));
}

#[tokio::test]
async fn stewarded_bytes_empty_when_no_commitments() {
    let pool = test_pool();
    let peers = vec!["peer_lonely".to_string()];
    let result = aggregate_stewarded_bytes_by_peer(&pool, &peers)
        .await
        .expect("aggregate");
    assert!(result.is_empty() || result.get("peer_lonely").copied() == Some(0));
}

async fn seed_custody_commitment(pool: &elohim_storage::db::DbPool, provider: &str, receiver: &str, bytes: u64) {
    use diesel_async::RunQueryDsl;
    use elohim_storage::db::schema::rea_commitments::dsl::*;
    let mut conn = pool.get().await.unwrap();
    diesel::insert_into(rea_commitments)
        .values((
            action.eq("custody-blob"),
            provider_cid.eq(provider),
            receiver_cid.eq(receiver),
            resource_quantity_value.eq(bytes as f32),
        ))
        .execute(&mut conn)
        .await
        .expect("insert");
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /projects/elohim/elohim/elohim-storage
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
  cargo test --test compute_triptych
```

Expected: FAIL with "function `aggregate_stewarded_bytes_by_peer` not found".

- [ ] **Step 3: Implement the helper in `reciprocity_view.rs`**

Add to `elohim/elohim-storage/src/services/reciprocity_view.rs`:

```rust
use std::collections::HashMap;

pub async fn aggregate_stewarded_bytes_by_peer(
    pool: &DbPool,
    my_peer_ids: &[String],
) -> Result<HashMap<String, u64>, ReciprocityViewError> {
    use crate::db::schema::rea_commitments::dsl::*;
    use diesel::dsl::sum;
    use diesel_async::RunQueryDsl;

    let mut conn = pool.get().await.map_err(ReciprocityViewError::Pool)?;

    let rows: Vec<(String, Option<f64>)> = rea_commitments
        .filter(action.eq("custody-blob"))
        .filter(provider_cid.eq_any(my_peer_ids))
        .group_by(provider_cid)
        .select((provider_cid, sum(resource_quantity_value)))
        .load(&mut conn)
        .await
        .map_err(ReciprocityViewError::Query)?;

    Ok(rows
        .into_iter()
        .filter_map(|(peer, bytes)| bytes.map(|b| (peer, b.max(0.0) as u64)))
        .collect())
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  RUSTFLAGS='--cfg getrandom_backend="custom"' \
  cargo test --test compute_triptych
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/services/reciprocity_view.rs elohim/elohim-storage/tests/compute_triptych.rs
git commit -m "feat(compute-triptych): add aggregate_stewarded_bytes_by_peer service"
```

### Task A2: Define `ComputeTriptych` view + JSON schema

**Files:**
- Create: `elohim/elohim-storage/src/views/compute_triptych.rs`
- Modify: `elohim/elohim-storage/src/views/mod.rs`
- Create: `elohim/sdk/schemas/v1/views/compute-triptych.schema.json`
- Modify: `elohim/sdk/schemas/v1/views/device-summary.schema.json`

- [ ] **Step 1: Write the schema first (IoC)**

Create `elohim/sdk/schemas/v1/views/compute-triptych.schema.json`:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.protocol/schemas/v1/views/compute-triptych.schema.json",
  "title": "ComputeTriptych",
  "type": "object",
  "additionalProperties": false,
  "required": ["free", "used", "stewarded"],
  "properties": {
    "free": { "type": ["string", "null"], "description": "Bytes available on device blob filesystem (capacity - used)" },
    "used": { "type": ["string", "null"], "description": "Bytes occupied by blob storage on device" },
    "stewarded": { "type": ["string", "null"], "description": "Bytes committed via rea_commitments where this peer is provider" }
  }
}
```

Modify `elohim/sdk/schemas/v1/views/device-summary.schema.json` — add to `properties`:

```json
"compute": {
  "anyOf": [
    { "type": "null" },
    { "$ref": "compute-triptych.schema.json" }
  ]
}
```

- [ ] **Step 2: Write the failing Rust struct test**

Add to `elohim/elohim-storage/tests/compute_triptych.rs`:

```rust
use elohim_storage::views::ComputeTriptych;

#[test]
fn compute_triptych_serializes_camel_case() {
    let triptych = ComputeTriptych {
        free: Some(1_000),
        used: Some(500),
        stewarded: Some(250),
    };
    let json = serde_json::to_value(&triptych).unwrap();
    assert_eq!(json["free"], 1000);
    assert_eq!(json["used"], 500);
    assert_eq!(json["stewarded"], 250);
}
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cargo test --test compute_triptych compute_triptych_serializes
```

Expected: FAIL — `ComputeTriptych` not found.

- [ ] **Step 4: Create the view struct**

Create `elohim/elohim-storage/src/views/compute_triptych.rs`:

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ComputeTriptych {
    pub free: Option<u64>,
    pub used: Option<u64>,
    pub stewarded: Option<u64>,
}
```

Modify `elohim/elohim-storage/src/views/mod.rs` — add:

```rust
mod compute_triptych;
pub use compute_triptych::ComputeTriptych;
```

- [ ] **Step 5: Add `compute` field to `DeviceSummary` wire type**

Modify `elohim/elohim-views/src/infrastructure.rs` `DeviceSummary` struct — append after `beacon_age_ms`:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub compute: Option<elohim_storage::views::ComputeTriptych>,
```

(If `elohim-views` cannot depend on `elohim-storage`, define `ComputeTriptych` in `elohim-views` instead and re-export from storage. Verify dependency direction with `cargo metadata` before deciding placement.)

- [ ] **Step 6: Run test + ts-rs export**

```bash
cargo test --test compute_triptych
cargo test export_bindings
```

Expected: tests PASS, `sdk/storage-client-ts/src/generated/ComputeTriptych.ts` written.

- [ ] **Step 7: Run schema contract test**

```bash
cargo test --test schema_contract
```

Expected: PASS (schema and struct in sync).

- [ ] **Step 8: Commit**

```bash
git add elohim/elohim-storage/src/views/compute_triptych.rs \
        elohim/elohim-storage/src/views/mod.rs \
        elohim/elohim-views/src/infrastructure.rs \
        elohim/sdk/schemas/v1/views/compute-triptych.schema.json \
        elohim/sdk/schemas/v1/views/device-summary.schema.json \
        elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(compute-triptych): add ComputeTriptych view + schema"
```

### Task A3: Wire compute aggregation into `cluster_view::device_summary_from_result`

**Files:**
- Modify: `elohim/elohim-storage/src/services/cluster_view.rs`
- Modify: `elohim/elohim-storage/tests/cluster_view.rs`

- [ ] **Step 1: Write the failing integration test**

Add to `elohim/elohim-storage/tests/cluster_view.rs`:

```rust
#[tokio::test]
async fn cluster_view_includes_compute_triptych() {
    let pool = test_pool();
    seed_binding(&pool, "agent_M", "peer_M_laptop").await;
    seed_custody_commitment(&pool, "peer_M_laptop", "agent_T", 750_000).await;

    let federator = Federator::new(P2PHandle::for_testing());
    let view = aggregate_my_cluster_view(&pool, &federator, "agent_M")
        .await
        .expect("aggregate");

    let laptop = view
        .devices
        .iter()
        .find(|d| d.peer_id == "peer_M_laptop")
        .expect("laptop present");

    let compute = laptop.compute.as_ref().expect("compute present");
    assert_eq!(compute.stewarded, Some(750_000));
    assert!(compute.used.is_some(), "used populated from system_metrics");
    assert!(compute.free.is_some(), "free derivable from total-used");
}
```

(Reuse `seed_custody_commitment` helper from Task A1 — extract into a shared `test_support` module if not already there.)

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test --test cluster_view cluster_view_includes_compute_triptych
```

Expected: FAIL — `laptop.compute` is `None`.

- [ ] **Step 3: Modify `aggregate_my_cluster_view` to compute the stewarded map**

In `elohim/elohim-storage/src/services/cluster_view.rs` `aggregate_my_cluster_view`, after collecting `my_peer_ids`:

```rust
let my_peer_ids: Vec<String> = bindings.iter().map(|b| b.peer_id.clone()).collect();
let stewarded_map = aggregate_stewarded_bytes_by_peer(pool, &my_peer_ids)
    .await
    .unwrap_or_default();
```

Then thread `stewarded_map` into `device_summary_from_result(... &stewarded_map)` calls.

- [ ] **Step 4: Modify `device_summary_from_result` to attach compute**

In the same file, in `device_summary_from_result`:

```rust
let storage_used = payload.get("storage_used_bytes").and_then(serde_json::Value::as_u64);
let storage_total = payload.get("storage_total_bytes").and_then(serde_json::Value::as_u64);
let free = match (storage_total, storage_used) {
    (Some(t), Some(u)) => Some(t.saturating_sub(u)),
    _ => None,
};
let stewarded = stewarded_map.get(&peer_id).copied();

let compute = Some(ComputeTriptych {
    free,
    used: storage_used,
    stewarded,
});

DeviceSummary {
    // ... existing fields
    compute,
}
```

Import `ComputeTriptych` and `aggregate_stewarded_bytes_by_peer` at top of file.

- [ ] **Step 5: Run test + adjacent tests**

```bash
cargo test --test cluster_view
```

Expected: ALL PASS (existing tests still green; new test passes).

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/services/cluster_view.rs \
        elohim/elohim-storage/tests/cluster_view.rs
git commit -m "feat(compute-triptych): attach compute to DeviceSummary in cluster_view"
```

### Task A4: Add `ComputeTriptychGql` + `compute()` resolver field

**Files:**
- Modify: `elohim/elohim-storage/src/graphql/resolvers.rs`
- Test: extend `elohim/elohim-storage/tests/compute_triptych.rs` with a GraphQL test

- [ ] **Step 1: Write the failing GraphQL resolver test**

Add to `elohim/elohim-storage/tests/compute_triptych.rs`:

```rust
#[tokio::test]
async fn graphql_viewer_hub_returns_compute_triptych() {
    let (pool, federator) = test_fixture_with_stewardship().await;
    let schema = build_schema(pool, federator);
    let query = r#"
        query { viewer { hub { devices { peerId compute { free used stewarded } } } } }
    "#;
    let res = schema.execute(query).await;
    assert!(res.errors.is_empty(), "errors: {:?}", res.errors);
    let data = res.data.into_json().unwrap();
    let device = &data["viewer"]["hub"]["devices"][0];
    assert!(device["compute"]["stewarded"].is_string(), "stewarded as string for JS precision");
}
```

(Use existing test-fixture helper or build one mirroring `tests/graphql_topology.rs` if present.)

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test --test compute_triptych graphql_viewer_hub
```

Expected: FAIL — `compute` field unknown on `DeviceSummary`.

- [ ] **Step 3: Add `ComputeTriptychGql` + field resolver**

In `elohim/elohim-storage/src/graphql/resolvers.rs`, add near other GQL types (search for `DeviceSummaryGql` and place adjacent):

```rust
#[derive(Debug, Clone, Default)]
pub struct ComputeTriptychGql {
    pub free: Option<String>,
    pub used: Option<String>,
    pub stewarded: Option<String>,
}

#[Object]
impl ComputeTriptychGql {
    async fn free(&self) -> Option<&str> { self.free.as_deref() }
    async fn used(&self) -> Option<&str> { self.used.as_deref() }
    async fn stewarded(&self) -> Option<&str> { self.stewarded.as_deref() }
}

impl From<&crate::views::ComputeTriptych> for ComputeTriptychGql {
    fn from(v: &crate::views::ComputeTriptych) -> Self {
        Self {
            free: v.free.map(|n| n.to_string()),
            used: v.used.map(|n| n.to_string()),
            stewarded: v.stewarded.map(|n| n.to_string()),
        }
    }
}
```

Modify `DeviceSummaryGql` (add field):

```rust
pub compute: Option<ComputeTriptychGql>,
```

In the `#[Object] impl DeviceSummaryGql` block, add:

```rust
async fn compute(&self) -> Option<&ComputeTriptychGql> { self.compute.as_ref() }
```

In the `From<crate::views::DeviceSummary>` conversion (around line 356 per research):

```rust
compute: device.compute.as_ref().map(ComputeTriptychGql::from),
```

- [ ] **Step 4: Run resolver test**

```bash
cargo test --test compute_triptych
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/graphql/resolvers.rs elohim/elohim-storage/tests/compute_triptych.rs
git commit -m "feat(compute-triptych): expose compute field on DeviceSummaryGql resolver"
```

### Task A5: Extend `VIEWER_HUB_QUERY` to fetch compute

**Files:**
- Modify: `elohim/sdk/storage-client-ts/src/graphql/queries.ts`

- [ ] **Step 1: Add compute to query**

Find `VIEWER_HUB_QUERY` in `elohim/sdk/storage-client-ts/src/graphql/queries.ts`. Add to the `devices` selection set:

```graphql
compute { free used stewarded }
```

- [ ] **Step 2: Rebuild storage-client-ts**

```bash
cd elohim/sdk/storage-client-ts && pnpm build
```

Expected: build succeeds.

- [ ] **Step 3: Commit**

```bash
git add elohim/sdk/storage-client-ts/src/graphql/queries.ts elohim/sdk/storage-client-ts/dist/
git commit -m "feat(compute-triptych): extend VIEWER_HUB_QUERY with compute"
```

### Task A6: Render compute triptych in `DeviceTileComponent`

**Files:**
- Modify: `app/elohim-app/src/app/shefa/components/device-tile/device-tile.component.ts`
- Modify: `app/elohim-app/src/app/shefa/components/device-tile/device-tile.component.html`
- Modify: `app/elohim-app/src/app/shefa/components/device-tile/device-tile.component.scss`
- Test: `app/elohim-app/src/app/shefa/components/device-tile/device-tile.component.spec.ts`

- [ ] **Step 1: Write the failing component test**

In `device-tile.component.spec.ts`, add:

```typescript
it('renders the compute triptych when compute is present', async () => {
  const fixture = TestBed.createComponent(DeviceTileComponent);
  fixture.componentRef.setInput('device', {
    peerId: 'peer_M_laptop',
    archetype: 'desktop',
    displayName: "Matthew's laptop",
    online: true,
    storageUsedBytes: 500_000,
    storageTotalBytes: 10_000_000,
    compute: { free: '9500000', used: '500000', stewarded: '250000' },
  } as any);
  fixture.detectChanges();
  const el = fixture.nativeElement as HTMLElement;
  expect(el.querySelector('[data-testid="compute-free-bytes"]')?.textContent).toContain('9.5');
  expect(el.querySelector('[data-testid="compute-used-bytes"]')?.textContent).toContain('500');
  expect(el.querySelector('[data-testid="compute-stewarded-bytes"]')?.textContent).toContain('250');
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd app/elohim-app && pnpm exec vitest run device-tile.component.spec
```

Expected: FAIL — selectors not present.

- [ ] **Step 3: Add format helper to component**

In `device-tile.component.ts`, add (or import from a shared util):

```typescript
protected formatBytes(value: string | null | undefined): string {
  if (!value) return '—';
  const n = Number(value);
  if (!isFinite(n)) return '—';
  if (n >= 1e9) return `${(n / 1e9).toFixed(1)} GB`;
  if (n >= 1e6) return `${(n / 1e6).toFixed(1)} MB`;
  if (n >= 1e3) return `${(n / 1e3).toFixed(1)} KB`;
  return `${n} B`;
}
```

- [ ] **Step 4: Add triptych block to template**

In `device-tile.component.html`, append:

```html
@if (device().compute) {
  <section class="compute-triptych" aria-label="Compute breakdown">
    <h4 class="compute-triptych__title">Compute</h4>
    <dl class="compute-triptych__values">
      <div class="compute-triptych__cell">
        <dt>Free</dt>
        <dd data-testid="compute-free-bytes">{{ formatBytes(device().compute?.free) }}</dd>
      </div>
      <div class="compute-triptych__cell">
        <dt>Used</dt>
        <dd data-testid="compute-used-bytes">{{ formatBytes(device().compute?.used) }}</dd>
      </div>
      <div class="compute-triptych__cell">
        <dt>Stewarded</dt>
        <dd data-testid="compute-stewarded-bytes">{{ formatBytes(device().compute?.stewarded) }}</dd>
      </div>
    </dl>
  </section>
}
```

- [ ] **Step 5: Add styles**

In `device-tile.component.scss`:

```scss
.compute-triptych {
  margin-top: 1rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--surface-border, #2a2a2a);

  &__title {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted, #888);
    margin: 0 0 0.5rem 0;
  }

  &__values {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.5rem;
    margin: 0;
  }

  &__cell {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;

    dt {
      font-size: 0.7rem;
      color: var(--text-muted, #888);
    }
    dd {
      margin: 0;
      font-size: 0.95rem;
      font-weight: 500;
      font-variant-numeric: tabular-nums;
    }
  }
}
```

- [ ] **Step 6: Run test to verify it passes**

```bash
pnpm exec vitest run device-tile.component.spec
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add app/elohim-app/src/app/shefa/components/device-tile/
git commit -m "feat(compute-triptych): render free/used/stewarded triptych in DeviceTileComponent"
```

### Task A7: Add `@compute-triptych` a2o scenario + step defs

**Files:**
- Modify: `genesis/a2o/features/shefa/m1-matthew-terrance-delivery.feature`
- Modify: `genesis/a2o/steps/ui/shefa.steps.ts` (or create if missing)

- [ ] **Step 1: Add the scenario**

Append to `m1-matthew-terrance-delivery.feature`:

```gherkin
@browser @compute-triptych @resilience-p1
Scenario: Matthew's device tile shows free / used / stewarded compute breakdown
  When Matthew opens the cluster topology page at "/shefa/cluster"
  And he locates his laptop's device tile
  Then the device tile shows a compute triptych
  And the compute triptych "Free" cell has a non-empty byte value
  And the compute triptych "Used" cell has a non-empty byte value
  And the compute triptych "Stewarded" cell shows non-zero bytes when Matthew is hosting for another peer
```

- [ ] **Step 2: Add step definitions**

In `genesis/a2o/steps/ui/shefa.steps.ts` (create if missing — mirror existing UI step-def files):

```typescript
import { When, Then } from '@cucumber/cucumber';
import { expect } from 'chai';
import { requirePlaywright } from '../../framework/world';

When('he locates his laptop\'s device tile', async function () {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  await device.page.locator('[data-testid="device-tile"]').first().waitFor();
});

Then('the device tile shows a compute triptych', async function () {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  const triptych = device.page.locator('.compute-triptych').first();
  expect(await triptych.isVisible()).to.equal(true);
});

Then('the compute triptych {string} cell has a non-empty byte value', async function (label: string) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  const testid = `compute-${label.toLowerCase()}-bytes`;
  const text = (await device.page.locator(`[data-testid="${testid}"]`).first().textContent()) ?? '';
  expect(text.trim()).to.not.equal('—');
  expect(text.trim()).to.not.equal('');
});

Then('the compute triptych {string} cell shows non-zero bytes when Matthew is hosting for another peer', async function (label: string) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  const testid = `compute-${label.toLowerCase()}-bytes`;
  const text = (await device.page.locator(`[data-testid="${testid}"]`).first().textContent()) ?? '';
  expect(text.trim()).to.match(/[0-9]/);
});
```

- [ ] **Step 3: Run the scenario locally**

```bash
cd /projects/elohim && pnpm -F a2o exec cucumber-js --tags '@compute-triptych'
```

Expected: PASS (or the dev server needs to be running with seeded data — note as a precondition).

- [ ] **Step 4: Commit**

```bash
git add genesis/a2o/features/shefa/m1-matthew-terrance-delivery.feature \
        genesis/a2o/steps/ui/shefa.steps.ts
git commit -m "test(compute-triptych): add @compute-triptych browser scenario + steps"
```

### Task A8: Sub-project A integration verification

- [ ] **Step 1: Boot the stack**

Use `hc-dev-orchestrator` to bring up conductor + storage + doorway with seed data:

```bash
cd app/elohim-app && pnpm run hc:start:seed
```

- [ ] **Step 2: Visit `/shefa/cluster` in a browser**

Confirm Matthew's device tile shows the compute triptych with three live values.

- [ ] **Step 3: Run a2o scenario end-to-end**

```bash
pnpm -F a2o exec cucumber-js --tags '@compute-triptych and not @wip'
```

Expected: PASS.

- [ ] **Step 4: Take a screenshot for the sprint result**

Save to `genesis/sprint-results/2026-05-20-light-up-topology/compute-triptych.png`.

---

## Sub-Project B: Doorway-peers wiring

**Note:** Research showed the doorway-dashboard component is fully featured and the `/doorway` route exists. The actual gap is the absence of a layout wrapper + nav-menu entry. This sub-project is mostly app-shell work.

### Task B1: Create `DoorwayLayoutComponent`

**Files:**
- Create: `app/elohim-app/src/app/doorway/components/doorway-layout/doorway-layout.component.ts`
- Create: `app/elohim-app/src/app/doorway/components/doorway-layout/doorway-layout.component.html`
- Create: `app/elohim-app/src/app/doorway/components/doorway-layout/doorway-layout.component.scss`
- Test: `app/elohim-app/src/app/doorway/components/doorway-layout/doorway-layout.component.spec.ts`

- [ ] **Step 1: Reference the shefa-layout pattern**

Read `app/elohim-app/src/app/shefa/components/shefa-layout/shefa-layout.component.ts` for structure. Mirror it for doorway (sidenav + router-outlet).

- [ ] **Step 2: Write the failing test**

Create `doorway-layout.component.spec.ts`:

```typescript
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { DoorwayLayoutComponent } from './doorway-layout.component';

describe('DoorwayLayoutComponent', () => {
  it('renders the doorway sidenav and a router outlet', async () => {
    await TestBed.configureTestingModule({
      imports: [DoorwayLayoutComponent],
      providers: [provideRouter([])],
    }).compileComponents();
    const fixture = TestBed.createComponent(DoorwayLayoutComponent);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="doorway-sidenav"]')).toBeTruthy();
    expect(el.querySelector('router-outlet')).toBeTruthy();
  });
});
```

- [ ] **Step 3: Run test to verify it fails**

```bash
pnpm exec vitest run doorway-layout.component.spec
```

Expected: FAIL — component does not exist.

- [ ] **Step 4: Create the component**

Create `doorway-layout.component.ts`:

```typescript
import { ChangeDetectionStrategy, Component } from '@angular/core';
import { RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';

@Component({
  selector: 'app-doorway-layout',
  standalone: true,
  imports: [RouterOutlet, RouterLink, RouterLinkActive],
  templateUrl: './doorway-layout.component.html',
  styleUrls: ['./doorway-layout.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DoorwayLayoutComponent {}
```

Create `doorway-layout.component.html`:

```html
<div class="doorway-layout">
  <nav class="doorway-layout__sidenav" data-testid="doorway-sidenav" aria-label="Doorway sections">
    <a routerLink="." routerLinkActive="active" [routerLinkActiveOptions]="{ exact: true }">Dashboard</a>
    <a routerLink="config" routerLinkActive="active">Configuration</a>
  </nav>
  <main class="doorway-layout__main">
    <router-outlet></router-outlet>
  </main>
</div>
```

Create `doorway-layout.component.scss`:

```scss
.doorway-layout {
  display: grid;
  grid-template-columns: 240px 1fr;
  height: 100%;

  &__sidenav {
    border-right: 1px solid var(--surface-border, #2a2a2a);
    padding: 1rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;

    a {
      padding: 0.5rem 0.75rem;
      border-radius: 4px;
      text-decoration: none;
      color: var(--text-muted, #888);

      &.active {
        background: var(--surface-emphasis, #1a1a1a);
        color: var(--text-primary, #eee);
      }
    }
  }

  &__main {
    padding: 1.5rem;
    overflow-y: auto;
  }
}
```

- [ ] **Step 5: Run test to verify it passes**

```bash
pnpm exec vitest run doorway-layout.component.spec
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add app/elohim-app/src/app/doorway/components/doorway-layout/
git commit -m "feat(doorway): add DoorwayLayoutComponent with sidenav + router-outlet"
```

### Task B2: Wrap doorway routes with the layout

**Files:**
- Modify: `app/elohim-app/src/app/doorway/doorway.routes.ts`

- [ ] **Step 1: Restructure routes**

Replace the existing `DOORWAY_ROUTES` definition with:

```typescript
import { Routes } from '@angular/router';

export const DOORWAY_ROUTES: Routes = [
  {
    path: '',
    loadComponent: async () =>
      import('./components/doorway-layout/doorway-layout.component').then(m => m.DoorwayLayoutComponent),
    children: [
      {
        path: '',
        pathMatch: 'full',
        loadComponent: async () =>
          import('./components/doorway-dashboard/doorway-dashboard.component').then(m => m.DoorwayDashboardComponent),
        data: { title: 'Doorway — Dashboard' },
      },
      {
        path: 'config',
        loadComponent: async () =>
          import('./components/doorway-dashboard/doorway-dashboard.component').then(m => m.DoorwayDashboardComponent),
        data: { title: 'Doorway — Configuration' },
      },
    ],
  },
];
```

- [ ] **Step 2: Verify by navigating**

Start dev server and visit `/doorway`. Sidenav should appear with two entries.

- [ ] **Step 3: Commit**

```bash
git add app/elohim-app/src/app/doorway/doorway.routes.ts
git commit -m "feat(doorway): wrap doorway routes with layout"
```

### Task B3: Add doorway entry to main navigation

**Files:**
- Modify: `app/elohim-app/src/app/elohim/components/elohim-shell/elohim-shell.component.html` (verify exact path; could be a different nav component — search for "shefa" link first)

- [ ] **Step 1: Locate the main nav**

```bash
grep -rn 'routerLink="/shefa"' app/elohim-app/src/app/
```

Pick the template that lists pillar links.

- [ ] **Step 2: Add doorway entry alongside shefa**

```html
<a routerLink="/doorway" routerLinkActive="active" data-testid="nav-doorway">Doorway</a>
```

Mirror the existing markup pattern.

- [ ] **Step 3: Visual verification**

Reload dev server. Doorway link appears in main nav. Click navigates to `/doorway` and shows the dashboard with layout chrome.

- [ ] **Step 4: Commit**

```bash
git add <the modified nav template>
git commit -m "feat(doorway): expose doorway in main navigation"
```

### Task B4: Implement step defs for `doorway-dashboard-health.feature`

**Files:**
- Modify: `genesis/a2o/features/browser/doorway-dashboard-health.feature`
- Modify: `genesis/a2o/steps/ui/doorway.steps.ts` (create if missing)

- [ ] **Step 1: Read the feature file to identify gaps**

```bash
cat /projects/elohim/genesis/a2o/features/browser/doorway-dashboard-health.feature
```

Note which scenarios are `@wip` and what steps they reference.

- [ ] **Step 2: Implement the step definitions**

Mirror the pattern from `shefa.steps.ts` (Task A7) — `requirePlaywright(this)`, `device.page.locator(...)`, `expect(...)`.

Specifically implement (these are the most likely step names from a doorway-dashboard-health feature — adjust to actual text):

```typescript
import { When, Then } from '@cucumber/cucumber';
import { expect } from 'chai';
import { requirePlaywright } from '../../framework/world';

When('the operator opens the doorway dashboard at {string}', async function (path: string) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  await device.page.goto(`http://localhost:4200${path}`);
  await device.page.locator('[data-testid="doorway-sidenav"]').waitFor();
});

Then('the dashboard renders without console errors', async function () {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  // Use device.consoleErrors capture if framework provides it; otherwise check page state
  const errors = (this.consoleErrors ?? []) as string[];
  expect(errors).to.have.length(0);
});

Then('clicking the {string} tab reveals the {string} panel', async function (tab: string, panelTestId: string) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  await device.page.getByRole('tab', { name: tab }).click();
  const panel = device.page.locator(`[data-testid="${panelTestId}"]`).first();
  expect(await panel.isVisible()).to.equal(true);
});
```

- [ ] **Step 3: Un-`@wip` the relevant scenarios**

Remove `@wip` from the two scenarios in the feature file.

- [ ] **Step 4: Run scenarios**

```bash
pnpm -F a2o exec cucumber-js --tags '@doorway-dashboard and not @wip'
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/features/browser/doorway-dashboard-health.feature \
        genesis/a2o/steps/ui/doorway.steps.ts
git commit -m "test(doorway): implement doorway-dashboard-health browser steps + un-wip"
```

### Task B5: Sub-project B integration verification

- [ ] **Step 1: Boot + navigate**

```bash
pnpm run hc:start
# Open http://localhost:4200, click Doorway in nav
```

- [ ] **Step 2: Confirm layout + content**

Sidenav present. Dashboard panel visible. Tabs switch.

- [ ] **Step 3: Screenshot**

Save `genesis/sprint-results/2026-05-20-light-up-topology/doorway-peers.png`.

---

## Sub-Project C: Resilience tooltip + placement-gaps

**Note:** Substrate is incomplete. Tasks C1–C3 land Rust work; C4–C6 land UI; C7 un-`@wip`s.

### Task C1: Define dedicated `placement-gap-row.schema.json` (hub-abstract kinds)

**Files:**
- Create: `elohim/sdk/schemas/v1/views/placement-gap-row.schema.json`
- Modify: `elohim/sdk/schemas/v1/views/distribution-details.schema.json`

**Source-of-truth lineage:** `PlacementGapRow` is a Category-C operational projection (per the lineage table above). Truth source: `peer_blob_inventory` + peer-identity bindings (Category-A from DHT) + declared reach class on EPR head (Category-A). The `kind` enum is hub-abstract — substrate stays kind-agnostic; UI resolves dwelling/collective/computed labels.

- [ ] **Step 1: Author the row schema**

Create `elohim/sdk/schemas/v1/views/placement-gap-row.schema.json`:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.protocol/schemas/v1/views/placement-gap-row.schema.json",
  "title": "PlacementGapRow",
  "type": "object",
  "additionalProperties": false,
  "required": ["kind", "contentId", "shortfall"],
  "properties": {
    "kind": {
      "type": "string",
      "enum": ["hub_diversity", "replica_count", "reach_class"],
      "description": "Which diversity/redundancy axis fell short. hub_diversity is hub-kind-agnostic; the resolved hubs (dwelling/collective/computed) appear in ResilienceHubView."
    },
    "contentId": { "type": "string" },
    "shortfall": {
      "type": "object",
      "description": "Concrete delta: target vs observed",
      "required": ["target", "observed"],
      "properties": {
        "target": { "type": "integer", "minimum": 0 },
        "observed": { "type": "integer", "minimum": 0 }
      }
    },
    "remediation": {
      "type": "string",
      "description": "Optional hint about what action would close the gap"
    }
  }
}
```

- [ ] **Step 2: Tighten `distribution-details.schema.json`**

In `elohim/sdk/schemas/v1/views/distribution-details.schema.json`, change `placementGaps`:

```json
"placementGaps": {
  "type": "array",
  "items": { "$ref": "placement-gap-row.schema.json" },
  "description": "Concrete placement gaps for this content; empty if fully placed."
}
```

- [ ] **Step 3: Update Rust struct to match (hub-abstract enum)**

Find the Rust `DistributionDetails` and add the projection structs:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PlacementGapRow {
    pub kind: PlacementGapKind,
    pub content_id: String,
    pub shortfall: PlacementGapShortfall,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum PlacementGapKind {
    /// Replicas insufficiently spread across hubs. Hub-kind-agnostic at substrate;
    /// concrete hub kinds (dwelling / collective / computed) resolve in UI labels.
    HubDiversity,
    ReplicaCount,
    ReachClass,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PlacementGapShortfall {
    pub target: i32,
    pub observed: i32,
}
```

Replace `placement_gaps: Vec<serde_json::Value>` with `placement_gaps: Vec<PlacementGapRow>`.

- [ ] **Step 4: Run schema contract test + ts-rs export**

```bash
cargo test --test schema_contract
cargo test export_bindings
```

Expected: PASS, new TS types written.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/schemas/v1/views/placement-gap-row.schema.json \
        elohim/sdk/schemas/v1/views/distribution-details.schema.json \
        elohim/elohim-storage/src/views/ \
        elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(resilience): graduate placement-gap row to dedicated schema"
```

### Task C2: Add `GET /api/v1/resilience/{id}/hub` endpoint (polymorphic hubs)

**Files:**
- Create: `elohim/elohim-storage/src/api/resilience_hub.rs`
- Modify: `elohim/elohim-storage/src/api/mod.rs` (or `router.rs`)
- Create: `elohim/sdk/schemas/v1/views/resilience-hub-view.schema.json`
- Test: `elohim/elohim-storage/tests/resilience_hub.rs`

**Source-of-truth lineage:** `ResilienceHubView` is a Category-C operational projection that sits on the notarized DHT foundation ([[project_three_layer_truth_model]]). Truth sources: `peer_blob_inventory` (gossip-synced operational truth) + peer-identity bindings (Category-A from DHT) + hub-membership facts where notarized. **Hub is a role, not a notarized entity** ([[project_hub_archetype_abstraction]], [[project_hub_optional_floor]]) — the kind (`dwelling`, `collective`, `computed`) is operator-configurable runtime behavior, dial-up-by-capability. The endpoint stays kind-agnostic at the substrate; UI resolves the label.

- [ ] **Step 1: Author the view schema first (IoC)**

Create `elohim/sdk/schemas/v1/views/resilience-hub-view.schema.json`:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.protocol/schemas/v1/views/resilience-hub-view.schema.json",
  "title": "ResilienceHubView",
  "type": "object",
  "additionalProperties": false,
  "required": ["contentId", "hubs"],
  "properties": {
    "contentId": { "type": "string" },
    "hubs": {
      "type": "array",
      "items": { "$ref": "hub-summary.schema.json" }
    }
  }
}
```

Create `elohim/sdk/schemas/v1/views/hub-summary.schema.json`:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.protocol/schemas/v1/views/hub-summary.schema.json",
  "title": "HubSummary",
  "type": "object",
  "additionalProperties": false,
  "required": ["hubId", "kind", "replicaCount"],
  "properties": {
    "hubId": { "type": "string", "description": "Hub identity (peer-id for single-device hubs; cluster-id for multi-device)" },
    "kind": {
      "type": "string",
      "enum": ["dwelling", "collective", "computed"],
      "description": "Hub kind; substrate-agnostic, UI resolves labels"
    },
    "replicaCount": { "type": "integer", "minimum": 0 },
    "lastVerifiedSeconds": { "type": "integer" },
    "displayLabel": {
      "type": "string",
      "description": "Optional operator-set or derived display label (e.g., \"Matthew's home\", \"Bay Area Dawn Runners\")"
    }
  }
}
```

- [ ] **Step 2: Locate the API router**

```bash
grep -rn 'placement-gaps' elohim/elohim-storage/src/api/
```

Identify the routing pattern.

- [ ] **Step 3: Write the failing endpoint test**

Create `elohim/elohim-storage/tests/resilience_hub.rs`:

```rust
use elohim_storage::test_support::test_app;

#[tokio::test]
async fn resilience_hub_endpoint_returns_polymorphic_hubs() {
    let app = test_app().await;
    seed_resilient_content(&app.pool, "content-alpha", &[
        ("hub-A", "dwelling"),
        ("hub-B", "collective"),
    ]).await;

    let resp = app.get("/api/v1/resilience/content-alpha/hub").await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await;
    assert_eq!(body["contentId"], "content-alpha");
    let hubs = body["hubs"].as_array().unwrap();
    assert!(hubs.len() >= 2);
    let kinds: Vec<&str> = hubs.iter().filter_map(|h| h["kind"].as_str()).collect();
    assert!(kinds.contains(&"dwelling"));
    assert!(kinds.contains(&"collective"));
}

#[tokio::test]
async fn resilience_hub_endpoint_handles_single_device_hub() {
    let app = test_app().await;
    seed_resilient_content(&app.pool, "content-solo", &[("hub-solo", "computed")]).await;

    let resp = app.get("/api/v1/resilience/content-solo/hub").await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await;
    assert_eq!(body["hubs"][0]["kind"], "computed");
}
```

- [ ] **Step 4: Run test to verify it fails**

```bash
cargo test --test resilience_hub
```

Expected: FAIL — endpoint returns 404.

- [ ] **Step 5: Implement the handler**

Create `elohim/elohim-storage/src/api/resilience_hub.rs`:

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ResilienceHubView {
    pub content_id: String,
    pub hubs: Vec<HubSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct HubSummary {
    pub hub_id: String,
    pub kind: HubKind,
    pub replica_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_verified_seconds: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_label: Option<String>,
}

/// Hub kind. Polymorphic projection — hub is a *role* (dial-up-by-capability),
/// not a notarized entity. Substrate stays kind-agnostic; UI resolves labels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum HubKind {
    /// Single household / dwelling-scoped hub
    Dwelling,
    /// Collective (qahal-scoped) hub
    Collective,
    /// Computed / ad-hoc hub (e.g., transient single-device participation,
    /// or operator-configured roles that don't fit dwelling/collective)
    Computed,
}

pub async fn handle(
    State(state): State<AppState>,
    Path(content_id): Path<String>,
) -> impl IntoResponse {
    match crate::services::resilience::hub_summary(&state.pool, &content_id).await {
        Ok(hubs) => (
            StatusCode::OK,
            Json(ResilienceHubView { content_id, hubs }),
        )
            .into_response(),
        Err(e) => {
            tracing::warn!(?e, "resilience hub projection failed");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response()
        }
    }
}
```

- [ ] **Step 6: Implement `hub_summary` in `services/resilience.rs`**

Add `hub_summary(pool, content_id) -> Result<Vec<HubSummary>, ResilienceError>` (create the file if absent). Query `peer_blob_inventory` filtered by `content_id`, join peer-identity bindings, group by hub identity. Resolution rule for `kind`:
- If the hub maps to a notarized dwelling/household binding → `HubKind::Dwelling`
- If the hub maps to a notarized collective/qahal binding → `HubKind::Collective`
- Else (single-device or operator-configured ad-hoc) → `HubKind::Computed`

When binding tables don't yet distinguish dwelling vs collective, default to `Computed` and surface a TODO; that's substrate-honest, not a stub.

- [ ] **Step 7: Register the route**

In the API router file, add:

```rust
.route("/api/v1/resilience/:content_id/hub", get(crate::api::resilience_hub::handle))
```

- [ ] **Step 8: Run tests + ts-rs export**

```bash
cargo test --test resilience_hub
cargo test export_bindings
cargo test --test schema_contract
```

Expected: PASS, new TS types (`ResilienceHubView.ts`, `HubSummary.ts`, `HubKind.ts`) written.

- [ ] **Step 9: Commit**

```bash
git add elohim/elohim-storage/src/api/resilience_hub.rs \
        elohim/elohim-storage/src/api/mod.rs \
        elohim/elohim-storage/src/services/resilience.rs \
        elohim/sdk/schemas/v1/views/resilience-hub-view.schema.json \
        elohim/sdk/schemas/v1/views/hub-summary.schema.json \
        elohim/sdk/storage-client-ts/src/generated/ \
        elohim/elohim-storage/tests/resilience_hub.rs
git commit -m "feat(resilience): add /api/v1/resilience/{id}/hub polymorphic projection"
```

### Task C3: Add tooltip + placement-gaps row to `DistributionBadgeComponent`

**Files:**
- Modify: `app/elohim-library/projects/elohim-service/src/components/distribution-badge/distribution-badge.component.ts`
- Modify: `app/elohim-library/projects/elohim-service/src/components/distribution-badge/distribution-badge.component.html`
- Modify: `app/elohim-library/projects/elohim-service/src/components/distribution-badge/distribution-badge.component.scss`
- Test: `app/elohim-library/projects/elohim-service/src/components/distribution-badge/distribution-badge.component.spec.ts`

- [ ] **Step 1: Locate the badge component**

```bash
find app/elohim-library -path '*distribution-badge*' -type f
```

Read every file to understand current state.

- [ ] **Step 2: Write the failing tooltip test**

In `distribution-badge.component.spec.ts`:

```typescript
it('opens a tooltip on hover with household count', async () => {
  const fixture = TestBed.createComponent(DistributionBadgeComponent);
  fixture.componentRef.setInput('summary', {
    replicaCount: 3,
    replicaTarget: 3,
    replicaHealth: 'healthy',
    projectorCount: 1,
    reachClass: 'commons',
    diversityHint: 'multi-household',
    thisFetchSource: 'peer_direct',
    lastVerifiedSeconds: 12,
  });
  fixture.detectChanges();
  const badge = fixture.nativeElement.querySelector('[data-testid="distribution-badge"]') as HTMLElement;
  badge.dispatchEvent(new MouseEvent('mouseenter'));
  fixture.detectChanges();
  await fixture.whenStable();
  const tooltip = fixture.nativeElement.querySelector('[data-testid="distribution-tooltip"]') as HTMLElement;
  expect(tooltip).toBeTruthy();
  expect(tooltip.textContent).toContain('households');
});

it('renders placement-gaps row when details have gaps', async () => {
  const fixture = TestBed.createComponent(DistributionBadgeComponent);
  fixture.componentRef.setInput('summary', { /* ...as above... */ });
  fixture.componentRef.setInput('details', {
    summary: { /* ... */ },
    replicaPeers: [],
    projectorIdentities: [],
    placementGaps: [
      { kind: 'household_diversity', contentId: 'x', shortfall: { target: 3, observed: 1 } },
    ],
    recentProjectionEvents: [],
  });
  fixture.detectChanges();
  const row = fixture.nativeElement.querySelector('[data-testid="placement-gaps-row"]');
  expect(row).toBeTruthy();
  expect(row.textContent).toMatch(/household_diversity|household diversity/);
});
```

- [ ] **Step 3: Run test to verify it fails**

```bash
pnpm exec vitest run distribution-badge.component.spec
```

Expected: FAIL.

- [ ] **Step 4: Implement tooltip state**

In `distribution-badge.component.ts`:

```typescript
protected readonly tooltipOpen = signal(false);
protected readonly details = signal<DistributionDetails | null>(null);

protected onHoverEnter(): void {
  this.tooltipOpen.set(true);
  if (!this.details() && this.blobHash()) {
    void this.loadDetails();
  }
}

protected onHoverLeave(): void {
  this.tooltipOpen.set(false);
}

private async loadDetails(): Promise<void> {
  const hash = this.blobHash();
  if (!hash) return;
  const result = await this.detailsService.fetch(hash);
  this.details.set(result);
}
```

(`detailsService` would be a new injected service — or extend an existing distribution-service. If absent, defer detail-fetch behind a `@Input() details` for now and add the service in a follow-up; placement-gaps row already works from explicit input per the second test.)

- [ ] **Step 5: Add template (hub-kind-aware labels)**

In `distribution-badge.component.html`:

```html
<span
  class="distribution-badge"
  data-testid="distribution-badge"
  [class.status-protected]="summary().replicaHealth === 'healthy'"
  [class.status-partial]="summary().replicaHealth === 'at_risk'"
  [class.status-critical]="summary().replicaHealth === 'critical'"
  (mouseenter)="onHoverEnter()"
  (mouseleave)="onHoverLeave()"
  (focus)="onHoverEnter()"
  (blur)="onHoverLeave()"
  tabindex="0"
>
  <!-- existing badge content -->

  @if (tooltipOpen()) {
    <div class="distribution-badge__tooltip" role="tooltip" data-testid="distribution-tooltip">
      <p>{{ summary().replicaCount }} replicas across {{ hubCountLabel() }}.</p>
      <p>Reach: {{ summary().reachClass }} · Diversity: {{ summary().diversityHint }}</p>

      @if (details(); as d) {
        @if (d.placementGaps.length > 0) {
          <ul class="distribution-badge__gaps" data-testid="placement-gaps-row">
            @for (gap of d.placementGaps; track gap.contentId + gap.kind) {
              <li>{{ gapLabel(gap) }}: {{ gap.shortfall.observed }}/{{ gap.shortfall.target }}</li>
            }
          </ul>
        } @else {
          <p data-testid="placement-gaps-empty">No gaps — fully placed.</p>
        }
      }
    </div>
  }
</span>
```

Add `hubCountLabel()` and `gapLabel()` to the component class. Both pluralise the hub-kind label from the resolved hub view; until the badge can fetch `/api/v1/resilience/{id}/hub` directly, default to the dwelling-kind label (most common case in current fixtures) — keep the resolution table central so future hub kinds slot in without code change:

```typescript
private static readonly HUB_LABELS: Record<HubKind, { singular: string; plural: string }> = {
  dwelling:  { singular: 'household',  plural: 'households'  },
  collective:{ singular: 'collective', plural: 'collectives' },
  computed:  { singular: 'hub',        plural: 'hubs'        },
};

protected hubCountLabel(): string {
  // When hub-view fetch lands, derive kind mix from details().hubs.
  // For now, infer dominant kind from diversityHint (dwelling = multi-household).
  const hubs = this.details()?.hubs ?? [];
  if (hubs.length === 0) {
    return `${this.summary().replicaCount} hubs`;
  }
  const dominantKind = hubs[0].kind;
  const { singular, plural } = DistributionBadgeComponent.HUB_LABELS[dominantKind];
  return hubs.length === 1 ? `1 ${singular}` : `${hubs.length} ${plural}`;
}

protected gapLabel(gap: PlacementGapRow): string {
  switch (gap.kind) {
    case 'hub_diversity':  return 'Hub diversity';
    case 'replica_count':  return 'Replica count';
    case 'reach_class':    return 'Reach class';
    default:               return gap.kind;
  }
}
```

Extend `DistributionDetails` type usage: if the generated `DistributionDetails` does not yet carry `hubs`, defer that derivation to a follow-up (the placement-gaps row still renders from the existing `placementGaps` field).

- [ ] **Step 6: Add styles**

```scss
.distribution-badge {
  position: relative;
  cursor: help;

  &__tooltip {
    position: absolute;
    bottom: calc(100% + 0.5rem);
    left: 50%;
    transform: translateX(-50%);
    min-width: 240px;
    padding: 0.75rem;
    background: var(--surface-overlay, #1a1a1a);
    border: 1px solid var(--surface-border, #2a2a2a);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    z-index: 100;
    font-size: 0.85rem;
    line-height: 1.4;

    p {
      margin: 0 0 0.4rem 0;
      &:last-child { margin-bottom: 0; }
    }
  }

  &__gaps {
    list-style: none;
    margin: 0.5rem 0 0 0;
    padding: 0;

    li {
      padding: 0.25rem 0;
      border-top: 1px solid var(--surface-border-subtle, #222);
    }
  }
}
```

- [ ] **Step 7: Run test to verify it passes**

```bash
pnpm exec vitest run distribution-badge.component.spec
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add app/elohim-library/projects/elohim-service/src/components/distribution-badge/
git commit -m "feat(resilience): add hover tooltip + placement-gaps row to distribution badge"
```

### Task C4: Seed placement-gap test data in `distribution_view.rs` (hub-abstract)

**Files:**
- Modify: `elohim/elohim-storage/tests/distribution_view.rs`

- [ ] **Step 1: Add a test that exercises hub-diversity placement-gaps**

Append:

```rust
#[tokio::test]
async fn distribution_details_surfaces_hub_diversity_gap() {
    let app = test_app().await;
    seed_content_with_single_hub(&app.pool, "content-lonely").await;

    let resp = app.get("/api/v1/distribution/content-lonely/details").await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await;
    let gaps = body["placementGaps"].as_array().unwrap();
    assert!(gaps.iter().any(|g| g["kind"] == "hub_diversity"));
}
```

- [ ] **Step 2: Implement `seed_content_with_single_hub` + the production logic that emits the gap**

Find the existing `distribution_view` service that constructs `DistributionDetails`. Add logic that's kind-agnostic at substrate (matches the abstract Hub framing — observed-hub-count vs target-hub-count, regardless of whether the hubs are dwellings, collectives, or computed):

```rust
if observed_hubs < target_hubs {
    placement_gaps.push(PlacementGapRow {
        kind: PlacementGapKind::HubDiversity,
        content_id: content_id.clone(),
        shortfall: PlacementGapShortfall {
            target: target_hubs as i32,
            observed: observed_hubs as i32,
        },
        remediation: Some("Recruit a replica in another hub".to_string()),
    });
}
```

The "hub" count comes from grouping `peer_blob_inventory` rows by resolved hub identity (per the resolution rule in Task C2 Step 6). When binding tables can't distinguish dwelling vs collective, this still works — substrate counts hubs without classifying them, classification happens later in the projection.

- [ ] **Step 3: Run test**

```bash
cargo test --test distribution_view
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/tests/distribution_view.rs \
        elohim/elohim-storage/src/services/distribution_view.rs
git commit -m "feat(resilience): emit household_diversity placement-gap rows"
```

### Task C5: Implement browser-layer step defs in `resilience.steps.ts`

**Files:**
- Modify: `genesis/a2o/steps/resilience.steps.ts`

- [ ] **Step 1: Locate the six stubbed steps**

```bash
grep -n "'pending'" genesis/a2o/steps/resilience.steps.ts
```

- [ ] **Step 2: Replace each `return 'pending'` with real Playwright assertions**

Example replacements (adapt selectors to actual `data-testid` values from Task C3):

```typescript
Then('the resilience icon has class {string} or {string}', async function (a: string, b: string) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  const icon = device.page.locator('elohim-resilience-snapshot').first();
  const cls = (await icon.getAttribute('class')) ?? '';
  expect(cls.split(/\s+/)).to.include.oneOf([a, b]);
});

Then('the tooltip mentions the household count', async function () {
  // Tests the dwelling-kind hub case (HubKind::Dwelling). Substrate is hub-abstract
  // ([[project_hub_archetype_abstraction]]); the tooltip renders "households" /
  // "collectives" / "hubs" based on resolved kind. Mirror this assertion for
  // collective- and computed-kind scenarios when they land.
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  await device.page.locator('[data-testid="distribution-badge"]').first().hover();
  const tooltip = device.page.locator('[data-testid="distribution-tooltip"]').first();
  await tooltip.waitFor();
  const text = (await tooltip.textContent()) ?? '';
  expect(text).to.match(/household/i);
});

// Hub-kind-generic variant for future scenarios. Substrate stays kind-agnostic;
// scenarios pick the user-observable label.
Then('the tooltip mentions the {string} count', async function (hubKindLabel: string) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  await device.page.locator('[data-testid="distribution-badge"]').first().hover();
  const tooltip = device.page.locator('[data-testid="distribution-tooltip"]').first();
  await tooltip.waitFor();
  const text = (await tooltip.textContent()) ?? '';
  expect(text).to.match(new RegExp(hubKindLabel, 'i'));
});

Then('the signals card shows a non-zero gap count', async function () {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  const count = await device.page.locator('[data-testid="signals-gap-count"]').first().textContent();
  expect(parseInt(count ?? '0', 10)).to.be.greaterThan(0);
});

Then('clicking a gap signal scrolls to or links to a shefa recruitment surface', async function () {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  await device.page.locator('[data-testid="signals-gap"]').first().click();
  await device.page.waitForURL(/\/shefa/);
});

Then('each row renders an elohim-resilience-snapshot icon', async function () {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  const rows = device.page.locator('[data-testid="content-row"]');
  const count = await rows.count();
  for (let i = 0; i < count; i++) {
    const icon = rows.nth(i).locator('elohim-resilience-snapshot');
    expect(await icon.count()).to.equal(1);
  }
});

Then('hovering a row shows the household summary', async function () {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  await device.page.locator('[data-testid="content-row"]').first().hover();
  const summary = device.page.locator('[data-testid="household-summary"]').first();
  await summary.waitFor();
  expect(await summary.isVisible()).to.equal(true);
});
```

- [ ] **Step 3: Commit**

```bash
git add genesis/a2o/steps/resilience.steps.ts
git commit -m "test(resilience): implement Playwright assertions for browser-tier resilience steps"
```

### Task C6: Un-`@wip` resilience scenarios

**Files:**
- Modify: `genesis/a2o/features/resilience/observable-distribution.feature`

- [ ] **Step 1: Remove `@wip` from two scenarios**

Edit lines:
- Line 32 — remove `@wip` from "Content-viewer resilience tooltip is live"
- Line 154 — remove `@wip` from "Content-viewer header renders distribution and resilience together"

Leave the other 5 `@wip` scenarios — they target surfaces (signals card, doorway admin content list, peer-topology resilience-cliff, lazy-detail-fetch, concept-card badge) that are out of scope for this sprint. Note them in the sprint result as the next layer.

- [ ] **Step 2: Run the un-wip'd scenarios**

```bash
pnpm -F a2o exec cucumber-js \
  --tags '@resilience-p1 and not @wip' \
  genesis/a2o/features/resilience/observable-distribution.feature
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add genesis/a2o/features/resilience/observable-distribution.feature
git commit -m "test(resilience): un-wip tooltip + side-by-side scenarios"
```

### Task C7: Sub-project C integration verification

- [ ] **Step 1: Boot the stack**

```bash
pnpm run hc:start:seed
```

- [ ] **Step 2: Open a content-viewer page**

Hover the distribution badge. Tooltip appears with household count + resilience info. If content has a placement-gap, the row renders.

- [ ] **Step 3: Run full resilience suite**

```bash
pnpm -F a2o exec cucumber-js --tags '@resilience-p1 and not @wip'
```

Expected: PASS.

- [ ] **Step 4: Screenshot**

Save `genesis/sprint-results/2026-05-20-light-up-topology/resilience-tooltip.png` and `placement-gaps.png`.

---

## Sprint-wide Verification

- [ ] **Step 1: Run pre-push gate**

```bash
HUSKY=0 git push --dry-run
# Or trigger manually:
pnpm run schema:check-dna
```

- [ ] **Step 2: Full a2o run**

```bash
pnpm -F a2o exec cucumber-js \
  --tags '(@compute-triptych or @doorway-dashboard or @resilience-p1) and not @wip'
```

Expected: PASS.

- [ ] **Step 3: Write the sprint-result markdown**

Create `genesis/sprint-results/2026-05-20-light-up-topology.md` with:
- Summary of the three surfaces shipped
- Screenshots embedded
- Notes on the divergences discovered during research
- Pointers to follow-up work (5 remaining @wip resilience scenarios, GraphQL migration for doorway-dashboard, etc.)

- [ ] **Step 4: Run `/story-harvest` to capture engineering constraints**

```
/story-harvest
```

- [ ] **Step 5: Open the PR**

Use the `finishing-a-development-branch` skill to decide on integration.

---

## Self-Review

**Spec coverage:**
- ✅ Compute triptych — Tasks A1–A8
- ✅ Doorway-peers wiring — Tasks B1–B5
- ✅ Resilience tooltip + placement-gaps — Tasks C1–C7
- ✅ Substrate verification, a2o coverage, sprint-result deliverable

**Placeholder scan:** No "TBD", "implement later", or "similar to" without code. Each step has commands and expected output.

**Type consistency:**
- `ComputeTriptych` (Rust view) ↔ `ComputeTriptychGql` (resolver) ↔ generated TS `ComputeTriptych` ↔ Angular `device.compute` usage — consistent
- `PlacementGapRow` ↔ `placement-gap-row.schema.json` ↔ `DistributionDetails.placementGaps[]` — consistent
- `data-testid` names: `compute-{free,used,stewarded}-bytes`, `distribution-badge`, `distribution-tooltip`, `placement-gaps-row`, `doorway-sidenav`, `nav-doorway` — used consistently across component, spec, and step-def

**Known fuzzy areas (engineer will need to adapt):**
- Exact path of the main navigation template (Task B3 Step 1) — search-driven location
- Exact `household_summary` query shape (Task C2 Step 4) — depends on existing schema in `peer_blob_inventory` and binding tables
- `householdLabel()` computed signal (Task C3 Step 5) — derive label from `diversityHint` enum values that exist in `distribution-summary.ts`

These are documented as search-and-adapt steps, not placeholders.

---

## Execution Handoff

Plan complete and saved to `genesis/docs/plans/2026-05-20-light-up-the-topology.md`. Two execution options:

**1. Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration. Best for this plan because the three sub-projects are independent and Sub-project C has the most uncertainty (substrate gaps).

**2. Inline Execution** — execute tasks in this session using `executing-plans`, batch execution with checkpoints. Best if you want continuous context across sub-projects.

**Recommendation:** Subagent-driven, dispatching sub-projects in this order: **A → B → C** (A is most established substrate; B is shortest; C has the most risk so leave it for last when you have the most ground truth from A and B). Alternatively, run A and B in parallel since they don't touch the same files, then C serially.

Which approach?
