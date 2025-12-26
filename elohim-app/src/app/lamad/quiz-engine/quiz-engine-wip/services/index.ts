/**
 * Quiz Engine Services
 *
 * Services for quiz session management:
 * - Question pool loading and selection
 * - Quiz session state machine
 * - Streak tracking
 * - Attempt cooldowns
 * - Path adaptation
 * - Discovery assessments
 */

export { QuestionPoolService } from './question-pool.service';
export { QuizSessionService } from './quiz-session.service';
export { StreakTrackerService } from './streak-tracker.service';
export { AttemptCooldownService } from './attempt-cooldown.service';
export { QuizSoundService } from './quiz-sound.service';
export { PathAdaptationService } from './path-adaptation.service';
export { DiscoveryAttestationService } from './discovery-attestation.service';
export type {
  GateStatus,
  GateLockReason,
  ContentRecommendation,
  RecommendationReason,
  SkipAheadResult,
  SkippableSection,
  PathAdaptationState,
  PathAdaptationConfig
} from './path-adaptation.service';
