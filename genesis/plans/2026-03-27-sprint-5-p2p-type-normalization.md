# Sprint 5: P2P Type Normalization — Conductor/DHT Content Through the Typed Pipeline

**Parent design:** `2026-03-27-typed-content-pipeline-design.md`
**Depends on:** Sprint 4 (signal harness + typed renderers)
**Goal:** Content arriving from any source (projection, conductor, Helia, IndexedDB) passes through the same typed pipeline. The generated types from Sprints 1-4 become the universal contract — not just the HTTP path.

> **P2P note:** No new storage entities. This sprint normalizes the wire format from existing Category A content arriving via conductor zome calls to match the typed ContentView shape that the projection tier already provides.

## Problem

Three content tiers exist, but only one is typed:

| Tier | Source | Wire Format | Typed Today? |
|------|--------|-------------|-------------|
| Local | IndexedDB | Pre-transformed ContentNode | Yes |
| Projection | Doorway HTTP → storage | ContentView (camelCase, parsed metadata) | Yes (Sprints 1-4) |
| Authoritative | Conductor → DHT zome calls | Content (snake_case, metadata_json: String) | **No** |

When the conductor tier is used (Tauri desktop, projection cache miss), zome responses arrive in snake_case with stringified metadata. The typed pipeline (`ContentService.transformContent()` → `parsePathView()` → typed renderers → signal harness) expects camelCase with parsed metadata.

There is **no normalization adapter** between conductor responses and the typed pipeline. On the projection path, elohim-storage's `ContentView` already does the conversion (Rust `#[serde(rename_all = "camelCase")]` + `parse_json_opt()`). On the conductor path, this conversion doesn't happen.

## Approach

Create a normalization adapter at the connection strategy boundary. When the conductor tier returns content, the adapter transforms it to match `ContentView` before the rest of the pipeline sees it. One transform, one place, all tiers produce the same shape.

## Tasks

### 1. Create conductor response normalizer

**File:** `app/elohim-library/projects/elohim-service/src/adapters/conductor-normalizer.ts`

Transforms conductor zome response → ContentView shape:

```typescript
import type { ContentView } from '../generated/content-view';

interface ConductorContentResponse {
  id: string;
  content_type: string;
  title: string;
  description: string;
  content: string;              // conductor uses 'content', not 'contentBody'
  content_format: string;
  tags: string[];
  source_path: string | null;
  related_node_ids: string[];
  reach: string;
  metadata_json: string;        // stringified, not parsed
  blob_cid: string | null;
  blob_hash: string | null;
  content_size_bytes: number | null;
  created_at: string;
  updated_at: string;
}

export function normalizeConductorContent(raw: ConductorContentResponse): ContentView {
  return {
    id: raw.id,
    appId: 'lamad',
    title: raw.title,
    description: raw.description ?? undefined,
    contentType: raw.content_type,
    contentFormat: raw.content_format,
    contentBody: raw.content ?? undefined,
    blobHash: raw.blob_hash ?? undefined,
    blobCid: raw.blob_cid ?? undefined,
    contentSizeBytes: raw.content_size_bytes ?? undefined,
    metadata: raw.metadata_json ? JSON.parse(raw.metadata_json) : undefined,
    reach: raw.reach,
    validationStatus: 'valid',
    createdBy: undefined,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
  };
}
```

**Verify:** Import generated `ContentView` type — the normalizer's return type IS the schema-generated contract. If the wire format changes, tsc catches it.

### 2. Wire normalizer into DirectConnectionStrategy

**File:** `app/elohim-library/projects/elohim-service/src/connection/direct-connection-strategy.ts`

When the direct strategy fetches content via zome call, normalize the response before returning:

- Find where `callZome('content_store', 'get_content', ...)` is called
- Wrap the response through `normalizeConductorContent()` before returning to the content resolver
- Same for `get_path`, `bulk_get_content`, and any other content-returning zome calls

### 3. Wire normalizer into TauriConnectionStrategy

**File:** `app/elohim-library/projects/elohim-service/src/connection/tauri-connection-strategy.ts`

Same pattern as DirectConnectionStrategy — normalize conductor responses before they enter the typed pipeline.

The Tauri strategy already has a `mapSessionResponse()` for snake_case → camelCase session data (line 565). Follow this pattern for content responses.

### 4. Wire normalizer into DoorwayConnectionStrategy conductor fallback

**File:** `app/elohim-library/projects/elohim-service/src/connection/doorway-connection-strategy.ts`

When doorway's projection cache misses and falls back to conductor (SourceTier.Authoritative), the response comes from the doorway's resolver which calls `__doorway_get` on the zome.

Check: does doorway normalize the conductor response before returning it as JSON? If doorway's `resolve_with_identity()` returns the conductor response as-is (snake_case), the Angular side needs to normalize. If doorway already converts via its Rust serde, this step is unnecessary for this strategy.

### 5. Normalize conductor blob path responses

**File:** `app/elohim-library/projects/elohim-service/src/connection/direct-connection-strategy.ts`

The direct/Tauri strategies use `/store/{hash}` for blobs (elohim-storage direct) vs `/api/blob/{hash}` (doorway proxy). Both return raw bytes — no normalization needed for blob content itself.

But blob URL construction differs:
- Doorway: `${baseUrl}/api/blob/${hash}`
- Direct: `${storageUrl}/store/${hash}`

Verify that `StorageClientService.getBlobUrl()` uses the strategy's `getBlobStorageUrl()` method, which already handles this. If not, wire it.

### 6. Add normalization tests

**File:** `app/elohim-library/projects/elohim-service/src/adapters/conductor-normalizer.spec.ts`

Test cases:
- Snake_case fields mapped to camelCase
- `content` → `contentBody`
- `metadata_json` string → parsed `metadata` object
- `metadata_json` null → `metadata: undefined`
- `content_size_bytes` number → `contentSizeBytes` number
- Return type matches `ContentView` (compile-time check)

### 7. Verify Tauri content loading end-to-end

If a Tauri dev environment is available:
- Fetch content by ID via direct conductor path
- Verify it produces a typed `ContentNode` with parsed metadata
- Verify `parsePathView()` works on path content from conductor
- Verify blob URLs resolve through the direct storage path

If not available, verify via unit tests with mock conductor responses.

### 8. Verify doorway conductor fallback

Test the doorway strategy's conductor fallback:
- Mock a projection cache miss
- Verify the conductor tier response is normalized to `ContentView`
- Or verify that doorway's Rust resolver already normalizes (making this a no-op)

## Verification

```bash
# Library builds with normalizer
cd app/elohim-library && pnpm run build

# Normalizer tests pass
cd app/elohim-library && pnpm test

# App builds (imports from library)
cd app/elohim-app && pnpm run build

# Seeder still clean (no seeder changes in this sprint)
cd genesis/seeder && npx tsc --noEmit && npx vitest run

# Type contract: normalizer return type matches ContentView
grep "ContentView" app/elohim-library/projects/elohim-service/src/adapters/conductor-normalizer.ts
```

## Exit Criteria

Content from ALL three tiers passes through the same typed pipeline:

```
Conductor (snake_case) → normalizeConductorContent() → ContentView (camelCase)
Storage HTTP (camelCase) → already ContentView
IndexedDB (pre-transformed) → already ContentNode

All three → ContentService.transformContent() → TypedContentNode
         → parsePathView() if path
         → typed renderer
         → signal harness → CreateEconomicEventInput
```

One contract. Three transports. No guesswork.

## Not In Scope

- Feedback coupling leg (Sprint 6 — architectural design needed first)
- Helia P2P content fetching for non-blob content (future — currently blobs only)
- Conductor write path normalization (seeder writes via HTTP, not conductor)
- DHT post-commit signal normalization (doorway handles this in Rust)
