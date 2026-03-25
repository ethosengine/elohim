# Unify LearningPaths as ContentNodes — EPR Composite Pattern

**Date**: 2026-03-25
**Status**: Approved
**Approach**: Clean break with seed data regeneration (Approach C)

## Context

The projection activation sprint (2026-03-24) revealed doorway implementing lamad domain logic because LearningPath has its own parallel type system (table, endpoints, views, Angular models) alongside ContentNode. The warm-up code needs `fetch_paths_individually` because `/db/paths` returns metadata-only — domain knowledge leaking into doorway.

The EPR specification already describes the solution: "Landing page becomes a ContentNode whose body is a layout of EPR references — the front door is inside the system."

## Core Decisions

### Two Surfaces

1. **EPR Composite Artifact** — A path is a ContentNode with `contentType: "path"`. Body is a fixed layout of EPR references. Publishable, cacheable, governed, projected through doorway. The crystallized form.

2. **Knowledge Graph** — Content relationships, mastery state, learner context. Pure DHT graph. Where discovery, authorship, and elohim personalization happen. The living form. NOT cached in doorway — per-learner, per-moment.

The published path is a snapshot projection of the knowledge graph. The graph is alive; the artifact is crystallized. Custom learning paths are captured as edits to the ContentNode.

### SDK Boundary (Web Components Model — Option B)

| Layer | Responsibility |
|-------|---------------|
| **elohim-core** (protocol) | ContentNode, Relationship, EPR addressing, reach, signals |
| **elohim-sdk** (pattern library) | Reference resolution (`epr:` refs → ContentNodes), renderer registry, `contentFormat: "epr-composite"` convention |
| **Pillar app** (lamad, qahal) | Content types, body schemas, renderers, business logic |

Each pillar defines its own body JSON schema. The SDK provides reference resolution and renderer registration — not body structure. Discovery, not design.

### DNA Deprecation

`LearningPath`, `PathChapter`, `PathStep` entry types deprecated — validation logic retained for existing DHT entries, coordinator functions stop creating them. New paths created as `Content` entries. Step connections as `Relationship` entries. Reclaims 3 entry type slots (~75 → ~72).

## Storage Schema

The `content` table absorbs all path data:

| Path field | Maps to | Notes |
|---|---|---|
| `id` | `id` | Same |
| `title` | `title` | Same |
| `description` | `description` | Same |
| `path_type` | `metadata_json.pathType` | Path-specific metadata |
| `difficulty` | `metadata_json.difficulty` | Path-specific metadata |
| `estimated_duration` | `metadata_json.estimatedDuration` | Path-specific metadata |
| `thumbnail_url` | `metadata_json.thumbnailUrl` | Path-specific metadata |
| `visibility` | `reach` | Semantic alignment |
| chapters/steps structure | `content_body` | Authored layout JSON |
| tags | `content_tags` join | Already exists |
| attestations | `content_attestations` | Already exists |

Content row fields:
- `content_type: "path"`
- `content_format: "epr-composite"`
- `content_body`: authored layout JSON (lamad's body schema)
- `metadata_json`: path-specific fields

### P2P Source-of-Truth Classification

| Entity | Classification | Source of Truth | Storage Role |
|--------|---------------|-----------------|--------------|
| Path (as ContentNode) | **A — Notarized** | DHT (`Content` entry type, `dht_anchor_hash`) | Projection for fast query |
| Step relationship | **A — Notarized** | DHT (`Relationship` entry type, `dht_anchor_hash`) | Projection for fast query |
| Path body (composite layout) | Part of Content entry | DHT (stored in `content` field of Content entry) | Projected into `content_body` column |
| Path metadata (difficulty, etc.) | Part of Content entry | DHT (stored in Content entry fields) | Projected into `metadata_json` column |

No new entity types introduced. Paths map into the existing `Content` (A) and `Relationship` (A) classifications. Storage tables are projections of DHT state — not sources of truth.

**Tables dropped**: `paths`, `chapters`, `steps`, `path_tags`, `path_attestations`

**Step graph edges** in existing `relationships` table:
- `source_id`: path content ID
- `target_id`: step content ID
- `relationship_type`: `"step"`
- `metadata_json`: `{"orderIndex": 1}`
- `inference_source`: `"explicit"`

## Path Composite Body Schema (Lamad's Concern)

> **P2P classification**: This body schema is the JSON content stored inside a `Content` entry's `content_body` field — not a separate entity. Source of truth is the DHT `Content` entry (Classification A — Notarized). The storage column is a projection.

Recursive nested sections supporting full LMS scope-and-sequence hierarchy:

```json
{
  "schemaVersion": 1,
  "pathType": "journey",
  "layout": "sequential",
  "sections": [
    {
      "id": "course-foundations",
      "title": "Foundations of Stewardship",
      "level": "course",
      "description": "...",
      "sections": [
        {
          "id": "unit-reach",
          "title": "The Reach Model",
          "level": "unit",
          "sections": [
            {
              "id": "lesson-concentric-circles",
              "title": "Understanding Concentric Circles",
              "level": "lesson",
              "items": [
                {
                  "ref": "epr:concept-concentric-circles",
                  "role": "step",
                  "narrative": "...",
                  "learningObjectives": ["..."],
                  "completionCriteria": { "type": "view" }
                },
                {
                  "ref": "epr:quiz-circles-check",
                  "role": "checkpoint",
                  "completionCriteria": { "type": "score", "threshold": 0.8 }
                }
              ]
            }
          ]
        }
      ]
    }
  ]
}
```

Scope-and-sequence mapping:

| Scope & Sequence | Body level | Old Angular model |
|---|---|---|
| Subject | top-level ContentNode | LearningPath |
| Course | `level: "course"` | PathChapter |
| Unit | `level: "unit"` | PathModule |
| Lesson | `level: "lesson"` | PathSection |
| Content | `items[].ref` | conceptIds / PathStep |

Sections nest recursively — not locked to 4 levels. Body format evolves via `schemaVersion` field. Enables flexible imports (e.g., Kolibri Studio content mapped to EPR).

Key principles:
- `ref` is an EPR reference — SDK resolves these. Everything else is lamad-specific.
- Pedagogical context (narrative, objectives, completionCriteria) lives in the body, not on graph edges. This is the teacher's editorial arrangement.
- `role` distinguishes step/checkpoint/reflection — lamad vocabulary.
- Graph edges (Relationships) created alongside: body = how to render, graph = how to traverse/discover.

## Import Pipeline + Re-seed

Clean break — all data from genesis, no user-created paths to preserve:

1. **Protocol schema** — Add `"path"` to `contentType` enum, `"epr-composite"` to `contentFormat` enum. Schema-first.
2. **Import CLI** — Path source content produces ContentNode JSON with composite body. Steps generate Relationship entries.
3. **Seeder** — Writes to `/db/content` (bulk) and `/db/relationships` (bulk). Never touches `/db/paths`.
4. **Diesel migration** — Drops path tables.
5. **Re-seed** — Fresh database, all paths as ContentNodes.

## Doorway Simplification

- Delete `fetch_paths_individually` from `warm.rs`
- `cache_stream.rs` sends paths as `cache.content` events (not separate `cache.path`)
- Remove `"LearningPath"` doc_type references in projection collections
- Doorway never learns the word "path"

## Angular Changes

- `LearningPath` interface → view parser function: `parsePathView(node: ContentNode): PathView`
- `PathService.getPath()` delegates to `ContentService.getContent()` then parses
- `PathOverviewComponent` renders from parsed view — same tree, different source
- `DataLoaderService.getPath()` → `getContent()` with type assertion
- Renderer registry: register `PathOverviewComponent` for `contentType: "path"`
- Old models (`LearningPath`, `PathChapter`, `PathModule`, `PathSection`, `PathStep`) replaced by `PathView` types derived from ContentNode

## DNA Deprecation

- `LearningPath`, `PathChapter`, `PathStep`: keep validation, stop creating
- New paths as `Content` entries with `content_type: "path"`
- Step connections as `Relationship` entries with `relationship_type: "step"`
- Link types (`PathToChapter`, `ChapterToStep`, etc.) become unused
- 3 entry types freed (~75 → ~72)

## SDK Surface (Emergent)

Not pre-built — discovered through this sprint:

- **`contentFormat: "epr-composite"`** — convention signaling "body contains EPR references"
- **Reference resolution** — walks JSON body, finds `ref: "epr:..."`, resolves to ContentNodes
- **Renderer registry** — `registerRenderer(contentType, component)` at Angular bootstrap
- **Relationship conventions** — `"step"`, `"contains"`, `"prerequisite"` as edge types with `orderIndex` metadata

Documented as patterns emerge. This sprint produces the first reference implementation. Future composites (governance proposals, stewardship portfolios, curated collections) follow the same pattern.

## What Doesn't Change

- EPR Head format (already carries contentType)
- Signal subscriber (already type-agnostic)
- Projection engine (already type-agnostic)
- Reach/governance (already per-ContentNode)

## References

- EPR composite pattern: `genesis/docs/content/elohim-protocol/protocol-specification.md`
- Protocol schema: `genesis/schema/`
- Current path model: `app/elohim-app/src/app/lamad/models/learning-path.model.ts`
- Current path storage views: `elohim/elohim-storage/src/views.rs`
- Doorway warm-up hack: `doorway/doorway-service/src/projection/warm.rs:fetch_paths_individually`
- Cache stream: `elohim/elohim-storage/src/cache_stream.rs`
- Emerging SDK boundary: memory `emerging-sdk-boundary.md`
- Two surfaces: memory `project-two-surfaces-artifact-and-graph.md`
