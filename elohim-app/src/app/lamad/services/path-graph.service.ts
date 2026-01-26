import { Injectable } from '@angular/core';

import { map, switchMap, shareReplay, catchError } from 'rxjs/operators';

import { Observable, forkJoin, of } from 'rxjs';

import { DataLoaderService } from '@app/elohim/services/data-loader.service';

import { ContentNode, ContentMetadata } from '../models/content-node.model';
import {
  LearningPath,
  PathStep,
  PathContentMetadata,
  PathReference,
} from '../models/learning-path.model';

/**
 * PathGraphService - Manages learning paths as ContentNodes on the graph.
 *
 * This service bridges the gap between LearningPath (curated journeys) and
 * ContentNode (graph nodes). When a path is registered on the graph, it becomes
 * discoverable via graph traversal.
 *
 * Key capabilities:
 * - Register paths as ContentNodes with type 'path'
 * - Find paths containing specific content nodes
 * - Extract content node IDs from paths (flattening steps/chapters)
 * - Sync path data to graph representation
 *
 * From the design:
 * "Paths are views/projections OVER the prerequisite graph, not separate from it."
 *
 * Usage:
 * ```typescript
 * // Register a path as a graph node
 * pathGraphService.registerPathAsNode(learningPath).subscribe(pathNode => {
 *   console.log('Path is now discoverable:', pathNode.id);
 * });
 *
 * // Find paths containing a topic
 * pathGraphService.findPathsContainingNode('algebra-basics').subscribe(paths => {
 *   console.log('Paths covering algebra:', paths.length);
 * });
 * ```
 */
@Injectable({ providedIn: 'root' })
export class PathGraphService {
  // Cache for path nodes (path-type ContentNodes)
  private pathNodesCache$: Observable<Map<string, ContentNode>> | null = null;

  constructor(private readonly dataLoader: DataLoaderService) {}

  // =========================================================================
  // Core Registration Methods
  // =========================================================================

  /**
   * Register a LearningPath as a ContentNode on the graph.
   *
   * Creates a ContentNode with contentType: 'path' that:
   * - Has CONTAINS relationships to all step content
   * - Stores PathContentMetadata for discovery queries
   * - Participates in the attestation/governance system
   *
   * @param path - The LearningPath to register
   * @returns Observable<ContentNode> - The created path node
   */
  registerPathAsNode(path: LearningPath): Observable<ContentNode> {
    const contentNodeIds = this.extractContentNodeIds(path);
    const nestedPathIds = this.extractNestedPathIds(path);

    const pathMetadata: PathContentMetadata = {
      pathId: path.id,
      difficulty: path.difficulty,
      estimatedDuration: path.estimatedDuration,
      stepCount: this.countSteps(path),
      chapterCount: path.chapters?.length,
      contentNodeIds,
      nestedPathIds: nestedPathIds.length > 0 ? nestedPathIds : undefined,
      creatorInfo: path.createdBy ? { presenceId: path.createdBy } : undefined,
      forkedFromPathId: path.forkedFrom,
      canonicalStatus: 'draft', // New paths start as draft
      pathType: path.pathType,
      attestationsGranted: path.attestationsGranted,
    };

    const pathNode: ContentNode = {
      id: `path-${path.id}`,
      contentType: 'path',
      title: path.title,
      description: path.description,
      content: path.purpose,
      contentFormat: 'markdown',
      tags: [...path.tags, 'path', path.difficulty],
      relatedNodeIds: contentNodeIds,
      metadata: pathMetadata as unknown as ContentMetadata,
      authorId: path.createdBy,
      reach: this.mapVisibilityToReach(path.visibility),
      createdAt: path.createdAt,
      updatedAt: path.updatedAt,
    };

    // In prototype mode, we just return the node
    // In production, this would write to Holochain
    return of(pathNode);
  }

  /**
   * Find all paths that contain a specific content node.
   *
   * Uses reverse lookup: given a content ID, find which path-type nodes
   * have it in their relatedNodeIds (CONTAINS relationship).
   *
   * @param nodeId - The content node ID to search for
   * @returns Observable<PathReference[]> - Paths containing this content
   */
  findPathsContainingNode(nodeId: string): Observable<PathReference[]> {
    return this.loadPathNodes().pipe(
      map(pathNodes => {
        const results: PathReference[] = [];

        for (const [pathNodeId, pathNode] of pathNodes) {
          const metadata = pathNode.metadata as unknown as PathContentMetadata;

          // Check if this path contains the node
          if (metadata.contentNodeIds?.includes(nodeId)) {
            results.push({
              nodeId: pathNodeId,
              pathId: metadata.pathId,
              title: pathNode.title,
              relationship: 'contains',
            });
          }

          // Check if this path references the node as a nested path
          if (metadata.nestedPathIds?.includes(nodeId)) {
            results.push({
              nodeId: pathNodeId,
              pathId: metadata.pathId,
              title: pathNode.title,
              relationship: 'references',
            });
          }
        }

        return results;
      })
    );
  }

  /**
   * Get the list of content nodes referenced by a path.
   *
   * Flattens steps and chapters to extract all resourceIds.
   * Does NOT expand nested paths (stepType: 'path').
   *
   * @param pathId - The learning path ID
   * @returns Observable<string[]> - Content node IDs
   */
  getPathContentNodes(pathId: string): Observable<string[]> {
    return this.dataLoader.getPath(pathId).pipe(map(path => this.extractContentNodeIds(path)));
  }

  /**
   * Get paths that share content with a given path.
   *
   * Useful for:
   * - "Related paths" suggestions
   * - Cross-path completion tracking (Khan Academy style)
   * - Prerequisite chain analysis
   *
   * @param pathId - The path to find relatives for
   * @returns Observable<PathReference[]> - Paths sharing content
   */
  getRelatedPaths(pathId: string): Observable<PathReference[]> {
    return forkJoin({
      contentNodes: this.getPathContentNodes(pathId),
      allPathNodes: this.loadPathNodes(),
    }).pipe(
      map(({ contentNodes, allPathNodes }) => {
        const contentSet = new Set(contentNodes);
        const results: PathReference[] = [];

        for (const [pathNodeId, pathNode] of allPathNodes) {
          const metadata = pathNode.metadata as unknown as PathContentMetadata;

          // Skip self
          if (metadata.pathId === pathId) continue;

          // Count shared content
          const sharedContent = metadata.contentNodeIds?.filter(id => contentSet.has(id)) || [];

          if (sharedContent.length > 0) {
            results.push({
              nodeId: pathNodeId,
              pathId: metadata.pathId,
              title: pathNode.title,
              relationship: 'contains', // They share content
            });
          }
        }

        // Sort by number of shared content (most related first)
        return results;
      })
    );
  }

  /**
   * Sync a LearningPath to its ContentNode representation.
   *
   * Call this when a path is updated to keep the graph in sync.
   *
   * @param path - The updated LearningPath
   * @returns Observable<ContentNode> - The updated path node
   */
  syncPathNode(path: LearningPath): Observable<ContentNode> {
    // For now, just re-register (prototype behavior)
    // In production, this would update the existing entry
    return this.registerPathAsNode(path);
  }

  // =========================================================================
  // Path Index Loading
  // =========================================================================

  /**
   * Load all path-type nodes from the graph.
   *
   * This uses the graph's nodesByType index for efficient filtering.
   */
  loadPathNodes(): Observable<Map<string, ContentNode>> {
    this.pathNodesCache$ ??= this.buildPathNodesFromIndex().pipe(shareReplay(1));
    return this.pathNodesCache$;
  }

  /**
   * Build path nodes from the path index.
   *
   * In prototype mode, we generate ContentNodes from the path index.
   * In production, we'd query the graph directly.
   */
  private buildPathNodesFromIndex(): Observable<Map<string, ContentNode>> {
    return this.dataLoader.getPathIndex().pipe(
      switchMap(pathIndex => {
        if (!pathIndex.paths || pathIndex.paths.length === 0) {
          return of(new Map<string, ContentNode>());
        }

        // Load each path and convert to ContentNode
        const pathLoads = pathIndex.paths.map(entry =>
          this.dataLoader.getPath(entry.id).pipe(
            switchMap(path => this.registerPathAsNode(path)),
            catchError(() => of(null)) // Skip paths that fail to load
          )
        );

        return forkJoin(pathLoads).pipe(
          map(pathNodes => {
            const nodeMap = new Map<string, ContentNode>();
            for (const node of pathNodes) {
              if (node) {
                nodeMap.set(node.id, node);
              }
            }
            return nodeMap;
          })
        );
      }),
      catchError(() => of(new Map<string, ContentNode>()))
    );
  }

  /**
   * Clear the path nodes cache.
   * Call this when paths are created/updated/deleted.
   */
  clearCache(): void {
    this.pathNodesCache$ = null;
  }

  // =========================================================================
  // Helper Methods
  // =========================================================================

  /**
   * Extract all content node IDs from a path's steps.
   *
   * Handles both flat paths (steps[]) and chapter paths (chapters[].steps[]).
   * Only includes stepType: 'content' steps (skips path, external, checkpoint).
   */
  private extractContentNodeIds(path: LearningPath): string[] {
    const ids = new Set<string>();

    // Process flat steps
    for (const step of path.steps) {
      if (this.isContentStep(step)) {
        ids.add(step.resourceId);
      }
    }

    // Process chapter steps (if any)
    if (path.chapters) {
      for (const chapter of path.chapters) {
        for (const step of chapter.steps) {
          if (this.isContentStep(step)) {
            ids.add(step.resourceId);
          }
        }
      }
    }

    return Array.from(ids);
  }

  /**
   * Extract nested path IDs (stepType: 'path').
   */
  private extractNestedPathIds(path: LearningPath): string[] {
    const ids = new Set<string>();

    const extractFromSteps = (steps: PathStep[]) => {
      for (const step of steps) {
        if (step.stepType === 'path' && step.pathId) {
          ids.add(step.pathId);
        }
      }
    };

    extractFromSteps(path.steps);

    if (path.chapters) {
      for (const chapter of path.chapters) {
        extractFromSteps(chapter.steps);
      }
    }

    return Array.from(ids);
  }

  /**
   * Check if a step is a content step (not path, external, or checkpoint).
   */
  private isContentStep(step: PathStep): boolean {
    // Default stepType is 'content' if not specified
    return !step.stepType || step.stepType === 'content';
  }

  /**
   * Count total steps in a path (including chapter steps).
   */
  private countSteps(path: LearningPath): number {
    if (path.chapters && path.chapters.length > 0) {
      return path.chapters.reduce((sum, ch) => sum + ch.steps.length, 0);
    }
    return path.steps.length;
  }

  /**
   * Map path visibility to content reach.
   */
  private mapVisibilityToReach(
    visibility: string
  ): 'private' | 'invited' | 'local' | 'community' | 'federated' | 'commons' {
    switch (visibility) {
      case 'public':
        return 'commons';
      case 'connections':
      case 'organization':
        return 'community';
      case 'trusted':
        return 'local';
      case 'intimate':
      case 'private':
        return 'private';
      default:
        return 'commons';
    }
  }

  // =========================================================================
  // Graph Integration (Future)
  // =========================================================================

  /**
   * Create CONTAINS relationships from path node to all its content.
   *
   * In production, this writes relationship entries to the DHT.
   * Currently a no-op in prototype mode (relationships are implicit in relatedNodeIds).
   *
   * @param pathNodeId - The path ContentNode ID
   * @param contentNodeIds - Content nodes to link
   */
  linkPathToContent(pathNodeId: string, contentNodeIds: string[]): Observable<void> {
    // In prototype mode, relationships are stored in relatedNodeIds
    // In production, this would create ContentRelationship entries
    return of(undefined);
  }

  /**
   * Get path nodes containing a specific tag.
   *
   * @param tag - Tag to filter by
   * @returns Observable<ContentNode[]> - Path nodes with this tag
   */
  getPathNodesByTag(tag: string): Observable<ContentNode[]> {
    return this.loadPathNodes().pipe(
      map(pathNodes => {
        const results: ContentNode[] = [];
        for (const [, node] of pathNodes) {
          if (node.tags.includes(tag)) {
            results.push(node);
          }
        }
        return results;
      })
    );
  }

  /**
   * Get path nodes by difficulty level.
   *
   * @param difficulty - 'beginner' | 'intermediate' | 'advanced'
   * @returns Observable<ContentNode[]> - Path nodes at this difficulty
   */
  getPathNodesByDifficulty(
    difficulty: 'beginner' | 'intermediate' | 'advanced'
  ): Observable<ContentNode[]> {
    return this.loadPathNodes().pipe(
      map(pathNodes => {
        const results: ContentNode[] = [];
        for (const [, node] of pathNodes) {
          const metadata = node.metadata as unknown as PathContentMetadata;
          if (metadata.difficulty === difficulty) {
            results.push(node);
          }
        }
        return results;
      })
    );
  }
}
