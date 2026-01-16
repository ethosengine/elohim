/**
 * Practice Service
 *
 * Khan Academy-style practice pool and mastery challenge management.
 * Orchestrates spaced repetition, discovery, and level progression.
 *
 * Key concepts:
 * - Practice Pool: Agent's rotating set of content for organic learning
 * - Mastery Challenges: Mixed assessments that can level up or down
 * - Discoveries: New content unlocked through graph exploration
 * - Cooldown: Rate limiting between challenges
 *
 * Patterns:
 * - BehaviorSubject for reactive state
 * - Observable APIs for async operations
 * - Delegates to LearnerBackendService for zome calls
 */

import { Injectable } from '@angular/core';
import { BehaviorSubject, Observable, from, of } from 'rxjs';
import { map, tap, catchError, switchMap } from 'rxjs/operators';

import { LearnerBackendService } from '@app/elohim/services/learner-backend.service';

import type {
  PracticePool,
  MasteryChallenge,
  ChallengeResult,
  MasteryChallengeResponse,
  PoolRecommendations,
  CooldownCheckResult,
  CreatePoolInput,
  StartChallengeInput,
} from '../models/practice.model';

@Injectable({
  providedIn: 'root',
})
export class PracticeService {
  // ===========================================================================
  // Reactive State
  // ===========================================================================

  private readonly poolSubject = new BehaviorSubject<PracticePool | null>(null);
  private readonly currentChallengeSubject = new BehaviorSubject<MasteryChallenge | null>(null);
  private readonly recommendationsSubject = new BehaviorSubject<PoolRecommendations | null>(null);
  private readonly cooldownSubject = new BehaviorSubject<CooldownCheckResult | null>(null);
  private readonly challengeHistorySubject = new BehaviorSubject<MasteryChallenge[]>([]);

  /** Current practice pool */
  readonly pool$ = this.poolSubject.asObservable();

  /** Current in-progress challenge */
  readonly currentChallenge$ = this.currentChallengeSubject.asObservable();

  /** Pool recommendations */
  readonly recommendations$ = this.recommendationsSubject.asObservable();

  /** Challenge cooldown status */
  readonly cooldown$ = this.cooldownSubject.asObservable();

  /** Challenge history */
  readonly challengeHistory$ = this.challengeHistorySubject.asObservable();

  constructor(private readonly backend: LearnerBackendService) {}

  // ===========================================================================
  // Pool Management
  // ===========================================================================

  /**
   * Initialize or get practice pool for given paths.
   */
  initializePool(pathIds: string[]): Observable<PracticePool | null> {
    const input: CreatePoolInput = {
      contributing_path_ids: pathIds,
    };

    return from(this.backend.getOrCreatePracticePool(input)).pipe(
      map(output => output?.pool ?? null),
      tap(pool => {
        this.poolSubject.next(pool);
        if (pool) {
          this.refreshRecommendations();
          this.checkCooldown();
        }
      }),
      catchError(err => {
        console.warn('[Practice] Failed to initialize pool:', err);
        return of(null);
      })
    );
  }

  /**
   * Add a path to the practice pool.
   */
  addPathToPool(pathId: string): Observable<PracticePool | null> {
    return from(this.backend.addPathToPool(pathId)).pipe(
      map(output => output?.pool ?? null),
      tap(pool => {
        if (pool) {
          this.poolSubject.next(pool);
          this.refreshRecommendations();
        }
      }),
      catchError(err => {
        console.warn('[Practice] Failed to add path to pool:', err);
        return of(null);
      })
    );
  }

  /**
   * Refresh the practice pool with latest content.
   */
  refreshPool(): Observable<PracticePool | null> {
    return from(this.backend.refreshPracticePool()).pipe(
      map(output => output?.pool ?? null),
      tap(pool => {
        if (pool) {
          this.poolSubject.next(pool);
          this.refreshRecommendations();
        }
      }),
      catchError(err => {
        console.warn('[Practice] Failed to refresh pool:', err);
        return of(null);
      })
    );
  }

  /**
   * Get current pool synchronously.
   */
  getPoolSync(): PracticePool | null {
    return this.poolSubject.value;
  }

  // ===========================================================================
  // Recommendations
  // ===========================================================================

  /**
   * Refresh pool recommendations.
   */
  refreshRecommendations(): void {
    from(this.backend.getPoolRecommendations()).pipe(
      tap(recommendations => {
        this.recommendationsSubject.next(recommendations);
      }),
      catchError(err => {
        console.warn('[Practice] Failed to get recommendations:', err);
        return of(null);
      })
    ).subscribe();
  }

  /**
   * Get recommendations as observable.
   */
  getRecommendations$(): Observable<PoolRecommendations | null> {
    // Refresh if we don't have any
    if (!this.recommendationsSubject.value) {
      this.refreshRecommendations();
    }
    return this.recommendations$;
  }

  // ===========================================================================
  // Challenge Cooldown
  // ===========================================================================

  /**
   * Check if user can take a challenge.
   */
  checkCooldown(): void {
    from(this.backend.checkChallengeCooldown()).pipe(
      tap(cooldown => {
        this.cooldownSubject.next(cooldown);
      }),
      catchError(err => {
        console.warn('[Practice] Failed to check cooldown:', err);
        return of(null);
      })
    ).subscribe();
  }

  /**
   * Observable for whether user can take a challenge.
   */
  canTakeChallenge$(): Observable<boolean> {
    return this.cooldown$.pipe(
      map(cooldown => cooldown?.can_take_challenge ?? false)
    );
  }

  /**
   * Get cooldown info synchronously.
   */
  getCooldownSync(): CooldownCheckResult | null {
    return this.cooldownSubject.value;
  }

  // ===========================================================================
  // Challenge Flow
  // ===========================================================================

  /**
   * Start a new mastery challenge.
   *
   * @param questionCount Number of questions (typically 5-20)
   * @param pathId Optional path to focus on
   * @param includeDiscoveries Whether to include discovery content
   * @param timeLimitSeconds Optional time limit
   */
  startChallenge(
    questionCount: number,
    pathId?: string,
    includeDiscoveries = true,
    timeLimitSeconds?: number
  ): Observable<MasteryChallenge | null> {
    // Check cooldown first
    const cooldown = this.cooldownSubject.value;
    if (cooldown && !cooldown.can_take_challenge) {
      console.warn('[Practice] Cannot start challenge - cooldown active');
      return of(null);
    }

    const input: StartChallengeInput = {
      question_count: questionCount,
      path_id: pathId,
      include_discoveries: includeDiscoveries,
      time_limit_seconds: timeLimitSeconds,
    };

    return from(this.backend.startMasteryChallenge(input)).pipe(
      map(output => output?.challenge ?? null),
      tap(challenge => {
        this.currentChallengeSubject.next(challenge);
      }),
      catchError(err => {
        console.warn('[Practice] Failed to start challenge:', err);
        return of(null);
      })
    );
  }

  /**
   * Submit challenge responses.
   */
  submitChallenge(
    challengeId: string,
    responses: MasteryChallengeResponse[],
    actualTimeSeconds: number
  ): Observable<ChallengeResult | null> {
    return from(this.backend.submitMasteryChallenge({
      challenge_id: challengeId,
      responses,
      actual_time_seconds: actualTimeSeconds,
    })).pipe(
      tap(result => {
        if (result) {
          // Clear current challenge
          this.currentChallengeSubject.next(null);

          // Update pool with new stats
          if (result.challenge?.challenge) {
            const pool = this.poolSubject.value;
            if (pool) {
              // Pool stats are updated on the backend
              this.refreshPool().subscribe();
            }
          }

          // Refresh cooldown
          this.checkCooldown();

          // Add to history
          if (result.challenge?.challenge) {
            const history = this.challengeHistorySubject.value;
            this.challengeHistorySubject.next([result.challenge.challenge, ...history]);
          }
        }
      }),
      catchError(err => {
        console.warn('[Practice] Failed to submit challenge:', err);
        return of(null);
      })
    );
  }

  /**
   * Abandon current challenge without submitting.
   */
  abandonChallenge(): void {
    this.currentChallengeSubject.next(null);
  }

  /**
   * Get current challenge synchronously.
   */
  getCurrentChallengeSync(): MasteryChallenge | null {
    return this.currentChallengeSubject.value;
  }

  /**
   * Check if there's an active challenge.
   */
  hasActiveChallenge(): boolean {
    const challenge = this.currentChallengeSubject.value;
    return challenge !== null && challenge.state === 'in_progress';
  }

  // ===========================================================================
  // Challenge History
  // ===========================================================================

  /**
   * Load challenge history from backend.
   */
  loadChallengeHistory(): Observable<MasteryChallenge[]> {
    return from(this.backend.getChallengeHistory()).pipe(
      map(outputs => outputs.map(o => o.challenge)),
      tap(history => {
        this.challengeHistorySubject.next(history);
      }),
      catchError(err => {
        console.warn('[Practice] Failed to load challenge history:', err);
        return of([]);
      })
    );
  }

  /**
   * Get challenge history synchronously.
   */
  getChallengeHistorySync(): MasteryChallenge[] {
    return this.challengeHistorySubject.value;
  }

  // ===========================================================================
  // Pool Analysis Helpers
  // ===========================================================================

  /**
   * Get count of items in active rotation.
   */
  getActiveCount(): number {
    const pool = this.poolSubject.value;
    if (!pool) return 0;
    const ids = (pool.activeContentIds ?? []) as string[];
    return ids.length;
  }

  /**
   * Get count of items needing refresh.
   */
  getRefreshCount(): number {
    const pool = this.poolSubject.value;
    if (!pool) return 0;
    const ids = (pool.refreshQueueIds ?? []) as string[];
    return ids.length;
  }

  /**
   * Get count of discovery candidates.
   */
  getDiscoveryCount(): number {
    const pool = this.poolSubject.value;
    if (!pool) return 0;
    const candidates = (pool.discoveryCandidates ?? []) as unknown[];
    return candidates.length;
  }

  /**
   * Get pool statistics.
   */
  getPoolStats(): {
    active: number;
    refresh: number;
    discovery: number;
    totalChallenges: number;
    levelUps: number;
    levelDowns: number;
    discoveries: number;
  } {
    const pool = this.poolSubject.value;
    if (!pool) {
      return {
        active: 0,
        refresh: 0,
        discovery: 0,
        totalChallenges: 0,
        levelUps: 0,
        levelDowns: 0,
        discoveries: 0,
      };
    }

    return {
      active: this.getActiveCount(),
      refresh: this.getRefreshCount(),
      discovery: this.getDiscoveryCount(),
      totalChallenges: pool.total_challenges_taken,
      levelUps: pool.total_level_ups,
      levelDowns: pool.total_level_downs,
      discoveries: pool.discoveries_unlocked,
    };
  }
}
