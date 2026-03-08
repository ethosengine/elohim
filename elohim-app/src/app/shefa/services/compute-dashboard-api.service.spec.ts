import { TestBed } from '@angular/core/testing';
import {
  HttpTestingController,
  provideHttpClientTesting,
} from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';

import { ComputeDashboardApiService } from './compute-dashboard-api.service';
import type { SheafaDashboardState } from '../models/shefa-dashboard.model';

describe('ComputeDashboardApiService', () => {
  let service: ComputeDashboardApiService;
  let httpMock: HttpTestingController;

  const stubDashboard: SheafaDashboardState = {
    operatorId: 'agent-1',
    operatorName: 'Test Operator',
    stewardedResourceId: 'sr-1',
    nodeId: 'node-1',
    status: 'online',
    lastHeartbeat: '2026-03-08T00:00:00Z',
    uptime: {
      upPercent: 99.9,
      downtime: { hours24: 0.1, hours7d: 0.5, hours30d: 2 },
      consecutiveUptime: '14 days',
      reliability: 'excellent',
    },
    computeMetrics: {
      cpu: {
        totalCores: 8,
        available: 6,
        usagePercent: 25,
        usageHistory: [],
      },
      memory: {
        totalGB: 32,
        usedGB: 8,
        availableGB: 24,
        usagePercent: 25,
        usageHistory: [],
      },
      storage: {
        totalGB: 1000,
        usedGB: 200,
        availableGB: 800,
        usagePercent: 20,
        usageHistory: [],
        breakdown: {
          holochain: 50,
          cache: 30,
          custodianData: 100,
          userApplications: 20,
        },
      },
      network: {
        bandwidth: {
          upstreamMbps: 100,
          downstreamMbps: 500,
          usedUpstreamMbps: 10,
          usedDownstreamMbps: 50,
        },
        latency: { p50: 5, p95: 20, p99: 50 },
        connections: { total: 50, holochain: 30, cache: 10, custodian: 10 },
      },
      loadAverage: { oneMinute: 1.5, fiveMinutes: 1.2, fifteenMinutes: 1.0 },
    },
    allocations: {
      byGovernanceLevel: {
        individual: { cpuPercent: 25, memoryPercent: 25, storagePercent: 25, bandwidthPercent: 25 },
        household: { cpuPercent: 25, memoryPercent: 25, storagePercent: 25, bandwidthPercent: 25 },
        community: { cpuPercent: 25, memoryPercent: 25, storagePercent: 25, bandwidthPercent: 25 },
        network: { cpuPercent: 25, memoryPercent: 25, storagePercent: 25, bandwidthPercent: 25 },
      },
      totalAllocated: { cpuPercent: 100, memoryPercent: 100, storagePercent: 100, bandwidthPercent: 100 },
      allocationBlocks: [],
    },
    familyCommunityProtection: {
      redundancy: { strategy: 'full_replica', redundancyFactor: 3, recoveryThreshold: 2 },
      custodians: [],
      totalCustodians: 3,
      geographicDistribution: { regions: [], riskProfile: 'distributed' },
      trustGraph: [],
      protectionLevel: 'protected',
      estimatedRecoveryTime: '< 1 hour',
      lastVerification: '2026-03-08T00:00:00Z',
      verificationStatus: 'verified',
    },
    infrastructureTokens: {
      balance: { tokens: 100, estimatedValue: { value: 10, currency: 'USD' } },
      earningRate: {
        tokensPerHour: 1,
        basedOn: { cpuAllocation: 25, storageAllocation: 20, bandwidthAllocation: 10 },
        estimatedMonthly: 720,
      },
      decay: {
        demurrageRate: 0.5,
        lastCalculated: '2026-03-08T00:00:00Z',
        projectedNextMonth: { tokens: 96, valueUSD: 9.6 },
      },
      transactions: [],
      tokenHistory: { last24Hours: 24, last7Days: 168, last30Days: 720, allTime: 5000 },
      exchangeRates: [],
    },
    economicEvents: [],
    constitutionalLimits: {
      dignityFloor: {
        computeMinCores: 2,
        computeMinMemoryGB: 4,
        computeMinStorageGB: 100,
        computeMinBandwidthMbps: 10,
        status: 'met',
        percentOfFloor: 300,
        enforcement: 'progressive',
      },
      ceilingLimit: {
        computeMaxCores: 64,
        computeMaxMemoryGB: 256,
        computeMaxStorageGB: 10000,
        computeMaxBandwidthMbps: 10000,
        tokenAccumulationCeiling: 100000,
        currentAccumulation: 100,
        percentOfCeiling: 0.1,
        status: 'safe',
        enforcement: 'voluntary',
      },
      safeZone: { cpu: 50, memory: 50, storage: 50, bandwidth: 50, tokens: 50 },
      alerts: [],
    },
    lastUpdated: '2026-03-08T00:00:00Z',
    updateFrequency: 5000,
  };

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [
        ComputeDashboardApiService,
        provideHttpClient(),
        provideHttpClientTesting(),
      ],
    });
    service = TestBed.inject(ComputeDashboardApiService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => httpMock.verify());

  it('getDashboard calls GET /api/v1/compute/dashboard', async () => {
    const promise = service.getDashboard();
    const req = httpMock.expectOne('/api/v1/compute/dashboard');
    expect(req.request.method).toBe('GET');
    req.flush(stubDashboard);
    expect(await promise).toEqual(stubDashboard);
  });

  it('getDashboardForGovernanceLevel calls GET with level param', async () => {
    const promise = service.getDashboardForGovernanceLevel('household');
    const req = httpMock.expectOne(
      (r) => r.url === '/api/v1/compute/dashboard' && r.params.get('level') === 'household'
    );
    expect(req.request.method).toBe('GET');
    req.flush(stubDashboard);
    expect(await promise).toEqual(stubDashboard);
  });

  it('refreshDashboard calls POST /api/v1/compute/dashboard/refresh', async () => {
    const promise = service.refreshDashboard();
    const req = httpMock.expectOne('/api/v1/compute/dashboard/refresh');
    expect(req.request.method).toBe('POST');
    req.flush(stubDashboard);
    expect(await promise).toEqual(stubDashboard);
  });
});
