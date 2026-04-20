import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { Observable } from 'rxjs';

import type {
  ComputeRouteInputView,
  RouteResultView,
  DistributionRequestView,
  DistributionPlanView,
} from '@elohim/storage-client';

@Injectable({ providedIn: 'root' })
export class RoutingApiService {
  private readonly http = inject(HttpClient);
  private readonly baseUrl = '';

  computeRoute(input: ComputeRouteInputView): Observable<RouteResultView> {
    return this.http.post<RouteResultView>(`${this.baseUrl}/api/v1/routing/compute`, input);
  }

  computeDistribution(input: DistributionRequestView): Observable<DistributionPlanView> {
    return this.http.post<DistributionPlanView>(
      `${this.baseUrl}/api/v1/routing/distribution`,
      input
    );
  }
}
