import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, Input } from '@angular/core';

import type { PeerHouseholdEdge } from '@app/generated/peer-household-edge';

@Component({
  selector: 'elohim-peer-household-card',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="peer-card" [class.dark]="!edge.online" data-testid="peer-household-card">
      <header>
        <strong>{{ edge.displayName ?? edge.householdId }}</strong>
        <span data-testid="peer-card-status">{{ edge.online ? 'reachable' : 'dark' }}</span>
      </header>
      <div class="reciprocity">
        hosts {{ edge.myCidsHostedByThem }} of mine ·
        I host {{ edge.theirCidsHostedByMe }} of theirs ·
        <span
          data-testid="peer-card-net-diff"
          [class.positive]="(edge.netDiff ?? 0) > 0"
          [class.negative]="(edge.netDiff ?? 0) < 0"
        >
          net {{ formatDiff(edge.netDiff ?? 0) }}
        </span>
      </div>
      @if (edge.isCriticalForMe) {
        <div class="critical" data-testid="peer-card-critical-for-me">
          ⚠ critical-for-me — they hold sole-replica content for you
        </div>
      }
      @if (edge.iAmCriticalForThem) {
        <div class="critical" data-testid="peer-card-critical-for-them">
          I am critical-for-them — sole external replica of their content
        </div>
      }
    </div>
  `,
  styles: [
    `
      .peer-card {
        padding: 0.5rem 0;
        border-bottom: 1px solid #eee;
      }
      .peer-card.dark {
        opacity: 0.55;
      }
      header {
        display: flex;
        gap: 0.5rem;
        align-items: center;
      }
      .reciprocity .positive {
        color: var(--diff-positive, #2a7);
      }
      .reciprocity .negative {
        color: var(--diff-negative, #c30);
      }
      .critical {
        color: var(--health-critical, #c30);
        font-size: 0.85rem;
      }
    `,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PeerHouseholdCardComponent {
  @Input({ required: true }) edge!: PeerHouseholdEdge;

  formatDiff(n: number): string {
    return n > 0 ? `+${n}` : `${n}`;
  }
}
