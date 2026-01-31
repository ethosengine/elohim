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

  /*
   * Tests to escalate:
   *
   * Async flow tests:
   * - Exponential backoff timing
   * - Concurrent sync handling with multiple pending operations
   * - Connection state change during sync
   * - Timeout handling for long-running operations
   *
   * Comprehensive mocks:
   * - Mock LoggerService.createChild()
   * - Mock IndexedDB storage operations
   * - Mock crypto.getRandomValues for deterministic ID generation
   *
   * Business logic tests:
   * - Retry count increment logic
   * - Operation deduplication
   * - Queue ordering guarantees
   * - State consistency across signal updates
   *
   * Queue state tests:
   * - Queue persistence verification
   * - State after service destruction/recreation
   * - Memory cleanup (Map of pending retries)
   */
});
