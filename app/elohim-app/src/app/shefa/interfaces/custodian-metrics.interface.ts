/**
 * ICustodianMetrics -- Abstract interface for querying, reporting, and
 * ranking custodian health/performance metrics within the Shefa layer.
 *
 * Decouples the metrics lifecycle (DHT queries, caching, periodic reporting,
 * ranking, alerting) from the concrete persistence layer.
 * Consumers inject the CUSTODIAN_METRICS token; the default factory resolves
 * to CustodianMetricsApiService.
 *
 * Tests provide a mock ICustodianMetrics via the same token -- no concrete
 * service import needed.
 *
 * @example
 * ```typescript
 * @Injectable({ providedIn: 'root' })
 * export class DashboardService {
 *   private readonly metrics = inject(CUSTODIAN_METRICS);
 *
 *   async loadCustodian(id: string) {
 *     return this.metrics.getMetrics(id);
 *   }
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import { CustodianMetricsApiService } from '../services/custodian-metrics-api.service';

// =============================================================================
// TYPES
// =============================================================================

/**
 * Complete metrics snapshot for a single custodian node.
 */
export interface CustodianMetrics {
  custodianId: string;
  tier: 1 | 2 | 3 | 4;

  /** Health metrics */
  health: {
    uptimePercent: number;
    availability: boolean;
    responseTimeP50Ms: number;
    responseTimeP95Ms: number;
    responseTimeP99Ms: number;
    errorRate: number;
    slaCompliance: boolean;
  };

  /** Storage metrics */
  storage: {
    totalCapacityBytes: number;
    usedBytes: number;
    freeBytes: number;
    utilizationPercent: number;
    byDomain: Map<string, number>;
    fullReplicaBytes: number;
    thresholdBytes: number;
    erasureCodedBytes: number;
  };

  /** Bandwidth metrics */
  bandwidth: {
    declaredMbps: number;
    currentUsageMbps: number;
    peakUsageMbps: number;
    averageUsageMbps: number;
    utilizationPercent: number;
    inboundMbps: number;
    outboundMbps: number;
    byDomain: Map<string, number>;
  };

  /** Computation metrics */
  computation: {
    cpuCores: number;
    cpuUsagePercent: number;
    memoryGb: number;
    memoryUsagePercent: number;
    zomeOpsPerSecond: number;
    reconstructionWorkloadPercent: number;
  };

  /** Reputation metrics */
  reputation: {
    reliabilityRating: number;
    speedRating: number;
    reputationScore: number;
    specializationBonus: number;
    commitmentFulfillment: number;
  };

  /** Economic metrics */
  economic: {
    stewardTier: 1 | 2 | 3 | 4;
    pricePerGb: number;
    monthlyEarnings: number;
    lifetimeEarnings: number;
    activeCommitments: number;
    totalCommittedBytes: number;
  };

  /** Timestamps */
  collectedAt: number;
  lastUpdatedAt: number;
}

/**
 * Alert for custodian operators.
 */
export interface CustodianAlert {
  custodianId: string;
  severity: 'warning' | 'critical';
  category: string;
  message: string;
  suggestion?: string;
}

/**
 * Recommendation for custodian operators.
 */
export interface CustodianRecommendation {
  custodianId: string;
  category: string;
  opportunity: string;
  potential_revenue?: number;
}

// =============================================================================
// INTERFACE
// =============================================================================

/**
 * Abstract custodian metrics manager -- queries, reports, ranks, and alerts
 * on custodian health and performance.
 *
 * Implementations handle DHT queries, caching, periodic metric reporting,
 * and derived computations (rankings, alerts, recommendations).
 */
export interface ICustodianMetrics {
  /**
   * Get metrics for a specific custodian.
   *
   * @param custodianId - Agent ID of the custodian
   * @returns Metrics snapshot or null if not found
   */
  getMetrics(custodianId: string): Promise<CustodianMetrics | null>;

  /**
   * Get metrics for all custodians.
   *
   * @returns Array of all custodian metrics
   */
  getAllMetrics(): Promise<CustodianMetrics[]>;

  /**
   * Report metrics from a custodian node to the DHT.
   *
   * @param metrics - Complete metrics snapshot to publish
   * @returns Success status with optional error message
   */
  reportMetrics(metrics: CustodianMetrics): Promise<{ success: boolean; error?: string }>;

  /**
   * Get custodians ranked by health (uptime %).
   *
   * @param limit - Maximum number of results (default 10)
   * @returns Custodians sorted by uptime descending
   */
  getRankedByHealth(limit?: number): Promise<CustodianMetrics[]>;

  /**
   * Get custodians ranked by speed (response time).
   *
   * @param limit - Maximum number of results (default 10)
   * @returns Custodians sorted by p95 response time ascending
   */
  getRankedBySpeed(limit?: number): Promise<CustodianMetrics[]>;

  /**
   * Get custodians ranked by reputation.
   *
   * @param limit - Maximum number of results (default 10)
   * @returns Custodians sorted by reputation score descending
   */
  getRankedByReputation(limit?: number): Promise<CustodianMetrics[]>;

  /**
   * Get custodians that are currently available (online + healthy).
   *
   * @returns Available custodians meeting SLA requirements
   */
  getAvailableCustodians(): Promise<CustodianMetrics[]>;

  /**
   * Get alerts for custodian operators.
   *
   * @returns Alerts for unhealthy or problematic custodians
   */
  getAlerts(): Promise<CustodianAlert[]>;

  /**
   * Get recommendations for custodian operators.
   *
   * @returns Optimization opportunities and suggestions
   */
  getRecommendations(): Promise<CustodianRecommendation[]>;

  /**
   * Clear the metrics cache.
   */
  clearCache(): void;

  /**
   * Stop periodic metric reporting.
   */
  stopPeriodicReporting(): void;
}

// =============================================================================
// INJECTION TOKEN
// =============================================================================

/**
 * Injection token for custodian metrics management.
 *
 * Default factory resolves to CustodianMetricsApiService which provides:
 * - DHT queries for custodian metrics
 * - Caching with configurable TTL
 * - Periodic metric reporting for custodian nodes
 * - Ranking by health, speed, and reputation
 * - Alerting for unhealthy custodians
 * - Operator recommendations
 *
 * Override in tests:
 * ```typescript
 * { provide: CUSTODIAN_METRICS, useValue: mockCustodianMetrics }
 * ```
 */
export const CUSTODIAN_METRICS = new InjectionToken<ICustodianMetrics>('CustodianMetrics', {
  providedIn: 'root',
  factory: () => inject(CustodianMetricsApiService),
});
