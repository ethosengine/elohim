/**
 * Practice Pool & Mastery Challenge Models
 *
 * Khan Academy-style practice system types for organic learning.
 * The practice pool manages a rotating set of content for spaced repetition,
 * and mastery challenges test knowledge with potential level up/down.
 *
 * Key concepts:
 * - Practice Pool: Agent's learning rotation with graph-aware discovery
 * - Mastery Challenge: Mixed assessment that can change mastery levels
 * - Discovery: New content unlocked through graph exploration
 *
 * These types mirror the Holochain zome types in content_store_integrity.
 */

import type { MasteryLevel, ActionHash } from './content-mastery.model';

// =============================================================================
// Holochain Output Wrapper Types
// =============================================================================

/** Output wrapper for practice pool */
export interface PracticePoolOutput {
  action_hash: ActionHash;
  pool: PracticePool;
}

/** Output wrapper for mastery challenge */
export interface MasteryChallengeOutput {
  action_hash: ActionHash;
  challenge: MasteryChallenge;
}

// =============================================================================
// Pool Source Types
// =============================================================================

/** Pool content sources */
export const PoolSources = {
  PATH_ACTIVE: 'path_active',
  REFRESH_QUEUE: 'refresh_queue',
  GRAPH_NEIGHBOR: 'graph_neighbor',
  SERENDIPITY: 'serendipity',
} as const;

export type PoolSource = typeof PoolSources[keyof typeof PoolSources];

// =============================================================================
// Challenge State Types
// =============================================================================

/**
 * Mastery challenge states.
 * (Distinct from governance ChallengeState in governance-feedback.model)
 */
export const MasteryChallengeStates = {
  IN_PROGRESS: 'in_progress',
  COMPLETED: 'completed',
  ABANDONED: 'abandoned',
} as const;

export type MasteryChallengeState = typeof MasteryChallengeStates[keyof typeof MasteryChallengeStates];

// =============================================================================
// Practice Pool
// =============================================================================

/**
 * Practice Pool - agent's learning rotation with graph-aware discovery.
 *
 * The pool maintains:
 * - Active content: Current focus areas from enrolled paths
 * - Refresh queue: Stale content needing spaced repetition
 * - Discovery candidates: Related content from knowledge graph
 */
export interface PracticePool {
  id: string;
  agent_id: string;

  /** JSON array of content IDs in active rotation */
  active_content_ids_json: string;

  /** JSON array of content IDs needing refresh */
  refresh_queue_ids_json: string;

  /** JSON array of DiscoveryCandidate objects */
  discovery_candidates_json: string;

  /** JSON array of path IDs contributing to this pool */
  contributing_path_ids_json: string;

  /** Maximum items in active rotation */
  max_active_size: number;

  /** Freshness threshold (0.0-1.0) below which content goes to refresh queue */
  refresh_threshold: number;

  /** Probability (0.0-1.0) of including discovery content in challenges */
  discovery_probability: number;

  /** Whether mastery levels can decrease */
  regression_enabled: boolean;

  /** Hours between challenge attempts */
  challenge_cooldown_hours: number;

  /** Timestamp of last challenge (ISO 8601) */
  last_challenge_at: string | null;

  /** ID of last challenge taken */
  last_challenge_id: string | null;

  /** Total challenges taken from this pool */
  total_challenges_taken: number;

  /** Total level ups achieved */
  total_level_ups: number;

  /** Total level downs (regressions) */
  total_level_downs: number;

  /** Total new content discovered */
  discoveries_unlocked: number;

  created_at: string;
  updated_at: string;
}

// =============================================================================
// Mastery Challenge
// =============================================================================

/**
 * Mastery Challenge - mixed assessment with level up/down.
 *
 * Challenges draw content from multiple sources:
 * - Active pool content (practice)
 * - Refresh queue (spaced repetition)
 * - Discovery candidates (exploration)
 *
 * Results can increase or decrease mastery levels based on performance.
 */
export interface MasteryChallenge {
  id: string;
  agent_id: string;
  pool_id: string;
  path_id: string | null;

  /** JSON array of ContentMixEntry objects */
  content_mix_json: string;

  total_questions: number;
  discovery_questions: number;

  /** Challenge state: in_progress, completed, abandoned */
  state: string;

  started_at: string;
  completed_at: string | null;
  time_limit_seconds: number | null;
  actual_time_seconds: number | null;

  /** JSON array of ChallengeQuestion objects */
  questions_json: string;

  /** JSON array of MasteryChallengeResponse objects */
  responses_json: string;

  /** Score as percentage (0.0-1.0) */
  score: number | null;

  /** JSON object mapping content_id to score */
  score_by_content_json: string;

  /** JSON array of LevelChange objects */
  level_changes_json: string;

  /** Net change in mastery levels across all content */
  net_level_change: number;

  /** JSON array of ChallengeDiscovery objects */
  discoveries_json: string;

  created_at: string;
}

// =============================================================================
// Supporting Types
// =============================================================================

/** Discovery candidate from knowledge graph */
export interface DiscoveryCandidate {
  content_id: string;
  source_content_id: string;
  relationship_type: string;
  discovery_reason: string;
}

/** Content mix entry for a challenge */
export interface ContentMixEntry {
  content_id: string;
  source: PoolSource;
  question_count: number;
}

/** Challenge question */
export interface ChallengeQuestion {
  content_id: string;
  question_type: string;
  question_text: string;
  options_json: string;
  correct_answer: string;
}

/**
 * Response to a mastery challenge question.
 * (Distinct from governance ChallengeResponse in governance-feedback.model)
 */
export interface MasteryChallengeResponse {
  content_id: string;
  question_index: number;
  response: string;
  correct: boolean;
  time_taken_ms: number;
}

/** Level change from a challenge */
export interface LevelChange {
  content_id: string;
  from_level: MasteryLevel;
  to_level: MasteryLevel;
  from_index: number;
  to_index: number;
  change: 'up' | 'down' | 'same';
}

/** Discovery made from challenge */
export interface ChallengeDiscovery {
  content_id: string;
  discovered_via: string;
  relationship_type: string;
}

// =============================================================================
// Input/Output Types for Zome Calls
// =============================================================================

/** Input for creating/updating practice pool */
export interface CreatePoolInput {
  contributing_path_ids: string[];
  max_active_size?: number;
  refresh_threshold?: number;
  discovery_probability?: number;
  regression_enabled?: boolean;
  challenge_cooldown_hours?: number;
}

/** Input for starting a mastery challenge */
export interface StartChallengeInput {
  path_id?: string;
  question_count: number;
  include_discoveries: boolean;
  time_limit_seconds?: number;
}

/** Input for submitting challenge responses */
export interface SubmitChallengeInput {
  challenge_id: string;
  responses: MasteryChallengeResponse[];
  actual_time_seconds: number;
}

/** Challenge result after submission */
export interface ChallengeResult {
  challenge: MasteryChallengeOutput;
  score: number;
  level_changes: LevelChange[];
  discoveries: ChallengeDiscovery[];
  net_level_change: number;
  can_retake_at: string;
}

/** Cooldown check result */
export interface CooldownCheckResult {
  can_take_challenge: boolean;
  cooldown_remaining_hours: number;
  last_challenge_at: string | null;
  next_available_at: string | null;
}

/** Pool recommendations for what to practice */
export interface PoolRecommendations {
  priority_refresh: string[];
  active_practice: string[];
  discovery_suggestions: DiscoveryCandidate[];
  total_pool_size: number;
}

// =============================================================================
// Helper Functions
// =============================================================================

/**
 * Parse content mix from JSON string.
 */
export function parseContentMix(json: string): ContentMixEntry[] {
  try {
    return JSON.parse(json) as ContentMixEntry[];
  } catch {
    console.warn('[Practice] Failed to parse content mix JSON');
    return [];
  }
}

/**
 * Parse level changes from JSON string.
 */
export function parseLevelChanges(json: string): LevelChange[] {
  try {
    return JSON.parse(json) as LevelChange[];
  } catch {
    console.warn('[Practice] Failed to parse level changes JSON');
    return [];
  }
}

/**
 * Parse questions from JSON string.
 */
export function parseQuestions(json: string): ChallengeQuestion[] {
  try {
    return JSON.parse(json) as ChallengeQuestion[];
  } catch {
    console.warn('[Practice] Failed to parse questions JSON');
    return [];
  }
}

/**
 * Parse responses from JSON string.
 */
export function parseResponses(json: string): MasteryChallengeResponse[] {
  try {
    return JSON.parse(json) as MasteryChallengeResponse[];
  } catch {
    console.warn('[Practice] Failed to parse responses JSON');
    return [];
  }
}

/**
 * Parse discoveries from JSON string.
 */
export function parseDiscoveries(json: string): ChallengeDiscovery[] {
  try {
    return JSON.parse(json) as ChallengeDiscovery[];
  } catch {
    console.warn('[Practice] Failed to parse discoveries JSON');
    return [];
  }
}

/**
 * Get active content IDs from pool.
 */
export function getActiveContentIds(pool: PracticePool): string[] {
  try {
    return JSON.parse(pool.active_content_ids_json) as string[];
  } catch {
    return [];
  }
}

/**
 * Get refresh queue IDs from pool.
 */
export function getRefreshQueueIds(pool: PracticePool): string[] {
  try {
    return JSON.parse(pool.refresh_queue_ids_json) as string[];
  } catch {
    return [];
  }
}

/**
 * Get discovery candidates from pool.
 */
export function getDiscoveryCandidates(pool: PracticePool): DiscoveryCandidate[] {
  try {
    return JSON.parse(pool.discovery_candidates_json) as DiscoveryCandidate[];
  } catch {
    return [];
  }
}
