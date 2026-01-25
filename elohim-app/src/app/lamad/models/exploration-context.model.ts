/**
 * Exploration Context Models
 *
 * Types for maintaining path context during exploration detours
 * and organizing related concepts for discovery.
 */

import { ContentNode } from './content-node.model';

/**
 * Path context maintained during learning navigation.
 * Enables "Return to Path" functionality when exploring related content.
 */
export interface PathContext {
  /** The learning path ID */
  pathId: string;

  /** Display title of the path */
  pathTitle: string;

  /** Current step index within the path */
  stepIndex: number;

  /** Total number of steps in the path */
  totalSteps: number;

  /** Current chapter title (if path has chapters) */
  chapterTitle?: string;

  /** Route segments to return to current position */
  returnRoute: string[];

  /** Stack of detours taken from this path position */
  detourStack?: DetourInfo[];
}

/**
 * Information about an exploration detour from a path.
 */
export interface DetourInfo {
  /** Content ID we detoured from */
  fromContentId: string;

  /** Content ID we detoured to */
  toContentId: string;

  /** Type of detour taken */
  detourType: 'related' | 'prerequisite' | 'extension' | 'graph-explore';

  /** When the detour was taken */
  timestamp: string;
}

/**
 * Related concepts grouped by relationship type.
 * Used by RelatedConceptsPanelComponent for organized display.
 */
export interface RelatedConceptsResult {
  /** Concepts that should be understood before this one (DEPENDS_ON, PREREQUISITE, FOUNDATION incoming) */
  prerequisites: ContentNode[];

  /** Concepts that extend or build on this one (EXTENDS outgoing) */
  extensions: ContentNode[];

  /** Generally related concepts (RELATES_TO bidirectional) */
  related: ContentNode[];

  /** Child concepts contained within this one (CONTAINS outgoing) */
  children: ContentNode[];

  /** Parent concepts that contain this one (CONTAINS incoming) */
  parents: ContentNode[];

  /** All raw relationship edges for graph visualization */
  allRelationships: RelationshipEdge[];
}

/**
 * A relationship edge between two concepts.
 */
export interface RelationshipEdge {
  id: string;
  source: string;
  target: string;
  type: RelationshipType;
  metadata?: {
    level?: number;
    [key: string]: unknown;
  };
}

/**
 * Relationship types aligned with Holochain DNA.
 */
export type RelationshipType =
  | 'CONTAINS'
  | 'DEPENDS_ON'
  | 'RELATES_TO'
  | 'EXTENDS'
  | 'PREREQUISITE'
  | 'FOUNDATION'
  | 'IMPLEMENTS'
  | 'REFERENCES'
  | 'DERIVED_FROM';

/**
 * Relationship type groupings for UI display.
 */
export const PREREQUISITE_RELATIONSHIP_TYPES: RelationshipType[] = [
  'PREREQUISITE',
  'FOUNDATION',
  'DEPENDS_ON',
];

export const EXTENSION_RELATIONSHIP_TYPES: RelationshipType[] = ['EXTENDS'];

export const RELATED_RELATIONSHIP_TYPES: RelationshipType[] = ['RELATES_TO'];

export const HIERARCHY_RELATIONSHIP_TYPES: RelationshipType[] = ['CONTAINS'];

/**
 * Lightweight graph view for mini-graph visualization.
 */
export interface MiniGraphData {
  /** The focus node at center */
  focus: MiniGraphNode;

  /** Neighboring nodes */
  neighbors: MiniGraphNode[];

  /** Edges connecting nodes */
  edges: MiniGraphEdge[];
}

/**
 * Node representation for mini-graph.
 */
export interface MiniGraphNode {
  id: string;
  title: string;
  contentType: string;
  isFocus: boolean;
  /** Distance from focus (1 = direct neighbor) */
  depth: number;
}

/**
 * Edge representation for mini-graph.
 */
export interface MiniGraphEdge {
  source: string;
  target: string;
  relationshipType: RelationshipType;
}

/**
 * Options for querying related concepts.
 */
export interface RelatedConceptsOptions {
  /** Maximum number of concepts per category */
  limit?: number;

  /** Include only specific relationship types */
  includeTypes?: RelationshipType[];

  /** Exclude specific relationship types */
  excludeTypes?: RelationshipType[];

  /** Include full content or just metadata */
  includeContent?: boolean;
}

/**
 * Query options for neighborhood graph.
 */
export interface NeighborhoodQueryOptions {
  /** Traversal depth from focus node */
  depth?: number;

  /** Maximum nodes to return */
  maxNodes?: number;

  /** Relationship types to follow */
  relationshipTypes?: RelationshipType[];
}
