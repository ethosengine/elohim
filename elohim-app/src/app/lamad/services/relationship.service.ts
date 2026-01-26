/**
 * RelationshipService - Domain service for content graph relationships.
 *
 * Provides high-level operations for working with content relationships:
 * - Fetching relationships for content nodes
 * - Creating relationships with optional inverses
 * - Bidirectional relationship queries
 *
 * Uses StorageApiService for HTTP communication with elohim-storage.
 */

import { Injectable } from '@angular/core';

import { map } from 'rxjs/operators';

import { Observable, forkJoin } from 'rxjs';

import { RelationshipView } from '@app/elohim/adapters/storage-types.adapter';
import {
  StorageApiService,
  CreatePresenceInput,
  CreateMasteryInput,
  CreateEventInput,
} from '@app/elohim/services/storage-api.service';
import { RelationshipQuery, CreateRelationshipInput } from '@app/lamad/models/content-node.model';

@Injectable({
  providedIn: 'root',
})
export class RelationshipService {
  constructor(private readonly storageApi: StorageApiService) {}

  /**
   * Get all relationships for a content node (as source).
   */
  getRelationshipsForContent(contentId: string): Observable<RelationshipView[]> {
    return this.storageApi.getRelationships({ sourceId: contentId });
  }

  /**
   * Get bidirectional relationships (both outgoing and incoming).
   */
  getBidirectionalRelationships(contentId: string): Observable<RelationshipView[]> {
    return forkJoin([
      this.storageApi.getRelationships({ sourceId: contentId }),
      this.storageApi.getRelationships({ targetId: contentId }),
    ]).pipe(
      map(([outgoing, incoming]) => {
        // Deduplicate by ID (in case a relationship appears in both)
        const seen = new Set<string>();
        const result: RelationshipView[] = [];

        for (const rel of [...outgoing, ...incoming]) {
          if (!seen.has(rel.id)) {
            seen.add(rel.id);
            result.push(rel);
          }
        }

        return result;
      })
    );
  }

  /**
   * Get relationships of a specific type.
   */
  getRelationshipsByType(
    contentId: string,
    relationshipType: string
  ): Observable<RelationshipView[]> {
    return this.storageApi.getRelationships({
      sourceId: contentId,
      relationshipType,
    });
  }

  /**
   * Get relationships with minimum confidence threshold.
   */
  getHighConfidenceRelationships(
    contentId: string,
    minConfidence = 0.8
  ): Observable<RelationshipView[]> {
    return this.storageApi.getRelationships({
      sourceId: contentId,
      minConfidence,
    });
  }

  /**
   * Create a new relationship.
   */
  createRelationship(input: CreateRelationshipInput): Observable<RelationshipView> {
    return this.storageApi.createRelationship(input);
  }

  /**
   * Create a bidirectional relationship pair (e.g., CONTAINS/BELONGS_TO).
   *
   * @param sourceId Source content ID
   * @param targetId Target content ID
   * @param forwardType Forward relationship type (e.g., CONTAINS)
   * @param inverseType Inverse relationship type (e.g., BELONGS_TO)
   * @param confidence Confidence score (default 1.0)
   */
  createBidirectionalRelationship(
    sourceId: string,
    targetId: string,
    forwardType: string,
    inverseType: string,
    confidence = 1.0
  ): Observable<RelationshipView> {
    return this.storageApi.createRelationship({
      sourceId,
      targetId,
      relationshipType: forwardType,
      confidence,
      inferenceSource: 'author',
      createInverse: true,
      inverseType,
    });
  }

  /**
   * Get the relationship graph starting from a content node.
   *
   * @param rootId Root content ID to start from
   * @param depth Maximum depth to traverse (default 1)
   */
  getRelationshipGraph(rootId: string, depth = 1): Observable<Map<string, RelationshipView[]>> {
    const graph = new Map<string, RelationshipView[]>();

    // For depth 1, just get direct relationships
    return this.getBidirectionalRelationships(rootId).pipe(
      map(relationships => {
        graph.set(rootId, relationships);
        return graph;
      })
    );
  }
}
