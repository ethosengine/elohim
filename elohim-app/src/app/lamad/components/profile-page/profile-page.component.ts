import { CommonModule } from '@angular/common';
import { Component, OnInit, OnDestroy, inject, computed, signal, effect } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { RouterModule, ActivatedRoute } from '@angular/router';

import { takeUntil, catchError } from 'rxjs/operators';

import { Subject, Observable, of } from 'rxjs';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
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
import { SovereigntyService } from '@app/imagodei/services/sovereignty.service';

import { MasteryStats, MasteryLevel } from '../../models';
import { ContentMasteryService } from '../../services/content-mastery.service';

/**
 * ProfilePageComponent - Session Human profile management.
 *
 * Tabs:
 * 1. Overview - Resume point, current focus, capabilities, journey stats
 * 2. Paths - In progress, completed, suggested
 * 3. Timeline - Chronological events with type filter
 * 4. Network - Sovereignty status, data residency, connection details
 * 5. Settings - Profile editing, export, reset
 *
 * Features:
 * - Editable display name, bio, interests
 * - Session badge with "Upgrade to save permanently" prompt
 * - Quick stats summary
 * - Avatar support (URL-based for MVP)
 * - Sovereignty/data residency visualization
 */
@Component({
  selector: 'app-profile-page',
  standalone: true,
  imports: [CommonModule, FormsModule, RouterModule],
  templateUrl: './profile-page.component.html',
  styleUrls: ['./profile-page.component.css'],
})
export class ProfilePageComponent implements OnInit, OnDestroy {
  // Injected services for sovereignty/network tab
  private readonly sovereigntyService = inject(SovereigntyService);
  private readonly holochainService = inject(HolochainClientService);
  readonly identityService = inject(IdentityService); // Public for template access
  private readonly route = inject(ActivatedRoute);

  // Tab management
  activeTab: 'overview' | 'paths' | 'timeline' | 'network' | 'settings' = 'overview';

  // Sovereignty state (reactive)
  readonly sovereigntyState = this.sovereigntyService.sovereigntyState;
  readonly stageInfo = this.sovereigntyService.stageInfo;
  readonly connectionStatus = this.sovereigntyService.connectionStatus;
  readonly canUpgrade = this.sovereigntyService.canUpgrade;
  readonly edgeNodeInfo = computed(() => this.holochainService.getDisplayInfo());

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

  /** Holochain profile (when authenticated) */
  readonly holochainProfile = this.identityService.profile;

  /** Display name from identity (prefers Holochain, falls back to session) */
  readonly displayName = computed(() => {
    if (this.isNetworkAuthenticated()) {
      return this.identityService.displayName();
    }
    return this.session?.displayName ?? 'Traveler';
  });

  /** Bio from identity (prefers Holochain, falls back to session) */
  readonly bio = computed(() => {
    if (this.isNetworkAuthenticated()) {
      return this.holochainProfile()?.bio ?? null;
    }
    return this.session?.bio ?? null;
  });

  /** Interests/affinities (prefers Holochain, falls back to session) */
  readonly interests = computed(() => {
    if (this.isNetworkAuthenticated()) {
      return this.holochainProfile()?.affinities ?? [];
    }
    return this.session?.interests ?? [];
  });

  /** Location from Holochain profile */
  readonly location = computed(() => this.holochainProfile()?.location ?? null);

  /** Profile reach from Holochain profile */
  readonly profileReach = computed(() => this.holochainProfile()?.profileReach ?? null);

  /** Member since date */
  readonly memberSince = computed(() => {
    if (this.isNetworkAuthenticated()) {
      return this.holochainProfile()?.createdAt ?? null;
    }
    return this.session?.createdAt ?? null;
  });

  /** Badge text based on identity mode */
  readonly modeBadgeText = computed(() => {
    const mode = this.identityMode();
    switch (mode) {
      case 'hosted':
        return 'Hosted Human';
      case 'steward':
        return 'Steward';
      case 'self-sovereign':
        return 'Steward'; // deprecated, mapped to steward
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
      case 'self-sovereign':
        return 'verified_user'; // deprecated, mapped to steward
      case 'session':
        return 'local_activity';
      case 'migrating':
        return 'sync';
      default:
        return 'person';
    }
  });

  // Session data
  session: SessionHuman | null = null;

  // Stats
  masteryStats: MasteryStats | null = null;
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

  // Edit mode for settings
  isEditing = false;
  editForm = {
    displayName: '',
    avatarUrl: '',
    bio: '',
    locale: '',
    interests: '',
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
    create: 'Create',
  };

  constructor(
    private readonly sessionHumanService: SessionHumanService,
    private readonly contentMasteryService: ContentMasteryService,
    private readonly profileService: ProfileService
  ) {}

  ngOnInit(): void {
    this.loadData();

    // Handle fragment navigation (e.g., #network from sovereignty badge)
    this.route.fragment.pipe(takeUntil(this.destroy$)).subscribe(fragment => {
      if (fragment === 'network') {
        this.activeTab = 'network';
      }
    });
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
      this.initEditForm();
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

  private initEditForm(): void {
    // Use Holochain profile data if authenticated, otherwise use session
    if (this.isNetworkAuthenticated()) {
      const profile = this.holochainProfile();
      this.editForm = {
        displayName: profile?.displayName ?? '',
        avatarUrl: '', // Holochain profile doesn't have avatarUrl in current schema
        bio: profile?.bio ?? '',
        locale: '', // Not in Holochain schema yet
        interests: profile?.affinities?.join(', ') ?? '',
      };
    } else if (this.session) {
      this.editForm = {
        displayName: this.session.displayName,
        avatarUrl: this.session.avatarUrl ?? '',
        bio: this.session.bio ?? '',
        locale: this.session.locale ?? '',
        interests: this.session.interests?.join(', ') ?? '',
      };
    }
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

  setActiveTab(tab: 'overview' | 'paths' | 'timeline' | 'network' | 'settings'): void {
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

  async saveProfile(): Promise<void> {
    // Parse interests (comma-separated to array)
    const interests = this.editForm.interests
      .split(',')
      .map(i => i.trim())
      .filter(i => i.length > 0);

    // If authenticated via Holochain, update the Holochain profile
    if (this.isNetworkAuthenticated()) {
      try {
        await this.identityService.updateProfile({
          displayName: this.editForm.displayName,
          bio: this.editForm.bio || undefined,
          affinities: interests,
          // Note: avatarUrl and locale not in Holochain schema yet
        });
        this.isEditing = false;
      } catch (err) {
        console.error('[ProfilePage] Failed to update Holochain profile:', err);
        // Show error to user (could add error state signal)
      }
    } else {
      // Update session data for visitors
      this.sessionHumanService.setDisplayName(this.editForm.displayName);
      this.sessionHumanService.setAvatarUrl(this.editForm.avatarUrl);
      this.sessionHumanService.setBio(this.editForm.bio);
      this.sessionHumanService.setLocale(this.editForm.locale);
      this.sessionHumanService.setInterests(interests);
      this.isEditing = false;
    }
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

  // =========================================================================
  // NETWORK / SOVEREIGNTY HELPERS
  // =========================================================================

  /**
   * Get CSS class for sovereignty stage badge.
   */
  getStageBadgeClass(): string {
    const stage = this.sovereigntyState().currentStage;
    return `stage-badge--${stage}`;
  }

  /**
   * Get CSS class for connection status dot.
   */
  getStatusDotClass(): string {
    const status = this.connectionStatus().state;
    return `status-dot--${status}`;
  }

  /**
   * Get icon for data location.
   */
  getLocationIcon(location: string): string {
    const icons: Record<string, string> = {
      'browser-memory': 'memory',
      'browser-storage': 'storage',
      'hosted-server': 'cloud',
      'local-holochain': 'smartphone',
      dht: 'lan',
      'encrypted-backup': 'lock',
    };
    return icons[location] ?? 'folder';
  }

  /**
   * Truncate hash for display.
   */
  truncateHash(hash: string | null): string {
    if (!hash) return 'N/A';
    if (hash.length <= 16) return hash;
    return `${hash.substring(0, 8)}...${hash.substring(hash.length - 4)}`;
  }

  /**
   * Copy value to clipboard.
   */
  async copyToClipboard(value: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(value);
    } catch (err) {
      console.warn('Failed to copy to clipboard:', err);
    }
  }

  /**
   * Reconnect to network.
   */
  async reconnect(): Promise<void> {
    await this.holochainService.disconnect();
    await this.holochainService.connect();
  }

  /**
   * Check if connected to network.
   */
  isConnected(): boolean {
    return this.connectionStatus().state === 'connected';
  }
}
