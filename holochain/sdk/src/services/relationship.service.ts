/**
 * Relationship Service
 *
 * High-level service for content relationship operations.
 * Replaces Kuzu graph database for relationship storage.
 */

import { ZomeClient } from '../client/zome-client.js';
import { BatchExecutor, type BatchResult } from '../client/batch-executor.js';
import {
  type RelationshipOutput,
  type CreateRelationshipInput,
  type ContentOutput,
  type ContentGraph,
  RelationshipTypes,
  InferenceSources,
} from '../types.js';

/**
 * Service for relationship management between content nodes
 *
 * Provides:
 * - Relationship CRUD operations
 * - Graph traversal
 * - Bulk relationship creation with batching
 */
export class RelationshipService {
  private client: ZomeClient;
  private batchExecutor: BatchExecutor;

  constructor(client: ZomeClient) {
    this.client = client;
    this.batchExecutor = new BatchExecutor(client);
  }

  /**
   * Create a relationship between two content nodes
   */
  async create(input: CreateRelationshipInput): Promise<RelationshipOutput> {
    return this.client.createRelationship(input);
  }

  /**
   * Create a relationship with defaults (explicit, high confidence)
   */
  async createExplicit(
    sourceId: string,
    targetId: string,
    type: string,
    confidence: number = 1.0
  ): Promise<RelationshipOutput> {
    return this.client.createRelationship({
      source_id: sourceId,
      target_id: targetId,
      relationship_type: type,
      confidence,
      inference_source: InferenceSources.EXPLICIT,
    });
  }

  /**
   * Create a RELATES_TO relationship
   */
  async relatesTo(
    sourceId: string,
    targetId: string,
    confidence: number = 0.8
  ): Promise<RelationshipOutput> {
    return this.createExplicit(
      sourceId,
      targetId,
      RelationshipTypes.RELATES_TO,
      confidence
    );
  }

  /**
   * Create a CONTAINS relationship
   */
  async contains(
    parentId: string,
    childId: string,
    confidence: number = 1.0
  ): Promise<RelationshipOutput> {
    return this.createExplicit(
      parentId,
      childId,
      RelationshipTypes.CONTAINS,
      confidence
    );
  }

  /**
   * Create a DEPENDS_ON relationship
   */
  async dependsOn(
    sourceId: string,
    dependencyId: string,
    confidence: number = 1.0
  ): Promise<RelationshipOutput> {
    return this.createExplicit(
      sourceId,
      dependencyId,
      RelationshipTypes.DEPENDS_ON,
      confidence
    );
  }

  /**
   * Bulk create relationships with automatic batching
   */
  async bulkCreate(
    relationships: CreateRelationshipInput[]
  ): Promise<BatchResult<RelationshipOutput>> {
    return this.batchExecutor.bulkCreateRelationships(relationships);
  }

  /**
   * Get all relationships for a content node
   */
  async getAll(contentId: string): Promise<RelationshipOutput[]> {
    return this.client.getRelationships({
      content_id: contentId,
      direction: 'both',
    });
  }

  /**
   * Get outgoing relationships (content is source)
   */
  async getOutgoing(contentId: string): Promise<RelationshipOutput[]> {
    return this.client.getRelationships({
      content_id: contentId,
      direction: 'outgoing',
    });
  }

  /**
   * Get incoming relationships (content is target)
   */
  async getIncoming(contentId: string): Promise<RelationshipOutput[]> {
    return this.client.getRelationships({
      content_id: contentId,
      direction: 'incoming',
    });
  }

  /**
   * Get related content (follows outgoing relationships)
   */
  async getRelatedContent(
    contentId: string,
    types?: string[]
  ): Promise<ContentOutput[]> {
    return this.client.queryRelatedContent({
      content_id: contentId,
      relationship_types: types,
    });
  }

  /**
   * Get content graph starting from a root node
   */
  async getGraph(contentId: string, depth: number = 1): Promise<ContentGraph> {
    return this.client.getContentGraph({
      content_id: contentId,
      depth,
    });
  }

  /**
   * Check if a relationship exists
   */
  async exists(
    sourceId: string,
    targetId: string,
    type?: string
  ): Promise<boolean> {
    const relationships = await this.getOutgoing(sourceId);
    return relationships.some(
      (r) =>
        r.relationship.target_id === targetId &&
        (!type || r.relationship.relationship_type === type)
    );
  }

  /**
   * Get all content that this content contains (CONTAINS relationships)
   */
  async getChildren(contentId: string): Promise<ContentOutput[]> {
    return this.getRelatedContent(contentId, [RelationshipTypes.CONTAINS]);
  }

  /**
   * Get all content that contains this content (reverse CONTAINS lookup)
   */
  async getParents(contentId: string): Promise<ContentOutput[]> {
    const incoming = await this.getIncoming(contentId);
    const parentIds = incoming
      .filter((r) => r.relationship.relationship_type === RelationshipTypes.CONTAINS)
      .map((r) => r.relationship.source_id);

    // Fetch parent content
    const parents: ContentOutput[] = [];
    for (const id of parentIds) {
      const content = await this.client.getContentById(id);
      if (content) {
        parents.push(content);
      }
    }
    return parents;
  }
}
