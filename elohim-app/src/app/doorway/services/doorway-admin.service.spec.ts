import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import {
  HttpClientTestingModule,
  HttpTestingController,
} from '@angular/common/http/testing';

import { DoorwayAdminService, ConnectionState } from './doorway-admin.service';
import {
  NodesResponse,
  NodeDetails,
  ClusterMetrics,
  ResourceSummary,
  CustodianNetwork,
  NodeSnapshot,
  ClusterSnapshot,
} from '../models/doorway.model';

describe('DoorwayAdminService', () => {
  let service: DoorwayAdminService;
  let httpMock: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [DoorwayAdminService],
    });

    service = TestBed.inject(DoorwayAdminService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
    service.disconnect();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('initial state', () => {
    it('should have disconnected connection state', () => {
      expect(service.connectionState()).toBe('disconnected');
    });

    it('should have isConnected as false', () => {
      expect(service.isConnected()).toBeFalse();
    });

    it('should have empty nodes array', () => {
      expect(service.nodes()).toEqual([]);
    });

    it('should have null cluster', () => {
      expect(service.cluster()).toBeNull();
    });
  });

  describe('REST API: getNodes()', () => {
    it('should fetch nodes successfully', () => {
      const mockResponse: NodesResponse = {
        total: 2,
        byStatus: {
          online: 2,
          degraded: 0,
          offline: 0,
          failed: 0,
          discovering: 0,
          registering: 0,
        },
        nodes: [
          createMockNodeDetails('node-1', 'online'),
          createMockNodeDetails('node-2', 'online'),
        ],
      };

      service.getNodes().subscribe(response => {
        expect(response.total).toBe(2);
        expect(response.nodes.length).toBe(2);
      });

      const req = httpMock.expectOne('/admin/nodes');
      expect(req.request.method).toBe('GET');
      req.flush(mockResponse);
    });

    it('should return fallback on error after retries', fakeAsync(() => {
      let result: NodesResponse | undefined;
      service.getNodes().subscribe(response => {
        result = response;
      });

      // Initial request + 2 retries = 3 total
      for (let i = 0; i < 3; i++) {
        const req = httpMock.expectOne('/admin/nodes');
        req.error(new ProgressEvent('error'), { status: 500 });
        tick(0);
      }

      expect(result).toBeDefined();
      expect(result!.total).toBe(0);
      expect(result!.nodes).toEqual([]);
    }));
  });

  describe('REST API: getNode()', () => {
    it('should fetch specific node', () => {
      const mockNode = createMockNodeDetails('node-123', 'online');

      service.getNode('node-123').subscribe(node => {
        expect(node?.nodeId).toBe('node-123');
      });

      const req = httpMock.expectOne('/admin/nodes/node-123');
      expect(req.request.method).toBe('GET');
      req.flush(mockNode);
    });

    it('should return null on error after retries', fakeAsync(() => {
      let result: NodeDetails | null | undefined;
      service.getNode('nonexistent').subscribe(node => {
        result = node;
      });

      // Initial + 2 retries
      for (let i = 0; i < 3; i++) {
        const req = httpMock.expectOne('/admin/nodes/nonexistent');
        req.error(new ProgressEvent('error'), { status: 404 });
        tick(0);
      }

      expect(result).toBeNull();
    }));
  });

  describe('REST API: getClusterMetrics()', () => {
    it('should fetch cluster metrics', () => {
      const mockCluster: ClusterMetrics = {
        region: 'us-west',
        totalNodes: 10,
        onlineNodes: 8,
        healthRatio: 0.8,
        totalCpuCores: 100,
        totalMemoryGb: 500,
        totalStorageTb: 50,
        totalBandwidthMbps: 10000,
        avgCpuUsagePercent: 45,
        avgMemoryUsagePercent: 60,
        totalStorageUsedTb: 25,
        totalActiveConnections: 500,
        totalCustodiedContentGb: 1000,
        avgTrustScore: 0.85,
        avgImpactScore: 0.75,
        totalHumansServed: 5000,
        totalContentCustodied: 10000,
        clusterDeliverySuccessRate: 0.99,
        stewardCounts: { pioneers: 2, stewards: 3, guardians: 3, caretakers: 2 },
        reachCoverage: {
          private: 100,
          invited: 200,
          local: 500,
          neighborhood: 800,
          municipal: 1000,
          bioregional: 500,
          regional: 200,
          commons: 100,
        },
      };

      service.getClusterMetrics().subscribe(cluster => {
        expect(cluster?.totalNodes).toBe(10);
        expect(cluster?.healthRatio).toBe(0.8);
      });

      const req = httpMock.expectOne('/admin/cluster');
      req.flush(mockCluster);
    });

    it('should return null on error after retries', fakeAsync(() => {
      let result: ClusterMetrics | null | undefined;
      service.getClusterMetrics().subscribe(cluster => {
        result = cluster;
      });

      for (let i = 0; i < 3; i++) {
        const req = httpMock.expectOne('/admin/cluster');
        req.error(new ProgressEvent('error'), { status: 500 });
        tick(0);
      }

      expect(result).toBeNull();
    }));
  });

  describe('REST API: getResources()', () => {
    it('should fetch resource summary', () => {
      const mockResources: ResourceSummary = {
        cpu: { total: 100, used: 45, available: 55, utilizationPercent: 45 },
        memory: { total: 500, used: 300, available: 200, utilizationPercent: 60 },
        storage: {
          totalTb: 50,
          usedTb: 25,
          availableTb: 25,
          utilizationPercent: 50,
          custodiedContentGb: 1000,
        },
        bandwidth: { totalMbps: 10000, activeConnections: 500, avgBandwidthPerConnectionMbps: 20 },
        cache: { entries: 1000, hits: 9000, misses: 1000, hitRate: 0.9 },
      };

      service.getResources().subscribe(resources => {
        expect(resources?.cpu.utilizationPercent).toBe(45);
        expect(resources?.cache.hitRate).toBe(0.9);
      });

      const req = httpMock.expectOne('/admin/resources');
      req.flush(mockResources);
    });

    it('should return null on error after retries', fakeAsync(() => {
      let result: ResourceSummary | null | undefined;
      service.getResources().subscribe(resources => {
        result = resources;
      });

      for (let i = 0; i < 3; i++) {
        const req = httpMock.expectOne('/admin/resources');
        req.error(new ProgressEvent('error'), { status: 500 });
        tick(0);
      }

      expect(result).toBeNull();
    }));
  });

  describe('REST API: getCustodians()', () => {
    it('should fetch custodian network', () => {
      const mockCustodians: CustodianNetwork = {
        registeredCustodians: 50,
        trackedBlobs: 10000,
        totalCommitments: 25000,
        totalProbes: 5000,
        successfulProbes: 4800,
        probeSuccessRate: 0.96,
        totalSelections: 10000,
        healthyCustodians: 48,
      };

      service.getCustodians().subscribe(custodians => {
        expect(custodians?.registeredCustodians).toBe(50);
        expect(custodians?.probeSuccessRate).toBe(0.96);
      });

      const req = httpMock.expectOne('/admin/custodians');
      req.flush(mockCustodians);
    });

    it('should return null on error after retries', fakeAsync(() => {
      let result: CustodianNetwork | null | undefined;
      service.getCustodians().subscribe(custodians => {
        result = custodians;
      });

      for (let i = 0; i < 3; i++) {
        const req = httpMock.expectOne('/admin/custodians');
        req.error(new ProgressEvent('error'), { status: 500 });
        tick(0);
      }

      expect(result).toBeNull();
    }));

    it('should log warning on 503 after retries (orchestrator not enabled)', fakeAsync(() => {
      spyOn(console, 'warn');

      service.getCustodians().subscribe();

      // 503 will trigger retries before logging
      for (let i = 0; i < 3; i++) {
        const req = httpMock.expectOne('/admin/custodians');
        req.error(new ProgressEvent('error'), { status: 503 });
        tick(0);
      }

      expect(console.warn).toHaveBeenCalledWith(
        jasmine.stringContaining('Orchestrator not enabled')
      );
    }));
  });

  describe('WebSocket: connection management', () => {
    it('should not reconnect if already connected', () => {
      // First connect
      service.connect();
      expect(service.connectionState()).toBe('connecting');

      // Try to connect again - should be no-op
      service.connect();
      expect(service.connectionState()).toBe('connecting');
    });

    it('should set state to disconnected on disconnect', () => {
      service.disconnect();
      expect(service.connectionState()).toBe('disconnected');
    });

    it('should handle disconnect when not connected', () => {
      expect(() => service.disconnect()).not.toThrow();
      expect(service.connectionState()).toBe('disconnected');
    });
  });

  describe('WebSocket: ping', () => {
    it('should not throw when ping called while disconnected', () => {
      expect(() => service.ping()).not.toThrow();
    });
  });

  describe('messages$ observable', () => {
    it('should return observable', () => {
      expect(service.messages$).toBeTruthy();
      expect(typeof service.messages$.subscribe).toBe('function');
    });
  });

  describe('handleMessage()', () => {
    // Access private method for testing - use unknown to allow test message shapes
    const callHandleMessage = (msg: unknown) => {
      (service as any).handleMessage(msg);
    };

    it('should handle initial_state message', () => {
      const mockNodes: NodeSnapshot[] = [
        { nodeId: 'node-1', status: 'online', combinedScore: 0.85, stewardTier: 'guardian', trustScore: 0.9, lastHeartbeatSecsAgo: 30 },
        { nodeId: 'node-2', status: 'degraded', combinedScore: 0.7, stewardTier: 'steward', trustScore: 0.8, lastHeartbeatSecsAgo: 45 },
      ];
      const mockCluster: ClusterSnapshot = {
        onlineNodes: 5,
        totalNodes: 6,
        healthRatio: 0.83,
        avgTrustScore: 0.9,
        avgImpactScore: 0.8,
      };

      callHandleMessage({
        type: 'initial_state',
        nodes: mockNodes,
        cluster: mockCluster,
      });

      expect(service.nodes()).toEqual(mockNodes);
      expect(service.cluster()).toEqual(mockCluster);
    });

    it('should handle node_update message - update existing node', () => {
      // Set initial state
      (service as any)._nodes.set([
        { nodeId: 'node-1', status: 'online', combinedScore: 0.85, stewardTier: 'guardian', trustScore: 0.9, lastHeartbeatSecsAgo: 30 },
        { nodeId: 'node-2', status: 'online', combinedScore: 0.9, stewardTier: 'steward', trustScore: 0.85, lastHeartbeatSecsAgo: 25 },
      ]);

      // Update node-1
      callHandleMessage({
        type: 'node_update',
        nodeId: 'node-1',
        status: 'degraded',
        combinedScore: 0.6,
      });

      const nodes = service.nodes();
      expect(nodes[0].status).toBe('degraded');
      expect(nodes[0].combinedScore).toBe(0.6);
      expect(nodes[1].status).toBe('online'); // Unchanged
    });

    it('should handle node_update message - node not found', () => {
      (service as any)._nodes.set([
        { nodeId: 'node-1', status: 'online', combinedScore: 0.85, stewardTier: 'guardian', trustScore: 0.9, lastHeartbeatSecsAgo: 30 },
      ]);

      // Try to update non-existent node
      callHandleMessage({
        type: 'node_update',
        nodeId: 'node-999',
        status: 'offline',
        combinedScore: 0,
      });

      // Should not throw, nodes unchanged
      expect(service.nodes().length).toBe(1);
      expect(service.nodes()[0].nodeId).toBe('node-1');
    });

    it('should handle cluster_update message', () => {
      callHandleMessage({
        type: 'cluster_update',
        onlineNodes: 10,
        totalNodes: 12,
        healthRatio: 0.833,
        avgTrustScore: 0.92,
        avgImpactScore: 0.78,
      });

      const cluster = service.cluster();
      expect(cluster?.onlineNodes).toBe(10);
      expect(cluster?.totalNodes).toBe(12);
      expect(cluster?.healthRatio).toBe(0.833);
      expect(cluster?.avgTrustScore).toBe(0.92);
      expect(cluster?.avgImpactScore).toBe(0.78);
    });

    it('should handle heartbeat message', () => {
      spyOn(console, 'debug');

      callHandleMessage({ type: 'heartbeat' });

      expect(console.debug).toHaveBeenCalledWith('[DoorwayAdmin] Heartbeat received');
    });

    it('should handle pong message', () => {
      // Pong is a no-op, just ensure it doesn't throw
      expect(() => callHandleMessage({ type: 'pong' })).not.toThrow();
    });

    it('should handle error message', () => {
      spyOn(console, 'error');

      callHandleMessage({
        type: 'error',
        message: 'Test error message',
      });

      expect(console.error).toHaveBeenCalledWith(
        '[DoorwayAdmin] Server error:',
        'Test error message'
      );
    });
  });

  describe('isDashboardMessage()', () => {
    const callIsDashboardMessage = (msg: unknown): boolean => {
      return (service as any).isDashboardMessage(msg);
    };

    it('should return true for valid dashboard message', () => {
      expect(callIsDashboardMessage({ type: 'heartbeat' })).toBeTrue();
      expect(callIsDashboardMessage({ type: 'pong' })).toBeTrue();
      expect(callIsDashboardMessage({ type: 'error', message: 'test' })).toBeTrue();
    });

    it('should return false for null', () => {
      expect(callIsDashboardMessage(null)).toBeFalse();
    });

    it('should return false for non-object', () => {
      expect(callIsDashboardMessage('string')).toBeFalse();
      expect(callIsDashboardMessage(123)).toBeFalse();
      expect(callIsDashboardMessage(undefined)).toBeFalse();
    });

    it('should return false for object without type', () => {
      expect(callIsDashboardMessage({})).toBeFalse();
      expect(callIsDashboardMessage({ data: 'test' })).toBeFalse();
    });
  });
});

// =============================================================================
// Test Helpers
// =============================================================================

function createMockNodeDetails(nodeId: string, status: string): NodeDetails {
  return {
    nodeId,
    status: status as any,
    natsProvisioned: true,
    lastHeartbeatSecsAgo: 30,
    cpuCores: 4,
    memoryGb: 16,
    storageTb: 1,
    bandwidthMbps: 100,
    cpuUsagePercent: 25,
    memoryUsagePercent: 50,
    storageUsageTb: 0.5,
    activeConnections: 10,
    custodiedContentGb: 50,
    stewardTier: 'guardian',
    maxReachLevel: 5,
    activeReachLevels: [1, 2, 3],
    trustScore: 0.9,
    humansServed: 100,
    contentCustodied: 500,
    successfulDeliveries: 1000,
    failedDeliveries: 10,
    deliverySuccessRate: 0.99,
    impactScore: 0.8,
    combinedScore: 0.85,
    region: 'us-west',
  };
}
