import { ContentNode } from './content-node.model';
import { OpenGraphMetadata } from '@app/elohim/models/open-graph.model';
import { JsonLdMetadata } from '@app/elohim/models/json-ld.model';

/**
 * LearningPath - A curated journey through Territory resources.
 *
 * Paths can be structured in two ways:
 * 1. Flat: Just `steps[]` - simple sequential journey
 * 2. Chapters: `chapters[]` containing steps - thematic groupings
 *
 * Paths can also compose other paths:
 * - A step can reference another path (stepType: 'path')
 * - This enables "journeys within journeys" without rigid hierarchy
 *
 * Holochain mapping:
 * - Entry type: "learning_path"
 * - Steps/chapters stored inline (they're small)
 * - resourceId/pathId links via action hash
 */
export interface LearningPath {
  // Identity - becomes action hash in Holochain
  id: string;
  version: string;

  // Descriptive metadata
  title: string;
  description: string;
  purpose: string;

  // Authorship - agent public keys in Holochain
  createdBy: string;
  contributors: string[];
  forkedFrom?: string;
  createdAt: string;  // ISO 8601
  updatedAt: string;

  /**
   * Journey structure - use ONE of:
   * - `steps[]` for flat sequential paths
   * - `chapters[]` for thematically grouped paths
   *
   * If both exist, `chapters` takes precedence.
   */
  steps: PathStep[];

  /**
   * Chapters - thematic groupings within a path.
   * Use for longer journeys that benefit from organization.
   * Each chapter contains its own steps.
   */
  chapters?: PathChapter[];

  // Classification
  tags: string[];
  difficulty: 'beginner' | 'intermediate' | 'advanced';
  estimatedDuration: string;

  // =========================================================================
  // Visibility & Consent (Graduated Intimacy Model)
  // =========================================================================

  /**
   * Path visibility level - determines who can discover and access this path.
   *
   * Graduated intimacy model (matching human consent levels):
   * - 'public': Anyone can discover and follow
   * - 'connections': Only mutual connections can see
   * - 'trusted': Only trusted circle can see
   * - 'intimate': Only love-map participants (emergent paths)
   *
   * Legacy values still supported:
   * - 'organization': Maps to 'connections' (deprecated)
   * - 'private': Maps to creator-only (no participants)
   */
  visibility: PathVisibility;

  /**
   * For intimate/shared paths - human IDs who can access this path.
   * Empty or undefined means only the creator can see (private).
   */
  participantIds?: string[];

  /**
   * Is this path emergent (generated from affinity analysis)?
   *
   * Emergent paths are created by comparing two humans' affinity patterns
   * and generating a bridging path that connects their shared interests.
   * These are "love map" paths - personalized journeys toward understanding.
   */
  isEmergent?: boolean;

  /**
   * For emergent paths - the negotiation that created this path.
   * Links to PathNegotiation record for provenance.
   */
  negotiationId?: string;

  /**
   * Required attestations to access this path.
   *
   * Attestation-gated visibility enables consent-verified access:
   * - "I attest I am married to X" + "I attest I am married to Y"
   * - Only X and Y (with mutual attestations) can access their love map
   *
   * This is more powerful than participantIds because:
   * - Attestations can be revoked (divorce)
   * - Attestations carry semantic meaning (not just "who" but "relationship")
   * - Attestations can be verified by others if needed
   *
   * Format: Array of attestation type IDs that are required.
   * All required attestations must be present for access.
   */
  requiredAttestations?: string[];

  /**
   * For mutual attestation requirements (love maps).
   *
   * When true, the path requires BOTH participants to have
   * attested the relationship (e.g., mutual marriage attestation).
   * The attestation must reference the other participant.
   */
  requiresMutualAttestation?: boolean;

  // Prerequisites and outcomes
  prerequisitePaths?: string[];
  attestationsGranted?: string[];

  /**
   * Path type hint for UI rendering:
   * - 'journey': Standard learning path (default)
   * - 'quest': Achievement-oriented with milestones
   * - 'expedition': Long-form deep dive
   * - 'practice': Skill-building through repetition
   */
  pathType?: 'journey' | 'quest' | 'expedition' | 'practice';

  // =========================================================================
  // Social Graph Metadata (for sharing learning paths)
  // =========================================================================

  /**
   * Open Graph metadata for social sharing.
   * When a path is shared, this provides rich preview cards.
   */
  socialMetadata?: OpenGraphMetadata;

  /**
   * Optional JSON-LD metadata for semantic web interoperability.
   *
   * Future: Schema.org Course or LearningResource types.
   * Prevents tech debt when we need semantic web export.
   */
  linkedData?: JsonLdMetadata;

  /**
   * ActivityPub Collection type for federated social web.
   *
   * Learning paths map to ActivityStreams collections:
   * - Default → 'OrderedCollection' (sequential steps maintain order)
   * - pathType='practice' → 'Collection' (order less important)
   *
   * Reference: https://www.w3.org/TR/activitystreams-vocabulary/#collections
   */
  activityPubType?: 'Collection' | 'OrderedCollection';

  /**
   * Decentralized Identifier (DID) for cryptographic identity.
   *
   * Separate from `id` to maintain human-friendly URLs and filenames.
   * The `id` field remains the primary routing identifier.
   *
   * Example: "did:web:elohim.host:paths:elohim-protocol"
   *
   * Reference: https://www.w3.org/TR/did-core/
   */
  did?: string;

  // =========================================================================
  // Visual Assets
  // =========================================================================

  /**
   * Thumbnail/cover image URL for path cards and landing pages.
   * Falls back to socialMetadata.ogImage if not set.
   */
  thumbnailUrl?: string;

  /** Alt text for thumbnail (accessibility) */
  thumbnailAlt?: string;
}

/**
 * PathChapter - A thematic grouping of steps within a path.
 *
 * Chapters provide:
 * - Visual/conceptual organization for longer journeys
 * - Optional attestations at chapter completion (milestones)
 * - Clearer progress indication ("Chapter 2 of 5")
 *
 * Named "chapter" rather than "unit" or "module" to evoke
 * narrative journey rather than institutional curriculum.
 */
export interface PathChapter {
  id: string;

  /** Chapter title */
  title: string;

  /** What this chapter covers */
  description?: string;

  /** Order within path */
  order: number;

  /** Steps in this chapter */
  steps: PathStep[];

  /** Estimated duration for this chapter */
  estimatedDuration?: string;

  /** Attestation granted on chapter completion (milestone) */
  attestationGranted?: string;

  /** Whether this chapter is optional */
  optional?: boolean;
}

/**
 * PathStep - A single step in a learning path.
 *
 * Steps can reference:
 * - Content (stepType: 'content') - default, links to ContentNode
 * - Another path (stepType: 'path') - enables composition
 * - External resource (stepType: 'external') - links outside Territory
 * - Checkpoint (stepType: 'checkpoint') - reflection/assessment moment
 */
export interface PathStep {
  order: number;

  /**
   * What this step references:
   * - 'content': A ContentNode in the Territory (default)
   * - 'path': Another LearningPath (journey composition)
   * - 'external': An external URL
   * - 'checkpoint': A reflection/assessment moment (no content reference)
   */
  stepType?: 'content' | 'path' | 'external' | 'checkpoint';

  /**
   * Reference ID - interpretation depends on stepType:
   * - content: ContentNode.id
   * - path: LearningPath.id (nested journey)
   * - external: (use externalUrl instead)
   * - checkpoint: (not used)
   */
  resourceId: string;

  /**
   * For stepType: 'path' - the nested path ID.
   * When present, this step represents completing an entire sub-journey.
   */
  pathId?: string;

  /**
   * For stepType: 'external' - the external URL.
   */
  externalUrl?: string;

  // Path-specific context (not in the content node itself)
  stepTitle: string;
  stepNarrative: string;
  learningObjectives: string[];
  reflectionPrompts?: string[];
  practiceExercises?: string[];

  // Metadata
  estimatedTime?: string;
  optional: boolean;

  // Alternatives and gating
  alternativeResourceIds?: string[];
  completionCriteria: string[];
  attestationRequired?: string;
  attestationGranted?: string;

  // Phase 6 Extension (Post-MVP) - Concepts addressed by this step
  conceptsAddressed?: string[];
}

/**
 * PathStepView - Composite returned by PathService.getPathStep()
 * Combines step context with resolved content.
 */
export interface PathStepView {
  step: PathStep;

  /** Resolved content (for stepType: 'content') */
  content: ContentNode;

  /** Resolved nested path (for stepType: 'path') */
  nestedPath?: LearningPath;

  /** Chapter context (if step is within a chapter) */
  chapter?: {
    id: string;
    title: string;
    order: number;
    stepIndexWithinChapter: number;
    totalStepsInChapter: number;
  };

  // Navigation context
  hasPrevious: boolean;
  hasNext: boolean;
  previousStepIndex?: number;
  nextStepIndex?: number;

  // Progress for authenticated user
  isCompleted?: boolean;
  affinity?: number;
  notes?: string;
}

/**
 * PathOverviewView - Rich view for path landing/overview pages.
 */
export interface PathOverviewView {
  path: LearningPath;

  /** Resolved chapter summaries (if path has chapters) */
  chapterSummaries?: ChapterSummary[];

  /** Flat step count (total across all chapters) */
  totalStepCount: number;

  /** Human's progress (if authenticated) */
  progress?: {
    completedSteps: number;
    totalRequiredSteps: number;
    completionPercentage: number;
    currentStepIndex: number;
    currentChapterIndex?: number;
    attestationsEarned: string[];
  };

  /** Nested path summaries (for steps that reference other paths) */
  nestedPathSummaries?: PathIndexEntry[];

  /** Prerequisites with completion status */
  prerequisites?: Array<{
    pathId: string;
    title: string;
    isCompleted: boolean;
  }>;
}

/**
 * ChapterSummary - Lightweight chapter info for overview display.
 */
export interface ChapterSummary {
  id: string;
  title: string;
  description?: string;
  order: number;
  stepCount: number;
  estimatedDuration?: string;
  attestationGranted?: string;

  /** Progress (if authenticated) */
  completedSteps?: number;
  isComplete?: boolean;
}

/**
 * PathIndex - Catalog entry for path discovery (lightweight)
 */
export interface PathIndexEntry {
  id: string;
  title: string;
  description: string;
  difficulty: 'beginner' | 'intermediate' | 'advanced';
  estimatedDuration: string;
  stepCount: number;
  tags: string[];

  /** Chapter count (if path uses chapters) */
  chapterCount?: number;

  /** Path type for UI hints */
  pathType?: 'journey' | 'quest' | 'expedition' | 'practice';

  /** Attestations granted upon completion */
  attestationsGranted?: string[];

  /** Category for grouping */
  category?: string;

  /** Thumbnail image URL for path cards */
  thumbnailUrl?: string;

  /** Alt text for thumbnail (accessibility) */
  thumbnailAlt?: string;
}

/**
 * PathIndex - Response from path catalog endpoint
 */
export interface PathIndex {
  lastUpdated: string;
  totalCount: number;
  paths: PathIndexEntry[];
}

// =========================================================================
// Path as ContentNode (Graph Integration)
// =========================================================================

/**
 * PathContentMetadata - Metadata for paths represented as ContentNodes on the graph.
 *
 * When a LearningPath is registered on the Content Graph, it becomes a ContentNode
 * with contentType: 'path'. This interface defines the metadata structure stored
 * in the ContentNode.metadata field.
 *
 * This enables:
 * - Graph traversal to discover paths containing specific content
 * - Paths can be found via "what paths cover topic X?" queries
 * - Paths participate in the attestation/governance system
 * - Unified search across content and paths
 *
 * The ContentNode with type 'path' links to the full LearningPath via pathId,
 * which contains the complete step definitions. The ContentNode is a lightweight
 * index entry for graph discoverability.
 */
export interface PathContentMetadata {
  /** Reference to the full LearningPath definition */
  pathId: string;

  /** Difficulty level for filtering */
  difficulty: 'beginner' | 'intermediate' | 'advanced';

  /** Human-readable estimated duration (e.g., "2-3 hours") */
  estimatedDuration: string;

  /** Total number of steps in the path */
  stepCount: number;

  /** Number of chapters (if path uses chapter organization) */
  chapterCount?: number;

  /**
   * Content node IDs referenced by this path's steps.
   * Enables reverse lookup: "which paths contain this content?"
   *
   * This is a flattened list of all resourceIds from steps/chapters.
   * Nested paths (stepType: 'path') are NOT expanded - only direct content.
   */
  contentNodeIds: string[];

  /**
   * Nested path IDs (for stepType: 'path' steps).
   * Enables path composition graph: "which paths include this path?"
   */
  nestedPathIds?: string[];

  /**
   * Creator information for branded paths.
   * Links to the ContributorPresence for economic attribution.
   */
  creatorInfo?: {
    /** ContributorPresence ID for the creator */
    presenceId: string;

    /** Brand name for display (e.g., "Bare Marriage") */
    brandName?: string;

    /** Brand logo URL for path cards */
    brandLogoUrl?: string;
  };

  /**
   * If this path was forked from another, link to the original.
   * Enables fork tree visualization and attribution.
   */
  forkedFromPathId?: string;

  /**
   * Fork generation (0 = original, 1 = direct fork, 2 = fork of fork, etc.)
   */
  forkGeneration?: number;

  /**
   * Canonical status for governance.
   * - 'draft': Author working copy
   * - 'community': Community-endorsed but not canonical
   * - 'canonical': Officially ratified path
   */
  canonicalStatus?: 'draft' | 'community' | 'canonical';

  /** Path type hint for UI rendering */
  pathType?: 'journey' | 'quest' | 'expedition' | 'practice';

  /** Attestations granted upon path completion */
  attestationsGranted?: string[];
}

/**
 * PathReference - Lightweight reference to a path, used in graph queries.
 */
export interface PathReference {
  /** Path node ID (the ContentNode ID, not the LearningPath ID) */
  nodeId: string;

  /** The underlying LearningPath ID */
  pathId: string;

  /** Path title */
  title: string;

  /** Relationship type (how this path relates to the query) */
  relationship: 'contains' | 'references' | 'requires';
}

// =========================================================================
// Path Visibility (Graduated Intimacy Model)
// =========================================================================

/**
 * PathVisibility - Graduated intimacy levels for path access.
 *
 * Mirrors human consent levels to ensure paths can only be seen
 * by those with appropriate relationship consent.
 *
 * - 'public': Anyone can discover and follow (commons)
 * - 'connections': Mutual connections only (friend-level)
 * - 'trusted': Trusted circle only (elevated trust)
 * - 'intimate': Love-map participants only (attestation-gated)
 *
 * Legacy values (deprecated but supported):
 * - 'organization': Maps to 'connections'
 * - 'private': Creator-only (no participants)
 */
export type PathVisibility =
  | 'public'
  | 'connections'
  | 'trusted'
  | 'intimate'
  // Legacy values (deprecated)
  | 'organization'
  | 'private';

/**
 * Maps legacy visibility values to graduated intimacy levels.
 */
export const VISIBILITY_MIGRATION: Record<string, PathVisibility> = {
  organization: 'connections',
  private: 'intimate', // Private paths become intimate (creator is only participant)
};

/**
 * Checks if a visibility level requires consent verification.
 */
export function requiresConsent(visibility: PathVisibility): boolean {
  return visibility !== 'public';
}

/**
 * Checks if a visibility level requires attestation verification.
 */
export function requiresAttestation(visibility: PathVisibility): boolean {
  return visibility === 'intimate';
}
