/**
 * Threshold Register Component
 *
 * Handles new user registration at the doorway.
 * When elohim-app redirects a user here, this component:
 * 1. Reads OAuth params from URL (?client_id, ?redirect_uri, ?response_type, ?state)
 * 2. Shows registration form for the user to create an account
 * 3. Registers auth credentials with doorway (which creates the Holochain
 *    identity — human entry in imagodei zome — server-side)
 * 4. On success, generates authorization code and redirects back to elohim-app
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

// Wire shape (/auth/register) is the schema-contract-pinned generated
// contract (auth-wire plan Task 4 — the drifted local duplicate, e.g.
// `expiresAt: string`, was retired).
import type { AuthResponse } from '../../generated/auth-response';
import { gatewayDomain } from '../../core/gateway-domain';
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

/** State machine for registration flow */
type RegisterState = 'form' | 'registering' | 'authorizing' | 'error';

@Component({
  selector: 'app-threshold-register',
  standalone: true,
  imports: [CommonModule, FormsModule],
  template: `
    <div class="portal-page">
      <section class="portal-card" aria-labelledby="threshold-register-title">
        <span class="portal-mark" role="img" aria-label="Elohim"></span>
        <div class="portal-heading">
          <h1 id="threshold-register-title">Create your account</h1>
          <p>Welcome. This takes a minute.</p>
        </div>

        @if (oauthParams()) {
          <!-- prettier-ignore -->
          <p class="portal-context oauth-info">
            Create an account to continue to <strong class="app-name">{{ clientDisplayName() }}</strong>.
          </p>
        }

        @if (error()) {
          <div class="error-banner" role="alert" data-testid="threshold-register-error">
            <span>{{ error() }}</span>
            <button
              class="dismiss"
              type="button"
              aria-label="Dismiss this message"
              (click)="clearError()"
              data-testid="threshold-register-error-dismiss"
            >
              ×
            </button>
          </div>
        }

        @if (state() === 'form' || state() === 'registering') {
          <form
            class="portal-form"
            (ngSubmit)="onSubmit(registerForm.valid)"
            #registerForm="ngForm"
            [attr.aria-busy]="busy() ? 'true' : null"
          >
            <div class="portal-field">
              <label for="displayName">Your name</label>
              <input
                type="text"
                id="displayName"
                name="displayName"
                data-testid="threshold-register-display-name"
                [(ngModel)]="form.displayName"
                required
                autocomplete="name"
                [readonly]="busy()"
                [attr.aria-invalid]="displayNameError() ? 'true' : null"
                [attr.aria-describedby]="
                  displayNameError() ? 'display-name-error display-name-hint' : 'display-name-hint'
                "
              />
              @if (displayNameError(); as message) {
                <p class="field-error" id="display-name-error">{{ message }}</p>
              }
              <p class="input-hint" id="display-name-hint">
                What people will see. You can change it later.
              </p>
            </div>

            <div class="portal-field">
              <label for="email">Username</label>
              <div class="identifier-wrapper" [class.is-invalid]="usernameError()">
                <input
                  type="text"
                  id="email"
                  name="email"
                  [ngModel]="form.email"
                  (ngModelChange)="onIdentifierChange($event)"
                  required
                  autocomplete="username"
                  [readonly]="busy()"
                  autocapitalize="none"
                  spellcheck="false"
                  pattern="[^@\\s]+"
                  inputmode="text"
                  class="identifier-input"
                  data-testid="threshold-register-email"
                  [attr.aria-invalid]="usernameError() ? 'true' : null"
                  [attr.aria-describedby]="
                    usernameError() ? 'username-error username-hint' : 'username-hint'
                  "
                />
                <span
                  class="domain-suffix"
                  data-testid="threshold-register-domain-suffix"
                  aria-hidden="true"
                >
                  &#64;{{ gatewayDomain() }}
                </span>
              </div>
              @if (usernameError(); as message) {
                <p class="field-error" id="username-error">{{ message }}</p>
              }
              <!-- prettier-ignore -->
              <p class="input-hint" id="username-hint">
                Your account will live at <strong>{{ gatewayDomain() }}</strong>, so choose only the part before the &#64;.
              </p>
            </div>

            <div class="portal-field">
              <label for="password">Password</label>
              <input
                type="password"
                id="password"
                name="password"
                [(ngModel)]="form.password"
                required
                minlength="8"
                autocomplete="new-password"
                [readonly]="busy()"
                data-testid="threshold-register-password"
                [attr.aria-invalid]="passwordError() ? 'true' : null"
                [attr.aria-describedby]="
                  passwordError() ? 'password-error password-hint' : 'password-hint'
                "
              />
              @if (passwordError(); as message) {
                <p class="field-error" id="password-error">{{ message }}</p>
              }
              <p class="input-hint" id="password-hint">At least 8 characters.</p>
            </div>

            <div class="portal-field">
              <label for="confirmPassword">Type the password again</label>
              <input
                type="password"
                id="confirmPassword"
                name="confirmPassword"
                [(ngModel)]="form.confirmPassword"
                required
                autocomplete="new-password"
                [readonly]="busy()"
                data-testid="threshold-register-confirm-password"
                [attr.aria-invalid]="confirmError() ? 'true' : null"
                [attr.aria-describedby]="confirmError() ? 'confirm-password-error' : null"
              />
              @if (confirmError(); as message) {
                <p class="field-error" id="confirm-password-error">{{ message }}</p>
              }
            </div>

            <button
              type="submit"
              class="btn-primary"
              data-testid="threshold-register-submit"
              [attr.aria-disabled]="busy() ? 'true' : null"
            >
              {{ busy() ? 'Creating your account…' : 'Create account' }}
            </button>

            @if (oauthParams()) {
              <div class="federated-section">
                <div class="divider"><span>or</span></div>
                <a
                  [href]="federatedRegisterUrl()"
                  class="federated-link"
                  data-testid="threshold-register-federated"
                >
                  Create an account at a different doorway
                </a>
              </div>
            }
          </form>
        }

        @if (state() === 'authorizing') {
          <div class="loading-state" role="status">
            <div class="trace" aria-hidden="true">
              <span></span>
              <span></span>
              <span></span>
            </div>
            <p>Taking you to {{ clientDisplayName() }}…</p>
          </div>
        }

        @if (state() === 'error') {
          <div class="error-state">
            <button
              class="btn-secondary"
              type="button"
              (click)="retry()"
              data-testid="threshold-register-retry"
            >
              Try again
            </button>
          </div>
        }

        <div class="portal-footer">
          <p>
            Already have an account?
            <a class="portal-link" [href]="loginUrl()" data-testid="threshold-register-login-link">
              Sign in
            </a>
          </p>
        </div>
      </section>
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
  /** True once the person has tried to submit — field messages show from then on. */
  readonly attempted = signal(false);
  /** The request is in flight: the button says so and the form holds still. */
  readonly busy = computed(() => this.state() === 'registering');

  // Form model
  form: RegisterForm = {
    displayName: '',
    email: '',
    password: '',
    confirmPassword: '',
  };

  // Computed values
  /** doorway-alpha.elohim.host --> alpha.elohim.host */
  readonly gatewayDomain = computed(() => gatewayDomain(window.location.hostname));

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

  /** Message for the name field, once a submit has been tried. */
  displayNameError(): string | null {
    return this.attempted() && !this.form.displayName.trim() ? 'Tell us what to call you.' : null;
  }

  /** Message for the username field, once a submit has been tried. */
  usernameError(): string | null {
    if (!this.attempted()) return null;
    if (!this.form.email) return 'Choose a username.';
    if (/\s/.test(this.form.email)) return 'A username can’t contain spaces.';
    return null;
  }

  /** Message for the password field, once a submit has been tried. */
  passwordError(): string | null {
    if (!this.attempted()) return null;
    if (!this.form.password) return 'Choose a password.';
    if (this.form.password.length < 8) return 'Use at least 8 characters.';
    return null;
  }

  /** Shown as soon as both password fields are filled and differ. */
  confirmError(): string | null {
    const { password, confirmPassword } = this.form;
    if (password && confirmPassword && password !== confirmPassword) {
      return 'These passwords don’t match.';
    }
    return this.attempted() && !confirmPassword ? 'Type the password again.' : null;
  }

  /**
   * @param formValid the template form's validity; the button is never
   *   disabled, so an incomplete form explains itself instead of looking dead.
   */
  async onSubmit(formValid: boolean | null = true): Promise<void> {
    if (this.state() !== 'form') return; // already in flight
    this.attempted.set(true);
    // Validate form
    if (formValid === false || !this.form.displayName || !this.form.email || !this.form.password) {
      return;
    }

    // Mismatch and length are explained beside their fields (confirmError /
    // passwordError); nothing is sent until they are right.
    if (this.form.password !== this.form.confirmPassword || this.form.password.length < 8) {
      return;
    }

    this.error.set('');

    try {
      // Register. The doorway creates the Holochain identity INSIDE
      // /auth/register (call_create_human / provision_agent) — there is no
      // client-side identity step to show a spinner for.
      this.state.set('registering');
      const authResult = await this.registerCredentials();

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
          this.error.set('That username is already taken here. Try another, or sign in.');
        } else {
          this.error.set(errorMsg);
        }
      } else {
        this.error.set(err instanceof Error ? err.message : 'An error occurred');
      }
    }
  }

  /**
   * Register auth credentials with doorway.
   *
   * The body is camelCase because doorway's `RegisterRequest` is
   * `#[serde(rename_all = "camelCase")]` (doorway-service/src/routes/auth_routes.rs).
   * snake_case keys deserialize into serde defaults — which is how a typed
   * Display Name used to be silently discarded and the profile named after the
   * identifier's local-part instead.
   *
   * `humanId` / `agentPubKey` are sent empty: the doorway mints the Holochain
   * identity itself when they are, so there is nothing for the browser to
   * create first.
   */
  private async registerCredentials(): Promise<AuthResponse | null> {
    const response = await firstValueFrom(
      this.http.post<AuthResponse>('/auth/register', {
        humanId: '',
        agentPubKey: '',
        identifier: this.form.email,
        identifierType: 'email',
        password: this.form.password,
        displayName: this.form.displayName,
      })
    );
    return response;
  }

  /**
   * Complete OAuth authorization flow.
   * Redirects user back to elohim-app with authorization code.
   */
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

  /**
   * Strip any '@' segment as the human types. The doorway re-qualifies every
   * identifier to `localpart@<gateway domain>` (`normalize_identifier` in
   * auth_routes.rs), so a typed foreign domain is never honoured — showing it
   * in the field promises an account this doorway will not create. Same rule
   * as threshold-login.
   */
  onIdentifierChange(value: string): void {
    const atIndex = value.indexOf('@');
    this.form.email = atIndex === -1 ? value : value.slice(0, atIndex);
  }

  clearError(): void {
    this.error.set('');
  }

  retry(): void {
    this.state.set('form');
    this.error.set('');
  }
}
