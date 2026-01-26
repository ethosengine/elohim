import { CommonModule } from '@angular/common';
import {
  Component,
  Input,
  Output,
  EventEmitter,
  OnInit,
  OnChanges,
  SimpleChanges,
  ElementRef,
  ViewChild,
  AfterViewInit,
} from '@angular/core';

import {
  GovernanceSignalService,
  OpinionCluster,
} from '@app/elohim/services/governance-signal.service';

/**
 * OpinionClusterComponent - Polis-style 2D Opinion Visualization
 *
 * Implements consensus discovery through:
 * - 2D scatter plot of opinions (PCA-reduced dimensions)
 * - Cluster identification (consensus groups)
 * - Consensus vs divisive statement highlighting
 * - Interactive exploration of opinion space
 *
 * Inspired by Polis: "The goal is not to win, but to understand."
 *
 * The visualization helps communities:
 * - See where opinions cluster (potential consensus)
 * - Identify bridging statements (unifying)
 * - Understand divisive topics (need more dialogue)
 * - Locate their own position in the opinion landscape
 */
@Component({
  selector: 'app-opinion-cluster',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './opinion-cluster.component.html',
  styleUrls: ['./opinion-cluster.component.css'],
})
export class OpinionClusterComponent implements OnInit, OnChanges, AfterViewInit {
  @ViewChild('canvas') canvasRef!: ElementRef<HTMLCanvasElement>;

  @Input() contextId!: string;
  @Input() statements: Statement[] = [];
  @Input() votes: StatementVote[] = [];
  @Input() showLabels = true;
  @Input() highlightConsensus = true;
  @Input() interactive = true;

  @Output() statementSelected = new EventEmitter<Statement>();
  @Output() clusterSelected = new EventEmitter<OpinionCluster>();

  // Visualization state
  clusters: OpinionCluster[] = [];
  participants: ParticipantPosition[] = [];
  consensusStatements: Statement[] = [];
  divisiveStatements: Statement[] = [];

  // Canvas state
  private ctx: CanvasRenderingContext2D | null = null;
  private readonly animationFrame: number | null = null;
  private hoverParticipant: ParticipantPosition | null = null;
  private selectedCluster: OpinionCluster | null = null;

  // Viewport
  private readonly padding = 40;
  private readonly scale = 1;
  private readonly offsetX = 0;
  private readonly offsetY = 0;

  // Current user position
  currentUserPosition: ParticipantPosition | null = null;

  // Summary stats
  totalParticipants = 0;
  clusterCount = 0;
  consensusScore = 0;

  constructor(private readonly signalService: GovernanceSignalService) {}

  ngOnInit(): void {
    this.loadClusterData();
  }

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['votes'] || changes['statements']) {
      this.recalculateClusters();
    }
  }

  ngAfterViewInit(): void {
    this.initCanvas();
    this.render();
  }

  /**
   * Load cluster data from service.
   */
  private loadClusterData(): void {
    this.signalService.computeOpinionClusters(this.contextId).subscribe(clusters => {
      this.clusters = clusters;
      this.clusterCount = clusters.length;
      this.identifyConsensusAndDivisive();
      this.render();
    });
  }

  /**
   * Recalculate clusters based on new data.
   */
  private recalculateClusters(): void {
    // Simulate PCA reduction and clustering
    this.participants = this.computeParticipantPositions();
    this.clusters = this.computeClusters();
    this.identifyConsensusAndDivisive();
    this.updateStats();
    this.render();
  }

  /**
   * Compute 2D positions for participants based on their votes.
   * This simulates PCA - in production would use actual dimensionality reduction.
   */
  private computeParticipantPositions(): ParticipantPosition[] {
    if (this.statements.length === 0 || this.votes.length === 0) {
      return [];
    }

    // Group votes by participant
    const votesByParticipant = new Map<string, Map<string, number>>();

    for (const vote of this.votes) {
      if (!votesByParticipant.has(vote.participantId)) {
        votesByParticipant.set(vote.participantId, new Map());
      }
      votesByParticipant.get(vote.participantId)!.set(vote.statementId, vote.value);
    }

    // Simple 2D projection (simulating PCA)
    // In production: use actual PCA/t-SNE/UMAP
    const positions: ParticipantPosition[] = [];
    let index = 0;

    votesByParticipant.forEach((votes, participantId) => {
      // Create feature vector from votes
      const vector = this.statements.map(s => votes.get(s.id) ?? 0);

      // Project to 2D using simplified approach
      // Split statements into two groups for x/y axes
      const midpoint = Math.floor(this.statements.length / 2);
      const xStatements = this.statements.slice(0, midpoint);
      const yStatements = this.statements.slice(midpoint);

      const x =
        xStatements.reduce((sum, s, i) => {
          const v = votes.get(s.id) ?? 0;
          return sum + v * Math.cos((i / xStatements.length) * Math.PI);
        }, 0) / Math.max(xStatements.length, 1);

      const y =
        yStatements.reduce((sum, s, i) => {
          const v = votes.get(s.id) ?? 0;
          return sum + v * Math.sin((i / yStatements.length) * Math.PI);
        }, 0) / Math.max(yStatements.length, 1);

      // Normalize to [-1, 1] range
      const normalizedX = Math.max(-1, Math.min(1, x));
      const normalizedY = Math.max(-1, Math.min(1, y));

      positions.push({
        participantId,
        x: normalizedX,
        y: normalizedY,
        cluster: null, // Will be assigned
        isCurrentUser: participantId === 'current-user', // MVP: hardcoded
        voteCount: votes.size,
      });

      index++;
    });

    this.totalParticipants = positions.length;
    return positions;
  }

  /**
   * Compute clusters using k-means-like approach.
   */
  private computeClusters(): OpinionCluster[] {
    if (this.participants.length < 3) {
      return [];
    }

    // Simple clustering: divide space into quadrants + center
    const clusters: OpinionCluster[] = [
      {
        id: 'progressive',
        label: 'Progressive',
        color: '#3498db',
        centroid: [0.5, 0.5],
        memberCount: 0,
        averagePosition: 0.5,
      },
      {
        id: 'conservative',
        label: 'Traditionalist',
        color: '#9b59b6',
        centroid: [-0.5, 0.5],
        memberCount: 0,
        averagePosition: -0.5,
      },
      {
        id: 'pragmatic',
        label: 'Pragmatic',
        color: '#27ae60',
        centroid: [0, -0.5],
        memberCount: 0,
        averagePosition: 0,
      },
      {
        id: 'center',
        label: 'Centrist',
        color: '#f39c12',
        centroid: [0, 0],
        memberCount: 0,
        averagePosition: 0,
      },
    ];

    // Assign participants to nearest cluster
    for (const p of this.participants) {
      let nearestCluster = clusters[0];
      let minDist = Infinity;

      for (const cluster of clusters) {
        const dist = Math.sqrt(
          Math.pow(p.x - cluster.centroid[0], 2) + Math.pow(p.y - cluster.centroid[1], 2)
        );
        if (dist < minDist) {
          minDist = dist;
          nearestCluster = cluster;
        }
      }

      p.cluster = nearestCluster.id;
      nearestCluster.memberCount++;
    }

    // Filter empty clusters
    return clusters.filter(c => c.memberCount > 0);
  }

  /**
   * Identify consensus and divisive statements.
   */
  private identifyConsensusAndDivisive(): void {
    if (this.statements.length === 0) {
      this.consensusStatements = [];
      this.divisiveStatements = [];
      return;
    }

    const statementStats = this.statements.map(statement => {
      const statementVotes = this.votes.filter(v => v.statementId === statement.id);

      if (statementVotes.length === 0) {
        return { statement, agreement: 0, variance: 1 };
      }

      const values = statementVotes.map(v => v.value);
      const mean = values.reduce((a, b) => a + b, 0) / values.length;
      const variance = values.reduce((sum, v) => sum + Math.pow(v - mean, 2), 0) / values.length;

      return {
        statement,
        agreement: mean, // Positive = agreement, negative = disagreement
        variance, // Low variance = consensus, high = divisive
      };
    });

    // Sort by variance
    statementStats.sort((a, b) => a.variance - b.variance);

    // Top 20% lowest variance = consensus
    const consensusCount = Math.ceil(statementStats.length * 0.2);
    this.consensusStatements = statementStats.slice(0, consensusCount).map(s => s.statement);

    // Top 20% highest variance = divisive
    this.divisiveStatements = statementStats.slice(-consensusCount).map(s => s.statement);

    // Calculate overall consensus score (inverse of average variance)
    const avgVariance =
      statementStats.reduce((sum, s) => sum + s.variance, 0) / statementStats.length;
    this.consensusScore = Math.round((1 - Math.min(avgVariance, 1)) * 100);
  }

  /**
   * Update summary statistics.
   */
  private updateStats(): void {
    this.totalParticipants = this.participants.length;
    this.clusterCount = this.clusters.length;

    // Find current user
    this.currentUserPosition = this.participants.find(p => p.isCurrentUser) ?? null;
  }

  /**
   * Initialize canvas context.
   */
  private initCanvas(): void {
    const canvas = this.canvasRef?.nativeElement;
    if (!canvas) return;

    this.ctx = canvas.getContext('2d');

    // Handle resize
    this.resizeCanvas();
    window.addEventListener('resize', () => this.resizeCanvas());

    // Handle mouse events
    if (this.interactive) {
      canvas.addEventListener('mousemove', e => this.onMouseMove(e));
      canvas.addEventListener('click', e => this.onClick(e));
    }
  }

  /**
   * Resize canvas to container.
   */
  private resizeCanvas(): void {
    const canvas = this.canvasRef?.nativeElement;
    if (!canvas) return;

    const rect = canvas.parentElement?.getBoundingClientRect();
    if (rect) {
      canvas.width = rect.width;
      canvas.height = rect.height;
    }

    this.render();
  }

  /**
   * Main render function.
   */
  render(): void {
    if (!this.ctx) return;

    const canvas = this.canvasRef?.nativeElement;
    if (!canvas) return;

    const ctx = this.ctx;
    const width = canvas.width;
    const height = canvas.height;

    // Clear
    ctx.clearRect(0, 0, width, height);

    // Background
    ctx.fillStyle = getComputedStyle(canvas).getPropertyValue('--surface-secondary') || '#f5f5f5';
    ctx.fillRect(0, 0, width, height);

    // Draw grid
    this.drawGrid(ctx, width, height);

    // Draw clusters
    this.drawClusters(ctx, width, height);

    // Draw participants
    this.drawParticipants(ctx, width, height);

    // Draw axes labels
    if (this.showLabels) {
      this.drawLabels(ctx, width, height);
    }

    // Draw legend
    this.drawLegend(ctx, width, height);
  }

  /**
   * Draw background grid.
   */
  private drawGrid(ctx: CanvasRenderingContext2D, width: number, height: number): void {
    const centerX = width / 2;
    const centerY = height / 2;

    ctx.strokeStyle = 'rgba(0, 0, 0, 0.1)';
    ctx.lineWidth = 1;

    // Horizontal lines
    for (let y = this.padding; y <= height - this.padding; y += 40) {
      ctx.beginPath();
      ctx.moveTo(this.padding, y);
      ctx.lineTo(width - this.padding, y);
      ctx.stroke();
    }

    // Vertical lines
    for (let x = this.padding; x <= width - this.padding; x += 40) {
      ctx.beginPath();
      ctx.moveTo(x, this.padding);
      ctx.lineTo(x, height - this.padding);
      ctx.stroke();
    }

    // Center axes
    ctx.strokeStyle = 'rgba(0, 0, 0, 0.3)';
    ctx.lineWidth = 2;

    // Horizontal center
    ctx.beginPath();
    ctx.moveTo(this.padding, centerY);
    ctx.lineTo(width - this.padding, centerY);
    ctx.stroke();

    // Vertical center
    ctx.beginPath();
    ctx.moveTo(centerX, this.padding);
    ctx.lineTo(centerX, height - this.padding);
    ctx.stroke();
  }

  /**
   * Draw cluster regions.
   */
  private drawClusters(ctx: CanvasRenderingContext2D, width: number, height: number): void {
    for (const cluster of this.clusters) {
      // Draw cluster region as transparent circle
      const pos = this.toCanvasCoords(cluster.centroid[0], cluster.centroid[1], width, height);
      const radius = Math.sqrt(cluster.memberCount) * 20 + 30;

      ctx.beginPath();
      ctx.arc(pos.x, pos.y, radius, 0, Math.PI * 2);
      ctx.fillStyle = this.hexToRgba(cluster.color, 0.1);
      ctx.fill();
      ctx.strokeStyle = this.hexToRgba(cluster.color, 0.3);
      ctx.lineWidth = 2;
      ctx.stroke();

      // Cluster label
      if (this.showLabels) {
        ctx.font = '12px system-ui, sans-serif';
        ctx.fillStyle = cluster.color;
        ctx.textAlign = 'center';
        ctx.fillText(cluster.label, pos.x, pos.y - radius - 8);
        ctx.fillText(`(${cluster.memberCount})`, pos.x, pos.y - radius + 6);
      }
    }
  }

  /**
   * Draw participant points.
   */
  private drawParticipants(ctx: CanvasRenderingContext2D, width: number, height: number): void {
    for (const p of this.participants) {
      const pos = this.toCanvasCoords(p.x, p.y, width, height);
      const cluster = this.clusters.find(c => c.id === p.cluster);
      const color = cluster?.color ?? '#999';

      // Draw point
      ctx.beginPath();
      ctx.arc(pos.x, pos.y, p.isCurrentUser ? 10 : 6, 0, Math.PI * 2);

      if (p.isCurrentUser) {
        // Current user - highlighted
        ctx.fillStyle = '#2c3e50';
        ctx.fill();
        ctx.strokeStyle = '#f39c12';
        ctx.lineWidth = 3;
        ctx.stroke();
      } else if (p === this.hoverParticipant) {
        // Hovered
        ctx.fillStyle = color;
        ctx.fill();
        ctx.strokeStyle = '#fff';
        ctx.lineWidth = 2;
        ctx.stroke();
      } else {
        // Normal
        ctx.fillStyle = color;
        ctx.fill();
      }
    }

    // Draw "You are here" label for current user
    if (this.currentUserPosition) {
      const pos = this.toCanvasCoords(
        this.currentUserPosition.x,
        this.currentUserPosition.y,
        width,
        height
      );
      ctx.font = 'bold 11px system-ui, sans-serif';
      ctx.fillStyle = '#2c3e50';
      ctx.textAlign = 'center';
      ctx.fillText('You', pos.x, pos.y - 16);
    }
  }

  /**
   * Draw axis labels.
   */
  private drawLabels(ctx: CanvasRenderingContext2D, width: number, height: number): void {
    ctx.font = '11px system-ui, sans-serif';
    ctx.fillStyle = '#666';

    // X-axis labels
    ctx.textAlign = 'center';
    ctx.fillText('← More Traditional', this.padding + 60, height - 10);
    ctx.fillText('More Progressive →', width - this.padding - 60, height - 10);

    // Y-axis labels
    ctx.save();
    ctx.translate(12, height / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText('← More Pragmatic', 60, 0);
    ctx.fillText('More Idealistic →', -60, 0);
    ctx.restore();
  }

  /**
   * Draw legend.
   */
  private drawLegend(ctx: CanvasRenderingContext2D, width: number, height: number): void {
    const legendX = width - 120;
    const legendY = this.padding;

    ctx.font = 'bold 11px system-ui, sans-serif';
    ctx.fillStyle = '#333';
    ctx.textAlign = 'left';
    ctx.fillText('Opinion Groups', legendX, legendY);

    let y = legendY + 18;
    for (const cluster of this.clusters) {
      // Color dot
      ctx.beginPath();
      ctx.arc(legendX + 6, y - 4, 5, 0, Math.PI * 2);
      ctx.fillStyle = cluster.color;
      ctx.fill();

      // Label
      ctx.font = '10px system-ui, sans-serif';
      ctx.fillStyle = '#666';
      ctx.fillText(`${cluster.label} (${cluster.memberCount})`, legendX + 16, y);

      y += 16;
    }
  }

  /**
   * Convert data coordinates to canvas coordinates.
   */
  private toCanvasCoords(
    x: number,
    y: number,
    width: number,
    height: number
  ): { x: number; y: number } {
    const usableWidth = width - this.padding * 2;
    const usableHeight = height - this.padding * 2;

    return {
      x: this.padding + ((x + 1) / 2) * usableWidth,
      y: this.padding + ((1 - y) / 2) * usableHeight, // Flip Y axis
    };
  }

  /**
   * Convert canvas coordinates to data coordinates.
   */
  private toDataCoords(
    canvasX: number,
    canvasY: number,
    width: number,
    height: number
  ): { x: number; y: number } {
    const usableWidth = width - this.padding * 2;
    const usableHeight = height - this.padding * 2;

    return {
      x: ((canvasX - this.padding) / usableWidth) * 2 - 1,
      y: 1 - ((canvasY - this.padding) / usableHeight) * 2,
    };
  }

  /**
   * Handle mouse move for hover effects.
   */
  private onMouseMove(e: MouseEvent): void {
    const canvas = this.canvasRef?.nativeElement;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    // Find participant under cursor
    let found: ParticipantPosition | null = null;
    for (const p of this.participants) {
      const pos = this.toCanvasCoords(p.x, p.y, canvas.width, canvas.height);
      const dist = Math.sqrt(Math.pow(x - pos.x, 2) + Math.pow(y - pos.y, 2));
      if (dist < 12) {
        found = p;
        break;
      }
    }

    if (found !== this.hoverParticipant) {
      this.hoverParticipant = found;
      canvas.style.cursor = found ? 'pointer' : 'default';
      this.render();
    }
  }

  /**
   * Handle click for selection.
   */
  private onClick(e: MouseEvent): void {
    const canvas = this.canvasRef?.nativeElement;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    // Check if clicked on a cluster
    for (const cluster of this.clusters) {
      const pos = this.toCanvasCoords(
        cluster.centroid[0],
        cluster.centroid[1],
        canvas.width,
        canvas.height
      );
      const radius = Math.sqrt(cluster.memberCount) * 20 + 30;
      const dist = Math.sqrt(Math.pow(x - pos.x, 2) + Math.pow(y - pos.y, 2));

      if (dist < radius) {
        this.selectedCluster = cluster;
        this.clusterSelected.emit(cluster);
        this.render();
        return;
      }
    }
  }

  /**
   * Helper: Convert hex color to rgba.
   */
  private hexToRgba(hex: string, alpha: number): string {
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
  }

  /**
   * Select a statement to highlight related participants.
   */
  selectStatement(statement: Statement): void {
    this.statementSelected.emit(statement);
    // Could highlight participants who voted on this statement
    this.render();
  }

  /**
   * Get statement classification.
   */
  getStatementType(statement: Statement): 'consensus' | 'divisive' | 'neutral' {
    if (this.consensusStatements.includes(statement)) return 'consensus';
    if (this.divisiveStatements.includes(statement)) return 'divisive';
    return 'neutral';
  }

  /**
   * Get the cluster for the current user.
   */
  getUserCluster(): OpinionCluster | undefined {
    if (!this.currentUserPosition?.cluster) return undefined;
    return this.clusters.find(c => c.id === this.currentUserPosition!.cluster);
  }

  /**
   * Get the color of the current user's cluster.
   */
  getUserClusterColor(): string {
    return this.getUserCluster()?.color ?? 'transparent';
  }

  /**
   * Get the label of the current user's cluster.
   */
  getUserClusterLabel(): string {
    return this.getUserCluster()?.label ?? '';
  }
}

// ===========================================================================
// Types
// ===========================================================================

export interface Statement {
  id: string;
  text: string;
  author?: string;
  createdAt?: Date;
}

export interface StatementVote {
  participantId: string;
  statementId: string;
  value: number; // -1 (disagree), 0 (pass), 1 (agree)
  timestamp?: Date;
}

interface ParticipantPosition {
  participantId: string;
  x: number;
  y: number;
  cluster: string | null;
  isCurrentUser: boolean;
  voteCount: number;
}
