# Typed Content Pipeline: Schema to Screen

## Problem

The content pipeline has five structural brittleness patterns:

1. **No schema-to-screen contract** — protocol schema generates input types, Angular hand-writes everything else (410-line ContentNode, 650-line learning-path.model.ts)
2. **Untyped metadata bag** — 96 `as Record<string, unknown>` casts, 10 files reaching in with string keys
3. **No body schema** — epr-composite sections/items structure is a handshake between seeder and parsePathView(), no validation
4. **Scattered field resolution** — thumbnailUrl accessed 13 different ways across 13 files
5. **Signal gap** — manifest declares coupling (value + governance), renderers emit completion events, but nothing bridges them

## Goal

Land on `https://alpha.elohim.host/lamad` and see:
- Path cards with thumbnails (resolved, not broken)
- Click a path → course overview with chapters, modules, concept items (correctly counted)
- Click into a lesson → content renders via typed renderer (markdown, sophia, iframe)
- Complete a quiz → sophia produces Recognition → signal harness translates to REA economic event + governance signal, as declared in the lamad manifest

All typed. The seeder reads the same type definitions Angular renders with. No hand-written types. No guesswork.

## Architecture: Three Layers

```
Protocol SDK (elohim/sdk/schemas/)
  Owns: wire types, EPR resolution, substrate signal enum, coupling validation
  Enforces: every content interaction produces declared economic events

App Manifest (app/lamad/)
  Owns: domain vocabulary, metadata schemas, body schemas, renderer declarations
  Declares: how signals map to core primitives, what metadata means per content type

Angular App (app/elohim-app/)
  Consumes: generated types from both layers
  Never hand-writes: ContentNode, PathView, metadata interfaces, renderer registration
```

## Four-Sprint Build Order

### Sprint 1: Protocol Schema Foundation

Complete the 30% gap in protocol schemas. Everything downstream inherits from here.

**Deliverables:**
- Substrate signal enum as proper schema (`substrate-signal.schema.json`) with `attention`, `compute`, `storage`, `bandwidth`, `energy`, `time`, `resource`
- `metadataSchema` slot added to `ContentTypeDeclaration` in app-manifest.schema.json
- `bodySchema` slot added to `ContentFormatDeclaration`
- REA schemas: `create-economic-event-input.schema.json`, `economic-event-view.schema.json` (capturing the 25+ fields already in Rust ts-rs types)
- Attestation schema: `create-attestation-input.schema.json`
- Codegen produces all as TypeScript — distributed to seeder + app + library
- Existing `schema:test` + `schema:validate` cover new schemas

**Exit criteria:** `pnpm run schema:codegen:ts` generates typed `CreateEconomicEventInput`, `EconomicEventView`, `SubstrateSignal` enum. All 3 distribution locations have identical files. Existing tests still pass.

### Sprint 2: Lamad Domain Types

Lamad defines its metadata and body schemas using the protocol foundation.

**Deliverables:**
- `app/lamad/schemas/path-metadata.schema.json` — `{ pathType, difficulty, thumbnailUrl, thumbnailAlt, estimatedDuration, version, purpose }`
- `app/lamad/schemas/concept-metadata.schema.json` — `{ summary, sourcePath, relatedNodeIds, estimatedMinutes, thumbnailUrl, bloomsLevel }`
- `app/lamad/schemas/assessment-metadata.schema.json` — `{ instrument, scoringRules, mode }`
- `app/lamad/schemas/epr-composite-body.schema.json` — `{ sections: Section[] }` with `Section { id, title, items: Item[], sections?: Section[] }` and `Item { ref, role, title, completionCriteria }`
- Manifest `contentTypes` entries gain `metadataSchema: { "$ref": "./schemas/path-metadata.schema.json" }`
- Manifest `contentFormats.epr-composite` gains `bodySchema: { "$ref": "./schemas/epr-composite-body.schema.json" }`
- Codegen reads manifest + companion schemas → produces:
  - `PathMetadata`, `ConceptMetadata`, `AssessmentMetadata` interfaces
  - `EprCompositeBody`, `Section`, `Item` interfaces
  - Discriminated `ContentNode<T>` union narrowed by `contentType`
  - Type guard functions: `isPathNode(node)`, `isConceptNode(node)`

**Exit criteria:**
- `parsePathView()` uses generated `EprCompositeBody` — no more `RawSection`/`RawItem` hand-written interfaces
- `transformContentNodesToPathIndex()` uses `PathMetadata` — no more `(meta as Record<string, unknown>)['thumbnailUrl']`
- seeder's `transformContent()` and `transformPathToContent()` produce typed metadata matching the schemas
- `tsc --noEmit` passes with zero `as Record<string, unknown>` casts for metadata access in lamad pillar

### Sprint 3: Codegen Helper + Seeder/App Alignment

Single command generates both Rust and TypeScript from lamad schemas.

**Deliverables:**
- `app/lamad/scripts/codegen.mjs` reads manifest + schemas, produces:
  - TypeScript: discriminated unions, metadata interfaces, type guards, renderer registry config
  - Output to `app/lamad/generated/` (consumed by Angular) and `genesis/seeder/src/generated/` (consumed by seeder)
- Seeder imports generated types — `transformContent()` returns `ContentNode<'concept'>`, `transformPathToContent()` returns `ContentNode<'path'>`
- Angular imports same generated types — `ContentService.transformContent()` returns discriminated union
- `content-node.model.ts` (410 lines) replaced by generated types + thin app-layer extensions
- `learning-path.model.ts` `parsePathView()` uses generated `EprCompositeBody` with `Section`/`Item` types
- Constants-sync test verifies seeder and app generated files are identical

**Exit criteria:**
- `pnpm run lamad:codegen` regenerates all types
- Zero hand-written content type interfaces remain in seeder or Angular
- Thumbnails load on landing page (thumbnailUrl resolved from typed PathMetadata)
- Start Chapter navigates (sections parsed from typed EprCompositeBody)
- Chapter concept counts correct (typed Section.items → Section.conceptIds)

### Sprint 4: Signal Harness + Typed Renderers

Wire renderers to manifest coupling. Completion events produce economic events.

**Deliverables:**
- **Signal harness service** (`app/lamad/services/signal-harness.service.ts`):
  - Reads manifest coupling for content type
  - Translates `RendererCompletionEvent` → `CreateEconomicEventInput` (using generated REA type from sprint 1)
  - Dispatches to `EconomicEventsApiService.createEconomicEvent()`
  - Emits governance signal as declared in manifest's `signalTypes`
- **Renderer registry auto-wired from manifest**:
  - `renderer-initializer.service.ts` reads manifest `rendering` section
  - No more hard-coded `registry.register(['markdown'], MarkdownRendererComponent)` calls
  - BYO renderer: add format to manifest + register component
- **Typed renderer inputs**:
  - `SophiaRendererComponent` receives `ContentNode<'assessment'>` (not generic `ContentNode`)
  - `MarkdownRendererComponent` receives `ContentNode<'concept'>` or `ContentNode<'article'>`
  - `PathViewerComponent` receives `ContentNode<'path'>` with typed `EprCompositeBody`
- **Signal flow wired**:
  - `content-viewer.component.ts` calls `signalHarness.onRendererComplete(node, event)`
  - Harness looks up manifest: `concept.coupling.value.onComplete → { action: 'produce', resourceConformsTo: 'mastery-attestation' }`
  - Creates `CreateEconomicEventInput { action: 'produce', provider: agentId, receiver: contentId, resourceConformsTo: 'mastery-attestation' }`
  - Posts to `/db/events/bulk`
- **Scaffold for sprint 5+ (pseudo-code)**:
  - Manifest governance revocation (EPR lifecycle)
  - Aggregation instruments (mastery from signals, stewardship from contributions)
  - Manifest version enforcement (minimum version decree)

**Exit criteria:**
- Complete a sophia quiz on alpha → network tab shows economic event POST
- Event contains correct `action`, `resourceConformsTo`, `lamadEventType` as declared in manifest
- Renderer registry has zero hard-coded format lists — all from manifest
- `RendererCompletionEvent` type includes manifest signal context

## Key Decisions

1. **Protocol schema owns the envelope, manifest owns the payload.** `ContentView.metadata: {}` is protocol. `PathMetadata.thumbnailUrl` is lamad. They compose via `$ref`.

2. **Manifest is an EPR artifact.** Content-addressed, versioned, governed. Protocol validates three-leg coupling structure. Governance can revoke bad manifests.

3. **Signal harness is SDK-level enforcement.** The harness is the only path from renderer to protocol. Apps can't skip economic events because the harness IS the render-to-protocol bridge.

4. **One codegen, both languages.** Seeder (TypeScript) and Angular (TypeScript) share identical generated types. Rust types continue from ts-rs but protocol schemas are authoritative.

## Files Changed (Sprint 1 Preview)

| File | Change |
|------|--------|
| `elohim/sdk/schemas/v1/enums/substrate-signal.schema.json` | New — enum with attention, compute, storage, bandwidth, energy, time, resource |
| `elohim/sdk/schemas/v1/inputs/create-economic-event-input.schema.json` | New — 25 fields matching Rust CreateEconomicEventInputView |
| `elohim/sdk/schemas/v1/views/economic-event-view.schema.json` | New — matching Rust EconomicEventView |
| `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json` | Add metadataSchema to ContentTypeDeclaration, bodySchema to ContentFormatDeclaration |
| `elohim/sdk/schemas/scripts/codegen-ts.mjs` | Extend to scan new schema directories |
| `elohim/sdk/schemas/scripts/test-manifest-schema.mjs` | Add tests for new metadataSchema/bodySchema slots |
