/**
 * CapabilitiesDashboardComponent - Transparency view for device policies
 *
 * Shows the current user their active restrictions, time status, and
 * provides access to steward contact and appeal functions.
 *
 * Philosophy: "Everyone has limits - even the most capable benefit from
 * exploring their own constraints."
 *
 * Features:
 * - Time status (session/daily remaining)
 * - Active restrictions with explanations
 * - Steward contact information
 * - Appeal filing access
 */

import { Component, OnInit, OnDestroy, inject, signal, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';
import { interval, Subscription } from 'rxjs';

import { StewardshipService } from '../../services/stewardship.service';
import {
  type ComputedPolicy,
  type StewardshipGrant,
  type TimeAccessDecision,
  getStewardTierLabel,
  getAuthorityBasisLabel,
  INALIENABLE_FEATURES,
} from '../../models/stewardship.model';

/** Restriction display item */
interface RestrictionItem {
  type: 'feature' | 'route' | 'category' | 'time' | 'age_rating' | 'reach';
  label: string;
  description: string;
  canAppeal: boolean;
}

@Component({
  selector: 'app-capabilities-dashboard',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './capabilities-dashboard.component.html',
  styleUrls: ['./capabilities-dashboard.component.css'],
})
export class CapabilitiesDashboardComponent implements OnInit, OnDestroy {
  private readonly stewardship = inject(StewardshipService);
  private timerSubscription?: Subscription;

  // ===========================================================================
  // State
  // ===========================================================================

  readonly isLoading = signal(true);
  readonly error = signal<string | null>(null);
  readonly policy = signal<ComputedPolicy | null>(null);
  readonly stewards = signal<StewardshipGrant[]>([]);
  readonly timeAccess = signal<TimeAccessDecision | null>(null);

  // Time display updates every minute
  readonly currentTime = signal(new Date());

  // ===========================================================================
  // Computed State
  // ===========================================================================

  /** Whether there are any active restrictions */
  readonly hasRestrictions = computed(() => {
    const p = this.policy();
    if (!p) return false;

    return (
      p.blockedCategories.length > 0 ||
      p.disabledFeatures.length > 0 ||
      p.disabledRoutes.length > 0 ||
      p.sessionMaxMinutes !== undefined ||
      p.dailyMaxMinutes !== undefined ||
      p.ageRatingMax !== undefined ||
      p.reachLevelMax !== undefined
    );
  });

  /** Time remaining in session (formatted) */
  readonly sessionRemaining = computed(() => {
    const ta = this.timeAccess();
    if (ta?.status !== 'allowed' || ta.remainingSession === undefined) {
      return null;
    }
    return this.formatMinutes(ta.remainingSession);
  });

  /** Time remaining today (formatted) */
  readonly dailyRemaining = computed(() => {
    const ta = this.timeAccess();
    if (ta?.status !== 'allowed' || ta.remainingDaily === undefined) {
      return null;
    }
    return this.formatMinutes(ta.remainingDaily);
  });

  /** Time status message */
  readonly timeStatus = computed(() => {
    const ta = this.timeAccess();
    if (!ta) return null;

    switch (ta.status) {
      case 'allowed':
        return { status: 'ok', message: 'Access allowed' };
      case 'outside_window':
        return { status: 'blocked', message: 'Outside allowed time window' };
      case 'session_limit':
        return { status: 'blocked', message: 'Session time limit reached' };
      case 'daily_limit':
        return { status: 'blocked', message: 'Daily time limit reached' };
      default:
        return null;
    }
  });

  /** List of restrictions for display */
  readonly restrictions = computed<RestrictionItem[]>(() => {
    const p = this.policy();
    if (!p) return [];

    const items: RestrictionItem[] = [];

    // Blocked categories
    for (const cat of p.blockedCategories) {
      items.push({
        type: 'category',
        label: `${this.formatCategory(cat)} content blocked`,
        description: 'Content in this category is filtered by your policy.',
        canAppeal: true,
      });
    }

    // Disabled features
    for (const feat of p.disabledFeatures) {
      items.push({
        type: 'feature',
        label: `${this.formatFeature(feat)} disabled`,
        description: 'This feature is restricted by your policy.',
        canAppeal: true,
      });
    }

    // Disabled routes
    for (const route of p.disabledRoutes) {
      items.push({
        type: 'route',
        label: `${route} section disabled`,
        description: 'Access to this area is restricted by your policy.',
        canAppeal: true,
      });
    }

    // Time limits
    if (p.sessionMaxMinutes) {
      items.push({
        type: 'time',
        label: `Session limit: ${p.sessionMaxMinutes} minutes`,
        description: 'Maximum time per session.',
        canAppeal: true,
      });
    }

    if (p.dailyMaxMinutes) {
      items.push({
        type: 'time',
        label: `Daily limit: ${p.dailyMaxMinutes} minutes`,
        description: 'Maximum time per day.',
        canAppeal: true,
      });
    }

    // Age rating
    if (p.ageRatingMax) {
      items.push({
        type: 'age_rating',
        label: `Age rating limit: ${p.ageRatingMax}`,
        description: `Content rated above ${p.ageRatingMax} is filtered.`,
        canAppeal: true,
      });
    }

    // Reach level
    if (p.reachLevelMax !== undefined) {
      items.push({
        type: 'reach',
        label: `Reach level limit: ${p.reachLevelMax}`,
        description: `Content with reach above ${p.reachLevelMax} is filtered.`,
        canAppeal: true,
      });
    }

    return items;
  });

  /** Primary steward (first active grant) */
  readonly primarySteward = computed(() => {
    const grants = this.stewards();
    const active = grants.find(g => g.status === 'active');
    return active ?? null;
  });

  // ===========================================================================
  // Lifecycle
  // ===========================================================================

  ngOnInit(): void {
    this.loadData();

    // Update time display every minute
    this.timerSubscription = interval(60000).subscribe(() => {
      this.currentTime.set(new Date());
      this.refreshTimeAccess();
    });
  }

  ngOnDestroy(): void {
    this.timerSubscription?.unsubscribe();
  }

  // ===========================================================================
  // Data Loading
  // ===========================================================================

  async loadData(): Promise<void> {
    this.isLoading.set(true);
    this.error.set(null);

    try {
      // Load policy and stewards in parallel
      const [policy, stewards, timeAccess] = await Promise.all([
        this.stewardship.getMyPolicy(),
        this.stewardship.getMyStewards(),
        this.stewardship.checkTimeAccess(),
      ]);

      this.policy.set(policy);
      this.stewards.set(stewards);
      this.timeAccess.set(timeAccess);
    } catch (err) {
      console.error('[CapabilitiesDashboard] Load failed:', err);
      this.error.set('Failed to load capabilities information.');
    } finally {
      this.isLoading.set(false);
    }
  }

  async refreshTimeAccess(): Promise<void> {
    try {
      const timeAccess = await this.stewardship.checkTimeAccess();
      this.timeAccess.set(timeAccess);
    } catch (err) {
      console.error('[CapabilitiesDashboard] Time refresh failed:', err);
    }
  }

  refresh(): void {
    this.loadData();
  }

  // ===========================================================================
  // Actions
  // ===========================================================================

  /** File an appeal against a restriction */
  fileAppeal(restriction: RestrictionItem): void {
    // TODO: Navigate to appeal wizard with context
    console.log('[CapabilitiesDashboard] File appeal:', restriction);
  }

  /** Contact steward */
  contactSteward(): void {
    const steward = this.primarySteward();
    if (!steward) return;

    // TODO: Open messaging/contact interface
    console.log('[CapabilitiesDashboard] Contact steward:', steward.stewardId);
  }

  // ===========================================================================
  // Formatters
  // ===========================================================================

  formatMinutes(minutes: number): string {
    if (minutes >= 60) {
      const hours = Math.floor(minutes / 60);
      const mins = minutes % 60;
      return mins > 0 ? `${hours}h ${mins}m` : `${hours}h`;
    }
    return `${minutes}m`;
  }

  formatCategory(category: string): string {
    // Convert snake_case to Title Case
    return category
      .split('_')
      .map(word => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ');
  }

  formatFeature(feature: string): string {
    // Convert snake_case to Title Case
    return feature
      .split('_')
      .map(word => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ');
  }

  getStewardTierLabel(tier: string): string {
    return getStewardTierLabel(tier as any);
  }

  getAuthorityBasisLabel(basis: string): string {
    return getAuthorityBasisLabel(basis as any);
  }

  clearError(): void {
    this.error.set(null);
  }
}
