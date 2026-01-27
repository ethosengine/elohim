import { CommonModule } from '@angular/common';
import {
  Component,
  Input,
  Output,
  EventEmitter,
  OnChanges,
  OnDestroy,
  SimpleChanges,
  ElementRef,
  ViewChild,
  AfterViewInit,
  ChangeDetectionStrategy,
  ChangeDetectorRef,
} from '@angular/core';

import { takeUntil } from 'rxjs/operators';

import * as d3 from 'd3';
import { Subject } from 'rxjs';

import {
  MiniGraphData,
  MiniGraphNode,
  RelationshipType,
} from '../../models/exploration-context.model';
import { RelatedConceptsService } from '../../services/related-concepts.service';

/**
 * D3 node interface extending MiniGraphNode with simulation properties.
 */
interface D3Node extends MiniGraphNode, d3.SimulationNodeDatum {
  x?: number;
  y?: number;
  fx?: number | null;
  fy?: number | null;
}

/**
 * D3 link interface for simulation.
 */
interface D3Link extends d3.SimulationLinkDatum<D3Node> {
  source: string | D3Node;
  target: string | D3Node;
  relationshipType: RelationshipType;
}

/**
 * MiniGraphComponent - Compact D3.js neighborhood visualization.
 *
 * Shows immediate neighborhood of a concept in a small, interactive graph.
 * Optimized for embedding in sidebars and panels.
 *
 * Features:
 * - Simplified force-directed layout
 * - Focus node highlighted in center
 * - Relationship type indicated by edge style
 * - Click to navigate, double-click to explore in full graph
 *
 * Usage:
 * ```html
 * <app-mini-graph
 *   [focusNodeId]="conceptId"
 *   [depth]="1"
 *   [height]="200"
 *   (nodeSelected)="onConceptClick($event)"
 *   (exploreRequested)="openFullGraph()">
 * </app-mini-graph>
 * ```
 */
@Component({
  selector: 'app-mini-graph',
  standalone: true,
  imports: [CommonModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="mini-graph-container" [style.height.px]="height">
      @if (isLoading) {
        <div class="loading-overlay">
          <span class="loading-spinner"></span>
        </div>
      }

      @if (isEmpty && !isLoading) {
        <div class="empty-state">
          <span>No connections</span>
        </div>
      }

      <div #graphContainer class="graph-viewport"></div>

      @if (hoveredNode && !hoveredNode.isFocus) {
        <div class="node-tooltip" [style.left.px]="tooltipX" [style.top.px]="tooltipY">
          <span class="tooltip-title">{{ hoveredNode.title }}</span>
          <span class="tooltip-type">{{ hoveredNode.contentType }}</span>
        </div>
      }

      <button
        class="expand-button"
        (click)="onExpandClick()"
        title="Explore in full graph"
        aria-label="Open full graph explorer"
      >
        ⤢
      </button>
    </div>
  `,
  styles: [
    `
      .mini-graph-container {
        position: relative;
        width: 100%;
        background: var(--surface-secondary, #f8f9fa);
        border-radius: var(--radius-md, 8px);
        overflow: hidden;
      }

      .graph-viewport {
        width: 100%;
        height: 100%;
      }

      .loading-overlay,
      .empty-state {
        position: absolute;
        inset: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        background: var(--surface-secondary, #f8f9fa);
      }

      .loading-spinner {
        width: 1.5rem;
        height: 1.5rem;
        border: 2px solid var(--border-color, #e9ecef);
        border-top-color: var(--primary, #4285f4);
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
      }

      @keyframes spin {
        to {
          transform: rotate(360deg);
        }
      }

      .empty-state {
        color: var(--text-tertiary, #80868b);
        font-size: 0.875rem;
      }

      .node-tooltip {
        position: absolute;
        padding: 0.375rem 0.625rem;
        background: var(--surface-elevated, #fff);
        border: 1px solid var(--border-color, #e9ecef);
        border-radius: var(--radius-sm, 4px);
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
        pointer-events: none;
        z-index: 10;
        display: flex;
        flex-direction: column;
        gap: 0.125rem;
        transform: translate(-50%, -100%);
        margin-top: -8px;
      }

      .tooltip-title {
        font-weight: 500;
        font-size: 0.8125rem;
        color: var(--text-primary, #202124);
        white-space: nowrap;
        max-width: 150px;
        overflow: hidden;
        text-overflow: ellipsis;
      }

      .tooltip-type {
        font-size: 0.6875rem;
        color: var(--text-tertiary, #80868b);
        text-transform: uppercase;
      }

      .expand-button {
        position: absolute;
        bottom: 0.5rem;
        right: 0.5rem;
        width: 1.75rem;
        height: 1.75rem;
        display: flex;
        align-items: center;
        justify-content: center;
        background: var(--surface-elevated, #fff);
        border: 1px solid var(--border-color, #e9ecef);
        border-radius: var(--radius-sm, 4px);
        cursor: pointer;
        font-size: 1rem;
        color: var(--text-secondary, #5f6368);
        transition: all 0.15s ease;
      }

      .expand-button:hover {
        background: var(--primary, #4285f4);
        color: white;
        border-color: var(--primary, #4285f4);
      }

      /* SVG Styles (applied via D3) */
      :host ::ng-deep .mini-graph-svg {
        display: block;
      }

      :host ::ng-deep .node-circle {
        cursor: pointer;
        transition: all 0.15s ease;
      }

      :host ::ng-deep .node-circle:hover {
        stroke-width: 3px;
      }

      :host ::ng-deep .node-circle.focus {
        fill: var(--primary, #4285f4);
        stroke: var(--primary-dark, #1967d2);
        stroke-width: 3px;
      }

      :host ::ng-deep .node-circle.neighbor {
        fill: var(--accent, #fbbc04);
        stroke: var(--accent-dark, #e37400);
      }

      :host ::ng-deep .edge-line {
        stroke: var(--border-color, #dadce0);
        stroke-width: 1.5px;
        fill: none;
      }

      :host ::ng-deep .edge-line.prerequisite {
        stroke: var(--warning, #ea8600);
        stroke-dasharray: 4, 2;
      }

      :host ::ng-deep .edge-line.extension {
        stroke: var(--success, #34a853);
      }

      :host ::ng-deep .edge-line.related {
        stroke: var(--info, #4285f4);
        stroke-dasharray: 2, 2;
      }

      :host ::ng-deep .edge-line.contains {
        stroke: var(--text-tertiary, #80868b);
      }

      :host ::ng-deep .node-label {
        font-size: 10px;
        fill: var(--text-primary, #202124);
        text-anchor: middle;
        pointer-events: none;
        user-select: none;
      }
    `,
  ],
})
export class MiniGraphComponent implements OnChanges, OnDestroy, AfterViewInit {
  /** Focus node ID (center of the graph) */
  @Input({ required: true }) focusNodeId!: string;

  /** Traversal depth from focus */
  @Input() depth = 1;

  /** Maximum nodes to display */
  @Input() maxNodes = 15;

  /** Container height in pixels */
  @Input() height = 200;

  /** Emitted when a node is clicked */
  @Output() nodeSelected = new EventEmitter<string>();

  /** Emitted when expand button is clicked */
  @Output() exploreRequested = new EventEmitter<void>();

  @ViewChild('graphContainer', { static: true }) graphContainer!: ElementRef<HTMLDivElement>;

  isLoading = true;
  isEmpty = false;
  hoveredNode: MiniGraphNode | null = null;
  tooltipX = 0;
  tooltipY = 0;

  private graphData: MiniGraphData | null = null;
  private svg: d3.Selection<SVGSVGElement, unknown, null, undefined> | null = null;
  private simulation: d3.Simulation<D3Node, D3Link> | null = null;
  private width = 300;
  private readonly destroy$ = new Subject<void>();

  constructor(
    private readonly relatedConceptsService: RelatedConceptsService,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngAfterViewInit(): void {
    this.initializeSvg();
  }

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['focusNodeId'] && this.focusNodeId) {
      this.loadNeighborhood();
    }
    if (changes['height'] && this.svg) {
      this.updateDimensions();
    }
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
    this.stopSimulation();
  }

  /**
   * Handle expand button click.
   */
  onExpandClick(): void {
    this.exploreRequested.emit();
  }

  /**
   * Initialize the SVG canvas.
   */
  private initializeSvg(): void {
    const container = this.graphContainer.nativeElement;
    this.width = container.clientWidth || 300;

    // Clear existing
    d3.select(container).selectAll('*').remove();

    // Create SVG
    this.svg = d3
      .select(container)
      .append('svg')
      .attr('class', 'mini-graph-svg')
      .attr('width', '100%')
      .attr('height', '100%')
      .attr('viewBox', `${-this.width / 2} ${-this.height / 2} ${this.width} ${this.height}`);

    // Add groups for edges and nodes (edges first so nodes are on top)
    this.svg.append('g').attr('class', 'edges-group');
    this.svg.append('g').attr('class', 'nodes-group');
  }

  /**
   * Update dimensions when height changes.
   */
  private updateDimensions(): void {
    if (!this.svg) return;
    this.svg.attr('viewBox', `${-this.width / 2} ${-this.height / 2} ${this.width} ${this.height}`);
    this.renderGraph();
  }

  /**
   * Load neighborhood data.
   */
  private loadNeighborhood(): void {
    this.isLoading = true;
    this.isEmpty = false;
    this.cdr.markForCheck();

    this.relatedConceptsService
      .getNeighborhood(this.focusNodeId, {
        depth: this.depth,
        maxNodes: this.maxNodes,
      })
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: data => {
          this.graphData = data;
          this.isEmpty = data.neighbors.length === 0;
          this.isLoading = false;
          this.cdr.markForCheck();
          this.renderGraph();
        },
        error: err => {
          console.error('Failed to load neighborhood:', err);
          this.isLoading = false;
          this.isEmpty = true;
          this.cdr.markForCheck();
        },
      });
  }

  /**
   * Render the graph visualization.
   */
  private renderGraph(): void {
    if (!this.svg || !this.graphData || this.isEmpty) return;

    // Stop existing simulation
    this.stopSimulation();

    // Prepare nodes
    const nodes: D3Node[] = [{ ...this.graphData.focus }, ...this.graphData.neighbors];

    // Prepare links (convert string IDs to node references)
    const links: D3Link[] = this.graphData.edges.map(edge => ({
      source: edge.source,
      target: edge.target,
      relationshipType: edge.relationshipType,
    }));

    // Create simulation
    this.simulation = d3
      .forceSimulation<D3Node, D3Link>(nodes)
      .force(
        'link',
        d3
          .forceLink<D3Node, D3Link>(links)
          .id(d => d.id)
          .distance(60)
      )
      .force('charge', d3.forceManyBody().strength(-150))
      .force('center', d3.forceCenter(0, 0))
      .force('collision', d3.forceCollide().radius(25));

    // Pin focus node to center
    const focusNode = nodes.find(n => n.isFocus);
    if (focusNode) {
      focusNode.fx = 0;
      focusNode.fy = 0;
    }

    // Render edges
    const edgesGroup = this.svg.select<SVGGElement>('g.edges-group');
    edgesGroup.selectAll('*').remove();

    const edgeSelection = edgesGroup
      .selectAll<SVGLineElement, D3Link>('line')
      .data(links)
      .enter()
      .append('line')
      .attr('class', d => `edge-line ${this.getEdgeClass(d.relationshipType)}`);

    // Render nodes
    const nodesGroup = this.svg.select<SVGGElement>('g.nodes-group');
    nodesGroup.selectAll('*').remove();

    const nodeSelection = nodesGroup
      .selectAll<SVGCircleElement, D3Node>('circle')
      .data(nodes)
      .enter()
      .append('circle')
      .attr('class', d => `node-circle ${d.isFocus ? 'focus' : 'neighbor'}`)
      .attr('r', d => (d.isFocus ? 14 : 10))
      .on('click', (_event, d) => {
        if (!d.isFocus) {
          this.nodeSelected.emit(d.id);
        }
      })
      .on('mouseenter', (event, d) => {
        this.hoveredNode = d;
        const rect = this.graphContainer.nativeElement.getBoundingClientRect();
        this.tooltipX = event.clientX - rect.left;
        this.tooltipY = event.clientY - rect.top;
        this.cdr.markForCheck();
      })
      .on('mouseleave', () => {
        this.hoveredNode = null;
        this.cdr.markForCheck();
      });

    // Add labels for focus node
    nodesGroup
      .selectAll<SVGTextElement, D3Node>('text')
      .data(nodes.filter(n => n.isFocus))
      .enter()
      .append('text')
      .attr('class', 'node-label')
      .attr('dy', 28)
      .text(d => this.truncateLabel(d.title, 15));

    // Update positions on tick
    this.simulation.on('tick', () => {
      edgeSelection
        .attr('x1', d => (d.source as D3Node).x ?? 0)
        .attr('y1', d => (d.source as D3Node).y ?? 0)
        .attr('x2', d => (d.target as D3Node).x ?? 0)
        .attr('y2', d => (d.target as D3Node).y ?? 0);

      nodeSelection.attr('cx', d => d.x ?? 0).attr('cy', d => d.y ?? 0);

      nodesGroup
        .selectAll<SVGTextElement, D3Node>('text')
        .attr('x', d => d.x ?? 0)
        .attr('y', d => d.y ?? 0);
    });

    // Run simulation for a short time then stop
    this.simulation.alpha(1).restart();
    setTimeout(() => {
      this.simulation?.alpha(0);
    }, 1500);
  }

  /**
   * Stop the D3 simulation.
   */
  private stopSimulation(): void {
    if (this.simulation) {
      this.simulation.stop();
      this.simulation = null;
    }
  }

  /**
   * Get CSS class for edge based on relationship type.
   */
  private getEdgeClass(type: RelationshipType): string {
    const classMap: Record<string, string> = {
      PREREQUISITE: 'prerequisite',
      FOUNDATION: 'prerequisite',
      DEPENDS_ON: 'prerequisite',
      EXTENDS: 'extension',
      RELATES_TO: 'related',
      CONTAINS: 'contains',
    };
    return classMap[type] || '';
  }

  /**
   * Truncate label to max length.
   */
  private truncateLabel(text: string, maxLength: number): string {
    if (text.length <= maxLength) return text;
    return text.substring(0, maxLength - 1) + '…';
  }
}
