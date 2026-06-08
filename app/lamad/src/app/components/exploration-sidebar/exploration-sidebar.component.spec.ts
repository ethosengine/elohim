import { ComponentFixture, TestBed } from '@angular/core/testing';
import { Component, EventEmitter, Input, Output } from '@angular/core';
import { By } from '@angular/platform-browser';
import { vi } from 'vitest';
import { ExplorationSidebarComponent } from './exploration-sidebar.component';
import { MiniGraphComponent } from '../mini-graph/mini-graph.component';
import { RelatedConceptsPanelComponent } from '../related-concepts-panel/related-concepts-panel.component';

// Mock children — standalone, selector-matched. This keeps the compositional
// wrapper's test shallow: no deep DI (d3, RelatedConceptsService, HttpClient).
@Component({ selector: 'app-mini-graph', standalone: true, template: '' })
class MockMiniGraphComponent {
  @Input() focusNodeId!: string;
  @Input() depth = 1;
  @Input() maxNodes = 15;
  @Input() height = 200;
  @Output() nodeSelected = new EventEmitter<string>();
  @Output() exploreRequested = new EventEmitter<void>();
}

@Component({ selector: 'app-related-concepts-panel', standalone: true, template: '' })
class MockRelatedConceptsPanelComponent {
  @Input() contentId!: string;
  @Input() showHierarchy = true;
  @Input() compact = false;
  @Input() limit = 5;
  @Output() navigate = new EventEmitter<string>();
}

describe('ExplorationSidebarComponent', () => {
  let component: ExplorationSidebarComponent;
  let fixture: ComponentFixture<ExplorationSidebarComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ExplorationSidebarComponent],
    })
      // Swap the real (DI-heavy) children for mock standalone components, matching
      // the bundle's shallow-render convention for compositional wrappers
      // (see lesson-view.component.spec.ts overrideComponent).
      .overrideComponent(ExplorationSidebarComponent, {
        remove: {
          imports: [MiniGraphComponent, RelatedConceptsPanelComponent],
        },
        add: {
          imports: [MockMiniGraphComponent, MockRelatedConceptsPanelComponent],
        },
      })
      .compileComponents();

    fixture = TestBed.createComponent(ExplorationSidebarComponent);
    component = fixture.componentInstance;
    component.contentId = 'test-content-id';
    component.open = true; // ensure off-canvas content is rendered for collapsible default
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('renders the mini-graph, related-concepts panel, and explore button', () => {
    const el: HTMLElement = fixture.nativeElement;
    expect(el.querySelector('app-mini-graph')).toBeTruthy();
    expect(el.querySelector('app-related-concepts-panel')).toBeTruthy();
    expect(el.querySelector('[data-testid="exploration-explore-graph"]')).toBeTruthy();
  });

  it('passes contentId through to both children', () => {
    const graphCmp = fixture.debugElement.query(By.directive(MockMiniGraphComponent))
      ?.componentInstance as MockMiniGraphComponent;
    const panelCmp = fixture.debugElement.query(By.directive(MockRelatedConceptsPanelComponent))
      ?.componentInstance as MockRelatedConceptsPanelComponent;
    expect(graphCmp).toBeTruthy();
    expect(panelCmp).toBeTruthy();
    expect(graphCmp.focusNodeId).toBe('test-content-id');
    expect(panelCmp.contentId).toBe('test-content-id');
  });

  it('emits exploreInGraph when the explore button is clicked', () => {
    const emitted = vi.fn();
    component.exploreInGraph.subscribe(emitted);
    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="exploration-explore-graph"]'
    );
    btn.click();
    expect(emitted).toHaveBeenCalledTimes(1);
  });

  it('emits exploreInGraph when the mini-graph requests explore', () => {
    const emitted = vi.fn();
    component.exploreInGraph.subscribe(emitted);
    const graph = fixture.debugElement.query(By.directive(MockMiniGraphComponent))
      .componentInstance as MockMiniGraphComponent;
    graph.exploreRequested.emit();
    expect(emitted).toHaveBeenCalledTimes(1);
  });

  it('emits exploreContent when the related-concepts panel navigates', () => {
    const emitted = vi.fn();
    component.exploreContent.subscribe(emitted);
    const panel = fixture.debugElement.query(By.directive(MockRelatedConceptsPanelComponent))
      .componentInstance as MockRelatedConceptsPanelComponent;
    panel.navigate.emit('next-concept-id');
    expect(emitted).toHaveBeenCalledWith('next-concept-id');
  });

  it('emits exploreContent when a mini-graph node is selected', () => {
    const emitted = vi.fn();
    component.exploreContent.subscribe(emitted);
    const graph = fixture.debugElement.query(By.directive(MockMiniGraphComponent))
      .componentInstance as MockMiniGraphComponent;
    graph.nodeSelected.emit('graph-node-id');
    expect(emitted).toHaveBeenCalledWith('graph-node-id');
  });

  it('supports two-way open binding via openChange', () => {
    const emitted = vi.fn();
    component.openChange.subscribe(emitted);
    component.open = true;
    fixture.detectChanges();
    // Closing via the close affordance emits openChange(false).
    const close: HTMLButtonElement | null = fixture.nativeElement.querySelector(
      '[data-testid="exploration-panel-close"]'
    );
    expect(close).toBeTruthy();
    close!.click();
    expect(emitted).toHaveBeenCalledWith(false);
  });

  it('when not collapsible, renders a pinned rail without a toggle/close', () => {
    // setInput marks the OnPush view dirty (a plain field mutation would not).
    fixture.componentRef.setInput('collapsible', false);
    fixture.detectChanges();
    const el: HTMLElement = fixture.nativeElement;
    expect(el.querySelector('app-mini-graph')).toBeTruthy();
    expect(el.querySelector('app-related-concepts-panel')).toBeTruthy();
    expect(el.querySelector('[data-testid="exploration-panel-toggle"]')).toBeNull();
    expect(el.querySelector('[data-testid="exploration-panel-close"]')).toBeNull();
  });
});
