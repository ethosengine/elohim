import {
  ChangeDetectionStrategy,
  Component,
  EventEmitter,
  Input,
  Output,
} from '@angular/core';

import { MiniGraphComponent } from '../mini-graph/mini-graph.component';
import { RelatedConceptsPanelComponent } from '../related-concepts-panel/related-concepts-panel.component';

/**
 * ExplorationSidebarComponent — the shared "Explore" sidebar.
 *
 * A thin compositional wrapper that bundles the three exploration affordances
 * (Concept Map mini-graph + Related Concepts panel + "Explore in Full Graph"
 * button) into one reusable surface, so both the path-coupled `LessonViewComponent`
 * and the standalone `ContentViewerComponent` render the SAME sidebar.
 *
 * It owns presentation/composition only — the children (`<app-mini-graph>`,
 * `<app-related-concepts-panel>`) carry the data-fetching and graph logic; this
 * wrapper just wires their inputs/outputs and shapes the off-canvas vs pinned
 * chrome. No domain truth lives here.
 *
 * Two layouts:
 * - `collapsible = true` (lesson-view): off-canvas panel driven by `open`/`openChange`,
 *   with a mobile toggle, close button, and backdrop.
 * - `collapsible = false` (content-viewer): a pinned rail that is always visible.
 *
 * Usage:
 * ```html
 * <app-exploration-sidebar
 *   [contentId]="content.id"
 *   [(open)]="explorationPanelOpen"
 *   (exploreContent)="navigateTo($event)"
 *   (exploreInGraph)="openFullGraph()">
 * </app-exploration-sidebar>
 * ```
 */
@Component({
  selector: 'app-exploration-sidebar',
  standalone: true,
  imports: [MiniGraphComponent, RelatedConceptsPanelComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (collapsible) {
      <!-- Exploration panel toggle (mobile) -->
      <button
        class="panel-toggle"
        type="button"
        (click)="toggle()"
        [attr.aria-expanded]="open"
        aria-controls="exploration-panel"
        data-testid="exploration-panel-toggle"
      >
        <span class="toggle-icon">{{ open ? '→' : '←' }}</span>
        <span class="toggle-label">{{ open ? 'Hide' : 'Explore' }}</span>
      </button>
    }

    <aside
      id="exploration-panel"
      class="exploration-panel"
      [class.collapsible]="collapsible"
      [class.pinned]="!collapsible"
      [class.collapsed]="collapsible && !open"
      data-testid="exploration-sidebar"
    >
      <div class="panel-header">
        <h2 class="panel-title">Explore</h2>
        @if (collapsible) {
          <button
            class="panel-close"
            type="button"
            (click)="close()"
            aria-label="Close exploration panel"
            data-testid="exploration-panel-close"
          >
            ×
          </button>
        }
      </div>

      <div class="panel-content">
        <!-- Mini Graph -->
        <section class="panel-section graph-section">
          <h3 class="section-title">Concept Map</h3>
          <app-mini-graph
            [focusNodeId]="contentId"
            [depth]="graphDepth"
            [height]="graphHeight"
            (nodeSelected)="onChildExplore($event)"
            (exploreRequested)="exploreInGraph.emit()"
          ></app-mini-graph>
        </section>

        <!-- Related Concepts -->
        <section class="panel-section related-section">
          <h3 class="section-title">Related Concepts</h3>
          <app-related-concepts-panel
            [contentId]="contentId"
            [showHierarchy]="true"
            [compact]="compact"
            [limit]="relatedLimit"
            (navigate)="exploreContent.emit($event)"
          ></app-related-concepts-panel>
        </section>

        <!-- Explore in Full Graph button -->
        <div class="panel-actions">
          <button
            class="btn-explore-graph"
            type="button"
            (click)="exploreInGraph.emit()"
            data-testid="exploration-explore-graph"
          >
            <span class="btn-icon">🔭</span>
            Explore in Full Graph
          </button>
        </div>
      </div>
    </aside>

    <!-- Backdrop for mobile (collapsible only) -->
    @if (collapsible && open) {
      <div class="panel-backdrop" (click)="close()" aria-hidden="true"></div>
    }
  `,
  styles: [
    `
      :host {
        display: contents;
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

      /* Exploration panel */
      .exploration-panel {
        background: var(--surface-elevated, #fff);
        border-left: 1px solid var(--border-color, #e9ecef);
        display: flex;
        flex-direction: column;
        overflow: hidden;
      }

      /* Off-canvas (collapsible) behavior */
      .exploration-panel.collapsible {
        position: fixed;
        right: 0;
        top: 0;
        bottom: 0;
        width: 320px;
        transform: translateX(100%);
        transition: transform 0.3s ease;
        z-index: 30;
      }

      .exploration-panel.collapsible:not(.collapsed) {
        transform: translateX(0);
      }

      /* Pinned rail (non-collapsible) — always visible, in-flow */
      .exploration-panel.pinned {
        position: relative;
        width: 320px;
        flex-shrink: 0;
        height: 100%;
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

      /* Desktop layout (collapsible panel becomes in-flow when open) */
      @media (min-width: 1024px) {
        .panel-toggle {
          display: none;
        }

        .exploration-panel.collapsible {
          position: relative;
          transform: none;
          width: 320px;
          flex-shrink: 0;
        }

        .exploration-panel.collapsible.collapsed {
          display: none;
        }
      }

      /* Mobile layout */
      @media (max-width: 1023px) {
        .exploration-panel.collapsible {
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
    `,
  ],
})
export class ExplorationSidebarComponent {
  /** The content node id whose neighborhood + related concepts we explore. */
  @Input({ required: true }) contentId!: string;

  /**
   * Off-canvas (lesson-view) vs pinned rail (content-viewer).
   * When false, the panel is always visible and the toggle/close/backdrop are
   * suppressed.
   */
  @Input() collapsible = true;

  /** Panel open state (collapsible mode only). Supports `[(open)]`. */
  @Input() open = false;
  @Output() openChange = new EventEmitter<boolean>();

  /** Forwarded to the related-concepts panel's compact layout. */
  @Input() compact = true;

  /** Max related concepts to surface. */
  @Input() relatedLimit = 4;

  /** Mini-graph traversal depth. */
  @Input() graphDepth = 1;

  /** Mini-graph render height (px). */
  @Input() graphHeight = 180;

  /**
   * A neighboring content was chosen (mini-graph node click OR related-concepts
   * navigate). Emits the target contentId for the host to navigate to.
   */
  @Output() exploreContent = new EventEmitter<string>();

  /** The learner asked to open the full graph explorer. */
  @Output() exploreInGraph = new EventEmitter<void>();

  /** Toggle the off-canvas panel and propagate the new open state. */
  toggle(): void {
    this.setOpen(!this.open);
  }

  /** Close the off-canvas panel. */
  close(): void {
    this.setOpen(false);
  }

  private setOpen(next: boolean): void {
    this.open = next;
    this.openChange.emit(next);
  }

  /**
   * Route a mini-graph node selection to the shared exploreContent stream.
   * (`nodeSelected` already emits the target contentId string.)
   */
  onChildExplore(contentId: string): void {
    this.exploreContent.emit(contentId);
  }
}
