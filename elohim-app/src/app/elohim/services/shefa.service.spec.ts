import { TestBed } from '@angular/core/testing';

import { CustodianCommitmentService } from './custodian-commitment.service';
import { HolochainClientService } from './holochain-client.service';
import { PerformanceMetricsService } from './performance-metrics.service';
import { ShefaService } from './shefa.service';

describe('ShefaService', () => {
  let service: ShefaService;
  let clientMock: jasmine.SpyObj<HolochainClientService>;
  let perfMock: jasmine.SpyObj<PerformanceMetricsService>;
  let commitmentMock: jasmine.SpyObj<CustodianCommitmentService>;

  beforeEach(() => {
    const clientSpy = jasmine.createSpyObj('HolochainClientService', ['callZome']);
    const perfSpy = jasmine.createSpyObj('PerformanceMetricsService', ['getMetrics']);
    const commitmentSpy = jasmine.createSpyObj('CustodianCommitmentService', [
      'getActiveCommitmentCount',
    ]);

    TestBed.configureTestingModule({
      providers: [
        ShefaService,
        { provide: HolochainClientService, useValue: clientSpy },
        { provide: PerformanceMetricsService, useValue: perfSpy },
        { provide: CustodianCommitmentService, useValue: commitmentSpy },
      ],
    });

    service = TestBed.inject(ShefaService);
    clientMock = TestBed.inject(HolochainClientService) as jasmine.SpyObj<HolochainClientService>;
    perfMock = TestBed.inject(PerformanceMetricsService) as jasmine.SpyObj<PerformanceMetricsService>;
    commitmentMock = TestBed.inject(
      CustodianCommitmentService
    ) as jasmine.SpyObj<CustodianCommitmentService>;
  });

  afterEach(() => {
    if (service) {
      service.stopPeriodicReporting();
    }
  });

  // ==========================================================================
  // Service Creation
  // ==========================================================================

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // ==========================================================================
  // Public Methods
  // ==========================================================================

  it('should have getMetrics method', () => {
    expect(service.getMetrics).toBeDefined();
    expect(typeof service.getMetrics).toBe('function');
  });

  it('should have getAllMetrics method', () => {
    expect(service.getAllMetrics).toBeDefined();
    expect(typeof service.getAllMetrics).toBe('function');
  });

  it('should have reportMetrics method', () => {
    expect(service.reportMetrics).toBeDefined();
    expect(typeof service.reportMetrics).toBe('function');
  });

  it('should have getRankedByHealth method', () => {
    expect(service.getRankedByHealth).toBeDefined();
    expect(typeof service.getRankedByHealth).toBe('function');
  });

  it('should have getRankedBySpeed method', () => {
    expect(service.getRankedBySpeed).toBeDefined();
    expect(typeof service.getRankedBySpeed).toBe('function');
  });

  it('should have getRankedByReputation method', () => {
    expect(service.getRankedByReputation).toBeDefined();
    expect(typeof service.getRankedByReputation).toBe('function');
  });

  it('should have getAvailableCustodians method', () => {
    expect(service.getAvailableCustodians).toBeDefined();
    expect(typeof service.getAvailableCustodians).toBe('function');
  });

  it('should have getAlerts method', () => {
    expect(service.getAlerts).toBeDefined();
    expect(typeof service.getAlerts).toBe('function');
  });

  it('should have getRecommendations method', () => {
    expect(service.getRecommendations).toBeDefined();
    expect(typeof service.getRecommendations).toBe('function');
  });

  it('should have clearCache method', () => {
    expect(service.clearCache).toBeDefined();
    expect(typeof service.clearCache).toBe('function');
  });

  it('should have stopPeriodicReporting method', () => {
    expect(service.stopPeriodicReporting).toBeDefined();
    expect(typeof service.stopPeriodicReporting).toBe('function');
  });

  // ==========================================================================
  // Public Signals
  // ==========================================================================

  it('should have localMetrics computed signal', () => {
    expect(service.localMetrics).toBeDefined();
  });

  // ==========================================================================
  // getMetrics - Fetch Single Custodian Metrics
  // ==========================================================================

  it('should get metrics for a single custodian', async () => {
    const mockMetrics = {
      custodianId: 'custodian-1',
      tier: 2 as 1 | 2 | 3 | 4,
      health: {
        uptimePercent: 99.5,
        availability: true,
        responseTimeP50Ms: 50,
        responseTimeP95Ms: 150,
        responseTimeP99Ms: 300,
        errorRate: 0.001,
        slaCompliance: true,
      },
      storage: {
        totalCapacityBytes: 1_000_000_000,
        usedBytes: 600_000_000,
        freeBytes: 400_000_000,
        utilizationPercent: 60,
        byDomain: new Map(),
        fullReplicaBytes: 500_000_000,
        thresholdBytes: 800_000_000,
        erasureCodedBytes: 100_000_000,
      },
      bandwidth: {
        declaredMbps: 1000,
        currentUsageMbps: 400,
        peakUsageMbps: 750,
        averageUsageMbps: 450,
        utilizationPercent: 45,
        inboundMbps: 200,
        outboundMbps: 200,
        byDomain: new Map(),
      },
      computation: {
        cpuCores: 8,
        cpuUsagePercent: 35,
        memoryGb: 32,
        memoryUsagePercent: 55,
        zomeOpsPerSecond: 150,
        reconstructionWorkloadPercent: 10,
      },
      reputation: {
        reliabilityRating: 4.5,
        speedRating: 4.8,
        reputationScore: 92,
        specializationBonus: 0.05,
        commitmentFulfillment: 0.98,
      },
      economic: {
        stewardTier: 2 as 1 | 2 | 3 | 4,
        pricePerGb: 0.50,
        monthlyEarnings: 15000,
        lifetimeEarnings: 150000,
        activeCommitments: 5,
        totalCommittedBytes: 500_000_000,
      },
      collectedAt: Date.now(),
      lastUpdatedAt: Date.now(),
    };

    clientMock.callZome.and.returnValue(Promise.resolve({ success: true, data: mockMetrics }));

    const result = await service.getMetrics('custodian-1');

    expect(result).toBeDefined();
    expect(result?.custodianId).toBe('custodian-1');
    expect(result?.health.uptimePercent).toBe(99.5);
  });

  it('should return null when getMetrics fails', async () => {
    clientMock.callZome.and.returnValue(Promise.resolve({ success: false, data: null }));

    const result = await service.getMetrics('custodian-1');

    expect(result).toBeNull();
  });

  it('should cache metrics and return cached value on subsequent call', async () => {
    const mockMetrics = {
      custodianId: 'custodian-1',
      tier: 1 as 1 | 2 | 3 | 4,
      health: {
        uptimePercent: 99.5,
        availability: true,
        responseTimeP50Ms: 50,
        responseTimeP95Ms: 150,
        responseTimeP99Ms: 300,
        errorRate: 0.001,
        slaCompliance: true,
      },
      storage: {
        totalCapacityBytes: 1_000_000_000,
        usedBytes: 600_000_000,
        freeBytes: 400_000_000,
        utilizationPercent: 60,
        byDomain: new Map(),
        fullReplicaBytes: 500_000_000,
        thresholdBytes: 800_000_000,
        erasureCodedBytes: 100_000_000,
      },
      bandwidth: {
        declaredMbps: 1000,
        currentUsageMbps: 400,
        peakUsageMbps: 750,
        averageUsageMbps: 450,
        utilizationPercent: 45,
        inboundMbps: 200,
        outboundMbps: 200,
        byDomain: new Map(),
      },
      computation: {
        cpuCores: 8,
        cpuUsagePercent: 35,
        memoryGb: 32,
        memoryUsagePercent: 55,
        zomeOpsPerSecond: 150,
        reconstructionWorkloadPercent: 10,
      },
      reputation: {
        reliabilityRating: 4.5,
        speedRating: 4.8,
        reputationScore: 92,
        specializationBonus: 0.05,
        commitmentFulfillment: 0.98,
      },
      economic: {
        stewardTier: 1 as 1 | 2 | 3 | 4,
        pricePerGb: 0.50,
        monthlyEarnings: 15000,
        lifetimeEarnings: 150000,
        activeCommitments: 5,
        totalCommittedBytes: 500_000_000,
      },
      collectedAt: Date.now(),
      lastUpdatedAt: Date.now(),
    };

    clientMock.callZome.and.returnValue(Promise.resolve({ success: true, data: mockMetrics }));

    await service.getMetrics('custodian-1');
    await service.getMetrics('custodian-1');

    expect(clientMock.callZome).toHaveBeenCalledTimes(1);
  });

  // ==========================================================================
  // getAllMetrics - Fetch All Custodian Metrics
  // ==========================================================================

  it('should get all custodian metrics', async () => {
    const mockAllMetrics = [
      {
        custodianId: 'custodian-1',
        tier: 2 as 1 | 2 | 3 | 4,
        health: {
          uptimePercent: 99.5,
          availability: true,
          responseTimeP50Ms: 50,
          responseTimeP95Ms: 150,
          responseTimeP99Ms: 300,
          errorRate: 0.001,
          slaCompliance: true,
        },
        storage: {
          totalCapacityBytes: 1_000_000_000,
          usedBytes: 600_000_000,
          freeBytes: 400_000_000,
          utilizationPercent: 60,
          byDomain: new Map(),
          fullReplicaBytes: 500_000_000,
          thresholdBytes: 800_000_000,
          erasureCodedBytes: 100_000_000,
        },
        bandwidth: {
          declaredMbps: 1000,
          currentUsageMbps: 400,
          peakUsageMbps: 750,
          averageUsageMbps: 450,
          utilizationPercent: 45,
          inboundMbps: 200,
          outboundMbps: 200,
          byDomain: new Map(),
        },
        computation: {
          cpuCores: 8,
          cpuUsagePercent: 35,
          memoryGb: 32,
          memoryUsagePercent: 55,
          zomeOpsPerSecond: 150,
          reconstructionWorkloadPercent: 10,
        },
        reputation: {
          reliabilityRating: 4.5,
          speedRating: 4.8,
          reputationScore: 92,
          specializationBonus: 0.05,
          commitmentFulfillment: 0.98,
        },
        economic: {
          stewardTier: 2,
          pricePerGb: 0.50,
          monthlyEarnings: 15000,
          lifetimeEarnings: 150000,
          activeCommitments: 5,
          totalCommittedBytes: 500_000_000,
        },
        collectedAt: Date.now(),
        lastUpdatedAt: Date.now(),
      },
    ];

    clientMock.callZome.and.returnValue(
      Promise.resolve({ success: true, data: mockAllMetrics })
    );

    const result = await service.getAllMetrics();

    expect(result).toBeDefined();
    expect(result.length).toBe(1);
  });

  it('should return empty array when getAllMetrics fails', async () => {
    clientMock.callZome.and.returnValue(Promise.resolve({ success: false, data: null }));

    const result = await service.getAllMetrics();

    expect(result).toEqual([]);
  });

  // ==========================================================================
  // reportMetrics - Report Custodian Metrics
  // ==========================================================================

  it('should report metrics successfully', async () => {
    const mockMetrics = {
      custodianId: 'custodian-1',
      tier: 1 as 1 | 2 | 3 | 4,
      health: {
        uptimePercent: 99.9,
        availability: true,
        responseTimeP50Ms: 45,
        responseTimeP95Ms: 120,
        responseTimeP99Ms: 250,
        errorRate: 0.0005,
        slaCompliance: true,
      },
      storage: {
        totalCapacityBytes: 2_000_000_000,
        usedBytes: 1_200_000_000,
        freeBytes: 800_000_000,
        utilizationPercent: 60,
        byDomain: new Map(),
        fullReplicaBytes: 1_000_000_000,
        thresholdBytes: 1_600_000_000,
        erasureCodedBytes: 200_000_000,
      },
      bandwidth: {
        declaredMbps: 2000,
        currentUsageMbps: 800,
        peakUsageMbps: 1500,
        averageUsageMbps: 900,
        utilizationPercent: 45,
        inboundMbps: 400,
        outboundMbps: 400,
        byDomain: new Map(),
      },
      computation: {
        cpuCores: 16,
        cpuUsagePercent: 40,
        memoryGb: 64,
        memoryUsagePercent: 50,
        zomeOpsPerSecond: 300,
        reconstructionWorkloadPercent: 8,
      },
      reputation: {
        reliabilityRating: 4.8,
        speedRating: 4.9,
        reputationScore: 95,
        specializationBonus: 0.08,
        commitmentFulfillment: 0.99,
      },
      economic: {
        stewardTier: 1 as 1 | 2 | 3 | 4,
        pricePerGb: 0.45,
        monthlyEarnings: 25000,
        lifetimeEarnings: 300000,
        activeCommitments: 10,
        totalCommittedBytes: 1_000_000_000,
      },
      collectedAt: Date.now(),
      lastUpdatedAt: Date.now(),
    };

    clientMock.callZome.and.returnValue(Promise.resolve({ success: true, data: null }));

    const result = await service.reportMetrics(mockMetrics);

    expect(result.success).toBe(true);
  });

  it('should return error when reportMetrics fails', async () => {
    const mockMetrics = {
      custodianId: 'custodian-1',
      tier: 1 as 1 | 2 | 3 | 4,
      health: {
        uptimePercent: 99.9,
        availability: true,
        responseTimeP50Ms: 45,
        responseTimeP95Ms: 120,
        responseTimeP99Ms: 250,
        errorRate: 0.0005,
        slaCompliance: true,
      },
      storage: {
        totalCapacityBytes: 2_000_000_000,
        usedBytes: 1_200_000_000,
        freeBytes: 800_000_000,
        utilizationPercent: 60,
        byDomain: new Map(),
        fullReplicaBytes: 1_000_000_000,
        thresholdBytes: 1_600_000_000,
        erasureCodedBytes: 200_000_000,
      },
      bandwidth: {
        declaredMbps: 2000,
        currentUsageMbps: 800,
        peakUsageMbps: 1500,
        averageUsageMbps: 900,
        utilizationPercent: 45,
        inboundMbps: 400,
        outboundMbps: 400,
        byDomain: new Map(),
      },
      computation: {
        cpuCores: 16,
        cpuUsagePercent: 40,
        memoryGb: 64,
        memoryUsagePercent: 50,
        zomeOpsPerSecond: 300,
        reconstructionWorkloadPercent: 8,
      },
      reputation: {
        reliabilityRating: 4.8,
        speedRating: 4.9,
        reputationScore: 95,
        specializationBonus: 0.08,
        commitmentFulfillment: 0.99,
      },
      economic: {
        stewardTier: 1 as 1 | 2 | 3 | 4,
        pricePerGb: 0.45,
        monthlyEarnings: 25000,
        lifetimeEarnings: 300000,
        activeCommitments: 10,
        totalCommittedBytes: 1_000_000_000,
      },
      collectedAt: Date.now(),
      lastUpdatedAt: Date.now(),
    };

    clientMock.callZome.and.returnValue(
      Promise.resolve({ success: false, error: 'Storage full' })
    );

    const result = await service.reportMetrics(mockMetrics);

    expect(result.success).toBe(false);
    expect(result.error).toBe('Storage full');
  });

  // ==========================================================================
  // Ranking Methods
  // ==========================================================================

  it('should get custodians ranked by health', async () => {
    const mockMetrics = [
      {
        custodianId: 'custodian-1',
        tier: 1 as 1 | 2 | 3 | 4,
        health: {
          uptimePercent: 99.5,
          availability: true,
          responseTimeP50Ms: 50,
          responseTimeP95Ms: 150,
          responseTimeP99Ms: 300,
          errorRate: 0.001,
          slaCompliance: true,
        },
        storage: {
          totalCapacityBytes: 1_000_000_000,
          usedBytes: 600_000_000,
          freeBytes: 400_000_000,
          utilizationPercent: 60,
          byDomain: new Map(),
          fullReplicaBytes: 500_000_000,
          thresholdBytes: 800_000_000,
          erasureCodedBytes: 100_000_000,
        },
        bandwidth: {
          declaredMbps: 1000,
          currentUsageMbps: 400,
          peakUsageMbps: 750,
          averageUsageMbps: 450,
          utilizationPercent: 45,
          inboundMbps: 200,
          outboundMbps: 200,
          byDomain: new Map(),
        },
        computation: {
          cpuCores: 8,
          cpuUsagePercent: 35,
          memoryGb: 32,
          memoryUsagePercent: 55,
          zomeOpsPerSecond: 150,
          reconstructionWorkloadPercent: 10,
        },
        reputation: {
          reliabilityRating: 4.5,
          speedRating: 4.8,
          reputationScore: 92,
          specializationBonus: 0.05,
          commitmentFulfillment: 0.98,
        },
        economic: {
          stewardTier: 1 as 1 | 2 | 3 | 4,
          pricePerGb: 0.50,
          monthlyEarnings: 15000,
          lifetimeEarnings: 150000,
          activeCommitments: 5,
          totalCommittedBytes: 500_000_000,
        },
        collectedAt: Date.now(),
        lastUpdatedAt: Date.now(),
      },
    ];

    clientMock.callZome.and.returnValue(
      Promise.resolve({ success: true, data: mockMetrics })
    );

    const result = await service.getRankedByHealth();

    expect(result).toBeDefined();
    expect(result.length).toBe(1);
  });

  it('should respect limit parameter in getRankedByHealth', async () => {
    const mockMetrics = Array.from({ length: 15 }, (_, i) => ({
      custodianId: `custodian-${i}`,
      tier: 1 as 1 | 2 | 3 | 4,
      health: {
        uptimePercent: 95 + i * 0.1,
        availability: true,
        responseTimeP50Ms: 50,
        responseTimeP95Ms: 150,
        responseTimeP99Ms: 300,
        errorRate: 0.001,
        slaCompliance: true,
      },
      storage: {
        totalCapacityBytes: 1_000_000_000,
        usedBytes: 600_000_000,
        freeBytes: 400_000_000,
        utilizationPercent: 60,
        byDomain: new Map(),
        fullReplicaBytes: 500_000_000,
        thresholdBytes: 800_000_000,
        erasureCodedBytes: 100_000_000,
      },
      bandwidth: {
        declaredMbps: 1000,
        currentUsageMbps: 400,
        peakUsageMbps: 750,
        averageUsageMbps: 450,
        utilizationPercent: 45,
        inboundMbps: 200,
        outboundMbps: 200,
        byDomain: new Map(),
      },
      computation: {
        cpuCores: 8,
        cpuUsagePercent: 35,
        memoryGb: 32,
        memoryUsagePercent: 55,
        zomeOpsPerSecond: 150,
        reconstructionWorkloadPercent: 10,
      },
      reputation: {
        reliabilityRating: 4.5,
        speedRating: 4.8,
        reputationScore: 92,
        specializationBonus: 0.05,
        commitmentFulfillment: 0.98,
      },
      economic: {
        stewardTier: 1 as 1 | 2 | 3 | 4,
        pricePerGb: 0.50,
        monthlyEarnings: 15000,
        lifetimeEarnings: 150000,
        activeCommitments: 5,
        totalCommittedBytes: 500_000_000,
      },
      collectedAt: Date.now(),
      lastUpdatedAt: Date.now(),
    }));

    clientMock.callZome.and.returnValue(
      Promise.resolve({ success: true, data: mockMetrics })
    );

    const result = await service.getRankedByHealth(5);

    expect(result.length).toBe(5);
  });

  it('should get custodians ranked by speed', async () => {
    const mockMetrics = [
      {
        custodianId: 'custodian-1',
        tier: 1 as 1 | 2 | 3 | 4,
        health: {
          uptimePercent: 99.5,
          availability: true,
          responseTimeP50Ms: 50,
          responseTimeP95Ms: 150,
          responseTimeP99Ms: 300,
          errorRate: 0.001,
          slaCompliance: true,
        },
        storage: {
          totalCapacityBytes: 1_000_000_000,
          usedBytes: 600_000_000,
          freeBytes: 400_000_000,
          utilizationPercent: 60,
          byDomain: new Map(),
          fullReplicaBytes: 500_000_000,
          thresholdBytes: 800_000_000,
          erasureCodedBytes: 100_000_000,
        },
        bandwidth: {
          declaredMbps: 1000,
          currentUsageMbps: 400,
          peakUsageMbps: 750,
          averageUsageMbps: 450,
          utilizationPercent: 45,
          inboundMbps: 200,
          outboundMbps: 200,
          byDomain: new Map(),
        },
        computation: {
          cpuCores: 8,
          cpuUsagePercent: 35,
          memoryGb: 32,
          memoryUsagePercent: 55,
          zomeOpsPerSecond: 150,
          reconstructionWorkloadPercent: 10,
        },
        reputation: {
          reliabilityRating: 4.5,
          speedRating: 4.8,
          reputationScore: 92,
          specializationBonus: 0.05,
          commitmentFulfillment: 0.98,
        },
        economic: {
          stewardTier: 1 as 1 | 2 | 3 | 4,
          pricePerGb: 0.50,
          monthlyEarnings: 15000,
          lifetimeEarnings: 150000,
          activeCommitments: 5,
          totalCommittedBytes: 500_000_000,
        },
        collectedAt: Date.now(),
        lastUpdatedAt: Date.now(),
      },
    ];

    clientMock.callZome.and.returnValue(
      Promise.resolve({ success: true, data: mockMetrics })
    );

    const result = await service.getRankedBySpeed();

    expect(result).toBeDefined();
    expect(result.length).toBe(1);
  });

  it('should get custodians ranked by reputation', async () => {
    const mockMetrics = [
      {
        custodianId: 'custodian-1',
        tier: 1 as 1 | 2 | 3 | 4,
        health: {
          uptimePercent: 99.5,
          availability: true,
          responseTimeP50Ms: 50,
          responseTimeP95Ms: 150,
          responseTimeP99Ms: 300,
          errorRate: 0.001,
          slaCompliance: true,
        },
        storage: {
          totalCapacityBytes: 1_000_000_000,
          usedBytes: 600_000_000,
          freeBytes: 400_000_000,
          utilizationPercent: 60,
          byDomain: new Map(),
          fullReplicaBytes: 500_000_000,
          thresholdBytes: 800_000_000,
          erasureCodedBytes: 100_000_000,
        },
        bandwidth: {
          declaredMbps: 1000,
          currentUsageMbps: 400,
          peakUsageMbps: 750,
          averageUsageMbps: 450,
          utilizationPercent: 45,
          inboundMbps: 200,
          outboundMbps: 200,
          byDomain: new Map(),
        },
        computation: {
          cpuCores: 8,
          cpuUsagePercent: 35,
          memoryGb: 32,
          memoryUsagePercent: 55,
          zomeOpsPerSecond: 150,
          reconstructionWorkloadPercent: 10,
        },
        reputation: {
          reliabilityRating: 4.5,
          speedRating: 4.8,
          reputationScore: 92,
          specializationBonus: 0.05,
          commitmentFulfillment: 0.98,
        },
        economic: {
          stewardTier: 1 as 1 | 2 | 3 | 4,
          pricePerGb: 0.50,
          monthlyEarnings: 15000,
          lifetimeEarnings: 150000,
          activeCommitments: 5,
          totalCommittedBytes: 500_000_000,
        },
        collectedAt: Date.now(),
        lastUpdatedAt: Date.now(),
      },
    ];

    clientMock.callZome.and.returnValue(
      Promise.resolve({ success: true, data: mockMetrics })
    );

    const result = await service.getRankedByReputation();

    expect(result).toBeDefined();
    expect(result.length).toBe(1);
  });

  // ==========================================================================
  // Availability Check
  // ==========================================================================

  it('should get available custodians', async () => {
    const mockMetrics = [
      {
        custodianId: 'custodian-1',
        tier: 1 as 1 | 2 | 3 | 4,
        health: {
          uptimePercent: 99.5,
          availability: true,
          responseTimeP50Ms: 50,
          responseTimeP95Ms: 150,
          responseTimeP99Ms: 300,
          errorRate: 0.001,
          slaCompliance: true,
        },
        storage: {
          totalCapacityBytes: 1_000_000_000,
          usedBytes: 600_000_000,
          freeBytes: 400_000_000,
          utilizationPercent: 60,
          byDomain: new Map(),
          fullReplicaBytes: 500_000_000,
          thresholdBytes: 800_000_000,
          erasureCodedBytes: 100_000_000,
        },
        bandwidth: {
          declaredMbps: 1000,
          currentUsageMbps: 400,
          peakUsageMbps: 750,
          averageUsageMbps: 450,
          utilizationPercent: 45,
          inboundMbps: 200,
          outboundMbps: 200,
          byDomain: new Map(),
        },
        computation: {
          cpuCores: 8,
          cpuUsagePercent: 35,
          memoryGb: 32,
          memoryUsagePercent: 55,
          zomeOpsPerSecond: 150,
          reconstructionWorkloadPercent: 10,
        },
        reputation: {
          reliabilityRating: 4.5,
          speedRating: 4.8,
          reputationScore: 92,
          specializationBonus: 0.05,
          commitmentFulfillment: 0.98,
        },
        economic: {
          stewardTier: 1 as 1 | 2 | 3 | 4,
          pricePerGb: 0.50,
          monthlyEarnings: 15000,
          lifetimeEarnings: 150000,
          activeCommitments: 5,
          totalCommittedBytes: 500_000_000,
        },
        collectedAt: Date.now(),
        lastUpdatedAt: Date.now(),
      },
    ];

    clientMock.callZome.and.returnValue(
      Promise.resolve({ success: true, data: mockMetrics })
    );

    const result = await service.getAvailableCustodians();

    expect(result).toBeDefined();
    expect(result.length).toBeGreaterThanOrEqual(0);
  });

  // ==========================================================================
  // Alerts
  // ==========================================================================

  it('should get alerts for custodians', async () => {
    const mockMetrics = [
      {
        custodianId: 'custodian-unhealthy',
        tier: 1 as 1 | 2 | 3 | 4,
        health: {
          uptimePercent: 90,
          availability: true,
          responseTimeP50Ms: 50,
          responseTimeP95Ms: 2000,
          responseTimeP99Ms: 3000,
          errorRate: 0.1,
          slaCompliance: false,
        },
        storage: {
          totalCapacityBytes: 1_000_000_000,
          usedBytes: 950_000_000,
          freeBytes: 50_000_000,
          utilizationPercent: 95,
          byDomain: new Map(),
          fullReplicaBytes: 900_000_000,
          thresholdBytes: 800_000_000,
          erasureCodedBytes: 50_000_000,
        },
        bandwidth: {
          declaredMbps: 1000,
          currentUsageMbps: 400,
          peakUsageMbps: 750,
          averageUsageMbps: 450,
          utilizationPercent: 45,
          inboundMbps: 200,
          outboundMbps: 200,
          byDomain: new Map(),
        },
        computation: {
          cpuCores: 8,
          cpuUsagePercent: 85,
          memoryGb: 32,
          memoryUsagePercent: 90,
          zomeOpsPerSecond: 150,
          reconstructionWorkloadPercent: 10,
        },
        reputation: {
          reliabilityRating: 2.0,
          speedRating: 1.5,
          reputationScore: 45,
          specializationBonus: 0.0,
          commitmentFulfillment: 0.5,
        },
        economic: {
          stewardTier: 1 as 1 | 2 | 3 | 4,
          pricePerGb: 0.50,
          monthlyEarnings: 5000,
          lifetimeEarnings: 50000,
          activeCommitments: 2,
          totalCommittedBytes: 100_000_000,
        },
        collectedAt: Date.now(),
        lastUpdatedAt: Date.now(),
      },
    ];

    clientMock.callZome.and.returnValue(
      Promise.resolve({ success: true, data: mockMetrics })
    );

    const result = await service.getAlerts();

    expect(result).toBeDefined();
    expect(Array.isArray(result)).toBe(true);
  });

  // ==========================================================================
  // Recommendations
  // ==========================================================================

  it('should get recommendations for custodians', async () => {
    const mockMetrics = [
      {
        custodianId: 'custodian-1',
        tier: 1 as 1 | 2 | 3 | 4,
        health: {
          uptimePercent: 99.5,
          availability: true,
          responseTimeP50Ms: 50,
          responseTimeP95Ms: 150,
          responseTimeP99Ms: 300,
          errorRate: 0.001,
          slaCompliance: true,
        },
        storage: {
          totalCapacityBytes: 1_000_000_000,
          usedBytes: 600_000_000,
          freeBytes: 400_000_000,
          utilizationPercent: 60,
          byDomain: new Map(),
          fullReplicaBytes: 500_000_000,
          thresholdBytes: 800_000_000,
          erasureCodedBytes: 100_000_000,
        },
        bandwidth: {
          declaredMbps: 1000,
          currentUsageMbps: 200,
          peakUsageMbps: 400,
          averageUsageMbps: 250,
          utilizationPercent: 25,
          inboundMbps: 100,
          outboundMbps: 100,
          byDomain: new Map(),
        },
        computation: {
          cpuCores: 8,
          cpuUsagePercent: 25,
          memoryGb: 32,
          memoryUsagePercent: 40,
          zomeOpsPerSecond: 150,
          reconstructionWorkloadPercent: 10,
        },
        reputation: {
          reliabilityRating: 4.8,
          speedRating: 4.9,
          reputationScore: 95,
          specializationBonus: 0.02,
          commitmentFulfillment: 0.99,
        },
        economic: {
          stewardTier: 2,
          pricePerGb: 0.50,
          monthlyEarnings: 15000,
          lifetimeEarnings: 150000,
          activeCommitments: 5,
          totalCommittedBytes: 500_000_000,
        },
        collectedAt: Date.now(),
        lastUpdatedAt: Date.now(),
      },
    ];

    clientMock.callZome.and.returnValue(
      Promise.resolve({ success: true, data: mockMetrics })
    );

    const result = await service.getRecommendations();

    expect(result).toBeDefined();
    expect(Array.isArray(result)).toBe(true);
  });

  // ==========================================================================
  // Cache Management
  // ==========================================================================

  it('should clear cache when clearCache is called', async () => {
    const mockMetrics = {
      custodianId: 'custodian-1',
      tier: 1 as 1 | 2 | 3 | 4,
      health: {
        uptimePercent: 99.5,
        availability: true,
        responseTimeP50Ms: 50,
        responseTimeP95Ms: 150,
        responseTimeP99Ms: 300,
        errorRate: 0.001,
        slaCompliance: true,
      },
      storage: {
        totalCapacityBytes: 1_000_000_000,
        usedBytes: 600_000_000,
        freeBytes: 400_000_000,
        utilizationPercent: 60,
        byDomain: new Map(),
        fullReplicaBytes: 500_000_000,
        thresholdBytes: 800_000_000,
        erasureCodedBytes: 100_000_000,
      },
      bandwidth: {
        declaredMbps: 1000,
        currentUsageMbps: 400,
        peakUsageMbps: 750,
        averageUsageMbps: 450,
        utilizationPercent: 45,
        inboundMbps: 200,
        outboundMbps: 200,
        byDomain: new Map(),
      },
      computation: {
        cpuCores: 8,
        cpuUsagePercent: 35,
        memoryGb: 32,
        memoryUsagePercent: 55,
        zomeOpsPerSecond: 150,
        reconstructionWorkloadPercent: 10,
      },
      reputation: {
        reliabilityRating: 4.5,
        speedRating: 4.8,
        reputationScore: 92,
        specializationBonus: 0.05,
        commitmentFulfillment: 0.98,
      },
      economic: {
        stewardTier: 1 as 1 | 2 | 3 | 4,
        pricePerGb: 0.50,
        monthlyEarnings: 15000,
        lifetimeEarnings: 150000,
        activeCommitments: 5,
        totalCommittedBytes: 500_000_000,
      },
      collectedAt: Date.now(),
      lastUpdatedAt: Date.now(),
    };

    clientMock.callZome.and.returnValue(Promise.resolve({ success: true, data: mockMetrics }));

    await service.getMetrics('custodian-1');
    service.clearCache();
    await service.getMetrics('custodian-1');

    expect(clientMock.callZome).toHaveBeenCalledTimes(2);
  });
});
