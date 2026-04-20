import { CommonModule } from '@angular/common';
import { Component, OnInit, OnDestroy, inject } from '@angular/core';
import { RouterModule } from '@angular/router';

// @coverage: 94.9% (2026-02-24)

import { Subscription, forkJoin } from 'rxjs';

import { AgentService } from '@app/elohim/services/agent.service';

import {
  getMasteryColor,
  getMasteryIcon,
  getMasteryLabel,
} from '../../models/mastery-visualization';
import { ContentMasteryService } from '../../services/content-mastery.service';
import { MasteryStatsService } from '../../services/mastery-stats.service';
import { PathService } from '../../services/path.service';
import { AttentionFlowComponent } from '../attention-flow/attention-flow.component';

import { RefreshQueueComponent } from './refresh-queue/refresh-queue.component';
import { StatsRowComponent } from './stats-row/stats-row.component';

import type { MasteryLevel } from '../../models/content-mastery.model';
import type {
  DashboardPathEntry,
  DashboardPathsOverview,
  LearnerMasteryProfile,
} from '../../models/learner-mastery-profile.model';

/**
 * LearnerDashboardComponent - Gamified personal learning dashboard.
 *
 * Route: /lamad/me
 *
 * Displays:
 * - Learner level badge + XP progress bar
 * - Engagement streak with 30-day activity dots
 * - Mastery distribution (Bloom's taxonomy bars)
 * - Active paths with progress
 * - Recent level-up timeline
 * - Practice challenge stats
 * - Stats overview (mastered nodes, gate, freshness, XP)
 * - Practice queue (spaced repetition refresh)
 */
@Component({
  selector: 'app-learner-dashboard',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    AttentionFlowComponent,
    StatsRowComponent,
    RefreshQueueComponent,
  ],
  templateUrl: './learner-dashboard.component.html',
  styleUrls: ['./learner-dashboard.component.css'],
})
export class LearnerDashboardComponent implements OnInit, OnDestroy {
  private readonly masteryStats = inject(MasteryStatsService);
  private readonly contentMastery = inject(ContentMasteryService);
  private readonly pathService = inject(PathService);
  private readonly agentService = inject(AgentService);
  private subscription: Subscription | null = null;

  profile: LearnerMasteryProfile | null = null;
  isLoading = true;
  freshnessStats = { healthPercent: 100, fresh: 0, stale: 0, critical: 0 };

  /** Mastery levels for the distribution bars (skip not_started) */
  readonly masteryLevels: MasteryLevel[] = [
    'seen',
    'remember',
    'understand',
    'apply',
    'analyze',
    'evaluate',
    'create',
  ];

  /** Last 30 days as YYYY-MM-DD strings for streak dots */
  readonly last30Days: string[] = this.buildLast30Days();

  // Expose visualization helpers to template
  readonly getMasteryColor = getMasteryColor;
  readonly getMasteryIcon = getMasteryIcon;
  readonly getMasteryLabel = getMasteryLabel;

  get xpEarnedThisWeek(): number {
    if (!this.profile?.recentLevelUps) return 0;
    const weekAgo = new Date();
    weekAgo.setDate(weekAgo.getDate() - 7);
    return this.profile.recentLevelUps
      .filter(e => new Date(e.timestamp) >= weekAgo)
      .reduce((sum, e) => sum + e.pointsEarned, 0);
  }

  ngOnInit(): void {
    this.subscription = this.masteryStats.learnerProfile$.subscribe(profile => {
      if (!profile) {
        this.profile = profile;
        this.isLoading = false;
        return;
      }
      // Enrich profile with paths data from PathService + AgentService
      this.loadPathsData(profile);
    });

    this.subscription.add(
      this.contentMastery.getAllMastery().subscribe(all => {
        if (all.length === 0) {
          this.freshnessStats = { healthPercent: 100, fresh: 0, stale: 0, critical: 0 };
          return;
        }
        let fresh = 0,
          stale = 0,
          critical = 0;
        for (const m of all) {
          if (m.freshness >= 0.7) fresh++;
          else if (m.freshness >= 0.3) stale++;
          else critical++;
        }
        const total = fresh + stale + critical;
        this.freshnessStats = {
          healthPercent: total > 0 ? Math.round((fresh / total) * 100) : 100,
          fresh,
          stale,
          critical,
        };
      })
    );

    // Record dashboard visit as daily engagement
    this.masteryStats.recordDailyEngagement('dashboard_visit');
  }

  ngOnDestroy(): void {
    this.subscription?.unsubscribe();
  }

  /**
   * Compute bar width percentage for a mastery level.
   * Relative to the max count across all levels.
   */
  getBarWidth(level: MasteryLevel): number {
    if (!this.profile) return 0;

    const count = this.profile.levelDistribution[level] ?? 0;
    if (count === 0) return 0;

    let max = 0;
    for (const l of this.masteryLevels) {
      const c = this.profile.levelDistribution[l] ?? 0;
      if (c > max) max = c;
    }

    return max > 0 ? (count / max) * 100 : 0;
  }

  /**
   * Load paths data from AgentService + PathService and merge into profile.
   * AgentService.getAgentProgress() gives all started paths (from localStorage).
   * PathService.listPaths() gives path metadata (titles, step counts).
   */
  private loadPathsData(profile: LearnerMasteryProfile): void {
    forkJoin({
      progressList: this.agentService.getAgentProgress(),
      pathIndex: this.pathService.listPaths(),
    }).subscribe({
      next: ({ progressList, pathIndex }) => {
        const pathMap = new Map(pathIndex.paths.map(p => [p.id, p]));
        const inProgress: DashboardPathEntry[] = [];
        const completed: DashboardPathEntry[] = [];

        for (const progress of progressList) {
          // Skip the __global__ pseudo-path used for cross-path tracking
          if (progress.pathId === '__global__') continue;

          const meta = pathMap.get(progress.pathId);
          if (!meta) continue;

          const completedSteps = progress.completedStepIndices.length;
          const totalSteps = meta.stepCount;
          const progressPercent =
            totalSteps > 0 ? Math.round((completedSteps / totalSteps) * 100) : 0;

          const entry: DashboardPathEntry = {
            pathId: progress.pathId,
            title: meta.title,
            progressPercent,
            completedSteps,
            totalSteps,
            lastActiveAt: progress.lastActivityAt,
          };

          if (progress.completedAt) {
            completed.push(entry);
          } else {
            inProgress.push(entry);
          }
        }

        // Sort by most recent activity
        inProgress.sort(
          (a, b) => new Date(b.lastActiveAt).getTime() - new Date(a.lastActiveAt).getTime()
        );
        completed.sort(
          (a, b) => new Date(b.lastActiveAt).getTime() - new Date(a.lastActiveAt).getTime()
        );

        const paths: DashboardPathsOverview = { inProgress, completed };
        this.profile = { ...profile, paths };
        this.isLoading = false;
      },
      error: () => {
        // Fall back to profile without enriched paths
        this.profile = profile;
        this.isLoading = false;
      },
    });
  }

  private buildLast30Days(): string[] {
    const days: string[] = [];
    for (let i = 29; i >= 0; i--) {
      const d = new Date();
      d.setDate(d.getDate() - i);
      days.push(d.toISOString().slice(0, 10));
    }
    return days;
  }
}
