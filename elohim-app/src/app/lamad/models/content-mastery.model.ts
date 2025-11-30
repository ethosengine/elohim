/**
 * Content Mastery Model - Bloom's Taxonomy based mastery tracking.
 *
 * This model tracks a human's mastery state for specific content nodes,
 * implementing Bloom's Taxonomy progression from passive consumption
 * to active contribution.
 *
 * Key concepts:
 * - BloomMasteryLevel: 8-level progression (not_started → create)
 * - Attestation Gate: "apply" level unlocks participation privileges
 * - Freshness: Mastery decays based on time and graph evolution
 * - Expertise Discovery: Graph reveals who knows what
 *
 * Reference: Anderson & Krathwohl (2001), Bloom's Revised Taxonomy
 */

import type { BloomMasteryLevel } from './agent.model';

// Re-export for convenience
export type { BloomMasteryLevel } from './agent.model';
export { BLOOM_LEVEL_VALUES, ATTESTATION_GATE_LEVEL } from './agent.model';

/**
 * ContentMastery - A human's mastery state for a specific content node.
 *
 * This is the per-content-node tracking that powers:
 * - Khan Academy-style cross-path completion views
 * - Bloom's taxonomy progression
 * - Freshness/decay tracking
 * - Participation privilege gating
 * - Expertise discovery queries
 *
 * Stored on human's private source chain (never DHT without consent).
 */
export interface ContentMastery {
  /** Content node ID */
  contentId: string;

  /** Human/agent ID */
  humanId: string;

  /** Current Bloom's level achieved */
  level: BloomMasteryLevel;

  /** When this level was achieved */
  levelAchievedAt: string; // ISO 8601

  /** History of level progression */
  levelHistory: LevelProgressionEvent[];

  // =========================================================================
  // Freshness Tracking
  // =========================================================================

  /** Last engagement with this content */
  lastEngagementAt: string; // ISO 8601

  /** Type of last engagement */
  lastEngagementType: EngagementType;

  /** Content version when mastery was achieved */
  contentVersionAtMastery: string;

  /** Computed freshness (0.0-1.0) */
  freshness: number;

  /** Does this need refresh? */
  needsRefresh: boolean;

  /** Suggested refresh action */
  refreshType?: 'review' | 'retest' | 'relearn';

  // =========================================================================
  // Assessment Evidence
  // =========================================================================

  /** Quiz/assessment results that contributed to this level */
  assessmentEvidence: AssessmentEvidence[];

  /** Peer evaluations received (for analyze+ levels) */
  peerEvaluations?: PeerEvaluation[];

  /** Contributions made (for create level) */
  contributions?: ContentContribution[];

  // =========================================================================
  // Participation Privileges
  // =========================================================================

  /** Privileges currently granted based on level */
  privileges: ContentPrivilege[];

  /** Privileges earned but suspended (e.g., due to freshness decay) */
  suspendedPrivileges?: ContentPrivilege[];
}

/**
 * EngagementType - How the human engaged with content.
 */
export type EngagementType =
  | 'view' // Passive viewing
  | 'quiz' // Took assessment
  | 'practice' // Practice exercise
  | 'comment' // Added comment/discussion
  | 'review' // Peer reviewed content
  | 'contribute' // Made contribution
  | 'path_step' // Encountered in learning path
  | 'refresh'; // Explicit refresh engagement

/**
 * LevelProgressionEvent - Record of mastery level change.
 */
export interface LevelProgressionEvent {
  fromLevel: BloomMasteryLevel;
  toLevel: BloomMasteryLevel;
  timestamp: string;
  trigger: 'assessment' | 'engagement' | 'contribution' | 'decay' | 'refresh';
  evidence?: string; // Assessment ID, contribution ID, etc.
}

/**
 * AssessmentEvidence - Quiz/test result contributing to mastery.
 */
export interface AssessmentEvidence {
  assessmentId: string;
  assessmentType: 'recall' | 'comprehension' | 'application' | 'analysis';
  score: number; // 0.0-1.0
  passedAt: string;
  contributesToLevel: BloomMasteryLevel;
}

/**
 * PeerEvaluation - Evaluation from another human at evaluate+ level.
 */
export interface PeerEvaluation {
  evaluatorId: string;
  evaluatorLevel: BloomMasteryLevel;
  rating: number; // 0.0-1.0
  feedback?: string;
  timestamp: string;
}

/**
 * ContentContribution - Original contribution at create level.
 */
export interface ContentContribution {
  contributionType: 'comment' | 'edit' | 'derivative' | 'original';
  contributionId: string;
  createdAt: string;
  status: 'pending' | 'accepted' | 'rejected';
  peerReviewScore?: number;
}

/**
 * ContentPrivilege - What the human can do with this content.
 */
export interface ContentPrivilege {
  privilege: PrivilegeType;
  grantedAt: string;
  grantedByLevel: BloomMasteryLevel;
  active: boolean;
  suspendedReason?: 'freshness_decay' | 'moderation' | 'content_change';
}

/**
 * PrivilegeType - Available content interaction privileges.
 *
 * Privileges unlock at specific Bloom's levels:
 * - view, practice: Always available (not_started+)
 * - comment, suggest_edit: analyze+
 * - peer_review, rate_quality: evaluate+
 * - create_derivative, contribute_path, govern: create
 */
export type PrivilegeType =
  | 'view' // Basic viewing (always granted)
  | 'practice' // Take practice quizzes (always granted)
  | 'comment' // Add comments/discussion (analyze+)
  | 'suggest_edit' // Propose content edits (analyze+)
  | 'peer_review' // Review others' contributions (evaluate+)
  | 'rate_quality' // Rate content quality (evaluate+)
  | 'create_derivative' // Create derivative content (create)
  | 'contribute_path' // Add content to paths (create)
  | 'govern'; // Participate in content governance (create)

/**
 * Map of privileges to minimum required Bloom's level.
 */
export const PRIVILEGE_REQUIREMENTS: Record<PrivilegeType, BloomMasteryLevel> = {
  view: 'not_started',
  practice: 'not_started',
  comment: 'analyze',
  suggest_edit: 'analyze',
  peer_review: 'evaluate',
  rate_quality: 'evaluate',
  create_derivative: 'create',
  contribute_path: 'create',
  govern: 'create',
};

// =========================================================================
// Mastery Statistics & Aggregations
// =========================================================================

/**
 * MasteryStats - Aggregate mastery statistics for a human.
 */
export interface MasteryStats {
  humanId: string;
  computedAt: string;

  /** Total content nodes with any mastery */
  totalMasteredNodes: number;

  /** Distribution by level */
  levelDistribution: Record<BloomMasteryLevel, number>;

  /** Nodes at or above attestation gate */
  nodesAboveGate: number;

  /** Percentage of mastery that is fresh */
  freshPercentage: number;

  /** Nodes needing refresh */
  nodesNeedingRefresh: number;

  /** Stats by content category */
  byCategory: Map<string, CategoryMasteryStats>;

  /** Stats by content type */
  byType: Map<string, TypeMasteryStats>;
}

export interface CategoryMasteryStats {
  category: string;
  nodeCount: number;
  averageLevel: number; // Numeric average of levels
  aboveGateCount: number;
  freshPercentage: number;
}

export interface TypeMasteryStats {
  type: string;
  nodeCount: number;
  averageLevel: number;
  aboveGateCount: number;
  freshPercentage: number;
}

// =========================================================================
// Mastery Snapshot (for embedding in AgentProgress)
// =========================================================================

/**
 * ContentMasterySnapshot - Lightweight mastery summary for embedding.
 *
 * Used in AgentProgress to track mastery for path steps without
 * duplicating the full ContentMastery record.
 */
export interface ContentMasterySnapshot {
  contentId: string;
  level: BloomMasteryLevel;
  freshness: number;
  levelAchievedAt: string;
  hasGatePrivileges: boolean; // Is at or above apply level?
}

// =========================================================================
// Freshness Computation
// =========================================================================

/**
 * FreshnessFactors - Components of freshness calculation.
 */
export interface FreshnessFactors {
  /** Time-based personal decay (0.0-1.0) */
  personalDecay: number;

  /** Graph evolution factor (0.0-1.0) - how much content has changed */
  graphEvolution: number;

  /** Activity factor (0.0-1.0) - engagement on related nodes */
  relatedActivity: number;

  /** Composite freshness score */
  composite: number;
}

/**
 * Freshness thresholds for refresh recommendations.
 */
export const FRESHNESS_THRESHOLDS = {
  /** Above this, mastery is considered fresh */
  FRESH: 0.7,

  /** Between STALE and FRESH, may need review */
  STALE: 0.4,

  /** Below this, significant relearning needed */
  CRITICAL: 0.2,
} as const;

/**
 * Decay rates by mastery level (lambda for exponential decay).
 * Higher levels decay slower when actively used.
 */
export const DECAY_RATES: Record<BloomMasteryLevel, number> = {
  not_started: 0, // No decay
  seen: 0.05, // Fast decay - just viewing
  remember: 0.03, // Moderate decay
  understand: 0.02, // Slower decay
  apply: 0.015, // Even slower - demonstrated application
  analyze: 0.01, // Slow - active engagement
  evaluate: 0.008, // Very slow - ongoing participation
  create: 0.005, // Slowest - creating maintains mastery
};
