import { TestBed, fakeAsync, tick, discardPeriodicTasks } from '@angular/core/testing';
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';

import { of, take } from 'rxjs';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { IdentityService } from '@app/imagodei/services/identity.service';

import { RunningContextService, RegisteredNode, ComputeContext } from './running-context.service';

describe('RunningContextService', () => {
  let service: RunningContextService;
  let httpMock: HttpTestingController;
  let mockHolochainClient: jasmine.SpyObj<HolochainClientService>;
  let mockIdentityService: jasmine.SpyObj<IdentityService>;

  beforeEach(() => {
    mockHolochainClient = jasmine.createSpyObj('HolochainClientService', ['callZome'], {
      state: jasmine.createSpy('state').and.returnValue('disconnected'),
    });

    mockIdentityService = jasmine.createSpyObj('IdentityService', [], {
      mode: jasmine.createSpy('mode').and.returnValue('anonymous'),
    });

    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [
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
      expect(context.hasRegisteredNodes).toBeFalse();
      expect(context.registeredNodes).toEqual([]);
      expect(context.primaryNode).toBeNull();
      expect(context.totalNodes).toBe(0);
      expect(context.onlineNodes).toBe(0);
      expect(context.hasDoorwayCapableNode).toBeFalse();
      expect(context.doorwayNodes).toEqual([]);
    });

    it('should have computed signals that return correct values', () => {
      expect(service.hasRegisteredNodes()).toBeFalse();
      expect(service.registeredNodes()).toEqual([]);
      expect(service.primaryNode()).toBeNull();
      expect(service.hasDoorwayCapableNode()).toBeFalse();
      expect(service.doorwayNodes()).toEqual([]);
    });
  });

  describe('detect()', () => {
    it('should return empty context when user is not authenticated', async () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('anonymous');

      const context = await service.detect();

      expect(context.hasRegisteredNodes).toBeFalse();
      expect(context.registeredNodes).toEqual([]);
      expect(context.totalNodes).toBe(0);
    });

    it('should query holochain for nodes when user is hosted', async () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockHolochainClient.callZome.and.returnValue(
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
      expect(context.hasRegisteredNodes).toBeTrue();
      expect(context.totalNodes).toBe(1);
      expect(context.registeredNodes[0].nodeId).toBe('node-1');
    });

    it('should query holochain for nodes when user is steward', async () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('steward');
      mockHolochainClient.callZome.and.returnValue(Promise.resolve({ success: true, data: [] }));

      const context = await service.detect();

      expect(mockHolochainClient.callZome).toHaveBeenCalled();
      expect(context.hasRegisteredNodes).toBeFalse();
    });

    it('should handle holochain errors gracefully', async () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockHolochainClient.callZome.and.returnValue(Promise.reject(new Error('Connection failed')));

      const context = await service.detect();

      expect(context.hasRegisteredNodes).toBeFalse();
      expect(context.registeredNodes).toEqual([]);
    });

    it('should handle empty holochain response', async () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockHolochainClient.callZome.and.returnValue(Promise.resolve({ success: false, data: null }));

      const context = await service.detect();

      expect(context.hasRegisteredNodes).toBeFalse();
    });
  });

  describe('node type detection', () => {
    it('should identify holoport as primary node', async () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockHolochainClient.callZome.and.returnValue(
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
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockHolochainClient.callZome.and.returnValue(
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
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockHolochainClient.callZome.and.returnValue(
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
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({
          success: true,
          data: [{ node_id: 'hp-1', node_type: 'holoport', status: 'online' }],
        })
      );

      const context = await service.detect();

      expect(context.hasDoorwayCapableNode).toBeTrue();
      expect(context.doorwayNodes.length).toBe(1);
      expect(context.doorwayNodes[0].hasDoorway).toBeTrue();
    });

    it('should mark holoport-plus as doorway capable', async () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({
          success: true,
          data: [{ node_id: 'hp-plus-1', node_type: 'holoport-plus', status: 'online' }],
        })
      );

      const context = await service.detect();

      expect(context.hasDoorwayCapableNode).toBeTrue();
    });

    it('should mark self-hosted with doorway_url as doorway capable', async () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockHolochainClient.callZome.and.returnValue(
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

      expect(context.hasDoorwayCapableNode).toBeTrue();
      expect(context.doorwayNodes[0].doorwayUrl).toBe('https://my-doorway.example.com');
    });

    it('should NOT mark self-hosted without doorway_url as doorway capable', async () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({
          success: true,
          data: [
            { node_id: 'sh-1', node_type: 'self-hosted', status: 'online', doorway_url: null },
          ],
        })
      );

      const context = await service.detect();

      expect(context.hasDoorwayCapableNode).toBeFalse();
      expect(context.doorwayNodes.length).toBe(0);
    });
  });

  describe('status mapping', () => {
    it('should map online status correctly', async () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({
          success: true,
          data: [{ node_id: 'n1', status: 'online' }],
        })
      );

      const context = await service.detect();
      expect(context.registeredNodes[0].status).toBe('online');
    });

    it('should map offline status correctly', async () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({
          success: true,
          data: [{ node_id: 'n1', status: 'offline' }],
        })
      );

      const context = await service.detect();
      expect(context.registeredNodes[0].status).toBe('offline');
    });

    it('should map degraded status correctly', async () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({
          success: true,
          data: [{ node_id: 'n1', status: 'degraded' }],
        })
      );

      const context = await service.detect();
      expect(context.registeredNodes[0].status).toBe('degraded');
    });

    it('should map unknown status for unrecognized values', async () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({
          success: true,
          data: [{ node_id: 'n1', status: 'weird-status' }],
        })
      );

      const context = await service.detect();
      expect(context.registeredNodes[0].status).toBe('unknown');
    });

    it('should handle undefined status', async () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockHolochainClient.callZome.and.returnValue(
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
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockHolochainClient.callZome.and.returnValue(
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
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({
          success: true,
          data: [{ node_id: 'n1', status: 'online' }],
        })
      );

      await service.detect();

      expect(service.isHolochainNative()).toBeTrue();
    });

    it('should return true when holochain client is connected', () => {
      (mockHolochainClient.state as jasmine.Spy).and.returnValue('connected');

      expect(service.isHolochainNative()).toBeTrue();
    });

    it('should return false when no nodes and not connected', () => {
      (mockHolochainClient.state as jasmine.Spy).and.returnValue('disconnected');

      expect(service.isHolochainNative()).toBeFalse();
    });
  });

  describe('periodic detection', () => {
    it('should start periodic detection and run initial detect', fakeAsync(() => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('anonymous');

      service.startPeriodicDetection();

      expect(service.context().detectedAt).toBeTruthy();
      discardPeriodicTasks();
    }));

    it('should not start multiple detection intervals', fakeAsync(() => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('anonymous');

      service.startPeriodicDetection();
      service.startPeriodicDetection(); // Second call should be ignored

      discardPeriodicTasks();
    }));

    it('should refresh detection every 60 seconds', fakeAsync(() => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('anonymous');

      service.startPeriodicDetection();
      const initialDetectedAt = service.context().detectedAt;

      tick(60000); // Wait 60 seconds

      expect(service.context().detectedAt.getTime()).toBeGreaterThan(initialDetectedAt.getTime());
      discardPeriodicTasks();
    }));

    it('should stop periodic detection', fakeAsync(() => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('anonymous');

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
    it('should emit current context immediately', done => {
      service.context$.pipe(take(1)).subscribe(context => {
        expect(context).toBeTruthy();
        expect(context.hasRegisteredNodes).toBeFalse();
        done();
      });
    });
  });

  describe('display name generation', () => {
    it('should use display_name when provided', async () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({
          success: true,
          data: [{ node_id: 'node-12345678', display_name: 'My Custom Name', status: 'online' }],
        })
      );

      const context = await service.detect();

      expect(context.registeredNodes[0].displayName).toBe('My Custom Name');
    });

    it('should truncate node_id when no display_name', async () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockHolochainClient.callZome.and.returnValue(
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
