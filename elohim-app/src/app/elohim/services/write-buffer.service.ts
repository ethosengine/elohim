/**
 * Write Buffer Service - Angular Wrapper
 *
 * Provides Angular dependency injection wrapper around the framework-agnostic
 * write buffer. Automatically initializes WASM or falls back to TypeScript.
 *
 * Protects the conductor from heavy write loads during seeding, sync,
 * and recovery operations.
 *
 * Usage:
 * ```typescript
 * @Component({...})
 * export class SeedingComponent {
 *   constructor(private writeBuffer: WriteBufferService) {}
 *
 *   async ngOnInit() {
 *     await this.writeBuffer.initialize({ preset: 'seeding' });
 *
 *     // Queue bulk writes
 *     for (const content of contents) {
 *       this.writeBuffer.queueWrite(
 *         content.id,
 *         WriteOpType.CreateEntry,
 *         JSON.stringify(content),
 *         WritePriority.Bulk
 *       );
 *     }
 *
 *     // Flush loop
 *     await this.writeBuffer.flushAll(async (batch) => {
 *       await this.conductor.callZome('content_store', 'create_batch', batch);
 *     });
 *   }
 * }
 * ```
 */

import { Injectable, OnDestroy, inject } from '@angular/core';

import { takeUntil } from 'rxjs/operators';

import { BehaviorSubject, Subject, interval } from 'rxjs';

import {
  WritePriority,
  WriteOpType,
  createWriteBuffer,
  isWasmBufferAvailable,
  TsWriteBuffer,
} from '@elohim/service/cache/write-buffer';

import { LoggerService } from './logger.service';

// Import from framework-agnostic write buffer
import type {
  IWriteBuffer,
  WriteOperation,
  WriteBatch,
  BatchResult,
  WriteBufferStats,
  WriteBufferConfig,
} from '@elohim/service/cache/write-buffer';

// Re-export types and enums for convenience
export {
  WritePriority,
  WriteOpType,
  type IWriteBuffer,
  type WriteOperation,
  type WriteBatch,
  type BatchResult,
  type WriteBufferStats,
};

/** Service state */
export type BufferServiceState = 'uninitialized' | 'initializing' | 'ready' | 'flushing' | 'error';

/** Initialization result */
export interface BufferInitializationResult {
  success: boolean;
  implementation: 'wasm' | 'typescript';
  error?: string;
}

/**
 * Result from processing individual operations in a batch.
 * Allows tracking per-operation success/failure.
 */
export interface BatchOperationResult {
  /** Operation ID */
  opId: string;
  /** Whether this specific operation succeeded */
  success: boolean;
  /** Error message if failed */
  error?: string;
}

/**
 * Result from a batch callback.
 * Can indicate partial success (some ops succeeded, some failed).
 */
export interface BatchCallbackResult {
  /** Overall batch status */
  success: boolean;
  /** Per-operation results (if available) */
  operationResults?: BatchOperationResult[];
  /** Global error message if entire batch failed */
  error?: string;
}

/**
 * Flush callback type.
 * Can return void (all-or-nothing) or BatchCallbackResult (partial success).
 */
export type FlushCallback = (batch: WriteBatch) => Promise<void | BatchCallbackResult>;

/** Flush result with detailed operation tracking */
export interface FlushResult {
  /** Whether all operations succeeded */
  success: boolean;
  /** Batch ID */
  batchId: string;
  /** Total operations in batch */
  operationCount: number;
  /** Number of operations that succeeded */
  successCount: number;
  /** Number of operations that failed */
  failureCount: number;
  /** IDs of failed operations (for debugging/retry) */
  failedOperationIds: string[];
  /** Error message if any failures */
  error?: string;
}

/** Result from flushing all operations */
export interface FlushAllResult {
  /** Total operations successfully committed */
  totalCommitted: number;
  /** Total operations that failed */
  totalFailed: number;
  /** Number of batches processed */
  batchCount: number;
  /** IDs of operations that failed (across all batches) */
  failedOperationIds: string[];
}

/**
 * Angular service providing write buffering for Holochain operations.
 *
 * Features:
 * - Automatic WASM/TypeScript selection
 * - Priority-based queuing (High → Normal → Bulk)
 * - Deduplication (last write wins)
 * - Retry logic with backoff
 * - Backpressure signaling
 * - Observable state for reactive UIs
 */
@Injectable({
  providedIn: 'root',
})
export class WriteBufferService implements OnDestroy {
  private readonly logger = inject(LoggerService).createChild('WriteBuffer');
  private buffer: IWriteBuffer | null = null;

  private readonly stateSubject = new BehaviorSubject<BufferServiceState>('uninitialized');
  private readonly statsSubject = new BehaviorSubject<WriteBufferStats | null>(null);
  private readonly backpressureSubject = new BehaviorSubject<number>(0);

  private implementation: 'wasm' | 'typescript' = 'typescript';
  private initPromise: Promise<BufferInitializationResult> | null = null;

  private autoFlushInterval: number | null = null;
  private flushCallback: FlushCallback | null = null;
  private readonly destroy$ = new Subject<void>();

  /** Observable buffer service state */
  readonly state$ = this.stateSubject.asObservable();

  /** Observable statistics */
  readonly stats$ = this.statsSubject.asObservable();

  /** Observable backpressure level (0-100) */
  readonly backpressure$ = this.backpressureSubject.asObservable();

  /** Current state */
  get state(): BufferServiceState {
    return this.stateSubject.value;
  }

  /** Current implementation type */
  get implementationType(): 'wasm' | 'typescript' {
    return this.implementation;
  }

  /** Check if service is ready */
  get isReady(): boolean {
    return this.state === 'ready' || this.state === 'flushing';
  }

  /** Check if currently flushing */
  get isFlushing(): boolean {
    return this.state === 'flushing';
  }

  /**
   * Initialize the write buffer service.
   * Call this before using any buffer operations.
   *
   * @param config - Optional configuration
   * @returns Initialization result
   */
  async initialize(config?: WriteBufferConfig): Promise<BufferInitializationResult> {
    // Return existing promise if already initializing
    if (this.initPromise) {
      return this.initPromise;
    }

    // Already initialized
    if (this.state === 'ready' || this.state === 'flushing') {
      return {
        success: true,
        implementation: this.implementation,
      };
    }

    this.stateSubject.next('initializing');
    this.initPromise = this.doInitialize(config);
    return this.initPromise;
  }

  private async doInitialize(config?: WriteBufferConfig): Promise<BufferInitializationResult> {
    try {
      const result = await createWriteBuffer(config);
      this.buffer = result.buffer;
      this.implementation = result.implementation;

      this.stateSubject.next('ready');
      this.updateStats();

      this.logger.info('Initialized', { implementation: this.implementation });

      return {
        success: true,
        implementation: this.implementation,
      };
    } catch (error) {
      this.logger.error(
        'Initialization failed',
        error instanceof Error ? error : new Error(String(error))
      );

      // Fallback to TypeScript implementation
      try {
        this.buffer = new TsWriteBuffer();
        this.implementation = 'typescript';
        this.stateSubject.next('ready');
        this.updateStats();

        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        return {
          success: true,
          implementation: 'typescript',
          error: `WASM failed, using TypeScript fallback: ${errorMessage}`,
        };
      } catch (fallbackError) {
        this.stateSubject.next('error');
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        return {
          success: false,
          implementation: 'typescript',
          error: `All initialization attempts failed: ${errorMessage}`,
        };
      }
    }
  }

  // ==========================================================================
  // Queuing Operations
  // ==========================================================================

  /**
   * Queue a write operation.
   *
   * @param opId - Unique operation ID
   * @param opType - Type of operation (CreateEntry, CreateLink, etc.)
   * @param payload - Serialized operation data (JSON)
   * @param priority - Priority level (default: Normal)
   * @returns true if queued, false if backpressure is full
   */
  queueWrite(
    opId: string,
    opType: WriteOpType,
    payload: string,
    priority: WritePriority = WritePriority.Normal
  ): boolean {
    this.ensureReady();
    const result = this.buffer!.queueWrite(opId, opType, payload, priority);
    this.updateStats();
    return result;
  }

  /**
   * Queue a write operation with deduplication.
   *
   * If another operation with the same dedupKey is already queued,
   * the old one is replaced (last write wins).
   */
  queueWriteWithDedup(
    opId: string,
    opType: WriteOpType,
    payload: string,
    priority: WritePriority,
    dedupKey: string
  ): boolean {
    this.ensureReady();
    const result = this.buffer!.queueWriteWithDedup(opId, opType, payload, priority, dedupKey);
    this.updateStats();
    return result;
  }

  /**
   * Queue a create entry operation.
   * Convenience method for common operation.
   */
  queueCreateEntry(
    opId: string,
    payload: string,
    priority: WritePriority = WritePriority.Normal
  ): boolean {
    return this.queueWrite(opId, WriteOpType.CreateEntry, payload, priority);
  }

  /**
   * Queue an update entry operation with deduplication.
   * Uses the entry hash as dedup key (last update wins).
   */
  queueUpdateEntry(
    opId: string,
    entryHash: string,
    payload: string,
    priority: WritePriority = WritePriority.Normal
  ): boolean {
    return this.queueWriteWithDedup(opId, WriteOpType.UpdateEntry, payload, priority, entryHash);
  }

  /**
   * Queue a create link operation.
   */
  queueCreateLink(
    opId: string,
    payload: string,
    priority: WritePriority = WritePriority.Normal
  ): boolean {
    return this.queueWrite(opId, WriteOpType.CreateLink, payload, priority);
  }

  // ==========================================================================
  // Flushing
  // ==========================================================================

  /**
   * Check if buffer should be flushed.
   */
  shouldFlush(): boolean {
    this.ensureReady();
    return this.buffer!.shouldFlush();
  }

  /**
   * Get the next batch of operations to send.
   */
  getPendingBatch(): BatchResult {
    this.ensureReady();
    return this.buffer!.getPendingBatch();
  }

  /**
   * Flush a single batch using the provided callback.
   *
   * Supports partial success - if the callback returns BatchCallbackResult
   * with per-operation results, only failed operations will be retried.
   *
   * @param callback - Function to send batch to conductor
   * @returns Flush result with detailed operation tracking
   */
  async flushBatch(callback: FlushCallback): Promise<FlushResult | null> {
    this.ensureReady();

    const batchResult = this.buffer!.getPendingBatch();
    if (!batchResult.hasBatch || !batchResult.batch) {
      return null;
    }

    const batch = batchResult.batch;
    const operationCount = batch.operations.length;
    this.stateSubject.next('flushing');

    try {
      const result = await callback(batch);

      // Handle different result types
      if (!result || result === undefined) {
        // Legacy void callback - treat as all success
        this.buffer!.markBatchCommitted(batch.batchId);
        this.updateStats();
        this.updateStateAfterFlush();

        return {
          success: true,
          batchId: batch.batchId,
          operationCount,
          successCount: operationCount,
          failureCount: 0,
          failedOperationIds: [],
        };
      }

      // BatchCallbackResult - may have partial success
      return this.handleBatchCallbackResult(batch, result);
    } catch (error) {
      // Exception thrown - all operations failed
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      this.buffer!.markBatchFailed(batch.batchId, errorMessage);
      this.updateStats();
      this.stateSubject.next('ready');

      return {
        success: false,
        batchId: batch.batchId,
        operationCount,
        successCount: 0,
        failureCount: operationCount,
        failedOperationIds: batch.operations.map(op => op.opId),
        error: errorMessage,
      };
    }
  }

  /**
   * Process BatchCallbackResult to handle partial success.
   */
  private handleBatchCallbackResult(batch: WriteBatch, result: BatchCallbackResult): FlushResult {
    const operationCount = batch.operations.length;

    if (result.success && !result.operationResults) {
      // All operations succeeded
      this.buffer!.markBatchCommitted(batch.batchId);
      this.updateStats();
      this.updateStateAfterFlush();

      return {
        success: true,
        batchId: batch.batchId,
        operationCount,
        successCount: operationCount,
        failureCount: 0,
        failedOperationIds: [],
      };
    }

    if (result.operationResults && result.operationResults.length > 0) {
      // Per-operation results available - handle partial success
      const failedOps = result.operationResults.filter(op => !op.success);
      const failedOpIds = failedOps.map(op => op.opId);
      const successCount = operationCount - failedOps.length;
      const failureCount = failedOps.length;

      if (failureCount === 0) {
        // All succeeded
        this.buffer!.markBatchCommitted(batch.batchId);
      } else if (failureCount === operationCount) {
        // All failed
        const errorMsg = failedOps[0]?.error || result.error || 'All operations failed';
        this.buffer!.markBatchFailed(batch.batchId, errorMsg);
      } else {
        // Partial success - only retry failed operations
        this.buffer!.markOperationsFailed(batch.batchId, failedOpIds);
        this.logger.info('Partial batch success', {
          successCount,
          operationCount,
          failureCount,
          retriesQueued: failureCount,
        });
      }

      this.updateStats();
      this.updateStateAfterFlush();

      const firstError = failedOps[0]?.error || result.error;
      return {
        success: failureCount === 0,
        batchId: batch.batchId,
        operationCount,
        successCount,
        failureCount,
        failedOperationIds: failedOpIds,
        error: failureCount > 0 ? firstError : undefined,
      };
    }

    // success: false with no operationResults - all failed
    const errorMessage = result.error || 'Batch failed';
    this.buffer!.markBatchFailed(batch.batchId, errorMessage);
    this.updateStats();
    this.stateSubject.next('ready');

    return {
      success: false,
      batchId: batch.batchId,
      operationCount,
      successCount: 0,
      failureCount: operationCount,
      failedOperationIds: batch.operations.map(op => op.opId),
      error: errorMessage,
    };
  }

  /**
   * Update state after flush completes.
   */
  private updateStateAfterFlush(): void {
    if (this.buffer!.totalQueued() === 0) {
      this.stateSubject.next('ready');
    } else {
      // Still have items - stay in ready (not flushing) until next batch
      this.stateSubject.next('ready');
    }
  }

  /**
   * Flush all pending operations using the provided callback.
   *
   * Continues flushing until all queues are empty.
   * Supports partial success - failed operations are automatically retried.
   *
   * @param callback - Function to send batch to conductor
   * @param onProgress - Optional progress callback with detailed results
   * @returns Detailed flush results including success/failure counts
   */
  async flushAll(
    callback: FlushCallback,
    onProgress?: (committed: number, remaining: number, failed?: number) => void
  ): Promise<number> {
    this.ensureReady();

    let totalCommitted = 0;
    let totalFailed = 0;
    const allFailedIds: string[] = [];
    let batchCount = 0;
    let consecutiveFailures = 0;
    const maxConsecutiveFailures = 3;

    while (this.buffer!.shouldFlush()) {
      const result = await this.flushBatch(callback);
      batchCount++;

      if (result) {
        totalCommitted += result.successCount;
        totalFailed += result.failureCount;
        allFailedIds.push(...result.failedOperationIds);

        if (result.success) {
          consecutiveFailures = 0;
        } else {
          consecutiveFailures++;

          // Stop if we're getting too many consecutive complete failures
          if (consecutiveFailures >= maxConsecutiveFailures && result.successCount === 0) {
            this.logger.warn('Stopping flush after consecutive failures', {
              consecutiveFailures,
              totalCommitted,
              totalFailed,
            });
            break;
          }
        }
      }

      if (onProgress) {
        onProgress(totalCommitted, this.buffer!.totalQueued(), totalFailed);
      }

      // Small delay between batches to prevent overwhelming conductor
      await new Promise(resolve => setTimeout(resolve, 10));
    }

    this.stateSubject.next('ready');

    // Log summary if there were failures
    if (totalFailed > 0) {
      this.logger.warn('FlushAll completed with partial success', {
        totalCommitted,
        totalFailed,
        batchCount,
      });
    }

    return totalCommitted;
  }

  /**
   * Flush all pending operations with detailed result tracking.
   *
   * Unlike flushAll(), returns complete details about what succeeded/failed.
   *
   * @param callback - Function to send batch to conductor
   * @param onProgress - Optional progress callback
   * @returns Detailed flush results
   */
  async flushAllWithDetails(
    callback: FlushCallback,
    onProgress?: (committed: number, remaining: number, failed?: number) => void
  ): Promise<{
    totalCommitted: number;
    totalFailed: number;
    batchCount: number;
    failedOperationIds: string[];
  }> {
    this.ensureReady();

    let totalCommitted = 0;
    let totalFailed = 0;
    const allFailedIds: string[] = [];
    let batchCount = 0;
    let consecutiveFailures = 0;
    const maxConsecutiveFailures = 3;

    while (this.buffer!.shouldFlush()) {
      const result = await this.flushBatch(callback);
      batchCount++;

      if (result) {
        totalCommitted += result.successCount;
        totalFailed += result.failureCount;
        allFailedIds.push(...result.failedOperationIds);

        if (result.success) {
          consecutiveFailures = 0;
        } else {
          consecutiveFailures++;

          if (consecutiveFailures >= maxConsecutiveFailures && result.successCount === 0) {
            this.logger.warn('Stopping flush after consecutive failures', {
              consecutiveFailures,
              totalCommitted,
              totalFailed,
            });
            break;
          }
        }
      }

      if (onProgress) {
        onProgress(totalCommitted, this.buffer!.totalQueued(), totalFailed);
      }

      await new Promise(resolve => setTimeout(resolve, 10));
    }

    this.stateSubject.next('ready');

    return {
      totalCommitted,
      totalFailed,
      batchCount,
      failedOperationIds: allFailedIds,
    };
  }

  /**
   * Start automatic flushing at regular intervals.
   *
   * @param callback - Function to send batch to conductor
   * @param intervalMs - Check interval in milliseconds (default: 100)
   */
  startAutoFlush(callback: FlushCallback, intervalMs = 100): void {
    this.ensureReady();

    this.stopAutoFlush();
    this.flushCallback = callback;

    interval(intervalMs)
      .pipe(takeUntil(this.destroy$))
      .subscribe(async () => {
        if (this.buffer && this.buffer.shouldFlush() && this.flushCallback) {
          await this.flushBatch(this.flushCallback);
        }
      });

    this.logger.debug('Auto-flush started', { intervalMs });
  }

  /**
   * Stop automatic flushing.
   */
  stopAutoFlush(): void {
    this.flushCallback = null;
    // The destroy$ subject will clean up subscriptions in ngOnDestroy
  }

  // ==========================================================================
  // Batch Result Reporting
  // ==========================================================================

  /**
   * Mark a batch as successfully committed.
   */
  markBatchCommitted(batchId: string): void {
    this.ensureReady();
    this.buffer!.markBatchCommitted(batchId);
    this.updateStats();
  }

  /**
   * Mark a batch as failed, queuing operations for retry.
   */
  markBatchFailed(batchId: string, error: string): void {
    this.ensureReady();
    this.buffer!.markBatchFailed(batchId, error);
    this.updateStats();
  }

  /**
   * Mark specific operations within a batch as failed.
   */
  markOperationsFailed(batchId: string, failedOpIds: string[]): void {
    this.ensureReady();
    this.buffer!.markOperationsFailed(batchId, failedOpIds);
    this.updateStats();
  }

  // ==========================================================================
  // Status and Statistics
  // ==========================================================================

  /**
   * Get total number of queued operations.
   */
  totalQueued(): number {
    this.ensureReady();
    return this.buffer!.totalQueued();
  }

  /**
   * Get number of in-flight batches.
   */
  inFlightCount(): number {
    this.ensureReady();
    return this.buffer!.inFlightCount();
  }

  /**
   * Get current backpressure level (0-100).
   */
  currentBackpressure(): number {
    this.ensureReady();
    return this.buffer!.backpressure();
  }

  /**
   * Check if buffer is under backpressure.
   */
  isBackpressured(): boolean {
    this.ensureReady();
    return this.buffer!.isBackpressured();
  }

  /**
   * Get current statistics.
   */
  getStats(): WriteBufferStats {
    this.ensureReady();
    return this.buffer!.getStats();
  }

  /**
   * Reset statistics (but keep queued operations).
   */
  resetStats(): void {
    this.ensureReady();
    this.buffer!.resetStats();
    this.updateStats();
  }

  // ==========================================================================
  // Configuration
  // ==========================================================================

  /**
   * Set maximum queue size for backpressure.
   */
  setMaxQueueSize(size: number): void {
    this.ensureReady();
    this.buffer!.setMaxQueueSize(size);
  }

  // ==========================================================================
  // Persistence
  // ==========================================================================

  /**
   * Clear all queued operations.
   *
   * Warning: This drops all pending writes!
   */
  clear(): void {
    this.ensureReady();
    this.buffer!.clear();
    this.updateStats();
  }

  /**
   * Drain all queues and return remaining operations.
   *
   * Use this for graceful shutdown to persist pending writes.
   */
  drainAll(): WriteOperation[] {
    this.ensureReady();
    const ops = this.buffer!.drainAll();
    this.updateStats();
    return ops;
  }

  /**
   * Restore operations (e.g., after restart).
   *
   * Use with drainAll() for graceful shutdown/restart.
   */
  restore(operations: WriteOperation[]): void {
    this.ensureReady();
    this.buffer!.restore(operations);
    this.updateStats();
  }

  // ==========================================================================
  // Utility Methods
  // ==========================================================================

  /**
   * Check if WASM buffer is available.
   */
  async checkWasmAvailable(): Promise<boolean> {
    return isWasmBufferAvailable();
  }

  private updateStats(): void {
    if (this.buffer) {
      const stats = this.buffer.getStats();
      this.statsSubject.next(stats);
      this.backpressureSubject.next(stats.backpressure);
    }
  }

  // ==========================================================================
  // Lifecycle
  // ==========================================================================

  private ensureReady(): void {
    if (!this.isReady) {
      throw new Error('[WriteBufferService] Service not initialized. Call initialize() first.');
    }
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();

    this.buffer?.dispose();
    this.stateSubject.complete();
    this.statsSubject.complete();
    this.backpressureSubject.complete();
  }
}
