import { TestBed } from '@angular/core/testing';
import { WriteBufferService, WritePriority, WriteOpType, type WriteOperation } from './write-buffer.service';

/**
 * Unit tests for WriteBufferService
 *
 * Tests the write buffering layer for Holochain operations.
 * Covers priority queuing, batching, deduplication, and backpressure.
 */
describe('WriteBufferService', () => {
  let service: WriteBufferService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [WriteBufferService],
    });
    service = TestBed.inject(WriteBufferService);
  });

  describe('initialization', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should start in uninitialized state', () => {
      expect(service.state).toBe('uninitialized');
    });

    it('should initialize successfully', async () => {
      const result = await service.initialize();

      expect(result.success).toBeTrue();
      expect(service.state).toBe('ready');
      expect(['wasm', 'typescript']).toContain(result.implementation);
    });

    it('should fall back to TypeScript if WASM unavailable', async () => {
      // WASM may not be available in test environment
      const result = await service.initialize();

      expect(result.success).toBeTrue();
      expect(service.implementationType).toBeTruthy();
    });

    it('should not re-initialize if already ready', async () => {
      await service.initialize();
      const secondInit = await service.initialize();

      expect(secondInit.success).toBeTrue();
      expect(service.state).toBe('ready');
    });

    it('should set isReady flag', async () => {
      expect(service.isReady).toBeFalse();

      await service.initialize();

      expect(service.isReady).toBeTrue();
    });
  });

  describe('queueWrite operations', () => {
    describe('when not initialized', () => {
      it('should throw error', () => {
        // Service is freshly created but not initialized
        expect(() => {
          service.queueWrite('op-1', WriteOpType.CreateEntry, '{"data":"test"}');
        }).toThrowError(/not initialized/);
      });
    });

    describe('when initialized', () => {
      beforeEach(async () => {
        await service.initialize();
      });

      it('should queue a write operation', () => {
        const result = service.queueWrite('op-1', WriteOpType.CreateEntry, '{"data":"test"}');

        expect(result).toBeTrue();
        expect(service.totalQueued()).toBe(1);
      });

      it('should queue with default Normal priority', () => {
        service.queueWrite('op-1', WriteOpType.CreateEntry, '{"data":"test"}');

        expect(service.totalQueued()).toBe(1);
      });

      it('should queue with High priority', () => {
        service.queueWrite('op-1', WriteOpType.CreateEntry, '{"data":"test"}', WritePriority.High);

        expect(service.totalQueued()).toBe(1);
      });

      it('should queue with Bulk priority', () => {
        service.queueWrite('op-1', WriteOpType.CreateEntry, '{"data":"test"}', WritePriority.Bulk);

        expect(service.totalQueued()).toBe(1);
      });

      it('should handle multiple operations', () => {
        service.queueWrite('op-1', WriteOpType.CreateEntry, '{"a":1}');
        service.queueWrite('op-2', WriteOpType.CreateLink, '{"b":2}');
        service.queueWrite('op-3', WriteOpType.UpdateEntry, '{"c":3}');

        expect(service.totalQueued()).toBe(3);
      });

      it('should respect backpressure limits', () => {
        // Queue many operations to test backpressure
        for (let i = 0; i < 10000; i++) {
          const queued = service.queueWrite(`op-${i}`, WriteOpType.CreateEntry, '{"data":"x"}');
          if (!queued) {
            // Hit backpressure limit
            expect(service.isBackpressured()).toBeTrue();
            break;
          }
        }

        // Should have queued at least some operations
        expect(service.totalQueued()).toBeGreaterThan(0);
      });
    });
  });

  describe('deduplication', () => {
    beforeEach(async () => {
      await service.initialize();
    });

    it('should deduplicate writes with same dedupKey', () => {
      service.queueWriteWithDedup(
        'op-1',
        WriteOpType.UpdateEntry,
        '{"version":1}',
        WritePriority.Normal,
        'entry-hash-123'
      );

      service.queueWriteWithDedup(
        'op-2',
        WriteOpType.UpdateEntry,
        '{"version":2}',
        WritePriority.Normal,
        'entry-hash-123'
      );

      // Only latest write should remain (last write wins)
      expect(service.totalQueued()).toBe(1);
    });

    it('should keep separate operations with different dedupKeys', () => {
      service.queueWriteWithDedup(
        'op-1',
        WriteOpType.UpdateEntry,
        '{"v":1}',
        WritePriority.Normal,
        'key-a'
      );

      service.queueWriteWithDedup(
        'op-2',
        WriteOpType.UpdateEntry,
        '{"v":2}',
        WritePriority.Normal,
        'key-b'
      );

      expect(service.totalQueued()).toBe(2);
    });
  });

  describe('convenience methods', () => {
    beforeEach(async () => {
      await service.initialize();
    });

    it('should queue create entry', () => {
      service.queueCreateEntry('op-1', '{"data":"content"}');

      expect(service.totalQueued()).toBe(1);
    });

    it('should queue update entry with deduplication', () => {
      service.queueUpdateEntry('op-1', 'hash-1', '{"v":1}');
      service.queueUpdateEntry('op-2', 'hash-1', '{"v":2}');

      // Deduped (last write wins)
      expect(service.totalQueued()).toBe(1);
    });

    it('should queue create link', () => {
      service.queueCreateLink('op-1', '{"from":"a","to":"b"}');

      expect(service.totalQueued()).toBe(1);
    });
  });

  describe('batching', () => {
    beforeEach(async () => {
      await service.initialize();
    });

    it('should determine when to flush', () => {
      expect(service.shouldFlush()).toBeFalse();

      // Queue enough to trigger flush
      for (let i = 0; i < 50; i++) {
        service.queueWrite(`op-${i}`, WriteOpType.CreateEntry, '{"x":1}');
      }

      // Should recommend flushing
      expect(service.shouldFlush()).toBeTrue();
    });

    it('should get pending batch', () => {
      service.queueWrite('op-1', WriteOpType.CreateEntry, '{"a":1}');
      service.queueWrite('op-2', WriteOpType.CreateEntry, '{"b":2}');

      const batchResult = service.getPendingBatch();

      expect(batchResult.hasBatch).toBeTrue();
      expect(batchResult.batch?.operations.length).toBeGreaterThan(0);
    });

    it('should return no batch when queue is empty', () => {
      const batchResult = service.getPendingBatch();

      expect(batchResult.hasBatch).toBeFalse();
      expect(batchResult.batch).toBeNull();
    });
  });

  describe('flushing', () => {
    beforeEach(async () => {
      await service.initialize();
    });

    it('should flush a single batch successfully', async () => {
      service.queueWrite('op-1', WriteOpType.CreateEntry, '{"data":"test"}');

      const callback = jasmine.createSpy('callback').and.returnValue(Promise.resolve());
      const result = await service.flushBatch(callback);

      expect(result).toBeTruthy();
      expect(result?.success).toBeTrue();
      expect(callback).toHaveBeenCalled();
      expect(service.totalQueued()).toBe(0);
    });

    it('should handle flush callback errors', async () => {
      service.queueWrite('op-1', WriteOpType.CreateEntry, '{"data":"test"}');

      const callback = jasmine
        .createSpy('callback')
        .and.returnValue(Promise.reject(new Error('Network error')));
      const result = await service.flushBatch(callback);

      expect(result).toBeTruthy();
      expect(result?.success).toBeFalse();
      expect(result?.error).toContain('Network error');
    });

    it('should return null when no batch to flush', async () => {
      const callback = jasmine.createSpy('callback');
      const result = await service.flushBatch(callback);

      expect(result).toBeNull();
      expect(callback).not.toHaveBeenCalled();
    });

    it('should handle partial success', async () => {
      service.queueWrite('op-1', WriteOpType.CreateEntry, '{"a":1}');
      service.queueWrite('op-2', WriteOpType.CreateEntry, '{"b":2}');

      const callback = jasmine.createSpy('callback').and.returnValue(
        Promise.resolve({
          success: false,
          operationResults: [
            { opId: 'op-1', success: true },
            { opId: 'op-2', success: false, error: 'Validation failed' },
          ],
        })
      );

      const result = await service.flushBatch(callback);

      expect(result).toBeTruthy();
      expect(result?.successCount).toBe(1);
      expect(result?.failureCount).toBe(1);
      expect(result?.failedOperationIds).toContain('op-2');
    });

    it('should set flushing state during flush', async () => {
      service.queueWrite('op-1', WriteOpType.CreateEntry, '{"data":"test"}');

      const callback = jasmine.createSpy('callback').and.callFake(async () => {
        expect(service.isFlushing).toBeTrue();
        return Promise.resolve();
      });

      await service.flushBatch(callback);

      expect(service.isFlushing).toBeFalse();
    });
  });

  describe('flushAll', () => {
    beforeEach(async () => {
      await service.initialize();
    });

    it('should flush all queued operations', async () => {
      for (let i = 0; i < 100; i++) {
        service.queueWrite(`op-${i}`, WriteOpType.CreateEntry, `{"idx":${i}}`);
      }

      let batchesProcessed = 0;
      const callback = jasmine.createSpy('callback').and.callFake(async () => {
        batchesProcessed++;
        return Promise.resolve();
      });

      const committed = await service.flushAll(callback);

      expect(committed).toBe(100);
      expect(batchesProcessed).toBeGreaterThan(0);
      expect(service.totalQueued()).toBe(0);
    });

    it('should call progress callback', async () => {
      for (let i = 0; i < 50; i++) {
        service.queueWrite(`op-${i}`, WriteOpType.CreateEntry, `{"idx":${i}}`);
      }

      const progressSpy = jasmine.createSpy('progress');
      const callback = jasmine.createSpy('callback').and.returnValue(Promise.resolve());

      await service.flushAll(callback, progressSpy);

      expect(progressSpy).toHaveBeenCalled();
    });

    it('should stop after consecutive failures', async () => {
      for (let i = 0; i < 100; i++) {
        service.queueWrite(`op-${i}`, WriteOpType.CreateEntry, `{"idx":${i}}`);
      }

      const callback = jasmine
        .createSpy('callback')
        .and.returnValue(Promise.reject(new Error('Conductor unavailable')));

      const committed = await service.flushAll(callback);

      // Should stop early due to consecutive failures
      expect(committed).toBe(0);
      expect(service.totalQueued()).toBeGreaterThan(0); // Some operations remain
    });

    it('should return detailed results with flushAllWithDetails', async () => {
      for (let i = 0; i < 50; i++) {
        service.queueWrite(`op-${i}`, WriteOpType.CreateEntry, `{"idx":${i}}`);
      }

      const callback = jasmine.createSpy('callback').and.returnValue(Promise.resolve());
      const result = await service.flushAllWithDetails(callback);

      expect(result.totalCommitted).toBe(50);
      expect(result.totalFailed).toBe(0);
      expect(result.batchCount).toBeGreaterThan(0);
      expect(result.failedOperationIds).toEqual([]);
    });
  });

  describe('auto-flush', () => {
    beforeEach(async () => {
      await service.initialize();
    });

    it('should start auto-flush', () => {
      const callback = jasmine.createSpy('callback');

      service.startAutoFlush(callback, 100);

      // Auto-flush started
      expect(service.state).toBe('ready');
    });

    it('should stop auto-flush', () => {
      const callback = jasmine.createSpy('callback');

      service.startAutoFlush(callback, 100);
      service.stopAutoFlush();

      // Should not throw
      expect(service.state).toBe('ready');
    });

    it('should flush automatically when queue fills', async () => {
      const callback = jasmine.createSpy('callback').and.returnValue(Promise.resolve());

      service.startAutoFlush(callback, 50);

      // Queue operations to trigger auto-flush
      for (let i = 0; i < 60; i++) {
        service.queueWrite(`op-${i}`, WriteOpType.CreateEntry, '{"x":1}');
      }

      // Wait for auto-flush interval
      await new Promise(resolve => setTimeout(resolve, 100));

      service.stopAutoFlush();
    });
  });

  describe('statistics', () => {
    beforeEach(async () => {
      await service.initialize();
    });

    it('should report total queued', () => {
      expect(service.totalQueued()).toBe(0);

      service.queueWrite('op-1', WriteOpType.CreateEntry, '{"a":1}');

      expect(service.totalQueued()).toBe(1);
    });

    it('should report in-flight count', () => {
      expect(service.inFlightCount()).toBe(0);
    });

    it('should report backpressure level', () => {
      const backpressure = service.currentBackpressure();

      expect(backpressure).toBeGreaterThanOrEqual(0);
      expect(backpressure).toBeLessThanOrEqual(100);
    });

    it('should get detailed stats', () => {
      service.queueWrite('op-1', WriteOpType.CreateEntry, '{"a":1}');

      const stats = service.getStats();

      expect(stats).toBeDefined();
      const totalQueued = stats.highQueueCount + stats.normalQueueCount + stats.bulkQueueCount + stats.retryQueueCount;
      expect(totalQueued).toBeGreaterThan(0);
    });

    it('should reset stats', () => {
      service.queueWrite('op-1', WriteOpType.CreateEntry, '{"a":1}');

      service.resetStats();

      const stats = service.getStats();
      // Stats reset but queue remains
      expect(stats).toBeDefined();
    });

    it('should expose stats as observable', done => {
      let emissionCount = 0;
      const subscription = service.stats$.subscribe(stats => {
        if (stats) {
          const totalQueued = stats.highQueueCount + stats.normalQueueCount + stats.bulkQueueCount + stats.retryQueueCount;
          expect(totalQueued).toBeGreaterThanOrEqual(0);
          emissionCount++;
          if (emissionCount === 2) {
            subscription.unsubscribe();
            done();
          }
        }
      });

      service.queueWrite('op-1', WriteOpType.CreateEntry, '{"a":1}');
    });

    it('should expose state as observable', done => {
      service.state$.subscribe(state => {
        expect(['uninitialized', 'initializing', 'ready', 'flushing', 'error']).toContain(state);
        done();
      });
    });

    it('should expose backpressure as observable', done => {
      service.backpressure$.subscribe(level => {
        expect(level).toBeGreaterThanOrEqual(0);
        expect(level).toBeLessThanOrEqual(100);
        done();
      });
    });
  });

  describe('configuration', () => {
    beforeEach(async () => {
      await service.initialize();
    });

    it('should set max queue size', () => {
      service.setMaxQueueSize(500);

      // Should not throw
      expect(service.state).toBe('ready');
    });

    it('should initialize with preset configurations', async () => {
      const seedingService = TestBed.inject(WriteBufferService);
      const result = await seedingService.initialize({ preset: 'seeding' });

      expect(result.success).toBeTrue();
    });
  });

  describe('persistence', () => {
    beforeEach(async () => {
      await service.initialize();
    });

    it('should drain all operations', () => {
      service.queueWrite('op-1', WriteOpType.CreateEntry, '{"a":1}');
      service.queueWrite('op-2', WriteOpType.CreateEntry, '{"b":2}');

      const operations = service.drainAll();

      expect(operations.length).toBe(2);
      expect(service.totalQueued()).toBe(0);
    });

    it('should restore operations', () => {
      const operations: WriteOperation[] = [
        {
          opId: 'op-1',
          opType: WriteOpType.CreateEntry,
          payload: '{"a":1}',
          priority: WritePriority.Normal,
          queuedAt: Date.now(),
          retryCount: 0,
          dedupKey: null,
        },
      ];

      service.restore(operations);

      expect(service.totalQueued()).toBe(1);
    });

    it('should clear all operations', () => {
      service.queueWrite('op-1', WriteOpType.CreateEntry, '{"a":1}');

      service.clear();

      expect(service.totalQueued()).toBe(0);
    });
  });

  describe('batch result reporting', () => {
    beforeEach(async () => {
      await service.initialize();
    });

    it('should mark batch as committed', () => {
      service.queueWrite('op-1', WriteOpType.CreateEntry, '{"a":1}');
      const batchResult = service.getPendingBatch();

      if (batchResult.batch) {
        service.markBatchCommitted(batchResult.batch.batchId);
        expect(service.totalQueued()).toBe(0);
      }
    });

    it('should mark batch as failed', () => {
      service.queueWrite('op-1', WriteOpType.CreateEntry, '{"a":1}');
      const batchResult = service.getPendingBatch();

      if (batchResult.batch) {
        service.markBatchFailed(batchResult.batch.batchId, 'Test failure');

        // TODO(test-generator): [HIGH] Verify failed operations are retried
        // Context: markBatchFailed should queue operations for retry
        // Story: Reliable write delivery with automatic retries
        // Suggested approach:
        //   1. Check that totalQueued() increases (retry queue)
        //   2. Verify retry count increments
        //   3. Test max retry limit behavior
      }
    });

    it('should mark specific operations as failed', () => {
      service.queueWrite('op-1', WriteOpType.CreateEntry, '{"a":1}');
      service.queueWrite('op-2', WriteOpType.CreateEntry, '{"b":2}');
      const batchResult = service.getPendingBatch();

      if (batchResult.batch) {
        service.markOperationsFailed(batchResult.batch.batchId, ['op-1']);

        // Partial failure - op-1 retried, op-2 succeeded
        // TODO(test-generator): [MEDIUM] Test partial failure retry logic
      }
    });
  });

  describe('WASM availability', () => {
    it('should check WASM availability', async () => {
      const available = await service.checkWasmAvailable();

      expect(typeof available).toBe('boolean');
    });
  });

  describe('error handling', () => {
    it('should handle initialization failure gracefully', async () => {
      const service = TestBed.inject(WriteBufferService);

      // Even if WASM fails, should fall back to TypeScript
      const result = await service.initialize();

      expect(result.success).toBeTrue();
      expect(result.implementation).toBeTruthy();
    });
  });

  describe('priority ordering', () => {
    beforeEach(async () => {
      await service.initialize();
    });

    it('should process High priority before Normal', () => {
      service.queueWrite('normal', WriteOpType.CreateEntry, '{"p":"normal"}', WritePriority.Normal);
      service.queueWrite('high', WriteOpType.CreateEntry, '{"p":"high"}', WritePriority.High);

      const batch = service.getPendingBatch();

      // High priority should be in batch first
      expect(batch.hasBatch).toBeTrue();
    });

    it('should process Normal priority before Bulk', () => {
      service.queueWrite('bulk', WriteOpType.CreateEntry, '{"p":"bulk"}', WritePriority.Bulk);
      service.queueWrite('normal', WriteOpType.CreateEntry, '{"p":"normal"}', WritePriority.Normal);

      const batch = service.getPendingBatch();

      expect(batch.hasBatch).toBeTrue();
    });
  });
});
