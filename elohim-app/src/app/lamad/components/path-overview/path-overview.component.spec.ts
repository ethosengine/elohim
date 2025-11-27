import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute, Router } from '@angular/router';
import { of, throwError, BehaviorSubject } from 'rxjs';
import { PathOverviewComponent } from './path-overview.component';
import { PathService } from '../../services/path.service';
import { AgentService } from '../../services/agent.service';
import { LearningPath } from '../../models/learning-path.model';
import { AgentProgress } from '../../models/agent.model';

describe('PathOverviewComponent', () => {
  let component: PathOverviewComponent;
  let fixture: ComponentFixture<PathOverviewComponent>;
  let pathService: jasmine.SpyObj<PathService>;
  let agentService: jasmine.SpyObj<AgentService>;
  let router: jasmine.SpyObj<Router>;
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

  beforeEach(async () => {
    const pathServiceSpy = jasmine.createSpyObj('PathService', ['getPath', 'getAccessibleSteps']);
    const agentServiceSpy = jasmine.createSpyObj('AgentService', ['getProgressForPath']);
    const routerSpy = jasmine.createSpyObj('Router', ['navigate']);

    paramsSubject = new BehaviorSubject({ pathId: 'test-path' });

    await TestBed.configureTestingModule({
      imports: [PathOverviewComponent],
      providers: [
        { provide: PathService, useValue: pathServiceSpy },
        { provide: AgentService, useValue: agentServiceSpy },
        { provide: Router, useValue: routerSpy },
        {
          provide: ActivatedRoute,
          useValue: { params: paramsSubject.asObservable() }
        }
      ]
    }).compileComponents();

    pathService = TestBed.inject(PathService) as jasmine.SpyObj<PathService>;
    agentService = TestBed.inject(AgentService) as jasmine.SpyObj<AgentService>;
    router = TestBed.inject(Router) as jasmine.SpyObj<Router>;

    pathService.getPath.and.returnValue(of(mockPath));
    agentService.getProgressForPath.and.returnValue(of(mockProgress));
    pathService.getAccessibleSteps.and.returnValue(of(mockAccessibleSteps));

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

  it('should calculate completion percentage', () => {
    fixture.detectChanges();

    expect(component.getCompletionPercentage()).toBe(50); // 2/4 = 50%
  });

  it('should return 0 completion if no progress', () => {
    component.progress = null;
    expect(component.getCompletionPercentage()).toBe(0);
  });

  it('should check if step is completed', () => {
    fixture.detectChanges();

    expect(component.isStepCompleted(0)).toBe(true);
    expect(component.isStepCompleted(1)).toBe(true);
    expect(component.isStepCompleted(2)).toBe(false);
  });

  it('should check if step is accessible', () => {
    fixture.detectChanges();

    expect(component.isStepAccessible(0)).toBe(true);
    expect(component.isStepAccessible(1)).toBe(true);
    expect(component.isStepAccessible(2)).toBe(true);
    expect(component.isStepAccessible(3)).toBe(false);
  });

  it('should check if step is locked', () => {
    fixture.detectChanges();

    expect(component.isStepLocked(0)).toBe(false);
    expect(component.isStepLocked(3)).toBe(true);
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

  it('should get step status class for completed step', () => {
    fixture.detectChanges();

    expect(component.getStepStatusClass(0)).toBe('completed');
  });

  it('should get step status class for locked step', () => {
    fixture.detectChanges();

    expect(component.getStepStatusClass(3)).toBe('locked');
  });

  it('should get step status class for current step', () => {
    fixture.detectChanges();

    expect(component.getStepStatusClass(2)).toBe('current');
  });

  it('should get step status class for accessible step', () => {
    fixture.detectChanges();

    // Step 1 is completed, so let's modify progress to make it accessible but not completed
    component.progress = {
      agentId: 'test-agent',
      pathId: 'test-path',
      currentStepIndex: 0,
      completedStepIndices: [0],
      startedAt: '2025-01-01T00:00:00.000Z',
      lastActivityAt: '2025-01-01T00:00:00.000Z',
      stepAffinity: {},
      stepNotes: {},
      reflectionResponses: {},
      attestationsEarned: []
    };

    expect(component.getStepStatusClass(1)).toBe('accessible');
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
