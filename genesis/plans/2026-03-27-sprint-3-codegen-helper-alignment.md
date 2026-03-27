# Sprint 3: Codegen Helper + Seeder/App Alignment

**Parent design:** `2026-03-27-typed-content-pipeline-design.md`
**Depends on:** Sprint 2 (lamad domain types)
**Goal:** Single codegen command produces identical types for seeder and Angular. Hand-written content interfaces eliminated. The seeder seeds what Angular renders — no guesswork.

> **P2P note:** No new storage entities. This sprint is pure tooling — codegen reads existing schemas and produces TypeScript. All storage shapes are Category A (content, events, attestations) already defined in sprints 1-2.

## Tasks

### 1. Consolidate codegen into single entry point

**File:** `app/lamad/scripts/codegen.mjs`

Reads:
- Protocol schemas (`elohim/sdk/schemas/v1/`) — wire types, enums
- Lamad manifest (`app/lamad/manifest.json`) — vocabulary, coupling
- Lamad schemas (`app/lamad/schemas/`) — metadata, body shapes

Produces identical output to TWO locations:
- `app/lamad/generated/` — consumed by Angular (via `@app/lamad/generated`)
- `genesis/seeder/src/generated/` — consumed by seeder

Output files:
```
generated/
  schema-enums.ts          # (existing) protocol enums
  create-content-input.ts  # (existing) protocol wire input
  content-view.ts          # (existing) protocol wire output
  create-economic-event-input.ts  # (sprint 1) REA input
  economic-event-view.ts          # (sprint 1) REA view
  metadata-types.ts        # (sprint 2) PathMetadata, ConceptMetadata, etc.
  body-types.ts            # (sprint 2) EprCompositeBody, Section, Item
  content-node-types.ts    # (sprint 2) discriminated union + type guards
  manifest-constants.ts    # content type lists, renderer map, signal map
```

**Command:** `pnpm run lamad:codegen`

### 2. Replace ContentNode interface in Angular

**Current:** `app/elohim-app/src/app/lamad/models/content-node.model.ts` (410 lines, 30+ unpopulated fields)

**Replace with:**
- Import `ContentView` from `@app/generated/content-view` (protocol wire type)
- Import `TypedContentNode` from `@app/lamad/generated/content-node-types` (discriminated union)
- Keep only app-layer extensions (computed/derived fields not from storage):
  - `blobs?: ContentBlob[]` (blob attachment system)
  - `stewardedBy?: ContentSteward[]` (affinity decorations)

```typescript
// content-node.model.ts — thin app extension, NOT a type definition
import type { TypedContentNode } from '@app/lamad/generated/content-node-types';
import type { ContentBlob, ContentSteward } from './content-extensions.model';

export type ContentNode = TypedContentNode & {
  // App-layer computed fields (not from storage)
  blobs?: ContentBlob[];
  stewardedBy?: ContentSteward[];
};

// Re-export for convenience
export type { PathMetadata, ConceptMetadata, AssessmentMetadata } from '@app/lamad/generated/metadata-types';
export { isPathNode, isConceptNode, isAssessmentNode } from '@app/lamad/generated/content-node-types';
```

### 3. Replace parsePathView hand-written types

**File:** `app/elohim-app/src/app/lamad/models/learning-path.model.ts`

- Delete `interface RawSection` and `interface RawItem`
- Import `EprCompositeBody`, `Section`, `Item` from generated types
- `parsePathView()` parses body as `EprCompositeBody`:

```typescript
import type { EprCompositeBody, Section, Item } from '@app/lamad/generated/body-types';

export function parsePathView(node: ContentNode): PathView {
  let body: EprCompositeBody;
  if (typeof node.content === 'string') {
    body = JSON.parse(node.content) as EprCompositeBody;
  } else {
    body = (node.content as EprCompositeBody) ?? { sections: [] };
  }
  // sections are now Section[], items are Item[] — fully typed
}
```

- `enrichSection()` accepts `Section` — typed items, typed nested sections
- `collectSteps()` produces typed steps from `Item[]`
- `sectionsToChapters()` preserves typed `Section.conceptIds`

### 4. Replace ContentService.transformContent() output

**File:** `app/elohim-app/src/app/elohim/services/content.service.ts`

- `transformContent()` returns `TypedContentNode` (discriminated union) instead of `ContentNode` via `as` cast
- `thumbnailUrl` extracted from typed metadata:

```typescript
private transformContent(data: RawContentData): TypedContentNode {
  const metadata = data.metadata ?? {};
  return {
    id: data.id,
    title: data.title ?? '',
    contentType: data.contentType,
    contentFormat: data.contentFormat ?? 'markdown',
    contentBody: data.contentBody ?? '',
    metadata,
    reach: data.reach ?? 'commons',
    // thumbnailUrl from metadata (typed — no string key guessing)
    thumbnailUrl: this.resolveBlobUrl(
      (metadata as Record<string, unknown>)['thumbnailUrl'] as string
    ),
    // ... remaining fields
  };
}
```

Note: during this sprint, metadata is still `Record<string, unknown>` at the wire level. The discriminated union narrows it by contentType. Full type narrowing happens when consumers use `isPathNode(node)`.

### 5. Replace seeder transform types

**File:** `genesis/seeder/src/seed-sqlite.ts`

- `transformContent()` returns `TypedContentNode` (or `CreateContentInput` with typed metadata)
- Metadata constructed as `ConceptMetadata` — typed, not `Record<string, unknown>`:

```typescript
import type { ConceptMetadata } from './generated/metadata-types';

const metadata: ConceptMetadata = {};
if (json.estimatedMinutes) metadata.estimatedMinutes = json.estimatedMinutes;
if (json.thumbnailUrl) metadata.thumbnailUrl = json.thumbnailUrl;
// tsc catches: metadata.nonExistentField = 'oops' ← compile error
```

- `transformPathToContent()` builds `PathMetadata` and `EprCompositeBody`:

```typescript
import type { PathMetadata } from './generated/metadata-types';
import type { EprCompositeBody, Section, Item } from './generated/body-types';

const metadata: PathMetadata = {
  pathType: json.pathType,
  difficulty: json.difficulty || 'beginner',
  // tsc catches type mismatches
};

const body: EprCompositeBody = {
  sections: chaptersToSections(json), // returns Section[]
};
```

### 6. Align seeder sections output with Angular parse expectations

The epr-composite body schema (sprint 2) is now the contract between seeder and parsePathView. Both read/write the same `Section` and `Item` types. The bugs we fixed today (wrong nesting depth, missing conceptIds, broken chapter counting) become impossible because:

- Seeder constructs `Section[]` with `Item[]` — typed
- Angular parses `Section[]` with `Item[]` — same types
- `sectionsToChapters()` receives typed sections — `section.items`, `section.sections`, `section.conceptIds` all known at compile time

### 7. Update constants-sync test

**File:** `genesis/seeder/src/__tests__/constants-sync.test.ts`

Add tests verifying ALL generated files match between seeder and app:
- `metadata-types.ts` identical in both locations
- `body-types.ts` identical in both locations
- `content-node-types.ts` identical in both locations
- `manifest-constants.ts` identical in both locations

### 8. Delete dead type definitions

Remove after all imports updated:
- `content-node.model.ts` 400+ lines of interface → thin re-export wrapper
- `RawSection`, `RawItem` from `learning-path.model.ts`
- Hand-written `CreateContentInput` from `validators.ts` (already done in earlier commit, verify clean)
- Hand-written `Content` interface from `seed-entities.ts` (already uses `metadata` object)

## Verification

```bash
# Single codegen command
pnpm run lamad:codegen

# Generated files identical in both locations
diff app/lamad/generated/metadata-types.ts genesis/seeder/src/generated/metadata-types.ts
diff app/lamad/generated/body-types.ts genesis/seeder/src/generated/body-types.ts

# Zero hand-written content interfaces
grep -r "interface RawSection\|interface RawItem\|interface ContentNode {" app/elohim-app/src/app/lamad/models/ | wc -l  # 0

# Zero untyped metadata access in lamad models
grep -r "as Record<string, unknown>" app/elohim-app/src/app/lamad/models/ | wc -l  # 0

# App builds
cd app/elohim-app && pnpm run build

# Seeder compiles
cd genesis/seeder && npx tsc --noEmit && npx vitest run

# Full constants sync
cd genesis/seeder && npx vitest run constants-sync

# Landing page: thumbnails load (thumbnailUrl from typed PathMetadata, resolved by ContentService)
# Path overview: chapters show correct concept counts (typed Section.items → conceptIds)
# Start Chapter: navigates (typed getChapterConceptCount reads typed Section)
```
