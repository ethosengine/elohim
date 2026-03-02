import { CommonModule } from '@angular/common';
import {
  Component,
  Input,
  OnChanges,
  SimpleChanges,
  Output,
  EventEmitter,
  ViewChild,
  ElementRef,
  AfterViewInit,
  OnDestroy,
  NgZone,
} from '@angular/core';

import {
  HexNode,
  HexEdge,
  HexColorMode,
  HexLayoutMode,
  HexFocusState,
  HexLockState,
  BLOOM_ORDER,
} from './hexagon-grid.model';
import { getHexColor } from './hexagon-grid.palettes';
import { LAYOUTS, LayoutResult, ClusterInfo, DividerInfo } from './hexagon-grid.layout';
import { HexAnimationLoop } from './hexagon-grid.animation';

// Re-export for backward compatibility
export type { HexNode } from './hexagon-grid.model';

@Component({
  selector: 'lamad-hexagon-grid',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './hexagon-grid.component.html',
  styleUrls: ['./hexagon-grid.component.css'],
})
export class HexagonGridComponent implements OnChanges, AfterViewInit, OnDestroy {
  @Input() nodes: HexNode[] = [];
  @Input() itemsPerRow = 12;
  @Input() colorMode: HexColorMode = 'domain';
  @Input() showFreshness = false;
  @Input() edges: HexEdge[] = [];
  @Input() showEdges = false;
  @Input() layoutMode: HexLayoutMode = 'grid';
  @Input() showLockState = false;
  @Output() nodeClick = new EventEmitter<HexNode>();
  @Output() nodeFocus = new EventEmitter<HexNode | null>();

  @ViewChild('hexCanvas') canvasRef!: ElementRef<HTMLCanvasElement>;
  @ViewChild('wrapper') wrapperRef!: ElementRef<HTMLDivElement>;

  private ctx!: CanvasRenderingContext2D;
  private resizeObserver!: ResizeObserver;
  private animationLoop!: HexAnimationLoop;

  // Configuration
  private readonly hexRadius = 16;
  private readonly hexGap = 2;

  // Math helpers
  private readonly hexWidth = Math.sqrt(3) * this.hexRadius;
  private readonly hexHeight = 2 * this.hexRadius;
  private readonly xStep = this.hexWidth + this.hexGap;
  private readonly yStep = this.hexHeight * 0.75 + this.hexGap;

  hoveredNode: HexNode | null = null;
  private computedNodes: HexNode[] = [];
  private nodeMap = new Map<string, HexNode>();

  // Layout metadata for rendering
  private clusters: ClusterInfo[] = [];
  private dividers: DividerInfo[] = [];

  // Focus state
  private focusState: HexFocusState = {
    focusedNodeId: null,
    connectedNodeIds: new Set(),
    activeEdgeIds: new Set(),
    prerequisiteChainIds: new Set(),
  };

  // Lock state map
  private lockStates = new Map<string, HexLockState>();

  // Pulse animation for available nodes
  private pulsePhase = 0;
  private pulseRafId: number | null = null;

  // Tooltip state
  tooltipX = 0;
  tooltipY = 0;

  constructor(private ngZone: NgZone) {}

  ngAfterViewInit(): void {
    const canvas = this.canvasRef.nativeElement;
    this.ctx = canvas.getContext('2d')!;

    // Create animation loop outside Angular zone
    this.ngZone.runOutsideAngular(() => {
      this.animationLoop = new HexAnimationLoop(() => this.draw());
    });

    this.resizeObserver = new ResizeObserver(() => this.resizeCanvas());
    this.resizeObserver.observe(this.wrapperRef.nativeElement);
  }

  ngOnChanges(changes: SimpleChanges): void {
    if (
      this.ctx &&
      (changes['nodes'] ||
        changes['itemsPerRow'] ||
        changes['colorMode'] ||
        changes['showFreshness'] ||
        changes['layoutMode'])
    ) {
      this.resizeCanvas();
    }
    // Rebuild node map for edge lookups
    if (changes['nodes']) {
      this.nodeMap.clear();
      this.computedNodes.forEach(n => this.nodeMap.set(n.id, n));
    }
    // Compute lock states when relevant inputs change
    if (changes['nodes'] || changes['edges'] || changes['showLockState']) {
      if (this.showLockState) {
        this.computeLockStates();
        this.startPulseAnimation();
      } else {
        this.lockStates.clear();
        this.stopPulseAnimation();
      }
    }
    // Redraw on edge/palette change without relayout
    if (this.ctx && (changes['edges'] || changes['showEdges'] || changes['showLockState']) && !changes['nodes']) {
      this.draw();
    }
  }

  ngOnDestroy(): void {
    if (this.resizeObserver) {
      this.resizeObserver.disconnect();
    }
    if (this.animationLoop) {
      this.animationLoop.destroy();
    }
    this.stopPulseAnimation();
  }

  // ---------------------------------------------------------------------------
  // Layout & Canvas Sizing
  // ---------------------------------------------------------------------------

  private resizeCanvas(): void {
    const wrapper = this.wrapperRef.nativeElement;
    const canvas = this.canvasRef.nativeElement;

    if (!wrapper) return;

    const wrapperWidth = wrapper.offsetWidth;

    // Save old positions for animation
    const oldPositions = new Map<string, { x: number; y: number; scale: number; opacity: number }>();
    for (const n of this.computedNodes) {
      if (n.x !== undefined && n.y !== undefined) {
        oldPositions.set(n.id, { x: n.x, y: n.y, scale: n.scale ?? 1, opacity: n.opacity ?? 1 });
      }
    }

    const layoutResult = this.applyLayout(wrapperWidth);

    const dpr = window.devicePixelRatio || 1;
    const height = Math.max(layoutResult.totalHeight + 40, 200);

    canvas.width = wrapperWidth * dpr;
    canvas.height = height * dpr;

    canvas.style.width = `${wrapperWidth}px`;
    canvas.style.height = `${height}px`;

    this.ctx.resetTransform();
    this.ctx.scale(dpr, dpr);

    // Set up animation if old positions exist
    let hasOldPositions = false;
    for (const n of this.computedNodes) {
      const old = oldPositions.get(n.id);
      if (old) {
        // Store target position from layout, set current to old position
        n.targetX = n.x;
        n.targetY = n.y;
        n.targetScale = n.scale ?? 1;
        n.targetOpacity = n.opacity ?? 1;
        n.x = old.x;
        n.y = old.y;
        n.scale = old.scale;
        n.opacity = old.opacity;
        hasOldPositions = true;
      }
    }

    if (hasOldPositions && this.animationLoop) {
      this.ngZone.runOutsideAngular(() => {
        this.animationLoop.start(this.computedNodes);
      });
    } else {
      this.draw();
    }
  }

  private applyLayout(availableWidth: number): LayoutResult {
    const layoutFn = LAYOUTS[this.layoutMode] || LAYOUTS['grid'];
    const result = layoutFn(this.nodes, this.edges, {
      width: availableWidth,
      paddingTop: 40,
      paddingLeft: 0,
      paddingRight: 0,
    }, {
      hexRadius: this.hexRadius,
      hexGap: this.hexGap,
      hexWidth: this.hexWidth,
      hexHeight: this.hexHeight,
      xStep: this.xStep,
      yStep: this.yStep,
      itemsPerRow: this.itemsPerRow,
    });

    this.computedNodes = result.nodes;
    this.clusters = result.clusters;
    this.dividers = result.dividers;

    // Rebuild node map
    this.nodeMap.clear();
    this.computedNodes.forEach(n => this.nodeMap.set(n.id, n));

    return result;
  }

  // ---------------------------------------------------------------------------
  // Lock State Computation
  // ---------------------------------------------------------------------------

  private computeLockStates(): void {
    this.lockStates.clear();
    const prereqTypes = new Set(['requires', 'depends_on']);

    // Build prerequisite map: node → set of prerequisite node IDs
    const prereqMap = new Map<string, Set<string>>();
    for (const n of this.nodes) {
      prereqMap.set(n.id, new Set());
    }
    for (const e of this.edges) {
      if (prereqTypes.has(e.edgeType) && prereqMap.has(e.targetId)) {
        prereqMap.get(e.targetId)!.add(e.sourceId);
      }
    }

    const nodeById = new Map(this.nodes.map(n => [n.id, n]));

    for (const n of this.nodes) {
      const bloomOrder = n.masteryLevel ? BLOOM_ORDER[n.masteryLevel] : 0;

      if (bloomOrder >= BLOOM_ORDER['apply']) {
        this.lockStates.set(n.id, 'mastered');
      } else if (bloomOrder > 0) {
        this.lockStates.set(n.id, 'started');
      } else {
        // Check if all prerequisites are mastered
        const prereqs = prereqMap.get(n.id) || new Set();
        if (prereqs.size === 0) {
          this.lockStates.set(n.id, 'available');
        } else {
          const allPrereqsMet = Array.from(prereqs).every(pid => {
            const pNode = nodeById.get(pid);
            return pNode?.masteryLevel && BLOOM_ORDER[pNode.masteryLevel] >= BLOOM_ORDER['apply'];
          });
          this.lockStates.set(n.id, allPrereqsMet ? 'available' : 'locked');
        }
      }
    }
  }

  // ---------------------------------------------------------------------------
  // Pulse Animation (available nodes glow)
  // ---------------------------------------------------------------------------

  private startPulseAnimation(): void {
    if (this.pulseRafId !== null) return;
    const hasAvailable = Array.from(this.lockStates.values()).some(s => s === 'available');
    if (!hasAvailable) return;

    this.ngZone.runOutsideAngular(() => {
      const tick = () => {
        this.pulsePhase = (this.pulsePhase + 0.03) % (Math.PI * 2);
        this.draw();
        this.pulseRafId = requestAnimationFrame(tick);
      };
      this.pulseRafId = requestAnimationFrame(tick);
    });
  }

  private stopPulseAnimation(): void {
    if (this.pulseRafId !== null) {
      cancelAnimationFrame(this.pulseRafId);
      this.pulseRafId = null;
    }
  }

  // ---------------------------------------------------------------------------
  // Focus State (click-to-expand)
  // ---------------------------------------------------------------------------

  private setFocus(nodeId: string | null): void {
    if (nodeId === null) {
      // Collapse: restore all nodes
      this.focusState = {
        focusedNodeId: null,
        connectedNodeIds: new Set(),
        activeEdgeIds: new Set(),
        prerequisiteChainIds: new Set(),
      };

      // Restore original positions
      for (const n of this.computedNodes) {
        if (n.originalX !== undefined) {
          n.targetX = n.originalX;
          n.targetY = n.originalY;
          n.targetScale = (n as any)._origScale ?? 1;
          n.targetOpacity = 1;
        }
      }
    } else {
      const focused = this.nodeMap.get(nodeId);
      if (!focused) return;

      // Find 1-hop connected nodes
      const connectedIds = new Set<string>();
      const activeEdgeIds = new Set<string>();
      for (const e of this.edges) {
        if (e.sourceId === nodeId) {
          connectedIds.add(e.targetId);
          activeEdgeIds.add(e.id);
        }
        if (e.targetId === nodeId) {
          connectedIds.add(e.sourceId);
          activeEdgeIds.add(e.id);
        }
      }

      // Trace prerequisite chain backward
      const prereqChain = new Set<string>();
      const prereqTypes = new Set(['requires', 'depends_on']);
      const traceQueue = [nodeId];
      let head = 0;
      while (head < traceQueue.length) {
        const current = traceQueue[head++];
        for (const e of this.edges) {
          if (e.targetId === current && prereqTypes.has(e.edgeType) && !prereqChain.has(e.sourceId)) {
            prereqChain.add(e.sourceId);
            activeEdgeIds.add(e.id);
            traceQueue.push(e.sourceId);
          }
        }
      }

      this.focusState = {
        focusedNodeId: nodeId,
        connectedNodeIds: connectedIds,
        activeEdgeIds,
        prerequisiteChainIds: prereqChain,
      };

      // Save original positions and compute focus layout
      const canvas = this.canvasRef.nativeElement;
      const canvasW = canvas.width / (window.devicePixelRatio || 1);
      const canvasH = canvas.height / (window.devicePixelRatio || 1);
      const centerX = canvasW / 2;
      const centerY = canvasH / 2;

      for (const n of this.computedNodes) {
        if (n.originalX === undefined) {
          n.originalX = n.x;
          n.originalY = n.y;
          (n as any)._origScale = n.scale ?? 1;
        }

        if (n.id === nodeId) {
          // Focused node: animate to center, scale up
          n.targetX = centerX;
          n.targetY = centerY;
          n.targetScale = 1.8;
          n.targetOpacity = 1;
        } else if (connectedIds.has(n.id)) {
          // Connected nodes: arrange in ring around focused
          const connArr = Array.from(connectedIds);
          const idx = connArr.indexOf(n.id);
          const angle = (idx / connArr.length) * Math.PI * 2 - Math.PI / 2;
          const ringRadius = this.hexRadius * 6;
          n.targetX = centerX + ringRadius * Math.cos(angle);
          n.targetY = centerY + ringRadius * Math.sin(angle);
          n.targetScale = 1.2;
          n.targetOpacity = 1;
        } else if (prereqChain.has(n.id)) {
          // Prereq chain: position above focused node
          const chainArr = Array.from(prereqChain);
          const idx = chainArr.indexOf(n.id);
          n.targetX = centerX;
          n.targetY = centerY - (idx + 1) * this.hexRadius * 3.5;
          n.targetScale = 1;
          n.targetOpacity = 0.7;
        } else {
          // All others: fade out
          n.targetScale = 0.6;
          n.targetOpacity = 0.12;
          // Keep original position but shrink
          n.targetX = n.originalX;
          n.targetY = n.originalY;
        }
      }
    }

    // Animate the transition
    if (this.animationLoop) {
      this.ngZone.runOutsideAngular(() => {
        this.animationLoop.start(this.computedNodes);
      });
    }

    this.nodeFocus.emit(nodeId ? this.nodeMap.get(nodeId) || null : null);
  }

  // ---------------------------------------------------------------------------
  // Draw Pipeline
  // ---------------------------------------------------------------------------

  private draw(): void {
    if (!this.ctx || !this.computedNodes.length) return;

    const width = this.canvasRef.nativeElement.width / (window.devicePixelRatio || 1);
    const height = this.canvasRef.nativeElement.height / (window.devicePixelRatio || 1);

    this.ctx.clearRect(0, 0, width, height);

    // 1. Draw cluster background hulls
    this.drawClusterHulls();

    // 2. Draw Bloom gate line (DAG + bloom mode)
    if (this.layoutMode === 'dag' && this.colorMode === 'bloom') {
      this.drawBloomGateLine(width, height);
    }

    // 2b. Draw scatter axis lines (only for scatter layout)
    if (this.layoutMode === 'scatter') {
      this.drawScatterAxes(width, height);
    }

    // 3. Draw connections/edges
    if (this.showEdges && this.edges.length) {
      this.drawConnections();
    }

    // 4. Draw category labels when no dividers exist (non-grid/tree layouts)
    if (!this.dividers.length) {
      this.drawCategoryLabels();
    }

    // 5. Draw all hex nodes
    this.computedNodes.forEach(node => {
      this.drawHexNode(node, node === this.hoveredNode);
    });

    // 6. Draw divider labels ON TOP of hexagons so they're never obscured
    this.drawDividers(width);
  }

  // ---------------------------------------------------------------------------
  // Cluster Background Hulls
  // ---------------------------------------------------------------------------

  private drawClusterHulls(): void {
    if (!this.clusters.length) return;
    const ctx = this.ctx;

    for (const cluster of this.clusters) {
      if (cluster.hull.length < 3) continue;

      ctx.save();

      // Determine cluster color from first node's palette
      const firstNode = cluster.nodes[0];
      const colors = getHexColor(firstNode, this.colorMode, this.showFreshness);

      // Draw filled hull
      ctx.beginPath();
      ctx.moveTo(cluster.hull[0].x, cluster.hull[0].y);
      for (let i = 1; i < cluster.hull.length; i++) {
        ctx.lineTo(cluster.hull[i].x, cluster.hull[i].y);
      }
      ctx.closePath();

      ctx.fillStyle = this.withAlpha(colors.fill, 0.06);
      ctx.fill();

      ctx.strokeStyle = this.withAlpha(colors.fill, 0.12);
      ctx.lineWidth = 1;
      ctx.stroke();

      ctx.restore();
    }
  }

  // ---------------------------------------------------------------------------
  // Grid Dividers (Khan-style unit row headers)
  // ---------------------------------------------------------------------------

  private drawDividers(canvasWidth: number): void {
    if (!this.dividers.length) return;
    const ctx = this.ctx;

    ctx.save();
    ctx.font = '600 10px Inter, system-ui, sans-serif';
    ctx.textBaseline = 'bottom';

    for (const div of this.dividers) {
      // Horizontal rule
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.08)';
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(div.x, div.y - 2);
      ctx.lineTo(div.x + div.width, div.y - 2);
      ctx.stroke();

      // Label with background pill for legibility
      const labelText = div.label.toUpperCase();
      const textMetrics = ctx.measureText(labelText);
      const labelX = div.x;
      const labelY = div.y + 12;
      const pillPadX = 4;
      const pillPadY = 2;

      ctx.fillStyle = 'rgba(15, 23, 42, 0.7)';
      ctx.beginPath();
      ctx.roundRect(
        labelX - pillPadX,
        labelY - 10 - pillPadY,
        textMetrics.width + pillPadX * 2,
        12 + pillPadY * 2,
        3,
      );
      ctx.fill();

      ctx.fillStyle = 'rgba(255, 255, 255, 0.6)';
      ctx.fillText(labelText, labelX, labelY);
    }

    ctx.restore();
  }

  // ---------------------------------------------------------------------------
  // Scatter Axis Lines
  // ---------------------------------------------------------------------------

  private drawScatterAxes(canvasWidth: number, canvasHeight: number): void {
    const ctx = this.ctx;
    const padding = 60;
    const cx = canvasWidth / 2;
    const cy = this.computedNodes.length
      ? this.computedNodes.reduce((s, n) => s + (n.y || 0), 0) / this.computedNodes.length
      : canvasHeight / 2;

    ctx.save();
    ctx.setLineDash([6, 4]);
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.12)';
    ctx.lineWidth = 1;

    // Vertical axis
    ctx.beginPath();
    ctx.moveTo(cx, padding);
    ctx.lineTo(cx, canvasHeight - padding);
    ctx.stroke();

    // Horizontal axis
    ctx.beginPath();
    ctx.moveTo(padding, cy);
    ctx.lineTo(canvasWidth - padding, cy);
    ctx.stroke();

    ctx.setLineDash([]);

    // Quadrant labels
    ctx.font = '600 9px Inter, system-ui, sans-serif';
    ctx.textBaseline = 'top';
    ctx.fillStyle = 'rgba(255, 255, 255, 0.2)';
    ctx.textAlign = 'center';
    ctx.fillText('AUTHORITARIAN LEFT', cx / 2, padding + 4);
    ctx.fillText('AUTHORITARIAN RIGHT', cx + cx / 2, padding + 4);
    ctx.textBaseline = 'bottom';
    ctx.fillText('LIBERTARIAN LEFT', cx / 2, canvasHeight - padding - 4);
    ctx.fillText('LIBERTARIAN RIGHT', cx + cx / 2, canvasHeight - padding - 4);

    ctx.restore();
  }

  // ---------------------------------------------------------------------------
  // Bloom Gate Line (DAG + bloom mode)
  // ---------------------------------------------------------------------------

  private drawBloomGateLine(canvasWidth: number, canvasHeight: number): void {
    // Find the Y boundary between "understand" and "apply" nodes
    let maxUnderstandY = 0;
    let minApplyY = canvasHeight;

    for (const n of this.computedNodes) {
      if (!n.masteryLevel || n.y === undefined) continue;
      const bloomOrder = BLOOM_ORDER[n.masteryLevel];
      if (bloomOrder <= BLOOM_ORDER['understand'] && n.y > maxUnderstandY) {
        maxUnderstandY = n.y;
      }
      if (bloomOrder >= BLOOM_ORDER['apply'] && n.y < minApplyY) {
        minApplyY = n.y;
      }
    }

    // Only draw if there's a clear boundary
    if (maxUnderstandY === 0 || minApplyY >= canvasHeight) return;
    const gateY = (maxUnderstandY + minApplyY) / 2;

    const ctx = this.ctx;
    const padding = 30;

    ctx.save();

    // Zone background washes
    // Above gate: warm wash (passive knowledge zone)
    ctx.fillStyle = 'rgba(251, 191, 36, 0.03)';
    ctx.fillRect(padding, 0, canvasWidth - padding * 2, gateY);

    // Below gate: cool wash (active capability zone)
    ctx.fillStyle = 'rgba(59, 130, 246, 0.03)';
    ctx.fillRect(padding, gateY, canvasWidth - padding * 2, canvasHeight - gateY);

    // Dashed gate line
    ctx.setLineDash([8, 6]);
    ctx.strokeStyle = 'rgba(251, 191, 36, 0.35)';
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(padding, gateY);
    ctx.lineTo(canvasWidth - padding, gateY);
    ctx.stroke();
    ctx.setLineDash([]);

    // Zone labels
    ctx.font = '600 8px Inter, system-ui, sans-serif';
    ctx.textBaseline = 'bottom';
    ctx.fillStyle = 'rgba(251, 191, 36, 0.4)';
    ctx.fillText('PASSIVE KNOWLEDGE', padding + 4, gateY - 4);

    ctx.textBaseline = 'top';
    ctx.fillStyle = 'rgba(59, 130, 246, 0.4)';
    ctx.fillText('ACTIVE CAPABILITY', padding + 4, gateY + 4);

    ctx.restore();
  }

  // ---------------------------------------------------------------------------
  // Connection Renderer (structural edges)
  // ---------------------------------------------------------------------------

  private drawConnections(): void {
    const ctx = this.ctx;
    const isFocused = this.focusState.focusedNodeId !== null;

    for (const edge of this.edges) {
      const source = this.nodeMap.get(edge.sourceId);
      const target = this.nodeMap.get(edge.targetId);
      if (!source?.x || !source?.y || !target?.x || !target?.y) continue;

      const dx = target.x - source.x;
      const dy = target.y - source.y;
      const dist = Math.sqrt(dx * dx + dy * dy);
      const confidence = edge.confidence ?? 0.7;

      ctx.save();

      // Focus dimming: non-active edges fade out
      if (isFocused && !this.focusState.activeEdgeIds.has(edge.id)) {
        ctx.globalAlpha = 0.05;
      } else if (isFocused && this.focusState.activeEdgeIds.has(edge.id)) {
        // Active edges get glow
        ctx.shadowColor = 'rgba(255, 255, 255, 0.3)';
        ctx.shadowBlur = 8;
      }

      // Opposes edges get red glow
      if (edge.edgeType === 'opposes') {
        ctx.shadowColor = 'rgba(239, 68, 68, 0.3)';
        ctx.shadowBlur = 4;
      }

      // Select connection style based on context
      if (this.layoutMode === 'radial' && this.isHubNode(source)) {
        this.drawTaperedSpoke(ctx, source, target, edge, confidence);
      } else if (dist < this.hexRadius * 4.5 && this.layoutMode === 'dag') {
        this.drawTightBridge(ctx, source, target, edge, confidence);
      } else if (source.category !== target.category || edge.edgeType === 'shared_concept' || edge.edgeType === 'teaches') {
        this.drawBridgeArc(ctx, source, target, edge, confidence);
      } else if (confidence < 0.4 || edge.edgeType === 'relates_to') {
        this.drawGhostThread(ctx, source, target, edge, confidence);
      } else {
        this.drawBridgeArc(ctx, source, target, edge, confidence);
      }

      ctx.restore();
    }
  }

  /** Tight bridge: filled trapezoid between close nodes (DAG chains). */
  private drawTightBridge(
    ctx: CanvasRenderingContext2D,
    source: HexNode,
    target: HexNode,
    edge: HexEdge,
    confidence: number,
  ): void {
    const sx = source.x!;
    const sy = source.y!;
    const tx = target.x!;
    const ty = target.y!;

    const angle = Math.atan2(ty - sy, tx - sx);
    const perpX = -Math.sin(angle);
    const perpY = Math.cos(angle);

    const sourceW = this.hexRadius * 0.5;
    const targetW = this.hexRadius * 0.5;

    // Start and end points pulled to hex edges
    const startX = sx + Math.cos(angle) * this.hexRadius * 0.8;
    const startY = sy + Math.sin(angle) * this.hexRadius * 0.8;
    const endX = tx - Math.cos(angle) * this.hexRadius * 0.8;
    const endY = ty - Math.sin(angle) * this.hexRadius * 0.8;

    // Get colors for gradient blend
    const sourceColors = getHexColor(source, this.colorMode, this.showFreshness);
    const targetColors = getHexColor(target, this.colorMode, this.showFreshness);

    ctx.beginPath();
    ctx.moveTo(startX + perpX * sourceW, startY + perpY * sourceW);
    ctx.lineTo(endX + perpX * targetW, endY + perpY * targetW);
    ctx.lineTo(endX - perpX * targetW, endY - perpY * targetW);
    ctx.lineTo(startX - perpX * sourceW, startY - perpY * sourceW);
    ctx.closePath();

    ctx.fillStyle = this.withAlpha(this.blendColors(sourceColors.fill, targetColors.fill), 0.35 * confidence);
    ctx.fill();

    // Arrowhead for directed edges
    if (edge.directed) {
      this.drawArrowhead(ctx, sx, sy, tx, ty, edge.color || sourceColors.fill);
    }
  }

  /** Tapered spoke: width tapers from hub to spoke tip (radial). */
  private drawTaperedSpoke(
    ctx: CanvasRenderingContext2D,
    source: HexNode,
    target: HexNode,
    edge: HexEdge,
    confidence: number,
  ): void {
    const sx = source.x!;
    const sy = source.y!;
    const tx = target.x!;
    const ty = target.y!;

    const angle = Math.atan2(ty - sy, tx - sx);
    const perpX = -Math.sin(angle);
    const perpY = Math.cos(angle);

    const hubW = 3.5; // Wide at hub
    const tipW = 1.0; // Narrow at tip

    const startX = sx + Math.cos(angle) * this.hexRadius * 1.2;
    const startY = sy + Math.sin(angle) * this.hexRadius * 1.2;
    const endX = tx - Math.cos(angle) * this.hexRadius;
    const endY = ty - Math.sin(angle) * this.hexRadius;

    const sourceColors = getHexColor(source, this.colorMode, this.showFreshness);
    const targetColors = getHexColor(target, this.colorMode, this.showFreshness);

    // Create gradient along the spoke
    const gradient = ctx.createLinearGradient(startX, startY, endX, endY);
    gradient.addColorStop(0, this.withAlpha(sourceColors.fill, 0.5 * confidence));
    gradient.addColorStop(1, this.withAlpha(targetColors.fill, 0.25 * confidence));

    ctx.beginPath();
    ctx.moveTo(startX + perpX * hubW, startY + perpY * hubW);
    ctx.lineTo(endX + perpX * tipW, endY + perpY * tipW);
    ctx.lineTo(endX - perpX * tipW, endY - perpY * tipW);
    ctx.lineTo(startX - perpX * hubW, startY - perpY * hubW);
    ctx.closePath();

    ctx.fillStyle = gradient;
    ctx.fill();
  }

  /** Bridge arc: curved bezier with color gradient (cross-category). */
  private drawBridgeArc(
    ctx: CanvasRenderingContext2D,
    source: HexNode,
    target: HexNode,
    edge: HexEdge,
    confidence: number,
  ): void {
    const sx = source.x!;
    const sy = source.y!;
    const tx = target.x!;
    const ty = target.y!;

    const midX = (sx + tx) / 2;
    const midY = (sy + ty) / 2;
    const dx = tx - sx;
    const dy = ty - sy;
    const dist = Math.sqrt(dx * dx + dy * dy);

    // Perpendicular offset for curve
    const curvature = Math.min(dist * 0.15, 30);
    const offsetX = (-dy / dist) * curvature;
    const offsetY = (dx / dist) * curvature;

    const sourceColors = getHexColor(source, this.colorMode, this.showFreshness);
    const style = this.EDGE_STYLES[edge.edgeType] || this.EDGE_STYLES['custom'];

    // Create gradient
    const gradient = ctx.createLinearGradient(sx, sy, tx, ty);
    gradient.addColorStop(0, edge.color || this.withAlpha(sourceColors.fill, 0.5 * confidence));
    gradient.addColorStop(1, edge.color || this.withAlpha(
      getHexColor(target, this.colorMode, this.showFreshness).fill,
      0.5 * confidence,
    ));

    ctx.beginPath();
    ctx.moveTo(sx, sy);
    ctx.quadraticCurveTo(midX + offsetX, midY + offsetY, tx, ty);

    ctx.strokeStyle = gradient;
    ctx.lineWidth = (style.width || 2) * confidence;
    ctx.setLineDash(style.dash || []);
    ctx.globalAlpha = Math.min(ctx.globalAlpha, confidence);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.globalAlpha = 1;

    // Arrowhead for directed edges
    if (edge.directed) {
      this.drawArrowhead(ctx, midX + offsetX, midY + offsetY, tx, ty, edge.color || sourceColors.fill);
    }
  }

  /** Ghost thread: thin dotted line for weak connections. */
  private drawGhostThread(
    ctx: CanvasRenderingContext2D,
    source: HexNode,
    target: HexNode,
    edge: HexEdge,
    confidence: number,
  ): void {
    const style = this.EDGE_STYLES[edge.edgeType] || this.EDGE_STYLES['custom'];

    ctx.beginPath();
    ctx.moveTo(source.x!, source.y!);
    ctx.lineTo(target.x!, target.y!);

    ctx.strokeStyle = edge.color || style.color;
    ctx.lineWidth = 1;
    ctx.setLineDash([2, 3]);
    ctx.globalAlpha = Math.min(ctx.globalAlpha, confidence * 0.3);
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.globalAlpha = 1;
  }

  private isHubNode(node: HexNode): boolean {
    // A hub is the radial center (scale > 1) or has high degree
    if (node.scale && node.scale > 1.3) return true;
    if (node.owner === 'self' || node.group === 'self') return true;
    const degree = this.edges.filter(e => e.sourceId === node.id || e.targetId === node.id).length;
    return degree > 5;
  }

  // ---------------------------------------------------------------------------
  // Hex Node Rendering
  // ---------------------------------------------------------------------------

  private drawHexNode(node: HexNode, isHovered: boolean): void {
    if (node.x === undefined || node.y === undefined) return;

    const ctx = this.ctx;
    const nodeScale = node.scale || 1;
    const size = (isHovered ? this.hexRadius * 1.4 : this.hexRadius) * nodeScale;
    const nodeOpacity = node.opacity ?? 1;

    const colors = getHexColor(node, this.colorMode, this.showFreshness);
    const lockState = this.lockStates.get(node.id);

    ctx.save();
    if (nodeOpacity < 1) {
      ctx.globalAlpha = nodeOpacity;
    }

    // Lock state: locked nodes are desaturated and dimmed
    if (this.showLockState && lockState === 'locked') {
      ctx.globalAlpha = (node.opacity ?? 1) * 0.3;
    }

    // Build hex path
    this.buildHexPath(ctx, node.x, node.y, size);

    // Fill with palette color
    ctx.fillStyle = colors.fill;

    // Glow effect
    if (isHovered) {
      ctx.shadowColor = colors.glowColor || colors.fill;
      ctx.shadowBlur = 20;
    } else if (colors.glowBlur) {
      ctx.shadowColor = colors.glowColor!;
      ctx.shadowBlur = colors.glowBlur;
    } else {
      ctx.shadowBlur = 0;
    }

    ctx.fill();
    ctx.shadowBlur = 0;

    // Ring (gate glow for Bloom's "apply" and "create" levels)
    if (colors.ringColor) {
      this.buildHexPath(ctx, node.x, node.y, size + 1);
      ctx.strokeStyle = colors.ringColor;
      ctx.lineWidth = colors.ringWidth || 2;
      ctx.stroke();
    }

    // Lock state visuals
    if (this.showLockState && lockState) {
      this.drawLockStateOverlay(ctx, node, size, lockState);
    }

    // Focus indicator: glow ring on focused node
    if (this.focusState.focusedNodeId === node.id) {
      this.buildHexPath(ctx, node.x, node.y, size + 3);
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.8)';
      ctx.lineWidth = 2;
      ctx.shadowColor = 'rgba(255, 255, 255, 0.5)';
      ctx.shadowBlur = 12;
      ctx.stroke();
      ctx.shadowBlur = 0;
    }

    ctx.restore();
  }

  /** Draw lock state overlay (lock icon for locked, pulse ring for available). */
  private drawLockStateOverlay(
    ctx: CanvasRenderingContext2D,
    node: HexNode,
    size: number,
    state: HexLockState,
  ): void {
    if (state === 'locked') {
      // Draw padlock icon
      const x = node.x!;
      const y = node.y!;
      const iconSize = size * 0.45;

      ctx.save();
      ctx.globalAlpha = 0.6;

      // Lock body (rectangle)
      const bodyW = iconSize * 0.7;
      const bodyH = iconSize * 0.5;
      const bodyX = x - bodyW / 2;
      const bodyY = y - bodyH / 2 + iconSize * 0.15;

      ctx.fillStyle = 'rgba(0, 0, 0, 0.5)';
      ctx.beginPath();
      ctx.roundRect(bodyX, bodyY, bodyW, bodyH, 1);
      ctx.fill();

      // Lock shackle (arc)
      ctx.strokeStyle = 'rgba(0, 0, 0, 0.5)';
      ctx.lineWidth = iconSize * 0.12;
      ctx.beginPath();
      ctx.arc(x, bodyY, bodyW * 0.35, Math.PI, 0);
      ctx.stroke();

      ctx.restore();
    } else if (state === 'available') {
      // Animated pulse ring
      const pulseAlpha = 0.2 + Math.sin(this.pulsePhase) * 0.15;
      const pulseSize = size + 2 + Math.sin(this.pulsePhase) * 1.5;

      ctx.save();
      this.buildHexPath(ctx, node.x!, node.y!, pulseSize);
      ctx.strokeStyle = `rgba(74, 222, 128, ${pulseAlpha})`;
      ctx.lineWidth = 1.5;
      ctx.stroke();
      ctx.restore();
    }
  }

  /** Build a pointy-topped hexagon path centered at (cx, cy). */
  private buildHexPath(ctx: CanvasRenderingContext2D, cx: number, cy: number, size: number): void {
    ctx.beginPath();
    for (let i = 0; i < 6; i++) {
      const angle_deg = 60 * i - 30;
      const angle_rad = (Math.PI / 180) * angle_deg;
      const px = cx + size * Math.cos(angle_rad);
      const py = cy + size * Math.sin(angle_rad);
      if (i === 0) ctx.moveTo(px, py);
      else ctx.lineTo(px, py);
    }
    ctx.closePath();
  }

  // ---------------------------------------------------------------------------
  // Edge Styles (kept for fallback reference)
  // ---------------------------------------------------------------------------

  private readonly EDGE_STYLES: Record<string, { dash: number[]; width: number; color: string }> = {
    contains: { dash: [], width: 1, color: 'rgba(148, 163, 184, 0.3)' },
    depends_on: { dash: [], width: 2, color: 'rgba(251, 191, 36, 0.5)' },
    follows: { dash: [6, 4], width: 1.5, color: 'rgba(56, 189, 248, 0.4)' },
    relates_to: { dash: [2, 3], width: 1, color: 'rgba(148, 163, 184, 0.25)' },
    requires: { dash: [], width: 2.5, color: 'rgba(239, 68, 68, 0.5)' },
    shared_concept: { dash: [4, 2], width: 2, color: 'rgba(168, 85, 247, 0.5)' },
    social: { dash: [], width: 1.5, color: 'rgba(52, 211, 153, 0.4)' },
    teaches: { dash: [8, 3], width: 1.5, color: 'rgba(251, 146, 60, 0.5)' },
    opposes: { dash: [], width: 2, color: 'rgba(239, 68, 68, 0.5)' },
    custom: { dash: [], width: 1, color: 'rgba(148, 163, 184, 0.3)' },
  };

  // ---------------------------------------------------------------------------
  // Drawing Helpers
  // ---------------------------------------------------------------------------

  private drawArrowhead(
    ctx: CanvasRenderingContext2D,
    fromX: number,
    fromY: number,
    toX: number,
    toY: number,
    color?: string,
  ): void {
    const headLen = 8;
    const angle = Math.atan2(toY - fromY, toX - fromX);
    const tipX = toX - Math.cos(angle) * this.hexRadius;
    const tipY = toY - Math.sin(angle) * this.hexRadius;

    ctx.beginPath();
    ctx.moveTo(tipX, tipY);
    ctx.lineTo(tipX - headLen * Math.cos(angle - Math.PI / 6), tipY - headLen * Math.sin(angle - Math.PI / 6));
    ctx.lineTo(tipX - headLen * Math.cos(angle + Math.PI / 6), tipY - headLen * Math.sin(angle + Math.PI / 6));
    ctx.closePath();
    ctx.fillStyle = color || (ctx.strokeStyle as string);
    ctx.fill();
  }

  private drawCategoryLabels(): void {
    if (!this.computedNodes.some(n => n.category)) return;

    const ctx = this.ctx;
    const categories = new Map<string, { minY: number; x: number }>();

    for (const node of this.computedNodes) {
      if (!node.category || node.x === undefined || node.y === undefined) continue;
      const existing = categories.get(node.category);
      if (!existing || node.y < existing.minY) {
        categories.set(node.category, { minY: node.y, x: node.x });
      }
    }

    ctx.save();
    ctx.font = '600 10px Inter, system-ui, sans-serif';
    ctx.textBaseline = 'bottom';

    for (const [label, pos] of categories) {
      ctx.fillStyle = 'rgba(255, 255, 255, 0.45)';
      ctx.fillText(
        label.toUpperCase(),
        pos.x - ctx.measureText(label.toUpperCase()).width / 2,
        pos.minY - this.hexRadius - 4,
      );
    }

    ctx.restore();
  }

  // ---------------------------------------------------------------------------
  // Color Helpers
  // ---------------------------------------------------------------------------

  /** Set the alpha of an rgba/hsla color string. */
  private withAlpha(color: string, alpha: number): string {
    // Handle rgba
    const rgbaMatch = color.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*[\d.]+)?\)/);
    if (rgbaMatch) {
      return `rgba(${rgbaMatch[1]}, ${rgbaMatch[2]}, ${rgbaMatch[3]}, ${alpha})`;
    }
    // Handle hsla
    const hslaMatch = color.match(/hsla?\((\d+),\s*([\d.]+)%,\s*([\d.]+)%(?:,\s*[\d.]+)?\)/);
    if (hslaMatch) {
      return `hsla(${hslaMatch[1]}, ${hslaMatch[2]}%, ${hslaMatch[3]}%, ${alpha})`;
    }
    // Handle hex
    if (color.startsWith('#')) {
      const r = parseInt(color.slice(1, 3), 16);
      const g = parseInt(color.slice(3, 5), 16);
      const b = parseInt(color.slice(5, 7), 16);
      return `rgba(${r}, ${g}, ${b}, ${alpha})`;
    }
    return color;
  }

  /** Simple midpoint blend of two color strings. */
  private blendColors(c1: string, c2: string): string {
    const parse = (c: string) => {
      const m = c.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
      if (m) return [+m[1], +m[2], +m[3]];
      if (c.startsWith('#')) {
        return [
          parseInt(c.slice(1, 3), 16),
          parseInt(c.slice(3, 5), 16),
          parseInt(c.slice(5, 7), 16),
        ];
      }
      return [128, 128, 128];
    };
    const [r1, g1, b1] = parse(c1);
    const [r2, g2, b2] = parse(c2);
    return `rgb(${Math.round((r1 + r2) / 2)}, ${Math.round((g1 + g2) / 2)}, ${Math.round((b1 + b2) / 2)})`;
  }

  // ---------------------------------------------------------------------------
  // Mouse / Touch / Keyboard Interaction
  // ---------------------------------------------------------------------------

  onMouseMove(event: MouseEvent): void {
    const rect = this.canvasRef.nativeElement.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;

    const hit = this.computedNodes.find(node => {
      if (node.x === undefined || node.y === undefined) return false;
      const nodeScale = node.scale || 1;
      const dx = node.x - x;
      const dy = node.y - y;
      return Math.sqrt(dx * dx + dy * dy) < this.hexRadius * nodeScale;
    });

    if ((hit ?? null) !== this.hoveredNode) {
      this.hoveredNode = hit ?? null;
      this.draw();
    }

    if (this.hoveredNode) {
      this.tooltipX = x;
      this.tooltipY = y;
    }
  }

  getAffinityPercentage(affinity: number): number {
    return Math.round((affinity || 0) * 100);
  }

  getLockStateLabel(node: HexNode): string {
    const state = this.lockStates.get(node.id);
    if (!state) return '';
    switch (state) {
      case 'locked': return 'Prerequisites needed';
      case 'available': return 'Ready to start';
      case 'started': return 'In progress';
      case 'mastered': return 'Mastered';
    }
  }

  onMouseLeave(): void {
    if (this.hoveredNode) {
      this.hoveredNode = null;
      this.draw();
    }
  }

  onClick(_event: MouseEvent): void {
    if (this.hoveredNode) {
      // Toggle focus on click
      if (this.focusState.focusedNodeId === this.hoveredNode.id) {
        this.setFocus(null); // Collapse
      } else {
        this.setFocus(this.hoveredNode.id); // Focus new node
      }
      this.nodeClick.emit(this.hoveredNode);
    } else if (this.focusState.focusedNodeId) {
      // Click on background → collapse
      this.setFocus(null);
    }
  }

  onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && this.focusState.focusedNodeId) {
      this.setFocus(null);
      event.preventDefault();
    } else if (event.key === 'Tab' && this.focusState.focusedNodeId) {
      // Tab through connected nodes
      const connArr = Array.from(this.focusState.connectedNodeIds);
      if (connArr.length > 0) {
        event.preventDefault();
        const currentIdx = connArr.indexOf(this.focusState.focusedNodeId);
        const nextIdx = event.shiftKey
          ? (currentIdx - 1 + connArr.length) % connArr.length
          : (currentIdx + 1) % connArr.length;
        this.setFocus(connArr[nextIdx]);
      }
    }
  }
}
