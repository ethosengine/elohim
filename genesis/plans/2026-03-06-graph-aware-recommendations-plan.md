# Graph-Aware Recommendations — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire RelationshipService into PathAdaptationService so failed quiz recommendations traverse the content graph for prerequisites/reinforcements, and surface them as EPR-linked cards in the quiz result and path overview.

**Architecture:** PathAdaptationService.generateRecommendations() calls RelationshipService depth-1 lookups (PREREQUISITE, REINFORCES) for struggling concepts. A new RecommendationListComponent renders results as `<app-epr-link display="card">` with adaptation context labels. The component embeds in two surfaces: assessment-completion-summary (inline after failure) and path-overview (persistent panel).

**Tech Stack:** Angular 19 (signals, standalone components), Vitest, RxJS, existing RelationshipService + EprLinkComponent

---

### Task 1: Add graph config to PathAdaptationConfig

**Files:**
- Modify: `elohim-app/src/app/lamad/quiz-engine/services/path-adaptation.service.ts:157-184`

**Step 1: Add config fields**

In `PathAdaptationConfig` interface (line 157), add after `requireInlineBeforeMastery`:

```typescript
  /** Maximum graph traversal depth for recommendations (1 = direct relationships only) */
  maxGraphDepth: number;

  /** Relationship types to query for recommendations */
  graphRelationshipTypes: string[];
```

In `DEFAULT_CONFIG` (line 177), add:

```typescript
  maxGraphDepth: 1,
  graphRelationshipTypes: ['PREREQUISITE', 'REINFORCES'],
```

**Step 2: Run tests to verify nothing breaks**

Run: `cd /projects/elohim && pnpm exec vitest run --config elohim-app/vite.config.ts "path-adaptation.service"`
Expected: All existing tests pass (config defaults are additive)

**Step 3: Add config default test**

In `path-adaptation.service.spec.ts`, inside the "Configuration matches feature spec defaults" describe block, add:

```typescript
    it('default maxGraphDepth should be 1', () => {
      const config = service.getConfig();
      expect(config.maxGraphDepth).toBe(1);
    });

    it('default graphRelationshipTypes should include PREREQUISITE and REINFORCES', () => {
      const config = service.getConfig();
      expect(config.graphRelationshipTypes).toEqual(['PREREQUISITE', 'REINFORCES']);
    });
```

**Step 4: Run tests**

Run: `cd /projects/elohim && pnpm exec vitest run --config elohim-app/vite.config.ts "path-adaptation.service"`
Expected: All tests pass including new config tests

**Step 5: Commit**

```bash
git add elohim-app/src/app/lamad/quiz-engine/services/path-adaptation.service.ts elohim-app/src/app/lamad/quiz-engine/services/path-adaptation.service.spec.ts
git commit -m "feat(lamad): add graph config to PathAdaptationConfig

Add maxGraphDepth and graphRelationshipTypes to PathAdaptationConfig
as the seam for future ElohimAgent-driven deep graph traversal.
Defaults: depth 1, types [PREREQUISITE, REINFORCES]."
```

---

### Task 2: Wire RelationshipService into generateRecommendations

**Files:**
- Modify: `elohim-app/src/app/lamad/quiz-engine/services/path-adaptation.service.ts:1-10,675-715`
- Modify: `elohim-app/src/app/lamad/quiz-engine/services/path-adaptation.service.spec.ts`

**Step 1: Write the failing test**

In `path-adaptation.service.spec.ts`, add a new describe block after "Recommendations surface when learner struggles":

```typescript
  // ═══════════════════════════════════════════════════════════════════════════
  // Scenario: Graph-aware recommendations resolve prerequisites
  // Design: 2026-03-06-graph-aware-recommendations-design.md
  // ═══════════════════════════════════════════════════════════════════════════

  describe('Graph-aware recommendations resolve prerequisite content', () => {
    it('should look up PREREQUISITE relationships for struggling concepts', () => {
      const result = makeQuizResult({
        passed: false,
        score: 0.3,
        contentScores: [makeContentScore('concept-trust', 0.2)],
      });

      service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      expect(relationshipSpy.getRelationshipsByType).toHaveBeenCalledWith(
        'concept-trust',
        'PREREQUISITE'
      );
    });

    it('should include prerequisite content in recommendations with reason prerequisite_gap', () => {
      relationshipSpy.getRelationshipsByType.mockImplementation(
        (contentId: string, type: string) => {
          if (type === 'PREREQUISITE') {
            return of([
              {
                id: 'rel-1',
                sourceId: contentId,
                targetId: 'prereq-foundations',
                relationshipType: 'PREREQUISITE',
                confidence: 0.9,
              },
            ]);
          }
          return of([]);
        }
      );

      const result = makeQuizResult({
        passed: false,
        score: 0.3,
        contentScores: [makeContentScore('concept-trust', 0.2)],
      });

      service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      const recs = service.getRecommendations(PATH_ID, HUMAN_ID);
      const prereqRec = recs.find(r => r.contentId === 'prereq-foundations');
      expect(prereqRec).toBeDefined();
      expect(prereqRec!.reason).toBe('prerequisite_gap');
    });

    it('should include REINFORCES content with reason reinforcement', () => {
      relationshipSpy.getRelationshipsByType.mockImplementation(
        (contentId: string, type: string) => {
          if (type === 'REINFORCES') {
            return of([
              {
                id: 'rel-2',
                sourceId: contentId,
                targetId: 'alt-perspective',
                relationshipType: 'REINFORCES',
                confidence: 0.7,
              },
            ]);
          }
          return of([]);
        }
      );

      const result = makeQuizResult({
        passed: false,
        score: 0.3,
        contentScores: [makeContentScore('concept-trust', 0.2)],
      });

      service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      const recs = service.getRecommendations(PATH_ID, HUMAN_ID);
      const reinforceRec = recs.find(r => r.contentId === 'alt-perspective');
      expect(reinforceRec).toBeDefined();
      expect(reinforceRec!.reason).toBe('reinforcement');
    });

    it('should fall back to struggled_with_concept when no graph relationships exist', () => {
      // Default mock returns empty arrays
      const result = makeQuizResult({
        passed: false,
        score: 0.3,
        contentScores: [makeContentScore('concept-trust', 0.2)],
      });

      service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      const recs = service.getRecommendations(PATH_ID, HUMAN_ID);
      expect(recs.length).toBeGreaterThan(0);
      expect(recs[0].reason).toBe('struggled_with_concept');
    });

    it('should rank prerequisite_gap above reinforcement', () => {
      relationshipSpy.getRelationshipsByType.mockImplementation(
        (_contentId: string, type: string) => {
          if (type === 'PREREQUISITE') {
            return of([
              {
                id: 'rel-1',
                sourceId: 'concept-trust',
                targetId: 'prereq-foundations',
                relationshipType: 'PREREQUISITE',
                confidence: 0.7,
              },
            ]);
          }
          if (type === 'REINFORCES') {
            return of([
              {
                id: 'rel-2',
                sourceId: 'concept-trust',
                targetId: 'alt-perspective',
                relationshipType: 'REINFORCES',
                confidence: 0.9,
              },
            ]);
          }
          return of([]);
        }
      );

      const result = makeQuizResult({
        passed: false,
        score: 0.3,
        contentScores: [makeContentScore('concept-trust', 0.2)],
      });

      service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      const recs = service.getRecommendations(PATH_ID, HUMAN_ID);
      const prereqIdx = recs.findIndex(r => r.reason === 'prerequisite_gap');
      const reinforceIdx = recs.findIndex(r => r.reason === 'reinforcement');
      if (prereqIdx >= 0 && reinforceIdx >= 0) {
        expect(prereqIdx).toBeLessThan(reinforceIdx);
      }
    });
  });
```

Also update the test setup. Add to imports at top of spec file:

```typescript
import { of } from 'rxjs';
import { RelationshipService } from '@app/lamad/services/relationship.service';
```

Add `relationshipSpy` declaration alongside other spies:

```typescript
  let relationshipSpy: any;
```

In `beforeEach`, add the mock and provider:

```typescript
    relationshipSpy = {
      getRelationshipsByType: vi.fn().mockReturnValue(of([])),
      getBidirectionalRelationships: vi.fn().mockReturnValue(of([])),
    };
```

Add to providers array:

```typescript
        { provide: RelationshipService, useValue: relationshipSpy },
```

**Step 2: Run tests to verify they fail**

Run: `cd /projects/elohim && pnpm exec vitest run --config elohim-app/vite.config.ts "path-adaptation.service"`
Expected: New graph-aware tests FAIL (RelationshipService not called yet)

**Step 3: Implement graph-aware generateRecommendations**

In `path-adaptation.service.ts`:

Add import at top (after existing imports):

```typescript
import { firstValueFrom } from 'rxjs';

import { RelationshipService } from '@app/lamad/services/relationship.service';
```

Add injection in class (after existing injects, line ~221):

```typescript
  private readonly relationshipService = inject(RelationshipService);
```

Replace `generateRecommendations` method (lines 675-715) with:

```typescript
  private generateRecommendations(pathId: string, humanId: string, result: QuizResult): void {
    const state = this.getOrCreateState(pathId, humanId);
    const newRecs: ContentRecommendation[] = [];

    // Find concepts with low scores
    const strugglingConcepts = result.contentScores.filter(
      cs => cs.averageScore < this.config.recommendationThreshold
    );

    for (const contentScore of strugglingConcepts) {
      // Query content graph for prerequisite and reinforcing content
      const graphRecs = this.lookupGraphRecommendations(contentScore.contentId, contentScore.averageScore);

      if (graphRecs.length > 0) {
        newRecs.push(...graphRecs);
      } else {
        // Fallback: flag the concept itself when no graph relationships exist
        newRecs.push({
          contentId: contentScore.contentId,
          reason: 'struggled_with_concept',
          confidence: 1 - contentScore.averageScore,
          triggerContext: {
            quizType: result.type === 'mastery' ? 'mastery' : 'practice',
            conceptIds: [contentScore.contentId],
            score: contentScore.averageScore,
          },
        });
      }
    }

    // Sort: prerequisites first, then by confidence
    const reasonPriority: Record<RecommendationReason, number> = {
      prerequisite_gap: 0,
      struggled_with_concept: 1,
      reinforcement: 2,
      exploration_interest: 3,
      advanced_option: 4,
    };

    const filtered = newRecs
      .filter(r => r.confidence >= this.config.recommendationThreshold)
      .sort((a, b) => {
        const priorityDiff = (reasonPriority[a.reason] ?? 99) - (reasonPriority[b.reason] ?? 99);
        return priorityDiff !== 0 ? priorityDiff : b.confidence - a.confidence;
      })
      .slice(0, this.config.maxRecommendations);

    // Merge with existing, avoiding duplicates
    const existingIds = new Set(state.recommendations.map(r => r.contentId));
    for (const rec of filtered) {
      if (!existingIds.has(rec.contentId)) {
        state.recommendations.push(rec);
      }
    }

    // Trim to max
    state.recommendations = state.recommendations.slice(0, this.config.maxRecommendations);

    this.saveState(pathId, humanId, state);
  }

  /**
   * Look up prerequisite and reinforcing content from the content graph.
   * Uses depth-1 traversal (configurable via maxGraphDepth for future ElohimAgent integration).
   */
  private lookupGraphRecommendations(contentId: string, score: number): ContentRecommendation[] {
    const recs: ContentRecommendation[] = [];

    for (const relType of this.config.graphRelationshipTypes) {
      try {
        // Synchronous-style: subscribe and collect in same tick for localStorage-backed services
        // For HTTP-backed RelationshipService, use firstValueFrom in an async wrapper
        let relationships: { targetId: string; confidence: number; relationshipType: string }[] = [];
        this.relationshipService
          .getRelationshipsByType(contentId, relType)
          .subscribe(rels => {
            relationships = rels;
          });

        for (const rel of relationships) {
          recs.push({
            contentId: rel.targetId,
            reason: relType === 'PREREQUISITE' ? 'prerequisite_gap' : 'reinforcement',
            confidence: rel.confidence * (1 - score),
            triggerContext: {
              quizType: 'mastery',
              conceptIds: [contentId],
              score,
            },
          });
        }
      } catch {
        // Graph lookup failure is non-critical — fall through to concept-only recommendation
      }
    }

    return recs;
  }
```

**Step 4: Run tests to verify they pass**

Run: `cd /projects/elohim && pnpm exec vitest run --config elohim-app/vite.config.ts "path-adaptation.service"`
Expected: ALL tests pass including new graph-aware tests

**Step 5: Commit**

```bash
git add elohim-app/src/app/lamad/quiz-engine/services/path-adaptation.service.ts elohim-app/src/app/lamad/quiz-engine/services/path-adaptation.service.spec.ts
git commit -m "feat(lamad): wire RelationshipService into recommendation generation

Replace the TODO at line 682 with actual content graph traversal.
When a learner struggles with a concept, look up PREREQUISITE and
REINFORCES relationships to surface richer recommendations.
Prerequisites rank above reinforcements in the recommendation list.
Falls back to struggled_with_concept when no graph relationships exist."
```

---

### Task 3: Create RecommendationListComponent

**Files:**
- Create: `elohim-app/src/app/lamad/quiz-engine/components/recommendation-list/recommendation-list.component.ts`
- Create: `elohim-app/src/app/lamad/quiz-engine/components/recommendation-list/recommendation-list.component.spec.ts`
- Modify: `elohim-app/src/app/lamad/quiz-engine/components/index.ts`

**Step 1: Write the failing test**

Create `recommendation-list.component.spec.ts`:

```typescript
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { provideRouter } from '@angular/router';
import { vi } from 'vitest';

import { RecommendationListComponent } from './recommendation-list.component';
import { EprResolverService } from '@app/elohim/services/epr-resolver.service';
import type { ContentRecommendation } from '../../services/path-adaptation.service';

describe('RecommendationListComponent', () => {
  let component: RecommendationListComponent;
  let fixture: ComponentFixture<RecommendationListComponent>;

  const mockRecs: ContentRecommendation[] = [
    {
      contentId: 'prereq-foundations',
      reason: 'prerequisite_gap',
      confidence: 0.8,
      triggerContext: { quizType: 'mastery', conceptIds: ['concept-trust'], score: 0.3 },
    },
    {
      contentId: 'alt-perspective',
      reason: 'reinforcement',
      confidence: 0.7,
      triggerContext: { quizType: 'mastery', conceptIds: ['concept-trust'], score: 0.4 },
    },
  ];

  beforeEach(async () => {
    const mockResolver = {
      resolve: vi.fn().mockReturnValue({ subscribe: vi.fn() }),
      resolveEprHead: vi.fn().mockReturnValue({ subscribe: vi.fn() }),
    };

    await TestBed.configureTestingModule({
      imports: [RecommendationListComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter([]),
        { provide: EprResolverService, useValue: mockResolver },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(RecommendationListComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should render nothing when recommendations is empty', () => {
    component.recommendations = [];
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="recommendation-list"]')).toBeNull();
  });

  it('should render recommendation cards when recommendations provided', () => {
    component.recommendations = mockRecs;
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    const list = el.querySelector('[data-testid="recommendation-list"]');
    expect(list).toBeTruthy();

    const items = el.querySelectorAll('[data-testid^="recommendation-item-"]');
    expect(items.length).toBe(2);
  });

  it('should show prerequisite context label for prerequisite_gap reason', () => {
    component.recommendations = [mockRecs[0]];
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    const label = el.querySelector('[data-testid="recommendation-context-0"]');
    expect(label?.textContent).toContain('Foundation');
  });

  it('should show reinforcement context label for reinforcement reason', () => {
    component.recommendations = [mockRecs[1]];
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    const label = el.querySelector('[data-testid="recommendation-context-0"]');
    expect(label?.textContent).toContain('Another angle');
  });

  it('should emit dismiss event when dismiss button clicked', () => {
    component.recommendations = mockRecs;
    fixture.detectChanges();

    const dismissSpy = vi.fn();
    component.dismiss.subscribe(dismissSpy);

    const btn = fixture.nativeElement.querySelector(
      '[data-testid="recommendation-dismiss-0"]'
    ) as HTMLButtonElement;
    btn?.click();

    expect(dismissSpy).toHaveBeenCalledWith('prereq-foundations');
  });

  it('should render epr-link for each recommendation', () => {
    component.recommendations = mockRecs;
    fixture.detectChanges();

    const links = fixture.nativeElement.querySelectorAll('app-epr-link');
    expect(links.length).toBe(2);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `cd /projects/elohim && pnpm exec vitest run --config elohim-app/vite.config.ts "recommendation-list"`
Expected: FAIL — component doesn't exist yet

**Step 3: Create the component**

Create `recommendation-list.component.ts`:

```typescript
/**
 * RecommendationListComponent — Presentational component for content recommendations.
 *
 * Renders ContentRecommendation[] as EPR-linked cards with adaptation context labels.
 * Used in two surfaces:
 * - Assessment completion summary (inline after quiz failure)
 * - Path overview (persistent panel near locked gates)
 *
 * Each recommendation renders as an <app-epr-link display="card"> wrapped with
 * a context label explaining WHY this content is recommended.
 */

import { CommonModule } from '@angular/common';
import { Component, ChangeDetectionStrategy, Input, Output, EventEmitter } from '@angular/core';

import { EprLinkComponent } from '@app/elohim/components/epr-link/epr-link.component';

import type { ContentRecommendation, RecommendationReason } from '../../services/path-adaptation.service';

@Component({
  selector: 'app-recommendation-list',
  standalone: true,
  imports: [CommonModule, EprLinkComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (recommendations.length > 0) {
      <section class="recommendation-list" data-testid="recommendation-list">
        <h5 class="recommendation-heading" data-testid="recommendation-heading">
          {{ heading }}
        </h5>
        @for (rec of recommendations; track rec.contentId; let i = $index) {
          <div
            class="recommendation-item"
            [attr.data-testid]="'recommendation-item-' + i"
          >
            <span
              class="recommendation-context"
              [attr.data-testid]="'recommendation-context-' + i"
            >
              {{ getContextLabel(rec) }}
            </span>
            <app-epr-link
              [epr]="'epr:' + rec.contentId"
              display="card"
            ></app-epr-link>
            <button
              class="recommendation-dismiss"
              [attr.data-testid]="'recommendation-dismiss-' + i"
              (click)="dismiss.emit(rec.contentId)"
              aria-label="Dismiss recommendation"
            >
              Dismiss
            </button>
          </div>
        }
      </section>
    }
  `,
  styles: [
    `
      .recommendation-list {
        margin: 1.5rem 0;
      }

      .recommendation-heading {
        font-size: 0.9rem;
        font-weight: 600;
        color: #374151;
        margin: 0 0 0.75rem;
      }

      .recommendation-item {
        margin-bottom: 1rem;
        position: relative;
      }

      .recommendation-context {
        display: block;
        font-size: 0.8rem;
        color: #6b7280;
        margin-bottom: 0.25rem;
        font-style: italic;
      }

      .recommendation-dismiss {
        position: absolute;
        top: 0.25rem;
        right: 0.25rem;
        background: none;
        border: none;
        font-size: 0.75rem;
        color: #9ca3af;
        cursor: pointer;
        padding: 0.25rem 0.5rem;
      }
      .recommendation-dismiss:hover {
        color: #6b7280;
      }
    `,
  ],
})
export class RecommendationListComponent {
  @Input() recommendations: ContentRecommendation[] = [];
  @Input() heading = 'Strengthen Your Foundations';
  @Output() dismiss = new EventEmitter<string>();

  private readonly contextLabels: Record<RecommendationReason, string> = {
    prerequisite_gap: 'Foundation for concepts you need',
    reinforcement: 'Another angle on this topic',
    struggled_with_concept: 'Review this before retrying',
    exploration_interest: 'You might find this interesting',
    advanced_option: 'Ready for a deeper dive',
  };

  getContextLabel(rec: ContentRecommendation): string {
    return this.contextLabels[rec.reason] ?? '';
  }
}
```

**Step 4: Export from barrel**

In `elohim-app/src/app/lamad/quiz-engine/components/index.ts`, add:

```typescript
export { RecommendationListComponent } from './recommendation-list/recommendation-list.component';
```

**Step 5: Run tests**

Run: `cd /projects/elohim && pnpm exec vitest run --config elohim-app/vite.config.ts "recommendation-list"`
Expected: All tests pass

**Step 6: Commit**

```bash
git add elohim-app/src/app/lamad/quiz-engine/components/recommendation-list/ elohim-app/src/app/lamad/quiz-engine/components/index.ts
git commit -m "feat(lamad): add RecommendationListComponent with EPR links

Presentational component that renders ContentRecommendation[] as
EPR-linked cards with adaptation context labels. Each card resolves
via epr: URI for context-aware navigation and three-pillar preview."
```

---

### Task 4: Embed recommendations in AssessmentCompletionSummaryComponent

**Files:**
- Modify: `elohim-app/src/app/lamad/quiz-engine/components/assessment-completion-summary/assessment-completion-summary.component.ts`

**Step 1: Read the full component to understand template structure**

Ref: `assessment-completion-summary.component.ts` — the mastery failed section (around line 116-156) is where we embed.

**Step 2: Add imports and inputs**

Add to component imports array:

```typescript
import { RecommendationListComponent } from '../recommendation-list/recommendation-list.component';
```

Add `RecommendationListComponent` to the `imports` array in `@Component`.

Add inputs to the component class:

```typescript
  /** Active recommendations for failed mastery quizzes */
  recommendations = input<ContentRecommendation[]>([]);

  /** Emitted when a recommendation is dismissed */
  dismissRecommendation = output<string>();
```

Add import for the type:

```typescript
import type { ContentRecommendation } from '../../services/path-adaptation.service';
```

**Step 3: Add template section**

After the mastery score display block (`@if (mode() === 'mastery')` section, after the learner profile preview `}`), before the Elohim Presence Insight section, add:

```html
      <!-- Mastery: Recommendations (shown on failure) -->
      @if (mode() === 'mastery' && passed() === false && recommendations().length > 0) {
        <app-recommendation-list
          [recommendations]="recommendations()"
          (dismiss)="dismissRecommendation.emit($event)"
          data-testid="completion-recommendations"
        ></app-recommendation-list>
      }
```

**Step 4: Run existing tests**

Run: `cd /projects/elohim && pnpm exec vitest run --config elohim-app/vite.config.ts "assessment-completion-summary"`
Expected: Existing tests pass (new section only renders when recommendations provided + failed)

**Step 5: Commit**

```bash
git add elohim-app/src/app/lamad/quiz-engine/components/assessment-completion-summary/assessment-completion-summary.component.ts
git commit -m "feat(lamad): embed recommendation list in assessment completion summary

Shows graph-aware recommendations inline after mastery quiz failure.
Recommendations render as EPR-linked cards with context labels."
```

---

### Task 5: Embed recommendations in PathOverviewComponent

**Files:**
- Modify: `elohim-app/src/app/lamad/components/path-overview/path-overview.component.ts`

**Step 1: Read the component to find the right insertion point**

Ref: `path-overview.component.ts` — find where locked gate sections display.

**Step 2: Add imports**

Add to imports:

```typescript
import { RecommendationListComponent } from '@app/lamad/quiz-engine/components/recommendation-list/recommendation-list.component';
import { PathAdaptationService, type ContentRecommendation } from '@app/lamad/quiz-engine/services/path-adaptation.service';
```

Add `RecommendationListComponent` to the component's `imports` array.

**Step 3: Add recommendation state**

Add to the component class:

```typescript
  private readonly adaptationService = inject(PathAdaptationService);

  /** Active recommendations from PathAdaptationService */
  recommendations = signal<ContentRecommendation[]>([]);
```

In `loadPath()` (or wherever the main data load completes), after loading path data, add:

```typescript
    // Load active recommendations
    const humanId = this.getHumanId();
    if (humanId) {
      this.adaptationService.getRecommendations$(this.pathId, humanId).subscribe(recs => {
        this.recommendations.set(recs);
      });
    }
```

**Step 4: Add template section**

Add after the path step list / chapter list, before any footer navigation:

```html
    <!-- Active Recommendations -->
    @if (recommendations().length > 0) {
      <app-recommendation-list
        [recommendations]="recommendations()"
        heading="Recommended Content"
        (dismiss)="onDismissRecommendation($event)"
        data-testid="path-overview-recommendations"
      ></app-recommendation-list>
    }
```

Add dismiss handler method:

```typescript
  onDismissRecommendation(contentId: string): void {
    const humanId = this.getHumanId();
    if (humanId) {
      this.adaptationService.dismissRecommendation(this.pathId, humanId, contentId);
    }
  }
```

**Step 5: Run tests**

Run: `cd /projects/elohim && pnpm exec vitest run --config elohim-app/vite.config.ts "path-overview"`
Expected: Existing tests pass (recommendations signal defaults to empty, component renders nothing extra)

**Step 6: Commit**

```bash
git add elohim-app/src/app/lamad/components/path-overview/path-overview.component.ts
git commit -m "feat(lamad): embed recommendation list in path overview

Shows persistent graph-aware recommendations in the path overview.
Recommendations dismissed via the panel or cleared when the gate is passed."
```

---

### Task 6: Add A2O scenario for graph-aware recommendations

**Files:**
- Modify: `genesis/a2o/features/lamad/path-adaptation.feature`

**Step 1: Add the scenario**

After the "Layer 3: Discovery-Informed Recommendations" comment, before the existing `@discovery` scenarios, add a new layer section:

```gherkin
  # ═══════════════════════════════════════════════════════════════════════════
  # Layer 4: Graph-Aware Recommendations
  # ═══════════════════════════════════════════════════════════════════════════

  @graph-recommendation @wip
  Scenario: Failed quiz surfaces prerequisite content from content graph
    Given Matthew is on step 4 of the "Elohim Protocol" path
    And the content for step 4 has a PREREQUISITE relationship to "foundations-of-trust"
    When Matthew fails the mastery quiz for the current section with score 30%
    Then a "Strengthen Your Foundations" section should appear
    And it should contain an EPR-linked card for "foundations-of-trust"
    And the card should show context "Foundation for concepts you need"
    And the recommendation should also appear in the path overview

  @graph-recommendation @dismiss @wip
  Scenario: Dismissing a recommendation removes it from both surfaces
    Given Matthew has an active recommendation for "foundations-of-trust"
    When Matthew dismisses the recommendation
    Then the recommendation should not appear in the quiz result
    And the recommendation should not appear in the path overview

  @graph-recommendation @gate-clear @wip
  Scenario: Passing the gate clears recommendations for that section
    Given Matthew has active recommendations from a failed quiz
    When Matthew passes the mastery quiz for the section
    Then all recommendations from that section should be cleared
```

**Step 2: Commit**

```bash
git add genesis/a2o/features/lamad/path-adaptation.feature
git commit -m "feat(a2o): add graph-aware recommendation scenarios

Three new scenarios covering prerequisite content surfacing,
recommendation dismissal, and gate-clear behavior. Tagged @wip
until E2E step definitions are implemented."
```

---

### Task 7: Run full test suite and verify

**Step 1: Run all affected test files**

```bash
cd /projects/elohim && pnpm exec vitest run --config elohim-app/vite.config.ts "path-adaptation.service"
cd /projects/elohim && pnpm exec vitest run --config elohim-app/vite.config.ts "recommendation-list"
cd /projects/elohim && pnpm exec vitest run --config elohim-app/vite.config.ts "assessment-completion-summary"
```

Expected: All tests pass

**Step 2: Run lint**

```bash
cd /projects/elohim/elohim-app && pnpm run lint
```

Expected: No new lint errors

**Step 3: Fix any issues found, commit if needed**

---

## File Summary

| File | Action | Purpose |
|------|--------|---------|
| `elohim-app/.../services/path-adaptation.service.ts` | Modify | Add config fields, inject RelationshipService, implement graph lookup |
| `elohim-app/.../services/path-adaptation.service.spec.ts` | Modify | Add graph-aware recommendation tests |
| `elohim-app/.../components/recommendation-list/recommendation-list.component.ts` | Create | Presentational component with EPR link cards |
| `elohim-app/.../components/recommendation-list/recommendation-list.component.spec.ts` | Create | Component tests |
| `elohim-app/.../components/index.ts` | Modify | Export RecommendationListComponent |
| `elohim-app/.../components/assessment-completion-summary/assessment-completion-summary.component.ts` | Modify | Embed recommendations inline after failure |
| `elohim-app/.../components/path-overview/path-overview.component.ts` | Modify | Embed recommendations in overview panel |
| `genesis/a2o/features/lamad/path-adaptation.feature` | Modify | Add graph-aware recommendation scenarios |
