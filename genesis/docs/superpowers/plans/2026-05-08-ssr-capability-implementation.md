# SSR Capability Advertisement — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the SSR capability claim system end-to-end: a doorway derives its render capability from disk + storage manifest, exposes it at `/admin/capability`, and elohim-storage layers it into `PeerStatusView` so peers can see which doorways carry which bundles, support which auth modes, and have what concurrency budget. Adds the framework-agnostic V8 auth-threading and the Tier-2 extensions hatch alongside.

**Architecture:** View-layer projection mirroring the existing `elohimCapability` pattern (`elohim-storage/src/views.rs:6386-6400`). DHT entry shape is unchanged. Doorway is the auto-honest source of capability truth (it can only claim what's on disk + in the manifest). Storage pulls capability from doorway via HTTP at startup. CSR fallback is the floor on every miss/mismatch/overflow.

**Tech Stack:** Rust (axum, reqwest, tokio, serde, ts-rs, deno_core for V8), TypeScript codegen via JSON Schema, doorway's existing `elohim_render::DataFetcher` trait, peer-status SQLite projection in elohim-storage.

**Spec:** `genesis/docs/superpowers/specs/2026-05-08-ssr-capability-design.md`

---

## Source-of-truth declaration (read before touching schemas)

**All capability data introduced by this plan is Category C operational state — view-layer projection only.** Per the spec's "Validation surface" section:

| Aspect | Classification |
|---|---|
| Entity classification (per p2p-design-gate) | **C — Operational** |
| Source of truth | Doorway's runtime-derived state from disk + storage manifest |
| DHT entry types added | **0** (zero) — `infrastructure_integrity::PeerStatus` shape is unchanged |
| Integrity zome validators added | **0** (zero) — no DNA build, no validator changes |
| Where the schemas land | `views/` (HTTP wire shape) and `enums/` — alongside the existing `elohim-capability-profile.schema.json` precedent |
| How notarization happens today | It doesn't — the claim is informational. Stage-3 elohim-defender observation is the deferred path that would graduate it. |
| How identity is addressed | Slug — capability is keyed by doorway peer-id (not content-addressed; no CID; not agent-composite). Justified because: (a) operational/runtime state has no notarized identity, (b) a doorway has exactly one capability claim at a time, (c) no third party needs to verify the content-hash of the claim against a DHT-anchored copy. |
| Coordinator function that creates it | None — derived at startup by `doorway::render::derive_capability` in process |
| Signal that projects it | None — pulled by `elohim_storage::load_render_capability_from_url` from doorway's `/admin/capability` HTTP endpoint at storage startup |
| HTTP route exposing it | `GET /admin/capability` on doorway (returns the `RenderCapabilityProfile` JSON) |
| HTTP route consuming it | `GET /api/peers/{peer_id}` on storage (returns `PeerStatusView` with `renderCapability` layered in by `build_peer_status_view`) |

If any task in this plan looks like it's introducing a DHT entry type, an integrity validator, a coordinator function, or a content-addressed identifier for capability data — **STOP and re-read the spec**. That would be off-pattern.

---

## File Map

### Schemas (new) — all view-layer / Category C operational; no DHT entries introduced
- `elohim/sdk/schemas/v1/enums/renderer-kind.schema.json` — view-layer enum (Category C)
- `elohim/sdk/schemas/v1/views/render-capability-profile.schema.json` — view-layer projection (Category C; doorway-derived)
- `elohim/sdk/schemas/v1/views/capability-extensions.schema.json` — view-layer hatch (Category C; Tier-2 capabilities)
- `elohim/sdk/schemas/v1/registries/capability-registry.json` — documentation-only registry (no source-of-truth claim; informational)

### Schemas (modify) — view-layer extension only; DHT entry shape untouched
- `elohim/sdk/schemas/v1/views/peer-status-view.schema.json` — add `renderCapability` + `extensions` (both Category C, layered post-construction)
- `elohim/sdk/schemas/scripts/codegen-ts.mjs` — codegen distribution (no source-of-truth implications)

### elohim-storage (modify)
- `elohim/elohim-storage/src/views.rs` — new types `RenderCapabilityProfile`, `BundleEntry`, `RendererKind`, `CapabilityExtensions`; extend `PeerStatusView`; extend `build_peer_status_view`; new `load_render_capability_from_url`
- `elohim/elohim-storage/src/main.rs` — wire `load_render_capability_from_url` into startup
- `elohim/elohim-storage/src/lib.rs` — re-exports for the new types
- `elohim/elohim-storage/src/api/peer_statuses.rs` — pass new args to `build_peer_status_view`
- `elohim/elohim-storage/src/app_context.rs` (or wherever `AppContext` lives) — store `render_capability` + `extensions`
- `elohim/elohim-storage/tests/schema_contract.rs` — round-trip tests for new types

### Doorway (new + modify)
- `doorway/doorway-service/src/render/mod.rs` (new) — module aggregator
- `doorway/doorway-service/src/render/capability.rs` (new) — bundle scanner, manifest fetcher, override parser, deriver
- `doorway/doorway-service/src/render/types.rs` (new) — Rust mirrors of `RenderCapabilityProfile`, `BundleEntry`, etc.
- `doorway/doorway-service/src/lib.rs` — register `render` module
- `doorway/doorway-service/src/ssr.rs` — extend `ResolverFetcher` with `user_credential: Option<UserCredential>`
- `doorway/doorway-service/src/server/http.rs` — `/admin/capability` route, auth-mode enforcement, concurrency semaphore, observability headers, capability deriver init

### Doorway tests (new)
- `doorway/doorway-service/tests/capability_publish.rs` — boot, derive, expose at `/admin/capability`
- `doorway/doorway-service/tests/auth_mode_enforcement.rs` — auth-mode mismatch → CSR fallback
- `doorway/doorway-service/tests/concurrency_overflow.rs` — semaphore overflow → CSR fallback

### a2o (new)
- `genesis/a2o/features/content/ssr_capability.feature`

---

## Phase 1: Schema foundation

### Task 1: Renderer-kind enum

**Files:**
- Create: `elohim/sdk/schemas/v1/enums/renderer-kind.schema.json` — Category C, view-layer enum (no DHT entry, no validator)

- [ ] **Step 1: Write the schema**

```json
{
  "$id": "epr:schema:enum:renderer-kind",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "RendererKind",
  "description": "Kind of server-side renderer a doorway carries. Reserved values are valid claim values; only `angular-ssr` is implemented in elohim-render today. Source of truth: doorway runtime (Category C operational). Not a DHT entry type.",
  "_sourceOfTruth": {
    "category": "C",
    "layer": "view",
    "owner": "doorway runtime",
    "notarized": false,
    "rationale": "Renderer kind is a runtime-derived label tied to bundle shape; it's part of the operator's compute-shape claim, not a notarized protocol primitive"
  },
  "type": "string",
  "enum": ["angular-ssr", "react-rsc", "vue-ssr", "svelte-ssr", "lit-ssr", "static-html"],
  "_tiers": {
    "core": {
      "values": ["angular-ssr"],
      "rationale": "angular-ssr is the only renderer the protocol ships today; others are reserved for future framework adapters"
    }
  }
}
```

- [ ] **Step 2: Validate the schema parses**

Run: `cd elohim/sdk && pnpm run schema:test`
Expected: PASS (no breakage to existing tests).

- [ ] **Step 3: Commit**

```bash
git add elohim/sdk/schemas/v1/enums/renderer-kind.schema.json
git commit -m "feat(schema): renderer-kind enum (Tier-1 reserve for SSR)"
```

---

### Task 2: Render-capability-profile schema

**Files:**
- Create: `elohim/sdk/schemas/v1/views/render-capability-profile.schema.json` — Category C, view-layer projection (no DHT entry, no validator)

- [ ] **Step 1: Write the schema**

```json
{
  "$id": "epr:schema:view:render-capability-profile",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "RenderCapabilityProfile",
  "description": "Source of truth: auto-derived at doorway startup from on-disk bundles intersected with elohim-storage's manifest of SSR-eligible routes (Operational, Category C). doorway-config.toml may reduce the claim but never inflate it. Layered into PeerStatusView via build_peer_status_view, mirroring the elohimCapability pattern. NOT a DHT entry.",
  "_sourceOfTruth": {
    "category": "C",
    "layer": "view",
    "owner": "doorway runtime (derived from disk + storage manifest)",
    "notarized": false,
    "consumed_by": "elohim-storage build_peer_status_view (layered post-construction)",
    "rationale": "Compute-shape claim about doorway-local SSR capability; informational visibility for substrate matchmaking. Stage-3 elohim-defender enforcement is the deferred path that would graduate the claim to DHT-attested."
  },
  "type": "object",
  "required": ["bundles", "authModes", "maxConcurrentRenders", "renderers"],
  "properties": {
    "bundles": {
      "type": "array",
      "minItems": 1,
      "items": {
        "type": "object",
        "required": ["name", "version", "renderer"],
        "properties": {
          "name":     { "type": "string", "description": "Bundle name (e.g. lamad-app, qahal-app)" },
          "version":  { "type": "string", "description": "Semver bundle version" },
          "renderer": { "$ref": "../enums/renderer-kind.schema.json" },
          "digest":   { "type": ["string", "null"], "description": "Optional sha256 hash of the bundle file" }
        },
        "additionalProperties": false
      }
    },
    "renderers": {
      "type": "array",
      "minItems": 1,
      "items": { "$ref": "../enums/renderer-kind.schema.json" },
      "description": "Distinct renderer kinds (deduplicated bundles[].renderer). Cheap-to-query summary."
    },
    "authModes": {
      "type": "array",
      "minItems": 1,
      "items": {
        "type": "string",
        "enum": ["anonymous", "doorway-hosted", "steward-presence"]
      },
      "description": "Auth modes this doorway honors for SSR. 'anonymous' must always be present."
    },
    "maxConcurrentRenders": {
      "type": "integer",
      "minimum": 0,
      "description": "Operator-declared concurrency budget. CSR fallback fires when reached."
    },
    "memoryBudgetMib": {
      "type": ["integer", "null"],
      "minimum": 0,
      "description": "Operator-declared memory ceiling for the renderer (informational; null = cgroup-managed)"
    }
  },
  "additionalProperties": false
}
```

- [ ] **Step 2: Verify the schema is well-formed**

Run: `cd elohim/sdk && pnpm run schema:test`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add elohim/sdk/schemas/v1/views/render-capability-profile.schema.json
git commit -m "feat(schema): render-capability-profile (Tier-1 SSR claim)"
```

---

### Task 3: Capability-extensions schema (Tier-2 hatch)

**Files:**
- Create: `elohim/sdk/schemas/v1/views/capability-extensions.schema.json` — Category C, view-layer hatch (no DHT entry, no validator)

- [ ] **Step 1: Write the schema**

```json
{
  "$id": "epr:schema:view:capability-extensions",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "CapabilityExtensions",
  "description": "Tier-2 capability claims map. Source of truth: each capability's owner declares it via runtime registration (Category C operational). Each key is a kebab-case capability name registered in the protocol's capability registry. Each value carries a schemaRef pointer (so consumers can resolve the profile schema) and an opaque structured profile. The validator checks structural well-formedness only — claim CONTENTS are interpreted by consumers who recognize the capability name. When a Tier-2 capability proves load-bearing, it graduates to Tier 1 with a typed sibling field on peer-status. NOT a DHT entry.",
  "_sourceOfTruth": {
    "category": "C",
    "layer": "view",
    "owner": "per-capability runtime owners (each capability declares its own profile)",
    "notarized": false,
    "rationale": "Generic extension hatch for Tier-2 capabilities. Inherits the parent peer-status-view's view-layer projection model. Promotion to Tier 1 (typed sibling field) requires substrate-wide load-bearing use."
  },
  "type": "object",
  "patternProperties": {
    "^[a-z][a-z0-9-]{2,30}$": {
      "type": "object",
      "required": ["schemaRef", "profile"],
      "properties": {
        "schemaRef": {
          "type": "string",
          "pattern": "^epr:schema:",
          "description": "Schema URI for this capability's profile (e.g., 'epr:schema:view:transcode-capability-profile')"
        },
        "profile": {
          "type": "object",
          "description": "Capability-specific claim. Shape defined by schemaRef. Validator checks 'is an object'; consumers do deep validation."
        }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

- [ ] **Step 2: Verify the schema is well-formed**

Run: `cd elohim/sdk && pnpm run schema:test`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add elohim/sdk/schemas/v1/views/capability-extensions.schema.json
git commit -m "feat(schema): capability-extensions (Tier-2 claim hatch)"
```

---

### Task 4: Extend peer-status-view with new fields and run codegen

**Files:**
- Modify: `elohim/sdk/schemas/v1/views/peer-status-view.schema.json`
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs:35` (INTERFACE_FILES)

- [ ] **Step 1: Add the two new optional fields to peer-status-view**

In `elohim/sdk/schemas/v1/views/peer-status-view.schema.json`, after the `elohimCapability` field block add:

```json
    "renderCapability": {
      "oneOf": [
        { "$ref": "render-capability-profile.schema.json" },
        { "type": "null" }
      ],
      "description": "If this peer runs a doorway that can server-render, advertises its render-capability profile. Null or absent = no SSR (storage-only peer, or operator opted out). Layered post-construction by build_peer_status_view."
    },
    "extensions": {
      "oneOf": [
        { "$ref": "capability-extensions.schema.json" },
        { "type": "null" }
      ],
      "description": "Tier-2 extension capabilities. Apps register kebab-case capability names; each entry has a schemaRef + structured profile. Layered post-construction by build_peer_status_view."
    }
```

- [ ] **Step 2: Add new files to INTERFACE_FILES**

In `elohim/sdk/schemas/scripts/codegen-ts.mjs`, after line 51 (`elohim-capability-profile.ts`), insert:

```javascript
  { src: 'views/render-capability-profile.ts', dest: 'render-capability-profile.ts' },
  { src: 'views/capability-extensions.ts', dest: 'capability-extensions.ts' },
```

- [ ] **Step 3: Run codegen**

Run: `cd elohim/sdk && pnpm run schema:codegen:ts`
Expected: completes without error; new `.ts` files appear in all three distribution dirs (`genesis/seeder/src/generated/`, `app/elohim-app/src/app/generated/`, `app/elohim-library/projects/elohim-service/src/generated/`).

- [ ] **Step 4: Run schema validation**

Run: `cd elohim/sdk && pnpm run schema:test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/schemas/v1/views/peer-status-view.schema.json \
        elohim/sdk/schemas/scripts/codegen-ts.mjs \
        genesis/seeder/src/generated/ \
        app/elohim-app/src/app/generated/ \
        app/elohim-library/projects/elohim-service/src/generated/
git commit -m "feat(schema): wire renderCapability + extensions into peer-status-view"
```

---

## Phase 2: Storage view-layer types

### Task 5: Add Rust types in views.rs

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs` (add new types near the existing `ElohimCapabilityProfile` definition around line 6313)

- [ ] **Step 1: Write a failing schema-contract test stub**

In `elohim/elohim-storage/tests/schema_contract.rs`, add at the end:

```rust
#[test]
fn render_capability_profile_round_trips_against_schema() {
    use elohim_storage::{RenderCapabilityProfile, BundleEntry, RendererKind};
    let profile = RenderCapabilityProfile {
        bundles: vec![BundleEntry {
            name: "lamad-app".into(),
            version: "1.0.3".into(),
            renderer: RendererKind::AngularSsr,
            digest: Some("sha256:abc123".into()),
        }],
        renderers: vec![RendererKind::AngularSsr],
        auth_modes: vec!["anonymous".into(), "doorway-hosted".into()],
        max_concurrent_renders: 8,
        memory_budget_mib: Some(1024),
    };
    let json = serde_json::to_string(&profile).expect("serialize");
    let back: RenderCapabilityProfile = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.bundles[0].name, "lamad-app");
    assert_eq!(back.bundles[0].renderer, RendererKind::AngularSsr);
    assert!(back.auth_modes.contains(&"anonymous".to_string()));
}

#[test]
fn capability_extensions_round_trips() {
    use elohim_storage::CapabilityExtensions;
    use serde_json::json;
    let ext_json = json!({
        "transcode": {
            "schemaRef": "epr:schema:view:transcode-capability-profile",
            "profile": { "codecs": ["h264", "av1"] }
        }
    });
    let ext: CapabilityExtensions = serde_json::from_value(ext_json.clone()).expect("deserialize");
    let back = serde_json::to_value(&ext).expect("serialize");
    assert_eq!(back, ext_json);
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract render_capability_profile_round_trips_against_schema`
Expected: FAIL with "unresolved import" for `RenderCapabilityProfile`, `BundleEntry`, `RendererKind`.

- [ ] **Step 3: Add the Rust types**

In `elohim/elohim-storage/src/views.rs`, near line 6313 (just before the existing `ElohimCapabilityProfile` struct), add:

```rust
/// Renderer kind a bundle targets. Mirrors `enums/renderer-kind.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum RendererKind {
    AngularSsr,
    ReactRsc,
    VueSsr,
    SvelteSsr,
    LitSsr,
    StaticHtml,
}

/// One bundle a doorway carries (mirrors `bundles[]` items in the profile schema).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct BundleEntry {
    pub name: String,
    pub version: String,
    pub renderer: RendererKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
}

/// Tier-1 render capability profile. View-layer Category C operational state,
/// layered into PeerStatusView post-construction. NOT a DHT entry.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RenderCapabilityProfile {
    pub bundles: Vec<BundleEntry>,
    pub renderers: Vec<RendererKind>,
    pub auth_modes: Vec<String>,
    pub max_concurrent_renders: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_budget_mib: Option<u32>,
}

/// Tier-2 extension capability claim (one entry in the extensions map).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CapabilityExtensionEntry {
    pub schema_ref: String,
    pub profile: serde_json::Value,
}

/// Tier-2 extensions map. Keys are kebab-case capability names registered in
/// the capability registry. Validation checks shape only; consumers interpret content.
pub type CapabilityExtensions = std::collections::BTreeMap<String, CapabilityExtensionEntry>;
```

- [ ] **Step 4: Re-export from `lib.rs`**

In `elohim/elohim-storage/src/lib.rs`, find the existing line that re-exports `ElohimCapabilityProfile` (around line 184) and add to the same export block:

```rust
RendererKind, BundleEntry, RenderCapabilityProfile, CapabilityExtensions, CapabilityExtensionEntry,
```

- [ ] **Step 5: Verify the test compiles and passes**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract render_capability_profile_round_trips_against_schema capability_extensions_round_trips`
Expected: PASS for both tests.

- [ ] **Step 6: Regenerate ts-rs bindings**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings`
Expected: PASS, generated `.ts` files appear in `elohim/sdk/storage-client-ts/src/generated/`.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/views.rs \
        elohim/elohim-storage/src/lib.rs \
        elohim/elohim-storage/tests/schema_contract.rs \
        elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(storage): RenderCapabilityProfile + CapabilityExtensions types"
```

---

### Task 6: Extend PeerStatusView struct

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs:6367` (the `PeerStatusView` struct)
- Modify: `elohim/elohim-storage/src/views.rs:6386-6400` (the `build_peer_status_view` function)
- Modify: `elohim/elohim-storage/src/views.rs:6370-6383` (the `From<PeerStatusRow> for PeerStatusView` impl)

- [ ] **Step 1: Write a failing test for the extended view**

Append to `elohim/elohim-storage/tests/schema_contract.rs`:

```rust
#[test]
fn peer_status_view_carries_render_capability_when_layered() {
    use elohim_storage::{
        build_peer_status_view, BundleEntry, RenderCapabilityProfile,
        RendererKind, PeerStatusRow,
    };
    let row = PeerStatusRow {
        peer_id: "peer-x".into(),
        status: "online".into(),
        general_pool_member: 1,
        accepting_stewardship_reserves: 0,
        archetype_class: Some("home-nuc".into()),
        timestamp: 1_700_000_000_000_000,
        dht_anchor_hash: "anchor-1".into(),
        updated_at: 1_700_000_000_000_000,
    };
    let render_cap = RenderCapabilityProfile {
        bundles: vec![BundleEntry {
            name: "lamad-app".into(),
            version: "1.0.3".into(),
            renderer: RendererKind::AngularSsr,
            digest: None,
        }],
        renderers: vec![RendererKind::AngularSsr],
        auth_modes: vec!["anonymous".into()],
        max_concurrent_renders: 4,
        memory_budget_mib: None,
    };
    let view = build_peer_status_view(row, None, Some(&render_cap), None);
    assert!(view.render_capability.is_some());
    assert_eq!(view.render_capability.unwrap().bundles[0].name, "lamad-app");
    assert!(view.extensions.is_none());
}

#[test]
fn peer_status_view_renders_null_capability_when_unlayered() {
    use elohim_storage::{build_peer_status_view, PeerStatusRow};
    let row = PeerStatusRow {
        peer_id: "peer-y".into(),
        status: "online".into(),
        general_pool_member: 1,
        accepting_stewardship_reserves: 0,
        archetype_class: None,
        timestamp: 1_700_000_000_000_000,
        dht_anchor_hash: "anchor-2".into(),
        updated_at: 1_700_000_000_000_000,
    };
    let view = build_peer_status_view(row, None, None, None);
    assert!(view.render_capability.is_none());
    assert!(view.extensions.is_none());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract peer_status_view_carries_render_capability`
Expected: FAIL — `build_peer_status_view` doesn't take 4 args, `PeerStatusView` has no `render_capability` field.

- [ ] **Step 3: Extend `PeerStatusView` struct**

In `elohim/elohim-storage/src/views.rs` around line 6367, after the `elohim_capability` field, add:

```rust
    /// Render capability profile if a doorway co-located with this peer can SSR.
    /// Layered post-construction via build_peer_status_view; not in DHT entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_capability: Option<RenderCapabilityProfile>,

    /// Tier-2 extension capabilities. Layered post-construction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<CapabilityExtensions>,
```

- [ ] **Step 4: Update `From<PeerStatusRow>` impl**

In the same file, in `From<PeerStatusRow> for PeerStatusView` (around line 6370), add to the struct literal:

```rust
            render_capability: None, // Layered post-construction via build_peer_status_view()
            extensions: None,        // Layered post-construction via build_peer_status_view()
```

- [ ] **Step 5: Update `build_peer_status_view` signature and body**

Replace the existing `build_peer_status_view` (around line 6393) with:

```rust
pub fn build_peer_status_view(
    row: PeerStatusRow,
    elohim_capability: Option<&ElohimCapabilityProfile>,
    render_capability: Option<&RenderCapabilityProfile>,
    extensions: Option<&CapabilityExtensions>,
) -> PeerStatusView {
    let mut view = PeerStatusView::from(row);
    view.elohim_capability = elohim_capability.cloned();
    view.render_capability = render_capability.cloned();
    view.extensions = extensions.cloned();
    view
}
```

- [ ] **Step 6: Update all callers of build_peer_status_view**

Run: `grep -rn "build_peer_status_view" /projects/elohim/elohim/elohim-storage/src /projects/elohim/elohim/elohim-storage/tests`
Expected: list of call sites; each one's signature must be updated to pass `None, None` (or appropriate values) for the two new args.

For each call site, change `build_peer_status_view(row, cap)` to `build_peer_status_view(row, cap, None, None)` unless context provides values.

- [ ] **Step 7: Run the round-trip and view tests**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract`
Expected: PASS for all schema_contract tests.

- [ ] **Step 8: Run full crate build to catch any other call-site breakage**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build`
Expected: success, no warnings about unused imports.

- [ ] **Step 9: Regenerate ts-rs bindings**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings`
Expected: PASS, `peer-status-view.ts` updated with new fields.

- [ ] **Step 10: Commit**

```bash
git add elohim/elohim-storage/ elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(storage): layer renderCapability + extensions into PeerStatusView"
```

---

### Task 7: Add `load_render_capability_from_url`

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs` — add new loader near `load_elohim_capability_from_env` (line 6402)

- [ ] **Step 1: Write a failing test**

Append to `elohim/elohim-storage/tests/schema_contract.rs`:

```rust
#[test]
fn load_render_capability_from_url_returns_none_when_unset() {
    // env var unset → None (honest degradation)
    std::env::remove_var("DOORWAY_CAPABILITY_URL");
    let result = elohim_storage::load_render_capability_from_url_blocking();
    assert!(result.is_none());
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract load_render_capability_from_url_returns_none_when_unset`
Expected: FAIL with "function not found".

- [ ] **Step 3: Add the loader functions**

In `elohim/elohim-storage/src/views.rs`, after `load_elohim_capability_from_env` (around line 6440), add:

```rust
/// Load the render capability profile from a doorway's `/admin/capability` HTTP endpoint.
///
/// Uses the URL in `DOORWAY_CAPABILITY_URL`. Returns `None` (honest degradation) when:
/// - The env var is unset
/// - The URL is unreachable
/// - The response is non-2xx
/// - The body fails to parse as `RenderCapabilityProfile`
pub async fn load_render_capability_from_url() -> Option<RenderCapabilityProfile> {
    let url = std::env::var("DOORWAY_CAPABILITY_URL").ok()?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        tracing::warn!(
            url = %url,
            status = %resp.status(),
            "DOORWAY_CAPABILITY_URL returned non-success — render_capability will be None"
        );
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    match serde_json::from_slice::<RenderCapabilityProfile>(&bytes) {
        Ok(profile) => Some(profile),
        Err(e) => {
            tracing::warn!(
                url = %url,
                error = %e,
                "DOORWAY_CAPABILITY_URL response did not parse as RenderCapabilityProfile"
            );
            None
        }
    }
}

/// Synchronous wrapper for tests / non-async startup paths.
pub fn load_render_capability_from_url_blocking() -> Option<RenderCapabilityProfile> {
    if std::env::var("DOORWAY_CAPABILITY_URL").is_err() {
        return None;
    }
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    rt.block_on(load_render_capability_from_url())
}
```

- [ ] **Step 4: Re-export from `lib.rs`**

In `elohim/elohim-storage/src/lib.rs`, add `load_render_capability_from_url`, `load_render_capability_from_url_blocking` to the existing re-export list near `load_elohim_capability_from_env` (around line 184).

- [ ] **Step 5: Run the test**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract load_render_capability_from_url_returns_none_when_unset`
Expected: PASS.

- [ ] **Step 6: Add an integration test for a successful pull**

Append to `elohim/elohim-storage/tests/schema_contract.rs`:

```rust
#[tokio::test]
async fn load_render_capability_from_url_parses_valid_response() {
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    let body = serde_json::json!({
        "bundles": [{ "name": "lamad-app", "version": "1.0.3", "renderer": "angular-ssr" }],
        "renderers": ["angular-ssr"],
        "authModes": ["anonymous"],
        "maxConcurrentRenders": 4
    });
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/admin/capability"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    std::env::set_var("DOORWAY_CAPABILITY_URL", format!("{}/admin/capability", server.uri()));
    let result = elohim_storage::load_render_capability_from_url().await;
    std::env::remove_var("DOORWAY_CAPABILITY_URL");
    assert!(result.is_some());
    let profile = result.unwrap();
    assert_eq!(profile.bundles.len(), 1);
    assert_eq!(profile.bundles[0].name, "lamad-app");
    assert!(profile.auth_modes.contains(&"anonymous".to_string()));
}
```

If `wiremock` is not yet a dev-dependency, add it to `elohim/elohim-storage/Cargo.toml`:

```toml
[dev-dependencies]
wiremock = "0.6"
```

- [ ] **Step 7: Run the integration test**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract load_render_capability_from_url_parses_valid_response`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add elohim/elohim-storage/
git commit -m "feat(storage): load_render_capability_from_url with env-driven fetch"
```

---

### Task 8: Wire capability into AppContext + main startup

**Files:**
- Modify: `elohim/elohim-storage/src/main.rs:870-930` — startup region where `elohim_capability` is loaded and threaded
- Modify: `AppContext` struct (find with `grep -rn "struct AppContext" elohim/elohim-storage/src`)
- Modify: `elohim/elohim-storage/src/api/peer_statuses.rs` — the handler that calls `build_peer_status_view`

- [ ] **Step 1: Identify the AppContext path**

Run: `grep -rn "struct AppContext" /projects/elohim/elohim/elohim-storage/src | head -3`
Expected: a hit (likely in `db/mod.rs` or a top-level `app_context.rs`); note the path.

- [ ] **Step 2: Add fields to AppContext**

In the file containing `struct AppContext`, find the existing `elohim_capability: Option<ElohimCapabilityProfile>` field (or similar). After it, add:

```rust
    pub render_capability: Option<RenderCapabilityProfile>,
    pub extensions: Option<CapabilityExtensions>,
```

Update any constructors / `with_*` builder methods to default these to `None`. If there's a `with_elohim_capability` pattern at line 928 of main.rs, add a parallel `with_render_capability` builder method.

- [ ] **Step 3: Wire startup**

In `elohim/elohim-storage/src/main.rs` around line 877 (after `let elohim_capability = ...`), add:

```rust
    let render_capability = elohim_storage::load_render_capability_from_url().await;
    if render_capability.is_some() {
        tracing::info!("render_capability loaded from DOORWAY_CAPABILITY_URL");
    }
```

Then around line 928 (the builder chain), add `.with_render_capability(render_capability)` after `.with_elohim_capability(elohim_capability)`.

- [ ] **Step 4: Update peer-status handler to pass new args**

In `elohim/elohim-storage/src/api/peer_statuses.rs`, find each call to `build_peer_status_view` and update its arguments:

Change:
```rust
build_peer_status_view(row, app_ctx.elohim_capability.as_ref())
```

To:
```rust
build_peer_status_view(
    row,
    app_ctx.elohim_capability.as_ref(),
    app_ctx.render_capability.as_ref(),
    app_ctx.extensions.as_ref(),
)
```

- [ ] **Step 5: Run the storage build**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build`
Expected: clean build.

- [ ] **Step 6: Run all storage tests**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/
git commit -m "feat(storage): wire render_capability + extensions through AppContext"
```

---

### Task 9: Capability registry stub

**Files:**
- Create: `elohim/sdk/schemas/v1/registries/capability-registry.json`

- [ ] **Step 1: Create the registry directory and file**

```bash
mkdir -p elohim/sdk/schemas/v1/registries
```

Write `elohim/sdk/schemas/v1/registries/capability-registry.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "version": "1.0.0",
  "description": "Registry of capability claims that may appear on PeerStatusView. Tier-1 capabilities have typed sibling fields; Tier-2 capabilities live in the extensions map. Promotion criteria are documented in the SSR capability spec.",
  "capabilities": [
    {
      "name": "elohim",
      "tier": 1,
      "schemaRef": "epr:schema:view:elohim-capability-profile",
      "addedAt": "2026-01-01T00:00:00Z",
      "description": "Model-level claims about an elohim-agent running at this peer (model name, family, context window, specialties)."
    },
    {
      "name": "render",
      "tier": 1,
      "schemaRef": "epr:schema:view:render-capability-profile",
      "addedAt": "2026-05-08T00:00:00Z",
      "description": "Server-side rendering capability of a doorway co-located with this peer (bundles, auth modes, concurrency budget)."
    }
  ]
}
```

- [ ] **Step 2: Commit**

```bash
git add elohim/sdk/schemas/v1/registries/capability-registry.json
git commit -m "docs(schema): capability registry for Tier-1/Tier-2 claims"
```

---

## Phase 3: Doorway capability deriver

### Task 10: Scaffold `render` module on doorway

**Files:**
- Create: `doorway/doorway-service/src/render/mod.rs`
- Create: `doorway/doorway-service/src/render/types.rs`
- Modify: `doorway/doorway-service/src/lib.rs` — register `render` module

- [ ] **Step 1: Create the module files**

Write `doorway/doorway-service/src/render/mod.rs`:

```rust
//! Capability derivation, override layering, and exposure for the doorway's
//! SSR runtime. The deriver scans on-disk bundles, intersects with elohim-
//! storage's manifest of SSR-eligible routes, and produces a
//! `RenderCapabilityProfile` that doorway exposes at `/admin/capability`.
//!
//! The profile is the source of truth for "what this doorway claims it can
//! render" — auto-honest by construction (only what's on disk + in the
//! manifest can be claimed), with operator override able to reduce.

pub mod capability;
pub mod types;

pub use capability::{derive_capability, CapabilityDeriverError};
pub use types::{BundleEntry, RenderCapabilityProfile, RendererKind};
```

Write `doorway/doorway-service/src/render/types.rs`:

```rust
//! Rust mirrors of the protocol's render-capability profile types.
//! These match `elohim-storage::RenderCapabilityProfile` etc. on the wire
//! so storage can deserialize doorway's `/admin/capability` response directly.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RendererKind {
    AngularSsr,
    ReactRsc,
    VueSsr,
    SvelteSsr,
    LitSsr,
    StaticHtml,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleEntry {
    pub name: String,
    pub version: String,
    pub renderer: RendererKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderCapabilityProfile {
    pub bundles: Vec<BundleEntry>,
    pub renderers: Vec<RendererKind>,
    pub auth_modes: Vec<String>,
    pub max_concurrent_renders: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_budget_mib: Option<u32>,
}
```

Create empty stub for `capability.rs`:

```rust
//! Capability deriver: bundles ∩ manifest, with override.

use crate::render::types::RenderCapabilityProfile;

#[derive(Debug, thiserror::Error)]
pub enum CapabilityDeriverError {
    #[error("bundles directory unreadable: {0}")]
    BundleDirRead(String),
    #[error("manifest fetch failed: {0}")]
    ManifestFetch(String),
    #[error("override config malformed: {0}")]
    OverrideMalformed(String),
}

pub async fn derive_capability(
    _bundles_dir: &std::path::Path,
    _storage_manifest_url: &str,
    _override_path: Option<&std::path::Path>,
) -> Result<Option<RenderCapabilityProfile>, CapabilityDeriverError> {
    // Tasks 11-14 fill this in.
    Ok(None)
}
```

- [ ] **Step 2: Register the module**

In `doorway/doorway-service/src/lib.rs`, after the existing module declarations, add:

```rust
pub mod render;
```

- [ ] **Step 3: Verify it builds**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo build`
Expected: clean build (just warnings about unused fields are OK).

- [ ] **Step 4: Commit**

```bash
git add doorway/doorway-service/src/render/ doorway/doorway-service/src/lib.rs
git commit -m "feat(doorway): scaffold render::capability module"
```

---

### Task 11: Bundle scanner

**Files:**
- Modify: `doorway/doorway-service/src/render/capability.rs`
- Add: bundle-header parsing helper

- [ ] **Step 1: Write a failing test**

Append to `doorway/doorway-service/src/render/capability.rs`:

```rust
#[cfg(test)]
mod scan_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_bundle(dir: &std::path::Path, name: &str, header: &str) {
        let path = dir.join(format!("{name}.bundle.mjs"));
        fs::write(path, header).expect("write bundle stub");
    }

    #[tokio::test]
    async fn scans_bundles_with_protocol_manifest_header() {
        let tmp = TempDir::new().unwrap();
        // Convention: every SSR bundle starts with a JSON banner comment:
        // /* @elohim-bundle {"name":"lamad-app","version":"1.0.3","renderer":"angular-ssr"} */
        write_bundle(
            tmp.path(),
            "lamad",
            r#"/* @elohim-bundle {"name":"lamad-app","version":"1.0.3","renderer":"angular-ssr"} */
            export function bootstrap() {}"#,
        );
        let bundles = scan_bundles(tmp.path()).await.expect("scan succeeds");
        assert_eq!(bundles.len(), 1);
        assert_eq!(bundles[0].name, "lamad-app");
        assert_eq!(bundles[0].version, "1.0.3");
        assert_eq!(bundles[0].renderer, RendererKind::AngularSsr);
    }

    #[tokio::test]
    async fn skips_bundles_without_header() {
        let tmp = TempDir::new().unwrap();
        write_bundle(tmp.path(), "no-header", "export const x = 1;");
        let bundles = scan_bundles(tmp.path()).await.expect("scan succeeds");
        assert!(bundles.is_empty());
    }

    #[tokio::test]
    async fn returns_empty_when_dir_missing() {
        let bundles = scan_bundles(std::path::Path::new("/nonexistent/path")).await;
        // Missing dir is honest-degradation: empty, not error
        assert!(bundles.unwrap().is_empty());
    }
}
```

If `tempfile` is not in `[dev-dependencies]`, add it.

- [ ] **Step 2: Run to verify it fails**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo test --lib render::capability::scan_tests`
Expected: FAIL — `scan_bundles` not defined.

- [ ] **Step 3: Implement scan_bundles**

In `doorway/doorway-service/src/render/capability.rs`, add:

```rust
use crate::render::types::{BundleEntry, RendererKind};
use serde::Deserialize;

#[derive(Deserialize)]
struct BundleHeader {
    name: String,
    version: String,
    renderer: RendererKind,
}

/// Scan a directory for `*.bundle.mjs` files and parse the
/// `@elohim-bundle {...}` JSON header. Files without a header are skipped.
/// Missing directory is honest-degradation: empty result, no error.
pub async fn scan_bundles(
    dir: &std::path::Path,
) -> Result<Vec<BundleEntry>, CapabilityDeriverError> {
    let entries = match tokio::fs::read_dir(dir).await {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(CapabilityDeriverError::BundleDirRead(e.to_string())),
    };
    let mut entries = entries;
    let mut bundles = Vec::new();
    while let Some(entry) = entries.next_entry().await
        .map_err(|e| CapabilityDeriverError::BundleDirRead(e.to_string()))?
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("mjs") {
            continue;
        }
        let name_ok = path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(".bundle.mjs"))
            .unwrap_or(false);
        if !name_ok { continue; }
        let contents = match tokio::fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(_) => continue,
        };
        if let Some(bundle) = parse_bundle_header(&contents) {
            bundles.push(bundle);
        }
    }
    Ok(bundles)
}

/// Parse a bundle's `@elohim-bundle {...}` header. Returns None when the
/// banner is missing or malformed (the caller skips silently).
fn parse_bundle_header(contents: &str) -> Option<BundleEntry> {
    let marker = "@elohim-bundle";
    let start = contents.find(marker)? + marker.len();
    let rest = &contents[start..];
    let json_start = rest.find('{')?;
    let json_end = rest[json_start..].find("*/")?;
    let json_str = rest[json_start..json_start + json_end].trim();
    let header: BundleHeader = serde_json::from_str(json_str).ok()?;
    Some(BundleEntry {
        name: header.name,
        version: header.version,
        renderer: header.renderer,
        digest: None,
    })
}
```

- [ ] **Step 4: Run the tests**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo test --lib render::capability::scan_tests`
Expected: PASS for all three.

- [ ] **Step 5: Commit**

```bash
git add doorway/doorway-service/src/render/capability.rs doorway/doorway-service/Cargo.toml
git commit -m "feat(doorway): bundle scanner for capability deriver"
```

---

### Task 12: Storage manifest fetcher

**Files:**
- Modify: `doorway/doorway-service/src/render/capability.rs`

- [ ] **Step 1: Write a failing test**

Append to the same `#[cfg(test)] mod scan_tests` block (or add a new mod):

```rust
#[cfg(test)]
mod manifest_tests {
    use super::*;
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn fetches_ssr_eligible_renderers_from_manifest() {
        let server = MockServer::start().await;
        let manifest = serde_json::json!({
            "routes": [
                { "path": "/lamad/concept/{id}", "render": "angular-ssr" },
                { "path": "/lamad/path/{slug}", "render": "angular-ssr" },
                { "path": "/api/content/{id}" }
            ]
        });
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/admin/manifest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(manifest))
            .mount(&server)
            .await;
        let url = format!("{}/admin/manifest", server.uri());
        let renderers = fetch_manifest_renderers(&url).await.expect("fetch ok");
        assert!(renderers.contains(&RendererKind::AngularSsr));
        assert_eq!(renderers.len(), 1); // dedup
    }

    #[tokio::test]
    async fn manifest_unreachable_returns_error() {
        let result = fetch_manifest_renderers("http://127.0.0.1:1/never").await;
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo test --lib render::capability::manifest_tests`
Expected: FAIL — `fetch_manifest_renderers` not defined.

- [ ] **Step 3: Implement the fetcher**

In `doorway/doorway-service/src/render/capability.rs`, add:

```rust
#[derive(serde::Deserialize)]
struct ManifestResponse {
    routes: Vec<ManifestRoute>,
}

#[derive(serde::Deserialize)]
struct ManifestRoute {
    #[serde(default)]
    render: Option<String>,
}

/// Fetch elohim-storage's manifest and extract the unique set of renderers
/// declared by SSR-eligible routes. Errors propagate so the caller can
/// publish `renderCapability: null` and retry.
pub async fn fetch_manifest_renderers(
    storage_manifest_url: &str,
) -> Result<Vec<RendererKind>, CapabilityDeriverError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| CapabilityDeriverError::ManifestFetch(e.to_string()))?;
    let resp = client.get(storage_manifest_url).send().await
        .map_err(|e| CapabilityDeriverError::ManifestFetch(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(CapabilityDeriverError::ManifestFetch(format!(
            "HTTP {}", resp.status()
        )));
    }
    let manifest: ManifestResponse = resp.json().await
        .map_err(|e| CapabilityDeriverError::ManifestFetch(e.to_string()))?;
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for route in manifest.routes {
        if let Some(rstr) = route.render {
            if let Ok(kind) = serde_json::from_value::<RendererKind>(serde_json::Value::String(rstr)) {
                if seen.insert(kind.clone()) {
                    out.push(kind);
                }
            }
        }
    }
    Ok(out)
}
```

- [ ] **Step 4: Run the tests**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo test --lib render::capability::manifest_tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add doorway/doorway-service/src/render/capability.rs
git commit -m "feat(doorway): manifest fetcher for SSR-eligible renderers"
```

---

### Task 13: Override TOML parser

**Files:**
- Modify: `doorway/doorway-service/src/render/capability.rs`

- [ ] **Step 1: Write a failing test**

Append to `doorway/doorway-service/src/render/capability.rs`:

```rust
#[cfg(test)]
mod override_tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn parses_full_override() {
        let toml = r#"
[render]
bundles_hidden = ["qahal-app"]
max_concurrent = 2
auth_modes = ["anonymous"]
memory_budget_mib = 512
        "#;
        let parsed = parse_override(toml).expect("parses");
        assert_eq!(parsed.bundles_hidden, vec!["qahal-app".to_string()]);
        assert_eq!(parsed.max_concurrent, Some(2));
        assert_eq!(parsed.auth_modes, Some(vec!["anonymous".to_string()]));
        assert_eq!(parsed.memory_budget_mib, Some(512));
    }

    #[test]
    fn empty_override_returns_default() {
        let parsed = parse_override("").expect("parses");
        assert!(parsed.bundles_hidden.is_empty());
        assert!(parsed.max_concurrent.is_none());
        assert!(parsed.auth_modes.is_none());
    }

    #[test]
    fn malformed_toml_returns_error() {
        let result = parse_override("[render\nbroken");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn missing_file_returns_default() {
        let result = load_override(Some(std::path::Path::new("/nonexistent/override.toml"))).await;
        // Spec: malformed/missing is honest degradation — no override applied
        assert!(result.bundles_hidden.is_empty());
    }
}
```

If `toml` and `tempfile` aren't in dependencies, add them:

```toml
[dependencies]
toml = "0.8"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo test --lib render::capability::override_tests`
Expected: FAIL — `parse_override` and `load_override` not defined.

- [ ] **Step 3: Implement override parsing**

Add to `doorway/doorway-service/src/render/capability.rs`:

```rust
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct RenderOverride {
    pub bundles_hidden: Vec<String>,
    pub max_concurrent: Option<u32>,
    pub auth_modes: Option<Vec<String>>,
    pub memory_budget_mib: Option<u32>,
}

#[derive(serde::Deserialize)]
struct OverrideRoot {
    #[serde(default)]
    render: RenderOverride,
}

/// Parse override TOML text. Returns `RenderOverride::default()` on empty input.
pub fn parse_override(text: &str) -> Result<RenderOverride, CapabilityDeriverError> {
    if text.trim().is_empty() {
        return Ok(RenderOverride::default());
    }
    let root: OverrideRoot = toml::from_str(text)
        .map_err(|e| CapabilityDeriverError::OverrideMalformed(e.to_string()))?;
    Ok(root.render)
}

/// Load override from a path. Missing file or malformed contents → default
/// (honest degradation: no override applied; warning logged).
pub async fn load_override(path: Option<&std::path::Path>) -> RenderOverride {
    let Some(path) = path else { return RenderOverride::default(); };
    let contents = match tokio::fs::read_to_string(path).await {
        Ok(c) => c,
        Err(_) => {
            tracing::debug!(path = %path.display(), "override file not present — using defaults");
            return RenderOverride::default();
        }
    };
    match parse_override(&contents) {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "override file malformed — ignoring (using derived claim verbatim)"
            );
            RenderOverride::default()
        }
    }
}
```

- [ ] **Step 4: Run the tests**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo test --lib render::capability::override_tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add doorway/doorway-service/src/render/capability.rs doorway/doorway-service/Cargo.toml
git commit -m "feat(doorway): override TOML parser for render capability"
```

---

### Task 14: Capability deriver (orchestrator)

**Files:**
- Modify: `doorway/doorway-service/src/render/capability.rs` — replace the stub `derive_capability`

- [ ] **Step 1: Write a failing test**

Append to `doorway/doorway-service/src/render/capability.rs`:

```rust
#[cfg(test)]
mod derive_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn derives_full_profile_when_disk_and_manifest_align() {
        let bundles = TempDir::new().unwrap();
        fs::write(
            bundles.path().join("lamad.bundle.mjs"),
            r#"/* @elohim-bundle {"name":"lamad-app","version":"1.0.3","renderer":"angular-ssr"} */"#,
        ).unwrap();
        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/admin/manifest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "routes": [{ "path": "/lamad/concept/{id}", "render": "angular-ssr" }]
            })))
            .mount(&server).await;
        let manifest_url = format!("{}/admin/manifest", server.uri());
        let result = derive_capability(bundles.path(), &manifest_url, None).await
            .expect("derive ok");
        let profile = result.expect("non-null");
        assert_eq!(profile.bundles.len(), 1);
        assert!(profile.auth_modes.contains(&"anonymous".to_string()));
        assert_eq!(profile.max_concurrent_renders, 8); // default
    }

    #[tokio::test]
    async fn returns_none_when_no_bundles_match_manifest() {
        let bundles = TempDir::new().unwrap();
        // No bundle files
        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/admin/manifest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "routes": [{ "path": "/lamad/concept/{id}", "render": "angular-ssr" }]
            })))
            .mount(&server).await;
        let result = derive_capability(
            bundles.path(),
            &format!("{}/admin/manifest", server.uri()),
            None,
        ).await.expect("derive ok");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn override_reduces_max_concurrent() {
        let bundles = TempDir::new().unwrap();
        fs::write(
            bundles.path().join("lamad.bundle.mjs"),
            r#"/* @elohim-bundle {"name":"lamad-app","version":"1.0.3","renderer":"angular-ssr"} */"#,
        ).unwrap();
        let override_file = bundles.path().join("override.toml");
        fs::write(&override_file, "[render]\nmax_concurrent = 1\n").unwrap();
        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/admin/manifest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "routes": [{ "path": "/lamad/concept/{id}", "render": "angular-ssr" }]
            })))
            .mount(&server).await;
        let result = derive_capability(
            bundles.path(),
            &format!("{}/admin/manifest", server.uri()),
            Some(override_file.as_path()),
        ).await.expect("derive ok");
        let profile = result.unwrap();
        assert_eq!(profile.max_concurrent_renders, 1);
    }

    #[tokio::test]
    async fn override_hides_bundle() {
        let bundles = TempDir::new().unwrap();
        fs::write(
            bundles.path().join("lamad.bundle.mjs"),
            r#"/* @elohim-bundle {"name":"lamad-app","version":"1.0.3","renderer":"angular-ssr"} */"#,
        ).unwrap();
        fs::write(
            bundles.path().join("qahal.bundle.mjs"),
            r#"/* @elohim-bundle {"name":"qahal-app","version":"0.2.0","renderer":"angular-ssr"} */"#,
        ).unwrap();
        let override_file = bundles.path().join("override.toml");
        fs::write(&override_file, r#"[render]
bundles_hidden = ["qahal-app"]"#).unwrap();
        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/admin/manifest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "routes": [{ "path": "/lamad/concept/{id}", "render": "angular-ssr" }]
            })))
            .mount(&server).await;
        let result = derive_capability(
            bundles.path(),
            &format!("{}/admin/manifest", server.uri()),
            Some(override_file.as_path()),
        ).await.expect("derive ok");
        let profile = result.unwrap();
        assert_eq!(profile.bundles.len(), 1);
        assert_eq!(profile.bundles[0].name, "lamad-app");
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo test --lib render::capability::derive_tests`
Expected: FAIL — `derive_capability` is the stub returning `Ok(None)`.

- [ ] **Step 3: Replace the stub `derive_capability`**

Replace the existing `derive_capability` function in `doorway/doorway-service/src/render/capability.rs` with:

```rust
const DEFAULT_MAX_CONCURRENT: u32 = 8;
const DEFAULT_AUTH_MODES: &[&str] = &["anonymous", "doorway-hosted"];

/// Auto-derive a render-capability claim. Honest by construction:
/// only bundles on disk whose renderer is referenced in storage's manifest
/// can appear in the claim. Override may reduce the claim but never inflate.
pub async fn derive_capability(
    bundles_dir: &std::path::Path,
    storage_manifest_url: &str,
    override_path: Option<&std::path::Path>,
) -> Result<Option<RenderCapabilityProfile>, CapabilityDeriverError> {
    let on_disk = scan_bundles(bundles_dir).await?;
    let manifest_renderers = match fetch_manifest_renderers(storage_manifest_url).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "manifest fetch failed — publishing renderCapability=null");
            return Ok(None);
        }
    };
    let renderer_set: std::collections::HashSet<_> = manifest_renderers.iter().cloned().collect();

    // Filter on-disk bundles to those whose renderer is in the manifest set.
    let mut bundles: Vec<BundleEntry> = on_disk.into_iter()
        .filter(|b| renderer_set.contains(&b.renderer))
        .collect();

    let override_cfg = load_override(override_path).await;

    // Apply override: hide bundles
    if !override_cfg.bundles_hidden.is_empty() {
        bundles.retain(|b| !override_cfg.bundles_hidden.contains(&b.name));
    }

    if bundles.is_empty() {
        return Ok(None);
    }

    let renderers: Vec<RendererKind> = {
        let mut seen = std::collections::HashSet::new();
        bundles.iter()
            .filter_map(|b| if seen.insert(b.renderer.clone()) { Some(b.renderer.clone()) } else { None })
            .collect()
    };

    // Auth modes: override-restricted or default (anonymous + doorway-hosted).
    // Per spec, anonymous must always be present.
    let auth_modes: Vec<String> = match override_cfg.auth_modes {
        Some(modes) => {
            let mut m = modes;
            if !m.iter().any(|x| x == "anonymous") {
                m.insert(0, "anonymous".to_string());
            }
            m
        }
        None => DEFAULT_AUTH_MODES.iter().map(|s| s.to_string()).collect(),
    };

    let max_concurrent_renders = override_cfg.max_concurrent.unwrap_or(DEFAULT_MAX_CONCURRENT);
    let memory_budget_mib = override_cfg.memory_budget_mib;

    Ok(Some(RenderCapabilityProfile {
        bundles,
        renderers,
        auth_modes,
        max_concurrent_renders,
        memory_budget_mib,
    }))
}
```

- [ ] **Step 4: Run all derive tests**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo test --lib render::capability::derive_tests`
Expected: PASS for all four tests.

- [ ] **Step 5: Run the whole render module test suite**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo test --lib render::capability`
Expected: PASS for all tests across scan_tests, manifest_tests, override_tests, derive_tests.

- [ ] **Step 6: Commit**

```bash
git add doorway/doorway-service/src/render/capability.rs
git commit -m "feat(doorway): capability deriver — bundles ∩ manifest, override-reducing"
```

---

## Phase 4: Doorway capability endpoint + startup

### Task 15: `/admin/capability` HTTP route + startup wiring

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` — register `/admin/capability` route
- Modify: doorway startup (likely `main.rs` or `lib.rs`) — wire deriver into `AppState`

- [ ] **Step 1: Identify the route registration site**

Run: `grep -n "admin/manifest\|admin/routes" /projects/elohim/doorway/doorway-service/src/server/http.rs | head -5`
Expected: existing admin routes — note the file/line where the router is built.

Run: `grep -n "struct AppState" /projects/elohim/doorway/doorway-service/src | head`
Expected: the `AppState` struct definition.

- [ ] **Step 2: Add a field to AppState**

In the `AppState` struct definition, add:

```rust
    /// Cached render-capability claim. Populated at startup; served by /admin/capability.
    /// `None` means doorway has no SSR runtime or the deriver returned None.
    pub render_capability: Option<crate::render::types::RenderCapabilityProfile>,
```

- [ ] **Step 3: Wire startup**

In the doorway main / startup function (where `AppState` is constructed), after the renderer init (~line 244 of `server/http.rs`), call the deriver:

```rust
let render_capability = if let Ok(bundles_dir) = std::env::var("SSR_BUNDLES_DIR") {
    let manifest_url = std::env::var("STORAGE_URL")
        .map(|u| format!("{}/admin/manifest", u.trim_end_matches('/')))
        .unwrap_or_else(|_| "http://localhost:8090/admin/manifest".into());
    let override_path = std::env::var("DOORWAY_RENDER_OVERRIDE")
        .ok()
        .map(std::path::PathBuf::from);
    match crate::render::derive_capability(
        std::path::Path::new(&bundles_dir),
        &manifest_url,
        override_path.as_deref(),
    ).await {
        Ok(c) => { tracing::info!(?c, "render capability derived"); c }
        Err(e) => { tracing::error!(error = %e, "deriver failed"); None }
    }
} else {
    None
};
```

Pass `render_capability` into `AppState`.

- [ ] **Step 4: Add the route handler**

In `doorway/doorway-service/src/server/http.rs`, near the other admin routes, add:

```rust
async fn admin_capability_handler(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<AppState>>,
) -> impl axum::response::IntoResponse {
    use axum::http::StatusCode;
    use axum::Json;
    match &state.render_capability {
        Some(profile) => (StatusCode::OK, Json(profile.clone())).into_response(),
        None => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
    }
}
```

In the router builder (find the existing `.route("/admin/manifest", ...)` pattern), add:

```rust
.route("/admin/capability", axum::routing::get(admin_capability_handler))
```

- [ ] **Step 5: Write an integration test**

Create `doorway/doorway-service/tests/capability_publish.rs`:

```rust
use doorway_service::render::types::{
    BundleEntry, RenderCapabilityProfile, RendererKind,
};

#[tokio::test]
async fn admin_capability_returns_layered_profile() {
    // Spin up a doorway test instance with a fake state
    // (use the same test-helper as registry_render.rs; if it doesn't exist
    // yet, create a minimal version here.)
    let profile = RenderCapabilityProfile {
        bundles: vec![BundleEntry {
            name: "lamad-app".into(),
            version: "1.0.3".into(),
            renderer: RendererKind::AngularSsr,
            digest: None,
        }],
        renderers: vec![RendererKind::AngularSsr],
        auth_modes: vec!["anonymous".into(), "doorway-hosted".into()],
        max_concurrent_renders: 4,
        memory_budget_mib: None,
    };
    // Build AppState with the profile, spawn the server, GET /admin/capability,
    // assert body equals JSON-serialized profile.
    let app = doorway_service::server::http::test_app_with_capability(Some(profile.clone()));
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/admin/capability")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let returned: RenderCapabilityProfile = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(returned.bundles[0].name, "lamad-app");
    assert!(returned.auth_modes.contains(&"doorway-hosted".to_string()));
}

#[tokio::test]
async fn admin_capability_returns_null_when_unset() {
    let app = doorway_service::server::http::test_app_with_capability(None);
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/admin/capability")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&bytes[..], b"null");
}
```

If `test_app_with_capability` doesn't exist, add it to `server/http.rs` behind `#[cfg(test)] pub fn test_app_with_capability(cap: Option<RenderCapabilityProfile>) -> Router { ... }` returning a minimal axum Router with just the `/admin/capability` route and a stub `AppState`.

- [ ] **Step 6: Run the integration test**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo test --test capability_publish`
Expected: PASS for both tests.

- [ ] **Step 7: Commit**

```bash
git add doorway/doorway-service/src/ doorway/doorway-service/tests/capability_publish.rs
git commit -m "feat(doorway): /admin/capability endpoint + startup deriver wiring"
```

---

## Phase 5: V8 fetch shim — auth threading

### Task 16: Extend ResolverFetcher with user_credential

**Files:**
- Modify: `doorway/doorway-service/src/ssr.rs` (the `ResolverFetcher` struct and impl)

- [ ] **Step 1: Write a failing test**

Append to `doorway/doorway-service/src/ssr.rs`:

```rust
#[cfg(test)]
mod fetcher_auth_tests {
    use super::*;
    use elohim_render::FetchRequest;
    use std::collections::HashMap;
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn forwards_user_credential_header() {
        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::header("authorization", "Bearer user-token"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&server)
            .await;
        let client = std::sync::Arc::new(reqwest::Client::new());
        let fetcher = ResolverFetcher::new(client, server.uri())
            .with_user_credential(UserCredential {
                header_name: "Authorization".into(),
                header_value: "Bearer user-token".into(),
            });
        let req = FetchRequest {
            url: "/api/private".into(),
            method: "GET".into(),
            headers: HashMap::new(),
            body: None,
        };
        let resp = fetcher.fetch(req).await.expect("fetch ok");
        assert_eq!(resp.status, 200);
    }

    #[tokio::test]
    async fn omits_credential_when_none() {
        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/api/public"))
            .respond_with(ResponseTemplate::new(200).set_body_string("public"))
            .mount(&server)
            .await;
        let client = std::sync::Arc::new(reqwest::Client::new());
        let fetcher = ResolverFetcher::new(client, server.uri());
        let req = FetchRequest {
            url: "/api/public".into(),
            method: "GET".into(),
            headers: HashMap::new(),
            body: None,
        };
        let resp = fetcher.fetch(req).await.expect("fetch ok");
        assert_eq!(resp.status, 200);
        // Verify the request had no Authorization header by inspecting wiremock recordings.
        let received = server.received_requests().await.unwrap();
        let auth_header = received[0].headers.get("authorization");
        assert!(auth_header.is_none());
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo test --lib ssr::fetcher_auth_tests`
Expected: FAIL — `UserCredential` and `with_user_credential` don't exist.

- [ ] **Step 3: Add UserCredential + extend ResolverFetcher**

In `doorway/doorway-service/src/ssr.rs`, after the existing imports add:

```rust
/// Opaque user credential the V8 fetch shim attaches to outbound storage fetches.
/// Constructed by doorway's session layer; the shim doesn't decode or interpret it.
#[derive(Debug, Clone)]
pub struct UserCredential {
    pub header_name: String,
    pub header_value: String,
}
```

Modify the `ResolverFetcher` struct:

```rust
pub struct ResolverFetcher {
    storage_base: String,
    client: Arc<reqwest::Client>,
    user_credential: Option<UserCredential>,
}

impl ResolverFetcher {
    pub fn new(client: Arc<reqwest::Client>, storage_base_url: String) -> Self {
        Self {
            storage_base: storage_base_url.trim_end_matches('/').to_string(),
            client,
            user_credential: None,
        }
    }

    /// Builder method: attach a per-render user credential.
    /// The shim will add this as a header to every outbound storage fetch.
    pub fn with_user_credential(mut self, credential: UserCredential) -> Self {
        self.user_credential = Some(credential);
        self
    }
}
```

In the `impl DataFetcher for ResolverFetcher` block, modify the `fetch` body to attach the credential. After the existing header-forwarding loop, before sending:

```rust
        // Attach the originating user's credential if present.
        if let Some(cred) = &self.user_credential {
            req_builder = req_builder.header(&cred.header_name, &cred.header_value);
        }
```

- [ ] **Step 4: Run the auth tests**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo test --lib ssr::fetcher_auth_tests`
Expected: PASS.

- [ ] **Step 5: Run the existing ssr tests to confirm no regression**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo test --lib ssr`
Expected: PASS (existing tests + new auth tests).

- [ ] **Step 6: Commit**

```bash
git add doorway/doorway-service/src/ssr.rs
git commit -m "feat(doorway): ResolverFetcher.with_user_credential — V8 auth threading"
```

---

### Task 17: Wire credential through the render dispatch

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` around line 1641-1727 (the SsrRoute dispatch arm)

- [ ] **Step 1: Identify the existing SSR dispatch site**

Run: `grep -n "ResolverFetcher::new" /projects/elohim/doorway/doorway-service/src/server/http.rs`
Expected: line ~1708 (per the spec) — the place where the fetcher is constructed per-request.

- [ ] **Step 2: Build credential from request session**

Just above the `Arc::new(crate::ssr::ResolverFetcher::new(...))` call, add session lookup:

```rust
        // Determine auth posture and build the user credential the SSR fetch shim
        // will attach to outbound storage fetches. Anonymous → None (no header).
        // Doorway-hosted → forward the request's Authorization or Cookie header.
        // Steward-presence is wired by M5; this path returns None today.
        let user_credential = build_ssr_user_credential(&request_headers);
```

Then change the fetcher construction:

```rust
        let fetcher = std::sync::Arc::new(
            crate::ssr::ResolverFetcher::new(
                state.ssr_http_client.clone(),
                storage_url.clone(),
            ).maybe_with_user_credential(user_credential.clone()),
        );
```

If `maybe_with_user_credential` doesn't exist on the fetcher, add this convenience method to `src/ssr.rs`:

```rust
impl ResolverFetcher {
    /// Convenience: apply credential only if Some.
    pub fn maybe_with_user_credential(self, cred: Option<UserCredential>) -> Self {
        match cred {
            Some(c) => self.with_user_credential(c),
            None => self,
        }
    }
}
```

- [ ] **Step 3: Add `build_ssr_user_credential` helper**

In `doorway/doorway-service/src/server/http.rs` (or in a new `src/server/ssr_session.rs` module if you want to keep `http.rs` from growing further), add:

```rust
use axum::http::HeaderMap;
use crate::ssr::UserCredential;

/// Build a `UserCredential` from the originating request's headers.
/// Returns None for anonymous requests (no Authorization, no session Cookie).
fn build_ssr_user_credential(headers: &HeaderMap) -> Option<UserCredential> {
    // Prefer Authorization (Bearer / API key). Fall back to Cookie session.
    if let Some(auth) = headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(value) = auth.to_str() {
            return Some(UserCredential {
                header_name: "Authorization".into(),
                header_value: value.into(),
            });
        }
    }
    if let Some(cookie) = headers.get(axum::http::header::COOKIE) {
        if let Ok(value) = cookie.to_str() {
            // Forward only if the request contains a known session cookie.
            // Doorway's existing session layer's cookie name should be referenced;
            // for now we forward the whole Cookie header (storage validates).
            if value.contains("doorway_session=") || value.contains("steward_attestation=") {
                return Some(UserCredential {
                    header_name: "Cookie".into(),
                    header_value: value.into(),
                });
            }
        }
    }
    None
}
```

- [ ] **Step 4: Add unit tests for `build_ssr_user_credential`**

In the same file, in a `#[cfg(test)] mod ssr_session_tests`:

```rust
#[cfg(test)]
mod ssr_session_tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn anonymous_request_returns_none() {
        let headers = HeaderMap::new();
        assert!(build_ssr_user_credential(&headers).is_none());
    }

    #[test]
    fn authorization_bearer_is_forwarded() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer token123"));
        let cred = build_ssr_user_credential(&headers).expect("some");
        assert_eq!(cred.header_name, "Authorization");
        assert_eq!(cred.header_value, "Bearer token123");
    }

    #[test]
    fn doorway_session_cookie_is_forwarded() {
        let mut headers = HeaderMap::new();
        headers.insert("cookie", HeaderValue::from_static("doorway_session=abc; theme=dark"));
        let cred = build_ssr_user_credential(&headers).expect("some");
        assert_eq!(cred.header_name, "Cookie");
        assert!(cred.header_value.contains("doorway_session=abc"));
    }

    #[test]
    fn unknown_cookies_are_not_forwarded() {
        let mut headers = HeaderMap::new();
        headers.insert("cookie", HeaderValue::from_static("theme=dark; locale=en"));
        assert!(build_ssr_user_credential(&headers).is_none());
    }
}
```

- [ ] **Step 5: Run the tests**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo test --lib server::http::ssr_session_tests`
Expected: PASS for all four.

- [ ] **Step 6: Run the full doorway test suite**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo test --lib --bins`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add doorway/doorway-service/src/
git commit -m "feat(doorway): thread user credential through SsrRoute dispatch"
```

---

## Phase 6: Auth-mode + concurrency enforcement

### Task 18: Auth-mode mismatch → CSR fallback

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` (SsrRoute dispatch arm)

- [ ] **Step 1: Define the auth posture from a request**

Add to `doorway/doorway-service/src/server/http.rs`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum AuthPosture {
    Anonymous,
    DoorwayHosted,
    StewardPresence,
}

/// Determine the auth posture of an incoming request based on its headers
/// and (later) the session layer's lookup of any presented credential.
pub fn determine_auth_posture(headers: &axum::http::HeaderMap) -> AuthPosture {
    if headers.get(axum::http::header::AUTHORIZATION).is_some() {
        return AuthPosture::DoorwayHosted; // JWT/Bearer is always doorway-hosted today
    }
    if let Some(cookie) = headers.get(axum::http::header::COOKIE) {
        if let Ok(s) = cookie.to_str() {
            if s.contains("steward_attestation=") {
                return AuthPosture::StewardPresence;
            }
            if s.contains("doorway_session=") {
                return AuthPosture::DoorwayHosted;
            }
        }
    }
    AuthPosture::Anonymous
}

impl AuthPosture {
    pub fn as_claim_str(&self) -> &'static str {
        match self {
            AuthPosture::Anonymous => "anonymous",
            AuthPosture::DoorwayHosted => "doorway-hosted",
            AuthPosture::StewardPresence => "steward-presence",
        }
    }
}
```

- [ ] **Step 2: Add unit tests for `determine_auth_posture`**

```rust
#[cfg(test)]
mod auth_posture_tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn no_headers_is_anonymous() {
        assert_eq!(determine_auth_posture(&HeaderMap::new()), AuthPosture::Anonymous);
    }

    #[test]
    fn bearer_is_doorway_hosted() {
        let mut h = HeaderMap::new();
        h.insert("authorization", HeaderValue::from_static("Bearer x"));
        assert_eq!(determine_auth_posture(&h), AuthPosture::DoorwayHosted);
    }

    #[test]
    fn steward_cookie_is_steward_presence() {
        let mut h = HeaderMap::new();
        h.insert("cookie", HeaderValue::from_static("steward_attestation=abc"));
        assert_eq!(determine_auth_posture(&h), AuthPosture::StewardPresence);
    }

    #[test]
    fn doorway_cookie_is_doorway_hosted() {
        let mut h = HeaderMap::new();
        h.insert("cookie", HeaderValue::from_static("doorway_session=xyz"));
        assert_eq!(determine_auth_posture(&h), AuthPosture::DoorwayHosted);
    }
}
```

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo test --lib server::http::auth_posture_tests`
Expected: PASS.

- [ ] **Step 3: Apply auth-mode enforcement in the dispatch**

In the SsrRoute dispatch arm in `server/http.rs` (around line 1641-1727), just before kicking off the V8 render:

```rust
        let posture = determine_auth_posture(&request_headers);
        let allowed = state
            .render_capability
            .as_ref()
            .map(|c| c.auth_modes.iter().any(|m| m == posture.as_claim_str()))
            .unwrap_or(false);
        if !allowed {
            tracing::info!(
                posture = ?posture,
                claim = ?state.render_capability.as_ref().map(|c| &c.auth_modes),
                "auth mode not in claim — falling back to CSR shell"
            );
            return csr_fallback_response(
                "auth-mode-not-supported",
                /* fallback HTML */ &state.csr_shell_html,
            );
        }
```

If `csr_fallback_response` and `state.csr_shell_html` don't already exist, add them:

```rust
fn csr_fallback_response(skip_reason: &'static str, shell_html: &str) -> axum::response::Response {
    use axum::http::{header, StatusCode};
    use axum::response::IntoResponse;
    let mut resp = (StatusCode::OK, axum::body::Body::from(shell_html.to_string())).into_response();
    let h = resp.headers_mut();
    h.insert("content-type", "text/html; charset=utf-8".parse().unwrap());
    h.insert("x-ssr-rendered", "0".parse().unwrap());
    h.insert("x-ssr-skipped", skip_reason.parse().unwrap());
    resp
}
```

(The CSR shell HTML is whatever the existing fallback path produces. If unclear, search for "fallback" or "shell" in `server/http.rs`.)

- [ ] **Step 4: Write an integration test**

Create `doorway/doorway-service/tests/auth_mode_enforcement.rs`:

```rust
use axum::http::{Request, StatusCode};
use doorway_service::render::types::{
    BundleEntry, RenderCapabilityProfile, RendererKind,
};
use tower::ServiceExt;

#[tokio::test]
async fn authenticated_request_to_anonymous_only_doorway_falls_back() {
    let profile = RenderCapabilityProfile {
        bundles: vec![BundleEntry {
            name: "lamad-app".into(),
            version: "1.0.3".into(),
            renderer: RendererKind::AngularSsr,
            digest: None,
        }],
        renderers: vec![RendererKind::AngularSsr],
        auth_modes: vec!["anonymous".into()],
        max_concurrent_renders: 4,
        memory_budget_mib: None,
    };
    let app = doorway_service::server::http::test_app_with_capability(Some(profile));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/lamad/concept/abc")
                .header("authorization", "Bearer user-token")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers();
    assert_eq!(headers.get("x-ssr-rendered").unwrap(), "0");
    assert_eq!(headers.get("x-ssr-skipped").unwrap(), "auth-mode-not-supported");
}

#[tokio::test]
async fn anonymous_request_to_anonymous_only_doorway_renders() {
    // Set up a doorway with anonymous in auth_modes and a working renderer.
    // (May need to mock the renderer to return a known body.)
    // Assert x-ssr-rendered: 1 and no x-ssr-skipped header.
    // Implementation depends on test harness — extend as needed.
}
```

- [ ] **Step 5: Run the integration tests**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo test --test auth_mode_enforcement`
Expected: PASS for the first test (the second is a placeholder for fuller harness work).

- [ ] **Step 6: Commit**

```bash
git add doorway/doorway-service/src/ doorway/doorway-service/tests/auth_mode_enforcement.rs
git commit -m "feat(doorway): auth-mode enforcement — mismatch falls back to CSR with x-ssr-skipped"
```

---

### Task 19: Concurrency limiter (semaphore)

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` (SsrRoute dispatch arm + AppState)

- [ ] **Step 1: Add the semaphore field to AppState**

In `AppState`:

```rust
    /// Concurrency limiter. Sized to render_capability.maxConcurrentRenders at startup.
    /// Semaphore::try_acquire is used at dispatch time; failure → CSR fallback.
    pub render_semaphore: Option<std::sync::Arc<tokio::sync::Semaphore>>,
```

In startup (where `render_capability` is wired):

```rust
let render_semaphore = render_capability.as_ref().map(|c| {
    std::sync::Arc::new(tokio::sync::Semaphore::new(c.max_concurrent_renders as usize))
});
```

Pass to `AppState`.

- [ ] **Step 2: Apply the semaphore in dispatch**

In `server/http.rs` SsrRoute dispatch, just before the V8 render call:

```rust
        let _permit = match state.render_semaphore.as_ref() {
            Some(sem) => match sem.clone().try_acquire_owned() {
                Ok(p) => Some(p),
                Err(_) => {
                    tracing::info!("render semaphore at limit — falling back to CSR shell");
                    return csr_fallback_response(
                        "overflow",
                        &state.csr_shell_html,
                    );
                }
            },
            None => None, // No capability → no limit (renderer absent will fall through anyway)
        };
```

The `_permit` is held until the function returns, releasing automatically.

- [ ] **Step 3: Write an integration test**

Create `doorway/doorway-service/tests/concurrency_overflow.rs`:

```rust
use axum::http::{Request, StatusCode};
use doorway_service::render::types::{
    BundleEntry, RenderCapabilityProfile, RendererKind,
};
use tower::ServiceExt;

#[tokio::test]
async fn requests_beyond_max_concurrent_get_csr_fallback() {
    let profile = RenderCapabilityProfile {
        bundles: vec![BundleEntry {
            name: "lamad-app".into(),
            version: "1.0.3".into(),
            renderer: RendererKind::AngularSsr,
            digest: None,
        }],
        renderers: vec![RendererKind::AngularSsr],
        auth_modes: vec!["anonymous".into()],
        max_concurrent_renders: 1,
        memory_budget_mib: None,
    };
    let app = doorway_service::server::http::test_app_with_capability_and_slow_renderer(
        Some(profile),
        std::time::Duration::from_millis(500),
    );

    // Fire 3 requests in parallel; at most 1 should render, the others should fall back.
    let make_req = || {
        app.clone().oneshot(
            Request::builder()
                .uri("/lamad/concept/abc")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
    };
    let (r1, r2, r3) = tokio::join!(make_req(), make_req(), make_req());
    let r1 = r1.unwrap();
    let r2 = r2.unwrap();
    let r3 = r3.unwrap();

    let skipped_count = [&r1, &r2, &r3].iter()
        .filter(|r| r.headers().get("x-ssr-skipped").map(|v| v == "overflow").unwrap_or(false))
        .count();
    assert!(skipped_count >= 2, "expected at least 2 of 3 to be CSR-fallback overflow, got {skipped_count}");
}
```

If `test_app_with_capability_and_slow_renderer` doesn't exist, add it to `server/http.rs` behind `#[cfg(test)]` — same shape as `test_app_with_capability` but with a stub renderer that sleeps.

- [ ] **Step 4: Run the test**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo test --test concurrency_overflow`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add doorway/doorway-service/src/ doorway/doorway-service/tests/concurrency_overflow.rs
git commit -m "feat(doorway): concurrency semaphore — overflow falls back to CSR"
```

---

### Task 20: Standardize observability headers

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` (everywhere SSR responses are built)

- [ ] **Step 1: Audit existing SSR header writes**

Run: `grep -n "x-render-cache\|x-ssr-\|x_ssr_" /projects/elohim/doorway/doorway-service/src/server/http.rs | head -20`
Expected: list of current locations setting SSR headers.

- [ ] **Step 2: On successful SSR render, add the four new headers**

In the success branch of the SsrRoute render (after the V8 render produces HTML), set:

```rust
        let bundle = state.render_capability.as_ref()
            .and_then(|c| c.bundles.first())
            .map(|b| format!("{}@{}", b.name, b.version))
            .unwrap_or_else(|| "unknown".to_string());
        let renderer = state.render_capability.as_ref()
            .and_then(|c| c.renderers.first())
            .map(|r| format!("{:?}", r).to_lowercase().replace('_', "-"))
            .unwrap_or_else(|| "angular-ssr".to_string());

        response.headers_mut().insert("x-ssr-rendered", "1".parse().unwrap());
        response.headers_mut().insert("x-ssr-renderer", renderer.parse().unwrap());
        response.headers_mut().insert("x-ssr-bundle-version", bundle.parse().unwrap());
        // x-render-cache is already set by the cache layer.
```

- [ ] **Step 3: Audit CSR fallback paths**

Make sure `csr_fallback_response()` always sets `x-ssr-rendered: 0` and an `x-ssr-skipped` reason. This was already done in Task 18 — verify no other CSR exit path lacks the headers.

- [ ] **Step 4: Add a smoke test that verifies headers**

Append to `doorway/doorway-service/tests/capability_publish.rs`:

```rust
#[tokio::test]
async fn ssr_response_carries_observability_headers() {
    let profile = RenderCapabilityProfile {
        bundles: vec![BundleEntry {
            name: "lamad-app".into(),
            version: "1.0.3".into(),
            renderer: RendererKind::AngularSsr,
            digest: None,
        }],
        renderers: vec![RendererKind::AngularSsr],
        auth_modes: vec!["anonymous".into()],
        max_concurrent_renders: 4,
        memory_budget_mib: None,
    };
    let app = doorway_service::server::http::test_app_with_capability(Some(profile));
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/lamad/concept/abc")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let headers = response.headers();
    // Either rendered (1) or skipped (0); both must be present
    assert!(headers.contains_key("x-ssr-rendered"));
    let rendered = headers.get("x-ssr-rendered").unwrap();
    if rendered == "1" {
        assert!(headers.contains_key("x-ssr-renderer"));
        assert!(headers.contains_key("x-ssr-bundle-version"));
    } else {
        assert!(headers.contains_key("x-ssr-skipped"));
    }
}
```

- [ ] **Step 5: Run**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo test --test capability_publish`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add doorway/doorway-service/src/ doorway/doorway-service/tests/capability_publish.rs
git commit -m "feat(doorway): standardize x-ssr-* observability headers"
```

---

## Phase 7: a2o + cross-stack integration

### Task 21: a2o feature file

**Files:**
- Create: `genesis/a2o/features/content/ssr_capability.feature`

- [ ] **Step 1: Write the feature file**

```gherkin
Feature: SSR capability is advertised, honored, and accountable

  As a substrate operator
  I want each doorway to declare its SSR capability honestly
  So that peers can see which doorways carry which bundles, support which auth modes, and have what concurrency budget — and authenticated requests never silently downgrade to anonymous renders.

  Background:
    Given a doorway is running with bundle "lamad-app@1.0.3" present on disk
    And elohim-storage manifest declares "/lamad/concept/{id}" with render="angular-ssr"

  Scenario: doorway exposes its derived capability at /admin/capability
    When I GET "/admin/capability" on the doorway
    Then the response body's "bundles[0].name" is "lamad-app"
    And the response body's "authModes" includes "anonymous"
    And the response body's "renderers" includes "angular-ssr"

  Scenario: storage layers the capability into the peer-status view
    Given storage is started with DOORWAY_CAPABILITY_URL set to the doorway's /admin/capability
    When I GET "/api/peers/<storage_peer_id>" on storage
    Then the response body's "renderCapability.bundles[0].name" is "lamad-app"

  Scenario: storage degrades honestly when DOORWAY_CAPABILITY_URL is unset
    Given storage is started with DOORWAY_CAPABILITY_URL unset
    When I GET "/api/peers/<storage_peer_id>" on storage
    Then the response body's "renderCapability" is null

  Scenario: authenticated request to anonymous-only doorway falls back to CSR
    Given the doorway's render capability is restricted to authModes=["anonymous"]
    When I GET "/lamad/concept/abc" on the doorway with header "Authorization: Bearer user-token"
    Then the response status is 200
    And the response header "x-ssr-rendered" is "0"
    And the response header "x-ssr-skipped" is "auth-mode-not-supported"

  Scenario: authenticated request renders when the doorway honors the auth mode
    Given the doorway's render capability includes authModes=["anonymous","doorway-hosted"]
    When I GET "/lamad/concept/abc" on the doorway with header "Authorization: Bearer user-token"
    Then the response status is 200
    And the response header "x-ssr-rendered" is "1"

  Scenario: capacity overflow falls back to CSR rather than queueing
    Given the doorway's render capability sets maxConcurrentRenders=1
    And the renderer takes longer than 100ms per render
    When I issue 3 concurrent GETs to "/lamad/concept/abc" on the doorway
    Then at least 2 responses have header "x-ssr-skipped" equal to "overflow"

  Scenario: peer A inspecting peer B sees B's render capability
    Given peer A's storage knows peer B via libp2p gossip
    And peer B's doorway publishes a render capability with bundle "lamad-app@1.0.3"
    When peer A GETs "/api/peers/<peer_b_id>" on its own storage
    Then the response body's "renderCapability.bundles[0].name" is "lamad-app"
```

- [ ] **Step 2: Run the a2o validator if it exists**

Run: `cd genesis && pnpm a2o validate features/content/ssr_capability.feature 2>&1 || echo 'a2o validator may not be wired here'`
Expected: passes or graceful no-op. Step is informational.

- [ ] **Step 3: Commit**

```bash
git add genesis/a2o/features/content/ssr_capability.feature
git commit -m "feat(a2o): SSR capability scenarios — claim publish + honor + fallback"
```

---

### Task 22: Cross-stack integration test (storage + doorway)

**Files:**
- Create: `elohim/elohim-storage/tests/render_capability_view.rs`

- [ ] **Step 1: Write the integration test**

```rust
//! Integration test: storage pulls render capability from a fake doorway HTTP
//! endpoint and surfaces it in PeerStatusView.

use elohim_storage::{
    build_peer_status_view, load_render_capability_from_url, BundleEntry,
    PeerStatusRow, RendererKind,
};
use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn storage_fetches_capability_and_layers_it_into_view() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("GET"))
        .and(matchers::path("/admin/capability"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "bundles": [{ "name": "lamad-app", "version": "1.0.3", "renderer": "angular-ssr" }],
            "renderers": ["angular-ssr"],
            "authModes": ["anonymous", "doorway-hosted"],
            "maxConcurrentRenders": 4
        })))
        .mount(&server)
        .await;
    std::env::set_var("DOORWAY_CAPABILITY_URL", format!("{}/admin/capability", server.uri()));
    let cap = load_render_capability_from_url().await.expect("loads");
    std::env::remove_var("DOORWAY_CAPABILITY_URL");

    let row = PeerStatusRow {
        peer_id: "peer-z".into(),
        status: "online".into(),
        general_pool_member: 1,
        accepting_stewardship_reserves: 0,
        archetype_class: Some("home-nuc".into()),
        timestamp: 1_700_000_000_000_000,
        dht_anchor_hash: "anchor-z".into(),
        updated_at: 1_700_000_000_000_000,
    };
    let view = build_peer_status_view(row, None, Some(&cap), None);
    let rc = view.render_capability.expect("layered");
    assert_eq!(rc.bundles.len(), 1);
    assert_eq!(rc.bundles[0].name, "lamad-app");
    assert_eq!(rc.bundles[0].renderer, RendererKind::AngularSsr);
    assert!(rc.auth_modes.contains(&"doorway-hosted".to_string()));
}

#[tokio::test]
async fn storage_returns_null_capability_when_doorway_unreachable() {
    std::env::set_var("DOORWAY_CAPABILITY_URL", "http://127.0.0.1:1/never");
    let cap = load_render_capability_from_url().await;
    std::env::remove_var("DOORWAY_CAPABILITY_URL");
    assert!(cap.is_none());
}
```

- [ ] **Step 2: Run**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test render_capability_view`
Expected: PASS for both tests.

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/tests/render_capability_view.rs
git commit -m "test(storage): cross-stack — pull capability + layer into PeerStatusView"
```

---

## Phase 8: Final verification

### Task 23: Pre-push verification across stack

**Files:** None (verification only)

- [ ] **Step 1: Schema artefacts fresh**

Run: `cd elohim/sdk && pnpm run schema:codegen:ts -- --verify`
Expected: no diff (codegen output matches committed files).

- [ ] **Step 2: Run schema tests**

Run: `cd elohim/sdk && pnpm run schema:test`
Expected: PASS.

- [ ] **Step 3: Storage build + tests**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins`
Expected: PASS.

- [ ] **Step 4: Doorway build + tests**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo build --release && RUSTFLAGS='' cargo test --lib --bins`
Expected: PASS.

- [ ] **Step 5: Clippy on both**

Run: `cd doorway/doorway-service && RUSTFLAGS='' cargo clippy -- -D warnings`
Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings`
Expected: clean (no warnings escalated to errors).

- [ ] **Step 6: Smoke-test the local stack manually**

Run (in three terminals):
- `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo run --release` (should bind :8090)
- `cd doorway/doorway-service && RUSTFLAGS='' STORAGE_URL=http://localhost:8090 SSR_BUNDLES_DIR=./bundles cargo run --release` (should bind :8888)
- `curl -s http://localhost:8888/admin/capability | jq` — expect a JSON profile or `null`
- `curl -s "http://localhost:8090/api/peers/<peer-id>" | jq '.renderCapability'` — expect the same profile (after starting storage with `DOORWAY_CAPABILITY_URL=http://localhost:8888/admin/capability`)

Document the manual smoke-test outcome in the next commit body if anything surprises you.

- [ ] **Step 7: Commit verification artefacts (if any) + push to dev**

```bash
git log --oneline | head -25  # confirm task commits are stacked cleanly
git push origin dev
```

---

## Notes for the executing engineer

- **DRY:** the schema/types are mirrored in three places: protocol JSON Schema, `elohim-storage/src/views.rs` ts-rs structs, and `doorway/doorway-service/src/render/types.rs`. They're independently authored (different language semantics) but their wire shapes must match — `tests/schema_contract.rs` is the drift detector.
- **YAGNI:** do NOT add storage-side peer-selection logic, P2P proxy, or substrate-routing. Those are explicitly deferred.
- **CSR fallback is the floor:** every miss/mismatch/overflow returns the existing CSR shell with `x-ssr-rendered: 0` + a reason header. Never serve a partial render or 503.
- **Auth modes are protocol-defined:** the enum is closed (`anonymous`, `doorway-hosted`, `steward-presence`). New modes require a schema bump — that's intended friction.
- **Steward-presence is wire-ready but not session-wired:** the schema and runtime know about it, but doorway's session layer doesn't produce a steward-attestation cookie yet. M5 wires that side; this plan doesn't.
- **`@elohim-bundle` header convention:** every SSR bundle's compile output must start with the `/* @elohim-bundle {...} */` banner. This is a doorway-side convention; coordinate with the elohim-app build pipeline if the existing bundle doesn't have one yet (a `prepend` step in the bundler config is the usual fix; the existing AngularRenderer setup likely already does this — verify before assuming).
