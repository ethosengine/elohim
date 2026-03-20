# Schema Codegen: Single Source of Truth — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Eliminate 7 duplicate locations for enum constants by making JSON Schema the single source of truth, with codegen producing both Rust and TypeScript consumers.

**Architecture:** JSON Schema defines three-tier enums (core/storageOnly/extensible). Two codegen scripts emit constants for Rust (DNA zomes) and TypeScript (app + seeder). Hand-written constants are deleted and replaced with imports from generated modules. Pre-push hook enforces codegen freshness. Registry table enables runtime extension.

**Tech Stack:** Node.js (codegen scripts), Rust (generated_enums.rs), TypeScript (schema-enums.ts), SQLite (enum_registry migration), JSON Schema.

**Design doc:** `genesis/plans/2026-03-20-schema-codegen-single-source-design.md`

**Scope:** 8 enum constants — CONTENT_TYPES, CONTENT_FORMATS, REACH_LEVELS, MASTERY_LEVELS, PATH_VISIBILITIES, STEP_TYPES, ENGAGEMENT_TYPES, COMPLETION_CRITERIA. The ~70 other constants in lib.rs are out of scope.

---

### Task 1: Create Missing JSON Schemas

**Files:**
- Create: `elohim/sdk/schemas/v1/enums/path-visibility.schema.json`
- Create: `elohim/sdk/schemas/v1/enums/step-type.schema.json`
- Create: `elohim/sdk/schemas/v1/enums/engagement-type.schema.json`
- Create: `elohim/sdk/schemas/v1/enums/completion-criteria.schema.json`

Four constants have no schema yet. Create them with `_tiers` metadata. Values come from healing.rs (the de facto canonical source for the extended set).

**path-visibility.schema.json:**
```json
{
  "$id": "epr:schema:enum:path-visibility",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "PathVisibility",
  "description": "Visibility levels for learning paths.",
  "type": "string",
  "enum": ["private", "intimate", "unlisted", "community", "public", "draft"],
  "_tiers": {
    "core": {
      "values": ["private", "unlisted", "community", "public"],
      "rationale": "DNA-notarized visibility. Gates content distribution."
    },
    "extensible": {
      "values": ["intimate", "draft"],
      "rationale": "Storage-level refinements. Draft is pre-publication state."
    }
  },
  "_dna": {
    "constant": "PATH_VISIBILITIES",
    "zome": "content_store_integrity",
    "file": "elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs",
    "tier": "core"
  }
}
```

**step-type.schema.json:**
```json
{
  "$id": "epr:schema:enum:step-type",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "StepType",
  "description": "Types of steps within learning paths.",
  "type": "string",
  "enum": ["content", "read", "path", "external", "practice", "assess", "video", "interactive", "checkpoint", "reflection"],
  "_tiers": {
    "core": {
      "values": ["content", "path", "external", "checkpoint", "reflection"],
      "rationale": "DNA-notarized step types. Structural path elements."
    },
    "extensible": {
      "values": ["read", "practice", "assess", "video", "interactive"],
      "rationale": "Rendering hints. Storage-level refinements of core types."
    }
  },
  "_dna": {
    "constant": "STEP_TYPES",
    "zome": "content_store_integrity",
    "file": "elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs",
    "tier": "core"
  }
}
```

**engagement-type.schema.json:**
```json
{
  "$id": "epr:schema:enum:engagement-type",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "EngagementType",
  "description": "Types of learner engagement with content.",
  "type": "string",
  "enum": ["view", "quiz", "practice", "discuss", "create", "peer", "teach", "apply"],
  "_tiers": {
    "core": {
      "values": ["view", "quiz", "practice", "discuss", "create", "peer", "teach", "apply"],
      "rationale": "All engagement types are protocol-level — they drive recognition flows."
    }
  },
  "_dna": {
    "constant": "ENGAGEMENT_TYPES",
    "zome": "content_store_integrity",
    "file": "elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs",
    "tier": "core"
  }
}
```

Note: healing.rs values (view, quiz, practice, discuss, create, peer, teach, apply) are canonical. lib.rs has different values (comment, review, contribute, path_step, refresh) which are WRONG and will be replaced.

**completion-criteria.schema.json:**
```json
{
  "$id": "epr:schema:enum:completion-criteria",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "CompletionCriteria",
  "description": "Criteria for completing a learning path step.",
  "type": "string",
  "enum": ["all-required", "pass-assessment", "view-content"],
  "_tiers": {
    "core": {
      "values": ["all-required", "pass-assessment", "view-content"],
      "rationale": "All completion criteria are protocol-level — they gate path progression."
    }
  },
  "_dna": {
    "constant": "COMPLETION_CRITERIA",
    "zome": "content_store_integrity",
    "file": "elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs",
    "tier": "core"
  }
}
```

Note: healing.rs values are canonical. lib.rs has different values (view, quiz_pass, practice_complete, reflection_submit, time_spent) which are WRONG.

**Step: Add `_tiers` to existing schemas**

Update the 4 existing enum schemas that have `_dna` metadata to add `_tiers`:
- `content-type.schema.json` — core: 9 types, storageOnly: 3, extensible: rest
- `content-format.schema.json` — core: 6, extensible: 15 more from healing.rs
- `reach.schema.json` — core: all 8 (no tiers needed, all DNA-notarized)
- `mastery-level.schema.json` — core: 8 (Bloom's), extensible: 3 aliases (recognize, recall, synthesize)

**Verify:** `pnpm run schema:test` passes

**Commit:** `feat(schema): add 4 missing enum schemas + _tiers metadata`

---

### Task 2: Rust Codegen Script

**Files:**
- Create: `elohim/sdk/schemas/scripts/codegen-rs.mjs`

Script reads all enum schemas with `_dna` metadata and generates a single Rust file.

```javascript
#!/usr/bin/env node
/**
 * Generates Rust enum constants from protocol JSON schemas.
 *
 * Usage:
 *   node codegen-rs.mjs           # Generate
 *   node codegen-rs.mjs --verify  # Check if generated file is stale
 */
```

For each schema with `_dna`:
- Read `_tiers` → emit `CORE_{CONSTANT}: &[&str]` (core tier values)
- Read full `enum` → emit `ALL_{CONSTANT}: &[&str]` (all values)

Output file: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/generated_enums.rs`

Format:
```rust
//! AUTO-GENERATED from protocol JSON schemas.
//! DO NOT EDIT — regenerate with: pnpm run schema:codegen:rs
//!
//! Source: elohim/sdk/schemas/v1/enums/*.schema.json

/// Core content types — DNA-notarized, three-leg coupled.
pub const CORE_CONTENT_TYPES: &[&str] = &[
    "epic", "concept", "lesson", ...
];

/// All content types — includes storage-only and extensible.
pub const ALL_CONTENT_TYPES: &[&str] = &[
    "epic", "concept", "lesson", ..., "human", "role", "collective", ..., "video", ...
];

// ... repeat for each enum with _dna metadata
```

`--verify` mode: generate to temp file, diff against committed. Exit 1 if stale.

Add `schema:codegen:rs` script to root `package.json`.

**Verify:** `node elohim/sdk/schemas/scripts/codegen-rs.mjs` produces valid Rust

**Commit:** `feat(schema): add Rust codegen script`

---

### Task 3: Update TypeScript Codegen

**Files:**
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs`

Currently generates from `json-schema-to-typescript`. Needs a separate mode that reads `_dna` schemas and emits tier-aware constants to the existing `schema-enums.ts` output locations.

Add a function that:
1. Reads all enum schemas with `_dna` metadata
2. For each, emits `CORE_{NAME}` (core tier) and `ALL_{NAME}` (full enum) alongside existing exports
3. Updates both output locations (seeder + app)
4. Supports `--verify` mode

The existing `CONTENT_TYPES`, `CONTENT_FORMATS` etc. exports remain for backward compat but are now aliases for `ALL_*`.

**Verify:** `pnpm run schema:codegen:ts` regenerates both schema-enums.ts files

**Commit:** `feat(schema): tier-aware TypeScript codegen`

---

### Task 4: Generate Rust Constants + Wire Into DNA

**Files:**
- Create: `content_store_integrity/src/generated_enums.rs` (generated)
- Modify: `content_store_integrity/src/lib.rs` — delete 8 hand-written const arrays, add `pub mod generated_enums;`, re-export `CORE_*` as the existing names for backward compat
- Modify: `content_store_integrity/src/healing.rs` — delete 8 hand-written const arrays, import `ALL_*` from `generated_enums`
- Modify: `content_store/src/lib.rs` — delete duplicate MASTERY_LEVELS + ENGAGEMENT_TYPES, import from integrity crate
- Modify: `content_store/src/providers.rs` — delete inline CONTENT_TYPES, REACH_LEVELS, MASTERY_LEVELS, import from integrity crate

**Step 1:** Run `pnpm run schema:codegen:rs` to generate `generated_enums.rs`

**Step 2:** In `content_store_integrity/src/lib.rs`:
- Add `pub mod generated_enums;`
- Delete the 8 const arrays (CONTENT_TYPES, CONTENT_FORMATS, REACH_LEVELS, MASTERY_LEVELS, PATH_VISIBILITIES, ENGAGEMENT_TYPES, STEP_TYPES, COMPLETION_CRITERIA)
- Add re-exports:
```rust
pub use generated_enums::{
    CORE_CONTENT_TYPES as CONTENT_TYPES,
    CORE_CONTENT_FORMATS as CONTENT_FORMATS,
    CORE_REACH_LEVELS as REACH_LEVELS,
    CORE_MASTERY_LEVELS as MASTERY_LEVELS,
    CORE_PATH_VISIBILITIES as PATH_VISIBILITIES,
    CORE_ENGAGEMENT_TYPES as ENGAGEMENT_TYPES,
    CORE_STEP_TYPES as STEP_TYPES,
    CORE_COMPLETION_CRITERIA as COMPLETION_CRITERIA,
};
```

Note: fixed-size array types `[&str; N]` become `&[&str]` slices (generated). Update any code that expects fixed arrays.

**Step 3:** In `healing.rs`:
- Delete the 8 const arrays
- Import: `use super::generated_enums::*;`
- Replace references: `CONTENT_TYPES` → `ALL_CONTENT_TYPES`, etc.

**Step 4:** In `content_store/src/lib.rs` and `providers.rs`:
- Delete duplicate/inline constants
- Import from integrity: `use content_store_integrity::{CONTENT_TYPES, REACH_LEVELS, MASTERY_LEVELS};`

**Verify:** `cargo check` in the DNA workspace (WASM target)

**Commit:** `refactor(dna): replace hand-written constants with schema-generated`

---

### Task 5: Delete TypeScript Duplicates

**Files:**
- Modify: `genesis/seeder/src/seed.ts` — delete `VALID_STEP_TYPES`, import from schema-enums
- Modify: `genesis/seeder/src/seed-sqlite.ts` — delete `VALID_STEP_TYPES`, import from schema-enums
- Regenerate: `genesis/seeder/src/generated/schema-enums.ts` via codegen
- Regenerate: `app/elohim-app/src/app/generated/schema-enums.ts` via codegen

**Verify:** `cd genesis/seeder && pnpm exec vitest run` — all tests pass

**Commit:** `refactor(seeder): import constants from schema-generated enums`

---

### Task 6: Update check-dna.mjs

**Files:**
- Modify: `elohim/sdk/schemas/scripts/check-dna.mjs`

Currently parses `pub const NAME: [&str; N]` from lib.rs. After Task 4, the constants are in `generated_enums.rs` and use `&[&str]` (slice) syntax instead of fixed-size arrays.

Update the regex to match either format and point at `generated_enums.rs` instead of `lib.rs`.

Also update to check `_tiers.core` values against `CORE_*` constants (not full enum against `CONTENT_TYPES`).

**Verify:** `pnpm run schema:check-dna` passes

**Commit:** `fix(schema): update check-dna for generated_enums format`

---

### Task 7: Enum Registry Migration + API

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-03-20-000000_enum_registry/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-03-20-000000_enum_registry/down.sql`
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs` — add `enum_registry` table
- Modify: `elohim/elohim-storage/src/db/models.rs` — add `EnumRegistryEntry`, `NewEnumRegistryEntry`
- Create: `elohim/elohim-storage/src/db/enum_registry.rs` — CRUD
- Modify: `elohim/elohim-storage/src/db/mod.rs` — add module
- Create: `elohim/elohim-storage/src/api/registry.rs` — `GET/POST /api/v1/registry/{enumName}`
- Modify: `elohim/elohim-storage/src/api/mod.rs` — add dispatch

The registry table is seeded from JSON schema on storage startup. Community types are added via API.

**Verify:** `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check`

**Commit:** `feat(storage): enum registry table for extensible vocabulary`

---

### Task 8: Pre-Push Hook + Verification

**Files:**
- Modify: `.husky/pre-push` — add `schema-codegen` project detection and verify commands

Detection: trigger when `elohim/sdk/schemas/` changes

```bash
schema-codegen)
  pnpm run schema:codegen:ts --verify 2>&1 && \
  pnpm run schema:codegen:rs --verify 2>&1
  rc=$?
  ;;
```

**Verify:**
1. Edit a schema enum value without running codegen → pre-push fails
2. Run codegen → pre-push passes
3. Full test: `pnpm run schema:test && pnpm run schema:validate && pnpm run schema:check-dna && pnpm run schema:codegen:ts --verify && pnpm run schema:codegen:rs --verify`

**Commit:** `feat(hooks): enforce schema codegen freshness on push`

---

### Task 9: Final Verification + Cleanup

**Step 1:** Full pipeline simulation:
```bash
pnpm run schema:test
pnpm run schema:validate
pnpm run schema:check-dna
pnpm run schema:codegen:ts --verify
pnpm run schema:codegen:rs --verify
cd genesis/seeder && pnpm exec vitest run
cd app/elohim-app && pnpm exec ng build --configuration=development
```

**Step 2:** Verify no hand-written constants remain:
```bash
# Should find ZERO matches in these files:
grep -n "pub const CONTENT_TYPES\|pub const CONTENT_FORMATS\|pub const REACH_LEVELS\|pub const MASTERY_LEVELS" \
  elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs \
  elohim/holochain/dna/elohim/zomes/content_store_integrity/src/healing.rs \
  elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs \
  elohim/holochain/dna/elohim/zomes/content_store/src/providers.rs
```

**Step 3:** Push with hooks (no HUSKY=0):
```bash
git push
```

**Commit:** `chore: final verification of schema single source of truth`
