import { ComponentFixture, TestBed } from '@angular/core/testing';
import { Router, ActivatedRoute } from '@angular/router';
import { signal } from '@angular/core';

import { of } from 'rxjs';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { DiscoveryAttestationService } from '@app/lamad/quiz-engine/services/discovery-attestation.service';

import type { HumanProfile } from '../../models/identity.model';
import { AgencyService } from '../../services/agency.service';
import { AuthService } from '../../services/auth.service';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { HostingAccountService } from '../../services/hosting-account.service';
import { IdentityService } from '../../services/identity.service';
import { SessionHumanService } from '../../services/session-human.service';
import { TauriAuthService } from '../../services/tauri-auth.service';
import { ProfileComponent } from './profile.component';
import { vi } from 'vitest';

describe('ProfileComponent', () => {
  let component: ProfileComponent;
  let fixture: ComponentFixture<ProfileComponent>;
  let mockIdentityService: any;
  let mockAgencyService: any;
  let mockDiscoveryService: any;
  let mockRouter: any;

  const mockProfile: HumanProfile = {
    id: 'human-123',
    displayName: 'Test User',
    bio: 'Test bio',
    affinities: ['learning', 'philosophy'],
    location: 'Test Location',
    profileReach: 'community',
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
  };

  beforeEach(async () => {
    mockIdentityService = {
      getCurrentHuman: vi.fn(),
      updateProfile: vi.fn(),
      profile: signal(mockProfile),
      displayName: signal('Test User'),
      mode: signal('hosted'),
      isAuthenticated: signal(true),
      attestations: signal([]),
      isLoading: signal(false),
      did: signal('did:web:hosted.elohim.host:humans:human-123'),
      identity: signal({
        mode: 'hosted',
        isAuthenticated: true,
        humanId: 'human-123',
        displayName: 'Test User',
        agentPubKey: null,
        did: 'did:web:hosted.elohim.host:humans:human-123',
        profile: mockProfile,
        attestations: [],
        agencyStage: 'hosted',
        keyLocation: 'custodial',
        canExportKeys: false,
        keyBackup: null,
        isLocalConductor: false,
        conductorUrl: null,
        linkedSessionId: null,
        hasPendingMigration: false,
        hostingCost: null,
        nodeOperatorIncome: null,
        isLoading: false,
        error: null,
      }),
    };
    mockIdentityService.getCurrentHuman.mockReturnValue(Promise.resolve(mockProfile));
    mockIdentityService.updateProfile.mockReturnValue(Promise.resolve(mockProfile));

    mockAgencyService = {
      currentStage: signal('hosted'),
      stageInfo: signal({
        stage: 'hosted',
        description: 'Test',
        label: 'Hosted User',
        icon: 'cloud',
        tagline: 'Keys held by Elohim',
        benefits: [],
        limitations: [],
        order: 2,
      }),
      canUpgrade: signal(false),
      agencyState: signal({
        currentStage: 'hosted',
        keys: [],
        dataResidency: [],
        migrationTarget: null,
        connectionStatus: { state: 'offline', label: 'Offline' },
        stageInfo: {
          stage: 'hosted',
          label: 'Hosted User',
          tagline: 'Keys held by Elohim',
          description: 'Test',
          icon: 'cloud',
          benefits: [],
          limitations: [],
          order: 2,
        },
        hasStoredCredentials: false,
        migrationAvailable: false,
      }),
      connectionStatus: signal({
        state: 'offline',
        label: 'Offline',
        description: 'Not connected',
      }),
    };

    mockDiscoveryService = {
      toggleFeatured: vi.fn(),
      getBadgeDisplay: vi.fn(),
      featuredResults: signal([]),
    };
    mockDiscoveryService.getBadgeDisplay.mockReturnValue({
      label: 'ISTP',
      icon: '🔧',
      color: '#8B5CF6',
    });

    mockRouter = { navigate: vi.fn() };
    mockRouter.navigate.mockReturnValue(Promise.resolve(true));

    await TestBed.configureTestingModule({
      imports: [ProfileComponent],
      providers: [
        { provide: IdentityService, useValue: mockIdentityService },
        { provide: AgencyService, useValue: mockAgencyService },
        { provide: DiscoveryAttestationService, useValue: mockDiscoveryService },
        { provide: Router, useValue: mockRouter },
        {
          provide: AuthService,
          useValue: { isAuthenticated: vi.fn().mockReturnValue(true) },
        },
        {
          provide: SessionHumanService,
          useValue: { hasSession: signal(false), getSession: vi.fn().mockReturnValue(null) },
        },
        {
          provide: DoorwayRegistryService,
          useValue: { selected: signal(null), selectedUrl: vi.fn().mockReturnValue(null) },
        },
        {
          provide: HolochainClientService,
          useValue: { isConnected: signal(true), getDisplayInfo: vi.fn() },
        },
        {
          provide: TauriAuthService,
          useValue: { isTauri: signal(false), needsUnlock: vi.fn().mockReturnValue(false) },
        },
        {
          provide: HostingAccountService,
          useValue: { getHostingInfo: vi.fn().mockReturnValue(Promise.resolve(null)) },
        },
        {
          provide: ActivatedRoute,
          useValue: { queryParams: of({}), snapshot: { fragment: null } },
        },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(ProfileComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('Profile Loading', () => {
    it('should load profile on init', async () => {
      await component.loadProfile();

      expect(mockIdentityService.getCurrentHuman).toHaveBeenCalled();
    });

    it('should handle profile load failure silently', async () => {
      mockIdentityService.getCurrentHuman.mockReturnValue(
        Promise.reject(new Error('Network error'))
      );

      await component.loadProfile();

      // Should not throw, just log warning
    });
  });

  describe('Tab Navigation', () => {
    it('should default to identity tab', () => {
      expect(component.activeTab()).toBe('identity');
    });

    it('should switch tabs', () => {
      component.selectTab('network');
      expect(component.activeTab()).toBe('network');

      component.selectTab('data');
      expect(component.activeTab()).toBe('data');

      component.selectTab('identity');
      expect(component.activeTab()).toBe('identity');
    });
  });

  describe('Editing Mode', () => {
    it('should enter edit mode', () => {
      component.startEditing();

      expect(component.isEditing()).toBe(true);
      expect(component.form.displayName).toBe(mockProfile.displayName);
      expect(component.form.bio).toBe(mockProfile.bio ?? '');
      expect(component.form.location).toBe(mockProfile.location ?? '');
    });

    it('should cancel editing', () => {
      component.isEditing.set(true);

      component.cancelEditing();

      expect(component.isEditing()).toBe(false);
    });

    it('should require display name for save', async () => {
      component.form.displayName = '';

      await component.saveProfile();

      expect(component.error()).toContain('Display name is required');
    });

    it('should save profile with valid data', async () => {
      component.form.displayName = 'Updated Name';
      component.form.bio = 'Updated bio';

      await component.saveProfile();

      expect(mockIdentityService.updateProfile).toHaveBeenCalledWith(
        expect.objectContaining({
          displayName: 'Updated Name',
          bio: 'Updated bio',
        })
      );
      expect(component.isEditing()).toBe(false);
      expect(component.successMessage()).toContain('Profile updated');
    });

    it('should handle save error', async () => {
      mockIdentityService.updateProfile.mockReturnValue(Promise.reject(new Error('Save failed')));
      component.form.displayName = 'Test';

      await component.saveProfile();

      expect(component.error()).toBe('Save failed');
    });
  });

  describe('Computed Properties', () => {
    it('should compute canEdit for network mode', () => {
      expect(component.canEdit()).toBe(true);
    });

    it('should compute isNetworkUser for hosted mode', () => {
      expect(component.isNetworkUser()).toBe(true);
    });
  });

  describe('Message Clearing', () => {
    it('should clear error and success messages', () => {
      component.error.set('Error');
      component.successMessage.set('Success');

      component.clearMessages();

      expect(component.error()).toBeNull();
      expect(component.successMessage()).toBeNull();
    });
  });

  describe('Navigation', () => {
    it('should navigate back', () => {
      component.goBack();

      expect(mockRouter.navigate).toHaveBeenCalledWith(['/']);
    });

    it('should navigate to discovery', () => {
      component.navigateToDiscovery();

      expect(mockRouter.navigate).toHaveBeenCalledWith(['/lamad/discovery']);
    });

    it('should navigate to resource by contentNodeId', () => {
      component.navigateToResource('content-enneagram-001');

      expect(mockRouter.navigate).toHaveBeenCalledWith(['/resource', 'content-enneagram-001']);
    });
  });

  describe('Discovery Badge Helpers', () => {
    const mockResult = {
      id: 'discovery-1',
      assessmentId: 'assessment-mbti',
      assessmentTitle: 'MBTI Assessment',
      contentNodeId: 'content-mbti-001',
      framework: 'mbti',
      category: 'personality',
      humanId: 'human-123',
      completedAt: '2026-01-15T10:00:00.000Z',
      primaryType: { typeId: 'istp', name: 'ISTP', score: 0.9 },
      subscaleScores: {},
      displayString: 'ISTP',
      shortDisplay: 'ISTP',
    } as any;

    it('should return badge color from discovery service', () => {
      const color = component.getBadgeColor(mockResult);

      expect(mockDiscoveryService.getBadgeDisplay).toHaveBeenCalledWith(mockResult);
      expect(color).toBe('#8B5CF6');
    });

    it('should return category icon for discovery result', () => {
      const icon = component.getDiscoveryCategoryIcon(mockResult);

      expect(icon).toBeTruthy();
    });

    it('should expose featuredDiscoveryResults from service', () => {
      expect(component.featuredDiscoveryResults()).toEqual([]);
    });
  });

  describe('Helpers', () => {
    it('should get reach label', () => {
      const label = component.getReachLabel('community');

      expect(label).toBeTruthy();
    });

    it('should return Not set for undefined reach', () => {
      const label = component.getReachLabel(undefined);

      expect(label).toBe('Not set');
    });
  });
});
