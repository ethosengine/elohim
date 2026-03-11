import { TestBed, fakeAsync, tick, discardPeriodicTasks } from '@angular/core/testing';
import { provideHttpClientTesting, HttpTestingController } from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';

import { of, take } from 'rxjs';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { IdentityService } from '@app/imagodei/services/identity.service';

import { RunningContextService, RegisteredNode, ComputeContext } from './running-context.service';
import { vi, Mock } from 'vitest';

describe('RunningContextService', () => {
  let service: RunningContextService;
  let httpMock: HttpTestingController;
  let mockHolochainClient: any;
  let mockIdentityService: any;

  beforeEach(() => {
    mockHolochainClient = {
      callZome: vi.fn(),
      state: vi.fn().mockReturnValue('disconnected'),
    };

    mockIdentityService = {
      mode: vi.fn().mockReturnValue('anonymous'),
    };

    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        RunningContextService,
        { provide: HolochainClientService, useValue: mockHolochainClient },
        { provide: IdentityService, useValue: mockIdentityService },
      ],
    });

    service = TestBed.inject(RunningContextService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
    service.stopPeriodicDetection();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('initial state', () => {
    it('should have default context with no registered nodes', () => {
      const context = service.context();
      expect(context.hasRegisteredNodes).toBe(false);
      expect(context.registeredNodes).toEqual([]);
      expect(context.primaryNode).toBeNull();
      expect(context.totalNodes).toBe(0);
      expect(context.onlineNodes).toBe(0);
      expect(context.hasDoorwayCapableNode).toBe(false);
      expect(context.doorwayNodes).toEqual([]);
    });

    it('should have computed signals that return correct values', () => {
      expect(service.hasRegisteredNodes()).toBe(false);
      expect(service.registeredNodes()).toEqual([]);
      expect(service.primaryNode()).toBeNull();
      expect(service.hasDoorwayCapableNode()).toBe(false);
      expect(service.doorwayNodes()).toEqual([]);
    });
  });

  describe('detect()', () => {
    it('should return empty context when user is not authenticated', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('anonymous');

      const context = await service.detect();

      expect(context.hasRegisteredNodes).toBe(false);
      expect(context.registeredNodes).toEqual([]);
      expect(context.totalNodes).toBe(0);
    });

    it('should query holochain for nodes when user is hosted', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockHolochainClient.callZome.mockReturnValue(
        Promise.resolve({
          success: true,
          data: [
            {
              node_id: 'node-1',
              display_name: 'My Holoport',
              node_type: 'holoport',
              status: 'online',
              last_heartbeat: '2025-01-01T00:00:00Z',
              doorway_url: null,
            },
          ],
        })
      );

      const context = await service.detect();

      expect(mockHolochainClient.callZome).toHaveBeenCalledWith({
        zomeName: 'node_registry_coordinator',
        fnName: 'get_my_nodes',
        payload: null,
      });
      expect(context.hasRegisteredNodes).toBe(true);
      expect(context.totalNodes).toBe(1);
      expect(context.registeredNodes[0].nodeId).toBe('node-1');
    });

    it('should query holochain for nodes when user is steward', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('steward');
      mockHolochainClient.callZome.mockReturnValue(Promise.resolve({ success: true, data: [] }));

      const context = await service.detect();

      expect(mockHolochainClient.callZome).toHaveBeenCalled();
      expect(context.hasRegisteredNodes).toBe(false);
    });

    it('should handle holochain errors gracefully', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockHolochainClient.callZome.mockReturnValue(Promise.reject(new Error('Connection failed')));

      const context = await service.detect();

      expect(context.hasRegisteredNodes).toBe(false);
      expect(context.registeredNodes).toEqual([]);
    });

    it('should handle empty holochain response', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockHolochainClient.callZome.mockReturnValue(Promise.resolve({ success: false, data: null }));

      const context = await service.detect();

      expect(context.hasRegisteredNodes).toBe(false);
    });
  });

  describe('node type detection', () => {
    it('should identify holoport as primary node', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockHolochainClient.callZome.mockReturnValue(
        Promise.resolve({
          success: true,
          data: [
            { node_id: 'cloud-1', node_type: 'cloud', status: 'online' },
            { node_id: 'holoport-1', node_type: 'holoport', status: 'online' },
          ],
        })
      );

      const context = await service.detect();

      expect(context.primaryNode?.nodeId).toBe('holoport-1');
    });

    it('should identify holoport-plus as primary node', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockHolochainClient.callZome.mockReturnValue(
        Promise.resolve({
          success: true,
          data: [
            { node_id: 'self-hosted-1', node_type: 'self-hosted', status: 'online' },
            { node_id: 'holoport-plus-1', node_type: 'holoport-plus', status: 'online' },
          ],
        })
      );

      const context = await service.detect();

      expect(context.primaryNode?.nodeId).toBe('holoport-plus-1');
    });

    it('should use first node as primary when no holoport', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockHolochainClient.callZome.mockReturnValue(
        Promise.resolve({
          success: true,
          data: [
            { node_id: 'cloud-1', node_type: 'cloud', status: 'online' },
            { node_id: 'self-hosted-1', node_type: 'self-hosted', status: 'online' },
          ],
        })
      );

      const context = await service.detect();

      expect(context.primaryNode?.nodeId).toBe('cloud-1');
    });
  });

  describe('doorway capability detection', () => {
    it('should mark holoport as doorway capable', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockHolochainClient.callZome.mockReturnValue(
        Promise.resolve({
          success: true,
          data: [{ node_id: 'hp-1', node_type: 'holoport', status: 'online' }],
        })
      );

      const context = await service.detect();

      expect(context.hasDoorwayCapableNode).toBe(true);
      expect(context.doorwayNodes.length).toBe(1);
      expect(context.doorwayNodes[0].hasDoorway).toBe(true);
    });

    it('should mark holoport-plus as doorway capable', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockHolochainClient.callZome.mockReturnValue(
        Promise.resolve({
          success: true,
          data: [{ node_id: 'hp-plus-1', node_type: 'holoport-plus', status: 'online' }],
        })
      );

      const context = await service.detect();

      expect(context.hasDoorwayCapableNode).toBe(true);
    });

    it('should mark self-hosted with doorway_url as doorway capable', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockHolochainClient.callZome.mockReturnValue(
        Promise.resolve({
          success: true,
          data: [
            {
              node_id: 'sh-1',
              node_type: 'self-hosted',
              status: 'online',
              doorway_url: 'https://my-doorway.example.com',
            },
          ],
        })
      );

      const context = await service.detect();

      expect(context.hasDoorwayCapableNode).toBe(true);
      expect(context.doorwayNodes[0].doorwayUrl).toBe('https://my-doorway.example.com');
    });

    it('should NOT mark self-hosted without doorway_url as doorway capable', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockHolochainClient.callZome.mockReturnValue(
        Promise.resolve({
          success: true,
          data: [
            { node_id: 'sh-1', node_type: 'self-hosted', status: 'online', doorway_url: null },
          ],
        })
      );

      const context = await service.detect();

      expect(context.hasDoorwayCapableNode).toBe(false);
      expect(context.doorwayNodes.length).toBe(0);
    });
  });

  describe('status mapping', () => {
    it('should map online status correctly', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockHolochainClient.callZome.mockReturnValue(
        Promise.resolve({
          success: true,
          data: [{ node_id: 'n1', status: 'online' }],
        })
      );

      const context = await service.detect();
      expect(context.registeredNodes[0].status).toBe('online');
    });

    it('should map offline status correctly', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockHolochainClient.callZome.mockReturnValue(
        Promise.resolve({
          success: true,
          data: [{ node_id: 'n1', status: 'offline' }],
        })
      );

      const context = await service.detect();
      expect(context.registeredNodes[0].status).toBe('offline');
    });

    it('should map degraded status correctly', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockHolochainClient.callZome.mockReturnValue(
        Promise.resolve({
          success: true,
          data: [{ node_id: 'n1', status: 'degraded' }],
        })
      );

      const context = await service.detect();
      expect(context.registeredNodes[0].status).toBe('degraded');
    });

    it('should map unknown status for unrecognized values', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockHolochainClient.callZome.mockReturnValue(
        Promise.resolve({
          success: true,
          data: [{ node_id: 'n1', status: 'weird-status' }],
        })
      );

      const context = await service.detect();
      expect(context.registeredNodes[0].status).toBe('unknown');
    });

    it('should handle undefined status', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockHolochainClient.callZome.mockReturnValue(
        Promise.resolve({
          success: true,
          data: [{ node_id: 'n1' }], // No status field
        })
      );

      const context = await service.detect();
      expect(context.registeredNodes[0].status).toBe('unknown');
    });
  });

  describe('online node counting', () => {
    it('should count online nodes correctly', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockHolochainClient.callZome.mockReturnValue(
        Promise.resolve({
          success: true,
          data: [
            { node_id: 'n1', status: 'online' },
            { node_id: 'n2', status: 'online' },
            { node_id: 'n3', status: 'offline' },
            { node_id: 'n4', status: 'degraded' },
          ],
        })
      );

      const context = await service.detect();

      expect(context.totalNodes).toBe(4);
      expect(context.onlineNodes).toBe(2);
    });
  });

  describe('isHolochainNative', () => {
    it('should return true when user has registered nodes', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockHolochainClient.callZome.mockReturnValue(
        Promise.resolve({
          success: true,
          data: [{ node_id: 'n1', status: 'online' }],
        })
      );

      await service.detect();

      expect(service.isHolochainNative()).toBe(true);
    });

    it('should return true when holochain client is connected', () => {
      (mockHolochainClient.state as Mock).mockReturnValue('connected');

      expect(service.isHolochainNative()).toBe(true);
    });

    it('should return false when no nodes and not connected', () => {
      (mockHolochainClient.state as Mock).mockReturnValue('disconnected');

      expect(service.isHolochainNative()).toBe(false);
    });
  });

  describe('periodic detection', () => {
    it('should start periodic detection and run initial detect', fakeAsync(() => {
      (mockIdentityService.mode as Mock).mockReturnValue('anonymous');

      service.startPeriodicDetection();

      expect(service.context().detectedAt).toBeTruthy();
      discardPeriodicTasks();
    }));

    it('should not start multiple detection intervals', fakeAsync(() => {
      (mockIdentityService.mode as Mock).mockReturnValue('anonymous');

      service.startPeriodicDetection();
      service.startPeriodicDetection(); // Second call should be ignored

      discardPeriodicTasks();
    }));

    it('should refresh detection every 60 seconds', fakeAsync(() => {
      (mockIdentityService.mode as Mock).mockReturnValue('anonymous');

      service.startPeriodicDetection();
      const initialDetectedAt = service.context().detectedAt;

      tick(60000); // Wait 60 seconds

      expect(service.context().detectedAt.getTime()).toBeGreaterThanOrEqual(
        initialDetectedAt.getTime()
      );
      discardPeriodicTasks();
    }));

    it('should stop periodic detection', fakeAsync(() => {
      (mockIdentityService.mode as Mock).mockReturnValue('anonymous');

      service.startPeriodicDetection();
      service.stopPeriodicDetection();

      // Should not throw after stopping
      tick(120000);
    }));

    it('should handle stopPeriodicDetection when not started', () => {
      expect(() => service.stopPeriodicDetection()).not.toThrow();
    });
  });

  describe('context$ observable', () => {
    it('should emit current context immediately', () =>
      new Promise<void>(done => {
        service.context$.pipe(take(1)).subscribe(context => {
          expect(context).toBeTruthy();
          expect(context.hasRegisteredNodes).toBe(false);
          done();
        });
      }));
  });

  describe('display name generation', () => {
    it('should use display_name when provided', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockHolochainClient.callZome.mockReturnValue(
        Promise.resolve({
          success: true,
          data: [{ node_id: 'node-12345678', display_name: 'My Custom Name', status: 'online' }],
        })
      );

      const context = await service.detect();

      expect(context.registeredNodes[0].displayName).toBe('My Custom Name');
    });

    it('should truncate node_id when no display_name', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockHolochainClient.callZome.mockReturnValue(
        Promise.resolve({
          success: true,
          data: [{ node_id: 'node-12345678-abcdefgh', status: 'online' }],
        })
      );

      const context = await service.detect();

      expect(context.registeredNodes[0].displayName).toBe('node-123');
    });
  });
});
