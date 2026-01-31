/**
 * Content Mastery Model - Bloom's Taxonomy based mastery tracking.
 *
 * This model tracks a human's mastery state for specific content nodes,
 * implementing Bloom's Taxonomy progression from passive consumption
 * to active contribution.
 *
 * Key concepts:
 * - MasteryLevel: 8-level progression (not_started → create)
 * - Attestation Gate: "apply" level unlocks participation privileges
 * - Freshness: Mastery decays based on time and graph evolution
 * - Expertise Discovery: Graph reveals who knows what
 *
 * Reference: Anderson & Krathwohl (2001), Bloom's Revised Taxonomy
 */

import type { MasteryLevel } from '@app/elohim/models/agent.model';

// @coverage: 30.0% (2026-01-31)

// Re-export for convenience
export type { MasteryLevel } from '@app/elohim/models/agent.model';
export { MASTERY_LEVEL_VALUES, ATTESTATION_GATE_LEVEL } from '@app/elohim/models/agent.model';

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

  /** Current mastery level achieved (Bloom's taxonomy) */
  level: MasteryLevel;

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
  fromLevel: MasteryLevel;
  toLevel: MasteryLevel;
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
  contributesToLevel: MasteryLevel;
}

/**
 * PeerEvaluation - Evaluation from another human at evaluate+ level.
 */
export interface PeerEvaluation {
  evaluatorId: string;
  evaluatorLevel: MasteryLevel;
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
  grantedByLevel: MasteryLevel;
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
export const PRIVILEGE_REQUIREMENTS: Record<PrivilegeType, MasteryLevel> = {
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
  levelDistribution: Record<MasteryLevel, number>;

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
  level: MasteryLevel;
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
export const DECAY_RATES: Record<MasteryLevel, number> = {
  not_started: 0, // No decay
  seen: 0.05, // Fast decay - just viewing
  remember: 0.03, // Moderate decay
  understand: 0.02, // Slower decay
  apply: 0.015, // Even slower - demonstrated application
  analyze: 0.01, // Slow - active engagement
  evaluate: 0.008, // Very slow - ongoing participation
  create: 0.005, // Slowest - creating maintains mastery
};

// =============================================================================
// Holochain Zome Types
// =============================================================================
// Types for communication with the Holochain content_store zome.
// These mirror the Rust types in content_store_integrity.

/** Generic action hash from Holochain */
export type ActionHash = Uint8Array;

/**
 * ContentMastery wire format (snake_case for zome communication).
 * Use ContentMastery for app logic.
 */
export interface ContentMasteryWire {
  id: string;
  human_id: string;
  content_id: string;
  mastery_level: string;
  mastery_level_index: number;
  freshness_score: number;
  needs_refresh: boolean;
  engagement_count: number;
  last_engagement_type: string;
  last_engagement_at: string;
  level_achieved_at: string;
  content_version_at_mastery: string | null;
  assessment_evidence_json: string;
  privileges_json: string;
  created_at: string;
  updated_at: string;
}

/** Output wrapper for content mastery from zome */
export interface ContentMasteryOutput {
  action_hash: ActionHash;
  mastery: ContentMasteryWire;
}

/** Input for initializing mastery tracking */
export interface InitializeMasteryInput {
  content_id: string;
}

/** Input for recording engagement */
export interface RecordEngagementInput {
  content_id: string;
  engagement_type: string;
  duration_seconds?: number;
  metadata_json?: string;
}

/** Input for recording assessment (quiz/test) */
export interface RecordAssessmentInput {
  content_id: string;
  assessment_type: string;
  score: number;
  passing_threshold: number;
  time_spent_seconds: number;
  question_count: number;
  correct_count: number;
  evidence_json?: string;
}

/** Input for querying mastery */
export interface QueryMasteryInput {
  human_id: string;
  content_ids?: string[];
  min_level?: string;
}

/** Input for checking privilege access */
export interface CheckPrivilegeInput {
  content_id: string;
  privilege: string;
}

/** Result of privilege check */
export interface PrivilegeCheckResult {
  has_privilege: boolean;
  current_level: string;
  current_level_index: number;
  required_level: string;
  required_level_index: number;
  gap: number;
  privilege: string;
}

/** Compact mastery snapshot for missions view (Khan Academy style) */
export interface MasterySnapshot {
  content_id: string;
  level: string;
  level_index: number;
  freshness: number;
  needs_refresh: boolean;
}

/** Path mastery overview for missions view */
export interface PathMasteryOverview {
  path_id: string;
  path_title: string;
  total_content: number;
  mastery_snapshots: MasterySnapshot[];
  level_counts: Record<string, number>;
  completion_percentage: number;
  mastery_percentage: number;
}

/** Mastery statistics dashboard (wire format) */
export interface MasteryStatsWire {
  total_tracked: number;
  level_distribution: Record<string, number>;
  above_gate_count: number;
  fresh_count: number;
  stale_count: number;
  needs_refresh_count: number;
}

// =============================================================================
// Transformation Helpers
// =============================================================================

/**
 * Transform wire format to app ContentMastery.
 */
export function transformMasteryFromWire(wire: ContentMasteryWire): ContentMastery {
  let assessmentEvidence: AssessmentEvidence[] = [];
  try {
    assessmentEvidence = JSON.parse(wire.assessment_evidence_json || '[]');
  } catch {
    // Ignore parse errors
  }

  let privileges: ContentPrivilege[] = [];
  try {
    privileges = JSON.parse(wire.privileges_json || '[]');
  } catch {
    // Ignore parse errors
  }

  return {
    contentId: wire.content_id,
    humanId: wire.human_id,
    level: wire.mastery_level as MasteryLevel,
    levelAchievedAt: wire.level_achieved_at,
    levelHistory: [], // Not stored in wire format
    lastEngagementAt: wire.last_engagement_at,
    lastEngagementType: wire.last_engagement_type as EngagementType,
    contentVersionAtMastery: wire.content_version_at_mastery ?? '',
    freshness: wire.freshness_score,
    needsRefresh: wire.needs_refresh,
    assessmentEvidence,
    privileges,
  };
}
