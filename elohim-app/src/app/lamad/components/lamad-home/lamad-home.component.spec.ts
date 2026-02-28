import { ComponentFixture, TestBed } from '@angular/core/testing';
import { Router } from '@angular/router';
import { of, throwError } from 'rxjs';
import { LamadHomeComponent } from './lamad-home.component';
import { PathService } from '../../services/path.service';
import { PathFilterService } from '../../services/path-filter.service';
import { ProfileService } from '@app/elohim/services/profile.service';
import { AgentService } from '@app/elohim/services/agent.service';
import { IdentityService } from '@app/imagodei/services/identity.service';
import { PathIndex, PathIndexEntry } from '../../models/learning-path.model';
import { vi, Mock } from 'vitest';

describe('LamadHomeComponent', () => {
  let component: LamadHomeComponent;
  let fixture: ComponentFixture<LamadHomeComponent>;
  let pathService: any;
  let pathFilterService: any;
  let router: any;
  let profileService: any;
  let agentService: any;
  let identityService: any;
  let localStorageMock: { [key: string]: string };

  const mockPaths: PathIndexEntry[] = [
    {
      id: 'elohim-protocol',
      title: 'Elohim Protocol',
      description: 'Learn the Elohim Protocol',
      difficulty: 'beginner',
      estimatedDuration: '2 hours',
      stepCount: 5,
      tags: ['protocol', 'intro'],
    },
    {
      id: 'learning-platform',
      title: 'Learning Platform',
      description: 'Understanding Lamad',
      difficulty: 'intermediate',
      estimatedDuration: '1 hour',
      stepCount: 3,
      tags: ['learning', 'platform'],
    },
  ];

  const mockPathIndex: PathIndex = {
    lastUpdated: '2025-01-01T00:00:00.000Z',
    totalCount: 2,
    paths: mockPaths,
  };

  beforeEach(async () => {
    const pathServiceSpy = {
    listPaths: vi.fn(),
  };
    const pathFilterServiceSpy = {
    getFeaturedPaths: vi.fn(),
  };
    const routerSpy = {
    navigate: vi.fn(),
  };
    const profileServiceSpy = {
    getCurrentFocus: vi.fn(),
  };
    const agentServiceSpy = {
    getCurrentAgentId: vi.fn(),
    getAgentProgress: vi.fn(),
  };
    const identityServiceSpy = {
    mode: vi.fn(),
  };
    // mode() returns 'anonymous' by default (unauthenticated)
    identityServiceSpy.mode.mockReturnValue('anonymous');

    // Mock localStorage
    localStorageMock = {};
    vi.spyOn(localStorage, 'getItem').mockImplementation((key: string) => {
      return localStorageMock[key] || null;
    });
    vi.spyOn(localStorage, 'setItem').mockImplementation((key: string, value: string) => {
      localStorageMock[key] = value;
    });

    await TestBed.configureTestingModule({
      imports: [LamadHomeComponent],
      providers: [
        { provide: PathService, useValue: pathServiceSpy },
        { provide: PathFilterService, useValue: pathFilterServiceSpy },
        { provide: Router, useValue: routerSpy },
        { provide: ProfileService, useValue: profileServiceSpy },
        { provide: AgentService, useValue: agentServiceSpy },
        { provide: IdentityService, useValue: identityServiceSpy },
      ],
    }).compileComponents();

    pathService = TestBed.inject(PathService) as { [K in keyof PathService]?: Mock };
    pathFilterService = TestBed.inject(PathFilterService) as { [K in keyof PathFilterService]?: Mock };
    router = TestBed.inject(Router) as { [K in keyof Router]?: Mock };
    profileService = TestBed.inject(ProfileService) as { [K in keyof ProfileService]?: Mock };
    agentService = TestBed.inject(AgentService) as { [K in keyof AgentService]?: Mock };
    identityService = TestBed.inject(IdentityService) as { [K in keyof IdentityService]?: Mock };

    pathService.listPaths.mockReturnValue(of(mockPathIndex));
    pathFilterService.getFeaturedPaths.mockImplementation((paths: PathIndexEntry[]) => paths);
    profileService.getCurrentFocus.mockReturnValue(of([]));
    agentService.getCurrentAgentId.mockReturnValue('test-agent');
    agentService.getAgentProgress.mockReturnValue(of([]));

    fixture = TestBed.createComponent(LamadHomeComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should load paths on init', () => {
    fixture.detectChanges();

    expect(pathService.listPaths).toHaveBeenCalled();
    expect(component.paths.length).toBe(2);
    expect(component.isLoading).toBe(false);
  });

  it('should set featured path to elohim-protocol if available', () => {
    fixture.detectChanges();

    expect(component.featuredPath?.id).toBe('elohim-protocol');
  });

  it('should set featured path to first path if elohim-protocol not found', () => {
    const pathsWithoutElohim: PathIndex = {
      lastUpdated: '2025-01-01T00:00:00.000Z',
      totalCount: 1,
      paths: [mockPaths[1]],
    };
    pathService.listPaths.mockReturnValue(of(pathsWithoutElohim));

    fixture.detectChanges();

    expect(component.featuredPath?.id).toBe('learning-platform');
  });

  it('should handle empty paths array', () => {
    pathService.listPaths.mockReturnValue(
      of({
        lastUpdated: '2025-01-01T00:00:00.000Z',
        totalCount: 0,
        paths: [],
      })
    );

    fixture.detectChanges();

    expect(component.paths.length).toBe(0);
    expect(component.featuredPath).toBeNull();
  });

  it('should handle path loading error', () => {
    pathService.listPaths.mockReturnValue(throwError(() => new Error('Network error')));

    fixture.detectChanges();

    expect(component.isLoading).toBe(false);
    expect(component.error).toBe('Unable to load learning paths');
  });

  it('should navigate to path on goToPath', () => {
    component.goToPath('test-path');

    expect(router.navigate).toHaveBeenCalledWith(['/lamad/path', 'test-path']);
  });

  it('should start featured path at step 0', () => {
    fixture.detectChanges();
    component.startFeaturedPath();

    expect(router.navigate).toHaveBeenCalledWith(['/lamad/path', 'elohim-protocol', 'step', 0]);
  });

  it('should not navigate if no featured path', () => {
    component.featuredPath = null;
    component.startFeaturedPath();

    expect(router.navigate).not.toHaveBeenCalled();
  });

  it('should navigate to explore view', () => {
    component.goToExplore();

    expect(router.navigate).toHaveBeenCalledWith(['/lamad/explore']);
  });

  it('should navigate to search', () => {
    component.goToSearch();

    expect(router.navigate).toHaveBeenCalledWith(['/lamad/search']);
  });

  it('should navigate to dashboard', () => {
    component.goToDashboard();

    expect(router.navigate).toHaveBeenCalledWith(['/lamad/me']);
  });

  it('should get difficulty class', () => {
    expect(component.getDifficultyClass('advanced')).toBe('advanced');
    expect(component.getDifficultyClass('')).toBe('beginner');
  });

  it('should format difficulty for display', () => {
    expect(component.formatDifficulty('beginner')).toBe('Beginner');
    expect(component.formatDifficulty('intermediate')).toBe('Intermediate');
    expect(component.formatDifficulty('advanced')).toBe('Advanced');
    expect(component.formatDifficulty('unknown')).toBe('unknown');
  });

  it('should set view mode to paths', () => {
    component.setViewMode('paths');

    expect(component.viewMode).toBe('paths');
    expect(localStorage.setItem).toHaveBeenCalledWith('lamad-view-mode', 'paths');
  });

  it('should set view mode to explore and navigate', () => {
    component.setViewMode('explore');

    expect(component.viewMode).toBe('explore');
    expect(localStorage.setItem).toHaveBeenCalledWith('lamad-view-mode', 'explore');
    expect(router.navigate).toHaveBeenCalledWith(['/lamad/explore']);
  });

  it('should load saved view mode from localStorage', () => {
    localStorageMock['lamad-view-mode'] = 'explore';

    const newComponent = new LamadHomeComponent(
      pathService,
      pathFilterService,
      router,
      profileService,
      agentService,
      identityService
    );
    expect(newComponent.viewMode).toBe('explore');
  });

  it('should default to paths mode if no saved preference', () => {
    const newComponent = new LamadHomeComponent(
      pathService,
      pathFilterService,
      router,
      profileService,
      agentService,
      identityService
    );
    expect(newComponent.viewMode).toBe('paths');
  });

  it('should cleanup on destroy', () => {
    fixture.detectChanges();

    vi.spyOn(component['destroy$'], 'next');
    vi.spyOn(component['destroy$'], 'complete');

    component.ngOnDestroy();

    expect(component['destroy$'].next).toHaveBeenCalled();
    expect(component['destroy$'].complete).toHaveBeenCalled();
  });

  it('should return null for getPathProgress when no progress exists', () => {
    expect(component.getPathProgress('non-existent-path')).toBeNull();
  });

  it('should return progress percentage from pathProgressMap', () => {
    component.pathProgressMap.set('test-path', 50);
    expect(component.getPathProgress('test-path')).toBe(50);
  });

  it('should populate pathProgressMap from agent progress', () => {
    // Set identity mode to 'hosted' so agent progress is fetched (requires network authentication)
    identityService.mode.mockReturnValue('hosted');

    const mockAgentProgress = [
      {
        agentId: 'test-agent',
        pathId: 'elohim-protocol',
        currentStepIndex: 2,
        completedStepIndices: [0, 1],
        startedAt: '2025-01-01T00:00:00.000Z',
        lastActivityAt: '2025-01-02T00:00:00.000Z',
        stepAffinity: {},
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: [],
      },
      {
        agentId: 'test-agent',
        pathId: 'learning-platform',
        currentStepIndex: 3,
        completedStepIndices: [0, 1, 2],
        startedAt: '2025-01-01T00:00:00.000Z',
        lastActivityAt: '2025-01-03T00:00:00.000Z',
        completedAt: '2025-01-03T00:00:00.000Z',
        stepAffinity: {},
        stepNotes: {},
        reflectionResponses: {},
        attestationsEarned: [],
      },
    ];
    agentService.getAgentProgress.mockReturnValue(of(mockAgentProgress));

    fixture.detectChanges();

    // elohim-protocol has 5 steps, 2 completed = 40%
    expect(component.pathProgressMap.get('elohim-protocol')).toBe(40);
    // learning-platform is completed = 100%
    expect(component.pathProgressMap.get('learning-platform')).toBe(100);
  });
});
