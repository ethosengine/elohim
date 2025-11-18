import { DocumentNode } from './document-node.model';
import { EpicNode } from './epic-node.model';
import { FeatureNode } from './feature-node.model';
import { ScenarioNode } from './scenario-node.model';
import { NodeRelationship } from './node-relationship.model';

/**
 * Represents the complete documentation graph
 * This structure is designed to be easily serializable for future graph DB migration
 */
export interface DocumentGraph {
  /** All nodes in the graph */
  nodes: Map<string, DocumentNode>;

  /** All relationships in the graph */
  relationships: Map<string, NodeRelationship>;

  /** Index by node type for quick filtering */
  nodesByType: {
    epics: Map<string, EpicNode>;
    features: Map<string, FeatureNode>;
    scenarios: Map<string, ScenarioNode>;
  };

  /** Index by tag for quick filtering */
  nodesByTag: Map<string, Set<string>>; // tag -> Set of node IDs

  /** Index by category for quick filtering */
  nodesByCategory: Map<string, Set<string>>; // category -> Set of node IDs

  /** Adjacency list for quick traversal */
  adjacency: Map<string, Set<string>>; // nodeId -> Set of connected node IDs

  /** Reverse adjacency list */
  reverseAdjacency: Map<string, Set<string>>; // nodeId -> Set of nodes pointing to it

  /** Graph metadata */
  metadata: GraphMetadata;
}

export interface GraphMetadata {
  /** Total number of nodes */
  nodeCount: number;

  /** Total number of relationships */
  relationshipCount: number;

  /** Last build timestamp */
  lastBuilt: Date;

  /** Source directories/files scanned */
  sources: {
    epicPath: string;
    featurePath: string;
  };

  /** Statistics */
  stats: {
    epicCount: number;
    featureCount: number;
    scenarioCount: number;
    averageConnectionsPerNode: number;
  };
}

/**
 * Query interface for graph traversal
 */
export interface GraphQuery {
  /** Starting node ID */
  startNodeId?: string;

  /** Node type filter */
  nodeTypes?: string[];

  /** Tag filter (AND logic) */
  tags?: string[];

  /** Category filter */
  categories?: string[];

  /** Maximum traversal depth */
  maxDepth?: number;

  /** Relationship types to follow */
  relationshipTypes?: string[];

  /** Search text */
  searchText?: string;
}

/**
 * Graph traversal result
 */
export interface GraphTraversalResult {
  /** Nodes found */
  nodes: DocumentNode[];

  /** Relationships traversed */
  relationships: NodeRelationship[];

  /** Path from start to each node */
  paths: Map<string, string[]>; // nodeId -> path of node IDs

  /** Depth of each node from start */
  depths: Map<string, number>; // nodeId -> depth
}

/**
 * Serializable graph format for export
 */
export interface SerializableGraph {
  nodes: DocumentNode[];
  relationships: NodeRelationship[];
  metadata: GraphMetadata;
  version: string;
}

/**
 * Export graph to JSON-LD format (for future semantic web compatibility)
 */
export interface GraphJSONLD {
  '@context': {
    '@vocab': string;
    epic: string;
    feature: string;
    scenario: string;
    describes: string;
    implements: string;
  };
  '@graph': Array<{
    '@id': string;
    '@type': string;
    [key: string]: any;
  }>;
}
