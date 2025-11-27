import { Injectable } from '@angular/core';
import { Observable, of, throwError, BehaviorSubject } from 'rxjs';
import { map, switchMap, tap, catchError, take } from 'rxjs/operators';

import { DataLoaderService } from './data-loader.service';
import {
  ContentNode,
  ContentGraph,
  ContentRelationship,
  ContentRelationshipType
} from '../models/content-node.model';
import {
  GraphExplorationQuery,
  PathfindingQuery,
  GraphView,
  GraphViewSerialized,
  GraphEdge,
  PathResult,
  QueryCost,
  ExplorationMetadata,
  RateLimitStatus,
  RateLimitTier,
  RateLimitConfig,
  RATE_LIMIT_CONFIGS,
  DEPTH_ATTESTATION_REQUIREMENTS,
  AttestationCheck,
  ExplorationError,
  ExplorationErrorCode,
  ExplorationEvent
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
  private rateLimits: Map<string, AgentRateLimitState> = new Map();

  // Event log for analytics/audit
  private eventLog: ExplorationEvent[] = [];
  private readonly MAX_EVENT_LOG_SIZE = 1000;

  // Current agent ID (in production: from auth service)
  private currentAgentId = 'demo-learner';

  // Observable for rate limit status updates
  private rateLimitStatusSubject = new BehaviorSubject<RateLimitStatus | null>(null);
  public readonly rateLimitStatus$ = this.rateLimitStatusSubject.asObservable();

  constructor(
    private dataLoader: DataLoaderService
  ) {}

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

        // Check rate limit
        const rateLimitCheck = this.checkRateLimit(this.currentAgentId, 'exploration');
        if (!rateLimitCheck.allowed) {
          return this.createError('RATE_LIMIT_EXCEEDED', {
            rateLimitStatus: this.getRateLimitStatusSync(this.currentAgentId)
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
                creditsConsumed: result.metadata.resourceCredits
              }
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
          error: err
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
        if (attestationCheck.tier !== 'path-creator' && attestationCheck.tier !== 'advanced-researcher') {
          return this.createError('PATHFINDING_UNAUTHORIZED', {
            requiredAttestation: 'path-creator'
          });
        }

        // Check pathfinding rate limit
        const rateLimitCheck = this.checkRateLimit(this.currentAgentId, 'pathfinding');
        if (!rateLimitCheck.allowed) {
          return this.createError('RATE_LIMIT_EXCEEDED', {
            rateLimitStatus: this.getRateLimitStatusSync(this.currentAgentId)
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
            const pathResult = query.algorithm === 'shortest'
              ? this.dijkstraPath(graph, query, startTime)
              : this.semanticPath(graph, query, startTime);

            if (!pathResult) {
              return this.createError('NO_PATH_EXISTS', {
                from: query.from,
                to: query.to
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
  estimateCost(operation: string, params: Partial<GraphExplorationQuery | PathfindingQuery>): Observable<QueryCost> {
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
            blockedReason: 'query-too-expensive' as const
          };
        }

        if (operation === 'exploreNeighborhood') {
          const query = params as Partial<GraphExplorationQuery>;
          const depth = query.depth || 1;

          // Estimate based on average node degree
          const avgDegree = this.estimateAverageDegree(graph);
          const estimatedNodes = Math.pow(avgDegree, depth);
          const estimatedTimeMs = estimatedNodes * 0.5; // ~0.5ms per node

          // Check attestation requirement
          const requiredAttestation = DEPTH_ATTESTATION_REQUIREMENTS[depth];
          const canAfford = depth <= agentStatus.maxDepth &&
                           agentStatus.explorationRemaining > 0;

          return {
            estimatedNodes: Math.min(estimatedNodes, graph.nodes.size),
            estimatedTimeMs,
            resourceCredits: this.calculateCredits(depth, estimatedNodes),
            attestationRequired: requiredAttestation || undefined,
            rateLimitImpact: `${agentStatus.explorationRemaining} of ${agentStatus.explorationLimit} queries remaining this hour`,
            canExecute: canAfford,
            blockedReason: !canAfford
              ? (depth > agentStatus.maxDepth ? 'insufficient-attestation' as const : 'rate-limit-exceeded' as const)
              : undefined
          };
        }

        if (operation === 'findPath') {
          const canAfford = agentStatus.tier === 'path-creator' &&
                           agentStatus.pathfindingRemaining > 0;

          return {
            estimatedNodes: graph.nodes.size, // Worst case: traverse all nodes
            estimatedTimeMs: graph.nodes.size * 0.1,
            resourceCredits: 10, // Pathfinding is expensive
            attestationRequired: 'path-creator',
            rateLimitImpact: `${agentStatus.pathfindingRemaining} of ${agentStatus.pathfindingLimit} pathfinding queries remaining`,
            canExecute: canAfford,
            blockedReason: !canAfford ? 'insufficient-attestation' as const : undefined
          };
        }

        return {
          estimatedNodes: 0,
          estimatedTimeMs: 0,
          resourceCredits: 0,
          rateLimitImpact: 'Unknown operation',
          canExecute: false,
          blockedReason: 'invalid-query' as const
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
    return this.checkAttestations(agentId || this.currentAgentId, 0).pipe(
      map(attestation => this.getRateLimitStatusSync(agentId || this.currentAgentId, attestation.tier))
    );
  }

  /**
   * Synchronous rate limit status (for internal use).
   */
  private getRateLimitStatusSync(agentId: string, tier?: RateLimitTier): RateLimitStatus {
    const state = this.getOrCreateRateLimitState(agentId);
    const effectiveTier = tier || state.tier;
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
      resetsInMs: Math.max(0, resetsAt.getTime() - now)
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
    const neighbors = new Map<number, ContentNode[]>();
    const edges: GraphEdge[] = [];
    const visited = new Set<string>([query.focus]);

    // Initialize with focus at depth 0
    let currentFrontier = [query.focus];
    let nodesTraversed = 1;
    let edgesExamined = 0;

    for (let depth = 1; depth <= query.depth && currentFrontier.length > 0; depth++) {
      const nextFrontier: string[] = [];
      const nodesAtDepth: ContentNode[] = [];

      for (const nodeId of currentFrontier) {
        // Get adjacent nodes
        const adjacentIds = graph.adjacency.get(nodeId) || new Set();

        for (const adjacentId of adjacentIds) {
          edgesExamined++;

          // Get the relationship
          const relationship = this.findRelationship(graph, nodeId, adjacentId);

          // Apply relationship filter if specified
          if (query.relationshipFilter) {
            const filters = Array.isArray(query.relationshipFilter)
              ? query.relationshipFilter
              : [query.relationshipFilter];
            if (relationship && !filters.includes(relationship.relationshipType as ContentRelationshipType)) {
              continue;
            }
          }

          // Add edge
          if (relationship) {
            edges.push({
              source: nodeId,
              target: adjacentId,
              relationshipType: relationship.relationshipType
            });
          }

          // Process unvisited nodes
          if (!visited.has(adjacentId)) {
            visited.add(adjacentId);
            nodesTraversed++;

            const node = graph.nodes.get(adjacentId);
            if (node) {
              // Optionally strip content for performance
              const nodeToAdd = query.includeContent === false
                ? this.stripContent(node)
                : node;
              nodesAtDepth.push(nodeToAdd);
              nextFrontier.push(adjacentId);
            }

            // Check max nodes limit
            if (query.maxNodes && visited.size >= query.maxNodes) {
              break;
            }
          }
        }

        if (query.maxNodes && visited.size >= query.maxNodes) {
          break;
        }
      }

      if (nodesAtDepth.length > 0) {
        neighbors.set(depth, nodesAtDepth);
      }
      currentFrontier = nextFrontier;
    }

    const computeTimeMs = Date.now() - startTime;

    return {
      focus: focusNode,
      neighbors,
      edges,
      metadata: {
        nodesReturned: visited.size,
        depthTraversed: Math.min(query.depth, neighbors.size > 0 ? Math.max(...neighbors.keys()) : 0),
        computeTimeMs,
        resourceCredits: this.calculateCredits(query.depth, visited.size),
        nodesTraversed,
        edgesExamined,
        queriedAt: new Date().toISOString()
      }
    };
  }

  /**
   * Find relationship between two nodes.
   */
  private findRelationship(graph: ContentGraph, sourceId: string, targetId: string): ContentRelationship | null {
    // Try direct relationship
    const directId = `${sourceId}_${targetId}`;
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
      content: '[content stripped for performance]'
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
    const distances = new Map<string, number>();
    const previous = new Map<string, string>();
    const unvisited = new Set<string>(graph.nodes.keys());
    let nodesTraversed = 0;
    let edgesExamined = 0;

    // Initialize distances
    for (const nodeId of graph.nodes.keys()) {
      distances.set(nodeId, nodeId === query.from ? 0 : Infinity);
    }

    while (unvisited.size > 0) {
      // Find minimum distance node
      let minDist = Infinity;
      let current: string | null = null;

      for (const nodeId of unvisited) {
        const dist = distances.get(nodeId)!;
        if (dist < minDist) {
          minDist = dist;
          current = nodeId;
        }
      }

      if (!current || minDist === Infinity) break;
      if (current === query.to) break;

      unvisited.delete(current);
      nodesTraversed++;

      // Check max hops
      if (query.maxHops && minDist >= query.maxHops) continue;

      // Update neighbors
      const neighbors = graph.adjacency.get(current) || new Set();
      for (const neighborId of neighbors) {
        edgesExamined++;
        if (!unvisited.has(neighborId)) continue;

        const newDist = minDist + 1;
        if (newDist < distances.get(neighborId)!) {
          distances.set(neighborId, newDist);
          previous.set(neighborId, current);
        }
      }
    }

    // Reconstruct path
    if (!previous.has(query.to) && query.from !== query.to) {
      return null;
    }

    const path: string[] = [];
    let current: string | undefined = query.to;
    while (current) {
      path.unshift(current);
      current = previous.get(current);
    }

    // Build edges along path
    const edges: GraphEdge[] = [];
    for (let i = 0; i < path.length - 1; i++) {
      const rel = this.findRelationship(graph, path[i], path[i + 1]);
      edges.push({
        source: path[i],
        target: path[i + 1],
        relationshipType: rel?.relationshipType || 'unknown'
      });
    }

    const computeTimeMs = Date.now() - startTime;

    return {
      path,
      edges,
      length: path.length - 1,
      metadata: {
        nodesReturned: path.length,
        depthTraversed: path.length - 1,
        computeTimeMs,
        resourceCredits: 10,
        nodesTraversed,
        edgesExamined,
        queriedAt: new Date().toISOString()
      }
    };
  }

  /**
   * Semantic pathfinding that prefers pedagogically meaningful paths.
   */
  private semanticPath(
    graph: ContentGraph,
    query: PathfindingQuery,
    startTime: number
  ): PathResult | null {
    // For prototype: use weighted Dijkstra with relationship type preferences
    const relationshipWeights: Record<string, number> = {
      'BELONGS_TO': 1,
      'RELATES_TO': 2,
      'DEPENDS_ON': 1.5,
      'IMPLEMENTS': 1,
      'EXTENDS': 1.5
    };

    const distances = new Map<string, number>();
    const previous = new Map<string, string>();
    const unvisited = new Set<string>(graph.nodes.keys());
    let nodesTraversed = 0;
    let edgesExamined = 0;

    for (const nodeId of graph.nodes.keys()) {
      distances.set(nodeId, nodeId === query.from ? 0 : Infinity);
    }

    while (unvisited.size > 0) {
      let minDist = Infinity;
      let current: string | null = null;

      for (const nodeId of unvisited) {
        const dist = distances.get(nodeId)!;
        if (dist < minDist) {
          minDist = dist;
          current = nodeId;
        }
      }

      if (!current || minDist === Infinity) break;
      if (current === query.to) break;

      unvisited.delete(current);
      nodesTraversed++;

      const neighbors = graph.adjacency.get(current) || new Set();
      for (const neighborId of neighbors) {
        edgesExamined++;
        if (!unvisited.has(neighborId)) continue;

        // Get relationship weight
        const rel = this.findRelationship(graph, current, neighborId);
        const weight = rel
          ? (relationshipWeights[rel.relationshipType] || 2)
          : 2;

        // Prefer relationships specified in query
        let adjustedWeight = weight;
        if (query.preferredRelationships && rel) {
          if (query.preferredRelationships.includes(rel.relationshipType as ContentRelationshipType)) {
            adjustedWeight *= 0.5; // Prefer these relationships
          }
        }

        const newDist = minDist + adjustedWeight;
        if (newDist < distances.get(neighborId)!) {
          distances.set(neighborId, newDist);
          previous.set(neighborId, current);
        }
      }
    }

    // Reconstruct path
    if (!previous.has(query.to) && query.from !== query.to) {
      return null;
    }

    const path: string[] = [];
    let current: string | undefined = query.to;
    while (current) {
      path.unshift(current);
      current = previous.get(current);
    }

    // Build edges along path
    const edges: GraphEdge[] = [];
    for (let i = 0; i < path.length - 1; i++) {
      const rel = this.findRelationship(graph, path[i], path[i + 1]);
      edges.push({
        source: path[i],
        target: path[i + 1],
        relationshipType: rel?.relationshipType || 'unknown'
      });
    }

    const computeTimeMs = Date.now() - startTime;

    // Calculate semantic score (lower is better, invert for display)
    const totalDistance = distances.get(query.to) || Infinity;
    const semanticScore = totalDistance < Infinity ? 1 / totalDistance : 0;

    return {
      path,
      edges,
      length: path.length - 1,
      semanticScore,
      metadata: {
        nodesReturned: path.length,
        depthTraversed: path.length - 1,
        computeTimeMs,
        resourceCredits: 10,
        nodesTraversed,
        edgesExamined,
        queriedAt: new Date().toISOString()
      }
    };
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
            reason: 'Agent not found - authentication required'
          };
        }

        const attestations = agent.attestations || [];

        // Check for attestation tiers
        let tier: RateLimitTier = 'authenticated';
        let maxDepth = 1;

        if (attestations.includes('path-creator') || attestations.includes('curriculum-architect')) {
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
          requiredAttestation: requiredAttestation || undefined,
          reason: allowed ? undefined : `Depth ${requestedDepth} requires ${requiredAttestation} attestation`
        };
      }),
      catchError(() => {
        // If we can't load agents, treat as unauthenticated
        return of({
          allowed: requestedDepth === 0,
          maxAllowedDepth: 0,
          tier: 'unauthenticated' as RateLimitTier,
          requiredAttestation: 'authentication',
          reason: 'Could not verify agent attestations'
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
        pathfindingCount: 0
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

  private checkRateLimit(agentId: string, type: 'exploration' | 'pathfinding'): { allowed: boolean } {
    const state = this.getOrCreateRateLimitState(agentId);
    const config = RATE_LIMIT_CONFIGS[state.tier];

    if (type === 'exploration') {
      return { allowed: state.explorationCount < config.queriesPerHour };
    } else {
      return { allowed: state.pathfindingCount < config.pathfindingPerHour };
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

  private estimateAverageDegree(graph: ContentGraph): number {
    if (graph.nodes.size === 0) return 0;

    let totalDegree = 0;
    for (const [, neighbors] of graph.adjacency) {
      totalDegree += neighbors.size;
    }
    return totalDegree / graph.nodes.size;
  }

  private createError(code: ExplorationErrorCode, details?: any): Observable<never> {
    return throwError(() => this.buildError(code, undefined, details));
  }

  private buildError(code: ExplorationErrorCode, message?: string, details?: any): ExplorationError {
    const messages: Record<ExplorationErrorCode, string> = {
      'RESOURCE_NOT_FOUND': 'The requested resource was not found',
      'DEPTH_UNAUTHORIZED': 'You do not have permission to explore at this depth',
      'RATE_LIMIT_EXCEEDED': 'Query rate limit exceeded. Please wait before trying again.',
      'PATHFINDING_UNAUTHORIZED': 'Pathfinding requires path-creator attestation',
      'NO_PATH_EXISTS': 'No path exists between the specified resources',
      'QUERY_TOO_EXPENSIVE': 'The query would exceed computational limits',
      'INVALID_QUERY': 'Invalid query parameters'
    };

    return {
      code,
      message: message || messages[code],
      details
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
    return this.eventLog
      .filter(e => e.agentId === agentId)
      .slice(-limit);
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
      metadata: view.metadata
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
      metadata: serialized.metadata
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
