/**
 * ContributorApiService — Thin HTTP client for contributor endpoints.
 *
 * Calls doorway `/api/v1/contributors/*` endpoints, implementing IContributor.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { catchError } from 'rxjs/operators';
import { firstValueFrom, of } from 'rxjs';

import type {
  ContributorDashboardView,
  ContributorImpactView,
  ContributorRecognitionView,
} from '@elohim/storage-client/generated';

import type { IContributor } from '../interfaces/contributor.interface';

@Injectable({ providedIn: 'root' })
export class ContributorApiService implements IContributor {
  private readonly http = inject(HttpClient);

  async getDashboard(presenceId: string): Promise<ContributorDashboardView | null> {
    return firstValueFrom(
      this.http
        .get<ContributorDashboardView>(
          `/api/v1/contributors/${encodeURIComponent(presenceId)}/dashboard`
        )
        .pipe(catchError(() => of(null)))
    );
  }

  async getMyDashboard(): Promise<ContributorDashboardView | null> {
    return firstValueFrom(
      this.http
        .get<ContributorDashboardView>('/api/v1/contributors/me/dashboard')
        .pipe(catchError(() => of(null)))
    );
  }

  async getImpact(presenceId: string): Promise<ContributorImpactView> {
    return firstValueFrom(
      this.http.get<ContributorImpactView>(
        `/api/v1/contributors/${encodeURIComponent(presenceId)}/impact`
      )
    );
  }

  async getRecognitionHistory(presenceId: string): Promise<ContributorRecognitionView> {
    return firstValueFrom(
      this.http.get<ContributorRecognitionView>(
        `/api/v1/contributors/${encodeURIComponent(presenceId)}/recognition`
      )
    );
  }
}
