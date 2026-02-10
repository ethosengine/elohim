/**
 * ProfileComponent - View and edit user profile.
 *
 * Features:
 * - Display current profile information
 * - Edit mode for updating profile
 * - Profile reach selection
 * - Agency stage indicator
 */

import { CommonModule } from '@angular/common';
import { Component, OnInit, inject, signal, computed } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router, RouterModule } from '@angular/router';

// @coverage: 92.5% (2026-02-05)

import {
  type UpdateProfileRequest,
  type ProfileReach,
  getReachLabel,
  getReachDescription,
  getInitials,
} from '@app/imagodei/models/identity.model';
import {
  type DiscoveryResult,
  getFrameworkDisplayName,
  getCategoryIcon,
} from '@app/lamad/quiz-engine/models/discovery-assessment.model';
import { DiscoveryAttestationService } from '@app/lamad/quiz-engine/services/discovery-attestation.service';

import { AgencyService } from '../../services/agency.service';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { IdentityService } from '../../services/identity.service';

@Component({
  selector: 'app-profile',
  standalone: true,
  imports: [CommonModule, FormsModule, RouterModule],
  templateUrl: './profile.component.html',
  styleUrls: ['./profile.component.css'],
})
export class ProfileComponent implements OnInit {
  private readonly identityService = inject(IdentityService);
  private readonly agencyService = inject(AgencyService);
  private readonly discoveryService = inject(DiscoveryAttestationService);
  private readonly doorwayRegistry = inject(DoorwayRegistryService);
  private readonly router = inject(Router);

  // ==========================================================================
  // Component State
  // ==========================================================================

  readonly isEditing = signal(false);
  readonly isSaving = signal(false);
  readonly error = signal<string | null>(null);
  readonly successMessage = signal<string | null>(null);

  /** Form state for editing */
  form = {
    displayName: '',
    bio: '',
    affinities: '',
    location: '',
    profileReach: 'community' as ProfileReach,
  };

  // ==========================================================================
  // Identity Signals
  // ==========================================================================

  readonly profile = this.identityService.profile;
  readonly displayName = this.identityService.displayName;
  readonly mode = this.identityService.mode;
  readonly isAuthenticated = this.identityService.isAuthenticated;
  readonly attestations = this.identityService.attestations;
  readonly isLoading = this.identityService.isLoading;

  // ==========================================================================
  // Agency Signals
  // ==========================================================================

  readonly agencyStage = this.agencyService.currentStage;
  readonly agencyInfo = this.agencyService.stageInfo;

  // ==========================================================================
  // Discovery Signals
  // ==========================================================================

  /** Featured discovery results for profile display */
  readonly discoveryResults = this.discoveryService.featuredResults;

  /** All discovery results */
  readonly allDiscoveryResults = this.discoveryService.results;

  // ==========================================================================
  // Doorway Signals
  // ==========================================================================

  /** Registered doorways with health status */
  readonly registeredDoorways = this.doorwayRegistry.doorwaysWithHealth;

  /** Currently active doorway */
  readonly activeDoorway = computed(() => this.doorwayRegistry.selected()?.doorway ?? null);

  // ==========================================================================
  // Computed
  // ==========================================================================

  /** Initials for avatar placeholder */
  readonly initials = computed(() => getInitials(this.displayName()));

  /** Whether profile can be edited (requires network authentication) */
  readonly canEdit = computed(() => {
    const mode = this.mode();
    const isNetworkMode = mode === 'hosted' || mode === 'steward';
    return isNetworkMode && this.isAuthenticated();
  });

  /** Profile reach options */
  readonly reachOptions: { value: ProfileReach; label: string; description: string }[] = [
    {
      value: 'community',
      label: getReachLabel('community'),
      description: getReachDescription('community'),
    },
    { value: 'public', label: getReachLabel('public'), description: getReachDescription('public') },
    {
      value: 'trusted',
      label: getReachLabel('trusted'),
      description: getReachDescription('trusted'),
    },
    {
      value: 'private',
      label: getReachLabel('private'),
      description: getReachDescription('private'),
    },
  ];

  // ==========================================================================
  // Lifecycle
  // ==========================================================================

  ngOnInit(): void {
    // Load fresh profile data
    void this.loadProfile();
  }

  // ==========================================================================
  // Actions
  // ==========================================================================

  /**
   * Load profile from network.
   */
  async loadProfile(): Promise<void> {
    try {
      await this.identityService.getCurrentHuman();
    } catch (error) {
      // Intentionally silent - profile load failure is non-critical, uses cached data
      console.warn('[Profile] Non-critical profile refresh failed:', error);
    }
  }

  /**
   * Enter edit mode.
   */
  startEditing(): void {
    const profile = this.profile();
    if (profile) {
      this.form = {
        displayName: profile.displayName,
        bio: profile.bio ?? '',
        affinities: profile.affinities?.join(', ') ?? '',
        location: profile.location ?? '',
        profileReach: profile.profileReach ?? 'community',
      };
    }
    this.isEditing.set(true);
    this.error.set(null);
    this.successMessage.set(null);
  }

  /**
   * Cancel editing.
   */
  cancelEditing(): void {
    this.isEditing.set(false);
    this.error.set(null);
  }

  /**
   * Save profile changes.
   */
  async saveProfile(): Promise<void> {
    if (!this.form.displayName.trim()) {
      this.error.set('Display name is required.');
      return;
    }

    this.isSaving.set(true);
    this.error.set(null);

    try {
      const request: UpdateProfileRequest = {
        displayName: this.form.displayName.trim(),
        bio: this.form.bio.trim() || undefined,
        affinities: this.parseAffinities(this.form.affinities),
        location: this.form.location.trim() || undefined,
        profileReach: this.form.profileReach,
      };

      await this.identityService.updateProfile(request);

      this.isEditing.set(false);
      this.successMessage.set('Profile updated successfully!');

      // Clear success message after delay
      setTimeout(() => this.successMessage.set(null), 3000);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to update profile';
      this.error.set(errorMessage);
    } finally {
      this.isSaving.set(false);
    }
  }

  /**
   * Clear messages.
   */
  clearMessages(): void {
    this.error.set(null);
    this.successMessage.set(null);
  }

  /**
   * Navigate back.
   */
  goBack(): void {
    void this.router.navigate(['/']);
  }

  // ==========================================================================
  // Helpers
  // ==========================================================================

  /**
   * Parse comma-separated affinities string to array.
   */
  private parseAffinities(input: string): string[] {
    return input
      .split(',')
      .map(s => s.trim().toLowerCase())
      .filter(s => s.length > 0);
  }

  /**
   * Get reach label for display.
   */
  getReachLabel(reach: ProfileReach | undefined): string {
    return reach ? getReachLabel(reach) : 'Not set';
  }

  /**
   * Format date for display.
   */
  formatDate(dateString: string | undefined): string {
    if (!dateString) return 'Unknown';
    try {
      return new Date(dateString).toLocaleDateString(undefined, {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
      });
    } catch {
      return dateString;
    }
  }

  // ==========================================================================
  // Discovery Helpers
  // ==========================================================================

  /**
   * Get framework display name.
   */
  getFrameworkName(result: DiscoveryResult): string {
    return getFrameworkDisplayName(result.framework);
  }

  /**
   * Get category icon for a discovery result.
   */
  getCategoryIcon(result: DiscoveryResult): string {
    return getCategoryIcon(result.category);
  }

  /**
   * Toggle featured status for a discovery result.
   */
  toggleDiscoveryFeatured(resultId: string): void {
    this.discoveryService.toggleFeatured(resultId);
  }

  /**
   * Navigate to discovery assessment.
   */
  navigateToDiscovery(): void {
    void this.router.navigate(['/lamad/discovery']);
  }
}
