/**
 * Doorway Landing Component
 *
 * Public-facing community landing page for the doorway.
 * Shows doorway status, community stats, federated peers,
 * and action links (sign in, register, operator dashboard).
 */

import { Component, OnInit, inject, signal, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import { environment } from '../../../environments/environment';

/** Health endpoint response */
interface HealthResponse {
  status: string;
  version?: string;
  uptime?: number;
}

/** Status endpoint response */
interface StatusResponse {
  name: string;
  region: string | null;
  humansServed: number;
  contentAvailable: number;
  federatedPeers: number;
}

/** Federation doorway summary for landing */
interface FederationPeer {
  id: string;
  url: string;
  region?: string;
  status: string;
}

/** Federation doorways response */
interface FederationResponse {
  doorways: FederationPeer[];
  self_id: string | null;
  total: number;
}

type LoadingState = 'loading' | 'ready' | 'error';

@Component({
  selector: 'app-doorway-landing',
  standalone: true,
  imports: [CommonModule, RouterLink],
  template: `
    <div class="landing">
      <!-- Header -->
      <header class="landing-header">
        <img src="/threshold/images/elohim_logo_light.png" alt="Elohim" class="logo" />
        <div class="header-text">
          <h1>{{ doorwayName() }}</h1>
          @if (status()) {
            <div class="status-row">
              <span class="status-indicator" [class]="healthStatus()"></span>
              <span class="status-text">{{ healthStatus() | titlecase }}</span>
              @if (status()?.region) {
                <span class="region-badge">{{ status()?.region }}</span>
              }
            </div>
          }
        </div>
      </header>

      <!-- Loading -->
      @if (state() === 'loading') {
        <div class="loading-state">
          <div class="spinner"></div>
          <p>Connecting to doorway...</p>
        </div>
      }

      <!-- Error -->
      @if (state() === 'error') {
        <div class="error-state">
          <p>Unable to reach this doorway</p>
          <button class="btn-secondary" (click)="load()">Retry</button>
        </div>
      }

      <!-- Content -->
      @if (state() === 'ready') {
        <!-- Community Stats -->
        <section class="community-stats">
          <div class="stat">
            <span class="stat-value">{{ status()?.humansServed ?? 0 }}</span>
            <span class="stat-label">Humans Served</span>
          </div>
          <div class="stat">
            <span class="stat-value">{{ status()?.contentAvailable ?? 0 }}</span>
            <span class="stat-label">Content Available</span>
          </div>
          <div class="stat">
            <span class="stat-value">{{ peers().length }}</span>
            <span class="stat-label">Federated Doorways</span>
          </div>
        </section>

        <!-- Federation Peers -->
        @if (peers().length > 0) {
          <section class="federation-section">
            <h2>Federation Network</h2>
            <div class="peer-grid">
              @for (peer of displayPeers(); track peer.id) {
                <div class="peer-card" [class.self]="peer.id === selfId()">
                  <span class="peer-status" [class]="peer.status"></span>
                  <div class="peer-info">
                    <span class="peer-name">{{ peerName(peer) }}</span>
                    @if (peer.region) {
                      <span class="peer-region">{{ peer.region }}</span>
                    }
                  </div>
                  @if (peer.id === selfId()) {
                    <span class="self-badge">You are here</span>
                  }
                </div>
              }
            </div>
            @if (peers().length > 6) {
              <p class="peer-overflow">
                and {{ peers().length - 6 }} more doorways
              </p>
            }
          </section>
        }

        <!-- Actions -->
        <section class="actions">
          <a routerLink="/login" class="btn-primary" data-testid="landing-sign-in">Sign In</a>
          <a routerLink="/register" class="btn-secondary" data-testid="landing-create-account">Create Account</a>
          <a routerLink="/dashboard" class="btn-link" data-testid="landing-dashboard">Operator Dashboard</a>
        </section>
      }

      <!-- Footer -->
      <footer class="landing-footer">
        <p>Powered by the Elohim Protocol</p>
        @if (version()) {
          <p class="version">v{{ version() }}</p>
        }
      </footer>
    </div>
  `,
  styleUrl: './doorway-landing.component.css',
})
export class DoorwayLandingComponent implements OnInit {
  private readonly http = inject(HttpClient);
  private readonly baseUrl = environment.doorwayUrl ?? '';

  readonly state = signal<LoadingState>('loading');
  readonly health = signal<HealthResponse | null>(null);
  readonly status = signal<StatusResponse | null>(null);
  readonly peers = signal<FederationPeer[]>([]);
  readonly selfId = signal<string | null>(null);

  readonly healthStatus = computed(() => {
    return this.health()?.status ?? 'unknown';
  });

  readonly doorwayName = computed(() => {
    return this.status()?.name ?? window.location.hostname;
  });

  readonly version = computed(() => {
    return this.health()?.version ?? null;
  });

  /** Show at most 6 peers in the grid */
  readonly displayPeers = computed(() => {
    return this.peers().slice(0, 6);
  });

  ngOnInit(): void {
    this.load();
  }

  async load(): Promise<void> {
    this.state.set('loading');

    try {
      const [healthRes, statusRes, federationRes] = await Promise.allSettled([
        firstValueFrom(this.http.get<HealthResponse>(`${this.baseUrl}/health`)),
        firstValueFrom(this.http.get<StatusResponse>(`${this.baseUrl}/status`)),
        firstValueFrom(this.http.get<FederationResponse>(`${this.baseUrl}/api/v1/federation/doorways`)),
      ]);

      if (healthRes.status === 'fulfilled') {
        this.health.set(healthRes.value);
      }

      if (statusRes.status === 'fulfilled') {
        this.status.set(statusRes.value);
      }

      if (federationRes.status === 'fulfilled') {
        this.peers.set(federationRes.value.doorways);
        this.selfId.set(federationRes.value.self_id);
      }

      this.state.set('ready');
    } catch {
      this.state.set('error');
    }
  }

  peerName(peer: FederationPeer): string {
    try {
      return new URL(peer.url).hostname;
    } catch {
      return peer.id;
    }
  }
}
