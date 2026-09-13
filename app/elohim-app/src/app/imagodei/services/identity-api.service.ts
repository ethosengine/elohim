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

import { HttpClient, HttpHeaders } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { catchError } from 'rxjs/operators';

import { firstValueFrom } from 'rxjs';

import { BrowserSessionTokenStore } from './browser-session-token.store';

import type { IIdentityApi } from '../interfaces/identity.interface';
import type {
  HumanSessionResult,
  HumanUpdateResult,
  RegisterHumanPayload,
  UpdateHumanPayload,
} from '../models/identity.model';

/** The identity-scoped read/write route (doorway -> elohim-storage). */
const IDENTITY_ME = '/api/v1/identity/me';

/** Response shape from /api/v1/identity endpoints (matches HumanView in storage) */
interface HumanApiResponse {
  id: string;
  agentPubKey: string | null;
  displayName: string;
  bio: string | null;
  affinities: string[];
  profileReach: string;
  location: string | null;
  hAppId: string;
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
  private readonly sessionStore = inject(BrowserSessionTokenStore);

  /**
   * The session bearer, or null when nobody is signed in.
   *
   * `/api/v1/identity/*` is an identity-scoped read: the doorway resolves the
   * caller from this bearer's claims and tells storage who is asking. Without
   * it the request is anonymous by construction and storage answers 401 —
   * which is correct, and which every reader of this service was swallowing
   * while the browser logged it as a console error. Read from the same token
   * store AuthService hydrates from, so there is one source of session truth
   * and no service-level cycle back through AuthService.
   */
  // eslint-disable-next-line sonarjs/function-return-type -- intentional `string | null`; the rule misfires on nullable unions in this toolchain
  private bearer(): string | null {
    return this.sessionStore.get()?.token ?? null;
  }

  private authHeaders(token: string): { headers: HttpHeaders } {
    return { headers: new HttpHeaders({ Authorization: `Bearer ${token}` }) };
  }

  async createHuman(payload: RegisterHumanPayload): Promise<HumanSessionResult> {
    const response = await firstValueFrom(
      this.http.post<HumanApiResponse>('/api/v1/identity/register', payload)
    );
    return toSessionResult(response);
  }

  async getMyHuman(): Promise<HumanSessionResult | null> {
    const token = this.bearer();
    // "Who am I?" is not a question an anonymous visitor can ask. Asking it
    // anyway sent a credential-less request whose only possible answer was
    // 401 — swallowed here, but logged by the browser on every page load.
    if (!token) return null;

    try {
      const response = await firstValueFrom(
        this.http
          .get<HumanApiResponse>(IDENTITY_ME, this.authHeaders(token))
          .pipe(catchError(() => [null as unknown as HumanApiResponse]))
      );
      return response ? toSessionResult(response) : null;
    } catch {
      return null;
    }
  }

  async updateHuman(payload: UpdateHumanPayload): Promise<HumanUpdateResult> {
    const token = this.bearer();
    const response = await firstValueFrom(
      token
        ? this.http.put<HumanApiResponse>(IDENTITY_ME, payload, this.authHeaders(token))
        : this.http.put<HumanApiResponse>(IDENTITY_ME, payload)
    );
    return toUpdateResult(response);
  }
}
