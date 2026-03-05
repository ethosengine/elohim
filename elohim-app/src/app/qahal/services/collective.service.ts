/**
 * CollectiveService - Domain service for governance contexts.
 *
 * Provides high-level operations for managing collectives — named governance
 * contexts that humans participate in through graduated intimacy.
 *
 * No UI components — those come when community features are designed.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { Observable, map } from 'rxjs';

import { type ICollective } from '../interfaces/collective.interface';

import type { GovernanceLayer } from '@app/qahal/models/collective.model';
import type {
  CollectiveParticipationView,
  CollectiveView,
  CreateCollectiveInputView,
} from '@elohim/storage-client';

interface ListResponse<T> {
  items: T[];
  count: number;
}

@Injectable({
  providedIn: 'root',
})
export class CollectiveService implements ICollective {
  private readonly http = inject(HttpClient);

  // ===========================================================================
  // Collective CRUD
  // ===========================================================================

  /** List collectives, optionally filtered by governance layer */
  listCollectives(params?: {
    governanceLayer?: GovernanceLayer;
    reach?: string;
    activeOnly?: boolean;
  }): Observable<CollectiveView[]> {
    const queryParams: Record<string, string> = {};
    if (params?.governanceLayer) queryParams['governanceLayer'] = params.governanceLayer;
    if (params?.reach) queryParams['reach'] = params.reach;
    if (params?.activeOnly !== undefined) queryParams['activeOnly'] = String(params.activeOnly);

    return this.http
      .get<ListResponse<CollectiveView>>('/api/v1/collectives', {
        params: queryParams,
      })
      .pipe(map((res): CollectiveView[] => res.items));
  }

  /** Get a single collective by ID */
  getCollective(id: string): Observable<CollectiveView> {
    return this.http.get<CollectiveView>(`/api/v1/collectives/${id}`);
  }

  /** Create a collective */
  createCollective(input: CreateCollectiveInputView): Observable<CollectiveView> {
    return this.http.post<CollectiveView>('/api/v1/collectives', input);
  }

  // ===========================================================================
  // Participation Management
  // ===========================================================================

  /** Get all participants of a collective */
  getParticipants(collectiveId: string): Observable<CollectiveParticipationView[]> {
    return this.http
      .get<
        ListResponse<CollectiveParticipationView>
      >(`/api/v1/collectives/${collectiveId}/participants`)
      .pipe(map((res): CollectiveParticipationView[] => res.items));
  }

  /** Add a participant to a collective */
  addParticipant(
    collectiveId: string,
    humanId: string,
    options?: { intimacyLevel?: string; roleContext?: string }
  ): Observable<CollectiveParticipationView> {
    return this.http.post<CollectiveParticipationView>(
      `/api/v1/collectives/${collectiveId}/participants`,
      {
        humanId,
        intimacyLevel: options?.intimacyLevel,
        roleContext: options?.roleContext,
      }
    );
  }

  /** Depart from a collective (soft exit) */
  departCollective(collectiveId: string, humanId: string): Observable<void> {
    return this.http.delete<void>(`/api/v1/collectives/${collectiveId}/participants/${humanId}`);
  }

  /** Get all collectives a human participates in */
  getCollectivesForHuman(humanId: string): Observable<CollectiveParticipationView[]> {
    return this.http
      .get<ListResponse<CollectiveParticipationView>>(`/api/v1/humans/${humanId}/collectives`)
      .pipe(map((res): CollectiveParticipationView[] => res.items));
  }
}
