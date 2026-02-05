import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute } from '@angular/router';
import { ProfilePageComponent } from './profile-page.component';
import { ProfileService } from '@app/elohim/services/profile.service';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { IdentityService } from '@app/imagodei/services/identity.service';
import { SessionHumanService } from '@app/imagodei/services/session-human.service';
import { AgencyService } from '@app/imagodei/services/agency.service';
import { ContentMasteryService } from '../../services/content-mastery.service';
import { signal } from '@angular/core';
import { of, BehaviorSubject } from 'rxjs';

describe('ProfilePageComponent', () => {
  let component: ProfilePageComponent;
  let fixture: ComponentFixture<ProfilePageComponent>;
  let routerSpy: jasmine.SpyObj<ActivatedRoute>;
  let profileServiceSpy: jasmine.SpyObj<ProfileService>;
  let holochainSpy: jasmine.SpyObj<HolochainClientService>;
  let identityServiceSpy: jasmine.SpyObj<IdentityService>;
  let sessionHumanSpy: jasmine.SpyObj<SessionHumanService>;
  let agencySpy: jasmine.SpyObj<AgencyService>;
  let masteryServiceSpy: jasmine.SpyObj<ContentMasteryService>;

  beforeEach(async () => {
    // Mock session observable
    const mockSession$ = new BehaviorSubject({
      sessionId: 'test-session-123',
      displayName: 'Test User',
      avatarUrl: null,
      bio: 'Test bio',
      locale: 'en',
      interests: ['testing', 'angular'],
      createdAt: new Date().toISOString(),
    });

    routerSpy = jasmine.createSpyObj('ActivatedRoute', [], {
      fragment: of('overview'),
    });
    profileServiceSpy = jasmine.createSpyObj('ProfileService', [
      'getResumePoint',
      'getPathsOverview',
      'getTimeline',
    ]);
    // Mock ProfileService methods with observables
    profileServiceSpy.getResumePoint.and.returnValue(of(null));
    profileServiceSpy.getPathsOverview.and.returnValue(
      of({ inProgress: [], completed: [], suggested: [] })
    );
    profileServiceSpy.getTimeline.and.returnValue(of([]));

    holochainSpy = jasmine.createSpyObj(
      'HolochainClientService',
      ['getDisplayInfo', 'connect', 'disconnect'],
      { isConnected: signal(true) }
    );
    holochainSpy.getDisplayInfo.and.returnValue({
      state: 'disconnected',
      mode: 'doorway',
      adminUrl: 'ws://localhost:8888',
      appUrl: 'ws://localhost:8889',
      agentPubKey: null,
      cellId: null,
      appId: 'elohim',
      dnaHash: null,
      connectedAt: null,
      hasStoredCredentials: false,
      networkSeed: null,
      error: null,
    });

    identityServiceSpy = jasmine.createSpyObj(
      'IdentityService',
      ['updateProfile'],
      {
        mode: signal('session'),
        currentHuman: signal(null),
        profile: signal(null),
        displayName: signal('Test User'),
      }
    );

    sessionHumanSpy = jasmine.createSpyObj(
      'SessionHumanService',
      [
        'getAllPathProgress',
        'getActivityHistory',
        'setDisplayName',
        'setAvatarUrl',
        'setBio',
        'setLocale',
        'setInterests',
        'prepareMigration',
        'resetSession',
      ],
      {
        session$: mockSession$,
      }
    );
    // Mock SessionHumanService methods
    sessionHumanSpy.getAllPathProgress.and.returnValue([]);
    sessionHumanSpy.getActivityHistory.and.returnValue([]);

    agencySpy = jasmine.createSpyObj('AgencyService', [], {
      agencyState: signal({
        currentStage: 'session-visitor',
        canUpgrade: false,
      }),
      stageInfo: signal({
        stage: 'session-visitor',
        title: 'Session Visitor',
        description: 'Local browser storage',
      }),
      connectionStatus: signal({
        state: 'disconnected',
        message: 'Not connected',
      }),
      canUpgrade: signal(false),
    });

    masteryServiceSpy = jasmine.createSpyObj('ContentMasteryService', [
      'getMasteryStats',
    ]);
    // Mock ContentMasteryService with realistic mastery stats
    masteryServiceSpy.getMasteryStats.and.returnValue(
      of({
        humanId: 'test-session-123',
        computedAt: new Date().toISOString(),
        totalMasteredNodes: 0,
        levelDistribution: {
          not_started: 0,
          seen: 0,
          remember: 0,
          understand: 0,
          apply: 0,
          analyze: 0,
          evaluate: 0,
          create: 0,
        },
        nodesAboveGate: 0,
        freshPercentage: 0,
        nodesNeedingRefresh: 0,
        byCategory: new Map(),
        byType: new Map(),
      })
    );

    await TestBed.configureTestingModule({
      imports: [ProfilePageComponent],
      providers: [
        { provide: ActivatedRoute, useValue: routerSpy },
        { provide: ProfileService, useValue: profileServiceSpy },
        { provide: HolochainClientService, useValue: holochainSpy },
        { provide: IdentityService, useValue: identityServiceSpy },
        { provide: SessionHumanService, useValue: sessionHumanSpy },
        { provide: AgencyService, useValue: agencySpy },
        { provide: ContentMasteryService, useValue: masteryServiceSpy },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(ProfilePageComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('Component Initialization', () => {
    it('should have activeTab property', () => {
      expect(component.activeTab).toBeDefined();
    });

    it('should initialize with overview tab', () => {
      expect(component.activeTab).toBe('overview');
    });

    it('should have reactive state signals', () => {
      expect(component.agencyState).toBeDefined();
      expect(component.stageInfo).toBeDefined();
      expect(component.connectionStatus).toBeDefined();
      expect(component.canUpgrade).toBeDefined();
    });
  });

  describe('Identity State', () => {
    it('should have isNetworkAuthenticated computed signal', () => {
      expect(component.isNetworkAuthenticated).toBeDefined();
    });

    it('should have identityMode signal', () => {
      expect(component.identityMode).toBeDefined();
    });

    it('should expose identityService for template access', () => {
      expect(component.identityService).toBeDefined();
    });
  });

  describe('Component Lifecycle', () => {
    it('should implement OnInit', () => {
      expect(component.ngOnInit).toBeDefined();
      expect(typeof component.ngOnInit).toBe('function');
    });

    it('should implement OnDestroy', () => {
      expect(component.ngOnDestroy).toBeDefined();
      expect(typeof component.ngOnDestroy).toBe('function');
    });
  });
});
