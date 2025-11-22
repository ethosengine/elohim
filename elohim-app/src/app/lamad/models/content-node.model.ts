/**
 * Generic content node model inspired by WordPress's post concept.
 *
 * This flexible model can represent any type of content across domains:
 * - Documentation (epics, features, scenarios)
 * - Learning content (tutorials, exercises, assessments)
 * - Social content (posts, comments, discussions)
 * - Any custom domain content
 *
 * The key is the graph-based structure with flexible metadata that allows
 * each domain to extend the base model without rigid type hierarchies.
 */

export interface ContentNode {
  /** Unique identifier */
  id: string;

  /** Content type - domain-specific (e.g., 'epic', 'feature', 'scenario', 'tutorial') */
  contentType: string;

  /** Type property for compatibility with DocumentNode (optional, same as contentType) */
  type?: string;

  /** Display title */
  title: string;

  /** Brief description/summary */
  description: string;

  /** Full content body (markdown, HTML, Gherkin, etc.) */
  content: string;

  /** Content format for rendering ('markdown' | 'gherkin' | 'html' | 'plaintext') */
  contentFormat: ContentFormat;

  /** Tags for categorization and search */
  tags: string[];

  /** Source file path (if applicable) */
  sourcePath?: string;

  /** Related node IDs (bidirectional relationships) */
  relatedNodeIds: string[];

  /** Flexible metadata for domain-specific data */
  metadata: ContentMetadata;

  /** Creation timestamp */
  createdAt?: Date;

  /** Last updated timestamp */
  updatedAt?: Date;
}

export type ContentFormat = 'markdown' | 'gherkin' | 'html' | 'plaintext';

/**
 * Flexible metadata that can be extended per domain
 */
export interface ContentMetadata {
  /** Category for organization */
  category?: string;

  /** Authors/contributors */
  authors?: string[];

  /** Version number */
  version?: string;

  /** Status (planned, in-progress, completed, deprecated, etc.) */
  status?: string;

  /** Priority or order */
  priority?: number;

  /** Custom domain-specific fields */
  [key: string]: any;
}

/**
 * Relationship between nodes in the content graph
 */
export interface ContentRelationship {
  id: string;
  sourceNodeId: string;
  targetNodeId: string;
  relationshipType: ContentRelationshipType;
  metadata?: Record<string, any>;
}

export enum ContentRelationshipType {
  /** Parent-child hierarchical relationship */
  CONTAINS = 'CONTAINS',

  /** Child-parent reverse relationship */
  BELONGS_TO = 'BELONGS_TO',

  /** High-level narrative describes implementation */
  DESCRIBES = 'DESCRIBES',

  /** Implementation realizes narrative */
  IMPLEMENTS = 'IMPLEMENTS',

  /** Validation/testing relationship */
  VALIDATES = 'VALIDATES',

  /** Generic related content */
  RELATES_TO = 'RELATES_TO',

  /** Reference or citation */
  REFERENCES = 'REFERENCES',

  /** Dependency (this depends on that) */
  DEPENDS_ON = 'DEPENDS_ON',

  /** Prerequisite (must complete before this) */
  REQUIRES = 'REQUIRES',

  /** Suggested next content */
  FOLLOWS = 'FOLLOWS',
}

export { ContentRelationshipType as RelationshipType };

/**
 * Graph structure for content nodes
 */
export interface ContentGraph {
  /** All nodes keyed by ID */
  nodes: Map<string, ContentNode>;

  /** All relationships keyed by ID */
  relationships: Map<string, ContentRelationship>;

  /** Nodes organized by content type */
  nodesByType: Map<string, Set<string>>;

  /** Nodes organized by tag */
  nodesByTag: Map<string, Set<string>>;

  /** Nodes organized by category */
  nodesByCategory: Map<string, Set<string>>;

  /** Adjacency list for graph traversal */
  adjacency: Map<string, Set<string>>;

  /** Reverse adjacency for reverse lookup */
  reverseAdjacency: Map<string, Set<string>>;

  /** Graph metadata */
  metadata: ContentGraphMetadata;
}

export interface ContentGraphMetadata {
  /** Total number of nodes */
  nodeCount: number;

  /** Total number of relationships */
  relationshipCount: number;

  /** Last updated timestamp */
  lastUpdated: Date;

  /** Version of the graph schema */
  version: string;
}

/**
 * Utility type for migrating from old DocumentNode to ContentNode
 */
export interface DocumentNodeAdapter {
  fromDocumentNode(documentNode: any): ContentNode;
  toDocumentNode(contentNode: ContentNode): any;
}

// Alias for backward compatibility
export { ContentGraphMetadata as GraphMetadata };
