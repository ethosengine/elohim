/**
 * RecommendationListComponent — Presentational component for content recommendations.
 *
 * Renders ContentRecommendation[] as EPR-linked cards with adaptation context labels.
 * Used in two surfaces:
 * - Assessment completion summary (inline after quiz failure)
 * - Path overview (persistent panel near locked gates)
 *
 * Each recommendation renders as an <elohim-epr-link display="card"> wrapped with
 * a context label explaining WHY this content is recommended. The Lit element
 * owns resolution + popover behavior internally; this component wires the
 * EprResolverService as the resolver property and translates the Lit
 * 'navigate' CustomEvent into Angular Router navigation.
 */

import { CommonModule } from '@angular/common';
import {
  Component,
  ChangeDetectionStrategy,
  CUSTOM_ELEMENTS_SCHEMA,
  ElementRef,
  Input,
  Output,
  EventEmitter,
  AfterViewInit,
  OnDestroy,
  inject,
} from '@angular/core';
import { Router } from '@angular/router';

import { firstValueFrom } from 'rxjs';

import 'elohim-core/register';

import {
  LAMAD_EPR_RESOLVER,
  type ILamadEprResolver,
} from '../../../interfaces/cross-pillar.interface';

import type { ElohimEprLink } from 'elohim-core';

import type {
  ContentRecommendation,
  RecommendationReason,
} from '../../services/path-adaptation.service';

@Component({
  selector: 'app-recommendation-list',
  standalone: true,
  imports: [CommonModule],
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
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
            <elohim-epr-link
              [attr.epr]="'epr:' + rec.contentId"
              display="card"
            ></elohim-epr-link>
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
export class RecommendationListComponent implements AfterViewInit, OnDestroy {
  @Input() recommendations: ContentRecommendation[] = [];
  @Input() heading = 'Strengthen Your Foundations';
  @Output() dismiss = new EventEmitter<string>();

  private readonly eprResolver: ILamadEprResolver = inject(LAMAD_EPR_RESOLVER);
  private readonly router = inject(Router);
  private readonly elRef = inject<ElementRef<HTMLElement>>(ElementRef);

  private readonly navigateListener = (e: Event): void => {
    const epr = (e as CustomEvent<{ epr: string }>).detail.epr;
    this.eprResolver.resolve(epr).subscribe(resolved => {
      if (resolved?.route) {
        void this.router.navigate(resolved.route);
      }
    });
  };

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

  ngAfterViewInit(): void {
    // Wire the EprResolverService as the Lit element's resolver property on
    // each rendered <elohim-epr-link>. Property assignment must happen after
    // the element upgrades (i.e. after view init / @for-loop materializes).
    this.wireResolvers();

    // Translate the Lit 'navigate' event into Angular Router navigation.
    // Listener is delegated on the host so it catches events from any
    // <elohim-epr-link> rendered by the @for loop.
    this.elRef.nativeElement.addEventListener('navigate', this.navigateListener);
  }

  ngOnDestroy(): void {
    this.elRef.nativeElement.removeEventListener('navigate', this.navigateListener);
  }

  private wireResolvers(): void {
    const litEls = this.elRef.nativeElement.querySelectorAll<ElohimEprLink>('elohim-epr-link');
    for (const litEl of litEls) {
      litEl.resolver = async (eprRef: string) => {
        const resolved = await firstValueFrom(this.eprResolver.resolve(eprRef));
        if (!resolved) return null;
        return {
          title: resolved.content.title,
          description: resolved.content.description || undefined,
          reach: resolved.content.reach,
        };
      };
    }
  }
}
