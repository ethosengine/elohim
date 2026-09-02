import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';

import { eprToUniversalHref } from '@elohim/service';

import { EprRelationship } from '../../models/epr-head.model';
import { EprRelationshipsPanelComponent } from '../epr-relationships-panel/epr-relationships-panel.component';

import { EprHomeAtom, StewardRow, dayWords, holdingWords, shortAnchor } from './epr-home.model';

import type { ResilienceSnapshotView } from '@app/generated/resilience-snapshot-view';
import type { ChallengeView } from '@elohim/storage-client/generated';

/** "concept-bidirectional-trust" → "Bidirectional trust" (label absent → humanized slug). */
export function humanizeSlug(slug: string): string {
  const words = slug
    .replace(/^(concept|fct-module-\d+)-/, '')
    .split('-')
    .filter(Boolean);
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
  readonly pips = computed(() =>
    Array.from({ length: this.holding().wants }, (_, i) => i < this.holding().has)
  );
  readonly related = computed(() =>
    this.atom().relatedIds.map(id => ({
      id,
      label: humanizeSlug(id),
      href: eprToUniversalHref({ id, tier: 'head' }),
    }))
  );
  readonly openChallenges = computed(() =>
    this.challenges().filter(c => c.state !== 'responded' && !c.respondedAt && !c.resolvedAt)
  );
  readonly anchorShort = computed(() => {
    const h = this.atom().dhtAnchorHash;
    return h ? shortAnchor(h) : null;
  });
  readonly anchorVerified = computed(() => this.atom().dhtAnchorState === 'verified');
  readonly addedOn = computed(() => dayWords(this.atom().createdAt));
  readonly updatedOn = computed(() => dayWords(this.atom().updatedAt));
  readonly rawHref = computed(() =>
    eprToUniversalHref({ id: this.atom().id, tier: 'head', subview: 'raw' })
  );
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
