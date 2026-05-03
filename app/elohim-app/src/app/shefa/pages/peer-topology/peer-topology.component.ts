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
          <app-peer-household-card [edge]="edge"></app-peer-household-card>
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
}
