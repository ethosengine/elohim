/**
 * Holochain Import Service
 *
 * Adapts the existing import pipeline to write to Holochain instead of Kuzu.
 * Reuses: parsers, transformers, ContentNode model
 * Changes: storage layer only
 *
 * Usage:
 * ```typescript
 * const service = new HolochainImportService({
 *   adminUrl: 'wss://doorway-dev.elohim.host',
 *   appId: 'elohim',
 *   batchSize: 50,
 * });
 *
 * const result = await service.importNodes(contentNodes);
 * ```
 */

import { ContentNode } from '../models/content-node.model';
import { HolochainClientService } from './holochain-client.service';
import {
  HolochainImportConfig,
  HolochainImportResult,
  HolochainVerifyResult,
  CreateContentInput,
  BulkCreateContentOutput,
  ContentStats,
  HolochainContentOutput,
  QueryByTypeInput,
  QueryByIdInput,
} from '../models/holochain.model';

/**
 * Holochain Import Service
 *
 * Provides methods for importing ContentNode arrays to Holochain,
 * verifying content exists, and querying statistics.
 */
export class HolochainImportService {
  private client: HolochainClientService;
  private config: HolochainImportConfig;

  constructor(config: HolochainImportConfig) {
    this.config = config;
    this.client = new HolochainClientService({
      adminUrl: config.adminUrl,
      appId: config.appId,
      happPath: config.happPath,
    });
  }

  /**
   * Import an array of ContentNodes to Holochain
   *
   * Processes nodes in batches using bulk_create_content zome function.
   *
   * @param nodes - Array of ContentNode to import
   * @returns Import result with counts and any errors
   */
  async importNodes(nodes: ContentNode[]): Promise<HolochainImportResult> {
    const startTime = Date.now();
    const importId = `import-${startTime}`;
    const errors: string[] = [];
    let createdCount = 0;

    try {
      await this.client.connect();

      // Process in batches
      const batchSize = this.config.batchSize;
      const totalBatches = Math.ceil(nodes.length / batchSize);

      for (let i = 0; i < nodes.length; i += batchSize) {
        const batchNum = Math.floor(i / batchSize) + 1;
        const batch = nodes.slice(i, i + batchSize);

        console.log(`Processing batch ${batchNum}/${totalBatches} (${batch.length} nodes)...`);

        const inputs = batch.map((node) => this.nodeToHolochainInput(node));

        try {
          const result = await this.client.callZome<BulkCreateContentOutput>({
            zomeName: 'content_store',
            fnName: 'bulk_create_content',
            payload: {
              import_id: importId,
              contents: inputs,
            },
          });

          createdCount += result.created_count;

          if (result.errors.length > 0) {
            errors.push(...result.errors);
          }

          console.log(`  Batch ${batchNum}: ${result.created_count}/${batch.length} created`);
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          errors.push(`Batch ${batchNum} failed: ${message}`);
          console.error(`  Batch ${batchNum} failed: ${message}`);
        }
      }

      return {
        totalNodes: nodes.length,
        createdNodes: createdCount,
        errors,
        importId,
        durationMs: Date.now() - startTime,
      };
    } finally {
      await this.client.disconnect();
    }
  }

  /**
   * Get content statistics from Holochain
   *
   * @returns Statistics including total count and counts by type
   */
  async getStats(): Promise<ContentStats> {
    try {
      await this.client.connect();
      return await this.client.callZome<ContentStats>({
        zomeName: 'content_store',
        fnName: 'get_content_stats',
        payload: null,
      });
    } finally {
      await this.client.disconnect();
    }
  }

  /**
   * Verify that content exists in Holochain by ID
   *
   * @param ids - Array of content IDs to verify
   * @returns Object with found and missing arrays
   */
  async verifyContent(ids: string[]): Promise<HolochainVerifyResult> {
    const found: string[] = [];
    const missing: string[] = [];

    try {
      await this.client.connect();

      for (const id of ids) {
        try {
          const result = await this.client.callZome<HolochainContentOutput | null>({
            zomeName: 'content_store',
            fnName: 'get_content_by_id',
            payload: { id } as QueryByIdInput,
          });

          if (result) {
            found.push(id);
          } else {
            missing.push(id);
          }
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          console.warn(`Failed to verify content '${id}': ${message}`);
          missing.push(id);
        }
      }

      return { found, missing };
    } finally {
      await this.client.disconnect();
    }
  }

  /**
   * Get content by type from Holochain
   *
   * @param contentType - Content type to filter by
   * @param limit - Maximum number of results (default 100)
   * @returns Array of content outputs
   */
  async getContentByType(
    contentType: string,
    limit = 100
  ): Promise<HolochainContentOutput[]> {
    try {
      await this.client.connect();
      return await this.client.callZome<HolochainContentOutput[]>({
        zomeName: 'content_store',
        fnName: 'get_content_by_type',
        payload: { content_type: contentType, limit } as QueryByTypeInput,
      });
    } finally {
      await this.client.disconnect();
    }
  }

  /**
   * Get content by ID from Holochain
   *
   * @param id - Content ID to fetch
   * @returns Content output or null if not found
   */
  async getContentById(id: string): Promise<HolochainContentOutput | null> {
    try {
      await this.client.connect();
      return await this.client.callZome<HolochainContentOutput | null>({
        zomeName: 'content_store',
        fnName: 'get_content_by_id',
        payload: { id } as QueryByIdInput,
      });
    } finally {
      await this.client.disconnect();
    }
  }

  /**
   * Test connection to Holochain conductor
   *
   * @returns True if connection successful and zome call works
   */
  async testConnection(): Promise<boolean> {
    return await this.client.testConnection();
  }

  /**
   * Convert ContentNode to Holochain CreateContentInput
   *
   * Maps the TypeScript ContentNode interface to the Rust-compatible
   * input structure for the create_content zome function.
   */
  private nodeToHolochainInput(node: ContentNode): CreateContentInput {
    return {
      id: node.id,
      content_type: node.contentType,
      title: node.title,
      description: node.description,
      content:
        typeof node.content === 'string' ? node.content : JSON.stringify(node.content),
      content_format: node.contentFormat,
      tags: node.tags || [],
      source_path: node.sourcePath ?? null,
      related_node_ids: node.relatedNodeIds || [],
      reach: node.reach || 'commons',
      metadata_json: JSON.stringify(node.metadata || {}),
    };
  }
}

/**
 * Create a pre-configured import service for the elohim app
 *
 * @param adminUrl - Admin WebSocket URL (default: wss://doorway-dev.elohim.host)
 * @param batchSize - Number of entries per bulk call (default: 50)
 * @param happPath - Optional path to .happ file for installation
 */
export function createElohimImportService(
  adminUrl = 'wss://doorway-dev.elohim.host',
  batchSize = 50,
  happPath?: string
): HolochainImportService {
  return new HolochainImportService({
    adminUrl,
    appId: 'elohim',
    batchSize,
    happPath,
  });
}
