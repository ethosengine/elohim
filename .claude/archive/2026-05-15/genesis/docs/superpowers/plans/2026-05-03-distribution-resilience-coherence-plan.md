# Distribution + Resilience Coherence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Graduate `<app-distribution-badge>` (light-up-topology sprint) into the elohim-library as `<elohim-distribution-badge>`, render it side-by-side with `<elohim-resilience-snapshot>` in the content-viewer header, swap the concept-card embed, delete the in-app duplicate, and update all consumers + a2o scenarios.

**Architecture:** Two single-purpose protocol-vocabulary widgets, both in elohim-library, both rendering side-by-side where users ask both questions. No data-shape merging. No substrate change. The component contract is unchanged — only the location, selector, and import path change.

**Tech Stack:** Angular 19 (standalone, OnPush, signals), Vitest, RxJS HttpClient. No new dependencies.

**Spec:** `genesis/docs/superpowers/specs/2026-05-03-distribution-resilience-coherence-design.md`

---

## P2P Design Gate — No New Substrate Artifacts

This plan is a **UI-only graduation pass**. It introduces:

- **No new DHT entry types.**
- **No new SQLite tables, migrations, or schemas.**
- **No new HTTP routes or API surfaces.**
- **No new view types in `views.rs` and no changes to `compose_*` services.**
- **No changes to substrate, doorway, holochain, or steward code.**

All references in this plan to `DistributionSummary`, `DistributionDetails`,
`/api/v1/blob/{hash}/distribution/details`, and the
`distribution-summary` / `distribution-details` JSON schemas are
**references to existing artifacts** that were classified, designed, and
landed in the parent light-up-topology sprint. Their P2P Design Gate
classifications (all Operational, Category C — read-projections from DHT
truth) are documented in the parent plan
(`genesis/docs/superpowers/plans/2026-05-01-light-up-the-topology-plan.md`)
and parent spec
(`genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-design.md`).

The work in this plan is entirely on the Angular side: graduate one
component + one service from `app/elohim-app` into `app/elohim-library`,
rename the selector to match library convention, render it side-by-side
with an existing widget, and delete the in-app duplicate.

| Class of artifact | Touched by this plan? |
|---|---|
| Holochain DNA / zome / DHT entry types | No |
| Rust storage tables, migrations, view structs | No |
| HTTP route handlers, manifest entries | No |
| JSON schemas in `elohim/sdk/schemas/v1/` | No |
| ts-rs / schema:codegen:ts outputs | No |
| Angular components, services, templates | **Yes** — moved between projects |
| a2o feature files | Yes — selector string update only |

The post-tool-use P2P design audit hook will flag references to existing
routes / schemas in the body of this plan; those references describe
**what's being consumed**, not **what's being created**. This declaration
is the canonical source of truth for the gate.

---

## File Structure

### New files (in elohim-library)

```
app/elohim-library/projects/elohim-service/src/distribution/
  distribution-badge/
    distribution-badge.component.ts          // <elohim-distribution-badge>
    distribution-badge.component.html
    distribution-badge.component.scss
    distribution-badge.component.spec.ts
  distribution.service.ts                    // getDetails(blobHash): Promise<DistributionDetails>
  distribution.service.spec.ts
```

### Modified files

```
app/elohim-library/projects/elohim-service/src/public-api.ts
  + export DistributionBadgeComponent + DistributionService

app/elohim-app/src/app/lamad/components/concept-card/concept-card.component.ts
  selector swap: <app-distribution-badge> → <elohim-distribution-badge>
  import swap: @app/elohim/components/distribution-badge/... → @elohim/service/public-api

app/elohim-app/src/app/lamad/components/concept-card/concept-card.component.spec.ts
  swap testid assertion target if any references the old selector

app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts
  + import DistributionBadgeComponent from @elohim/service/public-api
  + add to component.imports

app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.html
  + render <elohim-distribution-badge> next to <elohim-resilience-snapshot>
    in the header (line ~71), gated on node?.distribution

app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.spec.ts
  + assert both widgets render in header when both data sources hydrate

genesis/a2o/features/resilience/observable-distribution.feature
  selector swap in the two scenarios that reference app-distribution-badge

app/elohim-app/src/app/lamad/models/content-node.model.ts
  doc-comment swap: <app-distribution-badge> → <elohim-distribution-badge>
```

### Deleted files (after consumers migrate)

```
app/elohim-app/src/app/elohim/components/distribution-badge/
  distribution-badge.component.ts
  distribution-badge.component.html
  distribution-badge.component.scss
  distribution-badge.component.spec.ts

app/elohim-app/src/app/elohim/services/distribution.service.ts
app/elohim-app/src/app/elohim/services/distribution.service.spec.ts
```

---

## Phase 1 — Graduate the badge into elohim-library

### Task 1: Scaffold the library distribution module + service

**Files:**
- Create: `app/elohim-library/projects/elohim-service/src/distribution/distribution.service.ts`
- Create: `app/elohim-library/projects/elohim-service/src/distribution/distribution.service.spec.ts`

- [ ] **Step 1: Read the existing in-app DistributionService to confirm the contract**

```bash
cat app/elohim-app/src/app/elohim/services/distribution.service.ts
cat app/elohim-app/src/app/elohim/services/distribution.service.spec.ts
```

Expected: a small service with `getDetails(blobHash: string): Promise<DistributionDetails>` calling `GET /api/v1/blob/{blobHash}/distribution/details`. Spec uses HttpTestingController.

- [ ] **Step 2: Create the library service file**

Write `app/elohim-library/projects/elohim-service/src/distribution/distribution.service.ts`:

```ts
import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { firstValueFrom } from 'rxjs';

import type { DistributionDetails } from '../generated/distribution-details';

/**
 * Fetches the lazy "deep tier" of distribution data for a single blob.
 *
 * The cheap inline tier (`DistributionSummary`) is hydrated on every
 * EPR head response and read directly from `node.distribution`. This
 * service is only invoked when the user asks for more — typically on
 * tooltip expand of `<elohim-distribution-badge>`.
 */
@Injectable({ providedIn: 'root' })
export class DistributionService {
  private readonly http = inject(HttpClient);

  async getDetails(blobHash: string): Promise<DistributionDetails> {
    return firstValueFrom(
      this.http.get<DistributionDetails>(`/api/v1/blob/${blobHash}/distribution/details`),
    );
  }
}
```

- [ ] **Step 3: Write the failing service spec**

Write `app/elohim-library/projects/elohim-service/src/distribution/distribution.service.spec.ts`:

```ts
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import { DistributionService } from './distribution.service';

describe('DistributionService', () => {
  let service: DistributionService;
  let http: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting()],
    });
    service = TestBed.inject(DistributionService);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  it('GETs /api/v1/blob/{hash}/distribution/details', async () => {
    const promise = service.getDetails('sha256-abc');
    const req = http.expectOne('/api/v1/blob/sha256-abc/distribution/details');
    expect(req.request.method).toBe('GET');
    req.flush({
      summary: {
        replicaCount: 2,
        replicaTarget: 3,
        replicaHealth: 'at_risk',
        projectorCount: 0,
        reachClass: 'household',
        diversityHint: { kind: 'none', value: '' },
        thisFetchSource: 'projected_via_doorway',
        lastVerifiedSeconds: 30,
      },
      replicaPeers: [],
      projectorIdentities: [],
      placementGaps: [],
      recentProjectionEvents: [],
    });
    const result = await promise;
    expect(result.summary.replicaCount).toBe(2);
  });
});
```

- [ ] **Step 4: Run the spec to verify it passes**

```bash
cd app/elohim-library/projects/elohim-service
pnpm test -- distribution.service
```

Expected: 1 test passing.

- [ ] **Step 5: Commit**

```bash
git add app/elohim-library/projects/elohim-service/src/distribution/
git commit -m "feat(elohim-service): graduate DistributionService into library

Lazy fetch of /api/v1/blob/{hash}/distribution/details. Mirrors the
ResilienceService pattern. Per coherence design — distribution is its
own dimension; service lives next to its widget in the library."
```

### Task 2: Move the badge component into the library

**Files:**
- Create: `app/elohim-library/projects/elohim-service/src/distribution/distribution-badge/distribution-badge.component.ts`
- Create: `app/elohim-library/projects/elohim-service/src/distribution/distribution-badge/distribution-badge.component.html`
- Create: `app/elohim-library/projects/elohim-service/src/distribution/distribution-badge/distribution-badge.component.scss`
- Create: `app/elohim-library/projects/elohim-service/src/distribution/distribution-badge/distribution-badge.component.spec.ts`
- Reference (do NOT delete yet): `app/elohim-app/src/app/elohim/components/distribution-badge/*`

- [ ] **Step 1: Read the existing component as the source**

```bash
cat app/elohim-app/src/app/elohim/components/distribution-badge/distribution-badge.component.ts
cat app/elohim-app/src/app/elohim/components/distribution-badge/distribution-badge.component.html
cat app/elohim-app/src/app/elohim/components/distribution-badge/distribution-badge.component.scss
cat app/elohim-app/src/app/elohim/components/distribution-badge/distribution-badge.component.spec.ts
```

- [ ] **Step 2: Create the library component .ts with renamed selector**

Write the new component file. Key differences from the in-app version:
- `selector: 'elohim-distribution-badge'` (was `'app-distribution-badge'`)
- Imports `DistributionService` from `'../distribution.service'` (was `'../../services/distribution.service'`)
- Imports types from `'../../generated/distribution-summary'` and `'../../generated/distribution-details'` (was `'@app/generated/...'`)

```ts
import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, Input, inject, signal } from '@angular/core';

import type { DistributionDetails } from '../../generated/distribution-details';
import type { DistributionSummary } from '../../generated/distribution-summary';
import { DistributionService } from '../distribution.service';

@Component({
  selector: 'elohim-distribution-badge',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './distribution-badge.component.html',
  styleUrls: ['./distribution-badge.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DistributionBadgeComponent {
  @Input({ required: true }) summary!: DistributionSummary;
  @Input() blobHash?: string;

  protected readonly expanded = signal(false);
  protected readonly details = signal<DistributionDetails | null>(null);
  protected readonly loadingDetails = signal(false);
  protected readonly Math = Math;

  private readonly distribution = inject(DistributionService);

  async onTooltipExpand(): Promise<void> {
    if (this.expanded()) return;
    this.expanded.set(true);
    if (!this.blobHash || this.details()) return;
    this.loadingDetails.set(true);
    try {
      const d = await this.distribution.getDetails(this.blobHash);
      this.details.set(d);
    } finally {
      this.loadingDetails.set(false);
    }
  }

  reachIcon(reach: DistributionSummary['reachClass']): string {
    if (reach === 'private') return '🔒 private';
    if (reach === 'intimate') return '🔒 peer-only';
    if (reach === 'public') return '🌐 public';
    return reach;
  }

  dotIndices(count: number): number[] {
    const n = Math.min(4, count);
    return Array.from({ length: n }, (_, i) => i);
  }
}
```

- [ ] **Step 3: Copy the .html and .scss verbatim from the in-app version**

```bash
cp app/elohim-app/src/app/elohim/components/distribution-badge/distribution-badge.component.html \
   app/elohim-library/projects/elohim-service/src/distribution/distribution-badge/distribution-badge.component.html

cp app/elohim-app/src/app/elohim/components/distribution-badge/distribution-badge.component.scss \
   app/elohim-library/projects/elohim-service/src/distribution/distribution-badge/distribution-badge.component.scss
```

The HTML uses the component's properties only — no selector references — so it ports without modification.

- [ ] **Step 4: Create the spec, mirroring the in-app version with adjusted imports**

Write `app/elohim-library/projects/elohim-service/src/distribution/distribution-badge/distribution-badge.component.spec.ts`:

```ts
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import { DistributionBadgeComponent } from './distribution-badge.component';
import type { DistributionSummary } from '../../generated/distribution-summary';

const mockSummary: DistributionSummary = {
  replicaCount: 3,
  replicaTarget: 4,
  replicaHealth: 'at_risk',
  projectorCount: 1,
  reachClass: 'public',
  diversityHint: { kind: 'region_metro', value: ['us-central'] },
  thisFetchSource: 'projected_via_doorway',
  lastVerifiedSeconds: 30,
};

describe('DistributionBadgeComponent (elohim-library)', () => {
  let fixture: ComponentFixture<DistributionBadgeComponent>;
  let http: HttpTestingController;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [DistributionBadgeComponent],
      providers: [provideHttpClient(), provideHttpClientTesting()],
    }).compileComponents();
    fixture = TestBed.createComponent(DistributionBadgeComponent);
    http = TestBed.inject(HttpTestingController);
    fixture.componentRef.setInput('summary', mockSummary);
    fixture.detectChanges();
  });

  afterEach(() => http.verify());

  it('renders with data-testid="distribution-badge"', () => {
    expect(
      fixture.nativeElement.querySelector('[data-testid="distribution-badge"]'),
    ).toBeTruthy();
  });

  it('renders replica count', () => {
    const el = fixture.nativeElement.querySelector('[data-testid="distribution-badge-replica-count"]');
    expect(el?.textContent?.trim()).toBe('3');
  });

  it('lazy-fetches details on first expand when blobHash is set', async () => {
    fixture.componentRef.setInput('blobHash', 'sha256-xyz');
    fixture.detectChanges();
    const root = fixture.nativeElement.querySelector('[data-testid="distribution-badge"]');
    root.dispatchEvent(new MouseEvent('mouseenter'));
    await fixture.whenStable();
    const req = http.expectOne('/api/v1/blob/sha256-xyz/distribution/details');
    req.flush({
      summary: mockSummary,
      replicaPeers: [],
      projectorIdentities: [],
      placementGaps: [],
      recentProjectionEvents: [],
    });
  });

  it('does not fetch when blobHash is absent', async () => {
    const root = fixture.nativeElement.querySelector('[data-testid="distribution-badge"]');
    root.dispatchEvent(new MouseEvent('mouseenter'));
    await fixture.whenStable();
    http.expectNone('/api/v1/blob/undefined/distribution/details');
  });
});
```

- [ ] **Step 5: Run the spec and verify it passes**

```bash
cd app/elohim-library/projects/elohim-service
pnpm test -- distribution-badge.component
```

Expected: 4 tests passing. If "module not found" on `../../generated/distribution-summary`, run `pnpm run schema:codegen:ts` from the repo root first to regenerate types into the library.

- [ ] **Step 6: Commit**

```bash
git add app/elohim-library/projects/elohim-service/src/distribution/distribution-badge/
git commit -m "feat(elohim-service): graduate <elohim-distribution-badge> into library

Single-purpose widget for the distribution dimension. Lazy details fetch
via DistributionService. OnPush + signals. Selector renamed from
app-distribution-badge to elohim-distribution-badge to match library
convention. Per coherence design."
```

### Task 3: Export from public-api

**Files:**
- Modify: `app/elohim-library/projects/elohim-service/src/public-api.ts`

- [ ] **Step 1: Read the current public-api**

```bash
cat app/elohim-library/projects/elohim-service/src/public-api.ts
```

- [ ] **Step 2: Add the new exports**

Append to `public-api.ts`:

```ts
export { DistributionService } from './distribution/distribution.service';
export { DistributionBadgeComponent } from './distribution/distribution-badge/distribution-badge.component';
```

- [ ] **Step 3: Build the library to verify no type errors**

```bash
cd app/elohim-library
pnpm exec ng build elohim-service 2>&1 | tail -10
```

Expected: build succeeds. If the library has a "build watch" config, run `--watch=false`.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-library/projects/elohim-service/src/public-api.ts
git commit -m "feat(elohim-service): export DistributionBadgeComponent + Service

Consumers import from @elohim/service/public-api alongside
ResilienceSnapshotComponent + ResilienceService."
```

---

## Phase 2 — Wire consumers to the library

### Task 4: Swap concept-card to the library widget

**Files:**
- Modify: `app/elohim-app/src/app/lamad/components/concept-card/concept-card.component.ts`
- Modify: `app/elohim-app/src/app/lamad/components/concept-card/concept-card.component.spec.ts`

- [ ] **Step 1: Read the current import in concept-card**

```bash
grep -n "DistributionBadgeComponent\|distribution-badge\|@app/elohim/components/distribution-badge" \
  app/elohim-app/src/app/lamad/components/concept-card/concept-card.component.ts
```

Expected: import from `@app/elohim/components/distribution-badge/distribution-badge.component`.

- [ ] **Step 2: Replace the import**

In `concept-card.component.ts`, replace:

```ts
import { DistributionBadgeComponent } from '@app/elohim/components/distribution-badge/distribution-badge.component';
```

with:

```ts
import { DistributionBadgeComponent } from '@elohim/service/public-api';
```

- [ ] **Step 3: Replace the selector in the template (still inside the .ts file)**

In the same file, find:

```html
<app-distribution-badge [summary]="concept.distribution"></app-distribution-badge>
```

Replace with:

```html
<elohim-distribution-badge [summary]="concept.distribution"></elohim-distribution-badge>
```

- [ ] **Step 4: Update the spec assertion**

Read `concept-card.component.spec.ts`. The T50 test block looks for `[data-testid="distribution-badge"]` — that testid is unchanged on the new widget, so the assertion passes as-is. The describe block is named "renders <app-distribution-badge>...". Change the describe block title only:

```ts
it('renders <elohim-distribution-badge> when concept.distribution is present', () => {
```

Verify with grep:

```bash
grep -n "app-distribution-badge\|elohim-distribution-badge" \
  app/elohim-app/src/app/lamad/components/concept-card/concept-card.component.spec.ts
```

Expected: only `elohim-distribution-badge` references remain.

- [ ] **Step 5: Run the concept-card spec and verify it passes**

```bash
cd app/elohim-app
pnpm exec vitest run --config vite.config.ts src/app/lamad/components/concept-card/concept-card.component.spec.ts
```

Expected: 6 tests passing.

- [ ] **Step 6: Commit**

```bash
git add app/elohim-app/src/app/lamad/components/concept-card/
git commit -m "refactor(concept-card): swap to <elohim-distribution-badge> from library

Per coherence design — distribution badge graduated into elohim-library.
Same widget, same testid, new selector + import path."
```

### Task 5: Add the distribution badge to content-viewer header

**Files:**
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts`
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.html`

- [ ] **Step 1: Add the import in content-viewer.component.ts**

After the existing `ResilienceSnapshotComponent` import (currently from `@elohim/service/public-api`), update the same import line to include the new component. Find:

```ts
import {
  ResilienceService as LibResilienceService,
  ResilienceSnapshotComponent,
} from '@elohim/service/public-api';
```

Replace with:

```ts
import {
  DistributionBadgeComponent,
  ResilienceService as LibResilienceService,
  ResilienceSnapshotComponent,
} from '@elohim/service/public-api';
```

- [ ] **Step 2: Add the component to the standalone imports array**

Find the `imports: [...]` array in the `@Component` decorator (around line 93–106). It currently ends with `ResilienceSnapshotComponent,`. Add the new component **after** ResilienceSnapshotComponent:

```ts
    EprRelationshipsPanelComponent,
    ResilienceSnapshotComponent,
    DistributionBadgeComponent,
  ],
```

- [ ] **Step 3: Render the new badge in the header next to the resilience snapshot**

Open `content-viewer.component.html`. Find the existing block (around line 71–76):

```html
          <elohim-resilience-snapshot
            *ngIf="resilienceSnapshot$ | async as snap"
            [snapshot]="snap"
            density="icon"
            data-testid="viewer-resilience-info"
          ></elohim-resilience-snapshot>
```

Add **immediately after** that closing `</elohim-resilience-snapshot>`:

```html
          @if (node && node.distribution) {
            <elohim-distribution-badge
              [summary]="node.distribution"
              [blobHash]="node.blobCid"
              data-testid="viewer-distribution-info"
            ></elohim-distribution-badge>
          }
```

- [ ] **Step 4: Verify the page renders without template errors**

```bash
cd app/elohim-app
pnpm exec ng build --configuration=development 2>&1 | tail -10
```

Expected: build succeeds.

- [ ] **Step 5: Commit**

```bash
git add app/elohim-app/src/app/lamad/components/content-viewer/
git commit -m "feat(content-viewer): render distribution + resilience side-by-side

Header now carries both <elohim-resilience-snapshot> (existing,
collective-grain, separate fetch) and <elohim-distribution-badge>
(library, replica-grain, inline on EPR head).

Per coherence design — distribution and resilience are orthogonal
readings; the resource page is the moment a user is asking both
questions: where is this and is it safe."
```

### Task 6: Add a content-viewer test asserting both widgets render

**Files:**
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.spec.ts`

- [ ] **Step 1: Read the current spec to find the right place to add the test**

```bash
grep -n "describe\|viewer-resilience-info\|viewer-distribution-info" \
  app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.spec.ts | head -20
```

Find an existing describe block that already exercises the header, or add a new one near the existing tests for `viewer-resilience-info`.

- [ ] **Step 2: Add a focused test**

Add the following describe block to the spec (place it after the existing top-level describe-init but at the same nesting level as other behaviour blocks):

```ts
describe('Header — distribution + resilience side-by-side', () => {
  it('renders <elohim-distribution-badge> when node.distribution is hydrated', async () => {
    // Component already initialised in the file's beforeEach with a node fixture.
    // Replace the node with one that has a hydrated distribution summary.
    component.node = {
      ...component.node!,
      blobCid: 'sha256-content-viewer-test',
      distribution: {
        replicaCount: 3,
        replicaTarget: 4,
        replicaHealth: 'at_risk',
        projectorCount: 1,
        reachClass: 'public',
        diversityHint: { kind: 'region_metro', value: ['us-central'] },
        thisFetchSource: 'projected_via_doorway',
        lastVerifiedSeconds: 30,
      },
    };
    fixture.detectChanges();
    expect(
      fixture.nativeElement.querySelector('[data-testid="viewer-distribution-info"]'),
    ).toBeTruthy();
  });

  it('hides the distribution badge when node.distribution is absent', () => {
    component.node = { ...component.node!, distribution: undefined };
    fixture.detectChanges();
    expect(
      fixture.nativeElement.querySelector('[data-testid="viewer-distribution-info"]'),
    ).toBeFalsy();
  });
});
```

If the existing spec does not pre-instantiate `component`/`fixture`/`node` in a `beforeEach` exactly like this, adjust the fixture setup minimally to do so — do not refactor surrounding tests.

- [ ] **Step 3: Run the spec and verify it passes**

```bash
cd app/elohim-app
pnpm exec vitest run --config vite.config.ts src/app/lamad/components/content-viewer/content-viewer.component.spec.ts
```

Expected: existing tests + 2 new tests pass.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.spec.ts
git commit -m "test(content-viewer): assert distribution badge in header

Two assertions: badge renders when node.distribution is hydrated; hidden
when absent. The viewer-resilience-info testid coverage is unchanged."
```

---

## Phase 3 — Delete the in-app duplicate

### Task 7: Remove the old in-app distribution badge + service

**Files:**
- Delete: `app/elohim-app/src/app/elohim/components/distribution-badge/` (whole directory)
- Delete: `app/elohim-app/src/app/elohim/services/distribution.service.ts`
- Delete: `app/elohim-app/src/app/elohim/services/distribution.service.spec.ts`

- [ ] **Step 1: Confirm no remaining consumers reference the old paths**

```bash
cd app/elohim-app
grep -rln "@app/elohim/components/distribution-badge\|@app/elohim/services/distribution.service\|app-distribution-badge" src/ 2>/dev/null
```

Expected: **no output**. If any consumer remains, swap it to the library import / new selector before deleting.

- [ ] **Step 2: Delete the files**

```bash
git rm -r app/elohim-app/src/app/elohim/components/distribution-badge/
git rm app/elohim-app/src/app/elohim/services/distribution.service.ts
git rm app/elohim-app/src/app/elohim/services/distribution.service.spec.ts
```

- [ ] **Step 3: Run the elohim-app build to confirm nothing breaks**

```bash
pnpm exec ng build --configuration=development 2>&1 | tail -10
```

Expected: build succeeds.

- [ ] **Step 4: Commit**

```bash
git commit -m "refactor: delete in-app distribution-badge + service

Graduated into elohim-library. Concept-card and content-viewer both
import from @elohim/service/public-api now. Per coherence design."
```

---

## Phase 4 — Doc + a2o coherence

### Task 8: Update content-node model doc-comment

**Files:**
- Modify: `app/elohim-app/src/app/lamad/models/content-node.model.ts`

- [ ] **Step 1: Find the doc-comment**

```bash
grep -n "elohim-distribution-badge\|app-distribution-badge\|distribution-badge" \
  app/elohim-app/src/app/lamad/models/content-node.model.ts
```

Expected: a doc-comment around `distribution?: DistributionSummary` referencing the badge widget.

- [ ] **Step 2: Update the comment to reference the library widget**

Replace the line referencing `<app-distribution-badge>` with `<elohim-distribution-badge>`. The body should mention that the widget lives in `@elohim/service/public-api`.

- [ ] **Step 3: Commit**

```bash
git add app/elohim-app/src/app/lamad/models/content-node.model.ts
git commit -m "docs(content-node): point distribution comment at library widget"
```

### Task 9: Update story-harvest scenarios

**Files:**
- Modify: `genesis/a2o/features/resilience/observable-distribution.feature`

- [ ] **Step 1: Find the references**

```bash
grep -n "app-distribution-badge" genesis/a2o/features/resilience/observable-distribution.feature
```

Expected: matches in two scenarios harvested in this branch (`Concept card renders distribution badge when summary is hydrated`, `Concept card hides badge when distribution is not yet known`).

- [ ] **Step 2: Replace the selector strings**

Use sed in-place:

```bash
sed -i 's/app-distribution-badge/elohim-distribution-badge/g' \
  genesis/a2o/features/resilience/observable-distribution.feature
```

- [ ] **Step 3: Verify**

```bash
grep -n "distribution-badge" genesis/a2o/features/resilience/observable-distribution.feature
```

Expected: only `elohim-distribution-badge` references.

- [ ] **Step 4: Commit**

```bash
git add genesis/a2o/features/resilience/observable-distribution.feature
git commit -m "docs(a2o): retag distribution scenarios to elohim-distribution-badge"
```

### Task 10: Open the substrate-sharing backlog stub

**Files:**
- Create: `genesis/docs/superpowers/specs/2026-05-03-distribution-resilience-substrate-sharing-backlog.md`

- [ ] **Step 1: Write the placeholder spec**

```markdown
# Distribution + Resilience Substrate-Sharing — Backlog Stub

**Date:** 2026-05-03
**Status:** Backlog (not scheduled)
**Parent:** `2026-05-03-distribution-resilience-coherence-design.md`

## Why this exists

The two compose pipelines —
`compose_resilience_snapshot` (collective-grain, in
`elohim-storage/src/services/household_resilience.rs`) and
`compose_distribution_summary` (replica-grain, in
`elohim-storage/src/services/distribution_view.rs`) — read from the same
DHT primitives (REA commitments, economic events, custodian state). They
currently run as parallel pipelines.

## Scope

- Identify shared SQL aggregations / index hits / per-CID cached primitives
  across the two compose paths.
- Each pipeline keeps its **public contract** (schema unchanged, route
  unchanged).
- Internals can converge to reduce duplicated work.
- This is **not** a schema-merge effort. The surfaces stay distinct (per
  coherence design).

## Out of scope

- UI changes (handled by the parent coherence design).
- Schema changes to either view type.
- Any change visible to clients.

## Trigger

Open this spec when:

- Performance work shows the two pipelines duplicating expensive joins, OR
- A new dimension (e.g. "carry-credit") composes from the same primitives
  and would benefit from a shared substrate helper.
```

- [ ] **Step 2: Commit**

```bash
git add genesis/docs/superpowers/specs/2026-05-03-distribution-resilience-substrate-sharing-backlog.md
git commit -m "docs(spec): backlog stub for substrate-side compose sharing"
```

---

## Phase 5 — Verify + finish

### Task 11: Run the full library + app test suites for affected areas

**Files:** none (verification only)

- [ ] **Step 1: Library tests**

```bash
cd app/elohim-library/projects/elohim-service
pnpm test -- distribution
```

Expected: 5 tests (1 service + 4 component) all pass.

- [ ] **Step 2: App tests for affected components**

```bash
cd app/elohim-app
pnpm exec vitest run --config vite.config.ts \
  src/app/lamad/components/concept-card/concept-card.component.spec.ts \
  src/app/lamad/components/content-viewer/content-viewer.component.spec.ts
```

Expected: all tests pass (concept-card 6, content-viewer existing+2 new).

- [ ] **Step 3: Build verification**

```bash
cd app/elohim-app
pnpm exec ng build --configuration=development 2>&1 | tail -5
```

Expected: build succeeds, no warnings about missing components.

### Task 12: Lint + format pass

**Files:** none (verification only)

- [ ] **Step 1: Lint elohim-library**

```bash
cd app/elohim-library
pnpm exec eslint projects/elohim-service/src/distribution 2>&1 | tail -20
```

Expected: clean. If errors, run `pnpm exec eslint --fix projects/elohim-service/src/distribution` and address remaining manually.

- [ ] **Step 2: Lint elohim-app touched files**

```bash
cd app/elohim-app
pnpm exec eslint src/app/lamad/components/concept-card src/app/lamad/components/content-viewer src/app/lamad/models/content-node.model.ts 2>&1 | tail -20
```

Expected: clean.

- [ ] **Step 3: Prettier check on touched files**

```bash
pnpm exec prettier --check \
  src/app/lamad/components/concept-card \
  src/app/lamad/components/content-viewer \
  src/app/lamad/models/content-node.model.ts
cd ../elohim-library
pnpm exec prettier --check projects/elohim-service/src/distribution
```

Expected: all matched files use Prettier code style. If issues, run `--write` on the listed paths.

- [ ] **Step 4: Commit any format/lint deltas**

```bash
git add -u
git commit -m "chore(format): prettier+lint sweep on coherence-pass files" || echo "nothing to format"
```

### Task 13: Final integration run + finish-branch handoff

**Files:** none (handoff)

- [ ] **Step 1: Run the previously-passing 689-test sweep one more time**

```bash
cd app/elohim-app
pnpm exec vitest run --config vite.config.ts \
  src/app/elohim/components \
  src/app/lamad/components/concept-card \
  src/app/lamad/components/content-viewer \
  src/app/shefa/services \
  src/app/shefa/pages 2>&1 | tail -10
```

Expected: all tests pass; count should be very close to 689 (small delta from added/moved specs).

- [ ] **Step 2: Verify git log is clean and tells the story**

```bash
git log feature/light-up-topology --oneline -15
```

Expected: the new commits read as a coherent narrative — graduate service, graduate component, export, swap concept-card, side-by-side in viewer, test the swap, delete in-app duplicate, update docs/a2o.

- [ ] **Step 3: Hand off to finishing-a-development-branch**

Invoke the `superpowers:finishing-a-development-branch` skill. The skill verifies tests pass, runs story-harvest if appropriate, and presents the four end-of-branch options.

---

## Self-Review (writing-plans skill)

**Spec coverage check:**

| Spec section | Plan task |
|---|---|
| Two dimensions framing | Implicit in component contracts (Task 2, 5) |
| `<elohim-distribution-badge>` in library | Task 2 |
| `<elohim-resilience-snapshot>` unchanged | (verified by absence of any task touching it) |
| concept-card embed swap | Task 4 |
| content-viewer side-by-side | Task 5, Task 6 |
| Old in-app component deleted | Task 7 |
| `viewer-distribution-info` testid added | Task 5 step 3 |
| Future-moment data hooks (schema reservations) | **GAP** — not scheduled in this plan; called out in spec as schema-only with `additionalProperties: true`. Can be deferred to the sprint that builds the first future moment. |
| Migration plan items | Tasks 1–9 |
| Substrate-sharing backlog stub | Task 10 |
| Lint/format/test passes | Tasks 11–12 |
| Story-harvest scenario re-tag | Task 9 |
| Content-node model doc-comment | Task 8 |

**Future-moment schema reservation gap:** The spec proposes adding optional fields to `distribution-details.schema.json` and `resilience-snapshot-view.schema.json`. Adding them now is low-cost (open-shape `additionalProperties: true` declarations), but each one is dead weight until its consuming sprint exists. **Decision: defer schema reservations to the sprint that first uses them.** The spec already documents the design intent; the schemas can grow at use-time without breaking either widget's contract today. No plan task needed.

**Placeholder scan:** every step has concrete code, exact file paths, and runnable commands. No "TBD" or "similar to Task N".

**Type consistency:** `DistributionSummary`, `DistributionDetails`, `DistributionService.getDetails(blobHash)`, `DistributionBadgeComponent`, `[summary]`, `[blobHash]` — same names across all tasks.
