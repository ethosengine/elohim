/**
 * Elohim Holochain SDK Types
 *
 * These types mirror the Rust zome entry types and input/output structs.
 * Keep in sync with:
 *   - zomes/content_store_integrity/src/lib.rs (entry types)
 *   - zomes/content_store/src/lib.rs (input/output types)
 *
 * CODEGEN TODO: Add ts-rs to Rust zome for automatic TypeScript generation:
 *
 *   // In Cargo.toml
 *   [dependencies]
 *   ts-rs = { version = "8.1", features = ["serde-compat"] }
 *
 *   // In lib.rs
 *   use ts_rs::TS;
 *
 *   #[hdk_entry_helper]
 *   #[derive(Clone, PartialEq, TS)]
 *   #[ts(export, export_to = "../sdk/src/generated/")]
 *   pub struct Content { ... }
 *
 *   Then run: cargo test
 *   This generates TypeScript interfaces in sdk/src/generated/
 */

import type { ActionHash, EntryHash } from '@holochain/client';

// Re-export ActionHash for convenience
export type { ActionHash, EntryHash } from '@holochain/client';

// =============================================================================
// Content Types
// =============================================================================

/** Content entry (mirrors Content struct in integrity zome) */
export interface Content {
  id: string;
  content_type: string;
  title: string;
  description: string;
  content: string;
  content_format: string;
  tags: string[];
  source_path: string | null;
  related_node_ids: string[];
  author_id: string | null;
  reach: string;
  trust_score: number;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/** Input for creating content */
export interface CreateContentInput {
  id: string;
  content_type: string;
  title: string;
  description: string;
  content: string;
  content_format: string;
  tags: string[];
  source_path?: string;
  related_node_ids: string[];
  reach: string;
  metadata_json: string;
}

/** Output when retrieving content */
export interface ContentOutput {
  action_hash: ActionHash;
  entry_hash: EntryHash;
  content: Content;
}

/** Input for bulk content creation */
export interface BulkCreateContentInput {
  import_id: string;
  contents: CreateContentInput[];
}

/** Output from bulk content creation */
export interface BulkCreateContentOutput {
  import_id: string;
  created_count: number;
  action_hashes: ActionHash[];
  errors: string[];
}

/** Content statistics */
export interface ContentStats {
  total_count: number;
  by_type: Record<string, number>;
}

/** Input for querying content by ID */
export interface QueryByIdInput {
  id: string;
}

/** Input for querying content by type */
export interface QueryByTypeInput {
  content_type: string;
  limit?: number;
}

// =============================================================================
// Learning Path Types
// =============================================================================

/** Learning path entry */
export interface LearningPath {
  id: string;
  version: string;
  title: string;
  description: string;
  purpose: string | null;
  created_by: string;
  difficulty: string;
  estimated_duration: string | null;
  visibility: string;
  path_type: string;
  tags: string[];
  created_at: string;
  updated_at: string;
}

/** Path step entry - enhanced with learning objectives and attestation gating */
export interface PathStep {
  id: string;
  path_id: string;
  chapter_id: string | null;           // If part of a chapter
  order_index: number;
  step_type: string;                   // content, path, external, checkpoint, reflection
  resource_id: string;
  step_title: string | null;
  step_narrative: string | null;
  is_optional: boolean;
  // Learning objectives and engagement
  learning_objectives_json: string;    // String[] as JSON
  reflection_prompts_json: string;     // String[] as JSON
  practice_exercises_json: string;     // String[] as JSON
  // Completion and gating
  estimated_minutes: number | null;
  completion_criteria: string | null;  // view, quiz_pass, practice_complete, reflection_submit
  attestation_required: string | null; // Required attestation to access this step
  attestation_granted: string | null;  // Attestation granted on completing this step
  mastery_threshold: number | null;    // Minimum mastery level (0-7) to proceed
  // Metadata
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/** Path chapter entry - thematic grouping of steps */
export interface PathChapter {
  id: string;
  path_id: string;
  order_index: number;
  title: string;
  description: string | null;
  learning_objectives_json: string;    // String[] as JSON
  estimated_minutes: number | null;
  is_optional: boolean;
  // Completion tracking
  attestation_granted: string | null;  // Milestone attestation on chapter completion
  mastery_threshold: number | null;    // Required mastery level to complete chapter
  // Metadata
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/** Input for creating a learning path */
export interface CreatePathInput {
  id: string;
  version: string;
  title: string;
  description: string;
  purpose?: string;
  difficulty: string;
  estimated_duration?: string;
  visibility: string;
  path_type: string;
  tags: string[];
}

/** Input for adding a step to a path */
export interface AddPathStepInput {
  path_id: string;
  chapter_id?: string;                 // If part of a chapter
  order_index: number;
  step_type: string;                   // content, path, external, checkpoint, reflection
  resource_id: string;
  step_title?: string;
  step_narrative?: string;
  is_optional: boolean;
  // Learning objectives and engagement
  learning_objectives?: string[];
  reflection_prompts?: string[];
  practice_exercises?: string[];
  // Completion and gating
  estimated_minutes?: number;
  completion_criteria?: string;
  attestation_required?: string;
  attestation_granted?: string;
  mastery_threshold?: number;
  metadata_json?: string;
}

/** Output for path step */
export interface PathStepOutput {
  action_hash: ActionHash;
  step: PathStep;
}

/** Output for learning path with steps */
export interface PathWithSteps {
  action_hash: ActionHash;
  path: LearningPath;
  steps: PathStepOutput[];
}

/** Input for creating a chapter */
export interface CreateChapterInput {
  path_id: string;
  order_index: number;
  title: string;
  description?: string;
  learning_objectives: string[];
  estimated_minutes?: number;
  is_optional: boolean;
  attestation_granted?: string;
  mastery_threshold?: number;
  metadata_json?: string;
}

/** Output for a chapter */
export interface ChapterOutput {
  action_hash: ActionHash;
  chapter: PathChapter;
}

/** Output for a chapter with its steps */
export interface ChapterWithSteps {
  action_hash: ActionHash;
  chapter: PathChapter;
  steps: PathStepOutput[];
}

/** Output for a full path with chapters and steps */
export interface PathWithChaptersAndSteps {
  action_hash: ActionHash;
  path: LearningPath;
  chapters: ChapterWithSteps[];
  ungrouped_steps: PathStepOutput[];  // Steps not in any chapter
}

/** Input for updating a path */
export interface UpdatePathInput {
  path_id: string;
  title?: string;
  description?: string;
  purpose?: string;
  difficulty?: string;
  estimated_duration?: string;
  visibility?: string;
  tags?: string[];
}

/** Input for updating a chapter */
export interface UpdateChapterInput {
  chapter_id: string;
  title?: string;
  description?: string;
  learning_objectives?: string[];
  estimated_minutes?: number;
  is_optional?: boolean;
  order_index?: number;
  attestation_granted?: string;
  mastery_threshold?: number;
}

/** Input for updating a step */
export interface UpdateStepInput {
  step_id: string;
  step_title?: string;
  step_narrative?: string;
  is_optional?: boolean;
  order_index?: number;
  chapter_id?: string;
  learning_objectives?: string[];
  reflection_prompts?: string[];
  practice_exercises?: string[];
  estimated_minutes?: number;
  completion_criteria?: string;
  attestation_required?: string;
  attestation_granted?: string;
  mastery_threshold?: number;
}

/** Path index entry */
export interface PathIndexEntry {
  id: string;
  title: string;
  description: string;
  difficulty: string;
  estimated_duration: string | null;
  step_count: number;
  tags: string[];
}

/** Path index output */
export interface PathIndex {
  paths: PathIndexEntry[];
  total_count: number;
  last_updated: string;
}

// =============================================================================
// Progress Tracking Types
// =============================================================================

/** Input for starting path progress */
export interface StartPathProgressInput {
  path_id: string;
}

/** Output for agent progress */
export interface AgentProgressOutput {
  action_hash: ActionHash;
  progress: AgentProgress;
}

/** Agent progress entry */
export interface AgentProgress {
  id: string;
  agent_id: string;
  path_id: string;
  current_step_index: number;
  completed_step_indices: number[];
  completed_content_ids: string[];
  step_affinity_json: string;         // Record<number, number> as JSON
  step_notes_json: string;            // Record<number, string> as JSON
  reflection_responses_json: string;  // Record<number, string[]> as JSON
  attestations_earned: string[];
  started_at: string;
  last_activity_at: string;
  completed_at: string | null;
}

/** Input for completing a step */
export interface CompleteStepInput {
  path_id: string;
  step_index: number;
  content_id?: string;
  affinity_score?: number;            // 0-10: How much did the learner enjoy this content?
  notes?: string;
  reflection_responses?: string[];
}

/** Summary of a learner's progress */
export interface ProgressSummary {
  path_id: string;
  path_title: string;
  total_steps: number;
  completed_steps: number;
  current_step_index: number;
  is_completed: boolean;
  attestations_earned: string[];
  started_at: string;
  last_activity_at: string;
  completed_at: string | null;
}

/** Input for granting attestation */
export interface GrantAttestationInput {
  path_id: string;
  attestation_id: string;             // The attestation type being granted
  reason: string;                     // e.g., "Completed Chapter 1", "Passed mastery quiz"
  source_type: string;                // "step", "chapter", "path"
  source_id: string;                  // step_id, chapter_id, or path_id
}

/** Input for checking attestation access */
export interface CheckAttestationAccessInput {
  path_id: string;
  required_attestation: string;
}

/** Result of attestation access check */
export interface AttestationAccessResult {
  has_access: boolean;
  required_attestation: string;
  attestations_earned: string[];
}

// =============================================================================
// Relationship Types
// =============================================================================

/** Relationship type constants */
export const RelationshipTypes = {
  RELATES_TO: 'RELATES_TO',
  CONTAINS: 'CONTAINS',
  DEPENDS_ON: 'DEPENDS_ON',
  IMPLEMENTS: 'IMPLEMENTS',
  REFERENCES: 'REFERENCES',
  BELONGS_TO: 'BELONGS_TO',
  DESCRIBES: 'DESCRIBES',
} as const;

export type RelationshipType = typeof RelationshipTypes[keyof typeof RelationshipTypes];

/** Inference source for relationships */
export const InferenceSources = {
  EXPLICIT: 'explicit',
  PATH: 'path',
  TAG: 'tag',
  SEMANTIC: 'semantic',
} as const;

export type InferenceSource = typeof InferenceSources[keyof typeof InferenceSources];

/** Relationship entry */
export interface Relationship {
  id: string;
  source_id: string;
  target_id: string;
  relationship_type: string;
  confidence: number;
  inference_source: string;
  metadata_json: string | null;
  created_at: string;
}

/** Input for creating a relationship */
export interface CreateRelationshipInput {
  source_id: string;
  target_id: string;
  relationship_type: string;
  confidence: number;
  inference_source: string;
  metadata_json?: string;
}

/** Output for relationship */
export interface RelationshipOutput {
  action_hash: ActionHash;
  relationship: Relationship;
}

/** Input for querying relationships */
export interface GetRelationshipsInput {
  content_id: string;
  direction: 'outgoing' | 'incoming' | 'both';
}

/** Input for querying related content */
export interface QueryRelatedContentInput {
  content_id: string;
  relationship_types?: string[];
  depth?: number;
}

/** Content graph node */
export interface ContentGraphNode {
  content: ContentOutput;
  relationship_type: string;
  confidence: number;
  children: ContentGraphNode[];
}

/** Content graph output */
export interface ContentGraph {
  root: ContentOutput | null;
  related: ContentGraphNode[];
  total_nodes: number;
}

// =============================================================================
// Human/Agent Types
// =============================================================================

/** Human entry */
export interface Human {
  id: string;
  display_name: string;
  bio: string | null;
  affinities: string[];
  profile_reach: string;
  location: string | null;
  created_at: string;
  updated_at: string;
}

/** Input for creating a human */
export interface CreateHumanInput {
  id: string;
  display_name: string;
  bio?: string;
  affinities: string[];
  profile_reach: string;
  location?: string;
}

/** Output for human */
export interface HumanOutput {
  action_hash: ActionHash;
  human: Human;
}

/** Input for querying humans by affinity */
export interface QueryHumansByAffinityInput {
  affinities: string[];
  limit?: number;
}

/** Human progress on a learning path */
export interface HumanProgress {
  id: string;
  human_id: string;
  path_id: string;
  current_step_index: number;
  completed_step_indices: number[];
  completed_content_ids: string[];
  started_at: string;
  last_activity_at: string;
  completed_at: string | null;
}

/** Input for recording content completion */
export interface RecordCompletionInput {
  human_id: string;
  path_id: string;
  content_id: string;
}

// =============================================================================
// Human Presence Types (Secure Login)
// =============================================================================

/** Input for registering a human (secure login flow) */
export interface RegisterHumanInput {
  display_name: string;
  bio?: string;
  affinities: string[];
  profile_reach: string;
  location?: string;
  email_hash?: string;
  passkey_credential_id?: string;
  external_identifiers_json: string;
}

/** Output for human session (includes attestations) */
export interface HumanSessionOutput {
  agent_pubkey: string;
  action_hash: ActionHash;
  human: Human;
  session_started_at: string;
  attestations: AttestationOutput[];
}

/** Input for updating human profile */
export interface UpdateHumanInput {
  display_name?: string;
  bio?: string;
  affinities?: string[];
  profile_reach?: string;
  location?: string;
}

/** Input for linking external identity */
export interface LinkExternalIdentityInput {
  provider: string;
  credential_hash: string;
}

// =============================================================================
// Agent Types (Expanded Identity Model)
// =============================================================================

/** Agent types in the network */
export const AgentTypes = {
  HUMAN: 'human',
  ORGANIZATION: 'organization',
  AI_AGENT: 'ai-agent',
  ELOHIM: 'elohim',
} as const;

export type AgentType = typeof AgentTypes[keyof typeof AgentTypes];

/** Agent entry - universal identity for humans, orgs, AI, and Elohim */
export interface Agent {
  id: string;
  agent_type: string;
  display_name: string;
  bio: string | null;
  avatar: string | null;
  affinities: string[];
  visibility: string;
  location: string | null;
  holochain_agent_key: string | null;
  did: string | null;
  activity_pub_type: string | null;
  created_at: string;
  updated_at: string;
}

/** Input for creating an agent */
export interface CreateAgentInput {
  id: string;
  agent_type: string;
  display_name: string;
  bio?: string;
  avatar?: string;
  affinities: string[];
  visibility: string;
  location?: string;
  did?: string;
  activity_pub_type?: string;
}

/** Output for agent */
export interface AgentOutput {
  action_hash: ActionHash;
  agent: Agent;
}

/** Input for querying agents */
export interface QueryAgentsInput {
  agent_type?: string;
  affinities?: string[];
  limit?: number;
}

// =============================================================================
// Agent Presence Types (Elohim Service Layer)
// =============================================================================

/** Elohim scopes - constitutional jurisdiction levels */
export const ElohimScopes = {
  FAMILY: 'family',
  COMMUNITY: 'community',
  GLOBAL: 'global',
} as const;

export type ElohimScope = typeof ElohimScopes[keyof typeof ElohimScopes];

/** Input for registering an Elohim agent */
export interface RegisterElohimInput {
  display_name: string;
  scope: string;
  capabilities_json: string;
  constitutional_commitment: string;
  bio?: string;
}

/** Output for agent session with interceptor capabilities */
export interface AgentSessionOutput {
  agent_pubkey: string;
  action_hash: ActionHash;
  agent: Agent;
  session_started_at: string;
  attestations: AttestationOutput[];
  scope: string | null;
  can_intercept: boolean;
}

/** Input for updating agent state */
export interface UpdateAgentStateInput {
  agent_id: string;
  new_visibility?: string;
  new_affinities?: string[];
  reason?: string;
}

// =============================================================================
// AgentProgress Types
// =============================================================================

/** AgentProgress - private progress on a learning path with enhanced tracking */
export interface AgentProgress {
  id: string;
  agent_id: string;
  path_id: string;
  current_step_index: number;
  completed_step_indices: number[];
  completed_content_ids: string[];
  step_affinity_json: string;
  step_notes_json: string;
  reflection_responses_json: string;
  attestations_earned: string[];
  started_at: string;
  last_activity_at: string;
  completed_at: string | null;
}

/** Input for creating agent progress */
export interface CreateAgentProgressInput {
  agent_id: string;
  path_id: string;
}

/** Output for agent progress */
export interface AgentProgressOutput {
  action_hash: ActionHash;
  progress: AgentProgress;
}

/** Input for updating agent progress */
export interface UpdateAgentProgressInput {
  agent_id: string;
  path_id: string;
  current_step_index?: number;
  completed_step_index?: number;
  completed_content_id?: string;
  step_affinity?: [number, number];
  step_note?: [number, string];
}

// =============================================================================
// ContentMastery Types (Bloom's Taxonomy)
// =============================================================================

/** Mastery levels based on Bloom's Taxonomy */
export const MasteryLevels = {
  NOT_STARTED: 'not_started',
  SEEN: 'seen',
  REMEMBER: 'remember',
  UNDERSTAND: 'understand',
  APPLY: 'apply',         // Attestation gate
  ANALYZE: 'analyze',
  EVALUATE: 'evaluate',
  CREATE: 'create',
} as const;

export type MasteryLevel = typeof MasteryLevels[keyof typeof MasteryLevels];

/** Numeric values for mastery levels */
export const MASTERY_LEVEL_VALUES: Record<MasteryLevel, number> = {
  not_started: 0,
  seen: 1,
  remember: 2,
  understand: 3,
  apply: 4,
  analyze: 5,
  evaluate: 6,
  create: 7,
};

/** Engagement types for mastery tracking */
export const EngagementTypes = {
  VIEW: 'view',
  QUIZ: 'quiz',
  PRACTICE: 'practice',
  COMMENT: 'comment',
  REVIEW: 'review',
  CONTRIBUTE: 'contribute',
  PATH_STEP: 'path_step',
  REFRESH: 'refresh',
} as const;

export type EngagementType = typeof EngagementTypes[keyof typeof EngagementTypes];

/** ContentMastery entry - tracks mastery for a specific content node */
export interface ContentMastery {
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

/** Input for creating/updating content mastery */
export interface UpsertMasteryInput {
  human_id: string;
  content_id: string;
  mastery_level: string;
  engagement_type: string;
}

/** Output for content mastery */
export interface ContentMasteryOutput {
  action_hash: ActionHash;
  mastery: ContentMastery;
}

/** Input for querying mastery */
export interface QueryMasteryInput {
  human_id: string;
  content_ids?: string[];
  min_level?: string;
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
  assessment_type: string;      // recall, comprehension, application, analysis, evaluation, synthesis
  score: number;                // 0.0-1.0
  passing_threshold: number;    // Usually 0.7
  time_spent_seconds: number;
  question_count: number;
  correct_count: number;
  evidence_json?: string;       // Detailed evidence (questions answered, etc.)
}

/** Input for querying mastery by level */
export interface MasteryByLevelQueryInput {
  level?: string;
  needs_refresh?: boolean;
  content_type?: string;
  limit?: number;
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
  completion_percentage: number;    // % at "seen" or above
  mastery_percentage: number;       // % at "apply" or above (past gate)
}

/** Mastery statistics dashboard */
export interface MasteryStats {
  total_tracked: number;
  level_distribution: Record<string, number>;
  above_gate_count: number;
  fresh_count: number;
  stale_count: number;
  needs_refresh_count: number;
}

// =============================================================================
// Assessment History Types
// =============================================================================

/** Assessment history entry (for viewing past attempts) */
export interface AssessmentHistoryEntry {
  assessment_type: string;
  score: number;
  passed: boolean;
  threshold: number;
  question_count: number;
  correct_count: number;
  time_seconds: number;
  timestamp: string;
  level_achieved: string | null;
}

/** Assessment history output */
export interface AssessmentHistory {
  content_id: string;
  entries: AssessmentHistoryEntry[];
  total_attempts: number;
  pass_count: number;
  best_score: number;
  current_level: string;
}

// =============================================================================
// Attestation Gating Types
// =============================================================================

/** Input for checking attestation eligibility */
export interface CheckAttestationEligibilityInput {
  path_id: string;
  attestation_id: string;
  required_content_ids: string[];
  required_mastery_level: string;
}

/** Attestation eligibility result */
export interface AttestationEligibilityResult {
  eligible: boolean;
  attestation_id: string;
  required_level: string;
  required_level_index: number;
  content_requirements: ContentMasteryRequirement[];
  all_requirements_met: boolean;
  missing_requirements: string[];
}

/** Individual content mastery requirement check */
export interface ContentMasteryRequirement {
  content_id: string;
  required_level: string;
  current_level: string;
  met: boolean;
  gap: number;
}

/** Input for granting attestation with mastery check */
export interface GrantAttestationWithMasteryInput {
  path_id: string;
  attestation_id: string;
  reason: string;
  source_type: string;
  source_id: string;
  required_content_ids: string[];
  required_mastery_level: string;
}

/** Result of step access check */
export interface StepAccessResult {
  step_id: string;
  access_granted: boolean;
  blockers: string[];
  attestation_required: string | null;
  mastery_threshold: number | null;
}

// =============================================================================
// Practice Pool Types (Khan Academy-style organic learning)
// =============================================================================

/** Pool content sources */
export const PoolSources = {
  PATH_ACTIVE: 'path_active',
  REFRESH_QUEUE: 'refresh_queue',
  GRAPH_NEIGHBOR: 'graph_neighbor',
  SERENDIPITY: 'serendipity',
} as const;

export type PoolSource = typeof PoolSources[keyof typeof PoolSources];

/** Challenge states */
export const ChallengeStates = {
  IN_PROGRESS: 'in_progress',
  COMPLETED: 'completed',
  ABANDONED: 'abandoned',
} as const;

export type ChallengeState = typeof ChallengeStates[keyof typeof ChallengeStates];

/** Practice Pool - agent's learning rotation with graph-aware discovery */
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

/** Mastery Challenge - mixed assessment with level up/down */
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
  source: string;
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

/** Response to a challenge question */
export interface ChallengeResponse {
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
  change: string;  // "up", "down", "same"
}

/** Discovery made from challenge */
export interface ChallengeDiscovery {
  content_id: string;
  discovered_via: string;
  relationship_type: string;
}

/** Output for practice pool */
export interface PracticePoolOutput {
  action_hash: ActionHash;
  pool: PracticePool;
}

/** Output for mastery challenge */
export interface MasteryChallengeOutput {
  action_hash: ActionHash;
  challenge: MasteryChallenge;
}

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
  responses: ChallengeResponse[];
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
// Attestation Types
// =============================================================================

/** Attestation categories */
export const AttestationCategories = {
  DOMAIN_MASTERY: 'domain-mastery',
  PATH_COMPLETION: 'path-completion',
  ROLE_CREDENTIAL: 'role-credential',
  ACHIEVEMENT: 'achievement',
} as const;

export type AttestationCategory = typeof AttestationCategories[keyof typeof AttestationCategories];

/** Attestation entry - permanent achievement record */
export interface Attestation {
  id: string;
  agent_id: string;
  category: string;
  attestation_type: string;
  display_name: string;
  description: string;
  icon_url: string | null;
  tier: string | null;
  earned_via_json: string;
  issued_at: string;
  issued_by: string;
  expires_at: string | null;
  proof: string | null;
}

/** Input for creating an attestation */
export interface CreateAttestationInput {
  agent_id: string;
  category: string;
  attestation_type: string;
  display_name: string;
  description: string;
  icon_url?: string;
  tier?: string;
  earned_via_json: string;
}

/** Output for attestation */
export interface AttestationOutput {
  action_hash: ActionHash;
  attestation: Attestation;
}

/** Input for querying attestations */
export interface QueryAttestationsInput {
  agent_id?: string;
  category?: string;
  limit?: number;
}

// =============================================================================
// Protocol Constants
// =============================================================================

/** Reach levels - graduated visibility */
export const ReachLevels = {
  PRIVATE: 'private',
  SELF: 'self',
  INTIMATE: 'intimate',
  TRUSTED: 'trusted',
  FAMILIAR: 'familiar',
  COMMUNITY: 'community',
  PUBLIC: 'public',
  COMMONS: 'commons',
} as const;

export type ReachLevel = typeof ReachLevels[keyof typeof ReachLevels];

/** Content types supported in Lamad */
export const ContentTypes = {
  EPIC: 'epic',
  CONCEPT: 'concept',
  SCENARIO: 'scenario',
  ASSESSMENT: 'assessment',
  RESOURCE: 'resource',
  REFLECTION: 'reflection',
  DISCUSSION: 'discussion',
  EXERCISE: 'exercise',
  EXAMPLE: 'example',
  REFERENCE: 'reference',
} as const;

export type ContentType = typeof ContentTypes[keyof typeof ContentTypes];

/** Path visibility types */
export const PathVisibilities = {
  PRIVATE: 'private',
  UNLISTED: 'unlisted',
  COMMUNITY: 'community',
  PUBLIC: 'public',
} as const;

export type PathVisibility = typeof PathVisibilities[keyof typeof PathVisibilities];

// =============================================================================
// Shefa Protocol Constants
// =============================================================================

/** REA Action vocabulary - what happened in an economic event */
export const REAActions = {
  // Input actions
  USE: 'use',
  CONSUME: 'consume',
  CITE: 'cite',
  // Output actions
  PRODUCE: 'produce',
  RAISE: 'raise',
  LOWER: 'lower',
  // Transfer actions
  TRANSFER: 'transfer',
  TRANSFER_CUSTODY: 'transfer-custody',
  TRANSFER_ALL_RIGHTS: 'transfer-all-rights',
  MOVE: 'move',
  // Modification actions
  MODIFY: 'modify',
  COMBINE: 'combine',
  SEPARATE: 'separate',
  // Work actions
  WORK: 'work',
  DELIVER_SERVICE: 'deliver-service',
  // Logistics actions
  PICKUP: 'pickup',
  DROPOFF: 'dropoff',
  // Exchange actions
  GIVE: 'give',
  TAKE: 'take',
  // Acceptance
  ACCEPT: 'accept',
} as const;

export type REAAction = typeof REAActions[keyof typeof REAActions];

/** Resource classifications for Shefa economy */
export const ResourceClassifications = {
  CONTENT: 'content',
  ATTENTION: 'attention',
  RECOGNITION: 'recognition',
  CREDENTIAL: 'credential',
  CURATION: 'curation',
  SYNTHESIS: 'synthesis',
  STEWARDSHIP: 'stewardship',
  MEMBERSHIP: 'membership',
  COMPUTE: 'compute',
  CURRENCY: 'currency',
  // Shefa token classifications
  CARE_TOKEN: 'care-token',
  TIME_TOKEN: 'time-token',
  LEARNING_TOKEN: 'learning-token',
  STEWARD_TOKEN: 'steward-token',
  CREATOR_TOKEN: 'creator-token',
  INFRASTRUCTURE_TOKEN: 'infrastructure-token',
} as const;

export type ResourceClassification = typeof ResourceClassifications[keyof typeof ResourceClassifications];

/** Lamad-specific event types */
export const LamadEventTypes = {
  // Attention Events
  CONTENT_VIEW: 'content-view',
  PATH_STEP_COMPLETE: 'path-step-complete',
  SESSION_START: 'session-start',
  SESSION_END: 'session-end',
  // Recognition Events
  AFFINITY_MARK: 'affinity-mark',
  ENDORSEMENT: 'endorsement',
  CITATION: 'citation',
  // Achievement Events
  PATH_COMPLETE: 'path-complete',
  ATTESTATION_GRANT: 'attestation-grant',
  CAPABILITY_EARN: 'capability-earn',
  // Creation Events
  CONTENT_CREATE: 'content-create',
  PATH_CREATE: 'path-create',
  EXTENSION_CREATE: 'extension-create',
  // Synthesis Events
  MAP_SYNTHESIS: 'map-synthesis',
  ANALYSIS_COMPLETE: 'analysis-complete',
  // Stewardship Events
  STEWARDSHIP_BEGIN: 'stewardship-begin',
  INVITATION_SEND: 'invitation-send',
  PRESENCE_CLAIM: 'presence-claim',
  RECOGNITION_TRANSFER: 'recognition-transfer',
  // Governance Events
  ATTESTATION_REVOKE: 'attestation-revoke',
  CONTENT_FLAG: 'content-flag',
  GOVERNANCE_VOTE: 'governance-vote',
  // Currency Events
  CREDIT_ISSUE: 'credit-issue',
  CREDIT_TRANSFER: 'credit-transfer',
  CREDIT_RETIRE: 'credit-retire',
  // Process Events
  PROCESS_START: 'process-start',
  PROCESS_COMPLETE: 'process-complete',
} as const;

export type LamadEventType = typeof LamadEventTypes[keyof typeof LamadEventTypes];

/** Economic event states */
export const EventStates = {
  PENDING: 'pending',
  VALIDATED: 'validated',
  COUNTERSIGNED: 'countersigned',
  DISPUTED: 'disputed',
  CORRECTED: 'corrected',
} as const;

export type EventState = typeof EventStates[keyof typeof EventStates];

/** Economic resource states */
export const ResourceStates = {
  AVAILABLE: 'available',
  COMMITTED: 'committed',
  IN_USE: 'in-use',
  CONSUMED: 'consumed',
  ARCHIVED: 'archived',
  DISPUTED: 'disputed',
} as const;

export type ResourceState = typeof ResourceStates[keyof typeof ResourceStates];

/** Contributor presence lifecycle states */
export const PresenceStates = {
  UNCLAIMED: 'unclaimed',
  STEWARDED: 'stewarded',
  CLAIMED: 'claimed',
} as const;

export type PresenceState = typeof PresenceStates[keyof typeof PresenceStates];

/** Process states */
export const ProcessStates = {
  NOT_STARTED: 'not-started',
  IN_PROGRESS: 'in-progress',
  FINISHED: 'finished',
  ABANDONED: 'abandoned',
} as const;

export type ProcessState = typeof ProcessStates[keyof typeof ProcessStates];

/** Commitment states */
export const CommitmentStates = {
  PROPOSED: 'proposed',
  ACCEPTED: 'accepted',
  IN_PROGRESS: 'in-progress',
  FULFILLED: 'fulfilled',
  CANCELLED: 'cancelled',
  BREACHED: 'breached',
} as const;

export type CommitmentState = typeof CommitmentStates[keyof typeof CommitmentStates];

/** Claim verification methods */
export const ClaimVerificationMethods = {
  DOMAIN: 'domain-verification',
  SOCIAL: 'social-verification',
  EMAIL: 'email-verification',
  ORCID: 'orcid-verification',
  GITHUB: 'github-verification',
  PUBLISHER: 'publisher-attestation',
  COMMUNITY: 'community-vouching',
  CRYPTOGRAPHIC: 'cryptographic-proof',
} as const;

export type ClaimVerificationMethod = typeof ClaimVerificationMethods[keyof typeof ClaimVerificationMethods];

/** Invitation channels */
export const InvitationChannels = {
  EMAIL: 'email',
  TWITTER: 'twitter',
  GITHUB: 'github',
  WEBSITE_CONTACT: 'website-contact',
  ORCID: 'orcid',
  PUBLISHER: 'publisher',
  COMMUNITY: 'community',
} as const;

export type InvitationChannel = typeof InvitationChannels[keyof typeof InvitationChannels];

/** Value flow governance layers */
export const ValueFlowLayers = {
  DIGNITY_FLOOR: 'dignity_floor',
  ATTRIBUTION: 'attribution',
  CIRCULATION: 'circulation',
  SUSTAINABILITY: 'sustainability',
} as const;

export type ValueFlowLayer = typeof ValueFlowLayers[keyof typeof ValueFlowLayers];

// =============================================================================
// Shefa: Economic Event Types
// =============================================================================

/** EconomicEvent - Immutable record of value flow between agents */
export interface EconomicEvent {
  id: string;
  action: string;
  provider: string;
  receiver: string;
  // Resource identification
  resource_conforms_to: string | null;
  resource_inventoried_as: string | null;
  to_resource_inventoried_as: string | null;
  resource_classified_as_json: string;
  // Quantities
  resource_quantity_value: number | null;
  resource_quantity_unit: string | null;
  effort_quantity_value: number | null;
  effort_quantity_unit: string | null;
  // Timing
  has_point_in_time: string;
  has_duration: string | null;
  // Process context
  input_of: string | null;
  output_of: string | null;
  // Commitment/Agreement context
  fulfills_json: string;
  realization_of: string | null;
  satisfies_json: string;
  in_scope_of_json: string;
  // Metadata
  note: string | null;
  state: string;
  triggered_by: string | null;
  at_location: string | null;
  image: string | null;
  lamad_event_type: string | null;
  metadata_json: string;
  created_at: string;
}

/** Input for creating an economic event */
export interface CreateEconomicEventInput {
  action: string;
  provider: string;
  receiver: string;
  resource_conforms_to?: string;
  resource_classified_as_json?: string;
  resource_quantity_value?: number;
  resource_quantity_unit?: string;
  effort_quantity_value?: number;
  effort_quantity_unit?: string;
  has_point_in_time?: string;
  input_of?: string;
  output_of?: string;
  lamad_event_type?: string;
  note?: string;
  metadata_json?: string;
}

/** Output for economic event */
export interface EconomicEventOutput {
  action_hash: ActionHash;
  event: EconomicEvent;
}

// =============================================================================
// Shefa: Economic Resource Types
// =============================================================================

/** EconomicResource - A resource that can flow through the network */
export interface EconomicResource {
  id: string;
  conforms_to: string;
  name: string;
  note: string | null;
  // Quantities
  accounting_quantity_value: number | null;
  accounting_quantity_unit: string | null;
  onhand_quantity_value: number | null;
  onhand_quantity_unit: string | null;
  // Ownership
  primary_accountable: string;
  custodian: string | null;
  // State
  state: string | null;
  classified_as_json: string;
  // References
  tracking_identifier: string | null;
  image: string | null;
  content_node_id: string | null;
  attestation_id: string | null;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/** Input for creating an economic resource */
export interface CreateEconomicResourceInput {
  conforms_to: string;
  name: string;
  note?: string;
  accounting_quantity_value?: number;
  accounting_quantity_unit?: string;
  primary_accountable: string;
  classified_as_json?: string;
  content_node_id?: string;
  metadata_json?: string;
}

/** Output for economic resource */
export interface EconomicResourceOutput {
  action_hash: ActionHash;
  resource: EconomicResource;
}

// =============================================================================
// Shefa: Contributor Presence Types
// =============================================================================

/** ContributorPresence - Stewardship lifecycle for absent contributors */
export interface ContributorPresence {
  id: string;
  display_name: string;
  presence_state: string;
  external_identifiers_json: string;
  establishing_content_ids_json: string;
  established_at: string;
  // Accumulated recognition
  affinity_total: number;
  unique_engagers: number;
  citation_count: number;
  endorsements_json: string;
  recognition_score: number;
  recognition_by_content_json: string;
  accumulating_since: string;
  last_recognition_at: string;
  // Stewardship
  steward_id: string | null;
  stewardship_started_at: string | null;
  stewardship_commitment_id: string | null;
  stewardship_quality_score: number | null;
  // Claim details
  claim_initiated_at: string | null;
  claim_verified_at: string | null;
  claim_verification_method: string | null;
  claim_evidence_json: string | null;
  claimed_agent_id: string | null;
  claim_recognition_transferred_value: number | null;
  claim_recognition_transferred_unit: string | null;
  claim_facilitated_by: string | null;
  // Invitations
  invitations_json: string;
  // Metadata
  note: string | null;
  image: string | null;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/** Input for creating a contributor presence */
export interface CreateContributorPresenceInput {
  display_name: string;
  external_identifiers_json?: string;
  establishing_content_ids_json?: string;
  note?: string;
  image?: string;
  metadata_json?: string;
}

/** Output for contributor presence */
export interface ContributorPresenceOutput {
  action_hash: ActionHash;
  presence: ContributorPresence;
}

/** Input for querying contributor presences */
export interface QueryPresencesInput {
  presence_state?: string;
  steward_id?: string;
  min_recognition_score?: number;
  limit?: number;
}

/** Input for beginning stewardship of a presence */
export interface BeginStewardshipInput {
  presence_id: string;
  steward_agent_id: string;
  commitment_note?: string;
}

/** Input for initiating a claim on a presence */
export interface InitiateClaimInput {
  presence_id: string;
  claim_evidence_json: string;
  verification_method: string;
}

// =============================================================================
// Shefa: Process Types
// =============================================================================

/** Process - A transformation that creates value */
export interface Process {
  id: string;
  based_on: string | null;
  name: string;
  note: string | null;
  has_beginning: string | null;
  has_end: string | null;
  finished: boolean;
  state: string;
  inputs_json: string;
  outputs_json: string;
  in_scope_of_json: string;
  path_id: string | null;
  performed_by: string | null;
  classified_as_json: string;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/** Input for creating a process */
export interface CreateProcessInput {
  name: string;
  based_on?: string;
  note?: string;
  path_id?: string;
  performed_by?: string;
  classified_as_json?: string;
  metadata_json?: string;
}

/** Output for process */
export interface ProcessOutput {
  action_hash: ActionHash;
  process: Process;
}

// =============================================================================
// Shefa: Intent Types
// =============================================================================

/** Intent - Expression of desired economic activity */
export interface Intent {
  id: string;
  name: string | null;
  action: string;
  provider: string | null;
  receiver: string | null;
  resource_conforms_to: string | null;
  resource_inventoried_as: string | null;
  resource_quantity_value: number | null;
  resource_quantity_unit: string | null;
  effort_quantity_value: number | null;
  effort_quantity_unit: string | null;
  available_quantity_value: number | null;
  available_quantity_unit: string | null;
  has_point_in_time: string | null;
  has_beginning: string | null;
  has_end: string | null;
  due: string | null;
  input_of: string | null;
  output_of: string | null;
  in_scope_of_json: string;
  classified_as_json: string;
  finished: boolean;
  note: string | null;
  image: string | null;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/** Input for creating an intent */
export interface CreateIntentInput {
  action: string;
  name?: string;
  provider?: string;
  receiver?: string;
  resource_conforms_to?: string;
  resource_quantity_value?: number;
  resource_quantity_unit?: string;
  due?: string;
  note?: string;
  metadata_json?: string;
}

/** Output for intent */
export interface IntentOutput {
  action_hash: ActionHash;
  intent: Intent;
}

// =============================================================================
// Shefa: Commitment Types
// =============================================================================

/** Commitment - A promise of future economic activity */
export interface Commitment {
  id: string;
  action: string;
  provider: string;
  receiver: string;
  resource_conforms_to: string | null;
  resource_inventoried_as: string | null;
  resource_classified_as_json: string;
  resource_quantity_value: number | null;
  resource_quantity_unit: string | null;
  effort_quantity_value: number | null;
  effort_quantity_unit: string | null;
  has_point_in_time: string | null;
  has_beginning: string | null;
  has_end: string | null;
  due: string | null;
  clause_of: string | null;
  agreed_in: string | null;
  input_of: string | null;
  output_of: string | null;
  satisfies: string | null;
  in_scope_of_json: string;
  finished: boolean;
  state: string;
  note: string | null;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/** Input for creating a commitment */
export interface CreateCommitmentInput {
  action: string;
  provider: string;
  receiver: string;
  resource_conforms_to?: string;
  resource_quantity_value?: number;
  resource_quantity_unit?: string;
  due?: string;
  satisfies?: string;
  note?: string;
  metadata_json?: string;
}

/** Output for commitment */
export interface CommitmentOutput {
  action_hash: ActionHash;
  commitment: Commitment;
}

// =============================================================================
// Shefa: Appreciation Types
// =============================================================================

/** Appreciation - Recognition of value created by another */
export interface Appreciation {
  id: string;
  appreciation_of: string;
  appreciated_by: string;
  appreciation_to: string;
  quantity_value: number;
  quantity_unit: string;
  note: string | null;
  created_at: string;
}

/** Input for creating an appreciation */
export interface CreateAppreciationInput {
  appreciation_of: string;
  appreciated_by: string;
  appreciation_to: string;
  quantity_value: number;
  quantity_unit: string;
  note?: string;
}

/** Output for appreciation */
export interface AppreciationOutput {
  action_hash: ActionHash;
  appreciation: Appreciation;
}

// =============================================================================
// Shefa: Claim & Settlement Types
// =============================================================================

/** Claim - A claim for a future economic event */
export interface Claim {
  id: string;
  action: string;
  triggered_by: string;
  provider: string | null;
  receiver: string | null;
  resource_conforms_to: string | null;
  resource_classified_as_json: string;
  resource_quantity_value: number | null;
  resource_quantity_unit: string | null;
  effort_quantity_value: number | null;
  effort_quantity_unit: string | null;
  due: string | null;
  agreed_in: string | null;
  in_scope_of_json: string;
  finished: boolean;
  note: string | null;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/** Input for creating a claim */
export interface CreateClaimInput {
  action: string;
  triggered_by: string;
  provider?: string;
  receiver?: string;
  resource_conforms_to?: string;
  resource_quantity_value?: number;
  resource_quantity_unit?: string;
  due?: string;
  note?: string;
  metadata_json?: string;
}

/** Output for claim */
export interface ClaimOutput {
  action_hash: ActionHash;
  claim: Claim;
}

/** Settlement - Records the settlement of a claim */
export interface Settlement {
  id: string;
  settles: string;
  settled_by: string;
  resource_quantity_value: number | null;
  resource_quantity_unit: string | null;
  effort_quantity_value: number | null;
  effort_quantity_unit: string | null;
  note: string | null;
  created_at: string;
}

/** Input for creating a settlement */
export interface CreateSettlementInput {
  settles: string;
  settled_by: string;
  resource_quantity_value?: number;
  resource_quantity_unit?: string;
  note?: string;
}

/** Output for settlement */
export interface SettlementOutput {
  action_hash: ActionHash;
  settlement: Settlement;
}

// =============================================================================
// Shefa: Insurance Mutual Types
// =============================================================================

/** MemberRiskProfile - Behavioral risk assessment for insurance member */
export interface MemberRiskProfile {
  id: string;
  member_id: string;
  risk_type: string;          // health, property, casualty, care
  care_maintenance_score: number;  // 0-100
  community_connectedness_score: number;  // 0-100
  historical_claims_rate: number;  // 0.0-1.0
  risk_score: number;         // Weighted average 0-100
  risk_tier: string;          // low, standard, high, uninsurable
  risk_tier_rationale: string;
  evidence_event_ids_json: string;  // EconomicEvent[] as JSON
  evidence_breakdown_json: string;  // Object as JSON
  risk_trend_direction: string;  // improving, stable, declining
  last_risk_score: number;
  assessed_at: string;
  last_assessment_at: string;
  next_assessment_due: string;
  assessment_event_ids_json: string;
  schema_version: number;
  validation_status: string;  // Valid, Migrated, Degraded, Healing
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/** CoveragePolicy - Coverage definition at graduated governance levels */
export interface CoveragePolicy {
  id: string;
  member_id: string;
  coverage_level: string;     // individual, household, community, network, constitutional
  governed_at: string;        // Qahal entity ID
  covered_risks_json: string; // CoveredRisk[] as JSON
  deductible_value: number | null;
  deductible_unit: string | null;
  coinsurance: number;        // 0-100 percentage
  out_of_pocket_maximum_value: number | null;
  out_of_pocket_maximum_unit: string | null;
  effective_from: string;
  renewal_terms: string;
  renewal_due_at: string;
  constitutional_basis: string;  // Reference to governance document
  last_premium_event_id: string | null;
  last_premium_paid_at: string | null;
  schema_version: number;
  validation_status: string;
  modification_event_ids_json: string;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/** CoveredRisk - Individual risk definition with coverage limits */
export interface CoveredRisk {
  id: string;
  risk_name: string;
  risk_description: string;
  risk_category: string;
  is_covered: boolean;
  coverage_limit_value: number | null;
  coverage_limit_unit: string | null;
  deductible_applies: boolean;
  coinsurance_percent: number;
  prevention_incentive_applies: boolean;
  created_at: string;
}

/** InsuranceClaim - Full claims lifecycle with immutable event trail */
export interface InsuranceClaim {
  id: string;
  claim_number: string;      // CLM-XXXXXXXXXX
  policy_id: string;
  member_id: string;
  filed_date: string;
  filed_by: string;
  loss_type: string;
  loss_date: string;
  description: string;
  estimated_amount_value: number | null;
  estimated_amount_unit: string | null;
  observer_attestation_ids_json: string;
  member_document_ids_json: string;
  status: string;            // filed, adjustment-made, approved, denied, settled, appealed, resolved
  status_history_json: string;
  adjustment_event_ids_json: string;
  appeal_event_ids_json: string;
  settlement_event_ids_json: string;
  schema_version: number;
  validation_status: string;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/** AdjustmentReasoning - Adjuster decisions with Bob Parr Principle (transparency + reasoning) */
export interface AdjustmentReasoning {
  id: string;
  claim_id: string;
  adjuster_id: string;
  coverage_decision: string;  // approved, denied, partial
  approved_amount_value: number | null;
  approved_amount_unit: string | null;
  plain_language_explanation: string;  // Required for member understanding
  interpretation_notes: string | null;
  applied_generosity_principle: boolean;
  constitutional_basis_documents_json: string;
  policy_citations_json: string;
  flagged_for_governance: boolean;
  governance_review_reason: string | null;
  adjustment_date: string;
  created_at: string;
}

/** Output for insurance claim */
export interface InsuranceClaimOutput {
  action_hash: ActionHash;
  claim: InsuranceClaim;
}

/** Input for enrolling member in insurance mutual */
export interface EnrollMemberInput {
  member_id: string;
  risk_type: string;
  coverage_level: string;
  metadata_json?: string;
}

/** Input for filing an insurance claim */
export interface FileClaimInput {
  policy_id: string;
  loss_type: string;
  loss_date: string;
  description: string;
  estimated_amount_value?: number;
  estimated_amount_unit?: string;
  document_ids?: string[];
  metadata_json?: string;
}

/** Input for adjusting a claim */
export interface AdjustClaimInput {
  claim_id: string;
  coverage_decision: string;
  approved_amount_value?: number;
  approved_amount_unit?: string;
  plain_language_explanation: string;
  constitutional_basis?: string;
  flagged_for_governance?: boolean;
  governance_review_reason?: string;
}

// =============================================================================
// Shefa: Requests & Offers Types
// =============================================================================

/** ServiceRequest - Someone is requesting a service (REA Intent) */
export interface ServiceRequest {
  id: string;
  request_number: string;    // REQ-XXXXXXXXXX
  requester_id: string;
  title: string;
  description: string;
  contact_preference: string; // email, phone, message, in-person
  contact_value: string;
  time_zone: string;
  time_preference: string;   // morning, afternoon, evening, any
  interaction_type: string;  // virtual, in-person, hybrid
  date_range_start: string;
  date_range_end: string | null;
  service_type_ids_json: string;  // string[] as JSON
  required_skills_json: string;   // string[] as JSON
  budget_value: number | null;
  budget_unit: string | null;
  medium_of_exchange_ids_json: string;
  status: string;            // pending, active, archived, deleted
  is_public: boolean;
  links_json: string;
  schema_version: number;
  validation_status: string;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/** ServiceOffer - Someone is offering a service (REA Intent) */
export interface ServiceOffer {
  id: string;
  offer_number: string;      // OFR-XXXXXXXXXX
  offeror_id: string;
  title: string;
  description: string;
  contact_preference: string;
  contact_value: string;
  time_zone: string;
  time_preference: string;
  interaction_type: string;
  hours_per_week: number;
  date_range_start: string;
  date_range_end: string | null;
  service_type_ids_json: string;
  offered_skills_json: string;
  rate_value: number | null;
  rate_unit: string | null;
  rate_per: string;          // hour, day, week, project, etc.
  medium_of_exchange_ids_json: string;
  accepts_alternative_payment: boolean;
  status: string;            // pending, active, archived, deleted
  is_public: boolean;
  links_json: string;
  schema_version: number;
  validation_status: string;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/** ServiceMatch - Request + Offer pairing with compatibility scoring */
export interface ServiceMatch {
  id: string;
  request_id: string;
  offer_id: string;
  match_reason: string;
  match_quality: number;     // 0-100 score
  shared_service_types_json: string;
  time_compatible: boolean;
  interaction_compatible: boolean;
  exchange_compatible: boolean;
  status: string;            // suggested → contacted → negotiating → agreed → completed
  proposal_id: string | null; // REA Proposal
  commitment_id: string | null;  // REA Commitment
  schema_version: number;
  validation_status: string;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/** Output for service request */
export interface ServiceRequestOutput {
  action_hash: ActionHash;
  request: ServiceRequest;
}

/** Output for service offer */
export interface ServiceOfferOutput {
  action_hash: ActionHash;
  offer: ServiceOffer;
}

/** Output for service match */
export interface ServiceMatchOutput {
  action_hash: ActionHash;
  match: ServiceMatch;
}

/** Input for creating a service request */
export interface CreateServiceRequestInput {
  title: string;
  description: string;
  service_type_ids: string[];
  contact_preference: string;
  contact_value: string;
  time_zone: string;
  time_preference: string;
  interaction_type: string;
  date_range_start: string;
  date_range_end?: string;
  required_skills?: string[];
  budget_value?: number;
  budget_unit?: string;
  medium_of_exchange_ids?: string[];
  links?: string[];
  metadata_json?: string;
}

/** Input for creating a service offer */
export interface CreateServiceOfferInput {
  title: string;
  description: string;
  service_type_ids: string[];
  contact_preference: string;
  contact_value: string;
  time_zone: string;
  time_preference: string;
  interaction_type: string;
  hours_per_week: number;
  date_range_start: string;
  date_range_end?: string;
  offered_skills?: string[];
  rate_value?: number;
  rate_unit?: string;
  rate_per?: string;
  medium_of_exchange_ids?: string[];
  accepts_alternative_payment?: boolean;
  links?: string[];
  metadata_json?: string;
}

/** Input for creating a service match */
export interface CreateServiceMatchInput {
  request_id: string;
  offer_id: string;
  match_quality: number;
}

/** Input for proposing offer to request */
export interface ProposeOfferToRequestInput {
  offer_id: string;
  request_id: string;
  proposal_message: string;
  metadata_json?: string;
}

/** Input for settling payment */
export interface SettlePaymentInput {
  match_id: string;
  amount_value: number;
  amount_unit: string;
  medium_of_exchange_id: string;
  payment_method?: string;
  metadata_json?: string;
}

// =============================================================================
// Shefa: Constitutional Limits & Transitions (Donut Economy & Limitarianism)
// =============================================================================

/** ConstitutionalLimit - Defines floor and ceiling bounds for resources */
export interface ConstitutionalLimit {
  id: string;
  category: string;
  dimension: string;
  floor_value: number;
  floor_rationale: string;
  ceiling_value: number;
  ceiling_rationale: string;
  safe_min_value: number;
  safe_max_value: number;
  enforcement_method: string; // "voluntary", "progressive", "hard"
  transition_deadline: string; // ISO 8601
  hard_stop_date: string | null;
  constitutional_basis_key: string;
  governance_level: string;
  governed_by: string;
  schema_version: number;
  validation_status: string;
  created_at: string;
  updated_at: string;
}

/** ResourcePosition - Assessment of where a resource stands relative to bounds */
export interface ResourcePosition {
  id: string;
  resource_id: string;
  limit_id: string;
  assessment_date: string;
  position_type: string; // "below-floor", "at-floor", "in-safe-zone", "above-ceiling", "far-above-ceiling"
  current_value: number;
  distance_from_floor: number;
  distance_from_ceiling: number;
  excess_above_ceiling: number;
  excess_percentage: number;
  surplus_available_for_transition: number;
  compliant: boolean;
  warning_level: string; // "none", "caution", "warning", "critical"
  days_to_hard_stop: number | null;
  has_active_transition: boolean;
  transition_path_id: string | null;
  transition_status: string | null;
  schema_version: number;
  validation_status: string;
  created_at: string;
  updated_at: string;
}

/** AssetSplit - How excess asset is divided among destinations */
export interface AssetSplit {
  id: string;
  transition_path_id: string;
  split_name: string;
  destination_type: string;
  destination_id: string;
  amount: number;
  percentage: number;
  legacy_role: string | null;
  legacy_role_details: string | null;
  status: string;
  agreed_at: string | null;
  completed_at: string | null;
  rationale: string;
  terms: string | null;
  conditions: string | null;
  schema_version: number;
  created_at: string;
  updated_at: string;
}

/** TransitionPhase - Sequential phase in the execution of a transition */
export interface TransitionPhase {
  id: string;
  transition_path_id: string;
  sequence_number: number;
  name: string;
  description: string;
  target_start_date: string;
  target_end_date: string;
  actual_start_date: string | null;
  actual_end_date: string | null;
  amount_in_phase: number;
  actions_json: string; // Vec<TransitionAction> as JSON
  total_actions: number;
  completed_actions: number;
  failed_actions: number;
  status: string;
  block_reason: string | null;
  block_date: string | null;
  schema_version: number;
  created_at: string;
  updated_at: string;
}

/** TransitionAction - Specific action within a phase */
export interface TransitionAction {
  id: string;
  phase_id: string;
  action_type: string;
  action_description: string;
  responsible_party: string;
  assigned_to: string | null;
  target_date: string;
  actual_date: string | null;
  deadline_critical: boolean;
  status: string;
  completion_notes: string | null;
  failure_reason: string | null;
  economic_event_id: string | null;
  schema_version: number;
  created_at: string;
  updated_at: string;
}

/** TransitionPath - Structured process for moving excess assets to community stewardship */
export interface TransitionPath {
  id: string;
  steward_id: string;
  resource_id: string;
  limit_id: string;
  current_value: number;
  constitutional_ceiling: number;
  excess_amount: number;
  proposed_splits_json: string; // Vec<AssetSplit> as JSON
  status: string;
  initiated_at: string;
  negotiation_deadline: string;
  execution_start_date: string | null;
  target_completion_date: string | null;
  actual_completion_date: string | null;
  governance_level: string;
  governing_body: string;
  approval_status: string;
  approval_date: string | null;
  approved_by: string | null;
  visibility: string;
  rationale: string;
  phases_json: string; // Vec<TransitionPhase> as JSON
  current_phase: number;
  transition_event_ids_json: string; // Vec<String> as JSON
  governance_proposal_event_id: string | null;
  block_reason: string | null;
  blocked_at: string | null;
  disputed: boolean;
  schema_version: number;
  validation_status: string;
  created_at: string;
  updated_at: string;
}

/** CommonsContribution - Recognition when asset transitions to community stewardship */
export interface CommonsContribution {
  id: string;
  steward_id: string;
  transition_path_id: string;
  asset_split_id: string;
  original_holding: number;
  contributed_amount: number;
  contribution_date: string;
  destination_commons_pool: string;
  commons_pool_id: string;
  governance_credit_amount: number;
  governance_credit_category: string;
  legacy_role: string | null;
  legacy_role_details: string | null;
  public_recognition: boolean;
  recognition_statement: string | null;
  recognition_date: string | null;
  economic_event_id: string;
  governance_proposal_id: string | null;
  schema_version: number;
  created_at: string;
  updated_at: string;
}

/** Input for creating a constitutional limit */
export interface CreateConstitutionalLimitInput {
  category: string;
  dimension: string;
  floor_value: number;
  floor_rationale: string;
  ceiling_value: number;
  ceiling_rationale: string;
  safe_min_value: number;
  safe_max_value: number;
  enforcement_method: string;
  transition_deadline: string;
  hard_stop_date?: string | null;
  constitutional_basis_key: string;
  governance_level: string;
  governed_by: string;
}

/** Input for assessing resource position */
export interface AssessResourcePositionInput {
  resource_id: string;
  limit_id: string;
}

/** Input for initiating a transition path */
export interface InitiateTransitionPathInput {
  resource_id: string;
  limit_id: string;
  proposed_splits_json: string; // Vec<AssetSplit> as JSON
  visibility: string;
  rationale: string;
}

/** Input for executing a transition phase */
export interface ExecuteTransitionPhaseInput {
  transition_path_id: string;
  phase_number: number;
}

// =============================================================================
// Lamad: Learning Economy (uses Shefa hREA primitives)
// =============================================================================
//
// Lamad is a learning platform implementation that uses Shefa's hREA economic
// substrate. This demonstrates how domain-specific applications compose the
// generalizable Shefa primitives:
//
//   Lamad Type              →  Shefa Primitive
//   ─────────────────────────────────────────────
//   PointEvent              →  EconomicEvent (learner activity)
//   LearnerPointBalance     →  EconomicResource (accumulated value)
//   ContributorRecognition  →  Appreciation (value flow to contributors)
//   ContributorDashboard    →  Aggregation view over hREA flows
//
// Other domains (marketplaces, care networks, etc.) would create their own
// domain-specific types that similarly compose Shefa primitives.

/**
 * Learning-specific triggers that generate Shefa EconomicEvents.
 * Each trigger maps to an hREA action (produce/consume) with a point value.
 */
export const LamadPointTriggers = {
  ENGAGEMENT_VIEW: 'engagement_view',
  ENGAGEMENT_PRACTICE: 'engagement_practice',
  CHALLENGE_CORRECT: 'challenge_correct',
  CHALLENGE_COMPLETE: 'challenge_complete',
  LEVEL_UP: 'level_up',
  LEVEL_DOWN: 'level_down',
  DISCOVERY: 'discovery',
  PATH_STEP_COMPLETE: 'path_step_complete',
  PATH_COMPLETE: 'path_complete',
  CONTRIBUTION: 'contribution',
} as const;

export type LamadPointTrigger = typeof LamadPointTriggers[keyof typeof LamadPointTriggers];

/** Default point amounts per learning trigger */
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

/**
 * Learner's point balance - a learning-specific EconomicResource.
 * Tracks accumulated value from learning activities.
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

/**
 * Learning point event - a learning-specific EconomicEvent.
 * Records a learner activity that produced or consumed value.
 */
export interface LamadPointEvent {
  id: string;
  agent_id: string;
  action: string;  // hREA action: "produce" or "consume"
  trigger: string; // LamadPointTrigger
  points: number;
  content_id: string | null;
  challenge_id: string | null;
  path_id: string | null;
  was_correct: boolean | null;
  note: string | null;
  metadata_json: string;
  occurred_at: string;
}

/**
 * Learning recognition - a learning-specific Appreciation.
 * Records value flowing from learner activity to content contributors.
 */
export interface LamadContributorRecognition {
  id: string;
  contributor_id: string;  // ContributorPresence ID (Shefa)
  content_id: string;
  learner_id: string;
  appreciation_of_event_id: string;  // References the triggering EconomicEvent
  flow_type: string;
  recognition_points: number;
  path_id: string | null;
  challenge_id: string | null;
  note: string | null;
  metadata_json: string;
  occurred_at: string;
}

/** Output for learner point balance */
export interface LearnerPointBalanceOutput {
  action_hash: ActionHash;
  balance: LearnerPointBalance;
}

/** Output for learning point event */
export interface LamadPointEventOutput {
  action_hash: ActionHash;
  event: LamadPointEvent;
}

/** Output for learning contributor recognition */
export interface LamadContributorRecognitionOutput {
  action_hash: ActionHash;
  recognition: LamadContributorRecognition;
}

/** Input for earning learning points */
export interface EarnLamadPointsInput {
  trigger: string;  // LamadPointTrigger
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
  recognition_sent: LamadContributorRecognitionOutput[];
  points_earned: number;
}

// -----------------------------------------------------------------------------
// Lamad: Learning Economy Aggregations
// -----------------------------------------------------------------------------
// Domain-specific aggregation views over Shefa hREA flows.
// These compose EconomicEvents, Resources, and Appreciations into
// learning-meaningful dashboards.

/** Aggregate learning impact for a contributor */
export interface LamadContributorImpact {
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

/** Output for contributor impact */
export interface LamadContributorImpactOutput {
  action_hash: ActionHash;
  impact: LamadContributorImpact;
}

/** Learning impact summary per content piece */
export interface LamadContentImpactSummary {
  content_id: string;
  recognition_points: number;
  learners_reached: number;
  mastery_count: number;
}

/** Recent learning recognition event for timeline display */
export interface LamadRecognitionEventSummary {
  learner_id: string;
  content_id: string;
  flow_type: string;
  recognition_points: number;
  occurred_at: string;
}

/**
 * Learning Contributor Dashboard - aggregated view of hREA value flows.
 * Shows the impact of a contributor's work as recognition flows from learners.
 *
 * This is a Lamad-specific aggregation. Other domains would create their own:
 *   - Marketplace: SellerDashboard, BuyerDashboard
 *   - Care Network: CaregiverImpact, CareRecipientJourney
 *   - Commons: StewardDashboard, ResourceHealthView
 */
export interface LamadContributorDashboard {
  contributor_id: string;
  total_recognition_points: number;
  total_learners_reached: number;
  total_content_mastered: number;
  total_discoveries_sparked: number;
  impact_by_content: LamadContentImpactSummary[];
  recent_events: LamadRecognitionEventSummary[];
  impact: LamadContributorImpactOutput | null;
}

// =============================================================================
// Lamad: Steward Economy Types
// =============================================================================
//
// The Steward Economy enables sustainable income for those who care-take the
// knowledge graph. Stewards may or may not be the original creators - they
// earn from maintaining, curating, and making knowledge accessible.
//
// Key concepts:
// - StewardCredential: Proof of qualification (mastery, peer attestations, track record)
// - PremiumGate: Access control with pricing and revenue sharing
// - AccessGrant: Record of learner gaining access
// - StewardRevenue: Three-way split (steward, contributor, commons)

/** Steward tiers - levels of stewardship */
export const StewardTiers = {
  CARETAKER: 'caretaker',     // Basic stewardship - maintains existing content
  CURATOR: 'curator',         // Active curation - organizes and improves paths
  EXPERT: 'expert',           // Domain expertise + peer attestations
  PIONEER: 'pioneer',         // Original researcher/synthesizer, primary steward
} as const;

export type StewardTier = typeof StewardTiers[keyof typeof StewardTiers];

/** Access grant types */
export const AccessGrantTypes = {
  LIFETIME: 'lifetime',       // One-time purchase, permanent access
  SUBSCRIPTION: 'subscription', // Recurring payment, time-limited access
  SCHOLARSHIP: 'scholarship', // Commons-funded access for qualifying learners
  CREATOR_GIFT: 'creator_gift', // Gifted by steward/creator
} as const;

export type AccessGrantType = typeof AccessGrantTypes[keyof typeof AccessGrantTypes];

/** Pricing models for premium gates */
export const PricingModels = {
  ONE_TIME: 'one_time',
  SUBSCRIPTION: 'subscription',
  PAY_WHAT_YOU_CAN: 'pay_what_you_can',
  FREE_WITH_ATTRIBUTION: 'free_with_attribution',
  COMMONS_SPONSORED: 'commons_sponsored',
} as const;

export type PricingModel = typeof PricingModels[keyof typeof PricingModels];

/** Access requirement types */
export const AccessRequirementTypes = {
  ATTESTATION: 'attestation',
  MASTERY: 'mastery',
  PAYMENT: 'payment',
  PEER_VOUCH: 'peer_vouch',
  SCHOLARSHIP: 'scholarship',
} as const;

export type AccessRequirementType = typeof AccessRequirementTypes[keyof typeof AccessRequirementTypes];

/**
 * StewardCredential - Proof of qualification to steward content.
 * Tracks mastery, peer attestations, and stewardship track record.
 */
export interface StewardCredential {
  id: string;
  steward_presence_id: string;
  agent_id: string;
  tier: string;
  // Stewardship scope
  stewarded_presence_ids_json: string;
  stewarded_content_ids_json: string;
  stewarded_path_ids_json: string;
  // Domain qualification
  mastery_content_ids_json: string;
  mastery_level_achieved: string;
  qualification_verified_at: string;
  // Peer attestation
  peer_attestation_ids_json: string;
  unique_attester_count: number;
  attester_reputation_sum: number;
  // Track record
  stewardship_quality_score: number;
  total_learners_served: number;
  total_content_improvements: number;
  // Domain scope
  domain_tags_json: string;
  // Status
  is_active: boolean;
  deactivation_reason: string | null;
  // Metadata
  note: string | null;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/**
 * PremiumGate - Access control for curated content/paths.
 * Defines requirements learners must meet to access premium content.
 */
export interface PremiumGate {
  id: string;
  steward_credential_id: string;
  steward_presence_id: string;
  contributor_presence_id: string | null;
  // What's being gated
  gated_resource_type: string;
  gated_resource_ids_json: string;
  gate_title: string;
  gate_description: string;
  gate_image: string | null;
  // Access requirements
  required_attestations_json: string;
  required_mastery_json: string;
  required_vouches_json: string;
  // Pricing
  pricing_model: string;
  price_amount: number | null;
  price_unit: string | null;
  subscription_period_days: number | null;
  min_amount: number | null;
  // Revenue share
  steward_share_percent: number;
  commons_share_percent: number;
  contributor_share_percent: number | null;
  // Scholarship support
  scholarship_eligible: boolean;
  max_scholarships_per_period: number | null;
  scholarship_criteria_json: string | null;
  // Status
  is_active: boolean;
  deactivation_reason: string | null;
  // Stats
  total_access_grants: number;
  total_revenue_generated: number;
  total_to_steward: number;
  total_to_contributor: number;
  total_to_commons: number;
  total_scholarships_granted: number;
  // Metadata
  note: string | null;
  metadata_json: string;
  created_at: string;
  updated_at: string;
}

/**
 * AccessGrant - Record of learner gaining access to gated content.
 */
export interface AccessGrant {
  id: string;
  gate_id: string;
  learner_agent_id: string;
  // How access was granted
  grant_type: string;
  granted_via: string;
  // Payment details
  payment_event_id: string | null;
  payment_amount: number | null;
  payment_unit: string | null;
  // Scholarship details
  scholarship_sponsor_id: string | null;
  scholarship_reason: string | null;
  // Access window
  granted_at: string;
  valid_until: string | null;
  renewal_due_at: string | null;
  // Status
  is_active: boolean;
  revoked_at: string | null;
  revoke_reason: string | null;
  // Metadata
  metadata_json: string;
  created_at: string;
}

/**
 * StewardRevenue - Value flowing from gate access to stewards and contributors.
 * Supports three-way split when steward holds content in trust.
 */
export interface StewardRevenue {
  id: string;
  access_grant_id: string;
  gate_id: string;
  // Parties
  from_learner_id: string;
  to_steward_presence_id: string;
  to_contributor_presence_id: string | null;
  // Amounts
  gross_amount: number;
  payment_unit: string;
  steward_amount: number;
  contributor_amount: number;
  commons_amount: number;
  // Shefa linkage
  steward_economic_event_id: string;
  contributor_economic_event_id: string | null;
  commons_economic_event_id: string;
  // Status
  status: string;
  completed_at: string | null;
  failure_reason: string | null;
  // Metadata
  note: string | null;
  metadata_json: string;
  created_at: string;
}

/** Output for steward credential */
export interface StewardCredentialOutput {
  action_hash: ActionHash;
  credential: StewardCredential;
}

/** Output for premium gate */
export interface PremiumGateOutput {
  action_hash: ActionHash;
  gate: PremiumGate;
}

/** Output for access grant */
export interface AccessGrantOutput {
  action_hash: ActionHash;
  grant: AccessGrant;
}

/** Output for steward revenue */
export interface StewardRevenueOutput {
  action_hash: ActionHash;
  revenue: StewardRevenue;
}

/** Input for creating a steward credential */
export interface CreateStewardCredentialInput {
  steward_presence_id: string;
  tier: string;
  domain_tags: string[];
  mastery_content_ids: string[];
  mastery_level_achieved: string;
  peer_attestation_ids: string[];
  stewarded_presence_ids: string[];
  stewarded_content_ids: string[];
  stewarded_path_ids: string[];
  note?: string;
}

/** Required attestation for gate access */
export interface RequiredAttestation {
  attestation_type: string;
  attestation_id?: string;
}

/** Required mastery for gate access */
export interface RequiredMastery {
  content_id: string;
  min_level: string;
}

/** Required vouches for gate access */
export interface RequiredVouches {
  min_count: number;
  from_tier?: string;
}

/** Input for creating a premium gate */
export interface CreatePremiumGateInput {
  steward_credential_id: string;
  steward_presence_id: string;
  contributor_presence_id?: string;
  gated_resource_type: string;
  gated_resource_ids: string[];
  gate_title: string;
  gate_description: string;
  gate_image?: string;
  required_attestations: RequiredAttestation[];
  required_mastery: RequiredMastery[];
  required_vouches?: RequiredVouches;
  pricing_model: string;
  price_amount?: number;
  price_unit?: string;
  subscription_period_days?: number;
  min_amount?: number;
  steward_share_percent: number;
  commons_share_percent: number;
  contributor_share_percent?: number;
  scholarship_eligible: boolean;
  max_scholarships_per_period?: number;
  scholarship_criteria_json?: string;
  note?: string;
}

/** Input for granting access */
export interface GrantAccessInput {
  gate_id: string;
  grant_type: string;
  granted_via: string;
  payment_amount?: number;
  payment_unit?: string;
  scholarship_sponsor_id?: string;
  scholarship_reason?: string;
}

/** Steward revenue summary */
export interface StewardRevenueSummary {
  steward_presence_id: string;
  total_revenue: number;
  total_grants: number;
  revenue_by_gate: GateRevenueSummary[];
}

/** Revenue summary per gate */
export interface GateRevenueSummary {
  gate_id: string;
  gate_title: string;
  total_revenue: number;
  grant_count: number;
}

// =============================================================================
// SDK Configuration Types
// =============================================================================

/** SDK connection configuration */
export interface ConnectionConfig {
  adminUrl: string;
  appUrl?: string;
  appId?: string;
  roleId?: string;
  timeout?: number;
}

/** Query criteria for content search */
export interface QueryCriteria {
  content_type?: string;
  tags?: string[];
  epic?: string;
  user_type?: string;
  limit?: number;
}

/** Criteria for path generation */
export interface PathGenerationCriteria {
  epic: string;
  user_type: string;
  difficulty: 'beginner' | 'intermediate' | 'advanced';
  max_steps?: number;
  include_types?: string[];
  exclude_types?: string[];
}

// =============================================================================
// Constants (shared with Rust)
// =============================================================================

/** Default batch size for bulk operations (WASM memory limit) */
export const DEFAULT_BATCH_SIZE = 50;

/** Maximum batch size before splitting */
export const MAX_BATCH_SIZE = 100;

/** Zome name */
export const ZOME_NAME = 'content_store';

/** App ID for lamad-spike */
export const DEFAULT_APP_ID = 'lamad-spike';

/** Role ID for lamad DNA */
export const DEFAULT_ROLE_ID = 'lamad';
