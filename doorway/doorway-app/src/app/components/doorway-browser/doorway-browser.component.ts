/**
 * Doorway Browser Component
 *
 * Displays a grid of federated doorways fetched from the current doorway's
 * federation API. Users can pick a different doorway to authenticate with,
 * which redirects them to that doorway's /auth/authorize endpoint with
 * the original OAuth params preserved.
 *
 * Route: /threshold/doorways?client_id=...&redirect_uri=...&state=...
 */

import { Component, OnInit, inject, signal, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { ActivatedRoute } from '@angular/router';
import {
  DoorwayFederationService,
  FederationDoorwayWithHealth,
  OAuthParams,
} from '../../services/doorway-federation.service';

@Component({
  selector: 'app-doorway-browser',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './doorway-browser.component.html',
  styleUrl: './doorway-browser.component.css',
})
export class DoorwayBrowserComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly federation = inject(DoorwayFederationService);

  // State
  readonly loading = signal(true);
  readonly error = signal('');
  readonly doorways = signal<FederationDoorwayWithHealth[]>([]);
  readonly selfId = signal<string | null>(null);
  readonly oauthParams = signal<OAuthParams | null>(null);

  // Filters
  searchQuery = '';
  regionFilter = '';

  // Computed
  readonly regions = computed(() => {
    const all = this.doorways().map(d => d.region).filter(Boolean) as string[];
    return [...new Set(all)].sort();
  });

  readonly filteredDoorways = computed(() => {
    let result = this.doorways();
    const query = this.searchQuery.toLowerCase().trim();
    if (query) {
      result = result.filter(
        d =>
          d.id.toLowerCase().includes(query) ||
          d.url.toLowerCase().includes(query) ||
          (d.region ?? '').toLowerCase().includes(query)
      );
    }
    if (this.regionFilter) {
      result = result.filter(d => d.region === this.regionFilter);
    }
    return result;
  });

  readonly loginUrl = computed(() => {
    const params = this.oauthParams();
    if (params) {
      const searchParams = new URLSearchParams({
        client_id: params.clientId,
        redirect_uri: params.redirectUri,
        response_type: params.responseType,
        state: params.state,
      });
      if (params.scope) {
        searchParams.set('scope', params.scope);
      }
      return `/threshold/login?${searchParams.toString()}`;
    }
    return '/threshold/login';
  });

  ngOnInit(): void {
    this.parseOAuthParams();
    this.loadDoorways();
  }

  private parseOAuthParams(): void {
    const params = this.route.snapshot.queryParams;
    const clientId = params['client_id'];
    const redirectUri = params['redirect_uri'];
    const responseType = params['response_type'];
    const state = params['state'];
    const scope = params['scope'];

    if (clientId && redirectUri && state) {
      this.oauthParams.set({
        clientId,
        redirectUri,
        responseType: responseType ?? 'code',
        state,
        scope,
      });
    }
  }

  private async loadDoorways(): Promise<void> {
    this.loading.set(true);
    this.error.set('');

    try {
      const response = await this.federation.loadDoorways();
      this.selfId.set(response.self_id);

      // Convert to health-check type (initially unknown)
      const withHealth: FederationDoorwayWithHealth[] = response.doorways.map(d => ({
        ...d,
        latencyMs: null,
        isReachable: false,
      }));
      this.doorways.set(withHealth);
      this.loading.set(false);

      // Run health checks in parallel (non-blocking)
      this.runHealthChecks(response.doorways);
    } catch (err) {
      this.loading.set(false);
      this.error.set(
        err instanceof Error ? err.message : 'Failed to load doorways'
      );
    }
  }

  private async runHealthChecks(
    doorways: { id: string; url: string; region?: string; tier: string; capabilities: string[]; status: string }[]
  ): Promise<void> {
    const checks = doorways.map(d => this.federation.checkHealth(d));
    const results = await Promise.allSettled(checks);

    const updated = results.map((r, i) => {
      if (r.status === 'fulfilled') {
        return r.value;
      }
      return {
        ...doorways[i],
        latencyMs: null,
        isReachable: false,
      } as FederationDoorwayWithHealth;
    });

    this.doorways.set(updated);
  }

  isSelf(doorway: FederationDoorwayWithHealth): boolean {
    return doorway.id === this.selfId();
  }

  selectDoorway(doorway: FederationDoorwayWithHealth): void {
    const params = this.oauthParams();

    if (this.isSelf(doorway)) {
      // Same doorway - go back to login
      window.location.href = this.loginUrl();
      return;
    }

    if (params) {
      // Different doorway - redirect to its authorize endpoint
      const url = this.federation.buildOAuthRedirectUrl(doorway, params);
      window.location.href = url;
    }
  }

  applySearch(): void {
    // Triggers computed re-evaluation via template binding
  }

  applyRegionFilter(): void {
    // Triggers computed re-evaluation via template binding
  }

  retry(): void {
    this.loadDoorways();
  }
}
