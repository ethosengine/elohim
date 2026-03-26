/**
 * Seeding Service - Batch Content Operations
 *
 * Provides high-level APIs for batch content operations that use
 * WriteBuffer to protect the conductor from heavy write loads.
 *
 * Use cases:
 * - Admin bulk content imports
 * - Edgenode recovery/sync operations
 * - Family network replication
 * - Path/content migrations
 *
 * Usage:
 * ```typescript
 * @Component({...})
 * export class AdminImportComponent {
 *   constructor(private seeding: SeedingService) {}
 *
 *   async importContent(contents: ContentNode[]) {
 *     const result = await this.seeding.bulkCreateContent(contents);
 *     console.log(`Created: ${result.successCount}, Failed: ${result.failureCount}`);
 *   }
 * }
 * ```
 */

import { Injectable, inject, OnDestroy } from '@angular/core';

// @coverage: 79.3% (2026-02-24)

import { BehaviorSubject, Subject, firstValueFrom } from 'rxjs';

import { ContentNode } from '../../lamad/models/content-node.model';

import { StorageClientService, type StorageContentNode } from './storage-client.service';
import {
  WriteBufferService,
  WritePriority,
  WriteOpType,
  type WriteBatch,
  type WriteBufferStats,
} from './write-buffer.service';

/** Seeding operation mode */
export type SeedingMode = 'import' | 'recovery' | 'sync';

/** Batch operation result */
export interface BatchResult {
  successCount: number;
  failureCount: number;
  failedIds: string[];
  durationMs: number;
}

/** Seeding progress info */
export interface SeedingProgress {
  mode: SeedingMode;
  phase: 'queuing' | 'flushing' | 'complete' | 'error';
  totalItems: number;
  processedItems: number;
  successCount: number;
  failureCount: number;
  percentComplete: number;
}

/** Seeding service state */
export type SeedingServiceState = 'idle' | 'seeding' | 'error';

/**
 * Angular service for batch content operations.
 *
 * Features:
 * - Automatic batching via WriteBuffer
 * - Progress tracking
 * - Error recovery
 * - Backpressure handling
 */
@Injectable({ providedIn: 'root' })
export class SeedingService implements OnDestroy {
  private readonly writeBuffer = inject(WriteBufferService);
  private readonly storageClient = inject(StorageClientService);

  private readonly stateSubject = new BehaviorSubject<SeedingServiceState>('idle');
  private readonly progressSubject = new BehaviorSubject<SeedingProgress | null>(null);
  private readonly destroy$ = new Subject<void>();

  /** Observable seeding state */
  readonly state$ = this.stateSubject.asObservable();

  /** Observable seeding progress */
  readonly progress$ = this.progressSubject.asObservable();

  /** WriteBuffer stats passthrough */
  readonly bufferStats$ = this.writeBuffer.stats$;

  /** Backpressure level passthrough */
  readonly backpressure$ = this.writeBuffer.backpressure$;

  get state(): SeedingServiceState {
    return this.stateSubject.value;
  }

  get isSeeding(): boolean {
    return this.state === 'seeding';
  }

  /**
   * Initialize for a seeding operation.
   *
   * @param mode - Seeding mode determines batching strategy
   */
  async initialize(mode: SeedingMode = 'import'): Promise<void> {
    // Initialize WriteBuffer with mode-appropriate settings
    const config = this.getConfigForMode(mode);
    await this.writeBuffer.initialize(config);
  }

  /**
   * Get WriteBuffer config for seeding mode.
   */
  private getConfigForMode(mode: SeedingMode) {
    switch (mode) {
      case 'import':
        // Large batches for bulk imports
        return { batchSize: 100, maxQueueSize: 10000, flushIntervalMs: 500 };
      case 'recovery':
        // Very large batches for recovery (rebuilding from family network)
        return { batchSize: 200, maxQueueSize: 50000, flushIntervalMs: 250 };
      case 'sync':
        // Moderate batches for incremental sync
        return { batchSize: 50, maxQueueSize: 5000, flushIntervalMs: 1000 };
      default:
        return { batchSize: 100, maxQueueSize: 10000, flushIntervalMs: 500 };
    }
  }

  /**
   * Bulk create content nodes.
   *
   * Uses WriteBuffer for efficient batching and conductor protection.
   *
   * @param contents - Array of content nodes to create
   * @param priority - Priority level (default: Bulk for seeding)
   * @returns Batch operation result
   */
  async bulkCreateContent(
    contents: ContentNode[],
    priority: WritePriority = WritePriority.Bulk
  ): Promise<BatchResult> {
    if (!this.writeBuffer.isReady) {
      await this.initialize('import');
    }

    const startTime = performance.now();
    this.stateSubject.next('seeding');

    const progress: SeedingProgress = {
      mode: 'import',
      phase: 'queuing',
      totalItems: contents.length,
      processedItems: 0,
      successCount: 0,
      failureCount: 0,
      percentComplete: 0,
    };
    this.progressSubject.next(progress);

    // Queue all content for writing (camelCase InputView)
    for (const content of contents) {
      const payload = JSON.stringify({
        id: content.id,
        contentType: content.contentType,
        title: content.title,
        description: content.description,
        content: content.content,
        contentFormat: content.contentFormat,
        tags: content.tags,
        relatedNodeIds: content.relatedNodeIds ?? [],
        metadata: content.metadata ?? {},
      });

      this.writeBuffer.queueWrite(content.id, WriteOpType.CreateEntry, payload, priority);
    }

    progress.phase = 'flushing';
    this.progressSubject.next({ ...progress });

    // Flush all batches
    const failedIds: string[] = [];
    const result = await this.flushWithTracking(progress, failedIds);

    progress.phase = 'complete';
    progress.percentComplete = 100;
    this.progressSubject.next({ ...progress });
    this.stateSubject.next('idle');

    return {
      successCount: result.successCount,
      failureCount: result.failureCount,
      failedIds,
      durationMs: performance.now() - startTime,
    };
  }

  /**
   * Recovery sync from family network.
   *
   * Used when an edgenode needs to rebuild from family network replication.
   * Paths are now ContentNodes — pass them in the contents array.
   *
   * @param contents - Content from family network (includes paths as ContentNodes)
   * @returns Batch result
   */
  async recoverySync(contents: ContentNode[]): Promise<BatchResult> {
    await this.initialize('recovery');

    // High priority for recovery - these are the user's actual data
    return this.bulkCreateContent(contents, WritePriority.High);
  }

  /**
   * Incremental sync from projection cache.
   *
   * Used for keeping local cache in sync with projection.
   *
   * @param contents - Updated content from projection
   * @returns Batch result
   */
  async incrementalSync(contents: ContentNode[]): Promise<BatchResult> {
    await this.initialize('sync');
    return this.bulkCreateContent(contents, WritePriority.Normal);
  }

  /**
   * Flush all pending writes with progress tracking.
   */
  private async flushWithTracking(
    progress: SeedingProgress,
    failedIds: string[]
  ): Promise<{ successCount: number; failureCount: number }> {
    let successCount = 0;
    let failureCount = 0;

    await this.writeBuffer.flushAll(async batch => {
      try {
        // Call the appropriate zome function based on operation type
        await this.executeBatch(batch);
        successCount += batch.operations.length;
      } catch {
        // Batch execution failed - zome call error, all operations in batch marked failed
        failureCount += batch.operations.length;
        for (const op of batch.operations) {
          failedIds.push(op.opId);
        }
      }

      progress.processedItems += batch.operations.length;
      progress.successCount = successCount;
      progress.failureCount = failureCount;
      progress.percentComplete = Math.round((progress.processedItems / progress.totalItems) * 100);
      this.progressSubject.next({ ...progress });
    });

    return { successCount, failureCount };
  }

  /**
   * Execute a batch of write operations via StorageClientService.
   *
   * Uses the bulk HTTP endpoints through Doorway → elohim-storage.
   */
  private async executeBatch(batch: WriteBatch): Promise<void> {
    // Group operations by type
    const contentOps = batch.operations.filter(op => op.opType === WriteOpType.CreateEntry);

    if (contentOps.length > 0) {
      // Parse payloads and transform to backend format
      const entries = contentOps.map(op => {
        interface ParsedContent {
          id: string;
          contentType: string;
          title: string;
          description: string;
          content: string;
          contentFormat: string;
          tags?: string[];
          metadataJson?: Record<string, unknown>;
        }
        const parsed = JSON.parse(op.payload) as ParsedContent;
        // Transform to backend format: content → contentBody
        // Cast parsed JSON strings to schema types at the trust boundary
        return {
          id: parsed.id,
          contentType: parsed.contentType,
          title: parsed.title,
          description: parsed.description,
          contentBody: parsed.content,
          contentFormat: parsed.contentFormat,
          tags: parsed.tags ?? [],
          metadataJson: parsed.metadataJson ? JSON.stringify(parsed.metadataJson) : null,
          reach: 'public',
        } as Partial<StorageContentNode>;
      });

      // Call bulk create via HTTP (through Doorway)
      await firstValueFrom(this.storageClient.bulkCreateContent(entries));
    }
  }

  /**
   * Cancel ongoing seeding operation.
   */
  cancel(): void {
    // Clear the write buffer
    this.writeBuffer.clear?.();
    this.stateSubject.next('idle');
    this.progressSubject.next(null);
  }

  /**
   * Get current buffer statistics.
   */
  getBufferStats(): WriteBufferStats | null {
    return this.writeBuffer.getStats();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }
}
