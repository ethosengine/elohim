import { ComponentFixture, TestBed } from '@angular/core/testing';
import { Router, provideRouter } from '@angular/router';
import { signal } from '@angular/core';

import { of, BehaviorSubject } from 'rxjs';

import { ProfileService } from '@app/elohim/services/profile.service';
import { IdentityService } from '@app/imagodei/services/identity.service';
import { SessionHumanService } from '@app/imagodei/services/session-human.service';

import { ContentMasteryService } from '../../services/content-mastery.service';
import { MasteryStatsService } from '../../services/mastery-stats.service';
import { ProfilePageComponent } from './profile-page.component';
import { vi, Mock } from 'vitest';

describe('ProfilePageComponent', () => {
  let component: ProfilePageComponent;
  let fixture: ComponentFixture<ProfilePageComponent>;
  let profileServiceSpy: any;
  let identityServiceSpy: any;
  let sessionHumanSpy: any;
  let masteryServiceSpy: any;
  let masteryStatsSpy: any;
  let router: Router;

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

    profileServiceSpy = {
      getResumePoint: vi.fn(),
      getPathsOverview: vi.fn(),
      getTimeline: vi.fn(),
    };
    // Mock ProfileService methods with observables
    profileServiceSpy.getResumePoint.mockReturnValue(of(null));
    profileServiceSpy.getPathsOverview.mockReturnValue(
      of({ inProgress: [], completed: [], suggested: [] })
    );
    profileServiceSpy.getTimeline.mockReturnValue(of([]));

    identityServiceSpy = {
      updateProfile: vi.fn(),
      mode: signal('session'),
      currentHuman: signal(null),
      profile: signal(null),
      displayName: signal('Test User'),
    };

    sessionHumanSpy = {
      session$: mockSession$,
      getAllPathProgress: vi.fn(),
      getActivityHistory: vi.fn(),
    };
    sessionHumanSpy.getAllPathProgress.mockReturnValue([]);
    sessionHumanSpy.getActivityHistory.mockReturnValue([]);

    masteryServiceSpy = {
      getMasteryStats: vi.fn(),
    };
    masteryServiceSpy.getMasteryStats.mockReturnValue(
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

    masteryStatsSpy = {
      recordDailyEngagement: vi.fn(),
      learnerProfile$: new BehaviorSubject(null).asObservable(),
    };

    await TestBed.configureTestingModule({
      imports: [ProfilePageComponent],
      providers: [
        { provide: ProfileService, useValue: profileServiceSpy },
        { provide: IdentityService, useValue: identityServiceSpy },
        { provide: SessionHumanService, useValue: sessionHumanSpy },
        { provide: ContentMasteryService, useValue: masteryServiceSpy },
        { provide: MasteryStatsService, useValue: masteryStatsSpy },
        provideRouter([]),
      ],
    }).compileComponents();

    router = TestBed.inject(Router);
    vi.spyOn(router, 'navigate');

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

  describe('Tab Navigation', () => {
    it('should switch to paths tab', () => {
      component.setActiveTab('paths');
      expect(component.activeTab).toBe('paths');
    });

    it('should switch to timeline tab', () => {
      component.setActiveTab('timeline');
      expect(component.activeTab).toBe('timeline');
    });

    it('should refresh activity history when switching to timeline', () => {
      component.setActiveTab('timeline');
      expect(sessionHumanSpy.getActivityHistory).toHaveBeenCalled();
    });

    it('should refresh path progress when switching to paths', () => {
      component.setActiveTab('paths');
      expect(sessionHumanSpy.getAllPathProgress).toHaveBeenCalled();
    });
  });

  describe('Navigation', () => {
    it('should navigate to identity profile', () => {
      component.goToIdentityProfile();
      expect(router.navigate).toHaveBeenCalledWith(['/identity/profile']);
    });

    it('should navigate to registration', () => {
      component.onJoinNetwork();
      expect(router.navigate).toHaveBeenCalledWith(['/identity/register']);
    });
  });

  describe('Computed Signals', () => {
    it('should compute display name from session', () => {
      expect(component.displayName()).toBe('Test User');
    });

    it('should compute mode badge text', () => {
      expect(component.modeBadgeText()).toBe('Session Visitor');
    });

    it('should compute mode badge icon', () => {
      expect(component.modeBadgeIcon()).toBe('local_activity');
    });
  });
});
