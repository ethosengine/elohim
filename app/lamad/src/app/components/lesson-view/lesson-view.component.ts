import { CommonModule } from '@angular/common';
import {
  Component,
  CUSTOM_ELEMENTS_SCHEMA,
  Input,
  Output,
  EventEmitter,
  OnChanges,
  OnDestroy,
  SecurityContext,
  SimpleChanges,
  ViewChild,
  ViewContainerRef,
  ComponentRef,
  ChangeDetectionStrategy,
  ChangeDetectorRef,
  inject,
} from '@angular/core';
import { DomSanitizer } from '@angular/platform-browser';
import { RouterModule } from '@angular/router';

// @coverage: 100.0% (2026-02-24)

import { takeUntil } from 'rxjs/operators';

import { Subject, Subscription } from 'rxjs';

import 'elohim-core/register';

import { marked } from 'marked';

import {
  LAMAD_EPR_RESOLVER,
  type ILamadEprResolver,
} from '../../interfaces/cross-pillar.interface';

import { ContentNode } from '../../models/content-node.model';
import { PathContext } from '../../models/exploration-context.model';
import {
  RendererRegistryService,
  ContentRenderer,
  InteractiveRenderer,
  RendererCompletionEvent,
} from '../../renderers/renderer-registry.service';
import { ExplorationSidebarComponent } from '../exploration-sidebar/exploration-sidebar.component';

import type { EprHead, EprRelationship } from '@elohim/storage-client';
// TODO: Quiz engine requires Perseus/React dependencies - enable when ready
// import { InlineQuizComponent, InlineQuizCompletionEvent } from '../../quiz-engine';

// Temporary stub types until quiz-engine is ready
interface InlineQuizCompletionEvent {
  streak: number;
  totalCorrect: number;
}

/**
 * LessonViewComponent - Primary atomic content display with exploration.
 *
 * This is the core component for displaying learning content within paths.
 * It combines:
 * - Dynamic content rendering (markdown, video, quiz, etc.)
 * - Related concepts panel (Wikipedia "See also" style)
 * - Mini-graph visualization of concept neighborhood
 * - Path context breadcrumbs and return navigation
 *
 * The exploration panel allows learners to discover related content
 * while maintaining their path context for easy return.
 *
 * Usage in PathNavigatorComponent:
 * ```html
 * <app-lesson-view
 *   [content]="stepView.content"
 *   [pathContext]="buildPathContext()"
 *   explorationMode="path"
 *   (exploreContent)="onExploreContent($event)"
 *   (exploreInGraph)="onExploreInGraph()"
 *   (complete)="onStepComplete($event)">
 * </app-lesson-view>
 * ```
 */
@Component({
  selector: 'app-lesson-view',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    ExplorationSidebarComponent,
    // TODO: InlineQuizComponent - requires Perseus/React dependencies
  ],
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div
      class="lesson-view"
      [class.has-path-context]="pathContext"
      [class.standalone]="explorationMode === 'standalone'"
      [class.flow]="layout === 'flow'"
    >
      <!-- Main content area -->
      <div class="lesson-main">
        <!-- Content header -->
        <header class="lesson-header">
          <div class="header-meta">
            <span class="content-type-badge">{{ getContentTypeLabel() }}</span>
            @if (content.tags.length) {
              <div class="content-tags">
                @for (tag of content.tags.slice(0, 3); track tag) {
                  <span class="tag">{{ tag }}</span>
                }
              </div>
            }
            @if (getFlags().length) {
              <div class="flag-tags">
                @for (flag of getFlags(); track flag.type) {
                  <span
                    [class]="getFlagClass(flag.type)"
                    [title]="flag.reason"
                    data-testid="lesson-content-flag"
                  >
                    {{ getFlagLabel(flag.type) }}
                  </span>
                }
              </div>
            }
            <elohim-gate-feedback-trigger
              class="lesson-feedback-trigger"
              [attr.content-id]="content.id"
              data-testid="lesson-feedback-trigger"
            ></elohim-gate-feedback-trigger>
          </div>
          <h1 class="lesson-title">
            {{ content.title || content.id }}
            <span
              class="reach-badge"
              [class]="'reach-badge reach-' + (content.reach || 'commons')"
              [title]="getReachTooltip()"
              data-testid="lesson-reach-badge"
            >
              {{ getReachIcon() }}
            </span>
            <span
              class="resilience-info"
              [title]="getResilienceTooltip()"
              data-testid="lesson-resilience-info"
            >
              {{ getResilienceIcon() }}
            </span>
          </h1>
          @if (descriptionHtml) {
            <p class="lesson-description" [innerHTML]="descriptionHtml"></p>
          }
        </header>

        <!-- Dynamic content renderer -->
        <article class="lesson-content">
          <!-- Renderer host must always be in DOM for ViewChild to work -->
          <ng-container #rendererHost></ng-container>
          <!-- Fallback shown only when no registered renderer -->
          @if (!hasRegisteredRenderer) {
            <div class="content-fallback">
              @if (isMarkdown()) {
                <div class="markdown-content" [innerHTML]="getContentString()"></div>
              } @else {
                <pre class="raw-content">{{ getContentString() }}</pre>
              }
            </div>
          }
        </article>

        <!-- Inline quiz for post-content knowledge check -->
        <!-- TODO: Enable when quiz-engine/Perseus is ready
        @if (showInlineQuiz && humanId) {
          <app-inline-quiz
            [contentId]="content.id"
            [humanId]="humanId"
            [targetStreak]="3"
            [collapseIfAchieved]="true"
            (completed)="onInlineQuizCompleted($event)"
            (attestationEarned)="onPracticedAttestation()">
          </app-inline-quiz>
        }
        -->

        <!-- Feedback Mechanism Gateway (governance at the point of content) -->
        <elohim-feedback-mechanism-gateway
          [attr.entity-type]="'content'"
          [attr.entity-id]="content.id"
        ></elohim-feedback-mechanism-gateway>

        @if (eprRelationships.length > 0) {
          <elohim-epr-relationships-panel
            [relationships]="eprRelationships"
            data-testid="lesson-relationships-panel"
          ></elohim-epr-relationships-panel>
        }
      </div>

      <!-- Shared exploration sidebar (mini-graph + related concepts + explore).
           Suppressed in standalone mode, matching the legacy
           standalone display-none rule on the old inline panel. -->
      @if (explorationMode !== 'standalone') {
        <app-exploration-sidebar
          [contentId]="content.id"
          [collapsible]="true"
          [(open)]="explorationPanelOpen"
          [compact]="true"
          [relatedLimit]="4"
          [graphDepth]="1"
          [graphHeight]="180"
          (exploreContent)="onRelatedConceptClick($event)"
          (exploreInGraph)="onExploreInGraphClick()"
        ></app-exploration-sidebar>
      }
    </div>
  `,
  styles: [
    `
      .lesson-view {
        display: flex;
        width: 100%;
        height: 100%;
        position: relative;
        overflow: hidden;
      }

      /* Main content area */
      .lesson-main {
        flex: 1;
        min-width: 0;
        display: flex;
        flex-direction: column;
        overflow-y: auto;
        padding: 1.5rem;
      }

      .lesson-header {
        margin-bottom: 1.5rem;
      }

      .header-meta {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        margin-bottom: 0.5rem;
      }

      /* Push the feedback trigger to the right edge of the header row. */
      .lesson-feedback-trigger {
        margin-left: auto;
      }

      .content-type-badge {
        display: inline-block;
        padding: 0.25rem 0.625rem;
        font-size: 0.6875rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.03em;
        background: color-mix(in srgb, var(--lamad-accent-primary) 12%, transparent);
        color: var(--lamad-accent-primary, #4f46e5);
        border-radius: var(--radius-sm, 4px);
      }

      .content-tags {
        display: flex;
        gap: 0.375rem;
      }

      .tag {
        padding: 0.125rem 0.5rem;
        font-size: 0.6875rem;
        background: var(--lamad-bg-tertiary, #e8eaed);
        color: var(--lamad-text-secondary, #5f6368);
        border-radius: var(--radius-sm, 4px);
      }

      .lesson-title {
        margin: 0 0 0.5rem;
        font-size: 1.5rem;
        font-weight: 600;
        color: var(--lamad-text-primary, #202124);
        line-height: 1.3;
        text-wrap: balance;
      }

      .reach-badge,
      .resilience-info {
        font-size: 0.45em;
        vertical-align: super;
        cursor: help;
        opacity: 0.7;
        transition: opacity 0.2s;
        margin-left: 0.2em;
        letter-spacing: -0.1em;
      }

      .reach-badge:hover,
      .resilience-info:hover {
        opacity: 1;
      }

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

      .lesson-description {
        margin: 0;
        font-size: 1rem;
        color: var(--lamad-text-secondary, #5f6368);
        line-height: 1.5;
      }

      .lesson-content {
        flex: 1;
        min-height: 0;
      }

      .content-fallback {
        line-height: 1.6;
      }

      .markdown-content {
        font-size: 1rem;
      }

      .raw-content {
        font-family: monospace;
        font-size: 0.875rem;
        background: var(--lamad-bg-secondary, #f8f9fa);
        padding: 1rem;
        border-radius: var(--radius-md, 8px);
        overflow-x: auto;
        white-space: pre-wrap;
      }

      /* Flow layout: the host scrolls the document, so this view must not be
         a scroll box of its own — an inner overflow would clip anything the
         renderer parks outside the column (the wide-screen Contents gutter)
         and would defeat sticky positioning inside it. */
      .lesson-view.flow {
        height: auto;
        overflow: visible;
      }

      .lesson-view.flow .lesson-main {
        overflow: visible;
      }

      /* The exploration sidebar's chrome (toggle/panel/backdrop/responsive
         breakpoints) lives in app-exploration-sidebar. Its host uses
         display: contents, so the inner .exploration-panel resolves its
         pinned/off-canvas positioning against this .lesson-view flex container,
         keeping the desktop rail in-flow beside .lesson-main exactly as before. */

      /* Desktop layout — drop the main column's right padding when the rail is
         pinned (path mode), matching the legacy behavior. */
      @media (min-width: 1024px) {
        .lesson-view:not(.standalone) .lesson-main {
          padding-right: 0;
        }
      }
    `,
  ],
})
export class LessonViewComponent implements OnChanges, OnDestroy {
  /** The content node to display */
  @Input({ required: true }) content!: ContentNode;

  /** Optional path context for breadcrumbs and return navigation */
  @Input() pathContext?: PathContext;

  /** Exploration mode - affects layout */
  @Input() explorationMode: 'path' | 'standalone' = 'path';

  /**
   * Scroll model. 'fill' (default) makes this view a fixed-height box whose
   * main column scrolls internally (content-viewer, focused view). 'flow' lets
   * the document scroll and keeps the view free of inner overflow.
   */
  @Input() layout: 'fill' | 'flow' = 'fill';

  /** Human ID for quiz tracking (required for inline quiz) */
  @Input() humanId?: string;

  /** Whether to show inline quiz after content */
  @Input() showInlineQuiz = false;

  /** Refresh key - when changed, triggers content reload (for focused view) */
  @Input() refreshKey?: number;

  /**
   * The host reserves a left gutter (--markdown-toc-gutter) on wide screens;
   * renderers that carry a table of contents park it there instead of in an
   * overlay. Forwarded to the renderer when it declares the input.
   */
  @Input() tocGutter = false;

  /** Emitted when user clicks on related content to explore */
  @Output() exploreContent = new EventEmitter<string>();

  /** Emitted when user wants to explore in full graph */
  @Output() exploreInGraph = new EventEmitter<void>();

  /** Emitted when an interactive renderer completes */
  @Output() complete = new EventEmitter<RendererCompletionEvent>();

  /** Emitted when inline quiz is completed */
  @Output() quizCompleted = new EventEmitter<InlineQuizCompletionEvent>();

  /** Emitted when practiced attestation is earned from inline quiz */
  @Output() practicedEarned = new EventEmitter<void>();

  /** ViewChild for dynamic renderer injection */
  @ViewChild('rendererHost', { read: ViewContainerRef, static: false })
  rendererHost!: ViewContainerRef;

  /**
   * Whether the exploration panel is open. Desktop (≥1024px — the sidebar's
   * CSS breakpoint) starts OPEN: at that width the panel renders as an in-flow
   * rail, its mobile toggle is display:none, and a closed panel would leave the
   * learner no affordance to ever open it. Mobile starts closed (off-canvas,
   * opened via the Explore toggle).
   */
  explorationPanelOpen = typeof window !== 'undefined' && window.innerWidth >= 1024;

  /** Whether we have a registered renderer for this content */
  hasRegisteredRenderer = false;

  private rendererRef: ComponentRef<ContentRenderer> | null = null;
  private rendererSubscription: Subscription | null = null;
  private readonly destroy$ = new Subject<void>();

  private readonly rendererRegistry = inject(RendererRegistryService);
  private readonly cdr = inject(ChangeDetectorRef);
  private readonly eprResolver: ILamadEprResolver = inject(LAMAD_EPR_RESOLVER);

  eprRelationships: EprRelationship[] = [];

  private readonly sanitizer = inject(DomSanitizer);
  private descriptionCache: { source: string; html: string | null } | null = null;

  /** Description with inline markdown (links, emphasis) rendered and sanitized. */
  get descriptionHtml(): string | null {
    const source = this.content?.description ?? '';
    if (!this.descriptionCache || this.descriptionCache.source !== source) {
      this.descriptionCache = { source, html: this.renderDescription(source) };
    }
    return this.descriptionCache.html;
  }

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['content'] && this.content) {
      // Load renderer after view updates
      setTimeout(() => this.loadRenderer(), 0);
      // Load EPR relationships (protocol-level relationships from EPR Head)
      this.loadEprRelationships(this.content.id);
    }
    // Handle refresh trigger (for focused view mode)
    if (changes['refreshKey'] && !changes['refreshKey'].firstChange) {
      setTimeout(() => this.loadRenderer(), 0);
    }
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
    this.destroyRenderer();
  }

  /**
   * Handle click on related concept.
   */
  onRelatedConceptClick(conceptId: string): void {
    this.exploreContent.emit(conceptId);
  }

  /**
   * Handle explore in full graph button.
   */
  onExploreInGraphClick(): void {
    this.exploreInGraph.emit();
  }

  /**
   * Handle inline quiz completion.
   */
  onInlineQuizCompleted(event: InlineQuizCompletionEvent): void {
    this.quizCompleted.emit(event);
  }

  /**
   * Handle practiced attestation earned from inline quiz.
   */
  onPracticedAttestation(): void {
    this.practicedEarned.emit();
  }

  /**
   * Get display label for content type.
   */
  getContentTypeLabel(): string {
    const labels: Record<string, string> = {
      epic: 'Epic',
      feature: 'Feature',
      scenario: 'Scenario',
      concept: 'Concept',
      simulation: 'Simulation',
      video: 'Video',
      assessment: 'Assessment',
      'discovery-assessment': 'Self-Discovery', // Enneagram, learning style, etc.
      organization: 'Organization',
      'book-chapter': 'Chapter',
      tool: 'Tool',
      role: 'Role',
      path: 'Path',
    };
    return labels[this.content.contentType] || this.content.contentType;
  }

  /** Reach icon — concentric circles metaphor. */
  getReachIcon(): string {
    const reach = (this.content?.reach as string) || 'commons';
    switch (reach) {
      case 'private':
      case 'self':
        return '\u{1F512}';
      case 'intimate':
        return '\u{25C9}';
      case 'trusted':
        return '\u{25CE}';
      case 'familiar':
      case 'invited':
        return '\u{25CB}\u{25CF}';
      case 'community':
      case 'local':
      case 'neighborhood':
      case 'municipal':
        return '\u{25CE}\u{25CB}';
      default:
        return '\u{25CB}\u{25CB}\u{25CB}';
    }
  }

  /** Reach tooltip. */
  getReachTooltip(): string {
    const reach = (this.content?.reach as string) || 'commons';
    const desc: Record<string, string> = {
      commons: 'Commons — accessible to everyone',
      public: 'Public — accessible to everyone',
      community: 'Community — requires collective membership',
      familiar: 'Familiar — requires shared collective with steward',
      trusted: 'Trusted — requires relationship with steward',
      intimate: 'Intimate — requires mutual intimate relationship',
      private: 'Private — creator only',
      self: 'Private — creator only',
    };
    return desc[reach] || `Reach: ${reach}`;
  }

  /** Resilience icon — stewardship health. */
  getResilienceIcon(): string {
    const count = this.content?.stewardedBy?.length || 0;
    if (count >= 3) return '\u{1F7E2}';
    if (count >= 1) return '\u{1F7E1}';
    return '\u{26AA}';
  }

  /** Resilience tooltip. */
  getResilienceTooltip(): string {
    if (!this.content?.stewardedBy?.length) return 'No stewards assigned';
    const stewards = this.content.stewardedBy
      .sort((a, b) => b.affinity - a.affinity)
      .map(s => `${s.humanId} (${s.role}, ${Math.round(s.affinity * 100)}%)`)
      .join(', ');
    return `Stewards: ${stewards}`;
  }

  /** Content flags as subtle tags. */
  getFlags(): { type: string; reason: string; flaggedAt: string }[] {
    return this.content?.flags || [];
  }

  getFlagLabel(type: string): string {
    const labels: Record<string, string> = {
      disputed: 'Disputed',
      outdated: 'Outdated',
      'appeal-pending': 'Appeal Pending',
      'under-review': 'Under Review',
      'partial-revocation': 'Partial Revocation',
    };
    return labels[type] || type;
  }

  getFlagClass(type: string): string {
    return `flag-tag flag-${type}`;
  }

  /**
   * Get content as string for fallback rendering.
   */
  getContentString(): string {
    if (!this.content?.content) return '';
    const content = this.content.content;
    return typeof content === 'string' ? content : JSON.stringify(content, null, 2);
  }

  /**
   * Check if content is markdown.
   */
  isMarkdown(): boolean {
    return this.content?.contentFormat === 'markdown';
  }

  /**
   * Content descriptions are authored as markdown (seed data carries
   * `[label](url)` links); render the inline subset and sanitize the result.
   */
  private renderDescription(description: string | undefined): string | null {
    const text = (description ?? '').trim();
    if (!text) return null;
    const html = marked.parseInline(text, { async: false });
    return this.sanitizer.sanitize(SecurityContext.HTML, html) || null;
  }

  /**
   * Load the appropriate renderer for the content.
   */
  private loadRenderer(): void {
    if (!this.content || !this.rendererHost) {
      this.hasRegisteredRenderer = false;
      this.cdr.markForCheck();
      return;
    }

    // Clean up previous renderer
    this.destroyRenderer();
    this.rendererHost.clear();

    // Get the renderer component
    const rendererComponent = this.rendererRegistry.getRenderer(this.content);

    if (!rendererComponent) {
      this.hasRegisteredRenderer = false;
      this.cdr.markForCheck();
      return;
    }

    this.hasRegisteredRenderer = true;

    // Create the renderer
    this.rendererRef = this.rendererHost.createComponent(rendererComponent);
    this.rendererRef.setInput('node', this.content);

    // Set embedded mode if supported
    if ('embedded' in this.rendererRef.instance) {
      this.rendererRef.setInput('embedded', true);
    }
    if ('tocGutter' in this.rendererRef.instance) {
      this.rendererRef.setInput('tocGutter', this.tocGutter);
    }

    // Subscribe to completion events if interactive
    const instance = this.rendererRef.instance as InteractiveRenderer;
    if (instance.complete) {
      this.rendererSubscription = instance.complete.subscribe(event => {
        this.complete.emit(event);
      });
    }

    this.cdr.markForCheck();
  }

  /**
   * Load EPR relationships from the EPR Head for this content.
   */
  private loadEprRelationships(contentId: string): void {
    this.eprRelationships = [];
    this.eprResolver
      .resolveEprHead(contentId)
      .pipe(takeUntil(this.destroy$))
      .subscribe(head => {
        this.eprRelationships = (head?.relationships ?? []) as EprRelationship[];
        this.cdr.markForCheck();
      });
  }

  /**
   * Clean up the renderer.
   */
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
}
