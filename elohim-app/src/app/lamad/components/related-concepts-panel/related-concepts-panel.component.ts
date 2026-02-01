import { CommonModule } from '@angular/common';
import {
  Component,
  Input,
  Output,
  EventEmitter,
  OnChanges,
  SimpleChanges,
  ChangeDetectionStrategy,
  ChangeDetectorRef,
  OnDestroy,
} from '@angular/core';

// @coverage: 4.2% (2026-01-31)

import { takeUntil } from 'rxjs/operators';

import { Subject } from 'rxjs';

import { RelatedConceptsResult } from '../../models/exploration-context.model';
import { RelatedConceptsService } from '../../services/related-concepts.service';
import { ConceptCardComponent } from '../concept-card/concept-card.component';

/**
 * RelatedConceptsPanelComponent - Wikipedia "See also" style panel.
 *
 * Displays related concepts organized by relationship type:
 * - Prerequisites: Concepts that should be understood first
 * - Related Topics: Generally related concepts
 * - Go Deeper: Extensions and advanced topics
 * - Parent/Child: Hierarchical relationships
 *
 * Usage:
 * ```html
 * <app-related-concepts-panel
 *   [contentId]="currentContentId"
 *   [showHierarchy]="true"
 *   (navigate)="onConceptClick($event)">
 * </app-related-concepts-panel>
 * ```
 */
@Component({
  selector: 'app-related-concepts-panel',
  standalone: true,
  imports: [CommonModule, ConceptCardComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="related-concepts-panel" [class.loading]="isLoading" [class.empty]="isEmpty">
      @if (isLoading) {
        <div class="loading-state">
          <span class="loading-spinner"></span>
          <span>Loading related concepts...</span>
        </div>
      }

      @if (!isLoading && isEmpty) {
        <div class="empty-state">
          <span class="empty-icon">🔗</span>
          <span>No related concepts found</span>
        </div>
      }

      @if (!isLoading && !isEmpty) {
        <!-- Prerequisites Section -->
        @if (result.prerequisites.length > 0) {
          <section class="relationship-section prerequisites">
            <h4 class="section-header">
              <span class="section-icon">📚</span>
              Prerequisites
              <span class="section-count">({{ result.prerequisites.length }})</span>
            </h4>
            <div class="section-content">
              @for (concept of result.prerequisites; track concept.id) {
                <app-concept-card
                  [concept]="concept"
                  relationshipType="PREREQUISITE"
                  [compact]="compact"
                  (navigate)="onNavigate($event)"
                ></app-concept-card>
              }
            </div>
          </section>
        }

        <!-- Parent Concepts Section -->
        @if (showHierarchy && result.parents.length > 0) {
          <section class="relationship-section parents">
            <h4 class="section-header">
              <span class="section-icon">⬆️</span>
              Part Of
              <span class="section-count">({{ result.parents.length }})</span>
            </h4>
            <div class="section-content">
              @for (concept of result.parents; track concept.id) {
                <app-concept-card
                  [concept]="concept"
                  relationshipType="CONTAINS"
                  [compact]="compact"
                  (navigate)="onNavigate($event)"
                ></app-concept-card>
              }
            </div>
          </section>
        }

        <!-- Related Topics Section -->
        @if (result.related.length > 0) {
          <section class="relationship-section related">
            <h4 class="section-header">
              <span class="section-icon">🔗</span>
              Related Topics
              <span class="section-count">({{ result.related.length }})</span>
            </h4>
            <div class="section-content">
              @for (concept of result.related; track concept.id) {
                <app-concept-card
                  [concept]="concept"
                  relationshipType="RELATES_TO"
                  [compact]="compact"
                  (navigate)="onNavigate($event)"
                ></app-concept-card>
              }
            </div>
          </section>
        }

        <!-- Extensions Section -->
        @if (result.extensions.length > 0) {
          <section class="relationship-section extensions">
            <h4 class="section-header">
              <span class="section-icon">🚀</span>
              Go Deeper
              <span class="section-count">({{ result.extensions.length }})</span>
            </h4>
            <div class="section-content">
              @for (concept of result.extensions; track concept.id) {
                <app-concept-card
                  [concept]="concept"
                  relationshipType="EXTENDS"
                  [compact]="compact"
                  (navigate)="onNavigate($event)"
                ></app-concept-card>
              }
            </div>
          </section>
        }

        <!-- Child Concepts Section -->
        @if (showHierarchy && result.children.length > 0) {
          <section class="relationship-section children">
            <h4 class="section-header">
              <span class="section-icon">⬇️</span>
              Contains
              <span class="section-count">({{ result.children.length }})</span>
            </h4>
            <div class="section-content">
              @for (concept of result.children; track concept.id) {
                <app-concept-card
                  [concept]="concept"
                  relationshipType="CONTAINS"
                  [compact]="compact"
                  (navigate)="onNavigate($event)"
                ></app-concept-card>
              }
            </div>
          </section>
        }
      }
    </div>
  `,
  styles: [
    `
      .related-concepts-panel {
        display: flex;
        flex-direction: column;
        gap: 1rem;
      }

      .loading-state,
      .empty-state {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        gap: 0.5rem;
        padding: 2rem 1rem;
        color: var(--text-secondary, #5f6368);
        text-align: center;
      }

      .loading-spinner {
        width: 1.5rem;
        height: 1.5rem;
        border: 2px solid var(--border-color, #e9ecef);
        border-top-color: var(--primary, #4285f4);
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
      }

      @keyframes spin {
        to {
          transform: rotate(360deg);
        }
      }

      .empty-icon {
        font-size: 2rem;
        opacity: 0.5;
      }

      .relationship-section {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
      }

      .section-header {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        margin: 0;
        padding: 0.25rem 0;
        font-size: 0.8125rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.03em;
        color: var(--text-secondary, #5f6368);
        border-bottom: 1px solid var(--border-color, #e9ecef);
      }

      .section-icon {
        font-size: 1rem;
      }

      .section-count {
        font-weight: 400;
        opacity: 0.7;
      }

      .section-content {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
      }

      /* Section-specific styling */
      .prerequisites .section-header {
        color: var(--warning-text, #b06000);
      }

      .extensions .section-header {
        color: var(--success-text, #137333);
      }

      .related .section-header {
        color: var(--info-text, #174ea6);
      }

      .parents .section-header,
      .children .section-header {
        color: var(--text-secondary, #5f6368);
      }
    `,
  ],
})
export class RelatedConceptsPanelComponent implements OnChanges, OnDestroy {
  /** The content ID to find related concepts for */
  @Input({ required: true }) contentId!: string;

  /** Whether to show hierarchical relationships (parents/children) */
  @Input() showHierarchy = true;

  /** Compact mode for cards */
  @Input() compact = false;

  /** Maximum concepts per section */
  @Input() limit = 5;

  /** Emitted when a concept is clicked */
  @Output() navigate = new EventEmitter<string>();

  /** Loading state */
  isLoading = true;

  /** Query result */
  result: RelatedConceptsResult = {
    prerequisites: [],
    extensions: [],
    related: [],
    children: [],
    parents: [],
    allRelationships: [],
  };

  private readonly destroy$ = new Subject<void>();

  constructor(
    private readonly relatedConceptsService: RelatedConceptsService,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['contentId'] && this.contentId) {
      this.loadRelatedConcepts();
    }
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  /**
   * Check if result is empty (no related concepts).
   */
  get isEmpty(): boolean {
    return (
      this.result.prerequisites.length === 0 &&
      this.result.extensions.length === 0 &&
      this.result.related.length === 0 &&
      this.result.children.length === 0 &&
      this.result.parents.length === 0
    );
  }

  /**
   * Handle navigation to a concept.
   */
  onNavigate(conceptId: string): void {
    this.navigate.emit(conceptId);
  }

  /**
   * Load related concepts for the current content ID.
   */
  private loadRelatedConcepts(): void {
    this.isLoading = true;
    this.cdr.markForCheck();

    this.relatedConceptsService
      .getRelatedConcepts(this.contentId, { limit: this.limit })
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: result => {
          this.result = result;
          this.isLoading = false;
          this.cdr.markForCheck();
        },
        error: _err => {
          this.isLoading = false;
          this.cdr.markForCheck();
        },
      });
  }
}
