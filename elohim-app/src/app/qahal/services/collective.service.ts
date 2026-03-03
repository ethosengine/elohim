/**
 * CollectiveService - Domain service for governance contexts.
 *
 * Provides high-level operations for managing collectives — named governance
 * contexts that humans participate in through graduated intimacy.
 *
 * No UI components — those come when community features are designed.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable } from '@angular/core';

import { Observable, map } from 'rxjs';

import { environment } from '../../../environments/environment';

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
export class CollectiveService {
  private readonly baseUrl: string;

  constructor(private readonly http: HttpClient) {
    this.baseUrl =
      environment.holochain?.storageUrl ??
      (environment as unknown as { client?: { doorwayUrl?: string } }).client?.doorwayUrl ??
      '';
  }

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
      .get<ListResponse<CollectiveView>>(`${this.baseUrl}/db/collectives`, {
        params: queryParams,
      })
      .pipe(map((res): CollectiveView[] => res.items));
  }

  /** Get a single collective by ID */
  getCollective(id: string): Observable<CollectiveView> {
    return this.http.get<CollectiveView>(`${this.baseUrl}/db/collectives/${id}`);
  }

  /** Create a collective */
  createCollective(input: CreateCollectiveInputView): Observable<CollectiveView> {
    return this.http.post<CollectiveView>(`${this.baseUrl}/db/collectives`, input);
  }

  // ===========================================================================
  // Participation Management
  // ===========================================================================

  /** Get all participants of a collective */
  getParticipants(collectiveId: string): Observable<CollectiveParticipationView[]> {
    return this.http
      .get<
        ListResponse<CollectiveParticipationView>
      >(`${this.baseUrl}/db/collectives/${collectiveId}/participants`)
      .pipe(map((res): CollectiveParticipationView[] => res.items));
  }

  /** Add a participant to a collective */
  addParticipant(
    collectiveId: string,
    humanId: string,
    options?: { intimacyLevel?: string; roleContext?: string }
  ): Observable<CollectiveParticipationView> {
    return this.http.post<CollectiveParticipationView>(
      `${this.baseUrl}/db/collectives/${collectiveId}/participants`,
      {
        humanId,
        intimacyLevel: options?.intimacyLevel,
        roleContext: options?.roleContext,
      }
    );
  }

  /** Depart from a collective (soft exit) */
  departCollective(collectiveId: string, humanId: string): Observable<void> {
    return this.http.delete<void>(
      `${this.baseUrl}/db/collectives/${collectiveId}/participants/${humanId}`
    );
  }

  /** Get all collectives a human participates in */
  getCollectivesForHuman(humanId: string): Observable<CollectiveParticipationView[]> {
    return this.http
      .get<
        ListResponse<CollectiveParticipationView>
      >(`${this.baseUrl}/db/participations/${humanId}`)
      .pipe(map((res): CollectiveParticipationView[] => res.items));
  }
}
