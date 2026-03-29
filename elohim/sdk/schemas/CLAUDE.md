# Protocol Schemas

Single source of truth for the Elohim Protocol's type system. JSON Schema definitions that generate TypeScript (and eventually Rust) types. Nothing downstream hand-writes types that these schemas own.

## Schema-Before-Code Rule

1. Edit the schema
2. Run codegen
3. Import the generated type

Never hand-write a TypeScript interface that mirrors a schema. It will drift.

## Directory Layout

```
v1/
├── enums/                          # Protocol enum types (→ CORE_*, ALL_*, Type alias)
│   ├── content-type.schema.json        # epic, concept, lesson, assessment, ...
│   ├── content-format.schema.json      # markdown, sophia-quiz-json, epr-composite, ...
│   ├── reach.schema.json               # private → self → intimate → ... → commons
│   ├── substrate-signal.schema.json    # attention, compute, storage, bandwidth, energy, time, resource
│   ├── mastery-level.schema.json       # not_started → remember → understand → ... → create
│   ├── engagement-type.schema.json     # view, quiz, practice, discuss, create, ...
│   ├── relationship-type.schema.json   # CONTAINS, RELATES_TO, VALIDATES, ...
│   ├── instrument-archetype.schema.json # retention-check, outcome-correlation, ...
│   ├── observation-polarity.schema.json # positive, negative
│   └── ... (24 total)
├── inputs/                         # Write-side wire types
│   ├── create-content-input.schema.json
│   ├── create-economic-event-input.schema.json
│   └── create-attestation-input.schema.json
├── views/                          # Read-side wire types
│   ├── content-view.schema.json
│   └── economic-event-view.schema.json
├── manifest/                       # App manifest schema
│   └── app-manifest.schema.json        # Validates three-leg coupling + claims + observations
└── objects/                        # Complex object schemas (geospatial, resource-nature)
    ├── resource-nature.schema.json
    ├── carrying-capacity.schema.json
    └── ...
```

## Codegen Pipeline

```bash
pnpm run schema:codegen:ts    # Generate all TypeScript from schemas
```

This runs `scripts/codegen-ts.mjs` which:

1. **Generates interfaces** from `inputs/` and `views/` schemas via `json-schema-to-typescript`
2. **Generates enum constants** from `enums/` schemas with `_dna` metadata → `CORE_*`, `ALL_*`, backward-compat alias, `Type` alias
3. **Distributes** identical files to three locations:

| Location | Consumer |
|----------|----------|
| `genesis/seeder/src/generated/` | Seeder |
| `app/elohim-app/src/app/generated/` | Angular app |
| `app/elohim-library/projects/elohim-service/src/generated/` | Shared library |

### What Gets Distributed

**Enum file** (`schema-enums.ts`): All enum constants + type aliases in one file.

**Interface files** (individual .ts per schema):
- `create-content-input.ts`
- `create-economic-event-input.ts`
- `create-attestation-input.ts`
- `content-view.ts`
- `economic-event-view.ts`

To add a new distributed file, add it to `INTERFACE_FILES` in `codegen-ts.mjs`.

## Adding a New Enum

1. Create `v1/enums/my-enum.schema.json`:
```json
{
  "$id": "epr:schema:enum:my-enum",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "MyEnum",
  "type": "string",
  "enum": ["value-a", "value-b", "value-c"],
  "_tiers": {
    "core": {
      "values": ["value-a", "value-b"],
      "rationale": "Why these are core"
    }
  },
  "_dna": {
    "constant": "MY_ENUMS",
    "zome": "content_store_integrity",
    "file": "elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs",
    "tier": "core"
  }
}
```

2. Run `pnpm run schema:codegen:ts`
3. Import: `import { MyEnum, ALL_MY_ENUMS, CORE_MY_ENUMS } from './generated/schema-enums.js'`

## Adding a New Input/View Schema

1. Create `v1/inputs/create-thing-input.schema.json` or `v1/views/thing-view.schema.json`
2. Use `$ref` for enum fields: `"myField": { "$ref": "../enums/my-enum.schema.json" }`
3. Add to `INTERFACE_FILES` in `codegen-ts.mjs` for distribution
4. Run `pnpm run schema:codegen:ts`

## Manifest Schema (app-manifest.schema.json)

Validates app manifests (like `elohim/sdk/domains/lamad/manifest.json`). Requires:

- Every content type declares **three-leg coupling** (knowledge + value + governance)
- Every content type declares **claims** (feedback: what outcomes it asserts)
- Every signal maps to a valid **SubstrateSignal** enum value
- Vocabulary includes **observations** with at least one negative-polarity entry

The manifest schema uses `$ref` to the substrate-signal enum. The test script (`test-manifest-schema.mjs`) registers referenced schemas with AJV before compilation.

## Two Type Layers

These schemas are the **protocol layer** — wire types, enums, generic metadata bag. They don't know what `PathMetadata.thumbnailUrl` means. That's the **app layer** (`elohim/sdk/domains/lamad/schemas/`).

```
Protocol (this directory)              Domain (sdk/domains/lamad/)
ContentView { metadata: {} }    →     PathMetadata { thumbnailUrl, difficulty }
CreateContentInput              →     ConceptMetadata { summary, bloomsLevel }
SubstrateSignal enum            →     coupling-map.ts per content type
```

App schemas `$ref` protocol schemas for shared primitives. Protocol never refs app schemas.

## Validation Commands

```bash
pnpm run schema:test          # 24+ assertions: coupling structure, signal validation, views
pnpm run schema:validate      # Validate 3500+ seed JSON files against content schema
pnpm run schema:check-dna     # Verify DNA constants match schema enums
pnpm run schema:codegen:ts    # Generate + distribute (or --verify to check freshness)
```

## Relationship to Rust ts-rs Types

Two parallel type generation paths exist:

| Path | Source | Output | Authoritative For |
|------|--------|--------|-------------------|
| **Protocol schemas** (this directory) | JSON Schema | TypeScript via `json-schema-to-typescript` | Wire format contract, enums, validation |
| **Rust ts-rs** (`elohim-storage/views.rs`) | Rust structs | TypeScript via `cargo test export_bindings` | Runtime field mapping, serde behavior |

Both should agree. When they diverge, the protocol schema is authoritative for field names and enum values. The Rust ts-rs types are authoritative for serde behavior (nullability, default values).

Long-term: protocol schemas generate Rust types too, eliminating the dual-path.
