/**
 * Offline Operation Queue Service Tests
 */

import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { OfflineOperationQueueService, OfflineOperation } from './offline-operation-queue.service';
import { HolochainClientService } from './holochain-client.service';

describe('OfflineOperationQueueService', () => {
  let service: OfflineOperationQueueService;
  let mockHolochainClient: jasmine.SpyObj<HolochainClientService>;

  beforeEach(() => {
    mockHolochainClient = jasmine.createSpyObj('HolochainClientService', [
      'isConnected',
      'callZome',
    ]);

    TestBed.configureTestingModule({
      providers: [
        OfflineOperationQueueService,
        { provide: HolochainClientService, useValue: mockHolochainClient },
      ],
    });

    service = TestBed.inject(OfflineOperationQueueService);
  });

  describe('enqueue', () => {
    it('should enqueue an operation', () => {
      const operationId = service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
        payload: { title: 'Test' },
        maxRetries: 3,
        description: 'Create test content',
      });

      expect(typeof operationId).toBe('string');
      expect(operationId.startsWith('op-')).toBe(true);
      expect(service.getQueueSize()).toBe(1);
    });

    it('should set default maxRetries', () => {
      service.enqueue({
        type: 'create',
      });

      const queue = service.getQueue();
      expect(queue[0].maxRetries).toBe(3);
    });

    it('should increment queue size', () => {
      expect(service.queueSize()).toBe(0);

      service.enqueue({ type: 'write' });
      expect(service.queueSize()).toBe(1);

      service.enqueue({ type: 'update' });
      expect(service.queueSize()).toBe(2);
    });

    it('should notify queue changed callbacks', () => {
      const callback = jasmine.createSpy('callback');
      service.onQueueChanged(callback);

      service.enqueue({ type: 'create' });

      expect(callback).toHaveBeenCalledTimes(1);
      expect(callback).toHaveBeenCalledWith(jasmine.any(Array));
    });
  });

  describe('dequeue', () => {
    it('should remove operation from queue', () => {
      const opId = service.enqueue({ type: 'create' });
      expect(service.queueSize()).toBe(1);

      service.dequeue(opId);
      expect(service.queueSize()).toBe(0);
    });

    it('should do nothing for unknown operation', () => {
      service.enqueue({ type: 'create' });
      expect(service.queueSize()).toBe(1);

      service.dequeue('unknown-id');
      expect(service.queueSize()).toBe(1);
    });

    it('should notify queue changed on dequeue', () => {
      const callback = jasmine.createSpy('callback');
      const opId = service.enqueue({ type: 'create' });

      service.onQueueChanged(callback);
      callback.calls.reset();

      service.dequeue(opId);

      expect(callback).toHaveBeenCalledTimes(1);
    });
  });

  describe('syncAll', () => {
    it('should return early if not connected', async () => {
      mockHolochainClient.isConnected.and.returnValue(false);

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
      });

      const result = await service.syncAll();

      expect(result.succeeded).toBe(0);
      expect(result.failed).toBe(0);
      expect(mockHolochainClient.callZome).not.toHaveBeenCalled();
    });

    it('should sync operations when connected', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: true, data: {} });

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
        payload: {},
      });

      const result = await service.syncAll();

      expect(result.succeeded).toBe(1);
      expect(result.failed).toBe(0);
      expect(service.queueSize()).toBe(0);
    });

    it('should count failed operations', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: false });

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
      });

      const result = await service.syncAll();

      expect(result.succeeded).toBe(0);
      expect(result.failed).toBe(1);
    });

    it('should skip operations without zomeName', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);

      service.enqueue({
        type: 'write', // No zomeName/fnName
      });

      const result = await service.syncAll();

      expect(result.failed).toBe(1);
      expect(mockHolochainClient.callZome).not.toHaveBeenCalled();
    });

    it('should notify sync complete callbacks', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: true });

      const callback = jasmine.createSpy('callback');
      service.onSyncComplete(callback);

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
      });

      await service.syncAll();

      expect(callback).toHaveBeenCalledWith(1, 0);
    });

    it('should update lastSync timestamp', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: true });

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
      });

      expect(service.lastSync()).toBeNull();

      await service.syncAll();

      expect(service.lastSync()).not.toBeNull();
      expect(typeof service.lastSync()).toBe('number');
    });

    it('should prevent concurrent syncs', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.callFake(async () => {
        await new Promise(r => setTimeout(r, 50));
        return { success: true };
      });

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
      });

      // Start first sync
      const sync1 = service.syncAll();

      // Try to start second sync while first is in progress
      const sync2 = service.syncAll();

      const [result1, result2] = await Promise.all([sync1, sync2]);

      expect(result1.succeeded).toBe(1);
      expect(result2.succeeded).toBe(0);
      expect(result2.failed).toBe(0);
    });
  });

  describe('syncOperation', () => {
    it('should sync single operation', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: true });

      const opId = service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
      });

      const result = await service.syncOperation(opId);

      expect(result).toBe(true);
      expect(service.queueSize()).toBe(0);
    });

    it('should return false for unknown operation', async () => {
      const result = await service.syncOperation('unknown-id');
      expect(result).toBe(false);
    });

    it('should return false on failure', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: false });

      const opId = service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
      });

      const result = await service.syncOperation(opId);

      expect(result).toBe(false);
      // Operation should still be in queue for retry
      expect(service.queueSize()).toBe(1);
    });
  });

  describe('clearQueue', () => {
    it('should clear all operations', () => {
      service.enqueue({ type: 'create' });
      service.enqueue({ type: 'update' });
      service.enqueue({ type: 'delete' });

      expect(service.queueSize()).toBe(3);

      service.clearQueue();

      expect(service.queueSize()).toBe(0);
    });

    it('should notify queue changed', () => {
      service.enqueue({ type: 'create' });

      const callback = jasmine.createSpy('callback');
      service.onQueueChanged(callback);
      callback.calls.reset();

      service.clearQueue();

      expect(callback).toHaveBeenCalledWith([]);
    });
  });

  describe('dismissOperation', () => {
    it('should remove operation (alias for dequeue)', () => {
      const opId = service.enqueue({ type: 'create' });
      expect(service.queueSize()).toBe(1);

      service.dismissOperation(opId);
      expect(service.queueSize()).toBe(0);
    });
  });

  describe('cancelRetry', () => {
    it('should cancel pending retry timeout', fakeAsync(() => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: false });

      const opId = service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
      });

      // Trigger sync to schedule a retry
      service.syncAll();
      tick(0);

      // Cancel the retry
      service.cancelRetry(opId);

      // Advance time past retry delay
      tick(5000);

      // Operation should not have been retried (only 1 callZome from initial syncAll)
      expect(mockHolochainClient.callZome).toHaveBeenCalledTimes(1);
    }));
  });

  describe('getStats', () => {
    it('should return empty stats for empty queue', () => {
      const stats = service.getStats();

      expect(stats.size).toBe(0);
      expect(stats.totalRetries).toBe(0);
      expect(stats.averageRetries).toBe(0);
      expect(stats.oldestOperation).toBe(0);
      expect(stats.lastSync).toBeNull();
    });

    it('should return accurate stats', () => {
      service.enqueue({ type: 'create' });
      service.enqueue({ type: 'update' });

      const stats = service.getStats();

      expect(stats.size).toBe(2);
      expect(stats.totalRetries).toBe(0);
      expect(stats.oldestOperation).toBeGreaterThanOrEqual(0);
    });
  });

  describe('computed signals', () => {
    it('should update queueSize reactively', () => {
      expect(service.queueSize()).toBe(0);

      const opId = service.enqueue({ type: 'create' });
      expect(service.queueSize()).toBe(1);

      service.dequeue(opId);
      expect(service.queueSize()).toBe(0);
    });

    it('should update isPending reactively', () => {
      expect(service.isPending()).toBe(false);

      const opId = service.enqueue({ type: 'create' });
      expect(service.isPending()).toBe(true);

      service.dequeue(opId);
      expect(service.isPending()).toBe(false);
    });
  });

  describe('getQueue', () => {
    it('should return copy of queue', () => {
      service.enqueue({ type: 'create' });

      const queue1 = service.getQueue();
      const queue2 = service.getQueue();

      expect(queue1).toEqual(queue2);
      expect(queue1).not.toBe(queue2); // Different array instances
    });

    it('should include all operation properties', () => {
      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
        payload: { test: true },
        maxRetries: 5,
        description: 'Test operation',
      });

      const queue = service.getQueue();
      const op = queue[0];

      expect(typeof op.id).toBe('string');
      expect(op.type).toBe('zome_call');
      expect(op.zomeName).toBe('content_store');
      expect(op.fnName).toBe('create_content');
      expect(op.payload).toEqual({ test: true });
      expect(op.maxRetries).toBe(5);
      expect(op.description).toBe('Test operation');
      expect(op.retryCount).toBe(0);
      expect(typeof op.timestamp).toBe('number');
    });
  });
});
