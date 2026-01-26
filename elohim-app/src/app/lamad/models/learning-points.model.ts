/**
 * Learning Points Model (Lamad Economy)
 *
 * Learning-specific types that compose Shefa's hREA economic primitives.
 * This demonstrates how domain-specific applications build on the
 * generalizable Shefa substrate.
 *
 * Mapping to Shefa primitives:
 *   Lamad Type              →  Shefa Primitive
 *   ─────────────────────────────────────────────
 *   LamadPointEvent         →  EconomicEvent (learner activity)
 *   LearnerPointBalance     →  EconomicResource (accumulated value)
 *   ContributorRecognition  →  Appreciation (value flow to contributors)
 *   ContributorDashboard    →  Aggregation view over hREA flows
 *
 * Key concepts:
 * - Points earned through learning activities
 * - Recognition flows to content contributors
 * - Triggers map to specific learning events
 *
 * Note: Wire format types (snake_case) are suffixed with "Wire" to distinguish
 * from app-level types in steward-economy.model.ts (camelCase).
 */

import type { ActionHash } from './content-mastery.model';

// =============================================================================
// Holochain Output Wrapper Types
// =============================================================================

/** Output wrapper for learner point balance */
export interface LearnerPointBalanceOutput {
  action_hash: ActionHash;
  balance: LearnerPointBalance;
}

/** Output wrapper for learning point event */
export interface LamadPointEventOutput {
  action_hash: ActionHash;
  event: LamadPointEvent;
}

/** Output wrapper for contributor recognition */
export interface LamadContributorRecognitionOutput {
  action_hash: ActionHash;
  recognition: LamadContributorRecognitionWire;
}

// =============================================================================
// Point Triggers
// =============================================================================

/**
 * Learning-specific triggers that generate Shefa EconomicEvents.
 * Each trigger maps to an hREA action (produce/consume) with a point value.
 */
export const LamadPointTriggers = {
  /** Viewed content (1 pt) */
  ENGAGEMENT_VIEW: 'engagement_view',

  /** Practiced with content (2 pts) */
  ENGAGEMENT_PRACTICE: 'engagement_practice',

  /** Answered challenge question correctly (5 pts) */
  CHALLENGE_CORRECT: 'challenge_correct',

  /** Completed a challenge (10 pts) */
  CHALLENGE_COMPLETE: 'challenge_complete',

  /** Leveled up mastery (20 pts) */
  LEVEL_UP: 'level_up',

  /** Leveled down mastery (-10 pts) */
  LEVEL_DOWN: 'level_down',

  /** Discovered new content via graph (15 pts) */
  DISCOVERY: 'discovery',

  /** Completed a path step (5 pts) */
  PATH_STEP_COMPLETE: 'path_step_complete',

  /** Completed an entire path (100 pts) */
  PATH_COMPLETE: 'path_complete',

  /** Made a contribution (50 pts) */
  CONTRIBUTION: 'contribution',
} as const;

export type LamadPointTrigger = (typeof LamadPointTriggers)[keyof typeof LamadPointTriggers];

/**
 * Default point amounts per learning trigger.
 */
export const LAMAD_POINT_AMOUNTS: Record<LamadPointTrigger, number> = {
  engagement_view: 1,
  engagement_practice: 2,
  challenge_correct: 5,
  challenge_complete: 10,
  level_up: 20,
  level_down: -10,
  discovery: 15,
  path_step_complete: 5,
  path_complete: 100,
  contribution: 50,
};

// =============================================================================
// Recognition Flow Types
// =============================================================================

/**
 * Types of recognition flows from learners to contributors.
 * (Lamad-specific - distinct from generic RecognitionFlowType in contributor-presence)
 */
export const LamadRecognitionFlowTypes = {
  /** Learner engaged with contributor's content */
  CONTENT_ENGAGEMENT: 'content_engagement',

  /** Learner achieved mastery on contributor's content */
  CONTENT_MASTERY: 'content_mastery',

  /** Learner completed a path containing contributor's content */
  PATH_COMPLETION: 'path_completion',

  /** Learner discovered content via contributor's work */
  DISCOVERY_SPARK: 'discovery_spark',
} as const;

export type LamadRecognitionFlowType =
  (typeof LamadRecognitionFlowTypes)[keyof typeof LamadRecognitionFlowTypes];

// =============================================================================
// Learner Point Balance
// =============================================================================

/**
 * Learner's point balance - a learning-specific EconomicResource.
 * Tracks accumulated value from learning activities.
 */
export interface LearnerPointBalance {
  id: string;
  agent_id: string;

  /** Current total points */
  total_points: number;

  /** JSON object mapping trigger to accumulated points */
  points_by_trigger_json: string;

  /** Total points ever earned */
  total_earned: number;

  /** Total points ever spent */
  total_spent: number;

  /** ID of most recent point event */
  last_point_event_id: string | null;

  /** Timestamp of most recent point event (ISO 8601) */
  last_point_event_at: string | null;

  created_at: string;
  updated_at: string;
}

// =============================================================================
// Learning Point Event
// =============================================================================

/**
 * Learning point event - a learning-specific EconomicEvent.
 * Records a learner activity that produced or consumed value.
 */
export interface LamadPointEvent {
  id: string;
  agent_id: string;

  /** hREA action: "produce" or "consume" */
  action: 'produce' | 'consume';

  /** Learning trigger that generated this event */
  trigger: LamadPointTrigger;

  /** Points earned (positive) or spent (negative) */
  points: number;

  /** Content associated with this event */
  content_id: string | null;

  /** Challenge associated with this event */
  challenge_id: string | null;

  /** Path associated with this event */
  path_id: string | null;

  /** Whether the answer was correct (for challenge events) */
  was_correct: boolean | null;

  /** Optional note describing the event */
  note: string | null;

  /** JSON object with additional metadata */
  metadata_json: string;

  /** When this event occurred (ISO 8601) */
  occurred_at: string;
}

// =============================================================================
// Contributor Recognition
// =============================================================================

/**
 * Learning recognition (wire format) - a learning-specific Appreciation.
 * Records value flowing from learner activity to content contributors.
 * Uses snake_case to match Holochain zome output.
 */
export interface LamadContributorRecognitionWire {
  id: string;

  /** ContributorPresence ID (Shefa) */
  contributor_id: string;

  /** Content that triggered this recognition */
  content_id: string;

  /** Learner whose activity triggered this recognition */
  learner_id: string;

  /** References the triggering EconomicEvent */
  appreciation_of_event_id: string;

  /** Type of recognition flow */
  flow_type: LamadRecognitionFlowType;

  /** Recognition points awarded to contributor */
  recognition_points: number;

  /** Path context (if applicable) */
  path_id: string | null;

  /** Challenge context (if applicable) */
  challenge_id: string | null;

  /** Optional note */
  note: string | null;

  /** JSON object with additional metadata */
  metadata_json: string;

  /** When this recognition occurred (ISO 8601) */
  occurred_at: string;
}

// =============================================================================
// Contributor Dashboard Types
// =============================================================================

/**
 * Aggregate learning impact for a contributor (wire format).
 * Uses snake_case to match Holochain zome output.
 */
export interface LamadContributorImpactWire {
  id: string;
  contributor_id: string;
  total_recognition_points: number;
  total_learners_reached: number;
  total_content_mastered: number;
  total_discoveries_sparked: number;
  unique_content_engaged: number;

  /** JSON object mapping content_id to impact details */
  impact_by_content_json: string;

  first_recognition_at: string;
  last_recognition_at: string;
  created_at: string;
  updated_at: string;
}

/** Learning impact summary per content piece (wire format) */
export interface LamadContentImpactSummaryWire {
  content_id: string;
  recognition_points: number;
  learners_reached: number;
  mastery_count: number;
}

/** Recent learning recognition event for timeline display (wire format) */
export interface LamadRecognitionEventSummaryWire {
  learner_id: string;
  content_id: string;
  flow_type: LamadRecognitionFlowType;
  recognition_points: number;
  occurred_at: string;
}

/**
 * Learning Contributor Dashboard (wire format) - aggregated view of hREA value flows.
 * Shows the impact of a contributor's work as recognition flows from learners.
 * Uses snake_case to match Holochain zome output.
 */
export interface LamadContributorDashboardWire {
  contributor_id: string;
  total_recognition_points: number;
  total_learners_reached: number;
  total_content_mastered: number;
  total_discoveries_sparked: number;
  impact_by_content: LamadContentImpactSummaryWire[];
  recent_events: LamadRecognitionEventSummaryWire[];
  impact: LamadContributorImpactWire | null;
}

// =============================================================================
// Input/Output Types for Zome Calls
// =============================================================================

/** Input for earning learning points */
export interface EarnLamadPointsInput {
  trigger: LamadPointTrigger;
  content_id?: string;
  challenge_id?: string;
  path_id?: string;
  was_correct?: boolean;
  note?: string;
}

/** Result of earning points - includes recognition flow via Shefa */
export interface EarnLamadPointsResult {
  point_event: LamadPointEventOutput;
  new_balance: LearnerPointBalanceOutput;

  /** Recognition sent to content contributors */
  recognition_sent: LamadContributorRecognitionOutput[];

  /** Points earned from this action */
  points_earned: number;
}

// =============================================================================
// Helper Functions
// =============================================================================

/**
 * Parse points by trigger from JSON string.
 */
export function parsePointsByTrigger(json: string): Record<LamadPointTrigger, number> {
  try {
    return JSON.parse(json) as Record<LamadPointTrigger, number>;
  } catch {
    console.warn('[LearningPoints] Failed to parse points by trigger JSON');
    return {} as Record<LamadPointTrigger, number>;
  }
}

/**
 * Parse impact by content from JSON string.
 */
export function parseImpactByContent(json: string): Record<string, LamadContentImpactSummaryWire> {
  try {
    return JSON.parse(json) as Record<string, LamadContentImpactSummaryWire>;
  } catch {
    console.warn('[LearningPoints] Failed to parse impact by content JSON');
    return {};
  }
}

/**
 * Get point amount for a trigger.
 */
export function getPointAmount(trigger: LamadPointTrigger): number {
  return LAMAD_POINT_AMOUNTS[trigger] ?? 0;
}

/**
 * Check if a trigger earns positive points.
 */
export function isPositiveTrigger(trigger: LamadPointTrigger): boolean {
  return getPointAmount(trigger) > 0;
}

/**
 * Format points with sign for display.
 */
export function formatPoints(points: number): string {
  if (points > 0) return `+${points}`;
  return points.toString();
}

/**
 * Get display label for a trigger.
 */
export function getTriggerLabel(trigger: LamadPointTrigger): string {
  const labels: Record<LamadPointTrigger, string> = {
    engagement_view: 'Viewed Content',
    engagement_practice: 'Practiced',
    challenge_correct: 'Correct Answer',
    challenge_complete: 'Challenge Complete',
    level_up: 'Level Up',
    level_down: 'Level Down',
    discovery: 'Discovery',
    path_step_complete: 'Step Complete',
    path_complete: 'Path Complete',
    contribution: 'Contribution',
  };
  return labels[trigger] ?? trigger;
}
