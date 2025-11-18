/**
 * Base interface for all document nodes in the living documentation graph
 */
export enum NodeType {
  EPIC = 'epic',
  FEATURE = 'feature',
  SCENARIO = 'scenario'
}

export interface DocumentNode {
  /** Unique identifier for this node */
  id: string;

  /** Type of node */
  type: NodeType;

  /** Display title */
  title: string;

  /** Brief description/summary */
  description: string;

  /** Tags for categorization and filtering */
  tags: string[];

  /** File path to the source document */
  sourcePath: string;

  /** Full content of the node */
  content: string;

  /** IDs of related nodes */
  relatedNodeIds: string[];

  /** Creation timestamp */
  createdAt?: Date;

  /** Last modified timestamp */
  updatedAt?: Date;

  /** Additional metadata */
  metadata: Record<string, any>;
}

/**
 * Search result with context
 */
export interface SearchResult {
  node: DocumentNode;
  score: number;
  highlightedContent: string;
  matchedIn: ('title' | 'description' | 'content' | 'tags')[];
}
