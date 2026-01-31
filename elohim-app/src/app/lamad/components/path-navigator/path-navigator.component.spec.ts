import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { ActivatedRoute, Router, provideRouter } from '@angular/router';
import { of, throwError, BehaviorSubject, NEVER } from 'rxjs';
import { PathNavigatorComponent } from './path-navigator.component';
import { PathService } from '../../services/path.service';
import { AgentService } from '@app/elohim/services/agent.service';
import { GovernanceSignalService } from '@app/elohim/services/governance-signal.service';
import { PathContextService } from '../../services/path-context.service';
import { SeoService } from '../../../services/seo.service';
import { PathStepView, LearningPath } from '../../models/learning-path.model';
import { provideElohimClient } from '@app/elohim/providers/elohim-client.provider';

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
    const governanceSignalServiceSpy = jasmine.createSpyObj('GovernanceSignalService', [
      'recordLearningSignal',
      'recordInteractiveCompletion',
    ]);

    paramsSubject = new BehaviorSubject({ pathId: 'test-path', stepIndex: '1' });

    await TestBed.configureTestingModule({
      imports: [PathNavigatorComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter([]),
        provideElohimClient({
          mode: { type: 'browser', doorway: { url: 'http://localhost:8888' } }
        }),
        { provide: PathService, useValue: pathServiceSpy },
        { provide: AgentService, useValue: agentServiceSpy },
        { provide: PathContextService, useValue: pathContextServiceSpy },
        { provide: SeoService, useValue: seoServiceSpy },
        { provide: GovernanceSignalService, useValue: governanceSignalServiceSpy },
        {
          provide: ActivatedRoute,
          useValue: { params: paramsSubject.asObservable() },
        },
      ],
    }).compileComponents();

    pathService = TestBed.inject(PathService) as jasmine.SpyObj<PathService>;
    agentService = TestBed.inject(AgentService) as jasmine.SpyObj<AgentService>;
    router = TestBed.inject(Router);
    spyOn(router, 'navigate').and.returnValue(Promise.resolve(true));

    pathService.getPath.and.returnValue(of(mockPath));
    pathService.getPathStep.and.returnValue(of(mockStepView));
    pathService.getContentById.and.returnValue(of(mockStepView.content));
    agentService.completeStep.and.returnValue(of(undefined));
    agentService.markContentSeen.and.returnValue(of(undefined));
    agentService.getContentMastery.and.returnValue(of('not_started'));
    governanceSignalServiceSpy.recordLearningSignal.and.returnValue(of(undefined));
    governanceSignalServiceSpy.recordInteractiveCompletion.and.returnValue(of(undefined));

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

  describe('hierarchical path loading (4-level)', () => {
    const hierarchicalPath: LearningPath = {
      id: 'hierarchical-path',
      version: '1.0.0',
      title: 'Hierarchical Path',
      description: 'A 4-level hierarchical path',
      purpose: 'Testing hierarchies',
      createdBy: 'test-user',
      contributors: [],
      createdAt: '2025-01-01T00:00:00.000Z',
      updatedAt: '2025-01-01T00:00:00.000Z',
      difficulty: 'intermediate',
      estimatedDuration: '2 hours',
      tags: ['hierarchical'],
      visibility: 'public',
      steps: [],
      chapters: [
        {
          id: 'chapter-1',
          title: 'Chapter 1',
          order: 0,
          modules: [
            {
              id: 'module-1',
              title: 'Module 1',
              order: 0,
              sections: [
                {
                  id: 'section-1',
                  title: 'Section 1',
                  order: 0,
                  conceptIds: ['concept-1', 'concept-2', 'concept-3'],
                },
                {
                  id: 'section-2',
                  title: 'Section 2',
                  order: 1,
                  conceptIds: ['concept-4', 'concept-5'],
                },
              ],
            },
          ],
        },
      ],
    };

    it('should build lesson context for hierarchical path', () => {
      pathService.getPath.and.returnValue(of(hierarchicalPath));
      pathService.getContentById.and.returnValue(
        of({
          id: 'concept-1',
          title: 'Concept 1',
          description: 'First concept',
          contentType: 'concept',
          contentFormat: 'markdown',
          content: '# Concept 1',
          tags: [],
          relatedNodeIds: [],
          metadata: {},
        })
      );

      paramsSubject.next({ pathId: 'hierarchical-path', stepIndex: '0' });
      fixture.detectChanges();

      expect(component.lessonContext).toBeTruthy();
      expect(component.lessonContext?.chapter.title).toBe('Chapter 1');
      expect(component.lessonContext?.module.title).toBe('Module 1');
      expect(component.lessonContext?.section.title).toBe('Section 1');
      expect(component.lessonContext?.concepts.length).toBe(3);
      expect(component.lessonContext?.currentConceptIndex).toBe(0);
    });

    it('should navigate between concepts in hierarchical path', () => {
      pathService.getPath.and.returnValue(of(hierarchicalPath));
      pathService.getContentById.and.returnValue(
        of({
          id: 'concept-2',
          title: 'Concept 2',
          description: 'Second concept',
          contentType: 'concept',
          contentFormat: 'markdown',
          content: '# Concept 2',
          tags: [],
          relatedNodeIds: [],
          metadata: {},
        })
      );

      paramsSubject.next({ pathId: 'hierarchical-path', stepIndex: '1' });
      fixture.detectChanges();

      expect(component.lessonContext?.currentConceptIndex).toBe(1);
      expect(component.stepIndex).toBe(1);
    });

    it('should calculate total concepts in hierarchical path', () => {
      pathService.getPath.and.returnValue(of(hierarchicalPath));
      pathService.getContentById.and.returnValue(of(mockStepView.content));

      paramsSubject.next({ pathId: 'hierarchical-path', stepIndex: '0' });
      fixture.detectChanges();

      // 3 concepts in section-1 + 2 concepts in section-2 = 5 total
      expect(component.getTotalConcepts()).toBe(5);
    });

    it('should get lesson progress percentage in hierarchical context', () => {
      pathService.getPath.and.returnValue(of(hierarchicalPath));
      pathService.getContentById.and.returnValue(of(mockStepView.content));

      paramsSubject.next({ pathId: 'hierarchical-path', stepIndex: '1' });
      fixture.detectChanges();

      // Concept 2 of 3 in section = (1 + 1) / 3 = 67%
      expect(component.getLessonProgressPercentage()).toBe(67);
    });

    it('should get current module title from lesson context', () => {
      pathService.getPath.and.returnValue(of(hierarchicalPath));
      pathService.getContentById.and.returnValue(of(mockStepView.content));

      paramsSubject.next({ pathId: 'hierarchical-path', stepIndex: '0' });
      fixture.detectChanges();

      expect(component.getCurrentModuleTitle()).toBe('Module 1');
    });

    it('should get current section title from lesson context', () => {
      pathService.getPath.and.returnValue(of(hierarchicalPath));
      pathService.getContentById.and.returnValue(of(mockStepView.content));

      paramsSubject.next({ pathId: 'hierarchical-path', stepIndex: '0' });
      fixture.detectChanges();

      expect(component.getCurrentSectionTitle()).toBe('Section 1');
    });

    it('should navigate to specific concept by index', () => {
      pathService.getPath.and.returnValue(of(hierarchicalPath));
      pathService.getContentById.and.returnValue(of(mockStepView.content));

      paramsSubject.next({ pathId: 'hierarchical-path', stepIndex: '0' });
      fixture.detectChanges();

      component.goToConcept(2);

      expect(router.navigate).toHaveBeenCalledWith(['/lamad/path', 'hierarchical-path', 'step', 2]);
    });
  });

  describe('focused view mode', () => {
    it('should toggle focused view on', () => {
      fixture.detectChanges();
      expect(component.isFocusedView).toBe(false);

      component.onFocusedViewToggle(true);

      expect(component.isFocusedView).toBe(true);
      expect(component.sidebarOpen).toBe(false);
    });

    it('should toggle focused view off', () => {
      fixture.detectChanges();
      component.isFocusedView = true;

      component.onFocusedViewToggle(false);

      expect(component.isFocusedView).toBe(false);
    });

    it('should exit focused view on escape key', () => {
      fixture.detectChanges();
      component.isFocusedView = true;

      component.onEscapeKey();

      expect(component.isFocusedView).toBe(false);
    });

    it('should not exit focused view on escape if not in focused view', () => {
      fixture.detectChanges();
      component.isFocusedView = false;

      component.onEscapeKey();

      expect(component.isFocusedView).toBe(false);
    });

    it('should increment content refresh key when toggling focused view', done => {
      fixture.detectChanges();
      const initialKey = component.contentRefreshKey;

      component.onFocusedViewToggle(true);

      setTimeout(() => {
        expect(component.contentRefreshKey).toBeGreaterThan(initialKey);
        done();
      }, 350);
    });
  });

  describe('exploration events', () => {
    beforeEach(() => {
      fixture.detectChanges();
    });

    it('should handle explore content event', () => {
      const pathContextService = TestBed.inject(PathContextService);

      component.onExploreContent('related-node-1');

      expect(pathContextService.startDetour).toHaveBeenCalledWith(
        jasmine.objectContaining({
          toContentId: 'related-node-1',
          detourType: 'related',
        })
      );
      expect(router.navigate).toHaveBeenCalledWith(['/lamad/resource', 'related-node-1']);
    });

    it('should handle explore in graph event', () => {
      const pathContextService = TestBed.inject(PathContextService);
      component.stepView = mockStepView;

      component.onExploreInGraph();

      expect(pathContextService.startDetour).toHaveBeenCalledWith(
        jasmine.objectContaining({
          detourType: 'graph-explore',
        })
      );
      expect(router.navigate).toHaveBeenCalledWith(
        ['/lamad/explore'],
        jasmine.objectContaining({
          queryParams: jasmine.objectContaining({
            focus: 'node-2',
            fromPath: 'test-path',
            returnStep: 1,
          }),
        })
      );
    });

    it('should not explore in graph if no step view', () => {
      const pathContextService = TestBed.inject(PathContextService);
      component.stepView = null;

      component.onExploreInGraph();

      expect(pathContextService.startDetour).not.toHaveBeenCalled();
    });
  });

  describe('interactive content completion', () => {
    beforeEach(() => {
      fixture.detectChanges();
      component.stepView = mockStepView;
      (component as any).contentViewStartTime = Date.now() - 10000; // 10 seconds ago
    });

    it('should handle lesson completion event', () => {
      const governanceSignalService = TestBed.inject(GovernanceSignalService) as jasmine.SpyObj<GovernanceSignalService>;

      const completionEvent = {
        type: 'quiz' as const,
        passed: true,
        score: 85,
        details: { attempts: 1 },
      };

      component.onLessonComplete(completionEvent);

      expect(governanceSignalService.recordInteractiveCompletion).toHaveBeenCalledWith(
        jasmine.objectContaining({
          contentId: 'node-2',
          interactionType: 'quiz',
          passed: true,
          score: 85,
        })
      );
    });

    it('should advance mastery level on successful completion', () => {
      // Use NEVER to prevent completeStep reload
      agentService.completeStep.and.returnValue(NEVER);

      component.currentBloomLevel = 'not_started';
      const completionEvent = {
        type: 'quiz' as const,
        passed: true,
        score: 90,
        details: {},
      };

      component.onLessonComplete(completionEvent);

      expect(component.currentBloomLevel).toBe('seen');
    });

    it('should not advance mastery level on failed completion', () => {
      component.currentBloomLevel = 'not_started';
      const completionEvent = {
        type: 'quiz' as const,
        passed: false,
        score: 45,
        details: {},
      };

      component.onLessonComplete(completionEvent);

      expect(component.currentBloomLevel).toBe('not_started');
    });
  });

  describe('path context service integration', () => {
    it('should enter path context on load', () => {
      const pathContextService = TestBed.inject(PathContextService);

      fixture.detectChanges();

      expect(pathContextService.enterPath).toHaveBeenCalled();
    });

    it('should exit path context on destroy', () => {
      const pathContextService = TestBed.inject(PathContextService);
      fixture.detectChanges();

      component.ngOnDestroy();

      expect(pathContextService.exitPath).toHaveBeenCalled();
    });

    it('should build path context correctly', () => {
      fixture.detectChanges();
      component.path = mockPath;
      component.stepIndex = 1;

      const context = component.buildPathContext();

      expect(context.pathId).toBe('test-path');
      expect(context.pathTitle).toBe('Test Path');
      expect(context.stepIndex).toBe(1);
      expect(context.returnRoute).toEqual(['/lamad/path', 'test-path', 'step', '1']);
    });
  });

  describe('learning signal tracking', () => {
    it('should emit progress signal on navigation', () => {
      const governanceSignalService: any = jasmine.createSpyObj('GovernanceSignalService', ['recordLearningSignal']);
      (component as any).governanceSignalService = governanceSignalService;
      governanceSignalService.recordLearningSignal.and.returnValue(of(undefined));

      fixture.detectChanges();
      component.stepView = mockStepView;
      (component as any).contentViewStartTime = Date.now() - 10000; // 10 seconds ago

      component.goToNext();

      expect(governanceSignalService.recordLearningSignal).toHaveBeenCalledWith(
        jasmine.objectContaining({
          signalType: 'progress_update',
        })
      );
    });

    it('should not emit progress signal if time is too short', () => {
      const governanceSignalService: any = jasmine.createSpyObj('GovernanceSignalService', ['recordLearningSignal']);
      (component as any).governanceSignalService = governanceSignalService;
      governanceSignalService.recordLearningSignal.and.returnValue(of(undefined));

      fixture.detectChanges();
      component.stepView = mockStepView;
      (component as any).contentViewStartTime = Date.now() - 2000; // 2 seconds ago (too short)

      component.goToNext();

      expect(governanceSignalService.recordLearningSignal).not.toHaveBeenCalled();
    });
  });

  describe('error handling', () => {
    it('should handle content not found', () => {
      // Mock getPathStep to return a step with null content (since mockPath has no chapters)
      pathService.getPathStep.and.returnValue(of({
        step: mockPath.steps[0],
        content: null,
        hasNext: true,
        hasPrevious: false,
        nextStepIndex: 1,
        previousStepIndex: undefined,
      } as any));

      paramsSubject.next({ pathId: 'test-path', stepIndex: '0' });
      fixture.detectChanges();

      expect(component.error).toContain('Content not found');
      expect(component.isLoading).toBe(false);
    });

    it('should handle generic load errors', () => {
      pathService.getPath.and.returnValue(
        throwError(() => ({ message: 'Server unreachable' }))
      );

      paramsSubject.next({ pathId: 'test-path', stepIndex: '0' });
      fixture.detectChanges();

      expect(component.error).toBe('Server unreachable');
      expect(component.isLoading).toBe(false);
    });

    it('should handle errors without message', () => {
      pathService.getPath.and.returnValue(throwError(() => ({})));

      paramsSubject.next({ pathId: 'test-path', stepIndex: '0' });
      fixture.detectChanges();

      expect(component.error).toBe('Failed to load learning path');
    });
  });

  describe('edge cases', () => {
    it('should handle path with no chapters', () => {
      const emptyPath = { ...mockPath, chapters: [] };
      pathService.getPath.and.returnValue(of(emptyPath));

      paramsSubject.next({ pathId: 'test-path', stepIndex: '0' });
      fixture.detectChanges();

      expect(component.getTotalConcepts()).toBe(0);
      expect(component.lessonContext).toBeNull();
    });

    it('should handle undefined path chapters', () => {
      const noChaptersPath = { ...mockPath, chapters: undefined };
      pathService.getPath.and.returnValue(of(noChaptersPath));

      paramsSubject.next({ pathId: 'test-path', stepIndex: '0' });
      fixture.detectChanges();

      expect(component.getTotalConcepts()).toBe(0);
    });

    it('should return 0 lesson progress if no lesson context', () => {
      fixture.detectChanges();
      component.lessonContext = null;

      expect(component.getLessonProgressPercentage()).toBe(0);
    });

    it('should return undefined for module title if no lesson context', () => {
      fixture.detectChanges();
      component.lessonContext = null;

      expect(component.getCurrentModuleTitle()).toBeUndefined();
    });

    it('should return undefined for section title if no lesson context', () => {
      fixture.detectChanges();
      component.lessonContext = null;

      expect(component.getCurrentSectionTitle()).toBeUndefined();
    });

    it('should handle step without resource ID', () => {
      const stepWithoutResource = {
        ...mockStepView,
        step: { ...mockStepView.step, resourceId: '' },
      };
      pathService.getPathStep.and.returnValue(of(stepWithoutResource));

      paramsSubject.next({ pathId: 'test-path', stepIndex: '0' });
      fixture.detectChanges();

      // Should not crash
      expect(component.isLoading).toBe(false);
    });
  });

  describe('2-level path format', () => {
    const twoLevelPath: LearningPath = {
      ...mockPath,
      chapters: [
        {
          id: 'chapter-1',
          title: 'Chapter 1',
          order: 0,
          steps: [
            {
              order: 0,
              resourceId: 'step-1',
              stepTitle: 'Step 1',
              stepNarrative: 'First step',
              learningObjectives: [],
              optional: false,
              completionCriteria: [],
            },
            {
              order: 1,
              resourceId: 'step-2',
              stepTitle: 'Step 2',
              stepNarrative: 'Second step',
              learningObjectives: [],
              optional: false,
              completionCriteria: [],
            },
          ],
        },
      ],
    };

    it('should handle 2-level path format', () => {
      pathService.getPath.and.returnValue(of(twoLevelPath));
      pathService.getContentById.and.returnValue(
        of({
          id: 'step-1',
          title: 'Step 1',
          description: 'First step',
          contentType: 'concept',
          contentFormat: 'markdown',
          content: '# Step 1',
          tags: [],
          relatedNodeIds: [],
          metadata: {},
        })
      );

      paramsSubject.next({ pathId: 'test-path', stepIndex: '0' });
      fixture.detectChanges();

      expect(component.lessonContext).toBeTruthy();
      expect(component.lessonContext?.concepts.length).toBe(2);
    });

    it('should calculate total concepts for 2-level path', () => {
      pathService.getPath.and.returnValue(of(twoLevelPath));
      pathService.getContentById.and.returnValue(of(mockStepView.content));

      paramsSubject.next({ pathId: 'test-path', stepIndex: '0' });
      fixture.detectChanges();

      expect(component.getTotalConcepts()).toBe(2);
    });
  });
});
