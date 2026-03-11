/**
 * CustodianMetricsApiService — Thin HTTP client for custodian metrics.
 *
 * Calls doorway `/api/v1/custodians/metrics/*` endpoints, implementing
 * ICustodianMetrics. Business logic lives behind the Rust API boundary.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import type {
  CustodianAlert,
  CustodianMetrics,
  CustodianRecommendation,
  ICustodianMetrics,
} from '../interfaces/custodian-metrics.interface';

@Injectable({ providedIn: 'root' })
export class CustodianMetricsApiService implements ICustodianMetrics {
  private readonly http = inject(HttpClient);

  async getMetrics(custodianId: string): Promise<CustodianMetrics | null> {
    return firstValueFrom(
      this.http.get<CustodianMetrics | null>(`/api/v1/custodians/metrics/${custodianId}`)
    );
  }

  async getAllMetrics(): Promise<CustodianMetrics[]> {
    return firstValueFrom(this.http.get<CustodianMetrics[]>('/api/v1/custodians/metrics'));
  }

  async reportMetrics(
    metrics: CustodianMetrics
  ): Promise<{ success: boolean; error?: string }> {
    return firstValueFrom(
      this.http.post<{ success: boolean; error?: string }>(
        '/api/v1/custodians/metrics',
        metrics
      )
    );
  }

  async getRankedByHealth(limit?: number): Promise<CustodianMetrics[]> {
    return firstValueFrom(
      this.http.get<CustodianMetrics[]>('/api/v1/custodians/metrics', {
        params: { rank: 'health', ...(limit != null ? { limit: limit.toString() } : {}) },
      })
    );
  }

  async getRankedBySpeed(limit?: number): Promise<CustodianMetrics[]> {
    return firstValueFrom(
      this.http.get<CustodianMetrics[]>('/api/v1/custodians/metrics', {
        params: { rank: 'speed', ...(limit != null ? { limit: limit.toString() } : {}) },
      })
    );
  }

  async getRankedByReputation(limit?: number): Promise<CustodianMetrics[]> {
    return firstValueFrom(
      this.http.get<CustodianMetrics[]>('/api/v1/custodians/metrics', {
        params: { rank: 'reputation', ...(limit != null ? { limit: limit.toString() } : {}) },
      })
    );
  }

  async getAvailableCustodians(): Promise<CustodianMetrics[]> {
    return firstValueFrom(
      this.http.get<CustodianMetrics[]>('/api/v1/custodians/metrics', {
        params: { available: 'true' },
      })
    );
  }

  async getAlerts(): Promise<CustodianAlert[]> {
    return firstValueFrom(
      this.http.get<CustodianAlert[]>('/api/v1/custodians/metrics/alerts')
    );
  }

  async getRecommendations(): Promise<CustodianRecommendation[]> {
    return firstValueFrom(
      this.http.get<CustodianRecommendation[]>(
        '/api/v1/custodians/metrics/recommendations'
      )
    );
  }

  clearCache(): void {
    // No-op: cache management is handled server-side
  }

  stopPeriodicReporting(): void {
    // No-op: periodic reporting is handled server-side
  }
}
