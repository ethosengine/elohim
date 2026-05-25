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
 * Wire types (snake_case, *_json fields) are defined in @app/elohim/models/zome-wire-types
 * and re-exported here for backwards compatibility.
 */

// @coverage: 100.0% (2026-02-24)

// =============================================================================
// Re-exported Wire Types from centralized zome-wire-types
// =============================================================================

export type {
  ActionHash,
  PracticePool,
  PracticePoolOutput,
  MasteryChallenge,
  MasteryChallengeOutput,
  DiscoveryCandidate,
  ContentMixEntry,
  ChallengeQuestion,
  MasteryChallengeResponse,
  LevelChange,
  ChallengeDiscovery,
  CreatePoolInput,
  StartChallengeInput,
  SubmitChallengeInput,
  ChallengeResult,
  CooldownCheckResult,
  PoolRecommendations,
  PoolSource,
  MasteryChallengeState,
} from '@app/elohim/models/zome-wire-types';

export {
  PoolSources,
  MasteryChallengeStates,
  parseContentMix,
  parseLevelChanges,
  parseQuestions,
  parseResponses,
  parseDiscoveries,
  getActiveContentIds,
  getRefreshQueueIds,
  getDiscoveryCandidates,
} from '@app/elohim/models/zome-wire-types';
