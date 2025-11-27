/**
 * ContentNode - The fundamental unit of content in the Territory.
 *
 * Holochain mapping:
 * - Entry type: "content_node"
 * - id becomes ActionHash
 * - Published to DHT based on reach level (not automatic)
 *
 * This flexible model can represent any type of content:
 * - Documentation (epics, features, scenarios)
 * - Learning content (tutorials, exercises, assessments)
 * - Media (videos, simulations, interactive apps)
 * - Any custom domain content
 *
 * Trust Model:
 * Content earns reach through attestations. New content starts at 'private'
 * and can expand to 'commons' through steward approval, community endorsement,
 * or governance ratification. See content-attestation.model.ts for details.
 */
export interface ContentNode {
  /** Unique identifier (ActionHash in Holochain) */
  id: string;

  /** Content type - domain-specific semantic category */
  contentType: ContentType;

  /** Display title */
  title: string;

  /** Brief description/summary */
  description: string;

  /** Full content body (markdown, HTML, URL, JSON, etc.) */
  content: string | object;

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

  // =========================================================================
  // Trust & Reach (Bidirectional Attestation Model)
  // =========================================================================

  /**
   * Author/creator of this content (AgentPubKey).
   * Required for trust model - anonymous content cannot earn reach beyond 'local'.
   * Optional during migration; defaults to 'system' for legacy content.
   */
  authorId?: string;

  /**
   * Current reach level - determines who can discover/access this content.
   * Computed from active attestations. See ContentReach type.
   * Defaults to 'commons' for legacy content (existing content is public).
   *
   * - 'private': Only author
   * - 'invited': Specific agents
   * - 'local': Author's network
   * - 'community': Community members
   * - 'federated': Multiple communities
   * - 'commons': Public/global
   */
  reach?: ContentReach;

  /**
   * Trust score (0.0 - 1.0) computed from attestation quality/quantity.
   * Higher scores indicate more/stronger attestations.
   * Defaults to 1.0 for legacy content.
   */
  trustScore?: number;

  /**
   * IDs of active attestations that grant this content's current reach.
   * Full attestation details fetched separately.
   * Empty array for legacy content.
   */
  activeAttestationIds?: string[];

  /**
   * Agents explicitly invited to access this content (when reach is 'invited').
   */
  invitedAgentIds?: string[];

  /**
   * Communities this content is shared with (when reach is 'community' or 'federated').
   */
  communityIds?: string[];

  /**
   * Active flags/warnings on this content (disputed, under-review, etc.).
   * Content with certain flags may have restricted reach.
   */
  flags?: ContentFlag[];

  // =========================================================================
  // Feedback Profile (What engagement is permitted)
  // =========================================================================

  /**
   * Feedback profile governing what engagement mechanisms are permitted.
   * Orthogonal to reach (reach = WHERE, feedbackProfile = HOW).
   * See feedback-profile.model.ts for details.
   *
   * "Virality is a privilege, not an entitlement."
   */
  feedbackProfileId?: string;

  /**
   * Denormalized permitted mechanisms for quick access.
   * Full profile fetched separately when needed.
   */
  permittedFeedbackMechanisms?: string[];

  // =========================================================================
  // Geographic Context (Embodied Place Awareness)
  // =========================================================================

  /**
   * Geographic context for this content (parallel to social reach).
   *
   * Social reach (ContentReach) determines WHO can access content.
   * Geographic reach determines WHERE content is physically relevant.
   * Elohim apply wisdom to align both dimensions with constitutional values.
   *
   * Example: A neighborhood newsletter might have:
   * - reach: 'community' (social - community members)
   * - geographicContext.reach: 'neighborhood' (spatial - only locally relevant)
   *
   * See place.model.ts for full geographic types.
   */
  geographicContext?: GeographicContext;

  /**
   * If this content IS a place (contentType: 'place').
   * Places are first-class ContentNodes - they have attestations, reach, governance.
   * Place names are Elohim-negotiated social constructs subject to deliberation.
   *
   * See place.model.ts for the full Place interface.
   */
  placeData?: Place;

  // =========================================================================
  // Timestamps
  // =========================================================================

  /** Creation timestamp (ISO 8601) */
  createdAt?: string;

  /** Last updated timestamp (ISO 8601) */
  updatedAt?: string;

  /** When trust profile was last computed */
  trustComputedAt?: string;
}

/**
 * ContentReach - The audience a piece of content can reach.
 * Re-exported from content-attestation.model.ts for convenience.
 */
export type ContentReach =
  | 'private'
  | 'invited'
  | 'local'
  | 'community'
  | 'federated'
  | 'commons';

/**
 * ContentFlag - Warning/issue on content (denormalized from trust profile).
 */
export interface ContentFlag {
  type: 'disputed' | 'outdated' | 'partial-revocation' | 'under-review' | 'appeal-pending';
  reason: string;
  flaggedAt: string;
}

/**
 * ContentType - The semantic type of content in the Territory.
 * Maps to different rendering strategies and metadata schemas.
 */
export type ContentType =
  | 'epic'
  | 'feature'
  | 'scenario'
  | 'concept'
  | 'simulation'
  | 'video'
  | 'assessment'
  | 'organization'
  | 'book-chapter'
  | 'tool';

/**
 * ContentFormat - How the content payload should be interpreted and rendered.
 * Maps to specific renderer components via RendererRegistryService.
 */
export type ContentFormat =
  | 'markdown'
  | 'html5-app'
  | 'video-embed'
  | 'video-file'
  | 'quiz-json'
  | 'external-link'
  | 'epub'
  | 'gherkin'
  | 'html'
  | 'plaintext';

/**
 * Flexible metadata that can be extended per domain
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

  /** Content license */
  license?: string;

  /** Estimated time to consume/complete */
  estimatedTime?: string;

  /** Embedding strategy for interactive content */
  embedStrategy?: 'iframe' | 'native' | 'web-component';

  /** Required browser/runtime capabilities */
  requiredCapabilities?: string[];

  /** Security policy for embedded content */
  securityPolicy?: {
    sandbox?: string[];
    csp?: string;
  };

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

  /** Last updated timestamp (ISO 8601 string) */
  lastUpdated: string;

  /** Version of the graph schema */
  version: string;
}

// Alias for backward compatibility
export type { ContentGraphMetadata as GraphMetadata };
