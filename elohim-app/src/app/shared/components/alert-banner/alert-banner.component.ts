/**
 * Generic Alert Banner Component
 *
 * Reusable banner/card component for any type of notification:
 * - Success messages
 * - Warning alerts
 * - Error notifications
 * - Info banners
 * - Custom alerts with actions
 *
 * Usage:
 * <app-alert-banner
 *   [alert]="{ severity: 'warning', title: 'Node Offline', message: '...' }"
 *   [displayMode]="'banner'"
 *   (dismissed)="onDismiss($event)"
 *   (actionClicked)="onAction($event)">
 * </app-alert-banner>
 */

import { Component, Input, Output, EventEmitter, TemplateRef } from '@angular/core';
import { CommonModule } from '@angular/common';

/**
 * Alert severity levels
 */
export type AlertSeverity = 'success' | 'info' | 'warning' | 'error' | 'critical';

/**
 * Alert action button
 */
export interface AlertAction {
  id: string;
  label: string;
  variant?: 'primary' | 'secondary' | 'text';
  icon?: string;
  href?: string;
  ariaLabel?: string;
}

/**
 * Generic alert data structure
 */
export interface AlertData {
  id?: string;
  severity: AlertSeverity;
  title: string;
  message?: string;
  subtitle?: string;

  // Optional metadata for context
  icon?: string;
  timestamp?: string;
  duration?: string;

  // Actions
  actions?: AlertAction[];
  dismissible?: boolean;

  // Additional data for custom rendering
  metadata?: Record<string, any>;
}

@Component({
  selector: 'app-alert-banner',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './alert-banner.component.html',
  styleUrls: ['./alert-banner.component.scss'],
})
export class AlertBannerComponent {
  /**
   * Single alert to display
   */
  @Input() alert?: AlertData;

  /**
   * Multiple alerts to display
   */
  @Input() alerts: AlertData[] = [];

  /**
   * Display mode: 'banner' for header, 'card' for dashboard, 'toast' for popup
   */
  @Input() displayMode: 'banner' | 'card' | 'toast' | 'inline' = 'banner';

  /**
   * Whether to show expanded view by default
   */
  @Input() expanded = false;

  /**
   * Whether the banner is expandable (for multiple alerts)
   */
  @Input() expandable = true;

  /**
   * Maximum alerts to show before "show more" (0 = all)
   */
  @Input() maxVisible = 3;

  /**
   * Custom icon template
   */
  @Input() iconTemplate?: TemplateRef<any>;

  /**
   * Custom content template
   */
  @Input() contentTemplate?: TemplateRef<any>;

  /**
   * Custom actions template
   */
  @Input() actionsTemplate?: TemplateRef<any>;

  /**
   * Auto-dismiss after milliseconds (0 = no auto-dismiss)
   */
  @Input() autoDismissMs = 0;

  /**
   * Emits when an alert is dismissed
   */
  @Output() dismissed = new EventEmitter<AlertData>();

  /**
   * Emits when an action button is clicked
   */
  @Output() actionClicked = new EventEmitter<{ alert: AlertData; action: AlertAction }>();

  /**
   * Emits when banner is expanded/collapsed
   */
  @Output() expandedChange = new EventEmitter<boolean>();

  // Internal state
  isExpanded = false;
  dismissedAlerts: Set<string> = new Set();

  ngOnInit(): void {
    this.isExpanded = this.expanded;

    // Handle auto-dismiss
    if (this.autoDismissMs > 0 && this.alert) {
      setTimeout(() => {
        this.dismissAlert(this.alert!);
      }, this.autoDismissMs);
    }
  }

  /**
   * Get all active alerts (single or multiple)
   */
  get activeAlerts(): AlertData[] {
    let all: AlertData[] = [];

    if (this.alert) {
      all = [this.alert];
    } else if (this.alerts.length > 0) {
      all = this.alerts;
    }

    // Filter out dismissed alerts
    return all.filter(a => !this.dismissedAlerts.has(a.id || a.title));
  }

  /**
   * Get visible alerts (respects maxVisible)
   */
  get visibleAlerts(): AlertData[] {
    if (this.maxVisible <= 0 || this.isExpanded) {
      return this.activeAlerts;
    }
    return this.activeAlerts.slice(0, this.maxVisible);
  }

  /**
   * Get the primary (first/most severe) alert
   */
  get primaryAlert(): AlertData | null {
    const sorted = [...this.activeAlerts].sort((a, b) => {
      const order: Record<AlertSeverity, number> = {
        critical: 0,
        error: 1,
        warning: 2,
        info: 3,
        success: 4,
      };
      return (order[a.severity] ?? 5) - (order[b.severity] ?? 5);
    });
    return sorted[0] || null;
  }

  /**
   * Check if there are any active alerts
   */
  get hasAlerts(): boolean {
    return this.activeAlerts.length > 0;
  }

  /**
   * Check if there are hidden alerts
   */
  get hasHiddenAlerts(): boolean {
    return this.maxVisible > 0 && this.activeAlerts.length > this.maxVisible && !this.isExpanded;
  }

  /**
   * Get count of hidden alerts
   */
  get hiddenCount(): number {
    if (this.maxVisible <= 0 || this.isExpanded) return 0;
    return Math.max(0, this.activeAlerts.length - this.maxVisible);
  }

  /**
   * Get CSS class for severity
   */
  getSeverityClass(severity: AlertSeverity): string {
    return `severity-${severity}`;
  }

  /**
   * Get default icon for severity
   */
  getDefaultIcon(severity: AlertSeverity): string {
    const icons: Record<AlertSeverity, string> = {
      success: '✓',
      info: 'ℹ',
      warning: '⚠',
      error: '✕',
      critical: '🚨',
    };
    return icons[severity] || 'ℹ';
  }

  /**
   * Get icon for an alert
   */
  getIcon(alert: AlertData): string {
    return alert.icon || this.getDefaultIcon(alert.severity);
  }

  /**
   * Toggle expanded state
   */
  toggleExpanded(): void {
    if (!this.expandable) return;
    this.isExpanded = !this.isExpanded;
    this.expandedChange.emit(this.isExpanded);
  }

  /**
   * Dismiss an alert
   */
  dismissAlert(alert: AlertData, event?: Event): void {
    event?.stopPropagation();

    const id = alert.id || alert.title;
    this.dismissedAlerts.add(id);
    this.dismissed.emit(alert);
  }

  /**
   * Handle action click
   */
  onActionClick(alert: AlertData, action: AlertAction, event: Event): void {
    event.stopPropagation();

    // If it's a link, let browser handle it
    if (action.href) {
      return;
    }

    this.actionClicked.emit({ alert, action });
  }

  /**
   * Get action variant class
   */
  getActionClass(action: AlertAction): string {
    return `btn-${action.variant || 'secondary'}`;
  }
}
