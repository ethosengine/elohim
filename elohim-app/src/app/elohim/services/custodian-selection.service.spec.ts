import { TestBed } from '@angular/core/testing';

import { CustodianSelectionService } from './custodian-selection.service';
import { ShefaService } from './shefa.service';
import { CustodianCommitmentService } from './custodian-commitment.service';

describe('CustodianSelectionService', () => {
  let service: CustodianSelectionService;
  let shefaMock: jasmine.SpyObj<ShefaService>;
  let commitmentsMock: jasmine.SpyObj<CustodianCommitmentService>;

  beforeEach(() => {
    const shefaSpy = jasmine.createSpyObj('ShefaService', ['getMetrics', 'getAllMetrics']);
    const commitmentsSpy = jasmine.createSpyObj('CustodianCommitmentService', [
      'getCommitmentsForContent',
    ]);

    TestBed.configureTestingModule({
      providers: [
        CustodianSelectionService,
        { provide: ShefaService, useValue: shefaSpy },
        { provide: CustodianCommitmentService, useValue: commitmentsSpy },
      ],
    });

    service = TestBed.inject(CustodianSelectionService);
    shefaMock = TestBed.inject(ShefaService) as jasmine.SpyObj<ShefaService>;
    commitmentsMock = TestBed.inject(
      CustodianCommitmentService
    ) as jasmine.SpyObj<CustodianCommitmentService>;
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('selectBestCustodian', () => {
    it('should have selectBestCustodian method', () => {
      expect(service.selectBestCustodian).toBeDefined();
      expect(typeof service.selectBestCustodian).toBe('function');
    });

    it('should return null when no custodians committed to content', async () => {
      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve([]));

      const result = await service.selectBestCustodian('content-1');

      expect(result).toBeNull();
    });

    it('should return highest scoring custodian', async () => {
      const mockCommitments = [
        {
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://cust1.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 3,
        },
        {
          custodianId: 'cust-2',
          doorwayEndpoint: 'http://cust2.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 2,
        },
      ];

      const mockMetrics1 = {
        custodianId: 'cust-1',
        health: { uptimePercent: 95, availability: true, responseTimeP95Ms: 100, slaCompliance: true },
        bandwidth: { currentUsageMbps: 50, declaredMbps: 100 },
        reputation: { specializationBonus: 0.05 },
        economic: { stewardTier: 3 },
      };

      const mockMetrics2 = {
        custodianId: 'cust-2',
        health: { uptimePercent: 90, availability: true, responseTimeP95Ms: 150, slaCompliance: false },
        bandwidth: { currentUsageMbps: 60, declaredMbps: 100 },
        reputation: { specializationBonus: 0.02 },
        economic: { stewardTier: 2 },
      };

      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve(mockCommitments as any));
      shefaMock.getMetrics.and.callFake((custId: string) => {
        if (custId === 'cust-1') return Promise.resolve(mockMetrics1 as any);
        return Promise.resolve(mockMetrics2 as any);
      });

      const result = await service.selectBestCustodian('content-1');

      expect(result).toBeTruthy();
      expect(result?.custodian.id).toBe('cust-1');
      expect(result?.finalScore).toBeGreaterThan(0);
    });

    it('should return null when all custodians unhealthy', async () => {
      const mockCommitments = [
        {
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://cust1.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 3,
        },
      ];

      const mockMetrics = {
        custodianId: 'cust-1',
        health: { uptimePercent: 40, availability: false, responseTimeP95Ms: 100, slaCompliance: false },
        bandwidth: { currentUsageMbps: 50, declaredMbps: 100 },
        reputation: { specializationBonus: 0 },
        economic: { stewardTier: 3 },
      };

      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve(mockCommitments as any));
      shefaMock.getMetrics.and.returnValue(Promise.resolve(mockMetrics as any));

      const result = await service.selectBestCustodian('content-1');

      expect(result).toBeNull();
    });

    it('should increment selections attempted counter', async () => {
      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve([]));

      const statsBefore = service.getStatistics();
      await service.selectBestCustodian('content-1');
      const statsAfter = service.getStatistics();

      expect(statsAfter.selectionsAttempted).toBe(statsBefore.selectionsAttempted + 1);
    });

    it('should cache results with 2 minute TTL', async () => {
      const mockCommitments = [
        {
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://cust1.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 3,
        },
      ];

      const mockMetrics = {
        custodianId: 'cust-1',
        health: { uptimePercent: 95, availability: true, responseTimeP95Ms: 100, slaCompliance: true },
        bandwidth: { currentUsageMbps: 50, declaredMbps: 100 },
        reputation: { specializationBonus: 0.05 },
        economic: { stewardTier: 3 },
      };

      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve(mockCommitments as any));
      shefaMock.getMetrics.and.returnValue(Promise.resolve(mockMetrics as any));

      const result1 = await service.selectBestCustodian('content-1');
      const callCountAfterFirst = shefaMock.getMetrics.calls.count();

      const result2 = await service.selectBestCustodian('content-1');
      const callCountAfterSecond = shefaMock.getMetrics.calls.count();

      expect(result1).toEqual(result2);
      expect(callCountAfterSecond).toBe(callCountAfterFirst);
    });
  });

  describe('scoreAllCustodians', () => {
    it('should have scoreAllCustodians method', () => {
      expect(service.scoreAllCustodians).toBeDefined();
      expect(typeof service.scoreAllCustodians).toBe('function');
    });

    it('should score all custodians and return sorted array', async () => {
      const mockMetrics = [
        {
          custodianId: 'cust-1',
          health: { uptimePercent: 95, availability: true, responseTimeP95Ms: 100, slaCompliance: true },
          bandwidth: { currentUsageMbps: 50, declaredMbps: 100 },
          reputation: { specializationBonus: 0.05 },
          economic: { stewardTier: 3 },
        },
        {
          custodianId: 'cust-2',
          health: { uptimePercent: 85, availability: true, responseTimeP95Ms: 150, slaCompliance: false },
          bandwidth: { currentUsageMbps: 60, declaredMbps: 100 },
          reputation: { specializationBonus: 0.02 },
          economic: { stewardTier: 2 },
        },
      ];

      shefaMock.getAllMetrics.and.returnValue(Promise.resolve(mockMetrics as any));

      const result = await service.scoreAllCustodians();

      expect(Array.isArray(result)).toBe(true);
      if (result.length > 1) {
        expect(result[0].finalScore).toBeGreaterThanOrEqual(result[1].finalScore);
      }
    });

    it('should return empty array on error', async () => {
      shefaMock.getAllMetrics.and.returnValue(Promise.reject(new Error('Service error')));

      const result = await service.scoreAllCustodians();

      expect(result).toEqual([]);
    });
  });

  describe('getTopCustodians', () => {
    it('should have getTopCustodians method', () => {
      expect(service.getTopCustodians).toBeDefined();
      expect(typeof service.getTopCustodians).toBe('function');
    });

    it('should return top N custodians', async () => {
      const mockMetrics = [
        {
          custodianId: 'cust-1',
          health: { uptimePercent: 99, availability: true, responseTimeP95Ms: 50, slaCompliance: true },
          bandwidth: { currentUsageMbps: 30, declaredMbps: 100 },
          reputation: { specializationBonus: 0.1 },
          economic: { stewardTier: 4 },
        },
        {
          custodianId: 'cust-2',
          health: { uptimePercent: 95, availability: true, responseTimeP95Ms: 100, slaCompliance: true },
          bandwidth: { currentUsageMbps: 50, declaredMbps: 100 },
          reputation: { specializationBonus: 0.05 },
          economic: { stewardTier: 3 },
        },
        {
          custodianId: 'cust-3',
          health: { uptimePercent: 90, availability: true, responseTimeP95Ms: 150, slaCompliance: false },
          bandwidth: { currentUsageMbps: 70, declaredMbps: 100 },
          reputation: { specializationBonus: 0.02 },
          economic: { stewardTier: 2 },
        },
      ];

      shefaMock.getAllMetrics.and.returnValue(Promise.resolve(mockMetrics as any));

      const result = await service.getTopCustodians(2);

      expect(result.length).toBe(2);
      expect(result[0].custodian.id).toBe('cust-1');
    });

    it('should accept default limit of 10', async () => {
      shefaMock.getAllMetrics.and.returnValue(Promise.resolve([] as any));

      const result = await service.getTopCustodians();

      expect(Array.isArray(result)).toBe(true);
    });
  });

  describe('clearCache', () => {
    it('should have clearCache method', () => {
      expect(service.clearCache).toBeDefined();
      expect(typeof service.clearCache).toBe('function');
    });

    it('should clear cache and force recomputation', async () => {
      const mockCommitments = [
        {
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://cust1.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 3,
        },
      ];

      const mockMetrics = {
        custodianId: 'cust-1',
        health: { uptimePercent: 95, availability: true, responseTimeP95Ms: 100, slaCompliance: true },
        bandwidth: { currentUsageMbps: 50, declaredMbps: 100 },
        reputation: { specializationBonus: 0.05 },
        economic: { stewardTier: 3 },
      };

      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve(mockCommitments as any));
      shefaMock.getMetrics.and.returnValue(Promise.resolve(mockMetrics as any));

      await service.selectBestCustodian('content-1');
      const callCountBefore = commitmentsMock.getCommitmentsForContent.calls.count();

      service.clearCache();

      await service.selectBestCustodian('content-1');
      const callCountAfter = commitmentsMock.getCommitmentsForContent.calls.count();

      expect(callCountAfter).toBeGreaterThan(callCountBefore);
    });
  });

  describe('getStatistics', () => {
    it('should have getStatistics method', () => {
      expect(service.getStatistics).toBeDefined();
      expect(typeof service.getStatistics).toBe('function');
    });

    it('should return statistics object', () => {
      const stats = service.getStatistics();

      expect(stats).toEqual(
        jasmine.objectContaining({
          selectionsAttempted: jasmine.any(Number),
          selectionsSuccessful: jasmine.any(Number),
          cacheHits: jasmine.any(Number),
          cacheMisses: jasmine.any(Number),
        })
      );
    });

    it('should track cache hits and misses', async () => {
      const mockCommitments = [
        {
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://cust1.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 3,
        },
      ];

      const mockMetrics = {
        custodianId: 'cust-1',
        health: { uptimePercent: 95, availability: true, responseTimeP95Ms: 100, slaCompliance: true },
        bandwidth: { currentUsageMbps: 50, declaredMbps: 100 },
        reputation: { specializationBonus: 0.05 },
        economic: { stewardTier: 3 },
      };

      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve(mockCommitments as any));
      shefaMock.getMetrics.and.returnValue(Promise.resolve(mockMetrics as any));

      const statsBefore = service.getStatistics();
      await service.selectBestCustodian('content-1');
      const statsAfter1 = service.getStatistics();

      expect(statsAfter1.cacheMisses).toBe(statsBefore.cacheMisses + 1);

      await service.selectBestCustodian('content-1');
      const statsAfter2 = service.getStatistics();

      expect(statsAfter2.cacheHits).toBe(statsAfter1.cacheHits + 1);
    });
  });

  describe('selectionStats signal', () => {
    it('should have selectionStats readonly signal', () => {
      expect(service.selectionStats).toBeDefined();
    });

    it('should track selections attempted', async () => {
      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve([]));

      const statsBefore = service.selectionStats();
      await service.selectBestCustodian('content-1');
      const statsAfter = service.selectionStats();

      expect(statsAfter.selectionsAttempted).toBe(statsBefore.selectionsAttempted + 1);
    });
  });

  describe('successRate computed signal', () => {
    it('should have successRate computed signal', () => {
      expect(service.successRate).toBeDefined();
    });

    it('should return 0 when no selections attempted', () => {
      const rate = service.successRate();
      expect(rate).toBe(0);
    });

    it('should calculate success percentage correctly', async () => {
      const mockCommitments = [
        {
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://cust1.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 3,
        },
      ];

      const mockMetrics = {
        custodianId: 'cust-1',
        health: { uptimePercent: 95, availability: true, responseTimeP95Ms: 100, slaCompliance: true },
        bandwidth: { currentUsageMbps: 50, declaredMbps: 100 },
        reputation: { specializationBonus: 0.05 },
        economic: { stewardTier: 3 },
      };

      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve(mockCommitments as any));
      shefaMock.getMetrics.and.returnValue(Promise.resolve(mockMetrics as any));

      const result = await service.selectBestCustodian('content-1');

      if (result) {
        const rate = service.successRate();
        expect(rate).toBeGreaterThan(0);
        expect(rate).toBeLessThanOrEqual(100);
      }
    });
  });

  describe('Scoring calculations', () => {
    it('should calculate health score correctly', async () => {
      const mockCommitments = [
        {
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://cust1.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 3,
        },
      ];

      const mockMetrics = {
        custodianId: 'cust-1',
        health: { uptimePercent: 99, availability: true, responseTimeP95Ms: 50, slaCompliance: true },
        bandwidth: { currentUsageMbps: 50, declaredMbps: 100 },
        reputation: { specializationBonus: 0 },
        economic: { stewardTier: 3 },
      };

      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve(mockCommitments as any));
      shefaMock.getMetrics.and.returnValue(Promise.resolve(mockMetrics as any));

      const result = await service.selectBestCustodian('content-1');

      expect(result?.health).toBe(99);
      expect(result?.breakdown.healthScore).toBeGreaterThan(90);
    });

    it('should calculate latency score correctly', async () => {
      const mockCommitments = [
        {
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://cust1.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 3,
        },
      ];

      const mockMetrics = {
        custodianId: 'cust-1',
        health: { uptimePercent: 95, availability: true, responseTimeP95Ms: 100, slaCompliance: true },
        bandwidth: { currentUsageMbps: 50, declaredMbps: 100 },
        reputation: { specializationBonus: 0 },
        economic: { stewardTier: 3 },
      };

      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve(mockCommitments as any));
      shefaMock.getMetrics.and.returnValue(Promise.resolve(mockMetrics as any));

      const result = await service.selectBestCustodian('content-1');

      expect(result?.latency).toBe(100);
      expect(result?.breakdown.latencyScore).toBeGreaterThan(80);
    });

    it('should calculate bandwidth score based on utilization', async () => {
      const mockCommitments = [
        {
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://cust1.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 3,
        },
      ];

      const mockMetrics = {
        custodianId: 'cust-1',
        health: { uptimePercent: 95, availability: true, responseTimeP95Ms: 100, slaCompliance: true },
        bandwidth: { currentUsageMbps: 25, declaredMbps: 100 },
        reputation: { specializationBonus: 0 },
        economic: { stewardTier: 3 },
      };

      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve(mockCommitments as any));
      shefaMock.getMetrics.and.returnValue(Promise.resolve(mockMetrics as any));

      const result = await service.selectBestCustodian('content-1');

      expect(result?.bandwidth).toBeLessThan(1);
      expect(result?.breakdown.bandwidthScore).toBe(100);
    });

    it('should calculate specialization score from bonus', async () => {
      const mockCommitments = [
        {
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://cust1.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 3,
        },
      ];

      const mockMetrics = {
        custodianId: 'cust-1',
        health: { uptimePercent: 95, availability: true, responseTimeP95Ms: 100, slaCompliance: true },
        bandwidth: { currentUsageMbps: 50, declaredMbps: 100 },
        reputation: { specializationBonus: 0.08 },
        economic: { stewardTier: 3 },
      };

      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve(mockCommitments as any));
      shefaMock.getMetrics.and.returnValue(Promise.resolve(mockMetrics as any));

      const result = await service.selectBestCustodian('content-1');

      expect(result?.specialization).toBe(0.08);
      expect(result?.breakdown.specializationScore).toBeGreaterThan(70);
    });

    it('should apply tier bonus to score', async () => {
      const mockCommitments = [
        {
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://cust1.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 4,
        },
        {
          custodianId: 'cust-2',
          doorwayEndpoint: 'http://cust2.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 1,
        },
      ];

      const mockMetrics = {
        custodianId: 'cust-1',
        health: { uptimePercent: 95, availability: true, responseTimeP95Ms: 100, slaCompliance: true },
        bandwidth: { currentUsageMbps: 50, declaredMbps: 100 },
        reputation: { specializationBonus: 0 },
        economic: { stewardTier: 4 },
      };

      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve(mockCommitments as any));
      shefaMock.getMetrics.and.callFake((custId: string) => {
        if (custId === 'cust-1') return Promise.resolve(mockMetrics as any);
        return Promise.resolve({
          ...mockMetrics,
          custodianId: 'cust-2',
          economic: { stewardTier: 1 },
        } as any);
      });

      const result = await service.selectBestCustodian('content-1');

      expect(result?.custodian.id).toBe('cust-1');
      expect(result?.finalScore).toBeGreaterThan(0);
    });

    it('should handle extreme latency values', async () => {
      const mockCommitments = [
        {
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://cust1.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 3,
        },
      ];

      const mockMetrics = {
        custodianId: 'cust-1',
        health: { uptimePercent: 95, availability: true, responseTimeP95Ms: 2500, slaCompliance: true },
        bandwidth: { currentUsageMbps: 50, declaredMbps: 100 },
        reputation: { specializationBonus: 0 },
        economic: { stewardTier: 3 },
      };

      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve(mockCommitments as any));
      shefaMock.getMetrics.and.returnValue(Promise.resolve(mockMetrics as any));

      const result = await service.selectBestCustodian('content-1');

      expect(result).toBeNull();
    });

    it('should handle extremely high bandwidth utilization', async () => {
      const mockCommitments = [
        {
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://cust1.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 3,
        },
      ];

      const mockMetrics = {
        custodianId: 'cust-1',
        health: { uptimePercent: 95, availability: true, responseTimeP95Ms: 100, slaCompliance: true },
        bandwidth: { currentUsageMbps: 99, declaredMbps: 100 },
        reputation: { specializationBonus: 0 },
        economic: { stewardTier: 3 },
      };

      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve(mockCommitments as any));
      shefaMock.getMetrics.and.returnValue(Promise.resolve(mockMetrics as any));

      const result = await service.selectBestCustodian('content-1');

      expect(result).toBeNull();
    });
  });

  describe('Error handling and resilience', () => {
    it('should handle metrics fetch errors gracefully', async () => {
      const mockCommitments = [
        {
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://cust1.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 3,
        },
      ];

      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve(mockCommitments as any));
      shefaMock.getMetrics.and.returnValue(Promise.reject(new Error('Metrics unavailable')));

      const result = await service.selectBestCustodian('content-1');

      expect(result).toBeNull();
    });

    it('should handle commitments fetch errors gracefully', async () => {
      commitmentsMock.getCommitmentsForContent.and.returnValue(
        Promise.reject(new Error('Commitments unavailable'))
      );

      const result = await service.selectBestCustodian('content-1');

      expect(result).toBeNull();
    });

    it('should handle mixed success and failure for multiple custodians', async () => {
      const mockCommitments = [
        {
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://cust1.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 3,
        },
        {
          custodianId: 'cust-2',
          doorwayEndpoint: 'http://cust2.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 2,
        },
      ];

      const mockMetrics1 = {
        custodianId: 'cust-1',
        health: { uptimePercent: 95, availability: true, responseTimeP95Ms: 100, slaCompliance: true },
        bandwidth: { currentUsageMbps: 50, declaredMbps: 100 },
        reputation: { specializationBonus: 0.05 },
        economic: { stewardTier: 3 },
      };

      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve(mockCommitments as any));
      shefaMock.getMetrics.and.callFake((custId: string) => {
        if (custId === 'cust-1') return Promise.resolve(mockMetrics1 as any);
        return Promise.reject(new Error('Metrics unavailable'));
      });

      const result = await service.selectBestCustodian('content-1');

      expect(result).toBeTruthy();
      expect(result?.custodian.id).toBe('cust-1');
    });
  });

  describe('Performance and monitoring', () => {
    it('should track successful selections', async () => {
      const mockCommitments = [
        {
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://cust1.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 3,
        },
      ];

      const mockMetrics = {
        custodianId: 'cust-1',
        health: { uptimePercent: 95, availability: true, responseTimeP95Ms: 100, slaCompliance: true },
        bandwidth: { currentUsageMbps: 50, declaredMbps: 100 },
        reputation: { specializationBonus: 0.05 },
        economic: { stewardTier: 3 },
      };

      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve(mockCommitments as any));
      shefaMock.getMetrics.and.returnValue(Promise.resolve(mockMetrics as any));

      const statsBefore = service.getStatistics();
      await service.selectBestCustodian('content-1');
      const statsAfter = service.getStatistics();

      expect(statsAfter.selectionsSuccessful).toBe(statsBefore.selectionsSuccessful + 1);
    });

    it('should report reasonable success rate', async () => {
      const mockCommitments = [
        {
          custodianId: 'cust-1',
          doorwayEndpoint: 'http://cust1.com',
          domain: 'elohim',
          epic: 'governance',
          stewardTier: 3,
        },
      ];

      const mockMetrics = {
        custodianId: 'cust-1',
        health: { uptimePercent: 95, availability: true, responseTimeP95Ms: 100, slaCompliance: true },
        bandwidth: { currentUsageMbps: 50, declaredMbps: 100 },
        reputation: { specializationBonus: 0.05 },
        economic: { stewardTier: 3 },
      };

      commitmentsMock.getCommitmentsForContent.and.returnValue(Promise.resolve(mockCommitments as any));
      shefaMock.getMetrics.and.returnValue(Promise.resolve(mockMetrics as any));

      await service.selectBestCustodian('content-1');
      await service.selectBestCustodian('content-2');
      await service.selectBestCustodian('content-3');

      const successRate = service.successRate();
      expect(successRate).toBeGreaterThanOrEqual(0);
      expect(successRate).toBeLessThanOrEqual(100);
    });
  });
});
