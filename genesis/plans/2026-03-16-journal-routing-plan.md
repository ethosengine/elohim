# Journal Routing Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire the journal "finish" event — a two-round flow where the elohim confirms intent, then generates EPR suggestion cards for filing the journal and spinning off derivative protocol artifacts. Sidecar intelligence is stubbed with keyword matching.

**Architecture:** New `JournalRoutingService` (component-scoped state machine) orchestrates the flow. JournalPageComponent becomes a state-driven orchestrator rendering JournalEditorComponent (existing), JournalConfirmComponent (new), JournalRoutingCardsComponent (new), and JournalRoutedComponent (new). Two sidecar seams (`analyzeIntent`, `generateSuggestions`) are the only things that change when inference lands.

**Tech Stack:** Angular 19 (signals, `@switch`), Vitest, existing StorageApiService for HTTP writes

**Design doc:** `genesis/plans/2026-03-16-journal-routing-design.md`

---

### Task 1: Journal Routing Models

**Files:**
- Create: `app/elohim-app/src/app/shefa/models/journal-routing.model.ts`

**Step 1: Create the model file**

```typescript
import type { ReachTier } from '@app/elohim/services/gate-interaction.service';

export type JournalRoutingState = 'writing' | 'confirming' | 'routing' | 'routed';

export type DestinationType = 'content' | 'exchange-request' | 'governance-proposal';

export type SuggestionKind = 'filing' | 'derivative';

export type SuggestionStatus = 'suggested' | 'posting' | 'posted' | 'dismissed';

export interface IntentAnalysis {
  summary: string;
  detectedTypes: DestinationType[];
  suggestedPath: string;
}

export interface RoutingSuggestion {
  id: string;
  kind: SuggestionKind;
  destinationType: 'journal-filing' | DestinationType;
  title: string;
  summary: string;
  suggestedPath: string;
  reach: ReachTier;
  contextMetadata: Record<string, unknown>;
  status: SuggestionStatus;
}
```

**Step 2: Verify no lint errors**

Run: `cd /projects/elohim && pnpm exec eslint app/elohim-app/src/app/shefa/models/journal-routing.model.ts --ext .ts`
Expected: No errors

**Step 3: Commit**

```bash
git add app/elohim-app/src/app/shefa/models/journal-routing.model.ts
git commit -m "feat(shefa): add journal routing data models"
```

---

### Task 2: JournalRoutingService — State Machine + Stubs

**Files:**
- Create: `app/elohim-app/src/app/shefa/services/journal-routing.service.ts`
- Test: `app/elohim-app/src/app/shefa/services/journal-routing.service.spec.ts`

**Step 1: Write the failing tests**

```typescript
import { TestBed } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { of } from 'rxjs';

import { JournalRoutingService } from './journal-routing.service';

describe('JournalRoutingService', () => {
  let service: JournalRoutingService;
  let mockHttp: { patch: ReturnType<typeof vi.fn>; post: ReturnType<typeof vi.fn> };

  beforeEach(() => {
    mockHttp = {
      patch: vi.fn().mockReturnValue(of({})),
      post: vi.fn().mockReturnValue(of({})),
    };

    TestBed.configureTestingModule({
      providers: [
        JournalRoutingService,
        { provide: HttpClient, useValue: mockHttp },
      ],
    });

    service = TestBed.inject(JournalRoutingService);
  });

  it('should start in writing state', () => {
    expect(service.state()).toBe('writing');
  });

  it('should transition to confirming on finish', () => {
    vi.useFakeTimers();
    service.finish('My journal entry about needing a roofer');
    expect(service.state()).toBe('confirming');
    vi.useRealTimers();
  });

  it('should populate intent summary after finish resolves', () => {
    vi.useFakeTimers();
    service.finish('I need someone to fix my roof');
    vi.advanceTimersByTime(1000);
    expect(service.intentSummary()).toBeTruthy();
    expect(service.intentSummary()).toContain('exchange');
    vi.useRealTimers();
  });

  it('should transition back to writing on edit', () => {
    vi.useFakeTimers();
    service.finish('Some text');
    vi.advanceTimersByTime(1000);
    service.edit();
    expect(service.state()).toBe('writing');
    vi.useRealTimers();
  });

  it('should transition to routing on confirm', () => {
    vi.useFakeTimers();
    service.finish('Some text');
    vi.advanceTimersByTime(1000);
    service.confirm();
    vi.advanceTimersByTime(1000);
    expect(service.state()).toBe('routing');
    vi.useRealTimers();
  });

  it('should always include a filing card in suggestions', () => {
    vi.useFakeTimers();
    service.finish('Some text');
    vi.advanceTimersByTime(1000);
    service.confirm();
    vi.advanceTimersByTime(1000);
    const filing = service.suggestions().find(s => s.kind === 'filing');
    expect(filing).toBeTruthy();
    expect(filing!.destinationType).toBe('journal-filing');
    expect(filing!.reach).toBe('private');
    vi.useRealTimers();
  });

  it('should detect exchange-request from need keywords', () => {
    vi.useFakeTimers();
    service.finish('I need someone to fix my leaking roof');
    vi.advanceTimersByTime(1000);
    service.confirm();
    vi.advanceTimersByTime(1000);
    const derivative = service.suggestions().find(s => s.destinationType === 'exchange-request');
    expect(derivative).toBeTruthy();
    expect(derivative!.kind).toBe('derivative');
    vi.useRealTimers();
  });

  it('should detect governance-proposal from policy keywords', () => {
    vi.useFakeTimers();
    service.finish('We should vote on a community maintenance fund');
    vi.advanceTimersByTime(1000);
    service.confirm();
    vi.advanceTimersByTime(1000);
    const derivative = service.suggestions().find(s => s.destinationType === 'governance-proposal');
    expect(derivative).toBeTruthy();
    vi.useRealTimers();
  });

  it('should post a filing card via PATCH', () => {
    vi.useFakeTimers();
    service.finish('Some text');
    vi.advanceTimersByTime(1000);
    service.confirm();
    vi.advanceTimersByTime(1000);
    const filing = service.suggestions().find(s => s.kind === 'filing')!;
    service.setContentId('journal-1');
    service.postCard(filing.id);
    expect(mockHttp.patch).toHaveBeenCalledWith(
      expect.stringContaining('/db/content/journal-1'),
      expect.objectContaining({ metadata: expect.objectContaining({ journalFolder: expect.any(String) }) }),
    );
    vi.useRealTimers();
  });

  it('should post a derivative card via POST', () => {
    vi.useFakeTimers();
    service.finish('I need a roofer');
    vi.advanceTimersByTime(1000);
    service.confirm();
    vi.advanceTimersByTime(1000);
    const card = service.suggestions().find(s => s.kind === 'derivative')!;
    service.postCard(card.id);
    expect(mockHttp.post).toHaveBeenCalledWith(
      expect.stringContaining('/db/content'),
      expect.objectContaining({ contentType: 'exchange-request' }),
    );
    vi.useRealTimers();
  });

  it('should mark card as posted after successful post', () => {
    vi.useFakeTimers();
    service.finish('I need a roofer');
    vi.advanceTimersByTime(1000);
    service.confirm();
    vi.advanceTimersByTime(1000);
    const card = service.suggestions().find(s => s.kind === 'derivative')!;
    service.postCard(card.id);
    const updated = service.suggestions().find(s => s.id === card.id);
    expect(updated!.status).toBe('posted');
    vi.useRealTimers();
  });

  it('should dismiss a card', () => {
    vi.useFakeTimers();
    service.finish('I need a roofer');
    vi.advanceTimersByTime(1000);
    service.confirm();
    vi.advanceTimersByTime(1000);
    const card = service.suggestions().find(s => s.kind === 'derivative')!;
    service.dismissCard(card.id);
    const updated = service.suggestions().find(s => s.id === card.id);
    expect(updated!.status).toBe('dismissed');
    vi.useRealTimers();
  });

  it('should transition to routed when all cards are resolved', () => {
    vi.useFakeTimers();
    service.finish('Hello world');
    vi.advanceTimersByTime(1000);
    service.confirm();
    vi.advanceTimersByTime(1000);
    service.setContentId('journal-1');
    // Should have at least the filing card
    for (const card of service.suggestions()) {
      service.postCard(card.id);
    }
    expect(service.state()).toBe('routed');
    vi.useRealTimers();
  });
});
```

**Step 2: Run tests to verify they fail**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts "journal-routing.service"`
Expected: FAIL — module not found

**Step 3: Implement the service**

Create `app/elohim-app/src/app/shefa/services/journal-routing.service.ts`:

```typescript
import { Injectable, inject, signal, computed } from '@angular/core';
import { timer } from 'rxjs';

import { StorageApiService } from '@app/elohim/services/storage-api.service';

import type {
  JournalRoutingState,
  IntentAnalysis,
  RoutingSuggestion,
  DestinationType,
} from '../models/journal-routing.model';

const NEED_KEYWORDS = ['need', 'want', 'broken', 'repair', 'help', 'fix', 'hire', 'find'];
const GOVERNANCE_KEYWORDS = ['should', 'vote', 'policy', 'propose', 'fund', 'community', 'rule'];
const CONTENT_KEYWORDS = ['learned', 'discovered', 'guide', 'how-to', 'tutorial', 'explain'];

const STUB_DELAY_MS = 800;

let nextId = 1;
function genId(): string {
  return `suggestion-${nextId++}`;
}

@Injectable()
export class JournalRoutingService {
  private readonly storageApi = inject(StorageApiService);

  private readonly _state = signal<JournalRoutingState>('writing');
  private readonly _intentSummary = signal('');
  private readonly _suggestions = signal<RoutingSuggestion[]>([]);
  private readonly _journalText = signal('');
  private _contentId = '';
  private _intent: IntentAnalysis | null = null;

  readonly state = this._state.asReadonly();
  readonly intentSummary = this._intentSummary.asReadonly();
  readonly suggestions = this._suggestions.asReadonly();
  readonly journalText = this._journalText.asReadonly();

  setContentId(id: string): void {
    this._contentId = id;
  }

  /** Round 1: Finish clicked — analyze intent */
  finish(text: string): void {
    if (this._state() === 'confirming') return;
    this._journalText.set(text);
    this._state.set('confirming');

    // Sidecar seam: replace with POST /api/v1/elohim/invoke
    this.analyzeIntent(text).subscribe((intent) => {
      this._intent = intent;
      this._intentSummary.set(intent.summary);
    });
  }

  /** Round 2: Human confirms intent — generate suggestion cards */
  confirm(): void {
    if (this._state() !== 'confirming' || !this._intent) return;

    // Sidecar seam: replace with POST /api/v1/elohim/invoke
    this.generateSuggestions(this._journalText(), this._intent).subscribe((suggestions) => {
      this._suggestions.set(suggestions);
      this._state.set('routing');
    });
  }

  /** Back to editing */
  edit(): void {
    this._state.set('writing');
    this._intentSummary.set('');
    this._suggestions.set([]);
    this._intent = null;
  }

  /** Post an individual card */
  postCard(id: string): void {
    const card = this._suggestions().find((s) => s.id === id);
    if (!card || card.status !== 'suggested') return;

    this.updateCardStatus(id, 'posting');

    if (card.kind === 'filing') {
      this.storageApi
        .updateContent(this._contentId, {
          metadata: { journalFolder: card.suggestedPath },
        })
        .subscribe({
          next: () => {
            this.updateCardStatus(id, 'posted');
            this.checkAllResolved();
          },
          error: () => this.updateCardStatus(id, 'suggested'),
        });
    } else {
      this.storageApi
        .createContent({
          id: `${card.destinationType}-${Date.now()}`,
          title: card.title,
          schemaVersion: 1,
          description: card.summary,
          contentType: card.destinationType,
          contentFormat: 'markdown',
          contentBody: this._journalText(),
          blobHash: null,
          blobCid: null,
          contentSizeBytes: null,
          metadata: { sourceJournalId: this._contentId },
          reach: card.reach,
          createdBy: null,
          tags: [],
        })
        .subscribe({
          next: () => {
            this.updateCardStatus(id, 'posted');
            this.checkAllResolved();
          },
          error: () => this.updateCardStatus(id, 'suggested'),
        });
    }
  }

  /** Dismiss a card */
  dismissCard(id: string): void {
    this.updateCardStatus(id, 'dismissed');
    this.checkAllResolved();
  }

  // ─── Sidecar Seams ────────────────────────────────────────────

  /** STUB: Keyword-based intent analysis. Replace with sidecar call. */
  private analyzeIntent(text: string) {
    const lower = text.toLowerCase();
    const detectedTypes: DestinationType[] = [];

    if (NEED_KEYWORDS.some((kw) => lower.includes(kw))) {
      detectedTypes.push('exchange-request');
    }
    if (GOVERNANCE_KEYWORDS.some((kw) => lower.includes(kw))) {
      detectedTypes.push('governance-proposal');
    }
    if (CONTENT_KEYWORDS.some((kw) => lower.includes(kw))) {
      detectedTypes.push('content');
    }

    const typeLabels: Record<DestinationType, string> = {
      'exchange-request': 'a need or request',
      'governance-proposal': 'a governance concern',
      content: 'shareable knowledge',
    };

    const parts = detectedTypes.map((t) => typeLabels[t]);
    const summary =
      parts.length > 0
        ? `This sounds like ${parts.join(' and ')}. The journal will be filed for you.`
        : 'This will be filed in your journal.';

    const suggestedPath = this.guessPath(text);

    return timer(STUB_DELAY_MS).pipe(
      map(() => ({ summary, detectedTypes, suggestedPath } satisfies IntentAnalysis)),
    );
  }

  /** STUB: Generate suggestion cards from intent. Replace with sidecar call. */
  private generateSuggestions(text: string, intent: IntentAnalysis) {
    const suggestions: RoutingSuggestion[] = [];

    // Always: filing card
    suggestions.push({
      id: genId(),
      kind: 'filing',
      destinationType: 'journal-filing',
      title: 'File journal entry',
      summary: `Save to ${intent.suggestedPath}`,
      suggestedPath: intent.suggestedPath,
      reach: 'private',
      contextMetadata: {},
      status: 'suggested',
    });

    // Derivative cards based on detected types
    const templates: Record<DestinationType, { title: string; summary: string; reach: ReachTier; path: string }> = {
      'exchange-request': {
        title: 'Post to Exchange',
        summary: 'Create a request on the community exchange',
        reach: 'community',
        path: '/shefa/exchange/',
      },
      'governance-proposal': {
        title: 'Draft governance proposal',
        summary: 'Start a governance conversation',
        reach: 'community',
        path: '/qahal/proposals/',
      },
      content: {
        title: 'Share as learning content',
        summary: 'Contribute to the knowledge commons',
        reach: 'network',
        path: '/lamad/contributions/',
      },
    };

    for (const dtype of intent.detectedTypes) {
      const tmpl = templates[dtype];
      suggestions.push({
        id: genId(),
        kind: 'derivative',
        destinationType: dtype,
        title: tmpl.title,
        summary: tmpl.summary,
        suggestedPath: tmpl.path,
        reach: tmpl.reach,
        contextMetadata: {},
        status: 'suggested',
      });
    }

    return timer(STUB_DELAY_MS).pipe(
      map(() => suggestions),
    );
  }

  private guessPath(text: string): string {
    const lower = text.toLowerCase();
    if (lower.includes('roof') || lower.includes('house') || lower.includes('repair')) {
      return '/journal/home-maintenance/';
    }
    if (lower.includes('learn') || lower.includes('study') || lower.includes('read')) {
      return '/journal/learning/';
    }
    if (lower.includes('work') || lower.includes('project') || lower.includes('team')) {
      return '/journal/work/';
    }
    return '/journal/general/';
  }

  private updateCardStatus(id: string, status: RoutingSuggestion['status']): void {
    this._suggestions.update((cards) =>
      cards.map((c) => (c.id === id ? { ...c, status } : c)),
    );
  }

  private checkAllResolved(): void {
    const all = this._suggestions();
    if (all.length > 0 && all.every((c) => c.status === 'posted' || c.status === 'dismissed')) {
      this._state.set('routed');
    }
  }
}
```

Note: The service needs `import { map } from 'rxjs/operators'` and `import type { ReachTier } from '@app/elohim/services/gate-interaction.service'` at the top.

**Step 4: Run tests to verify they pass**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts "journal-routing.service"`
Expected: All pass

**Step 5: Commit**

```bash
git add app/elohim-app/src/app/shefa/services/journal-routing.service.ts \
        app/elohim-app/src/app/shefa/services/journal-routing.service.spec.ts
git commit -m "feat(shefa): add JournalRoutingService with stubbed sidecar seams"
```

---

### Task 3: JournalConfirmComponent

**Files:**
- Create: `app/elohim-app/src/app/shefa/components/journal-page/journal-confirm.component.ts`
- Test: `app/elohim-app/src/app/shefa/components/journal-page/journal-confirm.component.spec.ts`

**Step 1: Write the failing tests**

```typescript
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, vi } from 'vitest';

import { JournalConfirmComponent } from './journal-confirm.component';

describe('JournalConfirmComponent', () => {
  let component: JournalConfirmComponent;
  let fixture: ComponentFixture<JournalConfirmComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [JournalConfirmComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(JournalConfirmComponent);
    component = fixture.componentInstance;
    fixture.componentRef.setInput('text', 'My journal about roof repair');
    fixture.componentRef.setInput('intentSummary', 'This sounds like a need.');
    fixture.componentRef.setInput('analyzing', false);
    fixture.detectChanges();
  });

  it('should show the journal text read-only', () => {
    const el = fixture.nativeElement.querySelector('[data-testid="confirm-text"]');
    expect(el).toBeTruthy();
    expect(el.textContent).toContain('My journal about roof repair');
  });

  it('should show intent summary', () => {
    const el = fixture.nativeElement.querySelector('[data-testid="intent-summary"]');
    expect(el).toBeTruthy();
    expect(el.textContent).toContain('This sounds like a need.');
  });

  it('should show shimmer when analyzing', () => {
    fixture.componentRef.setInput('analyzing', true);
    fixture.detectChanges();
    const el = fixture.nativeElement.querySelector('[data-testid="confirm-shimmer"]');
    expect(el).toBeTruthy();
  });

  it('should emit confirm on Looks good click', () => {
    const spy = vi.spyOn(component.confirmed, 'emit');
    const btn = fixture.nativeElement.querySelector('[data-testid="confirm-btn"]');
    btn.click();
    expect(spy).toHaveBeenCalled();
  });

  it('should emit editRequested on Edit click', () => {
    const spy = vi.spyOn(component.editRequested, 'emit');
    const btn = fixture.nativeElement.querySelector('[data-testid="edit-btn"]');
    btn.click();
    expect(spy).toHaveBeenCalled();
  });

  it('should disable Looks good button when analyzing', () => {
    fixture.componentRef.setInput('analyzing', true);
    fixture.detectChanges();
    const btn = fixture.nativeElement.querySelector('[data-testid="confirm-btn"]') as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
  });
});
```

**Step 2: Run tests to verify they fail**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts "journal-confirm"`
Expected: FAIL

**Step 3: Implement the component**

Presentational component with inputs/outputs. Uses inline template. Reuses shimmer animation from GateArtifactCard. The `confirm-text` shows the journal text in a read-only preview. Intent summary appears below when analyzing completes.

**Step 4: Run tests to verify they pass**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts "journal-confirm"`
Expected: All pass

**Step 5: Commit**

```bash
git add app/elohim-app/src/app/shefa/components/journal-page/journal-confirm.component.ts \
        app/elohim-app/src/app/shefa/components/journal-page/journal-confirm.component.spec.ts
git commit -m "feat(shefa): add JournalConfirmComponent for intent verification"
```

---

### Task 4: JournalRoutingCardsComponent

**Files:**
- Create: `app/elohim-app/src/app/shefa/components/journal-page/journal-routing-cards.component.ts`
- Test: `app/elohim-app/src/app/shefa/components/journal-page/journal-routing-cards.component.spec.ts`

**Step 1: Write the failing tests**

```typescript
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, vi } from 'vitest';

import { JournalRoutingCardsComponent } from './journal-routing-cards.component';
import type { RoutingSuggestion } from '../../models/journal-routing.model';

const MOCK_SUGGESTIONS: RoutingSuggestion[] = [
  {
    id: 'filing-1',
    kind: 'filing',
    destinationType: 'journal-filing',
    title: 'File journal entry',
    summary: 'Save to /journal/home-maintenance/',
    suggestedPath: '/journal/home-maintenance/',
    reach: 'private',
    contextMetadata: {},
    status: 'suggested',
  },
  {
    id: 'deriv-1',
    kind: 'derivative',
    destinationType: 'exchange-request',
    title: 'Post to Exchange',
    summary: 'Create a request on the community exchange',
    suggestedPath: '/shefa/exchange/',
    reach: 'community',
    contextMetadata: {},
    status: 'suggested',
  },
];

describe('JournalRoutingCardsComponent', () => {
  let component: JournalRoutingCardsComponent;
  let fixture: ComponentFixture<JournalRoutingCardsComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [JournalRoutingCardsComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(JournalRoutingCardsComponent);
    component = fixture.componentInstance;
    fixture.componentRef.setInput('suggestions', MOCK_SUGGESTIONS);
    fixture.componentRef.setInput('journalText', 'My journal about roof repair');
    fixture.detectChanges();
  });

  it('should render a card for each suggestion', () => {
    const cards = fixture.nativeElement.querySelectorAll('[data-testid="routing-card"]');
    expect(cards.length).toBe(2);
  });

  it('should render filing card first', () => {
    const cards = fixture.nativeElement.querySelectorAll('[data-testid="routing-card"]');
    expect(cards[0].querySelector('[data-testid="card-title"]').textContent).toContain('File journal entry');
  });

  it('should show reach badge on each card', () => {
    const badges = fixture.nativeElement.querySelectorAll('[data-testid="card-reach"]');
    expect(badges.length).toBe(2);
  });

  it('should emit postCard with card id on post click', () => {
    const spy = vi.spyOn(component.postCard, 'emit');
    const btns = fixture.nativeElement.querySelectorAll('[data-testid="card-post-btn"]');
    btns[1].click(); // derivative card
    expect(spy).toHaveBeenCalledWith('deriv-1');
  });

  it('should emit dismissCard with card id on dismiss click', () => {
    const spy = vi.spyOn(component.dismissCard, 'emit');
    const btns = fixture.nativeElement.querySelectorAll('[data-testid="card-dismiss-btn"]');
    btns[0].click();
    expect(spy).toHaveBeenCalledWith('filing-1');
  });

  it('should show posted state for posted cards', () => {
    const posted = MOCK_SUGGESTIONS.map((s) =>
      s.id === 'filing-1' ? { ...s, status: 'posted' as const } : s,
    );
    fixture.componentRef.setInput('suggestions', posted);
    fixture.detectChanges();
    const badge = fixture.nativeElement.querySelector('[data-testid="card-posted-badge"]');
    expect(badge).toBeTruthy();
  });

  it('should emit editRequested on edit button click', () => {
    const spy = vi.spyOn(component.editRequested, 'emit');
    const btn = fixture.nativeElement.querySelector('[data-testid="routing-edit-btn"]');
    btn.click();
    expect(spy).toHaveBeenCalled();
  });
});
```

**Step 2: Run tests to verify they fail**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts "journal-routing-cards"`
Expected: FAIL

**Step 3: Implement the component**

Presentational component. Receives `suggestions` input, renders cards with `@for`. Filing card gets a subtle visual treatment (lighter background, smaller). Derivative cards are more prominent with action buttons. Each card shows title, summary, reach badge, and post/dismiss buttons. Posted cards show a posted badge instead of buttons. Include an "Edit" button at the top to return to writing. Collapsed journal text preview at the top (first 3 lines + title).

Reuse reach badge styling from GateArtifactCard (same `REACH_ICONS` and `REACH_LABELS` maps — import `ReachTier` from gate-interaction.service).

**Step 4: Run tests to verify they pass**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts "journal-routing-cards"`
Expected: All pass

**Step 5: Commit**

```bash
git add app/elohim-app/src/app/shefa/components/journal-page/journal-routing-cards.component.ts \
        app/elohim-app/src/app/shefa/components/journal-page/journal-routing-cards.component.spec.ts
git commit -m "feat(shefa): add JournalRoutingCardsComponent for EPR suggestion cards"
```

---

### Task 5: JournalRoutedComponent

**Files:**
- Create: `app/elohim-app/src/app/shefa/components/journal-page/journal-routed.component.ts`
- Test: `app/elohim-app/src/app/shefa/components/journal-page/journal-routed.component.spec.ts`

**Step 1: Write the failing tests**

```typescript
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, vi } from 'vitest';

import { JournalRoutedComponent } from './journal-routed.component';
import type { RoutingSuggestion } from '../../models/journal-routing.model';

const POSTED: RoutingSuggestion[] = [
  {
    id: 'filing-1',
    kind: 'filing',
    destinationType: 'journal-filing',
    title: 'File journal entry',
    summary: 'Save to /journal/home-maintenance/',
    suggestedPath: '/journal/home-maintenance/',
    reach: 'private',
    contextMetadata: {},
    status: 'posted',
  },
  {
    id: 'deriv-1',
    kind: 'derivative',
    destinationType: 'exchange-request',
    title: 'Post to Exchange',
    summary: 'Create a request on the community exchange',
    suggestedPath: '/shefa/exchange/',
    reach: 'community',
    contextMetadata: {},
    status: 'posted',
  },
];

describe('JournalRoutedComponent', () => {
  let component: JournalRoutedComponent;
  let fixture: ComponentFixture<JournalRoutedComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [JournalRoutedComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(JournalRoutedComponent);
    component = fixture.componentInstance;
    fixture.componentRef.setInput('suggestions', POSTED);
    fixture.detectChanges();
  });

  it('should show posted cards with reach badges', () => {
    const badges = fixture.nativeElement.querySelectorAll('[data-testid="routed-reach"]');
    expect(badges.length).toBe(2);
  });

  it('should show completion message', () => {
    const el = fixture.nativeElement.querySelector('[data-testid="routed-message"]');
    expect(el).toBeTruthy();
  });

  it('should emit writeAnother on button click', () => {
    const spy = vi.spyOn(component.writeAnother, 'emit');
    const btn = fixture.nativeElement.querySelector('[data-testid="write-another-btn"]');
    btn.click();
    expect(spy).toHaveBeenCalled();
  });

  it('should not show dismissed cards', () => {
    const withDismissed = POSTED.map((s) =>
      s.id === 'deriv-1' ? { ...s, status: 'dismissed' as const } : s,
    );
    fixture.componentRef.setInput('suggestions', withDismissed);
    fixture.detectChanges();
    const badges = fixture.nativeElement.querySelectorAll('[data-testid="routed-reach"]');
    expect(badges.length).toBe(1); // Only the posted filing card
  });
});
```

**Step 2: Run tests to verify they fail**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts "journal-routed"`
Expected: FAIL

**Step 3: Implement the component**

Presentational component showing completion. Filters to posted cards only. Shows each with title, destination, reach badge. Warm completion message ("Your words are where they belong."). "Write another" button emits event for parent to navigate.

**Step 4: Run tests to verify they pass**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts "journal-routed"`
Expected: All pass

**Step 5: Commit**

```bash
git add app/elohim-app/src/app/shefa/components/journal-page/journal-routed.component.ts \
        app/elohim-app/src/app/shefa/components/journal-page/journal-routed.component.spec.ts
git commit -m "feat(shefa): add JournalRoutedComponent for finish completion state"
```

---

### Task 6: JournalEditorComponent — Add Finish Button + Readonly Mode

**Files:**
- Modify: `app/elohim-app/src/app/shefa/components/journal-page/journal-editor.component.ts`
- Modify: `app/elohim-app/src/app/shefa/components/journal-page/journal-editor.component.spec.ts`

**Step 1: Add failing tests for new behavior**

Add to existing spec:

```typescript
it('should show Finish button when body is non-empty', () => {
  component.loadContent('Title', 'Some body');
  fixture.detectChanges();
  const btn = fixture.nativeElement.querySelector('[data-testid="finish-btn"]');
  expect(btn).toBeTruthy();
});

it('should not show Finish button when body is empty', () => {
  component.loadContent('Title', '');
  fixture.detectChanges();
  const btn = fixture.nativeElement.querySelector('[data-testid="finish-btn"]');
  expect(btn).toBeFalsy();
});

it('should emit finish with title and body when clicked', () => {
  const spy = vi.fn();
  component.finished = { emit: spy } as any;
  component.loadContent('My Title', 'My body text');
  fixture.detectChanges();
  const btn = fixture.nativeElement.querySelector('[data-testid="finish-btn"]') as HTMLButtonElement;
  btn.click();
  expect(spy).toHaveBeenCalledWith({ title: 'My Title', body: 'My body text' });
});

it('should disable inputs when readonly', () => {
  fixture.componentRef.setInput('readonly', true);
  fixture.detectChanges();
  const title = fixture.nativeElement.querySelector('[data-testid="journal-title"]') as HTMLInputElement;
  const body = fixture.nativeElement.querySelector('[data-testid="journal-body"]') as HTMLTextAreaElement;
  expect(title.readOnly).toBe(true);
  expect(body.readOnly).toBe(true);
});

it('should hide Finish button when readonly', () => {
  fixture.componentRef.setInput('readonly', true);
  component.loadContent('Title', 'Body');
  fixture.detectChanges();
  const btn = fixture.nativeElement.querySelector('[data-testid="finish-btn"]');
  expect(btn).toBeFalsy();
});
```

**Step 2: Run tests to verify they fail**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts "journal-editor"`
Expected: New tests FAIL

**Step 3: Modify the component**

Add to `JournalEditorComponent`:
- `readonly readonly = input(false)` — input signal
- `@Output() finished = new EventEmitter<{ title: string; body: string }>()` — finish event
- Add `[readOnly]="readonly()"` to both inputs
- Add `@if (body() && !readonly())` guarded "Finish" button at bottom
- Button style: distinct from save — calm, intentional. Green like the affirm button in GateArtifactCard.

**Step 4: Run tests to verify they pass**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts "journal-editor"`
Expected: All pass (old + new)

**Step 5: Commit**

```bash
git add app/elohim-app/src/app/shefa/components/journal-page/journal-editor.component.ts \
        app/elohim-app/src/app/shefa/components/journal-page/journal-editor.component.spec.ts
git commit -m "feat(shefa): add Finish button and readonly mode to JournalEditorComponent"
```

---

### Task 7: Wire JournalPageComponent as State Orchestrator

**Files:**
- Modify: `app/elohim-app/src/app/shefa/components/journal-page/journal-page.component.ts`
- Modify: `app/elohim-app/src/app/shefa/components/journal-page/journal-page.component.spec.ts`

**Step 1: Add failing tests**

Replace existing spec with comprehensive orchestration tests:

```typescript
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { ActivatedRoute, Router } from '@angular/router';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { of } from 'rxjs';

import { JournalPageComponent } from './journal-page.component';

describe('JournalPageComponent', () => {
  let component: JournalPageComponent;
  let fixture: ComponentFixture<JournalPageComponent>;
  let mockRouter: { navigate: ReturnType<typeof vi.fn> };

  beforeEach(async () => {
    const mockHttp = {
      get: vi.fn().mockReturnValue(of({
        id: 'journal-1', title: 'Test', contentBody: 'Body text', contentType: 'journal',
      })),
      patch: vi.fn().mockReturnValue(of({})),
      post: vi.fn().mockReturnValue(of({})),
    };

    mockRouter = { navigate: vi.fn() };

    const mockRoute = {
      paramMap: of(new Map([['id', 'journal-1']])),
    };

    await TestBed.configureTestingModule({
      imports: [JournalPageComponent],
      providers: [
        { provide: HttpClient, useValue: mockHttp },
        { provide: ActivatedRoute, useValue: mockRoute },
        { provide: Router, useValue: mockRouter },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(JournalPageComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should render editor in writing state', () => {
    const editor = fixture.nativeElement.querySelector('app-journal-editor');
    expect(editor).toBeTruthy();
  });

  it('should render sidebar', () => {
    const sidebar = fixture.nativeElement.querySelector('app-elohim-sidebar');
    expect(sidebar).toBeTruthy();
  });

  it('should render two-panel layout', () => {
    const layout = fixture.nativeElement.querySelector('[data-testid="journal-layout"]');
    expect(layout).toBeTruthy();
  });
});
```

**Step 2: Run tests to verify they fail**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts "journal-page.component"`
Expected: New tests FAIL (old ones may break too from restructuring)

**Step 3: Rewrite JournalPageComponent as orchestrator**

The component provides `JournalRoutingService` in its `providers` array (component-scoped). Template uses `@switch (routing.state())` to render:

- `'writing'` — `<app-journal-editor>` (editable) with `(finished)` handler
- `'confirming'` — `<app-journal-confirm>` with intent summary, shimmer, buttons
- `'routing'` — `<app-journal-routing-cards>` with collapsed text preview + cards
- `'routed'` — `<app-journal-routed>` with completion + "Write another"

The sidebar (`<app-elohim-sidebar>`) renders in ALL states outside the switch.

Event handlers delegate to `JournalRoutingService`:
- `onFinish({ title, body })` → `routing.finish(body)`
- `onConfirm()` → `routing.confirm()`
- `onEdit()` → `routing.edit()`
- `onPostCard(id)` → `routing.postCard(id)`
- `onDismissCard(id)` → `routing.dismissCard(id)`
- `onWriteAnother()` → navigate to new journal

**Step 4: Run tests to verify they pass**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts "journal-page"`
Expected: All pass

**Step 5: Commit**

```bash
git add app/elohim-app/src/app/shefa/components/journal-page/journal-page.component.ts \
        app/elohim-app/src/app/shefa/components/journal-page/journal-page.component.spec.ts
git commit -m "feat(shefa): wire JournalPageComponent as state-driven orchestrator"
```

---

### Task 8: Integration Verification

**Step 1: Run all journal-related tests together**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts "journal"`
Expected: All pass

**Step 2: Run full shefa test suite**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts "shefa"`
Expected: All pass (no regressions)

**Step 3: Lint check**

Run: `cd /projects/elohim/app/elohim-app && pnpm run lint`
Expected: No new errors in journal files

**Step 4: Commit any fixes, then final commit if clean**

```bash
git add -A
git commit -m "test(shefa): verify journal routing integration"
```
