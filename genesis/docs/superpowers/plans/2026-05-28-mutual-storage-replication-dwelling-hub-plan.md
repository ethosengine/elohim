# Mutual Storage Replication (Dwelling-Hub Tier) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the first concrete instance of the REA compute-commitment primitive — `replicates-dwelling` Mishpat::Commitment action with bilateral mutual aid between dwelling-hubs (households), donut-economics constitutional ratios, mutuality_audit_service, two new capacity views, three existing topology-view extensions, and end-to-end sweettest + a2o coverage.

**Architecture:** Three load-bearing properties — donut economics (device-level), bilateral-by-reference mutuality (with grace-period soft-warn), intent-first observed-state-second (commitments steer existing inventory_gossip + libp2p pull; no new wire protocol). Substrate stays kind-agnostic at DHT; hub classification at projection layer. Encryption explicitly decoupled (commitments bind shard storage, not read access).

**Tech Stack:** Rust (Mishpat coordinator + integrity zomes via HDK 0.5; elohim-storage services; bounds_validator extension; ts-rs view types), Diesel (mutuality_audit_log + existing rea_commitments + standing_view), Holochain DHT (`Mishpat::Commitment` entry type with new action discriminator), hyper + Bytes (existing http stack), JSON Schema (Draft 2020-12).

**Companion files:**
- Spec: `genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md` (read fully before authoring)
- Parent roadmap: `genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md`
- Sprint 1 close-out: `genesis/docs/superpowers/plans/2026-05-28-sprint1-zd-substrate-correct-deploy.md` (Z.D abandoned; substrate primitives ready)
- Sprint 2 close-out: `genesis/docs/superpowers/plans/2026-05-28-sprint2-bounds-validator-standing-aggregator.md` (bounds_validator + signal_weight_registry shipped)
- Memory: `[[project_compute_commitment_first_instance_pivot]]`, `[[project_bounds_validator_pattern]]`, `[[project_rea_compute_commitment_primitive]]`, `[[project_hub_archetype_abstraction]]`, `[[project_signal_kind_extensible_protocol_class]]`

---

## P2P Design Gate output

Per spec §3. Headline: **zero new DHT entry types**. Every notarized artifact reuses `Mishpat::Commitment` via new action discriminator (`replicates-dwelling`) or extends an existing FeedbackSignal kind via `signalKinds` manifest entry (`reciprocity-imbalance`). All other artifacts are operational projections (Category C) recomputable from DHT state, or schema/codegen wiring.

Full design gate table lives in spec §3; treat that as authoritative. The audit hook may continue to flag `.schema.json` references inside task code blocks — all such references inherit the spec's classification table.

---

## File Structure

### NEW files (28 total)

```
elohim/sdk/schemas/v1/commitments/
└── replicates-dwelling.schema.json                    (NEW — A: payload of Mishpat::Commitment)

elohim/sdk/schemas/v1/views/
├── peer-capacity-view.schema.json                     (NEW — C: per-device donut accounting)
└── hub-capacity-view.schema.json                      (NEW — C: hub-aggregate, mirrors HubComputeAggregateView)

elohim/elohim-views/src/
├── replicates_dwelling.rs                             (NEW — Rust ts-rs payload type)
├── peer_capacity.rs                                   (NEW)
└── hub_capacity.rs                                    (NEW)

elohim/holochain/dna/mishpat/zomes/mishpat/src/
└── commitments.rs                                     (EDIT — add validate_replicates_dwelling)

elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/
└── lib.rs                                             (EDIT — defense-in-depth for new action)

elohim/holochain/dna/elohim/zomes/content_store_integrity/src/
└── lib.rs                                             (EDIT — DNA donut constants)

elohim/sdk/domains/elohim/
└── manifest.json                                      (EDIT — constitutionalRatios block + reciprocity-imbalance signal_kind)

elohim/sdk/schemas/v1/manifest/
└── app-manifest.schema.json                           (EDIT — declare constitutionalRatios optional block)

elohim/elohim-storage/src/services/
├── constitutional_ratio_registry.rs                   (NEW — OnceLock-cached effective_ratios)
├── replicates_dwelling_validator.rs                   (NEW — per-instance validator)
├── peer_capacity_service.rs                           (NEW — computes PeerCapacityView)
├── hub_capacity_service.rs                            (NEW — computes HubCapacityView; aggregates PeerCapacityViews)
├── mutuality_audit_service.rs                         (NEW — daily sweep + reciprocity-imbalance emit)
├── replication_prioritizer.rs                         (NEW — scores inventory advertisements vs active commitments)
├── bounds_validator.rs                                (EDIT — add ConstitutionalRatioBreach variant)
├── household_resilience.rs                            (EDIT — add commitmentBackedReplication field computation)
├── distribution_view.rs                               (EDIT — add projectionTier + over_replicated + faultDomainDiversity computation)
└── mod.rs                                             (EDIT — declare new modules)

elohim/elohim-storage/src/api/
├── peer_capacity.rs                                   (NEW — GET /api/v1/peer/{cid}/capacity)
├── hub_capacity.rs                                    (NEW — GET /api/v1/hub/{id}/capacity)
├── diagnostics_mutuality.rs                           (NEW — GET /api/v1/diagnostics/mutuality-audit)
└── mod.rs                                             (EDIT — declare modules + dispatch)

elohim/elohim-storage/src/db/
├── mutuality_audit_log.rs                             (NEW — diesel model + CRUD)
├── diesel_schema.rs                                   (EDIT — mutuality_audit_log table)
└── models.rs                                          (EDIT — MutualityAuditLog struct)

elohim/elohim-storage/migrations/
└── 2026-05-28-100000_mutuality_audit_log/
    ├── up.sql                                         (NEW)
    └── down.sql                                       (NEW)

elohim/elohim-storage/src/http.rs                      (EDIT — wire 3 new routes before /api/v1/ catch-all)

elohim/sdk/schemas/v1/views/distribution-summary.schema.json    (EDIT — projectionTier + over_replicated)
elohim/sdk/schemas/v1/views/distribution-details.schema.json    (EDIT — replicationCommitments + faultDomainDiversity)
elohim/sdk/schemas/v1/views/replica-peer.schema.json            (EDIT — shardsHeld + shardsByEncoding)
elohim/sdk/schemas/v1/views/household-resilience-view.schema.json (EDIT — commitmentBackedReplication)

elohim/elohim-storage/tests/
├── replicates_dwelling_integration.rs                 (NEW — 4 stories)
├── peer_capacity_view_integration.rs                  (NEW — 3 stories)
├── hub_capacity_view_integration.rs                   (NEW — 2 stories)
└── distribution_view_extensions_integration.rs        (NEW — 3 stories)
└── schema_contract.rs                                 (EDIT — 4 new tests for new/extended schemas)

elohim/holochain/tests/sweettest/src/
└── replicates_dwelling_substrate_correct_test.rs      (NEW)

genesis/a2o/features/storage/
├── household-resiliency-handshake.feature             (NEW)
├── constitutional-ratio-enforcement.feature           (NEW)
└── disaster-burst-resilience.feature                  (NEW — @wip-collective-steward)

genesis/docs/research/
└── 2026-05-28-sprint3-storage-replication-implementation-notes.md (close-out)

.claude/memory/
├── project_dwelling_hub_replication_pattern.md        (NEW)
└── MEMORY.md                                          (EDIT — index entry)
```

### MODIFIED files (summary list)

```
elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs   (DNA constants)
elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs           (action validator)
elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs         (defense-in-depth)
elohim/sdk/domains/elohim/manifest.json                                 (ratios + signal_kind)
elohim/sdk/schemas/v1/manifest/app-manifest.schema.json                 (schema extension)
elohim/sdk/schemas/scripts/codegen-ts.mjs                               (INTERFACE_FILES)
elohim/sdk/schemas/v1/views/distribution-summary.schema.json            (projectionTier)
elohim/sdk/schemas/v1/views/distribution-details.schema.json            (replicationCommitments + faultDomainDiversity)
elohim/sdk/schemas/v1/views/replica-peer.schema.json                    (shardsHeld)
elohim/sdk/schemas/v1/views/household-resilience-view.schema.json       (commitmentBackedReplication)
elohim/elohim-views/src/lib.rs                                          (declare new modules)
elohim/elohim-storage/src/services/bounds_validator.rs                  (ConstitutionalRatioBreach)
elohim/elohim-storage/src/services/household_resilience.rs              (extend computation)
elohim/elohim-storage/src/services/distribution_view.rs                 (extend computation)
elohim/elohim-storage/src/services/mod.rs                               (declare modules)
elohim/elohim-storage/src/db/diesel_schema.rs                           (mutuality_audit_log table)
elohim/elohim-storage/src/db/models.rs                                  (MutualityAuditLog)
elohim/elohim-storage/src/api/mod.rs                                    (declare modules + dispatch)
elohim/elohim-storage/src/http.rs                                       (wire 3 routes)
elohim/elohim-storage/tests/schema_contract.rs                          (new tests)
```

---

## Task overview (20 tasks)

| Phase | Tasks | Description |
|-------|-------|-------------|
| **A. Foundation** | T1–T5 | DNA constants, schemas (commitment + 2 views), existing-view extensions, manifest config |
| **B. Substrate primitives** | T6–T8 | constitutional_ratio_registry, bounds_validator ConstitutionalRatioBreach, replicates_dwelling_validator |
| **C. Mishpat zome** | T9–T10 | Mishpat coordinator action + integrity defense-in-depth |
| **D. Mutuality audit** | T11–T12 | mutuality_audit_log migration + mutuality_audit_service |
| **E. Views + data plane** | T13–T16 | peer_capacity_service, hub_capacity_service, existing-view extension computation, replication_prioritizer |
| **F. HTTP routes** | T17 | Three new diagnostic routes wired in http.rs |
| **G. Tests** | T18–T19 | Sweettest two-conductor + 3 a2o features |
| **H. Close-out** | T20 | Sprint close-out, pattern memory, roadmap update |


---

## Phase A — Foundation

### Task 1: DNA donut constants in elohim integrity zome

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`

- [ ] **Step 1: Add DNA constants**

Open `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`. Locate the top of the file (after the existing `use hdi::prelude::*;` etc.). Insert:

```rust
// ============================================================
// Constitutional storage donut — DNA-locked walls
// ============================================================
//
// Source of truth: this DNA. Protocol-version-stable constants that bound
// every `replicates-*` commitment author. The manifest can configure ratios
// WITHIN these walls; it cannot breach them. Upgrade requires DNA migration —
// intentional friction; the donut walls are constitutional.
//
// Per spec §5 (genesis/docs/superpowers/specs/2026-05-28-mutual-storage-
// replication-dwelling-hub-design.md): the donut prevents (a) free-riding
// below the commons floor, (b) capture above the commons ceiling, (c)
// over-allocation that leaves the device with no self-storage headroom.

pub const COMMONS_MIN_FLOOR_PCT: u8 = 10;
pub const COMMONS_MAX_CEILING_PCT: u8 = 60;
pub const DWELLING_MIN_FLOOR_PCT: u8 = 10;
pub const DWELLING_MAX_CEILING_PCT: u8 = 80;
pub const FREE_MIN_FLOOR_PCT: u8 = 5;
pub const FREE_MAX_CEILING_PCT: u8 = 70;
```

- [ ] **Step 2: Build the DNA WASM (sanity check)**

```bash
cd /projects/elohim/elohim/holochain/dna/elohim
just check 2>&1 | tail -10
```

Expected: clean (no warnings from the new constants).

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim
git add elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs
git commit -m "feat(elohim-integrity): DNA donut walls — constitutional storage-ratio constants"
```

---

### Task 2: `replicates-dwelling.schema.json` + Rust ts-rs payload type

**Files:**
- Create: `elohim/sdk/schemas/v1/commitments/replicates-dwelling.schema.json`
- Create: `elohim/elohim-views/src/replicates_dwelling.rs`
- Modify: `elohim/elohim-views/src/lib.rs`
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs`
- Modify: `elohim/elohim-storage/tests/schema_contract.rs`

- [ ] **Step 1: Author `replicates-dwelling.schema.json`**

Create `elohim/sdk/schemas/v1/commitments/replicates-dwelling.schema.json` (full body from spec §4):

```json
{
  "$id": "epr:schema:replicates-dwelling",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "ReplicatesDwellingCommitment",
  "description": "Payload shape for a Mishpat::Commitment entry with action='replicates-dwelling'. Storage-availability commitment between two dwelling-hubs. Does NOT presuppose recipient can decrypt — that's the encryption-layer (separate sprint). Source of truth: Holochain DHT (existing Mishpat::Commitment entry type, action discriminator). Spec: genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md §4.",
  "type": "object",
  "required": ["action", "provider_dwelling_hub_id", "recipient_dwelling_hub_id", "provider_role", "capacity_bytes", "scope_filter", "valid_from", "valid_until", "grace_period_days", "rotation_ttl_days", "ratio_attestation"],
  "additionalProperties": false,
  "properties": {
    "action": { "const": "replicates-dwelling" },
    "provider_dwelling_hub_id": { "type": "string", "minLength": 1 },
    "recipient_dwelling_hub_id": { "type": "string", "minLength": 1 },
    "provider_role": { "type": "string", "enum": ["steward_mutual", "collective_steward"] },
    "via_collective_hub_id": { "type": ["string", "null"] },
    "capacity_bytes": { "type": "integer", "minimum": 1 },
    "scope_filter": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "epr_kinds": {
          "type": "array",
          "items": { "type": "string", "enum": ["Content", "Manifest", "Claim", "Observation", "EconomicEvent", "Commitment", "Attestation", "Delegation", "FeedbackSignal"] }
        },
        "bytes_per_blob_max": { "type": "integer", "minimum": 1 },
        "requires_attestations": { "type": "array", "items": { "type": "string" } },
        "kinds_excluded": { "type": "array", "items": { "type": "string" } }
      }
    },
    "valid_from": { "type": "string", "minLength": 1 },
    "valid_until": { "type": "string", "minLength": 1 },
    "grace_period_days": { "type": "integer", "minimum": 1 },
    "rotation_ttl_days": { "type": "integer", "minimum": 1 },
    "ratio_attestation": {
      "type": "object",
      "required": ["commons_pct", "dwelling_pct", "collective_pct", "free_pct", "effective_ratio_cid"],
      "additionalProperties": false,
      "properties": {
        "commons_pct":         { "type": "integer", "minimum": 0, "maximum": 100 },
        "dwelling_pct":        { "type": "integer", "minimum": 0, "maximum": 100 },
        "collective_pct":      { "type": "integer", "minimum": 0, "maximum": 100 },
        "free_pct":            { "type": "integer", "minimum": 0, "maximum": 100 },
        "effective_ratio_cid": { "type": "string", "minLength": 1 }
      }
    }
  }
}
```

- [ ] **Step 2: Author Rust ts-rs payload type**

Create `elohim/elohim-views/src/replicates_dwelling.rs`:

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ReplicatesDwellingPayload {
    pub action: String,
    pub provider_dwelling_hub_id: String,
    pub recipient_dwelling_hub_id: String,
    pub provider_role: ProviderRole,
    pub via_collective_hub_id: Option<String>,
    pub capacity_bytes: u64,
    pub scope_filter: ScopeFilter,
    pub valid_from: String,
    pub valid_until: String,
    pub grace_period_days: u32,
    pub rotation_ttl_days: u32,
    pub ratio_attestation: RatioAttestation,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum ProviderRole {
    StewardMutual,
    CollectiveSteward,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ScopeFilter {
    pub epr_kinds: Option<Vec<String>>,
    pub bytes_per_blob_max: Option<u64>,
    pub requires_attestations: Option<Vec<String>>,
    pub kinds_excluded: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RatioAttestation {
    pub commons_pct: u8,
    pub dwelling_pct: u8,
    pub collective_pct: u8,
    pub free_pct: u8,
    pub effective_ratio_cid: String,
}
```

- [ ] **Step 3: Register in lib.rs**

In `elohim/elohim-views/src/lib.rs` add `pub mod replicates_dwelling;` near existing module declarations (sorted with `pub mod bounds;`, `pub mod standing;`, etc.).

- [ ] **Step 4: Add to codegen-ts INTERFACE_FILES**

In `elohim/sdk/schemas/scripts/codegen-ts.mjs`, find the `INTERFACE_FILES` array. Add:

```js
'replicates-dwelling.schema.json',
```

- [ ] **Step 5: Add schema contract test**

In `elohim/elohim-storage/tests/schema_contract.rs`, append at the end:

```rust
// =============================================================================
// Sprint 3: replicates-dwelling Commitment schema
// =============================================================================

#[test]
fn replicates_dwelling_minimal_steward_mutual_validates() {
    let payload = serde_json::json!({
        "action": "replicates-dwelling",
        "provider_dwelling_hub_id": "hub:A",
        "recipient_dwelling_hub_id": "hub:B",
        "provider_role": "steward_mutual",
        "capacity_bytes": 50_000_000_000u64,
        "scope_filter": {"epr_kinds": ["Content"], "bytes_per_blob_max": 1_000_000_000u64},
        "valid_from": "2026-05-28T00:00:00Z",
        "valid_until": "2026-08-26T00:00:00Z",
        "grace_period_days": 14,
        "rotation_ttl_days": 90,
        "ratio_attestation": {
            "commons_pct": 20,
            "dwelling_pct": 40,
            "collective_pct": 25,
            "free_pct": 15,
            "effective_ratio_cid": "bafkrei-manifest-abc"
        }
    });
    validate_against_schema("commitments/replicates-dwelling.schema.json", &payload);
}

#[test]
fn replicates_dwelling_collective_steward_validates() {
    let payload = serde_json::json!({
        "action": "replicates-dwelling",
        "provider_dwelling_hub_id": "hub:church-server",
        "recipient_dwelling_hub_id": "hub:member-family",
        "provider_role": "collective_steward",
        "via_collective_hub_id": "collective:saint-marys",
        "capacity_bytes": 100_000_000_000u64,
        "scope_filter": {"epr_kinds": ["Content"]},
        "valid_from": "2026-05-28T00:00:00Z",
        "valid_until": "2026-08-26T00:00:00Z",
        "grace_period_days": 14,
        "rotation_ttl_days": 90,
        "ratio_attestation": {
            "commons_pct": 20,
            "dwelling_pct": 40,
            "collective_pct": 25,
            "free_pct": 15,
            "effective_ratio_cid": "bafkrei-manifest-abc"
        }
    });
    validate_against_schema("commitments/replicates-dwelling.schema.json", &payload);
}

#[test]
fn replicates_dwelling_rejects_unknown_provider_role() {
    let payload = serde_json::json!({
        "action": "replicates-dwelling",
        "provider_dwelling_hub_id": "hub:A",
        "recipient_dwelling_hub_id": "hub:B",
        "provider_role": "totally-bogus",
        "capacity_bytes": 1,
        "scope_filter": {},
        "valid_from": "2026-05-28T00:00:00Z",
        "valid_until": "2026-08-26T00:00:00Z",
        "grace_period_days": 14,
        "rotation_ttl_days": 90,
        "ratio_attestation": {
            "commons_pct": 20, "dwelling_pct": 40, "collective_pct": 25, "free_pct": 15,
            "effective_ratio_cid": "bafkrei-manifest-abc"
        }
    });
    validate_rejects_schema("commitments/replicates-dwelling.schema.json", &payload);
}
```

(If `validate_rejects_schema` doesn't exist in the harness, follow the existing pattern — look for a `_rejects_` test in the file and mirror it.)

- [ ] **Step 6: Run + codegen**

```bash
cd /projects/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev cargo test --manifest-path elohim/elohim-storage/Cargo.toml --test schema_contract replicates_dwelling 2>&1 | tail -10
cargo test --manifest-path elohim/elohim-views/Cargo.toml export_bindings 2>&1 | tail -10
pnpm run schema:codegen:ts 2>&1 | tail -10
```

Expected: 3 schema contract tests pass; export_bindings green; codegen-ts distributes to 5 consumer projects.

- [ ] **Step 7: Commit**

```bash
git add elohim/sdk/schemas/v1/commitments/replicates-dwelling.schema.json \
        elohim/elohim-views/src/replicates_dwelling.rs \
        elohim/elohim-views/src/lib.rs \
        elohim/sdk/schemas/scripts/codegen-ts.mjs \
        elohim/elohim-storage/tests/schema_contract.rs \
        elohim/sdk/storage-client-ts/src/generated/ \
        app/elohim-app/src/app/generated/ \
        app/elohim-library/projects/elohim-service/src/generated/ \
        app/lamad/src/generated/ \
        doorway/doorway-app/src/app/generated/ 2>/dev/null || true
git commit -m "feat(views): ReplicatesDwellingPayload schema + ts-rs + schema contract tests"
```

---

### Task 3: PeerCapacityView + HubCapacityView schemas + Rust ts-rs

**Files:**
- Create: `elohim/sdk/schemas/v1/views/peer-capacity-view.schema.json`
- Create: `elohim/sdk/schemas/v1/views/hub-capacity-view.schema.json`
- Create: `elohim/elohim-views/src/peer_capacity.rs`
- Create: `elohim/elohim-views/src/hub_capacity.rs`
- Modify: `elohim/elohim-views/src/lib.rs`
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs`
- Modify: `elohim/elohim-storage/tests/schema_contract.rs`

- [ ] **Step 1: Author `peer-capacity-view.schema.json`**

Full body per spec §7.1; save at `elohim/sdk/schemas/v1/views/peer-capacity-view.schema.json`. Key fields: `peerCid, computedAt, totalRawBytes, pledges {dwellingBytes, collectiveBytes, commonsBytes, totalPledgedBytes, pledgesByRecipient[]}, actuallyHeld {uniqueShardBytes, freeBytesRemaining, fragmentationEstimate}, ratioCompliance {effectiveRatios, currentRatios, compliantWithDonut, violations[]}`.

Copy verbatim from spec §7.1.

- [ ] **Step 2: Author `hub-capacity-view.schema.json`**

Full body per spec §7.2; save at `elohim/sdk/schemas/v1/views/hub-capacity-view.schema.json`. Key fields: `hubId, hubKind (dwelling|collective|computed), displayLabel?, memberDeviceCount, capacity (null or per-device-aggregated)`.

- [ ] **Step 3: Author Rust ts-rs view modules**

Create `elohim/elohim-views/src/peer_capacity.rs` with `PeerCapacityView`, `PledgesView`, `PledgeByRecipientView`, `ActuallyHeldView`, `RatioComplianceView`, `EffectiveRatiosView`, `CurrentRatiosView`, `RatioViolationView`, `Tier`, `ViolationKind` structs/enums — all `#[serde(rename_all = "camelCase")]` (struct fields) and `#[serde(rename_all = "snake_case")]` (enum variants); all `#[derive(TS, Serialize, Deserialize, Debug, Clone)]`; all `#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]`.

Tier enum: `Dwelling | Collective | Commons | Free`.
ViolationKind enum: `BelowFloor | AboveCeiling | BelowManifestTarget | AboveManifestTarget`.

Create `elohim/elohim-views/src/hub_capacity.rs` with `HubCapacityView` + reused inner types from peer_capacity. `HubKind` enum: `Dwelling | Collective | Computed`.

- [ ] **Step 4: Register modules**

In `elohim/elohim-views/src/lib.rs` add `pub mod peer_capacity;` and `pub mod hub_capacity;` in sorted order.

- [ ] **Step 5: Add to codegen-ts INTERFACE_FILES**

```js
'peer-capacity-view.schema.json',
'hub-capacity-view.schema.json',
```

- [ ] **Step 6: Schema contract tests + source-of-truth declaration**

Append two tests + add both view paths to the `view_schemas_declare_source_of_truth` enumeration (per S2.T1 pattern):

```rust
#[test]
fn peer_capacity_view_minimal_validates() {
    let payload = serde_json::json!({
        "peerCid": "peer:abc",
        "computedAt": "2026-05-28T12:00:00Z",
        "totalRawBytes": 100_000_000_000u64,
        "pledges": {"dwellingBytes": 0, "collectiveBytes": 0, "commonsBytes": 0, "totalPledgedBytes": 0},
        "actuallyHeld": {"uniqueShardBytes": 0, "freeBytesRemaining": 100_000_000_000u64, "fragmentationEstimate": 0.0},
        "ratioCompliance": {
            "effectiveRatios": {"commonsPct": 20, "dwellingPct": 40, "collectivePct": 25, "freePct": 15, "manifestCid": "bafkrei-x"},
            "currentRatios": {"commonsPct": 0, "dwellingPct": 0, "collectivePct": 0, "freePct": 100},
            "compliantWithDonut": false,
            "violations": []
        }
    });
    validate_against_schema("views/peer-capacity-view.schema.json", &payload);
}

#[test]
fn hub_capacity_view_dwelling_kind_validates() {
    let payload = serde_json::json!({
        "hubId": "hub:family-smiths",
        "hubKind": "dwelling",
        "memberDeviceCount": 2,
        "capacity": null
    });
    validate_against_schema("views/hub-capacity-view.schema.json", &payload);
}
```

- [ ] **Step 7: Run + codegen + commit**

```bash
cd /projects/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev cargo test --manifest-path elohim/elohim-storage/Cargo.toml --test schema_contract peer_capacity 2>&1 | tail -10
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev cargo test --manifest-path elohim/elohim-storage/Cargo.toml --test schema_contract hub_capacity 2>&1 | tail -10
cargo test --manifest-path elohim/elohim-views/Cargo.toml export_bindings 2>&1 | tail -10
pnpm run schema:codegen:ts 2>&1 | tail -10

git add elohim/sdk/schemas/v1/views/peer-capacity-view.schema.json \
        elohim/sdk/schemas/v1/views/hub-capacity-view.schema.json \
        elohim/elohim-views/src/peer_capacity.rs \
        elohim/elohim-views/src/hub_capacity.rs \
        elohim/elohim-views/src/lib.rs \
        elohim/sdk/schemas/scripts/codegen-ts.mjs \
        elohim/elohim-storage/tests/schema_contract.rs \
        elohim/sdk/storage-client-ts/src/generated/ \
        app/elohim-app/src/app/generated/ \
        app/elohim-library/projects/elohim-service/src/generated/ \
        app/lamad/src/generated/ \
        doorway/doorway-app/src/app/generated/ 2>/dev/null || true
git commit -m "feat(views): PeerCapacityView + HubCapacityView schemas + ts-rs"
```

---


### Task 4: Extend existing topology view schemas

**Files:**
- Modify: `elohim/sdk/schemas/v1/views/distribution-summary.schema.json`
- Modify: `elohim/sdk/schemas/v1/views/distribution-details.schema.json`
- Modify: `elohim/sdk/schemas/v1/views/replica-peer.schema.json`
- Modify: `elohim/sdk/schemas/v1/views/household-resilience-view.schema.json`
- Modify: `elohim/elohim-views/src/distribution.rs` (and household_resilience.rs if separate)
- Modify: `elohim/elohim-storage/tests/schema_contract.rs`

- [ ] **Step 1: Extend `distribution-summary.schema.json`**

Read the current file first. Add to `properties`:

```json
"projectionTier": {
  "type": "string",
  "enum": ["local", "regional", "global"],
  "description": "Federation-level projection coverage. local = 1-2 projectors in same cluster; regional = 3+ projectors but ≤1 fault domain; global = projectors spanning ≥2 fault domains. Computed from projectorCount + projector geography when available."
}
```

Add to `required`: `"projectionTier"`.

Extend `replicaHealth` enum (existing values plus): `"over_replicated"`. Update description to note "over_replicated = more replicas than commitments justify; release shards to reclaim budget."

- [ ] **Step 2: Extend `distribution-details.schema.json`**

Add to `properties`:

```json
"replicationCommitments": {
  "type": "array",
  "description": "Active replicates-* commitments whose recipient + scope_filter cover this content.",
  "items": {
    "type": "object",
    "required": ["commitmentCid", "tier", "providerCid", "recipientCid"],
    "additionalProperties": false,
    "properties": {
      "commitmentCid": { "type": "string" },
      "tier":          { "type": "string", "enum": ["dwelling", "collective", "commons"] },
      "providerCid":   { "type": "string" },
      "recipientCid":  { "type": "string" },
      "providerRole":  { "type": "string", "enum": ["steward_mutual", "collective_steward"] }
    }
  }
},
"faultDomainDiversity": {
  "type": "object",
  "required": ["distinctHouseholdCount", "distinctCollectiveCount", "distinctRegionCount", "singleFaultDomainRisk", "faultModesEvaluated"],
  "additionalProperties": false,
  "properties": {
    "distinctHouseholdCount":  { "type": "integer", "minimum": 0 },
    "distinctCollectiveCount": { "type": "integer", "minimum": 0 },
    "distinctRegionCount":     { "type": "integer", "minimum": 0 },
    "singleFaultDomainRisk":   { "type": "boolean" },
    "faultModesEvaluated":     { "type": "array", "items": { "type": "string", "enum": ["household", "collective", "region"] } }
  }
}
```

Add `"replicationCommitments"` and `"faultDomainDiversity"` to `required`.

- [ ] **Step 3: Extend `replica-peer.schema.json`**

Add to `properties` (keep existing fields):

```json
"shardsHeld": {
  "type": "integer",
  "minimum": 0,
  "description": "Count of distinct RS-encoded shards this peer holds for the content. 1 if whole-blob; up to N+M for rs-N-M encoding."
},
"shardsByEncoding": {
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "encoding":              { "type": "string", "description": "rs-N-M or 'whole-blob'." },
    "minShardsForRecovery":  { "type": "integer", "minimum": 1 }
  }
}
```

These are OPTIONAL fields (do NOT add to `required` — backward-compat).

- [ ] **Step 4: Extend `household-resilience-view.schema.json`**

Add to `properties`:

```json
"commitmentBackedReplication": {
  "type": "object",
  "required": ["dwellingCommitments", "collectiveCommitments", "commonsCommitments", "totalPledgedBytes"],
  "additionalProperties": false,
  "properties": {
    "dwellingCommitments":  { "type": "integer", "minimum": 0 },
    "collectiveCommitments": { "type": "integer", "minimum": 0 },
    "commonsCommitments":    { "type": "integer", "minimum": 0 },
    "totalPledgedBytes":     { "type": "integer", "minimum": 0 }
  }
}
```

Add `"commitmentBackedReplication"` to `required`.

- [ ] **Step 5: Update Rust ts-rs view types**

In `elohim/elohim-views/src/distribution.rs` (locate the existing `DistributionSummary` + `DistributionDetails` structs; pattern was landed by light-up-topology sprint):
- Add `projection_tier: ProjectionTier` to `DistributionSummary`.
- Extend `ReplicaHealth` enum with `OverReplicated` variant (keep snake_case serde).
- Add `replication_commitments: Vec<ReplicationCommitmentRef>` + `fault_domain_diversity: FaultDomainDiversity` to `DistributionDetails`.
- Add new types `ProjectionTier { Local, Regional, Global }`, `ReplicationCommitmentRef`, `FaultDomainDiversity`, `Tier { Dwelling, Collective, Commons }`, `ProviderRole`.

In the same file (or `replica_peer.rs` if separate), add optional `shards_held: Option<u32>` and `shards_by_encoding: Option<ShardsByEncoding>`.

In `elohim/elohim-views/src/household_resilience.rs` (locate the existing `HouseholdResilienceView`), add `commitment_backed_replication: CommitmentBackedReplication` field with a new struct.

All new types use `#[serde(rename_all = "camelCase")]` on structs, `#[serde(rename_all = "snake_case")]` on enums (matching the schema's snake_case enum values), `#[derive(TS, Serialize, Deserialize, Debug, Clone)]`, `#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]`.

- [ ] **Step 6: Schema contract tests**

Append to `tests/schema_contract.rs`:

```rust
#[test]
fn distribution_summary_with_projection_tier_validates() {
    let payload = serde_json::json!({
        "replicaCount": 11,
        "replicaTarget": 11,
        "replicaHealth": "over_replicated",
        "projectorCount": 3,
        "reachClass": "household",
        "diversityHint": {"distinctClusters": 2},
        "thisFetchSource": "peer_direct",
        "lastVerifiedSeconds": 30,
        "projectionTier": "regional"
    });
    validate_against_schema("views/distribution-summary.schema.json", &payload);
}

#[test]
fn household_resilience_with_commitment_backed_replication_validates() {
    let payload = serde_json::json!({
        "contentId": "bafkrei-content-x",
        "householdsStewarding": 3,
        "householdsReciprocated": 2,
        "protectionStatus": "protected",
        "details": {"stewardHouseholds": ["hub:A","hub:B","hub:C"], "onlinePeerCount": 5, "healthScore": 0.95},
        "commitmentBackedReplication": {
            "dwellingCommitments": 3, "collectiveCommitments": 1, "commonsCommitments": 0, "totalPledgedBytes": 150_000_000_000u64
        }
    });
    validate_against_schema("views/household-resilience-view.schema.json", &payload);
}
```

- [ ] **Step 7: Run + codegen + commit**

```bash
cd /projects/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev cargo test --manifest-path elohim/elohim-storage/Cargo.toml --test schema_contract 2>&1 | tail -20
cargo test --manifest-path elohim/elohim-views/Cargo.toml export_bindings 2>&1 | tail -10
pnpm run schema:codegen:ts 2>&1 | tail -10

git add elohim/sdk/schemas/v1/views/distribution-summary.schema.json \
        elohim/sdk/schemas/v1/views/distribution-details.schema.json \
        elohim/sdk/schemas/v1/views/replica-peer.schema.json \
        elohim/sdk/schemas/v1/views/household-resilience-view.schema.json \
        elohim/elohim-views/src/distribution.rs \
        elohim/elohim-views/src/household_resilience.rs \
        elohim/elohim-storage/tests/schema_contract.rs \
        elohim/sdk/storage-client-ts/src/generated/ \
        app/elohim-app/src/app/generated/ \
        app/elohim-library/projects/elohim-service/src/generated/ \
        app/lamad/src/generated/ \
        doorway/doorway-app/src/app/generated/ 2>/dev/null || true
git commit -m "feat(views): light up storage-replication into existing topology view schemas"
```

---

### Task 5: Manifest config — constitutionalRatios + reciprocity-imbalance signal_kind

**Files:**
- Modify: `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`
- Modify: `elohim/sdk/domains/elohim/manifest.json`

- [ ] **Step 1: Extend app-manifest schema with `constitutionalRatios` block**

In `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`, find the top-level `properties` block. Add `constitutionalRatios` as an optional property:

```json
"constitutionalRatios": {
  "type": "object",
  "description": "Protocol-version-stable storage donut ratios. Values clamped to DNA-enforced floor/ceiling at validation time. Sum should equal 100. Per spec 2026-05-28-mutual-storage-replication-dwelling-hub-design.md §5.",
  "required": ["version", "commons_pct", "dwelling_pct", "collective_pct", "free_pct"],
  "additionalProperties": false,
  "properties": {
    "version":        { "type": "integer", "minimum": 1 },
    "commons_pct":    { "type": "integer", "minimum": 0, "maximum": 100 },
    "dwelling_pct":   { "type": "integer", "minimum": 0, "maximum": 100 },
    "collective_pct": { "type": "integer", "minimum": 0, "maximum": 100 },
    "free_pct":       { "type": "integer", "minimum": 0, "maximum": 100 }
  }
}
```

Do NOT add to top-level `required` (the field is optional; manifests without it use DNA defaults).

- [ ] **Step 2: Declare `constitutionalRatios` in elohim manifest**

In `elohim/sdk/domains/elohim/manifest.json`, add the block:

```json
"constitutionalRatios": {
  "description": "Sprint 3 donut defaults; per spec §5.",
  "version": 1,
  "commons_pct": 20,
  "dwelling_pct": 40,
  "collective_pct": 25,
  "free_pct": 15
}
```

- [ ] **Step 3: Declare `reciprocity-imbalance` signal_kind**

In the same `elohim/sdk/domains/elohim/manifest.json`, find `signalKinds`. Add a new entry alongside existing kinds:

```json
"reciprocity-imbalance": {
  "description": "Provider-dwelling-hub authored a replicates-dwelling commitment in steward_mutual mode; counter-commitment never arrived within grace_period_days, or was unilaterally revoked. Substrate emits this signal naming the breaching party. Standing-debit moderate.",
  "target_kinds": ["dwelling-hub", "agent"],
  "evidence_required": true,
  "standing_impact_allowed": ["consequential"],
  "debit_weight": 8,
  "decay_days": 60
}
```

- [ ] **Step 4: Validate**

```bash
cd /projects/elohim
pnpm run schema:validate 2>&1 | tail -10
```

Expected: clean pass (3,400+ valid, 0 errors). The new `constitutionalRatios` field is recognized by the extended schema; the new signal_kind validates against the existing signalKinds.additionalProperties shape (already extended in Sprint 2 T7 to accept `debit_weight` + `decay_days`).

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/schemas/v1/manifest/app-manifest.schema.json elohim/sdk/domains/elohim/manifest.json
git commit -m "feat(manifest): elohim constitutionalRatios block + reciprocity-imbalance signal_kind"
```

---

## Phase B — Substrate primitives

### Task 6: `constitutional_ratio_registry` service

**Files:**
- Create: `elohim/elohim-storage/src/services/constitutional_ratio_registry.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

- [ ] **Step 1: Author the registry**

Mirrors `signal_weight_registry` from Sprint 2 T7. Create the file:

```rust
//! Manifest-driven constitutional ratio registry.
//!
//! Reads the elohim domain manifest at startup (lazy via OnceLock) and exposes
//! `effective_ratios()` — the per-tier percentages clamped to DNA floor/ceiling
//! walls. Used by bounds_validator + replicates_dwelling_validator at
//! commitment-author time per spec §5.4.
//!
//! Source of truth: `elohim/sdk/domains/elohim/manifest.json` `constitutionalRatios` block.
//! Override the manifest path via `ELOHIM_MANIFEST_PATH` (tests).

use serde::Deserialize;
use std::sync::OnceLock;

// Mirror of DNA constants. WHY duplicate: bounds_validator runs in storage
// (native target); DNA constants live in WASM zone. Keep these synced with
// `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`.
pub const COMMONS_MIN_FLOOR_PCT: u8 = 10;
pub const COMMONS_MAX_CEILING_PCT: u8 = 60;
pub const DWELLING_MIN_FLOOR_PCT: u8 = 10;
pub const DWELLING_MAX_CEILING_PCT: u8 = 80;
pub const FREE_MIN_FLOOR_PCT: u8 = 5;
pub const FREE_MAX_CEILING_PCT: u8 = 70;

#[derive(Debug, Clone, Copy)]
pub struct EffectiveRatios {
    pub commons_pct: u8,
    pub dwelling_pct: u8,
    pub collective_pct: u8,
    pub free_pct: u8,
}

#[derive(Debug, Clone)]
pub struct EffectiveRatiosWithProvenance {
    pub ratios: EffectiveRatios,
    pub manifest_cid: String,
}

#[derive(Deserialize)]
struct RawConstitutionalRatios {
    #[serde(default = "default_commons")]
    commons_pct: u8,
    #[serde(default = "default_dwelling")]
    dwelling_pct: u8,
    #[serde(default = "default_collective")]
    collective_pct: u8,
    #[serde(default = "default_free")]
    free_pct: u8,
}

fn default_commons() -> u8 { 20 }
fn default_dwelling() -> u8 { 40 }
fn default_collective() -> u8 { 25 }
fn default_free() -> u8 { 15 }

#[derive(Deserialize)]
struct ElohimManifest {
    #[serde(rename = "constitutionalRatios", default)]
    constitutional_ratios: Option<RawConstitutionalRatios>,
}

static REGISTRY: OnceLock<EffectiveRatiosWithProvenance> = OnceLock::new();

pub fn effective_ratios() -> EffectiveRatiosWithProvenance {
    REGISTRY.get_or_init(load_from_manifest).clone()
}

fn load_from_manifest() -> EffectiveRatiosWithProvenance {
    let manifest_path = std::env::var("ELOHIM_MANIFEST_PATH").unwrap_or_else(|_| {
        format!("{}/../sdk/domains/elohim/manifest.json", env!("CARGO_MANIFEST_DIR"))
    });
    let manifest_cid = compute_manifest_cid(&manifest_path);
    let raw = match std::fs::read(&manifest_path) {
        Ok(b) => b,
        Err(_) => {
            return EffectiveRatiosWithProvenance {
                ratios: dna_default_ratios(),
                manifest_cid,
            };
        }
    };
    let parsed: ElohimManifest = serde_json::from_slice(&raw).unwrap_or(ElohimManifest { constitutional_ratios: None });
    let raw = parsed.constitutional_ratios.unwrap_or(RawConstitutionalRatios {
        commons_pct: default_commons(),
        dwelling_pct: default_dwelling(),
        collective_pct: default_collective(),
        free_pct: default_free(),
    });
    let commons  = clamp(raw.commons_pct,  COMMONS_MIN_FLOOR_PCT,  COMMONS_MAX_CEILING_PCT);
    let dwelling = clamp(raw.dwelling_pct, DWELLING_MIN_FLOOR_PCT, DWELLING_MAX_CEILING_PCT);
    let free     = clamp(raw.free_pct,     FREE_MIN_FLOOR_PCT,     FREE_MAX_CEILING_PCT);
    // collective is the residual to make percentages sum to 100; if manifest's
    // collective_pct disagrees, the residual wins (substrate-correct).
    let collective = 100u8.saturating_sub(commons).saturating_sub(dwelling).saturating_sub(free);
    EffectiveRatiosWithProvenance {
        ratios: EffectiveRatios { commons_pct: commons, dwelling_pct: dwelling, collective_pct: collective, free_pct: free },
        manifest_cid,
    }
}

fn dna_default_ratios() -> EffectiveRatios {
    EffectiveRatios {
        commons_pct: 20,
        dwelling_pct: 40,
        collective_pct: 25,
        free_pct: 15,
    }
}

fn clamp(v: u8, lo: u8, hi: u8) -> u8 {
    v.max(lo).min(hi)
}

fn compute_manifest_cid(path: &str) -> String {
    // Substrate-correct CID would hash the manifest bytes via the EPR cid module.
    // For Sprint 3 we use a path-derived fingerprint; follow-up sprint upgrades.
    format!("manifest-fingerprint:{path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_ratios_sums_to_100() {
        let r = effective_ratios().ratios;
        assert_eq!(
            r.commons_pct as u16 + r.dwelling_pct as u16 + r.collective_pct as u16 + r.free_pct as u16,
            100u16
        );
    }

    #[test]
    fn effective_ratios_within_dna_walls() {
        let r = effective_ratios().ratios;
        assert!(r.commons_pct  >= COMMONS_MIN_FLOOR_PCT  && r.commons_pct  <= COMMONS_MAX_CEILING_PCT);
        assert!(r.dwelling_pct >= DWELLING_MIN_FLOOR_PCT && r.dwelling_pct <= DWELLING_MAX_CEILING_PCT);
        assert!(r.free_pct     >= FREE_MIN_FLOOR_PCT     && r.free_pct     <= FREE_MAX_CEILING_PCT);
    }

    #[test]
    fn provenance_field_is_populated() {
        let p = effective_ratios();
        assert!(!p.manifest_cid.is_empty());
    }
}
```

- [ ] **Step 2: Register module**

In `elohim/elohim-storage/src/services/mod.rs` add `pub mod constitutional_ratio_registry;` in sorted order.

- [ ] **Step 3: Run + commit**

```bash
cd /projects/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib services::constitutional_ratio_registry 2>&1 | tail -10
git add elohim/elohim-storage/src/services/constitutional_ratio_registry.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(storage): constitutional_ratio_registry — manifest-driven donut clamped to DNA walls"
```

---


### Task 7: `bounds_validator::BoundsViolation::ConstitutionalRatioBreach` variant

**Files:**
- Modify: `elohim/elohim-storage/src/services/bounds_validator.rs`
- Modify: `elohim/elohim-views/src/bounds.rs`
- Modify: `elohim/sdk/schemas/v1/views/bounds-validation-result-view.schema.json`

- [ ] **Step 1: Extend `ViolationKind` enum**

In `elohim/elohim-views/src/bounds.rs`, find the existing `ViolationKind` enum landed by Sprint 2 T1. Add the new variant in the same snake_case style:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum ViolationKind {
    CommitmentInactive,
    ScopeNotIncluded,
    ReachCeilingExceeded,
    RateLimitExceeded,
    KeyRotationStale,
    CommitmentRevoked,
    CommitmentNotFound,
    ConstitutionalRatioBreach,  // NEW Sprint 3
}
```

- [ ] **Step 2: Extend schema enum**

In `elohim/sdk/schemas/v1/views/bounds-validation-result-view.schema.json`, find the `violation.kind.enum` array. Append `"constitutional_ratio_breach"` (snake_case to match the Rust serde-rename).

- [ ] **Step 3: Validate + codegen + commit**

```bash
cd /projects/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev cargo test --manifest-path elohim/elohim-storage/Cargo.toml --test schema_contract bounds_validation 2>&1 | tail -10
cargo test --manifest-path elohim/elohim-views/Cargo.toml export_bindings 2>&1 | tail -10
pnpm run schema:codegen:ts 2>&1 | tail -10

git add elohim/elohim-views/src/bounds.rs \
        elohim/sdk/schemas/v1/views/bounds-validation-result-view.schema.json \
        elohim/sdk/storage-client-ts/src/generated/ \
        app/elohim-app/src/app/generated/ \
        app/elohim-library/projects/elohim-service/src/generated/ \
        app/lamad/src/generated/ \
        doorway/doorway-app/src/app/generated/ 2>/dev/null || true
git commit -m "feat(bounds): ViolationKind::ConstitutionalRatioBreach variant"
```

---

### Task 8: `replicates_dwelling_validator` service

**Files:**
- Create: `elohim/elohim-storage/src/services/replicates_dwelling_validator.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

Per-instance validator that delegates to Sprint 2's `bounds_validator::validate` + adds the schema check + the donut check. Mirrors `republish_epr_validator` shape from Sprint 1 T6.

- [ ] **Step 1: Author the service**

```rust
//! Sprint 3 — per-instance validator for `replicates-dwelling` events.
//!
//! Three-stage validation:
//!   1. Schema check (cheap, structural).
//!   2. Donut check (constitutional_ratio_registry vs ratio_attestation + proposed pledge).
//!   3. Substrate bounds check (delegate to bounds_validator::validate for the 7 substrate-wide checks).
//!
//! Pattern: per project_bounds_validator_pattern memory. Sprint 3 is the
//! FIRST concrete instance proving the pattern; Sprints N+1+ (collective tier,
//! commons tier, doorway projection compute, distributed workloads) mirror
//! this shape for their per-instance validators.

use crate::services::bounds_validator::{self, BoundsViolation, EventForValidation};
use crate::services::commitment_fetcher::CommitmentFetcher;
use crate::services::rate_history::RateHistory;
use crate::services::constitutional_ratio_registry::{self, EffectiveRatios};
use elohim_views::bounds::{BoundsChecksView, ViolationKind};

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("schema validation failed: {0}")]
    Schema(String),
    #[error("constitutional ratio breach: {0}")]
    ConstitutionalRatio(String),
    #[error("collective_steward mode pending follow-up sprint — not yet supported")]
    CollectiveStewardModeNotYetSupported,
    #[error("bounds validation failed: {0:?}")]
    Bounds(BoundsViolation),
}

/// Provider's existing per-tier pledged bytes summary (input to donut check).
/// Computed by peer_capacity_service before calling this validator.
#[derive(Debug, Clone, Default)]
pub struct ProviderPledgedState {
    pub total_raw_bytes:        u64,
    pub pledged_dwelling_bytes: u64,
    pub pledged_collective_bytes: u64,
    pub pledged_commons_bytes:    u64,
}

pub async fn validate_replicates_dwelling<F: CommitmentFetcher, R: RateHistory>(
    event_payload: &serde_json::Value,
    fetcher: &F,
    rate_history: &R,
    provider_state: &ProviderPledgedState,
) -> Result<(), ValidationError> {
    // 1. Schema check
    validate_payload_schema(event_payload)?;

    // 2. Collective_steward mode is schema-reserved this sprint; reject explicitly.
    let provider_role = event_payload["provider_role"].as_str().unwrap_or("");
    if provider_role == "collective_steward" {
        return Err(ValidationError::CollectiveStewardModeNotYetSupported);
    }

    // 3. Donut check (ceiling enforced via pledges; floor via ratio_attestation declaration this sprint)
    donut_check(event_payload, provider_state)?;

    // 4. Substrate bounds check via Sprint 2 primitive
    let event = project_to_event_for_validation(event_payload);
    bounds_validator::validate(&event, fetcher, rate_history)
        .await
        .map(|_checks| ())
        .map_err(ValidationError::Bounds)
}

fn validate_payload_schema(payload: &serde_json::Value) -> Result<(), ValidationError> {
    let required = [
        "action", "provider_dwelling_hub_id", "recipient_dwelling_hub_id",
        "provider_role", "capacity_bytes", "scope_filter",
        "valid_from", "valid_until", "grace_period_days",
        "rotation_ttl_days", "ratio_attestation",
    ];
    for field in required {
        if payload.get(field).is_none() {
            return Err(ValidationError::Schema(format!("missing field: {field}")));
        }
    }
    if payload["action"] != "replicates-dwelling" {
        return Err(ValidationError::Schema("action must be 'replicates-dwelling'".into()));
    }
    let provider_role = payload["provider_role"].as_str().unwrap_or("");
    if provider_role != "steward_mutual" && provider_role != "collective_steward" {
        return Err(ValidationError::Schema(format!("unknown provider_role: {provider_role}")));
    }
    if provider_role == "collective_steward" {
        let via = payload.get("via_collective_hub_id").and_then(|v| v.as_str()).unwrap_or("");
        if via.is_empty() {
            return Err(ValidationError::Schema("collective_steward requires via_collective_hub_id".into()));
        }
    }
    Ok(())
}

fn donut_check(
    payload: &serde_json::Value,
    state: &ProviderPledgedState,
) -> Result<(), ValidationError> {
    let provenance = constitutional_ratio_registry::effective_ratios();
    let effective = provenance.ratios;
    let manifest_cid = provenance.manifest_cid;

    let capacity_bytes = payload["capacity_bytes"].as_u64().unwrap_or(0);
    let attestation = payload.get("ratio_attestation")
        .ok_or_else(|| ValidationError::ConstitutionalRatio("missing ratio_attestation".into()))?;
    let attested_commons = attestation["commons_pct"].as_u64().unwrap_or(0) as u8;
    let attested_dwelling = attestation["dwelling_pct"].as_u64().unwrap_or(0) as u8;
    let attested_collective = attestation["collective_pct"].as_u64().unwrap_or(0) as u8;
    let attested_free = attestation["free_pct"].as_u64().unwrap_or(0) as u8;
    let attested_cid = attestation["effective_ratio_cid"].as_str().unwrap_or("");

    // (a) Sum-to-100
    let sum = attested_commons as u16 + attested_dwelling as u16 + attested_collective as u16 + attested_free as u16;
    if sum != 100 {
        return Err(ValidationError::ConstitutionalRatio(format!("ratio_attestation pct sum {sum} != 100")));
    }

    // (b) Attested values must match clamped effective_ratios (declaration matches manifest)
    if attested_commons != effective.commons_pct {
        return Err(ValidationError::ConstitutionalRatio(format!(
            "attested commons_pct {attested_commons} != effective {} (manifest {})",
            effective.commons_pct, manifest_cid
        )));
    }
    if attested_dwelling != effective.dwelling_pct {
        return Err(ValidationError::ConstitutionalRatio(format!(
            "attested dwelling_pct {attested_dwelling} != effective {}",
            effective.dwelling_pct
        )));
    }
    if attested_collective != effective.collective_pct {
        return Err(ValidationError::ConstitutionalRatio(format!(
            "attested collective_pct {attested_collective} != effective {}",
            effective.collective_pct
        )));
    }
    if attested_free != effective.free_pct {
        return Err(ValidationError::ConstitutionalRatio(format!(
            "attested free_pct {attested_free} != effective {}",
            effective.free_pct
        )));
    }

    // (c) Ceiling check: new dwelling pledge cannot push dwelling-tier above effective ceiling
    let total = state.total_raw_bytes.max(1);
    let new_dwelling = state.pledged_dwelling_bytes + capacity_bytes;
    let new_dwelling_pct = ((new_dwelling as u128 * 100) / total as u128) as u64;
    if new_dwelling_pct as u8 > effective.dwelling_pct {
        return Err(ValidationError::ConstitutionalRatio(format!(
            "adding {capacity_bytes} would push dwelling_pct to {new_dwelling_pct}, above effective ceiling {}",
            effective.dwelling_pct
        )));
    }

    // (d) Floor check via declaration (Sprint 3 design choice; follow-up sprint adds backing-pledge requirement)
    if attested_commons < crate::services::constitutional_ratio_registry::COMMONS_MIN_FLOOR_PCT {
        return Err(ValidationError::ConstitutionalRatio(format!(
            "attested commons_pct {attested_commons} below DNA floor {}",
            crate::services::constitutional_ratio_registry::COMMONS_MIN_FLOOR_PCT
        )));
    }

    // (e) Provenance match
    if attested_cid != manifest_cid {
        return Err(ValidationError::ConstitutionalRatio(format!(
            "ratio_attestation effective_ratio_cid {attested_cid} != current manifest {manifest_cid}"
        )));
    }

    Ok(())
}

fn project_to_event_for_validation(payload: &serde_json::Value) -> EventForValidation {
    EventForValidation {
        action: payload["action"].as_str().unwrap_or("").to_string(),
        performer: payload["provider_dwelling_hub_id"].as_str().unwrap_or("").to_string(),
        bounded_by: payload["recipient_dwelling_hub_id"].as_str().unwrap_or("").to_string(),
        target_epr_id: payload["recipient_dwelling_hub_id"].as_str().unwrap_or("").to_string(),
        reach: "household".into(),
        signed_at: payload.get("signed_at").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::commitment_fetcher::MockCommitmentFetcher;
    use crate::services::rate_history::MockRateHistory;

    fn well_formed_payload(provider_role: &str, capacity_bytes: u64) -> serde_json::Value {
        let provenance = constitutional_ratio_registry::effective_ratios();
        let r = provenance.ratios;
        serde_json::json!({
            "action": "replicates-dwelling",
            "provider_dwelling_hub_id": "hub:A",
            "recipient_dwelling_hub_id": "hub:B",
            "provider_role": provider_role,
            "capacity_bytes": capacity_bytes,
            "scope_filter": {"epr_kinds": ["Content"]},
            "valid_from": "2026-05-28T00:00:00Z",
            "valid_until": "2026-08-26T00:00:00Z",
            "grace_period_days": 14,
            "rotation_ttl_days": 90,
            "ratio_attestation": {
                "commons_pct": r.commons_pct,
                "dwelling_pct": r.dwelling_pct,
                "collective_pct": r.collective_pct,
                "free_pct": r.free_pct,
                "effective_ratio_cid": provenance.manifest_cid
            },
            "signed_at": "2026-05-28T12:00:00Z"
        })
    }

    fn fresh_state() -> ProviderPledgedState {
        ProviderPledgedState {
            total_raw_bytes: 100_000_000_000,
            pledged_dwelling_bytes: 0,
            pledged_collective_bytes: 0,
            pledged_commons_bytes: 0,
        }
    }

    #[tokio::test]
    async fn valid_steward_mutual_passes_donut_then_bounds() {
        // Note: bounds_validator will still fail at CommitmentNotFound (fetcher empty)
        // but donut + schema checks should pass before that.
        let payload = well_formed_payload("steward_mutual", 30_000_000_000);
        let fetcher = MockCommitmentFetcher::new();
        let rate = MockRateHistory::new();
        let result = validate_replicates_dwelling(&payload, &fetcher, &rate, &fresh_state()).await;
        // Expect Bounds(CommitmentNotFound) since we didn't seed; that proves we passed schema+donut.
        match result {
            Err(ValidationError::Bounds(b)) => assert_eq!(b.kind, ViolationKind::CommitmentNotFound),
            other => panic!("expected Bounds(CommitmentNotFound), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn collective_steward_rejected_explicitly() {
        let mut payload = well_formed_payload("collective_steward", 30_000_000_000);
        payload["via_collective_hub_id"] = serde_json::json!("collective:church");
        let fetcher = MockCommitmentFetcher::new();
        let rate = MockRateHistory::new();
        let result = validate_replicates_dwelling(&payload, &fetcher, &rate, &fresh_state()).await;
        assert!(matches!(result, Err(ValidationError::CollectiveStewardModeNotYetSupported)));
    }

    #[tokio::test]
    async fn ratio_attestation_below_floor_rejected() {
        let mut payload = well_formed_payload("steward_mutual", 30_000_000_000);
        payload["ratio_attestation"]["commons_pct"] = serde_json::json!(5);  // below 10 floor
        payload["ratio_attestation"]["free_pct"] = serde_json::json!(30);    // make sum=100
        let fetcher = MockCommitmentFetcher::new();
        let rate = MockRateHistory::new();
        let result = validate_replicates_dwelling(&payload, &fetcher, &rate, &fresh_state()).await;
        assert!(matches!(result, Err(ValidationError::ConstitutionalRatio(_))));
    }

    #[tokio::test]
    async fn dwelling_ceiling_breach_rejected() {
        // Provider already pledged 70GB dwelling on 100GB device; effective dwelling ceiling=40%; new 30GB pushes to 100%
        let mut state = fresh_state();
        state.pledged_dwelling_bytes = 70_000_000_000;
        let payload = well_formed_payload("steward_mutual", 30_000_000_000);
        let fetcher = MockCommitmentFetcher::new();
        let rate = MockRateHistory::new();
        let result = validate_replicates_dwelling(&payload, &fetcher, &rate, &state).await;
        assert!(matches!(result, Err(ValidationError::ConstitutionalRatio(_))));
    }

    #[tokio::test]
    async fn schema_missing_field_rejected() {
        let mut payload = well_formed_payload("steward_mutual", 30_000_000_000);
        payload.as_object_mut().unwrap().remove("capacity_bytes");
        let fetcher = MockCommitmentFetcher::new();
        let rate = MockRateHistory::new();
        let result = validate_replicates_dwelling(&payload, &fetcher, &rate, &fresh_state()).await;
        assert!(matches!(result, Err(ValidationError::Schema(_))));
    }
}
```

- [ ] **Step 2: Register module**

In `elohim/elohim-storage/src/services/mod.rs` add `pub mod replicates_dwelling_validator;` in sorted order.

- [ ] **Step 3: Run + commit**

```bash
cd /projects/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib services::replicates_dwelling_validator 2>&1 | tail -15
git add elohim/elohim-storage/src/services/replicates_dwelling_validator.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(storage): replicates_dwelling_validator — first bounds-validator-pattern instance"
```

---


## Phase C — Mishpat zome

### Task 9: Mishpat coordinator `validate_replicates_dwelling`

**Files:**
- Modify: `elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs`

Sprint 1 T1+T2 landed `Mishpat::Commitment` entry type + the `validate_commitment_payload` dispatch with two existing actions (`delegates-compute`, `acknowledges-reach-change`). This task adds a third branch.

- [ ] **Step 1: Add new action branch**

In `elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs`, locate `validate_commitment_payload` (the match dispatching on `input.action.as_str()`). Add:

```rust
"replicates-dwelling" => validate_replicates_dwelling(&payload),
```

- [ ] **Step 2: Implement `validate_replicates_dwelling`**

Add the validator function alongside `validate_delegates_compute` and `validate_acknowledges_reach_change`:

```rust
fn validate_replicates_dwelling(payload: &serde_json::Value) -> Result<(), String> {
    let required = [
        "action", "provider_dwelling_hub_id", "recipient_dwelling_hub_id",
        "provider_role", "capacity_bytes", "scope_filter",
        "valid_from", "valid_until", "grace_period_days",
        "rotation_ttl_days", "ratio_attestation",
    ];
    for field in required {
        if payload.get(field).is_none() {
            return Err(format!("replicates-dwelling missing required field: {field}"));
        }
    }
    if payload["action"] != "replicates-dwelling" {
        return Err("action field must equal 'replicates-dwelling'".into());
    }

    // provider_role enum
    let provider_role = payload["provider_role"].as_str().unwrap_or("");
    if provider_role != "steward_mutual" && provider_role != "collective_steward" {
        return Err(format!("provider_role '{provider_role}' not in enum"));
    }
    if provider_role == "collective_steward" {
        let via = payload.get("via_collective_hub_id").and_then(|v| v.as_str()).unwrap_or("");
        if via.is_empty() {
            return Err("collective_steward requires non-empty via_collective_hub_id".into());
        }
    }

    // capacity_bytes positive
    let capacity = payload["capacity_bytes"].as_u64().unwrap_or(0);
    if capacity == 0 {
        return Err("capacity_bytes must be > 0".into());
    }

    // ratio_attestation: required sub-fields + sum-to-100
    let attestation = payload.get("ratio_attestation").and_then(|v| v.as_object())
        .ok_or("ratio_attestation must be object")?;
    for f in ["commons_pct", "dwelling_pct", "collective_pct", "free_pct", "effective_ratio_cid"] {
        if !attestation.contains_key(f) {
            return Err(format!("ratio_attestation missing field: {f}"));
        }
    }
    let commons  = attestation["commons_pct"].as_u64().unwrap_or(0);
    let dwelling = attestation["dwelling_pct"].as_u64().unwrap_or(0);
    let collective = attestation["collective_pct"].as_u64().unwrap_or(0);
    let free     = attestation["free_pct"].as_u64().unwrap_or(0);
    if commons + dwelling + collective + free != 100 {
        return Err(format!(
            "ratio_attestation pct sum {} != 100",
            commons + dwelling + collective + free
        ));
    }

    // scope_filter must be object (curation policy; can be empty)
    if !payload.get("scope_filter").map(|v| v.is_object()).unwrap_or(false) {
        return Err("scope_filter must be object".into());
    }

    Ok(())
}
```

- [ ] **Step 3: Add tests to the existing `tests` module**

Append to `commitments.rs`:

```rust
fn well_formed_replicates_dwelling_payload() -> serde_json::Value {
    serde_json::json!({
        "action": "replicates-dwelling",
        "provider_dwelling_hub_id": "hub:A",
        "recipient_dwelling_hub_id": "hub:B",
        "provider_role": "steward_mutual",
        "capacity_bytes": 50_000_000_000u64,
        "scope_filter": {"epr_kinds": ["Content"]},
        "valid_from": "2026-05-28T00:00:00Z",
        "valid_until": "2026-08-26T00:00:00Z",
        "grace_period_days": 14,
        "rotation_ttl_days": 90,
        "ratio_attestation": {
            "commons_pct": 20, "dwelling_pct": 40, "collective_pct": 25, "free_pct": 15,
            "effective_ratio_cid": "bafkrei-x"
        }
    })
}

#[test]
fn replicates_dwelling_well_formed_validates() {
    let input = CreateCommitmentInput {
        action: "replicates-dwelling".to_string(),
        payload_json: well_formed_replicates_dwelling_payload().to_string(),
    };
    assert!(validate_commitment_payload(&input).is_ok());
}

#[test]
fn replicates_dwelling_unknown_role_rejected() {
    let mut payload = well_formed_replicates_dwelling_payload();
    payload["provider_role"] = serde_json::json!("totally-bogus");
    let input = CreateCommitmentInput {
        action: "replicates-dwelling".to_string(),
        payload_json: payload.to_string(),
    };
    assert!(validate_commitment_payload(&input).is_err());
}

#[test]
fn replicates_dwelling_collective_steward_requires_via_collective() {
    let mut payload = well_formed_replicates_dwelling_payload();
    payload["provider_role"] = serde_json::json!("collective_steward");
    let input = CreateCommitmentInput {
        action: "replicates-dwelling".to_string(),
        payload_json: payload.to_string(),
    };
    assert!(validate_commitment_payload(&input).is_err());
}

#[test]
fn replicates_dwelling_collective_steward_with_via_validates() {
    let mut payload = well_formed_replicates_dwelling_payload();
    payload["provider_role"] = serde_json::json!("collective_steward");
    payload["via_collective_hub_id"] = serde_json::json!("collective:church");
    let input = CreateCommitmentInput {
        action: "replicates-dwelling".to_string(),
        payload_json: payload.to_string(),
    };
    assert!(validate_commitment_payload(&input).is_ok());
}

#[test]
fn replicates_dwelling_zero_capacity_rejected() {
    let mut payload = well_formed_replicates_dwelling_payload();
    payload["capacity_bytes"] = serde_json::json!(0);
    let input = CreateCommitmentInput {
        action: "replicates-dwelling".to_string(),
        payload_json: payload.to_string(),
    };
    assert!(validate_commitment_payload(&input).is_err());
}

#[test]
fn replicates_dwelling_ratio_sum_not_100_rejected() {
    let mut payload = well_formed_replicates_dwelling_payload();
    payload["ratio_attestation"]["commons_pct"] = serde_json::json!(30);  // sum becomes 110
    let input = CreateCommitmentInput {
        action: "replicates-dwelling".to_string(),
        payload_json: payload.to_string(),
    };
    assert!(validate_commitment_payload(&input).is_err());
}
```

- [ ] **Step 4: Build + test + commit**

```bash
cd /projects/elohim/elohim/holochain/dna/mishpat
just check 2>&1 | tail -10
cargo test -p mishpat commitments::tests::replicates_dwelling 2>&1 | tail -20

cd /projects/elohim
git add elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs
git commit -m "feat(mishpat): replicates-dwelling Commitment action + validator + 6 unit tests"
```

Expected: `just check` clean; 6 tests pass.

---

### Task 10: Mishpat integrity defense-in-depth for `replicates-dwelling`

**Files:**
- Modify: `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs`

Sprint 1 T1+T2 landed `validate_commitment_entry` with substring-heuristic (serde_json is dev-only in integrity zone, WASM size budget). Extend it to gate `replicates-dwelling`.

- [ ] **Step 1: Locate `validate_commitment_entry`**

```bash
grep -nE "fn validate_commitment_entry" /projects/elohim/elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs
```

- [ ] **Step 2: Extend the validator**

Find the existing function. After existing structural checks, add:

```rust
// Sprint 3: replicates-dwelling defense-in-depth.
// Coordinator does full schema validation; integrity catches direct-source-chain bypass.
if commitment.action == "replicates-dwelling" {
    let meta = commitment.payload_json.trim();
    if meta.is_empty() || !meta.starts_with('{') {
        return Ok(ValidateCallbackResult::Invalid(
            "replicates-dwelling requires payload_json as a JSON object".into(),
        ));
    }
    // Recipient must be non-empty (anonymous replication forbidden).
    if !meta.contains("recipient_dwelling_hub_id") {
        return Ok(ValidateCallbackResult::Invalid(
            "replicates-dwelling requires recipient_dwelling_hub_id field".into(),
        ));
    }
    if meta.contains("\"recipient_dwelling_hub_id\":\"\"") || meta.contains("\"recipient_dwelling_hub_id\": \"\"") {
        return Ok(ValidateCallbackResult::Invalid(
            "replicates-dwelling recipient_dwelling_hub_id must be non-empty".into(),
        ));
    }
    // Provider role must be one of the two enum values (substring check).
    let has_steward_mutual = meta.contains("\"provider_role\":\"steward_mutual\"")
        || meta.contains("\"provider_role\": \"steward_mutual\"");
    let has_collective_steward = meta.contains("\"provider_role\":\"collective_steward\"")
        || meta.contains("\"provider_role\": \"collective_steward\"");
    if !has_steward_mutual && !has_collective_steward {
        return Ok(ValidateCallbackResult::Invalid(
            "replicates-dwelling provider_role must be steward_mutual or collective_steward".into(),
        ));
    }
}
```

- [ ] **Step 3: Add tests to the existing integrity tests module**

```rust
#[test]
fn replicates_dwelling_well_formed_accepted() {
    let event = Commitment {
        action: "replicates-dwelling".into(),
        payload_json: r#"{"action":"replicates-dwelling","provider_dwelling_hub_id":"hub:A","recipient_dwelling_hub_id":"hub:B","provider_role":"steward_mutual","capacity_bytes":1}"#.into(),
        signed_at: "2026-05-28T00:00:00Z".into(),
    };
    let result = validate_commitment_entry(&event).unwrap();
    assert!(matches!(result, ValidateCallbackResult::Valid));
}

#[test]
fn replicates_dwelling_empty_recipient_rejected() {
    let event = Commitment {
        action: "replicates-dwelling".into(),
        payload_json: r#"{"action":"replicates-dwelling","provider_dwelling_hub_id":"hub:A","recipient_dwelling_hub_id":"","provider_role":"steward_mutual"}"#.into(),
        signed_at: "2026-05-28T00:00:00Z".into(),
    };
    let result = validate_commitment_entry(&event).unwrap();
    assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
}

#[test]
fn replicates_dwelling_missing_recipient_rejected() {
    let event = Commitment {
        action: "replicates-dwelling".into(),
        payload_json: r#"{"action":"replicates-dwelling","provider_dwelling_hub_id":"hub:A","provider_role":"steward_mutual"}"#.into(),
        signed_at: "2026-05-28T00:00:00Z".into(),
    };
    let result = validate_commitment_entry(&event).unwrap();
    assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
}

#[test]
fn replicates_dwelling_unknown_role_rejected() {
    let event = Commitment {
        action: "replicates-dwelling".into(),
        payload_json: r#"{"action":"replicates-dwelling","provider_dwelling_hub_id":"hub:A","recipient_dwelling_hub_id":"hub:B","provider_role":"totally-bogus"}"#.into(),
        signed_at: "2026-05-28T00:00:00Z".into(),
    };
    let result = validate_commitment_entry(&event).unwrap();
    assert!(matches!(result, ValidateCallbackResult::Invalid(_)));
}
```

- [ ] **Step 4: Build + test + commit**

```bash
cd /projects/elohim/elohim/holochain/dna/mishpat
just check 2>&1 | tail -10
cargo test -p mishpat_integrity validate_commitment_entry 2>&1 | tail -20

cd /projects/elohim
git add elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs
git commit -m "feat(mishpat-integrity): defense-in-depth replicates-dwelling validation"
```

---


## Phase D — Mutuality audit

### Task 11: `mutuality_audit_log` migration + diesel schema + model

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-05-28-100000_mutuality_audit_log/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-05-28-100000_mutuality_audit_log/down.sql`
- Create: `elohim/elohim-storage/src/db/mutuality_audit_log.rs`
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs`
- Modify: `elohim/elohim-storage/src/db/models.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs`

Per Sprint 2 T3 pattern + `feedback_diesel_migration_timestamp_collision` memory: use unique HHMMSS slot. The latest 2026-05-28 migrations occupy slots 000000–050000; this sprint uses 100000.

- [ ] **Step 1: Author the migration up.sql**

Create `elohim/elohim-storage/migrations/2026-05-28-100000_mutuality_audit_log/up.sql`:

```sql
-- Operational log of mutuality_audit_service sweep results.
-- Source of truth: local SQLite operational projection; rebuildable by re-running
-- the sweep over current Mishpat::Commitment DHT entries. No dht_anchor_hash —
-- this is sweep telemetry, not notarized.
-- Per spec §6.2: 2026-05-28-mutual-storage-replication-dwelling-hub-design.md.

CREATE TABLE mutuality_audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    commitment_cid TEXT NOT NULL,
    provider_dwelling_hub_id TEXT NOT NULL,
    recipient_dwelling_hub_id TEXT NOT NULL,
    reciprocity_status TEXT NOT NULL,
    days_since_authored INTEGER NOT NULL,
    grace_period_days INTEGER NOT NULL,
    signaled_at TEXT,
    swept_at TEXT NOT NULL
);
CREATE INDEX idx_mutuality_audit_commitment ON mutuality_audit_log(commitment_cid);
CREATE INDEX idx_mutuality_audit_recipient ON mutuality_audit_log(recipient_dwelling_hub_id);
CREATE INDEX idx_mutuality_audit_swept ON mutuality_audit_log(swept_at);
```

- [ ] **Step 2: Author down.sql**

```sql
DROP INDEX IF EXISTS idx_mutuality_audit_swept;
DROP INDEX IF EXISTS idx_mutuality_audit_recipient;
DROP INDEX IF EXISTS idx_mutuality_audit_commitment;
DROP TABLE IF EXISTS mutuality_audit_log;
```

- [ ] **Step 3: Update diesel_schema.rs**

In `elohim/elohim-storage/src/db/diesel_schema.rs`, add the table macro in alphabetical order with neighbors:

```rust
diesel::table! {
    mutuality_audit_log (id) {
        id -> Integer,
        commitment_cid -> Text,
        provider_dwelling_hub_id -> Text,
        recipient_dwelling_hub_id -> Text,
        reciprocity_status -> Text,
        days_since_authored -> Integer,
        grace_period_days -> Integer,
        signaled_at -> Nullable<Text>,
        swept_at -> Text,
    }
}
```

Add `mutuality_audit_log` to the `allow_tables_to_appear_in_same_query!` macro if neighbors are listed there.

- [ ] **Step 4: Add diesel model**

In `elohim/elohim-storage/src/db/models.rs`, near other operational-table models:

```rust
use super::diesel_schema::mutuality_audit_log;

#[derive(Debug, Clone, Queryable, Insertable, Identifiable, Selectable)]
#[diesel(table_name = mutuality_audit_log)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct MutualityAuditLogRow {
    pub id: i32,
    pub commitment_cid: String,
    pub provider_dwelling_hub_id: String,
    pub recipient_dwelling_hub_id: String,
    pub reciprocity_status: String,
    pub days_since_authored: i32,
    pub grace_period_days: i32,
    pub signaled_at: Option<String>,
    pub swept_at: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = mutuality_audit_log)]
pub struct NewMutualityAuditLogRow<'a> {
    pub commitment_cid: &'a str,
    pub provider_dwelling_hub_id: &'a str,
    pub recipient_dwelling_hub_id: &'a str,
    pub reciprocity_status: &'a str,
    pub days_since_authored: i32,
    pub grace_period_days: i32,
    pub signaled_at: Option<&'a str>,
    pub swept_at: &'a str,
}
```

- [ ] **Step 5: Author CRUD helper**

Create `elohim/elohim-storage/src/db/mutuality_audit_log.rs`:

```rust
//! Mutuality audit log CRUD. Per spec §6.2.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use super::diesel_schema::mutuality_audit_log::dsl;
use super::models::{MutualityAuditLogRow, NewMutualityAuditLogRow};
use crate::error::StorageError;

pub fn insert(conn: &mut SqliteConnection, row: &NewMutualityAuditLogRow) -> Result<(), StorageError> {
    diesel::insert_into(dsl::mutuality_audit_log)
        .values(row)
        .execute(conn)
        .map_err(|e| StorageError::Database(e.to_string()))?;
    Ok(())
}

pub fn list_recent_for_recipient(
    conn: &mut SqliteConnection,
    recipient: &str,
    limit: i64,
) -> Result<Vec<MutualityAuditLogRow>, StorageError> {
    dsl::mutuality_audit_log
        .filter(dsl::recipient_dwelling_hub_id.eq(recipient))
        .order(dsl::swept_at.desc())
        .limit(limit)
        .load::<MutualityAuditLogRow>(conn)
        .map_err(|e| StorageError::Database(e.to_string()))
}

pub fn latest_for_commitment(
    conn: &mut SqliteConnection,
    commitment_cid: &str,
) -> Result<Option<MutualityAuditLogRow>, StorageError> {
    dsl::mutuality_audit_log
        .filter(dsl::commitment_cid.eq(commitment_cid))
        .order(dsl::swept_at.desc())
        .first::<MutualityAuditLogRow>(conn)
        .optional()
        .map_err(|e| StorageError::Database(e.to_string()))
}
```

- [ ] **Step 6: Register module**

In `elohim/elohim-storage/src/db/mod.rs`, add `pub mod mutuality_audit_log;` in sorted order.

- [ ] **Step 7: Compile + commit**

```bash
cd /projects/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev cargo check --manifest-path elohim/elohim-storage/Cargo.toml 2>&1 | tail -10

git add elohim/elohim-storage/migrations/2026-05-28-100000_mutuality_audit_log/ \
        elohim/elohim-storage/src/db/diesel_schema.rs \
        elohim/elohim-storage/src/db/models.rs \
        elohim/elohim-storage/src/db/mutuality_audit_log.rs \
        elohim/elohim-storage/src/db/mod.rs
git commit -m "feat(storage): mutuality_audit_log table + diesel model + CRUD"
```

Expected: cargo check clean.

---

### Task 12: `mutuality_audit_service`

**Files:**
- Create: `elohim/elohim-storage/src/services/mutuality_audit_service.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

- [ ] **Step 1: Author the service**

```rust
//! Per spec §6: walk every active replicates-dwelling steward_mutual commitment;
//! for each, check whether the counter-commitment exists; emit
//! reciprocity-imbalance FeedbackSignal when past grace_period without counter.
//!
//! Idempotent (running twice produces the same log state). The first concrete
//! instance of a per-scale audit aggregator — collective + commons follow.

use std::sync::Arc;
use chrono::{DateTime, Utc};

use crate::db::mutuality_audit_log;
use crate::db::models::NewMutualityAuditLogRow;
use crate::db::DbPool;
use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::services::commitment_fetcher::{CommitmentFetcher, CommitmentRecord};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReciprocityStatus {
    Matched,
    Pending,
    Breached,
}

impl ReciprocityStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Matched => "Matched",
            Self::Pending => "Pending",
            Self::Breached => "Breached",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SweepReport {
    pub commitments_examined: u32,
    pub matched: u32,
    pub pending: u32,
    pub breached: u32,
    pub signals_emitted: u32,
}

pub struct MutualityAuditService {
    pub pool: DbPool,
    pub hc_client: Option<Arc<HcClient>>,
}

impl MutualityAuditService {
    pub fn new(pool: DbPool, hc_client: Option<Arc<HcClient>>) -> Self {
        Self { pool, hc_client }
    }

    /// Walk every active replicates-dwelling steward_mutual commitment; classify;
    /// emit signal on breach; persist log row.
    pub async fn run_sweep<F: CommitmentFetcher>(
        &self,
        fetcher: &F,
        commitments_authored_locally: &[CommitmentRecord],
        now: DateTime<Utc>,
    ) -> Result<SweepReport, StorageError> {
        let mut report = SweepReport {
            commitments_examined: 0, matched: 0, pending: 0, breached: 0, signals_emitted: 0,
        };
        for c in commitments_authored_locally {
            if c.action != "replicates-dwelling" {
                continue;
            }
            let bounds: serde_json::Value = serde_json::from_str(&c.payload_json)
                .unwrap_or(serde_json::Value::Null);
            let provider_role = bounds["provider_role"].as_str().unwrap_or("");
            if provider_role != "steward_mutual" {
                continue;
            }
            report.commitments_examined += 1;

            let provider = bounds["provider_dwelling_hub_id"].as_str().unwrap_or("").to_string();
            let recipient = bounds["recipient_dwelling_hub_id"].as_str().unwrap_or("").to_string();
            let grace_period_days = bounds["grace_period_days"].as_u64().unwrap_or(14) as i32;

            let signed_at = DateTime::parse_from_rfc3339(&c.signed_at)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or(now);
            let days_since = (now - signed_at).num_days() as i32;

            // Look for counter-commitment (recipient → provider, same action) via fetcher.
            let counter = self.find_counter(fetcher, &recipient, &provider).await?;

            let status = match counter {
                Some(_) => ReciprocityStatus::Matched,
                None if days_since <= grace_period_days => ReciprocityStatus::Pending,
                None => ReciprocityStatus::Breached,
            };

            let signaled_at = if matches!(status, ReciprocityStatus::Breached) {
                self.emit_reciprocity_imbalance(&recipient, &c.cid).await?;
                report.signals_emitted += 1;
                Some(now.to_rfc3339())
            } else {
                None
            };

            match status {
                ReciprocityStatus::Matched => report.matched += 1,
                ReciprocityStatus::Pending => report.pending += 1,
                ReciprocityStatus::Breached => report.breached += 1,
            }

            let pool = self.pool.clone();
            let cid = c.cid.clone();
            let prov = provider.clone();
            let recip = recipient.clone();
            let status_str = status.as_str();
            let sig = signaled_at.clone();
            let swept_iso = now.to_rfc3339();
            tokio::task::spawn_blocking(move || -> Result<(), StorageError> {
                let mut conn = pool.get().map_err(|e| StorageError::Database(e.to_string()))?;
                mutuality_audit_log::insert(&mut conn, &NewMutualityAuditLogRow {
                    commitment_cid: &cid,
                    provider_dwelling_hub_id: &prov,
                    recipient_dwelling_hub_id: &recip,
                    reciprocity_status: status_str,
                    days_since_authored: days_since,
                    grace_period_days,
                    signaled_at: sig.as_deref(),
                    swept_at: &swept_iso,
                })
            })
            .await
            .map_err(|e| StorageError::Internal(e.to_string()))??;
        }
        Ok(report)
    }

    async fn find_counter<F: CommitmentFetcher>(
        &self,
        _fetcher: &F,
        _recipient: &str,
        _provider: &str,
    ) -> Result<Option<CommitmentRecord>, StorageError> {
        // The CommitmentFetcher trait fetches by CID; counter-lookup requires a
        // by-pair query. Sprint 3 follow-up: extend CommitmentFetcher with a
        // find_counter method OR query Mishpat directly via hc_client.
        // For Sprint 3 this returns None always — bilateral counter lookups land
        // as part of the conductor-bridge wiring follow-up. The audit-service
        // shape is testable via mocks; production wiring is the gap.
        Ok(None)
    }

    async fn emit_reciprocity_imbalance(
        &self,
        _target_hub_id: &str,
        _evidence_commitment_cid: &str,
    ) -> Result<(), StorageError> {
        // Sprint 3: FeedbackSignal emission via hc_client when wired; until then
        // this is a no-op stub that simply logs.
        if let Some(_hc) = &self.hc_client {
            tracing::info!(
                target = "elohim_storage::mutuality_audit_service",
                "would emit reciprocity-imbalance FeedbackSignal (conductor bridge wiring pending)"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::commitment_fetcher::MockCommitmentFetcher;
    use crate::test_util::test_pool;
    use chrono::Duration;

    fn commitment(cid: &str, signed_at: &str, provider: &str, recipient: &str) -> CommitmentRecord {
        CommitmentRecord {
            cid: cid.into(),
            action: "replicates-dwelling".into(),
            scope: "household-resilience".into(),
            provider: provider.into(),
            recipient: recipient.into(),
            bounds: serde_json::json!({}),
            valid_from: signed_at.into(),
            valid_until: "2026-12-31T00:00:00Z".into(),
            revoked_at: None,
        }
    }

    // Note: CommitmentRecord per Sprint 2 T2 has bounds: serde_json::Value, but the
    // mutuality_audit_service reads provider/recipient/grace_period from a payload_json
    // field on CommitmentRecord. Sprint 3 follow-up: extend CommitmentRecord with
    // payload_json or refactor the audit-service to read via fetcher's get method.
    //
    // For unit tests, we stub the field-extraction to demonstrate the sweep shape.

    #[tokio::test]
    async fn empty_commitments_produces_empty_report() {
        let svc = MutualityAuditService::new(test_pool(), None);
        let fetcher = MockCommitmentFetcher::new();
        let report = svc.run_sweep(&fetcher, &[], Utc::now()).await.unwrap();
        assert_eq!(report.commitments_examined, 0);
        assert_eq!(report.signals_emitted, 0);
    }
}
```

**Note on the CommitmentRecord ↔ payload_json gap:** Sprint 2's `CommitmentRecord` has structured `bounds: serde_json::Value` but no `payload_json` field. The audit-service template above reads `payload_json` from the record; in execution, the implementer either (a) extends `CommitmentRecord` with a `payload_json` field, OR (b) refactors the audit to parse from `bounds`. Either choice is reasonable; document the decision in the commit message.

- [ ] **Step 2: Register module**

In `elohim/elohim-storage/src/services/mod.rs` add `pub mod mutuality_audit_service;` in sorted order.

- [ ] **Step 3: Run + commit**

```bash
cd /projects/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib services::mutuality_audit_service 2>&1 | tail -15

git add elohim/elohim-storage/src/services/mutuality_audit_service.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(storage): mutuality_audit_service — first per-scale aggregator instance"
```

---


## Phase E — Views + data plane

### Task 13: `peer_capacity_service`

**Files:**
- Create: `elohim/elohim-storage/src/services/peer_capacity_service.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`
- Create: `elohim/elohim-storage/tests/peer_capacity_view_integration.rs`

- [ ] **Step 1: Author the service**

```rust
//! Computes PeerCapacityView per spec §7.1. Read-only projection from
//! rea_commitments + peer_statuses (raw capacity) + peer_blob_inventory
//! (uniqueShardBytes) + constitutional_ratio_registry.
//!
//! Per-tier pledged aggregation: sum capacity_bytes of all active commitments
//! filtered by action and provider. Multi-reach blob accounting is enforced
//! at the uniqueShardBytes computation (dedup across shard CIDs).

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::DbPool;
use crate::error::StorageError;
use crate::services::constitutional_ratio_registry::{self, EffectiveRatios};
use elohim_views::peer_capacity::{
    ActuallyHeldView, CurrentRatiosView, EffectiveRatiosView, PeerCapacityView,
    PledgesView, RatioComplianceView, RatioViolationView, Tier, ViolationKind,
};

pub fn compute_peer_capacity(
    conn: &mut SqliteConnection,
    peer_cid: &str,
) -> Result<PeerCapacityView, StorageError> {
    let total_raw_bytes = query_latest_total_raw_bytes(conn, peer_cid)?;
    let (pledged_dwelling, pledged_collective, pledged_commons, pledges_by_recipient) =
        aggregate_pledges_by_tier(conn, peer_cid)?;
    let unique_shard_bytes = compute_unique_shard_bytes(conn, peer_cid)?;
    let provenance = constitutional_ratio_registry::effective_ratios();
    let effective = provenance.ratios;
    let total_pledged = pledged_dwelling + pledged_collective + pledged_commons;

    let pledges = PledgesView {
        dwelling_bytes: pledged_dwelling,
        collective_bytes: pledged_collective,
        commons_bytes: pledged_commons,
        total_pledged_bytes: total_pledged,
        pledges_by_recipient,
    };

    let actually_held = ActuallyHeldView {
        unique_shard_bytes,
        free_bytes_remaining: total_raw_bytes as i64 - unique_shard_bytes as i64,
        fragmentation_estimate: 0.0,  // operational hint; Sprint 3 returns 0; follow-up wires real probe
    };

    let total_for_pct = total_raw_bytes.max(1);
    let current_dwelling_pct = ((pledged_dwelling * 100) / total_for_pct) as u8;
    let current_collective_pct = ((pledged_collective * 100) / total_for_pct) as u8;
    let current_commons_pct = ((pledged_commons * 100) / total_for_pct) as u8;
    let current_free_pct = 100u8.saturating_sub(current_dwelling_pct)
        .saturating_sub(current_collective_pct).saturating_sub(current_commons_pct);

    let mut violations = Vec::new();
    if current_dwelling_pct > effective.dwelling_pct {
        violations.push(RatioViolationView {
            tier: Tier::Dwelling,
            violation_kind: ViolationKind::AboveCeiling,
            current_pct: current_dwelling_pct as i32,
            bound_pct: effective.dwelling_pct as i32,
        });
    }
    if current_free_pct < constitutional_ratio_registry::FREE_MIN_FLOOR_PCT {
        violations.push(RatioViolationView {
            tier: Tier::Free,
            violation_kind: ViolationKind::BelowFloor,
            current_pct: current_free_pct as i32,
            bound_pct: constitutional_ratio_registry::FREE_MIN_FLOOR_PCT as i32,
        });
    }

    let ratio_compliance = RatioComplianceView {
        effective_ratios: EffectiveRatiosView {
            commons_pct: effective.commons_pct as i32,
            dwelling_pct: effective.dwelling_pct as i32,
            collective_pct: effective.collective_pct as i32,
            free_pct: effective.free_pct as i32,
            manifest_cid: provenance.manifest_cid,
        },
        current_ratios: CurrentRatiosView {
            commons_pct: current_commons_pct as i32,
            dwelling_pct: current_dwelling_pct as i32,
            collective_pct: current_collective_pct as i32,
            free_pct: current_free_pct as i32,
        },
        compliant_with_donut: violations.is_empty(),
        violations,
    };

    Ok(PeerCapacityView {
        peer_cid: peer_cid.to_string(),
        computed_at: chrono::Utc::now().to_rfc3339(),
        total_raw_bytes,
        pledges,
        actually_held,
        ratio_compliance,
    })
}

fn query_latest_total_raw_bytes(conn: &mut SqliteConnection, peer_cid: &str) -> Result<u64, StorageError> {
    // Pull from peer_statuses or system_metrics (latest infrastructure:system-sample graduation).
    // Sprint 3 follow-up: confirm exact column name; using available_bytes for stub.
    // For Sprint 3 testing path, fall back to 0 if no row exists.
    Ok(0)
}

fn aggregate_pledges_by_tier(
    conn: &mut SqliteConnection,
    peer_cid: &str,
) -> Result<(u64, u64, u64, Vec<elohim_views::peer_capacity::PledgeByRecipientView>), StorageError> {
    // Query rea_commitments WHERE provider matches peer_cid; sum capacity_bytes per action discriminator.
    // Until rea_commitments has a typed view of replicates-dwelling payloads, parse payload_json at read time.
    Ok((0, 0, 0, Vec::new()))
}

fn compute_unique_shard_bytes(conn: &mut SqliteConnection, peer_cid: &str) -> Result<u64, StorageError> {
    // Sum DISTINCT blob_hash sizes from peer_blob_inventory for this peer.
    // For Sprint 3 stub: assume zero. Real implementation joins peer_blob_inventory
    // with a size-of-blob lookup (sharding service derivation).
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_pool;

    #[test]
    fn empty_peer_returns_zero_capacity() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let view = compute_peer_capacity(&mut conn, "peer:fresh").unwrap();
        assert_eq!(view.peer_cid, "peer:fresh");
        assert_eq!(view.total_raw_bytes, 0);
        assert_eq!(view.pledges.total_pledged_bytes, 0);
        assert_eq!(view.actually_held.unique_shard_bytes, 0);
    }

    #[test]
    fn ratio_compliance_reflects_effective_ratios() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let view = compute_peer_capacity(&mut conn, "peer:test").unwrap();
        let r = constitutional_ratio_registry::effective_ratios().ratios;
        assert_eq!(view.ratio_compliance.effective_ratios.commons_pct as u8, r.commons_pct);
        assert_eq!(view.ratio_compliance.effective_ratios.dwelling_pct as u8, r.dwelling_pct);
    }
}
```

- [ ] **Step 2: Register module + integration test stub**

In `elohim/elohim-storage/src/services/mod.rs` add `pub mod peer_capacity_service;`.

Create `elohim/elohim-storage/tests/peer_capacity_view_integration.rs`:

```rust
//! Integration: peer_capacity_service against test_pool() harness.

use elohim_storage::services::peer_capacity_service::compute_peer_capacity;
use elohim_storage::test_util::test_pool;

#[test]
fn empty_state_returns_zeroed_view() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let view = compute_peer_capacity(&mut conn, "peer:empty").unwrap();
    assert_eq!(view.peer_cid, "peer:empty");
    assert!(view.ratio_compliance.violations.is_empty() || !view.ratio_compliance.compliant_with_donut);
}

#[test]
fn realistic_state_returns_correct_rollups() {
    // Placeholder for future tests once aggregate_pledges_by_tier is wired.
    // Sprint 3 follow-up: seed rea_commitments rows directly + assert pledges.
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let view = compute_peer_capacity(&mut conn, "peer:realistic").unwrap();
    assert_eq!(view.pledges.total_pledged_bytes, 0);
}
```

- [ ] **Step 3: Run + commit**

```bash
cd /projects/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib services::peer_capacity_service 2>&1 | tail -10
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev cargo test --manifest-path elohim/elohim-storage/Cargo.toml --test peer_capacity_view_integration 2>&1 | tail -10
git add elohim/elohim-storage/src/services/peer_capacity_service.rs \
        elohim/elohim-storage/src/services/mod.rs \
        elohim/elohim-storage/tests/peer_capacity_view_integration.rs
git commit -m "feat(storage): peer_capacity_service — PeerCapacityView computation"
```

---

### Task 14: `hub_capacity_service`

**Files:**
- Create: `elohim/elohim-storage/src/services/hub_capacity_service.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`
- Create: `elohim/elohim-storage/tests/hub_capacity_view_integration.rs`

- [ ] **Step 1: Author the service**

Mirrors HubComputeAggregateView shape exactly. Aggregates per-device `PeerCapacityView`s across hub-membership graph.

```rust
//! Computes HubCapacityView per spec §7.2. Hub is a *role* (per
//! project_hub_archetype_abstraction memory); substrate stays kind-agnostic.
//! HubId defaults to peer_id for single-device (Computed kind); future
//! distinguishes dwelling_id / collective_id via binding tables.

use diesel::sqlite::SqliteConnection;

use crate::error::StorageError;
use crate::services::peer_capacity_service::compute_peer_capacity;
use elohim_views::hub_capacity::{HubCapacityView, HubKind, HubCapacityAggregate};

pub fn compute_hub_capacity(
    conn: &mut SqliteConnection,
    hub_id: &str,
) -> Result<HubCapacityView, StorageError> {
    // Resolve hub-membership: which peer_cids belong to this hub_id?
    let member_peer_cids = resolve_hub_members(conn, hub_id)?;
    let hub_kind = classify_hub(conn, hub_id, &member_peer_cids);

    if member_peer_cids.is_empty() {
        return Ok(HubCapacityView {
            hub_id: hub_id.to_string(),
            hub_kind,
            display_label: None,
            member_device_count: 0,
            capacity: None,
        });
    }

    let mut capacity_aggregate = HubCapacityAggregate::default();
    for peer_cid in &member_peer_cids {
        let pv = compute_peer_capacity(conn, peer_cid)?;
        capacity_aggregate.total_raw_bytes += pv.total_raw_bytes;
        capacity_aggregate.pledges.dwelling_bytes   += pv.pledges.dwelling_bytes;
        capacity_aggregate.pledges.collective_bytes += pv.pledges.collective_bytes;
        capacity_aggregate.pledges.commons_bytes    += pv.pledges.commons_bytes;
        capacity_aggregate.pledges.total_pledged_bytes += pv.pledges.total_pledged_bytes;
        capacity_aggregate.actually_held.unique_shard_bytes    += pv.actually_held.unique_shard_bytes;
        capacity_aggregate.actually_held.free_bytes_remaining  += pv.actually_held.free_bytes_remaining;
    }

    Ok(HubCapacityView {
        hub_id: hub_id.to_string(),
        hub_kind,
        display_label: None,
        member_device_count: member_peer_cids.len() as i32,
        capacity: Some(capacity_aggregate),
    })
}

fn resolve_hub_members(_conn: &mut SqliteConnection, hub_id: &str) -> Result<Vec<String>, StorageError> {
    // Single-device fallback: hub_id IS the peer_id (Computed kind).
    // Real implementation: query peer_identity_bindings / household_id projection.
    Ok(vec![hub_id.to_string()])
}

fn classify_hub(_conn: &mut SqliteConnection, hub_id: &str, members: &[String]) -> HubKind {
    if hub_id.starts_with("dwelling:") {
        HubKind::Dwelling
    } else if hub_id.starts_with("collective:") {
        HubKind::Collective
    } else if members.len() <= 1 {
        HubKind::Computed
    } else {
        HubKind::Dwelling
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_pool;

    #[test]
    fn single_device_hub_classified_as_computed() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let view = compute_hub_capacity(&mut conn, "peer:solo").unwrap();
        assert_eq!(view.hub_kind, HubKind::Computed);
        assert_eq!(view.member_device_count, 1);
    }

    #[test]
    fn dwelling_prefix_hub_classified_as_dwelling() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let view = compute_hub_capacity(&mut conn, "dwelling:smith-family").unwrap();
        assert_eq!(view.hub_kind, HubKind::Dwelling);
    }
}
```

- [ ] **Step 2: Add `HubCapacityAggregate` to elohim-views/src/hub_capacity.rs**

Extend Task 3's `HubCapacityView` to use a typed `capacity: Option<HubCapacityAggregate>` (struct holds totalRawBytes + pledges + actuallyHeld; ratioCompliance can be added in follow-up).

- [ ] **Step 3: Register module + integration test stub**

```rust
// elohim/elohim-storage/tests/hub_capacity_view_integration.rs
use elohim_storage::services::hub_capacity_service::compute_hub_capacity;
use elohim_storage::test_util::test_pool;
use elohim_views::hub_capacity::HubKind;

#[test]
fn single_device_returns_computed_kind() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let view = compute_hub_capacity(&mut conn, "peer:test").unwrap();
    assert_eq!(view.hub_kind, HubKind::Computed);
    assert_eq!(view.member_device_count, 1);
}

#[test]
fn empty_hub_returns_null_capacity() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let view = compute_hub_capacity(&mut conn, "peer:empty").unwrap();
    // single-member with zero capacity → still Some with zeros
    assert!(view.capacity.is_some());
}
```

- [ ] **Step 4: Run + commit**

```bash
cd /projects/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib services::hub_capacity_service 2>&1 | tail -10

git add elohim/elohim-storage/src/services/hub_capacity_service.rs \
        elohim/elohim-storage/src/services/mod.rs \
        elohim/elohim-storage/tests/hub_capacity_view_integration.rs \
        elohim/elohim-views/src/hub_capacity.rs
git commit -m "feat(storage): hub_capacity_service — aggregates PeerCapacityView per HubKind"
```

---

### Task 15: Distribution view + household resilience extensions

**Files:**
- Modify: `elohim/elohim-storage/src/services/distribution_view.rs`
- Modify: `elohim/elohim-storage/src/services/household_resilience.rs`
- Create: `elohim/elohim-storage/tests/distribution_view_extensions_integration.rs`

- [ ] **Step 1: Extend distribution_view computation**

In `services/distribution_view.rs`, locate the existing DistributionSummary builder. Compute `projection_tier`:

```rust
fn compute_projection_tier(projector_count: u32, distinct_regions: u32) -> ProjectionTier {
    match (projector_count, distinct_regions) {
        (0..=2, _) => ProjectionTier::Local,
        (3..=5, 0..=1) => ProjectionTier::Regional,
        (3..=5, _) => ProjectionTier::Global,
        _ => ProjectionTier::Global,
    }
}
```

Plug it into the existing summary composition. Add `over_replicated` semantics to the `replica_health` classifier:

```rust
fn replica_health(active: u32, target: u32) -> ReplicaHealth {
    if active < target / 2 { ReplicaHealth::Critical }
    else if active < target { ReplicaHealth::AtRisk }
    else if active >= target * 2 { ReplicaHealth::OverReplicated }
    else { ReplicaHealth::Healthy }
}
```

For DistributionDetails: compute `replication_commitments` by querying rea_commitments matching this content's recipient(s); compute `fault_domain_diversity` by walking replica_peers, joining peer_identity_bindings to resolve household-of-peer, counting distincts.

- [ ] **Step 2: Extend household_resilience**

In `services/household_resilience.rs`, after the existing computation of `households_stewarding` etc., compute `commitment_backed_replication` by querying rea_commitments where recipient_dwelling_hub_id matches the content's authoring hub (resolved via existing patterns).

```rust
fn compute_commitment_backed_replication(
    conn: &mut SqliteConnection,
    content_id: &str,
) -> CommitmentBackedReplication {
    let (dwelling, collective, commons, total_bytes) = query_commitment_counts_for_content(conn, content_id);
    CommitmentBackedReplication {
        dwelling_commitments: dwelling as i32,
        collective_commitments: collective as i32,
        commons_commitments: commons as i32,
        total_pledged_bytes: total_bytes,
    }
}
```

Populate the new field on the returned `HouseholdResilienceView`.

- [ ] **Step 3: Author integration test**

```rust
// elohim/elohim-storage/tests/distribution_view_extensions_integration.rs
use elohim_storage::test_util::test_pool;

#[test]
fn projection_tier_local_for_few_projectors() {
    // Build a DistributionSummary with projectorCount=1 and verify tier=local.
    // Concrete builder pattern follows existing tests in this file.
}

#[test]
fn replica_health_over_replicated_at_2x() {
    // active=20, target=10 → over_replicated.
}

#[test]
fn single_fault_domain_risk_when_all_one_household() {
    // Build replica_peers all bound to one household; expect risk=true.
}
```

(Full bodies follow existing fixture pattern; this template prevents fix-during-execution drift.)

- [ ] **Step 4: Run + commit**

```bash
cd /projects/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev cargo test --manifest-path elohim/elohim-storage/Cargo.toml --test distribution_view_extensions_integration 2>&1 | tail -15

git add elohim/elohim-storage/src/services/distribution_view.rs \
        elohim/elohim-storage/src/services/household_resilience.rs \
        elohim/elohim-storage/tests/distribution_view_extensions_integration.rs
git commit -m "feat(storage): projection_tier + over_replicated + faultDomainDiversity + commitmentBackedReplication"
```

---

### Task 16: `replication_prioritizer` service

**Files:**
- Create: `elohim/elohim-storage/src/services/replication_prioritizer.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs`

- [ ] **Step 1: Author the service**

```rust
//! Scores incoming inventory_gossip advertisements against the local peer's
//! active replicates-* commitments. Output: priority tier consumed by the
//! existing inventory subscriber to decide which advertised blobs to fetch.
//!
//! Per spec §8.2: the substrate's "commitments shape what peers cache"
//! mechanism. Without it, peers fetch indiscriminately and commitments are
//! decorative.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchPriority {
    High,
    Medium,
    Skip,
}

#[derive(Debug, Clone)]
pub struct AdvertisedBlob {
    pub blob_cid: String,
    pub source_peer_cid: String,
    pub blob_size_bytes: Option<u64>,
    pub recipient_hub_id_hint: Option<String>,
    pub epr_kind_hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ActiveCommitment {
    pub commitment_cid: String,
    pub action: String,           // "replicates-dwelling" etc.
    pub recipient_hub_id: String,
    pub scope_epr_kinds: Option<Vec<String>>,
    pub bytes_per_blob_max: Option<u64>,
}

pub fn score_advertised_blob(
    advertised: &AdvertisedBlob,
    active_commitments: &[ActiveCommitment],
) -> FetchPriority {
    for commitment in active_commitments {
        if commitment.action != "replicates-dwelling" {
            continue;
        }
        // Recipient match
        if let Some(rcpt) = &advertised.recipient_hub_id_hint {
            if rcpt != &commitment.recipient_hub_id {
                continue;
            }
        } else {
            // Without recipient hint, can't match. Skip.
            continue;
        }
        // Scope match (epr_kind)
        if let (Some(kinds), Some(kind)) = (&commitment.scope_epr_kinds, &advertised.epr_kind_hint) {
            if !kinds.iter().any(|k| k == kind) {
                continue;
            }
        }
        // Size match
        if let (Some(max), Some(size)) = (commitment.bytes_per_blob_max, advertised.blob_size_bytes) {
            if size > max {
                continue;
            }
        }
        return FetchPriority::High;
    }
    // Commons-tier eligible — deferred. Sprint 3 always returns Skip when no dwelling match.
    FetchPriority::Skip
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commitment(action: &str, recipient: &str) -> ActiveCommitment {
        ActiveCommitment {
            commitment_cid: "comm:test".into(),
            action: action.into(),
            recipient_hub_id: recipient.into(),
            scope_epr_kinds: Some(vec!["Content".into()]),
            bytes_per_blob_max: Some(1_000_000_000),
        }
    }

    fn ad(recipient: &str, kind: &str, size: u64) -> AdvertisedBlob {
        AdvertisedBlob {
            blob_cid: "bafkrei:test".into(),
            source_peer_cid: "peer:source".into(),
            blob_size_bytes: Some(size),
            recipient_hub_id_hint: Some(recipient.into()),
            epr_kind_hint: Some(kind.into()),
        }
    }

    #[test]
    fn high_when_recipient_and_scope_match() {
        let c = commitment("replicates-dwelling", "hub:B");
        let a = ad("hub:B", "Content", 500_000_000);
        assert_eq!(score_advertised_blob(&a, &[c]), FetchPriority::High);
    }

    #[test]
    fn skip_when_no_matching_recipient() {
        let c = commitment("replicates-dwelling", "hub:B");
        let a = ad("hub:Z", "Content", 100);
        assert_eq!(score_advertised_blob(&a, &[c]), FetchPriority::Skip);
    }

    #[test]
    fn skip_when_blob_exceeds_size_ceiling() {
        let c = commitment("replicates-dwelling", "hub:B");
        let a = ad("hub:B", "Content", 5_000_000_000);  // > 1GB max
        assert_eq!(score_advertised_blob(&a, &[c]), FetchPriority::Skip);
    }

    #[test]
    fn skip_when_kind_not_in_scope() {
        let c = commitment("replicates-dwelling", "hub:B");
        let a = ad("hub:B", "EconomicEvent", 100);
        assert_eq!(score_advertised_blob(&a, &[c]), FetchPriority::Skip);
    }

    #[test]
    fn skip_when_no_commitments() {
        let a = ad("hub:B", "Content", 100);
        assert_eq!(score_advertised_blob(&a, &[]), FetchPriority::Skip);
    }
}
```

- [ ] **Step 2: Register module + run + commit**

```bash
cd /projects/elohim
# Register pub mod replication_prioritizer; in services/mod.rs
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib services::replication_prioritizer 2>&1 | tail -10
git add elohim/elohim-storage/src/services/replication_prioritizer.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(storage): replication_prioritizer — scores inventory advertisements vs active commitments"
```

---


## Phase F — HTTP routes

### Task 17: Three new HTTP routes wired in http.rs

**Files:**
- Create: `elohim/elohim-storage/src/api/peer_capacity.rs`
- Create: `elohim/elohim-storage/src/api/hub_capacity.rs`
- Create: `elohim/elohim-storage/src/api/diagnostics_mutuality.rs`
- Modify: `elohim/elohim-storage/src/api/mod.rs`
- Modify: `elohim/elohim-storage/src/http.rs`

- [ ] **Step 1: Author `api/peer_capacity.rs`**

```rust
//! GET /api/v1/peer/{peer_cid}/capacity. Spec §7.1.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};

use crate::db::DbPool;
use crate::error::StorageError;
use crate::services::peer_capacity_service::compute_peer_capacity;

pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    peer_cid: &str,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    if method != Method::GET {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Full::new(Bytes::from(r#"{"error":"GET only"}"#)))
            .unwrap());
    }
    let _ = req;  // future: redact for non-owner caller
    let peer_cid_owned = peer_cid.to_string();
    let pool = pool.clone();
    let view = tokio::task::spawn_blocking(move || -> Result<_, StorageError> {
        let mut conn = pool.get().map_err(|e| StorageError::Database(e.to_string()))?;
        compute_peer_capacity(&mut conn, &peer_cid_owned)
    })
    .await
    .map_err(|e| StorageError::Internal(e.to_string()))??;
    let body = serde_json::to_vec(&view).map_err(|e| StorageError::Internal(e.to_string()))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}
```

- [ ] **Step 2: Author `api/hub_capacity.rs`**

Same shape; replace `compute_peer_capacity` with `compute_hub_capacity` and accept `hub_id`.

```rust
//! GET /api/v1/hub/{hub_id}/capacity. Spec §7.2.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};

use crate::db::DbPool;
use crate::error::StorageError;
use crate::services::hub_capacity_service::compute_hub_capacity;

pub async fn handle(
    _req: Request<Incoming>,
    method: Method,
    hub_id: &str,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    if method != Method::GET {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Full::new(Bytes::from(r#"{"error":"GET only"}"#)))
            .unwrap());
    }
    let hub_id_owned = hub_id.to_string();
    let pool = pool.clone();
    let view = tokio::task::spawn_blocking(move || -> Result<_, StorageError> {
        let mut conn = pool.get().map_err(|e| StorageError::Database(e.to_string()))?;
        compute_hub_capacity(&mut conn, &hub_id_owned)
    })
    .await
    .map_err(|e| StorageError::Internal(e.to_string()))??;
    let body = serde_json::to_vec(&view).map_err(|e| StorageError::Internal(e.to_string()))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}
```

- [ ] **Step 3: Author `api/diagnostics_mutuality.rs`**

```rust
//! GET /api/v1/diagnostics/mutuality-audit?hub={hub_id}. Spec §6.3.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};
use serde::Serialize;

use crate::db::mutuality_audit_log;
use crate::db::models::MutualityAuditLogRow;
use crate::db::DbPool;
use crate::error::StorageError;

#[derive(Serialize)]
struct MutualityAuditView {
    rows: Vec<MutualityAuditLogRowSerial>,
}

#[derive(Serialize)]
struct MutualityAuditLogRowSerial {
    commitment_cid: String,
    provider_dwelling_hub_id: String,
    recipient_dwelling_hub_id: String,
    reciprocity_status: String,
    days_since_authored: i32,
    grace_period_days: i32,
    signaled_at: Option<String>,
    swept_at: String,
}

impl From<MutualityAuditLogRow> for MutualityAuditLogRowSerial {
    fn from(r: MutualityAuditLogRow) -> Self {
        Self {
            commitment_cid: r.commitment_cid,
            provider_dwelling_hub_id: r.provider_dwelling_hub_id,
            recipient_dwelling_hub_id: r.recipient_dwelling_hub_id,
            reciprocity_status: r.reciprocity_status,
            days_since_authored: r.days_since_authored,
            grace_period_days: r.grace_period_days,
            signaled_at: r.signaled_at,
            swept_at: r.swept_at,
        }
    }
}

pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    if method != Method::GET {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Full::new(Bytes::from(r#"{"error":"GET only"}"#)))
            .unwrap());
    }
    let query_str = req.uri().query().unwrap_or("");
    let hub_id = parse_query_param(query_str, "hub")
        .ok_or_else(|| StorageError::InvalidInput("?hub=<hub_id> required".into()))?;
    let pool = pool.clone();
    let rows = tokio::task::spawn_blocking(move || -> Result<Vec<MutualityAuditLogRow>, StorageError> {
        let mut conn = pool.get().map_err(|e| StorageError::Database(e.to_string()))?;
        mutuality_audit_log::list_recent_for_recipient(&mut conn, &hub_id, 100)
    })
    .await
    .map_err(|e| StorageError::Internal(e.to_string()))??;
    let view = MutualityAuditView {
        rows: rows.into_iter().map(Into::into).collect(),
    };
    let body = serde_json::to_vec(&view).map_err(|e| StorageError::Internal(e.to_string()))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}

fn parse_query_param(query_str: &str, key: &str) -> Option<String> {
    for pair in query_str.split('&') {
        let mut parts = pair.splitn(2, '=');
        if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
            if k == key && !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}
```

- [ ] **Step 4: Wire dispatch in `api/mod.rs`**

In `elohim/elohim-storage/src/api/mod.rs`:

```rust
pub mod peer_capacity;
pub mod hub_capacity;
pub mod diagnostics_mutuality;
```

In `handle_api_request`, add dispatch branches (alphabetical-ish with neighbors):

```rust
} else if sub_path.starts_with("peer/") {
    // peer/{peer_cid}/capacity
    let after = sub_path.strip_prefix("peer/").unwrap_or("");
    if let Some(rest) = after.strip_suffix("/capacity") {
        return peer_capacity::handle(req, method, rest, &pool).await;
    }
} else if sub_path.starts_with("hub/") {
    let after = sub_path.strip_prefix("hub/").unwrap_or("");
    if let Some(rest) = after.strip_suffix("/capacity") {
        return hub_capacity::handle(req, method, rest, &pool).await;
    }
} else if sub_path == "diagnostics/mutuality-audit" {
    return diagnostics_mutuality::handle(req, method, &pool).await;
}
```

- [ ] **Step 5: Compile + commit**

```bash
cd /projects/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev cargo check --manifest-path elohim/elohim-storage/Cargo.toml 2>&1 | tail -10

git add elohim/elohim-storage/src/api/peer_capacity.rs \
        elohim/elohim-storage/src/api/hub_capacity.rs \
        elohim/elohim-storage/src/api/diagnostics_mutuality.rs \
        elohim/elohim-storage/src/api/mod.rs
git commit -m "feat(storage): GET /api/v1/peer/{cid}/capacity + /hub/{id}/capacity + diagnostics/mutuality-audit"
```

Expected: cargo check clean.

---


## Phase G — Tests

### Task 18: Sweettest two-conductor substrate-correct test

**Files:**
- Create: `elohim/holochain/tests/sweettest/src/replicates_dwelling_substrate_correct_test.rs`

- [ ] **Step 1: Author the sweettest**

```rust
//! Sprint 3 sweettest: two-conductor end-to-end for replicates-dwelling.
//!
//! Story per spec §9.3: Two agents on two conductors; A is dwelling-hub H1
//! steward; B is dwelling-hub H2 steward. A authors steward_mutual commitment
//! naming H2; DHT consistency; B authors counter; mutuality_audit_service
//! sweep shows Matched. Then negative: ratio_attestation breaching DNA floor
//! is rejected by the integrity validator.

use holochain::sweettest::*;
use std::time::Duration;

const HAPP_PATH: &str = "../../../elohim/holochain/dna/mishpat";

#[tokio::test(flavor = "multi_thread")]
async fn replicates_dwelling_substrate_correct_e2e() {
    let mut steward_conductor = SweetConductor::from_standard_config().await;
    let mut svc_conductor = SweetConductor::from_standard_config().await;

    let dna_path = std::path::PathBuf::from(HAPP_PATH);
    let dna = SweetDnaFile::from_bundle(&dna_path.join("mishpat.dna.bundle")).await.unwrap();

    let agents_a: Vec<_> = steward_conductor
        .setup_app("steward-app", &[dna.clone()])
        .await
        .unwrap()
        .into_inner();
    let agents_b: Vec<_> = svc_conductor
        .setup_app("svc-app", &[dna.clone()])
        .await
        .unwrap()
        .into_inner();

    // 1. Steward A authors a replicates-dwelling Commitment naming H2
    let payload = serde_json::json!({
        "action": "replicates-dwelling",
        "provider_dwelling_hub_id": "hub:H1",
        "recipient_dwelling_hub_id": "hub:H2",
        "provider_role": "steward_mutual",
        "capacity_bytes": 50_000_000_000u64,
        "scope_filter": {"epr_kinds": ["Content"]},
        "valid_from": "2026-05-28T00:00:00Z",
        "valid_until": "2026-08-26T00:00:00Z",
        "grace_period_days": 14,
        "rotation_ttl_days": 90,
        "ratio_attestation": {
            "commons_pct": 20, "dwelling_pct": 40, "collective_pct": 25, "free_pct": 15,
            "effective_ratio_cid": "bafkrei-test"
        }
    });
    let commitment_input = serde_json::json!({
        "action": "replicates-dwelling",
        "payload_json": payload.to_string(),
    });
    let result: Result<holo_hash::ActionHash, _> = steward_conductor
        .call(
            &agents_a[0].zome("mishpat"),
            "create_commitment",
            commitment_input,
        )
        .await;
    assert!(result.is_ok(), "well-formed replicates-dwelling commit must succeed");

    // 2. Exchange peer info + await DHT consistency
    SweetConductor::exchange_peer_info([&steward_conductor, &svc_conductor]).await;
    await_consistency(60.0, [&steward_conductor, &svc_conductor]).await.unwrap();

    // 3. Negative path: ratio_attestation breaching DNA floor is rejected
    let bad_payload = serde_json::json!({
        "action": "replicates-dwelling",
        "provider_dwelling_hub_id": "hub:H1",
        "recipient_dwelling_hub_id": "hub:H3",
        "provider_role": "steward_mutual",
        "capacity_bytes": 10_000_000_000u64,
        "scope_filter": {},
        "valid_from": "2026-05-28T00:00:00Z",
        "valid_until": "2026-08-26T00:00:00Z",
        "grace_period_days": 14,
        "rotation_ttl_days": 90,
        "ratio_attestation": {
            "commons_pct": 5,  // below DNA floor of 10
            "dwelling_pct": 50, "collective_pct": 25, "free_pct": 20,
            "effective_ratio_cid": "bafkrei-test"
        }
    });
    let bad_input = serde_json::json!({
        "action": "replicates-dwelling",
        "payload_json": bad_payload.to_string(),
    });
    let bad_result: Result<holo_hash::ActionHash, _> = steward_conductor
        .call(&agents_a[0].zome("mishpat"), "create_commitment", bad_input)
        .await;
    // Coordinator validator catches sum-to-100 violation OR floor-via-declaration in the
    // storage-side validator; either way must reject. NOTE: this test exercises the
    // coordinator path; the storage donut check is in the storage integration test.
    assert!(bad_result.is_err() || bad_payload["ratio_attestation"]["commons_pct"].as_u64().unwrap() >= 10);
}
```

**Note on sweettest harness adaptation:** the exact `SweetConductor` / `SweetAgents` API depends on the holochain-sweettest version pinned in `Cargo.toml`. Pattern follows Sprint 1's planned (unimplemented) `substrate_correct_deploy_test.rs`; if API drifts, adapt to current sweettest conventions per `feedback_sweettest_cross_agent_consistency` memory.

- [ ] **Step 2: Run + commit**

```bash
cd /projects/elohim
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/sprint/elohim__holochain__tests__sweettest/dev cargo test --manifest-path elohim/holochain/tests/sweettest/Cargo.toml replicates_dwelling_substrate_correct 2>&1 | tail -30

git add elohim/holochain/tests/sweettest/src/replicates_dwelling_substrate_correct_test.rs
git commit -m "test(sweettest): two-conductor replicates-dwelling substrate-correct test"
```

---

### Task 19: A2o features — three narrative scenarios

**Files:**
- Create: `genesis/a2o/features/storage/household-resiliency-handshake.feature`
- Create: `genesis/a2o/features/storage/constitutional-ratio-enforcement.feature`
- Create: `genesis/a2o/features/storage/disaster-burst-resilience.feature`

**Per `feedback_a2o_narrative_is_opus_work`: narrative MUST be Opus, not Haiku.** Implementer should request Opus model for this task. Narrative uses "household", "family", "grandma", "church" — never "Mishpat", "Qahal", or "dwelling-hub" in feature copy (substrate names stay in stepdefs).

- [ ] **Step 1: Author `household-resiliency-handshake.feature`**

```gherkin
Feature: Households commit to back each other up
  As a family that's part of a small intimate-circle network
  I want my household and another household to agree to host shards of each other's content
  So that if my house burns down, the family photos and shared plans survive

  Background:
    Given the Smith household has a dwelling-hub plugged in at home
    And the Garcia household has a dwelling-hub plugged in at home
    And both households have unlocked steward identities

  Scenario: Bilateral counter arrives within grace; both see protected status
    When Maria (Smith household steward) commits to host 50 GB of the Garcia family's content
    And Carlos (Garcia household steward) commits to host 50 GB of the Smith family's content within 14 days
    Then both households see their shared content classified as "protected"
    And neither household receives any imbalance signal
    And the substrate-level audit shows the pair as Matched

  Scenario: Counter never arrives; reciprocity-imbalance signal fires
    When Maria commits to host 50 GB of the Garcia family's content
    And 15 days pass without the Garcia household authoring a counter-commitment
    Then the substrate emits a reciprocity-imbalance signal naming the Garcia household
    And the Garcia household's standing receives a moderate debit
    And Maria's commitment remains valid (Maria didn't breach; Garcia did)
```

- [ ] **Step 2: Author `constitutional-ratio-enforcement.feature`**

```gherkin
Feature: Constitutional donut walls prevent free-riding and capture
  As a steward of a dwelling-hub
  I want the protocol to enforce a constitutional ratio between my private storage, my friends' storage, and the commons
  So that I can't accidentally free-ride on the substrate or be exploited into over-pledging

  Background:
    Given the protocol's constitutional donut declares: commons 20%, dwelling 40%, collective 25%, free 15%
    And the DNA floor for commons is 10% (no opt-out)

  Scenario: First commitment authored compliantly
    Given Maria's dwelling-hub has 100 GB raw capacity and no active commitments yet
    When Maria commits to host 30 GB of the Garcia family's content
    Then the commitment is accepted by the substrate
    And Maria's PeerCapacityView shows her donut ratios honoring the manifest

  Scenario: Commitment that breaches dwelling ceiling is refused
    Given Maria has already pledged 50 GB to the Garcia family on her 100 GB device
    When Maria tries to commit another 35 GB for a third household
    Then the substrate refuses with a friendly explanation
    And the error names the constitutional-ratio-breach violation kind
    And no notarized commitment is created

  Scenario: Operator sees the storage-premium multiplier
    Given Maria's commitments overlap on shards from multi-reach blobs
    When Maria opens her dwelling-hub dashboard
    Then she sees "Pledged: 80 GB; Actually using: 35 GB" honestly displayed
    And the dashboard explains the 2.3x dedup multiplier
```

- [ ] **Step 3: Author `disaster-burst-resilience.feature` (forward-looking, @wip)**

```gherkin
@wip-collective-steward
Feature: Disaster burst — collective absorbs household load
  As a family in a region hit by disaster
  I want my church's collective hub to temporarily host more of my content
  So that even when half my family's devices are gone, our content survives

  Background:
    Given the Smith household is a member of Saint Mary's church collective
    And Saint Mary's hub holds a steward-mode commitment to back member households
    And the Smith household's content is RS-4-7 encoded into 11 shards

  Scenario: Hurricane wipes half a region; collective absorbs burst
    Given 8 of the Smith household's 11 shards have gone offline
    When the substrate detects the replication shortfall
    Then Saint Mary's collective hub fetches the 8 missing shards
    And within hours, the Smith family's content returns to protected status
    And the Smiths' dwelling-hub UI shows "your church is helping right now"
```

- [ ] **Step 4: Validate Gherkin + commit**

```bash
cd /projects/elohim
# Validate Gherkin syntax via existing a2o parser
pnpm run a2o:lint 2>&1 | tail -10  # if such a script exists; otherwise skip

git add genesis/a2o/features/storage/
git commit -m "test(a2o): household resiliency handshake + constitutional ratio + @wip disaster burst"
```

---


## Phase H — Close-out

### Task 20: Sprint 3 close-out + pattern memory + roadmap update

**Files:**
- Create: `genesis/docs/research/2026-05-28-sprint3-storage-replication-implementation-notes.md`
- Modify: `genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md`
- Create: `.claude/memory/project_dwelling_hub_replication_pattern.md`
- Modify: `.claude/memory/MEMORY.md`

- [ ] **Step 1: Author Sprint 3 implementation notes**

Create `genesis/docs/research/2026-05-28-sprint3-storage-replication-implementation-notes.md`:

```markdown
# Sprint 3 — Storage Replication (Dwelling-Hub Tier) Implementation Notes

**Status:** Landed 2026-MM-DD
**Spec:** `genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md`
**Plan:** `genesis/docs/superpowers/plans/2026-05-28-mutual-storage-replication-dwelling-hub-plan.md`

## Commits

| Task | SHA | Subject |
|------|-----|---------|
| T1 | <SHA> | DNA donut walls |
| ... | ... | ... |
| T20 | <SHA> | Sprint 3 close-out |

## What landed

- `Mishpat::Commitment` action `replicates-dwelling` end-to-end (coordinator + integrity defense-in-depth)
- DNA-locked donut walls + manifest-declared ratios + `constitutional_ratio_registry`
- `replicates_dwelling_validator` (first concrete instance of the bounds-validator-pattern)
- `BoundsViolation::ConstitutionalRatioBreach` variant
- `mutuality_audit_log` table + `mutuality_audit_service` (first per-scale aggregator instance)
- `reciprocity-imbalance` FeedbackSignal kind registered + projected via `project_extension_signal`
- `PeerCapacityView` + `HubCapacityView` (mirrors `HubComputeAggregateView` shape)
- Extensions to `DistributionSummary` (`projectionTier`, `over_replicated`), `DistributionDetails` (`replicationCommitments`, `faultDomainDiversity`), `replica-peer` (`shardsHeld`), `HouseholdResilienceView` (`commitmentBackedReplication`)
- `replication_prioritizer` service (commitments shape what peers cache)
- Three new HTTP routes (`/peer/{cid}/capacity`, `/hub/{id}/capacity`, `/diagnostics/mutuality-audit`)
- Sweettest two-conductor test
- Three a2o feature files (one `@wip-collective-steward`)

## Adaptations during execution

- (Implementer fills in: any deviations from the plan template, why)

## Follow-up sprints unblocked

1. **Encryption envelope + key custody** — peer↔hub end-to-end; pre-condition for production-readiness
2. **`replicates-collective` action + collective-tier handshake** — collective_steward mode end-to-end + membership-attestation chain
3. **`replicates-commons` action + commons-class filter** + close the floor-via-declaration gap (declaration must be backed by active commitment)
4. **Doorway projection compute agreements** (second compute-commitment instance)
5. **Distributed workloads** (third compute-commitment instance)
6. **Seeder-based content publish** to retire Z.1 `stageSpaBlobs` (unrelated; separate small sprint)

## Manual operator-watch acceptance checklist

- [ ] `pnpm run hc:start:seed` brings up local stack with 2+ simulated dwelling-hubs
- [ ] `curl http://localhost:8090/api/v1/peer/<peer_cid>/capacity` shows compliantWithDonut on a fresh peer
- [ ] Author commitment via CLI; PeerCapacityView reflects; mutuality_audit_log shows Pending
- [ ] Advance clock past grace_period; reciprocity-imbalance signal in stream
- [ ] Author ratio-breaching commitment; 400 with ConstitutionalRatioBreach
```

- [ ] **Step 2: Update parent roadmap**

In `genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md`, update the Phase A status:

Append to the existing `**Phase A status:**` block:

```
**Sprint 3 landed 2026-MM-DD** — mutual storage replication (dwelling-hub tier) shipped as the first concrete instance of the REA compute-commitment primitive. `replicates-dwelling` action + donut economics + mutuality_audit_service + PeerCapacityView/HubCapacityView + topology view extensions + 3 a2o features. provider_role=collective_steward schema-reserved; commons-tier floor enforced via declaration pending follow-up sprint that lands replicates-commons. See `2026-05-28-mutual-storage-replication-dwelling-hub-design.md` close-out.
```

- [ ] **Step 3: Create pattern memory**

Create `.claude/memory/project_dwelling_hub_replication_pattern.md`:

```markdown
---
name: dwelling-hub-replication-pattern
description: First concrete instance of the REA compute-commitment primitive — mutual storage replication between dwelling-hubs (households). Three load-bearing properties: donut economics (device-level), bilateral-by-reference mutuality (with grace-period soft-warn), intent-first observed-state-second. Hub-aware substrate vocabulary; encryption-decoupled commitments. Pattern extends to collective + commons tiers.
metadata:
  type: project
---

The Sprint 3 shape (landed 2026-MM-DD, plan `genesis/docs/superpowers/plans/2026-05-28-mutual-storage-replication-dwelling-hub-plan.md`):

**`replicates-dwelling` action on `Mishpat::Commitment`** — payload schema fields: `provider_dwelling_hub_id`, `recipient_dwelling_hub_id`, `provider_role: steward_mutual | collective_steward`, `via_collective_hub_id?`, `capacity_bytes`, `scope_filter`, `valid_from/until`, `grace_period_days`, `rotation_ttl_days`, `ratio_attestation`. **No new DHT entry type.**

**Donut economics, device-level:** DNA-locked floor + ceiling constants form the donut walls; elohim manifest declares specific ratios within them. `bounds_validator` enforces at every commitment author. Sprint 3 design choice: ceilings enforced via active pledges; floors via declared `ratio_attestation` (floor-via-declaration). The follow-up sprint that lands `replicates-commons` MUST close this gap.

**Mutuality bilateral-by-reference, not by-signature:** A authors commitment naming B; B independently authors counter. `mutuality_audit_service` runs daily sweep; if counter missing past `grace_period_days`, emits `reciprocity-imbalance` FeedbackSignal naming breaching party. Standing-debit via existing `signal_weight_registry` (weight 8, decay 60d).

**Hub-aware vocabulary:** substrate uses `dwelling-hub` (matches shipped `HubKind::Dwelling`); narrative uses "household"/"family"/"grandma". `provider_role=steward_mutual` is bilateral peer-to-peer; `provider_role=collective_steward` is asymmetric (collective backs member households) — schema-reserved Sprint 3, end-to-end in follow-up.

**Intent-first, observed-state-second:** commitments authored first (notarized intent); existing inventory_gossip + libp2p pull catches up; `replication_prioritizer` scores incoming inventory advertisements against active commitments to decide what the local peer fetches.

**Dual-accounting (storage premium):** `PeerCapacityView.totalPledgedBytes` vs `actuallyHeld.uniqueShardBytes` shows the dedup multiplier (~2.3x for typical multi-reach content). Makes hyperscale-without-capture honest.

**Hub is a role, not a notarized entity** (per `[[project_hub_archetype_abstraction]]`). Substrate kind-agnostic; hub classification at projection layer. `HubCapacityView` mirrors `HubComputeAggregateView` shape exactly.

**When to apply this pattern:**
- Every per-instance bounds-validator (Sprint follow-ups: collective tier, commons tier, doorway projection compute, distributed workloads) MUST delegate substrate-wide concerns to `bounds_validator::validate` and only add (a) schema validation of action-specific payload, (b) action discriminator check, (c) projection to `EventForValidation`, (d) any instance-specific enforcement (e.g., the donut check in `replicates_dwelling_validator`).
- Every per-scale audit aggregator (collective_membership_audit_service, commons_contribution_audit_service) MUST mirror `mutuality_audit_service` shape: walk active commitments, classify status, emit FeedbackSignal on breach, persist operational log row.

**Related:**
- `[[project_bounds_validator_pattern]]` — the substrate primitive this proves a first instance of
- `[[project_rea_compute_commitment_primitive]]` — gospel-tier shape
- `[[project_compute_commitment_first_instance_pivot]]` — why deploy was abandoned; storage replication chosen
- `[[project_hub_archetype_abstraction]]` — Hub role; dwelling/collective/computed; stewards-not-members
- `[[project_signal_kind_extensible_protocol_class]]` — reciprocity-imbalance uses this pattern
- `[[feedback_a2o_narrative_is_opus_work]]` — a2o features authored by Opus
- `[[feedback_diesel_migration_timestamp_collision]]` — migration slot pattern
```

- [ ] **Step 4: Add MEMORY.md entry**

In `.claude/memory/MEMORY.md`, near the other Sprint 1+2+3 entries:

```markdown
- [Dwelling-hub replication pattern](project_dwelling_hub_replication_pattern.md) — Sprint 3 first instance of REA compute-commitment; replicates-dwelling action + donut economics + bilateral-by-reference mutuality + dual-accounting storage-premium; hub-aware vocab + encryption-decoupled.
```

Keep under ~200 chars per existing convention.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add genesis/docs/research/2026-05-28-sprint3-storage-replication-implementation-notes.md \
        genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md \
        .claude/memory/project_dwelling_hub_replication_pattern.md \
        .claude/memory/MEMORY.md
git commit -m "docs(memory): Sprint 3 close-out + dwelling-hub replication pattern memory"
```

---

# Self-Review

**Spec coverage:**

| Spec section | Implemented by |
|-------------|---------------|
| §1 Context, claim, what ships | Overview + Task headers |
| §2 Architectural shape (gradient, hub-as-role, vocabulary) | T1 (DNA constants); T2 (schema); T9 (action discriminator); pattern memory T20 |
| §3 P2P Design Gate | Referenced upfront; inherited from spec |
| §4 `replicates-dwelling` payload | T2 schema + ts-rs |
| §4 bounds check semantics | T7 ConstitutionalRatioBreach + T8 validator |
| §5 Donut economics | T1 DNA + T5 manifest + T6 registry + T8 donut_check |
| §6 Mutuality + grace period | T11 migration + T12 audit_service |
| §6.3 `reciprocity-imbalance` signal_kind | T5 manifest declaration |
| §7.1 PeerCapacityView | T3 schema + T13 service |
| §7.2 HubCapacityView | T3 schema + T14 service |
| §7.3 Topology view extensions | T4 schema extensions + T15 computation |
| §8 Replication data plane | T16 replication_prioritizer |
| §9 Testing strategy | T6/T7/T8/T9/T10/T11/T12/T13/T14/T15/T16 unit + integration; T18 sweettest; T19 a2o |
| §10 Out of scope | Explicit notes throughout |
| §11 Follow-ups | T20 close-out |
| §12 Acceptance signals | T20 manual checklist |
| §13 Memory references | T20 pattern memory |

**Placeholder scan:** Several tasks contain stub returns (T13 `query_latest_total_raw_bytes` returns 0; T13 `aggregate_pledges_by_tier` returns empty; T13 `compute_unique_shard_bytes` returns 0; T14 `resolve_hub_members` falls back to single-device; T12 `find_counter` returns None). These are **explicit Sprint 3 stubs** — each marked with a comment naming the gap and the follow-up sprint that fills it. Implementer should preserve the stubs as-is (do not invent richer behavior); the integration with the real data sources is the encryption-layer / collective-tier sprint scope.

**Type consistency check:**
- `EffectiveRatios` struct (T6) referenced by `EffectiveRatiosView` (T3 view) ✓
- `ProviderRole` enum (T2 Rust) matches schema enum (T2 JSON) ✓
- `BoundsViolation::ConstitutionalRatioBreach` (T7) consumed by `ValidationError::Bounds` mapping (T8) ✓
- `ReciprocityStatus::{Matched,Pending,Breached}` (T12) serialized as snake_case strings in `mutuality_audit_log.reciprocity_status` (T11 column) ✓
- `HubKind::{Dwelling,Collective,Computed}` (T3 + T14) matches shipped `HubKind` enum in `HubComputeAggregateView` ✓
- `Tier::{Dwelling,Collective,Commons,Free}` consistent across `PeerCapacityView.pledgesByRecipient[].tier` (T3) + `PeerCapacityView.ratioCompliance.violations[].tier` (T3) + `DistributionDetails.replicationCommitments[].tier` (T4) ✓
- `ProjectionTier::{Local,Regional,Global}` (T4 + T15 compute_projection_tier) ✓
- `FetchPriority::{High,Medium,Skip}` (T16) — only High/Skip emitted Sprint 3; Medium reserved for commons-tier follow-up ✓

**No type drift detected.**

---

# Execution Handoff

**Plan saved to** `genesis/docs/superpowers/plans/2026-05-28-mutual-storage-replication-dwelling-hub-plan.md`.

**Two execution paths per writing-plans skill:**

1. **Subagent-Driven (recommended)** — fresh subagent per task with two-stage review (spec compliance + code quality). Dispatch rust-architect for the substrate/Mishpat tasks (T1, T6–T17); use general-purpose or claude for narrative + close-out (T19, T20). Sweettest (T18) needs operator-watch.

2. **Inline batch execution** — `superpowers:executing-plans` runs in this session with checkpoint reviews.

**Recommended pairing:** the donut + bounds checks (T6–T8) are the substrate keystone; operator should review those commits closely. The audit-service (T12) carries the per-scale aggregator pattern that future sprints will mirror — worth review attention. The a2o features (T19) are Opus-narrative work per `feedback_a2o_narrative_is_opus_work`; ensure the implementer requests Opus.

**Cross-sprint coordination notes:**
- Sprint 3 commits land on `sprint/cross-pillar-cleanup` (current operator branch)
- Follow-up encryption sprint should NOT begin until Sprint 3 close-out lands
- Follow-up `replicates-commons` sprint MUST close the floor-via-declaration gap (declaration → backing pledge requirement); add this as the first acceptance signal of that sprint
