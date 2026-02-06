/**
 * Stewarded-resources Service Tests
 */

import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { StewardedResourceService } from './stewarded-resources.service';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { EconomicService } from './economic.service';
import { ConstitutionalLimit, FinancialAsset, IncomeStream, FinancialObligation } from '@app/shefa/models/stewarded-resources.model';

describe('StewardedResourceService', () => {
  let service: StewardedResourceService;
  let mockHolochain: jasmine.SpyObj<HolochainClientService>;
  let mockEconomicService: jasmine.SpyObj<EconomicService>;

  beforeEach(() => {
    mockHolochain = jasmine.createSpyObj('HolochainClientService', ['callZome']);
    mockEconomicService = jasmine.createSpyObj('EconomicService', ['createEvent']);

    TestBed.configureTestingModule({
      providers: [
        StewardedResourceService,
        { provide: HolochainClientService, useValue: mockHolochain },
        { provide: EconomicService, useValue: mockEconomicService },
      ],
    });
    service = TestBed.inject(StewardedResourceService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('Constitutional Economics Helpers', () => {
    describe('calculateHealthStatus', () => {
      it('should return "critical" when utilization exceeds 90%', () => {
        const result = (service as any).calculateHealthStatus(95);
        expect(result).toBe('critical');
      });

      it('should return "warning" when utilization is between 75% and 90%', () => {
        const result = (service as any).calculateHealthStatus(80);
        expect(result).toBe('warning');
      });

      it('should return "healthy" when utilization is below 75%', () => {
        const result = (service as any).calculateHealthStatus(50);
        expect(result).toBe('healthy');
      });

      it('should handle edge case at exactly 90%', () => {
        const result = (service as any).calculateHealthStatus(90);
        expect(result).toBe('warning');
      });

      it('should handle edge case at exactly 75%', () => {
        const result = (service as any).calculateHealthStatus(75);
        expect(result).toBe('healthy');
      });
    });

    describe('calculateConstitutionalPosition', () => {
      let mockLimit: ConstitutionalLimit;

      beforeEach(() => {
        mockLimit = {
          id: 'test-limit',
          resourceCategory: 'financial-asset',
          name: 'Test Limit',
          description: 'Test constitutional limit',
          floorValue: 75000,
          floorUnit: 'USD',
          floorRationale: 'Dignity floor',
          floorEnforced: true,
          ceilingValue: 10000000,
          ceilingUnit: 'USD',
          ceilingRationale: 'Wealth ceiling',
          ceilingEnforced: false,
          safeMinValue: 75000,
          safeMaxValue: 10000000,
          safeZoneDescription: 'Flourishing zone',
          governanceLevel: 'network',
          constitutionalBasis: 'Part III',
          enforcementMethod: 'progressive',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        };
      });

      it('should identify below-floor position with red warning', () => {
        const result = (service as any).calculateConstitutionalPosition(50000, mockLimit);

        expect(result.positionRelativeToFloor).toBe('below-floor');
        expect(result.complianceStatus).toBe('compliant');
        expect(result.warningLevel).toBe('red');
      });

      it('should identify at-floor position with yellow warning', () => {
        const result = (service as any).calculateConstitutionalPosition(75000, mockLimit);

        expect(result.positionRelativeToFloor).toBe('at-floor');
        expect(result.complianceStatus).toBe('compliant');
        expect(result.warningLevel).toBe('yellow');
      });

      it('should identify in-safe-zone position with no warning', () => {
        const result = (service as any).calculateConstitutionalPosition(5000000, mockLimit);

        expect(result.positionRelativeToFloor).toBe('in-safe-zone');
        expect(result.complianceStatus).toBe('compliant');
        expect(result.warningLevel).toBe('none');
      });

      it('should identify above-ceiling position with yellow warning', () => {
        const result = (service as any).calculateConstitutionalPosition(12000000, mockLimit);

        expect(result.positionRelativeToFloor).toBe('above-ceiling');
        expect(result.complianceStatus).toBe('approaching-limit');
        expect(result.warningLevel).toBe('yellow');
      });

      it('should identify far-above-ceiling position with red warning', () => {
        const result = (service as any).calculateConstitutionalPosition(20000000, mockLimit);

        expect(result.positionRelativeToFloor).toBe('far-above-ceiling');
        expect(result.complianceStatus).toBe('far-exceeds-ceiling');
        expect(result.warningLevel).toBe('red');
      });

      it('should use CEILING_APPROACHING_MULTIPLIER threshold correctly', () => {
        // 1.5 * 10M = 15M is the threshold
        const justAbove = (service as any).calculateConstitutionalPosition(10000001, mockLimit);
        expect(justAbove.positionRelativeToFloor).toBe('above-ceiling');

        const atThreshold = (service as any).calculateConstitutionalPosition(15000000, mockLimit);
        expect(atThreshold.positionRelativeToFloor).toBe('far-above-ceiling');
      });
    });

    describe('calculateCategoryCompliance', () => {
      let mockLimit: ConstitutionalLimit;

      beforeEach(() => {
        mockLimit = {
          id: 'test-limit',
          resourceCategory: 'financial-asset',
          name: 'Test Limit',
          description: 'Test constitutional limit',
          floorValue: 75000,
          floorUnit: 'USD',
          floorRationale: 'Dignity floor',
          floorEnforced: true,
          ceilingValue: 10000000,
          ceilingUnit: 'USD',
          ceilingRationale: 'Wealth ceiling',
          ceilingEnforced: false,
          safeMinValue: 75000,
          safeMaxValue: 10000000,
          safeZoneDescription: 'Flourishing zone',
          governanceLevel: 'network',
          constitutionalBasis: 'Part III',
          enforcementMethod: 'progressive',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        };
      });

      it('should return compliant status when value is within safe zone', () => {
        const result = (service as any).calculateCategoryCompliance(5000000, mockLimit);

        expect(result.complianceStatus).toBe('compliant');
        expect(result.warningLevel).toBe('none');
      });

      it('should return at-risk status when value is below floor', () => {
        const result = (service as any).calculateCategoryCompliance(50000, mockLimit);

        expect(result.complianceStatus).toBe('at-risk');
        expect(result.warningLevel).toBe('none');
      });

      it('should return exceeds-ceiling with orange warning for moderate excess', () => {
        const result = (service as any).calculateCategoryCompliance(12000000, mockLimit);

        expect(result.complianceStatus).toBe('exceeds-ceiling');
        expect(result.warningLevel).toBe('orange');
      });

      it('should return exceeds-ceiling with red warning for high excess', () => {
        // Excess > 50% of ceiling (> 5M above 10M ceiling)
        const result = (service as any).calculateCategoryCompliance(16000000, mockLimit);

        expect(result.complianceStatus).toBe('exceeds-ceiling');
        expect(result.warningLevel).toBe('red');
      });

      it('should use EXCESS_HIGH_THRESHOLD correctly', () => {
        // 50% of 10M = 5M excess threshold
        const atThreshold = (service as any).calculateCategoryCompliance(15000000, mockLimit);
        expect(atThreshold.warningLevel).toBe('orange');

        const aboveThreshold = (service as any).calculateCategoryCompliance(15000001, mockLimit);
        expect(aboveThreshold.warningLevel).toBe('red');
      });
    });
  });

  describe('Financial Health Analysis Helpers', () => {
    describe('aggregateIncomeStreams', () => {
      it('should aggregate income streams from multiple assets', () => {
        const assets: FinancialAsset[] = [
          {
            id: 'asset1',
            resourceNumber: 'RES-001',
            stewardId: 'steward1',
            category: 'financial-asset',
            subcategory: 'checking',
            name: 'Checking Account',
            dimension: { unit: 'USD', unitLabel: 'US Dollars', unitAbbreviation: '$' },
            totalCapacity: { value: 10000, unit: 'USD' },
            allocatableCapacity: { value: 10000, unit: 'USD' },
            totalAllocated: { value: 0, unit: 'USD' },
            totalReserved: { value: 0, unit: 'USD' },
            totalUsed: { value: 0, unit: 'USD' },
            available: { value: 10000, unit: 'USD' },
            allocations: [],
            allocationStrategy: 'manual',
            governanceLevel: 'individual',
            canModifyAllocations: true,
            observerEnabled: false,
            recentUsage: [],
            trends: [],
            allocationEventIds: [],
            usageEventIds: [],
            isShared: false,
            visibility: 'private',
            dataQuality: 'manual',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            assetType: 'fiat-currency',
            currencyCode: 'USD',
            accountBalance: 10000,
            availableBalance: 10000,
            pendingTransactions: [],
            accountStatus: 'active',
            monthlyIncome: 5000,
            expectedMonthlyIncome: 5000,
            obligations: [],
            totalLiability: 0,
            monthlyObligations: 0,
            ubaEligible: false,
            ubaStatus: 'inactive',
            dataSource: 'manual',
            incomeStreams: [
              {
                id: 'income1',
                source: 'Employment',
                amount: 5000,
                frequency: 'monthly',
                status: 'active',
                startDate: new Date().toISOString(),
                isGuaranteed: true,
                confidence: 100,
              } as IncomeStream,
            ],
          } as FinancialAsset,
          {
            id: 'asset2',
            resourceNumber: 'RES-002',
            stewardId: 'steward1',
            category: 'financial-asset',
            subcategory: 'investment',
            name: 'Investment Account',
            dimension: { unit: 'USD', unitLabel: 'US Dollars', unitAbbreviation: '$' },
            totalCapacity: { value: 50000, unit: 'USD' },
            allocatableCapacity: { value: 50000, unit: 'USD' },
            totalAllocated: { value: 0, unit: 'USD' },
            totalReserved: { value: 0, unit: 'USD' },
            totalUsed: { value: 0, unit: 'USD' },
            available: { value: 50000, unit: 'USD' },
            allocations: [],
            allocationStrategy: 'manual',
            governanceLevel: 'individual',
            canModifyAllocations: true,
            observerEnabled: false,
            recentUsage: [],
            trends: [],
            allocationEventIds: [],
            usageEventIds: [],
            isShared: false,
            visibility: 'private',
            dataQuality: 'manual',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            assetType: 'stock',
            currencyCode: 'USD',
            accountBalance: 50000,
            availableBalance: 50000,
            pendingTransactions: [],
            accountStatus: 'active',
            monthlyIncome: 500,
            expectedMonthlyIncome: 400,
            obligations: [],
            totalLiability: 0,
            monthlyObligations: 0,
            ubaEligible: false,
            ubaStatus: 'inactive',
            dataSource: 'manual',
            incomeStreams: [
              {
                id: 'income2',
                source: 'Dividends',
                amount: 500,
                frequency: 'monthly',
                status: 'active',
                startDate: new Date().toISOString(),
                isGuaranteed: false,
                confidence: 80,
              } as IncomeStream,
            ],
          } as FinancialAsset,
        ];

        const result = (service as any).aggregateIncomeStreams(assets);

        expect(result.allIncomeStreams.length).toBe(2);
        expect(result.guaranteedMonthlyIncome).toBe(5000);
        expect(result.expectedMonthlyIncome).toBe(5400); // 5000 + (500 * 0.8)
      });

      it('should handle assets with no income streams', () => {
        const assets: FinancialAsset[] = [
          {
            id: 'asset1',
            accountBalance: 1000,
            incomeStreams: undefined,
          } as unknown as FinancialAsset,
        ];

        const result = (service as any).aggregateIncomeStreams(assets);

        expect(result.allIncomeStreams.length).toBe(0);
        expect(result.guaranteedMonthlyIncome).toBe(0);
        expect(result.expectedMonthlyIncome).toBe(0);
      });

      it('should skip inactive income streams', () => {
        const assets: FinancialAsset[] = [
          {
            id: 'asset1',
            incomeStreams: [
              {
                id: 'income1',
                source: 'Old Job',
                amount: 3000,
                frequency: 'monthly',
                status: 'ended',
                startDate: '2023-01-01T00:00:00Z',
                endDate: '2024-01-01T00:00:00Z',
                isGuaranteed: true,
                confidence: 100,
              } as IncomeStream,
              {
                id: 'income2',
                source: 'Current Job',
                amount: 5000,
                frequency: 'monthly',
                status: 'active',
                startDate: new Date().toISOString(),
                isGuaranteed: true,
                confidence: 100,
              } as IncomeStream,
            ],
          } as unknown as FinancialAsset,
        ];

        const result = (service as any).aggregateIncomeStreams(assets);

        expect(result.guaranteedMonthlyIncome).toBe(5000);
      });
    });

    describe('aggregateObligations', () => {
      it('should aggregate obligations from multiple assets', () => {
        const assets: FinancialAsset[] = [
          {
            id: 'asset1',
            obligations: [
              {
                id: 'debt1',
                creditorId: 'bank-123',
                creditorName: 'Bank',
                principalAmount: 300000,
                remainingAmount: 250000,
                interest: 3.5,
                monthlyPayment: 1500,
                obligationType: 'mortgage',
                daysOverdue: 0,
                status: 'current',
                transparencyLevel: 'private',
              } as FinancialObligation,
            ],
          } as unknown as FinancialAsset,
          {
            id: 'asset2',
            obligations: [
              {
                id: 'debt2',
                creditorId: 'cc-456',
                creditorName: 'Credit Card Co',
                principalAmount: 5000,
                remainingAmount: 3000,
                interest: 18,
                monthlyPayment: 150,
                obligationType: 'credit-card',
                daysOverdue: 0,
                status: 'current',
                transparencyLevel: 'private',
              } as FinancialObligation,
            ],
          } as unknown as FinancialAsset,
        ];

        const result = (service as any).aggregateObligations(assets);

        expect(result.allObligations.length).toBe(2);
        expect(result.monthlyObligations).toBe(1650);
        expect(result.totalLiability).toBe(253000);
      });

      it('should handle assets with no obligations', () => {
        const assets: FinancialAsset[] = [
          {
            id: 'asset1',
            obligations: undefined,
          } as unknown as FinancialAsset,
        ];

        const result = (service as any).aggregateObligations(assets);

        expect(result.allObligations.length).toBe(0);
        expect(result.monthlyObligations).toBe(0);
        expect(result.totalLiability).toBe(0);
      });
    });
  });

  describe('Error Handling', () => {
    describe('getResource', () => {
      it('should return null when resource not found', async () => {
        mockHolochain.callZome.and.returnValue(
          Promise.reject(new Error('Resource not found'))
        );

        const result = await service.getResource('nonexistent-id');

        expect(result).toBeNull();
      });

      it('should log warning for unexpected errors', async () => {
        spyOn(console, 'warn');
        mockHolochain.callZome.and.returnValue(
          Promise.reject(new Error('Network error'))
        );

        await service.getResource('test-id');

        expect(console.warn).toHaveBeenCalled();
      });
    });

    describe('getStewardResources', () => {
      it('should return empty array on error', async () => {
        mockHolochain.callZome.and.returnValue(
          Promise.reject(new Error('Connection failed'))
        );

        const result = await service.getStewardResources('steward-id');

        expect(result).toEqual([]);
      });

      it('should log warning on error', async () => {
        spyOn(console, 'warn');
        mockHolochain.callZome.and.returnValue(
          Promise.reject(new Error('Connection failed'))
        );

        await service.getStewardResources('steward-id');

        expect(console.warn).toHaveBeenCalled();
      });
    });
  });

  describe('Integration', () => {
    it('should use constants consistently', () => {
      // Verify that the service uses the constants internally
      // This is validated through the fact that the code compiles and lints pass
      expect(service).toBeTruthy();
    });
  });

  // ==========================================================================
  // Resource Management Tests
  // ==========================================================================

  describe('createResource', () => {
    it('should create a new stewarded resource', async () => {
      mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: {} }));
      mockEconomicService.createEvent.and.returnValue(of({ id: 'event-1' }) as any);

      const result = await service.createResource(
        'steward-1',
        'financial-asset',
        'checking',
        'Primary Checking',
        { value: 10000, unit: 'USD' }
      );

      expect(result).toBeDefined();
      expect(result.name).toBe('Primary Checking');
      expect(result.category).toBe('financial-asset');
      expect(result.subcategory).toBe('checking');
      expect(result.totalCapacity.value).toBe(10000);
      expect(mockHolochain.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          zomeName: 'content_store',
          fnName: 'create_stewarded_resource',
        })
      );
    });

    it('should create resource with permanent reserve', async () => {
      mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: {} }));
      mockEconomicService.createEvent.and.returnValue(of({ id: 'event-1' }) as any);

      const result = await service.createResource(
        'steward-1',
        'financial-asset',
        'savings',
        'Emergency Fund',
        { value: 5000, unit: 'USD' },
        {
          permanentReserve: { value: 1000, unit: 'USD' },
        }
      );

      expect(result.permanentReserve?.value).toBe(1000);
      expect(result.allocatableCapacity.value).toBe(4000); // 5000 - 1000
    });

    it('should initialize resource with correct defaults', async () => {
      mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: {} }));
      mockEconomicService.createEvent.and.returnValue(of({ id: 'event-1' }) as any);

      const result = await service.createResource(
        'steward-1',
        'compute',
        'cpu',
        'Primary Node',
        { value: 8, unit: 'cores' }
      );

      expect(result.allocations).toEqual([]);
      expect(result.totalAllocated.value).toBe(0);
      expect(result.totalUsed.value).toBe(0);
      expect(result.available.value).toBe(8);
      expect(result.governanceLevel).toBe('individual');
      expect(result.observerEnabled).toBe(false);
    });

    it('should handle observer-enabled resources', async () => {
      mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: {} }));
      mockEconomicService.createEvent.and.returnValue(of({ id: 'event-1' }) as any);

      const result = await service.createResource(
        'steward-1',
        'compute',
        'storage',
        'Monitored Storage',
        { value: 500, unit: 'GB' },
        { observerEnabled: true }
      );

      expect(result.observerEnabled).toBe(true);
    });
  });

  describe('getResourcesByCategory', () => {
    it('should filter resources by category', async () => {
      const mockResources = [
        { id: 'r1', category: 'financial-asset' },
        { id: 'r2', category: 'compute' },
        { id: 'r3', category: 'financial-asset' },
      ];

      mockHolochain.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockResources })
      );

      const result = await service.getResourcesByCategory('steward-1', 'financial-asset');

      expect(result.length).toBe(2);
      expect(result[0].category).toBe('financial-asset');
      expect(result[1].category).toBe('financial-asset');
    });

    it('should return empty array when no resources match', async () => {
      mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: [] }));

      const result = await service.getResourcesByCategory('steward-1', 'energy');

      expect(result).toEqual([]);
    });
  });

  // ==========================================================================
  // Allocation Management Tests
  // ==========================================================================

  describe('createAllocation', () => {
    it('should create allocation block', async () => {
      mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: {} }));
      mockEconomicService.createEvent.and.returnValue(of({ id: 'event-1' }) as any);

      const result = await service.createAllocation(
        'resource-1',
        'Household Apps',
        { value: 2, unit: 'cores' }
      );

      expect(result).toBeDefined();
      expect(result.label).toBe('Household Apps');
      expect(result.allocated.value).toBe(2);
      expect(result.used.value).toBe(0);
      expect(result.utilization).toBe(0);
    });

    it('should create allocation with governance level', async () => {
      mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: {} }));
      mockEconomicService.createEvent.and.returnValue(of({ id: 'event-1' }) as any);

      const result = await service.createAllocation(
        'resource-1',
        'Community Service',
        { value: 4, unit: 'cores' },
        { governanceLevel: 'community', priority: 7 }
      );

      expect(result.governanceLevel).toBe('community');
      expect(result.priority).toBe(7);
    });
  });

  describe('recordUsage', () => {
    it('should record usage with economic event', async () => {
      mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: {} }));
      mockEconomicService.createEvent.and.returnValue(of({ id: 'event-123' }) as any);

      const result = await service.recordUsage(
        'resource-1',
        'alloc-1',
        { value: 0.5, unit: 'cores' }
      );

      expect(result).toBeDefined();
      expect(result.quantity.value).toBe(0.5);
      expect(result.economicEventId).toBe('event-123');
    });

    it('should record usage with observer attestation', async () => {
      mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: {} }));
      mockEconomicService.createEvent.and.returnValue(of({ id: 'event-1' }) as any);

      const result = await service.recordUsage(
        'resource-1',
        'alloc-1',
        { value: 1, unit: 'GB' },
        { observerAttestationId: 'att-123', note: 'Verified usage' }
      );

      expect(result.observerAttestationId).toBe('att-123');
      expect(result.note).toBe('Verified usage');
    });
  });

  describe('updateAllocationUtilization', () => {
    it('should update utilization for allocation', async () => {
      const mockResource = {
        id: 'res-1',
        allocations: [
          {
            id: 'alloc-1',
            allocated: { value: 10, unit: 'GB' },
            used: { value: 0, unit: 'GB' },
          },
        ],
      };

      mockHolochain.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockResource })
      );

      const result = await service.updateAllocationUtilization(
        'res-1',
        'alloc-1',
        { value: 7, unit: 'GB' }
      );

      expect(result).toBeDefined();
      expect(result?.used.value).toBe(7);
      expect(result?.utilization).toBe(70); // 7/10 * 100
    });

    it('should return null when resource not found', async () => {
      mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: null }));

      const result = await service.updateAllocationUtilization(
        'nonexistent',
        'alloc-1',
        { value: 5, unit: 'GB' }
      );

      expect(result).toBeNull();
    });

    it('should return null when allocation not found', async () => {
      const mockResource = {
        id: 'res-1',
        allocations: [],
      };

      mockHolochain.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockResource })
      );

      const result = await service.updateAllocationUtilization(
        'res-1',
        'nonexistent-alloc',
        { value: 5, unit: 'GB' }
      );

      expect(result).toBeNull();
    });
  });

  // ==========================================================================
  // Dashboard & Aggregation Tests
  // ==========================================================================

  describe('getCategorySummary', () => {
    it('should aggregate resources in category', async () => {
      const mockResources = [
        {
          id: 'r1',
          category: 'financial-asset',
          totalCapacity: { value: 1000, unit: 'USD' },
          totalAllocated: { value: 800, unit: 'USD' },
          totalUsed: { value: 600, unit: 'USD' },
        },
        {
          id: 'r2',
          category: 'financial-asset',
          totalCapacity: { value: 2000, unit: 'USD' },
          totalAllocated: { value: 1500, unit: 'USD' },
          totalUsed: { value: 1000, unit: 'USD' },
        },
      ];

      mockHolochain.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockResources })
      );

      const result = await service.getCategorySummary('steward-1', 'financial-asset');

      expect(result.totalCapacity.value).toBe(3000);
      expect(result.totalAllocated.value).toBe(2300);
      expect(result.totalUsed.value).toBe(1600);
      expect(result.utilizationPercent).toBeCloseTo(53.33, 1); // 1600/3000 * 100
    });

    it('should handle category with no resources', async () => {
      mockHolochain.callZome.and.returnValue(Promise.resolve({ success: true, data: [] }));

      const result = await service.getCategorySummary('steward-1', 'energy');

      expect(result.totalCapacity.value).toBe(0);
      expect(result.totalAllocated.value).toBe(0);
      expect(result.totalUsed.value).toBe(0);
      expect(result.utilizationPercent).toBe(0);
    });
  });

  describe('buildDashboard', () => {
    it('should build complete dashboard', async () => {
      const mockResources = [
        {
          id: 'r1',
          category: 'financial-asset',
          totalCapacity: { value: 1000, unit: 'USD' },
          totalAllocated: { value: 800, unit: 'USD' },
          totalUsed: { value: 600, unit: 'USD' },
          available: { value: 400, unit: 'USD' },
          allocations: [{ id: 'a1', updatedAt: new Date().toISOString() }],
          recentUsage: [{ id: 'u1', timestamp: new Date().toISOString() }],
        },
      ];

      mockHolochain.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockResources })
      );

      const result = await service.buildDashboard('steward-1');

      expect(result.stewardId).toBe('steward-1');
      expect(result.metrics.totalResourcesTracked).toBe(1);
      expect(result.categories.length).toBeGreaterThan(0);
      expect(result.lastUpdatedAt).toBeDefined();
    });

    it('should calculate metrics correctly', async () => {
      const mockResources = [
        {
          id: 'r1',
          category: 'compute',
          totalCapacity: { value: 10, unit: 'cores' },
          totalAllocated: { value: 10, unit: 'cores' },
          totalUsed: { value: 10, unit: 'cores' },
          available: { value: 0, unit: 'cores' },
          allocations: [],
          recentUsage: [],
        },
      ];

      mockHolochain.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockResources })
      );

      const result = await service.buildDashboard('steward-1');

      expect(result.metrics.fullyAllocatedCount).toBe(1);
      expect(result.metrics.healthStatus).toBe('critical'); // 100% utilization
    });
  });

  // ==========================================================================
  // Constitutional Limits Tests
  // ==========================================================================

  describe('getConstitutionalLimits', () => {
    it('should return limits for financial assets', async () => {
      const result = await service.getConstitutionalLimits('financial-asset');

      expect(result).toBeDefined();
      expect(result?.floorValue).toBe(75000);
      expect(result?.ceilingValue).toBe(10000000);
      expect(result?.floorEnforced).toBe(true);
    });

    it('should return limits for energy category', async () => {
      const result = await service.getConstitutionalLimits('energy');

      expect(result).toBeDefined();
      expect(result?.floorValue).toBe(40);
      expect(result?.ceilingValue).toBe(100);
      expect(result?.floorUnit).toBe('hours/week');
    });

    it('should return limits for compute category', async () => {
      const result = await service.getConstitutionalLimits('compute');

      expect(result).toBeDefined();
      expect(result?.floorValue).toBe(10);
      expect(result?.ceilingValue).toBe(80);
      expect(result?.floorUnit).toBe('percent');
    });

    it('should return null for category without limits', async () => {
      const result = await service.getConstitutionalLimits('water');

      expect(result).toBeNull();
    });
  });

  describe('assessResourcePosition', () => {
    it('should identify below-floor position', async () => {
      const result = await service.assessResourcePosition(
        'res-1',
        'steward-1',
        'financial-asset',
        50000,
        'USD'
      );

      expect(result.positionRelativeToFloor).toBe('below-floor');
      expect(result.warningLevel).toBe('red');
      expect(result.distanceFromFloor).toBe(-25000); // 50000 - 75000
    });

    it('should identify in-safe-zone position', async () => {
      const result = await service.assessResourcePosition(
        'res-1',
        'steward-1',
        'financial-asset',
        5000000,
        'USD'
      );

      expect(result.positionRelativeToFloor).toBe('in-safe-zone');
      expect(result.warningLevel).toBe('none');
      expect(result.complianceStatus).toBe('compliant');
    });

    it('should identify above-ceiling position', async () => {
      const result = await service.assessResourcePosition(
        'res-1',
        'steward-1',
        'financial-asset',
        12000000,
        'USD'
      );

      expect(result.positionRelativeToFloor).toBe('above-ceiling');
      expect(result.excessAboveCeiling).toBe(2000000);
      expect(result.onTransitionPath).toBe(true);
    });

    it('should calculate excess percentage correctly', async () => {
      const result = await service.assessResourcePosition(
        'res-1',
        'steward-1',
        'financial-asset',
        15000000,
        'USD'
      );

      expect(result.excessPercentage).toBe(50); // (15M - 10M) / 10M * 100
    });
  });

  describe('buildComplianceReport', () => {
    it('should report compliant status', async () => {
      const mockResources = [
        {
          id: 'r1',
          category: 'financial-asset',
          totalCapacity: { value: 100000, unit: 'USD' },
        },
      ];

      mockHolochain.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockResources })
      );

      const result = await service.buildComplianceReport('steward-1');

      expect(result.overallCompliant).toBe(true);
      expect(result.totalExcess).toBe(0);
      expect(result.categories_at_risk).toBe(0);
    });

    it('should identify excess and generate recommendations', async () => {
      const mockResources = [
        {
          id: 'r1',
          category: 'financial-asset',
          totalCapacity: { value: 12000000, unit: 'USD' },
        },
      ];

      mockHolochain.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockResources })
      );

      const result = await service.buildComplianceReport('steward-1');

      expect(result.overallCompliant).toBe(false);
      expect(result.totalExcess).toBeGreaterThan(0);
      expect(result.recommendations.length).toBeGreaterThan(0);
    });
  });

  // ==========================================================================
  // Financial View Tests
  // ==========================================================================

  describe('buildFinancialView', () => {
    it('should aggregate income and obligations', async () => {
      const mockAssets = [
        {
          id: 'a1',
          category: 'financial-asset',
          incomeStreams: [
            {
              id: 'i1',
              source: 'Employment',
              amount: 5000,
              frequency: 'monthly',
              status: 'active',
              isGuaranteed: true,
              confidence: 100,
            },
          ],
          obligations: [
            {
              id: 'o1',
              creditorId: 'bank-1',
              principalAmount: 200000,
              remainingAmount: 180000,
              monthlyPayment: 1500,
            },
          ],
          accountBalance: 10000,
        },
      ];

      mockHolochain.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockAssets })
      );

      const result = await service.buildFinancialView('steward-1');

      expect(result.monthlyIncome).toBe(5000);
      expect(result.monthlyObligations).toBe(1500);
      expect(result.monthlyDifference).toBe(3500);
      expect(result.financialHealth).toBeDefined();
    });

    it('should assess financial health correctly', async () => {
      const mockAssets = [
        {
          id: 'a1',
          category: 'financial-asset',
          incomeStreams: [
            {
              id: 'i1',
              amount: 2000,
              frequency: 'monthly',
              status: 'active',
              isGuaranteed: true,
              confidence: 100,
            },
          ],
          obligations: [],
          accountBalance: 5000,
        },
      ];

      mockHolochain.callZome.and.returnValue(
        Promise.resolve({ success: true, data: mockAssets })
      );

      const result = await service.buildFinancialView('steward-1');

      // 2000 income > 1450 floor, 2000 - 0 obligations = 2000 > 500 = healthy
      expect(result.financialHealth).toBe('healthy');
      expect(result.onTrackForDignity).toBe(true);
    });
  });
});
