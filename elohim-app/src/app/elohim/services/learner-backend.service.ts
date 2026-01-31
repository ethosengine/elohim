/**
 * Learner Backend Service
 *
 * Low-level wrapper around zome calls for learning operations.
 * Handles mastery tracking, practice pools, challenges, and learning points.
 *
 * This service is the backend integration layer for the Lamad learning system.
 * It provides typed async methods that wrap zome calls.
 *
 * Architecture:
 *   LearnerBackendService (this) → HolochainClientService → Conductor → DHT
 *
 * Patterns:
 * - All methods return Promise<T | null> - null on errors (graceful degradation)
 * - No caching at this layer - that's for domain services
 * - Types imported from lamad/models
 */

import { Injectable } from '@angular/core';

// @coverage: 3.5% (2026-01-31)

import { HolochainClientService } from './holochain-client.service';

import type {
  ContentMasteryOutput,
  InitializeMasteryInput,
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

const ZOME_NAME = 'content_store';

@Injectable({
  providedIn: 'root',
})
export class LearnerBackendService {
  constructor(private readonly holochainClient: HolochainClientService) {}

  // ===========================================================================
  // Connection Status
  // ===========================================================================

  /**
   * Check if Holochain is connected and available.
   */
  isAvailable(): boolean {
    return this.holochainClient.isConnected();
  }

  // ===========================================================================
  // Content Mastery Operations
  // ===========================================================================

  /**
   * Initialize mastery tracking for a content node.
   * Creates a mastery record at 'not_started' level.
   */
  async initializeMastery(contentId: string): Promise<ContentMasteryOutput | null> {
    const input: InitializeMasteryInput = { content_id: contentId };
    const result = await this.holochainClient.callZome<ContentMasteryOutput>({
      zomeName: ZOME_NAME,
      fnName: 'initialize_mastery',
      payload: input,
    });

    if (!result.success) {
      return null;
    }
    return result.data ?? null;
  }

  /**
   * Record engagement with content (view, practice, etc.).
   * May upgrade mastery level based on engagement type.
   */
  async recordEngagement(input: RecordEngagementInput): Promise<ContentMasteryOutput | null> {
    const result = await this.holochainClient.callZome<ContentMasteryOutput>({
      zomeName: ZOME_NAME,
      fnName: 'record_engagement',
      payload: input,
    });

    if (!result.success) {
      return null;
    }
    return result.data ?? null;
  }

  /**
   * Record assessment result (quiz/test) with potential level up.
   */
  async recordAssessment(input: RecordAssessmentInput): Promise<ContentMasteryOutput | null> {
    const result = await this.holochainClient.callZome<ContentMasteryOutput>({
      zomeName: ZOME_NAME,
      fnName: 'record_assessment',
      payload: input,
    });

    if (!result.success) {
      return null;
    }
    return result.data ?? null;
  }

  /**
   * Get my mastery for a specific content node.
   */
  async getMyMastery(contentId: string): Promise<ContentMasteryOutput | null> {
    const result = await this.holochainClient.callZome<ContentMasteryOutput | null>({
      zomeName: ZOME_NAME,
      fnName: 'get_my_mastery',
      payload: contentId,
    });

    if (!result.success) {
      return null;
    }
    return result.data ?? null;
  }

  /**
   * Get all my mastery records.
   */
  async getMyAllMastery(): Promise<ContentMasteryOutput[]> {
    const result = await this.holochainClient.callZome<ContentMasteryOutput[]>({
      zomeName: ZOME_NAME,
      fnName: 'get_my_all_mastery',
      payload: null,
    });

    if (!result.success) {
      return [];
    }
    return result.data ?? [];
  }

  /**
   * Batch query mastery for multiple content IDs (Khan Academy missions view).
   */
  async getMasteryBatch(contentIds: string[]): Promise<MasterySnapshot[]> {
    const result = await this.holochainClient.callZome<MasterySnapshot[]>({
      zomeName: ZOME_NAME,
      fnName: 'get_mastery_batch',
      payload: contentIds,
    });

    if (!result.success) {
      return [];
    }
    return result.data ?? [];
  }

  /**
   * Get full path mastery overview (Khan Academy missions view).
   */
  async getPathMasteryOverview(pathId: string): Promise<PathMasteryOverview | null> {
    const result = await this.holochainClient.callZome<PathMasteryOverview>({
      zomeName: ZOME_NAME,
      fnName: 'get_path_mastery_overview',
      payload: pathId,
    });

    if (!result.success) {
      return null;
    }
    return result.data ?? null;
  }

  /**
   * Get my mastery statistics dashboard.
   */
  async getMyMasteryStats(): Promise<MasteryStatsWire | null> {
    const result = await this.holochainClient.callZome<MasteryStatsWire>({
      zomeName: ZOME_NAME,
      fnName: 'get_my_mastery_stats',
      payload: null,
    });

    if (!result.success) {
      return null;
    }
    return result.data ?? null;
  }

  /**
   * Check if agent has privilege for a content node.
   */
  async checkPrivilege(input: CheckPrivilegeInput): Promise<PrivilegeCheckResult | null> {
    const result = await this.holochainClient.callZome<PrivilegeCheckResult>({
      zomeName: ZOME_NAME,
      fnName: 'check_privilege',
      payload: input,
    });

    if (!result.success) {
      return null;
    }
    return result.data ?? null;
  }

  // ===========================================================================
  // Practice Pool Operations
  // ===========================================================================

  /**
   * Get or create practice pool for current agent.
   */
  async getOrCreatePracticePool(input: CreatePoolInput): Promise<PracticePoolOutput | null> {
    const result = await this.holochainClient.callZome<PracticePoolOutput>({
      zomeName: ZOME_NAME,
      fnName: 'get_or_create_practice_pool',
      payload: input,
    });

    if (!result.success) {
      return null;
    }
    return result.data ?? null;
  }

  /**
   * Refresh practice pool with content from paths and knowledge graph.
   */
  async refreshPracticePool(): Promise<PracticePoolOutput | null> {
    const result = await this.holochainClient.callZome<PracticePoolOutput>({
      zomeName: ZOME_NAME,
      fnName: 'refresh_practice_pool',
      payload: null,
    });

    if (!result.success) {
      return null;
    }
    return result.data ?? null;
  }

  /**
   * Add a path to the practice pool.
   */
  async addPathToPool(pathId: string): Promise<PracticePoolOutput | null> {
    const result = await this.holochainClient.callZome<PracticePoolOutput>({
      zomeName: ZOME_NAME,
      fnName: 'add_path_to_pool',
      payload: pathId,
    });

    if (!result.success) {
      return null;
    }
    return result.data ?? null;
  }

  /**
   * Get pool recommendations for what to practice.
   */
  async getPoolRecommendations(): Promise<PoolRecommendations | null> {
    const result = await this.holochainClient.callZome<PoolRecommendations>({
      zomeName: ZOME_NAME,
      fnName: 'get_pool_recommendations',
      payload: null,
    });

    if (!result.success) {
      return null;
    }
    return result.data ?? null;
  }

  /**
   * Check if agent can take a mastery challenge (cooldown).
   */
  async checkChallengeCooldown(): Promise<CooldownCheckResult | null> {
    const result = await this.holochainClient.callZome<CooldownCheckResult>({
      zomeName: ZOME_NAME,
      fnName: 'check_challenge_cooldown',
      payload: null,
    });

    if (!result.success) {
      return null;
    }
    return result.data ?? null;
  }

  // ===========================================================================
  // Mastery Challenge Operations
  // ===========================================================================

  /**
   * Start a mastery challenge with mixed content from pool.
   */
  async startMasteryChallenge(input: StartChallengeInput): Promise<MasteryChallengeOutput | null> {
    const result = await this.holochainClient.callZome<MasteryChallengeOutput>({
      zomeName: ZOME_NAME,
      fnName: 'start_mastery_challenge',
      payload: input,
    });

    if (!result.success) {
      return null;
    }
    return result.data ?? null;
  }

  /**
   * Submit challenge responses and apply level changes (up or down).
   */
  async submitMasteryChallenge(input: SubmitChallengeInput): Promise<ChallengeResult | null> {
    const result = await this.holochainClient.callZome<ChallengeResult>({
      zomeName: ZOME_NAME,
      fnName: 'submit_mastery_challenge',
      payload: input,
    });

    if (!result.success) {
      return null;
    }
    return result.data ?? null;
  }

  /**
   * Get challenge history for current agent.
   */
  async getChallengeHistory(): Promise<MasteryChallengeOutput[]> {
    const result = await this.holochainClient.callZome<MasteryChallengeOutput[]>({
      zomeName: ZOME_NAME,
      fnName: 'get_challenge_history',
      payload: null,
    });

    if (!result.success) {
      return [];
    }
    return result.data ?? [];
  }

  // ===========================================================================
  // Learning Points Operations (Shefa Integration)
  // ===========================================================================

  /**
   * Earn learning points for an activity.
   * Points automatically flow recognition to content contributors via Shefa.
   */
  async earnLamadPoints(input: EarnLamadPointsInput): Promise<EarnLamadPointsResult | null> {
    const result = await this.holochainClient.callZome<EarnLamadPointsResult>({
      zomeName: ZOME_NAME,
      fnName: 'earn_lamad_points',
      payload: input,
    });

    if (!result.success) {
      return null;
    }
    return result.data ?? null;
  }

  /**
   * Get my current learning point balance.
   */
  async getMyLamadPointBalance(): Promise<LearnerPointBalanceOutput | null> {
    const result = await this.holochainClient.callZome<LearnerPointBalanceOutput | null>({
      zomeName: ZOME_NAME,
      fnName: 'get_my_lamad_point_balance',
      payload: null,
    });

    if (!result.success) {
      return null;
    }
    return result.data ?? null;
  }

  /**
   * Get my learning point history.
   */
  async getMyLamadPointHistory(limit?: number): Promise<LamadPointEventOutput[]> {
    const result = await this.holochainClient.callZome<LamadPointEventOutput[]>({
      zomeName: ZOME_NAME,
      fnName: 'get_my_lamad_point_history',
      payload: limit ?? null,
    });

    if (!result.success) {
      return [];
    }
    return result.data ?? [];
  }
}
