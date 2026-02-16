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
            <button class="dismiss" (click)="clearError()">×</button>
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
              />
              @if (form.password && form.confirmPassword && form.password !== form.confirmPassword) {
                <span class="field-error">Passwords do not match</span>
              }
            </div>

            <button
              type="submit"
              class="btn-primary"
              [disabled]="!registerForm.valid || form.password !== form.confirmPassword"
            >
              Create Account
            </button>

            @if (oauthParams()) {
              <div class="federated-section">
                <div class="divider"><span>or</span></div>
                <a [href]="federatedRegisterUrl()" class="federated-link">
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
            <button class="btn-secondary" (click)="retry()">Try Again</button>
          </div>
        }

        <!-- Footer -->
        <div class="footer">
          <p>Already have an account? <a [href]="loginUrl()">Sign in</a></p>
        </div>
      </div>
    </div>
  `,
  styles: [
    `
      .register-container {
        min-height: 100vh;
        display: flex;
        align-items: center;
        justify-content: center;
        background: linear-gradient(135deg, #0f172a 0%, #1e293b 100%);
        padding: 1rem;
      }

      .register-card {
        background: #1e293b;
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 1rem;
        padding: 2.5rem;
        width: 100%;
        max-width: 400px;
        box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
      }

      .branding {
        text-align: center;
        margin-bottom: 2rem;
      }

      .logo {
        width: 64px;
        height: 64px;
        margin-bottom: 1rem;
      }

      .branding h1 {
        color: #fff;
        font-size: 1.5rem;
        font-weight: 600;
        margin: 0 0 0.5rem;
      }

      .doorway-name {
        color: rgba(255, 255, 255, 0.5);
        font-size: 0.875rem;
        margin: 0;
      }

      .oauth-info {
        background: rgba(99, 102, 241, 0.1);
        border: 1px solid rgba(99, 102, 241, 0.2);
        border-radius: 0.5rem;
        padding: 0.75rem 1rem;
        margin-bottom: 1.5rem;
        text-align: center;
      }

      .app-name {
        color: #818cf8;
        font-weight: 600;
      }

      .oauth-action {
        color: rgba(255, 255, 255, 0.7);
        margin-left: 0.25rem;
      }

      .error-banner {
        background: rgba(239, 68, 68, 0.1);
        border: 1px solid rgba(239, 68, 68, 0.2);
        border-radius: 0.5rem;
        padding: 0.75rem 1rem;
        margin-bottom: 1.5rem;
        display: flex;
        align-items: center;
        justify-content: space-between;
        color: #fca5a5;
      }

      .dismiss {
        background: none;
        border: none;
        color: #fca5a5;
        font-size: 1.25rem;
        cursor: pointer;
        padding: 0 0.25rem;
      }

      form {
        display: flex;
        flex-direction: column;
        gap: 1.25rem;
      }

      .form-group {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
      }

      label {
        color: rgba(255, 255, 255, 0.8);
        font-size: 0.875rem;
        font-weight: 500;
      }

      input {
        background: #0f172a;
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 0.5rem;
        padding: 0.75rem 1rem;
        color: #fff;
        font-size: 1rem;
        transition: border-color 0.2s;
      }

      input::placeholder {
        color: rgba(255, 255, 255, 0.3);
      }

      input:focus {
        outline: none;
        border-color: #6366f1;
      }

      .field-error {
        color: #fca5a5;
        font-size: 0.75rem;
      }

      .btn-primary {
        background: #6366f1;
        color: white;
        border: none;
        border-radius: 0.5rem;
        padding: 0.875rem 1.5rem;
        font-size: 1rem;
        font-weight: 500;
        cursor: pointer;
        transition: background 0.2s;
        margin-top: 0.5rem;
      }

      .btn-primary:hover:not(:disabled) {
        background: #4f46e5;
      }

      .btn-primary:disabled {
        background: rgba(99, 102, 241, 0.5);
        cursor: not-allowed;
      }

      .btn-secondary {
        background: rgba(255, 255, 255, 0.1);
        color: white;
        border: 1px solid rgba(255, 255, 255, 0.2);
        border-radius: 0.5rem;
        padding: 0.75rem 1.5rem;
        font-size: 1rem;
        cursor: pointer;
        transition: background 0.2s;
      }

      .btn-secondary:hover {
        background: rgba(255, 255, 255, 0.15);
      }

      .loading-state {
        text-align: center;
        padding: 2rem 0;
      }

      .spinner {
        width: 40px;
        height: 40px;
        border: 3px solid rgba(255, 255, 255, 0.1);
        border-top-color: #6366f1;
        border-radius: 50%;
        animation: spin 1s linear infinite;
        margin: 0 auto 1rem;
      }

      @keyframes spin {
        to {
          transform: rotate(360deg);
        }
      }

      .loading-state p {
        color: rgba(255, 255, 255, 0.7);
        margin: 0;
      }

      .error-state {
        text-align: center;
        padding: 1rem 0;
      }

      .footer {
        text-align: center;
        margin-top: 2rem;
        padding-top: 1.5rem;
        border-top: 1px solid rgba(255, 255, 255, 0.1);
      }

      .footer p {
        color: rgba(255, 255, 255, 0.5);
        font-size: 0.875rem;
        margin: 0;
      }

      .footer a {
        color: #818cf8;
        text-decoration: none;
      }

      .footer a:hover {
        text-decoration: underline;
      }

      .identifier-wrapper {
        display: flex;
        align-items: center;
        background: #0f172a;
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 0.5rem;
        transition: border-color 0.2s;
      }

      .identifier-wrapper:focus-within {
        border-color: #6366f1;
      }

      .identifier-input {
        flex: 1;
        border: none !important;
        background: transparent !important;
        border-radius: 0.5rem 0 0 0.5rem;
        min-width: 0;
      }

      .identifier-input:focus {
        border-color: transparent !important;
      }

      .domain-suffix {
        color: rgba(255, 255, 255, 0.4);
        font-size: 0.875rem;
        padding: 0 0.75rem;
        white-space: nowrap;
        user-select: none;
      }

      .input-hint {
        color: rgba(255, 255, 255, 0.35);
        font-size: 0.75rem;
        margin: 0;
      }

      .federated-section {
        margin-top: 1rem;
      }

      .divider {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        margin-bottom: 1rem;
      }

      .divider::before,
      .divider::after {
        content: '';
        flex: 1;
        height: 1px;
        background: rgba(255, 255, 255, 0.1);
      }

      .divider span {
        color: rgba(255, 255, 255, 0.4);
        font-size: 0.75rem;
        text-transform: uppercase;
        letter-spacing: 0.05em;
      }

      .federated-link {
        display: block;
        text-align: center;
        color: #818cf8;
        text-decoration: none;
        font-size: 0.875rem;
        padding: 0.625rem 1rem;
        border: 1px solid rgba(99, 102, 241, 0.3);
        border-radius: 0.5rem;
        transition: background 0.2s, border-color 0.2s;
      }

      .federated-link:hover {
        background: rgba(99, 102, 241, 0.1);
        border-color: rgba(99, 102, 241, 0.5);
      }
    `,
  ],
})
export class ThresholdRegisterComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly http = inject(HttpClient);

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

      // Step 3: If OAuth flow, generate authorization code
      const params = this.oauthParams();
      if (params) {
        this.state.set('authorizing');
        await this.authorizeOAuth(authResult.token, params);
      } else {
        // Direct registration (no OAuth) - redirect to dashboard
        this.router.navigate(['/']);
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
