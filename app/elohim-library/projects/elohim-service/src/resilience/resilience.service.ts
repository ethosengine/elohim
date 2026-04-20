import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';
import { ResilienceSnapshotView } from '../generated/resilience-snapshot-view';
import { PlacementGapView } from '../generated/placement-gap-view';

@Injectable({ providedIn: 'root' })
export class ResilienceService {
  private readonly http = inject(HttpClient);

  getSnapshot(contentId: string, viewerHouseholdId?: string): Observable<ResilienceSnapshotView> {
    let params = new HttpParams();
    if (viewerHouseholdId) params = params.set('viewerHouseholdId', viewerHouseholdId);
    return this.http.get<ResilienceSnapshotView>(
      `/api/v1/resilience/${encodeURIComponent(contentId)}/household`,
      { params },
    );
  }

  listPlacementGaps(kind?: string): Observable<{ items: PlacementGapView[]; total: number }> {
    let params = new HttpParams();
    if (kind) params = params.set('kind', kind);
    return this.http.get<{ items: PlacementGapView[]; total: number }>(
      '/api/v1/placement-gaps',
      { params },
    );
  }
}
