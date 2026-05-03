/**
 * Topology Tab Component
 *
 * Operator-facing view of this doorway's storage stewards, federation peers,
 * projection coverage, and public surface health.
 *
 * Backed by `GET /admin/dashboard/topology` (T35), which composes
 * `DoorwayDashboardView` from cluster + federation + cache + DNS state.
 *
 * Schema source of truth: `elohim/sdk/schemas/v1/views/doorway-dashboard-view.schema.json`
 */

import { CommonModule, DecimalPipe } from '@angular/common';
import { HttpClient } from '@angular/common/http';
import { Component, OnInit, inject, signal } from '@angular/core';
import { firstValueFrom } from 'rxjs';

export type FederationDirection = 'bidirectional' | 'outbound_only' | 'inbound_only';

export interface DashboardSteward {
  peerId: string;
  archetype: string;
  displayName?: string;
  online: boolean;
  hostingCount: number;
  hopHint?: number;
}

export interface DashboardFederationPeer {
  doorwayHostname: string;
  online: boolean;
  direction: FederationDirection;
  sharedCidCount: number;
}

export interface ProjectionCoverage {
  projectedCidCount: number;
  knownCidCount: number;
  cacheHitRate24h: number;
  projectionLagMsAvg: number;
}

export interface PublicSurfaceState {
  dnsResolves: boolean;
  dnsTarget?: string;
  tlsValid: boolean;
  tlsExpiresInDays?: number;
  publicReachable: boolean;
}

export interface DoorwayDashboardView {
  doorwayHostname: string;
  storageStewards: DashboardSteward[];
  federationPeers: DashboardFederationPeer[];
  projectionCoverage: ProjectionCoverage;
  publicSurface: PublicSurfaceState;
}

@Component({
  selector: 'app-topology-tab',
  standalone: true,
  imports: [CommonModule, DecimalPipe],
  template: `
    <div class="topology-tab" data-testid="operator-topology">
      @if (loading()) {
        <div class="loading-state">
          <div class="spinner"></div>
          <p>Loading topology…</p>
        </div>
      } @else if (error()) {
        <div class="error-state">
          <p>{{ error() }}</p>
          <button type="button" (click)="refresh()" data-testid="topology-retry">Retry</button>
        </div>
      } @else if (view()) {
        @let v = view()!;
        <header class="topology-header">
          <h2 data-testid="topology-hostname">{{ v.doorwayHostname }}</h2>
          <div class="counts">
            <span data-testid="topology-stewards-count">{{ v.storageStewards.length }} storage stewards</span>
            ·
            <span data-testid="topology-federation-count">{{ v.federationPeers.length }} federation peers</span>
          </div>
        </header>

        <section class="projection-coverage">
          <h3>Projection coverage</h3>
          <div data-testid="projection-coverage">
            {{ v.projectionCoverage.projectedCidCount }} / {{ v.projectionCoverage.knownCidCount }} CIDs
            ({{ (v.projectionCoverage.cacheHitRate24h * 100) | number: '1.0-0' }}% hit rate over 24h)
          </div>
          <div class="lag" data-testid="projection-lag">
            avg projection lag: {{ v.projectionCoverage.projectionLagMsAvg }}ms
          </div>
        </section>

        <section class="public-surface">
          <h3>Public surface</h3>
          <ul>
            <li data-testid="surface-dns">
              DNS: {{ v.publicSurface.dnsResolves ? '✓ resolves' : '✗ does not resolve' }}
              @if (v.publicSurface.dnsTarget) {
                ({{ v.publicSurface.dnsTarget }})
              }
            </li>
            <li data-testid="surface-tls">
              TLS: {{ v.publicSurface.tlsValid ? '✓ valid' : '✗ invalid' }}
              @if (v.publicSurface.tlsExpiresInDays != null) {
                — expires in {{ v.publicSurface.tlsExpiresInDays }} days
              }
            </li>
            <li data-testid="surface-reachable">
              Reachability: {{ v.publicSurface.publicReachable ? '✓ reachable from public probe' : '✗ unreachable' }}
            </li>
          </ul>
        </section>

        <button
          type="button"
          (click)="toggleDetails()"
          data-testid="topology-details-toggle"
        >
          [{{ showDetails() ? 'hide' : 'show' }} details]
        </button>

        @if (showDetails()) {
          <pre data-testid="operator-topology-json">{{ v | json }}</pre>
        }
      }
    </div>
  `,
  styles: [
    `
      .topology-tab {
        display: flex;
        flex-direction: column;
        gap: 1rem;
        padding: 1rem;
      }
      .topology-header h2 {
        margin: 0;
      }
      .counts {
        color: var(--text-secondary, #555);
        font-size: 0.9rem;
      }
      ul {
        list-style: none;
        padding: 0;
        margin: 0;
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
      }
      .loading-state,
      .error-state {
        padding: 1rem;
      }
      pre {
        font-size: 0.75rem;
        background: #f6f6f6;
        padding: 0.5rem;
        overflow: auto;
      }
    `,
  ],
})
export class TopologyTabComponent implements OnInit {
  private readonly http = inject(HttpClient);

  protected readonly view = signal<DoorwayDashboardView | null>(null);
  protected readonly loading = signal(false);
  protected readonly error = signal<string | null>(null);
  protected readonly showDetails = signal(false);

  ngOnInit(): void {
    void this.refresh();
  }

  toggleDetails(): void {
    this.showDetails.update((b) => !b);
  }

  async refresh(): Promise<void> {
    this.loading.set(true);
    this.error.set(null);
    try {
      const v = await firstValueFrom(
        this.http.get<DoorwayDashboardView>('/admin/dashboard/topology'),
      );
      this.view.set(v);
    } catch (e: unknown) {
      this.error.set(e instanceof Error ? e.message : 'failed to load topology');
    } finally {
      this.loading.set(false);
    }
  }
}
