import { Component, OnInit, OnDestroy, ElementRef, ViewChild, AfterViewInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterModule } from '@angular/router';
import { HttpClient } from '@angular/common/http';
import { Subject } from 'rxjs';
import { takeUntil } from 'rxjs/operators';
import * as d3 from 'd3';
import { AffinityTrackingService } from '../../services/affinity-tracking.service';

/**
 * Graph node state for visualization
 */
interface GraphNode extends d3.SimulationNodeDatum {
  id: string;
  title: string;
  contentType: string;
  description: string;
  hasChildren: boolean;
  childCount: number;
  level: number;
  isRoot?: boolean;
  isParent?: boolean;
  // D3 simulation properties
  x?: number;
  y?: number;
  fx?: number | null;
  fy?: number | null;
  // Visual state
  state: 'unseen' | 'in-progress' | 'proficient' | 'recommended' | 'review' | 'locked';
  affinityScore: number;
  expanded: boolean;
}

/**
 * Graph edge for visualization
 */
interface GraphEdge extends d3.SimulationLinkDatum<GraphNode> {
  source: string | GraphNode;
  target: string | GraphNode;
  type: string;
}

/**
 * Graph level data structure
 */
interface GraphLevel {
  nodes: GraphNode[];
  edges: GraphEdge[];
  zoomLevel: number;
  parentId?: string;
}

/**
 * GraphExplorerComponent - Khan Academy "World of Math" inspired knowledge graph.
 *
 * Features:
 * - D3.js force-directed graph with semantic zoom
 * - Hierarchical lazy loading (Epic → Feature → Scenario)
 * - Color-coded node states (unseen, proficient, locked)
 * - Click to expand/navigate
 *
 * Route: /lamad/explore
 */
@Component({
  selector: 'app-graph-explorer',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './graph-explorer.component.html',
  styleUrls: ['./graph-explorer.component.css']
})
export class GraphExplorerComponent implements OnInit, OnDestroy, AfterViewInit {
  @ViewChild('graphContainer', { static: true }) graphContainer!: ElementRef<HTMLDivElement>;

  // Current graph state
  currentLevel: GraphLevel | null = null;
  breadcrumbs: { id: string; title: string; level: number }[] = [];

  // UI state
  isLoading = true;
  error: string | null = null;
  selectedNode: GraphNode | null = null;
  hoveredNode: GraphNode | null = null;

  // D3 elements
  private svg!: d3.Selection<SVGSVGElement, unknown, null, undefined>;
  private simulation!: d3.Simulation<GraphNode, GraphEdge>;
  private width = 800;
  private height = 600;
  private zoom!: d3.ZoomBehavior<SVGSVGElement, unknown>;

  private readonly destroy$ = new Subject<void>();

  constructor(
    private readonly http: HttpClient,
    private readonly router: Router,
    private readonly affinityService: AffinityTrackingService
  ) {}

  ngOnInit(): void {
    this.loadOverview();
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
   * Initialize the SVG element and zoom behavior
   */
  private initializeSvg(): void {
    const container = this.graphContainer.nativeElement;
    this.width = container.clientWidth ?? 800;
    this.height = container.clientHeight ?? 600;

    // Clear existing
    d3.select(container).selectAll('*').remove();

    // Create SVG
    this.svg = d3.select(container)
      .append('svg')
      .attr('width', '100%')
      .attr('height', '100%')
      .attr('viewBox', `${-this.width / 2} ${-this.height / 2} ${this.width} ${this.height}`);

    // Add zoom behavior
    this.zoom = d3.zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.5, 3])
      .on('zoom', (event) => {
        this.svg.select('g.graph-content').attr('transform', event.transform);
      });

    this.svg.call(this.zoom);

    // Add main group for content
    this.svg.append('g').attr('class', 'graph-content');

    // Add arrow marker for edges
    this.svg.append('defs').append('marker')
      .attr('id', 'arrowhead')
      .attr('viewBox', '-0 -5 10 10')
      .attr('refX', 20)
      .attr('refY', 0)
      .attr('orient', 'auto')
      .attr('markerWidth', 6)
      .attr('markerHeight', 6)
      .append('path')
      .attr('d', 'M 0,-5 L 10,0 L 0,5')
      .attr('fill', '#94a3b8');
  }

  /**
   * Load the overview level (zoom level 0)
   */
  loadOverview(): void {
    this.isLoading = true;
    this.error = null;

    this.http.get<any>('/assets/lamad-data/graph/overview.json')
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: (data) => {
          const nodes = data.nodes.map((n: any) => this.enrichNode(n));
          const edges = data.edges ?? [];

          this.currentLevel = {
            nodes,
            edges,
            zoomLevel: 0
          };

          this.breadcrumbs = [{ id: 'root', title: 'Protocol Overview', level: 0 }];
          this.isLoading = false;

          if (this.svg) {
            this.renderGraph();
          }
        },
        error: (err) => {
          this.error = 'Failed to load graph overview';
          this.isLoading = false;
          console.error('[GraphExplorer] Failed to load overview:', err);
        }
      });
  }

  /**
   * Load epic detail level (zoom level 1)
   */
  loadEpicDetail(epicId: string, epicTitle: string): void {
    // Map node ID to epic key
    const epicKey = epicId.replace('-epic', '').replace(/-/g, '_');

    this.isLoading = true;
    this.http.get<any>(`/assets/lamad-data/graph/epic-${epicKey}.json`)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: (data) => {
          const nodes = data.nodes.map((n: any) => this.enrichNode(n));
          const edges = data.edges ?? [];

          this.currentLevel = {
            nodes,
            edges,
            zoomLevel: 1,
            parentId: epicId
          };

          this.breadcrumbs = [
            { id: 'root', title: 'Protocol Overview', level: 0 },
            { id: epicId, title: epicTitle, level: 1 }
          ];

          this.isLoading = false;
          this.renderGraph();
        },
        error: (err) => {
          this.error = `Failed to load ${epicTitle} details`;
          this.isLoading = false;
          console.error('[GraphExplorer] Failed to load epic detail:', err);
        }
      });
  }

  /**
   * Enrich a node with affinity and state data
   */
  private enrichNode(node: any): GraphNode {
    const affinityScore = this.affinityService.getAffinity(node.id);

    let state: GraphNode['state'] = 'unseen';
    if (affinityScore > 0.66) {
      state = 'proficient';
    } else if (affinityScore > 0.33) {
      state = 'in-progress';
    } else if (affinityScore > 0) {
      state = 'unseen';
    }

    // Mark recommended nodes (manifesto is always recommended for new users)
    if (node.id === 'manifesto' && affinityScore === 0) {
      state = 'recommended';
    }

    return {
      ...node,
      state,
      affinityScore,
      expanded: false,
      x: node.position?.x || 0,
      y: node.position?.y || 0
    };
  }

  /**
   * Render the current graph level
   */
  private renderGraph(): void {
    if (!this.currentLevel || !this.svg) return;

    const g = this.svg.select<SVGGElement>('g.graph-content');
    g.selectAll('*').remove();

    const nodes = this.currentLevel.nodes;
    const edges = this.currentLevel.edges;

    // Create simulation
    this.simulation = d3.forceSimulation<GraphNode>(nodes)
      .force('link', d3.forceLink<GraphNode, GraphEdge>(edges as any)
        .id((d: GraphNode) => d.id)
        .distance(120))
      .force('charge', d3.forceManyBody().strength(-300))
      .force('center', d3.forceCenter(0, 0))
      .force('collision', d3.forceCollide().radius(60));

    // Draw edges
    const link = g.append('g')
      .attr('class', 'links')
      .selectAll('line')
      .data(edges)
      .enter()
      .append('line')
      .attr('class', d => `link link-${(d as any).type?.toLowerCase() ?? 'default'}`)
      .attr('stroke', '#475569')
      .attr('stroke-opacity', 0.6)
      .attr('stroke-width', 2)
      .attr('marker-end', 'url(#arrowhead)');

    // Draw nodes
    const node = g.append('g')
      .attr('class', 'nodes')
      .selectAll('g.node')
      .data(nodes)
      .enter()
      .append('g')
      .attr('class', d => `node node-${d.contentType} node-state-${d.state}`)
      .style('cursor', 'pointer')
      .call(this.drag());

    // Node circles
    node.append('circle')
      .attr('r', d => this.getNodeRadius(d))
      .attr('fill', d => this.getNodeColor(d))
      .attr('stroke', d => this.getNodeStroke(d))
      .attr('stroke-width', d => d.state === 'locked' ? 2 : 3)
      .attr('stroke-dasharray', d => d.state === 'locked' ? '4,4' : 'none');

    // Child count badge
    node.filter(d => d.hasChildren && d.childCount > 0)
      .append('circle')
      .attr('r', 12)
      .attr('cx', d => this.getNodeRadius(d) - 5)
      .attr('cy', d => -this.getNodeRadius(d) + 5)
      .attr('fill', '#3b82f6')
      .attr('stroke', '#fff')
      .attr('stroke-width', 2);

    node.filter(d => d.hasChildren && d.childCount > 0)
      .append('text')
      .attr('x', d => this.getNodeRadius(d) - 5)
      .attr('y', d => -this.getNodeRadius(d) + 9)
      .attr('text-anchor', 'middle')
      .attr('fill', '#fff')
      .attr('font-size', '10px')
      .attr('font-weight', 'bold')
      .text(d => d.childCount);

    // Node labels
    node.append('text')
      .attr('dy', d => this.getNodeRadius(d) + 20)
      .attr('text-anchor', 'middle')
      .attr('fill', '#e0e6ed')
      .attr('font-size', '12px')
      .attr('font-weight', '500')
      .text(d => this.truncateTitle(d.title, 20));

    // Event handlers
    node.on('click', (event: MouseEvent, d: GraphNode) => {
      event.stopPropagation();
      this.handleNodeClick(d);
    });

    node.on('mouseenter', (event: MouseEvent, d: GraphNode) => {
      this.hoveredNode = d;
    });

    node.on('mouseleave', () => {
      this.hoveredNode = null;
    });

    // Update positions on tick
    this.simulation.on('tick', () => {
      link
        .attr('x1', (d: any) => d.source.x)
        .attr('y1', (d: any) => d.source.y)
        .attr('x2', (d: any) => d.target.x)
        .attr('y2', (d: any) => d.target.y);

      node.attr('transform', d => `translate(${d.x},${d.y})`);
    });
  }

  /**
   * Get node radius based on type and level
   */
  private getNodeRadius(node: GraphNode): number {
    if (node.isRoot) return 50;
    if (node.contentType === 'epic') return 40;
    if (node.contentType === 'feature') return 30;
    return 20;
  }

  /**
   * Get node fill color based on state
   */
  private getNodeColor(node: GraphNode): string {
    switch (node.state) {
      case 'proficient': return '#fbbf24';  // Gold
      case 'in-progress': return '#facc15'; // Yellow
      case 'recommended': return '#22c55e'; // Green
      case 'review': return '#f97316';      // Orange
      case 'locked': return '#475569';      // Dim gray
      default: return '#64748b';            // Gray (unseen)
    }
  }

  /**
   * Get node stroke color based on state
   */
  private getNodeStroke(node: GraphNode): string {
    switch (node.state) {
      case 'proficient': return '#3b82f6';  // Blue
      case 'in-progress': return '#3b82f6'; // Blue
      case 'recommended': return '#22c55e'; // Green
      case 'review': return '#f97316';      // Orange
      case 'locked': return '#64748b';      // Gray
      default: return '#94a3b8';            // Light gray
    }
  }

  /**
   * Handle node click
   */
  handleNodeClick(node: GraphNode): void {
    this.selectedNode = node;

    if (node.state === 'locked') {
      // Show locked message (handled in template)
      return;
    }

    // If node has children, expand to next level
    if (node.hasChildren && node.contentType === 'epic') {
      this.loadEpicDetail(node.id, node.title);
    } else {
      // Navigate to content
      this.navigateToContent(node.id);
    }
  }

  /**
   * Navigate to content viewer
   */
  navigateToContent(nodeId: string): void {
    this.router.navigate(['/lamad/resource', nodeId]);
  }

  /**
   * Navigate to breadcrumb level
   */
  navigateToBreadcrumb(crumb: { id: string; title: string; level: number }): void {
    if (crumb.level === 0) {
      this.loadOverview();
    }
  }

  /**
   * Truncate title for display
   */
  private truncateTitle(title: string, maxLength: number): string {
    if (title.length <= maxLength) return title;
    return title.substring(0, maxLength - 3) + '...';
  }

  /**
   * Create drag behavior
   */
  private drag(): d3.DragBehavior<SVGGElement, GraphNode, GraphNode | d3.SubjectPosition> {
    return d3.drag<SVGGElement, GraphNode>()
      .on('start', (event: d3.D3DragEvent<SVGGElement, GraphNode, GraphNode | d3.SubjectPosition>, d: GraphNode) => {
        if (!event.active) this.simulation.alphaTarget(0.3).restart();
        d.fx = d.x;
        d.fy = d.y;
      })
      .on('drag', (event: d3.D3DragEvent<SVGGElement, GraphNode, GraphNode | d3.SubjectPosition>, d: GraphNode) => {
        d.fx = event.x;
        d.fy = event.y;
      })
      .on('end', (event: d3.D3DragEvent<SVGGElement, GraphNode, GraphNode | d3.SubjectPosition>, d: GraphNode) => {
        if (!event.active) this.simulation.alphaTarget(0);
        d.fx = null;
        d.fy = null;
      });
  }

  /**
   * Reset zoom to fit content
   */
  resetZoom(): void {
    if (this.svg && this.zoom) {
      this.svg.transition()
        .duration(500)
        .call(this.zoom.transform, d3.zoomIdentity);
    }
  }

  /**
   * Get state label for display
   */
  getStateLabel(state: string): string {
    const labels: Record<string, string> = {
      'unseen': 'Not Started',
      'in-progress': 'In Progress',
      'proficient': 'Completed',
      'recommended': 'Recommended',
      'review': 'Needs Review',
      'locked': 'Locked'
    };
    return labels[state] ?? state;
  }
}
