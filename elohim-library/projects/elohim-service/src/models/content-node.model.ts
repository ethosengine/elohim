/**
 * ContentNode model for elohim-service
 *
 * This is a standalone definition that mirrors the lamad ContentNode
 * to avoid Angular dependencies in the Node.js import tooling.
 *
 * When writing output, the generated JSON matches the lamad schema exactly.
 */

/**
 * Content types supported by the lamad platform
 */
export type ContentType =
  | 'source'       // Raw source file (provenance layer)
  | 'epic'         // Domain narrative
  | 'feature'      // Feature specification
  | 'scenario'     // Behavioral specification (gherkin)
  | 'concept'      // Abstract concept
  | 'role'         // Archetype/persona definition
  | 'video'        // Video content
  | 'organization' // Organization profile
  | 'book-chapter' // Reference material
  | 'tool'         // Tool/resource
  | 'path'         // Learning path (graph integration)
  | 'assessment'   // Assessment instrument
  | 'reference'    // External reference (books, articles, etc.)
  | 'example';     // Code or usage example

/**
 * Content format for rendering
 */
export type ContentFormat =
  | 'markdown'
  | 'gherkin'
  | 'html'
  | 'plaintext'
  | 'video-embed'
  | 'external-link'
  | 'quiz-json'
  | 'assessment-json';

/**
 * Relationship types between content nodes
 */
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
  /** Provenance: this content was derived from source */
  DERIVED_FROM = 'DERIVED_FROM',
  /** Provenance: source produced this derived content */
  SOURCE_OF = 'SOURCE_OF'
}

/**
 * Flexible metadata for domain-specific data
 */
export interface ContentMetadata {
  /** Category for organization */
  category?: string;
  /** Authors/contributors */
  authors?: string[];
  /** Primary author */
  author?: string;
  /** Version number */
  version?: string;
  /** Status (planned, in-progress, completed, deprecated, etc.) */
  status?: string;
  /** Priority or order */
  priority?: number;
  /** Content source identifier */
  source?: string;
  /** Original source URL */
  sourceUrl?: string;
  /** Estimated time to consume/complete */
  estimatedTime?: string;
  /** Keywords for SEO and discoverability */
  keywords?: string[];

  // Import-specific metadata
  /** Source file path (provenance) */
  sourcePath?: string;
  /** Import timestamp */
  importedAt?: string;
  /** Import tool version */
  importVersion?: string;
  /** For derived content - ID of source node */
  derivedFrom?: string;
  /** Extraction method used */
  extractionMethod?: string;
  /** Epic this content belongs to */
  epic?: string;
  /** User type/archetype */
  userType?: string;
  /** Governance scope levels */
  governanceScope?: string[];
  /** Source type (archetype-definition, epic-narrative, etc.) */
  sourceType?: string;

  /** Custom domain-specific fields */
  [key: string]: unknown;
}

/**
 * ContentNode - The fundamental unit of content in the lamad system.
 *
 * This interface defines the JSON structure written to
 * elohim-app/src/assets/lamad-data/content/*.json
 */
export interface ContentNode {
  /** Unique identifier */
  id: string;

  /** Content type - semantic category */
  contentType: ContentType;

  /** Display title */
  title: string;

  /** Brief description/summary */
  description: string;

  /** Full content body */
  content: string;

  /** Content format for rendering */
  contentFormat: ContentFormat;

  /** Tags for categorization and search */
  tags: string[];

  /** Source file path (for development/debugging) */
  sourcePath?: string;

  /** Related node IDs (bidirectional relationships) */
  relatedNodeIds: string[];

  /** Flexible metadata for domain-specific data */
  metadata: ContentMetadata;

  /** Author/creator ID (optional) */
  authorId?: string;

  /** Reach level (defaults to 'commons' for imported content) */
  reach?: 'private' | 'invited' | 'local' | 'community' | 'federated' | 'commons';

  /** Creation timestamp (ISO 8601) */
  createdAt: string;

  /** Last updated timestamp (ISO 8601) */
  updatedAt: string;
}

/**
 * Relationship between nodes in the content graph
 */
export interface ContentRelationship {
  id: string;
  sourceNodeId: string;
  targetNodeId: string;
  relationshipType: ContentRelationshipType;
  /** Confidence score for inferred relationships (0-1) */
  confidence?: number;
  /** How relationship was determined */
  inferenceSource?: 'explicit' | 'path' | 'tag' | 'semantic';
  metadata?: Record<string, unknown>;
}
