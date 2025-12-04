import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { RouterModule } from '@angular/router';
import { Subject } from 'rxjs';
import { takeUntil } from 'rxjs/operators';
import { SessionHumanService } from '../../services/session-human.service';
import { ContentMasteryService } from '../../services/content-mastery.service';
import { SessionHuman, SessionActivity, SessionPathProgress, MasteryStats, MasteryLevel } from '../../models';

/**
 * ProfilePageComponent - Session Human profile management.
 *
 * Tabs:
 * 1. Overview - Resume point, current focus, capabilities, journey stats
 * 2. Paths - In progress, completed, suggested
 * 3. Timeline - Chronological events with type filter
 * 4. Settings - Profile editing, export, reset
 *
 * Features:
 * - Editable display name, bio, interests
 * - Session badge with "Upgrade to save permanently" prompt
 * - Quick stats summary
 * - Avatar support (URL-based for MVP)
 */
@Component({
  selector: 'app-profile-page',
  standalone: true,
  imports: [CommonModule, FormsModule, RouterModule],
  templateUrl: './profile-page.component.html',
  styleUrls: ['./profile-page.component.css']
})
export class ProfilePageComponent implements OnInit, OnDestroy {
  // Tab management
  activeTab: 'overview' | 'paths' | 'timeline' | 'settings' = 'overview';

  // Session data
  session: SessionHuman | null = null;

  // Stats
  masteryStats: MasteryStats | null = null;
  pathProgressList: SessionPathProgress[] = [];
  activityHistory: SessionActivity[] = [];

  // Mastery breakdown
  masteryByLevel: { level: MasteryLevel; count: number; label: string }[] = [];

  // Activity filter
  activityFilter: 'all' | 'view' | 'affinity' | 'path-start' | 'path-complete' | 'step-complete' = 'all';
  filteredActivities: SessionActivity[] = [];

  // Edit mode for settings
  isEditing = false;
  editForm = {
    displayName: '',
    avatarUrl: '',
    bio: '',
    locale: '',
    interests: ''
  };

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
    create: 'Create'
  };

  constructor(
    private readonly sessionHumanService: SessionHumanService,
    private readonly contentMasteryService: ContentMasteryService
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
    this.sessionHumanService.session$
      .pipe(takeUntil(this.destroy$))
      .subscribe(session => {
        this.session = session;
        this.initEditForm();
      });

    // Load path progress
    this.pathProgressList = this.sessionHumanService.getAllPathProgress();

    // Load activity history
    this.activityHistory = this.sessionHumanService.getActivityHistory();
    this.applyActivityFilter();

    // Subscribe to mastery stats
    this.contentMasteryService.getMasteryStats()
      .pipe(takeUntil(this.destroy$))
      .subscribe(stats => {
        this.masteryStats = stats;
        this.computeMasteryBreakdown(stats);
        this.isLoading = false;
      });
  }

  private initEditForm(): void {
    if (this.session) {
      this.editForm = {
        displayName: this.session.displayName,
        avatarUrl: this.session.avatarUrl ?? '',
        bio: this.session.bio ?? '',
        locale: this.session.locale ?? '',
        interests: this.session.interests?.join(', ') ?? ''
      };
    }
  }

  private computeMasteryBreakdown(stats: MasteryStats): void {
    const levels: MasteryLevel[] = [
      'seen', 'remember', 'understand', 'apply', 'analyze', 'evaluate', 'create'
    ];

    this.masteryByLevel = levels
      .map(level => ({
        level,
        count: stats.levelDistribution[level],
        label: this.levelLabels[level]
      }))
      .filter(item => item.count > 0);
  }

  // =========================================================================
  // TAB NAVIGATION
  // =========================================================================

  setActiveTab(tab: 'overview' | 'paths' | 'timeline' | 'settings'): void {
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
      case 'view': return 'visibility';
      case 'affinity': return 'favorite';
      case 'path-start': return 'play_arrow';
      case 'path-complete': return 'check_circle';
      case 'step-complete': return 'done';
      case 'explore': return 'explore';
      default: return 'lens';
    }
  }

  getActivityLabel(type: SessionActivity['type']): string {
    switch (type) {
      case 'view': return 'Viewed';
      case 'affinity': return 'Marked Affinity';
      case 'path-start': return 'Started Path';
      case 'path-complete': return 'Completed Path';
      case 'step-complete': return 'Completed Step';
      case 'explore': return 'Explored';
      default: return type;
    }
  }

  formatTimestamp(timestamp: string): string {
    const date = new Date(timestamp);
    return date.toLocaleDateString() + ' ' + date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  // =========================================================================
  // PATH PROGRESS
  // =========================================================================

  getPathProgressPercentage(progress: SessionPathProgress): number {
    const totalSteps = progress.completedStepIndices.length + 1; // Rough estimate
    return Math.min(100, (progress.completedStepIndices.length / totalSteps) * 100);
  }

  // =========================================================================
  // SETTINGS / EDIT
  // =========================================================================

  startEditing(): void {
    this.isEditing = true;
    this.initEditForm();
  }

  cancelEditing(): void {
    this.isEditing = false;
    this.initEditForm();
  }

  saveProfile(): void {
    // Update display name
    this.sessionHumanService.setDisplayName(this.editForm.displayName);

    // Update avatar
    this.sessionHumanService.setAvatarUrl(this.editForm.avatarUrl);

    // Update bio
    this.sessionHumanService.setBio(this.editForm.bio);

    // Update locale
    this.sessionHumanService.setLocale(this.editForm.locale);

    // Update interests (comma-separated to array)
    const interests = this.editForm.interests
      .split(',')
      .map(i => i.trim())
      .filter(i => i.length > 0);
    this.sessionHumanService.setInterests(interests);

    this.isEditing = false;
  }

  // =========================================================================
  // DATA MANAGEMENT
  // =========================================================================

  exportData(): void {
    const migration = this.sessionHumanService.prepareMigration();
    if (migration) {
      const blob = new Blob([JSON.stringify(migration, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `lamad-session-${this.session?.sessionId ?? 'export'}.json`;
      a.click();
      URL.revokeObjectURL(url);
    }
  }

  resetSession(): void {
    if (confirm('Are you sure you want to reset your session? All progress will be lost.')) {
      this.sessionHumanService.resetSession();
      this.loadData();
    }
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
      create: '#4fc3f7'
    };
    return colors[level];
  }
}
