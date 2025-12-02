/**
 * Content Lifecycle Model - Lifecycle state and refresh policies.
 *
 * Implements "the right to be forgotten" - not everything needs to
 * be remembered or renewed indefinitely. Content has natural lifecycles.
 *
 * Key concepts:
 * - Lifecycle Status: draft → published → stale → deprecated → archived → forgotten
 * - Refresh Policy: How often content should be reviewed
 * - Version Tracking: Enables mastery freshness comparisons
 * - Governance: Who decides lifecycle transitions
 */

/**
 * ContentLifecycle - Lifecycle state and refresh policy for content.
 *
 * Attached to ContentNode to track its lifecycle and trigger
 * mastery freshness recalculations when content changes.
 */
export interface ContentLifecycle {
  /** Content node ID */
  contentId: string;

  /** Current lifecycle status */
  status: ContentLifecycleStatus;

  /** When content was first created */
  createdAt: string;

  /** When content was published (became visible) */
  publishedAt?: string;

  /** When content was last refreshed/updated */
  lastRefreshedAt?: string;

  /** Who last refreshed it */
  lastRefreshedBy?: string;

  // =========================================================================
  // Expiration & Archival
  // =========================================================================

  /** Content has a natural expiration date */
  expiresAt?: string;

  /** Content superseded by newer content */
  deprecatedAt?: string;
  supersededBy?: string; // ContentNode ID of replacement

  /** Content archived (read-only historical record) */
  archivedAt?: string;
  archiveReason?: string;

  /** Content permanently removed */
  forgottenAt?: string;
  forgetReason?: ForgetReason;

  // =========================================================================
  // Refresh Policy
  // =========================================================================

  /** How often should this content be reviewed? */
  refreshInterval?: string; // ISO 8601 duration, e.g., "P6M"

  /** When is the next refresh due? */
  nextRefreshDue?: string;

  /** Who can refresh? */
  refreshPermissions: RefreshPermission;

  /** What happens if not refreshed on time? */
  staleAction: StaleAction;

  /** Grace period before stale action */
  gracePeriod?: string; // ISO 8601 duration

  // =========================================================================
  // Version Tracking
  // =========================================================================

  /** Current content version (hash or semver) */
  currentVersion: string;

  /** Version history for freshness comparisons */
  versionHistory: VersionEvent[];

  // =========================================================================
  // Governance
  // =========================================================================

  /** Who decides lifecycle transitions? */
  lifecycleGovernance: LifecycleGovernance;

  /** Designated steward(s) for this content */
  stewardIds?: string[];
}

/**
 * ContentLifecycleStatus - The lifecycle state of content.
 */
export type ContentLifecycleStatus =
  | 'draft' // Author working, not visible
  | 'published' // Active, fresh content
  | 'stale' // Needs review/refresh
  | 'deprecated' // Superseded, redirects to successor
  | 'archived' // Historical record, read-only
  | 'forgotten'; // Removed from graph

/**
 * RefreshPermission - Who can refresh content.
 */
export type RefreshPermission =
  | 'author' // Only original author
  | 'contributors' // Author + contributors
  | 'masters' // Anyone at create level mastery
  | 'steward' // Content steward only
  | 'community'; // Community governance

/**
 * StaleAction - What happens if content isn't refreshed on time.
 */
export type StaleAction =
  | 'flag' // Just mark as stale, keep accessible
  | 'deprecate' // Mark deprecated, suggest alternatives
  | 'archive' // Move to archive, read-only
  | 'forget'; // Remove from graph

/**
 * LifecycleGovernance - Who decides lifecycle transitions.
 */
export type LifecycleGovernance =
  | 'author' // Author controls lifecycle
  | 'steward' // Designated steward controls
  | 'community' // Community vote decides
  | 'elohim'; // AI-assisted governance

/**
 * ForgetReason - Why content was forgotten.
 */
export type ForgetReason =
  | 'author_request' // Author requested removal
  | 'governance' // Community/steward decision
  | 'expiration' // Natural expiration date passed
  | 'policy' // Policy violation
  | 'superseded'; // Fully replaced by successor

/**
 * VersionEvent - Record of content version change.
 */
export interface VersionEvent {
  version: string;
  timestamp: string;
  changedBy: string;
  changeType: ContentVersionChangeType;
  summary?: string;

  /** Content hash for integrity verification */
  contentHash?: string;

  /** Previous version for diff */
  previousVersion?: string;
}

export type ContentVersionChangeType =
  | 'initial' // First version
  | 'minor' // Small corrections, typos
  | 'major' // Significant content change
  | 'refresh' // Periodic review, no major changes
  | 'deprecation'; // Marked as deprecated

// =========================================================================
// Lifecycle Transition Events
// =========================================================================

/**
 * LifecycleTransition - Record of lifecycle status change.
 */
export interface LifecycleTransition {
  contentId: string;
  fromStatus: ContentLifecycleStatus;
  toStatus: ContentLifecycleStatus;
  transitionedAt: string;
  transitionedBy: string;
  reason?: string;

  /** For deprecation: what content replaces this */
  successorId?: string;

  /** For archival: where the archive lives */
  archiveLocation?: string;
}

// =========================================================================
// Refresh Notifications
// =========================================================================

/**
 * RefreshNotification - Notification that content needs refresh.
 *
 * Sent to:
 * - Content stewards
 * - Humans with create-level mastery (experts who can help)
 * - Original author
 */
export interface RefreshNotification {
  contentId: string;
  contentTitle: string;

  /** When the notification was created */
  createdAt: string;

  /** How urgent is this refresh? */
  urgency: RefreshUrgency;

  /** Days until stale action triggers */
  daysUntilAction?: number;

  /** What will happen if not refreshed */
  pendingAction: StaleAction;

  /** Who can respond to this notification */
  eligibleRefreshers: string[];

  /** Has anyone claimed this refresh? */
  claimedBy?: string;
  claimedAt?: string;
}

export type RefreshUrgency =
  | 'routine' // Normal refresh cycle
  | 'soon' // Approaching deadline
  | 'urgent' // Past deadline, in grace period
  | 'critical'; // Grace period expired

// =========================================================================
// Content Succession
// =========================================================================

/**
 * ContentSuccession - Tracks content replacement chains.
 *
 * When content is deprecated, it points to its successor.
 * This creates a chain that helps:
 * - Redirect old links
 * - Migrate mastery to new content
 * - Maintain knowledge graph coherence
 */
export interface ContentSuccession {
  /** The deprecated content */
  predecessorId: string;

  /** The replacing content */
  successorId: string;

  /** When the succession occurred */
  successionAt: string;

  /** How much of the predecessor's content is in the successor */
  coveragePercentage: number;

  /** Should mastery transfer? */
  masteryTransfer: MasteryTransferPolicy;

  /** Redirect behavior for links */
  redirectBehavior: 'automatic' | 'prompt' | 'none';
}

export type MasteryTransferPolicy =
  | 'full' // Transfer mastery 1:1
  | 'partial' // Transfer at reduced level
  | 'retest' // Require retest for new content
  | 'none'; // No transfer, start fresh

// =========================================================================
// Default Lifecycle Configurations
// =========================================================================

/**
 * Default lifecycle configuration by content type.
 */
export const DEFAULT_LIFECYCLE_BY_TYPE: Record<
  string,
  Partial<ContentLifecycle>
> = {
  concept: {
    refreshInterval: 'P1Y', // Concepts: yearly review
    staleAction: 'flag',
    lifecycleGovernance: 'steward',
  },
  video: {
    refreshInterval: 'P2Y', // Videos: less frequent
    staleAction: 'deprecate',
    lifecycleGovernance: 'author',
  },
  assessment: {
    refreshInterval: 'P6M', // Assessments: more frequent
    staleAction: 'flag',
    lifecycleGovernance: 'steward',
  },
  tool: {
    refreshInterval: 'P3M', // Tools: frequent (tech changes fast)
    staleAction: 'deprecate',
    lifecycleGovernance: 'community',
  },
  'book-chapter': {
    refreshInterval: 'P5Y', // Books: long-lived
    staleAction: 'archive',
    lifecycleGovernance: 'author',
  },
};

/**
 * Grace period defaults by stale action.
 */
export const DEFAULT_GRACE_PERIODS: Record<StaleAction, string> = {
  flag: 'P30D', // 30 days before flagging
  deprecate: 'P60D', // 60 days before deprecation
  archive: 'P90D', // 90 days before archival
  forget: 'P180D', // 180 days before forgetting
};
