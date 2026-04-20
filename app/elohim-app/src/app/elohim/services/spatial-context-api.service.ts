import { HttpClient, HttpParams } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { Observable } from 'rxjs';

import type {
  SpatialContextView,
  CreateSpatialContextInputView,
  UpdateSpatialContextInputView,
} from '@elohim/storage-client';

@Injectable({ providedIn: 'root' })
export class SpatialContextApiService {
  private readonly http = inject(HttpClient);
  private readonly baseUrl = '';

  create(input: CreateSpatialContextInputView): Observable<SpatialContextView> {
    return this.http.post<SpatialContextView>(`${this.baseUrl}/api/v1/spatial-contexts`, input);
  }

  getById(id: string): Observable<SpatialContextView> {
    return this.http.get<SpatialContextView>(`${this.baseUrl}/api/v1/spatial-contexts/${id}`);
  }

  getForEntity(entityType: string, entityId: string): Observable<SpatialContextView[]> {
    const params = new HttpParams().set('entityType', entityType).set('entityId', entityId);
    return this.http.get<SpatialContextView[]>(`${this.baseUrl}/api/v1/spatial-contexts`, {
      params,
    });
  }

  listByH3(resolution: 5 | 7 | 9, h3Index: string): Observable<SpatialContextView[]> {
    const key = `h3Res${resolution}`;
    const params = new HttpParams().set(key, h3Index);
    return this.http.get<SpatialContextView[]>(`${this.baseUrl}/api/v1/spatial-contexts`, {
      params,
    });
  }

  listByPlace(placeId: string): Observable<SpatialContextView[]> {
    const params = new HttpParams().set('placeId', placeId);
    return this.http.get<SpatialContextView[]>(`${this.baseUrl}/api/v1/spatial-contexts`, {
      params,
    });
  }

  update(
    id: string,
    patch: Partial<UpdateSpatialContextInputView>
  ): Observable<SpatialContextView> {
    return this.http.patch<SpatialContextView>(
      `${this.baseUrl}/api/v1/spatial-contexts/${id}`,
      patch
    );
  }

  delete(id: string): Observable<void> {
    return this.http.delete<void>(`${this.baseUrl}/api/v1/spatial-contexts/${id}`);
  }

  /** Get full spatial history for an entity (current + all past positions), newest first */
  getHistory(entityType: string, entityId: string): Observable<SpatialContextView[]> {
    return this.http.get<SpatialContextView[]>(
      `${this.baseUrl}/api/v1/spatial-contexts/history/${entityType}/${entityId}`
    );
  }
}
