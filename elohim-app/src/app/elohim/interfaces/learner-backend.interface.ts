/**
 * ILearnerBackend -- Abstract interface for learner mastery tracking,
 * practice pools, challenges, and learning points in Lamad.
 *
 * Decouples the learning backend from the concrete implementation.
 * Consumers inject the LEARNER_BACKEND token; the default factory
 * resolves to LearnerBackendApiService.
 *
 * Tests provide a mock ILearnerBackend via the same token -- no
 * concrete service import needed.
 *
 * @example
 * ```typescript
 * @Injectable({ providedIn: 'root' })
 * export class MasteryTrackingService {
 *   private readonly backend = inject(LEARNER_BACKEND);
 *
 *   async trackEngagement(input: RecordEngagementInput) {
 *     return this.backend.recordEngagement(input);
 *   }
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import { LearnerBackendApiService } from '../services/learner-backend-api.service';

import type {
  ContentMasteryOutput,
  RecordEngagementInput,
  RecordAssessmentInput,
  MasterySnapshot,
  PathMasteryOverview,
  MasteryStatsWire,
  CheckPrivilegeInput,
  PrivilegeCheckResult,
} from '@app/lamad/models/content-mastery.model';
import type {
  LearnerPointBalanceOutput,
  LamadPointEventOutput,
  EarnLamadPointsInput,
  EarnLamadPointsResult,
} from '@app/lamad/models/learning-points.model';
import type {
  PracticePoolOutput,
  CreatePoolInput,
  PoolRecommendations,
  CooldownCheckResult,
  MasteryChallengeOutput,
  StartChallengeInput,
  SubmitChallengeInput,
  ChallengeResult,
} from '@app/lamad/models/practice.model';

/**
 * Abstract learner backend -- manages mastery tracking, practice pools,
 * mastery challenges, and learning points.
 *
 * Implementations handle HTTP calls, Holochain zome calls, or test mocks.
 */
export interface ILearnerBackend {
  // ==========================================================================
  // Connection Status
  // ==========================================================================

  /**
   * Check if the backend is connected and available.
   */
  isAvailable(): boolean;

  // ==========================================================================
  // Content Mastery Operations
  // ==========================================================================

  /**
   * Initialize mastery tracking for a content node.
   * Creates a mastery record at 'not_started' level.
   */
  initializeMastery(contentId: string): Promise<ContentMasteryOutput | null>;

  /**
   * Record engagement with content (view, practice, etc.).
   * May upgrade mastery level based on engagement type.
   */
  recordEngagement(input: RecordEngagementInput): Promise<ContentMasteryOutput | null>;

  /**
   * Record assessment result (quiz/test) with potential level up.
   */
  recordAssessment(input: RecordAssessmentInput): Promise<ContentMasteryOutput | null>;

  /**
   * Get my mastery for a specific content node.
   */
  getMyMastery(contentId: string): Promise<ContentMasteryOutput | null>;

  /**
   * Get all my mastery records.
   */
  getMyAllMastery(): Promise<ContentMasteryOutput[]>;

  /**
   * Batch query mastery for multiple content IDs (Khan Academy missions view).
   */
  getMasteryBatch(contentIds: string[]): Promise<MasterySnapshot[]>;

  /**
   * Get full path mastery overview (Khan Academy missions view).
   */
  getPathMasteryOverview(pathId: string): Promise<PathMasteryOverview | null>;

  /**
   * Get my mastery statistics dashboard.
   */
  getMyMasteryStats(): Promise<MasteryStatsWire | null>;

  /**
   * Check if agent has privilege for a content node.
   */
  checkPrivilege(input: CheckPrivilegeInput): Promise<PrivilegeCheckResult | null>;

  // ==========================================================================
  // Practice Pool Operations
  // ==========================================================================

  /**
   * Get or create practice pool for current agent.
   */
  getOrCreatePracticePool(input: CreatePoolInput): Promise<PracticePoolOutput | null>;

  /**
   * Refresh practice pool with content from paths and knowledge graph.
   */
  refreshPracticePool(): Promise<PracticePoolOutput | null>;

  /**
   * Add a path to the practice pool.
   */
  addPathToPool(pathId: string): Promise<PracticePoolOutput | null>;

  /**
   * Get pool recommendations for what to practice.
   */
  getPoolRecommendations(): Promise<PoolRecommendations | null>;

  /**
   * Check if agent can take a mastery challenge (cooldown).
   */
  checkChallengeCooldown(): Promise<CooldownCheckResult | null>;

  // ==========================================================================
  // Mastery Challenge Operations
  // ==========================================================================

  /**
   * Start a mastery challenge with mixed content from pool.
   */
  startMasteryChallenge(input: StartChallengeInput): Promise<MasteryChallengeOutput | null>;

  /**
   * Submit challenge responses and apply level changes (up or down).
   */
  submitMasteryChallenge(input: SubmitChallengeInput): Promise<ChallengeResult | null>;

  /**
   * Get challenge history for current agent.
   */
  getChallengeHistory(): Promise<MasteryChallengeOutput[]>;

  // ==========================================================================
  // Learning Points Operations (Shefa Integration)
  // ==========================================================================

  /**
   * Earn learning points for an activity.
   * Points automatically flow recognition to content contributors via Shefa.
   */
  earnLamadPoints(input: EarnLamadPointsInput): Promise<EarnLamadPointsResult | null>;

  /**
   * Get my current learning point balance.
   */
  getMyLamadPointBalance(): Promise<LearnerPointBalanceOutput | null>;

  /**
   * Get my learning point history.
   */
  getMyLamadPointHistory(limit?: number): Promise<LamadPointEventOutput[]>;
}

/**
 * Injection token for the learner backend.
 *
 * Default factory resolves to LearnerBackendApiService which provides:
 * - Content mastery CRUD via /api/v1/mastery/* endpoints
 * - Practice pool management
 * - Mastery challenge lifecycle
 * - Learning points (Shefa integration)
 *
 * Override in tests:
 * ```typescript
 * { provide: LEARNER_BACKEND, useValue: mockBackend }
 * ```
 */
export const LEARNER_BACKEND = new InjectionToken<ILearnerBackend>('LearnerBackend', {
  providedIn: 'root',
  factory: () => inject(LearnerBackendApiService),
});
