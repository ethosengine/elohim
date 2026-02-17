import { CommonModule } from '@angular/common';
import { Component, OnInit, OnDestroy, inject } from '@angular/core';
import { RouterModule } from '@angular/router';

import { Subscription } from 'rxjs';

import {
  getMasteryColor,
  getMasteryIcon,
  getMasteryLabel,
} from '../../models/mastery-visualization';
import { MasteryStatsService } from '../../services/mastery-stats.service';

import type { MasteryLevel } from '../../models/content-mastery.model';
import type { LearnerMasteryProfile } from '../../models/learner-mastery-profile.model';

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
 */
@Component({
  selector: 'app-learner-dashboard',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './learner-dashboard.component.html',
  styleUrls: ['./learner-dashboard.component.css'],
})
export class LearnerDashboardComponent implements OnInit, OnDestroy {
  private readonly masteryStats = inject(MasteryStatsService);
  private subscription: Subscription | null = null;

  profile: LearnerMasteryProfile | null = null;
  isLoading = true;

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

  ngOnInit(): void {
    this.subscription = this.masteryStats.learnerProfile$.subscribe(profile => {
      this.profile = profile;
      this.isLoading = false;
    });

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
