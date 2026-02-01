import { TestBed } from '@angular/core/testing';
import { PerformanceMetricsService, LocalMetrics, ResponseTimeMetrics } from './performance-metrics.service';

describe('PerformanceMetricsService', () => {
  let service: PerformanceMetricsService;

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [PerformanceMetricsService] });
    service = TestBed.inject(PerformanceMetricsService);
  });

  describe('Service Creation', () => {
    it('should be created', () => { expect(service).toBeTruthy(); });
    it('should be a singleton (providedIn: root)', () => {
      const service2 = TestBed.inject(PerformanceMetricsService);
      expect(service).toBe(service2);
    });
  });

  describe('Property Initialization', () => {
    it('should initialize currentMetrics signal as readonly', () => {
      const metrics = service.currentMetrics();
      expect(metrics).toBeTruthy();
      expect(typeof metrics).toBe('object');
    });
    it('should initialize errorRatePercent computed signal', () => {
      const errorRate = service.errorRatePercent();
      expect(typeof errorRate).toBe('number');
      expect(errorRate).toBeGreaterThanOrEqual(0);
    });
    it('should initialize systemHealthScore computed signal', () => {
      const healthScore = service.systemHealthScore();
      expect(typeof healthScore).toBe('number');
      expect(healthScore).toBeGreaterThanOrEqual(0);
    });
    it('should have initial metrics with queryResponseTimes', () => {
      const metrics = service.currentMetrics();
      expect(metrics.queryResponseTimes).toBeTruthy();
      expect(metrics.queryResponseTimes.count).toBe(0);
    });
    it('should have initial uptime metrics', () => {
      const metrics = service.currentMetrics();
      expect(metrics.startTime).toBeGreaterThan(0);
      expect(metrics.uptimePercent).toBe(100);
      expect(Array.isArray(metrics.downEvents)).toBe(true);
    });
    it('should have initial resource usage metrics', () => {
      const metrics = service.currentMetrics();
      expect(metrics.cpuUsagePercent).toBe(0);
      expect(metrics.memoryUsagePercent).toBe(0);
      expect(metrics.diskUsagePercent).toBe(0);
    });
    it('should have initial operation counters', () => {
      const metrics = service.currentMetrics();
      expect(metrics.queriesProcessed).toBe(0);
      expect(metrics.mutationsProcessed).toBe(0);
      expect(metrics.validationsProcessed).toBe(0);
      expect(metrics.failedOperations).toBe(0);
    });
  });

  describe('Method Existence', () => {
    it('should have recordQuery method', () => { expect(typeof service.recordQuery).toBe('function'); });
    it('should have recordMutation method', () => { expect(typeof service.recordMutation).toBe('function'); });
    it('should have recordValidation method', () => { expect(typeof service.recordValidation).toBe('function'); });
    it('should have updateResourceUsage method', () => { expect(typeof service.updateResourceUsage).toBe('function'); });
    it('should have recordDowntime method', () => { expect(typeof service.recordDowntime).toBe('function'); });
    it('should have updateReplicationWorkload method', () => { expect(typeof service.updateReplicationWorkload).toBe('function'); });
    it('should have getMetrics method', () => { expect(typeof service.getMetrics).toBe('function'); });
    it('should have getMetricsForReport method', () => { expect(typeof service.getMetricsForReport).toBe('function'); });
    it('should have reset method', () => { expect(typeof service.reset).toBe('function'); });
  });

  describe('recordQuery', () => {
    it('should accept duration and success parameters', () => { expect(() => service.recordQuery(100, true)).not.toThrow(); });
    it('should increment queriesProcessed on successful query', () => {
      const before = service.currentMetrics().queriesProcessed;
      service.recordQuery(50, true);
      const after = service.currentMetrics().queriesProcessed;
      expect(after).toBe(before + 1);
    });
    it('should increment failedOperations when success is false', () => {
      const before = service.currentMetrics().failedOperations;
      service.recordQuery(50, false);
      const after = service.currentMetrics().failedOperations;
      expect(after).toBe(before + 1);
    });
    it('should not increment failedOperations when success is true', () => {
      const before = service.currentMetrics().failedOperations;
      service.recordQuery(50, true);
      const after = service.currentMetrics().failedOperations;
      expect(after).toBe(before);
    });
    it('should update queryResponseTimes metrics after recording', () => {
      const before = service.currentMetrics().queryResponseTimes.count;
      service.recordQuery(100, true);
      const after = service.currentMetrics().queryResponseTimes.count;
      expect(after).toBeGreaterThan(before);
    });
    it('should handle multiple query recordings', () => {
      service.recordQuery(50, true);
      service.recordQuery(100, true);
      service.recordQuery(150, true);
      expect(service.currentMetrics().queriesProcessed).toBe(3);
    });
    it('should accept zero duration', () => {
      expect(() => service.recordQuery(0, true)).not.toThrow();
      expect(service.currentMetrics().queriesProcessed).toBe(1);
    });
    it('should accept large duration values', () => {
      expect(() => service.recordQuery(999999, true)).not.toThrow();
    });
  });

  describe('recordMutation', () => {
    it('should accept duration and success parameters', () => { expect(() => service.recordMutation(100, true)).not.toThrow(); });
    it('should increment mutationsProcessed on successful mutation', () => {
      const before = service.currentMetrics().mutationsProcessed;
      service.recordMutation(50, true);
      const after = service.currentMetrics().mutationsProcessed;
      expect(after).toBe(before + 1);
    });
    it('should increment failedOperations when success is false', () => {
      const before = service.currentMetrics().failedOperations;
      service.recordMutation(50, false);
      const after = service.currentMetrics().failedOperations;
      expect(after).toBe(before + 1);
    });
    it('should not increment failedOperations when success is true', () => {
      const before = service.currentMetrics().failedOperations;
      service.recordMutation(50, true);
      const after = service.currentMetrics().failedOperations;
      expect(after).toBe(before);
    });
    it('should handle multiple mutation recordings', () => {
      service.recordMutation(50, true);
      service.recordMutation(100, true);
      service.recordMutation(150, true);
      expect(service.currentMetrics().mutationsProcessed).toBe(3);
    });
  });

  describe('recordValidation', () => {
    it('should accept duration and success parameters', () => { expect(() => service.recordValidation(100, true)).not.toThrow(); });
    it('should increment validationsProcessed on successful validation', () => {
      const before = service.currentMetrics().validationsProcessed;
      service.recordValidation(50, true);
      const after = service.currentMetrics().validationsProcessed;
      expect(after).toBe(before + 1);
    });
    it('should handle multiple validation recordings', () => {
      service.recordValidation(50, true);
      service.recordValidation(100, false);
      service.recordValidation(150, true);
      expect(service.currentMetrics().validationsProcessed).toBe(3);
      expect(service.currentMetrics().failedOperations).toBe(1);
    });
  });

  describe('updateResourceUsage', () => {
    it('should accept cpuPercent, memoryPercent, diskPercent parameters', () => { expect(() => service.updateResourceUsage(50, 60, 70)).not.toThrow(); });
    it('should update cpuUsagePercent', () => {
      service.updateResourceUsage(45, 0, 0);
      expect(service.currentMetrics().cpuUsagePercent).toBe(45);
    });
    it('should update memoryUsagePercent', () => {
      service.updateResourceUsage(0, 65, 0);
      expect(service.currentMetrics().memoryUsagePercent).toBe(65);
    });
    it('should handle 100 percent values', () => {
      service.updateResourceUsage(100, 100, 100);
      const metrics = service.currentMetrics();
      expect(metrics.cpuUsagePercent).toBe(100);
    });
  });

  describe('recordDowntime', () => {
    it('should accept reason and durationMs parameters', () => { expect(() => service.recordDowntime('Network failure', 5000)).not.toThrow(); });
    it('should add downtime event to downEvents array', () => {
      const before = service.currentMetrics().downEvents.length;
      service.recordDowntime('Connection lost', 1000);
      const after = service.currentMetrics().downEvents.length;
      expect(after).toBe(before + 1);
    });
    it('should create downtime event with reason', () => {
      service.recordDowntime('Server unavailable', 5000);
      const event = service.currentMetrics().downEvents[0];
      expect(event.reason).toBe('Server unavailable');
    });
    it('should affect uptimePercent calculation', () => {
      const initialUptime = service.currentMetrics().uptimePercent;
      service.recordDowntime('Downtime event', 5000);
      const afterDowntime = service.currentMetrics().uptimePercent;
      expect(afterDowntime).toBeLessThanOrEqual(initialUptime);
    });
  });

  describe('updateReplicationWorkload', () => {
    it('should accept tasksRunning, reconstructionTasks, avgReconstructionTimeMs parameters', () => { expect(() => service.updateReplicationWorkload(5, 3, 250)).not.toThrow(); });
    it('should update all replication metrics simultaneously', () => {
      service.updateReplicationWorkload(5, 3, 250);
      const metrics = service.currentMetrics();
      expect(metrics.replicationTasksRunning).toBe(5);
      expect(metrics.reconstructionTasksRunning).toBe(3);
      expect(metrics.avgReconstructionTimeMs).toBe(250);
    });
  });

  describe('getMetrics', () => {
    it('should return LocalMetrics object', () => {
      const metrics = service.getMetrics();
      expect(metrics).toBeTruthy();
      expect(typeof metrics).toBe('object');
    });
    it('should reflect changes from recordQuery', () => {
      service.recordQuery(100, true);
      const metrics = service.getMetrics();
      expect(metrics.queriesProcessed).toBe(1);
    });
  });

  describe('getMetricsForReport', () => {
    it('should return report object', () => {
      const report = service.getMetricsForReport();
      expect(report).toBeTruthy();
      expect(typeof report).toBe('object');
    });
    it('should have health.errorRate as decimal (0-1)', () => {
      const report = service.getMetricsForReport();
      expect(report.health.errorRate).toBeDefined();
      expect(typeof report.health.errorRate).toBe('number');
      expect(report.health.errorRate).toBeGreaterThanOrEqual(0);
      expect(report.health.errorRate).toBeLessThanOrEqual(1);
    });
  });

  describe('reset', () => {
    it('should reset queriesProcessed to 0', () => {
      service.recordQuery(100, true);
      service.recordQuery(100, true);
      expect(service.currentMetrics().queriesProcessed).toBe(2);
      service.reset();
      expect(service.currentMetrics().queriesProcessed).toBe(0);
    });
    it('should allow recording after reset', () => {
      service.recordQuery(100, true);
      service.reset();
      expect(service.currentMetrics().queriesProcessed).toBe(0);
      service.recordQuery(100, true);
      expect(service.currentMetrics().queriesProcessed).toBe(1);
    });
  });

  describe('errorRatePercent computed signal', () => {
    it('should return 0 when no operations recorded', () => { expect(service.errorRatePercent()).toBe(0); });
    it('should calculate error rate from failed operations', () => {
      service.recordQuery(100, true);
      service.recordQuery(100, false);
      expect(service.errorRatePercent()).toBe(50);
    });
  });

  describe('systemHealthScore computed signal', () => {
    it('should return a number', () => { expect(typeof service.systemHealthScore()).toBe('number'); });
    it('should have initial health score of 100 (perfect uptime, no errors)', () => { expect(service.systemHealthScore()).toBe(100); });
  });

  // TODO: Add async flow tests - constructor calls startUptimeTracking with setInterval
  // TODO: Add comprehensive mocks - Date.now() calls throughout, timing-dependent tests
  // TODO: Add metrics calculation tests - percentile calculations, error rate formulas, health score algorithm
});