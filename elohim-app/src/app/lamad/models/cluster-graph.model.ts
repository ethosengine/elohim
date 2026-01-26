/**
 * Cluster Graph Models for Hierarchical Graph Visualization
 *
 * These models support the hierarchical clustering view in GraphExplorerComponent,
 * where content is organized by learning path structure:
 *
 * Path → Chapter → Module → Section → Concept
 *
 * Clusters can be expanded/collapsed to show their children, enabling
 * lazy loading of the graph for performance with 3000+ relationships.
 */

import * as d3 from 'd3';

/**
 * Cluster type indicates the hierarchical level of a cluster node.
 */
export type ClusterType = 'path' | 'chapter' | 'module' | 'section' | null;

/**
 * ClusterNode extends D3 simulation node with cluster-specific properties.
 *
 * A cluster node can represent either:
 * - A collapsed cluster (path, chapter, module, or section)
 * - An individual concept (leaf node)
 */
export interface ClusterNode extends d3.SimulationNodeDatum {
  // Identity
  id: string;
  title: string;
  description?: string;

  // Content type for rendering
  contentType: string;

  // Cluster hierarchy
  isCluster: boolean;
  clusterType: ClusterType;
  clusterLevel: number; // 0=path, 1=chapter, 2=module, 3=section, 4=concept

  // Hierarchy relationships
  parentClusterId: string | null;
  childClusterIds: string[];
  conceptIds: string[]; // Leaf concepts (populated for sections)

  // Visual state
  isExpanded: boolean;
  isLoading: boolean;

  // Aggregated metrics
  totalConceptCount: number;
  completedConceptCount: number;
  externalConnectionCount: number;

  // Learning state (for affinity visualization)
  state: 'unseen' | 'in-progress' | 'proficient' | 'recommended' | 'review' | 'locked';
  affinityScore: number;

  // D3 simulation properties (inherited)
  x?: number;
  y?: number;
  fx?: number | null;
  fy?: number | null;

  // Visual sizing
  clusterRadius?: number;

  // Optional metadata from path structure
  order?: number;
  estimatedMinutes?: number;
  attestationGranted?: string;
}

/**
 * ClusterConnection represents aggregated relationships between clusters.
 *
 * Instead of showing individual concept-to-concept edges (which could be thousands),
 * we aggregate connections between clusters and show a summary.
 */
export interface ClusterConnection {
  sourceClusterId: string;
  targetClusterId: string;
  connectionCount: number;
  relationshipTypes: string[];
}

/**
 * ClusterConnectionDetail - Full relationship metadata for cluster edges.
 *
 * Extends ClusterConnection with relationship data from the backend,
 * enabling advanced graph sensemaking features:
 * - Filter edges by relationship type
 * - Color edges by confidence level
 * - Show inference source breakdown
 * - Identify bidirectional vs unidirectional links
 *
 * Used by HierarchicalGraphService for graph visualization.
 */
export interface ClusterConnectionDetail extends ClusterConnection {
  /**
   * Relationship summaries underlying this aggregated connection.
   * Only populated when expanded/requested (lazy loading).
   */
  relationships: RelationshipSummary[];

  /**
   * Primary relationship type (most common among underlying relationships).
   */
  primaryRelationshipType: string;

  /**
   * Average confidence across all underlying relationships (0.0 - 1.0).
   * Higher confidence = stronger/more certain connection.
   */
  averageConfidence: number;

  /**
   * Does this connection have bidirectional links?
   * True if any underlying relationship is marked bidirectional.
   */
  hasBidirectionalLinks: boolean;

  /**
   * Breakdown of how relationships were inferred.
   * Key: inference source, Value: count
   * Useful for understanding connection provenance.
   */
  inferenceSourceCounts: Record<string, number>;

  /**
   * Minimum confidence among underlying relationships.
   * Useful for filtering weak connections.
   */
  minConfidence: number;

  /**
   * Maximum confidence among underlying relationships.
   */
  maxConfidence: number;

  /**
   * Count of unique relationship types.
   */
  uniqueTypeCount: number;
}

/**
 * RelationshipSummary - Lightweight summary of a relationship for aggregation.
 *
 * Contains enough data for visualization without full relationship payload.
 */
export interface RelationshipSummary {
  /** Relationship ID */
  id: string;

  /** Source content node ID */
  sourceId: string;

  /** Target content node ID */
  targetId: string;

  /** Relationship type */
  relationshipType: string;

  /** Confidence score (0.0 - 1.0) */
  confidence: number;

  /** How this relationship was discovered */
  inferenceSource: string;

  /** Is this relationship bidirectional? */
  isBidirectional: boolean;
}

/**
 * Create a ClusterConnectionDetail from basic connection + relationships.
 */
export function createClusterConnectionDetail(
  basic: ClusterConnection,
  relationships: RelationshipSummary[]
): ClusterConnectionDetail {
  // Count inference sources
  const inferenceSourceCounts: Record<string, number> = {};
  for (const rel of relationships) {
    inferenceSourceCounts[rel.inferenceSource] =
      (inferenceSourceCounts[rel.inferenceSource] || 0) + 1;
  }

  // Count relationship types
  const typeCounts: Record<string, number> = {};
  for (const rel of relationships) {
    typeCounts[rel.relationshipType] = (typeCounts[rel.relationshipType] || 0) + 1;
  }

  // Find primary type (most common)
  let primaryType = basic.relationshipTypes[0] || 'RELATES_TO';
  let maxCount = 0;
  for (const [type, count] of Object.entries(typeCounts)) {
    if (count > maxCount) {
      maxCount = count;
      primaryType = type;
    }
  }

  // Calculate confidence stats
  const confidences = relationships.map(r => r.confidence);
  const avgConfidence =
    confidences.length > 0 ? confidences.reduce((a, b) => a + b, 0) / confidences.length : 0;
  const minConfidence = confidences.length > 0 ? Math.min(...confidences) : 0;
  const maxConfidence = confidences.length > 0 ? Math.max(...confidences) : 0;

  return {
    ...basic,
    relationships,
    primaryRelationshipType: primaryType,
    averageConfidence: avgConfidence,
    hasBidirectionalLinks: relationships.some(r => r.isBidirectional),
    inferenceSourceCounts,
    minConfidence,
    maxConfidence,
    uniqueTypeCount: Object.keys(typeCounts).length,
  };
}

/**
 * ClusterEdge for D3 force simulation.
 * Represents an edge between visible nodes (either clusters or concepts).
 */
export interface ClusterEdge extends d3.SimulationLinkDatum<ClusterNode> {
  source: string | ClusterNode;
  target: string | ClusterNode;
  type: string;
  isAggregated: boolean; // True if this represents multiple underlying relationships
  connectionCount?: number;
}

/**
 * ClusterGraphData is the complete graph state for rendering.
 */
export interface ClusterGraphData {
  root: ClusterNode;
  clusters: Map<string, ClusterNode>;
  edges: ClusterEdge[];
  connections: ClusterConnection[];
}

/**
 * ClusterExpansionResult is returned when a cluster is expanded.
 */
export interface ClusterExpansionResult {
  clusterId: string;
  children: ClusterNode[];
  edges: ClusterEdge[];
  connections: ClusterConnection[];
}

/**
 * ClusterConnectionSummary aggregates relationships for a set of concepts.
 */
export interface ClusterConnectionSummary {
  /** Cluster ID these connections are for */
  clusterId: string;

  /** Total outgoing connections to other clusters */
  outgoingByCluster: Map<string, ClusterConnection>;

  /** Total incoming connections from other clusters */
  incomingByCluster: Map<string, ClusterConnection>;

  /** Total connection count */
  totalConnections: number;
}

/**
 * HierarchyLevel maps cluster levels to their properties.
 */
export const CLUSTER_LEVEL_CONFIG: Record<
  number,
  {
    name: string;
    baseRadius: number;
    color: string;
    strokeColor: string;
  }
> = {
  0: { name: 'Path', baseRadius: 60, color: 'rgba(99, 102, 241, 0.3)', strokeColor: '#6366f1' },
  1: { name: 'Chapter', baseRadius: 50, color: 'rgba(99, 102, 241, 0.2)', strokeColor: '#6366f1' },
  2: { name: 'Module', baseRadius: 35, color: 'rgba(34, 197, 94, 0.2)', strokeColor: '#22c55e' },
  3: { name: 'Section', baseRadius: 25, color: 'rgba(250, 204, 21, 0.2)', strokeColor: '#facc15' },
  4: { name: 'Concept', baseRadius: 15, color: '#64748b', strokeColor: '#94a3b8' },
};

/**
 * Create a cluster node from path chapter data.
 */
export function createChapterCluster(
  chapter: {
    id: string;
    title: string;
    description?: string;
    order: number;
    estimatedDuration?: string;
    attestationGranted?: string;
  },
  pathId: string,
  moduleCount: number,
  totalConceptCount: number
): ClusterNode {
  return {
    id: chapter.id,
    title: chapter.title,
    description: chapter.description,
    contentType: 'chapter',
    isCluster: true,
    clusterType: 'chapter',
    clusterLevel: 1,
    parentClusterId: pathId,
    childClusterIds: [], // Will be populated with module IDs
    conceptIds: [],
    isExpanded: false,
    isLoading: false,
    totalConceptCount,
    completedConceptCount: 0,
    externalConnectionCount: 0,
    state: 'unseen',
    affinityScore: 0,
    order: chapter.order,
    attestationGranted: chapter.attestationGranted,
  };
}

/**
 * Create a cluster node from path module data.
 */
export function createModuleCluster(
  module: { id: string; title: string; description?: string; order: number },
  chapterId: string,
  sectionCount: number,
  totalConceptCount: number
): ClusterNode {
  return {
    id: module.id,
    title: module.title,
    description: module.description,
    contentType: 'module',
    isCluster: true,
    clusterType: 'module',
    clusterLevel: 2,
    parentClusterId: chapterId,
    childClusterIds: [], // Will be populated with section IDs
    conceptIds: [],
    isExpanded: false,
    isLoading: false,
    totalConceptCount,
    completedConceptCount: 0,
    externalConnectionCount: 0,
    state: 'unseen',
    affinityScore: 0,
    order: module.order,
  };
}

/**
 * Create a cluster node from path section data.
 */
export function createSectionCluster(
  section: {
    id: string;
    title: string;
    description?: string;
    order: number;
    conceptIds: string[];
    estimatedMinutes?: number;
  },
  moduleId: string
): ClusterNode {
  return {
    id: section.id,
    title: section.title,
    description: section.description,
    contentType: 'section',
    isCluster: true,
    clusterType: 'section',
    clusterLevel: 3,
    parentClusterId: moduleId,
    childClusterIds: [],
    conceptIds: section.conceptIds,
    isExpanded: false,
    isLoading: false,
    totalConceptCount: section.conceptIds.length,
    completedConceptCount: 0,
    externalConnectionCount: 0,
    state: 'unseen',
    affinityScore: 0,
    order: section.order,
    estimatedMinutes: section.estimatedMinutes,
  };
}

/**
 * Create a concept node (leaf node, not a cluster).
 */
export function createConceptNode(
  id: string,
  title: string,
  contentType: string,
  sectionId: string,
  state: ClusterNode['state'] = 'unseen',
  affinityScore = 0
): ClusterNode {
  return {
    id,
    title,
    contentType,
    isCluster: false,
    clusterType: null,
    clusterLevel: 4,
    parentClusterId: sectionId,
    childClusterIds: [],
    conceptIds: [],
    isExpanded: false,
    isLoading: false,
    totalConceptCount: 0,
    completedConceptCount: 0,
    externalConnectionCount: 0,
    state,
    affinityScore,
  };
}

/**
 * Calculate cluster radius based on concept count.
 * Uses logarithmic scaling to prevent very large clusters.
 */
export function calculateClusterRadius(cluster: ClusterNode): number {
  const baseRadius = CLUSTER_LEVEL_CONFIG[cluster.clusterLevel]?.baseRadius ?? 20;

  if (!cluster.isCluster || cluster.totalConceptCount <= 1) {
    return baseRadius;
  }

  // Logarithmic scaling: base + log(count) * factor
  const scaleFactor = 5;
  return baseRadius + Math.log2(cluster.totalConceptCount + 1) * scaleFactor;
}
