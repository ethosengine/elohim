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
          <img src="/assets/elohim-logo.svg" alt="Elohim" class="logo" />
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
          <div class="error-banner">
            <span>{{ error() }}</span>
            <button class="dismiss" (click)="clearError()">×</button>
          </div>
        }

        <!-- Login form -->
        @if (state() === 'form') {
          <form (ngSubmit)="onSubmit()" #loginForm="ngForm">
            <div class="form-group">
              <label for="identifier">Email or Username</label>
              <input
                type="text"
                id="identifier"
                name="identifier"
                [(ngModel)]="form.identifier"
                required
                autocomplete="username"
                placeholder="you@example.com"
              />
            </div>

            <div class="form-group">
              <label for="password">Password</label>
              <input
                type="password"
                id="password"
                name="password"
                [(ngModel)]="form.password"
                required
                autocomplete="current-password"
                placeholder="••••••••"
              />
            </div>

            <button type="submit" class="btn-primary" [disabled]="!loginForm.valid">
              Sign In
            </button>
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
          <p>Don't have an account? <a [href]="registerUrl()">Register here</a></p>
        </div>
      </div>
    </div>
  `,
  styles: [`
    .login-container {
      min-height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
      background: linear-gradient(135deg, #0f172a 0%, #1e293b 100%);
      padding: 1rem;
    }

    .login-card {
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
      to { transform: rotate(360deg); }
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
  `]
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
    // Get doorway name from window location or config
    return window.location.hostname;
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
    // Link to registration in elohim-app (same doorway)
    const params = this.oauthParams();
    if (params) {
      return `${params.redirectUri.split('/auth')[0]}/identity/register`;
    }
    return '/identity/register';
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
        this.router.navigate(['/']);
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

    // Make request with auth token - this should redirect
    // Use fetch to follow redirect
    const response = await fetch(authorizeUrl.toString(), {
      method: 'GET',
      headers: {
        'Authorization': `Bearer ${token}`,
      },
      redirect: 'follow',
    });

    // If we get a 3xx redirect, follow it
    if (response.redirected) {
      window.location.href = response.url;
    } else if (response.ok) {
      // Check if response contains a redirect URL
      try {
        const data = await response.json();
        if (data.redirect_uri) {
          window.location.href = data.redirect_uri;
        }
      } catch {
        // Not JSON, might be HTML - check for meta refresh or location header
        this.error.set('Authorization completed but redirect failed');
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
