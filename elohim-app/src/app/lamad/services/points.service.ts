/**
 * Points Service
 *
 * Learning economy integration with Shefa economic primitives.
 * Manages learner point balance, history, and recognition flows.
 *
 * Key concepts:
 * - Points earned through learning activities (view, practice, challenge, etc.)
 * - Recognition flows to content contributors via Shefa Appreciation
 * - Point triggers map to specific learning events
 *
 * Patterns:
 * - BehaviorSubject for reactive state
 * - Observable APIs for async operations
 * - Delegates to LearnerBackendService for zome calls
 */

import { Injectable } from '@angular/core';
import { BehaviorSubject, Observable, from, of } from 'rxjs';
import { map, tap, catchError } from 'rxjs/operators';

import { LearnerBackendService } from '@app/elohim/services/learner-backend.service';

import type {
  LearnerPointBalance,
  LamadPointEvent,
  EarnLamadPointsInput,
  EarnLamadPointsResult,
  LamadPointTrigger,
} from '../models/learning-points.model';

import {
  parsePointsByTrigger,
  getPointAmount,
  formatPoints,
  getTriggerLabel,
} from '../models/learning-points.model';

@Injectable({
  providedIn: 'root',
})
export class PointsService {
  // ===========================================================================
  // Reactive State
  // ===========================================================================

  private readonly balanceSubject = new BehaviorSubject<LearnerPointBalance | null>(null);
  private readonly historySubject = new BehaviorSubject<LamadPointEvent[]>([]);
  private readonly loadingSubject = new BehaviorSubject<boolean>(false);

  /** Current point balance */
  readonly balance$ = this.balanceSubject.asObservable();

  /** Point event history */
  readonly history$ = this.historySubject.asObservable();

  /** Loading state */
  readonly loading$ = this.loadingSubject.asObservable();

  /** Total points as observable */
  readonly totalPoints$ = this.balance$.pipe(
    map(balance => balance?.total_points ?? 0)
  );

  constructor(private readonly backend: LearnerBackendService) {}

  // ===========================================================================
  // Balance Management
  // ===========================================================================

  /**
   * Get current balance as observable.
   * Automatically refreshes if not loaded.
   */
  getBalance$(): Observable<LearnerPointBalance | null> {
    if (!this.balanceSubject.value) {
      this.refreshBalance();
    }
    return this.balance$;
  }

  /**
   * Refresh balance from backend.
   */
  refreshBalance(): void {
    this.loadingSubject.next(true);

    from(this.backend.getMyLamadPointBalance()).pipe(
      tap(output => {
        this.balanceSubject.next(output?.balance ?? null);
        this.loadingSubject.next(false);
      }),
      catchError(err => {
        console.warn('[Points] Failed to get balance:', err);
        this.loadingSubject.next(false);
        return of(null);
      })
    ).subscribe();
  }

  /**
   * Get balance synchronously.
   */
  getBalanceSync(): LearnerPointBalance | null {
    return this.balanceSubject.value;
  }

  /**
   * Get total points synchronously.
   */
  getTotalPointsSync(): number {
    return this.balanceSubject.value?.total_points ?? 0;
  }

  // ===========================================================================
  // Point Earning
  // ===========================================================================

  /**
   * Earn points for a learning activity.
   * Points automatically flow recognition to content contributors.
   *
   * @param trigger The learning activity type
   * @param contentId Optional content associated with the activity
   * @param pathId Optional path context
   * @param challengeId Optional challenge context
   * @param wasCorrect Whether answer was correct (for challenge events)
   * @param note Optional note
   */
  earnPoints(
    trigger: LamadPointTrigger,
    contentId?: string,
    pathId?: string,
    challengeId?: string,
    wasCorrect?: boolean,
    note?: string
  ): Observable<EarnLamadPointsResult | null> {
    const input: EarnLamadPointsInput = {
      trigger,
      content_id: contentId,
      path_id: pathId,
      challenge_id: challengeId,
      was_correct: wasCorrect,
      note,
    };

    return from(this.backend.earnLamadPoints(input)).pipe(
      tap(result => {
        if (result) {
          // Update balance
          this.balanceSubject.next(result.new_balance.balance);

          // Add event to history
          const history = this.historySubject.value;
          this.historySubject.next([result.point_event.event, ...history]);
        }
      }),
      catchError(err => {
        console.warn('[Points] Failed to earn points:', err);
        return of(null);
      })
    );
  }

  /**
   * Earn points for viewing content.
   */
  earnViewPoints(contentId: string): Observable<EarnLamadPointsResult | null> {
    return this.earnPoints('engagement_view', contentId);
  }

  /**
   * Earn points for practicing content.
   */
  earnPracticePoints(contentId: string): Observable<EarnLamadPointsResult | null> {
    return this.earnPoints('engagement_practice', contentId);
  }

  /**
   * Earn points for completing a path step.
   */
  earnPathStepPoints(pathId: string, contentId?: string): Observable<EarnLamadPointsResult | null> {
    return this.earnPoints('path_step_complete', contentId, pathId);
  }

  /**
   * Earn points for completing a path.
   */
  earnPathCompletePoints(pathId: string): Observable<EarnLamadPointsResult | null> {
    return this.earnPoints('path_complete', undefined, pathId);
  }

  // ===========================================================================
  // History
  // ===========================================================================

  /**
   * Get point history as observable.
   */
  getHistory$(limit?: number): Observable<LamadPointEvent[]> {
    return this.history$.pipe(
      map(history => limit ? history.slice(0, limit) : history)
    );
  }

  /**
   * Load history from backend.
   */
  loadHistory(limit?: number): Observable<LamadPointEvent[]> {
    return from(this.backend.getMyLamadPointHistory(limit)).pipe(
      map(outputs => outputs.map(o => o.event)),
      tap(history => {
        this.historySubject.next(history);
      }),
      catchError(err => {
        console.warn('[Points] Failed to load history:', err);
        return of([]);
      })
    );
  }

  /**
   * Get history synchronously.
   */
  getHistorySync(): LamadPointEvent[] {
    return this.historySubject.value;
  }

  // ===========================================================================
  // Analytics
  // ===========================================================================

  /**
   * Get points breakdown by trigger as observable.
   */
  getPointsByTrigger$(): Observable<Record<string, number>> {
    return this.balance$.pipe(
      map(balance => {
        if (!balance) return {};
        return parsePointsByTrigger(balance.pointsByTriggerJson);
      })
    );
  }

  /**
   * Get points breakdown by trigger synchronously.
   */
  getPointsByTriggerSync(): Record<string, number> {
    const balance = this.balanceSubject.value;
    if (!balance) return {};
    return parsePointsByTrigger(balance.pointsByTriggerJson);
  }

  /**
   * Get total earned (lifetime) points.
   */
  getTotalEarnedSync(): number {
    return this.balanceSubject.value?.total_earned ?? 0;
  }

  /**
   * Get total spent points.
   */
  getTotalSpentSync(): number {
    return this.balanceSubject.value?.total_spent ?? 0;
  }

  /**
   * Get recent point events count.
   */
  getRecentEventsCount(days = 7): number {
    const history = this.historySubject.value;
    const cutoff = new Date();
    cutoff.setDate(cutoff.getDate() - days);
    const cutoffStr = cutoff.toISOString();

    return history.filter(event => event.occurred_at >= cutoffStr).length;
  }

  /**
   * Get points earned in recent period.
   */
  getRecentPointsEarned(days = 7): number {
    const history = this.historySubject.value;
    const cutoff = new Date();
    cutoff.setDate(cutoff.getDate() - days);
    const cutoffStr = cutoff.toISOString();

    return history
      .filter(event => event.occurred_at >= cutoffStr && event.points > 0)
      .reduce((sum, event) => sum + event.points, 0);
  }

  // ===========================================================================
  // Display Helpers
  // ===========================================================================

  /**
   * Format points for display with sign.
   */
  formatPoints(points: number): string {
    return formatPoints(points);
  }

  /**
   * Get display label for a trigger.
   */
  getTriggerLabel(trigger: LamadPointTrigger): string {
    return getTriggerLabel(trigger);
  }

  /**
   * Get expected point amount for a trigger.
   */
  getPointAmount(trigger: LamadPointTrigger): number {
    return getPointAmount(trigger);
  }
}
