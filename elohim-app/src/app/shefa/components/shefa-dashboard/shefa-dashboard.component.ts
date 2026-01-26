/**
 * Shefa Dashboard Component
 *
 * Main dashboard providing operator visibility into:
 * - Node compute resources (CPU, memory, storage, network)
 * - Digital footprint replication (where data is protected)
 * - Family-community protection (who's protecting my data)
 * - Infrastructure-token economics (how much compute I'm earning)
 * - Constitutional limits enforcement (dignity floor/ceiling)
 * - Recent compute events (economic flows)
 *
 * "Shefa" = overflow, abundance in Hebrew
 * Represents the abundance of resources flowing through the network
 */

import { Component, OnInit, OnDestroy, Input } from '@angular/core';

import { takeUntil, tap, debounceTime } from 'rxjs/operators';

import { Observable, Subject, combineLatest } from 'rxjs';

import { SheafaDashboardState } from '../../models/shefa-dashboard.model';
import { ComputeEventService } from '../../services/compute-event.service';
import { FamilyCommunityProtectionService } from '../../services/family-community-protection.service';
import { ShefaComputeService } from '../../services/shefa-compute.service';

/**
 * Display configuration for dashboard
 */
interface DashboardConfig {
  displayMode: 'compact' | 'detailed' | 'monitoring';
  refreshInterval: number; // Milliseconds
  chartTimescale: '1h' | '24h' | '7d' | '30d';
  alertThreshold: 'high' | 'medium' | 'low';
  enableNotifications: boolean;
  visiblePanels: {
    computeMetrics: boolean;
    familyProtection: boolean;
    tokenEarnings: boolean;
    economicEvents: boolean;
    constitutionalLimits: boolean;
  };
}

const DEFAULT_CONFIG: DashboardConfig = {
  displayMode: 'detailed',
  refreshInterval: 5000,
  chartTimescale: '24h',
  alertThreshold: 'medium',
  enableNotifications: true,
  visiblePanels: {
    computeMetrics: true,
    familyProtection: true,
    tokenEarnings: true,
    economicEvents: true,
    constitutionalLimits: true,
  },
};

@Component({
  selector: 'app-shefa-dashboard',
  templateUrl: './shefa-dashboard.component.html',
  styleUrls: ['./shefa-dashboard.component.scss'],
})
export class ShefaDashboardComponent implements OnInit, OnDestroy {
  // Required inputs
  @Input() operatorId!: string;
  @Input() stewardedResourceId!: string;

  // Optional configuration override
  @Input() config: Partial<DashboardConfig> = {};

  // Dashboard state observable
  dashboardState$: Observable<SheafaDashboardState | null> | null = null;

  // Current state (for template access)
  currentState: SheafaDashboardState | null = null;

  // UI state
  isLoading = true;
  lastUpdateTime: Date | null = null;
  selectedPanel: 'compute' | 'protection' | 'tokens' | 'events' | 'limits' = 'compute';
  showConfigPanel = false;

  // Computed config
  mergedConfig: DashboardConfig;

  // Cleanup
  private readonly destroy$ = new Subject<void>();

  constructor(
    private readonly shefaCompute: ShefaComputeService,
    private readonly familyProtection: FamilyCommunityProtectionService,
    private readonly computeEvents: ComputeEventService
  ) {
    this.mergedConfig = { ...DEFAULT_CONFIG, ...this.config };
  }

  ngOnInit(): void {
    if (!this.operatorId || !this.stewardedResourceId) {
      console.error('[ShefaDashboard] Missing required inputs: operatorId or stewardedResourceId');
      return;
    }

    // Initialize dashboard
    this.dashboardState$ = this.shefaCompute
      .initializeDashboard(this.operatorId, this.stewardedResourceId)
      .pipe(
        tap(state => {
          this.currentState = state;
          this.lastUpdateTime = new Date();
          this.isLoading = false;
        }),
        takeUntil(this.destroy$)
      );

    // Initialize family-community protection monitoring
    this.familyProtection
      .initializeProtectionMonitoring(this.operatorId, this.mergedConfig.refreshInterval)
      .pipe(takeUntil(this.destroy$))
      .subscribe();

    // Initialize compute event emission
    this.computeEvents
      .initializeEventEmission(this.operatorId, this.stewardedResourceId)
      .pipe(takeUntil(this.destroy$))
      .subscribe(event => {
        if (this.mergedConfig.enableNotifications && event.tokensEarned > 0) {
          this.showNotification(`Earned ${event.tokensEarned.toFixed(2)} tokens for compute`);
        }
      });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  /**
   * Get status badge class based on node status
   */
  getStatusClass(): string {
    if (!this.currentState) return 'status-unknown';
    switch (this.currentState.status) {
      case 'online':
        return 'status-online';
      case 'degraded':
        return 'status-degraded';
      case 'offline':
        return 'status-offline';
      case 'maintenance':
        return 'status-maintenance';
      default:
        return 'status-unknown';
    }
  }

  /**
   * Get status text
   */
  getStatusText(): string {
    return this.currentState?.status.toUpperCase() || 'UNKNOWN';
  }

  /**
   * Get uptime percentage
   */
  getUptimePercent(): number {
    return this.currentState?.uptime.upPercent || 0;
  }

  /**
   * Get uptime reliability label
   */
  getReliabilityLabel(): string {
    const uptime = this.currentState?.uptime.reliability;
    return uptime ? uptime.charAt(0).toUpperCase() + uptime.slice(1) : 'Unknown';
  }

  /**
   * Get node location display
   */
  getNodeLocation(): string {
    const node = this.currentState?.nodeLocation;
    if (!node) return 'Location unknown';
    return `${node.region}, ${node.country}`;
  }

  /**
   * Panel visibility checks
   */
  isComputeMetricsVisible(): boolean {
    return this.mergedConfig.visiblePanels.computeMetrics;
  }

  isFamilyProtectionVisible(): boolean {
    return this.mergedConfig.visiblePanels.familyProtection;
  }

  isTokenEarningsVisible(): boolean {
    return this.mergedConfig.visiblePanels.tokenEarnings;
  }

  isEconomicEventsVisible(): boolean {
    return this.mergedConfig.visiblePanels.economicEvents;
  }

  isConstitutionalLimitsVisible(): boolean {
    return this.mergedConfig.visiblePanels.constitutionalLimits;
  }

  /**
   * Select panel to focus
   */
  selectPanel(panel: 'compute' | 'protection' | 'tokens' | 'events' | 'limits'): void {
    this.selectedPanel = panel;
  }

  /**
   * Get CPU usage percentage
   */
  getCpuUsage(): number {
    return this.currentState?.computeMetrics.cpu.usagePercent || 0;
  }

  /**
   * Get memory usage percentage
   */
  getMemoryUsage(): number {
    return this.currentState?.computeMetrics.memory.usagePercent || 0;
  }

  /**
   * Get storage usage percentage
   */
  getStorageUsage(): number {
    return this.currentState?.computeMetrics.storage.usagePercent || 0;
  }

  /**
   * Get total custodians protecting data
   */
  getTotalCustodians(): number {
    return this.currentState?.familyCommunityProtection.totalCustodians || 0;
  }

  /**
   * Get protection level with color coding
   */
  getProtectionLevelClass(): string {
    const level = this.currentState?.familyCommunityProtection.protectionLevel;
    switch (level) {
      case 'highly-protected':
        return 'protection-high';
      case 'protected':
        return 'protection-medium';
      case 'vulnerable':
        return 'protection-low';
      default:
        return 'protection-unknown';
    }
  }

  /**
   * Get infrastructure token balance
   */
  getTokenBalance(): number {
    return this.currentState?.infrastructureTokens.balance.tokens || 0;
  }

  /**
   * Get token earning rate (tokens/hour)
   */
  getTokenEarningRate(): number {
    return this.currentState?.infrastructureTokens.earningRate.tokensPerHour || 0;
  }

  /**
   * Get estimated monthly token earnings
   */
  getEstimatedMonthlyEarnings(): number {
    return this.currentState?.infrastructureTokens.earningRate.estimatedMonthly || 0;
  }

  /**
   * Get constitutional limit status
   */
  getConstitutionalStatus(): string {
    const limits = this.currentState?.constitutionalLimits;
    if (!limits) return 'Unknown';

    if (limits.dignityFloor.status === 'breached') {
      return 'Dignity floor breached';
    }
    if (limits.ceilingLimit.status === 'breached') {
      return 'Token ceiling exceeded';
    }
    if (limits.dignityFloor.status === 'warning' || limits.ceilingLimit.status === 'warning') {
      return 'Warning';
    }
    return 'Safe';
  }

  /**
   * Get number of active alerts
   */
  getAlertCount(): number {
    const limits = this.currentState?.constitutionalLimits;
    return limits?.alerts.length || 0;
  }

  /**
   * Get critical alerts only
   */
  getCriticalAlerts() {
    return (
      this.currentState?.constitutionalLimits.alerts.filter(a => a.severity === 'critical') || []
    );
  }

  /**
   * Format time since last update
   */
  getTimeSinceUpdate(): string {
    if (!this.lastUpdateTime) return 'Never';

    const now = new Date();
    const diffMs = now.getTime() - this.lastUpdateTime.getTime();
    const diffSecs = Math.floor(diffMs / 1000);

    if (diffSecs < 60) return `${diffSecs}s ago`;
    const diffMins = Math.floor(diffSecs / 60);
    if (diffMins < 60) return `${diffMins}m ago`;
    const diffHours = Math.floor(diffMins / 60);
    return `${diffHours}h ago`;
  }

  /**
   * Toggle configuration panel
   */
  toggleConfigPanel(): void {
    this.showConfigPanel = !this.showConfigPanel;
  }

  /**
   * Update display mode
   */
  setDisplayMode(mode: 'compact' | 'detailed' | 'monitoring'): void {
    this.mergedConfig.displayMode = mode;
  }

  /**
   * Toggle panel visibility
   */
  togglePanelVisibility(panel: keyof DashboardConfig['visiblePanels']): void {
    this.mergedConfig.visiblePanels[panel] = !this.mergedConfig.visiblePanels[panel];
  }

  /**
   * Show notification (would integrate with toast/notification service)
   */
  private showNotification(message: string): void {
    console.log('[ShefaDashboard] Notification:', message);
    // TODO: Integrate with NotificationService for actual toast notifications
  }

  /**
   * Export dashboard as CSV/JSON
   */
  exportDashboard(format: 'json' | 'csv'): void {
    if (!this.currentState) return;

    if (format === 'json') {
      const dataStr = JSON.stringify(this.currentState, null, 2);
      this.downloadFile(
        dataStr,
        `shefa-dashboard-${new Date().toISOString()}.json`,
        'application/json'
      );
    } else if (format === 'csv') {
      const csv = this.convertToCsv(this.currentState);
      this.downloadFile(csv, `shefa-dashboard-${new Date().toISOString()}.csv`, 'text/csv');
    }
  }

  /**
   * Convert dashboard state to CSV format
   */
  private convertToCsv(state: SheafaDashboardState): string {
    const rows: string[] = [];

    // Header
    rows.push('Dashboard Export');
    rows.push(`Timestamp,${new Date().toISOString()}`);
    rows.push(`Operator ID,${state.operatorId}`);
    rows.push(`Status,${state.status}`);
    rows.push('');

    // Compute metrics
    rows.push('Compute Metrics');
    rows.push(`CPU Usage,${state.computeMetrics.cpu.usagePercent.toFixed(2)}%`);
    rows.push(`Memory Usage,${state.computeMetrics.memory.usagePercent.toFixed(2)}%`);
    rows.push(`Storage Usage,${state.computeMetrics.storage.usagePercent.toFixed(2)}%`);
    rows.push('');

    // Tokens
    rows.push('Infrastructure Tokens');
    rows.push(`Balance,${state.infrastructureTokens.balance.tokens.toFixed(2)} tokens`);
    rows.push(
      `Earning Rate,${state.infrastructureTokens.earningRate.tokensPerHour.toFixed(4)} tokens/hour`
    );
    rows.push('');

    // Protection
    rows.push('Data Protection');
    rows.push(`Custodians,${state.familyCommunityProtection.totalCustodians}`);
    rows.push(`Protection Level,${state.familyCommunityProtection.protectionLevel}`);
    rows.push('');

    return rows.join('\n');
  }

  /**
   * Download file helper
   */
  private downloadFile(content: string, filename: string, type: string): void {
    const blob = new Blob([content], { type });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  }
}
