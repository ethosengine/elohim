# Journal Writing Surface — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a distraction-free journal writing surface at `/shefa/journal/:id` with a collapsible elohim chat sidebar using the GateArtifactCard for input and canned responses.

**Architecture:** Two-panel layout in the shefa pillar. Journal entries are content nodes (`contentType: 'journal'`) stored via existing `StorageApiService.createContent/updateContent`. Elohim sidebar reuses `GateArtifactCardComponent` without `gateApiCall` — canned responses for this sprint. Autosave via debounced PATCH.

**Tech Stack:** Angular 19 (signals, OnPush, standalone components), Vitest, existing StorageApiService + content CRUD.

---

## Task 1: CannedResponseService

### Files
- Create: `app/elohim-app/src/app/shefa/services/canned-response.service.ts`
- Create: `app/elohim-app/src/app/shefa/services/canned-response.service.spec.ts`

### Step 1: Write the failing tests

```typescript
// canned-response.service.spec.ts
import { describe, it, expect } from 'vitest';

import { CannedResponseService } from './canned-response.service';

describe('CannedResponseService', () => {
  let service: CannedResponseService;

  beforeEach(() => {
    service = new CannedResponseService();
  });

  it('should respond to "what do you think" with reflection prompt', () => {
    const response = service.respond('So what do you think about this?');
    expect(response).toContain('working through something');
  });

  it('should respond to "help" with patience message', () => {
    const response = service.respond('I need help with this');
    expect(response).toContain('Take your time');
  });

  it('should respond to "publish" with routing hint', () => {
    const response = service.respond('Should I publish this?');
    expect(response).toContain('share');
  });

  it('should respond to "done" with affirmation', () => {
    const response = service.respond('I think I am done');
    expect(response).toContain('reads well');
  });

  it('should respond to "delete" with gentle pushback', () => {
    const response = service.respond('I want to delete this');
    expect(response).toContain('draft');
  });

  it('should return default for unrecognized input', () => {
    const response = service.respond('The sky is blue');
    expect(response).toContain('here when you need me');
  });

  it('should be case insensitive', () => {
    const response = service.respond('WHAT DO YOU THINK');
    expect(response).toContain('working through something');
  });
});
```

### Step 2: Run tests to verify they fail

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "canned-response" 2>&1 | tail -15
```

Expected: FAIL — module not found.

### Step 3: Write the implementation

```typescript
// canned-response.service.ts
import { Injectable } from '@angular/core';

interface ResponseRule {
  keywords: string[];
  response: string;
}

const RULES: ResponseRule[] = [
  {
    keywords: ['what do you think', 'thoughts', 'opinion'],
    response:
      "I can see you're working through something here. When you're ready, I can help you find where this belongs.",
  },
  {
    keywords: ['help', 'stuck', "don't know", 'confused'],
    response: 'Take your time. Sometimes the writing itself is the point.',
  },
  {
    keywords: ['publish', 'share', 'post', 'send'],
    response:
      "When you're ready to share this, we can talk about where it would have the most impact. That's a conversation for when it feels right to you.",
  },
  {
    keywords: ['done', 'finished', 'ready', 'complete'],
    response: 'It reads well. What would you like to do with it?',
  },
  {
    keywords: ['delete', 'trash', 'scrap', 'throw away'],
    response: 'Your words, your call. Want to keep it as a draft instead?',
  },
];

const DEFAULT_RESPONSE = "I'm here when you need me.";

@Injectable({ providedIn: 'root' })
export class CannedResponseService {
  respond(text: string): string {
    const lower = text.toLowerCase();
    for (const rule of RULES) {
      if (rule.keywords.some(kw => lower.includes(kw))) {
        return rule.response;
      }
    }
    return DEFAULT_RESPONSE;
  }
}
```

### Step 4: Run tests to verify they pass

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "canned-response" 2>&1 | tail -15
```

Expected: 7 tests PASS.

### Step 5: Commit

```bash
git add app/elohim-app/src/app/shefa/services/canned-response.service.ts \
       app/elohim-app/src/app/shefa/services/canned-response.service.spec.ts
git commit -m "feat(shefa): add CannedResponseService for journal sidebar"
```

---

## Task 2: ElohimMessageListComponent

### Files
- Create: `app/elohim-app/src/app/shefa/components/journal-page/elohim-message-list.component.ts`
- Create: `app/elohim-app/src/app/shefa/components/journal-page/elohim-message-list.component.spec.ts`

### Step 1: Write the failing tests

```typescript
// elohim-message-list.component.spec.ts
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { vi, describe, it, expect, beforeEach } from 'vitest';

import { ElohimMessageListComponent, type ChatMessage } from './elohim-message-list.component';

describe('ElohimMessageListComponent', () => {
  let component: ElohimMessageListComponent;
  let fixture: ComponentFixture<ElohimMessageListComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ElohimMessageListComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(ElohimMessageListComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should render empty state when no messages', () => {
    const el = fixture.nativeElement;
    expect(el.querySelectorAll('[data-testid^="chat-message-"]').length).toBe(0);
  });

  it('should render human messages right-aligned', () => {
    fixture.componentRef.setInput('messages', [
      { role: 'human', text: 'Hello there' },
    ] satisfies ChatMessage[]);
    fixture.detectChanges();

    const msg = fixture.nativeElement.querySelector('[data-testid="chat-message-0"]');
    expect(msg).toBeTruthy();
    expect(msg.textContent).toContain('Hello there');
    expect(msg.classList.contains('human')).toBe(true);
  });

  it('should render elohim messages left-aligned', () => {
    fixture.componentRef.setInput('messages', [
      { role: 'elohim', text: "I'm here" },
    ] satisfies ChatMessage[]);
    fixture.detectChanges();

    const msg = fixture.nativeElement.querySelector('[data-testid="chat-message-0"]');
    expect(msg).toBeTruthy();
    expect(msg.textContent).toContain("I'm here");
    expect(msg.classList.contains('elohim')).toBe(true);
  });

  it('should render multiple messages in order', () => {
    fixture.componentRef.setInput('messages', [
      { role: 'human', text: 'First' },
      { role: 'elohim', text: 'Second' },
      { role: 'human', text: 'Third' },
    ] satisfies ChatMessage[]);
    fixture.detectChanges();

    const msgs = fixture.nativeElement.querySelectorAll('[data-testid^="chat-message-"]');
    expect(msgs.length).toBe(3);
    expect(msgs[0].textContent).toContain('First');
    expect(msgs[2].textContent).toContain('Third');
  });
});
```

### Step 2: Run tests to verify they fail

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "elohim-message-list" 2>&1 | tail -15
```

### Step 3: Write the implementation

```typescript
// elohim-message-list.component.ts
import {
  Component,
  ChangeDetectionStrategy,
  input,
  ElementRef,
  viewChild,
  afterNextRender,
  effect,
} from '@angular/core';

export interface ChatMessage {
  role: 'human' | 'elohim';
  text: string;
}

@Component({
  selector: 'app-elohim-message-list',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="message-list" #scrollContainer data-testid="message-list">
      @for (msg of messages(); track $index) {
        <div
          class="chat-bubble"
          [class.human]="msg.role === 'human'"
          [class.elohim]="msg.role === 'elohim'"
          [attr.data-testid]="'chat-message-' + $index"
        >
          {{ msg.text }}
        </div>
      }
    </div>
  `,
  styles: [`
    .message-list {
      display: flex;
      flex-direction: column;
      gap: 0.75rem;
      overflow-y: auto;
      flex: 1;
      padding: 1rem;
    }

    .chat-bubble {
      max-width: 85%;
      padding: 0.625rem 0.875rem;
      border-radius: 12px;
      font-size: 0.875rem;
      line-height: 1.4;
      word-wrap: break-word;
    }

    .chat-bubble.human {
      align-self: flex-end;
      background: var(--primary-light, #e8f0fe);
      color: var(--text-primary, #202124);
    }

    .chat-bubble.elohim {
      align-self: flex-start;
      background: var(--surface-variant, #f1f3f4);
      color: var(--text-primary, #202124);
    }
  `],
})
export class ElohimMessageListComponent {
  readonly messages = input<ChatMessage[]>([]);
  private readonly scrollContainer = viewChild<ElementRef>('scrollContainer');

  constructor() {
    effect(() => {
      const msgs = this.messages();
      if (msgs.length > 0) {
        const container = this.scrollContainer()?.nativeElement;
        if (container) {
          requestAnimationFrame(() => {
            container.scrollTop = container.scrollHeight;
          });
        }
      }
    });
  }
}
```

### Step 4: Run tests to verify they pass

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "elohim-message-list" 2>&1 | tail -15
```

### Step 5: Commit

```bash
git add app/elohim-app/src/app/shefa/components/journal-page/
git commit -m "feat(shefa): add ElohimMessageListComponent for sidebar chat"
```

---

## Task 3: ElohimSidebarComponent

### Files
- Create: `app/elohim-app/src/app/shefa/components/journal-page/elohim-sidebar.component.ts`
- Create: `app/elohim-app/src/app/shefa/components/journal-page/elohim-sidebar.component.spec.ts`

### Step 1: Write the failing tests

```typescript
// elohim-sidebar.component.spec.ts
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { of } from 'rxjs';
import { vi, describe, it, expect, beforeEach } from 'vitest';

import { ElohimSidebarComponent } from './elohim-sidebar.component';

describe('ElohimSidebarComponent', () => {
  let component: ElohimSidebarComponent;
  let fixture: ComponentFixture<ElohimSidebarComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ElohimSidebarComponent],
      providers: [{ provide: HttpClient, useValue: { post: vi.fn().mockReturnValue(of({})) } }],
    }).compileComponents();

    fixture = TestBed.createComponent(ElohimSidebarComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  // --- Collapse / expand ---

  it('should be collapsed by default', () => {
    expect(component.expanded()).toBe(false);
    const panel = fixture.nativeElement.querySelector('[data-testid="sidebar-panel"]');
    expect(panel).toBeFalsy();
  });

  it('should show toggle button when collapsed', () => {
    const btn = fixture.nativeElement.querySelector('[data-testid="sidebar-toggle"]');
    expect(btn).toBeTruthy();
  });

  it('should expand when toggle clicked', () => {
    const btn: HTMLButtonElement = fixture.nativeElement.querySelector('[data-testid="sidebar-toggle"]');
    btn.click();
    fixture.detectChanges();

    expect(component.expanded()).toBe(true);
    const panel = fixture.nativeElement.querySelector('[data-testid="sidebar-panel"]');
    expect(panel).toBeTruthy();
  });

  it('should show message list when expanded', () => {
    component.expanded.set(true);
    fixture.detectChanges();

    const list = fixture.nativeElement.querySelector('app-elohim-message-list');
    expect(list).toBeTruthy();
  });

  it('should show artifact card input when expanded', () => {
    component.expanded.set(true);
    fixture.detectChanges();

    const card = fixture.nativeElement.querySelector('app-gate-artifact-card');
    expect(card).toBeTruthy();
  });

  // --- Message flow ---

  it('should add human message and canned response on posted', () => {
    component.expanded.set(true);
    fixture.detectChanges();

    component.onMessagePosted('What do you think?');

    expect(component.messages().length).toBe(2);
    expect(component.messages()[0].role).toBe('human');
    expect(component.messages()[0].text).toBe('What do you think?');
    expect(component.messages()[1].role).toBe('elohim');
  });

  it('should collapse when close button clicked', () => {
    component.expanded.set(true);
    fixture.detectChanges();

    const btn: HTMLButtonElement = fixture.nativeElement.querySelector('[data-testid="sidebar-close"]');
    btn.click();
    fixture.detectChanges();

    expect(component.expanded()).toBe(false);
  });
});
```

### Step 2: Run tests to verify they fail

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "elohim-sidebar" 2>&1 | tail -15
```

### Step 3: Write the implementation

```typescript
// elohim-sidebar.component.ts
import {
  Component,
  ChangeDetectionStrategy,
  inject,
  signal,
} from '@angular/core';

import { GateArtifactCardComponent } from '@app/elohim/components/gate-artifact-card/gate-artifact-card.component';
import { CannedResponseService } from '../../services/canned-response.service';
import { ElohimMessageListComponent, type ChatMessage } from './elohim-message-list.component';

@Component({
  selector: 'app-elohim-sidebar',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [GateArtifactCardComponent, ElohimMessageListComponent],
  template: `
    @if (!expanded()) {
      <button
        class="sidebar-tab"
        data-testid="sidebar-toggle"
        aria-label="Open elohim sidebar"
        (click)="expanded.set(true)"
      >
        <span class="tab-icon">&#x2726;</span>
      </button>
    } @else {
      <div class="sidebar-panel" data-testid="sidebar-panel">
        <div class="sidebar-header">
          <span class="sidebar-title">Elohim</span>
          <button
            class="btn-close"
            data-testid="sidebar-close"
            aria-label="Close sidebar"
            (click)="expanded.set(false)"
          >&times;</button>
        </div>
        <app-elohim-message-list [messages]="messages()" />
        <div class="sidebar-input">
          <app-gate-artifact-card
            [placeholder]="'Talk to your elohim...'"
            [mutationType]="'journal-chat'"
            [contextMetadata]="{}"
            (posted)="onPosted()"
          />
        </div>
      </div>
    }
  `,
  styles: [`
    :host {
      display: block;
      height: 100%;
    }

    .sidebar-tab {
      position: absolute;
      right: 0;
      top: 50%;
      transform: translateY(-50%);
      background: var(--surface-elevated, #fff);
      border: 1px solid var(--border, #dadce0);
      border-right: none;
      border-radius: 8px 0 0 8px;
      padding: 0.75rem 0.5rem;
      cursor: pointer;
      z-index: 10;
    }

    .tab-icon {
      font-size: 1.25rem;
      color: var(--text-secondary, #5f6368);
    }

    .sidebar-tab:hover .tab-icon {
      color: var(--primary, #4285f4);
    }

    .sidebar-panel {
      display: flex;
      flex-direction: column;
      height: 100%;
      width: 300px;
      border-left: 1px solid var(--border, #dadce0);
      background: var(--surface, #fff);
    }

    .sidebar-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0.75rem 1rem;
      border-bottom: 1px solid var(--border, #dadce0);
    }

    .sidebar-title {
      font-weight: 500;
      font-size: 0.875rem;
      color: var(--text-secondary, #5f6368);
    }

    .btn-close {
      background: none;
      border: none;
      font-size: 1.25rem;
      cursor: pointer;
      color: var(--text-secondary, #5f6368);
      padding: 0.25rem;
    }

    .btn-close:hover {
      color: var(--text-primary, #202124);
    }

    .sidebar-input {
      border-top: 1px solid var(--border, #dadce0);
      padding: 0.5rem;
    }
  `],
})
export class ElohimSidebarComponent {
  private readonly cannedResponse = inject(CannedResponseService);
  private readonly cardRef = signal<GateArtifactCardComponent | null>(null);

  readonly expanded = signal(false);
  readonly messages = signal<ChatMessage[]>([]);

  onPosted(): void {
    // The card doesn't expose the text via the posted event directly,
    // so we access it via the interaction service's draftText
    // For now, we use onMessagePosted called from the card's posted handler
  }

  onMessagePosted(text: string): void {
    const response = this.cannedResponse.respond(text);
    this.messages.update(msgs => [
      ...msgs,
      { role: 'human' as const, text },
      { role: 'elohim' as const, text: response },
    ]);
  }
}
```

**Note:** The card's `(posted)` event emits `{ reachTier }` not the text. We need to capture the text before it posts. The implementation agent should read the `GateArtifactCardComponent` and `GateInteractionService` to find how to extract the draft text — `interaction.draftText()` holds it. The sidebar should use `viewChild(GateArtifactCardComponent)` and read `card.interaction.draftText()` in the `(posted)` handler. Update the template to:

```html
<app-gate-artifact-card
  #sidebarCard
  [placeholder]="'Talk to your elohim...'"
  [mutationType]="'journal-chat'"
  [contextMetadata]="{}"
  (posted)="onCardPosted()"
/>
```

And the handler:

```typescript
private readonly sidebarCard = viewChild<GateArtifactCardComponent>('sidebarCard');

onCardPosted(): void {
  const card = this.sidebarCard();
  if (card) {
    const text = card.interaction.draftText();
    this.onMessagePosted(text);
    card.interaction.reset();
  }
}
```

### Step 4: Run tests to verify they pass

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "elohim-sidebar" 2>&1 | tail -15
```

### Step 5: Commit

```bash
git add app/elohim-app/src/app/shefa/components/journal-page/elohim-sidebar*
git commit -m "feat(shefa): add ElohimSidebarComponent — collapsible chat panel

Collapsed by default with toggle tab. Expanded shows message list +
GateArtifactCard input. Canned responses via CannedResponseService."
```

---

## Task 4: JournalEditorComponent

### Files
- Create: `app/elohim-app/src/app/shefa/components/journal-page/journal-editor.component.ts`
- Create: `app/elohim-app/src/app/shefa/components/journal-page/journal-editor.component.spec.ts`

### Step 1: Write the failing tests

```typescript
// journal-editor.component.spec.ts
import { ComponentFixture, TestBed, fakeAsync, tick } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { of } from 'rxjs';
import { vi, describe, it, expect, beforeEach } from 'vitest';

import { JournalEditorComponent } from './journal-editor.component';

describe('JournalEditorComponent', () => {
  let component: JournalEditorComponent;
  let fixture: ComponentFixture<JournalEditorComponent>;
  let httpMock: { patch: ReturnType<typeof vi.fn>; post: ReturnType<typeof vi.fn>; get: ReturnType<typeof vi.fn> };

  beforeEach(async () => {
    httpMock = {
      patch: vi.fn().mockReturnValue(of({})),
      post: vi.fn().mockReturnValue(of({})),
      get: vi.fn().mockReturnValue(of({})),
    };

    await TestBed.configureTestingModule({
      imports: [JournalEditorComponent],
      providers: [{ provide: HttpClient, useValue: httpMock }],
    }).compileComponents();

    fixture = TestBed.createComponent(JournalEditorComponent);
    component = fixture.componentInstance;
    fixture.componentRef.setInput('contentId', 'journal-1');
    fixture.detectChanges();
  });

  it('should render title input', () => {
    const input = fixture.nativeElement.querySelector('[data-testid="journal-title"]');
    expect(input).toBeTruthy();
  });

  it('should render body textarea', () => {
    const textarea = fixture.nativeElement.querySelector('[data-testid="journal-body"]');
    expect(textarea).toBeTruthy();
  });

  it('should show save status indicator', () => {
    const status = fixture.nativeElement.querySelector('[data-testid="save-status"]');
    expect(status).toBeTruthy();
  });

  it('should update title signal on input', () => {
    const input: HTMLInputElement = fixture.nativeElement.querySelector('[data-testid="journal-title"]');
    input.value = 'My Thoughts';
    input.dispatchEvent(new Event('input'));
    fixture.detectChanges();

    expect(component.title()).toBe('My Thoughts');
  });

  it('should update body signal on input', () => {
    const textarea: HTMLTextAreaElement = fixture.nativeElement.querySelector('[data-testid="journal-body"]');
    textarea.value = 'Some writing here';
    textarea.dispatchEvent(new Event('input'));
    fixture.detectChanges();

    expect(component.body()).toBe('Some writing here');
  });

  it('should save title on blur', () => {
    component.title.set('New Title');
    const input: HTMLInputElement = fixture.nativeElement.querySelector('[data-testid="journal-title"]');
    input.dispatchEvent(new Event('blur'));
    fixture.detectChanges();

    expect(httpMock.patch).toHaveBeenCalled();
  });

  it('should debounce body autosave', fakeAsync(() => {
    component.body.set('First draft');
    component.onBodyInput();
    tick(500);
    expect(httpMock.patch).not.toHaveBeenCalled();

    tick(1500);
    expect(httpMock.patch).toHaveBeenCalled();
  }));
});
```

### Step 2: Run tests to verify they fail

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "journal-editor" 2>&1 | tail -15
```

### Step 3: Write the implementation

```typescript
// journal-editor.component.ts
import {
  Component,
  ChangeDetectionStrategy,
  DestroyRef,
  inject,
  input,
  signal,
} from '@angular/core';
import { Subject, debounceTime, takeUntil } from 'rxjs';

import { StorageApiService } from '@app/elohim/services/storage-api.service';

@Component({
  selector: 'app-journal-editor',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="journal-editor">
      <input
        class="journal-title"
        type="text"
        [value]="title()"
        (input)="onTitleInput($event)"
        (blur)="saveTitle()"
        placeholder="Untitled"
        data-testid="journal-title"
        aria-label="Journal title"
      />
      <textarea
        class="journal-body"
        [value]="body()"
        (input)="onBodyInputEvent($event)"
        placeholder="Start writing..."
        data-testid="journal-body"
        aria-label="Journal body"
      ></textarea>
      <div class="save-status" data-testid="save-status">
        {{ saveStatus() }}
      </div>
    </div>
  `,
  styles: [`
    .journal-editor {
      display: flex;
      flex-direction: column;
      height: 100%;
      padding: 2rem;
      max-width: 720px;
      margin: 0 auto;
    }

    .journal-title {
      border: none;
      outline: none;
      font-size: 2rem;
      font-weight: 600;
      color: var(--text-primary, #202124);
      margin-bottom: 1.5rem;
      background: transparent;
    }

    .journal-title::placeholder {
      color: var(--text-disabled, #bdc1c6);
    }

    .journal-body {
      border: none;
      outline: none;
      font-size: 1.0625rem;
      line-height: 1.7;
      color: var(--text-primary, #202124);
      background: transparent;
      resize: none;
      flex: 1;
      font-family: inherit;
    }

    .journal-body::placeholder {
      color: var(--text-disabled, #bdc1c6);
    }

    .save-status {
      font-size: 0.75rem;
      color: var(--text-tertiary, #80868b);
      padding-top: 0.5rem;
      text-align: right;
    }
  `],
})
export class JournalEditorComponent {
  private readonly storageApi = inject(StorageApiService);
  private readonly destroyRef = inject(DestroyRef);
  private readonly destroy$ = new Subject<void>();
  private readonly bodyChange$ = new Subject<void>();

  readonly contentId = input.required<string>();
  readonly title = signal('');
  readonly body = signal('');
  readonly saveStatus = signal('');

  constructor() {
    this.bodyChange$
      .pipe(debounceTime(1500), takeUntil(this.destroy$))
      .subscribe(() => this.saveBody());

    this.destroyRef.onDestroy(() => {
      this.destroy$.next();
      this.destroy$.complete();
    });
  }

  loadContent(title: string, body: string): void {
    this.title.set(title);
    this.body.set(body);
  }

  onTitleInput(event: Event): void {
    this.title.set((event.target as HTMLInputElement).value);
  }

  onBodyInputEvent(event: Event): void {
    this.body.set((event.target as HTMLTextAreaElement).value);
    this.onBodyInput();
  }

  onBodyInput(): void {
    this.saveStatus.set('');
    this.bodyChange$.next();
  }

  saveTitle(): void {
    const id = this.contentId();
    if (!id) return;
    this.saveStatus.set('Saving...');
    this.storageApi.updateContent(id, { title: this.title() }).subscribe({
      next: () => this.saveStatus.set('Saved'),
      error: () => this.saveStatus.set('Save failed'),
    });
  }

  private saveBody(): void {
    const id = this.contentId();
    if (!id) return;
    this.saveStatus.set('Saving...');
    this.storageApi.updateContent(id, { contentBody: this.body() }).subscribe({
      next: () => this.saveStatus.set('Saved'),
      error: () => this.saveStatus.set('Save failed'),
    });
  }
}
```

### Step 4: Run tests to verify they pass

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "journal-editor" 2>&1 | tail -15
```

### Step 5: Commit

```bash
git add app/elohim-app/src/app/shefa/components/journal-page/journal-editor*
git commit -m "feat(shefa): add JournalEditorComponent with debounced autosave

Title saves on blur, body autosaves after 1.5s debounce via
StorageApiService.updateContent PATCH. Distraction-free styling."
```

---

## Task 5: JournalPageComponent + Route

### Files
- Create: `app/elohim-app/src/app/shefa/components/journal-page/journal-page.component.ts`
- Create: `app/elohim-app/src/app/shefa/components/journal-page/journal-page.component.spec.ts`
- Modify: `app/elohim-app/src/app/shefa/shefa.routes.ts` (add journal route)

### Step 1: Write the failing tests

```typescript
// journal-page.component.spec.ts
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute } from '@angular/router';
import { HttpClient } from '@angular/common/http';
import { of } from 'rxjs';
import { vi, describe, it, expect, beforeEach } from 'vitest';

import { JournalPageComponent } from './journal-page.component';

describe('JournalPageComponent', () => {
  let component: JournalPageComponent;
  let fixture: ComponentFixture<JournalPageComponent>;
  let httpMock: Record<string, ReturnType<typeof vi.fn>>;

  beforeEach(async () => {
    httpMock = {
      get: vi.fn().mockReturnValue(of({
        id: 'journal-1',
        title: 'My Journal',
        contentBody: 'Some thoughts',
        contentType: 'journal',
      })),
      patch: vi.fn().mockReturnValue(of({})),
      post: vi.fn().mockReturnValue(of({})),
    };

    await TestBed.configureTestingModule({
      imports: [JournalPageComponent],
      providers: [
        { provide: HttpClient, useValue: httpMock },
        {
          provide: ActivatedRoute,
          useValue: { paramMap: of(new Map([['id', 'journal-1']])) },
        },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(JournalPageComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should render two-panel layout', () => {
    const layout = fixture.nativeElement.querySelector('[data-testid="journal-layout"]');
    expect(layout).toBeTruthy();
  });

  it('should contain the editor', () => {
    const editor = fixture.nativeElement.querySelector('app-journal-editor');
    expect(editor).toBeTruthy();
  });

  it('should contain the sidebar', () => {
    const sidebar = fixture.nativeElement.querySelector('app-elohim-sidebar');
    expect(sidebar).toBeTruthy();
  });
});
```

### Step 2: Run tests to verify they fail

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "journal-page" 2>&1 | tail -15
```

### Step 3: Write the implementation

```typescript
// journal-page.component.ts
import {
  Component,
  ChangeDetectionStrategy,
  inject,
  viewChild,
  OnInit,
} from '@angular/core';
import { ActivatedRoute } from '@angular/router';
import { map, switchMap } from 'rxjs';

import { StorageApiService } from '@app/elohim/services/storage-api.service';
import { JournalEditorComponent } from './journal-editor.component';
import { ElohimSidebarComponent } from './elohim-sidebar.component';

@Component({
  selector: 'app-journal-page',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [JournalEditorComponent, ElohimSidebarComponent],
  template: `
    <div class="journal-layout" data-testid="journal-layout">
      <div class="journal-main">
        <app-journal-editor [contentId]="contentId" />
      </div>
      <div class="journal-sidebar">
        <app-elohim-sidebar />
      </div>
    </div>
  `,
  styles: [`
    .journal-layout {
      display: flex;
      height: 100%;
      position: relative;
    }

    .journal-main {
      flex: 1;
      overflow-y: auto;
    }

    .journal-sidebar {
      position: relative;
      flex-shrink: 0;
    }
  `],
})
export class JournalPageComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly storageApi = inject(StorageApiService);
  private readonly editor = viewChild(JournalEditorComponent);

  contentId = '';

  ngOnInit(): void {
    this.route.paramMap
      .pipe(
        map(params => params.get('id') ?? ''),
        switchMap(id => {
          this.contentId = id;
          return this.storageApi.getContent(id);
        }),
      )
      .subscribe(content => {
        if (content) {
          const editor = this.editor();
          if (editor) {
            editor.loadContent(
              (content as Record<string, string>).title ?? '',
              (content as Record<string, string>).contentBody ?? '',
            );
          }
        }
      });
  }
}
```

### Step 4: Add the route to shefa.routes.ts

In `app/elohim-app/src/app/shefa/shefa.routes.ts`, add inside the `children` array (after the `devices` route):

```typescript
{
  path: 'journal/:id',
  loadComponent: async () =>
    import('./components/journal-page/journal-page.component').then(
      m => m.JournalPageComponent,
    ),
  data: { title: 'Shefa - Journal' },
},
```

### Step 5: Run tests to verify they pass

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "journal-page" 2>&1 | tail -15
```

### Step 6: Commit

```bash
git add app/elohim-app/src/app/shefa/components/journal-page/journal-page* \
       app/elohim-app/src/app/shefa/shefa.routes.ts
git commit -m "feat(shefa): add JournalPageComponent at /shefa/journal/:id

Two-panel layout: editor (left) + elohim sidebar (right).
Loads content node by ID, populates editor with title and body."
```

---

## Task 6: Integration Verification

### Step 1: Run all journal tests

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "journal|canned-response|elohim-message|elohim-sidebar" 2>&1 | tail -30
```

Expected: All journal-related tests pass (~25+ tests).

### Step 2: Run full Angular test suite

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts 2>&1 | tail -20
```

Expected: No regressions.

### Step 3: Run lint

```bash
cd app/elohim-app && pnpm run lint 2>&1 | grep -E "journal|canned-response|elohim-sidebar|elohim-message"
```

Expected: No lint errors in our files.

### Step 4: Verify route is reachable

Check that the route is registered by reviewing the routes file:

```bash
grep -n "journal" app/elohim-app/src/app/shefa/shefa.routes.ts
```

Expected: `journal/:id` route present in shefa children.
