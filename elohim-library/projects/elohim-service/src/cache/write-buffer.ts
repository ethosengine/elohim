/**
 * Write Buffer - Batched write operations with priority queues
 *
 * Protects the conductor from heavy write loads during seeding, sync,
 * and recovery operations. Provides backpressure signaling and retry logic.
 *
 * Supports both WASM (high-performance) and pure TypeScript (portable) backends.
 *
 * Priority Levels:
 * 1. High - Identity, authentication, critical state (flushed immediately)
 * 2. Normal - Regular content updates (batched)
 * 3. Bulk - Seeding, imports, recovery sync (heavily batched, throttled)
 *
 * Usage:
 * ```typescript
 * import { createWriteBuffer, WritePriority, WriteOpType } from '@aspect/elohim-service/cache';
 *
 * const { buffer } = await createWriteBuffer({ preset: 'seeding' });
 *
 * // Queue writes
 * buffer.queueWrite('op1', WriteOpType.CreateEntry, '{"type":"ContentNode",...}', WritePriority.Bulk);
 * buffer.queueWrite('op2', WriteOpType.CreateLink, '{"base":"...", "target":"..."}', WritePriority.Bulk);
 *
 * // Check if flush needed
 * if (buffer.shouldFlush()) {
 *   const result = buffer.getPendingBatch();
 *   if (result.hasBatch) {
 *     // Send batch to conductor...
 *     // On success:
 *     buffer.markBatchCommitted(result.batch!.batchId);
 *     // On failure:
 *     buffer.markBatchFailed(result.batch!.batchId, 'conductor_unavailable');
 *   }
 * }
 * ```
 */

// ============================================================================
// Types
// ============================================================================

/** Priority level for write operations */
export enum WritePriority {
  /** Critical writes: identity, auth, consent - flush immediately */
  High = 0,
  /** Normal content updates - batch moderately */
  Normal = 1,
  /** Bulk operations: seeding, sync, recovery - batch aggressively */
  Bulk = 2,
}

/** Type of write operation */
export enum WriteOpType {
  /** Create a new entry */
  CreateEntry = 0,
  /** Update an existing entry */
  UpdateEntry = 1,
  /** Delete an entry */
  DeleteEntry = 2,
  /** Create a link between entries */
  CreateLink = 3,
  /** Delete a link */
  DeleteLink = 4,
}

/** A single write operation waiting to be flushed */
export interface WriteOperation {
  /** Unique operation ID (for deduplication and tracking) */
  opId: string;
  /** Type of write operation */
  opType: WriteOpType;
  /** Serialized payload (entry data, link data, etc.) */
  payload: string;
  /** Priority level */
  priority: WritePriority;
  /** When this operation was queued (ms since epoch) */
  queuedAt: number;
  /** Number of retry attempts */
  retryCount: number;
  /** Deduplication key (e.g., entry hash for updates) */
  dedupKey: string | null;
}

/** A batch of write operations ready to send to conductor */
export interface WriteBatch {
  /** Unique batch ID */
  batchId: string;
  /** Operations in this batch */
  operations: WriteOperation[];
  /** When this batch was created (ms since epoch) */
  createdAt: number;
  /** Priority of this batch (highest priority of contained ops) */
  priority: WritePriority;
}

/** Result of getting a pending batch */
export interface BatchResult {
  /** Whether a batch is available */
  hasBatch: boolean;
  /** The batch (if available) */
  batch: WriteBatch | null;
  /** Number of remaining operations across all queues */
  remainingCount: number;
}

/** Statistics about write buffer state */
export interface WriteBufferStats {
  /** Operations in high priority queue */
  highQueueCount: number;
  /** Operations in normal priority queue */
  normalQueueCount: number;
  /** Operations in bulk priority queue */
  bulkQueueCount: number;
  /** Operations waiting for retry */
  retryQueueCount: number;
  /** Total batches flushed */
  batchesFlushed: number;
  /** Total operations committed */
  opsCommitted: number;
  /** Total operations failed (after all retries) */
  opsFailed: number;
  /** Total operations deduplicated (collapsed) */
  opsDeduplicated: number;
  /** Current backpressure level (0-100) */
  backpressure: number;
}

/** Write buffer interface */
export interface IWriteBuffer {
  /** Queue a write operation */
  queueWrite(opId: string, opType: WriteOpType, payload: string, priority: WritePriority): boolean;

  /** Queue a write operation with deduplication key */
  queueWriteWithDedup(
    opId: string,
    opType: WriteOpType,
    payload: string,
    priority: WritePriority,
    dedupKey: string | null
  ): boolean;

  /** Check if buffer should be flushed */
  shouldFlush(): boolean;

  /** Get the next batch of operations to send */
  getPendingBatch(): BatchResult;

  /** Mark a batch as successfully committed */
  markBatchCommitted(batchId: string): void;

  /** Mark a batch as failed, queuing operations for retry */
  markBatchFailed(batchId: string, error: string): void;

  /** Mark specific operations within a batch as failed */
  markOperationsFailed(batchId: string, failedOpIds: string[]): void;

  /** Get total number of queued operations */
  totalQueued(): number;

  /** Get number of in-flight batches */
  inFlightCount(): number;

  /** Get current backpressure level (0-100) */
  backpressure(): number;

  /** Check if buffer is under backpressure */
  isBackpressured(): boolean;

  /** Get statistics */
  getStats(): WriteBufferStats;

  /** Reset statistics (but keep queued operations) */
  resetStats(): void;

  /** Set maximum queue size for backpressure */
  setMaxQueueSize(size: number): void;

  /** Clear all queued operations */
  clear(): void;

  /** Drain all queues and return remaining operations */
  drainAll(): WriteOperation[];

  /** Restore operations (e.g., after restart) */
  restore(operations: WriteOperation[]): void;

  /** Cleanup resources */
  dispose(): void;
}

/** Configuration for buffer creation */
export interface WriteBufferConfig {
  /** Preset configuration */
  preset?: 'seeding' | 'interactive' | 'recovery';
  /** Max operations per batch (default: 50) */
  batchSize?: number;
  /** Auto-flush interval in ms (default: 100) */
  flushIntervalMs?: number;
  /** Max retry attempts for failed ops (default: 3) */
  maxRetries?: number;
  /** Prefer WASM implementation (default: true) */
  preferWasm?: boolean;
}

/** Initialization result */
export interface WriteBufferInitResult {
  buffer: IWriteBuffer;
  implementation: 'wasm' | 'typescript';
}

// ============================================================================
// WASM Module Types
// ============================================================================

interface WasmWriteBuffer {
  queue_write(opId: string, opType: number, payload: string, priority: number): boolean;
  queue_write_with_dedup(
    opId: string,
    opType: number,
    payload: string,
    priority: number,
    dedupKey: string | null
  ): boolean;
  should_flush(): boolean;
  get_pending_batch(): string;
  mark_batch_committed(batchId: string): void;
  mark_batch_failed(batchId: string, error: string): void;
  mark_operations_failed(batchId: string, failedOpIdsJson: string): void;
  total_queued(): number;
  in_flight_count(): number;
  backpressure(): number;
  is_backpressured(): boolean;
  get_stats(): string;
  reset_stats(): void;
  set_max_queue_size(size: number): void;
  clear(): void;
  drain_all(): string;
  restore(operationsJson: string): void;
  free(): void;
}

interface WasmModule {
  WriteBuffer: new (batchSize: number, flushIntervalMs: bigint, maxRetries: number) => WasmWriteBuffer;
  WritePriority: { High: number; Normal: number; Bulk: number };
  WriteOpType: {
    CreateEntry: number;
    UpdateEntry: number;
    DeleteEntry: number;
    CreateLink: number;
    DeleteLink: number;
  };
}

// ============================================================================
// TypeScript Implementation (Fallback)
// ============================================================================

/**
 * Pure TypeScript write buffer implementation.
 */
export class TsWriteBuffer implements IWriteBuffer {
  private highQueue: WriteOperation[] = [];
  private normalQueue: WriteOperation[] = [];
  private bulkQueue: WriteOperation[] = [];
  private retryQueue: WriteOperation[] = [];

  private dedupIndex = new Map<string, string>(); // dedupKey -> opId
  private inFlight = new Map<string, WriteBatch>();

  private lastFlushAt: number;
  private nextBatchId = 0;

  private batchesFlushed = 0;
  private opsCommitted = 0;
  private opsFailed = 0;
  private opsDeduplicated = 0;

  private maxQueueSize: number;

  constructor(
    private batchSize: number = 50,
    private flushIntervalMs: number = 100,
    private maxRetries: number = 3
  ) {
    this.lastFlushAt = Date.now();
    this.maxQueueSize = batchSize * 100;
  }

  queueWrite(opId: string, opType: WriteOpType, payload: string, priority: WritePriority): boolean {
    return this.queueWriteWithDedup(opId, opType, payload, priority, null);
  }

  queueWriteWithDedup(
    opId: string,
    opType: WriteOpType,
    payload: string,
    priority: WritePriority,
    dedupKey: string | null
  ): boolean {
    // Check backpressure (but always allow high priority)
    if (priority !== WritePriority.High && this.totalQueued() >= this.maxQueueSize) {
      return false;
    }

    // Handle deduplication
    if (dedupKey !== null) {
      const oldOpId = this.dedupIndex.get(dedupKey);
      if (oldOpId) {
        this.removeFromQueues(oldOpId);
        this.opsDeduplicated++;
      }
      this.dedupIndex.set(dedupKey, opId);
    }

    const op: WriteOperation = {
      opId,
      opType,
      payload,
      priority,
      queuedAt: Date.now(),
      retryCount: 0,
      dedupKey,
    };

    // Add to appropriate queue
    switch (priority) {
      case WritePriority.High:
        this.highQueue.push(op);
        break;
      case WritePriority.Normal:
        this.normalQueue.push(op);
        break;
      case WritePriority.Bulk:
        this.bulkQueue.push(op);
        break;
    }

    return true;
  }

  private removeFromQueues(opId: string): void {
    this.highQueue = this.highQueue.filter(op => op.opId !== opId);
    this.normalQueue = this.normalQueue.filter(op => op.opId !== opId);
    this.bulkQueue = this.bulkQueue.filter(op => op.opId !== opId);
  }

  shouldFlush(): boolean {
    // High priority always flushes immediately
    if (this.highQueue.length > 0) {
      return true;
    }

    // Check if any queue exceeds batch size
    if (this.normalQueue.length >= this.batchSize || this.bulkQueue.length >= this.batchSize) {
      return true;
    }

    // Check flush interval
    const now = Date.now();
    if (now - this.lastFlushAt >= this.flushIntervalMs) {
      return this.totalQueued() > 0;
    }

    // Check retry queue
    return this.retryQueue.length > 0;
  }

  getPendingBatch(): BatchResult {
    const now = Date.now();
    this.lastFlushAt = now;

    let operations: WriteOperation[];
    let priority: WritePriority;

    if (this.highQueue.length > 0) {
      // High priority: drain all
      operations = [...this.highQueue];
      this.highQueue = [];
      priority = WritePriority.High;
    } else if (this.retryQueue.length > 0) {
      // Retry queue: take up to batchSize
      const count = Math.min(this.retryQueue.length, this.batchSize);
      operations = this.retryQueue.splice(0, count);
      priority = WritePriority.Normal;
    } else if (this.normalQueue.length > 0) {
      // Normal queue: take up to batchSize
      const count = Math.min(this.normalQueue.length, this.batchSize);
      operations = this.normalQueue.splice(0, count);
      priority = WritePriority.Normal;
    } else if (this.bulkQueue.length > 0) {
      // Bulk queue: take up to batchSize
      const count = Math.min(this.bulkQueue.length, this.batchSize);
      operations = this.bulkQueue.splice(0, count);
      priority = WritePriority.Bulk;
    } else {
      return { hasBatch: false, batch: null, remainingCount: 0 };
    }

    // Clean up dedup index
    for (const op of operations) {
      if (op.dedupKey !== null) {
        this.dedupIndex.delete(op.dedupKey);
      }
    }

    // Create batch
    const batchId = `batch-${this.nextBatchId++}`;
    const batch: WriteBatch = {
      batchId,
      operations,
      createdAt: now,
      priority,
    };

    // Track in-flight
    this.inFlight.set(batchId, batch);
    this.batchesFlushed++;

    return {
      hasBatch: true,
      batch,
      remainingCount: this.totalQueued(),
    };
  }

  markBatchCommitted(batchId: string): void {
    const batch = this.inFlight.get(batchId);
    if (batch) {
      this.opsCommitted += batch.operations.length;
      this.inFlight.delete(batchId);
    }
  }

  markBatchFailed(batchId: string, _error: string): void {
    const batch = this.inFlight.get(batchId);
    if (batch) {
      for (const op of batch.operations) {
        op.retryCount++;
        if (op.retryCount <= this.maxRetries) {
          this.retryQueue.push(op);
        } else {
          this.opsFailed++;
        }
      }
      this.inFlight.delete(batchId);
    }
  }

  markOperationsFailed(batchId: string, failedOpIds: string[]): void {
    const batch = this.inFlight.get(batchId);
    if (batch) {
      for (const op of batch.operations) {
        if (failedOpIds.includes(op.opId)) {
          op.retryCount++;
          if (op.retryCount <= this.maxRetries) {
            this.retryQueue.push(op);
          } else {
            this.opsFailed++;
          }
        } else {
          this.opsCommitted++;
        }
      }
      this.inFlight.delete(batchId);
    }
  }

  totalQueued(): number {
    return (
      this.highQueue.length +
      this.normalQueue.length +
      this.bulkQueue.length +
      this.retryQueue.length
    );
  }

  inFlightCount(): number {
    return this.inFlight.size;
  }

  backpressure(): number {
    const ratio = this.totalQueued() / this.maxQueueSize;
    return Math.min(Math.round(ratio * 100), 100);
  }

  isBackpressured(): boolean {
    return this.backpressure() >= 80;
  }

  getStats(): WriteBufferStats {
    return {
      highQueueCount: this.highQueue.length,
      normalQueueCount: this.normalQueue.length,
      bulkQueueCount: this.bulkQueue.length,
      retryQueueCount: this.retryQueue.length,
      batchesFlushed: this.batchesFlushed,
      opsCommitted: this.opsCommitted,
      opsFailed: this.opsFailed,
      opsDeduplicated: this.opsDeduplicated,
      backpressure: this.backpressure(),
    };
  }

  resetStats(): void {
    this.batchesFlushed = 0;
    this.opsCommitted = 0;
    this.opsFailed = 0;
    this.opsDeduplicated = 0;
  }

  setMaxQueueSize(size: number): void {
    this.maxQueueSize = Math.max(size, this.batchSize);
  }

  clear(): void {
    this.highQueue = [];
    this.normalQueue = [];
    this.bulkQueue = [];
    this.retryQueue = [];
    this.dedupIndex.clear();
    // Note: in_flight batches are NOT cleared (they're already sent)
  }

  drainAll(): WriteOperation[] {
    const all = [
      ...this.highQueue,
      ...this.normalQueue,
      ...this.bulkQueue,
      ...this.retryQueue,
    ];
    this.highQueue = [];
    this.normalQueue = [];
    this.bulkQueue = [];
    this.retryQueue = [];
    this.dedupIndex.clear();
    return all;
  }

  restore(operations: WriteOperation[]): void {
    for (const op of operations) {
      // Restore dedup index
      if (op.dedupKey !== null) {
        this.dedupIndex.set(op.dedupKey, op.opId);
      }

      // Add to appropriate queue
      if (op.retryCount > 0) {
        this.retryQueue.push(op);
      } else {
        switch (op.priority) {
          case WritePriority.High:
            this.highQueue.push(op);
            break;
          case WritePriority.Normal:
            this.normalQueue.push(op);
            break;
          case WritePriority.Bulk:
            this.bulkQueue.push(op);
            break;
        }
      }
    }
  }

  dispose(): void {
    this.clear();
    this.inFlight.clear();
  }
}

// ============================================================================
// WASM Wrapper
// ============================================================================

class WasmWriteBufferWrapper implements IWriteBuffer {
  constructor(private wasm: WasmWriteBuffer) {}

  queueWrite(opId: string, opType: WriteOpType, payload: string, priority: WritePriority): boolean {
    return this.wasm.queue_write(opId, opType, payload, priority);
  }

  queueWriteWithDedup(
    opId: string,
    opType: WriteOpType,
    payload: string,
    priority: WritePriority,
    dedupKey: string | null
  ): boolean {
    return this.wasm.queue_write_with_dedup(opId, opType, payload, priority, dedupKey);
  }

  shouldFlush(): boolean {
    return this.wasm.should_flush();
  }

  getPendingBatch(): BatchResult {
    const json = this.wasm.get_pending_batch();
    const parsed = JSON.parse(json);

    if (!parsed.has_batch) {
      return { hasBatch: false, batch: null, remainingCount: 0 };
    }

    const rawBatch = parsed.batch;
    const batch: WriteBatch = {
      batchId: rawBatch.batch_id,
      operations: rawBatch.operations.map((op: {
        op_id: string;
        op_type: number;
        payload: string;
        priority: number;
        queued_at: number;
        retry_count: number;
        dedup_key: string | null;
      }) => ({
        opId: op.op_id,
        opType: op.op_type as WriteOpType,
        payload: op.payload,
        priority: op.priority as WritePriority,
        queuedAt: op.queued_at,
        retryCount: op.retry_count,
        dedupKey: op.dedup_key,
      })),
      createdAt: rawBatch.created_at,
      priority: rawBatch.priority as WritePriority,
    };

    return {
      hasBatch: true,
      batch,
      remainingCount: parsed.remaining_count,
    };
  }

  markBatchCommitted(batchId: string): void {
    this.wasm.mark_batch_committed(batchId);
  }

  markBatchFailed(batchId: string, error: string): void {
    this.wasm.mark_batch_failed(batchId, error);
  }

  markOperationsFailed(batchId: string, failedOpIds: string[]): void {
    this.wasm.mark_operations_failed(batchId, JSON.stringify(failedOpIds));
  }

  totalQueued(): number {
    return this.wasm.total_queued();
  }

  inFlightCount(): number {
    return this.wasm.in_flight_count();
  }

  backpressure(): number {
    return this.wasm.backpressure();
  }

  isBackpressured(): boolean {
    return this.wasm.is_backpressured();
  }

  getStats(): WriteBufferStats {
    const json = this.wasm.get_stats();
    const parsed = JSON.parse(json);
    return {
      highQueueCount: parsed.high_queue_count,
      normalQueueCount: parsed.normal_queue_count,
      bulkQueueCount: parsed.bulk_queue_count,
      retryQueueCount: parsed.retry_queue_count,
      batchesFlushed: parsed.batches_flushed,
      opsCommitted: parsed.ops_committed,
      opsFailed: parsed.ops_failed,
      opsDeduplicated: parsed.ops_deduplicated,
      backpressure: parsed.backpressure,
    };
  }

  resetStats(): void {
    this.wasm.reset_stats();
  }

  setMaxQueueSize(size: number): void {
    this.wasm.set_max_queue_size(size);
  }

  clear(): void {
    this.wasm.clear();
  }

  drainAll(): WriteOperation[] {
    const json = this.wasm.drain_all();
    const parsed = JSON.parse(json);
    return parsed.map((op: {
      op_id: string;
      op_type: number;
      payload: string;
      priority: number;
      queued_at: number;
      retry_count: number;
      dedup_key: string | null;
    }) => ({
      opId: op.op_id,
      opType: op.op_type as WriteOpType,
      payload: op.payload,
      priority: op.priority as WritePriority,
      queuedAt: op.queued_at,
      retryCount: op.retry_count,
      dedupKey: op.dedup_key,
    }));
  }

  restore(operations: WriteOperation[]): void {
    const json = JSON.stringify(
      operations.map(op => ({
        op_id: op.opId,
        op_type: op.opType,
        payload: op.payload,
        priority: op.priority,
        queued_at: op.queuedAt,
        retry_count: op.retryCount,
        dedup_key: op.dedupKey,
      }))
    );
    this.wasm.restore(json);
  }

  dispose(): void {
    this.wasm.free();
  }
}

// ============================================================================
// Factory Functions
// ============================================================================

let wasmModule: WasmModule | null = null;

async function loadWasmModule(): Promise<WasmModule | null> {
  if (wasmModule) return wasmModule;

  try {
    // Dynamic import of WASM module from assets path
    // In browser: loads from /wasm/holochain-cache-core/
    // Falls back to TypeScript if WASM not available
    const wasmPath = '/wasm/holochain-cache-core/holochain_cache_core.js';
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const mod: any = await import(/* webpackIgnore: true */ wasmPath);
    await mod.default();
    wasmModule = mod as WasmModule;
    console.log('[WriteBuffer] WASM module loaded successfully');
    return wasmModule;
  } catch (error) {
    console.warn('[WriteBuffer] WASM module not available:', error);
    return null;
  }
}

/**
 * Check if WASM write buffer is available.
 */
export async function isWasmBufferAvailable(): Promise<boolean> {
  const mod = await loadWasmModule();
  return mod !== null;
}

/** Preset configurations */
const PRESETS: Record<string, { batchSize: number; flushIntervalMs: number; maxRetries: number }> = {
  seeding: { batchSize: 100, flushIntervalMs: 50, maxRetries: 5 },
  interactive: { batchSize: 20, flushIntervalMs: 100, maxRetries: 3 },
  recovery: { batchSize: 200, flushIntervalMs: 25, maxRetries: 10 },
};

/**
 * Create a write buffer instance.
 *
 * Prefers WASM implementation for performance, falls back to TypeScript.
 */
export async function createWriteBuffer(
  config?: WriteBufferConfig
): Promise<WriteBufferInitResult> {
  const preset = config?.preset ? PRESETS[config.preset] : null;
  const batchSize = config?.batchSize ?? preset?.batchSize ?? 50;
  const flushIntervalMs = config?.flushIntervalMs ?? preset?.flushIntervalMs ?? 100;
  const maxRetries = config?.maxRetries ?? preset?.maxRetries ?? 3;
  const preferWasm = config?.preferWasm ?? true;

  if (preferWasm) {
    const mod = await loadWasmModule();
    if (mod) {
      try {
        const wasmBuffer = new mod.WriteBuffer(batchSize, BigInt(flushIntervalMs), maxRetries);
        return {
          buffer: new WasmWriteBufferWrapper(wasmBuffer),
          implementation: 'wasm',
        };
      } catch (error) {
        console.warn('[WriteBuffer] WASM instantiation failed:', error);
      }
    }
  }

  // Fallback to TypeScript
  return {
    buffer: new TsWriteBuffer(batchSize, flushIntervalMs, maxRetries),
    implementation: 'typescript',
  };
}

/**
 * Create a write buffer with seeding preset.
 * Larger batches, faster flush, more retries.
 */
export async function createSeedingBuffer(): Promise<WriteBufferInitResult> {
  return createWriteBuffer({ preset: 'seeding' });
}

/**
 * Create a write buffer with interactive preset.
 * Smaller batches, responsive.
 */
export async function createInteractiveBuffer(): Promise<WriteBufferInitResult> {
  return createWriteBuffer({ preset: 'interactive' });
}

/**
 * Create a write buffer with recovery/sync preset.
 * Large batches, fast flush, many retries.
 */
export async function createRecoveryBuffer(): Promise<WriteBufferInitResult> {
  return createWriteBuffer({ preset: 'recovery' });
}
