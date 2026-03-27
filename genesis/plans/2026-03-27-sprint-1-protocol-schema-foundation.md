# Sprint 1: Protocol Schema Foundation

**Parent design:** `2026-03-27-typed-content-pipeline-design.md`
**Goal:** Complete the protocol schema layer so downstream sprints can inherit typed primitives.
**Scope:** Schema definitions + codegen. No Angular changes. No seeder changes (beyond verifying generated types).

> **P2P source-of-truth note:** All schemas in this sprint describe the SHAPE of existing storage entities. No new DHT entry types or database tables are created. EconomicEvent is Category A (notarized), Content is Category A, Attestation is Category A. These schemas codify what Rust `views.rs` already defines.

## Tasks

### 1. Create substrate-signal enum schema

**File:** `elohim/sdk/schemas/v1/enums/substrate-signal.schema.json`

```json
{
  "$id": "epr:schema:enum:substrate-signal",
  "title": "SubstrateSignal",
  "description": "Protocol substrate signal categories. Every app signal maps to one of these primitives.",
  "type": "string",
  "enum": ["attention", "compute", "storage", "bandwidth", "energy", "time", "resource"],
  "_dna": {
    "constant": "SUBSTRATE_SIGNALS",
    "entryField": null
  }
}
```

**Verify:** `pnpm run schema:codegen:ts` generates `CORE_SUBSTRATE_SIGNALS`, `ALL_SUBSTRATE_SIGNALS`, `SubstrateSignal` type.

### 2. Update app-manifest.schema.json SignalDeclaration

Replace the inline `substrateSignal` enum with a `$ref` to the new schema:

**File:** `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`

In `SignalDeclaration.properties.substrateSignal`, replace:
```json
"enum": ["attention", "compute", "storage", "bandwidth", "resource"]
```
with:
```json
"$ref": "../enums/substrate-signal.schema.json"
```

### 3. Add metadataSchema slot to ContentTypeDeclaration

**File:** `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`

Add to `ContentTypeDeclaration.properties`:
```json
"metadataSchema": {
  "description": "JSON Schema defining the metadata shape for this content type. Enables typed metadata access and validation. Can be inline or a $ref to a companion schema file.",
  "type": "object"
}
```

This is intentionally `"type": "object"` (accepts any JSON Schema) rather than a specific structure. The protocol validates that it IS a schema; the app validates the CONTENT of the schema.

### 4. Add bodySchema slot to ContentFormatDeclaration

**File:** `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`

Add to `ContentFormatDeclaration.properties`:
```json
"bodySchema": {
  "description": "JSON Schema defining the contentBody shape for this format. Enables typed body parsing and validation. Used by codegen to generate body interfaces.",
  "type": "object"
}
```

### 5. Create REA economic event input schema

**File:** `elohim/sdk/schemas/v1/inputs/create-economic-event-input.schema.json`

Capture the 25 fields from Rust `CreateEconomicEventInputView`. Key fields:

```json
{
  "$id": "epr:schema:input:create-economic-event-input",
  "title": "CreateEconomicEventInput",
  "description": "Input for creating REA economic events. Must match Rust CreateEconomicEventInputView.",
  "type": "object",
  "required": ["action", "provider", "receiver"],
  "properties": {
    "action": { "type": "string", "description": "REA action type (use, consume, produce, transfer, cite)" },
    "provider": { "type": "string", "description": "Agent providing the resource" },
    "receiver": { "type": "string", "description": "Agent/content receiving the resource" },
    "resourceConformsTo": { "type": "string", "description": "ResourceSpecification URI" },
    "resourceQuantityValue": { "type": "number" },
    "resourceQuantityUnit": { "type": "string" },
    "effortQuantityValue": { "type": "number" },
    "effortQuantityUnit": { "type": "string" },
    "lamadEventType": { "type": "string", "description": "App-specific event classification" },
    "contentId": { "type": "string" },
    "contributorPresenceId": { "type": "string" },
    "pathId": { "type": "string" },
    "triggeredBy": { "type": "string", "description": "ID of the signal/event that triggered this" },
    "note": { "type": "string" },
    "metadata": { "type": "object", "description": "Domain-specific metadata (parsed, not stringified)" },
    "atLocation": { "type": "string", "description": "Place ID for spatial grounding" }
  }
}
```

Also include: `hasPointInTime`, `hasDuration`, `inputOf`, `outputOf`, `resourceInventoriedAs`, `resourceClassifiedAs` (array of strings).

### 6. Create REA economic event view schema

**File:** `elohim/sdk/schemas/v1/views/economic-event-view.schema.json`

Same fields as input plus: `id` (required), `appId`, `state`, `dhtAnchorHash`, `createdAt`.

### 7. Create attestation input schema

**File:** `elohim/sdk/schemas/v1/inputs/create-attestation-input.schema.json`

Capture fields from Rust `CreateAttestationInputView`. This is needed for the signal harness to produce mastery attestations.

### 8. Extend codegen to scan new directories

**File:** `elohim/sdk/schemas/scripts/codegen-ts.mjs`

The codegen already scans `enums`, `inputs`, `views`. Verify the new files in `inputs/` and `views/` are picked up and distributed. The `substrate-signal` enum in `enums/` will be auto-discovered.

### 9. Update manifest schema tests

**File:** `elohim/sdk/schemas/scripts/test-manifest-schema.mjs`

Add tests:
- `metadataSchema` accepted as valid JSON Schema object
- `bodySchema` accepted as valid JSON Schema object
- `substrateSignal` validated against the enum schema (not inline list)
- `energy` and `time` are valid substrate signals

### 10. Update lamad manifest to use $ref for substrateSignal

**File:** `app/lamad/manifest.json`

No structural changes needed — the manifest already uses string values that match the new enum. But verify that `pnpm run schema:validate` passes with the updated manifest schema.

## Verification

```bash
# Schema tests pass
pnpm run schema:test

# Codegen produces all new types
pnpm run schema:codegen:ts

# Verify new files exist in all 3 distribution locations
ls genesis/seeder/src/generated/create-economic-event-input.ts
ls app/elohim-app/src/app/generated/create-economic-event-input.ts
ls app/elohim-library/projects/elohim-service/src/generated/create-economic-event-input.ts

# Verify substrate signal enum generated
grep "SubstrateSignal" genesis/seeder/src/generated/schema-enums.ts

# Manifest validates against updated schema
pnpm run schema:validate

# Existing seeder tests still pass
cd genesis/seeder && npx vitest run

# Existing app builds
cd app/elohim-app && pnpm run build
```

## Not In Scope

- Lamad metadata schemas (sprint 2)
- Angular type changes (sprint 2-3)
- Seeder type changes (sprint 3)
- Signal harness implementation (sprint 4)
- Renderer registry changes (sprint 4)
