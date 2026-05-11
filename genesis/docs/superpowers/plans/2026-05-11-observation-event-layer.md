# Observation/Event Layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the substrate Observation primitive on Track 2 (libp2p+iroh) with per-observer iroh-blob log + cursor gossip + SQL projection + diversity summary view + graduation evaluator producing Attestations and summary EconomicEvents, then retire the conflated DHT entry types (DoorwayHeartbeat, DoorwayHeartbeatSummary, HealthAttestation).

**Architecture:** New Track 2 plane (`Observation`) alongside the existing ten, dual-stack libp2p+iroh with shared MessagePack wire format. Per-observer iroh-blob append-only log is source of truth; libp2p gossipsub propagates ~200-byte cursor announcements; receivers pull-fetch new segments via iroh-blob plane and project to local SQL. Per-pillar manifest declares observation_kinds (schema, retention_class, reach, diversity_threshold, graduates_to, graduation_policy). Per-pillar graduation evaluator runs as a tokio task inside elohim-storage, polls `observation_diversity_summary`, and emits Content (attestation graduation) or EconomicEvent (summary graduation) via existing coordinators. Zero new DHT entry types.

**Tech Stack:** Rust (elohim-storage, Holochain HDK/HDI), MessagePack/CBOR (rmp-serde) for wire, BLAKE3 (iroh-blobs) for chunking, libp2p gossipsub for cursor propagation, Diesel + SQLite for projection, TypeScript (ts-rs codegen + @elohim/storage-client), Angular 19 service (elohim-service).

**Spec:** `genesis/docs/superpowers/specs/2026-05-11-observation-event-layer-design.md`

**Prerequisite:** Wave 0 Stage A (attestation consolidation) must be landed before Stage 5 of this plan (graduation Path 1 depends on `content_type: "attestation:*"` discriminator pattern).

---

## File Structure

**Schemas & manifests (create):**
- `elohim/sdk/schemas/v1/manifest/observation-kind.schema.json` — observation_kind declaration shape
- `elohim/sdk/domains/infrastructure/manifest.json` — currently only `types/` exists
- `elohim/sdk/domains/mishpat/manifest.json` — currently does not exist

**Schemas & manifests (modify):**
- `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json` — add `observation_kinds` array
- `elohim/sdk/schemas/scripts/codegen-manifest.mjs` — emit `ObservationKindDeclaration` type
- `elohim/sdk/domains/{lamad,imagodei,shefa,qahal,avodah}/manifest.json` — add `observation_kinds`
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/feedback_signal.rs` — add `"forget-request"` to the bootstrap `signal_kind` whitelist constants

**Schemas & manifests (create — additional):**
- `elohim/sdk/domains/elohim/manifest.json` — new directory; home for cross-cutting elohim-pillar declarations (`forget-request` signal_kind, future cross-cutting kinds). The directory does not exist today.

**Rust substrate (create):**
- `elohim/elohim-storage/src/p2p/observation_protocol.rs` — libp2p protocol handler
- `elohim/elohim-storage/src/p2p/observation_gossip.rs` — gossipsub topic + cursor announcement
- `elohim/elohim-storage/src/p2p_iroh/observation_backend.rs` — iroh ALPN backend
- `elohim/elohim-storage/src/observation/mod.rs` — primitive types + log management
- `elohim/elohim-storage/src/observation/wire.rs` — `Observation` wire struct
- `elohim/elohim-storage/src/observation/log.rs` — iroh-blob append-only log
- `elohim/elohim-storage/src/observation/manager.rs` — `ObservationManagerBackend`
- `elohim/elohim-storage/src/observation/cursor.rs` — cursor tracking per peer
- `elohim/elohim-storage/src/graduation/mod.rs` — graduation evaluator
- `elohim/elohim-storage/src/graduation/attestation.rs` — Path 1 (observation → attestation)
- `elohim/elohim-storage/src/graduation/summary_event.rs` — Path 2 (observation → EconomicEvent)
- `elohim/elohim-storage/src/api/observations.rs` — HTTP routes

**Rust substrate (modify):**
- `elohim/elohim-storage/src/p2p_iroh/peer_map.rs` — extend `IrohPlane` and `Libp2pPlane` enums with `Observation`
- `elohim/elohim-storage/src/views.rs` — add `ObservationView`, `ObservationLogView`, `ObservationDiversitySummaryView`
- `elohim/elohim-storage/src/api/mod.rs` — register observation routes
- `elohim/elohim-storage/Cargo.toml` — (if needed) feature flags / deps

**Migrations (create):**
- `elohim/elohim-storage/migrations/2026-05-13-100000_observations/up.sql` + `down.sql`
- `elohim/elohim-storage/migrations/2026-05-13-110000_observation_diversity_view/up.sql` + `down.sql`
- `elohim/elohim-storage/migrations/2026-05-13-120000_retire_doorway_heartbeat_entries/up.sql` + `down.sql`

**Holochain (modify):**
- `elohim/holochain/dna/elohim/zomes/content_store_coordinator/src/lib.rs` — stake-class gate on `create_economic_event`
- `elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs` — REMOVE `DoorwayHeartbeat`, `DoorwayHeartbeatSummary`, `HealthAttestation`
- `elohim/holochain/dna/infrastructure/zomes/infrastructure_coordinator/src/lib.rs` — REMOVE corresponding handlers + `record_heartbeat`, `record_summary`, `issue_health_attestation`
- Sweettest scenarios in `elohim/holochain/tests/sweettest/` — remove tests that exercise removed entries

**Tests (create):**
- `elohim/elohim-storage/tests/observation_wire_test.rs` — encode/decode + signature
- `elohim/elohim-storage/tests/observation_log_test.rs` — append, fetch, replay
- `elohim/elohim-storage/tests/observation_parity_test.rs` — libp2p vs iroh parity (per Phase 11 pattern)
- `elohim/elohim-storage/tests/observation_graduation_test.rs` — both paths end-to-end
- `elohim/elohim-storage/tests/observation_diversity_test.rs` — summary view correctness

**Angular (create):**
- `app/elohim-library/projects/elohim-service/src/services/observation.service.ts`
- `app/elohim-library/projects/elohim-service/src/services/observation.service.spec.ts`

---

# Stage 1 — Manifest declarations

## Task 1.1: Add observation_kind schema

**Files:**
- Create: `elohim/sdk/schemas/v1/manifest/observation-kind.schema.json`
- Test: `elohim/sdk/schemas/scripts/test-observation-kind-schema.mjs`

- [ ] **Step 1: Write the failing schema test**

Create `elohim/sdk/schemas/scripts/test-observation-kind-schema.mjs`:

```javascript
import Ajv from 'ajv';
import addFormats from 'ajv-formats';
import schema from '../v1/manifest/observation-kind.schema.json' with { type: 'json' };

const ajv = new Ajv({ strict: true, allErrors: true });
addFormats(ajv);
const validate = ajv.compile(schema);

const minimal = {
  kind: 'infrastructure:doorway-heartbeat',
  namespace: 'elohim/observations/infrastructure',
  schema: { doorway_id: 'Cid', peer_count: 'u32' },
  retention_class: 'operational',
  reach: 'community'
};
if (!validate(minimal)) {
  console.error(validate.errors);
  process.exit(1);
}

const withGraduation = {
  ...minimal,
  diversity_threshold: { distinct_households: 3, min_count: 5 },
  graduates_to: 'attestation:doorway-health',
  graduation_window_seconds: 3600,
  graduation_policy: 'diversity-threshold'
};
if (!validate(withGraduation)) {
  console.error(validate.errors);
  process.exit(1);
}

const invalid_reach = { ...minimal, reach: 'invalid' };
if (validate(invalid_reach)) {
  console.error('Expected reach validation to fail');
  process.exit(1);
}

console.log('observation-kind.schema.json validates');
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `node elohim/sdk/schemas/scripts/test-observation-kind-schema.mjs`
Expected: fail with "Cannot find module ... observation-kind.schema.json"

- [ ] **Step 3: Create the schema**

Create `elohim/sdk/schemas/v1/manifest/observation-kind.schema.json`:

```json
{
  "$id": "epr:schema:manifest:observation-kind",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "ObservationKindDeclaration",
  "description": "Per-pillar manifest declaration of an observation_kind. See genesis/docs/superpowers/specs/2026-05-11-observation-event-layer-design.md §7.",
  "type": "object",
  "required": ["kind", "namespace", "schema", "retention_class", "reach"],
  "properties": {
    "kind": {
      "type": "string",
      "description": "Fully-qualified kind name, e.g. 'infrastructure:doorway-heartbeat'. Must include namespace prefix.",
      "pattern": "^[a-z][a-z0-9-]*:[a-z][a-z0-9-]*$"
    },
    "namespace": {
      "type": "string",
      "description": "Gossipsub topic namespace, e.g. 'elohim/observations/infrastructure'.",
      "pattern": "^elohim/observations/[a-z][a-z0-9-]*$"
    },
    "schema": {
      "type": "object",
      "description": "Field map from name to Rust type (Cid, u64, f32, String, bool). Used by codegen and validators.",
      "additionalProperties": { "type": "string" }
    },
    "retention_class": {
      "type": "string",
      "enum": ["operational", "contextual", "archival", "attestation-feeding", "wisdom"]
    },
    "reach": {
      "type": "string",
      "enum": ["agent-private", "household", "community", "commons", "commons-attested"]
    },
    "diversity_threshold": {
      "type": ["object", "null"],
      "properties": {
        "distinct_households": { "type": "integer", "minimum": 1 },
        "distinct_collectives": { "type": "integer", "minimum": 1 },
        "distinct_regions": { "type": "integer", "minimum": 1 },
        "distinct_archetypes": { "type": "integer", "minimum": 1 },
        "min_count": { "type": "integer", "minimum": 1 }
      }
    },
    "graduates_to": {
      "type": ["string", "null"],
      "description": "Optional. 'attestation:<subtype>' or 'event:<verb>' indicating graduation target."
    },
    "graduation_window_seconds": {
      "type": ["integer", "null"],
      "minimum": 1
    },
    "graduation_policy": {
      "type": ["string", "null"],
      "enum": [null, "self-threshold", "diversity-threshold", "summarize"]
    }
  },
  "additionalProperties": false
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `node elohim/sdk/schemas/scripts/test-observation-kind-schema.mjs`
Expected: prints `observation-kind.schema.json validates`

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/schemas/v1/manifest/observation-kind.schema.json \
        elohim/sdk/schemas/scripts/test-observation-kind-schema.mjs
git commit -m "schema(manifest): add observation-kind declaration schema"
```

---

## Task 1.2: Extend app-manifest schema with observation_kinds array

**Files:**
- Modify: `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`

- [ ] **Step 1: Locate the top-level `properties` block**

Run: `grep -n '"vocabulary"' elohim/sdk/schemas/v1/manifest/app-manifest.schema.json | head -3`

- [ ] **Step 2: Add observation_kinds property next to existing top-level properties**

Add after the `"projections"` entry in the top-level `"properties"` block:

```json
"observation_kinds": {
  "type": "array",
  "description": "Optional. Observation kinds this app's substrate emits. See observation-kind.schema.json and the observation-event-layer spec.",
  "items": { "$ref": "./observation-kind.schema.json" }
}
```

- [ ] **Step 3: Verify existing schema tests still pass**

Run: `pnpm run schema:test`
Expected: all existing tests PASS

- [ ] **Step 4: Commit**

```bash
git add elohim/sdk/schemas/v1/manifest/app-manifest.schema.json
git commit -m "schema(manifest): allow observation_kinds array at app-manifest top level"
```

---

## Task 1.3: Add observation_kinds to infrastructure manifest

**Files:**
- Modify: `elohim/sdk/domains/infrastructure/manifest.json` (already created by attestation Stage A — commit `0f63468fc`; contains `vocabulary.contentTypes`, `attestations` sections)

**Coordination note:** Before starting this task, confirm `git log --oneline dev | head -5` includes the attestation Stage A commit. If not, parallel session's merge to dev is still pending — DO NOT start this task yet.

- [ ] **Step 1: Read the existing manifest to know what to preserve**

Run: `cat elohim/sdk/domains/infrastructure/manifest.json`
Expected: object with `id`, `name`, `version`, `description`, `vocabulary.contentTypes`, `attestations` (2 entries). Do not delete or modify any of these — only ADD the `observation_kinds` array.

- [ ] **Step 2: Add the observation_kinds top-level array**

Insert at the top level of the JSON (after the existing top-level fields, before the closing brace):

```json
"observation_kinds": [
    {
      "kind": "infrastructure:doorway-heartbeat",
      "namespace": "elohim/observations/infrastructure",
      "schema": {
        "doorway_id": "Cid",
        "peer_count": "u32",
        "uptime_secs": "u64"
      },
      "retention_class": "operational",
      "reach": "community",
      "diversity_threshold": { "distinct_households": 3, "min_count": 5 },
      "graduates_to": "attestation:doorway-health",
      "graduation_window_seconds": 3600,
      "graduation_policy": "diversity-threshold"
    },
    {
      "kind": "infrastructure:blob-served",
      "namespace": "elohim/observations/infrastructure",
      "schema": {
        "blob_cid": "Cid",
        "bytes": "u64",
        "peer_cid": "Cid"
      },
      "retention_class": "operational",
      "reach": "community",
      "diversity_threshold": null,
      "graduates_to": "event:served-blob-summary",
      "graduation_window_seconds": 3600,
      "graduation_policy": "summarize"
    },
    {
      "kind": "infrastructure:system-sample",
      "namespace": "elohim/observations/infrastructure",
      "schema": {
        "cpu_pct": "f32",
        "mem_used_bytes": "u64",
        "disk_used_bytes": "u64",
        "disk_total_bytes": "u64"
      },
      "retention_class": "operational",
      "reach": "agent-private",
      "diversity_threshold": null,
      "graduates_to": null,
      "graduation_policy": null
    }
  ]
```

(Note: the snippet above is the array property only. Add a comma after the previous top-level field's closing brace, then paste the property. The trailing `}` of the manifest is unchanged.)

- [ ] **Step 3: Validate**

Run: `pnpm run schema:validate -- elohim/sdk/domains/infrastructure/manifest.json`
Expected: PASS — and the manifest still contains the pre-existing `attestations` array (do not let it be removed by your edit).

- [ ] **Step 4: Commit**

```bash
git add elohim/sdk/domains/infrastructure/manifest.json
git commit -m "manifest(infrastructure): add observation_kinds — doorway-heartbeat, blob-served, system-sample"
```

---

## Task 1.4: Add forget-decision subtype + observation_kinds to mishpat manifest

**Files:**
- Modify: `elohim/sdk/domains/mishpat/manifest.json` (already created by attestation Stage A — commit `0f63468fc`; contains `vocabulary.contentTypes`, `attestations` (5 entries), `governance-actions` (3 entries))

**Coordination note:** Same as Task 1.3 — confirm attestation Stage A is on dev before starting.

- [ ] **Step 1: Read existing manifest**

Run: `cat elohim/sdk/domains/mishpat/manifest.json`
Confirm it contains `attestations` and `governance-actions` arrays. Note the existing attestation subtypes.

- [ ] **Step 2: Add `forget-decision` to the existing `attestations` array**

Append a new entry to the existing `attestations` array (do NOT replace the array). Use the same shape as the existing entries (the schema is locked by the attestation consolidation spec — examine an existing entry to mirror its required fields). The new entry:

```json
{
  "subtype": "forget-decision",
  "description": "Mishpat-issued decision on a forget-request signal. See observation-event-layer spec §9.4.",
  "subject_kinds": ["agent", "content", "attestation"],
  "metadata_schema": "./schemas/attestation-forget-decision-metadata.schema.json"
}
```

If the existing `attestations` entry shape differs (e.g., uses different field names like `kind` instead of `subtype`), mirror the existing fields exactly. Use `cat` and pattern-match.

- [ ] **Step 3: Add the empty observation_kinds top-level array**

Append at top level (after existing top-level fields, before closing brace):

```json
"observation_kinds": []
```

Mishpat has no observation kinds in this sprint — but the declaration makes the manifest's intent explicit and lets the codegen run idempotently.

- [ ] **Step 4: Create the forget-decision metadata schema**

Create `elohim/sdk/domains/mishpat/schemas/attestation-forget-decision-metadata.schema.json`. Mirror the shape of an existing attestation-subtype metadata schema in the same directory (e.g., `attestation-gate-decision-metadata.schema.json` if present). Use:

```json
{
  "$id": "epr:manifest:mishpat:attestation:forget-decision:metadata",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "ForgetDecisionMetadata",
  "type": "object",
  "required": ["outcome", "reasoning_md", "constraint_applied"],
  "properties": {
    "outcome": {
      "type": "string",
      "enum": ["granted", "granted-with-redaction", "refused-per-constraint"]
    },
    "reasoning_md": { "type": "string", "minLength": 1 },
    "constraint_applied": {
      "type": "string",
      "description": "Manifest-declared constraint key that drove the decision (graduated-harm tier, accountability claim, evidence dependency)."
    },
    "redacted_fields": {
      "type": "array",
      "items": { "type": "string" }
    }
  },
  "additionalProperties": false
}
```

- [ ] **Step 5: Validate**

Run: `pnpm run schema:validate -- elohim/sdk/domains/mishpat/manifest.json`
Expected: PASS — including the new subtype and schema reference resolving correctly.

- [ ] **Step 6: Commit**

```bash
git add elohim/sdk/domains/mishpat/manifest.json \
        elohim/sdk/domains/mishpat/schemas/attestation-forget-decision-metadata.schema.json
git commit -m "manifest(mishpat): add forget-decision attestation subtype + observation_kinds slot"
```

---

## Task 1.5: Add observation_kinds to lamad, imagodei, shefa, qahal manifests

**Files:**
- Modify: `elohim/sdk/domains/lamad/manifest.json`
- Modify: `elohim/sdk/domains/imagodei/manifest.json`
- Modify: `elohim/sdk/domains/shefa/manifest.json`
- Modify: `elohim/sdk/domains/qahal/manifest.json`

- [ ] **Step 1: Add observation_kinds to lamad manifest**

Add at top level (after `projections` if present, else after `vocabulary`):

```json
"observation_kinds": [
  {
    "kind": "lamad:content-viewed",
    "namespace": "elohim/observations/lamad",
    "schema": {
      "ref_cid": "Cid",
      "dwell_ms": "u64",
      "scroll_depth_pct": "u8",
      "session_id": "Cid"
    },
    "retention_class": "contextual",
    "reach": "agent-private",
    "diversity_threshold": null,
    "graduates_to": null,
    "graduation_policy": null
  },
  {
    "kind": "lamad:mastery-check-result",
    "namespace": "elohim/observations/lamad",
    "schema": {
      "node_id": "Cid",
      "score": "f32",
      "hint_count": "u16"
    },
    "retention_class": "archival",
    "reach": "agent-private",
    "diversity_threshold": null,
    "graduates_to": "attestation:mastery",
    "graduation_window_seconds": 86400,
    "graduation_policy": "self-threshold"
  }
]
```

- [ ] **Step 2: Add observation_kinds to imagodei manifest**

```json
"observation_kinds": [
  {
    "kind": "imagodei:presence-tick",
    "namespace": "elohim/observations/imagodei",
    "schema": { "device_cid": "Cid" },
    "retention_class": "operational",
    "reach": "community",
    "diversity_threshold": null,
    "graduates_to": null,
    "graduation_policy": null
  }
]
```

- [ ] **Step 3: Add observation_kinds to shefa manifest**

```json
"observation_kinds": [
  {
    "kind": "shefa:appreciation-emitted",
    "namespace": "elohim/observations/shefa",
    "schema": { "recipient_cid": "Cid", "magnitude": "f32", "context_cid": "Cid" },
    "retention_class": "archival",
    "reach": "community",
    "diversity_threshold": null,
    "graduates_to": "event:appreciation-summary",
    "graduation_window_seconds": 3600,
    "graduation_policy": "summarize"
  }
]
```

- [ ] **Step 4: Add observation_kinds to qahal manifest (initial empty array)**

```json
"observation_kinds": []
```

- [ ] **Step 5: Validate all**

Run: `pnpm run schema:validate`
Expected: PASS for all pillar manifests

- [ ] **Step 6: Commit**

```bash
git add elohim/sdk/domains/{lamad,imagodei,shefa,qahal}/manifest.json
git commit -m "manifest: declare observation_kinds per pillar"
```

---

## Task 1.6: Create elohim manifest + add `forget-request` signal_kind

**Files:**
- Create: `elohim/sdk/domains/elohim/manifest.json` (directory does not exist today)
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/feedback_signal.rs` — extend the bootstrap signal_kind whitelist

- [ ] **Step 1: Create the elohim manifest**

```json
{
  "id": "manifest-elohim",
  "name": "elohim",
  "version": "1.0.0",
  "description": "Cross-cutting elohim-pillar declarations — signal_kinds that span pillars (forget-request, etc.).",
  "vocabulary": {
    "contentTypes": {},
    "signalKinds": {
      "forget-request": {
        "description": "Right-to-be-forgotten request flowing through the EPR feedback pipeline. See observation-event-layer spec §9.4.",
        "target_kinds": ["agent", "content", "attestation"],
        "evidence_required": false
      }
    }
  },
  "observation_kinds": []
}
```

- [ ] **Step 2: Validate the manifest**

Run: `pnpm run schema:validate -- elohim/sdk/domains/elohim/manifest.json`
Expected: PASS

- [ ] **Step 3: Locate the bootstrap signal_kind whitelist**

Run: `grep -n 'BOOTSTRAP_SIGNAL_KINDS\|whitelist\|"vouch"\|"squelch"' elohim/holochain/dna/elohim/zomes/content_store_integrity/src/feedback_signal.rs | head -10`

- [ ] **Step 4: Write the failing zome unit test**

In `feedback_signal.rs` (or its tests module), add:

```rust
#[test]
fn forget_request_is_accepted_signal_kind() {
    let sig = FeedbackSignal {
        signal_kind: "forget-request".to_string(),
        target_cid: "agent:bafy...".to_string(),
        standing_impact: "advisory".to_string(),
        squelch: false,
        correction: false,
        evidence_cid: None,
        rationale_md: None,
    };
    assert!(sig.signal_kind_is_valid(), "forget-request must be in bootstrap whitelist");
}
```

- [ ] **Step 5: Run to verify failure**

Run: `cd elohim/holochain/dna/elohim && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --target wasm32-unknown-unknown -p content_store_integrity forget_request_is_accepted_signal_kind 2>&1 | tail -10`
Expected: assertion fails

- [ ] **Step 6: Add `forget-request` to the whitelist**

Locate the bootstrap constants in `feedback_signal.rs` (named like `BOOTSTRAP_SIGNAL_KINDS` or inline match arms). Append `"forget-request"` to the accepted set.

- [ ] **Step 7: Run to verify pass**

Run: same command as Step 5
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add elohim/sdk/domains/elohim/manifest.json \
        elohim/holochain/dna/elohim/zomes/content_store_integrity/src/feedback_signal.rs
git commit -m "manifest(elohim): declare forget-request signal_kind for observation forget flow"
```

---

## Task 1.7: Regenerate manifest TypeScript types

**Files:**
- Modify: `elohim/sdk/schemas/scripts/codegen-manifest.mjs` (if it needs to know about observation-kind.schema)
- Modify: generated `manifest-types.ts`

- [ ] **Step 1: Inspect codegen script**

Run: `grep -n 'interface\|EXPORT\|TYPES_TO_EMIT' elohim/sdk/schemas/scripts/codegen-manifest.mjs | head -20`

- [ ] **Step 2: Add ObservationKindDeclaration to codegen if not auto-discovered**

If the script auto-discovers all schemas in `v1/manifest/`, this step is a no-op. Otherwise, add `observation-kind.schema.json` to the input list and `ObservationKindDeclaration` to the export list.

- [ ] **Step 3: Run codegen**

Run: `pnpm run lamad:codegen`
Expected: success; `manifest-types.ts` includes `ObservationKindDeclaration` interface

- [ ] **Step 4: Verify the type is exported**

Run: `grep -n 'ObservationKindDeclaration' app/elohim-library/projects/elohim-service/src/generated/manifest-types.ts`
Expected: interface definition with all fields from the schema

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/schemas/scripts/codegen-manifest.mjs \
        app/elohim-library/projects/elohim-service/src/generated/manifest-types.ts
git commit -m "codegen(manifest): emit ObservationKindDeclaration type"
```

---

# Stage 2 — Wire format and ALPN

## Task 2.1: Observation wire struct

**Files:**
- Create: `elohim/elohim-storage/src/observation/mod.rs`
- Create: `elohim/elohim-storage/src/observation/wire.rs`
- Modify: `elohim/elohim-storage/src/lib.rs` — add `pub mod observation;`
- Test: `elohim/elohim-storage/tests/observation_wire_test.rs`

Set CARGO_TARGET_DIR before any cargo commands:
```bash
export CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev
```

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-storage/tests/observation_wire_test.rs`:

```rust
use elohim_storage::observation::wire::{Observation, ObservationKind};
use rmp_serde;

#[test]
fn observation_roundtrips_msgpack() {
    let obs = Observation {
        observer_cid: "agent:bafkrei...".to_string(),
        log_cid: "blake3:abc123".to_string(),
        log_offset: 42,
        observed_at: 1715420400,
        seq: 100,
        observation_kind: "infrastructure:doorway-heartbeat".to_string(),
        subject_cid: Some("doorway:bafkrei...".to_string()),
        subject_kind: Some("doorway".to_string()),
        payload_json: r#"{"peer_count":12,"uptime_secs":86400}"#.to_string(),
        observer_household_cid: Some("household:bafkrei...".to_string()),
        observer_collective_cid: None,
        observer_region: Some("us-west".to_string()),
        observer_archetype: Some("tier-1-hub".to_string()),
        observer_compute_class: Some("rpi4-arm64".to_string()),
        signature: vec![0x01, 0x02, 0x03, 0x04],
    };
    let bytes = rmp_serde::to_vec(&obs).expect("encode");
    let decoded: Observation = rmp_serde::from_slice(&bytes).expect("decode");
    assert_eq!(decoded.observer_cid, obs.observer_cid);
    assert_eq!(decoded.log_offset, 42);
    assert_eq!(decoded.signature, vec![0x01, 0x02, 0x03, 0x04]);
}

#[test]
fn canonical_encoding_is_deterministic() {
    let obs = Observation::test_fixture();
    let bytes1 = rmp_serde::to_vec(&obs).unwrap();
    let bytes2 = rmp_serde::to_vec(&obs).unwrap();
    assert_eq!(bytes1, bytes2, "encoding must be deterministic for signing");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cd elohim/elohim-storage && cargo test observation_wire_test`
Expected: compile error — `observation` module does not exist

- [ ] **Step 3: Create the module skeleton**

Create `elohim/elohim-storage/src/observation/mod.rs`:

```rust
pub mod wire;
```

Create `elohim/elohim-storage/src/observation/wire.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    pub observer_cid: String,
    pub log_cid: String,
    pub log_offset: u64,
    pub observed_at: i64,
    pub seq: u64,
    pub observation_kind: String,
    pub subject_cid: Option<String>,
    pub subject_kind: Option<String>,
    pub payload_json: String,
    pub observer_household_cid: Option<String>,
    pub observer_collective_cid: Option<String>,
    pub observer_region: Option<String>,
    pub observer_archetype: Option<String>,
    pub observer_compute_class: Option<String>,
    pub signature: Vec<u8>,
}

#[cfg(test)]
impl Observation {
    pub fn test_fixture() -> Self {
        Self {
            observer_cid: "agent:test".into(),
            log_cid: "blake3:test".into(),
            log_offset: 0,
            observed_at: 1715420400,
            seq: 0,
            observation_kind: "infrastructure:doorway-heartbeat".into(),
            subject_cid: None,
            subject_kind: None,
            payload_json: "{}".into(),
            observer_household_cid: None,
            observer_collective_cid: None,
            observer_region: None,
            observer_archetype: None,
            observer_compute_class: None,
            signature: vec![],
        }
    }
}

pub enum ObservationKind {}
```

Add to `elohim/elohim-storage/src/lib.rs`:

```rust
pub mod observation;
```

- [ ] **Step 4: Run the test to verify pass**

Run: `cargo test --test observation_wire_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/observation/ \
        elohim/elohim-storage/src/lib.rs \
        elohim/elohim-storage/tests/observation_wire_test.rs
git commit -m "feat(observation): add wire format struct with msgpack roundtrip"
```

---

## Task 2.2: Canonical encoding + signature scope

**Files:**
- Modify: `elohim/elohim-storage/src/observation/wire.rs`
- Test: extend `elohim/elohim-storage/tests/observation_wire_test.rs`

- [ ] **Step 1: Add the canonical-bytes-for-signing test**

Add to `tests/observation_wire_test.rs`:

```rust
#[test]
fn signature_bytes_exclude_signature_field() {
    let mut obs = Observation::test_fixture();
    obs.signature = vec![0xff; 64];
    let bytes_with_dummy_sig = obs.canonical_signing_bytes();

    obs.signature = vec![];
    let bytes_with_empty_sig = obs.canonical_signing_bytes();

    assert_eq!(
        bytes_with_dummy_sig, bytes_with_empty_sig,
        "canonical_signing_bytes must not depend on the signature field"
    );
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test observation_wire_test signature_bytes_exclude_signature_field`
Expected: compile error — `canonical_signing_bytes` method missing

- [ ] **Step 3: Add the method**

Add to `wire.rs`:

```rust
impl Observation {
    pub fn canonical_signing_bytes(&self) -> Vec<u8> {
        let mut clone = self.clone();
        clone.signature = vec![];
        rmp_serde::to_vec(&clone).expect("msgpack encoding of Observation is infallible for owned data")
    }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --test observation_wire_test`
Expected: PASS (all)

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/observation/wire.rs \
        elohim/elohim-storage/tests/observation_wire_test.rs
git commit -m "feat(observation): canonical signing bytes exclude signature field"
```

---

## Task 2.3: Register Plane::Observation in peer transport manifest

**Files:**
- Modify: `elohim/elohim-storage/src/p2p_iroh/peer_map.rs`
- Test: `elohim/elohim-storage/tests/observation_plane_registration_test.rs`

- [ ] **Step 1: Locate the existing plane enums**

Run: `grep -n 'enum IrohPlane\|enum Libp2pPlane' elohim/elohim-storage/src/p2p_iroh/peer_map.rs`

- [ ] **Step 2: Write the failing test**

Create `elohim/elohim-storage/tests/observation_plane_registration_test.rs`:

```rust
use elohim_storage::p2p_iroh::peer_map::{IrohPlane, Libp2pPlane};

#[test]
fn iroh_plane_has_observation_variant() {
    let _ = IrohPlane::Observation;
}

#[test]
fn libp2p_plane_has_observation_variant() {
    let _ = Libp2pPlane::Observation;
}

#[test]
fn observation_plane_serializes_stably() {
    let json = serde_json::to_string(&IrohPlane::Observation).unwrap();
    assert_eq!(json, "\"Observation\"");
}
```

- [ ] **Step 3: Run to verify failure**

Run: `cargo test --test observation_plane_registration_test`
Expected: compile error — `Observation` variant missing

- [ ] **Step 4: Add Observation variant to both enums**

In `peer_map.rs`, append `Observation,` to each enum.

- [ ] **Step 5: Run to verify pass**

Run: `cargo test --test observation_plane_registration_test`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/p2p_iroh/peer_map.rs \
        elohim/elohim-storage/tests/observation_plane_registration_test.rs
git commit -m "feat(observation): register Plane::Observation in cross-stack peer map"
```

---

## Task 2.4: ALPN constants

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/topics.rs` (libp2p ALPN + gossipsub topic prefix)
- Modify: `elohim/elohim-storage/src/p2p_iroh/codec.rs` (iroh ALPN constant)

- [ ] **Step 1: Add the ALPN constants**

In `p2p/topics.rs`:
```rust
pub const OBSERVATION_LOG_PROTOCOL: &str = "/elohim/observation/1.0.0";
pub const OBSERVATION_GOSSIP_TOPIC_PREFIX: &str = "elohim/observations/";
```

In `p2p_iroh/codec.rs`:
```rust
pub const IROH_OBSERVATION_ALPN: &[u8] = b"iroh-observation/1";
```

- [ ] **Step 2: Run the existing test suite to confirm no regressions**

Run: `cargo test --lib`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/src/p2p/topics.rs \
        elohim/elohim-storage/src/p2p_iroh/codec.rs
git commit -m "feat(observation): declare libp2p protocol id and iroh ALPN"
```

---

# Stage 3 — Storage tables

## Task 3.1: Observation tables migration

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-05-13-100000_observations/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-05-13-100000_observations/down.sql`

- [ ] **Step 1: Create up.sql**

```sql
-- Observation/Event Layer — Stage 3 of the observation-event-layer-design spec.
-- Source of truth: iroh-blob log (per-observer, content-addressed). Classification: C.
-- The substrate primitive for peer-witnessed evidence on Track 2 (libp2p+iroh).
-- See genesis/docs/superpowers/specs/2026-05-11-observation-event-layer-design.md

-- One row per observation. Reconstructable by replaying the observer's iroh-blob log.
CREATE TABLE observations (
    observer_cid              TEXT    NOT NULL,
    log_cid                   TEXT    NOT NULL,
    log_offset                BIGINT  NOT NULL,
    observed_at               BIGINT  NOT NULL,
    seq                       BIGINT  NOT NULL,
    observation_kind          TEXT    NOT NULL,
    subject_cid               TEXT,
    subject_kind              TEXT,
    payload_json              TEXT    NOT NULL,
    observer_household_cid    TEXT,
    observer_collective_cid   TEXT,
    observer_region           TEXT,
    observer_archetype        TEXT,
    observer_compute_class    TEXT,
    signature_b64             TEXT    NOT NULL,
    PRIMARY KEY (observer_cid, log_cid, log_offset)
);

CREATE INDEX observations_by_subject_kind
    ON observations (subject_cid, observation_kind, observed_at);

CREATE INDEX observations_by_kind_time
    ON observations (observation_kind, observed_at);

CREATE INDEX observations_by_observer_seq
    ON observations (observer_cid, seq);

-- Per-observer log roster — what is the latest log root for each observer we follow.
-- Source of truth: observer's iroh-blob log. Classification: C.
CREATE TABLE observation_logs (
    observer_cid       TEXT     PRIMARY KEY,
    latest_log_cid     TEXT     NOT NULL,
    latest_offset      BIGINT   NOT NULL,
    retention_class    TEXT     NOT NULL,
    last_attested_at   BIGINT
);

-- Per-(observer, viewer) cursor — how far this viewer has projected each observer's log.
-- Source of truth: SQLite (operational). Classification: C.
-- Mirrors the existing projector_cursor / peer_inventory_cursor pattern.
CREATE TABLE observation_cursors (
    observer_cid             TEXT    NOT NULL,
    viewer_peer_id           TEXT    NOT NULL,
    last_projected_offset    BIGINT  NOT NULL,
    last_seen_at             BIGINT  NOT NULL,
    PRIMARY KEY (observer_cid, viewer_peer_id)
);

-- Audit log of verify-path queries (point-in-time diversity re-checks).
-- Source of truth: SQLite (operational). Classification: C.
CREATE TABLE audit_observations (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    requested_at      BIGINT  NOT NULL,
    requester_cid     TEXT    NOT NULL,
    subject_cid       TEXT    NOT NULL,
    observation_kind  TEXT    NOT NULL,
    result_json       TEXT    NOT NULL
);
```

- [ ] **Step 2: Create down.sql**

```sql
DROP TABLE IF EXISTS audit_observations;
DROP TABLE IF EXISTS observation_cursors;
DROP TABLE IF EXISTS observation_logs;
DROP INDEX IF EXISTS observations_by_observer_seq;
DROP INDEX IF EXISTS observations_by_kind_time;
DROP INDEX IF EXISTS observations_by_subject_kind;
DROP TABLE IF EXISTS observations;
```

- [ ] **Step 3: Run migrations**

Run: `cd elohim/elohim-storage && CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo build --lib 2>&1 | tail -20`
Expected: build succeeds; diesel auto-regenerates schema.rs

- [ ] **Step 4: Verify schema regen**

Run: `grep -n 'observations\|observation_logs\|observation_cursors\|audit_observations' elohim/elohim-storage/src/db/schema.rs`
Expected: four table definitions present

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-05-13-100000_observations/ \
        elohim/elohim-storage/src/db/schema.rs
git commit -m "migration(observation): create observations, logs, cursors, audit tables"
```

---

## Task 3.2: Diversity summary view

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-05-13-110000_observation_diversity_view/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-05-13-110000_observation_diversity_view/down.sql`

- [ ] **Step 1: Create up.sql**

```sql
-- Materialized as a view (SQLite re-evaluates on each query — cheap for our scale).
-- If query cost dominates later, swap to a materialized table refreshed by tokio task.
-- Source of truth: aggregation over observations table. Classification: C.

CREATE VIEW observation_diversity_summary AS
SELECT
    subject_cid,
    observation_kind,
    COUNT(DISTINCT observer_cid)              AS distinct_agents,
    COUNT(DISTINCT observer_household_cid)    AS distinct_households,
    COUNT(DISTINCT observer_collective_cid)   AS distinct_collectives,
    COUNT(DISTINCT observer_region)           AS distinct_regions,
    COUNT(DISTINCT observer_archetype)        AS distinct_archetypes,
    COUNT(DISTINCT observer_compute_class)    AS distinct_compute_classes,
    COUNT(*)                                  AS total_count,
    MIN(observed_at)                          AS first_observed_at,
    MAX(observed_at)                          AS last_observed_at
FROM observations
WHERE subject_cid IS NOT NULL
GROUP BY subject_cid, observation_kind;
```

- [ ] **Step 2: Create down.sql**

```sql
DROP VIEW IF EXISTS observation_diversity_summary;
```

- [ ] **Step 3: Run migration**

Run: `cargo build --lib`
Expected: success

- [ ] **Step 4: Add the view to diesel schema**

The view doesn't auto-generate in schema.rs; add it manually:

In `elohim/elohim-storage/src/db/schema.rs`, add:

```rust
diesel::table! {
    observation_diversity_summary (subject_cid, observation_kind) {
        subject_cid -> Text,
        observation_kind -> Text,
        distinct_agents -> BigInt,
        distinct_households -> BigInt,
        distinct_collectives -> BigInt,
        distinct_regions -> BigInt,
        distinct_archetypes -> BigInt,
        distinct_compute_classes -> BigInt,
        total_count -> BigInt,
        first_observed_at -> BigInt,
        last_observed_at -> BigInt,
    }
}
```

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-05-13-110000_observation_diversity_view/ \
        elohim/elohim-storage/src/db/schema.rs
git commit -m "migration(observation): create observation_diversity_summary view"
```

---

## Task 3.3: Rust View types with ts-rs

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs`

- [ ] **Step 1: Write the failing export-bindings test**

Add to `views.rs` (or wherever the existing view types live):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS, Queryable)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ObservationView {
    pub observer_cid: String,
    pub log_cid: String,
    pub log_offset: i64,
    pub observed_at: i64,
    pub seq: i64,
    pub observation_kind: String,
    pub subject_cid: Option<String>,
    pub subject_kind: Option<String>,
    pub payload_json: String,
    pub observer_household_cid: Option<String>,
    pub observer_collective_cid: Option<String>,
    pub observer_region: Option<String>,
    pub observer_archetype: Option<String>,
    pub observer_compute_class: Option<String>,
    pub signature_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Queryable)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ObservationDiversitySummaryView {
    pub subject_cid: String,
    pub observation_kind: String,
    pub distinct_agents: i64,
    pub distinct_households: i64,
    pub distinct_collectives: i64,
    pub distinct_regions: i64,
    pub distinct_archetypes: i64,
    pub distinct_compute_classes: i64,
    pub total_count: i64,
    pub first_observed_at: i64,
    pub last_observed_at: i64,
}
```

- [ ] **Step 2: Run export bindings**

Run: `cargo test --lib export_bindings`
Expected: PASS; new `.ts` files emitted in `elohim/sdk/storage-client-ts/src/generated/`

- [ ] **Step 3: Confirm TS files exist**

Run: `ls elohim/sdk/storage-client-ts/src/generated/ObservationView.ts elohim/sdk/storage-client-ts/src/generated/ObservationDiversitySummaryView.ts`
Expected: both files present

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/views.rs \
        elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(observation): add view types with ts-rs export"
```

---

# Stage 4 — Backend manager service

## Task 4.1: ObservationLog primitive (in-memory + iroh-blob backed)

**Files:**
- Create: `elohim/elohim-storage/src/observation/log.rs`
- Modify: `elohim/elohim-storage/src/observation/mod.rs`
- Test: `elohim/elohim-storage/tests/observation_log_test.rs`

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-storage/tests/observation_log_test.rs`:

```rust
use elohim_storage::observation::log::{ObservationLog, ObservationLogError};
use elohim_storage::observation::wire::Observation;

#[tokio::test]
async fn append_then_read_returns_observations_in_order() {
    let mut log = ObservationLog::new_in_memory("agent:test".into());
    let obs1 = Observation { seq: 1, ..Observation::test_fixture() };
    let obs2 = Observation { seq: 2, ..Observation::test_fixture() };
    log.append(obs1.clone()).await.unwrap();
    log.append(obs2.clone()).await.unwrap();

    let all = log.read_from(0).await.unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].seq, 1);
    assert_eq!(all[1].seq, 2);
}

#[tokio::test]
async fn appending_advances_log_cid() {
    let mut log = ObservationLog::new_in_memory("agent:test".into());
    let initial_root = log.current_log_cid();
    log.append(Observation::test_fixture()).await.unwrap();
    let after_append = log.current_log_cid();
    assert_ne!(initial_root, after_append, "log_cid must advance after append");
}

#[tokio::test]
async fn read_from_offset_skips_earlier_rows() {
    let mut log = ObservationLog::new_in_memory("agent:test".into());
    for i in 0..5u64 {
        log.append(Observation { seq: i, ..Observation::test_fixture() }).await.unwrap();
    }
    let tail = log.read_from(3).await.unwrap();
    assert_eq!(tail.len(), 2);
    assert_eq!(tail[0].seq, 3);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test observation_log_test`
Expected: compile error — `log` module missing

- [ ] **Step 3: Create the log module**

Create `elohim/elohim-storage/src/observation/log.rs`:

```rust
use crate::observation::wire::Observation;
use blake3;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObservationLogError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("encoding error: {0}")]
    Encoding(String),
}

/// Per-observer append-only log. In-memory backed for unit tests; production
/// backing is iroh-blobs via the IrohBackend trait (Task 4.4).
pub struct ObservationLog {
    observer_cid: String,
    entries: Vec<Observation>,
    rolling_hasher: blake3::Hasher,
    current_root: String,
}

impl ObservationLog {
    pub fn new_in_memory(observer_cid: String) -> Self {
        Self {
            observer_cid,
            entries: Vec::new(),
            rolling_hasher: blake3::Hasher::new(),
            current_root: "blake3:".to_string() + &blake3::Hasher::new().finalize().to_hex().to_string(),
        }
    }

    pub fn observer_cid(&self) -> &str { &self.observer_cid }

    pub fn current_log_cid(&self) -> String { self.current_root.clone() }

    pub fn latest_offset(&self) -> u64 { self.entries.len() as u64 }

    pub async fn append(&mut self, obs: Observation) -> Result<(), ObservationLogError> {
        let bytes = rmp_serde::to_vec(&obs).map_err(|e| ObservationLogError::Encoding(e.to_string()))?;
        self.rolling_hasher.update(&bytes);
        self.current_root = format!("blake3:{}", self.rolling_hasher.finalize().to_hex());
        self.entries.push(obs);
        Ok(())
    }

    pub async fn read_from(&self, offset: u64) -> Result<Vec<Observation>, ObservationLogError> {
        Ok(self.entries.iter().skip(offset as usize).cloned().collect())
    }
}
```

Update `elohim/elohim-storage/src/observation/mod.rs`:

```rust
pub mod log;
pub mod wire;
```

Add `thiserror` and `blake3` to `Cargo.toml` dependencies if not present (usually already there).

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --test observation_log_test`
Expected: PASS (all three)

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/observation/log.rs \
        elohim/elohim-storage/src/observation/mod.rs \
        elohim/elohim-storage/tests/observation_log_test.rs
git commit -m "feat(observation): in-memory ObservationLog with BLAKE3 rolling root"
```

---

## Task 4.2: SQL projection writer

**Files:**
- Create: `elohim/elohim-storage/src/observation/projector.rs`
- Modify: `elohim/elohim-storage/src/observation/mod.rs`
- Test: `elohim/elohim-storage/tests/observation_projector_test.rs`

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-storage/tests/observation_projector_test.rs`:

```rust
use elohim_storage::db::test_helpers::test_connection;
use elohim_storage::observation::projector::ObservationProjector;
use elohim_storage::observation::wire::Observation;

#[test]
fn project_writes_row_then_query_returns_it() {
    let mut conn = test_connection();
    let projector = ObservationProjector::new();

    let obs = Observation {
        subject_cid: Some("doorway:abc".into()),
        observation_kind: "infrastructure:doorway-heartbeat".into(),
        ..Observation::test_fixture()
    };

    projector.project(&mut conn, &obs).unwrap();

    let rows = projector.by_subject(&mut conn, "doorway:abc", "infrastructure:doorway-heartbeat").unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].observer_cid, obs.observer_cid);
}

#[test]
fn idempotent_on_same_pk() {
    let mut conn = test_connection();
    let projector = ObservationProjector::new();
    let obs = Observation::test_fixture();

    projector.project(&mut conn, &obs).unwrap();
    projector.project(&mut conn, &obs).unwrap();

    let rows = projector.all(&mut conn).unwrap();
    assert_eq!(rows.len(), 1, "second project of same PK must be a no-op");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test observation_projector_test`
Expected: compile error — `projector` module missing

- [ ] **Step 3: Create the projector**

Create `elohim/elohim-storage/src/observation/projector.rs`:

```rust
use crate::db::schema::observations;
use crate::observation::wire::Observation;
use crate::views::ObservationView;
use base64::Engine;
use diesel::prelude::*;

pub struct ObservationProjector;

impl ObservationProjector {
    pub fn new() -> Self { Self }

    pub fn project(&self, conn: &mut SqliteConnection, obs: &Observation) -> Result<(), diesel::result::Error> {
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(&obs.signature);
        diesel::insert_or_ignore_into(observations::table)
            .values((
                observations::observer_cid.eq(&obs.observer_cid),
                observations::log_cid.eq(&obs.log_cid),
                observations::log_offset.eq(obs.log_offset as i64),
                observations::observed_at.eq(obs.observed_at),
                observations::seq.eq(obs.seq as i64),
                observations::observation_kind.eq(&obs.observation_kind),
                observations::subject_cid.eq(&obs.subject_cid),
                observations::subject_kind.eq(&obs.subject_kind),
                observations::payload_json.eq(&obs.payload_json),
                observations::observer_household_cid.eq(&obs.observer_household_cid),
                observations::observer_collective_cid.eq(&obs.observer_collective_cid),
                observations::observer_region.eq(&obs.observer_region),
                observations::observer_archetype.eq(&obs.observer_archetype),
                observations::observer_compute_class.eq(&obs.observer_compute_class),
                observations::signature_b64.eq(sig_b64),
            ))
            .execute(conn)?;
        Ok(())
    }

    pub fn by_subject(&self, conn: &mut SqliteConnection, subject: &str, kind: &str) -> Result<Vec<ObservationView>, diesel::result::Error> {
        observations::table
            .filter(observations::subject_cid.eq(subject))
            .filter(observations::observation_kind.eq(kind))
            .order(observations::observed_at.desc())
            .load::<ObservationView>(conn)
    }

    pub fn all(&self, conn: &mut SqliteConnection) -> Result<Vec<ObservationView>, diesel::result::Error> {
        observations::table.load::<ObservationView>(conn)
    }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --test observation_projector_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/observation/projector.rs \
        elohim/elohim-storage/src/observation/mod.rs \
        elohim/elohim-storage/tests/observation_projector_test.rs
git commit -m "feat(observation): SQL projector with idempotent insert"
```

---

## Task 4.3: Cursor announcement gossipsub topic

**Files:**
- Create: `elohim/elohim-storage/src/p2p/observation_gossip.rs`
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` — add `pub mod observation_gossip;`
- Test: `elohim/elohim-storage/tests/observation_gossip_test.rs`

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-storage/tests/observation_gossip_test.rs`:

```rust
use elohim_storage::p2p::observation_gossip::{CursorAnnouncement, observation_topic};

#[test]
fn topic_for_kind_includes_namespace_only() {
    assert_eq!(
        observation_topic("infrastructure:doorway-heartbeat"),
        "elohim/observations/infrastructure"
    );
    assert_eq!(
        observation_topic("lamad:mastery-check-result"),
        "elohim/observations/lamad"
    );
}

#[test]
fn cursor_announcement_serializes_to_msgpack_under_512_bytes() {
    let ann = CursorAnnouncement {
        observer_cid: "agent:bafyreigdyrzt5sfbtgnnwphhbofgw57x3sjyiyq2r4f4tymqxznkjkzgfa".into(),
        kind: "infrastructure:doorway-heartbeat".into(),
        log_cid: "blake3:abc123def456".into(),
        latest_offset: 12345,
        subject_cid: Some("doorway:bafyreigdyrzt5sfbtgnnwphhbofgw57x3sjyiyq2r4f4tymqxznkjkzgfa".into()),
        observed_at_window: 1715420400,
    };
    let bytes = rmp_serde::to_vec(&ann).unwrap();
    assert!(bytes.len() < 512, "announcement was {} bytes", bytes.len());
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test observation_gossip_test`
Expected: compile error

- [ ] **Step 3: Create the module**

Create `elohim/elohim-storage/src/p2p/observation_gossip.rs`:

```rust
use crate::p2p::topics::OBSERVATION_GOSSIP_TOPIC_PREFIX;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorAnnouncement {
    pub observer_cid: String,
    pub kind: String,
    pub log_cid: String,
    pub latest_offset: u64,
    pub subject_cid: Option<String>,
    pub observed_at_window: i64,
}

/// Map an observation kind to its gossipsub topic.
/// Kind format: `<namespace>:<name>` → topic: `elohim/observations/<namespace>`.
pub fn observation_topic(kind: &str) -> String {
    let namespace = kind.split(':').next().unwrap_or("unknown");
    format!("{}{}", OBSERVATION_GOSSIP_TOPIC_PREFIX, namespace)
}
```

Add to `elohim/elohim-storage/src/p2p/mod.rs`:

```rust
pub mod observation_gossip;
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --test observation_gossip_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/p2p/observation_gossip.rs \
        elohim/elohim-storage/src/p2p/mod.rs \
        elohim/elohim-storage/tests/observation_gossip_test.rs
git commit -m "feat(observation): cursor announcement type + topic mapping"
```

---

## Task 4.4: Wire gossipsub publish/subscribe behaviour

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/behaviour.rs` — subscribe to observation topics
- Modify: `elohim/elohim-storage/src/p2p/observation_gossip.rs` — publish helper

- [ ] **Step 1: Inspect existing gossipsub subscription points**

Run: `grep -n 'gossipsub.*subscribe\|gossipsub.*publish' elohim/elohim-storage/src/p2p/behaviour.rs | head -10`

- [ ] **Step 2: Add publish helper to observation_gossip.rs**

```rust
use libp2p::gossipsub::{IdentTopic, PublishError};

pub fn publish_announcement(
    gossipsub: &mut libp2p::gossipsub::Behaviour,
    announcement: &CursorAnnouncement,
) -> Result<(), PublishError> {
    let topic = IdentTopic::new(observation_topic(&announcement.kind));
    let bytes = rmp_serde::to_vec(announcement)
        .map_err(|e| PublishError::TransformFailed(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    gossipsub.publish(topic, bytes).map(|_| ())
}
```

- [ ] **Step 3: Subscribe to observation topics on swarm init**

In `behaviour.rs` constructor (find the existing subscribe block), append:

```rust
// Observation plane — subscribe to namespaces this peer cares about.
// Default subscription matrix (see observation-event-layer-design §5.3).
for namespace in &["infrastructure", "lamad", "imagodei", "shefa", "qahal"] {
    let topic = libp2p::gossipsub::IdentTopic::new(format!("elohim/observations/{}", namespace));
    gossipsub.subscribe(&topic).ok();
}
```

(The default subscribes to all five for now. Role-based subscription is Task 4.7 follow-up.)

- [ ] **Step 4: Build and run lib tests**

Run: `cargo build --lib && cargo test --lib`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/p2p/behaviour.rs \
        elohim/elohim-storage/src/p2p/observation_gossip.rs
git commit -m "feat(observation): wire gossipsub publish + per-namespace subscribe"
```

---

## Task 4.5: Iroh observation-log backend (segment fetch)

**Files:**
- Create: `elohim/elohim-storage/src/p2p_iroh/observation_backend.rs`
- Modify: `elohim/elohim-storage/src/p2p_iroh/mod.rs`
- Test: `elohim/elohim-storage/tests/observation_iroh_backend_test.rs`

- [ ] **Step 1: Write the failing parity-style test**

Create `elohim/elohim-storage/tests/observation_iroh_backend_test.rs`:

```rust
use elohim_storage::observation::log::ObservationLog;
use elohim_storage::observation::wire::Observation;
use elohim_storage::p2p_iroh::observation_backend::IrohObservationBackend;

#[tokio::test]
async fn fetch_segment_returns_appended_observations() {
    let mut log = ObservationLog::new_in_memory("agent:obs-A".into());
    for i in 0..3u64 {
        log.append(Observation { seq: i, ..Observation::test_fixture() }).await.unwrap();
    }
    let backend = IrohObservationBackend::new_in_memory(log);
    let segment = backend.fetch_segment("agent:obs-A", 0, 3).await.unwrap();
    assert_eq!(segment.len(), 3);
    assert_eq!(segment[0].seq, 0);
    assert_eq!(segment[2].seq, 2);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test observation_iroh_backend_test`
Expected: compile error — `observation_backend` module missing

- [ ] **Step 3: Create the backend**

Create `elohim/elohim-storage/src/p2p_iroh/observation_backend.rs`:

```rust
use crate::observation::log::{ObservationLog, ObservationLogError};
use crate::observation::wire::Observation;
use tokio::sync::RwLock;
use std::sync::Arc;

pub struct IrohObservationBackend {
    log: Arc<RwLock<ObservationLog>>,
}

impl IrohObservationBackend {
    pub fn new_in_memory(log: ObservationLog) -> Self {
        Self { log: Arc::new(RwLock::new(log)) }
    }

    pub async fn append(&self, obs: Observation) -> Result<(), ObservationLogError> {
        self.log.write().await.append(obs).await
    }

    pub async fn fetch_segment(
        &self,
        observer_cid: &str,
        from_offset: u64,
        to_offset: u64,
    ) -> Result<Vec<Observation>, ObservationLogError> {
        let guard = self.log.read().await;
        if guard.observer_cid() != observer_cid {
            return Ok(vec![]);
        }
        let all = guard.read_from(from_offset).await?;
        let span = (to_offset.saturating_sub(from_offset)) as usize;
        Ok(all.into_iter().take(span).collect())
    }
}
```

Update `elohim/elohim-storage/src/p2p_iroh/mod.rs`:

```rust
pub mod observation_backend;
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --test observation_iroh_backend_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/p2p_iroh/observation_backend.rs \
        elohim/elohim-storage/src/p2p_iroh/mod.rs \
        elohim/elohim-storage/tests/observation_iroh_backend_test.rs
git commit -m "feat(observation): iroh-backed observation log segment fetch"
```

---

## Task 4.6: ObservationManagerBackend — orchestrator

**Files:**
- Create: `elohim/elohim-storage/src/observation/manager.rs`
- Modify: `elohim/elohim-storage/src/observation/mod.rs`
- Test: `elohim/elohim-storage/tests/observation_manager_test.rs`

- [ ] **Step 1: Write the failing end-to-end test**

Create `elohim/elohim-storage/tests/observation_manager_test.rs`:

```rust
use elohim_storage::db::test_helpers::test_connection;
use elohim_storage::observation::manager::ObservationManagerBackend;
use elohim_storage::observation::wire::Observation;

#[tokio::test]
async fn local_append_projects_to_sql() {
    let mut conn = test_connection();
    let mgr = ObservationManagerBackend::new_in_memory("agent:self".into());

    let obs = Observation {
        subject_cid: Some("doorway:abc".into()),
        observation_kind: "infrastructure:doorway-heartbeat".into(),
        ..Observation::test_fixture()
    };
    mgr.append_local(&mut conn, obs).await.unwrap();

    let rows = mgr.query_by_subject(&mut conn, "doorway:abc", "infrastructure:doorway-heartbeat").unwrap();
    assert_eq!(rows.len(), 1);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test observation_manager_test`
Expected: compile error

- [ ] **Step 3: Create the manager**

Create `elohim/elohim-storage/src/observation/manager.rs`:

```rust
use crate::observation::log::{ObservationLog, ObservationLogError};
use crate::observation::projector::ObservationProjector;
use crate::observation::wire::Observation;
use crate::views::ObservationView;
use diesel::SqliteConnection;
use tokio::sync::RwLock;
use std::sync::Arc;

pub struct ObservationManagerBackend {
    local_log: Arc<RwLock<ObservationLog>>,
    projector: ObservationProjector,
}

impl ObservationManagerBackend {
    pub fn new_in_memory(observer_cid: String) -> Self {
        Self {
            local_log: Arc::new(RwLock::new(ObservationLog::new_in_memory(observer_cid))),
            projector: ObservationProjector::new(),
        }
    }

    /// Append a self-authored observation. Stamps log_cid + log_offset from the log, projects to SQL.
    pub async fn append_local(
        &self,
        conn: &mut SqliteConnection,
        mut obs: Observation,
    ) -> Result<(), ObservationLogError> {
        let mut log = self.local_log.write().await;
        obs.log_offset = log.latest_offset();
        log.append(obs.clone()).await?;
        obs.log_cid = log.current_log_cid();
        self.projector.project(conn, &obs).map_err(|e| ObservationLogError::Encoding(e.to_string()))?;
        Ok(())
    }

    /// Project a remote observation pulled from another observer's log.
    pub async fn project_remote(
        &self,
        conn: &mut SqliteConnection,
        obs: &Observation,
    ) -> Result<(), ObservationLogError> {
        self.projector.project(conn, obs).map_err(|e| ObservationLogError::Encoding(e.to_string()))?;
        Ok(())
    }

    pub fn query_by_subject(
        &self,
        conn: &mut SqliteConnection,
        subject: &str,
        kind: &str,
    ) -> Result<Vec<ObservationView>, diesel::result::Error> {
        self.projector.by_subject(conn, subject, kind)
    }
}
```

Update `elohim/elohim-storage/src/observation/mod.rs`:

```rust
pub mod log;
pub mod manager;
pub mod projector;
pub mod wire;
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --test observation_manager_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/observation/manager.rs \
        elohim/elohim-storage/src/observation/mod.rs \
        elohim/elohim-storage/tests/observation_manager_test.rs
git commit -m "feat(observation): ObservationManagerBackend wires log + projector"
```

---

# Stage 5 — Graduation evaluator

## Task 5.1: Diversity threshold check

**Files:**
- Create: `elohim/elohim-storage/src/graduation/mod.rs`
- Create: `elohim/elohim-storage/src/graduation/diversity.rs`
- Modify: `elohim/elohim-storage/src/lib.rs` — add `pub mod graduation;`
- Test: `elohim/elohim-storage/tests/graduation_diversity_test.rs`

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-storage/tests/graduation_diversity_test.rs`:

```rust
use elohim_storage::db::test_helpers::test_connection;
use elohim_storage::graduation::diversity::{DiversityThreshold, threshold_met};
use elohim_storage::observation::manager::ObservationManagerBackend;
use elohim_storage::observation::wire::Observation;

#[tokio::test]
async fn threshold_not_met_with_single_household() {
    let mut conn = test_connection();
    let mgr = ObservationManagerBackend::new_in_memory("agent:self".into());

    for i in 0..5u64 {
        mgr.append_local(&mut conn, Observation {
            seq: i,
            subject_cid: Some("doorway:abc".into()),
            observation_kind: "infrastructure:doorway-heartbeat".into(),
            observer_household_cid: Some("household:single".into()),
            ..Observation::test_fixture()
        }).await.unwrap();
    }

    let threshold = DiversityThreshold {
        distinct_households: Some(3),
        min_count: Some(5),
        ..Default::default()
    };
    assert!(!threshold_met(&mut conn, "doorway:abc", "infrastructure:doorway-heartbeat", &threshold).unwrap());
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test graduation_diversity_test`
Expected: compile error

- [ ] **Step 3: Create the threshold checker**

Create `elohim/elohim-storage/src/graduation/mod.rs`:

```rust
pub mod diversity;
```

Create `elohim/elohim-storage/src/graduation/diversity.rs`:

```rust
use crate::db::schema::observation_diversity_summary as ods;
use crate::views::ObservationDiversitySummaryView;
use diesel::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct DiversityThreshold {
    pub distinct_households: Option<i64>,
    pub distinct_collectives: Option<i64>,
    pub distinct_regions: Option<i64>,
    pub distinct_archetypes: Option<i64>,
    pub min_count: Option<i64>,
}

pub fn threshold_met(
    conn: &mut SqliteConnection,
    subject_cid: &str,
    observation_kind: &str,
    threshold: &DiversityThreshold,
) -> Result<bool, diesel::result::Error> {
    use ods::dsl::*;
    let row: Option<ObservationDiversitySummaryView> = ods::table
        .filter(ods::subject_cid.eq(subject_cid))
        .filter(ods::observation_kind.eq(observation_kind))
        .first(conn)
        .optional()?;
    let Some(row) = row else { return Ok(false); };
    if let Some(t) = threshold.distinct_households { if row.distinct_households < t { return Ok(false); } }
    if let Some(t) = threshold.distinct_collectives { if row.distinct_collectives < t { return Ok(false); } }
    if let Some(t) = threshold.distinct_regions { if row.distinct_regions < t { return Ok(false); } }
    if let Some(t) = threshold.distinct_archetypes { if row.distinct_archetypes < t { return Ok(false); } }
    if let Some(t) = threshold.min_count { if row.total_count < t { return Ok(false); } }
    Ok(true)
}
```

Add to `elohim/elohim-storage/src/lib.rs`:

```rust
pub mod graduation;
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --test graduation_diversity_test`
Expected: PASS

- [ ] **Step 5: Add the positive case**

Append to the same test file:

```rust
#[tokio::test]
async fn threshold_met_with_three_households() {
    let mut conn = test_connection();
    let mgr = ObservationManagerBackend::new_in_memory("agent:self".into());

    for (i, hh) in [(0, "h1"), (1, "h1"), (2, "h2"), (3, "h2"), (4, "h3")].iter() {
        mgr.append_local(&mut conn, Observation {
            seq: *i,
            subject_cid: Some("doorway:abc".into()),
            observation_kind: "infrastructure:doorway-heartbeat".into(),
            observer_household_cid: Some(format!("household:{}", hh)),
            ..Observation::test_fixture()
        }).await.unwrap();
    }

    let threshold = DiversityThreshold {
        distinct_households: Some(3),
        min_count: Some(5),
        ..Default::default()
    };
    assert!(threshold_met(&mut conn, "doorway:abc", "infrastructure:doorway-heartbeat", &threshold).unwrap());
}
```

- [ ] **Step 6: Run again**

Run: `cargo test --test graduation_diversity_test`
Expected: PASS (both)

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/graduation/ \
        elohim/elohim-storage/src/lib.rs \
        elohim/elohim-storage/tests/graduation_diversity_test.rs
git commit -m "feat(graduation): diversity threshold checker against summary view"
```

---

## Task 5.2: Path 2 — summary EconomicEvent graduation

**Files:**
- Create: `elohim/elohim-storage/src/graduation/summary_event.rs`
- Modify: `elohim/elohim-storage/src/graduation/mod.rs`
- Test: `elohim/elohim-storage/tests/graduation_summary_event_test.rs`

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-storage/tests/graduation_summary_event_test.rs`:

```rust
use elohim_storage::db::test_helpers::test_connection;
use elohim_storage::graduation::summary_event::SummaryEventSpec;
use elohim_storage::observation::manager::ObservationManagerBackend;
use elohim_storage::observation::wire::Observation;

#[tokio::test]
async fn summarize_blob_served_emits_one_event_per_provider_per_window() {
    let mut conn = test_connection();
    let mgr = ObservationManagerBackend::new_in_memory("provider:x".into());

    for i in 0..3u64 {
        mgr.append_local(&mut conn, Observation {
            seq: i,
            observation_kind: "infrastructure:blob-served".into(),
            payload_json: format!(r#"{{"blob_cid":"b{}","bytes":1000,"peer_cid":"r{}"}}"#, i, i),
            ..Observation::test_fixture()
        }).await.unwrap();
    }

    let spec = SummaryEventSpec {
        observation_kind: "infrastructure:blob-served".into(),
        action_verb: "served-blob-summary".into(),
        resource: "blob-bytes".into(),
        window_seconds: 3600,
    };
    let summary = spec.evaluate(&mut conn, 1715420400).unwrap().expect("summary should produce");
    assert_eq!(summary.action, "served-blob-summary");
    assert_eq!(summary.observation_refs.len(), 3);
    assert!(summary.total_quantity > 0.0);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test graduation_summary_event_test`
Expected: compile error

- [ ] **Step 3: Create the summary spec**

Create `elohim/elohim-storage/src/graduation/summary_event.rs`:

```rust
use crate::db::schema::observations;
use crate::views::ObservationView;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryEventSpec {
    pub observation_kind: String,
    pub action_verb: String,
    pub resource: String,
    pub window_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraduatedSummary {
    pub action: String,
    pub provider_cid: String,
    pub resource: String,
    pub total_quantity: f64,
    pub period_start: i64,
    pub period_end: i64,
    pub observation_refs: Vec<String>,
}

impl SummaryEventSpec {
    pub fn evaluate(
        &self,
        conn: &mut SqliteConnection,
        window_end: i64,
    ) -> Result<Option<GraduatedSummary>, diesel::result::Error> {
        let window_start = window_end - self.window_seconds;
        let rows: Vec<ObservationView> = observations::table
            .filter(observations::observation_kind.eq(&self.observation_kind))
            .filter(observations::observed_at.ge(window_start))
            .filter(observations::observed_at.lt(window_end))
            .load(conn)?;
        if rows.is_empty() { return Ok(None); }

        let provider_cid = rows[0].observer_cid.clone();
        let total_quantity: f64 = rows.iter().filter_map(|r| {
            serde_json::from_str::<serde_json::Value>(&r.payload_json).ok()
                .and_then(|v| v.get("bytes").and_then(|n| n.as_f64()))
        }).sum();
        let observation_refs = rows.iter()
            .map(|r| format!("iroh://{}@{}#{}", r.observer_cid, r.log_cid, r.log_offset))
            .collect();
        Ok(Some(GraduatedSummary {
            action: self.action_verb.clone(),
            provider_cid,
            resource: self.resource.clone(),
            total_quantity,
            period_start: window_start,
            period_end: window_end,
            observation_refs,
        }))
    }
}
```

Update `elohim/elohim-storage/src/graduation/mod.rs`:

```rust
pub mod diversity;
pub mod summary_event;
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --test graduation_summary_event_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/graduation/summary_event.rs \
        elohim/elohim-storage/src/graduation/mod.rs \
        elohim/elohim-storage/tests/graduation_summary_event_test.rs
git commit -m "feat(graduation): summary EconomicEvent path with observation_refs"
```

---

## Task 5.3: Path 1 — attestation graduation spec

**Files:**
- Create: `elohim/elohim-storage/src/graduation/attestation.rs`
- Modify: `elohim/elohim-storage/src/graduation/mod.rs`
- Test: `elohim/elohim-storage/tests/graduation_attestation_test.rs`

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-storage/tests/graduation_attestation_test.rs`:

```rust
use elohim_storage::db::test_helpers::test_connection;
use elohim_storage::graduation::attestation::{AttestationGraduationSpec};
use elohim_storage::graduation::diversity::DiversityThreshold;
use elohim_storage::observation::manager::ObservationManagerBackend;
use elohim_storage::observation::wire::Observation;

#[tokio::test]
async fn attestation_graduation_emits_when_threshold_met() {
    let mut conn = test_connection();
    let mgr = ObservationManagerBackend::new_in_memory("agent:self".into());

    for (i, hh) in [(0u64, "h1"), (1, "h2"), (2, "h3"), (3, "h1"), (4, "h2")].iter() {
        mgr.append_local(&mut conn, Observation {
            seq: *i,
            subject_cid: Some("doorway:abc".into()),
            observation_kind: "infrastructure:doorway-heartbeat".into(),
            observer_household_cid: Some(format!("household:{}", hh)),
            ..Observation::test_fixture()
        }).await.unwrap();
    }

    let spec = AttestationGraduationSpec {
        observation_kind: "infrastructure:doorway-heartbeat".into(),
        attestation_subtype: "doorway-health".into(),
        threshold: DiversityThreshold { distinct_households: Some(3), min_count: Some(5), ..Default::default() },
    };
    let plan = spec.evaluate(&mut conn, "doorway:abc").unwrap().expect("should graduate");
    assert_eq!(plan.attestation_content_type, "attestation:doorway-health");
    assert_eq!(plan.subject_cid, "doorway:abc");
    assert_eq!(plan.observation_refs.len(), 5);
}

#[tokio::test]
async fn attestation_graduation_returns_none_below_threshold() {
    let mut conn = test_connection();
    let mgr = ObservationManagerBackend::new_in_memory("agent:self".into());

    for i in 0..2u64 {
        mgr.append_local(&mut conn, Observation {
            seq: i,
            subject_cid: Some("doorway:abc".into()),
            observation_kind: "infrastructure:doorway-heartbeat".into(),
            observer_household_cid: Some("household:single".into()),
            ..Observation::test_fixture()
        }).await.unwrap();
    }
    let spec = AttestationGraduationSpec {
        observation_kind: "infrastructure:doorway-heartbeat".into(),
        attestation_subtype: "doorway-health".into(),
        threshold: DiversityThreshold { distinct_households: Some(3), min_count: Some(5), ..Default::default() },
    };
    assert!(spec.evaluate(&mut conn, "doorway:abc").unwrap().is_none());
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test graduation_attestation_test`
Expected: compile error

- [ ] **Step 3: Create the spec**

Create `elohim/elohim-storage/src/graduation/attestation.rs`:

```rust
use crate::db::schema::observations;
use crate::graduation::diversity::{threshold_met, DiversityThreshold};
use crate::views::ObservationView;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationGraduationSpec {
    pub observation_kind: String,
    pub attestation_subtype: String,
    pub threshold: DiversityThreshold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationPlan {
    pub attestation_content_type: String,
    pub subject_cid: String,
    pub observation_refs: Vec<String>,
    pub proof_class: String,
}

impl AttestationGraduationSpec {
    pub fn evaluate(
        &self,
        conn: &mut SqliteConnection,
        subject_cid: &str,
    ) -> Result<Option<AttestationPlan>, diesel::result::Error> {
        if !threshold_met(conn, subject_cid, &self.observation_kind, &self.threshold)? {
            return Ok(None);
        }
        let rows: Vec<ObservationView> = observations::table
            .filter(observations::subject_cid.eq(subject_cid))
            .filter(observations::observation_kind.eq(&self.observation_kind))
            .load(conn)?;
        let observation_refs = rows.iter()
            .map(|r| format!("iroh://{}@{}#{}", r.observer_cid, r.log_cid, r.log_offset))
            .collect();
        Ok(Some(AttestationPlan {
            attestation_content_type: format!("attestation:{}", self.attestation_subtype),
            subject_cid: subject_cid.to_string(),
            observation_refs,
            proof_class: "witness".to_string(),
        }))
    }
}
```

Update `elohim/elohim-storage/src/graduation/mod.rs`:

```rust
pub mod attestation;
pub mod diversity;
pub mod summary_event;
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --test graduation_attestation_test`
Expected: PASS (both)

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/graduation/attestation.rs \
        elohim/elohim-storage/src/graduation/mod.rs \
        elohim/elohim-storage/tests/graduation_attestation_test.rs
git commit -m "feat(graduation): attestation path with diversity-threshold gating"
```

---

## Task 5.4: Stake-class coordinator gate in elohim DNA

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_coordinator/src/lib.rs`
- Test: `elohim/holochain/tests/sweettest/src/observation/stake_class_gate.rs` (new) or extend existing sweettest

Set environment for HC builds:
```bash
export RUSTFLAGS='--cfg getrandom_backend="custom"'
```

- [ ] **Step 1: Locate `create_economic_event` handler**

Run: `grep -n 'fn create_economic_event\|pub fn create_economic_event' elohim/holochain/dna/elohim/zomes/content_store_coordinator/src/lib.rs | head -5`

- [ ] **Step 2: Add the stake-class gate**

Before the call to `create_entry`, add (inside `create_economic_event` handler):

```rust
// Stake-class gate (Observation/Event Layer spec §8.3).
// Manifest declares each action verb as 'high' or 'operational'.
// Operational verbs are accepted ONLY with non-empty observation_refs.
let stake_class = lookup_stake_class(&input.action)?;
match stake_class.as_str() {
    "high" => { /* accept directly */ }
    "operational" => {
        let refs = input.observation_refs.as_deref().unwrap_or(&[]);
        if refs.is_empty() {
            return Err(wasm_error!(WasmErrorInner::Guest(
                format!("operational action '{}' requires non-empty observation_refs", input.action)
            )));
        }
    }
    other => return Err(wasm_error!(WasmErrorInner::Guest(
        format!("unknown stake_class '{}' for action '{}'", other, input.action)
    ))),
}
```

Add a `lookup_stake_class` helper (read from a const map declared at the top of the file):

```rust
fn lookup_stake_class(action: &str) -> ExternResult<String> {
    // Initial bootstrap stake classes (until manifest is queryable from zome).
    // Manifest amendment moves verbs between classes.
    match action {
        // High-stakes verbs
        "transfer-standing" | "grant-stewardship" | "custody-handoff" | "enact-governance-action"
            => Ok("high".to_string()),
        // Operational verbs (require observation_refs)
        "served-blob-summary" | "appreciation-summary" | "consumed-compute-summary"
            => Ok("operational".to_string()),
        // Default high for unrecognized — fail-closed; new verbs need explicit declaration.
        _ => Err(wasm_error!(WasmErrorInner::Guest(
            format!("action '{}' has no stake_class declaration; declare in manifest", action)
        ))),
    }
}
```

Also ensure `CreateEconomicEventInput` has `observation_refs: Option<Vec<String>>`. If not, add it.

- [ ] **Step 3: Compile**

Run: `cd elohim/holochain/dna/elohim && cargo build --target wasm32-unknown-unknown -p content_store_coordinator 2>&1 | tail -20`
Expected: success

- [ ] **Step 4: Add a sweettest scenario**

Create `elohim/holochain/tests/sweettest/src/observation/mod.rs` and `stake_class_gate.rs` (consult existing sweettest pattern — see `feedback_sweettest_cross_agent_consistency`):

```rust
use sweettest::*;

#[tokio::test(flavor = "multi_thread")]
async fn operational_event_requires_observation_refs() {
    let (mut conductor, mut cells, _) = setup_two_agent_conductors().await;
    let zome = cells[0].zome("content_store_coordinator");

    // Without observation_refs -> should fail
    let result: Result<EntryHash, _> = conductor.call_fallible(&zome, "create_economic_event", json!({
        "action": "served-blob-summary",
        "provider_cid": "p",
        "resource": "blob-bytes",
        "quantity": 1.0,
        "observation_refs": null,
    })).await;
    assert!(result.is_err(), "operational event without refs must be rejected");

    // With observation_refs -> should succeed
    let ok: EntryHash = conductor.call(&zome, "create_economic_event", json!({
        "action": "served-blob-summary",
        "provider_cid": "p",
        "resource": "blob-bytes",
        "quantity": 1.0,
        "observation_refs": ["iroh://a@b#0"],
    })).await;
    assert!(!ok.get_raw_39().is_empty());
}
```

- [ ] **Step 5: Run sweettest**

Run: `cd elohim/holochain/tests/sweettest && CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__holochain__tests__sweettest/dev cargo test stake_class_gate -- --nocapture 2>&1 | tail -30`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add elohim/holochain/dna/elohim/zomes/content_store_coordinator/src/lib.rs \
        elohim/holochain/tests/sweettest/src/observation/
git commit -m "feat(elohim-dna): stake-class gate rejects operational verbs without observation_refs"
```

---

## Task 5.5: Graduation evaluator tokio task

**Files:**
- Create: `elohim/elohim-storage/src/graduation/evaluator.rs`
- Modify: `elohim/elohim-storage/src/graduation/mod.rs`
- Test: `elohim/elohim-storage/tests/graduation_evaluator_test.rs`

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-storage/tests/graduation_evaluator_test.rs`:

```rust
use elohim_storage::db::test_helpers::test_connection;
use elohim_storage::graduation::evaluator::{GraduationEvaluator, GraduationConfig};
use elohim_storage::graduation::diversity::DiversityThreshold;
use elohim_storage::observation::manager::ObservationManagerBackend;
use elohim_storage::observation::wire::Observation;

#[tokio::test]
async fn tick_emits_pending_graduations() {
    let mut conn = test_connection();
    let mgr = ObservationManagerBackend::new_in_memory("agent:self".into());

    for (i, hh) in [(0u64, "h1"), (1, "h2"), (2, "h3"), (3, "h1"), (4, "h2")].iter() {
        mgr.append_local(&mut conn, Observation {
            seq: *i,
            subject_cid: Some("doorway:abc".into()),
            observation_kind: "infrastructure:doorway-heartbeat".into(),
            observer_household_cid: Some(format!("household:{}", hh)),
            ..Observation::test_fixture()
        }).await.unwrap();
    }

    let mut evaluator = GraduationEvaluator::new(vec![
        GraduationConfig::attestation(
            "infrastructure:doorway-heartbeat",
            "doorway-health",
            DiversityThreshold { distinct_households: Some(3), min_count: Some(5), ..Default::default() },
        ),
    ]);
    let plans = evaluator.tick(&mut conn, 1715420400).unwrap();
    assert_eq!(plans.attestations.len(), 1);
    assert_eq!(plans.attestations[0].subject_cid, "doorway:abc");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test graduation_evaluator_test`
Expected: compile error

- [ ] **Step 3: Create the evaluator**

Create `elohim/elohim-storage/src/graduation/evaluator.rs`:

```rust
use crate::db::schema::observations;
use crate::graduation::attestation::{AttestationGraduationSpec, AttestationPlan};
use crate::graduation::diversity::DiversityThreshold;
use crate::graduation::summary_event::{SummaryEventSpec, GraduatedSummary};
use diesel::prelude::*;

#[derive(Debug, Clone)]
pub enum GraduationConfig {
    Attestation(AttestationGraduationSpec),
    Summary(SummaryEventSpec),
}

impl GraduationConfig {
    pub fn attestation(observation_kind: &str, subtype: &str, threshold: DiversityThreshold) -> Self {
        Self::Attestation(AttestationGraduationSpec {
            observation_kind: observation_kind.into(),
            attestation_subtype: subtype.into(),
            threshold,
        })
    }
    pub fn summary(observation_kind: &str, action_verb: &str, resource: &str, window_seconds: i64) -> Self {
        Self::Summary(SummaryEventSpec {
            observation_kind: observation_kind.into(),
            action_verb: action_verb.into(),
            resource: resource.into(),
            window_seconds,
        })
    }
}

#[derive(Debug, Default)]
pub struct GraduationPlans {
    pub attestations: Vec<AttestationPlan>,
    pub summaries: Vec<GraduatedSummary>,
}

pub struct GraduationEvaluator {
    configs: Vec<GraduationConfig>,
}

impl GraduationEvaluator {
    pub fn new(configs: Vec<GraduationConfig>) -> Self { Self { configs } }

    pub fn tick(
        &mut self,
        conn: &mut SqliteConnection,
        now: i64,
    ) -> Result<GraduationPlans, diesel::result::Error> {
        let mut plans = GraduationPlans::default();
        for cfg in &self.configs {
            match cfg {
                GraduationConfig::Attestation(spec) => {
                    let subjects: Vec<String> = observations::table
                        .filter(observations::observation_kind.eq(&spec.observation_kind))
                        .filter(observations::subject_cid.is_not_null())
                        .select(observations::subject_cid)
                        .distinct()
                        .load::<Option<String>>(conn)?
                        .into_iter().flatten().collect();
                    for subject in subjects {
                        if let Some(plan) = spec.evaluate(conn, &subject)? {
                            plans.attestations.push(plan);
                        }
                    }
                }
                GraduationConfig::Summary(spec) => {
                    if let Some(summary) = spec.evaluate(conn, now)? {
                        plans.summaries.push(summary);
                    }
                }
            }
        }
        Ok(plans)
    }
}
```

Update `elohim/elohim-storage/src/graduation/mod.rs`:

```rust
pub mod attestation;
pub mod diversity;
pub mod evaluator;
pub mod summary_event;
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --test graduation_evaluator_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/graduation/evaluator.rs \
        elohim/elohim-storage/src/graduation/mod.rs \
        elohim/elohim-storage/tests/graduation_evaluator_test.rs
git commit -m "feat(graduation): tokio-friendly evaluator emits plans per tick"
```

---

# Stage 6 — DHT entry-type retirement

## Task 6.1: Remove DoorwayHeartbeat from infrastructure DNA

**Files:**
- Modify: `elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs`
- Modify: `elohim/holochain/dna/infrastructure/zomes/infrastructure_coordinator/src/lib.rs`
- Delete sweettest scenarios that exercise DoorwayHeartbeat

- [ ] **Step 1: Identify call-sites**

Run: `grep -rn 'DoorwayHeartbeat\|doorway_heartbeat\|record_heartbeat' elohim/holochain/ --include='*.rs' | head -30`

- [ ] **Step 2: Remove `DoorwayHeartbeat` from integrity zome**

In `infrastructure_integrity/src/lib.rs`:
- Delete the `pub struct DoorwayHeartbeat { ... }` block
- Delete the `EntryTypes::DoorwayHeartbeat(...)` variant
- Delete the `DoorwayToHeartbeat` link type
- Delete any validator branches that reference it

- [ ] **Step 3: Remove coordinator handlers**

In `infrastructure_coordinator/src/lib.rs`:
- Delete `record_heartbeat`, `get_recent_heartbeats`, and any related signal emissions

- [ ] **Step 4: Delete sweettest scenarios**

Run: `grep -rln 'DoorwayHeartbeat\|record_heartbeat' elohim/holochain/tests/sweettest/ 2>/dev/null`

Delete the scenarios that no longer apply (or refactor to observation kind if they cover regression that matters).

- [ ] **Step 5: Compile**

Run: `cd elohim/holochain/dna/infrastructure && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --target wasm32-unknown-unknown 2>&1 | tail -20`
Expected: build succeeds

- [ ] **Step 6: Run remaining sweettests**

Run: `cd elohim/holochain/tests/sweettest && CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__holochain__tests__sweettest/dev cargo test infrastructure -- --nocapture 2>&1 | tail -30`
Expected: PASS (no test references the removed entry type)

- [ ] **Step 7: Commit**

```bash
git add elohim/holochain/dna/infrastructure/ \
        elohim/holochain/tests/sweettest/
git commit -m "feat(infrastructure-dna)!: retire DoorwayHeartbeat (moved to observation layer)"
```

---

## Task 6.2: Remove DoorwayHeartbeatSummary and HealthAttestation

**Files:**
- Modify: `elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs`
- Modify: `elohim/holochain/dna/infrastructure/zomes/infrastructure_coordinator/src/lib.rs`

- [ ] **Step 1: Remove DoorwayHeartbeatSummary**

Same pattern as Task 6.1: delete struct, entry variant, coordinator handlers, sweettests.

- [ ] **Step 2: Remove HealthAttestation**

Same pattern.

- [ ] **Step 3: Compile**

Run: `cd elohim/holochain/dna/infrastructure && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --target wasm32-unknown-unknown 2>&1 | tail -20`

- [ ] **Step 4: Run sweettests**

Run: `cd elohim/holochain/tests/sweettest && CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__holochain__tests__sweettest/dev cargo test infrastructure -- --nocapture 2>&1 | tail -30`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add elohim/holochain/dna/infrastructure/
git commit -m "feat(infrastructure-dna)!: retire DoorwayHeartbeatSummary and HealthAttestation"
```

---

## Task 6.3: Update storage projector to drop retired-entry signal handlers

**Files:**
- Modify: `elohim/elohim-storage/src/projector/signals.rs`

- [ ] **Step 1: Find retired-entry signal handlers**

Run: `grep -n 'DoorwayHeartbeat\|HealthAttestation' elohim/elohim-storage/src/projector/signals.rs`

- [ ] **Step 2: Delete the handlers**

Remove the post-commit signal projector arms for `Signal::DoorwayHeartbeatCreated`, `Signal::HealthAttestationCreated`, etc.

- [ ] **Step 3: Delete the migration that created `doorway_heartbeats` projection table (if it exists)**

Check: `ls elohim/elohim-storage/migrations/ | grep -i heartbeat`

If a heartbeat projection table exists, create a retirement migration:

```bash
mkdir -p elohim/elohim-storage/migrations/2026-05-13-120000_retire_doorway_heartbeat
```

`up.sql`:
```sql
DROP TABLE IF EXISTS doorway_heartbeats;
DROP TABLE IF EXISTS doorway_heartbeat_summaries;
DROP TABLE IF EXISTS health_attestations;
```

`down.sql`:
```sql
-- Retired tables; reconstruction via observation layer is intentional.
SELECT 1;
```

- [ ] **Step 4: Build**

Run: `cd elohim/elohim-storage && cargo build --lib`
Expected: success

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/projector/signals.rs \
        elohim/elohim-storage/migrations/2026-05-13-120000_retire_doorway_heartbeat/ \
        elohim/elohim-storage/src/db/schema.rs
git commit -m "feat(storage)!: drop retired DHT projection handlers and tables"
```

---

# Stage 7 — HTTP API + storage-client

## Task 7.1: GET /api/observations/by-subject

**Files:**
- Create: `elohim/elohim-storage/src/api/observations.rs`
- Modify: `elohim/elohim-storage/src/api/mod.rs`
- Test: `elohim/elohim-storage/tests/api_observations_test.rs`

- [ ] **Step 1: Write the failing test**

Create `elohim/elohim-storage/tests/api_observations_test.rs`:

```rust
use elohim_storage::api::observations::{by_subject_handler, BySubjectQuery};
use elohim_storage::db::test_helpers::test_state;
use elohim_storage::observation::manager::ObservationManagerBackend;
use elohim_storage::observation::wire::Observation;

#[tokio::test]
async fn by_subject_returns_matching_rows() {
    let state = test_state();
    let mgr = ObservationManagerBackend::new_in_memory("agent:self".into());
    {
        let mut conn = state.db.get().unwrap();
        mgr.append_local(&mut conn, Observation {
            subject_cid: Some("doorway:abc".into()),
            observation_kind: "infrastructure:doorway-heartbeat".into(),
            ..Observation::test_fixture()
        }).await.unwrap();
    }
    let q = BySubjectQuery {
        subject_cid: "doorway:abc".to_string(),
        kind: "infrastructure:doorway-heartbeat".to_string(),
    };
    let rows = by_subject_handler(state, q).await.unwrap();
    assert_eq!(rows.len(), 1);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test api_observations_test`
Expected: compile error

- [ ] **Step 3: Create the handler**

Create `elohim/elohim-storage/src/api/observations.rs`:

```rust
use crate::api::AppState;
use crate::db::schema::observations;
use crate::views::ObservationView;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BySubjectQuery {
    pub subject_cid: String,
    pub kind: String,
}

pub async fn by_subject_handler(
    state: AppState,
    q: BySubjectQuery,
) -> Result<Vec<ObservationView>, diesel::result::Error> {
    let mut conn = state.db.get().expect("connection from pool");
    observations::table
        .filter(observations::subject_cid.eq(&q.subject_cid))
        .filter(observations::observation_kind.eq(&q.kind))
        .order(observations::observed_at.desc())
        .load::<ObservationView>(&mut conn)
}
```

Update `elohim/elohim-storage/src/api/mod.rs`:

```rust
pub mod observations;
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --test api_observations_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/api/observations.rs \
        elohim/elohim-storage/src/api/mod.rs \
        elohim/elohim-storage/tests/api_observations_test.rs
git commit -m "feat(api): GET /api/observations/by-subject handler"
```

---

## Task 7.2: Diversity and by-observer routes

**Files:**
- Modify: `elohim/elohim-storage/src/api/observations.rs`

- [ ] **Step 1: Add the diversity handler test**

Append to `tests/api_observations_test.rs`:

```rust
use elohim_storage::api::observations::{diversity_handler, DiversityQuery};

#[tokio::test]
async fn diversity_returns_aggregates_for_subject() {
    let state = test_state();
    let mgr = ObservationManagerBackend::new_in_memory("agent:o1".into());
    {
        let mut conn = state.db.get().unwrap();
        for (i, hh) in [(0u64,"h1"),(1,"h2"),(2,"h3")].iter() {
            mgr.append_local(&mut conn, Observation {
                seq: *i,
                subject_cid: Some("doorway:abc".into()),
                observation_kind: "infrastructure:doorway-heartbeat".into(),
                observer_household_cid: Some(format!("household:{}", hh)),
                ..Observation::test_fixture()
            }).await.unwrap();
        }
    }
    let summary = diversity_handler(state, DiversityQuery {
        subject_cid: "doorway:abc".into(),
        kind: "infrastructure:doorway-heartbeat".into(),
    }).await.unwrap().expect("summary present");
    assert_eq!(summary.distinct_households, 3);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test api_observations_test diversity_returns_aggregates_for_subject`
Expected: compile error

- [ ] **Step 3: Add handlers**

Append to `api/observations.rs`:

```rust
use crate::db::schema::observation_diversity_summary as ods;
use crate::views::ObservationDiversitySummaryView;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiversityQuery {
    pub subject_cid: String,
    pub kind: String,
}

pub async fn diversity_handler(
    state: AppState,
    q: DiversityQuery,
) -> Result<Option<ObservationDiversitySummaryView>, diesel::result::Error> {
    let mut conn = state.db.get().expect("connection from pool");
    ods::table
        .filter(ods::subject_cid.eq(&q.subject_cid))
        .filter(ods::observation_kind.eq(&q.kind))
        .first(&mut conn)
        .optional()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ByObserverQuery {
    pub observer_cid: String,
    pub kind: Option<String>,
}

pub async fn by_observer_handler(
    state: AppState,
    q: ByObserverQuery,
) -> Result<Vec<ObservationView>, diesel::result::Error> {
    let mut conn = state.db.get().expect("connection from pool");
    let mut query = observations::table
        .filter(observations::observer_cid.eq(&q.observer_cid))
        .into_boxed();
    if let Some(k) = &q.kind {
        query = query.filter(observations::observation_kind.eq(k));
    }
    query.order(observations::observed_at.desc()).load::<ObservationView>(&mut conn)
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --test api_observations_test`
Expected: PASS (all)

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/api/observations.rs \
        elohim/elohim-storage/tests/api_observations_test.rs
git commit -m "feat(api): add diversity and by-observer handlers"
```

---

## Task 7.3: Wire HTTP routes into the router

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs` or the route registration point (depends on framework; verify location)

- [ ] **Step 1: Locate route registration**

Run: `grep -rn 'fn router\|Router::new\|route("/api' elohim/elohim-storage/src/ | head -10`

- [ ] **Step 2: Add routes**

Add to the route registration:

```rust
.route("/api/observations/by-subject", get(crate::api::observations::by_subject_handler_axum))
.route("/api/observations/by-observer", get(crate::api::observations::by_observer_handler_axum))
.route("/api/observations/diversity", get(crate::api::observations::diversity_handler_axum))
```

Add the axum-shaped wrappers next to the handlers (Query extractor + Json response).

- [ ] **Step 3: Build**

Run: `cargo build --lib`
Expected: success

- [ ] **Step 4: Integration smoke test (existing test suite)**

Run: `cargo test --test http_smoke 2>/dev/null || cargo test http`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/
git commit -m "feat(api): register observation routes in HTTP router"
```

---

## Task 7.4: Angular ObservationService

**Files:**
- Create: `app/elohim-library/projects/elohim-service/src/services/observation.service.ts`
- Create: `app/elohim-library/projects/elohim-service/src/services/observation.service.spec.ts`
- Modify: `app/elohim-library/projects/elohim-service/src/public-api.ts`

- [ ] **Step 1: Write the failing test**

Create `observation.service.spec.ts`:

```typescript
import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting, HttpTestingController } from '@angular/common/http/testing';
import { ObservationService } from './observation.service';

describe('ObservationService', () => {
  let service: ObservationService;
  let httpMock: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [ObservationService, provideHttpClient(), provideHttpClientTesting()]
    });
    service = TestBed.inject(ObservationService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => httpMock.verify());

  it('bySubject calls correct endpoint', () => {
    service.bySubject('doorway:abc', 'infrastructure:doorway-heartbeat').subscribe();
    const req = httpMock.expectOne(r => r.url === '/api/observations/by-subject');
    expect(req.request.params.get('subjectCid')).toBe('doorway:abc');
    expect(req.request.params.get('kind')).toBe('infrastructure:doorway-heartbeat');
    req.flush([]);
  });

  it('diversity returns summary view', () => {
    service.diversity('doorway:abc', 'infrastructure:doorway-heartbeat').subscribe(s => {
      expect(s?.distinctHouseholds).toBe(3);
    });
    const req = httpMock.expectOne(r => r.url === '/api/observations/diversity');
    req.flush({
      subjectCid: 'doorway:abc',
      observationKind: 'infrastructure:doorway-heartbeat',
      distinctAgents: 5,
      distinctHouseholds: 3,
      distinctCollectives: 1,
      distinctRegions: 1,
      distinctArchetypes: 2,
      distinctComputeClasses: 1,
      totalCount: 5,
      firstObservedAt: 0,
      lastObservedAt: 0,
    });
  });
});
```

- [ ] **Step 2: Run to verify failure**

Run: `cd app/elohim-library/projects/elohim-service && pnpm test observation.service`
Expected: fail — ObservationService not found

- [ ] **Step 3: Create the service**

Create `observation.service.ts`:

```typescript
import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';
import { ObservationView, ObservationDiversitySummaryView } from '@elohim/storage-client';

@Injectable({ providedIn: 'root' })
export class ObservationService {
  private readonly http = inject(HttpClient);
  private readonly base = '/api/observations';

  bySubject(subjectCid: string, kind: string): Observable<ObservationView[]> {
    return this.http.get<ObservationView[]>(`${this.base}/by-subject`, {
      params: new HttpParams().set('subjectCid', subjectCid).set('kind', kind),
    });
  }

  byObserver(observerCid: string, kind?: string): Observable<ObservationView[]> {
    let params = new HttpParams().set('observerCid', observerCid);
    if (kind) params = params.set('kind', kind);
    return this.http.get<ObservationView[]>(`${this.base}/by-observer`, { params });
  }

  diversity(subjectCid: string, kind: string): Observable<ObservationDiversitySummaryView | null> {
    return this.http.get<ObservationDiversitySummaryView | null>(`${this.base}/diversity`, {
      params: new HttpParams().set('subjectCid', subjectCid).set('kind', kind),
    });
  }
}
```

Add to `public-api.ts`:

```typescript
export { ObservationService } from './services/observation.service';
```

- [ ] **Step 4: Run to verify pass**

Run: `pnpm test observation.service`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add app/elohim-library/projects/elohim-service/src/services/observation.service.ts \
        app/elohim-library/projects/elohim-service/src/services/observation.service.spec.ts \
        app/elohim-library/projects/elohim-service/src/public-api.ts
git commit -m "feat(elohim-service): ObservationService HTTP client"
```

---

# Stage 8 — Existing-table reclassification

## Task 8.1: Annotate existing tables as observation projections

**Files:**
- Modify migration up.sql files (comments only):
  - `elohim/elohim-storage/migrations/2026-05-02-110000_peer_blob_inventory/up.sql`
  - `elohim/elohim-storage/migrations/2026-05-11-110000_projection_events/up.sql`
  - Any system_metrics migration

- [ ] **Step 1: Identify the precise migrations**

Run: `ls elohim/elohim-storage/migrations/ | grep -E 'peer_blob_inventory|projection_events|system_metrics' | head -5`

- [ ] **Step 2: Add doc comments**

For `peer_blob_inventory/up.sql`, prepend (do not edit the schema):

```sql
-- RECLASSIFICATION NOTE (Observation/Event Layer spec — Stage 8):
-- This table is the SQL projection of the 'infrastructure:blob-served' and
-- 'infrastructure:blob-hosted' observation kinds. The libp2p gossipsub topic
-- 'elohim/inventory/blob' is the legacy name for what is now formally the
-- observation cursor announcement stream for blob-served/blob-hosted.
-- See: genesis/docs/superpowers/specs/2026-05-11-observation-event-layer-design.md §10 Stage 8.
```

Same shape for `projection_events/up.sql` and the system_metrics migration.

- [ ] **Step 3: Verify migrations still apply (no schema change)**

Run: `cargo build --lib`
Expected: success

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/migrations/
git commit -m "docs(migrations): reclassify peer_blob_inventory et al as observation projections"
```

---

## Task 8.2: CLAUDE.md notes for reclassified tables

**Files:**
- Modify: `elohim/elohim-storage/CLAUDE.md` (or root `CLAUDE.md` if no local one — check)

- [ ] **Step 1: Check for local CLAUDE.md**

Run: `ls elohim/elohim-storage/CLAUDE.md 2>/dev/null; ls CLAUDE.md`

- [ ] **Step 2: Add an "Observation layer projections" section**

If a local CLAUDE.md exists, add a section explaining which existing tables now constitute observation projections; otherwise add to root CLAUDE.md under the "Architecture" section.

```markdown
### Observation Layer Projections

Per `genesis/docs/superpowers/specs/2026-05-11-observation-event-layer-design.md`, the following existing operational SQL tables are observation projections:

- `peer_blob_inventory` — `infrastructure:blob-served` and `infrastructure:blob-hosted`
- `system_metrics` — `infrastructure:system-sample` (per-node only)
- `projection_events` — operational log of projector acks; remains as-is

The Observation primitive lives in the `observations` table; aggregations are in the `observation_diversity_summary` view. New observation kinds are declared in pillar manifests under `observation_kinds`.
```

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/CLAUDE.md  # or root CLAUDE.md
git commit -m "docs(claude): document observation-layer projection table reclassification"
```

---

# Final verification

## Task F.1: End-to-end integration test

**Files:**
- Create: `elohim/elohim-storage/tests/observation_e2e_test.rs`

- [ ] **Step 1: Write the smoke test**

Create the file:

```rust
use elohim_storage::db::test_helpers::test_connection;
use elohim_storage::graduation::evaluator::{GraduationEvaluator, GraduationConfig};
use elohim_storage::graduation::diversity::DiversityThreshold;
use elohim_storage::observation::manager::ObservationManagerBackend;
use elohim_storage::observation::wire::Observation;

#[tokio::test]
async fn end_to_end_observation_then_graduation() {
    let mut conn = test_connection();
    let mgr = ObservationManagerBackend::new_in_memory("agent:witness".into());

    // Stage 1: observations stream in from diverse households
    for (i, hh, region) in [
        (0u64, "h1", "us-west"),
        (1, "h2", "us-east"),
        (2, "h3", "eu-west"),
        (3, "h1", "us-west"),
        (4, "h2", "us-east"),
    ].iter() {
        mgr.append_local(&mut conn, Observation {
            seq: *i,
            subject_cid: Some("doorway:civic-hub".into()),
            observation_kind: "infrastructure:doorway-heartbeat".into(),
            observer_household_cid: Some(format!("household:{}", hh)),
            observer_region: Some(region.to_string()),
            payload_json: r#"{"peer_count":12,"uptime_secs":86400}"#.into(),
            ..Observation::test_fixture()
        }).await.unwrap();
    }

    // Stage 2: evaluator detects threshold met, emits attestation plan
    let mut evaluator = GraduationEvaluator::new(vec![
        GraduationConfig::attestation(
            "infrastructure:doorway-heartbeat",
            "doorway-health",
            DiversityThreshold { distinct_households: Some(3), distinct_regions: Some(2), min_count: Some(5), ..Default::default() },
        ),
    ]);
    let plans = evaluator.tick(&mut conn, 1715420400).unwrap();
    assert_eq!(plans.attestations.len(), 1);
    let plan = &plans.attestations[0];
    assert_eq!(plan.attestation_content_type, "attestation:doorway-health");
    assert_eq!(plan.subject_cid, "doorway:civic-hub");
    assert_eq!(plan.observation_refs.len(), 5);
    assert_eq!(plan.proof_class, "witness");

    // Stage 3: every observation ref is iroh-shaped
    for r in &plan.observation_refs {
        assert!(r.starts_with("iroh://agent:witness@"));
    }
}
```

- [ ] **Step 2: Run**

Run: `cargo test --test observation_e2e_test`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/tests/observation_e2e_test.rs
git commit -m "test(observation): end-to-end smoke from append through graduation"
```

---

## Task F.2: Cross-DNA-pillar verification

- [ ] **Step 1: Run all storage tests**

Run: `cd elohim/elohim-storage && CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo test 2>&1 | tail -30`
Expected: PASS

- [ ] **Step 2: Run all sweettests**

Run: `cd elohim/holochain/tests/sweettest && CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__holochain__tests__sweettest/dev cargo test 2>&1 | tail -30`
Expected: PASS

- [ ] **Step 3: Run schema validation**

Run: `pnpm run schema:test && pnpm run schema:validate`
Expected: PASS

- [ ] **Step 4: Run elohim-service tests**

Run: `cd app/elohim-library/projects/elohim-service && pnpm test`
Expected: PASS

- [ ] **Step 5: Tag the milestone**

```bash
git tag -a observation-event-layer-landed -m "Observation/Event Layer landed per 2026-05-11 spec"
```

---

# Success criteria mapping

| Spec §14 criterion | Verified by |
|---|---|
| Append → gossip → fetch → project < 5s on alpha cluster | (Deferred to alpha cluster bench; not unit-test gated.) |
| Graduation evaluator emits within one window | Task 5.5 + Task F.1 |
| Auditor follows `iroh://...@...#...` refs and re-verifies | Task F.1 verifies ref shape; auditor flow is separate concern |
| DoorwayHeartbeat / Summary / HealthAttestation removed | Task 6.1, 6.2 |
| Reclassified tables carry doc references | Task 8.1, 8.2 |
| `forget-request` flow through mishpat round-trips | (Deferred to follow-on plan; this plan establishes the substrate.) |
| Diversity scores distinguish single- vs multi-household pools | Task 5.1, F.1 |
| Manifest schema rejects malformed observation_kinds | Task 1.1 |
| Phase 11 backend wiring parity test | Tasks 4.5 + (parity bench test as follow-on) |

---

# Out of scope for this plan

The following items from the spec defer to follow-on plans:

- **Forget-request → mishpat governance round-trip.** Stage 9 in a follow-on; requires mishpat coordinator additions.
- **Alpha cluster benchmarks.** End-to-end latency measurements run on the live cluster, not in unit tests.
- **Witness-peer subscription policy.** Default subscription matrix is hard-coded in Task 4.4; role-driven subscription is a follow-on.
- **Verify-path point-in-time queries.** Schema and audit_observations table land here; the actual verifier service is a follow-on (open question §13.5 in the spec).
- **Doorway projection of observations to ATProto/ActivityPub.** Track 4 work.

These are tracked in the spec's §13 open questions and will be folded into subsequent plans as priorities surface.
