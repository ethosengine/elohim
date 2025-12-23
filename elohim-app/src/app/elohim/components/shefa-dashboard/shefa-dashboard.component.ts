import { Component, OnInit, OnDestroy, inject, signal, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ShefaService } from '../../services/shefa.service';
import { CustodianSelectionService } from '../../services/custodian-selection.service';

/**
 * Shefa Dashboard Component
 *
 * Provides operators with visibility into:
 * - Network health overview
 * - Individual custodian metrics
 * - Alerts and recommendations
 * - Top performers by category
 *
 * Refreshes every 30 seconds for real-time visibility.
 */

@Component({
  selector: 'app-shefa-dashboard',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './shefa-dashboard.component.html',
  styleUrl: './shefa-dashboard.component.css'
})
export class ShefaDashboardComponent implements OnInit, OnDestroy {
  private readonly shefa = inject(ShefaService);
  private readonly selection = inject(CustodianSelectionService);

  // Expose Math for template use
  readonly Math = Math;

  // Data signals
  readonly allMetrics = signal<any[]>([]);
  readonly alerts = signal<any[]>([]);
  readonly recommendations = signal<any[]>([]);

  // Tab state
  readonly activeTab = signal<'overview' | 'custodians' | 'alerts' | 'performance'>('overview');

  // Filter state
  readonly filterByTier = signal<1 | 2 | 3 | 4 | 'all'>('all');
  readonly sortBy = signal<'health' | 'reputation' | 'capacity' | 'earnings'>('health');

  // Computed values
  readonly totalCustodians = computed(() => this.allMetrics().length);

  readonly healthyCustodians = computed(() =>
    this.allMetrics().filter(c => c.health.uptime_percent >= 95).length
  );

  readonly networkUptime = computed(() => {
    const metrics = this.allMetrics();
    if (metrics.length === 0) return 0;
    const sum = metrics.reduce((acc, c) => acc + c.health.uptime_percent, 0);
    return sum / metrics.length;
  });

  readonly avgResponseTime = computed(() => {
    const metrics = this.allMetrics();
    if (metrics.length === 0) return 0;
    const sum = metrics.reduce((acc, c) => acc + c.health.response_time_p95_ms, 0);
    return Math.round(sum / metrics.length);
  });

  readonly totalCommitments = computed(() =>
    this.allMetrics().reduce((sum, c) => sum + c.economic.active_commitments, 0)
  );

  readonly totalStorage = computed(() => {
    const bytes = this.allMetrics().reduce((sum, c) => sum + c.storage.used_bytes, 0);
    return this.formatBytes(bytes);
  });

  readonly totalBandwidth = computed(() => {
    const sum = this.allMetrics().reduce((acc, c) => acc + c.bandwidth.declared_mbps, 0);
    return Math.round(sum);
  });

  // Filtered custodians
  readonly filteredCustodians = computed(() => {
    let custodians = [...this.allMetrics()];

    // Filter by tier
    if (this.filterByTier() !== 'all') {
      custodians = custodians.filter(c => c.economic.steward_tier === this.filterByTier());
    }

    // Sort
    switch (this.sortBy()) {
      case 'health':
        custodians.sort((a, b) => b.health.uptime_percent - a.health.uptime_percent);
        break;
      case 'reputation':
        custodians.sort((a, b) => b.reputation.reputation_score - a.reputation.reputation_score);
        break;
      case 'capacity':
        custodians.sort(
          (a, b) =>
            b.bandwidth.declared_mbps - a.bandwidth.declared_mbps ||
            b.storage.total_capacity_bytes - a.storage.total_capacity_bytes
        );
        break;
      case 'earnings':
        custodians.sort((a, b) => b.economic.monthly_earnings - a.economic.monthly_earnings);
        break;
    }

    return custodians.slice(0, 20); // Show top 20
  });

  // Top performers
  readonly topByHealth = computed(() =>
    this.allMetrics()
      .sort((a, b) => b.health.uptime_percent - a.health.uptime_percent)
      .slice(0, 5)
  );

  readonly topBySpeed = computed(() =>
    this.allMetrics()
      .sort((a, b) => a.health.response_time_p95_ms - b.health.response_time_p95_ms)
      .slice(0, 5)
  );

  readonly topByReputation = computed(() =>
    this.allMetrics()
      .sort((a, b) => b.reputation.reputation_score - a.reputation.reputation_score)
      .slice(0, 5)
  );

  private refreshInterval: any;

  async ngOnInit(): Promise<void> {
    await this.loadData();

    // Refresh every 30 seconds
    this.refreshInterval = setInterval(() => this.loadData(), 30000);
  }

  ngOnDestroy(): void {
    if (this.refreshInterval) {
      clearInterval(this.refreshInterval);
    }
  }

  private async loadData(): Promise<void> {
    try {
      const [allMetrics, alerts, recommendations] = await Promise.all([
        this.shefa.getAllMetrics(),
        this.shefa.getAlerts(),
        this.shefa.getRecommendations()
      ]);

      this.allMetrics.set(allMetrics);
      this.alerts.set(alerts);
      this.recommendations.set(recommendations);
    } catch (err) {
      console.error('[ShefaDashboard] Error loading data:', err);
    }
  }

  // Helper methods
  formatBytes(bytes: number): string {
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    let size = bytes;
    let unitIndex = 0;

    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024;
      unitIndex++;
    }

    return `${size.toFixed(2)} ${units[unitIndex]}`;
  }

  formatPercentage(value: number, decimals: number = 1): string {
    return `${value.toFixed(decimals)}%`;
  }

  getHealthClass(uptime: number): string {
    if (uptime >= 99) return 'health-excellent';
    if (uptime >= 95) return 'health-good';
    if (uptime >= 90) return 'health-warning';
    return 'health-critical';
  }

  getHealthLabel(uptime: number): string {
    if (uptime >= 99) return 'Excellent';
    if (uptime >= 95) return 'Good';
    if (uptime >= 90) return 'Warning';
    return 'Critical';
  }

  getTierLabel(tier: number): string {
    const labels: Record<number, string> = {
      1: 'Caretaker',
      2: 'Curator',
      3: 'Expert',
      4: 'Pioneer'
    };
    return labels[tier] || 'Unknown';
  }

  getAlertIcon(severity: string): string {
    return severity === 'critical' ? '⚠️' : '⚠';
  }

  setActiveTab(tab: 'overview' | 'custodians' | 'alerts' | 'performance'): void {
    this.activeTab.set(tab);
  }

  refreshData(): void {
    this.loadData();
  }
}
