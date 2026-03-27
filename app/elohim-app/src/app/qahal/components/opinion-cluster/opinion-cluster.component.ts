import { CommonModule } from '@angular/common';
import {
  AfterViewInit,
  Component,
  ElementRef,
  EventEmitter,
  Input,
  OnChanges,
  OnInit,
  Output,
  SimpleChanges,
  ViewChild,
  inject,
} from '@angular/core';

import { from } from 'rxjs';

// @coverage: 80.0% (2026-02-24)

import { GovernanceApiService } from '@app/elohim/services/governance-api.service';
import {
  GovernanceSignalService,
  OpinionCluster,
} from '@app/elohim/services/governance-signal.service';

import type {
  StatementView,
  StatementVoteView,
  OpinionClusterView,
  ParticipantPositionView,
  SensemakingResultView,
  StatementMetricsView,
} from '@elohim/storage-client/generated';

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
  @Input() entityType?: string;
  @Input() entityId?: string;
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
  protected selectedCluster: OpinionCluster | null = null;

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

  private readonly signalService = inject(GovernanceSignalService);
  private readonly governanceApi = inject(GovernanceApiService);

  ngOnInit(): void {
    // If entityType/entityId are provided and no statements/votes were passed in,
    // load data from the sensemaking backend
    if (
      this.entityType &&
      this.entityId &&
      this.statements.length === 0 &&
      this.votes.length === 0
    ) {
      this.loadFromBackend(this.entityType, this.entityId);
    } else {
      this.loadClusterData();
    }
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
   * Load sensemaking data from the governance backend.
   *
   * The backend computes real PCA projections and agglomerative clustering,
   * returning participant 2D positions, cluster assignments, bridging/divisive
   * statements, and per-statement metrics. We use these directly rather than
   * running a local approximation.
   */
  private loadFromBackend(entityType: string, entityId: string): void {
    from(this.governanceApi.getClusters(entityType, entityId)).subscribe(
      (result: SensemakingResultView) => {
        // Map backend clusters to component's OpinionCluster type
        this.clusters = result.clusters.map(mapOpinionClusterViewToCluster);
        this.clusterCount = this.clusters.length;
        this.totalParticipants = result.totalParticipants;

        // Use PCA-projected positions from backend
        this.participants = result.participantPositions.map(p =>
          mapParticipantPositionToLocal(p)
        );

        // Also load statements/votes for the statement list display
        this.statements = result.bridgingStatements
          .concat(result.divisiveStatements)
          .map(mapStatementViewToStatement);

        // Use backend classifications for consensus/divisive
        this.consensusStatements = result.bridgingStatements.map(mapStatementViewToStatement);
        this.divisiveStatements = (result.divisiveStatements ?? []).map(
          mapStatementViewToStatement
        );

        this.updateStats();
        this.render();
      }
    );
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
   *
   * Local fallback used when statements/votes are passed via @Input()
   * rather than loaded from the backend (which computes real PCA).
   * Uses mean vote value as a simple 1D projection.
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

    const positions: ParticipantPosition[] = [];
    votesByParticipant.forEach((votes, participantId) => {
      // Simple projection: mean agreement as x, vote diversity as y
      const values = [...votes.values()];
      const mean = values.reduce((a, b) => a + b, 0) / Math.max(values.length, 1);
      const variance =
        values.reduce((sum, v) => sum + (v - mean) ** 2, 0) / Math.max(values.length, 1);

      positions.push({
        participantId,
        x: Math.max(-1, Math.min(1, mean)),
        y: Math.max(-1, Math.min(1, Math.sqrt(variance) * 2 - 1)),
        cluster: null,
        isCurrentUser: participantId === 'current-user',
        voteCount: votes.size,
      });
    });

    this.totalParticipants = positions.length;
    return positions;
  }

  /**
   * Compute clusters from participant positions using simple k-means.
   *
   * Local fallback when backend clusters aren't available. Dynamically
   * determines cluster count rather than using hardcoded quadrants.
   */
  private computeClusters(): OpinionCluster[] {
    if (this.participants.length < 3) {
      return [];
    }

    // Determine k: 2-4 clusters based on data spread
    const k = Math.min(4, Math.max(2, Math.ceil(this.participants.length / 5)));

    // Initialize centroids using k-means++ style: spread evenly
    const centroids: [number, number][] = [];
    for (let i = 0; i < k; i++) {
      const angle = (i / k) * Math.PI * 2;
      centroids.push([Math.cos(angle) * 0.5, Math.sin(angle) * 0.5]);
    }

    // Run k-means for a few iterations
    for (let iter = 0; iter < 10; iter++) {
      // Assign each participant to nearest centroid
      const assignments = this.participants.map(p => {
        let best = 0;
        let bestDist = Infinity;
        for (let c = 0; c < k; c++) {
          const dist = (p.x - centroids[c][0]) ** 2 + (p.y - centroids[c][1]) ** 2;
          if (dist < bestDist) {
            bestDist = dist;
            best = c;
          }
        }
        return best;
      });

      // Update centroids
      for (let c = 0; c < k; c++) {
        const members = this.participants.filter((_, i) => assignments[i] === c);
        if (members.length > 0) {
          centroids[c] = [
            members.reduce((s, p) => s + p.x, 0) / members.length,
            members.reduce((s, p) => s + p.y, 0) / members.length,
          ];
        }
      }

      // Final iteration: apply assignments
      if (iter === 9) {
        this.participants.forEach((p, i) => {
          p.cluster = `cluster-${assignments[i]}`;
        });
      }
    }

    // Build cluster objects
    const clusters: OpinionCluster[] = [];
    for (let c = 0; c < k; c++) {
      const members = this.participants.filter(p => p.cluster === `cluster-${c}`);
      if (members.length > 0) {
        clusters.push({
          id: `cluster-${c}`,
          label: `Group ${c + 1}`,
          color: CLUSTER_COLORS[c % CLUSTER_COLORS.length],
          centroid: centroids[c],
          memberCount: members.length,
          averagePosition: centroids[c][0],
        });
      }
    }

    return clusters;
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

    // X-axis labels (PCA axes represent emergent opinion dimensions, not political labels)
    ctx.textAlign = 'center';
    ctx.fillText('← Opinion Axis 1', this.padding + 60, height - 10);
    ctx.fillText('Opinion Axis 1 →', width - this.padding - 60, height - 10);

    // Y-axis labels
    ctx.save();
    ctx.translate(12, height / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText('← Opinion Axis 2', 60, 0);
    ctx.fillText('Opinion Axis 2 →', -60, 0);
    ctx.restore();
  }

  /**
   * Draw legend.
   */
  private drawLegend(ctx: CanvasRenderingContext2D, width: number, _height: number): void {
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
    const r = Number.parseInt(hex.slice(1, 3), 16);
    const g = Number.parseInt(hex.slice(3, 5), 16);
    const b = Number.parseInt(hex.slice(5, 7), 16);
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

// ===========================================================================
// Backend Type Mapping Functions
// ===========================================================================

/** Palette for dynamically assigning colors to backend clusters. */
const CLUSTER_COLORS = ['#3498db', '#9b59b6', '#27ae60', '#f39c12', '#e74c3c', '#1abc9c'];

/** Map a StatementView from the storage-client to the component's Statement type. */
function mapStatementViewToStatement(sv: StatementView): Statement {
  return {
    id: sv.id,
    text: sv.text,
    author: sv.humanId,
    createdAt: sv.createdAt ? new Date(sv.createdAt) : undefined,
  };
}

/**
 * Map a StatementVoteView from the storage-client to the component's StatementVote type.
 * The backend stores vote as a string ('agree'|'disagree'|'pass');
 * the component expects a numeric value: 1 (agree), -1 (disagree), 0 (pass).
 */
function mapStatementVoteViewToVote(vv: StatementVoteView): StatementVote {
  const VOTE_VALUE_MAP: Record<string, number> = {
    agree: 1,
    disagree: -1,
    pass: 0,
  };
  return {
    participantId: vv.humanId,
    statementId: vv.statementId,
    value: VOTE_VALUE_MAP[vv.vote] ?? 0,
    timestamp: vv.createdAt ? new Date(vv.createdAt) : undefined,
  };
}

/** Map an OpinionClusterView from the storage-client to the component's OpinionCluster type. */
function mapOpinionClusterViewToCluster(cv: OpinionClusterView, index: number): OpinionCluster {
  return {
    id: cv.id,
    label: `Group ${index + 1}`,
    memberCount: cv.memberCount,
    centroid: [
      // Spread clusters in a circle when no explicit centroid is provided
      Math.cos((index / Math.max(1, 4)) * Math.PI * 2) * 0.5,
      Math.sin((index / Math.max(1, 4)) * Math.PI * 2) * 0.5,
    ],
    averagePosition: cv.internalAgreement,
    color: CLUSTER_COLORS[index % CLUSTER_COLORS.length],
  };
}

/**
 * Map a ParticipantPositionView (PCA-projected by the backend) to the
 * component's local ParticipantPosition type.
 */
function mapParticipantPositionToLocal(
  pp: ParticipantPositionView,
): ParticipantPosition {
  return {
    participantId: pp.humanId,
    x: pp.x,
    y: pp.y,
    cluster: pp.clusterId,
    isCurrentUser: false, // Resolved by updateStats after session context is available
    voteCount: 0, // Not tracked in position view; available from votes if needed
  };
}
