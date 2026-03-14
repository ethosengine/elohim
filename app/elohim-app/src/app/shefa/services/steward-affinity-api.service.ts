/**
 * StewardAffinityApiService — Thin HTTP client for steward affinity queries and curation events.
 *
 * Calls elohim-storage `/api/v1/steward-affinity/*` endpoints.
 * The recognition pipeline uses affinity scores to weight distribution,
 * and curation events are the primary way affinity grows.
 */

import { HttpClient, HttpParams } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { catchError } from 'rxjs/operators';

import { Observable, of } from 'rxjs';

import type { StewardAffinityView, CurationEventInputView } from '@elohim/storage-client/generated';

export interface AffinityQuery {
  stewardId?: string;
  contentId?: string;
  limit?: number;
  offset?: number;
}

const BASE = '/api/v1/steward-affinity';

@Injectable({ providedIn: 'root' })
export class StewardAffinityApiService {
  private readonly http = inject(HttpClient);

  /**
   * List steward affinities with optional filters.
   */
  listAffinities(query?: AffinityQuery): Observable<StewardAffinityView[]> {
    let params = new HttpParams();
    if (query?.stewardId) params = params.set('stewardId', query.stewardId);
    if (query?.contentId) params = params.set('contentId', query.contentId);
    if (query?.limit !== undefined) params = params.set('limit', query.limit.toString());
    if (query?.offset !== undefined) params = params.set('offset', query.offset.toString());

    return this.http
      .get<StewardAffinityView[]>(BASE, { params })
      .pipe(catchError(() => of([] as StewardAffinityView[])));
  }

  /**
   * Get a specific affinity by ID.
   */
  getAffinity(affinityId: string): Observable<StewardAffinityView> {
    return this.http.get<StewardAffinityView>(`${BASE}/${affinityId}`);
  }

  /**
   * Record a curation activity (edit, review, tag, etc.) that builds affinity.
   *
   * This is the primary mechanism by which stewards build standing with content.
   * The backend calculates the affinity delta based on activity type.
   */
  recordCurationEvent(input: CurationEventInputView): Observable<StewardAffinityView> {
    return this.http.post<StewardAffinityView>(`${BASE}/curation-event`, input);
  }
}
