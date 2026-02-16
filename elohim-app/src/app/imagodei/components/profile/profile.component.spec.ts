import { ComponentFixture, TestBed } from '@angular/core/testing';
import { Router, ActivatedRoute } from '@angular/router';
import { signal } from '@angular/core';

import { of } from 'rxjs';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { DiscoveryAttestationService } from '@app/lamad/quiz-engine/services/discovery-attestation.service';

import type { HumanProfile } from '../../models/identity.model';
import { AgencyService } from '../../services/agency.service';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { IdentityService } from '../../services/identity.service';
import { SessionHumanService } from '../../services/session-human.service';
import { TauriAuthService } from '../../services/tauri-auth.service';
import { ProfileComponent } from './profile.component';

describe('ProfileComponent', () => {
  let component: ProfileComponent;
  let fixture: ComponentFixture<ProfileComponent>;
  let mockIdentityService: jasmine.SpyObj<IdentityService>;
  let mockAgencyService: jasmine.SpyObj<AgencyService>;
  let mockDiscoveryService: jasmine.SpyObj<DiscoveryAttestationService>;
  let mockRouter: jasmine.SpyObj<Router>;

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
    mockIdentityService = jasmine.createSpyObj(
      'IdentityService',
      ['getCurrentHuman', 'updateProfile'],
      {
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
      }
    );

    mockAgencyService = jasmine.createSpyObj('AgencyService', [], {
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
    });

    mockDiscoveryService = jasmine.createSpyObj('DiscoveryAttestationService', ['toggleFeatured'], {
      featuredResults: signal([]),
      results: signal([]),
    });

    mockRouter = jasmine.createSpyObj('Router', ['navigate']);

    mockIdentityService.getCurrentHuman.and.returnValue(Promise.resolve(mockProfile));
    mockIdentityService.updateProfile.and.returnValue(Promise.resolve(mockProfile));

    await TestBed.configureTestingModule({
      imports: [ProfileComponent],
      providers: [
        { provide: IdentityService, useValue: mockIdentityService },
        { provide: AgencyService, useValue: mockAgencyService },
        { provide: DiscoveryAttestationService, useValue: mockDiscoveryService },
        {
          provide: DoorwayRegistryService,
          useValue: {
            doorwaysWithHealth: signal([]),
            selected: signal(null),
            selectedUrl: signal(null),
            hasSelection: signal(false),
            selectDoorwayByUrl: jasmine.createSpy('selectDoorwayByUrl'),
            validateDoorway: jasmine
              .createSpy('validateDoorway')
              .and.returnValue(Promise.resolve({ isValid: false })),
          },
        },
        {
          provide: TauriAuthService,
          useValue: {
            isTauri: signal(false),
            graduationStatus: signal('idle'),
            graduationError: signal(''),
            isGraduationEligible: signal(false),
            confirmStewardship: jasmine
              .createSpy('confirmStewardship')
              .and.returnValue(Promise.resolve(false)),
          },
        },
        {
          provide: HolochainClientService,
          useValue: {
            getDisplayInfo: () => ({
              state: 'connected',
              mode: 'doorway',
              adminUrl: 'ws://localhost:4444',
              appUrl: 'ws://localhost:4445',
              agentPubKey: 'test-key',
              dnaHash: 'test-dna',
              connectedAt: new Date(),
              hasStoredCredentials: true,
              error: null,
            }),
            disconnect: jasmine.createSpy('disconnect').and.returnValue(Promise.resolve()),
            connect: jasmine.createSpy('connect').and.returnValue(Promise.resolve()),
          },
        },
        {
          provide: SessionHumanService,
          useValue: {
            prepareMigration: () => null,
          },
        },
        {
          provide: ActivatedRoute,
          useValue: {
            fragment: of(null),
          },
        },
        { provide: Router, useValue: mockRouter },
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
      mockIdentityService.getCurrentHuman.and.returnValue(
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
        jasmine.objectContaining({
          displayName: 'Updated Name',
          bio: 'Updated bio',
        })
      );
      expect(component.isEditing()).toBe(false);
      expect(component.successMessage()).toContain('Profile updated');
    });

    it('should handle save error', async () => {
      mockIdentityService.updateProfile.and.returnValue(Promise.reject(new Error('Save failed')));
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
