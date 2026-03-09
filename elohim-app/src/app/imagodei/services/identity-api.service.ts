/**
 * IdentityApiService — Thin HTTP client for human identity operations.
 *
 * Calls doorway `/api/v1/identity/*` endpoints, implementing IIdentityApi.
 * Mutable profile data lives in elohim-storage's Diesel layer. The Holochain
 * DNA retains its role for cryptographic provenance (attestations, agent keys).
 *
 * Attestations are not yet served by storage — they remain empty here
 * until the provenance query endpoint is built.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { firstValueFrom } from 'rxjs';
import { catchError } from 'rxjs/operators';

import type { IIdentityApi } from '../interfaces/identity.interface';
import type {
  HumanSessionResult,
  HumanUpdateResult,
  RegisterHumanPayload,
  UpdateHumanPayload,
} from '../models/identity.model';

/** Response shape from /api/v1/identity endpoints (matches HumanView in storage) */
interface HumanApiResponse {
  id: string;
  agentPubKey: string | null;
  displayName: string;
  bio: string | null;
  affinities: string[];
  profileReach: string;
  location: string | null;
  appId: string;
  createdAt: string;
  updatedAt: string;
}

/** Map storage HumanView to the HumanSessionResult shape identity.service expects */
function toSessionResult(response: HumanApiResponse): HumanSessionResult {
  return {
    agentPubkey: response.agentPubKey ?? '',
    actionHash: new Uint8Array(0), // Not applicable for storage-backed identity
    human: {
      id: response.id,
      displayName: response.displayName,
      bio: response.bio,
      affinities: response.affinities,
      profileReach: response.profileReach,
      location: response.location,
      createdAt: response.createdAt,
      updatedAt: response.updatedAt,
    },
    sessionStartedAt: response.createdAt,
    attestations: [], // Attestations live in the provenance layer (DNA), not storage
  };
}

/** Map storage HumanView to the HumanUpdateResult shape */
function toUpdateResult(response: HumanApiResponse): HumanUpdateResult {
  return {
    actionHash: new Uint8Array(0),
    human: {
      id: response.id,
      displayName: response.displayName,
      bio: response.bio,
      affinities: response.affinities,
      profileReach: response.profileReach,
      location: response.location,
      createdAt: response.createdAt,
      updatedAt: response.updatedAt,
    },
  };
}

@Injectable({ providedIn: 'root' })
export class IdentityApiService implements IIdentityApi {
  private readonly http = inject(HttpClient);

  async createHuman(payload: RegisterHumanPayload): Promise<HumanSessionResult> {
    const response = await firstValueFrom(
      this.http.post<HumanApiResponse>('/api/v1/identity/register', payload)
    );
    return toSessionResult(response);
  }

  async getMyHuman(): Promise<HumanSessionResult | null> {
    try {
      const response = await firstValueFrom(
        this.http
          .get<HumanApiResponse>('/api/v1/identity/me')
          .pipe(catchError(() => [null as unknown as HumanApiResponse]))
      );
      return response ? toSessionResult(response) : null;
    } catch {
      return null;
    }
  }

  async updateHuman(payload: UpdateHumanPayload): Promise<HumanUpdateResult> {
    const response = await firstValueFrom(
      this.http.put<HumanApiResponse>('/api/v1/identity/me', payload)
    );
    return toUpdateResult(response);
  }
}
