/**
 * StewardshipDashboardComponent Tests
 *
 * Tests for content stewardship dashboard.
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { StewardshipDashboardComponent } from './stewardship-dashboard.component';
import { IdentityService } from '../../services/identity.service';
import { PresenceService } from '../../services/presence.service';
import { StewardshipAllocationService } from '@app/lamad/services/stewardship-allocation.service';
import { signal } from '@angular/core';

describe('StewardshipDashboardComponent', () => {
  let component: StewardshipDashboardComponent;
  let fixture: ComponentFixture<StewardshipDashboardComponent>;
  let mockIdentityService: jasmine.SpyObj<IdentityService>;
  let mockPresenceService: jasmine.SpyObj<PresenceService>;
  let mockStewardshipService: jasmine.SpyObj<StewardshipAllocationService>;

  beforeEach(async () => {
    // Create mocks
    mockIdentityService = jasmine.createSpyObj(
      'IdentityService',
      [],
      {
        profile: signal(null),
        humanId: signal(null),
      }
    );

    mockPresenceService = jasmine.createSpyObj('PresenceService', ['loadPresences']);

    mockStewardshipService = jasmine.createSpyObj(
      'StewardshipAllocationService',
      ['getStewardPortfolio'],
      {}
    );

    await TestBed.configureTestingModule({
      imports: [StewardshipDashboardComponent],
      providers: [
        { provide: IdentityService, useValue: mockIdentityService },
        { provide: PresenceService, useValue: mockPresenceService },
        { provide: StewardshipAllocationService, useValue: mockStewardshipService },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(StewardshipDashboardComponent);
    component = fixture.componentInstance;
  });

  // ==========================================================================
  // Component Creation
  // ==========================================================================

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  // ==========================================================================
  // Signals
  // ==========================================================================

  it('should have isLoading signal', () => {
    expect(component.isLoading).toBeDefined();
  });

  it('should have error signal', () => {
    expect(component.error).toBeDefined();
  });

  it('should have portfolio signal', () => {
    expect(component.portfolio).toBeDefined();
  });

  it('should have allocations signal', () => {
    expect(component.allocations).toBeDefined();
  });

  // ==========================================================================
  // Computed Signals
  // ==========================================================================

  it('should have profile computed signal', () => {
    expect(component.profile).toBeDefined();
  });

  it('should have presenceId computed signal', () => {
    expect(component.presenceId).toBeDefined();
  });

  it('should have totalRecognition computed signal', () => {
    expect(component.totalRecognition).toBeDefined();
  });

  it('should have contentCount computed signal', () => {
    expect(component.contentCount).toBeDefined();
  });

  it('should have disputeCount computed signal', () => {
    expect(component.disputeCount).toBeDefined();
  });

  it('should have hasAllocations computed signal', () => {
    expect(component.hasAllocations).toBeDefined();
  });

  // ==========================================================================
  // Public Methods
  // ==========================================================================

  it('should have loadPortfolio method', () => {
    expect(component.loadPortfolio).toBeDefined();
    expect(typeof component.loadPortfolio).toBe('function');
  });

  it('should have refresh method', () => {
    expect(component.refresh).toBeDefined();
    expect(typeof component.refresh).toBe('function');
  });

  it('should have formatRatio method', () => {
    expect(component.formatRatio).toBeDefined();
    expect(typeof component.formatRatio).toBe('function');
  });

  it('should have formatRecognition method', () => {
    expect(component.formatRecognition).toBeDefined();
    expect(typeof component.formatRecognition).toBe('function');
  });

  // ==========================================================================
  // Initial State
  // ==========================================================================

  it('should initialize with loading true', () => {
    expect(component.isLoading()).toBe(true);
  });

  it('should initialize with no error', () => {
    expect(component.error()).toBeNull();
  });

  it('should initialize with no portfolio', () => {
    expect(component.portfolio()).toBeNull();
  });

  it('should initialize with empty allocations', () => {
    expect(component.allocations()).toEqual([]);
  });

  // ==========================================================================
  // Computed Values - No Portfolio
  // ==========================================================================

  it('should have zero recognition with no portfolio', () => {
    expect(component.totalRecognition()).toBe(0);
  });

  it('should have zero content count with no portfolio', () => {
    expect(component.contentCount()).toBe(0);
  });

  it('should have zero dispute count with no portfolio', () => {
    expect(component.disputeCount()).toBe(0);
  });

  it('should not have allocations when array is empty', () => {
    expect(component.hasAllocations()).toBe(false);
  });

  // ==========================================================================
  // Lifecycle
  // ==========================================================================

  it('should implement OnInit', () => {
    expect(component.ngOnInit).toBeDefined();
    expect(typeof component.ngOnInit).toBe('function');
  });

  // ==========================================================================
  // Data Loading
  // ==========================================================================

  describe('Portfolio Loading', () => {
    it('should set error when no presence ID', () => {
      Object.defineProperty(mockIdentityService, 'humanId', {
        get: () => signal(null),
        configurable: true
      });

      component.loadPortfolio();

      expect(component.error()).toContain('No steward presence');
      expect(component.isLoading()).toBe(false);
    });

    it('should call service with presence ID', () => {
      const presenceId = 'human-123';
      Object.defineProperty(mockIdentityService, 'humanId', {
        get: () => signal(presenceId),
        configurable: true
      });
      mockStewardshipService.getStewardPortfolio.and.returnValue({
        subscribe: jasmine.createSpy('subscribe')
      } as any);

      component.loadPortfolio();

      expect(mockStewardshipService.getStewardPortfolio).toHaveBeenCalledWith(presenceId);
    });

    it('should handle successful portfolio load', (done) => {
      const presenceId = 'human-123';
      const mockPortfolio = {
        stewardPresenceId: presenceId,
        totalRecognition: 1000,
        contentCount: 5,
        activeDisputeCount: 2,
        allocations: []
      };

      Object.defineProperty(mockIdentityService, 'humanId', {
        get: () => signal(presenceId),
        configurable: true
      });
      mockStewardshipService.getStewardPortfolio.and.returnValue({
        subscribe: (callbacks: any) => {
          callbacks.next(mockPortfolio);

          expect(component.portfolio()).toEqual(mockPortfolio);
          expect(component.isLoading()).toBe(false);
          done();

          return { unsubscribe: () => {} };
        }
      } as any);

      component.loadPortfolio();
    });

    it('should handle portfolio load error', (done) => {
      const presenceId = 'human-123';
      Object.defineProperty(mockIdentityService, 'humanId', {
        get: () => signal(presenceId),
        configurable: true
      });
      mockStewardshipService.getStewardPortfolio.and.returnValue({
        subscribe: (callbacks: any) => {
          callbacks.error(new Error('Network error'));

          expect(component.error()).toContain('Failed to load stewardship portfolio');
          expect(component.isLoading()).toBe(false);
          done();

          return { unsubscribe: () => {} };
        }
      } as any);

      component.loadPortfolio();
    });
  });

  // ==========================================================================
  // Refresh Functionality
  // ==========================================================================

  describe('Refresh', () => {
    it('should call loadPortfolio', () => {
      spyOn(component, 'loadPortfolio');

      component.refresh();

      expect(component.loadPortfolio).toHaveBeenCalled();
    });
  });

  // ==========================================================================
  // Display Formatting
  // ==========================================================================

  describe('Format Ratio', () => {
    it('should format ratio as percentage', () => {
      expect(component.formatRatio(0.5)).toBe('50%');
      expect(component.formatRatio(0.75)).toBe('75%');
      expect(component.formatRatio(1)).toBe('100%');
    });

    it('should round percentages', () => {
      expect(component.formatRatio(0.333)).toBe('33%');
      expect(component.formatRatio(0.666)).toBe('67%');
    });

    it('should handle zero', () => {
      expect(component.formatRatio(0)).toBe('0%');
    });
  });

  describe('Format Recognition', () => {
    it('should format small values as-is', () => {
      expect(component.formatRecognition(0)).toBe('0');
      expect(component.formatRecognition(50)).toBe('50');
      expect(component.formatRecognition(999)).toBe('999');
    });

    it('should format thousands with K', () => {
      expect(component.formatRecognition(1000)).toBe('1.0K');
      expect(component.formatRecognition(5500)).toBe('5.5K');
      expect(component.formatRecognition(999000)).toBe('999.0K');
    });

    it('should format millions with M', () => {
      expect(component.formatRecognition(1000000)).toBe('1.0M');
      expect(component.formatRecognition(2500000)).toBe('2.5M');
      expect(component.formatRecognition(10000000)).toBe('10.0M');
    });
  });

  // ==========================================================================
  // Error Handling
  // ==========================================================================

  describe('Clear Error', () => {
    it('should clear error message', () => {
      component.error.set('Test error');

      component.clearError();

      expect(component.error()).toBeNull();
    });
  });

  // ==========================================================================
  // Integration Tests
  // ==========================================================================

  describe('Portfolio Display', () => {
    it('should transform allocations for display', (done) => {
      const presenceId = 'human-123';
      const mockAllocation = {
        id: 'alloc-1',
        contentId: 'test-content-id',
        stewardPresenceId: presenceId,
        allocationRatio: 0.5,
        allocationMethod: 'computed' as const,
        contributionType: 'author' as const,
        contributionEvidenceJson: null,
        governanceState: 'active' as const,
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
        lastRecognitionAt: null,
        note: null,
        metadataJson: null,
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-01T00:00:00Z'
      };
      const mockPortfolio = {
        stewardPresenceId: presenceId,
        totalRecognition: 1000,
        contentCount: 1,
        activeDisputeCount: 0,
        allocations: [mockAllocation]
      };

      Object.defineProperty(mockIdentityService, 'humanId', {
        get: () => signal(presenceId),
        configurable: true
      });
      mockStewardshipService.getStewardPortfolio.and.returnValue({
        subscribe: (callbacks: any) => {
          callbacks.next(mockPortfolio);

          const allocations = component.allocations();
          expect(allocations.length).toBe(1);
          expect(allocations[0].allocation).toEqual(mockAllocation);
          expect(allocations[0].contentTitle).toBeDefined();
          expect(allocations[0].stateLabel).toBe('Active');
          expect(allocations[0].stateColor).toBe('green');
          done();

          return { unsubscribe: () => {} };
        }
      } as any);

      component.loadPortfolio();
    });

    it('should handle multiple allocations', (done) => {
      const presenceId = 'human-123';
      const mockAllocations = [
        {
          contentId: 'content-1',
          stewardId: presenceId,
          ratio: 0.5,
          governanceState: 'active' as const,
          totalRecognition: 100,
          createdAt: '2024-01-01T00:00:00Z',
          updatedAt: '2024-01-01T00:00:00Z'
        },
        {
          contentId: 'content-2',
          stewardId: presenceId,
          ratio: 0.3,
          governanceState: 'disputed' as const,
          totalRecognition: 50,
          createdAt: '2024-01-01T00:00:00Z',
          updatedAt: '2024-01-01T00:00:00Z'
        }
      ];
      const mockPortfolio = {
        stewardId: presenceId,
        totalRecognition: 150,
        contentCount: 2,
        activeDisputeCount: 1,
        allocations: mockAllocations
      };

      Object.defineProperty(mockIdentityService, 'humanId', {
        get: () => signal(presenceId),
        configurable: true
      });
      mockStewardshipService.getStewardPortfolio.and.returnValue({
        subscribe: (callbacks: any) => {
          callbacks.next(mockPortfolio);

          const allocations = component.allocations();
          expect(allocations.length).toBe(2);
          expect(allocations[0].stateLabel).toBe('Active');
          expect(allocations[1].stateLabel).toBe('Disputed');
          expect(allocations[1].stateColor).toBe('orange');
          done();

          return { unsubscribe: () => {} };
        }
      } as any);

      component.loadPortfolio();
    });
  });

  describe('Governance State Formatting', () => {
    it('should format active state', (done) => {
      const presenceId = 'human-123';
      const mockAllocation = {
        contentId: 'test-content',
        stewardId: presenceId,
        ratio: 1,
        governanceState: 'active' as const,
        totalRecognition: 100,
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-01T00:00:00Z'
      };

      Object.defineProperty(mockIdentityService, 'humanId', {
        get: () => signal(presenceId),
        configurable: true
      });
      mockStewardshipService.getStewardPortfolio.and.returnValue({
        subscribe: (callbacks: any) => {
          callbacks.next({
            stewardId: presenceId,
            totalRecognition: 100,
            contentCount: 1,
            activeDisputeCount: 0,
            allocations: [mockAllocation]
          });

          const allocation = component.allocations()[0];
          expect(allocation.stateLabel).toBe('Active');
          expect(allocation.stateColor).toBe('green');
          done();

          return { unsubscribe: () => {} };
        }
      } as any);

      component.loadPortfolio();
    });

    it('should format disputed state', (done) => {
      const presenceId = 'human-123';
      const mockAllocation = {
        contentId: 'test-content',
        stewardId: presenceId,
        ratio: 0.5,
        governanceState: 'disputed' as const,
        totalRecognition: 50,
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-01T00:00:00Z'
      };

      Object.defineProperty(mockIdentityService, 'humanId', {
        get: () => signal(presenceId),
        configurable: true
      });
      mockStewardshipService.getStewardPortfolio.and.returnValue({
        subscribe: (callbacks: any) => {
          callbacks.next({
            stewardId: presenceId,
            totalRecognition: 50,
            contentCount: 1,
            activeDisputeCount: 1,
            allocations: [mockAllocation]
          });

          const allocation = component.allocations()[0];
          expect(allocation.stateLabel).toBe('Disputed');
          expect(allocation.stateColor).toBe('orange');
          done();

          return { unsubscribe: () => {} };
        }
      } as any);

      component.loadPortfolio();
    });

    it('should format pending_review state', (done) => {
      const presenceId = 'human-123';
      const mockAllocation = {
        contentId: 'test-content',
        stewardId: presenceId,
        ratio: 0.7,
        governanceState: 'pending_review' as const,
        totalRecognition: 70,
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-01T00:00:00Z'
      };

      Object.defineProperty(mockIdentityService, 'humanId', {
        get: () => signal(presenceId),
        configurable: true
      });
      mockStewardshipService.getStewardPortfolio.and.returnValue({
        subscribe: (callbacks: any) => {
          callbacks.next({
            stewardId: presenceId,
            totalRecognition: 70,
            contentCount: 1,
            activeDisputeCount: 0,
            allocations: [mockAllocation]
          });

          const allocation = component.allocations()[0];
          expect(allocation.stateLabel).toBe('Pending Review');
          expect(allocation.stateColor).toBe('blue');
          done();

          return { unsubscribe: () => {} };
        }
      } as any);

      component.loadPortfolio();
    });

    it('should format superseded state', (done) => {
      const presenceId = 'human-123';
      const mockAllocation = {
        contentId: 'test-content',
        stewardId: presenceId,
        ratio: 0,
        governanceState: 'superseded' as const,
        totalRecognition: 0,
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-01T00:00:00Z'
      };

      Object.defineProperty(mockIdentityService, 'humanId', {
        get: () => signal(presenceId),
        configurable: true
      });
      mockStewardshipService.getStewardPortfolio.and.returnValue({
        subscribe: (callbacks: any) => {
          callbacks.next({
            stewardId: presenceId,
            totalRecognition: 0,
            contentCount: 1,
            activeDisputeCount: 0,
            allocations: [mockAllocation]
          });

          const allocation = component.allocations()[0];
          expect(allocation.stateLabel).toBe('Superseded');
          expect(allocation.stateColor).toBe('gray');
          done();

          return { unsubscribe: () => {} };
        }
      } as any);

      component.loadPortfolio();
    });
  });

  describe('Content ID Formatting', () => {
    it('should format kebab-case to Title Case', (done) => {
      const presenceId = 'human-123';
      const mockAllocation = {
        contentId: 'quiz-manifesto-foundations',
        stewardId: presenceId,
        ratio: 1,
        governanceState: 'active' as const,
        totalRecognition: 100,
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-01T00:00:00Z'
      };

      Object.defineProperty(mockIdentityService, 'humanId', {
        get: () => signal(presenceId),
        configurable: true
      });
      mockStewardshipService.getStewardPortfolio.and.returnValue({
        subscribe: (callbacks: any) => {
          callbacks.next({
            stewardId: presenceId,
            totalRecognition: 100,
            contentCount: 1,
            activeDisputeCount: 0,
            allocations: [mockAllocation]
          });

          const allocation = component.allocations()[0];
          expect(allocation.contentTitle).toBe('Quiz Manifesto Foundations');
          done();

          return { unsubscribe: () => {} };
        }
      } as any);

      component.loadPortfolio();
    });

    it('should handle single word content IDs', (done) => {
      const presenceId = 'human-123';
      const mockAllocation = {
        contentId: 'introduction',
        stewardId: presenceId,
        ratio: 1,
        governanceState: 'active' as const,
        totalRecognition: 50,
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-01T00:00:00Z'
      };

      Object.defineProperty(mockIdentityService, 'humanId', {
        get: () => signal(presenceId),
        configurable: true
      });
      mockStewardshipService.getStewardPortfolio.and.returnValue({
        subscribe: (callbacks: any) => {
          callbacks.next({
            stewardId: presenceId,
            totalRecognition: 50,
            contentCount: 1,
            activeDisputeCount: 0,
            allocations: [mockAllocation]
          });

          const allocation = component.allocations()[0];
          expect(allocation.contentTitle).toBe('Introduction');
          done();

          return { unsubscribe: () => {} };
        }
      } as any);

      component.loadPortfolio();
    });
  });

  describe('Edge Cases', () => {
    it('should handle empty portfolio', (done) => {
      const presenceId = 'human-123';
      const mockPortfolio = {
        stewardId: presenceId,
        totalRecognition: 0,
        contentCount: 0,
        activeDisputeCount: 0,
        allocations: []
      };

      Object.defineProperty(mockIdentityService, 'humanId', {
        get: () => signal(presenceId),
        configurable: true
      });
      mockStewardshipService.getStewardPortfolio.and.returnValue({
        subscribe: (callbacks: any) => {
          callbacks.next(mockPortfolio);

          expect(component.totalRecognition()).toBe(0);
          expect(component.contentCount()).toBe(0);
          expect(component.disputeCount()).toBe(0);
          expect(component.hasAllocations()).toBe(false);
          done();

          return { unsubscribe: () => {} };
        }
      } as any);

      component.loadPortfolio();
    });

    it('should handle very large recognition values', () => {
      expect(component.formatRecognition(1234567890)).toBe('1234.6M');
    });

    it('should handle negative ratios gracefully', () => {
      expect(component.formatRatio(-0.5)).toBe('-50%');
    });

    it('should handle ratios over 100%', () => {
      expect(component.formatRatio(1.5)).toBe('150%');
    });
  });
});
