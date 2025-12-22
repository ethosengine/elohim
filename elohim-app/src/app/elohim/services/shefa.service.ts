import { Injectable, inject, signal, computed } from '@angular/core';
import { HolochainClientService } from './holochain-client.service';
import { PerformanceMetricsService } from './performance-metrics.service';
import { CustodianCommitmentService } from './custodian-commitment.service';

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
    uptime_percent: number;        // 0-100
    availability: boolean;         // Currently online
    response_time_p50_ms: number;
    response_time_p95_ms: number;
    response_time_p99_ms: number;
    error_rate: number;           // 0-1 (percentage)
    sla_compliance: boolean;      // Meeting SLA targets
  };

  // Storage metrics
  storage: {
    total_capacity_bytes: number;
    used_bytes: number;
    free_bytes: number;
    utilization_percent: number;
    by_domain: Map<string, number>;
    full_replica_bytes: number;
    threshold_bytes: number;
    erasure_coded_bytes: number;
  };

  // Bandwidth metrics
  bandwidth: {
    declared_mbps: number;
    current_usage_mbps: number;
    peak_usage_mbps: number;
    average_usage_mbps: number;
    utilization_percent: number;
    inbound_mbps: number;
    outbound_mbps: number;
    by_domain: Map<string, number>;
  };

  // Computation metrics
  computation: {
    cpu_cores: number;
    cpu_usage_percent: number;
    memory_gb: number;
    memory_usage_percent: number;
    zome_ops_per_second: number;
    reconstruction_workload_percent: number;
  };

  // Reputation metrics
  reputation: {
    reliability_rating: number;     // 0-5 stars
    speed_rating: number;           // 0-5 stars
    reputation_score: number;       // 0-100
    specialization_bonus: number;   // 0-0.1 (10%)
    commitment_fulfillment: number; // 0-1 (percentage of commitments honored)
  };

  // Economic metrics
  economic: {
    steward_tier: 1 | 2 | 3 | 4;
    price_per_gb: number;           // $/GB/month
    monthly_earnings: number;
    lifetime_earnings: number;
    active_commitments: number;
    total_committed_bytes: number;
  };

  // Timestamp
  collected_at: number;
  last_updated_at: number;
}

@Injectable({
  providedIn: 'root'
})
export class ShefaService {
  private readonly holochain = inject(HolochainClientService);
  private readonly performance = inject(PerformanceMetricsService);
  private readonly commitments = inject(CustodianCommitmentService);

  // Cache custodian metrics (5 minute TTL)
  private metricsCache = new Map<string, { data: CustodianMetrics; timestamp: number }>();
  private readonly CACHE_TTL_MS = 5 * 60 * 1000;

  // All custodians cache
  private allMetricsCache = signal<{ data: CustodianMetrics[]; timestamp: number } | null>(null);

  // Reporting configuration
  private reportingInterval: any;
  private readonly REPORTING_INTERVAL_MS = 5 * 60 * 1000; // 5 minutes

  // Current custodian metrics
  readonly localMetrics = computed(() => {
    // This would be for a custodian node
    // Returns current node's metrics in Shefa format
    const perfMetrics = this.performance.getMetrics();
    return perfMetrics;
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
        payload: { custodian_id: custodianId }
      });

      if (!result.success) {
        console.warn(`[Shefa] Failed to fetch metrics for ${custodianId}:`, result.error);
        return null;
      }

      const metrics = result.data as CustodianMetrics;

      // Cache result
      this.metricsCache.set(custodianId, {
        data: metrics,
        timestamp: Date.now()
      });

      return metrics;
    } catch (err) {
      console.error('[Shefa] Error fetching metrics:', err);
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
        payload: {}
      });

      if (!result.success) {
        console.warn('[Shefa] Failed to fetch all metrics:', result.error);
        return [];
      }

      const metrics = result.data || [];

      // Cache result
      this.allMetricsCache.set({
        data: metrics,
        timestamp: Date.now()
      });

      return metrics;
    } catch (err) {
      console.error('[Shefa] Error fetching all metrics:', err);
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
          metrics: metrics
        }
      });

      if (!result.success) {
        console.warn('[Shefa] Failed to report metrics:', result.error);
        return { success: false, error: result.error };
      }

      // Invalidate cache after update
      this.metricsCache.clear();
      this.allMetricsCache.set(null);

      return { success: true };
    } catch (err) {
      console.error('[Shefa] Error reporting metrics:', err);
      return { success: false, error: String(err) };
    }
  }

  /**
   * Get custodians ranked by health (uptime %)
   */
  async getRankedByHealth(limit: number = 10): Promise<CustodianMetrics[]> {
    const allMetrics = await this.getAllMetrics();
    return allMetrics
      .sort((a, b) => b.health.uptime_percent - a.health.uptime_percent)
      .slice(0, limit);
  }

  /**
   * Get custodians ranked by speed (response time)
   */
  async getRankedBySpeed(limit: number = 10): Promise<CustodianMetrics[]> {
    const allMetrics = await this.getAllMetrics();
    return allMetrics
      .sort((a, b) => a.health.response_time_p95_ms - b.health.response_time_p95_ms)
      .slice(0, limit);
  }

  /**
   * Get custodians ranked by reputation
   */
  async getRankedByReputation(limit: number = 10): Promise<CustodianMetrics[]> {
    const allMetrics = await this.getAllMetrics();
    return allMetrics
      .sort((a, b) => b.reputation.reputation_score - a.reputation.reputation_score)
      .slice(0, limit);
  }

  /**
   * Get custodians available now (online + healthy)
   */
  async getAvailableCustodians(): Promise<CustodianMetrics[]> {
    const allMetrics = await this.getAllMetrics();
    return allMetrics.filter(
      m => m.health.availability && m.health.uptime_percent >= 95 && m.health.sla_compliance
    );
  }

  /**
   * Get alerts for operators
   *
   * @returns Array of alert objects for unhealthy or problematic custodians
   */
  async getAlerts(): Promise<
    Array<{
      custodianId: string;
      severity: 'warning' | 'critical';
      category: string;
      message: string;
      suggestion?: string;
    }>
  > {
    const allMetrics = await this.getAllMetrics();
    const alerts = [];

    for (const metrics of allMetrics) {
      // High memory usage
      if (metrics.computation.memory_usage_percent > 80) {
        alerts.push({
          custodianId: metrics.custodianId,
          severity: 'warning',
          category: 'resource',
          message: `Memory usage high: ${metrics.computation.memory_usage_percent.toFixed(1)}%`,
          suggestion: 'Consider upgrading memory or reducing commitments'
        });
      }

      // High latency
      if (metrics.health.response_time_p95_ms > 1000) {
        alerts.push({
          custodianId: metrics.custodianId,
          severity: 'warning',
          category: 'performance',
          message: `High latency: p95=${metrics.health.response_time_p95_ms}ms`,
          suggestion: 'Investigate network issues or reduce load'
        });
      }

      // Low uptime
      if (metrics.health.uptime_percent < 95) {
        alerts.push({
          custodianId: metrics.custodianId,
          severity: 'critical',
          category: 'reliability',
          message: `Low uptime: ${metrics.health.uptime_percent.toFixed(1)}%`,
          suggestion: 'Investigate outages and improve reliability'
        });
      }

      // High error rate
      if (metrics.health.error_rate > 0.05) {
        alerts.push({
          custodianId: metrics.custodianId,
          severity: 'critical',
          category: 'error',
          message: `High error rate: ${(metrics.health.error_rate * 100).toFixed(1)}%`,
          suggestion: 'Check logs and debug failing operations'
        });
      }

      // Storage full
      if (metrics.storage.utilization_percent > 90) {
        alerts.push({
          custodianId: metrics.custodianId,
          severity: 'critical',
          category: 'storage',
          message: `Storage nearly full: ${metrics.storage.utilization_percent.toFixed(1)}%`,
          suggestion: 'Expand storage capacity or reduce commitments'
        });
      }

      // SLA not met
      if (!metrics.health.sla_compliance) {
        alerts.push({
          custodianId: metrics.custodianId,
          severity: 'critical',
          category: 'sla',
          message: 'Not meeting SLA targets',
          suggestion: 'Take corrective action to restore SLA compliance'
        });
      }
    }

    return alerts;
  }

  /**
   * Get recommendations for custodian operators
   */
  async getRecommendations(): Promise<
    Array<{
      custodianId: string;
      category: string;
      opportunity: string;
      potential_revenue?: number;
    }>
  > {
    const allMetrics = await this.getAllMetrics();
    const recommendations = [];

    for (const metrics of allMetrics) {
      // Available bandwidth opportunity
      const avgUtilization = (metrics.bandwidth.current_usage_mbps / metrics.bandwidth.declared_mbps) * 100;
      if (avgUtilization < 50) {
        const availableMbps = metrics.bandwidth.declared_mbps - metrics.bandwidth.current_usage_mbps;
        recommendations.push({
          custodianId: metrics.custodianId,
          category: 'capacity',
          opportunity: `${availableMbps.toFixed(0)}Mbps available bandwidth - accept more commitments`,
          potential_revenue: (availableMbps / 100) * 500 // Rough estimate
        });
      }

      // CPU capacity opportunity
      if (metrics.computation.cpu_usage_percent < 50) {
        recommendations.push({
          custodianId: metrics.custodianId,
          category: 'capacity',
          opportunity: `Significant CPU capacity available (${100 - metrics.computation.cpu_usage_percent.toFixed(0)}%) - take on computation work`
        });
      }

      // Tier promotion opportunity
      if (metrics.economic.steward_tier < 4 && metrics.health.uptime_percent >= 99) {
        recommendations.push({
          custodianId: metrics.custodianId,
          category: 'tier',
          opportunity: `Eligible for tier promotion - would increase earnings and reputation`
        });
      }

      // Specialization opportunity
      if (metrics.reputation.specialization_bonus < 0.05) {
        recommendations.push({
          custodianId: metrics.custodianId,
          category: 'specialization',
          opportunity: `Develop specialization in specific domains to earn bonus multiplier (+5-10%)`
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
    this.reportingInterval = setInterval(async () => {
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
  private transformToShefaMetrics(perfMetrics: any): CustodianMetrics {
    // This would be implemented for custodian node
    return {} as any;
  }
}
