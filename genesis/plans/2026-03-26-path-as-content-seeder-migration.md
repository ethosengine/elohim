# Path-as-Content Seeder Migration — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Migrate path seeding from the removed `/db/paths/bulk` endpoint to `/db/content/bulk` with `contentType: 'path'`, producing the `sections` tree format that `parsePathView()` expects.

**Architecture:** Replace `transformPath()` → `seedPaths()` with `transformPathToContent()` → `seedContent()`. The new transform maps chapters/modules/sections/conceptIds/steps into a recursive `RawSection[]` tree with `RawItem[]` leaves, serialized as the content body. All other seeder infrastructure (loading, batching, logging) stays the same.

**Tech Stack:** TypeScript (seeder), existing `/db/content/bulk` API

---

### Task 1: Replace `transformPath()` with `transformPathToContent()`

**Files:**
- Modify: `genesis/seeder/src/seed-sqlite.ts:668-808`

**Step 1: Replace the `transformPath` function**

Delete lines 668-808 (`transformPath`) and replace with:

```typescript
/**
 * Transform a path JSON file into a CreateContentInput for /db/content/bulk.
 * Chapters/modules/sections/conceptIds → recursive RawSection[] tree with RawItem[] leaves.
 * This is the format parsePathView() in learning-path.model.ts expects.
 */
function transformPathToContent(json: PathJson): CreateContentInput {
  const sections = chaptersToSections(json);

  // Build metadata
  const metadata: Record<string, unknown> = {};
  if (json.pathType) metadata.pathType = json.pathType;
  if (json.difficulty) metadata.difficulty = json.difficulty;
  if (json.estimatedDuration) metadata.estimatedDuration = json.estimatedDuration;
  if (json.estimatedMinutes) metadata.estimatedDuration = `${json.estimatedMinutes} minutes`;
  if (json.version) metadata.version = json.version;
  if (json.purpose) metadata.purpose = json.purpose;
  if (json.thumbnailUrl) metadata.thumbnailUrl = json.thumbnailUrl;
  if (json.thumbnailAlt) metadata.thumbnailAlt = json.thumbnailAlt;

  const contentBody = JSON.stringify({ sections });

  return {
    id: json.id,
    title: json.title,
    description: json.description,
    contentType: 'path',
    contentFormat: 'structured',
    contentBody,
    contentSizeBytes: Buffer.byteLength(contentBody, 'utf-8'),
    metadataJson: Object.keys(metadata).length > 0 ? JSON.stringify(metadata) : undefined,
    reach: json.visibility || 'public',
    tags: json.tags || [],
  };
}
```

**Step 2: Add the `chaptersToSections` helper**

Add immediately above `transformPathToContent`:

```typescript
interface SectionNode {
  id?: string;
  title?: string;
  description?: string;
  level?: string;
  sections?: SectionNode[];
  items?: SectionItem[];
  estimatedDuration?: string;
  optional?: boolean;
}

interface SectionItem {
  ref: string;
  role?: string;
  title?: string;
  narrative?: string;
  learningObjectives?: string[];
  completionCriteria?: { type: string; threshold?: number };
}

/**
 * Convert path JSON chapters into the sections tree format.
 * Handles three input shapes:
 * 1. chapters → modules → sections → conceptIds (elohim-protocol)
 * 2. chapters → steps (governance paths, bdd-smoke-tests)
 * 3. flat conceptIds (no chapters)
 */
function chaptersToSections(json: PathJson): SectionNode[] {
  // Handle flat conceptIds (no chapters)
  if ((!json.chapters || json.chapters.length === 0) && json.conceptIds?.length) {
    return [{
      id: `${json.id}-default`,
      title: json.title,
      description: json.description,
      level: 'unit',
      items: json.conceptIds.map(id => ({
        ref: id,
        role: 'step',
        title: formatConceptTitle(id),
      })),
    }];
  }

  if (!json.chapters) return [];

  return json.chapters.map((chapter, ci) => {
    const section: SectionNode = {
      id: chapter.id,
      title: chapter.title,
      description: chapter.description,
      level: 'unit',
      estimatedDuration: chapter.estimatedDuration,
    };

    // Shape 1: chapters → modules → sections → conceptIds
    if (chapter.modules?.length) {
      section.sections = [];
      for (const mod of chapter.modules) {
        if (mod.sections) {
          for (const sec of mod.sections) {
            section.sections.push({
              id: sec.id,
              title: sec.title ?? mod.title,
              description: sec.description,
              level: 'lesson',
              items: (sec.conceptIds ?? []).map(id => ({
                ref: id,
                role: 'step',
                title: formatConceptTitle(id),
              })),
            });
          }
        }
      }
      return section;
    }

    // Shape 2: chapters → steps (flat)
    if (chapter.steps?.length) {
      section.items = chapter.steps.map(step => {
        const item: SectionItem = {
          ref: step.resourceId,
          role: normalizeStepType(step.stepType),
          title: step.stepTitle || step.title || formatConceptTitle(step.resourceId),
        };
        if (step.stepNarrative) item.narrative = step.stepNarrative;
        if (step.learningObjectives) item.learningObjectives = step.learningObjectives;
        if (step.completionCriteria) {
          item.completionCriteria = Array.isArray(step.completionCriteria)
            ? { type: step.completionCriteria.join(', ') }
            : step.completionCriteria;
        }
        return item;
      });
      return section;
    }

    // Shape 2b: chapters → conceptIds (flat)
    if (chapter.conceptIds?.length) {
      section.items = chapter.conceptIds.map(id => ({
        ref: id,
        role: 'step',
        title: formatConceptTitle(id),
      }));
      return section;
    }

    return section;
  });
}
```

**Step 3: Verify it compiles**

The seeder is TypeScript — no separate compile step needed, but check types:
```bash
cd genesis/seeder && npx tsc --noEmit src/seed-sqlite.ts 2>&1 | head -20
```

**Step 4: Commit**

```bash
git add genesis/seeder/src/seed-sqlite.ts
git commit -m "refactor(seeder): replace transformPath with sections-tree transform"
```

---

### Task 2: Replace `seedPaths()` call with `seedContent()`

**Files:**
- Modify: `genesis/seeder/src/seed-sqlite.ts:833-854` (seedPaths function)
- Modify: `genesis/seeder/src/seed-sqlite.ts:1095-1138` (Phase 2 call site)

**Step 1: Delete the `seedPaths` function and TODO comment**

Delete lines 833-854 (the TODO comment and `seedPaths` function). It's now dead code — paths go through `seedContent()`.

**Step 2: Update the Phase 2 call site (lines ~1095-1138)**

Replace the Phase 2 block with:

```typescript
  if (!CONTENT_ONLY) {
    console.log(`\n${'='.repeat(70)}`);
    console.log(`Phase 2: Seeding Paths as Content`);
    console.log(`${'='.repeat(70)}`);

    const pathTimer = new Timer();
    console.log(`\nLoading path files...`);
    let paths = loadPathFiles();
    console.log(`   Loaded ${formatCount(paths.length)} paths`);

    if (LIMIT > 0 && paths.length > LIMIT) {
      console.log(`   Limiting to ${LIMIT} items`);
      paths = paths.slice(0, LIMIT);
    }

    console.log(`\nTransforming paths to content nodes...`);
    const pathContentInputs = paths.map(p => {
      const input = transformPathToContent(p);
      // Update thumbnailUrl to blob reference if we uploaded one
      if (p.thumbnailUrl && uploadedThumbnails.has(p.thumbnailUrl)) {
        const blobHash = uploadedThumbnails.get(p.thumbnailUrl)!;
        const meta = input.metadataJson ? JSON.parse(input.metadataJson) : {};
        meta.thumbnailUrl = `/blob/${blobHash}`;
        input.metadataJson = JSON.stringify(meta);
      }
      return input;
    });

    // Count steps for logging
    const totalSteps = pathContentInputs.reduce((sum, p) => {
      try {
        const body = JSON.parse(p.contentBody || '{}');
        return sum + countItems(body.sections || []);
      } catch { return sum; }
    }, 0);
    console.log(`   Transformed ${formatCount(pathContentInputs.length)} paths with ${formatCount(totalSteps)} steps`);

    console.log(`\nSeeding paths to database...`);
    try {
      const result = await seedContent(pathContentInputs);
      totalInserted += result.inserted;
      totalSkipped += result.skipped;
      totalErrors.push(...result.errors);
      console.log(`   ${result.inserted} paths inserted, ${result.skipped} skipped`);
    } catch (err) {
      console.error(`   Path seeding failed: ${err}`);
      totalErrors.push(`Paths: ${err}`);
    }

    console.log(`\nPath seeding complete in ${pathTimer.elapsed()}`);
  }
```

**Step 3: Add `countItems` helper** (near the other helpers)

```typescript
/** Count total items across a sections tree (for logging) */
function countItems(sections: SectionNode[]): number {
  let count = 0;
  for (const s of sections) {
    count += s.items?.length ?? 0;
    if (s.sections) count += countItems(s.sections);
  }
  return count;
}
```

**Step 4: Commit**

```bash
git add genesis/seeder/src/seed-sqlite.ts
git commit -m "fix(seeder): seed paths via /db/content/bulk instead of removed /db/paths/bulk"
```

---

### Task 3: Remove dead types

**Files:**
- Modify: `genesis/seeder/src/seed-sqlite.ts:145-190` (dead type definitions)

**Step 1: Delete `CreatePathInput`, `CreateChapterInput`, `CreateStepInput` interfaces**

These are at lines 145-190. They're now unused — paths use `CreateContentInput`.

**Step 2: Verify no remaining references**

```bash
grep -n 'CreatePathInput\|CreateChapterInput\|CreateStepInput' genesis/seeder/src/seed-sqlite.ts
```

Expected: no matches.

**Step 3: Commit**

```bash
git add genesis/seeder/src/seed-sqlite.ts
git commit -m "chore(seeder): remove dead path/chapter/step input types"
```

---

### Task 4: Verify end-to-end

**Step 1: Run seeder dry-run**

```bash
cd genesis/seeder && pnpm start -- --dry-run --storage-url http://localhost:8090
```

Expected: "Would seed 7 content items" for paths, no errors.

**Step 2: Run seeder against dev storage (if available)**

```bash
cd genesis/seeder && pnpm start -- --storage-url http://localhost:8090
```

**Step 3: Verify paths appear as content**

```bash
curl "http://localhost:8090/db/content?contentType=path" | jq '.items | length'
```

Expected: 7 (or however many path JSON files exist).

**Step 4: Verify content body has sections tree**

```bash
curl "http://localhost:8090/db/content?contentType=path&limit=1" | jq '.items[0].contentBody' -r | jq '.sections[0]'
```

Expected: A section object with `id`, `title`, `level`, and either `items` or nested `sections`.

**Step 5: Push**

```bash
git push origin dev
```

---

## Key files reference

| File | Role |
|------|------|
| `genesis/seeder/src/seed-sqlite.ts` | Main file to modify — transform + seed functions |
| `genesis/data/lamad/paths/*.json` | 7 path JSON files (input, unchanged) |
| `app/elohim-app/src/app/lamad/models/learning-path.model.ts:589` | `parsePathView()` — defines the target format (reference only) |
| `elohim/elohim-storage/src/views.rs:997` | `CreateContentInputView` — API contract (reference only) |
