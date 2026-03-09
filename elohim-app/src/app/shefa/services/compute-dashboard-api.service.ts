/**
 * ComputeDashboardApiService -- Thin HTTP client for the compute dashboard.
 *
 * Calls doorway `/api/v1/compute/dashboard` endpoints, implementing
 * IComputeDashboard. All aggregation (metrics, allocations, protection,
 * tokens, constitutional limits) is performed server-side in Rust.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { Observable, firstValueFrom, from, interval, switchMap, startWith, tap } from 'rxjs';

import type { IComputeDashboard } from '../interfaces/compute-dashboard.interface';
import type {
  SheafaDashboardState,
  NodeTopologyState,
  ComputeNeedsAssessment,
  StorageContentDistribution,
  BidirectionalCustodianView,
} from '../models/shefa-dashboard.model';

@Injectable({ providedIn: 'root' })
export class ComputeDashboardApiService implements IComputeDashboard {
  private readonly http = inject(HttpClient);
  private cachedState: SheafaDashboardState | null = null;

  async getDashboard(): Promise<SheafaDashboardState> {
    return firstValueFrom(this.http.get<SheafaDashboardState>('/api/v1/compute/dashboard'));
  }

  async getDashboardForGovernanceLevel(level: string): Promise<SheafaDashboardState> {
    return firstValueFrom(
      this.http.get<SheafaDashboardState>('/api/v1/compute/dashboard', { params: { level } })
    );
  }

  async refreshDashboard(): Promise<SheafaDashboardState> {
    return firstValueFrom(
      this.http.post<SheafaDashboardState>('/api/v1/compute/dashboard/refresh', {})
    );
  }

  initializeDashboard(
    _operatorId: string,
    _stewardedResourceId: string
  ): Observable<SheafaDashboardState> {
    return interval(5000).pipe(
      startWith(0),
      switchMap(() => from(this.getDashboard())),
      tap(state => {
        this.cachedState = state;
      })
    );
  }

  getDashboardState(): SheafaDashboardState | null {
    return this.cachedState;
  }

  getNodeTopology(operatorId: string): Observable<NodeTopologyState> {
    return this.http.get<NodeTopologyState>('/api/v1/compute/topology', {
      params: { operatorId },
    });
  }

  getComputeNeedsAssessment(operatorId: string): Observable<ComputeNeedsAssessment> {
    return this.http.get<ComputeNeedsAssessment>('/api/v1/compute/needs-assessment', {
      params: { operatorId },
    });
  }

  getStorageContentDistribution(operatorId: string): Observable<StorageContentDistribution> {
    return this.http.get<StorageContentDistribution>('/api/v1/compute/storage-distribution', {
      params: { operatorId },
    });
  }

  getBidirectionalCustodianView(operatorId: string): Observable<BidirectionalCustodianView> {
    return this.http.get<BidirectionalCustodianView>('/api/v1/compute/custodian-view', {
      params: { operatorId },
    });
  }
}
