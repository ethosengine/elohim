# Sprint A: Move Lamad + Create Domain Structure

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Establish `elohim/sdk/domains/` and move lamad's manifest + schemas from `app/lamad/` to `sdk/domains/lamad/`, setting the pattern for all domain manifests.

**Architecture:** Domain manifests live in the SDK because they define protocol-level vocabulary (what "concept" means, how it couples). App directories keep only reference client views (generated types, renderers). Codegen still distributes to the same consumer locations.

**Tech Stack:** JSON Schema, Node.js codegen scripts, pnpm workspace

**Parent design:** `genesis/plans/2026-03-29-domain-manifests-sdk-boundary-design.md`

---

### Task 1: Create sdk/domains directory structure

**Files:**
- Create: `elohim/sdk/domains/README.md`

**Step 1:** Create the domains directory and a README explaining the pattern:

```markdown
# Protocol Domain Definitions

Each subdirectory is a protocol domain — a vocabulary that defines content types,
coupling declarations, metadata schemas, and signals for a pillar of the protocol.

Domains are part of the SDK. They enforce integrity — what signals the protocol
MUST see. Apps compose domain vocabulary into human experiences.

| Domain | Pillar | Purpose |
|--------|--------|---------|
| lamad | Learning | Concepts, paths, assessments, mastery |
| imagodei | Identity | Humans, attestations, presence, relationships |
| shefa | Economy | Economic events, stewardship, resources |
| qahal | Social + Governance | Collectives, proposals, relationships |

## For App Developers

Import a domain manifest to build on its vocabulary. Your app manifest
references the domain and adds app-specific content types.

See `lamad/CLAUDE.md` for the reference pattern.
```

**Step 2:** Commit.

```bash
git add elohim/sdk/domains/README.md
git commit -m "chore: create sdk/domains directory for protocol domain manifests"
```

### Task 2: Move lamad manifest + schemas to sdk/domains/lamad

**Files:**
- Move: `app/lamad/manifest.json` → `elohim/sdk/domains/lamad/manifest.json`
- Move: `app/lamad/schemas/` → `elohim/sdk/domains/lamad/schemas/`
- Move: `app/lamad/scripts/codegen.mjs` → `elohim/sdk/domains/lamad/scripts/codegen.mjs`
- Move: `app/lamad/CLAUDE.md` → `elohim/sdk/domains/lamad/CLAUDE.md`

**Step 1:** Move files using git mv to preserve history:

```bash
mkdir -p elohim/sdk/domains/lamad/scripts elohim/sdk/domains/lamad/schemas
git mv app/lamad/manifest.json elohim/sdk/domains/lamad/manifest.json
git mv app/lamad/schemas/path-metadata.schema.json elohim/sdk/domains/lamad/schemas/
git mv app/lamad/schemas/concept-metadata.schema.json elohim/sdk/domains/lamad/schemas/
git mv app/lamad/schemas/assessment-metadata.schema.json elohim/sdk/domains/lamad/schemas/
git mv app/lamad/schemas/epr-composite-body.schema.json elohim/sdk/domains/lamad/schemas/
git mv app/lamad/scripts/codegen.mjs elohim/sdk/domains/lamad/scripts/codegen.mjs
git mv app/lamad/CLAUDE.md elohim/sdk/domains/lamad/CLAUDE.md
```

**Step 2:** Verify `app/lamad/` is now empty (or remove the directory):

```bash
ls app/lamad/  # should be empty or only contain leftover files
```

**Step 3:** Commit the move.

```bash
git commit -m "refactor: move lamad manifest + schemas to sdk/domains/lamad"
```

### Task 3: Update codegen paths

**Files:**
- Modify: `elohim/sdk/domains/lamad/scripts/codegen.mjs`

The codegen script uses relative paths to find the manifest and schemas. Update all path references to work from the new location (`elohim/sdk/domains/lamad/scripts/`).

**Step 1:** Read the current codegen.mjs and update the relative path constants:

- `MANIFEST_PATH`: was relative to `app/lamad/`, now relative to `elohim/sdk/domains/lamad/`
- `SCHEMA_DIR`: same adjustment
- Output paths stay the same: `app/elohim-app/src/app/lamad/generated/` and `genesis/seeder/src/generated/`
- `REPO_ROOT` calculation: from `elohim/sdk/domains/lamad/scripts/` it's `../../../../../`

**Step 2:** Run the codegen to verify:

```bash
node elohim/sdk/domains/lamad/scripts/codegen.mjs
```

Expected: generates to both output locations, identical files.

**Step 3:** Update `package.json` script if `pnpm run lamad:codegen` exists — update the script path.

**Step 4:** Verify all generated files match:

```bash
diff app/elohim-app/src/app/lamad/generated/metadata-types.ts genesis/seeder/src/generated/metadata-types.ts
diff app/elohim-app/src/app/lamad/generated/body-types.ts genesis/seeder/src/generated/body-types.ts
```

**Step 5:** Commit.

```bash
git add elohim/sdk/domains/lamad/scripts/codegen.mjs package.json
git commit -m "fix: update lamad codegen paths for sdk/domains location"
```

### Task 4: Update CLAUDE.md for new location

**Files:**
- Modify: `elohim/sdk/domains/lamad/CLAUDE.md`
- Create: `app/lamad/CLAUDE.md` (thin pointer to sdk/domains/lamad)

**Step 1:** Update `elohim/sdk/domains/lamad/CLAUDE.md` — change directory references from `app/lamad/` to `elohim/sdk/domains/lamad/`. Update the "Directory Structure" section. Keep the content about vocabulary, coupling, typed metadata patterns.

**Step 2:** Create a thin `app/lamad/CLAUDE.md` that points to the domain:

```markdown
# Lamad Reference Client Views

This directory contains the reference Angular client's view layer for the
lamad (learning) domain. Generated types, renderers, and components.

The domain vocabulary (manifest, schemas, coupling declarations) lives in
`elohim/sdk/domains/lamad/`. This directory consumes those definitions.

## Generated Types

Types in `generated/` are produced by `sdk/domains/lamad/scripts/codegen.mjs`.
Do not hand-edit — regenerate with `pnpm run lamad:codegen`.

## See Also

- Domain vocabulary: `elohim/sdk/domains/lamad/CLAUDE.md`
- Protocol schemas: `elohim/sdk/schemas/CLAUDE.md`
```

**Step 3:** Commit.

### Task 5: Tag leaked enums

**Files:**
- Modify: `elohim/sdk/schemas/v1/enums/path-visibility.schema.json`
- Modify: `elohim/sdk/schemas/v1/enums/step-type.schema.json`
- Modify: `elohim/sdk/schemas/v1/enums/completion-criteria.schema.json`

**Step 1:** Add `"_migration"` metadata to each leaked enum schema. This documents intent without breaking anything:

For each file, add after the `"_dna"` block:

```json
"_migration": {
  "target": "domain:lamad",
  "reason": "Lamad-specific vocabulary that leaked into protocol schemas during prototyping. Move to sdk/domains/lamad/ when DNA enum registration supports domain-scoped types.",
  "tagged": "2026-03-29"
}
```

**Step 2:** Verify schema tests still pass:

```bash
pnpm run schema:test
```

**Step 3:** Commit.

```bash
git commit -m "chore: tag leaked lamad enums for future domain migration"
```

### Task 6: Full verification

**Step 1:** Run all verification commands:

```bash
# Codegen from new location
node elohim/sdk/domains/lamad/scripts/codegen.mjs

# Protocol schema tests
pnpm run schema:test

# Schema validation
pnpm run schema:validate

# Seeder compiles + tests
cd genesis/seeder && npx tsc --noEmit && npx vitest run

# App builds
cd app/elohim-app && pnpm exec ng build --configuration=development
```

**Step 2:** Verify the developer experience — can you answer these questions from the directory structure?

- "What does the protocol require?" → `elohim/sdk/schemas/`
- "What does learning mean in this protocol?" → `elohim/sdk/domains/lamad/`
- "How does the reference app render learning content?" → `app/elohim-app/src/app/lamad/`

**Step 3:** Final commit if any fixups needed.
