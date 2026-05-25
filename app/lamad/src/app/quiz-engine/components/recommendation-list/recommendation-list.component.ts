/**
 * RecommendationListComponent — Presentational component for content recommendations.
 *
 * Renders ContentRecommendation[] as EPR-linked cards with adaptation context labels.
 * Used in two surfaces:
 * - Assessment completion summary (inline after quiz failure)
 * - Path overview (persistent panel near locked gates)
 *
 * Each recommendation renders as an <app-epr-link display="card"> wrapped with
 * a context label explaining WHY this content is recommended.
 */

import { CommonModule } from '@angular/common';
import { Component, ChangeDetectionStrategy, Input, Output, EventEmitter } from '@angular/core';

import { EprLinkComponent } from '@app/elohim/components/epr-link/epr-link.component';

import type {
  ContentRecommendation,
  RecommendationReason,
} from '../../services/path-adaptation.service';

@Component({
  selector: 'app-recommendation-list',
  standalone: true,
  imports: [CommonModule, EprLinkComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (recommendations.length > 0) {
      <section class="recommendation-list" data-testid="recommendation-list">
        <h5 class="recommendation-heading" data-testid="recommendation-heading">
          {{ heading }}
        </h5>
        @for (rec of recommendations; track rec.contentId; let i = $index) {
          <div class="recommendation-item" [attr.data-testid]="'recommendation-item-' + i">
            <span class="recommendation-context" [attr.data-testid]="'recommendation-context-' + i">
              {{ getContextLabel(rec) }}
            </span>
            <app-epr-link [epr]="'epr:' + rec.contentId" display="card"></app-epr-link>
            <button
              class="recommendation-dismiss"
              data-testid="recommendation-dismiss"
              [attr.data-testid]="'recommendation-dismiss-' + i"
              (click)="dismiss.emit(rec.contentId)"
              aria-label="Dismiss recommendation"
            >
              Dismiss
            </button>
          </div>
        }
      </section>
    }
  `,
  styles: [
    `
      .recommendation-list {
        margin: 1.5rem 0;
      }

      .recommendation-heading {
        font-size: 0.9rem;
        font-weight: 600;
        color: #374151;
        margin: 0 0 0.75rem;
      }

      .recommendation-item {
        margin-bottom: 1rem;
        position: relative;
      }

      .recommendation-context {
        display: block;
        font-size: 0.8rem;
        color: #6b7280;
        margin-bottom: 0.25rem;
        font-style: italic;
      }

      .recommendation-dismiss {
        position: absolute;
        top: 0.25rem;
        right: 0.25rem;
        background: none;
        border: none;
        font-size: 0.75rem;
        color: #9ca3af;
        cursor: pointer;
        padding: 0.25rem 0.5rem;
      }
      .recommendation-dismiss:hover {
        color: #6b7280;
      }
    `,
  ],
})
export class RecommendationListComponent {
  @Input() recommendations: ContentRecommendation[] = [];
  @Input() heading = 'Strengthen Your Foundations';
  @Output() dismiss = new EventEmitter<string>();

  private readonly contextLabels: Record<RecommendationReason, string> = {
    prerequisite_gap: 'Foundation for concepts you need',
    reinforcement: 'Another angle on this topic',
    struggled_with_concept: 'Review this before retrying',
    exploration_interest: 'You might find this interesting',
    advanced_option: 'Ready for a deeper dive',
  };

  getContextLabel(rec: ContentRecommendation): string {
    return this.contextLabels[rec.reason] ?? '';
  }
}
