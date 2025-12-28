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

import { Injectable, OnDestroy } from '@angular/core';
import { BehaviorSubject, Subject, interval } from 'rxjs';
import { takeUntil } from 'rxjs/operators';

// Import from framework-agnostic write buffer
import type {
  IWriteBuffer,
  WriteOperation,
  WriteBatch,
  BatchResult,
  WriteBufferStats,
  WriteBufferConfig,
} from '../../../../../elohim-library/projects/elohim-service/src/cache/write-buffer';

import {
  WritePriority,
  WriteOpType,
  createWriteBuffer,
  isWasmBufferAvailable,
  TsWriteBuffer,
} from '../../../../../elohim-library/projects/elohim-service/src/cache/write-buffer';

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

/** Flush callback type */
export type FlushCallback = (batch: WriteBatch) => Promise<void>;

/** Flush result */
export interface FlushResult {
  success: boolean;
  batchId: string;
  operationCount: number;
  error?: string;
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

      console.log(`[WriteBufferService] Initialized with ${this.implementation} implementation`);

      return {
        success: true,
        implementation: this.implementation,
      };
    } catch (error) {
      console.error('[WriteBufferService] Initialization failed:', error);

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
   * @param callback - Function to send batch to conductor
   * @returns Flush result
   */
  async flushBatch(callback: FlushCallback): Promise<FlushResult | null> {
    this.ensureReady();

    const batchResult = this.buffer!.getPendingBatch();
    if (!batchResult.hasBatch || !batchResult.batch) {
      return null;
    }

    const batch = batchResult.batch;
    this.stateSubject.next('flushing');

    try {
      await callback(batch);
      this.buffer!.markBatchCommitted(batch.batchId);
      this.updateStats();

      if (this.buffer!.totalQueued() === 0) {
        this.stateSubject.next('ready');
      }

      return {
        success: true,
        batchId: batch.batchId,
        operationCount: batch.operations.length,
      };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      this.buffer!.markBatchFailed(batch.batchId, errorMessage);
      this.updateStats();
      this.stateSubject.next('ready');

      return {
        success: false,
        batchId: batch.batchId,
        operationCount: batch.operations.length,
        error: errorMessage,
      };
    }
  }

  /**
   * Flush all pending operations using the provided callback.
   *
   * Continues flushing until all queues are empty.
   *
   * @param callback - Function to send batch to conductor
   * @param onProgress - Optional progress callback
   * @returns Total operations committed
   */
  async flushAll(
    callback: FlushCallback,
    onProgress?: (committed: number, remaining: number) => void
  ): Promise<number> {
    this.ensureReady();

    let totalCommitted = 0;

    while (this.buffer!.shouldFlush()) {
      const result = await this.flushBatch(callback);

      if (result?.success) {
        totalCommitted += result.operationCount;
      }

      if (onProgress) {
        onProgress(totalCommitted, this.buffer!.totalQueued());
      }

      // Small delay between batches to prevent overwhelming conductor
      await new Promise(resolve => setTimeout(resolve, 10));
    }

    this.stateSubject.next('ready');
    return totalCommitted;
  }

  /**
   * Start automatic flushing at regular intervals.
   *
   * @param callback - Function to send batch to conductor
   * @param intervalMs - Check interval in milliseconds (default: 100)
   */
  startAutoFlush(callback: FlushCallback, intervalMs: number = 100): void {
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

    console.log(`[WriteBufferService] Auto-flush started (interval: ${intervalMs}ms)`);
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
      throw new Error(
        '[WriteBufferService] Service not initialized. Call initialize() first.'
      );
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
