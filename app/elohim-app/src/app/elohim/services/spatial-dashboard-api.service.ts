import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';

import type { SpatialDashboardView } from '@elohim/storage-client';

@Injectable({ providedIn: 'root' })
export class SpatialDashboardApiService {
  private readonly http = inject(HttpClient);
  private readonly baseUrl = '';

  getDashboard(query?: {
    constitutionalLayer?: string;
    h3Index?: string;
    h3Resolution?: number;
    parentPlaceId?: string;
    status?: string;
    include?: string;
    limit?: number;
  }): Observable<SpatialDashboardView> {
    let params = new HttpParams();
    if (query?.constitutionalLayer)
      params = params.set('constitutionalLayer', query.constitutionalLayer);
    if (query?.h3Index) params = params.set('h3Index', query.h3Index);
    if (query?.h3Resolution)
      params = params.set('h3Resolution', query.h3Resolution.toString());
    if (query?.parentPlaceId) params = params.set('parentPlaceId', query.parentPlaceId);
    if (query?.status) params = params.set('status', query.status);
    if (query?.include) params = params.set('include', query.include);
    if (query?.limit) params = params.set('limit', query.limit.toString());
    return this.http.get<SpatialDashboardView>(
      `${this.baseUrl}/api/v1/dashboard/spatial`,
      { params },
    );
  }
}
