import { CommonModule } from '@angular/common';
import { Component, OnInit, OnDestroy, inject, signal, computed } from '@angular/core';

// @coverage: 26.7% (2026-02-04)

import { CustodianSelectionService } from '../../services/custodian-selection.service';
import { ShefaService, CustodianMetrics } from '../../services/shefa.service';

/** Alert from the Shefa service */
interface ShefaAlert {
  readonly custodianId: string;
  readonly severity: 'warning' | 'critical';
  readonly category: string;
  readonly message: string;
  readonly suggestion?: string;
}

/** Recommendation from the Shefa service */
interface ShefaRecommendation {
  readonly custodianId: string;
  readonly category: string;
  readonly opportunity: string;
  readonly potentialRevenue?: number;
}

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
  styleUrl: './shefa-dashboard.component.css',
})
export class ShefaDashboardComponent implements OnInit, OnDestroy {
  private readonly shefa = inject(ShefaService);
  private readonly selection = inject(CustodianSelectionService);

  // Expose Math for template use
  readonly Math = Math;

  // Data signals
  readonly allMetrics = signal<CustodianMetrics[]>([]);
  readonly alerts = signal<ShefaAlert[]>([]);
  readonly recommendations = signal<ShefaRecommendation[]>([]);

  // Tab state
  readonly activeTab = signal<'overview' | 'custodians' | 'alerts' | 'performance'>('overview');

  // Filter state
  readonly filterByTier = signal<1 | 2 | 3 | 4 | 'all'>('all');
  readonly sortBy = signal<'health' | 'reputation' | 'capacity' | 'earnings'>('health');

  // Computed values
  readonly totalCustodians = computed(() => this.allMetrics().length);

  readonly healthyCustodians = computed(
    () => this.allMetrics().filter(c => c.health.uptimePercent >= 95).length
  );

  readonly networkUptime = computed(() => {
    const metrics = this.allMetrics();
    if (metrics.length === 0) return 0;
    const sum = metrics.reduce((acc, c) => acc + c.health.uptimePercent, 0);
    return sum / metrics.length;
  });

  readonly avgResponseTime = computed(() => {
    const metrics = this.allMetrics();
    if (metrics.length === 0) return 0;
    const sum = metrics.reduce((acc, c) => acc + c.health.responseTimeP95Ms, 0);
    return Math.round(sum / metrics.length);
  });

  readonly totalCommitments = computed(() =>
    this.allMetrics().reduce((sum, c) => sum + c.economic.activeCommitments, 0)
  );

  readonly totalStorage = computed(() => {
    const bytes = this.allMetrics().reduce((sum, c) => sum + c.storage.usedBytes, 0);
    return this.formatBytes(bytes);
  });

  readonly totalBandwidth = computed(() => {
    const sum = this.allMetrics().reduce((acc, c) => acc + c.bandwidth.declaredMbps, 0);
    return Math.round(sum);
  });

  // Filtered custodians
  readonly filteredCustodians = computed(() => {
    let custodians = [...this.allMetrics()];

    // Filter by tier
    if (this.filterByTier() !== 'all') {
      custodians = custodians.filter(c => c.economic.stewardTier === this.filterByTier());
    }

    // Sort
    switch (this.sortBy()) {
      case 'health':
        custodians.sort((a, b) => b.health.uptimePercent - a.health.uptimePercent);
        break;
      case 'reputation':
        custodians.sort((a, b) => b.reputation.reputationScore - a.reputation.reputationScore);
        break;
      case 'capacity':
        custodians.sort(
          (a, b) =>
            b.bandwidth.declaredMbps - a.bandwidth.declaredMbps ||
            b.storage.totalCapacityBytes - a.storage.totalCapacityBytes
        );
        break;
      case 'earnings':
        custodians.sort((a, b) => b.economic.monthlyEarnings - a.economic.monthlyEarnings);
        break;
    }

    return custodians.slice(0, 20); // Show top 20
  });

  // Top performers
  readonly topByHealth = computed(() =>
    [...this.allMetrics()]
      .sort((a, b) => b.health.uptimePercent - a.health.uptimePercent)
      .slice(0, 5)
  );

  readonly topBySpeed = computed(() =>
    [...this.allMetrics()]
      .sort((a, b) => a.health.responseTimeP95Ms - b.health.responseTimeP95Ms)
      .slice(0, 5)
  );

  readonly topByReputation = computed(() =>
    [...this.allMetrics()]
      .sort((a, b) => b.reputation.reputationScore - a.reputation.reputationScore)
      .slice(0, 5)
  );

  private refreshInterval: ReturnType<typeof setInterval> | null = null;

  ngOnInit(): void {
    void this.loadData();

    // Refresh every 30 seconds
    this.refreshInterval = setInterval(() => void this.loadData(), 30000);
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
        this.shefa.getRecommendations(),
      ]);

      this.allMetrics.set(allMetrics);
      this.alerts.set(alerts);
      this.recommendations.set(recommendations);
    } catch {
      // Metrics load failure is non-critical - dashboard will show empty state
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

  formatPercentage(value: number, decimals = 1): string {
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
      4: 'Pioneer',
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
    void this.loadData();
  }
}
