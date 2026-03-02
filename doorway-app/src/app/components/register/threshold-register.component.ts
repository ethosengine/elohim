/**
 * Threshold Register Component
 *
 * Handles new user registration at the doorway.
 * When elohim-app redirects a user here, this component:
 * 1. Reads OAuth params from URL (?client_id, ?redirect_uri, ?response_type, ?state)
 * 2. Shows registration form for the user to create an account
 * 3. Creates Holochain identity (human entry in imagodei zome)
 * 4. Registers auth credentials with doorway
 * 5. On success, generates authorization code and redirects back to elohim-app
 *
 * This enables the thin-federated architecture where any doorway
 * can be an identity provider for elohim-app.
 */

import { Component, OnInit, inject, signal, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { ActivatedRoute, Router } from '@angular/router';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';

import { AuthStateService } from '../../services/auth-state.service';

/** OAuth params from query string */
interface OAuthParams {
  clientId: string;
  redirectUri: string;
  responseType: string;
  state: string;
  scope?: string;
}

/** Registration form state */
interface RegisterForm {
  displayName: string;
  email: string;
  password: string;
  confirmPassword: string;
}

/** Auth response from /auth/register */
interface AuthResponse {
  token: string;
  humanId: string;
  agentPubKey: string;
  expiresAt: string;
  identifier: string;
}

/** State machine for registration flow */
type RegisterState =
  | 'form'
  | 'creating_identity'
  | 'registering'
  | 'authorizing'
  | 'error';

@Component({
  selector: 'app-threshold-register',
  standalone: true,
  imports: [CommonModule, FormsModule],
  template: `
    <div class="register-container">
      <div class="register-card">
        <!-- Doorway branding -->
        <div class="branding">
          <img src="/threshold/images/elohim_logo_light.png" alt="Elohim" class="logo" />
          <h1>Create Account</h1>
          <p class="doorway-name">{{ doorwayName() }}</p>
        </div>

        <!-- OAuth info -->
        @if (oauthParams()) {
          <div class="oauth-info">
            <span class="app-name">{{ clientDisplayName() }}</span>
            <span class="oauth-action">wants you to create an account</span>
          </div>
        }

        <!-- Error message -->
        @if (error()) {
          <div class="error-banner">
            <span>{{ error() }}</span>
            <button class="dismiss" (click)="clearError()" data-testid="threshold-register-error-dismiss">×</button>
          </div>
        }

        <!-- Registration form -->
        @if (state() === 'form') {
          <form (ngSubmit)="onSubmit()" #registerForm="ngForm">
            <div class="form-group">
              <label for="displayName">Display Name</label>
              <input
                type="text"
                id="displayName"
                name="displayName"
                data-testid="threshold-register-display-name"
                [(ngModel)]="form.displayName"
                required
                autocomplete="name"
                placeholder="Your name"
              />
            </div>

            <div class="form-group">
              <label for="email">Email</label>
              <div class="identifier-wrapper">
                <input
                  type="text"
                  id="email"
                  name="email"
                  [(ngModel)]="form.email"
                  required
                  autocomplete="email"
                  placeholder="username"
                  class="identifier-input"
                  data-testid="threshold-register-email"
                />
                <span class="domain-suffix">&#64;{{ gatewayDomain() }}</span>
              </div>
              <p class="input-hint">Or use your full email address</p>
            </div>

            <div class="form-group">
              <label for="password">Password</label>
              <input
                type="password"
                id="password"
                name="password"
                [(ngModel)]="form.password"
                required
                minlength="8"
                autocomplete="new-password"
                placeholder="At least 8 characters"
                data-testid="threshold-register-password"
              />
            </div>

            <div class="form-group">
              <label for="confirmPassword">Confirm Password</label>
              <input
                type="password"
                id="confirmPassword"
                name="confirmPassword"
                [(ngModel)]="form.confirmPassword"
                required
                autocomplete="new-password"
                placeholder="Re-enter your password"
                data-testid="threshold-register-confirm-password"
              />
              @if (form.password && form.confirmPassword && form.password !== form.confirmPassword) {
                <span class="field-error">Passwords do not match</span>
              }
            </div>

            <button
              type="submit"
              class="btn-primary"
              [disabled]="!registerForm.valid || form.password !== form.confirmPassword"
              data-testid="threshold-register-submit"
            >
              Create Account
            </button>

            @if (oauthParams()) {
              <div class="federated-section">
                <div class="divider"><span>or</span></div>
                <a [href]="federatedRegisterUrl()" class="federated-link" data-testid="threshold-register-federated">
                  Register with a different doorway
                </a>
              </div>
            }
          </form>
        }

        <!-- Loading states -->
        @if (state() === 'creating_identity') {
          <div class="loading-state">
            <div class="spinner"></div>
            <p>Creating your identity...</p>
          </div>
        }

        @if (state() === 'registering') {
          <div class="loading-state">
            <div class="spinner"></div>
            <p>Setting up your account...</p>
          </div>
        }

        @if (state() === 'authorizing') {
          <div class="loading-state">
            <div class="spinner"></div>
            <p>Authorizing {{ clientDisplayName() }}...</p>
          </div>
        }

        @if (state() === 'error') {
          <div class="error-state">
            <button class="btn-secondary" (click)="retry()" data-testid="threshold-register-retry">Try Again</button>
          </div>
        }

        <!-- Footer -->
        <div class="footer">
          <p>Already have an account? <a [href]="loginUrl()" data-testid="threshold-register-login-link">Sign in</a></p>
        </div>
      </div>
    </div>
  `,
  styleUrl: './threshold-register.component.css',
})
export class ThresholdRegisterComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly http = inject(HttpClient);
  private readonly authState = inject(AuthStateService);

  // State
  readonly state = signal<RegisterState>('form');
  readonly error = signal<string>('');
  readonly oauthParams = signal<OAuthParams | null>(null);

  // Form model
  form: RegisterForm = {
    displayName: '',
    email: '',
    password: '',
    confirmPassword: '',
  };

  // Computed values
  readonly doorwayName = computed(() => {
    return window.location.hostname;
  });

  readonly gatewayDomain = computed(() => {
    const hostname = window.location.hostname;
    // doorway-alpha.elohim.host --> alpha.elohim.host
    return hostname.startsWith('doorway-') ? hostname.replace(/^doorway-/, '') : hostname;
  });

  readonly clientDisplayName = computed(() => {
    const params = this.oauthParams();
    if (!params) return 'Unknown App';

    // Map known client IDs to friendly names
    const clientNames: Record<string, string> = {
      'elohim-app': 'Elohim App',
      'doorway-app': 'Doorway Dashboard',
    };

    return clientNames[params.clientId] ?? params.clientId;
  });

  readonly loginUrl = computed(() => {
    // Link to login page with same OAuth params
    const params = this.oauthParams();
    if (params) {
      const searchParams = new URLSearchParams({
        client_id: params.clientId,
        redirect_uri: params.redirectUri,
        response_type: params.responseType,
        state: params.state,
      });
      if (params.scope) {
        searchParams.set('scope', params.scope);
      }
      return `/threshold/login?${searchParams.toString()}`;
    }
    return '/threshold/login';
  });

  readonly federatedRegisterUrl = computed(() => {
    const params = this.oauthParams();
    if (params) {
      const searchParams = new URLSearchParams({
        client_id: params.clientId,
        redirect_uri: params.redirectUri,
        response_type: params.responseType,
        state: params.state,
      });
      if (params.scope) {
        searchParams.set('scope', params.scope);
      }
      return `/threshold/doorways?${searchParams.toString()}`;
    }
    return '/threshold/doorways';
  });

  ngOnInit(): void {
    // Parse OAuth params from URL
    this.parseOAuthParams();
  }

  private parseOAuthParams(): void {
    const params = this.route.snapshot.queryParams;

    const clientId = params['client_id'];
    const redirectUri = params['redirect_uri'];
    const responseType = params['response_type'];
    const state = params['state'];
    const scope = params['scope'];

    if (clientId && redirectUri && state) {
      this.oauthParams.set({
        clientId,
        redirectUri,
        responseType: responseType ?? 'code',
        state,
        scope,
      });
    }
  }

  async onSubmit(): Promise<void> {
    // Validate form
    if (!this.form.displayName || !this.form.email || !this.form.password) {
      return;
    }

    if (this.form.password !== this.form.confirmPassword) {
      this.error.set('Passwords do not match');
      return;
    }

    if (this.form.password.length < 8) {
      this.error.set('Password must be at least 8 characters');
      return;
    }

    this.error.set('');

    try {
      // Step 1: Create Holochain identity
      this.state.set('creating_identity');
      const identity = await this.createHolochainIdentity();

      // Step 2: Register auth credentials
      this.state.set('registering');
      const authResult = await this.registerCredentials(identity);

      if (!authResult) {
        throw new Error('Registration failed');
      }

      // Store token so the auth interceptor can attach it to subsequent requests
      this.authState.storeToken(authResult.token);

      // Step 3: If OAuth flow, generate authorization code
      const params = this.oauthParams();
      if (params) {
        this.state.set('authorizing');
        await this.authorizeOAuth(authResult.token, params);
      } else {
        // Direct registration (no OAuth) - refresh auth state and redirect to dashboard
        await this.authState.refresh();
        this.router.navigate(['/dashboard']);
      }
    } catch (err) {
      this.state.set('form');
      if (err instanceof HttpErrorResponse) {
        const errorMsg = err.error?.error ?? err.error?.message ?? 'Registration failed';
        // Handle specific error codes
        if (err.status === 409) {
          this.error.set('An account with this email already exists');
        } else {
          this.error.set(errorMsg);
        }
      } else {
        this.error.set(err instanceof Error ? err.message : 'An error occurred');
      }
    }
  }

  /**
   * Create Holochain identity via doorway's WebSocket proxy.
   * This calls the imagodei zome's create_human function.
   */
  private async createHolochainIdentity(): Promise<{
    humanId: string;
    agentPubKey: string;
  }> {
    // For doorway-hosted registration, we use an HTTP API endpoint
    // that wraps the zome call, since we don't have a WebSocket client here.
    //
    // The doorway's /auth/register endpoint can optionally create the identity
    // if humanId is not provided. This is the recommended approach.
    //
    // For now, we'll pass empty values and let the backend handle it.
    // The backend will create the identity if needed.
    return {
      humanId: '',
      agentPubKey: '',
    };
  }

  /**
   * Register auth credentials with doorway.
   */
  private async registerCredentials(identity: {
    humanId: string;
    agentPubKey: string;
  }): Promise<AuthResponse | null> {
    const response = await firstValueFrom(
      this.http.post<AuthResponse>('/auth/register', {
        human_id: identity.humanId,
        agent_pub_key: identity.agentPubKey,
        identifier: this.form.email,
        identifier_type: 'email',
        password: this.form.password,
        display_name: this.form.displayName,
      })
    );
    return response;
  }

  /**
   * Complete OAuth authorization flow.
   * Redirects user back to elohim-app with authorization code.
   */
  private async authorizeOAuth(
    token: string,
    params: OAuthParams
  ): Promise<void> {
    // Call /auth/authorize with the token to get the authorization code
    // The backend will redirect us to the client's redirect_uri
    const authorizeUrl = new URL('/auth/authorize', window.location.origin);
    authorizeUrl.searchParams.set('client_id', params.clientId);
    authorizeUrl.searchParams.set('redirect_uri', params.redirectUri);
    authorizeUrl.searchParams.set('response_type', params.responseType);
    authorizeUrl.searchParams.set('state', params.state);
    if (params.scope) {
      authorizeUrl.searchParams.set('scope', params.scope);
    }

    // Request authorization code. Backend returns JSON { redirect_uri }
    // when it sees a Bearer token (SPA flow), avoiding cross-origin 302.
    const response = await fetch(authorizeUrl.toString(), {
      method: 'GET',
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });

    if (response.ok) {
      const data = await response.json();
      if (data.redirect_uri) {
        window.location.href = data.redirect_uri;
      } else {
        this.error.set('Authorization completed but no redirect received');
        this.state.set('error');
      }
    } else {
      throw new Error('Authorization failed');
    }
  }

  clearError(): void {
    this.error.set('');
  }

  retry(): void {
    this.state.set('form');
    this.error.set('');
  }
}
