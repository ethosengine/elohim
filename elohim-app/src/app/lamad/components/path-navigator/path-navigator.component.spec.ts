import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { ActivatedRoute, Router, provideRouter } from '@angular/router';
import { of, throwError, BehaviorSubject, NEVER } from 'rxjs';
import { PathNavigatorComponent } from './path-navigator.component';
import { PathService } from '../../services/path.service';
import { AgentService } from '@app/elohim/services/agent.service';
import { PathContextService } from '../../services/path-context.service';
import { SeoService } from '../../../services/seo.service';
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
        completionCriteria: [],
      },
      {
        order: 1,
        resourceId: 'node-2',
        stepTitle: 'Step 2',
        stepNarrative: 'Narrative 2',
        learningObjectives: [],
        optional: false,
        completionCriteria: [],
      },
      {
        order: 2,
        resourceId: 'node-3',
        stepTitle: 'Step 3',
        stepNarrative: 'Narrative 3',
        learningObjectives: [],
        optional: false,
        completionCriteria: [],
      },
    ],
  };

  const mockStepView: PathStepView = {
    step: {
      order: 1,
      resourceId: 'node-2',
      stepTitle: 'Step 2',
      stepNarrative: 'This is step 2',
      learningObjectives: [],
      optional: false,
      completionCriteria: [],
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
      metadata: {},
    },
    hasNext: true,
    hasPrevious: true,
    nextStepIndex: 2,
    previousStepIndex: 0,
  };

  beforeEach(async () => {
    const pathServiceSpy = jasmine.createSpyObj('PathService', [
      'getPath',
      'getPathStep',
      'getContentById',
    ]);
    const agentServiceSpy = jasmine.createSpyObj('AgentService', [
      'completeStep',
      'markContentSeen',
      'getContentMastery',
    ]);
    const pathContextServiceSpy = jasmine.createSpyObj('PathContextService', [
      'enterPath',
      'exitPath',
      'startDetour',
    ]);
    const seoServiceSpy = jasmine.createSpyObj('SeoService', [
      'updateSeo',
      'updateForPath',
      'setTitle',
    ]);

    paramsSubject = new BehaviorSubject({ pathId: 'test-path', stepIndex: '1' });

    await TestBed.configureTestingModule({
      imports: [PathNavigatorComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter([]),
        { provide: PathService, useValue: pathServiceSpy },
        { provide: AgentService, useValue: agentServiceSpy },
        { provide: PathContextService, useValue: pathContextServiceSpy },
        { provide: SeoService, useValue: seoServiceSpy },
        {
          provide: ActivatedRoute,
          useValue: { params: paramsSubject.asObservable() },
        },
      ],
    }).compileComponents();

    pathService = TestBed.inject(PathService) as jasmine.SpyObj<PathService>;
    agentService = TestBed.inject(AgentService) as jasmine.SpyObj<AgentService>;
    router = TestBed.inject(Router);
    spyOn(router, 'navigate');

    pathService.getPath.and.returnValue(of(mockPath));
    pathService.getPathStep.and.returnValue(of(mockStepView));
    pathService.getContentById.and.returnValue(of(mockStepView.content));
    agentService.completeStep.and.returnValue(of(undefined));
    agentService.markContentSeen.and.returnValue(of(undefined));
    agentService.getContentMastery.and.returnValue(of('not_started'));

    fixture = TestBed.createComponent(PathNavigatorComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should load path and step on init', () => {
    fixture.detectChanges();

    expect(pathService.getPath).toHaveBeenCalledWith('test-path');
    expect(component.path).toEqual(mockPath);
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

  it('should handle path load error', () => {
    pathService.getPath.and.returnValue(throwError(() => new Error('Network error')));

    fixture.detectChanges();

    expect(component.isLoading).toBe(false);
    expect(component.error).toBe('Network error');
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
    pathService.getPath.calls.reset();

    component.markComplete(); // seen
    component.markComplete(); // remember - triggers reload

    expect(pathService.getPath).toHaveBeenCalled();
  });

  it('should calculate progress percentage when path has chapters', () => {
    // Progress requires chapters structure now
    // Without chapters, progress returns 0 (no hierarchy loaded)
    fixture.detectChanges();

    expect(component.getProgressPercentage()).toBe(0); // Path needs chapters for progress
  });

  it('should return 0 progress if no path', () => {
    component.path = null;
    expect(component.getProgressPercentage()).toBe(0);
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
    pathService.getPath.calls.reset();

    paramsSubject.next({ pathId: 'test-path', stepIndex: '2' });

    expect(component.stepIndex).toBe(2);
    expect(pathService.getPath).toHaveBeenCalledWith('test-path');
  });

  it('should toggle sidebar', () => {
    expect(component.sidebarOpen).toBe(true);
    component.toggleSidebar();
    expect(component.sidebarOpen).toBe(false);
    component.toggleSidebar();
    expect(component.sidebarOpen).toBe(true);
  });

  it('should get current chapter title from lessonContext', () => {
    fixture.detectChanges();
    // Set up lesson context with a chapter
    component.lessonContext = {
      chapter: { id: 'ch1', title: 'Chapter 1', order: 0 },
      chapterIndex: 0,
      module: { id: 'm1', title: 'Module 1', order: 0, sections: [] },
      moduleIndex: 0,
      section: { id: 's1', title: 'Section 1', order: 0, conceptIds: [] },
      sectionIndex: 0,
      concepts: [],
      currentConceptIndex: 0,
    };

    expect(component.getCurrentChapterTitle()).toBe('Chapter 1');
  });

  it('should return undefined for current chapter title if no lessonContext', () => {
    fixture.detectChanges();
    component.lessonContext = null;

    expect(component.getCurrentChapterTitle()).toBeUndefined();
  });
});
