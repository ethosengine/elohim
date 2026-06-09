/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/content-graph.schema.json -- DO NOT EDIT */

/**
 * How this edge was derived: explicit = author-declared; path = co-occurrence in a learning path; tag = shared-tag proximity; semantic = embedding-distance; system = system-generated (e.g. prerequisite inference).
 */
export type InferenceSource = 'explicit' | 'path' | 'tag' | 'semantic' | 'system';

/**
 * Content relationship graph rooted at a given content id. Source of truth: multi-provenance composite read. Explicit edges (inferenceSource=explicit) are Category A (DHT-notarized Relationship entries); computed edges (tag, semantic, system) are Category C (derived on read, not independently persisted); path edges are A2 (derived via link). The view itself is a read projection, never persisted.
 */
export interface ContentGraph {
  /**
   * CID of the root content node from which the graph was resolved.
   */
  rootId: string;
  /**
   * Number of related nodes (neighbour count; the root is implicit and not counted).
   */
  totalNodes: number;
  /**
   * Direct neighbours of the root node, each carrying their own recursive children.
   */
  related: ContentGraphNode[];
}
export interface ContentGraphNode {
  /**
   * CID of this related content node.
   */
  contentId: string;
  /**
   * Semantic relationship label (e.g. CONTAINS, RELATES_TO, VALIDATES).
   */
  relationshipType: string;
  /**
   * Confidence score for this relationship edge [0, 1].
   */
  confidence: number;
  inferenceSource: InferenceSource;
  /**
   * Graph depth of this node relative to the root (root neighbours = 1).
   */
  depth: number;
  /**
   * Recursive child nodes at depth + 1.
   */
  children: ContentGraphNode[];
}
