import { CommonModule } from '@angular/common';
import { Component, Input, Output, EventEmitter, ChangeDetectionStrategy } from '@angular/core';

// @coverage: 5.9% (2026-01-31)

import { ContentNode } from '../../models/content-node.model';
import { RelationshipType } from '../../models/exploration-context.model';

/**
 * ConceptCardComponent - Compact card for displaying a concept in lists.
 *
 * Used in the RelatedConceptsPanel to show prerequisites, related topics,
 * and extensions. Displays title, type icon, and optional mastery indicator.
 *
 * Usage:
 * ```html
 * <app-concept-card
 *   [concept]="conceptNode"
 *   [relationshipType]="'DEPENDS_ON'"
 *   [showMastery]="true"
 *   [mastered]="true"
 *   (navigate)="onConceptClick($event)">
 * </app-concept-card>
 * ```
 */
@Component({
  selector: 'app-concept-card',
  standalone: true,
  imports: [CommonModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <button
      class="concept-card"
      [class.mastered]="mastered"
      [class.compact]="compact"
      (click)="onCardClick()"
      [attr.aria-label]="'Navigate to ' + concept.title"
    >
      <span class="card-icon" [attr.aria-hidden]="true">
        {{ getContentTypeIcon() }}
      </span>

      <div class="card-content">
        <span class="card-title">{{ concept.title || concept.id }}</span>

        @if (concept.description && !compact) {
          <span class="card-description">{{ truncateDescription(concept.description) }}</span>
        }
      </div>

      @if (showMastery) {
        <span
          class="mastery-indicator"
          [class.mastered]="mastered"
          [attr.aria-label]="mastered ? 'Mastered' : 'Not yet mastered'"
        >
          {{ mastered ? '✓' : '○' }}
        </span>
      }

      @if (relationshipType) {
        <span class="relationship-badge" [class]="getRelationshipClass()">
          {{ getRelationshipLabel() }}
        </span>
      }
    </button>
  `,
  styles: [
    `
      .concept-card {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        width: 100%;
        padding: 0.75rem;
        background: var(--surface-secondary, #f8f9fa);
        border: 1px solid var(--border-color, #e9ecef);
        border-radius: var(--radius-md, 8px);
        cursor: pointer;
        transition: all 0.15s ease;
        text-align: left;
        font-family: inherit;
        font-size: inherit;
      }

      .concept-card:hover {
        background: var(--surface-hover, #f1f3f4);
        border-color: var(--primary, #4285f4);
        transform: translateX(2px);
      }

      .concept-card:focus {
        outline: 2px solid var(--primary, #4285f4);
        outline-offset: 2px;
      }

      .concept-card.mastered {
        border-left: 3px solid var(--success, #34a853);
      }

      .concept-card.compact {
        padding: 0.5rem 0.75rem;
      }

      .card-icon {
        flex-shrink: 0;
        font-size: 1.25rem;
        line-height: 1;
      }

      .card-content {
        flex: 1;
        min-width: 0;
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
      }

      .card-title {
        font-weight: 500;
        color: var(--text-primary, #202124);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
      }

      .card-description {
        font-size: 0.8125rem;
        color: var(--text-secondary, #5f6368);
        line-height: 1.3;
        display: -webkit-box;
        -webkit-line-clamp: 2;
        -webkit-box-orient: vertical;
        overflow: hidden;
      }

      .mastery-indicator {
        flex-shrink: 0;
        width: 1.25rem;
        height: 1.25rem;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 0.875rem;
        color: var(--text-tertiary, #80868b);
      }

      .mastery-indicator.mastered {
        color: var(--success, #34a853);
        font-weight: bold;
      }

      .relationship-badge {
        flex-shrink: 0;
        padding: 0.125rem 0.5rem;
        font-size: 0.6875rem;
        font-weight: 500;
        text-transform: uppercase;
        letter-spacing: 0.02em;
        border-radius: var(--radius-sm, 4px);
        background: var(--surface-tertiary, #e8eaed);
        color: var(--text-secondary, #5f6368);
      }

      .relationship-badge.prerequisite {
        background: var(--warning-surface, #fef7e0);
        color: var(--warning-text, #b06000);
      }

      .relationship-badge.extension {
        background: var(--success-surface, #e6f4ea);
        color: var(--success-text, #137333);
      }

      .relationship-badge.related {
        background: var(--info-surface, #e8f0fe);
        color: var(--info-text, #174ea6);
      }

      .relationship-badge.contains {
        background: var(--surface-tertiary, #e8eaed);
        color: var(--text-secondary, #5f6368);
      }
    `,
  ],
})
export class ConceptCardComponent {
  /** The concept to display */
  @Input({ required: true }) concept!: ContentNode;

  /** Optional relationship type for badge display */
  @Input() relationshipType?: RelationshipType;

  /** Whether to show mastery indicator */
  @Input() showMastery = false;

  /** Whether the concept is mastered */
  @Input() mastered = false;

  /** Compact mode (less padding, no description) */
  @Input() compact = false;

  /** Emitted when the card is clicked */
  @Output() navigate = new EventEmitter<string>();

  /**
   * Handle card click.
   */
  onCardClick(): void {
    this.navigate.emit(this.concept.id);
  }

  /**
   * Get icon for content type.
   */
  getContentTypeIcon(): string {
    const icons: Record<string, string> = {
      epic: '📖',
      feature: '⚡',
      scenario: '✓',
      concept: '💡',
      simulation: '🎮',
      video: '🎥',
      assessment: '📝',
      organization: '🏢',
      'book-chapter': '📚',
      tool: '🛠️',
      role: '👤',
      path: '🛤️',
    };
    return icons[this.concept.contentType] || '📄';
  }

  /**
   * Get CSS class for relationship badge.
   */
  getRelationshipClass(): string {
    if (!this.relationshipType) return '';

    const classMap: Record<string, string> = {
      PREREQUISITE: 'prerequisite',
      FOUNDATION: 'prerequisite',
      DEPENDS_ON: 'prerequisite',
      EXTENDS: 'extension',
      RELATES_TO: 'related',
      CONTAINS: 'contains',
    };

    return classMap[this.relationshipType] || '';
  }

  /**
   * Get label for relationship badge.
   */
  getRelationshipLabel(): string {
    if (!this.relationshipType) return '';

    const labelMap: Record<string, string> = {
      PREREQUISITE: 'prereq',
      FOUNDATION: 'foundation',
      DEPENDS_ON: 'depends',
      EXTENDS: 'extends',
      RELATES_TO: 'related',
      CONTAINS: 'contains',
    };

    return labelMap[this.relationshipType] || this.relationshipType.toLowerCase();
  }

  /**
   * Truncate description to reasonable length.
   */
  truncateDescription(description: string): string {
    const maxLength = 80;
    if (description.length <= maxLength) return description;
    return description.substring(0, maxLength).trim() + '...';
  }
}
