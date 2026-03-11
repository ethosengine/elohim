import { TestBed } from '@angular/core/testing';
import {
  HttpTestingController,
  provideHttpClientTesting,
} from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';

import { DataProtectionApiService } from './data-protection-api.service';
import type { FamilyCommunityProtectionStatus } from '../interfaces/data-protection.interface';

describe('DataProtectionApiService', () => {
  let service: DataProtectionApiService;
  let httpMock: HttpTestingController;

  const stubStatus: FamilyCommunityProtectionStatus = {
    redundancy: {
      strategy: 'full_replica',
      redundancyFactor: 3,
      recoveryThreshold: 2,
    },
    custodians: [
      {
        id: 'cust-1',
        name: 'Uncle Bob',
        type: 'family',
        location: { region: 'NA', country: 'US' },
        dataStored: { totalGB: 50, shardCount: 10, redundancyLevel: 3 },
        health: { upPercent: 99, lastHeartbeat: '2026-03-08T00:00:00Z', responseTime: 20 },
        commitment: {
          id: 'comm-1',
          status: 'active',
          startDate: '2026-01-01',
          expiryDate: '2027-01-01',
          renewalStatus: 'auto-renew',
        },
        trustLevel: 0.95,
        relationship: 'uncle',
      },
      {
        id: 'cust-2',
        name: 'Community Node',
        type: 'community',
        location: { region: 'EU', country: 'DE' },
        dataStored: { totalGB: 30, shardCount: 8, redundancyLevel: 2 },
        health: { upPercent: 80, lastHeartbeat: '2026-03-08T00:00:00Z', responseTime: 50 },
        commitment: {
          id: 'comm-2',
          status: 'active',
          startDate: '2026-01-01',
          expiryDate: '2027-01-01',
          renewalStatus: 'manual',
        },
        trustLevel: 0.7,
        relationship: 'community-peer',
      },
    ],
    totalCustodians: 2,
    geographicDistribution: {
      regions: [
        { region: 'NA', custodianCount: 1, dataShards: 10, redundancy: 3, riskFactors: [] },
        {
          region: 'EU',
          custodianCount: 1,
          dataShards: 8,
          redundancy: 2,
          riskFactors: ['single-custodian'],
        },
      ],
      riskProfile: 'distributed',
    },
    trustGraph: [],
    protectionLevel: 'protected',
    estimatedRecoveryTime: '2h',
    lastVerification: '2026-03-08T00:00:00Z',
    verificationStatus: 'verified',
  };

  /** Fetch and flush in one call, returning the result. */
  async function fetchAndFlush(
    operatorId = 'steward-1',
    data: FamilyCommunityProtectionStatus = stubStatus
  ): Promise<FamilyCommunityProtectionStatus> {
    const promise = service.fetchProtectionStatus(operatorId);
    httpMock
      .expectOne(`/api/v1/custodians/protection/${operatorId}/summary`)
      .flush(data);
    return promise;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [
        DataProtectionApiService,
        provideHttpClient(),
        provideHttpClientTesting(),
      ],
    });
    service = TestBed.inject(DataProtectionApiService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => httpMock.verify());

  it('fetchProtectionStatus calls GET summary endpoint', async () => {
    const promise = service.fetchProtectionStatus('steward-1');
    const req = httpMock.expectOne('/api/v1/custodians/protection/steward-1/summary');
    expect(req.request.method).toBe('GET');
    req.flush(stubStatus);
    expect(await promise).toEqual(stubStatus);
  });

  it('getProtectionStatus returns null before initialization', () => {
    expect(service.getProtectionStatus()).toBeNull();
  });

  it('getProtectionStatus returns cached status after fetch', async () => {
    await fetchAndFlush();
    expect(service.getProtectionStatus()).toEqual(stubStatus);
  });

  it('getProtectionStatus$ emits null then status after fetch', async () => {
    const emissions: (FamilyCommunityProtectionStatus | null)[] = [];
    const sub = service.getProtectionStatus$().subscribe((s) => emissions.push(s));

    expect(emissions).toEqual([null]);

    await fetchAndFlush();
    expect(emissions.length).toBe(2);
    expect(emissions[1]).toEqual(stubStatus);

    sub.unsubscribe();
  });

  it('getCustodiansByType filters by type', async () => {
    await fetchAndFlush();

    const family = service.getCustodiansByType('family');
    expect(family.length).toBe(1);
    expect(family[0].id).toBe('cust-1');

    const community = service.getCustodiansByType('community');
    expect(community.length).toBe(1);
    expect(community[0].id).toBe('cust-2');

    expect(service.getCustodiansByType('professional')).toEqual([]);
  });

  it('getCustodiansByType returns empty before initialization', () => {
    expect(service.getCustodiansByType('family')).toEqual([]);
  });

  it('getHighRiskRegions returns regions with risk factors or single custodian', async () => {
    await fetchAndFlush();

    const risky = service.getHighRiskRegions();
    // Both regions have custodianCount <= 1
    expect(risky.length).toBe(2);
    expect(risky.some((r) => r.region === 'EU')).toBe(true);
    expect(risky.some((r) => r.region === 'NA')).toBe(true);
  });

  it('isCustodianHealthy checks uptime >= 95%', async () => {
    await fetchAndFlush();

    expect(service.isCustodianHealthy('cust-1')).toBe(true);
    expect(service.isCustodianHealthy('cust-2')).toBe(false);
    expect(service.isCustodianHealthy('unknown')).toBe(false);
  });

  it('getAverageUptime computes mean', async () => {
    await fetchAndFlush();
    expect(service.getAverageUptime()).toBeCloseTo(89.5);
  });

  it('getAverageUptime returns 0 before initialization', () => {
    expect(service.getAverageUptime()).toBe(0);
  });

  it('getProtectionAlerts returns alerts for unhealthy custodians and risky regions', async () => {
    await fetchAndFlush();

    const alerts = service.getProtectionAlerts();
    // Unhealthy custodian (cust-2 at 80%) + both regions have custodianCount <= 1
    expect(alerts.length).toBe(3);
    expect(alerts[0]).toContain('Community Node');
    expect(alerts.some((a) => a.includes('NA'))).toBe(true);
    expect(alerts.some((a) => a.includes('EU'))).toBe(true);
  });

  it('getProtectionAlerts includes vulnerability alert', async () => {
    const vulnerable = { ...stubStatus, protectionLevel: 'vulnerable' as const };
    await fetchAndFlush('steward-1', vulnerable);

    const alerts = service.getProtectionAlerts();
    expect(alerts[0]).toContain('vulnerable');
  });
});
