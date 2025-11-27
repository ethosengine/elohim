/**
 * Exploration Model - Graph traversal and discovery types.
 *
 * These models support the ExplorationService which handles graph queries,
 * pathfinding, and research operations. Key principles:
 *
 * - Exploration is intentional, not casual (fog-of-war principle)
 * - Depth access is gated by attestations
 * - All queries have visible computational cost
 * - Rate limits enforce sustainable exploration patterns
 *
 * From API Spec Section 1.4 and 3.4:
 * "The ExplorationService receives query parameters and validates that
 * the requesting agent has appropriate attestations for the requested depth."
 */

import { ContentNode, ContentRelationshipType } from './content-node.model';

// ============================================================================
// Exploration Query Types
// ============================================================================

/**
 * GraphExplorationQuery - Parameters for exploring the knowledge graph.
 *
 * Maps to route: /lamad/explore?focus={resourceId}&depth={1|2|3}&relationship={type}&view={graph|list|tree}
 */
export interface GraphExplorationQuery {
  /** Center point of exploration - the resource ID to explore from */
  focus: string;

  /** How many hops from the focus to include
   * - 0: Focus only
   * - 1: Immediate neighbors (all authenticated users)
   * - 2: Requires "graph-researcher" attestation
   * - 3: Requires "advanced-researcher" attestation
   */
  depth: 0 | 1 | 2 | 3;

  /** Optional: Filter by relationship type */
  relationshipFilter?: ContentRelationshipType | ContentRelationshipType[];

  /** How to display results (used by UI, service returns data for all views) */
  view?: 'graph' | 'list' | 'tree';

  /** Optional: Limit maximum nodes returned (for performance) */
  maxNodes?: number;

  /** Optional: Include content body or just metadata */
  includeContent?: boolean;
}

/**
 * PathfindingQuery - Parameters for finding paths between resources.
 *
 * Maps to route: /lamad/explore?from={resourceA}&to={resourceB}&algorithm={shortest|semantic}
 */
export interface PathfindingQuery {
  /** Starting resource ID */
  from: string;

  /** Destination resource ID */
  to: string;

  /** Algorithm to use:
   * - shortest: Dijkstra's algorithm (minimum hops)
   * - semantic: Considers relationship types, prefers pedagogically meaningful paths
   */
  algorithm: 'shortest' | 'semantic';

  /** Optional: Maximum path length to consider */
  maxHops?: number;

  /** Optional: Relationship types to prefer (for semantic algorithm) */
  preferredRelationships?: ContentRelationshipType[];
}

// ============================================================================
// Exploration Result Types
// ============================================================================

/**
 * GraphView - Result of a neighborhood exploration.
 *
 * Represents a subgraph centered on a focus resource.
 */
export interface GraphView {
  /** Center of exploration */
  focus: ContentNode;

  /** Neighboring resources organized by hop distance.
   * Key is hop distance (1, 2, 3), value is array of nodes at that distance.
   */
  neighbors: Map<number, ContentNode[]>;

  /** Relationships between nodes in the subgraph */
  edges: GraphEdge[];

  /** Query metadata showing computational cost */
  metadata: ExplorationMetadata;
}

/**
 * GraphViewSerialized - JSON-serializable version of GraphView.
 *
 * Maps cannot be serialized to JSON, so this version uses a record.
 * Use for HTTP responses and state serialization.
 */
export interface GraphViewSerialized {
  focus: ContentNode;
  neighbors: Record<number, ContentNode[]>;
  edges: GraphEdge[];
  metadata: ExplorationMetadata;
}

/**
 * GraphEdge - A relationship between two nodes in the exploration result.
 */
export interface GraphEdge {
  /** Source node ID */
  source: string;

  /** Target node ID */
  target: string;

  /** Type of relationship */
  relationshipType: ContentRelationshipType | string;

  /** Optional: Weight for pathfinding algorithms */
  weight?: number;

  /** Optional: Metadata about the relationship */
  metadata?: Record<string, unknown>;
}

/**
 * ExplorationMetadata - Information about the query's computational cost.
 *
 * This is always returned to make cost visible to users.
 * "Users understand that graph queries are not free."
 */
export interface ExplorationMetadata {
  /** Number of nodes returned in result */
  nodesReturned: number;

  /** Actual depth traversed (may be less than requested if graph terminates) */
  depthTraversed: number;

  /** Time taken to execute the query */
  computeTimeMs: number;

  /** Credits consumed for REA accounting */
  resourceCredits: number;

  /** Total nodes examined during traversal (may exceed nodesReturned) */
  nodesTraversed?: number;

  /** Total edges examined during traversal */
  edgesExamined?: number;

  /** Timestamp when query was executed */
  queriedAt: string;
}

/**
 * PathResult - Result of a pathfinding query.
 */
export interface PathResult {
  /** Sequence of resource IDs forming the path, from source to destination */
  path: string[];

  /** Full nodes in order (if requested) */
  nodes?: ContentNode[];

  /** Edges along the path */
  edges: GraphEdge[];

  /** Total path length (number of hops) */
  length: number;

  /** For semantic algorithm: score indicating path quality */
  semanticScore?: number;

  /** Query metadata */
  metadata: ExplorationMetadata;
}

// ============================================================================
// Cost Estimation Types
// ============================================================================

/**
 * QueryCost - Estimated cost before executing a query.
 *
 * Allows users to preview expense before committing resources.
 */
export interface QueryCost {
  /** Estimated number of nodes that will be traversed */
  estimatedNodes: number;

  /** Estimated execution time in milliseconds */
  estimatedTimeMs: number;

  /** Resource credits this query will consume */
  resourceCredits: number;

  /** If depth/operation requires attestation, which one */
  attestationRequired?: string;

  /** Human-readable rate limit impact */
  rateLimitImpact: string;

  /** Whether the user can afford this query */
  canExecute: boolean;

  /** If cannot execute, why not */
  blockedReason?: 'rate-limit-exceeded' | 'insufficient-attestation' | 'query-too-expensive';
}

// ============================================================================
// Rate Limiting Types
// ============================================================================

/**
 * RateLimitTier - Rate limit tiers based on attestations.
 *
 * From spec:
 * - Unauthenticated: No access
 * - Authenticated (no attestation): 10 depth-1 queries/hour
 * - graph-researcher: 25 depth-2 queries/hour
 * - advanced-researcher: 50 depth-3 queries/hour
 * - path-creator: 5 pathfinding queries/hour
 */
export type RateLimitTier =
  | 'unauthenticated'
  | 'authenticated'
  | 'graph-researcher'
  | 'advanced-researcher'
  | 'path-creator';

/**
 * RateLimitConfig - Configuration for a rate limit tier.
 */
export interface RateLimitConfig {
  tier: RateLimitTier;
  maxDepth: 0 | 1 | 2 | 3;
  queriesPerHour: number;
  pathfindingPerHour: number;
  resetIntervalMs: number;
}

/**
 * Default rate limit configurations per tier.
 */
export const RATE_LIMIT_CONFIGS: Record<RateLimitTier, RateLimitConfig> = {
  'unauthenticated': {
    tier: 'unauthenticated',
    maxDepth: 0,
    queriesPerHour: 0,
    pathfindingPerHour: 0,
    resetIntervalMs: 3600000 // 1 hour
  },
  'authenticated': {
    tier: 'authenticated',
    maxDepth: 1,
    queriesPerHour: 10,
    pathfindingPerHour: 0,
    resetIntervalMs: 3600000
  },
  'graph-researcher': {
    tier: 'graph-researcher',
    maxDepth: 2,
    queriesPerHour: 25,
    pathfindingPerHour: 0,
    resetIntervalMs: 3600000
  },
  'advanced-researcher': {
    tier: 'advanced-researcher',
    maxDepth: 3,
    queriesPerHour: 50,
    pathfindingPerHour: 0,
    resetIntervalMs: 3600000
  },
  'path-creator': {
    tier: 'path-creator',
    maxDepth: 3,
    queriesPerHour: 50,
    pathfindingPerHour: 5,
    resetIntervalMs: 3600000
  }
};

/**
 * RateLimitStatus - Current rate limit state for an agent.
 */
export interface RateLimitStatus {
  /** Agent's current tier */
  tier: RateLimitTier;

  /** Maximum depth allowed for this tier */
  maxDepth: number;

  /** Exploration queries remaining this hour */
  explorationRemaining: number;

  /** Exploration query limit for this tier */
  explorationLimit: number;

  /** Pathfinding queries remaining this hour */
  pathfindingRemaining: number;

  /** Pathfinding query limit for this tier */
  pathfindingLimit: number;

  /** When the rate limit resets (ISO string) */
  resetsAt: string;

  /** Milliseconds until reset */
  resetsInMs: number;
}

// ============================================================================
// Exploration Errors
// ============================================================================

/**
 * ExplorationErrorCode - Error codes specific to exploration operations.
 */
export type ExplorationErrorCode =
  | 'RESOURCE_NOT_FOUND'
  | 'DEPTH_UNAUTHORIZED'
  | 'RATE_LIMIT_EXCEEDED'
  | 'PATHFINDING_UNAUTHORIZED'
  | 'NO_PATH_EXISTS'
  | 'QUERY_TOO_EXPENSIVE'
  | 'INVALID_QUERY';

/**
 * ExplorationError - Error structure for exploration failures.
 */
export interface ExplorationError {
  code: ExplorationErrorCode;
  message: string;
  details?: {
    requestedDepth?: number;
    allowedDepth?: number;
    requiredAttestation?: string;
    rateLimitStatus?: RateLimitStatus;
    queryCost?: QueryCost;
  };
}

// ============================================================================
// Attestation Requirements
// ============================================================================

/**
 * Mapping from depth levels to required attestations.
 */
export const DEPTH_ATTESTATION_REQUIREMENTS: Record<number, string | null> = {
  0: null, // No attestation needed for depth 0
  1: null, // Authenticated users can do depth 1
  2: 'graph-researcher',
  3: 'advanced-researcher'
};

/**
 * AttestationCheck - Result of checking an agent's exploration permissions.
 */
export interface AttestationCheck {
  /** Whether the agent can perform the requested operation */
  allowed: boolean;

  /** Maximum depth this agent can explore */
  maxAllowedDepth: number;

  /** Agent's effective rate limit tier */
  tier: RateLimitTier;

  /** If not allowed, the attestation needed */
  requiredAttestation?: string;

  /** Human-readable explanation */
  reason?: string;
}

// ============================================================================
// Helper Types for UI
// ============================================================================

/**
 * ExplorationViewState - State model for exploration UI.
 */
export interface ExplorationViewState {
  /** Current query parameters */
  query: GraphExplorationQuery | null;

  /** Current result */
  result: GraphView | null;

  /** Loading state */
  loading: boolean;

  /** Error state */
  error: ExplorationError | null;

  /** Rate limit status */
  rateLimitStatus: RateLimitStatus | null;

  /** History of recent explorations for back navigation */
  history: GraphExplorationQuery[];
}

/**
 * ExplorationEvent - Events emitted during exploration for logging/analytics.
 */
export interface ExplorationEvent {
  type: 'query-started' | 'query-completed' | 'query-failed' | 'rate-limit-hit';
  timestamp: string;
  agentId: string;
  query?: GraphExplorationQuery | PathfindingQuery;
  result?: {
    nodesReturned: number;
    computeTimeMs: number;
    creditsConsumed: number;
  };
  error?: ExplorationError;
}
