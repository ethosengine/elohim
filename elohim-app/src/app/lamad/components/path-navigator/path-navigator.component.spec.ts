import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute, Router } from '@angular/router';
import { of, throwError, BehaviorSubject } from 'rxjs';
import { PathNavigatorComponent } from './path-navigator.component';
import { PathService } from '../../services/path.service';
import { AgentService } from '../../services/agent.service';
import { PathStepView, LearningPath } from '../../models/learning-path.model';

describe('PathNavigatorComponent', () => {
  let component: PathNavigatorComponent;
  let fixture: ComponentFixture<PathNavigatorComponent>;
  let pathService: jasmine.SpyObj<PathService>;
  let agentService: jasmine.SpyObj<AgentService>;
  let router: jasmine.SpyObj<Router>;
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

  beforeEach(async () => {
    const pathServiceSpy = jasmine.createSpyObj('PathService', ['getPath', 'getPathStep']);
    const agentServiceSpy = jasmine.createSpyObj('AgentService', ['completeStep']);
    const routerSpy = jasmine.createSpyObj('Router', ['navigate']);

    paramsSubject = new BehaviorSubject({ pathId: 'test-path', stepIndex: '1' });

    await TestBed.configureTestingModule({
      imports: [PathNavigatorComponent],
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

  it('should handle path load error gracefully', () => {
    pathService.getPath.and.returnValue(throwError(() => new Error('Path error')));

    fixture.detectChanges();

    // Path error is logged but doesn't stop step loading
    expect(pathService.getPathStep).toHaveBeenCalled();
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

  it('should mark step as complete', () => {
    fixture.detectChanges();
    component.markComplete();

    expect(agentService.completeStep).toHaveBeenCalledWith('test-path', 1);
  });

  it('should reload step after marking complete', () => {
    fixture.detectChanges();
    pathService.getPathStep.calls.reset();

    component.markComplete();

    expect(pathService.getPathStep).toHaveBeenCalled();
  });

  it('should handle mark complete error', () => {
    agentService.completeStep.and.returnValue(throwError(() => new Error('Complete error')));
    fixture.detectChanges();

    expect(() => component.markComplete()).not.toThrow();
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

  it('should render markdown headers', () => {
    const markdown = '# Header 1\n## Header 2\n### Header 3';
    const html = component.renderMarkdown(markdown);

    expect(html).toContain('<h1>Header 1</h1>');
    expect(html).toContain('<h2>Header 2</h2>');
    expect(html).toContain('<h3>Header 3</h3>');
  });

  it('should render markdown bold and italic', () => {
    const markdown = '**bold** and *italic*';
    const html = component.renderMarkdown(markdown);

    expect(html).toContain('<strong>bold</strong>');
    expect(html).toContain('<em>italic</em>');
  });

  it('should render markdown links', () => {
    const markdown = '[Link Text](https://example.com)';
    const html = component.renderMarkdown(markdown);

    expect(html).toContain('<a href="https://example.com" target="_blank">Link Text</a>');
  });

  it('should render markdown code blocks', () => {
    const markdown = '```code block```';
    const html = component.renderMarkdown(markdown);

    expect(html).toContain('<pre><code>code block</code></pre>');
  });

  it('should render markdown inline code', () => {
    const markdown = 'inline `code` here';
    const html = component.renderMarkdown(markdown);

    expect(html).toContain('<code>code</code>');
  });

  it('should render markdown blockquotes', () => {
    const markdown = '> Quote';
    const html = component.renderMarkdown(markdown);

    expect(html).toContain('<blockquote>Quote</blockquote>');
  });

  it('should render markdown lists', () => {
    const markdown = '- Item 1\n- Item 2';
    const html = component.renderMarkdown(markdown);

    expect(html).toContain('<li>Item 1</li>');
    expect(html).toContain('<li>Item 2</li>');
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
    pathService.getPathStep.calls.reset();

    paramsSubject.next({ pathId: 'test-path', stepIndex: '2' });

    expect(component.stepIndex).toBe(2);
    expect(pathService.getPathStep).toHaveBeenCalledWith('test-path', 2);
  });
});
