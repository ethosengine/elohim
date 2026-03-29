# Sprint E: Typed Pipeline Cleanup — Eliminate Parallel Untyped Paths

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Every content access path uses the typed pipeline built in Sprints 1-5. No duplicate `transformContent`, no `metadata['key']` string access, no hand-written metadata interfaces that duplicate generated types.

**Architecture:** Sprints 1-5 built a typed pipeline: schema → codegen → generated types → `ContentService.transformContent()` → discriminated union → type guards → typed metadata. But a parallel untyped path survived in `projection-api.service.ts`, and 15 call sites still reach into metadata with string keys. This sprint eliminates those remnants.

**Tech Stack:** Angular, TypeScript, generated types from `sdk/domains/`

**Parent design:** `genesis/plans/2026-03-29-domain-manifests-sdk-boundary-design.md`

---

## Context

The typed pipeline works end-to-end for content flowing through `ContentService`:
```
Storage API → ContentService.transformContent() → TypedContentNode → type guards → typed metadata
```

But `ProjectionApiService` has its own `transformContent()` (line 529) that builds `ContentNode` from raw `Record<string, unknown>`. Components that use the projection API bypass the typed pipeline entirely. Additionally, 15 call sites access `metadata['key']` with string keys, and avodah has hand-written interfaces that duplicate generated types.

## Tasks

### Task 1: Audit projection-api.service.ts

**Files:**
- Read: `app/elohim-app/src/app/elohim/services/projection-api.service.ts`

**Step 1:** Read the file and understand:
- What does `ProjectionApiService.transformContent()` do that `ContentService.transformContent()` doesn't?
- Who calls `ProjectionApiService` directly instead of going through `ContentService`/`DataLoaderService`?
- Does the projection API return a different shape than the storage API?

**Step 2:** Map all callers of `ProjectionApiService.transformContent()`:
```bash
grep -rn "projectionApi\|ProjectionApiService" app/elohim-app/src/app/ --include="*.ts" | grep -v ".spec." | grep -v "generated/"
```

**Step 3:** Document findings — which callers can be rerouted to `ContentService`, which need the projection-specific behavior.

### Task 2: Route projection content through ContentService

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/projection-api.service.ts`
- Modify: Callers identified in Task 1

**Step 1:** If `ProjectionApiService.transformContent()` does the same job as `ContentService.transformContent()` (just with untyped access), replace it:

Option A (preferred): Have `ProjectionApiService` delegate content transforms to `ContentService`:
```typescript
// In ProjectionApiService
private transformContent(data: Record<string, unknown>): ContentNode {
  return this.contentService.transformRawContent(data);
}
```

Option B: If there's projection-specific behavior (e.g., different field names from MongoDB vs SQLite), create a normalizer (like the conductor normalizer from Sprint 5) that produces `ContentView` shape first.

**Step 2:** Delete the duplicate `resolveBlobUrl` from `ProjectionApiService` — use the one in `ContentService`.

**Step 3:** Verify callers still work — the return type should be identical.

**Step 4:** Run `pnpm exec ng build --configuration=development` to verify no type errors.

**Step 5:** Commit.

### Task 3: Replace avodah hand-written Meta interfaces

**Files:**
- Modify: `app/elohim-app/src/app/avodah/models/work-story.model.ts`
- Modify: `app/elohim-app/src/app/avodah/models/work-project.model.ts`

**Step 1:** In `work-story.model.ts`, replace the hand-written `WorkStoryMeta` interface with an import from generated types:

```typescript
// Before:
export interface WorkStoryMeta { ... }

// After:
export type { WorkStoryMeta } from '../generated/metadata-types';
```

If the hand-written interface has fields the generated one doesn't, add them to the schema (`sdk/domains/avodah/schemas/work-story-metadata.schema.json`) and regenerate.

**Step 2:** Same for `WorkProjectMeta` in `work-project.model.ts`.

**Step 3:** Run `pnpm run avodah:codegen` then build to verify.

**Step 4:** Commit.

### Task 4: Fix metadata string key access in lamad components

**Files:**
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts`
- Modify: `app/elohim-app/src/app/lamad/quiz-engine/models/quiz-session.model.ts`
- Modify: `app/elohim-app/src/app/lamad/renderers/iframe-renderer/iframe-renderer.component.ts`
- Modify: `app/elohim-app/src/app/lamad/content-io/plugins/sophia/sophia-format.plugin.ts`

**Step 1:** In `content-viewer.component.ts`, replace:
```typescript
// Before:
return this.node.metadata['category'];
const authors = this.node.metadata['authors'];
return this.node.metadata['version'];

// After — use type guard or typed access:
import type { ConceptMetadata } from '@app/lamad/generated/metadata-types';
const meta = this.node.metadata as ConceptMetadata;
return meta.category;
const authors = meta.authors;
return meta.version;
```

Note: `category`, `authors`, `version` may need to be added to `ConceptMetadata` in the schema if not already present. Check `sdk/domains/lamad/schemas/concept-metadata.schema.json`.

**Step 2:** In `quiz-session.model.ts`, replace:
```typescript
// Before:
question.item.metadata['bloomsLevel'] as string

// After:
import { isConceptNode } from '@app/lamad/generated/content-node-types';
// or cast metadata to AssessmentMetadata/ConceptMetadata as appropriate
```

**Step 3:** In `iframe-renderer.component.ts`, replace:
```typescript
// Before:
const url = (metadata['embedUrl'] ?? metadata['url']) as string;

// After — these fields should be in a format-specific metadata type or ConceptMetadata
```

**Step 4:** In `sophia-format.plugin.ts`, replace:
```typescript
// Before:
if (typeof metadata['title'] === 'string') { return metadata['title']; }

// After — title is on ContentNode directly, not metadata
```

**Step 5:** Build and verify.

**Step 6:** Commit.

### Task 5: Fix metadata string key access in elohim core services

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/data-loader.service.ts`
- Modify: `app/elohim-app/src/app/elohim/services/content.service.ts`
- Modify: `app/elohim-app/src/app/elohim/services/sla-monitor.service.ts`

**Step 1:** In `data-loader.service.ts` (line 1451):
```typescript
// Before:
metadata['confidence'] = hcRel.confidence;

// After — this is relationship metadata, not content metadata
// May need a RelationshipMetadata type or just use Record<string, unknown> with a comment
```

**Step 2:** In `content.service.ts` (line 677) — this was our fix from earlier sessions:
```typescript
data.thumbnailUrl ?? (metadata['thumbnailUrl'] as string | undefined)
```
This is in `transformContent()` itself — the entry point to the typed pipeline. It should use the metadata type to access thumbnailUrl. But since `transformContent` handles ALL content types before narrowing, it needs to stay generic here. Add a comment explaining why this access is intentionally untyped (pre-narrowing).

**Step 3:** In `sla-monitor.service.ts` (line 243):
```typescript
// Before:
(sla.metadata['escalationHistory'] as EscalationRecord[]) || []

// After — this is governance metadata. If qahal's manifest defines it, import from qahal generated types
```

**Step 4:** Build and verify.

**Step 5:** Commit.

### Task 6: Fix metadata string key access in shefa

**Files:**
- Modify: `app/elohim-app/src/app/shefa/services/insurance-mutual.service.ts`

**Step 1:** Lines 855-859 access `claim.metadata['coverageLimit']`. This should use the generated `AgreementMetadata` or a shefa-specific type from `@app/shefa/generated/metadata-types`.

**Step 2:** Check if `coverageLimit` is in `sdk/domains/shefa/schemas/agreement-metadata.schema.json`. If not, add it and regenerate.

**Step 3:** Build and verify.

**Step 4:** Commit.

### Task 7: Audit remaining Record<string, unknown> casts

**Step 1:** Run the full count:
```bash
grep -rn "as Record<string, unknown>" app/elohim-app/src/app/ --include="*.ts" | grep -v "generated/" | grep -v ".spec." | grep -v "node_modules" | wc -l
```

**Step 2:** Categorize each:
- **Generated code** — leave (codegen will fix in future)
- **Pre-narrowing** (in transformContent before type guards) — leave with comment
- **Should use type guard** — fix
- **Non-content metadata** (relationship metadata, governance metadata) — import from appropriate domain generated types

**Step 3:** Fix the "should use type guard" category. Leave the rest with `// Intentionally untyped: ...` comments explaining why.

**Step 4:** Report final count. Goal: reduce 77 → <20 (eliminating all content metadata casts, keeping only structural/pre-narrowing casts).

**Step 5:** Commit.

### Task 8: Final verification

```bash
# Build
cd app/elohim-app && pnpm exec ng build --configuration=development

# No metadata['key'] in lamad pillar (should be fully typed)
grep -rn "metadata\['" app/elohim-app/src/app/lamad/ --include="*.ts" | grep -v "generated/" | grep -v ".spec." | wc -l
# Target: 0

# Reduced Record<string, unknown> casts
grep -rn "as Record<string, unknown>" app/elohim-app/src/app/ --include="*.ts" | grep -v "generated/" | grep -v ".spec." | wc -l
# Target: <20

# No duplicate transformContent
grep -rn "private transformContent" app/elohim-app/src/app/elohim/services/ --include="*.ts" | wc -l
# Target: 1 (only in content.service.ts)

# Seeder still clean
cd genesis/seeder && npx tsc --noEmit && npx vitest run

# Schema tests
pnpm run schema:test
```

## Exit Criteria

1. Single `transformContent` — only in `ContentService`, nowhere else
2. Zero `metadata['key']` string access in lamad pillar
3. Avodah models import from generated types, no hand-written duplicates
4. `Record<string, unknown>` casts reduced from 77 to <20, remaining ones commented
5. App builds, seeder compiles, all tests pass
