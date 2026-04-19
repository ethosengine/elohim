import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { signal } from '@angular/core';

import { BehaviorSubject, of } from 'rxjs';

import { IdentityService } from '@app/imagodei/services/identity.service';
import { type IdentityState, INITIAL_IDENTITY_STATE } from '@app/imagodei/models/identity.model';
import { PointsService } from '@app/lamad/services/points.service';
import { MasteryService } from '@app/lamad/services/mastery.service';

import { DeviceStewardshipService } from '../../services/device-stewardship.service';
import { HouseholdDevicesService } from '../../services/household-devices.service';

import type { StewardedDevice } from '../../models/device-stewardship.model';
import type { LearnerPointBalance } from '@app/lamad/models/learning-points.model';
import type { HouseholdDevicesView } from '../../../generated/household-devices-view';

import { DeviceStewardshipComponent } from './device-stewardship.component';
import { vi, Mock } from 'vitest';

// =============================================================================
// Test Data
// =============================================================================

const MOCK_CURRENT_DEVICE: StewardedDevice = {
  deviceId: 'agent-pub-key-123',
  displayName: 'My Mac',
  category: 'app-steward',
  status: 'connected',
  lastSeen: new Date().toISOString(),
  isCurrentDevice: true,
  platform: 'desktop-macos',
  doorwayUrl: 'http://localhost:8090',
};

const MOCK_NODE_DEVICE: StewardedDevice = {
  deviceId: 'node-abc-123',
  displayName: 'Living Room HoloPort',
  category: 'node-steward',
  status: 'connected',
  lastSeen: new Date().toISOString(),
  isCurrentDevice: false,
  nodeType: 'holoport',
  location: { label: 'Home Office', region: 'US-West', country: 'US' },
  roles: [
    { role: 'storage', description: 'Content storage', utilizationPercent: 45 },
    { role: 'compute', description: 'Processing', utilizationPercent: 30 },
  ],
  resources: {
    cpuPercent: 35,
    memoryPercent: 60,
    storageUsedGB: 120,
    storageTotalGB: 500,
    bandwidthMbps: 100,
  },
  isPrimaryNode: true,
};

const MOCK_OFFLINE_NODE: StewardedDevice = {
  deviceId: 'node-xyz-789',
  displayName: 'Cloud Node',
  category: 'node-steward',
  status: 'offline',
  lastSeen: new Date(Date.now() - 3600000).toISOString(),
  isCurrentDevice: false,
  nodeType: 'cloud',
  resources: {
    cpuPercent: 0,
    memoryPercent: 0,
    storageUsedGB: 50,
    storageTotalGB: 200,
    bandwidthMbps: 0,
  },
  isPrimaryNode: false,
};

const MOCK_BALANCE: LearnerPointBalance = {
  id: 'bal-1',
  agent_id: 'agent-1',
  total_points: 150,
  total_earned: 200,
  total_spent: 50,
  points_by_trigger_json: JSON.stringify({
    engagement_view: 30,
    engagement_practice: 50,
    challenge_correct: 70,
  }),
  last_point_event_id: 'evt-1',
  last_point_event_at: new Date().toISOString(),
  created_at: new Date().toISOString(),
  updated_at: new Date().toISOString(),
};

// =============================================================================
// Mock Household Devices Data
// =============================================================================

const MOCK_HOUSEHOLD_DEVICES: HouseholdDevicesView = {
  householdId: 'household-matthew',
  devices: [
    {
      shape: {
        nodeId: 'node-nuc-01',
        hostname: 'living-room-nuc',
        deviceArchetypeId: 'home-nuc',
        householdId: 'household-matthew',
        role: 'edge',
        capabilityLevel: 3,
        committed: { cpuCores: 4, memoryGb: 16, storageTb: 2 },
        stewardTier: 'steward',
        region: 'us-east-1',
        signature: 'sig-abc',
        signedAt: new Date().toISOString(),
      },
      peer: {
        peerId: 'peer-pub-key-abc',
        status: 'online',
        generalPoolMember: true,
        acceptingStewardshipReserves: false,
        archetypeClass: 'home-nuc',
        timestamp: String(Date.now() * 1000),
        dhtAnchorHash: 'uhCkk-abc',
        updatedAt: String(Date.now() * 1000),
      },
    },
    {
      shape: {
        nodeId: 'node-blade-02',
        hostname: 'rack-blade-02',
        deviceArchetypeId: 'blade',
        householdId: 'household-matthew',
        role: 'archival',
        capabilityLevel: 5,
        committed: { cpuCores: 16, memoryGb: 64, storageTb: 20 },
        stewardTier: 'pioneer',
        signature: 'sig-def',
        signedAt: new Date().toISOString(),
      },
      peer: null,
    },
  ],
};

// =============================================================================
// Mock Service Factories
// =============================================================================

function createMockHouseholdDevicesService(view: HouseholdDevicesView | null = null) {
  return {
    list: vi.fn().mockReturnValue(of(view)),
    resolveHouseholdId: vi.fn().mockReturnValue(of(view ? view.householdId : null)),
    listForHuman: vi.fn().mockReturnValue(of(view)),
  };
}

function createMockDeviceStewardshipService() {
  return {
    loadDevices: vi.fn().mockReturnValue(Promise.resolve()),
    state: signal({
      devices: [],
      currentDevice: null,
      appStewardDevices: [],
      nodeStewardDevices: [],
      totalDevices: 0,
      connectedCount: 0,
      seenCount: 0,
      offlineCount: 0,
      isLoading: true,
      error: null,
      lastUpdated: new Date().toISOString(),
    }),
    devices: signal<StewardedDevice[]>([]),
    currentDevice: signal<StewardedDevice | null>(null),
    appStewardDevices: signal<StewardedDevice[]>([]),
    nodeStewardDevices: signal<StewardedDevice[]>([]),
    isLoading: signal(true),
    error: signal<string | null>(null),
    totalDevices: signal(0),
    connectedCount: signal(0),
    seenCount: signal(0),
    offlineCount: signal(0),
  };
}

function createMockIdentityService(stage: IdentityState['agencyStage'] = 'visitor') {
  const identityState: IdentityState = {
    ...INITIAL_IDENTITY_STATE,
    agencyStage: stage,
    humanId: 'human-123',
  };
  return {
    identity: signal(identityState),
    mode: signal(INITIAL_IDENTITY_STATE.mode),
    isAuthenticated: signal(false),
    agentPubKey: signal(null),
  };
}

function createMockPointsService(balance: LearnerPointBalance | null = null) {
  const balanceSubject = new BehaviorSubject<LearnerPointBalance | null>(balance);
  return {
    getBalance$: vi.fn().mockReturnValue(balanceSubject.asObservable()),
    loadHistory: vi.fn().mockReturnValue(of([])),
    getPointsByTriggerSync: vi
      .fn()
      .mockReturnValue(balance ? JSON.parse(balance.points_by_trigger_json) : {}),
    getRecentPointsEarned: vi.fn().mockReturnValue(25),
    refreshBalance: vi.fn(),
    _balanceSubject: balanceSubject,
  };
}

function createMockMasteryService() {
  return {
    getMasteryForHuman: vi.fn().mockReturnValue(
      of([
        {
          id: 'm1',
          appId: 'app',
          humanId: 'human-123',
          contentId: 'c1',
          masteryLevel: 'mastered',
          masteryLevelIndex: 6,
          freshnessScore: 95,
          needsRefresh: false,
          engagementCount: 10,
          lastEngagementType: null,
          lastEngagementAt: null,
          levelAchievedAt: null,
          contentVersionAtMastery: null,
          assessmentEvidence: null,
          privileges: null,
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        {
          id: 'm2',
          appId: 'app',
          humanId: 'human-123',
          contentId: 'c2',
          masteryLevel: 'understanding',
          masteryLevelIndex: 2,
          freshnessScore: 40,
          needsRefresh: true,
          engagementCount: 3,
          lastEngagementType: null,
          lastEngagementAt: null,
          levelAchievedAt: null,
          contentVersionAtMastery: null,
          assessmentEvidence: null,
          privileges: null,
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
      ])
    ),
  };
}

// =============================================================================
// Tests
// =============================================================================

describe('DeviceStewardshipComponent', () => {
  let component: DeviceStewardshipComponent;
  let fixture: ComponentFixture<DeviceStewardshipComponent>;
  let mockService: ReturnType<typeof createMockDeviceStewardshipService>;
  let mockIdentityService: ReturnType<typeof createMockIdentityService>;
  let mockPointsService: ReturnType<typeof createMockPointsService>;
  let mockMasteryService: ReturnType<typeof createMockMasteryService>;
  let mockHouseholdDevicesService: ReturnType<typeof createMockHouseholdDevicesService>;

  function setupTestBed(
    agencyStage: IdentityState['agencyStage'] = 'visitor',
    balance: LearnerPointBalance | null = null,
    householdDevices: HouseholdDevicesView | null = null
  ) {
    mockService = createMockDeviceStewardshipService();
    mockIdentityService = createMockIdentityService(agencyStage);
    mockPointsService = createMockPointsService(balance);
    mockMasteryService = createMockMasteryService();
    mockHouseholdDevicesService = createMockHouseholdDevicesService(householdDevices);

    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [DeviceStewardshipComponent],
      providers: [
        provideRouter([]),
        { provide: DeviceStewardshipService, useValue: mockService },
        { provide: IdentityService, useValue: mockIdentityService },
        { provide: PointsService, useValue: mockPointsService },
        { provide: MasteryService, useValue: mockMasteryService },
        { provide: HouseholdDevicesService, useValue: mockHouseholdDevicesService },
      ],
    });

    fixture = TestBed.createComponent(DeviceStewardshipComponent);
    component = fixture.componentInstance;
  }

  beforeEach(() => {
    setupTestBed();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  // ==========================================================================
  // Tab Behavior
  // ==========================================================================

  describe('Tab Behavior', () => {
    it('should default to activity tab', () => {
      expect(component.activeTab()).toBe('activity');
    });

    it('should render tab bar with two tabs', () => {
      fixture.detectChanges();
      const tabs = fixture.nativeElement.querySelectorAll('.tab-btn');
      expect(tabs.length).toBe(2);
      expect(tabs[0].textContent).toContain('Activity');
      expect(tabs[1].textContent).toContain('Devices');
    });

    it('should switch to devices tab', () => {
      component.selectTab('devices');
      expect(component.activeTab()).toBe('devices');
    });

    it('should switch back to activity tab', () => {
      component.selectTab('devices');
      component.selectTab('activity');
      expect(component.activeTab()).toBe('activity');
    });

    it('should show active class on selected tab', () => {
      fixture.detectChanges();
      const tabs = fixture.nativeElement.querySelectorAll('.tab-btn');
      expect(tabs[0].classList).toContain('active');
      expect(tabs[1].classList).not.toContain('active');

      component.selectTab('devices');
      fixture.detectChanges();
      const updatedTabs = fixture.nativeElement.querySelectorAll('.tab-btn');
      expect(updatedTabs[0].classList).not.toContain('active');
      expect(updatedTabs[1].classList).toContain('active');
    });

    it('should show lock icon on devices tab for non-steward', () => {
      fixture.detectChanges();
      const lockIcon = fixture.nativeElement.querySelector('.lock-icon');
      expect(lockIcon).toBeTruthy();
    });

    it('should not show lock icon on devices tab for steward', () => {
      setupTestBed('app-steward');
      fixture.detectChanges();
      const lockIcon = fixture.nativeElement.querySelector('.lock-icon');
      expect(lockIcon).toBeNull();
    });

    it('should show header with "Your Stewardship"', () => {
      fixture.detectChanges();
      const h1 = fixture.nativeElement.querySelector('h1');
      expect(h1.textContent).toContain('Your Stewardship');
    });
  });

  // ==========================================================================
  // Activity Tab - Points
  // ==========================================================================

  describe('Activity Tab - Points', () => {
    it('should show points strip when balance exists', () => {
      setupTestBed('hosted', MOCK_BALANCE);
      fixture.detectChanges();

      const strip = fixture.nativeElement.querySelector('.points-strip');
      expect(strip).toBeTruthy();
    });

    it('should display total points', () => {
      setupTestBed('hosted', MOCK_BALANCE);
      fixture.detectChanges();

      const statValues = fixture.nativeElement.querySelectorAll('.points-stat .stat-value');
      expect(statValues[0].textContent.trim()).toBe('150');
    });

    it('should display lifetime earned', () => {
      setupTestBed('hosted', MOCK_BALANCE);
      fixture.detectChanges();

      const statValues = fixture.nativeElement.querySelectorAll('.points-stat .stat-value');
      expect(statValues[2].textContent.trim()).toBe('200');
    });

    it('should display weekly points', () => {
      setupTestBed('hosted', MOCK_BALANCE);
      fixture.detectChanges();

      const statValues = fixture.nativeElement.querySelectorAll('.points-stat .stat-value');
      expect(statValues[1].textContent.trim()).toBe('25');
    });

    it('should show trigger breakdown cards', () => {
      setupTestBed('hosted', MOCK_BALANCE);
      fixture.detectChanges();

      const cards = fixture.nativeElement.querySelectorAll('.trigger-card');
      expect(cards.length).toBe(3);
    });

    it('should show empty state when no balance', () => {
      setupTestBed('hosted', null);
      mockService.isLoading.set(false);
      fixture.detectChanges();

      const emptyState = fixture.nativeElement.querySelector('.empty-state');
      expect(emptyState).toBeTruthy();
      expect(emptyState.textContent).toContain('Start Learning to Earn Points');
    });

    it('should show link to lamad in activity empty state', () => {
      setupTestBed('hosted', null);
      mockService.isLoading.set(false);
      fixture.detectChanges();

      const link = fixture.nativeElement.querySelector('.action-link');
      expect(link).toBeTruthy();
      expect(link.getAttribute('href')).toBe('/lamad');
    });
  });

  // ==========================================================================
  // Activity Tab - Mastery
  // ==========================================================================

  describe('Activity Tab - Mastery', () => {
    it('should show mastery stats when data exists', () => {
      setupTestBed('hosted', MOCK_BALANCE);
      fixture.detectChanges();

      const masteryStrip = fixture.nativeElement.querySelector('.mastery-strip');
      expect(masteryStrip).toBeTruthy();
    });

    it('should display mastery stat values', () => {
      setupTestBed('hosted', MOCK_BALANCE);
      fixture.detectChanges();

      const stats = fixture.nativeElement.querySelectorAll('.mastery-stat .stat-value');
      // totalRecords = 2, masteredCount = 1, freshnessPercent = 50%
      expect(stats[0].textContent.trim()).toBe('2');
      expect(stats[1].textContent.trim()).toBe('1');
      expect(stats[2].textContent.trim()).toBe('50%');
    });

    it('should call getMasteryForHuman with humanId', () => {
      setupTestBed('hosted', MOCK_BALANCE);
      fixture.detectChanges();
      expect(mockMasteryService.getMasteryForHuman).toHaveBeenCalledWith('human-123');
    });
  });

  // ==========================================================================
  // Devices Tab - Upgrade Prompt
  // ==========================================================================

  describe('Devices Tab - Upgrade Prompt', () => {
    beforeEach(() => {
      setupTestBed('hosted');
      component.selectTab('devices');
      fixture.detectChanges();
    });

    it('should show upgrade prompt for hosted users', () => {
      const prompt = fixture.nativeElement.querySelector('.upgrade-prompt');
      expect(prompt).toBeTruthy();
      expect(prompt.textContent).toContain('Become an App Steward');
    });

    it('should show stage progression chips', () => {
      const chips = fixture.nativeElement.querySelectorAll('.stage-chip');
      expect(chips.length).toBe(4);
    });

    it('should highlight current stage', () => {
      const currentChip = fixture.nativeElement.querySelector('.stage-chip.current');
      expect(currentChip).toBeTruthy();
      expect(currentChip.textContent).toContain('Hosted User');
    });

    it('should highlight next stage', () => {
      const nextChip = fixture.nativeElement.querySelector('.stage-chip.next');
      expect(nextChip).toBeTruthy();
      expect(nextChip.textContent).toContain('App Steward');
    });

    it('should show next stage benefits', () => {
      const benefits = fixture.nativeElement.querySelector('.upgrade-benefits');
      expect(benefits).toBeTruthy();
      expect(benefits.textContent).toContain('App Steward Benefits');
    });

    it('should show disabled download CTA', () => {
      const cta = fixture.nativeElement.querySelector('.upgrade-cta');
      expect(cta).toBeTruthy();
      expect(cta.disabled).toBe(true);
      expect(cta.textContent).toContain('Download the Desktop App');
    });

    it('should show upgrade prompt for visitor users', () => {
      setupTestBed('visitor');
      component.selectTab('devices');
      fixture.detectChanges();

      const prompt = fixture.nativeElement.querySelector('.upgrade-prompt');
      expect(prompt).toBeTruthy();
    });
  });

  // ==========================================================================
  // Devices Tab - Steward Content
  // ==========================================================================

  describe('Devices Tab - Steward Content', () => {
    beforeEach(() => {
      setupTestBed('app-steward');
      component.selectTab('devices');
    });

    it('should not show upgrade prompt for steward users', () => {
      fixture.detectChanges();
      const prompt = fixture.nativeElement.querySelector('.upgrade-prompt');
      expect(prompt).toBeNull();
    });

    it('should show loading state in devices tab while household devices load', () => {
      // Simulate in-flight state by setting loading signal after detectChanges
      fixture.detectChanges();
      component.householdDevicesLoading.set(true);
      component.householdDevicesError.set(null);
      fixture.detectChanges();
      const loading = fixture.nativeElement.querySelector('[data-testid="devices-loading"]');
      expect(loading).toBeTruthy();
      expect(loading.textContent).toContain('Discovering devices');
    });
  });

  // ==========================================================================
  // Initialization
  // ==========================================================================

  describe('Initialization', () => {
    it('should call loadDevices on init', () => {
      fixture.detectChanges();
      expect(mockService.loadDevices).toHaveBeenCalled();
    });

    it('should call getBalance$ on init', () => {
      fixture.detectChanges();
      expect(mockPointsService.getBalance$).toHaveBeenCalled();
    });

    it('should call loadHistory on init', () => {
      fixture.detectChanges();
      expect(mockPointsService.loadHistory).toHaveBeenCalled();
    });
  });

  // ==========================================================================
  // Device Cards Rendering (Devices Tab) — now driven by HouseholdDevicesView
  // ==========================================================================

  describe('Device Cards Rendering', () => {
    beforeEach(() => {
      setupTestBed('node-steward', null, MOCK_HOUSEHOLD_DEVICES);
      component.selectTab('devices');
      fixture.detectChanges();
    });

    it('should render device grid with household devices', () => {
      const grid = fixture.nativeElement.querySelector('[data-testid="device-list"]');
      expect(grid).toBeTruthy();
    });

    it('should render two device cards', () => {
      const cards = fixture.nativeElement.querySelectorAll(
        '[data-testid="device-node-nuc-01"], [data-testid="device-node-blade-02"]'
      );
      expect(cards.length).toBe(2);
    });

    it('should display hostname in card', () => {
      const card = fixture.nativeElement.querySelector('[data-testid="device-node-nuc-01"]');
      expect(card.textContent).toContain('living-room-nuc');
    });

    it('should show capability level', () => {
      const card = fixture.nativeElement.querySelector('[data-testid="device-node-nuc-01"]');
      expect(card.textContent).toContain('L3');
    });
  });

  // ==========================================================================
  // Summary Strip (Devices Tab) — now shows household id + device count
  // ==========================================================================

  describe('Summary Strip', () => {
    beforeEach(() => {
      setupTestBed('node-steward', null, MOCK_HOUSEHOLD_DEVICES);
      component.selectTab('devices');
      fixture.detectChanges();
    });

    it('should show device count in summary', () => {
      const summary = fixture.nativeElement.querySelector('[data-testid="device-summary"]');
      expect(summary).toBeTruthy();
      expect(summary.textContent).toContain('2 devices');
    });

    it('should show household id in summary', () => {
      const summary = fixture.nativeElement.querySelector('[data-testid="device-summary"]');
      expect(summary.textContent).toContain('household-matthew');
    });

    it('should show singular "device" for single-device household', () => {
      const singleDeviceView = {
        householdId: 'household-adam',
        devices: [MOCK_HOUSEHOLD_DEVICES.devices[0]],
      };
      setupTestBed('node-steward', null, singleDeviceView);
      component.selectTab('devices');
      fixture.detectChanges();

      const summary = fixture.nativeElement.querySelector('[data-testid="device-summary"]');
      expect(summary.textContent).toContain('1 device');
      expect(summary.textContent).not.toContain('1 devices');
    });

    it('should not show summary strip when no household data', () => {
      setupTestBed('node-steward', null, null);
      component.selectTab('devices');
      fixture.detectChanges();

      const summary = fixture.nativeElement.querySelector('[data-testid="device-summary"]');
      expect(summary).toBeNull();
    });
  });

  // ==========================================================================
  // Status Display
  // ==========================================================================

  describe('Status Display', () => {
    it('should return correct color for connected status', () => {
      const display = component.getStatusDisplay('connected');
      expect(display.color).toBe('#22c55e');
      expect(display.label).toBe('Connected');
    });

    it('should return correct color for seen status', () => {
      const display = component.getStatusDisplay('seen');
      expect(display.color).toBe('#f59e0b');
    });

    it('should return correct color for offline status', () => {
      const display = component.getStatusDisplay('offline');
      expect(display.color).toBe('#ef4444');
    });
  });

  // ==========================================================================
  // Refresh
  // ==========================================================================

  describe('Refresh', () => {
    it('should call loadDevices on refresh', () => {
      mockService.loadDevices.mockClear();
      component.refresh();
      expect(mockService.loadDevices).toHaveBeenCalled();
    });

    it('should disable refresh button while loading', () => {
      mockService.isLoading.set(true);
      fixture.detectChanges();
      const btn = fixture.nativeElement.querySelector('.refresh-btn');
      expect(btn.disabled).toBe(true);
    });
  });

  // ==========================================================================
  // Error State
  // ==========================================================================

  describe('Error State', () => {
    it('should show error banner when error exists', () => {
      mockService.isLoading.set(false);
      mockService.error.set('Failed to load device data');
      fixture.detectChanges();

      const banner = fixture.nativeElement.querySelector('.error-banner');
      expect(banner).toBeTruthy();
      expect(banner.textContent).toContain('Failed to load device data');
    });

    it('should hide error banner when no error', () => {
      mockService.isLoading.set(false);
      mockService.error.set(null);
      fixture.detectChanges();

      expect(fixture.nativeElement.querySelector('.error-banner')).toBeNull();
    });

    it('should retry on error banner button click', () => {
      mockService.isLoading.set(false);
      mockService.error.set('Some error');
      fixture.detectChanges();

      mockService.loadDevices.mockClear();
      const retryBtn = fixture.nativeElement.querySelector('.dismiss-btn');
      retryBtn.click();

      expect(mockService.loadDevices).toHaveBeenCalled();
    });
  });

  // ==========================================================================
  // Helper Methods
  // ==========================================================================

  describe('Helper Methods', () => {
    it('should compute storage percent correctly', () => {
      expect(component.storagePercent(MOCK_NODE_DEVICE)).toBe(24);
    });

    it('should return 0 for storage percent when total is 0', () => {
      const device: StewardedDevice = {
        ...MOCK_NODE_DEVICE,
        resources: { ...MOCK_NODE_DEVICE.resources!, storageTotalGB: 0 },
      };
      expect(component.storagePercent(device)).toBe(0);
    });

    it('should return 0 for storage percent when no resources', () => {
      const device: StewardedDevice = {
        ...MOCK_NODE_DEVICE,
        resources: undefined,
      };
      expect(component.storagePercent(device)).toBe(0);
    });

    it('should get node icon from nodeType', () => {
      expect(component.getNodeIcon(MOCK_NODE_DEVICE)).toBe('dns');
    });

    it('should get node label from nodeType', () => {
      expect(component.getNodeLabel(MOCK_NODE_DEVICE)).toBe('HoloPort');
    });

    it('should return default icon when no nodeType', () => {
      const device: StewardedDevice = { ...MOCK_NODE_DEVICE, nodeType: undefined };
      expect(component.getNodeIcon(device)).toBe('dns');
    });

    it('should compute agency label from identity', () => {
      mockIdentityService.identity.set({
        ...INITIAL_IDENTITY_STATE,
        agencyStage: 'app-steward',
      });
      expect(component.agencyLabel()).toBe('App Steward');
    });

    it('should compute agency label for visitor', () => {
      mockIdentityService.identity.set({
        ...INITIAL_IDENTITY_STATE,
        agencyStage: 'visitor',
      });
      expect(component.agencyLabel()).toBe('Visitor');
    });

    it('should filter non-current app devices', () => {
      mockService.appStewardDevices.set([
        MOCK_CURRENT_DEVICE,
        { ...MOCK_CURRENT_DEVICE, deviceId: 'other', isCurrentDevice: false },
      ]);
      expect(component.nonCurrentAppDevices().length).toBe(1);
      expect(component.nonCurrentAppDevices()[0].deviceId).toBe('other');
    });

    it('should format points with sign', () => {
      expect(component.formatPointsDisplay(10)).toBe('+10');
      expect(component.formatPointsDisplay(-5)).toBe('-5');
      expect(component.formatPointsDisplay(0)).toBe('0');
    });

    it('should compute isSteward for app-steward', () => {
      mockIdentityService.identity.set({
        ...INITIAL_IDENTITY_STATE,
        agencyStage: 'app-steward',
      });
      expect(component.isSteward()).toBe(true);
    });

    it('should compute isSteward for node-steward', () => {
      mockIdentityService.identity.set({
        ...INITIAL_IDENTITY_STATE,
        agencyStage: 'node-steward',
      });
      expect(component.isSteward()).toBe(true);
    });

    it('should compute isSteward false for hosted', () => {
      mockIdentityService.identity.set({
        ...INITIAL_IDENTITY_STATE,
        agencyStage: 'hosted',
      });
      expect(component.isSteward()).toBe(false);
    });
  });

  // ==========================================================================
  // Household Devices Tab (P2P Dataplane Visibility)
  // ==========================================================================

  describe('Household Devices Tab', () => {
    describe('Loading state', () => {
      it('should show loading overlay while fetching devices', () => {
        setupTestBed('node-steward', null, null);
        component.selectTab('devices');
        fixture.detectChanges();
        // Simulate in-flight state after initial render
        component.householdDevicesLoading.set(true);
        component.householdDevicesError.set(null);
        fixture.detectChanges();

        expect(component.householdDevicesLoading()).toBe(true);
        const loadingEl = fixture.nativeElement.querySelector('[data-testid="devices-loading"]');
        expect(loadingEl).toBeTruthy();
        expect(loadingEl.textContent).toContain('Discovering devices');
      });
    });

    describe('Happy path — devices rendered', () => {
      beforeEach(() => {
        setupTestBed('node-steward', null, MOCK_HOUSEHOLD_DEVICES);
        component.selectTab('devices');
        fixture.detectChanges();
      });

      it('should not show loading overlay after data arrives', () => {
        expect(component.householdDevicesLoading()).toBe(false);
        expect(fixture.nativeElement.querySelector('[data-testid="devices-loading"]')).toBeNull();
      });

      it('should render device-list container', () => {
        const list = fixture.nativeElement.querySelector('[data-testid="device-list"]');
        expect(list).toBeTruthy();
      });

      it('should render a card for each device', () => {
        // Use attribute selector that exactly matches the node-id pattern
        const cards = fixture.nativeElement.querySelectorAll(
          '[data-testid="device-node-nuc-01"], [data-testid="device-node-blade-02"]'
        );
        expect(cards.length).toBe(2);
      });

      it('should set data-testid with nodeId for each device card', () => {
        const card1 = fixture.nativeElement.querySelector('[data-testid="device-node-nuc-01"]');
        const card2 = fixture.nativeElement.querySelector('[data-testid="device-node-blade-02"]');
        expect(card1).toBeTruthy();
        expect(card2).toBeTruthy();
      });

      it('should display device hostname', () => {
        const card = fixture.nativeElement.querySelector('[data-testid="device-node-nuc-01"]');
        expect(card.textContent).toContain('living-room-nuc');
      });

      it('should display archetype label', () => {
        const card = fixture.nativeElement.querySelector('[data-testid="device-node-nuc-01"]');
        expect(card.textContent).toContain('Home NUC');
      });

      it('should display capability level', () => {
        const card = fixture.nativeElement.querySelector('[data-testid="device-node-nuc-01"]');
        expect(card.textContent).toContain('L3');
      });

      it('should display role', () => {
        const card = fixture.nativeElement.querySelector('[data-testid="device-node-nuc-01"]');
        expect(card.textContent).toContain('edge');
      });

      it('should display online status for peer with status online', () => {
        const card = fixture.nativeElement.querySelector('[data-testid="device-node-nuc-01"]');
        expect(card.textContent).toContain('Online');
      });

      it('should display Unknown status for device with null peer', () => {
        const card = fixture.nativeElement.querySelector('[data-testid="device-node-blade-02"]');
        expect(card.textContent).toContain('Unknown');
      });

      it('should show household ID in summary strip', () => {
        const summary = fixture.nativeElement.querySelector('[data-testid="device-summary"]');
        expect(summary).toBeTruthy();
        expect(summary.textContent).toContain('household-matthew');
      });

      it('should call listForHuman with humanId from identity', () => {
        expect(mockHouseholdDevicesService.listForHuman).toHaveBeenCalledWith('human-123');
      });
    });

    describe('Empty state', () => {
      it('should show empty state when household has no devices', () => {
        setupTestBed('node-steward', null, { householdId: 'household-matthew', devices: [] });
        component.selectTab('devices');
        fixture.detectChanges();

        const emptyEl = fixture.nativeElement.querySelector('[data-testid="devices-empty"]');
        expect(emptyEl).toBeTruthy();
        expect(emptyEl.textContent).toContain('No Devices Registered');
      });

      it('should show empty state when human has no household', () => {
        setupTestBed('node-steward', null, null);
        component.selectTab('devices');
        fixture.detectChanges();

        const emptyEl = fixture.nativeElement.querySelector('[data-testid="devices-empty"]');
        expect(emptyEl).toBeTruthy();
      });
    });

    describe('Error state', () => {
      it('should show error banner when fetch fails', () => {
        setupTestBed('node-steward', null, null);
        const { throwError } = require('rxjs');
        mockHouseholdDevicesService.listForHuman.mockReturnValue(
          throwError(() => new Error('Network error'))
        );

        component.selectTab('devices');
        fixture.detectChanges();

        expect(component.householdDevicesError()).toBe('Network error');
        const errorEl = fixture.nativeElement.querySelector('[data-testid="devices-error"]');
        expect(errorEl).toBeTruthy();
        expect(errorEl.textContent).toContain('Network error');
      });

      it('should retry on retry button click', () => {
        setupTestBed('node-steward', null, MOCK_HOUSEHOLD_DEVICES);
        component.selectTab('devices');
        fixture.detectChanges();
        // After initial render, force error state so retry button appears
        component.householdDevicesError.set('Some error');
        component.householdDevicesLoading.set(false);
        fixture.detectChanges();

        const retryBtn = fixture.nativeElement.querySelector('[data-testid="devices-retry-btn"]');
        expect(retryBtn).toBeTruthy();
        mockHouseholdDevicesService.listForHuman.mockClear();
        retryBtn.click();

        expect(mockHouseholdDevicesService.listForHuman).toHaveBeenCalled();
      });
    });

    describe('Display helpers', () => {
      beforeEach(() => setupTestBed('node-steward'));

      it('should return green color for online peer status', () => {
        expect(component.getPeerStatusColor('online')).toBe('#22c55e');
      });

      it('should return amber color for degraded peer status', () => {
        expect(component.getPeerStatusColor('degraded')).toBe('#f59e0b');
      });

      it('should return red color for offline peer status', () => {
        expect(component.getPeerStatusColor('offline')).toBe('#ef4444');
      });

      it('should return grey for unknown peer status', () => {
        expect(component.getPeerStatusColor(undefined)).toBe('#64748b');
      });

      it('should return correct label for online status', () => {
        expect(component.getPeerStatusLabel('online')).toBe('Online');
      });

      it('should return Unknown for undefined status', () => {
        expect(component.getPeerStatusLabel(undefined)).toBe('Unknown');
      });

      it('should map home-nuc archetype to readable label', () => {
        expect(component.getArchetypeLabel('home-nuc')).toBe('Home NUC');
      });

      it('should return raw id for unknown archetype', () => {
        expect(component.getArchetypeLabel('custom-thing')).toBe('custom-thing');
      });

      it('should map edge role to router icon', () => {
        expect(component.getRoleIcon('edge')).toBe('router');
      });

      it('should map inference role to psychology icon', () => {
        expect(component.getRoleIcon('inference')).toBe('psychology');
      });
    });
  });
});
