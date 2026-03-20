# Schema Codegen: Single Source of Truth — Design

## Problem

Enum constants (CONTENT_TYPES, CONTENT_FORMATS, REACH_LEVELS, etc.) are defined in up to 7 locations:

1. JSON Schema (`elohim/sdk/schemas/v1/enums/`) — intended source of truth
2. DNA integrity zome `lib.rs` — 12-value strict set
3. DNA integrity zome `healing.rs` — 31-value extended set
4. DNA coordinator `lib.rs` — partial duplicates of integrity
5. DNA coordinator `providers.rs` — inline copies with different values
6. TypeScript `schema-enums.ts` — generated from healing.rs (not schema)
7. Seeder `seed.ts` / `seed-sqlite.ts` — hand-written inline constants
8. MCP server — hand-written Zod schemas

Values disagree across locations. ENGAGEMENT_TYPES in lib.rs and healing.rs have entirely different values. STEP_TYPES has three incompatible versions. Four constants (PATH_VISIBILITIES, STEP_TYPES, ENGAGEMENT_TYPES, COMPLETION_CRITERIA) have no JSON schema at all.

The `resource` → `collective` rename required touching 7 files across 4 codebases and still missed `healing.rs`, breaking CI.

## Design Principle: Three-Tier Enums

Every enum has three tiers, driven by the protocol's three-legged stool (knowledge + value + governance):

### Core (DNA-notarized)
Values that couple all three legs. Immutable once signed into a DNA. Only changeable via DNA migration. The immune system.

**Content types (9)**: epic, concept, lesson, scenario, assessment, reflection, discussion, exercise, article

### Storage-Only
Cross-domain entity references. Notarized in their own DNAs (imagodei/mishpat), referenced in the content graph but not content primitives.

**Content types (3)**: human, role, collective

### Extensible
Community-defined vocabulary. Accepted by storage, not validated by DNA. Communities register new types without protocol changes. "squirrel" goes here.

**Content types (19+)**: reference, example, video, audio, book, contributor, podcast, feature, practice, documentary, etc. — plus anything communities add at runtime.

## Architecture: Schema → Codegen → Consumers

```
JSON Schema (single source of truth)
    │
    ├─→ codegen-ts.mjs (extend existing)
    │   ├── genesis/seeder/src/generated/schema-enums.ts
    │   └── app/elohim-app/src/app/generated/schema-enums.ts
    │   Emits per enum:
    │     CORE_{NAME}   — core tier values
    │     ALL_{NAME}    — all tiers (for storage validation)
    │     type {Name}   — union type of all values
    │
    ├─→ codegen-rs.mjs (NEW)
    │   └── content_store_integrity/src/generated_enums.rs
    │   Emits per enum:
    │     CORE_{NAME}: &[&str]  — core tier (DNA validation)
    │     ALL_{NAME}: &[&str]   — all tiers (healing/storage validation)
    │
    └─→ check-dna.mjs (update)
        Validates generated_enums.rs core tier matches schema _tiers.core
```

## Schema Structure

Each enum schema gains `_tiers` metadata:

```json
{
  "$id": "epr:schema:enum:content-type",
  "title": "ContentType",
  "type": "string",
  "enum": ["epic", "concept", "...all values..."],
  "_tiers": {
    "core": {
      "values": ["epic", "concept", "lesson", "scenario", "assessment",
                 "reflection", "discussion", "exercise", "article"],
      "rationale": "Three-leg coupled: knowledge + value + governance. DNA-notarized."
    },
    "storageOnly": {
      "values": ["human", "role", "collective"],
      "rationale": "Cross-domain entity references. Own DNA entry types."
    },
    "extensible": {
      "values": ["reference", "example", "video", "..."],
      "rationale": "Community vocabulary. No DNA validation."
    }
  },
  "_dna": {
    "constant": "CONTENT_TYPES",
    "zome": "content_store_integrity",
    "tier": "core"
  }
}
```

Schemas without `_tiers` default to all values being `core` (backward compatible).

## Registry Table

```sql
-- Source of truth: JSON Schema (seeded), community extensions (runtime)
-- Category C operational.
CREATE TABLE enum_registry (
    enum_name     TEXT NOT NULL,
    value         TEXT NOT NULL,
    tier          TEXT NOT NULL,  -- core, storage-only, extensible, community
    label         TEXT,
    description   TEXT,
    registered_by TEXT,           -- 'schema' for seeded, human_id for community
    created_at    TEXT NOT NULL,
    PRIMARY KEY (enum_name, value)
);
```

- Seeded on startup from JSON schema full enum
- Community types added via `POST /api/v1/registry/{enumName}`
- Storage validation queries registry (not const arrays)
- DNA validation uses compiled CORE_* constants (immutable)

## What Gets Deleted

| Location | Lines | What |
|----------|-------|------|
| `healing.rs` | ~80 | Hand-written CONTENT_TYPES, CONTENT_FORMATS, REACH_LEVELS, PATH_VISIBILITIES, STEP_TYPES, MASTERY_LEVELS, COMPLETION_CRITERIA, ENGAGEMENT_TYPES |
| `lib.rs` (integrity) | ~50 | Hand-written CONTENT_TYPES, CONTENT_FORMATS, REACH_LEVELS, MASTERY_LEVELS, PATH_VISIBILITIES, ENGAGEMENT_TYPES, STEP_TYPES, COMPLETION_CRITERIA |
| `lib.rs` (coordinator) | ~20 | Duplicate MASTERY_LEVELS, ENGAGEMENT_TYPES |
| `providers.rs` | ~20 | Inline CONTENT_TYPES, REACH_LEVELS, MASTERY_LEVELS |
| `seed.ts` | ~5 | VALID_STEP_TYPES |
| `seed-sqlite.ts` | ~10 | VALID_STEP_TYPES + mappings |

All replaced by imports from generated modules.

## What Gets Created

| File | Purpose |
|------|---------|
| `elohim/sdk/schemas/v1/enums/path-visibility.schema.json` | Missing schema |
| `elohim/sdk/schemas/v1/enums/step-type.schema.json` | Missing schema |
| `elohim/sdk/schemas/v1/enums/engagement-type.schema.json` | Missing schema |
| `elohim/sdk/schemas/v1/enums/completion-criteria.schema.json` | Missing schema |
| `elohim/sdk/schemas/scripts/codegen-rs.mjs` | Rust const generator |
| `content_store_integrity/src/generated_enums.rs` | Generated Rust constants |
| `migrations/2026-03-20-000000_enum_registry/` | Registry table |

## Pre-Push Enforcement

```bash
# New check in .husky/pre-push
schema-codegen)
  pnpm run schema:codegen:ts --verify && \
  pnpm run schema:codegen:rs --verify
```

`--verify` mode: regenerate to temp, diff against committed. Non-zero exit if stale. Catches any schema change that wasn't followed by codegen.

## Verification

1. `pnpm run schema:codegen:ts` — TS files regenerated, match committed versions
2. `pnpm run schema:codegen:rs` — Rust file regenerated, matches committed version
3. `pnpm run schema:check-dna` — DNA core constants match schema core tier
4. `pnpm run schema:test` — Schema self-tests pass
5. `pnpm run schema:validate` — All seed data validates against schema
6. `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check` — DNA compiles with generated_enums.rs
7. Add "squirrel" to registry via API → storage accepts it, DNA ignores it

## Notes

- `generated_enums.rs` is checked into git (not .gitignored). The codegen verify step catches staleness.
- App-layer extensions (content-node.model.ts `ALL_CONTENT_TYPES`) continue to extend the generated base. They import `CORE_CONTENT_TYPES` and `ALL_CONTENT_TYPES` from schema-enums.ts and spread with app-specific additions.
- The `_tiers` metadata is private (underscore prefix) — not part of the JSON Schema spec, consumed only by our codegen scripts.
- Content type `collective` moves from `_dna` to `_storageOnly`. It's a qahal entity notarized on Mishpat DNA, not a content primitive.
