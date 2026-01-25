/**
 * Compute Needs Component
 *
 * Help-flow component that:
 * - Assesses compute gaps
 * - Shows what's missing (CPU, storage, redundancy)
 * - Recommends nodes to order
 * - Links to Holoport ordering
 *
 * This guides users from "I have a problem" to "here's how to fix it"
 */

import { CommonModule } from '@angular/common';
import { Component, Input, Output, EventEmitter, OnInit, OnDestroy } from '@angular/core';

import { Subject, takeUntil } from 'rxjs';

import {
  ComputeNeedsAssessment,
  ComputeGap,
  NodeRecommendation,
} from '../../models/shefa-dashboard.model';
import { ShefaComputeService } from '../../services/shefa-compute.service';

@Component({
  selector: 'app-compute-needs',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './compute-needs.component.html',
  styleUrls: ['./compute-needs.component.scss'],
})
export class ComputeNeedsComponent implements OnInit, OnDestroy {
  /** Operator ID to assess compute needs for */
  @Input() operatorId!: string;

  /** Display mode: 'card' for compact, 'page' for full help-flow */
  @Input() displayMode: 'card' | 'page' = 'page';

  /** Emits when user clicks to order a node */
  @Output() orderNode = new EventEmitter<NodeRecommendation>();

  /** Emits when assessment is complete */
  @Output() assessmentComplete = new EventEmitter<ComputeNeedsAssessment>();

  // State
  assessment: ComputeNeedsAssessment | null = null;
  isLoading = true;
  error: string | null = null;
  selectedRecommendation: NodeRecommendation | null = null;

  private destroy$ = new Subject<void>();

  constructor(private shefaCompute: ShefaComputeService) {}

  ngOnInit(): void {
    if (!this.operatorId) {
      this.error = 'No operator ID provided';
      this.isLoading = false;
      return;
    }

    this.loadAssessment();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  /**
   * Load compute needs assessment
   */
  private loadAssessment(): void {
    this.shefaCompute
      .getComputeNeedsAssessment(this.operatorId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: assessment => {
          this.assessment = assessment;
          this.isLoading = false;
          this.assessmentComplete.emit(assessment);
        },
        error: err => {
          console.error('[ComputeNeeds] Failed to load:', err);
          this.error = 'Failed to assess compute needs';
          this.isLoading = false;
        },
      });
  }

  /**
   * Select a recommendation for more details
   */
  selectRecommendation(rec: NodeRecommendation): void {
    this.selectedRecommendation = this.selectedRecommendation === rec ? null : rec;
  }

  /**
   * Order a node
   */
  onOrderNode(rec: NodeRecommendation): void {
    this.orderNode.emit(rec);

    // If there's an order URL, open it
    if (rec.orderUrl) {
      window.open(rec.orderUrl, '_blank');
    }
  }

  /**
   * Get severity class for a gap
   */
  getSeverityClass(severity: string): string {
    return `severity-${severity}`;
  }

  /**
   * Get icon for a gap type
   */
  getGapIcon(resource: string): string {
    const icons: Record<string, string> = {
      cpu: '🖥️',
      memory: '💾',
      storage: '📦',
      bandwidth: '📡',
      redundancy: '🔄',
    };
    return icons[resource] || '⚙️';
  }

  /**
   * Get icon for severity
   */
  getSeverityIcon(severity: string): string {
    switch (severity) {
      case 'critical':
        return '🚨';
      case 'moderate':
        return '⚠️';
      default:
        return 'ℹ️';
    }
  }

  /**
   * Get icon for node type
   */
  getNodeIcon(type: string): string {
    const icons: Record<string, string> = {
      holoport: '🖥️',
      'holoport-plus': '🖥️✨',
      'holoport-nano': '📦',
      'self-hosted': '🏠',
      cloud: '☁️',
    };
    return icons[type] || '🖥️';
  }

  /**
   * Get priority class
   */
  getPriorityClass(priority: string): string {
    return `priority-${priority}`;
  }

  /**
   * Format cost
   */
  formatCost(rec: NodeRecommendation): string {
    if (!rec.estimatedCost) return 'Contact for pricing';

    const { value, currency, period } = rec.estimatedCost;
    let cost = `${currency === 'USD' ? '$' : currency}${value}`;

    if (period && period !== 'one-time') {
      cost += `/${period.replace('ly', '')}`;
    }

    return cost;
  }

  /**
   * Get overall health status text
   */
  get healthStatus(): string {
    if (!this.assessment) return 'Unknown';
    switch (this.assessment.overallGapSeverity) {
      case 'none':
        return 'Healthy';
      case 'minor':
        return 'Minor Issues';
      case 'moderate':
        return 'Needs Attention';
      case 'critical':
        return 'Critical';
      default:
        return 'Unknown';
    }
  }

  /**
   * Get health status class
   */
  get healthStatusClass(): string {
    return `health-${this.assessment?.overallGapSeverity || 'unknown'}`;
  }
}
