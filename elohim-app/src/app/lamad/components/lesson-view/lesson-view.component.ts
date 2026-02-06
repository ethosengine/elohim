import { CommonModule } from '@angular/common';
import {
  Component,
  Input,
  Output,
  EventEmitter,
  OnChanges,
  OnDestroy,
  SimpleChanges,
  ViewChild,
  ViewContainerRef,
  ComponentRef,
  ChangeDetectionStrategy,
  ChangeDetectorRef,
} from '@angular/core';
import { RouterModule } from '@angular/router';

// @coverage: 100.0% (2026-02-05)

import { Subject, Subscription } from 'rxjs';

import { ContentNode } from '../../models/content-node.model';
import { PathContext } from '../../models/exploration-context.model';
import {
  RendererRegistryService,
  ContentRenderer,
  InteractiveRenderer,
  RendererCompletionEvent,
} from '../../renderers/renderer-registry.service';
import { MiniGraphComponent } from '../mini-graph/mini-graph.component';
import { RelatedConceptsPanelComponent } from '../related-concepts-panel/related-concepts-panel.component';
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
    RelatedConceptsPanelComponent,
    MiniGraphComponent,
    // TODO: InlineQuizComponent - requires Perseus/React dependencies
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div
      class="lesson-view"
      [class.has-path-context]="pathContext"
      [class.standalone]="explorationMode === 'standalone'"
      [class.panel-open]="explorationPanelOpen"
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
          </div>
          <h1 class="lesson-title">{{ content.title || content.id }}</h1>
          @if (content.description) {
            <p class="lesson-description">{{ content.description }}</p>
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
      </div>

      <!-- Exploration panel toggle (mobile) -->
      <button
        class="panel-toggle"
        (click)="toggleExplorationPanel()"
        [attr.aria-expanded]="explorationPanelOpen"
        aria-controls="exploration-panel"
      >
        <span class="toggle-icon">{{ explorationPanelOpen ? '→' : '←' }}</span>
        <span class="toggle-label">{{ explorationPanelOpen ? 'Hide' : 'Explore' }}</span>
      </button>

      <!-- Exploration sidebar -->
      <aside
        id="exploration-panel"
        class="exploration-panel"
        [class.collapsed]="!explorationPanelOpen"
      >
        <div class="panel-header">
          <h2 class="panel-title">Explore</h2>
          <button
            class="panel-close"
            (click)="toggleExplorationPanel()"
            aria-label="Close exploration panel"
          >
            ×
          </button>
        </div>

        <div class="panel-content">
          <!-- Mini Graph -->
          <section class="panel-section graph-section">
            <h3 class="section-title">Concept Map</h3>
            <app-mini-graph
              [focusNodeId]="content.id"
              [depth]="1"
              [height]="180"
              (nodeSelected)="onGraphNodeClick($event)"
              (exploreRequested)="onExploreInGraphClick()"
            ></app-mini-graph>
          </section>

          <!-- Related Concepts -->
          <section class="panel-section related-section">
            <h3 class="section-title">Related Concepts</h3>
            <app-related-concepts-panel
              [contentId]="content.id"
              [showHierarchy]="true"
              [compact]="true"
              [limit]="4"
              (navigate)="onRelatedConceptClick($event)"
            ></app-related-concepts-panel>
          </section>

          <!-- Explore in Full Graph button -->
          <div class="panel-actions">
            <button class="btn-explore-graph" (click)="onExploreInGraphClick()">
              <span class="btn-icon">🔭</span>
              Explore in Full Graph
            </button>
          </div>
        </div>
      </aside>

      <!-- Backdrop for mobile -->
      @if (explorationPanelOpen) {
        <div class="panel-backdrop" (click)="toggleExplorationPanel()" aria-hidden="true"></div>
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

      .content-type-badge {
        display: inline-block;
        padding: 0.25rem 0.625rem;
        font-size: 0.6875rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.03em;
        background: var(--primary-surface, #e8f0fe);
        color: var(--primary-text, #174ea6);
        border-radius: var(--radius-sm, 4px);
      }

      .content-tags {
        display: flex;
        gap: 0.375rem;
      }

      .tag {
        padding: 0.125rem 0.5rem;
        font-size: 0.6875rem;
        background: var(--surface-tertiary, #e8eaed);
        color: var(--text-secondary, #5f6368);
        border-radius: var(--radius-sm, 4px);
      }

      .lesson-title {
        margin: 0 0 0.5rem;
        font-size: 1.5rem;
        font-weight: 600;
        color: var(--text-primary, #202124);
        line-height: 1.3;
      }

      .lesson-description {
        margin: 0;
        font-size: 1rem;
        color: var(--text-secondary, #5f6368);
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
        background: var(--surface-secondary, #f8f9fa);
        padding: 1rem;
        border-radius: var(--radius-md, 8px);
        overflow-x: auto;
        white-space: pre-wrap;
      }

      /* Panel toggle button */
      .panel-toggle {
        position: fixed;
        right: 0;
        top: 50%;
        transform: translateY(-50%);
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 0.25rem;
        padding: 0.75rem 0.5rem;
        background: var(--primary, #4285f4);
        color: white;
        border: none;
        border-radius: var(--radius-md, 8px) 0 0 var(--radius-md, 8px);
        cursor: pointer;
        z-index: 20;
        transition: all 0.2s ease;
      }

      .panel-toggle:hover {
        padding-right: 0.75rem;
      }

      .toggle-icon {
        font-size: 1rem;
      }

      .toggle-label {
        writing-mode: vertical-rl;
        text-orientation: mixed;
        font-size: 0.75rem;
        font-weight: 500;
      }

      .lesson-view.panel-open .panel-toggle {
        right: 320px;
      }

      /* Exploration panel */
      .exploration-panel {
        position: fixed;
        right: 0;
        top: 0;
        bottom: 0;
        width: 320px;
        background: var(--surface-elevated, #fff);
        border-left: 1px solid var(--border-color, #e9ecef);
        display: flex;
        flex-direction: column;
        transform: translateX(100%);
        transition: transform 0.3s ease;
        z-index: 30;
        overflow: hidden;
      }

      .exploration-panel:not(.collapsed) {
        transform: translateX(0);
      }

      .panel-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 1rem;
        border-bottom: 1px solid var(--border-color, #e9ecef);
      }

      .panel-title {
        margin: 0;
        font-size: 1rem;
        font-weight: 600;
        color: var(--text-primary, #202124);
      }

      .panel-close {
        width: 2rem;
        height: 2rem;
        display: flex;
        align-items: center;
        justify-content: center;
        background: none;
        border: none;
        font-size: 1.5rem;
        color: var(--text-secondary, #5f6368);
        cursor: pointer;
        border-radius: var(--radius-sm, 4px);
      }

      .panel-close:hover {
        background: var(--surface-hover, #f1f3f4);
      }

      .panel-content {
        flex: 1;
        overflow-y: auto;
        padding: 1rem;
        display: flex;
        flex-direction: column;
        gap: 1.5rem;
      }

      .panel-section {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
      }

      .section-title {
        margin: 0;
        font-size: 0.75rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: var(--text-tertiary, #80868b);
      }

      .panel-actions {
        margin-top: auto;
        padding-top: 1rem;
        border-top: 1px solid var(--border-color, #e9ecef);
      }

      .btn-explore-graph {
        width: 100%;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 0.5rem;
        padding: 0.75rem 1rem;
        background: var(--surface-secondary, #f8f9fa);
        border: 1px solid var(--border-color, #e9ecef);
        border-radius: var(--radius-md, 8px);
        color: var(--text-primary, #202124);
        font-weight: 500;
        cursor: pointer;
        transition: all 0.15s ease;
      }

      .btn-explore-graph:hover {
        background: var(--primary, #4285f4);
        border-color: var(--primary, #4285f4);
        color: white;
      }

      .btn-icon {
        font-size: 1.125rem;
      }

      /* Backdrop */
      .panel-backdrop {
        display: none;
      }

      /* Desktop layout (panel always visible) */
      @media (min-width: 1024px) {
        .panel-toggle {
          display: none;
        }

        .exploration-panel {
          position: relative;
          transform: none;
          width: 320px;
          flex-shrink: 0;
        }

        .exploration-panel.collapsed {
          display: none;
        }

        .lesson-view:not(.standalone) .lesson-main {
          padding-right: 0;
        }
      }

      /* Mobile layout */
      @media (max-width: 1023px) {
        .exploration-panel {
          width: 100%;
          max-width: 360px;
        }

        .panel-backdrop {
          display: block;
          position: fixed;
          inset: 0;
          background: rgba(0, 0, 0, 0.4);
          z-index: 25;
          opacity: 0;
          animation: fadeIn 0.2s ease forwards;
        }

        @keyframes fadeIn {
          to {
            opacity: 1;
          }
        }
      }

      /* Standalone mode (no sidebar) */
      .lesson-view.standalone .exploration-panel {
        display: none;
      }

      .lesson-view.standalone .panel-toggle {
        display: none;
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

  /** Human ID for quiz tracking (required for inline quiz) */
  @Input() humanId?: string;

  /** Whether to show inline quiz after content */
  @Input() showInlineQuiz = false;

  /** Refresh key - when changed, triggers content reload (for focused view) */
  @Input() refreshKey?: number;

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

  /** Whether exploration panel is open (mobile) */
  explorationPanelOpen = false;

  /** Whether we have a registered renderer for this content */
  hasRegisteredRenderer = false;

  private rendererRef: ComponentRef<ContentRenderer> | null = null;
  private rendererSubscription: Subscription | null = null;
  private readonly destroy$ = new Subject<void>();

  constructor(
    private readonly rendererRegistry: RendererRegistryService,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['content'] && this.content) {
      // Load renderer after view updates
      setTimeout(() => this.loadRenderer(), 0);
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
   * Toggle exploration panel visibility (mobile).
   */
  toggleExplorationPanel(): void {
    this.explorationPanelOpen = !this.explorationPanelOpen;
  }

  /**
   * Handle click on related concept.
   */
  onRelatedConceptClick(conceptId: string): void {
    this.exploreContent.emit(conceptId);
  }

  /**
   * Handle click on graph node.
   */
  onGraphNodeClick(nodeId: string): void {
    this.exploreContent.emit(nodeId);
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
