/**
 * PresenceApiService — Thin HTTP client for contributor presence lifecycle.
 *
 * Calls doorway `/api/v1/presence/*` endpoints, implementing IPresenceLifecycle.
 * Replaces the fat PresenceService (Holochain zome calls) when the business
 * logic lives behind the Rust API boundary.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { catchError } from 'rxjs/operators';

import { firstValueFrom, of } from 'rxjs';

import type { IPresenceLifecycle } from '../interfaces/presence.interface';
import type {
  ContributorPresenceView,
  CreatePresenceRequest,
  InitiateClaimRequest,
  PresenceState,
} from '../models/presence.model';

@Injectable({ providedIn: 'root' })
export class PresenceApiService implements IPresenceLifecycle {
  private readonly http = inject(HttpClient);

  async createPresence(request: CreatePresenceRequest): Promise<ContributorPresenceView> {
    return firstValueFrom(this.http.post<ContributorPresenceView>('/api/v1/presence', request));
  }

  async beginStewardship(
    presenceId: string,
    commitmentNote?: string
  ): Promise<ContributorPresenceView> {
    return firstValueFrom(
      this.http.post<ContributorPresenceView>(`/api/v1/presence/${presenceId}/steward`, {
        commitmentNote,
      })
    );
  }

  async getMyStewardedPresences(): Promise<ContributorPresenceView[]> {
    return firstValueFrom(
      this.http
        .get<ContributorPresenceView[]>('/api/v1/presence/steward/me')
        .pipe(catchError(() => of([])))
    );
  }

  async initiateClaim(request: InitiateClaimRequest): Promise<ContributorPresenceView> {
    return firstValueFrom(
      this.http.post<ContributorPresenceView>(
        `/api/v1/presence/${request.presenceId}/claim`,
        request
      )
    );
  }

  async verifyClaim(presenceId: string): Promise<ContributorPresenceView> {
    return firstValueFrom(
      this.http.post<ContributorPresenceView>(`/api/v1/presence/${presenceId}/verify`, {})
    );
  }

  async getPresenceById(id: string): Promise<ContributorPresenceView | null> {
    return firstValueFrom(
      this.http
        .get<ContributorPresenceView>(`/api/v1/presence/${id}`)
        .pipe(catchError(() => of(null)))
    );
  }

  async getPresencesByState(state: PresenceState): Promise<ContributorPresenceView[]> {
    return firstValueFrom(
      this.http
        .get<ContributorPresenceView[]>('/api/v1/presence', { params: { state } })
        .pipe(catchError(() => of([])))
    );
  }
}
