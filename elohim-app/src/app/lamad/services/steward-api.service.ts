/**
 * StewardApiService — Thin HTTP client for steward endpoints.
 *
 * Calls doorway `/api/v1/steward/*` endpoints, implementing ISteward.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { catchError, map } from 'rxjs/operators';
import { firstValueFrom, of } from 'rxjs';

import type {
  StewardCredentialView,
  PremiumGateView,
  AccessGrantView,
  StewardRevenueSummaryView,
  CreateCredentialInputView,
  CreateGateInputView,
  CreateGrantInputView,
} from '@elohim/storage-client/generated';

import type { ISteward } from '../interfaces/steward.interface';

@Injectable({ providedIn: 'root' })
export class StewardApiService implements ISteward {
  private readonly http = inject(HttpClient);

  async createCredential(input: CreateCredentialInputView): Promise<StewardCredentialView> {
    return firstValueFrom(
      this.http.post<StewardCredentialView>('/api/v1/steward/credentials', input)
    );
  }

  async getCredential(id: string): Promise<StewardCredentialView | null> {
    return firstValueFrom(
      this.http
        .get<StewardCredentialView>(`/api/v1/steward/credentials/${encodeURIComponent(id)}`)
        .pipe(catchError(() => of(null)))
    );
  }

  async queryCredentialsForPresence(presenceId: string): Promise<StewardCredentialView[]> {
    return firstValueFrom(
      this.http
        .get<StewardCredentialView[]>('/api/v1/steward/credentials', {
          params: { presenceId: encodeURIComponent(presenceId) },
        })
        .pipe(catchError(() => of([])))
    );
  }

  async queryCredentialsForContent(contentId: string): Promise<StewardCredentialView[]> {
    return firstValueFrom(
      this.http
        .get<StewardCredentialView[]>('/api/v1/steward/credentials', {
          params: { contentId: encodeURIComponent(contentId) },
        })
        .pipe(catchError(() => of([])))
    );
  }

  async createGate(input: CreateGateInputView): Promise<PremiumGateView> {
    return firstValueFrom(this.http.post<PremiumGateView>('/api/v1/steward/gates', input));
  }

  async getGate(id: string): Promise<PremiumGateView | null> {
    return firstValueFrom(
      this.http
        .get<PremiumGateView>(`/api/v1/steward/gates/${encodeURIComponent(id)}`)
        .pipe(catchError(() => of(null)))
    );
  }

  async queryGatesForResource(resourceType: string): Promise<PremiumGateView[]> {
    return firstValueFrom(
      this.http
        .get<PremiumGateView[]>('/api/v1/steward/gates', {
          params: { resourceType: encodeURIComponent(resourceType) },
        })
        .pipe(catchError(() => of([])))
    );
  }

  async createGrant(input: CreateGrantInputView): Promise<AccessGrantView> {
    return firstValueFrom(this.http.post<AccessGrantView>('/api/v1/steward/grants', input));
  }

  async getGrant(id: string): Promise<AccessGrantView | null> {
    return firstValueFrom(
      this.http
        .get<AccessGrantView>(`/api/v1/steward/grants/${encodeURIComponent(id)}`)
        .pipe(catchError(() => of(null)))
    );
  }

  async queryGrantsForGate(gateId: string): Promise<AccessGrantView[]> {
    return firstValueFrom(
      this.http
        .get<AccessGrantView[]>('/api/v1/steward/grants', {
          params: { gateId: encodeURIComponent(gateId) },
        })
        .pipe(catchError(() => of([])))
    );
  }

  async checkAccess(gateId: string, granteeId: string): Promise<boolean> {
    return firstValueFrom(
      this.http
        .get<{ hasAccess: boolean }>('/api/v1/steward/access', {
          params: {
            gateId: encodeURIComponent(gateId),
            granteeId: encodeURIComponent(granteeId),
          },
        })
        .pipe(
          map((response) => response.hasAccess),
          catchError(() => of(false))
        )
    );
  }

  async getRevenueSummary(presenceId: string): Promise<StewardRevenueSummaryView> {
    return firstValueFrom(
      this.http.get<StewardRevenueSummaryView>(
        `/api/v1/steward/revenue/${encodeURIComponent(presenceId)}`
      )
    );
  }
}
