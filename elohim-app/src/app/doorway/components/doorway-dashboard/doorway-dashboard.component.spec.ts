import {
  ComponentFixture,
  TestBed,
  fakeAsync,
  tick,
  discardPeriodicTasks,
} from '@angular/core/testing';
import { DecimalPipe } from '@angular/common';

import { of } from 'rxjs';

import { DoorwayDashboardComponent } from './doorway-dashboard.component';
import { DoorwayAdminService } from '../../services/doorway-admin.service';
import {
  NodesResponse,
  NodeDetails,
  ClusterMetrics,
  ResourceSummary,
  CustodianNetwork,
} from '../../models/doorway.model';

describe('DoorwayDashboardComponent', () => {
  let component: DoorwayDashboardComponent;
  let fixture: ComponentFixture<DoorwayDashboardComponent>;
  let mockAdminService: jasmine.SpyObj<DoorwayAdminService>;

  const mockNodesResponse: NodesResponse = {
    total: 3,
    byStatus: {
      online: 2,
      degraded: 1,
      offline: 0,
      failed: 0,
      discovering: 0,
      registering: 0,
    },
    nodes: [
      createMockNode('node-1', 'online', 0.9),
      createMockNode('node-2', 'online', 0.85),
      createMockNode('node-3', 'degraded', 0.7),
    ],
  };

  const mockClusterMetrics: ClusterMetrics = {
    region: 'us-west',
    totalNodes: 3,
    onlineNodes: 2,
    healthRatio: 0.67,
    totalCpuCores: 12,
    totalMemoryGb: 48,
    totalStorageTb: 3,
    totalBandwidthMbps: 300,
    avgCpuUsagePercent: 40,
    avgMemoryUsagePercent: 55,
    totalStorageUsedTb: 1.5,
    totalActiveConnections: 100,
    totalCustodiedContentGb: 500,
    avgTrustScore: 0.82,
    avgImpactScore: 0.75,
    totalHumansServed: 1000,
    totalContentCustodied: 5000,
    clusterDeliverySuccessRate: 0.98,
    stewardCounts: { pioneers: 1, stewards: 1, guardians: 1, caretakers: 0 },
    reachCoverage: {
      private: 50,
      invited: 100,
      local: 200,
      neighborhood: 300,
      municipal: 200,
      bioregional: 100,
      regional: 50,
      commons: 0,
    },
  };

  const mockResources: ResourceSummary = {
    cpu: { total: 12, used: 5, available: 7, utilizationPercent: 42 },
    memory: { total: 48, used: 26, available: 22, utilizationPercent: 55 },
    storage: {
      totalTb: 3,
      usedTb: 1.5,
      availableTb: 1.5,
      utilizationPercent: 50,
      custodiedContentGb: 500,
    },
    bandwidth: { totalMbps: 300, activeConnections: 100, avgBandwidthPerConnectionMbps: 3 },
    cache: { entries: 500, hits: 4500, misses: 500, hitRate: 0.9 },
  };

  const mockCustodians: CustodianNetwork = {
    registeredCustodians: 10,
    trackedBlobs: 1000,
    totalCommitments: 5000,
    totalProbes: 500,
    successfulProbes: 490,
    probeSuccessRate: 0.98,
    totalSelections: 1000,
    healthyCustodians: 9,
  };

  beforeEach(async () => {
    mockAdminService = jasmine.createSpyObj(
      'DoorwayAdminService',
      ['getNodes', 'getClusterMetrics', 'getResources', 'getCustodians', 'connect', 'disconnect'],
      {
        connectionState: jasmine.createSpy('connectionState').and.returnValue('disconnected'),
        isConnected: jasmine.createSpy('isConnected').and.returnValue(false),
      }
    );

    mockAdminService.getNodes.and.returnValue(of(mockNodesResponse));
    mockAdminService.getClusterMetrics.and.returnValue(of(mockClusterMetrics));
    mockAdminService.getResources.and.returnValue(of(mockResources));
    mockAdminService.getCustodians.and.returnValue(of(mockCustodians));

    await TestBed.configureTestingModule({
      imports: [DoorwayDashboardComponent],
      providers: [{ provide: DoorwayAdminService, useValue: mockAdminService }, DecimalPipe],
    }).compileComponents();

    fixture = TestBed.createComponent(DoorwayDashboardComponent);
    component = fixture.componentInstance;
  });

  afterEach(() => {
    // Clean up any intervals
    fixture.destroy();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('initialization', () => {
    it('should load data on init', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      expect(mockAdminService.getNodes).toHaveBeenCalled();
      expect(mockAdminService.getClusterMetrics).toHaveBeenCalled();
      expect(mockAdminService.getResources).toHaveBeenCalled();
      expect(mockAdminService.getCustodians).toHaveBeenCalled();

      discardPeriodicTasks();
    }));

    it('should connect to WebSocket on init', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      expect(mockAdminService.connect).toHaveBeenCalled();

      discardPeriodicTasks();
    }));

    it('should set loading to false after data loads', fakeAsync(() => {
      expect(component.loading()).toBeTrue();

      fixture.detectChanges();
      tick();

      expect(component.loading()).toBeFalse();

      discardPeriodicTasks();
    }));

    it('should populate nodes signal with fetched data', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      expect(component.nodes().length).toBe(3);
      expect(component.nodes()[0].nodeId).toBe('node-1');

      discardPeriodicTasks();
    }));

    it('should populate cluster signal with fetched data', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      expect(component.cluster()?.totalNodes).toBe(3);

      discardPeriodicTasks();
    }));
  });

  describe('ngOnDestroy', () => {
    it('should disconnect WebSocket on destroy', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      fixture.destroy();

      expect(mockAdminService.disconnect).toHaveBeenCalled();
    }));
  });

  describe('tab management', () => {
    it('should have overview as default active tab', () => {
      expect(component.activeTab()).toBe('overview');
    });

    it('should change active tab with setTab', () => {
      component.setTab('nodes');
      expect(component.activeTab()).toBe('nodes');

      component.setTab('resources');
      expect(component.activeTab()).toBe('resources');

      component.setTab('overview');
      expect(component.activeTab()).toBe('overview');
    });
  });

  describe('sorting', () => {
    it('should have combinedScore as default sort field', () => {
      expect(component.sortField()).toBe('combinedScore');
    });

    it('should have desc as default sort direction', () => {
      expect(component.sortDirection()).toBe('desc');
    });

    it('should change sort field', () => {
      component.setSort('nodeId');
      expect(component.sortField()).toBe('nodeId');
      expect(component.sortDirection()).toBe('desc');
    });

    it('should toggle sort direction when clicking same field', () => {
      component.setSort('combinedScore');
      expect(component.sortDirection()).toBe('asc');

      component.setSort('combinedScore');
      expect(component.sortDirection()).toBe('desc');
    });

    it('should reset direction to desc when changing field', () => {
      component.setSort('combinedScore'); // Toggle to asc
      expect(component.sortDirection()).toBe('asc');

      component.setSort('nodeId'); // Change field
      expect(component.sortDirection()).toBe('desc');
    });
  });

  describe('filtering', () => {
    it('should have all as default status filter', () => {
      expect(component.statusFilter()).toBe('all');
    });

    it('should change status filter', () => {
      component.setStatusFilter('online');
      expect(component.statusFilter()).toBe('online');

      component.setStatusFilter('degraded');
      expect(component.statusFilter()).toBe('degraded');
    });
  });

  describe('sortedNodes computed', () => {
    beforeEach(fakeAsync(() => {
      fixture.detectChanges();
      tick();
      discardPeriodicTasks();
    }));

    it('should return all nodes when filter is all', () => {
      component.setStatusFilter('all');
      expect(component.sortedNodes().length).toBe(3);
    });

    it('should filter by status', () => {
      component.setStatusFilter('online');
      const sorted = component.sortedNodes();
      expect(sorted.length).toBe(2);
      expect(sorted.every(n => n.status === 'online')).toBeTrue();
    });

    it('should sort by combinedScore descending', () => {
      // Initial sort is already combinedScore desc by default
      // Verify default sorting is correct
      const sorted = component.sortedNodes();
      expect(sorted[0].combinedScore).toBeGreaterThanOrEqual(sorted[1].combinedScore);
    });

    it('should sort by nodeId ascending when selected', () => {
      component.setSort('nodeId');
      // First click on nodeId sets it to desc (default for new field)
      // Toggle to ascending
      component.setSort('nodeId');
      const sorted = component.sortedNodes();
      expect(sorted[0].nodeId.localeCompare(sorted[1].nodeId)).toBeLessThanOrEqual(0);
    });
  });

  describe('computed values', () => {
    beforeEach(fakeAsync(() => {
      fixture.detectChanges();
      tick();
      discardPeriodicTasks();
    }));

    it('should compute healthyCount correctly', () => {
      expect(component.healthyCount()).toBe(2);
    });

    it('should compute totalCapacity correctly', () => {
      const capacity = component.totalCapacity();
      expect(capacity?.cpu).toBe(12);
      expect(capacity?.memory).toBe(48);
      expect(capacity?.storage).toBe(3);
      expect(capacity?.bandwidth).toBe(300);
    });

    it('should compute stewardDistribution correctly', () => {
      const dist = component.stewardDistribution();
      expect(dist.length).toBe(3); // 3 tiers with counts > 0
      expect(dist.find(d => d.tier === 'Pioneer')?.count).toBe(1);
    });

    it('should return null totalCapacity when no cluster', () => {
      component.cluster.set(null);
      expect(component.totalCapacity()).toBeNull();
    });

    it('should return empty stewardDistribution when no cluster', () => {
      component.cluster.set(null);
      expect(component.stewardDistribution()).toEqual([]);
    });
  });

  describe('format helpers', () => {
    it('should format bytes in GB', () => {
      expect(component.formatBytes(500)).toBe('500.0 GB');
    });

    it('should format bytes in TB for large values', () => {
      expect(component.formatBytes(2048)).toBe('2.0 TB');
    });

    it('should handle null bytes', () => {
      expect(component.formatBytes(null)).toBe('-');
    });

    it('should format percent', () => {
      expect(component.formatPercent(0.75)).toBe('75.0%');
    });

    it('should handle null percent', () => {
      expect(component.formatPercent(null)).toBe('-');
    });

    it('should format number with locale', () => {
      expect(component.formatNumber(1000)).toContain('1');
    });

    it('should handle null number', () => {
      expect(component.formatNumber(null)).toBe('-');
    });

    it('should format time in seconds', () => {
      expect(component.formatTime(30)).toBe('30s ago');
    });

    it('should format time in minutes', () => {
      expect(component.formatTime(120)).toBe('2m ago');
    });

    it('should format time in hours', () => {
      expect(component.formatTime(7200)).toBe('2h ago');
    });

    it('should handle null time', () => {
      expect(component.formatTime(null)).toBe('Never');
    });
  });

  describe('getReachCount', () => {
    beforeEach(fakeAsync(() => {
      fixture.detectChanges();
      tick();
      discardPeriodicTasks();
    }));

    it('should return count for valid reach level', () => {
      expect(component.getReachCount(0)).toBe(50); // private
      expect(component.getReachCount(1)).toBe(100); // invited
      expect(component.getReachCount(3)).toBe(300); // neighborhood
    });

    it('should return 0 for invalid reach level', () => {
      expect(component.getReachCount(99)).toBe(0);
    });

    it('should return 0 when no cluster', () => {
      component.cluster.set(null);
      expect(component.getReachCount(0)).toBe(0);
    });
  });

  describe('refresh', () => {
    it('should reload data on refresh', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      mockAdminService.getNodes.calls.reset();

      component.refresh();
      tick();

      expect(mockAdminService.getNodes).toHaveBeenCalled();

      discardPeriodicTasks();
    }));
  });

  describe('error handling', () => {
    it('should set error signal on load failure', fakeAsync(() => {
      mockAdminService.getNodes.and.returnValue(of({ total: 0, byStatus: {} as any, nodes: [] }));
      mockAdminService.getClusterMetrics.and.throwError('Network error');

      fixture.detectChanges();
      tick();

      expect(component.error()).toBeTruthy();

      discardPeriodicTasks();
    }));
  });

  describe('helper functions exposure', () => {
    it('should expose statusColor helper', () => {
      expect(typeof component.statusColor).toBe('function');
      expect(component.statusColor('online')).toBe('text-green-600');
    });

    it('should expose tierColor helper', () => {
      expect(typeof component.tierColor).toBe('function');
      expect(component.tierColor('pioneer')).toBe('text-purple-600');
    });

    it('should expose reachLevelName helper', () => {
      expect(typeof component.reachLevelName).toBe('function');
      expect(component.reachLevelName(0)).toBe('Private');
    });
  });
});

// =============================================================================
// Test Helpers
// =============================================================================

function createMockNode(nodeId: string, status: string, combinedScore: number): NodeDetails {
  return {
    nodeId,
    status: status as any,
    natsProvisioned: true,
    lastHeartbeatSecsAgo: 30,
    cpuCores: 4,
    memoryGb: 16,
    storageTb: 1,
    bandwidthMbps: 100,
    cpuUsagePercent: 40,
    memoryUsagePercent: 55,
    storageUsageTb: 0.5,
    activeConnections: 30,
    custodiedContentGb: 100,
    stewardTier: 'guardian',
    maxReachLevel: 5,
    activeReachLevels: [1, 2, 3],
    trustScore: combinedScore,
    humansServed: 100,
    contentCustodied: 500,
    successfulDeliveries: 1000,
    failedDeliveries: 10,
    deliverySuccessRate: 0.99,
    impactScore: combinedScore - 0.1,
    combinedScore,
    region: 'us-west',
  };
}
