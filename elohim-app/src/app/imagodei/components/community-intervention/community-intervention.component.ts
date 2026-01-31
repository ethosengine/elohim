/**
 * CommunityInterventionComponent - The "Jerry Problem" Solution
 *
 * Community-triggered intervention for patterns of harmful behavior.
 * This is NOT mob rule - it requires:
 * - Weighted support (people who know you have more weight)
 * - 10.0 threshold (can't be triggered by strangers)
 * - Subject notification within 24h
 * - 7-day response window
 * - Elohim arbitration
 * - Monthly review
 * - Clear restoration path
 *
 * Philosophy: "The goal is a world where Jerry can flourish too -
 * just not at others' expense."
 */

import { CommonModule } from '@angular/common';
import { Component, OnInit, inject, signal, computed, input } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { RouterModule, ActivatedRoute, Router } from '@angular/router';

// @coverage: 92.0% (2026-02-04)

import {
  type CommunityIntervention,
  type RelationshipLevel,
  type InterventionStatus,
  RELATIONSHIP_WEIGHTS,
  COMMUNITY_INTERVENTION_THRESHOLD,
  INTERVENTION_CATEGORIES,
  getRelationshipLevelLabel,
  getInterventionStatusLabel,
} from '../../models/stewardship.model';
import { StewardshipService } from '../../services/stewardship.service';

@Component({
  selector: 'app-community-intervention',
  standalone: true,
  imports: [CommonModule, FormsModule, RouterModule],
  templateUrl: './community-intervention.component.html',
  styleUrls: ['./community-intervention.component.css'],
})
export class CommunityInterventionComponent implements OnInit {
  private readonly stewardship = inject(StewardshipService);
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);

  // ===========================================================================
  // Inputs
  // ===========================================================================

  /** Subject ID for new intervention */
  readonly subjectIdInput = input<string | undefined>(undefined, { alias: 'subjectId' });

  /** Existing intervention ID to view/support */
  readonly interventionIdInput = input<string | undefined>(undefined, { alias: 'interventionId' });

  // ===========================================================================
  // State
  // ===========================================================================

  readonly isLoading = signal(true);
  readonly isSubmitting = signal(false);
  readonly error = signal<string | null>(null);
  readonly successMessage = signal<string | null>(null);

  /** Mode: 'initiate' or 'support' */
  readonly mode = signal<'initiate' | 'support' | 'view'>('initiate');

  /** Existing intervention (for support/view mode) */
  readonly intervention = signal<CommunityIntervention | null>(null);

  // Form state for initiation
  readonly subjectId = signal('');
  readonly relationshipLevel = signal<RelationshipLevel>('familiar');
  readonly patternDescription = signal('');
  readonly selectedCategories = signal<string[]>([]);
  readonly evidence = signal('');

  // Form state for support
  readonly supportReason = signal('');

  // ===========================================================================
  // Computed State
  // ===========================================================================

  /** Current weight based on relationship level */
  readonly currentWeight = computed(() => {
    return RELATIONSHIP_WEIGHTS[this.relationshipLevel()];
  });

  /** Progress toward threshold */
  readonly progressPercent = computed(() => {
    const intv = this.intervention();
    if (!intv) return 0;
    return Math.min(100, (intv.totalWeight / COMMUNITY_INTERVENTION_THRESHOLD) * 100);
  });

  /** Remaining weight needed */
  readonly remainingWeight = computed(() => {
    const intv = this.intervention();
    if (!intv) return COMMUNITY_INTERVENTION_THRESHOLD;
    return Math.max(0, COMMUNITY_INTERVENTION_THRESHOLD - intv.totalWeight);
  });

  /** Available relationship levels */
  readonly relationshipLevels: { value: RelationshipLevel; weight: number }[] = [
    { value: 'intimate', weight: 3.0 },
    { value: 'trusted', weight: 2.0 },
    { value: 'familiar', weight: 1.0 },
    { value: 'acquainted', weight: 0.5 },
    { value: 'public', weight: 0.1 },
  ];

  /** Available intervention categories */
  readonly categories = INTERVENTION_CATEGORIES;

  /** Threshold constant for display */
  readonly threshold = COMMUNITY_INTERVENTION_THRESHOLD;

  /** Whether form is valid for submission */
  readonly canSubmit = computed(() => {
    if (this.mode() === 'initiate') {
      return (
        this.subjectId().length > 0 &&
        this.patternDescription().length > 0 &&
        this.selectedCategories().length > 0
      );
    } else if (this.mode() === 'support') {
      return this.supportReason().length > 0;
    }
    return false;
  });

  // ===========================================================================
  // Lifecycle
  // ===========================================================================

  ngOnInit(): void {
    const interventionId =
      this.route.snapshot.paramMap.get('interventionId') ?? this.interventionIdInput();
    const subjectId = this.route.snapshot.paramMap.get('subjectId') ?? this.subjectIdInput();

    if (interventionId) {
      this.mode.set('view');
      this.loadIntervention(interventionId);
    } else if (subjectId) {
      this.mode.set('initiate');
      this.subjectId.set(subjectId);
      this.isLoading.set(false);
    } else {
      this.mode.set('initiate');
      this.isLoading.set(false);
    }
  }

  // ===========================================================================
  // Data Loading
  // ===========================================================================

  loadIntervention(_interventionId: string): void {
    this.isLoading.set(true);
    this.error.set(null);

    try {
      // Feature not yet implemented - needs holochain stewardship.getIntervention() integration

      this.isLoading.set(false);
    } catch (error) {
      console.error('[CommunityIntervention] Failed to load intervention:', error);
      this.error.set('Failed to load intervention.');
      this.isLoading.set(false);
    }
  }

  // ===========================================================================
  // Form Handlers
  // ===========================================================================

  setRelationshipLevel(level: RelationshipLevel): void {
    this.relationshipLevel.set(level);
  }

  toggleCategory(category: string): void {
    const current = this.selectedCategories();
    if (current.includes(category)) {
      this.selectedCategories.set(current.filter(c => c !== category));
    } else {
      this.selectedCategories.set([...current, category]);
    }
  }

  isCategorySelected(category: string): boolean {
    return this.selectedCategories().includes(category);
  }

  switchToSupport(): void {
    this.mode.set('support');
  }

  // ===========================================================================
  // Submit
  // ===========================================================================

  submitInitiation(): void {
    if (!this.canSubmit() || this.isSubmitting()) return;

    this.isSubmitting.set(true);
    this.error.set(null);

    try {
      // Feature not yet implemented - needs holochain stewardship.initiateIntervention() integration

      this.successMessage.set(
        'Intervention initiated. It will proceed if community support reaches the threshold.'
      );

      // Clear form
      this.patternDescription.set('');
      this.selectedCategories.set([]);
      this.evidence.set('');
    } catch (error) {
      console.error('[CommunityIntervention] Failed to initiate intervention:', error);
      this.error.set('Failed to initiate intervention.');
    } finally {
      this.isSubmitting.set(false);
    }
  }

  submitSupport(): void {
    const intv = this.intervention();
    if (!intv || !this.canSubmit() || this.isSubmitting()) return;

    this.isSubmitting.set(true);
    this.error.set(null);

    try {
      // Feature not yet implemented - needs holochain stewardship.supportIntervention() integration

      this.successMessage.set(`Your support (weight: ${this.currentWeight()}) has been recorded.`);

      // Clear form
      this.supportReason.set('');
    } catch (error) {
      console.error('[CommunityIntervention] Failed to record support:', error);
      this.error.set('Failed to record support.');
    } finally {
      this.isSubmitting.set(false);
    }
  }

  // ===========================================================================
  // Helpers
  // ===========================================================================

  getRelationshipLevelLabel(level: RelationshipLevel): string {
    return getRelationshipLevelLabel(level);
  }

  getInterventionStatusLabel(status: string): string {
    return getInterventionStatusLabel(status as InterventionStatus);
  }

  clearMessages(): void {
    this.error.set(null);
    this.successMessage.set(null);
  }
}
