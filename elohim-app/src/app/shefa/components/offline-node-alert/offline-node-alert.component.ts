/**
 * OfflineNodeAlert Component
 *
 * Shefa-specific wrapper around the generic AlertBanner component.
 * Transforms node topology alerts into generic AlertData format.
 *
 * Usage:
 * <app-offline-node-alert
 *   [operatorId]="operatorId"
 *   [displayMode]="'banner'"
 *   (viewDashboard)="goToDashboard()">
 * </app-offline-node-alert>
 */

import { CommonModule } from '@angular/common';
import { Component, Input, Output, EventEmitter, OnInit, OnDestroy } from '@angular/core';
import { Router } from '@angular/router';

// @coverage: 44.8% (2026-02-05)

import { Subject, takeUntil } from 'rxjs';

import {
  AlertBannerComponent,
  AlertData,
  AlertAction,
  AlertSeverity,
} from '../../../shared/components/alert-banner/alert-banner.component';
import {
  OfflineNodeAlert,
  NodeTopologyState,
  ComputeNeedsAssessment,
} from '../../models/shefa-dashboard.model';
import { ShefaComputeService } from '../../services/shefa-compute.service';

@Component({
  selector: 'app-offline-node-alert',
  standalone: true,
  imports: [CommonModule, AlertBannerComponent],
  template: `
    <app-alert-banner
      *ngIf="!isLoading"
      [alerts]="alertData"
      [displayMode]="displayMode"
      [maxVisible]="maxAlerts"
      [expandable]="true"
      (dismissed)="onAlertDismissed($event)"
      (actionClicked)="onActionClicked($event)"
    ></app-alert-banner>

    <div class="loading-indicator" *ngIf="isLoading">
      <span class="spinner"></span>
      Checking node status...
    </div>
  `,
  styles: [
    `
      .loading-indicator {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.5rem 1rem;
        font-size: 0.85rem;
        color: #6b7280;
      }

      .spinner {
        width: 1rem;
        height: 1rem;
        border: 2px solid #e5e7eb;
        border-top-color: #3b82f6;
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
      }

      @keyframes spin {
        to {
          transform: rotate(360deg);
        }
      }
    `,
  ],
})
export class OfflineNodeAlertComponent implements OnInit, OnDestroy {
  /** Operator ID to load alerts for */
  @Input() operatorId!: string;

  /** Display mode: 'banner' for header, 'card' for dashboard */
  @Input() displayMode: 'banner' | 'card' | 'toast' | 'inline' = 'banner';

  /** Maximum alerts to show */
  @Input() maxAlerts = 3;

  /** Emits when user clicks to view compute dashboard */
  @Output() viewDashboard = new EventEmitter<void>();

  /** Emits when user clicks help-flow link */
  @Output() viewHelpFlow = new EventEmitter<ComputeNeedsAssessment>();

  // State
  alertData: AlertData[] = [];
  topology: NodeTopologyState | null = null;
  computeNeeds: ComputeNeedsAssessment | null = null;
  isLoading = true;

  private readonly destroy$ = new Subject<void>();
  private rawAlerts: OfflineNodeAlert[] = [];

  constructor(
    private readonly shefaCompute: ShefaComputeService,
    private readonly router: Router
  ) {}

  ngOnInit(): void {
    if (!this.operatorId) {
      this.isLoading = false;
      return;
    }

    this.loadAlerts();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  /**
   * Load node topology and transform to generic alerts
   */
  private loadAlerts(): void {
    // Get node topology
    this.shefaCompute
      .getNodeTopology(this.operatorId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: topology => {
          this.topology = topology;
          this.rawAlerts = topology.alerts;
          this.updateAlertData();
          this.isLoading = false;
        },
        error: _err => {
          this.isLoading = false;
        },
      });

    // Get compute needs for help-flow actions
    this.shefaCompute
      .getComputeNeedsAssessment(this.operatorId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: needs => {
          this.computeNeeds = needs;
          this.updateAlertData();
        },
        error: _err => {
          // Silently fail for compute needs assessment - alerts can be shown without help-flow actions
        },
      });
  }

  /**
   * Transform Shefa alerts to generic AlertData
   */
  private updateAlertData(): void {
    this.alertData = this.rawAlerts
      .filter(a => !a.dismissedAt)
      .map(alert => this.transformToAlertData(alert));
  }

  /**
   * Transform a single OfflineNodeAlert to AlertData
   */
  private transformToAlertData(alert: OfflineNodeAlert): AlertData {
    const severity = this.mapSeverity(alert);
    const actions = this.buildActions(alert);

    return {
      id: alert.id,
      severity,
      title: alert.isPrimaryNode
        ? `Primary node "${alert.nodeName}" is offline`
        : `Node "${alert.nodeName}" is ${alert.eventType.replace('-', ' ')}`,
      message: alert.message,
      subtitle: `Offline for ${alert.offlineDuration}`,
      icon: this.getAlertIcon(alert.isPrimaryNode, severity),
      timestamp: alert.detectedAt,
      duration: alert.offlineDuration,
      actions,
      dismissible: true,
      metadata: {
        nodeId: alert.nodeId,
        isPrimary: alert.isPrimaryNode,
        impact: alert.impact,
        recommendedActions: alert.recommendedActions,
      },
    };
  }

  /**
   * Map Shefa severity to generic severity
   */
  private mapSeverity(alert: OfflineNodeAlert): AlertSeverity {
    if (alert.isPrimaryNode) return 'critical';
    switch (alert.severity) {
      case 'critical':
        return 'critical';
      case 'warning':
        return 'warning';
      default:
        return 'info';
    }
  }

  /**
   * Get alert icon based on node priority and severity
   */
  private getAlertIcon(isPrimaryNode: boolean, severity: AlertSeverity): string {
    if (isPrimaryNode) return '🚨';
    if (severity === 'critical') return '⚠️';
    return '⚡';
  }

  /**
   * Build action buttons for an alert
   */
  private buildActions(_alert: OfflineNodeAlert): AlertAction[] {
    const actions: AlertAction[] = [
      {
        id: 'view-dashboard',
        label: 'View Dashboard',
        variant: 'secondary',
      },
    ];

    if (this.computeNeeds?.hasGaps) {
      actions.push({
        id: 'help-flow',
        label: this.computeNeeds.helpFlowCTA ?? 'Get Help',
        variant: 'primary',
      });
    }

    return actions;
  }

  /**
   * Handle alert dismissal
   */
  onAlertDismissed(alert: AlertData): void {
    // Mark original alert as dismissed
    const original = this.rawAlerts.find(a => a.id === alert.id);
    if (original) {
      original.dismissedAt = new Date().toISOString();
    }
    this.updateAlertData();
  }

  /**
   * Handle action button clicks
   */
  onActionClicked(event: { alert: AlertData; action: AlertAction }): void {
    switch (event.action.id) {
      case 'view-dashboard':
        this.viewDashboard.emit();
        void this.router.navigate(['/shefa/dashboard']);
        break;

      case 'help-flow':
        if (this.computeNeeds) {
          this.viewHelpFlow.emit(this.computeNeeds);
        }
        void this.router.navigate(['/shefa/help-flow/compute-needs']);
        break;
    }
  }
}
