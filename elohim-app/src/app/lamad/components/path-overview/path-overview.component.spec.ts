import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter, ActivatedRoute, Router } from '@angular/router';
import { of, throwError, BehaviorSubject } from 'rxjs';
import { PathOverviewComponent } from './path-overview.component';
import { PathService } from '../../services/path.service';
import { AgentService } from '@app/elohim/services/agent.service';
import { SeoService } from '../../../services/seo.service';
import { LearningPath } from '../../models';
import { AgentProgress } from '@app/elohim/models/agent.model';

describe('PathOverviewComponent', () => {
  let component: PathOverviewComponent;
  let fixture: ComponentFixture<PathOverviewComponent>;
  let pathService: jasmine.SpyObj<PathService>;
  let agentService: jasmine.SpyObj<AgentService>;
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
        completionCriteria: []
      },
      {
        order: 1,
        resourceId: 'node-2',
        stepTitle: 'Step 2',
        stepNarrative: 'Second step',
        learningObjectives: [],
        optional: false,
        completionCriteria: []
      },
      {
        order: 2,
        resourceId: 'node-3',
        stepTitle: 'Step 3',
        stepNarrative: 'Third step',
        learningObjectives: [],
        optional: true,
        completionCriteria: []
      },
      {
        order: 3,
        resourceId: 'node-4',
        stepTitle: 'Step 4',
        stepNarrative: 'Fourth step',
        learningObjectives: [],
        optional: false,
        completionCriteria: []
      }
    ]
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
    attestationsEarned: []
  };

  const mockAccessibleSteps = [0, 1, 2];

  const mockCompletion = {
    totalSteps: 4,
    completedSteps: 2,
    totalUniqueContent: 4,
    completedUniqueContent: 2,
    contentCompletionPercentage: 50,
    stepCompletionPercentage: 50,
    sharedContentCompleted: 0
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
    metadata: {}
  };

  const mockStepsMetadata: any[] = [
    { step: mockPath.steps[0], stepIndex: 0, isCompleted: true, completedInOtherPath: false, masteryLevel: 2, masteryTier: 'practiced' },
    { step: mockPath.steps[1], stepIndex: 1, isCompleted: true, completedInOtherPath: false, masteryLevel: 2, masteryTier: 'practiced' },
    { step: mockPath.steps[2], stepIndex: 2, isCompleted: false, completedInOtherPath: false, masteryLevel: 0, masteryTier: 'unseen' },
    { step: mockPath.steps[3], stepIndex: 3, isCompleted: false, completedInOtherPath: false, masteryLevel: 0, masteryTier: 'unseen' }
  ];

  beforeEach(async () => {
    const pathServiceSpy = jasmine.createSpyObj('PathService', [
      'getPath',
      'getAccessibleSteps',
      'getPathCompletionByContent',
      'getChapterSummariesWithContent',
      'getAllStepsMetadata',
      'getConceptProgressForPath',
      'getChapterFirstStep'
    ]);
    const agentServiceSpy = jasmine.createSpyObj('AgentService', ['getProgressForPath']);
    const seoServiceSpy = jasmine.createSpyObj('SeoService', ['updateForPath', 'updateSeo', 'setTitle']);

    paramsSubject = new BehaviorSubject({ pathId: 'test-path' });

    await TestBed.configureTestingModule({
      imports: [PathOverviewComponent],
      providers: [
        provideRouter([]),
        { provide: PathService, useValue: pathServiceSpy },
        { provide: AgentService, useValue: agentServiceSpy },
        { provide: SeoService, useValue: seoServiceSpy },
        {
          provide: ActivatedRoute,
          useValue: { params: paramsSubject.asObservable() }
        }
      ]
    }).compileComponents();

    pathService = TestBed.inject(PathService) as jasmine.SpyObj<PathService>;
    agentService = TestBed.inject(AgentService) as jasmine.SpyObj<AgentService>;
    router = TestBed.inject(Router);
    spyOn(router, 'navigate');

    pathService.getPath.and.returnValue(of(mockPath));
    agentService.getProgressForPath.and.returnValue(of(mockProgress));
    pathService.getAccessibleSteps.and.returnValue(of(mockAccessibleSteps));
    pathService.getPathCompletionByContent.and.returnValue(of(mockCompletion));
    pathService.getChapterSummariesWithContent.and.returnValue(of([]));
    pathService.getAllStepsMetadata.and.returnValue(of(mockStepsMetadata));
    pathService.getConceptProgressForPath.and.returnValue(of([]));

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
    pathService.getPath.and.returnValue(throwError(() => new Error('Network error')));

    fixture.detectChanges();

    expect(component.isLoading).toBe(false);
    expect(component.error).toBe('Network error');
  });

  it('should calculate current step index from progress', () => {
    fixture.detectChanges();

    expect(component.getCurrentStepIndex()).toBe(2); // max(0,1) + 1 = 2
  });

  it('should return 0 as current step if no progress', () => {
    agentService.getProgressForPath.and.returnValue(of(null as any));
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
      attestationsEarned: []
    };
    agentService.getProgressForPath.and.returnValue(of(progressAllComplete));
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
      attestationsEarned: []
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
      attestationsEarned: []
    };
    agentService.getProgressForPath.and.returnValue(of(progressAllRequired));
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
      difficulty: 'advanced'
    };
    pathService.getPath.and.returnValue(of(advancedPath));
    fixture.detectChanges();

    expect(component.getDifficultyDisplay()).toBe('Advanced');
  });

  it('should cleanup on destroy', () => {
    fixture.detectChanges();

    spyOn(component['destroy$'], 'next');
    spyOn(component['destroy$'], 'complete');

    component.ngOnDestroy();

    expect(component['destroy$'].next).toHaveBeenCalled();
    expect(component['destroy$'].complete).toHaveBeenCalled();
  });

  it('should reload when route params change', () => {
    fixture.detectChanges();
    pathService.getPath.calls.reset();

    paramsSubject.next({ pathId: 'another-path' });

    expect(component.pathId).toBe('another-path');
    expect(pathService.getPath).toHaveBeenCalledWith('another-path');
  });
});
