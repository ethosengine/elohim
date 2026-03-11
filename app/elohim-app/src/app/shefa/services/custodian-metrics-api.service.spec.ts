import { TestBed } from '@angular/core/testing';
import {
  HttpTestingController,
  provideHttpClientTesting,
} from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';

import { CustodianMetricsApiService } from './custodian-metrics-api.service';
import type {
  CustodianAlert,
  CustodianMetrics,
  CustodianRecommendation,
} from '../interfaces/custodian-metrics.interface';

describe('CustodianMetricsApiService', () => {
  let service: CustodianMetricsApiService;
  let httpMock: HttpTestingController;

  const stubMetrics: CustodianMetrics = {
    custodianId: 'cust-1',
    tier: 2,
    health: {
      uptimePercent: 99.5,
      availability: true,
      responseTimeP50Ms: 10,
      responseTimeP95Ms: 50,
      responseTimeP99Ms: 200,
      errorRate: 0.01,
      slaCompliance: true,
    },
    storage: {
      totalCapacityBytes: 1e12,
      usedBytes: 5e11,
      freeBytes: 5e11,
      utilizationPercent: 50,
      byDomain: new Map(),
      fullReplicaBytes: 3e11,
      thresholdBytes: 1e11,
      erasureCodedBytes: 1e11,
    },
    bandwidth: {
      declaredMbps: 1000,
      currentUsageMbps: 200,
      peakUsageMbps: 800,
      averageUsageMbps: 300,
      utilizationPercent: 20,
      inboundMbps: 100,
      outboundMbps: 100,
      byDomain: new Map(),
    },
    computation: {
      cpuCores: 8,
      cpuUsagePercent: 30,
      memoryGb: 32,
      memoryUsagePercent: 45,
      zomeOpsPerSecond: 500,
      reconstructionWorkloadPercent: 5,
    },
    reputation: {
      reliabilityRating: 0.95,
      speedRating: 0.9,
      reputationScore: 0.92,
      specializationBonus: 0.1,
      commitmentFulfillment: 0.98,
    },
    economic: {
      stewardTier: 2,
      pricePerGb: 0.05,
      monthlyEarnings: 150,
      lifetimeEarnings: 3000,
      activeCommitments: 12,
      totalCommittedBytes: 4e11,
    },
    collectedAt: Date.now(),
    lastUpdatedAt: Date.now(),
  };

  const stubAlert: CustodianAlert = {
    custodianId: 'cust-1',
    severity: 'warning',
    category: 'storage',
    message: 'Storage utilization above 80%',
  };

  const stubRecommendation: CustodianRecommendation = {
    custodianId: 'cust-1',
    category: 'bandwidth',
    opportunity: 'Upgrade bandwidth to serve more stewards',
    potential_revenue: 50,
  };

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [
        CustodianMetricsApiService,
        provideHttpClient(),
        provideHttpClientTesting(),
      ],
    });
    service = TestBed.inject(CustodianMetricsApiService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => httpMock.verify());

  it('getMetrics calls GET with custodian id', async () => {
    const promise = service.getMetrics('cust-1');
    const req = httpMock.expectOne('/api/v1/custodians/metrics/cust-1');
    expect(req.request.method).toBe('GET');
    req.flush(stubMetrics);
    expect(await promise).toEqual(stubMetrics);
  });

  it('getAllMetrics calls GET /api/v1/custodians/metrics', async () => {
    const promise = service.getAllMetrics();
    const req = httpMock.expectOne('/api/v1/custodians/metrics');
    expect(req.request.method).toBe('GET');
    req.flush([stubMetrics]);
    expect(await promise).toEqual([stubMetrics]);
  });

  it('reportMetrics calls POST with metrics payload', async () => {
    const promise = service.reportMetrics(stubMetrics);
    const req = httpMock.expectOne('/api/v1/custodians/metrics');
    expect(req.request.method).toBe('POST');
    expect(req.request.body).toEqual(stubMetrics);
    req.flush({ success: true });
    expect(await promise).toEqual({ success: true });
  });

  it('getRankedByHealth calls GET with rank=health param', async () => {
    const promise = service.getRankedByHealth();
    const req = httpMock.expectOne(
      (r) => r.url === '/api/v1/custodians/metrics' && r.params.get('rank') === 'health'
    );
    expect(req.request.method).toBe('GET');
    req.flush([stubMetrics]);
    expect(await promise).toEqual([stubMetrics]);
  });

  it('getRankedByHealth passes limit param', async () => {
    const promise = service.getRankedByHealth(5);
    const req = httpMock.expectOne(
      (r) =>
        r.url === '/api/v1/custodians/metrics' &&
        r.params.get('rank') === 'health' &&
        r.params.get('limit') === '5'
    );
    expect(req.request.method).toBe('GET');
    req.flush([stubMetrics]);
    expect(await promise).toEqual([stubMetrics]);
  });

  it('getRankedBySpeed calls GET with rank=speed param', async () => {
    const promise = service.getRankedBySpeed();
    const req = httpMock.expectOne(
      (r) => r.url === '/api/v1/custodians/metrics' && r.params.get('rank') === 'speed'
    );
    expect(req.request.method).toBe('GET');
    req.flush([stubMetrics]);
    expect(await promise).toEqual([stubMetrics]);
  });

  it('getRankedByReputation calls GET with rank=reputation param', async () => {
    const promise = service.getRankedByReputation();
    const req = httpMock.expectOne(
      (r) =>
        r.url === '/api/v1/custodians/metrics' && r.params.get('rank') === 'reputation'
    );
    expect(req.request.method).toBe('GET');
    req.flush([stubMetrics]);
    expect(await promise).toEqual([stubMetrics]);
  });

  it('getAvailableCustodians calls GET with available=true param', async () => {
    const promise = service.getAvailableCustodians();
    const req = httpMock.expectOne(
      (r) =>
        r.url === '/api/v1/custodians/metrics' && r.params.get('available') === 'true'
    );
    expect(req.request.method).toBe('GET');
    req.flush([stubMetrics]);
    expect(await promise).toEqual([stubMetrics]);
  });

  it('getAlerts calls GET /api/v1/custodians/metrics/alerts', async () => {
    const promise = service.getAlerts();
    const req = httpMock.expectOne('/api/v1/custodians/metrics/alerts');
    expect(req.request.method).toBe('GET');
    req.flush([stubAlert]);
    expect(await promise).toEqual([stubAlert]);
  });

  it('getRecommendations calls GET /api/v1/custodians/metrics/recommendations', async () => {
    const promise = service.getRecommendations();
    const req = httpMock.expectOne('/api/v1/custodians/metrics/recommendations');
    expect(req.request.method).toBe('GET');
    req.flush([stubRecommendation]);
    expect(await promise).toEqual([stubRecommendation]);
  });

  it('clearCache is a no-op', () => {
    expect(() => service.clearCache()).not.toThrow();
  });

  it('stopPeriodicReporting is a no-op', () => {
    expect(() => service.stopPeriodicReporting()).not.toThrow();
  });
});
