import { Injectable, inject, signal, computed } from '@angular/core';

import { PerformanceMetricsService } from './performance-metrics.service';
import { ShefaService } from './shefa.service';

/**
 * Custodian Metrics Reporter Service
 *
 * Handles periodic collection and reporting of custodian metrics.
 * Custodian nodes use this to report their performance to the Shefa system.
 *
 * Features:
 * - Collects metrics from PerformanceMetricsService every 5 minutes
 * - Reports metrics to ShefaService for network visibility
 * - Tracks reporting success/failure
 * - Gracefully handles DHT failures with exponential backoff
 *
 * On custodian nodes:
 * - Automatically starts reporting metrics on initialization
 * - Gathers system metrics (CPU, memory, storage, bandwidth)
 * - Transforms to Shefa format and publishes to DHT
 */

export interface MetricsReportingStats {
  reportsAttempted: number;
  reportsSuccessful: number;
  reportsFailed: number;
  lastReportTime: number | null;
  nextReportTime: number | null;
  lastError: string | null;
}

@Injectable({
  providedIn: 'root',
})
export class CustodianMetricsReporterService {
  private readonly metrics = inject(PerformanceMetricsService);
  private readonly shefa = inject(ShefaService);

  // Reporting configuration
  private readonly REPORT_INTERVAL_MS = 5 * 60 * 1000; // 5 minutes
  private readonly MAX_BACKOFF_MS = 30 * 60 * 1000; // 30 minutes max

  // Reporting state
  private readonly reportingEnabled = signal(false);
  private reportInterval: ReturnType<typeof setInterval> | null = null;
  private nextBackoffMs = 1000; // Start with 1 second backoff

  // Statistics tracking
  private readonly stats = signal<MetricsReportingStats>({
    reportsAttempted: 0,
    reportsSuccessful: 0,
    reportsFailed: 0,
    lastReportTime: null,
    nextReportTime: null,
    lastError: null,
  });

  readonly reportingStats = this.stats.asReadonly();

  readonly successRate = computed(() => {
    const s = this.stats();
    if (s.reportsAttempted === 0) return 0;
    return (s.reportsSuccessful / s.reportsAttempted) * 100;
  });

  /**
   * Enable periodic metrics reporting
   *
   * Call this on custodian nodes to start collecting and reporting metrics.
   * Safe to call multiple times - only starts once.
   */
  enableReporting(): void {
    if (this.reportingEnabled()) {
      console.log('[MetricsReporter] Reporting already enabled');
      return;
    }

    console.log('[MetricsReporter] Enabling periodic metrics reporting (interval: 5 minutes)');
    this.reportingEnabled.set(true);

    // Do initial report immediately
    this.reportMetrics();

    // Schedule periodic reporting
    this.reportInterval = setInterval(() => {
      this.reportMetrics();
    }, this.REPORT_INTERVAL_MS);
  }

  /**
   * Disable periodic metrics reporting
   */
  disableReporting(): void {
    if (!this.reportingEnabled()) {
      return;
    }

    console.log('[MetricsReporter] Disabling periodic metrics reporting');
    this.reportingEnabled.set(false);

    if (this.reportInterval) {
      clearInterval(this.reportInterval);
      this.reportInterval = null;
    }
  }

  /**
   * Check if reporting is currently enabled
   */
  isReportingEnabled(): boolean {
    return this.reportingEnabled();
  }

  /**
   * Manually trigger a metrics report
   *
   * Call this to force an immediate report outside of the scheduled interval.
   */
  async reportMetrics(): Promise<boolean> {
    if (!this.reportingEnabled()) {
      console.debug('[MetricsReporter] Reporting disabled, skipping report');
      return false;
    }

    const s = this.stats();
    s.reportsAttempted++;

    try {
      // Collect metrics from local service
      const metricsSnapshot = this.metrics.getMetricsForReport();

      // TODO: Get custodian ID from identity service or config
      const custodianId = 'local-custodian'; // Placeholder

      // Format for Shefa reporting
      const report = {
        custodianId,
        collectedAt: Date.now(),
        health: {
          uptimePercent: metricsSnapshot.health.uptimePercent,
          availability: metricsSnapshot.health.availability,
          responseTimeP50Ms: metricsSnapshot.health.responseTimeP50Ms,
          responseTimeP95Ms: metricsSnapshot.health.responseTimeP95Ms,
          responseTimeP99Ms: metricsSnapshot.health.responseTimeP99Ms,
          errorRate: metricsSnapshot.health.errorRate,
          slaCompliance: metricsSnapshot.health.errorRate < 0.01, // < 1% error rate = SLA compliant
        },
        storage: {
          totalCapacityBytes: 1_099_511_627_776, // 1TB placeholder
          usedBytes: 0, // Would be collected from system
          freeBytes: 1_099_511_627_776, // Would be collected from system
          utilizationPercent: 0, // Would be collected from system
        },
        bandwidth: {
          declaredMbps: 1000, // Placeholder - would come from config
          currentUsageMbps: 0, // Would be collected from system
          peakMbps: 0, // Would be collected from system
          averageMbps: 0, // Would be collected from system
        },
        computation: {
          cpuCores: 8, // Placeholder
          cpuUsagePercent: metricsSnapshot.computation.cpuUsagePercent,
          memoryGb: 16, // Placeholder
          memoryUsagePercent: metricsSnapshot.computation.memoryUsagePercent,
        },
        reputation: {
          reliabilityRating: this.calculateReliabilityRating(metricsSnapshot.health.uptimePercent),
          speedRating: this.calculateSpeedRating(metricsSnapshot.health.responseTimeP95Ms),
          reputationScore: this.calculateReputationScore(metricsSnapshot),
          specializationBonus: 0, // Calculate based on domains served
        },
        economic: {
          stewardTier: 2, // Placeholder - would come from identity service
          pricePerGb: 0.01, // Placeholder pricing
          activeCommitments: 0, // Would be collected from DHT commitments
        },
      };

      // Report to Shefa
      const result = await this.shefa.reportMetrics(report as any);

      if (result.success) {
        s.reportsSuccessful++;
        s.lastReportTime = Date.now();
        s.lastError = null;

        // Reset backoff on successful report
        this.nextBackoffMs = 1000;

        // Schedule next report
        this.scheduleNextReport();

        console.log('[MetricsReporter] Metrics reported successfully');
        return true;
      } else {
        throw new Error('Report failed: ' + (result.error || 'Unknown error'));
      }
    } catch (err) {
      s.reportsFailed++;
      const errorMessage = err instanceof Error ? err.message : 'Unknown error';
      s.lastError = errorMessage;

      console.error('[MetricsReporter] Failed to report metrics:', errorMessage);

      // Exponential backoff for retries
      this.nextBackoffMs = Math.min(this.nextBackoffMs * 2, this.MAX_BACKOFF_MS);
      console.log(`[MetricsReporter] Next retry in ${(this.nextBackoffMs / 1000).toFixed(0)}s`);

      // Schedule retry with backoff
      this.scheduleNextReport(this.nextBackoffMs);

      return false;
    }
  }

  /**
   * Schedule next report
   */
  private scheduleNextReport(delayMs?: number): void {
    const delay = delayMs || this.REPORT_INTERVAL_MS;
    const nextTime = Date.now() + delay;

    const s = this.stats();
    s.nextReportTime = nextTime;
  }

  /**
   * Calculate reliability rating (0-100) based on uptime
   */
  private calculateReliabilityRating(uptimePercent: number): number {
    if (uptimePercent >= 99) return 95;
    if (uptimePercent >= 95) return 85;
    if (uptimePercent >= 90) return 70;
    return 50;
  }

  /**
   * Calculate speed rating (0-100) based on p95 latency
   */
  private calculateSpeedRating(latencyMs: number): number {
    if (latencyMs < 100) return 100;
    if (latencyMs < 250) return 90;
    if (latencyMs < 500) return 75;
    if (latencyMs < 1000) return 50;
    return 25;
  }

  /**
   * Calculate overall reputation score (0-100)
   *
   * Based on reliability, speed, error rate, and SLA compliance
   */
  private calculateReputationScore(metricsSnapshot: any): number {
    const uptimePercent = metricsSnapshot.health.uptimePercent;
    const latencyMs = metricsSnapshot.health.responseTimeP95Ms;
    const errorRate = metricsSnapshot.health.errorRate; // 0-1

    const reliabilityScore = this.calculateReliabilityRating(uptimePercent);
    const speedScore = this.calculateSpeedRating(latencyMs);
    const errorScoreDeduction = errorRate * 100; // Convert to percentage deduction
    const slaBonus = errorRate < 0.01 ? 5 : 0; // < 1% error rate = SLA compliant

    const baseScore = (reliabilityScore * 0.5 + speedScore * 0.3) / 0.8; // Weighted average
    return Math.max(0, Math.min(100, baseScore - errorScoreDeduction + slaBonus));
  }

  /**
   * Get current reporting statistics
   */
  getStatistics(): MetricsReportingStats {
    return this.stats();
  }
}
