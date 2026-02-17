/**
 * Learner Mastery Profile Model - Gamified Dashboard Aggregation
 *
 * Composes mastery + points + practice + streaks into a single
 * gamified learner profile for the dashboard (/lamad/me).
 *
 * Key concepts:
 * - Overall learner level derived from XP (earned points + mastery depth)
 * - Daily engagement streaks tracked on source chain
 * - Level-up events bridge mastery to points economy
 * - Practice summary from challenge pool stats
 */

import type { MasteryLevel } from '@app/elohim/models/agent.model';

// =============================================================================
// Learner Level System
// =============================================================================

/**
 * Definition for a learner level tier.
 */
export interface LearnerLevelDefinition {
  /** Level number (1-8) */
  level: number;

  /** Display label */
  label: string;

  /** Material icon name */
  icon: string;

  /** Minimum XP to reach this level */
  xpThreshold: number;

  /** Display color (hex) */
  color: string;
}

/**
 * 8 learner level tiers.
 * XP thresholds follow a curve rewarding sustained engagement.
 */
export const LEARNER_LEVELS: LearnerLevelDefinition[] = [
  { level: 1, label: 'Newcomer', icon: 'child_care', xpThreshold: 0, color: '#9e9e9e' },
  { level: 2, label: 'Explorer', icon: 'explore', xpThreshold: 100, color: '#ff9800' },
  { level: 3, label: 'Student', icon: 'school', xpThreshold: 500, color: '#ffc107' },
  { level: 4, label: 'Apprentice', icon: 'handyman', xpThreshold: 1500, color: '#4caf50' },
  { level: 5, label: 'Practitioner', icon: 'psychology', xpThreshold: 4000, color: '#03a9f4' },
  { level: 6, label: 'Scholar', icon: 'menu_book', xpThreshold: 8000, color: '#2196f3' },
  { level: 7, label: 'Sage', icon: 'self_improvement', xpThreshold: 15000, color: '#9c27b0' },
  { level: 8, label: 'Creator', icon: 'auto_awesome', xpThreshold: 30000, color: '#e91e63' },
];

/**
 * XP weight per mastery level for computing total mastery XP.
 * Higher Bloom's levels contribute more XP.
 */
export const MASTERY_XP_WEIGHTS: Record<MasteryLevel, number> = {
  not_started: 0,
  seen: 1,
  remember: 3,
  understand: 5,
  apply: 10,
  analyze: 15,
  evaluate: 20,
  create: 30,
};

// =============================================================================
// Learner Level Helpers
// =============================================================================

/**
 * Get the learner level definition for a given XP total.
 */
export function getLearnerLevel(xp: number): LearnerLevelDefinition {
  let result = LEARNER_LEVELS[0];
  for (const level of LEARNER_LEVELS) {
    if (xp >= level.xpThreshold) {
      result = level;
    } else {
      break;
    }
  }
  return result;
}

/**
 * Get progress (0-100) toward the next learner level.
 * Returns 100 if at max level.
 */
export function getLearnerLevelProgress(xp: number): number {
  const current = getLearnerLevel(xp);
  const currentIndex = LEARNER_LEVELS.indexOf(current);

  if (currentIndex >= LEARNER_LEVELS.length - 1) {
    return 100;
  }

  const next = LEARNER_LEVELS[currentIndex + 1];
  const range = next.xpThreshold - current.xpThreshold;
  const progress = xp - current.xpThreshold;

  return Math.min(100, Math.round((progress / range) * 100));
}

// =============================================================================
// Aggregate Profile
// =============================================================================

/**
 * LearnerMasteryProfile - The main aggregate for the gamified dashboard.
 */
export interface LearnerMasteryProfile {
  /** Current learner level definition */
  learnerLevel: LearnerLevelDefinition;

  /** Progress (0-100) toward next level */
  levelProgress: number;

  /** Total XP (earned points + mastery depth XP) */
  totalXP: number;

  /** Total earned learning points */
  earnedPoints: number;

  /** XP from mastery depth (sum of mastery weights) */
  masteryXP: number;

  /** Mastery level distribution */
  levelDistribution: Record<MasteryLevel, number>;

  /** Total content nodes with any mastery */
  totalMasteredNodes: number;

  /** Nodes at or above attestation gate */
  nodesAboveGate: number;

  /** Engagement streak info */
  streak: StreakInfo;

  /** Recent level-up events */
  recentLevelUps: LevelUpEvent[];

  /** Practice summary */
  practice: PracticeSummary;

  /** Paths overview */
  paths: DashboardPathsOverview;

  /** When this profile was computed */
  computedAt: string;
}

// =============================================================================
// Streak Tracking
// =============================================================================

/**
 * StreakInfo - Daily engagement streak state.
 */
export interface StreakInfo {
  /** Current consecutive days of engagement */
  currentStreak: number;

  /** Best streak ever achieved */
  bestStreak: number;

  /** Whether today has been marked as active */
  todayActive: boolean;

  /** Date of most recent activity (YYYY-MM-DD) */
  lastActiveDate: string;

  /** Date the current streak started (YYYY-MM-DD) */
  streakStartDate: string;

  /** Activity map for the last 30 days (YYYY-MM-DD -> true) */
  recentActivity: Record<string, boolean>;
}

/**
 * Content for a 'streak-record' source chain entry.
 */
export interface StreakRecordContent {
  /** Date of activity (YYYY-MM-DD) */
  activeDate: string;

  /** Types of engagement on this day */
  engagementTypes: string[];
}

// =============================================================================
// Level-Up Events
// =============================================================================

/**
 * LevelUpEvent - Records a mastery level increase for timeline and points.
 */
export interface LevelUpEvent {
  /** Entry hash from source chain */
  id: string;

  /** Content node that leveled up */
  contentId: string;

  /** Previous mastery level */
  fromLevel: MasteryLevel;

  /** New mastery level */
  toLevel: MasteryLevel;

  /** When the level-up occurred (ISO 8601) */
  timestamp: string;

  /** Points earned from this level-up */
  pointsEarned: number;

  /** Whether this crossed the attestation gate (apply) */
  isGateLevel: boolean;
}

/**
 * Content for a 'mastery-level-up' source chain entry.
 */
export interface MasteryLevelUpContent {
  contentId: string;
  fromLevel: string;
  toLevel: string;
  pointsEarned: number;
  isGateLevel: boolean;
}

// =============================================================================
// Practice Summary
// =============================================================================

/**
 * PracticeSummary - Aggregated practice/challenge stats.
 */
export interface PracticeSummary {
  /** Total challenges completed */
  totalChallenges: number;

  /** Total level-ups from challenges */
  totalLevelUps: number;

  /** Total level-downs from challenges */
  totalLevelDowns: number;

  /** Total new content discovered */
  totalDiscoveries: number;

  /** Active pool size */
  activePoolSize: number;

  /** Refresh queue size */
  refreshQueueSize: number;
}

// =============================================================================
// Dashboard Paths
// =============================================================================

/**
 * DashboardPathsOverview - Paths section of the dashboard.
 */
export interface DashboardPathsOverview {
  /** Paths currently in progress */
  inProgress: DashboardPathEntry[];

  /** Completed paths */
  completed: DashboardPathEntry[];
}

/**
 * DashboardPathEntry - A path with progress for dashboard display.
 */
export interface DashboardPathEntry {
  /** Path ID */
  pathId: string;

  /** Path title */
  title: string;

  /** Progress percentage (0-100) */
  progressPercent: number;

  /** Completed steps */
  completedSteps: number;

  /** Total steps */
  totalSteps: number;

  /** Last activity on this path (ISO 8601) */
  lastActiveAt: string;
}
