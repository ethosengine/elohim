/**
 * Content Service
 *
 * High-level service for content operations.
 * Similar to Spring @Service - provides business logic layer.
 */

import { ZomeClient } from '../client/zome-client.js';
import { BatchExecutor, type BatchResult } from '../client/batch-executor.js';
import {
  type ContentOutput,
  type CreateContentInput,
  type ContentStats,
  type QueryCriteria,
} from '../types.js';

/**
 * Service for content management operations
 *
 * Wraps ZomeClient with:
 * - Automatic batching for bulk operations
 * - Higher-level query methods
 * - Error handling and validation
 */
export class ContentService {
  private client: ZomeClient;
  private batchExecutor: BatchExecutor;

  constructor(client: ZomeClient) {
    this.client = client;
    this.batchExecutor = new BatchExecutor(client);
  }

  /**
   * Create a single content entry
   */
  async create(input: CreateContentInput): Promise<ContentOutput> {
    return this.client.createContent(input);
  }

  /**
   * Bulk create content entries with automatic batching
   *
   * Handles WASM memory limits by splitting into batches of 50.
   */
  async bulkCreate(
    contents: CreateContentInput[],
    importId?: string
  ): Promise<BatchResult<string>> {
    return this.batchExecutor.bulkCreateContent(contents, importId);
  }

  /**
   * Get content by ID
   */
  async getById(id: string): Promise<ContentOutput | null> {
    return this.client.getContentById(id);
  }

  /**
   * Check if content exists
   */
  async exists(id: string): Promise<boolean> {
    const content = await this.client.getContentById(id);
    return content !== null;
  }

  /**
   * Get content by type
   */
  async getByType(contentType: string, limit?: number): Promise<ContentOutput[]> {
    return this.client.getContentByType(contentType, limit);
  }

  /**
   * Get content by tag
   */
  async getByTag(tag: string): Promise<ContentOutput[]> {
    return this.client.getContentByTag(tag);
  }

  /**
   * Get all content created by current agent
   */
  async getMine(): Promise<ContentOutput[]> {
    return this.client.getMyContent();
  }

  /**
   * Query content by criteria
   *
   * Combines type and tag queries for flexible searching.
   */
  async query(criteria: QueryCriteria): Promise<ContentOutput[]> {
    let results: ContentOutput[] = [];

    // Query by type if specified
    if (criteria.content_type) {
      results = await this.client.getContentByType(
        criteria.content_type,
        criteria.limit
      );
    }

    // Filter by tags if specified
    if (criteria.tags && criteria.tags.length > 0) {
      if (results.length === 0 && !criteria.content_type) {
        // No type query, start with tag query
        results = await this.client.getContentByTag(criteria.tags[0]);
      }

      // Filter results by all specified tags
      results = results.filter((content) =>
        criteria.tags!.every((tag) => content.content.tags.includes(tag))
      );
    }

    // Apply limit
    if (criteria.limit && results.length > criteria.limit) {
      results = results.slice(0, criteria.limit);
    }

    return results;
  }

  /**
   * Get content statistics
   */
  async getStats(): Promise<ContentStats> {
    return this.client.getContentStats();
  }

  /**
   * Get all content types in the system
   */
  async getContentTypes(): Promise<string[]> {
    const stats = await this.client.getContentStats();
    return Object.keys(stats.by_type);
  }

  /**
   * Count content by type
   */
  async countByType(contentType: string): Promise<number> {
    const stats = await this.client.getContentStats();
    return stats.by_type[contentType] ?? 0;
  }
}
