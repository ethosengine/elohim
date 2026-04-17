import { CommonModule } from '@angular/common';
import {
  ChangeDetectionStrategy,
  ChangeDetectorRef,
  Component,
  Input,
  OnChanges,
  OnDestroy,
  SimpleChanges,
  inject,
} from '@angular/core';
import { RouterModule } from '@angular/router';

import { catchError, takeUntil } from 'rxjs/operators';

import { Subject, forkJoin, of } from 'rxjs';

import { ResilienceService, type ResilienceView } from '@app/lamad/services/resilience.service';

import { EprResolverService } from '../../services/epr-resolver.service';

import type { EprRelationship } from '../../models/epr-head.model';

// ── Type label map ────────────────────────────────────────────────────────────

const TYPE_LABELS: Record<string, string> = {
  PREREQUISITE: 'Prerequisite',
  TEACHES: 'Teaches',
  CONTAINS: 'Contains',
  REFERENCES: 'References',
};

function typeLabel(type: string): string {
  return TYPE_LABELS[type] ?? type.charAt(0).toUpperCase() + type.slice(1).toLowerCase();
}

// ── Reach icons ───────────────────────────────────────────────────────────────

function reachIcon(reach: string | undefined | null): string {
  switch (reach) {
    case 'commons':
    case 'public':
      return '◉';
    case 'community':
      return '◎';
    case 'trusted':
      return '◍';
    case 'personal':
    case 'private':
      return '○';
    default:
      return '·';
  }
}

// ── Resilience helpers ────────────────────────────────────────────────────────

function resilienceIcon(stewardCount: number): string {
  if (stewardCount >= 3) return '●';
  if (stewardCount >= 1) return '◐';
  return '○';
}

function resilienceTitle(resilience: ResilienceView): string {
  return `Stewards: ${resilience.stewardship.stewardCount} · Status: ${resilience.health.status}`;
}

// ── Component ─────────────────────────────────────────────────────────────────

@Component({
  selector: 'app-epr-relationship-card',
  standalone: true,
  imports: [CommonModule, RouterModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  styles: [
    `
      .epr-rel-card {
        display: block;
        text-decoration: none;
        color: inherit;
        border: 1px solid var(--epr-card-border, #dde1e7);
        border-radius: 8px;
        padding: 10px 12px;
        transition: box-shadow 0.15s ease;
        background: var(--epr-card-bg, #fff);
      }
      .epr-rel-card:hover {
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
      }
      .epr-rel-card__type {
        font-size: 0.7rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.04em;
        color: var(--epr-card-type-color, #6b7280);
        margin-bottom: 2px;
      }
      .epr-rel-card__title {
        font-size: 0.92rem;
        font-weight: 600;
        color: var(--epr-card-title-color, #1a202c);
        margin-bottom: 2px;
        overflow: hidden;
        display: -webkit-box;
        -webkit-line-clamp: 2;
        -webkit-box-orient: vertical;
      }
      .epr-rel-card__desc {
        font-size: 0.8rem;
        color: var(--epr-card-desc-color, #6b7280);
        overflow: hidden;
        display: -webkit-box;
        -webkit-line-clamp: 2;
        -webkit-box-orient: vertical;
        margin-bottom: 6px;
      }
      .epr-rel-card__meta {
        display: flex;
        gap: 8px;
        align-items: center;
        margin-top: 4px;
      }
      .epr-rel-card__badge {
        font-size: 0.78rem;
        cursor: default;
      }
      .epr-rel-card__badge--resilience-green {
        color: var(--epr-resilience-green, #16a34a);
      }
      .epr-rel-card__badge--resilience-yellow {
        color: var(--epr-resilience-yellow, #ca8a04);
      }
      .epr-rel-card__badge--resilience-none {
        color: var(--epr-resilience-none, #9ca3af);
      }
    `,
  ],
  template: `
    <a class="epr-rel-card" data-testid="epr-relationship-card" [routerLink]="route">
      <div class="epr-rel-card__type" data-testid="epr-rel-card-type">{{ label }}</div>
      <div class="epr-rel-card__title" data-testid="epr-rel-card-title">{{ title }}</div>
      @if (description) {
        <div class="epr-rel-card__desc">{{ description }}</div>
      }
      <div class="epr-rel-card__meta">
        <span
          class="epr-rel-card__badge"
          data-testid="epr-rel-card-reach"
          [title]="reach ?? ''"
          [attr.aria-label]="'Reach: ' + (reach ?? 'unknown')"
        >
          {{ reachIconValue }}
        </span>
        @if (resilience) {
          <span
            class="epr-rel-card__badge"
            [class.epr-rel-card__badge--resilience-green]="resilience.stewardship.stewardCount >= 3"
            [class.epr-rel-card__badge--resilience-yellow]="
              resilience.stewardship.stewardCount >= 1 && resilience.stewardship.stewardCount < 3
            "
            [class.epr-rel-card__badge--resilience-none]="resilience.stewardship.stewardCount === 0"
            data-testid="epr-rel-card-resilience"
            [title]="resilienceTitleValue"
          >
            {{ resilienceIconValue }}
          </span>
        }
      </div>
    </a>
  `,
})
export class EprRelationshipCardComponent implements OnChanges, OnDestroy {
  @Input({ required: true }) relationship!: EprRelationship;

  // ── State ──────────────────────────────────────────────────────────────────

  title = '';
  description: string | null = null;
  reach: string | null = null;
  route: string[] = [];
  label = '';
  reachIconValue = '·';
  resilience: ResilienceView | null = null;
  resilienceIconValue = '○';
  resilienceTitleValue = '';

  // ── DI ────────────────────────────────────────────────────────────────────

  private readonly resolver = inject(EprResolverService);
  private readonly resilienceService = inject(ResilienceService);
  private readonly cdr = inject(ChangeDetectorRef);

  // ── Cleanup ───────────────────────────────────────────────────────────────

  /** Emits on destroy to cancel in-flight subscriptions. */
  private readonly destroy$ = new Subject<void>();
  /** Emits whenever `relationship` changes to cancel the previous resolution. */
  private readonly inputChange$ = new Subject<void>();

  // ── Lifecycle ─────────────────────────────────────────────────────────────

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['relationship']) {
      this.inputChange$.next();
      this.reset();
      this.resolveRelationship();
    }
  }

  ngOnDestroy(): void {
    this.inputChange$.complete();
    this.destroy$.next();
    this.destroy$.complete();
  }

  // ── Internal ──────────────────────────────────────────────────────────────

  private reset(): void {
    const rel = this.relationship;
    this.label = typeLabel(rel.type);
    this.title = rel.target;
    this.route = ['/resource', rel.target];
    this.description = null;
    this.reach = null;
    this.reachIconValue = '·';
    this.resilience = null;
    this.resilienceIconValue = '○';
    this.resilienceTitleValue = '';
  }

  private resolveRelationship(): void {
    const target = this.relationship.target;

    forkJoin({
      content: this.resolver.resolve(target).pipe(catchError(() => of(null))),
      resilience: this.resilienceService
        .getContentResilience(target)
        .pipe(catchError(() => of(null))),
    })
      .pipe(takeUntil(this.inputChange$), takeUntil(this.destroy$))
      .subscribe(({ content, resilience }) => {
        if (content) {
          this.title = content.content.title;
          this.description = content.content.description ?? null;
          this.reach = (content.content as { reach?: string }).reach ?? null;
          this.route = content.route;
        } else {
          this.title = target;
          this.route = ['/resource', target];
        }

        this.reachIconValue = reachIcon(this.reach);

        if (resilience) {
          this.resilience = resilience;
          this.resilienceIconValue = resilienceIcon(resilience.stewardship.stewardCount);
          this.resilienceTitleValue = resilienceTitle(resilience);
        }

        this.cdr.markForCheck();
      });
  }
}
