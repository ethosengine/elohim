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
import {
  DoorwayAddressResolver,
  DoorwayResolution,
  gatewayCandidates,
} from '@elohim/service/client/doorway-address-resolver';

// =============================================================================
// Types
// =============================================================================

/** Doorway summary from the federation API */
export interface FederationDoorway {
  id: string;
  url: string;
  identity_root?: string | null;
  signing_key?: string | null;
  endpoints?: FederationDoorwayEndpoint[];
  record_serial?: number | null;
  record_signature?: number[] | null;
  region?: string;
  tier: string;
  capabilities: string[];
  status: string;
}

export interface FederationDoorwayEndpoint {
  service: string;
  url: string;
  priority: number;
  ttl_secs: number;
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
export class DoorwayFederationService implements DoorwayAddressResolver {
  private readonly http = inject(HttpClient);
  private readonly resolutions = new Map<string, DoorwayResolution>();

  /** Fetch known doorways from this doorway's federation endpoint */
  async loadDoorways(): Promise<FederationDoorwaysResponse> {
    const response = await firstValueFrom(
      this.http.get<FederationDoorwaysResponse>('/api/v1/federation/doorways')
    );
    for (const doorway of response.doorways) {
      const resolution = federationDoorwayResolution(doorway);
      this.resolutions.set(resolution.identity, resolution);
    }
    return response;
  }

  resolve(identity: string): DoorwayResolution {
    const resolution = this.resolutions.get(identity);
    if (!resolution) {
      throw new Error(`No discovered addresses for doorway identity "${identity}"`);
    }
    return resolution;
  }

  /** Health-check a single doorway */
  async checkHealth(doorway: FederationDoorway): Promise<FederationDoorwayWithHealth> {
    const start = performance.now();
    const identity = doorway.identity_root ?? doorway.signing_key ?? doorway.id;
    const resolution = this.resolutions.get(identity) ?? federationDoorwayResolution(doorway);
    for (const candidate of gatewayCandidates(resolution)) {
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 5000);
      try {
        await fetch(`${candidate}/health`, {
          signal: controller.signal,
          mode: 'no-cors',
        });
        return {
          ...doorway,
          url: candidate,
          latencyMs: Math.round(performance.now() - start),
          isReachable: true,
        };
      } catch {
        // Try the next signed/configured candidate.
      } finally {
        clearTimeout(timeout);
      }
    }
    return { ...doorway, latencyMs: null, isReachable: false };
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
    const baseUrl =
      gatewayCandidates(federationDoorwayResolution(doorway))[0] ?? doorway.url.replace(/\/$/, '');
    return `${baseUrl}/auth/authorize?${params.toString()}`;
  }
}

export function federationDoorwayResolution(doorway: FederationDoorway): DoorwayResolution {
  return {
    identity: doorway.identity_root ?? doorway.signing_key ?? doorway.id,
    source: doorway.record_signature?.length ? 'registration' : 'config',
    endpoints: doorway.endpoints?.map(endpoint => ({
      service: endpoint.service,
      url: endpoint.url,
      priority: endpoint.priority,
      ttlSecs: endpoint.ttl_secs,
    })) ?? [
      {
        service: 'gateway',
        url: doorway.url,
        priority: 0,
      },
    ],
  };
}
