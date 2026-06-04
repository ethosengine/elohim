import { CommonModule } from '@angular/common';
import {
  ChangeDetectionStrategy,
  Component,
  OnDestroy,
  OnInit,
  inject,
  signal,
} from '@angular/core';

import { PeerHouseholdCardComponent } from '@app/elohim/components/peer-household-card/peer-household-card.component';

import { PeerTopologyService } from '../../services/peer-topology.service';

@Component({
  selector: 'app-shefa-peer-topology',
  standalone: true,
  imports: [CommonModule, PeerHouseholdCardComponent],
  template: `
    <section class="peer-topology" data-testid="peer-topology-page">
      <header><h2>Your peer households</h2></header>

      @if (topology.topology(); as v) {
        <div class="summary" data-testid="peer-topology-summary">
          {{ v.edges.length }} peer households · {{ v.reciprocationCount }} reciprocating
        </div>

        @for (edge of v.edges; track edge.householdId) {
          <div
            class="peer-row"
            role="button"
            tabindex="0"
            data-testid="peer-household-expand"
            [attr.aria-expanded]="isExpanded(edge.householdId)"
            [attr.aria-label]="
              (edge.displayName ?? edge.householdId) +
              '. Activate to ' +
              (isExpanded(edge.householdId) ? 'collapse' : 'expand') +
              ' household details.'
            "
            (click)="toggleExpanded(edge.householdId)"
            (keydown.enter)="toggleExpanded(edge.householdId)"
            (keydown.space)="toggleExpanded(edge.householdId); $event.preventDefault()"
          >
            <app-peer-household-card [edge]="edge"></app-peer-household-card>
            @if (isExpanded(edge.householdId)) {
              <dl class="peer-detail" data-testid="peer-household-detail-panel">
                <dt>Last sync</dt>
                <dd data-testid="peer-detail-last-sync">
                  {{ edge.lastSyncSec != null ? edge.lastSyncSec + 's ago' : 'never' }}
                </dd>
                <dt>They host of mine</dt>
                <dd data-testid="peer-detail-hosted-by-them">{{ edge.myCidsHostedByThem }}</dd>
                <dt>I host of theirs</dt>
                <dd data-testid="peer-detail-hosted-by-me">{{ edge.theirCidsHostedByMe }}</dd>
                <dt>Critical for me</dt>
                <dd data-testid="peer-detail-critical-for-me">
                  {{ edge.isCriticalForMe ? 'yes' : 'no' }}
                </dd>
                <dt>I am critical for them</dt>
                <dd data-testid="peer-detail-critical-for-them">
                  {{ edge.iAmCriticalForThem ? 'yes' : 'no' }}
                </dd>
              </dl>
            }
          </div>
        }

        @if (v.resilienceCliffs.length > 0) {
          <div class="cliff" data-testid="peer-topology-cliff">
            ⚠ resilience cliff: {{ v.resilienceCliffs.length }} household(s) hold sole-replica
            content
          </div>
        }

        <button
          type="button"
          (click)="toggleDetails()"
          data-testid="peer-topology-show-details-toggle"
        >
          [{{ showDetails() ? 'hide' : 'show' }} details]
        </button>

        @if (showDetails()) {
          <pre data-testid="peer-topology-detail-json">{{ v | json }}</pre>
        }
      }
    </section>
  `,
  styles: [
    `
      .peer-topology {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
        padding: 1rem;
      }
      .cliff {
        color: var(--health-critical, #c30);
      }
      .peer-row {
        cursor: pointer;
      }
      .peer-row:focus-visible {
        outline: 2px solid var(--focus-ring, #1a73e8);
        outline-offset: 2px;
      }
      .peer-detail {
        display: grid;
        grid-template-columns: max-content 1fr;
        gap: 0.1rem 0.75rem;
        margin: 0 0 0.5rem 0.5rem;
        padding-left: 0.5rem;
        border-left: 2px solid #ddd;
        font-size: 0.8rem;
      }
      .peer-detail dt {
        font-weight: 600;
        color: var(--text-secondary, #666);
      }
      .peer-detail dd {
        margin: 0;
      }
      pre {
        font-size: 0.75rem;
        background: #f6f6f6;
        padding: 0.5rem;
      }
    `,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PeerTopologyComponent implements OnInit, OnDestroy {
  protected readonly topology = inject(PeerTopologyService);
  protected readonly showDetails = signal(false);
  private readonly expandedHouseholds = signal<ReadonlySet<string>>(new Set());
  private stopPolling?: () => void;

  ngOnInit(): void {
    void this.topology.getMyPeerTopology();
  }

  ngOnDestroy(): void {
    this.stopPolling?.();
  }

  startPolling(intervalMs = 5000): void {
    this.stopPolling?.();
    this.stopPolling = this.topology.startPolling(intervalMs);
  }

  toggleDetails(): void {
    this.showDetails.update(v => !v);
  }

  isExpanded(householdId: string): boolean {
    return this.expandedHouseholds().has(householdId);
  }

  toggleExpanded(householdId: string): void {
    this.expandedHouseholds.update(prev => {
      const next = new Set(prev);
      if (next.has(householdId)) {
        next.delete(householdId);
      } else {
        next.add(householdId);
      }
      return next;
    });
  }
}
