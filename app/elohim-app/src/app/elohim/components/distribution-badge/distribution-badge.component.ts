import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, Input, inject, signal } from '@angular/core';

import { DistributionService } from '../../services/distribution.service';

import type { DistributionDetails } from '@app/generated/distribution-details';
import type { DistributionSummary } from '@app/generated/distribution-summary';

@Component({
  selector: 'app-distribution-badge',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './distribution-badge.component.html',
  styleUrls: ['./distribution-badge.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DistributionBadgeComponent {
  @Input({ required: true }) summary!: DistributionSummary;
  @Input() blobHash?: string;

  protected readonly expanded = signal(false);
  protected readonly details = signal<DistributionDetails | null>(null);
  protected readonly loadingDetails = signal(false);
  protected readonly Math = Math;

  private readonly distribution = inject(DistributionService);

  async onTooltipExpand(): Promise<void> {
    if (this.expanded()) return;
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

  reachIcon(reach: DistributionSummary['reachClass']): string {
    if (reach === 'private') return '🔒 private';
    if (reach === 'intimate') return '🔒 peer-only';
    if (reach === 'public') return '🌐 public';
    return reach;
  }

  dotIndices(count: number): number[] {
    const n = Math.min(4, count);
    return Array.from({ length: n }, (_, i) => i);
  }
}
