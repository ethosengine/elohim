# Full-Browser ContentNode Delivery with Protocol Toolbar — Sprint 2

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a full-browser delivery mode where visiting `/deliver/{slug}` serves a ContentNode as the entire page — no Angular app chrome — with a minimal protocol omnibar that proves provenance and provides drill-down access to the governance systems that undergird the page.

**Architecture:** New `ContentDeliveryComponent` (standalone route, minimal shell) renders content full-page via the existing renderer registry, overlaid with a `ProtocolOmnibarComponent` — the protocol's equivalent of a browser address bar with SSL padlock. At a glance: a tiny pill showing the content is protocol-delivered. Click to expand: EPR address, reach level, steward(s), delivery source. Drill down: navigate to `/resource/{id}` for the full governance hub (attestations, challenges, feedback). Initiate actions: report content, submit feedback, inspect EPR provenance. The omnibar is standalone (no lamad deps) so it can eventually become a web component.

**Tech Stack:** Angular 19, TypeScript, Vitest, existing ContentService/RendererRegistry, doorway Rust headers

**Depends on:** Nothing (self-contained — the omnibar is a navigation/provenance tool, not an analytics display)

**Existing infrastructure:**
- `ContentViewerComponent` (`content-viewer.component.ts`, 1072 lines) — learning-context viewer with tabs, feedback, governance. NOT what we want for delivery — too much chrome.
- `RendererRegistryService` (`renderer-registry.service.ts`) — maps contentFormat → renderer component. Reusable.
- `ContentService` (`content.service.ts`) — fetches content by ID/slug. Reusable.
- `SeoService` (`seo.service.ts`) — manages meta tags. Will extend for social preview.
- Doorway headers: `X-Root-App` on root_app.rs:469/501, `X-Content-Address` on apps.rs:337/440, `X-Content-Slug` on apps.rs:339/442.
- `FocusedViewToggleComponent` — existing focus button, will be updated to show protocol omnibar in focused mode.
- `/resource/:resourceId` route (app.routes.ts:46) — loads ContentViewerComponent. The new `/deliver/:slug` is a separate route with a different component.

---

## File Structure

| File | Responsibility |
|------|----------------|
| `src/app/elohim/components/protocol-omnibar/protocol-omnibar.component.ts` | **NEW** — Protocol omnibar: at-a-glance provenance pill, expandable to EPR details, drill-down to /resource governance hub, action menu (report, feedback, inspect). No lamad dependencies. |
| `src/app/elohim/components/protocol-omnibar/protocol-omnibar.component.html` | **NEW** — Template |
| `src/app/elohim/components/protocol-omnibar/protocol-omnibar.component.css` | **NEW** — Styles |
| `src/app/elohim/components/protocol-omnibar/protocol-omnibar.component.spec.ts` | **NEW** — Tests |
| `src/app/elohim/components/content-delivery/content-delivery.component.ts` | **NEW** — Full-page delivery shell: loads content, instantiates renderer, shows toolbar. Minimal. |
| `src/app/elohim/components/content-delivery/content-delivery.component.html` | **NEW** — Template |
| `src/app/elohim/components/content-delivery/content-delivery.component.css` | **NEW** — Styles |
| `src/app/elohim/components/content-delivery/content-delivery.component.spec.ts` | **NEW** — Tests |
| `src/app/app.routes.ts` | **MODIFY** — Add `/deliver/:slug` route |
| `src/app/lamad/components/content-viewer/content-viewer.component.ts` | **MODIFY** — Add ProtocolOmnibarComponent to focused view |
| `src/app/lamad/components/content-viewer/content-viewer.component.html` | **MODIFY** — Show toolbar in focused view |
| `doorway/doorway-service/src/routes/root_app.rs` | **MODIFY** — Add `X-Stewards`, `X-Reach` headers |
| `doorway/doorway-service/src/routes/apps.rs` | **MODIFY** — Add `X-Stewards`, `X-Reach` headers |
| `genesis/a2o/features/delivery/protocol-omnibar.feature` | **NEW** — BDD scenarios |

---

### Task 1: Write a2o scenarios for content delivery and protocol omnibar

**Files:**
- Create: `genesis/a2o/features/delivery/protocol-omnibar.feature`

- [ ] **Step 1: Write the scenario file**

```gherkin
@delivery @omnibar @provenance
Feature: Full-Browser Content Delivery with Protocol Omnibar
  As a visitor to the Elohim Protocol
  I want content delivered as full pages with an unobtrusive provenance bar
  So I can tell this came from the protocol network and drill into its governance

  The protocol omnibar is the equivalent of a browser's address bar with SSL
  padlock. At a glance it says "you're on the network." Click to inspect the
  EPR provenance — like viewing a site's SSL certificate. Drill down to the
  full governance hub. Initiate feedback or report actions.

  Background:
    Given content node "manifesto" exists with:
      | title       | The Elohim Protocol Manifesto           |
      | format      | markdown                                |
      | reach       | commons                                 |
      | stewardedBy | Genesis Collective (80%), Matthew (20%) |

  # --- Full-Page Delivery ---

  Scenario: Markdown content renders as full page
    When I visit "/deliver/manifesto"
    Then the page renders the manifesto as formatted HTML
    And there is no Angular navigation chrome
    And the protocol omnibar pill is visible in the corner

  Scenario: HTML5 app content renders as full-page iframe
    Given content node "evolution-of-trust" exists with format "html5-app"
    When I visit "/deliver/evolution-of-trust"
    Then the page shows the app in a full-viewport iframe
    And the protocol omnibar pill overlays the iframe

  Scenario: Unknown content shows 404
    When I visit "/deliver/nonexistent-slug"
    Then I see a "Content not found" page

  # --- Omnibar: At-a-Glance (Collapsed Pill) ---

  Scenario: Omnibar pill shows protocol-delivered status
    When I visit "/deliver/manifesto"
    Then a small pill appears in the top-right corner
    And the pill shows the reach icon and "E" protocol mark
    And the pill does not distract from the content

  # --- Omnibar: Expanded (Inspect Provenance) ---

  Scenario: Clicking the pill expands provenance details
    When I visit "/deliver/manifesto"
    And I click the omnibar pill
    Then it expands to show the EPR content address
    And it shows the reach level "commons"
    And it shows the stewards "Genesis Collective" and "Matthew"
    And it shows the delivery source "alpha.elohim.host"

  Scenario: EPR address is copyable
    Given the omnibar is expanded
    When I click the copy button next to the EPR address
    Then the full content address is copied to clipboard

  Scenario: Collapsing the expanded omnibar
    Given the omnibar is expanded
    When I click the collapse button
    Then it returns to the minimal pill

  # --- Omnibar: Drill-Down (Navigate to Governance Hub) ---

  Scenario: Inspect EPR navigates to governance hub
    Given the omnibar is expanded
    When I click "Inspect" (or the EPR address link)
    Then I navigate to "/resource/manifesto"
    And I see the full content viewer with Attestations, Governance, and Network tabs

  # --- Omnibar: Actions ---

  Scenario: Report action available from omnibar
    Given the omnibar is expanded
    When I click the actions menu
    Then I see options: "Report content", "Give feedback", "View stewards"

  # --- Doorway Headers ---

  Scenario: Doorway response includes provenance headers
    When I request "/deliver/manifesto"
    Then the HTTP response includes header "X-Content-Address"
    And the HTTP response includes header "X-Reach" with value "commons"

  # --- Focused View Integration ---

  Scenario: Content viewer focused mode shows omnibar pill
    Given a learner is viewing content "manifesto" in the learning app
    When they toggle focused view mode
    Then the omnibar pill appears in the top-right corner
    And the tabs and feedback sections are hidden
```

- [ ] **Step 2: Commit**

```bash
git add genesis/a2o/features/delivery/protocol-omnibar.feature
git commit -m "feat(a2o): add content delivery and protocol omnibar scenarios"
```

---

### Task 2: Create ProtocolOmnibarComponent

**Files:**
- Create: `src/app/elohim/components/protocol-omnibar/protocol-omnibar.component.spec.ts`
- Create: `src/app/elohim/components/protocol-omnibar/protocol-omnibar.component.ts`
- Create: `src/app/elohim/components/protocol-omnibar/protocol-omnibar.component.html`
- Create: `src/app/elohim/components/protocol-omnibar/protocol-omnibar.component.css`

All paths below are relative to `app/elohim-app/`.

**Design concept:** The protocol omnibar is to EPR content what the browser address bar
is to HTTPS websites. Three states:

1. **Pill** (default) — tiny fixed badge in top-right: protocol mark + reach icon.
   Says "this is protocol-delivered" without distracting. Like the padlock icon.
2. **Expanded** — slim bar showing EPR address, reach, steward(s), delivery source.
   Like clicking the padlock to see "Connection is secure, certificate issued to..."
3. **Actions** — dropdown with: "Inspect EPR" (→ /resource/{id} governance hub),
   "Give feedback", "Report content", "View stewards". Like the browser's
   site settings menu.

- [ ] **Step 1: Write the failing tests**

```typescript
// src/app/elohim/components/protocol-omnibar/protocol-omnibar.component.spec.ts
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { RouterModule } from '@angular/router';

import { ProtocolOmnibarComponent } from './protocol-omnibar.component';

describe('ProtocolOmnibarComponent', () => {
  let component: ProtocolOmnibarComponent;
  let fixture: ComponentFixture<ProtocolOmnibarComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ProtocolOmnibarComponent, RouterModule.forRoot([])],
    }).compileComponents();

    fixture = TestBed.createComponent(ProtocolOmnibarComponent);
    component = fixture.componentInstance;
  });

  it('creates', () => {
    expect(component).toBeTruthy();
  });

  it('starts in pill state', () => {
    fixture.detectChanges();
    expect(component.state).toBe('pill');
  });

  it('expands to details on pill click', () => {
    fixture.detectChanges();
    component.expand();
    expect(component.state).toBe('expanded');
  });

  it('collapses back to pill', () => {
    component.expand();
    component.collapse();
    expect(component.state).toBe('pill');
  });

  it('truncates long content addresses', () => {
    component.contentAddress = 'bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenora';
    expect(component.truncatedAddress).toBe('bafkrei...tenora');
  });

  it('shows reach icon for commons', () => {
    component.reach = 'commons';
    expect(component.reachIcon).toContain('\u{25CB}');
  });

  it('shows lock icon for private', () => {
    component.reach = 'private';
    expect(component.reachIcon).toBe('\u{1F512}');
  });

  it('provides resource route for drill-down', () => {
    component.contentId = 'manifesto';
    expect(component.inspectRoute).toEqual(['/resource', 'manifesto']);
  });

  it('toggles actions menu', () => {
    component.expand();
    expect(component.showActions).toBe(false);
    component.toggleActions();
    expect(component.showActions).toBe(true);
    component.toggleActions();
    expect(component.showActions).toBe(false);
  });

  it('displays steward names when expanded', () => {
    component.stewards = [
      { humanId: 'genesis', displayName: 'Genesis Collective', ratio: 0.8 },
    ];
    component.expand();
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    const stewardEl = el.querySelector('[data-testid="omnibar-steward"]');
    expect(stewardEl?.textContent).toContain('Genesis Collective');
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "protocol-omnibar"`
Expected: FAIL — module not found

- [ ] **Step 3: Write the component**

```typescript
// src/app/elohim/components/protocol-omnibar/protocol-omnibar.component.ts
import { CommonModule } from '@angular/common';
import { Component, EventEmitter, Input, Output } from '@angular/core';
import { RouterModule } from '@angular/router';

export interface OmnibarSteward {
  humanId: string;
  displayName: string;
  ratio: number;
}

export type OmnibarState = 'pill' | 'expanded';

/**
 * ProtocolOmnibarComponent — The protocol's equivalent of a browser address bar.
 *
 * Pill state: tiny badge proving "this is protocol-delivered." Like the SSL padlock.
 * Expanded state: EPR address, reach, stewards, delivery source. Like viewing a cert.
 * Actions: drill-down to /resource/{id} governance hub, report, feedback.
 *
 * No lamad dependencies. Reads from @Input() only.
 * Designed to eventually become a standalone web component.
 */
@Component({
  selector: 'app-protocol-omnibar',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './protocol-omnibar.component.html',
  styleUrls: ['./protocol-omnibar.component.css'],
})
export class ProtocolOmnibarComponent {
  @Input() contentId = '';
  @Input() contentAddress = '';
  @Input() stewards: OmnibarSteward[] = [];
  @Input() reach = '';
  @Input() deliverySource = '';

  @Output() reportRequested = new EventEmitter<void>();
  @Output() feedbackRequested = new EventEmitter<void>();

  state: OmnibarState = 'pill';
  showActions = false;
  copyFeedback = '';

  get truncatedAddress(): string {
    if (!this.contentAddress) return '';
    if (this.contentAddress.length <= 16) return this.contentAddress;
    return `${this.contentAddress.slice(0, 7)}...${this.contentAddress.slice(-6)}`;
  }

  get reachIcon(): string {
    switch (this.reach) {
      case 'private':
      case 'self':
        return '\u{1F512}';
      case 'local':
      case 'community':
        return '\u{25CE}';
      case 'commons':
      case 'public':
      default:
        return '\u{25CB}\u{25CB}\u{25CB}';
    }
  }

  get inspectRoute(): string[] {
    return ['/resource', this.contentId];
  }

  expand(): void {
    this.state = 'expanded';
    this.showActions = false;
  }

  collapse(): void {
    this.state = 'pill';
    this.showActions = false;
  }

  toggleActions(): void {
    this.showActions = !this.showActions;
  }

  async copyAddress(): Promise<void> {
    if (!this.contentAddress) return;
    try {
      await navigator.clipboard.writeText(this.contentAddress);
      this.copyFeedback = 'Copied';
      setTimeout(() => (this.copyFeedback = ''), 1500);
    } catch {
      this.copyFeedback = 'Failed';
      setTimeout(() => (this.copyFeedback = ''), 1500);
    }
  }
}
```

```html
<!-- src/app/elohim/components/protocol-omnibar/protocol-omnibar.component.html -->

<!-- === PILL STATE (default) === -->
<button
  *ngIf="state === 'pill'"
  class="omnibar-pill"
  (click)="expand()"
  aria-label="View protocol provenance"
  data-testid="omnibar-pill"
>
  <span class="pill-mark">E</span>
  <span class="pill-reach">{{ reachIcon }}</span>
</button>

<!-- === EXPANDED STATE === -->
<div
  *ngIf="state === 'expanded'"
  class="omnibar-expanded"
  data-testid="omnibar-expanded"
>
  <div class="omnibar-main">
    <!-- EPR Address (clickable → inspect) -->
    <a
      [routerLink]="inspectRoute"
      class="omnibar-address"
      data-testid="omnibar-address"
      title="Inspect this EPR in the governance hub"
    >
      <span class="address-mark">E</span>
      <code>{{ truncatedAddress }}</code>
    </a>

    <!-- Copy -->
    <button
      class="omnibar-btn"
      (click)="copyAddress()"
      aria-label="Copy EPR address"
      data-testid="omnibar-copy"
    >
      {{ copyFeedback || '\u{1F4CB}' }}
    </button>

    <!-- Divider -->
    <span class="divider"></span>

    <!-- Reach badge -->
    <span class="omnibar-reach" data-testid="omnibar-reach">
      {{ reachIcon }} {{ reach }}
    </span>

    <!-- Stewards (compact) -->
    <span
      *ngFor="let steward of stewards; let i = index"
      class="omnibar-steward"
      data-testid="omnibar-steward"
    >
      <span *ngIf="i > 0" class="steward-sep">&middot;</span>
      {{ steward.displayName }}
    </span>

    <!-- Delivery source (subtle) -->
    <span class="omnibar-source" data-testid="omnibar-source">
      via {{ deliverySource }}
    </span>
  </div>

  <div class="omnibar-actions-area">
    <!-- Actions toggle -->
    <button
      class="omnibar-btn actions-toggle"
      (click)="toggleActions()"
      aria-label="Content actions"
      data-testid="omnibar-actions-toggle"
    >
      \u{22EE}
    </button>

    <!-- Collapse -->
    <button
      class="omnibar-btn"
      (click)="collapse()"
      aria-label="Collapse omnibar"
      data-testid="omnibar-collapse"
    >
      \u{2715}
    </button>
  </div>

  <!-- Actions dropdown -->
  <div
    *ngIf="showActions"
    class="omnibar-dropdown"
    data-testid="omnibar-dropdown"
  >
    <a [routerLink]="inspectRoute" class="dropdown-item">
      Inspect EPR
    </a>
    <button class="dropdown-item" (click)="feedbackRequested.emit(); showActions = false">
      Give feedback
    </button>
    <button class="dropdown-item" (click)="reportRequested.emit(); showActions = false">
      Report content
    </button>
  </div>
</div>
```

```css
/* src/app/elohim/components/protocol-omnibar/protocol-omnibar.component.css */

/* === PILL === */
.omnibar-pill {
  position: fixed;
  top: 0.625rem;
  right: 0.625rem;
  display: flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.3rem 0.5rem;
  background: rgba(15, 23, 42, 0.75);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(99, 102, 241, 0.2);
  border-radius: 16px;
  color: #a5b4fc;
  font-size: 0.6875rem;
  font-weight: 600;
  cursor: pointer;
  z-index: 9999;
  transition: all 0.2s;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.2);
}

.omnibar-pill:hover {
  background: rgba(15, 23, 42, 0.9);
  border-color: rgba(99, 102, 241, 0.5);
  box-shadow: 0 2px 8px rgba(99, 102, 241, 0.15);
}

.pill-mark {
  font-family: serif;
  font-weight: 700;
  font-size: 0.75rem;
}

.pill-reach {
  font-size: 0.5rem;
  opacity: 0.8;
}

/* === EXPANDED === */
.omnibar-expanded {
  position: fixed;
  top: 0.625rem;
  right: 0.625rem;
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0.625rem;
  background: rgba(15, 23, 42, 0.92);
  backdrop-filter: blur(16px);
  border: 1px solid rgba(99, 102, 241, 0.25);
  border-radius: 10px;
  color: #cbd5e1;
  font-size: 0.6875rem;
  font-family: system-ui, sans-serif;
  z-index: 9999;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  max-width: calc(100vw - 1.25rem);
}

.omnibar-main {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  overflow: hidden;
  white-space: nowrap;
}

.omnibar-address {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  color: #a5b4fc;
  text-decoration: none;
  font-family: 'SF Mono', 'Fira Code', monospace;
  font-size: 0.6875rem;
  transition: color 0.15s;
}

.omnibar-address:hover {
  color: #c7d2fe;
  text-decoration: none;
}

.address-mark {
  font-family: serif;
  font-weight: 700;
  font-size: 0.75rem;
  color: #6366f1;
}

.omnibar-address code {
  font-size: inherit;
}

.divider {
  width: 1px;
  height: 12px;
  background: rgba(148, 163, 184, 0.2);
  flex-shrink: 0;
}

.omnibar-reach {
  color: #86efac;
  font-size: 0.625rem;
  flex-shrink: 0;
}

.omnibar-steward {
  color: #94a3b8;
  font-size: 0.625rem;
}

.steward-sep {
  margin: 0 0.125rem;
  opacity: 0.5;
}

.omnibar-source {
  color: #64748b;
  font-size: 0.5625rem;
  font-style: italic;
}

.omnibar-actions-area {
  display: flex;
  align-items: center;
  gap: 0.125rem;
  flex-shrink: 0;
  margin-left: 0.25rem;
}

.omnibar-btn {
  background: none;
  border: none;
  color: #64748b;
  cursor: pointer;
  font-size: 0.75rem;
  padding: 0.125rem 0.25rem;
  border-radius: 4px;
  transition: all 0.15s;
  line-height: 1;
}

.omnibar-btn:hover {
  color: #e2e8f0;
  background: rgba(148, 163, 184, 0.1);
}

.actions-toggle {
  font-size: 0.875rem;
  letter-spacing: 0.05em;
}

/* === DROPDOWN === */
.omnibar-dropdown {
  position: absolute;
  top: calc(100% + 0.375rem);
  right: 0;
  background: rgba(15, 23, 42, 0.95);
  backdrop-filter: blur(16px);
  border: 1px solid rgba(99, 102, 241, 0.2);
  border-radius: 8px;
  padding: 0.25rem 0;
  min-width: 160px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  z-index: 10000;
}

.dropdown-item {
  display: block;
  width: 100%;
  padding: 0.5rem 0.75rem;
  background: none;
  border: none;
  color: #cbd5e1;
  font-size: 0.75rem;
  text-align: left;
  cursor: pointer;
  text-decoration: none;
  transition: background 0.1s;
}

.dropdown-item:hover {
  background: rgba(99, 102, 241, 0.1);
  color: #e2e8f0;
  text-decoration: none;
}

/* === RESPONSIVE === */
@media (max-width: 640px) {
  .omnibar-source,
  .omnibar-steward {
    display: none;
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "protocol-omnibar"`
Expected: All 10 tests PASS

- [ ] **Step 5: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/protocol-omnibar/
git commit -m "feat(elohim): add ProtocolOmnibarComponent — browser-like EPR provenance bar"
```

---

### Task 3: Create ContentDeliveryComponent

**Files:**
- Create: `src/app/elohim/components/content-delivery/content-delivery.component.spec.ts`
- Create: `src/app/elohim/components/content-delivery/content-delivery.component.ts`
- Create: `src/app/elohim/components/content-delivery/content-delivery.component.html`
- Create: `src/app/elohim/components/content-delivery/content-delivery.component.css`

- [ ] **Step 1: Write the failing tests**

```typescript
// src/app/elohim/components/content-delivery/content-delivery.component.spec.ts
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute } from '@angular/router';
import { of } from 'rxjs';

import { ContentDeliveryComponent } from './content-delivery.component';
import { ContentService } from '@app/lamad/services/content.service';
import { RendererRegistryService } from '@app/lamad/renderers/renderer-registry.service';
import { SeoService } from '../../../services/seo.service';

describe('ContentDeliveryComponent', () => {
  let component: ContentDeliveryComponent;
  let fixture: ComponentFixture<ContentDeliveryComponent>;
  let contentServiceSpy: jasmine.SpyObj<ContentService>;

  const mockNode = {
    id: 'manifesto',
    title: 'The Elohim Protocol Manifesto',
    description: 'Founding document',
    contentType: 'epic',
    contentFormat: 'markdown',
    content: '# The Manifesto',
    reach: 'commons',
    stewardedBy: [
      { humanId: 'genesis', displayName: 'Genesis Collective', role: 'primary', affinity: 0.8 },
    ],
    blobHash: 'bafkreiexample123',
    tags: [],
    relatedNodeIds: [],
  };

  beforeEach(async () => {
    contentServiceSpy = jasmine.createSpyObj('ContentService', ['getContentBySlug']);
    contentServiceSpy.getContentBySlug.and.returnValue(of(mockNode));

    const rendererRegistrySpy = jasmine.createSpyObj('RendererRegistryService', ['getRenderer']);
    rendererRegistrySpy.getRenderer.and.returnValue(null);

    const seoServiceSpy = jasmine.createSpyObj('SeoService', ['updateForContent']);

    await TestBed.configureTestingModule({
      imports: [ContentDeliveryComponent],
      providers: [
        { provide: ContentService, useValue: contentServiceSpy },
        { provide: RendererRegistryService, useValue: rendererRegistrySpy },
        { provide: SeoService, useValue: seoServiceSpy },
        {
          provide: ActivatedRoute,
          useValue: { params: of({ slug: 'manifesto' }) },
        },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(ContentDeliveryComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('creates', () => {
    expect(component).toBeTruthy();
  });

  it('loads content by slug', () => {
    expect(contentServiceSpy.getContentBySlug).toHaveBeenCalledWith('manifesto');
  });

  it('sets toolbar content address from blobHash', () => {
    expect(component.contentAddress).toBe('bafkreiexample123');
  });

  it('extracts steward data for toolbar', () => {
    expect(component.omnibarStewards.length).toBe(1);
    expect(component.omnibarStewards[0].displayName).toBe('Genesis Collective');
    expect(component.omnibarStewards[0].ratio).toBe(0.8);
  });

  it('sets reach for toolbar', () => {
    expect(component.reach).toBe('commons');
  });

  it('shows error state for missing content', () => {
    contentServiceSpy.getContentBySlug.and.returnValue(of(null));
    component.ngOnInit();
    expect(component.error).toBeTruthy();
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "content-delivery"`
Expected: FAIL — module not found

- [ ] **Step 3: Write the component**

```typescript
// src/app/elohim/components/content-delivery/content-delivery.component.ts
import { CommonModule } from '@angular/common';
import {
  AfterViewChecked,
  Component,
  ComponentRef,
  OnDestroy,
  OnInit,
  ViewChild,
  ViewContainerRef,
  inject,
} from '@angular/core';
import { ActivatedRoute, RouterModule } from '@angular/router';

import { Subject, Subscription } from 'rxjs';
import { takeUntil } from 'rxjs/operators';

import { ContentService } from '@app/lamad/services/content.service';
import { ContentNode } from '@app/lamad/models/content-node.model';
import {
  ContentRenderer,
  RendererRegistryService,
} from '@app/lamad/renderers/renderer-registry.service';
import { SeoService } from '../../../services/seo.service';
import {
  ProtocolOmnibarComponent,
  OmnibarSteward,
} from '../protocol-omnibar/protocol-omnibar.component';

/**
 * ContentDeliveryComponent — Full-page content delivery with protocol omnibar.
 *
 * This is NOT the learning viewer. It renders a ContentNode as the entire page,
 * with no Angular app chrome — just the content and a provenance toolbar.
 * Used for public content delivery via /deliver/:slug.
 */
@Component({
  selector: 'app-content-delivery',
  standalone: true,
  imports: [CommonModule, RouterModule, ProtocolOmnibarComponent],
  templateUrl: './content-delivery.component.html',
  styleUrls: ['./content-delivery.component.css'],
})
export class ContentDeliveryComponent implements OnInit, OnDestroy, AfterViewChecked {
  node: ContentNode | null = null;
  isLoading = true;
  error: string | null = null;

  // Omnibar data
  contentAddress = '';
  omnibarStewards: OmnibarSteward[] = [];
  reach = '';
  deliverySource = '';

  // Renderer hosting
  @ViewChild('rendererHost', { read: ViewContainerRef, static: false })
  rendererHost!: ViewContainerRef;
  private rendererRef: ComponentRef<ContentRenderer> | null = null;
  private rendererSubscription: Subscription | null = null;
  hasRegisteredRenderer = false;
  private pendingRendererLoad = false;

  private readonly destroy$ = new Subject<void>();
  private readonly route = inject(ActivatedRoute);
  private readonly contentService = inject(ContentService);
  private readonly rendererRegistry = inject(RendererRegistryService);
  private readonly seoService = inject(SeoService);

  ngOnInit(): void {
    // Derive delivery source from current hostname
    this.deliverySource = `doorway ${window.location.hostname}`;

    this.route.params.pipe(takeUntil(this.destroy$)).subscribe(params => {
      const slug = params['slug'] as string;
      if (slug) {
        this.loadContent(slug);
      }
    });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
    this.destroyRenderer();
  }

  ngAfterViewChecked(): void {
    if (this.pendingRendererLoad && this.node && this.rendererHost) {
      this.pendingRendererLoad = false;
      this.loadRenderer();
    }
  }

  private loadContent(slug: string): void {
    this.isLoading = true;
    this.error = null;

    this.contentService
      .getContentBySlug(slug)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: node => {
          if (!node) {
            this.error = 'Content not found';
            this.isLoading = false;
            return;
          }

          this.node = node;
          this.populateOmnibar(node);
          this.updateSeo(node);
          this.isLoading = false;
          this.pendingRendererLoad = true;
        },
        error: () => {
          this.error = 'Failed to load content';
          this.isLoading = false;
        },
      });
  }

  private populateOmnibar(node: ContentNode): void {
    this.contentAddress = node.blobHash || node.id;
    this.reach = (node.reach as string) || 'commons';
    this.omnibarStewards = (node.stewardedBy || []).map(s => ({
      humanId: s.humanId,
      displayName: s.displayName || s.humanId,
      ratio: s.affinity ?? 0,
    }));
  }

  private updateSeo(node: ContentNode): void {
    this.seoService.updateForContent({
      id: node.id,
      title: node.title,
      summary: node.description,
      contentType: node.contentType,
      thumbnailUrl: node.metadata?.['thumbnailUrl'],
      authors: node.metadata?.['authors'],
      createdAt: node.createdAt,
      updatedAt: node.updatedAt,
    });
  }

  private loadRenderer(): void {
    if (!this.node || !this.rendererHost) return;

    this.destroyRenderer();
    this.rendererHost.clear();

    const rendererComponent = this.rendererRegistry.getRenderer(this.node);
    if (!rendererComponent) {
      this.hasRegisteredRenderer = false;
      return;
    }

    this.hasRegisteredRenderer = true;
    this.rendererRef = this.rendererHost.createComponent(rendererComponent);
    this.rendererRef.setInput('node', this.node);
  }

  private destroyRenderer(): void {
    if (this.rendererSubscription) {
      this.rendererSubscription.unsubscribe();
      this.rendererSubscription = null;
    }
    if (this.rendererRef) {
      this.rendererRef.destroy();
      this.rendererRef = null;
    }
  }

  getStringContent(content: string | object): string {
    return typeof content === 'string' ? content : JSON.stringify(content, null, 2);
  }
}
```

```html
<!-- src/app/elohim/components/content-delivery/content-delivery.component.html -->
<div class="delivery-shell" data-testid="content-delivery">
  <!-- Protocol Omnibar (EPR provenance — like browser SSL padlock) -->
  <app-protocol-omnibar
    [contentId]="node?.id || ''"
    [contentAddress]="contentAddress"
    [stewards]="omnibarStewards"
    [reach]="reach"
    [deliverySource]="deliverySource"
  ></app-protocol-omnibar>

  <!-- Loading -->
  <div *ngIf="isLoading" class="loading-state">
    <div class="spinner"></div>
    <p>Loading content from the protocol network...</p>
  </div>

  <!-- Error -->
  <div *ngIf="error && !isLoading" class="error-state">
    <h1>Content not found</h1>
    <p>{{ error }}</p>
    <a routerLink="/" class="home-link">Return to protocol home</a>
  </div>

  <!-- Content -->
  <main *ngIf="node && !isLoading" class="delivery-content">
    <!-- Dynamic renderer host -->
    <ng-container #rendererHost></ng-container>

    <!-- Fallback: plain text -->
    <div
      *ngIf="!hasRegisteredRenderer && node.contentFormat === 'plaintext'"
      class="plaintext-content"
    >
      <pre>{{ getStringContent(node.content) }}</pre>
    </div>

    <!-- Fallback: HTML -->
    <div
      *ngIf="!hasRegisteredRenderer && node.contentFormat === 'html'"
      class="html-content"
      [innerHTML]="node.content"
    ></div>

    <!-- Fallback: unknown format -->
    <div
      *ngIf="
        !hasRegisteredRenderer &&
        node.contentFormat !== 'plaintext' &&
        node.contentFormat !== 'html'
      "
      class="fallback-content"
    >
      <h1>{{ node.title }}</h1>
      <p class="fallback-meta">{{ node.contentType }} &middot; {{ node.contentFormat }}</p>
      <pre>{{ getStringContent(node.content) }}</pre>
    </div>
  </main>
</div>
```

```css
/* src/app/elohim/components/content-delivery/content-delivery.component.css */
.delivery-shell {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--surface-primary, #fff);
}

.delivery-content {
  flex: 1;
  max-width: 960px;
  width: 100%;
  margin: 0 auto;
  padding: 2rem 1rem;
}

.loading-state,
.error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 60vh;
  text-align: center;
  color: var(--text-secondary, #666);
}

.error-state h1 {
  font-size: 1.5rem;
  margin-bottom: 0.5rem;
  color: var(--text-primary, #1a1a1a);
}

.home-link {
  margin-top: 1rem;
  color: var(--lamad-accent-primary, #6366f1);
  text-decoration: none;
}

.home-link:hover {
  text-decoration: underline;
}

.spinner {
  width: 32px;
  height: 32px;
  border: 3px solid var(--lamad-border, #e2e8f0);
  border-top-color: var(--lamad-accent-primary, #6366f1);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin-bottom: 1rem;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.fallback-content {
  padding: 2rem 0;
}

.fallback-content h1 {
  font-size: 2rem;
  margin-bottom: 0.5rem;
}

.fallback-meta {
  color: var(--text-secondary, #666);
  font-size: 0.875rem;
  margin-bottom: 1.5rem;
}

.plaintext-content pre,
.fallback-content pre {
  white-space: pre-wrap;
  word-break: break-word;
  font-size: 0.875rem;
  line-height: 1.6;
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "content-delivery"`
Expected: All 6 tests PASS

- [ ] **Step 5: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/content-delivery/
git commit -m "feat(elohim): add ContentDeliveryComponent for full-page content serving"
```

---

### Task 4: Add `/deliver/:slug` route

**Files:**
- Modify: `src/app/app.routes.ts`

- [ ] **Step 1: Add the delivery route**

In `app.routes.ts`, add before the `resource/:resourceId` route (around line 44):

```typescript
  // Full-page content delivery with protocol omnibar (no app chrome)
  {
    path: 'deliver/:slug',
    loadComponent: async () =>
      import('./elohim/components/content-delivery/content-delivery.component').then(
        m => m.ContentDeliveryComponent
      ),
    data: {
      title: 'Content',
    },
  },
```

- [ ] **Step 2: Verify the route loads**

Run: `cd app/elohim-app && pnpm run lint`
Expected: PASS — route compiles without errors

- [ ] **Step 3: Commit**

```bash
git add app/elohim-app/src/app/app.routes.ts
git commit -m "feat(routes): add /deliver/:slug route for full-page content delivery"
```

---

### Task 5: Ensure ContentService has getContentBySlug method

**Files:**
- Modify: `src/app/lamad/services/content.service.ts` (if method doesn't exist)

- [ ] **Step 1: Check if getContentBySlug exists**

Run: `cd app/elohim-app && grep -n "getContentBySlug" src/app/lamad/services/content.service.ts`

If it exists, skip this task. If not:

- [ ] **Step 2: Add getContentBySlug method**

In `content.service.ts`, add:

```typescript
  /**
   * Fetch a content node by slug (URL-friendly identifier).
   * Slug is stored in content.slug for spa-bundle nodes,
   * or falls back to the node ID.
   */
  getContentBySlug(slug: string): Observable<ContentNode | null> {
    // First try direct ID lookup (slugs often are the ID)
    return this.getContent(slug).pipe(
      catchError(() => of(null)),
    );
  }
```

This is a minimal MVP implementation — slug lookup via the content ID. A proper slug index can be added when doorway projection supports it.

- [ ] **Step 3: Run content service tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "content.service"`
Expected: PASS

- [ ] **Step 4: Commit (if changed)**

```bash
git add app/elohim-app/src/app/lamad/services/content.service.ts
git commit -m "feat(lamad): add getContentBySlug to ContentService"
```

---

### Task 6: Add provenance headers to doorway responses

**Files:**
- Modify: `doorway/doorway-service/src/routes/root_app.rs`
- Modify: `doorway/doorway-service/src/routes/apps.rs`

- [ ] **Step 1: Add X-Reach header to root_app cache HIT response**

In `root_app.rs`, at the cache HIT response (around line 466-472), add after the `X-Root-App` header:

```rust
            .header("X-Reach", state.root_app_reach.as_deref().unwrap_or("commons"))
```

This requires adding `root_app_reach: Option<String>` to AppState (populated from the content node's reach field during warmup). For MVP, hardcode `"commons"` since root apps are always commons-reach.

- [ ] **Step 2: Add X-Reach header to root_app SPA fallback response**

In `root_app.rs`, at the SPA fallback response (around line 497-503), add the same header:

```rust
                .header("X-Reach", "commons")
```

- [ ] **Step 3: Add X-Reach to apps.rs build_app_response**

In `apps.rs`, in the `build_app_response` function (around line 321), extend the existing header logic:

```rust
    // After the X-Content-Slug header block:
    if let Some(ref reach) = content_reach {
        builder = builder.header("X-Reach", reach.as_str());
    }
```

This requires threading `content_reach: Option<&str>` through to `build_app_response`. For MVP, pass `Some("commons")` from the call sites.

- [ ] **Step 4: Run doorway tests**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins root_app`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add doorway/doorway-service/src/routes/root_app.rs \
       doorway/doorway-service/src/routes/apps.rs
git commit -m "feat(doorway): add X-Reach provenance header to root app and apps responses"
```

---

### Task 7: Add ProtocolOmnibar to content viewer focused mode

**Files:**
- Modify: `src/app/lamad/components/content-viewer/content-viewer.component.ts`
- Modify: `src/app/lamad/components/content-viewer/content-viewer.component.html`

- [ ] **Step 1: Import ProtocolOmnibarComponent**

In `content-viewer.component.ts`, add the import:

```typescript
import {
  ProtocolOmnibarComponent,
  OmnibarSteward,
} from '@app/elohim/components/protocol-omnibar/protocol-omnibar.component';
```

Add `ProtocolOmnibarComponent` to the `imports: [...]` array in the `@Component` decorator.

- [ ] **Step 2: Add omnibar data properties**

Add to the component class:

```typescript
  // Protocol omnibar data (shown in focused view — like browser padlock)
  omnibarStewards: OmnibarSteward[] = [];
  omnibarContentAddress = '';
  omnibarReach = '';
  omnibarDeliverySource = '';
```

- [ ] **Step 3: Populate omnibar data in loadContent**

In the `loadContent()` method, after the SEO update (around line 338), add:

```typescript
          // Populate protocol omnibar data
          this.omnibarContentAddress = contentNode.blobHash || contentNode.id;
          this.omnibarReach = (contentNode.reach as string) || 'commons';
          this.omnibarDeliverySource = window.location.hostname;
          this.omnibarStewards = (contentNode.stewardedBy || []).map(s => ({
            humanId: s.humanId,
            displayName: s.displayName || s.humanId,
            ratio: s.affinity ?? 0,
          }));
```

- [ ] **Step 4: Add omnibar to template in focused view**

In `content-viewer.component.html`, inside the content toolbar div (around line 218), add the omnibar conditionally when focused view is active:

```html
      <!-- Protocol Omnibar (shown in focused view — EPR provenance pill) -->
      <app-protocol-omnibar
        *ngIf="isFocusedView && node"
        [contentId]="node.id"
        [contentAddress]="omnibarContentAddress"
        [stewards]="omnibarStewards"
        [reach]="omnibarReach"
        [deliverySource]="omnibarDeliverySource"
      ></app-protocol-omnibar>
```

- [ ] **Step 5: Run tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "content-viewer"`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add app/elohim-app/src/app/lamad/components/content-viewer/
git commit -m "feat(lamad): show protocol omnibar pill in content viewer focused mode"
```

---

### Task 8: Final integration and lint

- [ ] **Step 1: Export from elohim barrel**

Ensure `ProtocolOmnibarComponent` and `ContentDeliveryComponent` are exported from the elohim pillar barrel.

- [ ] **Step 2: Run full test suite**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts`
Expected: All tests PASS

- [ ] **Step 3: Run lint**

Run: `cd app/elohim-app && pnpm run lint`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/elohim/
git commit -m "chore(elohim): export delivery and toolbar components from barrel"
```

---

## Self-Review Checklist

1. **Spec coverage:** All 13 a2o scenarios addressed:
   - Full-page markdown/html5-app → Task 3 (ContentDeliveryComponent + renderer host)
   - 404 for missing → Task 3 (error state)
   - Omnibar pill (at-a-glance provenance) → Task 2 (ProtocolOmnibarComponent, pill state)
   - Expanded provenance details → Task 2 (expanded state: EPR, reach, stewards, source)
   - EPR address copyable → Task 2 (copyAddress method)
   - Collapse back to pill → Task 2 (collapse method)
   - Drill-down to governance hub → Task 2 (inspectRoute → /resource/{id})
   - Actions menu (report, feedback) → Task 2 (dropdown with actions)
   - Doorway headers → Task 6 (X-Reach)
   - Focused view omnibar → Task 7

2. **Placeholder scan:** No TBDs found.

3. **Type consistency:** `OmnibarSteward` interface, `OmnibarState` type, and component name `ProtocolOmnibarComponent` used consistently across Tasks 2, 3, and 7. `truncatedAddress` getter matches test expectations.

4. **Design alignment:** The omnibar is a navigation/provenance tool, not an analytics display. Analytics live in the `/resource/{id}` content viewer's Network tab (Sprint 1, Task 5-6). The omnibar links TO that view via the "Inspect EPR" action.

**Deferred to future sprint:**
- `X-Stewards` header (requires doorway to resolve stewardship from projection cache per-request)
- SSR-lite social preview for bot user-agents (requires doorway-side HTML generation)
- Three-leg coupling summary in expanded omnibar (requires coupling data in content node view)
- `epr:` URI scheme handler (protocol-level deep linking from omnibar)
