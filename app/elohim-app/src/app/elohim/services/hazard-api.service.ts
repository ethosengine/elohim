import { HttpClient, HttpParams } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { Observable } from 'rxjs';

import type { HazardView, CreateHazardInputView } from '@elohim/storage-client';

@Injectable({ providedIn: 'root' })
export class HazardApiService {
  private readonly http = inject(HttpClient);
  private readonly baseUrl = '';

  create(input: CreateHazardInputView): Observable<HazardView> {
    return this.http.post<HazardView>(`${this.baseUrl}/api/v1/hazards`, input);
  }

  getById(id: string): Observable<HazardView> {
    return this.http.get<HazardView>(`${this.baseUrl}/api/v1/hazards/${id}`);
  }

  list(query?: {
    placeId?: string;
    status?: string;
    hazardType?: string;
    limit?: number;
  }): Observable<HazardView[]> {
    let params = new HttpParams();
    if (query?.placeId) params = params.set('placeId', query.placeId);
    if (query?.status) params = params.set('status', query.status);
    if (query?.hazardType) params = params.set('hazardType', query.hazardType);
    if (query?.limit) params = params.set('limit', query.limit.toString());
    return this.http.get<HazardView[]>(`${this.baseUrl}/api/v1/hazards`, { params });
  }

  updateStatus(
    id: string,
    patch: { status: string; severity?: string; actualOnset?: string; resolvedAt?: string }
  ): Observable<HazardView> {
    return this.http.patch<HazardView>(`${this.baseUrl}/api/v1/hazards/${id}`, patch);
  }
}
