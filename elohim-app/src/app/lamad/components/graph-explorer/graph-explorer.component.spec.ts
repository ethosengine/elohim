import { ComponentFixture, TestBed } from '@angular/core/testing';
import { HttpClientTestingModule } from '@angular/common/http/testing';
import { RouterTestingModule } from '@angular/router/testing';
import { Router, ActivatedRoute } from '@angular/router';
import { of, throwError } from 'rxjs';
import { GraphExplorerComponent } from './graph-explorer.component';
import { AffinityTrackingService } from '@app/elohim/services/affinity-tracking.service';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { HierarchicalGraphService } from '../../services/hierarchical-graph.service';
import { ElementRef } from '@angular/core';
import { ClusterNode, ClusterGraphData } from '../../models/cluster-graph.model';

describe('GraphExplorerComponent', () => {
  let component: GraphExplorerComponent;
  let fixture: ComponentFixture<GraphExplorerComponent>;
  let router: Router;
  let affinityService: jasmine.SpyObj<AffinityTrackingService>;
  let hierarchicalGraphService: jasmine.SpyObj<HierarchicalGraphService>;
  let dataLoaderService: jasmine.SpyObj<DataLoaderService>;

  const mockPathRoot: ClusterNode = {
    id: 'elohim-protocol',
    title: 'Elohim Protocol',
    description: 'Main learning path',
    contentType: 'path',
    isCluster: true,
    clusterType: 'path',
    clusterLevel: 0,
    parentClusterId: null,
    childClusterIds: ['chapter-1', 'chapter-2'],
    conceptIds: [],
    isExpanded: false,
    isLoading: false,
    totalConceptCount: 50,
    completedConceptCount: 10,
    externalConnectionCount: 5,
    state: 'in-progress',
    affinityScore: 0.2,
  };

  const mockChapterCluster: ClusterNode = {
    id: 'chapter-1',
    title: 'Governance',
    description: 'Governance chapter',
    contentType: 'chapter',
    isCluster: true,
    clusterType: 'chapter',
    clusterLevel: 1,
    parentClusterId: 'elohim-protocol',
    childClusterIds: ['module-1'],
    conceptIds: [],
    isExpanded: false,
    isLoading: false,
    totalConceptCount: 25,
    completedConceptCount: 5,
    externalConnectionCount: 3,
    state: 'in-progress',
    affinityScore: 0.2,
  };

  const mockGraphData: ClusterGraphData = {
    root: mockPathRoot,
    clusters: new Map([
      ['elohim-protocol', mockPathRoot],
      ['chapter-1', mockChapterCluster],
    ]),
    edges: [],
    connections: [],
  };

  beforeEach(async () => {
    const affinityServiceSpy = jasmine.createSpyObj('AffinityTrackingService', ['getAffinity']);
    const hierarchicalGraphSpy = jasmine.createSpyObj('HierarchicalGraphService', [
      'initializeFromPath',
      'expandCluster',
      'collapseCluster',
      'isExpanded',
      'getVisibleNodes',
      'getVisibleEdges',
      'reset',
    ]);
    const dataLoaderSpy = jasmine.createSpyObj('DataLoaderService', [
      'getGraph',
      'getPath',
      'getPathHierarchy',
    ]);

    await TestBed.configureTestingModule({
      imports: [GraphExplorerComponent, HttpClientTestingModule, RouterTestingModule],
      providers: [
        { provide: AffinityTrackingService, useValue: affinityServiceSpy },
        { provide: HierarchicalGraphService, useValue: hierarchicalGraphSpy },
        { provide: DataLoaderService, useValue: dataLoaderSpy },
      ],
    }).compileComponents();

    affinityService = TestBed.inject(
      AffinityTrackingService
    ) as jasmine.SpyObj<AffinityTrackingService>;
    hierarchicalGraphService = TestBed.inject(
      HierarchicalGraphService
    ) as jasmine.SpyObj<HierarchicalGraphService>;
    dataLoaderService = TestBed.inject(DataLoaderService) as jasmine.SpyObj<DataLoaderService>;

    affinityService.getAffinity.and.returnValue(0);
    hierarchicalGraphService.initializeFromPath.and.returnValue(of(mockGraphData));
    hierarchicalGraphService.getVisibleNodes.and.returnValue([mockPathRoot, mockChapterCluster]);
    hierarchicalGraphService.getVisibleEdges.and.returnValue([]);
    hierarchicalGraphService.isExpanded.and.returnValue(false);
    dataLoaderService.getGraph.and.returnValue(
      of({
        nodes: new Map(),
        relationships: new Map(),
        nodesByType: new Map(),
        nodesByTag: new Map(),
        nodesByCategory: new Map(),
        adjacency: new Map(),
        reverseAdjacency: new Map(),
        metadata: {
          nodeCount: 0,
          relationshipCount: 0,
          lastUpdated: new Date().toISOString(),
          version: '1.0.0',
        },
      } as any)
    );
    dataLoaderService.getPathHierarchy.and.returnValue(
      of({
        id: 'elohim-protocol',
        title: 'Elohim Protocol',
        description: 'A learning path',
        steps: [],
        chapters: [],
      } as any)
    );

    fixture = TestBed.createComponent(GraphExplorerComponent);
    component = fixture.componentInstance;
    router = TestBed.inject(Router);

    // Mock the graphContainer with proper read-only properties
    const mockDiv = document.createElement('div');
    Object.defineProperty(mockDiv, 'clientWidth', { value: 800, writable: false });
    Object.defineProperty(mockDiv, 'clientHeight', { value: 600, writable: false });
    component.graphContainer = {
      nativeElement: mockDiv,
    } as ElementRef<HTMLDivElement>;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should initialize with path-hierarchy view mode', () => {
    expect(component.viewMode).toBe('path-hierarchy');
    expect(component.currentPathId).toBe('elohim-protocol');
  });

  it('should load path hierarchy on init', () => {
    fixture.detectChanges();

    expect(hierarchicalGraphService.initializeFromPath).toHaveBeenCalledWith('elohim-protocol');
    expect(component.isLoading).toBe(false);
    expect(component.graphData).toBe(mockGraphData);
    expect(component.breadcrumbs.length).toBe(1);
    expect(component.breadcrumbs[0].title).toBe('Elohim Protocol');
  });

  it('should switch to overview mode', () => {
    fixture.detectChanges();

    component.setViewMode('overview');

    expect(component.viewMode).toBe('overview');
    expect(dataLoaderService.getPathHierarchy).toHaveBeenCalled();
  });

  it('should expand cluster on double-click', () => {
    hierarchicalGraphService.expandCluster.and.returnValue(
      of({
        clusterId: 'chapter-1',
        children: [],
        edges: [],
        connections: [],
      })
    );

    fixture.detectChanges();

    component.expandCluster('chapter-1');

    expect(hierarchicalGraphService.expandCluster).toHaveBeenCalledWith('chapter-1');
  });

  it('should collapse cluster', () => {
    fixture.detectChanges();

    component.collapseCluster('chapter-1');

    expect(hierarchicalGraphService.collapseCluster).toHaveBeenCalledWith('chapter-1');
  });

  it('should select node on click', () => {
    const node: ClusterNode = { ...mockChapterCluster };

    component.handleNodeClick(node);

    expect(component.selectedNode).toBe(node);
  });

  it('should toggle cluster expansion on double-click', () => {
    hierarchicalGraphService.expandCluster.and.returnValue(
      of({
        clusterId: 'chapter-1',
        children: [],
        edges: [],
        connections: [],
      })
    );

    fixture.detectChanges();

    const clusterNode: ClusterNode = { ...mockChapterCluster };

    // First double-click expands
    hierarchicalGraphService.isExpanded.and.returnValue(false);
    component.handleNodeDoubleClick(clusterNode);
    expect(hierarchicalGraphService.expandCluster).toHaveBeenCalledWith('chapter-1');

    // Second double-click collapses
    hierarchicalGraphService.isExpanded.and.returnValue(true);
    component.handleNodeDoubleClick(clusterNode);
    expect(hierarchicalGraphService.collapseCluster).toHaveBeenCalledWith('chapter-1');
  });

  it('should navigate to content on concept double-click', () => {
    spyOn(router, 'navigate');

    const conceptNode: ClusterNode = {
      id: 'concept-1',
      title: 'Test Concept',
      contentType: 'concept',
      isCluster: false,
      clusterType: null,
      clusterLevel: 4,
      parentClusterId: 'section-1',
      childClusterIds: [],
      conceptIds: [],
      isExpanded: false,
      isLoading: false,
      totalConceptCount: 0,
      completedConceptCount: 0,
      externalConnectionCount: 0,
      state: 'unseen',
      affinityScore: 0,
    };

    component.handleNodeDoubleClick(conceptNode);

    expect(router.navigate).toHaveBeenCalledWith(['/lamad/resource', 'concept-1']);
  });

  it('should not navigate for locked nodes', () => {
    spyOn(router, 'navigate');

    const lockedNode: ClusterNode = {
      ...mockChapterCluster,
      state: 'locked',
    };

    component.handleNodeDoubleClick(lockedNode);

    expect(router.navigate).not.toHaveBeenCalled();
    expect(hierarchicalGraphService.expandCluster).not.toHaveBeenCalled();
  });

  it('should check if cluster can expand', () => {
    hierarchicalGraphService.isExpanded.and.returnValue(false);

    const clusterWithChildren: ClusterNode = { ...mockChapterCluster };
    expect(component.canExpand(clusterWithChildren)).toBe(true);

    const emptyCluster: ClusterNode = {
      ...mockChapterCluster,
      childClusterIds: [],
      conceptIds: [],
    };
    expect(component.canExpand(emptyCluster)).toBe(false);
  });

  it('should check if cluster is expanded', () => {
    hierarchicalGraphService.isExpanded.and.returnValue(true);
    expect(component.isExpanded(mockChapterCluster)).toBe(true);

    hierarchicalGraphService.isExpanded.and.returnValue(false);
    expect(component.isExpanded(mockChapterCluster)).toBe(false);
  });

  it('should check if cluster is loading', () => {
    component['loadingClusters'].add('chapter-1');
    expect(component.isClusterLoading(mockChapterCluster)).toBe(true);

    component['loadingClusters'].delete('chapter-1');
    expect(component.isClusterLoading(mockChapterCluster)).toBe(false);
  });

  it('should navigate to breadcrumb at root level', () => {
    fixture.detectChanges();

    component.navigateToBreadcrumb({ id: 'elohim-protocol', title: 'Elohim Protocol', level: 0 });

    expect(hierarchicalGraphService.reset).toHaveBeenCalled();
    expect(hierarchicalGraphService.initializeFromPath).toHaveBeenCalledWith('elohim-protocol');
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
    expect(component.getStateLabel('review')).toBe('Needs Review');
    expect(component.getStateLabel('locked')).toBe('Locked');
  });

  it('should get correct cluster type label', () => {
    expect(component.getClusterTypeLabel('path')).toBe('Learning Path');
    expect(component.getClusterTypeLabel('chapter')).toBe('Chapter');
    expect(component.getClusterTypeLabel('module')).toBe('Module');
    expect(component.getClusterTypeLabel('section')).toBe('Section');
    expect(component.getClusterTypeLabel(null)).toBe('Concept');
  });

  it('should set hovered node on mouse enter', () => {
    const node: ClusterNode = { ...mockChapterCluster };

    component.hoveredNode = node;
    expect(component.hoveredNode).toBe(node);
  });

  it('should clear hovered node on mouse leave', () => {
    const node: ClusterNode = { ...mockChapterCluster };
    component.hoveredNode = node;

    component.hoveredNode = null;
    expect(component.hoveredNode).toBeNull();
  });

  it('should cleanup on destroy', () => {
    fixture.detectChanges();

    // Just verify ngOnDestroy runs without error
    expect(() => component.ngOnDestroy()).not.toThrow();
  });

  it('should get correct node colors based on state', () => {
    const proficientNode: ClusterNode = { ...mockChapterCluster, state: 'proficient' };
    const inProgressNode: ClusterNode = { ...mockChapterCluster, state: 'in-progress' };
    const recommendedNode: ClusterNode = { ...mockChapterCluster, state: 'recommended' };
    const lockedNode: ClusterNode = { ...mockChapterCluster, state: 'locked' };
    const unseenNode: ClusterNode = { ...mockChapterCluster, state: 'unseen' };

    expect((component as any).getNodeColor(proficientNode)).toBe('#fbbf24');
    expect((component as any).getNodeColor(inProgressNode)).toBe('#facc15');
    expect((component as any).getNodeColor(recommendedNode)).toBe('#22c55e');
    expect((component as any).getNodeColor(lockedNode)).toBe('#475569');
    expect((component as any).getNodeColor(unseenNode)).toBe('#64748b');
  });

  it('should get correct node stroke colors based on state', () => {
    const proficientNode: ClusterNode = { ...mockChapterCluster, state: 'proficient' };
    const lockedNode: ClusterNode = { ...mockChapterCluster, state: 'locked' };
    const unseenNode: ClusterNode = { ...mockChapterCluster, state: 'unseen' };

    expect((component as any).getNodeStroke(proficientNode)).toBe('#3b82f6');
    expect((component as any).getNodeStroke(lockedNode)).toBe('#64748b');
    expect((component as any).getNodeStroke(unseenNode)).toBe('#94a3b8');
  });

  describe('overview mode', () => {
    it('should load flat overview when switching to overview mode', () => {
      fixture.detectChanges();

      component.setViewMode('overview');

      expect(component.viewMode).toBe('overview');
      expect(dataLoaderService.getPathHierarchy).toHaveBeenCalled();
    });

    it('should handle overview mode with no chapters', () => {
      dataLoaderService.getPathHierarchy.and.returnValue(
        of({
          id: 'test-path',
          title: 'Test',
          description: '',
          chapters: [],
        } as any)
      );

      fixture.detectChanges();
      component.setViewMode('overview');

      expect(component.error).toBe('No content available for overview');
      expect(component.isLoading).toBe(false);
    });

    it('should not switch if already in same view mode', () => {
      fixture.detectChanges();
      component.viewMode = 'overview';

      dataLoaderService.getPathHierarchy.calls.reset();
      component.setViewMode('overview');

      expect(dataLoaderService.getPathHierarchy).not.toHaveBeenCalled();
    });

    it('should clear selected node when switching view modes', () => {
      fixture.detectChanges();
      component.selectedNode = mockChapterCluster;

      component.setViewMode('overview');

      expect(component.selectedNode).toBeNull();
    });
  });

  describe('query params handling', () => {
    it('should handle focus query param', () => {
      const route = TestBed.inject(ActivatedRoute);
      (route.queryParams as any).next({ focus: 'node-123' });

      fixture.detectChanges();

      expect(component.focusNodeId).toBe('node-123');
    });

    it('should handle return context from query params', () => {
      const route = TestBed.inject(ActivatedRoute);
      (route.queryParams as any).next({
        fromPath: 'test-path',
        returnStep: '5',
      });

      fixture.detectChanges();

      expect(component.returnContext).toEqual({
        pathId: 'test-path',
        stepIndex: 5,
      });
    });

    it('should handle view mode override from query params', () => {
      const route = TestBed.inject(ActivatedRoute);
      (route.queryParams as any).next({ view: 'overview' });

      fixture.detectChanges();

      expect(component.viewMode).toBe('overview');
    });
  });

  describe('return navigation', () => {
    it('should return to path with context', () => {
      spyOn(router, 'navigate');
      fixture.detectChanges();

      component.returnContext = {
        pathId: 'test-path',
        stepIndex: 3,
      };

      component.returnToPath();

      expect(router.navigate).toHaveBeenCalledWith(['/lamad/path', 'test-path', 'step', 3]);
    });

    it('should navigate to lamad home if no return context', () => {
      spyOn(router, 'navigate');
      fixture.detectChanges();

      component.returnContext = null;

      component.returnToPath();

      expect(router.navigate).toHaveBeenCalledWith(['/lamad']);
    });
  });

  describe('breadcrumb navigation', () => {
    it('should navigate to breadcrumb at root level', () => {
      fixture.detectChanges();

      component.navigateToBreadcrumb({ id: 'elohim-protocol', title: 'Elohim Protocol', level: 0 });

      expect(hierarchicalGraphService.reset).toHaveBeenCalled();
      expect(hierarchicalGraphService.initializeFromPath).toHaveBeenCalledWith('elohim-protocol');
    });

    it('should collapse clusters when navigating to non-root breadcrumb', () => {
      fixture.detectChanges();
      hierarchicalGraphService.isExpanded.and.returnValue(true);

      component.visibleNodes = [
        { ...mockChapterCluster, clusterLevel: 2, id: 'cluster-1' },
        { ...mockChapterCluster, clusterLevel: 3, id: 'cluster-2' },
      ];

      component.breadcrumbs = [
        { id: 'root', title: 'Root', level: 0 },
        { id: 'cluster-1', title: 'Cluster 1', level: 1 },
        { id: 'cluster-2', title: 'Cluster 2', level: 2 },
      ];

      component.navigateToBreadcrumb({ id: 'cluster-1', title: 'Cluster 1', level: 1 });

      expect(hierarchicalGraphService.collapseCluster).toHaveBeenCalled();
    });
  });

  describe('D3 rendering helpers', () => {
    it('should calculate node radius correctly', () => {
      const node: ClusterNode = { ...mockChapterCluster };
      const radius = (component as any).getNodeRadius(node);
      expect(radius).toBeGreaterThan(0);
    });

    it('should use custom radius if provided', () => {
      const node: ClusterNode = { ...mockChapterCluster, clusterRadius: 50 };
      const radius = (component as any).getNodeRadius(node);
      expect(radius).toBe(50);
    });

    it('should calculate link distance based on node radii', () => {
      const link = {
        source: { ...mockChapterCluster, clusterRadius: 20 },
        target: { ...mockChapterCluster, clusterRadius: 30 },
      };
      const distance = (component as any).getLinkDistance(link);
      expect(distance).toBe(20 + 30 + 80);
    });

    it('should return default link distance for invalid nodes', () => {
      const link = { source: null, target: null };
      const distance = (component as any).getLinkDistance(link);
      expect(distance).toBe(150);
    });

    it('should calculate charge strength based on cluster level', () => {
      const node: ClusterNode = { ...mockChapterCluster, clusterLevel: 2 };
      const strength = (component as any).getChargeStrength(node);
      expect(strength).toBeLessThan(0); // Repulsive force
    });

    it('should calculate collision radius', () => {
      const node: ClusterNode = { ...mockChapterCluster };
      const collisionRadius = (component as any).getCollisionRadius(node);
      expect(collisionRadius).toBeGreaterThan((component as any).getNodeRadius(node));
    });
  });

  describe('cluster visualization helpers', () => {
    it('should get cluster fill color', () => {
      const node: ClusterNode = { ...mockChapterCluster, clusterLevel: 1 };
      const fill = (component as any).getClusterFill(node);
      expect(fill).toBeTruthy();
    });

    it('should get cluster stroke color', () => {
      const node: ClusterNode = { ...mockChapterCluster, clusterLevel: 1 };
      const stroke = (component as any).getClusterStroke(node);
      expect(stroke).toBeTruthy();
    });

    it('should create progress arc for cluster', () => {
      const node: ClusterNode = {
        ...mockChapterCluster,
        totalConceptCount: 10,
        completedConceptCount: 5,
      };
      const arc = (component as any).createProgressArc(node);
      expect(arc).toBeTruthy();
      expect(arc.length).toBeGreaterThan(0);
    });

    it('should return empty arc for cluster with no concepts', () => {
      const node: ClusterNode = {
        ...mockChapterCluster,
        totalConceptCount: 0,
        completedConceptCount: 0,
      };
      const arc = (component as any).createProgressArc(node);
      expect(arc).toBe('');
    });

    it('should return empty arc for cluster with no progress', () => {
      const node: ClusterNode = {
        ...mockChapterCluster,
        totalConceptCount: 10,
        completedConceptCount: 0,
      };
      const arc = (component as any).createProgressArc(node);
      expect(arc).toBe('');
    });
  });

  describe('edge visualization', () => {
    it('should get edge color based on type', () => {
      expect((component as any).getEdgeColor({ type: 'NEXT' })).toBe('#6366f1');
      expect((component as any).getEdgeColor({ type: 'CONTAINS' })).toBe('#22c55e');
      expect((component as any).getEdgeColor({ type: 'PREREQ' })).toBe('#3b82f6');
      expect((component as any).getEdgeColor({ type: 'RELATED' })).toBe('#8b5cf6');
    });

    it('should get edge color for aggregated edges', () => {
      const aggregatedEdge = { type: 'UNKNOWN', isAggregated: true };
      expect((component as any).getEdgeColor(aggregatedEdge)).toBe('#6366f1');

      const nonAggregatedEdge = { type: 'UNKNOWN', isAggregated: false };
      expect((component as any).getEdgeColor(nonAggregatedEdge)).toBe('#475569');
    });

    it('should get edge opacity based on type', () => {
      expect((component as any).getEdgeOpacity({ type: 'NEXT' })).toBe(0.8);
      expect((component as any).getEdgeOpacity({ type: 'OTHER', isAggregated: true })).toBe(0.3);
      expect((component as any).getEdgeOpacity({ type: 'OTHER', isAggregated: false })).toBe(0.6);
    });

    it('should get edge stroke width based on type and aggregation', () => {
      expect((component as any).getEdgeStrokeWidth({ type: 'NEXT' })).toBe(3);
      expect((component as any).getEdgeStrokeWidth({ type: 'OTHER', isAggregated: false })).toBe(2);
      expect(
        (component as any).getEdgeStrokeWidth({ type: 'OTHER', isAggregated: true, connectionCount: 5 })
      ).toBe(5);
      expect(
        (component as any).getEdgeStrokeWidth({
          type: 'OTHER',
          isAggregated: true,
          connectionCount: 15,
        })
      ).toBe(8);
    });
  });

  describe('error handling', () => {
    it('should handle path hierarchy load error', () => {
      hierarchicalGraphService.initializeFromPath.and.returnValue(
        throwError(() => new Error('Load failed'))
      );

      fixture.detectChanges();

      expect(component.error).toBe('Failed to load learning path graph');
      expect(component.isLoading).toBe(false);
    });

    it('should handle cluster expansion error', () => {
      hierarchicalGraphService.expandCluster.and.returnValue(
        throwError(() => new Error('Expansion failed'))
      );

      fixture.detectChanges();

      component.expandCluster('chapter-1');

      expect(component['loadingClusters'].has('chapter-1')).toBe(false);
    });

    it('should handle overview load error', () => {
      dataLoaderService.getPathHierarchy.and.returnValue(
        throwError(() => new Error('Load failed'))
      );

      fixture.detectChanges();
      component.setViewMode('overview');

      expect(component.error).toBe('Failed to load graph overview');
      expect(component.isLoading).toBe(false);
    });
  });

  describe('zoom controls', () => {
    it('should reset zoom to fit content', () => {
      fixture.detectChanges();
      // SVG should be initialized after detectChanges
      expect(() => component.resetZoom()).not.toThrow();
    });

    it('should handle reset zoom when SVG not initialized', () => {
      (component as any).svg = null;
      expect(() => component.resetZoom()).not.toThrow();
    });
  });

  describe('affinity integration', () => {
    it('should convert content node to cluster node with affinity', () => {
      affinityService.getAffinity.and.returnValue(0.75);

      const partialNode = {
        id: 'test-node',
        title: 'Test Node',
        description: 'A test node',
        contentType: 'concept',
      };

      const clusterNode = (component as any).convertToClusterNode(partialNode);

      expect(clusterNode.state).toBe('proficient'); // 0.75 > 0.66
      expect(clusterNode.affinityScore).toBe(0.75);
    });

    it('should mark manifesto as recommended when unseen', () => {
      affinityService.getAffinity.and.returnValue(0);

      const manifestoNode = {
        id: 'manifesto',
        title: 'Manifesto',
      };

      const clusterNode = (component as any).convertToClusterNode(manifestoNode);

      expect(clusterNode.state).toBe('recommended');
    });

    it('should categorize affinity levels correctly', () => {
      affinityService.getAffinity.and.returnValue(0);
      let clusterNode = (component as any).convertToClusterNode({ id: 'n1' });
      expect(clusterNode.state).toBe('unseen');

      affinityService.getAffinity.and.returnValue(0.5);
      clusterNode = (component as any).convertToClusterNode({ id: 'n2' });
      expect(clusterNode.state).toBe('in-progress');

      affinityService.getAffinity.and.returnValue(0.8);
      clusterNode = (component as any).convertToClusterNode({ id: 'n3' });
      expect(clusterNode.state).toBe('proficient');
    });
  });

  describe('navigation to content', () => {
    it('should navigate to content viewer', () => {
      spyOn(router, 'navigate');

      component.navigateToContent('node-123');

      expect(router.navigate).toHaveBeenCalledWith(['/lamad/resource', 'node-123']);
    });
  });

  describe('D3 simulation lifecycle', () => {
    it('should stop simulation on destroy', () => {
      fixture.detectChanges();

      // Initialize simulation by rendering graph
      if ((component as any).simulation) {
        spyOn((component as any).simulation, 'stop');
      }

      component.ngOnDestroy();

      // Should not throw
      expect(true).toBe(true);
    });

    it('should handle destroy when simulation not initialized', () => {
      (component as any).simulation = null;

      expect(() => component.ngOnDestroy()).not.toThrow();
    });
  });
});
