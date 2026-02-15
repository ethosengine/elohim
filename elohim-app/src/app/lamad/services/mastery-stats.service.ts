/**
 * Mastery Stats Service
 *
 * Aggregation layer composing mastery + points + practice into a
 * gamified learner profile for the dashboard (/lamad/me).
 *
 * Key responsibilities:
 * - Compose LearnerMasteryProfile from multiple upstream sources
 * - Track daily engagement streaks on source chain
 * - Record level-up events and bridge to points economy
 * - Provide reactive profile state for dashboard binding
 *
 * Patterns:
 * - BehaviorSubject for reactive state (same as PointsService, PracticeService)
 * - Subscribes to ContentMasteryService.levelUp$ for orchestration
 * - Idempotent daily streak records (one entry per day)
 */

import { Injectable, OnDestroy } from '@angular/core';

import { BehaviorSubject, Observable, Subscription, combineLatest } from 'rxjs';

import { isAboveGate } from '@app/elohim/models/agent.model';
import { LocalSourceChainService } from '@app/elohim/services/local-source-chain.service';
import { SessionHumanService } from '@app/imagodei/services/session-human.service';

import {
  MASTERY_XP_WEIGHTS,
  getLearnerLevel,
  getLearnerLevelProgress,
} from '../models/learner-mastery-profile.model';

import { ContentMasteryService } from './content-mastery.service';
import { PointsService } from './points.service';
import { PracticeService } from './practice.service';

import type { ContentMastery, MasteryLevel } from '../models';
import type {
  DashboardPathsOverview,
  LearnerMasteryProfile,
  LevelUpEvent,
  MasteryLevelUpContent,
  PracticeSummary,
  StreakInfo,
  StreakRecordContent,
} from '../models/learner-mastery-profile.model';

const ENTRY_TYPE_STREAK = 'streak-record' as const;
const ENTRY_TYPE_LEVEL_UP = 'mastery-level-up' as const;

@Injectable({ providedIn: 'root' })
export class MasteryStatsService implements OnDestroy {
  // ===========================================================================
  // Reactive State
  // ===========================================================================

  private readonly profileSubject = new BehaviorSubject<LearnerMasteryProfile | null>(null);
  private readonly streakSubject = new BehaviorSubject<StreakInfo>(this.emptyStreak());
  private readonly recentLevelUpsSubject = new BehaviorSubject<LevelUpEvent[]>([]);
  private readonly subscriptions = new Subscription();

  /** Aggregated learner profile */
  readonly learnerProfile$: Observable<LearnerMasteryProfile | null> =
    this.profileSubject.asObservable();

  /** Current streak info */
  readonly streakInfo$: Observable<StreakInfo> = this.streakSubject.asObservable();

  /** Recent level-up events */
  readonly recentLevelUps$: Observable<LevelUpEvent[]> = this.recentLevelUpsSubject.asObservable();

  constructor(
    private readonly contentMastery: ContentMasteryService,
    private readonly points: PointsService,
    private readonly sourceChain: LocalSourceChainService,
    private readonly sessionHuman: SessionHumanService,
    private readonly practice: PracticeService
  ) {
    this.initializeSubscriptions();
  }

  ngOnDestroy(): void {
    this.subscriptions.unsubscribe();
  }

  // ===========================================================================
  // Initialization
  // ===========================================================================

  private initializeSubscriptions(): void {
    // Recompute profile when mastery or points change
    this.subscriptions.add(
      combineLatest([this.contentMastery.mastery$, this.points.totalPoints$]).subscribe(
        ([masteryMap, earnedPoints]) => {
          this.recomputeProfile(masteryMap, earnedPoints);
        }
      )
    );

    // Orchestrate level-up events
    this.subscriptions.add(
      this.contentMastery.levelUp$.subscribe(event => {
        this.handleLevelUp(event);
      })
    );

    // Initialize streak from source chain when session starts
    this.subscriptions.add(
      this.sessionHuman.session$.subscribe(session => {
        if (session) {
          this.loadStreakFromChain();
          this.loadLevelUpsFromChain();
        }
      })
    );
  }

  // ===========================================================================
  // Profile Computation
  // ===========================================================================

  /**
   * Recompute the full learner profile from upstream sources.
   */
  private recomputeProfile(masteryMap: Map<string, ContentMastery>, earnedPoints: number): void {
    const levelDistribution = this.computeLevelDistribution(masteryMap);
    const masteryXP = this.computeMasteryXP(masteryMap);
    const totalXP = earnedPoints + masteryXP;
    const streak = this.streakSubject.value;
    const recentLevelUps = this.recentLevelUpsSubject.value;

    let totalMasteredNodes = 0;
    let nodesAboveGate = 0;

    for (const [, mastery] of masteryMap) {
      if (mastery.level !== 'not_started') {
        totalMasteredNodes++;
        if (isAboveGate(mastery.level)) {
          nodesAboveGate++;
        }
      }
    }

    const profile: LearnerMasteryProfile = {
      learnerLevel: getLearnerLevel(totalXP),
      levelProgress: getLearnerLevelProgress(totalXP),
      totalXP,
      earnedPoints,
      masteryXP,
      levelDistribution,
      totalMasteredNodes,
      nodesAboveGate,
      streak,
      recentLevelUps: recentLevelUps.slice(0, 5),
      practice: this.getPracticeSummary(),
      paths: this.getPathsOverview(),
      computedAt: new Date().toISOString(),
    };

    this.profileSubject.next(profile);
  }

  /**
   * Force a full profile refresh.
   */
  refreshProfile(): void {
    this.loadStreakFromChain();
    this.loadLevelUpsFromChain();
    this.points.refreshBalance();
    // Profile will recompute via combineLatest subscription
  }

  /**
   * Get profile synchronously from cache.
   */
  getProfileSync(): LearnerMasteryProfile | null {
    return this.profileSubject.value;
  }

  // ===========================================================================
  // Mastery XP Computation
  // ===========================================================================

  private computeMasteryXP(masteryMap: Map<string, ContentMastery>): number {
    let total = 0;
    for (const [, mastery] of masteryMap) {
      total += MASTERY_XP_WEIGHTS[mastery.level] ?? 0;
    }
    return total;
  }

  private computeLevelDistribution(
    masteryMap: Map<string, ContentMastery>
  ): Record<MasteryLevel, number> {
    const dist: Record<MasteryLevel, number> = {
      not_started: 0,
      seen: 0,
      remember: 0,
      understand: 0,
      apply: 0,
      analyze: 0,
      evaluate: 0,
      create: 0,
    };

    for (const [, mastery] of masteryMap) {
      dist[mastery.level]++;
    }

    return dist;
  }

  // ===========================================================================
  // Streak Management
  // ===========================================================================

  /**
   * Record daily engagement. Idempotent - one entry per calendar day.
   */
  recordDailyEngagement(engagementType: string): void {
    if (!this.sourceChain.isInitialized()) return;

    const streak = this.streakSubject.value;

    // Already recorded today
    if (streak.todayActive) return;

    const today = this.getToday();
    const content: StreakRecordContent = {
      activeDate: today,
      engagementTypes: [engagementType],
    };

    this.sourceChain.createEntry(ENTRY_TYPE_STREAK, content);

    // Update streak state
    const updated = this.computeStreakFromRecords();
    this.streakSubject.next(updated);

    // Trigger profile recomputation
    const profile = this.profileSubject.value;
    if (profile) {
      this.profileSubject.next({ ...profile, streak: updated });
    }
  }

  private loadStreakFromChain(): void {
    if (!this.sourceChain.isInitialized()) return;
    const streak = this.computeStreakFromRecords();
    this.streakSubject.next(streak);
  }

  private computeStreakFromRecords(): StreakInfo {
    const entries = this.sourceChain.getEntriesByType<StreakRecordContent>(ENTRY_TYPE_STREAK);

    if (entries.length === 0) {
      return this.emptyStreak();
    }

    // Collect unique active dates
    const activeDates = new Set<string>();
    for (const entry of entries) {
      activeDates.add(entry.content.activeDate);
    }

    const today = this.getToday();
    const todayActive = activeDates.has(today);

    // Build recent activity map (last 30 days)
    const recentActivity: Record<string, boolean> = {};
    for (let i = 0; i < 30; i++) {
      const date = this.getDateOffset(-i);
      if (activeDates.has(date)) {
        recentActivity[date] = true;
      }
    }

    // Compute current streak (consecutive days ending today or yesterday)
    let currentStreak = 0;
    const startDate = todayActive ? today : this.getDateOffset(-1);
    if (activeDates.has(startDate)) {
      let checkDate = startDate;
      while (activeDates.has(checkDate)) {
        currentStreak++;
        checkDate = this.offsetDate(checkDate, -1);
      }
    }

    // Compute best streak
    const sortedDates = Array.from(activeDates).sort((a, b) => a.localeCompare(b));
    let bestStreak = 0;
    let runLength = 1;

    for (let i = 1; i < sortedDates.length; i++) {
      if (this.offsetDate(sortedDates[i - 1], 1) === sortedDates[i]) {
        runLength++;
      } else {
        bestStreak = Math.max(bestStreak, runLength);
        runLength = 1;
      }
    }
    bestStreak = Math.max(bestStreak, runLength);

    // Streak start date
    let streakStartDate = startDate;
    if (currentStreak > 0) {
      streakStartDate = this.offsetDate(startDate, -(currentStreak - 1));
    }

    return {
      currentStreak,
      bestStreak,
      todayActive,
      lastActiveDate: sortedDates.at(-1)!,
      streakStartDate,
      recentActivity,
    };
  }

  private emptyStreak(): StreakInfo {
    return {
      currentStreak: 0,
      bestStreak: 0,
      todayActive: false,
      lastActiveDate: '',
      streakStartDate: '',
      recentActivity: {},
    };
  }

  // ===========================================================================
  // Level-Up Orchestration
  // ===========================================================================

  /**
   * Handle a level-up event from ContentMasteryService.
   * 1. Persist to source chain
   * 2. Earn points
   * 3. Record daily engagement
   * 4. Update recent level-ups list
   */
  private handleLevelUp(event: LevelUpEvent): void {
    // 1. Persist level-up event to source chain
    if (this.sourceChain.isInitialized()) {
      const content: MasteryLevelUpContent = {
        contentId: event.contentId,
        fromLevel: event.fromLevel,
        toLevel: event.toLevel,
        pointsEarned: event.pointsEarned,
        isGateLevel: event.isGateLevel,
      };
      this.sourceChain.createEntry(ENTRY_TYPE_LEVEL_UP, content);
    }

    // 2. Earn points for level-up
    this.points.earnPoints('level_up', event.contentId).subscribe();

    // 3. Record daily engagement for streak
    this.recordDailyEngagement('level_up');

    // 4. Update recent level-ups
    const current = this.recentLevelUpsSubject.value;
    this.recentLevelUpsSubject.next([event, ...current].slice(0, 10));
  }

  /**
   * Record a level-up event manually (for external callers).
   */
  recordLevelUp(event: LevelUpEvent): void {
    this.handleLevelUp(event);
  }

  private loadLevelUpsFromChain(): void {
    if (!this.sourceChain.isInitialized()) return;

    const entries = this.sourceChain.getEntriesByType<MasteryLevelUpContent>(ENTRY_TYPE_LEVEL_UP);

    const levelUps: LevelUpEvent[] = entries
      .map(entry => ({
        id: entry.entryHash,
        contentId: entry.content.contentId,
        fromLevel: entry.content.fromLevel as MasteryLevel,
        toLevel: entry.content.toLevel as MasteryLevel,
        timestamp: entry.timestamp,
        pointsEarned: entry.content.pointsEarned,
        isGateLevel: entry.content.isGateLevel,
      }))
      .reverse()
      .slice(0, 10);

    this.recentLevelUpsSubject.next(levelUps);
  }

  // ===========================================================================
  // Practice Summary
  // ===========================================================================

  private getPracticeSummary(): PracticeSummary {
    // Access pool via synchronous BehaviorSubject value
    const poolSubject = this.practice.pool$ as unknown as BehaviorSubject<unknown>;
    const pool = poolSubject?.value;

    if (!pool || typeof pool !== 'object') {
      return {
        totalChallenges: 0,
        totalLevelUps: 0,
        totalLevelDowns: 0,
        totalDiscoveries: 0,
        activePoolSize: 0,
        refreshQueueSize: 0,
      };
    }

    const p = pool as {
      total_challenges_taken?: number;
      total_level_ups?: number;
      total_level_downs?: number;
      discoveries_unlocked?: number;
      active_content_ids_json?: string;
      refresh_queue_ids_json?: string;
    };

    return {
      totalChallenges: p.total_challenges_taken ?? 0,
      totalLevelUps: p.total_level_ups ?? 0,
      totalLevelDowns: p.total_level_downs ?? 0,
      totalDiscoveries: p.discoveries_unlocked ?? 0,
      activePoolSize: this.parseJsonArray(p.active_content_ids_json).length,
      refreshQueueSize: this.parseJsonArray(p.refresh_queue_ids_json).length,
    };
  }

  private parseJsonArray(json?: string): unknown[] {
    if (!json) return [];
    try {
      return JSON.parse(json) as unknown[];
    } catch {
      return [];
    }
  }

  // ===========================================================================
  // Paths Overview
  // ===========================================================================

  private getPathsOverview(): DashboardPathsOverview {
    // Paths data comes from PathService which is outside this aggregation scope.
    // Dashboard component composes with PathService directly for path entries.
    return {
      inProgress: [],
      completed: [],
    };
  }

  // ===========================================================================
  // Date Helpers
  // ===========================================================================

  private getToday(): string {
    return new Date().toISOString().slice(0, 10);
  }

  private getDateOffset(days: number): string {
    const d = new Date();
    d.setDate(d.getDate() + days);
    return d.toISOString().slice(0, 10);
  }

  private offsetDate(dateStr: string, days: number): string {
    const d = new Date(dateStr + 'T00:00:00');
    d.setDate(d.getDate() + days);
    return d.toISOString().slice(0, 10);
  }
}
