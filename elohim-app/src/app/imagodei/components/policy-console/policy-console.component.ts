/**
 * PolicyConsoleComponent - Unified policy editor for all steward tiers
 *
 * Same interface for all capability tiers - depth of access determined by
 * earned capabilities:
 * - Self: Edit own capability customizations
 * - Guide: View others' settings, suggest changes
 * - Guardian: Full edit for verified dependents
 * - Coordinator: Org-level baseline policies
 * - Constitutional: Elohim governance rules
 *
 * Philosophy: "Power scales with responsibility, not role assignment."
 */

import { CommonModule } from '@angular/common';
import { Component, OnInit, inject, signal, computed, input } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { RouterModule, ActivatedRoute, Router } from '@angular/router';

import {
  type ComputedPolicy,
  type DevicePolicy,
  type StewardshipGrant,
  type PolicyChainLink,
  type ContentFilterRules,
  type TimeLimitRules,
  type FeatureRestrictionRules,
  type TimeWindow,
  type InalienableFeature,
  type StewardCapabilityTier,
  type AuthorityBasis,
  getStewardTierLabel,
  getAuthorityBasisLabel,
  CONTENT_CATEGORIES,
  RESTRICTABLE_FEATURES,
  AGE_RATINGS,
  INALIENABLE_FEATURES,
} from '../../models/stewardship.model';
import { StewardshipService } from '../../services/stewardship.service';

/** Active tab in the policy editor */
type PolicyTab = 'content' | 'time' | 'features' | 'monitoring';

@Component({
  selector: 'app-policy-console',
  standalone: true,
  imports: [CommonModule, FormsModule, RouterModule],
  templateUrl: './policy-console.component.html',
  styleUrls: ['./policy-console.component.css'],
})
export class PolicyConsoleComponent implements OnInit {
  private readonly stewardship = inject(StewardshipService);
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);

  // ===========================================================================
  // Inputs
  // ===========================================================================

  /** Subject ID from route or input */
  readonly subjectIdInput = input<string | undefined>(undefined, { alias: 'subjectId' });

  // ===========================================================================
  // State
  // ===========================================================================

  readonly isLoading = signal(true);
  readonly isSaving = signal(false);
  readonly error = signal<string | null>(null);
  readonly successMessage = signal<string | null>(null);

  /** Active tab */
  readonly activeTab = signal<PolicyTab>('content');

  /** Subject being edited */
  readonly subjectId = signal<string | null>(null);

  /** Current grant (stewardship relationship) */
  readonly grant = signal<StewardshipGrant | null>(null);

  /** Policy inheritance chain */
  readonly policyChain = signal<PolicyChainLink[]>([]);

  /** Policy being edited */
  readonly policy = signal<DevicePolicy | null>(null);

  /** Parent policy (for showing inherited rules) */
  readonly parentPolicy = signal<ComputedPolicy | null>(null);

  /** My steward capability tier */
  readonly myTier = signal<string>('self');

  // Editing state for content rules
  readonly editingContentRules = signal<ContentFilterRules>({
    blockedCategories: [],
    blockedHashes: [],
    ageRatingMax: undefined,
    reachLevelMax: undefined,
  });

  // Editing state for time rules
  readonly editingTimeRules = signal<TimeLimitRules>({
    sessionMaxMinutes: undefined,
    dailyMaxMinutes: undefined,
    timeWindows: [],
    cooldownMinutes: undefined,
  });

  // Editing state for feature rules
  readonly editingFeatureRules = signal<FeatureRestrictionRules>({
    disabledFeatures: [],
    disabledRoutes: [],
    requireApproval: [],
  });

  // ===========================================================================
  // Computed State
  // ===========================================================================

  /** Whether current user can edit content rules */
  readonly canEditContent = computed(() => {
    const tier = this.myTier();
    return ['guardian', 'coordinator', 'constitutional'].includes(tier);
  });

  /** Whether current user can edit time rules */
  readonly canEditTime = computed(() => {
    const tier = this.myTier();
    return ['guardian', 'coordinator', 'constitutional'].includes(tier);
  });

  /** Whether current user can edit feature rules */
  readonly canEditFeatures = computed(() => {
    const tier = this.myTier();
    return ['guardian', 'coordinator', 'constitutional'].includes(tier);
  });

  /** Whether current user can view monitoring (guides can view) */
  readonly canViewMonitoring = computed(() => {
    const tier = this.myTier();
    return ['guide', 'guardian', 'coordinator', 'constitutional'].includes(tier);
  });

  /** Whether current user can configure monitoring */
  readonly canEditMonitoring = computed(() => {
    const tier = this.myTier();
    return ['guardian', 'coordinator', 'constitutional'].includes(tier);
  });

  /** Whether editing own policy */
  readonly isEditingSelf = computed(() => {
    return !this.subjectId();
  });

  /** Display name for subject */
  readonly subjectDisplayName = computed(() => {
    const g = this.grant();
    if (!g) return 'Your Settings';
    // TODO: Lookup display name from identity service
    return `Settings for ${g.subjectId.substring(0, 8)}...`;
  });

  /** Available content categories for blocking */
  readonly availableCategories = CONTENT_CATEGORIES;

  /** Available features for restriction */
  readonly availableFeatures = RESTRICTABLE_FEATURES;

  /** Age ratings */
  readonly availableAgeRatings = AGE_RATINGS;

  /** Features that cannot be restricted */
  readonly inalienableFeatures = INALIENABLE_FEATURES;

  // ===========================================================================
  // Lifecycle
  // ===========================================================================

  ngOnInit(): void {
    // Get subject ID from route or input
    const routeSubjectId = this.route.snapshot.paramMap.get('subjectId');
    const inputSubjectId = this.subjectIdInput();
    const subjectId = routeSubjectId ?? inputSubjectId ?? null;

    this.subjectId.set(subjectId);
    void this.loadData();
  }

  // ===========================================================================
  // Data Loading
  // ===========================================================================

  async loadData(): Promise<void> {
    this.isLoading.set(true);
    this.error.set(null);

    try {
      const subjectId = this.subjectId();

      if (subjectId) {
        // Editing another's policy - need grant
        const [grant, chain, policy, parentPolicy] = await Promise.all([
          this.stewardship.getGrantForSubject(subjectId),
          this.stewardship.getPolicyChain(subjectId),
          this.stewardship.getSubjectPolicy(subjectId),
          this.stewardship.getParentPolicy(subjectId),
        ]);

        this.grant.set(grant);
        this.policyChain.set(chain);
        this.policy.set(policy);
        this.parentPolicy.set(parentPolicy);
        this.myTier.set(grant?.tier ?? 'self');

        // Initialize editing state
        if (policy) {
          this.editingContentRules.set({ ...policy.contentRules });
          this.editingTimeRules.set({ ...policy.timeRules });
          this.editingFeatureRules.set({ ...policy.featureRules });
        }
      } else {
        // Editing own policy
        const [myPolicy, chain] = await Promise.all([
          this.stewardship.getMyPolicy(),
          this.stewardship.getMyPolicyChain(),
        ]);

        this.policyChain.set(chain);
        this.myTier.set('self');

        // For self, we use computed policy and allow customization
        if (myPolicy) {
          this.editingContentRules.set({
            blockedCategories: myPolicy.blockedCategories,
            blockedHashes: [],
            ageRatingMax: myPolicy.ageRatingMax,
            reachLevelMax: myPolicy.reachLevelMax,
          });
          this.editingTimeRules.set({
            sessionMaxMinutes: myPolicy.sessionMaxMinutes,
            dailyMaxMinutes: myPolicy.dailyMaxMinutes,
            timeWindows: myPolicy.timeWindows ?? [],
            cooldownMinutes: myPolicy.cooldownMinutes,
          });
          this.editingFeatureRules.set({
            disabledFeatures: myPolicy.disabledFeatures,
            disabledRoutes: myPolicy.disabledRoutes,
            requireApproval: [],
          });
        }
      }
    } catch (_err) {
      this.error.set('Failed to load policy information.');
    } finally {
      this.isLoading.set(false);
    }
  }

  // ===========================================================================
  // Tab Navigation
  // ===========================================================================

  setActiveTab(tab: PolicyTab): void {
    this.activeTab.set(tab);
  }

  // ===========================================================================
  // Content Rules Editing
  // ===========================================================================

  toggleCategory(category: string): void {
    const rules = this.editingContentRules();
    const categories = [...rules.blockedCategories];
    const index = categories.indexOf(category);

    if (index >= 0) {
      categories.splice(index, 1);
    } else {
      categories.push(category);
    }

    this.editingContentRules.set({ ...rules, blockedCategories: categories });
  }

  isCategoryBlocked(category: string): boolean {
    return this.editingContentRules().blockedCategories.includes(category);
  }

  isCategoryInherited(category: string): boolean {
    const parent = this.parentPolicy();
    return parent?.blockedCategories.includes(category) ?? false;
  }

  setAgeRating(rating: string | undefined): void {
    const rules = this.editingContentRules();
    this.editingContentRules.set({ ...rules, ageRatingMax: rating });
  }

  setReachLevel(level: number | undefined): void {
    const rules = this.editingContentRules();
    this.editingContentRules.set({ ...rules, reachLevelMax: level });
  }

  // ===========================================================================
  // Time Rules Editing
  // ===========================================================================

  setSessionLimit(minutes: number | undefined): void {
    const rules = this.editingTimeRules();
    this.editingTimeRules.set({ ...rules, sessionMaxMinutes: minutes });
  }

  setDailyLimit(minutes: number | undefined): void {
    const rules = this.editingTimeRules();
    this.editingTimeRules.set({ ...rules, dailyMaxMinutes: minutes });
  }

  setCooldown(minutes: number | undefined): void {
    const rules = this.editingTimeRules();
    this.editingTimeRules.set({ ...rules, cooldownMinutes: minutes });
  }

  addTimeWindow(): void {
    const rules = this.editingTimeRules();
    const windows = [...rules.timeWindows];
    windows.push({
      startHour: 9,
      startMinute: 0,
      endHour: 21,
      endMinute: 0,
      daysOfWeek: [1, 2, 3, 4, 5], // Weekdays by default
    });
    this.editingTimeRules.set({ ...rules, timeWindows: windows });
  }

  removeTimeWindow(index: number): void {
    const rules = this.editingTimeRules();
    const windows = [...rules.timeWindows];
    windows.splice(index, 1);
    this.editingTimeRules.set({ ...rules, timeWindows: windows });
  }

  updateTimeWindow(
    index: number,
    field: keyof TimeWindow,
    value: TimeWindow[keyof TimeWindow]
  ): void {
    const rules = this.editingTimeRules();
    const windows = [...rules.timeWindows];
    windows[index] = { ...windows[index], [field]: value } as TimeWindow;
    this.editingTimeRules.set({ ...rules, timeWindows: windows });
  }

  // ===========================================================================
  // Feature Rules Editing
  // ===========================================================================

  toggleFeature(feature: string): void {
    // Cannot disable inalienable features
    if (this.inalienableFeatures.includes(feature as InalienableFeature)) {
      return;
    }

    const rules = this.editingFeatureRules();
    const features = [...rules.disabledFeatures];
    const index = features.indexOf(feature);

    if (index >= 0) {
      features.splice(index, 1);
    } else {
      features.push(feature);
    }

    this.editingFeatureRules.set({ ...rules, disabledFeatures: features });
  }

  isFeatureDisabled(feature: string): boolean {
    return this.editingFeatureRules().disabledFeatures.includes(feature);
  }

  isFeatureInalienable(feature: string): boolean {
    return this.inalienableFeatures.includes(feature as InalienableFeature);
  }

  isFeatureInherited(feature: string): boolean {
    const parent = this.parentPolicy();
    return parent?.disabledFeatures.includes(feature) ?? false;
  }

  // ===========================================================================
  // Save
  // ===========================================================================

  async savePolicy(): Promise<void> {
    if (this.isSaving()) return;

    this.isSaving.set(true);
    this.error.set(null);
    this.successMessage.set(null);

    try {
      const subjectId = this.subjectId();

      await this.stewardship.upsertPolicy({
        subjectId: subjectId ?? undefined,
        contentRules: this.editingContentRules(),
        timeRules: this.editingTimeRules(),
        featureRules: this.editingFeatureRules(),
      });

      this.successMessage.set('Policy saved successfully.');

      // Reload to get computed values
      await this.loadData();
    } catch (_err) {
      this.error.set('Failed to save policy changes.');
    } finally {
      this.isSaving.set(false);
    }
  }

  // ===========================================================================
  // Helpers
  // ===========================================================================

  formatCategory(category: string): string {
    return category
      .split('_')
      .map(word => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ');
  }

  formatFeature(feature: string): string {
    return feature
      .split('_')
      .map(word => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ');
  }

  getDayName(day: number): string {
    const days = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
    return days[day] || '';
  }

  formatTime(hour: number, minute: number): string {
    const h = hour.toString().padStart(2, '0');
    const m = minute.toString().padStart(2, '0');
    return `${h}:${m}`;
  }

  clearMessages(): void {
    this.error.set(null);
    this.successMessage.set(null);
  }

  getStewardTierLabel(tier: string): string {
    return getStewardTierLabel(tier as StewardCapabilityTier);
  }

  getAuthorityBasisLabel(basis: string): string {
    return getAuthorityBasisLabel(basis as AuthorityBasis);
  }
}
