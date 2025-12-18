/**
 * LoginComponent - Hosted human authentication.
 *
 * Features:
 * - Email/username + password login
 * - Remember me (stores identifier)
 * - Error display with clear messaging
 * - Link to register for new users
 * - Post-login redirect to return URL
 */

import { Component, OnInit, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Router, ActivatedRoute, RouterModule } from '@angular/router';
import { AuthService } from '../../services/auth.service';
import { PasswordAuthProvider } from '../../services/providers/password-auth.provider';
import { IdentityService } from '../../services/identity.service';
import { type PasswordCredentials, AUTH_IDENTIFIER_KEY } from '../../models/auth.model';

@Component({
  selector: 'app-login',
  standalone: true,
  imports: [CommonModule, FormsModule, RouterModule],
  templateUrl: './login.component.html',
  styleUrls: ['./login.component.css'],
})
export class LoginComponent implements OnInit {
  private readonly authService = inject(AuthService);
  private readonly passwordProvider = inject(PasswordAuthProvider);
  private readonly identityService = inject(IdentityService);
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

  /** Return URL after successful login */
  private returnUrl = '/';

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
    this.router.navigate(['/register'], {
      queryParams: { returnUrl: this.returnUrl },
    });
  }
}
