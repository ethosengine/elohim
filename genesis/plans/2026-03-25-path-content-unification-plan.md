# Path-Content Unification Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Eliminate the parallel LearningPath type system by making paths ContentNodes with `contentType: "path"` and `contentFormat: "epr-composite"`, re-seed from genesis, and remove all path-specific infrastructure.

**Architecture:** Clean break (Approach C) — all data originates from genesis seeder, no user-created paths to preserve. Protocol schema drives changes (IoC). Storage drops path tables, import pipeline writes paths as ContentNodes + Relationships, doorway and Angular consume via unified content API.

**Tech Stack:** Rust (elohim-storage, doorway), JSON Schema (protocol schema), TypeScript/Angular (frontend), Diesel (migrations), pnpm (workspace)

**Design doc:** `genesis/plans/2026-03-25-path-content-unification-design.md`

**P2P source-of-truth:** All entities classified in the design doc. No new entity types — paths map into existing Content (A — Notarized) and Relationship (A — Notarized) classifications. Storage tables are projections of DHT state.

---

### Task 1: Protocol Schema — Add path content type and epr-composite format

Schema-first. Everything downstream reads from these enums.

**Files:**
- Modify: `elohim/sdk/schemas/v1/enums/content-type.schema.json`
- Modify: `elohim/sdk/schemas/v1/enums/content-format.schema.json`

**Step 1: Add "path" to contentType enum**

In `content-type.schema.json`, add `"path"` to the `core` array:
```json
"core": [
  "epic", "concept", "lesson", "scenario", "assessment",
  "reflection", "discussion", "exercise", "article", "path"
]
```

**Step 2: Add "epr-composite" to contentFormat enum**

In `content-format.schema.json`, add `"epr-composite"` to the `core` array:
```json
"core": ["markdown", "html", "video", "audio", "interactive", "external", "epr-composite"]
```

**Step 3: Run schema validation**

```bash
pnpm run schema:test
pnpm run schema:validate
```
Expected: All assertions pass. If `schema:check-dna` fails, that's expected — DNA constants will be updated in Task 9.

**Step 4: Commit**

```bash
git add elohim/sdk/schemas/v1/enums/
git commit -m "feat(schema): add 'path' content type and 'epr-composite' format"
```

---

### Task 2: Storage — Diesel migration to drop path tables

Clean break — these tables will never be written to again after re-seed.

**Files:**
- Create: `elohim/elohim-storage/migrations/YYYY-MM-DD-NNNNNN_drop_path_tables/up.sql`
- Create: `elohim/elohim-storage/migrations/YYYY-MM-DD-NNNNNN_drop_path_tables/down.sql`

**Step 1: Create the migration directory**

```bash
cd elohim/elohim-storage
# Use diesel CLI or create manually with today's date
mkdir -p migrations/2026-03-25-100000_drop_path_tables
```

**Step 2: Write up.sql**

```sql
-- Drop path-related tables (clean break — all data re-seeded from genesis)
-- Order matters: children first due to foreign keys
DROP TABLE IF EXISTS path_attestations;
DROP TABLE IF EXISTS steps;
DROP TABLE IF EXISTS chapters;
DROP TABLE IF EXISTS path_tags;
DROP TABLE IF EXISTS paths;
```

**Step 3: Write down.sql**

Recreate the tables for rollback. Copy the original CREATE TABLE statements from the path creation migration files. Check:
- `elohim/elohim-storage/migrations/` — find the original migration that created `paths`, `chapters`, `steps`, `path_tags`, `path_attestations`
- Copy those CREATE TABLE + CREATE INDEX statements into `down.sql`

**Step 4: Verify migration compiles**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5
```

Note: This will cause compilation errors in path Rust code that references these tables. That's expected — Task 3 removes that code.

**Step 5: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-03-25-100000_drop_path_tables/
git commit -m "feat(storage): add migration to drop path tables"
```

---

### Task 3: Storage — Remove path Rust code

Remove all path-specific models, queries, views, routes, and services. This is the largest task — do it methodically.

**Files:**
- Delete: `elohim/elohim-storage/src/db/paths_diesel.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs` — remove `pub mod paths_diesel;`
- Modify: `elohim/elohim-storage/src/db/models.rs` — remove Path, NewPath, Chapter, NewChapter, Step, NewStep structs and their Diesel derives
- Modify: `elohim/elohim-storage/src/views.rs` — remove PathView, PathWithDetailsView, ChapterView, StepView, ChapterWithStepsView, PathAttestationView, PathWithStepsView, CreatePathInputView, CreateChapterInputView, CreateStepInputView, and all `impl From` conversions for path types
- Modify: `elohim/elohim-storage/src/http.rs` — remove `/db/paths/*` route registrations and handler functions, remove `/db/path-extensions/*` routes if path-extensions are also being removed
- Modify: `elohim/elohim-storage/src/lib.rs` — remove any path service module declarations
- Modify: any service files that reference path types

**Step 1: Delete paths_diesel.rs**

```bash
rm elohim/elohim-storage/src/db/paths_diesel.rs
```

**Step 2: Remove path module from db/mod.rs**

Remove the line `pub mod paths_diesel;` from `elohim/elohim-storage/src/db/mod.rs`.

**Step 3: Remove path models from models.rs**

Remove these structs from `elohim/elohim-storage/src/db/models.rs`:
- `Path` (Queryable) — around lines 148-165
- `NewPath` (Insertable) — around lines 170-183
- `Chapter` (Queryable) — around lines 213-223
- `NewChapter` (Insertable) — around lines 225-235
- `Step` (Queryable) — around lines 247-262
- `NewStep` (Insertable) — around lines 264-278

Also remove the Diesel `table!` macros for `paths`, `chapters`, `steps` if they're in a schema file. Check `elohim/elohim-storage/src/schema.rs` (auto-generated by Diesel).

**Step 4: Remove path views from views.rs**

Remove from `elohim/elohim-storage/src/views.rs`:
- `PathView` struct and its `From<Path>` impl
- `ChapterView` struct and its `From<Chapter>` impl
- `StepView` struct and its `From<Step>` impl
- `PathAttestationView` struct
- `ChapterWithStepsView` struct
- `PathWithDetailsView` struct
- `PathWithStepsView` struct
- `CreatePathInputView` struct
- `CreateChapterInputView` struct
- `CreateStepInputView` struct

**Step 5: Remove path routes from http.rs**

In `elohim/elohim-storage/src/http.rs`:
- Remove the route registrations for `/db/paths`, `/db/paths/{id}`, `/db/paths/bulk` (around lines 1495-1512)
- Remove the handler functions for these routes (search for `paths` handlers)
- Remove `/db/path-extensions/*` routes and handlers
- Remove any path-related service initialization

**Step 6: Fix compilation — chase all references**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | grep "^error" | head -20
```

Fix each compilation error by removing references to deleted types. Common patterns:
- Service structs that hold a path service — remove the field
- Stats endpoints that count paths — remove or adjust
- Cache stream sending path events — handled in Task 4
- Any `use` statements importing path types

**Step 7: Run tests**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test 2>&1 | tail -10
```

Remove any test files/functions that test path-specific functionality. Content tests should still pass.

**Step 8: Regenerate TypeScript types**

```bash
cd elohim/elohim-storage
cargo test export_bindings
```

This regenerates `elohim/sdk/storage-client-ts/src/generated/`. Verify that `PathView.ts`, `PathWithDetailsView.ts`, `ChapterView.ts`, `StepView.ts`, etc. are no longer generated.

**Step 9: Commit**

```bash
git add -A elohim/elohim-storage/
git add elohim/sdk/storage-client-ts/src/generated/
git commit -m "refactor(storage): remove path-specific tables, models, views, and routes

Paths are now ContentNodes with contentType 'path'. Step connections
are Relationships. The parallel path type system is eliminated."
```

---

### Task 4: Storage — Update cache_stream to send paths as content

The cache stream currently sends separate `cache.path` events. Since paths are now content, there is no separate path query.

**Files:**
- Modify: `elohim/elohim-storage/src/cache_stream.rs`

**Step 1: Read current cache_stream.rs**

Understand the current event types: `cache.content`, `cache.path`, `cache.human`, `cache.relationship`, `cache.done`.

**Step 2: Remove cache.path event production**

Remove the section that queries the `paths` table and sends `cache.path` events. Paths will come through as `cache.content` events since they're now rows in the content table.

**Step 3: Update cache.done counts**

The `cache.done` event currently reports `{"content": N, "paths": M, ...}`. Remove the `paths` count. Content count now includes paths.

**Step 4: Build and test**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test 2>&1 | tail -5
```

**Step 5: Commit**

```bash
git add elohim/elohim-storage/src/cache_stream.rs
git commit -m "refactor(storage): remove cache.path events — paths are content"
```

---

### Task 5: Import Pipeline — Update seeder to write paths as ContentNodes

The seeder currently calls `POST /db/paths/bulk`. It needs to produce ContentNodes with `contentType: "path"` and Relationships for step edges.

**Files:**
- Modify: `genesis/seeder/src/seed.ts` — change path seeding to use content + relationships endpoints
- Modify: seed data JSON files in `genesis/seeder/data/lamad/` — restructure path data as ContentNode format
- Possibly modify: `genesis/seeder/src/doorway-client.ts` or equivalent HTTP client

**Step 1: Read current path seeding code**

Read `genesis/seeder/src/seed.ts` around lines 1930-1970 where `doorwayClient.bulkCreatePaths(transformedPaths)` is called. Understand the current data shape.

**Step 2: Create path-to-content transformer**

Write a function that transforms the current path seed data into:

a) A `CreateContentInput` object:
```typescript
{
  id: path.id,
  title: path.title,
  description: path.description,
  contentType: 'path',
  contentFormat: 'epr-composite',
  contentBody: JSON.stringify({
    schemaVersion: 1,
    pathType: path.pathType || 'journey',
    layout: 'sequential',
    sections: transformChaptersToSections(path.chapters)
  }),
  metadata: {
    difficulty: path.difficulty,
    estimatedDuration: path.estimatedDuration,
    thumbnailUrl: path.thumbnailUrl,
    thumbnailAlt: path.thumbnailAlt,
    pathType: path.pathType
  },
  reach: path.visibility || 'public',
  createdBy: path.createdBy,
  tags: path.tags
}
```

b) An array of `CreateRelationshipInput` objects for step graph edges:
```typescript
path.chapters.flatMap((chapter, ci) =>
  chapter.steps.map((step, si) => ({
    id: `${path.id}-step-${ci}-${si}`,
    sourceId: path.id,
    targetId: step.resourceId,
    relationshipType: 'step',
    confidence: 1.0,
    inferenceSource: 'explicit',
    metadata: { orderIndex: globalStepIndex }
  }))
)
```

**Step 3: Transform sections helper**

The `transformChaptersToSections` function maps the current chapter/step structure to the nested body schema:

```typescript
function transformChaptersToSections(chapters: any[]): Section[] {
  return chapters.map(chapter => ({
    id: chapter.id,
    title: chapter.title,
    description: chapter.description,
    level: 'unit',
    items: chapter.steps.map((step: any, i: number) => ({
      ref: `epr:${step.resourceId}`,
      role: step.stepType === 'checkpoint' ? 'checkpoint' : 'step',
      title: step.title,
      narrative: step.description,
      completionCriteria: { type: 'view' }
    }))
  }));
}
```

**Step 4: Replace bulkCreatePaths with bulkCreateContent + bulkCreateRelationships**

In `seed.ts`, replace:
```typescript
// OLD
await doorwayClient.bulkCreatePaths(transformedPaths);
```
With:
```typescript
// NEW
const contentItems = paths.map(pathToContent);
const relationships = paths.flatMap(pathToRelationships);
await doorwayClient.bulkCreateContent(contentItems);
await doorwayClient.bulkCreateRelationships(relationships);
```

**Step 5: Verify bulk relationship endpoint exists**

Check that `POST /db/relationships/bulk` exists in elohim-storage. If not, it may need to be added (check `http.rs` for relationship routes). The content bulk endpoint `POST /db/content/bulk` already exists.

**Step 6: Test with dry run if available**

```bash
cd genesis/seeder
pnpm run seed -- --dry-run
```

**Step 7: Commit**

```bash
git add genesis/seeder/
git commit -m "feat(seeder): write paths as ContentNodes with epr-composite body

Paths seeded via /db/content/bulk with contentType 'path' and
contentFormat 'epr-composite'. Step edges seeded as Relationships."
```

---

### Task 6: Re-seed and verify

Full integration test — fresh database, seed all content, verify paths load as ContentNodes.

**Step 1: Start storage service**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo run --release
```

**Step 2: Run seeder**

```bash
cd genesis/seeder
pnpm run seed
```

**Step 3: Verify paths exist as content**

```bash
# List content with type "path"
curl -s http://localhost:8090/db/content?contentType=path | jq '.items | length'
# Should return the number of paths that were previously in /db/paths

# Get a specific path by ID
curl -s http://localhost:8090/db/content/PATH_ID_HERE | jq '.contentType, .contentFormat'
# Should return "path" and "epr-composite"

# Verify body has sections
curl -s http://localhost:8090/db/content/PATH_ID_HERE | jq '.contentBody | fromjson | .sections | length'
# Should return number of sections/chapters
```

**Step 4: Verify step relationships exist**

```bash
curl -s "http://localhost:8090/db/relationships?sourceId=PATH_ID_HERE&relationshipType=step" | jq '.items | length'
# Should return number of steps in that path
```

**Step 5: Verify cache stream includes paths**

```bash
curl -s -N http://localhost:8090/api/v1/cache/stream 2>&1 | head -50
# Should see cache.content events that include path ContentNodes
# Should NOT see cache.path events
```

**Step 6: Commit any fixes**

If any issues found during verification, fix and commit.

---

### Task 7: Doorway — Remove path-specific code

Doorway should never know the word "path."

**Files:**
- Modify: `doorway/doorway-service/src/projection/warm.rs` — delete `fetch_paths_individually` function and its call
- Delete or modify: `doorway/doorway-service/src/projection/collections/paths.rs` — remove path projection schema
- Modify: `doorway/doorway-service/src/projection/collections/mod.rs` — remove paths module
- Modify: any doorway code that references `"LearningPath"` doc_type

**Step 1: Delete fetch_paths_individually from warm.rs**

In `doorway/doorway-service/src/projection/warm.rs`:
- Remove the `fetch_paths_individually` function (lines ~147-224)
- Remove its call in `warm_projection_cache` (around line 55)
- Remove `path_count` from `WarmResult` if it exists
- Update the warm-up to only use generic `fetch_and_project` for all content types

**Step 2: Remove path projection collection**

Delete or gut `doorway/doorway-service/src/projection/collections/paths.rs`. If the collections module has a `mod paths;` declaration, remove it.

**Step 3: Search for remaining "path" references in doorway**

```bash
cd doorway/doorway-service
grep -rn "LearningPath\|path_count\|cache\.path\|PathProjection\|paths_diesel" src/
```

Remove all hits. Paths are just content now — doorway projects them generically.

**Step 4: Build and test**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo build --release 2>&1 | tail -5
RUSTFLAGS="" cargo clippy -- -D warnings 2>&1 | tail -5
RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -5
```

**Step 5: Commit**

```bash
git add doorway/doorway-service/
git commit -m "refactor(doorway): remove all path-specific code

Doorway is now fully type-agnostic. Paths are projected as generic
content. fetch_paths_individually hack eliminated."
```

---

### Task 8: Angular — LearningPath becomes view parser over ContentNode

The frontend shifts from a separate LearningPath model to a parsed view over ContentNode.

**Files:**
- Modify: `app/elohim-app/src/app/lamad/models/learning-path.model.ts` — replace with PathView types derived from ContentNode
- Modify: `app/elohim-app/src/app/lamad/services/path.service.ts` — delegate to ContentService, parse composite body
- Modify: `app/elohim-app/src/app/elohim/services/data-loader.service.ts` — remove separate getPath/getPathIndex methods (or delegate to getContent)
- Modify: `app/elohim-app/src/app/lamad/components/path-overview/` — render from parsed ContentNode
- Modify: `app/elohim-app/src/app/elohim/services/storage-client.service.ts` — remove path-specific HTTP calls
- Possibly modify: `app/elohim-app/src/app/elohim/services/content-resolver.service.ts`
- Modify: `elohim/sdk/storage-client-ts/src/` — remove path-specific client methods if they exist

**Step 1: Define PathView types derived from ContentNode**

Replace the contents of `learning-path.model.ts` with view types that parse a ContentNode:

```typescript
import { ContentNode } from './content-node.model';

/** Parsed view of a ContentNode with contentType 'path' */
export interface PathView {
  node: ContentNode;
  pathType: string;
  difficulty?: string;
  estimatedDuration?: string;
  thumbnailUrl?: string;
  sections: PathSection[];
}

export interface PathSection {
  id: string;
  title: string;
  description?: string;
  level: string;
  sections?: PathSection[];
  items?: PathItem[];
}

export interface PathItem {
  ref: string;         // EPR reference
  role: string;        // 'step' | 'checkpoint' | 'reflection'
  title?: string;
  narrative?: string;
  learningObjectives?: string[];
  completionCriteria?: { type: string; threshold?: number };
}

/** Parse a ContentNode into a PathView */
export function parsePathView(node: ContentNode): PathView {
  const body = typeof node.content === 'string'
    ? JSON.parse(node.content)
    : node.content;
  const meta = node.metadata ?? {};

  return {
    node,
    pathType: meta.pathType ?? 'journey',
    difficulty: meta.difficulty,
    estimatedDuration: meta.estimatedDuration,
    thumbnailUrl: meta.thumbnailUrl,
    sections: body?.sections ?? [],
  };
}
```

**Step 2: Update PathService to delegate to ContentService**

In `path.service.ts`, replace direct path API calls with content API calls + parsing:

```typescript
getPath(pathId: string): Observable<PathView> {
  return this.contentService.getContent(pathId).pipe(
    map(node => parsePathView(node))
  );
}

listPaths(): Observable<PathView[]> {
  return this.contentService.getContentByType('path').pipe(
    map(nodes => nodes.map(parsePathView))
  );
}
```

Keep all the fog-of-war, mastery, and access control logic — just change the data source.

**Step 3: Update DataLoaderService**

In `data-loader.service.ts`:
- `getPath(pathId)` → delegates to `getContent(pathId)` then parses
- `getPathIndex()` → delegates to `getContentIndex()` filtered by contentType 'path'
- Or simply remove these methods and have callers use `getContent()` directly

**Step 4: Update PathOverviewComponent**

The component currently expects a `LearningPath`. Update it to work with `PathView`:
- Change the type annotations
- Walk `pathView.sections` instead of `path.chapters`
- Nested sections render recursively for the scope-and-sequence hierarchy
- EPR references in items resolve via ContentService

**Step 5: Update storage-client to remove path methods**

In `elohim/sdk/storage-client-ts/src/`:
- Remove any `getPath()`, `listPaths()`, `createPath()` methods
- Remove imports of deleted generated types (PathView, PathWithDetailsView, etc.)

**Step 6: Build and test**

```bash
cd app/elohim-app
pnpm run lint
pnpm test
pnpm run build
```

Fix any TypeScript compilation errors from removed types. Components referencing `LearningPath` must switch to `PathView`.

**Step 7: Commit**

```bash
git add app/elohim-app/ app/elohim-library/ elohim/sdk/storage-client-ts/
git commit -m "refactor(angular): LearningPath becomes PathView over ContentNode

PathService delegates to ContentService. PathOverviewComponent renders
from parsed composite body. Separate path API calls eliminated."
```

---

### Task 9: DNA — Deprecate LearningPath, PathChapter, PathStep

Stop creating these entry types. Keep validation for existing DHT entries.

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` — add deprecation comments, keep validation
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_coordinator/src/lib.rs` (or equivalent coordinator) — remove or disable create functions for these types
- Modify: DNA constants for contentType/contentFormat enums — add `"path"` and `"epr-composite"`

**Step 1: Add "path" and "epr-composite" to DNA constants**

In the integrity zome's constants section, find the content type and content format arrays. Add:
- `"path"` to valid content types
- `"epr-composite"` to valid content formats

**Step 2: Mark entry types as deprecated in integrity zome**

In `content_store_integrity/src/lib.rs`, add comments to `LearningPath`, `PathChapter`, `PathStep`:

```rust
/// DEPRECATED: Paths are now Content entries with content_type "path".
/// Validation retained for existing DHT entries. Do not create new entries.
#[hdk_entry_helper]
pub struct LearningPath { ... }
```

Keep all validation logic intact — existing entries on the DHT must still validate.

**Step 3: Disable creation in coordinator zome**

In the coordinator zome, find the `create_learning_path`, `create_path_chapter`, `create_path_step` functions. Either:
- Return an error: `return Err(wasm_error!("LearningPath is deprecated. Use Content with content_type 'path'"));`
- Or remove them entirely if no existing callers depend on them

**Step 4: Build DNA**

```bash
cd elohim/holochain/dna
# Follow the DNA build process — check dna/CLAUDE.md for build commands
```

**Step 5: Run schema:check-dna to verify alignment**

```bash
pnpm run schema:check-dna
```

Should pass now that DNA constants include "path" and "epr-composite".

**Step 6: Commit**

```bash
git add elohim/holochain/dna/
git commit -m "refactor(dna): deprecate LearningPath, PathChapter, PathStep entry types

Paths are now Content entries with content_type 'path'. Existing DHT
entries retain validation. Coordinator functions disabled. DNA constants
updated with 'path' and 'epr-composite' values."
```

---

### Task 10: Final verification and cleanup

**Step 1: Full stack test**

Start the full stack and verify end-to-end:
```bash
cd app/elohim-app
pnpm run hc:start:seed
```

- Navigate to a path in the UI — should render from ContentNode data
- Content list should include paths when filtered by `contentType=path`
- Path overview should show scope-and-sequence hierarchy
- Individual step content should load via EPR reference resolution

**Step 2: Search for remaining path table references**

```bash
# Across entire repo
grep -rn "paths_diesel\|PathView\|PathWithDetails\|/db/paths\|bulkCreatePaths\|fetch_paths_individually\|cache\.path" --include="*.rs" --include="*.ts" --include="*.json" .
```

Remove any remaining references.

**Step 3: Run all quality gates**

```bash
# Storage
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test
cargo fmt --check

# Doorway
cd doorway/doorway-service
RUSTFLAGS="" cargo clippy -- -D warnings
RUSTFLAGS="" cargo test --lib --bins
cargo fmt --check

# Angular
cd app/elohim-app
pnpm run lint
pnpm test
pnpm run build

# Schema
pnpm run schema:test
pnpm run schema:validate
```

**Step 4: Final commit if any cleanup needed**

```bash
git commit -m "chore: final cleanup — path-content unification complete"
```
