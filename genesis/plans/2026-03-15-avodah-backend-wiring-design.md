# Avodah Backend Wiring — Design

**Date:** 2026-03-15
**Status:** Approved

## Overview

Replace `AvodahApiService` mock data with real storage API calls. Requires:
1. A new `PATCH /db/content/{id}` endpoint in `elohim-storage` (Rust)
2. `createContent()` and `updateContent()` methods added to `StorageApiService` (Angular)
3. `AvodahApiService` rewired to use real reads, creates, and patches

---

## Data Flow

```
AvodahApiService
  ├── getProjects()           → StorageApiService.getContents({ contentType: 'work-project' })
  ├── getStoriesForProject()  → StorageApiService.getContents({ contentType: 'work-story' })
  │                             + client-side filter by metadata.projectId
  ├── createProject()         → StorageApiService.createContent(input)
  ├── createStory()           → StorageApiService.createContent(input)
  └── updateStoryStatus()
        ├── non-terminal      → StorageApiService.updateContent(id, { metadata: { status } })
        └── terminal (isTerminal=true) → updateContent THEN createEconomicEvent (action: 'work')
```

---

## Rust Changes — `elohim-storage`

### New endpoint: `PATCH /db/content/{id}`

**File:** `elohim/elohim-storage/src/http.rs`

Add `PATCH` arm to the `GET/DELETE /db/content/{id}` handler (or a new handler).

### New input struct: `UpdateContentInput`

**File:** `elohim/elohim-storage/src/db/content.rs` (or `views.rs`)

```rust
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateContentInput {
    pub title: Option<String>,
    pub description: Option<Option<String>>,   // Some(None) = set null; None = no change
    pub content_body: Option<String>,
    pub content_format: Option<String>,
    pub metadata: Option<serde_json::Value>,   // shallow-merged into existing object
    pub tags: Option<Vec<String>>,
    pub reach: Option<String>,
}
```

### Metadata shallow-merge semantics

When `metadata` is provided in the patch, the handler:
1. Loads existing `metadata_json` from the DB row
2. Parses it as `serde_json::Map`
3. Iterates the patch object's keys and overwrites matching keys
4. Re-serializes to `metadata_json` string

This lets `PATCH { metadata: { status: "done" } }` update only `status`, leaving `projectId`, `priority`, cadence, etc. untouched.

### Returns

Updated `ContentWithTagsView` — same shape as `GET /db/content/{id}`.

---

## TypeScript SDK

Run `cargo test export_bindings` in `elohim/elohim-storage` after adding `#[ts(export)]` to
`UpdateContentInput` — this generates `UpdateContentInput.ts` into
`elohim/sdk/storage-client-ts/src/generated/`.

---

## Angular Changes — `StorageApiService`

### New interface methods (`IStorageApi`)

```typescript
// In storage-api.interface.ts
createContent(input: CreateContentInputView): Observable<ContentWithTagsView>;
updateContent(id: string, patch: UpdateContentPatch): Observable<ContentWithTagsView>;
```

### New patch type

```typescript
export interface UpdateContentPatch {
  title?: string;
  description?: string | null;
  contentBody?: string;
  contentFormat?: string;
  metadata?: Record<string, unknown>;  // shallow-merged server-side
  tags?: string[];
  reach?: string;
}
```

### New service methods (`StorageApiService`)

```typescript
createContent(input: CreateContentInputView): Observable<ContentWithTagsView> {
  return this.http.post<ContentWithTagsView>(`${this.baseUrl}/db/content`, input)
    .pipe(timeout(this.defaultTimeoutMs), catchError(...));
}

updateContent(id: string, patch: UpdateContentPatch): Observable<ContentWithTagsView> {
  return this.http.patch<ContentWithTagsView>(
    `${this.baseUrl}/db/content/${encodeURIComponent(id)}`, patch
  ).pipe(timeout(this.defaultTimeoutMs), catchError(...));
}
```

---

## Angular Changes — `AvodahApiService`

### Reads

`getContents()` returns `ContentWithTagsView[]`. A private `toContentNode()` helper maps
to the `ContentNode` domain type (same field projection as `ProjectionApiService.transformContent()`).

```typescript
private toContentNode(view: ContentWithTagsView): ContentNode {
  return {
    id: view.id,
    contentType: view.contentType,
    title: view.title,
    description: view.description ?? '',
    content: view.contentBody ?? '',
    contentFormat: view.contentFormat,
    tags: view.tags,
    relatedNodeIds: [],
    metadata: (view.metadata ?? {}) as ContentMetadata,
    reach: view.reach,
    createdAt: view.createdAt,
    updatedAt: view.updatedAt,
  };
}
```

### Writes

`updateStoryStatus()` signature gains an `isTerminal` flag:

```typescript
async updateStoryStatus(
  storyId: string,
  status: WorkStoryStatus,
  isTerminal = false
): Promise<void>
```

- Non-terminal: `PATCH { metadata: { status } }`
- Terminal: PATCH then `createEconomicEvent({ action: 'work', provider: currentUserId, contentId: storyId })`

`createStory()` and `createProject()` call `storageApi.createContent()` with a
`CreateContentInputView` built from the input.

### TODO (out of scope)

- Writes should eventually go through the Holochain conductor zome, not direct storage POST.
  Direct storage POST is intentional for MVP, consistent with the seed workflow.
  Tracked with `// TODO: [HOLOCHAIN-ZOME] route through conductor once zome supports work-story`

---

## What Stays Out of Scope

- Deep metadata merge (shallow merge is correct for status/priority patches)
- Story detail create/edit form (separate UI task)
- Conductor-first write path (future sprint)
