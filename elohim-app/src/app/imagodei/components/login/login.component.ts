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

// @coverage: 100.0% (2026-02-05)

import { environment } from '../../../../environments/environment';
import { type PasswordCredentials, AUTH_IDENTIFIER_KEY } from '../../models/auth.model';
import {
  type DoorwayInfo,
  parseFederatedIdentifier,
  resolveGatewayToDoorwayUrl,
} from '../../models/doorway.model';
import { AuthService } from '../../services/auth.service';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { IdentityService } from '../../services/identity.service';
import { OAuthAuthProvider } from '../../services/providers/oauth-auth.provider';
import { PasswordAuthProvider } from '../../services/providers/password-auth.provider';
import { TauriAuthService } from '../../services/tauri-auth.service';
import { AccountSwitcherComponent } from '../account-switcher/account-switcher.component';
import { DoorwayPickerComponent } from '../doorway-picker/doorway-picker.component';

/** Login step type */
type LoginStep = 'doorway' | 'credentials' | 'federated' | 'redirecting' | 'unlock' | 'restarting' | 'switch-account';

@Component({
  selector: 'app-login',
  standalone: true,
  imports: [CommonModule, FormsModule, RouterModule, AccountSwitcherComponent, DoorwayPickerComponent],
  templateUrl: './login.component.html',
  styleUrls: ['./login.component.css'],
})
export class LoginComponent implements OnInit {
  private readonly authService = inject(AuthService);
  private readonly passwordProvider = inject(PasswordAuthProvider);
  private readonly oauthProvider = inject(OAuthAuthProvider);
  private readonly identityService = inject(IdentityService);
  private readonly doorwayRegistry = inject(DoorwayRegistryService);
  private readonly tauriAuth = inject(TauriAuthService);
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

  /** Federated identifier input (user@gateway.host) */
  federatedIdentifier = '';

  // ==========================================================================
  // Component State
  // ==========================================================================

  readonly isLoading = signal(false);
  readonly error = signal<string | null>(null);
  readonly showPassword = signal(false);

  /** Current login step */
  readonly currentStep = signal<LoginStep>('doorway');

  /** Whether this is a first-launch Tauri launcher experience */
  readonly isLauncher = signal(false);

  /** Whether the user is a confirmed steward (set after login/unlock) */
  readonly isStewardResult = signal(false);

  /** Unlock password (separate from login form password) */
  unlockPassword = '';

  /** Doorway name shown during restarting step */
  readonly restartingDoorwayName = signal('');

  /** Return URL after successful login */
  returnUrl = '/';

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

    // Get return URL and launcher mode from query params
    this.route.queryParams.subscribe(params => {
      this.returnUrl = (params['returnUrl'] as string) ?? '/';
      if (params['launcher'] === 'true') {
        this.isLauncher.set(true);
      }
    });

    // Pre-fill identifier if remembered
    const storedIdentifier = localStorage.getItem(AUTH_IDENTIFIER_KEY);
    if (storedIdentifier) {
      this.form.identifier = storedIdentifier;
    }

    // Already authenticated -> redirect
    if (this.authService.isAuthenticated()) {
      void this.router.navigate([this.returnUrl]);
      return;
    }

    // === Context routing ===

    // Tauri: returning user with key bundle -> unlock; first-time -> doorway picker
    if (this.tauriAuth.isTauri()) {
      void this.initTauriStep();
      return;
    }

    // Browser: determine auto-redirect vs manual
    const doorwayUrl = environment.client?.doorwayUrl;
    const isProductionLike =
      !!doorwayUrl && !doorwayUrl.includes('localhost') && !doorwayUrl.includes('127.0.0.1');
    const hasSavedDoorway = this.hasDoorwaySelected();

    if (isProductionLike) {
      // Production: environment doorwayUrl always wins (app is deployed FOR this doorway)
      this.currentStep.set('redirecting');
      this.doorwayRegistry.selectDoorwayByUrl(doorwayUrl);
      this.oauthProvider.storeReturnUrl(this.returnUrl);
      const callbackUrl = `${globalThis.location.origin}/auth/callback`;
      this.oauthProvider.initiateLogin(doorwayUrl, callbackUrl);
      return;
    }

    if (hasSavedDoorway) {
      // Returning dev user: use their saved doorway
      this.currentStep.set('redirecting');
      this.oauthProvider.storeReturnUrl(this.returnUrl);
      const callbackUrl = `${globalThis.location.origin}/auth/callback`;
      this.oauthProvider.initiateLogin(this.doorwayRegistry.selectedUrl()!, callbackUrl);
      return;
    }

    // Dev/generic browser, no saved doorway -> federated identifier input
    this.currentStep.set('federated');
  }

  // ==========================================================================
  // Actions
  // ==========================================================================

  /**
   * Submit login form.
   *
   * In Tauri mode, uses loginWithPassword IPC which handles the full
   * doorway handoff flow. In browser mode, uses the auth service.
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
      // Tauri: use IPC-based login flow
      if (this.tauriAuth.isTauri()) {
        const doorwayUrl = this.doorwayRegistry.selectedUrl();
        if (!doorwayUrl) {
          this.error.set('No doorway selected.');
          return;
        }

        const result = await this.tauriAuth.loginWithPassword(
          doorwayUrl,
          this.form.identifier.trim(),
          this.form.password
        );

        if (result.success) {
          this.form.password = '';
          if (this.form.rememberMe) {
            localStorage.setItem(AUTH_IDENTIFIER_KEY, this.form.identifier.trim());
          }

          this.isStewardResult.set(result.isSteward);
          this.restartingDoorwayName.set(
            this.selectedDoorway()?.doorway?.name ?? 'your doorway'
          );

          if (result.needsRestart) {
            this.currentStep.set('restarting');
          } else {
            void this.router.navigate([this.returnUrl]);
          }
        } else {
          this.error.set(result.error ?? 'Login failed');
        }
        return;
      }

      // Browser: standard auth service login
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
        void this.router.navigate([this.returnUrl]);
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
   * Submit unlock form (returning Tauri user).
   */
  async onUnlock(): Promise<void> {
    if (!this.unlockPassword) {
      this.error.set('Please enter your password.');
      return;
    }

    this.isLoading.set(true);
    this.error.set(null);

    try {
      const result = await this.tauriAuth.unlockWithPassword(this.unlockPassword);

      if (result.success) {
        this.unlockPassword = '';
        this.isStewardResult.set(result.isSteward);

        // Navigate to main app — conductor is already running with the right identity
        void this.router.navigate([this.returnUrl]);
      } else {
        this.error.set(result.error ?? 'Invalid password');
      }
    } catch (err) {
      this.error.set(err instanceof Error ? err.message : 'Unlock failed');
    } finally {
      this.isLoading.set(false);
    }
  }

  /**
   * Trigger app exit (Tauri only).
   *
   * Exits the app so the user can reopen it. On restart, the conductor
   * reinitializes with the newly saved doorway identity.
   */
  restartApp(): void {
    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    if (window.__TAURI__?.core) {
      // Exit the process via Tauri's built-in exit command
      // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
      void window.__TAURI__.core.invoke('plugin:process|exit', { code: 0 })
        .catch(() => {
          // Fallback: close the webview window
          globalThis.close();
        });
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
    void this.router.navigate(['/identity/register'], {
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

  /**
   * Handle federated login (user@gateway.host).
   * Parses the identifier, resolves the gateway, and initiates OAuth.
   */
  onFederatedLogin(): void {
    this.error.set(null);

    const parsed = parseFederatedIdentifier(this.federatedIdentifier);
    if (!parsed) {
      this.error.set('Please enter a valid identity (e.g. you@your-doorway.host)');
      return;
    }

    this.isLoading.set(true);

    const doorwayUrl = resolveGatewayToDoorwayUrl(parsed.gatewayDomain);
    this.doorwayRegistry.selectDoorwayByUrl(doorwayUrl);

    this.currentStep.set('redirecting');
    this.oauthProvider.storeReturnUrl(this.returnUrl);
    const callbackUrl = `${globalThis.location.origin}/auth/callback`;
    this.oauthProvider.initiateLogin(doorwayUrl, callbackUrl, parsed.username);
  }

  /**
   * Switch to doorway browser/picker view.
   */
  showDoorwayBrowser(): void {
    this.currentStep.set('doorway');
  }

  // ==========================================================================
  // Account Switching
  // ==========================================================================

  /**
   * Show the account switcher overlay.
   */
  showAccountSwitcher(): void {
    this.currentStep.set('switch-account');
  }

  /**
   * Handle account switch that requires restart.
   */
  onAccountSwitchRestart(): void {
    this.restartingDoorwayName.set('the selected account');
    this.currentStep.set('restarting');
  }

  /**
   * Handle "Add Account" from the switcher — go to doorway picker.
   */
  onAccountSwitcherAddAccount(): void {
    this.isLauncher.set(false);
    this.currentStep.set('doorway');
  }

  /**
   * Cancel account switcher — return to unlock.
   */
  onAccountSwitcherCancelled(): void {
    this.currentStep.set('unlock');
  }

  // ==========================================================================
  // Tauri Initialization
  // ==========================================================================

  /**
   * Determine the correct login step for Tauri users.
   *
   * Returning user (has key bundle in doorway.json) -> unlock screen.
   * First-time user -> doorway picker or credentials.
   */
  private async initTauriStep(): Promise<void> {
    // If initialize() already determined needs_unlock, skip redundant IPC
    if (this.tauriAuth.needsUnlock()) {
      this.currentStep.set('unlock');
      return;
    }

    const status = await this.tauriAuth.getDoorwayStatus();

    if (status?.hasKeyBundle && status.hasIdentity) {
      // Returning user: has encrypted key bundle, needs password to unlock
      this.currentStep.set('unlock');
      return;
    }

    // First-time user: doorway picker for bootstrap
    if (!this.doorwayRegistry.hasSelection()) {
      this.isLauncher.set(true);
    }
    this.currentStep.set(this.hasDoorwaySelected() ? 'credentials' : 'doorway');
  }
}
