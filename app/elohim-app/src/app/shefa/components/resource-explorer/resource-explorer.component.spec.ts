import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute, Router } from '@angular/router';
import { signal } from '@angular/core';

import { of } from 'rxjs';
import { vi } from 'vitest';

import { IdentityService } from '@app/imagodei/services/identity.service';
import { INITIAL_IDENTITY_STATE } from '@app/imagodei/models/identity.model';
import { HouseholdResilienceService } from '@app/lamad/services/household-resilience.service';
import { StewardshipAllocationService } from '@app/lamad/services/stewardship-allocation.service';
import { HouseholdDevicesService } from '@app/shefa/services/household-devices.service';
import { LensRegistryService } from '@app/elohim/services/lens-registry.service';
import { ResourceExplorerService } from '@app/shefa/services/resource-explorer.service';

import type { StewardshipAllocationView } from '@elohim/storage-client/generated';
import type { HouseholdResilienceView } from '@app/generated/household-resilience-view';

import { ResourceExplorerComponent } from './resource-explorer.component';

// =============================================================================
// Test Data
// =============================================================================

const MOCK_ALLOCATION = (
  overrides: Partial<StewardshipAllocationView> = {},
): StewardshipAllocationView => ({
  id: 'alloc-1',
  hAppId: 'elohim',
  contentId: 'content-abc',
  stewardPresenceId: 'steward-1',
  allocationRatio: 0.8,
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
  recognitionAccumulated: 50,
  lastRecognitionAt: null,
  note: null,
  metadata: null,
  createdAt: '2024-01-01T00:00:00Z',
  updatedAt: '2024-01-01T00:00:00Z',
  dhtAnchorHash: null,
  ...overrides,
});

const MOCK_RESILIENCE = (
  overrides: Partial<HouseholdResilienceView> = {},
): HouseholdResilienceView => ({
  contentId: 'content-abc',
  householdsStewarding: 3,
  householdsReciprocated: 1,
  protectionStatus: 'protected',
  details: { onlinePeerCount: 3 },
  ...overrides,
});

// =============================================================================
// Helpers
// =============================================================================

function buildRoute(lensType: string | null = null): Partial<ActivatedRoute> {
  return {
    snapshot: {
      paramMap: {
        get: vi.fn().mockImplementation((key: string) => {
          if (key === 'lensType') return lensType;
          return null;
        }),
      } as any,
    } as any,
  };
}

// =============================================================================
// Tests
// =============================================================================

describe('ResourceExplorerComponent — category:content lens', () => {
  let component: ResourceExplorerComponent;
  let fixture: ComponentFixture<ResourceExplorerComponent>;

  let mockIdentityService: any;
  let mockHouseholdDevices: any;
  let mockStewardshipAllocation: any;
  let mockHouseholdResilience: any;
  let mockLensRegistry: any;
  let mockRouter: any;

  beforeEach(async () => {
    mockIdentityService = {
      identity: signal({
        ...INITIAL_IDENTITY_STATE,
        humanId: 'human-matthew-manager',
      }),
    };

    mockHouseholdDevices = {
      resolveHouseholdId: vi.fn().mockReturnValue(of('household-matthew')),
      listForHuman: vi.fn().mockReturnValue(of(null)),
    };

    mockStewardshipAllocation = {
      listForHousehold: vi.fn().mockReturnValue(of([])),
    };

    mockHouseholdResilience = {
      get: vi.fn().mockReturnValue(of(MOCK_RESILIENCE())),
    };

    mockLensRegistry = {
      lensDefinitions: signal([]),
      getProvider: vi.fn().mockReturnValue(null),
    };

    mockRouter = { navigate: vi.fn().mockResolvedValue(true) };
  });

  async function setup(lensType: string | null = 'category:content'): Promise<void> {
    await TestBed.configureTestingModule({
      imports: [ResourceExplorerComponent],
      providers: [
        { provide: ActivatedRoute, useValue: buildRoute(lensType) },
        { provide: Router, useValue: mockRouter },
        { provide: IdentityService, useValue: mockIdentityService },
        { provide: HouseholdDevicesService, useValue: mockHouseholdDevices },
        { provide: StewardshipAllocationService, useValue: mockStewardshipAllocation },
        { provide: HouseholdResilienceService, useValue: mockHouseholdResilience },
        { provide: LensRegistryService, useValue: mockLensRegistry },
        { provide: ResourceExplorerService, useValue: {} },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(ResourceExplorerComponent);
    component = fixture.componentInstance;
  }

  // ---------------------------------------------------------------------------
  // category:content — items rendered
  // ---------------------------------------------------------------------------

  it('renders the stewarded list when items are returned', async () => {
    const alloc = MOCK_ALLOCATION({ contentId: 'content-abc', allocationRatio: 0.8 });
    mockStewardshipAllocation.listForHousehold.mockReturnValue(of([alloc]));
    mockHouseholdResilience.get.mockReturnValue(
      of(MOCK_RESILIENCE({ contentId: 'content-abc', protectionStatus: 'protected', householdsStewarding: 3 })),
    );

    await setup('category:content');
    fixture.detectChanges();
    // Flush the async loadStewardedContent promise chain (firstValueFrom is a Promise)
    await new Promise(resolve => setTimeout(resolve, 0));
    fixture.detectChanges();

    const list = fixture.nativeElement.querySelector('[data-testid="stewarded-content-list"]');
    expect(list).toBeTruthy();

    const item = fixture.nativeElement.querySelector('[data-testid="stewarded-content-abc"]');
    expect(item).toBeTruthy();
    expect(item.textContent).toContain('content-abc');
    expect(item.textContent).toContain('80%');
  });

  it('renders resilience badge when resilience data arrives', async () => {
    const alloc = MOCK_ALLOCATION({ contentId: 'content-abc', allocationRatio: 1.0 });
    mockStewardshipAllocation.listForHousehold.mockReturnValue(of([alloc]));
    mockHouseholdResilience.get.mockReturnValue(
      of(MOCK_RESILIENCE({ contentId: 'content-abc', protectionStatus: 'partial', householdsStewarding: 2 })),
    );

    await setup('category:content');
    fixture.detectChanges();
    await new Promise(resolve => setTimeout(resolve, 0));
    fixture.detectChanges();

    const badge = fixture.nativeElement.querySelector('[data-testid="resilience-content-abc"]');
    expect(badge).toBeTruthy();
    expect(badge.textContent).toContain('partial');
    expect(badge.textContent).toContain('2 households');
    expect(badge.classList.contains('resilience-partial')).toBe(true);
  });

  // ---------------------------------------------------------------------------
  // category:content — empty state
  // ---------------------------------------------------------------------------

  it('renders empty state when no items returned', async () => {
    mockStewardshipAllocation.listForHousehold.mockReturnValue(of([]));

    await setup('category:content');
    fixture.detectChanges();
    await new Promise(resolve => setTimeout(resolve, 0));
    fixture.detectChanges();

    const empty = fixture.nativeElement.querySelector('[data-testid="stewarded-empty"]');
    expect(empty).toBeTruthy();
    expect(empty.textContent).toContain('No stewarded content yet');
  });

  it('renders empty state when household resolution returns null', async () => {
    mockHouseholdDevices.resolveHouseholdId.mockReturnValue(of(null));

    await setup('category:content');
    fixture.detectChanges();
    await new Promise(resolve => setTimeout(resolve, 0));
    fixture.detectChanges();

    // listForHousehold should not have been called
    expect(mockStewardshipAllocation.listForHousehold).not.toHaveBeenCalled();

    // isLoading is false so stewarded-content-list renders; items is empty → empty state shown
    const empty = fixture.nativeElement.querySelector('[data-testid="stewarded-empty"]');
    expect(empty).toBeTruthy();
  });

  it('renders empty state when humanId is not set', async () => {
    mockIdentityService.identity = signal({
      ...INITIAL_IDENTITY_STATE,
      humanId: null,
    });

    await setup('category:content');
    fixture.detectChanges();
    await new Promise(resolve => setTimeout(resolve, 0));
    fixture.detectChanges();

    expect(mockHouseholdDevices.resolveHouseholdId).not.toHaveBeenCalled();
    const empty = fixture.nativeElement.querySelector('[data-testid="stewarded-empty"]');
    expect(empty).toBeTruthy();
  });

  // ---------------------------------------------------------------------------
  // category:content — service interaction
  // ---------------------------------------------------------------------------

  it('calls listForHousehold with resolved householdId', async () => {
    mockHouseholdDevices.resolveHouseholdId.mockReturnValue(of('household-matthew'));
    mockStewardshipAllocation.listForHousehold.mockReturnValue(of([]));

    await setup('category:content');
    fixture.detectChanges();
    await new Promise(resolve => setTimeout(resolve, 0));

    expect(mockHouseholdDevices.resolveHouseholdId).toHaveBeenCalledWith('human-matthew-manager');
    expect(mockStewardshipAllocation.listForHousehold).toHaveBeenCalledWith('household-matthew');
  });

  it('calls HouseholdResilienceService.get for each item with householdId as viewerHouseholdId', async () => {
    const alloc1 = MOCK_ALLOCATION({ contentId: 'c1' });
    const alloc2 = MOCK_ALLOCATION({ contentId: 'c2' });
    mockStewardshipAllocation.listForHousehold.mockReturnValue(of([alloc1, alloc2]));
    mockHouseholdResilience.get.mockReturnValue(of(MOCK_RESILIENCE()));

    await setup('category:content');
    fixture.detectChanges();
    await new Promise(resolve => setTimeout(resolve, 0));

    expect(mockHouseholdResilience.get).toHaveBeenCalledWith('c1', 'household-matthew');
    expect(mockHouseholdResilience.get).toHaveBeenCalledWith('c2', 'household-matthew');
  });

  // ---------------------------------------------------------------------------
  // Non-category:content lens — does NOT trigger stewarded path
  // ---------------------------------------------------------------------------

  it('does not call listForHousehold when lensType is folders', async () => {
    await setup('folders');
    fixture.detectChanges();
    await fixture.whenStable();

    expect(mockStewardshipAllocation.listForHousehold).not.toHaveBeenCalled();
    const list = fixture.nativeElement.querySelector('[data-testid="stewarded-content-list"]');
    expect(list).toBeNull();
  });
});
