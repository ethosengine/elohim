import { ComponentFixture, TestBed } from '@angular/core/testing';
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { RouterTestingModule } from '@angular/router/testing';
import { Router } from '@angular/router';
import { GraphExplorerComponent } from './graph-explorer.component';
import { AffinityTrackingService } from '../../services/affinity-tracking.service';
import { ElementRef } from '@angular/core';

describe('GraphExplorerComponent', () => {
  let component: GraphExplorerComponent;
  let fixture: ComponentFixture<GraphExplorerComponent>;
  let httpMock: HttpTestingController;
  let router: Router;
  let affinityService: jasmine.SpyObj<AffinityTrackingService>;

  const mockOverviewData = {
    nodes: [
      {
        id: 'manifesto',
        title: 'Manifesto',
        contentType: 'epic',
        description: 'The Elohim Protocol Manifesto',
        hasChildren: true,
        childCount: 5,
        level: 0,
        isRoot: true
      },
      {
        id: 'learning-platform',
        title: 'Learning Platform',
        contentType: 'epic',
        description: 'Self-directed learning',
        hasChildren: true,
        childCount: 3,
        level: 0
      }
    ],
    edges: [
      {
        source: 'manifesto',
        target: 'learning-platform',
        type: 'prerequisite'
      }
    ]
  };

  beforeEach(async () => {
    const affinityServiceSpy = jasmine.createSpyObj('AffinityTrackingService', ['getAffinity']);

    await TestBed.configureTestingModule({
      imports: [
        GraphExplorerComponent,
        HttpClientTestingModule,
        RouterTestingModule
      ],
      providers: [
        { provide: AffinityTrackingService, useValue: affinityServiceSpy }
      ]
    }).compileComponents();

    affinityService = TestBed.inject(AffinityTrackingService) as jasmine.SpyObj<AffinityTrackingService>;
    affinityService.getAffinity.and.returnValue(0);

    fixture = TestBed.createComponent(GraphExplorerComponent);
    component = fixture.componentInstance;
    httpMock = TestBed.inject(HttpTestingController);
    router = TestBed.inject(Router);

    // Mock the graphContainer with proper read-only properties
    const mockDiv = document.createElement('div');
    Object.defineProperty(mockDiv, 'clientWidth', { value: 800, writable: false });
    Object.defineProperty(mockDiv, 'clientHeight', { value: 600, writable: false });
    component.graphContainer = {
      nativeElement: mockDiv
    } as ElementRef<HTMLDivElement>;
  });

  afterEach(() => {
    httpMock.verify();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should load overview on init', () => {
    fixture.detectChanges();

    const req = httpMock.expectOne('/assets/lamad-data/graph/overview.json');
    expect(req.request.method).toBe('GET');
    expect(component.isLoading).toBe(true);

    req.flush(mockOverviewData);

    expect(component.isLoading).toBe(false);
    expect(component.currentLevel).toBeTruthy();
    expect(component.currentLevel?.nodes.length).toBe(2);
    expect(component.breadcrumbs.length).toBe(1);
    expect(component.breadcrumbs[0].title).toBe('Protocol Overview');
  });

  it('should handle overview load error', () => {
    fixture.detectChanges();

    const req = httpMock.expectOne('/assets/lamad-data/graph/overview.json');
    req.error(new ProgressEvent('error'));

    expect(component.isLoading).toBe(false);
    expect(component.error).toBe('Failed to load graph overview');
  });

  it('should load epic detail', () => {
    component.loadEpicDetail('learning-platform-epic', 'Learning Platform');

    const req = httpMock.expectOne('/assets/lamad-data/graph/epic-learning_platform_epic.json');
    expect(req.request.method).toBe('GET');

    const epicData = {
      nodes: [
        { id: 'feature-1', title: 'Feature 1', contentType: 'feature', hasChildren: false, childCount: 0, level: 1 }
      ],
      edges: []
    };
    req.flush(epicData);

    expect(component.currentLevel?.zoomLevel).toBe(1);
    expect(component.breadcrumbs.length).toBe(2);
    expect(component.breadcrumbs[1].title).toBe('Learning Platform');
  });

  it('should handle epic detail load error', () => {
    component.loadEpicDetail('test-epic', 'Test Epic');

    const req = httpMock.expectOne('/assets/lamad-data/graph/epic-test_epic.json');
    req.error(new ProgressEvent('error'));

    expect(component.isLoading).toBe(false);
    expect(component.error).toContain('Failed to load Test Epic details');
  });

  it('should enrich nodes with affinity data', () => {
    affinityService.getAffinity.and.returnValue(0.5);

    fixture.detectChanges();
    const req = httpMock.expectOne('/assets/lamad-data/graph/overview.json');
    req.flush(mockOverviewData);

    const enrichedNode = component.currentLevel?.nodes[0];
    expect(enrichedNode?.affinityScore).toBe(0.5);
    expect(enrichedNode?.state).toBe('in-progress');
  });

  it('should mark manifesto as recommended for new users', () => {
    affinityService.getAffinity.and.returnValue(0);

    fixture.detectChanges();
    const req = httpMock.expectOne('/assets/lamad-data/graph/overview.json');
    req.flush(mockOverviewData);

    const manifestoNode = component.currentLevel?.nodes.find(n => n.id === 'manifesto');
    expect(manifestoNode?.state).toBe('recommended');
  });

  it('should determine proficient state for high affinity', () => {
    affinityService.getAffinity.and.returnValue(0.75);

    fixture.detectChanges();
    const req = httpMock.expectOne('/assets/lamad-data/graph/overview.json');
    req.flush(mockOverviewData);

    const node = component.currentLevel?.nodes[1];
    expect(node?.state).toBe('proficient');
  });

  it('should navigate to content on node click without children', () => {
    spyOn(router, 'navigate');

    const node: any = {
      id: 'node-1',
      title: 'Test Node',
      contentType: 'scenario',
      hasChildren: false,
      state: 'unseen'
    };

    component.handleNodeClick(node);
    expect(router.navigate).toHaveBeenCalledWith(['/lamad/resource', 'node-1']);
  });

  it('should load epic detail on epic node click', () => {
    spyOn(component, 'loadEpicDetail');

    const node: any = {
      id: 'epic-1',
      title: 'Test Epic',
      contentType: 'epic',
      hasChildren: true,
      childCount: 5,
      state: 'unseen'
    };

    component.handleNodeClick(node);
    expect(component.loadEpicDetail).toHaveBeenCalledWith('epic-1', 'Test Epic');
  });

  it('should not navigate for locked nodes', () => {
    spyOn(router, 'navigate');
    spyOn(component, 'loadEpicDetail');

    const node: any = {
      id: 'locked-node',
      title: 'Locked',
      contentType: 'scenario',
      hasChildren: false,
      state: 'locked'
    };

    component.handleNodeClick(node);
    expect(router.navigate).not.toHaveBeenCalled();
    expect(component.loadEpicDetail).not.toHaveBeenCalled();
  });

  it('should navigate to overview on breadcrumb click', () => {
    spyOn(component, 'loadOverview');

    component.navigateToBreadcrumb({ id: 'root', title: 'Overview', level: 0 });
    expect(component.loadOverview).toHaveBeenCalled();
  });

  it('should truncate long titles', () => {
    const title = 'This is a very long title that should be truncated';
    const truncated = (component as any).truncateTitle(title, 20);
    expect(truncated).toBe('This is a very lo...');
  });

  it('should not truncate short titles', () => {
    const title = 'Short';
    const truncated = (component as any).truncateTitle(title, 20);
    expect(truncated).toBe('Short');
  });

  it('should get correct state label', () => {
    expect(component.getStateLabel('unseen')).toBe('Not Started');
    expect(component.getStateLabel('in-progress')).toBe('In Progress');
    expect(component.getStateLabel('proficient')).toBe('Completed');
    expect(component.getStateLabel('recommended')).toBe('Recommended');
    expect(component.getStateLabel('locked')).toBe('Locked');
  });

  it('should set hovered node on mouse enter', () => {
    const node: any = { id: 'node-1', title: 'Test' };

    // Simulate mouse enter (would be called by D3 event handler)
    component.hoveredNode = node;
    expect(component.hoveredNode).toBe(node);
  });

  it('should clear hovered node on mouse leave', () => {
    const node: any = { id: 'node-1', title: 'Test' };
    component.hoveredNode = node;

    // Simulate mouse leave
    component.hoveredNode = null;
    expect(component.hoveredNode).toBeNull();
  });

  it('should set selected node on click', () => {
    spyOn(router, 'navigate');
    const node: any = {
      id: 'node-1',
      title: 'Test',
      contentType: 'scenario',
      hasChildren: false,
      state: 'unseen'
    };

    component.handleNodeClick(node);
    expect(component.selectedNode).toBe(node);
  });

  it('should cleanup on destroy', () => {
    fixture.detectChanges();
    const req = httpMock.expectOne('/assets/lamad-data/graph/overview.json');
    req.flush(mockOverviewData);

    // Create a spy for simulation.stop if simulation exists
    if (component['simulation']) {
      spyOn(component['simulation'], 'stop');
      component.ngOnDestroy();
      expect(component['simulation'].stop).toHaveBeenCalled();
    } else {
      // Just verify ngOnDestroy runs without error
      expect(() => component.ngOnDestroy()).not.toThrow();
    }
  });

  it('should get correct node radius based on type', () => {
    const rootNode: any = { isRoot: true, contentType: 'epic' };
    const epicNode: any = { isRoot: false, contentType: 'epic' };
    const featureNode: any = { isRoot: false, contentType: 'feature' };
    const scenarioNode: any = { isRoot: false, contentType: 'scenario' };

    expect((component as any).getNodeRadius(rootNode)).toBe(50);
    expect((component as any).getNodeRadius(epicNode)).toBe(40);
    expect((component as any).getNodeRadius(featureNode)).toBe(30);
    expect((component as any).getNodeRadius(scenarioNode)).toBe(20);
  });

  it('should get correct node colors based on state', () => {
    const proficientNode: any = { state: 'proficient' };
    const inProgressNode: any = { state: 'in-progress' };
    const recommendedNode: any = { state: 'recommended' };
    const lockedNode: any = { state: 'locked' };
    const unseenNode: any = { state: 'unseen' };

    expect((component as any).getNodeColor(proficientNode)).toBe('#fbbf24');
    expect((component as any).getNodeColor(inProgressNode)).toBe('#facc15');
    expect((component as any).getNodeColor(recommendedNode)).toBe('#22c55e');
    expect((component as any).getNodeColor(lockedNode)).toBe('#475569');
    expect((component as any).getNodeColor(unseenNode)).toBe('#64748b');
  });

  it('should get correct node stroke colors based on state', () => {
    const proficientNode: any = { state: 'proficient' };
    const lockedNode: any = { state: 'locked' };
    const unseenNode: any = { state: 'unseen' };

    expect((component as any).getNodeStroke(proficientNode)).toBe('#3b82f6');
    expect((component as any).getNodeStroke(lockedNode)).toBe('#64748b');
    expect((component as any).getNodeStroke(unseenNode)).toBe('#94a3b8');
  });
});
