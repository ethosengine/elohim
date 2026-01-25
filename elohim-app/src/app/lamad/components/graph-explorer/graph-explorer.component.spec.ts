import { ComponentFixture, TestBed } from '@angular/core/testing';
import { HttpClientTestingModule } from '@angular/common/http/testing';
import { RouterTestingModule } from '@angular/router/testing';
import { Router } from '@angular/router';
import { of } from 'rxjs';
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
});
