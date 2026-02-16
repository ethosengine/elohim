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
import { Component, OnInit, OnDestroy, inject, signal, computed } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router, RouterModule, ActivatedRoute } from '@angular/router';

// @coverage: 92.5% (2026-02-05)

import { takeUntil } from 'rxjs/operators';

import { Subject } from 'rxjs';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
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

import { AGENCY_STAGES, type AgencyStageInfo } from '../../models/agency.model';
import { AgencyService } from '../../services/agency.service';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { IdentityService } from '../../services/identity.service';
import { SessionHumanService } from '../../services/session-human.service';
import { TauriAuthService } from '../../services/tauri-auth.service';

@Component({
  selector: 'app-profile',
  standalone: true,
  imports: [CommonModule, FormsModule, RouterModule],
  templateUrl: './profile.component.html',
  styleUrls: ['./profile.component.css'],
})
export class ProfileComponent implements OnInit, OnDestroy {
  private readonly identityService = inject(IdentityService);
  private readonly agencyService = inject(AgencyService);
  private readonly discoveryService = inject(DiscoveryAttestationService);
  private readonly doorwayRegistry = inject(DoorwayRegistryService);
  private readonly tauriAuth = inject(TauriAuthService);
  private readonly holochainService = inject(HolochainClientService);
  private readonly sessionHumanService = inject(SessionHumanService);
  private readonly router = inject(Router);
  private readonly route = inject(ActivatedRoute);
  private readonly destroy$ = new Subject<void>();

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
  readonly canUpgrade = this.agencyService.canUpgrade;

  // ==========================================================================
  // Graduation Signals
  // ==========================================================================

  readonly isTauriApp = this.tauriAuth.isTauri;
  readonly graduationStatus = this.tauriAuth.graduationStatus;
  readonly graduationError = this.tauriAuth.graduationError;
  readonly isGraduationEligible = this.tauriAuth.isGraduationEligible;

  /** Password for graduation form */
  graduationPassword = '';

  /** Info about the next agency stage */
  readonly nextStageInfo = computed<AgencyStageInfo | null>(() => {
    const nextStage = this.agencyService.agencyState().migrationTarget;
    if (!nextStage) return null;
    return AGENCY_STAGES[nextStage];
  });

  /** Label for the next stage */
  readonly nextStageLabel = computed(() => this.nextStageInfo()?.label ?? 'Next Stage');

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
  // Network / Agency Signals
  // ==========================================================================

  readonly agencyState = this.agencyService.agencyState;
  readonly connectionStatus = this.agencyService.connectionStatus;
  readonly edgeNodeInfo = computed(() => this.holochainService.getDisplayInfo());

  // ==========================================================================
  // Doorway Management
  // ==========================================================================

  /** Whether the add-doorway inline form is visible */
  readonly showAddDoorway = signal(false);

  /** URL being validated */
  readonly newDoorwayUrl = signal('');

  /** Validation state */
  readonly doorwayValidating = signal(false);
  readonly doorwayValidationError = signal<string | null>(null);
  readonly doorwayValidationResult = signal<{ name: string; url: string } | null>(null);

  // ==========================================================================
  // Data Management
  // ==========================================================================

  /** Export session data as JSON file */
  exportData(): void {
    const migration = this.sessionHumanService.prepareMigration();
    if (migration) {
      const blob = new Blob([JSON.stringify(migration, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `elohim-identity-export.json`;
      a.click();
      URL.revokeObjectURL(url);
    }
  }

  // ==========================================================================
  // Lifecycle
  // ==========================================================================

  ngOnInit(): void {
    // Load fresh profile data
    void this.loadProfile();

    // Handle fragment navigation (e.g., #network from agency badge)
    this.route.fragment.pipe(takeUntil(this.destroy$)).subscribe(fragment => {
      if (fragment === 'network' || fragment === 'upgrade') {
        // Scroll to the section after a short delay to let the DOM render
        setTimeout(() => {
          const el = document.getElementById(fragment);
          el?.scrollIntoView({ behavior: 'smooth', block: 'start' });
        }, 100);
      }
    });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
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
   * Confirm graduation — call Tauri IPC to sign stewardship.
   */
  async confirmGraduation(): Promise<void> {
    if (!this.graduationPassword.trim()) {
      return;
    }

    const success = await this.tauriAuth.confirmStewardship(this.graduationPassword);

    if (success) {
      this.graduationPassword = '';
      this.successMessage.set('Stewardship confirmed! Your keys are now on your device.');
      setTimeout(() => this.successMessage.set(null), 5000);
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

  // ==========================================================================
  // Network / Agency Helpers
  // ==========================================================================

  /**
   * Get CSS class for agency stage badge.
   */
  getStageBadgeClass(): string {
    const stage = this.agencyState().currentStage;
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
    } catch {
      // Clipboard write failed silently - not all browsers support this API
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

  // ==========================================================================
  // Doorway Management
  // ==========================================================================

  /**
   * Handle input event for doorway URL field.
   */
  onDoorwayUrlInput(event: Event): void {
    const input = event.target as HTMLInputElement;
    this.newDoorwayUrl.set(input.value);
  }

  /**
   * Toggle inline add-doorway form.
   */
  toggleAddDoorway(): void {
    this.showAddDoorway.update(v => !v);
    this.doorwayValidationError.set(null);
    this.doorwayValidationResult.set(null);
    this.newDoorwayUrl.set('');
  }

  /**
   * Validate a doorway URL.
   */
  async validateNewDoorway(): Promise<void> {
    const url = this.newDoorwayUrl().trim();
    if (!url) return;

    this.doorwayValidating.set(true);
    this.doorwayValidationError.set(null);
    this.doorwayValidationResult.set(null);

    const result = await this.doorwayRegistry.validateDoorway(url);

    this.doorwayValidating.set(false);
    if (result.isValid && result.doorway) {
      this.doorwayValidationResult.set({ name: result.doorway.name, url: result.doorway.url });
    } else {
      this.doorwayValidationError.set(result.error ?? 'Could not reach doorway');
    }
  }

  /**
   * Add the validated doorway and select it.
   */
  addValidatedDoorway(): void {
    const result = this.doorwayValidationResult();
    if (result) {
      this.doorwayRegistry.selectDoorwayByUrl(result.url);
      this.toggleAddDoorway();
    }
  }

  /**
   * Set a doorway as the primary/active one.
   */
  setDoorwayAsPrimary(doorwayUrl: string): void {
    this.doorwayRegistry.selectDoorwayByUrl(doorwayUrl);
  }
}
