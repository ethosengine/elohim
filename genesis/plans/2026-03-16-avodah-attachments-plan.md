# Avodah Attachments — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Allow work-stories to have attachments — links to existing EPR ContentNodes via `ATTACHED_TO` relationships, with UI in the story detail view.

**Architecture:** New `ATTACHED_TO` relationship type in the existing enum. Three service methods on `AvodahApiService` wrapping `StorageApiService` calls (`getRelationships`, `createRelationship`, new `deleteRelationship`). Attachments section added to `StoryDetailComponent` with list, add (by content ID), and remove.

**Tech Stack:** Angular 19, standalone components, inline templates, `inject()` DI, Vitest

**Design doc:** `genesis/plans/2026-03-16-avodah-attachments-design.md`

**Critical rules:**
- **`inject()` only** — never constructor injection
- **Inline templates only** — never `templateUrl`
- **`@if`/`@for`** — not `*ngIf`/`*ngFor`
- **Import order:** builtin → external → `@app/*` → `@elohim/*`
- **Test command:** `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts <pattern>`

---

## Task 1: Add `ATTACHED_TO` to ContentRelationshipType + `deleteRelationship` to StorageApiService

### Files
- Modify: `app/elohim-app/src/app/lamad/models/content-node.model.ts`
- Modify: `app/elohim-app/src/app/elohim/services/storage-api.service.ts`
- Modify: `app/elohim-app/src/app/elohim/interfaces/storage-api.interface.ts`

### Step 1: Add ATTACHED_TO to ContentRelationshipType enum

In `app/elohim-app/src/app/lamad/models/content-node.model.ts`, find the `ContentRelationshipType` enum (around line 720). Add after `FOLLOWS`:

```typescript
/** Attachment link (story → attached content) */
ATTACHED_TO = 'ATTACHED_TO',
```

### Step 2: Add deleteRelationship to IStorageApi interface

In `app/elohim-app/src/app/elohim/interfaces/storage-api.interface.ts`, add to the `IStorageApi` interface after `updateContent`:

```typescript
/** Delete a relationship by ID. */
deleteRelationship(id: string): Observable<void>;
```

### Step 3: Implement deleteRelationship in StorageApiService

In `app/elohim-app/src/app/elohim/services/storage-api.service.ts`, add after the `createRelationship` method:

```typescript
deleteRelationship(id: string): Observable<void> {
  return this.http
    .delete<void>(`${this.baseUrl}/db/relationships/${encodeURIComponent(id)}`)
    .pipe(timeout(this.defaultTimeoutMs), catchError(error => this.handleError('deleteRelationship', error)));
}
```

### Step 4: Run lint and tests

```bash
cd app/elohim-app
pnpm run lint 2>&1 | grep ERROR | head -10
pnpm exec vitest run --config vite.config.ts avodah 2>&1 | tail -10
```
Expected: Clean.

### Step 5: Commit

```bash
git add app/elohim-app/src/app/lamad/models/content-node.model.ts \
        app/elohim-app/src/app/elohim/services/storage-api.service.ts \
        app/elohim-app/src/app/elohim/interfaces/storage-api.interface.ts
git commit -m "feat(elohim): add ATTACHED_TO relationship type + deleteRelationship to StorageApiService"
```

---

## Task 2: Add attachment methods to AvodahApiService

### Files
- Modify: `app/elohim-app/src/app/avodah/services/avodah-api.service.ts`
- Modify: `app/elohim-app/src/app/avodah/services/avodah-api.service.spec.ts`

### Step 1: Write failing tests

Add to the spec's `describe` block. First, add `RelationshipView` mock data at the top of the file (after `MOCK_STORY_VIEW`):

```typescript
const MOCK_RELATIONSHIP = {
  id: 'rel-1',
  sourceId: 'story-1',
  targetId: 'concept-abc',
  relationshipType: 'ATTACHED_TO',
  confidence: 1,
  inferenceSource: 'author',
  metadata: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};
```

Add `getRelationships` and `deleteRelationship` to the `storageSpy` setup:

```typescript
getRelationships: vi.fn(),
deleteRelationship: vi.fn().mockReturnValue(of(undefined)),
```

Add tests:

```typescript
it('getAttachments fetches ATTACHED_TO relationships and resolves content', async () => {
  storageSpy.getRelationships.mockReturnValue(of([MOCK_RELATIONSHIP]));
  storageSpy.getContents.mockReturnValue(of([{ ...MOCK_STORY_VIEW, id: 'concept-abc', contentType: 'concept', title: 'Test Concept' }]));
  const attachments = await service.getAttachments('story-1');
  expect(storageSpy.getRelationships).toHaveBeenCalledWith({
    sourceId: 'story-1',
    relationshipType: 'ATTACHED_TO',
  });
  expect(attachments).toHaveLength(1);
  expect(attachments[0].content.id).toBe('concept-abc');
});

it('attachContent creates ATTACHED_TO relationship', async () => {
  storageSpy.createRelationship.mockReturnValue(of(MOCK_RELATIONSHIP));
  await service.attachContent('story-1', 'concept-abc');
  expect(storageSpy.createRelationship).toHaveBeenCalledWith(
    expect.objectContaining({
      sourceId: 'story-1',
      targetId: 'concept-abc',
      relationshipType: 'ATTACHED_TO',
    }),
  );
});

it('detachContent deletes relationship', async () => {
  await service.detachContent('rel-1');
  expect(storageSpy.deleteRelationship).toHaveBeenCalledWith('rel-1');
});
```

Also add `createRelationship` to the `storageSpy` if not already there:
```typescript
createRelationship: vi.fn().mockReturnValue(of(MOCK_RELATIONSHIP)),
```

### Step 2: Run to verify tests fail

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts avodah-api.service
```
Expected: FAIL — methods don't exist.

### Step 3: Implement

Add to `AvodahApiService`. First add the needed imports:

```typescript
import { ContentRelationshipType, ContentNode } from '../../lamad/models/content-node.model';
```

Note: `ContentRelationshipType` may already be importable from where `ContentNode` is imported. Check the existing import and add to it.

Then add these methods to the class:

```typescript
/** Attachment = ATTACHED_TO relationship from story to content */

async getAttachments(storyId: string): Promise<{ relationshipId: string; content: ContentNode }[]> {
  const relationships = await firstValueFrom(
    this.storageApi.getRelationships({
      sourceId: storyId,
      relationshipType: ContentRelationshipType.ATTACHED_TO,
    }),
  );

  if (relationships.length === 0) return [];

  // Resolve target content nodes
  const targetIds = relationships.map(r => r.targetId);
  const allContent = await firstValueFrom(
    this.storageApi.getContents({}),
  );
  const contentMap = new Map(allContent.map(v => [v.id, toContentNode(v)]));

  return relationships
    .filter(r => contentMap.has(r.targetId))
    .map(r => ({
      relationshipId: r.id,
      content: contentMap.get(r.targetId)!,
    }));
}

async attachContent(storyId: string, contentId: string): Promise<void> {
  await firstValueFrom(
    this.storageApi.createRelationship({
      sourceId: storyId,
      targetId: contentId,
      relationshipType: ContentRelationshipType.ATTACHED_TO,
      confidence: 1,
      inferenceSource: 'author',
    }),
  );
}

async detachContent(relationshipId: string): Promise<void> {
  await firstValueFrom(this.storageApi.deleteRelationship(relationshipId));
}
```

Note: `getAttachments` fetches all content to resolve targets. This is a pragmatic MVP approach — a future optimization would batch-fetch by IDs.

### Step 4: Run to verify tests pass

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts avodah-api.service
```
Expected: All tests pass.

### Step 5: Run lint

```bash
cd app/elohim-app && pnpm run lint 2>&1 | grep ERROR | head -10
```

### Step 6: Commit

```bash
git add app/elohim-app/src/app/avodah/services/avodah-api.service.ts \
        app/elohim-app/src/app/avodah/services/avodah-api.service.spec.ts
git commit -m "feat(avodah): add getAttachments, attachContent, detachContent to AvodahApiService"
```

---

## Task 3: Add attachments section to StoryDetailComponent

### Files
- Modify: `app/elohim-app/src/app/avodah/components/story-detail/story-detail.component.ts`

### Step 1: Read the file first

Read the full `story-detail.component.ts` to understand the current template and class structure.

### Step 2: Add template section

Find the last `</div>` before `</main>` (after the "Attestation Gates" section). Add this attachments section before `</main>`:

```html
<div class="section">
  <label>Attachments</label>
  @if (attachments.length > 0) {
    <ul class="attachment-list">
      @for (att of attachments; track att.relationshipId) {
        <li class="attachment-item">
          <span class="att-icon">{{ contentIcon(att.content.contentType) }}</span>
          <span class="att-title">{{ att.content.title }}</span>
          <button
            class="att-remove"
            (click)="removeAttachment(att.relationshipId)"
            data-testid="remove-attachment"
            aria-label="Remove attachment"
          >✕</button>
        </li>
      }
    </ul>
  } @else if (!addingAttachment) {
    <span class="empty-hint">No attachments</span>
  }
  @if (addingAttachment) {
    <input
      class="attach-input"
      placeholder="Content ID…"
      data-testid="attach-input"
      (keydown.enter)="submitAttachment($event)"
      (keydown.escape)="addingAttachment = false"
      (blur)="addingAttachment = false"
    />
  } @else {
    <button
      class="attach-btn"
      data-testid="add-attachment-btn"
      (click)="addingAttachment = true"
    >+ Attach content</button>
  }
</div>
```

### Step 3: Add styles

Add to the styles array:

```css
.attachment-list { list-style: none; padding: 0; margin: 0 0 0.5rem; }
.attachment-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0.5rem;
  border-radius: 6px;
  font-size: 0.85rem;
}
.attachment-item:hover { background: rgba(99, 102, 241, 0.06); }
.att-icon { font-size: 1rem; }
.att-title { flex: 1; color: var(--lamad-text-secondary, #e2e8f0); }
.att-remove {
  background: none;
  border: none;
  color: var(--lamad-text-muted, #64748b);
  cursor: pointer;
  font-size: 0.75rem;
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
}
.att-remove:hover { background: rgba(239, 68, 68, 0.15); color: #f87171; }
.attach-input {
  width: 100%;
  background: rgba(15, 15, 26, 0.8);
  border: 1px solid var(--lamad-accent-primary, #6366f1);
  border-radius: 6px;
  padding: 0.4rem 0.75rem;
  font-size: 0.8rem;
  color: var(--lamad-text-secondary, #e2e8f0);
  outline: none;
  box-sizing: border-box;
}
.attach-btn {
  background: none;
  border: 1px dashed rgba(99, 102, 241, 0.25);
  border-radius: 6px;
  color: var(--lamad-text-muted, #64748b);
  padding: 0.375rem 0.75rem;
  font-size: 0.8rem;
  cursor: pointer;
  width: 100%;
}
.attach-btn:hover {
  border-color: var(--lamad-accent-primary, #6366f1);
  color: var(--lamad-accent-primary, #6366f1);
}
```

### Step 4: Add class properties and methods

Add an import for `CONTENT_TYPE_ICONS`:
```typescript
import { CONTENT_TYPE_ICONS } from '@app/lamad/utils/content-icons';
```

Add to the class:

```typescript
attachments: { relationshipId: string; content: ContentNode }[] = [];
addingAttachment = false;

contentIcon(contentType: string): string {
  return (CONTENT_TYPE_ICONS as Record<string, string>)[contentType] ?? '📄';
}

async submitAttachment(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement;
  const contentId = input.value.trim();
  if (!contentId || !this.story) return;

  await this.api.attachContent(this.story.id, contentId);
  this.attachments = await this.api.getAttachments(this.story.id);
  this.addingAttachment = false;
}

async removeAttachment(relationshipId: string): Promise<void> {
  await this.api.detachContent(relationshipId);
  this.attachments = this.attachments.filter(a => a.relationshipId !== relationshipId);
}
```

Update the `load()` method to also fetch attachments — add at the end of the `load()` method, after `this.story = stories.find(...)`:

```typescript
if (this.story) {
  this.attachments = await this.api.getAttachments(this.story.id);
}
```

### Step 5: Run lint and tests

```bash
cd app/elohim-app
pnpm run lint 2>&1 | grep ERROR | head -10
pnpm exec vitest run --config vite.config.ts avodah 2>&1 | tail -10
```
Expected: Clean.

### Step 6: Commit

```bash
git add app/elohim-app/src/app/avodah/components/story-detail/story-detail.component.ts
git commit -m "feat(avodah): attachments section in story detail — list, add by ID, remove"
```

---

## Finishing Up

Run the full avodah test suite:

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts avodah 2>&1 | tail -10
```

And lint:

```bash
cd app/elohim-app && pnpm run lint 2>&1 | grep ERROR | head -10
```

Both should be clean.
