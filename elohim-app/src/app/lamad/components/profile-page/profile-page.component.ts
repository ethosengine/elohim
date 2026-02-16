import { CommonModule } from '@angular/common';
import { Component, OnInit, OnDestroy, inject, computed } from '@angular/core';
import { RouterModule, Router } from '@angular/router';

// @coverage: 45.5% (2026-02-05)

import { takeUntil, catchError } from 'rxjs/operators';

import { Subject, of } from 'rxjs';

import { ProfileService } from '@app/elohim/services/profile.service';
import { isNetworkMode } from '@app/imagodei/models/identity.model';
import { ResumePoint, PathsOverview, TimelineEvent } from '@app/imagodei/models/profile.model';
import {
  SessionHuman,
  SessionActivity,
  SessionPathProgress,
} from '@app/imagodei/models/session-human.model';
import { IdentityService } from '@app/imagodei/services/identity.service';
import { SessionHumanService } from '@app/imagodei/services/session-human.service';

import { MasteryStats, MasteryLevel } from '../../models';
import { ContentMasteryService } from '../../services/content-mastery.service';
import { MasteryStatsService } from '../../services/mastery-stats.service';

import type { LearnerMasteryProfile } from '../../models/learner-mastery-profile.model';

/**
 * ProfilePageComponent - Learning profile for the lamad pillar.
 *
 * Shows learning-only content: stats, paths, mastery, timeline.
 * Identity management (profile editing, network, agency) lives in imagodei.
 *
 * Tabs:
 * 1. Overview - Resume point, current focus, capabilities, journey stats
 * 2. Paths - In progress, completed, suggested
 * 3. Timeline - Chronological events with type filter
 */
@Component({
  selector: 'app-profile-page',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './profile-page.component.html',
  styleUrls: ['./profile-page.component.css'],
})
export class ProfilePageComponent implements OnInit, OnDestroy {
  readonly identityService = inject(IdentityService); // Public for template access
  private readonly router = inject(Router);

  // Tab management
  activeTab: 'overview' | 'paths' | 'timeline' = 'overview';

  // ==========================================================================
  // Identity State (from IdentityService - unified source of truth)
  // ==========================================================================

  /** Whether user is authenticated via Holochain (hosted or steward) */
  readonly isNetworkAuthenticated = computed(() => {
    const mode = this.identityService.mode();
    return isNetworkMode(mode);
  });

  /** Current identity mode */
  readonly identityMode = this.identityService.mode;

  /** Display name from identity (prefers Holochain, falls back to session) */
  readonly displayName = computed(() => {
    if (this.isNetworkAuthenticated()) {
      return this.identityService.displayName();
    }
    return this.session?.displayName ?? 'Traveler';
  });

  /** Badge text based on identity mode */
  readonly modeBadgeText = computed(() => {
    const mode = this.identityMode();
    switch (mode) {
      case 'hosted':
        return 'Hosted Human';
      case 'steward':
        return 'Steward';
      case 'session':
        return 'Session Visitor';
      case 'anonymous':
        return 'Anonymous';
      case 'migrating':
        return 'Migrating...';
      default:
        return 'Visitor';
    }
  });

  /** Badge icon based on identity mode */
  readonly modeBadgeIcon = computed(() => {
    const mode = this.identityMode();
    switch (mode) {
      case 'hosted':
        return 'cloud_done';
      case 'steward':
        return 'verified_user';
      case 'session':
        return 'local_activity';
      case 'migrating':
        return 'sync';
      default:
        return 'person';
    }
  });

  // Mastery stats aggregation (gamified profile)
  private readonly masteryStatsService = inject(MasteryStatsService);

  // Session data
  session: SessionHuman | null = null;

  // Stats
  masteryStats: MasteryStats | null = null;
  learnerProfile: LearnerMasteryProfile | null = null;
  pathProgressList: SessionPathProgress[] = [];
  activityHistory: SessionActivity[] = [];

  // ProfileService data
  resumePoint: ResumePoint | null = null;
  pathsOverview: PathsOverview | null = null;
  timelineEvents: TimelineEvent[] = [];

  // Mastery breakdown
  masteryByLevel: { level: MasteryLevel; count: number; label: string }[] = [];

  // Activity filter
  activityFilter: 'all' | 'view' | 'affinity' | 'path-start' | 'path-complete' | 'step-complete' =
    'all';
  filteredActivities: SessionActivity[] = [];

  // Loading states
  isLoading = true;

  private readonly destroy$ = new Subject<void>();

  // Level labels for display
  readonly levelLabels: Record<MasteryLevel, string> = {
    not_started: 'Not Started',
    seen: 'Seen',
    remember: 'Remember',
    understand: 'Understand',
    apply: 'Apply',
    analyze: 'Analyze',
    evaluate: 'Evaluate',
    create: 'Create',
  };

  constructor(
    private readonly sessionHumanService: SessionHumanService,
    private readonly contentMasteryService: ContentMasteryService,
    private readonly profileService: ProfileService
  ) {}

  ngOnInit(): void {
    this.loadData();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  // =========================================================================
  // DATA LOADING
  // =========================================================================

  private loadData(): void {
    this.isLoading = true;

    // Subscribe to session changes
    this.sessionHumanService.session$.pipe(takeUntil(this.destroy$)).subscribe(session => {
      this.session = session;
    });

    // Load path progress
    this.pathProgressList = this.sessionHumanService.getAllPathProgress();

    // Load activity history
    this.activityHistory = this.sessionHumanService.getActivityHistory();
    this.applyActivityFilter();

    // Subscribe to mastery stats
    this.contentMasteryService
      .getMasteryStats()
      .pipe(takeUntil(this.destroy$))
      .subscribe(stats => {
        this.masteryStats = stats;
        this.computeMasteryBreakdown(stats);
        this.isLoading = false;
      });

    // Subscribe to gamified learner profile
    this.masteryStatsService.learnerProfile$.pipe(takeUntil(this.destroy$)).subscribe(profile => {
      this.learnerProfile = profile;
    });

    // Load resume point from ProfileService
    this.profileService
      .getResumePoint()
      .pipe(
        takeUntil(this.destroy$),
        catchError(() => of(null))
      )
      .subscribe(resumePoint => {
        this.resumePoint = resumePoint;
      });

    // Load paths overview from ProfileService
    this.profileService
      .getPathsOverview()
      .pipe(
        takeUntil(this.destroy$),
        catchError(() => of({ inProgress: [], completed: [], suggested: [] }))
      )
      .subscribe(overview => {
        this.pathsOverview = overview;
      });

    // Load timeline from ProfileService
    this.profileService
      .getTimeline(50)
      .pipe(
        takeUntil(this.destroy$),
        catchError(() => of([]))
      )
      .subscribe(events => {
        this.timelineEvents = events;
      });
  }

  private computeMasteryBreakdown(stats: MasteryStats): void {
    const levels: MasteryLevel[] = [
      'seen',
      'remember',
      'understand',
      'apply',
      'analyze',
      'evaluate',
      'create',
    ];

    this.masteryByLevel = levels
      .map(level => ({
        level,
        count: stats.levelDistribution[level],
        label: this.levelLabels[level],
      }))
      .filter(item => item.count > 0);
  }

  // =========================================================================
  // TAB NAVIGATION
  // =========================================================================

  setActiveTab(tab: 'overview' | 'paths' | 'timeline'): void {
    this.activeTab = tab;

    // Refresh data when switching tabs
    if (tab === 'timeline') {
      this.activityHistory = this.sessionHumanService.getActivityHistory();
      this.applyActivityFilter();
    } else if (tab === 'paths') {
      this.pathProgressList = this.sessionHumanService.getAllPathProgress();
    }
  }

  // =========================================================================
  // NAVIGATION
  // =========================================================================

  /** Navigate to identity profile for editing */
  goToIdentityProfile(): void {
    void this.router.navigate(['/identity/profile']);
  }

  /** Navigate to registration for network upgrade */
  onJoinNetwork(): void {
    void this.router.navigate(['/identity/register']);
  }

  // =========================================================================
  // ACTIVITY TIMELINE
  // =========================================================================

  setActivityFilter(filter: typeof this.activityFilter): void {
    this.activityFilter = filter;
    this.applyActivityFilter();
  }

  private applyActivityFilter(): void {
    if (this.activityFilter === 'all') {
      this.filteredActivities = this.activityHistory;
    } else {
      this.filteredActivities = this.activityHistory.filter(a => a.type === this.activityFilter);
    }

    // Sort by most recent first
    this.filteredActivities = [...this.filteredActivities].reverse();
  }

  getActivityIcon(type: SessionActivity['type']): string {
    switch (type) {
      case 'view':
        return 'visibility';
      case 'affinity':
        return 'favorite';
      case 'path-start':
        return 'play_arrow';
      case 'path-complete':
        return 'check_circle';
      case 'step-complete':
        return 'done';
      case 'explore':
        return 'explore';
      default:
        return 'lens';
    }
  }

  getActivityLabel(type: SessionActivity['type']): string {
    switch (type) {
      case 'view':
        return 'Viewed';
      case 'affinity':
        return 'Marked Affinity';
      case 'path-start':
        return 'Started Path';
      case 'path-complete':
        return 'Completed Path';
      case 'step-complete':
        return 'Completed Step';
      case 'explore':
        return 'Explored';
      default:
        return type;
    }
  }

  formatTimestamp(timestamp: string): string {
    const date = new Date(timestamp);
    return (
      date.toLocaleDateString() +
      ' ' +
      date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
    );
  }

  // =========================================================================
  // PATH PROGRESS
  // =========================================================================

  getPathProgressPercentage(progress: SessionPathProgress): number {
    const totalSteps = progress.completedStepIndices.length + 1; // Rough estimate
    return Math.min(100, (progress.completedStepIndices.length / totalSteps) * 100);
  }

  // =========================================================================
  // TIMELINE HELPERS
  // =========================================================================

  getTimelineEventIcon(type: string): string {
    const icons: Record<string, string> = {
      journey_started: 'play_circle',
      journey_completed: 'emoji_events',
      step_completed: 'check_circle',
      capability_earned: 'workspace_premium',
      meaningful_encounter: 'favorite',
      note_created: 'edit_note',
      return_visit: 'replay',
      first_exploration: 'explore',
    };
    return icons[type] ?? 'lens';
  }

  getTimelineEventColor(significance: string): string {
    const colors: Record<string, string> = {
      milestone: '#667eea',
      progress: '#4caf50',
      activity: '#9e9e9e',
    };
    return colors[significance] ?? '#9e9e9e';
  }

  formatTimelineDate(timestamp: string): string {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffDays === 0) return 'Today';
    if (diffDays === 1) return 'Yesterday';
    if (diffDays < 7) return `${diffDays} days ago`;
    return date.toLocaleDateString();
  }

  // =========================================================================
  // STATS HELPERS
  // =========================================================================

  getSessionDuration(): string {
    if (!this.session) return '0 days';

    const created = new Date(this.session.createdAt).getTime();
    const now = Date.now();
    const days = Math.floor((now - created) / (1000 * 60 * 60 * 24));

    if (days === 0) return 'Today';
    if (days === 1) return '1 day';
    return `${days} days`;
  }

  getMasteryLevelColor(level: MasteryLevel): string {
    const colors: Record<MasteryLevel, string> = {
      not_started: '#9e9e9e',
      seen: '#90caf9',
      remember: '#81c784',
      understand: '#aed581',
      apply: '#ffb74d',
      analyze: '#ff8a65',
      evaluate: '#ba68c8',
      create: '#4fc3f7',
    };
    return colors[level];
  }
}
