# Protocol Schema Contract

**Date**: 2026-03-17
**Status**: Design approved, pending implementation planning
**Scope**: Cross-cutting — DNA, elohim-storage, storage-client-ts, elohim-service, genesis seeder, CI/CD

## Problem

The codebase has three disconnected type systems for the same concepts:

| Layer | Source | Auto-synced? | Example |
|-------|--------|-------------|---------|
| DNA constants | `content_store_integrity/lib.rs` | No | `CONTENT_TYPES: [&str; 12]` |
| Storage API types | `elohim-storage/views.rs` (169 types) | Yes (ts-rs → TypeScript) | `ContentView`, `CreateContentInputView` |
| Genesis content models | `elohim-service/models/` (hand-written TS) | No | `ContentNode`, `ContentType`, `ContentReach` |

The Rust-to-TypeScript pipeline (views.rs → ts-rs → storage-client-ts → Angular) works well for the **app consumption** direction. But the **genesis content** direction is disconnected: `ContentNode` has 16 content types while the DNA allows 12; `ContentReach` has 6 values while the DNA defines 8; field names don't match (`content` vs `contentBody`).

Developers discover these mismatches at runtime (seeder failure), CI (pipeline failure), or worse — after deployment. There is no dev-time enforcement, no editor feedback, and no shared contract.

## Design Principles

1. **Inversion of Control**: All layers depend on the schema contract, not on each other. The schema is the abstraction; DNA, storage, app, and genesis are implementations.
2. **As merciless as a DNA**: The schema enforces discipline at every boundary — editor, pre-push, build, CI, runtime — with the same rigor that a signed DNA enforces at the DHT layer.
3. **P2P-native versioning**: Schema versions are content-addressed (CID), form a DAG (not a line), and coexist indefinitely. There is no central authority that sunsets old versions.
4. **Schema as protocol content**: Schemas are EPR-addressable artifacts. A peer encountering unfamiliar data can fetch the schema by CID from the DHT and understand what it's looking at. Self-describing data without URL dependencies.

## P2P Design Gate

### New Entity: ProtocolSchemaDocument (Phase 4)

- **Classification**: Notarized (A) — reuses existing `Content` entry type with `content_type: "protocol-schema"`. No new DHT entry type consumed.
- **Content Address Strategy**: Content-Derived (CID) — schema identity IS the hash of its canonical content. Immutable by definition.
- **Source of Truth**: Holochain DHT — schemas are gossipped and fetchable by any peer by CID.
- **Storage Projection**: Existing `content` table with `dht_anchor_hash`. No new table.
- **HTTP Route**: Existing `GET /db/content/{cid}` filtered by content_type.

### Existing Entities Covered (source-of-truth declarations)

The schema contract validates and generates types for existing entities. All source-of-truth declarations are pre-existing:

| Entity | Classification | Source of Truth | `dht_anchor_hash` |
|--------|---------------|-----------------|-------------------|
| Content | Notarized (A) | DHT | Yes |
| LearningPath / Chapter / Step | Notarized (A) / Derived (A2) | DHT | Yes |
| Relationship / HumanRelationship | Notarized (A) | DHT | Yes |
| Human / Agent | Notarized (A) | DHT | Yes |
| EconomicEvent / Agreement | Notarized (A) | DHT | Yes |
| GovernanceSignal / Proposal / Vote | Notarized (A) | DHT (Mishpat DNA) | Yes |
| ContentMastery | Agent-Scoped + Attestation (B2) | Private source chain | Attestation has anchor |
| ContributorPresence | Notarized (A) | DHT | Yes |

### Bootstrap Constraint

The genesis schema (v1) is bootstrapped — its CID is hardcoded as a known constant, not discovered via DHT lookup. The schema defines valid content types, but the schema itself is content, requiring `"protocol-schema"` to be added to the DNA's `CONTENT_TYPES` constant.

### Schema Size Constraint

Individual schema files (1-5KB each) fit in Content entries. A full schema directory (~50-100KB) exceeds the <1KB DHT entry target. Each schema file is a separate Content entry, linked together via a manifest Content entry.

## Architecture

### Dependency Direction (IoC)

```
                Protocol Schema (abstraction)
                   elohim/sdk/schemas/
                ↑       ↑       ↑       ↑
              DNA   Storage   TypeScript  Genesis
           (verify) (codegen) (codegen)  (validate)
```

All layers depend inward on the schema. The schema depends on nothing. No layer references another layer's types directly.

### Schema Directory Structure

```
elohim/sdk/schemas/
├── current -> bafkrei-v1/              # Symlink to active version (codegen target)
│
├── bafkrei-v1/                         # CID-named: hash of canonical schema
│   ├── _protocol.json                  # Identity, lineage, compatibility
│   ├── enums/                          # Shared vocabularies
│   │   ├── content-type.schema.json
│   │   ├── content-format.schema.json
│   │   ├── reach.schema.json
│   │   ├── mastery-level.schema.json
│   │   ├── constitutional-layer.schema.json
│   │   └── relationship-type.schema.json
│   ├── entries/                        # What the DNA notarizes (DHT types)
│   │   ├── content.schema.json
│   │   ├── learning-path.schema.json
│   │   ├── human.schema.json
│   │   ├── economic-event.schema.json
│   │   └── ...
│   ├── views/                          # What storage projects (superset of entries)
│   │   ├── content-view.schema.json
│   │   ├── content-with-tags-view.schema.json
│   │   ├── path-with-details-view.schema.json
│   │   └── ...
│   ├── inputs/                         # What the API accepts
│   │   ├── create-content-input.schema.json
│   │   ├── create-path-input.schema.json
│   │   └── ...
│   └── migrations/                     # Transforms from previous versions
│       └── (none for v1 — genesis version)
│
├── frozen-types/                       # Previously generated types (read-only)
│   └── (populated as versions are superseded)
│
└── registry.json                       # Local index of known schema CIDs
```

### Protocol Metadata (`_protocol.json`)

Each schema version carries metadata that enables P2P schema negotiation:

```json
{
  "cid": "bafkrei...",
  "version": 1,
  "parent": null,
  "canRead": [],
  "compatibility": "genesis",
  "breaking": false,
  "created": "2026-03-17T00:00:00Z",
  "migrationFrom": {}
}
```

For subsequent versions:

```json
{
  "cid": "bafkrei...",
  "version": 2,
  "parent": "bafkrei-v1-cid...",
  "canRead": ["bafkrei-v1-cid..."],
  "compatibility": "backward",
  "breaking": false,
  "migrationFrom": {
    "bafkrei-v1-cid...": { "$ref": "./migrations/from-v1.migration.json" }
  }
}
```

### Schema Composition via `$ref`

Schemas use JSON Schema `$ref` to compose shared definitions. Enum schemas are referenced from entry, view, and input schemas — defined once, enforced everywhere:

```json
{
  "$id": "epr:schema:create-content-input@bafkrei...",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["id", "title"],
  "properties": {
    "id": { "type": "string", "minLength": 1 },
    "title": { "type": "string", "minLength": 1 },
    "contentType": { "$ref": "../enums/content-type.schema.json" },
    "contentFormat": { "$ref": "../enums/content-format.schema.json" },
    "contentBody": { "type": "string" },
    "reach": { "$ref": "../enums/reach.schema.json" },
    "tags": { "type": "array", "items": { "type": "string" } },
    "metadata": { "type": "object" }
  },
  "additionalProperties": false
}
```

Enum schema example (`content-type.schema.json`):

```json
{
  "$id": "epr:schema:enum:content-type@bafkrei...",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "string",
  "enum": [
    "epic", "concept", "lesson", "scenario", "assessment",
    "resource", "reflection", "discussion", "exercise",
    "example", "reference", "article"
  ],
  "description": "Content type vocabulary. Matches DNA CONTENT_TYPES constant."
}
```

## Code Generation Pipeline

### Targets

| Source | Tool | Output | Replaces |
|--------|------|--------|----------|
| `schemas/current/views/*.schema.json` | typify | Rust view structs in `views_generated.rs` | Hand-written view structs in `views.rs` |
| `schemas/current/inputs/*.schema.json` | typify | Rust input structs in `inputs_generated.rs` | Hand-written input view structs in `views.rs` |
| `schemas/current/enums/*.schema.json` | typify | Rust enum types | String constants in views.rs |
| `schemas/current/**/*.schema.json` | json-schema-to-typescript | TypeScript interfaces in `storage-client-ts/src/generated/` | ts-rs output |
| `schemas/current/enums/*.schema.json` | json-schema-to-typescript | TypeScript union types | Hand-written ContentType, ContentFormat, ContentReach in elohim-service |

### What stays hand-written

- **Diesel models** (`db/models.rs`) — ORM layer, tied to SQLite column types
- **`From<Model> for View` impls** — projection logic (parsing JSON strings, coercing booleans)
- **HTTP route handlers** (`http.rs`) — business logic, request routing
- **DNA entry types + validation callbacks** — HDK-specific patterns, verified against schema by CI
- **Migration `From<Vn> for Current` impls** — behavioral transforms between schema versions

### What gets generated

- View structs (Rust) — field names, types, serde attributes
- Input structs (Rust) — field names, types, defaults
- Enum types (Rust) — proper enums, not `String`
- TypeScript interfaces — field names, types, union types for enums
- JSON Schema validation — seed files, API requests

### Codegen Command

```bash
pnpm run schema:codegen       # Generate Rust + TypeScript from current schema
pnpm run schema:validate      # Validate seed JSON against schemas
pnpm run schema:check-dna     # Verify DNA constants match schema enums
pnpm run schema:cid           # Compute CID for current schema version
```

## Versioning Model

### Schema versions are species, not releases

In a P2P system with no central upgrade authority, schema versions coexist indefinitely. The versioning model embraces this:

- **Schema identity = CID** (content hash of canonical JSON Schema directory). Immutable, like a DNA hash.
- **Versions form a DAG**, not a linear sequence. Forks are possible and expected.
- **Compatibility is declared**, not computed. Each version's `_protocol.json` says what it `canRead`.
- **Migrations are composable pure functions**. Chain them along DAG paths for multi-version jumps.

### Version DAG

```
v1 (genesis)
├──→ v1.1 (additive: new optional field, non-breaking)
├──→ v2 (breaking: governance fields)
│    ├──→ v2.1 (additive: new enum values)
│    └──→ v2.2 (additive: stewardship extension)
│         └──→ v3 (breaking: EPR Head as first-class entry)
└──→ v1-community (community fork with custom fields)
     └──→ (can rejoin v2 via migration)
```

### Evolution Rules

1. **Additive changes are non-breaking**: New optional fields, new enum values. Old peers ignore what they don't understand. These share the same parent and increment a minor version.
2. **Unknown fields are preserved, never stripped**: If v1 data has a field v2 doesn't recognize, v2 keeps it in the raw JSON. This allows community forks to rejoin the main lineage without data loss.
3. **Breaking changes create a new branch**: Required new fields, removed fields, type changes. These require explicit migration functions and parallel conductors.
4. **Enum removal is always breaking**: Existing signed data with removed enum values can't be validated. Migration must transform the value.

### Storage Layer Version Routing

The existing `schema_version` and `validation_status` fields handle multi-version data:

```rust
pub fn resolve_content(raw: &[u8], schema_cid: &str) -> Result<ContentView> {
    match schema_cid {
        cid if cid == CURRENT_SCHEMA_CID => {
            // Direct deserialization — current version
            serde_json::from_slice::<ContentView>(raw)
        }
        cid if migration_chain.can_reach(cid, CURRENT_SCHEMA_CID) => {
            // Deserialize as raw Value, apply migration chain
            let raw_value = serde_json::from_slice::<Value>(raw)?;
            let chain = migration_chain.find_path(cid, CURRENT_SCHEMA_CID);
            let migrated = chain.apply(raw_value)?;
            Ok(serde_json::from_value::<ContentView>(migrated)?)
        }
        _ => {
            // Unknown schema — fetch from DHT by CID, or degrade
            Err(SchemaError::Unknown(schema_cid.to_string()))
        }
    }
}
```

Validation status tracks migration state per record:
- **Valid** — matches current schema
- **Migrated** — older version, successfully transformed
- **Degraded** — couldn't fully migrate (missing required fields, unknown version)
- **Healing** — migration agent actively working

### Migration Manifests

Each breaking version includes a migration manifest describing the transform:

```json
{
  "from": "bafkrei-v1...",
  "to": "bafkrei-v2...",
  "changes": [
    {
      "type": "field_added",
      "entity": "content",
      "field": "governanceLayer",
      "datatype": "integer",
      "default": null,
      "required": false
    },
    {
      "type": "enum_extended",
      "enum": "content-type",
      "added": ["composite", "simulation"],
      "removed": []
    },
    {
      "type": "field_renamed",
      "entity": "content-input",
      "from": "content",
      "to": "contentBody"
    }
  ],
  "compatibility": "backward"
}
```

Migration manifests serve three purposes:
1. **Document** the migration for humans
2. **Drive** automated migration code generation (or validate hand-written impls)
3. **Inform** the parallel conductor about what changed

### Parallel Conductor Pattern

For breaking schema changes, two conductors run simultaneously:

```
Conductor A (DNA v1, schema v1)     Conductor B (DNA v2, schema v2)
├── Serves old entries               ├── Serves new entries
├── Read-only after migration start  ├── Accepts new writes
└── Shut down after migration done   └── Becomes sole conductor

Migration Agent:
├── Reads from Conductor A
├── Transforms via migration chain
├── Writes to Conductor B
└── Updates validation_status
```

The storage layer bridges both conductors, routing queries to the appropriate one based on schema version and returning unified `ContentView` (current version) regardless of source.

## Dev-Time Enforcement

### Editor Validation (instant feedback)

Seed JSON files reference their schema via `$schema`:

```json
{
  "$schema": "../../../elohim/sdk/schemas/current/inputs/create-content-input.schema.json",
  "id": "governance-epic",
  "title": "Governance",
  "contentType": "epic",
  "contentFormat": "markdown",
  "contentBody": "..."
}
```

VS Code validates this instantly. Invalid enum values, missing required fields, unknown properties — all show as red squiggles while typing.

### Husky Pre-Push (safety net)

```bash
# .husky/pre-push addition
pnpm run schema:validate   # AJV validates all seed JSON against schemas
pnpm run schema:check-dna  # Verify DNA constants match schema enums
```

### CI (last line of defense)

The CI pipeline runs full schema validation:
1. Validate all seed JSON files against current schemas
2. Verify DNA constants match schema enum definitions
3. Verify generated Rust types are up-to-date with schemas
4. Verify generated TypeScript types are up-to-date with schemas

## What Gets Replaced

### Eliminated (hand-written types that duplicate schema)

| File | What | Replaced By |
|------|------|-------------|
| `elohim-service/models/content-node.model.ts` | `ContentNode`, `ContentType`, `ContentFormat`, `ContentReach`, `ContentRelationshipType` | Generated TypeScript interfaces from schema |
| `genesis/seeder/src/validators.ts` | Hand-mirrored DNA constant validation | JSON Schema validation via AJV |
| `genesis/seeder/src/generate-schema-types.ts` | Rust-parsing constant extraction | Schema enum files (authoritative) |
| Parts of `views.rs` | Hand-written view/input struct definitions | typify-generated structs |
| `storage-client-ts/src/generated/*.ts` (ts-rs output) | ts-rs generated types | json-schema-to-typescript output |

### Preserved (behavior that can't be generated)

| File | What | Why |
|------|------|-----|
| `db/models.rs` | Diesel ORM models | Tied to SQLite schema, not API contract |
| `views.rs` `From<Model>` impls | Projection logic | Behavioral, not structural |
| `http.rs` | Route handlers | Business logic |
| DNA `lib.rs` entry types | HDK entry definitions | HDK-specific patterns |
| DNA `lib.rs` validation callbacks | Entry validation | Behavioral, verified against schema |

## Tooling

### Rust Code Generation: typify

[typify](https://github.com/oxidecomputer/typify) (Oxide Computer, v0.6.0+) generates Rust structs from JSON Schema with serde derives. Configuration via `typify.toml`:

```toml
[generation]
# Add standard derives to all generated types
additional_derives = ["Clone", "Debug"]

[type_mod]
# serde rename_all for camelCase wire format
rename_all = "camelCase"
```

### TypeScript Code Generation: json-schema-to-typescript

[json-schema-to-typescript](https://www.npmjs.com/package/json-schema-to-typescript) (v15+) generates TypeScript interfaces. Handles `$ref`, enums, nullable, optional properties.

### Validation: AJV

[AJV](https://ajv.js.org/) (v8+) for TypeScript/Node.js JSON Schema validation. Used in:
- Seeder pre-flight validation (replaces hand-written validators)
- Husky pre-push hook
- CI pipeline

### Schema CID Computation

A script computes the CID of a schema directory by:
1. Canonicalizing all JSON files (sorted keys, minified, no trailing whitespace)
2. Computing SHA-256 hash of the concatenated canonical content
3. Encoding as CIDv1 with raw codec (`bafkrei...` prefix)

## Relationship to Existing Patterns

### EPR Specification

The protocol schema is the **machine-readable companion** to the EPR specification (`genesis/docs/content/elohim-protocol/protocol-specification.md`). The spec describes intent and architecture in prose; the schema encodes the structural contract in JSON Schema.

Schema CIDs can be referenced from the EPR spec as canonical definitions:
> "Content types are defined by the protocol schema `epr:schema:enum:content-type@bafkrei...`"

### Existing `schema_version` and `validation_status` Fields

These fields in the storage layer (`views.rs` lines 64-86) were designed for exactly this purpose. The protocol schema gives them concrete semantics:
- `schema_version` becomes a CID reference to the schema that validated the data
- `validation_status` tracks migration state within the version DAG
- `SUPPORTED_SCHEMA_VERSIONS` becomes the set of schema CIDs the storage layer can read

### Connection Strategy Pattern

The connection strategy (doorway vs direct vs Tauri) is orthogonal to schema versioning. All connection modes serve the same view types — the schema contract applies regardless of transport.

### Adapters Layer

The adapters layer (`app/elohim-app/src/app/elohim/adapters/`) continues to add computed/derived fields on top of generated types. Adapters depend on the schema-generated types, not on hand-written mirrors.

## Migration Path

### Phase 1: Bootstrap (sprint 1)

Write the initial schema files by extracting from existing Rust types and DNA constants. Set up codegen pipeline. Validate seed files. This phase delivers immediate dev-time feedback without changing any runtime code.

### Phase 2: Codegen (sprint 2)

Replace hand-written view/input structs in views.rs with typify-generated code. Replace ts-rs output with json-schema-to-typescript output. Replace ContentNode in elohim-service with generated types. Wire husky hooks.

### Phase 3: Version Infrastructure (sprint 3)

Implement CID computation, `_protocol.json` metadata, migration chain in storage layer. Prepare for first schema version bump.

### Phase 4: Living Schema (ongoing)

Schema evolves via the DAG. New versions are proposed, reviewed, and adopted. Migration manifests accompany breaking changes. The protocol describes itself.

## Success Criteria

1. **Dev-time feedback**: Editing a seed JSON file with an invalid `contentType` shows a red squiggle in VS Code before saving.
2. **Single source of truth**: No hand-written TypeScript type duplicates a schema-defined type. `ContentNode` model is eliminated.
3. **Husky enforcement**: `git push` with schema-violating seed data is rejected with a clear error message.
4. **DNA coherence**: CI fails if DNA constants diverge from schema enum definitions.
5. **Bidirectional workflow**: Whether working Rust-first or genesis-first, the schema tells you immediately what's valid.
6. **Version coexistence**: Storage layer can read data from multiple schema versions and migrate transparently.
