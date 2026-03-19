import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';

import type {
  VulnerabilityAssessmentView,
  RiskAlertView,
  UpdateRiskAlertInputView,
} from '@elohim/storage-client';

@Injectable({ providedIn: 'root' })
export class RiskApiService {
  private readonly http = inject(HttpClient);
  private readonly baseUrl = '';

  getVulnerability(placeId: string): Observable<VulnerabilityAssessmentView> {
    return this.http.get<VulnerabilityAssessmentView>(
      `${this.baseUrl}/api/v1/risk/vulnerability/${placeId}`,
    );
  }

  listAlerts(query?: {
    placeId?: string;
    status?: string;
    alertType?: string;
    limit?: number;
  }): Observable<RiskAlertView[]> {
    let params = new HttpParams();
    if (query?.placeId) params = params.set('placeId', query.placeId);
    if (query?.status) params = params.set('status', query.status);
    if (query?.alertType) params = params.set('alertType', query.alertType);
    if (query?.limit) params = params.set('limit', query.limit.toString());
    return this.http.get<RiskAlertView[]>(`${this.baseUrl}/api/v1/risk/alerts`, { params });
  }

  updateAlert(id: string, patch: UpdateRiskAlertInputView): Observable<RiskAlertView> {
    return this.http.patch<RiskAlertView>(`${this.baseUrl}/api/v1/risk/alerts/${id}`, patch);
  }
}
