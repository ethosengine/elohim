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
 * Wire types (snake_case, *_json fields) are defined in @app/elohim/models/zome-wire-types
 * and re-exported here for backwards compatibility.
 */

// @coverage: 53.3% (2026-02-24)

// =============================================================================
// Re-exported Wire Types from centralized zome-wire-types
// =============================================================================

export type {
  ActionHash,
  LearnerPointBalance,
  LearnerPointBalanceOutput,
  LamadPointEvent,
  LamadPointEventOutput,
  LamadContributorRecognitionWire,
  LamadContributorRecognitionOutput,
  LamadContributorImpactWire,
  LamadContentImpactSummaryWire,
  LamadRecognitionEventSummaryWire,
  LamadContributorDashboardWire,
  EarnLamadPointsInput,
  EarnLamadPointsResult,
} from '@app/elohim/models/zome-wire-types';

export { parsePointsByTrigger, parseImpactByContent } from '@app/elohim/models/zome-wire-types';

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
// Helper Functions
// =============================================================================

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
