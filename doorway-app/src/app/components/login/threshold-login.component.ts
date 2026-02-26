/**
 * Threshold Login Component
 *
 * Handles OAuth authorization at the doorway.
 * When elohim-app redirects a user here, this component:
 * 1. Reads OAuth params from URL (?client_id, ?redirect_uri, ?response_type, ?state)
 * 2. Shows login form for the user to authenticate
 * 3. On success, generates authorization code and redirects back to elohim-app
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
  loginHint?: string;
}

/** Login form state */
interface LoginForm {
  identifier: string;
  password: string;
}

/** Auth response from /auth/login */
interface AuthResponse {
  token: string;
  humanId: string;
  agentPubKey: string;
  expiresAt: string;
  identifier: string;
}

/** State machine for login flow */
type LoginState = 'form' | 'authenticating' | 'authorizing' | 'error';

@Component({
  selector: 'app-threshold-login',
  standalone: true,
  imports: [CommonModule, FormsModule],
  template: `
    <div class="login-container">
      <div class="login-card">
        <!-- Doorway branding -->
        <div class="branding">
          <img src="/threshold/images/elohim_logo_light.png" alt="Elohim" class="logo" />
          <h1>Sign In</h1>
          <p class="doorway-name">{{ doorwayName() }}</p>
        </div>

        <!-- OAuth info -->
        @if (oauthParams()) {
          <div class="oauth-info">
            <span class="app-name">{{ clientDisplayName() }}</span>
            <span class="oauth-action">wants to access your account</span>
          </div>
        }

        <!-- Error message -->
        @if (error()) {
          <div class="error-banner" data-testid="threshold-error">
            <span>{{ error() }}</span>
            <button class="dismiss" (click)="clearError()" data-testid="threshold-error-dismiss">×</button>
          </div>
        }

        <!-- Login form -->
        @if (state() === 'form') {
          <form (ngSubmit)="onSubmit()" #loginForm="ngForm">
            <div class="form-group">
              <label for="identifier">Username</label>
              <div class="identifier-wrapper">
                <input
                  type="text"
                  id="identifier"
                  name="identifier"
                  data-testid="threshold-identifier"
                  [(ngModel)]="form.identifier"
                  required
                  autocomplete="username"
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
                data-testid="threshold-password"
                [(ngModel)]="form.password"
                required
                autocomplete="current-password"
                placeholder="••••••••"
              />
            </div>

            <button type="submit" class="btn-primary" data-testid="threshold-submit" [disabled]="!loginForm.valid">
              Sign In
            </button>

            @if (oauthParams()) {
              <div class="federated-section">
                <div class="divider"><span>or</span></div>
                <a [href]="federatedLoginUrl()" class="federated-link" data-testid="threshold-federated-login">
                  Login with a different doorway
                </a>
              </div>
            }
          </form>
        }

        <!-- Loading states -->
        @if (state() === 'authenticating') {
          <div class="loading-state">
            <div class="spinner"></div>
            <p>Verifying your credentials...</p>
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
          <p>Don't have an account? <a [href]="registerUrl()" data-testid="threshold-register-link">Register here</a></p>
        </div>
      </div>
    </div>
  `,
  styleUrl: './threshold-login.component.css'
})
export class ThresholdLoginComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly http = inject(HttpClient);

  // State
  readonly state = signal<LoginState>('form');
  readonly error = signal<string>('');
  readonly oauthParams = signal<OAuthParams | null>(null);

  // Form model
  form: LoginForm = {
    identifier: '',
    password: '',
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

  readonly registerUrl = computed(() => {
    // Link to doorway's own registration page with OAuth params
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
      return `/threshold/register?${searchParams.toString()}`;
    }
    return '/threshold/register';
  });

  readonly federatedLoginUrl = computed(() => {
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
    const loginHint = params['login_hint'];

    if (clientId && redirectUri && state) {
      this.oauthParams.set({
        clientId,
        redirectUri,
        responseType: responseType ?? 'code',
        state,
        scope,
        loginHint,
      });
    }

    // Pre-fill identifier from login_hint
    if (loginHint) {
      this.form.identifier = loginHint;
    }
  }

  async onSubmit(): Promise<void> {
    if (!this.form.identifier || !this.form.password) {
      return;
    }

    this.state.set('authenticating');
    this.error.set('');

    try {
      // Authenticate with doorway
      const authResult = await this.authenticate();

      if (!authResult) {
        throw new Error('Authentication failed');
      }

      // If OAuth flow, generate authorization code
      const params = this.oauthParams();
      if (params) {
        this.state.set('authorizing');
        await this.authorizeOAuth(authResult.token, params);
      } else {
        // Direct login (no OAuth) - redirect to dashboard
        this.router.navigate(['/dashboard']);
      }
    } catch (err) {
      this.state.set('form');
      if (err instanceof HttpErrorResponse) {
        this.error.set(err.error?.error ?? 'Authentication failed');
      } else {
        this.error.set(err instanceof Error ? err.message : 'An error occurred');
      }
    }
  }

  private async authenticate(): Promise<AuthResponse | null> {
    const response = await firstValueFrom(
      this.http.post<AuthResponse>('/auth/login', {
        identifier: this.form.identifier,
        password: this.form.password,
      })
    );
    return response;
  }

  private async authorizeOAuth(token: string, params: OAuthParams): Promise<void> {
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
        'Authorization': `Bearer ${token}`,
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
