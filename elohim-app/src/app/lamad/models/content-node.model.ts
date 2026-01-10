/**
 * ContentNode - The fundamental unit of content in the Territory.
 *
 * Part of the Lamad (למד) pillar of the Elohim Protocol.
 * Lamad provides the content graph - what you're learning/becoming.
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
 *
 * Protocol Core Integration:
 * - Uses ReachLevel from protocol-core (aliased as ContentReach for compatibility)
 * - Uses GovernanceLayer from protocol-core
 * - Uses GeographicContext from protocol-core
 * - Uses Attestation patterns from protocol-core
 */

import {
  type ReachLevel,
  type GeographicContext,
} from '@app/elohim/models/protocol-core.model';
import type { Place } from '@app/qahal/models/place.model';
import { JsonLdMetadata } from '@app/elohim/models/json-ld.model';

// Re-export GeographicContext for backward compatibility
export type { GeographicContext } from '@app/elohim/models/protocol-core.model';

/**
 * ContentBlob - Reference to large binary media for P2P distribution.
 *
 * Blobs are NOT stored in DHT (too large). Instead, ContentBlob stores:
 * - Cryptographic hash for integrity verification
 * - Size for cache planning
 * - Fallback URLs for resilience
 * - Bitrate variants for adaptive streaming
 *
 * Blobs are distributed via:
 * - HTTP Range requests (resume)
 * - HLS/DASH streaming (adaptive)
 * - Custodian network replication (P2P)
 */
export interface ContentBlob {
  /** Cryptographic hash of blob (SHA256 hex string) */
  hash: string;

  /** Size in bytes - used for cache allocation and streaming decisions */
  sizeBytes: number;

  /** MIME type (e.g., "video/mp4", "audio/mpeg", "application/pdf") */
  mimeType: string;

  /** Primary + fallback URLs for resilience (try in order) */
  fallbackUrls: string[];

  /** Bitrate in Mbps (useful for codec/quality tracking) */
  bitrateMbps?: number;

  /** Duration in seconds (for audio/video) */
  durationSeconds?: number;

  /** Codec information (H.264, H.265, VP9, AV1, AAC, OPUS, etc.) */
  codec?: string;

  /** Resolutions/bitrate variants for adaptive streaming */
  variants?: ContentBlobVariant[];

  /** Subtitle/caption tracks */
  captions?: ContentBlobCaption[];

  /** When this blob was created */
  createdAt?: string;

  /** When this blob was last verified/updated */
  verifiedAt?: string;
}

/**
 * Variant of a blob for adaptive streaming (e.g., 480p, 720p, 1080p, 4K).
 */
export interface ContentBlobVariant {
  /** Resolution (e.g., "1080p", "720p", "480p") or bitrate (e.g., "5000k") */
  label: string;

  /** Bitrate in Mbps */
  bitrateMbps: number;

  /** Width in pixels (for video) */
  width?: number;

  /** Height in pixels (for video) */
  height?: number;

  /** Fallback URLs for this variant (same structure as parent) */
  fallbackUrls: string[];

  /** Hash of this variant for verification */
  hash: string;

  /** Size in bytes */
  sizeBytes: number;
}

/**
 * Subtitle or caption track for media.
 */
export interface ContentBlobCaption {
  /** Language code (ISO 639-1: "en", "es", "fr", etc.) */
  language: string;

  /** Human-readable label ("English", "Spanish", "French with SDH") */
  label: string;

  /** Format (webvtt, srt, vtt, etc.) */
  format: 'webvtt' | 'srt' | 'vtt' | 'ass' | 'ssa';

  /** URL to caption file */
  url: string;

  /** Whether captions include hearing impaired info */
  isHardOfHearing?: boolean;
}

export interface ContentNode {
  /** Unique identifier (ActionHash in Holochain) */
  id: string;

  /** Content type - domain-specific semantic category */
  contentType: ContentType;

  /** Display title */
  title: string;

  /** Alternative name (used for videos, organizations, people - optional) */
  name?: string;

  /** Brief description/summary */
  description: string;

  /** External URL (for videos, websites, tools - optional) */
  url?: string;

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

  /** Large binary media (videos, podcasts, etc.) - Phase 1 blob pointer system */
  blobs?: ContentBlob[];

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

  /**
   * If this content IS a role (contentType: 'role').
   * Roles are capability attestation targets - knowledge/skill sets that humans
   * can demonstrate mastery of and earn attestations for.
   *
   * Examples: "TypeScript Developer", "FCT Facilitator", "Holochain Contributor"
   */
  roleMetadata?: RoleMetadata;

  // =========================================================================
  // Linked Data / Semantic Web (JSON-LD)
  // =========================================================================

  /**
   * Optional JSON-LD metadata for semantic web interoperability.
   *
   * When populated, enables this content to be serialized as Linked Data for:
   * - Schema.org structured data (SEO, rich snippets)
   * - RDF triple stores and SPARQL queries
   * - Decentralized knowledge graphs
   * - Interoperability with other semantic web systems
   *
   * Future: Schema.org types like Article, VideoObject, Course, etc.
   * Prevents tech debt - when we need semantic web export, structure is ready.
   */
  linkedData?: JsonLdMetadata;

  /**
   * ActivityPub Object type for federated social web.
   *
   * Maps ContentType to ActivityStreams vocabulary:
   * - contentType='video' → 'Video'
   * - contentType='epic'/'feature' → 'Article'
   * - contentType='book-chapter' → 'Document'
   * - contentType='simulation' → 'Application'
   * - contentType='assessment' → 'Question'
   * - Default → 'Page' (generic web content)
   *
   * Reference: https://www.w3.org/TR/activitystreams-vocabulary/#object-types
   */
  activityPubType?: 'Note' | 'Article' | 'Video' | 'Document' | 'Page' | 'Question' | 'Application';

  /**
   * Decentralized Identifier (DID) for cryptographic identity.
   *
   * Separate from `id` to maintain human-friendly URLs and filenames.
   * The `id` field remains the primary routing identifier.
   *
   * Example: "did:web:elohim.host:content:policy-maker-readme"
   *
   * Reference: https://www.w3.org/TR/did-core/
   */
  did?: string;

  /**
   * Open Graph metadata for social sharing.
   *
   * When populated, enables rich preview cards when content is shared on:
   * - Social media (Facebook, Twitter, LinkedIn, etc.)
   * - Messaging apps (Discord, Slack, etc.)
   * - Link preview services
   *
   * Auto-generated by import scripts from content metadata and git history.
   */
  openGraphMetadata?: {
    ogTitle: string;
    ogDescription: string;
    ogType: string;
    ogUrl: string;
    ogImage?: string;
    ogSiteName?: string;
    articleAuthor?: string;
    articlePublishedTime?: string;
    articleModifiedTime?: string;
    articleTag?: string[];
  };

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
 * ContentReach - Geographic scope of content visibility.
 *
 * This is an alias for ReachLevel from protocol-core, maintaining
 * backward compatibility while unifying the reach concept across pillars.
 *
 * Geographic levels (concentric circles):
 * - 'private' → Only author
 * - 'invited' → Specific agents (regardless of location)
 * - 'local' → Household/immediate dwelling
 * - 'neighborhood' → Block/building/immediate area
 * - 'municipal' → City/town
 * - 'bioregional' → Watershed/ecosystem boundary
 * - 'regional' → State/province level
 * - 'commons' → Globally public
 *
 * Note: For interest-based filtering (professional, faith, etc.),
 * see AffinityScope in protocol-core.model.ts
 */
export type ContentReach = ReachLevel;

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
  | 'discovery-assessment' // Self-discovery quizzes (Enneagram, learning style, etc.) - visible path steps
  | 'organization'
  | 'book-chapter'
  | 'tool'
  | 'role' // Capability attestation target (e.g., "TypeScript Developer", "FCT Facilitator")
  | 'path' // Learning path - curated journey through content nodes
  | 'placeholder'; // Missing/errored content - shown when content can't be loaded

/**
 * RoleMetadata - Extended metadata for role-type ContentNodes.
 *
 * Roles are capability attestation targets - knowledge/skill sets that humans
 * can demonstrate mastery of and earn attestations for.
 *
 * Examples:
 * - "TypeScript Developer" - demonstrates TS proficiency through path completion
 * - "FCT Facilitator" - earns facilitator role through FCT mastery path
 * - "Holochain Contributor" - recognized contributor to Holochain ecosystem
 *
 * Roles connect the knowledge graph to real-world capabilities and value creation.
 */
export interface RoleMetadata {
  /** Display title for the role */
  title: string;

  /** Description of what this role represents */
  description: string;

  /**
   * Category of role for grouping/filtering.
   *
   * - 'technical': Programming, engineering, technical skills
   * - 'facilitation': Teaching, mentoring, community leadership
   * - 'creative': Design, writing, content creation
   * - 'leadership': Governance, management, strategic roles
   * - 'domain': Domain-specific expertise (e.g., economics, ecology)
   * - 'other': Uncategorized roles
   */
  category: 'technical' | 'facilitation' | 'creative' | 'leadership' | 'domain' | 'other';

  /**
   * Learning paths that lead to this role attestation.
   * Completing any of these paths qualifies for role attestation.
   */
  attestationPathIds: string[];

  /**
   * Minimum mastery level required across the attestation path.
   * Default is 'apply' (attestation gate) - demonstrates practical application.
   */
  requiredMasteryLevel: import('@app/elohim/models/agent.model').MasteryLevel;

  /**
   * Skills/competencies this role encompasses.
   * Links to concept nodes that form the skill graph.
   */
  skillConceptIds?: string[];

  /**
   * Prerequisites - other roles that should be attested first.
   */
  prerequisiteRoleIds?: string[];

  /**
   * Organizations that recognize this role.
   * Useful for professional credentials/endorsements.
   */
  recognizedByOrgIds?: string[];

  /**
   * Is this role actively being used for attestations?
   * Roles can be deprecated but kept for historical attestations.
   */
  isActive: boolean;

  /**
   * How many humans currently hold this role attestation?
   * Denormalized for display purposes, updated periodically.
   */
  attestedCount?: number;
}

/**
 * ContentFormat - How the content payload should be interpreted and rendered.
 * Maps to specific renderer components via RendererRegistryService.
 *
 * Multimedia formats:
 * - 'markdown': Rich text with embedded media (default for most content)
 * - 'html': Raw HTML content
 * - 'plaintext': Unformatted text
 * - 'gherkin': Behavior-driven development scenarios
 *
 * Interactive formats:
 * - 'html5-app': Interactive web applications (e.g., https://github.com/ncase/trust)
 *   Rendered via iframe with sandbox. Ideal for simulations, games, explorable explanations.
 *   Can have attestation quizzes built to verify understanding.
 * - 'quiz-json': Structured quiz/assessment data
 * - 'assessment-json': Formal assessment instruments
 *
 * Media formats:
 * - 'video-embed': Embedded video (YouTube, Vimeo, etc.)
 * - 'video-file': Direct video file (blob-based streaming)
 * - 'audio-file': Direct audio file (podcasts, lectures, music)
 * - 'epub': E-book format
 *
 * Navigation:
 * - 'external-link': Link to external resource
 */
export type ContentFormat =
  | 'markdown'
  | 'html5-app'
  | 'video-embed'
  | 'video-file'
  | 'audio-file'
  | 'perseus-quiz-json'  // Khan Academy Perseus quiz format
  | 'external-link'
  | 'epub'
  | 'gherkin'
  | 'html'
  | 'plaintext';

/**
 * ContentPreview - Lightweight preview data for listing/composing content.
 *
 * Used for:
 * - Epic-level content listings (videos, orgs, books related to an epic)
 * - Path step previews
 * - Search results
 * - Related content suggestions
 *
 * Contains enough data to render a preview card and link without loading full content.
 */
export interface ContentPreview {
  /** Unique identifier */
  id: string;

  /** Display title */
  title: string;

  /** Short description (truncated to ~200 chars) */
  description: string;

  /** Content type for icon/styling */
  contentType: ContentType;

  /** Tags for filtering */
  tags: string[];

  // =========================================================================
  // Rich Media Fields (from Keen data)
  // =========================================================================

  /** External URL for direct linking (YouTube, org website, etc.) */
  url?: string;

  /** Display name (may differ from title, e.g., "Climate Town") */
  name?: string;

  /** Publisher/source (YouTube, book publisher, etc.) */
  publisher?: string;

  /** Category for grouping content */
  category?: string;

  // =========================================================================
  // Contributor & UI Rendering Hints
  // =========================================================================

  /** ContributorPresence ID for the creator/organization (if exists) */
  contributorPresenceId?: string;

  /** Whether this is playable media (video, audio) */
  isPlayable?: boolean;

  /** Thumbnail URL for preview (if available) */
  thumbnailUrl?: string;

  /** Estimated duration for media content */
  duration?: string;
}

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
  embedStrategy?: 'iframe' | 'steward' | 'web-component';

  /** Required browser/runtime capabilities */
  requiredCapabilities?: string[];

  /** Security policy for embedded content */
  securityPolicy?: {
    sandbox?: string[];
    csp?: string;
  };

  // =========================================================================
  // Social Graph / SEO Metadata (Open Graph protocol - platform-agnostic)
  // =========================================================================

  /** Thumbnail/preview image URL for social sharing (og:image) */
  thumbnailUrl?: string;

  /** Alt text for thumbnail image (accessibility + SEO) */
  imageAlt?: string;

  /** Canonical URL - authoritative location of this content (og:url) */
  canonicalUrl?: string;

  /** Content locale/language (og:locale, e.g., 'en_US', 'es_ES') */
  locale?: string;

  /** Original publication timestamp (article:published_time, ISO 8601) */
  publishedTime?: string;

  /** Last modification timestamp (article:modified_time, ISO 8601) */
  modifiedTime?: string;

  /** Content section/category for article metadata (article:section) */
  section?: string;

  /** Keywords for SEO and discoverability */
  keywords?: string[];

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

export type { ContentGraphMetadata as GraphMetadata };

// =============================================================================
// Content Relationship Detail (Diesel Backend)
// =============================================================================
// Full relationship model with metadata for graph sensemaking.
// Maps to the relationships table in elohim-storage.

/**
 * ContentRelationshipDetail - Full relationship record from backend.
 *
 * Extends the basic ContentRelationship with:
 * - Confidence scoring for inferred relationships
 * - Inference source tracking (how was this relationship discovered?)
 * - Bidirectionality and inverse relationship linking
 * - Provenance chain for trust/audit
 * - Reach/governance layers from protocol-core
 */
export interface ContentRelationshipDetail {
  /** Unique relationship ID */
  id: string;

  /** App ID for multi-tenant scoping */
  appId: string;

  /** Source content node ID */
  sourceNodeId: string;

  /** Target content node ID */
  targetNodeId: string;

  /** Relationship type (from ContentRelationshipType enum) */
  relationshipType: ContentRelationshipType | string;

  /**
   * Confidence score (0.0 - 1.0) for this relationship.
   * - 1.0: Explicitly defined by author
   * - 0.8-0.99: High confidence inference (structural analysis)
   * - 0.5-0.79: Medium confidence (semantic similarity)
   * - <0.5: Low confidence (speculative)
   */
  confidence: number;

  /**
   * How this relationship was discovered/created.
   * - 'author': Explicitly defined by content author
   * - 'structural': Inferred from path/chapter structure
   * - 'semantic': Inferred from content similarity
   * - 'usage': Inferred from user navigation patterns
   * - 'citation': Extracted from references/links
   * - 'system': System-generated (e.g., inverse relationships)
   */
  inferenceSource: RelationshipInferenceSource;

  /** Is this relationship bidirectional? */
  isBidirectional: boolean;

  /**
   * ID of the inverse relationship (for bidirectional pairs).
   * If A→B is CONTAINS, B→A is BELONGS_TO with linked IDs.
   */
  inverseRelationshipId?: string;

  /**
   * Provenance chain - IDs of sources that contributed to this relationship.
   * For citations: the source documents
   * For inferences: the algorithm/model versions
   */
  provenanceChain?: string[];

  /**
   * Governance layer this relationship belongs to.
   * Affects who can modify/delete it.
   */
  governanceLayer?: string;

  /**
   * Reach level for this relationship (who can see it).
   * Inherits from the more restrictive of source/target nodes.
   */
  reach: ContentReach;

  /** Additional metadata (JSON object) */
  metadata?: Record<string, unknown>;

  /** Creation timestamp */
  createdAt: string;

  /** Last updated timestamp */
  updatedAt: string;
}

/**
 * RelationshipInferenceSource - How a relationship was discovered.
 */
export type RelationshipInferenceSource =
  | 'author'      // Explicitly defined by content author
  | 'structural'  // Inferred from path/chapter structure
  | 'semantic'    // Inferred from content similarity
  | 'usage'       // Inferred from user navigation patterns
  | 'citation'    // Extracted from references/links
  | 'system';     // System-generated (e.g., inverse relationships)

/**
 * Wire format for ContentRelationshipDetail (snake_case for backend).
 */
export interface ContentRelationshipDetailWire {
  id: string;
  app_id: string;
  source_id: string;
  target_id: string;
  relationship_type: string;
  confidence: number;
  inference_source: string;
  is_bidirectional: number;  // SQLite stores boolean as 0/1
  inverse_relationship_id: string | null;
  provenance_chain_json: string | null;
  governance_layer: string | null;
  reach: string;
  metadata_json: string | null;
  created_at: string;
  updated_at: string;
}

/**
 * Transform wire format to ContentRelationshipDetail.
 */
export function transformRelationshipDetailFromWire(wire: ContentRelationshipDetailWire): ContentRelationshipDetail {
  let provenanceChain: string[] | undefined;
  try {
    provenanceChain = wire.provenance_chain_json
      ? JSON.parse(wire.provenance_chain_json)
      : undefined;
  } catch {
    // Ignore parse errors
  }

  let metadata: Record<string, unknown> | undefined;
  try {
    metadata = wire.metadata_json ? JSON.parse(wire.metadata_json) : undefined;
  } catch {
    // Ignore parse errors
  }

  return {
    id: wire.id,
    appId: wire.app_id,
    sourceNodeId: wire.source_id,
    targetNodeId: wire.target_id,
    relationshipType: wire.relationship_type as ContentRelationshipType,
    confidence: wire.confidence,
    inferenceSource: wire.inference_source as RelationshipInferenceSource,
    isBidirectional: wire.is_bidirectional === 1,
    inverseRelationshipId: wire.inverse_relationship_id ?? undefined,
    provenanceChain,
    governanceLayer: wire.governance_layer ?? undefined,
    reach: wire.reach as ContentReach,
    metadata,
    createdAt: wire.created_at,
    updatedAt: wire.updated_at,
  };
}

/**
 * Query parameters for listing relationships.
 */
export interface RelationshipQuery {
  sourceId?: string;
  targetId?: string;
  relationshipType?: string;
  minConfidence?: number;
  inferenceSource?: RelationshipInferenceSource;
  bidirectionalOnly?: boolean;
  limit?: number;
  offset?: number;
}

/**
 * Input for creating a relationship.
 */
export interface CreateRelationshipInput {
  sourceId: string;
  targetId: string;
  relationshipType: string;
  confidence?: number;
  inferenceSource?: RelationshipInferenceSource;
  createInverse?: boolean;
  inverseType?: string;
  provenanceChain?: string[];
  metadataJson?: string;
}
