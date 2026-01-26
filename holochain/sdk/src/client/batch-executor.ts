/**
 * Batch Executor
 *
 * Handles splitting large operations into smaller batches to avoid WASM memory limits.
 * Similar to Spring's @Transactional batching behavior.
 */

import { ZomeClient } from './zome-client.js';
import {
  DEFAULT_BATCH_SIZE,
  MAX_BATCH_SIZE,
  type CreateContentInput,
  type CreateRelationshipInput,
  type RelationshipOutput,
} from '../types.js';

export interface BatchExecutorConfig {
  batchSize?: number;
  onBatchComplete?: (batchIndex: number, totalBatches: number) => void;
  onError?: (error: Error, batchIndex: number) => 'continue' | 'stop';
}

export interface BatchResult<T> {
  success: T[];
  errors: Array<{ index: number; error: string }>;
  totalProcessed: number;
  totalBatches: number;
}

/**
 * Batch executor for handling large operations
 *
 * Automatically splits arrays into batches of DEFAULT_BATCH_SIZE (50)
 * to avoid WASM memory limits in Holochain.
 */
export class BatchExecutor {
  private client: ZomeClient;
  private config: Required<BatchExecutorConfig>;

  constructor(client: ZomeClient, config: BatchExecutorConfig = {}) {
    this.client = client;
    this.config = {
      batchSize: Math.min(config.batchSize ?? DEFAULT_BATCH_SIZE, MAX_BATCH_SIZE),
      onBatchComplete: config.onBatchComplete ?? (() => {}),
      onError: config.onError ?? (() => 'stop'),
    };
  }

  /**
   * Bulk create content with automatic batching
   */
  async bulkCreateContent(
    contents: CreateContentInput[],
    importId?: string
  ): Promise<BatchResult<string>> {
    const batches = this.splitIntoBatches(contents);
    const result: BatchResult<string> = {
      success: [],
      errors: [],
      totalProcessed: 0,
      totalBatches: batches.length,
    };

    for (let i = 0; i < batches.length; i++) {
      const batch = batches[i];
      const batchImportId = importId
        ? `${importId}-batch-${i + 1}`
        : `batch-${Date.now()}-${i + 1}`;

      try {
        const output = await this.client.bulkCreateContent({
          import_id: batchImportId,
          contents: batch,
        });

        result.success.push(...output.action_hashes.map((h) => h.toString()));
        result.totalProcessed += output.created_count;

        // Collect errors from this batch
        for (const error of output.errors) {
          result.errors.push({
            index: result.totalProcessed + result.errors.length,
            error,
          });
        }

        this.config.onBatchComplete(i + 1, batches.length);
      } catch (error) {
        const errorMessage =
          error instanceof Error ? error.message : String(error);
        result.errors.push({
          index: i * this.config.batchSize,
          error: `Batch ${i + 1} failed: ${errorMessage}`,
        });

        const action = this.config.onError(
          error instanceof Error ? error : new Error(errorMessage),
          i
        );
        if (action === 'stop') {
          break;
        }
      }
    }

    return result;
  }

  /**
   * Bulk create relationships with automatic batching
   */
  async bulkCreateRelationships(
    relationships: CreateRelationshipInput[]
  ): Promise<BatchResult<RelationshipOutput>> {
    const batches = this.splitIntoBatches(relationships);
    const result: BatchResult<RelationshipOutput> = {
      success: [],
      errors: [],
      totalProcessed: 0,
      totalBatches: batches.length,
    };

    for (let i = 0; i < batches.length; i++) {
      const batch = batches[i];

      for (let j = 0; j < batch.length; j++) {
        const rel = batch[j];
        const globalIndex = i * this.config.batchSize + j;

        try {
          const output = await this.client.createRelationship(rel);
          result.success.push(output);
          result.totalProcessed++;
        } catch (error) {
          const errorMessage =
            error instanceof Error ? error.message : String(error);
          result.errors.push({
            index: globalIndex,
            error: `Relationship ${rel.source_id}->${rel.target_id} failed: ${errorMessage}`,
          });

          const action = this.config.onError(
            error instanceof Error ? error : new Error(errorMessage),
            i
          );
          if (action === 'stop') {
            return result;
          }
        }
      }

      this.config.onBatchComplete(i + 1, batches.length);
    }

    return result;
  }

  /**
   * Split an array into batches of configured size
   */
  private splitIntoBatches<T>(items: T[]): T[][] {
    const batches: T[][] = [];
    for (let i = 0; i < items.length; i += this.config.batchSize) {
      batches.push(items.slice(i, i + this.config.batchSize));
    }
    return batches;
  }
}

/**
 * Create a batch executor with default configuration
 */
export function createBatchExecutor(
  client: ZomeClient,
  config?: BatchExecutorConfig
): BatchExecutor {
  return new BatchExecutor(client, config);
}
