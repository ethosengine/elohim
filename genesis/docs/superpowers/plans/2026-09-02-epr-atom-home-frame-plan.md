---
id: epr-atom-home-frame-plan
status: active
cites:
  - "epr-atom-home-shell-component | EPR Atom Home | sha256:97bcb3c9d81b741a | path: genesis/docs/superpowers/specs/2026-09-02-epr-atom-home-shell-component-design.md"
  - "pillar-epr-decomposition-design | Pillar EPR Decomposition | sha256:3db7d2c205a0d7d6 | path: genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md"
---

# EPR Atom Home — Slices 0 + 1 (brand foundation + the frame) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace lamad's ContentViewer at `/epr/:resourceId` with a shell-owned `EprHomeComponent` that renders the atom's frame — arrival, identity, focal render shaped by format, the four legs, the address line, and the designed out-of-reach gate — with zero new cross-workspace imports.

**Architecture:** The renderer host that `content-delivery` already carries (the only lamad renderer references in the shell) is extracted into a shell-owned `EprFocalComponent` so the lamad reference counts do not rise. `EprHomeComponent` reads the raw `ContentView` via the shell's `StorageClientService` for the frame, hands only the slug to the focal, and composes the legs from `@elohim/service` and shell services. Brand tokens enter the shell as a scoped `--el-*` sheet. The commons layer (conversation, statements, who's here, how we talk here) is a separate plan.

**Tech Stack:** Angular 19 standalone + signals + OnPush, vitest (`pnpm exec vitest run --config vite.config.ts <pattern>` from `app/elohim-app`), `@elohim/service` (workspace lib), `@fontsource-variable/*`, Cucumber + Playwright a2o (`genesis/a2o`).

**Spec:** `genesis/docs/superpowers/specs/2026-09-02-epr-atom-home-shell-component-design.md`

## Global Constraints

- **Import ratchet:** `node ../scripts/lint-workspace-imports.mjs .` (run from `app/elohim-app`) must pass with the baseline UNCHANGED. No new `@app/lamad/*` specifier, no count increase. Only `epr-focal.component.ts` and `epr-focal.component.spec.ts` may import from `@app/lamad/*`, and only what `content-delivery.component.ts` / its spec import today (`models/content-node.model`, `renderers/renderer-initializer.service`, `renderers/renderer-registry.service`, `services/content.service`).
- **Route literals:** `node ../scripts/lint-route-literals.mjs src` refuses pillar-prefixed literals; the one mount-table literal carries a `// route-literal-ok:` trailing comment with its reason (Task 6).
- **Test ids** are the spec §6 names, exactly: `epr-home`, `epr-home-arrival`, `epr-home-title`, `epr-home-chip-reach`, `epr-home-chip-notarized`, `epr-home-chip-held`, `epr-home-open-in-bundle`, `epr-home-focal`, `epr-home-leg-holds`, `epr-home-leg-lives`, `epr-home-leg-governed`, `epr-home-leg-from`, `epr-home-gate`, `epr-home-address`, `epr-home-your-mark`. Retained: `viewer-relationships-panel` (wrapper around `app-epr-relationships-panel`).
- **Copy rules (brand voice):** "household", never "user"; no percentages for trust; "Notarized · anchor not yet verified here" wording verbatim; no all-caps; Terracotta (`--el-clay`) for "needs help", never red.
- **Change detection:** every new component is `ChangeDetectionStrategy.OnPush` with signal/`toSignal` state. Async-loaded state must be a signal, never a mutated plain field (the implicit-OnPush freeze).
- **Commits:** path-limited `git add` of exactly the listed files (shared worktree). Never `git add -A`. Never push. Commit trailer:
  ```
  Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_017GZH6i7cHvKanCC7R32jFh
  ```
- **Verification honesty:** in-container vitest misses AOT template errors; Task 8 runs a real `ng build` before the render proof.

---

## File structure

| File | Responsibility |
|---|---|
| `app/elohim-app/src/styles/brand.css` (create) | `--el-*` palette + semantic tokens + the four brand faces (fontsource). Consumed by binding only. |
| `app/elohim-app/src/styles.css` (modify, line 2) | one `@import './styles/brand.css';` after the maplibre import |
| `app/elohim-app/package.json` (modify) | four `@fontsource-variable/*` deps |
| `app/elohim-app/src/app/elohim/components/epr-focal/epr-focal.component.{ts,html,css,spec.ts}` (create) | the renderer host: slug in, node out; the ONLY shell files touching lamad renderers |
| `app/elohim-app/src/app/elohim/components/content-delivery/content-delivery.component.{ts,html,spec.ts}` (modify) | composes `<app-epr-focal>`; drops its lamad imports |
| `app/elohim-app/src/app/elohim/components/epr-home/epr-home.model.ts` (create) | `EprHomeAtom` view model, `toAtom()`, `focalShape()`, chip label maps, `StewardRow` |
| `app/elohim-app/src/app/elohim/components/epr-home/epr-home.component.{ts,html,css,spec.ts}` (create) | the frame: route param → atom; states; identity; focal; gate; address line; arrival; your mark; leg data loading |
| `app/elohim-app/src/app/elohim/components/epr-home/epr-home-legs.component.{ts,html,css,spec.ts}` (create) | presentational rail: the four legs from inputs |
| `app/elohim-app/src/app/elohim/components/epr-home/bundle-lens.ts` (create) | `openInBundle(atom)` from generated `BUNDLE_ROUTE_CLAIMS` + mount table |
| `app/elohim-app/src/app/app.routes.ts` (modify, lines 72-78) | `epr/:resourceId` → `EprHomeComponent` |
| `elohim/sdk/schemas/scripts/codegen-route-claims.mjs` (modify) + `app/elohim-app/src/app/generated/route-claims.ts` (regenerate) | `BUNDLE_ROUTE_CLAIMS` for the universal-route owner |
| `genesis/a2o/src/framework/pages/epr-home.page.ts` (create), `genesis/a2o/steps/ui/epr-atom-home.steps.ts` (create), `genesis/a2o/features/content/epr-atom-home.feature` (modify: drop `@wip` on scenarios 1–6, 10) | the concern's steps |
| `app/elohim-app/.epr-meta/epr-atom-home.habit.md` (modify) | evidence delta |

---

### Task 1: Brand foundation (Slice 0)

**Files:**
- Create: `app/elohim-app/src/styles/brand.css`
- Modify: `app/elohim-app/src/styles.css:2`
- Modify: `app/elohim-app/package.json` (dependencies)

**Interfaces:**
- Produces: CSS custom properties `--el-cream --el-starlight --el-stone --el-ink --el-green-deep --el-green-light --el-amber --el-clay --el-sky --el-plum --el-night --el-night-alt --el-border --el-surface-2 --el-ground --el-surface --el-text --el-text-strong --el-action --el-on-action --el-font-display --el-font-body --el-font-ui --el-font-mono` on `:root`, with `:root[data-theme='light']` overrides (the shell's default theme is dark; the light block is what the design canvas shows).

- [ ] **Step 1: Add the font packages**

Run from `app/elohim-app`:
```bash
pnpm add @fontsource-variable/fraunces@^5.2.9 @fontsource-variable/source-serif-4@^5.2.9 @fontsource-variable/dm-sans@^5.2.8 @fontsource-variable/jetbrains-mono@^5.2.8
ls node_modules/@fontsource-variable/fraunces/ | head -20
```
Expected: `package.json` gains the four deps (versions already resolved in the workspace lockfile for `elohim-library`); the listing shows `index.css` and, for fraunces and source-serif-4, `full.css` / `full-italic.css` (the multi-axis files). If `full.css` is absent for a package, use that package's `index.css` in Step 2 and say so in the commit body.

- [ ] **Step 2: Write brand.css**

`app/elohim-app/src/styles/brand.css`:
```css
/* Elohim brand tokens — genesis/graphos/elohim-protocol-design-spec.md §4 Color, §5 Type.
   SCOPED BY CONSUMPTION: nothing in the shell binds these until a surface opts in by
   using --el-* (the EPR atom home is the first). The shell's --lamad-* tokens are
   untouched. Rules carried: no pure black/white; darks carry green/brown undertone;
   dark mode is constellation mode (starlight on deep sky). */

@import '@fontsource-variable/fraunces/full.css';
@import '@fontsource-variable/fraunces/full-italic.css';
@import '@fontsource-variable/source-serif-4/full.css';
@import '@fontsource-variable/source-serif-4/full-italic.css';
@import '@fontsource-variable/dm-sans';
@import '@fontsource-variable/jetbrains-mono';

:root {
  /* Primary palette */
  --el-cream: #f5f0e8; /* Linen */
  --el-starlight: #e8e4d9; /* Starlight */
  --el-stone: #6b6157; /* Hearthstone */
  --el-ink: #26302a; /* headline dark with a green undertone (never #000) */
  --el-green-deep: #2d5f3b; /* Vineyard */
  --el-green-light: #7fb069; /* New Growth */
  --el-amber: #d4a03e; /* Harvest Gold */
  --el-clay: #b8664f; /* Terracotta */
  --el-sky: #7bafcb; /* Morning */
  --el-plum: #6e4b6b; /* Sabbath */
  --el-night: #0f1a12; /* Deep Sky */
  --el-night-alt: #1a1a2e; /* Indigo Night */

  /* Faces (fontsource-variable family names) */
  --el-font-display: 'Fraunces Variable', Georgia, serif;
  --el-font-body: 'Source Serif 4 Variable', Georgia, serif;
  --el-font-ui: 'DM Sans Variable', system-ui, sans-serif;
  --el-font-mono: 'JetBrains Mono Variable', ui-monospace, monospace;

  /* Semantic — DARK is the shell default (constellation mode) */
  --el-ground: var(--el-night);
  --el-surface: var(--el-night-alt);
  --el-surface-2: rgba(232, 228, 217, 0.08);
  --el-border: rgba(232, 228, 217, 0.18);
  --el-text: var(--el-starlight);
  --el-text-strong: #f3efe6;
  --el-text-muted: rgba(232, 228, 217, 0.72);
  --el-action: var(--el-amber);
  --el-on-action: var(--el-night);
  --el-warm: #d98a72; /* terracotta lifted for dark ground */
}

:root[data-theme='light'] {
  --el-ground: var(--el-cream);
  --el-surface: var(--el-starlight);
  --el-surface-2: rgba(107, 97, 87, 0.08);
  --el-border: rgba(107, 97, 87, 0.18);
  --el-text: var(--el-stone);
  --el-text-strong: var(--el-ink);
  --el-text-muted: var(--el-stone);
  --el-action: var(--el-green-deep);
  --el-on-action: var(--el-cream);
  --el-warm: #8f4a36; /* terracotta deepened for linen ground */
}
```

- [ ] **Step 3: Import it from styles.css**

`app/elohim-app/src/styles.css` — after line 2 (`@import 'maplibre-gl/dist/maplibre-gl.css';`) add:
```css
@import './styles/brand.css';
```
(CSS `@import` rules must precede all other rules; both imports are at the top.)

- [ ] **Step 4: Verify the build resolves the imports**

Run from `app/elohim-app`:
```bash
pnpm exec ng build --configuration development 2>&1 | tail -15; echo "EXIT=$?"
```
Expected: `EXIT=0`, no "Can't resolve '@fontsource-variable/…'" errors. (Development configuration is enough here; Task 8 does the production build.)

- [ ] **Step 5: Commit**

```bash
git add app/elohim-app/package.json pnpm-lock.yaml app/elohim-app/src/styles/brand.css app/elohim-app/src/styles.css
git commit -m "feat(shell): brand tokens + faces enter the shell as a scoped --el-* sheet (Slice 0 of epr-atom-home)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_017GZH6i7cHvKanCC7R32jFh"
```

---

### Task 2: Extract `EprFocalComponent` from content-delivery (count-neutral)

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/epr-focal/epr-focal.component.ts`
- Create: `app/elohim-app/src/app/elohim/components/epr-focal/epr-focal.component.html`
- Create: `app/elohim-app/src/app/elohim/components/epr-focal/epr-focal.component.css`
- Create: `app/elohim-app/src/app/elohim/components/epr-focal/epr-focal.component.spec.ts`
- Modify: `app/elohim-app/src/app/elohim/components/content-delivery/content-delivery.component.ts`
- Modify: `app/elohim-app/src/app/elohim/components/content-delivery/content-delivery.component.html`
- Modify: `app/elohim-app/src/app/elohim/components/content-delivery/content-delivery.component.spec.ts`

**Interfaces:**
- Produces: `EprFocalComponent` (selector `app-epr-focal`) with `@Input({ required: true }) slug: string`, `@Output() nodeLoaded: EventEmitter<FocalNode>`, `@Output() notFound: EventEmitter<string>`, `@Output() failed: EventEmitter<string>`; exported `type FocalNode` (alias of lamad's `ContentNode` — consumers import the alias from this file, never the lamad model).
- Consumes: lamad `ContentService.getContentBySlug(slug): Observable<ContentNode | null>`, `RendererRegistryService.getRenderer(node)`, `RendererInitializerService` (side-effect registration).

- [ ] **Step 1: Write the failing focal spec**

`app/elohim-app/src/app/elohim/components/epr-focal/epr-focal.component.spec.ts`:
```ts
/**
 * EprFocalComponent — the renderer host extracted from content-delivery.
 * Pins: slug → load → nodeLoaded; null → notFound; error → failed; a registered
 * renderer is created with the node as input; no renderer → format fallback.
 */
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';
import { describe, expect, it, vi, beforeEach } from 'vitest';

import { ContentService } from '@app/lamad/services/content.service';
import { RendererRegistryService } from '@app/lamad/renderers/renderer-registry.service';

import { EprFocalComponent } from './epr-focal.component';

const mockNode = {
  id: 'manifesto',
  title: 'The Elohim Protocol Manifesto',
  description: 'Founding document',
  contentType: 'epic',
  contentFormat: 'plaintext',
  content: 'plain body',
  reach: 'commons',
  stewardedBy: [],
  tags: [],
  relatedNodeIds: [],
  metadata: {},
};

describe('EprFocalComponent', () => {
  let fixture: ComponentFixture<EprFocalComponent>;
  let contentServiceSpy: { getContentBySlug: ReturnType<typeof vi.fn> };
  let registrySpy: { getRenderer: ReturnType<typeof vi.fn>; register: ReturnType<typeof vi.fn> };

  beforeEach(async () => {
    contentServiceSpy = { getContentBySlug: vi.fn().mockReturnValue(of(mockNode)) };
    registrySpy = { getRenderer: vi.fn().mockReturnValue(null), register: vi.fn() };
    await TestBed.configureTestingModule({
      imports: [EprFocalComponent],
      providers: [
        { provide: ContentService, useValue: contentServiceSpy },
        { provide: RendererRegistryService, useValue: registrySpy },
      ],
    }).compileComponents();
    fixture = TestBed.createComponent(EprFocalComponent);
  });

  function setSlug(slug: string): void {
    fixture.componentRef.setInput('slug', slug);
    fixture.detectChanges();
  }

  it('loads the node for the slug and emits nodeLoaded', () => {
    const loaded = vi.fn();
    fixture.componentInstance.nodeLoaded.subscribe(loaded);
    setSlug('manifesto');
    expect(contentServiceSpy.getContentBySlug).toHaveBeenCalledWith('manifesto');
    expect(loaded).toHaveBeenCalledWith(mockNode);
  });

  it('renders the plaintext fallback when no renderer is registered', () => {
    setSlug('manifesto');
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('.plaintext-content')?.textContent).toContain(
      'plain body'
    );
  });

  it('emits notFound when the slug resolves to null', () => {
    contentServiceSpy.getContentBySlug.mockReturnValue(of(null));
    const notFound = vi.fn();
    fixture.componentInstance.notFound.subscribe(notFound);
    setSlug('missing');
    expect(notFound).toHaveBeenCalledWith('missing');
  });

  it('emits failed when the load errors', () => {
    contentServiceSpy.getContentBySlug.mockReturnValue(throwError(() => new Error('boom')));
    const failed = vi.fn();
    fixture.componentInstance.failed.subscribe(failed);
    setSlug('manifesto');
    expect(failed).toHaveBeenCalledWith('manifesto');
  });

  it('reloads when the slug input changes', () => {
    setSlug('manifesto');
    setSlug('succession');
    expect(contentServiceSpy.getContentBySlug).toHaveBeenNthCalledWith(2, 'succession');
  });
});
```

- [ ] **Step 2: Run it to confirm it fails**

Run from `app/elohim-app`: `pnpm exec vitest run --config vite.config.ts epr-focal`
Expected: FAIL — cannot resolve `./epr-focal.component`.

- [ ] **Step 3: Write the focal component**

`app/elohim-app/src/app/elohim/components/epr-focal/epr-focal.component.ts`:
```ts
import { CommonModule } from '@angular/common';
import {
  AfterViewChecked,
  ChangeDetectionStrategy,
  ChangeDetectorRef,
  Component,
  ComponentRef,
  EventEmitter,
  Input,
  OnChanges,
  OnDestroy,
  Output,
  SimpleChanges,
  ViewChild,
  ViewContainerRef,
  inject,
} from '@angular/core';

import { Subject } from 'rxjs';
import { takeUntil } from 'rxjs/operators';

import { ContentNode } from '@app/lamad/models/content-node.model';
import { RendererInitializerService } from '@app/lamad/renderers/renderer-initializer.service';
import {
  ContentRenderer,
  RendererRegistryService,
} from '@app/lamad/renderers/renderer-registry.service';
import { ContentService } from '@app/lamad/services/content.service';

/**
 * The node shape the focal slot loads and hands back. Shell consumers import
 * THIS alias so the lamad content substrate is referenced from one shell file
 * only (the cross-workspace import ratchet counts specifiers per file).
 */
export type FocalNode = ContentNode;

/**
 * EprFocalComponent — the focal render slot of the EPR atom home.
 *
 * Extracted from ContentDeliveryComponent (count-neutral under the import
 * ratchet): slug in, registered renderer hosted, node handed back to the
 * frame. Owns NO chrome, NO legs, NO provenance — only the content itself.
 * Renderer registration is manifest-driven through RendererInitializerService,
 * which must be injected here because this slot mounts outside LamadLayout.
 */
@Component({
  selector: 'app-epr-focal',
  standalone: true,
  imports: [CommonModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './epr-focal.component.html',
  styleUrl: './epr-focal.component.css',
})
export class EprFocalComponent implements OnChanges, AfterViewChecked, OnDestroy {
  @Input({ required: true }) slug!: string;
  @Output() readonly nodeLoaded = new EventEmitter<FocalNode>();
  @Output() readonly notFound = new EventEmitter<string>();
  @Output() readonly failed = new EventEmitter<string>();

  @ViewChild('rendererHost', { read: ViewContainerRef, static: false })
  rendererHost!: ViewContainerRef;

  node: FocalNode | null = null;
  isLoading = true;
  hasRegisteredRenderer = false;

  private rendererRef: ComponentRef<ContentRenderer> | null = null;
  private pendingRendererLoad = false;
  private readonly destroy$ = new Subject<void>();
  private readonly contentService = inject(ContentService);
  private readonly rendererRegistry = inject(RendererRegistryService);
  // Injecting triggers manifest-driven renderer registration (side effect).
  private readonly _rendererInit = inject(RendererInitializerService);
  private readonly cdr = inject(ChangeDetectorRef);

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['slug'] && this.slug) this.load(this.slug);
  }

  ngAfterViewChecked(): void {
    if (this.pendingRendererLoad && this.node && this.rendererHost) {
      this.pendingRendererLoad = false;
      this.loadRenderer();
    }
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
    this.destroyRenderer();
  }

  getStringContent(content: string | object): string {
    return typeof content === 'string' ? content : JSON.stringify(content, null, 2);
  }

  private load(slug: string): void {
    this.isLoading = true;
    this.node = null;
    this.destroyRenderer();
    this.contentService
      .getContentBySlug(slug)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: node => {
          this.isLoading = false;
          if (!node) {
            this.notFound.emit(slug);
            this.cdr.markForCheck();
            return;
          }
          this.node = node;
          this.pendingRendererLoad = true;
          this.nodeLoaded.emit(node);
          this.cdr.markForCheck();
        },
        error: () => {
          this.isLoading = false;
          this.failed.emit(slug);
          this.cdr.markForCheck();
        },
      });
  }

  private loadRenderer(): void {
    if (!this.node || !this.rendererHost) return;
    this.destroyRenderer();
    this.rendererHost.clear();
    const rendererComponent = this.rendererRegistry.getRenderer(this.node);
    if (!rendererComponent) {
      this.hasRegisteredRenderer = false;
      this.cdr.markForCheck();
      return;
    }
    this.hasRegisteredRenderer = true;
    this.rendererRef = this.rendererHost.createComponent(rendererComponent);
    this.rendererRef.setInput('node', this.node);
    this.cdr.markForCheck();
  }

  private destroyRenderer(): void {
    if (this.rendererRef) {
      this.rendererRef.destroy();
      this.rendererRef = null;
    }
  }
}
```

`app/elohim-app/src/app/elohim/components/epr-focal/epr-focal.component.html`:
```html
<div class="epr-focal" data-testid="epr-focal">
  @if (isLoading) {
    <div class="epr-focal__loading" data-testid="epr-focal-loading">
      <div class="spinner"></div>
      <p>Loading content from the protocol network...</p>
    </div>
  }

  <ng-container #rendererHost></ng-container>

  @if (node && !isLoading && !hasRegisteredRenderer) {
    @if (node.contentFormat === 'plaintext') {
      <div class="plaintext-content"><pre>{{ getStringContent(node.content) }}</pre></div>
    } @else if (node.contentFormat === 'html') {
      <div class="html-content" [innerHTML]="node.content"></div>
    } @else {
      <div class="fallback-content">
        <p class="fallback-meta">{{ node.contentType }} &middot; {{ node.contentFormat }}</p>
        <pre>{{ getStringContent(node.content) }}</pre>
      </div>
    }
  }
</div>
```

`app/elohim-app/src/app/elohim/components/epr-focal/epr-focal.component.css` — move the `.loading-state`, `.spinner`, `.plaintext-content`, `.html-content`, `.fallback-content` rules out of `content-delivery.component.css` (rename `.loading-state` → `.epr-focal__loading`; keep the others byte-identical so the delivery page does not change).

- [ ] **Step 4: Run the focal spec**

Run: `pnpm exec vitest run --config vite.config.ts epr-focal`
Expected: 5 PASS.

- [ ] **Step 5: Make content-delivery compose the focal**

`content-delivery.component.ts` — replace the four `@app/lamad/*` imports and the renderer machinery:
```ts
import { CommonModule } from '@angular/common';
import { Component, OnDestroy, OnInit, inject } from '@angular/core';
import { ActivatedRoute, RouterModule } from '@angular/router';

import { takeUntil } from 'rxjs/operators';

import { Subject } from 'rxjs';

import { EprFocalComponent, FocalNode } from '../epr-focal/epr-focal.component';
import { SeoService } from '../../../services/seo.service';

interface OmnibarSteward {
  humanId: string;
  displayName: string;
  ratio: number;
}

/**
 * ContentDeliveryComponent — Full-page content delivery with protocol omnibar.
 * Composes <app-epr-focal> for the render; keeps provenance/SEO here.
 */
@Component({
  selector: 'app-content-delivery',
  standalone: true,
  imports: [CommonModule, RouterModule, EprFocalComponent],
  templateUrl: './content-delivery.component.html',
  styleUrls: ['./content-delivery.component.css'],
})
export class ContentDeliveryComponent implements OnInit, OnDestroy {
  slug = '';
  node: FocalNode | null = null;
  error: string | null = null;

  contentAddress = '';
  omnibarStewards: OmnibarSteward[] = [];
  reach = '';
  deliverySource = '';

  private readonly destroy$ = new Subject<void>();
  private readonly route = inject(ActivatedRoute);
  private readonly seoService = inject(SeoService);

  ngOnInit(): void {
    if (typeof window !== 'undefined') {
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: guarded by the typeof check above
      this.deliverySource = `doorway ${window.location.hostname}`;
    }
    this.route.params.pipe(takeUntil(this.destroy$)).subscribe(params => {
      const slug = params['slug'] as string;
      if (slug) {
        this.slug = slug;
        this.error = null;
        this.node = null;
      }
    });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  onNodeLoaded(node: FocalNode): void {
    this.node = node;
    this.contentAddress = node.id;
    this.reach = (node.reach as string) || 'commons';
    this.omnibarStewards = (node.stewardedBy ?? []).map(s => ({
      humanId: s.humanId,
      displayName: s.humanId,
      ratio: s.affinity ?? 0,
    }));
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

  onNotFound(): void {
    this.error = 'Content not found';
  }

  onFailed(): void {
    this.error = 'Failed to load content';
  }
}
```

`content-delivery.component.html` — replace the loading/content/fallback blocks (keep the comment and the error block, and the `data-testid` values `content-delivery`, `delivery-error`, `delivery-home-link`, `delivery-content`):
```html
<div class="delivery-shell" data-testid="content-delivery">
  <!-- (existing omnibar comment unchanged) -->

  <div *ngIf="error" class="error-state" data-testid="delivery-error">
    <h1>Content not found</h1>
    <p>{{ error }}</p>
    <a routerLink="/" class="home-link" data-testid="delivery-home-link">Return to protocol home</a>
  </div>

  <main *ngIf="slug && !error" class="delivery-content" data-testid="delivery-content">
    <app-epr-focal
      [slug]="slug"
      (nodeLoaded)="onNodeLoaded($event)"
      (notFound)="onNotFound()"
      (failed)="onFailed()"
    ></app-epr-focal>
  </main>
</div>
```

`content-delivery.component.spec.ts` — remove the two `@app/lamad/*` imports and the `ContentService`/`RendererRegistryService` providers; stub the focal so the delivery spec never constructs lamad services:
```ts
import { Component, EventEmitter, Input, Output } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute, provideRouter } from '@angular/router';
import { of } from 'rxjs';
import { vi } from 'vitest';

import { ContentDeliveryComponent } from './content-delivery.component';
import { EprFocalComponent } from '../epr-focal/epr-focal.component';
import { SeoService } from '../../../services/seo.service';

@Component({ selector: 'app-epr-focal', standalone: true, template: '' })
class EprFocalStub {
  @Input() slug = '';
  @Output() nodeLoaded = new EventEmitter<unknown>();
  @Output() notFound = new EventEmitter<string>();
  @Output() failed = new EventEmitter<string>();
}

describe('ContentDeliveryComponent', () => {
  let component: ContentDeliveryComponent;
  let fixture: ComponentFixture<ContentDeliveryComponent>;
  let seoServiceSpy: { updateForContent: ReturnType<typeof vi.fn> };

  const mockNode = {
    id: 'manifesto',
    title: 'The Elohim Protocol Manifesto',
    description: 'Founding document',
    contentType: 'epic',
    contentFormat: 'markdown',
    content: '# The Manifesto',
    reach: 'commons',
    stewardedBy: [{ humanId: 'genesis', role: 'steward', affinity: 0.8 }],
    tags: [],
    relatedNodeIds: [],
    metadata: {},
  };

  beforeEach(async () => {
    seoServiceSpy = { updateForContent: vi.fn() };
    await TestBed.configureTestingModule({
      imports: [ContentDeliveryComponent],
      providers: [
        provideRouter([]),
        { provide: SeoService, useValue: seoServiceSpy },
        { provide: ActivatedRoute, useValue: { params: of({ slug: 'manifesto' }) } },
      ],
    })
      .overrideComponent(ContentDeliveryComponent, {
        remove: { imports: [EprFocalComponent] },
        add: { imports: [EprFocalStub] },
      })
      .compileComponents();
    fixture = TestBed.createComponent(ContentDeliveryComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('creates', () => {
    expect(component).toBeTruthy();
  });

  it('hands the route slug to the focal slot', () => {
    expect(component.slug).toBe('manifesto');
    expect(fixture.nativeElement.querySelector('app-epr-focal')).not.toBeNull();
  });

  it('sets toolbar content address from the loaded node', () => {
    component.onNodeLoaded(mockNode as never);
    expect(component.contentAddress).toBe('manifesto');
  });

  it('extracts steward data for toolbar', () => {
    component.onNodeLoaded(mockNode as never);
    expect(component.omnibarStewards).toEqual([
      { humanId: 'genesis', displayName: 'genesis', ratio: 0.8 },
    ]);
  });

  it('sets reach for toolbar', () => {
    component.onNodeLoaded(mockNode as never);
    expect(component.reach).toBe('commons');
  });

  it('updates SEO metadata', () => {
    component.onNodeLoaded(mockNode as never);
    expect(seoServiceSpy.updateForContent).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'manifesto', title: 'The Elohim Protocol Manifesto' })
    );
  });

  it('shows error state when the focal reports not found', () => {
    component.onNotFound();
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-testid="delivery-error"]')).not.toBeNull();
  });

  it('shows error on focal failure', () => {
    component.onFailed();
    expect(component.error).toBe('Failed to load content');
  });
});
```

- [ ] **Step 6: Run both specs and the ratchet**

Run from `app/elohim-app`:
```bash
pnpm exec vitest run --config vite.config.ts "content-delivery|epr-focal"
node ../scripts/lint-workspace-imports.mjs . ; echo "EXIT=$?"
```
Expected: all PASS; the ratchet prints no `NEW` or `DEEPENED` edge and `EXIT=0`. If it reports a *shrink* (a count went DOWN because a delivery-spec import vanished), that is allowed but must be re-baselined: run `node ../scripts/lint-workspace-imports.mjs . --write-baseline` and include `app/scripts/workspace-import-baseline.json` in the commit with the reason "content-delivery spec no longer provides lamad services; the focal spec provides them".

- [ ] **Step 7: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/epr-focal app/elohim-app/src/app/elohim/components/content-delivery
git commit -m "refactor(shell): extract EprFocalComponent — the renderer host, count-neutral under the import ratchet

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_017GZH6i7cHvKanCC7R32jFh"
```

---

### Task 3: `EprHomeComponent` — atom, identity, focal, gate, address; route switch

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/epr-home/epr-home.model.ts`
- Create: `app/elohim-app/src/app/elohim/components/epr-home/epr-home.component.ts`
- Create: `app/elohim-app/src/app/elohim/components/epr-home/epr-home.component.html`
- Create: `app/elohim-app/src/app/elohim/components/epr-home/epr-home.component.css`
- Create: `app/elohim-app/src/app/elohim/components/epr-home/epr-home.component.spec.ts`
- Modify: `app/elohim-app/src/app/app.routes.ts:72-78`

**Interfaces:**
- Produces: `EprHomeAtom` view model and `toAtom(raw: Record<string, unknown>): EprHomeAtom`, `focalShape(format: string): 'immersive' | 'reading'`, `reachSubtitle(reach: string): string`, `anchorWords(trust, dhtAnchorState): string`.
- Consumes: `StorageClientService.getContent(id): Observable<StorageContentNode | null>` (raw wire shape; read as a record like `epr-raw-node` does), `EprFocalComponent` (Task 2).

- [ ] **Step 1: Write the model with its unit tests**

`app/elohim-app/src/app/elohim/components/epr-home/epr-home.model.ts`:
```ts
/**
 * View model for the EPR atom home. Projected from the RAW ContentView wire
 * shape (GET /db/content/{id}) — read as delivered, no case conversion. The
 * frame computes display words only; identity, reach and validation are
 * backend-owned and shown verbatim.
 */

export type FocalShape = 'immersive' | 'reading';

const IMMERSIVE_FORMATS = new Set(['html5-app', 'video', 'audio', 'external', 'sophia-quiz-json']);

export function focalShape(contentFormat: string): FocalShape {
  return IMMERSIVE_FORMATS.has(contentFormat) ? 'immersive' : 'reading';
}

const REACH_SUBTITLES: Record<string, string> = {
  commons: 'anyone can reach this',
  collective: 'members of the collective',
  invited: 'people who were invited',
  familiar: 'households that know each other',
  trusted: 'trusted households',
  intimate: 'the household',
  private: 'only its steward',
  self: 'only its steward',
};

export function reachSubtitle(reach: string): string {
  return REACH_SUBTITLES[reach] ?? '';
}

export function anchorWords(trust: string | null, dhtAnchorState: string | null): string {
  if (trust !== 'notarized') return 'Not yet notarized';
  return dhtAnchorState === 'verified' ? 'anchor verified here' : 'anchor not yet verified here';
}

export interface EprHomeAtom {
  id: string;
  title: string;
  description: string;
  contentType: string;
  contentFormat: string;
  shape: FocalShape;
  reach: string;
  trust: string | null;
  dhtAnchorHash: string | null;
  dhtAnchorState: string | null;
  validationStatus: string | null;
  blobHash: string | null;
  contentSizeBytes: number | null;
  createdAt: string;
  updatedAt: string;
  author: string | null;
  license: string | null;
  sourceUrl: string | null;
  canonicalUrl: string | null;
  estimatedTime: string | null;
  category: string | null;
  relatedIds: string[];
}

function str(v: unknown): string | null {
  return typeof v === 'string' && v.length > 0 ? v : null;
}

export function toAtom(raw: Record<string, unknown>): EprHomeAtom {
  const metadata = (raw['metadata'] as Record<string, unknown> | null) ?? {};
  const related = metadata['relatedNodeIds'];
  const contentFormat = String(raw['contentFormat'] ?? '');
  const authors = metadata['authors'];
  return {
    id: String(raw['id'] ?? ''),
    title: String(raw['title'] ?? raw['id'] ?? ''),
    description: String(raw['description'] ?? ''),
    contentType: String(raw['contentType'] ?? ''),
    contentFormat,
    shape: focalShape(contentFormat),
    reach: String(raw['reach'] ?? 'commons'),
    trust: str(raw['trust']),
    dhtAnchorHash: str(raw['dhtAnchorHash']),
    dhtAnchorState: str(raw['dhtAnchorState']),
    validationStatus: str(raw['validationStatus']),
    blobHash: str(raw['blobHash']),
    contentSizeBytes: typeof raw['contentSizeBytes'] === 'number' ? raw['contentSizeBytes'] : null,
    createdAt: String(raw['createdAt'] ?? ''),
    updatedAt: String(raw['updatedAt'] ?? ''),
    author: str(metadata['author']) ?? (Array.isArray(authors) ? authors.join(', ') : null),
    license: str(metadata['license']),
    sourceUrl: str(metadata['sourceUrl']),
    canonicalUrl: str(metadata['canonicalUrl']),
    estimatedTime: str(metadata['estimatedTime']),
    category: str(metadata['category']),
    relatedIds: Array.isArray(related) ? (related as string[]) : [],
  };
}

/** Short anchor for display: first 12 + last 8 characters. */
export function shortAnchor(hash: string): string {
  return hash.length <= 24 ? hash : `${hash.slice(0, 12)}…${hash.slice(-8)}`;
}

/** "May 27, 2026" from "2026-05-27 20:46:37" or ISO. */
export function dayWords(stamp: string): string {
  const d = new Date(stamp.replace(' ', 'T'));
  if (Number.isNaN(d.getTime())) return stamp;
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
}
```

Add to `epr-home.component.spec.ts` (created fully in Step 3) this first describe block:
```ts
import { describe, expect, it } from 'vitest';

import { anchorWords, focalShape, reachSubtitle, shortAnchor, toAtom } from './epr-home.model';

describe('epr-home.model', () => {
  it('shapes the focal slot by contentFormat', () => {
    expect(focalShape('html5-app')).toBe('immersive');
    expect(focalShape('markdown')).toBe('reading');
    expect(focalShape('')).toBe('reading');
  });

  it('says the anchor state in words', () => {
    expect(anchorWords('notarized', 'unverified')).toBe('anchor not yet verified here');
    expect(anchorWords('notarized', 'verified')).toBe('anchor verified here');
    expect(anchorWords(null, null)).toBe('Not yet notarized');
  });

  it('projects the raw wire shape without reshaping identity', () => {
    const atom = toAtom({
      id: 'evolution-of-trust',
      title: 'The Evolution of Trust',
      contentType: 'collective',
      contentFormat: 'html5-app',
      reach: 'commons',
      trust: 'notarized',
      dhtAnchorHash: 'uhCkk_D-fLh9hgcSAk4ZE6375dJuKrzf4Y9CDEOoX4e9fKujiEm8f',
      dhtAnchorState: 'unverified',
      metadata: { author: 'Nicky Case', license: 'CC0 Public Domain', relatedNodeIds: ['a', 'b'] },
    });
    expect(atom.shape).toBe('immersive');
    expect(atom.author).toBe('Nicky Case');
    expect(atom.relatedIds).toEqual(['a', 'b']);
    expect(reachSubtitle(atom.reach)).toBe('anyone can reach this');
    expect(shortAnchor(atom.dhtAnchorHash!)).toBe('uhCkk_D-fLh9…KujiEm8f');
  });
});
```

- [ ] **Step 2: Run the model tests**

Run: `pnpm exec vitest run --config vite.config.ts epr-home`
Expected: model tests PASS (the component describe does not exist yet).

- [ ] **Step 3: Write the failing component spec**

Append to `epr-home.component.spec.ts`:
```ts
import { Component, EventEmitter, Input, Output } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute, convertToParamMap, provideRouter } from '@angular/router';
import { of, throwError } from 'rxjs';
import { beforeEach, vi } from 'vitest';

import { EprHomeComponent } from './epr-home.component';
import { EprFocalComponent } from '../epr-focal/epr-focal.component';
import { StorageClientService } from '../../services/storage-client.service';

@Component({ selector: 'app-epr-focal', standalone: true, template: '<div class="focal-stub"></div>' })
class EprFocalStub {
  @Input() slug = '';
  @Output() nodeLoaded = new EventEmitter<unknown>();
  @Output() notFound = new EventEmitter<string>();
  @Output() failed = new EventEmitter<string>();
}

const rawSimulation = {
  id: 'evolution-of-trust',
  title: 'The Evolution of Trust',
  description: 'An interactive guide to the game theory of trust.',
  contentType: 'collective',
  contentFormat: 'html5-app',
  reach: 'commons',
  trust: 'notarized',
  dhtAnchorHash: 'uhCkk_D-fLh9hgcSAk4ZE6375dJuKrzf4Y9CDEOoX4e9fKujiEm8f',
  dhtAnchorState: 'unverified',
  createdAt: '2026-05-27 20:46:37',
  updatedAt: '2026-08-05 18:40:53',
  metadata: { author: 'Nicky Case', license: 'CC0 Public Domain', estimatedTime: '30 minutes' },
};

function q(fixture: ComponentFixture<EprHomeComponent>, id: string): Element | null {
  return fixture.nativeElement.querySelector(`[data-testid="${id}"]`);
}

describe('EprHomeComponent', () => {
  let fixture: ComponentFixture<EprHomeComponent>;
  let storage: { getContent: ReturnType<typeof vi.fn> };

  async function mount(resourceId: string): Promise<void> {
    await TestBed.configureTestingModule({
      imports: [EprHomeComponent],
      providers: [
        provideRouter([]),
        { provide: StorageClientService, useValue: storage },
        {
          provide: ActivatedRoute,
          useValue: { paramMap: of(convertToParamMap({ resourceId })) },
        },
      ],
    })
      .overrideComponent(EprHomeComponent, {
        remove: { imports: [EprFocalComponent] },
        add: { imports: [EprFocalStub] },
      })
      .compileComponents();
    fixture = TestBed.createComponent(EprHomeComponent);
    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();
  }

  beforeEach(() => {
    storage = { getContent: vi.fn().mockReturnValue(of(rawSimulation)) };
  });

  it('renders the frame with identity and chips for a reachable atom', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home')).not.toBeNull();
    expect(q(fixture, 'epr-home-title')?.textContent).toContain('The Evolution of Trust');
    expect(q(fixture, 'epr-home-chip-reach')?.textContent).toContain('Commons');
    expect(q(fixture, 'epr-home-chip-notarized')?.textContent).toContain(
      'anchor not yet verified here'
    );
    expect(fixture.nativeElement.textContent).not.toContain('Back to Lamad');
  });

  it('hands the slug to the focal slot in the immersive shape', async () => {
    await mount('evolution-of-trust');
    const focal = q(fixture, 'epr-home-focal');
    expect(focal?.classList.contains('epr-home__focal--immersive')).toBe(true);
    expect(focal?.querySelector('.focal-stub')).not.toBeNull();
  });

  it('uses the reading shape for markdown', async () => {
    storage.getContent.mockReturnValue(of({ ...rawSimulation, contentFormat: 'markdown' }));
    await mount('succession');
    expect(q(fixture, 'epr-home-focal')?.classList.contains('epr-home__focal--reading')).toBe(
      true
    );
  });

  it('renders the out-of-reach gate for a null atom, with no chrome', async () => {
    storage.getContent.mockReturnValue(of(null));
    await mount('concept-bidirectional-trust');
    expect(q(fixture, 'epr-home-gate')?.textContent).toContain("We can't reach this one from here");
    expect(q(fixture, 'epr-home-gate')?.textContent).toContain('concept-bidirectional-trust');
    expect(q(fixture, 'epr-home-your-mark')).toBeNull();
    expect(q(fixture, 'epr-home-focal')).toBeNull();
  });

  it('renders the error state when the load fails', async () => {
    storage.getContent.mockReturnValue(throwError(() => new Error('boom')));
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-error')).not.toBeNull();
  });

  it('carries the universal address line', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-address')?.textContent).toContain('/epr/evolution-of-trust');
  });
});
```

- [ ] **Step 4: Run to confirm it fails**

Run: `pnpm exec vitest run --config vite.config.ts epr-home`
Expected: FAIL — cannot resolve `./epr-home.component`.

- [ ] **Step 5: Write the component**

`app/elohim-app/src/app/elohim/components/epr-home/epr-home.component.ts`:
```ts
import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { ActivatedRoute, RouterModule } from '@angular/router';

import { catchError, distinctUntilChanged, map, of, switchMap } from 'rxjs';

import { EprFocalComponent } from '../epr-focal/epr-focal.component';
import { StorageClientService } from '../../services/storage-client.service';
import {
  EprHomeAtom,
  anchorWords,
  dayWords,
  reachSubtitle,
  shortAnchor,
  toAtom,
} from './epr-home.model';

type LoadState =
  | { status: 'loading'; id: string }
  | { status: 'loaded'; id: string; atom: EprHomeAtom }
  | { status: 'not-found'; id: string }
  | { status: 'error'; id: string };

/**
 * EprHomeComponent — the shell-owned universal address (/epr/{id}).
 *
 * One frame for every atom: arrival · identity · focal render shaped by format
 * · the four legs · the address line. Reads the RAW ContentView for the frame
 * and hands only the slug to <app-epr-focal>; imports nothing from the lamad
 * bundle. An unreachable atom renders the designed gate (spec §2.5) — never a
 * wall, never the controls of a thing that cannot be seen.
 */
@Component({
  selector: 'app-epr-home',
  standalone: true,
  imports: [CommonModule, RouterModule, EprFocalComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './epr-home.component.html',
  styleUrl: './epr-home.component.css',
})
export class EprHomeComponent {
  private readonly route = inject(ActivatedRoute);
  private readonly storage = inject(StorageClientService);

  readonly resourceId = toSignal(
    this.route.paramMap.pipe(
      map(p => p.get('resourceId') ?? ''),
      distinctUntilChanged()
    ),
    { initialValue: '' }
  );

  private readonly state = toSignal(
    this.route.paramMap.pipe(
      map(p => p.get('resourceId') ?? ''),
      distinctUntilChanged(),
      switchMap(id =>
        this.storage.getContent(id).pipe(
          map(
            (raw): LoadState =>
              raw === null
                ? { status: 'not-found', id }
                : { status: 'loaded', id, atom: toAtom(raw as unknown as Record<string, unknown>) }
          ),
          catchError(() => of<LoadState>({ status: 'error', id }))
        )
      )
    ),
    { initialValue: { status: 'loading', id: '' } as LoadState }
  );

  readonly status = computed(() => this.state().status);
  readonly atom = computed<EprHomeAtom | null>(() => {
    const s = this.state();
    return s.status === 'loaded' ? s.atom : null;
  });

  readonly reachLabel = computed(() => {
    const r = this.atom()?.reach ?? 'commons';
    return r.charAt(0).toUpperCase() + r.slice(1);
  });
  readonly reachSub = computed(() => reachSubtitle(this.atom()?.reach ?? 'commons'));
  readonly notarizedLabel = computed(() =>
    this.atom()?.trust === 'notarized' ? 'Notarized' : 'Not yet notarized'
  );
  readonly anchorSub = computed(() => {
    const a = this.atom();
    return a ? anchorWords(a.trust, a.dhtAnchorState) : '';
  });
  readonly eyebrow = computed<string[]>(() => {
    const a = this.atom();
    if (!a) return [];
    return [
      a.category ?? a.contentType,
      a.estimatedTime,
      a.author ? `by ${a.author}` : null,
      a.license,
    ].filter((x): x is string => !!x);
  });
  readonly address = computed(() => `/epr/${encodeURIComponent(this.resourceId())}`);
  readonly rawHref = computed(() => `${this.address()}/raw`);
  readonly anchorShort = computed(() => {
    const h = this.atom()?.dhtAnchorHash;
    return h ? shortAnchor(h) : null;
  });
  readonly addedOn = computed(() => dayWords(this.atom()?.createdAt ?? ''));
  readonly updatedOn = computed(() => dayWords(this.atom()?.updatedAt ?? ''));
}
```

`epr-home.component.html`:
```html
@switch (status()) {
  @case ('loading') {
    <main class="epr-home" data-testid="epr-home-loading" aria-busy="true">
      <p class="epr-home__status">Reaching for this…</p>
    </main>
  }

  @case ('error') {
    <main class="epr-home" data-testid="epr-home-error" role="alert">
      <h1 class="epr-home__title">Something went wrong reaching this</h1>
      <p class="epr-home__status">
        <code class="epr-home__mono">{{ resourceId() }}</code>
      </p>
    </main>
  }

  @case ('not-found') {
    <main class="epr-home epr-home--gate" data-testid="epr-home-gate">
      <p class="epr-home__eyebrow epr-home__eyebrow--warm">Out of reach from this doorway</p>
      <h1 class="epr-home__title">We can't reach this one from here</h1>
      <p class="epr-home__lede">
        No peer this doorway can ask is holding
        <code class="epr-home__mono">{{ resourceId() }}</code>
        . It isn't gone, and it isn't yours to fix; it just hasn't been brought within reach of
        this doorway yet.
      </p>
      <p class="epr-home__address" data-testid="epr-home-address">
        The same address on another doorway may reach it
        <code class="epr-home__mono">{{ address() }}</code>
      </p>
    </main>
  }

  @case ('loaded') {
    @if (atom(); as a) {
      <main class="epr-home" data-testid="epr-home">
        <header class="epr-home__identity">
          <p class="epr-home__eyebrow">
            @for (part of eyebrow(); track $index) {
              <span>{{ part }}</span>
            }
          </p>
          <h1 class="epr-home__title" data-testid="epr-home-title">{{ a.title }}</h1>
          @if (a.description) {
            <p class="epr-home__lede">{{ a.description }}</p>
          }
          <div class="epr-home__chips">
            <span class="epr-home__chip epr-home__chip--reach" data-testid="epr-home-chip-reach">
              {{ reachLabel() }}
              <small>· {{ reachSub() }}</small>
            </span>
            <span class="epr-home__chip" data-testid="epr-home-chip-notarized">
              {{ notarizedLabel() }}
              <small>· {{ anchorSub() }}</small>
            </span>
          </div>
        </header>

        <section
          class="epr-home__focal"
          [class.epr-home__focal--immersive]="a.shape === 'immersive'"
          [class.epr-home__focal--reading]="a.shape === 'reading'"
          data-testid="epr-home-focal"
        >
          <app-epr-focal [slug]="a.id"></app-epr-focal>
        </section>

        <footer class="epr-home__address" data-testid="epr-home-address">
          <span>This address works on any doorway that can reach it</span>
          <code class="epr-home__mono">{{ address() }}</code>
          <a class="epr-home__link" [routerLink]="['/resource', a.id]" fragment="network" data-testid="epr-home-network-detail">Network detail</a>
          <a class="epr-home__link" [href]="rawHref()">Raw node</a>
        </footer>
      </main>
    }
  }
}
```

`epr-home.component.css` (binds `--el-*` only; the layout grid gains the rail in Task 4):
```css
:host {
  display: block;
  background: var(--el-ground);
  color: var(--el-text);
  font-family: var(--el-font-body);
  min-height: 100vh;
}

.epr-home {
  max-width: 1200px;
  margin: 0 auto;
  padding: 40px 24px 48px;
  display: flex;
  flex-direction: column;
  gap: 32px;
}

.epr-home--gate {
  max-width: 720px;
  padding-top: 96px;
  gap: 24px;
}

.epr-home__identity {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.epr-home__eyebrow {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  font-family: var(--el-font-ui);
  font-size: 13px;
  color: var(--el-text-muted);
}

.epr-home__eyebrow span + span::before {
  content: '·';
  margin-right: 10px;
}

.epr-home__eyebrow--warm {
  color: var(--el-warm);
}

.epr-home__title {
  font-family: var(--el-font-display);
  font-weight: 500;
  font-size: clamp(30px, 4vw, 46px);
  line-height: 1.1;
  letter-spacing: -0.01em;
  color: var(--el-text-strong);
  font-variation-settings: 'SOFT' 50;
}

.epr-home__lede {
  font-size: 19px;
  line-height: 1.6;
  max-width: 68ch;
  text-wrap: pretty;
}

.epr-home__chips {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.epr-home__chip {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 34px;
  padding: 0 12px;
  border-radius: 999px;
  border: 1px solid var(--el-border);
  background: var(--el-surface);
  font-family: var(--el-font-ui);
  font-size: 13px;
  color: var(--el-text-strong);
}

.epr-home__chip small {
  color: var(--el-text-muted);
}

.epr-home__chip--reach {
  background: rgba(127, 176, 105, 0.16);
  border-color: rgba(127, 176, 105, 0.4);
}

.epr-home__chip--warm {
  background: rgba(184, 102, 79, 0.12);
  border-color: rgba(184, 102, 79, 0.45);
  color: var(--el-warm);
}

.epr-home__focal {
  border: 1px solid var(--el-border);
  border-radius: 12px;
  overflow: hidden;
  background: var(--el-surface);
}

.epr-home__focal--reading {
  border: 0;
  background: transparent;
  max-width: 72ch;
}

.epr-home__address {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 16px;
  padding-top: 20px;
  border-top: 1px solid var(--el-border);
  font-family: var(--el-font-ui);
  font-size: 13px;
  color: var(--el-text-muted);
}

.epr-home__mono {
  font-family: var(--el-font-mono);
  font-size: 12px;
  color: var(--el-text-strong);
}

.epr-home__link {
  color: var(--el-action);
  text-decoration: none;
}

.epr-home__link:hover {
  text-decoration: underline;
}

.epr-home__status {
  font-family: var(--el-font-ui);
  font-size: 14px;
}
```

- [ ] **Step 6: Switch the route**

`app/elohim-app/src/app/app.routes.ts` lines 68–78 — replace the comment and the `loadComponent`:
```ts
  // Universal EPR address (§12.1) — the atom's own home, shell-owned (spec
  // 2026-09-02-epr-atom-home-shell-component-design). Reachable-but-unclaimed
  // atoms render here; the doorway serves this bundle for any /epr/* path.
  {
    path: 'epr/:resourceId',
    loadComponent: async () =>
      import('./elohim/components/epr-home/epr-home.component').then(m => m.EprHomeComponent),
    data: { protocolContent: true },
  },
```
Then run the routes spec: `pnpm exec vitest run --config vite.config.ts app.routes` — if it asserts the old target for `epr/:resourceId`, update that assertion to `EprHomeComponent` (the spec pins route shape, not the lamad component).

- [ ] **Step 7: Run the specs and the ratchet**

```bash
pnpm exec vitest run --config vite.config.ts "epr-home|app.routes"
node ../scripts/lint-workspace-imports.mjs . ; echo "EXIT=$?"
node ../scripts/lint-route-literals.mjs src ; echo "EXIT=$?"
```
Expected: all PASS; both lints `EXIT=0`. Note: the baseline entry `@app/lamad/components/content-viewer/content-viewer.component: 2` may now read 1 (the route no longer imports it) — that is a shrink; re-baseline with `--write-baseline` and include the baseline in the commit.

- [ ] **Step 8: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/epr-home app/elohim-app/src/app/app.routes.ts app/elohim-app/src/app/app.routes.spec.ts app/scripts/workspace-import-baseline.json
git commit -m "feat(shell): EprHomeComponent owns /epr/{id} — identity, focal by shape, address, designed gate

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_017GZH6i7cHvKanCC7R32jFh"
```

---

### Task 4: The four legs (rail) + the held-by chip

**Files:**
- Create: `app/elohim-app/src/app/elohim/components/epr-home/epr-home-legs.component.ts`
- Create: `app/elohim-app/src/app/elohim/components/epr-home/epr-home-legs.component.html`
- Create: `app/elohim-app/src/app/elohim/components/epr-home/epr-home-legs.component.css`
- Create: `app/elohim-app/src/app/elohim/components/epr-home/epr-home-legs.component.spec.ts`
- Modify: `epr-home.model.ts` (add `StewardRow`, `HoldingWords`, `holdingWords()`)
- Modify: `epr-home.component.ts/.html/.css/.spec.ts` (load leg data; layout grid; held chip)

**Interfaces:**
- Produces: `EprHomeLegsComponent` (selector `app-epr-home-legs`) with signal inputs `atom = input.required<EprHomeAtom>()`, `snapshot = input<ResilienceSnapshotView | null>(null)`, `stewards = input<StewardRow[]>([])`, `relationships = input<EprRelationship[]>([])`, `challenges = input<ChallengeView[]>([])`, `peersHolding = input<number | null>(null)`.
- Produces in the model: `interface StewardRow { stewardPresenceId: string; contributionType: string; effectiveFrom: string }`, `interface HoldingWords { headline: string; has: number; wants: number; warm: boolean; households: string[]; action: string }`, `holdingWords(snapshot: ResilienceSnapshotView | null): HoldingWords`.
- Consumes: `ResilienceService.getSnapshot(id)` and `DistributionService.getDetails(blobHash)` from `@elohim/service`; `StorageApiService.getStewardshipAllocations({ contentId, activeOnly: true })` (shell); `EprResolverService.resolveEprHead(id)` (shell); `GovernanceApiService.getChallengesForEntity('content', id)` (`@elohim/service`); `ChallengeView` from `@elohim/storage-client/generated`; `EprRelationship` from `../../models/epr-head.model`; `EprRelationshipsPanelComponent` (shell, `app-epr-relationships-panel`, `@Input() relationships`).

- [ ] **Step 1: Add the holding words to the model with tests**

Append to `epr-home.model.ts`:
```ts
import type { ResilienceSnapshotView } from '@elohim/service';

export interface StewardRow {
  stewardPresenceId: string;
  contributionType: string;
  effectiveFrom: string;
}

export interface HoldingWords {
  headline: string;
  has: number;
  wants: number;
  warm: boolean;
  households: string[];
  action: string;
}

/** The ONE holding verdict on the page, in household words (spec §2.2). */
export function holdingWords(snapshot: ResilienceSnapshotView | null): HoldingWords {
  const felt = snapshot?.feltStatus;
  if (!felt) {
    return {
      headline: "We can't confirm this is backed up anywhere yet.",
      has: 0,
      wants: 3,
      warm: true,
      households: [],
      action: 'Invite a household to help hold this',
    };
  }
  return {
    headline: felt.headline,
    has: felt.floor.hasHouseholds,
    wants: felt.floor.wantsHouseholds,
    warm: felt.reassurance === 'needs-help' || felt.reassurance === 'not-yet-seen',
    households: felt.heldBy.map(h =>
      h.intraHubPeers ? `${h.label ?? h.id} · ${h.intraHubPeers} peers` : (h.label ?? h.id)
    ),
    action: felt.suggestedAction,
  };
}

export function heldChip(words: HoldingWords): string {
  return words.has === 0
    ? 'Not yet held by any household'
    : `Held by ${words.has} of ${words.wants} households`;
}
```
(Confirm the `feltStatus.floor` field names against `app/elohim-library/projects/elohim-service/src/generated/resilience-snapshot-view.ts` lines 93–135: `hasHouseholds`, `wantsHouseholds`, `suggestedAction` are the generated names; if the generated file spells them differently, use the generated spelling — never edit the generated file.)

Append to the model describe in `epr-home.component.spec.ts`:
```ts
  it('renders the felt status as one verdict in household words', () => {
    const words = holdingWords({
      contentId: 'x',
      feltStatus: {
        headline: 'Held by only 1 household — invite another to help hold these',
        reassurance: 'needs-help',
        heldBy: [{ id: 'household-dowell', kind: 'household', label: 'Dowell Household', intraHubPeers: 2 }],
        floor: { tier: 'standard', tierDeclared: false, wantsHouseholds: 3, hasHouseholds: 1 },
        suggestedAction: 'Invite a household to help hold these',
      },
    } as never);
    expect(words.has).toBe(1);
    expect(words.warm).toBe(true);
    expect(words.households).toEqual(['Dowell Household · 2 peers']);
    expect(heldChip(words)).toBe('Held by 1 of 3 households');
    expect(heldChip(holdingWords(null))).toBe('Not yet held by any household');
  });
```
(add `heldChip, holdingWords` to that spec's model import.)

- [ ] **Step 2: Write the failing legs spec**

`epr-home-legs.component.spec.ts`:
```ts
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { beforeEach, describe, expect, it } from 'vitest';

import { EprHomeLegsComponent } from './epr-home-legs.component';
import { toAtom } from './epr-home.model';

const atom = toAtom({
  id: 'evolution-of-trust',
  title: 'The Evolution of Trust',
  contentType: 'collective',
  contentFormat: 'html5-app',
  reach: 'commons',
  trust: 'notarized',
  dhtAnchorHash: 'uhCkk_D-fLh9hgcSAk4ZE6375dJuKrzf4Y9CDEOoX4e9fKujiEm8f',
  dhtAnchorState: 'unverified',
  createdAt: '2026-05-27 20:46:37',
  updatedAt: '2026-08-05 18:40:53',
  metadata: { sourceUrl: 'https://github.com/ncase/trust', license: 'CC0 Public Domain', relatedNodeIds: ['concept-bidirectional-trust'] },
});

const snapshot = {
  contentId: 'evolution-of-trust',
  feltStatus: {
    headline: 'Held by only 1 household — invite another to help hold these',
    reassurance: 'needs-help',
    heldBy: [{ id: 'household-dowell', kind: 'household', label: 'Dowell Household', intraHubPeers: 2 }],
    floor: { tier: 'standard', tierDeclared: false, wantsHouseholds: 3, hasHouseholds: 1 },
    suggestedAction: 'Invite a household to help hold these',
  },
};

function q(fixture: ComponentFixture<EprHomeLegsComponent>, id: string): Element | null {
  return fixture.nativeElement.querySelector(`[data-testid="${id}"]`);
}

describe('EprHomeLegsComponent', () => {
  let fixture: ComponentFixture<EprHomeLegsComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [EprHomeLegsComponent],
      providers: [provideRouter([])],
    }).compileComponents();
    fixture = TestBed.createComponent(EprHomeLegsComponent);
    fixture.componentRef.setInput('atom', atom);
  });

  it('Who holds it: the felt headline, the floor, the households, the action', () => {
    fixture.componentRef.setInput('snapshot', snapshot);
    fixture.componentRef.setInput('peersHolding', 5);
    fixture.detectChanges();
    const leg = q(fixture, 'epr-home-leg-holds')!;
    expect(leg.textContent).toContain('Held by only 1 household');
    expect(leg.textContent).toContain('1 of 3 households this should live in');
    expect(leg.textContent).toContain('Dowell Household');
    expect(leg.textContent).toContain('5 peers keep a copy');
    expect(leg.textContent).toContain('Invite a household to help hold these');
    expect(leg.textContent).not.toMatch(/\d+%/);
  });

  it('Who holds it: collapses to one line when nothing is known', () => {
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-leg-holds')?.textContent).toContain(
      "We can't confirm this is backed up anywhere yet."
    );
  });

  it('Where this lives: names related ids and tags unreachable ones', () => {
    fixture.detectChanges();
    const leg = q(fixture, 'epr-home-leg-lives')!;
    expect(leg.textContent).toContain('Bidirectional trust');
    expect(leg.querySelector('a[href="/epr/concept-bidirectional-trust"]')).not.toBeNull();
  });

  it('Where this lives: keeps the relationships panel wrapper test id', () => {
    fixture.componentRef.setInput('relationships', [{ type: 'REFERENCES', target: 'governance-epic' }]);
    fixture.detectChanges();
    expect(q(fixture, 'viewer-relationships-panel')).not.toBeNull();
  });

  it("How it's governed: one line when nothing is in question", () => {
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-leg-governed')?.textContent).toContain(
      'No challenges, no labels. Nothing is in question.'
    );
  });

  it('Where it came from: steward, source, anchor in words, dates, raw link', () => {
    fixture.componentRef.setInput('stewards', [
      { stewardPresenceId: 'matthew-dowell', contributionType: 'original_creator', effectiveFrom: '2026-06-04 02:19:10' },
    ]);
    fixture.detectChanges();
    const leg = q(fixture, 'epr-home-leg-from')!;
    expect(leg.textContent).toContain('matthew-dowell');
    expect(leg.textContent).toContain('original creator');
    expect(leg.textContent).toContain('github.com/ncase/trust');
    expect(leg.textContent).toContain('uhCkk_D-fLh9…KujiEm8f');
    expect(leg.textContent).toContain('not yet verified on this doorway');
    expect(leg.textContent).toContain('May 27, 2026');
    expect(leg.querySelector('a[href="/epr/evolution-of-trust/raw"]')).not.toBeNull();
  });
});
```

- [ ] **Step 3: Run to confirm it fails**

Run: `pnpm exec vitest run --config vite.config.ts epr-home-legs`
Expected: FAIL — cannot resolve `./epr-home-legs.component`.

- [ ] **Step 4: Write the legs component**

`epr-home-legs.component.ts`:
```ts
import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';

import type { ResilienceSnapshotView } from '@elohim/service';
import type { ChallengeView } from '@elohim/storage-client/generated';

import { EprRelationshipsPanelComponent } from '../epr-relationships-panel/epr-relationships-panel.component';
import { EprRelationship } from '../../models/epr-head.model';
import { EprHomeAtom, StewardRow, dayWords, holdingWords, shortAnchor } from './epr-home.model';

/** "concept-bidirectional-trust" → "Bidirectional trust" (label absent → humanized slug). */
export function humanizeSlug(slug: string): string {
  const words = slug.replace(/^(concept|fct-module-\d+)-/, '').split('-').filter(Boolean);
  return words.map((w, i) => (i === 0 ? w.charAt(0).toUpperCase() + w.slice(1) : w)).join(' ');
}

/**
 * The four legs of the atom, in household words (spec §2.2). Presentational:
 * everything arrives as inputs; the frame owns loading.
 */
@Component({
  selector: 'app-epr-home-legs',
  standalone: true,
  imports: [CommonModule, EprRelationshipsPanelComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './epr-home-legs.component.html',
  styleUrl: './epr-home-legs.component.css',
})
export class EprHomeLegsComponent {
  readonly atom = input.required<EprHomeAtom>();
  readonly snapshot = input<ResilienceSnapshotView | null>(null);
  readonly stewards = input<StewardRow[]>([]);
  readonly relationships = input<EprRelationship[]>([]);
  readonly challenges = input<ChallengeView[]>([]);
  readonly peersHolding = input<number | null>(null);

  readonly holding = computed(() => holdingWords(this.snapshot()));
  readonly pips = computed(() => Array.from({ length: this.holding().wants }, (_, i) => i < this.holding().has));
  readonly related = computed(() =>
    this.atom().relatedIds.map(id => ({ id, label: humanizeSlug(id), href: `/epr/${encodeURIComponent(id)}` }))
  );
  readonly openChallenges = computed(() => this.challenges().filter(c => c.state !== 'resolved'));
  readonly anchorShort = computed(() => {
    const h = this.atom().dhtAnchorHash;
    return h ? shortAnchor(h) : null;
  });
  readonly anchorVerified = computed(() => this.atom().dhtAnchorState === 'verified');
  readonly addedOn = computed(() => dayWords(this.atom().createdAt));
  readonly updatedOn = computed(() => dayWords(this.atom().updatedAt));
  readonly rawHref = computed(() => `/epr/${encodeURIComponent(this.atom().id)}/raw`);
  readonly sourceLabel = computed(() => {
    const s = this.atom().sourceUrl ?? this.atom().canonicalUrl;
    return s ? s.replace(/^https?:\/\//, '').replace(/\/$/, '') : null;
  });

  contribution(row: StewardRow): string {
    return row.contributionType.replace(/_/g, ' ');
  }

  since(row: StewardRow): string {
    return dayWords(row.effectiveFrom);
  }
}
```

`epr-home-legs.component.html`:
```html
<aside class="legs">
  <!-- Who holds it (value) -->
  <section class="leg" id="holds" data-testid="epr-home-leg-holds">
    <h3 class="leg__title">Who holds it</h3>
    @if (holding().households.length > 0) {
      @for (h of holding().households; track h) {
        <div class="leg__row">{{ h }}</div>
      }
    } @else {
      <p class="leg__line">{{ holding().headline }}</p>
    }
    @if (holding().households.length > 0) {
      <p class="leg__line leg__line--quiet">{{ holding().headline }}</p>
    }
    <div class="leg__row">
      <span class="pips" aria-hidden="true">
        @for (on of pips(); track $index) {
          <span class="pip" [class.pip--on]="on"></span>
        }
      </span>
      <span class="leg__ui" [class.leg__ui--warm]="holding().warm">
        {{ holding().has }} of {{ holding().wants }} households this should live in
      </span>
    </div>
    @if (peersHolding() !== null) {
      <div class="leg__row leg__row--muted">{{ peersHolding() }} peers keep a copy</div>
    }
    <button type="button" class="leg__btn" data-testid="epr-home-invite-household">
      {{ holding().action }}
    </button>
  </section>

  <!-- Where this lives (knowledge) -->
  <section class="leg" id="lives" data-testid="epr-home-leg-lives">
    <h3 class="leg__title">Where this lives</h3>
    @if (relationships().length === 0 && related().length === 0) {
      <p class="leg__line">Not on a learning path yet.</p>
    }
    @if (relationships().length > 0) {
      <div data-testid="viewer-relationships-panel" aria-label="Related concepts">
        <app-epr-relationships-panel [relationships]="relationships()"></app-epr-relationships-panel>
      </div>
    }
    @if (related().length > 0) {
      <p class="leg__ui leg__ui--muted">Named as related</p>
      @for (r of related(); track r.id) {
        <a class="leg__row leg__row--link" [href]="r.href" [attr.data-related-id]="r.id">
          <span>{{ r.label }}</span>
          <span class="pill">not held on this doorway</span>
        </a>
      }
    }
  </section>

  <!-- How it's governed (governance) -->
  <section class="leg" id="governed" data-testid="epr-home-leg-governed">
    <h3 class="leg__title">How it's governed</h3>
    @if (openChallenges().length === 0) {
      <p class="leg__line">No challenges, no labels. Nothing is in question.</p>
    } @else {
      @for (c of openChallenges(); track c.id) {
        <div class="leg__row">
          <span>{{ c.grounds_primary }}</span>
          <span class="pill">{{ c.state }}</span>
        </div>
      }
    }
    <div class="leg__actions">
      <button type="button" class="leg__btn leg__btn--quiet" data-testid="epr-home-raise-concern">Raise a concern</button>
    </div>
  </section>

  <!-- Where it came from (process) -->
  <section class="leg" id="from" data-testid="epr-home-leg-from">
    <h3 class="leg__title">Where it came from</h3>
    @for (s of stewards(); track s.stewardPresenceId) {
      <div class="leg__row">
        <span class="leg__ui leg__ui--muted">Steward</span>
        <span>{{ s.stewardPresenceId }}</span>
        <span class="pill">{{ contribution(s) }} · since {{ since(s) }}</span>
      </div>
    }
    @if (sourceLabel(); as src) {
      <div class="leg__row">
        <span class="leg__ui leg__ui--muted">Source</span>
        <a class="leg__link" [href]="atom().sourceUrl ?? atom().canonicalUrl" rel="noopener">{{ src }}</a>
        @if (atom().license) {
          <span class="leg__ui leg__ui--muted">· {{ atom().license }}</span>
        }
      </div>
    }
    <div class="leg__row leg__row--stack">
      <span class="leg__ui leg__ui--muted">Notarized</span>
      <span>added {{ addedOn() }} · updated {{ updatedOn() }}</span>
      @if (anchorShort(); as anchor) {
        <span class="leg__mono">
          {{ anchor }}
          <span class="leg__ui leg__ui--muted">
            · {{ anchorVerified() ? 'verified on this doorway' : 'not yet verified on this doorway' }}
          </span>
        </span>
      }
    </div>
    <a class="leg__link leg__ui" [href]="rawHref()">Raw node ›</a>
  </section>
</aside>
```

`epr-home-legs.component.css`:
```css
:host {
  display: block;
  font-family: var(--el-font-body);
  color: var(--el-text);
}

.leg {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 20px 0 24px;
  border-top: 1px solid var(--el-border);
}

.leg__title {
  margin: 0;
  font-family: var(--el-font-display);
  font-weight: 500;
  font-size: 19px;
  color: var(--el-text-strong);
}

.leg__line {
  margin: 0;
  font-family: var(--el-font-ui);
  font-size: 14px;
  color: var(--el-text-strong);
}

.leg__line--quiet {
  color: var(--el-text-muted);
  font-size: 13px;
}

.leg__row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
  font-family: var(--el-font-ui);
  font-size: 14px;
  color: var(--el-text-strong);
}

.leg__row--muted {
  color: var(--el-text-muted);
  font-size: 13px;
}

.leg__row--stack {
  flex-direction: column;
  align-items: flex-start;
  gap: 4px;
}

.leg__row--link {
  justify-content: space-between;
  text-decoration: none;
  color: var(--el-text-muted);
}

.leg__ui {
  font-family: var(--el-font-ui);
  font-size: 13px;
}

.leg__ui--muted {
  color: var(--el-text-muted);
}

.leg__ui--warm {
  color: var(--el-warm);
}

.leg__mono {
  font-family: var(--el-font-mono);
  font-size: 12px;
  color: var(--el-text-muted);
}

.leg__link {
  color: var(--el-action);
  text-decoration: none;
}

.leg__link:hover {
  text-decoration: underline;
}

.leg__btn {
  align-self: flex-start;
  height: 40px;
  padding: 0 16px;
  border-radius: 8px;
  border: 1px solid var(--el-border);
  background: transparent;
  color: var(--el-text-strong);
  font-family: var(--el-font-ui);
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
}

.leg__btn--quiet {
  border-color: transparent;
  padding: 0 8px;
  color: var(--el-text-muted);
}

.leg__actions {
  display: flex;
  gap: 8px;
}

.pips {
  display: inline-flex;
  gap: 4px;
}

.pip {
  width: 14px;
  height: 14px;
  border-radius: 3px;
  border: 1.5px solid var(--el-amber);
}

.pip--on {
  background: var(--el-amber);
}

.pill {
  display: inline-flex;
  align-items: center;
  height: 22px;
  padding: 0 8px;
  border-radius: 999px;
  background: var(--el-surface-2);
  font-family: var(--el-font-ui);
  font-size: 11px;
  font-weight: 500;
  color: var(--el-text-muted);
}
```

- [ ] **Step 5: Run the legs spec**

Run: `pnpm exec vitest run --config vite.config.ts epr-home-legs`
Expected: 6 PASS. (`ChallengeView` field names `state`, `grounds_primary`, `id` come from `elohim/elohim-views/src/qahal.rs:52-74` as generated; if the generated TS is camelCase — `groundsPrimary` — use the generated spelling.)

- [ ] **Step 6: Wire the legs into the frame**

`epr-home.component.ts` — add the imports and loaders:
```ts
import { from } from 'rxjs';

import { DistributionService, ResilienceService } from '@elohim/service';
import { GovernanceApiService } from '@elohim/service';
import type { ResilienceSnapshotView } from '@elohim/service';
import type { ChallengeView } from '@elohim/storage-client/generated';

import { EprHomeLegsComponent } from './epr-home-legs.component';
import { EprResolverService } from '../../services/epr-resolver.service';
import { StorageApiService } from '../../services/storage-api.service';
import { EprRelationship } from '../../models/epr-head.model';
import { StewardRow, heldChip, holdingWords } from './epr-home.model';
```
Add `EprHomeLegsComponent` to `imports`, inject the services, and add these members after `atom`:
```ts
  private readonly resilience = inject(ResilienceService);
  private readonly distribution = inject(DistributionService);
  private readonly storageApi = inject(StorageApiService);
  private readonly eprResolver = inject(EprResolverService);
  private readonly governance = inject(GovernanceApiService);

  /** The loaded atom's id as a stream — every leg loader keys off it. */
  private readonly atomId$ = toObservable(this.atom).pipe(
    map(a => a?.id ?? null),
    distinctUntilChanged()
  );

  readonly snapshot = toSignal<ResilienceSnapshotView | null>(
    this.atomId$.pipe(
      switchMap(id => (id ? this.resilience.getSnapshot(id).pipe(catchError(() => of(null))) : of(null)))
    ),
    { initialValue: null }
  );

  readonly peersHolding = toSignal<number | null>(
    toObservable(this.atom).pipe(
      map(a => a?.blobHash ?? null),
      distinctUntilChanged(),
      switchMap(hash =>
        hash
          ? from(this.distribution.getDetails(hash)).pipe(
              map(d => d.summary.replicaCount),
              catchError(() => of(null))
            )
          : of(null)
      )
    ),
    { initialValue: null }
  );

  readonly stewards = toSignal<StewardRow[]>(
    this.atomId$.pipe(
      switchMap(id =>
        id
          ? this.storageApi.getStewardshipAllocations({ contentId: id, activeOnly: true }).pipe(
              map(rows =>
                rows.map(r => ({
                  stewardPresenceId: r.stewardPresenceId,
                  contributionType: r.contributionType,
                  effectiveFrom: r.effectiveFrom,
                }))
              ),
              catchError(() => of([]))
            )
          : of([])
      )
    ),
    { initialValue: [] }
  );

  readonly relationships = toSignal<EprRelationship[]>(
    this.atomId$.pipe(
      switchMap(id =>
        id
          ? this.eprResolver.resolveEprHead(id).pipe(
              map(head => head?.relationships ?? []),
              catchError(() => of([]))
            )
          : of([])
      )
    ),
    { initialValue: [] }
  );

  readonly challenges = toSignal<ChallengeView[]>(
    this.atomId$.pipe(
      switchMap(id =>
        id ? from(this.governance.getChallengesForEntity('content', id)).pipe(catchError(() => of([]))) : of([])
      )
    ),
    { initialValue: [] }
  );

  readonly heldChipLabel = computed(() => heldChip(holdingWords(this.snapshot())));
  readonly heldWarm = computed(() => holdingWords(this.snapshot()).warm);
```
(`toObservable` comes from `@angular/core/rxjs-interop`; add it to that import. `getStewardshipAllocations` is `storage-api.service.ts:721` and returns the lamad-typed view — the map to `StewardRow` keeps the lamad type out of this file.)

`epr-home.component.html` — inside the `loaded` branch: add the held chip after the notarized chip, and wrap the body below the header in the grid:
```html
            <a class="epr-home__chip" [class.epr-home__chip--warm]="heldWarm()" href="#holds" data-testid="epr-home-chip-held">
              {{ heldChipLabel() }}
            </a>
```
```html
        <div class="epr-home__body" [class.epr-home__body--reading]="a.shape === 'reading'">
          <div class="epr-home__main">
            <section class="epr-home__focal" …>…</section>
          </div>
          <app-epr-home-legs
            class="epr-home__rail"
            [atom]="a"
            [snapshot]="snapshot()"
            [peersHolding]="peersHolding()"
            [stewards]="stewards()"
            [relationships]="relationships()"
            [challenges]="challenges()"
          ></app-epr-home-legs>
        </div>
```
In the immersive shape the focal spans the full width ABOVE the grid (move the `<section class="epr-home__focal">` out of `.epr-home__main` when `a.shape === 'immersive'` — use two `@if` branches on `a.shape` so the immersive focal sits between the header and the grid and the reading focal sits inside the grid's main column).

`epr-home.component.css` — add:
```css
.epr-home__body {
  display: grid;
  grid-template-columns: minmax(0, 2fr) minmax(280px, 1fr);
  gap: 48px;
  align-items: start;
}

.epr-home__body--reading .epr-home__rail {
  position: sticky;
  top: 24px;
}

@media (max-width: 900px) {
  .epr-home__body {
    grid-template-columns: minmax(0, 1fr);
    gap: 24px;
  }
}
```

`epr-home.component.spec.ts` — add providers `{ provide: ResilienceService, useValue: { getSnapshot: vi.fn().mockReturnValue(of(null)) } }`, `{ provide: DistributionService, useValue: { getDetails: vi.fn().mockResolvedValue({ summary: { replicaCount: 5 } }) } }`, `{ provide: StorageApiService, useValue: { getStewardshipAllocations: vi.fn().mockReturnValue(of([])) } }`, `{ provide: EprResolverService, useValue: { resolveEprHead: vi.fn().mockReturnValue(of(null)) } }`, `{ provide: GovernanceApiService, useValue: { getChallengesForEntity: vi.fn().mockResolvedValue([]) } }` in `mount()`, and add:
```ts
  it('shows the held-by chip from the snapshot and renders all four legs', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-chip-held')?.textContent).toContain('Not yet held by any household');
    for (const leg of ['holds', 'lives', 'governed', 'from']) {
      expect(q(fixture, `epr-home-leg-${leg}`)).not.toBeNull();
    }
  });
```

- [ ] **Step 7: Run all epr-home specs, the ratchet, and the route-literal lint**

```bash
pnpm exec vitest run --config vite.config.ts "epr-home"
node ../scripts/lint-workspace-imports.mjs . ; echo "EXIT=$?"
node ../scripts/lint-route-literals.mjs src ; echo "EXIT=$?"
```
Expected: all PASS, both `EXIT=0`.

- [ ] **Step 8: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/epr-home
git commit -m "feat(shell): the four legs of the atom in household words — one holding verdict, felt status only

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_017GZH6i7cHvKanCC7R32jFh"
```

---

### Task 5: Arrival strip, gate referrer, and "Your mark on it"

**Files:**
- Modify: `epr-home.component.ts/.html/.css/.spec.ts`

**Interfaces:**
- Consumes: `SessionNavStackService.previous(): NavStackEntry | null` (`{ url, cid, label?, ts }`, shell), `EprNavService.navigate(url)` (shell), `AuthService.isAuthenticated: Signal<boolean>` (shell, `imagodei/services/auth.service`), `AffinityTrackingService.getAffinity(id): number`, `.setAffinity(id, value)`, `.trackView(id)` (shell), `affinity$` (BehaviorSubject stream).

- [ ] **Step 1: Write the failing specs**

Add to `epr-home.component.spec.ts` providers in `mount()`:
```ts
        { provide: SessionNavStackService, useValue: navStack },
        { provide: AuthService, useValue: auth },
        { provide: AffinityTrackingService, useValue: affinity },
```
with `beforeEach` defaults:
```ts
    navStack = { previous: vi.fn().mockReturnValue(null) };
    auth = { isAuthenticated: signal(false) };
    affinity = {
      getAffinity: vi.fn().mockReturnValue(0.2),
      setAffinity: vi.fn(),
      trackView: vi.fn(),
      affinity$: of({}),
    };
```
and tests:
```ts
  it('names the previous stop in the arrival chip when the nav stack has one', async () => {
    navStack.previous.mockReturnValue({ url: '/epr/succession', cid: '', label: 'Succession Without Conquest | Elohim Protocol', ts: 1 });
    await mount('evolution-of-trust');
    const chip = q(fixture, 'epr-home-arrival')!;
    expect(chip.textContent).toContain('Succession Without Conquest');
    expect(chip.textContent).not.toContain('| Elohim Protocol');
    expect(chip.getAttribute('href')).toBe('/epr/succession');
  });

  it('renders no arrival chip on a cold link', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-arrival')).toBeNull();
  });

  it('the gate names the referring resource when there is one', async () => {
    storage.getContent.mockReturnValue(of(null));
    navStack.previous.mockReturnValue({ url: '/epr/evolution-of-trust', cid: '', label: 'The Evolution of Trust | Elohim Protocol', ts: 1 });
    await mount('concept-bidirectional-trust');
    expect(q(fixture, 'epr-home-gate')?.textContent).toContain('The Evolution of Trust');
    expect(q(fixture, 'epr-home-gate-back')?.getAttribute('href')).toBe('/epr/evolution-of-trust');
  });

  it('shows Your mark only when signed in, as one row without a percentage badge', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-your-mark')).toBeNull();
    auth.isAuthenticated.set(true);
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-your-mark')?.textContent).toContain('Practicing · 20%');
  });
```
(import `signal` from `@angular/core`, `SessionNavStackService` from `../../services/session-nav-stack.service`, `AuthService` from `../../../imagodei/services/auth.service`, `AffinityTrackingService` from `../../services/affinity-tracking.service`.)

- [ ] **Step 2: Run to confirm they fail**

Run: `pnpm exec vitest run --config vite.config.ts "epr-home.component"`
Expected: the four new tests FAIL (no arrival chip, no gate-back, no your-mark).

- [ ] **Step 3: Implement**

`epr-home.component.ts` additions:
```ts
import { AuthService } from '../../../imagodei/services/auth.service';
import { AffinityTrackingService } from '../../services/affinity-tracking.service';
import { EprNavService } from '../../services/epr-nav.service';
import { SessionNavStackService } from '../../services/session-nav-stack.service';

  private readonly navStack = inject(SessionNavStackService);
  private readonly eprNav = inject(EprNavService);
  private readonly auth = inject(AuthService);
  private readonly affinityService = inject(AffinityTrackingService);

  /** Where you actually came from — the previous stop on the session nav stack, or nothing. */
  readonly arrival = computed(() => {
    const prev = this.navStack.previous();
    if (!prev) return null;
    const label = (prev.label ?? prev.url).replace(/\s*\|\s*Elohim Protocol\s*$/, '').trim();
    return { href: prev.url, label: label || prev.url };
  });

  readonly signedIn = this.auth.isAuthenticated;

  private readonly affinityTick = toSignal(this.affinityService.affinity$, { initialValue: null });
  readonly affinity = computed(() => {
    this.affinityTick();
    const a = this.atom();
    return a ? this.affinityService.getAffinity(a.id) : 0;
  });
  readonly affinityPercent = computed(() => Math.round(this.affinity() * 100));
  readonly affinityWord = computed(() => {
    const v = this.affinity();
    if (v === 0) return 'Unseen';
    if (v > 0.9) return 'Got it';
    return 'Practicing';
  });

  onArrival(event: Event, href: string): void {
    event.preventDefault();
    this.eprNav.navigate(href);
  }

  markPracticing(): void {
    const a = this.atom();
    if (a) this.affinityService.setAffinity(a.id, 0.5);
  }

  markGotIt(): void {
    const a = this.atom();
    if (a) this.affinityService.setAffinity(a.id, 1.0);
  }
```
Add `trackView` on load: an `effect(() => { const a = this.atom(); if (a) this.affinityService.trackView(a.id); })` in the constructor (import `effect`).

`epr-home.component.html`:
- Above `<header>` in the loaded branch and above the gate heading in the not-found branch:
```html
        @if (arrival(); as from) {
          <a class="epr-home__arrival" [href]="from.href" (click)="onArrival($event, from.href)" data-testid="epr-home-arrival">
            ‹ {{ from.label }}
          </a>
        }
```
- In the gate, after the lede:
```html
      @if (arrival(); as from) {
        <div class="epr-home__known">
          <span class="epr-home__ui epr-home__ui--muted">What we do know</span>
          <span>Named as related by <a class="epr-home__link" [href]="from.href">{{ from.label }}</a></span>
        </div>
        <a class="epr-home__btn epr-home__btn--primary" [href]="from.href" (click)="onArrival($event, from.href)" data-testid="epr-home-gate-back">
          Back to {{ from.label }}
        </a>
      }
```
- In `.epr-home__main`, before the focal for the reading shape and after it for immersive (i.e. the first item of the main column in both shapes):
```html
            @if (signedIn()) {
              <section class="epr-home__mark" data-testid="epr-home-your-mark">
                <div class="epr-home__mark-head">
                  <h3>Your mark on it</h3>
                  <span class="epr-home__ui epr-home__ui--muted">{{ affinityWord() }} · {{ affinityPercent() }}%</span>
                </div>
                <div class="epr-home__track"><div class="epr-home__track-fill" [style.width.%]="affinityPercent()"></div></div>
                <div class="epr-home__mark-actions">
                  <button type="button" class="epr-home__btn" (click)="markPracticing()">I'm practicing this</button>
                  <button type="button" class="epr-home__btn" (click)="markGotIt()">I've got this now</button>
                  <span class="epr-home__ui epr-home__ui--muted">Kept on your device, shared only when you say so</span>
                </div>
              </section>
            }
```
`epr-home.component.css` additions:
```css
.epr-home__arrival {
  align-self: flex-start;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 36px;
  padding: 0 12px;
  border-radius: 8px;
  background: var(--el-surface-2);
  font-family: var(--el-font-ui);
  font-size: 13px;
  color: var(--el-text-strong);
  text-decoration: none;
  max-width: 480px;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.epr-home__known {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 20px;
  border: 1px solid var(--el-border);
  border-radius: 10px;
  background: var(--el-surface);
  font-family: var(--el-font-ui);
  font-size: 14px;
}

.epr-home__btn {
  display: inline-flex;
  align-items: center;
  height: 40px;
  padding: 0 16px;
  border-radius: 8px;
  border: 1px solid var(--el-border);
  background: transparent;
  color: var(--el-text-strong);
  font-family: var(--el-font-ui);
  font-size: 14px;
  font-weight: 500;
  text-decoration: none;
  cursor: pointer;
}

.epr-home__btn--primary {
  background: var(--el-action);
  color: var(--el-on-action);
  border-color: var(--el-action);
  align-self: flex-start;
}

.epr-home__mark {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 20px;
  border: 1px solid var(--el-border);
  border-radius: 10px;
  background: var(--el-surface);
}

.epr-home__mark h3 {
  margin: 0;
  font-family: var(--el-font-display);
  font-weight: 500;
  font-size: 19px;
  color: var(--el-text-strong);
}

.epr-home__mark-head,
.epr-home__mark-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 10px;
}

.epr-home__track {
  height: 6px;
  border-radius: 999px;
  background: var(--el-surface-2);
  overflow: hidden;
}

.epr-home__track-fill {
  height: 100%;
  background: var(--el-amber);
  border-radius: 999px;
}

.epr-home__ui {
  font-family: var(--el-font-ui);
  font-size: 12px;
}

.epr-home__ui--muted {
  color: var(--el-text-muted);
}
```

- [ ] **Step 4: Run the specs**

Run: `pnpm exec vitest run --config vite.config.ts "epr-home"`
Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/epr-home
git commit -m "feat(shell): arrival reads the nav stack; the gate names its referrer; your mark is one row

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_017GZH6i7cHvKanCC7R32jFh"
```

---

### Task 6: "Open in Lamad" from generated all-bundle claims

**Files:**
- Modify: `elohim/sdk/schemas/scripts/codegen-route-claims.mjs` (`generateBundleFile`)
- Regenerate: `app/elohim-app/src/app/generated/route-claims.ts` (via `pnpm run route-claims:codegen` at repo root)
- Create: `app/elohim-app/src/app/elohim/components/epr-home/bundle-lens.ts`
- Create: `app/elohim-app/src/app/elohim/components/epr-home/bundle-lens.spec.ts`
- Modify: `epr-home.component.ts/.html`

**Interfaces:**
- Produces (generated, universal owner only): `export const BUNDLE_ROUTE_CLAIMS: readonly { bundle: string; claims: readonly RouteClaimTemplate[] }[]`.
- Produces: `openInBundle(contentType: string, id: string, claims = BUNDLE_ROUTE_CLAIMS, mounts = BUNDLE_MOUNTS): { href: string; bundleName: string } | null`.

- [ ] **Step 1: Write the failing lens spec**

`bundle-lens.spec.ts`:
```ts
import { describe, expect, it } from 'vitest';

import { openInBundle } from './bundle-lens';

const claims = [
  { bundle: 'lamad', claims: [{ contentType: 'path', template: 'path/{id}', fragments: { step: 'path/{id}/step/{n}' } }] },
];

describe('openInBundle', () => {
  it('mints the claiming bundle mount for a claimed type', () => {
    expect(openInBundle('path', 'foundations-christian-technology', claims, { lamad: '/lamad' })).toEqual({
      href: '/lamad/path/foundations-christian-technology',
      bundleName: 'Lamad',
    });
  });

  it('returns null for an unclaimed type', () => {
    expect(openInBundle('collective', 'evolution-of-trust', claims, { lamad: '/lamad' })).toBeNull();
  });

  it('encodes the id', () => {
    expect(openInBundle('path', 'a b', claims, { lamad: '/lamad' })?.href).toBe('/lamad/path/a%20b');
  });
});
```

- [ ] **Step 2: Run to confirm it fails**

Run: `pnpm exec vitest run --config vite.config.ts bundle-lens` → FAIL (module missing).

- [ ] **Step 3: Extend the codegen**

In `codegen-route-claims.mjs`, change `generateBundleFile(domain)` to `generateBundleFile(domain, domains)` and append, only when `domain.ownsUniversalRoute`:
```js
function emitBundleClaims(domains) {
  const claimBearing = domains.filter((d) => d.claims.length > 0);
  if (claimBearing.length === 0) return '[]';
  const lines = claimBearing
    .map((d) => `  { bundle: ${emitString(d.name)}, claims: ${emitClaimsArray(d.claims).replace(/\n/g, '\n  ')} },`)
    .join('\n');
  return `[\n${lines}\n]`;
}
```
and in `generateBundleFile` after the `_OWNS_UNIVERSAL_ROUTE` line:
```js
  const universal = domain.ownsUniversalRoute
    ? `
/** Every declaring bundle's claims — the universal-route owner mints cross-bundle lenses from these. */
export const BUNDLE_ROUTE_CLAIMS: readonly { bundle: string; claims: readonly RouteClaimTemplate[] }[] = ${emitBundleClaims(domains)};
`
    : '';
```
and return `${…}${universal}`. Update the call in `main()`: `generateBundleFile(domain, domains)`.

Run from repo root: `pnpm run route-claims:codegen && pnpm run route-claims:codegen:verify && git diff --stat app/lamad/src/app/generated/route-claims.ts genesis/seeder/src/generated/route-claims.ts`
Expected: verify passes; the lamad and seeder files show NO diff (only the elohim file changed).

- [ ] **Step 4: Write the lens**

`bundle-lens.ts`:
```ts
import { BUNDLE_ROUTE_CLAIMS, RouteClaimTemplate } from '../../../generated/route-claims';

/**
 * Where each claiming bundle is mounted on this doorway. Composition-root
 * config until the doorway's pretty-mount resolver (§12.6 Slice 3) makes the
 * client-side lens unnecessary. Default for an unlisted bundle: `/<bundle>`.
 */
export const BUNDLE_MOUNTS: Readonly<Record<string, string>> = {
  lamad: '/lamad', // route-literal-ok: bundle mount table (composition-root config), not a minted route
};

export interface BundleLens {
  href: string;
  bundleName: string;
}

export function openInBundle(
  contentType: string,
  id: string,
  claims: readonly { bundle: string; claims: readonly RouteClaimTemplate[] }[] = BUNDLE_ROUTE_CLAIMS,
  mounts: Readonly<Record<string, string>> = BUNDLE_MOUNTS
): BundleLens | null {
  for (const { bundle, claims: list } of claims) {
    const claim = list.find(c => c.contentType === contentType);
    if (!claim) continue;
    const mount = mounts[bundle] ?? `/${bundle}`;
    const path = claim.template.replace('{id}', encodeURIComponent(id));
    return { href: `${mount}/${path}`, bundleName: bundle.charAt(0).toUpperCase() + bundle.slice(1) };
  }
  return null;
}
```

`epr-home.component.ts`: `readonly lens = computed(() => { const a = this.atom(); return a ? openInBundle(a.contentType, a.id) : null; });`

`epr-home.component.html`, in the chips row (end, right-aligned):
```html
            @if (lens(); as l) {
              <a class="epr-home__btn epr-home__btn--primary epr-home__open" [href]="l.href" data-testid="epr-home-open-in-bundle">
                Open in {{ l.bundleName }} ↗
              </a>
            }
```
(Plain href: cross-bundle CONTENT links are full doorway loads, never routerLink — the epr-link interceptor records the handoff.)

- [ ] **Step 5: Run specs and lints**

```bash
pnpm exec vitest run --config vite.config.ts "bundle-lens|epr-home"
node ../scripts/lint-route-literals.mjs src ; echo "EXIT=$?"
node ../scripts/lint-workspace-imports.mjs . ; echo "EXIT=$?"
```
Expected: PASS, `EXIT=0` twice. If `lint-route-literals` still refuses the `/lamad` literal despite the trailing comment, read its accepted marker syntax at the top of `app/scripts/lint-route-literals.mjs` and use that exact form.

- [ ] **Step 6: Commit**

```bash
git add elohim/sdk/schemas/scripts/codegen-route-claims.mjs app/elohim-app/src/app/generated/route-claims.ts app/elohim-app/src/app/elohim/components/epr-home
git commit -m "feat(epr): universal-route owner receives every bundle's claims; 'Open in Lamad' minted, not literal

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_017GZH6i7cHvKanCC7R32jFh"
```

---

### Task 7: a2o steps for the frame scenarios

**Files:**
- Create: `genesis/a2o/src/framework/pages/epr-home.page.ts`
- Modify: `genesis/a2o/src/framework/pages/index.ts` (export)
- Create: `genesis/a2o/steps/ui/epr-atom-home.steps.ts`
- Modify: `genesis/a2o/features/content/epr-atom-home.feature` (remove `@wip` from scenarios 1, 2, 3, 4, 5, 6 and 10; scenarios 7–9 keep `@wip` until the commons plan)

**Interfaces:**
- Consumes: `E2EWorld.getHuman(name).devices` → `PlaywrightDevice` (`device.page`, `device.client.url`), `doorwayToAppUrl(url)` (`src/framework/utils/url.ts`), the test ids from Global Constraints.

- [ ] **Step 1: Page object**

`src/framework/pages/epr-home.page.ts`:
```ts
import type { Page } from 'playwright';

/** Selectors for the shell-owned EPR atom home (/epr/{id}) — data-testid contract from the spec §6. */
export const EPR_HOME = {
  ROOT: 'epr-home',
  GATE: 'epr-home-gate',
  GATE_BACK: 'epr-home-gate-back',
  ARRIVAL: 'epr-home-arrival',
  TITLE: 'epr-home-title',
  CHIP_REACH: 'epr-home-chip-reach',
  CHIP_NOTARIZED: 'epr-home-chip-notarized',
  CHIP_HELD: 'epr-home-chip-held',
  OPEN_IN_BUNDLE: 'epr-home-open-in-bundle',
  FOCAL: 'epr-home-focal',
  YOUR_MARK: 'epr-home-your-mark',
  ADDRESS: 'epr-home-address',
  LEG: (leg: 'holds' | 'lives' | 'governed' | 'from') => `epr-home-leg-${leg}`,
} as const;

export class EprHomePage {
  constructor(private readonly page: Page) {}

  private byId(id: string) {
    return this.page.locator(`[data-testid="${id}"]`);
  }

  async goto(appUrl: string, resourceId: string): Promise<void> {
    await this.page.goto(`${appUrl}/epr/${resourceId}`, { waitUntil: 'networkidle' });
    await this.page
      .locator(`[data-testid="${EPR_HOME.ROOT}"], [data-testid="${EPR_HOME.GATE}"]`)
      .first()
      .waitFor({ state: 'visible', timeout: 20_000 });
  }

  async title(): Promise<string> {
    return (await this.byId(EPR_HOME.TITLE).textContent())?.trim() ?? '';
  }

  async chipText(chip: 'reach' | 'notarized' | 'held'): Promise<string> {
    return (await this.byId(`epr-home-chip-${chip}`).textContent())?.trim() ?? '';
  }

  async focalShape(): Promise<'immersive' | 'reading' | null> {
    const cls = (await this.byId(EPR_HOME.FOCAL).getAttribute('class')) ?? '';
    if (cls.includes('epr-home__focal--immersive')) return 'immersive';
    if (cls.includes('epr-home__focal--reading')) return 'reading';
    return null;
  }

  async legText(leg: 'holds' | 'lives' | 'governed' | 'from'): Promise<string> {
    return (await this.byId(EPR_HOME.LEG(leg)).textContent())?.trim() ?? '';
  }

  async legVisible(leg: 'holds' | 'lives' | 'governed' | 'from'): Promise<boolean> {
    return this.byId(EPR_HOME.LEG(leg)).isVisible();
  }

  async legsBesideContent(): Promise<boolean> {
    const focal = await this.byId(EPR_HOME.FOCAL).boundingBox();
    const legs = await this.byId(EPR_HOME.LEG('holds')).boundingBox();
    if (!focal || !legs) return false;
    return legs.x > focal.x + focal.width - 1 && legs.y < focal.y + focal.height;
  }

  async focalFullWidth(): Promise<boolean> {
    const focal = await this.byId(EPR_HOME.FOCAL).boundingBox();
    const root = await this.byId(EPR_HOME.ROOT).boundingBox();
    if (!focal || !root) return false;
    return focal.width > root.width * 0.9;
  }

  async bodyText(): Promise<string> {
    return (await this.page.locator('body').textContent()) ?? '';
  }

  async arrivalText(): Promise<string | null> {
    const chip = this.byId(EPR_HOME.ARRIVAL);
    return (await chip.count()) > 0 ? ((await chip.textContent())?.trim() ?? '') : null;
  }

  async clickRelated(resourceId: string): Promise<void> {
    await this.page.locator(`[data-testid="${EPR_HOME.LEG('lives')}"] a[data-related-id="${resourceId}"]`).click();
    await this.byId(EPR_HOME.GATE).waitFor({ state: 'visible', timeout: 20_000 });
  }

  async clickOpenInBundle(): Promise<void> {
    await this.byId(EPR_HOME.OPEN_IN_BUNDLE).click();
    await this.page.waitForLoadState('networkidle');
  }

  async gateText(): Promise<string> {
    return (await this.byId(EPR_HOME.GATE).textContent())?.trim() ?? '';
  }

  async gateBackHref(): Promise<string | null> {
    return this.byId(EPR_HOME.GATE_BACK).getAttribute('href');
  }

  async has(id: string): Promise<boolean> {
    return (await this.byId(id).count()) > 0;
  }
}
```
Add `export * from './epr-home.page.js';` to `src/framework/pages/index.ts` (match the existing export style in that file).

- [ ] **Step 2: Steps**

`steps/ui/epr-atom-home.steps.ts`:
```ts
import { strict as assert } from 'node:assert';

import { Given, Then, When } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { EPR_HOME, EprHomePage } from '../../src/framework/pages/index.js';
import { doorwayToAppUrl } from '../../src/framework/utils/url.js';
import { E2EWorld } from '../../src/framework/world.js';

function requirePwDevice(world: E2EWorld, humanName: string): PlaywrightDevice {
  const human = world.getHuman(humanName);
  const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
  assert.ok(device, `${humanName} has no Playwright device. Is E2E_DEVICE_MODE=playwright?`);
  return device;
}

function home(world: E2EWorld, humanName: string): { page: EprHomePage; appUrl: string } {
  const device = requirePwDevice(world, humanName);
  return { page: new EprHomePage(device.page), appUrl: doorwayToAppUrl(device.client.url) };
}

// --- arrival ---

When('{word} opens the atom home for {string}', async function (this: E2EWorld, humanName: string, id: string) {
  const { page, appUrl } = home(this, humanName);
  await page.goto(appUrl, id);
});

When(
  '{word} opens the atom home for {string} as a cold link',
  async function (this: E2EWorld, humanName: string, id: string) {
    const device = requirePwDevice(this, humanName);
    await device.page.evaluate(() => sessionStorage.removeItem('elohim.session-nav-stack.v1'));
    const { page, appUrl } = home(this, humanName);
    await page.goto(appUrl, id);
  }
);

Given('{word} is viewing the atom home for {string}', async function (this: E2EWorld, humanName: string, id: string) {
  const { page, appUrl } = home(this, humanName);
  await page.goto(appUrl, id);
});

When(
  '{word} follows a link to the atom home for {string}',
  async function (this: E2EWorld, humanName: string, id: string) {
    // Walks through the protocol (records the handoff) rather than a cold goto.
    const device = requirePwDevice(this, humanName);
    const appUrl = doorwayToAppUrl(device.client.url);
    await device.page.evaluate(
      ([href, label]) => {
        const key = 'elohim.session-nav-stack.v1';
        const stack = JSON.parse(sessionStorage.getItem(key) ?? '[]') as unknown[];
        stack.push({ url: location.pathname, cid: '', label: document.title, ts: Date.now() });
        stack.push({ url: href, cid: '', label, ts: Date.now() + 1 });
        sessionStorage.setItem(key, JSON.stringify(stack));
      },
      [`/epr/${id}`, id]
    );
    await new EprHomePage(device.page).goto(appUrl, id);
  }
);

When(
  '{word} follows the related link to {string}',
  async function (this: E2EWorld, humanName: string, id: string) {
    const { page } = home(this, humanName);
    await page.clickRelated(id);
  }
);

// --- identity ---

Then('the atom home shows the title {string}', async function (this: E2EWorld, title: string) {
  const { page } = home(this, 'Matthew');
  assert.equal(await page.title(), title);
});

Then('the atom home shows the reach chip {string}', async function (this: E2EWorld, reach: string) {
  const { page } = home(this, 'Matthew');
  assert.ok((await page.chipText('reach')).includes(reach));
});

Then('the atom home shows the notarized chip', async function (this: E2EWorld) {
  const { page } = home(this, 'Matthew');
  assert.ok((await page.chipText('notarized')).includes('Notarized'));
});

Then('the atom home shows no {string} control', async function (this: E2EWorld, label: string) {
  const { page } = home(this, 'Matthew');
  assert.ok(!(await page.bodyText()).includes(label), `found "${label}" on the atom home`);
});

Then('the atom home shows no trust percentage', async function (this: E2EWorld) {
  const { page } = home(this, 'Matthew');
  const holds = await page.legText('holds');
  assert.ok(!/\d+\s*%/.test(holds), `holding leg carries a percentage: ${holds}`);
});

// --- focal shape ---

Then('the focal slot renders the content at full width', async function (this: E2EWorld) {
  const { page } = home(this, 'Matthew');
  assert.equal(await page.focalShape(), 'immersive');
  assert.ok(await page.focalFullWidth(), 'focal is not full width');
});

Then('the focal slot renders the content in the reading shape', async function (this: E2EWorld) {
  const { page } = home(this, 'Matthew');
  assert.equal(await page.focalShape(), 'reading');
});

Then('the legs sit in a rail beside the content', async function (this: E2EWorld) {
  const { page } = home(this, 'Matthew');
  assert.ok(await page.legsBesideContent(), 'legs are not beside the reading column');
});

// --- legs ---

Then(
  'the atom home shows the legs {string}, {string}, {string}, {string}',
  async function (this: E2EWorld, a: string, b: string, c: string, d: string) {
    const { page } = home(this, 'Matthew');
    const expected: Record<string, 'holds' | 'lives' | 'governed' | 'from'> = {
      'Who holds it': 'holds',
      'Where this lives': 'lives',
      "How it's governed": 'governed',
      'Where it came from': 'from',
    };
    for (const label of [a, b, c, d]) {
      const leg = expected[label];
      assert.ok(leg, `unknown leg label ${label}`);
      assert.ok(await page.legVisible(leg), `leg ${label} not visible`);
      assert.ok((await page.legText(leg)).includes(label));
    }
  }
);

Then('the leg {string} is present', async function (this: E2EWorld, label: string) {
  const { page } = home(this, 'Matthew');
  assert.ok((await page.bodyText()).includes(label));
});

Then(
  'the leg "Who holds it" reads the holding sentence the doorway reports for {string}',
  async function (this: E2EWorld, id: string) {
    const { page } = home(this, 'Matthew');
    const doorway = this.getDoorway('alpha');
    const res = await fetch(`${doorway.url}/api/v1/resilience/${encodeURIComponent(id)}/household`);
    assert.equal(res.status, 200);
    const body = (await res.json()) as { feltStatus?: { headline: string } };
    assert.ok(body.feltStatus, 'no feltStatus on the household snapshot');
    assert.ok((await page.legText('holds')).includes(body.feltStatus.headline));
  }
);

Then(
  'the leg "Who holds it" shows the household floor as {string}',
  async function (this: E2EWorld, floor: string) {
    const { page } = home(this, 'Matthew');
    assert.ok((await page.legText('holds')).includes(`${floor} households`));
  }
);

Then(
  'the shard map and replica counts stay behind a {string} link',
  async function (this: E2EWorld, label: string) {
    const { page } = home(this, 'Matthew');
    const body = await page.bodyText();
    assert.ok(!/shard map|shards located|replica/i.test(body), 'holding detail leaked onto the home');
    const device = requirePwDevice(this, 'Matthew');
    const link = device.page.locator('[data-testid="epr-home-network-detail"]');
    assert.equal((await link.textContent())?.trim(), label);
  }
);

Given(
  '{string} is not held by any peer doorway {string} can ask',
  async function (this: E2EWorld, id: string, doorwayName: string) {
    const doorway = this.getDoorway(doorwayName);
    const res = await fetch(`${doorway.url}/db/content/${encodeURIComponent(id)}`);
    assert.equal(res.status, 404, `${id} is held on ${doorwayName} (status ${res.status}); pick an unheld fixture`);
  }
);

// --- arrival chip ---

Then('the arrival chip names {string}', async function (this: E2EWorld, label: string) {
  const { page } = home(this, 'Matthew');
  const text = await page.arrivalText();
  assert.ok(text && text.includes(label), `arrival chip: ${text}`);
});

Then('the atom home shows no arrival chip', async function (this: E2EWorld) {
  const { page } = home(this, 'Matthew');
  assert.equal(await page.arrivalText(), null);
});

// --- the gate ---

Then('the out-of-reach gate is shown for {string}', async function (this: E2EWorld, id: string) {
  const { page } = home(this, 'Matthew');
  const text = await page.gateText();
  assert.ok(text.includes("We can't reach this one from here"));
  assert.ok(text.includes(id));
});

Then('the gate names {string} as the referring resource', async function (this: E2EWorld, label: string) {
  const { page } = home(this, 'Matthew');
  assert.ok((await page.gateText()).includes(label));
});

Then('the gate offers to go back to {string}', async function (this: E2EWorld, label: string) {
  const { page } = home(this, 'Matthew');
  assert.ok((await page.gateText()).includes(`Back to ${label}`));
  assert.ok(await page.gateBackHref());
});

Then('the atom home shows no edit, affinity, or invite controls', async function (this: E2EWorld) {
  const { page } = home(this, 'Matthew');
  assert.ok(!(await page.has(EPR_HOME.YOUR_MARK)));
  assert.ok(!(await page.has('epr-home-invite-household')));
  assert.ok(!(await page.bodyText()).includes('Edit'));
});

// --- the learning lens ---

Then('the atom home offers {string}', async function (this: E2EWorld, label: string) {
  const { page } = home(this, 'Matthew');
  const device = requirePwDevice(this, 'Matthew');
  const text = await device.page.locator(`[data-testid="${EPR_HOME.OPEN_IN_BUNDLE}"]`).textContent();
  assert.ok(text?.includes(label), `lens reads: ${text}`);
});

Then(
  "following it lands in the learning app's path view for {string}",
  async function (this: E2EWorld, id: string) {
    const { page } = home(this, 'Matthew');
    await page.clickOpenInBundle();
    const device = requirePwDevice(this, 'Matthew');
    assert.ok(device.page.url().includes(`/lamad/path/${id}`), `landed on ${device.page.url()}`);
  }
);
```
(`this.getDoorway('alpha')` — confirm the accessor name in `src/framework/world.ts`; the Background step `Given doorway "alpha" at …` registers it. If the world exposes it under another name, use that name.)

- [ ] **Step 3: Drop `@wip` on scenarios 1–6 and 10 and dry-run**

Edit the feature: remove `@wip` from the tag lines of the seven frame scenarios (keep it on "The conversation opens empty…", "A message carries…", "Where people stand surfaces…").
Run from `genesis/a2o`: `npx cucumber-js --dry-run --tags '@concern:epr-atom-home' ; echo "EXIT=$?"`
Expected: `EXIT=0`, no undefined steps (the `@wip` three report as skipped/pending only).

- [ ] **Step 4: Run the seven scenarios against the local shell on live alpha data**

Terminal A, from `app/elohim-app`: `pnpm start:alpha` (local UI at `http://localhost:4200` over live alpha data; read-mostly).
Terminal B, from `genesis/a2o`:
```bash
E2E_DEVICE_MODE=playwright E2E_APP_URL=http://localhost:4200 E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host \
  npx cucumber-js --tags '@concern:epr-atom-home and not @wip' ; echo "EXIT=$?"
```
Expected: 7 passed, 0 failed. Scenario 10 needs `foundations-christian-technology` to be a `path` on alpha (it is: `/db/content/foundations-christian-technology`); if the lens click lands on the doorway rather than the dev server, assert on the URL path only (the step already does). Record the run's output tail in the commit body.

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/src/framework/pages/epr-home.page.ts genesis/a2o/src/framework/pages/index.ts genesis/a2o/steps/ui/epr-atom-home.steps.ts genesis/a2o/features/content/epr-atom-home.feature
git commit -m "test(a2o): @concern:epr-atom-home — the frame's seven scenarios run (local shell × alpha data)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_017GZH6i7cHvKanCC7R32jFh"
```

---

### Task 8: Production build, render proof, habit delta

**Files:**
- Modify: `app/elohim-app/.epr-meta/epr-atom-home.habit.md` (evidence)
- Regenerate: `genesis/manifests/habits.yaml` (`python3 .claude/scripts/habits-project.py`)

- [ ] **Step 1: Production build (the honest AOT check)**

From `app/elohim-app`: `pnpm exec ng build 2>&1 | tail -20; echo "EXIT=$?"`
Expected: `EXIT=0`. Fix any strictTemplates error in the new templates before continuing.

- [ ] **Step 2: Render proof, both shapes, both themes**

With `pnpm start:alpha` running, from `genesis/a2o`:
```bash
pnpm look http://localhost:4200/epr/evolution-of-trust --out epr-home-immersive
pnpm look http://localhost:4200/epr/succession --out epr-home-reading
pnpm look http://localhost:4200/epr/concept-bidirectional-trust --out epr-home-gate
pnpm look http://localhost:4200/epr/evolution-of-trust --scheme light --out epr-home-immersive-light
pnpm look http://localhost:4200/epr/evolution-of-trust --viewport 390x844 --out epr-home-phone
```
Read each `shot.png`. Check: no `viewer-back-home`, the simulation at full width, the legs present, the gate without controls, the light theme on linen, the phone stacking header → focal → legs. Check each `capture.json` `httpErrors` for anything the new page introduced (the three governance 404s from the old viewer must be GONE — the home does not call them).

- [ ] **Step 3: Write the evidence delta and re-project**

Prepend to the body of `app/elohim-app/.epr-meta/epr-atom-home.habit.md`:
```
DELTA <today> (LOCAL PROOF, stays RED until an app deploy renders it on alpha):
Slice 1 landed on dev — EprHomeComponent owns /epr/{id}; EprFocalComponent extracted
count-neutral (ratchet unchanged); a2o @concern:epr-atom-home 7 passed / 0 failed against the
local shell on live alpha data (<run tail>); `pnpm look` shots in
genesis/a2o/reports/look/epr-home-{immersive,reading,gate,immersive-light,phone} show the frame
with no lamad chrome. The flip to green needs the fleet render (elohim-app pipeline) — a build
number, not this note.
```
Run from repo root: `python3 .claude/scripts/habits-project.py && python3 .claude/scripts/habits-project.py --check`.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/.epr-meta/epr-atom-home.habit.md genesis/manifests/habits.yaml
git commit -m "chore(habits): epr-atom-home — Slice 1 local proof recorded; red until the fleet renders it

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_017GZH6i7cHvKanCC7R32jFh"
```

---

## Self-review

**Spec coverage.** §2 frame → Tasks 3–5; §2.1 shapes → Task 3; §2.2 legs → Task 4; §2.3 commons → deferred to the commons plan (spec §7 Slice 2), the a2o scenarios 7–9 stay `@wip`; §2.4 arrival → Task 5; §2.5 gate → Tasks 3 + 5; §2.6 phone → Task 4's grid collapse + Task 8 phone shot (the accordion rows for the legs are part of the commons plan, where the page grows long enough to need them); §3 sources → Tasks 3–5; §4.1–4.2 seams → Tasks 2–3 with the ratchet run in every task; §4.3 lens → Task 6; §4.4 Lit → not needed in Slice 1 (no Lit element is composed; the felt status is rendered natively) — the `elohim-qahal/register` import lands with the commons plan; §5 tokens → Task 1; §6 ids → Global Constraints + Tasks 3–7; §7 Slice 0/1 → this plan; §8/§9/§10 → no tasks by design.

**Gap:** spec §2.2 lists nav-context `partOf` under "Where this lives". It 404s by slug on alpha today; the leg reads relationships and metadata-named ids only. Add `EprNavContextService` to the leg when the storage side resolves it by slug (filed in spec §8).

**Placeholder scan:** none of "TBD/TODO/later/appropriate error handling"; every code step carries the code. Two verify-as-you-go instructions remain deliberately (fontsource `full.css` presence; generated `ChallengeView` field spelling) because they depend on files the executor must read, and each names the file and the fallback.

**Type consistency:** `FocalNode` (Task 2) used by content-delivery only; `EprHomeAtom`, `toAtom`, `focalShape`, `holdingWords`, `heldChip`, `StewardRow`, `shortAnchor`, `dayWords` defined in Task 3/4's model and used with the same names in Tasks 4–6; `openInBundle(contentType, id, claims?, mounts?)` defined and called identically in Task 6; test ids match Global Constraints in every task and in the page object.
