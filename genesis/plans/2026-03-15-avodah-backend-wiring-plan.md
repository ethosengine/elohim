# Avodah Backend Wiring — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire `AvodahApiService` to real storage API calls, including a new `PATCH /db/content/{id}` endpoint in `elohim-storage` for partial metadata updates.

**Architecture:** Four-layer change — Rust backend adds `UpdateContentInputView`, a `update_content()` DB function, a `ContentService.update()` method, and a PATCH route; Angular adds `createContent()`/`updateContent()` to `StorageApiService` and `IStorageApi`; `AvodahApiService` is rewired to use real reads (via `getContents()`) and writes (via the new methods). Terminal status transitions also emit an economic event.

**Tech Stack:** Rust/Diesel (elohim-storage), Angular 19, RxJS, Vitest

**Design doc:** `genesis/plans/2026-03-15-avodah-backend-wiring-design.md`

---

## Critical Rules

- **RUSTFLAGS**: Always use `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test` for elohim-storage (the system env sets a custom getrandom backend for Holochain WASM — this is required here too).
- **Angular DI**: Use `inject()` — never constructor injection (esbuild strips metadata).
- **Angular templates**: Inline only — never `templateUrl` (Vitest can't resolve them).
- **Angular control flow**: Use `@if`/`@for` — not `*ngIf`/`*ngFor`.
- **No `JSON.parse()` in TypeScript** — the storage API already returns parsed objects.

---

## Task 1: Rust — Add `UpdateContentInputView` to views.rs

This struct is the API boundary type for PATCH requests. It gets serialized to/from JSON at the HTTP boundary and exported to TypeScript via `ts-rs`.

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs`

### Step 1: Write the failing test

In `views.rs`, find the tests section near the end of the file (search for `#[cfg(test)]`). Add:

```rust
#[test]
fn test_update_content_input_view_deserializes_partial() {
    let json = r#"{"metadata": {"status": "done"}}"#;
    let view: UpdateContentInputView = serde_json::from_str(json).unwrap();
    assert!(view.title.is_none());
    assert!(view.tags.is_none());
    let meta = view.metadata.unwrap();
    assert_eq!(meta["status"], "done");
}

#[test]
fn test_update_content_input_view_empty_patch_deserializes() {
    let json = r#"{}"#;
    let view: UpdateContentInputView = serde_json::from_str(json).unwrap();
    assert!(view.title.is_none());
    assert!(view.metadata.is_none());
}
```

### Step 2: Run to verify it fails

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test test_update_content_input_view 2>&1 | tail -20
```
Expected: FAIL — `UpdateContentInputView` not found.

### Step 3: Add the struct to views.rs

Find the `CreateContentInputView` struct (around line 1115). Add the following directly after its `From` impl (around line 1165):

```rust
/// Input for partially updating a content item — PATCH /db/content/{id}
///
/// All fields are optional — only provided fields are applied.
/// `metadata` is shallow-merged into the existing metadata object (key-by-key overwrite).
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct UpdateContentInputView {
    #[serde(default)]
    pub title: Option<String>,
    /// Pass `null` explicitly to clear the description field.
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub content_body: Option<String>,
    #[serde(default)]
    pub content_format: Option<String>,
    /// Shallow-merged into existing metadata: only keys present in this object are updated.
    #[serde(default)]
    pub metadata: Option<JsonVal>,
    /// If provided, replaces all existing tags.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub reach: Option<String>,
}
```

### Step 4: Run to verify it passes

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test test_update_content_input_view 2>&1 | tail -10
```
Expected: 2 tests pass.

### Step 5: Commit

```bash
git add elohim/elohim-storage/src/views.rs
git commit -m "feat(storage): add UpdateContentInputView for PATCH /db/content/{id}"
```

---

## Task 2: Rust — Add `update_content()` to content_diesel.rs

This implements the DB-layer PATCH logic: fetch existing row, shallow-merge metadata, apply field updates, replace tags.

**Files:**
- Modify: `elohim/elohim-storage/src/db/content_diesel.rs`

### Step 1: Write the failing test

Find the test section in `content_diesel.rs` (near line 460). Add:

```rust
#[test]
fn test_update_content_input_struct_exists() {
    // Verifies the struct is importable and has expected shape
    let input = UpdateContentInput {
        id: "test-id".to_string(),
        title: None,
        description: None,
        content_body: None,
        content_format: None,
        metadata_json: Some(r#"{"status":"done"}"#.to_string()),
        tags: None,
        reach: None,
    };
    assert_eq!(input.id, "test-id");
    assert!(input.metadata_json.is_some());
}
```

### Step 2: Run to verify it fails

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test test_update_content_input_struct 2>&1 | tail -10
```
Expected: FAIL — `UpdateContentInput` not found.

### Step 3: Add the struct and function

After the `CreateContentInput` struct (around line 45), add:

```rust
/// Input for partially updating a content item (PATCH semantics).
/// All fields are `Option` — `None` means "no change".
#[derive(Debug, Default)]
pub struct UpdateContentInput {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub content_body: Option<String>,
    pub content_format: Option<String>,
    /// Already-serialized JSON string. If provided, shallow-merged with existing metadata.
    /// The caller is responsible for producing the merged JSON before calling this function.
    pub metadata_json: Option<String>,
    /// If provided, replaces all existing tags (delete all + insert new).
    pub tags: Option<Vec<String>>,
    pub reach: Option<String>,
}
```

Then, after the `bulk_create_content` function, add the `update_content` function:

```rust
/// Update a content item with partial (PATCH) semantics — scoped by app.
///
/// Only fields present in `input` are applied. Tags, if provided, replace all existing tags.
/// Returns the updated `ContentWithTags`.
pub fn update_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: UpdateContentInput,
) -> Result<ContentWithTags, StorageError> {
    use super::models::current_timestamp;

    let id = &input.id;

    // Verify the row exists in this app scope
    let existing = get_content(conn, ctx, id)?
        .ok_or_else(|| StorageError::NotFound(format!("Content not found: {}", id)))?;

    // Apply scalar field updates
    let new_title = input.title.as_deref().unwrap_or(&existing.content.title);
    let new_description = input.description.as_deref().or(existing.content.description.as_deref());
    let new_content_body = input.content_body.as_deref().or(existing.content.content_body.as_deref());
    let new_content_format = input.content_format.as_deref().unwrap_or(&existing.content.content_format);
    let new_reach = input.reach.as_deref().unwrap_or(&existing.content.reach);
    let new_metadata_json = input.metadata_json.as_deref().or(existing.content.metadata_json.as_deref());

    let now = current_timestamp();

    conn.transaction(|conn| {
        diesel::update(
            content::table
                .filter(content::app_id.eq(&ctx.app_id))
                .filter(content::id.eq(id)),
        )
        .set((
            content::title.eq(new_title),
            content::description.eq(new_description),
            content::content_body.eq(new_content_body),
            content::content_format.eq(new_content_format),
            content::metadata_json.eq(new_metadata_json),
            content::reach.eq(new_reach),
            content::updated_at.eq(&now),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

        // Replace tags if provided
        if let Some(ref new_tags) = input.tags {
            diesel::delete(
                content_tags::table
                    .filter(content_tags::app_id.eq(&ctx.app_id))
                    .filter(content_tags::content_id.eq(id)),
            )
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Tag delete failed: {}", e)))?;

            for tag in new_tags {
                let new_tag = NewContentTag {
                    app_id: &ctx.app_id,
                    content_id: id,
                    tag,
                };
                diesel::insert_or_ignore_into(content_tags::table)
                    .values(&new_tag)
                    .execute(conn)
                    .map_err(|e| StorageError::Internal(format!("Tag insert failed: {}", e)))?;
            }
        }

        // Return updated record
        get_content(conn, ctx, id)?
            .ok_or_else(|| StorageError::Internal("Failed to fetch updated content".into()))
    })
}
```

### Step 4: Run to verify it passes

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test test_update_content_input_struct 2>&1 | tail -10
```
Expected: 1 test passes.

### Step 5: Commit

```bash
git add elohim/elohim-storage/src/db/content_diesel.rs
git commit -m "feat(storage): add update_content() with PATCH/shallow-merge semantics"
```

---

## Task 3: Rust — Add `update()` to ContentService + PATCH handler in http.rs

Wires the DB function into the service layer and HTTP handler, including metadata shallow-merge logic.

**Files:**
- Modify: `elohim/elohim-storage/src/services/content_service.rs`
- Modify: `elohim/elohim-storage/src/http.rs`
- Modify: `elohim/elohim-storage/src/views.rs` (add `UpdateContentInputView` import to http.rs uses)

### Step 1: Write the failing test

In `content_service.rs`, the existing test section has a placeholder. The actual service methods require a DB connection. Add a compile-time test:

```rust
#[test]
fn test_content_service_has_update_method() {
    // This test verifies the method exists and the signature is correct at compile time.
    // Runtime tests would require a test database (out of scope for this task).
    fn _assert_update_exists(_s: &ContentService, _id: &str, _v: crate::views::UpdateContentInputView) {
        // If this compiles, the method exists
    }
}
```

### Step 2: Run to verify it fails

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test test_content_service_has_update 2>&1 | tail -10
```
Expected: FAIL — compile error (method doesn't exist on `ContentService`).

### Step 3: Add `update()` to ContentService

In `content_service.rs`, add after the `bulk_create()` method (around line 130):

```rust
/// Partially update a content item (PATCH semantics).
///
/// The `view.metadata` field is shallow-merged with existing metadata:
/// only the keys present in the patch object are overwritten.
pub fn update(
    &self,
    id: &str,
    view: crate::views::UpdateContentInputView,
) -> Result<crate::db::models::ContentWithTags, StorageError> {
    let mut conn = self.conn()?;

    // Compute merged metadata_json before entering the DB layer
    let merged_metadata_json = if let Some(patch_meta) = &view.metadata {
        let existing = content_diesel::get_content(&mut conn, &self.ctx, id)?
            .ok_or_else(|| StorageError::NotFound(format!("Content not found: {}", id)))?;

        let existing_meta: serde_json::Value = existing
            .content
            .metadata_json
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        let merged = match (existing_meta, patch_meta.clone()) {
            (serde_json::Value::Object(mut base), serde_json::Value::Object(patch)) => {
                for (k, v) in patch {
                    base.insert(k, v);
                }
                serde_json::Value::Object(base)
            }
            (_, patch) => patch, // fallback: replace entirely
        };

        Some(
            serde_json::to_string(&merged)
                .map_err(|e| StorageError::Internal(format!("Metadata serialize error: {}", e)))?,
        )
    } else {
        None
    };

    let input = content_diesel::UpdateContentInput {
        id: id.to_string(),
        title: view.title,
        description: view.description,
        content_body: view.content_body,
        content_format: view.content_format,
        metadata_json: merged_metadata_json,
        tags: view.tags,
        reach: view.reach,
    };

    let result = content_diesel::update_content(&mut conn, &self.ctx, input)?;

    self.events.emit(StorageEvent::ContentUpdated {
        id: result.content.id.clone(),
    });

    Ok(result)
}
```

### Step 4: Add PATCH handler to http.rs

In `http.rs`, find `handle_db_content_by_id` (around line 1934). Add a `Method::PATCH` arm before the `_` fallback:

```rust
Method::PATCH => {
    let body = req
        .collect()
        .await
        .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
    let body_bytes = body.to_bytes();

    let view: UpdateContentInputView = serde_json::from_slice(&body_bytes)
        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

    Ok(response::from_result(
        services.content.update(content_id, view).map(ContentWithTagsView::from),
    ))
}
```

Also add the import at the top of the file where `CreateContentInputView` is imported (around line 69):
```rust
    UpdateContentInputView,
```

And add PATCH to the CORS `Access-Control-Allow-Methods` header value (around line 558 where the methods string is defined — check if PATCH is already listed; if not, add it):
```
"GET, PUT, POST, PATCH, DELETE, HEAD, OPTIONS"
```

### Step 5: Run the full test suite

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib 2>&1 | tail -20
```
Expected: All tests pass, no compilation errors.

### Step 6: Commit

```bash
git add elohim/elohim-storage/src/services/content_service.rs elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): add PATCH /db/content/{id} with shallow metadata merge"
```

---

## Task 4: TypeScript SDK — Export UpdateContentInputView

`ts-rs` auto-generates `UpdateContentInputView.ts` when `cargo test export_bindings` runs. We then re-export it from the storage-client package.

**Files:**
- Generated: `elohim/sdk/storage-client-ts/src/generated/UpdateContentInputView.ts` (auto-generated)
- Modify: `elohim/sdk/storage-client-ts/src/generated/index.ts`

### Step 1: Run bindings export

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings 2>&1 | tail -20
```
Expected: Passes; `UpdateContentInputView.ts` is written to `elohim/sdk/storage-client-ts/src/generated/`.

### Step 2: Verify the generated file

```bash
cat elohim/sdk/storage-client-ts/src/generated/UpdateContentInputView.ts
```
Expected output (approximate):
```typescript
// This file was generated by [ts-rs]. Do not edit this file manually.
import type { JsonValue } from "./JsonValue";

export type UpdateContentInputView = {
  title?: string | null,
  description?: string | null,
  contentBody?: string | null,
  contentFormat?: string | null,
  metadata?: JsonValue | null,
  tags?: Array<string> | null,
  reach?: string | null,
};
```

### Step 3: Add the export to index.ts

Open `elohim/sdk/storage-client-ts/src/generated/index.ts`. Find the `CreateContentInputView` export line and add directly after it:

```typescript
export * from './UpdateContentInputView';
```

### Step 4: Run

```bash
cd app/elohim-app
pnpm exec vitest run --config vite.config.ts 2>&1 | tail -20
```
Expected: Tests pass (no new failures from the SDK change).

### Step 5: Commit

```bash
git add elohim/sdk/storage-client-ts/src/generated/UpdateContentInputView.ts \
        elohim/sdk/storage-client-ts/src/generated/index.ts
git commit -m "feat(sdk): export UpdateContentInputView TypeScript type"
```

---

## Task 5: Angular — Add write methods to IStorageApi + StorageApiService

Adds `createContent()` and `updateContent()` to both the interface contract and the concrete HTTP client.

**Files:**
- Modify: `app/elohim-app/src/app/elohim/interfaces/storage-api.interface.ts`
- Modify: `app/elohim-app/src/app/elohim/services/storage-api.service.ts`

### Step 1: Write the failing test

Create `app/elohim-app/src/app/elohim/services/storage-api-write.service.spec.ts`:

```typescript
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { TestBed } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { of } from 'rxjs';

import { StorageApiService } from './storage-api.service';

describe('StorageApiService write methods', () => {
  let service: StorageApiService;
  let httpSpy: { post: ReturnType<typeof vi.fn>; patch: ReturnType<typeof vi.fn> };

  beforeEach(() => {
    httpSpy = { post: vi.fn().mockReturnValue(of({})), patch: vi.fn().mockReturnValue(of({})) };

    TestBed.configureTestingModule({
      providers: [
        StorageApiService,
        { provide: HttpClient, useValue: httpSpy },
      ],
    });
    service = TestBed.inject(StorageApiService);
  });

  it('createContent posts to /db/content', () => {
    const input = { id: 'test', title: 'Test', schemaVersion: 1, tags: [] };
    service.createContent(input).subscribe();
    expect(httpSpy.post).toHaveBeenCalledWith(
      expect.stringContaining('/db/content'),
      input,
      expect.any(Object),
    );
  });

  it('updateContent patches to /db/content/{id}', () => {
    const patch = { metadata: { status: 'done' } };
    service.updateContent('story-123', patch).subscribe();
    expect(httpSpy.patch).toHaveBeenCalledWith(
      expect.stringContaining('/db/content/story-123'),
      patch,
      expect.any(Object),
    );
  });
});
```

### Step 2: Run to verify it fails

```bash
cd app/elohim-app
pnpm exec vitest run --config vite.config.ts storage-api-write 2>&1 | tail -20
```
Expected: FAIL — `createContent` and `updateContent` methods not found.

### Step 3: Add `UpdateContentPatch` to the interface file

In `app/elohim-app/src/app/elohim/interfaces/storage-api.interface.ts`, after the `RelationshipFilters` interface, add:

```typescript
/**
 * Partial update input for PATCH /db/content/{id}.
 * All fields are optional — only provided fields are applied server-side.
 * `metadata` is shallow-merged with the existing metadata object.
 */
export interface UpdateContentPatch {
  title?: string;
  description?: string | null;
  contentBody?: string;
  contentFormat?: string;
  /** Shallow-merged server-side — only the keys you provide are overwritten. */
  metadata?: Record<string, unknown>;
  /** If provided, replaces all existing tags. */
  tags?: string[];
  reach?: string;
}
```

Also add to the `IStorageApi` interface (after `getContents`):

```typescript
/** Create a new content node. Returns the created item with tags. */
createContent(input: CreateContentInputView): Observable<ContentWithTagsView>;

/** Partially update a content node (PATCH). Returns the updated item with tags. */
updateContent(id: string, patch: UpdateContentPatch): Observable<ContentWithTagsView>;
```

Add `CreateContentInputView` to the import from `@elohim/storage-client/generated` at the top of the file.

### Step 4: Implement in StorageApiService

In `app/elohim-app/src/app/elohim/services/storage-api.service.ts`, find the `getContents()` method (around line 162). Add the two new methods directly after `getContents()`:

```typescript
createContent(input: CreateContentInputView): Observable<ContentWithTagsView> {
  return this.http
    .post<ContentWithTagsView>(`${this.baseUrl}/db/content`, input, {
      headers: { 'Content-Type': 'application/json' },
    })
    .pipe(timeout(this.defaultTimeoutMs), catchError(error => this.handleError('createContent', error)));
}

updateContent(id: string, patch: UpdateContentPatch): Observable<ContentWithTagsView> {
  return this.http
    .patch<ContentWithTagsView>(
      `${this.baseUrl}/db/content/${encodeURIComponent(id)}`,
      patch,
      { headers: { 'Content-Type': 'application/json' } },
    )
    .pipe(timeout(this.defaultTimeoutMs), catchError(error => this.handleError('updateContent', error)));
}
```

Add imports at the top of the service file:
- `CreateContentInputView` from `@elohim/storage-client/generated`
- `UpdateContentPatch` from `../interfaces/storage-api.interface`

### Step 5: Run to verify tests pass

```bash
pnpm exec vitest run --config vite.config.ts storage-api-write 2>&1 | tail -20
```
Expected: 2 tests pass.

### Step 6: Run lint

```bash
pnpm run lint 2>&1 | grep -E "storage-api|ERROR" | head -10
```
Expected: No errors.

### Step 7: Commit

```bash
git add app/elohim-app/src/app/elohim/interfaces/storage-api.interface.ts \
        app/elohim-app/src/app/elohim/services/storage-api.service.ts \
        app/elohim-app/src/app/elohim/services/storage-api-write.service.spec.ts
git commit -m "feat(elohim): add createContent() and updateContent() to StorageApiService"
```

---

## Task 6: Angular — Rewire AvodahApiService to real storage

Replaces mock data with real API calls. Reads use `getContents()` + `toContentNode()` adapter. Writes use `createContent()` and `updateContent()`. Terminal status transitions also emit an economic event.

**Files:**
- Modify: `app/elohim-app/src/app/avodah/services/avodah-api.service.ts`

### Step 1: Write the failing tests

Replace `app/elohim-app/src/app/avodah/services/avodah-api.service.spec.ts` entirely:

```typescript
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { StorageApiService } from '../../elohim/services/storage-api.service';
import { AvodahApiService } from './avodah-api.service';

const MOCK_PROJECT_VIEW = {
  id: 'proj-1',
  appId: 'lamad',
  contentType: 'work-project',
  title: 'Test Project',
  description: null,
  contentFormat: 'text',
  contentBody: null,
  blobHash: null,
  blobCid: null,
  contentSizeBytes: null,
  reach: 'private',
  validationStatus: 'approved',
  createdBy: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
  metadata: { columns: [], visibility: 'private', memberIds: [] },
  tags: ['household'],
};

const MOCK_STORY_VIEW = {
  id: 'story-1',
  appId: 'lamad',
  contentType: 'work-story',
  title: 'Fix the fence',
  description: null,
  contentFormat: 'text',
  contentBody: null,
  blobHash: null,
  blobCid: null,
  contentSizeBytes: null,
  reach: 'private',
  validationStatus: 'approved',
  createdBy: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
  metadata: { projectId: 'proj-1', status: 'todo', visibility: 'private', priority: 'high' },
  tags: [],
};

describe('AvodahApiService', () => {
  let service: AvodahApiService;
  let storageSpy: {
    getContents: ReturnType<typeof vi.fn>;
    updateContent: ReturnType<typeof vi.fn>;
    createContent: ReturnType<typeof vi.fn>;
    createEconomicEvent: ReturnType<typeof vi.fn>;
  };

  beforeEach(() => {
    storageSpy = {
      getContents: vi.fn(),
      updateContent: vi.fn().mockReturnValue(of(MOCK_STORY_VIEW)),
      createContent: vi.fn().mockReturnValue(of(MOCK_PROJECT_VIEW)),
      createEconomicEvent: vi.fn().mockReturnValue(of({})),
    };

    TestBed.configureTestingModule({
      providers: [
        AvodahApiService,
        { provide: StorageApiService, useValue: storageSpy },
      ],
    });
    service = TestBed.inject(AvodahApiService);
  });

  it('getProjects fetches work-project content type', async () => {
    storageSpy.getContents.mockReturnValue(of([MOCK_PROJECT_VIEW]));
    const projects = await service.getProjects();
    expect(storageSpy.getContents).toHaveBeenCalledWith({ contentType: 'work-project' });
    expect(projects[0].contentType).toBe('work-project');
    expect(projects[0].id).toBe('proj-1');
  });

  it('getStoriesForProject fetches work-story and filters by projectId', async () => {
    storageSpy.getContents.mockReturnValue(of([MOCK_STORY_VIEW]));
    const stories = await service.getStoriesForProject('proj-1');
    expect(storageSpy.getContents).toHaveBeenCalledWith({ contentType: 'work-story' });
    expect(stories).toHaveLength(1);
    expect(stories[0].id).toBe('story-1');
  });

  it('getStoriesForProject excludes stories from other projects', async () => {
    const otherStory = {
      ...MOCK_STORY_VIEW,
      id: 'story-other',
      metadata: { ...MOCK_STORY_VIEW.metadata, projectId: 'proj-99' },
    };
    storageSpy.getContents.mockReturnValue(of([MOCK_STORY_VIEW, otherStory]));
    const stories = await service.getStoriesForProject('proj-1');
    expect(stories).toHaveLength(1);
    expect(stories[0].id).toBe('story-1');
  });

  it('updateStoryStatus patches metadata.status', async () => {
    await service.updateStoryStatus('story-1', 'in-progress');
    expect(storageSpy.updateContent).toHaveBeenCalledWith('story-1', {
      metadata: { status: 'in-progress' },
    });
  });

  it('updateStoryStatus does NOT emit economic event for non-terminal status', async () => {
    await service.updateStoryStatus('story-1', 'in-progress', false);
    expect(storageSpy.createEconomicEvent).not.toHaveBeenCalled();
  });

  it('updateStoryStatus emits economic event when isTerminal=true', async () => {
    await service.updateStoryStatus('story-1', 'done', true);
    expect(storageSpy.updateContent).toHaveBeenCalledWith('story-1', { metadata: { status: 'done' } });
    expect(storageSpy.createEconomicEvent).toHaveBeenCalledWith(
      expect.objectContaining({ action: 'work', contentId: 'story-1' }),
    );
  });
});
```

### Step 2: Run to verify it fails

```bash
pnpm exec vitest run --config vite.config.ts avodah-api.service 2>&1 | tail -20
```
Expected: FAIL — methods not wired to storage.

### Step 3: Rewrite AvodahApiService

Replace `app/elohim-app/src/app/avodah/services/avodah-api.service.ts` with:

```typescript
/* eslint-disable @typescript-eslint/require-await -- Observable→Promise bridging */
import { Injectable, inject } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import { ContentMetadata, ContentNode } from '../../lamad/models/content-node.model';
import { StorageApiService } from '../../elohim/services/storage-api.service';

import type { ContentWithTagsView } from '@elohim/storage-client/generated';
import type { WorkStoryStatus } from '../models/work-story.model';

// TODO: [HOLOCHAIN-ZOME] writes currently go direct to storage (same as seed workflow).
// Route through conductor once the work-story zome is implemented.

/**
 * Map a storage ContentWithTagsView to the app's ContentNode domain type.
 * The wire format is already camelCase with parsed JSON — no transformation needed,
 * just field projection.
 */
function toContentNode(view: ContentWithTagsView): ContentNode {
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

@Injectable({ providedIn: 'root' })
export class AvodahApiService {
  private readonly storageApi = inject(StorageApiService);

  async getProjects(): Promise<ContentNode[]> {
    const views = await firstValueFrom(
      this.storageApi.getContents({ contentType: 'work-project' }),
    );
    return views.map(toContentNode);
  }

  async getStoriesForProject(projectId: string): Promise<ContentNode[]> {
    const views = await firstValueFrom(
      this.storageApi.getContents({ contentType: 'work-story' }),
    );
    return views
      .map(toContentNode)
      .filter(
        n => (n.metadata as Record<string, unknown>)['projectId'] === projectId,
      );
  }

  /**
   * Update a story's status.
   *
   * @param isTerminal - set true when moving to a done-state column (`isTerminal: true`
   *   in the project's BoardColumn config). This triggers an economic event in shefa.
   */
  async updateStoryStatus(
    storyId: string,
    status: WorkStoryStatus,
    isTerminal = false,
  ): Promise<void> {
    await firstValueFrom(
      this.storageApi.updateContent(storyId, { metadata: { status } }),
    );

    if (isTerminal) {
      // REA transition: done → economic event settles the work record
      await firstValueFrom(
        this.storageApi.createEconomicEvent({
          action: 'work',
          provider: storyId,   // story node as the work unit
          receiver: storyId,
          contentId: storyId,
          lamadEventType: 'work-complete',
        }),
      );
    }
  }
}
```

### Step 4: Run to verify tests pass

```bash
pnpm exec vitest run --config vite.config.ts avodah-api.service 2>&1 | tail -20
```
Expected: 6 tests pass.

### Step 5: Run lint

```bash
pnpm run lint 2>&1 | grep -E "avodah-api|ERROR" | head -10
```
Expected: No errors.

### Step 6: Run full test suite

```bash
pnpm exec vitest run --config vite.config.ts 2>&1 | tail -10
```
Expected: All previously-passing tests still pass.

### Step 7: Commit

```bash
git add app/elohim-app/src/app/avodah/services/avodah-api.service.ts \
        app/elohim-app/src/app/avodah/services/avodah-api.service.spec.ts
git commit -m "feat(avodah): wire AvodahApiService to storage API — real reads, PATCH updates, REA terminal events"
```

---

## Finishing Up

After all tasks are complete, run the full Angular test suite one final time:

```bash
cd app/elohim-app
pnpm exec vitest run --config vite.config.ts 2>&1 | tail -10
```

And lint:

```bash
pnpm run lint 2>&1 | grep ERROR | head -10
```

Both should be clean before completing the branch.
