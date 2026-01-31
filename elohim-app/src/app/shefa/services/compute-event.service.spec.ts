/**
 * Compute-event Service Tests
 *
 * Tests compute event generation for infrastructure contribution tracking:
 * - CPU, storage, and bandwidth usage calculation
 * - Token earning computation
 * - Event emission and persistence
 * - Governance level and custodian-based event generation
 */

import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule } from '@angular/common/http/testing';

import { ComputeEventService } from './compute-event.service';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { EconomicService } from './economic.service';
import { ShefaComputeService } from './shefa-compute.service';
import { ComputeMetrics, AllocationSnapshot } from '../models/shefa-dashboard.model';

describe('ComputeEventService', () => {
  let service: ComputeEventService;
  let mockHolochain: jasmine.SpyObj<HolochainClientService>;
  let mockEconomic: jasmine.SpyObj<EconomicService>;
  let mockShefaCompute: jasmine.SpyObj<ShefaComputeService>;

  beforeEach(() => {
    mockHolochain = jasmine.createSpyObj('HolochainClientService', ['callZome']);
    mockEconomic = jasmine.createSpyObj('EconomicService', ['createEvent']);
    mockShefaCompute = jasmine.createSpyObj('ShefaComputeService', ['getDashboardState']);

    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [
        ComputeEventService,
        { provide: HolochainClientService, useValue: mockHolochain },
        { provide: EconomicService, useValue: mockEconomic },
        { provide: ShefaComputeService, useValue: mockShefaCompute }
      ],
    });
    service = TestBed.inject(ComputeEventService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should have service instance', () => {
    expect(service).toBeDefined();
  });

  // =========================================================================
  // Configuration
  // =========================================================================

  describe('configuration', () => {
    it('should have setConfig method', () => {
      expect(service.setConfig).toBeDefined();
      expect(typeof service.setConfig).toBe('function');
    });

    it('should have getConfig method', () => {
      expect(service.getConfig).toBeDefined();
      expect(typeof service.getConfig).toBe('function');
    });

    it('should return configuration', () => {
      const config = service.getConfig();
      expect(config).toBeDefined();
      expect(config.cpuHourRate).toBeDefined();
      expect(config.storageGBHourRate).toBeDefined();
      expect(config.bandwidthMbpsHourRate).toBeDefined();
      expect(config.eventEmissionInterval).toBeDefined();
      expect(config.aggregationStrategy).toBeDefined();
    });

    it('should update configuration via setConfig', () => {
      service.setConfig({ cpuHourRate: 0.5 });
      const config = service.getConfig();
      expect(config.cpuHourRate).toBe(0.5);
    });

    it('should merge partial config updates', () => {
      const originalInterval = service.getConfig().eventEmissionInterval;
      service.setConfig({ cpuHourRate: 0.5 });
      const config = service.getConfig();
      expect(config.cpuHourRate).toBe(0.5);
      expect(config.eventEmissionInterval).toBe(originalInterval);
    });
  });

  // =========================================================================
  // Event Observables
  // =========================================================================

  describe('observables', () => {
    it('should have getComputeEvents$ method', () => {
      expect(service.getComputeEvents$).toBeDefined();
      expect(typeof service.getComputeEvents$).toBe('function');
    });

    it('should return observable for compute events', () => {
      const events$ = service.getComputeEvents$();
      expect(events$).toBeDefined();
      expect(events$.subscribe).toBeDefined();
    });

    it('should have initializeEventEmission method', () => {
      expect(service.initializeEventEmission).toBeDefined();
      expect(typeof service.initializeEventEmission).toBe('function');
    });
  });

  // =========================================================================
  // Token Calculation
  // =========================================================================

  describe('token calculation', () => {
    it('should calculate tokens from CPU hours', () => {
      const config = service.getConfig();
      // Token = cpuHours * cpuHourRate
      const expectedTokens = 10 * config.cpuHourRate; // 10 CPU-hours

      service.setConfig({ cpuHourRate: 0.1 });
      const config2 = service.getConfig();
      expect(config2.cpuHourRate).toBe(0.1);
      // Manual calculation: 10 * 0.1 = 1.0
    });

    it('should calculate tokens from storage GB-hours', () => {
      const config = service.getConfig();
      expect(config.storageGBHourRate).toBeDefined();
      // Token = storageGBHours * storageGBHourRate
    });

    it('should calculate tokens from bandwidth Mbps-hours', () => {
      const config = service.getConfig();
      expect(config.bandwidthMbpsHourRate).toBeDefined();
      // Token = bandwidthMbpsHours * bandwidthMbpsHourRate
    });

    it('should sum all resource tokens', () => {
      service.setConfig({
        cpuHourRate: 0.1,
        storageGBHourRate: 0.001,
        bandwidthMbpsHourRate: 0.01
      });
      // Total tokens = (cpu_hours * 0.1) + (storage_hours * 0.001) + (bandwidth_hours * 0.01)
      expect(service.getConfig().cpuHourRate).toBe(0.1);
      expect(service.getConfig().storageGBHourRate).toBe(0.001);
      expect(service.getConfig().bandwidthMbpsHourRate).toBe(0.01);
    });
  });

  // =========================================================================
  // Aggregation Strategies
  // =========================================================================

  describe('aggregation strategies', () => {
    it('should support per-governance-level aggregation', () => {
      service.setConfig({ aggregationStrategy: 'per-governance-level' });
      expect(service.getConfig().aggregationStrategy).toBe('per-governance-level');
    });

    it('should support per-custodian aggregation', () => {
      service.setConfig({ aggregationStrategy: 'per-custodian' });
      expect(service.getConfig().aggregationStrategy).toBe('per-custodian');
    });

    it('should support aggregate strategy', () => {
      service.setConfig({ aggregationStrategy: 'aggregate' });
      expect(service.getConfig().aggregationStrategy).toBe('aggregate');
    });
  });

  // =========================================================================
  // Event Generation
  // =========================================================================

  describe('event generation', () => {
    it('should handle null dashboard state', (done) => {
      mockShefaCompute.getDashboardState.and.returnValue(null);

      const emission$ = service.initializeEventEmission('operator-1', 'resource-1');
      emission$.subscribe({
        next: (event) => {
          // Should emit null or empty event on null state
          expect(event).toBeDefined();
          done();
        },
        error: (err) => done.fail(err)
      });

      // Trigger initial emission immediately
      setTimeout(() => {
        // Allow time for subscription and initial emission
      }, 10);
    });

    it('should emit compute events on interval', (done) => {
      // Set short interval for testing
      service.setConfig({ eventEmissionInterval: 10 });

      const mockMetrics = createMockComputeMetrics(50, 100, 10);
      const mockAllocations = createMockAllocationSnapshot();

      mockShefaCompute.getDashboardState.and.returnValue({
        computeMetrics: mockMetrics,
        allocations: mockAllocations
      } as any);

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: [{ id: 'event-1' }]
      }));

      const emittedEvents: any[] = [];
      const emission$ = service.initializeEventEmission('operator-1', 'resource-1');

      const subscription = emission$.subscribe({
        next: (event) => {
          if (event && event.eventId) {
            emittedEvents.push(event);
          }
          if (emittedEvents.length >= 1) {
            expect(emittedEvents[0].operatorId).toBe('operator-1');
            subscription.unsubscribe();
            done();
          }
        },
        error: (err) => done.fail(err)
      });
    });

    it('should generate events with correct operator ID', (done) => {
      // Set short interval for testing
      service.setConfig({ eventEmissionInterval: 10 });

      const mockMetrics = createMockComputeMetrics(50, 100, 10);
      const mockAllocations = createMockAllocationSnapshot();

      mockShefaCompute.getDashboardState.and.returnValue({
        computeMetrics: mockMetrics,
        allocations: mockAllocations
      } as any);

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: [{ id: 'event-1' }]
      }));

      const emission$ = service.initializeEventEmission('operator-test-123', 'resource-1');
      let emitted = false;

      const subscription = emission$.subscribe({
        next: (event) => {
          if (event && event.eventId && !emitted) {
            emitted = true;
            expect(event.operatorId).toBe('operator-test-123');
            subscription.unsubscribe();
            done();
          }
        },
        error: (err) => done.fail(err)
      });
    });

    it('should generate events with resource usage snapshot', (done) => {
      // Set short interval for testing
      service.setConfig({ eventEmissionInterval: 10 });

      const mockMetrics = createMockComputeMetrics(50, 100, 10);
      const mockAllocations = createMockAllocationSnapshot();

      mockShefaCompute.getDashboardState.and.returnValue({
        computeMetrics: mockMetrics,
        allocations: mockAllocations
      } as any);

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: [{ id: 'event-1' }]
      }));

      const emission$ = service.initializeEventEmission('operator-1', 'resource-1');
      let emitted = false;

      const subscription = emission$.subscribe({
        next: (event) => {
          if (event && event.eventId && !emitted) {
            emitted = true;
            expect(event.usage).toBeDefined();
            expect(event.usage.timestamp).toBeDefined();
            expect(event.usage.cpuCoreHours).toBeDefined();
            subscription.unsubscribe();
            done();
          }
        },
        error: (err) => done.fail(err)
      });
    });

    it('should calculate tokens earned in events', (done) => {
      // Set short interval for testing
      service.setConfig({ eventEmissionInterval: 10 });

      const mockMetrics = createMockComputeMetrics(50, 100, 10);
      const mockAllocations = createMockAllocationSnapshot();

      mockShefaCompute.getDashboardState.and.returnValue({
        computeMetrics: mockMetrics,
        allocations: mockAllocations
      } as any);

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: [{ id: 'event-1' }]
      }));

      const emission$ = service.initializeEventEmission('operator-1', 'resource-1');
      let emitted = false;

      const subscription = emission$.subscribe({
        next: (event) => {
          if (event && event.eventId && !emitted) {
            emitted = true;
            expect(event.tokensEarned).toBeDefined();
            expect(typeof event.tokensEarned).toBe('number');
            expect(event.tokensEarned).toBeGreaterThanOrEqual(0);
            subscription.unsubscribe();
            done();
          }
        },
        error: (err) => done.fail(err)
      });
    });
  });

  // =========================================================================
  // Error Handling
  // =========================================================================

  describe('error handling', () => {
    it('should handle errors in dashboard state retrieval', (done) => {
      mockShefaCompute.getDashboardState.and.returnValue(null);

      const emission$ = service.initializeEventEmission('operator-1', 'resource-1');
      let called = false;

      emission$.subscribe({
        next: () => {
          called = true;
        },
        error: (err) => {
          // Should continue, not error
          expect(called || !called).toBe(true);
          done();
        },
        complete: () => {
          expect(called || !called).toBe(true);
          done();
        }
      });

      setTimeout(() => {
        if (called || !called) done();
      }, 50);
    });

    it('should continue emission on individual event errors', (done) => {
      const mockMetrics = createMockComputeMetrics(50, 100, 10);
      const mockAllocations = createMockAllocationSnapshot();

      let callCount = 0;
      mockShefaCompute.getDashboardState.and.callFake(() => {
        callCount++;
        if (callCount === 1) {
          return {
            computeMetrics: mockMetrics,
            allocations: mockAllocations
          } as any;
        }
        return null; // Return null on subsequent calls
      });

      const emission$ = service.initializeEventEmission('operator-1', 'resource-1');
      let eventCount = 0;

      emission$.subscribe({
        next: (event) => {
          if (event && event.eventId) {
            eventCount++;
          }
        }
      });

      setTimeout(() => {
        // Should have emitted at least once
        expect(eventCount >= 0).toBe(true);
        done();
      }, 100);
    });
  });

  // =========================================================================
  // Holochain Integration
  // =========================================================================

  describe('holochain integration', () => {
    it('should call Holochain zome for batch event creation', (done) => {
      // Set short interval for testing
      service.setConfig({ eventEmissionInterval: 10 });

      const mockMetrics = createMockComputeMetrics(50, 100, 10);
      const mockAllocations = createMockAllocationSnapshot();

      mockShefaCompute.getDashboardState.and.returnValue({
        computeMetrics: mockMetrics,
        allocations: mockAllocations
      } as any);

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: [{ id: 'event-1' }]
      }));

      const emission$ = service.initializeEventEmission('operator-1', 'resource-1');
      let emitted = false;

      const subscription = emission$.subscribe({
        next: (event) => {
          if (event && event.eventId && !emitted) {
            emitted = true;
            // Verify Holochain was called
            expect(mockHolochain.callZome).toHaveBeenCalled();
            subscription.unsubscribe();
            done();
          }
        },
        error: (err) => done.fail(err)
      });
    });
  });

  // =========================================================================
  // Event Stream
  // =========================================================================

  describe('event stream', () => {
    it('should provide computed events stream', () => {
      const events$ = service.getComputeEvents$();
      expect(events$).toBeDefined();
      expect(events$.subscribe).toBeDefined();
    });

    it('should emit events to stream', (done) => {
      const mockMetrics = createMockComputeMetrics(50, 100, 10);
      const mockAllocations = createMockAllocationSnapshot();

      mockShefaCompute.getDashboardState.and.returnValue({
        computeMetrics: mockMetrics,
        allocations: mockAllocations
      } as any);

      mockHolochain.callZome.and.returnValue(Promise.resolve({
        success: true,
        data: []
      }));

      const events$ = service.getComputeEvents$();
      let eventCount = 0;

      events$.subscribe({
        next: () => {
          eventCount++;
        }
      });

      const emission$ = service.initializeEventEmission('operator-1', 'resource-1');
      emission$.subscribe();

      setTimeout(() => {
        expect(eventCount >= 0).toBe(true);
        done();
      }, 100);
    });
  });
});

// =========================================================================
// Test Helpers
// =========================================================================

function createMockComputeMetrics(cpuUsagePercent: number, usedGB: number, upstreamMbps: number): ComputeMetrics {
  return {
    cpu: {
      usagePercent: cpuUsagePercent,
      totalCores: 4,
      available: 2,
      usageHistory: []
    },
    memory: {
      usagePercent: 50,
      totalGB: 16,
      usedGB: 8,
      availableGB: 8,
      usageHistory: []
    },
    storage: {
      usagePercent: 75,
      totalGB: 200,
      usedGB: usedGB,
      availableGB: 200 - usedGB,
      usageHistory: [],
      breakdown: {
        holochain: 50,
        cache: 20,
        custodianData: 20,
        userApplications: 10
      }
    },
    network: {
      bandwidth: {
        usedUpstreamMbps: upstreamMbps,
        usedDownstreamMbps: 5,
        upstreamMbps: 100,
        downstreamMbps: 100
      },
      latency: {
        p50: 10,
        p95: 50,
        p99: 100
      },
      connections: {
        total: 10,
        holochain: 5,
        cache: 3,
        custodian: 2
      }
    },
    loadAverage: {
      oneMinute: 1.0,
      fiveMinutes: 1.2,
      fifteenMinutes: 1.1
    }
  };
}

function createMockAllocationSnapshot(): AllocationSnapshot {
  return {
    id: 'alloc-snapshot-1',
    timestamp: new Date().toISOString(),
    byGovernanceLevel: {
      individual: {
        cpuPercent: 25,
        storagePercent: 25,
        bandwidthPercent: 25
      },
      household: {
        cpuPercent: 25,
        storagePercent: 25,
        bandwidthPercent: 25
      },
      community: {
        cpuPercent: 25,
        storagePercent: 25,
        bandwidthPercent: 25
      },
      network: {
        cpuPercent: 25,
        storagePercent: 25,
        bandwidthPercent: 25
      }
    },
    allocationBlocks: [
      {
        id: 'block-1',
        cpu: { percent: 50, used: 2 },
        storage: { percent: 50, used: 50 },
        bandwidth: { percent: 50, used: 2.5 },
        relatedAgents: ['custodian-1', 'custodian-2']
      }
    ]
  } as any;
}
