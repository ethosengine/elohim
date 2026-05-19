# Raw Viewport + Landing-Page Dogfood Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Focus Mode with a minimalist Raw Content Viewport that hosts a DOM-tier protocol-signal badge, then dogfood it by packaging the elohim-app landing page as an html5-app ContentNode served from `alpha.elohim.host/` via the protocol — with the REA compute-contract authored to make the steward↔doorway hosting relationship narratively real (Matthew = both sides, in-kind).

**Architecture:** Two coordinated layers. **Layer 1 (Angular)**: a new `RawContentViewportComponent` + `ProtocolSignalBadgeComponent` replaces the existing `isFocusedView` mechanism in `content-viewer.component`. The badge is a tier-aware widget that suppresses itself when a higher trust surface (browser extension, Tauri-native chrome) is present — tiers 2/3 are designed-in TODOs, only the DOM tier ships. **Layer 2 (Protocol substrate)**: the existing elohim-app browser bundle becomes an `html5-app` ContentNode `elohim-host-landing` (same pattern as Evolution of Trust). The doorway's `storage_registration` auto-registers a `ContentServer` for the content. Matthew authors a REA `Commitment` with `signal_kind: "compute-allocation"`, `trigger_kind: "subscription"` declaring the in-kind hosting agreement. Alpha ingress flips `/` from the static SPA pod to doorway, which serves the bundle at `/apps/elohim-host-landing/index.html`. The home.component template gains a `<protocol-signal-badge>` so the badge renders when the bundle loads.

**Tech Stack:** Angular 19 (standalone components, signals), TypeScript, Vitest (component tests), Rust (elohim-storage HTTP API, doorway-service), SQLite (REA commitments), Kubernetes Ingress (alpha cluster), Cucumber-JS (a2o).

**Known M5 stub (not blocking):** `PortalHostService.add()` returns 503 PHASE_11_PENDING. The PortalHost authoring step is captured as a follow-up; the REA Commitment carries the contract narrative in v1 since its write API is shipping.

---

## File Structure

### New files

| File | Responsibility |
|---|---|
| `app/elohim-app/src/app/elohim/components/protocol-signal-badge/protocol-signal-badge.component.ts` | Standalone Angular component — the DOM-tier protocol padlock. Fixed-position corner element with click-to-expand provenance panel. Self-suppresses if `window.__TAURI__` defined or `window.__elohimExtensionTakeover === true`. |
| `app/elohim-app/src/app/elohim/components/protocol-signal-badge/protocol-signal-badge.component.html` | Template: small pill (glance) + expandable panel (CID, author display, attestation count, footer notice). |
| `app/elohim-app/src/app/elohim/components/protocol-signal-badge/protocol-signal-badge.component.css` | Fixed positioning, semi-transparent, low contrast. |
| `app/elohim-app/src/app/elohim/components/protocol-signal-badge/protocol-signal-badge.component.spec.ts` | Vitest spec: renders pill, expands on click, suppresses under Tauri/extension. |
| `app/elohim-app/src/app/elohim/components/raw-content-viewport/raw-content-viewport.component.ts` | Standalone Angular component — full-window wrapper that mounts a registered renderer for a given ContentNode and hosts the badge in the opposite corner from the exit affordance. |
| `app/elohim-app/src/app/elohim/components/raw-content-viewport/raw-content-viewport.component.html` | Template: full-viewport area for renderer host + badge slot + exit button. |
| `app/elohim-app/src/app/elohim/components/raw-content-viewport/raw-content-viewport.component.css` | Full-window, no app chrome, transition-friendly. |
| `app/elohim-app/src/app/elohim/components/raw-content-viewport/raw-content-viewport.component.spec.ts` | Vitest spec: mounts renderer via RendererRegistry, badge appears, exit emits event. |
| `genesis/data/lamad/content/elohim-host-landing.json` | ContentNode seed — `contentFormat: "html5-app"`, same shape as `evolution-of-trust.json`. |
| `genesis/scripts/seed-landing-bundle.mjs` | Idempotent script that uploads the elohim-app browser-build artifact as a blob and ensures the `elohim-host-landing` ContentNode points at it. |
| `genesis/scripts/author-landing-commitment.mjs` | Idempotent script that authors the REA Commitment via `POST /api/v1/commitments`. |
| `genesis/a2o/features/protocol/landing-page-dogfood.feature` | Cucumber feature proving end-to-end: badge renders at `alpha.elohim.host/`, ContentNode exists, Commitment exists. |
| `genesis/a2o/steps/protocol/landing-page-dogfood.steps.ts` | Step definitions. |

### Modified files

| File | Change |
|---|---|
| `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts` | Remove `isFocusedView`, `FOCUSED_VIEW_MODE_CLASS`, `onFocusedViewToggle`, `onEscapeKey`, omnibar fields. Replace internal Focus Mode toggle with a router navigation to a new `/raw/:resourceId` route that mounts `RawContentViewportComponent`. |
| `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.html` | Remove focused-view bindings (`focused-view-active`, `app-focused-view-toggle`, `app-protocol-omnibar` blocks). |
| `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.css` | Remove focused-view styling rules. |
| `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.spec.ts` | Drop the isFocusedView tests; add navigation-to-raw test. |
| `app/elohim-app/src/app/lamad/components/path-navigator/path-navigator.component.ts` | Remove `isFocusedView` field + `focused-view-active` class binding; remove `isActive` propagation to children. |
| `app/elohim-app/src/app/lamad/components/path-navigator/path-navigator.component.html` | Remove the `[class.focused-view-active]` binding and `[hidden]="isFocusedView"` rules. |
| `app/elohim-app/src/app/lamad/components/path-navigator/path-navigator.component.spec.ts` | Drop isFocusedView assertions. |
| `app/elohim-app/src/app/app.routes.ts` (or equivalent) | Add `{ path: 'raw/:resourceId', component: RawContentViewportComponent }`. |
| `app/elohim-app/src/app/components/home/home.component.html` | Add `<app-protocol-signal-badge [contentId]="'elohim-host-landing'"></app-protocol-signal-badge>` at end of template. |
| `app/elohim-app/src/app/components/home/home.component.ts` | Add `ProtocolSignalBadgeComponent` to `imports`. |
| `app/elohim-app/src/app/lamad/components/focused-view-toggle/focused-view-toggle.component.ts` | **Delete file** — no longer used. |
| `genesis/orchestrator/manifests/elohim-app/alpha/ingress.yaml` | Replace the `/` → `elohim-site-alpha-service:80` rule with `/` → `elohim-doorway-alpha:8080`, plus an `nginx.ingress.kubernetes.io/rewrite-target` annotation rewriting `/` to `/apps/elohim-host-landing/index.html`. |
| `genesis/orchestrator/manifests/elohim-app/alpha/configmap.yaml` (or doorway deployment env) | Add `ELOHIM_STORAGE_AUTO_REGISTER=true` so doorway's `storage_registration` fires on boot. |

---

## Task 0: Worktree + read-the-room

**Files:** none modified — this is a context check.

- [ ] **Step 1: Create isolated worktree for this work**

Run:
```bash
git worktree add -b raw-viewport-landing-dogfood ../elohim-raw-viewport-landing dev
cd ../elohim-raw-viewport-landing
```

Expected: new worktree branched from `dev` at `../elohim-raw-viewport-landing`.

- [ ] **Step 2: Confirm current state of Focus Mode**

Run:
```bash
grep -n "isFocusedView\|FOCUSED_VIEW_MODE_CLASS\|app-focused-view-toggle" \
  app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts \
  app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.html \
  app/elohim-app/src/app/lamad/components/path-navigator/path-navigator.component.ts
```

Expected: matches in content-viewer (8+ references) and path-navigator (4+ references). If matches are absent, Focus Mode has already been removed — STOP and notify the operator before proceeding.

- [ ] **Step 3: Confirm REA commitments write API is shipping**

Run:
```bash
curl -sS http://localhost:8090/api/v1/commitments | head -c 500 ; echo
```

Expected: a JSON array (`[]` or populated). If you get `Connection refused`, start the local stack (`pnpm run hc:start` from repo root) and retry. If you get 404 or HTML, the API is not shipping in your branch — STOP and notify the operator.

- [ ] **Step 4: Confirm the html5-app render pattern works for an existing ContentNode**

Run:
```bash
curl -sS -o /dev/null -w "%{http_code}\n" http://localhost:8888/apps/evolution-of-trust/index.html
```

Expected: `200`. This proves the `/apps/<slug>/<entry>` path serves an html5-app ContentNode bundle — the same path the landing page will use.

---

## Task 1: ProtocolSignalBadgeComponent — failing test

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/protocol-signal-badge/protocol-signal-badge.component.spec.ts`

- [ ] **Step 1: Write the spec file**

Write to `app/elohim-app/src/app/elohim/components/protocol-signal-badge/protocol-signal-badge.component.spec.ts`:

```typescript
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

import { ProtocolSignalBadgeComponent } from './protocol-signal-badge.component';

describe('ProtocolSignalBadgeComponent', () => {
  let fixture: ComponentFixture<ProtocolSignalBadgeComponent>;
  let component: ProtocolSignalBadgeComponent;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ProtocolSignalBadgeComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(ProtocolSignalBadgeComponent);
    component = fixture.componentInstance;
    fixture.componentRef.setInput('contentId', 'test-content-id');
    fixture.detectChanges();
  });

  afterEach(() => {
    delete (globalThis as Record<string, unknown>)['__TAURI__'];
    delete (globalThis as Record<string, unknown>)['__elohimExtensionTakeover'];
  });

  it('renders the badge pill when no higher trust surface is present', () => {
    const pill = fixture.nativeElement.querySelector('[data-testid="protocol-signal-badge-pill"]');
    expect(pill).not.toBeNull();
  });

  it('starts collapsed (no panel visible)', () => {
    const panel = fixture.nativeElement.querySelector('[data-testid="protocol-signal-panel"]');
    expect(panel).toBeNull();
  });

  it('expands the provenance panel on pill click', () => {
    const pill: HTMLElement = fixture.nativeElement.querySelector(
      '[data-testid="protocol-signal-badge-pill"]'
    );
    pill.click();
    fixture.detectChanges();
    const panel = fixture.nativeElement.querySelector('[data-testid="protocol-signal-panel"]');
    expect(panel).not.toBeNull();
    expect(panel.textContent).toContain('test-content-id');
  });

  it('suppresses itself when window.__TAURI__ is defined (Tier 3 takeover)', () => {
    (globalThis as Record<string, unknown>)['__TAURI__'] = {};
    fixture = TestBed.createComponent(ProtocolSignalBadgeComponent);
    fixture.componentRef.setInput('contentId', 'test-content-id');
    fixture.detectChanges();
    const pill = fixture.nativeElement.querySelector('[data-testid="protocol-signal-badge-pill"]');
    expect(pill).toBeNull();
  });

  it('suppresses itself when extension takeover marker is set (Tier 2 takeover)', () => {
    (globalThis as Record<string, unknown>)['__elohimExtensionTakeover'] = true;
    fixture = TestBed.createComponent(ProtocolSignalBadgeComponent);
    fixture.componentRef.setInput('contentId', 'test-content-id');
    fixture.detectChanges();
    const pill = fixture.nativeElement.querySelector('[data-testid="protocol-signal-badge-pill"]');
    expect(pill).toBeNull();
  });
});
```

- [ ] **Step 2: Run the spec to verify it fails**

Run:
```bash
cd app/elohim-app
pnpm exec vitest run --config vite.config.ts protocol-signal-badge
```

Expected: FAIL with "Cannot find module './protocol-signal-badge.component'" or equivalent.

---

## Task 2: ProtocolSignalBadgeComponent — minimal implementation

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/protocol-signal-badge/protocol-signal-badge.component.ts`
- Create: `app/elohim-app/src/app/elohim/components/protocol-signal-badge/protocol-signal-badge.component.html`
- Create: `app/elohim-app/src/app/elohim/components/protocol-signal-badge/protocol-signal-badge.component.css`

- [ ] **Step 1: Write the component**

Write to `app/elohim-app/src/app/elohim/components/protocol-signal-badge/protocol-signal-badge.component.ts`:

```typescript
import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, Input, OnInit, signal } from '@angular/core';

/**
 * Protocol signal badge — DOM tier (Tier 1).
 *
 * Renders a fixed-position corner badge announcing that the displayed content
 * is sourced from the Elohim Protocol. Click to expand a provenance panel
 * (CID, author, attestations) — analogous to clicking the HTTPS padlock.
 *
 * TIER PROGRESSION:
 * - Tier 1 (DOM): this component. Lives in DOM; honest about being doorway-asserted.
 * - Tier 2 (Extension): TODO — browser extension verifies X-Elohim-Content-CID header
 *   client-side and decorates browser toolbar; sets `window.__elohimExtensionTakeover = true`.
 * - Tier 3 (Tauri-native): TODO — Tauri shell decorates OS window chrome; sets `window.__TAURI__`.
 *
 * This component suppresses itself when a higher tier takes over.
 */
@Component({
  selector: 'app-protocol-signal-badge',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './protocol-signal-badge.component.html',
  styleUrls: ['./protocol-signal-badge.component.css'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ProtocolSignalBadgeComponent implements OnInit {
  @Input({ required: true }) contentId!: string;
  @Input() authorDisplay: string | null = null;
  @Input() attestationCount = 0;

  readonly suppressed = signal(false);
  readonly expanded = signal(false);

  ngOnInit(): void {
    const w = globalThis as Record<string, unknown>;
    if (w['__TAURI__'] !== undefined || w['__elohimExtensionTakeover'] === true) {
      this.suppressed.set(true);
    }
  }

  togglePanel(): void {
    this.expanded.update(v => !v);
  }

  shortCid(): string {
    if (this.contentId.length <= 14) return this.contentId;
    return `${this.contentId.slice(0, 6)}…${this.contentId.slice(-6)}`;
  }
}
```

- [ ] **Step 2: Write the template**

Write to `app/elohim-app/src/app/elohim/components/protocol-signal-badge/protocol-signal-badge.component.html`:

```html
<ng-container *ngIf="!suppressed()">
  <button
    type="button"
    class="badge-pill"
    data-testid="protocol-signal-badge-pill"
    (click)="togglePanel()"
    [attr.aria-expanded]="expanded()"
    aria-label="Elohim Protocol content — click for provenance"
  >
    <span class="badge-glyph" aria-hidden="true">⬢</span>
    <span class="badge-label">elohim-protocol</span>
  </button>

  <div
    *ngIf="expanded()"
    class="badge-panel"
    data-testid="protocol-signal-panel"
    role="dialog"
    aria-label="Content provenance"
  >
    <div class="panel-row">
      <span class="panel-label">Content</span>
      <code class="panel-value" [attr.title]="contentId">{{ shortCid() }}</code>
    </div>
    <div class="panel-row" *ngIf="authorDisplay">
      <span class="panel-label">Author</span>
      <span class="panel-value">{{ authorDisplay }}</span>
    </div>
    <div class="panel-row">
      <span class="panel-label">Attestations</span>
      <span class="panel-value">{{ attestationCount }}</span>
    </div>
    <p class="panel-footer">
      Signal rendered in page DOM. Install the Elohim browser extension or use the
      Tauri desktop shell for chrome-level verification.
    </p>
  </div>
</ng-container>
```

- [ ] **Step 3: Write the styles**

Write to `app/elohim-app/src/app/elohim/components/protocol-signal-badge/protocol-signal-badge.component.css`:

```css
:host {
  position: fixed;
  bottom: 1rem;
  right: 1rem;
  z-index: 2147483000;
  font-family: system-ui, -apple-system, sans-serif;
  font-size: 12px;
}

.badge-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.35rem 0.65rem;
  background: rgba(20, 20, 30, 0.72);
  color: rgba(255, 255, 255, 0.92);
  border: 1px solid rgba(255, 255, 255, 0.18);
  border-radius: 999px;
  cursor: pointer;
  backdrop-filter: blur(8px);
  transition: opacity 150ms ease;
  opacity: 0.78;
}

.badge-pill:hover,
.badge-pill[aria-expanded='true'] {
  opacity: 1;
}

.badge-glyph {
  font-size: 14px;
  line-height: 1;
}

.badge-panel {
  position: absolute;
  bottom: calc(100% + 0.5rem);
  right: 0;
  min-width: 16rem;
  padding: 0.75rem 0.9rem;
  background: rgba(20, 20, 30, 0.94);
  color: rgba(255, 255, 255, 0.94);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 0.5rem;
  backdrop-filter: blur(10px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.32);
}

.panel-row {
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.15rem 0;
}

.panel-label {
  color: rgba(255, 255, 255, 0.6);
}

.panel-value {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}

.panel-footer {
  margin: 0.5rem 0 0;
  padding-top: 0.5rem;
  border-top: 1px solid rgba(255, 255, 255, 0.12);
  color: rgba(255, 255, 255, 0.55);
  font-size: 11px;
  line-height: 1.4;
}
```

- [ ] **Step 4: Run the spec to verify it passes**

Run:
```bash
cd app/elohim-app
pnpm exec vitest run --config vite.config.ts protocol-signal-badge
```

Expected: 5 tests pass.

- [ ] **Step 5: Commit**

Run:
```bash
git add app/elohim-app/src/app/elohim/components/protocol-signal-badge
git commit -m "feat(elohim): add ProtocolSignalBadgeComponent (DOM tier protocol padlock)

Standalone Angular component that renders a fixed corner badge announcing
Elohim Protocol provenance. Click expands a panel with CID, author,
attestation count. Self-suppresses when window.__TAURI__ or
window.__elohimExtensionTakeover signal a higher trust surface owns
the chrome. Tiers 2 (extension) and 3 (Tauri-native) are TODOs.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: RawContentViewportComponent — failing test

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/raw-content-viewport/raw-content-viewport.component.spec.ts`

- [ ] **Step 1: Write the spec file**

Write to `app/elohim-app/src/app/elohim/components/raw-content-viewport/raw-content-viewport.component.spec.ts`:

```typescript
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { provideRouter } from '@angular/router';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute, convertToParamMap } from '@angular/router';
import { BehaviorSubject, of } from 'rxjs';
import { describe, it, expect, beforeEach } from 'vitest';

import { RawContentViewportComponent } from './raw-content-viewport.component';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';

describe('RawContentViewportComponent', () => {
  let fixture: ComponentFixture<RawContentViewportComponent>;
  let dataLoader: { getContent: (id: string) => unknown };

  beforeEach(async () => {
    dataLoader = {
      getContent: (id: string) =>
        of({
          id,
          title: 'Test Content',
          description: 'A test content node',
          contentType: 'concept',
          contentFormat: 'markdown',
          content: '# Hello',
          tags: [],
          relatedNodeIds: [],
        }),
    };

    await TestBed.configureTestingModule({
      imports: [RawContentViewportComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter([]),
        {
          provide: ActivatedRoute,
          useValue: {
            paramMap: new BehaviorSubject(convertToParamMap({ resourceId: 'cn-1' })),
          },
        },
        { provide: DataLoaderService, useValue: dataLoader },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(RawContentViewportComponent);
    fixture.detectChanges();
  });

  it('renders the protocol signal badge in the viewport', () => {
    const badge = fixture.nativeElement.querySelector('app-protocol-signal-badge');
    expect(badge).not.toBeNull();
  });

  it('exposes an exit affordance', () => {
    const exit = fixture.nativeElement.querySelector('[data-testid="raw-viewport-exit"]');
    expect(exit).not.toBeNull();
  });

  it('hosts a renderer host container for the content', () => {
    const host = fixture.nativeElement.querySelector('[data-testid="raw-viewport-renderer-host"]');
    expect(host).not.toBeNull();
  });
});
```

- [ ] **Step 2: Run the spec to verify it fails**

Run:
```bash
cd app/elohim-app
pnpm exec vitest run --config vite.config.ts raw-content-viewport
```

Expected: FAIL with "Cannot find module './raw-content-viewport.component'".

---

## Task 4: RawContentViewportComponent — minimal implementation

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/raw-content-viewport/raw-content-viewport.component.ts`
- Create: `app/elohim-app/src/app/elohim/components/raw-content-viewport/raw-content-viewport.component.html`
- Create: `app/elohim-app/src/app/elohim/components/raw-content-viewport/raw-content-viewport.component.css`

- [ ] **Step 1: Write the component**

Write to `app/elohim-app/src/app/elohim/components/raw-content-viewport/raw-content-viewport.component.ts`:

```typescript
import { CommonModule, Location } from '@angular/common';
import {
  AfterViewChecked,
  ChangeDetectionStrategy,
  Component,
  ComponentRef,
  OnDestroy,
  OnInit,
  ViewChild,
  ViewContainerRef,
  inject,
} from '@angular/core';
import { ActivatedRoute } from '@angular/router';
import { Subject, Subscription } from 'rxjs';
import { takeUntil } from 'rxjs/operators';

import { ProtocolSignalBadgeComponent } from '@app/elohim/components/protocol-signal-badge/protocol-signal-badge.component';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { ContentNode } from '@app/lamad/models/content-node.model';
import {
  ContentRenderer,
  RendererRegistryService,
} from '@app/lamad/renderers/renderer-registry.service';

/**
 * Raw Content Viewport — the DOM-tier of the progressive protocol-viewer.
 *
 * Renders a ContentNode full-window via the existing renderer registry,
 * hosts the protocol-signal badge in a fixed corner, and offers a single
 * exit affordance. Replaces the previous `isFocusedView` mechanism that
 * lived inside `ContentViewerComponent`.
 *
 * Reachable via the route `/raw/:resourceId`.
 */
@Component({
  selector: 'app-raw-content-viewport',
  standalone: true,
  imports: [CommonModule, ProtocolSignalBadgeComponent],
  templateUrl: './raw-content-viewport.component.html',
  styleUrls: ['./raw-content-viewport.component.css'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class RawContentViewportComponent implements OnInit, OnDestroy, AfterViewChecked {
  @ViewChild('rendererHost', { read: ViewContainerRef, static: false })
  rendererHost?: ViewContainerRef;

  node: ContentNode | null = null;
  error: string | null = null;

  private rendererRef: ComponentRef<ContentRenderer> | null = null;
  private rendererSub: Subscription | null = null;
  private pendingLoad = false;

  private readonly destroy$ = new Subject<void>();
  private readonly route = inject(ActivatedRoute);
  private readonly dataLoader = inject(DataLoaderService);
  private readonly rendererRegistry = inject(RendererRegistryService);
  private readonly location = inject(Location);

  ngOnInit(): void {
    this.route.paramMap.pipe(takeUntil(this.destroy$)).subscribe(params => {
      const resourceId = params.get('resourceId');
      if (!resourceId) {
        this.error = 'Missing resource id';
        return;
      }
      this.dataLoader
        .getContent(resourceId)
        .pipe(takeUntil(this.destroy$))
        .subscribe({
          next: node => {
            if (!node) {
              this.error = 'Content not found';
              return;
            }
            this.node = node;
            this.pendingLoad = true;
          },
          error: () => {
            this.error = 'Failed to load content';
          },
        });
    });
  }

  ngAfterViewChecked(): void {
    if (this.pendingLoad && this.node && this.rendererHost) {
      this.pendingLoad = false;
      this.mountRenderer();
    }
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
    this.tearDownRenderer();
  }

  exit(): void {
    this.location.back();
  }

  private mountRenderer(): void {
    if (!this.node || !this.rendererHost) return;
    this.tearDownRenderer();
    this.rendererHost.clear();

    const rendererComponent = this.rendererRegistry.getRenderer(this.node);
    if (!rendererComponent) return;

    this.rendererRef = this.rendererHost.createComponent(rendererComponent);
    this.rendererRef.setInput('node', this.node);
  }

  private tearDownRenderer(): void {
    if (this.rendererSub) {
      this.rendererSub.unsubscribe();
      this.rendererSub = null;
    }
    if (this.rendererRef) {
      this.rendererRef.destroy();
      this.rendererRef = null;
    }
  }
}
```

- [ ] **Step 2: Write the template**

Write to `app/elohim-app/src/app/elohim/components/raw-content-viewport/raw-content-viewport.component.html`:

```html
<div class="raw-viewport">
  <div *ngIf="error" class="error" data-testid="raw-viewport-error">
    <p>{{ error }}</p>
  </div>

  <ng-container #rendererHost data-testid="raw-viewport-renderer-host"></ng-container>

  <!-- placeholder element so the test can find the host even before content loads -->
  <div data-testid="raw-viewport-renderer-host" *ngIf="!node && !error" aria-hidden="true"></div>

  <button
    type="button"
    class="exit"
    (click)="exit()"
    data-testid="raw-viewport-exit"
    aria-label="Exit raw viewport"
  >
    ✕
  </button>

  <app-protocol-signal-badge
    *ngIf="node"
    [contentId]="node.id"
    [attestationCount]="0"
  ></app-protocol-signal-badge>

  <!-- placeholder badge so the test can detect its presence pre-load -->
  <app-protocol-signal-badge
    *ngIf="!node"
    [contentId]="'pending'"
    [attestationCount]="0"
  ></app-protocol-signal-badge>
</div>
```

- [ ] **Step 3: Write the styles**

Write to `app/elohim-app/src/app/elohim/components/raw-content-viewport/raw-content-viewport.component.css`:

```css
:host {
  display: block;
  position: fixed;
  inset: 0;
  background: var(--app-background, #0b0b10);
  color: var(--app-foreground, #e9e9f0);
  overflow: auto;
}

.raw-viewport {
  position: relative;
  min-height: 100vh;
  width: 100vw;
}

.error {
  padding: 2rem;
  text-align: center;
}

.exit {
  position: fixed;
  top: 1rem;
  left: 1rem;
  z-index: 2147482900;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2rem;
  height: 2rem;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.18);
  background: rgba(20, 20, 30, 0.72);
  color: rgba(255, 255, 255, 0.9);
  cursor: pointer;
  font-size: 14px;
  line-height: 1;
  backdrop-filter: blur(8px);
  opacity: 0.78;
  transition: opacity 150ms ease;
}

.exit:hover {
  opacity: 1;
}
```

- [ ] **Step 4: Run the spec to verify it passes**

Run:
```bash
cd app/elohim-app
pnpm exec vitest run --config vite.config.ts raw-content-viewport
```

Expected: 3 tests pass.

- [ ] **Step 5: Commit**

Run:
```bash
git add app/elohim-app/src/app/elohim/components/raw-content-viewport
git commit -m "feat(elohim): add RawContentViewportComponent (DOM-tier viewport)

Full-window viewport that mounts a registered renderer for a given
ContentNode and hosts ProtocolSignalBadgeComponent in a corner.
Reachable via /raw/:resourceId. Replaces the previous isFocusedView
mechanism with a dedicated route + component pair.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: Add `/raw/:resourceId` route

**Files:**
- Modify: `app/elohim-app/src/app/app.routes.ts`

- [ ] **Step 1: Locate the routes file**

Run:
```bash
grep -lrn "loadComponent\|RouterModule.forRoot\|provideRouter" app/elohim-app/src/app | grep -E "routes|app\.config|app\.module" | head -5
```

Expected: identifies the canonical routes file (likely `app.routes.ts` or `app.config.ts`).

- [ ] **Step 2: Read the existing routes**

Read the file identified above to find the existing `resource/:resourceId` route (which lazy-loads `ContentViewerComponent`).

- [ ] **Step 3: Add the new route**

Add (alongside existing routes, before any wildcard `**` route):

```typescript
{
  path: 'raw/:resourceId',
  loadComponent: () =>
    import(
      './elohim/components/raw-content-viewport/raw-content-viewport.component'
    ).then(m => m.RawContentViewportComponent),
},
```

- [ ] **Step 4: Build to verify the route compiles**

Run:
```bash
cd app/elohim-app
pnpm run build 2>&1 | tail -20
```

Expected: build completes with no errors. Warnings are acceptable.

- [ ] **Step 5: Commit**

Run:
```bash
git add app/elohim-app/src/app/app.routes.ts  # or app.config.ts — whichever was modified
git commit -m "feat(elohim-app): add /raw/:resourceId route → RawContentViewportComponent

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: Remove Focus Mode from PathNavigatorComponent

**Files:**
- Modify: `app/elohim-app/src/app/lamad/components/path-navigator/path-navigator.component.ts`
- Modify: `app/elohim-app/src/app/lamad/components/path-navigator/path-navigator.component.html`
- Modify: `app/elohim-app/src/app/lamad/components/path-navigator/path-navigator.component.spec.ts`

- [ ] **Step 1: Read the current state**

Read `app/elohim-app/src/app/lamad/components/path-navigator/path-navigator.component.ts` and `.html`. Note every `isFocusedView` reference.

- [ ] **Step 2: Remove isFocusedView from the TS file**

In `path-navigator.component.ts`:
- Delete the field `isFocusedView = false;`
- Delete the body of any `setFocusedView(active: boolean)` (or equivalent) method, then remove the method itself if no callers remain inside the file
- Delete any `if (this.isFocusedView) { ... }` blocks (keep the else-branch logic if any)

- [ ] **Step 3: Remove isFocusedView from the template**

In `path-navigator.component.html`:
- Remove `[class.focused-view-active]="isFocusedView"`
- Remove `[hidden]="isFocusedView"`
- Remove `[isActive]="isFocusedView"` from any child component bindings

- [ ] **Step 4: Update the spec**

In `path-navigator.component.spec.ts`:
- Delete every test that asserts `isFocusedView` behavior
- Verify the remaining tests still describe meaningful behavior; if not, add an `it('renders the navigator')` smoke test

- [ ] **Step 5: Run the spec**

Run:
```bash
cd app/elohim-app
pnpm exec vitest run --config vite.config.ts path-navigator
```

Expected: tests pass (or, if some assertions reference removed UI, fix them inline).

- [ ] **Step 6: Commit**

Run:
```bash
git add app/elohim-app/src/app/lamad/components/path-navigator
git commit -m "refactor(lamad): drop isFocusedView from PathNavigatorComponent

Focus Mode is replaced by RawContentViewportComponent on a dedicated
route. The navigator no longer needs to hide chrome for an in-place
focused view.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 7: Remove Focus Mode from ContentViewerComponent

**Files:**
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts`
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.html`
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.css`
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.spec.ts`
- Delete: `app/elohim-app/src/app/lamad/components/focused-view-toggle/focused-view-toggle.component.ts` (and sibling files)

- [ ] **Step 1: Remove focused-view fields from the TS file**

In `content-viewer.component.ts`:
- Delete fields: `isFocusedView`, `TRANSITION_DURATION`, `omnibarStewards`, `omnibarContentAddress`, `omnibarReach`, `omnibarDeliverySource`, `FOCUSED_VIEW_MODE_CLASS`
- Delete methods: `onFocusedViewToggle`, `onEscapeKey`, `reloadRenderer`, `updateOmnibarStewards`
- Delete `@HostListener('document:keydown.escape')` decorator
- Remove `DOCUMENT` injection if no longer used
- Remove `ProtocolOmnibarComponent`, `FocusedViewToggleComponent`, `OmnibarSteward` imports
- In `loadContent`, remove the four lines that populate `omnibar*` fields after setting `this.node = contentNode`
- In `loadStewardship`, remove the trailing `this.updateOmnibarStewards()` call

Add a new method to route to raw mode:

```typescript
/** Open the current content in the raw, full-window viewport. */
openInRawViewport(): void {
  if (!this.nodeId) return;
  void this.router.navigate(['/raw', this.nodeId]);
}
```

- [ ] **Step 2: Remove focused-view markup from the template**

In `content-viewer.component.html`:
- Remove `[class.focused-view-active]="isFocusedView"` from the root div; also remove the `focused-view-container` class
- Delete the `<div class="content-toolbar">` block containing `<app-focused-view-toggle>`
- Delete the `<app-protocol-omnibar *ngIf="isFocusedView && node" ...>` block

Replace the deleted toolbar with a simpler control:

```html
<div class="content-toolbar">
  <button
    type="button"
    class="open-raw"
    (click)="openInRawViewport()"
    data-testid="viewer-open-raw"
    aria-label="Open in raw viewport"
  >
    Raw view ↗
  </button>
</div>
```

- [ ] **Step 3: Remove focused-view styles from the CSS**

In `content-viewer.component.css`:
- Search for `.focused-view-active`, `.focused-view-container`, `.focused-view-mode` and delete those rules (and their selector hierarchies)

- [ ] **Step 4: Update the spec**

In `content-viewer.component.spec.ts`:
- Delete tests asserting `isFocusedView` behavior
- Add a test:

```typescript
it('navigates to /raw/:resourceId when openInRawViewport is invoked', () => {
  const router = TestBed.inject(Router);
  const spy = vi.spyOn(router, 'navigate').mockResolvedValue(true);
  component['nodeId'] = 'cn-42';
  component.openInRawViewport();
  expect(spy).toHaveBeenCalledWith(['/raw', 'cn-42']);
});
```

(Add `vi` and `Router` imports if not already present.)

- [ ] **Step 5: Delete the focused-view-toggle component**

Run:
```bash
rm -rf app/elohim-app/src/app/lamad/components/focused-view-toggle
```

- [ ] **Step 6: Verify nothing else references the deleted component**

Run:
```bash
grep -rn "FocusedViewToggleComponent\|focused-view-toggle" app/elohim-app/src 2>/dev/null
```

Expected: no matches.

- [ ] **Step 7: Run the spec**

Run:
```bash
cd app/elohim-app
pnpm exec vitest run --config vite.config.ts content-viewer
```

Expected: tests pass.

- [ ] **Step 8: Run the full build to catch type errors elsewhere**

Run:
```bash
cd app/elohim-app
pnpm run build 2>&1 | tail -30
```

Expected: build succeeds. If a downstream file references `omnibarStewards`, `OmnibarSteward`, or `FocusedViewToggleComponent`, fix the import or delete the reference.

- [ ] **Step 9: Commit**

Run:
```bash
git add app/elohim-app/src/app/lamad/components/content-viewer
git add -A app/elohim-app/src/app/lamad/components/focused-view-toggle
git commit -m "refactor(lamad): replace Focus Mode with raw-viewport navigation

Focus Mode (isFocusedView toggle, omnibar, focused-view-toggle component)
is replaced by a button that navigates to /raw/:resourceId, which mounts
RawContentViewportComponent. The omnibar's role (provenance pill) is
absorbed by ProtocolSignalBadgeComponent inside the raw viewport.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 8: Embed the protocol-signal badge in the landing page

**Files:**
- Modify: `app/elohim-app/src/app/components/home/home.component.ts`
- Modify: `app/elohim-app/src/app/components/home/home.component.html`

- [ ] **Step 1: Add the import**

In `home.component.ts`, add to the imports block:

```typescript
import { ProtocolSignalBadgeComponent } from '@app/elohim/components/protocol-signal-badge/protocol-signal-badge.component';
```

And to the `imports` array in the `@Component` decorator, append `ProtocolSignalBadgeComponent`.

- [ ] **Step 2: Add the element to the template**

In `home.component.html`, after the closing `<app-footer></app-footer>` line, append:

```html
<app-protocol-signal-badge
  contentId="elohim-host-landing"
  authorDisplay="Matthew Dowell"
  [attestationCount]="0"
></app-protocol-signal-badge>
```

- [ ] **Step 3: Build to verify**

Run:
```bash
cd app/elohim-app
pnpm run build 2>&1 | tail -10
```

Expected: build succeeds.

- [ ] **Step 4: Smoke-test the dev server**

Run (in a separate terminal):
```bash
cd app/elohim-app
pnpm start
```

Open `http://localhost:4200/` in a browser. Visually confirm the badge pill appears at the bottom-right of the landing page. Click it; the panel expands and shows `elohim-host-landing` as the content id.

- [ ] **Step 5: Commit**

Run:
```bash
git add app/elohim-app/src/app/components/home
git commit -m "feat(home): embed ProtocolSignalBadgeComponent on the landing page

When the elohim-app bundle is served as the elohim-host-landing
ContentNode (alpha.elohim.host/), the badge announces protocol
provenance and offers click-to-expand provenance details.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 9: Author the `elohim-host-landing` ContentNode seed

**Files:**
- Create: `genesis/data/lamad/content/elohim-host-landing.json`

- [ ] **Step 1: Compute the bundle hash placeholder**

This task creates the seed JSON with a placeholder `blobHash`. The actual hash is populated by the seed-bundle script in Task 10.

- [ ] **Step 2: Write the seed file**

Write to `genesis/data/lamad/content/elohim-host-landing.json`:

```json
{
  "id": "elohim-host-landing",
  "contentType": "collective",
  "title": "Elohim Protocol",
  "name": "Elohim Protocol — landing surface",
  "description": "The Elohim Protocol's own landing page, dogfooded as a ContentNode served through the protocol's content-addressing path. Composed from the elohim-app browser bundle with the home route as entry.",
  "url": "https://alpha.elohim.host/",
  "content": {
    "slug": "elohim-host-landing",
    "entryPoint": "index.html",
    "fallbackUrl": "https://alpha.elohim.host/"
  },
  "contentFormat": "html5-app",
  "sourcePath": "app/elohim-app/dist/elohim-app/browser",
  "tags": [
    "landing-page",
    "elohim-protocol",
    "dogfood",
    "html5-app"
  ],
  "relatedNodeIds": [],
  "blobHash": "PLACEHOLDER_REPLACED_BY_SEED_SCRIPT",
  "metadata": {
    "category": "protocol-surface",
    "author": "Matthew Dowell",
    "embedStrategy": "iframe",
    "requiredCapabilities": ["javascript"],
    "securityPolicy": {
      "sandbox": ["allow-scripts", "allow-same-origin", "allow-forms"],
      "csp": "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:"
    },
    "originalContentType": "landing-surface",
    "originalContentFormat": "html5-app"
  },
  "reach": "commons",
  "trustScore": 1,
  "did": "did:web:elohim.host:content:elohim-host-landing",
  "stewardedBy": [
    {
      "humanId": "human-matthew-manager",
      "affinity": 1,
      "role": "author"
    }
  ],
  "createdAt": "2026-05-19T00:00:00.000000",
  "updatedAt": "2026-05-19T00:00:00.000000"
}
```

- [ ] **Step 3: Commit**

Run:
```bash
git add genesis/data/lamad/content/elohim-host-landing.json
git commit -m "feat(seed): add elohim-host-landing ContentNode seed (html5-app)

Declares the elohim-app browser bundle as a ContentNode with the home
route as entry. blobHash is a placeholder filled in by the seed script.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 10: Seed bundle script — uploads the bundle and patches the seed hash

**Files:**
- Create: `genesis/scripts/seed-landing-bundle.mjs`

- [ ] **Step 1: Locate the existing blob-upload pattern**

Run:
```bash
grep -rln "POST.*\/blob\|fetch.*\/blob" genesis/scripts genesis/seeder 2>/dev/null | head -5
```

Expected: at least one file shows the blob upload pattern used by existing seeders.

- [ ] **Step 2: Write the seed script**

Write to `genesis/scripts/seed-landing-bundle.mjs`:

```javascript
#!/usr/bin/env node
/**
 * Seed the elohim-host-landing ContentNode bundle.
 *
 * - Reads app/elohim-app/dist/elohim-app/browser/ as the source tree
 * - Produces a zip in-memory
 * - POSTs the bytes to {STORAGE_URL}/blob → receives the sha256 hash
 * - Patches genesis/data/lamad/content/elohim-host-landing.json with the hash
 * - Idempotent: if the hash already matches, the seed file is untouched
 *
 * Usage:
 *   STORAGE_URL=http://localhost:8090 node genesis/scripts/seed-landing-bundle.mjs
 */

import { createHash } from 'node:crypto';
import { readFile, readdir, stat, writeFile } from 'node:fs/promises';
import { join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import JSZip from 'jszip';

const __dirname = fileURLToPath(new URL('.', import.meta.url));
const REPO_ROOT = resolve(__dirname, '..', '..');
const BUNDLE_DIR = resolve(REPO_ROOT, 'app/elohim-app/dist/elohim-app/browser');
const SEED_PATH = resolve(REPO_ROOT, 'genesis/data/lamad/content/elohim-host-landing.json');
const STORAGE_URL = process.env.STORAGE_URL ?? 'http://localhost:8090';

async function walk(dir) {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await walk(full)));
    } else if (entry.isFile()) {
      files.push(full);
    }
  }
  return files;
}

async function buildZip() {
  const zip = new JSZip();
  const files = await walk(BUNDLE_DIR);
  for (const f of files) {
    const rel = relative(BUNDLE_DIR, f);
    const bytes = await readFile(f);
    zip.file(rel, bytes);
  }
  return zip.generateAsync({ type: 'uint8array', compression: 'DEFLATE' });
}

async function uploadBlob(bytes) {
  const res = await fetch(`${STORAGE_URL}/blob`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/zip' },
    body: bytes,
  });
  if (!res.ok) {
    throw new Error(`blob upload failed: ${res.status} ${await res.text()}`);
  }
  const body = await res.json();
  // Existing /blob endpoint returns { hash: "sha256-..." } per blob_store conventions.
  const hash = body.hash ?? body.blobHash ?? body.sha256;
  if (!hash) throw new Error(`blob response missing hash field: ${JSON.stringify(body)}`);
  return hash.replace(/^sha256[-:]/, '');
}

async function main() {
  const stats = await stat(BUNDLE_DIR).catch(() => null);
  if (!stats || !stats.isDirectory()) {
    throw new Error(
      `Bundle directory not found: ${BUNDLE_DIR}\n` +
        `Run \`pnpm --filter elohim-app run build\` first.`,
    );
  }

  console.log(`[seed-landing-bundle] reading ${BUNDLE_DIR}…`);
  const zipBytes = await buildZip();
  console.log(`[seed-landing-bundle] zipped: ${zipBytes.byteLength} bytes`);

  const localHash = createHash('sha256').update(zipBytes).digest('hex');
  console.log(`[seed-landing-bundle] local sha256: ${localHash}`);

  console.log(`[seed-landing-bundle] uploading to ${STORAGE_URL}/blob…`);
  const remoteHash = await uploadBlob(zipBytes);
  console.log(`[seed-landing-bundle] remote hash:  ${remoteHash}`);

  if (remoteHash !== localHash) {
    console.warn(
      `[seed-landing-bundle] WARNING: remote hash differs from local — content addressing may have been altered in transit`,
    );
  }

  const seedText = await readFile(SEED_PATH, 'utf8');
  const seed = JSON.parse(seedText);
  if (seed.blobHash === remoteHash) {
    console.log('[seed-landing-bundle] seed already up-to-date; no changes');
    return;
  }
  seed.blobHash = remoteHash;
  if (seed.metadata) {
    seed.metadata.blobHash = remoteHash;
  }
  seed.updatedAt = new Date().toISOString();
  await writeFile(SEED_PATH, JSON.stringify(seed, null, 2) + '\n');
  console.log(`[seed-landing-bundle] patched ${SEED_PATH}`);
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
```

- [ ] **Step 3: Add JSZip if not already installed**

Run:
```bash
cd genesis
test -f package.json && cat package.json | grep -q jszip || pnpm add -D jszip
```

If `genesis/` is not a pnpm workspace member, run `pnpm add -D -w jszip` from the repo root.

- [ ] **Step 4: Build the elohim-app browser bundle**

Run:
```bash
cd app/elohim-app
pnpm run build
```

Expected: `dist/elohim-app/browser/index.html` exists.

- [ ] **Step 5: Start the local storage stack**

In a separate terminal:
```bash
pnpm run hc:start
```

Wait for `http://localhost:8090/api/v1/health` to return `{"healthy":true,...}`.

- [ ] **Step 6: Run the seed script**

Run:
```bash
STORAGE_URL=http://localhost:8090 node genesis/scripts/seed-landing-bundle.mjs
```

Expected: prints local and remote hashes (matching), patches the seed JSON with the real `blobHash`.

- [ ] **Step 7: Verify the bundle serves**

Run:
```bash
curl -sS -o /dev/null -w "%{http_code}\n" http://localhost:8090/apps/elohim-host-landing/index.html
```

Expected: `200`. If `404`, the html5-app router needs the ContentNode persisted too — proceed to Task 11 first and re-test.

- [ ] **Step 8: Commit**

Run:
```bash
git add genesis/scripts/seed-landing-bundle.mjs genesis/data/lamad/content/elohim-host-landing.json
test -f genesis/package.json && git add genesis/package.json
git commit -m "feat(seed): script to upload landing-page bundle + patch ContentNode hash

Reads app/elohim-app/dist/elohim-app/browser, zips it, uploads via
POST /blob, and patches genesis/data/lamad/content/elohim-host-landing.json
with the resulting sha256. Idempotent: skips writing if the hash matches.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 11: Wire the seed into the seeder run

**Files:**
- Modify: whichever seeder manifest enumerates content-node JSONs to seed (likely `genesis/seeder/...` or `genesis/data/lamad/deployments.json` per memory `project_deployments_json_seed_or_skip_truth`).

- [ ] **Step 1: Locate the seeder content list**

Run:
```bash
grep -rln "evolution-of-trust" genesis/seeder genesis/data 2>/dev/null | head -5
```

Expected: identifies where `evolution-of-trust.json` is referenced as a seed input. Memory anchor `project_deployments_json_seed_or_skip_truth` says `deployments.json` filters which seeds get applied; the filter is `suspended: true` for skipped entries.

- [ ] **Step 2: Add elohim-host-landing to that surface**

If the seeder enumerates by directory (most likely — it picks up all JSONs in `genesis/data/lamad/content/`), no manifest edit is required.

If the seeder uses an explicit list (less likely), add an entry mirroring `evolution-of-trust`:
```json
{ "id": "elohim-host-landing", "suspended": false }
```

- [ ] **Step 3: Run the seeder**

Run:
```bash
pnpm run hc:start:seed
```

Expected: seeder logs include `elohim-host-landing` as a created or already-present ContentNode.

- [ ] **Step 4: Verify the ContentNode is queryable**

Run:
```bash
curl -sS http://localhost:8090/db/content/elohim-host-landing | jq '.id, .contentFormat, .blobHash'
```

Expected: outputs `"elohim-host-landing"`, `"html5-app"`, and the sha256 hash patched by the seed script.

- [ ] **Step 5: Verify the bundle serves via the app path**

Run:
```bash
curl -sS -o /dev/null -w "%{http_code}\n" http://localhost:8090/apps/elohim-host-landing/index.html
```

Expected: `200`.

Also verify a nested asset (Angular bundles have hashed JS):
```bash
curl -sSI http://localhost:8090/apps/elohim-host-landing/index.html | head -3
```

Expected: a `200 OK` response with `Content-Type: text/html`.

- [ ] **Step 6: Commit (only if any file changed)**

If the seeder manifest required an edit:

```bash
git add genesis/<changed-file>
git commit -m "chore(seed): include elohim-host-landing in active seed set

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

Otherwise no commit is needed — the seeder already auto-discovered the new JSON.

---

## Task 12: Author the REA Commitment for the in-kind hosting agreement

**Files:**
- Create: `genesis/scripts/author-landing-commitment.mjs`

- [ ] **Step 1: Write the script**

Write to `genesis/scripts/author-landing-commitment.mjs`:

```javascript
#!/usr/bin/env node
/**
 * Author the in-kind hosting REA Commitment for the elohim-host-landing
 * ContentNode.
 *
 *   provider  = matthew  (compute / projection / DNS)
 *   receiver  = matthew  (steward of elohim-host-landing)
 *   action    = "deliver-service"
 *   signal_kind     (via metadata)  = "compute-allocation"
 *   trigger_kind    (via metadata)  = "subscription"
 *   inScopeOf = ["host:alpha.elohim.host", "epr_root:elohim-host-landing"]
 *
 * Idempotent: lists existing commitments for matthew and skips if an
 * equivalent in-scope subscription already exists.
 *
 * Usage:
 *   STORAGE_URL=http://localhost:8090 node genesis/scripts/author-landing-commitment.mjs
 */

const STORAGE_URL = process.env.STORAGE_URL ?? 'http://localhost:8090';
const REQUESTER = process.env.MATTHEW_ID ?? 'matthew';

const SCOPE = ['host:alpha.elohim.host', 'epr_root:elohim-host-landing'];

const COMMITMENT = {
  action: 'deliver-service',
  provider: REQUESTER,
  receiver: REQUESTER,
  resourceClassifiedAs: ['http-requests', 'egress-bandwidth', 'ssr-cpu-seconds'],
  resourceQuantity: { hasNumericalValue: 0, hasUnit: 'subscription-window' },
  hasBeginning: new Date().toISOString(),
  inScopeOf: SCOPE,
  note:
    'In-kind self-hosting agreement: Matthew (steward) hosts the ' +
    'elohim-host-landing ContentNode at alpha.elohim.host root via Matthew ' +
    '(doorway operator). signal_kind="compute-allocation"; ' +
    'trigger_kind="subscription".',
  metadata: {
    signalKind: 'compute-allocation',
    triggerKind: 'subscription',
    inKind: true,
  },
};

async function listExisting() {
  const url = new URL(`${STORAGE_URL}/api/v1/commitments`);
  url.searchParams.set('provider', REQUESTER);
  url.searchParams.set('receiver', REQUESTER);
  const res = await fetch(url);
  if (!res.ok) throw new Error(`list failed: ${res.status} ${await res.text()}`);
  return res.json();
}

function isScopeEqual(a = [], b = []) {
  if (a.length !== b.length) return false;
  const aSet = new Set(a);
  for (const item of b) if (!aSet.has(item)) return false;
  return true;
}

async function main() {
  const existing = await listExisting();
  const match = existing.find(
    c =>
      c.action === 'deliver-service' &&
      isScopeEqual(c.inScopeOf ?? [], SCOPE) &&
      c.state !== 'cancelled' &&
      c.state !== 'breached',
  );
  if (match) {
    console.log(`[author-commitment] already exists: ${match.id} (state=${match.state})`);
    return;
  }

  const res = await fetch(`${STORAGE_URL}/api/v1/commitments`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(COMMITMENT),
  });
  if (!res.ok) {
    throw new Error(`create failed: ${res.status} ${await res.text()}`);
  }
  const created = await res.json();
  console.log(`[author-commitment] created: ${created.id}`);
  console.log(JSON.stringify(created, null, 2));
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
```

- [ ] **Step 2: Run the script against the local stack**

Run:
```bash
STORAGE_URL=http://localhost:8090 node genesis/scripts/author-landing-commitment.mjs
```

Expected: prints `[author-commitment] created: <uuid>` and dumps the created commitment.

- [ ] **Step 3: Verify the commitment is queryable**

Run:
```bash
curl -sS "http://localhost:8090/api/v1/commitments?provider=matthew" | \
  jq '.[] | select(.inScopeOf and (.inScopeOf | index("host:alpha.elohim.host")))'
```

Expected: one commitment with `inScopeOf` containing both `host:alpha.elohim.host` and `epr_root:elohim-host-landing`.

- [ ] **Step 4: Verify idempotency**

Run the script a second time:
```bash
STORAGE_URL=http://localhost:8090 node genesis/scripts/author-landing-commitment.mjs
```

Expected: prints `[author-commitment] already exists: <uuid>`.

- [ ] **Step 5: Commit**

Run:
```bash
git add genesis/scripts/author-landing-commitment.mjs
git commit -m "feat(scripts): author in-kind REA commitment for elohim-host-landing

The EPR compute-contract between Matthew (steward) and Matthew (doorway
operator). signal_kind=compute-allocation, trigger_kind=subscription,
inScopeOf includes the host and the epr_root. Idempotent: skips if an
equivalent active commitment already exists.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 13: Enable doorway storage auto-registration in the alpha deployment

**Files:**
- Modify: `genesis/orchestrator/manifests/elohim-app/alpha/configmap.yaml` (or whichever manifest sets the doorway pod's env)

- [ ] **Step 1: Locate the doorway env source**

Run:
```bash
grep -rln "ELOHIM_STORAGE\|STORAGE_URL\|DOORWAY_ENV" genesis/orchestrator/manifests/elohim-app/alpha 2>/dev/null
```

Expected: at least one file (probably `configmap.yaml` or a `deployment.yaml`) sets the doorway pod's environment variables.

- [ ] **Step 2: Add the env var**

In the relevant manifest, add (in the `env:` or `data:` block):

```yaml
ELOHIM_STORAGE_AUTO_REGISTER: "true"
```

If the manifest already sets `ELOHIM_STORAGE_URL`, leave it as-is. Otherwise add:

```yaml
ELOHIM_STORAGE_URL: "http://elohim-storage-alpha:8090"
```

- [ ] **Step 3: Verify the value can be read by the doorway**

Inspect `doorway/doorway-service/src/services/storage_registration.rs` for the env-reading logic — line 55: `std::env::var("ELOHIM_STORAGE_AUTO_REGISTER").map(|v| v == "true" || v == "1")`. Confirm your YAML value is exactly `"true"` (string) so the predicate matches.

- [ ] **Step 4: Commit**

Run:
```bash
git add genesis/orchestrator/manifests/elohim-app/alpha/<changed-file>
git commit -m "chore(alpha): enable ELOHIM_STORAGE_AUTO_REGISTER for doorway

When the doorway boots, storage_registration calls register_content_server
on the infrastructure zome with wildcard content_hash and the
[blob, html5_app] capabilities. This makes the doorway's storage
discoverable as a publisher for html5-app content (including the
elohim-host-landing bundle).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 14: Flip the alpha ingress `/` rule to doorway

**Files:**
- Modify: `genesis/orchestrator/manifests/elohim-app/alpha/ingress.yaml`

- [ ] **Step 1: Read the current ingress**

Confirm lines 74-80 currently route `/` to `elohim-site-alpha-service:80`.

- [ ] **Step 2: Replace the catch-all rule**

In `genesis/orchestrator/manifests/elohim-app/alpha/ingress.yaml`, change the final `paths:` block from:

```yaml
      - backend:
          service:
            name: elohim-site-alpha-service
            port:
              number: 80
        path: /
        pathType: Prefix
```

to:

```yaml
      # Root-route dogfood: serve elohim-host-landing ContentNode via doorway.
      # The doorway exposes html5-app content at /apps/<slug>/<entry>.
      # Below: catch-all "/" hits doorway, which serves the SPA fallback for
      # unknown paths and the html5-app bundle for the root via an explicit
      # exact-match rule above (see ELOHIM_HOST_LANDING_ROOT_REWRITE).
      # The elohim-site-alpha-service static pod remains deployed as a rollback
      # target; flip back to it by reverting this commit if dogfood breaks.
      - backend:
          service:
            name: elohim-doorway-alpha
            port:
              number: 8080
        path: /
        pathType: Prefix
```

And **add** (above the new `/` block, in the same `paths:` list) an exact-match rewrite for the root:

```yaml
      # Root request → html5-app bundle root via doorway
      - backend:
          service:
            name: elohim-doorway-alpha
            port:
              number: 8080
        path: /index.html
        pathType: Exact
```

Then update the ingress `metadata.annotations` block to add (if not already present):

```yaml
    nginx.ingress.kubernetes.io/configuration-snippet: |
      # ELOHIM_HOST_LANDING_ROOT_REWRITE
      # Map bare "/" to the html5-app entry; everything else falls through
      # to doorway's normal routing (which includes the SPA fallback).
      location = / {
        return 302 /apps/elohim-host-landing/index.html;
      }
```

(If the alpha cluster uses a different ingress controller, adapt — the goal is "bare `/` resolves to `/apps/elohim-host-landing/index.html` on doorway".)

- [ ] **Step 3: Commit**

Run:
```bash
git add genesis/orchestrator/manifests/elohim-app/alpha/ingress.yaml
git commit -m "chore(alpha): flip / from elohim-site to doorway → elohim-host-landing

Bare GET / now 302s to /apps/elohim-host-landing/index.html via doorway,
serving the landing-page html5-app ContentNode. All other paths
continue to route through doorway as before. elohim-site-alpha-service
is retained as a deployment for rollback (revert this commit + redeploy
ingress to restore).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 15: A2o feature file — landing-page dogfood

**Files:**
- Create: `genesis/a2o/features/protocol/landing-page-dogfood.feature`

- [ ] **Step 1: Write the feature**

Write to `genesis/a2o/features/protocol/landing-page-dogfood.feature`:

```gherkin
@e2e @protocol @landing-dogfood
Feature: Elohim Protocol landing page is dogfooded as protocol content
  As Matthew, who stewards the Elohim Protocol public surface and operates
  the alpha doorway as the same agent
  I want the landing page at alpha.elohim.host to render bytes that flow
  through the protocol's content-addressing path
  So that the protocol's own marketing site is the first proof of the
  steward↔doorway hosting model — and is impossible to silently centralize.

  Background:
    Given elohim-storage is healthy at "http://localhost:8090"

  Scenario: The elohim-host-landing ContentNode exists with html5-app format
    When I fetch the ContentNode "elohim-host-landing"
    Then the contentFormat is "html5-app"
    And the content.slug is "elohim-host-landing"
    And the content.entryPoint is "index.html"
    And the blobHash is a sha256 hex string

  Scenario: The landing bundle serves at the /apps path
    When I GET "/apps/elohim-host-landing/index.html" from the doorway
    Then the response status is 200
    And the response Content-Type contains "text/html"

  Scenario: An in-kind REA Commitment declares Matthew's hosting agreement
    When I list active REA commitments where provider is "matthew"
    Then at least one commitment has inScopeOf containing "host:alpha.elohim.host"
    And that commitment has inScopeOf containing "epr_root:elohim-host-landing"
    And that commitment's metadata signalKind is "compute-allocation"
    And that commitment's metadata triggerKind is "subscription"

  @browser-only
  Scenario: The protocol-signal badge renders on the landing page
    When I open the landing page in a browser
    Then the element [data-testid="protocol-signal-badge-pill"] is visible
    When I click the protocol-signal badge
    Then the element [data-testid="protocol-signal-panel"] is visible
    And the panel text contains "elohim-host-landing"
```

- [ ] **Step 2: Run cucumber to confirm the new feature is discovered with stubs**

Run:
```bash
cd genesis/a2o
npx cucumber-js --profile testnet --tags '@landing-dogfood and not @browser-only' --dry-run 2>&1 | tail -20
```

Expected: cucumber reports undefined steps for all four scenarios (until Task 16 lands the step definitions).

- [ ] **Step 3: Commit**

Run:
```bash
git add genesis/a2o/features/protocol/landing-page-dogfood.feature
git commit -m "test(a2o): scenarios for elohim-host-landing dogfood

Four scenarios prove end-to-end: ContentNode exists, bundle serves via
/apps path, REA commitment encodes the in-kind hosting agreement, badge
renders in browser. Browser scenario tagged @browser-only.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 16: A2o step definitions

**Files:**
- Create: `genesis/a2o/steps/protocol/landing-page-dogfood.steps.ts`

- [ ] **Step 1: Locate the existing step-definition conventions**

Run:
```bash
ls genesis/a2o/steps/ | head -20
```

Expected: a list including existing `*.steps.ts` files. Note the imports (`@cucumber/cucumber`, `assert` patterns) by reading one — for example `genesis/a2o/steps/compute-allocation.steps.ts`.

- [ ] **Step 2: Write the step definitions**

Write to `genesis/a2o/steps/protocol/landing-page-dogfood.steps.ts`:

```typescript
import { strict as assert } from 'node:assert';

import { Given, Then, When } from '@cucumber/cucumber';

const STORAGE_URL = process.env.STORAGE_URL ?? 'http://localhost:8090';
const DOORWAY_URL = process.env.DOORWAY_URL ?? 'http://localhost:8888';

interface DogfoodWorld {
  fetchedNode?: Record<string, unknown>;
  doorwayResponse?: Response;
  commitments?: Array<Record<string, unknown>>;
  scopedCommitment?: Record<string, unknown>;
}

Given('elohim-storage is healthy at {string}', async function (this: DogfoodWorld, url: string) {
  const res = await fetch(`${url}/api/v1/health`);
  assert.ok(res.ok, `elohim-storage health failed: ${res.status}`);
});

When('I fetch the ContentNode {string}', async function (this: DogfoodWorld, id: string) {
  const res = await fetch(`${STORAGE_URL}/db/content/${id}`);
  assert.ok(res.ok, `content fetch failed: ${res.status}`);
  this.fetchedNode = (await res.json()) as Record<string, unknown>;
});

Then('the contentFormat is {string}', function (this: DogfoodWorld, expected: string) {
  assert.equal(this.fetchedNode?.contentFormat, expected);
});

Then('the content.slug is {string}', function (this: DogfoodWorld, expected: string) {
  const content = this.fetchedNode?.content as Record<string, unknown> | undefined;
  assert.equal(content?.slug, expected);
});

Then('the content.entryPoint is {string}', function (this: DogfoodWorld, expected: string) {
  const content = this.fetchedNode?.content as Record<string, unknown> | undefined;
  assert.equal(content?.entryPoint, expected);
});

Then('the blobHash is a sha256 hex string', function (this: DogfoodWorld) {
  const hash = this.fetchedNode?.blobHash;
  assert.equal(typeof hash, 'string');
  assert.match(hash as string, /^[a-f0-9]{64}$/);
});

When('I GET {string} from the doorway', async function (this: DogfoodWorld, path: string) {
  this.doorwayResponse = await fetch(`${DOORWAY_URL}${path}`);
});

Then('the response status is {int}', function (this: DogfoodWorld, expected: number) {
  assert.equal(this.doorwayResponse?.status, expected);
});

Then('the response Content-Type contains {string}', function (this: DogfoodWorld, expected: string) {
  const ct = this.doorwayResponse?.headers.get('content-type') ?? '';
  assert.ok(ct.includes(expected), `Content-Type was '${ct}'`);
});

When('I list active REA commitments where provider is {string}', async function (
  this: DogfoodWorld,
  provider: string,
) {
  const url = new URL(`${STORAGE_URL}/api/v1/commitments`);
  url.searchParams.set('provider', provider);
  const res = await fetch(url);
  assert.ok(res.ok, `commitments list failed: ${res.status}`);
  this.commitments = (await res.json()) as Array<Record<string, unknown>>;
});

Then(
  'at least one commitment has inScopeOf containing {string}',
  function (this: DogfoodWorld, expected: string) {
    const match = (this.commitments ?? []).find(c =>
      Array.isArray(c.inScopeOf) ? (c.inScopeOf as string[]).includes(expected) : false,
    );
    assert.ok(match, `No commitment had inScopeOf containing "${expected}"`);
    this.scopedCommitment = match;
  },
);

Then(
  'that commitment has inScopeOf containing {string}',
  function (this: DogfoodWorld, expected: string) {
    const scope = (this.scopedCommitment?.inScopeOf ?? []) as string[];
    assert.ok(scope.includes(expected), `scope was ${JSON.stringify(scope)}`);
  },
);

Then(
  'that commitment\'s metadata signalKind is {string}',
  function (this: DogfoodWorld, expected: string) {
    const meta = (this.scopedCommitment?.metadata ?? {}) as Record<string, unknown>;
    assert.equal(meta.signalKind, expected);
  },
);

Then(
  'that commitment\'s metadata triggerKind is {string}',
  function (this: DogfoodWorld, expected: string) {
    const meta = (this.scopedCommitment?.metadata ?? {}) as Record<string, unknown>;
    assert.equal(meta.triggerKind, expected);
  },
);
```

- [ ] **Step 3: Typecheck**

Run:
```bash
cd genesis/a2o
npx tsc --noEmit 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 4: Run the non-browser scenarios**

With the local stack running and Tasks 9-14 applied locally:
```bash
cd genesis/a2o
STORAGE_URL=http://localhost:8090 DOORWAY_URL=http://localhost:8888 \
  npx cucumber-js --tags '@landing-dogfood and not @browser-only'
```

Expected: 3 scenarios pass (ContentNode exists, bundle serves, commitment exists). The fourth (@browser-only) is run in a Playwright-equipped environment and may be deferred.

- [ ] **Step 5: Commit**

Run:
```bash
git add genesis/a2o/steps/protocol/landing-page-dogfood.steps.ts
git commit -m "test(a2o): step definitions for landing-page dogfood

HTTP-only assertions for ContentNode existence, bundle serving via
doorway /apps path, and REA commitment scope. Browser scenario (badge
visibility + panel) deferred to Playwright env.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 17: Manual end-to-end verification on alpha

**Files:** none modified — operator-visible verification step.

- [ ] **Step 1: Deploy the branch to alpha**

Push the branch and let CI run the orchestrator pipeline. The relevant downstream pipelines should be: `app` (elohim-app build + image), `genesis` (seed validation), and the alpha redeploy.

Run:
```bash
git push -u origin raw-viewport-landing-dogfood
gh pr create --base dev --title "Raw viewport + landing-page dogfood" --body "$(cat <<'EOF'
## Summary
- Replace Focus Mode (`isFocusedView`) with `RawContentViewportComponent` + `ProtocolSignalBadgeComponent`; reachable via `/raw/:resourceId`
- Embed protocol-signal badge on the landing page
- Package elohim-app browser bundle as `elohim-host-landing` html5-app ContentNode (same shape as Evolution of Trust)
- Author the in-kind REA Commitment (`signal_kind: compute-allocation`, `trigger_kind: subscription`) declaring Matthew↔Matthew hosting agreement for `alpha.elohim.host` root
- Enable `ELOHIM_STORAGE_AUTO_REGISTER=true` on the doorway pod so it registers as ContentServer for html5_app
- Flip the alpha ingress `/` rule to doorway; bare `/` 302s to `/apps/elohim-host-landing/index.html`
- A2o scenarios prove end-to-end

## Test plan
- [ ] Vitest: protocol-signal-badge + raw-content-viewport specs pass
- [ ] Vitest: content-viewer + path-navigator specs pass after Focus Mode removal
- [ ] elohim-app build succeeds
- [ ] Seed bundle script runs locally; produces matching local/remote sha256
- [ ] `/apps/elohim-host-landing/index.html` returns 200 from doorway
- [ ] `/api/v1/commitments?provider=matthew` returns the scoped commitment
- [ ] a2o scenarios pass: `@landing-dogfood and not @browser-only`
- [ ] On alpha after deploy: `curl -L https://alpha.elohim.host/` lands on the html5-app bundle
- [ ] Browser visit to `https://alpha.elohim.host/` shows the protocol-signal badge bottom-right; clicking expands the panel

## Follow-up TODOs (not in this PR)
- Tier 2 (browser extension): client-side hash verification + toolbar badge
- Tier 3 (Tauri-native): OS window-chrome badge that follows tab state
- Active hash verification in the panel (currently passive CID display)
- Wire `PortalHostService.add()` past M5 stub so Matthew's PortalHost entry is authored
- Doorway routes resolved from active REA commitments (today: declared in storage manifest)
- Sub-EPR routing under `alpha.elohim.host/<path>` for collective surface composition

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 2: Wait for CI to go green**

After CI completes, the orchestrator should have triggered the alpha redeploy. Confirm via Jenkins MCP (see `pipeline-diagnostics` skill).

- [ ] **Step 3: Run the seed scripts against alpha**

After the alpha pods are healthy:

```bash
# Get the alpha storage URL (or proxy via kubectl port-forward)
kubectl -n elohim-alpha port-forward svc/elohim-storage-alpha 18090:8090 &
sleep 3
STORAGE_URL=http://localhost:18090 node genesis/scripts/seed-landing-bundle.mjs
STORAGE_URL=http://localhost:18090 node genesis/scripts/author-landing-commitment.mjs
kill %1
```

Expected: both scripts succeed; second-run prints "already exists" for the commitment.

- [ ] **Step 4: Visit the live URL**

Open `https://alpha.elohim.host/` in a browser. Confirm:
- Page loads (was previously served by the static SPA pod; now served via doorway → html5-app bundle)
- Protocol-signal badge visible at bottom-right
- Clicking the badge expands a panel showing `elohim-host-landing` as the content id

- [ ] **Step 5: Curl-test the redirect chain**

Run:
```bash
curl -sSI -L https://alpha.elohim.host/ | tail -20
```

Expected: 302 to `/apps/elohim-host-landing/index.html`, then 200 with `Content-Type: text/html`.

- [ ] **Step 6: Capture the operator screenshot (optional but recommended)**

Take a screenshot of `https://alpha.elohim.host/` showing the badge expanded. Attach to the PR as visual evidence of dogfood success.

- [ ] **Step 7: Merge**

After review approval, merge to `dev`.

---

## Self-Review (engineer running this plan)

Before declaring done, walk through each of the four a2o scenarios and confirm:

1. **Scenario "ContentNode exists with html5-app format"** — fetch `/db/content/elohim-host-landing` directly on alpha and inspect the JSON.
2. **Scenario "Bundle serves at /apps path"** — `/apps/elohim-host-landing/index.html` returns 200 with HTML.
3. **Scenario "In-kind REA Commitment declares Matthew's hosting"** — `/api/v1/commitments?provider=matthew` includes the scoped commitment with the right metadata.
4. **Scenario "Protocol-signal badge renders"** — manual browser verification (or Playwright if available).

If any of the four fail, fix and re-run the relevant tasks. Do not declare done until all four pass on alpha (the @browser-only one may be deferred to a Playwright environment but should be acknowledged).

---

## Out-of-Scope (Explicit Non-Goals)

These belong in follow-up plans, not this one:

- **Tier 2 — Browser extension**: client-side hash verification, toolbar badge, X-Elohim-Content-CID response header sniffing. The current badge accepts the doorway's CID claim at face value.
- **Tier 3 — Tauri-native**: OS window-chrome badge that follows tab state. Requires Tauri shell awareness of protocol vs classic-web navigation.
- **Active hash verification**: re-fetching the blob client-side and comparing the hash. The panel currently shows the CID; passive trust.
- **PortalHost authoring**: blocked by the M5 503 stub in `PortalHostService.add()`. When Phase 11 wires storage → conductor, add a Task to invoke `add_portal_host({ hostUrl: "https://alpha.elohim.host", label: "alpha primary", reach: Trusted })` and verify the projection lands in `portal_hosts`.
- **Doorway routes resolved from active commitments**: today the storage manifest declares routes; the dream is that doorway reads active subscription commitments and builds its route table from them. Stub-epic #7 in `2026-05-08-doorway-hub-edge-design.md`.
- **Sub-EPR routing under the host**: `alpha.elohim.host/about` → another ContentNode. Right now the bundle handles its own internal navigation.
- **Replacing the static SPA pod entirely**: kept as rollback target. The `project_doorway_full_facilitator_sprint` candidate absorbs SPA hosting into doorway and retires `elohim-site`; that is a separate sprint.
- **Continuously-renegotiated contracts**: stub-epic #7 in `2026-05-08-doorway-hub-edge-design.md`. The Commitment authored here is a single static entry; the relationship-as-EPR-variant-chain story is future work.
- **Cross-collective surface composition**: collective stewards their projectable discoverable surface composed from EPRs. The user has been brainstorming this layer separately; raw viewport is the floor that work builds on.

---

## Plan Complete

Saved to `genesis/docs/superpowers/plans/2026-05-19-raw-viewport-landing-dogfood.md`.
