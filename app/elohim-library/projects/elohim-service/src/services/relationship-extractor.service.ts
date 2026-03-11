/**
 * Relationship Extractor Service
 *
 * Analyzes ContentNodes and extracts relationships between them.
 * Uses multiple strategies:
 * - Explicit: frontmatter references (related_to, derived_from, etc.)
 * - Path-based: same epic, same user type, same category
 * - Tag-based: shared tags indicate conceptual similarity
 * - Content-based: references to other content by ID or name
 */

import {
  ContentNode,
  ContentRelationship,
  ContentRelationshipType,
} from '../models/content-node.model';

/**
 * Relationship with score for ranking
 */
export interface ScoredRelationship {
  id: string;
  sourceNodeId: string;
  targetNodeId: string;
  relationshipType: ContentRelationshipType;
  score: number;
  reason: string;
  confidence?: number;
  inferenceSource?: 'explicit' | 'path' | 'tag' | 'semantic';
}

/**
 * Options for relationship extraction
 */
export interface RelationshipExtractionOptions {
  /** Include path-based relationships */
  includePath?: boolean;
  /** Include tag-based relationships */
  includeTags?: boolean;
  /** Include content references */
  includeContent?: boolean;
  /** Minimum score threshold for relationships */
  minScore?: number;
  /** Maximum relationships per node */
  maxPerNode?: number;
}

const DEFAULT_OPTIONS: RelationshipExtractionOptions = {
  includePath: true,
  includeTags: true,
  includeContent: false, // Disabled by default - expensive O(n²) text search
  minScore: 0.5, // Higher threshold to reduce relationship count
  maxPerNode: 10, // Fewer relationships per node
};

let relationshipIdCounter = 0;

function generateRelationshipId(): string {
  return `rel-${Date.now()}-${++relationshipIdCounter}`;
}

/**
 * Extract relationships between nodes
 */
export function extractRelationships(
  nodes: ContentNode[],
  options: RelationshipExtractionOptions = {}
): ContentRelationship[] {
  const opts = { ...DEFAULT_OPTIONS, ...options };
  const relationships: ScoredRelationship[] = [];
  const nodeMap = new Map<string, ContentNode>();

  // Build node map for quick lookup
  for (const node of nodes) {
    nodeMap.set(node.id, node);
  }

  // Extract relationships for each node
  for (const node of nodes) {
    const nodeRelationships = extractNodeRelationships(node, nodeMap, opts);
    relationships.push(...nodeRelationships);
  }

  // Deduplicate and filter by score
  const filtered = deduplicateRelationships(relationships, opts.minScore || 0);

  // Limit per node
  return limitRelationshipsPerNode(filtered, opts.maxPerNode || 20);
}

/**
 * Extract relationships for a single node
 */
function extractNodeRelationships(
  node: ContentNode,
  nodeMap: Map<string, ContentNode>,
  options: RelationshipExtractionOptions
): ScoredRelationship[] {
  const relationships: ScoredRelationship[] = [];

  // 1. Explicit relationships from relatedNodeIds
  for (const relatedId of node.relatedNodeIds || []) {
    if (nodeMap.has(relatedId)) {
      relationships.push({
        id: generateRelationshipId(),
        sourceNodeId: node.id,
        targetNodeId: relatedId,
        relationshipType: determineRelationshipType(node, nodeMap.get(relatedId)!),
        score: 1,
        reason: 'explicit-reference',
        confidence: 1,
        inferenceSource: 'explicit',
      });
    }
  }

  // 2. Provenance relationships from metadata
  if (node.metadata?.derivedFrom) {
    const sourceId = node.metadata.derivedFrom;
    if (nodeMap.has(sourceId)) {
      relationships.push({
        id: generateRelationshipId(),
        sourceNodeId: node.id,
        targetNodeId: sourceId,
        relationshipType: ContentRelationshipType.DERIVED_FROM,
        score: 1,
        reason: 'provenance-link',
        confidence: 1,
        inferenceSource: 'explicit',
      });
    }
  }

  // 3. Path-based relationships
  if (options.includePath) {
    const pathRelationships = extractPathRelationships(node, nodeMap);
    relationships.push(...pathRelationships);
  }

  // 4. Tag-based relationships
  if (options.includeTags) {
    const tagRelationships = extractTagRelationships(node, nodeMap);
    relationships.push(...tagRelationships);
  }

  // 5. Content-based relationships
  if (options.includeContent) {
    const contentRelationships = extractContentRelationships(node, nodeMap);
    relationships.push(...contentRelationships);
  }

  return relationships;
}

/**
 * Extract path-based relationships (same epic, user type, etc.)
 * Optimized to only create relationships for non-source nodes with high overlap
 */
function extractPathRelationships(
  node: ContentNode,
  nodeMap: Map<string, ContentNode>
): ScoredRelationship[] {
  const relationships: ScoredRelationship[] = [];

  // Skip source nodes to reduce relationship count
  if (node.contentType === 'source') {
    return relationships;
  }

  const nodeEpic = node.metadata?.epic;
  const nodeUserType = node.metadata?.userType;

  // Skip if no epic to compare
  if (!nodeEpic || nodeEpic === 'other') {
    return relationships;
  }

  for (const [otherId, other] of nodeMap) {
    if (otherId === node.id) continue;

    // Skip source nodes as targets too
    if (other.contentType === 'source') continue;

    const otherEpic = other.metadata?.epic;
    const otherUserType = other.metadata?.userType;

    // Must share same epic to be related
    if (!otherEpic || nodeEpic !== otherEpic) continue;

    let score = 0.4; // Base score for same epic
    const reasons: string[] = ['same-epic'];

    // Same user type adds more weight
    if (nodeUserType && otherUserType && nodeUserType === otherUserType) {
      score += 0.3;
      reasons.push('same-user-type');
    }

    // Same content type (but not both sources)
    if (node.contentType === other.contentType) {
      score += 0.2;
      reasons.push('same-content-type');
    }

    // Only include high-scoring relationships
    if (score >= 0.5) {
      relationships.push({
        id: generateRelationshipId(),
        sourceNodeId: node.id,
        targetNodeId: otherId,
        relationshipType: ContentRelationshipType.RELATES_TO,
        score,
        reason: reasons.join(', '),
        confidence: score,
        inferenceSource: 'path',
      });
    }
  }

  return relationships;
}

/**
 * Extract tag-based relationships
 * Optimized with higher threshold and skip source nodes
 */
function extractTagRelationships(
  node: ContentNode,
  nodeMap: Map<string, ContentNode>
): ScoredRelationship[] {
  const relationships: ScoredRelationship[] = [];

  // Skip source nodes
  if (node.contentType === 'source') {
    return relationships;
  }

  const nodeTags = new Set(node.tags || []);
  if (nodeTags.size === 0) return relationships;

  // Skip common tags that don't indicate meaningful relationships
  const commonTags = new Set([
    'source',
    'resource',
    'scenario',
    'epic',
    'role',
    'archetype',
    'feature',
    'documentation',
  ]);

  // Pre-calculate meaningful tags once
  const meaningfulNodeTags = [...nodeTags].filter(t => !commonTags.has(t));
  if (meaningfulNodeTags.length === 0) return relationships;

  for (const [otherId, other] of nodeMap) {
    if (otherId === node.id) continue;

    // Skip source nodes
    if (other.contentType === 'source') continue;

    const otherTags = new Set(other.tags || []);
    if (otherTags.size === 0) continue;

    const meaningfulOtherTags = [...otherTags].filter(t => !commonTags.has(t));
    if (meaningfulOtherTags.length === 0) continue;

    const intersection = meaningfulNodeTags.filter(t => meaningfulOtherTags.includes(t));

    // Require at least 2 shared meaningful tags
    if (intersection.length < 2) continue;

    const union = new Set([...meaningfulNodeTags, ...meaningfulOtherTags]);
    const similarity = intersection.length / union.size;

    // Higher threshold (0.5 instead of 0.3)
    if (similarity >= 0.5) {
      relationships.push({
        id: generateRelationshipId(),
        sourceNodeId: node.id,
        targetNodeId: otherId,
        relationshipType: ContentRelationshipType.RELATES_TO,
        score: similarity,
        reason: `shared-tags: ${intersection.slice(0, 3).join(', ')}`, // Limit tag list
        confidence: similarity,
        inferenceSource: 'tag',
      });
    }
  }

  return relationships;
}

/**
 * Extract content-based relationships (references in text)
 */
function extractContentRelationships(
  node: ContentNode,
  nodeMap: Map<string, ContentNode>
): ScoredRelationship[] {
  const relationships: ScoredRelationship[] = [];
  const content = node.content?.toLowerCase() || '';

  if (!content) return relationships;

  for (const [otherId, other] of nodeMap) {
    if (otherId === node.id) continue;

    // Check if content references other node's title
    const otherTitle = other.title?.toLowerCase() || '';
    if (otherTitle.length > 3 && content.includes(otherTitle)) {
      relationships.push({
        id: generateRelationshipId(),
        sourceNodeId: node.id,
        targetNodeId: otherId,
        relationshipType: ContentRelationshipType.REFERENCES,
        score: 0.6,
        reason: `references-title: ${other.title}`,
        confidence: 0.6,
        inferenceSource: 'semantic',
      });
      continue;
    }

    // Check if content references other node's ID
    if (content.includes(otherId.toLowerCase())) {
      relationships.push({
        id: generateRelationshipId(),
        sourceNodeId: node.id,
        targetNodeId: otherId,
        relationshipType: ContentRelationshipType.REFERENCES,
        score: 0.8,
        reason: `references-id: ${otherId}`,
        confidence: 0.8,
        inferenceSource: 'semantic',
      });
    }
  }

  return relationships;
}

/**
 * Determine relationship type between two nodes
 */
function determineRelationshipType(
  source: ContentNode,
  target: ContentNode
): ContentRelationshipType {
  // Source to derived content
  if (source.contentType === 'source' || target.metadata?.derivedFrom === source.id) {
    return ContentRelationshipType.SOURCE_OF;
  }

  // Derived from source
  if (target.contentType === 'source' || source.metadata?.derivedFrom === target.id) {
    return ContentRelationshipType.DERIVED_FROM;
  }

  // Epic to role/scenario
  if (
    source.contentType === 'epic' &&
    (target.contentType === 'role' || target.contentType === 'scenario')
  ) {
    return ContentRelationshipType.CONTAINS;
  }

  // Role to scenario
  if (source.contentType === 'role' && target.contentType === 'scenario') {
    return ContentRelationshipType.CONTAINS;
  }

  // Default to general relationship
  return ContentRelationshipType.RELATES_TO;
}

/**
 * Deduplicate relationships, keeping highest scored version
 */
function deduplicateRelationships(
  relationships: ScoredRelationship[],
  minScore: number
): ScoredRelationship[] {
  const seen = new Map<string, ScoredRelationship>();

  for (const rel of relationships) {
    if (rel.score < minScore) continue;

    // Create bidirectional key
    const key = [rel.sourceNodeId, rel.targetNodeId].sort().join('::');
    const existing = seen.get(key);

    if (!existing || rel.score > existing.score) {
      seen.set(key, rel);
    }
  }

  return Array.from(seen.values());
}

/**
 * Limit relationships per node to prevent overly dense graphs
 */
function limitRelationshipsPerNode(
  relationships: ScoredRelationship[],
  maxPerNode: number
): ContentRelationship[] {
  const bySource = new Map<string, ScoredRelationship[]>();

  // Group by source
  for (const rel of relationships) {
    const existing = bySource.get(rel.sourceNodeId) || [];
    existing.push(rel);
    bySource.set(rel.sourceNodeId, existing);
  }

  // Sort each group by score and limit
  const limited: ContentRelationship[] = [];

  for (const [, rels] of bySource) {
    const sorted = [...rels].sort((a, b) => b.score - a.score);
    const top = sorted.slice(0, maxPerNode);

    for (const rel of top) {
      limited.push({
        id: rel.id,
        sourceNodeId: rel.sourceNodeId,
        targetNodeId: rel.targetNodeId,
        relationshipType: rel.relationshipType,
        confidence: rel.confidence,
        inferenceSource: rel.inferenceSource,
      });
    }
  }

  return limited;
}

/**
 * Build a relationship graph from nodes
 */
export function buildRelationshipGraph(
  nodes: ContentNode[],
  relationships: ContentRelationship[]
): Map<string, Set<string>> {
  const graph = new Map<string, Set<string>>();

  // Initialize all nodes
  for (const node of nodes) {
    graph.set(node.id, new Set());
  }

  // Add relationships
  for (const rel of relationships) {
    graph.get(rel.sourceNodeId)?.add(rel.targetNodeId);
    graph.get(rel.targetNodeId)?.add(rel.sourceNodeId);
  }

  return graph;
}

/**
 * Find connected components in the graph
 */
export function findConnectedComponents(graph: Map<string, Set<string>>): string[][] {
  const visited = new Set<string>();
  const components: string[][] = [];

  function dfs(nodeId: string, component: string[]): void {
    visited.add(nodeId);
    component.push(nodeId);

    const neighbors = graph.get(nodeId) || new Set();
    for (const neighbor of neighbors) {
      if (!visited.has(neighbor)) {
        dfs(neighbor, component);
      }
    }
  }

  for (const nodeId of graph.keys()) {
    if (!visited.has(nodeId)) {
      const component: string[] = [];
      dfs(nodeId, component);
      components.push(component);
    }
  }

  return components;
}
