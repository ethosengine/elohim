/**
 * RegisterComponent - Network identity registration.
 *
 * Features:
 * - Step 1: Doorway selection (fediverse-style gateway choice)
 * - Step 2: Profile info + credentials
 * - Migration from session (if session exists)
 * - Post-registration redirect to return URL
 */

import { CommonModule } from '@angular/common';
import { Component, OnInit, inject, signal, computed } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router, ActivatedRoute, RouterModule } from '@angular/router';

// @coverage: 94.6% (2026-02-05)

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';

import { type DoorwayInfo } from '../../models/doorway.model';
import {
  type RegisterHumanRequest,
  type ProfileReach,
  getReachLabel,
  getReachDescription,
} from '../../models/identity.model';
import { AuthService } from '../../services/auth.service';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { IdentityService } from '../../services/identity.service';
import { PasswordAuthProvider } from '../../services/providers/password-auth.provider';
import { SessionHumanService } from '../../services/session-human.service';
import { SessionMigrationService } from '../../services/session-migration.service';
import { DoorwayPickerComponent } from '../doorway-picker/doorway-picker.component';

/** Registration step type */
type RegistrationStep = 'doorway' | 'credentials';

@Component({
  selector: 'app-register',
  standalone: true,
  imports: [CommonModule, FormsModule, RouterModule, DoorwayPickerComponent],
  templateUrl: './register.component.html',
  styleUrls: ['./register.component.css'],
})
export class RegisterComponent implements OnInit {
  private readonly identityService = inject(IdentityService);
  private readonly sessionHumanService = inject(SessionHumanService);
  private readonly migrationService = inject(SessionMigrationService);
  private readonly holochainClient = inject(HolochainClientService);
  private readonly authService = inject(AuthService);
  private readonly passwordProvider = inject(PasswordAuthProvider);
  private readonly doorwayRegistry = inject(DoorwayRegistryService);
  private readonly router = inject(Router);
  private readonly route = inject(ActivatedRoute);

  // ==========================================================================
  // Form State
  // ==========================================================================

  form = {
    displayName: '',
    bio: '',
    affinities: '',
    profileReach: 'community' as ProfileReach,
    location: '',
    // Authentication credentials
    email: '',
    password: '',
    confirmPassword: '',
  };

  // ==========================================================================
  // Component State
  // ==========================================================================

  readonly isRegistering = signal(false);
  readonly isMigrating = signal(false);
  readonly error = signal<string | null>(null);
  readonly showPassword = signal(false);

  /** Current registration step */
  readonly currentStep = signal<RegistrationStep>('doorway');

  /** Return URL after successful registration */
  private returnUrl = '/';

  // ==========================================================================
  // Doorway State
  // ==========================================================================

  /** Selected doorway */
  readonly selectedDoorway = this.doorwayRegistry.selected;

  /** Whether a doorway has been selected */
  readonly hasDoorwaySelected = this.doorwayRegistry.hasSelection;

  // ==========================================================================
  // Computed State
  // ==========================================================================

  /** Whether connected to network */
  readonly isConnected = this.holochainClient.isConnected;

  /** Whether user has an existing session */
  readonly hasSession = computed(() => this.sessionHumanService.hasSession());

  /** Session stats for migration preview */
  readonly sessionStats = computed(() => {
    const session = this.sessionHumanService.getSession();
    return session?.stats ?? null;
  });

  /** Session display name for pre-fill */
  readonly sessionDisplayName = computed(() => {
    const session = this.sessionHumanService.getSession();
    return session?.displayName ?? 'Traveler';
  });

  /** Whether migration is available */
  readonly canMigrate = this.migrationService.canMigrate;

  /** Migration state */
  readonly migrationState = this.migrationService.state;

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
    // Register password provider with auth service
    if (!this.authService.hasProvider('password')) {
      this.authService.registerProvider(this.passwordProvider);
    }

    // Get return URL from query params
    this.route.queryParams.subscribe(params => {
      this.returnUrl = params['returnUrl'] ?? '/';
    });

    // Skip doorway selection if already chosen
    if (this.hasDoorwaySelected()) {
      this.currentStep.set('credentials');
    }

    // Pre-fill from session if available
    if (this.hasSession()) {
      const session = this.sessionHumanService.getSession();
      if (session) {
        this.form.displayName = session.displayName !== 'Traveler' ? session.displayName : '';
        this.form.bio = session.bio ?? '';
        this.form.affinities = session.interests?.join(', ') ?? '';
      }
    }

    // Check if already authenticated - redirect if so
    const mode = this.identityService.mode();
    const isNetworkMode = mode === 'hosted' || mode === 'steward';
    if (isNetworkMode && this.identityService.isAuthenticated()) {
      void this.router.navigate([this.returnUrl]);
    }
  }

  // ==========================================================================
  // Actions
  // ==========================================================================

  /**
   * Register new identity.
   * If session data exists, automatically migrate it to preserve user's progress.
   */
  async onRegister(): Promise<void> {
    if (!this.isConnected()) {
      this.error.set('Not connected to network. Please wait for connection.');
      return;
    }

    if (!this.form.displayName.trim()) {
      this.error.set('Please enter a display name.');
      return;
    }

    // Validate email
    if (!this.form.email.trim()) {
      this.error.set('Please enter your email address.');
      return;
    }

    if (!this.isValidEmail(this.form.email)) {
      this.error.set('Please enter a valid email address.');
      return;
    }

    // Validate password
    if (!this.form.password) {
      this.error.set('Please enter a password.');
      return;
    }

    if (this.form.password.length < 8) {
      this.error.set('Password must be at least 8 characters.');
      return;
    }

    if (this.form.password !== this.form.confirmPassword) {
      this.error.set('Passwords do not match.');
      return;
    }

    this.isRegistering.set(true);
    this.error.set(null);

    try {
      // Check if we have session data to migrate
      const hasSessionToMigrate = this.hasSession() && this.canMigrate();

      if (hasSessionToMigrate) {
        // Use migration flow to preserve session progress
        // Migration service will handle identity creation + auth credentials
        const overrides: Partial<RegisterHumanRequest> = {
          displayName: this.form.displayName.trim(),
          bio: this.form.bio.trim() || undefined,
          affinities: this.parseAffinities(this.form.affinities),
          profileReach: this.form.profileReach,
          location: this.form.location.trim() || undefined,
          email: this.form.email.trim().toLowerCase(),
          password: this.form.password,
        };

        const migrationResult = await this.migrationService.migrate(overrides);

        if (!migrationResult.success) {
          throw new Error(migrationResult.error ?? 'Migration failed');
        }
      } else {
        // No session data - standard registration via doorway
        // Doorway handles identity creation + auth credentials atomically
        const request: RegisterHumanRequest = {
          displayName: this.form.displayName.trim(),
          bio: this.form.bio.trim() || undefined,
          affinities: this.parseAffinities(this.form.affinities),
          profileReach: this.form.profileReach,
          location: this.form.location.trim() || undefined,
          email: this.form.email.trim().toLowerCase(),
          password: this.form.password,
        };

        await this.identityService.registerHuman(request);
      }

      // Clear password from form for security
      this.form.password = '';
      this.form.confirmPassword = '';

      // Success - navigate to return URL
      void this.router.navigate([this.returnUrl]);
    } catch (err) {
      // Convert generic/unknown errors to user-friendly message
      let errorMessage = 'Registration failed';
      if (err instanceof Error && err.message !== 'Unknown error') {
        errorMessage = err.message;
      }
      this.error.set(errorMessage);
    } finally {
      this.isRegistering.set(false);
    }
  }

  /**
   * Migrate from session to network identity.
   * Note: This is now primarily for quick migration without email/password.
   * The main "Create Identity" button will also migrate if session exists.
   */
  async onMigrate(): Promise<void> {
    if (!this.canMigrate()) {
      this.error.set('Migration not available. Check network connection.');
      return;
    }

    // If email/password are filled in, use the full registration flow instead
    // This ensures auth credentials are created
    if (this.form.email.trim() && this.form.password) {
      return this.onRegister();
    }

    this.isMigrating.set(true);
    this.error.set(null);

    try {
      // Use form overrides if provided
      const overrides: Partial<RegisterHumanRequest> = {};
      if (this.form.displayName.trim()) {
        overrides.displayName = this.form.displayName.trim();
      }
      if (this.form.bio.trim()) {
        overrides.bio = this.form.bio.trim();
      }
      if (this.form.affinities.trim()) {
        overrides.affinities = this.parseAffinities(this.form.affinities);
      }
      overrides.profileReach = this.form.profileReach;

      const result = await this.migrationService.migrate(overrides);

      if (result.success) {
        // Success - navigate to return URL
        // Note: If email/password were provided, this code path isn't reached
        // (we redirect to onRegister() at the start of this function)
        void this.router.navigate([this.returnUrl]);
      } else {
        this.error.set(result.error ?? 'Migration failed');
      }
    } catch (err) {
      // Convert generic/unknown errors to user-friendly message
      let errorMessage = 'Migration failed';
      if (err instanceof Error && err.message !== 'Unknown error') {
        errorMessage = err.message;
      }
      this.error.set(errorMessage);
    } finally {
      this.isMigrating.set(false);
    }
  }

  /**
   * Clear error message.
   */
  clearError(): void {
    this.error.set(null);
  }

  /**
   * Toggle password visibility.
   */
  togglePasswordVisibility(): void {
    this.showPassword.update(v => !v);
  }

  /**
   * Navigate to login page.
   */
  goToLogin(): void {
    void this.router.navigate(['/identity/login'], {
      queryParams: { returnUrl: this.returnUrl },
    });
  }

  // ==========================================================================
  // Step Navigation
  // ==========================================================================

  /**
   * Handle doorway selection from picker.
   */
  onDoorwaySelected(_doorway: DoorwayInfo): void {
    this.currentStep.set('credentials');
  }

  /**
   * Handle doorway picker cancellation.
   */
  onDoorwayPickerCancelled(): void {
    void this.router.navigate(['/']);
  }

  /**
   * Go back to doorway selection.
   */
  goBackToDoorway(): void {
    this.currentStep.set('doorway');
  }

  // ==========================================================================
  // Helpers
  // ==========================================================================

  /**
   * Validate email format.
   */
  private isValidEmail(email: string): boolean {
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    return emailRegex.test(email);
  }

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
   * Get formatted session stats for display.
   */
  getSessionStatsDisplay(): string {
    const stats = this.sessionStats();
    if (!stats) return '';

    const parts: string[] = [];
    if (stats.nodesViewed > 0) parts.push(`${stats.nodesViewed} nodes viewed`);
    if (stats.pathsStarted > 0) parts.push(`${stats.pathsStarted} paths started`);
    if (stats.stepsCompleted > 0) parts.push(`${stats.stepsCompleted} steps completed`);

    return parts.join(', ') || 'No progress yet';
  }
}
