/**
 * Zome Wire Types - Centralized Holochain zome response types.
 *
 * All types in this file represent the WIRE FORMAT from Holochain zome calls:
 * - snake_case field names (matching Rust structs)
 * - `*_json: string` fields that need JSON.parse
 * - ActionHash references as Uint8Array
 *
 * These are NOT the same as the HTTP API types from elohim-storage (which
 * arrive as camelCase via doorway). These are direct zome call responses
 * via the Holochain conductor WebSocket.
 *
 * Usage:
 * 1. LearnerBackendService calls zome → receives wire type
 * 2. Call transformZomeResponse() at the boundary
 * 3. Domain services receive clean camelCase objects with parsed JSON
 */

// =============================================================================
// Generic ActionHash
// =============================================================================

/** Generic action hash from Holochain */
export type ActionHash = Uint8Array;

// =============================================================================
// Content Mastery Wire Types
// =============================================================================

/**
 * ContentMastery wire format from content_store zome.
 * Has assessment_evidence_json and privileges_json that need parsing.
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

// =============================================================================
// Practice Pool Wire Types
// =============================================================================

/**
 * Practice Pool wire format from content_store zome.
 * Has 4 `*_json` fields: active_content_ids, refresh_queue_ids,
 * discovery_candidates, contributing_path_ids.
 */
export interface PracticePool {
  id: string;
  agent_id: string;
  active_content_ids_json: string;
  refresh_queue_ids_json: string;
  discovery_candidates_json: string;
  contributing_path_ids_json: string;
  max_active_size: number;
  refresh_threshold: number;
  discovery_probability: number;
  regression_enabled: boolean;
  challenge_cooldown_hours: number;
  last_challenge_at: string | null;
  last_challenge_id: string | null;
  total_challenges_taken: number;
  total_level_ups: number;
  total_level_downs: number;
  discoveries_unlocked: number;
  created_at: string;
  updated_at: string;
}

/** Output wrapper for practice pool */
export interface PracticePoolOutput {
  action_hash: ActionHash;
  pool: PracticePool;
}

/**
 * Mastery Challenge wire format from content_store zome.
 * Has 6 `*_json` fields: content_mix, questions, responses,
 * score_by_content, level_changes, discoveries.
 */
export interface MasteryChallenge {
  id: string;
  agent_id: string;
  pool_id: string;
  path_id: string | null;
  content_mix_json: string;
  total_questions: number;
  discovery_questions: number;
  state: string;
  started_at: string;
  completed_at: string | null;
  time_limit_seconds: number | null;
  actual_time_seconds: number | null;
  questions_json: string;
  responses_json: string;
  score: number | null;
  score_by_content_json: string;
  level_changes_json: string;
  net_level_change: number;
  discoveries_json: string;
  created_at: string;
}

/** Output wrapper for mastery challenge */
export interface MasteryChallengeOutput {
  action_hash: ActionHash;
  challenge: MasteryChallenge;
}

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
  from_level: string;
  to_level: string;
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

/** Pool content sources */
export const PoolSources = {
  PATH_ACTIVE: 'path_active',
  REFRESH_QUEUE: 'refresh_queue',
  GRAPH_NEIGHBOR: 'graph_neighbor',
  SERENDIPITY: 'serendipity',
} as const;

export type PoolSource = (typeof PoolSources)[keyof typeof PoolSources];

/**
 * Mastery challenge states.
 * (Distinct from governance ChallengeState in governance-feedback.model)
 */
export const MasteryChallengeStates = {
  IN_PROGRESS: 'in_progress',
  COMPLETED: 'completed',
  ABANDONED: 'abandoned',
} as const;

export type MasteryChallengeState =
  (typeof MasteryChallengeStates)[keyof typeof MasteryChallengeStates];

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
// Learning Points Wire Types
// =============================================================================

/**
 * Learner's point balance wire format from content_store zome.
 * Has points_by_trigger_json that needs parsing.
 */
export interface LearnerPointBalance {
  id: string;
  agent_id: string;
  total_points: number;
  points_by_trigger_json: string;
  total_earned: number;
  total_spent: number;
  last_point_event_id: string | null;
  last_point_event_at: string | null;
  created_at: string;
  updated_at: string;
}

/** Output wrapper for learner point balance */
export interface LearnerPointBalanceOutput {
  action_hash: ActionHash;
  balance: LearnerPointBalance;
}

/**
 * Learning point event wire format.
 * Has metadata_json that needs parsing.
 */
export interface LamadPointEvent {
  id: string;
  agent_id: string;
  action: 'produce' | 'consume';
  trigger: string;
  points: number;
  content_id: string | null;
  challenge_id: string | null;
  path_id: string | null;
  was_correct: boolean | null;
  note: string | null;
  metadata_json: string;
  occurred_at: string;
}

/** Output wrapper for learning point event */
export interface LamadPointEventOutput {
  action_hash: ActionHash;
  event: LamadPointEvent;
}

/**
 * Learning recognition wire format.
 * Has metadata_json that needs parsing.
 */
export interface LamadContributorRecognitionWire {
  id: string;
  contributor_id: string;
  content_id: string;
  learner_id: string;
  appreciation_of_event_id: string;
  flow_type: string;
  recognition_points: number;
  path_id: string | null;
  challenge_id: string | null;
  note: string | null;
  metadata_json: string;
  occurred_at: string;
}

/** Output wrapper for contributor recognition */
export interface LamadContributorRecognitionOutput {
  action_hash: ActionHash;
  recognition: LamadContributorRecognitionWire;
}

/**
 * Aggregate learning impact for a contributor (wire format).
 * Has impact_by_content_json that needs parsing.
 */
export interface LamadContributorImpactWire {
  id: string;
  contributor_id: string;
  total_recognition_points: number;
  total_learners_reached: number;
  total_content_mastered: number;
  total_discoveries_sparked: number;
  unique_content_engaged: number;
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
  flow_type: string;
  recognition_points: number;
  occurred_at: string;
}

/**
 * Learning Contributor Dashboard (wire format).
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

/** Input for earning learning points */
export interface EarnLamadPointsInput {
  trigger: string;
  content_id?: string;
  challenge_id?: string;
  path_id?: string;
  was_correct?: boolean;
  note?: string;
}

/** Result of earning points */
export interface EarnLamadPointsResult {
  point_event: LamadPointEventOutput;
  new_balance: LearnerPointBalanceOutput;
  recognition_sent: LamadContributorRecognitionOutput[];
  points_earned: number;
}

// =============================================================================
// Transform Function
// =============================================================================

/**
 * Transform a zome wire response: snake_case → camelCase, JSON.parse `*_json` fields.
 *
 * This is the ONE place where zome responses are cleaned up. Call it at the
 * boundary (LearnerBackendService) immediately after receiving zome data.
 *
 * Handles:
 * - snake_case keys → camelCase keys
 * - `*_json` string fields → parsed JSON (with `Json` suffix removed from key)
 * - Nested objects (recursively)
 * - Arrays (element-wise)
 * - null/undefined pass-through
 */
export function transformZomeResponse<T>(raw: unknown): T {
  if (raw === null || raw === undefined) {
    return raw as T;
  }

  if (Array.isArray(raw)) {
    return raw.map(item => transformZomeResponse(item)) as T;
  }

  if (typeof raw !== 'object') {
    return raw as T;
  }

  // Don't transform Uint8Array (ActionHash) or other typed arrays
  if (raw instanceof Uint8Array || ArrayBuffer.isView(raw)) {
    return raw as T;
  }

  return transformObject(raw as Record<string, unknown>) as T;
}

function transformObject(raw: Record<string, unknown>): Record<string, unknown> {
  const result: Record<string, unknown> = {};

  for (const [key, value] of Object.entries(raw)) {
    if (key.endsWith('_json') && typeof value === 'string') {
      const camelKey = snakeToCamel(key.replace(/_json$/, ''));
      result[camelKey] = parseJsonField(value);
    } else {
      result[snakeToCamel(key)] = transformZomeResponse(value);
    }
  }

  return result;
}

function parseJsonField(value: string): unknown {
  try {
    return JSON.parse(value || '[]');
  } catch {
    return value === '' ? [] : value;
  }
}

/**
 * Convert snake_case string to camelCase.
 */
function snakeToCamel(s: string): string {
  return s.replace(/_([a-z])/g, (_, c: string) => c.toUpperCase());
}

// =============================================================================
// Per-Type Parse Helpers (for consuming wire types without full transform)
// =============================================================================

/** Parse content mix from JSON string */
export function parseContentMix(json: string): ContentMixEntry[] {
  try {
    return JSON.parse(json) as ContentMixEntry[];
  } catch {
    return [];
  }
}

/** Parse level changes from JSON string */
export function parseLevelChanges(json: string): LevelChange[] {
  try {
    return JSON.parse(json) as LevelChange[];
  } catch {
    return [];
  }
}

/** Parse questions from JSON string */
export function parseQuestions(json: string): ChallengeQuestion[] {
  try {
    return JSON.parse(json) as ChallengeQuestion[];
  } catch {
    return [];
  }
}

/** Parse responses from JSON string */
export function parseResponses(json: string): MasteryChallengeResponse[] {
  try {
    return JSON.parse(json) as MasteryChallengeResponse[];
  } catch {
    return [];
  }
}

/** Parse discoveries from JSON string */
export function parseDiscoveries(json: string): ChallengeDiscovery[] {
  try {
    return JSON.parse(json) as ChallengeDiscovery[];
  } catch {
    return [];
  }
}

/** Get active content IDs from pool */
export function getActiveContentIds(pool: PracticePool): string[] {
  try {
    return JSON.parse(pool.active_content_ids_json) as string[];
  } catch {
    return [];
  }
}

/** Get refresh queue IDs from pool */
export function getRefreshQueueIds(pool: PracticePool): string[] {
  try {
    return JSON.parse(pool.refresh_queue_ids_json) as string[];
  } catch {
    return [];
  }
}

/** Get discovery candidates from pool */
export function getDiscoveryCandidates(pool: PracticePool): DiscoveryCandidate[] {
  try {
    return JSON.parse(pool.discovery_candidates_json) as DiscoveryCandidate[];
  } catch {
    return [];
  }
}

/** Parse points by trigger from JSON string */
export function parsePointsByTrigger(json: string): Record<string, number> {
  try {
    return JSON.parse(json) as Record<string, number>;
  } catch {
    return {};
  }
}

/** Parse impact by content from JSON string */
export function parseImpactByContent(json: string): Record<string, LamadContentImpactSummaryWire> {
  try {
    return JSON.parse(json) as Record<string, LamadContentImpactSummaryWire>;
  } catch {
    return {};
  }
}
