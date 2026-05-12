/**
 * ObservationService — HTTP client for the observations API.
 *
 * Provides access to observation endpoints at `/api/observations/`:
 *   - bySubject(subjectCid, kind) → ObservationView[]
 *   - byObserver(observerCid, kind?) → ObservationView[]
 *   - diversity(subjectCid, kind) → ObservationDiversitySummaryView | null
 *
 * Observations are the foundational telemetry stream — every interaction
 * produces an observation record. This service is thin; it translates HTTP
 * parameters and returns parsed views.
 */

import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';
import type { ObservationView, ObservationDiversitySummaryView } from '@elohim/storage-client';

@Injectable({ providedIn: 'root' })
export class ObservationService {
  private readonly http = inject(HttpClient);
  private readonly base = '/api/observations';

  /**
   * Get observations by subject (what was observed).
   *
   * @param subjectCid - Content ID of the observed entity
   * @param kind - Observation kind filter (e.g., 'infrastructure:doorway-heartbeat')
   * @returns Observable of matching observation records
   */
  bySubject(subjectCid: string, kind: string): Observable<ObservationView[]> {
    return this.http.get<ObservationView[]>(`${this.base}/by-subject`, {
      params: new HttpParams().set('subjectCid', subjectCid).set('kind', kind),
    });
  }

  /**
   * Get observations by observer (who made the observation).
   *
   * @param observerCid - CID of the observer agent/device
   * @param kind - Optional observation kind filter
   * @returns Observable of matching observation records
   */
  byObserver(observerCid: string, kind?: string): Observable<ObservationView[]> {
    let params = new HttpParams().set('observerCid', observerCid);
    if (kind) {
      params = params.set('kind', kind);
    }
    return this.http.get<ObservationView[]>(`${this.base}/by-observer`, { params });
  }

  /**
   * Get diversity summary for a subject/kind pair.
   *
   * Used by the attestation-graduation evaluator to understand
   * how diverse the observation coverage is — distinct agents,
   * households, collectives, regions, archetypes, compute classes.
   *
   * @param subjectCid - Content ID of the observed entity
   * @param kind - Observation kind
   * @returns Observable of diversity summary, or null if no observations exist
   */
  diversity(
    subjectCid: string,
    kind: string
  ): Observable<ObservationDiversitySummaryView | null> {
    return this.http.get<ObservationDiversitySummaryView | null>(`${this.base}/diversity`, {
      params: new HttpParams().set('subjectCid', subjectCid).set('kind', kind),
    });
  }
}
