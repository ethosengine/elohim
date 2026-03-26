# Path-as-Content Seeder Migration

## Context

The path-content unification (commit 262ed4e1) removed separate path tables and the `/db/paths/bulk` endpoint. Paths are now ContentNodes with `contentType: 'path'`. The seeder's `seedPaths()` still calls the removed endpoint, causing 404s. Content seeds fine (1,362 items), but all 7 paths are lost.

## Design

### Approach: Seeder produces the canonical `sections` tree format

The Angular parser `parsePathView()` (learning-path.model.ts:589) expects a ContentNode whose `content` body is JSON with a recursive `sections` tree containing `items` at the leaves. The seeder will transform path JSON files into this format and seed them through the existing `/db/content/bulk` endpoint.

### Transformation: Path JSON → ContentNode

Each path JSON file becomes one `CreateContentInputView`:

```typescript
{
  id: path.id,
  contentType: 'path',
  contentFormat: 'structured',
  title: path.title,
  description: path.description,
  contentBody: JSON.stringify({ sections: chaptersToSections(path) }),
  metadata: {
    pathType: path.pathType ?? 'journey',
    difficulty: path.difficulty,
    estimatedDuration: path.estimatedDuration ?? path.estimatedMinutes + ' minutes',
    version: path.version ?? '1.0.0',
    purpose: path.purpose,
    thumbnailUrl: path.thumbnailUrl,
  },
  reach: 'public',
  tags: path.tags ?? [],
}
```

### Sections tree mapping

Path JSON uses two structures that both map to `RawSection[]`:

**Hierarchical** (elohim-protocol): `chapters` → `modules` → `sections` → `conceptIds[]`
```
chapter        → section { level: "unit" }
  module       →   section { level: "lesson" }
    section    →     section { level: "topic" }
      conceptId →      item { ref: conceptId, role: "step" }
```

**Flat** (governance paths, bdd-smoke-tests): `chapters` → `steps[]`
```
chapter        → section { level: "unit" }
  step         →   item { ref: resourceId, role: step.stepType ?? "step" }
```

### RawSection/RawItem format (matches parsePathView expectations)

```typescript
interface RawSection {
  id?: string;
  title?: string;
  description?: string;
  level?: string;           // "unit" | "lesson" | "topic"
  sections?: RawSection[];  // Recursive nesting
  items?: RawItem[];        // Leaf content references
  estimatedDuration?: string;
  optional?: boolean;
}

interface RawItem {
  ref: string;              // Content ID (conceptId or resourceId)
  role?: string;            // "step" | "checkpoint" | "reflection"
  title?: string;           // Display override
  narrative?: string;       // Pedagogical context
  learningObjectives?: string[];
  completionCriteria?: { type: string; threshold?: number };
}
```

### What changes in seed-sqlite.ts

1. **Replace `transformPath()`** — new function `transformPathToContent()` produces `CreateContentInput` with sections tree body
2. **Replace `seedPaths()`** — call existing `seedContent()` instead of `/db/paths/bulk`
3. **Remove `buildPathInput()`** — dead code
4. **Path loading unchanged** — `loadPathFiles()` stays the same
5. **Remove TODO comment** at lines 833-836

### What does NOT change

- `parsePathView()` in Angular — already parses sections/items format
- `/db/content/bulk` endpoint — handles any contentType
- Content seeding pipeline — paths become another content type

### No separate relationship records

Step references are embedded as `items[].ref` within the sections tree. `parsePathView()` resolves these by ID. No relationship records needed for navigation.

## Verification

```bash
# Run seeder against dev storage
cd genesis/seeder && pnpm start -- --storage-url http://localhost:8090

# Verify paths appear as content
curl "http://localhost:8090/db/content?contentType=path"

# Verify Angular renders
# Navigate to /lamad — should show path cards
```

## Files

- `genesis/seeder/src/seed-sqlite.ts` — main changes
- `genesis/data/lamad/paths/*.json` — 7 path files (input, unchanged)
- `app/elohim-app/src/app/lamad/models/learning-path.model.ts` — parsePathView (reference, unchanged)
