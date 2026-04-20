/**
 * NetworkHealthTabComponent Tests
 *
 * Coverage focus:
 * - Component creation
 * - Renders the network-posture-card testid
 * - Shows live metrics when service returns posture
 * - Shows unavailable state when service returns null
 * - data-testids present for all expected metrics
 * - formatPressure and pressureClass helpers
 * - Household-grouped breakdown renders household rows
 * - Household rows show peer count and commitment count
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { NO_ERRORS_SCHEMA } from '@angular/core';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';
import { of } from 'rxjs';

import { NetworkHealthTabComponent } from './network-health-tab.component';
import {
  NetworkPostureService,
  type NetworkPostureView,
} from '../../services/network-posture.service';
import { HouseholdDevicesService } from '@app/shefa/services/household-devices.service';
import { CollectiveService } from '@app/qahal/services/collective.service';
import type { HouseholdDevicesView } from '@app/generated/household-devices-view';
import type { CollectiveView } from '@elohim/storage-client/generated';

const MOCK_POSTURE: NetworkPostureView = {
  totalPeers: 50,
  activePeers: 40,
  stalePeers: 10,
  alwaysOnPeers: 15,
  householdsReciprocating: 8,
  computeAvailable: true,
  storagePressure: 0.45,
  computedAt: '2026-04-19T10:00:00.000Z',
};

const MOCK_COLLECTIVES: CollectiveView[] = [
  {
    id: 'household-matthew',
    name: 'Matthew Household',
    description: null,
    governanceLayer: 'household',
    constitutionalParentId: null,
    reach: 'local',
    metadata: null,
    createdBy: null,
    createdAt: '2026-01-01T00:00:00.000Z',
    updatedAt: '2026-01-01T00:00:00.000Z',
    dissolvedAt: null,
  },
];

const MOCK_HOUSEHOLD_DEVICES: HouseholdDevicesView = {
  householdId: 'household-matthew',
  devices: [
    {
      shape: {
        nodeId: 'uhCAkABC123',
        hostname: 'matthew-nuc',
        deviceArchetypeId: 'home-nuc',
        householdId: 'household-matthew',
        role: 'edge',
        capabilityLevel: 3,
        committed: {
          cpuCores: 8,
          memoryGb: 16,
          storageTb: 2,
        },
        signature: 'sig',
        signedAt: '2026-01-01T00:00:00.000Z',
      },
      peer: {
        peerId: 'uhCAkABC123',
        status: 'online',
        generalPoolMember: true,
        acceptingStewardshipReserves: true,
        archetypeClass: 'home-nuc',
        timestamp: '1700000000000000',
        dhtAnchorHash: 'hash123',
        updatedAt: '1700000000000000',
      },
    },
  ],
};

describe('NetworkHealthTabComponent', () => {
  let component: NetworkHealthTabComponent;
  let fixture: ComponentFixture<NetworkHealthTabComponent>;
  let httpMock: HttpTestingController;
  let networkPostureMock: { get: () => ReturnType<NetworkPostureService['get']> };
  let collectiveMock: { listCollectives: () => ReturnType<CollectiveService['listCollectives']> };
  let householdDevicesMock: { list: (id: string) => ReturnType<HouseholdDevicesService['list']> };

  function buildFixture(): Promise<void> {
    return TestBed.configureTestingModule({
      imports: [NetworkHealthTabComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: NetworkPostureService, useValue: networkPostureMock },
        { provide: CollectiveService, useValue: collectiveMock },
        { provide: HouseholdDevicesService, useValue: householdDevicesMock },
      ],
      schemas: [NO_ERRORS_SCHEMA],
    })
      .compileComponents()
      .then(() => {
        fixture = TestBed.createComponent(NetworkHealthTabComponent);
        component = fixture.componentInstance;
        httpMock = TestBed.inject(HttpTestingController);
      });
  }

  afterEach(() => {
    // Flush any pending commitments call before verify() to avoid spurious failures.
    // The commitments endpoint is always called during ngOnInit regardless of collective count.
    try {
      const pending = httpMock.match(req => req.url.includes('/api/v1/commitments'));
      pending.forEach(req => req.flush({ items: [], total: 0 }));
    } catch {
      // httpMock may not be initialised in every beforeEach
    }
    httpMock?.verify();
  });

  // =========================================================================
  // Creation
  // =========================================================================

  describe('component creation', () => {
    beforeEach(async () => {
      networkPostureMock = { get: () => of(MOCK_POSTURE) };
      collectiveMock = { listCollectives: () => of(MOCK_COLLECTIVES) };
      householdDevicesMock = { list: () => of(MOCK_HOUSEHOLD_DEVICES) };
      await buildFixture();
    });

    it('should create', () => {
      expect(component).toBeTruthy();
    });

    it('should render the network-posture-card testid', () => {
      fixture.detectChanges();
      const el: HTMLElement = fixture.nativeElement;
      expect(el.querySelector('[data-testid="network-posture-card"]')).toBeTruthy();
    });
  });

  // =========================================================================
  // Live posture state
  // =========================================================================

  describe('when service returns posture data', () => {
    beforeEach(async () => {
      networkPostureMock = { get: () => of(MOCK_POSTURE) };
      collectiveMock = { listCollectives: () => of(MOCK_COLLECTIVES) };
      householdDevicesMock = { list: () => of(MOCK_HOUSEHOLD_DEVICES) };
      await buildFixture();
      fixture.detectChanges();
    });

    it('should show active peers metric', () => {
      const el: HTMLElement = fixture.nativeElement;
      const span = el.querySelector('[data-testid="posture-active-peers"]');
      expect(span).toBeTruthy();
      expect(span?.textContent).toContain('40');
      expect(span?.textContent).toContain('50');
    });

    it('should show households-reciprocating metric', () => {
      const el: HTMLElement = fixture.nativeElement;
      const span = el.querySelector('[data-testid="posture-households-reciprocating"]');
      expect(span).toBeTruthy();
      expect(span?.textContent?.trim()).toBe('8');
    });

    it('should show always-on peers metric', () => {
      const el: HTMLElement = fixture.nativeElement;
      const span = el.querySelector('[data-testid="posture-always-on"]');
      expect(span).toBeTruthy();
      expect(span?.textContent?.trim()).toBe('15');
    });

    it('should show compute-available metric as Yes', () => {
      const el: HTMLElement = fixture.nativeElement;
      const span = el.querySelector('[data-testid="posture-compute"]');
      expect(span).toBeTruthy();
      expect(span?.textContent).toContain('Yes');
    });

    it('should show storage pressure metric', () => {
      const el: HTMLElement = fixture.nativeElement;
      const span = el.querySelector('[data-testid="posture-storage-pressure"]');
      expect(span).toBeTruthy();
      expect(span?.textContent).toContain('45%');
    });

    it('should NOT show the unavailable message', () => {
      const el: HTMLElement = fixture.nativeElement;
      expect(el.textContent).not.toContain('Network posture unavailable');
    });
  });

  // =========================================================================
  // Unavailable state (null from service)
  // =========================================================================

  describe('when service returns null', () => {
    beforeEach(async () => {
      networkPostureMock = { get: () => of(null) };
      collectiveMock = { listCollectives: () => of([]) };
      householdDevicesMock = { list: () => of(MOCK_HOUSEHOLD_DEVICES) };
      await buildFixture();
      fixture.detectChanges();
    });

    it('should show the unavailable message', () => {
      const el: HTMLElement = fixture.nativeElement;
      expect(el.textContent).toContain('Network posture unavailable');
    });

    it('should mention the F2 endpoint', () => {
      const el: HTMLElement = fixture.nativeElement;
      expect(el.textContent).toContain('F2');
    });

    it('should NOT show peer metric testids', () => {
      const el: HTMLElement = fixture.nativeElement;
      expect(el.querySelector('[data-testid="posture-active-peers"]')).toBeNull();
    });
  });

  // =========================================================================
  // compute-available = false
  // =========================================================================

  describe('when compute is unavailable', () => {
    beforeEach(async () => {
      networkPostureMock = { get: () => of({ ...MOCK_POSTURE, computeAvailable: false }) };
      collectiveMock = { listCollectives: () => of(MOCK_COLLECTIVES) };
      householdDevicesMock = { list: () => of(MOCK_HOUSEHOLD_DEVICES) };
      await buildFixture();
      fixture.detectChanges();
    });

    it('should show compute-available metric as No', () => {
      const el: HTMLElement = fixture.nativeElement;
      const span = el.querySelector('[data-testid="posture-compute"]');
      expect(span?.textContent).toContain('No');
    });
  });

  // =========================================================================
  // Household grouping
  // =========================================================================

  describe('household-grouped breakdown', () => {
    beforeEach(async () => {
      networkPostureMock = { get: () => of(MOCK_POSTURE) };
      collectiveMock = { listCollectives: () => of(MOCK_COLLECTIVES) };
      householdDevicesMock = { list: () => of(MOCK_HOUSEHOLD_DEVICES) };
      await buildFixture();
      fixture.detectChanges();
      // Flush active-commitments so the forkJoin + switchMap resolves
      httpMock
        .expectOne(req => req.url.includes('/api/v1/commitments'))
        .flush({ items: [], total: 0 });
      fixture.detectChanges();
    });

    it('should render the household-breakdown section', () => {
      const el: HTMLElement = fixture.nativeElement;
      expect(el.querySelector('[data-testid="household-breakdown"]')).toBeTruthy();
    });

    it('should render a household-row for household-matthew', () => {
      const el: HTMLElement = fixture.nativeElement;
      expect(el.querySelector('[data-testid="household-row-household-matthew"]')).toBeTruthy();
    });

    it('should render a peer-row for the peer in the household', () => {
      const el: HTMLElement = fixture.nativeElement;
      expect(el.querySelector('[data-testid="peer-row-uhCAkABC123"]')).toBeTruthy();
    });

    it('should report householdCount = 1', () => {
      expect(component.householdCount).toBe(1);
    });
  });

  // =========================================================================
  // Household grouping — empty collectives
  // =========================================================================

  describe('when no households are registered', () => {
    beforeEach(async () => {
      networkPostureMock = { get: () => of(MOCK_POSTURE) };
      collectiveMock = { listCollectives: () => of([]) };
      householdDevicesMock = { list: () => of(MOCK_HOUSEHOLD_DEVICES) };
      await buildFixture();
      fixture.detectChanges();
      // combineLatest fires commitments even with empty collectives
      httpMock
        .expectOne(req => req.url.includes('/api/v1/commitments'))
        .flush({ items: [], total: 0 });
      fixture.detectChanges();
    });

    it('should show empty state message', () => {
      const el: HTMLElement = fixture.nativeElement;
      expect(el.querySelector('[data-testid="household-groups-empty"]')).toBeTruthy();
    });

    it('should report householdCount = 0', () => {
      expect(component.householdCount).toBe(0);
    });
  });

  // =========================================================================
  // Helper methods
  // =========================================================================

  describe('formatPressure()', () => {
    beforeEach(async () => {
      networkPostureMock = { get: () => of(MOCK_POSTURE) };
      collectiveMock = { listCollectives: () => of([]) };
      householdDevicesMock = { list: () => of(MOCK_HOUSEHOLD_DEVICES) };
      await buildFixture();
    });

    it('should format 0 as 0%', () => {
      expect(component.formatPressure(0)).toBe('0%');
    });

    it('should format 0.5 as 50%', () => {
      expect(component.formatPressure(0.5)).toBe('50%');
    });

    it('should format 1 as 100%', () => {
      expect(component.formatPressure(1)).toBe('100%');
    });

    it('should round fractional values', () => {
      expect(component.formatPressure(0.456)).toBe('46%');
    });
  });

  describe('pressureClass()', () => {
    beforeEach(async () => {
      networkPostureMock = { get: () => of(MOCK_POSTURE) };
      collectiveMock = { listCollectives: () => of([]) };
      householdDevicesMock = { list: () => of(MOCK_HOUSEHOLD_DEVICES) };
      await buildFixture();
    });

    it('should return pressure-ok for low pressure', () => {
      expect(component.pressureClass(0.3)).toBe('pressure-ok');
    });

    it('should return pressure-warning for medium pressure', () => {
      expect(component.pressureClass(0.7)).toBe('pressure-warning');
    });

    it('should return pressure-critical for high pressure', () => {
      expect(component.pressureClass(0.9)).toBe('pressure-critical');
    });

    it('should treat 0.6 boundary as pressure-warning', () => {
      expect(component.pressureClass(0.6)).toBe('pressure-warning');
    });

    it('should treat 0.85 boundary as pressure-critical', () => {
      expect(component.pressureClass(0.85)).toBe('pressure-critical');
    });
  });

  // =========================================================================
  // Lifecycle
  // =========================================================================

  describe('lifecycle', () => {
    beforeEach(async () => {
      networkPostureMock = { get: () => of(MOCK_POSTURE) };
      collectiveMock = { listCollectives: () => of([]) };
      householdDevicesMock = { list: () => of(MOCK_HOUSEHOLD_DEVICES) };
      await buildFixture();
    });

    it('should handle ngOnDestroy without error', () => {
      fixture.detectChanges();
      expect(() => component.ngOnDestroy()).not.toThrow();
    });
  });
});
