/**
 * Doorway Federation Service
 *
 * REST-based doorway discovery for the doorway-app.
 * Fetches federation doorway list from the current doorway's API
 * and provides health checking + OAuth redirect URL building.
 */

import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';

// =============================================================================
// Types
// =============================================================================

/** Doorway summary from the federation API */
export interface FederationDoorway {
  id: string;
  url: string;
  region?: string;
  tier: string;
  capabilities: string[];
  status: string;
}

/** Federation API response shape */
export interface FederationDoorwaysResponse {
  doorways: FederationDoorway[];
  self_id: string | null;
  total: number;
}

/** Doorway with health check results */
export interface FederationDoorwayWithHealth extends FederationDoorway {
  latencyMs: number | null;
  isReachable: boolean;
}

/** OAuth params needed for redirect */
export interface OAuthParams {
  clientId: string;
  redirectUri: string;
  responseType: string;
  state: string;
  scope?: string;
  loginHint?: string;
}

// =============================================================================
// Service
// =============================================================================

@Injectable({ providedIn: 'root' })
export class DoorwayFederationService {
  private readonly http = inject(HttpClient);

  /** Fetch known doorways from this doorway's federation endpoint */
  async loadDoorways(): Promise<FederationDoorwaysResponse> {
    const response = await firstValueFrom(
      this.http.get<FederationDoorwaysResponse>('/api/v1/federation/doorways')
    );
    return response;
  }

  /** Health-check a single doorway */
  async checkHealth(doorway: FederationDoorway): Promise<FederationDoorwayWithHealth> {
    const start = performance.now();
    try {
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 5000);

      await fetch(`${doorway.url}/health`, {
        signal: controller.signal,
        mode: 'no-cors',
      });

      clearTimeout(timeout);
      const latencyMs = Math.round(performance.now() - start);

      return { ...doorway, latencyMs, isReachable: true };
    } catch {
      return { ...doorway, latencyMs: null, isReachable: false };
    }
  }

  /** Build OAuth redirect URL for a federated doorway */
  buildOAuthRedirectUrl(doorway: FederationDoorway, oauthParams: OAuthParams): string {
    const params = new URLSearchParams({
      client_id: oauthParams.clientId,
      redirect_uri: oauthParams.redirectUri,
      response_type: oauthParams.responseType,
      state: oauthParams.state,
    });
    if (oauthParams.scope) {
      params.set('scope', oauthParams.scope);
    }
    if (oauthParams.loginHint) {
      params.set('login_hint', oauthParams.loginHint);
    }

    // Redirect to the target doorway's authorize endpoint
    const baseUrl = doorway.url.replace(/\/$/, '');
    return `${baseUrl}/auth/authorize?${params.toString()}`;
  }
}
