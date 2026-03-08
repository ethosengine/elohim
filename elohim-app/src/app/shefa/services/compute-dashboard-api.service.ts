/**
 * ComputeDashboardApiService -- Thin HTTP client for the compute dashboard.
 *
 * Calls doorway `/api/v1/compute/dashboard` endpoints, implementing
 * IComputeDashboard. All aggregation (metrics, allocations, protection,
 * tokens, constitutional limits) is performed server-side in Rust.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import type { IComputeDashboard } from '../interfaces/compute-dashboard.interface';
import type { SheafaDashboardState } from '../models/shefa-dashboard.model';

@Injectable({ providedIn: 'root' })
export class ComputeDashboardApiService implements IComputeDashboard {
  private readonly http = inject(HttpClient);

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
}
