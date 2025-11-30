import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute, Router, provideRouter } from '@angular/router';
import { of, throwError, BehaviorSubject, NEVER } from 'rxjs';
import { PathNavigatorComponent } from './path-navigator.component';
import { PathService } from '../../services/path.service';
import { AgentService } from '../../services/agent.service';
import { PathStepView, LearningPath } from '../../models/learning-path.model';

describe('PathNavigatorComponent', () => {
  let component: PathNavigatorComponent;
  let fixture: ComponentFixture<PathNavigatorComponent>;
  let pathService: jasmine.SpyObj<PathService>;
  let agentService: jasmine.SpyObj<AgentService>;
  let router: Router;
  let paramsSubject: BehaviorSubject<any>;

  const mockPath: LearningPath = {
    id: 'test-path',
    version: '1.0.0',
    title: 'Test Path',
    description: 'A test learning path',
    purpose: 'Testing purposes',
    createdBy: 'test-user',
    contributors: [],
    createdAt: '2025-01-01T00:00:00.000Z',
    updatedAt: '2025-01-01T00:00:00.000Z',
    difficulty: 'beginner',
    estimatedDuration: '1 hour',
    tags: ['test'],
    visibility: 'public',
    steps: [
      {
        order: 0,
        resourceId: 'node-1',
        stepTitle: 'Step 1',
        stepNarrative: 'Narrative 1',
        learningObjectives: [],
        optional: false,
        completionCriteria: []
      },
      {
        order: 1,
        resourceId: 'node-2',
        stepTitle: 'Step 2',
        stepNarrative: 'Narrative 2',
        learningObjectives: [],
        optional: false,
        completionCriteria: []
      },
      {
        order: 2,
        resourceId: 'node-3',
        stepTitle: 'Step 3',
        stepNarrative: 'Narrative 3',
        learningObjectives: [],
        optional: false,
        completionCriteria: []
      }
    ]
  };

  const mockStepView: PathStepView = {
    step: {
      order: 1,
      resourceId: 'node-2',
      stepTitle: 'Step 2',
      stepNarrative: 'This is step 2',
      learningObjectives: [],
      optional: false,
      completionCriteria: []
    },
    content: {
      id: 'node-2',
      title: 'Content 2',
      description: 'Content for step 2',
      contentType: 'concept',
      contentFormat: 'markdown',
      content: '# Test Content\n\nThis is **markdown** content.',
      tags: [],
      relatedNodeIds: [],
      metadata: {}
    },
    hasNext: true,
    hasPrevious: true,
    nextStepIndex: 2,
    previousStepIndex: 0
  };

  const mockStepsWithStatus = [
    { isCompleted: true, completedInOtherPath: false, content: { contentType: 'concept' } },
    { isCompleted: false, completedInOtherPath: false, content: { contentType: 'concept' } },
    { isCompleted: false, completedInOtherPath: false, content: { contentType: 'concept' } }
  ];

  beforeEach(async () => {
    const pathServiceSpy = jasmine.createSpyObj('PathService', ['getPath', 'getPathStep', 'getAllStepsWithCompletionStatus']);
    const agentServiceSpy = jasmine.createSpyObj('AgentService', ['completeStep']);

    paramsSubject = new BehaviorSubject({ pathId: 'test-path', stepIndex: '1' });

    await TestBed.configureTestingModule({
      imports: [PathNavigatorComponent],
      providers: [
        provideRouter([]),
        { provide: PathService, useValue: pathServiceSpy },
        { provide: AgentService, useValue: agentServiceSpy },
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

    pathService.getAllStepsWithCompletionStatus.and.returnValue(of(mockStepsWithStatus as any));
    pathService.getPath.and.returnValue(of(mockPath));
    pathService.getPathStep.and.returnValue(of(mockStepView));
    agentService.completeStep.and.returnValue(of(undefined));

    fixture = TestBed.createComponent(PathNavigatorComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should load path and step on init', () => {
    fixture.detectChanges();

    expect(pathService.getAllStepsWithCompletionStatus).toHaveBeenCalledWith('test-path');
    expect(pathService.getPath).toHaveBeenCalledWith('test-path');
    expect(pathService.getPathStep).toHaveBeenCalledWith('test-path', 1);
    expect(component.path).toEqual(mockPath);
    expect(component.stepView).toEqual(mockStepView);
    expect(component.isLoading).toBe(false);
  });

  it('should parse stepIndex from route params', () => {
    fixture.detectChanges();

    expect(component.pathId).toBe('test-path');
    expect(component.stepIndex).toBe(1);
  });

  it('should default stepIndex to 0 if invalid', () => {
    paramsSubject.next({ pathId: 'test-path', stepIndex: 'invalid' });
    fixture.detectChanges();

    expect(component.stepIndex).toBe(0);
  });

  it('should handle step load error', () => {
    pathService.getPathStep.and.returnValue(throwError(() => new Error('Network error')));

    fixture.detectChanges();

    expect(component.isLoading).toBe(false);
    expect(component.error).toBe('Network error');
  });

  it('should handle getAllStepsWithCompletionStatus error', () => {
    pathService.getAllStepsWithCompletionStatus.and.returnValue(throwError(() => new Error('Steps error')));

    fixture.detectChanges();

    expect(component.isLoading).toBe(false);
    expect(component.error).toBe('Steps error');
  });

  it('should navigate to previous step', () => {
    fixture.detectChanges();
    component.goToPrevious();

    expect(router.navigate).toHaveBeenCalledWith(['/lamad/path', 'test-path', 'step', 0]);
  });

  it('should not navigate to previous if hasPrevious is false', () => {
    component.stepView = { ...mockStepView, hasPrevious: false };
    component.goToPrevious();

    expect(router.navigate).not.toHaveBeenCalled();
  });

  it('should navigate to next step', () => {
    fixture.detectChanges();
    component.goToNext();

    expect(router.navigate).toHaveBeenCalledWith(['/lamad/path', 'test-path', 'step', 2]);
  });

  it('should not navigate to next if hasNext is false', () => {
    component.stepView = { ...mockStepView, hasNext: false };
    component.goToNext();

    expect(router.navigate).not.toHaveBeenCalled();
  });

  it('should navigate to path overview', () => {
    fixture.detectChanges();
    component.goToPathOverview();

    expect(router.navigate).toHaveBeenCalledWith(['/lamad/path', 'test-path']);
  });

  it('should cycle through Bloom levels on markComplete', () => {
    // Use NEVER to prevent completeStep from triggering loadContext reload
    agentService.completeStep.and.returnValue(NEVER);
    fixture.detectChanges();
    expect(component.currentBloomLevel).toBe('not_started');

    component.markComplete();
    expect(component.currentBloomLevel).toBe('seen');

    component.markComplete();
    expect(component.currentBloomLevel).toBe('remember');
  });

  it('should mark step as complete when reaching remember level', () => {
    fixture.detectChanges();
    // Cycle from not_started -> seen -> remember
    component.markComplete(); // seen
    component.markComplete(); // remember - triggers completeStep

    expect(agentService.completeStep).toHaveBeenCalledWith('test-path', 1);
  });

  it('should reload context after marking complete at remember level', () => {
    fixture.detectChanges();
    pathService.getAllStepsWithCompletionStatus.calls.reset();

    component.markComplete(); // seen
    component.markComplete(); // remember - triggers reload

    expect(pathService.getAllStepsWithCompletionStatus).toHaveBeenCalled();
  });

  it('should calculate progress percentage', () => {
    fixture.detectChanges();

    expect(component.getProgressPercentage()).toBe(67); // (1+1)/3 * 100 = 66.67 rounded to 67
  });

  it('should return 0 progress if no path', () => {
    component.path = null;
    expect(component.getProgressPercentage()).toBe(0);
  });

  it('should get content as string', () => {
    fixture.detectChanges();

    const content = component.getContentString();
    expect(content).toContain('# Test Content');
  });

  it('should get empty string if no content', () => {
    component.stepView = null;
    expect(component.getContentString()).toBe('');
  });

  it('should stringify object content', () => {
    const objectContent = { test: 'data' };
    component.stepView = {
      ...mockStepView,
      content: {
        ...mockStepView.content!,
        content: objectContent
      }
    };

    const content = component.getContentString();
    expect(content).toContain('"test"');
    expect(content).toContain('"data"');
  });

  it('should detect markdown content', () => {
    fixture.detectChanges();
    expect(component.isMarkdown()).toBe(true);
  });

  it('should detect quiz content by format', () => {
    component.stepView = {
      ...mockStepView,
      content: {
        ...mockStepView.content!,
        contentFormat: 'quiz-json'
      }
    };
    expect(component.isQuiz()).toBe(true);
  });

  it('should detect quiz content by type', () => {
    component.stepView = {
      ...mockStepView,
      content: {
        ...mockStepView.content!,
        contentType: 'assessment'
      }
    };
    expect(component.isQuiz()).toBe(true);
  });

  it('should detect gherkin content', () => {
    component.stepView = {
      ...mockStepView,
      content: {
        ...mockStepView.content!,
        contentFormat: 'gherkin'
      }
    };
    expect(component.isGherkin()).toBe(true);
  });

  it('should get Bloom display formatted', () => {
    component.currentBloomLevel = 'not_started';
    expect(component.getBloomDisplay()).toBe('NOT STARTED');

    component.currentBloomLevel = 'remember';
    expect(component.getBloomDisplay()).toBe('REMEMBER');
  });

  it('should cleanup on destroy', () => {
    fixture.detectChanges();

    spyOn(component['destroy$'], 'next');
    spyOn(component['destroy$'], 'complete');

    component.ngOnDestroy();

    expect(component['destroy$'].next).toHaveBeenCalled();
    expect(component['destroy$'].complete).toHaveBeenCalled();
  });

  it('should reload step when route params change', () => {
    fixture.detectChanges();
    pathService.getAllStepsWithCompletionStatus.calls.reset();
    pathService.getPathStep.calls.reset();

    paramsSubject.next({ pathId: 'test-path', stepIndex: '2' });

    expect(component.stepIndex).toBe(2);
    expect(pathService.getAllStepsWithCompletionStatus).toHaveBeenCalledWith('test-path');
    expect(pathService.getPathStep).toHaveBeenCalledWith('test-path', 2);
  });

  it('should toggle sidebar', () => {
    expect(component.sidebarOpen).toBe(true);
    component.toggleSidebar();
    expect(component.sidebarOpen).toBe(false);
    component.toggleSidebar();
    expect(component.sidebarOpen).toBe(true);
  });

  it('should toggle chapter expansion', () => {
    fixture.detectChanges();
    // Add a chapter manually since mockPath doesn't have chapters
    component.sidebarChapters = [
      { id: 'ch1', title: 'Chapter 1', steps: [], isExpanded: true }
    ];

    component.toggleChapter('ch1');
    expect(component.sidebarChapters[0].isExpanded).toBe(false);

    component.toggleChapter('ch1');
    expect(component.sidebarChapters[0].isExpanded).toBe(true);
  });

  it('should get current chapter title', () => {
    fixture.detectChanges();
    component.sidebarChapters = [
      { id: 'ch1', title: 'Chapter 1', steps: [], isExpanded: true }
    ];
    component.currentChapterId = 'ch1';

    expect(component.getCurrentChapterTitle()).toBe('Chapter 1');
  });

  it('should return undefined for current chapter title if not found', () => {
    fixture.detectChanges();
    component.currentChapterId = 'nonexistent';

    expect(component.getCurrentChapterTitle()).toBeUndefined();
  });
});
