import { ChangeDetectionStrategy, Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ResilienceSnapshotView } from '../../generated/resilience-snapshot-view';
import { ResilienceSnapshotDensity } from './resilience-snapshot.types';

@Component({
  selector: 'elohim-resilience-snapshot',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './resilience-snapshot.component.html',
  styleUrls: ['./resilience-snapshot.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ResilienceSnapshotComponent {
  @Input({ required: true }) snapshot!: ResilienceSnapshotView;
  @Input() density: ResilienceSnapshotDensity = 'icon';

  get statusClass(): string {
    switch (this.snapshot?.protectionStatus) {
      case 'protected': return 'status-protected';
      case 'partial':   return 'status-partial';
      case 'at-risk':   return 'status-at-risk';
      default:          return 'status-unknown';
    }
  }

  get regionSummary(): string {
    const rd = this.snapshot?.regionalDistribution;
    if (!rd) return '';
    const parts: string[] = [];
    if (rd.local)    parts.push(`${rd.local} local`);
    if (rd.regional) parts.push(`${rd.regional} regional`);
    if (rd.global)   parts.push(`${rd.global} global`);
    if (rd.unknown)  parts.push(`${rd.unknown} unknown region`);
    return parts.join(' · ');
  }
}
