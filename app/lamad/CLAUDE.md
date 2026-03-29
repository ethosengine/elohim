# Lamad App Package

This directory is an **EPR artifact** — a protocol-validated app package that declares the learning domain's vocabulary, metadata schemas, and coupling contracts. The protocol validates its structure; lamad owns the semantic meaning.

## Two-Layer Type Architecture

```
Protocol SDK (elohim/sdk/schemas/)          This Package (app/lamad/)
├─ Wire types: ContentView,                 ├─ Domain types: PathMetadata,
│  CreateContentInput,                      │  ConceptMetadata, AssessmentMetadata
│  CreateEconomicEventInput                 ├─ Body types: EprCompositeBody,
├─ Enums: ContentType, Reach,              │  Section, Item
│  SubstrateSignal                          ├─ Coupling map: value flows + signals
├─ Manifest schema: validates              │  per content type
│  three-leg coupling structure             ├─ Type guards: isPathNode(),
└─ Generic metadata: {}                    │  isConceptNode(), isAssessmentNode()
                                            └─ manifest.json: vocabulary + coupling
```

The protocol owns the **envelope** (wire shape, field names, generic metadata bag). This package owns the **payload** (what metadata means for each content type, what the body contains for each format, what signals each interaction produces).

## Directory Structure

```
app/lamad/
├── manifest.json               # Vocabulary: 20 content types, formats, relationships, signals
│                                 Each content type declares three-leg coupling:
│                                 knowledge (graph edges) + value (REA flows) + governance (reach + model)
│                                 Plus claims (feedback: what outcomes it asserts, validity horizon)
├── schemas/                    # Metadata + body schemas per content type/format
│   ├── path-metadata.schema.json       # { pathType, difficulty, thumbnailUrl, estimatedDuration, ... }
│   ├── concept-metadata.schema.json    # { summary, sourcePath, relatedNodeIds, estimatedMinutes, ... }
│   ├── assessment-metadata.schema.json # { instrument, mode, scoringRules, ... }
│   └── epr-composite-body.schema.json  # { sections: Section[] } with items, refs, completion criteria
└── scripts/
    └── codegen.mjs             # Reads manifest + schemas → generates TypeScript
```

## Generated Output

`codegen.mjs` produces identical files to TWO locations:

| Location | Consumer |
|----------|----------|
| `app/elohim-app/src/app/lamad/generated/` | Angular app (import via `@app/lamad/generated/`) |
| `genesis/seeder/src/generated/` | Seeder (import via `./generated/`) |

Generated files:

| File | Contents |
|------|----------|
| `metadata-types.ts` | `PathMetadata`, `ConceptMetadata`, `AssessmentMetadata` interfaces |
| `body-types.ts` | `EprCompositeBody`, `Section`, `Item` interfaces |
| `content-node-types.ts` | `TypedContentNode` discriminated union, `isPathNode()` / `isConceptNode()` / `isAssessmentNode()` type guards |
| `coupling-map.ts` | `LAMAD_COUPLING_MAP` — value flows and governance signals per content type |
| `manifest-types.ts` | Content type lists, renderer map, signal map |

Protocol-level generated files (from `elohim/sdk/schemas/scripts/codegen-ts.mjs`) are ALSO distributed to these locations:

| File | Contents |
|------|----------|
| `schema-enums.ts` | `ContentType`, `ContentFormat`, `Reach`, `SubstrateSignal`, `MasteryLevel`, etc. |
| `create-content-input.ts` | `CreateContentInput` — wire input type |
| `content-view.ts` | `ContentView` — wire output type |
| `create-economic-event-input.ts` | `CreateEconomicEventInput` — REA event input |
| `economic-event-view.ts` | `EconomicEventView` — REA event output |
| `create-attestation-input.ts` | `CreateAttestationInput` — trust/reach attestation |

## Commands

```bash
# Generate lamad domain types (both locations)
pnpm run lamad:codegen

# Generate protocol types (both locations + library)
pnpm run schema:codegen:ts

# Validate manifest against protocol schema
pnpm run schema:test

# Validate seed data against content schemas
pnpm run schema:validate
```

## Rules

### Schema before code

Edit the schema first, then regenerate. Never hand-write types that a schema should own.

1. Protocol primitives (enums, wire types) → edit in `elohim/sdk/schemas/v1/`, run `pnpm run schema:codegen:ts`
2. Domain metadata/body shapes → edit in `app/lamad/schemas/`, run `pnpm run lamad:codegen`
3. Vocabulary (content types, signals, coupling) → edit `manifest.json`, run `pnpm run lamad:codegen`

### Typed metadata, not string keys

```typescript
// WRONG — untyped metadata access
const thumbUrl = (node.metadata as Record<string, unknown>)['thumbnailUrl'];

// RIGHT — use type guard to narrow, then access typed metadata
if (isPathNode(node)) {
  const thumbUrl = node.metadata.thumbnailUrl; // PathMetadata — typed
}
```

### Seeder and Angular share identical types

The codegen produces the same files to both locations. If a field exists in Angular, it exists in the seeder. The seeder seeds what Angular renders — no guesswork.

### Signal harness reads coupling from manifest

Renderers don't call economic event APIs directly. They emit `RendererCompletionEvent`. The `SignalHarnessService` reads `LAMAD_COUPLING_MAP` to translate:

```
Renderer → RendererCompletionEvent
    ↓
SignalHarnessService reads manifest coupling for contentType
    ↓
CreateEconomicEventInput { action, resourceConformsTo, ... }
    ↓
EconomicEventsApiService.createEconomicEvent()
```

### Three-leg coupling is required

The manifest schema (`app-manifest.schema.json`) rejects content types without `value` and `governance` legs. Claims (feedback) are also required — every content type must declare what outcomes it asserts and what would contradict them.

## Manifest Structure

```json
{
  "id": "manifest-lamad",
  "name": "lamad",
  "version": "1.0.0",
  "vocabulary": {
    "contentTypes": {
      "concept": {
        "description": "...",
        "metadataSchema": { "$ref": "./schemas/concept-metadata.schema.json" },
        "coupling": {
          "knowledge": { "relationships": { "CONTAINS": [...], "RELATES_TO": [...] } },
          "value": {
            "onConsume": { "action": "use", "resourceConformsTo": "learning-content" },
            "onComplete": { "action": "produce", "resourceConformsTo": "mastery-attestation" }
          },
          "governance": {
            "defaultReach": "commons",
            "governanceModel": "steward-consent",
            "signalTypes": ["learning-signal", "mastery-achieved"]
          },
          "claims": [{ "outcome": "learner-understands-concept", "contradictedBy": "retention-failure", ... }]
        }
      }
    },
    "contentFormats": {
      "epr-composite": {
        "renderer": "path-renderer",
        "bodySchema": { "$ref": "./schemas/epr-composite-body.schema.json" }
      }
    },
    "relationships": { "CONTAINS": { ... }, "RELATES_TO": { ... } },
    "signals": { "learning-signal": { "substrateSignal": "attention", "economicAction": "use" } },
    "observations": { "retention-check": { "polarity": "negative", "archetype": "retention-check" } }
  },
  "rendering": {
    "markdown-renderer": { "component": "MarkdownRendererComponent", "formats": ["markdown"] },
    "sophia-renderer": { "component": "SophiaRendererComponent", "formats": ["sophia-quiz-json"] }
  }
}
```

## Content Pipeline (end to end)

```
genesis/docs/*.md → elohim-import CLI → genesis/data/lamad/content/*.json
    ↓ seed-sqlite.ts (uses ConceptMetadata, PathMetadata from generated types)
POST /db/content/bulk (CreateContentInput — protocol wire type)
    ↓ elohim-storage (Rust: camelCase API → snake_case DB → camelCase response)
GET /db/content/{id} (ContentView — protocol wire type)
    ↓ Angular ContentService.transformContent() → TypedContentNode
    ↓ isPathNode() → parsePathView() uses EprCompositeBody/Section/Item
    ↓ RendererRegistry (from manifest) → MarkdownRenderer / SophiaRenderer
    ↓ RendererCompletionEvent → SignalHarnessService → CreateEconomicEventInput
    ↓ POST /db/events/bulk
```

## Related Files

| Purpose | Path |
|---------|------|
| Protocol schemas | `elohim/sdk/schemas/v1/` |
| Protocol codegen | `elohim/sdk/schemas/scripts/codegen-ts.mjs` |
| Manifest schema | `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json` |
| Angular generated | `app/elohim-app/src/app/lamad/generated/` |
| Seeder generated | `genesis/seeder/src/generated/` |
| Content service | `app/elohim-app/src/app/elohim/services/content.service.ts` |
| Path model | `app/elohim-app/src/app/lamad/models/learning-path.model.ts` |
| Signal harness | `app/elohim-app/src/app/lamad/services/signal-harness.service.ts` |
| Renderer registry | `app/elohim-app/src/app/lamad/renderers/renderer-registry.service.ts` |
| Design doc | `genesis/plans/2026-03-27-typed-content-pipeline-design.md` |
| Sprint plans | `genesis/plans/2026-03-27-sprint-{1-5}-*.md` |
| Feedback design | `genesis/plans/2026-03-28-feedback-information-flows-design.md` |
