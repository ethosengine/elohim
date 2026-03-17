# Avodah Work Management — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Scaffold the Avodah pillar — a Taiga-like work management UI (projects, kanban boards, backlog, task list) where stories are EPR ContentNodes, gated by lamad attestations and connected to the shefa exchange.

**Architecture:** New Angular pillar at `/avodah` following the qahal pattern (layout shell + ElohimNavigatorComponent + lazy-loaded routes). Stories and projects extend the existing `ContentNode` model via two new `contentType` values (`work-story`, `work-project`). Work-specific payload (status, cadence, columns) lives in `ContentNode.metadata`. API service returns mock data for MVP; backend wiring is future work.

**Tech Stack:** Angular 19, standalone components, inline templates (required — Vitest doesn't resolve external `templateUrl`), `inject()` for DI (required — esbuild strips constructor metadata), Vitest for tests, SCSS for styles.

**Design doc:** `genesis/plans/2026-03-15-avodah-work-management-design.md`

---

## Task 1: Extend ContentType and Register Pillar Route

### Files
- Modify: `app/elohim-library/projects/elohim-service/src/models/content-node.model.ts`
- Modify: `app/elohim-app/src/app/app.routes.ts`

### Step 1: Add work ContentTypes to the union

In `content-node.model.ts`, find the `ContentType` union and add two new entries after `'example'`:

```typescript
export type ContentType =
  | 'source'
  | 'epic'
  | 'feature'
  | 'scenario'
  | 'concept'
  | 'role'
  | 'video'
  | 'organization'
  | 'book-chapter'
  | 'tool'
  | 'path'
  | 'assessment'
  | 'reference'
  | 'example'
  | 'work-story'    // ← add
  | 'work-project'; // ← add
```

### Step 2: Add `/avodah` route to app.routes.ts

Add before the `'**'` catch-all:

```typescript
{
  path: 'avodah',
  loadChildren: async () => import('./avodah/avodah.routes').then(m => m.AVODAH_ROUTES),
},
```

### Step 3: Verify TypeScript compiles

```bash
cd app/elohim-app && pnpm exec tsc --noEmit
```

Expected: no errors.

### Step 4: Commit

```bash
git add app/elohim-library/projects/elohim-service/src/models/content-node.model.ts
git add app/elohim-app/src/app/app.routes.ts
git commit -m "feat(avodah): register pillar route and add work-story/work-project ContentTypes"
```

---

## Task 2: Data Models

### Files
- Create: `app/elohim-app/src/app/avodah/models/work-story.model.ts`
- Create: `app/elohim-app/src/app/avodah/models/work-project.model.ts`
- Create: `app/elohim-app/src/app/avodah/models/index.ts`
- Create: `app/elohim-app/src/app/avodah/models/work-story.model.spec.ts`

### Step 1: Write the failing test

Create `app/elohim-app/src/app/avodah/models/work-story.model.spec.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { parseWorkStoryMeta, DEFAULT_BOARD_COLUMNS } from './work-project.model';

describe('parseWorkStoryMeta', () => {
  it('returns defaults when metadata is empty', () => {
    const result = parseWorkStoryMeta({});
    expect(result.status).toBe('backlog');
    expect(result.visibility).toBe('private');
    expect(result.priority).toBe('medium');
  });

  it('preserves cadence when present', () => {
    const cadence = { interval: 'weekly' as const, resetToStatus: 'todo' as const, nextOccurrence: '2026-03-22' };
    const result = parseWorkStoryMeta({ cadence });
    expect(result.cadence?.interval).toBe('weekly');
  });
});

describe('DEFAULT_BOARD_COLUMNS', () => {
  it('has five columns ending with a terminal done column', () => {
    expect(DEFAULT_BOARD_COLUMNS).toHaveLength(5);
    const last = DEFAULT_BOARD_COLUMNS[DEFAULT_BOARD_COLUMNS.length - 1];
    expect(last.isTerminal).toBe(true);
    expect(last.id).toBe('done');
  });
});
```

### Step 2: Run test to verify it fails

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "work-story.model"
```

Expected: FAIL — modules not found.

### Step 3: Create work-story.model.ts

```typescript
// app/elohim-app/src/app/avodah/models/work-story.model.ts

export type WorkStoryStatus = 'backlog' | 'todo' | 'in-progress' | 'review' | 'done';
export type WorkVisibility = 'private' | 'community' | 'exchange';
export type WorkPriority = 'low' | 'medium' | 'high' | 'urgent';
export type CadenceInterval = 'daily' | 'weekly' | 'monthly' | 'custom';

export interface WorkCadence {
  interval: CadenceInterval;
  customIntervalDays?: number;
  resetToStatus: 'backlog' | 'todo';
  nextOccurrence: string; // ISO date
}

/** Structured payload stored in ContentNode.metadata for work-story nodes */
export interface WorkStoryMeta {
  projectId: string;
  status: WorkStoryStatus;
  visibility: WorkVisibility;
  priority: WorkPriority;
  storyPoints?: number;
  assigneeId?: string;
  /** lamad ContentNode IDs required to bid/accept this story */
  attestationGates?: string[];
  /** shefa ServiceRequest ID — set when published to exchange */
  exchangeRequestId?: string;
  cadence?: WorkCadence;
}

const DEFAULTS: WorkStoryMeta = {
  projectId: '',
  status: 'backlog',
  visibility: 'private',
  priority: 'medium',
};

export function parseWorkStoryMeta(raw: Record<string, unknown>): WorkStoryMeta {
  return { ...DEFAULTS, ...raw } as WorkStoryMeta;
}
```

### Step 4: Create work-project.model.ts

```typescript
// app/elohim-app/src/app/avodah/models/work-project.model.ts

export interface BoardColumn {
  id: string;
  name: string;
  color?: string;
  /** Terminal columns trigger cadence reset when story moves here */
  isTerminal?: boolean;
}

/** Structured payload stored in ContentNode.metadata for work-project nodes */
export interface WorkProjectMeta {
  columns: BoardColumn[];
  visibility: 'private' | 'community';
  memberIds?: string[];
}

export const DEFAULT_BOARD_COLUMNS: BoardColumn[] = [
  { id: 'backlog', name: 'Backlog', color: '#64748b' },
  { id: 'todo', name: 'To Do', color: '#6366f1' },
  { id: 'in-progress', name: 'In Progress', color: '#f59e0b' },
  { id: 'review', name: 'Review', color: '#8b5cf6' },
  { id: 'done', name: 'Done', color: '#10b981', isTerminal: true },
];

export function parseWorkProjectMeta(raw: Record<string, unknown>): WorkProjectMeta {
  return {
    columns: DEFAULT_BOARD_COLUMNS,
    visibility: 'private',
    ...raw,
  } as WorkProjectMeta;
}
```

### Step 5: Create models/index.ts

```typescript
export * from './work-story.model';
export * from './work-project.model';
```

### Step 6: Run test to verify it passes

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "work-story.model"
```

Expected: PASS (2 test suites, 3 tests).

### Step 7: Commit

```bash
git add app/elohim-app/src/app/avodah/models/
git commit -m "feat(avodah): add WorkStoryMeta and WorkProjectMeta models"
```

---

## Task 3: API Service (Mock Data)

### Files
- Create: `app/elohim-app/src/app/avodah/services/avodah-api.service.ts`
- Create: `app/elohim-app/src/app/avodah/services/index.ts`
- Create: `app/elohim-app/src/app/avodah/services/avodah-api.service.spec.ts`

### Step 1: Write the failing test

```typescript
// avodah-api.service.spec.ts
import { describe, it, expect } from 'vitest';
import { TestBed } from '@angular/core/testing';
import { AvodahApiService } from './avodah-api.service';

describe('AvodahApiService', () => {
  let service: AvodahApiService;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    service = TestBed.inject(AvodahApiService);
  });

  it('getProjects returns at least one mock project', async () => {
    const projects = await service.getProjects();
    expect(projects.length).toBeGreaterThan(0);
    expect(projects[0].contentType).toBe('work-project');
  });

  it('getStoriesForProject returns stories with matching projectId', async () => {
    const projects = await service.getProjects();
    const projectId = projects[0].id;
    const stories = await service.getStoriesForProject(projectId);
    expect(stories.every(s => {
      const meta = s.metadata as Record<string, unknown>;
      return meta['projectId'] === projectId;
    })).toBe(true);
  });
});
```

### Step 2: Run test to verify it fails

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "avodah-api.service"
```

Expected: FAIL — service not found.

### Step 3: Create avodah-api.service.ts

```typescript
// app/elohim-app/src/app/avodah/services/avodah-api.service.ts
import { Injectable } from '@angular/core';
import { ContentNode } from '@elohim/content-node.model';
import { DEFAULT_BOARD_COLUMNS } from '../models/work-project.model';

const NOW = new Date().toISOString();

const MOCK_PROJECT: ContentNode = {
  id: 'proj-household-2026',
  contentType: 'work-project',
  title: 'Household',
  description: 'Personal and household tasks',
  content: '',
  contentFormat: 'markdown',
  tags: ['household'],
  relatedNodeIds: [],
  metadata: {
    columns: DEFAULT_BOARD_COLUMNS,
    visibility: 'private',
  },
  reach: 'private',
  createdAt: NOW,
  updatedAt: NOW,
};

const MOCK_STORIES: ContentNode[] = [
  {
    id: 'story-001',
    contentType: 'work-story',
    title: 'Take out the trash',
    description: 'Weekly trash collection — bins to the curb by 7am Friday.',
    content: '',
    contentFormat: 'markdown',
    tags: ['chores', 'weekly'],
    relatedNodeIds: [],
    metadata: {
      projectId: 'proj-household-2026',
      status: 'todo',
      visibility: 'private',
      priority: 'medium',
      cadence: { interval: 'weekly', resetToStatus: 'todo', nextOccurrence: '2026-03-21' },
    },
    reach: 'private',
    createdAt: NOW,
    updatedAt: NOW,
  },
  {
    id: 'story-002',
    contentType: 'work-story',
    title: 'Fix the kitchen faucet',
    description: 'The cold handle drips. Replace the O-ring on the cartridge.',
    content: '',
    contentFormat: 'markdown',
    tags: ['plumbing', 'home'],
    relatedNodeIds: [],
    metadata: {
      projectId: 'proj-household-2026',
      status: 'backlog',
      visibility: 'private',
      priority: 'high',
      storyPoints: 3,
    },
    reach: 'private',
    createdAt: NOW,
    updatedAt: NOW,
  },
  {
    id: 'story-003',
    contentType: 'work-story',
    title: 'Cook meals for the week',
    description: 'Sunday meal prep — 5 dinners, lunches for Monday–Friday.',
    content: '',
    contentFormat: 'markdown',
    tags: ['cooking', 'weekly'],
    relatedNodeIds: [],
    metadata: {
      projectId: 'proj-household-2026',
      status: 'in-progress',
      visibility: 'community',
      priority: 'high',
      cadence: { interval: 'weekly', resetToStatus: 'todo', nextOccurrence: '2026-03-22' },
    },
    reach: 'community',
    createdAt: NOW,
    updatedAt: NOW,
  },
];

@Injectable({ providedIn: 'root' })
export class AvodahApiService {
  async getProjects(): Promise<ContentNode[]> {
    return [MOCK_PROJECT];
  }

  async getStoriesForProject(projectId: string): Promise<ContentNode[]> {
    return MOCK_STORIES.filter(s => (s.metadata as Record<string, unknown>)['projectId'] === projectId);
  }

  async updateStoryStatus(storyId: string, status: string): Promise<void> {
    const story = MOCK_STORIES.find(s => s.id === storyId);
    if (story) {
      (story.metadata as Record<string, unknown>)['status'] = status;
      story.updatedAt = new Date().toISOString();
    }
  }
}
```

### Step 4: Create services/index.ts

```typescript
export { AvodahApiService } from './avodah-api.service';
```

### Step 5: Run test to verify it passes

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "avodah-api.service"
```

Expected: PASS.

### Step 6: Commit

```bash
git add app/elohim-app/src/app/avodah/services/
git commit -m "feat(avodah): add AvodahApiService with mock project and story data"
```

---

## Task 4: Pillar Scaffold — Routes, Layout, Barrel

### Files
- Create: `app/elohim-app/src/app/avodah/avodah.routes.ts`
- Create: `app/elohim-app/src/app/avodah/components/avodah-layout/avodah-layout.component.ts`
- Create: `app/elohim-app/src/app/avodah/components/avodah-home/avodah-home.component.ts`
- Create: `app/elohim-app/src/app/avodah/index.ts`

### Step 1: Create avodah-layout.component.ts

Follow the qahal `CommunityLayoutComponent` pattern exactly — inline template, `ElohimNavigatorComponent`:

```typescript
// avodah-layout.component.ts
import { Component } from '@angular/core';
import { RouterOutlet } from '@angular/router';
import { ElohimNavigatorComponent } from '@app/elohim/components/elohim-navigator/elohim-navigator.component';

@Component({
  selector: 'app-avodah-layout',
  standalone: true,
  imports: [RouterOutlet, ElohimNavigatorComponent],
  template: `
    <div class="avodah-container">
      <app-elohim-navigator [context]="'avodah'" [showSearch]="true">
        <div class="avodah-main">
          <router-outlet></router-outlet>
        </div>
      </app-elohim-navigator>
    </div>
  `,
  styles: [`
    .avodah-container {
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      background: var(--lamad-bg-primary, #0f0f1a);
      color: var(--lamad-text-secondary, #e2e8f0);
    }
    .avodah-main {
      flex: 1;
      display: flex;
    }
  `],
})
export class AvodahLayoutComponent {}
```

### Step 2: Create avodah-home.component.ts (placeholder)

```typescript
// avodah-home.component.ts
import { Component } from '@angular/core';
import { RouterLink } from '@angular/router';

@Component({
  selector: 'app-avodah-home',
  standalone: true,
  imports: [RouterLink],
  template: `
    <div class="avodah-home">
      <h1>Avodah</h1>
      <p>Work management — projects, boards, and tasks.</p>
      <a routerLink="projects">View Projects</a>
    </div>
  `,
  styles: [`
    .avodah-home { padding: 2rem; }
    h1 { font-size: 2rem; margin-bottom: 0.5rem; }
  `],
})
export class AvodahHomeComponent {}
```

### Step 3: Create avodah.routes.ts

```typescript
// avodah.routes.ts
import { Routes } from '@angular/router';

export const AVODAH_ROUTES: Routes = [
  {
    path: '',
    loadComponent: async () =>
      import('./components/avodah-layout/avodah-layout.component').then(m => m.AvodahLayoutComponent),
    children: [
      {
        path: '',
        loadComponent: async () =>
          import('./components/avodah-home/avodah-home.component').then(m => m.AvodahHomeComponent),
        data: { title: 'Avodah — Work Management' },
      },
      {
        path: 'projects',
        loadComponent: async () =>
          import('./components/project-list/project-list.component').then(m => m.ProjectListComponent),
        data: { title: 'Avodah — Projects' },
      },
      {
        path: 'projects/:id/board',
        loadComponent: async () =>
          import('./components/project-board/project-board.component').then(m => m.ProjectBoardComponent),
        data: { title: 'Avodah — Board' },
      },
      {
        path: 'projects/:id/backlog',
        loadComponent: async () =>
          import('./components/project-backlog/project-backlog.component').then(m => m.ProjectBacklogComponent),
        data: { title: 'Avodah — Backlog' },
      },
      {
        path: 'projects/:id/tasks',
        loadComponent: async () =>
          import('./components/task-list/task-list.component').then(m => m.TaskListComponent),
        data: { title: 'Avodah — Tasks' },
      },
    ],
  },
];
```

### Step 4: Create index.ts

```typescript
// app/elohim-app/src/app/avodah/index.ts
export * from './models';
export * from './services';
```

### Step 5: Verify the app compiles and /avodah loads

```bash
cd app/elohim-app && pnpm exec tsc --noEmit
```

Expected: no errors (the board/backlog/task components don't exist yet — the routes use lazy loading so compile succeeds; they'll 404 at runtime until Task 6–8).

### Step 6: Commit

```bash
git add app/elohim-app/src/app/avodah/
git commit -m "feat(avodah): scaffold pillar — routes, layout, home, barrel"
```

---

## Task 5: Story Card Component

The story card is used in both board and backlog views. Build it first so the later tasks can use it.

### Files
- Create: `app/elohim-app/src/app/avodah/components/story-card/story-card.component.ts`
- Create: `app/elohim-app/src/app/avodah/components/story-card/story-card.component.spec.ts`

### Step 1: Write the failing test

```typescript
// story-card.component.spec.ts
import { describe, it, expect } from 'vitest';
import { TestBed } from '@angular/core/testing';
import { StoryCardComponent } from './story-card.component';
import { ContentNode } from '@elohim/content-node.model';

const MOCK_STORY: ContentNode = {
  id: 'story-test-001',
  contentType: 'work-story',
  title: 'Fix the kitchen faucet',
  description: '',
  content: '',
  contentFormat: 'markdown',
  tags: ['plumbing', 'home'],
  relatedNodeIds: [],
  metadata: {
    projectId: 'proj-001',
    status: 'backlog',
    visibility: 'exchange',
    priority: 'high',
    storyPoints: 3,
    attestationGates: ['path-plumbing-basics'],
    cadence: undefined,
  },
  reach: 'private',
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
};

describe('StoryCardComponent', () => {
  it('creates', async () => {
    const fixture = TestBed.createComponent(StoryCardComponent);
    fixture.componentRef.setInput('story', MOCK_STORY);
    expect(fixture.componentInstance).toBeTruthy();
  });

  it('shows attestation badge when gates are present', () => {
    const fixture = TestBed.createComponent(StoryCardComponent);
    fixture.componentRef.setInput('story', MOCK_STORY);
    fixture.detectChanges();
    const el: HTMLElement = fixture.nativeElement;
    expect(el.querySelector('[data-testid="badge-attestation"]')).toBeTruthy();
  });

  it('shows exchange badge when visibility is exchange', () => {
    const fixture = TestBed.createComponent(StoryCardComponent);
    fixture.componentRef.setInput('story', MOCK_STORY);
    fixture.detectChanges();
    const el: HTMLElement = fixture.nativeElement;
    expect(el.querySelector('[data-testid="badge-exchange"]')).toBeTruthy();
  });
});
```

### Step 2: Run test to verify it fails

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "story-card.component"
```

Expected: FAIL.

### Step 3: Create story-card.component.ts

```typescript
// story-card.component.ts
import { Component, input, output } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ContentNode } from '@elohim/content-node.model';
import { parseWorkStoryMeta, WorkStoryMeta } from '../../models/work-story.model';

@Component({
  selector: 'app-story-card',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="story-card" [attr.data-priority]="meta().priority" (click)="cardClick.emit(story())">
      <!-- Badges row -->
      <div class="badges">
        <span *ngIf="hasAttestationGates()" class="badge badge-attestation"
              data-testid="badge-attestation" title="Requires lamad attestation">🎓</span>
        <span *ngIf="isOnExchange()" class="badge badge-exchange"
              data-testid="badge-exchange" title="Published to exchange">◈</span>
        <span *ngIf="hasCadence()" class="badge badge-cadence"
              data-testid="badge-cadence" title="Recurring">↺</span>
        <span class="story-id">#{{ shortId() }}</span>
      </div>

      <!-- Title -->
      <p class="story-title">{{ story().title }}</p>

      <!-- Footer -->
      <div class="story-footer">
        <span class="priority-dot" [attr.data-priority]="meta().priority"></span>
        <span class="priority-label">{{ meta().priority | titlecase }}</span>
        <span *ngIf="meta().storyPoints" class="story-points">◷ {{ meta().storyPoints }}pts</span>
        <span class="assignee">{{ meta().assigneeId ? '@' + meta().assigneeId : '@unassigned' }}</span>
      </div>

      <!-- Tags -->
      <div class="tags" *ngIf="story().tags.length">
        <span class="tag" *ngFor="let tag of story().tags">#{{ tag }}</span>
      </div>
    </div>
  `,
  styles: [`
    .story-card {
      background: var(--lamad-surface, rgba(30,30,46,0.9));
      border: 1px solid var(--lamad-border, rgba(99,102,241,0.15));
      border-radius: 8px;
      padding: 0.75rem;
      cursor: pointer;
      transition: border-color 0.15s, transform 0.1s;
      &:hover { border-color: var(--lamad-accent-primary, #6366f1); transform: translateY(-1px); }
    }
    .badges { display: flex; align-items: center; gap: 4px; margin-bottom: 6px; font-size: 0.75rem; }
    .badge { opacity: 0.9; }
    .story-id { margin-left: auto; color: var(--lamad-text-muted, #64748b); }
    .story-title { font-size: 0.9rem; font-weight: 500; margin: 0 0 8px; line-height: 1.3; }
    .story-footer { display: flex; align-items: center; gap: 8px; font-size: 0.75rem; color: var(--lamad-text-muted, #64748b); }
    .priority-dot { width: 8px; height: 8px; border-radius: 50%; background: var(--priority-color, #6366f1); flex-shrink: 0; }
    [data-priority="urgent"] .priority-dot, .priority-dot[data-priority="urgent"] { background: #ef4444; }
    [data-priority="high"] .priority-dot, .priority-dot[data-priority="high"] { background: #f59e0b; }
    [data-priority="medium"] .priority-dot, .priority-dot[data-priority="medium"] { background: #6366f1; }
    [data-priority="low"] .priority-dot, .priority-dot[data-priority="low"] { background: #64748b; }
    .tags { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 6px; }
    .tag { font-size: 0.7rem; color: var(--lamad-text-muted, #64748b); }
  `],
})
export class StoryCardComponent {
  story = input.required<ContentNode>();
  cardClick = output<ContentNode>();

  meta(): WorkStoryMeta {
    return parseWorkStoryMeta(this.story().metadata as Record<string, unknown>);
  }

  shortId(): string {
    return this.story().id.split('-').pop() ?? this.story().id;
  }

  hasAttestationGates(): boolean {
    const gates = this.meta().attestationGates;
    return Array.isArray(gates) && gates.length > 0;
  }

  isOnExchange(): boolean {
    return this.meta().visibility === 'exchange';
  }

  hasCadence(): boolean {
    return !!this.meta().cadence;
  }
}
```

### Step 4: Run test to verify it passes

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "story-card.component"
```

Expected: PASS.

### Step 5: Commit

```bash
git add app/elohim-app/src/app/avodah/components/story-card/
git commit -m "feat(avodah): add StoryCardComponent with attestation, exchange, and cadence badges"
```

---

## Task 6: Project Board (Kanban View)

### Files
- Create: `app/elohim-app/src/app/avodah/components/project-board/project-board.component.ts`

### Step 1: Create project-board.component.ts

No test first here — the component is pure display, DOM interaction is hard to unit test meaningfully for kanban. Focus on correct rendering logic.

```typescript
// project-board.component.ts
import { Component, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, RouterLink } from '@angular/router';
import { ContentNode } from '@elohim/content-node.model';
import { AvodahApiService } from '../../services/avodah-api.service';
import { StoryCardComponent } from '../story-card/story-card.component';
import { BoardColumn, DEFAULT_BOARD_COLUMNS, parseWorkProjectMeta } from '../../models/work-project.model';
import { parseWorkStoryMeta } from '../../models/work-story.model';

@Component({
  selector: 'app-project-board',
  standalone: true,
  imports: [CommonModule, RouterLink, StoryCardComponent],
  template: `
    <div class="board-shell">
      <!-- Sidebar -->
      <nav class="board-sidebar">
        <div class="project-switcher">
          <span class="project-name">{{ project?.title ?? 'Project' }}</span>
        </div>
        <ul class="sidebar-nav">
          <li class="active">▦ Board</li>
          <li><a [routerLink]="['../backlog']">≡ Backlog</a></li>
          <li><a [routerLink]="['../tasks']">↺ Tasks</a></li>
        </ul>
        <a routerLink="/avodah/projects" class="new-project">+ New Project</a>
      </nav>

      <!-- Board columns -->
      <main class="board-main">
        <div class="columns">
          <div class="column" *ngFor="let col of columns">
            <div class="column-header">
              <span class="col-name">{{ col.name }}</span>
              <span class="col-count">{{ storiesInColumn(col.id).length }}</span>
            </div>
            <div class="column-stories">
              <app-story-card
                *ngFor="let story of storiesInColumn(col.id)"
                [story]="story"
              />
              <div class="add-story">+ Add story</div>
            </div>
          </div>
        </div>
      </main>
    </div>
  `,
  styles: [`
    .board-shell { display: flex; height: calc(100vh - 60px); overflow: hidden; }

    /* Sidebar */
    .board-sidebar {
      width: 200px;
      flex-shrink: 0;
      background: var(--lamad-surface, rgba(20,20,36,0.95));
      border-right: 1px solid var(--lamad-border, rgba(99,102,241,0.12));
      display: flex;
      flex-direction: column;
      padding: 1rem 0;
    }
    .project-switcher {
      padding: 0 1rem 1rem;
      border-bottom: 1px solid var(--lamad-border, rgba(99,102,241,0.12));
      margin-bottom: 0.5rem;
    }
    .project-name { font-weight: 600; font-size: 0.9rem; }
    .sidebar-nav { list-style: none; padding: 0; margin: 0; flex: 1; }
    .sidebar-nav li { padding: 0.6rem 1rem; font-size: 0.85rem; cursor: pointer; border-radius: 0 6px 6px 0; margin-right: 0.5rem; }
    .sidebar-nav li.active { background: var(--lamad-accent-primary, #6366f1); color: white; }
    .sidebar-nav li a { color: var(--lamad-text-secondary, #e2e8f0); text-decoration: none; display: block; }
    .sidebar-nav li:hover:not(.active) { background: rgba(99,102,241,0.1); }
    .new-project { padding: 0.75rem 1rem; font-size: 0.8rem; color: var(--lamad-accent-primary, #6366f1); text-decoration: none; cursor: pointer; }

    /* Board */
    .board-main { flex: 1; overflow-x: auto; padding: 1rem; }
    .columns { display: flex; gap: 1rem; height: 100%; align-items: flex-start; }
    .column {
      width: 280px;
      flex-shrink: 0;
      background: rgba(15,15,26,0.6);
      border-radius: 10px;
      padding: 0.75rem;
      max-height: 100%;
      overflow-y: auto;
    }
    .column-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.75rem; }
    .col-name { font-weight: 600; font-size: 0.85rem; }
    .col-count {
      background: rgba(99,102,241,0.2);
      border-radius: 999px;
      padding: 1px 7px;
      font-size: 0.75rem;
      color: var(--lamad-text-muted, #64748b);
    }
    .column-stories { display: flex; flex-direction: column; gap: 0.5rem; }
    .add-story {
      padding: 0.5rem;
      font-size: 0.8rem;
      color: var(--lamad-text-muted, #64748b);
      cursor: pointer;
      border-radius: 6px;
      text-align: center;
      border: 1px dashed var(--lamad-border, rgba(99,102,241,0.15));
      &:hover { border-color: var(--lamad-accent-primary, #6366f1); color: var(--lamad-accent-primary, #6366f1); }
    }
  `],
})
export class ProjectBoardComponent implements OnInit {
  private route = inject(ActivatedRoute);
  private api = inject(AvodahApiService);

  project: ContentNode | null = null;
  stories: ContentNode[] = [];
  columns: BoardColumn[] = DEFAULT_BOARD_COLUMNS;

  async ngOnInit(): Promise<void> {
    const projectId = this.route.snapshot.params['id'];
    const projects = await this.api.getProjects();
    this.project = projects.find(p => p.id === projectId) ?? null;
    if (this.project) {
      const meta = parseWorkProjectMeta(this.project.metadata as Record<string, unknown>);
      this.columns = meta.columns;
    }
    this.stories = await this.api.getStoriesForProject(projectId);
  }

  storiesInColumn(columnId: string): ContentNode[] {
    return this.stories.filter(s => {
      const meta = parseWorkStoryMeta(s.metadata as Record<string, unknown>);
      return meta.status === columnId;
    });
  }
}
```

### Step 2: Verify it compiles

```bash
cd app/elohim-app && pnpm exec tsc --noEmit
```

Expected: no errors.

### Step 3: Commit

```bash
git add app/elohim-app/src/app/avodah/components/project-board/
git commit -m "feat(avodah): add ProjectBoardComponent with sidebar nav and kanban columns"
```

---

## Task 7: Project Backlog View

### Files
- Create: `app/elohim-app/src/app/avodah/components/project-backlog/project-backlog.component.ts`

### Step 1: Create project-backlog.component.ts

```typescript
// project-backlog.component.ts
import { Component, OnInit, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, RouterLink } from '@angular/router';
import { ContentNode } from '@elohim/content-node.model';
import { AvodahApiService } from '../../services/avodah-api.service';
import { parseWorkStoryMeta, WorkPriority, WorkStoryStatus } from '../../models/work-story.model';

@Component({
  selector: 'app-project-backlog',
  standalone: true,
  imports: [CommonModule, RouterLink],
  template: `
    <div class="board-shell">
      <!-- Sidebar (same pattern as board) -->
      <nav class="board-sidebar">
        <div class="project-switcher">
          <span class="project-name">{{ project?.title ?? 'Project' }}</span>
        </div>
        <ul class="sidebar-nav">
          <li><a [routerLink]="['../board']">▦ Board</a></li>
          <li class="active">≡ Backlog</li>
          <li><a [routerLink]="['../tasks']">↺ Tasks</a></li>
        </ul>
        <a routerLink="/avodah/projects" class="new-project">+ New Project</a>
      </nav>

      <!-- Backlog list -->
      <main class="backlog-main">
        <!-- Filter bar -->
        <div class="filter-bar">
          <select (change)="filterStatus($event)">
            <option value="">All Statuses</option>
            <option value="backlog">Backlog</option>
            <option value="todo">To Do</option>
            <option value="in-progress">In Progress</option>
            <option value="review">Review</option>
            <option value="done">Done</option>
          </select>
          <select (change)="filterPriority($event)">
            <option value="">All Priorities</option>
            <option value="urgent">Urgent</option>
            <option value="high">High</option>
            <option value="medium">Medium</option>
            <option value="low">Low</option>
          </select>
          <button class="btn-new-story">+ New Story</button>
        </div>

        <!-- Story list -->
        <table class="backlog-table">
          <thead>
            <tr>
              <th>Title</th>
              <th>Status</th>
              <th>Priority</th>
              <th>Points</th>
              <th>Cadence</th>
              <th>Visibility</th>
            </tr>
          </thead>
          <tbody>
            <tr *ngFor="let story of filteredStories()" class="story-row">
              <td class="story-title">{{ story.title }}</td>
              <td><span class="status-pill" [attr.data-status]="storyMeta(story).status">{{ storyMeta(story).status }}</span></td>
              <td><span class="priority-dot" [attr.data-priority]="storyMeta(story).priority"></span> {{ storyMeta(story).priority }}</td>
              <td>{{ storyMeta(story).storyPoints ?? '—' }}</td>
              <td>{{ storyMeta(story).cadence?.interval ?? '—' }}</td>
              <td>{{ storyMeta(story).visibility }}</td>
            </tr>
            <tr *ngIf="filteredStories().length === 0">
              <td colspan="6" class="empty">No stories match the current filters.</td>
            </tr>
          </tbody>
        </table>
      </main>
    </div>
  `,
  styles: [`
    .board-shell { display: flex; height: calc(100vh - 60px); }

    /* Sidebar — same as board (copy) */
    .board-sidebar {
      width: 200px; flex-shrink: 0;
      background: var(--lamad-surface, rgba(20,20,36,0.95));
      border-right: 1px solid var(--lamad-border, rgba(99,102,241,0.12));
      display: flex; flex-direction: column; padding: 1rem 0;
    }
    .project-switcher { padding: 0 1rem 1rem; border-bottom: 1px solid var(--lamad-border, rgba(99,102,241,0.12)); margin-bottom: 0.5rem; }
    .project-name { font-weight: 600; font-size: 0.9rem; }
    .sidebar-nav { list-style: none; padding: 0; margin: 0; flex: 1; }
    .sidebar-nav li { padding: 0.6rem 1rem; font-size: 0.85rem; cursor: pointer; border-radius: 0 6px 6px 0; margin-right: 0.5rem; }
    .sidebar-nav li.active { background: var(--lamad-accent-primary, #6366f1); color: white; }
    .sidebar-nav li a { color: var(--lamad-text-secondary, #e2e8f0); text-decoration: none; display: block; }
    .new-project { padding: 0.75rem 1rem; font-size: 0.8rem; color: var(--lamad-accent-primary, #6366f1); text-decoration: none; }

    /* Backlog */
    .backlog-main { flex: 1; padding: 1.5rem; overflow-y: auto; }
    .filter-bar { display: flex; gap: 0.75rem; margin-bottom: 1.25rem; align-items: center; }
    .filter-bar select {
      background: var(--lamad-surface, rgba(30,30,46,0.9));
      border: 1px solid var(--lamad-border, rgba(99,102,241,0.2));
      color: var(--lamad-text-secondary, #e2e8f0);
      padding: 0.4rem 0.75rem; border-radius: 6px; font-size: 0.85rem;
    }
    .btn-new-story {
      margin-left: auto;
      background: var(--lamad-accent-primary, #6366f1);
      color: white; border: none; padding: 0.4rem 1rem;
      border-radius: 6px; font-size: 0.85rem; cursor: pointer;
    }
    .backlog-table { width: 100%; border-collapse: collapse; font-size: 0.875rem; }
    .backlog-table th { text-align: left; padding: 0.5rem 0.75rem; color: var(--lamad-text-muted, #64748b); border-bottom: 1px solid var(--lamad-border, rgba(99,102,241,0.12)); }
    .story-row td { padding: 0.6rem 0.75rem; border-bottom: 1px solid var(--lamad-border, rgba(99,102,241,0.07)); }
    .story-row:hover td { background: rgba(99,102,241,0.05); }
    .status-pill { padding: 2px 8px; border-radius: 999px; font-size: 0.75rem; background: rgba(99,102,241,0.15); }
    .priority-dot { display: inline-block; width: 8px; height: 8px; border-radius: 50%; margin-right: 4px; vertical-align: middle; }
    [data-priority="urgent"] { background: #ef4444; }
    [data-priority="high"] { background: #f59e0b; }
    [data-priority="medium"] { background: #6366f1; }
    [data-priority="low"] { background: #64748b; }
    .empty { text-align: center; color: var(--lamad-text-muted, #64748b); padding: 2rem; }
  `],
})
export class ProjectBacklogComponent implements OnInit {
  private route = inject(ActivatedRoute);
  private api = inject(AvodahApiService);

  project: ContentNode | null = null;
  stories: ContentNode[] = [];
  activeStatusFilter = signal<WorkStoryStatus | ''>('');
  activePriorityFilter = signal<WorkPriority | ''>('');

  async ngOnInit(): Promise<void> {
    const projectId = this.route.snapshot.params['id'];
    const projects = await this.api.getProjects();
    this.project = projects.find(p => p.id === projectId) ?? null;
    this.stories = await this.api.getStoriesForProject(projectId);
  }

  storyMeta(story: ContentNode) {
    return parseWorkStoryMeta(story.metadata as Record<string, unknown>);
  }

  filteredStories(): ContentNode[] {
    return this.stories.filter(s => {
      const meta = this.storyMeta(s);
      if (this.activeStatusFilter() && meta.status !== this.activeStatusFilter()) return false;
      if (this.activePriorityFilter() && meta.priority !== this.activePriorityFilter()) return false;
      return true;
    });
  }

  filterStatus(event: Event): void {
    this.activeStatusFilter.set((event.target as HTMLSelectElement).value as WorkStoryStatus | '');
  }

  filterPriority(event: Event): void {
    this.activePriorityFilter.set((event.target as HTMLSelectElement).value as WorkPriority | '');
  }
}
```

### Step 2: Verify compilation

```bash
cd app/elohim-app && pnpm exec tsc --noEmit
```

Expected: no errors.

### Step 3: Commit

```bash
git add app/elohim-app/src/app/avodah/components/project-backlog/
git commit -m "feat(avodah): add ProjectBacklogComponent with status/priority filters"
```

---

## Task 8: Task List View (Recurring Items)

### Files
- Create: `app/elohim-app/src/app/avodah/components/task-list/task-list.component.ts`

### Step 1: Create task-list.component.ts

```typescript
// task-list.component.ts
import { Component, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, RouterLink } from '@angular/router';
import { ContentNode } from '@elohim/content-node.model';
import { AvodahApiService } from '../../services/avodah-api.service';
import { parseWorkStoryMeta } from '../../models/work-story.model';

type IntervalGroup = 'daily' | 'weekly' | 'monthly' | 'custom';

@Component({
  selector: 'app-task-list',
  standalone: true,
  imports: [CommonModule, RouterLink],
  template: `
    <div class="board-shell">
      <!-- Sidebar -->
      <nav class="board-sidebar">
        <div class="project-switcher">
          <span class="project-name">{{ project?.title ?? 'Project' }}</span>
        </div>
        <ul class="sidebar-nav">
          <li><a [routerLink]="['../board']">▦ Board</a></li>
          <li><a [routerLink]="['../backlog']">≡ Backlog</a></li>
          <li class="active">↺ Tasks</li>
        </ul>
        <a routerLink="/avodah/projects" class="new-project">+ New Project</a>
      </nav>

      <!-- Task list -->
      <main class="tasks-main">
        <h2 class="section-title">Recurring Tasks</h2>

        <div *ngFor="let group of groups" class="interval-group">
          <h3 class="interval-heading">{{ group | titlecase }}</h3>
          <div class="task-rows">
            <div class="task-row" *ngFor="let story of storiesInGroup(group)">
              <button class="check-btn" title="Mark complete">○</button>
              <div class="task-info">
                <span class="task-title">{{ story.title }}</span>
                <span class="task-next">Next: {{ nextOccurrence(story) }}</span>
              </div>
              <div class="task-badges">
                <span *ngIf="isOnExchange(story)" class="badge-exchange" title="On exchange">◈</span>
                <span *ngIf="hasGates(story)" class="badge-gate" title="Has attestation gate">🎓</span>
              </div>
            </div>
            <div class="empty-group" *ngIf="storiesInGroup(group).length === 0">
              No {{ group }} tasks.
            </div>
          </div>
        </div>

        <div class="no-tasks" *ngIf="cadenceStories.length === 0">
          <p>No recurring tasks yet. Add a cadence to a story in the backlog to see it here.</p>
        </div>
      </main>
    </div>
  `,
  styles: [`
    .board-shell { display: flex; height: calc(100vh - 60px); }
    .board-sidebar {
      width: 200px; flex-shrink: 0;
      background: var(--lamad-surface, rgba(20,20,36,0.95));
      border-right: 1px solid var(--lamad-border, rgba(99,102,241,0.12));
      display: flex; flex-direction: column; padding: 1rem 0;
    }
    .project-switcher { padding: 0 1rem 1rem; border-bottom: 1px solid var(--lamad-border, rgba(99,102,241,0.12)); margin-bottom: 0.5rem; }
    .project-name { font-weight: 600; font-size: 0.9rem; }
    .sidebar-nav { list-style: none; padding: 0; margin: 0; flex: 1; }
    .sidebar-nav li { padding: 0.6rem 1rem; font-size: 0.85rem; cursor: pointer; border-radius: 0 6px 6px 0; margin-right: 0.5rem; }
    .sidebar-nav li.active { background: var(--lamad-accent-primary, #6366f1); color: white; }
    .sidebar-nav li a { color: var(--lamad-text-secondary, #e2e8f0); text-decoration: none; display: block; }
    .new-project { padding: 0.75rem 1rem; font-size: 0.8rem; color: var(--lamad-accent-primary, #6366f1); text-decoration: none; }

    .tasks-main { flex: 1; padding: 1.5rem; overflow-y: auto; }
    .section-title { font-size: 1.25rem; margin-bottom: 1.25rem; }
    .interval-group { margin-bottom: 2rem; }
    .interval-heading { font-size: 0.85rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--lamad-text-muted, #64748b); margin-bottom: 0.5rem; }
    .task-row {
      display: flex; align-items: center; gap: 0.75rem;
      padding: 0.6rem 0.75rem;
      border-radius: 6px;
      border: 1px solid var(--lamad-border, rgba(99,102,241,0.1));
      margin-bottom: 0.4rem;
      background: var(--lamad-surface, rgba(30,30,46,0.5));
    }
    .task-row:hover { border-color: rgba(99,102,241,0.3); }
    .check-btn { background: none; border: 2px solid var(--lamad-border, rgba(99,102,241,0.3)); border-radius: 50%; width: 22px; height: 22px; cursor: pointer; color: var(--lamad-text-muted, #64748b); font-size: 0.9rem; padding: 0; flex-shrink: 0; }
    .check-btn:hover { border-color: var(--lamad-accent-primary, #6366f1); color: var(--lamad-accent-primary, #6366f1); }
    .task-info { flex: 1; display: flex; flex-direction: column; gap: 2px; }
    .task-title { font-size: 0.875rem; }
    .task-next { font-size: 0.75rem; color: var(--lamad-text-muted, #64748b); }
    .task-badges { display: flex; gap: 4px; font-size: 0.8rem; }
    .empty-group, .no-tasks { color: var(--lamad-text-muted, #64748b); font-size: 0.875rem; padding: 0.5rem; }
  `],
})
export class TaskListComponent implements OnInit {
  private route = inject(ActivatedRoute);
  private api = inject(AvodahApiService);

  project: ContentNode | null = null;
  cadenceStories: ContentNode[] = [];
  readonly groups: IntervalGroup[] = ['daily', 'weekly', 'monthly', 'custom'];

  async ngOnInit(): Promise<void> {
    const projectId = this.route.snapshot.params['id'];
    const projects = await this.api.getProjects();
    this.project = projects.find(p => p.id === projectId) ?? null;
    const all = await this.api.getStoriesForProject(projectId);
    this.cadenceStories = all.filter(s => !!parseWorkStoryMeta(s.metadata as Record<string, unknown>).cadence);
  }

  storiesInGroup(interval: IntervalGroup): ContentNode[] {
    return this.cadenceStories.filter(s =>
      parseWorkStoryMeta(s.metadata as Record<string, unknown>).cadence?.interval === interval
    );
  }

  nextOccurrence(story: ContentNode): string {
    const cadence = parseWorkStoryMeta(story.metadata as Record<string, unknown>).cadence;
    return cadence?.nextOccurrence ?? '—';
  }

  isOnExchange(story: ContentNode): boolean {
    return parseWorkStoryMeta(story.metadata as Record<string, unknown>).visibility === 'exchange';
  }

  hasGates(story: ContentNode): boolean {
    const gates = parseWorkStoryMeta(story.metadata as Record<string, unknown>).attestationGates;
    return Array.isArray(gates) && gates.length > 0;
  }
}
```

### Step 2: Verify compilation

```bash
cd app/elohim-app && pnpm exec tsc --noEmit
```

Expected: no errors.

### Step 3: Commit

```bash
git add app/elohim-app/src/app/avodah/components/task-list/
git commit -m "feat(avodah): add TaskListComponent grouped by cadence interval"
```

---

## Task 9: Project List View

### Files
- Create: `app/elohim-app/src/app/avodah/components/project-list/project-list.component.ts`

### Step 1: Create project-list.component.ts

```typescript
// project-list.component.ts
import { Component, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router } from '@angular/router';
import { ContentNode } from '@elohim/content-node.model';
import { AvodahApiService } from '../../services/avodah-api.service';

@Component({
  selector: 'app-project-list',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="project-list">
      <div class="list-header">
        <h1>Projects</h1>
        <button class="btn-new">+ New Project</button>
      </div>

      <div class="projects-grid">
        <div class="project-card" *ngFor="let p of projects" (click)="openProject(p)">
          <div class="project-card-header">
            <span class="project-title">{{ p.title }}</span>
            <span class="project-visibility">{{ projectMeta(p).visibility }}</span>
          </div>
          <p class="project-desc">{{ p.description }}</p>
          <div class="project-tags">
            <span class="tag" *ngFor="let tag of p.tags">#{{ tag }}</span>
          </div>
          <div class="project-actions">
            <button (click)="$event.stopPropagation(); navigate(p, 'board')">▦ Board</button>
            <button (click)="$event.stopPropagation(); navigate(p, 'backlog')">≡ Backlog</button>
            <button (click)="$event.stopPropagation(); navigate(p, 'tasks')">↺ Tasks</button>
          </div>
        </div>
      </div>
    </div>
  `,
  styles: [`
    .project-list { padding: 2rem; max-width: 1200px; margin: 0 auto; }
    .list-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1.5rem; }
    h1 { font-size: 1.5rem; }
    .btn-new { background: var(--lamad-accent-primary, #6366f1); color: white; border: none; padding: 0.5rem 1.25rem; border-radius: 6px; cursor: pointer; font-size: 0.875rem; }
    .projects-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 1rem; }
    .project-card {
      background: var(--lamad-surface, rgba(30,30,46,0.9));
      border: 1px solid var(--lamad-border, rgba(99,102,241,0.15));
      border-radius: 10px; padding: 1.25rem; cursor: pointer;
      transition: border-color 0.15s, transform 0.1s;
    }
    .project-card:hover { border-color: var(--lamad-accent-primary, #6366f1); transform: translateY(-2px); }
    .project-card-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 0.5rem; }
    .project-title { font-weight: 600; font-size: 1rem; }
    .project-visibility { font-size: 0.7rem; color: var(--lamad-text-muted, #64748b); background: rgba(99,102,241,0.1); padding: 2px 6px; border-radius: 4px; }
    .project-desc { font-size: 0.85rem; color: var(--lamad-text-muted, #64748b); margin: 0 0 0.75rem; }
    .project-tags { display: flex; gap: 4px; flex-wrap: wrap; margin-bottom: 0.75rem; }
    .tag { font-size: 0.7rem; color: var(--lamad-text-muted, #64748b); }
    .project-actions { display: flex; gap: 0.5rem; }
    .project-actions button {
      flex: 1; background: transparent; border: 1px solid var(--lamad-border, rgba(99,102,241,0.2));
      color: var(--lamad-text-secondary, #e2e8f0); padding: 0.3rem 0; border-radius: 6px;
      font-size: 0.75rem; cursor: pointer;
    }
    .project-actions button:hover { background: rgba(99,102,241,0.1); }
  `],
})
export class ProjectListComponent implements OnInit {
  private api = inject(AvodahApiService);
  private router = inject(Router);

  projects: ContentNode[] = [];

  async ngOnInit(): Promise<void> {
    this.projects = await this.api.getProjects();
  }

  projectMeta(p: ContentNode) {
    return (p.metadata ?? {}) as { visibility?: string };
  }

  openProject(p: ContentNode): void {
    this.navigate(p, 'board');
  }

  navigate(p: ContentNode, view: string): void {
    this.router.navigate(['/avodah/projects', p.id, view]);
  }
}
```

### Step 2: Verify full compilation and navigate to /avodah/projects

```bash
cd app/elohim-app && pnpm exec tsc --noEmit
```

Expected: no errors.

### Step 3: Run all avodah tests

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "avodah"
```

Expected: all passing.

### Step 4: Commit

```bash
git add app/elohim-app/src/app/avodah/components/project-list/
git commit -m "feat(avodah): add ProjectListComponent with project cards and view navigation"
```

---

## Task 10: Smoke Test the Full Pillar

### Step 1: Start the dev server

```bash
cd app/elohim-app && pnpm start
```

### Step 2: Navigate to each route and verify no console errors

| URL | Expected |
|-----|----------|
| `/avodah` | Home page with "View Projects" link |
| `/avodah/projects` | Project grid with "Household" card |
| `/avodah/projects/proj-household-2026/board` | Kanban with 5 columns; stories in Backlog, To Do, In Progress |
| `/avodah/projects/proj-household-2026/backlog` | Table with 3 stories; filter dropdowns work |
| `/avodah/projects/proj-household-2026/tasks` | Weekly group with 2 recurring stories |

### Step 3: Final commit

```bash
git add .
git commit -m "feat(avodah): complete MVP pillar — kanban, backlog, task list, mock data"
```

---

## What's Not In This Plan (Future Work)

- **Story detail slide-out / full page** — click a card to open
- **Drag-and-drop** on the kanban board — requires CDK DragDrop integration
- **Create / edit story form** — inline or modal
- **Shefa exchange publish** — visibility toggle wired to ServiceRequest creation
- **Lamad attestation gate display** — query mastery service at accept-time
- **Real backend** — replace mock data with storage API calls
- **Cadence advance engine** — service-layer cron to advance nextOccurrence and reset status
- **Elohim agent hooks** — visibility promotion on life event detection
- **A2O scenario coverage** — `genesis/a2o/features/avodah/` feature files
