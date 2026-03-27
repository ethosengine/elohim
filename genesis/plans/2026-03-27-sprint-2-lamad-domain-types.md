# Sprint 2: Lamad Domain Types

**Parent design:** `2026-03-27-typed-content-pipeline-design.md`
**Depends on:** Sprint 1 (protocol schema foundation)
**Goal:** Lamad defines its metadata and body schemas. Codegen produces discriminated TypeScript unions. Angular and seeder stop hand-writing content type interfaces.

> **P2P note:** No new storage entities. These schemas describe the SHAPE of metadata and contentBody for existing Category A content records. The schemas live in the app manifest package, not the protocol — lamad vocabulary, not protocol law.

## Tasks

### 1. Create path metadata schema

**File:** `app/lamad/schemas/path-metadata.schema.json`

```json
{
  "$id": "lamad:schema:metadata:path",
  "title": "PathMetadata",
  "description": "Metadata shape for contentType 'path'. Stored in ContentView.metadata.",
  "type": "object",
  "properties": {
    "pathType": { "type": "string", "description": "journey | guided | self-paced | assessment" },
    "difficulty": { "type": "string", "description": "beginner | intermediate | advanced" },
    "estimatedDuration": { "type": "string", "description": "Human-readable duration (e.g. '6-8 hours')" },
    "version": { "type": "string" },
    "purpose": { "type": "string" },
    "thumbnailUrl": { "type": "string", "description": "Blob path or resolved URL for path thumbnail" },
    "thumbnailAlt": { "type": "string" }
  },
  "additionalProperties": true
}
```

### 2. Create concept metadata schema

**File:** `app/lamad/schemas/concept-metadata.schema.json`

Fields: `summary`, `sourcePath`, `relatedNodeIds` (string[]), `estimatedMinutes` (number), `thumbnailUrl`, `bloomsLevel`, `sourceDoc`, `relationships` (array), `did`, `openGraphMetadata`, `linkedData`.

### 3. Create assessment metadata schema

**File:** `app/lamad/schemas/assessment-metadata.schema.json`

Fields: `instrument` (string ref), `mode` (mastery | discovery | reflection), `scoringRules`, `subscales`.

### 4. Create epr-composite body schema

**File:** `app/lamad/schemas/epr-composite-body.schema.json`

```json
{
  "$id": "lamad:schema:body:epr-composite",
  "title": "EprCompositeBody",
  "description": "Body shape for contentFormat 'epr-composite'. Parsed from ContentView.contentBody.",
  "type": "object",
  "required": ["sections"],
  "properties": {
    "schemaVersion": { "type": "number" },
    "pathType": { "type": "string" },
    "layout": { "type": "string", "enum": ["sequential", "branching", "exploratory"] },
    "sections": {
      "type": "array",
      "items": { "$ref": "#/$defs/Section" }
    }
  },
  "$defs": {
    "Section": {
      "type": "object",
      "required": ["id"],
      "properties": {
        "id": { "type": "string" },
        "title": { "type": "string" },
        "description": { "type": "string" },
        "level": { "type": "string", "enum": ["course", "unit", "lesson", "item"] },
        "estimatedDuration": { "type": "string" },
        "optional": { "type": "boolean" },
        "items": { "type": "array", "items": { "$ref": "#/$defs/Item" } },
        "sections": { "type": "array", "items": { "$ref": "#/$defs/Section" } }
      }
    },
    "Item": {
      "type": "object",
      "required": ["ref"],
      "properties": {
        "ref": { "type": "string", "description": "EPR reference (e.g. 'epr:concept-id' or bare 'concept-id')" },
        "role": { "type": "string", "enum": ["step", "checkpoint", "optional", "reflection"] },
        "title": { "type": "string" },
        "narrative": { "type": "string" },
        "learningObjectives": { "type": "array", "items": { "type": "string" } },
        "completionCriteria": {
          "type": "object",
          "properties": {
            "type": { "type": "string", "enum": ["view", "score", "time", "interaction"] },
            "threshold": { "type": "number" }
          }
        }
      }
    }
  }
}
```

### 5. Wire schemas into lamad manifest

**File:** `app/lamad/manifest.json`

Add `metadataSchema` to each content type declaration:

```json
"path": {
  "description": "A curated learning journey...",
  "metadataSchema": { "$ref": "./schemas/path-metadata.schema.json" },
  "coupling": { ... }
}
```

Add `bodySchema` to epr-composite format:

```json
"epr-composite": {
  "description": "EPR composite layout...",
  "renderer": "path-renderer",
  "bodySchema": { "$ref": "./schemas/epr-composite-body.schema.json" }
}
```

### 6. Build manifest codegen for domain types

**File:** `app/lamad/scripts/codegen-types.mjs`

Reads manifest + companion schemas, produces:

- `app/lamad/generated/metadata-types.ts` — `PathMetadata`, `ConceptMetadata`, `AssessmentMetadata` interfaces
- `app/lamad/generated/body-types.ts` — `EprCompositeBody`, `Section`, `Item` interfaces
- `app/lamad/generated/content-node-types.ts` — discriminated union:

```typescript
export type TypedContentNode =
  | (ContentView & { contentType: 'path'; metadata: PathMetadata })
  | (ContentView & { contentType: 'concept'; metadata: ConceptMetadata })
  | (ContentView & { contentType: 'assessment'; metadata: AssessmentMetadata })
  | (ContentView & { contentType: string; metadata: Record<string, unknown> }); // fallback

export function isPathNode(node: ContentView): node is TypedContentNode & { contentType: 'path' };
export function isConceptNode(node: ContentView): node is TypedContentNode & { contentType: 'concept' };
```

`ContentView` is imported from the protocol-generated types (sprint 1).

### 7. Replace hand-written RawSection/RawItem in learning-path.model.ts

**File:** `app/elohim-app/src/app/lamad/models/learning-path.model.ts`

- Remove `interface RawSection` (lines 562-571) — import `Section` from generated
- Remove `interface RawItem` (lines 574-581) — import `Item` from generated
- `parsePathView()` types its body parse as `EprCompositeBody` instead of `Record<string, unknown>`
- `enrichSection()` accepts `Section` instead of `RawSection`
- `thumbnailUrl` comes from typed `PathMetadata.thumbnailUrl` (already resolved by ContentService)

### 8. Replace hand-written ContentNode metadata access

Replace every `(meta as Record<string, unknown>)['thumbnailUrl']` pattern:

- `parsePathView()` — `const meta: PathMetadata = node.metadata as PathMetadata`
- `transformContentNodesToPathIndex()` — `parsed.thumbnailUrl` (now typed)
- `data-loader.service.ts` — path index transform uses typed entries

### 9. Distribute generated types to seeder

Copy `app/lamad/generated/` types to `genesis/seeder/src/generated/` as part of codegen (same pattern as `schema-enums.ts` distribution).

The seeder's `transformContent()` and `transformPathToContent()` return properly typed metadata matching the schemas.

## Verification

```bash
# Manifest validates with new schema refs
pnpm run schema:validate

# Codegen produces domain types
cd app/lamad && node scripts/codegen-types.mjs

# Generated types exist
ls app/lamad/generated/metadata-types.ts
ls app/lamad/generated/body-types.ts
ls app/lamad/generated/content-node-types.ts

# App builds with zero metadata casting
cd app/elohim-app && pnpm run build
grep -r "as Record<string, unknown>" src/app/lamad/models/ | wc -l  # should be 0

# Seeder compiles with typed metadata
cd genesis/seeder && npx tsc --noEmit

# Seeder tests pass
cd genesis/seeder && npx vitest run

# parsePathView correctly types sections
grep "RawSection\|RawItem" app/elohim-app/src/app/lamad/models/learning-path.model.ts  # should be 0
```
