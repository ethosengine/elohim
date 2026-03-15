# Gate Feedback Modal — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the always-available governance context menu (⋮ → Flag/Challenge/Feedback) that opens a modal containing a GateArtifactCard, using the existing gate interaction pipeline.

**Architecture:** Two new Angular components — `GateFeedbackTriggerComponent` (⋮ button + dropdown) and `GateFeedbackModalComponent` (overlay + GateArtifactCard). No new services. The feedback type flows through `mutationType` and `contextMetadata.category`. Both live in the elohim pillar's components directory.

**Tech Stack:** Angular 19 (signals, OnPush, standalone), Vitest, existing GateArtifactCardComponent + GateInteractionService

**Design doc:** `genesis/plans/2026-03-15-gate-feedback-modal-design.md`

---

### Task 1: GateFeedbackModalComponent — failing tests

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-modal.component.spec.ts`

**Step 1: Write the failing tests**

Create the spec file with tests for the modal component. Follow the same pattern as `gate-artifact-card.component.spec.ts` — import the component, set up TestBed with HttpClient mock, test DOM behavior.

```typescript
import { ComponentFixture, TestBed, fakeAsync, tick } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { of } from 'rxjs';
import { vi } from 'vitest';

import type { GateEvaluationView, TrustContextView } from '@elohim/storage-client';

import { GateFeedbackModalComponent } from './gate-feedback-modal.component';

function makeTrustContext(
  overrides: Partial<TrustContextView> = {},
): TrustContextView {
  return {
    compositeTrust: 0.5,
    masteryDepth: 0.5,
    stewardStanding: 0.7,
    relationshipDensity: 0.3,
    governanceHealth: 0.9,
    behavioralTrust: 0.85,
    intentDivergence: 0.1,
    declaredIntent: null,
    ...overrides,
  };
}

function makeEvaluation(
  overrides: Partial<GateEvaluationView> = {},
): GateEvaluationView {
  return {
    tier: 'standard',
    trustContext: makeTrustContext(),
    pausePrompt: null,
    confirmToken: null,
    settlementBoundary: null,
    appealPath: null,
    ...overrides,
  };
}

describe('GateFeedbackModalComponent', () => {
  let component: GateFeedbackModalComponent;
  let fixture: ComponentFixture<GateFeedbackModalComponent>;
  let httpMock: { post: ReturnType<typeof vi.fn> };

  beforeEach(async () => {
    httpMock = { post: vi.fn().mockReturnValue(of({})) };

    await TestBed.configureTestingModule({
      imports: [GateFeedbackModalComponent],
      providers: [{ provide: HttpClient, useValue: httpMock }],
    }).compileComponents();

    fixture = TestBed.createComponent(GateFeedbackModalComponent);
    component = fixture.componentInstance;
    component.feedbackType = 'feedback';
    component.contentId = 'content-1';
    fixture.detectChanges();
  });

  // --- Rendering ---

  it('should render modal backdrop', () => {
    const backdrop = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-backdrop"]',
    );
    expect(backdrop).toBeTruthy();
  });

  it('should render modal panel', () => {
    const panel = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-panel"]',
    );
    expect(panel).toBeTruthy();
  });

  it('should render title based on feedback type', () => {
    const title = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-title"]',
    );
    expect(title.textContent.trim()).toBe('Share Feedback');
  });

  it('should render "Flag Content" title for flag type', () => {
    component.feedbackType = 'flag';
    fixture.detectChanges();

    const title = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-title"]',
    );
    expect(title.textContent.trim()).toBe('Flag Content');
  });

  it('should render "Challenge Content" title for challenge type', () => {
    component.feedbackType = 'challenge';
    fixture.detectChanges();

    const title = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-title"]',
    );
    expect(title.textContent.trim()).toBe('Challenge Content');
  });

  it('should render close button', () => {
    const btn = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-close"]',
    );
    expect(btn).toBeTruthy();
  });

  it('should contain a gate-artifact-card', () => {
    const card = fixture.nativeElement.querySelector('app-gate-artifact-card');
    expect(card).toBeTruthy();
  });

  // --- Placeholder text ---

  it('should set placeholder to "Describe the issue..." for flag type', () => {
    component.feedbackType = 'flag';
    fixture.detectChanges();

    const textarea = fixture.nativeElement.querySelector(
      '[data-testid="artifact-textarea"]',
    );
    expect(textarea.placeholder).toBe('Describe the issue...');
  });

  it('should set placeholder to "State your case..." for challenge type', () => {
    component.feedbackType = 'challenge';
    fixture.detectChanges();

    const textarea = fixture.nativeElement.querySelector(
      '[data-testid="artifact-textarea"]',
    );
    expect(textarea.placeholder).toBe('State your case...');
  });

  it('should set placeholder to "Share your thoughts..." for feedback type', () => {
    const textarea = fixture.nativeElement.querySelector(
      '[data-testid="artifact-textarea"]',
    );
    expect(textarea.placeholder).toBe('Share your thoughts...');
  });

  // --- Close behavior ---

  it('should emit closed when close button clicked', () => {
    const closedSpy = vi.fn();
    component.closed.subscribe(closedSpy);

    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-close"]',
    );
    btn.click();
    fixture.detectChanges();

    expect(closedSpy).toHaveBeenCalled();
  });

  it('should emit closed when backdrop clicked', () => {
    const closedSpy = vi.fn();
    component.closed.subscribe(closedSpy);

    const backdrop: HTMLElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-backdrop"]',
    );
    backdrop.click();
    fixture.detectChanges();

    expect(closedSpy).toHaveBeenCalled();
  });

  it('should NOT emit closed when panel clicked', () => {
    const closedSpy = vi.fn();
    component.closed.subscribe(closedSpy);

    const panel: HTMLElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-panel"]',
    );
    panel.click();
    fixture.detectChanges();

    expect(closedSpy).not.toHaveBeenCalled();
  });

  // --- Event forwarding ---

  it('should emit posted when artifact card posts', () => {
    const postedSpy = vi.fn();
    component.posted.subscribe(postedSpy);

    // Drive the inner card through the gate flow
    const textarea: HTMLTextAreaElement = fixture.nativeElement.querySelector(
      '[data-testid="artifact-textarea"]',
    );
    textarea.value = 'my feedback';
    textarea.dispatchEvent(new Event('input'));
    fixture.detectChanges();

    const submitBtn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="artifact-submit"]',
    );
    submitBtn.click();
    fixture.detectChanges();

    // Simulate gate evaluation (affirm path)
    const cardComponent = component.artifactCard();
    cardComponent.interaction.handleGateEvaluation(makeEvaluation());
    fixture.detectChanges();

    // Click affirm
    const affirmBtn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="artifact-affirm"]',
    );
    affirmBtn.click();
    fixture.detectChanges();

    expect(postedSpy).toHaveBeenCalledWith({ reachTier: 'community' });
  });

  it('should emit settled when artifact card settles', () => {
    const settledSpy = vi.fn();
    component.settled.subscribe(settledSpy);

    const textarea: HTMLTextAreaElement = fixture.nativeElement.querySelector(
      '[data-testid="artifact-textarea"]',
    );
    textarea.value = 'my challenge';
    textarea.dispatchEvent(new Event('input'));
    fixture.detectChanges();

    const submitBtn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="artifact-submit"]',
    );
    submitBtn.click();
    fixture.detectChanges();

    const cardComponent = component.artifactCard();
    cardComponent.interaction.handleGateEvaluation(
      makeEvaluation({
        settlementBoundary: 'harm-prevention',
        appealPath: '/appeal/42',
      }),
    );
    fixture.detectChanges();

    expect(settledSpy).toHaveBeenCalledWith({
      boundary: 'harm-prevention',
      appealPath: '/appeal/42',
    });
  });
});
```

**Step 2: Run tests to verify they fail**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate-feedback-modal"`
Expected: FAIL — module not found

**Step 3: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-modal.component.spec.ts
git commit -m "test(elohim): add failing tests for GateFeedbackModalComponent"
```

---

### Task 2: GateFeedbackModalComponent — implementation

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-modal.component.ts`

**Step 1: Write the component**

```typescript
import {
  Component,
  EventEmitter,
  HostListener,
  Input,
  Output,
  ChangeDetectionStrategy,
  computed,
  viewChild,
} from '@angular/core';

import {
  GateArtifactCardComponent,
} from '../gate-artifact-card/gate-artifact-card.component';
import type { ReachTier } from '../../services/gate-interaction.service';

export type FeedbackType = 'flag' | 'challenge' | 'feedback';

const TITLES: Record<FeedbackType, string> = {
  flag: 'Flag Content',
  challenge: 'Challenge Content',
  feedback: 'Share Feedback',
};

const PLACEHOLDERS: Record<FeedbackType, string> = {
  flag: 'Describe the issue...',
  challenge: 'State your case...',
  feedback: 'Share your thoughts...',
};

@Component({
  selector: 'app-gate-feedback-modal',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [GateArtifactCardComponent],
  template: `
    <div
      class="feedback-modal-backdrop"
      data-testid="feedback-modal-backdrop"
      (click)="onBackdropClick()"
    >
      <div
        class="feedback-modal-panel"
        data-testid="feedback-modal-panel"
        (click)="$event.stopPropagation()"
      >
        <div class="feedback-modal-header">
          <h3 data-testid="feedback-modal-title">{{ title() }}</h3>
          <button
            class="btn-close"
            data-testid="feedback-modal-close"
            aria-label="Close"
            (click)="onClose()"
          >
            &times;
          </button>
        </div>
        <app-gate-artifact-card
          [placeholder]="placeholder()"
          [mutationType]="feedbackType"
          [contextMetadata]="{ contentId: contentId, category: feedbackType }"
          (posted)="onPosted($event)"
          (settled)="onSettled($event)"
        ></app-gate-artifact-card>
      </div>
    </div>
  `,
  styles: [
    `
      .feedback-modal-backdrop {
        position: fixed;
        inset: 0;
        z-index: 1000;
        display: flex;
        align-items: center;
        justify-content: center;
        background: rgba(0, 0, 0, 0.4);
      }

      .feedback-modal-panel {
        width: 90%;
        max-width: 600px;
        max-height: 90vh;
        overflow-y: auto;
        background: var(--surface-elevated, #fff);
        border-radius: var(--radius-lg, 12px);
        padding: 1.5rem;
      }

      .feedback-modal-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: 1rem;
      }

      .feedback-modal-header h3 {
        margin: 0;
        font-size: 1.125rem;
        font-weight: 600;
        color: var(--text-primary, #202124);
      }

      .btn-close {
        background: none;
        border: none;
        font-size: 1.5rem;
        line-height: 1;
        color: var(--text-secondary, #5f6368);
        cursor: pointer;
        padding: 0.25rem;
      }

      .btn-close:hover {
        color: var(--text-primary, #202124);
      }
    `,
  ],
})
export class GateFeedbackModalComponent {
  @Input() feedbackType: FeedbackType = 'feedback';
  @Input() contentId = '';

  @Output() posted = new EventEmitter<{ reachTier: ReachTier }>();
  @Output() settled = new EventEmitter<{
    boundary: string;
    appealPath: string | null;
  }>();
  @Output() closed = new EventEmitter<void>();

  readonly artifactCard = viewChild.required(GateArtifactCardComponent);

  readonly title = computed(() => TITLES[this.feedbackType]);
  readonly placeholder = computed(() => PLACEHOLDERS[this.feedbackType]);

  @HostListener('document:keydown.escape')
  onEscapeKey(): void {
    this.onClose();
  }

  onBackdropClick(): void {
    this.closed.emit();
  }

  onClose(): void {
    this.closed.emit();
  }

  onPosted(event: { reachTier: ReachTier }): void {
    this.posted.emit(event);
  }

  onSettled(event: { boundary: string; appealPath: string | null }): void {
    this.settled.emit(event);
  }
}
```

**Important note on `title()` and `placeholder()`:** These use `computed()` but `feedbackType` is an `@Input`, not a signal. The computed will capture the initial value only. Two options:

- Use `input.required<FeedbackType>()` signal input (Angular 17.1+) — preferred
- Use plain getter methods instead of computed

Use **signal inputs** since we're on Angular 19:

```typescript
readonly feedbackType = input<FeedbackType>('feedback');
readonly contentId = input('');

readonly title = computed(() => TITLES[this.feedbackType()]);
readonly placeholder = computed(() => PLACEHOLDERS[this.feedbackType()]);
```

And update the template to use `feedbackType()`, `contentId()` in the binding:

```html
[mutationType]="feedbackType()"
[contextMetadata]="{ contentId: contentId(), category: feedbackType() }"
```

Update the test accordingly — set inputs via `fixture.componentRef.setInput('feedbackType', 'flag')` instead of `component.feedbackType = 'flag'`.

**Step 2: Run tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate-feedback-modal"`
Expected: All tests PASS

**Step 3: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-modal.component.ts
git commit -m "feat(elohim): implement GateFeedbackModalComponent — modal overlay with GateArtifactCard"
```

---

### Task 3: GateFeedbackTriggerComponent — failing tests

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-trigger.component.spec.ts`

**Step 1: Write the failing tests**

```typescript
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { of } from 'rxjs';
import { vi } from 'vitest';

import { GateFeedbackTriggerComponent } from './gate-feedback-trigger.component';

describe('GateFeedbackTriggerComponent', () => {
  let component: GateFeedbackTriggerComponent;
  let fixture: ComponentFixture<GateFeedbackTriggerComponent>;

  beforeEach(async () => {
    const httpMock = { post: vi.fn().mockReturnValue(of({})) };

    await TestBed.configureTestingModule({
      imports: [GateFeedbackTriggerComponent],
      providers: [{ provide: HttpClient, useValue: httpMock }],
    }).compileComponents();

    fixture = TestBed.createComponent(GateFeedbackTriggerComponent);
    component = fixture.componentInstance;
    fixture.componentRef.setInput('contentId', 'content-1');
    fixture.detectChanges();
  });

  // --- Trigger button ---

  it('should render the trigger button', () => {
    const btn = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-btn"]',
    );
    expect(btn).toBeTruthy();
  });

  it('should not show menu initially', () => {
    const menu = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-menu"]',
    );
    expect(menu).toBeFalsy();
  });

  // --- Menu ---

  it('should show menu when trigger clicked', () => {
    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-btn"]',
    );
    btn.click();
    fixture.detectChanges();

    const menu = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-menu"]',
    );
    expect(menu).toBeTruthy();
  });

  it('should show three menu items', () => {
    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-btn"]',
    );
    btn.click();
    fixture.detectChanges();

    const items = fixture.nativeElement.querySelectorAll(
      '[data-testid^="feedback-menu-item-"]',
    );
    expect(items.length).toBe(3);
  });

  it('should show Flag, Challenge, Feedback labels', () => {
    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-btn"]',
    );
    btn.click();
    fixture.detectChanges();

    const labels = Array.from(
      fixture.nativeElement.querySelectorAll('[data-testid^="feedback-menu-item-"]'),
    ).map((el: Element) => (el as HTMLElement).textContent?.trim());

    expect(labels).toEqual(['Flag', 'Challenge', 'Feedback']);
  });

  // --- Modal opening ---

  it('should open modal when Flag clicked', () => {
    // Open menu
    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-btn"]',
    );
    btn.click();
    fixture.detectChanges();

    // Click Flag
    const flagItem: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-menu-item-flag"]',
    );
    flagItem.click();
    fixture.detectChanges();

    // Modal should be visible
    const modal = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-backdrop"]',
    );
    expect(modal).toBeTruthy();
  });

  it('should close menu when item clicked', () => {
    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-btn"]',
    );
    btn.click();
    fixture.detectChanges();

    const item: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-menu-item-feedback"]',
    );
    item.click();
    fixture.detectChanges();

    const menu = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-menu"]',
    );
    expect(menu).toBeFalsy();
  });

  // --- Modal closing ---

  it('should close modal when modal emits closed', () => {
    // Open menu → click Feedback → modal opens
    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-btn"]',
    );
    btn.click();
    fixture.detectChanges();

    const item: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-menu-item-feedback"]',
    );
    item.click();
    fixture.detectChanges();

    // Click close button on modal
    const closeBtn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-close"]',
    );
    closeBtn.click();
    fixture.detectChanges();

    // Modal should be gone
    const modal = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-backdrop"]',
    );
    expect(modal).toBeFalsy();
  });

  // --- Event forwarding ---

  it('should forward feedbackPosted from modal', () => {
    const postedSpy = vi.fn();
    component.feedbackPosted.subscribe(postedSpy);

    // Open modal
    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-trigger-btn"]',
    );
    btn.click();
    fixture.detectChanges();
    const item: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-menu-item-challenge"]',
    );
    item.click();
    fixture.detectChanges();

    // Simulate modal emitting posted
    component.onModalPosted({ reachTier: 'community' });

    expect(postedSpy).toHaveBeenCalledWith({
      feedbackType: 'challenge',
      reachTier: 'community',
    });
  });
});
```

**Step 2: Run tests to verify they fail**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate-feedback-trigger"`
Expected: FAIL — module not found

**Step 3: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-trigger.component.spec.ts
git commit -m "test(elohim): add failing tests for GateFeedbackTriggerComponent"
```

---

### Task 4: GateFeedbackTriggerComponent — implementation

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-trigger.component.ts`

**Step 1: Write the component**

```typescript
import {
  Component,
  EventEmitter,
  Output,
  ChangeDetectionStrategy,
  signal,
  input,
} from '@angular/core';

import {
  GateFeedbackModalComponent,
  type FeedbackType,
} from './gate-feedback-modal.component';
import type { ReachTier } from '../../services/gate-interaction.service';

interface MenuItem {
  type: FeedbackType;
  label: string;
}

const MENU_ITEMS: MenuItem[] = [
  { type: 'flag', label: 'Flag' },
  { type: 'challenge', label: 'Challenge' },
  { type: 'feedback', label: 'Feedback' },
];

@Component({
  selector: 'app-gate-feedback-trigger',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [GateFeedbackModalComponent],
  template: `
    <div class="feedback-trigger">
      <button
        class="btn-trigger"
        data-testid="feedback-trigger-btn"
        aria-label="Governance feedback"
        (click)="toggleMenu()"
      >
        &#x22EE;
      </button>

      @if (menuOpen()) {
        <div class="feedback-menu" data-testid="feedback-trigger-menu">
          @for (item of menuItems; track item.type) {
            <button
              class="feedback-menu-item"
              [attr.data-testid]="'feedback-menu-item-' + item.type"
              (click)="openModal(item.type)"
            >
              {{ item.label }}
            </button>
          }
        </div>
      }

      @if (modalOpen()) {
        <app-gate-feedback-modal
          [feedbackType]="activeFeedbackType()"
          [contentId]="contentId()"
          (posted)="onModalPosted($event)"
          (settled)="onModalSettled($event)"
          (closed)="closeModal()"
        ></app-gate-feedback-modal>
      }
    </div>
  `,
  styles: [
    `
      .feedback-trigger {
        position: relative;
        display: inline-block;
      }

      .btn-trigger {
        background: none;
        border: none;
        font-size: 1.25rem;
        line-height: 1;
        color: var(--text-secondary, #5f6368);
        cursor: pointer;
        padding: 0.25rem 0.5rem;
        border-radius: var(--radius-sm, 4px);
      }

      .btn-trigger:hover {
        background: var(--surface-secondary, #f8f9fa);
        color: var(--text-primary, #202124);
      }

      .feedback-menu {
        position: absolute;
        right: 0;
        top: 100%;
        z-index: 100;
        min-width: 140px;
        background: var(--surface-elevated, #fff);
        border: 1px solid var(--border-color, #e9ecef);
        border-radius: var(--radius-md, 8px);
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
        padding: 0.25rem 0;
      }

      .feedback-menu-item {
        display: block;
        width: 100%;
        padding: 0.5rem 1rem;
        font-size: 0.875rem;
        color: var(--text-primary, #202124);
        background: none;
        border: none;
        text-align: left;
        cursor: pointer;
      }

      .feedback-menu-item:hover {
        background: var(--surface-secondary, #f8f9fa);
      }
    `,
  ],
})
export class GateFeedbackTriggerComponent {
  readonly contentId = input('');

  @Output() feedbackPosted = new EventEmitter<{
    feedbackType: string;
    reachTier: ReachTier;
  }>();
  @Output() feedbackSettled = new EventEmitter<{
    boundary: string;
    appealPath: string | null;
  }>();

  readonly menuOpen = signal(false);
  readonly modalOpen = signal(false);
  readonly activeFeedbackType = signal<FeedbackType>('feedback');
  readonly menuItems = MENU_ITEMS;

  toggleMenu(): void {
    this.menuOpen.update((v) => !v);
  }

  openModal(type: FeedbackType): void {
    this.activeFeedbackType.set(type);
    this.menuOpen.set(false);
    this.modalOpen.set(true);
  }

  closeModal(): void {
    this.modalOpen.set(false);
  }

  onModalPosted(event: { reachTier: ReachTier }): void {
    this.feedbackPosted.emit({
      feedbackType: this.activeFeedbackType(),
      reachTier: event.reachTier,
    });
  }

  onModalSettled(event: {
    boundary: string;
    appealPath: string | null;
  }): void {
    this.feedbackSettled.emit(event);
  }
}
```

**Step 2: Run tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate-feedback-trigger"`
Expected: All tests PASS

**Step 3: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-trigger.component.ts
git commit -m "feat(elohim): implement GateFeedbackTriggerComponent — context menu + modal trigger"
```

---

### Task 5: Barrel export + service index update

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/gate-feedback/index.ts`
- Modify: `app/elohim-app/src/app/elohim/services/index.ts` (add re-export of FeedbackType)

**Step 1: Create barrel export**

```typescript
export { GateFeedbackModalComponent } from './gate-feedback-modal.component';
export type { FeedbackType } from './gate-feedback-modal.component';
export { GateFeedbackTriggerComponent } from './gate-feedback-trigger.component';
```

**Step 2: Verify build still works**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate-feedback"`
Expected: All gate-feedback tests PASS

**Step 3: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/gate-feedback/index.ts
git commit -m "chore(elohim): add barrel export for gate-feedback components"
```

---

### Task 6: Auto-close on posted + delay test

**Files:**
- Modify: `app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-modal.component.ts`
- Modify: `app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-modal.component.spec.ts`

**Step 1: Add failing test for auto-close delay**

Add to the spec file:

```typescript
it('should emit closed ~800ms after posted', fakeAsync(() => {
  const closedSpy = vi.fn();
  component.closed.subscribe(closedSpy);

  // Simulate posted event from card
  component.onPosted({ reachTier: 'community' });
  fixture.detectChanges();

  expect(closedSpy).not.toHaveBeenCalled();

  tick(800);
  expect(closedSpy).toHaveBeenCalled();
}));

it('should NOT auto-close on settled', fakeAsync(() => {
  const closedSpy = vi.fn();
  component.closed.subscribe(closedSpy);

  component.onSettled({ boundary: 'harm', appealPath: null });
  fixture.detectChanges();

  tick(2000);
  expect(closedSpy).not.toHaveBeenCalled();
}));
```

**Step 2: Run tests to verify failure**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate-feedback-modal"`
Expected: FAIL — closes immediately or never

**Step 3: Update onPosted to auto-close**

In `gate-feedback-modal.component.ts`, update `onPosted`:

```typescript
import { ..., inject, DestroyRef } from '@angular/core';

// Inside the class:
private readonly destroyRef = inject(DestroyRef);

onPosted(event: { reachTier: ReachTier }): void {
  this.posted.emit(event);
  const timer = setTimeout(() => this.closed.emit(), 800);
  this.destroyRef.onDestroy(() => clearTimeout(timer));
}
```

**Step 4: Run tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate-feedback-modal"`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/gate-feedback/
git commit -m "feat(elohim): auto-close feedback modal 800ms after posted, stay open on settled"
```

---

### Task 7: Full integration test — trigger → modal → card flow

**Files:**
- Modify: `app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-trigger.component.spec.ts`

**Step 1: Add integration test**

Append to existing spec:

```typescript
it('should pass correct feedbackType through to modal card', () => {
  // Open menu → click Flag
  const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
    '[data-testid="feedback-trigger-btn"]',
  );
  btn.click();
  fixture.detectChanges();
  const flagItem: HTMLButtonElement = fixture.nativeElement.querySelector(
    '[data-testid="feedback-menu-item-flag"]',
  );
  flagItem.click();
  fixture.detectChanges();

  // Modal should show "Flag Content" title
  const title = fixture.nativeElement.querySelector(
    '[data-testid="feedback-modal-title"]',
  );
  expect(title.textContent.trim()).toBe('Flag Content');

  // Textarea should have flag placeholder
  const textarea = fixture.nativeElement.querySelector(
    '[data-testid="artifact-textarea"]',
  );
  expect(textarea.placeholder).toBe('Describe the issue...');
});
```

**Step 2: Run all gate-feedback tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate-feedback"`
Expected: All tests PASS

**Step 3: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-trigger.component.spec.ts
git commit -m "test(elohim): add integration test for trigger → modal → card flow"
```

---

### Task 8: Run full test suite + lint

**Step 1: Run all elohim-app tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts`
Expected: All existing tests still pass, gate-feedback tests pass

**Step 2: Run lint**

Run: `cd app/elohim-app && pnpm run lint`
Expected: No new lint errors

**Step 3: Fix any issues found, commit if needed**
