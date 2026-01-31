import { Injectable } from '@angular/core';

// @coverage: 0.9% (2026-01-31)

import { map, shareReplay, take, switchMap, catchError, tap } from 'rxjs/operators';

import { Observable, of, forkJoin } from 'rxjs';

import { DataLoaderService } from '@app/elohim/services/data-loader.service';

import { ContentNode, ContentGraph, ContentRelationship } from '../models/content-node.model';
import {
  RelatedConceptsResult,
  RelatedConceptsOptions,
  RelationshipEdge,
  RelationshipType,
  MiniGraphData,
  MiniGraphNode,
  MiniGraphEdge,
  NeighborhoodQueryOptions,
  PREREQUISITE_RELATIONSHIP_TYPES,
  EXTENSION_RELATIONSHIP_TYPES,
  RELATED_RELATIONSHIP_TYPES,
  HIERARCHY_RELATIONSHIP_TYPES,
} from '../models/exploration-context.model';

/**
 * Category names for relationship categorization.
 */
type CategoryName = 'prerequisites' | 'extensions' | 'related' | 'children' | 'parents';

/**
 * Rule for categorizing a relationship into a result category.
 * Data-driven approach eliminates boolean selector parameters.
 */
interface CategorizationRule {
  /** Target category name */
  readonly category: CategoryName;
  /** Relationship types that match this rule */
  readonly relationshipTypes: readonly RelationshipType[];
  /** Whether this rule applies to outgoing (true) or incoming (false) relationships */
  readonly isOutgoing: boolean;
  /** Whether to check for duplicates before adding */
  readonly checkDuplicates: boolean;
}

/**
 * Categorization rules define how relationships map to result categories.
 * Using a declarative approach reduces cognitive complexity.
 */
const CATEGORIZATION_RULES: readonly CategorizationRule[] = [
  // Outgoing rules (current node is source)
  {
    category: 'children',
    relationshipTypes: HIERARCHY_RELATIONSHIP_TYPES,
    isOutgoing: true,
    checkDuplicates: false,
  },
  {
    category: 'prerequisites',
    relationshipTypes: PREREQUISITE_RELATIONSHIP_TYPES,
    isOutgoing: true,
    checkDuplicates: false,
  },
  {
    category: 'extensions',
    relationshipTypes: EXTENSION_RELATIONSHIP_TYPES,
    isOutgoing: true,
    checkDuplicates: false,
  },
  {
    category: 'related',
    relationshipTypes: RELATED_RELATIONSHIP_TYPES,
    isOutgoing: true,
    checkDuplicates: true,
  },
  // Incoming rules (current node is target)
  {
    category: 'parents',
    relationshipTypes: HIERARCHY_RELATIONSHIP_TYPES,
    isOutgoing: false,
    checkDuplicates: false,
  },
  {
    category: 'related',
    relationshipTypes: RELATED_RELATIONSHIP_TYPES,
    isOutgoing: false,
    checkDuplicates: true,
  },
] as const;

/**
 * LRU Cache implementation for relationship queries.
 * Limits memory usage while maintaining fast access to frequently-used data.
 */
class LRUCache<K, V> {
  private readonly cache = new Map<K, V>();
  private readonly maxSize: number;

  constructor(maxSize = 100) {
    this.maxSize = maxSize;
  }

  get(key: K): V | undefined {
    const value = this.cache.get(key);
    if (value !== undefined) {
      // Move to end (most recently used)
      this.cache.delete(key);
      this.cache.set(key, value);
    }
    return value;
  }

  set(key: K, value: V): void {
    if (this.cache.has(key)) {
      this.cache.delete(key);
    } else if (this.cache.size >= this.maxSize) {
      // Remove oldest (first) entry
      const firstKey = this.cache.keys().next().value;
      if (firstKey !== undefined) {
        this.cache.delete(firstKey);
      }
    }
    this.cache.set(key, value);
  }

  has(key: K): boolean {
    return this.cache.has(key);
  }

  clear(): void {
    this.cache.clear();
  }

  get size(): number {
    return this.cache.size;
  }
}

/**
 * RelatedConceptsService - Efficient relationship queries with lazy loading and caching.
 *
 * Provides methods to query related concepts organized by relationship type
 * for the Related Concepts Panel and Mini Graph visualization.
 *
 * Optimization strategies:
 * 1. **Lazy Loading**: Queries only the relationships needed for a specific node
 *    instead of loading the entire graph upfront.
 * 2. **LRU Caching**: Caches relationship queries with bounded memory usage.
 * 3. **Relationship Index**: Builds indexes for O(1) lookups when full graph is loaded.
 * 4. **Content Stripping**: Removes content body from nodes to reduce memory.
 *
 * Uses shareReplay for caching the full graph when needed for complex queries,
 * but prefers lazy loading for simple per-node queries.
 */
@Injectable({ providedIn: 'root' })
export class RelatedConceptsService {
  /** Cached full graph observable - loaded only when needed */
  private graph$: Observable<ContentGraph> | null = null;

  /** LRU cache for per-node relationship queries (lazy loading) */
  private readonly relationshipCache = new LRUCache<string, RelatedConceptsResult>(200);

  /** LRU cache for neighborhood graph queries */
  private readonly neighborhoodCache = new LRUCache<string, MiniGraphData>(50);

  /** Relationship index for O(1) lookups (built when full graph is loaded) */
  private relationshipIndex: Map<string, ContentRelationship> | null = null;

  constructor(private readonly dataLoader: DataLoaderService) {}

  /**
   * Get related concepts for a content node, grouped by relationship type.
   *
   * Uses lazy loading with LRU caching for efficiency.
   * Only loads the full graph when complex filtering is needed.
   *
   * @param contentId - The content node ID to find relations for
   * @param options - Query options (limit, filters)
   * @returns Observable of grouped related concepts
   */
  getRelatedConcepts(
    contentId: string,
    options: RelatedConceptsOptions = {}
  ): Observable<RelatedConceptsResult> {
    // Build cache key from contentId and relevant options
    const cacheKey = this.buildCacheKey(contentId, options);

    // Check LRU cache first
    const cached = this.relationshipCache.get(cacheKey);
    if (cached) {
      return of(cached);
    }

    // Use lazy loading for simple queries (no complex filters)
    const useFullGraph =
      (options.includeTypes?.length ?? 0) > 0 || (options.excludeTypes?.length ?? 0) > 0;

    if (useFullGraph) {
      // Complex query - needs full graph
      return this.getGraph().pipe(
        map(graph => {
          const result = this.queryRelatedConcepts(graph, contentId, options);
          this.relationshipCache.set(cacheKey, result);
          return result;
        })
      );
    }

    // Simple query - use lazy loading via DataLoader
    return this.lazyLoadRelatedConcepts(contentId, options).pipe(
      tap(result => this.relationshipCache.set(cacheKey, result))
    );
  }

  /**
   * Build a cache key for relationship queries.
   */
  private buildCacheKey(contentId: string, options: RelatedConceptsOptions): string {
    const parts = [contentId];
    if (options.limit) parts.push(`l${options.limit}`);
    if (options.includeTypes?.length) parts.push(`i${options.includeTypes.join(',')}`);
    if (options.excludeTypes?.length) parts.push(`e${options.excludeTypes.join(',')}`);
    if (options.includeContent) parts.push('c');
    return parts.join(':');
  }

  /**
   * Lazy load related concepts for a single node without loading full graph.
   * Uses DataLoaderService.getRelationshipsForNode() for efficient queries.
   */
  private lazyLoadRelatedConcepts(
    contentId: string,
    options: RelatedConceptsOptions = {}
  ): Observable<RelatedConceptsResult> {
    const { limit = 10, includeContent = false } = options;

    // Query relationships for this node (both directions)
    return this.dataLoader.getRelationshipsForNode(contentId, 'both').pipe(
      switchMap(relationships => {
        // Collect unique node IDs to load
        const nodeIds = new Set<string>();
        for (const rel of relationships) {
          if (rel.sourceNodeId === contentId) {
            nodeIds.add(rel.targetNodeId);
          } else {
            nodeIds.add(rel.sourceNodeId);
          }
        }

        // Batch load related nodes (only metadata, not content)
        const nodeLoads = Array.from(nodeIds)
          .slice(0, limit * 3)
          .map(id =>
            this.dataLoader.getContent(id).pipe(
              catchError(() => of(null)),
              map(node => this.processLoadedNode(node, includeContent))
            )
          );

        if (nodeLoads.length === 0) {
          return of({ relationships, nodes: new Map<string, ContentNode>() });
        }

        return forkJoin(nodeLoads).pipe(
          map(loadedNodes => {
            const nodes = new Map<string, ContentNode>();
            loadedNodes.forEach(node => {
              if (node) nodes.set(node.id, node);
            });
            return { relationships, nodes };
          })
        );
      }),
      map(({ relationships, nodes }) =>
        this.categorizeRelationships(contentId, relationships, nodes, limit)
      )
    );
  }

  /**
   * Categorize relationships into groups (prerequisites, extensions, related, etc.)
   * Uses data-driven categorization rules to reduce cognitive complexity.
   */
  private categorizeRelationships(
    contentId: string,
    relationships: ContentRelationship[],
    nodes: Map<string, ContentNode>,
    limit: number
  ): RelatedConceptsResult {
    const categories = this.initializeCategories();
    const allRelationships: RelationshipEdge[] = [];

    for (const rel of relationships) {
      const relType = rel.relationshipType as RelationshipType;
      const isOutgoing = rel.sourceNodeId === contentId;
      const otherNodeId = isOutgoing ? rel.targetNodeId : rel.sourceNodeId;
      const node = nodes.get(otherNodeId);

      if (!node) continue;

      allRelationships.push(this.createRelationshipEdge(rel, relType));
      this.applyCategorizationRules(node, relType, isOutgoing, categories, limit);
    }

    return { ...categories, allRelationships };
  }

  /**
   * Initialize empty category arrays for relationship categorization.
   */
  private initializeCategories(): Record<CategoryName, ContentNode[]> {
    return {
      prerequisites: [],
      extensions: [],
      related: [],
      children: [],
      parents: [],
    };
  }

  /**
   * Create a RelationshipEdge from a ContentRelationship.
   */
  private createRelationshipEdge(
    rel: ContentRelationship,
    relType: RelationshipType
  ): RelationshipEdge {
    return {
      id: rel.id,
      source: rel.sourceNodeId,
      target: rel.targetNodeId,
      type: relType,
      metadata: rel.metadata,
    };
  }

  /**
   * Apply categorization rules to add a node to the appropriate category.
   * Data-driven approach: iterates rules instead of nested conditionals.
   */
  private applyCategorizationRules(
    node: ContentNode,
    relType: RelationshipType,
    isOutgoing: boolean,
    categories: Record<CategoryName, ContentNode[]>,
    limit: number
  ): void {
    for (const rule of CATEGORIZATION_RULES) {
      if (this.ruleMatches(rule, relType, isOutgoing)) {
        this.addToCategory(node, categories[rule.category], limit, rule.checkDuplicates);
        return; // First matching rule wins
      }
    }
  }

  /**
   * Check if a categorization rule matches the given relationship.
   */
  private ruleMatches(
    rule: CategorizationRule,
    relType: RelationshipType,
    isOutgoing: boolean
  ): boolean {
    return rule.isOutgoing === isOutgoing && rule.relationshipTypes.includes(relType);
  }

  /**
   * Add a node to a category array if under limit and not duplicate.
   */
  private addToCategory(
    node: ContentNode,
    category: ContentNode[],
    limit: number,
    checkDuplicates: boolean
  ): void {
    if (category.length >= limit) return;
    if (checkDuplicates && category.some(n => n.id === node.id)) return;
    category.push(node);
  }

  /**
   * Get neighborhood graph data for mini-graph visualization.
   *
   * Uses LRU caching for repeated queries.
   *
   * @param contentId - The focus node ID
   * @param options - Query options (depth, maxNodes)
   * @returns Observable of mini-graph data
   */
  getNeighborhood(
    contentId: string,
    options: NeighborhoodQueryOptions = {}
  ): Observable<MiniGraphData> {
    const { depth = 1, maxNodes = 15, relationshipTypes } = options;

    // Build cache key
    const cacheKey = `${contentId}:d${depth}:m${maxNodes}:${relationshipTypes?.join(',') ?? 'all'}`;

    // Check LRU cache first
    const cached = this.neighborhoodCache.get(cacheKey);
    if (cached) {
      return of(cached);
    }

    return this.getGraph().pipe(
      map(graph => {
        const result = this.buildNeighborhoodGraph(
          graph,
          contentId,
          depth,
          maxNodes,
          relationshipTypes
        );
        this.neighborhoodCache.set(cacheKey, result);
        return result;
      })
    );
  }

  /**
   * Get concepts by specific relationship type from a source node.
   *
   * @param contentId - Source content node ID
   * @param relationshipType - Type of relationship to query
   * @param direction - 'outgoing' (source→target) or 'incoming' (target→source)
   * @returns Observable of related content nodes
   */
  getByRelationshipType(
    contentId: string,
    relationshipType: RelationshipType,
    direction: 'outgoing' | 'incoming' = 'outgoing'
  ): Observable<ContentNode[]> {
    return this.getGraph().pipe(
      map(graph => {
        const relationships = this.findRelationships(graph, contentId, relationshipType, direction);
        return this.resolveNodes(graph, relationships, direction);
      })
    );
  }

  /**
   * Check if a concept has any related concepts.
   */
  hasRelatedConcepts(contentId: string): Observable<boolean> {
    return this.getGraph().pipe(
      map(graph => {
        const outgoing = graph.adjacency.get(contentId);
        const incoming = graph.reverseAdjacency.get(contentId);
        return (outgoing?.size ?? 0) > 0 || (incoming?.size ?? 0) > 0;
      })
    );
  }

  // =========================================================================
  // Private Methods
  // =========================================================================

  /**
   * Get cached graph observable.
   * Builds relationship index on first load for O(1) lookups.
   */
  private getGraph(): Observable<ContentGraph> {
    this.graph$ ??= this.dataLoader.getGraph().pipe(
      take(1),
      tap(graph => this.buildRelationshipIndex(graph)),
      shareReplay(1)
    );
    return this.graph$;
  }

  /**
   * Build relationship index for O(1) lookups.
   * Key format: "sourceId:targetId" → ContentRelationship
   */
  private buildRelationshipIndex(graph: ContentGraph): void {
    if (this.relationshipIndex) return; // Already built

    this.relationshipIndex = new Map();
    for (const [, rel] of graph.relationships) {
      const key = `${rel.sourceNodeId}:${rel.targetNodeId}`;
      this.relationshipIndex.set(key, rel);
    }
  }

  /**
   * Query and group related concepts by relationship type.
   * Refactored into helper functions to reduce cognitive complexity.
   */
  private queryRelatedConcepts(
    graph: ContentGraph,
    contentId: string,
    options: RelatedConceptsOptions
  ): RelatedConceptsResult {
    const { limit = 10, includeTypes, excludeTypes, includeContent = false } = options;

    const allRelationships: RelationshipEdge[] = [];
    const prerequisites: ContentNode[] = [];
    const extensions: ContentNode[] = [];
    const related: ContentNode[] = [];
    const children: ContentNode[] = [];
    const parents: ContentNode[] = [];

    const filterOptions = { includeTypes, excludeTypes };
    const outgoingIds = graph.adjacency.get(contentId) ?? new Set<string>();
    const incomingIds = graph.reverseAdjacency.get(contentId) ?? new Set<string>();

    // Process outgoing relationships (this node is source)
    this.processQueryOutgoing(
      graph,
      contentId,
      outgoingIds,
      filterOptions,
      includeContent,
      limit,
      allRelationships,
      prerequisites,
      extensions,
      related,
      children
    );

    // Process incoming relationships (this node is target)
    this.processQueryIncoming(
      graph,
      contentId,
      incomingIds,
      filterOptions,
      includeContent,
      limit,
      allRelationships,
      related,
      parents
    );

    // Handle prerequisites specially - DEPENDS_ON outgoing means this depends on target
    this.processQueryPrerequisites(
      graph,
      contentId,
      outgoingIds,
      includeContent,
      limit,
      prerequisites
    );

    return {
      prerequisites,
      extensions,
      related,
      children,
      parents,
      allRelationships,
    };
  }

  /**
   * Process outgoing relationships for queryRelatedConcepts.
   */
  private processQueryOutgoing(
    graph: ContentGraph,
    contentId: string,
    outgoingIds: Set<string>,
    filterOptions: { includeTypes?: RelationshipType[]; excludeTypes?: RelationshipType[] },
    includeContent: boolean,
    limit: number,
    allRelationships: RelationshipEdge[],
    prerequisites: ContentNode[],
    extensions: ContentNode[],
    related: ContentNode[],
    children: ContentNode[]
  ): void {
    const { includeTypes, excludeTypes } = filterOptions;

    for (const targetId of outgoingIds) {
      const rel = this.findRelationshipBetween(graph, contentId, targetId);
      if (!rel) continue;

      const relType = rel.relationshipType as RelationshipType;

      if (!this.passesFilters(relType, includeTypes, excludeTypes)) continue;

      allRelationships.push({
        id: rel.id,
        source: contentId,
        target: targetId,
        type: relType,
        metadata: rel.metadata,
      });

      const node = graph.nodes.get(targetId);
      if (!node) continue;

      const nodeToAdd = includeContent ? node : this.stripContent(node);

      this.categorizeOutgoingNode(
        relType,
        nodeToAdd,
        limit,
        prerequisites,
        extensions,
        related,
        children
      );
    }
  }

  /**
   * Process incoming relationships for queryRelatedConcepts.
   */
  private processQueryIncoming(
    graph: ContentGraph,
    contentId: string,
    incomingIds: Set<string>,
    filterOptions: { includeTypes?: RelationshipType[]; excludeTypes?: RelationshipType[] },
    includeContent: boolean,
    limit: number,
    allRelationships: RelationshipEdge[],
    related: ContentNode[],
    parents: ContentNode[]
  ): void {
    const { includeTypes, excludeTypes } = filterOptions;

    for (const sourceId of incomingIds) {
      const rel = this.findRelationshipBetween(graph, sourceId, contentId);
      if (!rel) continue;

      const relType = rel.relationshipType as RelationshipType;

      if (!this.passesFilters(relType, includeTypes, excludeTypes)) continue;

      allRelationships.push({
        id: rel.id,
        source: sourceId,
        target: contentId,
        type: relType,
        metadata: rel.metadata,
      });

      const node = graph.nodes.get(sourceId);
      if (!node) continue;

      const nodeToAdd = includeContent ? node : this.stripContent(node);

      this.categorizeIncomingNode(relType, nodeToAdd, sourceId, limit, related, parents);
    }
  }

  /**
   * Process outgoing prerequisites for queryRelatedConcepts.
   */
  private processQueryPrerequisites(
    graph: ContentGraph,
    contentId: string,
    outgoingIds: Set<string>,
    includeContent: boolean,
    limit: number,
    prerequisites: ContentNode[]
  ): void {
    for (const targetId of outgoingIds) {
      const rel = this.findRelationshipBetween(graph, contentId, targetId);
      if (!rel) continue;

      const relType = rel.relationshipType as RelationshipType;
      if (!PREREQUISITE_RELATIONSHIP_TYPES.includes(relType)) continue;
      if (prerequisites.length >= limit) continue;

      const node = graph.nodes.get(targetId);
      if (node && !prerequisites.some(n => n.id === targetId)) {
        prerequisites.push(includeContent ? node : this.stripContent(node));
      }
    }
  }

  /**
   * Check if a relationship passes include/exclude filters.
   */
  private passesFilters(
    relType: RelationshipType,
    includeTypes?: RelationshipType[],
    excludeTypes?: RelationshipType[]
  ): boolean {
    if (includeTypes && !includeTypes.includes(relType)) return false;
    if (excludeTypes?.includes(relType)) return false;
    return true;
  }

  /**
   * Categorize an outgoing node in queryRelatedConcepts.
   */
  private categorizeOutgoingNode(
    relType: RelationshipType,
    node: ContentNode,
    limit: number,
    prerequisites: ContentNode[],
    extensions: ContentNode[],
    related: ContentNode[],
    children: ContentNode[]
  ): void {
    if (HIERARCHY_RELATIONSHIP_TYPES.includes(relType) && children.length < limit) {
      children.push(node);
    } else if (EXTENSION_RELATIONSHIP_TYPES.includes(relType) && extensions.length < limit) {
      extensions.push(node);
    } else if (RELATED_RELATIONSHIP_TYPES.includes(relType) && related.length < limit) {
      related.push(node);
    }
  }

  /**
   * Categorize an incoming node in queryRelatedConcepts.
   */
  private categorizeIncomingNode(
    relType: RelationshipType,
    node: ContentNode,
    nodeId: string,
    limit: number,
    related: ContentNode[],
    parents: ContentNode[]
  ): void {
    if (HIERARCHY_RELATIONSHIP_TYPES.includes(relType) && parents.length < limit) {
      parents.push(node);
    } else if (
      RELATED_RELATIONSHIP_TYPES.includes(relType) &&
      related.length < limit &&
      !related.some(n => n.id === nodeId)
    ) {
      related.push(node);
    }
  }

  /**
   * Build neighborhood graph for mini-graph visualization.
   */
  private buildNeighborhoodGraph(
    graph: ContentGraph,
    focusId: string,
    depth: number,
    maxNodes: number,
    relationshipTypes?: RelationshipType[]
  ): MiniGraphData {
    const focusNode = graph.nodes.get(focusId);
    if (!focusNode) {
      return {
        focus: { id: focusId, title: 'Unknown', contentType: 'unknown', isFocus: true, depth: 0 },
        neighbors: [],
        edges: [],
      };
    }

    const visited = new Set<string>([focusId]);
    const neighbors: MiniGraphNode[] = [];
    const edges: MiniGraphEdge[] = [];

    // BFS to collect neighborhood
    let frontier = [focusId];

    for (let d = 1; d <= depth && neighbors.length < maxNodes - 1; d++) {
      const nextFrontier: string[] = [];

      for (const nodeId of frontier) {
        if (neighbors.length >= maxNodes - 1) break;

        // Outgoing edges
        const outgoing = graph.adjacency.get(nodeId) ?? new Set<string>();
        for (const targetId of outgoing) {
          if (visited.has(targetId)) continue;
          if (neighbors.length >= maxNodes - 1) break;

          const rel = this.findRelationshipBetween(graph, nodeId, targetId);
          const relType = (rel?.relationshipType ?? 'RELATES_TO') as RelationshipType;

          // Apply relationship filter
          if (relationshipTypes && !relationshipTypes.includes(relType)) continue;

          visited.add(targetId);
          nextFrontier.push(targetId);

          const targetNode = graph.nodes.get(targetId);
          if (targetNode) {
            neighbors.push({
              id: targetId,
              title: targetNode.title || targetId,
              contentType: targetNode.contentType,
              isFocus: false,
              depth: d,
            });
          }

          edges.push({
            source: nodeId,
            target: targetId,
            relationshipType: relType,
          });
        }

        // Incoming edges
        const incoming = graph.reverseAdjacency.get(nodeId) ?? new Set<string>();
        for (const sourceId of incoming) {
          if (visited.has(sourceId)) continue;
          if (neighbors.length >= maxNodes - 1) break;

          const rel = this.findRelationshipBetween(graph, sourceId, nodeId);
          const relType = (rel?.relationshipType ?? 'RELATES_TO') as RelationshipType;

          // Apply relationship filter
          if (relationshipTypes && !relationshipTypes.includes(relType)) continue;

          visited.add(sourceId);
          nextFrontier.push(sourceId);

          const sourceNode = graph.nodes.get(sourceId);
          if (sourceNode) {
            neighbors.push({
              id: sourceId,
              title: sourceNode.title || sourceId,
              contentType: sourceNode.contentType,
              isFocus: false,
              depth: d,
            });
          }

          edges.push({
            source: sourceId,
            target: nodeId,
            relationshipType: relType,
          });
        }
      }

      frontier = nextFrontier;
    }

    return {
      focus: {
        id: focusId,
        title: focusNode.title || focusId,
        contentType: focusNode.contentType,
        isFocus: true,
        depth: 0,
      },
      neighbors,
      edges,
    };
  }

  /**
   * Find relationships from a node by type and direction.
   */
  private findRelationships(
    graph: ContentGraph,
    contentId: string,
    relationshipType: RelationshipType,
    direction: 'outgoing' | 'incoming'
  ): ContentRelationship[] {
    const results: ContentRelationship[] = [];

    const adjacentIds =
      direction === 'outgoing'
        ? graph.adjacency.get(contentId)
        : graph.reverseAdjacency.get(contentId);

    if (!adjacentIds) return results;

    for (const adjacentId of adjacentIds) {
      const [sourceId, targetId] =
        direction === 'outgoing' ? [contentId, adjacentId] : [adjacentId, contentId];

      const rel = this.findRelationshipBetween(graph, sourceId, targetId);
      if (rel && rel.relationshipType === relationshipType) {
        results.push(rel);
      }
    }

    return results;
  }

  /**
   * Find a specific relationship between two nodes.
   * Uses indexed lookup for O(1) performance when index is available.
   */
  private findRelationshipBetween(
    graph: ContentGraph,
    sourceId: string,
    targetId: string
  ): ContentRelationship | null {
    // Use index for O(1) lookup if available
    if (this.relationshipIndex) {
      const key = `${sourceId}:${targetId}`;
      return this.relationshipIndex.get(key) ?? null;
    }

    // Fallback to linear search (only before index is built)
    for (const [, rel] of graph.relationships) {
      if (rel.sourceNodeId === sourceId && rel.targetNodeId === targetId) {
        return rel;
      }
    }
    return null;
  }

  /**
   * Resolve node IDs from relationships to ContentNode objects.
   */
  private resolveNodes(
    graph: ContentGraph,
    relationships: ContentRelationship[],
    direction: 'outgoing' | 'incoming'
  ): ContentNode[] {
    return relationships
      .map(rel => {
        const nodeId = direction === 'outgoing' ? rel.targetNodeId : rel.sourceNodeId;
        return graph.nodes.get(nodeId);
      })
      .filter((node): node is ContentNode => node !== undefined);
  }

  /**
   * Strip content body from node for lighter responses.
   * Replaces content with empty string to reduce memory while keeping type valid.
   */
  private stripContent(node: ContentNode): ContentNode {
    return {
      ...node,
      content: '',
    };
  }

  /**
   * Process loaded node - strip content if not needed
   */
  private processLoadedNode(node: ContentNode | null, includeContent: boolean): ContentNode | null {
    if (!node) {
      return null;
    }
    return includeContent ? node : this.stripContent(node);
  }

  /**
   * Clear all caches (useful for refreshing data).
   */
  clearCache(): void {
    this.graph$ = null;
    this.relationshipIndex = null;
    this.relationshipCache.clear();
    this.neighborhoodCache.clear();
  }

  /**
   * Get cache statistics for debugging/monitoring.
   */
  getCacheStats(): {
    relationshipCacheSize: number;
    neighborhoodCacheSize: number;
    hasGraph: boolean;
  } {
    return {
      relationshipCacheSize: this.relationshipCache.size,
      neighborhoodCacheSize: this.neighborhoodCache.size,
      hasGraph: this.graph$ !== null,
    };
  }
}
