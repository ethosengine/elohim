import { CommonModule, NgTemplateOutlet } from '@angular/common';
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

import { Subject, forkJoin, of, from } from 'rxjs';

import { ResilienceService, type ResilienceView } from '@app/lamad/services/resilience.service';

import { EPR_RESOLUTION_PROVIDER } from '../../providers/epr-resolution.provider';

import type { EprRelationship } from '../../models/epr-head.model';
import type { EprHeadResolution } from 'elohim-core';

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

/**
 * Guard a resilience payload before rendering it.
 *
 * Resilience is a best-effort enrichment: through the `start:alpha` dev proxy
 * the `/api/v1/resilience/{id}` route is unavailable and can answer with a
 * truthy-but-malformed body (SPA fallback / partial view) that lacks
 * `stewardship`. Reading `stewardship.stewardCount` off that threw
 * `Cannot read properties of undefined (reading 'stewardCount')` and blanked
 * the whole card. The badge is shown ONLY when every field the template and
 * tooltip read is a real number; otherwise resilience is treated as absent and
 * the card renders fully from head data alone.
 */
function isRenderableResilience(r: unknown): r is ResilienceView {
  if (!r || typeof r !== 'object') return false;
  const v = r as Partial<ResilienceView>;
  return (
    typeof v.stewardship?.stewardCount === 'number' &&
    typeof v.distribution?.distinctPeers === 'number' &&
    typeof v.health?.canSurviveFailures === 'number'
  );
}

function resilienceTitle(resilience: ResilienceView): string {
  // Enrich the badge tooltip with the fetched-but-otherwise-unused distribution
  // and health fields. All values are household-honest: at a single-household
  // scale distinctPeers / canSurviveFailures are small (often 0/1) — we surface
  // whatever the wire reports, never a fabricated multi-peer figure.
  const parts = [
    `Stewards: ${resilience.stewardship.stewardCount}`,
    `Status: ${resilience.health.status}`,
  ];

  const { distinctPeers, shardsWithLocations, totalShards } = resilience.distribution;
  if (distinctPeers > 0) {
    parts.push(`Distinct peers: ${distinctPeers}`);
  }
  if (totalShards > 0) {
    // k-of-n shard placement: how many shards have a known location.
    parts.push(`Shards placed: ${shardsWithLocations}/${totalShards}`);
  }
  parts.push(`Survives ${resilience.health.canSurviveFailures} failure(s)`);

  return parts.join(' · ');
}

// ── Component ─────────────────────────────────────────────────────────────────

@Component({
  selector: 'app-epr-relationship-card',
  standalone: true,
  imports: [CommonModule, RouterModule, NgTemplateOutlet],
  changeDetection: ChangeDetectionStrategy.OnPush,
  styles: [
    `
      .epr-rel-card {
        display: block;
        text-decoration: none;
        color: inherit;
        border: 1px solid var(--epr-card-border, var(--lamad-border, #dde1e7));
        border-radius: 8px;
        padding: 10px 12px;
        transition: box-shadow 0.15s ease;
        background: var(--epr-card-bg, var(--lamad-bg-secondary, #fff));
      }
      .epr-rel-card:hover {
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
      }
      .epr-rel-card__type {
        font-size: 0.7rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.04em;
        color: var(--epr-card-type-color, var(--lamad-text-tertiary, #6b7280));
        margin-bottom: 2px;
      }
      .epr-rel-card__title {
        font-size: 0.92rem;
        font-weight: 600;
        color: var(--epr-card-title-color, var(--lamad-text-primary, #1a202c));
        margin-bottom: 2px;
        overflow: hidden;
        display: -webkit-box;
        -webkit-line-clamp: 2;
        -webkit-box-orient: vertical;
      }
      .epr-rel-card__desc {
        font-size: 0.8rem;
        color: var(--epr-card-desc-color, var(--lamad-text-tertiary, #6b7280));
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
        color: var(
          --epr-resilience-green,
          #16a34a
        ); /* a11y-color-ok: semantic resilience-status palette (non-text indicator) */
      }
      .epr-rel-card__badge--resilience-yellow {
        color: var(
          --epr-resilience-yellow,
          #ca8a04
        ); /* a11y-color-ok: semantic resilience-status palette (non-text indicator) */
      }
      .epr-rel-card__badge--resilience-none {
        color: var(
          --epr-resilience-none,
          #9ca3af
        ); /* a11y-color-ok: semantic resilience-status palette (non-text indicator) */
      }
      .epr-rel-card__badge--peers {
        font-size: 0.72rem;
        color: var(--epr-card-desc-color, var(--lamad-text-tertiary, #6b7280));
      }
    `,
  ],
  template: `
    <ng-template #cardBody>
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
          @if (distinctPeers > 0) {
            <span
              class="epr-rel-card__badge epr-rel-card__badge--peers"
              data-testid="epr-rel-card-peers"
              [title]="'Distinct peers holding shards: ' + distinctPeers"
              [attr.aria-label]="distinctPeers + ' distinct peers'"
            >
              ⛁ {{ distinctPeers }}
            </span>
          }
        }
      </div>
    </ng-template>

    <a *ngIf="route" class="epr-rel-card" data-testid="epr-relationship-card" [routerLink]="route">
      <ng-container *ngTemplateOutlet="cardBody"></ng-container>
    </a>
    <a *ngIf="!route" class="epr-rel-card" data-testid="epr-relationship-card" [href]="href ?? ''">
      <ng-container *ngTemplateOutlet="cardBody"></ng-container>
    </a>
  `,
})
export class EprRelationshipCardComponent implements OnChanges, OnDestroy {
  @Input({ required: true }) relationship!: EprRelationship;

  // ── State ──────────────────────────────────────────────────────────────────

  title = '';
  description: string | null = null;
  reach: string | null = null;
  route: string[] | null = null;
  href: string | null = null;
  label = '';
  reachIconValue = '·';
  resilience: ResilienceView | null = null;
  resilienceIconValue = '○';
  resilienceTitleValue = '';
  /** Distinct peers holding shards (ResilienceView.distribution.distinctPeers).
   *  Surfaced as a subtle count beside the resilience glyph; the template hides
   *  the badge entirely when this is 0 so household-scale cards stay honest. */
  distinctPeers = 0;

  // ── DI ────────────────────────────────────────────────────────────────────

  private readonly resolution = inject(EPR_RESOLUTION_PROVIDER);
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
    this.route = null;
    this.href = `/epr/${encodeURIComponent(rel.target)}`; // route-literal-ok: pre-resolution universal /epr/{id} fallback href (resolver overwrites with the minted href on success), not a raw literal mint
    this.description = null;
    this.reach = null;
    this.reachIconValue = '·';
    this.resilience = null;
    this.resilienceIconValue = '○';
    this.resilienceTitleValue = '';
    this.distinctPeers = 0;
  }

  private resolveRelationship(): void {
    const target = this.relationship.target;

    forkJoin({
      // Head-first resolution via the ambient provider: title / description /
      // reach come from the anonymous-safe EPR Head, so a reach-gated body
      // (403/404) never blanks the card — it renders legibly from head alone.
      head: from(this.resolution.resolveHead(target)).pipe(
        catchError(() => of<EprHeadResolution>({ state: 'error' }))
      ),
      resilience: this.resilienceService
        .getContentResilience(target)
        .pipe(catchError(() => of(null))),
    })
      .pipe(takeUntil(this.inputChange$), takeUntil(this.destroy$))
      .subscribe(({ head, resilience }) => {
        // Only a fully `resolved` head lights the card face — unchanged from the
        // prior `resolve()` semantics (degraded heads fall back to the target).
        const preview = head.state === 'resolved' ? head.head : undefined;
        if (preview) {
          this.title = preview.title;
          this.description = preview.description ?? null;
          this.reach = preview.reach ?? null;
          this.route = preview.route ?? null;
          this.href = preview.href ?? `/epr/${encodeURIComponent(target)}`; // route-literal-ok: universal /epr/{id} fallback when the head carries no minted href, not a raw literal mint
        } else {
          this.title = target;
          this.route = null;
          this.href = `/epr/${encodeURIComponent(target)}`; // route-literal-ok: unresolved-target universal /epr/{id} fallback href (head not resolved), not a raw literal mint
        }

        this.reachIconValue = reachIcon(this.reach);

        // Resilience is a strictly-optional enrichment: only a well-formed
        // payload lights the steward badge; anything else leaves it hidden.
        if (isRenderableResilience(resilience)) {
          this.resilience = resilience;
          this.resilienceIconValue = resilienceIcon(resilience.stewardship.stewardCount);
          this.resilienceTitleValue = resilienceTitle(resilience);
          this.distinctPeers = resilience.distribution.distinctPeers ?? 0;
        }

        this.cdr.markForCheck();
      });
  }
}
