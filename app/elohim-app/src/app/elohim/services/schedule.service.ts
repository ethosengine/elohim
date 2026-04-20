// schedule.service.ts
import { HttpClient, HttpParams } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { Observable } from 'rxjs';

import type {
  ScheduleView,
  CreateScheduleInputView,
  UpdateScheduleInputView,
} from '@elohim/storage-client';

@Injectable({ providedIn: 'root' })
export class ScheduleService {
  private readonly http = inject(HttpClient);
  private readonly baseUrl = '';

  getSchedule(entityType: string, entityId: string): Observable<ScheduleView> {
    const params = new HttpParams().set('entityType', entityType).set('entityId', entityId);
    return this.http.get<ScheduleView>(`${this.baseUrl}/api/v1/schedules`, { params });
  }

  createSchedule(input: CreateScheduleInputView): Observable<ScheduleView> {
    return this.http.post<ScheduleView>(`${this.baseUrl}/api/v1/schedules`, input);
  }

  updateSchedule(id: string, patch: Partial<UpdateScheduleInputView>): Observable<ScheduleView> {
    return this.http.patch<ScheduleView>(`${this.baseUrl}/api/v1/schedules/${id}`, patch);
  }

  getDueSchedules(before?: string): Observable<ScheduleView[]> {
    let params = new HttpParams();
    if (before) {
      params = params.set('dueBefore', before);
    }
    return this.http.get<ScheduleView[]>(`${this.baseUrl}/api/v1/schedules`, { params });
  }

  advanceOccurrence(id: string): Observable<ScheduleView> {
    return this.http.post<ScheduleView>(`${this.baseUrl}/api/v1/schedules/${id}/advance`, {});
  }
}
