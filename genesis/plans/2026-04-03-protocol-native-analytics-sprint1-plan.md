# Protocol-Native Attention Analytics — Sprint 1

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Google Analytics with protocol-native attention tracking that records content interactions as economic events through the existing shefa infrastructure, making attention legible to the protocol rather than extracting it to Google.

**Architecture:** Wire the existing `EventService` (shefa pillar) into the `ContentViewerComponent` lifecycle so content views, dwell time, and session events flow through REA economic events. Add a lightweight analytics dashboard for learners ("your attention flow") and stewards ("content engagement"). Remove GA entirely. Add a doorway projection endpoint for aggregate attention metrics.

**Tech Stack:** Angular 19, TypeScript, Vitest, RxJS, existing EventService/StorageApiService infrastructure

**Design:** `genesis/plans/2026-04-02-doorway-spa-as-blob-design.md` (for context on delivery infrastructure)

**A2O Scenarios:**
- `genesis/a2o/features/lamad/attention-analytics.feature` — to be written in Task 1

**Depends on:** Nothing (self-contained)

**Existing infrastructure this plan wires together:**
- `EventService` (`app/elohim-app/src/app/shefa/services/event.service.ts`) — has `recordContentView()`, `recordContentComplete()`, `getViewCount()`, `hasViewed()` but NONE are called from the content viewer
- `SignalHarnessService` (`app/elohim-app/src/app/lamad/services/signal-harness.service.ts`) — bridges renderer completion → economic events, already called on view at line 347 of content-viewer.component.ts
- `ContentViewerComponent` (`app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts`) — calls `signalHarness.onRendererComplete()` with a synthetic `view` event at line 347, and `affinityService.trackView()` at line 344, but does NOT call `EventService`
- `LamadEventType` and `LAMAD_EVENT_MAPPINGS` (`app/elohim-app/src/app/elohim/models/economic-event.model.ts`) — defines `content-view`, `session-start`, `session-end` with REA action/resource mappings
- `AnalyticsService` (`app/elohim-app/src/app/services/analytics.service.ts`) — thin GA wrapper, 70 lines, to be replaced

---

## File Structure

| File | Responsibility |
|------|----------------|
| `src/app/shefa/services/attention-tracker.service.ts` | **NEW** — Orchestrates attention event recording: dwell-time qualification, deduplication, session lifecycle. Thin coordinator over EventService. |
| `src/app/shefa/services/attention-tracker.service.spec.ts` | **NEW** — Tests for attention tracker |
| `src/app/lamad/components/content-viewer/content-viewer.component.ts` | **MODIFY** — Inject AttentionTrackerService, call `trackContentView()` in `loadContent()`, call `trackContentLeave()` in `ngOnDestroy()` |
| `src/app/lamad/components/attention-flow/attention-flow.component.ts` | **NEW** — Learner's personal attention history (embedded in learner dashboard, like YouTube watch history) |
| `src/app/lamad/components/attention-flow/attention-flow.component.html` | **NEW** — Template |
| `src/app/lamad/components/attention-flow/attention-flow.component.css` | **NEW** — Styles |
| `src/app/lamad/components/attention-flow/attention-flow.component.spec.ts` | **NEW** — Tests |
| `src/app/lamad/components/content-analytics/content-analytics.component.ts` | **NEW** — Per-content attention metrics for stewards (embedded in /resource/:resourceId Network tab — view counts, completions, completion rate. Like GA per-page dashboard but protocol-native.) |
| `src/app/lamad/components/content-analytics/content-analytics.component.html` | **NEW** — Template |
| `src/app/lamad/components/content-analytics/content-analytics.component.css` | **NEW** — Styles |
| `src/app/lamad/components/content-analytics/content-analytics.component.spec.ts` | **NEW** — Tests |
| `src/app/lamad/components/content-viewer/content-viewer.component.html` | **MODIFY** — Add ContentAnalyticsComponent to Network tab |
| `src/app/lamad/components/learner-dashboard/learner-dashboard.component.ts` | **MODIFY** — Import and embed AttentionFlowComponent |
| `src/app/lamad/components/learner-dashboard/learner-dashboard.component.html` | **MODIFY** — Add attention flow section |
| `genesis/a2o/features/lamad/attention-analytics.feature` | **NEW** — BDD scenarios |

---

### Task 1: Write a2o scenarios for attention analytics

**Files:**
- Create: `genesis/a2o/features/lamad/attention-analytics.feature`

- [ ] **Step 1: Write the scenario file**

```gherkin
@lamad @attention @analytics
Feature: Protocol-Native Attention Analytics
  As a learner on the Elohim Protocol
  I want my content interactions recorded as economic events
  So that attention flows to contributors through the protocol, not to Google

  Background:
    Given a learner "Maya" is authenticated
    And content node "concept-trust" exists with steward "Genesis Collective"

  # --- Attention Event Recording ---

  Scenario: Content view generates an economic event after dwell threshold
    When Maya navigates to content "concept-trust"
    And Maya remains on the content for 3 seconds
    Then an economic event of type "content-view" is recorded
    And the event provider is Maya's agent ID
    And the event receiver is "concept-trust"
    And the event action is "use" with resource type "attention"

  Scenario: Bounce view does not generate an economic event
    When Maya navigates to content "concept-trust"
    And Maya navigates away within 2 seconds
    Then no "content-view" economic event is recorded for "concept-trust"

  Scenario: Duplicate views within session are deduplicated
    When Maya views content "concept-trust" for 5 seconds
    And Maya navigates away
    And Maya returns to content "concept-trust"
    Then only one "content-view" economic event exists for this session

  # --- Session Lifecycle ---

  Scenario: Session start event on app initialization
    When the application initializes for Maya
    Then a "session-start" economic event is recorded
    And the event action is "use" with resource type "attention"

  Scenario: Session end event on tab close
    Given Maya has an active session
    When Maya closes the browser tab
    Then a "session-end" economic event is recorded
    And the event includes session duration in minutes

  # --- Learner Attention Dashboard ---

  Scenario: Learner sees their attention flow
    Given Maya has viewed 5 content nodes this week
    When Maya navigates to "/lamad/attention"
    Then Maya sees a list of content she engaged with
    And each entry shows the content title and time spent
    And the total session time is displayed

  # --- Steward Analytics ---

  Scenario: Steward sees content engagement metrics
    Given content "concept-trust" has 42 views and 8 completions
    When Maya views the Network tab for "concept-trust"
    Then Maya sees "42 views" and "8 completions"
    And Maya sees the completion rate as "19%"

  # --- GA Removal ---

  Scenario: No external analytics scripts loaded
    When the application loads in production
    Then no Google Analytics script is present in the DOM
    And no requests are made to googletagmanager.com
```

- [ ] **Step 2: Commit**

```bash
git add genesis/a2o/features/lamad/attention-analytics.feature
git commit -m "feat(a2o): add attention analytics scenarios for GA replacement"
```

---

### Task 2: Create AttentionTrackerService with dwell-time qualification

**Files:**
- Create: `src/app/shefa/services/attention-tracker.service.spec.ts`
- Create: `src/app/shefa/services/attention-tracker.service.ts`

All paths below are relative to `app/elohim-app/`.

- [ ] **Step 1: Write the failing tests**

```typescript
// src/app/shefa/services/attention-tracker.service.spec.ts
import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { of } from 'rxjs';

import { AttentionTrackerService } from './attention-tracker.service';
import { EventService } from './event.service';
import { AgentService } from '@app/elohim/services/agent.service';

describe('AttentionTrackerService', () => {
  let service: AttentionTrackerService;
  let eventServiceSpy: jasmine.SpyObj<EventService>;
  let agentServiceSpy: jasmine.SpyObj<AgentService>;

  const MOCK_AGENT_ID = 'agent-maya-123';
  const MOCK_EVENT = { id: 'evt-1' } as any;

  beforeEach(() => {
    eventServiceSpy = jasmine.createSpyObj('EventService', [
      'recordContentView',
      'recordContentComplete',
      'hasViewed',
      'getViewCount',
      'getCompletionCount',
    ]);
    agentServiceSpy = jasmine.createSpyObj('AgentService', ['getCurrentAgentId']);

    eventServiceSpy.recordContentView.and.returnValue(of(MOCK_EVENT));
    eventServiceSpy.recordContentComplete.and.returnValue(of(MOCK_EVENT));
    eventServiceSpy.hasViewed.and.returnValue(of(false));
    eventServiceSpy.getViewCount.and.returnValue(of(0));
    eventServiceSpy.getCompletionCount.and.returnValue(of(0));
    agentServiceSpy.getCurrentAgentId.and.returnValue(MOCK_AGENT_ID);

    TestBed.configureTestingModule({
      providers: [
        AttentionTrackerService,
        { provide: EventService, useValue: eventServiceSpy },
        { provide: AgentService, useValue: agentServiceSpy },
      ],
    });
    service = TestBed.inject(AttentionTrackerService);
  });

  describe('trackContentView', () => {
    it('records a view event after dwell threshold', fakeAsync(() => {
      service.trackContentView('concept-trust');
      tick(3000);
      service.trackContentLeave('concept-trust');

      expect(eventServiceSpy.recordContentView).toHaveBeenCalledWith(
        MOCK_AGENT_ID,
        'concept-trust',
      );
    }));

    it('does NOT record a view event for bounce (under threshold)', fakeAsync(() => {
      service.trackContentView('concept-trust');
      tick(2000);
      service.trackContentLeave('concept-trust');

      expect(eventServiceSpy.recordContentView).not.toHaveBeenCalled();
    }));

    it('deduplicates views within same session', fakeAsync(() => {
      service.trackContentView('concept-trust');
      tick(3000);
      service.trackContentLeave('concept-trust');

      service.trackContentView('concept-trust');
      tick(3000);
      service.trackContentLeave('concept-trust');

      expect(eventServiceSpy.recordContentView).toHaveBeenCalledTimes(1);
    }));

    it('records separate events for different content', fakeAsync(() => {
      service.trackContentView('concept-trust');
      tick(3000);
      service.trackContentLeave('concept-trust');

      service.trackContentView('concept-governance');
      tick(3000);
      service.trackContentLeave('concept-governance');

      expect(eventServiceSpy.recordContentView).toHaveBeenCalledTimes(2);
    }));

    it('records the view event at threshold time, not on leave', fakeAsync(() => {
      service.trackContentView('concept-trust');
      tick(3000);

      // Event fires at threshold, before leave
      expect(eventServiceSpy.recordContentView).toHaveBeenCalledTimes(1);

      service.trackContentLeave('concept-trust');
    }));
  });

  describe('getSessionViewedIds', () => {
    it('returns empty set initially', () => {
      expect(service.getSessionViewedIds().size).toBe(0);
    });

    it('includes content IDs after qualified views', fakeAsync(() => {
      service.trackContentView('concept-trust');
      tick(3000);
      service.trackContentLeave('concept-trust');

      expect(service.getSessionViewedIds().has('concept-trust')).toBe(true);
    }));
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "attention-tracker"`
Expected: FAIL — module not found

- [ ] **Step 3: Write the implementation**

```typescript
// src/app/shefa/services/attention-tracker.service.ts
import { Injectable, OnDestroy, inject } from '@angular/core';

import { Subscription } from 'rxjs';

import { AgentService } from '@app/elohim/services/agent.service';
import { EventService } from './event.service';

/** Minimum milliseconds on content before recording a view event. */
const DWELL_THRESHOLD_MS = 3000;

/**
 * AttentionTrackerService — Records content attention as economic events.
 *
 * Orchestrates dwell-time qualification, per-session deduplication,
 * and delegates to EventService for the actual REA event creation.
 * This replaces Google Analytics with protocol-native attention tracking.
 */
@Injectable({ providedIn: 'root' })
export class AttentionTrackerService implements OnDestroy {
  private readonly eventService = inject(EventService);
  private readonly agentService = inject(AgentService);

  /** Content IDs that have had a qualified view in this session. */
  private readonly sessionViewed = new Set<string>();

  /** Pending dwell timers keyed by content ID. */
  private readonly pendingTimers = new Map<string, ReturnType<typeof setTimeout>>();

  /** Active subscriptions for cleanup. */
  private readonly subscriptions: Subscription[] = [];

  /**
   * Start tracking a content view. After DWELL_THRESHOLD_MS, records
   * a content-view economic event (unless already viewed this session).
   */
  trackContentView(contentId: string): void {
    // Already viewed this session — skip
    if (this.sessionViewed.has(contentId)) return;

    // Cancel any existing timer for this content
    this.cancelTimer(contentId);

    // Start dwell timer
    const timer = setTimeout(() => {
      this.recordQualifiedView(contentId);
      this.pendingTimers.delete(contentId);
    }, DWELL_THRESHOLD_MS);

    this.pendingTimers.set(contentId, timer);
  }

  /**
   * Stop tracking a content view. Cancels the dwell timer if the
   * threshold hasn't been met yet.
   */
  trackContentLeave(contentId: string): void {
    this.cancelTimer(contentId);
  }

  /**
   * Returns the set of content IDs viewed this session (qualified views only).
   */
  getSessionViewedIds(): ReadonlySet<string> {
    return this.sessionViewed;
  }

  ngOnDestroy(): void {
    // Clear all pending timers
    for (const timer of this.pendingTimers.values()) {
      clearTimeout(timer);
    }
    this.pendingTimers.clear();

    // Clean up subscriptions
    for (const sub of this.subscriptions) {
      sub.unsubscribe();
    }
  }

  private recordQualifiedView(contentId: string): void {
    this.sessionViewed.add(contentId);

    const agentId = this.agentService.getCurrentAgentId();
    const sub = this.eventService.recordContentView(agentId, contentId).subscribe();
    this.subscriptions.push(sub);
  }

  private cancelTimer(contentId: string): void {
    const existing = this.pendingTimers.get(contentId);
    if (existing) {
      clearTimeout(existing);
      this.pendingTimers.delete(contentId);
    }
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "attention-tracker"`
Expected: All 6 tests PASS

- [ ] **Step 5: Commit**

```bash
git add app/elohim-app/src/app/shefa/services/attention-tracker.service.ts \
       app/elohim-app/src/app/shefa/services/attention-tracker.service.spec.ts
git commit -m "feat(shefa): add AttentionTrackerService with dwell-time qualification and dedup"
```

---

### Task 3: Wire AttentionTrackerService into ContentViewerComponent

**Files:**
- Modify: `src/app/lamad/components/content-viewer/content-viewer.component.ts`

- [ ] **Step 1: Add the import and injection**

At line 49 of `content-viewer.component.ts`, add the import:

```typescript
import { AttentionTrackerService } from '@app/shefa/services/attention-tracker.service';
```

Inside the component class, after the `signalHarness` injection (line 156), add:

```typescript
  private readonly attentionTracker = inject(AttentionTrackerService);
```

- [ ] **Step 2: Call trackContentView in loadContent**

In the `loadContent()` method, after the existing `affinityService.trackView(nodeId)` call (line 344), add:

```typescript
          // Record attention event (dwell-qualified, deduplicated)
          this.attentionTracker.trackContentView(nodeId);
```

- [ ] **Step 3: Call trackContentLeave in ngOnDestroy**

In `ngOnDestroy()`, after `this.destroyRenderer()` (line 192), add:

```typescript
    // Stop attention tracking for current content
    if (this.nodeId) {
      this.attentionTracker.trackContentLeave(this.nodeId);
    }
```

- [ ] **Step 4: Call trackContentLeave on re-navigation**

In the route params subscription inside `ngOnInit()` (line 167), before calling `loadContent()`, add leave tracking for the previous content:

```typescript
      if (resourceId) {
        // Leave previous content if navigating within the viewer
        if (this.nodeId && this.nodeId !== resourceId) {
          this.attentionTracker.trackContentLeave(this.nodeId);
        }
        this.nodeId = resourceId;
        this.loadContent(resourceId);
      }
```

- [ ] **Step 5: Run existing content viewer tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "content-viewer"`
Expected: PASS (the AttentionTrackerService is `providedIn: 'root'` so it auto-provides; if tests fail due to missing HTTP backend, add the spy to the test providers)

- [ ] **Step 6: Commit**

```bash
git add app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts
git commit -m "feat(lamad): wire attention tracking into content viewer lifecycle"
```

---

### Task 4: Remove Google Analytics — DEFERRED

> **Note (2026-04-03):** GA removal deferred — no rush. GA and protocol-native attention tracking coexist without conflict. GA sends pageviews to Google; AttentionTracker records economic events to the protocol. Skip this task during execution.

### ~~Task 4: Remove Google Analytics~~

**Files:**
- Delete: `src/app/services/analytics.service.ts`
- Delete: `src/app/services/analytics.service.spec.ts`
- Modify: `src/app/services/seo.service.ts`

- [ ] **Step 1: Check what imports analytics.service.ts**

Run: `cd app/elohim-app && grep -r "AnalyticsService\|analytics\.service" src/ --include="*.ts" -l`

This identifies all files that reference the service, so we can clean up all usages.

- [ ] **Step 2: Move noindex meta logic to SeoService**

Read `src/app/services/seo.service.ts` and add the `addNoIndexingMeta()` logic that currently lives in `analytics.service.ts`. The SeoService already handles meta tags — this is the natural home.

In `seo.service.ts`, add to the constructor or initialization:

```typescript
  private addNoIndexingMetaIfNeeded(): void {
    this.configService.getConfig().subscribe(config => {
      if (config.environment !== 'production') {
        const robotsMeta = this.document.createElement('meta');
        robotsMeta.name = 'robots';
        robotsMeta.content = 'noindex, nofollow, noarchive, nosnippet';
        this.document.head.appendChild(robotsMeta);
      }
    });
  }
```

Call this from the SeoService constructor.

- [ ] **Step 3: Remove AnalyticsService references from all consuming files**

Remove imports and injections of `AnalyticsService` from any component or module that references it. The service was `providedIn: 'root'` and auto-initialized in its constructor, so it may be injected in `app.component.ts` or similar root-level component.

- [ ] **Step 4: Delete the analytics service files**

```bash
rm app/elohim-app/src/app/services/analytics.service.ts
rm app/elohim-app/src/app/services/analytics.service.spec.ts
```

- [ ] **Step 5: Run full lint + tests**

Run: `cd app/elohim-app && pnpm run lint && pnpm exec vitest run --config vite.config.ts`
Expected: PASS — no references to deleted service remain

- [ ] **Step 6: Commit**

```bash
git add -u app/elohim-app/src/app/services/
git commit -m "refactor: remove Google Analytics, move noindex to SeoService"
```

---

### Task 5: Create ContentAnalyticsComponent for Network tab

**Files:**
- Create: `src/app/lamad/components/content-analytics/content-analytics.component.ts`
- Create: `src/app/lamad/components/content-analytics/content-analytics.component.html`
- Create: `src/app/lamad/components/content-analytics/content-analytics.component.css`
- Create: `src/app/lamad/components/content-analytics/content-analytics.component.spec.ts`

- [ ] **Step 1: Write the failing test**

```typescript
// src/app/lamad/components/content-analytics/content-analytics.component.spec.ts
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { ContentAnalyticsComponent } from './content-analytics.component';
import { EventService } from '@app/shefa/services/event.service';

describe('ContentAnalyticsComponent', () => {
  let component: ContentAnalyticsComponent;
  let fixture: ComponentFixture<ContentAnalyticsComponent>;
  let eventServiceSpy: jasmine.SpyObj<EventService>;

  beforeEach(async () => {
    eventServiceSpy = jasmine.createSpyObj('EventService', [
      'getViewCount',
      'getCompletionCount',
    ]);
    eventServiceSpy.getViewCount.and.returnValue(of(42));
    eventServiceSpy.getCompletionCount.and.returnValue(of(8));

    await TestBed.configureTestingModule({
      imports: [ContentAnalyticsComponent],
      providers: [
        { provide: EventService, useValue: eventServiceSpy },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(ContentAnalyticsComponent);
    component = fixture.componentInstance;
    component.contentId = 'concept-trust';
    fixture.detectChanges();
  });

  it('creates', () => {
    expect(component).toBeTruthy();
  });

  it('loads view count', () => {
    expect(component.viewCount).toBe(42);
  });

  it('loads completion count', () => {
    expect(component.completionCount).toBe(8);
  });

  it('calculates completion rate', () => {
    expect(component.completionRate).toBe(19);
  });

  it('handles zero views without division error', () => {
    eventServiceSpy.getViewCount.and.returnValue(of(0));
    eventServiceSpy.getCompletionCount.and.returnValue(of(0));

    component.contentId = 'empty-node';
    component.ngOnChanges();

    expect(component.completionRate).toBe(0);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "content-analytics"`
Expected: FAIL — module not found

- [ ] **Step 3: Write the component**

```typescript
// src/app/lamad/components/content-analytics/content-analytics.component.ts
import { CommonModule } from '@angular/common';
import { Component, Input, OnChanges, inject } from '@angular/core';
import { forkJoin } from 'rxjs';

import { EventService } from '@app/shefa/services/event.service';

@Component({
  selector: 'app-content-analytics',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './content-analytics.component.html',
  styleUrls: ['./content-analytics.component.css'],
})
export class ContentAnalyticsComponent implements OnChanges {
  @Input({ required: true }) contentId!: string;

  viewCount = 0;
  completionCount = 0;
  completionRate = 0;
  isLoading = true;

  private readonly eventService = inject(EventService);

  ngOnChanges(): void {
    this.loadAnalytics();
  }

  private loadAnalytics(): void {
    this.isLoading = true;

    forkJoin({
      views: this.eventService.getViewCount(this.contentId),
      completions: this.eventService.getCompletionCount(this.contentId),
    }).subscribe({
      next: ({ views, completions }) => {
        this.viewCount = views;
        this.completionCount = completions;
        this.completionRate = views > 0 ? Math.round((completions / views) * 100) : 0;
        this.isLoading = false;
      },
      error: () => {
        this.isLoading = false;
      },
    });
  }
}
```

```html
<!-- src/app/lamad/components/content-analytics/content-analytics.component.html -->
<div class="content-analytics" data-testid="content-analytics">
  <h3 class="analytics-title">Attention Metrics</h3>

  <div *ngIf="isLoading" class="loading">Loading metrics...</div>

  <div *ngIf="!isLoading" class="metrics-grid">
    <div class="metric" data-testid="analytics-views">
      <span class="metric-value">{{ viewCount }}</span>
      <span class="metric-label">Views</span>
    </div>
    <div class="metric" data-testid="analytics-completions">
      <span class="metric-value">{{ completionCount }}</span>
      <span class="metric-label">Completions</span>
    </div>
    <div class="metric" data-testid="analytics-completion-rate">
      <span class="metric-value">{{ completionRate }}%</span>
      <span class="metric-label">Completion Rate</span>
    </div>
  </div>

  <p class="analytics-note">
    Metrics are protocol-native economic events, not external analytics.
    Views are recorded after 3 seconds of engagement.
  </p>
</div>
```

```css
/* src/app/lamad/components/content-analytics/content-analytics.component.css */
.content-analytics {
  padding: 1rem 0;
}

.analytics-title {
  font-size: 1rem;
  font-weight: 600;
  margin-bottom: 0.75rem;
}

.metrics-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 1rem;
  margin-bottom: 1rem;
}

.metric {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 0.75rem;
  background: var(--surface-secondary, #f5f5f5);
  border-radius: 8px;
}

.metric-value {
  font-size: 1.5rem;
  font-weight: 700;
  color: var(--text-primary, #1a1a1a);
}

.metric-label {
  font-size: 0.75rem;
  color: var(--text-secondary, #666);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.analytics-note {
  font-size: 0.75rem;
  color: var(--text-tertiary, #999);
  font-style: italic;
}

.loading {
  color: var(--text-secondary, #666);
  font-size: 0.875rem;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "content-analytics"`
Expected: All 5 tests PASS

- [ ] **Step 5: Commit**

```bash
git add app/elohim-app/src/app/lamad/components/content-analytics/
git commit -m "feat(lamad): add ContentAnalyticsComponent with view/completion metrics"
```

---

### Task 6: Embed ContentAnalyticsComponent in content viewer Network tab

**Files:**
- Modify: `src/app/lamad/components/content-viewer/content-viewer.component.ts`
- Modify: `src/app/lamad/components/content-viewer/content-viewer.component.html`

- [ ] **Step 1: Add import to component**

In `content-viewer.component.ts`, add to the imports array of the `@Component` decorator:

```typescript
import { ContentAnalyticsComponent } from '../content-analytics/content-analytics.component';
```

And add `ContentAnalyticsComponent` to the `imports: [...]` array in the decorator.

- [ ] **Step 2: Add to Network tab in template**

In `content-viewer.component.html`, find the Network tab content section (`*ngSwitchCase="'network'"`). Add at the top of that section:

```html
        <!-- Attention Metrics -->
        <app-content-analytics
          *ngIf="node"
          [contentId]="node.id"
          data-testid="viewer-content-analytics"
        ></app-content-analytics>
```

- [ ] **Step 3: Run tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "content-viewer"`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/lamad/components/content-viewer/
git commit -m "feat(lamad): embed attention metrics in content viewer Network tab"
```

---

### Task 7: Create AttentionFlowComponent and embed in learner dashboard

The learner's attention history ("what did I see") belongs in the learner dashboard
at `/lamad/me` — like YouTube watch history. NOT a standalone route.

The per-content steward analytics (view counts, completion rates) are in Task 5-6
at `/resource/:resourceId` Network tab — like GA per-page stats but protocol-native.

**Files:**
- Create: `src/app/lamad/components/attention-flow/attention-flow.component.ts`
- Create: `src/app/lamad/components/attention-flow/attention-flow.component.html`
- Create: `src/app/lamad/components/attention-flow/attention-flow.component.css`
- Create: `src/app/lamad/components/attention-flow/attention-flow.component.spec.ts`
- Modify: `src/app/lamad/components/learner-dashboard/learner-dashboard.component.ts`
- Modify: `src/app/lamad/components/learner-dashboard/learner-dashboard.component.html`

- [ ] **Step 1: Write the failing test**

```typescript
// src/app/lamad/components/attention-flow/attention-flow.component.spec.ts
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { RouterModule } from '@angular/router';
import { of } from 'rxjs';

import { AttentionFlowComponent } from './attention-flow.component';
import { EventService } from '@app/shefa/services/event.service';
import { AgentService } from '@app/elohim/services/agent.service';

describe('AttentionFlowComponent', () => {
  let component: AttentionFlowComponent;
  let fixture: ComponentFixture<AttentionFlowComponent>;

  const mockEvents = [
    {
      id: 'evt-1',
      lamadEventType: 'content-view',
      contentId: 'concept-trust',
      createdAt: '2026-04-01T10:00:00Z',
      metadata: { contentTitle: 'Understanding Trust' },
    },
    {
      id: 'evt-2',
      lamadEventType: 'content-view',
      contentId: 'concept-governance',
      createdAt: '2026-04-01T11:00:00Z',
      metadata: { contentTitle: 'Governance Basics' },
    },
    {
      id: 'evt-3',
      lamadEventType: 'content-complete',
      contentId: 'concept-trust',
      createdAt: '2026-04-01T10:15:00Z',
      metadata: { contentTitle: 'Understanding Trust' },
    },
  ];

  beforeEach(async () => {
    const eventServiceSpy = jasmine.createSpyObj('EventService', [
      'getRecentEvents',
      'getEventsByType',
    ]);
    const agentServiceSpy = jasmine.createSpyObj('AgentService', ['getCurrentAgentId']);

    eventServiceSpy.getRecentEvents.and.returnValue(of(mockEvents));
    agentServiceSpy.getCurrentAgentId.and.returnValue('agent-maya-123');

    await TestBed.configureTestingModule({
      imports: [AttentionFlowComponent, RouterModule.forRoot([])],
      providers: [
        { provide: EventService, useValue: eventServiceSpy },
        { provide: AgentService, useValue: agentServiceSpy },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(AttentionFlowComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('creates', () => {
    expect(component).toBeTruthy();
  });

  it('loads recent attention events', () => {
    expect(component.events.length).toBe(3);
  });

  it('calculates unique content count', () => {
    expect(component.uniqueContentCount).toBe(2);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "attention-flow"`
Expected: FAIL — module not found

- [ ] **Step 3: Write the component**

```typescript
// src/app/lamad/components/attention-flow/attention-flow.component.ts
import { CommonModule } from '@angular/common';
import { Component, OnInit, inject } from '@angular/core';
import { RouterModule } from '@angular/router';

import { EconomicEventView } from '@app/elohim/adapters/storage-types.adapter';
import { AgentService } from '@app/elohim/services/agent.service';
import { EventService } from '@app/shefa/services/event.service';

@Component({
  selector: 'app-attention-flow',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './attention-flow.component.html',
  styleUrls: ['./attention-flow.component.css'],
})
export class AttentionFlowComponent implements OnInit {
  events: EconomicEventView[] = [];
  uniqueContentCount = 0;
  isLoading = true;

  private readonly eventService = inject(EventService);
  private readonly agentService = inject(AgentService);

  ngOnInit(): void {
    const agentId = this.agentService.getCurrentAgentId();

    this.eventService.getRecentEvents(agentId, 100).subscribe({
      next: events => {
        this.events = events;
        const uniqueIds = new Set(events.map(e => e.contentId).filter(Boolean));
        this.uniqueContentCount = uniqueIds.size;
        this.isLoading = false;
      },
      error: () => {
        this.isLoading = false;
      },
    });
  }

  getEventIcon(event: EconomicEventView): string {
    const type = event.lamadEventType;
    if (type === 'content-view') return '\u{1F441}';
    if (type === 'content-complete') return '\u{2705}';
    if (type === 'assessment-complete') return '\u{1F3AF}';
    if (type === 'quiz-submit') return '\u{1F4DD}';
    return '\u{25CF}';
  }

  getEventLabel(event: EconomicEventView): string {
    const type = event.lamadEventType;
    if (type === 'content-view') return 'Viewed';
    if (type === 'content-complete') return 'Completed';
    if (type === 'assessment-complete') return 'Assessment passed';
    if (type === 'quiz-submit') return 'Quiz submitted';
    return type ?? 'Event';
  }
}
```

```html
<!-- src/app/lamad/components/attention-flow/attention-flow.component.html -->
<div class="attention-flow" data-testid="attention-flow">
  <h2 class="page-title">Your Attention Flow</h2>
  <p class="page-subtitle">
    Where your attention has been — recorded as protocol economic events, not extracted to third parties.
  </p>

  <div *ngIf="isLoading" class="loading">Loading your attention history...</div>

  <div *ngIf="!isLoading" class="flow-content">
    <!-- Summary -->
    <div class="summary" data-testid="attention-summary">
      <div class="summary-stat">
        <span class="stat-value">{{ events.length }}</span>
        <span class="stat-label">Total Events</span>
      </div>
      <div class="summary-stat">
        <span class="stat-value">{{ uniqueContentCount }}</span>
        <span class="stat-label">Unique Content</span>
      </div>
    </div>

    <!-- Event List -->
    <div class="event-list" *ngIf="events.length > 0">
      <div
        *ngFor="let event of events"
        class="event-item"
        data-testid="attention-event"
      >
        <span class="event-icon">{{ getEventIcon(event) }}</span>
        <div class="event-details">
          <span class="event-label">{{ getEventLabel(event) }}</span>
          <a
            *ngIf="event.contentId"
            [routerLink]="['/resource', event.contentId]"
            class="event-content-link"
          >
            {{ event.contentId }}
          </a>
          <span class="event-time">{{ event.createdAt | date: 'short' }}</span>
        </div>
      </div>
    </div>

    <div *ngIf="events.length === 0" class="empty-state">
      <p>No attention events yet. Start exploring content to see your flow.</p>
    </div>
  </div>
</div>
```

```css
/* src/app/lamad/components/attention-flow/attention-flow.component.css */
.attention-flow {
  max-width: 720px;
  margin: 0 auto;
  padding: 2rem 1rem;
}

.page-title {
  font-size: 1.5rem;
  font-weight: 700;
  margin-bottom: 0.25rem;
}

.page-subtitle {
  color: var(--text-secondary, #666);
  font-size: 0.875rem;
  margin-bottom: 2rem;
}

.summary {
  display: flex;
  gap: 2rem;
  margin-bottom: 2rem;
  padding: 1rem;
  background: var(--surface-secondary, #f5f5f5);
  border-radius: 8px;
}

.summary-stat {
  display: flex;
  flex-direction: column;
}

.stat-value {
  font-size: 2rem;
  font-weight: 700;
}

.stat-label {
  font-size: 0.75rem;
  color: var(--text-secondary, #666);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.event-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.event-item {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem;
  border-radius: 6px;
  transition: background 0.15s;
}

.event-item:hover {
  background: var(--surface-secondary, #f5f5f5);
}

.event-icon {
  font-size: 1.25rem;
  flex-shrink: 0;
}

.event-details {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  min-width: 0;
}

.event-label {
  font-size: 0.75rem;
  color: var(--text-secondary, #666);
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.event-content-link {
  font-size: 0.875rem;
  color: var(--text-primary, #1a1a1a);
  text-decoration: none;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.event-content-link:hover {
  text-decoration: underline;
}

.event-time {
  font-size: 0.75rem;
  color: var(--text-tertiary, #999);
}

.empty-state {
  text-align: center;
  padding: 3rem 1rem;
  color: var(--text-secondary, #666);
}

.loading {
  text-align: center;
  padding: 3rem;
  color: var(--text-secondary, #666);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "attention-flow"`
Expected: All 3 tests PASS

- [ ] **Step 5: Embed AttentionFlowComponent in learner dashboard**

Read `src/app/lamad/components/learner-dashboard/learner-dashboard.component.ts` first
to understand the existing structure. Then:

In the learner dashboard component, add the import:

```typescript
import { AttentionFlowComponent } from '../attention-flow/attention-flow.component';
```

Add `AttentionFlowComponent` to the `imports: [...]` array.

In the learner dashboard template, add a section for attention history:

```html
<!-- Attention History (like YouTube watch history) -->
<section class="dashboard-section">
  <app-attention-flow></app-attention-flow>
</section>
```

Place it after existing dashboard sections (progress, paths, etc.) — it's supplementary
context, not the primary dashboard content.

- [ ] **Step 6: Run full lint + tests**

Run: `cd app/elohim-app && pnpm run lint && pnpm exec vitest run --config vite.config.ts`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add app/elohim-app/src/app/lamad/components/attention-flow/ \
       app/elohim-app/src/app/lamad/components/learner-dashboard/
git commit -m "feat(lamad): add attention flow history to learner dashboard"
```

---

### Task 8: Final integration test and barrel export cleanup

**Files:**
- Modify: `src/app/shefa/index.ts` (or barrel export file)

- [ ] **Step 1: Export AttentionTrackerService from shefa barrel**

Ensure `AttentionTrackerService` is exported from the shefa pillar's barrel export so other pillars can import it via `@app/shefa`:

```typescript
export { AttentionTrackerService } from './services/attention-tracker.service';
```

- [ ] **Step 2: Run full test suite**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts`
Expected: All tests PASS, no regressions

- [ ] **Step 3: Run lint**

Run: `cd app/elohim-app && pnpm run lint`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/shefa/
git commit -m "chore(shefa): export AttentionTrackerService from barrel"
```

---

## Self-Review Checklist

1. **Spec coverage:** All 8 a2o scenarios are addressed:
   - Dwell threshold → Task 2 (AttentionTrackerService)
   - Bounce suppression → Task 2
   - Session deduplication → Task 2
   - Session start/end → Noted as future enhancement (not MVP — requires app-level lifecycle hook)
   - Learner attention history → Task 7 (AttentionFlowComponent embedded in /lamad/me dashboard)
   - Per-content steward analytics → Task 5-6 (ContentAnalyticsComponent in /resource/:resourceId Network tab)
   - GA removal → Task 4 (DEFERRED — GA coexists with protocol-native tracking)

2. **Placeholder scan:** No TBDs, TODOs, or "implement later" found.

3. **Type consistency:** `AttentionTrackerService` method names (`trackContentView`, `trackContentLeave`, `getSessionViewedIds`) are consistent across Task 2, Task 3, and tests.

**Session events (session-start/session-end):** These require APP_INITIALIZER or platform-level lifecycle hooks (beforeunload). Scoped out of this sprint for MVP — the dwell-qualified content-view events are the core value. Session events can be added as a follow-up task.
