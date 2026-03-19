import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';

import type { PlaceView, CreatePlaceInputView } from '@elohim/storage-client';

@Injectable({ providedIn: 'root' })
export class PlacesApiService {
  private readonly http = inject(HttpClient);
  private readonly baseUrl = '';

  create(input: CreatePlaceInputView): Observable<PlaceView> {
    return this.http.post<PlaceView>(`${this.baseUrl}/api/v1/places`, input);
  }

  getById(id: string): Observable<PlaceView> {
    return this.http.get<PlaceView>(`${this.baseUrl}/api/v1/places/${id}`);
  }

  list(query?: {
    placeType?: string;
    constitutionalLayer?: string;
    h3Index?: string;
    parentPlaceId?: string;
    status?: string;
    limit?: number;
  }): Observable<PlaceView[]> {
    let params = new HttpParams();
    if (query?.placeType) params = params.set('placeType', query.placeType);
    if (query?.constitutionalLayer)
      params = params.set('constitutionalLayer', query.constitutionalLayer);
    if (query?.h3Index) params = params.set('h3Index', query.h3Index);
    if (query?.parentPlaceId) params = params.set('parentPlaceId', query.parentPlaceId);
    if (query?.status) params = params.set('status', query.status);
    if (query?.limit) params = params.set('limit', query.limit.toString());
    return this.http.get<PlaceView[]>(`${this.baseUrl}/api/v1/places`, { params });
  }
}
