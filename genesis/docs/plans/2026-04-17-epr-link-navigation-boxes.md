# EPR Link Navigation Boxes (Surface 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface the typed `EprRelationship[]` carried in each content's EPR Head as navigable cards with reach + resilience badges, grouped by relationship type, rendered below the content body in both content-viewer and lesson-view.

**Architecture:** Two new standalone Angular components in the `elohim` pillar — `EprRelationshipCardComponent` renders one relationship as a card (title, type badge, reach badge, resilience badge, click-to-navigate); `EprRelationshipsPanelComponent` takes a full `EprRelationship[]`, groups by type, and renders a section per type. Both consumers (content-viewer, lesson-view) already load content; they gain a single call to `EprResolverService.resolveEprHead(contentId)` and render the panel. No new backend work — `EprHead.relationships` is already populated, `ResilienceService.getContentResilience(id)` already exists, `EprResolverService.resolve()` already returns reach via `ContentView`.

**Tech Stack:** Angular 19 standalone components, OnPush change detection, RxJS, Vitest + Angular TestBed, data-testid attributes, existing `@app/elohim` barrel exports.

---

## P2P Design Gate Declaration

**This plan introduces no new data entity.** It is a pure presentation layer change — new Angular components that read entities already notarized elsewhere. The p2p-design-gate skill fires "when creating, storing, referencing, or syncing data entities"; this plan only *references* (reads) existing entities.

Source-of-truth map for every entity consumed:

| Entity consumed | Category | Canonical source | Read path (already wired) |
|-----------------|----------|------------------|---------------------------|
| `EprHead` | A (notarized DHT entry) | imagodei DNA, IPLD DAG-CBOR encoded | `EprResolverService.resolveEprHead(id)` → `GET /epr-head/:id` |
| `EprRelationship` | A2 (derived link, sub-field of `EprHead.relationships`) | Same DHT entry as parent `EprHead` | Read through parent — no separate fetch |
| `ContentView` (used for reach + title) | C (read-model projection) | Projected from lamad DNA content entries into elohim-storage | `EprResolverService.resolve(id)` → `GET /db/content/:id` |
| `ResilienceView` (used for resilience badge) | C (read-model projection) | Computed from stewardship + shard distribution records | `ResilienceService.getContentResilience(id)` → `GET /api/v1/resilience/:id` |

**What this plan does NOT add:**
- No new DHT entry types (lamad stays at ~73/~100, mishpat stays at 11/~100).
- No new migrations, no new SQLite tables, no new columns.
- No new HTTP routes. All four read paths above exist on `dev`.
- No new sync messages, no new gossip payloads, no new protocol fields.
- No new Rust code at all.

**Identifiers this plan introduces** are UI-scoped only: two Angular selectors (`app-epr-relationship-card`, `app-epr-relationships-panel`) and a handful of `data-testid` attributes.

**P2P-native choices in the presentation layer:** card navigation keys off `relationship.target` (the stable content id that resolves to `EprHead.id`) and uses the existing `relationship.targetCid` field when present. Routing goes through `EprResolverService.resolveInContext()` so links adapt to the learner's path context (in-path / cross-path / standalone) rather than assuming a relational shape.

## Out of Scope

- Refactoring the existing reach/resilience inline badges on content-viewer/lesson-view headers into a shared component (keep them as-is; the new cards copy the icon logic, not the components).
- Surface 3 context menu (three-dot on each card) — designed but separate picks item.
- Pagination / lazy loading of cards — if a node has >20 relationships we render them all; address only if it becomes a performance issue.
- Backend changes. `EprHead.relationships` is already populated and served.

---

## File Structure

| File | Role |
|------|------|
| `app/elohim-app/src/app/elohim/components/epr-relationship-card/epr-relationship-card.component.ts` | **NEW.** Single card. Resolves target content + resilience, renders title + type/reach/resilience badges, routerLink. |
| `app/elohim-app/src/app/elohim/components/epr-relationship-card/epr-relationship-card.component.spec.ts` | **NEW.** Vitest + TestBed. Covers: loading state, resolved state, error fallback, click navigation, badge rendering. |
| `app/elohim-app/src/app/elohim/components/epr-relationships-panel/epr-relationships-panel.component.ts` | **NEW.** Takes `EprRelationship[]`, groups by `.type`, renders heading + card grid per group. Empty-state hidden. |
| `app/elohim-app/src/app/elohim/components/epr-relationships-panel/epr-relationships-panel.component.spec.ts` | **NEW.** Vitest + TestBed. Covers: grouping by type, type ordering, empty list → nothing rendered, single card per group. |
| `app/elohim-app/src/app/elohim/index.ts` | **MODIFY.** Add barrel exports for the two new components. |
| `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts` | **MODIFY.** Inject `EprResolverService`, fetch head on content load, store `eprHead` signal/property, import the panel component. |
| `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.spec.ts` | **MODIFY.** Add test covering panel render with mocked head. |
| `app/elohim-app/src/app/lamad/components/lesson-view/lesson-view.component.ts` | **MODIFY.** Same pattern as content-viewer. |
| `app/elohim-app/src/app/lamad/components/lesson-view/lesson-view.component.spec.ts` | **MODIFY.** Add test covering panel render with mocked head. |
| `genesis/a2o/features/lamad/epr-link-navigation.feature` | **NEW.** Two scenarios: "Learner sees typed relationship cards beneath a concept" and "Clicking a relationship card navigates to the target concept". |

**Relationship types to handle:** `PREREQUISITE`, `TEACHES`, `CONTAINS`, `REFERENCES`. Unknown types render in a generic "Related" group. Display order: PREREQUISITE → TEACHES → CONTAINS → REFERENCES → other.

---

## Task 1: `EprRelationshipCardComponent` — failing test

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/epr-relationship-card/epr-relationship-card.component.ts` (skeleton)
- Create: `app/elohim-app/src/app/elohim/components/epr-relationship-card/epr-relationship-card.component.spec.ts`

- [ ] **Step 1: Create the component skeleton (no logic yet)**

Create `epr-relationship-card.component.ts` with the minimum for imports to resolve:

```typescript
import { ChangeDetectionStrategy, Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';
import type { EprRelationship } from '../../models/epr-head.model';

@Component({
  selector: 'app-epr-relationship-card',
  standalone: true,
  imports: [CommonModule, RouterModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `<div data-testid="epr-relationship-card"></div>`,
})
export class EprRelationshipCardComponent {
  @Input({ required: true }) relationship!: EprRelationship;
}
```

- [ ] **Step 2: Write the failing test**

Create `epr-relationship-card.component.spec.ts`:

```typescript
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { of } from 'rxjs';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { EprResolverService, type ResolvedContent } from '../../services/epr-resolver.service';
import { ResilienceService, type ResilienceView } from '@app/lamad/services/resilience.service';
import type { EprRelationship } from '../../models/epr-head.model';

import { EprRelationshipCardComponent } from './epr-relationship-card.component';

describe('EprRelationshipCardComponent', () => {
  let fixture: ComponentFixture<EprRelationshipCardComponent>;
  let resolverMock: { resolve: ReturnType<typeof vi.fn> };
  let resilienceMock: { getContentResilience: ReturnType<typeof vi.fn> };

  const relationship: EprRelationship = {
    type: 'PREREQUISITE',
    target: 'systems-thinking',
  };

  const resolvedContent = {
    ref: { tier: 'head', id: 'systems-thinking' },
    content: {
      id: 'systems-thinking',
      title: 'Systems Thinking',
      description: 'An introduction.',
      contentType: 'concept',
      reach: 'community',
      tags: [],
    },
    blobUrl: null,
    route: ['/resource', 'systems-thinking'],
  } as unknown as ResolvedContent;

  const resilienceView: Partial<ResilienceView> = {
    contentId: 'systems-thinking',
    stewardship: { stewardCount: 3, allocations: [] },
    health: { score: 0.9, canSurviveFailures: 2, status: 'healthy' },
  };

  beforeEach(async () => {
    resolverMock = {
      resolve: vi.fn().mockReturnValue(of(resolvedContent)),
    };
    resilienceMock = {
      getContentResilience: vi.fn().mockReturnValue(of(resilienceView)),
    };

    await TestBed.configureTestingModule({
      imports: [EprRelationshipCardComponent],
      providers: [
        provideRouter([]),
        { provide: EprResolverService, useValue: resolverMock },
        { provide: ResilienceService, useValue: resilienceMock },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(EprRelationshipCardComponent);
    fixture.componentRef.setInput('relationship', relationship);
    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();
  });

  it('renders target title after resolution', () => {
    const title = fixture.nativeElement.querySelector('[data-testid="epr-rel-card-title"]');
    expect(title?.textContent).toContain('Systems Thinking');
  });

  it('renders relationship type label', () => {
    const label = fixture.nativeElement.querySelector('[data-testid="epr-rel-card-type"]');
    expect(label?.textContent).toContain('Prerequisite');
  });

  it('renders reach badge from resolved content', () => {
    const reach = fixture.nativeElement.querySelector('[data-testid="epr-rel-card-reach"]');
    expect(reach).toBeTruthy();
    expect(reach?.getAttribute('title')).toContain('community');
  });

  it('renders resilience badge with steward count', () => {
    const res = fixture.nativeElement.querySelector('[data-testid="epr-rel-card-resilience"]');
    expect(res).toBeTruthy();
    expect(res?.getAttribute('title')).toContain('3');
  });

  it('links to the resolved route', () => {
    const link = fixture.nativeElement.querySelector('[data-testid="epr-relationship-card"]');
    expect(link?.getAttribute('href')).toBe('/resource/systems-thinking');
  });

  it('falls back to target id when resolution returns null', async () => {
    resolverMock.resolve.mockReturnValue(of(null));
    fixture = TestBed.createComponent(EprRelationshipCardComponent);
    fixture.componentRef.setInput('relationship', relationship);
    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();

    const title = fixture.nativeElement.querySelector('[data-testid="epr-rel-card-title"]');
    expect(title?.textContent).toContain('systems-thinking');
  });
});
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts epr-relationship-card.component.spec`
Expected: 6 failures (elements not found / content empty).

- [ ] **Step 4: Commit the failing test**

```bash
git add app/elohim-app/src/app/elohim/components/epr-relationship-card/
git commit -m "test(elohim): add failing spec for EprRelationshipCardComponent"
```

---

## Task 2: Implement `EprRelationshipCardComponent`

**Files:**
- Modify: `app/elohim-app/src/app/elohim/components/epr-relationship-card/epr-relationship-card.component.ts`

- [ ] **Step 1: Replace the skeleton with the full implementation**

```typescript
import { CommonModule } from '@angular/common';
import {
  ChangeDetectionStrategy,
  ChangeDetectorRef,
  Component,
  DestroyRef,
  Input,
  OnChanges,
  SimpleChanges,
  inject,
} from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { RouterModule } from '@angular/router';

import { ResilienceService, type ResilienceView } from '@app/lamad/services/resilience.service';
import { EMPTY, catchError } from 'rxjs';

import type { EprRelationship } from '../../models/epr-head.model';
import { EprResolverService, type ResolvedContent } from '../../services/epr-resolver.service';

const RELATIONSHIP_LABELS: Record<string, string> = {
  PREREQUISITE: 'Prerequisite',
  TEACHES: 'Teaches',
  CONTAINS: 'Contains',
  REFERENCES: 'References',
};

@Component({
  selector: 'app-epr-relationship-card',
  standalone: true,
  imports: [CommonModule, RouterModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <a
      class="rel-card"
      data-testid="epr-relationship-card"
      [routerLink]="route"
      [attr.data-target]="relationship.target"
    >
      <span class="rel-type" data-testid="epr-rel-card-type">{{ typeLabel }}</span>
      <h4 class="rel-title" data-testid="epr-rel-card-title">{{ title }}</h4>
      <p *ngIf="description" class="rel-desc">{{ description }}</p>
      <div class="rel-meta">
        <span
          *ngIf="reach"
          class="rel-reach"
          data-testid="epr-rel-card-reach"
          [title]="'Reach: ' + reach"
          >{{ reachIcon }}</span
        >
        <span
          *ngIf="resilience"
          class="rel-resilience"
          data-testid="epr-rel-card-resilience"
          [title]="resilienceTooltip"
          >{{ resilienceIcon }}</span
        >
      </div>
    </a>
  `,
  styles: [
    `
      :host {
        display: block;
      }
      .rel-card {
        display: flex;
        flex-direction: column;
        gap: 4px;
        padding: 12px 14px;
        border: 1px solid var(--rel-card-border, #e5e7eb);
        border-radius: 8px;
        text-decoration: none;
        color: inherit;
        background: var(--primary-surface, #fff);
        transition: box-shadow 0.15s, border-color 0.15s;
      }
      .rel-card:hover {
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
        border-color: var(--rel-card-hover-border, #cbd5e1);
      }
      .rel-type {
        font-size: 0.7em;
        font-weight: 600;
        letter-spacing: 0.05em;
        text-transform: uppercase;
        opacity: 0.65;
      }
      .rel-title {
        margin: 0;
        font-size: 1em;
        font-weight: 600;
      }
      .rel-desc {
        margin: 0;
        font-size: 0.85em;
        opacity: 0.75;
        display: -webkit-box;
        -webkit-line-clamp: 2;
        -webkit-box-orient: vertical;
        overflow: hidden;
      }
      .rel-meta {
        display: flex;
        gap: 8px;
        margin-top: 4px;
        font-size: 0.95em;
      }
      .rel-reach,
      .rel-resilience {
        cursor: help;
      }
    `,
  ],
})
export class EprRelationshipCardComponent implements OnChanges {
  @Input({ required: true }) relationship!: EprRelationship;

  title = '';
  description?: string;
  reach?: string;
  resilience?: ResilienceView;
  route: string[] = [];

  private readonly resolver = inject(EprResolverService);
  private readonly resilienceSvc = inject(ResilienceService);
  private readonly cdr = inject(ChangeDetectorRef);
  private readonly destroyRef = inject(DestroyRef);

  get typeLabel(): string {
    const t = this.relationship?.type ?? '';
    return RELATIONSHIP_LABELS[t] ?? t.charAt(0) + t.slice(1).toLowerCase();
  }

  get reachIcon(): string {
    switch (this.reach) {
      case 'commons':
      case 'public':
        return '◉';
      case 'community':
        return '◎';
      case 'trusted':
        return '◍';
      case 'personal':
      case 'private':
        return '○';
      default:
        return '·';
    }
  }

  get resilienceIcon(): string {
    const count = this.resilience?.stewardship?.stewardCount ?? 0;
    if (count >= 3) return '●';
    if (count >= 1) return '◐';
    return '○';
  }

  get resilienceTooltip(): string {
    const count = this.resilience?.stewardship?.stewardCount ?? 0;
    const status = this.resilience?.health?.status ?? 'unknown';
    return `Stewards: ${count} · Status: ${status}`;
  }

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['relationship']) {
      this.reset();
      this.loadTarget();
    }
  }

  private reset(): void {
    this.title = this.relationship.target;
    this.description = undefined;
    this.reach = undefined;
    this.resilience = undefined;
    this.route = ['/resource', this.relationship.target];
  }

  private loadTarget(): void {
    const id = this.relationship.target;

    this.resolver
      .resolve(id)
      .pipe(
        catchError(() => EMPTY),
        takeUntilDestroyed(this.destroyRef),
      )
      .subscribe((resolved: ResolvedContent | null) => {
        if (resolved) {
          this.title = resolved.content.title ?? id;
          this.description = resolved.content.description;
          this.reach = resolved.content.reach;
          this.route = resolved.route;
        }
        this.cdr.markForCheck();
      });

    this.resilienceSvc
      .getContentResilience(id)
      .pipe(
        catchError(() => EMPTY),
        takeUntilDestroyed(this.destroyRef),
      )
      .subscribe(view => {
        this.resilience = view;
        this.cdr.markForCheck();
      });
  }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts epr-relationship-card.component.spec`
Expected: all 6 tests pass.

- [ ] **Step 3: Lint**

Run: `cd app/elohim-app && pnpm exec eslint src/app/elohim/components/epr-relationship-card --ext .ts`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/epr-relationship-card/epr-relationship-card.component.ts
git commit -m "feat(elohim): render EprRelationship as card with reach+resilience badges"
```

---

## Task 3: `EprRelationshipsPanelComponent` — failing test

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/epr-relationships-panel/epr-relationships-panel.component.ts` (skeleton)
- Create: `app/elohim-app/src/app/elohim/components/epr-relationships-panel/epr-relationships-panel.component.spec.ts`

- [ ] **Step 1: Create the skeleton**

```typescript
import { ChangeDetectionStrategy, Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import type { EprRelationship } from '../../models/epr-head.model';

@Component({
  selector: 'app-epr-relationships-panel',
  standalone: true,
  imports: [CommonModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `<div data-testid="epr-relationships-panel"></div>`,
})
export class EprRelationshipsPanelComponent {
  @Input() relationships: EprRelationship[] = [];
}
```

- [ ] **Step 2: Write the failing test**

```typescript
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { of } from 'rxjs';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { EprResolverService } from '../../services/epr-resolver.service';
import { ResilienceService } from '@app/lamad/services/resilience.service';
import type { EprRelationship } from '../../models/epr-head.model';

import { EprRelationshipsPanelComponent } from './epr-relationships-panel.component';

describe('EprRelationshipsPanelComponent', () => {
  let fixture: ComponentFixture<EprRelationshipsPanelComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [EprRelationshipsPanelComponent],
      providers: [
        provideRouter([]),
        {
          provide: EprResolverService,
          useValue: { resolve: vi.fn().mockReturnValue(of(null)) },
        },
        {
          provide: ResilienceService,
          useValue: { getContentResilience: vi.fn().mockReturnValue(of(null)) },
        },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(EprRelationshipsPanelComponent);
  });

  function setRelationships(relationships: EprRelationship[]): void {
    fixture.componentRef.setInput('relationships', relationships);
    fixture.detectChanges();
  }

  it('renders nothing when relationships are empty', () => {
    setRelationships([]);
    const root = fixture.nativeElement.querySelector('[data-testid="epr-relationships-panel"]');
    expect(root).toBeNull();
  });

  it('groups relationships by type with a section per group', () => {
    setRelationships([
      { type: 'PREREQUISITE', target: 'a' },
      { type: 'PREREQUISITE', target: 'b' },
      { type: 'TEACHES', target: 'c' },
    ]);

    const groups = fixture.nativeElement.querySelectorAll('[data-testid="epr-rel-group"]');
    expect(groups.length).toBe(2);

    const prereqGroup = fixture.nativeElement.querySelector(
      '[data-testid="epr-rel-group"][data-type="PREREQUISITE"]',
    );
    const cards = prereqGroup?.querySelectorAll('[data-testid="epr-relationship-card"]');
    expect(cards?.length).toBe(2);
  });

  it('orders groups PREREQUISITE → TEACHES → CONTAINS → REFERENCES', () => {
    setRelationships([
      { type: 'REFERENCES', target: 'r' },
      { type: 'CONTAINS', target: 'c' },
      { type: 'PREREQUISITE', target: 'p' },
      { type: 'TEACHES', target: 't' },
    ]);

    const groups = Array.from(
      fixture.nativeElement.querySelectorAll('[data-testid="epr-rel-group"]'),
    ) as HTMLElement[];
    const types = groups.map(g => g.getAttribute('data-type'));
    expect(types).toEqual(['PREREQUISITE', 'TEACHES', 'CONTAINS', 'REFERENCES']);
  });

  it('renders unknown types in the trailing order', () => {
    setRelationships([
      { type: 'TEACHES', target: 't' },
      { type: 'CITES', target: 'x' },
    ]);
    const groups = Array.from(
      fixture.nativeElement.querySelectorAll('[data-testid="epr-rel-group"]'),
    ) as HTMLElement[];
    const types = groups.map(g => g.getAttribute('data-type'));
    expect(types).toEqual(['TEACHES', 'CITES']);
  });
});
```

- [ ] **Step 3: Run the test to verify failure**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts epr-relationships-panel.component.spec`
Expected: 4 failures (no groups rendered, panel renders empty div even when empty).

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/epr-relationships-panel/
git commit -m "test(elohim): add failing spec for EprRelationshipsPanelComponent"
```

---

## Task 4: Implement `EprRelationshipsPanelComponent`

**Files:**
- Modify: `app/elohim-app/src/app/elohim/components/epr-relationships-panel/epr-relationships-panel.component.ts`

- [ ] **Step 1: Replace the skeleton with the full implementation**

```typescript
import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, Input } from '@angular/core';

import type { EprRelationship } from '../../models/epr-head.model';

import { EprRelationshipCardComponent } from '../epr-relationship-card/epr-relationship-card.component';

const TYPE_ORDER = ['PREREQUISITE', 'TEACHES', 'CONTAINS', 'REFERENCES'];

const GROUP_LABELS: Record<string, string> = {
  PREREQUISITE: 'Prerequisites',
  TEACHES: 'This teaches',
  CONTAINS: 'Contains',
  REFERENCES: 'References',
};

interface RelationshipGroup {
  type: string;
  label: string;
  items: EprRelationship[];
}

@Component({
  selector: 'app-epr-relationships-panel',
  standalone: true,
  imports: [CommonModule, EprRelationshipCardComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <section
      *ngIf="groups.length > 0"
      class="rel-panel"
      data-testid="epr-relationships-panel"
    >
      <div
        *ngFor="let group of groups; trackBy: trackGroup"
        class="rel-group"
        data-testid="epr-rel-group"
        [attr.data-type]="group.type"
      >
        <h3 class="rel-group-heading">{{ group.label }}</h3>
        <div class="rel-cards">
          <app-epr-relationship-card
            *ngFor="let rel of group.items; trackBy: trackRel"
            [relationship]="rel"
          ></app-epr-relationship-card>
        </div>
      </div>
    </section>
  `,
  styles: [
    `
      :host {
        display: block;
      }
      .rel-panel {
        display: flex;
        flex-direction: column;
        gap: 24px;
        margin-top: 32px;
      }
      .rel-group-heading {
        margin: 0 0 12px;
        font-size: 0.85em;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        opacity: 0.7;
      }
      .rel-cards {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
        gap: 12px;
      }
    `,
  ],
})
export class EprRelationshipsPanelComponent {
  private _relationships: EprRelationship[] = [];
  groups: RelationshipGroup[] = [];

  @Input()
  set relationships(value: EprRelationship[] | null | undefined) {
    this._relationships = value ?? [];
    this.groups = this.buildGroups(this._relationships);
  }

  get relationships(): EprRelationship[] {
    return this._relationships;
  }

  trackGroup(_: number, group: RelationshipGroup): string {
    return group.type;
  }

  trackRel(_: number, rel: EprRelationship): string {
    return `${rel.type}:${rel.target}`;
  }

  private buildGroups(relationships: EprRelationship[]): RelationshipGroup[] {
    const byType = new Map<string, EprRelationship[]>();
    for (const rel of relationships) {
      const list = byType.get(rel.type) ?? [];
      list.push(rel);
      byType.set(rel.type, list);
    }

    const knownGroups: RelationshipGroup[] = TYPE_ORDER.filter(t => byType.has(t)).map(type => ({
      type,
      label: GROUP_LABELS[type] ?? type,
      items: byType.get(type)!,
    }));

    const unknownGroups: RelationshipGroup[] = Array.from(byType.keys())
      .filter(t => !TYPE_ORDER.includes(t))
      .map(type => ({
        type,
        label: this.humanizeType(type),
        items: byType.get(type)!,
      }));

    return [...knownGroups, ...unknownGroups];
  }

  private humanizeType(type: string): string {
    if (!type) return '';
    return type.charAt(0) + type.slice(1).toLowerCase().replace(/_/g, ' ');
  }
}
```

- [ ] **Step 2: Run the panel tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts epr-relationships-panel.component.spec`
Expected: all 4 tests pass.

- [ ] **Step 3: Run the full epr-relationship suite to confirm nothing regressed**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts epr-relationship`
Expected: all tests in both spec files pass.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/epr-relationships-panel/epr-relationships-panel.component.ts
git commit -m "feat(elohim): group typed EprRelationships into panel sections"
```

---

## Task 5: Barrel exports

**Files:**
- Modify: `app/elohim-app/src/app/elohim/index.ts`

- [ ] **Step 1: Locate the barrel exports**

Run: `grep -n "epr-link" app/elohim-app/src/app/elohim/index.ts`
Expected: at least one line referencing the existing `epr-link` component export.

- [ ] **Step 2: Add the two new component exports**

Append near the existing component exports:

```typescript
export { EprRelationshipCardComponent } from './components/epr-relationship-card/epr-relationship-card.component';
export { EprRelationshipsPanelComponent } from './components/epr-relationships-panel/epr-relationships-panel.component';
```

If `app/elohim-app/src/app/elohim/index.ts` does not exist, locate the barrel by running:
`find app/elohim-app/src/app/elohim -maxdepth 2 -name 'index.ts'`
and add the two exports to whichever file already re-exports components. If no such barrel exists, skip this task — consumers can import from deep paths (Task 6 and 7 show both styles).

- [ ] **Step 3: Verify the app still builds**

Run: `cd app/elohim-app && pnpm exec tsc --noEmit -p tsconfig.json`
Expected: no type errors.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/elohim/index.ts
git commit -m "chore(elohim): export relationship card + panel from barrel"
```

---

## Task 6: Wire panel into `ContentViewerComponent`

**Files:**
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts`
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.spec.ts`

- [ ] **Step 1: Import and add the panel to the component imports**

At the top of `content-viewer.component.ts`, add:

```typescript
import { EprRelationshipsPanelComponent } from '@app/elohim/components/epr-relationships-panel/epr-relationships-panel.component';
import { EprResolverService } from '@app/elohim/services/epr-resolver.service';
import type { EprHead, EprRelationship } from '@app/elohim/models/epr-head.model';
```

Add `EprRelationshipsPanelComponent` to the component's `imports: []` array in its `@Component` decorator.

- [ ] **Step 2: Add injection + state**

In the `ContentViewerComponent` class body (near other service injections):

```typescript
private readonly eprResolver = inject(EprResolverService);

eprRelationships: EprRelationship[] = [];
```

- [ ] **Step 3: Fetch the EPR Head when content loads**

Find the existing content-load success branch — the place that calls `this.loadRelatedNodes(contentNode.relatedNodeIds);` around line 401. Immediately after that call, add:

```typescript
this.loadEprRelationships(contentNode.id);
```

Then add this method to the class:

```typescript
private loadEprRelationships(contentId: string): void {
  this.eprRelationships = [];
  this.eprResolver
    .resolveEprHead(contentId)
    .pipe(takeUntilDestroyed(this.destroyRef))
    .subscribe((head: EprHead | null) => {
      this.eprRelationships = head?.relationships ?? [];
      this.cdr.markForCheck();
    });
}
```

If `takeUntilDestroyed` / `DestroyRef` / `cdr` aren't already imported/injected on this component, add them following the pattern in `EprRelationshipCardComponent` (Task 2). If the component uses a `destroy$` Subject pattern instead, use `takeUntil(this.destroy$)` — match what's already in this file.

- [ ] **Step 4: Render the panel in the template**

The template is the `template:` string in `content-viewer.component.ts` (or a separate `.html` file if one exists — search for `templateUrl`). Find the section that renders related nodes (search for `relatedNodes`) and insert BELOW it, before the closing tag of the content area:

```html
<app-epr-relationships-panel
  *ngIf="eprRelationships.length > 0"
  [relationships]="eprRelationships"
  data-testid="viewer-relationships-panel"
></app-epr-relationships-panel>
```

- [ ] **Step 5: Add a test covering panel render**

In `content-viewer.component.spec.ts`, add inside the existing `describe` block. First, check the existing test setup to see how `EprResolverService` is mocked (search for it in the file). If not already mocked, extend the `TestBed.configureTestingModule` providers:

```typescript
{
  provide: EprResolverService,
  useValue: {
    resolve: vi.fn().mockReturnValue(of(null)),
    resolveEprHead: vi.fn().mockReturnValue(
      of({
        version: 1,
        id: 'test-content',
        content: 'bafk-test',
        lamad: { title: 'Test', contentType: 'concept' },
        shefa: {},
        qahal: {},
        relationships: [{ type: 'PREREQUISITE', target: 'foo' }],
      }),
    ),
  },
},
```

Then add a test:

```typescript
it('renders the EPR relationships panel when the head has relationships', async () => {
  // load content as existing tests do; after fixture.detectChanges() + whenStable:
  fixture.detectChanges();
  await fixture.whenStable();
  fixture.detectChanges();

  const panel = fixture.nativeElement.querySelector(
    '[data-testid="viewer-relationships-panel"]',
  );
  expect(panel).toBeTruthy();
});
```

Adjust the test's content-load flow to match what the other tests in the file already do — reuse the existing arrangement helpers.

- [ ] **Step 6: Run content-viewer tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts content-viewer.component.spec`
Expected: all existing tests still pass plus the new one.

- [ ] **Step 7: Run lint**

Run: `cd app/elohim-app && pnpm exec eslint src/app/lamad/components/content-viewer --ext .ts,.html`
Expected: no errors.

- [ ] **Step 8: Commit**

```bash
git add app/elohim-app/src/app/lamad/components/content-viewer/
git commit -m "feat(lamad): render EPR relationships panel below content body"
```

---

## Task 7: Wire panel into `LessonViewComponent`

**Files:**
- Modify: `app/elohim-app/src/app/lamad/components/lesson-view/lesson-view.component.ts`
- Modify: `app/elohim-app/src/app/lamad/components/lesson-view/lesson-view.component.spec.ts`

Repeat the pattern from Task 6 in `lesson-view.component.ts`:

- [ ] **Step 1: Imports and component imports array**

Add to the top of `lesson-view.component.ts`:

```typescript
import { EprRelationshipsPanelComponent } from '@app/elohim/components/epr-relationships-panel/epr-relationships-panel.component';
import { EprResolverService } from '@app/elohim/services/epr-resolver.service';
import type { EprHead, EprRelationship } from '@app/elohim/models/epr-head.model';
```

Add `EprRelationshipsPanelComponent` to `imports: []`.

- [ ] **Step 2: Inject resolver + state**

```typescript
private readonly eprResolver = inject(EprResolverService);

eprRelationships: EprRelationship[] = [];
```

- [ ] **Step 3: Fetch EPR Head when the step's content loads**

Find the place in `lesson-view.component.ts` where a new step's content node finishes loading (search for where `contentNode` is assigned — likely inside a `switchMap`/subscribe on a step or resource ID change). Call a new helper there:

```typescript
this.loadEprRelationships(contentNode.id);
```

```typescript
private loadEprRelationships(contentId: string): void {
  this.eprRelationships = [];
  this.eprResolver
    .resolveEprHead(contentId)
    .pipe(takeUntilDestroyed(this.destroyRef))
    .subscribe((head: EprHead | null) => {
      this.eprRelationships = head?.relationships ?? [];
      this.cdr.markForCheck();
    });
}
```

Match the file's existing teardown pattern (Subject `destroy$` or `takeUntilDestroyed`).

- [ ] **Step 4: Render the panel in the template**

Find the template section that renders related concepts (search for `relatedConcepts` or the section around line 215-221 per the explore notes). Insert directly below the lesson body, above the footer controls:

```html
<app-epr-relationships-panel
  *ngIf="eprRelationships.length > 0"
  [relationships]="eprRelationships"
  data-testid="lesson-relationships-panel"
></app-epr-relationships-panel>
```

- [ ] **Step 5: Add a test**

In `lesson-view.component.spec.ts`, mirror the mock from Task 6 Step 5:

```typescript
{
  provide: EprResolverService,
  useValue: {
    resolve: vi.fn().mockReturnValue(of(null)),
    resolveEprHead: vi.fn().mockReturnValue(
      of({
        version: 1,
        id: 'lesson-step',
        content: 'bafk-test',
        lamad: { title: 'Test', contentType: 'concept' },
        shefa: {},
        qahal: {},
        relationships: [{ type: 'TEACHES', target: 'bar' }],
      }),
    ),
  },
},
```

And add:

```typescript
it('renders the relationships panel for the current step', async () => {
  fixture.detectChanges();
  await fixture.whenStable();
  fixture.detectChanges();

  const panel = fixture.nativeElement.querySelector(
    '[data-testid="lesson-relationships-panel"]',
  );
  expect(panel).toBeTruthy();
});
```

- [ ] **Step 6: Run lesson-view tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts lesson-view.component.spec`
Expected: all existing tests pass plus the new one.

- [ ] **Step 7: Lint**

Run: `cd app/elohim-app && pnpm exec eslint src/app/lamad/components/lesson-view --ext .ts,.html`
Expected: no errors.

- [ ] **Step 8: Commit**

```bash
git add app/elohim-app/src/app/lamad/components/lesson-view/
git commit -m "feat(lamad): render EPR relationships panel in lesson view"
```

---

## Task 8: a2o scenario + close loop

**Files:**
- Create: `genesis/a2o/features/lamad/epr-link-navigation.feature`

- [ ] **Step 1: Check whether a similar feature file already exists**

Run: `ls genesis/a2o/features/lamad | grep -i 'epr\|relation\|navigation'`

If a closely related file already exists, add the two new scenarios to it instead of creating a new file.

- [ ] **Step 2: Write the feature file**

```gherkin
Feature: EPR relationship navigation boxes
  Learners see the typed relationships declared in a concept's EPR Head
  as navigable cards with trust signals (reach + stewardship resilience).
  Relationships are grouped by type so prerequisites, sub-topics, and
  references are legible at a glance.

  Background:
    Given a learner has signed in
    And the concept "systems-thinking" has been seeded with an EPR Head
      | type         | target                    |
      | PREREQUISITE | feedback-loops            |
      | TEACHES      | mental-models             |
      | REFERENCES   | donella-meadows-essay     |

  Scenario: Learner sees typed relationship cards beneath a concept
    When the learner opens the "systems-thinking" concept
    Then a "Prerequisites" section is visible
    And it contains a card linking to "feedback-loops"
    And the card shows a reach badge and a resilience badge
    And a "This teaches" section is visible
    And a "References" section is visible

  Scenario: Clicking a relationship card navigates to the target concept
    Given the learner is viewing the "systems-thinking" concept
    When the learner clicks the "feedback-loops" prerequisite card
    Then the "feedback-loops" concept is displayed
    And the URL reflects the "feedback-loops" concept
```

- [ ] **Step 3: Append the intent to the dev-intent log**

Append a new line to `.claude/data/dev-intent.jsonl`:

```json
{"date":"2026-04-17","branch":"<current>","summary":"Surfaced EprHead.relationships as a navigation panel under content body in content-viewer and lesson-view. Two new standalone components in the elohim pillar (EprRelationshipCardComponent, EprRelationshipsPanelComponent). Each card shows a reach badge (sourced from ContentView.reach) and a resilience badge (sourced from ResilienceService stewardship count). Groups ordered PREREQUISITE → TEACHES → CONTAINS → REFERENCES.","impact":"Learners can now follow the typed relationships declared in EPR Heads with trust signals visible before clicking — previously the data existed but was invisible.","a2o_features":["genesis/a2o/features/lamad/epr-link-navigation.feature"]}
```

- [ ] **Step 4: Commit scenario + intent**

```bash
git add genesis/a2o/features/lamad/epr-link-navigation.feature .claude/data/dev-intent.jsonl
git commit -m "feat(a2o): scenarios for EPR relationship navigation cards"
```

---

## Task 9: Full quality gate

- [ ] **Step 1: Run the elohim-app test suite**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts`
Expected: all tests pass. Note any pre-existing failures unrelated to this plan; do not fix them here.

- [ ] **Step 2: Lint (TS + SCSS)**

Run: `cd app/elohim-app && pnpm run lint`
Run: `cd app/elohim-app && pnpm run lint:css`
Expected: no errors in files modified by this plan.

- [ ] **Step 3: Build**

Run: `cd app/elohim-app && pnpm run build`
Expected: build succeeds.

- [ ] **Step 4: Manual smoke test in dev server**

Start the stack (`pnpm run hc:start:seed` from `app/elohim-app`) and in another shell `pnpm start`. In the browser at `localhost:4200`, open any seeded concept that has relationships in its EPR Head. Confirm:

- A "Prerequisites" / "This teaches" / etc. section appears below the content body.
- Each card shows a title and two badge characters (reach + resilience).
- Hovering the badges reveals tooltips.
- Clicking a card navigates to the target concept.
- Lesson view (within a path) shows the same panel for the active step.

Record what you tested in the terminal transcript; if something doesn't work, fix it and re-run the impacted Vitest spec.

- [ ] **Step 5: Final commit if any fixes were needed**

```bash
git add -A
git commit -m "fix(lamad): address smoke-test findings for EPR relationship panel"
```

(Skip this step if nothing needed fixing.)

---

## Self-Review Checklist (for reviewer)

- [ ] Every new file listed in "File Structure" exists and is referenced by at least one task.
- [ ] `EprRelationshipCardComponent` resolves `ResolvedContent | null` and falls back to target id on null (Task 1 test "falls back to target id").
- [ ] `EprRelationshipsPanelComponent` renders nothing when `relationships` is empty (Task 3 test "renders nothing when relationships are empty").
- [ ] Group ordering PREREQUISITE → TEACHES → CONTAINS → REFERENCES covered by Task 3 test.
- [ ] Both content-viewer and lesson-view tests cover panel render (Tasks 6 and 7).
- [ ] a2o feature file covers both see-cards and click-navigation flows.
- [ ] No direct coupling from `epr-relationship-card` back to lamad components — only imports from `@app/lamad/services/resilience.service`, which is an acceptable lower-level dependency.
- [ ] No backend changes. No new DHT entry types. No new protocol surface. All consumed entities are notarized entries or read-model projections — see "P2P Design Gate Declaration" above; source-of-truth for each is documented there.
