import { Injectable, signal, computed } from '@angular/core';

// @coverage: 98.9% (2026-01-31)

/**
 * PerformanceMetricsService
 *
 * Tracks local performance metrics for the custodian:
 * - Response times (p50, p95, p99)
 * - Uptime/availability
 * - Resource usage (CPU, memory)
 * - Error rates
 * - Zome operation counts
 *
 * These metrics are collected locally and periodically
 * reported to the DHT via ShefaService.
 */

export interface ResponseTimeMetrics {
  count: number;
  p50: number; // 50th percentile (median)
  p95: number; // 95th percentile
  p99: number; // 99th percentile
  min: number;
  max: number;
  mean: number;
}

export interface LocalMetrics {
  // Response times (milliseconds)
  queryResponseTimes: ResponseTimeMetrics;
  mutationResponseTimes: ResponseTimeMetrics;

  // Uptime tracking
  startTime: number;
  lastCheckTime: number;
  uptimePercent: number;
  downEvents: { startTime: number; endTime: number; reason: string }[];

  // Resource usage
  cpuUsagePercent: number;
  memoryUsagePercent: number;
  diskUsagePercent: number;

  // Operation counting
  queriesProcessed: number;
  mutationsProcessed: number;
  validationsProcessed: number;
  failedOperations: number;
  errorRate: number;

  // Replication workload
  replicationTasksRunning: number;
  reconstructionTasksRunning: number;
  avgReconstructionTimeMs: number;

  // Timestamp
  collectedAt: number;
}

class PercentileCalculator {
  private values: number[] = [];
  private readonly maxValues = 10000; // Keep last 10K measurements

  add(value: number): void {
    this.values.push(value);
    if (this.values.length > this.maxValues) {
      this.values.shift();
    }
  }

  getP50(): number {
    return this.getPercentile(0.5);
  }

  getP95(): number {
    return this.getPercentile(0.95);
  }

  getP99(): number {
    return this.getPercentile(0.99);
  }

  getMin(): number {
    return this.values.length > 0 ? Math.min(...this.values) : 0;
  }

  getMax(): number {
    return this.values.length > 0 ? Math.max(...this.values) : 0;
  }

  getMean(): number {
    if (this.values.length === 0) return 0;
    const sum = this.values.reduce((a, b) => a + b, 0);
    return sum / this.values.length;
  }

  private getPercentile(p: number): number {
    if (this.values.length === 0) return 0;

    const sorted = [...this.values].sort((a, b) => a - b);
    const index = Math.ceil(sorted.length * p) - 1;
    return sorted[Math.max(0, index)];
  }

  getCount(): number {
    return this.values.length;
  }

  clear(): void {
    this.values = [];
  }
}

@Injectable({
  providedIn: 'root',
})
export class PerformanceMetricsService {
  // Percentile calculators for response times
  private readonly queryPercentiles = new PercentileCalculator();
  private readonly mutationPercentiles = new PercentileCalculator();

  // Metrics storage
  private readonly metrics = signal<LocalMetrics>({
    queryResponseTimes: { count: 0, p50: 0, p95: 0, p99: 0, min: 0, max: 0, mean: 0 },
    mutationResponseTimes: { count: 0, p50: 0, p95: 0, p99: 0, min: 0, max: 0, mean: 0 },
    startTime: Date.now(),
    lastCheckTime: Date.now(),
    uptimePercent: 100,
    downEvents: [],
    cpuUsagePercent: 0,
    memoryUsagePercent: 0,
    diskUsagePercent: 0,
    queriesProcessed: 0,
    mutationsProcessed: 0,
    validationsProcessed: 0,
    failedOperations: 0,
    errorRate: 0,
    replicationTasksRunning: 0,
    reconstructionTasksRunning: 0,
    avgReconstructionTimeMs: 0,
    collectedAt: Date.now(),
  });

  // Expose as readonly signal
  readonly currentMetrics = this.metrics.asReadonly();

  // Computed metrics
  readonly errorRatePercent = computed(() => {
    const m = this.metrics();
    const total = m.queriesProcessed + m.mutationsProcessed;
    if (total === 0) return 0;
    return (m.failedOperations / total) * 100;
  });

  readonly systemHealthScore = computed(() => {
    const m = this.metrics();
    // Score 0-100 based on:
    // - Uptime (40%)
    // - Error rate (30%)
    // - Resource usage (20%)
    // - Replication lag (10%)

    const uptimeScore = m.uptimePercent; // 0-100
    const errorScore = Math.max(0, 100 - this.errorRatePercent()); // Lower errors = higher score
    const resourceScore = Math.max(0, 100 - Math.max(m.cpuUsagePercent, m.memoryUsagePercent));
    const replicationScore = m.replicationTasksRunning === 0 ? 100 : 50;

    return uptimeScore * 0.4 + errorScore * 0.3 + resourceScore * 0.2 + replicationScore * 0.1;
  });

  constructor() {
    // Start tracking uptime
    this.startUptimeTracking();
  }

  /**
   * Record a query operation
   */
  recordQuery(durationMs: number, success: boolean): void {
    this.queryPercentiles.add(durationMs);

    const m = this.metrics();
    m.queriesProcessed++;
    if (!success) {
      m.failedOperations++;
    }

    this.updateMetrics();
  }

  /**
   * Record a mutation operation
   */
  recordMutation(durationMs: number, success: boolean): void {
    this.mutationPercentiles.add(durationMs);

    const m = this.metrics();
    m.mutationsProcessed++;
    if (!success) {
      m.failedOperations++;
    }

    this.updateMetrics();
  }

  /**
   * Record a validation operation
   */
  recordValidation(durationMs: number, success: boolean): void {
    const m = this.metrics();
    m.validationsProcessed++;
    if (!success) {
      m.failedOperations++;
    }

    this.updateMetrics();
  }

  /**
   * Update CPU/Memory/Disk usage
   */
  updateResourceUsage(cpuPercent: number, memoryPercent: number, diskPercent: number): void {
    const m = this.metrics();
    m.cpuUsagePercent = cpuPercent;
    m.memoryUsagePercent = memoryPercent;
    m.diskUsagePercent = diskPercent;
    this.updateMetrics();
  }

  /**
   * Record downtime event
   */
  recordDowntime(reason: string, durationMs: number): void {
    const m = this.metrics();
    const now = Date.now();

    m.downEvents.push({
      startTime: now - durationMs,
      endTime: now,
      reason,
    });

    this.recalculateUptime();
  }

  /**
   * Update replication workload
   */
  updateReplicationWorkload(
    tasksRunning: number,
    reconstructionTasks: number,
    avgReconstructionTimeMs: number
  ): void {
    const m = this.metrics();
    m.replicationTasksRunning = tasksRunning;
    m.reconstructionTasksRunning = reconstructionTasks;
    m.avgReconstructionTimeMs = avgReconstructionTimeMs;
    this.updateMetrics();
  }

  /**
   * Get current metrics snapshot
   */
  getMetrics(): LocalMetrics {
    return this.metrics();
  }

  /**
   * Get metrics for reporting to Shefa
   */
  getMetricsForReport() {
    const m = this.metrics();

    return {
      health: {
        uptimePercent: m.uptimePercent,
        availability: true,
        responseTimeP50Ms: m.queryResponseTimes.p50,
        responseTimeP95Ms: m.queryResponseTimes.p95,
        responseTimeP99Ms: m.queryResponseTimes.p99,
        errorRate: this.errorRatePercent() / 100, // Convert to 0-1
      },

      computation: {
        cpuUsagePercent: m.cpuUsagePercent,
        memoryUsagePercent: m.memoryUsagePercent,
        zomeOpsPerSecond:
          (m.queriesProcessed + m.mutationsProcessed) / ((Date.now() - m.startTime) / 1000),
        reconstructionWorkloadPercent: m.reconstructionTasksRunning > 0 ? 50 : 0,
      },

      operations: {
        queries: m.queriesProcessed,
        mutations: m.mutationsProcessed,
        validations: m.validationsProcessed,
        failed: m.failedOperations,
        query_avg_ms: m.queryResponseTimes.mean,
        mutation_avg_ms: m.mutationResponseTimes.mean,
      },

      timestamp: Date.now(),
    };
  }

  /**
   * Reset metrics (start fresh window)
   */
  reset(): void {
    this.queryPercentiles.clear();
    this.mutationPercentiles.clear();

    this.metrics.set({
      queryResponseTimes: { count: 0, p50: 0, p95: 0, p99: 0, min: 0, max: 0, mean: 0 },
      mutationResponseTimes: { count: 0, p50: 0, p95: 0, p99: 0, min: 0, max: 0, mean: 0 },
      startTime: Date.now(),
      lastCheckTime: Date.now(),
      uptimePercent: 100,
      downEvents: [],
      cpuUsagePercent: 0,
      memoryUsagePercent: 0,
      diskUsagePercent: 0,
      queriesProcessed: 0,
      mutationsProcessed: 0,
      validationsProcessed: 0,
      failedOperations: 0,
      errorRate: 0,
      replicationTasksRunning: 0,
      reconstructionTasksRunning: 0,
      avgReconstructionTimeMs: 0,
      collectedAt: Date.now(),
    });
  }

  /**
   * Private helper: update percentile metrics
   */
  private updateMetrics(): void {
    const m = this.metrics();

    m.queryResponseTimes = {
      count: this.queryPercentiles.getCount(),
      p50: this.queryPercentiles.getP50(),
      p95: this.queryPercentiles.getP95(),
      p99: this.queryPercentiles.getP99(),
      min: this.queryPercentiles.getMin(),
      max: this.queryPercentiles.getMax(),
      mean: this.queryPercentiles.getMean(),
    };

    m.mutationResponseTimes = {
      count: this.mutationPercentiles.getCount(),
      p50: this.mutationPercentiles.getP50(),
      p95: this.mutationPercentiles.getP95(),
      p99: this.mutationPercentiles.getP99(),
      min: this.mutationPercentiles.getMin(),
      max: this.mutationPercentiles.getMax(),
      mean: this.mutationPercentiles.getMean(),
    };

    m.collectedAt = Date.now();
    this.recalculateErrorRate();
    this.recalculateUptime();
  }

  /**
   * Private helper: recalculate error rate
   */
  private recalculateErrorRate(): void {
    const m = this.metrics();
    const total = m.queriesProcessed + m.mutationsProcessed + m.validationsProcessed;
    m.errorRate = total > 0 ? m.failedOperations / total : 0;
  }

  /**
   * Private helper: recalculate uptime
   */
  private recalculateUptime(): void {
    const m = this.metrics();
    const totalTime = Date.now() - m.startTime;
    const downTime = m.downEvents.reduce(
      (sum, event) => sum + (event.endTime - event.startTime),
      0
    );
    const upTime = totalTime - downTime;
    m.uptimePercent = (upTime / totalTime) * 100;
  }

  /**
   * Private helper: start uptime tracking
   */
  private startUptimeTracking(): void {
    // Check periodically if conductor is still responding
    setInterval(() => {
      // In production, would ping conductor here
      // For now, just update lastCheckTime
      const m = this.metrics();
      m.lastCheckTime = Date.now();
    }, 30000); // Check every 30 seconds
  }
}
