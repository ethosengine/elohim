# Schema IoC Enforcement — Compile-Time Enum Safety

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the protocol JSON schema the single enforceable source of truth for enum values across all TypeScript, eliminating the class of bug where invalid string literals reach Rust validation at runtime.

**Architecture:** The codegen pipeline (`schema:codegen:ts`) already generates `schema-enums.ts` from protocol JSON schemas to two locations (seeder + app). We extend it to elohim-library, then wire the generated union types into every interface that crosses the HTTP boundary. `tsc --noEmit` in CI enforces at compile time; constants-sync tests enforce at test time.

**Tech Stack:** TypeScript strict mode, protocol JSON schemas, codegen-ts.mjs, Vitest

---

## Phase A: Infrastructure — Make the Schema Contract Reach Everywhere

### Task 1: Add `node-context` and `stewardship-context` to Content Type Schema

These are legitimate content types used by `seed-nodes.ts`. The schema must reflect reality.

**Files:**
- Modify: `elohim/sdk/schemas/v1/enums/content-type.schema.json`

**Step 1: Add values to schema**

Add `node-context` and `stewardship-context` to the `enum` array and the `extensible` tier:

```json
"enum": [
    "epic", "concept", "lesson", "scenario", "assessment",
    "reflection", "discussion", "exercise", "article", "path",
    "human", "role", "collective",
    "example", "reference",
    "feature", "practice", "contributor",
    "video", "audio", "book", "book-chapter", "documentary",
    "bible-verse", "activity", "narrative", "course-module",
    "module", "quiz", "podcast", "simulation",
    "node-context", "stewardship-context"
],
```

And in `_tiers.extensible.values`, append `"node-context"`, `"stewardship-context"`.

**Step 2: Regenerate all codegen targets**

Run: `pnpm run schema:codegen:ts`

**Step 3: Verify generated files updated**

Check that `genesis/seeder/src/generated/schema-enums.ts` and `app/elohim-app/src/app/generated/schema-enums.ts` both contain the new values in `ALL_CONTENT_TYPES`.

**Step 4: Regenerate Rust enums**

Run: `pnpm run schema:codegen:rs` (if this script exists; otherwise check how `generated_enums.rs` is regenerated — may be `pnpm run schema:check-dna` or manual)

**Step 5: Run schema tests**

Run: `pnpm run schema:test`
Expected: All 24+ assertions pass.

**Step 6: Run constants-sync tests**

Run: `cd genesis/seeder && npx vitest run src/__tests__/constants-sync.test.ts`
Expected: All 9 tests pass.

**Step 7: Commit**

```bash
git add elohim/sdk/schemas/v1/enums/content-type.schema.json \
        genesis/seeder/src/generated/schema-enums.ts \
        app/elohim-app/src/app/generated/schema-enums.ts
git commit -m "feat(schema): add node-context and stewardship-context to content type enum"
```

---

### Task 2: Extend Codegen to elohim-library

The import CLI in `app/elohim-library/projects/elohim-service/` has hand-maintained `ContentType`, `ContentFormat`, and `ContentReach` that have drifted from the schema. Fix by generating `schema-enums.ts` there too.

**Files:**
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs` (line 25-28)
- Create: `app/elohim-library/projects/elohim-service/src/generated/schema-enums.ts` (auto-generated)

**Step 1: Add third output path**

In `codegen-ts.mjs`, line 25-28:

```javascript
const ENUM_OUTPUT_PATHS = [
  resolve(REPO_ROOT, 'genesis/seeder/src/generated/schema-enums.ts'),
  resolve(REPO_ROOT, 'app/elohim-app/src/app/generated/schema-enums.ts'),
  resolve(REPO_ROOT, 'app/elohim-library/projects/elohim-service/src/generated/schema-enums.ts'),
];
```

**Step 2: Create the generated directory**

```bash
mkdir -p app/elohim-library/projects/elohim-service/src/generated
```

**Step 3: Run codegen**

Run: `pnpm run schema:codegen:ts`

**Step 4: Verify the file was created**

Run: `diff genesis/seeder/src/generated/schema-enums.ts app/elohim-library/projects/elohim-service/src/generated/schema-enums.ts`
Expected: No differences.

**Step 5: Commit**

```bash
git add elohim/sdk/schemas/scripts/codegen-ts.mjs \
        app/elohim-library/projects/elohim-service/src/generated/schema-enums.ts
git commit -m "feat(schema): extend codegen to generate schema-enums for elohim-library"
```

---

### Task 3: Replace Hand-Maintained Types in elohim-library

**Files:**
- Modify: `app/elohim-library/projects/elohim-service/src/models/content-node.model.ts`

**Step 1: Replace local type definitions with imports**

Remove the hand-maintained `ContentType`, `ContentFormat`, and `ContentReach` type aliases (lines 13-46) and replace with imports from the generated file:

```typescript
import type {
  ContentType,
  ContentFormat,
  Reach as ContentReach,
} from '../generated/schema-enums.js';

export type { ContentType, ContentFormat, ContentReach };
```

Keep the `ContentReach` alias name for backward compatibility with existing consumers.

**Step 2: Check for other hand-maintained types in elohim-library**

Search for `ReachLevel` in `app/elohim-library/projects/elohim-service/src/services/trust.service.ts` — it defines its own `ReachLevel` type. Update to import from generated:

```typescript
import type { Reach as ReachLevel } from '../generated/schema-enums.js';
```

**Step 3: Run elohim-library tests**

Run: `cd app/elohim-library/projects/elohim-service && pnpm test`
Expected: All tests pass.

**Step 4: Commit**

```bash
git add app/elohim-library/projects/elohim-service/src/models/content-node.model.ts \
        app/elohim-library/projects/elohim-service/src/services/trust.service.ts
git commit -m "refactor(library): replace hand-maintained enum types with schema-generated imports"
```

---

### Task 4: Add constants-sync coverage for elohim-library

Extend the existing constants-sync test to also verify that the elohim-library generated file matches.

**Files:**
- Modify: `genesis/seeder/src/__tests__/constants-sync.test.ts`

**Step 1: Add library generated path constant**

Near lines 35-44, add:

```typescript
const LIBRARY_GENERATED = path.join(
  REPO_ROOT,
  'app',
  'elohim-library',
  'projects',
  'elohim-service',
  'src',
  'generated',
  'schema-enums.ts',
);
```

**Step 2: Add test**

In the `Constants Sync: seeder generated ↔ app generated` describe block, add:

```typescript
it('library-side schema-enums.ts should match seeder-side schema-enums.ts', () => {
  if (!fs.existsSync(LIBRARY_GENERATED)) {
    throw new Error(
      `Library-side generated file not found at ${LIBRARY_GENERATED}. ` +
        'Run: pnpm run schema:codegen:ts',
    );
  }

  const libraryContent = fs.readFileSync(LIBRARY_GENERATED, 'utf8');
  const seederContent = fs.readFileSync(SEEDER_GENERATED, 'utf8');

  expect(libraryContent).toEqual(seederContent);
});
```

**Step 3: Run tests**

Run: `cd genesis/seeder && npx vitest run src/__tests__/constants-sync.test.ts`
Expected: All 10 tests pass.

**Step 4: Commit**

```bash
git add genesis/seeder/src/__tests__/constants-sync.test.ts
git commit -m "test(schema): add constants-sync coverage for elohim-library generated enums"
```

---

## Phase B: Seeder — Type Every Wire-Crossing Interface

### Task 5: Type `seed.ts` CreateContentInput

The DHT seeder's `CreateContentInput` (line 526) uses `string` for all enum fields.

**Files:**
- Modify: `genesis/seeder/src/seed.ts` (lines 23-25 imports, lines 526-547 interface)

**Step 1: Add type imports**

Near the top imports:

```typescript
import type { ContentFormat, ContentType, Reach } from './generated/schema-enums.js';
```

**Step 2: Update interface**

Change lines 526-547:

```typescript
interface CreateContentInput {
  id: string;
  contentType: ContentType;    // was: string
  title: string;
  description: string;
  summary: string | null;
  content: string;
  contentFormat: ContentFormat; // was: string
  tags: string[];
  sourcePath: string | null;
  relatedNodeIds: string[];
  reach: Reach;                // was: string
  estimatedMinutes: number | null;
  thumbnailUrl: string | null;
  metadataJson: string;
  blobCid: string | null;
  contentSizeBytes: number | null;
  contentHash: string | null;
  blobHash?: string;
}
```

**Step 3: Fix compilation errors**

Search `seed.ts` for assignments to these fields. Each dynamic value from JSON needs `as ContentType`, `as ContentFormat`, or `as Reach` at the trust boundary. Hardcoded literals will be type-checked automatically.

Key locations to check:
- `conceptToInput()` function (~line 760) — assigns contentType, contentFormat, reach from JSON
- Any hardcoded `reach:`, `contentFormat:`, `contentType:` literals

**Step 4: Run typecheck**

Run: `cd genesis/seeder && npx tsc --noEmit 2>&1 | grep seed.ts`
Expected: No errors in seed.ts.

**Step 5: Commit**

```bash
git add genesis/seeder/src/seed.ts
git commit -m "refactor(seeder): type seed.ts CreateContentInput with schema-generated enums"
```

---

### Task 6: Type `seed-nodes.ts` Wire-Crossing Types

**Files:**
- Modify: `genesis/seeder/src/seed-nodes.ts`

**Step 1: Add type imports**

```typescript
import type { ContentFormat, ContentType, Reach } from './generated/schema-enums.js';
```

**Step 2: Type the `createContextContent` function parameter**

Line 69: Change `contentType: string` to `contentType: ContentType`.

**Step 3: Type the inline object literal**

The anonymous object at lines 78-86 isn't type-constrained because it's inside `JSON.stringify()`. Add a type annotation:

```typescript
const payload: Array<{
  id: string;
  title: string;
  contentType: ContentType;
  contentFormat: ContentFormat;
  contentBody: string;
  reach: Reach;
}> = [{
  id,
  title,
  contentType,
  contentFormat: 'text',
  contentBody: body,
  reach: 'intimate',
}];

body: JSON.stringify(payload),
```

**Step 4: Run typecheck**

Run: `cd genesis/seeder && npx tsc --noEmit 2>&1 | grep seed-nodes`
Expected: No errors.

**Step 5: Run constants-sync tests**

Run: `cd genesis/seeder && npx vitest run src/__tests__/constants-sync.test.ts`
Expected: All pass (including the TS-code-scanning test, since `'node-context'` and `'stewardship-context'` were added to the schema in Task 1).

**Step 6: Commit**

```bash
git add genesis/seeder/src/seed-nodes.ts
git commit -m "refactor(seeder): type seed-nodes.ts with schema-generated enums"
```

---

### Task 7: Type `doorway-client.ts` Bulk Operation Types

**Files:**
- Modify: `genesis/seeder/src/doorway-client.ts`

**Step 1: Add type imports**

```typescript
import type { ContentFormat, ContentType, Reach, MasteryLevel } from './generated/schema-enums.js';
```

**Step 2: Type the inline array item types**

Find the `bulkCreateContent` method (~line 794). Change the inline item type's `contentType`, `contentFormat`, and `reach` fields from `string` to the generated types.

Find the `bulkUpsertMastery` method (~line 969). Change `masteryLevel` from `string` to `MasteryLevel`.

**Step 3: Fix callers that pass `string` values**

Add `as ContentType` etc. at trust boundaries where dynamic values enter.

**Step 4: Run typecheck**

Run: `cd genesis/seeder && npx tsc --noEmit 2>&1 | grep doorway-client`
Expected: No errors.

**Step 5: Commit**

```bash
git add genesis/seeder/src/doorway-client.ts
git commit -m "refactor(seeder): type doorway-client.ts bulk operations with schema-generated enums"
```

---

### Task 8: Clean Up `validators.ts` `as any` Casts

With the generated types now in the ecosystem, the 7 `as any` casts in validators.ts can be cleaned up. Note: validators.ts keeps `string` fields in its interface because it validates UNTRUSTED input. But the `.includes()` calls don't need `as any`.

**Files:**
- Modify: `genesis/seeder/src/validators.ts`

**Step 1: Replace `as any` casts with proper narrowing**

Each `.includes(content.contentType as any)` can use a type-safe includes helper:

```typescript
function isValidEnum<T extends string>(value: string, values: readonly T[]): value is T {
  return (values as readonly string[]).includes(value);
}
```

Then replace:
```typescript
// Before:
if (content.contentType && !CONTENT_TYPES.includes(content.contentType as any)) {
// After:
if (content.contentType && !isValidEnum(content.contentType, CONTENT_TYPES)) {
```

Apply to all 7 locations (lines 123, 129, 135, 178, 214, 250, 264).

**Step 2: Run tests**

Run: `cd genesis/seeder && npx vitest run`
Expected: All 53 tests pass.

**Step 3: Run typecheck**

Run: `cd genesis/seeder && npx tsc --noEmit`
Expected: No errors.

**Step 4: Commit**

```bash
git add genesis/seeder/src/validators.ts
git commit -m "refactor(seeder): replace as-any casts in validators with type-safe isValidEnum helper"
```

---

## Phase C: Angular App — Type the Frontend Boundary

### Task 9: Type `StorageClientService` Interfaces

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/storage-client.service.ts`

**Step 1: Add import**

```typescript
import type { ContentType, ContentFormat, Reach } from '@app/generated/schema-enums';
```

(The app already has the generated file at `src/app/generated/schema-enums.ts` and a `@app/generated` path alias should work — verify in `tsconfig.json`.)

**Step 2: Type StorageContentNode**

```typescript
export interface StorageContentNode {
  id: string;
  contentType: ContentType;      // was: string
  title: string;
  description: string;
  contentBody: string | null;
  contentFormat: ContentFormat;  // was: string
  // ... rest unchanged
  reach?: Reach;                 // was: string
  // ...
}
```

**Step 3: Type ContentFilter**

```typescript
export interface ContentFilter {
  contentType?: ContentType;     // was: string
  contentFormat?: ContentFormat; // was: string
  tags?: string[];
  limit?: number;
  offset?: number;
}
```

**Step 4: Fix compilation errors**

Run `npx tsc --noEmit` from `app/elohim-app` to find all callers that now need type assertions or fixes. Each is a potential silent failure that was previously invisible.

**Step 5: Run app tests**

Run: `cd app/elohim-app && pnpm test`
Expected: All tests pass.

**Step 6: Commit**

```bash
git add app/elohim-app/src/app/elohim/services/storage-client.service.ts
git commit -m "refactor(app): type StorageContentNode and ContentFilter with schema-generated enums"
```

---

### Task 10: Type `IStorageWriter` and `IStorageApi` Interfaces

**Files:**
- Modify: `app/elohim-app/src/app/elohim/interfaces/storage-writer.interface.ts`
- Modify: `app/elohim-app/src/app/elohim/interfaces/storage-api.interface.ts`

**Step 1: Add imports and type the fields**

In `storage-writer.interface.ts`, import types and change `masteryLevel?: string` to `masteryLevel?: MasteryLevel`, `engagementType?: string` to `EngagementType`.

In `storage-api.interface.ts`, import types and change `contentType?: string` to `contentType?: ContentType`, `reach?: string` to `reach?: Reach`, `contentFormat?: string` to `contentFormat?: ContentFormat`.

**Step 2: Fix compilation errors**

Run: `cd app/elohim-app && npx tsc --noEmit 2>&1 | head -50`
Fix cascading type errors — each one is a potential bug site.

**Step 3: Run tests**

Run: `cd app/elohim-app && pnpm test`

**Step 4: Commit**

```bash
git add app/elohim-app/src/app/elohim/interfaces/
git commit -m "refactor(app): type storage writer and API interfaces with schema-generated enums"
```

---

## Verification

After all tasks complete:

1. **Seeder typecheck**: `cd genesis/seeder && npx tsc --noEmit` → 0 errors
2. **Seeder tests**: `cd genesis/seeder && npx vitest run` → all pass
3. **Schema tests**: `pnpm run schema:test` → all pass
4. **App lint**: `cd app/elohim-app && pnpm run lint` → clean
5. **App tests**: `cd app/elohim-app && pnpm test` → all pass
6. **Library tests**: `cd app/elohim-library/projects/elohim-service && pnpm test` → all pass

The proof: revert `contentFormat: 'json'` to `contentFormat: 'structured'` in `seed-sqlite.ts` — `tsc --noEmit` should immediately reject it with a type error. Revert and confirm.
