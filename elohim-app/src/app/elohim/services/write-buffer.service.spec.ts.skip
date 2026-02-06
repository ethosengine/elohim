/**
 * Write Buffer Service Tests
 */

import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { WriteBufferService, WritePriority, WriteOpType, BatchCallbackResult } from './write-buffer.service';
import { LoggerService } from './logger.service';

// Mock the write-buffer module
jest.mock('../../../../../elohim-library/projects/elohim-service/src/cache/write-buffer', () => ({
  WritePriority: { High: 0, Normal: 1, Bulk: 2 },
  WriteOpType: { CreateEntry: 0, UpdateEntry: 1, DeleteEntry: 2, CreateLink: 3, DeleteLink: 4 },
  createWriteBuffer: jest.fn().mockResolvedValue({
    buffer: {
      queueWrite: jest.fn().mockReturnValue(true),
      queueWriteWithDedup: jest.fn().mockReturnValue(true),
      shouldFlush: jest.fn().mockReturnValue(false),
      getPendingBatch: jest.fn().mockReturnValue({ hasBatch: false, batch: null }),
      markBatchCommitted: jest.fn(),
      markBatchFailed: jest.fn(),
      markOperationsFailed: jest.fn(),
      totalQueued: jest.fn().mockReturnValue(0),
      inFlightCount: jest.fn().mockReturnValue(0),
      backpressure: jest.fn().mockReturnValue(0),
      isBackpressured: jest.fn().mockReturnValue(false),
      getStats: jest.fn().mockReturnValue({
        totalQueued: 0,
        highPriorityQueued: 0,
        normalPriorityQueued: 0,
        bulkPriorityQueued: 0,
        inFlight: 0,
        committed: 0,
        failed: 0,
        retried: 0,
        backpressure: 0,
      }),
      resetStats: jest.fn(),
      setMaxQueueSize: jest.fn(),
      clear: jest.fn(),
      drainAll: jest.fn().mockReturnValue([]),
      restore: jest.fn(),
      dispose: jest.fn(),
    },
    implementation: 'typescript',
  }),
  isWasmBufferAvailable: jest.fn().mockResolvedValue(false),
  TsWriteBuffer: jest.fn().mockImplementation(() => ({
    queueWrite: jest.fn().mockReturnValue(true),
    queueWriteWithDedup: jest.fn().mockReturnValue(true),
    shouldFlush: jest.fn().mockReturnValue(false),
    getPendingBatch: jest.fn().mockReturnValue({ hasBatch: false, batch: null }),
    markBatchCommitted: jest.fn(),
    markBatchFailed: jest.fn(),
    markOperationsFailed: jest.fn(),
    totalQueued: jest.fn().mockReturnValue(0),
    inFlightCount: jest.fn().mockReturnValue(0),
    backpressure: jest.fn().mockReturnValue(0),
    isBackpressured: jest.fn().mockReturnValue(false),
    getStats: jest.fn().mockReturnValue({
      totalQueued: 0,
      highPriorityQueued: 0,
      normalPriorityQueued: 0,
      bulkPriorityQueued: 0,
      inFlight: 0,
      committed: 0,
      failed: 0,
      retried: 0,
      backpressure: 0,
    }),
    resetStats: jest.fn(),
    setMaxQueueSize: jest.fn(),
    clear: jest.fn(),
    drainAll: jest.fn().mockReturnValue([]),
    restore: jest.fn(),
    dispose: jest.fn(),
  })),
}));

describe('WriteBufferService', () => {
  let service: WriteBufferService;
  let mockLogger: jasmine.SpyObj<LoggerService>;
  let mockChildLogger: jasmine.SpyObj<ReturnType<LoggerService['createChild']>>;

  beforeEach(() => {
    mockChildLogger = jasmine.createSpyObj('ChildLogger', ['debug', 'info', 'warn', 'error']);
    mockLogger = jasmine.createSpyObj('LoggerService', ['createChild']);
    mockLogger.createChild.and.returnValue(mockChildLogger);

    TestBed.configureTestingModule({
      providers: [
        WriteBufferService,
        { provide: LoggerService, useValue: mockLogger },
      ],
    });

    service = TestBed.inject(WriteBufferService);
  });

  afterEach(() => {
    service.ngOnDestroy();
  });

  describe('initialization', () => {
    it('should start in uninitialized state', () => {
      expect(service.state).toBe('uninitialized');
      expect(service.isReady).toBe(false);
    });

    it('should initialize successfully', async () => {
      const result = await service.initialize();

      expect(result.success).toBe(true);
      expect(result.implementation).toBe('typescript');
      expect(service.state).toBe('ready');
      expect(service.isReady).toBe(true);
    });

    it('should return existing result on re-initialization', async () => {
      await service.initialize();
      const result = await service.initialize();

      expect(result.success).toBe(true);
    });
  });

  describe('queueing operations', () => {
    beforeEach(async () => {
      await service.initialize();
    });

    it('should queue write operation', () => {
      const result = service.queueWrite('op-1', WriteOpType.CreateEntry, '{}');
      expect(result).toBe(true);
    });

    it('should queue write with deduplication', () => {
      const result = service.queueWriteWithDedup(
        'op-1',
        WriteOpType.UpdateEntry,
        '{}',
        WritePriority.Normal,
        'dedup-key-1'
      );
      expect(result).toBe(true);
    });

    it('should queue create entry', () => {
      const result = service.queueCreateEntry('op-1', '{}');
      expect(result).toBe(true);
    });

    it('should queue update entry with dedup', () => {
      const result = service.queueUpdateEntry('op-1', 'entry-hash', '{}');
      expect(result).toBe(true);
    });

    it('should queue create link', () => {
      const result = service.queueCreateLink('op-1', '{}');
      expect(result).toBe(true);
    });

    it('should throw if not initialized', () => {
      const uninitializedService = new WriteBufferService();

      expect(() => uninitializedService.queueWrite('op-1', WriteOpType.CreateEntry, '{}'))
        .toThrowError(/not initialized/);
    });
  });

  describe('flushing', () => {
    beforeEach(async () => {
      await service.initialize();
    });

    it('should return null when no batch pending', async () => {
      const result = await service.flushBatch(async () => {});
      expect(result).toBeNull();
    });

    it('should flush batch with legacy void callback', async () => {
      // Mock buffer to return a batch
      const mockBuffer = (service as any).buffer;
      mockBuffer.getPendingBatch.mockReturnValueOnce({
        hasBatch: true,
        batch: {
          batchId: 'batch-1',
          operations: [
            { opId: 'op-1', opType: WriteOpType.CreateEntry, payload: '{}', priority: WritePriority.Normal },
          ],
        },
      });

      const callback = jasmine.createSpy('callback').and.resolveTo(undefined);
      const result = await service.flushBatch(callback);

      expect(result).not.toBeNull();
      expect(result!.success).toBe(true);
      expect(result!.batchId).toBe('batch-1');
      expect(result!.operationCount).toBe(1);
      expect(result!.successCount).toBe(1);
      expect(result!.failureCount).toBe(0);
      expect(callback).toHaveBeenCalled();
    });

    it('should handle callback exception as batch failure', async () => {
      const mockBuffer = (service as any).buffer;
      mockBuffer.getPendingBatch.mockReturnValueOnce({
        hasBatch: true,
        batch: {
          batchId: 'batch-1',
          operations: [
            { opId: 'op-1', opType: WriteOpType.CreateEntry, payload: '{}', priority: WritePriority.Normal },
          ],
        },
      });

      const callback = jasmine.createSpy('callback').and.rejectWith(new Error('Network error'));
      const result = await service.flushBatch(callback);

      expect(result).not.toBeNull();
      expect(result!.success).toBe(false);
      expect(result!.failureCount).toBe(1);
      expect(result!.error).toBe('Network error');
      expect(mockBuffer.markBatchFailed).toHaveBeenCalledWith('batch-1', 'Network error');
    });

    it('should handle partial success with BatchCallbackResult', async () => {
      const mockBuffer = (service as any).buffer;
      mockBuffer.getPendingBatch.mockReturnValueOnce({
        hasBatch: true,
        batch: {
          batchId: 'batch-1',
          operations: [
            { opId: 'op-1', opType: WriteOpType.CreateEntry, payload: '{}', priority: WritePriority.Normal },
            { opId: 'op-2', opType: WriteOpType.CreateEntry, payload: '{}', priority: WritePriority.Normal },
            { opId: 'op-3', opType: WriteOpType.CreateEntry, payload: '{}', priority: WritePriority.Normal },
          ],
        },
      });

      const partialResult: BatchCallbackResult = {
        success: false,
        operationResults: [
          { opId: 'op-1', success: true },
          { opId: 'op-2', success: false, error: 'Validation failed' },
          { opId: 'op-3', success: true },
        ],
      };

      const callback = jasmine.createSpy('callback').and.resolveTo(partialResult);
      const result = await service.flushBatch(callback);

      expect(result).not.toBeNull();
      expect(result!.success).toBe(false);
      expect(result!.operationCount).toBe(3);
      expect(result!.successCount).toBe(2);
      expect(result!.failureCount).toBe(1);
      expect(result!.failedOperationIds).toEqual(['op-2']);
      expect(mockBuffer.markOperationsFailed).toHaveBeenCalledWith('batch-1', ['op-2']);
    });

    it('should handle all operations failed in BatchCallbackResult', async () => {
      const mockBuffer = (service as any).buffer;
      mockBuffer.getPendingBatch.mockReturnValueOnce({
        hasBatch: true,
        batch: {
          batchId: 'batch-1',
          operations: [
            { opId: 'op-1', opType: WriteOpType.CreateEntry, payload: '{}', priority: WritePriority.Normal },
          ],
        },
      });

      const allFailed: BatchCallbackResult = {
        success: false,
        operationResults: [
          { opId: 'op-1', success: false, error: 'Failed' },
        ],
      };

      const callback = jasmine.createSpy('callback').and.resolveTo(allFailed);
      const result = await service.flushBatch(callback);

      expect(result!.success).toBe(false);
      expect(result!.failureCount).toBe(1);
      expect(mockBuffer.markBatchFailed).toHaveBeenCalled();
    });
  });

  describe('statistics', () => {
    beforeEach(async () => {
      await service.initialize();
    });

    it('should return queued count', () => {
      expect(service.totalQueued()).toBe(0);
    });

    it('should return in-flight count', () => {
      expect(service.inFlightCount()).toBe(0);
    });

    it('should return backpressure level', () => {
      expect(service.currentBackpressure()).toBe(0);
    });

    it('should return backpressure status', () => {
      expect(service.isBackpressured()).toBe(false);
    });

    it('should return stats', () => {
      const stats = service.getStats();
      expect(stats.totalQueued).toBe(0);
      expect(stats.committed).toBe(0);
    });
  });

  describe('observables', () => {
    it('should emit state changes', async () => {
      const states: string[] = [];
      service.state$.subscribe(s => states.push(s));

      await service.initialize();

      expect(states).toContain('uninitialized');
      expect(states).toContain('initializing');
      expect(states).toContain('ready');
    });

    it('should emit stats changes', async () => {
      const emissions: any[] = [];
      service.stats$.subscribe(s => emissions.push(s));

      await service.initialize();

      expect(emissions.length).toBeGreaterThan(0);
    });
  });

  describe('persistence', () => {
    beforeEach(async () => {
      await service.initialize();
    });

    it('should clear buffer', () => {
      service.clear();
      expect(service.totalQueued()).toBe(0);
    });

    it('should drain all operations', () => {
      const ops = service.drainAll();
      expect(Array.isArray(ops)).toBe(true);
    });

    it('should restore operations', () => {
      const ops = [
        { opId: 'op-1', opType: WriteOpType.CreateEntry, payload: '{}', priority: WritePriority.Normal },
      ];
      expect(() => service.restore(ops)).not.toThrow();
    });
  });

  describe('auto-flush', () => {
    beforeEach(async () => {
      await service.initialize();
    });

    it('should start auto-flush', () => {
      const callback = jasmine.createSpy('callback');
      expect(() => service.startAutoFlush(callback, 100)).not.toThrow();
      service.stopAutoFlush();
    });

    it('should stop auto-flush', () => {
      const callback = jasmine.createSpy('callback');
      service.startAutoFlush(callback);
      expect(() => service.stopAutoFlush()).not.toThrow();
    });
  });
});
