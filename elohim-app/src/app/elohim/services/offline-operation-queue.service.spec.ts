/**
 * Offline Operation Queue Service Tests
 */

import { TestBed, fakeAsync, tick } from '@angular/core/testing';

import { HolochainClientService } from './holochain-client.service';
import { OfflineOperationQueueService } from './offline-operation-queue.service';

describe('OfflineOperationQueueService', () => {
  let service: OfflineOperationQueueService;
  let mockHolochainClient: jasmine.SpyObj<HolochainClientService>;

  const unknownOperationId = 'unknown-id';

  beforeEach(() => {
    mockHolochainClient = jasmine.createSpyObj<HolochainClientService>('HolochainClientService', [
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

  describe('Service Creation', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should be singleton (providedIn: root)', () => {
      const service2 = TestBed.inject(OfflineOperationQueueService);
      expect(service).toBe(service2);
    });
  });

  describe('Public Methods Existence', () => {
    it('should have enqueue method', () => {
      expect(typeof service.enqueue).toBe('function');
    });

    it('should have dequeue method', () => {
      expect(typeof service.dequeue).toBe('function');
    });

    it('should have syncAll method', () => {
      expect(typeof service.syncAll).toBe('function');
    });

    it('should have syncOperation method', () => {
      expect(typeof service.syncOperation).toBe('function');
    });

    it('should have cancelRetry method', () => {
      expect(typeof service.cancelRetry).toBe('function');
    });

    it('should have getQueue method', () => {
      expect(typeof service.getQueue).toBe('function');
    });

    it('should have getQueueSize method', () => {
      expect(typeof service.getQueueSize).toBe('function');
    });

    it('should have clearQueue method', () => {
      expect(typeof service.clearQueue).toBe('function');
    });

    it('should have dismissOperation method', () => {
      expect(typeof service.dismissOperation).toBe('function');
    });

    it('should have onQueueChanged method', () => {
      expect(typeof service.onQueueChanged).toBe('function');
    });

    it('should have onSyncComplete method', () => {
      expect(typeof service.onSyncComplete).toBe('function');
    });

    it('should have getStats method', () => {
      expect(typeof service.getStats).toBe('function');
    });
  });

  describe('Public Properties/Signals Existence', () => {
    it('should expose queueSize signal', () => {
      expect(service.queueSize).toBeDefined();
      expect(typeof service.queueSize()).toBe('number');
    });

    it('should expose isPending signal', () => {
      expect(service.isPending).toBeDefined();
      expect(typeof service.isPending()).toBe('boolean');
    });

    it('should expose syncInProgress signal', () => {
      expect(service.syncInProgress).toBeDefined();
      expect(typeof service.syncInProgress()).toBe('boolean');
    });

    it('should expose lastSync signal', () => {
      expect(service.lastSync).toBeDefined();
      expect(service.lastSync()).toBeNull();
    });
  });

  describe('Property Initialization', () => {
    it('should initialize queueSize to 0', () => {
      expect(service.queueSize()).toBe(0);
    });

    it('should initialize isPending to false', () => {
      expect(service.isPending()).toBe(false);
    });

    it('should initialize syncInProgress to false', () => {
      expect(service.syncInProgress()).toBe(false);
    });

    it('should initialize lastSync to null', () => {
      expect(service.lastSync()).toBeNull();
    });

    it('should initialize getQueueSize to 0', () => {
      expect(service.getQueueSize()).toBe(0);
    });

    it('should initialize getQueue to empty array', () => {
      expect(service.getQueue()).toEqual([]);
      expect(Array.isArray(service.getQueue())).toBe(true);
    });
  });

  describe('Signal Type Tests', () => {
    it('queueSize should be a Signal', () => {
      expect(typeof service.queueSize()).toBe('number');
      service.enqueue({ type: 'create' });
      expect(service.queueSize()).toBe(1);
    });

    it('isPending should be a Signal', () => {
      expect(typeof service.isPending()).toBe('boolean');
      service.enqueue({ type: 'create' });
      expect(service.isPending()).toBe(true);
    });

    it('syncInProgress should be a readonly Signal', () => {
      expect(typeof service.syncInProgress()).toBe('boolean');
    });

    it('lastSync should be a readonly Signal', () => {
      const lastSync = service.lastSync();
      expect(lastSync === null || typeof lastSync === 'number').toBe(true);
    });
  });

  describe('Method Return Type Tests', () => {
    it('enqueue should return a string', () => {
      const result = service.enqueue({ type: 'create' });
      expect(typeof result).toBe('string');
    });

    it('dequeue should return undefined', () => {
      const operationId = service.enqueue({ type: 'create' });
      const result = service.dequeue(operationId);
      expect(result).toBeUndefined();
    });

    it('getQueue should return an array', () => {
      const result = service.getQueue();
      expect(Array.isArray(result)).toBe(true);
    });

    it('getQueueSize should return a number', () => {
      const result = service.getQueueSize();
      expect(typeof result).toBe('number');
    });

    it('clearQueue should return undefined', () => {
      const result = service.clearQueue();
      expect(result).toBeUndefined();
    });

    it('dismissOperation should return undefined', () => {
      const operationId = service.enqueue({ type: 'create' });
      const result = service.dismissOperation(operationId);
      expect(result).toBeUndefined();
    });

    it('cancelRetry should return undefined', () => {
      const operationId = service.enqueue({ type: 'create' });
      const result = service.cancelRetry(operationId);
      expect(result).toBeUndefined();
    });

    it('onQueueChanged should return undefined', () => {
      const result = service.onQueueChanged(() => {
        /* noop */
      });
      expect(result).toBeUndefined();
    });

    it('onSyncComplete should return undefined', () => {
      const result = service.onSyncComplete(() => {
        /* noop */
      });
      expect(result).toBeUndefined();
    });

    it('syncAll should return a Promise with succeeded and failed', async () => {
      mockHolochainClient.isConnected.and.returnValue(false);
      const result = await service.syncAll();
      expect(result).toBeDefined();
      expect(typeof result.succeeded).toBe('number');
      expect(typeof result.failed).toBe('number');
    });

    it('syncOperation should return a Promise<boolean>', async () => {
      const result = await service.syncOperation(unknownOperationId);
      expect(typeof result).toBe('boolean');
    });

    it('getStats should return stats object', () => {
      const stats = service.getStats();
      const typeNumber = 'number';
      expect(stats).toBeDefined();
      expect(typeof stats.size).toBe(typeNumber);
      expect(typeof stats.totalRetries).toBe(typeNumber);
      expect(typeof stats.averageRetries).toBe(typeNumber);
      expect(typeof stats.oldestOperation).toBe(typeNumber);
      expect(stats.lastSync === null || typeof stats.lastSync === typeNumber).toBe(true);
    });
  });

  describe('OfflineOperation Interface Tests', () => {
    it('should create operation with required fields', () => {
      service.enqueue({
        type: 'create',
      });

      const queue = service.getQueue();
      const operation = queue[0];

      expect(operation.id).toBeDefined();
      expect(typeof operation.id).toBe('string');
      expect(operation.timestamp).toBeDefined();
      expect(typeof operation.timestamp).toBe('number');
      expect(operation.type).toBe('create');
      expect(operation.retryCount).toBe(0);
      expect(operation.maxRetries).toBeDefined();
      expect(typeof operation.maxRetries).toBe('number');
    });

    it('should support all operation types', () => {
      const types: ('zome_call' | 'write' | 'create' | 'update' | 'delete')[] = [
        'zome_call',
        'write',
        'create',
        'update',
        'delete',
      ];

      types.forEach(type => {
        service.clearQueue();
        service.enqueue({ type });
        const queue = service.getQueue();
        expect(queue[0].type).toBe(type);
      });
    });

    it('should preserve optional fields', () => {
      service.enqueue({
        type: 'zome_call',
        zomeName: 'test_zome',
        fnName: 'test_fn',
        payload: { key: 'value' },
        description: 'Test description',
        maxRetries: 5,
      });

      const queue = service.getQueue();
      const operation = queue[0];

      expect(operation.zomeName).toBe('test_zome');
      expect(operation.fnName).toBe('test_fn');
      expect(operation.payload).toEqual({ key: 'value' });
      expect(operation.description).toBe('Test description');
      expect(operation.maxRetries).toBe(5);
    });
  });

  describe('Callback Registration Tests', () => {
    it('should accept queue change callback', () => {
      const callback = jasmine.createSpy('callback');
      expect(() => service.onQueueChanged(callback)).not.toThrow();
    });

    it('should accept sync complete callback', () => {
      const callback = jasmine.createSpy('callback');
      expect(() => service.onSyncComplete(callback)).not.toThrow();
    });

    it('should allow multiple queue change callbacks', () => {
      const callback1 = jasmine.createSpy('callback1');
      const callback2 = jasmine.createSpy('callback2');

      service.onQueueChanged(callback1);
      service.onQueueChanged(callback2);

      service.enqueue({ type: 'create' });

      expect(callback1).toHaveBeenCalled();
      expect(callback2).toHaveBeenCalled();
    });

    it('should allow multiple sync complete callbacks', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: {} })
      );

      const callback1 = jasmine.createSpy('callback1');
      const callback2 = jasmine.createSpy('callback2');

      service.onSyncComplete(callback1);
      service.onSyncComplete(callback2);

      // Queue an operation to sync
      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'test',
        payload: { test: 'data' },
      });

      await service.syncAll();

      expect(callback1).toHaveBeenCalled();
      expect(callback2).toHaveBeenCalled();
    });
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

      service.dequeue(unknownOperationId);
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
      const result = await service.syncOperation(unknownOperationId);
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
    it('should cancel pending retry timeout', fakeAsync(async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: false });

      const opId = service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
      });

      // Trigger sync to schedule a retry
      await service.syncAll();
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

  // ==========================================================================
  // Exponential Backoff & Retry Logic
  // ==========================================================================

  describe('exponential backoff retry', () => {
    it('should increment retry count on failure', fakeAsync(async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: false });

      const opId = service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
        maxRetries: 3,
      });

      // First sync attempt
      await service.syncAll();
      tick(0);

      const queue = service.getQueue();
      const operation = queue.find(op => op.id === opId);
      expect(operation?.retryCount).toBe(1);
    }));

    it('should schedule retry with exponential delay', fakeAsync(async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: false });

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
        maxRetries: 3,
      });

      // Initial sync
      await service.syncAll();
      tick(0);
      const initialCalls = mockHolochainClient.callZome.calls.count();

      // Wait 500ms - should not retry yet (delay is 1000ms for retry count 0)
      tick(500);
      expect(mockHolochainClient.callZome.calls.count()).toBe(initialCalls);

      // Wait another 600ms (total 1100ms) - should retry now
      tick(600);
      expect(mockHolochainClient.callZome.calls.count()).toBeGreaterThan(initialCalls);
    }));

    it('should stop retrying after max retries exceeded', fakeAsync(async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: false });

      const opId = service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
        maxRetries: 2,
      });

      // Sync 1
      await service.syncAll();
      tick(0);
      expect(service.getQueue()[0].retryCount).toBe(1);

      // Trigger retry 1
      tick(1000);
      tick(0);
      expect(service.getQueue()[0].retryCount).toBe(2);

      // Trigger retry 2
      tick(2000);
      tick(0);

      // After maxRetries, should still be in queue but no more scheduled retries
      const operation = service.getQueue().find(op => op.id === opId);
      expect(operation?.retryCount).toBe(2);
    }));

    it('should calculate exponential backoff correctly', fakeAsync(async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: false });

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
        maxRetries: 4,
      });

      const callCounts: number[] = [];
      callCounts.push(mockHolochainClient.callZome.calls.count());

      // Initial sync
      await service.syncAll();
      tick(0);
      callCounts.push(mockHolochainClient.callZome.calls.count());

      // Retry 1: 1000ms (2^0 * 1000)
      tick(1000);
      tick(0);
      callCounts.push(mockHolochainClient.callZome.calls.count());

      // Retry 2: 2000ms (2^1 * 1000)
      tick(2000);
      tick(0);
      callCounts.push(mockHolochainClient.callZome.calls.count());

      // Verify each retry happened
      expect(callCounts[1]).toBeGreaterThan(callCounts[0]); // Initial sync
      expect(callCounts[2]).toBeGreaterThan(callCounts[1]); // First retry
      expect(callCounts[3]).toBeGreaterThan(callCounts[2]); // Second retry
    }));

    it('should not schedule duplicate retries', fakeAsync(async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: false });

      const opId = service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
        maxRetries: 3,
      });

      // Initial sync
      await service.syncAll();
      tick(0);

      // Try to sync same operation again before retry
      await service.syncOperation(opId);
      tick(0);

      const callCount = mockHolochainClient.callZome.calls.count();

      // Wait for scheduled retry
      tick(1100);

      // Should only be 1 additional call (the scheduled retry)
      expect(mockHolochainClient.callZome.calls.count()).toBe(callCount + 1);
    }));
  });

  // ==========================================================================
  // Concurrent Operations & Race Conditions
  // ==========================================================================

  describe('concurrent operations', () => {
    it('should handle multiple operations in queue', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: true });

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'op1',
      });
      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'op2',
      });
      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'op3',
      });

      const result = await service.syncAll();

      expect(result.succeeded).toBe(3);
      expect(service.queueSize()).toBe(0);
    });

    it('should process operations in FIFO order', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);

      const callOrder: string[] = [];
      mockHolochainClient.callZome.and.callFake(async (params: any) => {
        callOrder.push(params.fnName);
        return { success: true };
      });

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'first',
      });
      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'second',
      });
      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'third',
      });

      await service.syncAll();

      expect(callOrder).toEqual(['first', 'second', 'third']);
    });

    it('should handle mixed success/failure in batch', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);

      let callCount = 0;
      mockHolochainClient.callZome.and.callFake(async () => {
        callCount++;
        // Fail every other call
        return { success: callCount % 2 === 1 };
      });

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'op1',
      });
      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'op2',
      });
      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'op3',
      });
      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'op4',
      });

      const result = await service.syncAll();

      expect(result.succeeded).toBe(2); // op1, op3
      expect(result.failed).toBe(2); // op2, op4
      expect(service.queueSize()).toBe(2); // Failed ops remain
    });

    it('should handle errors thrown during sync', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);

      let callCount = 0;
      mockHolochainClient.callZome.and.callFake(async () => {
        callCount++;
        if (callCount === 2) {
          throw new Error('Network error');
        }
        return { success: true };
      });

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'op1',
      });
      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'op2', // Will throw
      });
      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'op3',
      });

      const result = await service.syncAll();

      expect(result.succeeded).toBe(2);
      expect(result.failed).toBe(1);
    });
  });

  // ==========================================================================
  // Connection State Handling
  // ==========================================================================

  describe('connection state handling', () => {
    it('should defer retry when connection lost', fakeAsync(async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: false });

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
        maxRetries: 3,
      });

      // Initial sync
      await service.syncAll();
      tick(0);

      // Lose connection
      mockHolochainClient.isConnected.and.returnValue(false);

      // Wait for retry attempt
      tick(1100);

      // Retry should be deferred, not executed
      expect(mockHolochainClient.callZome.calls.count()).toBe(1); // Only initial
    }));

    it('should skip operations removed from queue during retry', fakeAsync(async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: false });

      const opId = service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
        maxRetries: 3,
      });

      // Initial sync
      await service.syncAll();
      tick(0);

      // Remove operation before retry
      service.dequeue(opId);

      // Wait for retry attempt
      tick(1100);

      // Should not retry removed operation
      expect(mockHolochainClient.callZome.calls.count()).toBe(1);
    }));
  });

  // ==========================================================================
  // Error Recovery & Edge Cases
  // ==========================================================================

  describe('error recovery', () => {
    it('should handle malformed operation gracefully', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        // Missing fnName - should be caught
      } as any);

      const result = await service.syncAll();

      expect(result.failed).toBe(1);
      expect(result.succeeded).toBe(0);
    });

    it('should handle exceptions in callZome', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.rejectWith(new Error('Connection timeout'));

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
      });

      const result = await service.syncAll();

      expect(result.failed).toBe(1);
      expect(service.queueSize()).toBe(1); // Still in queue for retry
    });

    it('should handle payload serialization issues', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: true });

      // Create circular reference
      const circular: any = { prop: 'value' };
      circular.self = circular;

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
        payload: circular,
      });

      // Should not crash, payload is stored as-is
      expect(service.queueSize()).toBe(1);
    });
  });

  // ==========================================================================
  // Queue State Consistency
  // ==========================================================================

  describe('queue state consistency', () => {
    it('should maintain signal consistency after rapid enqueues', () => {
      for (let i = 0; i < 10; i++) {
        service.enqueue({ type: 'create' });
      }

      expect(service.queueSize()).toBe(10);
      expect(service.isPending()).toBe(true);
      expect(service.getQueue().length).toBe(10);
    });

    it('should maintain signal consistency after rapid dequeues', () => {
      const ids: string[] = [];
      for (let i = 0; i < 10; i++) {
        ids.push(service.enqueue({ type: 'create' }));
      }

      for (const id of ids) {
        service.dequeue(id);
      }

      expect(service.queueSize()).toBe(0);
      expect(service.isPending()).toBe(false);
      expect(service.getQueue().length).toBe(0);
    });

    it('should notify all callbacks for each change', () => {
      const callbacks = [
        jasmine.createSpy('callback1'),
        jasmine.createSpy('callback2'),
        jasmine.createSpy('callback3'),
      ];

      callbacks.forEach(cb => service.onQueueChanged(cb));

      service.enqueue({ type: 'create' });
      service.enqueue({ type: 'update' });
      service.clearQueue();

      callbacks.forEach(cb => {
        expect(cb).toHaveBeenCalledTimes(3);
      });
    });
  });

  // ==========================================================================
  // Operation ID Generation
  // ==========================================================================

  describe('operation ID generation', () => {
    it('should generate unique IDs', () => {
      const ids = new Set<string>();

      for (let i = 0; i < 100; i++) {
        const id = service.enqueue({ type: 'create' });
        ids.add(id);
      }

      expect(ids.size).toBe(100);
    });

    it('should generate IDs with correct format', () => {
      const id = service.enqueue({ type: 'create' });

      expect(id).toMatch(/^op-\d+-[a-z0-9]+$/);
    });
  });

  // ==========================================================================
  // Stats Calculation
  // ==========================================================================

  describe('stats calculation edge cases', () => {
    it('should calculate average retries correctly', fakeAsync(async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: false });

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'op1',
        maxRetries: 5,
      });
      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'op2',
        maxRetries: 5,
      });

      // Sync once
      await service.syncAll();
      tick(0);

      // Both should have retryCount = 1
      const stats = service.getStats();
      expect(stats.totalRetries).toBe(2);
      expect(stats.averageRetries).toBe(1.0);
    }));

    it('should handle oldest operation age', () => {
      const beforeEnqueue = Date.now();
      service.enqueue({ type: 'create' });

      // Wait a bit
      const delay = 100;
      const start = Date.now();
      while (Date.now() - start < delay) {
        // Busy wait
      }

      const stats = service.getStats();
      const ageSeconds = stats.oldestOperation;

      expect(ageSeconds).toBeGreaterThanOrEqual(0);
      expect(ageSeconds).toBeLessThan(10); // Should be very recent
    });

    it('should round average retries to 1 decimal', fakeAsync(async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: false });

      // Create operations with different retry counts
      const op1 = service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'op1',
        maxRetries: 5,
      });

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'op2',
        maxRetries: 5,
      });

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'op3',
        maxRetries: 5,
      });

      // Sync once - all get 1 retry
      await service.syncAll();
      tick(0);

      // Manually set different retry counts for testing
      // (In real scenario, this would happen through multiple failed syncs)
      const queue = service.getQueue();
      (queue[0] as any).retryCount = 1;
      (queue[1] as any).retryCount = 2;
      (queue[2] as any).retryCount = 3;

      // Average: (1 + 2 + 3) / 3 = 2.0
      const stats = service.getStats();
      expect(stats.averageRetries).toBe(2.0);
    }));
  });

  // ==========================================================================
  // Memory Management
  // ==========================================================================

  describe('memory management', () => {
    it('should clean up retry timeouts when operation dequeued', fakeAsync(async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: false });

      const opId = service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'create_content',
        maxRetries: 3,
      });

      // Trigger sync to schedule retry
      await service.syncAll();
      tick(0);

      // Dequeue should clean up pending timeout
      service.dequeue(opId);

      // Advance time
      tick(2000);

      // Should not have any more calls
      expect(mockHolochainClient.callZome.calls.count()).toBe(1);
    }));

    it('should handle many operations without memory issues', () => {
      // Create 1000 operations
      for (let i = 0; i < 1000; i++) {
        service.enqueue({
          type: 'create',
          payload: { index: i, data: 'test'.repeat(10) },
        });
      }

      expect(service.queueSize()).toBe(1000);

      // Clear should free memory
      service.clearQueue();

      expect(service.queueSize()).toBe(0);
      expect(service.getQueue()).toEqual([]);
    });
  });

  // ==========================================================================
  // Complex Async Scenarios
  // ==========================================================================

  describe('complex async scenarios', () => {
    it('should handle operation added during sync', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);

      let syncStarted = false;
      mockHolochainClient.callZome.and.callFake(async () => {
        syncStarted = true;
        await new Promise(r => setTimeout(r, 10));
        return { success: true };
      });

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'op1',
      });

      // Start sync
      const syncPromise = service.syncAll();

      // Wait for sync to start
      while (!syncStarted) {
        await new Promise(r => setTimeout(r, 1));
      }

      // Add operation during sync
      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'op2',
      });

      await syncPromise;

      // Second operation should still be in queue
      expect(service.queueSize()).toBe(1);
    });

    it('should handle rapid sync calls', async () => {
      mockHolochainClient.isConnected.and.returnValue(true);
      mockHolochainClient.callZome.and.resolveTo({ success: true });

      service.enqueue({
        type: 'zome_call',
        zomeName: 'content_store',
        fnName: 'op1',
      });

      // Multiple rapid sync calls
      const results = await Promise.all([
        service.syncAll(),
        service.syncAll(),
        service.syncAll(),
      ]);

      // Only first should succeed, others should return early
      expect(results[0].succeeded).toBe(1);
      expect(results[1].succeeded).toBe(0);
      expect(results[2].succeeded).toBe(0);
    });
  });
});
