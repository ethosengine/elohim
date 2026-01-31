import { CommonModule, DecimalPipe } from '@angular/common';
import { Component, OnInit, OnDestroy, inject, signal, computed } from '@angular/core';

// @coverage: 93.7% (2026-01-31)

import { firstValueFrom } from 'rxjs';

import {
  NodeDetails,
  ClusterMetrics,
  ResourceSummary,
  CustodianNetwork,
  NodeStatus,
  statusColor,
  tierColor,
  reachLevelName,
} from '../../models/doorway.model';
import { DoorwayAdminService } from '../../services/doorway-admin.service';

type SortField = 'nodeId' | 'status' | 'combinedScore' | 'trustScore' | 'stewardTier';
type SortDirection = 'asc' | 'desc';

/**
 * Doorway Operator Dashboard
 *
 * Displays compute resources, node health, and human-scale metrics
 * for doorway operators. Connects via WebSocket for real-time updates.
 */
@Component({
  selector: 'app-doorway-dashboard',
  standalone: true,
  imports: [CommonModule, DecimalPipe],
  templateUrl: './doorway-dashboard.component.html',
  styleUrl: './doorway-dashboard.component.scss',
})
export class DoorwayDashboardComponent implements OnInit, OnDestroy {
  private readonly adminService = inject(DoorwayAdminService);

  // Data state
  readonly nodes = signal<NodeDetails[]>([]);
  readonly cluster = signal<ClusterMetrics | null>(null);
  readonly resources = signal<ResourceSummary | null>(null);
  readonly custodians = signal<CustodianNetwork | null>(null);
  readonly loading = signal(true);
  readonly error = signal<string | null>(null);

  // UI state
  readonly activeTab = signal<'overview' | 'nodes' | 'resources'>('overview');
  readonly sortField = signal<SortField>('combinedScore');
  readonly sortDirection = signal<SortDirection>('desc');
  readonly statusFilter = signal<NodeStatus | 'all'>('all');

  // Connection state
  readonly connectionState = this.adminService.connectionState;
  readonly isConnected = this.adminService.isConnected;

  // Computed values
  readonly sortedNodes = computed(() => {
    let filtered = this.nodes();

    // Apply status filter
    const filter = this.statusFilter();
    if (filter !== 'all') {
      filtered = filtered.filter(n => n.status === filter);
    }

    // Sort
    const field = this.sortField();
    const dir = this.sortDirection();
    return [...filtered].sort((a, b) => {
      let cmp = 0;
      switch (field) {
        case 'nodeId':
          cmp = a.nodeId.localeCompare(b.nodeId);
          break;
        case 'status':
          cmp = a.status.localeCompare(b.status);
          break;
        case 'combinedScore':
          cmp = a.combinedScore - b.combinedScore;
          break;
        case 'trustScore':
          cmp = (a.trustScore ?? 0) - (b.trustScore ?? 0);
          break;
        case 'stewardTier':
          cmp = (a.stewardTier ?? '').localeCompare(b.stewardTier ?? '');
          break;
      }
      return dir === 'asc' ? cmp : -cmp;
    });
  });

  readonly healthyCount = computed(() => this.nodes().filter(n => n.status === 'online').length);

  readonly totalCapacity = computed(() => {
    const c = this.cluster();
    if (!c) return null;
    return {
      cpu: c.totalCpuCores,
      memory: c.totalMemoryGb,
      storage: c.totalStorageTb,
      bandwidth: c.totalBandwidthMbps,
    };
  });

  readonly stewardDistribution = computed(() => {
    const c = this.cluster();
    if (!c) return [];
    const counts = c.stewardCounts;
    const total = counts.pioneers + counts.stewards + counts.guardians + counts.caretakers;
    if (total === 0) return [];
    return [
      { tier: 'Pioneer', count: counts.pioneers, pct: counts.pioneers / total },
      { tier: 'Steward', count: counts.stewards, pct: counts.stewards / total },
      { tier: 'Guardian', count: counts.guardians, pct: counts.guardians / total },
      { tier: 'Caretaker', count: counts.caretakers, pct: counts.caretakers / total },
    ].filter(d => d.count > 0);
  });

  private refreshInterval: ReturnType<typeof setInterval> | null = null;

  // Helper methods exposed to template
  readonly statusColor = statusColor;
  readonly tierColor = tierColor;
  readonly reachLevelName = reachLevelName;

  ngOnInit(): void {
    void this.loadData();
    this.startRefresh();
    this.adminService.connect();
  }

  ngOnDestroy(): void {
    this.stopRefresh();
    this.adminService.disconnect();
  }

  setTab(tab: 'overview' | 'nodes' | 'resources'): void {
    this.activeTab.set(tab);
  }

  setSort(field: SortField): void {
    if (this.sortField() === field) {
      this.sortDirection.update(d => (d === 'asc' ? 'desc' : 'asc'));
    } else {
      this.sortField.set(field);
      this.sortDirection.set('desc');
    }
  }

  setStatusFilter(status: NodeStatus | 'all'): void {
    this.statusFilter.set(status);
  }

  async refresh(): Promise<void> {
    await this.loadData();
  }

  private async loadData(): Promise<void> {
    this.loading.set(true);
    this.error.set(null);

    try {
      // Load all data in parallel
      const [nodesRes, cluster, resources, custodians] = await Promise.all([
        firstValueFrom(this.adminService.getNodes()),
        firstValueFrom(this.adminService.getClusterMetrics()),
        firstValueFrom(this.adminService.getResources()),
        firstValueFrom(this.adminService.getCustodians()),
      ]);

      if (nodesRes) {
        this.nodes.set(nodesRes.nodes);
      }
      this.cluster.set(cluster ?? null);
      this.resources.set(resources ?? null);
      this.custodians.set(custodians ?? null);
    } catch {
      this.error.set('Failed to load dashboard data');
    } finally {
      this.loading.set(false);
    }
  }

  private startRefresh(): void {
    // Refresh every 30 seconds
    this.refreshInterval = setInterval(() => {
      void this.loadData();
    }, 30000);
  }

  private stopRefresh(): void {
    if (this.refreshInterval) {
      clearInterval(this.refreshInterval);
      this.refreshInterval = null;
    }
  }

  // Format helpers
  formatBytes(gb: number | null): string {
    if (gb === null) return '-';
    if (gb >= 1024) return `${(gb / 1024).toFixed(1)} TB`;
    return `${gb.toFixed(1)} GB`;
  }

  formatPercent(value: number | null): string {
    if (value === null) return '-';
    return `${(value * 100).toFixed(1)}%`;
  }

  formatNumber(value: number | null): string {
    if (value === null) return '-';
    return value.toLocaleString();
  }

  formatTime(secs: number | null): string {
    if (secs === null) return 'Never';
    if (secs < 60) return `${secs}s ago`;
    if (secs < 3600) return `${Math.floor(secs / 60)}m ago`;
    return `${Math.floor(secs / 3600)}h ago`;
  }

  getReachCount(level: number): number {
    const coverage = this.cluster()?.reachCoverage;
    if (!coverage) return 0;
    const counts: Record<number, number> = {
      0: coverage.private,
      1: coverage.invited,
      2: coverage.local,
      3: coverage.neighborhood,
      4: coverage.municipal,
      5: coverage.bioregional,
      6: coverage.regional,
      7: coverage.commons,
    };
    return counts[level] ?? 0;
  }
}
