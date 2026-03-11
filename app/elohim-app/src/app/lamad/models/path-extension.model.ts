/**
 * Path Extension Models - Learner-owned mutations to curated paths.
 *
 * Core insight: Paths should be both authoritative AND extensible.
 * - Curators create canonical paths (immutable, versioned)
 * - Learners can extend paths for their own use
 * - Extensions can be shared, forked, and merged upstream
 *
 * This enables:
 * - Personal learning customization without fragmenting curation
 * - Community contribution to path improvement
 * - A/B testing of path variations
 * - Adaptive learning through personal annotation
 *
 * Holochain mapping:
 * - Entry type: "path_extension"
 * - Links to base path via pathId
 * - Private extensions on source chain
 * - Shared extensions on DHT
 */

import { PathStep } from './learning-path.model';

/**
 * PathExtension - A learner's personal modifications to a base path.
 *
 * Extensions are layered on top of canonical paths, allowing
 * personalization without modifying the original. Think of it
 * like a "diff" that applies to a specific path version.
 */
export interface PathExtension {
  /** Unique identifier for this extension */
  id: string;

  /** The canonical path being extended */
  basePathId: string;

  /** Pinned to specific version (extensions may break on updates) */
  basePathVersion: string;

  /** Agent who created this extension */
  extendedBy: string;

  /** Human-readable title for the extension */
  title: string;

  /** Why this extension exists */
  description?: string;

  /** The modifications */
  insertions: PathStepInsertion[];
  annotations: PathStepAnnotation[];
  reorderings: PathStepReorder[];
  exclusions: PathStepExclusion[];

  /**
   * Visibility:
   * - 'private': Only the creator can see/use
   * - 'shared': Specific agents granted access
   * - 'public': Anyone can fork/use
   */
  visibility: 'private' | 'shared' | 'public';

  /** Agents granted access (when visibility is 'shared') */
  sharedWith?: string[];

  /** If this extension was forked from another */
  forkedFrom?: string;

  /** Extensions that forked from this one */
  forks?: string[];

  /** Has this been proposed for upstream merge? */
  upstreamProposal?: UpstreamProposal;

  /** Usage statistics */
  stats?: ExtensionStats;

  /** Timestamps */
  createdAt: string;
  updatedAt: string;
}

/**
 * PathStepInsertion - Insert new steps into the path.
 */
export interface PathStepInsertion {
  /** Unique ID for this insertion */
  id: string;

  /** Insert after this step index (-1 for beginning) */
  afterStepIndex: number;

  /** The steps to insert */
  steps: PathStep[];

  /** Why this was added */
  rationale?: string;

  /** Source of the insertion (who suggested it) */
  source?: InsertionSource;
}

export interface InsertionSource {
  type: 'self' | 'ai-suggestion' | 'community' | 'instructor';
  sourceId?: string;
  confidence?: number;
}

/**
 * PathStepAnnotation - Personal notes on existing steps.
 */
export interface PathStepAnnotation {
  /** Unique ID for this annotation */
  id: string;

  /** Which step this annotates */
  stepIndex: number;

  /** The annotation type */
  type: AnnotationType;

  /** Annotation content */
  content: string;

  /** Additional resources the learner found helpful */
  additionalResources?: AdditionalResource[];

  /** Personal difficulty rating */
  personalDifficulty?: 'easier' | 'as-expected' | 'harder';

  /** Time actually spent (vs estimated) */
  actualTime?: string;

  /** Created timestamp */
  createdAt: string;
}

export type AnnotationType =
  | 'note' // General notes
  | 'question' // Questions for later/mentor
  | 'insight' // Personal insights
  | 'connection' // Connections to other knowledge
  | 'struggle' // Where learner struggled
  | 'breakthrough' // Aha moments
  | 'application' // How to apply this
  | 'disagreement'; // Respectful disagreement with content

export interface AdditionalResource {
  title: string;
  url?: string;
  resourceId?: string; // If it's a Lamad content node
  description?: string;
}

/**
 * PathStepReorder - Change the order of steps for personal learning.
 */
export interface PathStepReorder {
  /** Unique ID for this reordering */
  id: string;

  /** Original step index */
  fromIndex: number;

  /** New position */
  toIndex: number;

  /** Why the reorder */
  rationale?: string;
}

/**
 * PathStepExclusion - Skip certain steps (for personal use only).
 */
export interface PathStepExclusion {
  /** Unique ID for this exclusion */
  id: string;

  /** Step to exclude */
  stepIndex: number;

  /** Why excluded */
  reason: ExclusionReason;

  /** Additional notes */
  notes?: string;
}

export type ExclusionReason =
  | 'already-mastered' // Learner already knows this
  | 'not-relevant' // Doesn't apply to learner's context
  | 'prerequisite-missing' // Need something else first
  | 'too-advanced' // Saving for later
  | 'accessibility' // Content not accessible to learner
  | 'other';

/**
 * UpstreamProposal - Suggest extension changes to path maintainers.
 */
export interface UpstreamProposal {
  /** Status of the proposal */
  status: 'draft' | 'submitted' | 'under-review' | 'accepted' | 'rejected' | 'partial';

  /** When submitted */
  submittedAt?: string;

  /** Response from maintainers */
  response?: string;

  /** Which parts were accepted (if partial) */
  acceptedParts?: string[]; // IDs of insertions/annotations
}

/**
 * ExtensionStats - Usage metrics for shared extensions.
 */
export interface ExtensionStats {
  /** How many learners use this extension */
  activeUsers: number;

  /** How many times forked */
  forkCount: number;

  /** Average rating (if rated) */
  averageRating?: number;

  /** Completion rate with this extension */
  completionRate?: number;
}

// ============================================================================
// Collaborative Paths
// ============================================================================

/**
 * CollaborativePath - Multiple authors building a path together.
 *
 * Unlike extensions (which layer on top), collaborative paths
 * are co-owned from the start. Useful for:
 * - Teams creating training materials
 * - Communities curating shared knowledge
 * - Mentor-mentee path creation
 */
export interface CollaborativePath {
  /** Path ID (same as LearningPath.id) */
  pathId: string;

  /** How collaboration works */
  collaborationType: CollaborationType;

  /** Role assignments */
  roles: Map<string, CollaboratorRole>;

  /** Pending contributions awaiting approval */
  pendingProposals: PathProposal[];

  /** Collaboration settings */
  settings: CollaborationSettings;

  /** Activity log */
  activityLog: CollaborationActivity[];
}

export type CollaborationType =
  | 'sequential' // One author at a time, pass the baton
  | 'parallel' // Multiple authors work simultaneously
  | 'review-required' // Changes need approval
  | 'open'; // Anyone with access can edit

export type CollaboratorRole =
  | 'owner' // Full control, can delete
  | 'editor' // Can make changes directly
  | 'suggester' // Can propose changes
  | 'reviewer' // Can approve/reject proposals
  | 'viewer'; // Read-only access

export interface CollaborationSettings {
  /** Require approval for changes? */
  requireApproval: boolean;

  /** Minimum approvals needed */
  minApprovals?: number;

  /** Who can approve */
  approvers?: string[];

  /** Allow anonymous suggestions? */
  allowAnonymousSuggestions: boolean;

  /** Notify on changes */
  notifyOnChange: boolean;
}

/**
 * PathProposal - A suggested change to a collaborative path.
 */
export interface PathProposal {
  /** Unique ID */
  id: string;

  /** Who proposed */
  proposedBy: string;

  /** What type of change */
  changeType: 'add-step' | 'edit-step' | 'remove-step' | 'reorder' | 'edit-metadata';

  /** The proposed change */
  change: ProposedChange;

  /** Explanation */
  rationale: string;

  /** Current status */
  status: 'pending' | 'approved' | 'rejected' | 'withdrawn';

  /** Votes (for consensus-based approval) */
  votes?: Map<string, 'approve' | 'reject' | 'abstain'>;

  /** Comments/discussion */
  comments?: ProposalComment[];

  /** Timestamps */
  createdAt: string;
  resolvedAt?: string;
}

export interface ProposedChange {
  /** For add/edit step */
  step?: PathStep;

  /** For reorder/remove */
  stepIndex?: number;

  /** For reorder */
  newIndex?: number;

  /** For metadata edits */
  metadata?: Record<string, unknown>;
}

export interface ProposalComment {
  id: string;
  authorId: string;
  content: string;
  createdAt: string;
}

export interface CollaborationActivity {
  id: string;
  type:
    | 'proposal-created'
    | 'proposal-approved'
    | 'proposal-rejected'
    | 'step-added'
    | 'step-edited'
    | 'member-joined'
    | 'member-left';
  actorId: string;
  details: Record<string, unknown>;
  timestamp: string;
}

// ============================================================================
// Extension Index & Discovery
// ============================================================================

/**
 * PathExtensionIndexEntry - Lightweight entry for extension discovery.
 */
export interface PathExtensionIndexEntry {
  id: string;
  basePathId: string;
  basePathTitle: string;
  title: string;
  description?: string;
  extendedBy: string;
  extenderName: string;
  visibility: string;
  insertionCount: number;
  annotationCount: number;
  forkCount: number;
  rating?: number;
  updatedAt: string;
}

/**
 * PathExtensionIndex - Response from extension catalog endpoint.
 */
export interface PathExtensionIndex {
  lastUpdated: string;
  totalCount: number;
  extensions: PathExtensionIndexEntry[];
}

// ============================================================================
// Extension Operations
// ============================================================================

/**
 * ApplyExtensionResult - What you get when applying an extension to a path.
 */
export interface ApplyExtensionResult {
  /** The extended path (virtual, not persisted) */
  effectiveSteps: PathStep[];

  /** Mapping from effective index to base index (or insertion ID) */
  indexMapping: Map<number, string>;

  /** Annotations keyed by effective index */
  annotations: Map<number, PathStepAnnotation[]>;

  /** Any conflicts or warnings */
  warnings: ExtensionWarning[];
}

export interface ExtensionWarning {
  type: 'version-mismatch' | 'missing-step' | 'conflict';
  message: string;
  affectedItems: string[];
}
