# Avodah UI Wiring — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the avodah kanban board interactive — drag-and-drop cards between columns, inline story creation, and a full-page story detail view.

**Architecture:** Native HTML5 drag-and-drop (no CDK dependency). Story detail as a full-page route consistent with lamad's content viewer pattern. Inline creation via Taiga-style "type and Enter" UX. All mutations go through `AvodahApiService` → `StorageApiService`.

**Tech Stack:** Angular 19, standalone components, inline templates, `inject()` DI, Vitest, native HTML5 DnD API

**Design doc:** `genesis/plans/2026-03-15-avodah-ui-wiring-design.md`

**Critical rules:**
- **`inject()` only** — never constructor injection (esbuild strips metadata)
- **Inline templates only** — never `templateUrl` (Vitest can't resolve them)
- **`@if`/`@for`** — not `*ngIf`/`*ngFor` (Angular 17+ control flow)
- **Import order:** builtin → external → `@app/*` → `@elohim/*`
- **Test command:** `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts <pattern>`

---

## Task 1: Add `createStory()` to AvodahApiService

Adds the method needed by both inline creation UIs (board and backlog).

### Files
- Modify: `app/elohim-app/src/app/avodah/services/avodah-api.service.ts`
- Modify: `app/elohim-app/src/app/avodah/services/avodah-api.service.spec.ts`

### Step 1: Write the failing test

Add to the existing spec file's `describe` block:

```typescript
it('createStory creates a work-story via storageApi', async () => {
  storageSpy.createContent.mockReturnValue(of(MOCK_STORY_VIEW));
  const result = await service.createStory('proj-1', 'New task', 'todo');
  expect(storageSpy.createContent).toHaveBeenCalledWith(
    expect.objectContaining({
      title: 'New task',
      contentType: 'work-story',
      metadata: expect.objectContaining({ projectId: 'proj-1', status: 'todo' }),
    }),
  );
  expect(result.title).toBe('Fix the fence');
});
```

### Step 2: Run to verify it fails

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts avodah-api.service
```
Expected: FAIL — `createStory` not found on service.

### Step 3: Implement createStory

Add to `AvodahApiService` class:

```typescript
async createStory(
  projectId: string,
  title: string,
  status: WorkStoryStatus = 'backlog',
): Promise<ContentNode> {
  const id = `story-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  const view = await firstValueFrom(
    this.storageApi.createContent({
      id,
      title,
      schemaVersion: 1,
      contentType: 'work-story',
      contentFormat: 'text',
      reach: 'private',
      metadata: {
        projectId,
        status,
        visibility: 'private',
        priority: 'medium',
      },
      tags: [],
    }),
  );
  return toContentNode(view);
}
```

Add `CreateContentInputView` to the import from `@elohim/storage-client/generated` if not already there. Note: the `CreateContentInputView` has `metadata: Option<JsonVal>` which accepts plain objects.

### Step 4: Run to verify it passes

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts avodah-api.service
```
Expected: All tests pass (including the new one).

### Step 5: Commit

```bash
git add app/elohim-app/src/app/avodah/services/avodah-api.service.ts \
        app/elohim-app/src/app/avodah/services/avodah-api.service.spec.ts
git commit -m "feat(avodah): add createStory() to AvodahApiService"
```

---

## Task 2: Drag-and-Drop on Kanban Board

Adds native HTML5 drag-and-drop to `ProjectBoardComponent`. Cards can be dragged between columns. On drop, the story's status is updated via the API.

### Files
- Modify: `app/elohim-app/src/app/avodah/components/project-board/project-board.component.ts`

### Step 1: Implement drag-and-drop

**Template changes — replace the `col-stories` and `add-story` sections:**

Replace the current `@for` loop for story cards in the template:

```html
@for (story of storiesInColumn(col.id); track story.id) {
  <app-story-card [story]="story" />
}
```

With:

```html
@for (story of storiesInColumn(col.id); track story.id) {
  <app-story-card
    [story]="story"
    draggable="true"
    [attr.data-story-id]="story.id"
    (dragstart)="onDragStart($event, story)"
    (cardClick)="openStory(story)"
  />
}
```

Replace the current column `div.col-stories`:

```html
<div class="col-stories">
```

With:

```html
<div
  class="col-stories"
  [attr.data-column-id]="col.id"
  (dragover)="onDragOver($event)"
  (dragleave)="onDragLeave($event)"
  (drop)="onDrop($event, col)"
>
```

**Add imports:**

Add `Router` to the import from `@angular/router`:
```typescript
import { RouterLink, ActivatedRoute, Router } from '@angular/router';
```

**Add class properties and methods:**

```typescript
private readonly router = inject(Router);
private draggedStoryId: string | null = null;

onDragStart(event: DragEvent, story: ContentNode): void {
  this.draggedStoryId = story.id;
  event.dataTransfer?.setData('text/plain', story.id);
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move';
  }
}

onDragOver(event: DragEvent): void {
  event.preventDefault();
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'move';
  }
  (event.currentTarget as HTMLElement).classList.add('drop-target');
}

onDragLeave(event: DragEvent): void {
  (event.currentTarget as HTMLElement).classList.remove('drop-target');
}

onDrop(event: DragEvent, column: BoardColumn): void {
  event.preventDefault();
  (event.currentTarget as HTMLElement).classList.remove('drop-target');

  const storyId = this.draggedStoryId;
  this.draggedStoryId = null;
  if (!storyId) return;

  // Optimistic: move card in local array
  const story = this.stories.find(s => s.id === storyId);
  if (story) {
    (story.metadata as Record<string, unknown>)['status'] = column.id;
  }

  // Persist via API
  void this.api.updateStoryStatus(storyId, column.id as WorkStoryStatus, column.isTerminal ?? false);
}

openStory(story: ContentNode): void {
  const projectId = this.route.snapshot.params['id'] as string;
  void this.router.navigate(['/avodah/projects', projectId, 'stories', story.id]);
}
```

Add import for `WorkStoryStatus`:
```typescript
import { parseWorkStoryMeta, type WorkStoryStatus } from '../../models/work-story.model';
```

**Style additions:**

```css
.col-stories.drop-target {
  background: rgba(99, 102, 241, 0.08);
  border-radius: 6px;
  outline: 2px dashed rgba(99, 102, 241, 0.3);
}

app-story-card[draggable='true'] {
  cursor: grab;
}

app-story-card[draggable='true']:active {
  cursor: grabbing;
  opacity: 0.6;
}
```

### Step 2: Run lint and existing tests

```bash
cd app/elohim-app && pnpm run lint 2>&1 | grep -E "project-board|ERROR" | head -10
pnpm exec vitest run --config vite.config.ts avodah 2>&1 | tail -10
```
Expected: No new errors, existing avodah tests pass.

### Step 3: Commit

```bash
git add app/elohim-app/src/app/avodah/components/project-board/project-board.component.ts
git commit -m "feat(avodah): add drag-and-drop between kanban columns"
```

---

## Task 3: Inline Story Creation on Board

Clicking "+ Add story" at the bottom of a column reveals a text input. Press Enter to create the story with `status: columnId`.

### Files
- Modify: `app/elohim-app/src/app/avodah/components/project-board/project-board.component.ts`

### Step 1: Implement inline creation

**Template — replace the `add-story` div:**

```html
<div class="add-story">+ Add story</div>
```

With:

```html
@if (addingInColumn === col.id) {
  <input
    class="add-story-input"
    placeholder="Story title…"
    data-testid="add-story-input"
    (keydown.enter)="submitNewStory($event, col.id)"
    (keydown.escape)="addingInColumn = null"
    (blur)="addingInColumn = null"
    autofocus
  />
} @else {
  <div
    class="add-story"
    data-testid="add-story-btn"
    (click)="addingInColumn = col.id"
  >+ Add story</div>
}
```

**Add class property and method:**

```typescript
addingInColumn: string | null = null;

async submitNewStory(event: Event, columnId: string): Promise<void> {
  const input = event.target as HTMLInputElement;
  const title = input.value.trim();
  if (!title) return;

  const projectId = this.route.snapshot.params['id'] as string;
  const newStory = await this.api.createStory(
    projectId,
    title,
    columnId as WorkStoryStatus,
  );
  this.stories = [...this.stories, newStory];
  this.addingInColumn = null;
}
```

**Style additions:**

```css
.add-story-input {
  width: 100%;
  padding: 0.5rem;
  font-size: 0.8rem;
  border: 1px solid var(--lamad-accent-primary, #6366f1);
  border-radius: 6px;
  background: rgba(15, 15, 26, 0.8);
  color: var(--lamad-text-secondary, #e2e8f0);
  margin-top: 0.5rem;
  outline: none;
  box-sizing: border-box;
}
```

### Step 2: Run lint and tests

```bash
cd app/elohim-app && pnpm run lint 2>&1 | grep -E "project-board|ERROR" | head -10
pnpm exec vitest run --config vite.config.ts avodah 2>&1 | tail -10
```
Expected: Clean.

### Step 3: Commit

```bash
git add app/elohim-app/src/app/avodah/components/project-board/project-board.component.ts
git commit -m "feat(avodah): inline story creation on kanban board columns"
```

---

## Task 4: Inline Story Creation on Backlog

"+ New Story" button reveals an inline row with title input at the top of the table.

### Files
- Modify: `app/elohim-app/src/app/avodah/components/project-backlog/project-backlog.component.ts`

### Step 1: Implement

**Template — replace the `btn-new` button:**

```html
<button class="btn-new" data-testid="btn-new-story">+ New Story</button>
```

With:

```html
@if (addingStory) {
  <input
    class="inline-title-input"
    placeholder="Story title…"
    data-testid="new-story-input"
    (keydown.enter)="submitNewStory($event)"
    (keydown.escape)="addingStory = false"
    (blur)="addingStory = false"
    autofocus
  />
} @else {
  <button class="btn-new" data-testid="btn-new-story" (click)="addingStory = true">
    + New Story
  </button>
}
```

**Add to backlog table rows — make rows clickable.** Replace:

```html
<tr data-testid="backlog-row">
```

With:

```html
<tr
  data-testid="backlog-row"
  class="clickable-row"
  (click)="openStory(story)"
  (keydown.enter)="openStory(story)"
  tabindex="0"
  role="button"
>
```

**Add imports:**

```typescript
import { RouterLink, ActivatedRoute, Router } from '@angular/router';
```

**Add class properties and methods:**

```typescript
private readonly router = inject(Router);
addingStory = false;

async submitNewStory(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement;
  const title = input.value.trim();
  if (!title) return;

  const newStory = await this.api.createStory(this.projectId, title, 'backlog');
  this.stories = [...this.stories, newStory];
  this.addingStory = false;
}

openStory(story: ContentNode): void {
  void this.router.navigate(['/avodah/projects', this.projectId, 'stories', story.id]);
}
```

Add import for `WorkStoryStatus`:
```typescript
import { parseWorkStoryMeta, type WorkStoryStatus } from '../../models/work-story.model';
```

**Style additions:**

```css
.inline-title-input {
  background: rgba(15, 15, 26, 0.8);
  border: 1px solid var(--lamad-accent-primary, #6366f1);
  border-radius: 6px;
  padding: 0.4rem 0.75rem;
  font-size: 0.8rem;
  color: var(--lamad-text-secondary, #e2e8f0);
  outline: none;
  min-width: 200px;
}
.clickable-row { cursor: pointer; }
.clickable-row:hover td { background: rgba(99, 102, 241, 0.06); }
```

### Step 2: Run lint and tests

```bash
cd app/elohim-app && pnpm run lint 2>&1 | grep -E "backlog|ERROR" | head -10
pnpm exec vitest run --config vite.config.ts avodah 2>&1 | tail -10
```
Expected: Clean.

### Step 3: Commit

```bash
git add app/elohim-app/src/app/avodah/components/project-backlog/project-backlog.component.ts
git commit -m "feat(avodah): inline story creation + clickable rows in backlog"
```

---

## Task 5: Story Detail Component

Full-page view for a single story. Displays all metadata. Status and priority are clickable inline-edit dropdowns. Title and description are click-to-edit.

### Files
- Create: `app/elohim-app/src/app/avodah/components/story-detail/story-detail.component.ts`
- Modify: `app/elohim-app/src/app/avodah/avodah.routes.ts`

### Step 1: Create the component

Create `app/elohim-app/src/app/avodah/components/story-detail/story-detail.component.ts`:

```typescript
import { Component, OnInit, inject, signal } from '@angular/core';
import { ActivatedRoute, Router, RouterLink } from '@angular/router';

import { ContentNode } from '@app/lamad/models/content-node.model';

import {
  parseWorkStoryMeta,
  type WorkStoryMeta,
  type WorkStoryStatus,
  type WorkPriority,
  type WorkVisibility,
} from '../../models/work-story.model';
import { AvodahApiService } from '../../services/avodah-api.service';

@Component({
  selector: 'app-story-detail',
  standalone: true,
  imports: [RouterLink],
  template: `
    <div class="detail-shell">
      @if (story) {
        <header class="detail-header">
          <a
            [routerLink]="['/avodah/projects', projectId, 'board']"
            class="back-link"
            data-testid="back-to-board"
          >← Back to Board</a>
          <span class="project-ref">{{ projectTitle }}</span>
        </header>

        <main class="detail-body">
          <!-- Title (click to edit) -->
          @if (editingTitle()) {
            <input
              class="title-input"
              [value]="story.title"
              data-testid="title-input"
              (keydown.enter)="saveTitle($event)"
              (keydown.escape)="editingTitle.set(false)"
              (blur)="saveTitle($event)"
              autofocus
            />
          } @else {
            <h1
              class="story-title"
              data-testid="story-title"
              (click)="editingTitle.set(true)"
            >{{ story.title }}</h1>
          }

          <!-- Description (click to edit) -->
          @if (editingDescription()) {
            <textarea
              class="desc-input"
              [value]="story.description"
              data-testid="desc-input"
              rows="3"
              (keydown.escape)="editingDescription.set(false)"
              (blur)="saveDescription($event)"
            ></textarea>
          } @else {
            <p
              class="story-desc"
              data-testid="story-desc"
              (click)="editingDescription.set(true)"
            >{{ story.description || 'Click to add a description…' }}</p>
          }

          <!-- Metadata cards -->
          <div class="meta-row">
            <div class="meta-card">
              <label>Status</label>
              <select
                [value]="meta().status"
                (change)="changeStatus($event)"
                data-testid="status-select"
              >
                <option value="backlog">Backlog</option>
                <option value="todo">To Do</option>
                <option value="in-progress">In Progress</option>
                <option value="review">Review</option>
                <option value="done">Done</option>
              </select>
            </div>
            <div class="meta-card">
              <label>Priority</label>
              <select
                [value]="meta().priority"
                (change)="changePriority($event)"
                data-testid="priority-select"
              >
                <option value="low">Low</option>
                <option value="medium">Medium</option>
                <option value="high">High</option>
                <option value="urgent">Urgent</option>
              </select>
            </div>
            <div class="meta-card">
              <label>Visibility</label>
              <select
                [value]="meta().visibility"
                (change)="changeVisibility($event)"
                data-testid="visibility-select"
              >
                <option value="private">🔒 Private</option>
                <option value="community">👥 Community</option>
                <option value="exchange">⚖ Exchange</option>
              </select>
            </div>
          </div>

          <div class="meta-row">
            <div class="meta-card">
              <label>Assigned</label>
              <span class="meta-value">{{ meta().assigneeId ? '@' + meta().assigneeId : 'Unassigned' }}</span>
            </div>
            <div class="meta-card">
              <label>Story Points</label>
              <span class="meta-value">{{ meta().storyPoints ?? '—' }}</span>
            </div>
          </div>

          <!-- Tags -->
          <div class="section">
            <label>Tags</label>
            <div class="tags-row">
              @for (tag of story.tags; track tag) {
                <span class="tag">#{{ tag }}</span>
              }
              @if (story.tags.length === 0) {
                <span class="empty-hint">No tags</span>
              }
            </div>
          </div>

          <!-- Cadence -->
          <div class="section">
            <label>Cadence</label>
            @if (meta().cadence) {
              <span class="meta-value">
                {{ meta().cadence!.interval }}
                — next: {{ formatDate(meta().cadence!.nextOccurrence) }}
              </span>
            } @else {
              <span class="empty-hint">One-time story (no recurrence)</span>
            }
          </div>

          <!-- Attestation gates -->
          <div class="section">
            <label>Attestation Gates</label>
            @if (meta().attestationGates?.length) {
              <ul class="gates-list">
                @for (gate of meta().attestationGates!; track gate) {
                  <li>🎓 {{ gate }}</li>
                }
              </ul>
            } @else {
              <span class="empty-hint">Open to all — no mastery required</span>
            }
          </div>
        </main>
      } @else {
        <div class="loading">Loading story…</div>
      }
    </div>
  `,
  styles: [
    `
      .detail-shell {
        max-width: 720px;
        margin: 0 auto;
        padding: 2rem 1.5rem;
      }
      .detail-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 2rem;
      }
      .back-link {
        color: var(--lamad-accent-primary, #6366f1);
        text-decoration: none;
        font-size: 0.85rem;
      }
      .back-link:hover { text-decoration: underline; }
      .project-ref {
        font-size: 0.8rem;
        color: var(--lamad-text-muted, #64748b);
      }
      .story-title {
        font-size: 1.5rem;
        font-weight: 600;
        margin: 0 0 0.5rem;
        cursor: pointer;
        border-bottom: 1px dashed transparent;
      }
      .story-title:hover {
        border-bottom-color: rgba(99, 102, 241, 0.3);
      }
      .title-input {
        font-size: 1.5rem;
        font-weight: 600;
        width: 100%;
        background: rgba(15, 15, 26, 0.8);
        border: 1px solid var(--lamad-accent-primary, #6366f1);
        border-radius: 6px;
        color: var(--lamad-text-secondary, #e2e8f0);
        padding: 0.25rem 0.5rem;
        margin-bottom: 0.5rem;
        outline: none;
      }
      .story-desc {
        color: var(--lamad-text-secondary, #e2e8f0);
        font-size: 0.9rem;
        line-height: 1.6;
        cursor: pointer;
        margin: 0 0 1.5rem;
        min-height: 1.5rem;
      }
      .desc-input {
        width: 100%;
        background: rgba(15, 15, 26, 0.8);
        border: 1px solid var(--lamad-accent-primary, #6366f1);
        border-radius: 6px;
        color: var(--lamad-text-secondary, #e2e8f0);
        padding: 0.5rem;
        font-size: 0.9rem;
        resize: vertical;
        margin-bottom: 1.5rem;
        outline: none;
        font-family: inherit;
      }
      .meta-row {
        display: flex;
        gap: 1rem;
        margin-bottom: 1rem;
        flex-wrap: wrap;
      }
      .meta-card {
        flex: 1;
        min-width: 150px;
        background: rgba(15, 15, 26, 0.6);
        border: 1px solid rgba(99, 102, 241, 0.12);
        border-radius: 8px;
        padding: 0.75rem;
      }
      .meta-card label {
        display: block;
        font-size: 0.7rem;
        text-transform: uppercase;
        letter-spacing: 0.04em;
        color: var(--lamad-text-muted, #64748b);
        margin-bottom: 0.375rem;
      }
      .meta-card select {
        background: transparent;
        border: none;
        color: var(--lamad-text-secondary, #e2e8f0);
        font-size: 0.85rem;
        cursor: pointer;
        padding: 0;
        width: 100%;
        outline: none;
      }
      .meta-card select option {
        background: #1a1a2e;
      }
      .meta-value {
        font-size: 0.85rem;
        color: var(--lamad-text-secondary, #e2e8f0);
      }
      .section {
        margin-bottom: 1.25rem;
      }
      .section label {
        display: block;
        font-size: 0.7rem;
        text-transform: uppercase;
        letter-spacing: 0.04em;
        color: var(--lamad-text-muted, #64748b);
        margin-bottom: 0.375rem;
      }
      .tags-row {
        display: flex;
        gap: 0.375rem;
        flex-wrap: wrap;
      }
      .tag {
        font-size: 0.75rem;
        color: var(--lamad-accent-primary, #6366f1);
        background: rgba(99, 102, 241, 0.1);
        padding: 0.125rem 0.5rem;
        border-radius: 999px;
      }
      .empty-hint {
        font-size: 0.8rem;
        color: var(--lamad-text-muted, #64748b);
        font-style: italic;
      }
      .gates-list {
        list-style: none;
        padding: 0;
        margin: 0;
      }
      .gates-list li {
        font-size: 0.8rem;
        padding: 0.25rem 0;
        color: #a78bfa;
      }
      .loading {
        text-align: center;
        padding: 3rem;
        color: var(--lamad-text-muted, #64748b);
      }
    `,
  ],
})
export class StoryDetailComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly api = inject(AvodahApiService);

  story: ContentNode | null = null;
  projectId = '';
  projectTitle = '';
  readonly editingTitle = signal(false);
  readonly editingDescription = signal(false);

  ngOnInit(): void {
    void this.load();
  }

  meta(): WorkStoryMeta {
    return parseWorkStoryMeta(
      (this.story?.metadata ?? {}) as Record<string, unknown>,
    );
  }

  formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
    });
  }

  async changeStatus(event: Event): Promise<void> {
    const status = (event.target as HTMLSelectElement).value as WorkStoryStatus;
    if (!this.story) return;
    await this.api.updateStoryStatus(this.story.id, status);
    (this.story.metadata as Record<string, unknown>)['status'] = status;
  }

  async changePriority(event: Event): Promise<void> {
    const priority = (event.target as HTMLSelectElement).value as WorkPriority;
    if (!this.story) return;
    await firstValueFrom(
      this.api['storageApi'].updateContent(this.story.id, {
        metadata: { priority },
      }),
    );
    (this.story.metadata as Record<string, unknown>)['priority'] = priority;
  }

  async changeVisibility(event: Event): Promise<void> {
    const visibility = (event.target as HTMLSelectElement).value as WorkVisibility;
    if (!this.story) return;
    await firstValueFrom(
      this.api['storageApi'].updateContent(this.story.id, {
        metadata: { visibility },
      }),
    );
    (this.story.metadata as Record<string, unknown>)['visibility'] = visibility;
  }

  async saveTitle(event: Event): Promise<void> {
    const input = event.target as HTMLInputElement;
    const title = input.value.trim();
    if (!title || !this.story || title === this.story.title) {
      this.editingTitle.set(false);
      return;
    }
    await firstValueFrom(
      this.api['storageApi'].updateContent(this.story.id, { title }),
    );
    this.story = { ...this.story, title };
    this.editingTitle.set(false);
  }

  async saveDescription(event: Event): Promise<void> {
    const textarea = event.target as HTMLTextAreaElement;
    const description = textarea.value.trim();
    if (!this.story) {
      this.editingDescription.set(false);
      return;
    }
    await firstValueFrom(
      this.api['storageApi'].updateContent(this.story.id, {
        description,
      }),
    );
    this.story = { ...this.story, description };
    this.editingDescription.set(false);
  }

  private async load(): Promise<void> {
    this.projectId = this.route.snapshot.params['id'] as string;
    const storyId = this.route.snapshot.params['storyId'] as string;

    const projects = await this.api.getProjects();
    const project = projects.find(p => p.id === this.projectId);
    this.projectTitle = project?.title ?? 'Project';

    const stories = await this.api.getStoriesForProject(this.projectId);
    this.story = stories.find(s => s.id === storyId) ?? null;

    if (!this.story) {
      void this.router.navigate(['/avodah/projects', this.projectId, 'board']);
    }
  }
}
```

**IMPORTANT:** The component uses `firstValueFrom` from `rxjs` — add that import:
```typescript
import { firstValueFrom } from 'rxjs';
```

It also directly accesses `this.api['storageApi']` for fields not wrapped by `AvodahApiService` (priority, visibility, title, description). This is a pragmatic shortcut — a future refactor can add `updateStoryField()` to the service. The `StorageApiService` on the injected `AvodahApiService` is accessed via bracket notation to avoid exposing it publicly.

**ALTERNATIVE (cleaner):** Add a `updateStoryField(storyId: string, patch: Record<string, unknown>)` method to `AvodahApiService` that delegates to `storageApi.updateContent()`. If the implementer prefers this approach, add it as a thin wrapper. The plan shows the direct approach for brevity, but either works.

### Step 2: Add route

In `app/elohim-app/src/app/avodah/avodah.routes.ts`, add a new child route after the `tasks` route:

```typescript
{
  path: 'projects/:id/stories/:storyId',
  loadComponent: async () =>
    import('./components/story-detail/story-detail.component').then(
      m => m.StoryDetailComponent
    ),
  data: { title: 'Avodah — Story' },
},
```

### Step 3: Run lint and tests

```bash
cd app/elohim-app && pnpm run lint 2>&1 | grep -E "story-detail|avodah|ERROR" | head -10
pnpm exec vitest run --config vite.config.ts avodah 2>&1 | tail -10
```
Expected: Clean.

### Step 4: Commit

```bash
git add app/elohim-app/src/app/avodah/components/story-detail/story-detail.component.ts \
        app/elohim-app/src/app/avodah/avodah.routes.ts
git commit -m "feat(avodah): story detail view with inline edit for status, priority, visibility, title, description"
```

---

## Task 6: Navigate to Story Detail from Task List

Make task list items clickable, navigating to the story detail view.

### Files
- Modify: `app/elohim-app/src/app/avodah/components/task-list/task-list.component.ts`

### Step 1: Implement

Add Router import and inject:
```typescript
import { RouterLink, ActivatedRoute, Router } from '@angular/router';
```

```typescript
private readonly router = inject(Router);

openStory(story: ContentNode): void {
  void this.router.navigate(['/avodah/projects', this.projectId, 'stories', story.id]);
}
```

In the template, make task rows clickable by adding `(click)="openStory(story)"` and `class="clickable-row"` to each `.task-row` element.

Add style:
```css
.clickable-row { cursor: pointer; }
.clickable-row:hover { background: rgba(99, 102, 241, 0.06); border-radius: 6px; }
```

### Step 2: Run lint and tests

```bash
cd app/elohim-app && pnpm run lint 2>&1 | grep -E "task-list|ERROR" | head -10
pnpm exec vitest run --config vite.config.ts avodah 2>&1 | tail -10
```
Expected: Clean.

### Step 3: Commit

```bash
git add app/elohim-app/src/app/avodah/components/task-list/task-list.component.ts
git commit -m "feat(avodah): clickable task list items navigate to story detail"
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
