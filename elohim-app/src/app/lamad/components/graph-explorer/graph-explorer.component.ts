import { CommonModule } from '@angular/common';
import { Component, OnInit, OnDestroy, ElementRef, ViewChild, AfterViewInit } from '@angular/core';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';

// @coverage: 84.6% (2026-02-05)

import { takeUntil } from 'rxjs/operators';

import * as d3 from 'd3';
import { Subject } from 'rxjs';

import { AffinityTrackingService } from '@app/elohim/services/affinity-tracking.service';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';

import {
  ClusterNode,
  ClusterEdge,
  ClusterGraphData,
  CLUSTER_LEVEL_CONFIG,
  calculateClusterRadius,
} from '../../models/cluster-graph.model';
import { HierarchicalGraphService } from '../../services/hierarchical-graph.service';

/**
 * View mode for the graph explorer.
 */
type ViewMode = 'path-hierarchy' | 'overview';

// Cluster state constants to avoid magic strings
const STATE_UNSEEN: ClusterNode['state'] = 'unseen';
const STATE_PROFICIENT: ClusterNode['state'] = 'proficient';
const STATE_IN_PROGRESS: ClusterNode['state'] = 'in-progress';
const STATE_RECOMMENDED: ClusterNode['state'] = 'recommended';

// Edge type constants
const EDGE_TYPE_NEXT: ClusterEdge['type'] = 'NEXT';

/**
 * GraphExplorerComponent - Hierarchical cluster visualization for learning paths.
 *
 * Features:
 * - D3.js force-directed graph with hierarchical clustering
 * - Lazy loading from elohim-protocol learning path structure
 * - Chapter → Module → Section → Concept drill-down
 * - Color-coded cluster states showing completion progress
 * - Double-click to expand clusters, single-click for info panel
 *
 * Route: /lamad/explore
 */
@Component({
  selector: 'app-graph-explorer',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './graph-explorer.component.html',
  styleUrls: ['./graph-explorer.component.css'],
})
export class GraphExplorerComponent implements OnInit, OnDestroy, AfterViewInit {
  @ViewChild('graphContainer', { static: true }) graphContainer!: ElementRef<HTMLDivElement>;

  // View mode
  viewMode: ViewMode = 'path-hierarchy';
  currentPathId = 'elohim-protocol';

  // Current graph state
  graphData: ClusterGraphData | null = null;
  visibleNodes: ClusterNode[] = [];
  visibleEdges: ClusterEdge[] = [];
  breadcrumbs: { id: string; title: string; level: number }[] = [];

  // Cluster state
  loadingClusters = new Set<string>();

  // UI state
  isLoading = true;
  error: string | null = null;
  selectedNode: ClusterNode | null = null;
  hoveredNode: ClusterNode | null = null;

  // Path context for return navigation
  returnContext: { pathId: string; stepIndex: number } | null = null;
  focusNodeId: string | null = null;

  // D3 elements
  private svg!: d3.Selection<SVGSVGElement, unknown, null, undefined>;
  private simulation!: d3.Simulation<ClusterNode, ClusterEdge>;
  private width = 800;
  private height = 600;
  private zoom!: d3.ZoomBehavior<SVGSVGElement, unknown>;

  private readonly destroy$ = new Subject<void>();

  constructor(
    private readonly route: ActivatedRoute,
    private readonly dataLoader: DataLoaderService,
    private readonly router: Router,
    private readonly affinityService: AffinityTrackingService,
    private readonly hierarchicalGraph: HierarchicalGraphService
  ) {}

  ngOnInit(): void {
    // Subscribe to query params for path context
    this.route.queryParams.pipe(takeUntil(this.destroy$)).subscribe(params => {
      if (params['fromPath']) {
        this.returnContext = {
          pathId: params['fromPath'] as string,
          stepIndex: Number.parseInt((params['returnStep'] as string) ?? '0', 10),
        };
      }

      if (params['focus']) {
        this.focusNodeId = params['focus'] as string;
      }

      // Check for view mode override
      if (params['view'] === 'overview') {
        this.viewMode = 'overview';
      }
    });

    // Load hierarchical graph by default
    this.loadPathHierarchy(this.currentPathId);
  }

  ngAfterViewInit(): void {
    this.initializeSvg();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
    if (this.simulation) {
      this.simulation.stop();
    }
  }

  /**
   * Return to the path that launched this exploration.
   */
  returnToPath(): void {
    if (this.returnContext) {
      void this.router.navigate([
        '/lamad/path',
        this.returnContext.pathId,
        'step',
        this.returnContext.stepIndex,
      ]);
    } else {
      void this.router.navigate(['/lamad']);
    }
  }

  /**
   * Switch view mode between path hierarchy and flat overview.
   */
  setViewMode(mode: ViewMode): void {
    if (this.viewMode === mode) return;

    this.viewMode = mode;
    this.selectedNode = null;
    this.hoveredNode = null;

    if (mode === 'path-hierarchy') {
      this.loadPathHierarchy(this.currentPathId);
    } else {
      this.loadOverview();
    }
  }

  /**
   * Initialize the SVG element and zoom behavior.
   */
  private initializeSvg(): void {
    const container = this.graphContainer.nativeElement;
    this.width = container.clientWidth ?? 800;
    this.height = container.clientHeight ?? 600;

    // Clear existing
    d3.select(container).selectAll('*').remove();

    // Create SVG
    this.svg = d3
      .select(container)
      .append('svg')
      .attr('width', '100%')
      .attr('height', '100%')
      .attr('viewBox', `${-this.width / 2} ${-this.height / 2} ${this.width} ${this.height}`);

    // Add zoom behavior
    this.zoom = d3
      .zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.3, 3])
      .on('zoom', (event: d3.D3ZoomEvent<SVGSVGElement, unknown>) => {
        this.svg.select('g.graph-content').attr('transform', event.transform.toString());
      });

    this.svg.call(this.zoom);

    // Click on background to deselect
    this.svg.on('click', () => {
      this.selectedNode = null;
    });

    // Add main group for content
    this.svg.append('g').attr('class', 'graph-content');

    // Add gradient definitions for clusters
    this.addGradientDefs();
  }

  /**
   * Add gradient and marker definitions for cluster visualization.
   */
  private addGradientDefs(): void {
    const defs = this.svg.append('defs');

    // Radial gradient for cluster backgrounds
    const gradient = defs
      .append('radialGradient')
      .attr('id', 'cluster-gradient')
      .attr('cx', '50%')
      .attr('cy', '50%')
      .attr('r', '50%');

    gradient.append('stop').attr('offset', '0%').attr('stop-color', 'rgba(99, 102, 241, 0.3)');

    gradient.append('stop').attr('offset', '100%').attr('stop-color', 'rgba(99, 102, 241, 0.1)');

    // Arrow marker for progression edges
    defs
      .append('marker')
      .attr('id', 'arrow-next')
      .attr('viewBox', '0 -5 10 10')
      .attr('refX', 25) // Offset from end of line to account for node radius
      .attr('refY', 0)
      .attr('markerWidth', 8)
      .attr('markerHeight', 8)
      .attr('orient', 'auto')
      .attr('class', 'arrow-marker')
      .append('path')
      .attr('d', 'M0,-5L10,0L0,5')
      .attr('fill', '#6366f1');
  }

  /**
   * Load hierarchical cluster graph from learning path.
   */
  loadPathHierarchy(pathId: string): void {
    this.isLoading = true;
    this.error = null;
    this.currentPathId = pathId;

    this.hierarchicalGraph
      .initializeFromPath(pathId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: graph => {
          this.graphData = graph;
          this.updateVisibleNodes();
          this.breadcrumbs = [{ id: graph.root.id, title: graph.root.title, level: 0 }];
          this.isLoading = false;

          if (this.svg) {
            this.renderClusterGraph();
          }
        },
        error: _err => {
          this.error = 'Failed to load learning path graph';
          this.isLoading = false;
        },
      });
  }

  /**
   * Load flat overview graph showing all chapters as a flat view.
   * Uses the same learning path data but without hierarchical nesting.
   */
  loadOverview(): void {
    this.isLoading = true;
    this.error = null;

    // Use the same learning path but show chapters as flat nodes
    this.dataLoader
      .getPathHierarchy(this.currentPathId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: path => {
          if (!path?.chapters || path.chapters.length === 0) {
            this.error = 'No content available for overview';
            this.isLoading = false;
            return;
          }

          // Convert chapters to flat cluster nodes (no hierarchy, just nodes)
          this.visibleNodes = path.chapters.map((chapter, index) => {
            let totalConcepts = 0;
            if (chapter.modules) {
              for (const module of chapter.modules) {
                for (const section of module.sections || []) {
                  totalConcepts += section.conceptIds?.length || 0;
                }
              }
            }

            return {
              id: chapter.id,
              title: chapter.title,
              description: chapter.description,
              contentType: 'chapter',
              isCluster: true,
              clusterType: 'chapter' as const,
              clusterLevel: 1,
              parentClusterId: null, // No parent in flat view
              childClusterIds: [],
              conceptIds: [],
              isExpanded: false,
              isLoading: false,
              totalConceptCount: totalConcepts,
              completedConceptCount: 0,
              externalConnectionCount: 0,
              state: 'unseen' as const,
              affinityScore: 0,
              order: chapter.order ?? index,
            };
          });

          // No progression edges in flat overview - just shows all nodes
          this.visibleEdges = [];
          this.breadcrumbs = [{ id: 'root', title: 'Protocol Overview', level: 0 }];
          this.isLoading = false;

          if (this.svg) {
            this.renderClusterGraph();
          }
        },
        error: () => {
          this.error = 'Failed to load graph overview';
          this.isLoading = false;
        },
      });
  }

  /**
   * Convert a ContentNode to ClusterNode format.
   */
  private convertToClusterNode(node: Partial<ClusterNode>): ClusterNode {
    const affinityScore = this.affinityService.getAffinity(node.id ?? '');

    let state: ClusterNode['state'] = STATE_UNSEEN;
    if (affinityScore > 0.66) {
      state = STATE_PROFICIENT;
    } else if (affinityScore > 0.33) {
      state = STATE_IN_PROGRESS;
    }

    if (node.id === 'manifesto' && affinityScore === 0) {
      state = STATE_RECOMMENDED;
    }

    return {
      id: node.id ?? '',
      title: node.title ?? node.id ?? '',
      description: node.description ?? '',
      contentType: node.contentType ?? 'concept',
      isCluster: false,
      clusterType: null,
      clusterLevel: 4,
      parentClusterId: null,
      childClusterIds: [],
      conceptIds: [],
      isExpanded: false,
      isLoading: false,
      totalConceptCount: 0,
      completedConceptCount: 0,
      externalConnectionCount: 0,
      state,
      affinityScore,
      x: node.x ?? 0,
      y: node.y ?? 0,
    };
  }

  /**
   * Update visible nodes based on expansion state.
   */
  private updateVisibleNodes(): void {
    if (!this.graphData) return;
    this.visibleNodes = this.hierarchicalGraph.getVisibleNodes(this.graphData);
    const visibleIds = new Set(this.visibleNodes.map(n => n.id));
    this.visibleEdges = this.hierarchicalGraph.getVisibleEdges(visibleIds);
  }

  /**
   * Expand a cluster to show its children.
   */
  expandCluster(clusterId: string): void {
    if (this.loadingClusters.has(clusterId)) return;

    this.loadingClusters.add(clusterId);

    this.hierarchicalGraph
      .expandCluster(clusterId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: _result => {
          this.loadingClusters.delete(clusterId);

          // Update visible nodes
          this.updateVisibleNodes();

          // Update breadcrumbs
          const cluster = this.graphData?.clusters.get(clusterId);
          if (cluster) {
            this.updateBreadcrumbs(cluster);
          }

          // Re-render
          this.renderClusterGraph();
        },
        error: _err => {
          this.loadingClusters.delete(clusterId);
        },
      });
  }

  /**
   * Collapse a cluster to hide its children.
   */
  collapseCluster(clusterId: string): void {
    this.hierarchicalGraph.collapseCluster(clusterId);
    this.updateVisibleNodes();
    this.renderClusterGraph();
  }

  /**
   * Update breadcrumbs based on selected cluster.
   */
  private updateBreadcrumbs(cluster: ClusterNode): void {
    const crumbs: { id: string; title: string; level: number }[] = [];

    // Build path from root to current
    let current: ClusterNode | undefined = cluster;
    while (current) {
      crumbs.unshift({
        id: current.id,
        title: current.title,
        level: current.clusterLevel,
      });
      current = current.parentClusterId
        ? this.graphData?.clusters.get(current.parentClusterId)
        : undefined;
    }

    this.breadcrumbs = crumbs;
  }

  /**
   * Render the cluster graph with D3.
   */
  private renderClusterGraph(): void {
    if (!this.svg) return;

    const g = this.svg.select<SVGGElement>('g.graph-content');
    g.selectAll('*').remove();

    const nodes = this.visibleNodes;
    const edges = this.visibleEdges;

    // Create simulation with cluster-aware forces
    this.simulation = d3
      .forceSimulation<ClusterNode>(nodes)
      .force(
        'link',
        d3
          .forceLink<ClusterNode, ClusterEdge>(edges)
          .id((d: ClusterNode) => d.id)
          .distance(d => this.getLinkDistance(d))
      )
      .force(
        'charge',
        d3.forceManyBody().strength(d => this.getChargeStrength(d as ClusterNode))
      )
      .force('center', d3.forceCenter(0, 0))
      .force(
        'collision',
        d3.forceCollide<ClusterNode>().radius(d => this.getCollisionRadius(d))
      );

    // Draw edges first (under nodes)
    // D3 uses standard SVG attribute names - disable string duplication for D3 fluent API
    /* eslint-disable sonarjs/no-duplicate-string */
    const link = g
      .append('g')
      .attr('class', 'links')
      .selectAll('line')
      .data(edges)
      .enter()
      .append('line')
      .attr('class', d => `link link-${d.type.toLowerCase()}`)
      .attr('stroke', d => this.getEdgeColor(d))
      .attr('stroke-opacity', d => this.getEdgeOpacity(d))
      .attr('stroke-width', d => this.getEdgeStrokeWidth(d))
      .attr('stroke-dasharray', d => (d.isAggregated ? '5,5' : 'none'))
      .attr('marker-end', d => (d.type === EDGE_TYPE_NEXT ? 'url(#arrow-next)' : null));

    // Draw nodes
    const node = g
      .append('g')
      .attr('class', 'nodes')
      .selectAll('g.node')
      .data(nodes)
      .enter()
      .append('g')
      .attr(
        'class',
        d =>
          `node node-${d.contentType} node-state-${d.state} ${d.isCluster ? 'cluster' : 'concept'}`
      )
      .style('cursor', 'pointer')
      .call(this.drag());

    // Cluster circles (with fill)
    node
      .filter(d => d.isCluster)
      .append('circle')
      .attr('r', d => this.getNodeRadius(d))
      .attr('fill', d => this.getClusterFill(d))
      .attr('stroke', d => this.getClusterStroke(d))
      .attr('stroke-width', 2)
      .attr('stroke-dasharray', d => (d.isExpanded ? '5,3' : 'none'))
      .attr('opacity', d => (d.isExpanded ? 0.5 : 0.8));

    // Concept circles (solid fill)
    node
      .filter(d => !d.isCluster)
      .append('circle')
      .attr('r', d => this.getNodeRadius(d))
      .attr('fill', d => this.getNodeColor(d))
      .attr('stroke', d => this.getNodeStroke(d))
      .attr('stroke-width', 3);

    // Progress arc for clusters
    node
      .filter(d => d.isCluster && d.totalConceptCount > 0)
      .append('path')
      .attr('class', 'progress-arc')
      .attr('d', d => this.createProgressArc(d))
      .attr('fill', 'none')
      .attr('stroke', '#22c55e')
      .attr('stroke-width', 4)
      .attr('opacity', 0.8);

    // Child count badge for clusters
    node
      .filter(d => d.isCluster && d.totalConceptCount > 0 && !d.isExpanded)
      .append('circle')
      .attr('r', 14)
      .attr('cx', d => this.getNodeRadius(d) - 8)
      .attr('cy', d => -this.getNodeRadius(d) + 8)
      .attr('fill', '#3b82f6')
      .attr('stroke', '#fff')
      .attr('stroke-width', 2);

    node
      .filter(d => d.isCluster && d.totalConceptCount > 0 && !d.isExpanded)
      .append('text')
      .attr('x', d => this.getNodeRadius(d) - 8)
      .attr('y', d => -this.getNodeRadius(d) + 13)
      .attr('text-anchor', 'middle')
      .attr('fill', '#fff')
      .attr('font-size', '11px')
      .attr('font-weight', 'bold')
      .text(d => d.totalConceptCount);

    // Expand/collapse indicator for clusters
    node
      .filter(d => d.isCluster && d.childClusterIds.length > 0)
      .append('text')
      .attr('class', 'expand-indicator')
      .attr('x', 0)
      .attr('y', 5)
      .attr('text-anchor', 'middle')
      .attr('fill', '#e0e6ed')
      .attr('font-size', '18px')
      .attr('font-weight', 'bold')
      .text(d => (this.hierarchicalGraph.isExpanded(d.id) ? '−' : '+'));

    // Node labels
    node
      .append('text')
      .attr('dy', d => this.getNodeRadius(d) + 18)
      .attr('text-anchor', 'middle')
      .attr('fill', '#e0e6ed')
      .attr('font-size', d => (d.isCluster ? '13px' : '11px'))
      .attr('font-weight', d => (d.isCluster ? '600' : '500'))
      .text(d => this.truncateTitle(d.title, d.isCluster ? 25 : 18));

    // Cluster type label (above title)
    node
      .filter(d => d.isCluster)
      .append('text')
      .attr('dy', d => -this.getNodeRadius(d) - 8)
      .attr('text-anchor', 'middle')
      .attr('fill', '#94a3b8')
      .attr('font-size', '10px')
      .attr('text-transform', 'uppercase')
      .attr('letter-spacing', '0.05em')
      .text(d => CLUSTER_LEVEL_CONFIG[d.clusterLevel]?.name || '');

    // Event handlers
    node.on('click', (event: MouseEvent, d: ClusterNode) => {
      event.stopPropagation();
      this.handleNodeClick(d);
    });

    node.on('dblclick', (event: MouseEvent, d: ClusterNode) => {
      event.stopPropagation();
      this.handleNodeDoubleClick(d);
    });

    node.on('mouseenter', (_event: MouseEvent, d: ClusterNode) => {
      this.hoveredNode = d;
    });

    node.on('mouseleave', () => {
      this.hoveredNode = null;
    });

    // Update positions on tick
    this.simulation.on('tick', () => {
      link
        .attr('x1', (d: d3.SimulationLinkDatum<ClusterNode>) => (d.source as ClusterNode).x ?? 0)
        .attr('y1', (d: d3.SimulationLinkDatum<ClusterNode>) => (d.source as ClusterNode).y ?? 0)
        .attr('x2', (d: d3.SimulationLinkDatum<ClusterNode>) => (d.target as ClusterNode).x ?? 0)
        .attr('y2', (d: d3.SimulationLinkDatum<ClusterNode>) => (d.target as ClusterNode).y ?? 0);

      node.attr('transform', d => `translate(${d.x},${d.y})`);
    });
    /* eslint-enable sonarjs/no-duplicate-string */
  }

  /**
   * Get node radius based on cluster level and type.
   */
  private getNodeRadius(node: ClusterNode): number {
    if (node.clusterRadius) return node.clusterRadius;
    return calculateClusterRadius(node);
  }

  /**
   * Get link distance based on cluster levels.
   */
  private getLinkDistance(link: d3.SimulationLinkDatum<ClusterNode>): number {
    const source = link.source as ClusterNode;
    const target = link.target as ClusterNode;

    if (!source || !target) return 150;

    const sourceRadius = this.getNodeRadius(source);
    const targetRadius = this.getNodeRadius(target);

    return sourceRadius + targetRadius + 80;
  }

  /**
   * Get charge strength based on cluster level.
   */
  private getChargeStrength(node: ClusterNode): number {
    const baseStrength = -400;
    const levelMultiplier = Math.pow(0.7, node.clusterLevel);
    return baseStrength * levelMultiplier;
  }

  /**
   * Get collision radius for a node.
   */
  private getCollisionRadius(node: ClusterNode): number {
    return this.getNodeRadius(node) + 10;
  }

  /**
   * Get cluster fill color based on level and state.
   */
  private getClusterFill(node: ClusterNode): string {
    const config = CLUSTER_LEVEL_CONFIG[node.clusterLevel];
    return config?.color || 'rgba(99, 102, 241, 0.2)';
  }

  /**
   * Get cluster stroke color based on level.
   */
  private getClusterStroke(node: ClusterNode): string {
    const config = CLUSTER_LEVEL_CONFIG[node.clusterLevel];
    return config?.strokeColor || '#6366f1';
  }

  /**
   * Get node fill color based on state (for concepts).
   */
  private getNodeColor(node: ClusterNode): string {
    switch (node.state) {
      case STATE_PROFICIENT:
        return '#fbbf24';
      case STATE_IN_PROGRESS:
        return '#facc15';
      case STATE_RECOMMENDED:
        return '#22c55e';
      case 'review':
        return '#f97316';
      case 'locked':
        return '#475569';
      default:
        return '#64748b';
    }
  }

  /**
   * Get node stroke color based on state.
   */
  private getNodeStroke(node: ClusterNode): string {
    switch (node.state) {
      case STATE_PROFICIENT:
        return '#3b82f6';
      case STATE_IN_PROGRESS:
        return '#3b82f6';
      case STATE_RECOMMENDED:
        return '#22c55e';
      case 'review':
        return '#f97316';
      case 'locked':
        return '#64748b';
      default:
        return '#94a3b8';
    }
  }

  /**
   * Get edge color based on type.
   */
  private getEdgeColor(edge: ClusterEdge): string {
    switch (edge.type) {
      case EDGE_TYPE_NEXT:
        return '#6366f1'; // Indigo for progression
      case 'CONTAINS':
        return '#22c55e';
      case 'PREREQ':
        return '#3b82f6';
      case 'RELATED':
        return '#8b5cf6';
      default:
        return edge.isAggregated ? '#6366f1' : '#475569';
    }
  }

  /**
   * Get edge opacity based on type and aggregation.
   */
  private getEdgeOpacity(edge: ClusterEdge): number {
    if (edge.type === EDGE_TYPE_NEXT) return 0.8;
    if (edge.isAggregated) return 0.3;
    return 0.6;
  }

  /**
   * Get edge stroke width based on type and aggregation.
   */
  private getEdgeStrokeWidth(edge: ClusterEdge): number {
    if (edge.type === EDGE_TYPE_NEXT) return 3;
    if (edge.isAggregated) return Math.min(edge.connectionCount ?? 1, 8);
    return 2;
  }

  /**
   * Create progress arc SVG path for cluster completion.
   */
  private createProgressArc(node: ClusterNode): string {
    if (node.totalConceptCount === 0) return '';

    const progress = node.completedConceptCount / node.totalConceptCount;
    if (progress === 0) return '';

    const radius = this.getNodeRadius(node) + 6;
    const startAngle = -Math.PI / 2;
    const endAngle = startAngle + progress * 2 * Math.PI;

    const x1 = Math.cos(startAngle) * radius;
    const y1 = Math.sin(startAngle) * radius;
    const x2 = Math.cos(endAngle) * radius;
    const y2 = Math.sin(endAngle) * radius;

    const largeArc = progress > 0.5 ? 1 : 0;

    return `M ${x1} ${y1} A ${radius} ${radius} 0 ${largeArc} 1 ${x2} ${y2}`;
  }

  /**
   * Handle single click on node (select/info).
   */
  handleNodeClick(node: ClusterNode): void {
    this.selectedNode = node;
  }

  /**
   * Handle double-click on node (expand/navigate).
   */
  handleNodeDoubleClick(node: ClusterNode): void {
    if (node.state === 'locked') return;

    if (node.isCluster) {
      // Toggle cluster expansion
      if (this.hierarchicalGraph.isExpanded(node.id)) {
        this.collapseCluster(node.id);
      } else {
        this.expandCluster(node.id);
      }
    } else {
      // Navigate to content
      this.navigateToContent(node.id);
    }
  }

  /**
   * Navigate to content viewer.
   */
  navigateToContent(nodeId: string): void {
    void this.router.navigate(['/lamad/resource', nodeId]);
  }

  /**
   * Navigate to breadcrumb level.
   */
  navigateToBreadcrumb(crumb: { id: string; title: string; level: number }): void {
    if (crumb.level === 0) {
      // Reset to root level
      this.hierarchicalGraph.reset();
      this.loadPathHierarchy(this.currentPathId);
    } else {
      // Collapse everything below this level
      const currentCrumb = this.breadcrumbs.find(b => b.id === crumb.id);
      if (currentCrumb) {
        // Collapse all clusters at or below this level
        for (const node of this.visibleNodes) {
          if (node.clusterLevel >= crumb.level && this.hierarchicalGraph.isExpanded(node.id)) {
            this.hierarchicalGraph.collapseCluster(node.id);
          }
        }
        this.updateVisibleNodes();
        this.breadcrumbs = this.breadcrumbs.slice(0, this.breadcrumbs.indexOf(currentCrumb) + 1);
        this.renderClusterGraph();
      }
    }
  }

  /**
   * Truncate title for display.
   */
  private truncateTitle(title: string, maxLength: number): string {
    if (title.length <= maxLength) return title;
    return title.substring(0, maxLength - 3) + '...';
  }

  /**
   * Create drag behavior.
   */
  private drag(): d3.DragBehavior<SVGGElement, ClusterNode, ClusterNode | d3.SubjectPosition> {
    return d3
      .drag<SVGGElement, ClusterNode>()
      .on(
        'start',
        (
          event: d3.D3DragEvent<SVGGElement, ClusterNode, ClusterNode | d3.SubjectPosition>,
          d: ClusterNode
        ) => {
          if (!event.active) this.simulation.alphaTarget(0.3).restart();
          d.fx = d.x;
          d.fy = d.y;
        }
      )
      .on(
        'drag',
        (
          event: d3.D3DragEvent<SVGGElement, ClusterNode, ClusterNode | d3.SubjectPosition>,
          d: ClusterNode
        ) => {
          d.fx = event.x;
          d.fy = event.y;
        }
      )
      .on(
        'end',
        (
          event: d3.D3DragEvent<SVGGElement, ClusterNode, ClusterNode | d3.SubjectPosition>,
          d: ClusterNode
        ) => {
          if (!event.active) this.simulation.alphaTarget(0);
          d.fx = null;
          d.fy = null;
        }
      );
  }

  /**
   * Reset zoom to fit content.
   */
  resetZoom(): void {
    if (this.svg && this.zoom) {
      this.svg.transition().duration(500).call(this.zoom.transform, d3.zoomIdentity);
    }
  }

  /**
   * Get state label for display.
   */
  getStateLabel(state: string): string {
    const labels: Record<string, string> = {
      [STATE_UNSEEN]: 'Not Started',
      [STATE_IN_PROGRESS]: 'In Progress',
      [STATE_PROFICIENT]: 'Completed',
      [STATE_RECOMMENDED]: 'Recommended',
      review: 'Needs Review',
      locked: 'Locked',
    };
    return labels[state] ?? state;
  }

  /**
   * Get cluster type label for display.
   */
  getClusterTypeLabel(clusterType: string | null): string {
    if (!clusterType) return 'Concept';
    const labels: Record<string, string> = {
      path: 'Learning Path',
      chapter: 'Chapter',
      module: 'Module',
      section: 'Section',
    };
    return labels[clusterType] ?? clusterType;
  }

  /**
   * Check if a cluster can be expanded.
   */
  canExpand(node: ClusterNode): boolean {
    return (
      node.isCluster &&
      (node.childClusterIds.length > 0 || node.conceptIds.length > 0) &&
      !this.hierarchicalGraph.isExpanded(node.id)
    );
  }

  /**
   * Check if a cluster is currently expanded.
   */
  isExpanded(node: ClusterNode): boolean {
    return this.hierarchicalGraph.isExpanded(node.id);
  }

  /**
   * Check if a cluster is currently loading.
   */
  isClusterLoading(node: ClusterNode): boolean {
    return this.loadingClusters.has(node.id);
  }
}
