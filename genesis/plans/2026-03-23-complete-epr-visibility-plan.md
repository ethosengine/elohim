# Complete EPR Visibility Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Render content flags as subtle tags in content headers and display stewardship allocations in the trust tab.

**Architecture:** No new components. Content flags render inline after `#tags` using the same tag pattern. Stewardship loads alongside trust badge data via `StewardshipAllocationService` and renders as a card section in the trust tab.

**Tech Stack:** Angular 19 (standalone components, inject(), Vitest)

---

### Task 1: Content Flag Tags — Test

**Files:**
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.spec.ts`

**Step 1: Write failing tests for flag helper methods and rendering**

Add to the existing describe block, after the existing tests:

```typescript
describe('Content Flags', () => {
  it('should return empty array when node has no flags', () => {
    component.node = { ...mockContentNode, flags: undefined };
    expect(component.getFlags()).toEqual([]);
  });

  it('should return flags when node has flags', () => {
    const flags = [
      { type: 'disputed' as const, reason: 'Factual accuracy questioned', flaggedAt: '2026-03-01' },
    ];
    component.node = { ...mockContentNode, flags };
    expect(component.getFlags()).toEqual(flags);
  });

  it('should return correct flag label', () => {
    expect(component.getFlagLabel('disputed')).toBe('Disputed');
    expect(component.getFlagLabel('outdated')).toBe('Outdated');
    expect(component.getFlagLabel('appeal-pending')).toBe('Appeal Pending');
    expect(component.getFlagLabel('under-review')).toBe('Under Review');
    expect(component.getFlagLabel('partial-revocation')).toBe('Partial Revocation');
  });

  it('should return correct flag CSS class', () => {
    expect(component.getFlagClass('disputed')).toBe('flag-tag flag-disputed');
    expect(component.getFlagClass('outdated')).toBe('flag-tag flag-outdated');
    expect(component.getFlagClass('under-review')).toBe('flag-tag flag-under-review');
  });
});
```

**Step 2: Run tests to verify they fail**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts "content-viewer.component"`
Expected: FAIL — `getFlags`, `getFlagLabel`, `getFlagClass` are not defined

---

### Task 2: Content Flag Tags — Implementation

**Files:**
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts` (after `getResilienceTooltip()` at ~line 898)
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.html` (after tags div at ~line 88)
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.css` (after `.tag` styles at ~line 267)

**Step 1: Add flag helper methods to component.ts**

After the `getResilienceTooltip()` method (~line 898), add:

```typescript
  // =========================================================================
  // Content Flag Helpers
  // =========================================================================

  getFlags(): ContentFlag[] {
    return this.node?.flags || [];
  }

  getFlagLabel(type: string): string {
    const labels: Record<string, string> = {
      'disputed': 'Disputed',
      'outdated': 'Outdated',
      'appeal-pending': 'Appeal Pending',
      'under-review': 'Under Review',
      'partial-revocation': 'Partial Revocation',
    };
    return labels[type] || type;
  }

  getFlagClass(type: string): string {
    return `flag-tag flag-${type}`;
  }
```

Also add the `ContentFlag` import — it's already exported from `content-node.model.ts` at the same path as `ContentNode`:

```typescript
import { ContentNode, ContentFlag } from '../../models/content-node.model';
```

**Step 2: Add flag tags to HTML template**

After the tags div (line 88: `</div>`) and before the content-actions div (line 91), add:

```html
        <div class="flag-tags" *ngIf="getFlags().length > 0">
          <span
            *ngFor="let flag of getFlags()"
            [class]="getFlagClass(flag.type)"
            [title]="flag.reason"
            (click)="setActiveTab('trust')"
            role="button"
            [attr.aria-label]="getFlagLabel(flag.type) + ': ' + flag.reason"
            data-testid="viewer-content-flag"
          >{{ getFlagLabel(flag.type) }}</span>
        </div>
```

**Step 3: Add flag tag CSS**

After the `.tag` styles (~line 267), add:

```css
/* Content flag tags — only visible when flags exist */
.flag-tags {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
  margin-top: 0.75rem;
}

.flag-tag {
  padding: 0.3rem 0.7rem;
  border-radius: 6px;
  font-size: 0.8rem;
  font-weight: 500;
  cursor: pointer;
  transition: opacity 0.2s;
  opacity: 0.85;
}

.flag-tag:hover {
  opacity: 1;
}

.flag-disputed {
  background: rgb(239 68 68 / 8%);
  color: #dc2626;
  border: 1px solid rgb(239 68 68 / 25%);
}

.flag-outdated {
  background: rgb(245 158 11 / 8%);
  color: #d97706;
  border: 1px solid rgb(245 158 11 / 25%);
}

.flag-appeal-pending {
  background: rgb(139 92 246 / 8%);
  color: #7c3aed;
  border: 1px solid rgb(139 92 246 / 25%);
}

.flag-under-review {
  background: rgb(59 130 246 / 8%);
  color: #2563eb;
  border: 1px solid rgb(59 130 246 / 25%);
}

.flag-partial-revocation {
  background: rgb(239 68 68 / 8%);
  color: #dc2626;
  border: 1px solid rgb(239 68 68 / 25%);
}
```

**Step 4: Run tests to verify they pass**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts "content-viewer.component"`
Expected: All content flag tests PASS

**Step 5: Commit**

```bash
git add app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.*
git commit -m "feat(lamad): add content flag tags to content viewer header"
```

---

### Task 3: Content Flag Tags in Lesson View

**Files:**
- Modify: `app/elohim-app/src/app/lamad/components/lesson-view/lesson-view.component.ts`

**Step 1: Add flag helper methods to lesson-view component**

After the `getResilienceTooltip()` method (~line 729), add the same three helpers:

```typescript
  /** Content flags as subtle tags. */
  getFlags(): { type: string; reason: string; flaggedAt: string }[] {
    return this.content?.flags || [];
  }

  getFlagLabel(type: string): string {
    const labels: Record<string, string> = {
      'disputed': 'Disputed',
      'outdated': 'Outdated',
      'appeal-pending': 'Appeal Pending',
      'under-review': 'Under Review',
      'partial-revocation': 'Partial Revocation',
    };
    return labels[type] || type;
  }

  getFlagClass(type: string): string {
    return `flag-tag flag-${type}`;
  }
```

**Step 2: Add flag tags to inline template**

After the `</div>` closing the `content-tags` block (~line 100) and before the `</div>` closing `header-meta` (~line 101), add:

```html
            @if (getFlags().length) {
              <div class="flag-tags">
                @for (flag of getFlags(); track flag.type) {
                  <span
                    [class]="getFlagClass(flag.type)"
                    [title]="flag.reason"
                    data-testid="lesson-content-flag"
                  >{{ getFlagLabel(flag.type) }}</span>
                }
              </div>
            }
```

**Step 3: Add flag tag styles to inline styles**

In the `styles` array of the component decorator, add after `.reach-badge:hover, .resilience-info:hover { opacity: 1; }`:

```css
      .flag-tags {
        display: flex;
        gap: 0.4rem;
        flex-wrap: wrap;
        margin-top: 0.5rem;
      }
      .flag-tag {
        padding: 0.2rem 0.6rem;
        border-radius: 6px;
        font-size: 0.75rem;
        font-weight: 500;
        cursor: help;
        opacity: 0.85;
      }
      .flag-tag:hover { opacity: 1; }
      .flag-disputed { background: rgb(239 68 68 / 8%); color: #dc2626; border: 1px solid rgb(239 68 68 / 25%); }
      .flag-outdated { background: rgb(245 158 11 / 8%); color: #d97706; border: 1px solid rgb(245 158 11 / 25%); }
      .flag-appeal-pending { background: rgb(139 92 246 / 8%); color: #7c3aed; border: 1px solid rgb(139 92 246 / 25%); }
      .flag-under-review { background: rgb(59 130 246 / 8%); color: #2563eb; border: 1px solid rgb(59 130 246 / 25%); }
      .flag-partial-revocation { background: rgb(239 68 68 / 8%); color: #dc2626; border: 1px solid rgb(239 68 68 / 25%); }
```

**Step 4: Run tests**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts "lesson-view"`
Expected: Existing tests still PASS

**Step 5: Commit**

```bash
git add app/elohim-app/src/app/lamad/components/lesson-view/lesson-view.component.ts
git commit -m "feat(lamad): add content flag tags to lesson view header"
```

---

### Task 4: Stewardship in Trust Tab — Test

**Files:**
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.spec.ts`

**Step 1: Add StewardshipAllocationService mock to test setup**

Import at top of spec file:
```typescript
import { StewardshipAllocationService } from '../../services/stewardship-allocation.service';
```

Add mock in `beforeEach`:
```typescript
    const stewardshipSpyObj = {
      getContentStewardship: vi.fn().mockReturnValue(of({
        contentId: 'test-content-1',
        allocations: [],
        totalAllocation: 0,
        hasDisputes: false,
        primarySteward: null,
      })),
    };
```

Add to providers:
```typescript
        { provide: StewardshipAllocationService, useValue: stewardshipSpyObj },
```

Add variable:
```typescript
  let stewardshipServiceSpy: any;
```

And assignment after `TestBed.inject`:
```typescript
    stewardshipServiceSpy = TestBed.inject(StewardshipAllocationService);
```

**Step 2: Write failing tests for stewardship loading**

```typescript
describe('Stewardship in Trust Tab', () => {
  it('should load stewardship data when content loads', fakeAsync(() => {
    fixture.detectChanges();
    tick();
    expect(stewardshipServiceSpy.getContentStewardship).toHaveBeenCalledWith('test-content-1');
  }));

  it('should store stewardship data on component', fakeAsync(() => {
    const mockStewardship = {
      contentId: 'test-content-1',
      allocations: [{
        steward: { id: 's1', displayName: 'Alice', presenceState: 'active' },
        id: 'alloc-1',
        appId: 'test',
        contentId: 'test-content-1',
        stewardPresenceId: 'sp-1',
        allocationRatio: 0.6,
        allocationMethod: 'manual',
        contributionType: 'author',
        contributionEvidence: null,
        governanceState: 'active',
        disputeId: null,
        disputeReason: null,
        disputedAt: null,
        disputedBy: null,
        negotiationSessionId: null,
        elohimRatifiedAt: null,
        elohimRatifierId: null,
        effectiveFrom: '2026-01-01',
        effectiveUntil: null,
        supersededBy: null,
        recognitionAccumulated: 42.5,
        lastRecognitionAt: '2026-03-20',
        note: null,
        metadata: null,
        createdAt: '2026-01-01',
        updatedAt: '2026-03-20',
        dhtAnchorHash: null,
      }],
      totalAllocation: 0.6,
      hasDisputes: false,
      primarySteward: null,
    };
    stewardshipServiceSpy.getContentStewardship.mockReturnValue(of(mockStewardship));
    fixture.detectChanges();
    tick();
    expect(component.stewardship).toEqual(mockStewardship);
  }));

  it('should handle stewardship load error gracefully', fakeAsync(() => {
    stewardshipServiceSpy.getContentStewardship.mockReturnValue(of({
      contentId: 'test-content-1',
      allocations: [],
      totalAllocation: 0,
      hasDisputes: false,
      primarySteward: null,
    }));
    fixture.detectChanges();
    tick();
    expect(component.stewardship).toBeTruthy();
    expect(component.stewardship!.allocations).toEqual([]);
  }));
});
```

**Step 3: Run tests to verify they fail**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts "content-viewer.component"`
Expected: FAIL — `stewardship` property doesn't exist on component

---

### Task 5: Stewardship in Trust Tab — Implementation

**Files:**
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts`
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.html`
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.css`

**Step 1: Add stewardship state and service injection**

In the imports section of the TS file, add:
```typescript
import { StewardshipAllocationService } from '../../services/stewardship-allocation.service';
```

Add the import for the generated type:
```typescript
import type { ContentStewardshipView } from '@elohim/storage-client/generated';
```

Add property after `isLoadingTrust` (~line 92):
```typescript
  // Stewardship data
  stewardship: ContentStewardshipView | null = null;
```

Add injection after the existing injects (~line 149):
```typescript
  private readonly stewardshipService = inject(StewardshipAllocationService);
```

**Step 2: Add stewardship loading method**

After the `loadTrustBadge` method (~line 403), add:

```typescript
  /**
   * Load stewardship allocation data for the trust tab.
   */
  private loadStewardship(nodeId: string): void {
    this.stewardshipService
      .getContentStewardship(nodeId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: stewardship => {
          this.stewardship = stewardship;
        },
        error: () => {
          // Stewardship is supplemental — don't block on failure
        },
      });
  }
```

**Step 3: Call loadStewardship alongside loadTrustBadge**

Find where `loadTrustBadge(nodeId)` is called (~line 341) and add after it:

```typescript
          this.loadStewardship(nodeId);
```

**Step 4: Add stewardship section to trust tab HTML**

After the reach level div (line 406 `</div>` closing `trust-reach`), before the warnings section (line 408), add:

```html

          <!-- Stewardship -->
          <div class="stewardship-section" *ngIf="stewardship">
            <h3>Stewardship</h3>
            <div *ngIf="stewardship.allocations.length === 0" class="stewardship-empty">
              No stewardship allocations yet.
            </div>
            <div *ngIf="stewardship.allocations.length > 0" class="stewardship-list">
              <div
                *ngFor="let alloc of stewardship.allocations"
                class="stewardship-card"
                [class.stewardship-disputed]="alloc.governanceState === 'disputed'"
                data-testid="trust-stewardship-card"
              >
                <div class="steward-identity">
                  <span class="steward-name">{{ alloc.steward?.displayName || alloc.stewardPresenceId }}</span>
                  <span class="steward-role">{{ alloc.contributionType }}</span>
                </div>
                <div class="steward-metrics">
                  <div class="allocation-bar">
                    <div
                      class="allocation-fill"
                      [style.width.%]="alloc.allocationRatio * 100"
                    ></div>
                  </div>
                  <span class="allocation-pct">{{ (alloc.allocationRatio * 100) | number:'1.0-0' }}%</span>
                </div>
                <div class="steward-recognition" *ngIf="alloc.recognitionAccumulated > 0">
                  {{ alloc.recognitionAccumulated | number:'1.1-1' }} recognition
                </div>
                <div class="steward-dispute" *ngIf="alloc.governanceState === 'disputed'">
                  Disputed{{ alloc.disputeReason ? ': ' + alloc.disputeReason : '' }}
                </div>
              </div>
            </div>
          </div>
```

**Step 5: Add stewardship CSS**

After the trust tab styles section, add:

```css
/* Stewardship Section */

.stewardship-section {
  margin-bottom: 1rem;
}

.stewardship-section h3 {
  font-size: 1.1rem;
  color: var(--lamad-text-primary);
  margin-bottom: 1rem;
}

.stewardship-empty {
  color: var(--lamad-text-muted);
  font-size: 0.9rem;
  font-style: italic;
}

.stewardship-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.stewardship-card {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  background: var(--lamad-surface);
  border: 1px solid var(--lamad-border);
  border-radius: 8px;
  padding: 1rem;
  border-left: 3px solid #10b981;
}

.stewardship-card.stewardship-disputed {
  border-left-color: #ef4444;
}

.steward-identity {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.steward-name {
  font-weight: 600;
  color: var(--lamad-text-primary);
}

.steward-role {
  font-size: 0.8rem;
  padding: 0.15rem 0.5rem;
  border-radius: 4px;
  background: rgb(99 102 241 / 10%);
  color: var(--lamad-accent-primary);
  text-transform: capitalize;
}

.steward-metrics {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.allocation-bar {
  flex: 1;
  height: 6px;
  background: var(--lamad-bg-tertiary, #f3f4f6);
  border-radius: 3px;
  overflow: hidden;
}

.allocation-fill {
  height: 100%;
  background: #10b981;
  border-radius: 3px;
  transition: width 0.3s;
}

.allocation-pct {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--lamad-text-secondary);
  min-width: 3ch;
  text-align: right;
}

.steward-recognition {
  font-size: 0.85rem;
  color: var(--lamad-text-secondary);
}

.steward-dispute {
  font-size: 0.85rem;
  color: #ef4444;
  font-style: italic;
}
```

**Step 6: Run tests to verify they pass**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts "content-viewer.component"`
Expected: All tests PASS including stewardship tests

**Step 7: Commit**

```bash
git add app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.*
git commit -m "feat(lamad): add stewardship section to trust tab"
```

---

### Task 6: Final Verification

**Step 1: Run full lamad test suite**

Run: `cd /projects/elohim && pnpm exec vitest run --config app/elohim-app/vite.config.ts`
Expected: No regressions

**Step 2: Run lint**

Run: `cd /projects/elohim/app/elohim-app && pnpm run lint`
Expected: No new errors

**Step 3: Update CLAUDE-PICKS.md**

Move "Stewardship display in trust tab" and "Content flags display" from Small Gaps to Completed section with date.

**Step 4: Commit**

```bash
git add CLAUDE-PICKS.md
git commit -m "docs: mark content flags and stewardship display as complete"
```
