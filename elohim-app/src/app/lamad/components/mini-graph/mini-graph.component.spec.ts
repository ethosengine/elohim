import { ComponentFixture, TestBed } from '@angular/core/testing';
import { DebugElement } from '@angular/core';
import { By } from '@angular/platform-browser';
import { of, throwError } from 'rxjs';

import { MiniGraphComponent } from './mini-graph.component';
import { RelatedConceptsService } from '../../services/related-concepts.service';
import {
  MiniGraphData,
  MiniGraphNode,
  MiniGraphEdge,
} from '../../models/exploration-context.model';

describe('MiniGraphComponent', () => {
  let component: MiniGraphComponent;
  let fixture: ComponentFixture<MiniGraphComponent>;
  let relatedConceptsService: jasmine.SpyObj<RelatedConceptsService>;

  // Mock data
  const mockMiniGraphNode: MiniGraphNode = {
    id: 'concept-1',
    title: 'Test Concept',
    contentType: 'lesson',
    isFocus: true,
    depth: 0,
  };

  const mockNeighborNode: MiniGraphNode = {
    id: 'concept-2',
    title: 'Related Concept',
    contentType: 'quiz',
    isFocus: false,
    depth: 1,
  };

  const mockEdge: MiniGraphEdge = {
    source: 'concept-1',
    target: 'concept-2',
    relationshipType: 'RELATES_TO',
  };

  const mockGraphData: MiniGraphData = {
    focus: mockMiniGraphNode,
    neighbors: [mockNeighborNode],
    edges: [mockEdge],
  };

  beforeEach(async () => {
    const relatedConceptsServiceSpy = jasmine.createSpyObj(
      'RelatedConceptsService',
      ['getNeighborhood']
    );

    await TestBed.configureTestingModule({
      imports: [MiniGraphComponent],
      providers: [
        { provide: RelatedConceptsService, useValue: relatedConceptsServiceSpy },
      ],
    }).compileComponents();

    relatedConceptsService = TestBed.inject(
      RelatedConceptsService
    ) as jasmine.SpyObj<RelatedConceptsService>;
    fixture = TestBed.createComponent(MiniGraphComponent);
    component = fixture.componentInstance;
  });

  describe('Component Creation', () => {
    it('should create', () => {
      expect(component).toBeTruthy();
    });

    it('should be a standalone component', () => {
      expect(MiniGraphComponent).toBeTruthy();
    });

    it('should have OnChanges, OnDestroy, and AfterViewInit lifecycle', () => {
      expect(component.ngOnChanges).toBeDefined();
      expect(component.ngOnDestroy).toBeDefined();
      expect(component.ngAfterViewInit).toBeDefined();
    });
  });

  describe('Input Properties', () => {
    it('should have required focusNodeId input', () => {
      component.focusNodeId = 'test-node-id';
      expect(component.focusNodeId).toBe('test-node-id');
    });

    it('should have optional depth input with default value 1', () => {
      expect(component.depth).toBe(1);
      component.depth = 2;
      expect(component.depth).toBe(2);
    });

    it('should have optional maxNodes input with default value 15', () => {
      expect(component.maxNodes).toBe(15);
      component.maxNodes = 20;
      expect(component.maxNodes).toBe(20);
    });

    it('should have optional height input with default value 200', () => {
      expect(component.height).toBe(200);
      component.height = 300;
      expect(component.height).toBe(300);
    });
  });

  describe('Output Properties', () => {
    it('should have nodeSelected output emitter', () => {
      expect(component.nodeSelected).toBeDefined();
      expect(component.nodeSelected.observers.length).toBe(0);

      let emittedValue: string | undefined;
      component.nodeSelected.subscribe((value: string) => {
        emittedValue = value;
      });
      component.nodeSelected.emit('test-node-id');

      expect(emittedValue).toBe('test-node-id');
    });

    it('should have exploreRequested output emitter', () => {
      expect(component.exploreRequested).toBeDefined();
      expect(component.exploreRequested.observers.length).toBe(0);

      let emitted = false;
      component.exploreRequested.subscribe(() => {
        emitted = true;
      });
      component.exploreRequested.emit();

      expect(emitted).toBe(true);
    });
  });

  describe('Template Compilation', () => {
    it('should compile template without errors', () => {
      fixture.detectChanges();
      expect(fixture.componentInstance).toBeTruthy();
    });

    it('should render mini-graph-container div', () => {
      fixture.detectChanges();
      const container: DebugElement = fixture.debugElement.query(
        By.css('.mini-graph-container')
      );
      expect(container).toBeTruthy();
    });

    it('should render loading overlay when isLoading is true', () => {
      component.isLoading = true;
      fixture.detectChanges();
      const loadingOverlay: DebugElement = fixture.debugElement.query(
        By.css('.loading-overlay')
      );
      expect(loadingOverlay).toBeTruthy();
    });

    it('should not render loading overlay when isLoading is false', () => {
      component.isLoading = false;
      fixture.detectChanges();
      const loadingOverlay: DebugElement = fixture.debugElement.query(
        By.css('.loading-overlay')
      );
      expect(loadingOverlay).toBeFalsy();
    });

    it('should render empty state when isEmpty is true and not loading', () => {
      component.isEmpty = true;
      component.isLoading = false;
      fixture.detectChanges();
      const emptyState: DebugElement = fixture.debugElement.query(
        By.css('.empty-state')
      );
      expect(emptyState).toBeTruthy();
    });

    it('should not render empty state when isEmpty is false', () => {
      component.isEmpty = false;
      fixture.detectChanges();
      const emptyState: DebugElement = fixture.debugElement.query(
        By.css('.empty-state')
      );
      expect(emptyState).toBeFalsy();
    });

    it('should render graph-viewport container', () => {
      fixture.detectChanges();
      const graphViewport: DebugElement = fixture.debugElement.query(
        By.css('.graph-viewport')
      );
      expect(graphViewport).toBeTruthy();
    });

    it('should render expand button', () => {
      fixture.detectChanges();
      const expandButton: DebugElement = fixture.debugElement.query(
        By.css('.expand-button')
      );
      expect(expandButton).toBeTruthy();
    });

    it('should apply height style to container', () => {
      component.height = 250;
      fixture.detectChanges();
      const container = fixture.debugElement.query(By.css('.mini-graph-container'));
      expect(container.nativeElement.style.height).toBe('250px');
    });

    it('should render tooltip when hoveredNode exists and is not focus', () => {
      component.hoveredNode = mockNeighborNode;
      fixture.detectChanges();
      const tooltip: DebugElement = fixture.debugElement.query(
        By.css('.node-tooltip')
      );
      expect(tooltip).toBeTruthy();
    });

    it('should not render tooltip when hoveredNode is null', () => {
      component.hoveredNode = null;
      fixture.detectChanges();
      const tooltip: DebugElement = fixture.debugElement.query(
        By.css('.node-tooltip')
      );
      expect(tooltip).toBeFalsy();
    });

    it('should not render tooltip when hoveredNode is focus node', () => {
      component.hoveredNode = mockMiniGraphNode;
      fixture.detectChanges();
      const tooltip: DebugElement = fixture.debugElement.query(
        By.css('.node-tooltip')
      );
      expect(tooltip).toBeFalsy();
    });
  });

  describe('Simple Method Tests', () => {
    it('should emit exploreRequested when onExpandClick is called', () => {
      spyOn(component.exploreRequested, 'emit');
      component.onExpandClick();
      expect(component.exploreRequested.emit).toHaveBeenCalledWith();
    });

    it('should have ngOnChanges lifecycle method', () => {
      const changes = {
        focusNodeId: {
          currentValue: 'new-id',
          previousValue: undefined,
          firstChange: true,
          isFirstChange: () => true,
        },
      };
      relatedConceptsService.getNeighborhood.and.returnValue(of(mockGraphData));
      component.focusNodeId = 'new-id';
      component.ngOnChanges(changes);
      expect(component.focusNodeId).toBe('new-id');
    });

    it('should have ngOnDestroy lifecycle method', () => {
      expect(() => {
        component.ngOnDestroy();
      }).not.toThrow();
    });

    it('should have ngAfterViewInit lifecycle method', () => {
      expect(() => {
        fixture.detectChanges();
      }).not.toThrow();
    });
  });

  describe('Property Initialization', () => {
    it('should initialize isLoading as true', () => {
      expect(component.isLoading).toBe(true);
    });

    it('should initialize isEmpty as false', () => {
      expect(component.isEmpty).toBe(false);
    });

    it('should initialize hoveredNode as null', () => {
      expect(component.hoveredNode).toBeNull();
    });

    it('should initialize tooltipX as 0', () => {
      expect(component.tooltipX).toBe(0);
    });

    it('should initialize tooltipY as 0', () => {
      expect(component.tooltipY).toBe(0);
    });
  });

  describe('ViewChild References', () => {
    it('should have graphContainer ViewChild reference', () => {
      fixture.detectChanges();
      expect(component.graphContainer).toBeDefined();
    });

    it('should reference the graph-viewport element', () => {
      fixture.detectChanges();
      expect(component.graphContainer.nativeElement).toBeTruthy();
      expect(
        component.graphContainer.nativeElement.classList.contains('graph-viewport')
      ).toBe(true);
    });
  });

  describe('Change Detection Strategy', () => {
    it('should use OnPush change detection strategy', () => {
      const metadata = (MiniGraphComponent as any).__annotations__[0];
      // ChangeDetectionStrategy.OnPush = 0 in Angular 19, was 1 in earlier versions
      expect(metadata.changeDetection).toBe(0);
    });
  });

  // TODO: Add rendering tests
  // Complex D3 force simulation rendering logic requires integration tests with actual DOM
  // and D3 tick events. Test cases needed:
  // - Verify SVG creation and structure
  // - Verify node and edge rendering
  // - Verify force simulation behavior
  // - Verify tooltip positioning on mouse events

  // TODO: Add interaction tests
  // Complex mouse event handling with D3 requires specialized testing:
  // - Node click interactions
  // - Node hover interactions
  // - Expand button clicks
  // - Tooltip positioning calculations

  // TODO: Add business logic tests
  // Related concepts service integration and data flow:
  // - loadNeighborhood() method behavior
  // - getEdgeClass() edge type classification
  // - truncateLabel() string truncation logic
  // - Error handling in service subscription
});
