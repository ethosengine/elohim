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
          <a routerLink="/login" class="btn-primary">Sign In</a>
          <a routerLink="/register" class="btn-secondary">Create Account</a>
          <a routerLink="/dashboard" class="btn-link">Operator Dashboard</a>
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
  styles: [`
    .landing {
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      align-items: center;
      background: linear-gradient(135deg, #0f172a 0%, #1e293b 100%);
      padding: 2rem 1rem;
      color: #fff;
    }

    .landing-header {
      display: flex;
      align-items: center;
      gap: 1.25rem;
      margin-bottom: 3rem;
      margin-top: 2rem;
    }

    .logo {
      width: 72px;
      height: 72px;
    }

    .header-text h1 {
      font-size: 1.75rem;
      font-weight: 600;
      margin: 0 0 0.5rem;
    }

    .status-row {
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .status-indicator {
      width: 10px;
      height: 10px;
      border-radius: 50%;
      background: #9ca3af;

      &.healthy, &.ok { background: #10b981; }
      &.degraded { background: #f59e0b; }
      &.unhealthy, &.offline { background: #ef4444; }
    }

    .status-text {
      font-size: 0.875rem;
      color: rgba(255, 255, 255, 0.7);
    }

    .region-badge {
      display: inline-block;
      padding: 0.125rem 0.5rem;
      border-radius: 9999px;
      background: rgba(99, 102, 241, 0.2);
      color: #a5b4fc;
      font-size: 0.75rem;
      font-weight: 500;
    }

    .loading-state, .error-state {
      display: flex;
      flex-direction: column;
      align-items: center;
      padding: 4rem 0;
      color: rgba(255, 255, 255, 0.7);
    }

    .spinner {
      width: 40px;
      height: 40px;
      border: 3px solid rgba(255, 255, 255, 0.1);
      border-top-color: #6366f1;
      border-radius: 50%;
      animation: spin 1s linear infinite;
      margin-bottom: 1rem;
    }

    @keyframes spin {
      to { transform: rotate(360deg); }
    }

    .community-stats {
      display: flex;
      gap: 2rem;
      margin-bottom: 3rem;
    }

    .stat {
      display: flex;
      flex-direction: column;
      align-items: center;
      min-width: 120px;
      padding: 1.25rem 1.5rem;
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 0.75rem;
    }

    .stat-value {
      font-size: 2rem;
      font-weight: 700;
      color: #fff;
    }

    .stat-label {
      font-size: 0.75rem;
      color: rgba(255, 255, 255, 0.5);
      text-transform: uppercase;
      letter-spacing: 0.05em;
      margin-top: 0.25rem;
    }

    .federation-section {
      width: 100%;
      max-width: 600px;
      margin-bottom: 3rem;

      h2 {
        font-size: 1rem;
        font-weight: 500;
        color: rgba(255, 255, 255, 0.6);
        text-align: center;
        margin: 0 0 1rem;
      }
    }

    .peer-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
      gap: 0.75rem;
    }

    .peer-card {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      padding: 0.75rem 1rem;
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 0.5rem;
      transition: border-color 0.2s;

      &:hover {
        border-color: rgba(255, 255, 255, 0.2);
      }

      &.self {
        border-color: rgba(99, 102, 241, 0.4);
        background: rgba(99, 102, 241, 0.1);
      }
    }

    .peer-status {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: #9ca3af;
      flex-shrink: 0;

      &.active, &.online { background: #10b981; }
      &.degraded { background: #f59e0b; }
      &.offline { background: #ef4444; }
    }

    .peer-info {
      display: flex;
      flex-direction: column;
      min-width: 0;
    }

    .peer-name {
      font-size: 0.875rem;
      color: rgba(255, 255, 255, 0.9);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .peer-region {
      font-size: 0.6875rem;
      color: rgba(255, 255, 255, 0.4);
    }

    .self-badge {
      margin-left: auto;
      font-size: 0.625rem;
      color: #818cf8;
      white-space: nowrap;
    }

    .peer-overflow {
      text-align: center;
      font-size: 0.8125rem;
      color: rgba(255, 255, 255, 0.4);
      margin: 0.75rem 0 0;
    }

    .actions {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 0.75rem;
      width: 100%;
      max-width: 320px;
      margin-bottom: 3rem;
    }

    .btn-primary {
      display: block;
      width: 100%;
      padding: 0.875rem 1.5rem;
      background: #6366f1;
      color: white;
      border: none;
      border-radius: 0.5rem;
      font-size: 1rem;
      font-weight: 500;
      text-align: center;
      text-decoration: none;
      cursor: pointer;
      transition: background 0.2s;

      &:hover { background: #4f46e5; }
    }

    .btn-secondary {
      display: block;
      width: 100%;
      padding: 0.875rem 1.5rem;
      background: rgba(255, 255, 255, 0.1);
      color: white;
      border: 1px solid rgba(255, 255, 255, 0.2);
      border-radius: 0.5rem;
      font-size: 1rem;
      font-weight: 500;
      text-align: center;
      text-decoration: none;
      cursor: pointer;
      transition: background 0.2s;

      &:hover { background: rgba(255, 255, 255, 0.15); }
    }

    .btn-link {
      color: rgba(255, 255, 255, 0.5);
      font-size: 0.875rem;
      text-decoration: none;
      margin-top: 0.5rem;
      transition: color 0.2s;

      &:hover { color: rgba(255, 255, 255, 0.8); }
    }

    .landing-footer {
      margin-top: auto;
      text-align: center;
      padding-top: 2rem;

      p {
        color: rgba(255, 255, 255, 0.3);
        font-size: 0.75rem;
        margin: 0;
      }

      .version {
        margin-top: 0.25rem;
        font-size: 0.6875rem;
      }
    }

    @media (max-width: 600px) {
      .community-stats {
        flex-direction: column;
        gap: 0.75rem;
        width: 100%;
      }

      .stat {
        flex-direction: row;
        justify-content: space-between;
      }
    }
  `],
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
