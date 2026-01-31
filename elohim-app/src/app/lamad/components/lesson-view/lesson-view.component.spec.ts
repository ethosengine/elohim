import { ComponentFixture, TestBed, fakeAsync, tick } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { LessonViewComponent } from './lesson-view.component';
import { RendererRegistryService } from '../../renderers/renderer-registry.service';
import { ContentNode } from '../../models/content-node.model';
import { PathContext } from '../../models/exploration-context.model';
import { Component, EventEmitter, Input, Output } from '@angular/core';
import { RelatedConceptsPanelComponent } from '../related-concepts-panel/related-concepts-panel.component';
import { MiniGraphComponent } from '../mini-graph/mini-graph.component';

// Mock components
@Component({ selector: 'app-mini-graph', standalone: true, template: '' })
class MockMiniGraphComponent {
  @Input() focusNodeId!: string;
  @Input() depth = 1;
  @Input() height = 180;
  @Output() nodeSelected = new EventEmitter<string>();
  @Output() exploreRequested = new EventEmitter<void>();
}

@Component({ selector: 'app-related-concepts-panel', standalone: true, template: '' })
class MockRelatedConceptsPanelComponent {
  @Input() contentId!: string;
  @Input() showHierarchy = true;
  @Input() compact = true;
  @Input() limit = 4;
  @Output() navigate = new EventEmitter<string>();
}

// Mock renderer component
@Component({ selector: 'app-mock-renderer', standalone: true, template: '<div>Mock Renderer</div>' })
class MockRendererComponent {
  @Input() node!: ContentNode;
  @Input() embedded = false;
  @Output() complete = new EventEmitter<any>();
}

describe('LessonViewComponent', () => {
  let component: LessonViewComponent;
  let fixture: ComponentFixture<LessonViewComponent>;
  let rendererRegistrySpy: jasmine.SpyObj<RendererRegistryService>;

  const mockContent: ContentNode = {
    id: 'test-concept',
    title: 'Test Concept',
    description: 'A test concept for learning',
    contentType: 'concept',
    contentFormat: 'markdown',
    content: '# Test Concept\n\nThis is test content.',
    tags: ['test', 'learning', 'sample'],
    relatedNodeIds: [],
    metadata: {},
  } as ContentNode;

  const mockPathContext: PathContext = {
    pathId: 'test-path',
    pathTitle: 'Test Path',
    stepIndex: 2,
    totalSteps: 10,
    chapterTitle: 'Introduction',
    returnRoute: ['/lamad/path', 'test-path', 'step', '2'],
    detourStack: [],
  };

  beforeEach(async () => {
    rendererRegistrySpy = jasmine.createSpyObj('RendererRegistryService', ['getRenderer']);
    rendererRegistrySpy.getRenderer.and.returnValue(null);

    await TestBed.configureTestingModule({
      imports: [LessonViewComponent],
      providers: [
        provideHttpClient(),
        { provide: RendererRegistryService, useValue: rendererRegistrySpy },
      ],
    })
      // Override component imports to use mocks for shallow testing
      // This prevents deep dependency injection chains that require complex setup
      .overrideComponent(LessonViewComponent, {
        remove: {
          imports: [RelatedConceptsPanelComponent, MiniGraphComponent],
        },
        add: {
          imports: [MockMiniGraphComponent, MockRelatedConceptsPanelComponent],
        },
      })
      .compileComponents();

    fixture = TestBed.createComponent(LessonViewComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('initialization', () => {
    it('should initialize with default values', () => {
      expect(component.explorationMode).toBe('path');
      expect(component.showInlineQuiz).toBe(false);
      expect(component.explorationPanelOpen).toBe(false);
      expect(component.hasRegisteredRenderer).toBe(false);
    });

    it('should accept required content input', () => {
      component.content = mockContent;
      fixture.detectChanges();
      expect(component.content).toEqual(mockContent);
    });

    it('should accept optional inputs', () => {
      component.content = mockContent;
      component.pathContext = mockPathContext;
      component.explorationMode = 'standalone';
      component.humanId = 'test-human';
      component.showInlineQuiz = true;
      fixture.detectChanges();

      expect(component.pathContext).toEqual(mockPathContext);
      expect(component.explorationMode).toBe('standalone');
      expect(component.humanId).toBe('test-human');
      expect(component.showInlineQuiz).toBe(true);
    });
  });

  describe('content rendering', () => {
    beforeEach(() => {
      // Set content BEFORE first detectChanges (required input)
      component.content = mockContent;
    });

    it('should display content title', () => {
      fixture.detectChanges();
      const compiled = fixture.nativeElement as HTMLElement;
      const title = compiled.querySelector('.lesson-title');
      expect(title?.textContent).toContain('Test Concept');
    });

    it('should display content description', () => {
      fixture.detectChanges();
      const compiled = fixture.nativeElement as HTMLElement;
      const description = compiled.querySelector('.lesson-description');
      expect(description?.textContent).toContain('A test concept for learning');
    });

    it('should display content type badge', () => {
      fixture.detectChanges();
      const compiled = fixture.nativeElement as HTMLElement;
      const badge = compiled.querySelector('.content-type-badge');
      expect(badge?.textContent).toContain('Concept');
    });

    it('should display tags (max 3)', () => {
      fixture.detectChanges();
      const compiled = fixture.nativeElement as HTMLElement;
      const tags = compiled.querySelectorAll('.tag');
      expect(tags.length).toBeLessThanOrEqual(3);
    });

    it('should use fallback for content without description', () => {
      component.content = { ...mockContent, description: '' } as ContentNode;
      fixture.detectChanges();
      const compiled = fixture.nativeElement as HTMLElement;
      const description = compiled.querySelector('.lesson-description');
      expect(description).toBeNull();
    });

    it('should render markdown fallback when no renderer registered', fakeAsync(() => {
      fixture.detectChanges();
      tick(); // Wait for setTimeout(0) in ngOnChanges
      expect(component.hasRegisteredRenderer).toBe(false);
      const compiled = fixture.nativeElement as HTMLElement;
      const fallback = compiled.querySelector('.content-fallback');
      expect(fallback).toBeTruthy();
    }));

    it('should load registered renderer when available', fakeAsync(() => {
      rendererRegistrySpy.getRenderer.and.returnValue(MockRendererComponent);

      // Manually trigger ngOnChanges
      component.ngOnChanges({
        content: {
          currentValue: mockContent,
          previousValue: undefined,
          firstChange: true,
          isFirstChange: () => true,
        },
      });
      fixture.detectChanges();
      tick(); // Execute setTimeout(0) in loadRenderer
      fixture.detectChanges(); // Apply renderer creation changes

      expect(component.hasRegisteredRenderer).toBe(true);
      expect(rendererRegistrySpy.getRenderer).toHaveBeenCalledWith(mockContent);
    }));

    it('should handle content format changes', fakeAsync(() => {
      component.ngOnChanges({
        content: {
          currentValue: mockContent,
          previousValue: undefined,
          firstChange: true,
          isFirstChange: () => true,
        },
      });
      fixture.detectChanges();
      tick();

      const newContent = { ...mockContent, contentFormat: 'html' as ContentNode['contentFormat'] };
      component.content = newContent;
      component.ngOnChanges({
        content: {
          currentValue: newContent,
          previousValue: mockContent,
          firstChange: false,
          isFirstChange: () => false,
        },
      });
      fixture.detectChanges();
      tick();

      expect(rendererRegistrySpy.getRenderer).toHaveBeenCalled();
    }));
  });

  describe('getContentTypeLabel', () => {
    it('should return correct label for known types', () => {
      component.content = { ...mockContent, contentType: 'epic' };
      expect(component.getContentTypeLabel()).toBe('Epic');

      component.content = { ...mockContent, contentType: 'feature' };
      expect(component.getContentTypeLabel()).toBe('Feature');

      component.content = { ...mockContent, contentType: 'scenario' };
      expect(component.getContentTypeLabel()).toBe('Scenario');

      component.content = { ...mockContent, contentType: 'simulation' };
      expect(component.getContentTypeLabel()).toBe('Simulation');
    });

    it('should return content type for unknown types', () => {
      component.content = { ...mockContent, contentType: 'custom-type' as any };
      expect(component.getContentTypeLabel()).toBe('custom-type');
    });

    it('should handle discovery-assessment type', () => {
      component.content = { ...mockContent, contentType: 'discovery-assessment' };
      expect(component.getContentTypeLabel()).toBe('Self-Discovery');
    });
  });

  describe('getContentString', () => {
    it('should return content as string', () => {
      component.content = mockContent;
      const contentStr = component.getContentString();
      expect(contentStr).toBe('# Test Concept\n\nThis is test content.');
    });

    it('should handle empty content', () => {
      component.content = { ...mockContent, content: '' };
      expect(component.getContentString()).toBe('');
    });

    it('should stringify object content', () => {
      component.content = {
        ...mockContent,
        content: { key: 'value', nested: { prop: 42 } },
      };
      const result = component.getContentString();
      expect(result).toContain('"key"');
      expect(result).toContain('"value"');
    });

    it('should handle null content', () => {
      component.content = { ...mockContent, content: null as any };
      expect(component.getContentString()).toBe('');
    });
  });

  describe('isMarkdown', () => {
    it('should return true for markdown format', () => {
      component.content = { ...mockContent, contentFormat: 'markdown' };
      expect(component.isMarkdown()).toBe(true);
    });

    it('should return false for non-markdown formats', () => {
      component.content = { ...mockContent, contentFormat: 'html' };
      expect(component.isMarkdown()).toBe(false);

      component.content = { ...mockContent, contentFormat: 'sophia-quiz-json' };
      expect(component.isMarkdown()).toBe(false);
    });
  });

  describe('exploration panel', () => {
    beforeEach(() => {
      // Set content BEFORE first detectChanges (required input)
      component.content = mockContent;
      fixture.detectChanges();
    });

    it('should toggle exploration panel', () => {
      expect(component.explorationPanelOpen).toBe(false);
      component.toggleExplorationPanel();
      expect(component.explorationPanelOpen).toBe(true);
      component.toggleExplorationPanel();
      expect(component.explorationPanelOpen).toBe(false);
    });

    it('should show panel toggle button', () => {
      const compiled = fixture.nativeElement as HTMLElement;
      const toggleBtn = compiled.querySelector('.panel-toggle');
      expect(toggleBtn).toBeTruthy();
    });

    it('should include mini-graph in panel', () => {
      const compiled = fixture.nativeElement as HTMLElement;
      const miniGraph = compiled.querySelector('app-mini-graph');
      expect(miniGraph).toBeTruthy();
    });

    it('should include related concepts panel', () => {
      const compiled = fixture.nativeElement as HTMLElement;
      const relatedPanel = compiled.querySelector('app-related-concepts-panel');
      expect(relatedPanel).toBeTruthy();
    });

    it('should have explore in graph button', () => {
      const compiled = fixture.nativeElement as HTMLElement;
      const exploreBtn = compiled.querySelector('.btn-explore-graph');
      expect(exploreBtn).toBeTruthy();
    });
  });

  describe('event emissions', () => {
    beforeEach(() => {
      // Set content BEFORE first detectChanges (required input)
      component.content = mockContent;
      fixture.detectChanges();
    });

    it('should emit exploreContent when related concept clicked', done => {
      component.exploreContent.subscribe(conceptId => {
        expect(conceptId).toBe('related-1');
        done();
      });

      component.onRelatedConceptClick('related-1');
    });

    it('should emit exploreContent when graph node clicked', done => {
      component.exploreContent.subscribe(nodeId => {
        expect(nodeId).toBe('node-123');
        done();
      });

      component.onGraphNodeClick('node-123');
    });

    it('should emit exploreInGraph when explore button clicked', done => {
      component.exploreInGraph.subscribe(() => {
        expect(true).toBe(true);
        done();
      });

      component.onExploreInGraphClick();
    });

    it('should emit quizCompleted when inline quiz completes', done => {
      const quizEvent = { streak: 3, totalCorrect: 5 };
      component.quizCompleted.subscribe(event => {
        expect(event).toEqual(quizEvent);
        done();
      });

      component.onInlineQuizCompleted(quizEvent);
    });

    it('should emit practicedEarned when attestation earned', done => {
      component.practicedEarned.subscribe(() => {
        expect(true).toBe(true);
        done();
      });

      component.onPracticedAttestation();
    });
  });

  describe('renderer lifecycle', () => {
    it('should create renderer on content change', fakeAsync(() => {
      rendererRegistrySpy.getRenderer.and.returnValue(MockRendererComponent);

      // Set content and initialize component
      component.content = mockContent;
      // Manually trigger ngOnChanges since Angular may not detect the initial set
      component.ngOnChanges({
        content: {
          currentValue: mockContent,
          previousValue: undefined,
          firstChange: true,
          isFirstChange: () => true,
        },
      });
      fixture.detectChanges(); // Initialize ViewChild
      tick(); // Wait for setTimeout(0) in loadRenderer
      fixture.detectChanges(); // Apply renderer creation changes

      expect(component.hasRegisteredRenderer).toBe(true);
    }));

    it('should destroy previous renderer on content change', fakeAsync(() => {
      rendererRegistrySpy.getRenderer.and.returnValue(MockRendererComponent);

      component.content = mockContent;
      component.ngOnChanges({
        content: {
          currentValue: mockContent,
          previousValue: undefined,
          firstChange: true,
          isFirstChange: () => true,
        },
      });
      fixture.detectChanges();
      tick();

      const oldRendererExists = (component as any).rendererRef !== null;
      expect(oldRendererExists).toBe(true);

      const newContent = { ...mockContent, id: 'new-content' };
      component.content = newContent;
      component.ngOnChanges({
        content: {
          currentValue: newContent,
          previousValue: mockContent,
          firstChange: false,
          isFirstChange: () => false,
        },
      });
      fixture.detectChanges();
      tick();

      // Verify a new renderer was created (getRenderer called twice)
      expect(rendererRegistrySpy.getRenderer).toHaveBeenCalledTimes(2);
    }));

    it('should clean up renderer on destroy', fakeAsync(() => {
      rendererRegistrySpy.getRenderer.and.returnValue(MockRendererComponent);

      component.content = mockContent;
      component.ngOnChanges({
        content: {
          currentValue: mockContent,
          previousValue: undefined,
          firstChange: true,
          isFirstChange: () => true,
        },
      });
      fixture.detectChanges();
      tick();

      const rendererExists = (component as any).rendererRef !== null;
      expect(rendererExists).toBe(true);

      component.ngOnDestroy();

      // Verify renderer was cleaned up
      expect((component as any).rendererRef).toBeNull();
    }));

    it('should subscribe to renderer completion events', fakeAsync(() => {
      rendererRegistrySpy.getRenderer.and.returnValue(MockRendererComponent);

      let emittedEvent: any;
      component.complete.subscribe(event => {
        emittedEvent = event;
      });

      component.content = mockContent;
      component.ngOnChanges({
        content: {
          currentValue: mockContent,
          previousValue: undefined,
          firstChange: true,
          isFirstChange: () => true,
        },
      });
      fixture.detectChanges();
      tick();
      fixture.detectChanges();

      const renderer = (component as any).rendererRef?.instance;
      if (renderer?.complete) {
        renderer.complete.emit({ completed: true });
        expect(emittedEvent).toBeDefined();
      }
    }));

    it('should set embedded mode on renderer', fakeAsync(() => {
      rendererRegistrySpy.getRenderer.and.returnValue(MockRendererComponent);

      component.content = mockContent;
      component.ngOnChanges({
        content: {
          currentValue: mockContent,
          previousValue: undefined,
          firstChange: true,
          isFirstChange: () => true,
        },
      });
      fixture.detectChanges();
      tick();
      fixture.detectChanges();

      const renderer = (component as any).rendererRef?.instance;
      expect(renderer).toBeDefined();
      expect(renderer.embedded).toBe(true);
    }));
  });

  describe('refreshKey input', () => {
    it('should reload renderer when refreshKey changes', fakeAsync(() => {
      rendererRegistrySpy.getRenderer.and.returnValue(MockRendererComponent);

      component.content = mockContent;
      component.refreshKey = 1;
      component.ngOnChanges({
        content: {
          currentValue: mockContent,
          previousValue: undefined,
          firstChange: true,
          isFirstChange: () => true,
        },
        refreshKey: {
          currentValue: 1,
          previousValue: undefined,
          firstChange: true,
          isFirstChange: () => true,
        },
      });
      fixture.detectChanges();
      tick();
      fixture.detectChanges();

      expect(component.hasRegisteredRenderer).toBe(true);

      component.refreshKey = 2;
      component.ngOnChanges({
        refreshKey: {
          currentValue: 2,
          previousValue: 1,
          firstChange: false,
          isFirstChange: () => false,
        },
      });
      fixture.detectChanges();
      tick();

      // Verify getRenderer was called twice (once for content, once for refreshKey)
      expect(rendererRegistrySpy.getRenderer).toHaveBeenCalledTimes(2);
    }));

    it('should not reload on first refreshKey set', fakeAsync(() => {
      component.content = mockContent;
      component.refreshKey = 1;
      fixture.detectChanges();
      tick();

      // Should only call getRenderer once (from content change, not refreshKey)
      expect(rendererRegistrySpy.getRenderer.calls.count()).toBeLessThanOrEqual(1);
    }));
  });

  describe('exploration modes', () => {
    beforeEach(() => {
      // Set content BEFORE first detectChanges (required input)
      component.content = mockContent;
    });

    it('should apply path mode classes', () => {
      component.explorationMode = 'path';
      fixture.detectChanges();
      const compiled = fixture.nativeElement as HTMLElement;
      const container = compiled.querySelector('.lesson-view');
      expect(container?.classList.contains('standalone')).toBe(false);
    });

    it('should apply standalone mode classes', () => {
      component.explorationMode = 'standalone';
      fixture.detectChanges();
      const compiled = fixture.nativeElement as HTMLElement;
      const container = compiled.querySelector('.lesson-view');
      expect(container?.classList.contains('standalone')).toBe(true);
    });

    it('should show path context indicator when context provided', () => {
      component.pathContext = mockPathContext;
      fixture.detectChanges();
      const compiled = fixture.nativeElement as HTMLElement;
      const container = compiled.querySelector('.lesson-view');
      expect(container?.classList.contains('has-path-context')).toBe(true);
    });
  });

  describe('edge cases', () => {
    it('should handle content with no tags', () => {
      component.content = { ...mockContent, tags: [] };
      fixture.detectChanges();
      const compiled = fixture.nativeElement as HTMLElement;
      const tags = compiled.querySelectorAll('.tag');
      expect(tags.length).toBe(0);
    });

    it('should handle content with many tags (show only 3)', () => {
      component.content = {
        ...mockContent,
        tags: ['tag1', 'tag2', 'tag3', 'tag4', 'tag5'],
      };
      fixture.detectChanges();
      const compiled = fixture.nativeElement as HTMLElement;
      const tags = compiled.querySelectorAll('.tag');
      expect(tags.length).toBe(3);
    });

    it('should use content id as fallback title', () => {
      component.content = { ...mockContent, title: undefined as any };
      fixture.detectChanges();
      const compiled = fixture.nativeElement as HTMLElement;
      const title = compiled.querySelector('.lesson-title');
      expect(title?.textContent).toContain('test-concept');
    });

    it('should handle renderer that does not support embedded mode', fakeAsync(() => {
      @Component({ selector: 'app-simple-renderer', standalone: true, template: '' })
      class SimpleRendererComponent {
        @Input() node!: ContentNode;
        // No embedded input
      }

      rendererRegistrySpy.getRenderer.and.returnValue(SimpleRendererComponent);

      component.content = mockContent;
      component.ngOnChanges({
        content: {
          currentValue: mockContent,
          previousValue: undefined,
          firstChange: true,
          isFirstChange: () => true,
        },
      });
      fixture.detectChanges();
      tick();
      fixture.detectChanges();

      expect(component.hasRegisteredRenderer).toBe(true);
      // Should not throw when setting embedded
    }));

    it('should handle renderer without complete event', fakeAsync(() => {
      @Component({ selector: 'app-static-renderer', standalone: true, template: '' })
      class StaticRendererComponent {
        @Input() node!: ContentNode;
        @Input() embedded = false;
        // No complete output
      }

      rendererRegistrySpy.getRenderer.and.returnValue(StaticRendererComponent);

      component.content = mockContent;
      component.ngOnChanges({
        content: {
          currentValue: mockContent,
          previousValue: undefined,
          firstChange: true,
          isFirstChange: () => true,
        },
      });
      fixture.detectChanges();
      tick();
      fixture.detectChanges();

      expect(component.hasRegisteredRenderer).toBe(true);
      expect((component as any).rendererSubscription).toBeNull();
    }));

    it('should handle rapid content changes', fakeAsync(() => {
      rendererRegistrySpy.getRenderer.and.returnValue(MockRendererComponent);

      component.content = mockContent;
      component.ngOnChanges({
        content: {
          currentValue: mockContent,
          previousValue: undefined,
          firstChange: true,
          isFirstChange: () => true,
        },
      });
      fixture.detectChanges();
      tick();

      const content2 = { ...mockContent, id: 'content-2' };
      component.content = content2;
      component.ngOnChanges({
        content: {
          currentValue: content2,
          previousValue: mockContent,
          firstChange: false,
          isFirstChange: () => false,
        },
      });
      fixture.detectChanges();
      tick();

      const content3 = { ...mockContent, id: 'content-3' };
      component.content = content3;
      component.ngOnChanges({
        content: {
          currentValue: content3,
          previousValue: content2,
          firstChange: false,
          isFirstChange: () => false,
        },
      });
      fixture.detectChanges();
      tick();

      expect(component.hasRegisteredRenderer).toBe(true);
    }));

    it('should handle ViewChild not ready', () => {
      component.content = mockContent;
      // Simulate ViewChild not initialized
      (component as any).rendererHost = null;

      expect(() => {
        (component as any).loadRenderer();
      }).not.toThrow();

      expect(component.hasRegisteredRenderer).toBe(false);
    });
  });

  describe('accessibility', () => {
    beforeEach(() => {
      // Set content BEFORE first detectChanges (required input)
      component.content = mockContent;
      fixture.detectChanges();
    });

    it('should have aria attributes on panel toggle', () => {
      const compiled = fixture.nativeElement as HTMLElement;
      const toggle = compiled.querySelector('.panel-toggle');
      expect(toggle?.getAttribute('aria-expanded')).toBe('false');
      expect(toggle?.getAttribute('aria-controls')).toBe('exploration-panel');
    });

    it('should update aria-expanded when panel opens', () => {
      expect(component.explorationPanelOpen).toBe(false);
      component.toggleExplorationPanel();
      expect(component.explorationPanelOpen).toBe(true);
      // OnPush change detection requires explicit marking
      (component as any).cdr.markForCheck();
      fixture.detectChanges();
      const compiled = fixture.nativeElement as HTMLElement;
      const toggle = compiled.querySelector('.panel-toggle');
      expect(toggle?.getAttribute('aria-expanded')).toBe('true');
    });

    it('should have proper heading hierarchy', () => {
      const compiled = fixture.nativeElement as HTMLElement;
      const h1 = compiled.querySelector('h1');
      expect(h1).toBeTruthy();
      expect(h1?.classList.contains('lesson-title')).toBe(true);
    });
  });
});
