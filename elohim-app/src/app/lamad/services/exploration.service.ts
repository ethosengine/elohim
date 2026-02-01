import { Injectable } from '@angular/core';

// @coverage: 92.3% (2026-01-31)

import { map, switchMap, catchError, take } from 'rxjs/operators';

import { Observable, of, throwError, BehaviorSubject } from 'rxjs';

import { DataLoaderService } from '@app/elohim/services/data-loader.service';

import { ContentNode, ContentGraph, ContentRelationship } from '../models/content-node.model';
import {
  GraphExplorationQuery,
  PathfindingQuery,
  GraphView,
  GraphViewSerialized,
  GraphEdge,
  PathResult,
  QueryCost,
  RateLimitStatus,
  RateLimitTier,
  RATE_LIMIT_CONFIGS,
  DEPTH_ATTESTATION_REQUIREMENTS,
  AttestationCheck,
  ExplorationError,
  ExplorationErrorCode,
  ExplorationEvent,
} from '../models/exploration.model';

/**
 * ExplorationService - Graph traversal and discovery with attestation-based access control.
 *
 * This service implements the fog-of-war principle: exploration is intentional, not casual.
 * All queries have visible computational cost and are rate-limited based on attestations.
 *
 * From API Spec Section 3.4:
 * "The exploration service handles graph queries, pathfinding, and research operations."
 *
 * Key features:
 * - Attestation-gated depth limits
 * - Rate limiting per agent
 * - Cost estimation before execution
 * - BFS traversal with relationship filtering
 * - Pathfinding (Dijkstra and semantic)
 *
 * Usage:
 * ```typescript
 * // Simple exploration
 * explorationService.exploreNeighborhood({
 *   focus: 'manifesto',
 *   depth: 1,
 *   view: 'graph'
 * }).subscribe(result => {
 *   console.log('Focus:', result.focus.title);
 *   console.log('Neighbors at depth 1:', result.neighbors.get(1)?.length);
 * });
 *
 * // Check cost before expensive query
 * const cost = await explorationService.estimateCost('exploreNeighborhood', { depth: 2 });
 * if (cost.canExecute) {
 *   explorationService.exploreNeighborhood({ focus: 'manifesto', depth: 2 });
 * }
 * ```
 */
@Injectable({ providedIn: 'root' })
export class ExplorationService {
  // Rate limit tracking per agent
  private readonly rateLimits = new Map<string, AgentRateLimitState>();

  // Event log for analytics/audit
  private eventLog: ExplorationEvent[] = [];
  private readonly MAX_EVENT_LOG_SIZE = 1000;

  // Current agent ID (in production: from auth service)
  private currentAgentId = 'demo-learner';

  // Observable for rate limit status updates
  private readonly rateLimitStatusSubject = new BehaviorSubject<RateLimitStatus | null>(null);
  public readonly rateLimitStatus$ = this.rateLimitStatusSubject.asObservable();

  constructor(private readonly dataLoader: DataLoaderService) {}

  // =========================================================================
  // Core Exploration Methods
  // =========================================================================

  /**
   * Explore the neighborhood around a resource.
   * Returns the focus resource plus neighbors within specified depth.
   *
   * @param query - Exploration parameters
   * @returns Observable<GraphView> - Subgraph centered on focus
   * @throws ExplorationError if unauthorized or rate limited
   */
  exploreNeighborhood(query: GraphExplorationQuery): Observable<GraphView> {
    const startTime = Date.now();

    // Check attestation permissions
    return this.checkAttestations(this.currentAgentId, query.depth).pipe(
      switchMap(attestationCheck => {
        if (!attestationCheck.allowed) {
          return this.createError('DEPTH_UNAUTHORIZED', attestationCheck);
        }

        // Update rate limit state with correct tier from attestations
        this.updateAgentTier(this.currentAgentId, attestationCheck.tier);

        // Check rate limit
        const rateLimitCheck = this.checkRateLimit(this.currentAgentId, 'exploration');
        if (!rateLimitCheck.allowed) {
          return this.createError('RATE_LIMIT_EXCEEDED', {
            rateLimitStatus: this.getRateLimitStatusSync(this.currentAgentId),
          });
        }

        // Get the graph
        return this.dataLoader.getGraph().pipe(
          take(1),
          switchMap(graph => {
            if (!graph || graph.nodes.size === 0) {
              return throwError(() => this.buildError('INVALID_QUERY', 'Graph not loaded'));
            }

            // Find focus node
            const focusNode = graph.nodes.get(query.focus);
            if (!focusNode) {
              return this.createError('RESOURCE_NOT_FOUND', { resourceId: query.focus });
            }

            // Perform BFS traversal
            const result = this.bfsTraversal(graph, query, startTime);

            // Consume rate limit
            this.consumeRateLimit(this.currentAgentId, 'exploration');

            // Log event
            this.logEvent({
              type: 'query-completed',
              timestamp: new Date().toISOString(),
              agentId: this.currentAgentId,
              query,
              result: {
                nodesReturned: result.metadata.nodesReturned,
                computeTimeMs: result.metadata.computeTimeMs,
                creditsConsumed: result.metadata.resourceCredits,
              },
            });

            return of(result);
          })
        );
      }),
      catchError(err => {
        this.logEvent({
          type: 'query-failed',
          timestamp: new Date().toISOString(),
          agentId: this.currentAgentId,
          query,
          error: err,
        });
        return throwError(() => err);
      })
    );
  }

  /**
   * Find a path through the graph between two resources.
   *
   * @param query - Pathfinding parameters
   * @returns Observable<PathResult> - Sequence of resources forming a path
   * @throws ExplorationError if unauthorized or no path exists
   */
  findPath(query: PathfindingQuery): Observable<PathResult> {
    const startTime = Date.now();

    // Pathfinding requires path-creator attestation
    return this.checkAttestations(this.currentAgentId, 3).pipe(
      switchMap(attestationCheck => {
        if (
          attestationCheck.tier !== 'path-creator' &&
          attestationCheck.tier !== 'advanced-researcher'
        ) {
          return this.createError('PATHFINDING_UNAUTHORIZED', {
            requiredAttestation: 'path-creator',
          });
        }

        // Update rate limit state with correct tier from attestations
        this.updateAgentTier(this.currentAgentId, attestationCheck.tier);

        // Check pathfinding rate limit
        const rateLimitCheck = this.checkRateLimit(this.currentAgentId, 'pathfinding');
        if (!rateLimitCheck.allowed) {
          return this.createError('RATE_LIMIT_EXCEEDED', {
            rateLimitStatus: this.getRateLimitStatusSync(this.currentAgentId),
          });
        }

        return this.dataLoader.getGraph().pipe(
          take(1),
          switchMap(graph => {
            if (!graph || graph.nodes.size === 0) {
              return throwError(() => this.buildError('INVALID_QUERY', 'Graph not loaded'));
            }

            // Verify both nodes exist
            if (!graph.nodes.has(query.from)) {
              return this.createError('RESOURCE_NOT_FOUND', { resourceId: query.from });
            }
            if (!graph.nodes.has(query.to)) {
              return this.createError('RESOURCE_NOT_FOUND', { resourceId: query.to });
            }

            // Execute pathfinding
            const pathResult =
              query.algorithm === 'shortest'
                ? this.dijkstraPath(graph, query, startTime)
                : this.semanticPath(graph, query, startTime);

            if (!pathResult) {
              return this.createError('NO_PATH_EXISTS', {
                from: query.from,
                to: query.to,
              });
            }

            // Consume rate limit
            this.consumeRateLimit(this.currentAgentId, 'pathfinding');

            return of(pathResult);
          })
        );
      })
    );
  }

  /**
   * Estimate computational cost before executing a query.
   *
   * @param operation - Operation name ('exploreNeighborhood', 'findPath')
   * @param params - Parameters for the operation
   * @returns Observable<QueryCost> - Estimated cost
   */
  estimateCost(
    operation: string,
    params: Partial<GraphExplorationQuery | PathfindingQuery>
  ): Observable<QueryCost> {
    const agentStatus = this.getRateLimitStatusSync(this.currentAgentId);

    return this.dataLoader.getGraph().pipe(
      take(1),
      map(graph => {
        if (!graph || graph.nodes.size === 0) {
          return {
            estimatedNodes: 0,
            estimatedTimeMs: 0,
            resourceCredits: 0,
            rateLimitImpact: 'Graph not loaded',
            canExecute: false,
            blockedReason: 'query-too-expensive' as const,
          };
        }

        if (operation === 'exploreNeighborhood') {
          const query = params as Partial<GraphExplorationQuery>;
          const depth = query.depth ?? 1;

          // Estimate based on average node degree
          const avgDegree = this.estimateAverageDegree(graph);
          const estimatedNodes = Math.pow(avgDegree, depth);
          const estimatedTimeMs = estimatedNodes * 0.5; // ~0.5ms per node

          // Check attestation requirement
          const requiredAttestation = DEPTH_ATTESTATION_REQUIREMENTS[depth];
          const canAfford = depth <= agentStatus.maxDepth && agentStatus.explorationRemaining > 0;

          return {
            estimatedNodes: Math.min(estimatedNodes, graph.nodes.size),
            estimatedTimeMs,
            resourceCredits: this.calculateCredits(depth, estimatedNodes),
            attestationRequired: requiredAttestation ?? undefined,
            rateLimitImpact: `${agentStatus.explorationRemaining} of ${agentStatus.explorationLimit} queries remaining this hour`,
            canExecute: canAfford,
            blockedReason: !canAfford
              ? this.getBlockedReason(depth, agentStatus.maxDepth)
              : undefined,
          };
        }

        if (operation === 'findPath') {
          const canAfford =
            agentStatus.tier === 'path-creator' && agentStatus.pathfindingRemaining > 0;

          return {
            estimatedNodes: graph.nodes.size, // Worst case: traverse all nodes
            estimatedTimeMs: graph.nodes.size * 0.1,
            resourceCredits: 10, // Pathfinding is expensive
            attestationRequired: 'path-creator',
            rateLimitImpact: `${agentStatus.pathfindingRemaining} of ${agentStatus.pathfindingLimit} pathfinding queries remaining`,
            canExecute: canAfford,
            blockedReason: !canAfford ? ('insufficient-attestation' as const) : undefined,
          };
        }

        return {
          estimatedNodes: 0,
          estimatedTimeMs: 0,
          resourceCredits: 0,
          rateLimitImpact: 'Unknown operation',
          canExecute: false,
          blockedReason: 'invalid-query' as const,
        };
      })
    );
  }

  // =========================================================================
  // Rate Limit Management
  // =========================================================================

  /**
   * Get current rate limit status for an agent.
   */
  getRateLimitStatus(agentId?: string): Observable<RateLimitStatus> {
    return this.checkAttestations(agentId ?? this.currentAgentId, 0).pipe(
      map(attestation =>
        this.getRateLimitStatusSync(agentId ?? this.currentAgentId, attestation.tier)
      )
    );
  }

  /**
   * Synchronous rate limit status (for internal use).
   */
  private getRateLimitStatusSync(agentId: string, tier?: RateLimitTier): RateLimitStatus {
    const state = this.getOrCreateRateLimitState(agentId);
    const effectiveTier = tier ?? state.tier;
    const config = RATE_LIMIT_CONFIGS[effectiveTier];
    const now = Date.now();
    const resetsAt = new Date(state.windowStart + config.resetIntervalMs);

    return {
      tier: effectiveTier,
      maxDepth: config.maxDepth,
      explorationRemaining: config.queriesPerHour - state.explorationCount,
      explorationLimit: config.queriesPerHour,
      pathfindingRemaining: config.pathfindingPerHour - state.pathfindingCount,
      pathfindingLimit: config.pathfindingPerHour,
      resetsAt: resetsAt.toISOString(),
      resetsInMs: Math.max(0, resetsAt.getTime() - now),
    };
  }

  /**
   * Set the current agent ID (for auth integration).
   */
  setCurrentAgent(agentId: string): void {
    this.currentAgentId = agentId;
    this.rateLimitStatusSubject.next(this.getRateLimitStatusSync(agentId));
  }

  // =========================================================================
  // Graph Traversal Implementation
  // =========================================================================

  /**
   * BFS traversal from focus node to specified depth.
   */
  private bfsTraversal(
    graph: ContentGraph,
    query: GraphExplorationQuery,
    startTime: number
  ): GraphView {
    const focusNode = graph.nodes.get(query.focus)!;
    const ctx: BfsContext = {
      graph,
      query,
      neighbors: new Map(),
      edges: [],
      visited: new Set([query.focus]),
      nodesTraversed: 1,
      edgesExamined: 0,
    };

    let currentFrontier = [query.focus];

    for (let depth = 1; depth <= query.depth && currentFrontier.length > 0; depth++) {
      const result = this.processBfsDepth(currentFrontier, depth, ctx);
      if (result.nodesAtDepth.length > 0) {
        ctx.neighbors.set(depth, result.nodesAtDepth);
      }
      currentFrontier = result.nextFrontier;
      if (result.maxReached) break;
    }

    const computeTimeMs = Date.now() - startTime;
    const maxDepth = ctx.neighbors.size > 0 ? Math.max(...ctx.neighbors.keys()) : 0;

    return {
      focus: focusNode,
      neighbors: ctx.neighbors,
      edges: ctx.edges,
      metadata: {
        nodesReturned: ctx.visited.size,
        depthTraversed: Math.min(query.depth, maxDepth),
        computeTimeMs,
        resourceCredits: this.calculateCredits(query.depth, ctx.visited.size),
        nodesTraversed: ctx.nodesTraversed,
        edgesExamined: ctx.edgesExamined,
        queriedAt: new Date().toISOString(),
      },
    };
  }

  /** Process one depth level of BFS */
  private processBfsDepth(
    frontier: string[],
    depth: number,
    ctx: BfsContext
  ): { nextFrontier: string[]; nodesAtDepth: ContentNode[]; maxReached: boolean } {
    const nextFrontier: string[] = [];
    const nodesAtDepth: ContentNode[] = [];

    for (const nodeId of frontier) {
      const adjacentIds = ctx.graph.adjacency.get(nodeId) ?? new Set<string>();
      for (const adjacentId of adjacentIds) {
        ctx.edgesExamined++;
        if (
          this.processAdjacentNode(nodeId, adjacentId, nodesAtDepth, nextFrontier, ctx) &&
          ctx.query.maxNodes &&
          ctx.visited.size >= ctx.query.maxNodes
        ) {
          return { nextFrontier, nodesAtDepth, maxReached: true };
        }
      }
      if (ctx.query.maxNodes && ctx.visited.size >= ctx.query.maxNodes) {
        return { nextFrontier, nodesAtDepth, maxReached: true };
      }
    }
    return { nextFrontier, nodesAtDepth, maxReached: false };
  }

  /** Process a single adjacent node during BFS */
  private processAdjacentNode(
    sourceId: string,
    targetId: string,
    nodesAtDepth: ContentNode[],
    nextFrontier: string[],
    ctx: BfsContext
  ): boolean {
    const relationship = this.findRelationship(ctx.graph, sourceId, targetId);

    // Apply relationship filter
    if (!this.passesRelationshipFilter(relationship, ctx.query.relationshipFilter)) {
      return false;
    }

    // Add edge
    if (relationship) {
      ctx.edges.push({
        source: sourceId,
        target: targetId,
        relationshipType: relationship.relationshipType,
      });
    }

    // Process unvisited nodes
    if (ctx.visited.has(targetId)) return false;

    ctx.visited.add(targetId);
    ctx.nodesTraversed++;

    const node = ctx.graph.nodes.get(targetId);
    if (node) {
      // Apply content type filter
      if (
        !this.passesContentTypeFilter(
          node,
          ctx.query.contentTypeFilter,
          ctx.query.excludeContentTypes
        )
      ) {
        // Node is filtered out, but we still add it to frontier for traversal
        // This allows traversing THROUGH filtered nodes to reach others
        nextFrontier.push(targetId);
        return true;
      }

      const nodeToAdd = ctx.query.includeContent === false ? this.stripContent(node) : node;
      nodesAtDepth.push(nodeToAdd);
      nextFrontier.push(targetId);
    }
    return true;
  }

  /** Check if relationship passes the filter */
  private passesRelationshipFilter(
    relationship: ContentRelationship | null,
    filter: string | string[] | undefined
  ): boolean {
    if (!filter) return true;
    if (!relationship) return true;
    const filters = Array.isArray(filter) ? filter : [filter];
    return filters.includes(relationship.relationshipType);
  }

  /** Check if a node passes content type filters */
  private passesContentTypeFilter(
    node: ContentNode,
    includeFilter: string | string[] | undefined,
    excludeFilter: string[] | undefined
  ): boolean {
    // Check exclusion first
    if (excludeFilter?.includes(node.contentType)) {
      return false;
    }

    // Check inclusion filter
    if (!includeFilter) return true;
    const filters = Array.isArray(includeFilter) ? includeFilter : [includeFilter];
    return filters.includes(node.contentType);
  }

  /**
   * Find relationship between two nodes.
   */
  private findRelationship(
    graph: ContentGraph,
    sourceId: string,
    targetId: string
  ): ContentRelationship | null {
    for (const [, rel] of graph.relationships) {
      if (rel.sourceNodeId === sourceId && rel.targetNodeId === targetId) {
        return rel;
      }
    }
    return null;
  }

  /**
   * Strip content body from node for lighter responses.
   */
  private stripContent(node: ContentNode): ContentNode {
    return {
      ...node,
      content: '[content stripped for performance]',
    };
  }

  // =========================================================================
  // Pathfinding Implementation
  // =========================================================================

  /**
   * Dijkstra's algorithm for shortest path.
   */
  private dijkstraPath(
    graph: ContentGraph,
    query: PathfindingQuery,
    startTime: number
  ): PathResult | null {
    const ctx = this.initPathfindingContext(graph, query);
    this.runDijkstra(ctx, () => 1); // Uniform weight of 1
    return this.buildPathResult(graph, query, ctx, startTime);
  }

  /** Initialize pathfinding context */
  private initPathfindingContext(graph: ContentGraph, query: PathfindingQuery): PathfindingContext {
    const distances = new Map<string, number>();
    for (const nodeId of graph.nodes.keys()) {
      distances.set(nodeId, nodeId === query.from ? 0 : Infinity);
    }
    return {
      graph,
      query,
      distances,
      previous: new Map(),
      unvisited: new Set(graph.nodes.keys()),
      nodesTraversed: 0,
      edgesExamined: 0,
    };
  }

  /** Run Dijkstra's algorithm with custom weight function */
  private runDijkstra(
    ctx: PathfindingContext,
    getWeight: (rel: ContentRelationship | null, neighborId: string) => number
  ): void {
    while (ctx.unvisited.size > 0) {
      const result = this.findMinDistanceNode(ctx);
      if (!result) break;

      const { nodeId: current, distance: minDist } = result;
      if (current === ctx.query.to) break;

      ctx.unvisited.delete(current);
      ctx.nodesTraversed++;

      if (ctx.query.maxHops && minDist >= ctx.query.maxHops) continue;

      this.updateNeighborDistances(current, minDist, ctx, getWeight);
    }
  }

  /** Find the unvisited node with minimum distance */
  private findMinDistanceNode(
    ctx: PathfindingContext
  ): { nodeId: string; distance: number } | null {
    let minDist = Infinity;
    let minNode: string | null = null;

    for (const nodeId of ctx.unvisited) {
      const dist = ctx.distances.get(nodeId)!;
      if (dist < minDist) {
        minDist = dist;
        minNode = nodeId;
      }
    }

    return minNode && minDist < Infinity ? { nodeId: minNode, distance: minDist } : null;
  }

  /** Update distances to neighbors */
  private updateNeighborDistances(
    current: string,
    currentDist: number,
    ctx: PathfindingContext,
    getWeight: (rel: ContentRelationship | null, neighborId: string) => number
  ): void {
    const neighbors = ctx.graph.adjacency.get(current) ?? new Set<string>();
    for (const neighborId of neighbors) {
      ctx.edgesExamined++;
      if (!ctx.unvisited.has(neighborId)) continue;

      const rel = this.findRelationship(ctx.graph, current, neighborId);
      const weight = getWeight(rel, neighborId);
      const newDist = currentDist + weight;

      if (newDist < ctx.distances.get(neighborId)!) {
        ctx.distances.set(neighborId, newDist);
        ctx.previous.set(neighborId, current);
      }
    }
  }

  /** Build path result from pathfinding context */
  private buildPathResult(
    graph: ContentGraph,
    query: PathfindingQuery,
    ctx: PathfindingContext,
    startTime: number,
    semanticScore?: number
  ): PathResult | null {
    if (!ctx.previous.has(query.to) && query.from !== query.to) {
      return null;
    }

    const path = this.reconstructPath(query.to, ctx.previous);
    const edges = this.buildPathEdges(graph, path);
    const computeTimeMs = Date.now() - startTime;

    const result: PathResult = {
      path,
      edges,
      length: path.length - 1,
      metadata: {
        nodesReturned: path.length,
        depthTraversed: path.length - 1,
        computeTimeMs,
        resourceCredits: 10,
        nodesTraversed: ctx.nodesTraversed,
        edgesExamined: ctx.edgesExamined,
        queriedAt: new Date().toISOString(),
      },
    };
    if (semanticScore !== undefined) {
      result.semanticScore = semanticScore;
    }
    return result;
  }

  /** Reconstruct path from previous map */
  private reconstructPath(target: string, previous: Map<string, string>): string[] {
    const path: string[] = [];
    let current: string | undefined = target;
    while (current) {
      path.unshift(current);
      current = previous.get(current);
    }
    return path;
  }

  /** Build edges array for a path */
  private buildPathEdges(graph: ContentGraph, path: string[]): GraphEdge[] {
    const edges: GraphEdge[] = [];
    for (let i = 0; i < path.length - 1; i++) {
      const rel = this.findRelationship(graph, path[i], path[i + 1]);
      edges.push({
        source: path[i],
        target: path[i + 1],
        relationshipType: rel?.relationshipType ?? 'unknown',
      });
    }
    return edges;
  }

  /**
   * Semantic pathfinding that prefers pedagogically meaningful paths.
   */
  private semanticPath(
    graph: ContentGraph,
    query: PathfindingQuery,
    startTime: number
  ): PathResult | null {
    const relationshipWeights: Record<string, number> = {
      BELONGS_TO: 1,
      RELATES_TO: 2,
      DEPENDS_ON: 1.5,
      IMPLEMENTS: 1,
      EXTENDS: 1.5,
    };

    const ctx = this.initPathfindingContext(graph, query);

    // Run Dijkstra with semantic weights
    this.runDijkstra(ctx, rel => {
      const baseWeight = rel ? (relationshipWeights[rel.relationshipType] ?? 2) : 2;
      const relType = rel?.relationshipType;
      const preferred = relType ? query.preferredRelationships?.includes(relType) : false;
      return preferred ? baseWeight * 0.5 : baseWeight;
    });

    // Calculate semantic score
    const totalDistance = ctx.distances.get(query.to) ?? Infinity;
    const semanticScore = totalDistance < Infinity ? 1 / totalDistance : 0;

    return this.buildPathResult(graph, query, ctx, startTime, semanticScore);
  }

  // =========================================================================
  // Attestation Checking
  // =========================================================================

  /**
   * Check what exploration capabilities an agent has based on attestations.
   */
  private checkAttestations(agentId: string, requestedDepth: number): Observable<AttestationCheck> {
    return this.dataLoader.getAgentIndex().pipe(
      map(response => {
        const agent = response.agents.find(a => a.id === agentId);

        if (!agent) {
          // Unauthenticated - no access
          return {
            allowed: requestedDepth === 0,
            maxAllowedDepth: 0,
            tier: 'unauthenticated' as RateLimitTier,
            requiredAttestation: 'authentication',
            reason: 'Agent not found - authentication required',
          };
        }

        const attestations = agent.attestations ?? [];

        // Check for attestation tiers
        let tier: RateLimitTier = 'authenticated';
        let maxDepth = 1;

        if (
          attestations.includes('path-creator') ||
          attestations.includes('curriculum-architect')
        ) {
          tier = 'path-creator';
          maxDepth = 3;
        } else if (attestations.includes('advanced-researcher')) {
          tier = 'advanced-researcher';
          maxDepth = 3;
        } else if (attestations.includes('graph-researcher')) {
          tier = 'graph-researcher';
          maxDepth = 2;
        }

        const allowed = requestedDepth <= maxDepth;
        const requiredAttestation = !allowed
          ? DEPTH_ATTESTATION_REQUIREMENTS[requestedDepth]
          : undefined;

        return {
          allowed,
          maxAllowedDepth: maxDepth,
          tier,
          requiredAttestation: requiredAttestation ?? undefined,
          reason: allowed
            ? undefined
            : `Depth ${requestedDepth} requires ${requiredAttestation} attestation`,
        };
      }),
      catchError(() => {
        // If we can't load agents, treat as unauthenticated
        return of({
          allowed: requestedDepth === 0,
          maxAllowedDepth: 0,
          tier: 'unauthenticated' as RateLimitTier,
          requiredAttestation: 'authentication',
          reason: 'Could not verify agent attestations',
        });
      })
    );
  }

  // =========================================================================
  // Rate Limit Implementation
  // =========================================================================

  private getOrCreateRateLimitState(agentId: string): AgentRateLimitState {
    let state = this.rateLimits.get(agentId);
    const now = Date.now();

    if (!state) {
      state = {
        agentId,
        tier: 'authenticated',
        windowStart: now,
        explorationCount: 0,
        pathfindingCount: 0,
      };
      this.rateLimits.set(agentId, state);
    }

    // Check if window has expired
    const config = RATE_LIMIT_CONFIGS[state.tier];
    if (now - state.windowStart >= config.resetIntervalMs) {
      state.windowStart = now;
      state.explorationCount = 0;
      state.pathfindingCount = 0;
    }

    return state;
  }

  private checkRateLimit(
    agentId: string,
    type: 'exploration' | 'pathfinding'
  ): { allowed: boolean } {
    const state = this.getOrCreateRateLimitState(agentId);
    const config = RATE_LIMIT_CONFIGS[state.tier];

    if (type === 'exploration') {
      return { allowed: state.explorationCount < config.queriesPerHour };
    } else {
      return { allowed: state.pathfindingCount < config.pathfindingPerHour };
    }
  }

  /**
   * Update the tier for an agent's rate limit state.
   * Called after attestations are verified to ensure correct rate limits are applied.
   */
  private updateAgentTier(agentId: string, tier: RateLimitTier): void {
    const state = this.getOrCreateRateLimitState(agentId);
    if (state.tier !== tier) {
      state.tier = tier;
      this.rateLimitStatusSubject.next(this.getRateLimitStatusSync(agentId));
    }
  }

  private consumeRateLimit(agentId: string, type: 'exploration' | 'pathfinding'): void {
    const state = this.getOrCreateRateLimitState(agentId);

    if (type === 'exploration') {
      state.explorationCount++;
    } else {
      state.pathfindingCount++;
    }

    // Update observable
    this.rateLimitStatusSubject.next(this.getRateLimitStatusSync(agentId));
  }

  // =========================================================================
  // Helper Methods
  // =========================================================================

  private calculateCredits(depth: number, nodeCount: number): number {
    // Simple credit calculation: depth^2 * log(nodes)
    return Math.ceil(Math.pow(depth + 1, 2) * Math.log2(nodeCount + 1));
  }

  private getBlockedReason(
    depth: number,
    maxDepth: number
  ): 'insufficient-attestation' | 'rate-limit-exceeded' {
    return depth > maxDepth ? 'insufficient-attestation' : 'rate-limit-exceeded';
  }

  private estimateAverageDegree(graph: ContentGraph): number {
    if (graph.nodes.size === 0) return 0;

    let totalDegree = 0;
    for (const [, neighbors] of graph.adjacency) {
      totalDegree += neighbors.size;
    }
    return totalDegree / graph.nodes.size;
  }

  private createError(
    code: ExplorationErrorCode,
    details?: ExplorationError['details']
  ): Observable<never> {
    return throwError(() => this.buildError(code, undefined, details));
  }

  private buildError(
    code: ExplorationErrorCode,
    message?: string,
    details?: ExplorationError['details']
  ): ExplorationError {
    const messages: Record<ExplorationErrorCode, string> = {
      RESOURCE_NOT_FOUND: 'The requested resource was not found',
      DEPTH_UNAUTHORIZED: 'You do not have permission to explore at this depth',
      RATE_LIMIT_EXCEEDED: 'Query rate limit exceeded. Please wait before trying again.',
      PATHFINDING_UNAUTHORIZED: 'Pathfinding requires path-creator attestation',
      NO_PATH_EXISTS: 'No path exists between the specified resources',
      QUERY_TOO_EXPENSIVE: 'The query would exceed computational limits',
      INVALID_QUERY: 'Invalid query parameters',
    };

    return {
      code,
      message: message ?? messages[code],
      details,
    };
  }

  private logEvent(event: ExplorationEvent): void {
    this.eventLog.push(event);

    // Trim log if too large
    if (this.eventLog.length > this.MAX_EVENT_LOG_SIZE) {
      this.eventLog = this.eventLog.slice(-this.MAX_EVENT_LOG_SIZE / 2);
    }
  }

  // =========================================================================
  // Audit and Transparency
  // =========================================================================

  /**
   * Get recent exploration events for audit/analytics.
   */
  getRecentEvents(limit = 50): ExplorationEvent[] {
    return this.eventLog.slice(-limit);
  }

  /**
   * Get events for a specific agent.
   */
  getAgentEvents(agentId: string, limit = 20): ExplorationEvent[] {
    return this.eventLog.filter(e => e.agentId === agentId).slice(-limit);
  }

  // =========================================================================
  // Serialization Helpers
  // =========================================================================

  /**
   * Convert GraphView to JSON-serializable format.
   */
  serializeGraphView(view: GraphView): GraphViewSerialized {
    const neighbors: Record<number, ContentNode[]> = {};
    view.neighbors.forEach((nodes, depth) => {
      neighbors[depth] = nodes;
    });

    return {
      focus: view.focus,
      neighbors,
      edges: view.edges,
      metadata: view.metadata,
    };
  }

  /**
   * Convert serialized GraphView back to Map-based format.
   */
  deserializeGraphView(serialized: GraphViewSerialized): GraphView {
    const neighbors = new Map<number, ContentNode[]>();
    for (const [depth, nodes] of Object.entries(serialized.neighbors)) {
      neighbors.set(parseInt(depth, 10), nodes);
    }

    return {
      focus: serialized.focus,
      neighbors,
      edges: serialized.edges,
      metadata: serialized.metadata,
    };
  }
}

// =========================================================================
// Internal Types
// =========================================================================

interface AgentRateLimitState {
  agentId: string;
  tier: RateLimitTier;
  windowStart: number;
  explorationCount: number;
  pathfindingCount: number;
}

interface BfsContext {
  graph: ContentGraph;
  query: GraphExplorationQuery;
  neighbors: Map<number, ContentNode[]>;
  edges: GraphEdge[];
  visited: Set<string>;
  nodesTraversed: number;
  edgesExamined: number;
}

interface PathfindingContext {
  graph: ContentGraph;
  query: PathfindingQuery;
  distances: Map<string, number>;
  previous: Map<string, string>;
  unvisited: Set<string>;
  nodesTraversed: number;
  edgesExamined: number;
}
