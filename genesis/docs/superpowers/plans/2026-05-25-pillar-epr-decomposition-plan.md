---
id: pillar-epr-decomposition-plan
status: Draft
cites:
  - ../specs/2026-05-25-pillar-epr-decomposition-design.md   # the design spec this plan implements
---

# Pillar EPR Decomposition Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the substrate (project-epr REA Commitment + validator + seeds) and the delivery (doorway router + elohim-core extract + lamad bundle split) so that `alpha.elohim.host/` serves the landing EPR and `alpha.elohim.host/lamad/...` serves the lamad app as its own bundle.

**Architecture:** Two phases. Phase A adds a new `action="project-epr"` discriminator to the existing REA Commitment infrastructure (no new DHT entry types), plus seed scripts that create the four projection commitments. Phase B teaches the doorway to consult those projections (longest-prefix URL match), extracts shared primitives (Loader, Session, EPR-link, page-chrome, omnibar contract, context-menu) into the elohim-core Lit element library, and splits the lamad pillar into its own deliverable Angular bundle at `app/lamad/` with `<base href="/lamad/">`.

**Tech Stack:** Rust (Holochain DNA + storage HTTP service + doorway), TypeScript (seeder), Lit + TypeScript (elohim-core elements), Angular 19 (lamad + elohim-app), Jenkins pipeline, Cypress + Cucumber (a2o).

**Spec:** `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md`

**Branch:** `design/pillar-epr-decomposition` (branched from dev). Implementation may continue on this branch or fork per phase.

## P2P Design Gate (run during brainstorming; documented in spec Appendix B)

This plan does NOT introduce a new DHT entry type. The classifications were made in the spec:

| Entity | Tier | Substrate location | Why this tier |
|---|---|---|---|
| `project-epr` projection commitment | **A — notarized** | Existing `Commitment` entry in elohim DNA `content_store_integrity` zome | Reuses existing REA Commitment infrastructure with a new `action` discriminator (mirrors the already-landed `operate-doorway` pattern from the 2026-05-19 stewardship-chain design). Content-addressed id; deterministic; idempotent. |
| `ElementRegistryView` | **C — operational projection** | Existing `Content` entry, new `contentFormat: "element-registry-manifest"` | Element registries are projected from content rows; consistency comes from the underlying content row's existing notarization flow. No new entry type. |
| `GET /api/v1/epr/{id}` | Read projection over existing Content | n/a — read-only HTTP route | Lookup by id over already-notarized Content rows. Identity is the existing content row id (slug). No new entity. |
| `EprProjectionView` (Rust struct, JSON schema) | Wire shape — view layer | `elohim/elohim-views/src/projection.rs` + JSON schema | Projection of the Commitment entry's metadata into the HTTP API's camelCase wire shape per the established Rust-to-TypeScript boundary pattern. |

**Source-of-truth declaration for every new file in this plan:**

- `elohim/elohim-views/src/projection.rs` — view-layer wire shape over `Commitment` (existing DHT entry, content_store_integrity zome). Source of truth: Holochain Commitment record.
- `elohim/sdk/schemas/v1/views/epr-projection-view.schema.json` — JSON schema for the same wire shape. Schema-first; the Rust struct + JSON schema co-validate via `tests/schema_contract.rs`.
- `elohim/elohim-views/src/element_registry.rs` + JSON schema — wire shape over Content rows with `contentFormat: "element-registry-manifest"`. Source of truth: Content entry (existing DHT type).
- `doorway/doorway-service/src/projection/epr_router.rs` — operational projection cache over active project-epr Commitments scoped to this doorway. Rebuilt from substrate on boot + SSE events.
- `doorway/doorway-service/src/routes/epr.rs` — read-only HTTP projection over existing Content rows.
- `app/elohim-elements/elohim-core/src/loader/loader.ts` — client-side transport multiplexer. Verifies CID against source-of-truth bytes from any transport.

No new DHT entry types are introduced by this plan. All new artifacts are projections, view shapes, or routing infrastructure layered over existing substrate primitives. The post-write `p2p-design-gate` heuristic audit may flag the new schema/route additions; this section is the authoritative answer to those flags. See spec §11 and Appendix B for the full design-gate analysis.

---

## File Structure

### Phase A creates/modifies

| Path | New? | Responsibility |
|---|---|---|
| `elohim/elohim-views/src/projection.rs` | new | EprProjectionView + GateHintRef + StewardDirectEndpoint structs |
| `elohim/elohim-views/src/element_registry.rs` | new | ElementRegistryView + ElementEntry structs |
| `elohim/elohim-views/src/lib.rs` | modify | Export new modules |
| `elohim/sdk/schemas/v1/views/epr-projection-view.schema.json` | new | JSON schema for EprProjectionView |
| `elohim/sdk/schemas/v1/views/element-registry-view.schema.json` | new | JSON schema for ElementRegistryView |
| `elohim/sdk/schemas/scripts/codegen-ts.mjs` | modify | Add new views to INTERFACE_FILES |
| `elohim/elohim-storage/src/db/rea_commitments.rs` | modify | PROJECT_EPR_ACTION constant + validator + resolvers |
| `elohim/elohim-storage/src/services/events.rs` | modify | ProjectionRegistered + ProjectionRevoked events |
| `elohim/elohim-storage/src/services/rea_commitment_service.rs` | modify | Emit projection events from create/cancel paths |
| `elohim/elohim-storage/tests/schema_contract.rs` | modify | Add EprProjectionView + ElementRegistryView |
| `elohim/sdk/domains/lamad/manifest.json` | modify | Add `element-registry-manifest` content format |
| `genesis/seeder/src/seed-projections.ts` | new | Seed 4 default project-epr commitments |
| `genesis/seeder/src/seed.ts` | modify | Wire seed-projections into main seed flow |
| `genesis/data/elements/elohim-core-registry.json` | new | Seed content for elohim-core-elements registry |
| `genesis/seeder/src/__tests__/seed-projections.test.ts` | new | Unit tests for seed-projections body builder |

### Phase B creates/modifies

| Path | New? | Responsibility |
|---|---|---|
| `doorway/doorway-service/src/projection/epr_router.rs` | new | Path-prefix table + dispatcher |
| `doorway/doorway-service/src/projection/mod.rs` | modify | Export epr_router |
| `doorway/doorway-service/src/projection/storage_events_subscriber.rs` | modify | Handle projection events |
| `doorway/doorway-service/src/routes/epr.rs` | new | GET /api/v1/epr/{id} |
| `doorway/doorway-service/src/routes/mod.rs` | modify | Export epr module |
| `doorway/doorway-service/src/server/http.rs` | modify | Wire epr_router into dispatch |
| `doorway/doorway-service/src/config.rs` | modify | Drop ROOT_APP_SLUG field |
| `doorway/doorway-service/src/main.rs` | modify | Drop ROOT_APP_SLUG usage; load projections at boot |
| `app/elohim-elements/elohim-core/src/loader/loader.ts` | new | Transport-agnostic CID resolution |
| `app/elohim-elements/elohim-core/src/loader/loader.spec.ts` | new | |
| `app/elohim-elements/elohim-core/src/session/session.ts` | new | Reactive session primitive |
| `app/elohim-elements/elohim-core/src/session/session.spec.ts` | new | |
| `app/elohim-elements/elohim-core/src/contracts/omnibar.contract.ts` | new | OmnibarContext interface |
| `app/elohim-elements/elohim-core/src/elohim-epr-link.ts` | new | HyperCard primitive |
| `app/elohim-elements/elohim-core/src/elohim-epr-link.spec.ts` | new | |
| `app/elohim-elements/elohim-core/src/elohim-page-chrome.ts` | new | Slotted omnibar wrapper |
| `app/elohim-elements/elohim-core/src/elohim-page-chrome.spec.ts` | new | |
| `app/elohim-elements/elohim-core/src/elohim-default-omnibar.ts` | new | Default omnibar |
| `app/elohim-elements/elohim-core/src/elohim-skeleton.ts` | new | Sized shimmer placeholder |
| `app/elohim-elements/elohim-core/src/elohim-mention-base.ts` | new | Generic cross-pillar fallback chip |
| `app/elohim-elements/elohim-core/src/elohim-context-menu.ts` | new | Google Drive-style menu (MVP: 3 items) |
| `app/elohim-elements/elohim-core/src/elohim-context-menu.spec.ts` | new | |
| `app/elohim-elements/elohim-core/src/index.ts` | modify | Export new |
| `app/elohim-elements/elohim-core/src/register.ts` | modify | Register new custom elements |
| `app/elohim-library/projects/graphos/src/default/core/__docs__/*.default.stories.ts` | new | Library A stories (6 files, one per new element) |
| `app/elohim-library/projects/graphos/src/designed/core/__docs__/*.designed.stories.ts` | new | Library B stories (6 files) |
| `app/lamad/` | new | New Angular project (own angular.json, package.json, tsconfig) |
| `app/lamad/src/index.html` | new | `<base href="/lamad/">` |
| `app/lamad/src/app/**` | move | Pillar code from app/elohim-app/src/app/lamad/ |
| `app/elohim-app/src/app/lamad/` | delete | Migrated to app/lamad/ |
| `app/elohim-app/src/app/app.routes.ts` | modify | Remove /lamad subtree |
| `app/elohim-app/src/app/elohim/components/epr-link/epr-link.component.ts` | modify | Thin Angular wrapper around `<elohim-epr-link>` |
| `pnpm-workspace.yaml` | modify | Add app/lamad |
| `Jenkinsfile` | modify | Build/upload/patch two bundles |
| `genesis/a2o/features/doorway/native-epr-projection.feature` | new | a2o scenarios |
| `genesis/a2o/steps/doorway/native-epr-projection.steps.ts` | new | |
| `genesis/a2o/features/elohim-core/epr-link-hypercard.feature` | new | a2o scenarios |
| `genesis/a2o/steps/elohim-core/epr-link-hypercard.steps.ts` | new | |

---

# PHASE A — Substrate & Seeds

Phase A produces working software independently: project-epr commitments can be created via the storage HTTP API, validated, queried by doorway_id, seeded idempotently, and observed via SSE events. The doorway doesn't consult them yet — that's Phase B. Phase A is independently committable and CI-testable.

## Task A1: Add PROJECT_EPR_ACTION constant

**Files:**
- Modify: `elohim/elohim-storage/src/db/rea_commitments.rs:350` (next to existing OPERATE_DOORWAY_ACTION)

- [ ] **Step 1: Write the failing test**

In the `#[cfg(test)] mod tests` block at the bottom of `rea_commitments.rs`, add:

```rust
#[test]
fn project_epr_action_constant_is_stable() {
    // The action string is content-addressed into commitment ids;
    // changing it breaks idempotency of every existing seed.
    assert_eq!(PROJECT_EPR_ACTION, "project-epr");
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib project_epr_action_constant_is_stable 2>&1 | tail -20
```

Expected: FAIL with "cannot find value `PROJECT_EPR_ACTION` in this scope".

- [ ] **Step 3: Add the constant**

After the existing `pub const OPERATE_DOORWAY_ACTION: &str = "operate-doorway";` line (~line 350):

```rust
pub const PROJECT_EPR_ACTION: &str = "project-epr";
```

- [ ] **Step 4: Run test to verify it passes**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib project_epr_action_constant_is_stable 2>&1 | tail -10
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/db/rea_commitments.rs
git commit -m "feat(rea): add PROJECT_EPR_ACTION constant for projection commitments"
```

---

## Task A2: Create projection view types (Rust structs)

**Files:**
- Create: `elohim/elohim-views/src/projection.rs`
- Modify: `elohim/elohim-views/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-views/src/projection.rs` with the test stub at the bottom (we'll fill the structs in step 3):

```rust
// elohim/elohim-views/src/projection.rs

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// (structs go here in step 3)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_view_serializes_to_camel_case() {
        let view = EprProjectionView {
            commitment_id: "abc".into(),
            epr_id: "lamad-spa".into(),
            doorway_id: "doorway:alpha-elohim-host".into(),
            url_path: "/lamad".into(),
            mode: ProjectionMode::Cached,
            reach: "commons".into(),
            base_href: "/lamad/".into(),
            entry_file: "index.html".into(),
            redirects_from: vec![],
            preview_epr_ref: None,
            gate_hints: vec![],
            dead_end: false,
            steward_direct_endpoint: None,
            seeded_at: "2026-05-25T00:00:00Z".into(),
            seeded_by: "12D3Koo...".into(),
        };
        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains("\"commitmentId\":\"abc\""));
        assert!(json.contains("\"eprId\":\"lamad-spa\""));
        assert!(json.contains("\"urlPath\":\"/lamad\""));
        assert!(json.contains("\"mode\":\"cached\""));
        assert!(json.contains("\"baseHref\":\"/lamad/\""));
    }

    #[test]
    fn projection_mode_steward_direct_serializes_correctly() {
        let view = ProjectionMode::StewardDirect;
        let json = serde_json::to_string(&view).unwrap();
        assert_eq!(json, "\"stewardDirect\"");
    }

    #[test]
    fn gate_hint_relation_all_variants_serialize() {
        use GateHintRelation::*;
        for (variant, expected) in [
            (PersonWhoCanGrant, "\"personWhoCanGrant\""),
            (MembershipPrerequisite, "\"membershipPrerequisite\""),
            (ContentToSync, "\"contentToSync\""),
            (PlaceToVisit, "\"placeToVisit\""),
            (CapabilityToEarn, "\"capabilityToEarn\""),
            (PaymentToOffer, "\"paymentToOffer\""),
            (WitnessToInvolve, "\"witnessToInvolve\""),
        ] {
            assert_eq!(serde_json::to_string(&variant).unwrap(), expected);
        }
    }
}
```

- [ ] **Step 2: Wire the module into lib.rs and run test to see expected compilation failure**

Edit `elohim/elohim-views/src/lib.rs` — find the existing `pub mod` declarations and add:

```rust
pub mod projection;
```

Then:

```bash
cd elohim/elohim-views
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test projection 2>&1 | tail -20
```

Expected: COMPILE FAILURE — structs not defined.

- [ ] **Step 3: Implement the structs**

In `elohim/elohim-views/src/projection.rs`, replace the comment with:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EprProjectionView {
    pub commitment_id: String,
    pub epr_id: String,
    pub doorway_id: String,
    pub url_path: String,
    pub mode: ProjectionMode,
    pub reach: String,
    pub base_href: String,
    pub entry_file: String,
    pub redirects_from: Vec<String>,
    pub preview_epr_ref: Option<String>,
    pub gate_hints: Vec<GateHintRef>,
    pub dead_end: bool,
    pub steward_direct_endpoint: Option<StewardDirectEndpoint>,
    pub seeded_at: String,
    pub seeded_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum ProjectionMode {
    Cached,
    StewardDirect,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct GateHintRef {
    pub epr_ref: String,
    pub label: Option<String>,
    pub relation: GateHintRelation,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum GateHintRelation {
    PersonWhoCanGrant,
    MembershipPrerequisite,
    ContentToSync,
    PlaceToVisit,
    CapabilityToEarn,
    PaymentToOffer,
    WitnessToInvolve,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StewardDirectEndpoint {
    pub peer_id: String,
    pub alt_host: Option<String>,
    pub tls_cert_san: String,
    pub accepts_projection_for: Vec<String>,
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test projection 2>&1 | tail -10
```

Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-views/src/projection.rs elohim/elohim-views/src/lib.rs
git commit -m "feat(views): add EprProjectionView + GateHintRef + StewardDirectEndpoint"
```

---

## Task A3: Generate TypeScript types from new Rust views

**Files:**
- Generated: `elohim/sdk/storage-client-ts/src/generated/EprProjectionView.ts`
- Generated: `elohim/sdk/storage-client-ts/src/generated/ProjectionMode.ts`
- Generated: `elohim/sdk/storage-client-ts/src/generated/GateHintRef.ts`
- Generated: `elohim/sdk/storage-client-ts/src/generated/GateHintRelation.ts`
- Generated: `elohim/sdk/storage-client-ts/src/generated/StewardDirectEndpoint.ts`

- [ ] **Step 1: Run the ts-rs codegen tests**

```bash
cd elohim/elohim-views
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings 2>&1 | tail -10
```

Expected: Tests pass and write `.ts` files to `elohim/sdk/storage-client-ts/src/generated/`.

- [ ] **Step 2: Verify generated files exist**

```bash
ls elohim/sdk/storage-client-ts/src/generated/{EprProjectionView,ProjectionMode,GateHintRef,GateHintRelation,StewardDirectEndpoint}.ts
```

Expected: All 5 files present.

- [ ] **Step 3: Verify generated content has camelCase**

```bash
grep -E "commitmentId|eprId|urlPath" elohim/sdk/storage-client-ts/src/generated/EprProjectionView.ts
```

Expected: All three field names appear with camelCase.

- [ ] **Step 4: Commit the generated files**

```bash
git add elohim/sdk/storage-client-ts/src/generated/
git commit -m "chore(codegen): regenerate TS types from EprProjectionView"
```

---

## Task A4: Add EprProjectionView JSON schema

**Files:**
- Create: `elohim/sdk/schemas/v1/views/epr-projection-view.schema.json`

- [ ] **Step 1: Look at the existing operator-binding schema as the canonical pattern**

```bash
cat elohim/sdk/schemas/v1/views/doorway-operator-binding-view.schema.json
```

This is the structural model to follow (same JSON Schema draft, same `$id` shape, same field-style).

- [ ] **Step 2: Create the schema file**

Create `elohim/sdk/schemas/v1/views/epr-projection-view.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://elohim.host/schemas/v1/views/epr-projection-view.schema.json",
  "title": "EprProjectionView",
  "description": "Notarized contract: this doorway projects this EPR at this URL path under these terms. Created via REA Commitment with action='project-epr'.",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "commitmentId", "eprId", "doorwayId", "urlPath", "mode", "reach",
    "baseHref", "entryFile", "redirectsFrom", "gateHints", "deadEnd",
    "seededAt", "seededBy"
  ],
  "properties": {
    "commitmentId": { "type": "string", "description": "sha256(provider_peer_id|action|scope)" },
    "eprId":        { "type": "string", "description": "Content row id of the projected EPR" },
    "doorwayId":    { "type": "string", "pattern": "^doorway:" },
    "urlPath":      { "type": "string", "pattern": "^/" },
    "mode":         { "type": "string", "enum": ["cached", "stewardDirect"] },
    "reach":        { "type": "string", "description": "commons | qahal:xxx | household:xxx | etc." },
    "baseHref":     { "type": "string" },
    "entryFile":    { "type": "string", "default": "index.html" },
    "redirectsFrom": { "type": "array", "items": { "type": "string" } },
    "previewEprRef": { "type": ["string", "null"] },
    "gateHints":     { "type": "array", "items": { "$ref": "#/$defs/gateHintRef" } },
    "deadEnd":       { "type": "boolean" },
    "stewardDirectEndpoint": {
      "oneOf": [{ "type": "null" }, { "$ref": "#/$defs/stewardDirectEndpoint" }]
    },
    "seededAt":     { "type": "string", "format": "date-time" },
    "seededBy":     { "type": "string", "description": "Steward peer_id" }
  },
  "$defs": {
    "gateHintRef": {
      "type": "object",
      "additionalProperties": false,
      "required": ["eprRef", "relation"],
      "properties": {
        "eprRef":   { "type": "string" },
        "label":    { "type": ["string", "null"] },
        "relation": { "type": "string", "enum": [
          "personWhoCanGrant", "membershipPrerequisite", "contentToSync",
          "placeToVisit", "capabilityToEarn", "paymentToOffer", "witnessToInvolve"
        ]}
      }
    },
    "stewardDirectEndpoint": {
      "type": "object",
      "additionalProperties": false,
      "required": ["peerId", "tlsCertSan", "acceptsProjectionFor"],
      "properties": {
        "peerId":              { "type": "string" },
        "altHost":             { "type": ["string", "null"] },
        "tlsCertSan":          { "type": "string" },
        "acceptsProjectionFor": { "type": "array", "items": { "type": "string" } }
      }
    }
  }
}
```

- [ ] **Step 3: Validate the schema is syntactically correct**

```bash
pnpm run schema:test 2>&1 | tail -10
```

Expected: Schema test pass (it walks all .schema.json files).

- [ ] **Step 4: Commit**

```bash
git add elohim/sdk/schemas/v1/views/epr-projection-view.schema.json
git commit -m "feat(schemas): add EprProjectionView JSON schema"
```

---

## Task A5: Add schema contract test for EprProjectionView

**Files:**
- Modify: `elohim/elohim-storage/tests/schema_contract.rs`

- [ ] **Step 1: Find the existing schema contract pattern**

```bash
grep -n "DoorwayOperatorBindingView\|doorway-operator-binding" elohim/elohim-storage/tests/schema_contract.rs
```

Expected: Find the existing test that validates DoorwayOperatorBindingView against its schema.

- [ ] **Step 2: Add a parallel test for EprProjectionView**

In `elohim/elohim-storage/tests/schema_contract.rs`, after the existing DoorwayOperatorBindingView contract test, add:

```rust
#[test]
fn epr_projection_view_matches_schema() {
    use elohim_views::projection::*;

    let view = EprProjectionView {
        commitment_id: "test-commitment".into(),
        epr_id: "lamad-spa".into(),
        doorway_id: "doorway:alpha-elohim-host".into(),
        url_path: "/lamad".into(),
        mode: ProjectionMode::Cached,
        reach: "commons".into(),
        base_href: "/lamad/".into(),
        entry_file: "index.html".into(),
        redirects_from: vec![],
        preview_epr_ref: None,
        gate_hints: vec![],
        dead_end: false,
        steward_direct_endpoint: None,
        seeded_at: "2026-05-25T00:00:00Z".into(),
        seeded_by: "12D3Koo...".into(),
    };

    let json = serde_json::to_value(&view).unwrap();
    let schema_path = "../sdk/schemas/v1/views/epr-projection-view.schema.json";
    let schema_text = std::fs::read_to_string(schema_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", schema_path, e));
    let schema: serde_json::Value = serde_json::from_str(&schema_text).unwrap();
    let validator = jsonschema::JSONSchema::compile(&schema).unwrap();
    if let Err(errors) = validator.validate(&json) {
        for e in errors {
            eprintln!("Schema validation error: {}", e);
        }
        panic!("EprProjectionView does not match its JSON schema");
    }
}
```

- [ ] **Step 3: Run the test**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract epr_projection_view 2>&1 | tail -15
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/tests/schema_contract.rs
git commit -m "test(schema): contract test EprProjectionView against JSON schema"
```

---

## Task A6: Implement the substrate validator

**Files:**
- Modify: `elohim/elohim-storage/src/db/rea_commitments.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)] mod tests` block in `rea_commitments.rs`:

```rust
#[test]
fn validator_accepts_commons_reach_without_preview_or_hints() {
    let input = make_project_epr_input_for_test(/* reach */ "commons", None, vec![], false, None);
    assert!(validate_project_epr_commitment(&input).is_ok());
}

#[test]
fn validator_rejects_non_commons_reach_with_nothing_set() {
    let input = make_project_epr_input_for_test("qahal:aleph-members", None, vec![], false, None);
    let err = validate_project_epr_commitment(&input).expect_err("should reject");
    assert!(err.to_string().contains("must declare at least one of"));
}

#[test]
fn validator_accepts_non_commons_with_dead_end() {
    let input = make_project_epr_input_for_test("qahal:xyz", None, vec![], true, None);
    assert!(validate_project_epr_commitment(&input).is_ok());
}

#[test]
fn validator_accepts_non_commons_with_preview() {
    let input = make_project_epr_input_for_test("qahal:xyz", Some("epr:preview-xyz".into()), vec![], false, None);
    assert!(validate_project_epr_commitment(&input).is_ok());
}

#[test]
fn validator_accepts_non_commons_with_hints() {
    let hint = GateHintRef {
        epr_ref: "epr:susan".into(),
        label: Some("Talk to Susan".into()),
        relation: GateHintRelation::PersonWhoCanGrant,
    };
    let input = make_project_epr_input_for_test("qahal:xyz", None, vec![hint], false, None);
    assert!(validate_project_epr_commitment(&input).is_ok());
}

#[test]
fn validator_rejects_steward_direct_without_endpoint() {
    let mut input = make_project_epr_input_for_test("commons", None, vec![], false, None);
    input.mode = ProjectionMode::StewardDirect;
    let err = validate_project_epr_commitment(&input).expect_err("should reject");
    assert!(err.to_string().contains("steward-direct mode requires"));
}

#[test]
fn validator_rejects_url_path_without_leading_slash() {
    let mut input = make_project_epr_input_for_test("commons", None, vec![], false, None);
    input.url_path = "lamad".into();
    let err = validate_project_epr_commitment(&input).expect_err("should reject");
    assert!(err.to_string().contains("urlPath must start with"));
}

// Test helper — replicates the wire input shape for validator tests
fn make_project_epr_input_for_test(
    reach: &str,
    preview: Option<String>,
    hints: Vec<GateHintRef>,
    dead_end: bool,
    endpoint: Option<StewardDirectEndpoint>,
) -> ProjectEprValidationInput {
    ProjectEprValidationInput {
        url_path: "/test".into(),
        mode: ProjectionMode::Cached,
        reach: reach.into(),
        preview_epr_ref: preview,
        gate_hints: hints,
        dead_end,
        steward_direct_endpoint: endpoint,
    }
}
```

- [ ] **Step 2: Run to verify failure**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib validator 2>&1 | tail -15
```

Expected: COMPILE FAILURE — `validate_project_epr_commitment` and `ProjectEprValidationInput` not defined.

- [ ] **Step 3: Implement the validator**

In `rea_commitments.rs`, after the PROJECT_EPR_ACTION constant, add:

```rust
use elohim_views::projection::{
    EprProjectionView, GateHintRef, ProjectionMode, StewardDirectEndpoint,
};

/// Subset of EprProjectionView fields that the validator needs.
/// Allows validation of incoming requests before constructing the full view.
#[derive(Debug, Clone)]
pub struct ProjectEprValidationInput {
    pub url_path: String,
    pub mode: ProjectionMode,
    pub reach: String,
    pub preview_epr_ref: Option<String>,
    pub gate_hints: Vec<GateHintRef>,
    pub dead_end: bool,
    pub steward_direct_endpoint: Option<StewardDirectEndpoint>,
}

/// Validate a project-epr commitment per the spec rules (§2.4).
pub fn validate_project_epr_commitment(
    input: &ProjectEprValidationInput,
) -> Result<(), StorageError> {
    // Rule 1: non-commons reach requires a path forward
    if input.reach != "commons"
        && input.preview_epr_ref.is_none()
        && input.gate_hints.is_empty()
        && !input.dead_end
    {
        return Err(StorageError::Validation(
            "Gated projection must declare at least one of: \
             previewEprRef, gateHints (non-empty), or deadEnd=true".into()
        ));
    }

    // Rule 2: steward-direct mode requires endpoint
    if input.mode == ProjectionMode::StewardDirect
        && input.steward_direct_endpoint.is_none()
    {
        return Err(StorageError::Validation(
            "steward-direct mode requires stewardDirectEndpoint".into()
        ));
    }

    // Rule 3: url_path must start with /
    if !input.url_path.starts_with('/') {
        return Err(StorageError::Validation(
            format!("urlPath must start with '/', got: {}", input.url_path)
        ));
    }

    // Rule 4: url_path cannot have trailing slash unless it IS "/"
    if input.url_path.len() > 1 && input.url_path.ends_with('/') {
        return Err(StorageError::Validation(
            format!("urlPath must not have trailing slash (except '/'): {}", input.url_path)
        ));
    }

    Ok(())
}
```

If `StorageError::Validation` doesn't exist yet, add the variant in `elohim/elohim-storage/src/error.rs`:

```rust
// Add to the StorageError enum
#[error("Validation error: {0}")]
Validation(String),
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib validator 2>&1 | tail -10
```

Expected: All 7 validator tests pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/db/rea_commitments.rs elohim/elohim-storage/src/error.rs
git commit -m "feat(rea): substrate validator for project-epr commitments (4 rules)"
```

---

## Task A7: Implement projection resolvers (find_active_projections, find_projection_by_url_path)

**Files:**
- Modify: `elohim/elohim-storage/src/db/rea_commitments.rs`

- [ ] **Step 1: Write the failing tests**

Add to the test module:

```rust
#[test]
fn find_active_projections_filters_by_doorway_id() {
    let conn = &mut test_db();
    let ctx = AppContext::default_lamad();
    seed_test_projections(conn, &ctx);

    let alpha = find_active_projections(conn, &ctx, "doorway:alpha-elohim-host").unwrap();
    assert_eq!(alpha.len(), 2);
    assert!(alpha.iter().all(|p| p.doorway_id == "doorway:alpha-elohim-host"));

    let beta = find_active_projections(conn, &ctx, "doorway:elohim-host").unwrap();
    assert_eq!(beta.len(), 2);
}

#[test]
fn find_projection_by_url_path_longest_prefix_wins() {
    let conn = &mut test_db();
    let ctx = AppContext::default_lamad();
    seed_test_projections(conn, &ctx);

    let landing = find_projection_by_url_path(conn, &ctx, "doorway:alpha-elohim-host", "/").unwrap();
    assert_eq!(landing.unwrap().epr_id, "elohim-host-landing");

    let lamad = find_projection_by_url_path(
        conn, &ctx, "doorway:alpha-elohim-host", "/lamad/concept/foo"
    ).unwrap();
    assert_eq!(lamad.unwrap().epr_id, "lamad-spa");

    let lamad_root = find_projection_by_url_path(
        conn, &ctx, "doorway:alpha-elohim-host", "/lamad"
    ).unwrap();
    assert_eq!(lamad_root.unwrap().epr_id, "lamad-spa");
}

#[test]
fn find_projection_by_url_path_returns_none_when_no_match() {
    let conn = &mut test_db();
    let ctx = AppContext::default_lamad();
    seed_test_projections(conn, &ctx);

    let nothing = find_projection_by_url_path(
        conn, &ctx, "doorway:unknown", "/anything"
    ).unwrap();
    assert!(nothing.is_none());
}

// Test helper — seeds 4 test projections (2 EPRs × 2 doorways)
fn seed_test_projections(conn: &mut SqliteConnection, ctx: &AppContext) {
    let test_seed = |epr: &str, doorway: &str, url_path: &str| {
        create_rea_commitment(conn, ctx, CreateReaCommitmentInput {
            id: Some(format!("{}-{}-{}", epr, doorway, url_path).replace("/", "_")),
            action: PROJECT_EPR_ACTION.into(),
            provider: "test-steward".into(),
            receiver: "test-operator".into(),
            in_scope_of: Some(format!("doorway:{}|epr:{}", doorway, epr)),
            note: Some(epr.into()),
            metadata_json: Some(serde_json::json!({
                "urlPath": url_path,
                "mode": "cached",
                "reach": "commons",
                "baseHref": format!("{}/", if url_path == "/" { "" } else { url_path }),
                "entryFile": "index.html",
                "redirectsFrom": [],
                "gateHints": [],
                "deadEnd": false
            }).to_string()),
            ..Default::default()
        }).unwrap();
    };
    test_seed("elohim-host-landing", "alpha-elohim-host", "/");
    test_seed("elohim-host-landing", "elohim-host", "/");
    test_seed("lamad-spa", "alpha-elohim-host", "/lamad");
    test_seed("lamad-spa", "elohim-host", "/lamad");
}
```

- [ ] **Step 2: Run to see failure**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib find_ 2>&1 | tail -15
```

Expected: COMPILE FAILURE — `find_active_projections` and `find_projection_by_url_path` not defined.

- [ ] **Step 3: Implement the resolvers**

In `rea_commitments.rs`, after the validator and using `OPERATE_DOORWAY` resolvers as the pattern:

```rust
/// Find all active project-epr commitments scoped to a specific doorway.
///
/// Active = not cancelled. Returns the full view shape ready for the
/// doorway's epr_router to consume.
pub fn find_active_projections(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    doorway_id: &str,
) -> Result<Vec<EprProjectionView>, StorageError> {
    let scope_filter = format!("%doorway:{}|%", doorway_id);

    let commitments: Vec<ReaCommitment> = rea_commitments::table
        .filter(rea_commitments::action.eq(PROJECT_EPR_ACTION))
        .filter(rea_commitments::in_scope_of.like(&scope_filter))
        .filter(rea_commitments::cancelled_at.is_null())
        .filter(rea_commitments::tenant_id.eq(&ctx.tenant_id))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("DB error: {}", e)))?;

    commitments.into_iter().map(commitment_to_projection_view).collect()
}

/// Find the project-epr commitment whose urlPath is the longest prefix
/// of the requested path on the given doorway.
pub fn find_projection_by_url_path(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    doorway_id: &str,
    request_path: &str,
) -> Result<Option<EprProjectionView>, StorageError> {
    let all = find_active_projections(conn, ctx, doorway_id)?;

    // Longest prefix wins. "/lamad" matches "/lamad" and "/lamad/foo"
    // but NOT "/lamadx". "/" matches everything as the universal root.
    let best = all.into_iter()
        .filter(|p| path_matches_prefix(request_path, &p.url_path))
        .max_by_key(|p| p.url_path.len());

    Ok(best)
}

fn path_matches_prefix(request_path: &str, projection_path: &str) -> bool {
    if projection_path == "/" {
        return true;
    }
    request_path == projection_path
        || request_path.starts_with(&format!("{}/", projection_path))
}

/// Convert a stored REA Commitment row into an EprProjectionView.
fn commitment_to_projection_view(c: ReaCommitment) -> Result<EprProjectionView, StorageError> {
    let metadata: serde_json::Value = c.metadata_json.as_deref()
        .map(|s| serde_json::from_str(s))
        .transpose()
        .map_err(|e| StorageError::Internal(format!("metadata parse: {}", e)))?
        .unwrap_or(serde_json::Value::Null);

    let scope = c.in_scope_of.unwrap_or_default();
    let (doorway_id, epr_id) = parse_projection_scope(&scope)?;

    Ok(EprProjectionView {
        commitment_id: c.id,
        epr_id,
        doorway_id,
        url_path: metadata.get("urlPath").and_then(|v| v.as_str()).unwrap_or("/").to_string(),
        mode: serde_json::from_value(metadata.get("mode").cloned().unwrap_or(serde_json::json!("cached")))
            .unwrap_or(ProjectionMode::Cached),
        reach: metadata.get("reach").and_then(|v| v.as_str()).unwrap_or("commons").to_string(),
        base_href: metadata.get("baseHref").and_then(|v| v.as_str()).unwrap_or("/").to_string(),
        entry_file: metadata.get("entryFile").and_then(|v| v.as_str()).unwrap_or("index.html").to_string(),
        redirects_from: metadata.get("redirectsFrom")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
        preview_epr_ref: metadata.get("previewEprRef").and_then(|v| v.as_str()).map(String::from),
        gate_hints: metadata.get("gateHints")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
        dead_end: metadata.get("deadEnd").and_then(|v| v.as_bool()).unwrap_or(false),
        steward_direct_endpoint: metadata.get("stewardDirectEndpoint")
            .and_then(|v| if v.is_null() { None } else { serde_json::from_value(v.clone()).ok() }),
        seeded_at: c.created_at.to_string(),
        seeded_by: c.provider,
    })
}

fn parse_projection_scope(scope: &str) -> Result<(String, String), StorageError> {
    // Scope format: "doorway:alpha-elohim-host|epr:lamad-spa"
    let parts: Vec<&str> = scope.split('|').collect();
    if parts.len() != 2 {
        return Err(StorageError::Internal(format!("Malformed projection scope: {}", scope)));
    }
    let doorway_id = parts[0].strip_prefix("doorway:")
        .ok_or_else(|| StorageError::Internal(format!("Scope missing 'doorway:' prefix: {}", scope)))?
        .to_string();
    let epr_id = parts[1].strip_prefix("epr:")
        .ok_or_else(|| StorageError::Internal(format!("Scope missing 'epr:' prefix: {}", scope)))?
        .to_string();
    Ok((format!("doorway:{}", doorway_id), epr_id))
}
```

- [ ] **Step 4: Add a unit test for path_matches_prefix**

```rust
#[test]
fn path_matches_prefix_handles_root_and_segments() {
    assert!(path_matches_prefix("/", "/"));
    assert!(path_matches_prefix("/anything", "/"));
    assert!(path_matches_prefix("/lamad", "/lamad"));
    assert!(path_matches_prefix("/lamad/", "/lamad"));
    assert!(path_matches_prefix("/lamad/concept/x", "/lamad"));
    assert!(!path_matches_prefix("/lamadx", "/lamad"));
    assert!(!path_matches_prefix("/lam", "/lamad"));
    assert!(!path_matches_prefix("/other", "/lamad"));
}
```

- [ ] **Step 5: Run tests**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib 2>&1 | grep -E "find_|path_matches_prefix" | tail -10
```

Expected: All 4 find_* tests + the path_matches_prefix test pass.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/db/rea_commitments.rs
git commit -m "feat(rea): projection resolvers (find_active_projections + longest-prefix match)"
```

---

## Task A8: Add ProjectionRegistered + ProjectionRevoked events

**Files:**
- Modify: `elohim/elohim-storage/src/services/events.rs`
- Modify: `elohim/elohim-storage/src/services/rea_commitment_service.rs` (or wherever Commitment create/cancel lives)

- [ ] **Step 1: Locate the Commitment create/cancel handlers**

```bash
grep -rn "PROJECT_EPR_ACTION\|OPERATE_DOORWAY_ACTION\|create_rea_commitment\|cancel_rea_commitment" elohim/elohim-storage/src/ 2>/dev/null | head -10
```

Expected: Find the function that creates/cancels commitments.

- [ ] **Step 2: Add the new event variants**

In `elohim/elohim-storage/src/services/events.rs`, extend the StorageEvent enum:

```rust
// Add these variants to StorageEvent
ProjectionRegistered {
    commitment_id: String,
},
ProjectionRevoked {
    commitment_id: String,
},
```

Update the kind/data formatters:

```rust
// In event_kind(), add:
StorageEvent::ProjectionRegistered { .. } => "projection.registered",
StorageEvent::ProjectionRevoked { .. } => "projection.revoked",

// In event_data(), add:
StorageEvent::ProjectionRegistered { commitment_id } |
StorageEvent::ProjectionRevoked { commitment_id } => {
    serde_json::json!({ "commitmentId": commitment_id }).to_string()
}
```

- [ ] **Step 3: Write the failing test**

In `events.rs` tests:

```rust
#[test]
fn projection_registered_event_formats_correctly() {
    let evt = StorageEvent::ProjectionRegistered { commitment_id: "test-cid".into() };
    assert_eq!(event_kind(&evt), "projection.registered");
    assert!(event_data(&evt).contains("\"commitmentId\":\"test-cid\""));
}

#[test]
fn projection_revoked_event_formats_correctly() {
    let evt = StorageEvent::ProjectionRevoked { commitment_id: "test-cid".into() };
    assert_eq!(event_kind(&evt), "projection.revoked");
}
```

- [ ] **Step 4: Run test**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib projection_ 2>&1 | tail -10
```

Expected: Both pass.

- [ ] **Step 5: Wire emission from the Commitment create/cancel paths**

In the service file that calls `create_rea_commitment` (likely `rea_commitment_service.rs`), after a successful create:

```rust
if commitment.action == PROJECT_EPR_ACTION {
    self.events.emit(StorageEvent::ProjectionRegistered {
        commitment_id: commitment.id.clone(),
    });
}
```

And in the cancel path:

```rust
if cancelled_commitment.action == PROJECT_EPR_ACTION {
    self.events.emit(StorageEvent::ProjectionRevoked {
        commitment_id: cancelled_commitment.id.clone(),
    });
}
```

- [ ] **Step 6: Add integration test for emission**

In the service's tests:

```rust
#[test]
fn create_project_epr_commitment_emits_projection_registered_event() {
    let bus = Arc::new(EventBus::new());
    let mut rx = bus.subscribe();
    let svc = ReaCommitmentService::new_with_events(bus.clone());
    // ... create a project-epr commitment via the service ...
    let event = rx.try_recv().expect("expected event");
    matches!(event, StorageEvent::ProjectionRegistered { .. });
}
```

(adapt the constructor to your service's actual signature)

- [ ] **Step 7: Run all events tests**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib events 2>&1 | tail -10
```

Expected: All pass.

- [ ] **Step 8: Commit**

```bash
git add elohim/elohim-storage/src/services/
git commit -m "feat(events): emit projection.registered/revoked on project-epr commitment lifecycle"
```

---

## Task A9: Add ElementRegistryView types + schema

**Files:**
- Create: `elohim/elohim-views/src/element_registry.rs`
- Create: `elohim/sdk/schemas/v1/views/element-registry-view.schema.json`
- Modify: `elohim/elohim-views/src/lib.rs`

- [ ] **Step 1: Create the Rust types**

Create `elohim/elohim-views/src/element_registry.rs`:

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ElementRegistryView {
    pub epr_id: String,
    pub pillar: String,
    pub elements: Vec<ElementEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ElementEntry {
    pub tag_name: String,
    pub cid: String,
    pub version: String,
    pub view_deps: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_registry_view_serializes_to_camel_case() {
        let view = ElementRegistryView {
            epr_id: "elohim-core-elements".into(),
            pillar: "elohim-core".into(),
            elements: vec![ElementEntry {
                tag_name: "elohim-button".into(),
                cid: "sha256-abc".into(),
                version: "1.0.0".into(),
                view_deps: vec![],
            }],
        };
        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains("\"eprId\":\"elohim-core-elements\""));
        assert!(json.contains("\"tagName\":\"elohim-button\""));
    }
}
```

Add to `lib.rs`:

```rust
pub mod element_registry;
```

- [ ] **Step 2: Create the JSON schema**

Create `elohim/sdk/schemas/v1/views/element-registry-view.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://elohim.host/schemas/v1/views/element-registry-view.schema.json",
  "title": "ElementRegistryView",
  "description": "Manifest of custom elements a pillar exposes for cross-pillar embedding.",
  "type": "object",
  "additionalProperties": false,
  "required": ["eprId", "pillar", "elements"],
  "properties": {
    "eprId":   { "type": "string" },
    "pillar":  { "type": "string" },
    "elements": {
      "type": "array",
      "items": { "$ref": "#/$defs/elementEntry" }
    }
  },
  "$defs": {
    "elementEntry": {
      "type": "object",
      "additionalProperties": false,
      "required": ["tagName", "cid", "version", "viewDeps"],
      "properties": {
        "tagName":  { "type": "string", "pattern": "^[a-z][a-z0-9]*-[a-z0-9-]+$" },
        "cid":      { "type": "string", "pattern": "^sha256-" },
        "version":  { "type": "string", "pattern": "^\\d+\\.\\d+\\.\\d+" },
        "viewDeps": { "type": "array", "items": { "type": "string" } }
      }
    }
  }
}
```

- [ ] **Step 3: Run tests + codegen**

```bash
cd elohim/elohim-views
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test element_registry export_bindings 2>&1 | tail -10
ls elohim/sdk/storage-client-ts/src/generated/ElementRegistryView.ts
pnpm run schema:test 2>&1 | tail -5
```

Expected: tests pass, generated file present, schema test passes.

- [ ] **Step 4: Add contract test**

In `elohim/elohim-storage/tests/schema_contract.rs`:

```rust
#[test]
fn element_registry_view_matches_schema() {
    use elohim_views::element_registry::*;
    let view = ElementRegistryView {
        epr_id: "elohim-core-elements".into(),
        pillar: "elohim-core".into(),
        elements: vec![ElementEntry {
            tag_name: "elohim-button".into(),
            cid: "sha256-0000000000000000000000000000000000000000000000000000000000000000".into(),
            version: "1.0.0".into(),
            view_deps: vec![],
        }],
    };
    // ... reuse the same schema-validate pattern as Task A5 ...
}
```

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-views/src/element_registry.rs elohim/elohim-views/src/lib.rs \
        elohim/sdk/schemas/v1/views/element-registry-view.schema.json \
        elohim/elohim-storage/tests/schema_contract.rs \
        elohim/sdk/storage-client-ts/src/generated/ElementRegistryView.ts \
        elohim/sdk/storage-client-ts/src/generated/ElementEntry.ts
git commit -m "feat(views): ElementRegistryView for pillar element manifests"
```

---

## Task A10: Add element-registry-manifest content format to lamad manifest

**Files:**
- Modify: `elohim/sdk/domains/lamad/manifest.json`

- [ ] **Step 1: Find the content_formats section**

```bash
grep -n "content_formats\|contentFormats" elohim/sdk/domains/lamad/manifest.json | head -5
```

- [ ] **Step 2: Add the new format**

In the `content_formats` (or `contentFormats`) object, add:

```json
"element-registry-manifest": {
  "renderer": "elohim-element-registry",
  "description": "A manifest of custom elements a pillar exposes (ElementRegistryView wire shape)."
}
```

(Adapt the exact field names to whatever the existing entries use.)

- [ ] **Step 3: Run manifest codegen**

```bash
pnpm run lamad:codegen 2>&1 | tail -10
```

Expected: regenerates `manifest-types.ts`; the new format appears.

- [ ] **Step 4: Verify**

```bash
grep "element-registry-manifest" app/elohim-library/projects/elohim-service/src/generated/manifest-types.ts
```

Expected: Present.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/domains/lamad/manifest.json app/elohim-library/projects/elohim-service/src/generated/manifest-types.ts
git commit -m "feat(manifest): add element-registry-manifest content format"
```

---

## Task A11: Create seed-projections.ts

**Files:**
- Create: `genesis/seeder/src/seed-projections.ts`

- [ ] **Step 1: Read the operate-doorway seeder as the template**

```bash
cat genesis/seeder/src/seed-operator-bindings.ts | head -120
```

This is the canonical structure: builder fn → client wrapper → seed runner → CLI entry.

- [ ] **Step 2: Write the body builder + happy-path test first**

Create `genesis/seeder/src/__tests__/seed-projections.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { buildProjectionCommitmentBody, type ProjectionSpec } from '../seed-projections.js';

describe('buildProjectionCommitmentBody', () => {
  it('builds a commons-reach lamad projection at /lamad', () => {
    const spec: ProjectionSpec = {
      stewardHumanId: 'human-matthew-manager',
      stewardArchetype: 'manager',
      doorwayId: 'alpha-elohim-host',
      eprId: 'lamad-spa',
      urlPath: '/lamad',
      mode: 'cached',
      reach: 'commons',
      baseHref: '/lamad/',
      entryFile: 'index.html',
      redirectsFrom: [],
      previewEprRef: null,
      gateHints: [],
      deadEnd: false,
      stewardDirectEndpoint: null,
    };
    const body = buildProjectionCommitmentBody(spec);
    expect(body.action).toBe('project-epr');
    expect(body.inScopeOf).toContain('doorway:alpha-elohim-host');
    expect(body.inScopeOf).toContain('epr:lamad-spa');
    const meta = JSON.parse(body.metadataJson);
    expect(meta.urlPath).toBe('/lamad');
    expect(meta.mode).toBe('cached');
    expect(meta.reach).toBe('commons');
  });

  it('produces deterministic id for same spec (idempotent re-seed)', () => {
    const spec: ProjectionSpec = {
      stewardHumanId: 'human-matthew-manager',
      stewardArchetype: 'manager',
      doorwayId: 'alpha-elohim-host',
      eprId: 'lamad-spa',
      urlPath: '/lamad',
      mode: 'cached',
      reach: 'commons',
      baseHref: '/lamad/',
      entryFile: 'index.html',
      redirectsFrom: [],
      previewEprRef: null,
      gateHints: [],
      deadEnd: false,
      stewardDirectEndpoint: null,
    };
    const a = buildProjectionCommitmentBody(spec);
    const b = buildProjectionCommitmentBody(spec);
    expect(a.id).toBe(b.id);
  });

  it('produces different ids for different (doorway, epr) pairs', () => {
    const base: ProjectionSpec = {
      stewardHumanId: 'human-matthew-manager',
      stewardArchetype: 'manager',
      doorwayId: 'alpha-elohim-host',
      eprId: 'lamad-spa',
      urlPath: '/lamad',
      mode: 'cached',
      reach: 'commons',
      baseHref: '/lamad/',
      entryFile: 'index.html',
      redirectsFrom: [],
      previewEprRef: null,
      gateHints: [],
      deadEnd: false,
      stewardDirectEndpoint: null,
    };
    const a = buildProjectionCommitmentBody(base);
    const b = buildProjectionCommitmentBody({ ...base, doorwayId: 'elohim-host' });
    const c = buildProjectionCommitmentBody({ ...base, eprId: 'elohim-host-landing' });
    expect(a.id).not.toBe(b.id);
    expect(a.id).not.toBe(c.id);
  });
});
```

- [ ] **Step 3: Run tests to verify failure**

```bash
cd genesis/seeder
pnpm vitest run src/__tests__/seed-projections.test.ts 2>&1 | tail -15
```

Expected: FAIL — `seed-projections.ts` doesn't exist.

- [ ] **Step 4: Implement seed-projections.ts**

Create `genesis/seeder/src/seed-projections.ts`:

```typescript
/**
 * Seed EPR-Projection Commitments (REA project-epr action)
 *
 * Each projection is an REA Commitment with action='project-epr' that
 * notarizes "doorway D will project EPR E at urlPath U under terms T."
 *
 * Substrate references:
 *   - elohim/sdk/schemas/v1/views/epr-projection-view.schema.json
 *   - elohim/elohim-storage/src/db/rea_commitments.rs (PROJECT_EPR_ACTION + validator)
 *   - genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md
 *
 * Sister to seed-operator-bindings.ts — same POST endpoint, different
 * action discriminator. Same idempotency story: id is content-addressed
 * over (steward_peer_id, action, scope), re-runs collapse to 409.
 *
 * Default MVP projection set:
 *   - elohim-host-landing @ doorway:alpha-elohim-host  urlPath: "/"
 *   - elohim-host-landing @ doorway:elohim-host        urlPath: "/"
 *   - lamad-spa            @ doorway:alpha-elohim-host  urlPath: "/lamad"
 *   - lamad-spa            @ doorway:elohim-host        urlPath: "/lamad"
 *
 * Two EPRs, two doorways, four projections. All commons-reach, cached mode.
 *
 * Usage:
 *   DOORWAY_URL=http://localhost:8888 npx tsx src/seed-projections.ts
 *   DOORWAY_URL=https://alpha.elohim.host DOORWAY_API_KEY=xxx \
 *     PROJECTIONS_JSON=./projections.json npx tsx src/seed-projections.ts
 */

import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { DoorwayClient } from './doorway-client.js';
import { deterministicPeerId, type Archetype } from './peer-id.js';

// =============================================================================
// Types
// =============================================================================

export type ProjectionMode = 'cached' | 'stewardDirect';

export interface GateHintRef {
  eprRef: string;
  label: string | null;
  relation:
    | 'personWhoCanGrant' | 'membershipPrerequisite' | 'contentToSync'
    | 'placeToVisit'      | 'capabilityToEarn'        | 'paymentToOffer'
    | 'witnessToInvolve';
}

export interface StewardDirectEndpoint {
  peerId: string;
  altHost: string | null;
  tlsCertSan: string;
  acceptsProjectionFor: string[];
}

export interface ProjectionSpec {
  stewardHumanId: string;
  stewardArchetype: Archetype;
  doorwayId: string;            // e.g. "alpha-elohim-host"
  eprId: string;                // e.g. "lamad-spa"
  urlPath: string;              // e.g. "/lamad"
  mode: ProjectionMode;
  reach: string;                // "commons" | "qahal:xyz" | etc.
  baseHref: string;
  entryFile: string;
  redirectsFrom: string[];
  previewEprRef: string | null;
  gateHints: GateHintRef[];
  deadEnd: boolean;
  stewardDirectEndpoint: StewardDirectEndpoint | null;
}

interface CommitmentBody {
  id: string;
  action: 'project-epr';
  provider: string;
  receiver: string;
  inScopeOf: string;
  note: string;
  metadataJson: string;
}

// =============================================================================
// Body builder (testable in isolation)
// =============================================================================

export function buildProjectionCommitmentBody(spec: ProjectionSpec): CommitmentBody {
  const stewardPeerId = deterministicPeerId(spec.stewardHumanId, spec.stewardArchetype);
  const scope = `doorway:${spec.doorwayId}|epr:${spec.eprId}`;

  const idDigest = createHash('sha256')
    .update(`${stewardPeerId}|project-epr|${scope}`, 'utf8')
    .digest('hex')
    .slice(0, 16);

  const metadata = {
    urlPath: spec.urlPath,
    mode: spec.mode,
    reach: spec.reach,
    baseHref: spec.baseHref,
    entryFile: spec.entryFile,
    redirectsFrom: spec.redirectsFrom,
    previewEprRef: spec.previewEprRef,
    gateHints: spec.gateHints,
    deadEnd: spec.deadEnd,
    stewardDirectEndpoint: spec.stewardDirectEndpoint,
  };

  return {
    id: `project-epr-${idDigest}`,
    action: 'project-epr',
    provider: stewardPeerId,
    receiver: stewardPeerId,   // for MVP both ends are the same steward
    inScopeOf: scope,
    note: `Project ${spec.eprId} at ${spec.urlPath} on ${spec.doorwayId}`,
    metadataJson: JSON.stringify(metadata),
  };
}

// =============================================================================
// Default seed set (MVP)
// =============================================================================

export function defaultProjectionSeeds(): ProjectionSpec[] {
  const base = {
    stewardHumanId: 'human-matthew-manager',
    stewardArchetype: 'manager' as Archetype,
    mode: 'cached' as ProjectionMode,
    reach: 'commons',
    entryFile: 'index.html',
    redirectsFrom: [],
    previewEprRef: null,
    gateHints: [],
    deadEnd: false,
    stewardDirectEndpoint: null,
  };

  const landingAt = (doorwayId: string): ProjectionSpec => ({
    ...base,
    doorwayId,
    eprId: 'elohim-host-landing',
    urlPath: '/',
    baseHref: '/',
  });

  const lamadAt = (doorwayId: string): ProjectionSpec => ({
    ...base,
    doorwayId,
    eprId: 'lamad-spa',
    urlPath: '/lamad',
    baseHref: '/lamad/',
  });

  return [
    landingAt('alpha-elohim-host'),
    landingAt('elohim-host'),
    lamadAt('alpha-elohim-host'),
    lamadAt('elohim-host'),
  ];
}

// =============================================================================
// Client + runner
// =============================================================================

class ProjectionClient extends DoorwayClient {
  async createCommitment(body: CommitmentBody): Promise<Response> {
    return this.fetch('/db/rea_commitments', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
  }
}

export async function seedProjections(
  client: ProjectionClient,
  specs: ProjectionSpec[],
): Promise<void> {
  for (const spec of specs) {
    const body = buildProjectionCommitmentBody(spec);
    const response = await client.createCommitment(body);
    if (response.status === 201) {
      console.log(`  ✓ created ${body.id} (${spec.eprId} @ ${spec.urlPath} on ${spec.doorwayId})`);
    } else if (response.status === 409) {
      console.log(`  ⊜ already exists ${body.id}`);
    } else {
      const text = await response.text();
      throw new Error(`Seed failed: ${response.status} ${text}`);
    }
  }
}

// =============================================================================
// CLI entry
// =============================================================================

async function main() {
  const doorwayUrl = process.env.DOORWAY_URL || 'http://localhost:8888';
  const apiKey = process.env.DOORWAY_API_KEY;
  const customJson = process.env.PROJECTIONS_JSON;

  const specs: ProjectionSpec[] = customJson
    ? JSON.parse(readFileSync(customJson, 'utf8'))
    : defaultProjectionSeeds();

  const client = new ProjectionClient({ baseUrl: doorwayUrl, apiKey });

  console.log(`EPR-Projection Seeder — ${specs.length} projections`);
  console.log(`Target: ${doorwayUrl}\n`);

  await seedProjections(client, specs);

  console.log('\nDone.');
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(err => {
    console.error(err);
    process.exit(1);
  });
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd genesis/seeder
pnpm vitest run src/__tests__/seed-projections.test.ts 2>&1 | tail -10
```

Expected: 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add genesis/seeder/src/seed-projections.ts genesis/seeder/src/__tests__/seed-projections.test.ts
git commit -m "feat(seeder): seed-projections.ts (4 default projections, idempotent)"
```

---

## Task A12: Wire seed-projections into main seed flow

**Files:**
- Modify: `genesis/seeder/src/seed.ts`

- [ ] **Step 1: Find where seed-operator-bindings is invoked**

```bash
grep -n "seedOperatorBindings\|seed-operator-bindings\|operator-bindings" genesis/seeder/src/seed.ts
```

- [ ] **Step 2: Add the projection seeding after operator bindings**

In `seed.ts`, after the operator-bindings step:

```typescript
import { seedProjections, defaultProjectionSeeds } from './seed-projections.js';
import { DoorwayClient } from './doorway-client.js';

// ... after seedOperatorBindings call ...

console.log('\n=== Seeding EPR Projections ===');
const projectionClient = new DoorwayClient({ baseUrl: doorwayUrl, apiKey });
await seedProjections(projectionClient as any, defaultProjectionSeeds());
```

(adapt to the actual structure of seed.ts)

- [ ] **Step 3: Run a dry-run end-to-end seed locally**

```bash
cd genesis/seeder
DOORWAY_URL=http://localhost:8888 pnpm exec tsx src/seed.ts 2>&1 | tail -20
```

(requires a local dev stack running per `pnpm run hc:start` from elohim-app)

Expected: Seeding succeeds; 4 projection lines appear in output.

- [ ] **Step 4: Commit**

```bash
git add genesis/seeder/src/seed.ts
git commit -m "feat(seeder): wire projections into main seed flow"
```

---

## Task A13: Create elohim-core-elements registry seed JSON

**Files:**
- Create: `genesis/data/elements/elohim-core-registry.json`

- [ ] **Step 1: Look at how other content rows are seeded**

```bash
ls genesis/data/lamad/content/ | head -10
cat genesis/data/lamad/content/elohim-host-landing.json | head -30
```

- [ ] **Step 2: Create the elements directory and registry seed**

```bash
mkdir -p genesis/data/elements
```

Create `genesis/data/elements/elohim-core-registry.json`:

```json
{
  "id": "elohim-core-elements",
  "contentType": "element-registry",
  "title": "Elohim Core Element Registry",
  "description": "Manifest of custom elements published by the elohim-core package — base atoms, page chrome, EPR-link primitive, context menu, etc. Imported at build time by every pillar bundle.",
  "content": {
    "eprId": "elohim-core-elements",
    "pillar": "elohim-core",
    "elements": [
      { "tagName": "elohim-button", "cid": "sha256-pending-build", "version": "1.0.0", "viewDeps": [] },
      { "tagName": "elohim-card", "cid": "sha256-pending-build", "version": "1.0.0", "viewDeps": [] },
      { "tagName": "elohim-compute-tile", "cid": "sha256-pending-build", "version": "1.0.0", "viewDeps": ["ComputeTileView"] },
      { "tagName": "elohim-epr-link", "cid": "sha256-pending-build", "version": "1.0.0", "viewDeps": ["EprHead"] },
      { "tagName": "elohim-page-chrome", "cid": "sha256-pending-build", "version": "1.0.0", "viewDeps": [] },
      { "tagName": "elohim-default-omnibar", "cid": "sha256-pending-build", "version": "1.0.0", "viewDeps": ["CurrentUserView"] },
      { "tagName": "elohim-skeleton", "cid": "sha256-pending-build", "version": "1.0.0", "viewDeps": [] },
      { "tagName": "elohim-mention-base", "cid": "sha256-pending-build", "version": "1.0.0", "viewDeps": [] },
      { "tagName": "elohim-context-menu", "cid": "sha256-pending-build", "version": "1.0.0", "viewDeps": [] }
    ]
  },
  "contentFormat": "element-registry-manifest",
  "tags": ["element-registry", "elohim-core"],
  "blobHash": "",
  "reach": "commons",
  "metadata": {
    "category": "element-registry",
    "blobPopulatedBy": "Jenkinsfile post-build: write elohim-core package bundle CIDs into the elements[*].cid fields, then PATCH this content row."
  },
  "createdAt": "2026-05-25T00:00:00.000000",
  "updatedAt": "2026-05-25T00:00:00.000000"
}
```

- [ ] **Step 3: Verify the seed flows through the existing content seeder**

```bash
grep -rn "genesis/data/elements\|elements/\*" genesis/seeder/src/ 2>/dev/null | head -5
```

If the seeder doesn't pick up `genesis/data/elements/` automatically, add it. Most likely it does via a directory walker. If not, find the directory list and add `elements`.

- [ ] **Step 4: Commit**

```bash
git add genesis/data/elements/elohim-core-registry.json
git commit -m "feat(seed): elohim-core-elements registry content row"
```

---

## Task A14: Phase A integration test — full seed produces expected substrate state

**Files:**
- Reuse: existing seeder integration test infrastructure

- [ ] **Step 1: Locate seeder integration test pattern**

```bash
find genesis/seeder/src/__tests__/integration -name "*.test.ts" 2>/dev/null | head -3
cat genesis/seeder/src/__tests__/integration/import-pipeline.test.ts 2>/dev/null | head -40
```

- [ ] **Step 2: Add a Phase A end-to-end test**

Create `genesis/seeder/src/__tests__/integration/projections-substrate.test.ts`:

```typescript
import { describe, it, expect, beforeAll } from 'vitest';
import { DoorwayClient } from '../../doorway-client.js';
import { seedProjections, defaultProjectionSeeds } from '../../seed-projections.js';

const DOORWAY_URL = process.env.TEST_DOORWAY_URL || 'http://localhost:8888';

describe('Projections substrate — end-to-end', () => {
  let client: DoorwayClient;

  beforeAll(() => {
    client = new DoorwayClient({ baseUrl: DOORWAY_URL });
  });

  it('seeds 4 default projections', async () => {
    await seedProjections(client as any, defaultProjectionSeeds());
    // Re-seed should be idempotent (409 collapse)
    await seedProjections(client as any, defaultProjectionSeeds());
  });

  it('queries projections by doorway_id and gets exactly 2 each', async () => {
    const alpha = await client.fetch('/db/rea_commitments?action=project-epr&doorwayId=alpha-elohim-host');
    const alphaJson = await alpha.json();
    expect(alphaJson.length).toBe(2);

    const beta = await client.fetch('/db/rea_commitments?action=project-epr&doorwayId=elohim-host');
    const betaJson = await beta.json();
    expect(betaJson.length).toBe(2);
  });

  it('validates: cannot create gated projection without preview/hints/deadEnd', async () => {
    const badBody = {
      id: 'bad-test-id',
      action: 'project-epr',
      provider: 'test',
      receiver: 'test',
      inScopeOf: 'doorway:test|epr:test',
      note: 'should fail',
      metadataJson: JSON.stringify({
        urlPath: '/test',
        mode: 'cached',
        reach: 'qahal:nonexistent',  // gated
        baseHref: '/test/',
        entryFile: 'index.html',
        redirectsFrom: [],
        previewEprRef: null,
        gateHints: [],
        deadEnd: false,
      }),
    };
    const response = await client.fetch('/db/rea_commitments', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(badBody),
    });
    expect(response.status).toBe(400);
  });
});
```

- [ ] **Step 3: Run integration test against local dev stack**

```bash
# Start dev stack in another terminal: pnpm run hc:start
cd genesis/seeder
TEST_DOORWAY_URL=http://localhost:8888 pnpm vitest run src/__tests__/integration/projections-substrate.test.ts 2>&1 | tail -20
```

Expected: All 3 tests pass.

- [ ] **Step 4: Commit**

```bash
git add genesis/seeder/src/__tests__/integration/projections-substrate.test.ts
git commit -m "test(seeder): integration test for Phase A substrate end-to-end"
```

---

## Phase A Checkpoint

- [ ] **Verify Phase A complete**

```bash
# Run all Phase A tests
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract
cd ../elohim-views && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test
cd ../../genesis/seeder && pnpm vitest run src/__tests__/seed-projections.test.ts
```

Expected: all green.

- [ ] **Commit Phase A summary tag** (optional, for clean handoff)

```bash
git tag phase-a-pillar-epr-substrate
```

Phase A complete: substrate primitive in place, validator enforced, resolvers implemented, events emitted, 4 default projections seedable, integration tested. Phase B can now build on this foundation.

---

# PHASE B — Delivery (Doorway router + elohim-core extract + lamad bundle split)

Phase B produces user-visible delivery. Requires Phase A complete (commitments exist in storage; events flow through the SSE bus).

## Task B0: Audit lamad pillar service dependencies on elohim pillar

**Files:**
- Create: `genesis/docs/superpowers/notes/2026-05-25-lamad-pillar-dependency-audit.md`

**Purpose:** Open question #4 in the spec — the lamad pillar's 37 services may import from the elohim pillar's 60 services. Before moving lamad to its own bundle, we need to know which dependencies exist so we can decide (extract to elohim-core / duplicate in lamad / consume via HTTP API) per case.

- [ ] **Step 1: Run the audit**

```bash
echo "=== lamad pillar imports from elohim pillar ===" > /tmp/lamad-deps.txt
grep -rn "from '@app/elohim'\|from '../elohim/\|from '../../elohim/" app/elohim-app/src/app/lamad/ 2>/dev/null \
  | grep -v ".spec.ts" >> /tmp/lamad-deps.txt
echo "" >> /tmp/lamad-deps.txt
echo "=== lamad pillar imports from imagodei/qahal/shefa/avodah pillars ===" >> /tmp/lamad-deps.txt
grep -rn "from '@app/imagodei\|from '@app/qahal\|from '@app/shefa\|from '@app/avodah" app/elohim-app/src/app/lamad/ 2>/dev/null \
  | grep -v ".spec.ts" >> /tmp/lamad-deps.txt
echo "" >> /tmp/lamad-deps.txt
echo "=== Service-name frequency in the dependency set ===" >> /tmp/lamad-deps.txt
grep -h "import.*from" /tmp/lamad-deps.txt | grep -oE '\{[^}]+\}' | tr ',' '\n' | sort -u >> /tmp/lamad-deps.txt
cat /tmp/lamad-deps.txt
```

- [ ] **Step 2: Write the audit report**

Create `genesis/docs/superpowers/notes/2026-05-25-lamad-pillar-dependency-audit.md`:

```markdown
# Lamad Pillar Dependency Audit (for bundle split)

**Generated:** 2026-05-25
**Source:** `grep -rn "from '@app/elohim'..." app/elohim-app/src/app/lamad/`
**Purpose:** Per pillar-EPR-decomposition spec §11.4, identify cross-pillar
imports before splitting lamad to its own bundle.

## Dependencies on @app/elohim pillar

[paste filtered grep results, grouped by service/component imported]

## Dependencies on other pillars (imagodei, qahal, shefa, avodah)

[paste filtered results]

## Disposition decisions

For each unique imported symbol:

| Symbol | Used by | Disposition |
|---|---|---|
| ContentService | (count) lamad files | Extract to elohim-core (truly cross-pillar) |
| EprResolverService | (count) lamad files | Extract to elohim-core (per spec §4.1) |
| (etc.) | | (extract / duplicate / consume-via-API) |

## Open items

- [list any cross-pillar imports that require design discussion before
  the bundle split can proceed]
```

Fill in the actual data from the audit.

- [ ] **Step 3: Commit**

```bash
git add genesis/docs/superpowers/notes/2026-05-25-lamad-pillar-dependency-audit.md
git commit -m "docs(audit): lamad pillar dependency audit pre bundle-split"
```

This audit informs subsequent tasks. If audit reveals heavy cross-pillar coupling not anticipated, pause and discuss with operator before proceeding.

---

## Task B1: Loader class — transport-agnostic CID resolution

**Files:**
- Create: `app/elohim-elements/elohim-core/src/loader/loader.ts`
- Create: `app/elohim-elements/elohim-core/src/loader/loader.spec.ts`

- [ ] **Step 1: Write the failing test**

Create `app/elohim-elements/elohim-core/src/loader/loader.spec.ts`:

```typescript
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { Loader, type LoaderTransport } from './loader.js';

class FakeTransport implements LoaderTransport {
  constructor(public name: string, public response: Uint8Array | 'fail' | 'unavailable') {}

  async fetch(cid: string): Promise<Uint8Array | null> {
    if (this.response === 'fail') throw new Error(`${this.name} fetch error`);
    if (this.response === 'unavailable') return null;
    return this.response;
  }

  isAvailable(): boolean {
    return this.response !== 'unavailable';
  }
}

describe('Loader', () => {
  const SAMPLE_BYTES = new Uint8Array([1, 2, 3, 4]);
  const SAMPLE_CID = 'sha256-9f64a747e1b97f131fabb6b447296c9b6f0201e79fb3c5356e6c77e89b6a806a';
  // (CID computed for SAMPLE_BYTES; replace with actual hash in real test)

  it('resolves from the first available transport (localCache hit)', async () => {
    const loader = new Loader([
      new FakeTransport('localCache', SAMPLE_BYTES),
      new FakeTransport('tauri', 'fail'),
      new FakeTransport('doorway', 'fail'),
    ], { verifyCid: false });  // skip CID verification for this test
    const result = await loader.resolve(SAMPLE_CID);
    expect(result.bytes).toEqual(SAMPLE_BYTES);
    expect(result.source).toBe('localCache');
  });

  it('falls through to next transport when first returns null', async () => {
    const loader = new Loader([
      new FakeTransport('localCache', 'unavailable'),
      new FakeTransport('tauri', SAMPLE_BYTES),
    ], { verifyCid: false });
    const result = await loader.resolve(SAMPLE_CID);
    expect(result.source).toBe('tauri');
  });

  it('falls through to next transport when first throws', async () => {
    const loader = new Loader([
      new FakeTransport('doorway', 'fail'),
      new FakeTransport('peer', SAMPLE_BYTES),
    ], { verifyCid: false });
    const result = await loader.resolve(SAMPLE_CID);
    expect(result.source).toBe('peer');
  });

  it('returns unresolved when all transports fail', async () => {
    const loader = new Loader([
      new FakeTransport('doorway', 'fail'),
      new FakeTransport('peer', 'unavailable'),
    ], { verifyCid: false });
    const result = await loader.resolve(SAMPLE_CID);
    expect(result.bytes).toBeNull();
    expect(result.source).toBe('unresolved');
  });

  it('verifies CID matches returned bytes when verifyCid is true', async () => {
    // SAMPLE_BYTES has a known sha256; resolve with a WRONG CID should fail
    const loader = new Loader([
      new FakeTransport('doorway', SAMPLE_BYTES),
    ], { verifyCid: true });
    await expect(loader.resolve('sha256-wrongwrongwrong')).rejects.toThrow(/CID mismatch/);
  });
});
```

- [ ] **Step 2: Run test to verify failure**

```bash
cd app/elohim-elements/elohim-core
pnpm vitest run src/loader/loader.spec.ts 2>&1 | tail -15
```

Expected: COMPILE FAILURE — loader.ts not found.

- [ ] **Step 3: Implement the Loader**

Create `app/elohim-elements/elohim-core/src/loader/loader.ts`:

```typescript
/**
 * Loader — transport-agnostic content-addressed resolution.
 *
 * The protocol is peer-native first. Bundles, elements, and arbitrary
 * EPR content all resolve via CID. The Loader tries a configured list
 * of transports in order:
 *
 *   1. localCache  — service-worker / IndexedDB cache
 *   2. tauri       — direct HTTP to localhost:8090 storage instance
 *   3. doorway     — projection cache via current origin's doorway
 *   4. peer (future) — WebTransport / WebRTC to other peers
 *
 * Each transport returns either bytes, null (unavailable for this CID),
 * or throws (transport-level error — try next). When all transports
 * fail, returns { bytes: null, source: 'unresolved' }.
 *
 * Optional CID verification re-hashes the returned bytes and refuses
 * to return them if the hash doesn't match the requested CID. ALWAYS
 * enabled in production; disabled in tests where computing real SHAs
 * is unnecessary friction.
 */

export interface LoaderTransport {
  /** Source name for telemetry / debugging. */
  readonly name: string;
  /** Returns bytes if available, null if unavailable, throws on transport error. */
  fetch(cid: string): Promise<Uint8Array | null>;
  /** Whether this transport is reachable right now. */
  isAvailable(): boolean;
}

export interface LoaderResolution {
  bytes: Uint8Array | null;
  source: string; // transport name or 'unresolved'
  cid: string;
}

export interface LoaderOptions {
  verifyCid?: boolean;
}

export class Loader {
  constructor(
    private readonly transports: LoaderTransport[],
    private readonly options: LoaderOptions = { verifyCid: true },
  ) {}

  async resolve(cid: string): Promise<LoaderResolution> {
    for (const transport of this.transports) {
      if (!transport.isAvailable()) continue;
      try {
        const bytes = await transport.fetch(cid);
        if (bytes === null) continue;
        if (this.options.verifyCid) {
          await this.verifyCidMatches(cid, bytes);
        }
        return { bytes, source: transport.name, cid };
      } catch {
        // Transport-level error → try next.
        continue;
      }
    }
    return { bytes: null, source: 'unresolved', cid };
  }

  private async verifyCidMatches(cid: string, bytes: Uint8Array): Promise<void> {
    // CID format: "sha256-<hex>"
    const [algo, expected] = cid.split('-');
    if (algo !== 'sha256') throw new Error(`Unsupported CID algo: ${algo}`);
    const hashBuffer = await crypto.subtle.digest('SHA-256', bytes);
    const actual = Array.from(new Uint8Array(hashBuffer))
      .map(b => b.toString(16).padStart(2, '0'))
      .join('');
    if (actual !== expected) {
      throw new Error(`CID mismatch: expected ${expected}, got ${actual}`);
    }
  }
}
```

- [ ] **Step 4: Run tests, all should pass except possibly the CID-verification one which needs a real hash**

```bash
pnpm vitest run src/loader/loader.spec.ts 2>&1 | tail -10
```

- [ ] **Step 5: Fix the CID-verification test with a real hash**

Compute the actual sha256 of `[1, 2, 3, 4]`:

```bash
echo -ne '\x01\x02\x03\x04' | sha256sum
# Output: 9f64a747e1b97f131fabb6b447296c9b6f0201e79fb3c5356e6c77e89b6a806a
```

Confirm the SAMPLE_CID in the test matches this. If not, update.

- [ ] **Step 6: Commit**

```bash
git add app/elohim-elements/elohim-core/src/loader/
git commit -m "feat(elohim-core): Loader — transport-agnostic CID resolution with fallback chain"
```

---

## Task B2: Session primitive

**Files:**
- Create: `app/elohim-elements/elohim-core/src/session/session.ts`
- Create: `app/elohim-elements/elohim-core/src/session/session.spec.ts`

- [ ] **Step 1: Write the failing tests**

Create `app/elohim-elements/elohim-core/src/session/session.spec.ts`:

```typescript
import { describe, it, expect, beforeEach } from 'vitest';
import { Session } from './session.js';

describe('Session', () => {
  beforeEach(() => {
    document.cookie = 'elohim_session=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=/;';
  });

  it('exposes null currentUser when no session cookie is present', () => {
    const s = new Session();
    expect(s.currentUser).toBeNull();
    expect(s.isAuthenticated).toBe(false);
  });

  it('parses the session cookie and exposes currentUser', () => {
    const fakeSession = {
      humanId: 'matthew',
      capabilities: ['*'],
      reach: 'authenticated',
    };
    document.cookie = `elohim_session=${encodeURIComponent(JSON.stringify(fakeSession))}; path=/`;
    const s = new Session();
    expect(s.currentUser?.humanId).toBe('matthew');
    expect(s.isAuthenticated).toBe(true);
  });

  it('notifies subscribers when refreshFromCookies updates state', () => {
    const s = new Session();
    let notifyCount = 0;
    s.subscribe(() => notifyCount++);
    document.cookie = `elohim_session=${encodeURIComponent(JSON.stringify({ humanId: 'matthew', capabilities: [], reach: 'authenticated' }))}; path=/`;
    s.refreshFromCookies();
    expect(notifyCount).toBe(1);
    expect(s.currentUser?.humanId).toBe('matthew');
  });
});
```

- [ ] **Step 2: Run to see failure**

```bash
pnpm vitest run src/session/session.spec.ts 2>&1 | tail -10
```

Expected: COMPILE FAILURE.

- [ ] **Step 3: Implement Session**

Create `app/elohim-elements/elohim-core/src/session/session.ts`:

```typescript
/**
 * Session — reactive identity/capability state for the current browser.
 *
 * Reads the elohim_session cookie set by doorway-side auth flows.
 * Pure TS; no DOM rendering. Elements that need session state subscribe
 * and re-render on change.
 *
 * Cookie shape (set by doorway after successful imagodei auth):
 *   elohim_session={"humanId":"...","capabilities":[...],"reach":"..."}
 *
 * In practice the cookie is signed/encrypted by doorway; this class
 * parses the public JSON portion. Verification of authenticity is
 * doorway's responsibility — by the time the cookie reaches us, it
 * is trusted within the browser's origin boundary.
 */

export interface CurrentUserView {
  humanId: string;
  capabilities: string[];
  reach: string;
}

type Subscriber = (user: CurrentUserView | null) => void;

export class Session {
  private _currentUser: CurrentUserView | null = null;
  private subscribers = new Set<Subscriber>();

  constructor() {
    this.refreshFromCookies();
  }

  get currentUser(): CurrentUserView | null {
    return this._currentUser;
  }

  get isAuthenticated(): boolean {
    return this._currentUser !== null;
  }

  refreshFromCookies(): void {
    const cookie = this.readCookie('elohim_session');
    const prev = this._currentUser;
    if (!cookie) {
      this._currentUser = null;
    } else {
      try {
        this._currentUser = JSON.parse(decodeURIComponent(cookie));
      } catch {
        this._currentUser = null;
      }
    }
    if (this.shallowEquals(prev, this._currentUser)) return;
    this.subscribers.forEach(s => s(this._currentUser));
  }

  subscribe(fn: Subscriber): () => void {
    this.subscribers.add(fn);
    return () => this.subscribers.delete(fn);
  }

  private readCookie(name: string): string | null {
    const all = document.cookie.split(';').map(c => c.trim());
    const found = all.find(c => c.startsWith(`${name}=`));
    return found ? found.slice(name.length + 1) : null;
  }

  private shallowEquals(a: CurrentUserView | null, b: CurrentUserView | null): boolean {
    if (a === b) return true;
    if (!a || !b) return false;
    return a.humanId === b.humanId && a.reach === b.reach
      && a.capabilities.length === b.capabilities.length
      && a.capabilities.every((cap, i) => cap === b.capabilities[i]);
  }
}
```

- [ ] **Step 4: Run tests**

```bash
pnpm vitest run src/session/session.spec.ts 2>&1 | tail -10
```

Expected: 3 pass.

- [ ] **Step 5: Commit**

```bash
git add app/elohim-elements/elohim-core/src/session/
git commit -m "feat(elohim-core): Session primitive — reactive identity state from cookie"
```

---

## Task B3: Omnibar contract types

**Files:**
- Create: `app/elohim-elements/elohim-core/src/contracts/omnibar.contract.ts`

- [ ] **Step 1: Write the contract**

Create `app/elohim-elements/elohim-core/src/contracts/omnibar.contract.ts`:

```typescript
import type { CurrentUserView } from '../session/session.js';

/**
 * The OmnibarContext object passed to any element placed in
 * <elohim-page-chrome slot="omnibar">. Custom omnibar implementations
 * read this and render accordingly.
 *
 * The contract is duck-typed at runtime: <elohim-page-chrome> sets
 * the context as a property on its slotted child. The child reads
 * (or ignores) it freely.
 */
export interface OmnibarContext {
  currentUser: CurrentUserView | null;
  currentEpr: EprRef | null;
  capabilities: CapabilitySnapshot;
  reach: ReachContext;
  onNavigate: (target: EprRef) => void;
}

export interface EprRef {
  eprId: string;
  displayLabel?: string;
}

export interface CapabilitySnapshot {
  has(capability: string): boolean;
  list: string[];
}

export interface ReachContext {
  kind: 'anonymous' | 'authenticated' | 'household' | 'qahal';
  details: Record<string, unknown>;
}
```

- [ ] **Step 2: Commit (no test for pure type declarations)**

```bash
git add app/elohim-elements/elohim-core/src/contracts/
git commit -m "feat(elohim-core): omnibar contract types"
```

---

## Task B4: `<elohim-skeleton>` element (sized shimmer placeholder)

**Files:**
- Create: `app/elohim-elements/elohim-core/src/elohim-skeleton.ts`

- [ ] **Step 1: Implement (small element, no test needed beyond manual story)**

Create `app/elohim-elements/elohim-core/src/elohim-skeleton.ts`:

```typescript
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

/**
 * <elohim-skeleton> — sized shimmer placeholder used during progressive
 * loading. Renders an empty box at the declared width/height with a
 * subtle shimmer animation. Page doesn't reflow when real content arrives.
 *
 * @element elohim-skeleton
 *
 * @prop {string} width  - CSS width (e.g. "200px" or "100%")
 * @prop {string} height - CSS height
 * @prop {string} radius - Optional border-radius (default: var(--elohim-radius-sm, 4px))
 *
 * @cssprop --elohim-skeleton-bg     - Override base color
 * @cssprop --elohim-skeleton-shimmer - Override shimmer highlight color
 */
export class ElohimSkeleton extends LitElement {
  @property() width = '100%';
  @property() height = '1rem';
  @property() radius = 'var(--elohim-radius-sm, 4px)';

  static styles = css`
    :host {
      display: inline-block;
      background: var(--elohim-skeleton-bg, color-mix(in oklch, currentColor 12%, transparent));
      border-radius: var(--elohim-skeleton-radius, var(--elohim-radius-sm, 4px));
      position: relative;
      overflow: hidden;
    }
    :host::before {
      content: '';
      position: absolute;
      inset: 0;
      background: linear-gradient(
        90deg,
        transparent 0%,
        var(--elohim-skeleton-shimmer, color-mix(in oklch, currentColor 20%, transparent)) 50%,
        transparent 100%
      );
      animation: shimmer 1.5s infinite;
    }
    @keyframes shimmer {
      0% { transform: translateX(-100%); }
      100% { transform: translateX(100%); }
    }
    @media (prefers-reduced-motion: reduce) {
      :host::before { animation: none; }
    }
  `;

  render() {
    return html`<style>:host{width:${this.width};height:${this.height};border-radius:${this.radius};}</style>`;
  }
}
```

- [ ] **Step 2: Add to register.ts**

In `app/elohim-elements/elohim-core/src/register.ts`:

```typescript
import { ElohimSkeleton } from './elohim-skeleton.js';
// ...
if (!customElements.get('elohim-skeleton')) {
  customElements.define('elohim-skeleton', ElohimSkeleton);
}
```

- [ ] **Step 3: Add to index.ts exports**

```typescript
export { ElohimSkeleton } from './elohim-skeleton.js';
```

- [ ] **Step 4: Verify build**

```bash
cd app/elohim-elements/elohim-core
pnpm build 2>&1 | tail -10
```

Expected: build succeeds.

- [ ] **Step 5: Commit**

```bash
git add app/elohim-elements/elohim-core/src/elohim-skeleton.ts \
        app/elohim-elements/elohim-core/src/register.ts \
        app/elohim-elements/elohim-core/src/index.ts
git commit -m "feat(elohim-core): <elohim-skeleton> sized shimmer placeholder"
```

---

## Task B5: `<elohim-mention-base>` — generic cross-pillar fallback chip

**Files:**
- Create: `app/elohim-elements/elohim-core/src/elohim-mention-base.ts`
- Modify: `register.ts`, `index.ts`

- [ ] **Step 1: Implement**

Create `app/elohim-elements/elohim-core/src/elohim-mention-base.ts`:

```typescript
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

/**
 * <elohim-mention-base> — the generic fallback chip when a specific pillar's
 * mention element (e.g. <qahal-mention>) isn't loaded. Renders title + EPR id
 * from whatever metadata is resolvable, with a subtle indicator that this is
 * a fallback rendering.
 *
 * @element elohim-mention-base
 * @prop {string} epr - The EPR id being mentioned
 * @prop {string} title - Optional title (rendered if known)
 * @prop {string} pillar - Optional pillar hint for iconography
 */
export class ElohimMentionBase extends LitElement {
  @property() epr = '';
  @property() title = '';
  @property() pillar = '';

  static styles = css`
    :host {
      display: inline-flex;
      align-items: center;
      gap: 0.25rem;
      padding: 0.125rem 0.5rem;
      border: 1px solid color-mix(in oklch, currentColor 25%, transparent);
      border-radius: 999px;
      font-size: 0.875rem;
    }
    .id { opacity: 0.6; font-size: 0.75rem; }
    .fallback-mark {
      width: 6px; height: 6px;
      background: color-mix(in oklch, currentColor 40%, transparent);
      border-radius: 50%;
    }
  `;

  render() {
    return html`
      <span class="fallback-mark" title="generic chip (pillar element not loaded)"></span>
      <span class="title">${this.title || this.epr}</span>
      ${this.title ? html`<span class="id">${this.epr}</span>` : ''}
    `;
  }
}
```

- [ ] **Step 2: Register + export + commit**

(Pattern from Task B4)

```bash
git add app/elohim-elements/elohim-core/src/elohim-mention-base.ts \
        app/elohim-elements/elohim-core/src/register.ts \
        app/elohim-elements/elohim-core/src/index.ts
git commit -m "feat(elohim-core): <elohim-mention-base> generic cross-pillar fallback chip"
```

---

## Task B6: `<elohim-page-chrome>` + `<elohim-default-omnibar>`

**Files:**
- Create: `app/elohim-elements/elohim-core/src/elohim-page-chrome.ts`
- Create: `app/elohim-elements/elohim-core/src/elohim-page-chrome.spec.ts`
- Create: `app/elohim-elements/elohim-core/src/elohim-default-omnibar.ts`

- [ ] **Step 1: Write tests for page-chrome**

Create `app/elohim-elements/elohim-core/src/elohim-page-chrome.spec.ts`:

```typescript
import { describe, it, expect, beforeEach } from 'vitest';
import './elohim-page-chrome.js';
import './elohim-default-omnibar.js';

describe('<elohim-page-chrome>', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
  });

  it('renders <elohim-default-omnibar> when no slotted omnibar present', async () => {
    document.body.innerHTML = `
      <elohim-page-chrome>
        <main>content</main>
      </elohim-page-chrome>
    `;
    await customElements.whenDefined('elohim-page-chrome');
    const chrome = document.querySelector('elohim-page-chrome')!;
    // The default omnibar is rendered in shadow DOM
    const defaultBar = chrome.shadowRoot!.querySelector('elohim-default-omnibar');
    expect(defaultBar).toBeTruthy();
  });

  it('suppresses default when omnibar slot is filled', async () => {
    document.body.innerHTML = `
      <elohim-page-chrome>
        <div slot="omnibar" id="custom-bar">my custom bar</div>
        <main>content</main>
      </elohim-page-chrome>
    `;
    await customElements.whenDefined('elohim-page-chrome');
    const chrome = document.querySelector('elohim-page-chrome')!;
    // The slot's assignedNodes should be the custom div
    const slot = chrome.shadowRoot!.querySelector('slot[name="omnibar"]') as HTMLSlotElement;
    const assigned = slot.assignedNodes({ flatten: true });
    expect(assigned.some(n => (n as HTMLElement).id === 'custom-bar')).toBe(true);
    // Default omnibar should NOT also render — implementation hides it when slot is filled
    const defaultBar = chrome.shadowRoot!.querySelector('elohim-default-omnibar');
    expect(defaultBar?.hasAttribute('hidden')).toBe(true);
  });
});
```

- [ ] **Step 2: Run test (will fail)**

```bash
pnpm vitest run src/elohim-page-chrome.spec.ts 2>&1 | tail -15
```

Expected: COMPILE FAILURE.

- [ ] **Step 3: Implement page-chrome**

Create `app/elohim-elements/elohim-core/src/elohim-page-chrome.ts`:

```typescript
import { css, html, LitElement } from 'lit';
import { state } from 'lit/decorators.js';

/**
 * <elohim-page-chrome> — the bundle's root wrapper. Provides a slotted
 * omnibar contract: bundles that have their own toolbar place it in
 * slot="omnibar"; bundles that don't get the default automatically.
 *
 * @element elohim-page-chrome
 * @slot omnibar - The omnibar surface; defaults to <elohim-default-omnibar>
 * @slot         - Default slot for page content
 */
export class ElohimPageChrome extends LitElement {
  @state() private hasSlottedOmnibar = false;

  static styles = css`
    :host {
      display: flex;
      flex-direction: column;
      min-height: 100vh;
    }
    .omnibar-host {
      position: sticky;
      top: 0;
      z-index: 10;
    }
    main {
      flex: 1;
    }
  `;

  render() {
    return html`
      <div class="omnibar-host">
        <slot name="omnibar" @slotchange=${this.onOmnibarSlotChange}></slot>
        <elohim-default-omnibar ?hidden=${this.hasSlottedOmnibar}></elohim-default-omnibar>
      </div>
      <main>
        <slot></slot>
      </main>
    `;
  }

  private onOmnibarSlotChange(e: Event) {
    const slot = e.target as HTMLSlotElement;
    const assigned = slot.assignedNodes({ flatten: true })
      .filter(n => n.nodeType === Node.ELEMENT_NODE || (n.textContent?.trim().length ?? 0) > 0);
    this.hasSlottedOmnibar = assigned.length > 0;
  }
}
```

- [ ] **Step 4: Implement default omnibar (minimal)**

Create `app/elohim-elements/elohim-core/src/elohim-default-omnibar.ts`:

```typescript
import { css, html, LitElement } from 'lit';
import { state } from 'lit/decorators.js';
import { Session, type CurrentUserView } from './session/session.js';

/**
 * <elohim-default-omnibar> — the fallback omnibar used when a bundle doesn't
 * BYO. Minimal MVP shape: brand mark + auth indicator + current location.
 *
 * @element elohim-default-omnibar
 */
export class ElohimDefaultOmnibar extends LitElement {
  private session = new Session();
  @state() private user: CurrentUserView | null = this.session.currentUser;
  private unsub?: () => void;

  static styles = css`
    :host {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0.5rem 1rem;
      background: color-mix(in oklch, Canvas 92%, currentColor);
      border-bottom: 1px solid color-mix(in oklch, currentColor 12%, transparent);
    }
    .brand { font-weight: 600; }
    .user { font-size: 0.875rem; opacity: 0.8; }
  `;

  connectedCallback(): void {
    super.connectedCallback();
    this.unsub = this.session.subscribe(u => { this.user = u; });
  }

  disconnectedCallback(): void {
    super.disconnectedCallback();
    this.unsub?.();
  }

  render() {
    return html`
      <span class="brand">elohim.host</span>
      <span class="user">
        ${this.user
          ? html`${this.user.humanId}`
          : html`<a href="/auth/signin">sign in</a>`}
      </span>
    `;
  }
}
```

- [ ] **Step 5: Register both + test passes**

Update `register.ts`:

```typescript
import { ElohimPageChrome } from './elohim-page-chrome.js';
import { ElohimDefaultOmnibar } from './elohim-default-omnibar.js';

if (!customElements.get('elohim-page-chrome')) {
  customElements.define('elohim-page-chrome', ElohimPageChrome);
}
if (!customElements.get('elohim-default-omnibar')) {
  customElements.define('elohim-default-omnibar', ElohimDefaultOmnibar);
}
```

```bash
pnpm vitest run src/elohim-page-chrome.spec.ts 2>&1 | tail -10
```

Expected: both pass.

- [ ] **Step 6: Commit**

```bash
git add app/elohim-elements/elohim-core/src/elohim-page-chrome.ts \
        app/elohim-elements/elohim-core/src/elohim-page-chrome.spec.ts \
        app/elohim-elements/elohim-core/src/elohim-default-omnibar.ts \
        app/elohim-elements/elohim-core/src/register.ts \
        app/elohim-elements/elohim-core/src/index.ts
git commit -m "feat(elohim-core): <elohim-page-chrome> + <elohim-default-omnibar>"
```

---

## Task B7: `<elohim-context-menu>` (MVP — 3 items)

**Files:**
- Create: `app/elohim-elements/elohim-core/src/elohim-context-menu.ts`
- Create: `app/elohim-elements/elohim-core/src/elohim-context-menu.spec.ts`

- [ ] **Step 1: Tests first**

```typescript
// app/elohim-elements/elohim-core/src/elohim-context-menu.spec.ts
import { describe, it, expect, beforeEach } from 'vitest';
import './elohim-context-menu.js';

describe('<elohim-context-menu>', () => {
  beforeEach(() => { document.body.innerHTML = ''; });

  it('renders three MVP items: Open, About this EPR, Copy EPR link', async () => {
    document.body.innerHTML = `
      <elohim-context-menu open
        .items=${JSON.stringify([
          { id: 'open', label: 'Open' },
          { id: 'about', label: 'About this EPR' },
          { id: 'copy', label: 'Copy EPR link' },
        ])}>
      </elohim-context-menu>
    `;
    await customElements.whenDefined('elohim-context-menu');
    const menu = document.querySelector('elohim-context-menu')!;
    const items = menu.shadowRoot!.querySelectorAll('[role="menuitem"]');
    expect(items.length).toBe(3);
    expect(items[0].textContent?.trim()).toBe('Open');
  });

  it('emits item-select event when item is clicked', async () => {
    document.body.innerHTML = `<elohim-context-menu open></elohim-context-menu>`;
    await customElements.whenDefined('elohim-context-menu');
    const menu = document.querySelector('elohim-context-menu')! as any;
    menu.items = [{ id: 'open', label: 'Open' }];
    await menu.updateComplete;

    let received: any = null;
    menu.addEventListener('item-select', (e: CustomEvent) => { received = e.detail; });
    (menu.shadowRoot!.querySelector('[role="menuitem"]') as HTMLElement).click();
    expect(received?.id).toBe('open');
  });

  it('closes on Escape', async () => {
    document.body.innerHTML = `<elohim-context-menu open></elohim-context-menu>`;
    await customElements.whenDefined('elohim-context-menu');
    const menu = document.querySelector('elohim-context-menu')! as any;
    menu.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await menu.updateComplete;
    expect(menu.open).toBe(false);
  });
});
```

- [ ] **Step 2: Implement**

```typescript
// app/elohim-elements/elohim-core/src/elohim-context-menu.ts
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export interface ContextMenuItem {
  id: string;
  label: string;
  disabled?: boolean;
}

/**
 * <elohim-context-menu> — Google Drive-style fold-down menu.
 *
 * MVP: simple flat list of items. Submenu support (View as..., Where this
 * leads...) deferred to fast-follow shift.
 *
 * Accessibility: arrow-key navigation, Enter to select, Escape to close,
 * focus trap while open, ARIA menu role.
 *
 * @element elohim-context-menu
 * @prop {boolean} open - Whether the menu is visible
 * @prop {ContextMenuItem[]} items - The menu items
 * @fires item-select - { id: string } when an item is selected
 * @fires close - When the menu closes
 */
export class ElohimContextMenu extends LitElement {
  @property({ type: Boolean, reflect: true }) open = false;
  @property({ type: Array }) items: ContextMenuItem[] = [];

  static styles = css`
    :host { display: none; }
    :host([open]) {
      display: block;
      position: absolute;
      background: Canvas;
      border: 1px solid color-mix(in oklch, currentColor 15%, transparent);
      border-radius: var(--elohim-radius-sm, 6px);
      box-shadow: 0 4px 12px rgba(0,0,0,0.12);
      min-width: 180px;
      padding: 0.25rem 0;
      animation: fold-down 120ms ease-out;
    }
    @keyframes fold-down {
      from { opacity: 0; transform: translateY(-4px) scaleY(0.95); transform-origin: top; }
      to { opacity: 1; transform: translateY(0) scaleY(1); }
    }
    [role="menu"] { list-style: none; margin: 0; padding: 0; }
    [role="menuitem"] {
      display: block;
      padding: 0.5rem 1rem;
      cursor: pointer;
      user-select: none;
    }
    [role="menuitem"]:hover, [role="menuitem"]:focus {
      background: color-mix(in oklch, currentColor 8%, transparent);
      outline: none;
    }
    [role="menuitem"][aria-disabled="true"] { opacity: 0.5; cursor: default; }
    @media (prefers-reduced-motion: reduce) {
      :host([open]) { animation: none; }
    }
  `;

  connectedCallback(): void {
    super.connectedCallback();
    this.addEventListener('keydown', this.handleKeydown);
    this.setAttribute('role', 'presentation');
  }

  disconnectedCallback(): void {
    super.disconnectedCallback();
    this.removeEventListener('keydown', this.handleKeydown);
  }

  render() {
    return html`
      <ul role="menu">
        ${this.items.map(item => html`
          <li
            role="menuitem"
            tabindex="0"
            aria-disabled=${item.disabled ? 'true' : 'false'}
            @click=${() => this.select(item)}
            @keydown=${(e: KeyboardEvent) => { if (e.key === 'Enter') this.select(item); }}
          >${item.label}</li>
        `)}
      </ul>
    `;
  }

  private handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      this.open = false;
      this.dispatchEvent(new CustomEvent('close'));
    }
    // arrow nav handled by default tabindex flow for MVP simplicity
  };

  private select(item: ContextMenuItem) {
    if (item.disabled) return;
    this.dispatchEvent(new CustomEvent('item-select', { detail: { id: item.id }, bubbles: true }));
    this.open = false;
  }
}
```

- [ ] **Step 3: Register + run tests**

Update `register.ts` and `index.ts`. Run tests, expect all pass.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-elements/elohim-core/src/elohim-context-menu.ts \
        app/elohim-elements/elohim-core/src/elohim-context-menu.spec.ts \
        app/elohim-elements/elohim-core/src/register.ts \
        app/elohim-elements/elohim-core/src/index.ts
git commit -m "feat(elohim-core): <elohim-context-menu> MVP — 3 items, accessible"
```

---

## Task B8: `<elohim-epr-link>` element (HyperCard primitive)

**Files:**
- Create: `app/elohim-elements/elohim-core/src/elohim-epr-link.ts`
- Create: `app/elohim-elements/elohim-core/src/elohim-epr-link.spec.ts`

This is the centerpiece of the elohim-core extract. It uses Loader (B1) + Session (B2) + Skeleton (B4) + Mention-base (B5) + Context-menu (B7).

- [ ] **Step 1: Write the tests covering: rendering variants, progressive loading, default click, right-click menu, unreachable preview**

Create `app/elohim-elements/elohim-core/src/elohim-epr-link.spec.ts`:

```typescript
import { describe, it, expect, beforeEach, vi } from 'vitest';
import './elohim-epr-link.js';
import './elohim-skeleton.js';
import './elohim-mention-base.js';
import './elohim-context-menu.js';

describe('<elohim-epr-link>', () => {
  beforeEach(() => { document.body.innerHTML = ''; });

  it('renders skeleton at L1 (instant, EPR id only)', async () => {
    document.body.innerHTML = `<elohim-epr-link epr="epr:lamad-spa" display="chip"></elohim-epr-link>`;
    await customElements.whenDefined('elohim-epr-link');
    const link = document.querySelector('elohim-epr-link')!;
    expect(link.shadowRoot!.querySelector('elohim-skeleton')).toBeTruthy();
  });

  it('renders chip variant after L2 resolves', async () => {
    document.body.innerHTML = `<elohim-epr-link epr="epr:lamad-spa" display="chip"></elohim-epr-link>`;
    await customElements.whenDefined('elohim-epr-link');
    const link = document.querySelector('elohim-epr-link')! as any;
    // Simulate L2 resolution
    link._resolvedTitle = 'Lamad Learning Platform';
    link._loadLevel = 2;
    await link.updateComplete;
    expect(link.shadowRoot!.textContent).toContain('Lamad Learning Platform');
  });

  it('emits navigate event on single click (default action)', async () => {
    document.body.innerHTML = `<elohim-epr-link epr="epr:lamad-spa" display="chip"></elohim-epr-link>`;
    await customElements.whenDefined('elohim-epr-link');
    const link = document.querySelector('elohim-epr-link')! as any;
    let navigated: any = null;
    link.addEventListener('navigate', (e: CustomEvent) => { navigated = e.detail; });
    link.shadowRoot!.querySelector('button, a, [role="link"]')?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(navigated?.epr).toBe('epr:lamad-spa');
  });

  it('opens context menu on right-click', async () => {
    document.body.innerHTML = `<elohim-epr-link epr="epr:lamad-spa" display="chip"></elohim-epr-link>`;
    await customElements.whenDefined('elohim-epr-link');
    const link = document.querySelector('elohim-epr-link')! as any;
    link.shadowRoot!.querySelector('button, a, [role="link"]')?.dispatchEvent(
      new MouseEvent('contextmenu', { bubbles: true })
    );
    await link.updateComplete;
    const menu = link.shadowRoot!.querySelector('elohim-context-menu') as any;
    expect(menu.open).toBe(true);
  });

  it('falls back to <elohim-mention-base> when target is unreachable', async () => {
    document.body.innerHTML = `<elohim-epr-link epr="epr:nonexistent" display="chip"></elohim-epr-link>`;
    await customElements.whenDefined('elohim-epr-link');
    const link = document.querySelector('elohim-epr-link')! as any;
    link._loadLevel = 4;  // simulate L4 (preview fallback)
    link._unreachable = true;
    await link.updateComplete;
    expect(link.shadowRoot!.querySelector('elohim-mention-base')).toBeTruthy();
  });
});
```

- [ ] **Step 2: Run to verify failure**

```bash
pnpm vitest run src/elohim-epr-link.spec.ts 2>&1 | tail -15
```

Expected: FAIL.

- [ ] **Step 3: Implement the element**

Create `app/elohim-elements/elohim-core/src/elohim-epr-link.ts`:

```typescript
import { css, html, LitElement } from 'lit';
import { property, state } from 'lit/decorators.js';
import './elohim-skeleton.js';
import './elohim-mention-base.js';
import './elohim-context-menu.js';

export type EprLinkDisplay = 'inline' | 'chip' | 'card' | 'popover';

/**
 * <elohim-epr-link> — the protocol's HyperCard navigation primitive.
 *
 * Progressive loading (L1–L4): renders a sized skeleton instantly, fills
 * in title/metadata as Loader resolves, falls back to preview if the
 * target is unreachable.
 *
 * Default click: emits 'navigate' with the EPR ref. Parent decides what
 * to do (card-flip in place, or hard nav, depending on display variant).
 *
 * Right-click: opens <elohim-context-menu> with MVP items (Open / About /
 * Copy EPR link). Submenu items (View as..., Where this leads...) deferred.
 *
 * @element elohim-epr-link
 * @prop {string} epr - The epr:... reference
 * @prop {EprLinkDisplay} display - Visual variant
 * @fires navigate - { epr: string } on default-click activation
 */
export class ElohimEprLink extends LitElement {
  @property() epr = '';
  @property() display: EprLinkDisplay = 'inline';

  @state() private _loadLevel = 1;  // 1=skeleton, 2=title+badge, 3=full, 4=preview-fallback
  @state() private _resolvedTitle = '';
  @state() private _unreachable = false;
  @state() private _menuOpen = false;

  private menuItems = [
    { id: 'open', label: 'Open' },
    { id: 'about', label: 'About this EPR' },
    { id: 'copy', label: 'Copy EPR link' },
  ];

  static styles = css`
    :host { display: inline-block; }
    [role="link"] {
      display: inline-flex; align-items: center; gap: 0.25rem;
      padding: 0.125rem 0.5rem;
      border: 1px solid color-mix(in oklch, currentColor 25%, transparent);
      border-radius: 999px;
      cursor: pointer;
      background: none;
      color: inherit;
      font: inherit;
    }
    [role="link"]:hover { background: color-mix(in oklch, currentColor 6%, transparent); }
    .menu-anchor { position: relative; }
  `;

  connectedCallback(): void {
    super.connectedCallback();
    // Schedule L2 resolution. In MVP this would use the Loader to fetch
    // metadata via /api/v1/epr/{id}. Stubbed here; real wiring happens
    // when consumed by a bundle.
    queueMicrotask(() => this.resolveL2());
  }

  render() {
    if (this._loadLevel === 1) {
      return html`<elohim-skeleton width="6rem" height="1rem"></elohim-skeleton>`;
    }
    if (this._loadLevel === 4 && this._unreachable) {
      return html`
        <span class="menu-anchor">
          <elohim-mention-base epr=${this.epr} title=${this._resolvedTitle}></elohim-mention-base>
          ${this.renderContextMenu()}
        </span>
      `;
    }
    return html`
      <span class="menu-anchor">
        <button
          role="link"
          @click=${this.handleClick}
          @contextmenu=${this.handleContextMenu}
        >${this._resolvedTitle || this.epr}</button>
        ${this.renderContextMenu()}
      </span>
    `;
  }

  private renderContextMenu() {
    return html`
      <elohim-context-menu
        ?open=${this._menuOpen}
        .items=${this.menuItems}
        @item-select=${this.handleMenuSelect}
        @close=${() => this._menuOpen = false}
      ></elohim-context-menu>
    `;
  }

  private handleClick = (e: MouseEvent) => {
    e.preventDefault();
    this.dispatchEvent(new CustomEvent('navigate', { detail: { epr: this.epr }, bubbles: true }));
  };

  private handleContextMenu = (e: MouseEvent) => {
    e.preventDefault();
    this._menuOpen = true;
  };

  private handleMenuSelect = (e: CustomEvent) => {
    const id = e.detail.id;
    if (id === 'open') {
      this.dispatchEvent(new CustomEvent('navigate', { detail: { epr: this.epr }, bubbles: true }));
    } else if (id === 'copy') {
      navigator.clipboard?.writeText(this.epr).catch(() => {});
    } else if (id === 'about') {
      this.dispatchEvent(new CustomEvent('about', { detail: { epr: this.epr }, bubbles: true }));
    }
  };

  // Stub: real implementation uses Loader to resolve EPR head via
  // /api/v1/epr/{id} on the current origin's doorway. Production binds
  // this to the bundle's Loader instance.
  private async resolveL2(): Promise<void> {
    // Placeholder — bundle integration overrides this.
    this._loadLevel = 2;
  }
}
```

- [ ] **Step 4: Register + index export + run tests**

```bash
pnpm vitest run src/elohim-epr-link.spec.ts 2>&1 | tail -10
```

Expected: all 5 pass.

- [ ] **Step 5: Commit**

```bash
git add app/elohim-elements/elohim-core/src/elohim-epr-link.ts \
        app/elohim-elements/elohim-core/src/elohim-epr-link.spec.ts \
        app/elohim-elements/elohim-core/src/register.ts \
        app/elohim-elements/elohim-core/src/index.ts
git commit -m "feat(elohim-core): <elohim-epr-link> HyperCard primitive with progressive loading + context menu"
```

---

## Task B9: Library A default stories for new primitives

**Files:**
- Create one story file per new primitive in `app/elohim-library/projects/graphos/src/default/core/__docs__/`

- [ ] **Step 1: Look at existing default story pattern**

```bash
ls app/elohim-library/projects/graphos/src/default/core/__docs__/ 2>/dev/null
cat app/elohim-library/projects/graphos/src/default/core/__docs__/*.default.stories.ts 2>/dev/null | head -60
```

- [ ] **Step 2: Author six default stories** (one per: elohim-skeleton, elohim-mention-base, elohim-page-chrome, elohim-default-omnibar, elohim-context-menu, elohim-epr-link)

Pattern for each — `<element>.default.stories.ts` includes:
- `Unstyled (blank-slate proof)` story wrapped in `style="all: initial;"`
- `CustomTheme` story with a deliberately non-Elohim theme binding
- Lens-coverage stories per the element's claimed `@capability*` tags

(Dispatch to component-architect agent if available; otherwise author manually following the elohim-button.default.stories.ts pattern)

- [ ] **Step 3: Verify Storybook builds**

```bash
cd app/elohim-library
pnpm exec ng run graphos:storybook 2>&1 | tail -10 &
# Wait for "Storybook started" then kill the background process
```

Expected: stories appear under `Default/Core/...` in the Storybook UI.

- [ ] **Step 4: Commit each story as a separate commit (granular history)**

```bash
git add app/elohim-library/projects/graphos/src/default/core/__docs__/elohim-skeleton.default.stories.ts
git commit -m "docs(stories): Library A default for elohim-skeleton"
# ... repeat for each ...
```

---

## Task B10: Library B designed stories for new primitives

**Files:**
- Create one story file per new primitive in `app/elohim-library/projects/graphos/src/designed/core/__docs__/`

(Dispatch to graphos-designer agent; binds the Elohim brand tokens via story-decorator overrides per the elohim-library CLAUDE.md conventions.)

- [ ] **Step 1: Author six designed stories** (parallel to Task B9 files)

- [ ] **Step 2: Verify in Storybook UI**

- [ ] **Step 3: Commit each as a separate commit**

```bash
git add app/elohim-library/projects/graphos/src/designed/core/__docs__/elohim-skeleton.designed.stories.ts
git commit -m "docs(stories): Library B designed for elohim-skeleton (brand-bound)"
# ... repeat ...
```

---

## Task B11: Doorway epr_router module

**Files:**
- Create: `doorway/doorway-service/src/projection/epr_router.rs`
- Modify: `doorway/doorway-service/src/projection/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `doorway/doorway-service/src/projection/epr_router.rs` with the test module first:

```rust
//! EPR Router — consults active project-epr commitments on this doorway
//! and dispatches incoming URLs to the right EPR by longest-prefix match.

use std::collections::BTreeMap;
use std::sync::RwLock;

// Re-export the view type from the storage client for clarity
pub use elohim_storage_client::generated::EprProjectionView;

pub struct EprRouter {
    /// urlPath -> projection. Sorted naturally; longest-prefix lookup
    /// iterates from longest to shortest.
    table: RwLock<BTreeMap<String, EprProjectionView>>,
}

impl EprRouter {
    pub fn new() -> Self {
        Self { table: RwLock::new(BTreeMap::new()) }
    }

    pub fn replace_all(&self, projections: Vec<EprProjectionView>) {
        let mut table = self.table.write().unwrap();
        table.clear();
        for p in projections {
            table.insert(p.url_path.clone(), p);
        }
    }

    pub fn dispatch(&self, request_path: &str) -> Option<EprProjectionView> {
        let table = self.table.read().unwrap();
        // BTreeMap sorts ascending; reverse to get longest first
        let mut candidates: Vec<&EprProjectionView> = table.values().collect();
        candidates.sort_by_key(|p| std::cmp::Reverse(p.url_path.len()));

        for p in candidates {
            if Self::path_matches_prefix(request_path, &p.url_path) {
                return Some(p.clone());
            }
        }
        None
    }

    fn path_matches_prefix(request_path: &str, projection_path: &str) -> bool {
        if projection_path == "/" {
            return true;
        }
        request_path == projection_path
            || request_path.starts_with(&format!("{}/", projection_path))
    }
}

impl Default for EprRouter {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_projection(epr_id: &str, url_path: &str) -> EprProjectionView {
        EprProjectionView {
            commitment_id: format!("test-{}", epr_id),
            epr_id: epr_id.into(),
            doorway_id: "doorway:test".into(),
            url_path: url_path.into(),
            mode: elohim_storage_client::generated::ProjectionMode::Cached,
            reach: "commons".into(),
            base_href: format!("{}/", if url_path == "/" { "" } else { url_path }),
            entry_file: "index.html".into(),
            redirects_from: vec![],
            preview_epr_ref: None,
            gate_hints: vec![],
            dead_end: false,
            steward_direct_endpoint: None,
            seeded_at: "2026-05-25T00:00:00Z".into(),
            seeded_by: "test".into(),
        }
    }

    #[test]
    fn dispatch_returns_none_for_empty_router() {
        let router = EprRouter::new();
        assert!(router.dispatch("/anything").is_none());
    }

    #[test]
    fn dispatch_returns_landing_for_root() {
        let router = EprRouter::new();
        router.replace_all(vec![make_projection("landing", "/")]);
        assert_eq!(router.dispatch("/").unwrap().epr_id, "landing");
        assert_eq!(router.dispatch("/anything").unwrap().epr_id, "landing");
    }

    #[test]
    fn dispatch_longest_prefix_wins() {
        let router = EprRouter::new();
        router.replace_all(vec![
            make_projection("landing", "/"),
            make_projection("lamad", "/lamad"),
        ]);
        assert_eq!(router.dispatch("/").unwrap().epr_id, "landing");
        assert_eq!(router.dispatch("/lamad").unwrap().epr_id, "lamad");
        assert_eq!(router.dispatch("/lamad/concept/x").unwrap().epr_id, "lamad");
        assert_eq!(router.dispatch("/other").unwrap().epr_id, "landing");
    }

    #[test]
    fn dispatch_does_not_match_partial_segment() {
        let router = EprRouter::new();
        router.replace_all(vec![make_projection("lamad", "/lamad")]);
        assert!(router.dispatch("/lamadx").is_none());
        assert!(router.dispatch("/lamadextra").is_none());
    }

    #[test]
    fn replace_all_drops_previous_state() {
        let router = EprRouter::new();
        router.replace_all(vec![make_projection("a", "/a")]);
        router.replace_all(vec![make_projection("b", "/b")]);
        assert!(router.dispatch("/a").is_none());
        assert_eq!(router.dispatch("/b").unwrap().epr_id, "b");
    }
}
```

- [ ] **Step 2: Wire into projection/mod.rs**

```rust
// In doorway/doorway-service/src/projection/mod.rs
pub mod epr_router;
pub use epr_router::EprRouter;
```

- [ ] **Step 3: Run tests**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo test --lib epr_router 2>&1 | tail -15
```

Expected: 5 tests pass.

- [ ] **Step 4: Commit**

```bash
git add doorway/doorway-service/src/projection/
git commit -m "feat(doorway): epr_router module with longest-prefix dispatch"
```

---

## Task B12: Load projections at boot

**Files:**
- Modify: `doorway/doorway-service/src/main.rs`

- [ ] **Step 1: Find where doorway makes its initial storage requests**

```bash
grep -n "warm_stream\|load_at_boot\|initial.*projection\|find_active_operator" doorway/doorway-service/src/main.rs 2>/dev/null | head -5
```

- [ ] **Step 2: Add an initial projection load**

In `main.rs`, near where `warm_stream` runs (cold-start projection snapshot), add:

```rust
// Load EPR projections scoped to this doorway and seed the router
let projections = match storage_client.list_projections(&doorway_id).await {
    Ok(p) => p,
    Err(e) => {
        warn!("Could not load EPR projections at boot: {}. Router starts empty.", e);
        vec![]
    }
};
info!("Loaded {} EPR projections for {}", projections.len(), doorway_id);
let epr_router = Arc::new(EprRouter::new());
epr_router.replace_all(projections);
```

(Adapt to the actual storage_client API; you may need to add a `list_projections` method on the client wrapper.)

- [ ] **Step 3: Build + run smoke test**

```bash
RUSTFLAGS="" cargo build 2>&1 | tail -10
```

Expected: compiles.

- [ ] **Step 4: Commit**

```bash
git add doorway/doorway-service/src/main.rs
git commit -m "feat(doorway): load EPR projections at boot, seed router"
```

---

## Task B13: Wire epr_router into request dispatch

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs`

- [ ] **Step 1: Find the request entry point**

```bash
grep -n "fn handle_request\|async fn handle\|service_fn" doorway/doorway-service/src/server/http.rs | head -5
```

- [ ] **Step 2: Insert router consultation at request entry**

At the request entry function, before falling through to existing routes:

```rust
// Consult EPR router first — projection-driven dispatch
if let Some(projection) = state.epr_router.dispatch(req.uri().path()) {
    return dispatch_to_projected_epr(state, req, projection).await;
}
// Fall through to existing routes
```

Implement `dispatch_to_projected_epr`:

```rust
async fn dispatch_to_projected_epr(
    state: &AppState,
    req: Request<Incoming>,
    projection: EprProjectionView,
) -> Result<Response<Full<Bytes>>, Infallible> {
    // For MVP: reach=commons → serve directly via storage's /apps/{slug}
    // (storage already knows how to serve bundle files; we proxy with
    // the projection's epr_id as the slug)
    if projection.reach != "commons" {
        return Ok(Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Full::new(Bytes::from("Reach-gated EPRs not yet supported")))
            .unwrap());
    }

    // Strip the urlPath prefix to get the sub-path within the bundle
    let request_path = req.uri().path();
    let sub_path = if projection.url_path == "/" {
        request_path.trim_start_matches('/')
    } else {
        request_path.trim_start_matches(&projection.url_path).trim_start_matches('/')
    };
    let sub_path = if sub_path.is_empty() { &projection.entry_file } else { sub_path };

    // Proxy to storage's /apps/{slug}/{sub_path}
    let storage_url = format!("{}/apps/{}/{}", state.storage_url, projection.epr_id, sub_path);
    proxy_to_storage(state, req, storage_url).await
}
```

(Adapt to actual function signatures in your http.rs.)

- [ ] **Step 3: Add integration test**

```rust
#[tokio::test]
async fn projection_dispatch_serves_landing_at_root() {
    let state = test_state_with_projections(vec![
        make_test_projection("elohim-host-landing", "/"),
    ]);
    let response = handle_request_test_helper(&state, "GET", "/", "").await;
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn projection_dispatch_serves_lamad_at_subpath() {
    let state = test_state_with_projections(vec![
        make_test_projection("lamad-spa", "/lamad"),
    ]);
    let response = handle_request_test_helper(&state, "GET", "/lamad/concept/x", "").await;
    assert_eq!(response.status(), 200);
}
```

- [ ] **Step 4: Run + commit**

```bash
RUSTFLAGS="" cargo test --lib projection_dispatch 2>&1 | tail -10
git add doorway/doorway-service/src/server/http.rs
git commit -m "feat(doorway): consult epr_router for projection-driven URL dispatch"
```

---

## Task B14: Drop ROOT_APP_SLUG config

**Files:**
- Modify: `doorway/doorway-service/src/config.rs`
- Modify: `doorway/doorway-service/src/main.rs`
- Modify: `genesis/orchestrator/manifests/doorway/alpha.yaml`
- Modify: `genesis/orchestrator/manifests/doorway/alpha-b.yaml`

- [ ] **Step 1: Remove from config struct**

```rust
// Delete this line from config.rs:
#[arg(long, env = "ROOT_APP_SLUG")]
pub root_app_slug: Option<String>,
```

- [ ] **Step 2: Remove usages**

```bash
grep -rn "root_app_slug\|ROOT_APP_SLUG" doorway/doorway-service/src/ 2>/dev/null
```

Delete each usage. The router replaces this.

- [ ] **Step 3: Remove from k8s manifests**

In `genesis/orchestrator/manifests/doorway/alpha.yaml` and `alpha-b.yaml`, delete the `ROOT_APP_SLUG` env entries.

- [ ] **Step 4: Build + commit**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo build 2>&1 | tail -5
git add doorway/doorway-service/src/config.rs doorway/doorway-service/src/main.rs \
        genesis/orchestrator/manifests/doorway/alpha.yaml genesis/orchestrator/manifests/doorway/alpha-b.yaml
git commit -m "refactor(doorway): drop ROOT_APP_SLUG (replaced by projection at urlPath=/)"
```

---

## Task B15: Wire projection events into storage_events_subscriber

**Files:**
- Modify: `doorway/doorway-service/src/projection/storage_events_subscriber.rs`

- [ ] **Step 1: Extend the event-kind matcher**

In the SSE subscriber's event handler, add cases for `projection.registered` and `projection.revoked`:

```rust
match event_kind.as_str() {
    "content.created" | "content.updated" | "content.deleted" => {
        // existing handler
    }
    "projection.registered" | "projection.revoked" => {
        // Re-fetch the doorway's full projection set and replace the router state
        if let Ok(projections) = self.storage_client.list_projections(&self.doorway_id).await {
            self.epr_router.replace_all(projections);
            info!("Refreshed epr_router after {} ({} projections)", event_kind, /* count */);
        }
    }
    _ => {}
}
```

- [ ] **Step 2: Add integration test (uses local SSE bus mock)**

(Adapt to existing test infrastructure; verify a fake `projection.registered` event triggers a call to list_projections + replace_all)

- [ ] **Step 3: Commit**

```bash
git add doorway/doorway-service/src/projection/storage_events_subscriber.rs
git commit -m "feat(doorway): refresh epr_router on projection.{registered,revoked} SSE events"
```

---

## Task B16: New /api/v1/epr/{id} route (HyperCard resolution endpoint)

**Files:**
- Create: `doorway/doorway-service/src/routes/epr.rs`
- Modify: `doorway/doorway-service/src/routes/mod.rs`

- [ ] **Step 1: Write the test**

```rust
#[tokio::test]
async fn api_v1_epr_returns_resolved_epr_head() {
    let state = test_state();
    seed_test_epr(&state, "test-epr", "Test Title").await;
    let response = handle_request_test_helper(&state, "GET", "/api/v1/epr/test-epr", "").await;
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = serde_json::from_slice(&response_body_bytes(response).await).unwrap();
    assert_eq!(body["eprId"], "test-epr");
    assert!(body["title"].is_string());
}
```

- [ ] **Step 2: Implement the route**

Create `doorway/doorway-service/src/routes/epr.rs`:

```rust
//! GET /api/v1/epr/{id} — HyperCard resolution endpoint.
//!
//! Returns the EPR head + bundle reference for inline rendering by
//! <elohim-epr-link> in a mounted bundle. No bundle bytes; this is
//! metadata-only.

use crate::error::DoorwayError;

pub async fn handle_epr_resolve(
    state: &AppState,
    epr_id: &str,
) -> Result<Response<Full<Bytes>>, DoorwayError> {
    // Fetch from storage's /db/content/{id} — already in projection cache
    let content = state.storage_client.get_content(epr_id).await
        .map_err(|e| DoorwayError::Internal(format!("storage fetch: {}", e)))?;

    let Some(content) = content else {
        return Ok(json_response(404, &serde_json::json!({
            "error": "EPR not found",
            "eprId": epr_id,
        })));
    };

    // Wrap in EprResolution shape
    let resolution = serde_json::json!({
        "eprId": content.id,
        "title": content.title,
        "description": content.description,
        "contentType": content.content_type,
        "contentFormat": content.content_format,
        "reach": content.reach,
        "blobHash": content.blob_hash,
        "metadata": content.metadata,
    });

    Ok(json_response(200, &resolution))
}
```

Wire into the routes/mod.rs and the request dispatcher.

- [ ] **Step 3: Run test + commit**

```bash
RUSTFLAGS="" cargo test --lib api_v1_epr 2>&1 | tail -10
git add doorway/doorway-service/src/routes/epr.rs doorway/doorway-service/src/routes/mod.rs
git commit -m "feat(doorway): GET /api/v1/epr/{id} HyperCard resolution endpoint"
```

---

## Task B17: Scaffold app/lamad Angular project

**Files:**
- Create: `app/lamad/angular.json`
- Create: `app/lamad/package.json`
- Create: `app/lamad/tsconfig.json`
- Create: `app/lamad/tsconfig.app.json`
- Modify: `pnpm-workspace.yaml`

- [ ] **Step 1: Look at app/elohim-app's config as the template**

```bash
cat app/elohim-app/angular.json | head -60
cat app/elohim-app/package.json | head -40
cat app/elohim-app/tsconfig.json
cat app/elohim-app/tsconfig.app.json
```

- [ ] **Step 2: Add app/lamad to pnpm workspace**

In `pnpm-workspace.yaml`:

```yaml
packages:
  - app/elohim-app
  - app/elohim-elements/*
  - app/elohim-library
  - app/lamad     # NEW
  # ... other entries
```

- [ ] **Step 3: Create app/lamad/package.json**

Mirror elohim-app's package.json but with name="lamad", and minimal dependencies (the lamad pillar's own deps + elohim-core).

- [ ] **Step 4: Create app/lamad/angular.json**

Mirror elohim-app's but with `projectType: "application"`, `root: ""` relative to app/lamad, outputPath: `dist/lamad`, base href baked into `src/index.html` as `<base href="/lamad/">`.

- [ ] **Step 5: Create app/lamad/tsconfig.json + tsconfig.app.json**

Mirror elohim-app's; ensure paths include any elohim-app types that are still imported.

- [ ] **Step 6: pnpm install**

```bash
cd /projects/elohim
pnpm install 2>&1 | tail -10
```

Expected: succeeds; app/lamad recognized as a workspace.

- [ ] **Step 7: Commit**

```bash
git add app/lamad/angular.json app/lamad/package.json app/lamad/tsconfig.json app/lamad/tsconfig.app.json pnpm-workspace.yaml pnpm-lock.yaml
git commit -m "feat(lamad): scaffold app/lamad Angular project (empty)"
```

---

## Task B18: Move lamad pillar code into app/lamad

**Files:**
- Move: `app/elohim-app/src/app/lamad/**` → `app/lamad/src/app/**`
- Delete: original location
- Modify: imports throughout

- [ ] **Step 1: Use git mv for history preservation**

```bash
mkdir -p app/lamad/src/app
git mv app/elohim-app/src/app/lamad/* app/lamad/src/app/
```

- [ ] **Step 2: Update imports inside the moved files**

The audit from Task B0 lists every `from '@app/elohim'` or relative import that needs adjustment. For each:
- If the import is from elohim-core's substrate (Loader, Session, EPR-link): update to `from 'elohim-core'`
- If from elohim-app (services that stay there): consume via HTTP API (no direct import across bundles)
- If from another pillar (qahal, shefa, etc.): consume via HTTP API

This is the substantive task. Work file by file; commit per logical group.

- [ ] **Step 3: Create app/lamad/src/index.html with the correct base href**

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>Lamad — Learning</title>
  <base href="/lamad/">
  <!-- ... -->
</head>
<body>
  <elohim-page-chrome>
    <lamad-toolbar slot="omnibar"></lamad-toolbar>
    <lamad-root></lamad-root>
  </elohim-page-chrome>
  <script type="module" src="main.ts"></script>
</body>
</html>
```

- [ ] **Step 4: Create app/lamad/src/main.ts**

```typescript
import { bootstrapApplication } from '@angular/platform-browser';
import { AppComponent } from './app/app.component';
import { appConfig } from './app/app.config';
import 'elohim-core/register';  // Side-effect: registers custom elements

bootstrapApplication(AppComponent, appConfig).catch(err => console.error(err));
```

- [ ] **Step 5: Build lamad bundle**

```bash
cd app/lamad
pnpm run build 2>&1 | tail -10
```

Expected: succeeds. dist/lamad/browser/ contains index.html with <base href="/lamad/">.

- [ ] **Step 6: Wrap lamad's existing toolbar as the omnibar slot consumer**

Locate the lamad toolbar component (from audit). Either wrap it as a custom element (if it's pure Angular) or modify its host template to position via `slot="omnibar"`.

- [ ] **Step 7: Commit (multiple commits — one per dependency group)**

```bash
git add app/lamad/src/
git commit -m "feat(lamad): move pillar code to app/lamad (own bundle, base href /lamad/)"
```

---

## Task B19: Remove /lamad from elohim-app routes

**Files:**
- Modify: `app/elohim-app/src/app/app.routes.ts`

- [ ] **Step 1: Delete the /lamad route block**

In `app/elohim-app/src/app/app.routes.ts`, remove:

```typescript
{
  path: 'lamad',
  loadChildren: async () => import('./lamad/lamad.routes').then(m => m.LAMAD_ROUTES),
},
```

- [ ] **Step 2: Delete the old app/elohim-app/src/app/lamad/ directory**

```bash
rm -rf app/elohim-app/src/app/lamad/
```

- [ ] **Step 3: Build elohim-app**

```bash
cd app/elohim-app
pnpm run build 2>&1 | tail -10
```

Expected: succeeds (no lamad imports remain).

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/app.routes.ts
git rm -r app/elohim-app/src/app/lamad/
git commit -m "refactor(elohim-app): remove /lamad subtree (migrated to app/lamad bundle)"
```

---

## Task B20: Angular wrapper for elohim-epr-link in remaining monolith

**Files:**
- Modify: `app/elohim-app/src/app/elohim/components/epr-link/epr-link.component.ts`

- [ ] **Step 1: Grep to confirm wrapper is still needed**

```bash
grep -rn "app-epr-link\|<app-epr-link" app/elohim-app/src/ 2>/dev/null | grep -v ".spec.ts" | head -10
```

If empty → skip this task entirely. The monolith no longer uses EPR-link.

If non-empty → proceed.

- [ ] **Step 2: Replace the component with a thin wrapper around the Lit element**

```typescript
import { Component, ElementRef, Input, OnChanges } from '@angular/core';
import 'elohim-core/register';

@Component({
  selector: 'app-epr-link',
  standalone: true,
  template: '<elohim-epr-link [attr.epr]="epr" [attr.display]="display"></elohim-epr-link>',
})
export class EprLinkComponent {
  @Input() epr = '';
  @Input() display = 'inline';
}
```

- [ ] **Step 3: Verify build**

```bash
cd app/elohim-app && pnpm run build 2>&1 | tail -5
```

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/epr-link/epr-link.component.ts
git commit -m "refactor(elohim-app): epr-link Angular wrapper delegates to elohim-core Lit element"
```

---

## Task B21: Extend Jenkinsfile to build + upload + patch two bundles

**Files:**
- Modify: `Jenkinsfile`

- [ ] **Step 1: Review the existing stageSpaBlob function**

```bash
grep -n "def stageSpaBlob\|stageSpaBlob(" Jenkinsfile | head -5
```

- [ ] **Step 2: Refactor stageSpaBlob to accept a list of (bundle, slug) pairs**

```groovy
def stageSpaBlobs(String doorwayEprUrl, List<Map> bundles, String adminKey) {
  // bundles = [
  //   [distDir: "dist/elohim-app/browser", slug: "elohim-host-landing"],
  //   [distDir: "dist/lamad/browser",      slug: "lamad-spa"],
  // ]
  for (bundle in bundles) {
    // For each: zip, sha256, upload, patch+verify
    // (mostly the existing logic, parameterized)
  }
}
```

- [ ] **Step 3: Add lamad build step before stageSpaBlobs**

```groovy
stage('Build lamad bundle') {
  steps {
    sh '''
      cd app/lamad
      pnpm run build
    '''
  }
}

// Then stageSpaBlobs called with both bundles
```

- [ ] **Step 4: Local syntax check**

```bash
grep -n "stageSpaBlob\|stageSpaBlobs" Jenkinsfile
```

(Full validation runs in CI.)

- [ ] **Step 5: Commit**

```bash
git add Jenkinsfile
git commit -m "feat(ci): build + upload + patch two bundles (elohim-app + lamad) [build:app]"
```

---

## Task B22: a2o feature — native-epr-projection

**Files:**
- Create: `genesis/a2o/features/doorway/native-epr-projection.feature`
- Create: `genesis/a2o/steps/doorway/native-epr-projection.steps.ts`

- [ ] **Step 1: Write the feature file (per spec §9.6)**

Paste the exact Gherkin from spec §9.6 into the new file.

- [ ] **Step 2: Implement step definitions**

```typescript
import { Given, When, Then } from '@cucumber/cucumber';
import { expect } from 'chai';
// ... import existing a2o framework

Given('the alpha.elohim.host doorway has an active project-epr commitment for {string} at urlPath {string}',
  async function (eprId: string, urlPath: string) {
    // Seed or verify the commitment exists
    const response = await this.doorwayClient.fetch(
      `/db/rea_commitments?action=project-epr&doorwayId=alpha-elohim-host`
    );
    const projections = await response.json();
    const found = projections.find((p: any) => p.eprId === eprId && p.urlPath === urlPath);
    expect(found).to.exist;
  }
);

// ... implement remaining steps from the feature file
```

- [ ] **Step 3: Run a2o for this feature**

```bash
cd genesis/a2o
pnpm run test:features features/doorway/native-epr-projection.feature 2>&1 | tail -20
```

Expected: passes.

- [ ] **Step 4: Commit**

```bash
git add genesis/a2o/features/doorway/native-epr-projection.feature \
        genesis/a2o/steps/doorway/native-epr-projection.steps.ts
git commit -m "test(a2o): native-epr-projection scenarios"
```

---

## Task B23: a2o feature — epr-link-hypercard

**Files:**
- Create: `genesis/a2o/features/elohim-core/epr-link-hypercard.feature`
- Create: `genesis/a2o/steps/elohim-core/epr-link-hypercard.steps.ts`

- [ ] **Step 1: Write the feature file**

Copy from spec §9.6.

- [ ] **Step 2: Implement step definitions**

Use Cypress + Cucumber adapters to:
- Navigate to /lamad/concept/fair-exchange in a browser
- Verify <elohim-epr-link> components render
- Click them and assert no navigation occurred
- Right-click and assert context menu appears

- [ ] **Step 3: Run**

```bash
cd genesis/a2o
pnpm run test:features features/elohim-core/epr-link-hypercard.feature 2>&1 | tail -20
```

- [ ] **Step 4: Commit**

```bash
git add genesis/a2o/features/elohim-core/epr-link-hypercard.feature \
        genesis/a2o/steps/elohim-core/epr-link-hypercard.steps.ts
git commit -m "test(a2o): epr-link-hypercard scenarios"
```

---

## Phase B Checkpoint — Definition of Done verification

- [ ] **Run all Phase B tests**

```bash
cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib
cd ../../app/elohim-elements/elohim-core && pnpm vitest run
cd ../../lamad && pnpm test
cd ../../genesis/a2o && pnpm run test:features features/doorway/ features/elohim-core/
```

Expected: all green.

- [ ] **Reseed a dev environment, verify 4 commitments + 1 registry**

```bash
pnpm run hc:start:seed  # if it includes the new seed-projections
# OR
cd genesis/seeder && DOORWAY_URL=http://localhost:8888 pnpm exec tsx src/seed-projections.ts
# Verify:
curl http://localhost:8888/db/rea_commitments?action=project-epr | jq 'length'
# Expected: 4
curl http://localhost:8888/db/content/elohim-core-elements | jq '.id'
# Expected: "elohim-core-elements"
```

- [ ] **Verify ROOT_APP_SLUG fully removed**

```bash
grep -rn "ROOT_APP_SLUG\|root_app_slug" doorway/ genesis/orchestrator/manifests/ 2>/dev/null
```

Expected: empty.

- [ ] **Manual dogfood smoke** (operator-of-record signoff)

After deploying:
- `https://alpha.elohim.host/` loads the landing
- `https://alpha.elohim.host/lamad/` loads the lamad app
- View source: `<base href="/lamad/">` present in lamad's index.html
- Click an EPR-link in lamad → card flips, no shell reload
- Right-click → context menu appears with Open/About/Copy
- Redeploy lamad bundle → fresh bytes appear without manual cache clear

- [ ] **Tag MVP complete**

```bash
git tag mvp-pillar-epr-decomposition
```

---

# Self-Review

## Spec coverage

Walking through spec §8.1 (MVP scope) and confirming each item has a task:

| Spec item | Task |
|---|---|
| project-epr REA action constant | A1 ✓ |
| EprProjectionView + GateHintRef + StewardDirectEndpoint types | A2 ✓ |
| JSON schema + schema contract test | A4, A5 ✓ |
| Validator | A6 ✓ |
| find_active_projections + find_projection_by_url_path | A7 ✓ |
| ProjectionRegistered/Revoked events | A8 ✓ |
| seed-projections.ts | A11 ✓ |
| 4 default projections seed | A11 + A12 ✓ |
| elohim-core-elements registry seed | A13 ✓ |
| epr_router module | B11 ✓ |
| EPR resolution endpoint /api/v1/epr/{id} | B16 ✓ |
| Drop ROOT_APP_SLUG | B14 ✓ |
| Cache eviction on projection events | B15 ✓ |
| Lamad new Angular project | B17 ✓ |
| Pillar code move | B18 ✓ |
| Toolbar slot integration | B18 (step 6) ✓ |
| /lamad subtree removed | B19 ✓ |
| Loader, Session, EPR-link, page-chrome, default-omnibar, skeleton, mention-base, context-menu | B1-B8 ✓ |
| Library A + B stories | B9, B10 ✓ |
| Jenkinsfile two-bundle build | B21 ✓ |
| Two a2o feature files | B22, B23 ✓ |

ElementRegistryView types: A9 ✓
Element-registry-manifest format: A10 ✓

All MVP items covered.

## Placeholder scan

No "TBD" / "TODO (in plan)" / "implement later" found. The plan specifies code or explicit fall-through for every step.

Areas where the plan delegates to existing-pattern-following:
- Task B9/B10 (Library stories) — "follow the elohim-button.default.stories.ts pattern" — pattern is canonical and well-established
- Task B17 step 4 (angular.json setup) — "mirror elohim-app's" — concrete pattern to copy
- Task B18 step 2 (import updates) — driven by Task B0 audit output, which is part of Phase B's first task

## Type consistency

- `EprProjectionView` fields: consistent across Tasks A2, A4, A5, A7, A11
- `PROJECT_EPR_ACTION = "project-epr"` constant: same value in A1, A7, A11
- `ProjectionMode` enum variants: Cached, StewardDirect (camelCase serialization) — consistent
- `GateHintRelation` variants: matched across Rust enum (A2), JSON schema (A4), TS types (A3 generated), seeder types (A11)
- `find_active_projections(conn, ctx, doorway_id)` signature consistent A7 → B11 consumer

No drift detected.

---

# Execution Handoff

Plan complete and saved to `genesis/docs/superpowers/plans/2026-05-25-pillar-epr-decomposition-plan.md`.

**Two execution options:**

1. **Subagent-Driven (recommended)** — Dispatch a fresh subagent per task, review between tasks, fast iteration. Pairs well with the path-scoped subagents already configured (`rust-architect` for Phase A and the doorway tasks, `component-architect` for elohim-core elements + Library A stories, `graphos-designer` for Library B stories, `angular-architect` for the lamad split).

2. **Inline Execution** — Execute tasks in this session, batch execution with checkpoints for review.

Which approach?
