import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter, ActivatedRoute, Router } from '@angular/router';
import { of, throwError, BehaviorSubject } from 'rxjs';
import { PathOverviewComponent } from './path-overview.component';
import { PathService } from '../../services/path.service';
import { PathAdaptationService } from '../../quiz-engine/services/path-adaptation.service';
import { AgentService } from '@app/elohim/services/agent.service';
import { SeoService } from '../../../services/seo.service';
import { ContentMasteryService } from '../../services/content-mastery.service';
import { LearningPath } from '../../models';
import { AgentProgress } from '@app/elohim/models/agent.model';
import { vi, Mock } from 'vitest';

describe('PathOverviewComponent', () => {
  let component: PathOverviewComponent;
  let fixture: ComponentFixture<PathOverviewComponent>;
  let pathService: any;
  let agentService: any;
  let router: Router;
  let paramsSubject: BehaviorSubject<any>;

  const mockPath: LearningPath = {
    id: 'test-path',
    version: '1.0.0',
    title: 'Test Learning Path',
    description: 'A comprehensive learning path',
    purpose: 'Testing',
    createdBy: 'test-user',
    contributors: [],
    createdAt: '2025-01-01T00:00:00.000Z',
    updatedAt: '2025-01-01T00:00:00.000Z',
    difficulty: 'intermediate',
    estimatedDuration: '2 hours',
    tags: ['test'],
    visibility: 'public',
    steps: [
      {
        order: 0,
        resourceId: 'node-1',
        stepTitle: 'Step 1',
        stepNarrative: 'First step',
        learningObjectives: [],
        optional: false,
        completionCriteria: [],
      },
      {
        order: 1,
        resourceId: 'node-2',
        stepTitle: 'Step 2',
        stepNarrative: 'Second step',
        learningObjectives: [],
        optional: false,
        completionCriteria: [],
      },
      {
        order: 2,
        resourceId: 'node-3',
        stepTitle: 'Step 3',
        stepNarrative: 'Third step',
        learningObjectives: [],
        optional: true,
        completionCriteria: [],
      },
      {
        order: 3,
        resourceId: 'node-4',
        stepTitle: 'Step 4',
        stepNarrative: 'Fourth step',
        learningObjectives: [],
        optional: false,
        completionCriteria: [],
      },
    ],
  };

  const mockProgress: AgentProgress = {
    agentId: 'test-agent',
    pathId: 'test-path',
    currentStepIndex: 1,
    completedStepIndices: [0, 1],
    startedAt: '2025-01-01T00:00:00.000Z',
    lastActivityAt: '2025-01-01T00:00:00.000Z',
    stepAffinity: {},
    stepNotes: {},
    reflectionResponses: {},
    attestationsEarned: [],
  };

  const mockAccessibleSteps = [0, 1, 2];

  const mockCompletion = {
    totalSteps: 4,
    completedSteps: 2,
    totalUniqueContent: 4,
    completedUniqueContent: 2,
    contentCompletionPercentage: 50,
    stepCompletionPercentage: 50,
    sharedContentCompleted: 0,
  };

  const mockContent: any = {
    id: 'node-1',
    contentType: 'concept',
    title: 'Test Concept',
    description: 'A test concept',
    content: 'Test content',
    contentFormat: 'markdown',
    tags: [],
    relatedNodeIds: [],
    metadata: {},
  };

  const mockStepsMetadata: any[] = [
    {
      step: mockPath.steps[0],
      stepIndex: 0,
      isCompleted: true,
      completedInOtherPath: false,
      masteryLevel: 2,
      masteryTier: 'practiced',
    },
    {
      step: mockPath.steps[1],
      stepIndex: 1,
      isCompleted: true,
      completedInOtherPath: false,
      masteryLevel: 2,
      masteryTier: 'practiced',
    },
    {
      step: mockPath.steps[2],
      stepIndex: 2,
      isCompleted: false,
      completedInOtherPath: false,
      masteryLevel: 0,
      masteryTier: 'unseen',
    },
    {
      step: mockPath.steps[3],
      stepIndex: 3,
      isCompleted: false,
      completedInOtherPath: false,
      masteryLevel: 0,
      masteryTier: 'unseen',
    },
  ];

  beforeEach(async () => {
    localStorage.clear();

    const pathServiceSpy = {
      getPath: vi.fn(),
      getAccessibleSteps: vi.fn(),
      getAccessCheckResults: vi.fn(),
      getPathCompletionByContent: vi.fn(),
      getChapterSummariesWithContent: vi.fn(),
      getAllStepsMetadata: vi.fn(),
      getConceptProgressForPath: vi.fn(),
      getChapterFirstStep: vi.fn(),
    };
    const agentServiceSpy = {
      getProgressForPath: vi.fn(),
      getCurrentAgentId: vi.fn().mockReturnValue('test-agent'),
    };
    const adaptationServiceSpy = {
      getRecommendations$: vi.fn().mockReturnValue(of([])),
      dismissRecommendation: vi.fn(),
    };
    const seoServiceSpy = {
      updateForPath: vi.fn(),
      updateSeo: vi.fn(),
      setTitle: vi.fn(),
    };
    const contentMasteryServiceSpy = {
      getMasteryLevelSync: vi.fn(),
    };
    contentMasteryServiceSpy.getMasteryLevelSync.mockReturnValue('not_started');

    paramsSubject = new BehaviorSubject({ pathId: 'test-path' });

    await TestBed.configureTestingModule({
      imports: [PathOverviewComponent],
      providers: [
        provideRouter([]),
        { provide: PathService, useValue: pathServiceSpy },
        { provide: AgentService, useValue: agentServiceSpy },
        { provide: PathAdaptationService, useValue: adaptationServiceSpy },
        { provide: SeoService, useValue: seoServiceSpy },
        { provide: ContentMasteryService, useValue: contentMasteryServiceSpy },
        {
          provide: ActivatedRoute,
          useValue: { params: paramsSubject.asObservable() },
        },
      ],
    }).compileComponents();

    pathService = TestBed.inject(PathService) as { [K in keyof PathService]?: Mock };
    agentService = TestBed.inject(AgentService) as { [K in keyof AgentService]?: Mock };
    router = TestBed.inject(Router);
    vi.spyOn(router, 'navigate');

    pathService.getPath.mockReturnValue(of(mockPath));
    agentService.getProgressForPath.mockReturnValue(of(mockProgress));
    pathService.getAccessibleSteps.mockReturnValue(of(mockAccessibleSteps));
    pathService.getAccessCheckResults.mockReturnValue(
      of(
        new Map<number, any>([
          [0, { accessible: true, accessType: 'sequential' }],
          [1, { accessible: true, accessType: 'sequential' }],
          [2, { accessible: true, accessType: 'sequential' }],
          [3, { accessible: false }],
        ])
      )
    );
    pathService.getPathCompletionByContent.mockReturnValue(of(mockCompletion));
    pathService.getChapterSummariesWithContent.mockReturnValue(of([]));
    pathService.getAllStepsMetadata.mockReturnValue(of(mockStepsMetadata));
    pathService.getConceptProgressForPath.mockReturnValue(of([]));

    fixture = TestBed.createComponent(PathOverviewComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should load path, progress, and accessible steps on init', () => {
    fixture.detectChanges();

    expect(pathService.getPath).toHaveBeenCalledWith('test-path');
    expect(agentService.getProgressForPath).toHaveBeenCalledWith('test-path');
    expect(pathService.getAccessibleSteps).toHaveBeenCalledWith('test-path');
    expect(component.path).toEqual(mockPath);
    expect(component.progress).toEqual(mockProgress);
    expect(component.accessibleSteps).toEqual(mockAccessibleSteps);
    expect(component.isLoading).toBe(false);
  });

  it('should handle load error', () => {
    pathService.getPath.mockReturnValue(throwError(() => new Error('Network error')));

    fixture.detectChanges();

    expect(component.isLoading).toBe(false);
    expect(component.error).toBe('Network error');
  });

  it('should calculate current step index from progress', () => {
    fixture.detectChanges();

    expect(component.getCurrentStepIndex()).toBe(2); // max(0,1) + 1 = 2
  });

  it('should return 0 as current step if no progress', () => {
    agentService.getProgressForPath.mockReturnValue(of(null as any));
    fixture.detectChanges();

    expect(component.getCurrentStepIndex()).toBe(0);
  });

  it('should not exceed total steps when calculating current step', () => {
    const progressAllComplete: AgentProgress = {
      agentId: 'test-agent',
      pathId: 'test-path',
      currentStepIndex: 3,
      completedStepIndices: [0, 1, 2, 3],
      startedAt: '2025-01-01T00:00:00.000Z',
      lastActivityAt: '2025-01-01T00:00:00.000Z',
      stepAffinity: {},
      stepNotes: {},
      reflectionResponses: {},
      attestationsEarned: [],
    };
    agentService.getProgressForPath.mockReturnValue(of(progressAllComplete));
    fixture.detectChanges();

    expect(component.getCurrentStepIndex()).toBe(3); // min(4, 3) = 3
  });

  it('should check if path has been started', () => {
    fixture.detectChanges();
    expect(component.hasStarted()).toBe(true);
  });

  it('should return false for hasStarted if no progress', () => {
    component.progress = null;
    expect(component.hasStarted()).toBe(false);
  });

  it('should return false for hasStarted if no completed steps', () => {
    component.progress = {
      agentId: 'test-agent',
      pathId: 'test-path',
      currentStepIndex: 0,
      completedStepIndices: [],
      startedAt: '2025-01-01T00:00:00.000Z',
      lastActivityAt: '2025-01-01T00:00:00.000Z',
      stepAffinity: {},
      stepNotes: {},
      reflectionResponses: {},
      attestationsEarned: [],
    };
    expect(component.hasStarted()).toBe(false);
  });

  it('should check if path is completed (only required steps)', () => {
    const progressAllRequired: AgentProgress = {
      agentId: 'test-agent',
      pathId: 'test-path',
      currentStepIndex: 3,
      completedStepIndices: [0, 1, 3], // Missing optional step 2
      startedAt: '2025-01-01T00:00:00.000Z',
      lastActivityAt: '2025-01-01T00:00:00.000Z',
      stepAffinity: {},
      stepNotes: {},
      reflectionResponses: {},
      attestationsEarned: [],
    };
    agentService.getProgressForPath.mockReturnValue(of(progressAllRequired));
    fixture.detectChanges();

    expect(component.isCompleted()).toBe(true);
  });

  it('should return false for isCompleted if required steps missing', () => {
    fixture.detectChanges();
    expect(component.isCompleted()).toBe(false); // Missing step 3 (required)
  });

  it('should calculate completion percentage from pathCompletion', () => {
    fixture.detectChanges();

    expect(component.getCompletionPercentage()).toBe(50); // from mockCompletion
  });

  it('should return 0 completion if no pathCompletion', () => {
    component.pathCompletion = null;
    expect(component.getCompletionPercentage()).toBe(0);
  });

  it('should begin journey at step 0', () => {
    fixture.detectChanges();
    component.beginJourney();

    expect(router.navigate).toHaveBeenCalledWith(['/lamad/path', 'test-path', 'step', 0]);
  });

  it('should continue journey from current step', () => {
    fixture.detectChanges();
    component.continueJourney();

    expect(router.navigate).toHaveBeenCalledWith(['/lamad/path', 'test-path', 'step', 2]);
  });

  it('should navigate to specific accessible step', () => {
    fixture.detectChanges();
    component.goToStep(1);

    expect(router.navigate).toHaveBeenCalledWith(['/lamad/path', 'test-path', 'step', 1]);
  });

  it('should not navigate to locked step', () => {
    fixture.detectChanges();
    component.goToStep(3);

    expect(router.navigate).not.toHaveBeenCalled();
  });

  it('should navigate to home', () => {
    component.goHome();

    expect(router.navigate).toHaveBeenCalledWith(['/lamad']);
  });

  it('should get difficulty display', () => {
    fixture.detectChanges();

    expect(component.getDifficultyDisplay()).toBe('Intermediate');
  });

  it('should display advanced difficulty', () => {
    const advancedPath: LearningPath = {
      ...mockPath,
      difficulty: 'advanced',
    };
    pathService.getPath.mockReturnValue(of(advancedPath));
    fixture.detectChanges();

    expect(component.getDifficultyDisplay()).toBe('Advanced');
  });

  it('should cleanup on destroy', () => {
    fixture.detectChanges();

    vi.spyOn(component['destroy$'], 'next');
    vi.spyOn(component['destroy$'], 'complete');

    component.ngOnDestroy();

    expect(component['destroy$'].next).toHaveBeenCalled();
    expect(component['destroy$'].complete).toHaveBeenCalled();
  });

  it('should reload when route params change', () => {
    fixture.detectChanges();
    pathService.getPath.mockClear();

    paramsSubject.next({ pathId: 'another-path' });

    expect(component.pathId).toBe('another-path');
    expect(pathService.getPath).toHaveBeenCalledWith('another-path');
  });
});
