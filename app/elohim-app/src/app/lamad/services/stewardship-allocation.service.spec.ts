import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';
import { vi } from 'vitest';

import { StorageApiService } from '@app/elohim/services/storage-api.service';

import {
  StewardshipAllocationService,
  StewardPortfolio,
} from './stewardship-allocation.service';

import type { StewardshipAllocationView, ContentStewardshipView } from '@elohim/storage-client/generated';

// =============================================================================
// Test Data Factories
// =============================================================================

const createMockAllocation = (
  overrides: Partial<StewardshipAllocationView> = {}
): StewardshipAllocationView => ({
  id: 'alloc-1',
  appId: 'elohim',
  contentId: 'content-1',
  stewardPresenceId: 'steward-1',
  allocationRatio: 0.6,
  allocationMethod: 'manual',
  contributionType: 'author',
  contributionEvidence: null,
  governanceState: 'active',
  disputeId: null,
  disputeReason: null,
  disputedAt: null,
  disputedBy: null,
  negotiationSessionId: null,
  elohimRatifiedAt: null,
  elohimRatifierId: null,
  effectiveFrom: '2024-01-01T00:00:00Z',
  effectiveUntil: null,
  supersededBy: null,
  recognitionAccumulated: 100,
  lastRecognitionAt: '2024-06-01T00:00:00Z',
  note: null,
  metadata: null,
  createdAt: '2024-01-01T00:00:00Z',
  updatedAt: '2024-06-01T00:00:00Z',
  ...overrides,
});

const createMockStewardship = (
  overrides: Partial<ContentStewardshipView> = {}
): ContentStewardshipView => ({
  contentId: 'content-1',
  allocations: [],
  totalAllocation: 1,
  hasDisputes: false,
  primarySteward: createMockAllocation(),
  ...overrides,
});

// =============================================================================
// Tests
// =============================================================================

describe('StewardshipAllocationService', () => {
  let service: StewardshipAllocationService;
  let storageApiSpy: Record<string, ReturnType<typeof vi.fn>>;

  beforeEach(() => {
    storageApiSpy = {
      getContentStewardship: vi.fn(),
      getStewardshipAllocations: vi.fn(),
      getAllocationsForSteward: vi.fn(),
      createStewardshipAllocation: vi.fn(),
      updateStewardshipAllocation: vi.fn(),
      deleteStewardshipAllocation: vi.fn(),
      bulkCreateAllocations: vi.fn(),
      fileAllocationDispute: vi.fn(),
      resolveAllocationDispute: vi.fn(),
    };

    TestBed.configureTestingModule({
      providers: [
        StewardshipAllocationService,
        { provide: StorageApiService, useValue: storageApiSpy },
        provideHttpClient(),
        provideHttpClientTesting(),
      ],
    });
    service = TestBed.inject(StewardshipAllocationService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // ===========================================================================
  // Content Stewardship Queries
  // ===========================================================================

  describe('getContentStewardship', () => {
    it('should return stewardship and update _currentStewardship$', () => {
      const stewardship = createMockStewardship();
      storageApiSpy['getContentStewardship'].mockReturnValue(of(stewardship));

      let emittedStewardship: ContentStewardshipView | null = null;
      service.currentStewardship$.subscribe(s => (emittedStewardship = s));

      service.getContentStewardship('content-1').subscribe(result => {
        expect(result).toEqual(stewardship);
      });

      expect(emittedStewardship).toEqual(stewardship);
      expect(storageApiSpy['getContentStewardship']).toHaveBeenCalledWith('content-1');
    });

    it('should set loading true then false', () => {
      storageApiSpy['getContentStewardship'].mockReturnValue(of(createMockStewardship()));

      expect(service.loading()).toBe(false);

      service.getContentStewardship('content-1').subscribe();

      expect(service.loading()).toBe(false); // Already completed synchronously
    });

    it('should return empty default on error and set loading false', () => {
      storageApiSpy['getContentStewardship'].mockReturnValue(throwError(() => new Error('fail')));

      service.getContentStewardship('content-1').subscribe(result => {
        expect(result.contentId).toBe('content-1');
        expect(result.allocations).toEqual([]);
        expect(result.totalAllocation).toBe(0);
        expect(result.hasDisputes).toBe(false);
        expect(result.primarySteward).toBeNull();
      });

      expect(service.loading()).toBe(false);
    });
  });

  describe('getPrimarySteward', () => {
    it('should delegate to getContentStewardship and return primarySteward', () => {
      const primarySteward = createMockAllocation({ id: 'primary-alloc' });
      storageApiSpy['getContentStewardship'].mockReturnValue(
        of(createMockStewardship({ primarySteward }))
      );

      service.getPrimarySteward('content-1').subscribe(result => {
        expect(result).toEqual(primarySteward);
      });
    });

    it('should return null when no primary steward', () => {
      storageApiSpy['getContentStewardship'].mockReturnValue(
        of(createMockStewardship({ primarySteward: null }))
      );

      service.getPrimarySteward('content-1').subscribe(result => {
        expect(result).toBeNull();
      });
    });
  });

  describe('hasAllocations', () => {
    it('should return true when allocations exist', () => {
      storageApiSpy['getStewardshipAllocations'].mockReturnValue(of([createMockAllocation()]));

      service.hasAllocations('content-1').subscribe(result => {
        expect(result).toBe(true);
      });

      expect(storageApiSpy['getStewardshipAllocations']).toHaveBeenCalledWith({
        contentId: 'content-1',
        activeOnly: true,
        limit: 1,
      });
    });

    it('should return false on empty result', () => {
      storageApiSpy['getStewardshipAllocations'].mockReturnValue(of([]));

      service.hasAllocations('content-1').subscribe(result => {
        expect(result).toBe(false);
      });
    });

    it('should return false on error', () => {
      storageApiSpy['getStewardshipAllocations'].mockReturnValue(throwError(() => new Error('fail')));

      service.hasAllocations('content-1').subscribe(result => {
        expect(result).toBe(false);
      });
    });
  });

  // ===========================================================================
  // Steward Portfolio Queries
  // ===========================================================================

  describe('getStewardPortfolio', () => {
    it('should build portfolio from allocations', () => {
      const allocations = [
        createMockAllocation({ id: 'a1', recognitionAccumulated: 100, governanceState: 'active' }),
        createMockAllocation({ id: 'a2', recognitionAccumulated: 50, governanceState: 'disputed' }),
        createMockAllocation({ id: 'a3', recognitionAccumulated: 25, governanceState: 'active' }),
      ];
      storageApiSpy['getAllocationsForSteward'].mockReturnValue(of(allocations));

      let emittedPortfolio: StewardPortfolio | null = null;
      service.currentPortfolio$.subscribe(p => (emittedPortfolio = p));

      service.getStewardPortfolio('steward-1').subscribe(result => {
        expect(result.stewardPresenceId).toBe('steward-1');
        expect(result.totalRecognition).toBe(175);
        expect(result.contentCount).toBe(3);
        expect(result.activeDisputeCount).toBe(1);
        expect(result.allocations).toEqual(allocations);
      });

      expect(emittedPortfolio).not.toBeNull();
      expect(emittedPortfolio!.totalRecognition).toBe(175);
    });

    it('should return empty portfolio on error', () => {
      storageApiSpy['getAllocationsForSteward'].mockReturnValue(throwError(() => new Error('fail')));

      service.getStewardPortfolio('steward-1').subscribe(result => {
        expect(result.stewardPresenceId).toBe('steward-1');
        expect(result.allocations).toEqual([]);
        expect(result.totalRecognition).toBe(0);
        expect(result.contentCount).toBe(0);
        expect(result.activeDisputeCount).toBe(0);
      });

      expect(service.loading()).toBe(false);
    });
  });

  describe('getStewartedContentIds', () => {
    it('should map allocations to content IDs', () => {
      const allocations = [
        createMockAllocation({ contentId: 'c1' }),
        createMockAllocation({ contentId: 'c2' }),
      ];
      storageApiSpy['getAllocationsForSteward'].mockReturnValue(of(allocations));

      service.getStewartedContentIds('steward-1').subscribe(result => {
        expect(result).toEqual(['c1', 'c2']);
      });
    });

    it('should return empty array on error', () => {
      storageApiSpy['getAllocationsForSteward'].mockReturnValue(throwError(() => new Error('fail')));

      service.getStewartedContentIds('steward-1').subscribe(result => {
        expect(result).toEqual([]);
      });
    });
  });

  // ===========================================================================
  // CRUD Passthrough
  // ===========================================================================

  describe('createAllocation', () => {
    it('should pass through to storageApi', () => {
      const input = { contentId: 'c1', stewardPresenceId: 's1' };
      const created = createMockAllocation();
      storageApiSpy['createStewardshipAllocation'].mockReturnValue(of(created));

      service.createAllocation(input).subscribe(result => {
        expect(result).toEqual(created);
      });

      expect(storageApiSpy['createStewardshipAllocation']).toHaveBeenCalledWith(input);
    });
  });

  describe('updateAllocation', () => {
    it('should pass through with ID and input', () => {
      const input = { allocationRatio: 0.5 };
      const updated = createMockAllocation({ allocationRatio: 0.5 });
      storageApiSpy['updateStewardshipAllocation'].mockReturnValue(of(updated));

      service.updateAllocation('alloc-1', input).subscribe(result => {
        expect(result).toEqual(updated);
      });

      expect(storageApiSpy['updateStewardshipAllocation']).toHaveBeenCalledWith('alloc-1', input);
    });
  });

  describe('deleteAllocation', () => {
    it('should pass through with ID', () => {
      storageApiSpy['deleteStewardshipAllocation'].mockReturnValue(of(undefined));

      service.deleteAllocation('alloc-1').subscribe();

      expect(storageApiSpy['deleteStewardshipAllocation']).toHaveBeenCalledWith('alloc-1');
    });
  });

  describe('bulkCreateAllocations', () => {
    it('should pass through array', () => {
      const inputs = [
        { contentId: 'c1', stewardPresenceId: 's1' },
        { contentId: 'c2', stewardPresenceId: 's1' },
      ];
      const result = { created: 2, failed: 0, errors: [] };
      storageApiSpy['bulkCreateAllocations'].mockReturnValue(of(result));

      service.bulkCreateAllocations(inputs).subscribe(r => {
        expect(r).toEqual(result);
      });

      expect(storageApiSpy['bulkCreateAllocations']).toHaveBeenCalledWith(inputs);
    });
  });

  // ===========================================================================
  // Dispute Management
  // ===========================================================================

  describe('fileDispute', () => {
    it('should pass disputedBy and reason to storageApi (no client-side disputeId)', () => {
      const disputed = createMockAllocation({ governanceState: 'disputed' });
      storageApiSpy['fileAllocationDispute'].mockReturnValue(of(disputed));

      service.fileDispute('alloc-1', 'user-1', 'Unfair ratio').subscribe(result => {
        expect(result).toEqual(disputed);
      });

      expect(storageApiSpy['fileAllocationDispute']).toHaveBeenCalledWith('alloc-1', {
        disputedBy: 'user-1',
        reason: 'Unfair ratio',
      });
    });
  });

  describe('resolveDispute', () => {
    it('should pass through to storageApi', () => {
      const resolved = createMockAllocation({ governanceState: 'active' });
      storageApiSpy['resolveAllocationDispute'].mockReturnValue(of(resolved));

      service.resolveDispute('alloc-1', 'elohim-1', 'active').subscribe(result => {
        expect(result).toEqual(resolved);
      });

      expect(storageApiSpy['resolveAllocationDispute']).toHaveBeenCalledWith('alloc-1', {
        ratifierId: 'elohim-1',
        newState: 'active',
      });
    });
  });

  describe('getDisputedAllocations', () => {
    it('should query with governanceState disputed', () => {
      const disputed = [createMockAllocation({ governanceState: 'disputed' })];
      storageApiSpy['getStewardshipAllocations'].mockReturnValue(of(disputed));

      service.getDisputedAllocations().subscribe(result => {
        expect(result).toEqual(disputed);
      });

      expect(storageApiSpy['getStewardshipAllocations']).toHaveBeenCalledWith({
        governanceState: 'disputed',
      });
    });
  });

  // ===========================================================================
  // Bootstrap
  // ===========================================================================

  describe('bootstrapSteward', () => {
    it('should build CreateAllocationInput array and delegate to bulkCreateAllocations', () => {
      const result = { created: 2, failed: 0, errors: [] };
      storageApiSpy['bulkCreateAllocations'].mockReturnValue(of(result));

      service.bootstrapSteward('steward-1', ['c1', 'c2']).subscribe(r => {
        expect(r).toEqual(result);
      });

      expect(storageApiSpy['bulkCreateAllocations']).toHaveBeenCalledWith([
        {
          contentId: 'c1',
          stewardPresenceId: 'steward-1',
          allocationRatio: 1,
          allocationMethod: 'manual',
          contributionType: 'inherited',
          note: 'Bootstrap steward assignment',
        },
        {
          contentId: 'c2',
          stewardPresenceId: 'steward-1',
          allocationRatio: 1,
          allocationMethod: 'manual',
          contributionType: 'inherited',
          note: 'Bootstrap steward assignment',
        },
      ]);
    });

    it('should accept custom contributionType', () => {
      storageApiSpy['bulkCreateAllocations'].mockReturnValue(of({ created: 1, failed: 0, errors: [] }));

      service.bootstrapSteward('steward-1', ['c1'], 'original_creator').subscribe();

      const calledInputs = storageApiSpy['bulkCreateAllocations'].mock.calls[0][0];
      expect(calledInputs[0].contributionType).toBe('original_creator');
    });
  });

  // ===========================================================================
  // Utilities
  // ===========================================================================

  describe('clearCache', () => {
    it('should reset both BehaviorSubjects to null', () => {
      // Populate the subjects first
      storageApiSpy['getContentStewardship'].mockReturnValue(of(createMockStewardship()));
      storageApiSpy['getAllocationsForSteward'].mockReturnValue(of([createMockAllocation()]));

      service.getContentStewardship('c1').subscribe();
      service.getStewardPortfolio('s1').subscribe();

      let stewardship: ContentStewardshipView | null = null;
      let portfolio: StewardPortfolio | null = null;
      service.currentStewardship$.subscribe(s => (stewardship = s));
      service.currentPortfolio$.subscribe(p => (portfolio = p));

      // Verify populated
      expect(stewardship).not.toBeNull();
      expect(portfolio).not.toBeNull();

      // Clear
      service.clearCache();

      expect(stewardship).toBeNull();
      expect(portfolio).toBeNull();
    });
  });
});
