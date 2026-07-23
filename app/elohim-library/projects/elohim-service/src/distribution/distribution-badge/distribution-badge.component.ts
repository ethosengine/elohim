import { CommonModule } from '@angular/common';
import {
  ChangeDetectionStrategy,
  Component,
  Input,
  OnChanges,
  SimpleChanges,
  inject,
  signal,
} from '@angular/core';

import { DistributionService } from '../distribution.service';

import type { DistributionDetails, PlacementGapRow } from '../../generated/distribution-details';
import type { DistributionSummary } from '../../generated/distribution-summary';

/**
 * Minimal typed shape for a placement-gap entry as projected by C1/C2.
 * The local generated `DistributionDetails.placementGaps` is still
 * `Record<string, unknown>[]` (open-shape during bring-up). This interface
 * mirrors `PlacementGapRow` from storage-client-ts for display purposes only.
 * When the local generated type graduates to the full shape, remove this.
 *
 * TODO(rust-migration): retire once distribution-details.ts local generated
 * type uses the PlacementGapRow shape from storage-client-ts.
 */
interface GapRow {
  kind: string;
  contentId: string;
  shortfall: { target: number; observed: number };
  remediation: string | null;
}

const GAP_KIND_LABELS: Record<string, string> = {
  hub_diversity: 'Hub diversity',
  replica_count: 'Replica count',
  reach_class: 'Reach class',
};

/**
 * Schema-8 reach vocabulary (elohim/sdk/schemas/v1/enums/reach.schema.json) →
 * badge icon + label. Fallthrough in reachIcon() returns the raw string only
 * for future-proofing against an as-yet-uncovered value.
 */
const REACH_ICONS: Record<string, string> = {
  private: '🔒 private',
  self: '🔒 self',
  intimate: '🔒 peer-only',
  trusted: '🤝 trusted',
  familiar: '🤝 familiar',
  community: '🏘️ community',
  public: '🌐 public',
  commons: '🌍 commons',
};

@Component({
  selector: 'elohim-distribution-badge',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './distribution-badge.component.html',
  styleUrls: ['./distribution-badge.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DistributionBadgeComponent implements OnChanges {
  @Input({ required: true }) summary!: DistributionSummary;
  @Input() blobHash?: string;

  /**
   * Optional pre-fetched distribution details. When provided, placement-gaps
   * and per-peer rows render directly from this input without a lazy fetch.
   * Use this when the parent already holds full details (e.g. a resilience
   * panel). When absent with a blobHash, details are fetched lazily on expand.
   */
  @Input() detailsInput?: DistributionDetails;

  /**
   * Inline-axis pin for the hover tooltip. Hosts whose badge trails the line
   * (e.g. content-viewer's title row) set 'end' so the fold-down grows INTO
   * the viewport instead of off its edge — the 2026-06-12 phone regression
   * class shared with elohim-resilience-snapshot's panelAlign.
   */
  @Input() tooltipAlign: 'start' | 'end' = 'start';

  protected readonly expanded = signal(false);
  protected readonly details = signal<DistributionDetails | null>(null);
  protected readonly loadingDetails = signal(false);

  private readonly distribution = inject(DistributionService);

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['detailsInput'] && this.detailsInput !== undefined) {
      this.details.set(this.detailsInput);
    }
  }

  async onTooltipExpand(): Promise<void> {
    this.expanded.set(true);
    if (!this.blobHash || this.details()) return;
    this.loadingDetails.set(true);
    try {
      const d = await this.distribution.getDetails(this.blobHash);
      this.details.set(d);
    } finally {
      this.loadingDetails.set(false);
    }
  }

  protected onHoverLeave(): void {
    this.expanded.set(false);
  }

  /**
   * Derives a human-readable hub-count label from the diversity hint.
   * diversityHint.kind values:
   *   - 'household_archetypes' → distinct device archetypes across households
   *   - 'region_metro'         → geographically distributed
   *   - 'collective_member_count' → collective-level diversity
   *   - 'none'                 → no diversity signal
   */
  protected hubCountLabel(): string {
    const hint = this.summary.diversityHint;
    const replicas = this.summary.replicaCount;

    switch (hint.kind) {
      case 'household_archetypes':
        return `${replicas} households (archetype mix)`;
      case 'region_metro':
        return `${replicas} peers (multi-region)`;
      case 'collective_member_count':
        return `${replicas} peers (collective spread)`;
      case 'none':
      default:
        return `${replicas} peers`;
    }
  }

  protected gapLabel(gap: GapRow): string {
    return GAP_KIND_LABELS[gap.kind] ?? gap.kind;
  }

  /**
   * Cast a raw open-shape gap record to GapRow for display.
   * Safe: we only read fields if they exist, rendering '-' otherwise.
   */
  // After the schema codegen catch-up (040724c80) DistributionDetails.placementGaps
  // is now typed as PlacementGapRow[] rather than Record<string, unknown>[]. We
  // accept either shape (open-shape from older REST callers, typed shape from
  // newer codegen) and read fields defensively — the runtime guards still
  // protect against unexpected nulls.
  protected asGapRow(raw: Record<string, unknown> | PlacementGapRow): GapRow {
    const r = raw as Record<string, unknown>;
    const shortfall = r['shortfall'] as { target: number; observed: number } | undefined;
    return {
      kind: typeof r['kind'] === 'string' ? r['kind'] : 'unknown',
      contentId: typeof r['contentId'] === 'string' ? r['contentId'] : '',
      shortfall: shortfall ?? { target: 0, observed: 0 },
      remediation: typeof r['remediation'] === 'string' ? r['remediation'] : null,
    };
  }

  reachIcon(reach: DistributionSummary['reachClass']): string {
    return REACH_ICONS[reach] ?? reach;
  }

  dotIndices(count: number): number[] {
    const n = Math.min(4, count);
    return Array.from({ length: n }, (_, i) => i);
  }
}
