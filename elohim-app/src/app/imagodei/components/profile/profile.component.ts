/**
 * ProfileComponent - Tabbed profile view with sub-component sections.
 *
 * Features:
 * - 3-tab layout: Identity, Network, Data & Privacy
 * - Sub-component decomposition for each section
 * - Agency-stage conditional rendering
 * - Fragment navigation (#network, #data)
 */

import { CommonModule } from '@angular/common';
import { Component, OnInit, OnDestroy, inject, signal, computed } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router, RouterModule, ActivatedRoute } from '@angular/router';

import { takeUntil } from 'rxjs/operators';

import { Subject } from 'rxjs';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import {
  type UpdateProfileRequest,
  type ProfileReach,
  getReachLabel,
  getReachDescription,
} from '@app/imagodei/models/identity.model';
import { DiscoveryAttestationService } from '@app/lamad/quiz-engine/services/discovery-attestation.service';

import { AGENCY_STAGES, type AgencyStageInfo } from '../../models/agency.model';
import { AgencyService } from '../../services/agency.service';
import { AuthService } from '../../services/auth.service';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { IdentityService } from '../../services/identity.service';
import { SessionHumanService } from '../../services/session-human.service';
import { TauriAuthService } from '../../services/tauri-auth.service';

import { ProfileAgencySectionComponent } from './sections/profile-agency-section/profile-agency-section.component';
import { ProfileAttestationsSectionComponent } from './sections/profile-attestations-section/profile-attestations-section.component';
import { ProfileDataSectionComponent } from './sections/profile-data-section/profile-data-section.component';
import { ProfileDiscoverySectionComponent } from './sections/profile-discovery-section/profile-discovery-section.component';
import { ProfileDoorwaysSectionComponent } from './sections/profile-doorways-section/profile-doorways-section.component';
import { ProfileHeaderComponent } from './sections/profile-header/profile-header.component';
import { ProfileIdentitySectionComponent } from './sections/profile-identity-section/profile-identity-section.component';
import { ProfileNetworkSectionComponent } from './sections/profile-network-section/profile-network-section.component';

export type ProfileTab = 'identity' | 'network' | 'data';

@Component({
  selector: 'app-profile',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    RouterModule,
    ProfileHeaderComponent,
    ProfileIdentitySectionComponent,
    ProfileDiscoverySectionComponent,
    ProfileAttestationsSectionComponent,
    ProfileAgencySectionComponent,
    ProfileDoorwaysSectionComponent,
    ProfileNetworkSectionComponent,
    ProfileDataSectionComponent,
  ],
  templateUrl: './profile.component.html',
  styleUrls: ['./profile.component.css'],
})
export class ProfileComponent implements OnInit, OnDestroy {
  private readonly identityService = inject(IdentityService);
  private readonly authService = inject(AuthService);
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
  // Tab State
  // ==========================================================================

  readonly activeTab = signal<ProfileTab>('identity');

  // ==========================================================================
  // Component State
  // ==========================================================================

  readonly isEditing = signal(false);
  readonly isSaving = signal(false);
  readonly error = signal<string | null>(null);
  readonly successMessage = signal<string | null>(null);

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
  readonly did = this.identityService.did;
  readonly identityState = this.identityService.identity;

  // ==========================================================================
  // Agency Signals
  // ==========================================================================

  readonly agencyStage = this.agencyService.currentStage;
  readonly agencyInfo = this.agencyService.stageInfo;
  readonly canUpgrade = this.agencyService.canUpgrade;
  readonly agencyState = this.agencyService.agencyState;
  readonly connectionStatus = this.agencyService.connectionStatus;

  // ==========================================================================
  // Graduation Signals
  // ==========================================================================

  readonly isTauriApp = this.tauriAuth.isTauri;
  readonly graduationStatus = this.tauriAuth.graduationStatus;
  readonly graduationError = this.tauriAuth.graduationError;
  readonly isGraduationEligible = this.tauriAuth.isGraduationEligible;

  // ==========================================================================
  // Discovery Signals
  // ==========================================================================

  readonly allDiscoveryResults = this.discoveryService.results;

  // ==========================================================================
  // Doorway Signals
  // ==========================================================================

  readonly registeredDoorways = this.doorwayRegistry.doorwaysWithHealth;
  readonly activeDoorway = computed(() => this.doorwayRegistry.selected()?.doorway ?? null);

  readonly doorwayRegistrationContext = computed(() => {
    if (!this.isAuthenticated()) return null;
    const profile = this.profile();
    return {
      identifier: this.authService.identifier(),
      registeredSince: profile?.createdAt ?? null,
      credentialStorage: this.edgeNodeInfo().hasStoredCredentials ? ('browser' as const) : null,
    };
  });

  // ==========================================================================
  // Computed
  // ==========================================================================

  readonly canEdit = computed(() => {
    const mode = this.mode();
    const isNetworkMode = mode === 'hosted' || mode === 'steward';
    return isNetworkMode && this.isAuthenticated();
  });

  readonly edgeNodeInfo = computed(() => this.holochainService.getDisplayInfo());

  readonly nextStageInfo = computed<AgencyStageInfo | null>(() => {
    const nextStage = this.agencyService.agencyState().migrationTarget;
    if (!nextStage) return null;
    return AGENCY_STAGES[nextStage];
  });

  readonly nextStageLabel = computed(() => this.nextStageInfo()?.label ?? 'Next Stage');

  /** Whether the user is in a non-visitor mode (has at least a session) */
  readonly isNetworkUser = computed(() => {
    const mode = this.mode();
    return mode === 'hosted' || mode === 'steward';
  });

  readonly reachOptions: { value: ProfileReach; label: string; description: string }[] = [
    {
      value: 'community',
      label: getReachLabel('community'),
      description: getReachDescription('community'),
    },
    {
      value: 'public',
      label: getReachLabel('public'),
      description: getReachDescription('public'),
    },
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
    void this.loadProfile();

    this.route.fragment.pipe(takeUntil(this.destroy$)).subscribe(fragment => {
      if (fragment === 'network' || fragment === 'upgrade') {
        this.activeTab.set('network');
      } else if (fragment === 'data') {
        this.activeTab.set('data');
      }
    });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  // ==========================================================================
  // Tab Navigation
  // ==========================================================================

  selectTab(tab: ProfileTab): void {
    this.activeTab.set(tab);
  }

  // ==========================================================================
  // Actions
  // ==========================================================================

  async loadProfile(): Promise<void> {
    try {
      await this.identityService.getCurrentHuman();
    } catch (error) {
      console.warn('[Profile] Non-critical profile refresh failed:', error);
    }
  }

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

  cancelEditing(): void {
    this.isEditing.set(false);
    this.error.set(null);
  }

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
      setTimeout(() => this.successMessage.set(null), 3000);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to update profile';
      this.error.set(errorMessage);
    } finally {
      this.isSaving.set(false);
    }
  }

  async confirmGraduation(password: string): Promise<void> {
    const success = await this.tauriAuth.confirmStewardship(password);
    if (success) {
      this.successMessage.set('Stewardship confirmed! Your keys are now on your device.');
      setTimeout(() => this.successMessage.set(null), 5000);
    }
  }

  clearMessages(): void {
    this.error.set(null);
    this.successMessage.set(null);
  }

  goBack(): void {
    void this.router.navigate(['/']);
  }

  // ==========================================================================
  // Event Handlers (from sub-components)
  // ==========================================================================

  navigateToDiscovery(): void {
    void this.router.navigate(['/lamad/discovery']);
  }

  setDoorwayAsPrimary(doorwayUrl: string): void {
    this.doorwayRegistry.selectDoorwayByUrl(doorwayUrl);
  }

  async validateDoorway(url: string): Promise<void> {
    // Delegated to doorway-registry for validation
    await this.doorwayRegistry.validateDoorway(url);
  }

  addDoorway(url: string): void {
    this.doorwayRegistry.selectDoorwayByUrl(url);
  }

  async reconnect(): Promise<void> {
    await this.holochainService.disconnect();
    await this.holochainService.connect();
  }

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
  // Helpers
  // ==========================================================================

  getReachLabel(reach: ProfileReach | undefined): string {
    return reach ? getReachLabel(reach) : 'Not set';
  }

  private parseAffinities(input: string): string[] {
    return input
      .split(',')
      .map(s => s.trim().toLowerCase())
      .filter(s => s.length > 0);
  }
}
