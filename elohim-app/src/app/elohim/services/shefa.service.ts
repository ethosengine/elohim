import { Injectable, inject, computed, signal } from '@angular/core';

// @coverage: 13.0% (2026-01-31)

import { CustodianCommitmentService } from './custodian-commitment.service';
import { HolochainClientService } from './holochain-client.service';
import { PerformanceMetricsService } from './performance-metrics.service';

/**
 * ShefaService
 *
 * The Shefa system provides visibility into custodian health and performance.
 *
 * "Shefa" (Hebrew: שפע) = abundance, overflow - representing the flow of
 * content through the network enabled by healthy custodians.
 *
 * Responsibilities:
 * 1. Collect metrics from PerformanceMetricsService
 * 2. Enrich with DHT data (commitments, history)
 * 3. Calculate health scores and reputation
 * 4. Report metrics to DHT periodically
 * 5. Provide queries for custodian selection
 */

export interface CustodianMetrics {
  custodianId: string;
  tier: 1 | 2 | 3 | 4;

  // Health metrics
  health: {
    uptimePercent: number; // 0-100
    availability: boolean; // Currently online
    responseTimeP50Ms: number;
    responseTimeP95Ms: number;
    responseTimeP99Ms: number;
    errorRate: number; // 0-1 (percentage)
    slaCompliance: boolean; // Meeting SLA targets
  };

  // Storage metrics
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

  // Bandwidth metrics
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

  // Computation metrics
  computation: {
    cpuCores: number;
    cpuUsagePercent: number;
    memoryGb: number;
    memoryUsagePercent: number;
    zomeOpsPerSecond: number;
    reconstructionWorkloadPercent: number;
  };

  // Reputation metrics
  reputation: {
    reliabilityRating: number; // 0-5 stars
    speedRating: number; // 0-5 stars
    reputationScore: number; // 0-100
    specializationBonus: number; // 0-0.1 (10%)
    commitmentFulfillment: number; // 0-1 (percentage of commitments honored)
  };

  // Economic metrics
  economic: {
    stewardTier: 1 | 2 | 3 | 4;
    pricePerGb: number; // $/GB/month
    monthlyEarnings: number;
    lifetimeEarnings: number;
    activeCommitments: number;
    totalCommittedBytes: number;
  };

  // Timestamp
  collectedAt: number;
  lastUpdatedAt: number;
}

@Injectable({
  providedIn: 'root',
})
export class ShefaService {
  private readonly holochain = inject(HolochainClientService);
  private readonly performance = inject(PerformanceMetricsService);
  private readonly commitments = inject(CustodianCommitmentService);

  // Cache custodian metrics (5 minute TTL)
  private readonly metricsCache = new Map<string, { data: CustodianMetrics; timestamp: number }>();
  private readonly CACHE_TTL_MS = 5 * 60 * 1000;

  // All custodians cache
  private readonly allMetricsCache = signal<{ data: CustodianMetrics[]; timestamp: number } | null>(
    null
  );

  // Reporting configuration
  private reportingInterval: NodeJS.Timeout | null = null;
  private readonly REPORTING_INTERVAL_MS = 5 * 60 * 1000; // 5 minutes

  // Current custodian metrics
  readonly localMetrics = computed(() => {
    // This would be for a custodian node
    // Returns current node's metrics in Shefa format
    return this.performance.getMetrics();
  });

  constructor() {
    // Start periodic reporting if this is a custodian node
    // (only if we have commitments)
    this.startPeriodicReporting();
  }

  /**
   * Get metrics for a specific custodian
   *
   * @param custodianId - Agent ID of custodian
   * @returns Metrics or null if not found
   */
  async getMetrics(custodianId: string): Promise<CustodianMetrics | null> {
    // Check cache first
    const cached = this.metricsCache.get(custodianId);
    if (cached && Date.now() - cached.timestamp < this.CACHE_TTL_MS) {
      return cached.data;
    }

    try {
      // Query DHT for custodian metrics entry
      const result = await this.holochain.callZome({
        zomeName: 'metrics',
        fnName: 'get_custodian_metrics',
        payload: { custodian_id: custodianId },
      });

      if (!result.success) {
        return null;
      }

      const metrics = result.data as CustodianMetrics;

      // Cache result
      this.metricsCache.set(custodianId, {
        data: metrics,
        timestamp: Date.now(),
      });

      return metrics;
    } catch {
      // Metrics fetch failed - specific custodian metrics unavailable, return null
      return null;
    }
  }

  /**
   * Get metrics for all custodians
   *
   * @returns Array of custodian metrics
   */
  async getAllMetrics(): Promise<CustodianMetrics[]> {
    // Check cache first
    const cached = this.allMetricsCache();
    if (cached && Date.now() - cached.timestamp < this.CACHE_TTL_MS) {
      return cached.data;
    }

    try {
      const result = await this.holochain.callZome({
        zomeName: 'metrics',
        fnName: 'list_all_custodian_metrics',
        payload: {},
      });

      if (!result.success) {
        return [];
      }

      const metrics = Array.isArray(result.data) ? (result.data as CustodianMetrics[]) : [];

      // Cache result
      this.allMetricsCache.set({
        data: metrics,
        timestamp: Date.now(),
      });

      return metrics;
    } catch {
      // Metrics query failed - all custodian metrics unavailable, return empty list
      return [];
    }
  }

  /**
   * Report metrics from custodian to DHT
   *
   * Called periodically by custodian node to publish its metrics.
   */
  async reportMetrics(metrics: CustodianMetrics): Promise<{ success: boolean; error?: string }> {
    try {
      const result = await this.holochain.callZome({
        zomeName: 'metrics',
        fnName: 'report_custodian_metrics',
        payload: {
          metrics: metrics,
        },
      });

      if (!result.success) {
        return { success: false, error: result.error };
      }

      // Invalidate cache after update
      this.metricsCache.clear();
      this.allMetricsCache.set(null);

      return { success: true };
    } catch (err) {
      return { success: false, error: String(err) };
    }
  }

  /**
   * Get custodians ranked by health (uptime %)
   */
  async getRankedByHealth(limit = 10): Promise<CustodianMetrics[]> {
    const allMetrics = await this.getAllMetrics();
    const sorted = [...allMetrics].sort((a, b) => b.health.uptimePercent - a.health.uptimePercent);
    return sorted.slice(0, limit);
  }

  /**
   * Get custodians ranked by speed (response time)
   */
  async getRankedBySpeed(limit = 10): Promise<CustodianMetrics[]> {
    const allMetrics = await this.getAllMetrics();
    const sorted = [...allMetrics].sort(
      (a, b) => a.health.responseTimeP95Ms - b.health.responseTimeP95Ms
    );
    return sorted.slice(0, limit);
  }

  /**
   * Get custodians ranked by reputation
   */
  async getRankedByReputation(limit = 10): Promise<CustodianMetrics[]> {
    const allMetrics = await this.getAllMetrics();
    const sorted = [...allMetrics].sort(
      (a, b) => b.reputation.reputationScore - a.reputation.reputationScore
    );
    return sorted.slice(0, limit);
  }

  /**
   * Get custodians available now (online + healthy)
   */
  async getAvailableCustodians(): Promise<CustodianMetrics[]> {
    const allMetrics = await this.getAllMetrics();
    return allMetrics.filter(
      m => m.health.availability && m.health.uptimePercent >= 95 && m.health.slaCompliance
    );
  }

  /**
   * Get alerts for operators
   *
   * @returns Array of alert objects for unhealthy or problematic custodians
   */
  async getAlerts(): Promise<
    {
      custodianId: string;
      severity: 'warning' | 'critical';
      category: string;
      message: string;
      suggestion?: string;
    }[]
  > {
    const allMetrics = await this.getAllMetrics();
    const alerts: {
      custodianId: string;
      severity: 'warning' | 'critical';
      category: string;
      message: string;
      suggestion?: string;
    }[] = [];

    for (const metrics of allMetrics) {
      // High memory usage
      if (metrics.computation.memoryUsagePercent > 80) {
        alerts.push({
          custodianId: metrics.custodianId,
          severity: 'warning',
          category: 'resource',
          message: `Memory usage high: ${metrics.computation.memoryUsagePercent.toFixed(1)}%`,
          suggestion: 'Consider upgrading memory or reducing commitments',
        });
      }

      // High latency
      if (metrics.health.responseTimeP95Ms > 1000) {
        alerts.push({
          custodianId: metrics.custodianId,
          severity: 'warning',
          category: 'performance',
          message: `High latency: p95=${metrics.health.responseTimeP95Ms}ms`,
          suggestion: 'Investigate network issues or reduce load',
        });
      }

      // Low uptime
      if (metrics.health.uptimePercent < 95) {
        alerts.push({
          custodianId: metrics.custodianId,
          severity: 'critical',
          category: 'reliability',
          message: `Low uptime: ${metrics.health.uptimePercent.toFixed(1)}%`,
          suggestion: 'Investigate outages and improve reliability',
        });
      }

      // High error rate
      if (metrics.health.errorRate > 0.05) {
        alerts.push({
          custodianId: metrics.custodianId,
          severity: 'critical',
          category: 'error',
          message: `High error rate: ${(metrics.health.errorRate * 100).toFixed(1)}%`,
          suggestion: 'Check logs and debug failing operations',
        });
      }

      // Storage full
      if (metrics.storage.utilizationPercent > 90) {
        alerts.push({
          custodianId: metrics.custodianId,
          severity: 'critical',
          category: 'storage',
          message: `Storage nearly full: ${metrics.storage.utilizationPercent.toFixed(1)}%`,
          suggestion: 'Expand storage capacity or reduce commitments',
        });
      }

      // SLA not met
      if (!metrics.health.slaCompliance) {
        alerts.push({
          custodianId: metrics.custodianId,
          severity: 'critical',
          category: 'sla',
          message: 'Not meeting SLA targets',
          suggestion: 'Take corrective action to restore SLA compliance',
        });
      }
    }

    return alerts;
  }

  /**
   * Get recommendations for custodian operators
   */
  async getRecommendations(): Promise<
    {
      custodianId: string;
      category: string;
      opportunity: string;
      potential_revenue?: number;
    }[]
  > {
    const allMetrics = await this.getAllMetrics();
    const recommendations: {
      custodianId: string;
      category: string;
      opportunity: string;
      potential_revenue?: number;
    }[] = [];

    for (const metrics of allMetrics) {
      // Available bandwidth opportunity
      const avgUtilization =
        (metrics.bandwidth.currentUsageMbps / metrics.bandwidth.declaredMbps) * 100;
      if (avgUtilization < 50) {
        const availableMbps = metrics.bandwidth.declaredMbps - metrics.bandwidth.currentUsageMbps;
        recommendations.push({
          custodianId: metrics.custodianId,
          category: 'capacity',
          opportunity: `${availableMbps.toFixed(0)}Mbps available bandwidth - accept more commitments`,
          potential_revenue: (availableMbps / 100) * 500, // Rough estimate
        });
      }

      // CPU capacity opportunity
      if (metrics.computation.cpuUsagePercent < 50) {
        recommendations.push({
          custodianId: metrics.custodianId,
          category: 'capacity',
          opportunity: `Significant CPU capacity available (${Math.round(100 - metrics.computation.cpuUsagePercent)}%) - take on computation work`,
        });
      }

      // Tier promotion opportunity
      if (metrics.economic.stewardTier < 4 && metrics.health.uptimePercent >= 99) {
        recommendations.push({
          custodianId: metrics.custodianId,
          category: 'tier',
          opportunity: `Eligible for tier promotion - would increase earnings and reputation`,
        });
      }

      // Specialization opportunity
      if (metrics.reputation.specializationBonus < 0.05) {
        recommendations.push({
          custodianId: metrics.custodianId,
          category: 'specialization',
          opportunity: `Develop specialization in specific domains to earn bonus multiplier (+5-10%)`,
        });
      }
    }

    return recommendations;
  }

  /**
   * Clear metrics cache
   */
  clearCache(): void {
    this.metricsCache.clear();
    this.allMetricsCache.set(null);
  }

  /**
   * Start periodic reporting of local metrics (for custodian node)
   */
  private startPeriodicReporting(): void {
    this.reportingInterval = setInterval(() => {
      // Only report if this is a custodian node (has commitments)
      // In production, would check if node has commitments
      // For now, skip reporting from app node
      // const custodianId = this.getUserId();
      // const hasCommitments = (await this.commitments.getActiveCommitmentCount(custodianId)) > 0;
      // if (hasCommitments) {
      //   const perfMetrics = this.performance.getMetrics();
      //   const metrics = this.transformToShefaMetrics(perfMetrics);
      //   await this.reportMetrics(metrics);
      // }
    }, this.REPORTING_INTERVAL_MS);
  }

  /**
   * Stop periodic reporting
   */
  stopPeriodicReporting(): void {
    if (this.reportingInterval) {
      clearInterval(this.reportingInterval);
    }
  }

  /**
   * Private helper: transform performance metrics to Shefa format
   */
  private transformToShefaMetrics(_perfMetrics: Record<string, unknown>): CustodianMetrics {
    // This would be implemented for custodian node
    return {} as CustodianMetrics;
  }
}
