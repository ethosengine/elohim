/**
 * LoginComponent - Hosted human authentication.
 *
 * Features:
 * - Doorway-aware: Shows selected doorway, allows changing
 * - Email/username + password login
 * - Remember me (stores identifier)
 * - Error display with clear messaging
 * - Link to register for new users
 * - Post-login redirect to return URL
 * - Connection status indicator
 */

import { CommonModule } from '@angular/common';
import { Component, OnInit, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router, ActivatedRoute, RouterModule } from '@angular/router';

import { type PasswordCredentials, AUTH_IDENTIFIER_KEY } from '../../models/auth.model';
import { type DoorwayInfo } from '../../models/doorway.model';
import { AuthService } from '../../services/auth.service';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { IdentityService } from '../../services/identity.service';
import { PasswordAuthProvider } from '../../services/providers/password-auth.provider';
import { DoorwayPickerComponent } from '../doorway-picker/doorway-picker.component';

/** Login step type */
type LoginStep = 'doorway' | 'credentials';

@Component({
  selector: 'app-login',
  standalone: true,
  imports: [CommonModule, FormsModule, RouterModule, DoorwayPickerComponent],
  templateUrl: './login.component.html',
  styleUrls: ['./login.component.css'],
})
export class LoginComponent implements OnInit {
  private readonly authService = inject(AuthService);
  private readonly passwordProvider = inject(PasswordAuthProvider);
  private readonly identityService = inject(IdentityService);
  private readonly doorwayRegistry = inject(DoorwayRegistryService);
  private readonly router = inject(Router);
  private readonly route = inject(ActivatedRoute);

  // ==========================================================================
  // Form State
  // ==========================================================================

  form = {
    identifier: '',
    password: '',
    rememberMe: true,
  };

  // ==========================================================================
  // Component State
  // ==========================================================================

  readonly isLoading = signal(false);
  readonly error = signal<string | null>(null);
  readonly showPassword = signal(false);

  /** Current login step */
  readonly currentStep = signal<LoginStep>('doorway');

  /** Return URL after successful login */
  private returnUrl = '/';

  // ==========================================================================
  // Doorway State
  // ==========================================================================

  /** Selected doorway */
  readonly selectedDoorway = this.doorwayRegistry.selected;

  /** Whether a doorway has been selected */
  readonly hasDoorwaySelected = this.doorwayRegistry.hasSelection;

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

    // Pre-fill identifier if remembered
    const storedIdentifier = localStorage.getItem(AUTH_IDENTIFIER_KEY);
    if (storedIdentifier) {
      this.form.identifier = storedIdentifier;
    }

    // Skip doorway selection if already chosen
    if (this.hasDoorwaySelected()) {
      this.currentStep.set('credentials');
    }

    // Check if already authenticated - redirect if so
    if (this.authService.isAuthenticated()) {
      this.router.navigate([this.returnUrl]);
    }
  }

  // ==========================================================================
  // Actions
  // ==========================================================================

  /**
   * Submit login form.
   */
  async onLogin(): Promise<void> {
    // Validate form
    if (!this.form.identifier.trim()) {
      this.error.set('Please enter your email or username.');
      return;
    }

    if (!this.form.password) {
      this.error.set('Please enter your password.');
      return;
    }

    this.isLoading.set(true);
    this.error.set(null);

    try {
      const credentials: PasswordCredentials = {
        type: 'password',
        identifier: this.form.identifier.trim(),
        password: this.form.password,
      };

      const result = await this.authService.login('password', credentials);

      if (result.success) {
        // Clear password from form
        this.form.password = '';

        // Store or clear identifier based on remember me
        if (this.form.rememberMe) {
          localStorage.setItem(AUTH_IDENTIFIER_KEY, this.form.identifier.trim());
        } else {
          localStorage.removeItem(AUTH_IDENTIFIER_KEY);
        }

        // Wait for identity state to be fully established before navigating
        // This ensures the UI shows authenticated state immediately after redirect
        await this.identityService.waitForAuthenticatedState(3000);

        // Navigate to return URL
        this.router.navigate([this.returnUrl]);
      } else {
        this.error.set(result.error);
      }
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Login failed';
      this.error.set(errorMessage);
    } finally {
      this.isLoading.set(false);
    }
  }

  /**
   * Toggle password visibility.
   */
  togglePasswordVisibility(): void {
    this.showPassword.update(v => !v);
  }

  /**
   * Clear error message.
   */
  clearError(): void {
    this.error.set(null);
  }

  /**
   * Navigate to register page.
   */
  goToRegister(): void {
    this.router.navigate(['/identity/register'], {
      queryParams: { returnUrl: this.returnUrl },
    });
  }

  // ==========================================================================
  // Step Navigation
  // ==========================================================================

  /**
   * Handle doorway selection from picker.
   */
  onDoorwaySelected(doorway: DoorwayInfo): void {
    console.log('[Login] Doorway selected:', doorway.name);
    this.currentStep.set('credentials');
  }

  /**
   * Handle doorway picker cancellation.
   */
  onDoorwayPickerCancelled(): void {
    this.router.navigate(['/']);
  }

  /**
   * Go back to doorway selection.
   */
  goBackToDoorway(): void {
    this.currentStep.set('doorway');
  }
}
