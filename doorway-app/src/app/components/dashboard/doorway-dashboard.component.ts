import { Component, OnInit, OnDestroy, inject, signal, computed } from '@angular/core';
import { CommonModule, DecimalPipe } from '@angular/common';
import { DoorwayAdminService } from '../../services/doorway-admin.service';
import {
  NodeDetails,
  ClusterMetrics,
  ResourceSummary,
  CustodianNetwork,
  NodeStatus,
  statusColor,
  tierColor,
  reachLevelName,
  // User admin models
  UserSummary,
  UserDetails,
  ListUsersParams,
  UserPermissionLevel,
  permissionLevelColor,
  permissionLevelName,
  quotaStatusColor,
  formatBytes,
} from '../../models/doorway.model';

type SortField = 'nodeId' | 'status' | 'combinedScore' | 'trustScore' | 'stewardTier';
type SortDirection = 'asc' | 'desc';
type UserSortField = 'identifier' | 'permissionLevel' | 'isActive' | 'storagePercent' | 'createdAt';

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
  readonly activeTab = signal<'overview' | 'nodes' | 'resources' | 'users'>('overview');
  readonly sortField = signal<SortField>('combinedScore');
  readonly sortDirection = signal<SortDirection>('desc');
  readonly statusFilter = signal<NodeStatus | 'all'>('all');

  // User admin state
  readonly users = signal<UserSummary[]>([]);
  readonly usersTotal = signal(0);
  readonly usersPage = signal(1);
  readonly usersPageSize = signal(20);
  readonly usersTotalPages = signal(0);
  readonly usersLoading = signal(false);
  readonly usersSearch = signal('');
  readonly usersPermFilter = signal<UserPermissionLevel | 'all'>('all');
  readonly usersActiveFilter = signal<boolean | 'all'>('all');
  readonly usersQuotaFilter = signal<boolean | 'all'>('all');
  readonly userSortField = signal<UserSortField>('createdAt');
  readonly userSortDir = signal<SortDirection>('desc');
  readonly selectedUser = signal<UserDetails | null>(null);
  readonly showUserDetail = signal(false);

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

  readonly healthyCount = computed(() =>
    this.nodes().filter(n => n.status === 'online').length
  );

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
  readonly permissionLevelColor = permissionLevelColor;
  readonly permissionLevelName = permissionLevelName;
  readonly quotaStatusColor = quotaStatusColor;
  readonly formatBytesHelper = formatBytes;
  readonly Math = Math; // Expose Math for template

  async ngOnInit(): Promise<void> {
    await this.loadData();
    this.startRefresh();
    this.adminService.connect();
  }

  ngOnDestroy(): void {
    this.stopRefresh();
    this.adminService.disconnect();
  }

  setTab(tab: 'overview' | 'nodes' | 'resources' | 'users'): void {
    this.activeTab.set(tab);
    // Load users when switching to users tab
    if (tab === 'users' && this.users().length === 0) {
      this.loadUsers();
    }
  }

  setSort(field: SortField): void {
    if (this.sortField() === field) {
      this.sortDirection.update(d => d === 'asc' ? 'desc' : 'asc');
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
    if (this.activeTab() === 'users') {
      await this.loadUsers();
    }
  }

  // ============================================================================
  // User Admin Methods
  // ============================================================================

  async loadUsers(): Promise<void> {
    this.usersLoading.set(true);
    try {
      const params: ListUsersParams = {
        page: this.usersPage(),
        limit: this.usersPageSize(),
        sortBy: this.userSortField(),
        sortDir: this.userSortDir(),
      };

      const search = this.usersSearch();
      if (search) params.search = search;

      const permFilter = this.usersPermFilter();
      if (permFilter !== 'all') params.permissionLevel = permFilter;

      const activeFilter = this.usersActiveFilter();
      if (activeFilter !== 'all') params.isActive = activeFilter;

      const quotaFilter = this.usersQuotaFilter();
      if (quotaFilter !== 'all') params.overQuota = quotaFilter;

      const response = await this.adminService.listUsers(params).toPromise();
      if (response) {
        this.users.set(response.users);
        this.usersTotal.set(response.total);
        this.usersTotalPages.set(response.totalPages);
      }
    } catch {
      // Failed to load users
    } finally {
      this.usersLoading.set(false);
    }
  }

  setUserSort(field: UserSortField): void {
    if (this.userSortField() === field) {
      this.userSortDir.update(d => d === 'asc' ? 'desc' : 'asc');
    } else {
      this.userSortField.set(field);
      this.userSortDir.set('desc');
    }
    this.loadUsers();
  }

  setUsersPage(page: number): void {
    this.usersPage.set(page);
    this.loadUsers();
  }

  searchUsers(term: string): void {
    this.usersSearch.set(term);
    this.usersPage.set(1);
    this.loadUsers();
  }

  filterByPermission(level: UserPermissionLevel | 'all'): void {
    this.usersPermFilter.set(level);
    this.usersPage.set(1);
    this.loadUsers();
  }

  filterByActive(active: boolean | 'all'): void {
    this.usersActiveFilter.set(active);
    this.usersPage.set(1);
    this.loadUsers();
  }

  filterByQuota(overQuota: boolean | 'all'): void {
    this.usersQuotaFilter.set(overQuota);
    this.usersPage.set(1);
    this.loadUsers();
  }

  async viewUser(userId: string): Promise<void> {
    const user = await this.adminService.getUser(userId).toPromise();
    if (user) {
      this.selectedUser.set(user);
      this.showUserDetail.set(true);
    }
  }

  closeUserDetail(): void {
    this.showUserDetail.set(false);
    this.selectedUser.set(null);
  }

  async toggleUserStatus(userId: string, currentActive: boolean): Promise<void> {
    const result = await this.adminService.updateUserStatus(userId, !currentActive).toPromise();
    if (result?.success) {
      this.loadUsers();
      // Update selected user if open
      if (this.selectedUser()?.id === userId) {
        this.viewUser(userId);
      }
    } else {
      // Failed to toggle user status
    }
  }

  async forceUserLogout(userId: string): Promise<void> {
    if (confirm('This will invalidate all active sessions for this user. Continue?')) {
      const result = await this.adminService.forceLogout(userId).toPromise();
      if (result?.success) {
        alert('User has been logged out from all sessions.');
        if (this.selectedUser()?.id === userId) {
          this.viewUser(userId);
        }
      } else {
        alert('Failed to force logout: ' + result?.message);
      }
    }
  }

  async deleteUser(userId: string): Promise<void> {
    if (confirm('Are you sure you want to delete this user? This action cannot be undone.')) {
      const result = await this.adminService.deleteUser(userId).toPromise();
      if (result?.success) {
        this.closeUserDetail();
        this.loadUsers();
      } else {
        alert('Failed to delete user: ' + result?.message);
      }
    }
  }

  async resetUserUsage(userId: string): Promise<void> {
    if (confirm('Reset all usage counters for this user?')) {
      const result = await this.adminService.resetUsage(userId).toPromise();
      if (result?.success) {
        if (this.selectedUser()?.id === userId) {
          this.viewUser(userId);
        }
        this.loadUsers();
      } else {
        alert('Failed to reset usage: ' + result?.message);
      }
    }
  }

  async updateUserPermission(userId: string, level: UserPermissionLevel): Promise<void> {
    const result = await this.adminService.updatePermission(userId, level).toPromise();
    if (result?.success) {
      if (this.selectedUser()?.id === userId) {
        this.viewUser(userId);
      }
      this.loadUsers();
    } else {
      alert('Failed to update permission: ' + result?.message);
    }
  }

  private async loadData(): Promise<void> {
    this.loading.set(true);
    this.error.set(null);

    try {
      // Load all data in parallel
      const [nodesRes, cluster, resources, custodians] = await Promise.all([
        this.adminService.getNodes().toPromise(),
        this.adminService.getClusterMetrics().toPromise(),
        this.adminService.getResources().toPromise(),
        this.adminService.getCustodians().toPromise(),
      ]);

      if (nodesRes) {
        this.nodes.set(nodesRes.nodes);
      }
      this.cluster.set(cluster ?? null);
      this.resources.set(resources ?? null);
      this.custodians.set(custodians ?? null);
    } catch (e) {
      console.error('[Dashboard] Failed to load data:', e);
      this.error.set('Failed to load dashboard data');
    } finally {
      this.loading.set(false);
    }
  }

  private startRefresh(): void {
    // Refresh every 30 seconds
    this.refreshInterval = setInterval(() => this.loadData(), 30000);
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
